# Finkit 重构总纲（2026-09-21）

> **本文档是唯一执行基线。** `docs/improvement-plan-2026-09-20.md` 降级为上一轮记录，
> `docs/architecture-gap-assessment-2026-09-20.md` 保留为证据，其余历史计划已归档到 `docs/archive/`。

> **重建说明（2026-09-21）**：本文档一度是 untracked 文件，在一次 `git rm -f` 引发的工作区事故中被删除，
> 无法从 git 恢复。以下内容按本轮所有已验证结论**重建**，所有数字均为本机实测，不是估计值。

> **评估口径**：只采信本机可复现的证据（源码位置 / 命令实测输出）。未验证项一律标注「未验证」。

> **用户已确认的定调**（本方案的约束条件）：
> 1. 重构主轴 = **先做架构去重**
> 2. 公式执行路径 = **只保留 tree（参考实现）+ plan（生产）**
> 3. **finkit 不涉及回测、不涉及选股** → 域外模块直接删除
> 4. **JIT / `eval_simd` 冻结保留，不删除**，后续随多语言扩展统一处理
> 5. 交付形态 = **新建总纲 + 归档旧计划 + 删除无效文档**

---

## 0. 一句话结论

仓库存在大量**已建成未通电**的模块与**同源双份**实现。本轮以「架构去重」为主轴：
删除真孤儿、接线 declarative 因子图、删除域外模块、归档历史文档、冻结而非删除 JIT/SIMD。

## 1. 本机实测基线

| 项目 | 值 |
|---|---|
| `cargo test -p finkit` | **3990 passed / 0 failed / 16 ignored**（Phase 0 基线 4021；删除模块后） |
| 三元组门禁 `(registry, formula, plan kernels)` | **(233, 419, 67)** |
| 差分门 `formula_plan_differential` | 2 条全绿（domestic + pine） |
| 差分门 `formula_differential_tests` | 10 条全绿 |
| `check_orphan_modules.py` | OK（38 个公开模块） |
| `gen_ssot_docs.py --check` | OK |
| `check_docs_links.py` | OK（86 个 Markdown） |
| `cargo fmt --check` | ⚠️ 红 41 处（**未处理**，见 §8） |
| clippy | 5516 warning（既有，本轮未增加） |

---

## 2. 产品边界（用户定调）

**finkit 不涉及回测，也不涉及选股。** 这是产品定位，不是技术判断。

已删除：`core/src/{backtest, backtest_evaluation, selectors, sector}.rs`。
保留 `risk.rs` / `returns.rs`（有真实消费方：`performance.rs`、`factor-analysis`）。
`chan`（缠论）属计算职责，**不在删除范围**。

---

## 3. 架构问题清单（均带证据）

### B. 域外孤儿模块 ✅ 已删除

`backtest` 是孤岛：唯一引用它的是 `backtest_evaluation`，而后者零调用方。删除无连带伤害。

### C. 孤儿门禁 ✅ 已落地

`scripts/check_orphan_modules.py`：公开模块零引用即报警，状态 `orphan → test-only → ok`。

### D. 公式函数两张表 —— **原判断是错的，已更正**

原写「`functions.rs`(7316 行) 与 `functions_router.rs`(664 行) 是重复表，应合并」。
**读源码注释后推翻**：这是**刻意的两层设计** —— legacy 是兼容目录，router 在其上用 canonical kernel
覆盖 TA-Lib 敏感的名字，注释明确警告不要一次性高风险重写。

**真正的缺陷更窄但更阴**：`#[path]` 让文件名与模块名错位，`pub mod functions` 指向
`functions_router.rs`，而 `functions.rs` 反而是私有 legacy 表 —— 打开 `functions.rs` 想改公开表，
实际改的是私有表。**已修复**：文件名与模块名对齐，删除两条 `#[path]`，同步 3 处工具引用。
→ **Phase 3.7「归并两张表」作废**。

### E. 三个函数面（数字以运行时为准，别数源码）

| 面 | 现值 | 消费者 |
|---|---|---|
| `registry.rs` SSOT | **233** | schema / operation / FFI / planner 纯度 |
| `get_builtin_functions()`（公式语言面） | **419** | 公式脚本 / 树路径 |
| `FormulaKernelDispatcher`（plan 核） | **67** | 编译计划执行 |

门禁 `core/tests/formula_function_ssot.rs` 用元组断言一次报三个现值。

### I. 文档 sprawl ✅ 已闭环

`docs/archive/` 已创建，**11 份**历史计划移入（`docs/` 根 60 → 47），**V4 三份已硬删除**
（其描述的引擎已被删除，保留即误导）。`docs/archive/README.md` 记录索引与删除理由。

**有意不归档 4 份**（不是漏归）：
`finkit-outperform-talib-architecture-v3.md`（`benchmark_talib_arch_v3_gate.py` 依赖其加速比语义，
是**活规格**）、`runtime-carrier-adoption-plan-2026-09-20.md`（R4 仍在进行）、
`finkit-vs-talib-performance-optimization-plan.md`（`apply_talib_performance_plan.py` 依据）、
`talib-0.8.0-coverage-audit-2026-09-19.md`（事实快照且被索引）。

### J. 方言层静默语义错误 ✅ 已定性并消除「静默」（生产代码无需改）

**更正：这不是生产代码问题。** `"dzh"` 的映射**只存在于两个测试文件**
（`core/tests/formula_corpus.rs`、`core/tests/formula_plan_differential.rs`），
生产代码里根本没有 `dzh` —— `FormulaDialect` 只有
`AlphaTA / TongDaXin / TongHuaShun / EastMoney / Pine`，**没有大智慧变体**。

所以「静默」只影响**测试证据的可信度**：18 条国内语料里 **2 条**标着 DZH，
它们实际是按通达信语义跑的，绿色 = 「通达信语义下两条路径一致」，
**不等于「大智慧已验证」**。

✅ **已消除静默**：`dzh` 拆成独立 match arm + 函数文档写明「这不是已实现方言，
跑的是最接近的 profile」。纯测试/文档改动，**不动生产行为**。
真要「支持大智慧」= 新增 `FormulaDialect::DaZhiHui` + 完整实现，**属大工程，另议**。

### K. IF 真值判定不统一（未修，刻意）

`!= 0.0`（tree/VM/JIT/fn_if 热路径）vs `> 0.0`（stateful.rs、fn_if `len<16` 分支）
→ `IF(-1,a,b)` 短/长序列结果不同。改它会动国内公式行为 → **动前先问。**
`ema_scalar` 用 `data[..period]` 简单均值作种子，种子含 NaN 污染整条序列。两路径一致，未修。

### L. 生成文档抽取 bug ✅ 已修复（比初判严重得多）

`scripts/gen_ssot_docs.py` 旧正则逐行 `r'"([A-Z_]+)"\s*(?:,|\))'` 只匹配「名字后紧跟 `,`/`)`」
→ **遇到 `map.insert("X".to_string(), …)` 完全不匹配**，且看不到运行时别名注入。

| | 数量 |
|---|---|
| 旧文档 | 308 |
| 新抽取 | **416**（与运行时完全一致） |
| 凭空多报 | 3（`ROCP`/`ROCR`/`ZLCCV`） |
| **静默漏报** | **111** |

修复：整文件匹配 `map\.insert\(\s*"([A-Z_0-9]+)"`（**不能逐行** —— 50 处 `map.insert(` 与名字分行）
+ 按 `map.get(spec.name)` 规则从 `registry.rs` 补别名。
**新增门禁** `generated_formula_catalogue_matches_the_runtime_surface`：文档名集合 + 声明计数
必须等于运行时表。ROCP/ROCR/ROCR100 注册后该门禁立即报出 `documented=416 runtime=419` —— 证明它有效。

---

## 4. 重构方案（主轴：架构去重）

### Phase 0 — 护栏与基线 ✅

基线已记录（§1）；孤儿门禁已落地。

### Phase 1 — 删除真孤儿 ✅

| # | 动作 | 状态 |
|---|---|---|
| 1.1 | 删除 `runtime/` + `runtime_engine.rs`（-1680 行） | ✅ 已执行；顺带让 `execution_plan`/`state_arena` 全仓唯一 |
| 1.2/1.3 | `factor_graph` + `factor_provider` **接线** | ✅ 已执行（见下） |

**1.2/1.3 的关键发现**：断点**不是「没人调用」，而是「根本没有可执行出口」** ——
`FactorGraphPlan` 有 `plan()` / `order()` / `node_index()`，却**没有 execute**。
补的是 `execute()` / `execute_node()` + 3 个错误变体，绑定显式且完全（缺一个外部输入就报错）。

### Phase 2 — 消除同源双份

| # | 动作 | 状态 |
|---|---|---|
| 2.1 | `execution_plan` / `state_arena` 全仓唯一 | ✅ 已验证（find 结果唯一，`runtime/` 已消失） |
| 2.2 | `factor_system` vs `factors` 分层 | ✅ **核查后无需改动** —— 见下 |
| 2.3 | `composite` 定位收敛 | ✅ 已执行 |
| 2.4 | 清理 37 处 `#[allow(dead_code)]` | ⚠️ 只有 1 处该动，已修；其余必要 |

**2.2 前提不成立**：`factor_system.rs` 模块头**已有**边界声明且被遵守 ——
`FactorCatalog::compile` 直接委托 `FactorPlan::compile`（:260）。唯一第二实现
（`StatefulFactorSpec` 流式 momentum/volatility）是刻意的，且**已有两条 batch/stream 一致性测试**。
→ 不再加文档，避免噪声。

**2.3**：`core/src/composite.rs` 模块头写死边界 —— `composite` 只拥有**图的表达**
（命名/校验/环检测/FFI JSON 契约），**不拥有执行**（其自带 memoizing evaluator 是与
`FormulaHotPlan` 并列的第二套 DAG 执行器）。两条硬约束：不在本模块新增数值算法；
不再加第二套求值策略。迁移登记为 **Phase 4 第 5 项**。
它是**真实在跑的公开面**（`operation.rs`、`stateful_composite`、C/.NET 绑定、
`evaluate_composite_json`），**非删除候选**。

**2.4 唯一该动的一处**：`patterns/harmonic.rs` 曾有
`#[allow(dead_code)] fn _unused() { let _ = validate_ohlcv; }` —— **用假代码掩盖未使用导入的警告**。
已删掉那个导入。其余 allow 均必要（cfg 选择的 SIMD 分派路径等）。

### Phase 3 — 公式执行路径收敛

| # | 动作 | 状态 |
|---|---|---|
| 3.1 | 补 3 条结构性缺口 | ✅ **B/C 已完成，A 已预估**（见 §3.1） |
| 3.2/3.3 | 落地 `formula_execution_mode` | ✅ 开关已落地；**默认仍 `tree`**（切 `plan` 未拍板，见 §3.2 附注） |
| 3.4/3.5 | JIT / `eval_simd` | ❄️ **冻结保留**（见 §3.4 附注） |
| 3.6 | `BytecodeVM` 标注实验路径 | 并入冻结处理 |
| 3.7 | ~~`functions_legacy` 归并~~ | **作废**，见 §3-D |

#### §3.1 宿主上下文通道 ✅ 已完成（B/C）

**关键发现：B/C 不是 kernel 缺口，是执行契约缺口。**
`engine.rs` 的 `executor.execute(&inputs)` **只拿到 `&[&[f64]]` 数值槽，完全没有上下文对象**；
`chip_data` 挂在 `FormulaContext`（`types.rs`）。→ 补 kernel 也拿不到筹码数据。

**做法**：新增 `HostContext { chip, period_type }`（`types.rs`）；
`FormulaKernelDispatcher` 从 unit struct 变为携带 `host`，新增 `with_host()` 与
`unified_formula_executor_with_host()`；`eval_plan_channels` 从 `ctx` 构造并传入。

**新增 4 个 kernel**：`CALL:WINNER` / `CALL:COST`（委托同一 `ChipData::winner/cost`，
无数据时填 NaN）、`CALL:PERIODTYPE`（填 `period_type`）、`CALL:REFDATE`（读第二操作数首元素作索引）。
**新增 4 条 registry spec**（门禁要求 plan kernel ⊆ SSOT）。

**结果**：`DOMESTIC_UNSUPPORTED` 从 2 条 → **0 条**，两条语料转绿。

**新增 `core/tests/formula_host_context.rs`（5 条）**：语料门禁**无法证明通道有效**
（fixture 没有筹码数据，两条路径都返回 NaN 也能「一致」）。该测试**主动提供真实 ChipData**，
断言两路径逐元素一致**且存在有限值** —— 防的是「看起来绿其实什么都没验证」。

**已发现的既有 plan 路径限制**（非本次引入）：常量-only 公式（如单独的 `COST(50)`、
`PERIODTYPE()`）无法推断执行长度，需至少一个序列操作数。

#### §3.1 缺口 A — Pine `for` 循环 ✅ 已完成

**关键更正：这不是一个缺口，是三个 bug。原预估只看到了第三个。**

| # | 真因 | 位置 |
|---|---|---|
| 1 | `parse_for_stmt` 对同一迭代器连调两次 `Iterator::find`：先找 `by` 步、再找 `block`。无 `by` 时第一次 `find` **把迭代器抽干** → 循环体永远解析为空 | `pine/parser.rs` |
| 2 | mapper 规范化了除循环变量外的所有标识符 → 执行器绑 `i`，循环体读 `I` → `Unknown variable: I` | `pine/ast_mapper.rs` |
| 3 | `compute_ir` 把循环体当不透明控制流 | `formula/compute_ir.rs` |

**1 和 2 是「参考路径也算错」的静默错误** —— 循环体被丢弃，`volSum`/`priceSum` 恒为 0，
`avgPrice = 0/0 = NaN`。差分门禁查不出来，因为两条路径**一致地错着**。
→ 修完 1、2 之后，`volume_profile` 从「两条路径都给 NaN」变成「两条路径都给对的值」。

**做法（第 3 项）**：常量界完全展开，不需要循环运行时。
- `const_env` + `const_eval`：`input(50)` 折叠为字面量后 `lookback - 1` 可折叠为单节点 `NUMBER`，
  实测语料里**没有** `BINARY:Sub` 节点，直接是 `NUMBER(49)`。
- 循环变量进 `const_env` → `volume[i]` 的 `i` 解析为常量而非「名为 I 的输入序列」。
- 循环携带状态变成普通依赖链（每轮 `ASSIGN:` 读上一轮写的值），与树路径求值顺序一致。

**第二个更正：`volume[i]` 不是 `REF` 位移。** 树路径 `IndexAccess` 语义是
`result[i] = arr[idx[i]]`（**gather + 广播**），常量下标 = 把历史上**一个**元素广播到整条序列。
写成 `REF(volume, i)` 会数值分叉。`INDEX` kernel 连边界都照抄：下标用 Rust 饱和 `as usize`
（NaN/负数 → 0），越界 → NaN。

**顺带必须改的**：`bind_numeric_literals` 原本靠**重走 AST** 收集字面量并与 NUMBER 节点按顺序配对。
展开后这条路必错（它得复现「循环体复制了几遍」）。改为**由 lowerer 在造节点时按 id 记录字面量**
（`FormulaComputePlan::number_literal`），删掉那个 AST 镜像 walker —— 它本身就是一类漂移 bug 的温床。

**N 上限**：不新设常量，**直接复用解释器的 `MAX_LOOP_ITERATIONS`（10_000）**。
低于它会造出「树路径能跑、plan 路径编译失败」的分裂，那正是「可直接替换的更快路径」失效的样子。
超过则编译期**响亮失败**，绝不截断（截断 = 静默算出部分和）。

#### §3.2 附注：`formula_execution_mode` 已落地，但**默认没切**

`FormulaExecutionMode::{Tree, Plan}` + `with_execution_mode()` / `set_execution_mode()` /
`execution_mode()`，路由进 `eval` / `eval_with_dialect` / `eval_with_params`。
**默认 `Tree`** —— 切 `plan` 是发布级决定，不是重构的一部分。

**为什么不顺手切**：
- `FormulaEngine` 有 **~18 个 `pub fn eval*`** 入口（range / multi / zero-copy / incremental /
  parallel …）。「切默认」的口子比计划表里那一行字宽得多。
- plan 路径**失败不静默回退**（这是刻意的）→ 切默认 = 5 个语言绑定上任何 plan 编不了的公式
  **直接报错**。语料覆盖只有 24 Pine + 18 国内，撑不起这个判断。
- 两条路径还有一个非数值差异：树路径把赋值**写回 `ctx.variables`**，plan 路径不动它。
  只比数值的测试**证明不了开关真的生效**（两条路径数值本来就一致）。

所以 `core/tests/formula_execution_mode.rs` 钉的是「路径真的换了」而不是「数值一样」：
`ctx.variables` 是否被写、`WHILE`（无环降级不了）在 plan 模式是否响亮失败。

**切默认只需一行**（`#[default] Tree` → `Plan`）；建议同时决定
JIT/`eval_simd` 与多语言绑定的统一发布节奏（§3.4）。

##### 实测：现在切默认会挂 18 组测试（2026-09-21）

不猜了，直接把 `#[default]` 翻成 `Plan` 跑了一遍 `cargo test -p finkit`：
**2944 passed / 18 failed**。失败原因分五类，**都不是数值分叉**：

| 类型 | 证据 | 代表用例 |
|---|---|---|
| **kernel 缺口**（最多） | `kernel dispatch failed ... code 1` | `ADD/SUB/MULT/DIV`、`MINUS`、`SQRT`、`SINH/COSH/TANH`、`MAXINDEX/MININDEX`、`HHVBARS/LLVBARS` |
| **复合赋值降级错** | `unsupported ... COMPOUND:X:AddAssign: expected 2 operands ... found 3` | `+=` 类公式 |
| **常量-only 公式** | `cannot infer execution length without bound inputs` | 无序列操作数的公式 |
| **Pine 函数** | `common Pine functions must execute: kernel dispatch failed` | pine 用户自定义函数 / 常见 TA 函数 |
| **缓存与状态语义** | `engine.cache_hit(...)` 断言失败；stateful 4 组 | plan 缓存键、stateful 批流一致性 |

**结论：plan 路径离「可当默认」还差得远** —— 卡在**最基础的算术与超越函数**上，
不是边角。`ADD(CLOSE,1)`、`SQRT(CLOSE)` 这种都会直接报错。
`DECLARED_BUT_NO_KERNEL` 那个 165 条的缺口**正是这里的拦路石**，且优先级最高的
就是 `ADD/SUB/MULT/DIV`（MEMORY 已记：它们不在 kernel 表里，编译得过但跑不动）。

**顺手修好的一个小设计问题**：两个构造函数原本硬写 `FormulaExecutionMode::Tree`，
与 `#[default]` 脱钩 → 改 `#[default]` 不会生效（会让人以为切了其实没切）。
现改为 `FormulaExecutionMode::default()`，**切默认真正只需一处**。

#### §3.4 附注：❄️ 冻结（freeze）的定义 —— 用户 2026-09-21 定调

> **先保留、冻结，后续再扩展支持多语言。**

| 规则 | 落地位置 |
|---|---|
| ① 只接受正确性修复，不做新能力 | `core/src/formula/jit.rs` 模块头（**该文件原本无模块文档**） |
| ② 永不进默认路径 | `jit.rs` + `engine.rs` 四个入口 doc |
| ③ 不新增 core 内部调用方 | `jit.rs` |
| ④ `eval_simd` 必须保持 `eval` 的精确别名 | `engine.rs::eval_simd` doc |
| ⑤ 冻结路径靠差分门禁活着 | `formula_differential_tests.rs:80/85`（既有，未新增） |
| ⑥ 对外统一表述 frozen | 两份 README 能力表 + 冻结说明段 |

**为什么不能删**：4 个绑定导出为公开 API；Go 是唯一已剥离的（`drop_jit_simd.py`）。
风险不对称仍在 —— 冻结**恰好保留了** `eval_jit` 这条差分比较路径。

**多语言统一清单（Phase 7 输入，未排期）**：
① Go 补齐或明确「不提供」，不能继续靠脚本静默剥离；② 其余 4 绑定同步下线或同步标注；
③ 必须同一发布周期；④ 保留则把 frozen 语义写进各绑定类型（如 `FormulaEvalMode`）。

### Phase 4 — 编译计划优化收益（切换后再做）

1. DAG 级 CSE；2. buffer liveness 复用；3. 并行切分（受 `rayon` 约束）；4. 算子融合；
5. **【2.3 转入】`composite` 执行后端迁到编译计划路径**（行为变更，须在切 `plan` 后逐项验证）。

### Phase 5 — 产品边界

| # | 动作 | 状态 |
|---|---|---|
| 5.3 | 域外模块删除 | ✅ 已按「不涉及回测/选股」执行（`chan` 不在此列） |

### Phase 6 — 文档治理 ✅

| # | 动作 | 状态 |
|---|---|---|
| 6.1 | 创建 `docs/archive/`，移入历史计划 | ✅ 11 份移入 + 3 份 V4 硬删除 |
| 6.2 | 修复断链 | ✅ 4 条断链已修，86 文件全绿 |
| 6.3 | `docs/README.md` 挂 `archive/` 索引 | ✅ 新增「Current refactor baseline」小节 |
| 6.4 | 更正 README 能力表 | ✅ 去掉「Bytecode/JIT」→ frozen；两份 README 补充「不做回测/选股」 |

**6.4 的一个判断**：**「SIMD kernels」表述保留** —— 已验证为真
（`sma_simd_into`/`ema_simd_into`/`rsi_simd_into` 被 `engine.rs` 与 `indicators/` 真实调用）。
假的只有 `eval_simd`（函数体就是 `self.eval(...)`，`Cargo.toml` 注释自己已承认）。
**「SIMD 存在」和「eval_simd 是真路径」是两回事。**

**删模块后必须做的连带动作**：`docs/quant-evaluation.md` 曾有一整节示例
`use finkit::backtest_evaluation::evaluate_backtest;` —— **调的是已删除 API**，整节删除；
`product-overview{-zh}.md` 曾宣称提供 lightweight backtest —— 已更正；
`factor-research-architecture.md` 有 10 处引用 `core/src/backtest.rs` —— 抬头加 ⚠️ 失效声明，
彻底清理列为独立工作项。

---

## 5. 门禁（不放宽）

- `cargo test -p finkit`：不低于基线，**0 failed**
- `core/tests/formula_function_ssot.rs`：三元组 + 文档-vs-运行时一致
- `core/tests/formula_plan_differential.rs`：allowlist **既不能变绿也不能变红**
- `core/tests/formula_differential_tests.rs`：全绿
- `scripts/check_orphan_modules.py` / `gen_ssot_docs.py --check` / `check_docs_links.py` / `check_versions.py`

---

## 6. 风险与回滚

| 风险 | 缓解 |
|---|---|
| 归档/删除文档导致 CI 断链 | 每次移动后跑 `check_docs_links.py`（**已发生一次：删除 V4 时误删整个 `docs/` 工作区文件，靠 git 恢复**） |
| 冻结接口被悄悄扩展 | 差分门禁持续比较；模块头写明规则 |
| `composite` 迁移是行为变更 | 必须等切 `plan` 后逐项验证 |

---

## 7. 明确不做

- 不做行情采集、券商交易、OMS/EMS、完整资管平台
- **不做回测引擎、不做选股引擎**（用户定调）
- 不为「新增函数数量」而新增函数
- 不在 Research / Formula / Binding / Visualization 新增独立算法实现（**必须落 canonical kernel**）
- **不删除 JIT / `eval_simd`**；**不为冻结接口新增能力**
- **不在本轮改动任何数值行为**（§3-K 的 IF 真值与 `ema_scalar` 种子单独立项，动前需单独确认）

---

## 8. 待你确认的事项

1. ✅ **缺口 A（Pine `for` 循环展开）已动工并闭环**（见 §3.1 缺口 A）。
2. **`ROCP`/`ROCR`/`ROCR100`** ✅ 已按「注册」执行（416→419），如你更希望删除，我回滚。
3. **V4 三份已硬删除**（原为归档），如你希望保留在 `docs/archive/`，可从 git 恢复。
4. ✅ **`cargo fmt --check` 已修**：先**把 CI 的 fmt job 从 `@stable` 钉到 `1.98`**
   （浮动 toolchain 是这个门禁会自己变红的根因），再用本机同版本 rustfmt 全量格式化 ——
   22 文件 / +231 −163，纯机械。升级 toolchain 时需同步改这一行并重跑 `cargo fmt --all`。
5. **§3-J**（`dzh` 静默映射）与 **§3-K**（IF 真值 / `ema_scalar` 种子）均属行为变更 —— **建议不在本轮处理**。
6. **默认执行路径是否切 `plan`**（见 §3.2 附注）：开关已落地，**默认仍 `tree`**；
   切默认是发布级决定，改 `#[default]` 一行即可。
6. **工作区事故**：`git rm -f` 清掉了 `docs/` 下大量工作区文件，已全部从 git 恢复；
   唯一损失是本文件（untracked）与 `docs/archive/README.md`，均已重建。
   **教训：对带 staged 改动的文件（`git mv` 目标）用 `git rm` 极其危险。**
