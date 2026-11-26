# Alpha 因子库完整实现计划

> 盘点基线：`main` 分支，远端快进后提交 `0eb584d1`（2026-11-26）。
> 范围：Qlib Alpha158、Qlib Alpha360、Kakushadze《101 Formulaic Alphas》。
> 下文原计划记录目标范围；第 0 节持续记录实际实施、验证和授权状态。

## 0. 本轮执行进度（2026-11-26）

- **Alpha158**：现有 158 个公式库保持不变；新增 Rust `FeatureMatrix` 有序矩阵入口、Python `feature_set("alpha158")` 和 CLI `features alpha158`，沿用 Qlib 表达式顺序。
- **Alpha360**：新增 360 列实现与相同三种调用面；按 Qlib 字段组顺序、滞后、归一化及 NaN 预热定义实现。Rust 单元测试、Python 绑定编译、隔离环境 Python 调用测试和 CLI 集成测试通过；发布门仍在执行。
- **WorldQuant 101**：没有新增或恢复公式内容。用户确认尚无/不确定完整公式再分发授权，并要求先不公开公式内容；因此完整论文公式清单、公式源码和含完整公式的公开版本继续等待授权。当前已有实现不代表论文 101 项完整可算。
- **版本计划**：下一发布目标为 `0.3.2`，本轮可交付范围是 Alpha158 有序矩阵与 Alpha360；不能将其描述为已经完成完整 WorldQuant101 库。
- **本地发布验证**：Rust 工作区编译、相关 Rust 测试、feature 子集、crate 打包和 Rust 文档构建通过；Windows ABI3 Python 轮子安装后，绑定契约检查及 151 项 Python 测试通过。另有 5 项既有 TA-Lib 扩展兼容用例需要本机 TA-Lib 提供 DPO/AC 等扩展函数，当前环境缺少这些函数；远端发布工作流仍需在候选提交上完成。

## 1. 目标与结论

让用户可以通过稳定、可发现的接口直接计算上述三套公开因子/特征定义；每个结果可追溯到明确的论文或参考实现，并有输入、数值语义、缺失值、输出顺序和适用数据范围的说明。

三者的计算对象并不相同：Alpha158 是 158 列工程特征，Alpha360 是过去 60 个交易日的 360 维原始价量输入，WorldQuant 101 才是公式化 alpha 信号。实现时统一目录和调用体验，但保留与各自输出形状相匹配的 API。

## 2. 基线代码盘点（`0eb584d1`）

| 库 | 当前状态 | 当前调用与缺口 |
|---|---|---|
| Alpha158 | **已实现 158/158**。代码含 Qlib 默认表达式，`core/tests/alpha158_parity.rs` 对照 Qlib 生成的数值金标。 | Rust `factors::builtin::factor_library("alpha158")` 和 Python `finkit.factor_library("alpha158")` 可直接调用；Python 支持单因子和整库计算。尚需将有序矩阵输出、统一目录元数据和文档契约补齐。 |
| Alpha360 | **未实现为内置库**。当前内置目录只注册 `alpha158`、`worldquant101`。 | 通用特征工程组件已存在，但未找到 Qlib Alpha360 的 360 列定义、专用入口或相应金标。Qlib 定义为 CLOSE、OPEN、HIGH、LOW、VWAP、VOLUME 六组、每组最近 60 日，按当前 close / volume 归一化。 |
| WorldQuant 101 | **部分实现，且现有来源清单与论文不一致**。当前接口公开 17 条可单序列计算表达式；本地金标含 71 条公式、54 条标记为横截面算子阻塞，并把 30 个编号标作“保留”。 | Rust/Python 可通过 `factor_library("worldquant101")` 直接取得这 17 条。论文附录实际列出 Alpha#1 至 Alpha#101（含当前被标成保留的 Alpha#10、Alpha#25 等），因此“71 条公式 + 30 个保留编号”不能作为完整论文口径。当前 WQ 库的单序列表达式也不能替代需要横截面排名或分组中性的定义。 |

当前核心已有 `CrossSectionView`、按日期运行的横截面因子原语，以及公式/因子的横截面 JSON 合同；这些接口每次处理一行横截面，尚不等于在同一个表达式计划中同时正确处理“沿时间轴的滚动计算 + 沿资产轴的横截面计算”。WorldQuant 需要后者。

**关键实施门槛：**原论文声明 Appendix A 的公式和代码经 WorldQuant 许可提供，并称 WorldQuant 保留相关权利及版权。发布完整公式库之前，先核验本项目目标用途和再分发权限；同时检查 Qlib 定义的来源与许可要求。Alpha360 / Alpha158 和不含受限表达式内容的通用计算内核可先行开发，WQ 公式清单的对外发布须通过该门槛。

## 3. 交付范围

### 纳入首期

1. Alpha158：保留现有 158 个表达式和 Qlib 数值语义，补稳定的批量矩阵输出、元数据与统一文档。
2. Alpha360：新增 360 列特征集，精确复现 Qlib 字段、顺序、60 日回看、归一化和预热缺失行为。
3. WorldQuant 101：以原论文附录重新建立 1–101 编号清单，恢复当前漏掉的论文公式；在授权允许后实现可由输入数据完整支持的公式，并对缺少字段或分类数据的项返回明确状态。
4. 首批直接调用表面：Rust 核心、Python 包、CLI；为需要语言中立传输的调用增加版本化 JSON 合同，便于后续 Node、C、Go、Java、.NET 等绑定复用。
5. 提供按名称查询、枚举、描述、依赖检查、单项计算和批量计算。目录需区分“因子库”与“特征集”。

### 不纳入首期

- 行情下载、数据清洗服务、证券池维护、回测、交易和组合构建。
- 自动推出未被上述三套来源定义的“相似因子”。新增因子家族（如其他券商研报或平台库）须单独确认来源、口径和许可后再纳入。
- 把 Alpha360 的 360 维特征误称为 360 个独立的公式化 alpha。

## 4. 用户接口设计

延续现有可调用入口，并区分一维因子输出与二维特征输出：

```python
import finkit as ta

alpha158 = ta.factor_library("alpha158")
alpha_values = alpha158.evaluate_all(
    open=open_, high=high, low=low, close=close,
    volume=volume, vwap=vwap,
)

alpha360 = ta.feature_set("alpha360")
features = alpha360.evaluate(
    open=open_, high=high, low=low, close=close,
    volume=volume, vwap=vwap,
)  # shape: (time, 360)

wq101 = ta.factor_library("worldquant101")
panel_values = wq101.evaluate_panel(
    panel, vwap=vwap, dollar_volume=dollar_volume, cap=cap,
    sector=sector, industry=industry, subindustry=subindustry,
)  # shape: (time, symbols, factors)
```

接口要求：

- 保持 `factor_library("alpha158")` 和现有单序列 WQ 调用兼容；给 Alpha360 增加明确的 `feature_set("alpha360")` 入口。提供 `available_factor_libraries()`、`available_feature_sets()` 和统一目录查询。
- Alpha360 / Alpha158 返回有稳定列顺序和列名的 `FeatureMatrix` 或等价矩阵对象。保留源定义的列顺序，不依赖哈希映射顺序。
- WQ 同时保留单序列 `evaluate`（只计算无需横截面上下文的公式）和新增面板 `evaluate_panel`。面板包含有序时间戳、证券代码及每个输入字段的 `time × symbols` 数值矩阵；不同字段必须严格对齐。
- WQ 的因子元数据至少包括来源编号、表达式、输入字段、所需算子、所需分类字段、最小预热长度、可用性状态及缺失原因。缺数据不应被静默跳过或用近似算子代替。
- 批量调用默认报告可计算项与不可计算项；单因子调用在必需输入缺失时返回指明字段/日期轴/分类字段的错误。
- FFI JSON 合同版本化，输出明确声明轴顺序、名称顺序和 NaN/null 约定；Python 的 NumPy 矩阵应遵循相同契约。
- CLI 提供库目录命令和 CSV 批量计算入口；面板数据至少支持 `timestamp,symbol,open,high,low,close,volume,vwap`，可选 dollar volume、cap 及多套分类列。

## 5. 数值和输入语义

### Alpha360

- 固定分组顺序为 CLOSE、OPEN、HIGH、LOW、VWAP、VOLUME，每组包含滞后 59 至 1 日及当日，共 60 列，合计 360 列。
- 价格组用各日价格除以当日 close；成交量组按 Qlib 定义除以当日 volume 加 `1e-12`。保留零分母处理，不擅自换成填充或裁剪。
- 历史不足 60 个观测时，明确规定并对齐 Qlib 的 Ref / 缺失值语义；缺失日不得跨停牌日或无数据日意外前移填补。
- 仅输出特征。Qlib handler 的未来收益标签属于监督目标，不并入 Alpha360 特征矩阵。

### Alpha158

- 冻结现有 Qlib 默认 158 条表达式、字段顺序、窗口 `[5, 10, 20, 30, 60]` 和 `+1e-12` 等细节。
- 继续以现有 Qlib 金标验证表达式数值；批量矩阵列顺序固定为 Qlib 定义顺序。若 Python 现有 `names()` 返回字典排序，新增矩阵接口不得沿用该排序来推断特征列序。

### WorldQuant 101

- 以论文 Appendix A 的 101 个公式为公式编号和语义基线；Qlib 转录、其他项目实现用于交叉核对，不用于覆盖原论文中已定义的公式。
- 按论文定义区分 `rank`（同一日期、跨证券排名）与 `ts_rank`（同一证券、跨历史时间排名）；保留 ties、NaN、无效输入、预热区间和除零处理的显式策略。禁止把 `rank` 替换为滚动 `RANK_PCT`。
- 支持二维 `time × symbols` 执行：时间序列滚动、滞后、相关、协方差、乘积、线性衰减、argmax/argmin 与横截面 rank / scale / 分组中性化可以在表达式中组合。
- 按论文把非整数回看长度向下取整；不要四舍五入。确认现有算子在最小长度、并列极值及预热行为上是否一致。
- 输入字段除 OHLCV / returns / VWAP 外，还需 `cap`、`adv{d}` 和分类字段。论文将 `adv{d}` 定义为过去 d 日平均美元成交额；若从 close × volume 推导，必须记录价格复权和成交量口径，并允许用户直接提供 dollar volume。`indneutralize` 可能需要 sector、industry、subindustry 多种分组；分组缺失时给出逐因子状态。
- 不把论文中的历史收益、生产表现或回测结论当作实现正确性的证据；本项目交付是公式计算，不承诺收益或策略表现。

## 6. 分阶段开发计划

估时为单人投入的工程日，仅用于排序和容量规划；不包括授权等待、外部数据接入、跨平台发布等待。

| 阶段 | 内容 | 预计投入 | 验收门槛 / 依赖 |
|---|---|---:|---|
| P0 来源、范围与许可门 | 对照论文与当前 golden 逐号建立差异表；确认 WQ 公式清单可否按目标用途发布；核对 Qlib Alpha158 / Alpha360 源文件的版本与许可、来源记录。 | 1–3 天 + 外部审核时间 | 得到编号 1–101 完整映射；公式分“已定义 / 缺输入 / 语义待定”，没有将已发表公式称为保留编号；明确 WQ 再分发路径。WQ 内容发布等待书面/合规结论。 |
| P1 库目录与数据契约 | 定义 `factor_library`、`feature_set` 目录项、元数据结构、稳定输出顺序、面板和特征矩阵输入输出契约。复用现有 `FeatureMatrix`、`CrossSectionView`、FactorGraph / Formula 编译计划，先做能力差距分析。 | 2–4 天 | Rust 和 Python 能枚举库、查看描述与依赖；时间戳/证券轴错位、重复证券及输入长度不一致均报错；JSON 合同明确版本与 shape。 |
| P2 Alpha360 特征集 | 新增 360 条 Qlib 特征定义及批量执行；优先复用 Formula/FeatureEngine 的预编译与共享执行计划，避免 360 次重复解析。 | 2–4 天 | 列数、列名和列序为 360 项；若干边界行情上与 Qlib `Alpha360DL.get_feature_config()` 输出一致；预热、零成交量、零/缺失 close 行为锁定。 |
| P3 面板因子执行内核 | 将输入作为 `time × symbols` 对齐矩阵；实现/接通二维时间算子与按日期横截面算子，支持同一 AST 内跨轴组合；补 dollar volume、cap 与多分组绑定。 | 5–10 天 | 时间算子沿时间轴执行、rank/scale/neutralize 沿资产轴执行；合成小面板可手算核对；缺失证券和分组不改变其他证券的轴位置；没有单序列 API 的静默语义变更。 |
| P4 WQ101 完整公式清单和分类 | 以原论文重新生成 101 项清单；修正当前 71/30 划分；为每项记录表达式、依赖字段、操作符、轴语义和来源。授权通过后实现可由本内核支持的公式，逐项登记真实缺失原因。 | 4–8 天（不含 P3 中重复算子开发） | 101 个编号均存在且唯一；不存在未解释的“保留”项；输入完整时每个公式进入执行路径；缺 `cap` 或分组列时只对依赖它的因子报状态。 |
| P5 用户调用面 | 扩展 Rust/Python API；新增 CLI 目录、单因子、批量和 CSV 面板入口；按既有 FFI 模式发布版本化跨语言 JSON 合同及 Python 示例。 | 3–6 天 | 用户不需自建 AST、写公式或复制因子表达式即可调库；CLI / Python 对同一 fixture 的名称顺序、shape 与数值语义一致；现有 Python 调用保持兼容。 |
| P6 金标、完整性和性能门 | Alpha158 延用 Qlib numeric golden；Alpha360 增加 Qlib 特征金标；WQ 对公式编号/表达式做来源核对，算子单测与小面板手算，增加独立实现的对照向量；做批量性能和内存基线。 | 3–6 天 | 所有差异有逐项记录；完整率按“公式定义”与“给定输入可计算率”分别报告；基准不引入未来数据；回归验证所有现有用户入口。 |
| P7 发布和文档 | 更新 Rust/Python/CLI/API 文档、示例、生成 API 文档和变更日志；检查 crates/wheels/npm/跨平台构建与许可声明。 | 2–4 天 | 三套库在发布包中可发现；安装后的示例可运行；WQ 再分发许可门已通过，或发布包明确不包含受限公式内容。 |

粗略总量：**20–39 个工程日**，其中面板执行、WQ 来源核验和许可审查是不确定性最大的项目。P2 Alpha360 可和 P0/P1 并行；P3 是 WQ 全面执行的前置条件；P5 和 P6 应等 API 与核心数值语义稳定后收口。

## 7. 预计改动位置

以下是实施时优先检查的现有入口；若 P1 发现边界不合适，应先修订架构方案再落代码：

- 内置注册与编译：`core/src/factors/builtin.rs`、`core/src/factors/builtin/alpha158.rs`、`core/src/factors/builtin/worldquant101.rs`。
- 单序列 / 横截面能力：`core/src/factors.rs`、`core/src/formula/ops/cross_sectional.rs`、`core/src/data_contract.rs`、`core/src/factor_graph.rs`、`core/src/formula/`。
- 特征矩阵：`core/src/features/`，优先复用 `FeatureMatrix` 与已有 FeatureEngine。
- Python 直接调用：`ffi/python-binding/src/factor_library.rs`、`ffi/python-binding/src/lib.rs`、`ffi/python-binding/finkit/__init__.pyi`。
- 语言中立执行合同：`ffi/ffi-common/src/factor.rs`、对应 schema/contract fixtures；再由 Node、C、Go、Java、.NET 等已有薄绑定接入。
- CLI：`cli/src/main.rs`、`docs/cli.md`。
- 数值基线及生成器：`core/tests/alpha158_parity.rs`、`core/tests/worldquant101_library.rs`、`tests/golden/alpha158/`、`tests/golden/worldquant101/`、`scripts/gen_worldquant101.py`。
- API 与使用说明：`docs/api-reference.md`、`docs/python.md`、`docs/api-reference-zh.md`、`docs/features.md`、README。

## 8. 质量验收清单

- [ ] Alpha158：158 个唯一名称，全部表达式与 Qlib 默认版逐项相符；数值金标覆盖正常值、短序列、零成交量、无效值及预热边界。
- [ ] Alpha360：恰好 360 列；字段定义、归一化公式、滞后方向和输出顺序逐项与 Qlib 对齐；标签未混入特征。
- [ ] WQ：原论文编号 1–101 全覆盖；源码不再声称其中 30 个编号没有公式；对具备完整输入的 101 项均可执行，或经证据确认存在非实现性约束并公开说明。
- [ ] 横截面轴：所有 WQ `rank` 在日期内跨证券排名；`ts_*` 在证券内跨日期运算；包含 ties、NaN、空组、单证券和停牌间隙测试。
- [ ] 输入契约：OHLCV、VWAP、returns、dollar volume / `adv{d}`、cap 和每类 `IndClass` 的映射与缺失报告明确。
- [ ] API：Rust、Python、CLI 具有一行获取库/特征集并计算的示例；名字、依赖和 shape 可查询；批量与单项调用不静默丢因子。
- [ ] 跨语言：JSON 合同版本化、轴顺序确定、null 语义固定；绑定不复制公式、不分叉数值实现。
- [ ] 许可：WQ 公式对外发布前完成审查/授权结论；代码注明原始论文、Qlib 参考版本和本项目自行实现范围。
- [ ] 研究边界：功能只计算特征/因子值；不声称已实现论文收益、实盘能力或回测。

## 9. 来源

- Kakushadze, Z. (2015), *101 Formulaic Alphas*, arXiv:1601.00991：论文 Appendix A 列出 101 个编号、算子定义和输入数据；Appendix B 说明公式/代码权利归属。 [论文页面](https://arxiv.org/abs/1601.00991) · [PDF](https://arxiv.org/pdf/1601.00991)
- Microsoft Qlib `qlib/contrib/data/loader.py`：Alpha360DL、Alpha158DL 字段表达式与顺序。 [源文件](https://github.com/microsoft/qlib/blob/main/qlib/contrib/data/loader.py)
- Microsoft Qlib `qlib/contrib/data/handler.py`：Alpha360、Alpha158 handler 如何组合 feature loader 和 label。 [源文件](https://github.com/microsoft/qlib/blob/main/qlib/contrib/data/handler.py)
- 本地现状：本计划“最新代码盘点”中的路径与测试均指向基线提交 `0eb584d1`。
