#!/usr/bin/env python3
"""Keep the formula engine's public contract and its docs in lock-step.

Why this exists
---------------
`docs/architecture/formula-engine.md` once described a **different engine** from
the one in the tree: it said "production execution goes through the compiled-plan
path" while `FormulaExecutionMode::default()` was — and is — `Tree`, and it
described the cache as one LRU while the engine actually holds several bounded
caches (one LRU AST cache plus FIFO plan/semantic/bytecode caches). Nothing
objected, because prose has no compiler.

The same drift already happened once to the *entry-point matrix*: the
`FormulaEngine` doc comment, the runtime contract and the test that is supposed
to check the split disagreed on how many tree-only entries exist and which ones
they are.

This gate is the compiler for those claims. It reads one source of truth —
`require_tree_backend("NAME")` call sites in `core/src/formula/engine.rs` — and
asserts four separate documents agree with it:

  1. the table in `docs/architecture/formula-engine.md`;
  2. the tree-only roster in `docs/formula-runtime-contract.md` (both the
     bullet list under "Tree-only 入口" and the "仅 tree" table row);
  3. the `assert_tree_only!("NAME", ..)` roster in
     `core/tests/formula_execution_mode.rs`;
  4. the enum default (`#[default]` must sit on `Tree`).

Document 2 is the one that drifted in practice: its §3.4 table omitted
`eval_multi_with_dialect(Pine)` while §1.1 of the same file listed it, so the
two rosters inside one document disagreed and nothing noticed. That is why it
is checked here rather than trusted.

A fifth assertion closes the loop the 2026-10-13 review asked for in its phase
1.2: every tree-only entry must carry a recorded **disposition** (`by-design`,
`backlog` or `deprecate`) in `docs/architecture/formula-entry-matrix.md` §1.2.1,
any suggested plan alternative must not itself be tree-only, the counts the prose
states must equal the table's own counts, and the plan doc's phase-1.2 bullets
must name exactly the guarded entries. Without it, a newly guarded entry — or an
undecided one — would silently pass as a contract; prose like "two `backlog`"
could survive a table that lists four; and a bullet could name `eval` (a
plan-capable entry) while omitting four that are genuinely tree-only.

A sixth assertion keeps phase 1.3's conclusions from being re-opened: the entry
matrix must still carry the §1.3.1 backend-explicit rule and the §3.1
range/last/into decision, and the "still to be defined" wording that preceded
them is a stale phrase. A recorded decision that can be quietly deleted is not
a contract either.

It also asserts the capacities written in the doc still match the constants the
code defines, so a cache-capacity change cannot land with stale prose.

Usage
-----
    python scripts/check_formula_engine_contract.py

Exit codes: 0 = consistent, 1 = drift, 2 = a source file could not be read.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ENGINE = ROOT / "core" / "src" / "formula" / "engine.rs"
TEST = ROOT / "core" / "tests" / "formula_execution_mode.rs"
DOC = ROOT / "docs" / "architecture" / "formula-engine.md"
RUNTIME_CONTRACT = ROOT / "docs" / "formula-runtime-contract.md"
ENTRY_MATRIX = ROOT / "docs" / "architecture" / "formula-entry-matrix.md"
PLAN_DOC = ROOT / "docs" / "ARCHITECTURE_REVIEW_AND_REFACTOR_PLAN_2026-10-13.md"
BINDING_ROOT = ROOT / "ffi"

# A binding that can select the backend contains one of these. The entry matrix
# §5 must report the capability of each binding accurately, rather than freezing
# the historical claim that none of them can choose a backend.
BINDING_BACKEND_SELECTOR = re.compile(
    r"\b(?:with_execution_mode|set_execution_mode|FormulaExecutionMode)\b"
)
BINDING_BACKEND_REPORT = re.compile(r"\bformula_backend\b|\bformulaBackend\b")

# The controlled vocabulary the entry matrix must use for each tree-only entry.
# Phase 1.2 of the 2026-10-13 review: every guarded entry needs a recorded
# decision, so "not decided yet" cannot masquerade as a contract.
DISPOSITIONS = {"by-design", "backlog", "deprecate"}

# Phrases that described the pre-2026-09 engine and must not come back. The
# last two describe the string-literal blocker as unclosable; it was closed on
# 2026-10-04 (see core/tests/formula_string_literals.rs), so a document that
# claims a literal-bearing formula "cannot run under Plan at all" is stale.
STALE_PHRASES = (
    "Production execution goes through the compiled-plan path",
    "LRU eviction bounds the working set",
    "cannot run under `Plan` at all",
    "with no context to append to",
    # Phase 1.3 landed a decision for range/last/into and a governing rule for the
    # backend-explicit split. Both used to be written as open questions; leaving
    # the old wording would re-open a question that is now answered.
    "阶段 1 需要显式界定",
    "不擅自改动公开契约",
)

# The entry matrix must carry the phase-1.3 conclusions, not just the facts. These
# are the section anchors whose presence proves the decision was recorded rather
# than deferred; the exact prose may change, the anchor must not.
ENTRY_MATRIX_ANCHORS = (
    "#### 1.3.1",
    "### 3.1",
    "### 5.1",
    "### 5.2",
)

REQUIRE_TREE = re.compile(r'require_tree_backend\("([^"]+)"\)')
ASSERT_TREE = re.compile(r'assert_tree_only!\(\s*"([^"]+)"')
ENUM_DEFAULT_TREE = re.compile(r"#\[default\]\s*\n\s*Tree,")
CACHE_CAPACITY = re.compile(r"const ENGINE_CACHE_CAPACITY: usize = (\d+);")
AST_CACHE_CAPACITY = re.compile(r"FormulaCache::new\((\d+)\)")
EVAL_TOKEN = re.compile(r"`(eval[^`]*)`")


def read(path: Path) -> str:
    if not path.is_file():
        print(f"FAILED: missing {path.relative_to(ROOT)}", file=sys.stderr)
        raise SystemExit(2)
    return path.read_text(encoding="utf-8", errors="replace")


def doc_tree_only_entries(text: str) -> set[str]:
    """Backticked `eval*` names from the doc's 'Governed, tree-only' row."""
    for line in text.splitlines():
        if "Governed, tree-only" in line:
            return {
                token
                for token in re.findall(r"`([^`]+)`", line)
                if token.startswith("eval")
            }
    return set()


def runtime_contract_tree_only_rosters(text: str) -> dict[str, set[str]]:
    """The runtime contract's two tree-only rosters, kept *separate*.

    That document states the roster twice: a bullet list introduced by
    "Tree-only 入口" and a table row marked "仅 tree". They are returned as two
    independent sets rather than a union on purpose: the drift this gate exists
    to catch was §3.4's table omitting `eval_multi_with_dialect(Pine)` while
    §1.1 of the same file listed it. A union would have hidden exactly that —
    the union still equals the code set. Each roster must match the code on its
    own.

    The bullet roster is separated from its heading by a blank line and ends on
    the sentence-final "。" — a naive "read until the next non-token line" would
    either stop at the blank line (reading nothing) or run on into the
    "Plan-capable" paragraph below it. The terminator is therefore explicit.
    """
    lines = text.splitlines()
    rosters: dict[str, set[str]] = {"Tree-only 入口": set(), "仅 tree": set()}
    for index, line in enumerate(lines):
        if "仅 tree" in line:  # single-line table roster
            rosters["仅 tree"] |= set(EVAL_TOKEN.findall(line))
            continue
        if "Tree-only 入口" in line:  # multi-line bullet roster
            started = False
            for follow in lines[index + 1:]:
                tokens = set(EVAL_TOKEN.findall(follow))
                if tokens:
                    rosters["Tree-only 入口"] |= tokens
                    started = True
                    if follow.rstrip().endswith("。"):
                        break
                elif started:
                    break
    return rosters


def plan_doc_phase_1_2_roster(text: str) -> set[str]:
    """Entry names the plan doc's phase-1.2 bullets claim to dispose.

    Reads only the `### 阶段 1.2` section, and only the bullet lines that open
    with a controlled-vocabulary disposition, plus their indented continuation
    lines. Within each bullet, only the part *before* the `——` separator counts:
    that is the roster, and everything after it is rationale, which legitimately
    names plan-capable entries such as `eval_with_params` for contrast. Prose
    paragraphs outside the bullets are excluded for the same reason.
    """
    lines = text.splitlines()
    roster: set[str] = set()
    inside = False
    bullet: list[str] = []

    def flush() -> None:
        if bullet:
            head = "".join(bullet).split("——", 1)[0]
            roster.update(EVAL_TOKEN.findall(head))
            bullet.clear()

    for line in lines:
        if line.startswith("### 阶段 1.2"):
            inside = True
            continue
        if inside and line.startswith("### "):
            break
        if not inside:
            continue
        opens_bullet = any(
            line.startswith(f"- `{name}`（") for name in sorted(DISPOSITIONS)
        )
        if opens_bullet:
            flush()
            bullet.append(line)
        elif line.startswith("  ") and bullet:
            bullet.append(line)
        else:
            flush()
    flush()
    return roster


def binding_crates() -> list[Path]:
    """Every `ffi/*-binding` crate, sorted, whether or not it is documented."""
    if not BINDING_ROOT.is_dir():
        return []
    return sorted(
        path
        for path in BINDING_ROOT.iterdir()
        if path.is_dir() and path.name.endswith("-binding")
    )


def binding_matrix_rows(text: str) -> dict[str, tuple[bool, bool]]:
    """Binding backend selection/report claims from the §5 capability table."""
    rows: dict[str, tuple[bool, bool]] = {}
    inside = False
    for line in text.splitlines():
        if line.startswith("## 5."):
            inside = True
            continue
        if inside and line.startswith("## "):
            break
        if not inside or not line.startswith("|"):
            continue
        cells = [cell.strip() for cell in line.strip().strip("|").split("|")]
        if len(cells) != 4:
            continue
        name = cells[0].strip("`")
        if re.fullmatch(r"[a-z0-9-]+-binding", name):
            rows[name] = ("✅" in cells[1], "✅" in cells[2])
    return rows


def disposition_rows(text: str) -> dict[str, tuple[str, str]]:
    """Parse the entry matrix's `§1.2.1` disposition table.

    Returns `entry -> (disposition, plan-alternative-cell)`. Rows are read only
    inside the `#### 1.2.1` section, so the capability matrix and the
    backend-explicit table below it cannot be mistaken for dispositions. A row
    counts only when its first cell is exactly one backticked token, which skips
    the header and the `|---|` separator.
    """
    rows: dict[str, tuple[str, str]] = {}
    inside = False
    for line in text.splitlines():
        if line.startswith("#### 1.2.1"):
            inside = True
            continue
        if inside and (line.startswith("### ") or line.startswith("## ")):
            break
        if not inside or not line.startswith("|"):
            continue
        cells = [cell.strip() for cell in line.strip().strip("|").split("|")]
        if len(cells) != 4:
            continue
        name = re.fullmatch(r"`([^`]+)`", cells[0])
        if not name:
            continue
        # The disposition is written as `by-design` (backticked for emphasis in
        # the rendered table), so the code span is stripped before comparison.
        rows[name.group(1)] = (cells[1].strip("`"), cells[3])
    return rows


def report(label: str, code: set[str], other: set[str]) -> list[str]:
    problems = []
    if code - other:
        problems.append(f"{label} is missing: {sorted(code - other)}")
    if other - code:
        problems.append(f"{label} lists entries the code does not guard: {sorted(other - code)}")
    return problems


def main() -> int:
    engine = read(ENGINE)
    test = read(TEST)
    doc = read(DOC)
    runtime_contract = read(RUNTIME_CONTRACT)
    entry_matrix = read(ENTRY_MATRIX)
    plan_doc = read(PLAN_DOC)

    code_entries = set(REQUIRE_TREE.findall(engine))
    if not code_entries:
        print("FAILED: no require_tree_backend(\"..\") sites found — did the guard move?")
        return 2

    failures: list[str] = []

    # 1. The enum default must still be Tree.
    if not ENUM_DEFAULT_TREE.search(engine):
        failures.append(
            "FormulaExecutionMode's #[default] is no longer on `Tree`; "
            "flipping the default is a release decision that must update this gate "
            "and docs/architecture/formula-engine.md together"
        )

    # 2. Doc + test must agree with the guarded entry set.
    failures += report(
        "docs/architecture/formula-engine.md",
        code_entries,
        doc_tree_only_entries(doc),
    )
    failures += report(
        "core/tests/formula_execution_mode.rs",
        code_entries,
        set(ASSERT_TREE.findall(test)),
    )

    # 2b. Both rosters must be present *and* individually agree with the code.
    rosters = runtime_contract_tree_only_rosters(runtime_contract)
    for marker, roster in rosters.items():
        if not roster:
            failures.append(
                f"docs/formula-runtime-contract.md no longer contains the {marker!r} "
                f"tree-only roster; this gate would silently stop checking it"
            )
            continue
        failures += report(
            f"docs/formula-runtime-contract.md [{marker}]",
            code_entries,
            roster,
        )

    # 3. No stale prose in either contract document.
    for path, text in (
        ("docs/architecture/formula-engine.md", doc),
        ("docs/formula-runtime-contract.md", runtime_contract),
        ("docs/architecture/formula-entry-matrix.md", entry_matrix),
    ):
        for phrase in STALE_PHRASES:
            if phrase in text:
                failures.append(f"{path} still contains stale text: {phrase!r}")

    # 3a. The entry matrix must record the phase-1.3 decisions, not just the raw
    # backend facts. Without the anchors, a future edit could delete the
    # conclusion while every entry still "has" a disposition.
    for anchor in ENTRY_MATRIX_ANCHORS:
        if anchor not in entry_matrix:
            failures.append(
                f"docs/architecture/formula-entry-matrix.md is missing the "
                f"phase-1.3 anchor {anchor!r}; the range/last/into boundary and the "
                f"backend-explicit rule must stay recorded"
            )

    # 3b. Every tree-only entry must carry a recorded disposition, and the
    # disposition table must not invent entries the code does not guard. This is
    # what stops "we have not decided yet" from passing as a contract.
    dispositions = disposition_rows(entry_matrix)
    if not dispositions:
        failures.append(
            "docs/architecture/formula-entry-matrix.md has no §1.2.1 disposition "
            "table; every tree-only entry must record what happens to it"
        )
    for entry in sorted(code_entries - set(dispositions)):
        failures.append(
            f"tree-only entry `{entry}` has no disposition in "
            f"docs/architecture/formula-entry-matrix.md §1.2.1"
        )
    for entry in sorted(set(dispositions) - code_entries):
        failures.append(
            f"docs/architecture/formula-entry-matrix.md §1.2.1 disposes `{entry}`, "
            f"which the engine does not guard with require_tree_backend"
        )
    for entry, (disposition, alternative) in sorted(dispositions.items()):
        if disposition not in DISPOSITIONS:
            failures.append(
                f"`{entry}` has disposition {disposition!r}, which is not one of "
                f"{sorted(DISPOSITIONS)}"
            )
        # A suggested alternative that is itself tree-only would just move the
        # caller from one BackendUnsupported to the next.
        named = re.search(r"`([^`]+)`", alternative)
        if named and named.group(1) in code_entries:
            failures.append(
                f"`{entry}`'s plan alternative `{named.group(1)}` is itself "
                f"tree-only; the alternative must be a plan-capable entry"
            )

    # 3c. The counts the §1.2.1 prose states must equal the table's own counts.
    # The prose once said "two `backlog`" while the table listed four, and every
    # other check still passed because each row was individually valid.
    counts = {name: 0 for name in DISPOSITIONS}
    for disposition, _ in dispositions.values():
        if disposition in counts:
            counts[disposition] += 1
    for name in sorted(DISPOSITIONS):
        stated = re.search(rf"`{re.escape(name)}`\s*(\d+)", entry_matrix)
        if not stated:
            failures.append(
                f"docs/architecture/formula-entry-matrix.md §1.2.1 states no count "
                f"for `{name}`; add a `计数` line so prose cannot drift from the table"
            )
            continue
        if int(stated.group(1)) != counts[name]:
            failures.append(
                f"docs/architecture/formula-entry-matrix.md says `{name}` "
                f"{stated.group(1)}, but the table has {counts[name]}"
            )
    total_stated = re.search(r"合计\s*(\d+)", entry_matrix)
    if total_stated and int(total_stated.group(1)) != len(dispositions):
        failures.append(
            f"docs/architecture/formula-entry-matrix.md says the dispositions total "
            f"{total_stated.group(1)}, but {len(dispositions)} rows were parsed"
        )

    # 3d. The plan doc's phase-1.2 bullets must name exactly the guarded entries.
    # This caught a real error: the `by-design` bullet listed `eval` (a
    # plan-capable §1.1 entry) and `eval_range` (backend-explicit §1.3), neither of
    # which is in the tree-only roster, while omitting four that are.
    plan_roster = plan_doc_phase_1_2_roster(plan_doc)
    if not plan_roster:
        failures.append(
            "docs/ARCHITECTURE_REVIEW_AND_REFACTOR_PLAN_2026-10-13.md has no "
            "phase-1.2 disposition bullets naming the tree-only entries"
        )
    for entry in sorted(code_entries - plan_roster):
        failures.append(
            f"tree-only entry `{entry}` is not named in the plan doc's phase-1.2 "
            f"disposition bullets"
        )
    for entry in sorted(plan_roster - code_entries):
        failures.append(
            f"the plan doc's phase-1.2 disposition bullets name `{entry}`, which the "
            f"engine does not guard with require_tree_backend"
        )

    # 3e. The §5 binding matrix must list exactly the binding crates on disk and
    # its selection/report claims must match the Rust source. This lets active
    # bindings expose an opt-in backend without making the matrix stale.
    documented_bindings = binding_matrix_rows(entry_matrix)
    actual_bindings = {path.name for path in binding_crates()}
    if not documented_bindings:
        failures.append(
            "docs/architecture/formula-entry-matrix.md has no §5 binding capability "
            "table; every binding's backend-select/report status must be recorded"
        )
    for name in sorted(actual_bindings - documented_bindings.keys()):
        failures.append(
            f"binding crate `ffi/{name}` is missing from the entry matrix §5 table"
        )
    for name in sorted(documented_bindings.keys() - actual_bindings):
        failures.append(
            f"the entry matrix §5 table lists `{name}`, which is not an ffi/*-binding crate"
        )
    for crate in binding_crates():
        selected = False
        reported = False
        for source in sorted((crate / "src").rglob("*.rs")):
            text = source.read_text(encoding="utf-8", errors="replace")
            # Skip comment-only lines: prose *about* the backend (the C header
            # documents error code 11 that way) is not a selector, and flagging it
            # would train people to ignore this check.
            code = "\n".join(
                line for line in text.splitlines() if not line.lstrip().startswith("//")
            )
            selected |= bool(BINDING_BACKEND_SELECTOR.search(code))
            reported |= bool(BINDING_BACKEND_REPORT.search(code))
        if crate.name in documented_bindings:
            expected_selection, expected_report = documented_bindings[crate.name]
            if selected != expected_selection:
                failures.append(
                    f"ffi/{crate.name} backend selection is {selected}, but the entry "
                    f"matrix §5 records {expected_selection}"
                )
            if reported != expected_report:
                failures.append(
                    f"ffi/{crate.name} backend reporting is {reported}, but the entry "
                    f"matrix §5 records {expected_report}"
                )

    # 4. Cache capacities in the doc track the constants.
    for match in CACHE_CAPACITY.findall(engine):
        if f"= {match}" not in doc and match not in doc:
            failures.append(f"doc does not mention ENGINE_CACHE_CAPACITY value {match}")
    for match in AST_CACHE_CAPACITY.findall(engine):
        if match not in doc:
            failures.append(f"doc does not mention the AST cache capacity {match}")

    if failures:
        print(f"Formula-engine contract drift ({len(failures)}):")
        for line in failures:
            print(f"  - {line}")
        return 1

    print(
        f"OK: {len(code_entries)} tree-only entries agree across the engine guard, "
        f"docs/architecture/formula-engine.md, docs/formula-runtime-contract.md "
        f"(both rosters) and formula_execution_mode.rs; all {len(dispositions)} carry "
        f"a §1.2.1 disposition; default is Tree; cache capacities match the constants"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
