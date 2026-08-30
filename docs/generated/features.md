# Feature Engineering Modules

> **SSOT** — auto-generated from `core/src/features/mod.rs`.
> Do not edit manually. Regenerate: `python scripts/gen_ssot_docs.py --generate`

Feature engineering submodules: **3**

Feature engineering transforms raw OHLCV data into ML-ready feature matrices.

## Module Reference

| Module |
|--------|
| `complexity` |
| `fourier` |
| `wavelet` |

## Usage Example

```rust
use finkit::features::{FeatureMatrix, FeatureSet};

let mut features = FeatureSet::new();
features.add_indicator("sma", &[5, 10, 20]);
features.add_indicator("rsi", &[14]);
let matrix = features.generate(&ohlcv)?;
```

## Regenerate

```bash
python scripts/gen_ssot_docs.py --generate
python scripts/gen_ssot_docs.py --check   # CI gate
```
