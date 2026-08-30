# Migration Guide

本仓库是面向多语言的新一代实现（Rust 内核）。本文说明从旧 API 形态迁移的要点。

## 1. Rust 用户

- 统一入口：`use finkit::{indicators, streaming, formula};`
- 批量指标签名：`fn rsi(close: &[f64], period: usize) -> Result<Array1<f64>, TaError>`。
- 流式接口：实现 `StreamingIndicator` 的 `StreamingXxx::next(&mut input) -> Option<f64>`。
- 若你之前用 `ta` crate：`ta` 与 `finkit` 的批量函数名基本一致（`Sma->sma`、`Ema->ema`、`Rsi->rsi`），返回值从 `Vec/usize 下标` 变为 `Array1<f64>`；用 `ta` 仅为基准对照（见 `core/Cargo.toml` 的 dev-dependency）。

## 2. 公式语言用户

- 公式引擎同时支持多种方言（TDX/THS/DZH/Pine），由 `FormulaEngine::parse(src, dialect)` 选择。
- 传统 `MA(CLOSE,5)`、`HHV/LLV/REF` 等 TDX 函数均可用（详见 `docs/formula/`）。

## 3. 其它语言

- 函数命名统一为 `ta_*`（C ABI）／语言惯用命名（各绑定）。
- 所有分配资源的函数必须配对释放：C 用 `ta_free_result`，.NET 用 `ta_free_cstring`。

## 4. 从旧版本升级

- 完整变更见 [RELEASE_NOTES_v2.0.md](./RELEASE_NOTES_v2.0.md) 与 [CHANGELOG.md](../CHANGELOG.md)。
- 破坏性改动集中在：版本号规范、仓库身份（现为 `github.com/coeasy/finkit`）、FFI ABI 前缀统一。

## 5. 迁移清单

- [ ] 确认目标语言绑定已随发行包更新。
- [ ] 将旧 import 路径改为新路径（如 Go：`github.com/coeasy/finkit/go/ta`）。
- [ ] 运行 `cargo test -p finkit` 与各语言冒烟测试。
- [ ] 用 golden 用例核对输出与 TA-Lib 对齐（`tests/golden/`）。