# 测试组织索引

本文档说明 `core/tests/` 目录下测试文件的分类和组织结构。

## 测试分类

### 兼容性测试 (Compatibility)
测试与外部系统/格式的兼容性。

- `dzh_compat_tests.rs` - 大智慧指标兼容性
- `em_compat_tests.rs` - 东方财富指标兼容性
- `fox_compat_tests.rs` - 同花顺指标兼容性
- `tdx_compat_tests.rs` - 通达信指标兼容性
- `ths_compat_tests.rs` - 同花顺指标兼容性
- `pine_integration_tests.rs` - Pine Script 兼容性
- `polars_integration_tests.rs` - Polars DataFrame 集成

### 边界情况测试 (Edge Cases)
测试极端输入、无效数据、边界条件。

- `edge_case_tests.rs` - 通用边界情况
- `edge_case_invalid_input.rs` - 无效输入处理
- `f32_tests.rs` - f32 精度测试

### 黄金测试 (Golden Tests)
基于参考实现的基准测试。

- `golden_tests.rs` - 核心黄金测试
- `golden_talib_tests.rs` - TA-Lib 对比测试
- `golden_example.rs` - 示例黄金测试
- `common/golden_loader.rs` - 黄金测试数据加载器

### 属性测试 (Property Tests)
基于 proptest 的不变量测试。

- `property_tests.rs` - 流式指标属性测试
- `common/property_templates.rs` - 属性测试模板

### 重绘/快照测试 (Repaint & Snapshot)
测试 forming-bar 重绘和状态快照功能。

- `repaint_tests.rs` - 重绘行为测试
- `serde_roundtrip_tests.rs` - 序列化/反序列化测试
- `classic_patterns_tests.rs` - 经典使用模式测试

### 公式引擎测试 (Formula Engine)
测试公式引擎功能。

- `formula_engine_integration.rs` - 公式引擎集成测试
- `formula_differential_tests.rs` - 公式差异测试
- `formula_regression.rs` - 公式回归测试
- `formula_cache_tests.rs` - 公式缓存测试
- `formula_compat.rs` - 公式兼容性测试
- `formula_partial_eval.rs` - 公式部分求值

### Builder 测试
测试 Builder 模式 API。

- `builder_tests.rs` - Builder 模式测试（22+ 测试用例）

### 价格源测试 (Price Source)
测试不同价格源配置。

- `price_source_tests.rs` - 价格源配置测试

### 稳定性测试 (Stability)
长时间运行稳定性测试。

- `long_run_stability.rs` - 长期运行稳定性

### 调试工具 (Debug)
调试和诊断工具。

- `debug_cvar.rs` - CVAR 调试

## 公共模块 (Common)

`tests/common/` 目录包含测试共享工具：

- `mod.rs` - 模块导出
- `golden_loader.rs` - 黄金测试数据加载
- `property_templates.rs` - 属性测试模板
- `streaming_test_templates.rs` - 流式指标测试模板宏

## 运行测试

```bash
# 运行所有测试
cargo test -p alpha-ta-core

# 运行特定类别测试
cargo test -p alpha-ta-core --test builder_tests
cargo test -p alpha-ta-core --test repaint_tests
cargo test -p alpha-ta-core --test property_tests

# 运行 golden 测试
cargo test -p alpha-ta-core --test golden_tests

# 运行兼容性测试
cargo test -p alpha-ta-core --test dzh_compat_tests
cargo test -p alpha-ta-core --test ths_compat_tests
```

## 测试模板使用

使用 `streaming_test_templates.rs` 中的宏简化测试编写：

```rust
use alphata_core::streaming::indicators::StreamingSma;

// Checkpoint/Restore 测试
test_checkpoint_f64!(test_sma_checkpoint, StreamingSma::new(14), 20);

// Clone/Snapshot 测试
test_clone_snapshot_f64!(test_sma_clone, StreamingSma::new(3), 2);

// Repaint 测试
test_repaint_f64!(test_sma_repaint, StreamingSma::new(3));

// Builder 测试
test_builder_ok!(test_sma_builder, StreamingSma, |b| b.period(20).build());
test_builder_err!(test_sma_builder_err, StreamingSma, |b| b.period(0).build());
```

## 测试覆盖统计

- **Builder 测试**: 22+ 测试用例（覆盖所有 Builder 类型）
- **Repaint 测试**: 15+ 测试用例（覆盖重绘、快照、克隆）
- **Golden 测试**: 100+ 测试用例（对比 TA-Lib 参考实现）
- **Property 测试**: 50+ 属性测试（proptest 生成）
- **兼容性测试**: 200+ 测试用例（多平台兼容性）

## 测试最佳实践

1. **使用模板宏**: 优先使用 `streaming_test_templates.rs` 中的宏
2. **命名规范**: 测试函数名应清晰表达测试意图
3. **边界测试**: 每个指标至少测试：空输入、单值、warmup 期、正常期
4. **精度测试**: 浮点比较使用 `assert!((a - b).abs() < 1e-10)`
5. **状态测试**: 测试 `reset()`、`clone()`、`save_state()`、`restore_state()`
