# Finkit vs TA-Lib C Benchmark Report

> Auto-generated from Criterion JSON by scripts/bench_report.py.
> Results are valid only for the recorded commit, CPU, compiler, build flags, dataset, and TA-Lib version.

| Indicator | Category | Finkit (µs) | TA-Lib C (µs) | Speedup | Status |
|---|---|---:|---:|---:|:---:|
| acos | math_transform | 143.52 | 137.41 | 0.96x | ⚠️ |
| ad | volume | 12.32 | 12.43 | 1.01x | ✅ |
| add | math_operators | 3.07 | 6.04 | 1.97x | ✅ |
| adosc_3_10 | volume | 94.84 | 99.00 | 1.04x | ✅ |
| adx_14 | directional | 83.86 | 74.24 | 0.89x | ⚠️ |
| adxr_14 | directional | 86.00 | 77.14 | 0.90x | ⚠️ |
| apo_12_26 | momentum_extra | 17.19 | 42.17 | 2.45x | ✅ |
| aroon_14 | directional | 57.54 | 50.22 | 0.87x | ⚠️ |
| aroonosc_14 | momentum | 59.14 | 44.84 | 0.76x | ❌ |
| atr_14 | volatility | 28.55 | 48.02 | 1.68x | ✅ |
| avgdev_14 | statistics_extra | 104.64 | 156.39 | 1.49x | ✅ |
| avgprice | price_transform_full | 6.86 | 8.33 | 1.21x | ✅ |
| bbands_20 | overlap | 49.64 | 72.18 | 1.45x | ✅ |
| bbands_20@10000 | scaled_10k | 33.47 | 47.73 | 1.43x | ✅ |
| bbands_20@100000 | scaled_100k | 837.26 | 978.72 | 1.17x | ✅ |
| bbands_20@1000000 | scaled_1m | 12160.17 | 11240.69 | 0.92x | ⚠️ |
| bop | momentum_extra | 6.79 | 10.58 | 1.56x | ✅ |
| cci_14 | momentum | 186.78 | 220.91 | 1.18x | ✅ |
| ceil | math_transform | 24.16 | 24.95 | 1.03x | ✅ |
| cmo_14 | momentum | 37.21 | 51.54 | 1.39x | ✅ |
| correl_30 | statistics_extra | 61.58 | 75.04 | 1.22x | ✅ |
| cos | math_transform | 23.06 | 25.91 | 1.12x | ✅ |
| dema_20 | overlap | 31.26 | 91.19 | 2.92x | ✅ |
| ema_12 | overlap | 18.48 | 49.79 | 2.69x | ✅ |
| ema_12@10000 | scaled_10k | 18.99 | 50.08 | 2.64x | ✅ |
| ema_12@100000 | scaled_100k | 183.39 | 470.85 | 2.57x | ✅ |
| ema_12@1000000 | scaled_1m | 3391.25 | 6051.15 | 1.78x | ✅ |
| exp | math_transform | 43.93 | 45.61 | 1.04x | ✅ |
| floor | math_transform | 24.76 | 26.47 | 1.07x | ✅ |
| ht_dcperiod | cycle_extra | 463.30 | 600.82 | 1.30x | ✅ |
| ht_dcphase | cycle_extra | 1152.41 | 3226.95 | 2.80x | ✅ |
| ht_phasor | cycle | 475.77 | 558.92 | 1.17x | ✅ |
| ht_sine | cycle | 1355.73 | 3415.75 | 2.52x | ✅ |
| ht_trendline | cycle_extra | 479.11 | 901.31 | 1.88x | ✅ |
| kama_30 | overlap | 27.45 | 102.20 | 3.72x | ✅ |
| linearreg_14 | statistics | 36.27 | 65.87 | 1.82x | ✅ |
| linreg_angle_14 | statistics_extra | 95.69 | 89.41 | 0.93x | ⚠️ |
| linreg_intercept_14 | statistics_extra | 34.77 | 25.88 | 0.74x | ❌ |
| linreg_slope_14 | statistics | 34.51 | 23.94 | 0.69x | ❌ |
| ln | math_transform | 64.40 | 48.31 | 0.75x | ❌ |
| ma_20 | overlap_extra | 13.86 | 19.44 | 1.40x | ✅ |
| macd@10000 | scaled_10k | 143.33 | 148.55 | 1.04x | ✅ |
| macd@100000 | scaled_100k | 1770.38 | 1897.88 | 1.07x | ✅ |
| macd@1000000 | scaled_1m | 18425.85 | 17714.74 | 0.96x | ⚠️ |
| macd_12_26_9 | momentum | 144.30 | 151.64 | 1.05x | ✅ |
| mama | momentum_extra | 524.78 | 890.76 | 1.70x | ✅ |
| max_30 | math_operators | 19.11 | 15.30 | 0.80x | ⚠️ |
| medprice | price_transform_full | 4.00 | 6.18 | 1.55x | ✅ |
| mfi_14 | momentum | 33.78 | 62.41 | 1.85x | ✅ |
| min_30 | math_operators | 20.50 | 15.74 | 0.77x | ❌ |
| minus_di_14 | directional | 57.34 | 58.15 | 1.01x | ✅ |
| minus_dm_14 | momentum | 9.80 | 46.23 | 4.72x | ✅ |
| mom_10 | momentum | 3.33 | 5.56 | 1.67x | ✅ |
| mult | math_operators | 3.20 | 6.24 | 1.95x | ✅ |
| natr_14 | volatility | 31.78 | 52.47 | 1.65x | ✅ |
| obv | volume | 9.84 | 11.31 | 1.15x | ✅ |
| percentrank_30 | statistics_extra | 498.82 | 371.48 | 0.74x | ❌ |
| plus_di_14 | directional | 57.16 | 58.37 | 1.02x | ✅ |
| plus_dm_14 | momentum | 8.53 | 46.46 | 5.44x | ✅ |
| ppo_12_26 | momentum_extra | 54.11 | 46.69 | 0.86x | ⚠️ |
| roc_10 | momentum | 6.28 | 11.08 | 1.76x | ✅ |
| rsi_14 | momentum | 33.20 | 32.92 | 0.99x | ⚠️ |
| rsi_14@10000 | scaled_10k | 23.98 | 31.67 | 1.32x | ✅ |
| rsi_14@100000 | scaled_100k | 226.13 | 307.98 | 1.36x | ✅ |
| rsi_14@1000000 | scaled_1m | 4190.31 | 4265.69 | 1.02x | ✅ |
| sar | overlap_extra | 69.95 | 61.35 | 0.88x | ⚠️ |
| sin | math_transform | 22.31 | 25.07 | 1.12x | ✅ |
| sma_20 | overlap | 14.46 | 20.05 | 1.39x | ✅ |
| sma_20@10000 | scaled_10k | 15.34 | 22.35 | 1.46x | ✅ |
| sma_20@100000 | scaled_100k | 135.37 | 198.07 | 1.46x | ✅ |
| sma_20@1000000 | scaled_1m | 3372.10 | 3277.28 | 0.97x | ⚠️ |
| sqrt | math_transform | 8.64 | 15.37 | 1.78x | ✅ |
| stddev_20 | volatility | 29.35 | 39.91 | 1.36x | ✅ |
| stoch_14_3_3 | momentum | 84.57 | 97.21 | 1.15x | ✅ |
| stochf_14_3 | momentum | 134.63 | 79.00 | 0.59x | ❌ |
| stochrsi_14_14_3_3 | momentum | 237.07 | 106.80 | 0.45x | ❌ |
| sub | math_operators | 3.06 | 6.20 | 2.03x | ✅ |
| sum_30 | math_operators | 12.85 | 19.50 | 1.52x | ✅ |
| t3_5 | overlap_extra | 38.20 | 350.93 | 9.19x | ✅ |
| tanh | math_transform | 44.54 | 46.13 | 1.04x | ✅ |
| tema_20 | overlap | 28.73 | 126.42 | 4.40x | ✅ |
| trima_20 | overlap | 31.56 | 40.00 | 1.27x | ✅ |
| trix_15 | momentum | 44.43 | 131.76 | 2.97x | ✅ |
| tsf_14 | statistics_extra | 35.61 | 66.70 | 1.87x | ✅ |
| typprice | price_transform_full | 6.26 | 10.69 | 1.71x | ✅ |
| ultosc_7_14_28 | momentum | 69.10 | 57.71 | 0.84x | ⚠️ |
| var_20 | statistics | 28.74 | 25.19 | 0.88x | ⚠️ |
| wclprice | price_transform | 5.18 | 47.48 | 9.16x | ✅ |
| willr_14 | momentum | 32.30 | 35.23 | 1.09x | ✅ |
| wma_20 | overlap | 25.15 | 21.69 | 0.86x | ⚠️ |

- **Total paired benchmarks**: 90
- **Finkit faster or equal on this run**: 67

Do not convert one machine snapshot into a universal performance claim. Re-run the suite on the target deployment hardware.
