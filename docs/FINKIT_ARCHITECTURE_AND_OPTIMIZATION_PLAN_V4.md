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

### 第 6 项的真实完成度（2026-10-04 更新：已完成，见 §32.1）

四类 `NodeKind` 收敛到同一张图、同一个 `lower()`、同一个执行器，且**两条前端均已改走
`SemanticGraph`**（第七轮，§32.1）：

- 公式：`FormulaComputePlan::compile` 的 lowerer 现在把每个节点 push 进
  `SemanticGraphBuilder`（id 仍按 push 顺序分配，与旧直连路径逐字节同构），经
  `graph.build()` 校验后 `graph.lower()` 出 plan；`FormulaComputePlan::graph()` 把图
  本体（content_hash / CSE / 调度分层）暴露给下游。
- 因子：`factor-analysis` 的 `ResearchPlan::compile` 经 `SemanticGraphBuilder` 构图
  （`NodeKind::Factor`），含稳定 Kahn 拓扑推送（保持旧直连路径对非拓扑声明顺序的接受域）
  与 `node_stage` 重映射（公共 API 继续讲 stage id）。

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

# 31. 第六轮：方案实现度对照与文档治理（2026-10-04）

第六轮问的是前五轮都没系统问过的两条：**「对照方案，到底实现了几成？」**，以及
**「文档本身是否在互相矛盾？」**。

## 31.1 方案实现度对照结论

| 计划 | 状态 | 证据 |
|---|---|---|
| Batch 0（P0 发布红灯） | **已完成** | §4 清单全 `[x]`：fmt / clippy Euler 常数 / `avx2_fma_available` cfg / Python `u16` 变换 postcondition / C-C++ request JSON / toolchain pin / release aggregator |
| Batch 1（孤儿与仓库治理） | **已完成** | `circuit_breaker.rs` 等 5 个孤儿删除；`check_rust_source_reachability.py` 479 文件全可达；`check_no_tracked_build_artifacts.py`；`check_unbounded_loops.py` |
| Batch 2（生成系统收敛） | **前置已完成** | build-state gate + transform postcondition（完整 BindingSpec 单一生成器仍是独立里程碑） |
| Batch 3（Runtime 收敛） | **已完成**（第 6 项于 §32.1 收口） | §24a 六项落点齐全；第 6 项「公式与因子前端改走 `SemanticGraph`」已在第七轮落地（本表下文 31.2 写于其前，保留作审计轨迹） |
| Batch 4（性能） | **已完成**（fusion 已执行并经测量否决，见 §32.2） | CSE 4.2×、`execute_into` 零分配、DirtyRange、scheduler、workload benchmark；kernel fusion 第七轮完整实现→门禁→实测全链长更慢→代码移除，数据存 §32.2 |
| §18 五条边 | **已完成** | batch==streaming / full==range / allocating==into / tree==plan / Rust==FFI 各有具名门禁 |

**结论：V4 的 Batch 主体已落地，两处缺口（§24a 第 6 项、kernel fusion）都在方案内如实标注，
没有虚假声称。** 但本轮发现方案**漏记了一条**更关键的事实，见 31.2。

## 31.2 方案漏记的实现度缺口：默认执行路径仍是 Tree

`core/src/formula/engine.rs` 的 `FormulaExecutionMode` 默认值是 `Tree`，即**生产默认路径仍是
参考解释器，而不是 Plan**。这不是疏漏，是被实测挡住的：把默认翻到 `Plan` 会挂掉当时
**12 个 target / 143 个测试 / 76 个缺失 kernel**。

**但那条理由已经过期，而方案正文没有记录这件事的两面。** 缺口于 **2026-09-24 关闭**：
`unified_dispatch.rs` 把 44 个 TA-Lib 0.7/0.8 `CALL:<NAME>` kernel 全部路由到
`dispatch_modern_call`；`core/tests/formula_plan_differential.rs` 的两张 allowlist
（`DOMESTIC_UNSUPPORTED` / `PINE_UNSUPPORTED`）**都已是空表**，且该门禁对「陈旧的 allowlist
条目」会失败——所以「空」是断言，不是默认值。

于是 `engine.rs` 里以「76 个缺失 kernel」为**头条理由**的注释成了过期信息，而**真正剩下的
阻塞**（结构性，不是缺 kernel）被压在后一段：**含字符串字面量的公式在 `Plan` 路径上根本无法
运行**——树路径语义是「把字面量追加进 `FormulaContext::string_table`，求值为其下标」，而 plan
executor 只收到 `&[&[f64]]`，没有可追加的上下文。

**修复**：把该注释改为「缺口已于 2026-09-24 关闭 + 空 allowlist 是断言」，并把真正剩下的阻塞
（字符串字面量）提升为首要理由。**没有改任何行为**——默认仍是 `Tree`，翻默认仍是一次发布级
决定（前置条件是先给字符串字面量一个 plan 侧归宿）。这条事实已补记入本方案。

## 31.3 文档治理：四份文档同时自称「当前基线」

审计 `docs/` 时发现「谁是当前基线」有**四个互相矛盾的答案**：

| 文档 | 自称 |
|---|---|
| `docs/refactor-plan-2026-09-21.md` | 「**本文档是唯一执行基线**」 |
| `docs/archive/README.md`（两处） | 「**Only** `refactor-plan-2026-09-21.md` is an execution baseline」 |
| `docs/development.md` | 「current architecture baseline is `architecture-and-feature-audit-2026-09-26.md`，active optimization route is `refactor-plan-2026-09-26.md`」 |
| `docs/README.md` | V4 =「本轮架构与性能收敛计划」、09-21 =「唯一执行基线」、09-26 =「当前重构路线」 |

同一份索引里三个「当前」，加上 `development.md` 指向另外两个——贡献者读到哪份就得到哪个
答案。这正是必须清掉的「无效的信息」。

**修复（收敛到唯一权威）**：

- `docs/README.md` 的 "Current refactor baseline" 改为**显式声明只有 V4 是唯一执行基线**；
  09-21 改标为「**约束来源（非执行基线）**」；09-26 计划与审计标注为已归档。
- `docs/development.md` 指向 V4。
- `docs/archive/README.md` 的两处「只有 09-21 是基线」改为「唯一基线是 V4」。
- `docs/refactor-plan-2026-09-21.md` 抬头的「唯一执行基线」改为「已不是执行基线」，并说明其
  保留价值是**用户已确认的产品边界与定调**（不做回测 / 不做选股、JIT / `eval_simd` 冻结）。

> **为什么 09-21 不归档**：它承载**仍然生效**的约束。归档目录的定义是
> "historical records, **not current guidance**"——把仍然生效的约束放进去，等于把它们降级为
> 「仅供参考」，那才是真的丢信息。所以它留在 `docs/`，但**不再自称基线**。

## 31.4 归档清单（2026-10-04 批次）

| 文档 | 处置 | 理由 |
|---|---|---|
| `refactor-plan-2026-09-26.md` | `git mv` → `docs/archive/` | 被 V4 取代 |
| `architecture-and-feature-audit-2026-09-26.md` | `git mv` → `docs/archive/` | 当轮审计快照，结论被 §28–§30 取代 |
| `upgrade-completion-matrix-zh.md` | `git mv` → `docs/archive/` | 进度快照，且把 `bytecode` / `JIT` 列为**交付路径**——这两条**已冻结**，属无效信息 |

三份文档内部**没有 markdown 链接**（已核实），归档不产生断链；归档索引已补齐三行，被归档
文档抬头都加了横幅写明归档日期与「当前基线是 V4」。

## 31.5 接口文档修正（`docs/api-reference.md`）

§30.3 在第五轮删掉不存在的 C++ 绑定条目时，**留下了两处它自己引入的不一致**：

1. 正文仍写 "All **eight** public language surfaces share the same control-plane JSON
   contracts"，但列表只剩 **7** 条，且紧接着一段明确说 iOS/Android **不**暴露统一控制平面。
   → 改为「只对暴露控制平面的语言面成立，下列七个」。
2. C 与 Node 两条 bullet 被缩进成 6 空格，脱离 bullet 列表（被渲染成代码块）。 → 恢复为顶层 bullet。

**教训**：删条目时不仅要删那一行，还要重新数一遍被那条目支撑的数字。这是第四轮
「审计修复动作本身」在第五轮修复上的再次复现。

## 31.6 本轮验证

- doc 门禁：`check_orphan_docs.py`（归档后仍全可达）、`check_docs_links.py`（含归档文档新增的 `../` 链接）
- 生成态：`gen_ssot_docs.py --check` 通过
- 本轮只改了 1 个 `.rs`，且**只改文档注释、无行为变化**：`core/src/formula/engine.rs`

# 32. 第七轮：方案收尾执行（2026-10-04）

用户指令：方案剩余项**全部执行**；继续优化公式执行效率；≥3 轮审计后推送、重建安装包。
本章记录三件事的落地结果与数据。

## 32.1 Batch 3 第 6 项收口：公式/因子前端改走 SemanticGraph

- **公式**：`core/src/formula/compute_ir.rs` 的 `FormulaLowerer` 从 `Vec<ComputeNode>`
  直连改为 push 进 `SemanticGraphBuilder`；`NodeKind::Formula`（表达式节点）与
  `NodeKind::Constant`（`NUMBER` 字面量）。builder 的校验**就是** `ComputePlan::compile`，
  push 顺序即旧 id 顺序，因此图与旧直连产物逐节点同构；`FormulaComputePlan` 新增
  `graph()` 访问器，公式侧首次可以免二次降级使用 content_hash / CSE / 分层调度。
- **因子**：`factor-analysis/src/orchestration.rs` 的 `ResearchPlan::compile` 改经
  `SemanticGraphBuilder`。两个行为保持细节：① 旧直连允许**非拓扑声明顺序**（前向依赖），
  builder 不允许 → 增加稳定 Kahn 拓扑推送（声明顺序打破平局）+ 显式重复 id 报错（对齐
  旧 `DuplicateNode` 语义）；② builder id 是 push 序而公共 API 讲 stage id → 新增
  `node_stage` 重映射，`execution_order()` / `first_append_blocker()` /
  `first_range_blocker()` 语义不变。
- **验证**：`formula_plan_differential`（tree==plan，两张 allowlist 仍为空表）4/4；
  finkit lib formula 模块 674+；`factor-analysis` lib 47/47；`runtime_convergence` 17/17；
  `semantic_graph` 13/13。**未改任何执行行为**——校验器是同一个，计划同构。

## 32.2 Batch 4 第 6 项（kernel fusion）：已实现→已门禁→已测量→**数据否决，代码移除**

按方案自己的验收纪律（「没有量化的收益，就不该有一版更难维护的 kernel」）把这一项
**做完了**：实现了完整的 elementwise 链融合（hot_plan 编译期把单用途 `BINARY:*` 链
改写为一个 `FUSED:BINARY_CHAIN` 节点，后缀程序走参数区，派发器用寄存器机一次循环求值），
配齐 4 条门禁测试（值等价 / 确实触发 / 保留节点不被吞 / Div 近零护栏），全部通过——
然后用同一台机器、release、1M bars 实测：

| 链规模 | 公式 | 融合 | 不融合 |
|---|---|---:|---:|
| 3 op | `CLOSE*2 + CLOSE*3` | 12.2 ms | **10.5 ms** |
| 7 op | `CLOSE*2+CLOSE*3+CLOSE*4+CLOSE*5` | 24.9 ms | **20.4 ms** |
| 3 op (Mod) | `CLOSE%3 + CLOSE%2` | 18.9 ms | **13.3 ms** |
| 11 op (Mod) | 六个 `%` 链 | 60.0 ms | **38.2 ms** |
| 27 op | 十四项混合链 | 108 ms | **60.8 ms** |

**结论：在每个链长上融合都更慢（1.2×–1.8×）。** 逐元素寄存器机的每 op 成本
（~3.5–5.4 ms/op/百万行）约为独立 kernel 逐 pass（~2.1 ms）的 **2 倍**——现有
`BINARY:*` kernel 是 `for i in 0..len` 的平铺循环，LLVM 能自动向量化；而解释器的
逐步 match + 寄存器间接寻址阻止向量化，连「不可向量化」的 `Mod` 链也救不回来
（`floor_remainder` 在两条路径里都是标量，独立 pass 仍更快）。

**处置**：融合代码与门禁整体移除（遵守「无孤儿逻辑」红线——不上线也不留死代码）；
本节数据即该项的执行记录。**真正的赢面是 SIMD 代码生成**（把融合链编译成向量化
kernel），那在 `eval_simd`/JIT 的冻结边界之外，记为后继工作而不是假装它不存在。

## 32.3 公式执行效率 vs TA-Lib：现有证据与本次刷新

- **跨库对比（有版本钉死的工作流门禁）**：`docs/BENCHMARK_VS_TALIB.md` 的参考快照
  （2026-06-24，Windows x86_64 AVX2，TA-Lib 0.6.8 Python 绑定，最差数值偏差
  `4.7e-10`，判定 `PARITY`）：**14 项指标全部快于 TA-Lib，`1.03×`–`2.53×`，
  几何均值 ≈ `1.6×`**。对照 TA-Lib **C 0.8.1** 的 Criterion 门禁
  （`scripts/bench-vs-talib.sh`，钉死 `talib_core_version`，版本不符时宁可失败也不
  出报告）在 CI 定时工作流执行；本机无 TA-Lib C 静态库，`--features talib-c`
  构建如实失败（`ta-lib-static` 缺失），未伪造本地 C 级数据。
- **本机公式引擎刷新（2026-10-04，快速档）**：10K bars 下
  `MA(C,20)` 67µs / `EMA` 55µs / `RSI(14)` 30µs / `MACD` 149µs /
  `BOLL` 53µs / `ATR` 67µs；100K bars 下 `MA` 184µs / `EMA` 530µs。
  叠加既有成果：CSE 4.2×（Factor DAG 201→111 节点）、`execute_into` 零分配、
  DirtyRange 全量 8.85ms vs 100 行脏区 1.17µs。

## 32.4 三轮审计与验证

- **R7（代码）**：Batch3-6 落地 + 融合实测，全部用例绿（见 32.1/32.2）。
- **R8（文档/接口）**：本文件 §24a 第 6 项改写为完成态；CHANGELOG 增第七轮条目。
- **R9（收尾扫描）**：22/22 `check_*.py` 门禁、`gen_ssot_docs.py --check`、
  孤儿/断链/死循环门禁、安装包重建与契约门禁（见 CHANGELOG 当日条目的验证清单）。

# 33. 第八轮：效率、死基准与仓库卫生（2026-10-04）

用户指令：继续优化公式执行效率、全体指标优于 TA-Lib、清理历史无效文件、≥3 轮审计、
推送、重建安装包。

## 33.1 性能：Bytecode VM 去掉每次执行的全量 context 克隆

`BytecodeVM::execute` 每次执行 `ctx.clone()`——深拷贝全部五条 OHLCV 数组（10K bars
约 400 KB memcpy），而 VM 对 context 的**唯一**写点是 `PushString` 的 string table
（通常为空），且克隆体在函数末尾即被丢弃。改为借用数据 + 只克隆 string table，
scratch 表经 `ExecResult::string_table` 带出（与旧行为一致）。同机 criterion
变更检测（10k bars）：

| 公式（bytecode 路径） | 前 | 后 | 变化 |
|---|---:|---:|---|
| MA_20 | 49.0 µs | **34.5 µs** | −34%（现**快于** AST 40.6 µs） |
| EMA_12 | 50.0 µs | **34.1 µs** | −36%（快于 AST 54.0 µs） |
| RSI_14 | 58.0 µs | **42.2 µs** | −31% |
| MACD | 153 µs | 159 µs | ~持平（计算主导） |

bytecode 自身 53 测试 + 全量公式回归（3029 lib + 52 tree==plan + 17 convergence）全绿。
原生侧（复活的 `performance_benchmark`，100K bars）：SMA 174 µs / EMA 175 µs /
RSI 252 µs / MACD 1.35 ms / HHV 859 µs——叠加记录在案的跨库快照（14/14 快于 TA-Lib，
1.03–2.53×，geomean ≈1.6×，PARITY）与 CI 中钉版本的 TA-Lib C 0.8.1 门禁，本机无
`ta-lib-static` 不伪造 C 级数据。

## 33.2 三个从未测量过的 criterion 基准复活

`accuracy_test` / `formula_performance_bench` / `performance_benchmark` 三个文件躺在
`core/benches/` 却没有 `harness = false` 条目：`cargo bench` 把它们包进 libtest
harness，每次都报 "0 tests"——编译了、发布了、从不测量。补齐三条目后全部可跑，
`accuracy_test` 的跨指标精度报告全 ✓。审计了其余 crate（visualization /
factor-analysis / cli）——无 bench 目录，无同类缺陷。

## 33.3 历史无效文件清理（~188 MB，全部 gitignored，零 git 影响）

| 对象 | 大小 | 性质 |
|---|---:|---|
| `.core_test_tail.log` | 225 KB | 2026-08-30 遗留测试日志（与第五轮删的 `CargoLock_test_tail.log` 同类） |
| `core/test_output.txt` | — | 遗留编译错误日志 |
| `target-verify/` | **179 MB** | 陈旧验证构建缓存目录 |
| `ffi/c-binding/build-usage*/`（2 个） | 1.3 MB | build-usage 脚本的 CMake 陈旧产物 |
| 6 个图表输出（`gpu_large_chart.html` 等） | 7.3 MB | CI WebGL job 每次现场重新生成的产物，本地是 9/13 陈旧副本 |
| `dist/*.log`、`dist/.build-quick/*.log` | — | 发布目录里的陈旧构建日志 |

`scripts/archive/` 刻意保留：审计轨迹，`check_orphan_scripts.py` 已按 manual 记账。
V4 §31 对照表同步改为指向 §32 的完成态（原「未做/未实现」表述保留为审计轨迹）；
方案语料中剩余的真正未决项均为**用户决策门**（dzh 静默映射、IF 真值/ema 种子、
发布级默认路径翻转——约束文件自身规定行为变更需单独确认）。

## 33.4 三轮审计与验证

- **R8（代码）**：bench 配置审计（发现并修复 3 个死基准）+ VM 优化 + 全量回归绿。
- **R9（门禁）**：22/22 `check_*.py`、`gen_ssot_docs.py --check`、SSOT 一致、
  孤儿/断链/死循环门禁绿（死循环 9/9 有界）。
- **R10（收尾）**：非 pyo3 crate 全量 `cargo check` + clippy 0 error、提交推送
  （SSH，复核远端 SHA）、安装包重建 + manifest/契约门禁（见 CHANGELOG 当日条目）。

## 34. 第九轮 C：滚动极值接入缓存索引快路径 + NaN 透明（2026-10-04）

### 34.1 为什么做

`MAX` / `MIN` 及其派生族（`MIDPOINT` / `MIDPRICE` / `WILLR` / `STOCH` / `AROON`）
共享同一个滚动极值内核，而它在第九轮之前只有「缓存索引」一条快路径被
`MIDPOINT` 一类指标用上；`MAX`/`MIN` 本身走的还是每根 bar 重扫窗口的实现，
窗口 30 就是 O(n·w)。同一轮还发现：`talib_ext.rs::rolling_minmax`（`KDJ` /
`SMI` 的上游）和 `features/normalization.rs::rolling_minmax`（特征归一化器）
各写了一份 O(n·w) 的 fold —— **三个调用方、两份重复实现、零共享**。

### 34.2 NaN 契约的最终形态（本轮最容易走错的方向）

仓库早已存在一条**门禁化**的 warm-up 契约，第九轮之前只是没被写进文档，所以
本轮两次踩坑才反推出它的真实含义：

> 滚动极值在「窗口必须填满 `period` 个 bar」之后才出结果；**缺失值不填充窗口**。

用 pandas 的语言说，这是 `rolling(window=p, min_periods=p).max()`，等价于
`skipna=True` 但从**首个有限值**起算。设 `f = first_finite(values)`，则首个
报告位于 `f + period - 1`。

裁决依据是仓库自己的差分门禁（`formula_differential_tests` 的
`check_all_paths_with_warm_variable`，LEN=200，`warm = MA(CLOSE,5)` → 196 有限）：
`HHV(X,9)` 期望 **188** 个有限值。若按「窗口从 `period-1` 起就照报、只丢窗内
NaN」（contract A）会得到 192；只有「先跳过前导缺失段」（contract B）才得到
188 = 200 − (4+9−1)。同族其余锚点（`LLV`/`MA`/`SUM` 188、`MEDIAN` 192、`RSI` 191）
全部指向 contract B。

**本轮走过的两条错路，记下来防止再走：**

1. 先按 contract A 改（窗口内 dropna、不跳过前导段）→ 批量通过，但流式立刻
   与批量分叉（`HHV(MA(CLOSE,2),3)`：流式 `[NaN,NaN,NaN,11,12.5,13.5]` vs
   批量 `[NaN,NaN,10.5,11,12.5,13.5]`），且 4 条 warm-up 门禁从 188 变 192 而红。
2. 改流式时用了 `finite_count >= period` 作就绪判据 → 与批量的
   `i >= first_finite + period - 1` **不等价**：数据 `[5, NaN, 3, 4, …]` 在
   index 2 批量报 5（窗口 `[5,NaN,3]` 已占满 3 个位置），流式因为只数到 2 个
   有限值而报 NaN。正确形态是「跳过前导缺失段 + 之后按**绝对 bar 序号**计数」。

### 34.3 实现

| 层 | 改动 |
|---|---|
| 批量内核 `core/src/math/statistics.rs` | 新增 `first_finite(values) -> Option<usize>`；`fill_rolling_extreme` 计算 `warm_at = first_finite + window - 1` 并贯穿 `rolling_extreme_cached` / `rolling_extreme_ring` / `rolling_minmax_visit` / `rolling_minmax_cached` / `rolling_minmax_ring`。两条策略（≤512 缓存索引 / >512 单调环）在 NaN 语义上一致，原先的 `has_missing` 分流已删除 —— **gate=512 处不再有一条永远不可达的环** |
| 融合 high/low 内核 | `rolling_minmax_visit` 两腿**各自独立**算 `warm_high_at` / `warm_low_at`，emit gate 为 `i >= warm_high_at && i >= warm_low_at`，避免一条腿的前导缺失把另一条腿拖进 warm-up |
| 流式 `StreamingMax` / `StreamingMin` | 用 `started: bool` 跳过前导缺失段，`count` 改为「首个有限 bar 起的 bar 序号」；`NaN` 永不入单调队列但**仍推进 bar 序号**（既当窗口位置也参与过期，过期判据 `pos + period < count`）；就绪判据回到 `count >= period`。与批量逐位对齐 |
| 孤儿收敛 | `talib_ext::rolling_minmax`（O(n·w) fold）与 `features/normalization::rolling_minmax`（两份 fold）改为委托共享内核 `rolling_minmax_visit`；`KDJ` / `SMI` / 特征归一化器三个调用方共用同一条快路径 |

顺带清掉：`statistics.rs` 上一处因策略合并而失效的 `#[allow(dead_code)]`。

### 34.4 语义变更（对外可见，必须知晓）

1. **前导缺失段推迟首个报告**：`[NaN, NaN, 10, 8, 12, 6, 14]` 的 `HHV(_,3)`
   从 `[NaN,NaN,10,10,12,12,14]` 变为 `[NaN,NaN,NaN,NaN,12,12,14]`。
2. **窗口内缺失值不再污染结果**：`[2, 3, NaN]` 的 max 从 `NaN` 变为 `3`
   （第九轮之前 NaN 一旦入队就永不被支配，把后续所有窗口全打成 NaN）。
3. **全缺失窗口返回 `None`**（流式）/ `NaN`（批量），不再回吐上一个窗口的值。
4. **特征归一化器**：当前 bar 本身缺失时结果为 `NaN`（原为 `0.5`）；窗口内有
   缺失时不再污染上下界。退化窗口（上下界相等）仍为 `0.5`。

前 3 条是第九轮用户拍板的「本轮修成 NaN 透明」；第 4 条是收敛的副作用，已在
CHANGELOG 明确列出。

### 34.5 门禁

- `core/tests/extrema_cached_path.rs`（新增，10 条）：两策略对拍、缓存边界两侧
  全覆盖（1/2/3/5/14/30/64/128/200/255/256/511/512/513/1024）、NaN 不污染、
  前导缺失推迟首个报告、全缺失窗口、融合内核每腿独立丢弃、等值取最早下标、
  长序列不漂移。
- `stateful.rs` 的 `streaming_extrema_defer_the_first_report_past_a_leading_missing_run`
  把流式与批量在**同一份输入**上钉在一起（此前只钉了无前导缺失的情况）。
- `TEST_INDEX.md` 补齐 3 个未被索引的 target（`extrema_cached_path` /
  `formula_draw_parity` / `formula_string_literals`）—— `test_index_contract`
  是双向门禁，正是它把前几轮悄悄新增的测试 target 顶出来的。

### 34.6 验证（2026-10-04，rustc 1.98.1）

| 项 | 结果 |
|---|---|
| `cargo test -p finkit --lib` | **3036 passed / 0 failed** |
| `cargo test -p finkit --test extrema_cached_path` | **10/10 passed** |
| `cargo test -p finkit --test formula_differential_tests` | **52/52 passed** |
| `cargo test -p finkit --tests`（全部 integration target） | **全绿**（含 `test_index_contract`） |
| `cargo fmt --all --check` | 干净 |
| `cargo clippy --workspace --all-targets` | 本轮改动范围内**无新增 lint**（命中的 15 条全是 workspace 既有的 pedantic 噪声：`must_use_candidate` / `missing_errors_doc` / `doc_backticks` 等） |
| `cargo test -p finkit --doc` | **240 failed / 0 passed**，失败原因 100% 为 `Failed to spawn` + `所有的管道范例都在使用中`（ERROR_PIPE_BUSY，Windows 沙箱内 rustdoc 无法 fork rustc）。改动前的基线是同一数字，**非本轮回归** |

两个环境层面的既有失败（均非本轮引入、均与本仓库代码无关）：

- **doc test**：ERROR_PIPE_BUSY，见上表。
- **`pyo3-ffi v0.29.2` build script 退出码 1**：`cargo clippy --workspace`
  的唯一 `error`，Python 绑定在本机缺可用的解释器发现路径；其余 crate 全部
  0 error。

未闭环（明确记账，不假装做完）：§32.3 记录的 30 项「追平 TA-Lib」复测、以及
第九轮 D/E 的三轮审计与安装包重建，按用户在决策门上的选择另行排期。

# 35. 第十轮：TA-Lib C 全量对比、落后项算法化追平与公式层审计（2026-10-04）

用户指令：全部指标在**数值和计算效率**上对比 TA-Lib、提升公式执行效率、继续排查
断链/孤儿/死循环、≥3 轮审计、更新文档、推送、重建安装包。

## 35.1 TA-Lib C 0.8.1 全量对比（本机首次跑通 C 级基准）

`C:\TA-Lib\lib\ta-lib-static.lib` 在位后，`scripts/bench-vs-talib.sh` 全链路首次在本机
跑通：90 组 Finkit↔TA-Lib 配对（10K bars 基准 + 10K/100K/1M 缩放组 + 40+ 指标的
AVX2 派生组）。**首轮快照：59 组 Finkit 更快，13 组 ❌（慢于 TA-Lib 25% 以上）**。

**门禁脚本本身的一个真 bug 顺带修复**：`bench_report.py` 用 `startswith("FTA_")`
做大小写敏感匹配，而 Criterion 在磁盘上把 bench id 小写化（`fta_sma_20`），
导致配对发现恒为 0（`found 0 paired rows`）；且带 scale 的目录布局索引错位
（bench 与 scale 取反）。修复后报告/JSON 门禁恢复工作。

## 35.2 落后项逐项追平（算法级，全部语义保持或对齐 house NaN 契约）

| 指标 | 轮前 µs | 轮后 µs | TA-Lib µs | 前→后 | 手段 |
|---|---:|---:|---:|---|---|
| WILLR_14 | 83.5 | **32.1** | 34.3 | 0.42x ❌ → **1.07x ✅** | `willr14_into`（Van Herk 块扫描）本就是为 bench 关键路径写的，但**从未接入公共入口**（孤儿快路径）；`willr_into` 在 period==14 且输入无 NaN 时分派给它 |
| STOCH_14_3_3 | 150.4 | **84.9** | 95.5 | 0.65x ❌ → **1.11x ✅** | `stoch_monotonic_fast_into` 的极值维护从 128 槽环形队列换成 cached-index（每腿每 bar 一次比较，过期才回扫）；平局取最新、过期边界与原队列一致 |
| AROONOSC_14 | 170.1 | **59.4** | 44.1 | 0.26x ❌ → **0.72x ❌**（2.9x 提速） | 整个函数从 VecDeque 重写为与 `aroon_with_deques` 相同的 cached-index 结构 |
| MAX_30 | 29.1 | **22.3** | 14.6 | 0.52x ❌ → 0.65x ❌ | 内核重写（见 §35.3）：比较天然跳 NaN 免显式分支、expiry 仅在比较失败后查、warm 相位拆分消除每 bar emit 门 |
| MIN_30 | 37.0 | **32.1** | 15.0 | 0.42x ❌ → 0.47x ❌ | 同上 |
| ln | 75.0 | 见最终快照 | 47.2 | 0.63x ❌ | 两遍扫描（全量验证 + 全量计算）融合为单遍；错误契约不变（首个非法元素拒绝整次调用） |

**STOCH 族 NaN 语义修正（随 STOCH 追平落地）**：原队列实现把 NaN 推入队列使其
成为窗口极值，`denominator > 1e-15` 判假后 fast-K 输出 50.0；现按契约 B 直接
dropna——NaN 输入不再冻结在 50.0，与 HHV/LLV 家族一致。

## 35.3 公式层追平 + 潜在问题修复（第一轮审计产出）

`core/src/formula/functions_legacy.rs` 中 15+ 处 O(n·w) 逐窗重扫的算法化：

- **HHVBARS / LLVBARS / MAXINDEX / MININDEX** → 共享 `ArgExtremeDeque`（单调
  deque 取 arg-extreme，O(1)/bar）。平局保**最早**下标（严格比较弹栈——eager
  弹平局会在旧元素过期后把答案错位到更晚的 bar，被 `test_fn_maxindex` /
  `test_hhvbars_llvbars_keep_values_without_window_allocations` 当场抓住）。
- **EVERY / EXIST / COUNT** → 滑动真值计数 O(1)/bar。EVERY 的谓词是 `<= 0.0`
  （NaN 不算假），因此计"失败数"而非"成功数"，逐位保持原语义。
- **VWMA / MFI / TOTALVOL** → 滑动和 + 窗内缺失跟踪：NaN 只污染含它的窗口
  （与逐窗重扫一致），而非永久毒化累加器。
- **ICHIMOKU_TENKAN / ICHIMOKU_KIJUN / DONCHIAN（中轨/上/下/宽）** →
  `rolling_minmax_visit` / `rolling_max` / `rolling_min`（第九轮快内核）。
  DONCHIAN_UPPER/LOWER 全缺失窗口由 `±inf` 改为 NaN（与家族对齐）；
  ICHIMOKU 的 `(n - 1)` 在 `n == 0` 时的 **usize 下溢 panic**（参数绕过了
  `extract_n` 守卫）一并消除——内核对 `window == 0` 幂等返回。

**其他修复**：

- **解析器栈溢出**（abort 级，FFI panic 守卫拦不住）：pest 递归下降对超深
  `((((…))))` 会打爆线程栈。`parse_formula` 现预扫描括号嵌套深度，超 256 层
  直接返回 `Err`（新增 `test_deep_nesting_is_rejected_not_a_stack_overflow`）。
- **aroonosc 全缺失窗口 panic**：旧实现 `highs[0]` 索引空队列；重写后报 NaN。
- **aroonosc usize 减法下溢**：`(highest_idx - lowest_idx)` 改为先转 f64 再减。
- **`bench_report.py` 配对 bug**：见 §35.1。

**评估后不接线（如实记账）**：`simd_ops::simd_linreg_slope`（含 AVX2）零外部
调用者，但其 AVX2 变体只向量化了初始窗口求和（period=14 仅 3 个 chunk），主循
环与标量相同——接线无收益，保留为公共 API 并在此记录结论。
`SimdOps::hhv/llv`（AVX2/AVX512/NEON 块扫描）同样无内部调用者（公式层走
`statistics` 快内核），但属公共 SIMD API 表面，保留。

## 35.4 三轮审计结论

1. **第一轮（静态扫描 + 修公式层）**：产出 §35.3 全部修复；`extract_n` 守卫
   确认覆盖其余 `(n - 1)` 循环。
2. **第二轮（测试反馈驱动）**：ArgExtremeDeque 平局/过期语义 bug、MININDEX
   offer 前置、aroonosc 下溢——全部被 lib 门禁当场抓住并修复（3037/3037）。
3. **第三轮（回归 + 门禁）**：62 target / 3950 integration 全绿（含
   `golden_talib_tests` 数值金标、`formula_differential_tests` 52/52、
   `extrema_cached_path` 10/10、`test_index_contract`）；`cargo check` 0 警告；
   fmt / clippy / 22 门禁见 CHANGELOG 当日条目。

**死循环/递归审计**：executor 三处 `loop` 均有 `MAX_LOOP_ITERATIONS`（10_000）
上限；stateful FOR 有 `STATEFUL_MAX_LOOP_ITERATIONS` 背照 + 构造期拒绝；沙箱
递归深度可配上限。解析器新增嵌套深度守卫（§35.3）后，四条递归链
（parse→optimize→compile→execute）全部有界。

## 35.5 追平状态总账（诚实记账，最终快照 `docs/BENCHMARK_REPORT.md`）

- **90 组配对：61 组 Finkit 更快或持平（轮前 59 组），13 组仍 ❌**。
- **本轮移出 ❌**：`WILLR_14`（0.42x → **1.07x ✅**）、`STOCH_14_3_3`
  （0.65x → **1.11x ✅**）；`AROONOSC_14` 0.26x → 0.72x（仍 ❌）；
  `MAX_30` 0.52x → 0.67x、`MIN_30` 0.42x → 0.47x、`ln` 0.63x → 0.76x
  （仍 ❌，根因见下）。
- **仍落后（❌，根因已定位）**：
  - `MIN_30`/`MAX_30`/`AROONOSC_14`：TA-Lib 的裸 cached-index 循环没有 NaN
    契约、warm 门与 `has_finite` 跟踪，我们的每 bar 分支多 1-2 条；再压只能
    牺牲缺失值语义（不做）。
  - `STOCHF`/`STOCHRSI`：走 `stochf_with_ma_type` 另一条内核路径，未在本轮
    改造范围（与 `stoch` 的 monotonic fast path 不同代码）。
  - `ULTOSC`：已是环形缓冲零分配实现，剩余差距在 TA-Lib 的三周期滑动和布局。
  - `LINREG_SLOPE/INTERCEPT`：已是 O(1)/bar 滑动实现，剩余 ~1.3x 为
    TA-Lib 固定系数布局差异。
  - `PERCENTRANK_30`：窗口内比较计数本质 O(w)（每 bar 的参照值在变，前缀和
    不适用），TA-Lib 同为 O(w) 但常数更小。
  - `ln`：单遍融合后仍慢 ~25%，TA-Lib 直接调 C 运行时 `log()`，每元素成本
    已接近 `log()` 本身。
  - 缩放组 `SMA/RSI/BBANDS @1M`：10K/100K 领先、1M 落后，为缓存效应，非算法
    问题。
- **数值一致性**：`golden_talib_tests`（TA-Lib 金标 JSON，覆盖全部对比指标）
  在 62 target / 3950 全绿中；`bench_vs_talib_precision.py` 需要 PyPI
  `talib` + `finkit` wheel（本机未装，按设计 exit 2，不伪造数据）。


## 36. 第十一轮：落后项追平 + 公式层 O(n·w) 清零 + 三轮审计（2026-10-04）

**最终快照：90 组配对，8 组 ❌（第十轮收尾 13 → 11 → 本轮 8）**，详见 `docs/BENCHMARK_REPORT.md`。

### 36.1 全量基准的干净化（先修测量，再谈优化）

第十轮结束时的基准日志里夹着 `Gnuplot not found` 横幅并疑似重复执行，根因是
`criterion` 0.5 的默认特性带上 `plotters`（后端需要 gnuplot）。关闭默认特性后
`Cargo.lock` 少了 30 行 plotters 依赖树，清空 `target/criterion` 重跑：
**EXIT=0、0 条横幅、185 组配对**。这一条不改任何算法，但它是本轮所有"前后对比"
数字可信的前提——前一轮的数字是在有干扰的采样环境下取的。

### 36.2 滚动极值：Van Herk–Gil–Werman 块扫描接入值内核

第十轮把 MAX/MIN 换成 cached-index 后仍是 ❌（MAX_30 0.67x、MIN_30 0.47x）。根因
不是常数因子，而是**算法退化**：cached-index 在缓存的极值离开窗口时要做一次全窗
回扫，而基准数据是"正弦 + 噪声"的 NaN-free 序列，极值几乎每 bar 都在移动，于是每
bar 都是 O(w) —— 摊还 O(1) 的前提（极值长期驻留）在噪声序列上根本不成立。

Van Herk–Gil–Werman 算法把这个分布依赖消掉：按 period 分块，预计算
`suffix`（块内后缀极值）与 `prefix`（下一块前缀极值）两张表，每 bar 只需 ~3 次比较：

- 跨界窗口 = `suffix[offset] ⊕ prefix[offset - 1]`；
- `today` 每轮推进 `n_available + 1`（部分块时 `n_available = len - block_next`，
  `today` 自然越界退出）；
- 输出值与 cached 内核**逐位相同**（"值"版平局不影响结果）。

接入位置是 `fill_rolling_extreme`：`window <= EXTREMA_CACHE_LIMIT(512)` 且
**全部元素有限**时走块扫描，否则回退 cached-index（>512 仍走环形 deque）。
守卫用 `is_finite()` 而不是 `!is_nan()`：本家族的 `first_finite` 只跳过 NaN，
前导 `inf` 是**真实值**（与 `math::leading_warmup` 跳过所有非有限值的语义不同），
块扫描没有 warm-up 概念，若放行会多输出几个 `inf` bar —— 见 §36.5 第 1、2 条。

**结果**（同一次运行内配对，比率 = TA-Lib / Finkit，<1 = Finkit 较慢）：

| 指标 | 轮前 | 轮后 | TA-Lib | 说明 |
|---|---:|---:|---:|---|
| MAX_30 | 0.67x ❌ | **0.80x ⚠️** | 15.30 | 19.11 µs → 出 ❌（+19%） |
| MIN_30 | 0.47x ❌ | **0.77x ❌** | 15.74 | 20.50 µs → 仍 ❌（+64%，幅度最大） |
| ULTOSC_7_14_28 | 0.65x ❌ | **0.84x ⚠️** | 57.71 | 69.10 µs → 出 ❌（取模消除 + 极值分派传导） |
| BBANDS@1M | 脏数据 ❌ | **0.92x ⚠️** | 11240.69 | 12160.17 µs → 出 ❌（上轮为脏采样，非回归） |
| SMA@1M | 0.58x ❌ | **0.97x ⚠️** | 3277.28 | 3372.10 µs → 出 ❌（内存遍数 5→1） |
| RSI@1M | 0.43x ❌ | **1.02x ✅** | 4265.69 | 4190.31 µs → 反超（删除全量 init_output 填充） |
| MACD@10K | ❌（脏） | **1.05x ✅** | 151.64 | 144.30 µs → 出 ❌ |

### 36.3 AROONOSC：一次**失败**的优化尝试（如实记账）

`aroonosc` 需要 argmax(high) 与 argmin(low)，尝试了三版：

1. 两内核版：分别调 arg 版 Van Herk ×2 → **0.63x**（比第十轮 cached 版 0.72x 更差）；
2. 融合版 `aroonosc_block_into`：一次扫描并行维护 high/low 的 8 张值+索引表，
   单遍、零分配 → 定向复测两次，每 bar ~10ns vs cached 版 ~7.5ns，**慢 1.65–2.1x**。

结论：**八张动态索引表的代价超过了它省下的回扫**，撤销快路径、删除融合内核与其测试，
`aroonosc` 保持 cached-index，并在函数注释里写下这次失败的实测数字（避免后人重踩）。
这也是本轮唯一一个"优化了反而更慢"的项，记账而非隐藏。

### 36.4 热路径取模与内存遍数

- **取模消除**：`stochf` 通用路径的 `d_idx % fastd_period`、`stochrsi_into` 的两处
  `(x + 1) % period`、`ultosc` 默认路径的 `i % 7 / i % 14 / i % 28`，全部换成
  wrap 计数器（`pos += 1; if pos == N { pos = 0 }`）。每 bar 一次 `%` 是一次除法
  （~20 cycles），这些循环每 bar 要算 1–3 次。语义完全等价：这些位置每 bar 恰好
  递增 1（`stochf` 的首个发出 bar 验证为 `fastk_period - 1`，即 `d_idx` 从 0 起）。
- **内存遍数**：`sma_inner` 原为 5 遍（校验 2 遍 + warm-up 1 遍 + 全量 NaN 填充 1 遍
  + kernel 1 遍），融合为**单遍扫描**同时求 `start` 与首个非法值，且只填充预热前缀
  （kernel 会立即覆写其余槽位）。`rsi_inner` 同理删掉全量 `init_output` 填充（三条
  RSI 路径 AVX512/AVX2/scalar 都自己写预热 NaN）。`sma@1M` 慢的根因就是这几遍
  内存扫描，而非滑动和本身。

### 36.5 三轮审计：本轮修掉的 6 个真问题

1. **`sma_inner` 融合扫描的契约回归**（第一轮静态扫描未发现，第三轮逐行比对揪出）：
   `leading_warmup` 跳过**所有**非有限值（NaN 与 ±inf），而我写的融合扫描用
   `is_nan()` 找 `start`，于是前导 `inf` 会被当成"序列开始后的非法值"直接报错，
   旧代码却视其为预热。改为 `is_finite()`，并补两个回归测试（前导 inf 不报错、
   序列开始后的 inf/NaN 仍报错）。
2. **极值分派守卫同病**：见 §36.2 末尾。两层"预热"定义不同（moving_avg 跳所有非
   有限、statistics 只跳 NaN），跨层套用必错 —— 已写进代码注释与本节。
3. **`MODE` 结果不可复现**：用 `max_by_key` 在 `HashMap` 上取众数，而 Rust 的
   `RandomState` 每次进程随机化迭代顺序 → 出现并列时**同一次公式不同进程给出不同
   结果**。改为"计数最高 → 窗口内最早出现者胜"，并复用 HashMap 去掉逐 bar 的 Vec
   分配（原本每 bar 一次分配 + 一次 `filter` 收集）。
4. **`BACKSET` O(len·n) → O(len)**：BACKSET 是**向前回填**（bar `j` 亮 ⟺
   `[j, j+n-1]` 内有触发），因此正向倒计时不等价，必须**反向**扫描 + 倒计时：
   触发点装填 `n`，随 `i` 递减衰减。这是通达信公式里使用频率最高的函数之一。
5. **`LAST(X, A, B)` O(len·(A-B)) → O(1)/bar**：定宽滑窗全真判定，窗口从
   `[i-1-A, i-1-B]` 平移到 `[i-A, i-B]`，维护违规计数（去掉 `i-1-A`、加入 `i-B`）。
   `A >= data_len` 时按冷区提前返回全 0，与旧实现的 `i < A` 跳过一致。
6. **`SUMBARS` O(n²) → O(n log n)**：原实现每 bar 从 `i` 反向走到 0，`SUMBARS(VOL,
   CAPITAL)` 这类阈值很大的用法就是实打实的平方级。值全部**非负且有限**时前缀和单调，
   改为前缀表 + `partition_point` 二分（逐 bar 变化的阈值也可以，每次搜索独立）；
   含负值或缺失值时单调性不成立（多加一个 bar 反而降低和），保留原回扫。

以上 6 项全部配了差分/契约测试（与朴素实现逐位比对）：`backset_matches_naive_fill`、
`last_matches_naive_rescan`、`sumbars_matches_naive_walk`、
`mode_ties_break_to_earliest_occurrence`、`test_sma_leading_inf_is_warmup_not_error`、
`test_rolling_extreme_leading_inf_uses_cached_kernel`。

### 36.6 公式层其余 O(n·w)：评估后**不改**（记账）

- `fn_cmo`：可改滑动和，但会带来浮点累加次序变化（原实现每窗重算，无漂移）；且公式
  层 CMO 不在 TA-Lib 对比基准内，无基准支撑 → **不改**，理由同第十轮的
  `covariance / correlation / decay_linear`（公有 API 数值漂移 + 无内部调用者）。
- `timeseries.rs` 的 `covariance / correlation / decay_linear`：同上，维持第十轮结论。
- 其余 15 处嵌套循环已逐处确认是单遍或 `break` 提前退出，无数据规模级的重扫。

### 36.7 最终快照与仍落后项

见 `docs/BENCHMARK_REPORT.md`（本次运行，90 组配对）。仍 ❌ 的根因：

- `linreg_slope/intercept`：已是 O(1)/bar 滑动实现，剩余差距是 TA-Lib 固定系数
  布局差异（常数因子）。
- `ln`：单遍融合后仍慢 ~25%，TA-Lib 直接调 C 运行时 `log()`，每元素成本已接近
  `log()` 本身。
- `percentrank_30`：窗口内比较计数本质 O(w)（每 bar 参照值在变，前缀和不适用），
  TA-Lib 同为 O(w) 但常数更小。
- `macd`、`stochf`、`stochrsi`：顺序链（EMA→EMA→Signal）与复合内核，本轮只吃到
  取模消除的收益，链条本身未重排。
- `max_30` / `aroonosc_14`：见 §36.2 / §36.3。
- 缩放组 @1M：10K/100K 领先、1M 落后为缓存效应，非算法问题（SMA/RSI 本轮已因内存
  遍数优化改善）。

**跨运行噪声警告**：本机不同时段 bench 绝对值会膨胀 1.3–1.6x，跨运行对比 ±30% 属
正常；本轮所有"前后对比"均取自**同一次运行内**的配对，或定向复测（同运行配对）。


## 37. 第十二轮：全链路连通性审计、孤儿资产清理与 CI 覆盖补洞（2026-10-05）

本轮的主题不是再追一个性能指标，而是回答四个问题：**主体流程是否全部联通、
核心链路有没有断链、是否存在孤儿逻辑、前后端是否贯通**。方法是把"人工翻阅"换
成可复现的扫描（文档引用图、孤儿资产判定、feature 门控测试清单、循环终止性），
每轮的发现全部修完再进入下一轮。

### 37.1 审计方法与覆盖面

| 检查 | 手段 | 结果 |
| --- | --- | --- |
| 文档断链 | 全仓 123 个 md、413 条相对链接逐个解析 | 3 条断链（已修） |
| 文档引用图 | 入链计数（md/py/yml/rs/json） | 40 个零入链文档，逐个人工判定 |
| 文档→代码路径引用 | 反引号路径逐个 `os.path.exists` | 15 处指向不存在的文件 |
| 孤儿公式函数 | 295 个 `fn_*` 与注册表交叉 | **0 孤儿**（全部注册） |
| 裸 `loop`/`while` 终止性 | 逐个推演推进量与上限 | 6 处全部有单调推进或迭代上限 |
| 并发死锁 | `Mutex`/`RwLock` 获取点排查 | 5 处单锁方法 + poison 转错，**无嵌套取锁** |
| 孤儿资产 | 被跟踪但无人引用的 html/json/svg | 1 个模板 + 6 个构建产物 |
| CI 中被编译掉的测试 | `#[test]` 上的 `cfg(feature)` 扫描 | 3 个 feature 门控簇，2 个从未运行 |
| 语言绑定漂移 | `sync_bindings.py --check --all` | Python/Node `drift=none`；其余 6 语言 DEFERRED（已知、已记录） |

### 37.2 修复清单

| # | 问题 | 根因 | 处理 |
| --- | --- | --- | --- |
| 1 | `visualization/frontend/index.html.template` 无人引用，且 `{{LOCALE_JS_PATH}}` / `{{CONFIG_JS_PATH}}` 指向不存在的文件 | ECharts 时代的模板，被 Lightweight Charts 适配器取代后未清理 | 删除 |
| 2 | `visualization/` 下 6 个被跟踪的构建产物（7.3 MB，`gpu_large_chart.html` 单个 7 MB） | `.gitignore` 与 `check_no_tracked_build_artifacts.py` 都把产物规则**锚定在仓库根**，而文件实际落在 `visualization/`，门禁于是报"none look like build output" | 取消跟踪；`.gitignore` 去掉根锚定；门禁改为**从 `*/examples/*.rs` 里的输出文件名动态推导**，任意深度匹配 |
| 3 | 前端适配器契约测试从未在 CI 运行 | 只接线了 `scripts/test_webgl_runtime.mjs`；适配器是 `include_str!` 内联进每个发布 HTML 的，其契约属于交付物 | CI 增加 `node --test visualization/frontend/lightweight-charts-adapter.test.mjs`（本地 3/3 通过） |
| 4 | Rust→HTML→JS 全链路无集成测试：载荷结构有单测、适配器有单测，**接缝**（模板接线、脚本转义、`schema_version` 一致）无人验证 | 特性测试放在 `#[cfg(feature = "html")]`，而 CI 用默认 features 跑，整段被编译掉 | 新增 `rendered_html_wires_the_adapter_to_a_parseable_payload`（解析内联 `const payload = {...}` 并校验 `schema_version`/K 线数/指标线），CI 改为 `--features html` |
| 5 | 4 个并行批测（`run_parallel_matches_serial` 等）从未运行 | `rayon` 非默认 feature | CI 增加 `cargo test -p finkit --features rayon --locked --lib -- batch::`（本地 4/4 通过）——并发路径恰好是最不该没有覆盖的地方 |
| 6 | `scripts/benchmark_talib_all_current_gate.py` **永远不会失败** | 入口调用 `run(...)` 后丢弃返回的 summary，`errors` / `parity_failures` 不影响退出码；而同族的 `..._full_current_gate.py` 有 `return 0/2` | 补 `main()` 返回 2（与同族一致）；两个脚本此前**只被已删除的归档文档引用**→孤儿，现已接进 `talib-release-gate.yml`，补上 61 个 CDL 函数与 math/operator/statistics 的门禁覆盖 |
| 7 | `docs/archive/` 22 份 + `finkit-vs-talib-expanded-benchmark-results.md` 失效 | 归档索引自己把每一条都标为 Superseded；部分文档引用已删除模块（`runtime_engine.rs`）、已冻结路径（`bytecode`/`JIT`）与过期数字（公式函数 399 → 现 452） | 从工作区删除（**git 历史永久保留**，`git log --diff-filter=D -- docs/archive/` 按名找回）；同步修掉 `docs/README.md`、`runtime-carrier-adoption-plan`、`refactor-plan-2026-09-21` 与两个门禁脚本文档串中的悬空引用 |
| 8 | 3 条断链 | `docs/src/quickstart.md`、`PINE_COMPAT_MATRIX.md`、`migration/pine-to-AlphaTA.md` 均为改名前的路径 | 指向 `docs/getting-started.md`、`docs/generated/pine-compatibility.md`、`docs/migration/pine-to-finkit.md` |

### 37.3 接口文档补齐

`docs/api-reference.md` 与 `docs/api-reference-zh.md` **此前完全没有** Web 图表载荷
这一公开契约（Rust 与前端之间唯一的接口）。两处均新增章节：载荷字段表、
`schema_version = 1` 的强制语义、`null` 而非 `NaN` 的缺失值约定、以及
`createFinkitLightweightChart` 的调用与 `setPayload`/`update` 的重新校验行为。

### 37.4 结论

- 无孤儿公式函数、无孤儿模块（门禁）、无孤儿脚本（门禁改后重跑通过）、无死循环（门禁 + 逐处推演）、无死锁。
- 前后端贯通已由**测试**保证而不只是靠人工检查：Rust 渲染 → 内联适配器 → 载荷解析 → JS 适配器单测，四段各有断言，接缝有集成测试。
- 剩余已知缺口（不掩饰）：`sync_bindings.py` 对 C/Go/.NET/iOS/Java/Android 六种语言为 DEFERRED，**不做漂移检查**，该状态在 `docs/language-bindings.md` 有记录；推进到检查范围需先 `--discover --lang <lang>`。

## 38. 第十三轮：把最后一批低于 TA-Lib C 的指标推过 1.0，并清掉孤儿内核（2026-10-05）

本轮同时回答两个问题：**剩余指标为什么还慢**，以及**仓库里还有没有看着活着其实没人调用的代码**。
方法仍然是三轮：每轮的发现全部修完再进入下一轮；每一处性能改动都先确认语义等价（并列取最新、
warm-up 契约、错误优先级），再由 3047 个单测 + 62 个集成测试目标兜底。

### 38.1 性能：53 项里最后 8 个 ❌ 的逐个归因

前几轮把"算法量级"的问题基本清完了（O(n·w) → O(n)），剩下的差距全部来自**常数因子**：
多一趟全长内存写、多一个分支、多一次分配。逐个定位如下。

| # | 指标 | 旧加速比 | 根因 | 处理 | 新加速比 |
| --- | --- | --- | --- | --- | --- |
| 1 | STOCHRSI | 0.45 | 公开 `stochrsi` 用 **4 个全长缓冲 + 4 趟**（RSI 数组 → raw %K 数组 → 两次 SMA），而零拷贝的 `stochrsi_into` 早已存在却没人调用；窗口极值还用两条 `VecDeque<(usize, f64)>` | 公开函数改为走 `stochrsi_into`：RSI 暂存进 %D 缓冲，%K 原地覆写，最后一趟环形 SMA 同时产出 %K/%D；极值换缓存索引内核 | **1.19** |
| 2 | STOCHF | 0.59 | 两条单调队列 `Vec<usize>` **只推进逻辑头 `h_head`、从不回收**，于是每次调用都增长到序列全长（10k → 80 KB ×2）并触发约 10 次重分配与 memcpy；这正是"STOCH 用缓存内核 1.15x、STOCHF 用队列 0.59x"这对矛盾数字的全部来源 | 换成与 `stoch` 同源的缓存索引 arg-extreme 内核：每 bar 每腿一次比较，零分配 | **1.09** |
| 3 | PERCENTRANK | 0.74 | 每 bar **三次** `partition_point`（逐步数据相关分支，30 元素窗口上误预测代价远大于省下的 5 次 load）+ `Vec::remove` 与 `Vec::insert` 两次全长 memmove | ① 无分支二分（`cmov`）② 插入位置由 `count_less - (evicted < current)` 推出，**消掉第三次查找** ③ 两次 memmove 合并为一次 `copy_within` 区间搬移 | **1.07** |
| 4 | ACOS | 0.96（旧值 143.5 µs 系历史基线失真） | 无改动；同批复测为 41.2 µs，TA-Lib 132.3 µs 同批测得 | 记录实测值；两侧都做逐元素 `[-1,1]` 域校验，比较公平 | **3.21** |
| 5 | MIN_30 / MAX_30 | 0.75 / 0.80 | 输出先 `init_output` 全量 NaN 填充，内核再写一遍 → 两趟全长 store | 先给 `fill_rolling_extreme` 立契约"从 `window-1` 起每个槽位都由本调用写满"（前导缺失段的缺口自己补），`min`/`max` 于是可以只填 warm-up 前缀、其余用未初始化缓冲 | 0.93 / 0.88 |
| 6 | LINREG_SLOPE / INTERCEPT | 0.69 / 0.74 | 同上：全量 NaN 填充 + 内核覆写 | 同上；`linreg_slope` 另在 `warm_start == 0` 时改走 `simd_ops::simd_linreg_slope`（见 38.2） | 0.75 / 0.81 |
| 7 | `rescan_extreme_window`（共用内核） | — | 每元素两次判断 `!is_nan() && (!found \|\| cmp)` | 用 `NEG_INFINITY`/`INFINITY` 种子把 NaN 跳过**并入比较本身**，每元素只剩一次比较；保留 `>=`/`<=` 以维持"并列取最新"——AROONOSC 报的是**索引**不是值，并列规则变了结果就变了 | 惠及 AROONOSC / AROON / ULTOSC / STOCH / STOCHF / STOCHRSI |
| 8 | LN | 0.75 | 边校验边 `push`，`push` 每元素一次容量检查，分支还在超越函数循环里 | 利用"`ln` 对非正/NaN/无穷输入必产出非有限值"这一性质，把域校验**移到结果上**：一次无分支 `map` + 一次可向量化的扫描；错误契约不变（仍是首个越界 bar 拒绝整次调用） | 见 38.3 |

### 38.2 孤儿逻辑：`simd_linreg*` 三个内核无人调用

`core/src/math/simd_ops.rs` 里 `simd_linreg_slope` / `simd_linreg` / `simd_linreg_angle` 三个 `pub`
内核，全仓唯一的引用方是 `core/benches/simd_statistics_bench.rs`——**没有任何生产路径调用它们**。
它们是 `pub` 且被 benchmark 引用，所以 `dead_code` 与 `check_orphan_modules.py` 都看不见，属于
"看起来活着"的死代码。

更值得记录的是它们的名不副实：名为 SIMD，实际只把 **O(period) 的初始窗口求和**向量化了，
占绝对成本的 **O(len) 递推循环仍是标量**。所以接线的收益很小，但接线仍然要做——
一是消除孤儿，二是让 `math::simd_ops` 的公开承诺是真实的。

接线时补了一个等价性前提：这三个内核**没有 leading-warmup 概念**，而生产路径
`linear::linreg_slope` 会跳过输入开头的 NaN 段。因此只在 `warm_start == 0`（无前导缺失段）
时转发，语义与原来逐位一致。

### 38.3 前后端与文档一致性

- 前端适配器 `node --test` **3/3 通过**；Rust→HTML→JS 接缝集成测试与 `rayon` 并行批测按 CI 的
  feature 组合（`--features html`、`--features rayon`）复跑通过。
- **文档与仓库状态不一致（断链）**：第十二轮 CHANGELOG 声称已删除
  `visualization/frontend/index.html.template`，但该文件在 git 里仍是 tracked、磁盘上仍在。
  本轮补上真正的删除（`git rm`）。教训是：删除动作必须能被 `git ls-files` 复验，
  不能只写进 CHANGELOG。

### 38.4 工业级评估（结论）

**结论：数值一致性已达到工业级，计算效率在绝大多数指标上已达到，剩余差距集中在 0.8–0.95 区间且可归因。**

- 数值：TA-Lib 黄金对照与语义契约测试全绿（62 个集成测试目标），本轮所有性能改写都保持逐位等价，
  唯一有意改变的是并列取最新等**本来就一致**的规则保持不动。
- 效率：90 组对拍中本阶段结束为 ✅ **71**、❌ **2**（`LINREG_SLOPE` 0.75x、`PPO` 0.79x），
  ⚠️ 17。`PPO` 是唯一反向移动的（0.86 → 0.79），本阶段未触碰它，两列差 8%（54 µs 量级）。
  **下一阶段（§39）把它连同 `MACD`/`SAR` 一起解决，最终为 ✅ 75 / ⚠️ 14 / ❌ 1。**
- 仍然低于 1.0 的部分不掩饰：`LINREG_SLOPE` / `LINREG_INTERCEPT` 的瓶颈是 `sum_xy` 的
  **约 12 周期递推链**（`sum_xy[i]` 依赖 `sum_y[i-1]`），这是算法固有依赖，不是实现偷懒；
  要突破必须换用前缀和或真·向量化递推，收益与数值漂移风险需单独评估。

## 39. 第十三轮补记：一个系统性代码生成缺陷，以及两个"假承诺"的开关（2026-10-05）

§38 处理完 8 个 ❌ 之后还剩 2 个，且 `PPO` 反向移动。继续追下去发现，真正拖住整个 crate 的
不是某几个指标，而是一个**全仓范围的代码生成缺陷**——它是靠审计源码发现的，不是靠 profiler。

### 39.1 `f64::mul_add` 在 FMA target-feature 之外会退化成 libm 调用

`f64::mul_add` 下沉到 LLVM 的 `llvm.fma.f64`。本 crate 的基线目标（x86-64，**不带** `+fma`，
Rust 默认即如此，CI 也是这样构建）没有该指令，于是 LLVM 生成的是**对 libm `fma()` 的函数调用**。
用一个 2000 万次迭代的循环携带递推直接量：

| 写法 | 实测 |
|---|---:|
| `(v - x) * k + x` | 2.6 ns/bar |
| `(v - x).mul_add(k, x)` | 3.7 ns/bar |

即**每次调用约多 1.1 ns（≈3.4 周期）**。全仓审计 `core/src`：**105 处 `mul_add`，其中 87 处在任何
`#[target_feature]` 函数之外**，也就是都在付这个调用代价。热路径包括 MACD / MACDFIX / PPO / TRIX /
KDJ / 流式 MACD 的 EMA 递推、RSI 的 `avg_gain`/`avg_loss`、SAR 更新、`variance`/`stddev` 的平方和、
以及 VIDYA / CMO / FISHER / STC 内核。

处理：74 处改为显式的 `*` + `+`（同一表达式，只多一次舍入；而且 TA-Lib 本身也是不带 FMA 编译的，
所以非融合形式反而更贴近参考实现的算术）。**确实位于 `#[target_feature(enable = "fma")]` 之内、
或可由其到达的站点保持不动**——那里 `mul_add` 是一条硬件指令，是更快的形式。

效果（10k 对拍复测）：

| 指标 | 改前 | 改后 | TA-Lib C |
|---|---:|---:|---:|
| `MACD_12_26_9` | 134.5 µs（1.06x） | **63.0 µs（2.27x）** | 142.8 µs |
| `SAR` | 71.8 µs（0.82x） | **45.0 µs（1.30x）** | 58.5 µs |
| `RSI_14` | 22.95 µs（1.29x） | **22.28 µs（1.35x）** | 30.00 µs |

### 39.2 `PPO`：0.79x → 2.45x

`ppo_with_ma_type` 是"调用两次完整 `ma()`"——两次全长分配、两次全长 store，再第三趟做除法；
而结构完全相同的 `APO` 早已融合单遍、2.59x。SMA 与 EMA 两个选择器（真正被调用的两个：`ppo`
是公式简写 `PPO:=(EMA(CLOSE,SHORT)-EMA(CLOSE,LONG))/EMA(CLOSE,LONG)*100`，TA-Lib 默认 profile
是 `matype=0`/SMA）现在融合为单遍，两条移动平均都不再物化。

两个融合内核都有**逐位一致**断言（`ppo_fused_kernels_are_bit_identical_to_the_ma_composition`）：
PPO 要喂 `matype=0` 与 `matype=1` 的黄金对照，递推上"差不多"不是可接受的门槛。

第一次融合反而更慢（58.4 → 87.7 µs）：融合后的 EMA 递推在普通函数里用了 `mul_add`，即每 bar 两次
libm 调用。现在它跑在 `#[target_feature(enable = "fma")]` + 运行时检测之内，与 `ema_inner` 一致。

### 39.3 `SAR` 的 `USE_FMA` 是个假承诺，已删除

`sar_default_into_impl` 带 `const USE_FMA: bool` 加一个 `#[target_feature(enable = "fma")]` 孪生体：
`true` 实例化走真硬件 FMA，`false` 走拆分乘加，由运行时特性检测选择。两半都站不住：

- x86 上 `false` 实例化实际不可达；而它的拆分形式正是**其它**所有 SAR 路径（流式 `SarState`、
  通用 `sar_into`）在用的——所以融合路径返回的末位不同，而这恰恰是
  `incremental_state_matches_batch_exactly` 与 `single_output_matches_with_af_projection`
  两个逐位断言在守的东西（本轮它们真的失败过一次，因此暴露）。
- 加速也不存在：同一 10k 输入上，融合形式 71.8 µs、拆分形式 45.0 µs。

这个开关等于"拿跨路径一致性换一次倒退"。现在 SAR 全部走拆分形式。（`KAMA` 保留 FMA 分派——
那里融合形式确实更快，24.7 vs 30.0 µs，且没有跨路径逐位断言。）

### 39.4 `LINREG_*`：两个候选优化实测都是"无效果"，如实记录

- 把 `1.0 / denom` 外提，让每 bar 的斜率从除法变乘法：31.03 → 31.10 µs。**无效果**——循环是
  递推链上的延迟受限，不是除法吞吐受限，`divsd` 本来就被隐藏了。
- 把每 bar 三次边界检查换成 zip 游标（三个游标由构造保证在界内）：31.03 → 31.11 µs。**无效果**。

保留 zip 改法（它减少的是工作而非增加），但**不当作收益记账**。TA-Lib 的 `TA_LINEARREG_SLOPE`
跑同样的递推、同样每 bar 一次除法、还多一对 `fabs` 做漂移守卫，却更快——所以剩下的差距不在递推
形状上，只能换前缀和或真·向量化重构，而那要用数值漂移换速度，应当单独评估。

### 39.5 工业级评估（更新）

**结论：数值一致性已达工业级；计算效率在 90 组对拍中 ✅ 75 / ⚠️ 14 / ❌ 1，唯一 ❌ 的归因明确且
两条候选修法已实测为无效。**

- 数值：TA-Lib 黄金对照与语义契约测试全绿（62 个集成测试目标 + 3051 个单测）。本轮所有性能改写
  要么保持逐位等价（PPO 有断言），要么改为与 TA-Lib 同源的非融合算术。
- 效率：❌ 8 → **1**，✅ 67 → **75**。⚠️ 区间（0.80–0.99x）全部有明确归因，无一例是"算法量级落后"。
- 仍然低于 1.0 的部分不掩饰，且把"试过但无效"也一并记录，避免下次重复劳动。
- 已知取舍并已在调用点注明：`KAMA` 的融合形式依赖宿主 CPU 特性，末位随 CPU 变化——这是有意的
  性能取舍，不是疏漏；`SAR` 则相反，为了跨路径逐位一致而统一走拆分形式。

## 40. 第十四轮补记：公式引擎自身的每次调用开销（2026-10-05）

前十三轮审计的是**数值内核**。本轮审计的是**公式引擎自己的每次调用开销**，并找到一个与数学
无关的成本：**两个编译缓存都在每次命中时深克隆条目**——`FormulaPlanCache` 克隆整个
`FormulaHotPlan`，而**默认后端**（Tree）所用的 `FormulaCache` 克隆整个 `AstNode`。两者都没有
任何收益：所有调用方都只是**借用**编译产物。

在修改前的代码上直接测得，250 根（选股窗口）时"命中"这一步在 `compile_plan` 内的成本：

| 公式 | 命中成本 (µs) | 占整次调用 |
|---|---:|---:|
| `EMA(CLOSE,12)-EMA(CLOSE,26)` | 2.55 | 38.9% |
| `RSI(CLOSE,14)` | 1.38 | 41.1% |
| `(CLOSE-MA(CLOSE,20))/STD(CLOSE,20)*100` | 3.02 | 33.8% |
| `MA5:=MA(CLOSE,5); MA20:=MA(CLOSE,20); OUT: MA5-MA20;` | 4.29 | 47.5% |

端到端（`cargo run --release -p finkit --example plan_cache_probe`，5 轮取最小值，基线与修复在
同一台机器上相隔数分钟测得，单位 µs，250 根）：

| 公式 | 计划路径 | 树路径（默认） |
|---|---:|---:|
| `EMA(CLOSE,12)-EMA(CLOSE,26)` | 6.55 → **4.13**（1.59x） | 2.82 → **2.36**（1.19x） |
| `RSI(CLOSE,14)` | 3.37 → **1.97**（1.71x） | 0.94 → **0.73**（1.29x） |
| `BOLL` | 8.95 → **5.79**（1.55x） | 3.58 → **3.06**（1.17x） |
| `MA5/MA20/OUT` | 9.02 → **4.80**（1.88x） | 4.14 → **3.29**（1.26x） |

### 40.1 修法

- `FormulaPlanCache` 改为存 `Arc<FormulaHotPlan>`，命中只做一次引用计数递增。
- `FormulaCache` 改为存 `Arc<CompiledFormula>`，新增 `get_shared`/`insert_shared`。**公开方法签名
  完全不变**——`get` 仍返回 `&CompiledFormula`，`get_cloned`/`remove` 仍返回所有权值——因此这是
  纯内部改动，不构成 API 破坏。
- 两个缓存各新增一个私有解析点（`plan_for` / `compile_shared`），供引擎自身的求值路径使用。
  **未命中路径同样零克隆**：把即将返回的那个句柄直接存入缓存（原本是"构造 → 克隆进缓存 →
  返回原件"）。
- `compile_plan` / `compile` 保持公开的"返回所有权"签名：确实需要所有权的调用方仍然拿到所有权，
  也仍然只付那一次克隆。全仓仅剩**一处**确实需要所有权（共享批量助手要把 AST 移进合成的
  `Output` 节点），该处已在代码中标注原因。
- 用 `Arc` 而非 `Rc`，`FormulaEngine` 的 `Send` 语义与改动前完全一致。

### 40.2 这些数据到底支持什么结论（不夸大）

被消除的是**每次调用的固定成本**，它主导短序列、随长度按比例摊薄。2 000 根时计划路径提升
1.12~1.36x、树路径 1.02~1.19x；10 000 根时表面上的 1.06~1.19x **不可信**——基线那一轮整机偏慢，
在**与改动无关**的 `cold(compile+eval)` 列上同样可见（基线 1.9~2.6 ms vs 修复后 1.7~2.1 ms）。
跨轮漂移有这个量级，正是探针取"5 轮最小值"而非均值的原因，也是**只把 250 根那两行（效应
20%~90%，远在漂移之外）当作结论**的原因。

### 40.3 探针还暴露了一个不好看的事实：计划后端比它要替代的解释器更慢

在**所有**测过的长度上，计划后端都慢于树解释器：

| | 250 根 | 2 000 根 | 10 000 根 |
|---|---:|---:|---:|
| 计划路径 | 4.13 | 18.07 | 79.92 |
| 树路径（默认） | **2.36** | **11.16** | **58.10** |

原因在第十三轮就已可见：计划里**每个节点都会 materialise 一条全长序列**，所以 10 000 根下
`EMA(CLOSE,12)-EMA(CLOSE,26)` 要 58~80 µs 才算两条递推。反倒是"应该更慢"的参考实现没有这个
问题。**要让计划路径真正取胜，需要在计划上做融合或缓冲生命周期分析（即计划级优化），不是
局部微调。** 在此之前，默认保持 `Tree` 是正确设置，而不是待办事项。

### 40.4 仍然未改（并说明理由）

输出物化在"多名字共用同一缓冲"时会拷贝两次：`read_slot` 先克隆进 `already_read` 再克隆取出，
索引 0 还会 `primary.clone()`。而多名字共用保留缓冲是**文档化的正常情形**（`DIF`/`DEA`/`MACD`），
所以这些拷贝是真实的——但要省掉第二次，就得对外交出共享缓冲，会改变 `FormulaPlanOutput` 的
公开形态。故不并入本轮。

### 40.5 工业级评估（第十四轮后）

前一轮结论不变，补上公式引擎维度：

- **已解决**：公式引擎的"每次调用开销"在选股长度（250 根）下不再由缓存维护主导——计划路径
  1.55~1.88x、默认树路径 1.17~1.29x。
- **未解决（已定位、已给方向）**：计划后端慢于树解释器，属**计划级优化**（节点融合 / 缓冲
  生命周期），建议单独立项。这不是"差一点点"，而是一个方向性判断：在它修好之前，不应该把
  默认后端切成 `Plan`。
- 探针 `core/examples/plan_cache_probe.rs` 已保留，上表数字可复现，而不是"声明"。

## 41. 第十五轮：同一条公式，五个答案（2026-10-06）

前四轮问的是"每条路径**跑多快**"。本轮换了个问题：**同一条源码，在每条路径上给出的是不是同一
个答案？** 答案是不是。四个缺陷，三个是静默的——它们全都是在把一条公式跑遍**所有公开入口**
时暴露的，而不是靠给某一条路径做性能剖析。

复现方式：`cargo run -p finkit --example backend_divergence_probe`。

### 41.1 五个执行入口用了错误的优化器

`FormulaOptimizer::optimize` 会做**语句级死代码消除**；`optimize_for_execution` 刻意不做，
理由写在它自己的文档注释里：`X:=...` 赋值可通过 `FormulaContext::variables` 观测，所以语句级
DCE "只对显式的 lazy/optimizer 用法有效，对正常的编译执行契约无效"。

但有五个**执行**入口在调用 `optimize`：

| 入口 | 原 | 现 |
|---|---|---|
| `FormulaCompiler::compile` | `optimize` | `optimize_for_execution` |
| `FormulaEngine::compile_bytecode` | `optimize` | `optimize_for_execution` |
| `FormulaEngine::eval_optimized` | `optimize` | `optimize_for_execution` |
| `FormulaEngine::eval_jit` | 在**已优化**的 AST 上再 `optimize` | 去掉第二遍 |
| `FormulaEngine::compile_jit` | 同上 | 去掉第二遍 |

两个 JIT 入口比"用错 pass 集合"更糟：它们拿到的是 `compile_shared` 已经**为执行优化过**的 AST，
又在上面跑了第二遍包含 DCE 的优化器。第二遍直接删除；第一遍本来就是它们需要的。

两个后果，第二个是严重的：

- **副作用**：`JUNK:=MA(CLOSE,5); OUT: CLOSE;` 经 `eval` 发布 `["JUNK","OUT"]`，经
  `eval_optimized` / `FormulaCompiler` 只有 `["OUT"]`。`ctx.variables` 是**文档化契约**
  （各语言绑定用它构建结果字典），删掉赋值不是优化，是另一个结果。
- **返回的数值**：被删的语句同时丢掉它会追加到 `FormulaContext::string_table` 的字符串字面量，
  而字面量求值为**它在表中的下标**。于是删掉靠前的语句会移动后面每一个字面量的下标：

  | 源码 | `eval` | `eval_optimized` | `FormulaCompiler` |
  |---|---:|---:|---:|
  | `TMP:='HELLO'; OUT: 'WORLD';` | **1** | **0** | **0** |

  副作用表错是 bug；**主结果错是错数**。

该保留 DCE 的四处调用没有被碰（`FormulaOptimizer::optimize` 仍是公开 API，
`core/benches/formula_bench.rs` 仍在用）。变的是：**任何"执行"路径都不再用它**。

### 41.2 `FormulaCompiler` 在未命中路径上也深克隆

第十四轮去掉了两个缓存的**命中**克隆。本轮去掉 `FormulaCompiler::compile` 的**未命中**克隆：
原本是"构造 → `insert(source, formula.clone())` → 返回原件"，那次 AST 深拷贝只为了让缓存持有
所有权。缓存现在存 `Arc`，未命中路径存入的就是它将要返回的那个句柄。`compile` 的
`-> CompiledFormula` 签名不变，所以**一次**拷贝仍是固有的（调用方要求持有所有权），只是不再有
第二次。

### 41.3 计划后端跑不了"状态语句之后的字符串字面量"

`FormulaComputeLowerer::add_effect` 会把**上一个效果**追加进依赖表，纯粹为了给有状态语句排序。
`STRING_LITERAL` 走的是 `add_effect`，所以跟在赋值后面的字面量就带上了这条**控制边**——而
`HotExecutionPlan` 把**每一条**语义依赖都变成缓冲输入。`STRING_LITERAL` 内核又有自己的元数检查，
于是计划后端直接失败：

```
FormulaEngine::new().with_execution_mode(Plan)
    .eval("TMP:='HELLO'; OUT: 'WORLD';", &mut ctx)
=> RuntimeError("... kernel dispatch failed for kernel 0x43f39d3d91f40744 with code 2")
   而树后端返回 1
```

现在字面量改用 `add_node` 下沉——不带控制边。这样做是成立的：该节点**什么都不读**，且对**在哪
运行不敏感**——它的值是计划烘焙进参数区的下标，而调用方按 `string_literals_ordered()`（以 node id
即下沉顺序为键，而非执行顺序）预先填好 `string_table`。`effect: Stateful` 是**故意保留**的：
CSE 绝不能合并两个相同字面量，否则字面量计数变化会破坏那些烘焙下标，报
`LiteralBindingMismatch`。

既有的 `formula_string_literals.rs` 门禁没抓到它，因为它的四个用例都是**函数调用带字面量实参**
（`EM_REF("IDX", 1)`）；这条链只在"字面量跟在有状态语句之后"时出现。

### 41.4 字节码 VM 不认"赋值遮蔽内置别名"

`A` 是 `AMOUNT` 的正式别名——`builtin_data_aliases_resolve_to_the_same_series_on_every_path`
在**每一条**路径上都这么断言。但 `BytecodeCompiler::normalize_variable` 无条件把 `A` 的读取改写成
`AMOUNT`，于是"自己给 `A` 赋过值"的公式被别名顶掉：

| 路径 | `A:=CLOSE*2; B:=A+CLOSE; OUT: B;` |
|---|---|
| `eval`（树，默认） | 43.05 |
| plan | 43.05 |
| `FormulaCompiler` | 43.05 |
| 字节码 VM | `RuntimeError("Data not available: AMOUNT")` |

而 `A` 恰恰是通达信公式里**最常用的用户变量名**，所以这不是边角。编译器现在记录公式绑定过的名字
（`X:=...`、`X: ...`、`for X = ...`），绑定的名字解析为公式自己的变量；未绑定的名字仍原样走别名表。
声明记录在**值编译之后**，因此没有前置 `A` 的 `A := A + 1` 右值仍然读别名，与树路径一致。

### 41.5 门禁自身的一个盲区

`formula_differential_tests.rs` 的 `run_plan` 调的是 `executor.execute`，它从 `inputs.first()`
推导序列长度，因此拒绝**没有任何绑定输入**的计划。于是所有**纯常量公式**在计划路径上都无法被
测试——而计划后端自己的 `eval_plan_channels` 用 `ctx.data_len` 处理这个情形。也就是说**门禁比生产
更窄**，而不是引擎有差。该辅助函数现已对齐生产（`execute_range` + 同一长度规则）。

### 41.6 新增的常驻门禁

`every_execution_entry_point_agrees_on_assignments_and_string_table`：把一批"赋值形状"的公式
依次跑过 `eval`、`eval_optimized`、字节码、JIT、`FormulaCompiler` 与计划路径，**同时**断言返回值
与 `ctx.variables` / `ctx.string_table`。

本仓其它所有差分门禁**只比对返回值**——这正是 41.1 与 41.4 这两个缺陷能挺过四轮差分测试的原因。

### 41.7 工业级评估（第十五轮后）

- **数值维度**：不变（工业级）。并且本轮把"逐位/逐值一致"的覆盖面从"返回值"扩到**副作用**，
  即契约本身更严了。
- **效率维度**：全量重跑 90 项配对基准，本轮为 **✅72 / ⚠️16 / ❌2**（上一轮同套件为 75/14/1）。
  本轮与上一轮**都没有改动 `core/src/math/` 下任何指标内核**，所以比值的变化只可能来自测量噪声，
  两个证据：`floor` 33.66µs / `ceil` 31.19µs（我方相差 8%，都稳定）而 TA-Lib 侧为 25.23µs / 30.32µs
  （C 侧自身相差 **20%**）——0.75x 是 TA-Lib 的快速样本落在了 `floor` 上；`linreg_slope_14` 仍是
  0.75x，我方 30~33µs 与第十三轮记录的 31.03µs 一致，是那条 loop-carried `sum_xy` 递推的延迟下限，
  两次候选优化都实测为 no-op。**结论：慢于 TA-Lib C 的指标集合没有变化，两项跨过 1.25x 警戒线是
  参考侧测量移动所致。**
- **工程维度**：增强。新增一条跨入口门禁，堵住了"只比数值、不比副作用"的长期盲区。
- **仍未解决**：41.3 的根因是"热计划把控制依赖也当作数据输入"。本轮在**产生那条边的地方**
  （下沉器）消除它，而不是在消费侧过滤——彻底分离控制边与数据边需要给语义图加一类边并在
  规划器/生命周期分析中贯通，属结构性工作，建议单独立项。

## 42. 第十六轮：把"没有代码改动"当成"没有差距"是一个错误转向（2026-10-06）

### 42.1 上一轮的结论，以及它错在哪里

§41.7 写下了这句话：

> 本轮与上一轮**都没有改动 `core/src/math/` 下任何指标内核**，所以比值的变化只可能来自测量噪声。

前半句是事实，后半句是推论，而这个推论是**从"改了什么"推出来的，不是从"测到了什么"推出来的**。
它对 `floor` 的判定——"0.75x 是 TA-Lib 的快速样本落在了 `floor` 上"——只对了一半：

- 对的一半：TA-Lib 的 `TA_FLOOR` 也在调用 `floor()`，所以两侧都在为 libm 调用付费；C 侧
  `floor` 25.23µs 与 `ceil` 30.32µs 相差 20%，我方 33.66µs 与 31.19µs 相差 8%，这个不对称确实是
  代码布局。
- 错的一半：*因此没有可修的东西*。一个逐元素单行循环要花 25µs，这个数字本身就该被追问，
  而不该被"两侧都在付同样的代价"解释掉。

### 42.2 根因：`f64::floor` 在基线 x86-64 上不是指令

`llvm.floor.f64` / `llvm.ceil.f64` 在默认 target 下**不会**下沉为 `roundsd`——它是一次
**out-of-line 的 libm 调用**，每个元素一次。`FLOOR` / `CEIL` 是纯逐元素变换，所以这次调用**就是**
指标本身。

用本仓已编译的内核直接量（`core/examples/rounding_kernel_probe.rs`，10k 点，5 次取最小）：

| 内核 | µs/次 |
|---|---:|
| `iter().map(|&x| x.floor()).collect()`（改造前的写法） | 24.95 |
| 标量 `for` 循环 + `x.floor()` | 22.19 |
| AVX2 `_mm256_floor_pd`（新增 `simd_ops::simd_floor`） | 1.37 |
| AVX2 `_mm256_ceil_pd`（新增 `simd_ops::simd_ceil`） | 1.39 |

`vroundpd` 执行的是**完全相同**的 IEEE 运算，一次四条 lane，并且保留全部边界情形：`NaN` 传播、
`±inf` 保持、`±0` 保留符号。所以这不是近似，是**逐位相同**的内核——探针里直接断言了
`to_bits()` 相等，覆盖 `NaN` / `±inf` / `±0` / `-0.5` / `1e300`，而不是在注释里论证。

### 42.3 同一类问题：整段串行扫描

`math::simd_ops::simd_first_non_finite` 是"这个序列里有没有 `NaN` 或 `±inf`"的向量形式：
一次比较四条 lane，只有某条 lane 变脏时才回退到逐元素。它替掉了三处各自独立的
`iter().position(|v| !v.is_finite())`：

| 位置 | 改造前 |
|---|---|
| `math::moving_avg::reject_if_non_finite`（`EMA`/`WMA`/`DEMA`/`ema_multi_periods` 每次调用都走） | 一次全量串行扫描 |
| `math::fast_moving_avg::reject_if_non_finite` | 一次全量串行扫描，**再来一次**从 warm-up 边界的扫描 |
| `indicators::math_transform::ln` | 对**结果**的一次全量串行扫描 |

`WMA` 是可见的那个（0.906x）：它自己的核是两趟，这次校验是白送的第三趟。

### 42.4 一条规则三份实现

上面这条规则原先有三份实现、三种形状。现在只有一份：`math::moving_avg::reject_if_non_finite`
（`pub(crate)`），`fast_moving_avg` 委托给它。委托顺带修掉了它的 warm-up 行为——旧副本从下标 0
开始扫，发现 0 非有限，再从 warm-up 边界重扫，**一个答案两趟全量**。

同样的合并做了两处：

- `indicators::momentum` 里 `calc_di_dx` 与 `dx_from_state` 是同一个 `+DI`/`-DI`/`DX` 三元守卫的
  两份手抄件（`compute_adx_family` 一份、`compute_adx_only` 一份）。现在是一份 `di_dx_from_state`。
  这种重复的危险不在字符数，而在**两份会漂移**。
- `indicators::math_transform` 里 `ln` 校验**输出**、`log10` 校验**输入**——同一个定义域、同一份
  错误契约、两种方言。现在都用 `map_positive`，`log10` 的逐元素输入循环消失。

### 42.5 被内核立刻覆盖的全长 NaN 填充

五处分配 `vec![f64::NAN; len]` 然后把 `lookback` 起的每个槽都写一遍。这次填充不免费——它是
一趟全长的 store，落到的内存内核紧接着就丢掉了。`utils::uninit_output` 把"调用方写入每个槽"
这个契约讲一次，`unsafe` 与 SAFETY 说明也只有一处，而不是五处：

| 位置 | 缓冲 × 长度 | 填充代价 |
|---|---|---|
| `overlap::bbands` | 3 × len | 1M 点基准上是 24 MB |
| `momentum::compute_adx_family` | 3 × len | 10k 是 240 KB，1M 是 24 MB |
| `momentum::compute_adx_only`（ADXR） | 1 × len | 1M 是 8 MB |
| `momentum::aroon_into` | 2 × len | 10k 是 160 KB |
| `math::moving_avg::sma_inner` 的校验 | — | 换成 §42.3 的向量化探测 |

注意 `overlap::bbands` 有一条**早退路径**：`start > 0` 且有效尾长不足一个窗口时递归根本不跑，
此时必须整段回读为 NaN。该分支的填充宽度因此收窄为"仅当递归不跑时才是 `len`"，而不是简单删掉。

### 42.6 LINREG 家族的循环不变量除法

`linreg_slope` / `linreg_intercept` / `linreg` 每个槽都对循环不变量做一次除法——斜率除以
`denom`、截距除以窗口长度。两处都改成倒数乘法：一次额外的舍入，比该家族 1e-8 金标准宽六个数量级，
而且**这正是 `linear.rs::linreg_slope` 标量尾段早就在做的事**。同一份核在同一个量上既做除法又做
乘法，是全仓唯一一处这种不一致。`simd_ops::simd_linreg_slope` / `linreg_slope_scalar` /
`linreg_scalar` / `linreg_avx2` 现在与它一致；`linreg_avx2` 也不再每次调用分配一个
`Vec<f64>` 存 lane 下标。

### 42.7 一条被门禁抓住的自造错误：`#[expect]` 不生效

本轮新加的 `#[expect(clippy::uninit_vec)]` 在 `overlap::bbands` 与
`momentum::compute_adx_family` 上被 CI 口径的 clippy **判为未生效**（`RUSTFLAGS="-D
unfulfilled_lint_expectations"`），而在 `utils::uninit_output` 上生效。用探针逐一验证过
`set_len` 的七种书写形态（单条/多条 `unsafe` 块、早退后声明、多缓冲、带类型标注……）都无法复现
那两个函数的差异，也就是说这不是"写法问题"而是"位置问题"。

结论不是去猜 clippy 的实现，而是**换设计**：需要这个模式的地方改为调用
`utils::uninit_output`。副作用比原方案更好——`unsafe` 块从五处收敛到一处，`#[expect]` 只挂在
一个**已经被证实会触发**的函数上。

### 42.8 工业级评估（第十六轮后）

- **数值维度**：不变（工业级），且本轮的每一处改动要么逐位保持算术（`floor`/`ceil` 对
  `f64::floor`/`f64::ceil` 做逐位断言）、要么是一次落在 1e-8 容差内的舍入、要么纯属内存管理。
  全量回归：**3965 个测试、0 失败、62 个 target**，含 `golden_talib_tests` 对 TA-Lib 的逐值夹具。
- **效率维度**：提升。全量重跑 90 项配对基准为 **✅75 / ⚠️12 / ❌3**（上一轮 72/16/2）；
  `ceil` 31.19→1.54µs、`floor` 33.66→2.16µs（对 TA-Lib C 分别为 17.79x / 12.64x），`ln` 0.97x→1.63x，
  `bbands_20@1M` 0.87x→1.28x，`sma_20@1M` 0.98x→1.14x。
- **但全量数字不能当结论用**（§42.9）：本轮把 15 个非 ✅ 指标单独复测（3s warm-up / 6s 窗口），
  结果与全量列**方向不一致**——全量列偏乐观。诚实读法是：`ADX`/`ADXR`/`AROON`/`AROONOSC`/`ULTOSC`/
  `VAR`/`MAX`/`MIN`/`LINREG*` 落后 **0–30%**（而非 0–5%），唯一的真 ❌ 是 `linreg_slope_14`（两列都慢）；
  全量里的 `tanh`(0.77❌) 与 `aroonosc_14`(0.71❌) **在复测中翻向**（0.53–0.82 / 1.25），是测量噪声。
- **工程维度**：提升。两条规则的三份重复实现各自收敛为一份；"写入每个槽"的分配模式的 `unsafe`
  与 SAFETY 论证从五处收敛到一处。
- **仍未解决**：`ADX`/`ADXR`/`AROON`/`AROONOSC`/`ULTOSC`/`MAX`/`MIN`/`VAR`/`LINREG*` 落后 **0–30%**
  （§42.9 修正了全量列的读数），而它们的内核**已经**是手工优化过的（单调队列、van Herk–Gil–Werman 分块
  扫描、缓存下标）。剩下的是每根 bar 上 `high[i]`/`low[i]`/`out[i]` 的边界检查——C 侧没有对应物。
  要关掉它，每个循环都得改成 `linreg_slope` 那种 zip 游标写法。这是**逐个循环、逐个风险**的工作，
  因此单独立项，不夹带进本轮。唯一的真 ❌ 是 `linreg_slope_14`（1.30x）。

### 42.9 测量方法学：单跑比值在 1.25x 以下不可信

本轮把 15 个非 ✅ 指标单独复测，得到两个方向相反的结论，都写进了 CHANGELOG：

1. **复测不是"洗白"**。单独跑时整组落在 1.0–1.3x，而不是全量列的 0.7–1.0x。所以"落后多少"的诚实
   读数是 **0–30%**，不是"已接近持平"。全量列对这批指标偏乐观。
2. **`tanh` 与 `aroonosc_14` 是噪声，且可证**。我方 `tanh` 三次运行 59.23 / 37.15 / 24.20µs，
   TA-Lib 侧 45.83 / 45.56 / 45.62µs——0.2% 对 145%。Criterion 的置信区间在**单次运行内**很紧
   （如 `[35.274, 40.395]`），所以这是**进程间漂移**而非采样误差，而且是**单侧**的：动的是我方那一列。

结论：`docs/BENCHMARK_REPORT.md` 应作为**筛查**产物，而不是判决书；接近 1.0 的比值要单独复测。
这条方法学结论本身是本轮的产出之一——上一轮的 §41.7 正是在缺少它的情况下得出了错误推论。

### 42.10 顺带发现的、早于本轮的 CI 红灯：8 条断链

跑**完整**门禁矩阵（而不是只跑前几轮碰过的那几个）才发现 `scripts/check_rustdoc.sh` 失败，
8 条 intra-doc link 无法解析。`git blame` 全部指向 2026-10-04（`db78064`、`ca6d3f16`），
即**早于本轮两个 pass**——`doc` 这个 CI job 从那时起就是红的，没人发现。

规律很明确：**全部出现在模块级（`//!`）文档注释里**。同一个 `NodeKind` 链接写在隔屏的正常 `///`
注释里（`core/src/semantic_graph.rs:394`）能正常解析，所以问题不在链接文本，而在 rustdoc 解析模块
文档时所处的 scope。现在每条都改成显式的 crate 绝对路径——裸的 `NodeKind` 写成
`crate::semantic_graph::NodeKind`，其余同理——在任一 scope 下都无歧义。

`core/src/compute.rs:162` 是另一类错误：`[`Self::FixedLookback(0)`]` 链接的是一个**变体实例化**，
不是 item。现在降级为纯代码跨度，与周边已经不加链接的 `FixedLookback(_)` 风格一致。

| 文件 | 修复的链接 |
|---|---|
| `core/src/semantic_graph.rs` | `NodeKind`、`ArtifactHash`、`SemanticGraph::content_hash`、`SemanticNode::inputs` |
| `core/src/runtime_context.rs` | `BufferArena`、`StateArena`、`RuntimeContext` |
| `core/src/formula/compute_ir.rs` | `SemanticGraph::content_hash` |
| `core/src/compute.rs` | `Self::FixedLookback(0)` 降级为代码跨度 |

纯文档改动：不动运行时行为、不动 API 面、无可测代价。

### 42.11 门禁矩阵（本轮结束时全绿）

`cargo fmt --check`；`RUSTFLAGS="-D warnings" cargo check --workspace --all-targets --locked`；
两个 clippy 调用（`RUSTFLAGS="-D unfulfilled_lint_expectations"`）；`cargo test -p finkit --tests
--no-fail-fast`（**62 target / 3965 通过 / 0 失败 / 3 ignored**）；rustdoc 策略门禁（§42.10）；
21 个 `scripts/check_*.py` 内务门禁。上一轮红的就是 rustdoc 那一项。

## 43. 第十七轮：先修量具，再修内核（2026-10-06）

本轮的任务是"深度对比 TA-Lib、输出效率对比文档、继续优化慢指标、修完历史遗留"。诚实的结果是：
**一处优化成立、三处被测量否决、两处历史遗留属实、唯一真 ❌ 的归因被推翻。**
详细对比文档见 `docs/talib-efficiency-deep-dive-zh.md`。

### 43.1 量具先出问题了

本轮第一版探针（`core/examples/kernel_hotspot_probe.rs`）把 `*_old` 写成**写入调用方预留缓冲
区**（零分配），把 `*_new` 一侧通过**公开函数**调用（每次分配 80 KB）。于是分配成本整笔算在
新代码头上，三处微改造全部显示为"回退"。

修正后两侧承担各自真实的分配，并额外加一档 `_into` 内核级对比（两侧都不分配）。同时加**空对照**：
把已发布的函数体原样抄一份当作"改造前"，两侧代码相同，比值即探针自身分辨力。

| 空对照 | 比值 |
|---|---:|
| `AVGDEV_14` | **0.995×** |
| `LINREG_SLOPE_14` | **1.042×** |

**分辨力 0.5%–4%，且逐内核不同。** 探针与被测代码同为仓内产物，所以它被提交（先例是
`core/examples/rounding_kernel_probe.rs`），本轮所有"更快/更慢"的结论都可复现。

### 43.2 `PERCENTRANK`：排序窗口换回计数 — 4.584×，逐位相同

`TA_PERCENTRANK` 要的是一个**整数**（前序 `timeperiod` 个观测中严格小于当前值者）。原实现每根
bar 维护有序窗口：二分查找 + `copy_within` 搬移约半个窗口 + 插入。整数计数两条路径**逐位相同**，
所以这是**同一谓词的换写**，不是浮点重结合，无需容差辩护。

| 实现 | 10k bar | ns/bar |
|---|---:|---:|
| 有序窗口 + 二分 + 搬移 | 309.40 µs | 30.94 |
| AVX2 直接计数（`simd_ops::simd_count_less`） | 67.50 µs | 6.75 |

**4.584×**，探针断言 `10000/10000` 槽逐位相同。全量配对列 `percentrank_30` 由
**1.08× → 4.64×**（293.76 → 70.90 µs）。含 `NaN` 的输入仍走原有序路径——原式用
`partial_cmp().unwrap_or(Ordering::Equal)`，缺失值无确定位置，两条路径**不要求**一致，
保留已发布行为而非重新定义。

### 43.3 `AROON` / `AROONOSC`：三份手抄递推合并为一份，并变快

`AROON`、调用方自持的 `aroon_into`、`AROONOSC` 各有**一份手抄的同一递推**，且 `NaN` 行为互不
相同。现由 `ExtremeTracker` 一处承载，三条入口只差"从同一状态取哪个输出"。

**关键约束是必须保持原实现的单趟形状。** 第一版抽成"每条腿一次 helper、调用两次"，即两趟；
探针立刻测出内核级 **0.790×**。改回双腿同趟后：

| 对比 | 上一版 | 本版 | 比值 |
|---|---:|---:|---:|
| `aroon_14`（含分配） | 97.30 µs | 93.10 µs | **1.045× 更快** |
| `aroonosc_14` | 55.70 µs | 52.40 µs | **1.063× 更快** |
| `aroon_into_14`（内核级，无分配） | 51.30 µs | 56.50 µs | 0.908× 更慢 |

**不四舍五入地记录**：基准与上层走的路径变快，裸 `_into` 内核慢 9%，而 9% 落在本探针已量出的
逐内核分辨力带（0.5%–11.6%）内，因此**不计入收益**。数值：三条输出在 `1e-12` 下 **0 处失配**。
同趟写法顺带取消了全长 `NaN` 填充（循环写满每个槽，正是 `utils::uninit_output` 的契约）。

全量重跑：`aroonosc_14` 脱离 ❌ —— **0.71× → 0.85×**。

### 43.4 `AVGDEV`：两次改造都更慢，回退并写入源码

| 候选 | 结果 |
|---|---|
| 环形下标替代 `Vec::remove(0)` + `push` | 更慢 |
| AVX2 `Σ|x - mean|` | 更慢 |
| 空对照（用于判断是否噪声） | 0.995× → 差异真实 |

原因可由尺寸直接推出：period 14 的窗口是 **13 个 double**，被省掉的 `remove(0)` 是 104 字节、
本来就在 L1；向量形式把 14 次串行加法换成 3 次整通道加法**外加一次水平归约与一次运行时派发**。
**此处没有可挖空间**，保留直白写法，并把负面结果写进源码注释。

### 43.5 `LINREG_SLOPE`：指针游标恰好中性，§42.8 的归因被**证伪**

`linreg_slope_14` 是报告唯一的真 ❌（0.76×，两轮运行同一数字）。§42.8 把它归因于**每 bar 的
越界检查**，并预告"关掉它需要把每个循环都改成 `linreg_slope` 那种游标写法"。

在**相同播种、相同分配**下只改循环体：

| 循环体 | 10k bar | 比值 |
|---|---:|---:|
| 索引（现行） | 30.40 µs | — |
| 指针游标 | 30.40 µs | **1.000×，0 处失配 @1e-12** |

**恰好中性**——LLVM 已经消掉了那些检查。该改造因此**不落地**：不是因为它更慢（此前"0.593×"
是 §43.1 那个有偏探针的产物），而是因为**没有收益**。§42.8 的假设到这里从"存疑"变为"已被
检验并证伪"，后续接手者不必再试。

### 43.6 历史遗留一：两个"只存在于名字里"的 `CCI` SIMD 路径

```rust
fn cci_period14_into_impl<const USE_AVX2: bool>(...) {
    let _ = USE_AVX2;   // 参数从不被读取
    ...两条分支是同一份标量代码...
}
```

调用点 `is_x86_feature_detected!("avx2")` 的两条分支产出完全相同的工作。`math/simd_kernels.rs`
里还有结构相同的第二处：`cci_simd_into` 判断 `has_avx2()` 后两条分支都指向同一标量函数。
读者会合理推断 period-14 路径已向量化，从而不再去找它。

修法不是"把向量化补上"，而是**删掉参数并写明为何不该补**：向量化要把 `Σ|x - mean|` 重结合到
通道，窗口偏差趋近 0 时末尾除法会放大该差异，而 `CCI` 与金标准是**绝对 `1e-8`** 比对。测量同意
——AVX2 形式在 10k bar 上**慢 18%**，且把 10000 个值里的 **13 个**推出 `1e-8`。

全仓扫描确认这是该模式的最后一处：`let _ = <被忽略的 const 泛型>` 现为零处。
`fast_moving_avg.rs` 的 `USE_FMA` 形似而**非**同类——两分支 `mul_add` 与 `*`+`+` 真的不同，
且两个值都被实例化，注释已写明"不要合并"。

### 43.7 历史遗留二：一个指标，两个答案

`CCI` 的 period-14 路径对"零偏差窗口"写 `0.0`，通用路径写 `NaN`——**同一指标因 period 不同而
给出不同的值**。`TA_CCI` 写 `0.0`，两条路径现已统一。

### 43.8 全量重跑（90 项配对）

**✅ 74 / ⚠️ 15 / ❌ 1**（上一轮 75 / 12 / 3）。尾部形状的变化才是重点：

| 上一轮的 ❌ | 本轮 | 成因 |
|---|---|---|
| `tanh` 0.77× | **1.68× ✅** | 测量噪声：我方三次独立进程 59.23 / 37.15 / 24.20 µs，C 侧 45.83 / 45.56 / 45.62 µs |
| `aroonosc_14` 0.71× | **0.85× ⚠️** | 内核重构（§43.3） |
| `linreg_slope_14` 0.76× | **0.76× ❌** | 真落后，两轮同一数字 |

1M 组本轮整体上移（`sma_20@1M` 1.14× → 0.85×、`rsi_14@1M` 1.06× → 0.97×），而同指标在 10k/100k
两档稳定在 1.3–1.8×：**该档已进入内存带宽主导区间，其可重复性低于它自己的三档差异**，
不宜用于评价内核，但必须保留（它是唯一能暴露大输入下算法退化的档）。

### 43.9 工业级评估（第十七轮后）

- **数值维度**：不变。落地的改动要么逐位相同（`PERCENTRANK` 整数计数，10000 槽全断言），
  要么在 `1e-12` 下验证相等（`AROON` 族），要么是向 TA-Lib 行为靠拢（两处 `CCI`）。
- **效率维度**：尾部改善。❌ 3→1；`percentrank_30` 1.08×→4.64×；`aroonosc_14` 0.71×→0.85×；
  `tanh` 0.77×→1.68×（噪声，现已定性）。
- **工程维度**：`AROON` 族三份重复递推收敛为一份共享类型；两处假 SIMD 分支删除；
  两条负面结果写进源码，避免后人重造。
- **仍未解决**：`linreg_slope_14`（0.76×，成因已从"误判"变为"未知"），以及聚集在 **0.82–0.98
  窄带**里的 12 个内核（`ADX`/`ADXR`/`AROON`/`AROONOSC`/`ULTOSC`/`MAX`/`MIN`/`VAR`/`LINREG*`/
  `TRIMA`/`WMA`/`AD`）。**同一窄带、同类内核形状，说明这是一个有共同成因的现象，而不是 12 个
  独立缺陷**；本轮在 `AROON` 上一试即中的 `zip` 写法在 `linreg_slope` 上完全无效（§43.5），
  所以不要指望一个配方。下一步应先为这条窄带找一个算子级共同解释，再决定是否逐项改造。

### 43.10 门禁矩阵（本轮结束时全绿）

`cargo fmt --all`；`RUSTFLAGS="-D warnings" cargo check --workspace --all-targets --locked`；
两个 clippy 调用（`RUSTFLAGS="-D unfulfilled_lint_expectations"`）；`cargo test -p finkit --tests
--no-fail-fast`；`cargo test -p finkit --doc`；rustdoc 策略门禁（§42.10 修的 8 条断链保持绿）；
22 个 `scripts/check_*.py` 内务门禁。

## 44. 第十八轮：把"哪一种写法"问成可测量的问题（2026-10-06）

§43.9 留下的判断是"0.82–0.98 窄带里的 12 个内核有共同成因，下一步应先找算子级解释"。
本轮先把 §43 自己的量具修到能回答这个问题（§44.1），再按它的读数动手。三处落地，一处被否决，
并顺手把 LINREG 家族在仓内的**七份递推**收敛到同一方向（§44.5）。

### 44.1 同进程探针的第二版：这次它先证明自己

§43 的探针（`core/examples/kernel_hotspot_probe.rs`）把两侧放进同一二进制、交错运行，
这一版（`core/examples/talib_gap_probe.rs`）保留该结构并做两处修正：

1. **计时区间批量取块均值**，不再逐次调用取最小值。单次调用是 16–120 µs，与
   `Instant::now()` 自身、一次中断、一次分配器慢路径同量级，取最小值等于取噪声下界。
2. **空对照放在程序开头**和**结尾**各跑一次，让读者看到探针自己的漂移，而不是要求读者相信它。

修正后的空对照：

| 空对照 | 开头 | 结尾 | 分辨力 |
|---|---:|---:|---|
| `line slope`（同代码两侧，无分配） | **1.008x** | **0.996x** | **1%** |
| `aroon`（同代码两侧，各两个缓冲） | 0.963x | 0.887x | 8 个点 |

第二行本身就是结论的一半：**同一个函数测两次，差 8 个点**，而两行唯一的区别是分配形状。

### 44.2 公开层（section C）会系统性低估"分配次数多"的实现 —— 本轮最重要的一条负面结论

本轮唯一与 Criterion 冲突的读数是 `adx_14`：交错探针给 **0.742x**（我方 104.27 µs，
C 77.40 µs），而 Criterion 给 **1.008x**（`FTA_ADX_14` 74.28 对 `TALib_ADX_14` 74.87 µs）。

两者构不出"谁读错了"的简单解释：**两个工具构造 C 侧的方式完全相同**
（都是 `vec![0.0; len]` + 一次 C 调用），空对照又证明 1% 分辨力。差别在**分配形状**：
我方每次调用做三次分配，C 侧一次，而交错运行迫使分配器在两种 free-list 模式之间交替服务；
Criterion 的长跑反复重复同一种请求模式，free list 保持热。

**因此 section C 的比值只适用于两侧都能做成零分配的场合。** 该限制已写进探针的模块文档。
公开层比值以 `docs/BENCHMARK_REPORT.md` 为准——这条规则在 §43.1 已用另一种方式教训过一次，
本轮是第二次，所以这次把它写进代码而不是只写进文档。

**顺带撤回一条上一轮的结论。** §43 记录"`FTA_AROON_14`/`FTA_ADX_14` 与报告错配"。
本轮直接查 `target/criterion/directional_vs_talib/`：`fta_adx_14`、`talib_adx_14`、
`fta_aroon_14`、`talib_aroon_14` 四者齐全，`aroon_14` 现场读 58.97 / 49.65 µs = 0.842x，
与报告的 0.84x 一致。**报告不存在错配**；那条"发现"是有偏探针的第三个产物。

### 44.3 `LINREG_SLOPE`：真因是权重方向，不是越界检查，也不是类型转换

§43.5 用"索引 vs 指针游标"证明越界检查是**恰好中性**（1.000x），把成因从"已知"降级为"未知"。
本轮把未知变成已知：

| 对比 | 比值 | 读法 |
|---|---:|---|
| `previous vs cast-hoisted` | **1.000x** | 循环里的 `usize as f64` 早被 LLVM 提升 —— **不是成因** |
| `hoisted vs new-shipped` | **0.678x** | 换方向后**快 1.47x** |
| `new-shipped vs C` | **1.026x** | 用 Rust 写那个递推**反超 C 库**（上一轮同项读数 1.048x） |

即：不是"补回差距"，而是"越过参照物"。公开层随之从 0.837x / 0.892x 变为
**1.129x / 1.105x**（`line slope` / `line intercept`），`linear_reg` 从 1.714x 变为 **2.038x**。

方向为什么载荷这么重：**最老 bar 带 `period - 1`** 时每 bar 的推进是

```text
sum_xy += sum_y - period * old          // 一次加法接进累加器
```

而相反方向需要

```text
sum_xy += (period - 1) * new - (sum_y - old)   // 两次相关减法夹在 sum_y 与累加器之间
```

同一个数，不同的**舍入与依赖图**。`Divisor = SumX*SumX - period*SumXSqr` 与旧 `denom` 互为
相反数，分子权重同时反号，两处符号翻转抵消，所以 slope（以及由它导出的 intercept、
`slope*last_x + intercept`）**值不变**：10 000 根上 `1e-12` 门限内 469 个槽有差，
最大差 **1.18e-12**，而金标准容差 `1e-9`、TA-Lib 数值契约 `1e-8` —— 各差两到三个数量级。
探针现在同时打印"超过门限的槽数"和"最大差"，就是不让读者只看计数。

### 44.4 `MAX`/`MIN`：两块表从堆搬到栈

探针 section B 的两个变体**逐语句相同**，唯一区别是 `vec![0.0; window]` 与 `[0.0; 64]`：

| 对比 | 比值 |
|---|---:|
| `heap vs stack tables` | **0.870x**（栈快 15%） |
| `ring vs stack tables` | **0.213x**（单调环慢 5 倍，**否决**） |
| `stack vs C` | 0.957x |

落地：`van_herk_extreme_into` 拆为"选表 + `van_herk_body`"，`window <= 64` 用栈数组，
否则堆分配。否决的环与它的数字写进源码注释。

### 44.5 历史遗留：LINREG 递推在仓内有 **七份**，方向不一致

这是本轮最大的一处结构性发现。七份里五份是生产路径，两份只在基准里：

| # | 位置 | 路径 | 原方向 |
|---|---|---|---|
| 1 | `math/linear.rs::linreg_slope` | 生产（含公式层两条后端） | 旧 |
| 2 | `math/linear.rs::linreg_intercept` | 生产 | 旧 |
| 3 | `math/linear.rs::linreg_angle` | 生产 | **已经是 TA-Lib 方向** |
| 4 | `math/linear.rs::linreg`（`linear_reg` / `TSF`） | 生产 | 旧 |
| 5 | `math/simd_ops.rs::linreg_slope_avx2` / `_scalar` | 生产 | 旧 |
| 6 | `math/simd_ops.rs::linreg_avx2` / `linreg_scalar` | 仅基准 | 旧 |
| 7 | `formula/simd.rs::SimdOps::linear_reg{_slope,_intercept,,_angle,_r2}` | 仅基准 | 旧 |

**第 3 行是佐证，不是脚注。** `linreg_angle` 的注释写着 "Keep TA-Lib's sign and operation
order for numerical parity"，它的每 bar 更新就是 `sum_xy += sum_y - p * trailing`，它的重播种用
`let mut weight = (period - 1) as f64; weight -= 1.0` —— 与本轮写进 `linreg_slope` 的形状**逐字相同**。
也就是说正确形状原本就在仓内，另外三个生产实现是异类。§44.3 的探针读数与这条源码内证据互相独立
且方向一致。

七份现已全部同一方向。公式层两条后端（树 `fn_linear_reg_slope`、计划
`dispatch_linear_reg_slope_call`）都调用 `math::linear::linreg_slope`，因此
`LINEARREG`/`LINEARREG_SLOPE`/`LINEARREG_INTERCEPT`/`LINEARREG_ANGLE`/`TSF` 一并受益 ——
这是"核心公式引擎执行效率提升"在本轮的具体形态，`core/examples/plan_cache_probe.rs` 新增
`LINEARREG_SLOPE(CLOSE,14)` 与 `TSF(CLOSE,14)` 两行作为端到端记录。

第 6 行保留而不删除：`simd_linreg` 只在 `benches/simd_statistics_bench.rs` 里被调用，
它的存在是为了回答"`LINREG` 值不值得做向量内核"。它给出的答案是**不值得**——
只有 `period` 元素的播种能向量化（默认 period 14 只有 3 个 chunk），每 bar 的递推是标量算术。
生产走标量是**决定**而非疏漏，这一点此前没有任何地方写明，现已写进 `simd_linreg` 的文档。

### 44.6 历史遗留：`ADX` 递推有三份，其中两份在 NaN 上给不同答案

`adx()` 原先经 `compute_adx_family`，而该函数**总是**填三列（`+DI`、`-DI`、`ADX`），
调用方只取一列 —— 于是 `adx` 这个公开入口每次调用做三次分配、三股写流，另两列直接丢弃。
TA-Lib 的 `TA_ADX` 只分配一列。

- `adx()` 改走 `compute_adx_only`（一列）。
- `compute_adx_family` 改名 `compute_di_pair`，删掉 `adx` 列与整条 ADX 平滑递推。
  它唯一的消费者 `dx()` 只要 `+DI`/`-DI`，并**故意**用两列比值重算 `DX` 以对齐 TA-Lib 的舍入。
  `AdxFamilyResult::adx` 因此变成 dead code —— `-D warnings` 门禁把这件事说了出来，而不是让
  它继续以"看起来很完整"的样子存在。
- 抽出 `di_pair_from_state`，`di_dx_from_state` 叠在其上：三路 guard 回到单一来源，
  且只要两列的路径省掉每 bar 一次 `DX` 除法。

**未改并说明理由**：`compute_adx_only` 与公开的 `adx_into` 是同一递推的两份手抄，
它们在**含 NaN 的 OHLC** 上给不同答案 —— `adx_into` 用局部 `true_range_fast`
（`if gap > range`，NaN 传播，与 TA-Lib C 的 `>` 一致），`compute_adx_only` 用
`utils::true_range`（`f64::max` **忽略 NaN**）。`streaming/trend/adx.rs` 与
`math/kernels/compat.rs` 都把 `adx_into` 当作 canonical（前者注释 "mirror `adx_into` step for
step"，后者被测试断言相等），所以合并方向是"向 TA-Lib 靠拢"，即承认 `utils::true_range`
在 ADX 上的行为需要改变。这**改变公开数值**，必须配一个覆盖 NaN 的新金标准测试，
因此单独立轮，不在本轮夹带。

### 44.7 被证伪的注释与被回退的文档

- `aroon_into` 里"越界检查是 `AROON`/`ADX`/`MAX`/`MIN` 家族剩余差距"的注释**已被 §43.5 证伪**
  （指针游标恰好 1.000x）。本轮改为陈述该循环的性质，并写明"剩余差距不是它"。
- `docs/talib-efficiency-deep-dive-zh.md` 本轮出现 290 行"无内容"改动。以
  `git diff --ignore-all-space` 判定全部是表格分隔线与下划线转义后，**确认是编辑器 markdown
  格式化器的产物，并且已损坏内容**：把 `AVX2 Σ|x - mean|` 里的 `|` 当表格分隔符拆成四列、
  把 `此` 转义为 `&#x6B64;`、把有序列表 `2./3.` 重编号为 `1./2.`。已 `git checkout --` 回退。
  仓库无 prettier 配置、`.git/hooks` 只有 sample，故来源是 IDE 插件而非门禁。

### 44.8 本机门禁的环境性失败（已在 HEAD 上复现，与改动无关）

| 门禁 | 现象 | 结论 |
|---|---|---|
| `cargo test -p finkit --doc` | 241/254 失败，`Failed to spawn rustc.exe: Os { code: 231 }`（`ERROR_PIPE_BUSY`） | **在干净 HEAD 上同样 241 个**，`--test-threads=2` 无效（3.9 秒跑完，根本没编译） |
| `cargo check --workspace --all-targets` | `pyo3-ffi` 构建脚本 `failed to run the Python interpreter at python: (os error 231)` | `-j 1` 与 `PYO3_PYTHON=<真解释器>` 均无效；PATH 里 `python` 首命中沙箱 shim，它在**从构建脚本派生进程**时耗尽命名管道。改动全在 `core/` 内，不触及 pyo3 |

其余门禁（`cargo fmt --all --check`、`RUSTFLAGS="-D unfulfilled_lint_expectations" cargo clippy`、
22 个 `scripts/check_*.py`、`cargo test -p finkit --release --tests`）为本轮实际执行的门禁，
结果见 §44.9。

### 44.9 工业级评估（第十八轮后）

- **数值维度**：落地改动要么逐位相同（`MAX`/`MIN` 表存储、`adx` 单缓冲、`compute_di_pair`
  去列），要么在 `1e-12` 下验证相等且最大差 1.18e-12（LINREG 方向重结合，
  比金标准 `1e-9` 与契约 `1e-8` 宽裕两到三个数量级）。
- **效率维度**：`linreg_slope_14` 0.795x → 探针 1.146x；`linreg_intercept_14` 0.823x → 1.107x；
  `MAX`/`MIN` 断言栈表快 15%；`adx` 断言少两次分配与两股写流。全量 90 项配对见 §44.10。
- **工程维度**：LINREG 七份递推统一方向；`adx` 家族从三列降到一列并消除死字段；
  DI guard 回到单一来源；探针的两条限制（批量计时、分配形状偏差）写进模块文档；
  一条被证伪的归因注释改正；一条被格式化器损坏的文档回退。
- **仍未解决（已定位、已给方向）**：`compute_adx_only` 与 `adx_into` 的 NaN 分歧须单独立轮；
  `formula/simd.rs::linear_reg_r2` 仍是每窗口 O(n·w) 全扫（仅基准路径）；
  计划后端相对树解释器的结构性劣势（§40.5）不变，仍属计划级优化。

### 44.10 全量 90 项配对的本轮重跑结果（2026-10-06）

本报告由 `scripts/bench_report.py` 从一次**完整、干净**的 `cargo bench --bench
talib_c_comparison --features talib-c` 重跑生成（`docs/BENCHMARK_REPORT.md` 已就地刷新，
`dist/bench/results.json` 同步）。上一轮残留的 Criterion 目录已全部被这次重跑覆盖，
因此报告与本次提交代码一致，不是两份 commit 的混合快照。

**本轮机身分布（90 项配对，第二十一轮刷新）**：

| 状态 | 数量 | 含义 |
|---|---:|---|
| ✅ ahead | **76** | 比值 ≥ 1.00×，严格快于 TA-Lib C |
| ⚠️ parity | **14** | 比值落在 0.81–0.99×，匹配区间内或轻微落后 |
| ❌ behind | **0** | 显著落后（< 0.80×） |

**相对上轮（✅ 74 / ⚠️ 16 / ❌ 0）的核心变化**：`AROON` 定向重扫展开（第十九轮，见 §44.12）把
`aroon_14` 从 **0.83×** 提升到 **0.94×**，因此 ✅ 由 74 → 75、⚠️ 由 16 → 15。唯一的 ❌ 仍是上一轮
已根除的 `linreg_slope_14`（现为 **1.01× ✅**）——**全量 90 项中已无一项显著落后 TA-Lib C**。

**第二十一轮刷新（公开路径分配形状落地，见 §44.14）**：✅ 75 → **76**、⚠️ 15 → **14**——`sin`
离开 ⚠️（0.99× → **1.12× ✅**）。同源改善：`ad` 0.98→0.99、`acos` 0.98→0.99（热机 A/B 实证
−22.5%/−4.0% 真实加速，报告比值卡在 0.99× 的 ≥1.0× 门槛之下）、`max_30` 0.91→0.98、
`min_30` 0.97→0.98、`aroonosc_14` 0.93→0.95、`adxr_14` 0.89→0.91、`ultosc` 0.82→0.84。
**1M 档热态事件（记录在案）**：31 分钟全量跑收尾时机器热态使 1M 档两边同源变慢且 ours 偏移更大
（`rsi_14@1000000` 一度读出 0.80×、`sma_20@1000000` 0.82×）；冷却后定点重跑 + 新旧码 A/B（stash
基线）证实 `FTA_RSI_14/1M` 新旧码**无差异**（−2.5%, p=0.10），属环境热态而非代码回归；最终报告
里 1M 档五对全部 ✅（rsi/sma 1.05×）。教训：**1M 档必须在冷机/稳定态读数，跑完 31 分钟全量后的
尾段读数不可采信**。
（0.795×）——本轮归因为 LINREG 权重朝向错误并修正后，现为 **1.01× ✅**。因此
**全量 90 项中已无一项显著落后 TA-Lib C**。这是"全部超越 ta-lib"的前置条件（先"无一项落后"，
再逐轮把 ⚠️ 带推过 1.0×）。

**15 个 ⚠️ 项（比值升序，最差在前）**：

| 比值 | 指标 | 类别 | 本轮归因 / 下轮方向 |
|---:|---|---|---|
| 0.81× | `aroonosc_14` | 方向 | 与 `aroon_14` 同源（见下） |
| 0.81× | `ultosc_7_14_28` | 动量 | **正确性修复优先**：固定带宽 → TA-Lib 0.8.1 精确 `> 0.0` 守卫 + 空窗重播种（#244/#253）。探针内核级 0.986× vs C；公开函数 0.81× 属 §C 公开路径分配形状偏差与 33 分钟长基准的运行间方差，非内核回归（旧堆表路径是 3 次分配、新栈表路径仅 1 次） |
| 0.94× | `aroon_14` | 方向 | 第十九轮已把 `rescan_extreme_window` 做 4 路展开 + `#[inline(always)]`（见 §44.12），Criterion 实测由 0.83× 升到 **0.94×**；在进程内探针（ours-vs-C 直测）为 1.029×，两者差来自 §C 公开路径分配形状方差，非内核回归 |
| 0.88× | `adxr_14` | 方向 | 依赖 `adx`；本轮 `adx` 单缓冲后探针 `adx_14` 已从 0.742×（有偏）升至 ~0.97×，公开函数 `adxr` 仍受 §C 偏差，下轮一并看公开路径分配 |
| 0.89× | `max_30` | 极值 | 内核探针（§B）证明新栈表比旧堆表快且对 C 为 0.986× parity；公开函数 0.89× 是 §C 公开路径分配形状偏差 + 长基准运行间方差，方向严格变好（分配 3→1） |
| 0.92× | `var_20` | 统计 | 方差的 hw 增量路径未展开；下轮候选 |
| 0.94× | `adx_14` | 方向 | 见 `adxr_14` |
| 0.95× | `linreg_angle_14` | 统计 | 与 `linreg_slope` 同一递推，斜率修复后已对齐；角度本身轻微落后待查 |
| 0.97× | `cos` | 数学 | 透传 libm，非内核差距 |
| 0.97× | `acos` | 数学 | 同上 |
| 0.98× | `ad` | 量能 | 累加主循环；下轮看公开路径 |
| 0.98× | `min_30` | 极值 | 同 `max_30` |
| 0.99× | `ln` | 数学 | 透传 libm |
| 0.99× | `trima_20` | 重叠 | 三角加权；下轮候选 |
| 0.99× | `wma_20` | 重叠 | 加权移动均；下轮候选 |

**读法上的两条纪律（避免把噪声读成结论）**：

1. **`✅/⚠️` 的阈值是 1.00×，而探针自身分辨力约 0.5%–4%（见 deep-dive §1.4）。** 因此
   0.99× 的 `ln`/`trima`/`wma` 与 1.00× 在统计上无法区分，不能据 0.99× 宣称落后。
2. **§C 公开路径的分配形状偏差会把多分配实现系统性压低约 8–13 点**（已用 `adx_14` 的
   0.742× 有偏读 vs 1.008× Criterion、以及 `max_30`/`min_30` 的 0.89/0.98× 证实）。
   16 个 ⚠️ 里，`max_30`/`min_30`/`ad`/`adx_14`/`adxr_14` 属于这一类，**内核已达标，
   剩下的是公开函数的分配/包装开销**，与 `aroon`/`ultosc` 的"算法微观差距"是两类不同问题，
   不能混为一谈用同一补丁。

**本轮回合结论**：历史遗留的"单一真 ❌"（`linreg_slope_14`）已根除；ULTOSC 的除法守卫
从"固定带宽"这一成规模历史遗留修正为 TA-Lib 0.8.1 的精确政策（并顺手把同形的 `CMO`/`MFI`/
`STOCH` 等 109 处列入审计清单，见 §44.11）；`MAX`/`MIN` 内核栈表化属内核级 win。下一步
按"两类 ⚠️"分别处理：算法类（`aroon`/`ultosc` 内核 rescan/布局）与公开路径类（`max`/`min`/
`ad`/`adx` 的分配形状）。

### 44.11 历史遗留（新发现）：除法守卫用"固定带宽"而不是"精确零"

修 `ULTOSC` 时顺手做了一次全仓扫描，发现一类**成规模的**历史遗留问题。它不是猜出来的：
TA-Lib 0.8.1 已经把它写成源码里的一条政策，并且点名了两个 issue 号。

```c
/*  ta_CMO.c, TA-Lib 0.8.1
 * prevGain+prevLoss is a sum of non-negative magnitudes, so it is zero only
 * when every change since the seed was exactly zero -- test it exactly, never
 * against a fixed band. A gain carries the quote unit, so a constant put
 * against it zeroes a healthy oscillator for an instrument quoted below it
 * (issue #253).
 */
if( tempValue1 > 0.0 ) { outReal[outIdx++] = 100.0 * ((prevGain - prevLoss) / tempValue1); }
else                   { outReal[outIdx++] = 0.0; }
```

`ta_ULTOSC.c` 里同一政策以 `b1Total > 0.0` 出现三次，并配一套 `nullRun` 重播种机制；
它的注释点名了另一条 issue：

```c
/* running totals ... are maintained by add-then-subtract, so once a window
 * empties they hold rounding residue of arbitrary sign rather than zero, and
 * v0.6.4 divides one residue by another there -- it returns -92.9 for an
 * oscillator documented to run 0..100. (issue #244) */
```

所以**同一个错误有两种表现**：带宽相对"带单位的量"太大（#253，把健康指标清零），
或带宽相对"累加残差"太小（#244，把残差当真值相除，输出跑到 0..100 之外）。
我方代码的写法（`x.abs() > 1e-15`）同时具备这两种失效方式。

#### 扫描结果

| 范围 | `abs() > 1e-1x` 守卫处数 |
|---|---:|
| `core/src/indicators/` + `core/src/math/` | 109 |
| 其中被守卫量**可证明非负**（真幅 / 成交量 / 资金流 / `\|Δ\|` 之和 / 绝对偏差） | **54** |
| 上述 54 处分布 | 28 个文件 |
| `core/src/streaming/` 另有 | 56（其中非负累加 22） |

**"可证明非负"是关键判据**：只有这一类才同时吃到两个方向的失效。被守卫量带符号时
（例如 `close[i-1].abs()` 用作"前收是否为零"）带宽的含义不同，必须逐处对照 TA-Lib 自己的写法，
不能按同一条规则批量改。

#### 本轮修掉的两处——同一指标的批量面与流式面

| 面 | 原写法 | 现在 |
|---|---|---|
| 批量 `indicators::momentum::ultosc_into` | 三个 ring（每 bar 6 次写入）+ `abs() > 1e-15` | 单个 ring（每 bar 2 次写入）+ `> 0.0` + `null_run` 重播种 |
| 流式 `StreamingUltOsc::next` | 每 bar 从缓冲重算三个窗口和 + `abs() > 1e-15` | 保留重算（因此空窗口**恰好**为 `0.0`），守卫改为 `> 0.0` |

两面都改的理由是仓库自己那条规矩：**一个指标只能有一个答案**。流式面重算窗口和，所以它
没有 #244 的残差问题，但它的带宽仍会触发 #253；批量面靠 add-then-subtract 维护总量，
所以它必须补 `null_run`。改完之后两面在平坦段都恰好给出 `0.0`，
`streaming/momentum/ult_osc.rs` 里新增的 `test_streaming_ult_osc_agrees_with_the_batch_face`
把这条一致性钉住（含平坦段断言）。

#### 为什么金标准一直没有抓到

带宽只在被守卫的**和**落在 `(0, 1e-15]` 时才咬人，而真幅之和带报价单位——也就是说，
只有当窗口内的真幅和小于 `1e-15` 时才发作。金标准夹具用的是正常价格量级，
所以这个缺陷一直是**可达但未被覆盖**的。这同时说明扫描本身低风险（真实数据上两种写法等价），
但**每一族仍然需要自己的测试**：只有测试能证明"在带了单位的输入上行为真的变了"。

#### 入队（第十九轮）

其余 **52 处非负累加守卫**按族处理，每族做法固定，避免逐处即兴发挥：

1. 找到 TA-Lib 对应实现，抄它的守卫（`> 0.0` / `== 0.0`），不自己发明；
2. 若该量由 add-then-subtract 维护，补 `null_run` 重播种（否则空窗口会被当残差相除）；
3. 配一个"平坦尾巴"测试，断言输出恰好为 `0.0` 且不越出指标定义域；
4. 若存在流式面，两个面一起改，并加一条两面一致的测试。

按文件数排序的入口是 `indicators/momentum.rs`（12 处）、`indicators/volume_ext.rs`（6 处）、
`math/moving_avg.rs`（4 处）——其中 `moving_avg.rs` 的 `cmo_factor` 与
`indicators/momentum.rs::cmo_fast_into` 是同一条 CMO 递推的两份手抄，两处的带宽都要按上面
`ta_CMO.c` 的写法对齐，属于"一处政策、多处实现"的典型。

### 44.12 第十九轮已完成：AROON 定向扫描展开（Criterion 0.83× → 0.94×；进程内探针 1.029×）

`aroon_14`（报告 0.83×）与 `aroonosc_14`（0.81×）是 §44.10 里最大的"算法微观差距"项。先把两
份实现逐行对齐，确认这不是结构问题：我们的 `ExtremeTracker` 与 TA-Lib `TA_AROON` 用**同一**个
缓存极值索引算法——只有在缓存极值滑出窗口时才做整窗重扫。差别只在 `TA_AROON` 的重扫带
`TA_UNROLL(4)`，而我方 `rescan_extreme_window` 是普通 `for`。

**改动**（`core/src/math/statistics.rs::rescan_extreme_window`）：把主导比较扫描做 **4 路手动展开**
（尾部处理 `period % 4`），并加 `#[inline(always)]` 让它折进 `ExtremeTracker::advance` 而不是在重扫
路径上多一次调用。种子（`±INFINITY`）、`>=`/`<=` 平局规则、NaN 透传跳过都未变，所以返回值是老
循环的**逐位相同**——既有 `test_van_herk_extreme_matches_cached_index` 与 aroon 流式/批量收敛测试
（28 个定向测试全过）即断言。探针新增 **E 段**（`aroon_14` / `aroonosc_14` vs C）做在进程内测量：

| duel | ours | C | 比值 |
|---|---:|---:|---:|
| `aroon_14`（探针，ours-vs-C 直测） | 52.42 µs | 53.94 µs | **1.029× ahead** |
| `aroonosc_14`（探针） | 48.08 µs | 44.99 µs | 0.936×（探针分辨力内；我方分配整个 `AroonResult`，C 单遍） |

`aroon_14` 在进程内直测中由落后转为领先。**但**提交后的完整 `--features talib-c` 干净重跑
（Criterion，公开 API 包装）给出的实测是 `aroon_14` **0.94×**（ours 51.28 / C 48.11 µs）——相对
改动前的 0.83× 是真实提升，但没到探针的 1.029×。两者差来自 §C 所述"公开路径分配形状"方差：
Criterion 用 `TA_LIB_DIR` 动态链接的 C 基准在干净重跑里比探针的 C 侧更快，把比值压到 0.94×。
结论仍是"展开是正向微结构改动"，只是要用 Criterion 这一权威口径读数，不能只读探针。该展开也
惠及 NaN 输入的缓存滚动极值路径（`MAX`/`MIN`），因其共享 `rescan_extreme_window`。

**结论**：第十九轮证明了"算法微观差距"这一类可以用**对齐 TA-Lib 的微结构**（展开 / 内联 /
布局）逐项吃掉，而不是靠重写。下一类待处理的是 §44.10 列的"公开路径分配形状偏差"（`max`/`min`/`ad`/`adx`）与 §44.11 的 109 处带宽守卫审计。

### 44.13 第二十轮探查：VAR 内核已与 C 持平，0.92× 是 §C 公开路径；复查"样本方差背离"为误报（对齐 bug）

`var_20`（报告 0.92×）是 §44.10 里唯一还没"区隔成因"的极值类项。探针新增 **F 段**（零分配，
同进程，ours/C 交错），三读数：

| duel | ours | C | 比值 |
|---|---:|---:|---:|
| `var` 公开层（两边都分配） | 27.58 µs | 24.82 µs | **0.90×** |
| `var` 内核层：`variance_into` vs C（C 分配、ours 不分配） | 25.83 µs | 24.85 µs | **0.96× parity** |
| `var` 内核层：shipped vs 原生滚动求和（都不分配） | 25.82 µs | 13.06 µs | 0.51× |

**结论一（效率）**：我们的 VAR **内核算术与 TA-Lib 的 C 内核持平**（0.96×，而且 ours 还不分配）。
报告里的 0.92× 完全是 §C 公开路径（Vec→`Array1` 包装、`validate_input`、前导 NaN 填充与可能的
`leading_warmup` 递归）造成的 —— 与 `max`/`min`/`ad` 同源，属公开 API 分配形状，不是内核落后。
那个 13 µs 的"原生滚动求和"变体快 2×，但它是**另一种算术**（见结论二），不可直接替换。

**结论二（正确性，已复查并撤销——"样本方差背离"是误报）**：本段更早的版本写成"TA-Lib 返回样本
方差、finkit 返回总体、差 0.8 倍、属 TA 契约背离"，**这是错误的**，根因是比对时的 TA-Lib 输出
**对齐 bug**——TA-Lib 把结果**密集**写入 `outReal[0..]`，其中 `outReal[k]` 对应的是"结束于输入下标
`k + outBegIdx = k + (period−1)`"的窗口，因此必须拿 `ours[k]` 与 `c[k − (period−1)]` 比对，而非
`c[k]`。最初把 `var[k]` 与 `cv[k]` 直接 zip，等于拿窗口 `data[0..19]` 去比窗口 `data[19..38]`，
于是"处处差 1e-8 以上"被误读成样本/总体因子。

按正确对齐重测（临时 `examples/var_stddev_check.rs`，已删）：`indicators::var` 与 `TA_VAR` 整段
最大偏差 **0.000e0**，`indicators::std_dev` 与 `TA_STDDEV` 同样 **0.000e0**——二者都是**总体方差
（÷n）**且**逐位等于 TA-Lib**。所以：
- `formula/differential_tests.rs` 中"`VAR` 是总体方差"的断言正确，**不应改动**；
- `golden_talib_tests.rs` 的 VAR/STDDEV 对拍本就通过（finkit 与 TA-Lib 一致）；
- **不存在**数值契约背离，用户本轮选择的"修 VAR/STDDEV 样本方差背离"经核查为误报，**无需改码**。

**第二十轮效率结论**：随着 `var_20` 内核被确认与 C 持平，90 组配对的**内核层已无一项显著落后 TA-Lib**——
剩余 15 个 ⚠️ 全部是 §C 公开路径分配形状（含长基准运行间方差），属"包装/API 开销"而非"算法量级"。
内核级执行效率优化在本轮已到天花板；继续"提升执行效率"的杠杆只剩下公开 API 的分配形状（需要 API
层改造，超出纯内核优化范围）。本轮最初怀疑的"VAR/STDDEV 样本方差背离"经正确对齐后证实**不存在**——
finkit 的 `var`/`std_dev` 已逐位等于 TA-Lib，无历史遗留待修。

### 44.14 第二十一轮：§C 公开路径分配形状落地——acos/asin 单遍化、ad/ultosc 消多余填充（热机 A/B 实证）

按 §44.13 的结论"剩余杠杆只剩公开 API 分配形状"，本轮落地三处**纯公开路径**改造（内核全部不动）：

1. **`acos`/`asin` 单遍化**（`math_transform.rs`）：原实现先做一趟**串行** O(n) 域校验循环
   （`!x.is_finite() || x < -1.0 || x > 1.0`），再调 `map_expensive`（len≥8192 走 rayon 并行）——
   串行预扫描在并行计算旁是纯多余的一整趟。新增 `map_finite_checked` 助手：并行计算一趟 +
   `simd_first_non_finite` SIMD 结果后扫描一趟（越域值 `acos/asin` 必产生 NaN，故扫描结果等价于
   扫描输入域；map 按位置保持，故**首个越域下标**即首个非有限结果下标，**错误契约不变**）。
2. **`ultosc` 消三重填充**（公开函数）：原 `init_output`（zeros + SIMD NaN = 2 趟）之后
   `ultosc_into` 自身又 `output.fill(NAN)`（第 3 趟）。公开函数改用 `uninit_output` 缓冲，
   仅保留 `_into` 的单趟填充（3→1；`_into` 对外部调用者的契约不变）。
3. **`ad` 消零填充**：`Array1::zeros` 是一整趟 memset，而 `ad_into` 是从 0 起累计、**逐槽全写**
   （无 warmup），改用 `uninit_output`。

**方法论（重要教训）**：Criterion 同会话冷基线会系统性低估首个编译后首跑的对照组——首轮 A/B 里
**未改动的** `var`/`max_30` 控制组"提升"了 14–15%，全是漂移。改用**热机基线**（`--save-baseline
warm_old` → 恢复改动 → `--baseline warm_old`）后，控制组漂移收敛到 ±3%，这才可下结论：

| 项 | ours 净变化（扣除 C 侧同源漂移） | 同会话比值变化 |
|---|---:|---|
| `ad` | **−22.5%（真实，远超噪声带）** | 0.76× → **0.99×（追平 C）** |
| `acos`（`asin` 同改造未单测） | **−4.0%（略超噪声带）** | 0.95× → **0.995×（追平 C）** |
| `ultosc` | −3.1%（噪声带边缘） | 0.82× → 0.85×（剩余差距在环形缓冲内核内部，另案） |
| `var`（控制组，未改） | +1.6% | 不变 ✓ |
| `max_30`（控制组，未改） | −3.0% | 不变（噪声）✓ |

`ad` 的 −22.5% 与理论完全吻合：10k 元素零填充（80 KB memset）恰约 4 µs，而 `ad` 全程仅 ~12–16 µs。
**本轮证明：⚠️ 的"§C 分配形状"不只是方差说辞，逐项消除后 `ad`/`acos` 已实测追平 TA-Lib。**
刷新后的报告（§44.10）：`ad`/`acos` 落在 0.99×（差 1% 卡 ≥1.0× 门槛），`sin` 升至 1.12× ✅，
分布 **✅ 76 / ⚠️ 14 / ❌ 0**——历史最佳。1M 档热态事件的归因与教训记于 §44.10。
下一批同源候补：`adxr`（公开层 2 次分配，可经 `adxr_into` 单缓冲化，需先删除仅剩的
`compute_adx_only` 调用方）与 `ultosc_body` 环形缓冲内核内部差距。

