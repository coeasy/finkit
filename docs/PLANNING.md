# Finkit Industrial-Grade Optimization — Planning Overview

> **Generated:** 2026-08-30 | **Branch:** main | **Status:** in-progress (v0.1.0 阶段 A 已执行)

## Progress

- ✅ Completed: **219**
- 🔄 Pending: **57**
- ❌ Blocked: **0**
- 📊 Total: **276**

## Tech Stack

- **language**: Rust

## Global Constraints

- 所有变更必须向后兼容现有批量 API
- cargo clippy --workspace --all-targets --all-features -- -D warnings 必须通过
- cargo test --workspace 必须通过
- 不得引入新的 unsafe 代码（SIMD/FFI 路径除外）
- 每个 Story 完成后更新相关文档

## Out of Scope

（无明确排除项）

## Implementation Phases

| Phase | Title | Stories | Status | Rollback Plan |
|-------|-------|---------|--------|---------------|
| 1 | P0: 编译健康与质量基线 | 2 stories | completed | git revert phase-1 commits |
| 2 | P1: 核心架构升级 | 3 stories | completed | git revert phase-2 commits; streaming 模块为新增代码可直接删除 |
| 3 | P2: 测试与验证体系 | 3 stories | completed | git revert phase-3 commits; 测试文件为新增可直接删除 |
| 4 | P3: 性能优化 | 2 stories | completed | git revert phase-4 commits |
| 5 | P4: 生态完善 | 1 stories | completed | git revert phase-5 commits |

## Launch Phases

（无明确发布阶段）

## Risks & Mitigations

（无已知风险）

## Stories

| ID | Title | Priority | Phase | Type | Status | Criteria |
|----|-------|----------|-------|------|--------|---------|
| TASK-001 | 修复所有 Clippy 警告并加固 CI 门禁 |  |  | refactor | ✅ Done | 4 AC |
| TASK-002 | 文档一致性修复与占位符替换 |  |  | docs | ✅ Done | 4 AC |
| TASK-003 | 定义 Ohlcv trait 和 StreamingIndicator |  |  | backend | ✅ Done | 4 AC |
| TASK-004 | 工业级错误类型重构 |  |  | refactor | ✅ Done | 4 AC |
| TASK-005 | 核心指标流式实现（SMA/EMA/RSI/MACD/BOLL/ATR） |  |  | backend | ✅ Done | 4 AC |
| TASK-006 | 属性测试框架 + 核心不变量验证 |  |  | test | ✅ Done | 4 AC |
| TASK-007 | 黄金测试体系（vs TA-Lib 参考输出） |  |  | test | ✅ Done | 4 AC |
| TASK-008 | 指标注册表 + JSON 导出 |  |  | backend | ✅ Done | 4 AC |
| TASK-009 | SIMD 完善与基准测试扩展 |  |  | backend | ✅ Done | 4 AC |
| TASK-010 | 公式引擎零拷贝完成与性能验证 |  |  | backend | ✅ Done | 4 AC |
| TASK-011 | 发布基础设施与文档完善 |  |  | docs | ✅ Done | 4 AC |
| TASK-013 | TA-Lib 对比基准测试套件 | P0 |  | test | ✅ Done | 4 AC |
| TASK-014 | 批量指标 SIMD 向量化深度优化 | P0 |  | backend | ✅ Done | 6 AC |
| TASK-015 | 公式引擎执行开销优化 | P0 |  | backend | ✅ Done | 6 AC |
| TASK-016 | 流式指标微优化 | P0 |  | backend | ✅ Done | 7 AC |
| TASK-017 | 全面性能基准测试报告与结果保存 | P0 |  | docs | ✅ Done | 5 AC |
| TASK-018 | TA-Lib Overlap 缺失函数补全（MA/MAVP/SAREX | P0 |  | backend | ✅ Done | 6 AC |
| TASK-019 | TA-Lib Momentum 缺失函数补全（11个） | P0 |  | backend | ✅ Done | 5 AC |
| TASK-020 | TA-Lib 缺失蜡烛图形态补全（10个 CDL） | P1 |  | backend | ✅ Done | 4 AC |
| TASK-021 | Statistics VAR 函数 + 公式引擎注册 | P1 |  | backend | ✅ Done | 5 AC |
| TASK-022 | 国内核心指标实现（KDJ/BIAS/PSY/VR/CR/DPO） | P1 |  | backend | ✅ Done | 9 AC |
| TASK-023 | 国内扩展指标实现（AR/BR/DMA/ENE/EXPMA） | P1 |  | backend | ✅ Done | 7 AC |
| TASK-024 | 国际知名趋势指标（HMA/ALMA/McGinley/ZLEMA/VI | P1 |  | backend | ✅ Done | 4 AC |
| TASK-025 | 国际知名动量/振荡指标（AO/Fisher/TSI/Coppock/K | P1 |  | backend | ✅ Done | 5 AC |
| TASK-026 | 国际知名成交量/资金流指标（CMF/Force/EOM/KVO/NVI | P1 |  | backend | ✅ Done | 6 AC |
| TASK-027 | 波动率扩展指标（Mass Index/Ulcer Index/RVI） | P1 |  | backend | ✅ Done | 5 AC |
| TASK-028 | 新增指标流式版本（KDJ/BIAS/HMA/CMF/Fisher 等） | P1 |  | backend | ✅ Done | 5 AC |
| TASK-029 | Heikin-Ashi 和 ZigZag 工具函数 | P1 |  | backend | ✅ Done | 5 AC |
| TASK-030 | 新增指标性能基准测试与优化 | P1 |  | test | ✅ Done | 5 AC |
| TASK-031 | 公式引擎新增指标注册与FFI绑定更新 | P1 |  | backend | ✅ Done | 6 AC |
| TASK-032 | NaN→0 系统性偏差修复（工业级质量） | P1 |  | backend | ✅ Done | 6 AC |
| TASK-033 | ZigZag 除零修复与 Edge Case 加固 | P1 |  | backend | ✅ Done | 3 AC |
| TASK-034 | O(n×period)→O(n) 滑动窗口优化 | P1 |  | backend | ✅ Done | 3 AC |
| TASK-035 | 堆分配减少与内存优化 | P1 |  | backend | ✅ Done | 2 AC |
| TASK-036 | 完整性能基准测试运行与结果保存 | P1 |  | test | ✅ Done | 2 AC |
| TASK-037 | 一键构建脚本（build.ps1 + build.sh） | P1 |  | backend | ✅ Done | 3 AC |
| TASK-038 | FFI 构建问题修复 | P1 |  | backend | ✅ Done | 3 AC |
| TASK-039 | README 与文档全面更新 | P1 |  | docs | ✅ Done | 3 AC |
| TASK-040 | rolling_max/rolling_min O(n) 单调队列优化 | P1 |  | backend | ✅ Done | 4 AC |
| TASK-041 | WMA O(n) 递推算法优化 | P1 |  | backend | ✅ Done | 4 AC |
| TASK-042 | DI/ADX 链路 O(n) Wilder 平滑修复 | P1 |  | backend | ✅ Done | 4 AC |
| TASK-043 | CCI/CMO/MFI/ULTOSC 滑动窗口优化 | P1 |  | backend | ✅ Done | 5 AC |
| TASK-044 | AROON O(n) 单调队列优化 | P1 |  | backend | ✅ Done | 3 AC |
| TASK-045 | 公式引擎 collect 消除与 SIMD 路径优化 | P1 |  | backend | ✅ Done | 3 AC |
| TASK-046 | MACD/DEMA/TEMA 中间 Vec 消除与 ema_in_pl | P1 |  | backend | ✅ Done | 4 AC |
| TASK-047 | BBANDS 单遍 sum+sum_sq 优化 | P1 |  | backend | ✅ Done | 3 AC |
| TASK-048 | KAMA volatility 滑动和优化 | P1 |  | backend | ✅ Done | 2 AC |
| TASK-049 | 流式指标 VecDeque→环形缓冲统一改造 | P1 |  | backend | ✅ Done | 3 AC |
| TASK-050 | 热路径 #[inline] 标注与微优化 | P1 |  | backend | ✅ Done | 2 AC |
| TASK-051 | 最终性能验证与基准测试报告更新 | P1 |  | test | ✅ Done | 3 AC |
| TASK-052 | 流式 API 重构：Option<Output> 语义 + value | P1 |  | backend | ✅ Done | 5 AC |
| TASK-053 | Ohlcv trait 扩展 + Forming Bar Repain | P1 |  | backend | ✅ Done | 5 AC |
| TASK-054 | 流式指标 serde 状态序列化 Checkpoint/Restore | P1 |  | backend | ✅ Done | 5 AC |
| TASK-055 | 批量指标零分配输出 API（_into 变体） | P1 |  | backend | ✅ Done | 5 AC |
| TASK-056 | AVX-512 SIMD 运行时分发 | P1 |  | backend | ✅ Done | 5 AC |
| TASK-057 | 批量参数扫描 API（Parameter Sweep） | P1 |  | backend | ✅ Done | 5 AC |
| TASK-058 | 流式指标全量覆盖（国内指标组 8个） | P1 |  | backend | ✅ Done | 5 AC |
| TASK-059 | 流式指标全量覆盖（动量组 4个） | P1 |  | backend | ✅ Done | 5 AC |
| TASK-060 | 流式指标全量覆盖（成交量组 6个） | P2 |  | backend | ✅ Done | 5 AC |
| TASK-061 | 流式指标全量覆盖（波动率+MA组 7个） | P2 |  | backend | ✅ Done | 5 AC |
| TASK-062 | Python 绑定 GIL 释放 + 零拷贝 NumPy 优化 | P2 |  | backend | ✅ Done | 4 AC |
| TASK-063 | docs.rs 完整文档属性与模块文档 | P2 |  | docs | ✅ Done | 5 AC |
| TASK-064 | 数据转换管道 Transform Pipeline | P2 |  | backend | ✅ Done | 5 AC |
| TASK-065 | 全面测试覆盖率提升至 1500+ | P2 |  | test | ✅ Done | 5 AC |
| TASK-066 | 性能基准测试更新与竞品对比报告 | P2 |  | test | ✅ Done | 5 AC |
| TASK-067 | 流式指标极致性能优化（追平 quantedge-ta） | P2 |  | backend | ✅ Done | 5 AC |
| TASK-068 | Builder 模式统一与 IndicatorBuilder Trai | P2 |  | backend | ✅ Done | 6 AC |
| TASK-069 | 泛型精度支持 Float Trait（f32/f64） | P2 |  | backend | ✅ Done | 5 AC |
| TASK-070 | 类型安全输出结构体重构 | P2 |  | backend | ✅ Done | 5 AC |
| TASK-071 | PriceSource 枚举与统一价格源选择 | P2 |  | backend | ✅ Done | 5 AC |
| TASK-072 | 方向性指标组流式化（DX/PLUS_DI/MINUS_DI/ADXR/ | P2 |  | backend | ✅ Done | 6 AC |
| TASK-073 | 高级动量指标流式化（CMO/PPO/STOCHF/STOCHRSI/U | P2 |  | backend | ✅ Done | 6 AC |
| TASK-074 | 成交量高级指标流式化（AD/ADOSC/AnchoredVWAP/VW | P2 |  | backend | ✅ Done | 6 AC |
| TASK-075 | 统计指标流式化（STDDEV/BETA/CORREL/TSF/LinR | P2 |  | backend | ✅ Done | 6 AC |
| TASK-076 | Hilbert 变换流式化（HT_DCPERIOD/HT_SINE/H | P2 |  | backend | ✅ Done | 5 AC |
| TASK-077 | 统一 Checkpoint/Restore Trait 与全覆盖 | P2 |  | backend | ✅ Done | 5 AC |
| TASK-078 | 通用参数扫描框架 SweepEngine | P2 |  | backend | ✅ Done | 6 AC |
| TASK-079 | Transform Pipeline FFI 暴露与扩展 | P2 |  | backend | ✅ Done | 6 AC |
| TASK-080 | Polars/Arrow 零拷贝集成（AlphaTA-polars fe | P2 |  | backend | ✅ Done | 5 AC |
| TASK-081 | Forming Bar Repaint 原生支持完善 | P2 |  | backend | ✅ Done | 5 AC |
| TASK-082 | SIMD 覆盖扩展到全部批量指标 | P2 |  | backend | ✅ Done | 5 AC |
| TASK-083 | docs.rs 发布配置与完整 API 文档 | P2 |  | docs | ✅ Done | 5 AC |
| TASK-084 | 全面竞品基准测试与自动化报告 | P2 |  | test | ✅ Done | 6 AC |
| TASK-085 | 流式指标全覆盖验证与 Registry 100% 对齐 | P2 |  | test | ✅ Done | 5 AC |
| TASK-086 | README 与竞品对比文档全面更新 | P2 |  | docs | ✅ Done | 5 AC |
| TASK-087 | 深度性能优化 Round 5 — 等价指标全面超越 TA-Lib C | P2 |  | refactor | ✅ Done | 5 AC |
| TASK-088 | 深度性能优化 Round 5 — 非等价指标性能收敛至 2x 以内 | P2 |  | refactor | ✅ Done | 5 AC |
| TASK-089 | 一键构建多语言安装包脚本（Python whl + Java jar） | P2 |  | infra | ✅ Done | 6 AC |
| TASK-090 | 性能优化最终验证与文档更新 | P2 |  | docs | ✅ Done | 5 AC |
| TASK-091 | TA-Lib C FFI 全量指标真实对比基准测试 | P2 |  | test | ✅ Done | 4 AC |
| TASK-093 | Rust 竞品性能对比框架（ta-rs/yata 对比） | P2 |  | test | ✅ Done | 4 AC |
| TASK-094 | 指标特征工程核心框架 (FeatureEngine) | P2 |  | backend | ✅ Done | 5 AC |
| TASK-095 | 多周期指标特征批量生成 | P2 |  | backend | ✅ Done | 5 AC |
| TASK-096 | 指标信号检测（交叉/背离/穿越） | P2 |  | backend | ✅ Done | 6 AC |
| TASK-097 | 滚动高阶统计特征（skewness/kurtosis/entropy） | P2 |  | backend | ✅ Done | 6 AC |
| TASK-098 | 指标组合特征（ratio/spread/correlation mat | P2 |  | backend | ✅ Done | 5 AC |
| TASK-099 | 时间序列特征（lag/lead/diff/rolling_apply） | P2 |  | backend | ✅ Done | 5 AC |
| TASK-100 | ML 标签生成工具（forward return/triple bar | P2 |  | backend | ✅ Done | 7 AC |
| TASK-101 | 特征标准化与归一化（auto normalization） | P2 |  | backend | ✅ Done | 5 AC |
| TASK-102 | 特征重要性与选择辅助 | P2 |  | backend | ✅ Done | 5 AC |
| TASK-103 | 特征矩阵输出与导出（CSV/Arrow/DataFrame） | P2 |  | backend | ✅ Done | 5 AC |
| TASK-104 | 特征工程 SIMD 与性能优化 | P2 |  | backend | ✅ Done | 5 AC |
| TASK-105 | 特征工程完整文档与使用示例 | P2 |  | docs | ✅ Done | 5 AC |
| TASK-106 | 特征工程 FFI 绑定与 Python 接口 | P2 |  | backend | ✅ Done | 5 AC |
| TASK-092a | 性能瓶颈指标分析与逐项优化 |  |  | backend | ✅ Done | 3 AC |
| TASK-092b | 性能回归 CI Gate 建立 |  |  | infra | ✅ Done | 4 AC |
| TASK-107 | 运行 TA-Lib C 对比基准并识别性能瓶颈指标 | P2 |  | test | ✅ Done | 5 AC |
| TASK-109 | 高级移动平均指标扩展（HMA/ALMA/VIDYA/MAMA/FRAM | P2 |  | feature | ✅ Done | 5 AC |
| TASK-110 | 高级动量指标扩展（Connors RSI/StochRSI/RVI） | P2 |  | feature | ✅ Done | 5 AC |
| TASK-111 | 高阶波动率指标扩展（GK/Parkinson/RS/YZ/实现波动率） | P2 |  | feature | ✅ Done | 5 AC |
| TASK-112 | 量价关系指标扩展（EMV/ForceIndex/KVO/NVI/PVI | P2 |  | feature | ✅ Done | 5 AC |
| TASK-114 | ML特征交叉与偏离特征自动生成 | P2 |  | feature | ✅ Done | 5 AC |
| TASK-115 | 波动率状态分类与市场regime检测 | P2 |  | feature | ✅ Done | 6 AC |
| TASK-116 | 市场微观结构特征（tick/volume imbalance, kyl | P2 |  | feature | ✅ Done | 6 AC |
| TASK-117 | 性能基准自动报告生成器 | P2 |  | infra | ✅ Done | 5 AC |
| TASK-118 | 新增指标全量基准测试覆盖 | P2 |  | test | ✅ Done | 5 AC |
| TASK-119 | 新增特征与指标的FFI绑定更新 | P2 |  | feature | ✅ Done | 4 AC |
| TASK-120 | 全量性能验证与竞品对标最终报告 | P2 |  | docs | ✅ Done | 5 AC |
| TASK-108a | 均线类指标深度优化（SMA/EMA/WMA/DEMA/TEMA/KAM |  |  | refactor | ✅ Done | 4 AC |
| TASK-108b | 动量/方向/统计类指标深度优化（RSI/MACD/ADX/CCI/ST |  |  | refactor | ✅ Done | 4 AC |
| TASK-113a | 统计特征扩展Part1（Hurst/ACF/PACF/半方差） |  |  | feature | ✅ Done | 4 AC |
| TASK-113b | 统计特征扩展Part2（ADF检验/协整检验） |  |  | feature | ✅ Done | 4 AC |
| TASK-121 | STOCH/STOCHF 单遍融合管线优化 | P2 |  | refactor | ✅ Done | 5 AC |
| TASK-122 | WILLR 简单窗口最大/最小优化 | P2 |  | refactor | ✅ Done | 4 AC |
| TASK-123 | AROON 索引追踪重构优化 | P2 |  | refactor | ✅ Done | 4 AC |
| TASK-124 | ADX Wilder 路径 SIMD 融合优化 | P2 |  | refactor | ✅ Done | 5 AC |
| TASK-125 | OBV 分支消除与 SIMD 批量累加 | P2 |  | refactor | ✅ Done | 4 AC |
| TASK-126 | TRIMA 三角核卷积单遍优化 | P2 |  | refactor | ✅ Done | 4 AC |
| TASK-127 | MFI 滑动窗口融合优化 | P2 |  | refactor | ✅ Done | 4 AC |
| TASK-128 | Ehlers 滤波器系列实现 | P2 |  | feature | ✅ Done | 5 AC |
| TASK-129 | 高级波动率估计器（GK/Parkinson/YZ/RS） | P2 |  | feature | ✅ Done | 3 AC |
| TASK-130 | 市场结构指标（Elder Ray/Chande Kroll Stop/ | P2 |  | feature | ✅ Done | 4 AC |
| TASK-131 | 特征重要性自动评估（信息增益/互信息） | P2 |  | feature | ✅ Done | 5 AC |
| TASK-132 | 特征时序稳定性检测（PSI/CSI） | P2 |  | feature | ✅ Done | 5 AC |
| TASK-133 | 自动特征交叉与多项式扩展 | P2 |  | feature | ✅ Done | 5 AC |
| TASK-134 | 目标编码（Target Encoding）支持 | P2 |  | feature | ✅ Done | 4 AC |
| TASK-135 | 时序交叉验证分割器（Purged K-Fold/Embargo） | P2 |  | feature | ✅ Done | 5 AC |
| TASK-136 | 特征存储与版本化 | P2 |  | feature | ✅ Done | 5 AC |
| TASK-137 | 流式 STOCH/STOCHF/AROON 性能对齐 | P2 |  | refactor | ✅ Done | 5 AC |
| TASK-138 | 核心批量指标 SIMD 内核（SMA/EMA/RSI/MACD） | P2 |  | refactor | ✅ Done | 5 AC |
| TASK-139 | 统计特征 SIMD 加速（rolling_mean/rolling_s | P2 |  | refactor | ✅ Done | 5 AC |
| TASK-140 | 全量性能回归验证与竞品对标更新 | P2 |  | test | ✅ Done | 5 AC |
| TASK-141 | Supertrend 趋势指标实现与流式版本 | P2 |  | backend | ✅ Done | 5 AC |
| TASK-142 | Ichimoku Cloud 一目均衡表完整实现 | P2 |  | backend | ✅ Done | 5 AC |
| TASK-143 | TTM Squeeze Momentum 挤压动量指标 | P2 |  | backend | ✅ Done | 4 AC |
| TASK-144 | Williams Fractal 分形指标实现 | P2 |  | backend | ✅ Done | 5 AC |
| TASK-145 | VWAP 与 Anchored VWAP 实现 | P2 |  | backend | ✅ Done | 5 AC |
| TASK-146 | 风险统计指标（Sortino/Calmar/Information R | P2 |  | backend | ✅ Done | 5 AC |
| TASK-147 | 时间周期特征编码（正弦余弦/交易时段） | P2 |  | backend | ✅ Done | 5 AC |
| TASK-148 | Support/Resistance 自动检测与趋势强度量化 | P2 |  | backend | ✅ Done | 5 AC |
| TASK-149 | 指标背离自动检测（RSI/MACD Divergence） | P2 |  | backend | ✅ Done | 4 AC |
| TASK-150 | Meta-labeling 与 Event-driven Labels | P2 |  | backend | ✅ Done | 5 AC |
| TASK-151 | PCA 在线近似与特征重要性排序 | P2 |  | backend | ✅ Done | 5 AC |
| TASK-152 | GARCH 波动率状态增强与状态转换概率 | P2 |  | backend | ✅ Done | 5 AC |
| TASK-153 | Rayon 并行特征矩阵生成 | P2 |  | backend | ✅ Done | 5 AC |
| TASK-154 | 新增指标全量 Benchmark 与流式版本覆盖 | P2 |  | backend | ✅ Done | 5 AC |
| TASK-155 | 1M 数据规模性能对标与自动化回归报告 | P2 |  | backend | ✅ Done | 5 AC |
| TASK-158 | WMA SIMD 向量化深度优化（目标≥1.0x TA-Lib） | P2 |  | refactor | ✅ Done | 4 AC |
| TASK-159 | STOCH/STOCHF 单遍融合管线重写（目标≥1.0x TA-Li | P2 |  | refactor | ✅ Done | 4 AC |
| TASK-160 | WILLR 单调队列 + SIMD 优化（目标≥1.0x TA-Lib | P2 |  | refactor | ✅ Done | 3 AC |
| TASK-161 | AROON 索引追踪算法重构（目标≥1.0x TA-Lib） | P2 |  | refactor | ✅ Done | 3 AC |
| TASK-162 | OBV SIMD 批量累加 + 分支消除优化（目标≥1.0x TA-L | P2 |  | refactor | ✅ Done | 3 AC |
| TASK-163 | Watch List 指标批量优化（KAMA/TEMA/TRIMA/R | P2 |  | refactor | ✅ Done | 6 AC |
| TASK-164 | Watch List 方向性指标优化（ADX/ADXR/DI+/DI- | P2 |  | refactor | ✅ Done | 7 AC |
| TASK-165 | 全量性能验证 — 确保所有指标≥TA-Lib | P2 |  | test | ✅ Done | 4 AC |
| TASK-166 | Vortex Indicator (VI) 实现 | P2 |  | feature | ✅ Done | 4 AC |
| TASK-167 | Inertia Indicator 实现 | P2 |  | feature | ✅ Done | 4 AC |
| TASK-168 | Squeeze Momentum (John Carter版) 实现 | P2 |  | feature | ✅ Done | 4 AC |
| TASK-169 | QStick 指标实现 | P2 |  | feature | ✅ Done | 3 AC |
| TASK-170 | Jurik Moving Average (JMA) 实现 | P2 |  | feature | ✅ Done | 4 AC |
| TASK-171 | Kaufman Efficiency Ratio 独立导出 | P2 |  | feature | ✅ Done | 3 AC |
| TASK-172 | Chande Forecast Oscillator 实现 | P2 |  | feature | ✅ Done | 3 AC |
| TASK-173 | Twiggs Money Flow 实现 | P2 |  | feature | ✅ Done | 4 AC |
| TASK-174 | Keltner Channel 批处理版实现 | P2 |  | feature | ✅ Done | 3 AC |
| TASK-175 | Average Day Range (ADR) 实现 | P2 |  | feature | ✅ Done | 3 AC |
| TASK-176 | Chaikin Volatility 实现 | P2 |  | feature | ✅ Done | 3 AC |
| TASK-177 | Historical Volatility (Close-to-Clo | P2 |  | feature | ✅ Done | 3 AC |
| TASK-178 | Volume Zone Oscillator (VZO) 实现 | P2 |  | feature | ✅ Done | 3 AC |
| TASK-179 | Multi-timeframe VWAP 实现 | P2 |  | feature | ✅ Done | 3 AC |
| TASK-180 | Volume Momentum 实现 | P2 |  | feature | ✅ Done | 3 AC |
| TASK-181 | Kendall Tau 与 Spearman Rank 相关性 | P2 |  | feature | ✅ Done | 4 AC |
| TASK-182 | Rolling Quantile Regression 实现 | P2 |  | feature | ✅ Done | 3 AC |
| TASK-183 | Theil-Sen 稳健回归估计器 | P2 |  | feature | ✅ Done | 3 AC |
| TASK-184 | Mann-Kendall Trend Test 实现 | P2 |  | feature | ✅ Done | 3 AC |
| TASK-185 | Fractal Dimension (Higuchi + Box-co | P2 |  | feature | ✅ Done | 4 AC |
| TASK-186 | Approximate Entropy (ApEn) + Sample | P2 |  | feature | ✅ Done | 4 AC |
| TASK-187 | Detrended Fluctuation Analysis (DFA | P2 |  | feature | ✅ Done | 3 AC |
| TASK-188 | Lyapunov Exponent 滚动估计 | P2 |  | feature | ✅ Done | 3 AC |
| TASK-189 | Wavelet Transform Features (Haar/Da | P2 |  | feature | ✅ Done | 4 AC |
| TASK-190 | Fourier Transform Features 实现 | P2 |  | feature | ✅ Done | 4 AC |
| TASK-191 | Cross-Correlation Matrix 多资产实现 | P2 |  | feature | ✅ Done | 3 AC |
| TASK-192 | Granger Causality 滚动窗口检验 | P2 |  | feature | ✅ Done | 3 AC |
| TASK-193 | Information Coefficient (IC) 滚动计算 | P2 |  | feature | ✅ Done | 3 AC |
| TASK-194 | Turnover Ratio 特征实现 | P2 |  | feature | ✅ Done | 3 AC |
| TASK-195 | Order Flow Imbalance 增强版 | P2 |  | feature | ✅ Done | 3 AC |
| TASK-196 | 新增指标全量流式版本与 Registry 注册 | P2 |  | infra | ✅ Done | 3 AC |
| TASK-197 | 新增指标 FeatureEngine 集成 | P2 |  | infra | ✅ Done | 3 AC |
| TASK-198 | 新增指标与特征 Criterion Benchmark 全覆盖 | P2 |  | test | ✅ Done | 3 AC |
| TASK-199 | FFI 绑定更新 — Python/Node/C 暴露新增 API | P2 |  | infra | ✅ Done | 3 AC |
| TASK-200 | 最终性能全景报告 + 竞品对标 | P2 |  | test | ✅ Done | 3 AC |
| TASK-201 | 公式引擎核心引用函数补全（VALUEWHEN/LAST/BARSLAS | P2 |  | feature | ✅ Done | 6 AC |
| TASK-202 | ZigZag 系列公式函数（PEAK/TROUGH/PEAKBARS/ | P2 |  | feature | ✅ Done | 4 AC |
| TASK-203 | 高级查找函数（FINDHIGH/FINDLOW/TOPN/DRAWNU | P2 |  | feature | ✅ Done | 4 AC |
| TASK-204 | 语法兼容性增强（块注释/单引号/等号赋值/#注释） | P2 |  | feature | ✅ Done | 8 AC |
| TASK-205 | 信号过滤与交易标记函数（文华财经兼容） | P2 |  | feature | ✅ Done | 4 AC |
| TASK-206 | 绘图函数扩展（DRAWSL/DRAWTEXT_FIX/DRAWNUMB | P2 |  | feature | ✅ Done | 4 AC |
| TASK-207 | 多输出机制实现 | P2 |  | feature | ✅ Done | 4 AC |
| TASK-208 | 数组/序列操作增强（CUMSUM/CUMMAX/CUMMIN/PERC | P2 |  | feature | ✅ Done | 4 AC |
| TASK-209 | 高阶统计函数（SKEW/KURT/MODE/SORT/RANK） | P2 |  | feature | ✅ Done | 4 AC |
| TASK-210 | 跨周期引用支持（#WEEK/#MONTH/PERIODTYPE/REF | P2 |  | feature | ✅ Done | 4 AC |
| TASK-211 | 公式引擎惰性求值优化 | P2 |  | refactor | ✅ Done | 6 AC |
| TASK-212 | 公式引擎增量计算支持 | P2 |  | feature | ✅ Done | 4 AC |
| TASK-213 | 公式引擎并行计算优化 | P2 |  | refactor | ✅ Done | 4 AC |
| TASK-214 | 公式系统集成测试与兼容性验证 | P2 |  | test | ✅ Done | 6 AC |
| TASK-215 | 公式系统文档全面更新 | P2 |  | docs | ✅ Done | 5 AC |
| TASK-216 | 初始化 git 仓库与分支策略 | P0 |  | infra | ✅ Done | 4 AC |
| TASK-217 | 根 CI 质量门禁骨架 | P0 |  | infra | ✅ Done | 5 AC |
| TASK-218 | 共享测试数据集生成器 | P0 |  | test | 🔄 Pending | 3 AC |
| TASK-219 | 文档诚信审计与版本治理 | P0 |  | docs | ✅ Done | 3 AC |
| TASK-220 | TA-Lib C 参考输出生成器 | P0 |  | test | ✅ Done | 3 AC |
| TASK-221 | 逐函数比对器与分指标族容差策略 | P0 |  | test | 🔄 Pending | 2 AC |
| TASK-222 | COMPAT_MATRIX 自动生成并入 CI | P0 |  | docs | 🔄 Pending | 3 AC |
| TASK-223 | 统一 Criterion benchmark harness 与唯一权 | P0 |  | test | 🔄 Pending | 3 AC |
| TASK-224 | 竞品对标诚信化与性能回归门禁 | P0 |  | test | 🔄 Pending | 2 AC |
| TASK-225 | TDX/THS/DZH 真实公式语料回归集 | P0 |  | test | 🔄 Pending | 3 AC |
| TASK-226 | 四执行路径差分一致性测试 | P0 |  | test | 🔄 Pending | 2 AC |
| TASK-227 | 公式解析器 fuzzing 扩展与执行沙箱 | P0 |  | backend | 🔄 Pending | 2 AC |
| TASK-228 | 公式文法规范文档(EBNF+方言差异) | P0 |  | docs | 🔄 Pending | 2 AC |
| TASK-229 | FFI 全导出函数 panic 隔离与稳定错误码 | P0 |  | backend | 🔄 Pending | 4 AC |
| TASK-230 | FFI 内存所有权契约与泄漏测试 | P0 |  | test | 🔄 Pending | 2 AC |
| TASK-231 | cbindgen 头文件自动化与 ABI CI 校验 | P0 |  | infra | 🔄 Pending | 2 AC |
| TASK-232 | Python 多平台 wheel 发布与类型 stub | P0 |  | infra | 🔄 Pending | 3 AC |
| TASK-233 | pandas/polars 访问器与一行策略批跑 | P0 |  | backend | 🔄 Pending | 2 AC |
| TASK-234 | FFI 错误码到 Python 语义化异常映射 | P0 |  | backend | 🔄 Pending | 2 AC |
| TASK-235 | 绑定分级决策与成熟度标注 | P0 |  | docs | 🔄 Pending | 2 AC |
| TASK-236 | Tier1 绑定深化:真实包+类型+包内测试 | P0 |  | backend | 🔄 Pending | 2 AC |
| TASK-237 | 数据格式适配示例 | P0 |  | backend | 🔄 Pending | 2 AC |
| TASK-238 | 两套端到端教程(A股TDX迁移/加密实时) | P0 |  | docs | 🔄 Pending | 2 AC |
| TASK-239 | Pine v5 词法/语法解析器 | P0 |  | backend | 🔄 Pending | 3 AC |
| TASK-240 | Pine→AlphaTA AST 映射与内置函数表 | P0 |  | backend | 🔄 Pending | 3 AC |
| TASK-241 | Pine series/na 语义与跨周期映射 | P0 |  | backend | 🔄 Pending | 2 AC |
| TASK-242 | Pine 真实脚本语料回归与兼容矩阵 | P0 |  | test | 🔄 Pending | 3 AC |
| TASK-243 | Pine 方言在 CLI/Python/FFI 暴露 | P0 |  | backend | 🔄 Pending | 2 AC |
| TASK-244 | Pine 文法文档与迁移指南 | P0 |  | docs | 🔄 Pending | 2 AC |
| TASK-245 | 文档审计清理与 IA 重组 | P0 |  | docs | 🔄 Pending | 2 AC |
| TASK-246 | SSOT 生成器 | P0 |  | infra | 🔄 Pending | 2 AC |
| TASK-247 | mdBook 文档站点 | P0 |  | docs | 🔄 Pending | 3 AC |
| TASK-248 | 文档 CI 校验 | P0 |  | infra | 🔄 Pending | 2 AC |
| TASK-301 | SliceOutput trait: 消除 ndarray 冗余分配 | P0 |  | refactor | 🔄 Pending | 1 AC |
| TASK-302 | 单遍扫描重写 CCI/WILLR/AROONOSC | P0 |  | refactor | 🔄 Pending | 1 AC |
| TASK-303 | 单遍扫描重写 STOCH/STOCHF/STOCHRSI | P0 |  | refactor | 🔄 Pending | 1 AC |
| TASK-304 | 融合计算 ADX 系列共享 TR 路径 | P0 |  | refactor | 🔄 Pending | 1 AC |
| TASK-305 | EMA/DEMA/TEMA/KAMA 就地更新消除临时数组 | P0 |  | refactor | 🔄 Pending | 1 AC |
| TASK-306 | AVX2/SSE4 自动检测与 SMA/WMA SIMD 内核 | P0 |  | feature | 🔄 Pending | 1 AC |
| TASK-307 | 统计类 SIMD: STDDEV/VAR/LINEARREG | P0 |  | feature | 🔄 Pending | 1 AC |
| TASK-308 | HT_SINE/HT_PHASOR 算法重写 | P0 |  | refactor | 🔄 Pending | 1 AC |
| TASK-309 | unsafe slice 快速路径 feature gate | P0 |  | feature | 🔄 Pending | 1 AC |
| TASK-310 | Criterion 环境指纹标准化 + baseline | P0 |  | infra | 🔄 Pending | 1 AC |
| TASK-311 | 竞品对比脚本 Kand/ta-rs/quantedge-ta | P0 |  | infra | 🔄 Pending | 1 AC |
| TASK-312 | 补全 MINMAX/MINMAXINDEX | P0 |  | feature | 🔄 Pending | 1 AC |
| TASK-313 | 导出修复 top_bottom + pivot_points 清理 | P0 |  | bugfix | 🔄 Pending | 2 AC |
| TASK-314 | 命名统一: linearreg/stochrsi 别名 | P0 |  | refactor | 🔄 Pending | 1 AC |
| TASK-315 | _into 零拷贝 API 扩展至 20 指标 | P0 |  | feature | 🔄 Pending | 1 AC |
| TASK-316 | 流式 Batch 1: 动量类 15 指标 streaming | P0 |  | feature | 🔄 Pending | 1 AC |
| TASK-317 | 流式 Batch 2: 趋势/波动率/统计 15 指标 | P0 |  | feature | 🔄 Pending | 1 AC |
| TASK-318 | 流式 Batch 3: 成交量/广度 15 指标 | P0 |  | feature | 🔄 Pending | 1 AC |
| TASK-319 | 流式 Batch 4: CDL 前 20 个流式化 | P0 |  | feature | 🔄 Pending | 1 AC |
| TASK-320 | Pine 内置函数映射补全 15 个 | P0 |  | feature | 🔄 Pending | 1 AC |
| TASK-321 | Pine series history [] 与 barstate | P0 |  | feature | 🔄 Pending | 1 AC |
| TASK-322 | 公式引擎批量模式 eval_batch | P0 |  | feature | 🔄 Pending | 1 AC |
| TASK-323 | Python df.ta accessor 完整实现 | P0 |  | feature | 🔄 Pending | 1 AC |
| TASK-324 | Python 语义化异常实现 | P0 |  | feature | 🔄 Pending | 1 AC |
| TASK-325 | Node.js TypeScript 类型定义 | P0 |  | feature | 🔄 Pending | 1 AC |
| TASK-326 | CLI 工具 AlphaTA calc | P0 |  | feature | 🔄 Pending | 1 AC |
| TASK-327 | Jupyter Notebook 示例集 | P0 |  | docs | 🔄 Pending | 1 AC |
| TASK-328 | mdBook 构建 + GitHub Pages CI | P0 |  | infra | 🔄 Pending | 1 AC |

## Documents

- [PRD Markdown](PRD.md)
- [prd.json](prd.json) — machine-readable spec
- [docs/PROGRESS.md](docs/PROGRESS.md) — session-level progress log
- [.aza/specs/](.aza/specs/) — per-story spec folders

---
> This file is auto-generated by AzaLoop. Re-read before major decisions.