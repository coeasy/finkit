//! 公式优化等级枚举
//!
//! 用于控制 [`crate::formula::FormulaOptimizer`] 应用哪些 pass。
//!
//! # 等级递进
//!
//! - [`OptLevel::None`] — 不优化（保留原始 AST）
//! - [`OptLevel::Basic`] — 常量折叠 + 死代码消除
//! - [`OptLevel::Standard`] — 在 Basic 基础上 + 代数化简 + 强度削减 + 公共子表达式消除
//! - [`OptLevel::Aggressive`] — 在 Standard 基础上 + 循环不变量代码外提
//!
//! 默认等级 [`OptLevel::Standard`]，适合绝大多数公式。
//!
//! # 浮点语义
//!
//! `Aggressive` 等级会改变部分表达式的求值顺序（强度削减如 `x*2` → `x+x`），
//! 结果与原表达式在 1e-15 容差内一致，但**位精确结果可能不同**。
//! 如需位精确，可降级到 `Standard` 或 `None`。
//!
//! # 示例
//!
//! ```no_run
//! use alpha_ta_core::formula::{AstNode, FormulaOptimizer, OptLevel};
//!
//! let ast = AstNode::Number(1.0);
//! // 默认 Standard
//! let opt = FormulaOptimizer::optimize_with(&ast, OptLevel::default());
//!
//! // 显式指定
//! let opt = FormulaOptimizer::optimize_with(&ast, OptLevel::Aggressive);
//! ```

/// 公式优化等级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum OptLevel {
    /// 不优化：保留原始 AST。
    None,
    /// 基础：常量折叠 + 死代码消除。
    Basic,
    /// 标准：在 Basic 基础上 + 代数化简 + 强度削减 + CSE。
    /// **默认等级**。
    #[default]
    Standard,
    /// 激进：在 Standard 基础上 + 循环不变量代码外提。
    /// **注意**：可能改变浮点求值顺序，结果在 1e-15 容差内。
    Aggressive,
}

impl OptLevel {
    /// 返回 `OptLevel` 的描述字符串（用于日志/调试）。
    pub fn as_str(&self) -> &'static str {
        match self {
            OptLevel::None => "none",
            OptLevel::Basic => "basic",
            OptLevel::Standard => "standard",
            OptLevel::Aggressive => "aggressive",
        }
    }

    /// 返回本等级应启用的 pass 数量（仅用于诊断/基准报告）。
    pub fn pass_count(&self) -> usize {
        match self {
            OptLevel::None => 0,
            OptLevel::Basic => 2,
            OptLevel::Standard => 5,
            OptLevel::Aggressive => 6,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_is_standard() {
        assert_eq!(OptLevel::default(), OptLevel::Standard);
    }

    #[test]
    fn test_as_str() {
        assert_eq!(OptLevel::None.as_str(), "none");
        assert_eq!(OptLevel::Basic.as_str(), "basic");
        assert_eq!(OptLevel::Standard.as_str(), "standard");
        assert_eq!(OptLevel::Aggressive.as_str(), "aggressive");
    }

    #[test]
    fn test_pass_count() {
        assert_eq!(OptLevel::None.pass_count(), 0);
        assert_eq!(OptLevel::Basic.pass_count(), 2);
        assert_eq!(OptLevel::Standard.pass_count(), 5);
        assert_eq!(OptLevel::Aggressive.pass_count(), 6);
    }

    #[test]
    fn test_ordering() {
        // OptLevel 应按 None < Basic < Standard < Aggressive 排序
        assert!(OptLevel::None < OptLevel::Basic);
        assert!(OptLevel::Basic < OptLevel::Standard);
        assert!(OptLevel::Standard < OptLevel::Aggressive);
    }
}
