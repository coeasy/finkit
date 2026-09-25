#!/usr/bin/env python3
"""check_workflow_liveness.py — fail when a GitHub Actions workflow can never run.

Orphan automation is worse than missing automation: a workflow file reads as a
live gate, so its absence from the pipeline is invisible.  This repository has
already produced one instance of the defect — a one-shot release orchestrator
pinned to `release/v0.1.4`, a hard-coded commit SHA and a hard-coded run id,
left behind after the branch it waits for was deleted.

Reachability
------------
A workflow is reachable when at least one of its triggers can fire:

  * an unconditional trigger: `workflow_dispatch`, `schedule`, `release`,
    `workflow_call`, `merge_group`, `repository_dispatch`, or `push` /
    `pull_request` with no branch filter at all (every branch);
  * a `push` / `pull_request` branch filter that matches at least one branch
    that exists on the remote (glob-aware).

Otherwise the workflow fails the check.  A workflow that is still reachable but
carries a *dormant* branch filter (one matching no existing branch) is reported
as information, not as a failure — the check is about "can this ever run", not
"is every branch filter still meaningful".

Exit codes: 0 clean, 1 at least one unreachable workflow.
"""
from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
WORKFLOWS = ROOT / '.github' / 'workflows'

# Triggers that fire without depending on which branch is pushed.
UNCONDITIONAL = {
    'workflow_dispatch', 'schedule', 'release', 'workflow_call',
    'merge_group', 'repository_dispatch',
}
BRANCH_FILTERED = {'push', 'pull_request', 'pull_request_target'}
BRANCH_KEYS = ('branches', 'branches-ignore')

KEY_RE = re.compile(r'^(\s*)([A-Za-z_][A-Za-z0-9_-]*)\s*:\s*(.*)$')


def workflow_files() -> list[Path]:
    if not WORKFLOWS.is_dir():
        return []
    return sorted([*WORKFLOWS.glob('*.yml'), *WORKFLOWS.glob('*.yaml')])


def parse_on(text: str) -> dict[str, dict[str, list[str]]] | None:
    """Return {trigger: {branch_key: [patterns]}} for the `on:` block.

    Deliberately a hand-rolled indentation scan rather than a YAML dependency:
    the other gate scripts in this directory are dependency-free too, and the
    `on:` block is a small, regular subset of YAML.
    """
    lines = text.splitlines()
    start = None
    for i, ln in enumerate(lines):
        if re.match(r'^on\s*:', ln):
            start = i
            break
    if start is None:
        return None

    block = [lines[start]]
    for ln in lines[start + 1:]:
        if ln.strip() and not ln[0].isspace():
            break
        block.append(ln)

    head = block[0].split(':', 1)[1].strip()
    if head:
        # `on: push`  or  `on: [push, pull_request]`
        if head.startswith('['):
            return {t.strip().strip('\'"'): {} for t in head.strip('[]').split(',') if t.strip()}
        return {head: {}}

    triggers: dict[str, dict[str, list[str]]] = {}
    current = None
    branch_key = None
    for ln in block[1:]:
        if not ln.strip() or ln.lstrip().startswith('#'):
            continue
        m = KEY_RE.match(ln)
        indent = len(ln) - len(ln.lstrip())
        if m and indent <= 2:
            current = m.group(2)
            branch_key = None
            triggers[current] = {}
            inline = m.group(3).strip()
            if inline.startswith('['):
                triggers[current]['__inline__'] = [
                    t.strip().strip('\'"') for t in inline.strip('[]').split(',') if t.strip()
                ]
        elif m and current is not None and indent <= 4:
            branch_key = m.group(2)
            triggers[current][branch_key] = []
        elif current is not None and branch_key is not None and re.match(r'^\s*-\s', ln):
            triggers[current][branch_key].append(ln.strip()[1:].strip().strip('\'"'))
    return triggers


def glob_to_regex(pattern: str) -> re.Pattern:
    """GitHub branch globs: `*` does not cross `/`, `**` does, `?` is one char."""
    out = ['^']
    i = 0
    while i < len(pattern):
        ch = pattern[i]
        if ch == '*':
            if pattern[i:i + 2] == '**':
                out.append('.*')
                i += 2
                continue
            out.append('[^/]*')
        elif ch == '?':
            out.append('[^/]')
        else:
            out.append(re.escape(ch))
        i += 1
    out.append('$')
    return re.compile(''.join(out))


def existing_branches() -> tuple[set[str], str | None]:
    """Branches on the remote; falls back to local refs. Returns (branches, warning)."""
    try:
        out = subprocess.run(
            ['git', '-c', 'credential.helper=', '-c', 'credential.helper=manager',
             'ls-remote', '--heads', 'origin'],
            cwd=ROOT, capture_output=True, text=True, timeout=60, check=True,
        ).stdout
        names = {ln.split('refs/heads/', 1)[1] for ln in out.splitlines() if 'refs/heads/' in ln}
        if names:
            return names, None
    except (subprocess.SubprocessError, OSError):
        pass

    try:
        out = subprocess.run(
            ['git', 'for-each-ref', '--format=%(refname:short)', 'refs/heads', 'refs/remotes/origin'],
            cwd=ROOT, capture_output=True, text=True, timeout=60, check=True,
        ).stdout
        names = set()
        for ln in out.splitlines():
            ln = ln.strip()
            if not ln or ln.endswith('/HEAD'):
                continue
            names.add(ln.split('origin/', 1)[1] if ln.startswith('origin/') else ln)
        if names:
            return names, 'remote branch list unavailable; used local refs'
    except (subprocess.SubprocessError, OSError):
        pass

    return set(), 'could not determine the branch list'


def branch_patterns(spec: dict[str, list[str]]) -> list[str]:
    """All branch patterns declared for one trigger."""
    patterns: list[str] = list(spec.get('__inline__', []))
    for key in BRANCH_KEYS:
        patterns.extend(spec.get(key, []))
    return patterns


def dormant_notes(triggers: dict[str, dict[str, list[str]]], branches: set[str],
                  branches_known: bool) -> list[str]:
    """Report branch filters that match no existing branch (informational only)."""
    notes = []
    if not branches_known:
        return notes
    for trigger in sorted(set(triggers) & BRANCH_FILTERED):
        patterns = branch_patterns(triggers[trigger])
        if not patterns:
            continue
        if not any(glob_to_regex(p).match(b) for b in branches for p in patterns):
            notes.append(f'{trigger}: dormant branch filter {patterns} matches no existing branch')
    return notes


def evaluate(triggers: dict[str, dict[str, list[str]]], branches: set[str],
             branches_known: bool) -> tuple[bool, list[str], list[str]]:
    """Return (reachable, reasons_why_not, informational_notes)."""
    keys = set(triggers)
    notes = dormant_notes(triggers, branches, branches_known)

    if keys & UNCONDITIONAL:
        return True, [], notes

    if not (keys & BRANCH_FILTERED):
        return False, [f'no trigger can fire (found only: {sorted(keys) or "none"})'], notes

    for trigger in sorted(keys & BRANCH_FILTERED):
        patterns = branch_patterns(triggers[trigger])
        if not patterns:
            return True, [], notes          # fires on every branch
        if not branches_known:
            notes.append(f'{trigger}: branch filter {patterns} not verifiable (no branch list)')
            return True, [], notes
        if any(glob_to_regex(p).match(b) for b in branches for p in patterns):
            return True, [], notes

    reasons = [
        'every trigger is branch-filtered and no filtered branch exists, '
        'and there is no workflow_dispatch/schedule/release/workflow_call',
    ]
    return False, reasons, notes


def main() -> int:
    files = workflow_files()
    if not files:
        print('no workflow files found; nothing to check')
        return 0

    branches, warning = existing_branches()
    branches_known = bool(branches)
    if warning:
        print(f'note: {warning}')

    unreachable: list[tuple[str, list[str]]] = []
    dormant: list[tuple[str, list[str]]] = []
    unparsed: list[str] = []

    for path in files:
        text = path.read_text(encoding='utf-8')
        triggers = parse_on(text)
        if triggers is None:
            unparsed.append(path.name)
            continue
        ok, reasons, notes = evaluate(triggers, branches, branches_known)
        if not ok:
            unreachable.append((path.name, reasons))
        if notes:
            dormant.append((path.name, notes))

    print(f'scanned {len(files)} workflow(s) against {len(branches)} known branch(es)')
    for name, notes in dormant:
        for note in notes:
            print(f'  info: {name}: {note}')

    if unparsed:
        print('FAIL: no `on:` block found:')
        for name in unparsed:
            print(f'  - .github/workflows/{name}')
        print('FAIL: 1 problem(s)')
        return 1

    if unreachable:
        print('FAIL: workflow(s) that can never be triggered:')
        for name, reasons in unreachable:
            for reason in reasons:
                print(f'  - .github/workflows/{name}: {reason}')
        print('  Add `workflow_dispatch`, or fix/remove the stale branch filter.')
        print(f'FAIL: {len(unreachable)} problem(s)')
        return 1

    print('OK: every workflow has a trigger that can fire')
    return 0


if __name__ == '__main__':
    sys.exit(main())
