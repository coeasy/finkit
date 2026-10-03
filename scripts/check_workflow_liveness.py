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

Dangling `workflow_run` dependencies
------------------------------------
"Can this ever run" has a second, less obvious form. `on.workflow_run.workflows`
lists workflows **by display name**, and a name that does not match any
workflow's `name:` in this tree makes the trigger dead — the workflow waits for
something that never reports. That defect is invisible to the reachability
analysis above, because such a workflow usually still carries
`workflow_dispatch` and therefore looks alive.

`.github/workflows/release-readiness.yml` is the case that matters: it is the
release licence ("same SHA, every required workflow green"). A typo in its eight
names would leave release readiness permanently un-triggered while CI stayed
green, which is precisely the silent-gate failure this repository's gates exist
to prevent. So every `workflow_run` target is resolved against the set of
declared names, and the aggregator's independent list is checked against the
same names in both directions — one declaration, one gate, no drift.

Exit codes: 0 clean, 1 at least one unreachable/dangling workflow or a
release-readiness list that has drifted.
"""
from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
WORKFLOWS = ROOT / '.github' / 'workflows'
AGGREGATOR = ROOT / 'scripts' / 'release_readiness_aggregate.py'
RELEASE_READINESS = WORKFLOWS / 'release-readiness.yml'

# Triggers that fire without depending on which branch is pushed.
#
# `workflow_dispatch` is deliberately **absent** from this set even though it is
# unconditional in practice. Including it made the whole check meaningless: every
# workflow here is created with `workflow_dispatch` (it is part of GitHub's own
# template), so `keys & UNCONDITIONAL` was always true and `evaluate` returned
# "reachable" before reading a single branch filter. A workflow pinned to a
# branch deleted months ago still reported "every workflow has a trigger that
# can fire". Manual triggers are honoured as a *fallback* below, not as a
# blanket excuse to stop looking.
#
# `workflow_run` *is* automatic: it fires when a named workflow completes,
# independently of branches. Leaving it out caused two opposite errors on
# `.github/workflows/release-readiness.yml`, whose only automatic trigger is
# `workflow_run`: it was reported as "no automatic trigger; fires only via
# workflow_dispatch" (misleading), and a workflow relying on `workflow_run`
# *alone* would have been failed as unreachable (false positive). Its
# reachability is conditional on the named targets existing, which is exactly
# what `workflow_run_targets` below now verifies — the two halves belong
# together.
UNCONDITIONAL = {
    'schedule', 'release', 'workflow_call', 'workflow_run',
    'merge_group', 'repository_dispatch',
}
MANUAL_ONLY = {'workflow_dispatch'}
BRANCH_FILTERED = {'push', 'pull_request', 'pull_request_target'}
BRANCH_KEYS = ('branches', 'branches-ignore')

KEY_RE = re.compile(r'^(\s*)([A-Za-z_][A-Za-z0-9_-]*)\s*:\s*(.*)$')


def workflow_files() -> list[Path]:
    if not WORKFLOWS.is_dir():
        return []
    return sorted([*WORKFLOWS.glob('*.yml'), *WORKFLOWS.glob('*.yaml')])


def declared_names(texts: dict[str, str]) -> dict[str, str]:
    """Map each workflow's display `name:` to the file that declares it.

    `name:` must start at column 0 — a nested `name:` (a job name, a step name)
    is not the workflow's identity and must not shadow one.
    """
    names: dict[str, str] = {}
    for filename, text in texts.items():
        match = re.search(r'^name\s*:\s*(.+)$', text, re.MULTILINE)
        if match:
            names[match.group(1).strip().strip('\'"')] = filename
    return names


def workflow_run_targets(text: str) -> list[str]:
    """Workflow names this file waits on via `on.workflow_run.workflows`.

    Hand-rolled for the same reason `parse_on` is: no YAML dependency. Handles
    both the block sequence and the inline list, and stops at the end of the
    `workflow_run:` mapping so a later `branches:`/`types:` list is not mistaken
    for workflow names.
    """
    targets: list[str] = []
    in_on = False
    in_workflow_run = False
    workflows_indent: int | None = None

    for line in text.splitlines():
        if not line.strip() or line.lstrip().startswith('#'):
            continue
        indent = len(line) - len(line.lstrip())
        stripped = line.strip()

        if re.match(r'^on\s*:', line):
            in_on = True
            in_workflow_run = False
            workflows_indent = None
            continue
        if not in_on:
            continue
        if indent == 0:
            break  # left the `on:` block

        if re.match(r'^workflow_run\s*:', stripped):
            in_workflow_run = True
            workflows_indent = None
            continue
        if not in_workflow_run:
            continue

        match = re.match(r'^workflows\s*:\s*(.*)$', stripped)
        if match:
            inline = match.group(1).strip()
            if inline.startswith('['):
                targets.extend(
                    t.strip().strip('\'"') for t in inline.strip('[]').split(',') if t.strip()
                )
                workflows_indent = None
            else:
                workflows_indent = indent
            continue

        if workflows_indent is None:
            # A sibling key (`types:`) resets the sequence scan.
            if re.match(r'^[A-Za-z_][\w-]*\s*:', stripped) and indent <= 4:
                in_workflow_run = False
            continue

        if stripped.startswith('- ') and indent > workflows_indent:
            targets.append(stripped[2:].strip().strip('\'"'))
        elif indent <= workflows_indent:
            workflows_indent = None

    return targets


def release_readiness_consistency() -> list[str]:
    """The aggregator's list and the workflow's list must say the same thing.

    `release-readiness.yml` declares which workflows it waits on (by display
    name); `release_readiness_aggregate.py` declares which workflows it verifies
    (by file name). They are two halves of one contract, and nothing else
    compares them — so a rename on one side silently narrows the release
    licence. Checked in both directions: a name verified but not waited on means
    the aggregate runs before that workflow finished, and a name waited on but
    not verified means it is never actually judged.
    """
    if not RELEASE_READINESS.exists() or not AGGREGATOR.exists():
        return []

    waited_on = set(workflow_run_targets(RELEASE_READINESS.read_text(encoding='utf-8')))
    required = re.findall(
        r'\(\s*"([\w.-]+\.ya?ml)"\s*,\s*"([^"]+)"\s*\)',
        AGGREGATOR.read_text(encoding='utf-8'),
    )
    verified = {label for _, label in required}

    problems = []
    for label in sorted(waited_on - verified):
        problems.append(
            f'release-readiness.yml waits on `{label}`, but '
            f'release_readiness_aggregate.py never verifies it'
        )
    for label in sorted(verified - waited_on):
        problems.append(
            f'release_readiness_aggregate.py verifies `{label}`, but '
            f'release-readiness.yml never waits on it'
        )
    for filename, label in required:
        if not (WORKFLOWS / filename).exists():
            problems.append(
                f'release_readiness_aggregate.py lists `{label}` as '
                f'.github/workflows/{filename}, which does not exist'
            )
    return problems


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
            cwd=ROOT, capture_output=True, encoding='utf-8', errors='replace',
            timeout=60, check=True,
        ).stdout
        names = {ln.split('refs/heads/', 1)[1] for ln in out.splitlines() if 'refs/heads/' in ln}
        if names:
            return names, None
    except (subprocess.SubprocessError, OSError):
        pass

    try:
        out = subprocess.run(
            ['git', 'for-each-ref', '--format=%(refname:short)', 'refs/heads', 'refs/remotes/origin'],
            cwd=ROOT, capture_output=True, encoding='utf-8', errors='replace',
            timeout=60, check=True,
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
    """Return (reachable, reasons_why_not, informational_notes).

    A workflow is reachable when something *automatic* can start it. Once the
    automatic triggers have been examined and none can fire, `workflow_dispatch`
    is accepted as a fallback -- but loudly, as a note, because a workflow that
    only a human can start is not what "liveness" is trying to establish.

    The important difference from the previous version is ordering: the
    branch-filter analysis below now actually runs. Previously a single
    `workflow_dispatch` short-circuited it, so a workflow whose every branch
    filter pointed at a deleted branch was indistinguishable from a healthy one.
    """
    keys = set(triggers)
    notes = dormant_notes(triggers, branches, branches_known)

    if keys & UNCONDITIONAL:
        return True, [], notes

    if not (keys & BRANCH_FILTERED):
        if keys & MANUAL_ONLY:
            notes.append(
                'no automatic trigger; fires only via workflow_dispatch '
                '(manual release tooling, assumed intentional)'
            )
            return True, [], notes
        return False, [f'no trigger can fire (found only: {sorted(keys) or "none"})'], notes

    # Every branch-filtered trigger whose patterns all match nothing is dormant.
    # The OR across *all* triggers is what matters: a workflow may legitimately
    # list `main` plus a retired release-branch pattern.
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
        'every branch filter matches no existing branch',
    ]
    if keys & MANUAL_ONLY:
        # Not reachable automatically, but a human can still start it. Report it
        # and pass: the alternative would red the pipeline for any retired
        # branch reference, which is a warning signal rather than a blocker.
        notes.append(
            'reachable only by hand via workflow_dispatch; no automatic trigger '
            'can fire - review whether this workflow still earns its keep'
        )
    else:
        reasons.append('and there is no workflow_dispatch/schedule/release/workflow_call')
        return False, reasons, notes

    return True, [], notes


def main() -> int:
    files = workflow_files()
    if not files:
        print('no workflow files found; nothing to check')
        return 0

    branches, warning = existing_branches()
    branches_known = bool(branches)
    if warning:
        print(f'note: {warning}')

    texts = {path.name: path.read_text(encoding='utf-8') for path in files}
    names = declared_names(texts)

    unreachable: list[tuple[str, list[str]]] = []
    dormant: list[tuple[str, list[str]]] = []
    unparsed: list[str] = []
    dangling: list[tuple[str, list[str]]] = []

    for path in files:
        text = texts[path.name]
        triggers = parse_on(text)
        if triggers is None:
            unparsed.append(path.name)
        else:
            ok, reasons, notes = evaluate(triggers, branches, branches_known)
            if not ok:
                unreachable.append((path.name, reasons))
            if notes:
                dormant.append((path.name, notes))

        missing = [target for target in workflow_run_targets(text) if target not in names]
        if missing:
            dangling.append((path.name, missing))

    print(f'scanned {len(files)} workflow(s) against {len(branches)} known branch(es)')
    for name, notes in dormant:
        for note in notes:
            print(f'  info: {name}: {note}')

    drifted = release_readiness_consistency()

    if unparsed:
        print('FAIL: no `on:` block found:')
        for name in unparsed:
            print(f'  - .github/workflows/{name}')

    if unreachable:
        print('FAIL: workflow(s) that can never be triggered:')
        for name, reasons in unreachable:
            for reason in reasons:
                print(f'  - .github/workflows/{name}: {reason}')
        print('  Add `workflow_dispatch`, or fix/remove the stale branch filter.')

    if dangling:
        print('FAIL: `workflow_run` waiting on a workflow name that does not exist:')
        for name, missing in dangling:
            for target in missing:
                print(f'  - .github/workflows/{name}: waits on `{target}`')
        print(
            "  GitHub matches `on.workflow_run.workflows` against the target "
            "workflow's `name:`; a typo makes the trigger dead."
        )

    if drifted:
        print('FAIL: release-readiness contract drift:')
        for problem in drifted:
            print(f'  - {problem}')

    total = len(unparsed) + len(unreachable) + len(dangling) + len(drifted)
    if total:
        print(f'FAIL: {total} problem(s)')
        return 1

    print('OK: every workflow has a trigger that can fire')
    return 0


if __name__ == '__main__':
    sys.exit(main())
