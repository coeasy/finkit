# Finkit vs TA-Lib C Benchmark Report

> Auto-generated from Criterion JSON by scripts/bench_report.py.
> Results are valid only for the recorded commit, CPU, compiler, build flags, dataset, and TA-Lib version.

| Indicator | Category | Finkit (µs) | TA-Lib C (µs) | Speedup | Status |
|---|---|---:|---:|---:|:---:|
| acos | math_transform | 138.41 | 134.98 | 0.98x | ⚠️ |
| ad | volume | 12.07 | 11.89 | 0.98x | ⚠️ |
| add | math_operators | 3.00 | 5.63 | 1.88x | ✅ |
| adosc_3_10 | volume | 92.86 | 98.54 | 1.06x | ✅ |
| adx_14 | directional | 77.93 | 73.29 | 0.94x | ⚠️ |
| adxr_14 | directional | 84.92 | 74.50 | 0.88x | ⚠️ |
| apo_12_26 | momentum_extra | 16.21 | 41.16 | 2.54x | ✅ |
| aroon_14 | directional | 56.83 | 47.22 | 0.83x | ⚠️ |
| aroonosc_14 | momentum | 51.73 | 41.83 | 0.81x | ⚠️ |
| atr_14 | volatility | 28.29 | 47.02 | 1.66x | ✅ |
| avgdev_14 | statistics_extra | 96.48 | 145.00 | 1.50x | ✅ |
| avgprice | price_transform_full | 6.53 | 7.80 | 1.19x | ✅ |
| bbands_20 | overlap | 31.55 | 48.84 | 1.55x | ✅ |
| bbands_20@10000 | scaled_10k | 30.14 | 47.43 | 1.57x | ✅ |
| bbands_20@100000 | scaled_100k | 704.32 | 1096.94 | 1.56x | ✅ |
| bbands_20@1000000 | scaled_1m | 7876.82 | 10009.82 | 1.27x | ✅ |
| bop | momentum_extra | 6.54 | 10.33 | 1.58x | ✅ |
| cci_14 | momentum | 178.62 | 211.42 | 1.18x | ✅ |
| ceil | math_transform | 1.94 | 23.86 | 12.28x | ✅ |
| cmo_14 | momentum | 35.79 | 50.87 | 1.42x | ✅ |
| correl_30 | statistics_extra | 60.01 | 72.81 | 1.21x | ✅ |
| cos | math_transform | 24.10 | 23.33 | 0.97x | ⚠️ |
| dema_20 | overlap | 27.89 | 83.31 | 2.99x | ✅ |
| ema_12 | overlap | 17.85 | 45.45 | 2.55x | ✅ |
| ema_12@10000 | scaled_10k | 18.28 | 48.42 | 2.65x | ✅ |
| ema_12@100000 | scaled_100k | 176.27 | 462.08 | 2.62x | ✅ |
| ema_12@1000000 | scaled_1m | 3156.80 | 5796.26 | 1.84x | ✅ |
| exp | math_transform | 40.13 | 41.40 | 1.03x | ✅ |
| floor | math_transform | 1.99 | 24.02 | 12.05x | ✅ |
| ht_dcperiod | cycle_extra | 448.43 | 574.36 | 1.28x | ✅ |
| ht_dcphase | cycle_extra | 876.82 | 3121.22 | 3.56x | ✅ |
| ht_phasor | cycle | 462.25 | 552.14 | 1.19x | ✅ |
| ht_sine | cycle | 1191.42 | 3449.98 | 2.90x | ✅ |
| ht_trendline | cycle_extra | 457.14 | 873.53 | 1.91x | ✅ |
| kama_30 | overlap | 24.41 | 97.04 | 3.98x | ✅ |
| linearreg_14 | statistics | 26.89 | 65.39 | 2.43x | ✅ |
| linreg_angle_14 | statistics_extra | 93.33 | 88.51 | 0.95x | ⚠️ |
| linreg_intercept_14 | statistics_extra | 21.82 | 26.68 | 1.22x | ✅ |
| linreg_slope_14 | statistics | 22.38 | 22.69 | 1.01x | ✅ |
| ln | math_transform | 45.04 | 44.42 | 0.99x | ⚠️ |
| ma_20 | overlap_extra | 10.79 | 18.94 | 1.76x | ✅ |
| macd@10000 | scaled_10k | 59.32 | 141.76 | 2.39x | ✅ |
| macd@100000 | scaled_100k | 946.73 | 2178.31 | 2.30x | ✅ |
| macd@1000000 | scaled_1m | 9931.76 | 17164.19 | 1.73x | ✅ |
| macd_12_26_9 | momentum | 58.06 | 134.75 | 2.32x | ✅ |
| mama | momentum_extra | 505.63 | 864.99 | 1.71x | ✅ |
| max_30 | math_operators | 16.07 | 14.34 | 0.89x | ⚠️ |
| medprice | price_transform_full | 3.84 | 5.63 | 1.47x | ✅ |
| mfi_14 | momentum | 37.10 | 57.25 | 1.54x | ✅ |
| min_30 | math_operators | 15.08 | 14.71 | 0.98x | ⚠️ |
| minus_di_14 | directional | 55.65 | 56.74 | 1.02x | ✅ |
| minus_dm_14 | momentum | 8.39 | 45.68 | 5.44x | ✅ |
| mom_10 | momentum | 3.35 | 5.09 | 1.52x | ✅ |
| mult | math_operators | 3.02 | 5.63 | 1.86x | ✅ |
| natr_14 | volatility | 30.88 | 50.85 | 1.65x | ✅ |
| obv | volume | 8.80 | 10.16 | 1.16x | ✅ |
| percentrank_30 | statistics_extra | 67.06 | 337.33 | 5.03x | ✅ |
| plus_di_14 | directional | 55.22 | 56.76 | 1.03x | ✅ |
| plus_dm_14 | momentum | 8.79 | 45.76 | 5.21x | ✅ |
| ppo_12_26 | momentum_extra | 19.77 | 45.46 | 2.30x | ✅ |
| roc_10 | momentum | 5.85 | 10.28 | 1.76x | ✅ |
| rsi_14 | momentum | 21.89 | 30.12 | 1.38x | ✅ |
| rsi_14@10000 | scaled_10k | 22.34 | 31.44 | 1.41x | ✅ |
| rsi_14@100000 | scaled_100k | 219.88 | 306.60 | 1.39x | ✅ |
| rsi_14@1000000 | scaled_1m | 3785.81 | 4090.70 | 1.08x | ✅ |
| sar | overlap_extra | 41.69 | 58.00 | 1.39x | ✅ |
| sin | math_transform | 24.04 | 23.99 | 1.00x | ⚠️ |
| sma_20 | overlap | 11.13 | 19.13 | 1.72x | ✅ |
| sma_20@10000 | scaled_10k | 10.81 | 19.61 | 1.81x | ✅ |
| sma_20@100000 | scaled_100k | 110.85 | 195.72 | 1.77x | ✅ |
| sma_20@1000000 | scaled_1m | 2831.75 | 3188.18 | 1.13x | ✅ |
| sqrt | math_transform | 8.27 | 14.55 | 1.76x | ✅ |
| stddev_20 | volatility | 27.88 | 37.05 | 1.33x | ✅ |
| stoch_14_3_3 | momentum | 80.03 | 94.02 | 1.17x | ✅ |
| stochf_14_3 | momentum | 72.99 | 76.41 | 1.05x | ✅ |
| stochrsi_14_14_3_3 | momentum | 95.88 | 104.62 | 1.09x | ✅ |
| sub | math_operators | 3.05 | 5.62 | 1.84x | ✅ |
| sum_30 | math_operators | 11.33 | 19.09 | 1.69x | ✅ |
| t3_5 | overlap_extra | 37.51 | 341.97 | 9.12x | ✅ |
| tanh | math_transform | 43.58 | 45.58 | 1.05x | ✅ |
| tema_20 | overlap | 26.28 | 121.04 | 4.61x | ✅ |
| trima_20 | overlap | 28.33 | 27.94 | 0.99x | ⚠️ |
| trix_15 | momentum | 40.95 | 123.76 | 3.02x | ✅ |
| tsf_14 | statistics_extra | 35.28 | 67.21 | 1.90x | ✅ |
| typprice | price_transform_full | 6.79 | 10.60 | 1.56x | ✅ |
| ultosc_7_14_28 | momentum | 68.77 | 55.91 | 0.81x | ⚠️ |
| var_20 | statistics | 26.88 | 24.80 | 0.92x | ⚠️ |
| wclprice | price_transform | 4.79 | 46.72 | 9.75x | ✅ |
| willr_14 | momentum | 30.41 | 33.04 | 1.09x | ✅ |
| wma_20 | overlap | 20.61 | 20.42 | 0.99x | ⚠️ |

- **Total paired benchmarks**: 90
- **Finkit faster or equal on this run**: 74

Do not convert one machine snapshot into a universal performance claim. Re-run the suite on the target deployment hardware.
