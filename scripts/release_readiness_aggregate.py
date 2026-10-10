#!/usr/bin/env python3
"""Release readiness aggregator (V4 plan §22 / P0-07).

"Same SHA, all required workflows green" is the only release license. Before
this aggregator existed, `CI`, `Python wheels`, the installed-wheel gate and
the multilang pipelines could disagree about the *same* commit, and a tag
could be cut from a tree whose wheel build was actually red.

Given a commit SHA, this script queries the GitHub Actions API for the latest
run of every required workflow on exactly that SHA and fails (NO RELEASE)
unless every one of them is `success`. A missing run is a failure by default:
absence of evidence is not readiness. Use `--allow-missing` while a workflow
is being onboarded, never to bless a release.

Usage:
    python scripts/release_readiness_aggregate.py <sha> [--allow-missing]

Requires `GH_TOKEN` (and `gh` or network access to api.github.com).
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import urllib.request
from pathlib import Path

# (workflow_file, human name). Keep in sync with §22 of
# the V4 plan (archived in git history: `git log --diff-filter=D --
# docs/FINKIT_ARCHITECTURE_AND_OPTIMIZATION_PLAN_V4.md`).
REQUIRED_WORKFLOWS = [
    ("ci.yml", "CI"),
    ("docs-check.yml", "Docs Check"),
    ("python-wheels.yml", "Python wheels"),
    ("talib-release-gate.yml", "TA-Lib installed-wheel release gate"),
    ("multilang-cross-platform.yml", "Multilang cross-platform"),
    ("multilang-release.yml", "Multilang release"),
    ("competitive-benchmark.yml", "Competitive benchmark"),
    ("release-installers.yml", "Release installers"),
]


def repo() -> tuple[str, str]:
    origin = subprocess.run(
        ["git", "remote", "get-url", "origin"], check=True, capture_output=True, text=True
    ).stdout.strip()
    for prefix in ("git@github.com:", "https://github.com/"):
        if origin.startswith(prefix):
            path = origin[len(prefix) :].removesuffix(".git")
            owner, name = path.split("/")
            return owner, name
    raise SystemExit(f"cannot parse origin remote: {origin}")


def api(url: str) -> dict:
    token = os.environ.get("GH_TOKEN") or os.environ.get("GITHUB_TOKEN")
    request = urllib.request.Request(url)
    if token:
        request.add_header("Authorization", f"Bearer {token}")
    with urllib.request.urlopen(request, timeout=30) as response:
        return json.load(response)


def latest_run(owner: str, name: str, sha: str, workflow_file: str) -> dict | None:
    url = (
        f"https://api.github.com/repos/{owner}/{name}/actions/workflows/{workflow_file}"
        f"/runs?head_sha={sha}&per_page=1"
    )
    data = api(url)
    runs = data.get("workflow_runs") or []
    return runs[0] if runs else None


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("sha", help="commit SHA whose readiness is being judged")
    parser.add_argument(
        "--allow-missing",
        action="store_true",
        help="treat a workflow with no run on this SHA as skipped instead of failing",
    )
    args = parser.parse_args()

    owner, name = repo()
    failures = []
    missing = []
    print(f"release readiness for {owner}/{name}@{args.sha[:12]}")
    for workflow_file, label in REQUIRED_WORKFLOWS:
        try:
            run = latest_run(owner, name, args.sha, workflow_file)
        except Exception as error:  # noqa: BLE001 - API errors are readiness failures
            failures.append(f"{label}: API error ({error})")
            continue
        if run is None:
            (missing if args.allow_missing else failures).append(
                f"{label}: no run on this SHA"
            )
            continue
        conclusion = run.get("conclusion")
        status = run.get("status")
        if status != "completed":
            failures.append(f"{label}: still {status}")
        elif conclusion != "success":
            failures.append(f"{label}: {conclusion}")
        else:
            print(f"  PASS  {label}")

    for item in missing:
        print(f"  SKIP  {item}")
    for item in failures:
        print(f"::error {item}")

    if failures:
        print("RELEASE READINESS: NO RELEASE")
        return 1
    print("RELEASE READINESS: PASS (all required checks succeeded on this SHA)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
