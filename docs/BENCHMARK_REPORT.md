# Finkit vs TA-Lib C Benchmark Report

> Auto-generated from Criterion JSON by scripts/bench_report.py.
> Results are valid only for the recorded commit, CPU, compiler, build flags, dataset, and TA-Lib version.

| Indicator | Category | Finkit (µs) | TA-Lib C (µs) | Speedup | Status |
|---|---|---:|---:|---:|:---:|
| acos | math_transform | 52.52 | 136.27 | 2.59x | ✅ |
| ad | volume | 12.35 | 12.63 | 1.02x | ✅ |
| add | math_operators | 2.87 | 5.84 | 2.03x | ✅ |
| adosc_3_10 | volume | 96.27 | 102.09 | 1.06x | ✅ |
| adx_14 | directional | 81.47 | 74.80 | 0.92x | ⚠️ |
| adxr_14 | directional | 84.48 | 77.95 | 0.92x | ⚠️ |
| apo_12_26 | momentum_extra | 16.48 | 41.46 | 2.52x | ✅ |
| aroon_14 | directional | 56.57 | 49.82 | 0.88x | ⚠️ |
| aroonosc_14 | momentum | 54.32 | 45.46 | 0.84x | ⚠️ |
| atr_14 | volatility | 28.80 | 48.80 | 1.69x | ✅ |
| avgdev_14 | statistics_extra | 104.62 | 161.88 | 1.55x | ✅ |
| avgprice | price_transform_full | 6.44 | 7.78 | 1.21x | ✅ |
| bbands_20 | overlap | 36.88 | 48.67 | 1.32x | ✅ |
| bbands_20@10000 | scaled_10k | 34.81 | 48.27 | 1.39x | ✅ |
| bbands_20@100000 | scaled_100k | 863.34 | 1032.02 | 1.20x | ✅ |
| bbands_20@1000000 | scaled_1m | 12712.75 | 11111.94 | 0.87x | ⚠️ |
| bop | momentum_extra | 6.69 | 10.63 | 1.59x | ✅ |
| cci_14 | momentum | 185.08 | 219.20 | 1.18x | ✅ |
| ceil | math_transform | 31.19 | 30.32 | 0.97x | ⚠️ |
| cmo_14 | momentum | 36.81 | 51.31 | 1.39x | ✅ |
| correl_30 | statistics_extra | 61.12 | 90.67 | 1.48x | ✅ |
| cos | math_transform | 22.29 | 23.86 | 1.07x | ✅ |
| dema_20 | overlap | 30.17 | 85.16 | 2.82x | ✅ |
| ema_12 | overlap | 18.56 | 46.48 | 2.50x | ✅ |
| ema_12@10000 | scaled_10k | 18.66 | 46.23 | 2.48x | ✅ |
| ema_12@100000 | scaled_100k | 179.46 | 474.47 | 2.64x | ✅ |
| ema_12@1000000 | scaled_1m | 3346.25 | 5970.55 | 1.78x | ✅ |
| exp | math_transform | 34.97 | 43.68 | 1.25x | ✅ |
| floor | math_transform | 33.66 | 25.23 | 0.75x | ❌ |
| ht_dcperiod | cycle_extra | 464.08 | 594.25 | 1.28x | ✅ |
| ht_dcphase | cycle_extra | 904.18 | 3178.93 | 3.52x | ✅ |
| ht_phasor | cycle | 460.11 | 548.42 | 1.19x | ✅ |
| ht_sine | cycle | 1177.61 | 3370.40 | 2.86x | ✅ |
| ht_trendline | cycle_extra | 461.95 | 862.26 | 1.87x | ✅ |
| kama_30 | overlap | 25.25 | 97.79 | 3.87x | ✅ |
| linearreg_14 | statistics | 36.70 | 68.45 | 1.87x | ✅ |
| linreg_angle_14 | statistics_extra | 93.62 | 92.08 | 0.98x | ⚠️ |
| linreg_intercept_14 | statistics_extra | 30.44 | 25.14 | 0.83x | ⚠️ |
| linreg_slope_14 | statistics | 32.99 | 24.74 | 0.75x | ❌ |
| ln | math_transform | 48.42 | 46.77 | 0.97x | ⚠️ |
| ma_20 | overlap_extra | 13.60 | 18.96 | 1.39x | ✅ |
| macd@10000 | scaled_10k | 59.51 | 140.74 | 2.37x | ✅ |
| macd@100000 | scaled_100k | 994.47 | 1910.19 | 1.92x | ✅ |
| macd@1000000 | scaled_1m | 10531.53 | 17146.24 | 1.63x | ✅ |
| macd_12_26_9 | momentum | 59.95 | 138.12 | 2.30x | ✅ |
| mama | momentum_extra | 525.26 | 904.28 | 1.72x | ✅ |
| max_30 | math_operators | 16.63 | 15.39 | 0.93x | ⚠️ |
| medprice | price_transform_full | 4.47 | 5.87 | 1.31x | ✅ |
| mfi_14 | momentum | 32.93 | 63.25 | 1.92x | ✅ |
| min_30 | math_operators | 16.69 | 15.32 | 0.92x | ⚠️ |
| minus_di_14 | directional | 57.38 | 57.90 | 1.01x | ✅ |
| minus_dm_14 | momentum | 8.20 | 46.64 | 5.69x | ✅ |
| mom_10 | momentum | 3.24 | 5.28 | 1.63x | ✅ |
| mult | math_operators | 2.86 | 5.77 | 2.02x | ✅ |
| natr_14 | volatility | 32.16 | 51.91 | 1.61x | ✅ |
| obv | volume | 9.56 | 11.11 | 1.16x | ✅ |
| percentrank_30 | statistics_extra | 347.91 | 353.46 | 1.02x | ✅ |
| plus_di_14 | directional | 57.25 | 58.69 | 1.03x | ✅ |
| plus_dm_14 | momentum | 8.62 | 46.20 | 5.36x | ✅ |
| ppo_12_26 | momentum_extra | 19.17 | 46.11 | 2.41x | ✅ |
| roc_10 | momentum | 6.02 | 10.58 | 1.76x | ✅ |
| rsi_14 | momentum | 22.60 | 30.89 | 1.37x | ✅ |
| rsi_14@10000 | scaled_10k | 22.55 | 30.64 | 1.36x | ✅ |
| rsi_14@100000 | scaled_100k | 226.36 | 307.62 | 1.36x | ✅ |
| rsi_14@1000000 | scaled_1m | 4129.62 | 4285.94 | 1.04x | ✅ |
| sar | overlap_extra | 44.04 | 58.94 | 1.34x | ✅ |
| sin | math_transform | 24.93 | 26.03 | 1.04x | ✅ |
| sma_20 | overlap | 14.07 | 19.67 | 1.40x | ✅ |
| sma_20@10000 | scaled_10k | 14.43 | 19.70 | 1.37x | ✅ |
| sma_20@100000 | scaled_100k | 135.97 | 198.94 | 1.46x | ✅ |
| sma_20@1000000 | scaled_1m | 3322.32 | 3272.11 | 0.98x | ⚠️ |
| sqrt | math_transform | 8.25 | 15.00 | 1.82x | ✅ |
| stddev_20 | volatility | 29.41 | 38.53 | 1.31x | ✅ |
| stoch_14_3_3 | momentum | 84.22 | 98.37 | 1.17x | ✅ |
| stochf_14_3 | momentum | 76.57 | 80.60 | 1.05x | ✅ |
| stochrsi_14_14_3_3 | momentum | 100.45 | 109.84 | 1.09x | ✅ |
| sub | math_operators | 2.76 | 5.68 | 2.06x | ✅ |
| sum_30 | math_operators | 12.18 | 20.09 | 1.65x | ✅ |
| t3_5 | overlap_extra | 37.68 | 346.64 | 9.20x | ✅ |
| tanh | math_transform | 32.88 | 48.16 | 1.46x | ✅ |
| tema_20 | overlap | 26.86 | 122.93 | 4.58x | ✅ |
| trima_20 | overlap | 29.54 | 28.73 | 0.97x | ⚠️ |
| trix_15 | momentum | 42.17 | 127.75 | 3.03x | ✅ |
| tsf_14 | statistics_extra | 35.59 | 66.13 | 1.86x | ✅ |
| typprice | price_transform_full | 6.03 | 11.66 | 1.94x | ✅ |
| ultosc_7_14_28 | momentum | 69.63 | 57.80 | 0.83x | ⚠️ |
| var_20 | statistics | 28.71 | 25.62 | 0.89x | ⚠️ |
| wclprice | price_transform | 4.73 | 46.14 | 9.76x | ✅ |
| willr_14 | momentum | 31.99 | 34.19 | 1.07x | ✅ |
| wma_20 | overlap | 22.97 | 20.80 | 0.91x | ⚠️ |

- **Total paired benchmarks**: 90
- **Finkit faster or equal on this run**: 72

Do not convert one machine snapshot into a universal performance claim. Re-run the suite on the target deployment hardware.
