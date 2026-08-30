# FFI 内存所有权契约（Memory Ownership Contract）

> 配套任务：**A4 — FFI 内存所有权契约 + 泄漏测试**（见 `docs/UPGRADE_PLAN_2026.md`）。
> 自动化泄漏测试实现见 `alpha_ta_ffi_common::leak` + 各绑定 `mod tests::ffi_heap_no_leak_*`。

本文件是 AlphaTA 八语言 FFI 绑定的**单一内存所有权事实源**。它规定：每个把
Rust 堆所有权转移给宿主语言的 `ta_*` 导出函数，必须有一个配对释放函数；调用
方负责释放，绑定侧**绝不**自动释放。

---

## 通用原则

1. **生产者分配，消费者释放。** 任何返回裸指针 / 句柄的导出函数，其返回值的
   释放责任在调用方。绑定本身不会在返回后替你 free。
2. **不释放 null 是安全的。** 所有 `ta_free_*` 对 null 指针 / 零长度均为 no-op
   （由 A3 的 `ffi_catch_void` 包裹，panic 也被隔离）。
3. **不要重复释放、不要释放栈/宿主内存。** 只有由配对 `ta_*` 分配器产出的指针
   才允许传入对应 `ta_free_*`。
4. **错误结果也要释放。** `make_error_result` 等错误路径同样分配，必须用同一个
   `ta_free_*` 释放。

---

## 逐绑定契约

### Go（`alpha-ta-go`）

| 分配函数 | 返回类型 | 释放函数 | 备注 |
|----------|----------|----------|------|
| `ta_sma` / `ta_ema` / …（指标） | `*mut TaResult` | `ta_free_result` | `TaResult` 内含 `data: Vec<f64>` + `error: CString`，`ta_free_result` 一并释放 |
| `ta_formula_eval*` / `ta_formula_eval_*` | `*mut c_char` | `ta_free_string` | 公式引擎返回的 JSON 字符串 |

- `TaResult` 的内存布局：`Box<TaResult>` → 内部 `Vec<f64>`（经 `into_raw_parts`）
  + 可选 `CString` 错误串。`ta_free_result` 先释放 `data` 再释放 `error` 后释放
  外层 `Box`。
- **测试覆盖**：`ffi_heap_no_leak_alloc_free_cycle`（go）循环 400 次
  `ta_sma`+`ta_free_result` 与 `ta_formula_eval`+`ta_free_string`，断言 live 堆字节
  回到基线。

### .NET（`alpha-ta-dotnet`）

| 分配函数 | 返回类型 | 释放函数 | 备注 |
|----------|----------|----------|------|
| 指标函数（`ta_sma` 等） | `c_int`（写入调用方 `out` 缓冲） | — | **零拷贝**，不转移所有权 |
| `ta_formula_eval*` | `*mut c_char` | `ta_free_cstring` | 公式引擎 JSON 串 |
| （直接释放标量） | `*mut c_double` | `ta_free` | 标量 `Box<f64>` |
| （直接释放数组） | `*mut c_double` + `length` | `ta_free_array` | `Vec<f64>`（`from_raw_parts`） |

- .NET 指标走**调用方缓冲区**（caller-allocated `out`），不跨边界转移堆所有权；
  唯一转移路径是公式引擎的 `*mut c_char`。
- **测试覆盖**：`ffi_heap_no_leak_formula_eval_cycle`（dotnet）循环 400 次
  `ta_formula_eval`+`ta_free_cstring`，并直接演练 `ta_free` / `ta_free_array` 释放路径。

### iOS（`alpha-ta-ios`）

- 所有指标：`alpha_ta_sma(input, len, period, out: *mut f64) -> i32`。
  **调用方拥有 `out` 缓冲区**，函数返回 `0` 成功 / `-1` 错误，不转移任何堆所有权。
- 蜡烛图：`alpha_ta_detect_candlestick(...) -> i32`（返回检测计数），同样不转移堆。
- **无 `ta_free_*` 契约**（没有跨边界堆所有权）。
- **测试覆盖**：`ffi_heap_no_leak_indicator_cycle`（ios）循环 400 次指标 + 蜡烛图
  调用，断言 Rust 侧内部临时分配被完全回收（防止间接泄漏）。

### Java（`alpha-ta-java`）

Java 导出函数需要活着的 `JNIEnv`（即一个 JVM），因此**不能在 `cargo test` 中单元测试**
（测试二进制内无 JVM）。其所有权契约有两类：

1. **JNI 局部引用（local references）。** 每个返回 `jstring` 的导出
   （`formulaEval*`、`klineChartToSvg` 等）产出一个局部引用，调用方的 JNI 帧拥有，
   必须用 `freeJString` 释放（或帧 detach 时自动释放）。忘记释放会泄漏 JVM 局部
   引用槽。`freeJString` 对 null 安全。
2. **按句柄管理的长生命周期 Rust 状态。** `klineDataNew` / `klineChartNew` 把
   `KlineData` / `KlineChart` 存入进程级全局 map，返回 `i64` 句柄；**必须**用
   `klineDataFree` / `klineChartFree` 释放，否则句柄泄漏 = 整个进程生命周期内泄漏
   底层 Rust 对象。

- **验证方式**：由 Android / 宿主 JVM 集成测试覆盖（不在本仓库 `cargo test` 范围）。

### C / Python / Node / Android

- C 绑定沿用 Go 风格（`*mut TaResult` + `ta_free_result`）；Python/Node 经 ctypes 绑定，
  释放由各自语言的 GC 包装层负责（见各绑定 `src`）。
- Android 复用 Java 契约（见上）。

---

## 泄漏测试机制

- 计数分配器 `alpha_ta_ffi_common::leak::CountingAlloc`：在测试的二进制里通过
  `#[global_allocator]` 安装（仅 `#[cfg(test)]`），对每次 Rust 堆分配/释放做净字节
  计数。
- 每个绑定测试快照「前 / 后」live 字节；正确 alloc+free 配对下两者（留小容差）相等；
  遗漏 `ta_free_*` 则每个循环累计增长，断言失败。
- 这是 Windows / stable Rust 下 valgrind / ASan 的**可移植替代**，覆盖「调用方忘记
  释放」这类泄漏；不覆盖 use-after-free / double-free（那类由 `ffi_catch_void` 的
  null 守卫 + 调用方规范约束）。

---

## 违规示例（禁止）

```c
// ❌ 忘记释放 → 每次调用泄漏一个 TaResult（含 Vec + CString）
TaResult* r = ta_sma(input, 512, 14);
double v = r->data[0];
// 缺失 ta_free_result(r);

// ✅ 正确配对
TaResult* r = ta_sma(input, 512, 14);
double v = r->data[0];
ta_free_result(r);
```
