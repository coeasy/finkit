# Streaming Indicators Catalog

> **SSOT** — auto-generated from `core/src/streaming/mod.rs` and submodule `pub struct` exports.
> Do not edit manually. Regenerate: `python scripts/gen_ssot_docs.py --generate`

Streaming indicator modules: **5** | Public indicator structs: **24**

Streaming indicators provide O(1) per-bar updates via the `StreamingIndicator` trait.

## builder

| Struct |
|--------|
| `AlmaBuilder` |
| `BollBuilder` |
| `EneBuilder` |
| `KeltnerBuilder` |
| `KstBuilder` |
| `SarBuilder` |
| `SmaBuilder` |
| `StochRsiBuilder` |
| `SuperTrendBuilder` |
| `VwapBandsBuilder` |

## float_trait

| Struct |
|--------|
| `GenericAtr` |
| `GenericBoll` |
| `GenericBollOutput` |
| `GenericEma` |
| `GenericMacd` |
| `GenericMacdOutput` |
| `GenericRsi` |
| `GenericSma` |

## registry

| Struct |
|--------|
| `IndicatorInfo` |
| `ParamInfo` |
| `RegistryDocument` |

## ring_buffer

| Struct |
|--------|
| `RingBuffer` |

## rolling_minmax

| Struct |
|--------|
| `RollingMax` |
| `RollingMin` |

## Usage Example

```rust
use AlphaTA_core::streaming::{StreamingIndicator, OhlcvBar};
use AlphaTA_core::streaming::indicators::StreamingSma;

let mut sma = StreamingSma::new(20);
let bar = OhlcvBar::new(open, high, low, close, volume);
if let Some(value) = sma.next(&bar) {
    println!("SMA: {}", value);
}
```

## Regenerate

```bash
python scripts/gen_ssot_docs.py --generate
python scripts/gen_ssot_docs.py --check   # CI gate
```
