# Finkit v0.1.0 Implementation Roadmap

## Release Goal

交付第一个真正可安装、可验证、版本统一的 Finkit 基础版本。

## Phase A — Identity & Version Baseline

- [x] Rust workspace 版本统一到 `0.1.0`
- [x] Python distribution 切换到 `finkit==0.1.0`
- [x] Python 公共入口切换到 `import finkit`
- [x] Node 主包和 native package 版本统一 `0.1.0`
- [x] 修复 Node loader 与 native package 名称断链
- [x] Java Maven metadata 统一 `0.1.0`
- [x] .NET NuGet metadata 统一 `0.1.0`
- [x] Cargo.lock 本地 workspace package version 完成同步并提交

## Phase B — Installable Python Package

- [x] maturin package metadata
- [x] `finkit.alpha_ta` native extension layout
- [x] Python ABI3 (`py38+`) 方案
- [x] Linux wheel build workflow
- [x] wheel 安装 smoke test
- [x] Python 3.8 / 3.11 / 3.13 安装验证矩阵
- [ ] PR CI 全绿验证

## Phase C — CLI Productization

- [x] Cargo binary 输出名调整为 `finkit`
- [x] CLI help 中公共名称切换为 Finkit
- [ ] Linux/macOS/Windows release binary 验证

## Phase D — Release Pipeline

- [x] `v0.1.0` tag release workflow
- [x] Python Linux/macOS/Windows wheel assets
- [x] Python sdist asset
- [x] Linux/macOS/Windows CLI assets
- [x] GitHub Release 自动创建/更新
- [ ] 在 CI green 后合并分支
- [ ] 创建 `v0.1.0` tag
- [ ] 验证 Release Assets 可实际安装运行

## Phase E — After v0.1.0

### v0.1.x

- 扩展交易终端公式兼容矩阵
- 通达信语义精确对齐
- 同花顺兼容层
- 东方财富常用公式兼容层
- Pine Script 常用 TA 子集
- 类型定义与 API 文档完善
- 多语言包构建 smoke test

### v0.2.0

- 公开 Finkit namespace 向 Node / Java / .NET 扩展
- Factor API 标准化
- Formula IR 稳定化
- 更完整的 benchmark / precision gates

### v1.0.0

- 稳定 API / ABI
- 完整多语言 registry 发布
- 完整公式兼容测试集
- 长期兼容与性能基线
