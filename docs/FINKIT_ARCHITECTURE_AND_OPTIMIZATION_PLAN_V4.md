# Finkit 架构、全链路审计与优化改进方案 V4

> 仓库：`coeasy/finkit`  
> 审计基线：`main`  
> 基线提交：`3fb8b3b16a7038201ca99cf428e20ae958f778f7`  
> 审计日期：2026-10-03  
> Workspace 开发版本：`0.2.0`  
> 当前最后正式 GitHub Release：`v0.1.15`  
> 文档目标：在当前真实代码和真实 CI 状态上，连续执行至少三轮架构/逻辑/发布链审计，识别主体流程断链、核心链路断链、孤儿逻辑、死循环/不可终止路径、跨语言断链、生成代码漂移、平台差异和发布门禁缺口，并给出可直接落地的修复顺序和验收合同。

---

## 0. 执行结论

Finkit 当前已经具备比较完整的 **Rust-first Quant Compute Runtime** 骨架，核心数值、公式、执行计划、Factor、Streaming、研究分析、可视化、多语言绑定、CLI、WASM、打包和安装器都不是空壳。

本次重新以 2026-10-03 当前 `main` 为基线做三轮检查后，结论是：

1. **主体架构方向正确，不需要推倒重写。**
2. Formula → Plan → UnifiedExecutor、FactorPlan → UnifiedRuntime、FFI shared contract、CLI/Visualization、WASM 等主要链路已经真实存在。
3. 9 月 26 日旧审计中的多项问题已经被后续提交修复，不能继续按旧清单重复开发。
4. 但当前 `main` 仍然**不能宣布“主体流程全部联通且达到发布条件”**，因为同一 SHA 下仍有多个真实 CI 红灯。
5. 本轮发现了两个旧门禁没有覆盖的结构性问题：
   - `core/src/circuit_breaker.rs` 是**真实未进入 Rust module graph 的孤儿源码文件**；
   - Python wheel 构建存在**构建前原地源码变换**，普通 workspace compile 绿并不能证明变换后的源码可编译。
6. 当前发布链至少有五类 P0 缺陷：
   - Rustfmt 失败；
   - Workspace Clippy 因新 lint 失败；
   - Python 四平台 wheel 全部编译失败；
   - macOS / AArch64 因 CPU helper `cfg` 边界失败；
   - C/C++ TA-Lib numeric contract 生成请求 JSON 缺少字符串引号。
7. 已有无界循环经过历史审计，当前未发现新的不可终止循环；但仍应增加“循环终止性”专用永久门禁，而不是依赖人工复核。
8. 当前正确策略不是继续扩功能，而是先完成 **P0 发布闭环 → P1 架构收敛 → P2 性能与产品化优化**。

### 当前发布判断

**NOT RELEASE READY。**

在以下条件同时满足前，不应创建 `v0.2.0` 正式 tag：

- `CI` 同一 SHA 全绿；
- `Python wheels` 四平台构建、安装、消费测试全绿；
- `TA-Lib installed-wheel release gate` 全绿；
- `Multilang cross-platform` 全绿；
- `Multilang release` dry-run / package contract 全绿；
- Docs、security、runtime integration、performance/memory gate 全绿；
- 不存在 tracked build artifacts；
- Rust source reachability gate 无孤儿源码；
- release readiness aggregator 对同一 SHA 给出 PASS。

---

# 1. 当前真实架构

## 1.1 Workspace

当前根 workspace 主要包含 14 个成员：

```text
core
factor-analysis
visualization

ffi/
├─ ffi-common
├─ c-binding
├─ python-binding
├─ node-binding
├─ go-binding
├─ dotnet-binding
├─ ios-binding
├─ java-binding
└─ android-binding

cli
wasm
```

职责分层：

```text
┌───────────────────────────────────────────────────────┐
│ Product / SDK                                        │
│ Python / Node / Java / Go / .NET / C/C++             │
│ iOS / Android / WASM / CLI                           │
├───────────────────────────┬───────────────────────────┤
│ Shared Host Contract                                  │
│ ffi-common: operation / error / JSON / ownership      │
│ panic boundary / streaming / factor / research       │
├───────────────────────────┴───────────────────────────┤
│ Unified Runtime                                      │
│ ComputePlan / FactorPlan / UnifiedRuntime             │
│ UnifiedExecutor / BufferArena / StateArena            │
├───────────────────────────────────────────────────────┤
│ Canonical Semantic Layer                             │
│ Formula AST / Operation / FactorGraph / dependencies  │
│ lookback / capabilities / effects                    │
├───────────────────────────────────────────────────────┤
│ Canonical Numeric Kernel Layer                       │
│ math / indicators / rolling / patterns / transforms  │
│ batch / streaming / SIMD                             │
├───────────────────────────────────────────────────────┤
│ Factor Research                                      │
│ prepare / IC / validation / risk / report            │
├───────────────────────────────────────────────────────┤
│ Visualization                                        │
│ scene / layout / render / SVG / HTML / WebGL/WebGPU  │
└───────────────────────────────────────────────────────┘
```

## 1.2 当前最重要的核心主线

目标架构应继续收敛为：

```text
FunctionSpec / OperationSpec (SSOT)
                │
                ▼
        Canonical Semantic Graph
                │
                ▼
           Compute Plan
                │
       ┌────────┼─────────┐
       ▼        ▼         ▼
     Full     Range     Streaming
       └────────┼─────────┘
                ▼
        Unified Executor
                │
      ┌─────────┼─────────┐
      ▼         ▼         ▼
   Kernels   BufferArena StateArena
                │
                ▼
        Canonical Artifact
```

所有面向用户的 API 最终都应只是这一主线的 Adapter：

```text
Python
Node
Java
Go
.NET
C/C++
CLI
WASM
   │
   ▼
Shared Contract
   │
   ▼
Unified Runtime
```

---

# 2. 三轮审计方法

本次不是只读旧文档，而是按当前 `main` 重新检查：

### Round 1 — 主体流程、核心链路、模块图、孤儿逻辑

检查：

- workspace / crate 依赖；
- Formula / Factor / Runtime 主链；
- FFI / CLI / WASM / Visualization 接线；
- tracked source 是否进入 module graph；
- orphan scripts / orphan docs / orphan public modules；
- tracked build outputs。

### Round 2 — 错误语义、终止性、状态、缓存、资源、CI 基线

检查：

- panic / silent fallback；
- NaN；
- `loop {}` 和迭代上限；
- full / range / streaming；
- CI format / clippy / core test / runtime integration；
- 构建工具链漂移。

### Round 3 — 多语言、生成代码、跨平台、发布与安装消费

检查：

- Python wheel；
- Node / .NET / AArch64；
- C/C++ shared operation contract；
- installed-wheel gate；
- multilang release；
- 同一 SHA 的发布判定。

---

# 3. Round 1：主体流程 / 核心链路 / 孤儿逻辑

## 3.1 已确认连通的链路

### Formula

当前已经形成：

```text
source
  ↓
parser / dialect
  ↓
AST
  ↓
semantic metadata
  ↓
plan / hot plan
  ↓
unified dispatch
  ↓
UnifiedExecutor
  ↓
canonical kernels
```

最近提交已经关闭大量 Plan kernel backlog，并通过 registry signature test 自动构造调用验证 Tree / Plan 双路径。

因此，不应继续沿用 9 月旧审计中“Plan 大量缺 kernel”的旧判断。

### Factor

当前存在：

```text
FactorProvider
FactorGraph
FactorEngine
FactorPlan
BorrowedFactorContext
UnifiedRuntime
DirtyRange
```

`core/src/unified_runtime.rs` 已经把 `FactorPlan` 真实接入 full / borrowed / range / range-into 执行。

### DirtyRange

实现已经包含：

- 输入 dirty interval；
- forward dependency propagation；
- backward lookback expansion；
- retained output splice；
- full/range trace；
- recomputed rows 证据。

方向正确。

### FFI

`ffi/ffi-common` 已经有：

```text
operation
execute
formula
factor
stream contract
research
error
panic
shared runtime
contract conformance
```

多语言绑定不再全部自己维护独立业务语义。

### Visualization

当前有真实 scene / renderer / WebGL / WebGPU / SVG / HTML 等实现。旧版 CLI SVG/HTML 绕过 renderer 的问题已经不是当前基线的主要缺陷。

### CI 已存在的 orphan 门禁

仓库已经有：

- `check_orphan_scripts.py`
- `check_orphan_docs.py`
- `check_orphan_modules.py`
- `check_workflow_liveness.py`
- `check_script_references.py`
- `check_dead_code_reasons.py`

这些方向是正确的。

---

## 3.2 Round 1 新发现：真实 Rust 孤儿源码

### 问题

对各 crate 顶层 `.rs` 文件与 `lib.rs` module declaration 做交叉检查时：

- `factor-analysis/src`：无顶层漏接；
- `visualization/src`：无顶层漏接；
- `ffi/ffi-common/src`：无顶层漏接；
- `wasm/src`：无顶层漏接；
- `core/src`：发现 1 个真实未声明文件：

```text
core/src/circuit_breaker.rs
```

`core/src/lib.rs` 没有：

```rust
mod circuit_breaker;
```

也没有：

```rust
pub mod circuit_breaker;
```

GitHub 代码引用搜索也没有发现该模块调用方。

因此这不是“public module no caller”，而是更早一步：

> **源码文件根本未进入 Rust module graph。**

文件中包含：

```text
CircuitBreaker
CircuitBreakerError
State
```

并且文档示例声称：

```rust
use finkit::circuit_breaker::CircuitBreaker;
```

但实际上该 public path 并不存在。

这是典型“代码看起来已经实现、实际上永远不会进入产品”的孤儿逻辑。

### 为什么现有门禁漏掉

`check_orphan_modules.py` 自己明确写了限制：

- 只从 `core/src/lib.rs` 中读取已经声明的 `pub mod`；
- 检测“已经进入 module graph 但没有 caller”的模块；
- 无法看到一个 `.rs` 文件压根没被声明的情况。

因此 `circuit_breaker.rs` 正好位于门禁盲区。

### 建议修复

Finkit 的产品边界是 Quant Compute Runtime，不做远程服务编排。

`circuit_breaker.rs` 注释的主要使用场景是：

> remote formula evaluator / downstream service

这不属于当前核心边界。

**建议优先删除该孤儿文件，而不是为了“保留代码”把它强行暴露成公共 API。**

如果后续确实需要 host-side resilience，则应单独设计：

```text
HostRuntimePolicy
├─ timeout
├─ retry
├─ cancellation
└─ circuit breaker
```

而不是把一个孤立 utility 直接挂到 core。

### 新永久门禁

新增：

```text
scripts/check_rust_source_reachability.py
```

要求：

1. 遍历所有 workspace crate `src/**/*.rs`；
2. 从 crate root `lib.rs/main.rs` 建 module graph；
3. 识别：
   - `mod foo;`
   - `pub mod foo;`
   - `#[path = "..."]`
   - `mod.rs`
4. 任何 tracked `.rs`：
   - 必须 module-reachable；
   - 或明确记录在 generated / build-only allowlist；
5. allowlist 必须 non-rotting；
6. CI 发现新孤儿文件立即失败。

---

## 3.3 Round 1 新发现：tracked Criterion 生成产物

递归 Git tree 显示：

```text
core/target/criterion/**
```

约 **337 个**文件已经被 Git 跟踪。

内容包括：

```text
benchmark.json
estimates.json
sample.json
tukey.json
report/index.html
mean.svg
median.svg
regression.svg
violin.svg
...
```

而 `.gitignore` 已经明确忽略：

```text
/target
target/criterion/
```

说明这些文件属于历史上已经被提交后留下的 tracked build outputs。

### 风险

1. 仓库体积膨胀；
2. benchmark 原始环境数据混入源代码历史；
3. 用户容易把旧 Criterion report 当成当前性能结论；
4. 基准结果与机器/CPU/工具链耦合；
5. `git diff` 噪音；
6. release/source archive 带入无关构建数据。

### 修复

执行：

```bash
git rm -r --cached core/target
```

保留真正需要长期版本化的基准：

```text
docs/benchmark-baseline.json
docs/benchmark-results.md
machine-readable summary
```

原始 Criterion report 改为：

```text
GitHub Actions artifact
```

### 新永久门禁

```text
scripts/check_no_tracked_build_artifacts.py
```

禁止 Git 跟踪：

```text
**/target/**
**/dist/**
**/node_modules/**
**/bin/**
**/obj/**
*.so
*.dll
*.dylib
*.node
```

除非路径明确加入 checked-in fixture allowlist。

---

# 4. Round 1 修复后验收

Round 1 必须满足：

```text
[x] circuit_breaker.rs 删除或真实接入并测试
      -> 已删除：git 记录了 core/src/circuit_breaker.rs 的删除，全树零引用。
         同轮删除的还有 formula/range_zero_copy.rs、indicators/compat.rs、
         math/buffer_pool.rs、math/kernels/macd_core.rs。
[x] Rust source reachability gate 0 orphan
      -> check_rust_source_reachability.py: 479 tracked source files, all module-reachable
[x] core/target/criterion 从 Git history 当前 tree 移除
      -> 282 个 tracked 产物已 staged delete；.gitignore 的 core/target/ 规则保留
[x] no-tracked-build-artifacts gate 通过
      -> 1299 tracked files，无构建产物。本轮该门禁自身补上了「仓库根目录可渲染
         产物」规则：此前它对 7.3 MB 的 example 输出视而不见却报告 "none look
         like build output"（见 §28.3）
[x] check_orphan_scripts 通过
[x] check_orphan_docs 通过
      -> 本轮首次运行时报出 8 个不可达文档（见 §28.1）
[x] check_orphan_modules 通过
[x] workspace cargo check 通过
      -> 用 --all-targets 跑：examples 与 benches 一并编译，证明删除的模块
         没有被任何非测试 target 引用
```

只有这些完成后，才进入下一轮。

---

# 5. Round 2：错误、死循环、状态、缓存与 CI 基线

## 5.1 NaN 防护明显增强

最新提交已经修复 45 个：

```rust
partial_cmp(...).unwrap()
```

在 NaN 下 panic 的问题，并增加：

```text
scripts/check_nan_unsafe_ordering.py
```

这是正确的“修 bug class + 永久 gate”方法。

以后所有修复都应该尽量复制这种模式，而不是只改单点。

---

## 5.2 死循环 / 不可终止路径

已有历史审计明确核验 core 中无界 `loop {}`：

- Formula loop 有 `MAX_LOOP_ITERATIONS`；
- Stateful Formula 有 `STATEFUL_MAX_LOOP_ITERATIONS`；
- combinations 等循环具有可证明终止条件；
- harmonic pivot merge 有有限输入边界。

本轮基于当前代码/测试和最新 CI，没有发现新的无限循环证据。

### 但当前门禁仍不够形式化

建议新增：

```text
scripts/check_unbounded_loops.py
```

规则：

```text
loop { ... }
while true { ... }
recursive evaluator
```

必须满足至少一种：

1. 显式 iteration budget；
2. timeout / cancellation token；
3. 单调变量 + 结构化 break；
4. 源码旁 `// SAFETY-TERMINATION:` 注释；
5. allowlist + 自动 non-rotting。

对 Formula / Pine / Stateful parser / DAG walk 再增加：

```text
max_ast_depth
max_recursion_depth
max_nodes
max_execution_steps
max_allocated_bytes
```

---

## 5.3 当前 main：Format 实际失败

最新 `CI` 不是全绿。

Format job：

```text
cargo fmt --all -- --check
```

失败。

主要差异位于：

```text
core/src/indicators/classic_patterns.rs
core/tests/formula_registry_signature.rs
```

### 修复

使用 CI 相同 rustfmt：

```bash
cargo +<pinned-format-toolchain> fmt --all
cargo +<pinned-format-toolchain> fmt --all -- --check
```

不要手动改换行。

---

## 5.4 当前 main：Workspace Clippy 实际失败

当前 workspace clippy 的 fatal issue：

```text
factor-analysis/src/validation.rs:339
```

存在：

```rust
const EULER_GAMMA: f64 = 0.5772156649015329;
```

当前 stable Clippy 触发：

```text
clippy::approx_constant
```

并且该 lint 当前为 deny。

### 修复策略

优先顺序：

1. 如果项目决定提高/固定编译工具链，并且标准库已经暴露相应 Euler–Mascheroni 常数：
   使用标准库常数；
2. 如果必须保持当前 MSRV 且该常数在 MSRV 不可用：
   保留项目常数，但在**最小范围**增加：
   ```rust
   #[allow(clippy::approx_constant)]
   ```
   并写明：
   ```text
   required for MSRV compatibility
   ```
3. 不允许全 crate / 全 workspace disable 该 lint。

---

## 5.5 更深层根因：工具链漂移

当前 CI：

- Format 明确 pin 到 Rust `1.98`；
- Clippy 大量 job 使用：
  ```yaml
  dtolnay/rust-toolchain@stable
  ```
- workspace `rust-version` 是 `1.85`。

这形成三套语义：

```text
MSRV = 1.85
format = 1.98
lint = moving stable
```

`stable` 更新后可能出现：

```text
昨天全绿
今天无代码变化却新增 deny lint
```

### 建议

增加：

```text
rust-toolchain.toml
```

至少定义：

```text
channel = <project-pinned>
components = rustfmt, clippy
```

CI 分成三类：

### 1. Canonical CI

固定 toolchain：

```text
fmt
clippy
tests
package
```

### 2. MSRV

```text
Rust 1.85
cargo check
selected tests
```

### 3. Future compatibility

```text
stable
beta
```

允许警告或独立非阻塞 job。

这样既不被 moving stable 随机打红，也不会丢掉未来兼容性预警。

---

## 5.6 Clippy warning debt

日志中：

```text
factor-analysis: 261 warnings
visualization: 652 warnings
```

大量是：

- cast precision loss；
- manual midpoint；
- must_use candidate；
- formatting；
- assert style；
- field_reassign_with_default。

不要一次性无脑 `cargo clippy --fix` 全仓。

应该：

1. 先修 correctness-sensitive：
   - overflow；
   - precision；
   - NaN；
   - invalid conversion；
2. 然后修 API contract；
3. 最后处理纯 style。

建立：

```text
clippy-warning-baseline.json
```

要求 warning 数只能下降不能上升。

---

# 6. Round 2 修复后验收

```text
[ ] cargo fmt --all -- --check PASS
[ ] canonical pinned clippy PASS
[ ] MSRV cargo check PASS
[ ] core tests PASS
[ ] factor-analysis tests PASS
[ ] visualization tests PASS
[ ] NaN safety gate PASS
[ ] unbounded-loop gate PASS
[ ] no silent-error regression PASS
[ ] timeout / recursion / step-budget tests PASS
```

---

# 7. Round 3：多语言 / 生成代码 / 跨平台 / 发布链

这是本轮最重要的一组发现。

当前同一 `main` SHA：

```text
Docs Check                         PASS
CI                                 FAIL
Python wheels                      FAIL
TA-Lib installed-wheel release gate FAIL
Multilang cross-platform           FAIL
Multilang release                  FAIL
```

因此当前绝不能宣称：

> 所有前后端、多语言和发布链已经全部贯通。

---

# 8. P0：Python 四平台 wheel 全红

四个平台：

```text
linux-x86_64
macos-x86_64
macos-arm64
windows-x86_64
```

全部在 Build ABI3 wheel 失败。

## 8.1 实际错误

编译错误：

```text
can't compare `u16` with `&str`
```

发生在构建准备后的：

```text
ffi/python-binding/src/native_fast_path.rs
```

逻辑：

```rust
if operation == "var" && timeperiod == 20 {
    ...
}
```

但构建前脚本已经把函数签名：

```rust
operation: &str
```

重写为：

```rust
operation: u16
```

## 8.2 根因

Python wheel 在真正编译前执行：

```text
scripts/prepare_python_hot_bindings.py
```

该脚本继续调用：

```text
repair_python_new_indicator_bindings.py
prepare_python_registry_ssot.py
apply_architecture_v3_unified_kernel.py
sync_bindings.py --generate
apply_formula_v3_fast_path.py
optimize_python_bindings
...
```

其中 `apply_architecture_v3_unified_kernel.py` 对：

```rust
fast_unary_period_scale
```

做文本变换：

```text
operation: &str -> operation: u16
"stddev" => 1 =>
"var"    => 2 =>
```

但是漏掉前置判断：

```rust
if operation == "var"
```

于是：

```text
canonical tracked source     可以编译
      ↓ build-time mutation
transformed source           编译失败
```

这解释了为什么普通 workspace compile 可以绿，而 wheel 全红。

---

## 8.3 立即修复

在 transformation 中完整处理函数语义，而不是只处理 match arm。

临时修复至少需要把：

```rust
if operation == "var" && timeperiod == 20
```

变成：

```rust
if operation == 2 && timeperiod == 20
```

同时 postcondition 必须验证：

- 参数类型是 `u16`；
- 函数作用域内不再出现：
  ```text
  operation == "
  match operation { "...
  ```
- facade 数字 ID 与 Rust 数字 ID 一致。

---

## 8.4 正确的永久修复

当前“构建前 patch tracked source”的方案长期风险很高。

建议从：

```text
tracked Rust source
  ↓
Python text patch
  ↓
another patch
  ↓
generator
  ↓
optimizer
  ↓
compile
```

升级为：

```text
FunctionSpec / BindingSpec SSOT
        ↓
one deterministic generator
        ↓
generated Rust module
        ↓
compile
```

推荐：

```text
OUT_DIR / generated.rs
```

或：

```text
src/generated/
```

由 generator 完整产生，不再用字符串 replace 修改 handwritten Rust。

### 必须新增 build-state gate

PR 上先跑：

```bash
python scripts/prepare_python_hot_bindings.py
cargo check -p finkit-python --release --locked
cargo test -p finkit-python --locked
cargo fmt --all -- --check
```

并做：

```text
generator idempotence
postcondition assertions
git diff --check
```

这会在进入 wheel matrix 前几分钟就发现问题，而不是等四个平台都重复失败。

---

# 9. P0：macOS Node / .NET + AArch64 同根失败

当前失败：

```text
dotnet-macos
node-macos
aarch64-warning-contract
```

三者日志都指向：

```text
function `avx2_fma_available` is never used
```

因为：

```text
RUSTFLAGS=-D warnings
```

导致 dead_code 成为 error。

## 根因

x86/AVX2/FMA 专用 helper 的定义和其 caller 的 `cfg` 边界没有完全一致。

在 ARM/macOS：

```text
helper 被编译
caller 被 cfg 掉
=> helper dead code
=> -D warnings
=> build fail
```

## 修复

不要：

```rust
#[allow(dead_code)]
fn avx2_fma_available() ...
```

作为长期方案。

应该让 helper 和调用方使用同一 architecture contract：

```rust
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn avx2_fma_available() -> bool {
    ...
}
```

如果还涉及 feature：

```rust
#[cfg(all(
    feature = "simd",
    any(target_arch = "x86", target_arch = "x86_64")
))]
```

### 更好的结构

集中成：

```text
core/src/cpu_features.rs
```

对外只暴露：

```rust
CpuCapabilities
```

各架构：

```text
x86/x86_64 -> AVX2/FMA detection
aarch64    -> NEON
wasm32     -> SIMD128
fallback   -> scalar
```

避免每个 kernel 自己散落 `cfg`。

---

# 10. P0：C/C++ TA-Lib numeric contract 的请求 JSON 有结构错误

当前 `cpp-linux`：

- Rust C binding tests：31/31 PASS；
- C++ `test_indicators` FAIL；
- 第一条 vector 显示：
  ```text
  TA-Lib operation returned an error: ACCBANDS
  ```

继续追踪后确认：

**ACCBANDS 内核本身并不是这里的首要根因。**

## 10.1 生成数据

`talib_numeric_contract_generated.hpp` 生成：

```cpp
kSemanticProfile = R"finkit(talib_0_8_0)finkit";
vector.operation  = R"finkit(ACCBANDS)finkit";
```

这些都是普通文本，不带 JSON 引号。

## 10.2 C++ 请求拼接

测试使用：

```cpp
std::string("{\"operation\":") + vector.operation +
",\"semantic_profile\":" + kSemanticProfile +
...
```

实际得到近似：

```json
{
  "operation": ACCBANDS,
  "semantic_profile": talib_0_8_0,
  ...
}
```

这是无效 JSON。

因此 ACCBANDS 只是第一条 vector，第一个暴露请求解析失败。

## 10.3 C 测试也存在相同风险

C 测试：

```c
"{\"operation\":%s,\"semantic_profile\":%s,..."
```

同样直接插入：

```text
ACCBANDS
talib_0_8_0
```

而 generator 中：

```text
finkit_test_contract_semantic_profile[]
```

也是普通字符串，不是 JSON fragment。

---

## 10.4 正确修复

不要在 C/C++ 测试里手工猜 quoting。

Generator 应直接生成：

```text
operation_json
semantic_profile_json
```

例如 Python generator：

```python
json.dumps(vector["operation"])
json.dumps(payload["semantic_profile"])
```

然后 C/C++ fixture 中字段明确叫：

```text
operation_json
semantic_profile_json
```

测试只做拼接，不再二次决定 JSON 编码规则。

更进一步，可以生成**完整 request JSON**：

```text
vector.request_json
```

C/C++ 测试只调用：

```text
ta_operation_execute_json(vector.request_json)
```

彻底消灭 host 侧手拼 JSON。

---

## 10.5 必须增加两层测试

### Layer 1 — ffi-common 原生合同

直接对 201 个 vector：

```text
execute_operation_json(request_json)
```

全部执行。

这能证明：

```text
golden contract
  ↓
shared dispatcher
```

是通的。

### Layer 2 — C / C++

复用**同一 request JSON bytes**调用 C ABI。

这样失败时能准确区分：

```text
shared runtime defect
vs
FFI serialization defect
```

同时错误日志必须打印：

```text
operation
request
response
error code
```

不能只打印：

```text
operation returned an error
```

---

# 11. P0：Installed-wheel gate 同样被 Python transform 问题阻断

`TA-Lib installed-wheel release gate` 当前失败，不是独立问题，而是 Python wheel 同一 build-time transform defect 的下游表现。

这说明 release gate 的方向正确：

> 必须验证“安装后的真实 wheel”，不能只验证源码 crate。

修复 Python generator 后，必须继续保留 installed-wheel gate，不要为了绿灯绕掉它。

---

# 12. 当前多语言状态

当前同一 SHA 下，Multilang release 已有多项成功：

```text
wasm32          PASS
dotnet-linux    PASS
node-linux      PASS
go-linux        PASS
ios-xcframework PASS
rust-package    PASS
java-linux      PASS
android-aar     PASS
```

失败：

```text
cpp-linux       FAIL
```

Multilang cross-platform：

```text
dotnet-windows-x64 PASS
node-windows-x64   PASS

dotnet-macos       FAIL
node-macos         FAIL
aarch64 contract   FAIL
```

因此可以得出：

> 多语言架构主体存在，但当前平台矩阵还没有同 SHA 全闭环。

---

# 13. 三轮后的核心问题清单

## P0 — 发布前必须修复

| ID | 问题 | 根因 | 修复 |
|---|---|---|---|
| P0-01 | Python 4 平台 wheel 全失败 | build-time transform 将 `operation` 变成 u16，但漏改字符串比较 | 修 generator + generated-state compile gate |
| P0-02 | macOS Node/.NET + AArch64 失败 | AVX helper cfg 与 caller cfg 不一致 | CPU capability cfg 收敛 |
| P0-03 | C/C++ numeric contract 首条失败 | operation/profile 被拼成无效 JSON | generator 输出完整 request JSON |
| P0-04 | Format 红 | latest source 未用 pinned rustfmt 格式化 | pinned cargo fmt |
| P0-05 | Workspace Clippy 红 | moving stable 新 `approx_constant` lint | pin lint toolchain + MSRV-safe constant |
| P0-06 | CI toolchain 漂移 | fmt pin / clippy stable / MSRV 三套 | canonical toolchain + MSRV + future lanes |
| P0-07 | 同 SHA 多 workflow 红 | 缺统一 release readiness aggregator | 增加同 SHA 发布总门禁 |

## P1 — 架构与仓库治理

| ID | 问题 | 修复 |
|---|---|---|
| P1-01 | `circuit_breaker.rs` 未进入 module graph | 删除，或按正式 HostRuntimePolicy 接入 |
| P1-02 | 现有 orphan module gate 看不到 undeclared `.rs` | 新增 Rust source reachability gate |
| P1-03 | `core/target/criterion` 337 个生成文件被 tracked | 从 Git 移除，改 CI artifact |
| P1-04 | Python build 使用多重文本 patch | SSOT → deterministic generator |
| P1-05 | Formula / Factor / Operation 多套图模型仍偏多 | 收敛 SemanticGraph → ComputePlan |
| P1-06 | warning debt 很大 | correctness-first warning budget |
| P1-07 | 循环终止性主要靠人工历史审计 | 新增 unbounded loop gate |

## P2 — 性能与长期产品化

- DAG CSE；
- kernel fusion；
- scheduler；
- BufferArena / StateArena → RuntimeContext；
- Arrow / NumPy / Polars zero-copy；
- workload benchmark；
- WASM/npm 产品化；
- binding/schema/doc 自动生成。

---

# 14. 建议的最终运行时架构

```text
                     FunctionSpec
                         │
                         ▼
                 Canonical Registry
                         │
       ┌─────────────────┼─────────────────┐
       ▼                 ▼                 ▼
 Formula frontend   Factor frontend   Direct frontend
       └─────────────────┼─────────────────┘
                         ▼
                  Semantic Graph
                         │
        dependency / effect / lookback
                         │
                         ▼
                    ComputePlan
                         │
       ┌─────────────────┼─────────────────┐
       ▼                 ▼                 ▼
      Full              Range           Streaming
       └─────────────────┼─────────────────┘
                         ▼
                 UnifiedExecutor
                         │
           ┌─────────────┼─────────────┐
           ▼             ▼             ▼
        Kernel       BufferArena    StateArena
                         │
                         ▼
                    Artifact
                         │
          ┌──────────────┼──────────────┐
          ▼              ▼              ▼
       Python           Node          Native
```

---

# 15. FunctionSpec 必须成为真正 SSOT

现在已经有 registry/schema/ffi registry 等基础。

下一步应继续收敛成：

```rust
struct FunctionSpec {
    id: FunctionId,
    name: &'static str,
    aliases: &'static [&'static str],

    inputs: &'static [InputSpec],
    params: &'static [ParamSpec],
    outputs: &'static [OutputSpec],

    lookback: LookbackSpec,
    dependency: DependencyShape,

    batch: Capability,
    range: Capability,
    streaming: Capability,
    into: Capability,

    deterministic: bool,
    causal: bool,
    stateful: bool,

    bindings: BindingCapabilities,
}
```

然后由这一份 SSOT 生成/验证：

```text
Rust API metadata
Formula catalog
Plan kernel map
Python stub
Node types
C header
Java declarations
.NET declarations
Go wrapper
WASM surface
Docs tables
Numeric contracts
```

---

# 16. Formula / Factor / Operation Graph 收敛

当前概念较多：

```text
Formula AST
ComputeIR
Operation
FactorGraph
FactorPlan
ComputePlan
HotExecutionPlan
```

建议目标：

```text
Frontend AST
     ↓
SemanticGraph
     ↓
ExecutionPlan
```

`Formula`、`Factor`、`Feature`、`Composite` 只是：

```text
NodeKind
```

这样才能真正统一：

- dependency dedup；
- lookback；
- DirtyRange；
- scheduling；
- CSE；
- buffer allocation；
- metrics；
- artifact hash。

---

# 17. DirtyRange 合同继续类型化

建议：

```rust
enum DependencyShape {
    FixedLookback(usize),
    Expanding,
    Dynamic,
    CrossSectional,
    Global,
}
```

规则：

```text
FixedLookback
    -> range execution allowed

Dynamic / CrossSectional / Global / Unknown
    -> conservative full fallback
```

禁止由具体 kernel 私自判断局部重算安全性。

---

# 18. Streaming / Batch / Range 语义统一

目标不是：

```text
batch.rs 一套数学
streaming.rs 一套数学
range.rs 再一套数学
```

而是：

```text
Canonical Kernel Contract
        │
   ┌────┼────┐
   ▼    ▼    ▼
 Batch Stream Range
```

必须增加系统性 convergence gates：

```text
batch == streaming
full == range
allocating == into
tree == plan
Rust == FFI
```

---

# 19. RuntimeContext

将当前：

```text
BufferArena
StateArena
scratch
cache
metrics
diagnostics
timeout
```

统一为：

```rust
RuntimeContext
```

例如：

```rust
struct RuntimeContext {
    buffers: BufferArena,
    states: StateArena,
    cache: ArtifactCache,
    limits: ExecutionLimits,
    metrics: RuntimeMetrics,
    diagnostics: Diagnostics,
}
```

所有 executor 只接受同一个 context。

---

# 20. 性能优化顺序

不要优先追求“再多 50 个 SIMD 函数”。

优先级：

### 1. 避免重复计算

```text
DAG CSE
```

### 2. 避免重复分配

```text
_into
BufferArena
persistent output
```

### 3. 避免重复解析/编译

```text
compile once / execute many
```

### 4. 减少重算

```text
DirtyRange
```

### 5. 降低跨语言复制

```text
zero-copy
```

### 6. 最后才是

```text
SIMD / kernel fusion
```

---

# 21. Benchmark 必须从“函数”升级到“真实工作负载”

继续保留：

```text
RSI vs TA-Lib
SMA vs TA-Lib
MACD vs TA-Lib
```

同时新增：

### Scanner

```text
5000 symbols
20 indicators
50 formulas
```

### Factor DAG

```text
1000 symbols
100 factors
shared intermediates
```

### DirtyRange

```text
1M rows
dirty 1 / 10 / 100 rows
lookback 5 / 20 / 60 / 252
```

### Streaming

```text
10K symbols
one-bar update
```

### Cross-language

```text
Python -> Rust -> NumPy
Node -> Rust -> TypedArray
C ABI
JVM
.NET
WASM
```

报告必须区分：

```text
kernel time
binding time
allocation
end-to-end time
```

---

# 22. Release Readiness Aggregator

新增：

```text
.github/workflows/release-readiness.yml
```

对同一 commit SHA 聚合：

```text
CI
Docs Check
Python wheels
TA-Lib installed-wheel gate
Multilang cross-platform
Multilang release dry-run
Competitive benchmark
Security audit
Installer contract
```

只有：

```text
ALL_REQUIRED_CHECKS == SUCCESS
```

才能允许：

```text
tag vX.Y.Z
```

否则：

```text
NO RELEASE
```

---

# 23. 发布前最终验收矩阵

## Core

```text
[ ] cargo check --workspace --all-targets --locked
[ ] cargo test core
[ ] formula tree/plan parity
[ ] batch/streaming parity
[ ] full/range parity
[ ] NaN safety
[ ] no panic on public invalid input
[ ] no unbounded loops
```

## Architecture

```text
[ ] zero Rust source orphan
[ ] zero script orphan
[ ] zero doc orphan
[ ] zero public module orphan
[ ] no tracked build artifact
[ ] no unexplained dead-code allow
```

## Python

```text
[ ] generator idempotent
[ ] transformed source cargo check
[ ] linux wheel
[ ] Windows wheel
[ ] macOS x64 wheel
[ ] macOS ARM wheel
[ ] Python 3.8–3.14 ABI3 install tests
[ ] installed-wheel stub contract
[ ] TA-Lib installed-wheel parity
```

## Native / Other Languages

```text
[ ] C Rust binding tests
[ ] C numeric contract 201/201
[ ] C++ numeric contract 201/201
[ ] Node Linux / Windows / macOS
[ ] .NET Linux / Windows / macOS
[ ] Java
[ ] Go
[ ] iOS
[ ] Android
[ ] WASM
[ ] AArch64 warning contract
```

## Release

```text
[ ] version contract
[ ] package metadata
[ ] source crate
[ ] native archives
[ ] installers
[ ] checksums
[ ] same-SHA readiness PASS
```

---

# 24. 推荐执行顺序

## Batch 0 — 先恢复全绿

一次性修：

1. `cargo fmt`；
2. `factor-analysis` Euler constant Clippy；
3. `avx2_fma_available` cfg；
4. Python `u16` / `"var"` transformation；
5. C/C++ request JSON generation；
6. rerun all failing workflows。

这一批不要混入架构大改。

验收：

```text
same SHA
CI green
Python wheels green
TA-Lib installed-wheel green
Multilang cross-platform green
Multilang release green
```

---

## Batch 1 — 孤儿与仓库治理

1. 删除/正式接入 `circuit_breaker.rs`；
2. 新增 `check_rust_source_reachability.py`；
3. 清理 `core/target/criterion`；
4. 新增 build-artifact gate；
5. 新增 unbounded-loop gate。

---

## Batch 2 — 生成系统收敛

把：

```text
prepare_python_hot_bindings
apply_architecture...
repair...
optimize...
sync...
```

逐渐替换为：

```text
BindingSpec
   ↓
single generator
   ↓
generated source
```

目标：

> build 不再原地修改 handwritten tracked Rust source。

---

## Batch 3 — Runtime 收敛

1. FunctionSpec SSOT；
2. SemanticGraph；
3. ComputePlan；
4. UnifiedRuntime；
5. RuntimeContext；
6. Factor / Formula / Composite 共用 DAG。

---

## Batch 4 — 性能

1. CSE；
2. allocation；
3. DirtyRange；
4. scheduler；
5. kernel fusion；
6. workload benchmarks。

---

# 24a. Batch 3 / Batch 4 实现状态

> 本节是实现落地后追加的状态标注，不修改上面的原始计划文本。每条给出落点与
> **持有它的门禁**——没有门禁的「已实现」等于没实现，下一轮就会悄悄回退。

## Batch 3 — Runtime 收敛

| # | 项 | 落点 | 持有它的门禁 |
|---|---|---|---|
| 1 | FunctionSpec SSOT | `registry::FunctionSpec::dependency()` / `allows_range_execution()`；`ComputeCapabilities::from_function_spec` 改为回读该访问器而非第二次推导 | `every_function_derives_its_dependency_shape_from_its_lookback`（走遍全注册表，双向非空断言）、`registry_dependency_matches_the_plan_node_capability`、`only_a_constant_lookback_unlocks_range_execution` |
| 2 | SemanticGraph | 新增 `core/src/semantic_graph.rs`：`NodeKind` / `SemanticNode` / `SemanticGraph` / `SemanticGraphBuilder`；生产入口 `UnifiedRuntime::compile_semantic_graph()` + `GraphOptimization` / `CompiledSemanticGraph` / `GraphPlanError` | `every_frontend_kind_lowers_to_the_same_plan_for_the_same_semantics`、`graph_validates_unknown_operands_and_cycles`、`the_production_graph_entry_point_matches_the_direct_route`（同时断言生产路由确实少跑 kernel，防止"只报告不优化"） |
| 3 | ComputePlan | 既有 `core/src/compute.rs`，现由 `SemanticGraph::lower()` 唯一到达；图的校验直接复用 `ComputePlan::compile`，全局只有一个 DAG 校验器 | `an_empty_graph_is_valid_and_has_a_stable_identity`、上项 |
| 4 | UnifiedRuntime | 既有 `UnifiedRuntime::*_with_context`；`DependencyShape::combine_all` 收为唯一的依赖合并规则（原先 `unified_runtime` 内有一份私有副本） | `combined_dependency_is_conservative`、`range_execution_is_refused_without_a_proven_lookback` |
| 5 | RuntimeContext | `core/src/runtime_context.rs`；`UnifiedExecutor` 拥有一个，全部执行入口经它记账 | `context_separates_full_and_range_executions`、`context_classifies_every_entry_point_and_counts_kernels`、`context_budget_rejects_plan_before_allocating` |
| 6 | Factor / Formula / Composite 共用 DAG | §16 的四类 `NodeKind` 是**同一张图上的标签**，不是四种图 | `every_frontend_kind_lowers_to_the_same_plan_for_the_same_semantics`（同时断言 lowering 结果与 `content_hash` 都一致——身份里不含 `NodeKind`，否则公式与因子算同一个序列却无法共享缓存） |

### 第 6 项的真实完成度（不要过度声称）

四类 `NodeKind` 确实收敛到同一张图、同一个 `lower()`、同一个执行器，**但公式与因子
两条前端目前仍各自直接 lower 自己的 plan**，尚未改走 `SemanticGraph`：

- 公式：`FormulaComputePlan::compile` 直接把 lowerer 的节点交给 `ComputePlan::compile`。
- 因子：`FactorPlan` 走自己的 `execute_borrowed`。

所以 §16 的"一个图"目前在**图这一层**成立，在**前端接入这一层**只完成了一半。本轮
补上的是生产入口（`UnifiedRuntime::compile_semantic_graph`），使 CSE 从"只有测试和
基准能拿到"变成"任何调用方都能拿到"；把两条前端迁到图上是一次独立的、必须逐条过
`formula_plan_differential` 的正数等价门的重构，未在本轮做。

`check_orphan_modules.py` 正是抓出这件事的门禁——它把 `semantic_graph` 标为
`test-only`（生产零引用），门禁原话是 "wire it to a caller or delete it, or record
it deliberately"。本轮选择了 wire，而不是记进 KNOWN 白名单。

## Batch 4 — 性能

| # | 项 | 落点 | 实测 | 持有它的门禁 |
|---|---|---|---|---|
| 1 | CSE | `SemanticGraph::eliminate_common_subexpressions` + `CseReport` | Factor DAG 1000 symbols：**201 节点 → 111**（合并 90），**44.7ms → 10.6ms（4.2×）** | `cse_preserves_every_value_it_folds_away`（执行层逐 bar 数值等价 + kernel 调用数必须下降，防止「等价但无用」的空转）、`cse_folds_duplicated_pure_intermediates`、`cse_is_idempotent_and_preserves_validation`、`cse_refuses_impure_stateful_and_nondeterministic_nodes`、`cse_keeps_different_lookbacks_apart` |
| 2 | allocation | 新增 `UnifiedExecutor::execute_into`（§20.2 的 persistent output） | 见下 | `repeated_execution_reuses_the_arena_instead_of_allocating`（双向：`execute` 每次恰好重分配 1 个结果缓冲，`execute_into` 预热后 **0**）、`execute_into_rejects_mismatched_destinations`、`into_kernel_equals_allocating_kernel`（22 例） |
| 3 | DirtyRange | 既有 typed `DependencyShape` 合同；本轮补上「赢了多少」的度量 | 1M 行 / lookback 252：全量 **8.85ms** vs 100 行脏区 **1.17µs** | `factor_dirty_range_splice_equals_full_execution`（拼接 == 全量重算，且 `recomputed_rows < rows`） |
| 4 | scheduler | `SemanticGraph::levels()` / `scheduled_order()` | 分组，不重排 | `levels_are_a_valid_topological_schedule`、`scheduling_never_reorders_observable_effects`（可观察副作用的相对顺序必须原样保留） |
| 5 | kernel fusion | **未实现** | — | — |
| 6 | workload benchmarks | 新增 `core/benches/workload_bench.rs`（Scanner / Factor DAG / DirtyRange / Streaming，即 §21 的四项尺寸） | Scanner 5000×260 bars×90 kernels：**91.5ms**（14.2M symbol-bars/s）；Streaming 10K symbols×3 指标：**8.64ms**（3.47M updates/s） | 基准自带前置断言：Factor DAG 的 naive 与 CSE 两张图必须先在小universe上逐 bar 数值一致，否则基准本身就是假的 |

### 关于第 5 项（kernel fusion）

**没有实现，不打算谎报。** §20 本身把它排在最后，理由是收益必须建立在「已经
不重复计算、不重复分配」之上；本轮的实测增量全部来自第 1、2 项。在没有
profiling 指的明确融合目标之前做融合，只会得到一版更难维护、也没有人能量化
收益的 kernel。留待有具体热点时再做。

### 顺带发现并修掉的真实缺陷

`§18` 的 `batch == streaming` 门禁一上线就抓到 **6 个流式内核与批量内核算的不是
同一个指标**——ADX / DX / +DI / −DI 用 `EMA` 平滑而批量用 Wilder 平滑，
TRANGE / ATR 在 bar 0 用 `high - low` 而批量按 TA-Lib 约定留 `NaN` 并把整个
预热窗口前移一根。这 6 个都在既有流式测试「值落在合理区间」的判据下长期为绿。
全部按批量语义修正，**没有一条进 allowlist**。

另外把 `core/tests/TEST_INDEX.md` 自己的契约做成了门禁
（`core/tests/test_index_contract.rs`）：该文件声称「清单必须与 `ls core/tests/*.rs`
一致」但从来没有人检查，写门禁时已经漏了 8 个 target、并且指向 2 个不带目录的
`common/` 助手。

### 全量测试与其余门禁抓到的真实缺陷

§18 那条边修完之后，把「跑全 suite」当验收（而不是只跑相关模块）又抓到四件事——
它们都不是回归，是**一直就错着**的东西：

1. **`test_adosc_into_matches_allocating_path` 是 NaN 不安全的容差比较**。
   `(NaN - NaN).abs() <= 1e-12` 恒为 `false`，所以这条断言在输出含预热 NaN 时
   **永远不可能通过**。基线能绿只是因为旧 SIMD 版 `adosc` 每行都填值；当 ADOSC
   被修成 TA-Lib 正确的预热 NaN 之后它就炸了。已改为「两边都是 NaN 才算一致」。
   同类模式全树扫过：12 处跨路径容差比较，10 处已 NaN-aware，余下 2 处
   （WCLPRICE / AD）本就无预热洞，安全。
2. **`test_atr_builder_ok` 钉的 14 是旧 bug 值**。ATR 修正后 `warm_up_period()`
   合法地变成 `period + 1 = 15`（与 registry 的 `convergence: 15` 一致）。已改成
   同时断言 registry 值：**字面量抓 registry 漂移，registry 抓内核退回旧行为**。
3. **`test_ht_sine_throughput` 是脆弱计时断言**。阈值 200 ns/bar，孤立实测 ~170
   （仅 15% 余量），在 3000 条测试并行时随机翻红。按其自身注释"generous upper
   bound"放宽到 1000（≈6×）。`cycle.rs` 本轮未被触碰，与 ATR 改动无关。
   一个会在争用下翻红的门禁比没有门禁更糟——它训练人去重跑而不是去查。
4. **§18 的 roster 原本只校验 artifact 文件存在**，于是「改名掉
   `streaming_agrees_with_batch`」不会失败：文件还在，边已经没人守，而 roster
   仍然给它发合格证。已改为校验**符号**（`fn <symbol>(`），`tree == plan` 与
   `Rust == FFI` 两条边也补上了真实符号名，roster 这才真的双向。

`check_orphan_modules.py` 则是另一个独立发现：`semantic_graph` 的生产引用数为
**0**（`prod=0 test=1`），即 §16 的图层当时只有测试够得着。已补生产入口（见 Batch 3
第 2 项）。

---

# 25. 三轮审计结果汇总

## Round 1

发现：

- 1 个真实 Rust 源码孤儿：`core/src/circuit_breaker.rs`；
- 约 337 个 tracked Criterion build artifacts；
- 主要 runtime/FFI/UI 链主体存在。

结论：

> 架构不是断裂状态，但仓库仍有“未进入 module graph 的实现”和生成产物污染。

## Round 2

发现：

- 当前没有新的明显无限循环；
- NaN panic class 已经建立永久 gate；
- Format 当前实际红；
- Workspace Clippy 当前实际红；
- moving stable 造成 lint policy 不稳定。

结论：

> 核心算法测试主体较稳定，但工程门禁并没有达到发布级全绿。

## Round 3

发现：

- Python 四平台 wheel 全红；
- 原因是 build-time source transform 漏改语义；
- macOS Node/.NET + AArch64 同根 cfg defect；
- C/C++ numeric contract request JSON 缺字符串 quoting；
- 同一 SHA 多个发布 workflow 仍红。

结论：

> 真正的剩余断链主要集中在“生成态 / 跨平台 / 安装态”，不是普通 Rust core。

---

# 26. 最终建议

Finkit 当前不需要继续横向堆能力。

近期最重要的工程目标应该变为：

> **任何功能只有在 Source → Generated State → Runtime → FFI → Package → Installed Artifact → Cross-platform Matrix 全部证明后，才算“实现完成”。**

以后对“完成”的定义统一成：

```text
implemented
    ↓
module reachable
    ↓
contracted
    ↓
tested
    ↓
cross-path parity
    ↓
cross-platform validated
    ↓
packaged
    ↓
installed-consumer tested
    ↓
published
```

而不是：

```text
文件存在 == 完成
能 cargo check == 完成
binding 源码存在 == 完成
```

这次三轮审计暴露出的 Python transformation、`circuit_breaker.rs` 和 C/C++ JSON contract，恰好证明必须采用这个更严格的完成定义。

---

# 27. 当前项目状态一句话

Finkit 已经从“技术指标库”发展为真正的 **Rust 原生量化计算运行时雏形**；当前最大瓶颈已经不再是“功能缺失”，而是：

> **让单一语义在所有执行模式、生成态、多语言、CPU 架构和最终安装包中保持同一份真实合同。**

先把上述 P0 全部收敛到同一 SHA 全绿，再进入 V4 runtime 架构收敛，是当前风险最低、收益最高的推进路径。

---

# 28. 第三轮发布前审计（孤儿逻辑 / 断链 / 死循环）

前三轮（Batch 2–4）完成后的独立复查。这一轮不再寻找「功能缺失」，而是找
**绿着的假象**：门禁看不见的盲区、没有调用方的实现、注释声称保证终止但实际
不保证的循环。全部 8 项都是真实缺陷，全部修复，无一项进 allowlist。

## 28.1 门禁对「未跟踪文件」完全不可见（严重）

`check_orphan_docs.py` / `check_orphan_scripts.py` 都基于 `git ls-files`。这在
CI 上是对的（checkout 永远是干净的），但在**本地**它是盲区：新写的文档、脚本
只要没 `git add` 就不在门禁视野里，于是门禁全绿而提交后 CI 立刻变红。

把改动 stage 后重跑，`check_orphan_docs.py` 立刻报出 **8 个不可达文档**：

| 文档 | 处置 |
|---|---|
| `docs/new.md` | 这是 v3.1 计划，文件名无信息量；`git mv` 到 `docs/archive/finkit-architecture-v3.1-implementation-plan.md` 并加入归档索引 |
| `docs/FINKIT_ARCHITECTURE_AND_OPTIMIZATION_PLAN_V4.md` | 本轮活跃计划，加入 `docs/README.md` 的 "Current refactor baseline" |
| `docs/archive/ALPHATA_VS_TALIB*.md`（3 个） | 项目改名前的对比报告，加入 `docs/archive/README.md` 索引 |
| `docs/archive/IMPROVEMENT_PLAN.md` | 同上 |
| `docs/archive/benchmark-{baseline,results}.md` | 同上 |

归档索引同时补了一句说明：`docs/archive/README.md` 里「三个 V4 文档已于
2026-09-21 删除」与 `docs/FINKIT_ARCHITECTURE_AND_OPTIMIZATION_PLAN_V4.md`
**名字相同但不是一回事**——前者指的是已删除的 `runtime_engine.rs` 架构计划。
不写清楚，读者会把当前唯一基线当成已废弃文档。

## 28.2 `workflow_run` 是一条没有任何门禁保护的断链

`release-readiness.yml` 是本仓库的**发布准入**（"same SHA, every required
workflow green"）。它通过 `on.workflow_run.workflows` 等待 8 个工作流，而 GitHub
是拿**工作流显示名**匹配的——名字里有一个字母写错，这个触发器就永远不会触发，
而 `check_workflow_liveness.py` 只会看到它还挂着 `workflow_dispatch`，报告
"OK: every workflow has a trigger that can fire"。

同一份「必需工作流清单」还维护在 `release_readiness_aggregate.py` 的
`REQUIRED_WORKFLOWS` 里，但用的是**文件名**。两处独立维护、无任何一致性校验。

修复（都在 `check_workflow_liveness.py` 内，不新增脚本）：

1. 解析每个工作流的顶层 `name:`，校验 `workflow_run.workflows` 的每一项都能
   解析到某一个 `name:`——解析不到即失败（"waits on `X`"）。
2. 校验 yml 的等待清单与聚合脚本的验证清单**双向一致**：等待但未验证 = 该
   workflow 从未被真正判定；验证但未等待 = 聚合跑在它完成之前。
3. 顺带修掉一个**假阴性**：`workflow_run` 本是自动触发器，但门禁的
   `UNCONDITIONAL` 集合里没有它，于是 `release-readiness.yml` 被报成
   "no automatic trigger; fires only via workflow_dispatch"（误导），而一个**只**
   依赖 `workflow_run` 的工作流会被直接判为不可达。

门禁做过注入验证：把 `- CI` 改成 `- CI-INJECTED-TYPO` 后，dangling 与双向漂移
同时报出、exit=1；还原后恢复全绿。写一个永远不会红的门禁比不写更糟。

## 28.3 7.3 MB 构建产物被跟踪，而门禁声称"没有构建产物"

`ci.yml` 的 WebGL job 会现场执行 `cargo run --example gpu_large_chart` 把
`gpu_large_chart.html`（**7 MB**）写到仓库根目录，紧接着用
`scripts/test_webgl_runtime.mjs` 消费它。`--example improved_chart` 另外写出 5 个
兄弟文件。这 6 个文件**全部被提交**。

它们既不在 `target/`、`dist/` 路径下，也没有二进制扩展名，所以
`check_no_tracked_build_artifacts.py`（本轮新增的门禁）报告
"1305 tracked files, none look like build output"——而当时树里躺着 7.3 MB 的
构建输出。门禁的动机段写着它要防的是"污染源码树、发布归档和每一次 `git diff`"，
这 6 个文件正是如此。

修复：`.gitignore` 加根目录生成物规则、`git rm --cached` 6 个文件（工作树保留）、
门禁新增「仓库根目录的 `.html/.svg/.json/.csv`」规则。规则做成**根目录专属**而非
全局扩展名，因为 `docs/`、`tests/` 下的这些格式是合法内容。同样做了注入验证：
`git add -f improved_chart.svg` 后门禁 exit=1，撤销后恢复绿。

## 28.4 `ArtifactCache` 是孤儿：计划要求它被共享，实际零生产调用方

`RuntimeContext::ArtifactCache` 的文档写着它是"the one cache the runtime context
owns"，要取代 formula / factor / composite / research 各自生长的缓存。而
`grep` 的结果是：**除自身测试外零调用方**。计划 §16 状态表里也写明
`content_hash` 的存在理由是"否则公式与因子算同一个序列却无法共享缓存"——缓存
与图身份都实现了，但两者从未接在一起。

这属于必须消除的孤儿逻辑。修复选择**接线**而不是删除：

- 新增 `UnifiedRuntime::compile_semantic_graph_cached`，把
  `CompiledPlanArtifact`（plan + CSE 报告 + 依赖形状，**纯数据**）存进调用方传入
  的 `RuntimeContext`。
- **缓存的是 plan，不是 executor。** executor 持有 buffer arena 与持久内核状态，
  共享它会让扫描的第二个 symbol 覆盖第一个的工作集。
- 键 = 图的内容身份 + 优化设置所在的 namespace（`FINPLAN0` / `FINPLAN1`）。
  不先跑 CSE 去算「优化后图的身份」——那正是缓存要避免的开销。
- 编译失败不入缓存，避免修正后的图被先前的失败"毒化"。

门禁：`recompiling_the_same_declaration_is_served_from_the_artifact_cache` 断言
**命中计数器**（1 hit / 1 miss / 1 entry），而不是"两次都成功"——
一个没人写入的缓存与一个工作正常的缓存，在只看返回值时完全一样。

## 28.5 `execute_into` 失败会污染持久内核状态

`execute_into` 的文档承诺"destinations are validated before any is written, so a
mismatch is a clean rejection rather than a half-updated result"。但实现是
**先 `execute()`、后校验长度**：状态推进（stateful 内核写自己的 arena slot）
已经发生，调用方才拿到 `OutputLength`。修正目标长度后重试，得到的是**第二次**
执行的数值，而调用方会把它当作第一次。

逻辑长度在执行前就是确定的（`execute` 取首个输入的长度，`run` 用同一 extent
从 arena 取缓冲），所以长度校验可以前移。现在 count 与 length 都在**运行前**
校验，被拒绝的调用零副作用。

门禁：`a_rejected_execute_into_leaves_the_executor_untouched` 通过
`RuntimeMetrics`（只在 `run` 的成功路径上递增）证明 `executions() == 0`、
`kernel_calls == 0`，然后再用修正后的目标执行一次，断言拿到的是**第一次**的
结果。

## 28.6 Cohen–Sutherland 裁剪的终止性只有注释在保证

`visualization/src/geometry.rs::ClipRect::clip_line` 的注释写着"每次迭代要么
返回、要么裁掉一个外部区域，而外部区域只有四个，所以不会无限循环"。这个论证
**在浮点下不成立**：相交点用浮点算出，舍入可能让新坐标停在边界外侧一点点，
该 outcode 位存活，无界 `loop` 就会对同一个端点反复裁剪。而
`check_unbounded_loops.py` 只要求「注释 + break + 预算词」，这里连 `break` 都
没有，全靠注释通过。

修复：改成显式 `for _ in 0..4`。四次迭代对精确算术足够（四个 outcode 位），
四轮后仍有残差则判定为不可裁剪并返回 `None`——**拒绝绘制严格优于输出一个仍在
矩形外的点**。

## 28.7 Pine 解析器可被深嵌套输入打爆栈

`parse_pine` 走 pest 语法（递归下降）并递归遍历 pair 树，C 栈用量随源码嵌套深度
增长。它是**用户输入**的直接入口（CLI、Python 绑定、HTTP 层都接受 Pine 源码），
而入口处没有任何长度或深度限制——两万层 `(` 会让进程栈溢出 abort，而不是返回
`PineError`。这是崩溃，不是诊断。

修复：入口处加一次**迭代式、词法感知**的预算检查（`MAX_NESTING_DEPTH = 128`、
`MAX_SOURCE_BYTES = 1 MiB`）。预检本身绝不递归，否则它自己就会带上它要防的
故障；字符串字面量与 `//` 注释被跳过，闭合括号不会让计数器下溢。

真实脚本嵌套不超过十几层，所以这个预算只会拒绝本来就解析不了的输入。5 个测试
覆盖：正常嵌套通过、5000 层被拒且错误信息给出上限、字符串/注释里的括号不计、
不配对闭括号不 panic、超长源码在执行前被拒。

## 28.8 复查未发现问题的部分

- **死循环**：全树裸 `loop{}` / `while true{}` 共 10 处，逐个人工核验；
  其余 9 处都有真实预算（`formula/executor.rs` 的 `WhileLoop` 用
  `MAX_LOOP_ITERATIONS`、`formula/stateful.rs` 用 `MAX_LOOP_ITERATIONS`、
  `cv_split.rs` 的 `combinations` 在内层扫描耗尽即 return、
  `harmonic.rs` 的 pivot 合并每轮恰消费一个 pivot）。
- **环检测**：`ComputePlan::compile` 用 Kahn 算法（indegree + 队列），
  `execution_order.len() != by_id.len()` 时报 `DependencyCycle`，不会在环上打转。
- **递归**：真实自递归只有 AST 遍历（formula/pine/optimizer/composite 的
  `visit` / `parse_expr_node` / `eval_expr` 等）。除 28.7 的入口预算外，
  用户自定义函数展开另有独立的 `MAX_EXPANSION_DEPTH` 守卫。
- **文档→代码符号**：`finkit::module[::Item]` 形式的引用全部可解析。
  `docs/refactor-plan-2026-09-21.md` 里的 `finkit::backtest_evaluation::...`
  是**历史叙述**（记录该模块已删），不是断链。
- **FFI 契约**：`sync_bindings.py --check --all` 漂移为 none（Python 71/78、
  Node 76/78 在活跃 tier；其余 6 语言按设计 DEFERRED，由各自的跨平台工作流
  以编译+集成测覆盖）；C 头 78 签名 / 99 导出 / 96 声明全部匹配。

## 28.9 这一轮的方法论

前两轮修的是"实现不对"，这一轮修的是"**看起来对**"：门禁绿、注释写了理由、
文档声称有保证、API 存在——而实际没有调用方、没有生效、或没有真的保证。

三类缺陷各有其固定检查手法，值得固化：

1. **孤儿** → 不看"文件在不在"，看"谁调用它"。第二次 grep 必须覆盖
   `core/tests`、`factor-analysis`、`benches`、`ffi/*`，否则同文件内的调用会被
   自己的 `grep -v` 过滤掉而误判。
2. **盲区** → 找出门禁的输入范围（这里是 `git ls-files`），然后**制造它在范围
   之外的状态**（把文件 stage 起来）再跑一次。
3. **注释式保证** → 逐条核验注释里的论证，而不是核验注释的存在。
   28.6 的论证在整数域成立、在浮点域不成立，正是这类缺陷的典型。

每一个修复都配一个**能失败的**门禁，并用注入法验证它真的会失败。

# 29. 第四轮发布前审计（复审"修复动作本身"）

前三轮把"看起来对"的东西挑了一遍。第四轮换了个提问角度：**前三轮那 81 项改动、
2 万行删除，本身就是新的孤儿与断链来源**。一个删掉两万行的重构，和被它删掉的代码
一样需要被审计。

结论是 **3 项缺陷，外加 1 个新门禁**；其中 1 项在审计开始时**距离被当成"修复"提交
只剩一条命令**。

## 29.1 `indicators::volume::adosc_into` 被删，而唯一发现它的门禁正准备掩盖它（严重）

去重本身是对的：`indicators::volume` 里那份融合式 ADOSC 与
`math::volume_kernels::adosc_into` 是同一算法的两份实现，删掉重复、让分配式路径改调
规范内核，都应当保留。错在**公开名字没有接回去**。

于是 `finkit::indicators::adosc_into` 这个公开入口凭空消失，而所有门禁全绿：

- `cargo check --workspace --all-targets` **看不见"被删除的 `pub fn`"**——它只检查
  现有引用能否解析，而 crate 内部没有任何调用方（所有内部调用都直连 `math::` 内核）。
  "没有调用方"被当成了"不需要存在"。
- `core/src/indicators/volume.rs` 里那个名为
  `test_adosc_into_matches_allocating_path` 的单元测试**被改成直接调内核**
  （`crate::math::volume_kernels::adosc_into`）。测试名说的是公开入口，实际测的是它
  的实现体，于是这个"看起来覆盖了公开 API"的测试成了掩盖者。
- 唯一发出声音的是 `gen_ssot_docs.py --check`：生成态快照里 `adosc_into` 消失
  （公开函数计数 389 → 388）。

**最危险的地方在于门禁给出的修法是错的。** 它的提示是
`Run: python scripts/gen_ssot_docs.py --generate`；照做确实会让门禁变绿，但那是把一次
真实的 API 破坏**写成新的基线**。快照型门禁记录的是"现在是什么"，不是"应该是什么"，
因此它有能力发现漂移，却**没有立场判断漂移的方向**。差一步就把破坏当成修复提交了。

修复：把 `adosc_into` 作为**校验 + 转发**的包装补回 `volume.rs`（与 `ad_into`/
`obv_into` 同构），委托给规范内核。附带收益是它继承了内核更严格的前置条件——零周期
现在会被拒绝，序列长度不足时报 `TaError::InsufficientData`，而不是越过预热区继续读。

## 29.2 补的是"这一类"，不只是"这一个"

只修 `adosc_into` 等于赌下一次性重构不会再犯。新增 target
`core/tests/indicator_api_surface.rs`，把公开 API 面变成**可失败的规格**：

- 逐条钉住 53 个 `finkit::indicators::<模块>::<名字>_into` 路径；
- 单独钉住 `core/src/indicators/mod.rs` 的根级重导出接缝——`pub use volume::*` 与
  显式 `pub use math::volume_kernels::{ad, adosc, obv}` 并存处，**显式条目会遮蔽同名
  glob 成员**，这正是 `adosc_into` 漏掉的位置（`indicators::ad`/`adosc` 解析到内核，
  `indicators::volume::ad` 解析到模块包装，两条路径都公开、都在用，所以都要钉）；
- 实现方式为 `let _ = <path>;`，只强制名字解析、不做函数指针强制转换，因此条目**无需
  写签名**，参数列表变化也不会让守卫失效；
- 方向是**单向的**：只约束删除与改名，不管新增。新增入口不需要改这个文件，守卫就不会
  退化成"没人愿意维护的清单"而被删掉。

删除/改名任一被钉住的名字 → 本 target **编译失败**，使移除成为一次刻意的、可评审的
决定。已用注入法验证：把 `volume::adosc_into` 改成 `adosc_into_RENAMED` 得
`error[E0425]: cannot find value adosc_into_RENAMED in module finkit::indicators::volume`，
随后还原。

同时把 `volume.rs` 里那个"名不副实"的单元测试改回调用公开包装
（`adosc_into(...)`，即 `volume::adosc_into`），并在注释里写明为什么不能改回直调内核。

## 29.3 两处 `#[allow(clippy::uninit_vec)]` 活过了它们的理由

同一次去重把 `indicators::volume::ad` 与 `obv` 的实现从
`Vec::with_capacity` + `unsafe { set_len }` 换成了 `Array1::zeros` /
`vec![0.0; len]`，却把 `#[allow(clippy::uninit_vec)]` 留在原地。这是**安全相关**的豁免
挂在已经不做那件事的代码上：它向读者声明"此处有意使用未初始化内存"，而事实上没有。
更糟的是下一位作者很可能把 `ad` 当模板照抄，顺手继承这份"许可"。

全工作区扫描 6 处 `uninit_vec` 豁免（其余 4 处在 `momentum.rs:1916`、
`moving_avg.rs:450/781/1115`，均真实调用 `set_len`，保留），删掉
`volume.rs` 的 2 处。

**把这"一类"也钉住。** 保留的那 4 处由 `#[allow]` 改为
`#[expect(clippy::uninit_vec)]`，并在 `ci.yml` 的 clippy job 上 deny
`unfulfilled_lint_expectations`。区别是本质性的：

- `#[allow]` 是**单向**的——它只能关掉检查，无法表达"我预期这里会触发该 lint"。
  于是它的理由消失后，豁免会**永远静默地继续生效**，正是 29.3 这个缺陷的成因。
- `#[expect]` 是**双向**的——lint 不触发时，编译器报
  `unfulfilled_lint_expectations`（该 lint 默认 warn，本例中显式 deny 成 error）。
  也就是说，"豁免的理由已经不存在了"这件事由**编译器**发现，不需要人再写一个
  Python 门禁。

两个方向都实测过：

- 4 处合法豁免在 clippy 下**被满足**（无 `unfulfilled` 告警）；
- 在 `RUSTFLAGS="-D unfulfilled_lint_expectations"` 下
  `cargo clippy --workspace --all-targets --locked` 全绿（exit 0）；
- 注入法：把该 `expect` 挂到已经不调用 `set_len` 的 `volume::obv` 上，clippy 报
  `warning: this lint expectation is unfulfilled` 并指向该属性行，随后还原。

一个必须验证而不该假设的细节：`RUSTFLAGS="-D warnings"` 的
`workspace-check`/`test` 等 job 跑的是 `cargo check`/`cargo test`，**不加载 clippy**。
工具 lint 在工具缺席时不被求值，因此 `#[expect(clippy::*)]` 在那些 job 下**不会**
被误判为"未满足"——已用 `RUSTFLAGS="-D warnings" cargo check -p finkit` 实测为 0
error 确认。这正是"把 `#[expect]` 用在工具 lint 上"最容易踩的坑。

## 29.4 SSOT 生成器每次运行都用 CRLF 覆写输出

`Path.write_text` 默认把 `\n` 转成平台分隔符，而 `.gitattributes` 把 `*.md` 钉为
`eol=lf`。于是每次 `--generate` 之后，7 个生成文件的**差异为空、`git status` 却显示
已修改**。一个永远脏的门禁目录，教出来的习惯就是不再看 `git status`。改为
`newline="\n"` 后工作树恢复干净。

补充说明（本地环境现象，非仓库缺陷）：本地 `core.autocrlf=true` 与该
`.gitattributes` 冗余冲突时，会出现"工作树与 blob 逐字节相同、`git diff --quiet`
退出 0，但 `git status` 报 M"的索引假阳性；`git add` 后暂存区零差异、幻影标记消失。
`.gitattributes` 已经完整覆盖行尾规则，本地可以把 `core.autocrlf` 设为 `false`。

## 29.5 复查未发现问题的部分

- **全工作区公开 API 面差分（本节最强的一条证据）**：对
  `core/src`、`visualization/src`、`factor-analysis/src`、`cli/src`、`wasm/src`、`ffi`
  逐文件提取 `pub fn/struct/enum/trait/type/const/static` 名称集合，比对上一版
  `3fb8b3b` → 本次发布 `87d5991`：
  - **存活文件中被移除的公开项 = 0**（`adosc_into` 恢复后归零）；
  - 新增公开项 119 个（全部来自本轮的 `DependencyShape` / `RuntimeContext` /
    `ArtifactCache` 等新增面）；
  - 整体删除的文件只有 3 个，且**全部核实为从未参与编译的孤儿**（见下）。
- **三个被删文件逐一核实**（"文件被删"不等于"API 被删"，必须区分）：
  - `core/src/indicators/compat.rs`：`3fb8b3b` 的 `mod.rs` 里没有 `pub mod compat;`，
    也从未出现在生成态文档的模块列表中；其内容
    `crate::math::cci::cci(...).map(Array1::from_vec)` 与 `math/cci.rs` 早已返回
    `Array1<f64>` 的签名不符，**根本编译不过**。
  - `core/src/formula/range_zero_copy.rs`：`formula/mod.rs` 里没有声明；真正的实现是
    `formula/engine.rs:1187` 的 `eval_range_zero_copy_inputs`（被
    `core/tests/talib_semantic_contract.rs` 与 `ffi/python-binding/src/formula_plan.rs`
    调用），孤儿文件里那份是**重复实现**。
  - `core/src/circuit_breaker.rs`：`check_rust_source_reachability.py` 的 docstring 里
    已记录该案例 —— 完整、有文档、有测试，但从未进入模块图，因此从未编入任何产物。
  - 三者的共同点：**不在 `rust_source_reachability_allowlist.json` 里**，因为该 allowlist
    针对"已声明但不可达"，而它们是"从未被声明"——这是两个不同的类别。
- **`cargo check --workspace --all-targets --locked`**：0 error。
- **`cargo test -p finkit --lib`**：3028 passed / 0 failed；`-- volume`：150 passed。
- **`cargo test -p finkit --test runtime_convergence`**：17 passed / 0 failed，含
  `into_kernel_equals_allocating_kernel`（"五条边"中的 `allocating == into`）。
- **`cargo fmt --all -- --check`**：干净。
- **clippy（严格）**：`RUSTFLAGS="-D unfulfilled_lint_expectations"
  cargo clippy --workspace --all-targets --locked` → exit 0（1m34s），四处
  `#[expect(clippy::uninit_vec)]` 全部被满足。
- **`python scripts/gen_ssot_docs.py --check`**：通过（且
  `git diff --numstat docs/generated/` 为空——恢复公开面后快照重新自洽，本轮**没有**
  产生任何生成态改动）。

## 29.6 第四轮的方法论

第三轮的三问（谁调用它 / 门禁的输入范围是什么 / 注释里的论证成立吗）依然有效，但这一轮
补了两条更针对"治理动作"的问法：

1. **审计"修复动作本身"**。一次删掉两万行的重构，和被它删除的代码一样需要被审计。
   孤儿不会只存在于被留下的代码里，也会存在于"被删掉的东西留下的窟窿"里。
2. **区分"规格"与"快照"**。快照型门禁（生成态文档、覆盖率数值）能发现漂移，但**不能
   判断漂移的方向**——它红的时候，正确反应是"去查漂移的原因"，而不是"按提示重新生成"。
   凡是门禁提示"运行 --generate 即可"，都要先回答"为什么它变了"。
   真正的规格必须是**独立于当前源码**的，29.2 的编译期守卫就是这样一个规格。

补充一条判读经验：**"没有调用方"不是"不需要存在"的证据**。下游绑定与仓外使用者也是
调用方，而它们对 `cargo check` 不可见。

# 30. 第五轮发布前审计（前端 CI 覆盖 / 接口文档 / 死循环）

第四轮修的是"自己这轮改动引入的孤儿"。第五轮换了提问角度：把"前端贯通"和"接口
文档准确"这两个前四轮从未系统打开的维度，重新跑一遍三问（主体联通、无断链、无孤儿、
无死循环、前后端贯通）。

## 30.1 CI 触发路径覆盖缺口（真实断链）

`multilang-release.yml` 与 `multilang-cross-platform.yml` 的 `pull_request.paths` 是手工
维护的列表，但漏了 `visualization/**` 与 `factor-analysis/**` —— 这两个目录被 6 个 crate
依赖（含 `wasm` 浏览器前端，它依赖 `visualization`）。`wasm32` 构建只存在于
`multilang-release.yml`，于是"改 visualization 的 API → PR 上该 workflow 完全不触发 →
wasm32 与三个语言绑定零验证 → 只在发版打 tag 时才炸"。`cli/**` 在 `multilang-release.yml`
与 `python-wheels.yml` 同样缺失。

修复：三个 workflow 的 paths 补齐 `cli/**`、`factor-analysis/**`、`visualization/**`。

## 30.2 新门禁：check_workflow_path_coverage.py

手工维护的 path 列表必然漂移，所以配套的不能是"记得加"，而是一个会失败的门禁：对每个带
`pull_request.paths` 的 workflow，提取其实际构建的 crate（`cargo ... -p <crate>`），计算本地
`path = "..."` 依赖的传递闭包，凡闭包内成员目录不在 paths 中即失败。注入法验证：从已修复的
workflow 删掉 `visualization/**` 立即报缺失，还原即绿——不是永远绿的摆设。规则用"祖先目录
匹配"避免对 `core/src/formula/simd.rs` 这种精准子路径的假阳性。

## 30.3 接口文档断链

`docs/api-reference.md` 把 `C++: finkit::operation_execute_json` 列为八个语言面之一，但
仓库根本没有 C++ 绑定（`ffi/` 下只有 c/python/node/go/dotnet/ios/java/android 八个）；ios 与
android 走 per-indicator FFI（`alpha_ta_*` 与 `Java_com_finkit_...*Native`），并不参与统一
JSON 控制平面。同时 `FormulaEvalContractJSON` 这个 PascalCase 在全树 0 文件，真实是
`formulaEvalContractJson`。两处都对照源码核实后修正。中文版 `api-reference-zh.md` 没有这段
"八语言控制平面"段落，故无需同步。

## 30.4 死循环复查

全树 9 个 `loop { }`（另有 1 个是 WGSL 着色器字符串，已剔除）逐一核对：公式执行器的
WHILE/FOR 有 `iterations >= MAX_LOOP_ITERATIONS`（=10000）兜底；stateful 的 For、组合生成、
谐波合并、Bresenham 画线、MACD 回填均靠游标/索引单调推进 `break`。`check_unbounded_loops.py`
9/9 通过。结论：用户输入公式无法令引擎挂起。

## 30.5 复查未发现问题

- **公开 API 面**：第四轮已做全工作区差分（存活文件 0 删除）。本轮未改 rust，不再重复。
- **全部门禁 22/22 绿**（含新门禁），`gen_ssot_docs.py --check` 通过，四个 workflow YAML 合法。
- **cargo 编译**：排除 pyo3 的核心 crate 全绿；完整 `--all-targets` 因沙箱管道配额在 pyo3-ffi
  build script 调起 Python 解释器时耗尽（os error 231），属环境限制，非代码改动——本轮未改
  任何 `.rs`，第四轮已确认 0-error 基线。
