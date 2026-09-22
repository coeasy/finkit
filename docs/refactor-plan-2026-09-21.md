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

##### 实测：现在切默认会挂 3 组测试（2026-09-21，第五轮）

不猜了，直接把 `#[default]` 翻成 `Plan` 跑了一遍 `cargo test -p finkit`，
**每轮都实测、只记录实测值**：

| 轮次 | 结果 | 这一轮消掉的 |
|---|---|---|
| 第一轮（补 kernel 前） | **18 failed** | —— |
| 第二轮（算术/超越/窗口 kernel） | **12 failed** | `ADD/SUB/MULT/DIV`、`MINUS`、`SQRT`、`SINH/COSH/TANH`、`MAXINDEX/MININDEX`、`HHVBARS/LLVBARS` |
| 第三轮（沙箱 + COMPOUND + 元素级 kernel） | **8 failed** | 沙箱 2 组、`COMPOUND:X:AddAssign`、`CROSS`/`FIXNAN`/`STDDEV` |
| 第四轮（`CROSSBELOW`/`VAR`） | **6 failed** | `CROSSBELOW`、`VAR` |
| 第五轮（缓存 API + 常量公式定长） | **3 failed** | 缓存语义 2 组、常量-only 公式 1 组 |

**剩余 3 组是同一个根因，且没有一条是数值分叉**：

| 类型 | 证据 | 代表用例 |
|---|---|---|
| **Pine 具名输出** | `runtime did not publish Pine output W` | plan 只回传主结果 + `plan.outputs()`，不写 `ctx.variables` |
| **Pine 用户自定义函数** | `function result assignment must exist` | 同上，函数结果取不到 |

**优先级**（按「不修就不能切」排）：

1. ~~**沙箱**~~ ✅ **已修**（见下）。
2. ~~**COMPOUND 降级 arity**~~ ✅ **已修**：`add_effect` 会给每个 effect 节点追加一条
   排序边，所以 `X += Y` 后面若还有别的语句，节点就带 **3 个**依赖而不是 2 个。
   原来的检查写死 `len != 2` → 直接报错；另一种「顺手修法」（把第三个也当操作数）
   会让合成出的 `BINARY:Add` 拿到 3 个输入、在 dispatch 处撞 arity 检查。
   正确做法是只取前两个（`ASSIGN`/`OUTPUT` 早就是这么做的），排序边丢弃是安全的
   —— 写入已登记进 `local_writes`，后续读取会重新建立真实的数据依赖。
3. ~~**缓存语义**~~ ✅ **已修**：见下。
4. ~~**常量-only 公式**~~ ✅ **已修**：见下。
5. **Pine 具名输出 / 用户自定义函数**：**唯一剩下的，需要用户拍板** —— 见下。

##### 缓存 API 与常量公式定长（2026-09-21，第五轮）

**① 缓存统计必须描述「引擎正在用的那个缓存」。**
引擎有两个编译缓存：树路径的 AST 缓存、plan 路径的 plan 缓存。而
`cache_hit`/`cache_size`/`clear_cache` 只看 AST 缓存 → 在 plan 模式下**恒为
「未命中 / 0 条」**，统计描述的是一个引擎根本没在查的缓存，调用方分不出冷热。
现按 `execution_mode` 分派；`cache_hit(source)` 只收 source，plan 缓存却按
source+dialect+参数指纹做键，所以它在 plan 模式下查**默认 dialect + 空参数**
（与 `eval` 同键），完整信息走 `plan_cache_stats`。

`clear_cache` 则**两个缓存都清**，与模式无关 —— 只清一个会让下一次求值在调用方
刚要求「清空」之后照样跳过重编译，而「某个 source 现在解析到别的东西了」正是调用它的原因。

**② 常量公式的长度来自 `ctx.data_len`。**
plan 路径原来从**第一个输入槽**推断执行长度，于是 `10 + 20` 这种没有输入槽的公式
直接报 `cannot infer execution length without bound inputs`；而树路径会把常量广播到
`ctx.data_len`。两条路径连「能不能跑」都不一致。
修法有两层：`eval_plan_channels` 在无输入时显式用 `ctx.data_len` 定长；
`UnifiedExecutor::validate_inputs` 的返回值从 `usize` 改成 `Option<usize>`
（`None` = 计划没有输入槽），否则 `range.end > common_len` 会把**任何非空区间**
都判成越界。

**③ 剩下的 3 组：原判断「会改 5 个语言绑定」是错的，已修完。**
原结论是「plan 路径刻意不写 `ctx.variables`，要修就得改签名，跨 5 个语言绑定」。
**实测把成本估错了**：

| 断言 | 实测 |
|---|---|
| `eval_plan*` 被谁调用 | **只有 `engine.rs` 内部 3 处 + 2 个测试文件**；绑定一个都没调 |
| 绑定走哪个入口 | `eval` / `eval_with_dialect` / `eval_with_params`，**签名本来就是 `&mut FormulaContext`** |
| 真正的阻力 | 不是签名，是 **plan 会把赋值全部剪掉**（只保留 `OUTPUT:` 节点当 root），所以即便改了签名也没有东西可写回 |

所以修法分两层，都不涉及绑定：

1. **保留赋值**：`lower_formula_plumbing` 把 `ASSIGN:`/`COMPOUND:` 的目标也登记为 root，
   并作为 `FormulaVariableBinding` 暴露（`FormulaHotPlan::variables()`）。
   保留 root 意味着缓冲区不被回收 —— 这与树路径把同一批值放在 `ctx.variables` 里
   是**同等内存**，不是新增开销。
2. **写回**：`eval_plan_channels` 收 `&mut FormulaContext`，把变量写进 `ctx.variables`、
   把声明通道推进 `ctx.output_names`、把样式写进 `ctx.output_modifiers`。

**踩到的两个坑（都不是签名问题）**：

- **重名写入要「后者胜」**：`x = 1; x = 2` 在 `ctx.variables` 里只剩一个条目，
  所以去重必须在**选 root 之前**做，否则会为一个死掉的首次写入白留一个缓冲区，
  并按顺序发布错误的值。
- **大小写不能丢**：`ASSIGN:`/`OUTPUT:` 的操作名是 **canonical（全大写）**的，
  依赖解析靠它；但 `ctx.variables`/`ctx.output_names` 是**按源码原样**做键的，
  而 parser 不做大小写归一。所以发布名必须从 `ComputeEffect`（`WriteVariable`/
  `EmitOutput`）里取回原样拼写 —— 否则 `golden:` 会被发布成 `GOLDEN`。
  上例两个测试都用**小写输出名**正是为了钉住这一点。
- `ctx.output_modifiers` 也补齐了：样式不属于数值计划，所以随 `FormulaComputePlan`
  一起带出（新增一个 `BTreeMap`，不动 `ComputeEffect`，也就不用给 `OutputModifier`
  补 `PartialEq`）。

**顺带发现（未修，属既有行为）**：树路径对**变量读**走 canonical 名、对**变量写**按原样
存储，所以小写变量名在参考路径上直接失败（`Unknown variable: MA5`）。这是既有实现
不一致，不是本次改动引入的，**没有动**。

**结果：3 → 1。** 剩下那 1 组（Pine 语义映射器）不再是上下文问题，而是
**kernel 覆盖缺口**：它一次性调用 13 个函数，缺一个就整组红。本轮补了 `HMA`、`CORREL`
（两者都是「公式面可调用但 registry 未声明」，已一并注册）；仍缺 6 个：
`BARSSINCE`、`TRANGE`、`CUMSUM`、`MEDIAN`、`ROLLING_RANGE`、`RMA`
（前两个还要补注册）。缺口清单由错误信息里的 kernel 哈希反查得到 ——
`KernelId` 是 `"CALL:<NAME>"` 的 FNV-1a 64，所以从哈希反查名字是定位缺口最快的手段。

##### 第七轮：写回改动把「隐藏的覆盖缺口」翻了出来（2026-09-21）

上面 6 个 kernel 连同 `ISNA` 已补齐（`ISNA` 是写回改动带出来的：保留赋值 root 后
`nz()` 这个原先被死代码消除的节点现在必须执行），Pine 语料门禁转绿。但同一次翻转默认
又暴露出**一整组此前从未被测量到的缺口** —— `dzh_compat_tests` 35 个测试里红了 34 个，
涉及 15 个缺失 kernel。这不是「再补几个 kernel」，15 个缺口分属四类：

| 类别 | 函数 | 为什么不是「补个 kernel」 |
|---|---|---|
| ① 字符串字面量 | `STRING_LITERAL` | plan 路径**根本没有字符串字面量的表示**。树路径把字面量追加到 `FormulaContext::string_table`，而 kernel ABI 只有数值 buffer、没有上下文对象。**任何含字符串参数的公式在 plan 路径上都跑不动**，不止 DZH 板块引用函数 |
| ② 宿主数据通道 | `MONEYFLOW` / `NETINFLOW` / `MAININFLOW` / `MAININFLOWPCT` / `BIGORDER` / `SUPERBIGORDER` / `SMALLORDER`（7 个） | 全部**零参**，读 `ctx.money_flow_data`；而 `HostContext` 只有 `chip` / `period_type`。同 `WINNER`/`COST` 的处理方式，需要扩 `HostContext` |
| ③ 隐式 OHLC | `TR()` | 也是**零参**，读 `ctx.high/low/close`，不从公式操作数取输入 |
| ④ 纯数值 | `MOD` / `INTPART` / `FRACPART` / `REVERSE` / `SUMBARS`（5 个） | 真正机械，本轮已补（含 registry 注册） |

四类里只有 ④ 是机械缺口；①②③ 都需要先决定 plan 路径如何表示**非数值输入**。
补完 ④ 后重新实测：翻转默认时 `dzh_compat_tests` 从 **34 个红降到 26 个红**
（缺失 kernel 从 15 个降到 9 个），剩下的 9 个**全部**落在 ①②③ ——
即 `STRING_LITERAL` + `TR` + 7 个资金流函数。

另外两个**与 DZH 无关**的独立发现：

- **`DRAW:DRAWLINE` 从未被派发。** `dispatch_draw_call` 的清单列着
  `DRAW:FILL` / `STICK_LINE` / `DRAW_TEXT` / `DRAW_ICON`，并声称「这是 IR 今天产出的完整
  集合」；实际上 `DRAW:FILL` **从来不会被产出**，而 parser 能产出的 11 个 `DRAW:<command>`
  （`DRAWLINE` 首当其冲）**一个都没处理**。根因是 `KernelId` 是不透明哈希，派发只能手工
  枚举，而手工枚举会腐烂。已把 `DrawGeneric` 收敛为单一名字 `DRAW_GENERIC`，**从结构上
  消灭这类腐烂** —— 命令串本就是渲染信息，数值计划不携带它，`STICK_LINE` / `DRAW_TEXT` /
  `DRAW_ICON` 早就是这个形态。
- **测试夹具的绑定缺口会伪装成 kernel 分歧。** 差分夹具原先按硬编码别名表绑定输入槽，而
  `SUMBARS(VOL, 5000)` 开出的槽是 `VARIABLE:VOL`（不是 `VOLUME`），该槽因此被静默留在
  CLOSE 默认值上，`SUMBARS` 看起来「算错了」。已给 `InputLayout` 增加 `operations()`
  访问器，夹具改为**按计划声明的槽逐个用 `ctx.get_data` 解析** —— 与树路径同一套别名规则，
  也与真实前端的绑定方式一致。

**同时钉住了一条既有分歧（未修）**：周期大于序列长度时，树路径的 `canonical_*` 包装层
吞掉 kernel 的 `InsufficientData` 并返回全 NaN，而 `dispatch_periodic_call` 把它映射成
`ERR_PARAMETER` 抛出。这是既有行为、非本轮引入，且当前无任何语料覆盖（所以差分门禁一直
没看见）。已用 `out_of_range_period_is_a_recorded_divergence` 钉住：翻转默认时它会红，
逼着这条错误策略被**显式决定**，而不是把 NaN 悄悄变成异常。

**结论**：plan 路径仍不能当默认。拦路石已从「算术 / 沙箱 / Pine 函数 / 复合赋值 / 上下文
写回」退到 **kernel 覆盖**，并进一步分化出两条**非机械**缺口 —— **字符串字面量在 plan
路径上没有表示**，以及**零参宿主数据函数需要扩 `HostContext`**。这两条需要先拍板。

##### 第八轮：**测量方法本身是错的** —— 缺口不是 1 个，是 87 个（2026-09-21）

第七轮之后我按惯例翻转默认、跑 `cargo test -p finkit`、数红，得到「26 个」。**这个数字
是错的，而且从第一次翻转起就一直错着。**

`cargo test` 在**第一个失败的 test target 上就停**。所以历次「18 → 12 → 8 → 7 → 6 → 3 → 1
→ 26」记录的是**第一个红 target 的红数**，不是全量缺口。加 `--no-fail-fast` 重跑，真实数字是：

| 口径 | 数值 |
|---|---|
| 红 test target | **11 个**（`dzh` / `em` / `fox` / `tdx` / `ths` / `formula_compat` / `formula_regression` / `formula_differential_tests` / `formula_engine_integration` / `formula_execution_mode` / `formula_cache_tests`） |
| 红测试 | **179 个** |
| **缺失 kernel（`code 1`）** | **87 个**（去重后） |
| 其它派发错误 | `code 3`（`ERR_PARAMETER`）6 处、`code 2`（`ERR_ARITY`）2 处 |

> **方法论教训（第二条）**：翻转默认做实验时，**必须加 `--no-fail-fast`**。只看退出码或
> 第一个红 target，会把「一个 target 红了 26 个」误当成「总共红了 26 个」，从而系统性
> 低估缺口、误判收敛。第一条同类教训见 §3.3（探针在默认仍为 `Plan` 的窗口期跑，把 plan
> 当成了 tree）。两条合起来是一条规则：**测量工具本身要先被验证。**

**本轮已闭环的部分（第八轮）**：第七轮列的四类缺口里 ② ③ 已全部补完 ——
`MONEYFLOW` / `NETINFLOW` / `MAININFLOW` / `MAININFLOWPCT` / `BIGORDER` / `SMALLORDER` /
`SUPERBIGORDER`（7 个）与 `TR()`（1 个），共 8 个 kernel + 8 条 registry 注册。
`dzh_compat_tests` 从 26 红降到 **12 红，且 12 个全部只差 `STRING_LITERAL`**
（`BLOCKINDEX`/`BLOCKAVG`/`BLOCKDATA` 的测试全部要传字符串）。

实现要点：

- **`HostContext` 改为借用（`HostContext<'a>`）。** 原先 `chip: Option<ChipData>` 是
  **按值克隆**，每次 plan 求值都克隆一次筹码分布。再往里塞资金流（9 条序列）和 OHLC
  （3 条序列）就会变成每次求值 14 条序列的 `memcpy` —— 与 plan 路径「为吞吐而生」的目的
  相反。现在 `chip` / `money_flow` 借用、`ohlc` 借用切片，构造即零拷贝；调用点只有 4 处，
  且 `engine.rs` 里用块作用域把借用收在 `output` 上，写回仍能取 `&mut ctx`。
- **`TR()` 不能复用 `TRANGE` 的 kernel。** `TRANGE` 第 0 根是 `NaN`（TA-Lib 口径，没有前收），
  DZH `TR()` 第 0 根是 `high - low`。两者从第 1 根起完全相同 —— 复用会**只错一根**，差分
  语料若只比「两条路径是否一致」根本看不出来。已抽出 `volatility::trange_dzh_into`，
  `fn_tr` 与 kernel **共用同一份实现**，并新增测试直接断言第 0 根 `== high - low`。
- **7 个资金流函数共用一条派发体**（`MoneyFlowKernel` 枚举）。它们的差别只是「取哪条宿主
  序列」，长度守卫必须只有一处 —— 分散成 7 份就一定会有一份漂移。

**87 个缺失 kernel 的构成（这才是真正的待办）**：

| 组 | 数量 | 函数（部分） | 处置 |
|---|---|---|---|
| **域外：回测 / 选股** | ~22 | `FOX_BACKTEST`、`FOX_BUY`、`FOX_SELL`、`FOX_TRADE_SIGNAL`、`FOX_WIN_RATE`、`FOX_PROFIT_RATIO`、`FOX_MAX_DRAWDOWN`、`ENTERLONG`、`AUTOFILTER`、`CHECKSIG`、`MULTSIG`、`SELECTCOND`、`SMARTSELECT`、`SORT`、`TOPN`、`RANK` | **按 §2 产品边界应删除，不是补 kernel。** 补 kernel = 把已定调不做的能力实现一遍 |
| 字符串 / 板块 | 4 | `STRING_LITERAL` + `BLOCKINDEX` / `BLOCKAVG` / `BLOCKDATA` | 见下：设计已由树路径确定，属机械 |
| 宿主数据 | ~12 | `FINANCE`、`DYNAINFO`、`DKCOL`、`INDEXC`、`EM_REF`、`EM_COSTEX`、`EM_ZLCCV`、`EM_ZIG`、`EM_PEAK(BARS)`、`EM_TROUGH(BARS)`、`EM_CROSS` | 需扩 `HostContext`（`block_data` / `index_data` / `em_data` / 财务与动态数据） |
| 普通指标 / 统计 | ~50 | `PDI`/`DMI`/`DX`/`ADXR`/`AROONOSC`、`SKEW`/`KURT`/`DEVSQ`/`AVEDEV`/`PERCENTILE`/`MODE`/`SLOPE`/`FORCAST`、`BARSLAST`/`BARSLASTCOUNT`/`BARSCOUNT`/`BARSSINCEN`/`CURRBARSCOUNT`/`TOTALBARSCOUNT`/`ISLASTBAR`/`BARSTATUS`、`VALUEWHEN`/`COUNT`/`CUM`/`CUMMAX`/`CUMMIN`/`RANGE`/`BETWEEN`/`CEILING`/`POW`/`SIN`/`WR`/`DATE`/`YEAR`/`FROMOPEN`/`CONST`/`LAST`/`PEAK(BARS)`/`TROUGH(BARS)`/`FINDHIGH`/`FINDLOW`/`ZIGZAG`/`MAXPRICE`/`MINPRICE`/`AVGPRICE_N`/`TOTALVOL`/`DPO`/`PSY`/`LWINNER` | 机械，但量大 |

**关键判断**：**翻转默认这件事与产品边界是耦合的。** 87 个缺口里约四分之一是
「按定调不该存在的函数」—— 在删除它们之前补 kernel，等于把 §2 已经划出去的域重新
实现一遍。所以正确的顺序是**先删域外，再补剩余**，而不是先补完再删。

**另有 8 处非 kernel 的分歧（同一批测出来，均未修）**：

| kernel | 错误码 | 现象 |
|---|---|---|
| `MA` / `EMA` / `MACD` | `code 3` (`ERR_PARAMETER`) | 周期大于序列长度或只有 1 根时，树路径的 `canonical_*` 吞掉 `InsufficientData` 返回全 `NaN`，plan 路径抛错。即第七轮已钉住的那条分歧，现在测出它还牵连 `EMA` / `MACD` 与单根输入 |
| `PLUS_DI` / `MINUS_DI` | `code 2` (`ERR_ARITY`) | kernel 要求的实参个数与公式实际传入的不一致 —— 是**真 kernel bug**，不是错误策略分歧 |

**本轮对「字符串字面量」的判断更正**：第七轮记的是「需要先拍板」，实际读代码后发现
**没有可选项** —— 树路径的语义就是「字面量追加进 `FormulaContext::string_table`，
表达式求值为**该表的下标**」（`executor.rs` 追加、`get_string_from_hash` 取回）。
所以 plan 路径只能照抄这条语义：计划携带自己的字面量表，绑定时由引擎注册进
`ctx.string_table` 并把解析出的下标经 `HostContext` 交给 dispatcher，
`STRING_LITERAL` 携带「第几个字面量」作为常量参数。**这不是设计选择，是镜像。**

真正需要拍板的是另一件事：**是否把 §2 划定的域外函数（回测/选股，约 22 个）连同
它们的测试一并删除**。删掉它们会同时消掉 87 个缺口里的约四分之一。

##### 沙箱如何在 plan 路径落地（2026-09-21）

三条限制在 plan 路径上没有一一对应的机制，所以**映射方式必须显式**，不能假装一致：

| 限制 | 树路径 | plan 路径 | 为什么这么映射 |
|---|---|---|---|
| `max_recursion_depth` | 每次 `execute_val` 递归 `+1` | 用 **AST 深度**（lowering 时量出来的 `max_ast_depth`） | 两条路径「吃栈」的地方不同：树路径在执行时递归，plan 路径在**降级**时递归。执行时 plan 已是一张平图，没有可数的递归 |
| `max_memory_bytes` | 顶层成功后记一次 `data_len * 8` | 同左，逐字照搬 | 保持「同配置 ⇒ 同判定」，避免 plan 比树更严而误伤 |
| `timeout_ms` | 每个 AST 节点检查 | **进入时 + 执行后各一次** | 粒度确实更粗（executor 把整张图当一次调用跑）。已在代码注释里写明，没有假装等价 |

`max_ast_depth` 由 lowering 记录进 `FormulaComputePlan`，随缓存一起复用，**不在执行时重算**。
新增 3 个测试：两个钉「限制在 plan 路径确实生效」，第三个钉**反向**——
限制不能误伤普通公式（`MA`、`EMA 差`、`CLOSE+OPEN` 必须照常通过）。

##### 补 kernel 时踩到的两个 SSOT / 语义坑（2026-09-21）

**① 公式函数没进 `registry.rs`，kernel 就是半残的。**
`MINUS`/`HHVBARS`/`LLVBARS`/`CROSSBELOW` 都是**公式面可调用但 registry 里没有**的名字。
未声明 → planner 按 `stateful` 处理 → 该节点永远拿不到纯度（挡 CSE、多一条幽灵尾依赖），
而且门禁 `every_plan_kernel_is_registered_in_the_ssot` 会直接红。
已按实现补齐注册，`LookbackSpec` 逐个核对：

| 函数 | lookback | 依据 |
|---|---|---|
| `MINUS` | `Period` | 预热恰好 n 根（`i >= n` 才有值） |
| `HHVBARS` / `LLVBARS` | `None` | 窗口起点 `saturating_sub`，第 0 根就有值 |
| `CROSSBELOW` | `Dynamic` | 逐根谓词，与 `CROSS` 同形 |

**两组预热规则不同，正是 `MINUS` 与 `HHVBARS/LLVBARS` 必须分开钉的原因。**

**② `VAR` 是「总体方差」，不是「样本方差」——而且差点被写成 `STD * STD`。**
`VAR` 看起来就是 `STD²`，两者只差一个舍入步，所以**用 `stddev_into` 的输出平方来凑
kernel 会通过宽松比较却是错的算术**。实测（同一输入、period 6、窗口
`[102.0, 102.6, 103.2, 103.8, 104.4, 105.0]`）：

| 路径 | `VAR` | `STD` |
|---|---|---|
| tree / `eval` / `eval_with_dialect` | `1.05` = 总体 | `1.0247` = 总体 |
| bytecode | `1.05` = 总体 | `1.0247` = 总体 |
| streaming（TongDaXin） | `1.05` = 总体 | —— |

三条真实路径**本来就一致**（总体口径，`indicators::statistics::var` →
`math::rolling_stats::variance`）。真正的坑是 `functions_legacy.rs` 里那个被 router
遮蔽的 `fn_var` 用的是**样本**方差 —— 如果照着它写 kernel，plan 路径就会和其余三条
全部对不上。现改为直接委托 `rolling_stats::variance_into`（为此把它从私有改成公开，
它就是 `variance` 内部调用的那个函数，比平方少一个舍入步），并用
`check_all_paths("VAR", ...)` + `VAR - STD*STD` 两条断言同时钉住「口径」与
「`VAR == STD²` 关系」。

> **方法论教训**：`VAR` 那次我一度得出「tree 与 bytecode 分叉」的结论，原因是我在
> **默认仍是 `Plan` 的窗口期**跑的探针 —— 探针打印的 "tree" 其实是 plan 路径。
> 翻默认做实验时，**任何旁路测量都必须先确认默认值已经回滚**。

**结论**：plan 路径仍不能当默认，但拦路石已经从「最基础的算术」退到
**纯 kernel 覆盖**这一类机械缺口（详见上文 ③ 的收尾）。

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
7. ✅ **【第八轮新增】域外的「回测 / 选股」公式函数已按方案 A 删除**（用户授权「你全权处理」）。
   翻转默认的缺口实测为 **87 个缺失 kernel / 179 个红测试 / 11 个红 target**（不是此前
   误记的 1 个或 26 个，原因见 §3.3 第八轮）。
   - **实际删除清单（20 个注册，18 个实现）**，按**实现语义**而非名字判定：
     - 回测指标（8）：`FOX_BACKTEST`、`FOX_BUY`、`FOX_SELL`、`FOX_TRADE_SIGNAL`、
       `FOX_TRADE_COUNT`、`FOX_WIN_RATE`、`FOX_PROFIT_RATIO`、`FOX_MAX_DRAWDOWN`
     - 文华信号指令（9）：`AUTOFILTER`、`CHECKSIG`、`MULTSIG`、`ENTERLONG`、`EXITLONG`、
       `ENTERSHORT`、`EXITSHORT`、`BUY`、`SELL`
     - 同花顺条件选股（3）：`SMARTSELECT`、`SELECTCOND`、`TOPN`
   - **`SORT` / `RANK` 已从原清单剔除 —— 它们不是选股。** 二者都是**单序列滚动窗口**
     统计（`SORT(X,N,DIR)` = 当前值在窗口内的排序位次；`RANK(X,N)` = 窗口内百分位），
     与横截面选股无关。**教训：删除清单必须按实现推导，不能按名字**；原清单正是靠名字
     把 `SORT`/`RANK` 误判成选股函数。
   - 连带清理：`templates.rs` 的 15 条 FoxTrader 策略模板、`optimizer.rs` 的
     `SMARTSELECT` 非纯标记、`types.rs` 中**零生产消费者**的 `SelectionResult`
     （选股信号结果类型），以及 6 个测试 target 里的 21 个用例。
   - **未删（刻意保留）**：`ALERT` / `ALERTONCE` 与 `AlertCommand`（预警语义，不在
     「回测 / 选股」两条边界内）、`FOX_ZIG` / `FOX_PEAK` / `FOX_TROUGH`（峰谷指标）。
   - **公式语言面 419 → 399**；registry（254）与 plan kernel（107）不变 —— 这 20 个
     函数**从未在 `registry.rs` 或 FFI 注册**，所以删除不是 FFI 破坏性变更。
8. ✅ **`PLUS_DI` / `MINUS_DI` 的 `ERR_ARITY` 已修**，且**同类问题一并修掉**：
   `AROON_UP` / `AROON_DN`（`resolve_hl_args` 的 1 参形式）与
   `DX` / `ADXR` / `AROONOSC` 同属 `resolve_hl*` 短实参族。见 §9。
9. **`MA` / `EMA` / `MACD` 在「周期 > 序列长度」或「只有 1 根」时 tree=NaN / plan=抛错** ——
   这是**既有**分歧（非本轮引入），已由 `out_of_range_period_is_a_recorded_divergence` 钉住。
   要么让 plan 跟随 tree 返回 NaN，要么明确 plan 的严格语义并同步改 tree。**属行为变更，需你定。**

---

## 9. 第九轮：核心链路验证（2026-09-21）

用户要求：**「继续验证核心功能和核心链路还有哪些潜在问题？全部修复，保证核心功能无错误。」**

### 9.1 修掉的真 bug：`resolve_hl*` 短实参在 plan 路径上 `ERR_ARITY`

**现象**：`PLUS_DI(CLOSE, 14)` 树路径算得出值，plan 路径 `ERR_ARITY`（`code 2`）。
**根因**：树路径的 `resolve_hlc_args` / `resolve_hl_args` 会把缺失的价格序列从
**求值上下文**补齐：

| 解析器 | 函数 | 显式形式 | 短形式 | 周期取自 |
|---|---|---|---|---|
| `resolve_hlc_args` | `DX` `PLUS_DI` `MINUS_DI` `ADXR` | ≥4 参 | 2–3 参（补 `HIGH`/`LOW`/`CLOSE`） | **参数 1** |
| `resolve_hl_args` | `AROONOSC` `AROON_UP` `AROON_DN` | ≥3 参 | 1–2 参（补 `HIGH`/`LOW`） | **参数 0** |

plan 路径**没有上下文可补** —— 它的 input layout 只能承载源码里**写出来**的序列名。
**修复**：在 `FormulaComputePlan::compile_with_registry` 里新增
`expand_implicit_price_args`（`formula/params.rs`），把短形式改写成显式拼写。
`HIGH`/`LOW`/`CLOSE` 经 `FormulaContext::get_data` 解析到的**正是**上下文里那三条序列，
所以这是**保值的**（不是新语义）。

**三个关键判断**：
1. **改在 lowerer 里，不是 `FormulaEngine::build_plan` 里。** 差分门禁
   （`formula_differential_tests.rs` / `formula_plan_differential.rs`）**绕过引擎**，
   直接 `parse_formula` + `FormulaHotPlan::compile` —— 放在 `build_plan` 里的话
   门禁**根本测不到**。（这是本轮发现的门禁盲区，值得记住。）
2. **镜像解析器的实参位置，包括它的怪癖**：`resolve_hlc_args` 的 3 参形式读**参数 1**
   并把最后一个参数丢掉。expand 照抄这个行为 —— 本轮目标是让两条路径**一致**，
   不是重新设计一个调用方可能已经依赖的契约。
3. **Pine mapper 已经展开过的调用不会被二次展开**：展开后实参数已不在短形式区间。
   该 pass 只会**增加**操作数。

**验收**：`formula_differential_tests::formula_differential_implicit_price_arguments`
（短形式 + 显式形式，四条路径全比），33 passed / 0 failed。

### 9.2 审计结论（无改动）

- **5 处 `unreachable!()` 全部不可由输入触达**：`error.rs:451` 在 `#[cfg(test)]` 内；
  `stateful.rs:1331` 被 `direct_expression` 的前置校验保证；`cycle.rs:1231` 是
  const-generic `MODE`（只实例化 1/3/5）的死分支；`rank.rs:48` 在已排除该 policy 的
  match 臂内；`operation.rs:2254` 的 `talib_ma_type` 在 `> 8.0` / `fract() != 0.0`
  已返回错误后必然落在 0..=8。
- **索引访问有纪律**：抽查 `cycle.rs`、`classic_patterns.rs`、`classic_tools.rs`、
  `momentum.rs` 的 68 处 `x[0]`/`x[1]`，全部有 `validate_input` 或等长前置校验。
  `point_and_figure` 先查 `high.len() != low.len()` 再 `validate_input(high.len(), 1)`。
- **`unified_dispatch.rs` 的 `unsafe from_raw_parts` 是可靠的**：`len` 取自
  `buffers[output_slot].len()`，而 `BufferLayout::take_buffers` 对**每个** slot 都
  `take_overwrite(len)`，所以所有 buffer 等长；`validate_inputs` 又强制所有输入等长。
- **资金流 7 个 kernel 与树路径逐项同构**（`fn_netinflow` 的 level 映射、越界回退、
  长度不符 → NaN 全部一致）。
- **全部生成物门禁通过**：`check_versions` / `gen_ssot_docs --check` /
  `check_docs_links`（83 文件）/ `check_orphan_modules`（38 模块）/ `sync_bindings --check --all`。

### 9.3 文档说谎：宣称了划出去的域

按「**删代码不删宣称 = 文档在撒谎**」逐条核对，发现 4 处：

| 文件 | 原文 | 处理 |
|---|---|---|
| `api-reference-zh.md` | 目录 + 章节标题「跨市场**选股**公式」 | →「跨市场**信号**公式」，并补一句「只做逐 bar 计算，不含横截面排序 / 标的池 / 回测」 |
| `screening-formulas.md` | 标题「Cross-market **screening** formulas」、正文「**screening** layer for common **selection** logic」 | →「Cross-market **signal** formulas」+ 显式 Scope 声明 |
| `competitive-positioning-zh.md` | 对比矩阵行 `portfolio/backtest product \| lightweight / 非核心` | → `❌ 不做（只做底层计算）` |
| `api-reference-zh.md:1853` | 「适合批量扫描、**回测**和实时循环」 | 保留（说的是「可被回测系统使用」，不是「提供回测」） |

**未动**：`factor-research-architecture.md` 全文仍在描述已删的 `core/src/backtest.rs`
（约 10 处），但抬头已有 ⚠️ 失效声明，属**诚实的历史设计文档**。是否归档到
`docs/archive/` 由你定 —— 它有 8 处文档入链，归档需连带改链。

### 9.4 已知但未改（需要你拍板，均属行为变更）

1. **`ema_into` 与 `ema` 的 NaN 策略不一致**：`ema_into` → `ema_simd_into` →
   `ema_scalar` 用 `data[..period]` 的简单均值作种子，**种子含 NaN 会污染整条序列**；
   而 `ema()`（`Array1` 版）与 `ema_fast_into` 都显式 `reject_if_non_finite` 报错。
   两条路径**一致地**产出全 NaN，所以差分门禁看不见 —— 但它与
   **TA-Lib 行为相符**（TA-Lib 的 SMA 种子同样被 NaN 污染并传播），
   所以**不一定是 bug**，只是 `_into` 与 `Array1` 两个入口的契约不一致。
2. **`factor-research-architecture.md` 是否归档**（见 9.3）。
3. **`screening.rs` 模块名**：`core/src/indicators/screening.rs` 的 9 个函数都是
   **单/双序列逐 bar 信号计算**，且**已在 `registry.rs` 注册**（属公开 API + FFI 面），
   所以只改了文档措辞、**没有删模块**。若你认为模块名「screening」本身就是产品定位
   问题，需要一次破坏性变更 —— 请确认。
4. §8 第 9 条（越界周期 tree=NaN / plan=抛错）仍未决。

---

## 10. 第十轮：滚动指标不可组合（2026-09-21）

### 10.1 症状

**任何滚动指标都无法通过变量组合。** 用干净数据（200 根、无缺失）验证：

| 公式 | 修复前 |
|---|---|
| `X:=MA(CLOSE,5); Y:=MA(X,9); Y` | 整条 NaN |
| `DIF:=EMA(CLOSE,12)-EMA(CLOSE,26); DEA:=EMA(DIF,9); (DIF-DEA)*2` | 整条 NaN |
| `X:=MA(CLOSE,5); Y:=SUM(X,9); Y` / `STD` / `VAR` / `WMA` / `TRIMA` / `TRIX` | 整条 NaN |
| `BOLL(MA(CLOSE,5),20,2)` / `ATR(MA(HIGH,5),MA(LOW,5),MA(CLOSE,5),9)` | 整条 NaN |

内置的 `MACD(CLOSE,12,26,9)` 一直正常 —— 因为它是**手写的、没有组合**。
一旦用户按定义手写 MACD（`DEA:=EMA(DIF,9)`），就全 NaN。

### 10.2 根因

两个事实叠加：

1. **每个滚动指标的输出都以「前导 NaN 预热段」开头**（`MA(CLOSE,5)` 前 4 个是 NaN）。
   这是结构，不是数据。
2. **滚动核用增量累加器**（`sum += input[i] - input[i-period]`），而 `NaN` 是**吸收态**
   （`NaN - NaN = NaN`）。核从 `input[..period]` 取种子 → 种子含 NaN → **整条序列永久 NaN**。

于是「把 A 的输出喂给 B」必然失效。

### 10.3 为什么所有门禁都没发现

- **差分门禁只断言「两条路径一致」**，而 `values_match` 把 `NaN == NaN` 算作一致。
  树路径和 plan 路径**错得一模一样**，所以门禁全绿。
- 唯一能发现它的是**断言有限值个数**的探针。

### 10.4 修法（用户拍板：math 层 NaN-only 分支）

在 `math` 层引入 `leading_warmup(input)`（第一个有限值的下标），让滚动核**从该处起算**：

- 前导非有限段 = 上游指标的预热前缀，**跳过**，不是坏数据；
- **`start == 0` 时逐位等同原实现** → goldens / benchmark 一个数字都不动；
- R-1 仍拒**内部**非有限（既有拒绝测试全部使用内部 NaN，因此放宽「允许前导段」不会弄红它们）。

**为什么不改公式层**：公式层有 78 处 `Err(_) => Ok(nan_vec(..))` 调用点，逐个包裹等于把
同一个规则抄 78 遍；math 层一处生效。

**实现模式（无分配）**：`let (tail_in, tail_out) = (&input[start..], &mut output[start..]);`
然后**递归调用自己**。递归那一层 `tail[0]` 必然有限 → `leading_warmup == 0` → 不会二次递归。
比 `vec!` 临时缓冲干净，也避开 `no_std` 没有 `alloc` 的问题。

### 10.5 改动清单（13 个文件）

| 文件 | 改动 |
|---|---|
| `math/mod.rs` | 新增 `pub(crate) fn leading_warmup`（唯一定义） |
| `math/moving_avg.rs` | `sma_inner` / `sma_into` / `ema_inner` 前导偏移；`wma` 改用 `reject_if_non_finite`；`wma_kernel_into` 种子与滑动偏移；`trima_into` 尾部递归 |
| `math/fast_moving_avg.rs` | `reject_if_non_finite` 放宽（只拒内部）；`ema_into` / `wma_into` 前导偏移 |
| `math/rolling_stats.rs` | `variance_into` / `stddev` / `stddev_into` / `bbands_upper_into` 尾部递归 |
| `math/kernels/compat.rs` | `rolling_sample_variance_into` 从第一个有限值起喂 Welford |
| `indicators/overlap.rs` | `bbands` / `bbands_into` 放宽为只拒内部 + 尾部递归 |
| `indicators/volatility.rs` | `atr_into` / `natr_into` 按三条序列**最宽**的前导段整体偏移 |
| `indicators/momentum.rs` | `rsi` 的 metrics 门控拒绝改为跳过前导段（原与 `rsi_into` 不对称）；`trix_into` 尾部递归 |
| `formula/functions_legacy.rs` | `fn_sum` 累加器从第一个有限值起（SUM 没有 math 层对应物）；`fn_dema` / `fn_tema` / `fn_macd` 走 `warmup_offset` + `shift_back` |
| `formula/functions.rs` | 新增 `warmup_offset` / `shift_back`；`canonical_bband_component` / `canonical_atr` 接上 |

### 10.6 踩到的坑（已写进 `TRAPS.md` §D）

**同一个核有两份实现，且其中一份是「覆盖」而非「并列」。** `math/mod.rs` 里

```rust
pub mod moving_avg {
    pub use super::fast_moving_avg::{ema_into, kama, kama_into, wma_into}; // 遮蔽！
    pub use super::moving_avg_legacy::*;
}
```

显式 `use` **遮蔽** glob 导入的同名项 → `moving_avg::ema` 走 legacy，而
`moving_avg::ema_into` 走 `fast_moving_avg`，**两个不同的 `reject_if_non_finite`**。

只修 legacy 那份的直接后果：**树路径（走 `ema`）修好了、plan 路径（走 `ema_into`）仍全 NaN
→ 制造出新的树/plan 分叉**（比原来「两边一致地错」更糟，因为差分门禁会红）。
`BOLL` / `ATR` 也踩了同一个坑（树路径在 `formula` 层、plan 路径在 `unified_dispatch`），
最终两边都收敛到 math 层才一致。

### 10.7 验证

| 项 | 结果 |
|---|---|
| lib 测试 | 2969 passed / 0 failed / 1 ignored |
| 组合探针（40 个用例 × 树/plan） | 两路径可执行处**全部一致且有有限值**；`plan=UNSUPPORTED` 的用例已在 §10.8 记录 |
| 既有拒绝测试 | `edge_case_invalid_input.rs` / `property_tests.rs` 全绿（内部 NaN 仍报错） |
| goldens / benchmark | 未动（`start == 0` 短路保证） |

新增回归门禁 `core/tests/formula_differential_tests.rs`：

- `formula_differential_rolling_composition` —— 16 个组合用例，**同时**跑两路径并**钉住有限值个数**
  （这才是能发现本缺陷的断言）；
- `formula_rolling_composition_of_composites` —— `DEMA`/`TEMA`/`MACD`/`APO`/`STOCH` 的树路径用例，
  带注释说明「kernel 落地后上移到上面那个测试」。

### 10.8 未闭环（不属本缺陷）

`DEMA` / `TEMA` / `MACD(<表达式>, ...)` / `APO` / `STOCH` / `COUNT` / `DMA` 在 **plan 路径没有 kernel**
（`plan=UNSUPPORTED`），属 §7 第 1 条的「剩余 ~65 个机械缺口」；树路径已正确。
`#[default]` 仍是 `Tree`，所以这不是线上缺陷。

---

## 11. 第十一轮：simple-formula 快速路径吃掉预热段（2026-09-21）

### 11.1 发现方式

第十轮的教训是「**等价门禁看不见两边一致地错**」，所以本轮换了方法：写**绝对性质探针**
（断言有限值个数，而不是断言两路径一致）。探针枚举「根节点形如 `NAME(<变量>, <字面量>)`」
的公式，把带预热段的序列**通过 `set_variable` 绑定成变量**再求值。

一次就命中 **10 个用例**（`MA`/`EMA`/`RSI`/`BOLLMID`/`SUM`/`HHV`/`LLV`/`STD`/`WMA`/`SUM(X,9)+0`）：
引擎返回**全 NaN**，而 plan 路径返回 188~191 个有限值。

### 11.2 根因

`FormulaEngine` 有三条「简单公式」快速路径：

| 函数 | 入口 |
|---|---|
| `try_execute_simple_formula` | `execute` |
| `try_execute_simple_formula_into` | `eval_into` |
| `try_execute_simple_formula_slices` | `eval_zero_copy_inputs` |

三条都是同一段结构：匹配 `FunctionCall` → 取 `args[0]` 为变量、`args[1]` 为字面量 →
**然后先做非有限值检查，最后才 match 函数名**。

```rust
if input.iter().any(|value| !value.is_finite()) {
    return Some(Array1::from_elem(ctx.data_len, f64::NAN));   // ← 在 match 之前
}
match name.to_ascii_uppercase().as_str() {
    "MA" | "BOLLMID" => ..., "EMA" => ..., "RSI" => ..., _ => return None,
}
```

两个后果：

1. **未支持的函数名也被这里回答**。`SUM`/`HHV`/`LLV`/`STD`/`WMA` 根本不在 match 里，
   但只要**结构**匹配且输入含非有限值，就会返回全 NaN —— 永远不会落到通用执行器。
2. **`SUM(X,9)+0` 也中招**：优化器把 `+0` 恒等式折叠掉，根节点又变回那个调用。

第十轮之前这不表现为缺陷，因为通用执行器对同样的输入也返回全 NaN（**两边一致地错**）。
math 层学会「跳过前导预热段」之后，通用路径算出有限值，而快速路径仍返回全 NaN
→ **变成真实的快速路径 vs plan 分叉**。

### 11.3 修法

规则收敛为一条：**快速路径只处理完全有限的输入，其余一律落回通用执行器**；
且**函数名检查必须在非有限值检查之前**。

```rust
let upper = name.to_ascii_uppercase();
if !matches!(upper.as_str(), "MA" | "BOLLMID" | "EMA" | "RSI")
    || input.iter().any(|value| !value.is_finite())
{
    return None;   // 或 _into 变体里 return false
}
```

为什么「非有限就落回」是安全的：

- **前导** NaN 段 → 通用执行器按 math 层规则**算过去**（这正是要修的）；
- **内部** 非有限值 → 通用执行器走 `fn_ma` 等包装函数，`Err(_) => Ok(nan_vec(..))`
  → 仍然全 NaN。**可观测结果不变**，只是不再由快速路径给出。

三条路径都改了，避免只改一条造成新的分叉（第十轮的教训）。

### 11.4 验证

| 项 | 结果 |
|---|---|
| 探针（16 用例 × 快速路径/plan） | 修前 **10 处分叉**，修后 **0 处**，全部 `AGREE` |
| 新增门禁 `formula_differential_simple_formula_fast_path_defers` | 15 用例，同时断言两路径一致**与**精确有限值个数 |
| `formula_differential_tests` | 36 passed / 0 failed |

### 11.5 方法论（已固化为 skill）

绝对性质探针 → 定位 → 修共享层 → 升格为正式门禁 → 删探针。
已存为 skill `equivalence-gate-blind-spot`（差分门禁盲区审计）。


---

## 12. 第十二轮：库层增量累加器的预热段污染（同型缺陷的第三处）

第十轮（滚动核）、第十一轮（formula 快速路径）、第十二轮（库层累加器）是**同一个缺陷类的三次显形**。
共同机制：`NaN` 对增量递推是**吸收元**（`NaN - NaN = NaN`），而滚动指标的输出天然带一段
**前导 NaN 预热段**。只要递推的种子窗口覆盖到那段，整个序列就永久 NaN。

### 12.1 发现方法

沿用第十一轮的**绝对性质探针**：把 `MA(CLOSE,5)`（196/200 有限）绑定为变量 `X`，
对 36 个滚动/统计/回归函数断言**有限值个数**（而不是两路径是否一致）。

结果：**6 处全 NaN** —— `AVGDEV`、`LINEARREG`、`LINEAR_REG`、`LINEARREG_SLOPE`、
`LINEARREG_INTERCEPT`、`TSF`。

注意 `AVEDEV(X,9)` 是好的、`AVGDEV(X,9)` 是坏的 —— 同一指标的两份实现，一份自愈一份不自愈。
这个不对称是定位整个缺陷类的钥匙。

### 12.2 根因：库层累加器

这些函数都维护 `sum += newest - oldest` 形式的增量累加器，种子取自 `input[..period]`：

| 站点 | 函数 |
|---|---|
| `math/linear.rs` | `linreg` / `linreg_slope` / `linreg_intercept` / `linreg_angle` |
| `indicators/statistics.rs` | `avgdev` / `zscore` / `zscore_into` / `tsf` |
| `math/kernels/compat.rs` | `sma_into` / `wma_into`（`MovingAverageState`） |

`fn_zscore` 走的是 `lib_stat::rolling_mean`（**不是** `statistics::zscore`），
而 `rolling_mean` → `compat::rolling_mean_into` → `compat::sma_into` → `MovingAverageState`。
所以只修 `statistics.rs` 修不好 `ZSCORE`。

### 12.3 修法：种子与循环全部按 `warm_start` 偏移

```rust
let warm_start = crate::math::leading_warmup(input);   // 全 NaN 输入返回 input.len()
if warm_start + period > len { return Ok(output); }    // 有限尾巴不够长 → 保持全 NaN
// 种子：input[warm_start..warm_start + period]
// 首值：output[warm_start + period - 1]
// 循环：for i in warm_start + period..len
```

`warm_start == 0`（无 NaN 输入）时 `skip(0)` 与 `+0` 偏移都是空操作 → **逐位不变**，
golden / benchmark 不受影响。`compat.rs` 用第十轮 `rolling_sample_variance_into`
的既有形状（`.skip(start)` + `index + 1 >= start + window`）。

### 12.4 自己踩的坑：guard 忘了偏移（只在 plan 路径暴露）

`zscore_into` 把「滑窗」折进了主循环：

```rust
for index in warm_start + timeperiod - 1..input.len() {
    if index >= timeperiod {          // ← 忘了偏移！
        let old = input[index - timeperiod];
        sum += new - old;             // old 落在 NaN 前缀里 → 永久 NaN
    }
```

在**种子那根 bar** 上窗口还没开始滑动，但 `index >= timeperiod` 已经成立 →
驱逐了 `input[warm_start - timeperiod]`。必须写成 `index >= warm_start + timeperiod`。

**为什么只有 plan 路径暴露**：tree 路径的 `fn_zscore` 根本不调 `zscore_into`，
plan kernel 才调。所以症状是 `tree=188 plan=0` —— 一个**方向相反的**分叉。
这也说明：**改完共享层必须两条路径都跑**。

### 12.5 连带发现：一个被全 NaN 掩盖的既有分叉

修完预热段后 `ZSCORE` 变成 `tree=188 plan=188`，但两路径**数值不等**：

| 输入 | tree vs plan `max_abs` | `max_rel` | 位置 |
|---|---|---|---|
| NaN-free `ZSCORE(CLOSE,9)` | **2.243e-10** | 1.536e-10 | 199 |
| 预热段 `ZSCORE(X,9)` | 2.248e-10 | 1.539e-10 | 199 |

**NaN-free 输入上就已经超出门禁的 1e-10 绝对容差** —— 这是**既有缺陷**，不是本轮引入的。
它一直被两件事掩盖：(1) 全 NaN 时 `values_match` 判 NaN/NaN 相等；
(2) 语料里从没用精确容差覆盖过 `ZSCORE` 的 plan kernel。

根因是**两份不同算法**：

| 实现 | σ 的算法 | 实测差异 |
|---|---|---|
| `fn_zscore`（tree，手写） | `rolling_std_dev` → **Welford 可移除方差** | 基线 |
| `zscore_into`（plan kernel） | 朴素 `sum` / `sum_sq` 递推 | **2.524e-10** |
| `statistics::zscore` | 与 `zscore_into` **逐位一致**（`max_abs = 0`） | — |

**修法**：按项目既有政策（两份实现冲突时收敛到有参考覆盖的那份，删掉手写那份），
让 `fn_zscore` 直接委托 canonical 实现 —— 与 `fn_avgdev`/`fn_std` 同形：

```rust
match lib_zscore(values, n) {          // indicators::statistics::zscore
    Ok(result) => Ok(result),
    Err(_) => Ok(nan_vec(data_len)),
}
```

方向说明：**plan 路径一个字节都没动**（它本来就用 canonical），改的是 tree 路径这份
手写重复实现。NaN 语义不变（`zscore_into` 同样有 `std_dev > 1e-15` 门槛）。

### 12.6 验证

| 项 | 结果 |
|---|---|
| 探针（36 用例） | 修前 **6 处全 NaN** → 修后 **0 处** |
| `ZSCORE` 两路径 | 修前 `tree=0 plan=0` → 修后 `tree=188 plan=188` 且逐位一致 |
| 新增门禁 `formula_differential_incremental_accumulators_survive_warmup` | 13 用例精确有限值 + `ZSCORE` plan 一致性 + **重复实现互校**（`AVGDEV`≡`AVEDEV`、`SLOPE`≡`LINEARREG_SLOPE`、`FORCAST`≡`LINEARREG`）+ NaN-free 输入的预热形状不变 |
| `formula_differential_tests` | 38 passed / 0 failed |
| `formula_compat` | 73 passed / 0 failed |
| **全量 `--no-fail-fast`** | **50 targets / 4013 passed / 0 failed / 16 ignored，EXIT=0** |
| 文档门禁 + rustfmt | 全绿（versions 0.1.15 / SSOT / links 83 / orphan 38） |

**全函数面分叉复扫**（NaN-free 输入，`tree` vs `plan` 逐点 `max_abs`，阈值 1e-10）：
扫了 31 个代表函数（`BOLL*`/`STD`/`VAR`/`HISTVOL`/`ZSCORE`/`AVGDEV`/`TSF`/`LINEARREG*`/
`MA`/`EMA`/`WMA`/`RSI`/`CCI`/`CORREL`/`SUM`/`HHV`/`LLV`/`MEDIAN`/`RMA`/`TRIMA`/`TRIX`/`SKEW`/`KURT`），
**超出 1e-10 的 = 0 处**；最大的两个是 `BOLL` 4.2e-12、`WMA` 5.7e-14（都在容差内）。
`ZSCORE(CLOSE,20)` 修后 **`max_abs = 0`（逐位一致）**。
→ 说明「重复实现数值分叉」这一类在公式面上**只有 `ZSCORE` 一处**，已闭环。

门禁里那条**重复实现互校**是本轮新增的断言类型：它在两条路径**一致地错**时依然有效，
正好补上差分门禁的盲区。

### 12.7 教训

1. **「两路径一致」不是正确性**。全 NaN == 全 NaN 在 `values_match` 里算相等。
2. **修一个缺陷会激活另一个被掩盖的缺陷** —— 本轮一次修出两处（`zscore_into` 的 guard、
   `fn_zscore` 的算法分叉），都是修完预热段才看得见。
3. **同一逻辑指标可能有三份实现**（`fn_zscore` 手写 / `statistics::zscore` / plan kernel 的
   `zscore_into`）。修之前先 grep 清楚到底有几条路。
4. **改完共享层必须两条路径都跑**：`zscore_into` 的 guard 错误只在 plan 路径可见。
5. **guard 的每一个下标都要按 `warm_start` 偏移**，不只是种子和循环边界。
