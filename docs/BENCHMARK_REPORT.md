# Finkit vs TA-Lib C Benchmark Report

> Auto-generated from Criterion JSON by scripts/bench_report.py.
> Results are valid only for the recorded commit, CPU, compiler, build flags, dataset, and TA-Lib version.

| Indicator | Category | Finkit (µs) | TA-Lib C (µs) | Speedup | Status |
|---|---|---:|---:|---:|:---:|
| acos | math_transform | 133.42 | 131.95 | 0.99x | ⚠️ |
| ad | volume | 12.28 | 12.14 | 0.99x | ⚠️ |
| add | math_operators | 3.28 | 6.53 | 1.99x | ✅ |
| adosc_3_10 | volume | 94.01 | 97.82 | 1.04x | ✅ |
| adx_14 | directional | 77.81 | 73.09 | 0.94x | ⚠️ |
| adxr_14 | directional | 86.01 | 77.92 | 0.91x | ⚠️ |
| apo_12_26 | momentum_extra | 16.33 | 42.43 | 2.60x | ✅ |
| aroon_14 | directional | 51.98 | 49.66 | 0.96x | ⚠️ |
| aroonosc_14 | momentum | 45.53 | 43.03 | 0.95x | ⚠️ |
| atr_14 | volatility | 28.28 | 48.20 | 1.70x | ✅ |
| avgdev_14 | statistics_extra | 97.27 | 143.43 | 1.47x | ✅ |
| avgprice | price_transform_full | 6.84 | 8.10 | 1.18x | ✅ |
| bbands_20 | overlap | 39.11 | 60.28 | 1.54x | ✅ |
| bbands_20@10000 | scaled_10k | 36.39 | 54.39 | 1.49x | ✅ |
| bbands_20@100000 | scaled_100k | 960.73 | 1390.47 | 1.45x | ✅ |
| bbands_20@1000000 | scaled_1m | 8535.27 | 10671.57 | 1.25x | ✅ |
| bop | momentum_extra | 6.80 | 10.40 | 1.53x | ✅ |
| cci_14 | momentum | 182.96 | 213.61 | 1.17x | ✅ |
| ceil | math_transform | 1.50 | 23.17 | 15.50x | ✅ |
| cmo_14 | momentum | 37.46 | 51.42 | 1.37x | ✅ |
| correl_30 | statistics_extra | 60.00 | 72.72 | 1.21x | ✅ |
| cos | math_transform | 20.75 | 25.62 | 1.23x | ✅ |
| dema_20 | overlap | 31.12 | 86.76 | 2.79x | ✅ |
| ema_12 | overlap | 17.84 | 48.67 | 2.73x | ✅ |
| ema_12@10000 | scaled_10k | 18.20 | 46.27 | 2.54x | ✅ |
| ema_12@100000 | scaled_100k | 179.18 | 470.50 | 2.63x | ✅ |
| ema_12@1000000 | scaled_1m | 3278.31 | 5898.26 | 1.80x | ✅ |
| exp | math_transform | 39.95 | 41.72 | 1.04x | ✅ |
| floor | math_transform | 1.50 | 25.40 | 16.91x | ✅ |
| ht_dcperiod | cycle_extra | 450.79 | 578.15 | 1.28x | ✅ |
| ht_dcphase | cycle_extra | 882.20 | 3130.90 | 3.55x | ✅ |
| ht_phasor | cycle | 457.61 | 549.49 | 1.20x | ✅ |
| ht_sine | cycle | 1173.19 | 3341.38 | 2.85x | ✅ |
| ht_trendline | cycle_extra | 462.31 | 859.46 | 1.86x | ✅ |
| kama_30 | overlap | 27.29 | 100.21 | 3.67x | ✅ |
| linearreg_14 | statistics | 26.22 | 64.52 | 2.46x | ✅ |
| linreg_angle_14 | statistics_extra | 92.57 | 88.16 | 0.95x | ⚠️ |
| linreg_intercept_14 | statistics_extra | 21.74 | 25.17 | 1.16x | ✅ |
| linreg_slope_14 | statistics | 21.87 | 23.18 | 1.06x | ✅ |
| ln | math_transform | 45.04 | 44.44 | 0.99x | ⚠️ |
| ma_20 | overlap_extra | 11.18 | 18.99 | 1.70x | ✅ |
| macd@10000 | scaled_10k | 64.01 | 147.53 | 2.30x | ✅ |
| macd@100000 | scaled_100k | 1186.43 | 2181.10 | 1.84x | ✅ |
| macd@1000000 | scaled_1m | 10556.10 | 17089.46 | 1.62x | ✅ |
| macd_12_26_9 | momentum | 63.83 | 140.99 | 2.21x | ✅ |
| mama | momentum_extra | 509.64 | 877.00 | 1.72x | ✅ |
| max_30 | math_operators | 14.89 | 14.57 | 0.98x | ⚠️ |
| medprice | price_transform_full | 3.86 | 5.51 | 1.43x | ✅ |
| mfi_14 | momentum | 32.02 | 61.68 | 1.93x | ✅ |
| min_30 | math_operators | 14.86 | 14.62 | 0.98x | ⚠️ |
| minus_di_14 | directional | 56.57 | 56.75 | 1.00x | ✅ |
| minus_dm_14 | momentum | 8.79 | 45.27 | 5.15x | ✅ |
| mom_10 | momentum | 3.29 | 5.14 | 1.56x | ✅ |
| mult | math_operators | 2.98 | 5.79 | 1.94x | ✅ |
| natr_14 | volatility | 31.14 | 51.24 | 1.65x | ✅ |
| obv | volume | 8.97 | 10.29 | 1.15x | ✅ |
| percentrank_30 | statistics_extra | 71.92 | 314.96 | 4.38x | ✅ |
| plus_di_14 | directional | 56.05 | 57.37 | 1.02x | ✅ |
| plus_dm_14 | momentum | 7.96 | 45.42 | 5.70x | ✅ |
| ppo_12_26 | momentum_extra | 19.09 | 47.61 | 2.49x | ✅ |
| roc_10 | momentum | 5.98 | 10.44 | 1.75x | ✅ |
| rsi_14 | momentum | 22.42 | 30.10 | 1.34x | ✅ |
| rsi_14@10000 | scaled_10k | 22.53 | 30.83 | 1.37x | ✅ |
| rsi_14@100000 | scaled_100k | 229.05 | 317.19 | 1.38x | ✅ |
| rsi_14@1000000 | scaled_1m | 4023.76 | 4209.08 | 1.05x | ✅ |
| sar | overlap_extra | 43.54 | 60.33 | 1.39x | ✅ |
| sin | math_transform | 20.75 | 23.20 | 1.12x | ✅ |
| sma_20 | overlap | 10.98 | 19.07 | 1.74x | ✅ |
| sma_20@10000 | scaled_10k | 11.44 | 19.44 | 1.70x | ✅ |
| sma_20@100000 | scaled_100k | 112.40 | 199.65 | 1.78x | ✅ |
| sma_20@1000000 | scaled_1m | 3135.27 | 3297.12 | 1.05x | ✅ |
| sqrt | math_transform | 8.19 | 14.57 | 1.78x | ✅ |
| stddev_20 | volatility | 28.30 | 37.46 | 1.32x | ✅ |
| stoch_14_3_3 | momentum | 80.08 | 101.66 | 1.27x | ✅ |
| stochf_14_3 | momentum | 67.40 | 84.27 | 1.25x | ✅ |
| stochrsi_14_14_3_3 | momentum | 91.30 | 118.87 | 1.30x | ✅ |
| sub | math_operators | 3.31 | 6.54 | 1.98x | ✅ |
| sum_30 | math_operators | 11.24 | 18.83 | 1.68x | ✅ |
| t3_5 | overlap_extra | 37.65 | 339.81 | 9.03x | ✅ |
| tanh | math_transform | 42.48 | 45.24 | 1.06x | ✅ |
| tema_20 | overlap | 29.48 | 123.94 | 4.20x | ✅ |
| trima_20 | overlap | 31.60 | 31.37 | 0.99x | ⚠️ |
| trix_15 | momentum | 42.35 | 125.29 | 2.96x | ✅ |
| tsf_14 | statistics_extra | 35.58 | 64.98 | 1.83x | ✅ |
| typprice | price_transform_full | 5.96 | 10.32 | 1.73x | ✅ |
| ultosc_7_14_28 | momentum | 67.64 | 56.54 | 0.84x | ⚠️ |
| var_20 | statistics | 27.71 | 24.66 | 0.89x | ⚠️ |
| wclprice | price_transform | 4.91 | 46.09 | 9.38x | ✅ |
| willr_14 | momentum | 31.36 | 33.37 | 1.06x | ✅ |
| wma_20 | overlap | 23.48 | 23.29 | 0.99x | ⚠️ |

- **Total paired benchmarks**: 90
- **Finkit faster or equal on this run**: 76

Do not convert one machine snapshot into a universal performance claim. Re-run the suite on the target deployment hardware.
