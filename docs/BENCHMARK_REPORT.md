# Finkit vs TA-Lib C Benchmark Report

> Auto-generated from Criterion JSON by scripts/bench_report.py.
> Results are valid only for the recorded commit, CPU, compiler, build flags, dataset, and TA-Lib version.

| Indicator | Category | Finkit (µs) | TA-Lib C (µs) | Speedup | Status |
|---|---|---:|---:|---:|:---:|
| acos | math_transform | 135.95 | 135.01 | 0.99x | ⚠️ |
| ad | volume | 13.07 | 12.83 | 0.98x | ⚠️ |
| add | math_operators | 2.87 | 5.76 | 2.01x | ✅ |
| adosc_3_10 | volume | 95.33 | 101.25 | 1.06x | ✅ |
| adx_14 | directional | 61.98 | 73.66 | 1.19x | ✅ |
| adxr_14 | directional | 71.53 | 78.44 | 1.10x | ✅ |
| apo_12_26 | momentum_extra | 17.17 | 43.57 | 2.54x | ✅ |
| aroon_14 | directional | 53.52 | 52.51 | 0.98x | ⚠️ |
| aroonosc_14 | momentum | 46.56 | 43.57 | 0.94x | ⚠️ |
| atr_14 | volatility | 28.73 | 47.76 | 1.66x | ✅ |
| avgdev_14 | statistics_extra | 100.65 | 145.23 | 1.44x | ✅ |
| avgprice | price_transform_full | 6.67 | 8.47 | 1.27x | ✅ |
| bbands_20 | overlap | 37.18 | 54.56 | 1.47x | ✅ |
| bbands_20@10000 | scaled_10k | 34.51 | 53.83 | 1.56x | ✅ |
| bbands_20@100000 | scaled_100k | 751.76 | 1029.62 | 1.37x | ✅ |
| bbands_20@1000000 | scaled_1m | 8379.50 | 10171.47 | 1.21x | ✅ |
| bop | momentum_extra | 6.89 | 10.74 | 1.56x | ✅ |
| cci_14 | momentum | 184.07 | 216.76 | 1.18x | ✅ |
| ceil | math_transform | 1.57 | 24.18 | 15.44x | ✅ |
| cmo_14 | momentum | 38.35 | 51.21 | 1.34x | ✅ |
| correl_30 | statistics_extra | 60.88 | 74.89 | 1.23x | ✅ |
| cos | math_transform | 22.42 | 24.13 | 1.08x | ✅ |
| dema_20 | overlap | 29.96 | 86.32 | 2.88x | ✅ |
| ema_12 | overlap | 21.18 | 49.35 | 2.33x | ✅ |
| ema_12@10000 | scaled_10k | 18.22 | 46.44 | 2.55x | ✅ |
| ema_12@100000 | scaled_100k | 184.27 | 478.14 | 2.59x | ✅ |
| ema_12@1000000 | scaled_1m | 3227.00 | 5890.10 | 1.83x | ✅ |
| exp | math_transform | 41.13 | 42.63 | 1.04x | ✅ |
| floor | math_transform | 1.56 | 24.15 | 15.45x | ✅ |
| ht_dcperiod | cycle_extra | 473.32 | 623.16 | 1.32x | ✅ |
| ht_dcphase | cycle_extra | 908.24 | 3660.10 | 4.03x | ✅ |
| ht_phasor | cycle | 475.83 | 570.76 | 1.20x | ✅ |
| ht_sine | cycle | 1210.41 | 3402.49 | 2.81x | ✅ |
| ht_trendline | cycle_extra | 491.64 | 880.32 | 1.79x | ✅ |
| kama_30 | overlap | 25.95 | 100.36 | 3.87x | ✅ |
| linearreg_14 | statistics | 24.66 | 67.71 | 2.75x | ✅ |
| linreg_angle_14 | statistics_extra | 91.82 | 88.99 | 0.97x | ⚠️ |
| linreg_intercept_14 | statistics_extra | 22.52 | 25.69 | 1.14x | ✅ |
| linreg_slope_14 | statistics | 22.78 | 24.06 | 1.06x | ✅ |
| ln | math_transform | 46.59 | 45.91 | 0.99x | ⚠️ |
| ma_20 | overlap_extra | 11.66 | 20.99 | 1.80x | ✅ |
| macd@10000 | scaled_10k | 64.46 | 146.36 | 2.27x | ✅ |
| macd@100000 | scaled_100k | 1007.47 | 1949.35 | 1.93x | ✅ |
| macd@1000000 | scaled_1m | 10209.04 | 16785.43 | 1.64x | ✅ |
| macd_12_26_9 | momentum | 64.12 | 147.09 | 2.29x | ✅ |
| mama | momentum_extra | 514.05 | 893.81 | 1.74x | ✅ |
| max_30 | math_operators | 15.27 | 14.79 | 0.97x | ⚠️ |
| medprice | price_transform_full | 4.12 | 5.79 | 1.41x | ✅ |
| mfi_14 | momentum | 29.74 | 59.69 | 2.01x | ✅ |
| min_30 | math_operators | 16.30 | 14.83 | 0.91x | ⚠️ |
| minus_di_14 | directional | 58.92 | 58.45 | 0.99x | ⚠️ |
| minus_dm_14 | momentum | 8.19 | 46.27 | 5.65x | ✅ |
| mom_10 | momentum | 3.66 | 5.46 | 1.49x | ✅ |
| mult | math_operators | 2.90 | 5.80 | 2.00x | ✅ |
| natr_14 | volatility | 33.06 | 53.55 | 1.62x | ✅ |
| obv | volume | 10.09 | 12.14 | 1.20x | ✅ |
| percentrank_30 | statistics_extra | 70.78 | 372.42 | 5.26x | ✅ |
| plus_di_14 | directional | 56.92 | 57.76 | 1.01x | ✅ |
| plus_dm_14 | momentum | 8.72 | 46.38 | 5.32x | ✅ |
| ppo_12_26 | momentum_extra | 19.58 | 48.04 | 2.45x | ✅ |
| roc_10 | momentum | 6.12 | 10.65 | 1.74x | ✅ |
| rsi_14 | momentum | 23.03 | 30.99 | 1.35x | ✅ |
| rsi_14@10000 | scaled_10k | 22.70 | 30.77 | 1.36x | ✅ |
| rsi_14@100000 | scaled_100k | 229.73 | 313.49 | 1.36x | ✅ |
| rsi_14@1000000 | scaled_1m | 4083.18 | 4194.34 | 1.03x | ✅ |
| sar | overlap_extra | 44.17 | 59.34 | 1.34x | ✅ |
| sin | math_transform | 23.52 | 23.90 | 1.02x | ✅ |
| sma_20 | overlap | 11.09 | 22.53 | 2.03x | ✅ |
| sma_20@10000 | scaled_10k | 11.46 | 19.47 | 1.70x | ✅ |
| sma_20@100000 | scaled_100k | 119.22 | 203.53 | 1.71x | ✅ |
| sma_20@1000000 | scaled_1m | 3085.61 | 3390.46 | 1.10x | ✅ |
| sqrt | math_transform | 8.31 | 15.12 | 1.82x | ✅ |
| stddev_20 | volatility | 30.16 | 41.16 | 1.36x | ✅ |
| stoch_14_3_3 | momentum | 82.75 | 102.03 | 1.23x | ✅ |
| stochf_14_3 | momentum | 69.21 | 85.07 | 1.23x | ✅ |
| stochrsi_14_14_3_3 | momentum | 94.39 | 117.84 | 1.25x | ✅ |
| sub | math_operators | 2.92 | 5.80 | 1.99x | ✅ |
| sum_30 | math_operators | 11.96 | 19.24 | 1.61x | ✅ |
| t3_5 | overlap_extra | 38.64 | 348.63 | 9.02x | ✅ |
| tanh | math_transform | 46.15 | 49.48 | 1.07x | ✅ |
| tema_20 | overlap | 28.60 | 124.28 | 4.35x | ✅ |
| trima_20 | overlap | 30.48 | 30.01 | 0.98x | ⚠️ |
| trix_15 | momentum | 42.96 | 127.67 | 2.97x | ✅ |
| tsf_14 | statistics_extra | 35.73 | 66.51 | 1.86x | ✅ |
| typprice | price_transform_full | 6.19 | 10.61 | 1.72x | ✅ |
| ultosc_7_14_28 | momentum | 61.84 | 57.81 | 0.93x | ⚠️ |
| var_20 | statistics | 28.16 | 26.16 | 0.93x | ⚠️ |
| wclprice | price_transform | 5.16 | 47.98 | 9.30x | ✅ |
| willr_14 | momentum | 32.17 | 34.15 | 1.06x | ✅ |
| wma_20 | overlap | 23.72 | 23.73 | 1.00x | ✅ |

- **Total paired benchmarks**: 90
- **Finkit faster or equal on this run**: 78

Do not convert one machine snapshot into a universal performance claim. Re-run the suite on the target deployment hardware.
