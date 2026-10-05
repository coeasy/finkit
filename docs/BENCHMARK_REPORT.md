# Finkit vs TA-Lib C Benchmark Report

> Auto-generated from Criterion JSON by scripts/bench_report.py.
> Results are valid only for the recorded commit, CPU, compiler, build flags, dataset, and TA-Lib version.

| Indicator | Category | Finkit (µs) | TA-Lib C (µs) | Speedup | Status |
|---|---|---:|---:|---:|:---:|
| acos | math_transform | 41.38 | 131.61 | 3.18x | ✅ |
| ad | volume | 12.04 | 11.84 | 0.98x | ⚠️ |
| add | math_operators | 2.74 | 5.73 | 2.09x | ✅ |
| adosc_3_10 | volume | 91.32 | 96.98 | 1.06x | ✅ |
| adx_14 | directional | 85.42 | 73.85 | 0.86x | ⚠️ |
| adxr_14 | directional | 84.51 | 78.23 | 0.93x | ⚠️ |
| apo_12_26 | momentum_extra | 15.84 | 42.16 | 2.66x | ✅ |
| aroon_14 | directional | 56.01 | 49.69 | 0.89x | ⚠️ |
| aroonosc_14 | momentum | 53.49 | 44.52 | 0.83x | ⚠️ |
| atr_14 | volatility | 28.27 | 47.24 | 1.67x | ✅ |
| avgdev_14 | statistics_extra | 96.46 | 144.15 | 1.49x | ✅ |
| avgprice | price_transform_full | 6.45 | 7.70 | 1.19x | ✅ |
| bbands_20 | overlap | 44.39 | 57.23 | 1.29x | ✅ |
| bbands_20@10000 | scaled_10k | 37.05 | 50.64 | 1.37x | ✅ |
| bbands_20@100000 | scaled_100k | 817.87 | 1009.91 | 1.23x | ✅ |
| bbands_20@1000000 | scaled_1m | 10528.13 | 10364.38 | 0.98x | ⚠️ |
| bop | momentum_extra | 6.50 | 10.36 | 1.59x | ✅ |
| cci_14 | momentum | 182.53 | 214.04 | 1.17x | ✅ |
| ceil | math_transform | 20.62 | 27.37 | 1.33x | ✅ |
| cmo_14 | momentum | 36.93 | 50.51 | 1.37x | ✅ |
| correl_30 | statistics_extra | 59.65 | 72.71 | 1.22x | ✅ |
| cos | math_transform | 20.39 | 23.22 | 1.14x | ✅ |
| dema_20 | overlap | 29.63 | 83.49 | 2.82x | ✅ |
| ema_12 | overlap | 17.86 | 45.68 | 2.56x | ✅ |
| ema_12@10000 | scaled_10k | 17.89 | 45.94 | 2.57x | ✅ |
| ema_12@100000 | scaled_100k | 181.20 | 471.07 | 2.60x | ✅ |
| ema_12@1000000 | scaled_1m | 2924.98 | 5586.62 | 1.91x | ✅ |
| exp | math_transform | 23.88 | 41.30 | 1.73x | ✅ |
| floor | math_transform | 20.50 | 23.15 | 1.13x | ✅ |
| ht_dcperiod | cycle_extra | 450.33 | 572.39 | 1.27x | ✅ |
| ht_dcphase | cycle_extra | 882.19 | 3127.92 | 3.55x | ✅ |
| ht_phasor | cycle | 458.09 | 544.67 | 1.19x | ✅ |
| ht_sine | cycle | 1166.90 | 3319.13 | 2.84x | ✅ |
| ht_trendline | cycle_extra | 460.83 | 856.06 | 1.86x | ✅ |
| kama_30 | overlap | 24.69 | 98.39 | 3.99x | ✅ |
| linearreg_14 | statistics | 34.63 | 64.62 | 1.87x | ✅ |
| linreg_angle_14 | statistics_extra | 92.73 | 86.92 | 0.94x | ⚠️ |
| linreg_intercept_14 | statistics_extra | 30.32 | 25.11 | 0.83x | ⚠️ |
| linreg_slope_14 | statistics | 30.91 | 23.14 | 0.75x | ❌ |
| ln | math_transform | 47.19 | 44.51 | 0.94x | ⚠️ |
| ma_20 | overlap_extra | 13.43 | 18.98 | 1.41x | ✅ |
| macd@10000 | scaled_10k | 62.68 | 143.07 | 2.28x | ✅ |
| macd@100000 | scaled_100k | 968.29 | 1879.92 | 1.94x | ✅ |
| macd@1000000 | scaled_1m | 10446.37 | 17090.94 | 1.64x | ✅ |
| macd_12_26_9 | momentum | 62.96 | 142.79 | 2.27x | ✅ |
| mama | momentum_extra | 510.94 | 871.63 | 1.71x | ✅ |
| max_30 | math_operators | 15.43 | 14.63 | 0.95x | ⚠️ |
| medprice | price_transform_full | 3.95 | 5.46 | 1.38x | ✅ |
| mfi_14 | momentum | 32.62 | 62.09 | 1.90x | ✅ |
| min_30 | math_operators | 15.52 | 14.31 | 0.92x | ⚠️ |
| minus_di_14 | directional | 56.20 | 56.63 | 1.01x | ✅ |
| minus_dm_14 | momentum | 8.40 | 46.09 | 5.49x | ✅ |
| mom_10 | momentum | 3.48 | 5.23 | 1.50x | ✅ |
| mult | math_operators | 2.86 | 5.63 | 1.97x | ✅ |
| natr_14 | volatility | 31.27 | 51.04 | 1.63x | ✅ |
| obv | volume | 9.06 | 10.32 | 1.14x | ✅ |
| percentrank_30 | statistics_extra | 295.43 | 318.09 | 1.08x | ✅ |
| plus_di_14 | directional | 55.66 | 57.03 | 1.02x | ✅ |
| plus_dm_14 | momentum | 8.40 | 45.90 | 5.46x | ✅ |
| ppo_12_26 | momentum_extra | 19.08 | 46.69 | 2.45x | ✅ |
| roc_10 | momentum | 5.95 | 10.44 | 1.75x | ✅ |
| rsi_14 | momentum | 22.28 | 30.00 | 1.35x | ✅ |
| rsi_14@10000 | scaled_10k | 22.14 | 30.66 | 1.38x | ✅ |
| rsi_14@100000 | scaled_100k | 224.47 | 307.83 | 1.37x | ✅ |
| rsi_14@1000000 | scaled_1m | 3640.33 | 3941.45 | 1.08x | ✅ |
| sar | overlap_extra | 45.01 | 58.45 | 1.30x | ✅ |
| sin | math_transform | 20.25 | 25.32 | 1.25x | ✅ |
| sma_20 | overlap | 13.60 | 19.02 | 1.40x | ✅ |
| sma_20@10000 | scaled_10k | 13.62 | 19.12 | 1.40x | ✅ |
| sma_20@100000 | scaled_100k | 138.24 | 203.61 | 1.47x | ✅ |
| sma_20@1000000 | scaled_1m | 3060.66 | 3094.39 | 1.01x | ✅ |
| sqrt | math_transform | 7.99 | 14.54 | 1.82x | ✅ |
| stddev_20 | volatility | 28.43 | 37.73 | 1.33x | ✅ |
| stoch_14_3_3 | momentum | 83.43 | 101.44 | 1.22x | ✅ |
| stochf_14_3 | momentum | 76.89 | 82.87 | 1.08x | ✅ |
| stochrsi_14_14_3_3 | momentum | 101.73 | 116.81 | 1.15x | ✅ |
| sub | math_operators | 2.83 | 5.65 | 2.00x | ✅ |
| sum_30 | math_operators | 11.29 | 18.95 | 1.68x | ✅ |
| t3_5 | overlap_extra | 37.41 | 341.80 | 9.14x | ✅ |
| tanh | math_transform | 23.46 | 46.84 | 2.00x | ✅ |
| tema_20 | overlap | 26.76 | 123.03 | 4.60x | ✅ |
| trima_20 | overlap | 28.57 | 30.57 | 1.07x | ✅ |
| trix_15 | momentum | 41.81 | 125.54 | 3.00x | ✅ |
| tsf_14 | statistics_extra | 35.17 | 64.41 | 1.83x | ✅ |
| typprice | price_transform_full | 5.91 | 10.22 | 1.73x | ✅ |
| ultosc_7_14_28 | momentum | 68.99 | 56.58 | 0.82x | ⚠️ |
| var_20 | statistics | 26.88 | 24.65 | 0.92x | ⚠️ |
| wclprice | price_transform | 4.68 | 45.92 | 9.81x | ✅ |
| willr_14 | momentum | 31.52 | 33.62 | 1.07x | ✅ |
| wma_20 | overlap | 22.36 | 20.46 | 0.91x | ⚠️ |

- **Total paired benchmarks**: 90
- **Finkit faster or equal on this run**: 75

Do not convert one machine snapshot into a universal performance claim. Re-run the suite on the target deployment hardware.
