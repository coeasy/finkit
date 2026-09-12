# Finkit Architecture Review & Refactor Plan V4

> Status: Draft for architecture decision review
>
> Review baseline: `feat/factor-research-architecture` / PR #29, head `cc2f245f9a3ec6cdf7516a7fe312d72d9ca29b04`
>
> Date: 2026-09-11
>
> Scope: architecture, major functions, execution chains, implementation ownership, release gates, and staged refactoring. This document does **not** expand Finkit into market-data ingestion, broker integration, order execution, or a full trading platform.

---

## 1. Executive conclusion

Finkit has evolved from a TA-Lib-compatible indicator library into a broader **quantitative compute and research infrastructure**. The repository now contains:

- a high-performance Rust indicator/math kernel;
- Formula / FormulaGraph / ComputePlan based expression and execution infrastructure;
- batch, streaming, feature, transform, risk, portfolio and financial-computation capabilities;
- a new `finkit-factor-analysis` research layer with Alphalens-style semantics;
- GPU/web-oriented visualization;
- CLI and multi-language / multi-platform bindings;
- CI, performance and release gates spanning the workspace and bindings.

The project is therefore no longer correctly described as only a technical-indicator library. The proper product boundary is:

> **A reusable quantitative calculation + factor research engine, with stable multi-language contracts, but without owning data acquisition or trade execution.**

### 1.1 Is every core function complete?

**No, not under a production-ready definition.**

Most major *functional modules* are already present, and the native Rust factor-study happy path is implemented. However, the current branch cannot yet be classified as fully complete because:

1. the latest PR #29 head is not green on all required CI/release gates;
2. the core suite has a real `HT_SINE` throughput gate failure on the current checked-in head (while functional tests are otherwise broadly healthy);
3. Format, Visualization integration and Research SSOT gates are still red on the checked-in head;
4. Node and Go release jobs are still red in the multilang release workflow;
5. the research semantic DAG (`ResearchPlan`) is not yet the authoritative execution path;
6. incremental execution, research cache, report production, visualization and FFI are not yet unified behind one execution/materialization chain;
7. several advanced research services exist, but are standalone services rather than declaratively composed stages of a standard/custom research pipeline.

The right status is therefore:

> **Function coverage: high. Main Rust happy path: connected. Target architecture: partially connected. Release completeness: not reached.**

### 1.2 Is the main chain fully connected?

The answer is layered:

| Chain | Status | Assessment |
|---|---|---|
| Math / indicator canonical kernels | Mostly connected | Mature, broad coverage and tests; remaining performance/SSOT cleanup exists. |
| Formula → graph → ComputePlan → runtime | Connected | This is one of the most mature architectural areas. |
| Native factor-study happy path | Connected | Request → panel → forward returns → quantization → analytics → portfolio performance → report works. |
| Advanced factor research services | Implemented, partially orchestrated | Multifactor, validation, event, mining, stability, risk-model and scenario capabilities exist but are not all part of one declarative executor path. |
| ResearchPlan → actual execution | **Not fully connected** | ResearchPlan compiles semantic dependencies, while `FactorStudy::full_report()` still manually invokes stages. |
| Incremental → executor/cache | **Not fully connected** | Incremental forward-return maturity exists, but uses a parallel state path rather than a shared executor invalidation model. |
| Cache → typed research artifacts | **Not complete** | Cache is currently narrow (`Vec<f64>` values and a simple key), insufficient for heterogeneous research outputs. |
| Report → visualization | Partially connected | Reporting and visualization exist, but the report/artifact contract is not yet the sole source consumed by visualization. |
| Rust research → FFI JSON bridge | Mostly connected | Central research JSON bridge is a good SSOT direction. |
| Typed/low-copy parity across all bindings | **Not complete** | Semantics are largely centralized, but ergonomic and copy-cost parity varies by binding. |
| Release pipeline | **Not complete** | Current branch still has red required jobs. |

---

## 2. Current architecture map

### 2.1 Workspace level

At a high level the current repository can be viewed as:

```text
                           +-----------------------+
                           |      End users        |
                           +-----------+-----------+
                                       |
                  +--------------------+--------------------+
                  |                    |                    |
                CLI               Language SDKs        Visualization
                  |                    |                    |
                  +--------------------+--------------------+
                                       |
                               API / FFI adapters
                                       |
                     +-----------------+------------------+
                     |                                    |
             factor-analysis                         core / finkit
                     |                                    |
                     +--------------------+---------------+
                                          |
                             canonical compute kernels
```

The important dependency direction is currently reasonable:

```text
factor-analysis -> finkit(core)
```

Research functions can reuse canonical math, ranking, regression, returns, feature and runtime primitives instead of reimplementing them.

### 2.2 `core`: current responsibility set

`core` currently owns a large number of domains:

- math/statistics;
- technical indicators;
- streaming indicators/state;
- Formula parser/execution;
- FormulaGraph;
- ComputePlan;
- generic runtime execution;
- features and transforms;
- normalization/calendar utilities;
- risk/performance/backtest/financial functions;
- factor registry and related primitives.

This has produced a strong reusable kernel, but it also means the crate has become a **broad ownership boundary**. The problem is not its size alone; the problem is that new modules may not know where canonical semantics belong.

### 2.3 `factor-analysis`: current responsibility set

The current research crate exposes modules covering:

```text
analysis
api
cache
catalog
compat
data
error
evaluation_api
event
incremental
mining
multifactor
orchestration
performance
portfolio
portfolio_performance
prepare
provenance
report
research_ops
risk_model
scenario
stability
validation
```

The intended dependency direction is good: this layer should own **research semantics and workflow composition**, while numerical algorithms that are broadly reusable should remain canonical in the core/kernel layer.

### 2.4 FFI and language adapters

The repository supports a broad adapter surface around the Rust implementation. The currently relevant architecture is:

```text
Language binding
    -> shared FFI / research bridge
        -> versioned research request
            -> FactorStudy
                -> native Rust analysis
                    -> JSON/report/error result
```

Centralizing research behavior in Rust rather than reimplementing Alphalens/factor analytics in every binding is correct. The weakness is that the common FFI layer now imports high-level research dependencies and uses JSON strings as the main common transport, so its name and responsibility no longer match a truly low-level `ffi-common` abstraction.

---

## 3. Major functional inventory

### 3.1 Technical indicator and quantitative kernel

Status: **highly implemented / mature**.

Main capabilities include:

- TA-Lib-style indicators and mathematical operators;
- rolling/statistical functions;
- trend, momentum, volatility, volume and cycle families;
- streaming/stateful calculation paths;
- reusable returns/rank/regression/statistical kernels;
- SIMD and hot-path optimizations in selected indicators;
- parity/performance testing infrastructure.

Main remaining concern is not broad functional absence but **canonical ownership + performance convergence**. The current `HT_SINE` throughput failure is an example: correctness can be good while release performance criteria are still not satisfied.

### 3.2 Formula and execution infrastructure

Status: **implemented and structurally mature**.

Important pieces include:

- FormulaEngine;
- FormulaGraph;
- ComputePlan;
- dependency/effect metadata;
- graph scheduling;
- cache/fusion/incremental execution primitives;
- shared feature/operator execution.

This should remain the lower-level execution substrate. Research semantics should reuse it where useful, but should **not** be forced into FormulaRequest syntax or become Formula-specific.

### 3.3 Factor data preparation

Status: **implemented**.

Current research data model includes panel-aware structures such as `PanelIndex` / `ResearchFrame`, horizon-based forward returns and quantization. The main path prepares:

```text
panel data
  -> validate/index
  -> forward returns
  -> horizon maturity
  -> quantiles
  -> prepared factor data
```

This is a solid foundation for Alphalens-style analysis.

### 3.4 Factor analytics

Status: **implemented**.

Native services include:

- factor weights;
- factor returns;
- information coefficient;
- mean IC;
- mean returns by quantile;
- factor alpha/beta;
- quantile turnover;
- factor rank autocorrelation;
- group-adjusted / demeaned analysis paths.

The major architectural issue is that multiple analytics repeat similar date/group cross-sectional traversals. These semantics should be centralized into reusable cross-section views/executors to reduce allocation, sorting and policy drift.

### 3.5 Multifactor analytics

Status: **implemented as services, partially integrated**.

Capabilities include:

- factor correlation matrices;
- VIF;
- PCA reuse;
- cross-sectional neutralization;
- Fama-MacBeth style analysis.

These functions should not necessarily run in every basic factor report. The missing architecture is an explicit **pipeline/profile model** deciding which services participate in a study.

### 3.6 Portfolio/performance layer

Status: **implemented**.

The research branch includes overlapping holding-period portfolio logic plus cost/capacity-related models and performance outputs. This is an appropriate research layer capability as long as it remains analytical/simulated and is not coupled to live execution.

### 3.7 Validation and inference

Status: **implemented with excessive module cohesion**.

The current validation area contains multiple distinct concepts, including:

- Purged K-Fold;
- embargo validation;
- CPCV;
- walk-forward;
- interval purge;
- bootstrap / permutation utilities;
- multiple-testing helpers.

They belong to one broad domain but should not remain a single large implementation unit. Internal modularization is needed.

### 3.8 Event / stability / mining / risk / scenario

Status: **implemented as research services; not all part of the default study pipeline**.

The branch contains capabilities for event studies, factor stability/health, candidate deduplication/screening, HAC/Newey-West/risk-model/attribution style analysis and scenario shocks.

Their existence should not be confused with default orchestration. They should become selectable research stages under explicit profiles or custom plans.

### 3.9 Alphalens compatibility layer

Status: **semantically implemented; should remain a facade**.

The compatibility module exposes Alphalens-like function concepts while delegating to canonical native implementations. This is the correct strategy.

The target should be **behavior/result-semantic compatibility**, not a second independent implementation and not necessarily a byte-for-byte recreation of every Python object and plotting API.

### 3.10 Reporting and visualization

Status: **implemented, but contract boundary needs redesign**.

`FactorStudyReport` is serializable and can carry full prepared arrays and result vectors. That is useful for a first complete pipeline but mixes:

- internal materialized computation artifacts;
- public report schema;
- persistence/wire DTO;
- potentially very large result series.

Those roles should be separated before the report schema becomes difficult to evolve.

### 3.11 Multi-language bindings

Status: **broadly implemented; release parity incomplete**.

The architecture has a central Rust-owned research JSON path and multiple language/platform adapters. This is much better than duplicating research algorithms in each SDK.

Remaining work:

- one versioned request/response/error contract;
- consistent golden-corpus tests;
- typed/ergonomic APIs where language ecosystems need them;
- lower-copy paths for large numerical arrays;
- release jobs green on the same final SHA.

---

## 4. Main execution chains in current code

### 4.1 Current native factor-study happy path

The current standard flow is effectively:

```text
FactorStudyRequest
   |
   v
FactorStudy::new
   |
   +-- validate panel
   +-- prepare_factor_data (eager)
          |
          +-- forward returns
          +-- quantization
   |
   v
FactorStudy::full_report
   |
   +-- factor_weights
   +-- factor_returns
   +-- information_coefficient
   +-- mean_information_coefficient
   +-- mean_return_by_quantile
   +-- factor_alpha_beta
   +-- quantile_turnover
   +-- factor_rank_autocorrelation
   +-- portfolio_performance
   |
   v
FactorStudyReport::build
```

This path is **functionally connected**, but architecture-wise it is hand-orchestrated.

### 4.2 Current ResearchPlan path

`ResearchPlan` models semantic research nodes/dependencies and performs topological validation/compilation. However, it is not yet the sole execution authority for the standard study.

Current conceptual split:

```text
ResearchPlan
   -> semantic DAG / ordering

FactorStudy::full_report
   -> actual stage execution (manual)
```

This creates two sources of workflow truth.

### 4.3 Current incremental path

`IncrementalResearchState` tracks:

- revision;
- panel;
- horizons;
- forward returns;
- maturity state.

Append operations update maturity incrementally. Row replacements correctly fall back to wider/full forward-return recomputation.

The issue is that this is still a **parallel state machine**, not simply another input/revision mode of the same ResearchExecutor.

### 4.4 Current cache path

Current research cache shape is conceptually:

```text
ResearchCacheKey {
    node,
    revision,
    parameter_hash,
}
    -> Vec<f64>
```

This is useful as a prototype, but insufficient for the heterogeneous outputs now produced by the research layer.

### 4.5 Current FFI research path

The common research bridge is effectively:

```text
request JSON string
   -> deserialize versioned request
   -> FactorStudy
   -> full_report
   -> serialize JSON string
```

This gives cross-language semantic SSOT, but pays serialization/copy costs and weakens typed error/result contracts at the lowest common layer.

---

## 5. Architecture and implementation problems

## P0 — must fix before declaring the architecture/release complete

### P0.1 ResearchPlan is not the authoritative execution path

**Problem**

The project has already introduced the right semantic abstraction (`ResearchPlan`), but the standard high-level API still manually sequences the real computation in `FactorStudy::full_report()`.

**Risk**

- a node can exist in the plan but not in the actual execution path;
- dependency changes require edits in multiple places;
- cache/incremental/provenance cannot reliably attach to one execution lifecycle;
- advanced research services will encourage an ever-larger `full_report()` method.

**Target**

Add `ResearchExecutor` and make `ResearchPlan` the semantic source of truth for orchestration.

### P0.2 `FactorStudy::new` performs eager preparation

**Problem**

Construction currently does material work (forward-return preparation/quantization).

**Why unreasonable**

A constructor that is expected to represent a study also becomes an execution boundary. This makes lazy execution, cache reuse, profile selection and incremental invalidation harder.

**Target**

`FactorStudy::new` should validate/normalize request/context only. Execution starts explicitly through the executor.

### P0.3 Canonical algorithm ownership is not fully enforced

**Problem**

The recent SSOT gate has identified ambiguous/repeated ownership around rank/regression, rolling risk names and language facades.

**Target**

Define a canonical owner registry. Facades may delegate, but may not contain independent algorithm bodies. Explicitly separate naming such as:

```text
rolling_sortino_ratio
rolling_max_drawdown
```

from whole-series/aggregate risk metrics.

### P0.4 Current branch is not release-green

At the reviewed head, the project still has real red gates. In particular:

- Format: red;
- core tests: red because of `HT_SINE` throughput gate;
- Visualization integration: red;
- Factor research SSOT: red;
- multilang release: Node and Go release jobs are red.

The core test evidence is especially important: almost the whole core test set is functionally passing, but `HT_SINE` misses its throughput budget. This means **implementation coverage is high, but release acceptance is not complete**.

### P0.5 CI must verify source, not mutate/push source

The temporary PR #29 one-shot repair workflow/script applies fixes, formats source and can push a new commit after verification. This was useful for diagnosing a branch, but should not become permanent project architecture.

**Target CI rule**:

> CI may generate disposable artifacts and reports, but required verification workflows must not rewrite production source and push fixes back to the branch.

Validated fixes should be normal reviewable commits. CI should only verify them.

### P0.6 FFI common layer has high-level application ownership

`ffi-common` now depends on factor research and owns JSON study bridging. That makes the abstraction name misleading and pulls high-level application dependencies through otherwise low-level adapters.

**Target**

Separate:

```text
ffi-common        -> ABI/error/memory/basic transport primitives
api-contract      -> versioned request/response/error research contracts
binding adapter   -> language-specific conversions
```

No research algorithm belongs in any binding.

---

## P1 — structural improvements after baseline is green

### P1.1 Incremental execution is not integrated with the research executor

Incremental state should express **revision + dirty ranges + maturity changes**, not own a parallel workflow. The executor should determine which nodes are invalid and which can be reused.

### P1.2 ResearchCache is too narrowly typed

A `Vec<f64>` materialization cannot represent all research artifacts cleanly. IC tables, grouped results, portfolio traces, regressions, validation splits, reports and provenance all need typed artifacts.

Introduce a typed artifact/materialization model.

### P1.3 Cache identity is incomplete

A safe key should include more than node + revision + parameter hash. It should cover at least:

- input data revision or fingerprint;
- normalized research-policy hash;
- node/algorithm version;
- request/report schema version where relevant;
- horizon/group universe semantics;
- dirty-range compatibility.

### P1.4 Report object doubles as internal artifact and wire format

Split:

```text
ResearchArtifacts            // internal materializations
FactorStudyReportV1          // stable external contract
ReportRenderer / Serializer  // JSON/HTML/etc.
```

Large arrays should be optional/referenced where possible rather than always embedded in every external report.

### P1.5 Research policy is scattered

Policies such as missing-value handling, quantization, grouping, weighting, horizons, annualization, benchmark behavior, cost and capacity should be explicit inputs rather than implicit defaults spread across application methods.

Introduce a normalized `ResearchPolicy` tree.

### P1.6 Validation module has too many responsibilities

Refactor internally into something like:

```text
validation/
  splits.rs
  resampling.rs
  inference.rs
  multiple_testing.rs
```

Public APIs can remain stable through re-exports.

### P1.7 Repeated cross-sectional grouping/traversal patterns

Many analytics operate date-by-date, optionally group-by-group, with similar filtering/ranking/normalization behavior.

Introduce a reusable cross-sectional view/executor that owns deterministic grouping, finite-value masks, ordering and scratch reuse.

### P1.8 Advanced services lack declarative pipeline profiles

Do not put every capability into `full_report()`.

Provide profiles such as:

```text
CoreStudy
FullStudy
Custom(ResearchPlan)
```

with explicit optional stages for multifactor, validation, event, stability, mining, risk and scenario analysis.

### P1.9 Binding parity should be contract-driven

Use one golden request corpus and compare normalized response/error digests across supported bindings. Typed convenience APIs can differ by language, but semantics must not.

---

## P2 — optimization and selective physical decomposition

### P2.1 Do not immediately split the workspace into many crates

The core crate is broad, but a big-bang crate split would create significant churn in public paths, features and bindings.

First enforce logical dependency boundaries inside existing crates. Split physically only when the dependency graph and compile-time/runtime gains are measurable.

A possible later layout is:

```text
finkit-kernel
finkit-runtime
finkit-research-model
finkit-factor-analysis
finkit-api-contract
finkit-visualization
finkit                 // stable facade / re-exports
```

This is an **optional later target**, not the first refactor action.

### P2.2 Add end-to-end performance budgets

Current micro/per-indicator performance gates are useful, but research workloads also need budgets for:

- full FactorStudy;
- cross-sectional IC/quantization;
- multi-horizon forward returns;
- incremental append/update;
- report serialization;
- FFI/binding overhead;
- large-panel memory high-water marks.

### P2.3 Generate capability/version documentation from SSOT

The repository currently distinguishes published distribution versions from source-tree capabilities. That is valid, but easy to misunderstand.

Generate a capability manifest from the same source used by CI, then produce docs tables from it.

---

## 6. Target architecture

The recommended architecture is **logical layering first**:

```text
+------------------------------------------------------------------+
| Layer 8: Language / Platform Bindings                             |
| Python / Node / C / WASM / Go / .NET / Java / Android / ...      |
+-------------------------------+----------------------------------+
                                |
+-------------------------------v----------------------------------+
| Layer 7: Versioned API Contract                                  |
| RequestVn / ResponseVn / ErrorEnvelope / CapabilityManifest      |
+-------------------------------+----------------------------------+
                                |
+-------------------------------v----------------------------------+
| Layer 6: Reporting & Rendering                                   |
| FactorStudyReportV1 / JSON / HTML / chart adapters               |
+-------------------------------+----------------------------------+
                                |
+-------------------------------v----------------------------------+
| Layer 5: Research Orchestration                                  |
| ResearchPlan -> ResearchExecutor -> ArtifactStore -> Trace        |
+-------------------------------+----------------------------------+
                                |
+-------------------------------v----------------------------------+
| Layer 4: Research Services                                       |
| Analysis / Portfolio / Multifactor / Validation / Event / Risk   |
| Stability / Mining / Scenario                                    |
+-------------------------------+----------------------------------+
                                |
+-------------------------------v----------------------------------+
| Layer 3: Research Model & Policy                                 |
| PanelIndex / ResearchFrame / Horizon / Policy / Provenance       |
+-------------------------------+----------------------------------+
                                |
+-------------------------------v----------------------------------+
| Layer 2: Runtime                                                 |
| FormulaEngine / FormulaGraph / ComputePlan / scheduler / cache   |
+-------------------------------+----------------------------------+
                                |
+-------------------------------v----------------------------------+
| Layer 1: Indicators & Features                                   |
| batch / streaming / transforms / rolling operators              |
+-------------------------------+----------------------------------+
                                |
+-------------------------------v----------------------------------+
| Layer 0: Canonical Kernel                                        |
| returns / rank / quantile / regression / statistics / math       |
+------------------------------------------------------------------+
```

### Dependency rules

1. Bindings depend downward only.
2. API contract contains DTO/schema/error definitions, not algorithms.
3. Reporting consumes materialized artifacts; it does not recompute research.
4. Research orchestration owns workflow sequencing.
5. Research services do not know language bindings.
6. Research model owns policy semantics and data identity.
7. Runtime owns generic scheduling/materialization mechanics, not Alphalens semantics.
8. Canonical kernel does not import research, reporting or FFI layers.
9. Facades can delegate to canonical owners; they cannot reimplement them.
10. The stable top-level `finkit` facade can re-export compatibility APIs during migration.

---

## 7. Target unified research main chain

```text
Raw arrays / panel input
        |
        v
ResearchRequestV2
        |
        v
normalize + validate
        |
        v
ResearchContext
  - data identity
  - normalized policies
  - provenance
  - revision / dirty ranges
        |
        v
ResearchPlan
  - semantic DAG
  - requested profile
        |
        v
ResearchExecutor::compile
  - validate dependencies
  - bind canonical services
  - determine cacheability/effects
        |
        v
ResearchExecutor::execute
        |
        +--> core ComputePlan/runtime where reusable
        +--> factor research services
        |
        v
Typed ResearchArtifactStore
        |
        +--> revision-aware materialization cache
        +--> incremental invalidation/reuse
        |
        v
FactorStudyReportV1
        |
        +--> JSON
        +--> HTML
        +--> visualization
        +--> binding-native adapters
```

The critical rule is:

> **Full execution and incremental execution must use the same ResearchPlan + ResearchExecutor.**

There must not be one algorithmic workflow for batch and another for incremental mode.

---

## 8. Key target types

The exact Rust names can change during implementation, but the responsibilities should look like this:

```rust
pub struct ResearchContext {
    pub frame: ResearchFrame,
    pub policy: ResearchPolicy,
    pub revision: DataRevision,
    pub provenance: Provenance,
}

pub struct ResearchPolicy {
    pub missing: MissingPolicy,
    pub quantization: QuantizationPolicy,
    pub grouping: GroupPolicy,
    pub weighting: WeightPolicy,
    pub horizons: HorizonPolicy,
    pub annualization: AnnualizationPolicy,
    pub benchmark: BenchmarkPolicy,
    pub cost: CostPolicy,
}

pub struct ResearchExecutor {
    // semantic registry, runtime adapter, materialization store
}

pub enum ResearchArtifact {
    PreparedFactorData(...),
    FactorWeights(...),
    FactorReturns(...),
    InformationCoefficient(...),
    QuantileReturns(...),
    AlphaBeta(...),
    Turnover(...),
    RankAutocorrelation(...),
    PortfolioPerformance(...),
    Multifactor(...),
    Validation(...),
    EventStudy(...),
    RiskModel(...),
    Scenario(...),
}

pub struct MaterializationKey {
    pub data_revision: DataRevision,
    pub node: ResearchNodeId,
    pub policy_hash: u64,
    pub parameter_hash: u64,
    pub algorithm_version: u32,
    pub schema_version: u32,
}
```

The artifact enum may ultimately be implemented using typed IDs/generics instead of one large enum. The important requirement is **typed heterogeneous materialization**, not the specific syntax.

---

## 9. Refactoring roadmap

## R0 — stabilize the current branch

Goal: create a trustworthy baseline before structural change.

Actions:

1. land already-validated source fixes as normal commits rather than CI-side mutations;
2. remove the temporary self-modifying/pushing one-shot workflow and helper after its diagnostic purpose is complete;
3. restore `cargo fmt --all --check`;
4. converge Research SSOT ownership failures;
5. fix `HT_SINE` so the checked-in source passes the same throughput budget used in CI without lowering the threshold;
6. repair Visualization integration;
7. repair Node and Go release gates;
8. verify the **same final SHA** across core/workspace/Clippy/docs/performance/research/bindings/release checks.

Acceptance:

- all required gates green on one SHA;
- no required CI workflow rewrites/pushes production code;
- no skipped required gate hidden by dependency failure;
- no reduction of parity or performance thresholds.

## R1 — canonical ownership and dependency contracts

Actions:

1. document machine-readable canonical owners for shared algorithms;
2. enforce ownership in CI;
3. rename ambiguous rolling vs aggregate metrics;
4. keep compatibility aliases for the agreed migration window;
5. require facade-only wrappers to be thin delegates;
6. add differential tests where both facade and canonical API exist.

Acceptance:

- one algorithm body per semantic operation;
- wrapper output/error behavior matches canonical owner;
- no new reverse dependency from core/runtime into research/FFI.

## R2 — introduce the authoritative ResearchExecutor

Actions:

1. add `ResearchContext`;
2. add `ResearchExecutor`;
3. bind each `ResearchNodeKind` to one canonical research service;
4. make the executor produce typed artifacts and execution trace;
5. change `FactorStudy::new` to validation/context creation only;
6. implement `FactorStudy::full_report()` as a compatibility facade that executes a standard ResearchPlan;
7. retain ResearchPlan as a semantic DAG rather than converting it into FormulaRequest.

Acceptance:

- no manual stage list remains as business logic in `full_report()`;
- `FactorStudy` and direct ResearchPlan execution produce identical normalized outputs;
- missing/cyclic dependencies fail before stage execution;
- provenance records executed node versions/policies.

## R3 — centralize policy semantics

Actions:

1. create normalized policy objects;
2. move current implicit high-level defaults into named profiles;
3. centralize finite/NaN handling;
4. centralize tie/rank/quantile semantics;
5. centralize date/group ordering guarantees;
6. record all policies in provenance/report.

Acceptance:

- no hidden policy choice is required to reproduce a report;
- deterministic golden inputs yield deterministic results;
- compatibility profiles can intentionally reproduce historical behavior.

## R4 — merge cache and incremental execution into the executor lifecycle

Actions:

1. replace ad-hoc revision invalidation with dirty-range-aware execution metadata;
2. extend cache identity to data/policy/algorithm/schema identity;
3. introduce typed artifact materialization;
4. make append/replace operations update context revision + dirty ranges;
5. executor chooses reuse/recompute per node;
6. keep correctness-first fallback to full recomputation for non-incremental-safe nodes.

Acceptance:

- batch/full result == incremental result within defined tolerance;
- randomized append/replace sequences have property tests;
- stale revisions cannot hit cache;
- non-incremental-safe node reuse is impossible by contract.

## R5 — separate artifacts, report schema and visualization

Actions:

1. introduce internal `ResearchArtifacts`/store;
2. version external report as `FactorStudyReportV1` (or next agreed schema version);
3. distinguish summary metrics from large optional series;
4. add renderer/serializer layer;
5. make visualization consume report/artifact interfaces rather than recomputing analytics;
6. add round-trip/backward schema tests.

Acceptance:

- report schema is explicitly versioned;
- JSON and visualization use the same computed artifacts;
- report generation performs no hidden research recomputation;
- large results can avoid unnecessary wire copies.

## R6 — normalize API/FFI contracts

Actions:

1. keep low-level ABI primitives in `ffi-common`;
2. move high-level research transport/schema into an explicit API-contract/application adapter module or crate;
3. preserve structured error code/category/details across bindings;
4. retain JSON compatibility APIs;
5. add typed/low-copy array paths where meaningful;
6. create a single golden request/response/error corpus;
7. validate response digests across all supported adapters.

Acceptance:

- the same invalid request returns the same semantic error across bindings;
- the same golden study produces the same normalized result digest;
- no binding contains independent factor algorithm logic.

## R7 — modularize research services and introduce profiles

Actions:

1. split validation internals;
2. add reusable cross-sectional view/group index/scratch strategy;
3. define `CoreStudy`, `FullStudy` and custom plan profiles;
4. connect multifactor/validation/event/stability/mining/risk/scenario through explicit optional plan stages;
5. keep expensive/advanced capabilities opt-in unless profile semantics demand them.

Acceptance:

- adding a research node does not require editing a giant `full_report()` switch/sequence;
- advanced services are reachable through the same executor;
- default profile cost is bounded and documented.

## R8 — performance, physical crate split and documentation automation

Actions:

1. add end-to-end research benchmarks and memory budgets;
2. profile allocation/sorting in cross-sectional operations;
3. optimize report serialization and binding copies;
4. generate capability/version docs from SSOT;
5. measure dependency/compile/runtime impact before any physical crate split;
6. split crates only when there is a clear ownership or build/runtime benefit.

Acceptance:

- benchmark budgets enforced in CI;
- no regression hidden by aggregate benchmarks;
- dependency graph follows allowed direction;
- published/source capability differences are machine-verifiable.

---

## 10. Migration strategy

This should **not** be a big-bang rewrite.

Recommended compatibility strategy:

1. retain current top-level public Rust APIs as facades while internals migrate;
2. keep deprecated aliases for one agreed release window;
3. preserve current accepted request schema v1 compatibility while v2 remains canonical;
4. keep current research JSON bridge available while introducing cleaner typed contracts;
5. switch `FactorStudy::full_report()` implementation internally to ResearchExecutor without initially changing its public call shape;
6. make new executor/cache/report APIs additive first;
7. remove legacy paths only after golden compatibility gates prove callers have a migration path.

---

## 11. Required tests and gates after refactor

The following should become hard architecture gates.

### 11.1 Correctness

- TA-Lib parity where applicable;
- factor-analysis golden datasets;
- batch vs incremental equivalence;
- direct canonical API vs compatibility facade equivalence;
- profile vs explicit ResearchPlan equivalence;
- report JSON round-trip and schema compatibility.

### 11.2 Invariants

- no look-ahead in forward returns;
- date-local quantization;
- stable tie/rank behavior;
- group-neutrality invariants;
- weight normalization invariants;
- purge/embargo leakage invariants;
- deterministic output ordering.

### 11.3 Architecture

- canonical owner checker;
- allowed dependency-direction checker;
- no algorithm bodies in bindings;
- no required workflow that commits/pushes source;
- capability-manifest consistency.

### 11.4 Performance

- hot indicator budgets including HT_SINE;
- full study ns/row or rows/s budgets on reference panel sizes;
- peak allocation/memory budget;
- incremental append speedup vs full recompute;
- cache hit speedup;
- serialization overhead;
- FFI/binding overhead.

### 11.5 Release

One final SHA must prove:

```text
format
+ workspace compile
+ clippy
+ core tests
+ docs
+ factor research tests
+ SSOT gates
+ visualization
+ performance
+ Python
+ Node
+ C
+ WASM
+ Go
+ .NET
+ Java/Android/iOS (as supported by release policy)
+ final release gate
```

No release decision should be based on green jobs from different SHAs.

---

## 12. Function completion matrix

| Capability | Implementation | Standard-chain integration | Refactor priority |
|---|---|---|---|
| Technical indicators | High | High | P0 performance/SSOT cleanup |
| FormulaEngine/FormulaGraph | High | High | Maintain |
| ComputePlan/runtime | High | High in core | Reuse below research executor |
| Streaming/stateful | High | High in core | Maintain/benchmark |
| Factor panel model | Implemented | High | P1 policy/context normalization |
| Forward returns | Implemented | High | P1 incremental unification |
| Quantization | Implemented | High | P1 policy normalization |
| Weights/returns | Implemented | High | P1 cross-section reuse |
| IC/mean IC | Implemented | High | P1 cross-section reuse |
| Quantile returns | Implemented | High | P1 cross-section reuse |
| Alpha/beta | Implemented | High | SSOT regression ownership |
| Turnover/autocorrelation | Implemented | High | P1 executor integration |
| Portfolio performance | Implemented | High | P1 policy/profile integration |
| Multifactor | Implemented | Optional/standalone | P1 profile integration |
| Purged/CPCV/walk-forward | Implemented | Optional/standalone | P1 module/profile integration |
| Bootstrap/permutation/multiple testing | Implemented | Optional/standalone | P1 module/profile integration |
| Event study | Implemented | Optional/standalone | P1 profile integration |
| Stability/health | Implemented | Optional/standalone | P1 profile integration |
| Mining/dedup/screening | Implemented | Optional/standalone | P1 profile integration |
| Risk model/attribution | Implemented | Optional/standalone | P1 profile integration |
| Scenario analysis | Implemented | Optional/standalone | P1 profile integration |
| ResearchPlan | Implemented semantic DAG | **Not authoritative executor** | **P0** |
| Incremental research | Implemented partial path | **Parallel path** | **P1** |
| Research cache | Prototype/limited | **Not universal** | **P1** |
| FactorStudyReport | Implemented | High | P1 schema/artifact split |
| Visualization | Implemented | Partial | P0 current gate, P1 artifact contract |
| FFI JSON research API | Mostly implemented | Mostly connected | P0/P1 contract cleanup |
| Typed binding parity | Partial | Partial | P1 |
| Release readiness | **Not complete** | N/A | **P0** |

---

## 13. What should not be refactored away

Several current design decisions are good and should be preserved:

1. **Reuse-first canonical kernels.** Research must reuse core algorithms rather than fork them.
2. **One-way research dependency.** `factor-analysis -> finkit` is the right direction.
3. **ResearchPlan is semantic, ComputePlan is generic runtime.** Reuse runtime concepts without forcing research into Formula syntax.
4. **Versioned request schemas.** Current v2 with v1 compatibility is the right migration pattern.
5. **Central Rust semantics across bindings.** Keep business logic out of Node/Go/Python/etc. wrappers.
6. **Explicit release/performance gates.** Fix implementation rather than lowering thresholds.
7. **Research/trading boundary.** Keep live execution and broker/data-provider coupling out of the core library.

---

## 14. Architecture decisions requiring owner confirmation

The code review exposes five decisions that affect the exact refactor shape. This plan uses conservative defaults so documentation can be reviewed now, but implementation beyond R0/R1 should follow the confirmed decisions.

### Decision A — public API compatibility window

**Question:** Must existing public Rust APIs and all language-binding entry points remain source/behavior compatible for at least one release?

Recommended default: **Yes.** Use deprecated aliases/facades for at least one release instead of hard breaks.

### Decision B — product boundary

**Question:** Should Finkit remain a calculation/research infrastructure library and continue excluding market-data collection, broker integration and live order execution?

Recommended default: **Yes.** Keep these outside the repository or in downstream applications.

### Decision C — Alphalens compatibility goal

**Question:** Is the desired goal semantic/result compatibility with Alphalens rather than a complete clone of every Python object, plotting function and historical API quirk?

Recommended default: **Semantic/result compatibility.** Add explicit compatibility tests and avoid duplicating an entire Python-specific architecture.

### Decision D — crate decomposition

**Question:** Should the refactor enforce logical module/dependency boundaries first and only physically split `core` after measurement?

Recommended default: **Yes.** Avoid a high-risk big-bang workspace split.

### Decision E — report/FFI evolution

**Question:** May the next report contract become summary-first with optional large arrays/artifact references, while the current JSON shape remains available through a legacy compatibility adapter for one release?

Recommended default: **Yes.** This provides a path to lower serialization/memory cost without abruptly breaking consumers.

---

## 15. Recommended execution order

Do **not** start with a large crate move. Execute in this order:

```text
R0  Current-head green baseline
 -> R1 Canonical ownership
 -> R2 ResearchExecutor / single execution path
 -> R3 Policy normalization
 -> R4 Incremental + typed cache/materialization
 -> R5 Artifact/report/visualization contract
 -> R6 API/FFI contract
 -> R7 Research profiles/module cohesion
 -> R8 Performance + selective physical split + generated docs
```

The most important architectural milestone is the end of **R4**. At that point, batch, incremental, cache, provenance and high-level reporting are all driven by one execution chain. That eliminates the current largest class of future orphan logic and semantic drift.

---

## 16. Definition of done

The refactor is complete only when all of the following are true:

- every semantic algorithm has one canonical implementation owner;
- all public compatibility APIs are delegates, not copied algorithms;
- `ResearchPlan` is the semantic source of truth and `ResearchExecutor` is the execution source of truth;
- `FactorStudy::full_report()` is a facade/profile, not a manual orchestration engine;
- batch and incremental modes execute through the same plan/executor;
- cache materialization is typed and revision/policy/version safe;
- report and visualization consume the same computed artifacts;
- all language bindings share versioned request/response/error semantics;
- advanced research services are reachable through explicit plans/profiles without being forced into the default path;
- no circular/reverse architectural dependency exists;
- no required CI workflow modifies and pushes production source;
- same-SHA correctness, SSOT, performance, visualization, binding and release gates are all green;
- documentation/capability/version state is mechanically checked.

Until these conditions are met, the correct project status is:

> **Feature-rich and substantially implemented, but still converging from multiple capable subsystems into one production-grade execution architecture.**
