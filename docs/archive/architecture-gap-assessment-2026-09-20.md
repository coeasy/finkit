# Finkit 架构现状与目标差距评估（2026-09-20）

> 评估对象：`feature/finkit-v1-unified-engine-20260917` @ `f441e76`
> 评估口径：只采信本机可复现的证据（源码位置 / 脚本实测输出 / 编译结果）。未验证项一律标注为「未验证」，不写成「已支持」。
> 对照目标：`docs/finkit-unified-quant-platform-refactor-plan-v6.md`（最高架构基线）与 `docs/FINAL_REFACTOR_PLAN_2026-09-19.md`（收敛方案）。

---

## 1. 结论

**方向符合预期，落地存在系统性落差。**

- 目标架构与文档体系：**高度一致**。V6 与最终收敛方案描述的分层（AST → Typed IR → DAG → ExecutionPlan → Executor）与产品定位，和对外表述的目标基本逐条对应，且文档本身极其诚实（明确写了「不能宣称全面超过 TA-Lib」「未验证项不视为通过」）。
- 实现程度：**呈「两头实、中间虚」**。两头（底层数值内核 / TA-Lib 201 项 parity、上层多语言 binding 与 contract）很扎实；中间的**统一编译执行链（IR / DAG / 统一 Executor）已实现但未接入生产路径**——默认执行仍是 AST 逐节点树遍历。
- 因此，愿景第一句「任何来源的金融公式 → 统一 AST → Factor IR → 优化 DAG → Rust Runtime 执行」目前**只有前半段（统一 AST）真正在生产生效**，后半段（IR / DAG / Runtime）是"已建成但没通电"。

一句话：**不是设计跑偏，而是"最后一公里"没接上；同时多语言 SSOT 存在一个会误导发布判断的检查漏洞。**

---

## 2. 本机实测基线

| 项目 | 结果 |
|---|---|
| 核心 crate 编译 | `cargo check -p finkit --lib` **通过**（fresh clone，50s，0 error） |
| 依赖来源 | `Cargo.lock` 齐全，可离线构建 |
| 工作区规模 | 1547 个受版本控制文件，74 份文档 |
| 本机构建障碍 | 环境问题，非项目问题：coreutils 的 GNU `link` 抢占 MSVC `link.exe`；需手动注入 `LIB`（Windows SDK 路径）；`vcvars64.bat` 因沙箱禁用 `reg.exe` 无法运行。已用「显式 linker + 手动 LIB」绕开 |

---

## 3. 差距一：统一编译执行链未接入生产路径（P0，价值最大）

| 层 | 声称 | 实际 | 证据 |
|---|---|---|---|
| 统一 AST | ✅ | **真实生效** | `core/src/formula/ast.rs`；`formula/mod.rs:158 parse_formula_with_dialect`；Pine 经 `pine/ast_mapper.rs:38` 映射到同一 AST |
| Factor IR / DAG | ✅ | **已实现，未接线** | `core/src/compute.rs:178 ComputePlan`（拓扑序 + 环检测）；`formula/compute_ir.rs:18 FormulaComputePlan`；`formula/hot_plan.rs:20 FormulaHotPlan`（含 DAG 级 CSE `:145`）。但 `FormulaHotPlan` 的引用**只出现在测试中** |
| 统一 Executor | ✅ | **已实现，未接线** | `core/src/unified_executor.rs:169 UnifiedExecutor`、`formula/unified_dispatch.rs:14 FormulaKernelDispatcher`——除自身与测试外**无生产调用方** |
| 默认执行模型 | — | **AST 树遍历** | `formula/executor.rs:56/88` `match ast {...}` 逐节点解释；函数按整数组求值（`functions.rs:96 fn_ema`） |
| 字节码 VM | — | 真实但旁路 | `formula/bytecode.rs:394 BytecodeVM` 完整；仅经 FFI 作为 opt-in 暴露（`ffi/python-binding/src/lib.rs:2064`、`node-binding/src/lib.rs:2531`、`java-binding/src/lib.rs:2362`、`dotnet-binding/src/lib.rs:847`） |
| JIT | 命名存在 | **桩级** | `formula/jit.rs:80 compile` 只做字节码 peephole；`:119 execute_optimized` 是 `while pc < ...` 解释循环，无 cranelift/LLVM/mmap codegen |
| SIMD | ✅ | **真实** | `formula/simd.rs` AVX2/NEON/AVX-512；`executor.rs:830` 实际调用 |
| 优化器 | ✅ | **AST 级真实，DAG 级缺** | `formula/optimizer.rs` 常量折叠/DCE/代数化简/强度削减/CSE/循环不变量；**无** DAG 调度、算子融合、并行切分 |

**影响**：跨节点公共子表达式复用、liveness/buffer 复用、并行调度、算子融合这些"统一 Runtime 才有"的收益目前拿不到；`hot_plan` / `UnifiedExecutor` 属于「有代码、无流量」，长期会腐化。

---

## 4. 差距二：方言层「声明 5 个、实际 1 套解析器」

| 体系 | 目标 | 实际状态 | 证据 |
|---|---|---|---|
| TA-Lib 0.8.0 | ✅ | **完整**（201/201 parity，201 goldens） | `docs/talib-0.8.0-coverage-audit-2026-09-19.md`；`tests/golden/talib/*.json` 202 个 |
| 通达信 TDX | ✅ | 支持 | `FormulaDialect::TongDaXin`；`tests/formula_corpus/*_tdx.json` 10 个 |
| 同花顺 THS | ✅ | 支持 | `CLOSE1→REF(CLOSE,1)` 别名；2 个 corpus |
| 东方财富 EM | ✅ | 支持 | `FormulaDialect::EastMoney` |
| **大智慧 DZH** | ✅ | **非一等** | **无方言枚举**；`tests/formula_corpus.rs:51` 把 `"dzh"` 直接映射到 `TongDaXin`；块函数存在但无 profile |
| **文华财经 WH6/WH8** | ✅ | **几乎缺失** | 仅 `core/src/formula/functions.rs:5297` 注册了 `AUTOFILTER/CHECKSIG/BUY/SELL` 等信号函数 + `core/tests/formula_compat.rs` 7 个测试；**无 terminal、无 dialect、无 corpus、docs 零提及** |
| TradingView Pine | ✅ | 受控子集 | 33 个内置函数；26 个 corpus 中 25 通过、1 个 host_required；`strategy()/UDT/import/array.*/plot 对象` 明确不支持 |
| **pandas-ta 风格组合** | ✅ | **完全缺失** | 全仓库仅在竞品分析文档出现；最接近的是 `core/src/polars_ext/`（仅 4 个指标） |
| 自定义因子 DSL | ✅ | 存在但受限 | `formula/custom.rs`：仅表达式级，无赋值/控制流 |

**关键结构问题**：`FormulaDialect`（`formula/mod.rs:100`）的 5 个变体中，TDX / THS / EastMoney **全部路由到同一个 `parse_formula`**（`mod.rs:162-166`），只有 Pine 有独立解析器。也就是说「每个方言独立 profile、不强行合并同名函数」目前只体现在 `SemanticProfile` 元数据层，**解析层并未真正分方言**——终端特有语法缺口无法在 parse 阶段被拦截。

---

## 5. 差距三：多语言 SSOT 断裂（P0，会误导发布判断）

这是本次评估**最需要立刻处理**的问题：项目对外承诺「同一 Rust canonical core + 一套 SSOT → 多语言一致」，但代码生成链路只对 Python 闭环。

**实测证据（本机运行，非推断）**：

```
$ python scripts/sync_bindings.py --check --all
[check/c]      registry=78 extracted=92  drift=none
[check/python] registry=78 extracted=130 drift=none
[check/node]   registry=78 extracted=144 drift=none
[check/go]     registry=78 extracted=85  drift=none
[check/dotnet] registry=78 extracted=62  drift=none
[check/ios]    registry=78 extracted=18  drift=none
[check/java]   registry=78 extracted=122 drift=none
[check/android]registry=78 extracted=15  drift=none
exit=0
```

看起来 8 种语言全部干净。但：

```
$ python -c "...统计 docs/ffi_registry.json 的 bodies..."
indicator count: 78
stored bodies per lang: {'python': 71}
```

- `scripts/sync_bindings.py:573` 的逻辑是 `if body_stored is None: continue`——**没有存储 body 的语言直接被跳过，却仍打印 `drift=none`**。因此上表中 c / node / go / dotnet / ios / java / android 的 `drift=none` 是**空检查**，不构成任何证据。
- `scripts/gen_binding.py` 已是**死路径**：`--check` 输出 `registry indicator count: 0`（真实 registry 是 78），说明它读的 registry 视图早已不含 `ffi` 块。

**影响**：V6 §12「统一描述 + 自动生成」与 R9「Multi-language SDK V2」的核心前提不成立；`drift=none` 这种输出会让发布评审误判「多语言已一致」。

**绑定成熟度（综合）**：

| 语言 | 代码生成 | 对外 API | 测试 | 发布 | 判定 |
|---|---|---|---|---|---|
| Python | 1939 行 / 71 | 指标+14 JSON+流式+因子/研究+图表 | 12 文件 ~2000 行 | maturin + wheels CI | 完整 |
| Node | 1170 行 / 76 | 指标+14 JSON+28 流式类 | 2 文件 310 行 | npm + 8 平台包 | 完整（测试偏薄） |
| C/C++ | 2350 行 / 78 | 78 指标 + kline | 3 文件 ~1070 行 | CMake | 指标完整，**无流式** |
| Java | 801 行 / 42 | 42 指标 + 模式 + 图表 | 1 文件 394 行 | pom + CI | 能跑，偏薄 |
| .NET | 1246 行 / 41 | 41 指标 + 研究 | 1 文件 456 行 | csproj + CI | 能跑，偏薄 |
| Go | 947 行 / 40 | 40 指标 + 7 流式族 | 985 行 / 50 测试 | go.mod + CI | 能跑，偏薄 |
| iOS | 305 行 / 15 | 15 指标 | **无** | xcframework CI | 骨架 |
| Android | 36 行 / 15 | 15 指标 | **无** | AAR CI | 骨架 |

流式能力分布：核心 `core/src/streaming/` 145/236 指标标记 streaming，但 FFI 侧 Python ~12 类、Node 28 类、Go 7 族，**C/C++/Java/.NET/iOS/Android 为 0**。

---

## 6. 差距四：双轨 crate 与文档 sprawl（P1/P2，工程健康度）

1. **双轨 crate**：`crates/finkit-array | finkit-series | finkit-math | finkit-factor | finkit-runtime` 构成一条平行的 V2 骨架。实测依赖关系：**core / ffi / cli / wasm / visualization 没有任何一个依赖它们**，五个 crate 只互相引用。`docs/FINAL_REFACTOR_PLAN_2026-09-19.md` 自己也写明「本轮没有把旧 `crates/finkit-runtime` 单依赖 Executor 扩展成第二套生产 Runtime」——即：**这是一条在飞但没接地的迁移轨道**。在它接入生产或明确废弃之前，任何"Runtime 已统一"的表述都不成立。**（2026-09-20 更新：已解决 —— 其中 3 项真缺口已接入生产（R2/R3/R4），五个 crate 随后整体删除；见 [`runtime-carrier-adoption-plan-2026-09-20.md`](../runtime-carrier-adoption-plan-2026-09-20.md) §6。）**
2. **文档 sprawl**：`docs/` 共 74 个文件，其中架构/重构计划类 ≥10 份且内容高度重叠（V2 / V3 / V4 / V5 / V6、`ARCHITECTURE_REVIEW_2026-09-17`、`-09-18`、`FINAL_2026-09-19`、`optimal-architecture-refactor-plan-v5`、`pr28-architecture-v3-refactor-plan`、`finkit-architecture-review-refactor-plan-v4` …）。**判断当前真实进度需要读 5 份以上文档且互相矛盾**，这本身就是风险。建议收敛为「1 份权威基线 + 1 份执行状态」，其余归档到 `docs/archive/`。

---

## 7. 性能与门禁现状（沿用项目自有结论，未在本机复测）

- 数值：TA-Lib 201/201 parity，`parity_failures=[]`。
- 性能：**release 性能门禁未通过**。最新记录 top-20 最低约 `1.0362x`，门槛 `1.05x`；指标几何平均 `1.7433x`、100K `1.8398x`、1M `1.5878x`。
- 结论口径正确：文档已明确「不能宣称全面超过 TA-Lib」。**这一点做得好，应保持。**

---

## 8. 优先级改进清单

> 排序原则沿用项目自身约定：正确性/一致性 → SSOT → 统一 Runtime → 方言 → 性能。每项都必须配「语义矩阵 + 参考向量 + 跨语言向量」，否则不进 production catalog。

### P0

1. **让多语言 SSOT 真实闭环（建议最先做，低风险、可立即验证）**
   - 为 7 种语言补齐 `docs/ffi_registry.json` 的 `bodies`，或改用「从 Rust catalog 单向生成」替代「存储 body 再比对」。
   - 同时修 `scripts/sync_bindings.py:573`：**未覆盖语言必须显式报告为 `unchecked` 并以非零码退出**（或新增 `--strict`），禁止再用 `drift=none` 掩盖空检查。
   - 清理 `scripts/gen_binding.py` 死路径（修复或删除并说明）。
   - 验收：`--check --all` 对 8 种语言都能给出**真实**结论；人为改坏任一语言的一个函数体会被检出。

2. **打通「编译计划 → 生产执行」（价值最大，需独立排期）**
   - 把 `FormulaHotPlan` / `UnifiedExecutor` 接入 `FormulaEngine` 默认路径，AST 树遍历降级为对照实现。
   - 必须先建 differential test：同一公式在「树遍历」与「plan 执行」下逐元素一致（含 warm-up/NaN）。
   - 收益兑现顺序：DAG CSE → buffer liveness 复用 → 并行调度 → 算子融合。

3. **方言一等化：大智慧 + 文华财经**
   - 新增 `FormulaTerminal::DaZhiHui` / `Wenhua` 与对应 `FormulaDialect`，**停止把 DZH 静默路由到 TDX**（这是当前的真实语义错误）。
   - 每个新方言配语义矩阵 + 参考向量 + corpus，按 `CompatibilityLevel::CommonSubset` 诚实标注，不宣称完整兼容。

### P1

4. **Pine `var` / `varip` 跨 bar 状态语义**：当前 `pine/ast_mapper.rs:116` 把 `var` 映射成普通赋值、`is_varip` 被忽略——这是**静默语义错误**（Pine 用户会得到错误结果而非报错），优先级应高于新增函数。
5. **终端解析分方言**：让 TDX/THS/EM 的差异落到 parse 阶段（而非仅 `SemanticProfile` 元数据），使方言特有语法缺口可被拦截。
6. **流式能力下沉到 C/C++/Java/.NET**：这四门语言目前 0 流式，与「统一 Runtime」承诺不符。
7. **性能门禁**：继续收敛 top-20 慢项（历史为 `MIDPRICE14`/`VAR20`/`WILLR14`）。

### P2

8. **`crates/*` V2 轨道**：明确「接入生产」或「标记 deprecated 并删除」，消除双轨。
9. **文档收敛**：合并为 1 份权威基线 + 1 份执行状态，其余归档。
10. **pandas-ta 风格组合 API**：目标清单里有、当前完全缺失；建议在 P0/P1 稳定后再评估，避免过早铺开。

---

## 9. 建议的下一步（最小可执行）

先做 **P0-1（多语言 SSOT 闭环）**：改动集中在 `scripts/` 与 `docs/ffi_registry.json`，不触碰数值内核，可用现有脚本直接验证，且直接修复一个会误导发布判断的漏洞。

再排期 **P0-2（打通编译计划到生产执行）**：这是把「统一 AST → IR → DAG → Runtime」从文档变成事实的唯一路径，也是本次评估中最本质的一项。

---

## 10. 本文档的验证原则

本文件所有结论均标注了源码位置或本机实测输出。凡未在本机复跑的项目（性能基准、多语言宿主运行、CI 状态）均标注为「未复测/未验证」，不据此下结论。手工修改文档不构成验证。

---

## 11. 本轮已执行的修复：P0-1 多语言 SSOT 空检查

### 改动

| 文件 | 改动 |
|---|---|
| `scripts/sync_bindings.py` | `do_check` 增加**覆盖率**维度：先统计每种语言在 registry 中实际存储了多少 body（`covered=M/N`）；`covered=0` 的语言标记为 `status=UNCHECKED`，**不再打印 `drift=none`**，并默认以退出码 1 失败。新增 `--allow-unchecked` 用于迁移期显式承认缺口（仍会打印 `unchecked languages` 行）。`skipped=K` 暴露"有意不作为独立函数暴露"的指标数，避免把有意的缺省与未覆盖混为一谈。 |
| `scripts/gen_binding.py` | `indicators_with_ffi()` 在结果为空时 `SystemExit`，而不是返回空列表——此前会让 `--check` 空过、让 `--generate` 写出**空绑定文件**。文档字符串标注 deprecated 并指向 `sync_bindings.py`。 |

### 验证记录（本机实际运行）

| 用例 | 命令 | 结果 |
|---|---|---|
| 默认全量 | `sync_bindings.py --check --all` | 7 种语言 `UNCHECKED` + 明确 FAIL，**exit=1** |
| 单语言（CI 现状） | `sync_bindings.py --check --lang python` | `covered=71/78 skipped=7 drift=none`，**exit=0**（不破坏现有 workflow） |
| 迁移逃生口 | `sync_bindings.py --check --all --allow-unchecked` | 仍打印 `unchecked languages` 行，**exit=0** |
| 死生成器 | `gen_binding.py --lang python --check` | 明确报错并指向 `sync_bindings.py`，**exit=1** |
| **负向测试** | 经 `target/python_registry_ssot.json` overlay 注入一处 body 漂移 | 检出 `drift=['changed:ta_sma']`，**exit=1**；移除 overlay 后恢复 `drift=none` |
| 语法/调用点 | `py_compile` 两个脚本；检查外部调用方 | 通过；无外部调用 `do_check`；`prepare_python_registry_ssot.py` 与 `apply_talib_performance_plan.py` 的代码改写断言字符串均仍存在，不会被本次改动踩坏 |

### 仍未解决（需要决策，不在本轮范围）

1. **7 种语言仍然没有 body**。本轮只让"检查不再撒谎"，没有真正补齐覆盖。补齐需要先决定策略：
   - 方案 A：对每种语言跑 `--discover` 把 body 落库（沿用现有 verbatim 机制，改动最小）；
   - 方案 B：改为"从 Rust catalog 单向生成"，不再存储 body（更彻底，但要先补 `c_params` / `core_call` / `copies` / `out_kind` / `core_arg_kinds` 等元数据，即 `gen_binding.py` 当初依赖的那批字段）。
2. **CI 仍未接入该门禁**。目前只有 `apply-talib-performance-plan.yml`（且只跑 `--lang python --check`，触发分支为 `fix/talib-performance-plan-20260904`）。在主 CI 加入 `--check --all` 会立刻变红——这正是真实状态，但需要团队先接受或先补齐覆盖。
3. `--allow-unchecked` 只是一个显式逃生口，不是 allow-failure 的替代品；建议在补齐覆盖后从 CI 调用中移除。

---

## 12. 后续执行记录的位置

本文档是**一次性评估**，不再追加执行记录，避免与 `docs/improvement-plan-2026-09-20.md` 重复。

- P0-1（多语言 SSOT 空检查）执行记录 → 见本文 §11。
- **P0-2（统一编译执行链接入生产）执行记录 → 见 `docs/improvement-plan-2026-09-20.md` §7。**

§7 记录了 differential harness 一上线就抓出的 3 个真实缺陷（非交换算子操作数被排序交换、plumbing 节点污染外部输入 ABI、无 DCE），
以及一个量化结论：`functions.rs` 实现 327 个函数而 `registry.rs` 只声明 104 个，
**223 个已实现函数对注册表不可见**并因此落入保守兜底路径。该结论强化了本文 §4「多语言 SSOT 断裂」的判断。
