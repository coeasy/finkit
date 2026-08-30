# Finkit Streaming — Optimization Plan

> Priority-ordered roadmap. See `STREAMING_VS_TALIB_EFFICIENCY.md` for comparison data.

## Phase 1: Easy O(P) → O(1) — HIGH PRIORITY

| # | Indicator | File | Current | Fix | Risk | Status |
|---|-----------|------|---------|-----|------|--------|
| 1.1 | CFO | `momentum/cfo.rs` | O(P) inline linear regression | Running sum_y/sum_xy (like TSF) | Low | ✅ Done |
| 1.2 | EfficiencyRatio | `overlap/efficiency_ratio.rs` | O(P) abs-diff loop | Ring buffer + running sum | Low | ✅ Done |
| 1.3 | Inertia | `trend/inertia.rs` | O(P) inline linear regression | Running sums (like TSF) | Low | ✅ Done |
| 1.4 | SqueezeMomentum | `pattern/squeeze_momentum.rs` | O(P) inline linear regression | Running sums (like TSF) | Low | ✅ Done |
| 1.5 | VwapBands | `overlap/vwap_bands.rs` | `.iter().sum()` per bar | Welford online (like var.rs) | Low | ✅ Done |
| 1.6 | WMA | `overlap/wma.rs` | ~~O(P)~~ O(1) | Running weighted_sum | Low | ✅ Done |

## Phase 2: Medium Difficulty — MEDIUM PRIORITY

| # | Indicator | File | Current | Fix | Risk | Status |
|---|-----------|------|---------|-----|------|--------|
| 2.1 | Max/Min | `math/math_operators.rs` | O(P) linear scan | Monotonic deque | Low | ✅ Done |
| 2.2 | ALMA | `overlap/alma.rs` | O(P) weighted dot product | Weights already precomputed; O(P) dot product is inherent | — | Skip |
| 2.3 | UlcerIndex | `volatility/ulcer_index.rs` | O(P) dd_sum recompute | Track prev_max, incremental when stable | Medium | ✅ Done |

## Phase 3: Hard O(P) — Accept for P ≤ 200

| # | Indicator | File | Why O(P) | Notes |
|---|-----------|------|----------|-------|
| 3.1 | CCI | `momentum/cci.rs` | Mean absolute deviation | Inherently O(P) |
| 3.2 | AvgDev | `statistics/avgdev.rs` | Mean absolute deviation | Inherently O(P) |
| 3.3 | PercentRank | `statistics/percent_rank.rs` | Count below threshold | Inherently O(P) |

## Phase 4: Macro Migration — MEDIUM PRIORITY

### 4.1 Non-Trait Indicators (add StreamingIndicator impl)
- `ichimoku.rs`, `supertrend.rs`, `donchian.rs`, `keltner.rs`, `mfi.rs`
- `beta.rs`, `correl.rs`, `sar.rs`, `patterns.rs` (19 CDL_*)

### 4.2 Extend impl_indicator_meta! Macro
- Add warm_up expression variant: `impl_indicator_meta!(Type, "NAME", "cat", "desc", warm_up_expr)`

## Phase 5: Missing TA-Lib Candlestick Patterns — LOW PRIORITY

TA-Lib has ~61 patterns; Finkit has 19. Missing ~42. Implement as needed per user demand.

## Execution Order

1. ✅ WMA O(1) — Done
2. ✅ CFO, EfficiencyRatio, Inertia, SqueezeMomentum — Done (same linear regression pattern)
3. ✅ VwapBands — Done (Welford's algorithm)
4. ✅ Max/Min — Done (monotonic deque)
5. ✅ ALMA — Skipped (weights already precomputed; O(P) dot product is inherent for non-uniform weights)
6. ✅ UlcerIndex — Done (incremental drawdown when max is stable)
7. Macro migration for non-trait indicators
8. Document remaining O(P) as acceptable
