# Finkit vs TA-Lib C Benchmark Report

> Auto-generated from Criterion JSON by scripts/bench_report.py.
> Results are valid only for the recorded commit, CPU, compiler, build flags, dataset, and TA-Lib version.

| Indicator | Category | Finkit (µs) | TA-Lib C (µs) | Speedup | Status |
|---|---|---:|---:|---:|:---:|
| acos | math_transform | 137.24 | 134.18 | 0.98x | ⚠️ |
| ad | volume | 12.67 | 12.42 | 0.98x | ⚠️ |
| add | math_operators | 2.62 | 5.83 | 2.23x | ✅ |
| adosc_3_10 | volume | 93.42 | 99.46 | 1.06x | ✅ |
| adx_14 | directional | 77.12 | 72.14 | 0.94x | ⚠️ |
| adxr_14 | directional | 83.82 | 74.99 | 0.89x | ⚠️ |
| apo_12_26 | momentum_extra | 17.58 | 41.49 | 2.36x | ✅ |
| aroon_14 | directional | 51.28 | 48.11 | 0.94x | ⚠️ |
| aroonosc_14 | momentum | 45.27 | 42.18 | 0.93x | ⚠️ |
| atr_14 | volatility | 27.95 | 49.53 | 1.77x | ✅ |
| avgdev_14 | statistics_extra | 100.35 | 157.72 | 1.57x | ✅ |
| avgprice | price_transform_full | 6.37 | 7.82 | 1.23x | ✅ |
| bbands_20 | overlap | 31.75 | 49.12 | 1.55x | ✅ |
| bbands_20@10000 | scaled_10k | 30.24 | 47.17 | 1.56x | ✅ |
| bbands_20@100000 | scaled_100k | 673.69 | 946.84 | 1.41x | ✅ |
| bbands_20@1000000 | scaled_1m | 8121.68 | 10196.28 | 1.26x | ✅ |
| bop | momentum_extra | 6.67 | 10.42 | 1.56x | ✅ |
| cci_14 | momentum | 180.37 | 210.79 | 1.17x | ✅ |
| ceil | math_transform | 1.94 | 23.78 | 12.25x | ✅ |
| cmo_14 | momentum | 35.64 | 50.37 | 1.41x | ✅ |
| correl_30 | statistics_extra | 60.46 | 74.39 | 1.23x | ✅ |
| cos | math_transform | 23.18 | 23.62 | 1.02x | ✅ |
| dema_20 | overlap | 27.89 | 82.63 | 2.96x | ✅ |
| ema_12 | overlap | 17.77 | 45.14 | 2.54x | ✅ |
| ema_12@10000 | scaled_10k | 17.72 | 45.10 | 2.54x | ✅ |
| ema_12@100000 | scaled_100k | 174.70 | 465.32 | 2.66x | ✅ |
| ema_12@1000000 | scaled_1m | 3079.59 | 5666.05 | 1.84x | ✅ |
| exp | math_transform | 40.09 | 41.84 | 1.04x | ✅ |
| floor | math_transform | 1.86 | 23.71 | 12.73x | ✅ |
| ht_dcperiod | cycle_extra | 450.55 | 579.52 | 1.29x | ✅ |
| ht_dcphase | cycle_extra | 871.53 | 3171.88 | 3.64x | ✅ |
| ht_phasor | cycle | 458.39 | 552.61 | 1.21x | ✅ |
| ht_sine | cycle | 1176.84 | 3370.50 | 2.86x | ✅ |
| ht_trendline | cycle_extra | 466.04 | 869.13 | 1.86x | ✅ |
| kama_30 | overlap | 23.95 | 97.82 | 4.08x | ✅ |
| linearreg_14 | statistics | 26.89 | 65.14 | 2.42x | ✅ |
| linreg_angle_14 | statistics_extra | 93.75 | 89.13 | 0.95x | ⚠️ |
| linreg_intercept_14 | statistics_extra | 22.29 | 25.38 | 1.14x | ✅ |
| linreg_slope_14 | statistics | 22.22 | 23.28 | 1.05x | ✅ |
| ln | math_transform | 45.67 | 45.11 | 0.99x | ⚠️ |
| ma_20 | overlap_extra | 10.83 | 19.00 | 1.75x | ✅ |
| macd@10000 | scaled_10k | 58.52 | 137.47 | 2.35x | ✅ |
| macd@100000 | scaled_100k | 949.02 | 1839.02 | 1.94x | ✅ |
| macd@1000000 | scaled_1m | 9872.97 | 16213.15 | 1.64x | ✅ |
| macd_12_26_9 | momentum | 58.18 | 137.82 | 2.37x | ✅ |
| mama | momentum_extra | 509.23 | 878.86 | 1.73x | ✅ |
| max_30 | math_operators | 15.76 | 14.32 | 0.91x | ⚠️ |
| medprice | price_transform_full | 3.87 | 5.64 | 1.46x | ✅ |
| mfi_14 | momentum | 31.91 | 57.66 | 1.81x | ✅ |
| min_30 | math_operators | 15.03 | 14.51 | 0.97x | ⚠️ |
| minus_di_14 | directional | 55.70 | 57.04 | 1.02x | ✅ |
| minus_dm_14 | momentum | 7.73 | 45.08 | 5.84x | ✅ |
| mom_10 | momentum | 3.15 | 5.10 | 1.62x | ✅ |
| mult | math_operators | 2.76 | 5.52 | 2.00x | ✅ |
| natr_14 | volatility | 30.83 | 50.65 | 1.64x | ✅ |
| obv | volume | 9.00 | 10.27 | 1.14x | ✅ |
| percentrank_30 | statistics_extra | 68.17 | 344.31 | 5.05x | ✅ |
| plus_di_14 | directional | 55.76 | 56.37 | 1.01x | ✅ |
| plus_dm_14 | momentum | 8.35 | 45.06 | 5.40x | ✅ |
| ppo_12_26 | momentum_extra | 19.11 | 46.56 | 2.44x | ✅ |
| roc_10 | momentum | 5.88 | 10.27 | 1.75x | ✅ |
| rsi_14 | momentum | 21.84 | 30.17 | 1.38x | ✅ |
| rsi_14@10000 | scaled_10k | 21.98 | 29.95 | 1.36x | ✅ |
| rsi_14@100000 | scaled_100k | 219.65 | 300.24 | 1.37x | ✅ |
| rsi_14@1000000 | scaled_1m | 3740.44 | 4014.18 | 1.07x | ✅ |
| sar | overlap_extra | 41.27 | 57.99 | 1.41x | ✅ |
| sin | math_transform | 24.06 | 23.92 | 0.99x | ⚠️ |
| sma_20 | overlap | 10.98 | 18.99 | 1.73x | ✅ |
| sma_20@10000 | scaled_10k | 11.02 | 18.93 | 1.72x | ✅ |
| sma_20@100000 | scaled_100k | 110.90 | 193.31 | 1.74x | ✅ |
| sma_20@1000000 | scaled_1m | 2862.66 | 3093.41 | 1.08x | ✅ |
| sqrt | math_transform | 8.19 | 14.66 | 1.79x | ✅ |
| stddev_20 | volatility | 28.11 | 37.60 | 1.34x | ✅ |
| stoch_14_3_3 | momentum | 78.88 | 94.71 | 1.20x | ✅ |
| stochf_14_3 | momentum | 66.09 | 76.97 | 1.16x | ✅ |
| stochrsi_14_14_3_3 | momentum | 90.18 | 104.67 | 1.16x | ✅ |
| sub | math_operators | 2.71 | 5.52 | 2.04x | ✅ |
| sum_30 | math_operators | 11.17 | 18.78 | 1.68x | ✅ |
| t3_5 | overlap_extra | 37.32 | 343.12 | 9.19x | ✅ |
| tanh | math_transform | 43.55 | 44.58 | 1.02x | ✅ |
| tema_20 | overlap | 26.38 | 119.28 | 4.52x | ✅ |
| trima_20 | overlap | 28.47 | 28.03 | 0.98x | ⚠️ |
| trix_15 | momentum | 40.84 | 123.13 | 3.01x | ✅ |
| tsf_14 | statistics_extra | 35.00 | 65.25 | 1.86x | ✅ |
| typprice | price_transform_full | 6.01 | 10.33 | 1.72x | ✅ |
| ultosc_7_14_28 | momentum | 68.89 | 56.26 | 0.82x | ⚠️ |
| var_20 | statistics | 27.66 | 25.00 | 0.90x | ⚠️ |
| wclprice | price_transform | 4.75 | 46.38 | 9.76x | ✅ |
| willr_14 | momentum | 31.22 | 33.27 | 1.07x | ✅ |
| wma_20 | overlap | 20.53 | 20.38 | 0.99x | ⚠️ |

- **Total paired benchmarks**: 90
- **Finkit faster or equal on this run**: 75

Do not convert one machine snapshot into a universal performance claim. Re-run the suite on the target deployment hardware.
