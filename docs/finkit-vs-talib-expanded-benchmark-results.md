# Finkit v0.1.4 vs TA-Lib 0.7.1 扩展基准结果

> 日期：2026-09-04  
> Canonical run：GitHub Actions `33857338618`  
> Artifact：`finkit-v0.1.4-vs-talib-0.7.1`，artifact id `9930937116`  
> 配套优化方案：[finkit-vs-talib-performance-optimization-plan.md](finkit-vs-talib-performance-optimization-plan.md)

## 1. 环境

- Ubuntu 22.04 hosted runner
- Linux x86_64 / Azure
- Python 3.12.14
- NumPy 2.3.3
- Finkit 0.1.4 official manylinux ABI3 wheel
- TA-Lib Python 0.7.1
- TA-Lib Core 0.7.1
- 5 measurement trials + 2 warmups for the extended suite
- identical contiguous `numpy.float64` OHLCV arrays

所有结果都来自已安装发布包，不是源码静态推算。

---

## 2. 扩展指标结果摘要

配置了 32 个指标。由于发布包 API/类型桩不一致，有 4 项不能按文档声明的 API 调用，因此本轮得到：

- 有效指标观测：**84**（28 个指标 × 10K/100K/1M）
- Finkit public Python API 胜出：**0 / 84**
- 几何平均 `TA-Lib time / Finkit time`：**0.01614x**
- 等价表达：TA-Lib public Python API 几何平均约 **61.9x faster**

该结果和第一轮 6 指标 × 4 规模的 `0/24`、约 56-57x 差距一致，说明问题是系统性的，不是某几个指标的偶然实现问题。

### 2.1 1M bars 代表性结果

| 指标 | Finkit | TA-Lib | TA-Lib 更快约 | finite mask |
| --- | ---: | ---: | ---: | --- |
| SMA20 | 246.43 ms | 2.11 ms | 116.68x | equal |
| EMA20 | 243.23 ms | 2.93 ms | 82.88x | equal |
| WMA20 | 242.07 ms | 2.20 ms | 110.12x | equal |
| DEMA20 | 236.95 ms | 6.94 ms | 34.13x | equal |
| TEMA20 | 239.39 ms | 10.28 ms | 23.29x | equal |
| KAMA20 | 235.04 ms | 3.08 ms | 76.30x | **different** |
| SAR | 490.97 ms | 3.38 ms | 145.46x | **different / shape-parity failure** |
| MIDPOINT14 | 247.31 ms | 5.92 ms | 41.76x | equal |
| MIDPRICE14 | 248.14 ms | 6.34 ms | 39.16x | equal |
| RSI14 | 236.78 ms | 6.45 ms | 36.72x | equal |
| MACD | 763.32 ms | 20.82 ms | 36.66x | **different** |
| ADX14 | 244.81 ms | 10.51 ms | 23.28x | equal |
| CCI14 | 298.86 ms | 25.54 ms | 11.70x | equal |
| MOM10 | 230.90 ms | 1.25 ms | 184.19x | equal |
| ROC10 | 231.71 ms | 1.64 ms | 141.57x | equal |
| WILLR14 | 242.39 ms | 6.90 ms | 35.11x | equal |
| CMO14 | 237.63 ms | 6.51 ms | 36.51x | equal |
| MFI14 | 240.45 ms | 4.41 ms | 54.58x | equal |
| PLUS_DI14 | 237.94 ms | 6.83 ms | 34.82x | equal |
| MINUS_DI14 | 239.86 ms | 6.80 ms | 35.26x | equal |
| OBV | 233.11 ms | 1.50 ms | 155.28x | equal |
| AD | 233.87 ms | 2.39 ms | 97.81x | equal |
| ADOSC | 240.17 ms | 2.62 ms | 91.62x | **different** |
| ATR14 | 234.45 ms | 8.20 ms | 28.59x | equal |
| NATR14 | 236.83 ms | 7.90 ms | 29.97x | equal |
| TRANGE | 232.52 ms | 1.75 ms | 133.22x | **different** |
| VAR20 | 235.12 ms | 2.94 ms | 80.01x | equal |
| BOP | 233.06 ms | 2.34 ms | 99.72x | equal |

### 2.2 最重要的性能形态

大量完全不同复杂度的单输出指标在 1M bars 上都聚集在约 **230-250 ms**：

- SMA ~246 ms
- EMA ~243 ms
- KAMA ~235 ms
- RSI ~237 ms
- MOM ~231 ms
- ROC ~232 ms
- OBV ~233 ms
- AD ~234 ms
- ATR ~234 ms
- VAR ~235 ms

如果算法本身是主要瓶颈，这些指标不应出现如此接近的总耗时。

而三输出 MACD 为约 **763 ms**，大约是单输出指标的三倍。

这与当前 Python binding 的实现完全吻合：native 函数返回 `Vec<f64>` / tuple of Vec，PyO3 先物化 Python list，再由包级 `_as_numpy_result()` 逐层 `np.asarray()`。

因此，本轮扩展测试进一步确认：**公共 Python 指标层的主要性能瓶颈是输出物化/转换，而不是 Rust kernel。**

---

## 3. CompiledFormula 扩展结果

公式矩阵：

- `MA(CLOSE,20)`
- `EMA(CLOSE,20)`
- `RSI(CLOSE,14)`
- `ATR(HIGH,LOW,CLOSE,14)`
- `ROC(CLOSE,10)`
- `MA(CLOSE,20)+2*STD(CLOSE,20)`
- `CROSS(MA(CLOSE,5),MA(CLOSE,20))`
- `REF(CLOSE,1)`

每条公式同时测试：

- `CompiledFormula.eval()`
- `CompiledFormula.eval_zero_copy()`

共：

- 48 个有效公式观测
- Finkit 胜出：**11 / 48**
- 公式整体几何平均：reference（TA-Lib/NumPy）约 **2.4x faster**

这个结果和普通 public indicator 的 61.9x 差距完全不同，说明公式 runtime 的直接 NumPy 路径已经证明 Rust core 可以达到竞争水平。

---

## 4. 最关键的正面结果：zero-copy 简单公式已经达到/超过 TA-Lib

### 4.1 1M bars

| 公式 | Finkit `eval_zero_copy` | TA-Lib reference | 结果 |
| --- | ---: | ---: | --- |
| MA20 | **1.616 ms** | 1.941 ms | Finkit ~1.20x faster |
| EMA20 | 3.748 ms | **3.236 ms** | TA-Lib ~1.16x faster |
| RSI14 | **6.280 ms** | 7.743 ms | Finkit ~1.23x faster |

### 4.2 100K bars

| 公式 | Finkit `eval_zero_copy` | TA-Lib reference | 结果 |
| --- | ---: | ---: | --- |
| MA20 | **153.88 us** | 192.32 us | Finkit ~1.25x faster |
| EMA20 | **273.70 us** | 288.77 us | Finkit ~1.06x faster |
| RSI14 | **375.99 us** | 645.57 us | Finkit ~1.72x faster |

### 4.3 10K bars

| 公式 | Finkit `eval_zero_copy` | TA-Lib reference | 结果 |
| --- | ---: | ---: | --- |
| MA20 | **17.36 us** | 20.27 us | Finkit ~1.17x faster |
| EMA20 | **28.76 us** | 29.66 us | Finkit ~1.03x faster |
| RSI14 | **39.38 us** | 64.32 us | Finkit ~1.63x faster |

这组结果是本次基准最重要的证据：

> **只要绕过普通指标的 Vec -> Python list -> ndarray 返回链，Finkit 的 MA/EMA/RSI 核心计算已经可以和 TA-Lib 0.7.1 同量级，MA/RSI 甚至可以领先。**

---

## 5. Finkit 自身路径之间的巨大差异

同一个 1M MA20：

- `finkit.sma()` public indicator：**246.43 ms**
- `CompiledFormula("MA(CLOSE,20)").eval_zero_copy()`：**1.616 ms**

即 Finkit 自己的 formula zero-copy 路径约比自己的 public indicator API 快 **152x**。

1M EMA20：

- public indicator：243.23 ms
- formula zero-copy：3.748 ms
- 约 **65x** 差距

1M RSI14：

- public indicator：236.78 ms
- formula zero-copy：6.280 ms
- 约 **38x** 差距

这比任何外部 TA-Lib 对比都更能定位根因，因为两条路径共享 Finkit Rust core，而语言边界/输出路径不同。

因此 P0 必须直接修 binding generator，而不是先继续优化 SMA/EMA/RSI kernel。

---

## 6. `eval()` 输入复制成本也已经量化

1M bars：

| 公式 | `eval()` | `eval_zero_copy()` | zero-copy 改善 |
| --- | ---: | ---: | ---: |
| MA20 | 10.04 ms | 1.62 ms | ~6.2x |
| EMA20 | 9.99 ms | 3.75 ms | ~2.7x |
| RSI14 | 11.68 ms | 6.28 ms | ~1.9x |

这与源码一致：`eval()` 会把完整 OHLCV `slice.to_vec()` 进入 owned FormulaContext；而 `eval_zero_copy()` 可以借用 NumPy 输入。

说明优化方案中的第二个 P0 同样成立：

- 批量公式默认应优先 zero-copy；
- `eval()` 应明确为 owned/stream-retaining 语义；
- `eval_range()` 不能继续先全量复制再截范围。

---

## 7. zero-copy 并非所有公式都已经优化

复杂/非 fast-path 公式仍有明显问题。

### 1M bars

| 公式 | `eval_zero_copy()` | Reference | reference 更快 |
| --- | ---: | ---: | ---: |
| ATR14 | 45.71 ms | 11.81 ms | 3.87x |
| ROC10 | 19.80 ms | 1.53 ms | 12.94x |
| BOLL upper expression | 16.90 ms | 7.30 ms | 2.31x |
| MA CROSS | 16.59 ms | 7.88 ms | 2.11x |
| REF1 | 17.88 ms | 0.558 ms | 32.03x |

尤其值得注意：

- 1M ATR `eval_zero_copy()` 比普通 `eval()` 还慢；
- 1M ROC `eval_zero_copy()` 比普通 `eval()` 也更慢；
- REF 这种基础历史引用仍比简单 NumPy shift 慢约 32x。

源码已经说明：direct MA/EMA/RSI/BOLLMID 有专门的 zero-copy fast path，复杂公式仍可能走 array-based builtin ABI 并分配中间数组。

因此下一步不能只“保留 zero-copy API”，而必须：

1. 扩大 direct borrowed-kernel mapping；
2. 检查 `eval_zero_copy_inputs()` 对 ATR/ROC/REF 等 builtin 的 fallback；
3. 为每个 builtin 记录中间 allocation/bytes materialized；
4. 建立规则：`eval_zero_copy()` 不应在同一公式/规模上系统性慢于 `eval()`。

---

## 8. 发现的兼容性/语义问题

本轮不仅发现性能问题，还发现若干发布条件级问题。

### 8.1 public stub / native runtime API 不一致

当前类型桩/文档声明和正式 wheel 实际行为存在不一致：

1. `bollinger_bands(..., matype=0)`：正式 wheel 报 `unexpected keyword argument 'matype'`。
2. `stoch(..., slowk_matype=0, slowd_matype=0)`：正式 wheel 报 `unexpected keyword argument 'slowk_matype'`。
3. `stddev`：类型桩存在，但正式 `finkit` package 没有该 attribute。
4. `correl`：类型桩存在，但正式 `finkit` package 没有该 attribute。

这不是 benchmark 脚本应该“绕过去”的普通问题，而是 release public API contract 本身不一致。

必须加入 wheel-level contract test：

```python
for exported_name in pyi_public_api:
    assert hasattr(finkit, exported_name)
```

并检查签名参数集合一致。

### 8.2 finite mask / warm-up 不一致

扩展指标中发现：

- KAMA
- MACD
- ADOSC
- TRANGE
- SAR

存在 finite mask 或 shape/alignment 差异。

对宣称 TA-Lib-compatible 的 API，这些都应该成为兼容性 gate，而不能仅比较重叠区域数值。

### 8.3 Formula ATR 语义差异

`ATR(HIGH,LOW,CLOSE,14)` 对 TA-Lib：

- finite mask 不一致
- 最大绝对差约 **0.1946**

这已经不是浮点 ULP 级误差，需要检查：

- TR 第一根定义
- Wilder smoothing seed
- lookback/warm-up
- ATR 初始均值区间

### 8.4 Formula `STD` / BOLL 语义差异

`MA(CLOSE,20)+2*STD(CLOSE,20)` 与 TA-Lib SMA+STDDEV：

- finite mask 一致
- 最大绝对差约 **0.0869**

需要确认 Finkit `STD` 的：

- population vs sample variance
- ddof
- `nbdev`
- rolling seed
- 数值算法

在修正语义前，这个公式的性能对比不能作为“完全等价计算”的性能宣称。

---

## 9. 优化优先级根据扩展结果进一步收敛

### P0-1：普通 Python indicator 直接 PyArray 输出

最高优先级，收益预计是数量级的。

目标是让普通：

```python
finkit.sma(close)
```

的语言边界路径接近当前：

```python
CompiledFormula("MA(CLOSE,20)").eval_zero_copy(...)
```

而不是继续让前者慢 100x 以上。

### P0-2：Public API SSOT / stub / wheel contract

修复：

- BBANDS matype signature
- STOCH MA type signature
- stddev export
- correl export
- 全量 `.pyi` vs runtime introspection 测试

### P0-3：Formula compatibility

先修：

- ATR
- BOLL/STD
- MACD warm-up
- KAMA/ADOSC/TRANGE/SAR alignment

### P0-4：zero-copy builtin coverage

优先 profile/fix：

- ROC
- ATR
- REF
- CROSS / composite DAG

### P0-5：wheel-level performance gate

不能再只依赖 Rust core benchmark。

---

## 10. 本轮结论

扩展测试已经把问题从“可能是 Python binding”提升到几乎可以确定的工程结论：

1. **普通 public indicator API 的 30x-180x 差距主要来自 binding/output materialization。**
2. **Finkit Rust core 本身并没有普遍落后 TA-Lib。** MA/RSI zero-copy 已经实测领先，EMA 接近同量级。
3. **Formula runtime 的简单 direct fast path 方向正确。**
4. **复杂 builtin zero-copy 仍有 fallback/中间分配问题。**
5. **release API contract 存在 stub/运行时签名与导出不一致。**
6. **存在多处 TA-Lib warm-up/语义不一致，ATR/STD 类必须先纠正语义再讨论性能。**

因此下一轮开发不应该先做泛化 SIMD 大重构，而应该按：

**Python binding NumPy 直出 -> API contract 修复 -> Formula semantic parity -> zero-copy builtin 扩面 -> buffer/DAG -> Core hotspot**

执行。

这也是配套优化方案中 P0/P1/P2 排序的实测依据。
