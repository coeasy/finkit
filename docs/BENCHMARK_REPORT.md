# Finkit vs TA-Lib C Benchmark Report

> Auto-generated from Criterion JSON by scripts/bench_report.py.
> Results are valid only for the recorded commit, CPU, compiler, build flags, dataset, and TA-Lib version.

| Indicator | Category | Finkit (µs) | TA-Lib C (µs) | Speedup | Status |
|---|---|---:|---:|---:|:---:|
| acos | math_transform | 139.00 | 135.40 | 0.97x | ⚠️ |
| ad | volume | 12.69 | 12.85 | 1.01x | ✅ |
| add | math_operators | 3.14 | 6.00 | 1.91x | ✅ |
| adosc_3_10 | volume | 94.88 | 104.39 | 1.10x | ✅ |
| adx_14 | directional | 81.23 | 74.59 | 0.92x | ⚠️ |
| adxr_14 | directional | 84.57 | 77.89 | 0.92x | ⚠️ |
| apo_12_26 | momentum_extra | 18.11 | 42.31 | 2.34x | ✅ |
| aroon_14 | directional | 57.18 | 50.34 | 0.88x | ⚠️ |
| aroonosc_14 | momentum | 61.85 | 44.68 | 0.72x | ❌ |
| atr_14 | volatility | 29.55 | 49.15 | 1.66x | ✅ |
| avgdev_14 | statistics_extra | 98.19 | 144.84 | 1.48x | ✅ |
| avgprice | price_transform_full | 7.67 | 8.07 | 1.05x | ✅ |
| bbands_20 | overlap | 34.96 | 47.09 | 1.35x | ✅ |
| bbands_20@10000 | scaled_10k | 35.05 | 47.01 | 1.34x | ✅ |
| bbands_20@100000 | scaled_100k | 381.21 | 494.21 | 1.30x | ✅ |
| bbands_20@1000000 | scaled_1m | 22065.74 | 13318.03 | 0.60x | ❌ |
| bop | momentum_extra | 6.83 | 10.79 | 1.58x | ✅ |
| cci_14 | momentum | 185.45 | 218.83 | 1.18x | ✅ |
| ceil | math_transform | 27.42 | 25.87 | 0.94x | ⚠️ |
| cmo_14 | momentum | 37.27 | 51.61 | 1.38x | ✅ |
| correl_30 | statistics_extra | 60.46 | 73.77 | 1.22x | ✅ |
| cos | math_transform | 27.25 | 24.97 | 0.92x | ⚠️ |
| dema_20 | overlap | 30.42 | 88.06 | 2.90x | ✅ |
| ema_12 | overlap | 19.45 | 46.69 | 2.40x | ✅ |
| ema_12@10000 | scaled_10k | 18.14 | 45.97 | 2.53x | ✅ |
| ema_12@100000 | scaled_100k | 178.24 | 472.68 | 2.65x | ✅ |
| ema_12@1000000 | scaled_1m | 3795.93 | 6337.70 | 1.67x | ✅ |
| exp | math_transform | 43.55 | 43.05 | 0.99x | ⚠️ |
| floor | math_transform | 28.13 | 27.53 | 0.98x | ⚠️ |
| ht_dcperiod | cycle_extra | 458.62 | 590.05 | 1.29x | ✅ |
| ht_dcphase | cycle_extra | 1144.00 | 3186.61 | 2.79x | ✅ |
| ht_phasor | cycle | 475.40 | 554.95 | 1.17x | ✅ |
| ht_sine | cycle | 1319.79 | 3406.70 | 2.58x | ✅ |
| ht_trendline | cycle_extra | 476.97 | 934.01 | 1.96x | ✅ |
| kama_30 | overlap | 25.92 | 102.16 | 3.94x | ✅ |
| linearreg_14 | statistics | 36.59 | 66.60 | 1.82x | ✅ |
| linreg_angle_14 | statistics_extra | 95.32 | 89.26 | 0.94x | ⚠️ |
| linreg_intercept_14 | statistics_extra | 34.38 | 26.33 | 0.77x | ❌ |
| linreg_slope_14 | statistics | 35.36 | 23.83 | 0.67x | ❌ |
| ln | math_transform | 63.71 | 48.72 | 0.76x | ❌ |
| ma_20 | overlap_extra | 16.40 | 19.50 | 1.19x | ✅ |
| macd@10000 | scaled_10k | 132.17 | 138.21 | 1.05x | ✅ |
| macd@100000 | scaled_100k | 1364.13 | 1364.47 | 1.00x | ✅ |
| macd@1000000 | scaled_1m | 18593.07 | 18079.51 | 0.97x | ⚠️ |
| macd_12_26_9 | momentum | 138.37 | 139.89 | 1.01x | ✅ |
| mama | momentum_extra | 523.46 | 882.94 | 1.69x | ✅ |
| max_30 | math_operators | 21.85 | 14.66 | 0.67x | ❌ |
| medprice | price_transform_full | 4.61 | 6.21 | 1.35x | ✅ |
| mfi_14 | momentum | 43.59 | 61.59 | 1.41x | ✅ |
| min_30 | math_operators | 31.48 | 14.75 | 0.47x | ❌ |
| minus_di_14 | directional | 63.79 | 59.05 | 0.93x | ⚠️ |
| minus_dm_14 | momentum | 10.30 | 48.04 | 4.66x | ✅ |
| mom_10 | momentum | 3.37 | 5.46 | 1.62x | ✅ |
| mult | math_operators | 3.20 | 5.94 | 1.85x | ✅ |
| natr_14 | volatility | 31.91 | 52.15 | 1.63x | ✅ |
| obv | volume | 11.27 | 12.54 | 1.11x | ✅ |
| percentrank_30 | statistics_extra | 465.20 | 362.69 | 0.78x | ❌ |
| plus_di_14 | directional | 58.77 | 66.14 | 1.13x | ✅ |
| plus_dm_14 | momentum | 9.91 | 47.98 | 4.84x | ✅ |
| ppo_12_26 | momentum_extra | 53.39 | 46.47 | 0.87x | ⚠️ |
| roc_10 | momentum | 6.23 | 10.85 | 1.74x | ✅ |
| rsi_14 | momentum | 25.91 | 31.19 | 1.20x | ✅ |
| rsi_14@10000 | scaled_10k | 25.05 | 31.02 | 1.24x | ✅ |
| rsi_14@100000 | scaled_100k | 261.18 | 306.71 | 1.17x | ✅ |
| rsi_14@1000000 | scaled_1m | 6081.47 | 4339.54 | 0.71x | ❌ |
| sar | overlap_extra | 69.34 | 60.53 | 0.87x | ⚠️ |
| sin | math_transform | 23.60 | 24.83 | 1.05x | ✅ |
| sma_20 | overlap | 18.38 | 26.37 | 1.43x | ✅ |
| sma_20@10000 | scaled_10k | 16.29 | 19.26 | 1.18x | ✅ |
| sma_20@100000 | scaled_100k | 171.53 | 198.46 | 1.16x | ✅ |
| sma_20@1000000 | scaled_1m | 4227.49 | 3171.37 | 0.75x | ❌ |
| sqrt | math_transform | 8.96 | 16.60 | 1.85x | ✅ |
| stddev_20 | volatility | 29.30 | 43.88 | 1.50x | ✅ |
| stoch_14_3_3 | momentum | 85.48 | 96.64 | 1.13x | ✅ |
| stochf_14_3 | momentum | 137.15 | 78.50 | 0.57x | ❌ |
| stochrsi_14_14_3_3 | momentum | 243.01 | 109.06 | 0.45x | ❌ |
| sub | math_operators | 3.39 | 5.90 | 1.74x | ✅ |
| sum_30 | math_operators | 11.95 | 19.25 | 1.61x | ✅ |
| t3_5 | overlap_extra | 38.21 | 346.87 | 9.08x | ✅ |
| tanh | math_transform | 43.51 | 45.27 | 1.04x | ✅ |
| tema_20 | overlap | 28.04 | 125.75 | 4.49x | ✅ |
| trima_20 | overlap | 35.94 | 29.02 | 0.81x | ⚠️ |
| trix_15 | momentum | 42.55 | 128.32 | 3.02x | ✅ |
| tsf_14 | statistics_extra | 35.76 | 66.08 | 1.85x | ✅ |
| typprice | price_transform_full | 6.74 | 10.67 | 1.58x | ✅ |
| ultosc_7_14_28 | momentum | 88.79 | 58.05 | 0.65x | ❌ |
| var_20 | statistics | 28.51 | 25.52 | 0.90x | ⚠️ |
| wclprice | price_transform | 6.40 | 47.40 | 7.40x | ✅ |
| willr_14 | momentum | 32.56 | 34.52 | 1.06x | ✅ |
| wma_20 | overlap | 24.69 | 20.66 | 0.84x | ⚠️ |

- **Total paired benchmarks**: 90
- **Finkit faster or equal on this run**: 61

Do not convert one machine snapshot into a universal performance claim. Re-run the suite on the target deployment hardware.
