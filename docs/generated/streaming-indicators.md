# Streaming Indicators Catalog

> **SSOT** — auto-generated from `core/src/streaming/mod.rs` and submodule `pub struct` exports.
> Do not edit manually. Regenerate: `python scripts/gen_ssot_docs.py --generate`

Streaming source modules: **5** | Direct public structs: **24**
Registered indicator entries marked streaming in `docs/indicator_registry.json`: **145**

Streaming indicators provide O(1) per-bar updates via the `StreamingIndicator` trait.
The source scan lists directly detected public structs; the registered count is the user-facing indicator count.

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
use finkit::streaming::{StreamingIndicator, OhlcvBar};
use finkit::streaming::indicators::StreamingSma;

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
