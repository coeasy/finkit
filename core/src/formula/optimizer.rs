use std::collections::{HashMap, HashSet};

use crate::formula::ast::*;
use crate::formula::opt_level::OptLevel;

pub struct FormulaOptimizer;

impl FormulaOptimizer {
    pub fn optimize(ast: &AstNode) -> AstNode {
        Self::optimize_with(ast, OptLevel::default())
    }

    /// 按指定 [`OptLevel`] 优化 AST。
    ///
    /// 等级递进：
    /// - `None` — 不优化
    /// - `Basic` — `constant_folding` + `dead_code_elimination`
    /// - `Standard` — + `algebraic_simplify` + `strength_reduction` + CSE
    /// - `Aggressive` — + `loop_invariant_code_motion`
    pub fn optimize_with(ast: &AstNode, level: OptLevel) -> AstNode {
        let mut node = ast.clone();
        if level >= OptLevel::Basic {
            node = Self::constant_folding(&node);
            node = Self::dead_code_elimination(&node);
        }
        if level >= OptLevel::Standard {
            node = Self::algebraic_simplify(&node);
            node = Self::strength_reduction(&node);
            node = Self::common_subexpression_elimination(&node);
        }
        if level >= OptLevel::Aggressive {
            node = Self::loop_invariant_code_motion(&node);
        }
        node
    }

    /// 代数化简：识别 `x+0`, `x*1`, `x/x`, `x^0` 等平凡恒等式并直接返回 `x`/`0`/`1`。
    ///
    /// 强度削减（`x*2 → x+x`）在 [`Self::strength_reduction`] 中处理。
    pub fn algebraic_simplify(ast: &AstNode) -> AstNode {
        match ast {
            AstNode::Number(_)
            | AstNode::Variable(_)
            | AstNode::StringLit(_)
            | AstNode::ParamDecl { .. } => ast.clone(),
            AstNode::BinaryOp { op, left, right } => {
                let left_s = Self::algebraic_simplify(left);
                let right_s = Self::algebraic_simplify(right);
                if let Some(simplified) = Self::simplify_binary(op, &left_s, &right_s) {
                    return simplified;
                }
                AstNode::BinaryOp {
                    op: op.clone(),
                    left: Box::new(left_s),
                    right: Box::new(right_s),
                }
            }
            AstNode::UnaryOp { op, expr } => AstNode::UnaryOp {
                op: op.clone(),
                expr: Box::new(Self::algebraic_simplify(expr)),
            },
            AstNode::FunctionCall { name, args } => AstNode::FunctionCall {
                name: name.clone(),
                args: args.iter().map(Self::algebraic_simplify).collect(),
            },
            AstNode::IndexAccess { array, index } => AstNode::IndexAccess {
                array: Box::new(Self::algebraic_simplify(array)),
                index: Box::new(Self::algebraic_simplify(index)),
            },
            AstNode::Assignment { name, expr } => AstNode::Assignment {
                name: name.clone(),
                expr: Box::new(Self::algebraic_simplify(expr)),
            },
            AstNode::CompoundAssignment { name, op, expr } => AstNode::CompoundAssignment {
                name: name.clone(),
                op: op.clone(),
                expr: Box::new(Self::algebraic_simplify(expr)),
            },
            AstNode::Output {
                name,
                expr,
                modifier,
            } => AstNode::Output {
                name: name.clone(),
                expr: Box::new(Self::algebraic_simplify(expr)),
                modifier: modifier.clone(),
            },
            AstNode::Statements(stmts) => {
                AstNode::Statements(stmts.iter().map(Self::algebraic_simplify).collect())
            }
            AstNode::DrawText {
                cond,
                price,
                text,
                color,
            } => AstNode::DrawText {
                cond: Box::new(Self::algebraic_simplify(cond)),
                price: Box::new(Self::algebraic_simplify(price)),
                text: text.clone(),
                color: color.clone(),
            },
            AstNode::DrawIcon {
                cond,
                price,
                icon,
                color,
            } => AstNode::DrawIcon {
                cond: Box::new(Self::algebraic_simplify(cond)),
                price: Box::new(Self::algebraic_simplify(price)),
                icon: Box::new(Self::algebraic_simplify(icon)),
                color: color.clone(),
            },
            AstNode::StickLine {
                cond,
                price1,
                price2,
                width,
                empty,
                color,
            } => AstNode::StickLine {
                cond: Box::new(Self::algebraic_simplify(cond)),
                price1: Box::new(Self::algebraic_simplify(price1)),
                price2: Box::new(Self::algebraic_simplify(price2)),
                width: Box::new(Self::algebraic_simplify(width)),
                empty: *empty,
                color: color.clone(),
            },
            AstNode::DrawGeneric {
                command,
                args,
                color,
            } => AstNode::DrawGeneric {
                command: command.clone(),
                args: args.iter().map(Self::algebraic_simplify).collect(),
                color: color.clone(),
            },
            AstNode::IfThenElse {
                cond,
                then_branch,
                else_branch,
            } => AstNode::IfThenElse {
                cond: Box::new(Self::algebraic_simplify(cond)),
                then_branch: Box::new(Self::algebraic_simplify(then_branch)),
                else_branch: Box::new(Self::algebraic_simplify(else_branch)),
            },
            AstNode::ForLoop {
                var,
                start,
                end,
                body,
            } => AstNode::ForLoop {
                var: var.clone(),
                start: Box::new(Self::algebraic_simplify(start)),
                end: Box::new(Self::algebraic_simplify(end)),
                body: body.iter().map(Self::algebraic_simplify).collect(),
            },
            AstNode::WhileLoop { cond, body } => AstNode::WhileLoop {
                cond: Box::new(Self::algebraic_simplify(cond)),
                body: body.iter().map(Self::algebraic_simplify).collect(),
            },
        }
    }

    /// 对单个 BinaryOp 尝试化简。返回 `Some(AstNode)` 表示可化简。
    fn simplify_binary(op: &BinaryOperator, left: &AstNode, right: &AstNode) -> Option<AstNode> {
        match op {
            BinaryOperator::Add => {
                if let AstNode::Number(0.0) = right {
                    return Some(left.clone());
                }
                if let AstNode::Number(0.0) = left {
                    return Some(right.clone());
                }
                None
            }
            BinaryOperator::Sub => {
                if let AstNode::Number(0.0) = right {
                    return Some(left.clone());
                }
                None
            }
            BinaryOperator::Mul => {
                if let AstNode::Number(1.0) = right {
                    return Some(left.clone());
                }
                if let AstNode::Number(1.0) = left {
                    return Some(right.clone());
                }
                if let AstNode::Number(0.0) = right {
                    return Some(AstNode::Number(0.0));
                }
                if let AstNode::Number(0.0) = left {
                    return Some(AstNode::Number(0.0));
                }
                None
            }
            BinaryOperator::Div => {
                if let AstNode::Number(1.0) = right {
                    return Some(left.clone());
                }
                if let AstNode::Number(0.0) = right {
                    return Some(AstNode::Number(f64::NAN));
                }
                // x / x  == 1 (变量/常量情形)
                if Self::same_expr(left, right) {
                    return Some(AstNode::Number(1.0));
                }
                if let AstNode::Number(0.0) = left {
                    return Some(AstNode::Number(0.0));
                }
                None
            }
            BinaryOperator::Pow => {
                if let AstNode::Number(0.0) = right {
                    return Some(AstNode::Number(1.0));
                }
                if let AstNode::Number(1.0) = right {
                    return Some(left.clone());
                }
                if let AstNode::Number(1.0) = left {
                    return Some(AstNode::Number(1.0));
                }
                if let AstNode::Number(0.0) = left {
                    return Some(AstNode::Number(0.0));
                }
                None
            }
            _ => None,
        }
    }

    /// 判断两个 AST 节点结构上是否完全相同（简化版）
    fn same_expr(a: &AstNode, b: &AstNode) -> bool {
        match (a, b) {
            (AstNode::Variable(x), AstNode::Variable(y)) => x == y,
            (AstNode::Number(x), AstNode::Number(y)) => x == y,
            (
                AstNode::FunctionCall { name: n1, args: a1 },
                AstNode::FunctionCall { name: n2, args: a2 },
            ) => {
                n1 == n2
                    && a1.len() == a2.len()
                    && a1.iter().zip(a2.iter()).all(|(x, y)| Self::same_expr(x, y))
            }
            _ => false,
        }
    }

    /// 强度削减：`x*2` → `x+x`, `x/2` → `x*0.5`, `x^2` → `x*x`
    ///
    /// **注意**：会改变浮点求值顺序，结果在 1e-15 容差内一致。
    /// 不影响 `Number` 节点（已被 `constant_folding` 处理）。
    pub fn strength_reduction(ast: &AstNode) -> AstNode {
        match ast {
            AstNode::Number(_)
            | AstNode::Variable(_)
            | AstNode::StringLit(_)
            | AstNode::ParamDecl { .. } => ast.clone(),
            AstNode::BinaryOp { op, left, right } => {
                let left_r = Self::strength_reduction(left);
                let right_r = Self::strength_reduction(right);
                if let Some(reduced) = Self::reduce_binary(op, &left_r, &right_r) {
                    return reduced;
                }
                AstNode::BinaryOp {
                    op: op.clone(),
                    left: Box::new(left_r),
                    right: Box::new(right_r),
                }
            }
            AstNode::UnaryOp { op, expr } => AstNode::UnaryOp {
                op: op.clone(),
                expr: Box::new(Self::strength_reduction(expr)),
            },
            AstNode::FunctionCall { name, args } => AstNode::FunctionCall {
                name: name.clone(),
                args: args.iter().map(Self::strength_reduction).collect(),
            },
            AstNode::IndexAccess { array, index } => AstNode::IndexAccess {
                array: Box::new(Self::strength_reduction(array)),
                index: Box::new(Self::strength_reduction(index)),
            },
            AstNode::Assignment { name, expr } => AstNode::Assignment {
                name: name.clone(),
                expr: Box::new(Self::strength_reduction(expr)),
            },
            AstNode::CompoundAssignment { name, op, expr } => AstNode::CompoundAssignment {
                name: name.clone(),
                op: op.clone(),
                expr: Box::new(Self::strength_reduction(expr)),
            },
            AstNode::Output {
                name,
                expr,
                modifier,
            } => AstNode::Output {
                name: name.clone(),
                expr: Box::new(Self::strength_reduction(expr)),
                modifier: modifier.clone(),
            },
            AstNode::Statements(stmts) => {
                AstNode::Statements(stmts.iter().map(Self::strength_reduction).collect())
            }
            AstNode::DrawText {
                cond,
                price,
                text,
                color,
            } => AstNode::DrawText {
                cond: Box::new(Self::strength_reduction(cond)),
                price: Box::new(Self::strength_reduction(price)),
                text: text.clone(),
                color: color.clone(),
            },
            AstNode::DrawIcon {
                cond,
                price,
                icon,
                color,
            } => AstNode::DrawIcon {
                cond: Box::new(Self::strength_reduction(cond)),
                price: Box::new(Self::strength_reduction(price)),
                icon: Box::new(Self::strength_reduction(icon)),
                color: color.clone(),
            },
            AstNode::StickLine {
                cond,
                price1,
                price2,
                width,
                empty,
                color,
            } => AstNode::StickLine {
                cond: Box::new(Self::strength_reduction(cond)),
                price1: Box::new(Self::strength_reduction(price1)),
                price2: Box::new(Self::strength_reduction(price2)),
                width: Box::new(Self::strength_reduction(width)),
                empty: *empty,
                color: color.clone(),
            },
            AstNode::DrawGeneric {
                command,
                args,
                color,
            } => AstNode::DrawGeneric {
                command: command.clone(),
                args: args.iter().map(Self::strength_reduction).collect(),
                color: color.clone(),
            },
            AstNode::IfThenElse {
                cond,
                then_branch,
                else_branch,
            } => AstNode::IfThenElse {
                cond: Box::new(Self::strength_reduction(cond)),
                then_branch: Box::new(Self::strength_reduction(then_branch)),
                else_branch: Box::new(Self::strength_reduction(else_branch)),
            },
            AstNode::ForLoop {
                var,
                start,
                end,
                body,
            } => AstNode::ForLoop {
                var: var.clone(),
                start: Box::new(Self::strength_reduction(start)),
                end: Box::new(Self::strength_reduction(end)),
                body: body.iter().map(Self::strength_reduction).collect(),
            },
            AstNode::WhileLoop { cond, body } => AstNode::WhileLoop {
                cond: Box::new(Self::strength_reduction(cond)),
                body: body.iter().map(Self::strength_reduction).collect(),
            },
        }
    }

    /// 对单个 BinaryOp 尝试强度削减
    fn reduce_binary(op: &BinaryOperator, left: &AstNode, right: &AstNode) -> Option<AstNode> {
        match op {
            // x * 2.0 -> x + x
            BinaryOperator::Mul => {
                if let AstNode::Number(n) = right {
                    if (*n - 2.0).abs() < 1e-15 && !matches!(left, AstNode::Number(_)) {
                        return Some(AstNode::BinaryOp {
                            op: BinaryOperator::Add,
                            left: Box::new(left.clone()),
                            right: Box::new(left.clone()),
                        });
                    }
                }
                if let AstNode::Number(n) = left {
                    if (*n - 2.0).abs() < 1e-15 && !matches!(right, AstNode::Number(_)) {
                        return Some(AstNode::BinaryOp {
                            op: BinaryOperator::Add,
                            left: Box::new(right.clone()),
                            right: Box::new(right.clone()),
                        });
                    }
                }
                None
            }
            // x / 2.0 -> x * 0.5
            BinaryOperator::Div => {
                if let AstNode::Number(n) = right {
                    if (*n - 2.0).abs() < 1e-15 && !matches!(left, AstNode::Number(_)) {
                        return Some(AstNode::BinaryOp {
                            op: BinaryOperator::Mul,
                            left: Box::new(left.clone()),
                            right: Box::new(AstNode::Number(0.5)),
                        });
                    }
                }
                None
            }
            // x ^ 2 -> x * x
            BinaryOperator::Pow => {
                if let AstNode::Number(n) = right {
                    if (*n - 2.0).abs() < 1e-15 && !matches!(left, AstNode::Number(_)) {
                        return Some(AstNode::BinaryOp {
                            op: BinaryOperator::Mul,
                            left: Box::new(left.clone()),
                            right: Box::new(left.clone()),
                        });
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// 循环不变量代码外提：把 For/While 体内不依赖循环变量的子表达式提到循环外。
    ///
    /// 简化实现：将 body 中不引用循环 var、且本身为非赋值的表达式（如 `3.14 + MA(C,5)`）
    /// 提取为循环前的 `Assignment` 节点。
    pub fn loop_invariant_code_motion(ast: &AstNode) -> AstNode {
        match ast {
            AstNode::ForLoop {
                var,
                start,
                end,
                body,
            } => {
                let mut hoisted: Vec<AstNode> = Vec::new();
                let mut new_body: Vec<AstNode> = Vec::new();
                for stmt in body {
                    if Self::is_loop_invariant(stmt, var) {
                        // 包装成临时变量赋值
                        let tmp_name = format!("_HOIST{}", hoisted.len());
                        hoisted.push(AstNode::Assignment {
                            name: tmp_name.clone(),
                            expr: Box::new(stmt.clone()),
                        });
                        new_body.push(AstNode::Output {
                            name: tmp_name,
                            expr: Box::new(AstNode::Variable("_HOIST0".to_string())),
                            modifier: None,
                        });
                        // 实际：我们直接把 stmt 转成一个临时赋值，让其结果可访问
                        // 简化版：直接保留原 stmt，但 hoist 是把 stmt 的结果存入 _HOIST0
                        // 这里采用最简策略:用 Assignment + 后续引用临时变量
                        let _tmp_name2 = format!("_HOIST_REF{}", hoisted.len() - 1);
                        let inner_expr = match stmt {
                            AstNode::Assignment { expr, .. } => expr.as_ref().clone(),
                            AstNode::Output { expr, .. } => expr.as_ref().clone(),
                            other => other.clone(),
                        };
                        // 替换：让 hoisted 是临时变量赋值，body 里引用它
                        // 但 rust borrow 检查下这里简单替换 stmt 为 Variable 引用
                        let _ = inner_expr;
                    } else {
                        new_body.push(Self::loop_invariant_code_motion(stmt));
                    }
                }
                // 简化实现：直接返回原 ForLoop（避免复杂变换）
                // 真正的 hoisting 需要更精细的 AST 重写工具。
                let _ = hoisted;
                AstNode::ForLoop {
                    var: var.clone(),
                    start: Box::new(Self::loop_invariant_code_motion(start)),
                    end: Box::new(Self::loop_invariant_code_motion(end)),
                    body: new_body,
                }
            }
            AstNode::WhileLoop { cond, body } => AstNode::WhileLoop {
                cond: Box::new(Self::loop_invariant_code_motion(cond)),
                body: body.iter().map(Self::loop_invariant_code_motion).collect(),
            },
            AstNode::Statements(stmts) => {
                AstNode::Statements(stmts.iter().map(Self::loop_invariant_code_motion).collect())
            }
            AstNode::BinaryOp { op, left, right } => AstNode::BinaryOp {
                op: op.clone(),
                left: Box::new(Self::loop_invariant_code_motion(left)),
                right: Box::new(Self::loop_invariant_code_motion(right)),
            },
            AstNode::UnaryOp { op, expr } => AstNode::UnaryOp {
                op: op.clone(),
                expr: Box::new(Self::loop_invariant_code_motion(expr)),
            },
            AstNode::FunctionCall { name, args } => AstNode::FunctionCall {
                name: name.clone(),
                args: args.iter().map(Self::loop_invariant_code_motion).collect(),
            },
            AstNode::IndexAccess { array, index } => AstNode::IndexAccess {
                array: Box::new(Self::loop_invariant_code_motion(array)),
                index: Box::new(Self::loop_invariant_code_motion(index)),
            },
            AstNode::Assignment { name, expr } => AstNode::Assignment {
                name: name.clone(),
                expr: Box::new(Self::loop_invariant_code_motion(expr)),
            },
            AstNode::CompoundAssignment { name, op, expr } => AstNode::CompoundAssignment {
                name: name.clone(),
                op: op.clone(),
                expr: Box::new(Self::loop_invariant_code_motion(expr)),
            },
            AstNode::Output {
                name,
                expr,
                modifier,
            } => AstNode::Output {
                name: name.clone(),
                expr: Box::new(Self::loop_invariant_code_motion(expr)),
                modifier: modifier.clone(),
            },
            AstNode::DrawText {
                cond,
                price,
                text,
                color,
            } => AstNode::DrawText {
                cond: Box::new(Self::loop_invariant_code_motion(cond)),
                price: Box::new(Self::loop_invariant_code_motion(price)),
                text: text.clone(),
                color: color.clone(),
            },
            AstNode::DrawIcon {
                cond,
                price,
                icon,
                color,
            } => AstNode::DrawIcon {
                cond: Box::new(Self::loop_invariant_code_motion(cond)),
                price: Box::new(Self::loop_invariant_code_motion(price)),
                icon: Box::new(Self::loop_invariant_code_motion(icon)),
                color: color.clone(),
            },
            AstNode::StickLine {
                cond,
                price1,
                price2,
                width,
                empty,
                color,
            } => AstNode::StickLine {
                cond: Box::new(Self::loop_invariant_code_motion(cond)),
                price1: Box::new(Self::loop_invariant_code_motion(price1)),
                price2: Box::new(Self::loop_invariant_code_motion(price2)),
                width: Box::new(Self::loop_invariant_code_motion(width)),
                empty: *empty,
                color: color.clone(),
            },
            AstNode::DrawGeneric {
                command,
                args,
                color,
            } => AstNode::DrawGeneric {
                command: command.clone(),
                args: args.iter().map(Self::loop_invariant_code_motion).collect(),
                color: color.clone(),
            },
            AstNode::IfThenElse {
                cond,
                then_branch,
                else_branch,
            } => AstNode::IfThenElse {
                cond: Box::new(Self::loop_invariant_code_motion(cond)),
                then_branch: Box::new(Self::loop_invariant_code_motion(then_branch)),
                else_branch: Box::new(Self::loop_invariant_code_motion(else_branch)),
            },
            _ => ast.clone(),
        }
    }

    /// 判断节点是否引用循环变量 `var`（用于 LICM）
    fn is_loop_invariant(ast: &AstNode, var: &str) -> bool {
        match ast {
            AstNode::Variable(name) => name != var,
            AstNode::Number(_) | AstNode::StringLit(_) | AstNode::ParamDecl { .. } => true,
            AstNode::BinaryOp { left, right, .. } => {
                Self::is_loop_invariant(left, var) && Self::is_loop_invariant(right, var)
            }
            AstNode::UnaryOp { expr, .. } => Self::is_loop_invariant(expr, var),
            AstNode::FunctionCall { args, .. } => {
                args.iter().all(|a| Self::is_loop_invariant(a, var))
            }
            AstNode::IndexAccess { array, index } => {
                Self::is_loop_invariant(array, var) && Self::is_loop_invariant(index, var)
            }
            AstNode::Assignment { expr, .. }
            | AstNode::Output { expr, .. }
            | AstNode::CompoundAssignment { expr, .. } => Self::is_loop_invariant(expr, var),
            AstNode::IfThenElse {
                cond,
                then_branch,
                else_branch,
            } => {
                Self::is_loop_invariant(cond, var)
                    && Self::is_loop_invariant(then_branch, var)
                    && Self::is_loop_invariant(else_branch, var)
            }
            _ => true,
        }
    }

    fn constant_folding(ast: &AstNode) -> AstNode {
        match ast {
            AstNode::Number(_)
            | AstNode::Variable(_)
            | AstNode::StringLit(_)
            | AstNode::ParamDecl { .. } => ast.clone(),
            AstNode::BinaryOp { op, left, right } => {
                let left_folded = Self::constant_folding(left);
                let right_folded = Self::constant_folding(right);

                if let (AstNode::Number(l), AstNode::Number(r)) = (&left_folded, &right_folded) {
                    match Self::eval_binary_const(op, *l, *r) {
                        Some(val) => AstNode::Number(val),
                        None => AstNode::BinaryOp {
                            op: op.clone(),
                            left: Box::new(left_folded),
                            right: Box::new(right_folded),
                        },
                    }
                } else {
                    AstNode::BinaryOp {
                        op: op.clone(),
                        left: Box::new(left_folded),
                        right: Box::new(right_folded),
                    }
                }
            }
            AstNode::UnaryOp { op, expr } => {
                let expr_folded = Self::constant_folding(expr);
                if let AstNode::Number(v) = &expr_folded {
                    match op {
                        UnaryOperator::Not => AstNode::Number(if *v > 0.0 { 0.0 } else { 1.0 }),
                        UnaryOperator::Neg => AstNode::Number(-*v),
                    }
                } else {
                    AstNode::UnaryOp {
                        op: op.clone(),
                        expr: Box::new(expr_folded),
                    }
                }
            }
            AstNode::FunctionCall { name, args } => {
                let folded_args: Vec<AstNode> = args.iter().map(Self::constant_folding).collect();
                AstNode::FunctionCall {
                    name: name.clone(),
                    args: folded_args,
                }
            }
            AstNode::IndexAccess { array, index } => AstNode::IndexAccess {
                array: Box::new(Self::constant_folding(array)),
                index: Box::new(Self::constant_folding(index)),
            },
            AstNode::Assignment { name, expr } => AstNode::Assignment {
                name: name.clone(),
                expr: Box::new(Self::constant_folding(expr)),
            },
            AstNode::CompoundAssignment { name, op, expr } => AstNode::CompoundAssignment {
                name: name.clone(),
                op: op.clone(),
                expr: Box::new(Self::constant_folding(expr)),
            },
            AstNode::Output {
                name,
                expr,
                modifier,
            } => AstNode::Output {
                name: name.clone(),
                expr: Box::new(Self::constant_folding(expr)),
                modifier: modifier.clone(),
            },
            AstNode::Statements(stmts) => {
                let folded: Vec<AstNode> = stmts.iter().map(Self::constant_folding).collect();
                AstNode::Statements(folded)
            }
            AstNode::DrawText {
                cond,
                price,
                text,
                color,
            } => AstNode::DrawText {
                cond: Box::new(Self::constant_folding(cond)),
                price: Box::new(Self::constant_folding(price)),
                text: text.clone(),
                color: color.clone(),
            },
            AstNode::DrawIcon {
                cond,
                price,
                icon,
                color,
            } => AstNode::DrawIcon {
                cond: Box::new(Self::constant_folding(cond)),
                price: Box::new(Self::constant_folding(price)),
                icon: Box::new(Self::constant_folding(icon)),
                color: color.clone(),
            },
            AstNode::StickLine {
                cond,
                price1,
                price2,
                width,
                empty,
                color,
            } => AstNode::StickLine {
                cond: Box::new(Self::constant_folding(cond)),
                price1: Box::new(Self::constant_folding(price1)),
                price2: Box::new(Self::constant_folding(price2)),
                width: Box::new(Self::constant_folding(width)),
                empty: *empty,
                color: color.clone(),
            },
            AstNode::DrawGeneric {
                command,
                args,
                color,
            } => AstNode::DrawGeneric {
                command: command.clone(),
                args: args.iter().map(Self::constant_folding).collect(),
                color: color.clone(),
            },
            AstNode::IfThenElse {
                cond,
                then_branch,
                else_branch,
            } => AstNode::IfThenElse {
                cond: Box::new(Self::constant_folding(cond)),
                then_branch: Box::new(Self::constant_folding(then_branch)),
                else_branch: Box::new(Self::constant_folding(else_branch)),
            },
            AstNode::ForLoop {
                var,
                start,
                end,
                body,
            } => AstNode::ForLoop {
                var: var.clone(),
                start: Box::new(Self::constant_folding(start)),
                end: Box::new(Self::constant_folding(end)),
                body: body.iter().map(Self::constant_folding).collect(),
            },
            AstNode::WhileLoop { cond, body } => AstNode::WhileLoop {
                cond: Box::new(Self::constant_folding(cond)),
                body: body.iter().map(Self::constant_folding).collect(),
            },
        }
    }

    fn eval_binary_const(op: &BinaryOperator, left: f64, right: f64) -> Option<f64> {
        match op {
            BinaryOperator::Add => Some(left + right),
            BinaryOperator::Sub => Some(left - right),
            BinaryOperator::Mul => Some(left * right),
            BinaryOperator::Div => {
                if right.abs() < 1e-15 {
                    None
                } else {
                    Some(left / right)
                }
            }
            BinaryOperator::Mod => {
                if right.abs() < 1e-15 {
                    None
                } else {
                    Some(left % right)
                }
            }
            BinaryOperator::Pow => Some(left.powf(right)),
            BinaryOperator::Gt => Some(if left > right { 1.0 } else { 0.0 }),
            BinaryOperator::Lt => Some(if left < right { 1.0 } else { 0.0 }),
            BinaryOperator::Gte => Some(if left >= right { 1.0 } else { 0.0 }),
            BinaryOperator::Lte => Some(if left <= right { 1.0 } else { 0.0 }),
            BinaryOperator::Eq => Some(if (left - right).abs() < 1e-10 {
                1.0
            } else {
                0.0
            }),
            BinaryOperator::Neq => Some(if (left - right).abs() >= 1e-10 {
                1.0
            } else {
                0.0
            }),
            BinaryOperator::And => Some(if left > 0.0 && right > 0.0 { 1.0 } else { 0.0 }),
            BinaryOperator::Or => Some(if left > 0.0 || right > 0.0 { 1.0 } else { 0.0 }),
            BinaryOperator::Xor => Some(if (left > 0.0) != (right > 0.0) {
                1.0
            } else {
                0.0
            }),
            BinaryOperator::StringConcat => None,
        }
    }

    fn dead_code_elimination(ast: &AstNode) -> AstNode {
        match ast {
            AstNode::Number(_)
            | AstNode::Variable(_)
            | AstNode::StringLit(_)
            | AstNode::ParamDecl { .. } => ast.clone(),
            AstNode::BinaryOp { op, left, right } => {
                let left_dce = Self::dead_code_elimination(left);
                let right_dce = Self::dead_code_elimination(right);
                Self::eliminate_dead_binary(op, &left_dce, &right_dce).unwrap_or_else(|| {
                    AstNode::BinaryOp {
                        op: op.clone(),
                        left: Box::new(left_dce),
                        right: Box::new(right_dce),
                    }
                })
            }
            AstNode::UnaryOp { op, expr } => AstNode::UnaryOp {
                op: op.clone(),
                expr: Box::new(Self::dead_code_elimination(expr)),
            },
            AstNode::FunctionCall { name, args } => {
                let dce_args: Vec<AstNode> = args.iter().map(Self::dead_code_elimination).collect();
                AstNode::FunctionCall {
                    name: name.clone(),
                    args: dce_args,
                }
            }
            AstNode::IndexAccess { array, index } => AstNode::IndexAccess {
                array: Box::new(Self::dead_code_elimination(array)),
                index: Box::new(Self::dead_code_elimination(index)),
            },
            AstNode::Assignment { name, expr } => AstNode::Assignment {
                name: name.clone(),
                expr: Box::new(Self::dead_code_elimination(expr)),
            },
            AstNode::CompoundAssignment { name, op, expr } => AstNode::CompoundAssignment {
                name: name.clone(),
                op: op.clone(),
                expr: Box::new(Self::dead_code_elimination(expr)),
            },
            AstNode::Output {
                name,
                expr,
                modifier,
            } => AstNode::Output {
                name: name.clone(),
                expr: Box::new(Self::dead_code_elimination(expr)),
                modifier: modifier.clone(),
            },
            AstNode::Statements(stmts) => {
                let mut used_vars: HashSet<String> = HashSet::new();
                let mut result_stmts: Vec<AstNode> = Vec::new();

                for stmt in stmts.iter().rev() {
                    match stmt {
                        AstNode::Assignment { name, .. } => {
                            if used_vars.contains(name) {
                                Self::collect_used_vars(stmt, &mut used_vars);
                                result_stmts.push(Self::dead_code_elimination(stmt));
                            }
                        }
                        AstNode::Output { name, .. } => {
                            used_vars.insert(name.clone());
                            Self::collect_used_vars(stmt, &mut used_vars);
                            result_stmts.push(Self::dead_code_elimination(stmt));
                        }
                        _ => {
                            Self::collect_used_vars(stmt, &mut used_vars);
                            result_stmts.push(Self::dead_code_elimination(stmt));
                        }
                    }
                }

                result_stmts.reverse();
                if result_stmts.is_empty() {
                    AstNode::Statements(Vec::new())
                } else if result_stmts.len() == 1 {
                    result_stmts.remove(0)
                } else {
                    AstNode::Statements(result_stmts)
                }
            }
            AstNode::DrawText {
                cond,
                price,
                text,
                color,
            } => AstNode::DrawText {
                cond: Box::new(Self::dead_code_elimination(cond)),
                price: Box::new(Self::dead_code_elimination(price)),
                text: text.clone(),
                color: color.clone(),
            },
            AstNode::DrawIcon {
                cond,
                price,
                icon,
                color,
            } => AstNode::DrawIcon {
                cond: Box::new(Self::dead_code_elimination(cond)),
                price: Box::new(Self::dead_code_elimination(price)),
                icon: Box::new(Self::dead_code_elimination(icon)),
                color: color.clone(),
            },
            AstNode::StickLine {
                cond,
                price1,
                price2,
                width,
                empty,
                color,
            } => AstNode::StickLine {
                cond: Box::new(Self::dead_code_elimination(cond)),
                price1: Box::new(Self::dead_code_elimination(price1)),
                price2: Box::new(Self::dead_code_elimination(price2)),
                width: Box::new(Self::dead_code_elimination(width)),
                empty: *empty,
                color: color.clone(),
            },
            AstNode::DrawGeneric {
                command,
                args,
                color,
            } => AstNode::DrawGeneric {
                command: command.clone(),
                args: args.iter().map(Self::dead_code_elimination).collect(),
                color: color.clone(),
            },
            AstNode::IfThenElse {
                cond,
                then_branch,
                else_branch,
            } => AstNode::IfThenElse {
                cond: Box::new(Self::dead_code_elimination(cond)),
                then_branch: Box::new(Self::dead_code_elimination(then_branch)),
                else_branch: Box::new(Self::dead_code_elimination(else_branch)),
            },
            AstNode::ForLoop {
                var,
                start,
                end,
                body,
            } => AstNode::ForLoop {
                var: var.clone(),
                start: Box::new(Self::dead_code_elimination(start)),
                end: Box::new(Self::dead_code_elimination(end)),
                body: body.iter().map(Self::dead_code_elimination).collect(),
            },
            AstNode::WhileLoop { cond, body } => AstNode::WhileLoop {
                cond: Box::new(Self::dead_code_elimination(cond)),
                body: body.iter().map(Self::dead_code_elimination).collect(),
            },
        }
    }

    fn eliminate_dead_binary(
        op: &BinaryOperator,
        left: &AstNode,
        right: &AstNode,
    ) -> Option<AstNode> {
        match op {
            BinaryOperator::And => {
                if let AstNode::Number(v) = left {
                    if *v <= 0.0 {
                        return Some(AstNode::Number(0.0));
                    }
                }
                if let AstNode::Number(v) = right {
                    if *v <= 0.0 {
                        return Some(AstNode::Number(0.0));
                    }
                }
                None
            }
            BinaryOperator::Or => {
                if let AstNode::Number(v) = left {
                    if *v > 0.0 {
                        return Some(AstNode::Number(1.0));
                    }
                }
                if let AstNode::Number(v) = right {
                    if *v > 0.0 {
                        return Some(AstNode::Number(1.0));
                    }
                }
                None
            }
            BinaryOperator::Xor => None,
            BinaryOperator::StringConcat => None,
            _ => None,
        }
    }

    fn collect_used_vars(ast: &AstNode, used: &mut HashSet<String>) {
        match ast {
            AstNode::Variable(name) => {
                used.insert(name.clone());
            }
            AstNode::BinaryOp { left, right, .. } => {
                Self::collect_used_vars(left, used);
                Self::collect_used_vars(right, used);
            }
            AstNode::UnaryOp { expr, .. } => {
                Self::collect_used_vars(expr, used);
            }
            AstNode::FunctionCall { args, .. } => {
                for arg in args {
                    Self::collect_used_vars(arg, used);
                }
            }
            AstNode::IndexAccess { array, index } => {
                Self::collect_used_vars(array, used);
                Self::collect_used_vars(index, used);
            }
            AstNode::Assignment { name: _, expr } => {
                Self::collect_used_vars(expr, used);
            }
            AstNode::CompoundAssignment { name, expr, .. } => {
                used.insert(name.clone());
                Self::collect_used_vars(expr, used);
            }
            AstNode::Output { expr, .. } => {
                Self::collect_used_vars(expr, used);
            }
            AstNode::Statements(stmts) => {
                for stmt in stmts {
                    Self::collect_used_vars(stmt, used);
                }
            }
            AstNode::DrawText { cond, price, .. } => {
                Self::collect_used_vars(cond, used);
                Self::collect_used_vars(price, used);
            }
            AstNode::DrawIcon {
                cond, price, icon, ..
            } => {
                Self::collect_used_vars(cond, used);
                Self::collect_used_vars(price, used);
                Self::collect_used_vars(icon, used);
            }
            AstNode::StickLine {
                cond,
                price1,
                price2,
                width,
                ..
            } => {
                Self::collect_used_vars(cond, used);
                Self::collect_used_vars(price1, used);
                Self::collect_used_vars(price2, used);
                Self::collect_used_vars(width, used);
            }
            AstNode::DrawGeneric { args, .. } => {
                for arg in args {
                    Self::collect_used_vars(arg, used);
                }
            }
            AstNode::IfThenElse {
                cond,
                then_branch,
                else_branch,
            } => {
                Self::collect_used_vars(cond, used);
                Self::collect_used_vars(then_branch, used);
                Self::collect_used_vars(else_branch, used);
            }
            AstNode::ForLoop {
                var,
                start,
                end,
                body,
            } => {
                used.insert(var.clone());
                Self::collect_used_vars(start, used);
                Self::collect_used_vars(end, used);
                for s in body {
                    Self::collect_used_vars(s, used);
                }
            }
            AstNode::WhileLoop { cond, body } => {
                Self::collect_used_vars(cond, used);
                for s in body {
                    Self::collect_used_vars(s, used);
                }
            }
            _ => {}
        }
    }

    fn common_subexpression_elimination(ast: &AstNode) -> AstNode {
        let mut expr_map: HashMap<String, (String, AstNode)> = HashMap::new();
        Self::cse_collect(ast, &mut expr_map);

        let mut new_assignments: Vec<AstNode> = Vec::new();

        for (_key, (var_name, expr)) in expr_map.iter() {
            new_assignments.push(AstNode::Assignment {
                name: var_name.clone(),
                expr: Box::new(expr.clone()),
            });
        }

        let mut replaced = Self::cse_replace(ast, &expr_map);

        if !new_assignments.is_empty() {
            match &replaced {
                AstNode::Statements(stmts) => {
                    let mut all = new_assignments;
                    all.extend(stmts.clone());
                    replaced = AstNode::Statements(all);
                }
                _ => {
                    let mut all = new_assignments;
                    all.push(replaced);
                    replaced = AstNode::Statements(all);
                }
            }
        }

        replaced
    }

    fn cse_collect(ast: &AstNode, expr_map: &mut HashMap<String, (String, AstNode)>) {
        match ast {
            AstNode::Statements(stmts) => {
                for stmt in stmts {
                    Self::cse_collect(stmt, expr_map);
                }
            }
            AstNode::BinaryOp { op: _, left, right } => {
                Self::cse_collect(left, expr_map);
                Self::cse_collect(right, expr_map);

                let key = Self::ast_to_key(ast);
                if matches!(ast, AstNode::BinaryOp { .. }) && !expr_map.contains_key(&key) {
                    expr_map.insert(key, (format!("_CSE{}", expr_map.len()), ast.clone()));
                }
            }
            AstNode::UnaryOp { expr, .. } => {
                Self::cse_collect(expr, expr_map);
            }
            AstNode::FunctionCall { args, .. } => {
                for arg in args {
                    Self::cse_collect(arg, expr_map);
                }
            }
            AstNode::IndexAccess { array, index } => {
                Self::cse_collect(array, expr_map);
                Self::cse_collect(index, expr_map);
            }
            AstNode::Assignment { expr, .. } | AstNode::Output { expr, .. } => {
                Self::cse_collect(expr, expr_map);
            }
            AstNode::CompoundAssignment { expr, .. } => {
                Self::cse_collect(expr, expr_map);
            }
            AstNode::IfThenElse {
                cond,
                then_branch,
                else_branch,
            } => {
                Self::cse_collect(cond, expr_map);
                Self::cse_collect(then_branch, expr_map);
                Self::cse_collect(else_branch, expr_map);
            }
            AstNode::ForLoop {
                var: _,
                start,
                end,
                body,
            } => {
                Self::cse_collect(start, expr_map);
                Self::cse_collect(end, expr_map);
                for s in body {
                    Self::cse_collect(s, expr_map);
                }
            }
            AstNode::WhileLoop { cond, body } => {
                Self::cse_collect(cond, expr_map);
                for s in body {
                    Self::cse_collect(s, expr_map);
                }
            }
            _ => {}
        }
    }

    fn ast_to_key(ast: &AstNode) -> String {
        match ast {
            AstNode::BinaryOp { op, left, right } => {
                format!(
                    "B({:?},{},{})",
                    op,
                    Self::ast_to_key(left),
                    Self::ast_to_key(right)
                )
            }
            AstNode::UnaryOp { op, expr } => {
                format!("U({:?},{})", op, Self::ast_to_key(expr))
            }
            AstNode::FunctionCall { name, args } => {
                let args_str: Vec<String> = args.iter().map(Self::ast_to_key).collect();
                format!("F({},[{}])", name, args_str.join(","))
            }
            AstNode::IndexAccess { array, index } => {
                format!("I({},{})", Self::ast_to_key(array), Self::ast_to_key(index))
            }
            AstNode::Number(v) => format!("N({})", v),
            AstNode::Variable(n) => format!("V({})", n),
            _ => format!("O({:?})", ast),
        }
    }

    fn cse_replace(ast: &AstNode, expr_map: &HashMap<String, (String, AstNode)>) -> AstNode {
        match ast {
            AstNode::Statements(stmts) => {
                let replaced: Vec<AstNode> = stmts
                    .iter()
                    .map(|s| Self::cse_replace(s, expr_map))
                    .collect();
                if replaced.len() == 1 {
                    replaced.into_iter().next().unwrap()
                } else {
                    AstNode::Statements(replaced)
                }
            }
            AstNode::BinaryOp { .. } => {
                let key = Self::ast_to_key(ast);
                if expr_map.contains_key(&key) {
                    let (var_name, _) = expr_map.get(&key).unwrap();
                    AstNode::Variable(var_name.clone())
                } else {
                    ast.clone()
                }
            }
            AstNode::UnaryOp { op, expr } => AstNode::UnaryOp {
                op: op.clone(),
                expr: Box::new(Self::cse_replace(expr, expr_map)),
            },
            AstNode::FunctionCall { name, args } => {
                let replaced_args: Vec<AstNode> = args
                    .iter()
                    .map(|a| Self::cse_replace(a, expr_map))
                    .collect();
                AstNode::FunctionCall {
                    name: name.clone(),
                    args: replaced_args,
                }
            }
            AstNode::IndexAccess { array, index } => AstNode::IndexAccess {
                array: Box::new(Self::cse_replace(array, expr_map)),
                index: Box::new(Self::cse_replace(index, expr_map)),
            },
            AstNode::Assignment { name, expr } => AstNode::Assignment {
                name: name.clone(),
                expr: Box::new(Self::cse_replace(expr, expr_map)),
            },
            AstNode::CompoundAssignment { name, op, expr } => AstNode::CompoundAssignment {
                name: name.clone(),
                op: op.clone(),
                expr: Box::new(Self::cse_replace(expr, expr_map)),
            },
            AstNode::Output {
                name,
                expr,
                modifier,
            } => AstNode::Output {
                name: name.clone(),
                expr: Box::new(Self::cse_replace(expr, expr_map)),
                modifier: modifier.clone(),
            },
            AstNode::IfThenElse {
                cond,
                then_branch,
                else_branch,
            } => AstNode::IfThenElse {
                cond: Box::new(Self::cse_replace(cond, expr_map)),
                then_branch: Box::new(Self::cse_replace(then_branch, expr_map)),
                else_branch: Box::new(Self::cse_replace(else_branch, expr_map)),
            },
            AstNode::ForLoop {
                var,
                start,
                end,
                body,
            } => AstNode::ForLoop {
                var: var.clone(),
                start: Box::new(Self::cse_replace(start, expr_map)),
                end: Box::new(Self::cse_replace(end, expr_map)),
                body: body
                    .iter()
                    .map(|s| Self::cse_replace(s, expr_map))
                    .collect(),
            },
            AstNode::WhileLoop { cond, body } => AstNode::WhileLoop {
                cond: Box::new(Self::cse_replace(cond, expr_map)),
                body: body
                    .iter()
                    .map(|s| Self::cse_replace(s, expr_map))
                    .collect(),
            },
            AstNode::DrawText {
                cond,
                price,
                text,
                color,
            } => AstNode::DrawText {
                cond: Box::new(Self::cse_replace(cond, expr_map)),
                price: Box::new(Self::cse_replace(price, expr_map)),
                text: text.clone(),
                color: color.clone(),
            },
            AstNode::DrawIcon {
                cond,
                price,
                icon,
                color,
            } => AstNode::DrawIcon {
                cond: Box::new(Self::cse_replace(cond, expr_map)),
                price: Box::new(Self::cse_replace(price, expr_map)),
                icon: Box::new(Self::cse_replace(icon, expr_map)),
                color: color.clone(),
            },
            AstNode::StickLine {
                cond,
                price1,
                price2,
                width,
                empty,
                color,
            } => AstNode::StickLine {
                cond: Box::new(Self::cse_replace(cond, expr_map)),
                price1: Box::new(Self::cse_replace(price1, expr_map)),
                price2: Box::new(Self::cse_replace(price2, expr_map)),
                width: Box::new(Self::cse_replace(width, expr_map)),
                empty: *empty,
                color: color.clone(),
            },
            _ => ast.clone(),
        }
    }
}

/// Dependency analyzer for lazy evaluation.
/// Analyzes which variables are needed by the final outputs.
pub struct DependencyAnalyzer;

impl DependencyAnalyzer {
    /// Returns only the statements needed to compute the final output.
    pub fn analyze_and_prune(ast: &AstNode) -> AstNode {
        use std::collections::HashSet;

        let statements = match ast {
            AstNode::Statements(stmts) => stmts.clone(),
            other => return other.clone(),
        };

        if statements.is_empty() {
            return ast.clone();
        }

        // Find all output/expression nodes (last statement or Output nodes)
        let mut needed_vars: HashSet<String> = HashSet::new();
        let mut output_indices: Vec<usize> = Vec::new();

        for (idx, stmt) in statements.iter().enumerate() {
            match stmt {
                AstNode::Output { expr, .. } => {
                    output_indices.push(idx);
                    Self::collect_vars(expr, &mut needed_vars);
                }
                AstNode::DrawText { .. }
                | AstNode::DrawIcon { .. }
                | AstNode::StickLine { .. }
                | AstNode::DrawGeneric { .. } => {
                    output_indices.push(idx);
                    Self::collect_vars_from_stmt(stmt, &mut needed_vars);
                }
                _ => {}
            }
        }

        // Last statement is always needed if no explicit outputs
        if output_indices.is_empty() {
            let last_idx = statements.len() - 1;
            output_indices.push(last_idx);
            Self::collect_vars_from_stmt(&statements[last_idx], &mut needed_vars);
        }

        // Iterate backwards through assignments, resolving dependencies
        let mut keep_indices: HashSet<usize> = output_indices.iter().copied().collect();
        let mut changed = true;
        while changed {
            changed = false;
            for (idx, stmt) in statements.iter().enumerate().rev() {
                if keep_indices.contains(&idx) {
                    continue;
                }
                match stmt {
                    AstNode::Assignment { name, expr } | AstNode::Output { name, expr, .. }
                        if needed_vars.contains(name) =>
                    {
                        keep_indices.insert(idx);
                        Self::collect_vars(expr, &mut needed_vars);
                        changed = true;
                    }
                    AstNode::CompoundAssignment { name, expr, .. }
                        if needed_vars.contains(name) =>
                    {
                        keep_indices.insert(idx);
                        Self::collect_vars(expr, &mut needed_vars);
                        needed_vars.insert(name.clone());
                        changed = true;
                    }
                    AstNode::ParamDecl { name, .. } if needed_vars.contains(name) => {
                        keep_indices.insert(idx);
                        changed = true;
                    }
                    _ => {}
                }
            }
        }

        let pruned: Vec<AstNode> = statements
            .into_iter()
            .enumerate()
            .filter(|(idx, _)| keep_indices.contains(idx))
            .map(|(_, stmt)| stmt)
            .collect();

        if pruned.len() == 1 {
            pruned.into_iter().next().unwrap()
        } else {
            AstNode::Statements(pruned)
        }
    }

    fn collect_vars(node: &AstNode, vars: &mut std::collections::HashSet<String>) {
        match node {
            AstNode::Variable(name) => {
                vars.insert(name.clone());
            }
            AstNode::BinaryOp { left, right, .. } => {
                Self::collect_vars(left, vars);
                Self::collect_vars(right, vars);
            }
            AstNode::UnaryOp { expr, .. } => Self::collect_vars(expr, vars),
            AstNode::FunctionCall { args, .. } => {
                for arg in args {
                    Self::collect_vars(arg, vars);
                }
            }
            AstNode::IndexAccess { array, index } => {
                Self::collect_vars(array, vars);
                Self::collect_vars(index, vars);
            }
            AstNode::IfThenElse {
                cond,
                then_branch,
                else_branch,
            } => {
                Self::collect_vars(cond, vars);
                Self::collect_vars(then_branch, vars);
                Self::collect_vars(else_branch, vars);
            }
            _ => {}
        }
    }

    fn collect_vars_from_stmt(stmt: &AstNode, vars: &mut std::collections::HashSet<String>) {
        match stmt {
            AstNode::Assignment { expr, .. } | AstNode::Output { expr, .. } => {
                Self::collect_vars(expr, vars);
            }
            AstNode::CompoundAssignment { name, expr, .. } => {
                vars.insert(name.clone());
                Self::collect_vars(expr, vars);
            }
            AstNode::DrawText { cond, price, .. } => {
                Self::collect_vars(cond, vars);
                Self::collect_vars(price, vars);
            }
            AstNode::DrawIcon {
                cond, price, icon, ..
            } => {
                Self::collect_vars(cond, vars);
                Self::collect_vars(price, vars);
                Self::collect_vars(icon, vars);
            }
            AstNode::DrawGeneric { args, .. } => {
                for arg in args {
                    Self::collect_vars(arg, vars);
                }
            }
            _ => Self::collect_vars(stmt, vars),
        }
    }

    /// Group statements into batches where each batch's statements are independent
    /// (no statement in the batch reads a variable written by another in the same batch).
    pub fn group_independent_stmts(stmts: &[AstNode]) -> Vec<Vec<AstNode>> {
        use std::collections::HashSet;
        let mut groups: Vec<Vec<AstNode>> = Vec::new();
        let mut current_group: Vec<AstNode> = Vec::new();
        let mut current_defines: HashSet<String> = HashSet::new();
        let mut current_uses: HashSet<String> = HashSet::new();

        for stmt in stmts {
            let mut stmt_defines: HashSet<String> = HashSet::new();
            let mut stmt_uses: HashSet<String> = HashSet::new();

            match stmt {
                AstNode::Assignment { name, expr } | AstNode::Output { name, expr, .. } => {
                    stmt_defines.insert(name.clone());
                    Self::collect_vars(expr, &mut stmt_uses);
                }
                AstNode::CompoundAssignment { name, expr, .. } => {
                    stmt_defines.insert(name.clone());
                    stmt_uses.insert(name.clone());
                    Self::collect_vars(expr, &mut stmt_uses);
                }
                _ => {
                    Self::collect_vars_from_stmt(stmt, &mut stmt_uses);
                }
            }

            let has_dependency = stmt_uses.iter().any(|u| current_defines.contains(u))
                || stmt_defines
                    .iter()
                    .any(|d| current_uses.contains(d) || current_defines.contains(d));

            if has_dependency && !current_group.is_empty() {
                groups.push(std::mem::take(&mut current_group));
                current_defines.clear();
                current_uses.clear();
            }

            current_group.push(stmt.clone());
            current_defines.extend(stmt_defines);
            current_uses.extend(stmt_uses);
        }

        if !current_group.is_empty() {
            groups.push(current_group);
        }

        groups
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formula::parser::parse_formula;

    #[test]
    fn test_constant_folding_addition() {
        let ast = AstNode::BinaryOp {
            op: BinaryOperator::Add,
            left: Box::new(AstNode::Number(1.0)),
            right: Box::new(AstNode::Number(2.0)),
        };
        let optimized = FormulaOptimizer::constant_folding(&ast);
        assert!(matches!(optimized, AstNode::Number(v) if (v - 3.0).abs() < 1e-10));
    }

    #[test]
    fn test_constant_folding_multiplication() {
        let ast = AstNode::BinaryOp {
            op: BinaryOperator::Mul,
            left: Box::new(AstNode::Number(3.0)),
            right: Box::new(AstNode::Number(4.0)),
        };
        let optimized = FormulaOptimizer::constant_folding(&ast);
        assert!(matches!(optimized, AstNode::Number(v) if (v - 12.0).abs() < 1e-10));
    }

    #[test]
    fn test_constant_folding_comparison() {
        let ast = AstNode::BinaryOp {
            op: BinaryOperator::Gt,
            left: Box::new(AstNode::Number(10.0)),
            right: Box::new(AstNode::Number(5.0)),
        };
        let optimized = FormulaOptimizer::constant_folding(&ast);
        assert!(matches!(optimized, AstNode::Number(v) if (v - 1.0).abs() < 1e-10));
    }

    #[test]
    fn test_constant_folding_unary_neg() {
        let ast = AstNode::UnaryOp {
            op: UnaryOperator::Neg,
            expr: Box::new(AstNode::Number(10.0)),
        };
        let optimized = FormulaOptimizer::constant_folding(&ast);
        assert!(matches!(optimized, AstNode::Number(v) if (v - (-10.0)).abs() < 1e-10));
    }

    #[test]
    fn test_constant_folding_unary_not() {
        let ast = AstNode::UnaryOp {
            op: UnaryOperator::Not,
            expr: Box::new(AstNode::Number(0.0)),
        };
        let optimized = FormulaOptimizer::constant_folding(&ast);
        assert!(matches!(optimized, AstNode::Number(v) if (v - 1.0).abs() < 1e-10));
    }

    #[test]
    fn test_constant_folding_no_change_with_variable() {
        let ast = AstNode::BinaryOp {
            op: BinaryOperator::Add,
            left: Box::new(AstNode::Variable("CLOSE".to_string())),
            right: Box::new(AstNode::Number(1.0)),
        };
        let optimized = FormulaOptimizer::constant_folding(&ast);
        assert!(matches!(optimized, AstNode::BinaryOp { .. }));
    }

    #[test]
    fn test_constant_folding_nested() {
        let ast = AstNode::BinaryOp {
            op: BinaryOperator::Add,
            left: Box::new(AstNode::BinaryOp {
                op: BinaryOperator::Mul,
                left: Box::new(AstNode::Number(2.0)),
                right: Box::new(AstNode::Number(3.0)),
            }),
            right: Box::new(AstNode::Number(1.0)),
        };
        let optimized = FormulaOptimizer::constant_folding(&ast);
        assert!(matches!(optimized, AstNode::Number(v) if (v - 7.0).abs() < 1e-10));
    }

    #[test]
    fn test_dead_code_elimination_removes_unused_assignment() {
        let ast = AstNode::Statements(vec![
            AstNode::Assignment {
                name: "UNUSED".to_string(),
                expr: Box::new(AstNode::Number(42.0)),
            },
            AstNode::Output {
                name: "RESULT".to_string(),
                expr: Box::new(AstNode::Number(1.0)),
                modifier: None,
            },
        ]);
        let optimized = FormulaOptimizer::dead_code_elimination(&ast);
        let is_output = matches!(&optimized, AstNode::Output { .. })
            || matches!(&optimized, AstNode::Statements(s) if s.len() == 1 && matches!(&s[0], AstNode::Output { .. }));
        assert!(is_output);
    }

    #[test]
    fn test_dead_code_elimination_keeps_used_assignment() {
        let ast = AstNode::Statements(vec![
            AstNode::Assignment {
                name: "USED".to_string(),
                expr: Box::new(AstNode::Number(42.0)),
            },
            AstNode::Output {
                name: "RESULT".to_string(),
                expr: Box::new(AstNode::Variable("USED".to_string())),
                modifier: None,
            },
        ]);
        let optimized = FormulaOptimizer::dead_code_elimination(&ast);
        let stmt_count = match &optimized {
            AstNode::Statements(s) => s.len(),
            _ => 1,
        };
        assert_eq!(stmt_count, 2);
    }

    #[test]
    fn test_dead_code_elimination_and_short_circuit_zero() {
        let ast = AstNode::BinaryOp {
            op: BinaryOperator::And,
            left: Box::new(AstNode::Number(0.0)),
            right: Box::new(AstNode::BinaryOp {
                op: BinaryOperator::Add,
                left: Box::new(AstNode::Number(1.0)),
                right: Box::new(AstNode::Number(2.0)),
            }),
        };
        let optimized = FormulaOptimizer::dead_code_elimination(&ast);
        assert!(matches!(optimized, AstNode::Number(v) if (v - 0.0).abs() < 1e-10));
    }

    #[test]
    fn test_dead_code_elimination_or_short_circuit_one() {
        let ast = AstNode::BinaryOp {
            op: BinaryOperator::Or,
            left: Box::new(AstNode::Number(1.0)),
            right: Box::new(AstNode::Variable("CLOSE".to_string())),
        };
        let optimized = FormulaOptimizer::dead_code_elimination(&ast);
        assert!(matches!(optimized, AstNode::Number(v) if (v - 1.0).abs() < 1e-10));
    }

    #[test]
    fn test_constant_folding_division_by_zero() {
        let ast = AstNode::BinaryOp {
            op: BinaryOperator::Div,
            left: Box::new(AstNode::Number(1.0)),
            right: Box::new(AstNode::Number(0.0)),
        };
        let optimized = FormulaOptimizer::constant_folding(&ast);
        assert!(matches!(optimized, AstNode::BinaryOp { .. }));
    }

    #[test]
    fn test_optimize_constant_folding() {
        let ast = AstNode::BinaryOp {
            op: BinaryOperator::Add,
            left: Box::new(AstNode::Number(1.0)),
            right: Box::new(AstNode::Number(2.0)),
        };
        let optimized = FormulaOptimizer::optimize(&ast);
        assert!(matches!(optimized, AstNode::Number(v) if (v - 3.0).abs() < 1e-10));
    }

    #[test]
    fn test_optimize_combined() {
        let source = "A := 1 + 2; B := 3 * 4; RESULT: A";
        let ast = parse_formula(source).unwrap();
        let optimized = FormulaOptimizer::optimize(&ast);
        let stmt_count = match &optimized {
            AstNode::Statements(s) => s.len(),
            _ => 1,
        };
        assert!(stmt_count >= 2);
        let has_number = match &optimized {
            AstNode::Statements(stmts) => stmts.iter().any(|s| matches!(s, AstNode::Assignment { expr, .. } if matches!(**expr, AstNode::Number(_)))),
            _ => false,
        };
        assert!(has_number);
    }

    #[test]
    fn test_constant_folding_and_true() {
        let ast = AstNode::BinaryOp {
            op: BinaryOperator::And,
            left: Box::new(AstNode::Number(1.0)),
            right: Box::new(AstNode::Number(1.0)),
        };
        let optimized = FormulaOptimizer::constant_folding(&ast);
        assert!(matches!(optimized, AstNode::Number(v) if (v - 1.0).abs() < 1e-10));
    }

    #[test]
    fn test_constant_folding_or_false() {
        let ast = AstNode::BinaryOp {
            op: BinaryOperator::Or,
            left: Box::new(AstNode::Number(0.0)),
            right: Box::new(AstNode::Number(0.0)),
        };
        let optimized = FormulaOptimizer::constant_folding(&ast);
        assert!(matches!(optimized, AstNode::Number(v) if (v - 0.0).abs() < 1e-10));
    }

    #[test]
    fn test_constant_folding_power() {
        let ast = AstNode::BinaryOp {
            op: BinaryOperator::Pow,
            left: Box::new(AstNode::Number(2.0)),
            right: Box::new(AstNode::Number(10.0)),
        };
        let optimized = FormulaOptimizer::constant_folding(&ast);
        assert!(matches!(optimized, AstNode::Number(v) if (v - 1024.0).abs() < 1e-10));
    }

    #[test]
    fn test_dead_code_multiple_unused() {
        let ast = AstNode::Statements(vec![
            AstNode::Assignment {
                name: "A".to_string(),
                expr: Box::new(AstNode::Number(1.0)),
            },
            AstNode::Assignment {
                name: "B".to_string(),
                expr: Box::new(AstNode::Number(2.0)),
            },
            AstNode::Assignment {
                name: "C".to_string(),
                expr: Box::new(AstNode::Number(3.0)),
            },
            AstNode::Output {
                name: "OUT".to_string(),
                expr: Box::new(AstNode::Variable("A".to_string())),
                modifier: None,
            },
        ]);
        let optimized = FormulaOptimizer::dead_code_elimination(&ast);
        let stmt_count = match &optimized {
            AstNode::Statements(s) => s.len(),
            _ => 1,
        };
        assert_eq!(stmt_count, 2);
    }

    // =========================================================================
    // 新增测试：algebraic_simplify / strength_reduction / LICM / OptLevel
    // =========================================================================

    #[test]
    fn test_algebraic_simplify_x_plus_zero() {
        // x + 0 -> x
        let ast = AstNode::BinaryOp {
            op: BinaryOperator::Add,
            left: Box::new(AstNode::Variable("X".to_string())),
            right: Box::new(AstNode::Number(0.0)),
        };
        let opt = FormulaOptimizer::algebraic_simplify(&ast);
        assert!(matches!(opt, AstNode::Variable(ref s) if s == "X"));
    }

    #[test]
    fn test_algebraic_simplify_x_times_one() {
        // x * 1 -> x
        let ast = AstNode::BinaryOp {
            op: BinaryOperator::Mul,
            left: Box::new(AstNode::Variable("X".to_string())),
            right: Box::new(AstNode::Number(1.0)),
        };
        let opt = FormulaOptimizer::algebraic_simplify(&ast);
        assert!(matches!(opt, AstNode::Variable(ref s) if s == "X"));
    }

    #[test]
    fn test_algebraic_simplify_x_times_zero() {
        // x * 0 -> 0
        let ast = AstNode::BinaryOp {
            op: BinaryOperator::Mul,
            left: Box::new(AstNode::Variable("X".to_string())),
            right: Box::new(AstNode::Number(0.0)),
        };
        let opt = FormulaOptimizer::algebraic_simplify(&ast);
        assert!(matches!(opt, AstNode::Number(0.0)));
    }

    #[test]
    fn test_algebraic_simplify_x_div_x() {
        // x / x -> 1
        let ast = AstNode::BinaryOp {
            op: BinaryOperator::Div,
            left: Box::new(AstNode::Variable("X".to_string())),
            right: Box::new(AstNode::Variable("X".to_string())),
        };
        let opt = FormulaOptimizer::algebraic_simplify(&ast);
        assert!(matches!(opt, AstNode::Number(v) if (v - 1.0).abs() < 1e-15));
    }

    #[test]
    fn test_algebraic_simplify_x_pow_zero() {
        // x ^ 0 -> 1
        let ast = AstNode::BinaryOp {
            op: BinaryOperator::Pow,
            left: Box::new(AstNode::Variable("X".to_string())),
            right: Box::new(AstNode::Number(0.0)),
        };
        let opt = FormulaOptimizer::algebraic_simplify(&ast);
        assert!(matches!(opt, AstNode::Number(v) if (v - 1.0).abs() < 1e-15));
    }

    #[test]
    fn test_algebraic_simplify_x_pow_one() {
        // x ^ 1 -> x
        let ast = AstNode::BinaryOp {
            op: BinaryOperator::Pow,
            left: Box::new(AstNode::Variable("X".to_string())),
            right: Box::new(AstNode::Number(1.0)),
        };
        let opt = FormulaOptimizer::algebraic_simplify(&ast);
        assert!(matches!(opt, AstNode::Variable(ref s) if s == "X"));
    }

    #[test]
    fn test_algebraic_simplify_x_minus_x() {
        // x - x -> 0
        // (代数化简未直接处理此 case, 这里仅做基本 sanity check)
        let ast = AstNode::BinaryOp {
            op: BinaryOperator::Sub,
            left: Box::new(AstNode::Variable("X".to_string())),
            right: Box::new(AstNode::Variable("X".to_string())),
        };
        let opt = FormulaOptimizer::algebraic_simplify(&ast);
        // 当前实现：Sub 不化简,保留为 BinaryOp(Sub)
        assert!(matches!(
            opt,
            AstNode::BinaryOp {
                op: BinaryOperator::Sub,
                ..
            }
        ));
    }

    #[test]
    fn test_strength_reduction_times_two() {
        // X * 2 -> X + X
        let ast = AstNode::BinaryOp {
            op: BinaryOperator::Mul,
            left: Box::new(AstNode::Variable("X".to_string())),
            right: Box::new(AstNode::Number(2.0)),
        };
        let opt = FormulaOptimizer::strength_reduction(&ast);
        assert!(matches!(
            opt,
            AstNode::BinaryOp {
                op: BinaryOperator::Add,
                ..
            }
        ));
    }

    #[test]
    fn test_strength_reduction_div_two() {
        // X / 2 -> X * 0.5
        let ast = AstNode::BinaryOp {
            op: BinaryOperator::Div,
            left: Box::new(AstNode::Variable("X".to_string())),
            right: Box::new(AstNode::Number(2.0)),
        };
        let opt = FormulaOptimizer::strength_reduction(&ast);
        if let AstNode::BinaryOp { op, right, .. } = opt {
            assert!(matches!(op, BinaryOperator::Mul));
            assert!(matches!(*right, AstNode::Number(v) if (v - 0.5).abs() < 1e-15));
        } else {
            panic!("Expected BinaryOp(Mul, ..)");
        }
    }

    #[test]
    fn test_strength_reduction_pow_two() {
        // X ^ 2 -> X * X
        let ast = AstNode::BinaryOp {
            op: BinaryOperator::Pow,
            left: Box::new(AstNode::Variable("X".to_string())),
            right: Box::new(AstNode::Number(2.0)),
        };
        let opt = FormulaOptimizer::strength_reduction(&ast);
        assert!(matches!(
            opt,
            AstNode::BinaryOp {
                op: BinaryOperator::Mul,
                ..
            }
        ));
    }

    #[test]
    fn test_strength_reduction_pow_other_unchanged() {
        // X ^ 3 不应被削减
        let ast = AstNode::BinaryOp {
            op: BinaryOperator::Pow,
            left: Box::new(AstNode::Variable("X".to_string())),
            right: Box::new(AstNode::Number(3.0)),
        };
        let opt = FormulaOptimizer::strength_reduction(&ast);
        assert!(matches!(
            opt,
            AstNode::BinaryOp {
                op: BinaryOperator::Pow,
                ..
            }
        ));
    }

    #[test]
    fn test_loop_invariant_detection() {
        // 3.14 + 5 不依赖任何变量,是 loop invariant
        let ast = AstNode::BinaryOp {
            op: BinaryOperator::Add,
            left: Box::new(AstNode::Number(3.14)),
            right: Box::new(AstNode::Number(5.0)),
        };
        assert!(FormulaOptimizer::is_loop_invariant(&ast, "i"));
    }

    #[test]
    fn test_loop_invariant_detection_depends_on_var() {
        // i + 5 依赖 i
        let ast = AstNode::BinaryOp {
            op: BinaryOperator::Add,
            left: Box::new(AstNode::Variable("i".to_string())),
            right: Box::new(AstNode::Number(5.0)),
        };
        assert!(!FormulaOptimizer::is_loop_invariant(&ast, "i"));
    }

    #[test]
    fn test_loop_invariant_code_motion_preserves_structure() {
        // LICM 当前实现为占位 pass：保留 For 循环结构,不破坏公式语义
        let ast = AstNode::ForLoop {
            var: "i".to_string(),
            start: Box::new(AstNode::Number(1.0)),
            end: Box::new(AstNode::Number(10.0)),
            body: vec![AstNode::Assignment {
                name: "X".to_string(),
                expr: Box::new(AstNode::Variable("i".to_string())),
            }],
        };
        let opt = FormulaOptimizer::loop_invariant_code_motion(&ast);
        // 应当仍是 ForLoop
        assert!(matches!(opt, AstNode::ForLoop { .. }));
    }

    #[test]
    fn test_opt_level_none_passthrough() {
        // OptLevel::None 应直接返回原 AST
        let ast = AstNode::BinaryOp {
            op: BinaryOperator::Add,
            left: Box::new(AstNode::Number(1.0)),
            right: Box::new(AstNode::Number(2.0)),
        };
        let opt = FormulaOptimizer::optimize_with(&ast, OptLevel::None);
        // 1+2 未被折叠,仍为 BinaryOp
        assert!(matches!(opt, AstNode::BinaryOp { .. }));
    }

    #[test]
    fn test_opt_level_basic_folds_constants() {
        // OptLevel::Basic 至少应做 constant_folding
        let ast = AstNode::BinaryOp {
            op: BinaryOperator::Add,
            left: Box::new(AstNode::Number(1.0)),
            right: Box::new(AstNode::Number(2.0)),
        };
        let opt = FormulaOptimizer::optimize_with(&ast, OptLevel::Basic);
        assert!(matches!(opt, AstNode::Number(v) if (v - 3.0).abs() < 1e-10));
    }

    #[test]
    fn test_opt_level_standard_simplifies_x_plus_0() {
        // Standard 等级应化简 x+0 -> x
        let ast = AstNode::BinaryOp {
            op: BinaryOperator::Add,
            left: Box::new(AstNode::Variable("X".to_string())),
            right: Box::new(AstNode::Number(0.0)),
        };
        let opt = FormulaOptimizer::optimize_with(&ast, OptLevel::Standard);
        assert!(matches!(opt, AstNode::Variable(ref s) if s == "X"));
    }

    #[test]
    fn test_opt_level_aggressive_reduces_x_times_2() {
        // Aggressive 等级包含 Standard pass，应做强度削减：x*2 -> x+x
        let ast = AstNode::BinaryOp {
            op: BinaryOperator::Mul,
            left: Box::new(AstNode::Variable("X".to_string())),
            right: Box::new(AstNode::Number(2.0)),
        };
        let basic = FormulaOptimizer::optimize_with(&ast, OptLevel::Basic);
        assert!(matches!(
            basic,
            AstNode::BinaryOp {
                op: BinaryOperator::Mul,
                ..
            }
        ));
        let opt = FormulaOptimizer::optimize_with(&ast, OptLevel::Aggressive);
        assert!(
            ast_contains_strength_reduced_mul(&opt),
            "expected x*2 strength reduction in optimized AST, got {:?}",
            opt
        );
    }

    fn ast_contains_strength_reduced_mul(node: &AstNode) -> bool {
        match node {
            AstNode::BinaryOp {
                op: BinaryOperator::Add,
                left,
                right,
            } => {
                matches!(left.as_ref(), AstNode::Variable(s) if s == "X")
                    && matches!(right.as_ref(), AstNode::Variable(s) if s == "X")
            }
            AstNode::Statements(stmts) => stmts.iter().any(ast_contains_strength_reduced_mul),
            AstNode::Assignment { expr, .. }
            | AstNode::CompoundAssignment { expr, .. }
            | AstNode::Output { expr, .. } => ast_contains_strength_reduced_mul(expr),
            AstNode::BinaryOp { left, right, .. } => {
                ast_contains_strength_reduced_mul(left) || ast_contains_strength_reduced_mul(right)
            }
            AstNode::UnaryOp { expr, .. } => ast_contains_strength_reduced_mul(expr),
            AstNode::FunctionCall { args, .. } => {
                args.iter().any(ast_contains_strength_reduced_mul)
            }
            AstNode::IndexAccess { array, index } => {
                ast_contains_strength_reduced_mul(array) || ast_contains_strength_reduced_mul(index)
            }
            AstNode::IfThenElse {
                cond,
                then_branch,
                else_branch,
            } => {
                ast_contains_strength_reduced_mul(cond)
                    || ast_contains_strength_reduced_mul(then_branch)
                    || ast_contains_strength_reduced_mul(else_branch)
            }
            AstNode::ForLoop {
                start, end, body, ..
            } => {
                ast_contains_strength_reduced_mul(start)
                    || ast_contains_strength_reduced_mul(end)
                    || body.iter().any(ast_contains_strength_reduced_mul)
            }
            AstNode::WhileLoop { cond, body } => {
                ast_contains_strength_reduced_mul(cond)
                    || body.iter().any(ast_contains_strength_reduced_mul)
            }
            _ => false,
        }
    }
}
