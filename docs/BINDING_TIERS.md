# AlphaTA 绑定分级与成熟度

AlphaTA 提供多语言 FFI 绑定。为管理用户预期与维护投入，按 **Tier1 / Tier 2 / Tier 3** 分级，并为每项能力标注成熟度。

## 分级概览

| Tier | 绑定 | 定位 | 成熟度 |
|------|------|------|--------|
| **Tier1** | Python | 生产就绪，PyPI 发布，完整 CI | `stable` |
| **Tier1** | Rust (native / `alpha-ta-core`) | 生产就绪，crates.io 发布 | `stable` |
| **Tier1** | Node.js / WASM | 生产就绪，npm 发布 | `stable` |
| **Tier2** | Go | 可用但 API 覆盖有限 | `beta` |
| **Tier2** | Java | 可用但 API 覆盖有限 | `beta` |
| **Tier2** | .NET | 可用但 API 覆盖有限 | `beta` |
| **Tier2** | C | 可用但 API 覆盖有限 | `beta` |
| **Tier3** | iOS | 实验性，无兼容承诺 | `experimental` |
| **Tier3** | Android | 实验性，无兼容承诺 | `experimental` |

## 能力矩阵

能力列：**指标计算** · **流式** · **公式引擎** · **ML 特征** · **可视化**

| 绑定 | Tier | 指标计算 | 流式 | 公式引擎 | ML 特征 | 可视化 | 成熟度 |
|------|------|----------|------|----------|---------|--------|--------|
| Python | Tier1 | ✅ 完整 | ✅ | ✅ | ✅ | ✅ | `stable` |
| Rust (native) | Tier1 | ✅ 完整 | ✅ | ✅ | ✅ | ✅ | `stable` |
| Node.js / WASM | Tier1 | ✅ 完整 | ✅ | ✅ | ⚠️ 部分 | ⚠️ 部分 | `stable` |
| Go | Tier2 | ✅ 核心子集 | ⚠️ 部分 | ❌ | ❌ | ❌ | `beta` |
| Java | Tier2 | ✅ 核心子集 | ⚠️ 部分 | ❌ | ❌ | ❌ | `beta` |
| .NET | Tier2 | ✅ 核心子集 | ⚠️ 部分 | ❌ | ❌ | ❌ | `beta` |
| C | Tier2 | ✅ 核心子集 | ⚠️ 部分 | ❌ | ❌ | ❌ | `beta` |
| iOS | Tier3 | ⚠️ 部分 | ❌ | ❌ | ❌ | ❌ | `experimental` |
| Android | Tier3 | ⚠️ 部分 | ❌ | ❌ | ❌ | ❌ | `experimental` |

图例：✅ 完整支持 · ⚠️ 部分支持 · ❌ 未支持或未验证

## Tier1 — 生产就绪

**要求**：真实包发布、CI 门禁、类型定义、包内测试矩阵、完整 API 文档与教程。

| 绑定 | 包管理器 | 目录 |
|------|----------|------|
| Python | `pip install alpha-ta` | `ffi/python-binding/` |
| Rust | `cargo add alpha-ta-core` | `core/` |
| Node.js | `npm install @alphata/node` | `ffi/node-binding/` |
| WASM | `npm install @alphata/wasm` | `wasm/` |

## Tier2 — 可用但有限

**要求**：保持可用、错误码映射、冒烟测试；README 明确标注成熟度，不做过度承诺。

| 绑定 | 目录 |
|------|------|
| Go | `ffi/go-binding/` |
| Java | `ffi/java-binding/` |
| .NET | `ffi/dotnet-binding/` |
| C | `ffi/c-binding/` |

## Tier3 — 实验性

**要求**：明确 `experimental` 标签，无向后兼容承诺，验证覆盖不足。

| 绑定 | 目录 |
|------|------|
| iOS | `ffi/ios-binding/` |
| Android | `ffi/android-binding/` |

## 成熟度标签定义

| 标签 | 含义 | 用户预期 |
|------|------|----------|
| `stable` | 生产可用 | API 稳定，semver 兼容承诺，完整 CI |
| `beta` | 功能可用但覆盖不全 | 核心路径可靠，高级特性可能缺失 |
| `experimental` | 实验性 | 可能随时变更，不建议生产依赖 |
| `planned` | 规划中 | 尚未实现或仅有设计文档 |

## 维护策略

1. **Tier1** 优先投入：新特性首先落地 Rust core，再同步到 Python / Node / WASM。
2. **Tier2** 维护模式：接 P1-4 统一错误码，保持冒烟测试绿灯。
3. **Tier3** 标注即止：不承诺 timeline，用户自行评估风险。

## 相关文档

- [Python 绑定 README](../ffi/python-binding/README.md)
- [Node 绑定 README](../ffi/node-binding/README.md)
- [Core README](../core/README.md)
- [优化计划 — 绑定分级](OPTIMIZATION_PLAN_2026-06-23.md#p2-2-绑定聚焦做深而非广撒网-)
