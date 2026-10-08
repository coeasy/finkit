# Streaming Indicators Catalog

> **SSOT** — auto-generated from `core/src/streaming/mod.rs` and submodule `pub struct` exports.
> Do not edit manually. Regenerate: `python scripts/gen_ssot_docs.py --generate`

Streaming source modules: **17** | Direct public structs: **219**
Registered indicator entries marked streaming in `docs/indicator_registry.json`: **144**

Streaming indicators provide O(1) per-bar updates via the `StreamingIndicator` trait.
The source scan lists directly detected public structs; the registered count is the user-facing indicator count.

## breadth

| Struct |
|--------|
| `StreamingAdvanceDeclineLine` |
| `StreamingAr` |
| `StreamingBr` |
| `StreamingCr` |
| `StreamingFearGreedIndex` |
| `StreamingPutCallRatio` |
| `StreamingTrin` |

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

## cycle

| Struct |
|--------|
| `HtSineOutput` |
| `StreamingBandpass` |
| `StreamingDecycler` |
| `StreamingHtDcPeriod` |
| `StreamingHtDcPhase` |
| `StreamingHtPhasor` |
| `StreamingHtSine` |
| `StreamingInstantaneousTrendline` |
| `StreamingMassIndex` |
| `StreamingMcClellanOscillator` |
| `StreamingRoofingFilter` |
| `StreamingSuperSmoother` |
| `StreamingSuperSmoother3Pole` |

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

## forming_bar

| Struct |
|--------|
| `FormingBar` |

## math

| Struct |
|--------|
| `StreamingAcos` |
| `StreamingAdd` |
| `StreamingAsin` |
| `StreamingAtan` |
| `StreamingCeil` |
| `StreamingCos` |
| `StreamingCosh` |
| `StreamingDiv` |
| `StreamingExp` |
| `StreamingFloor` |
| `StreamingLn` |
| `StreamingLog10` |
| `StreamingMax` |
| `StreamingMin` |
| `StreamingMinus` |
| `StreamingMult` |
| `StreamingSin` |
| `StreamingSinh` |
| `StreamingSqrt` |
| `StreamingSub` |
| `StreamingSum` |
| `StreamingTan` |
| `StreamingTanh` |

## momentum

| Struct |
|--------|
| `AroonOutput` |
| `ElderRayOutput` |
| `FisherOutput` |
| `KdjOutput` |
| `KstOutput` |
| `MacdOutput` |
| `RviOutput` |
| `StochOutput` |
| `StreamingAo` |
| `StreamingApo` |
| `StreamingAroon` |
| `StreamingAroonOsc` |
| `StreamingBias` |
| `StreamingCci` |
| `StreamingCfo` |
| `StreamingCmo` |
| `StreamingCoppock` |
| `StreamingDpo` |
| `StreamingElderRay` |
| `StreamingFisher` |
| `StreamingKdj` |
| `StreamingKst` |
| `StreamingMacd` |
| `StreamingMacdExt` |
| `StreamingMacdFix` |
| `StreamingMom` |
| `StreamingPpo` |
| `StreamingPsy` |
| `StreamingRoc` |
| `StreamingRocp` |
| `StreamingRocr` |
| `StreamingRocr100` |
| `StreamingRsi` |
| `StreamingRvi` |
| `StreamingStc` |
| `StreamingStoch` |
| `StreamingStochF` |
| `StreamingStochRsi` |
| `StreamingTrix` |
| `StreamingTsf` |
| `StreamingTsi` |
| `StreamingUltOsc` |
| `StreamingWillR` |
| `UnsupportedMaType` |

## overlap

| Struct |
|--------|
| `DmaOutput` |
| `ExpmaOutput` |
| `IchimokuOutput` |
| `SmaSnapshot` |
| `StreamingAlma` |
| `StreamingAnchoredVwap` |
| `StreamingDema` |
| `StreamingDma` |
| `StreamingEfficiencyRatio` |
| `StreamingEma` |
| `StreamingExpma` |
| `StreamingHma` |
| `StreamingIchimoku` |
| `StreamingJma` |
| `StreamingKama` |
| `StreamingMama` |
| `StreamingMcGinley` |
| `StreamingMidpoint` |
| `StreamingSma` |
| `StreamingT3` |
| `StreamingTema` |
| `StreamingTrima` |
| `StreamingVidya` |
| `StreamingVwap` |
| `StreamingVwapBands` |
| `StreamingVwapMtf` |
| `StreamingVwma` |
| `StreamingWma` |
| `StreamingZlema` |
| `VwapBandsOutput` |
| `VwapMtfInput` |

## pattern

| Struct |
|--------|
| `SqueezeMomentumOutput` |
| `StreamingFairValueGap` |
| `StreamingOrderBlock` |
| `StreamingSqueezeMomentum` |

## price_transform

| Struct |
|--------|
| `StreamingAvgPrice` |
| `StreamingBop` |
| `StreamingMedPrice` |
| `StreamingMidprice` |
| `StreamingQStick` |
| `StreamingTypPrice` |
| `StreamingWclPrice` |

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

## statistics

| Struct |
|--------|
| `StreamingAvgdev` |
| `StreamingBeta` |
| `StreamingCorrel` |
| `StreamingLinReg` |
| `StreamingLinRegAngle` |
| `StreamingLinRegIntercept` |
| `StreamingLinRegSlope` |
| `StreamingMaxIndex` |
| `StreamingMinIndex` |
| `StreamingPercentRank` |
| `StreamingZscore` |

## trend

| Struct |
|--------|
| `SarOutput` |
| `StreamingAdx` |
| `StreamingAdxr` |
| `StreamingDx` |
| `StreamingHtMeasurement` |
| `StreamingHtTrendMode` |
| `StreamingHtTrendline` |
| `StreamingInertia` |
| `StreamingMinusDi` |
| `StreamingMinusDm` |
| `StreamingPlusDi` |
| `StreamingPlusDm` |
| `StreamingSar` |
| `StreamingSuperTrend` |
| `StreamingVortex` |
| `SuperTrendOutput` |
| `VortexOutput` |

## volatility

| Struct |
|--------|
| `BollOutput` |
| `DonchianOutput` |
| `EneOutput` |
| `KeltnerOutput` |
| `StreamingAdr` |
| `StreamingAtr` |
| `StreamingBoll` |
| `StreamingChaikinVol` |
| `StreamingChop` |
| `StreamingDonchian` |
| `StreamingEne` |
| `StreamingHv` |
| `StreamingKeltner` |
| `StreamingNatr` |
| `StreamingStdDev` |
| `StreamingTrange` |
| `StreamingUlcerIndex` |
| `StreamingVar` |

## volume

| Struct |
|--------|
| `KvoOutput` |
| `StreamingAd` |
| `StreamingAdosc` |
| `StreamingCmf` |
| `StreamingEom` |
| `StreamingForceIndex` |
| `StreamingKvo` |
| `StreamingMfi` |
| `StreamingMoneyFlow` |
| `StreamingNvi` |
| `StreamingObv` |
| `StreamingPvi` |
| `StreamingPvt` |
| `StreamingTwiggsMf` |
| `StreamingVolumeMomentum` |
| `StreamingVolumeOscillator` |
| `StreamingVolumeRoc` |
| `StreamingVr` |
| `StreamingVzo` |

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
