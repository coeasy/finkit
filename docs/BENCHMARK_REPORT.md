# Finkit vs TA-Lib C Benchmark Report

> Auto-generated from Criterion JSON by scripts/bench_report.py.
> Results are valid only for the recorded commit, CPU, compiler, build flags, dataset, and TA-Lib version.

| Indicator | Category | Finkit (µs) | TA-Lib C (µs) | Speedup | Status |
|---|---|---:|---:|---:|:---:|
| acos | math_transform | 137.48 | 136.12 | 0.99x | ⚠️ |
| ad | volume | 12.52 | 12.69 | 1.01x | ✅ |
| add | math_operators | 3.25 | 6.10 | 1.88x | ✅ |
| adosc_3_10 | volume | 94.83 | 101.29 | 1.07x | ✅ |
| adx_14 | directional | 77.16 | 73.09 | 0.95x | ⚠️ |
| adxr_14 | directional | 70.06 | 76.22 | 1.09x | ✅ |
| apo_12_26 | momentum_extra | 19.25 | 46.10 | 2.40x | ✅ |
| aroon_14 | directional | 51.69 | 48.99 | 0.95x | ⚠️ |
| aroonosc_14 | momentum | 46.53 | 43.06 | 0.93x | ⚠️ |
| atr_14 | volatility | 29.02 | 49.89 | 1.72x | ✅ |
| avgdev_14 | statistics_extra | 99.66 | 147.03 | 1.48x | ✅ |
| avgprice | price_transform_full | 6.78 | 8.19 | 1.21x | ✅ |
| bbands_20 | overlap | 32.11 | 47.98 | 1.49x | ✅ |
| bbands_20@10000 | scaled_10k | 31.49 | 50.23 | 1.59x | ✅ |
| bbands_20@100000 | scaled_100k | 836.67 | 1141.14 | 1.36x | ✅ |
| bbands_20@1000000 | scaled_1m | 7915.69 | 10074.50 | 1.27x | ✅ |
| bop | momentum_extra | 6.93 | 10.82 | 1.56x | ✅ |
| cci_14 | momentum | 181.44 | 212.24 | 1.17x | ✅ |
| ceil | math_transform | 1.98 | 25.48 | 12.89x | ✅ |
| cmo_14 | momentum | 36.24 | 51.06 | 1.41x | ✅ |
| correl_30 | statistics_extra | 61.15 | 75.42 | 1.23x | ✅ |
| cos | math_transform | 24.40 | 24.35 | 1.00x | ⚠️ |
| dema_20 | overlap | 28.71 | 84.96 | 2.96x | ✅ |
| ema_12 | overlap | 18.54 | 46.18 | 2.49x | ✅ |
| ema_12@10000 | scaled_10k | 18.71 | 47.54 | 2.54x | ✅ |
| ema_12@100000 | scaled_100k | 187.01 | 506.96 | 2.71x | ✅ |
| ema_12@1000000 | scaled_1m | 3321.83 | 5835.81 | 1.76x | ✅ |
| exp | math_transform | 42.50 | 54.20 | 1.28x | ✅ |
| floor | math_transform | 1.95 | 25.37 | 13.03x | ✅ |
| ht_dcperiod | cycle_extra | 474.83 | 623.03 | 1.31x | ✅ |
| ht_dcphase | cycle_extra | 916.09 | 3348.32 | 3.66x | ✅ |
| ht_phasor | cycle | 491.74 | 588.02 | 1.20x | ✅ |
| ht_sine | cycle | 1289.99 | 3643.52 | 2.82x | ✅ |
| ht_trendline | cycle_extra | 483.66 | 919.41 | 1.90x | ✅ |
| kama_30 | overlap | 24.84 | 95.82 | 3.86x | ✅ |
| linearreg_14 | statistics | 29.06 | 72.06 | 2.48x | ✅ |
| linreg_angle_14 | statistics_extra | 94.67 | 90.82 | 0.96x | ⚠️ |
| linreg_intercept_14 | statistics_extra | 22.59 | 26.21 | 1.16x | ✅ |
| linreg_slope_14 | statistics | 23.76 | 25.01 | 1.05x | ✅ |
| ln | math_transform | 48.44 | 46.89 | 0.97x | ⚠️ |
| ma_20 | overlap_extra | 13.04 | 20.98 | 1.61x | ✅ |
| macd@10000 | scaled_10k | 59.76 | 141.85 | 2.37x | ✅ |
| macd@100000 | scaled_100k | 1081.19 | 2157.33 | 2.00x | ✅ |
| macd@1000000 | scaled_1m | 12727.22 | 16658.93 | 1.31x | ✅ |
| macd_12_26_9 | momentum | 58.52 | 146.01 | 2.49x | ✅ |
| mama | momentum_extra | 550.00 | 914.62 | 1.66x | ✅ |
| max_30 | math_operators | 15.79 | 14.80 | 0.94x | ⚠️ |
| medprice | price_transform_full | 4.16 | 6.30 | 1.51x | ✅ |
| mfi_14 | momentum | 33.58 | 60.02 | 1.79x | ✅ |
| min_30 | math_operators | 16.08 | 16.69 | 1.04x | ✅ |
| minus_di_14 | directional | 56.31 | 58.25 | 1.03x | ✅ |
| minus_dm_14 | momentum | 7.77 | 45.47 | 5.85x | ✅ |
| mom_10 | momentum | 3.68 | 5.30 | 1.44x | ✅ |
| mult | math_operators | 3.37 | 5.95 | 1.77x | ✅ |
| natr_14 | volatility | 31.72 | 52.24 | 1.65x | ✅ |
| obv | volume | 10.11 | 11.69 | 1.16x | ✅ |
| percentrank_30 | statistics_extra | 69.86 | 362.15 | 5.18x | ✅ |
| plus_di_14 | directional | 55.86 | 56.99 | 1.02x | ✅ |
| plus_dm_14 | momentum | 8.15 | 45.39 | 5.57x | ✅ |
| ppo_12_26 | momentum_extra | 21.27 | 52.43 | 2.46x | ✅ |
| roc_10 | momentum | 6.35 | 10.33 | 1.63x | ✅ |
| rsi_14 | momentum | 21.92 | 30.03 | 1.37x | ✅ |
| rsi_14@10000 | scaled_10k | 23.18 | 31.44 | 1.36x | ✅ |
| rsi_14@100000 | scaled_100k | 239.78 | 332.98 | 1.39x | ✅ |
| rsi_14@1000000 | scaled_1m | 4874.09 | 4478.89 | 0.92x | ⚠️ |
| sar | overlap_extra | 45.21 | 65.13 | 1.44x | ✅ |
| sin | math_transform | 24.64 | 24.46 | 0.99x | ⚠️ |
| sma_20 | overlap | 12.60 | 19.58 | 1.55x | ✅ |
| sma_20@10000 | scaled_10k | 12.28 | 20.15 | 1.64x | ✅ |
| sma_20@100000 | scaled_100k | 118.88 | 206.31 | 1.74x | ✅ |
| sma_20@1000000 | scaled_1m | 3156.95 | 3301.39 | 1.05x | ✅ |
| sqrt | math_transform | 8.58 | 15.40 | 1.79x | ✅ |
| stddev_20 | volatility | 28.72 | 38.98 | 1.36x | ✅ |
| stoch_14_3_3 | momentum | 81.96 | 96.26 | 1.17x | ✅ |
| stochf_14_3 | momentum | 67.23 | 77.73 | 1.16x | ✅ |
| stochrsi_14_14_3_3 | momentum | 92.06 | 108.06 | 1.17x | ✅ |
| sub | math_operators | 3.22 | 6.23 | 1.94x | ✅ |
| sum_30 | math_operators | 13.20 | 20.27 | 1.54x | ✅ |
| t3_5 | overlap_extra | 42.52 | 359.21 | 8.45x | ✅ |
| tanh | math_transform | 47.42 | 47.36 | 1.00x | ⚠️ |
| tema_20 | overlap | 26.23 | 121.64 | 4.64x | ✅ |
| trima_20 | overlap | 28.87 | 28.32 | 0.98x | ⚠️ |
| trix_15 | momentum | 41.35 | 124.98 | 3.02x | ✅ |
| tsf_14 | statistics_extra | 36.27 | 66.65 | 1.84x | ✅ |
| typprice | price_transform_full | 6.20 | 11.18 | 1.80x | ✅ |
| ultosc_7_14_28 | momentum | 66.86 | 57.09 | 0.85x | ⚠️ |
| var_20 | statistics | 30.83 | 31.82 | 1.03x | ✅ |
| wclprice | price_transform | 5.37 | 50.07 | 9.33x | ✅ |
| willr_14 | momentum | 31.30 | 33.46 | 1.07x | ✅ |
| wma_20 | overlap | 21.58 | 20.66 | 0.96x | ⚠️ |

- **Total paired benchmarks**: 90
- **Finkit faster or equal on this run**: 76

Do not convert one machine snapshot into a universal performance claim. Re-run the suite on the target deployment hardware.
