# Indicator Catalog

> **SSOT** — auto-generated from `core/src/indicators/mod.rs` and submodule `pub fn` exports.
> Do not edit manually. Regenerate: `python scripts/gen_ssot_docs.py --generate`

Modules exported from `indicators/mod.rs`: **31** | Public indicator functions: **280**

## astock

| Function |
|----------|
| `committee_ratio` |
| `consecutive_limit` |
| `consecutive_limit_days` |
| `cost` |
| `dragon_tiger_net_buy` |
| `limit_down` |
| `limit_up` |
| `limit_up_strength` |
| `main_net_inflow` |
| `margin_balance` |
| `margin_buy_amount` |
| `money_flow` |
| `north_bound_flow` |
| `rs_ratio` |
| `seal_amount` |
| `sector_strength` |
| `short_balance` |
| `turnover` |
| `turnover_percentile` |
| `volume_ratio` |
| `winner` |

## breadth

| Function |
|----------|
| `advance_decline_line` |
| `advance_decline_ratio` |
| `mcclellan_oscillator` |
| `mcclellan_summation` |
| `new_highs_lows` |
| `trin` |

## chart

| Function |
|----------|
| `heikin_ashi` |
| `zigzag` |

## china

| Function |
|----------|
| `ar` |
| `bias` |
| `br` |
| `cr` |
| `dma` |
| `dpo` |
| `ene` |
| `expma` |
| `kdj` |
| `psy` |
| `vr` |

## classic_patterns

| Function |
|----------|
| `darvas_box` |
| `kagi` |
| `point_and_figure` |
| `renko` |
| `three_line_break` |
| `williams_alligator` |

## classic_tools

| Function |
|----------|
| `andrews_pitchfork` |
| `gann_angles` |
| `median_price` |
| `speed_resistance_lines` |
| `weighted_close` |

## consolidation

| Function |
|----------|
| `bottom_breakout` |
| `bottom_breakout_score` |
| `consolidation_score` |
| `is_sideways` |
| `sideways_duration` |
| `sideways_quality` |
| `sideways_tilt` |
| `top_breakdown` |
| `top_breakdown_score` |

## cycle

| Function |
|----------|
| `bandpass` |
| `decycler` |
| `ht_dcperiod` |
| `ht_dcphase` |
| `ht_measurement` |
| `ht_phasor` |
| `ht_sine` |
| `ht_trendline` |
| `ht_trendmode` |
| `instantaneous_trendline` |
| `roofing_filter` |
| `super_smoother` |
| `super_smoother_3pole` |

## donchian

| Function |
|----------|
| `donchian` |

## fibonacci

| Function |
|----------|
| `fibonacci_retracement` |

## ichimoku

| Function |
|----------|
| `ichimoku` |
| `ichimoku_default` |

## math_operators

| Function |
|----------|
| `add` |
| `div` |
| `max` |
| `maxindex` |
| `min` |
| `minindex` |
| `minmax` |
| `minmaxindex` |
| `minus` |
| `mult` |
| `sub` |
| `sum` |

## math_transform

| Function |
|----------|
| `acos` |
| `asin` |
| `atan` |
| `ceil` |
| `cos` |
| `cosh` |
| `exp` |
| `floor` |
| `ln` |
| `log10` |
| `sin` |
| `sinh` |
| `sqrt` |
| `tan` |
| `tanh` |

## momentum

| Function |
|----------|
| `adx` |
| `adx_into` |
| `adxr` |
| `apo` |
| `aroon` |
| `aroonosc` |
| `bop` |
| `cci` |
| `cci_into` |
| `cmo` |
| `dx` |
| `elder_ray` |
| `macd` |
| `macd_into` |
| `macdext` |
| `macdfix` |
| `mfi` |
| `minus_di` |
| `minus_dm` |
| `mom` |
| `mom_into` |
| `plus_di` |
| `plus_dm` |
| `ppo` |
| `roc` |
| `roc_into` |
| `rocp` |
| `rocr` |
| `rocr100` |
| `rsi` |
| `rsi_into` |
| `stoch` |
| `stoch_into` |
| `stochf` |
| `stochrsi` |
| `trix` |
| `ultosc` |
| `willr` |
| `willr_into` |

## momentum_ext

| Function |
|----------|
| `ao` |
| `chande_forecast_oscillator` |
| `chande_kroll_stop` |
| `chop` |
| `connors_rsi` |
| `coppock` |
| `fisher` |
| `inertia` |
| `kst` |
| `qstick` |
| `rvi` |
| `squeeze_momentum` |
| `stc` |
| `stoch_rsi` |
| `tsi` |
| `ttm_squeeze` |
| `vortex` |
| `williams_fractal` |

## overlap

| Function |
|----------|
| `alma` |
| `bbands` |
| `bbands_into` |
| `dema_into` |
| `efficiency_ratio` |
| `frama` |
| `hma` |
| `jma` |
| `ma` |
| `mama` |
| `midpoint` |
| `midprice` |
| `sar` |
| `sarext` |
| `t3` |
| `tema_into` |
| `vidya` |

## pivot

| Function |
|----------|
| `pivot_points` |

## price_transform

| Function |
|----------|
| `avgprice` |
| `medprice` |
| `typprice` |
| `wclprice` |

## relative_strength

| Function |
|----------|
| `is_strong` |
| `is_weak` |
| `relative_strength_rank` |
| `rs_momentum` |
| `rs_rating` |
| `rs_slope` |

## sentiment

| Function |
|----------|
| `fear_greed_index` |
| `put_call_ratio` |
| `vix_like_volatility` |
| `volatility_index` |

## short_term

| Function |
|----------|
| `big_yang_count` |
| `big_yang_score` |
| `big_yin_count` |
| `big_yin_score` |
| `decline_momentum` |
| `inverted_v_reversal` |
| `inverted_v_reversal_score` |
| `limit_up_streak` |
| `rebound_momentum` |
| `strong_decline` |
| `strong_decline_score` |
| `strong_rebound` |
| `strong_rebound_score` |
| `v_shape_reversal` |
| `v_shape_reversal_score` |

## statistics

| Function |
|----------|
| `avgdev` |
| `beta` |
| `correlation` |
| `linear_reg` |
| `linearreg` |
| `linearreg_angle` |
| `linearreg_intercept` |
| `linearreg_slope` |
| `percent_rank` |
| `pr` |
| `std_dev` |
| `tsf` |
| `var` |
| `zscore` |

## supertrend

| Function |
|----------|
| `supertrend` |
| `supertrend_default` |

## sweep

| Function |
|----------|
| `ema_sweep` |
| `rsi_sweep` |
| `sma_sweep` |

## sweep_engine

| Function |
|----------|
| `new` |
| `run` |
| `sequential` |
| `values` |

## sweepable

_No `pub fn` exports in this module file._

## top_bottom

| Function |
|----------|
| `local_extremum` |
| `swing_high_low` |
| `trend_reversal_confirm` |

## volatility

| Function |
|----------|
| `atr` |
| `atr_into` |
| `natr` |
| `natr_into` |
| `trange` |

## volatility_ext

| Function |
|----------|
| `adr` |
| `calmar_ratio` |
| `chaikin_volatility` |
| `choppiness_index` |
| `garman_klass_volatility` |
| `historical_volatility` |
| `information_ratio` |
| `keltner` |
| `keltner_channel` |
| `mass_index` |
| `max_drawdown` |
| `parkinson_volatility` |
| `realized_volatility` |
| `rogers_satchell_volatility` |
| `semivariance` |
| `sortino_ratio` |
| `ulcer_index` |
| `yang_zhang_volatility` |

## volume

| Function |
|----------|
| `ad` |
| `adosc` |
| `anchored_vwap` |
| `obv` |
| `obv_into` |
| `volume_profile` |
| `vwap` |
| `vwap_bands` |
| `vwap_mtf` |

## volume_ext

| Function |
|----------|
| `cmf` |
| `eom` |
| `force_index` |
| `kvo` |
| `mfi_ext` |
| `nvi` |
| `pvi` |
| `pvt` |
| `twiggs_money_flow` |
| `volume_momentum` |
| `volume_oscillator` |
| `volume_roc` |
| `vwmacd` |
| `vzo` |

## Regenerate

```bash
python scripts/gen_ssot_docs.py --generate
python scripts/gen_ssot_docs.py --check   # CI gate
```
