# Finkit vs TA-Lib C Benchmark Report

> Auto-generated from Criterion JSON by scripts/bench_report.py.
> Results are valid only for the recorded commit, CPU, compiler, build flags, dataset, and TA-Lib version.

| Indicator | Category | Finkit (µs) | TA-Lib C (µs) | Speedup | Status |
|---|---|---:|---:|---:|:---:|
| acos | math_transform | 45.44 | 136.35 | 3.00x | ✅ |
| ad | volume | 12.38 | 12.15 | 0.98x | ⚠️ |
| add | math_operators | 2.81 | 5.76 | 2.05x | ✅ |
| adosc_3_10 | volume | 97.51 | 99.61 | 1.02x | ✅ |
| adx_14 | directional | 75.42 | 75.43 | 1.00x | ✅ |
| adxr_14 | directional | 84.43 | 78.56 | 0.93x | ⚠️ |
| apo_12_26 | momentum_extra | 17.04 | 42.75 | 2.51x | ✅ |
| aroon_14 | directional | 59.19 | 49.86 | 0.84x | ⚠️ |
| aroonosc_14 | momentum | 54.06 | 46.20 | 0.85x | ⚠️ |
| atr_14 | volatility | 28.93 | 48.52 | 1.68x | ✅ |
| avgdev_14 | statistics_extra | 101.16 | 148.92 | 1.47x | ✅ |
| avgprice | price_transform_full | 6.48 | 7.86 | 1.21x | ✅ |
| bbands_20 | overlap | 32.05 | 50.78 | 1.58x | ✅ |
| bbands_20@10000 | scaled_10k | 29.23 | 47.43 | 1.62x | ✅ |
| bbands_20@100000 | scaled_100k | 721.74 | 1206.81 | 1.67x | ✅ |
| bbands_20@1000000 | scaled_1m | 10688.58 | 13073.44 | 1.22x | ✅ |
| bop | momentum_extra | 6.69 | 10.59 | 1.58x | ✅ |
| cci_14 | momentum | 186.40 | 218.81 | 1.17x | ✅ |
| ceil | math_transform | 1.64 | 28.32 | 17.29x | ✅ |
| cmo_14 | momentum | 37.84 | 51.61 | 1.36x | ✅ |
| correl_30 | statistics_extra | 61.88 | 77.65 | 1.25x | ✅ |
| cos | math_transform | 26.44 | 24.02 | 0.91x | ⚠️ |
| dema_20 | overlap | 29.57 | 87.63 | 2.96x | ✅ |
| ema_12 | overlap | 18.64 | 47.05 | 2.52x | ✅ |
| ema_12@10000 | scaled_10k | 17.79 | 45.20 | 2.54x | ✅ |
| ema_12@100000 | scaled_100k | 175.44 | 465.02 | 2.65x | ✅ |
| ema_12@1000000 | scaled_1m | 4168.34 | 6444.88 | 1.55x | ✅ |
| exp | math_transform | 30.80 | 42.64 | 1.38x | ✅ |
| floor | math_transform | 1.66 | 23.94 | 14.41x | ✅ |
| ht_dcperiod | cycle_extra | 461.21 | 594.56 | 1.29x | ✅ |
| ht_dcphase | cycle_extra | 887.14 | 3235.94 | 3.65x | ✅ |
| ht_phasor | cycle | 475.40 | 569.75 | 1.20x | ✅ |
| ht_sine | cycle | 1197.87 | 3413.19 | 2.85x | ✅ |
| ht_trendline | cycle_extra | 477.99 | 893.00 | 1.87x | ✅ |
| kama_30 | overlap | 25.37 | 101.17 | 3.99x | ✅ |
| linearreg_14 | statistics | 35.59 | 66.86 | 1.88x | ✅ |
| linreg_angle_14 | statistics_extra | 95.12 | 90.26 | 0.95x | ⚠️ |
| linreg_intercept_14 | statistics_extra | 31.42 | 25.87 | 0.82x | ⚠️ |
| linreg_slope_14 | statistics | 32.11 | 24.48 | 0.76x | ❌ |
| ln | math_transform | 41.78 | 45.62 | 1.09x | ✅ |
| ma_20 | overlap_extra | 11.50 | 19.62 | 1.71x | ✅ |
| macd@10000 | scaled_10k | 58.52 | 139.96 | 2.39x | ✅ |
| macd@100000 | scaled_100k | 995.34 | 1990.11 | 2.00x | ✅ |
| macd@1000000 | scaled_1m | 12748.07 | 18416.16 | 1.44x | ✅ |
| macd_12_26_9 | momentum | 60.61 | 138.96 | 2.29x | ✅ |
| mama | momentum_extra | 526.87 | 899.78 | 1.71x | ✅ |
| max_30 | math_operators | 16.08 | 15.18 | 0.94x | ⚠️ |
| medprice | price_transform_full | 4.20 | 5.80 | 1.38x | ✅ |
| mfi_14 | momentum | 43.68 | 64.59 | 1.48x | ✅ |
| min_30 | math_operators | 15.71 | 14.83 | 0.94x | ⚠️ |
| minus_di_14 | directional | 57.75 | 58.54 | 1.01x | ✅ |
| minus_dm_14 | momentum | 8.66 | 46.64 | 5.39x | ✅ |
| mom_10 | momentum | 3.42 | 5.33 | 1.56x | ✅ |
| mult | math_operators | 2.87 | 5.87 | 2.05x | ✅ |
| natr_14 | volatility | 31.87 | 52.60 | 1.65x | ✅ |
| obv | volume | 9.71 | 11.26 | 1.16x | ✅ |
| percentrank_30 | statistics_extra | 70.90 | 329.06 | 4.64x | ✅ |
| plus_di_14 | directional | 57.91 | 58.98 | 1.02x | ✅ |
| plus_dm_14 | momentum | 8.77 | 47.36 | 5.40x | ✅ |
| ppo_12_26 | momentum_extra | 19.81 | 47.71 | 2.41x | ✅ |
| roc_10 | momentum | 6.11 | 10.71 | 1.75x | ✅ |
| rsi_14 | momentum | 24.08 | 31.38 | 1.30x | ✅ |
| rsi_14@10000 | scaled_10k | 23.08 | 30.73 | 1.33x | ✅ |
| rsi_14@100000 | scaled_100k | 230.24 | 303.56 | 1.32x | ✅ |
| rsi_14@1000000 | scaled_1m | 5068.10 | 4935.18 | 0.97x | ⚠️ |
| sar | overlap_extra | 43.04 | 60.40 | 1.40x | ✅ |
| sin | math_transform | 24.06 | 26.30 | 1.09x | ✅ |
| sma_20 | overlap | 11.78 | 19.91 | 1.69x | ✅ |
| sma_20@10000 | scaled_10k | 11.42 | 19.03 | 1.67x | ✅ |
| sma_20@100000 | scaled_100k | 111.19 | 194.12 | 1.75x | ✅ |
| sma_20@1000000 | scaled_1m | 5098.03 | 4322.07 | 0.85x | ⚠️ |
| sqrt | math_transform | 8.38 | 15.17 | 1.81x | ✅ |
| stddev_20 | volatility | 29.10 | 38.81 | 1.33x | ✅ |
| stoch_14_3_3 | momentum | 84.03 | 98.86 | 1.18x | ✅ |
| stochf_14_3 | momentum | 78.12 | 81.29 | 1.04x | ✅ |
| stochrsi_14_14_3_3 | momentum | 102.72 | 110.19 | 1.07x | ✅ |
| sub | math_operators | 2.86 | 5.80 | 2.02x | ✅ |
| sum_30 | math_operators | 12.05 | 19.57 | 1.62x | ✅ |
| t3_5 | overlap_extra | 38.59 | 350.91 | 9.09x | ✅ |
| tanh | math_transform | 28.00 | 46.96 | 1.68x | ✅ |
| tema_20 | overlap | 27.49 | 124.07 | 4.51x | ✅ |
| trima_20 | overlap | 29.52 | 29.01 | 0.98x | ⚠️ |
| trix_15 | momentum | 43.43 | 128.81 | 2.97x | ✅ |
| tsf_14 | statistics_extra | 36.38 | 69.51 | 1.91x | ✅ |
| typprice | price_transform_full | 6.36 | 10.66 | 1.68x | ✅ |
| ultosc_7_14_28 | momentum | 70.29 | 58.14 | 0.83x | ⚠️ |
| var_20 | statistics | 30.64 | 26.69 | 0.87x | ⚠️ |
| wclprice | price_transform | 4.99 | 47.29 | 9.48x | ✅ |
| willr_14 | momentum | 32.11 | 34.36 | 1.07x | ✅ |
| wma_20 | overlap | 21.98 | 21.16 | 0.96x | ⚠️ |

- **Total paired benchmarks**: 90
- **Finkit faster or equal on this run**: 74

Do not convert one machine snapshot into a universal performance claim. Re-run the suite on the target deployment hardware.
