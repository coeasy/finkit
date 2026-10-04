# Archived documents

These documents are **historical records, not current guidance**. They are kept for audit
trail only — nothing here should be treated as a description of how the code works today.

Moved here on 2026-09-21 (see `docs/refactor-plan-2026-09-21.md` Phase 6).
A second batch was moved on 2026-10-04: the 2026-09-26 optimization plan, its companion
audit and the `upgrade-completion-matrix-zh.md` progress snapshot were superseded by the
V4 plan and its audit rounds.

> **Restored from git after an accidental working-tree deletion.** `git rm` on files that
> carry staged changes (as every `git mv` target does) refuses, but a force-run alongside
> it cleared the directory. Recovery was possible only because the moves were staged;
> `docs/archive/README.md` itself was untracked and had to be rewritten.

## Reading rules

- **The single current execution baseline is
  `docs/FINKIT_ARCHITECTURE_AND_OPTIMIZATION_PLAN_V4.md`.** Every other plan in this
  repository is a record of a past round.
- `docs/refactor-plan-2026-09-21.md` is **not** an execution baseline. It is retained
  outside the archive because it is the master record of the user-confirmed product
  boundary and constraints (no backtesting, no stock selection, JIT/`eval_simd` frozen),
  which V4 builds on. Read it for *constraints*, not for current status.
- Where an archived document conflicts with current code, **the code wins**. Several of
  these were written before modules they describe were deleted.
- Cross-references inside this directory were rewritten to point at `../`; links from
  live documents into this directory use `archive/`.

## Deleted, not archived

The three V4 documents were removed outright on 2026-09-21 rather than archived. They
described a runtime engine (`runtime_engine.rs` + `runtime/`) that Phase 1.1 deleted, so
keeping them would have preserved a description of something that no longer exists:

| Document | Why it was deleted |
|---|---|
| `ARCHITECTURE_V4_PLAN.md` | V4 runtime plan; the target was removed. |
| `V4_MIGRATION_EXECUTION_STATUS.md` | Execution status for that same removed migration. |
| `finkit-architecture-review-refactor-plan-v4.md` | Same V4 target architecture. |

**The names are not related.** Those three were *architecture* V4 plans aimed at
`runtime_engine.rs`, which no longer exists. `docs/FINKIT_ARCHITECTURE_AND_OPTIMIZATION_PLAN_V4.md`
is the *current* plan and is deliberately **not** archived: it is indexed under "Current refactor
baseline" in `docs/README.md`. Reach for that one when you want current guidance.

## Index

| Document | Round | Status |
|---|---|---|
| `pr28-architecture-v3-refactor-plan.md` | PR #28, v3 | Superseded |
| `ARCHITECTURE_REVIEW_AND_REFACTOR_PLAN_2026-09-17.md` | 2026-09-17 | Superseded |
| `ARCHITECTURE_REVIEW_AND_REFACTOR_PLAN_2026-09-18.md` | 2026-09-18 | Superseded |
| `FINAL_REFACTOR_PLAN_2026-09-19.md` | 2026-09-19 | Superseded (despite the name) |
| `finkit-optimal-architecture-refactor-plan-v5.md` | V5 | Superseded |
| `finkit-unified-quant-platform-refactor-plan-v6.md` | V6 | Superseded |
| `finkit-performance-architecture-v2-plan.md` | perf v2 | Superseded |
| `architecture-gap-assessment-2026-09-20.md` | 2026-09-20 | Evidence for the 09-21 plan |
| `improvement-plan-2026-09-20.md` | 2026-09-20 | Superseded by the 09-21 plan |
| `factor-research-expansion-plan-v2.md` | v2 | Explicitly superseded by `docs/factor-research-architecture.md` |
| `FINKIT_QUANT_FACTOR_ENGINE_ARCHITECTURE_V2.md` | V2 | Superseded by later architecture rounds |
| `release-v0.1.5.md` | v0.1.5 | Release checklist for a published version; superseded by `docs/release-checklist.md` |
| `finkit-architecture-v3.1-implementation-plan.md` | v3.1 | Superseded by `docs/FINKIT_ARCHITECTURE_AND_OPTIMIZATION_PLAN_V4.md`; was misnamed `docs/new.md` |
| `IMPROVEMENT_PLAN.md` | 2026-05-24 (AlphaTA era) | Industrial-grade improvement plan written under the repository's pre-rename name; superseded by later rounds |
| `benchmark-baseline.md` | 2026-05-27 (AlphaTA era) | AlphaTA-vs-TA-Lib baseline; superseded by `docs/BENCHMARK_VS_TALIB.md` and `docs/benchmark-baseline.json` |
| `benchmark-results.md` | 2026-05-27 (AlphaTA era) | AlphaTA optimization report, single machine; superseded by `docs/benchmark-results.md` |
| `ALPHATA_VS_TALIB.md` | 2026-07-08 (AlphaTA era) | Early single-machine comparison; superseded by `docs/BENCHMARK_VS_TALIB.md` |
| `ALPHATA_VS_TALIB_REPORT.md` | 2026-07-11 (AlphaTA era) | Same class of report, later run |
| `ALPHATA_VS_TALIB_COMPARISON_REPORT.md` | 2026-07-12 (AlphaTA era) | Same class of report, last run under the old name |
| `refactor-plan-2026-09-26.md` | 2026-09-26 | Superseded by `docs/FINKIT_ARCHITECTURE_AND_OPTIMIZATION_PLAN_V4.md` |
| `architecture-and-feature-audit-2026-09-26.md` | 2026-09-26 | Dated audit; conclusions superseded by the V4 audit rounds (§28–§30) |
| `upgrade-completion-matrix-zh.md` | — | Stale progress snapshot (still claimed live `bytecode`/`JIT` paths after they were frozen); superseded by the generated SSOT catalogues |

## Deliberately not archived

| Document | Reason |
|---|---|
| `docs/finkit-outperform-talib-architecture-v3.md` | `scripts/benchmark_talib_arch_v3_gate.py` encodes its speedup semantics — it is a live spec, not a past plan. Indexed in `docs/README.md`. |
| `docs/runtime-carrier-adoption-plan-2026-09-20.md` | Still the R2/R3/R4 adoption spec; R4 is in progress. Indexed in `docs/README.md`. |
| `docs/finkit-vs-talib-performance-optimization-plan.md` | Backs `scripts/apply_talib_performance_plan.py` and its workflow. Indexed in `docs/README.md`. |
| `docs/talib-0.8.0-coverage-audit-2026-09-19.md` | Factual coverage snapshot, indexed in `docs/README.md`. |
| `docs/refactor-plan-2026-09-21.md` | Master record of the user-confirmed product boundary and constraints. **Not** an execution baseline — current status lives in `docs/FINKIT_ARCHITECTURE_AND_OPTIMIZATION_PLAN_V4.md`. Indexed in `docs/README.md`. |
| `docs/competitive-analysis/*` | Dated competitive analyses, not architecture plans. Indexed under "Dated analyses and roadmaps" in `docs/README.md`. |
| `docs/*-zh.md` roadmaps and progress matrices | Dated proposals/snapshots, not architecture plans. Indexed under "Dated analyses and roadmaps" in `docs/README.md`. |
