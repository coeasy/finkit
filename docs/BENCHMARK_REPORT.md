# Finkit vs TA-Lib C Benchmark Report

> Auto-generated from Criterion JSON by scripts/bench_report.py.
> Results are valid only for the recorded commit, CPU, compiler, build flags, dataset, and TA-Lib version.

| Indicator | Category | Finkit (µs) | TA-Lib C (µs) | Speedup | Status |
|---|---|---:|---:|---:|:---:|
| acos | math_transform | 41.55 | 132.85 | 3.20x | ✅ |
| ad | volume | 12.11 | 11.98 | 0.99x | ⚠️ |
| add | math_operators | 3.36 | 6.15 | 1.83x | ✅ |
| adosc_3_10 | volume | 92.32 | 97.74 | 1.06x | ✅ |
| adx_14 | directional | 74.49 | 73.24 | 0.98x | ⚠️ |
| adxr_14 | directional | 81.43 | 76.43 | 0.94x | ⚠️ |
| apo_12_26 | momentum_extra | 16.38 | 42.38 | 2.59x | ✅ |
| aroon_14 | directional | 55.13 | 48.73 | 0.88x | ⚠️ |
| aroonosc_14 | momentum | 64.81 | 45.85 | 0.71x | ❌ |
| atr_14 | volatility | 28.53 | 47.29 | 1.66x | ✅ |
| avgdev_14 | statistics_extra | 98.98 | 178.32 | 1.80x | ✅ |
| avgprice | price_transform_full | 6.64 | 7.87 | 1.18x | ✅ |
| bbands_20 | overlap | 29.28 | 49.20 | 1.68x | ✅ |
| bbands_20@10000 | scaled_10k | 29.29 | 47.30 | 1.61x | ✅ |
| bbands_20@100000 | scaled_100k | 698.67 | 962.77 | 1.38x | ✅ |
| bbands_20@1000000 | scaled_1m | 7485.53 | 9554.55 | 1.28x | ✅ |
| bop | momentum_extra | 6.64 | 10.61 | 1.60x | ✅ |
| cci_14 | momentum | 186.52 | 216.88 | 1.16x | ✅ |
| ceil | math_transform | 1.54 | 27.42 | 17.79x | ✅ |
| cmo_14 | momentum | 36.45 | 52.34 | 1.44x | ✅ |
| correl_30 | statistics_extra | 60.81 | 73.02 | 1.20x | ✅ |
| cos | math_transform | 22.85 | 25.43 | 1.11x | ✅ |
| dema_20 | overlap | 28.85 | 85.08 | 2.95x | ✅ |
| ema_12 | overlap | 17.96 | 45.52 | 2.53x | ✅ |
| ema_12@10000 | scaled_10k | 17.95 | 45.74 | 2.55x | ✅ |
| ema_12@100000 | scaled_100k | 175.25 | 465.52 | 2.66x | ✅ |
| ema_12@1000000 | scaled_1m | 2981.00 | 5611.66 | 1.88x | ✅ |
| exp | math_transform | 23.77 | 42.65 | 1.79x | ✅ |
| floor | math_transform | 2.16 | 27.36 | 12.64x | ✅ |
| ht_dcperiod | cycle_extra | 459.89 | 588.79 | 1.28x | ✅ |
| ht_dcphase | cycle_extra | 857.01 | 3164.64 | 3.69x | ✅ |
| ht_phasor | cycle | 475.19 | 560.37 | 1.18x | ✅ |
| ht_sine | cycle | 1193.87 | 3418.15 | 2.86x | ✅ |
| ht_trendline | cycle_extra | 469.66 | 884.73 | 1.88x | ✅ |
| kama_30 | overlap | 24.97 | 98.15 | 3.93x | ✅ |
| linearreg_14 | statistics | 35.33 | 65.62 | 1.86x | ✅ |
| linreg_angle_14 | statistics_extra | 94.34 | 89.44 | 0.95x | ⚠️ |
| linreg_intercept_14 | statistics_extra | 31.30 | 25.95 | 0.83x | ⚠️ |
| linreg_slope_14 | statistics | 31.34 | 23.91 | 0.76x | ❌ |
| ln | math_transform | 27.58 | 44.90 | 1.63x | ✅ |
| ma_20 | overlap_extra | 11.87 | 19.35 | 1.63x | ✅ |
| macd@10000 | scaled_10k | 58.23 | 137.00 | 2.35x | ✅ |
| macd@100000 | scaled_100k | 937.22 | 1833.29 | 1.96x | ✅ |
| macd@1000000 | scaled_1m | 9703.02 | 16218.20 | 1.67x | ✅ |
| macd_12_26_9 | momentum | 58.66 | 139.40 | 2.38x | ✅ |
| mama | momentum_extra | 524.88 | 884.43 | 1.69x | ✅ |
| max_30 | math_operators | 15.46 | 14.63 | 0.95x | ⚠️ |
| medprice | price_transform_full | 3.91 | 5.69 | 1.45x | ✅ |
| mfi_14 | momentum | 42.19 | 63.60 | 1.51x | ✅ |
| min_30 | math_operators | 15.51 | 14.23 | 0.92x | ⚠️ |
| minus_di_14 | directional | 56.74 | 56.83 | 1.00x | ✅ |
| minus_dm_14 | momentum | 9.50 | 45.34 | 4.77x | ✅ |
| mom_10 | momentum | 3.35 | 5.13 | 1.53x | ✅ |
| mult | math_operators | 3.11 | 6.20 | 1.99x | ✅ |
| natr_14 | volatility | 31.15 | 51.51 | 1.65x | ✅ |
| obv | volume | 9.05 | 10.50 | 1.16x | ✅ |
| percentrank_30 | statistics_extra | 293.76 | 317.38 | 1.08x | ✅ |
| plus_di_14 | directional | 55.99 | 57.62 | 1.03x | ✅ |
| plus_dm_14 | momentum | 9.86 | 51.09 | 5.18x | ✅ |
| ppo_12_26 | momentum_extra | 19.56 | 47.09 | 2.41x | ✅ |
| roc_10 | momentum | 5.89 | 10.38 | 1.76x | ✅ |
| rsi_14 | momentum | 22.20 | 31.01 | 1.40x | ✅ |
| rsi_14@10000 | scaled_10k | 22.07 | 30.26 | 1.37x | ✅ |
| rsi_14@100000 | scaled_100k | 219.77 | 300.41 | 1.37x | ✅ |
| rsi_14@1000000 | scaled_1m | 3741.61 | 3957.28 | 1.06x | ✅ |
| sar | overlap_extra | 42.56 | 62.32 | 1.46x | ✅ |
| sin | math_transform | 19.87 | 23.29 | 1.17x | ✅ |
| sma_20 | overlap | 12.32 | 19.19 | 1.56x | ✅ |
| sma_20@10000 | scaled_10k | 11.31 | 18.94 | 1.68x | ✅ |
| sma_20@100000 | scaled_100k | 109.37 | 194.31 | 1.78x | ✅ |
| sma_20@1000000 | scaled_1m | 2620.41 | 2992.56 | 1.14x | ✅ |
| sqrt | math_transform | 8.05 | 14.51 | 1.80x | ✅ |
| stddev_20 | volatility | 28.30 | 37.65 | 1.33x | ✅ |
| stoch_14_3_3 | momentum | 87.62 | 108.56 | 1.24x | ✅ |
| stochf_14_3 | momentum | 84.91 | 87.41 | 1.03x | ✅ |
| stochrsi_14_14_3_3 | momentum | 99.83 | 128.83 | 1.29x | ✅ |
| sub | math_operators | 3.11 | 5.71 | 1.84x | ✅ |
| sum_30 | math_operators | 11.20 | 18.88 | 1.69x | ✅ |
| t3_5 | overlap_extra | 38.32 | 346.83 | 9.05x | ✅ |
| tanh | math_transform | 59.23 | 45.83 | 0.77x | ❌ |
| tema_20 | overlap | 27.15 | 122.95 | 4.53x | ✅ |
| trima_20 | overlap | 28.77 | 28.13 | 0.98x | ⚠️ |
| trix_15 | momentum | 65.13 | 135.83 | 2.09x | ✅ |
| tsf_14 | statistics_extra | 36.53 | 67.43 | 1.85x | ✅ |
| typprice | price_transform_full | 6.08 | 10.58 | 1.74x | ✅ |
| ultosc_7_14_28 | momentum | 73.09 | 58.73 | 0.80x | ⚠️ |
| var_20 | statistics | 28.70 | 25.63 | 0.89x | ⚠️ |
| wclprice | price_transform | 4.94 | 47.86 | 9.68x | ✅ |
| willr_14 | momentum | 32.51 | 37.34 | 1.15x | ✅ |
| wma_20 | overlap | 20.98 | 20.47 | 0.98x | ⚠️ |

- **Total paired benchmarks**: 90
- **Finkit faster or equal on this run**: 75

Do not convert one machine snapshot into a universal performance claim. Re-run the suite on the target deployment hardware.
