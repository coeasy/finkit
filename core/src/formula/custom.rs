//! Registered, composable formula components.
//!
//! A custom component is an expression-level macro with named parameters. It
//! is expanded into the canonical AST before planning, optimization and
//! execution, so custom indicators keep the same semantics and fast paths as
//! built-in formulas. Definitions are deliberately expression-only: this
//! prevents a component from silently leaking assignments, drawing commands,
//! or control-flow state into its caller.

use super::ast::AstNode;
use super::functions::get_builtin_functions;
use super::parser::parse_formula;
use std::collections::{HashMap, HashSet};

const DEFAULT_MAX_DEPTH: usize = 32;
const DEFAULT_MAX_NODES: usize = 100_000;

/// A parameterized, expression-only formula component.
#[derive(Debug, Clone)]
pub struct CustomFormula {
    name: String,
    parameters: Vec<String>,
    body: AstNode,
    source: String,
}

impl CustomFormula {
    /// Canonical uppercase name used by the registry.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Parameter names in declaration order.
    pub fn parameters(&self) -> &[String] {
        &self.parameters
    }

    /// Original expression source used to define the component.
    pub fn source(&self) -> &str {
        &self.source
    }
}

/// Registry for reusable formula components.
///
/// The registry is intentionally separate from the built-in function map:
/// built-ins cannot be shadowed, while custom names can be added and removed
/// at runtime. Registration validates the body and all expansion limits before
/// a component becomes visible to the formula engine.
#[derive(Debug, Clone)]
pub struct FormulaRegistry {
    definitions: HashMap<String, CustomFormula>,
    max_depth: usize,
    max_nodes: usize,
}

impl Default for FormulaRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl FormulaRegistry {
    /// Create an empty registry with conservative expansion limits.
    pub fn new() -> Self {
        Self {
            definitions: HashMap::new(),
            max_depth: DEFAULT_MAX_DEPTH,
            max_nodes: DEFAULT_MAX_NODES,
        }
    }

    /// Set the maximum nested component depth and return the updated registry.
    pub fn with_max_depth(mut self, max_depth: usize) -> Self {
        self.max_depth = max_depth.max(1);
        self
    }

    /// Set the maximum expanded AST node count and return the updated registry.
    pub fn with_max_nodes(mut self, max_nodes: usize) -> Self {
        self.max_nodes = max_nodes.max(1);
        self
    }

    /// Register an expression such as `RSI(X, N) + EMA(X, M)`.
    pub fn register(
        &mut self,
        name: &str,
        parameters: &[&str],
        source: &str,
    ) -> Result<(), String> {
        let canonical = normalize_identifier(name)?;
        if get_builtin_functions().contains_key(&canonical) {
            return Err(format!(
                "cannot shadow built-in formula function `{canonical}`"
            ));
        }
        if parameters.is_empty() {
            return Err(format!(
                "custom formula `{canonical}` requires at least one parameter"
            ));
        }

        let mut canonical_parameters = Vec::with_capacity(parameters.len());
        let mut seen = HashSet::new();
        for parameter in parameters {
            let parameter = normalize_identifier(parameter)?;
            if !seen.insert(parameter.clone()) {
                return Err(format!("duplicate custom formula parameter `{parameter}`"));
            }
            canonical_parameters.push(parameter);
        }

        let body = parse_formula(source)
            .map_err(|error| format!("custom formula `{canonical}` body parse error: {error}"))?;
        if !is_expression_only(&body) {
            return Err(format!(
                "custom formula `{canonical}` must contain one expression without assignments, outputs, drawing, or loops"
            ));
        }
        if contains_call(&body, &canonical) {
            return Err(format!("custom formula `{canonical}` cannot call itself"));
        }

        self.definitions.insert(
            canonical.clone(),
            CustomFormula {
                name: canonical,
                parameters: canonical_parameters,
                body,
                source: source.to_string(),
            },
        );
        Ok(())
    }

    /// Remove a registered component, returning whether it existed.
    pub fn unregister(&mut self, name: &str) -> Result<bool, String> {
        let name = normalize_identifier(name)?;
        Ok(self.definitions.remove(&name).is_some())
    }

    /// Remove all registered components.
    pub fn clear(&mut self) {
        self.definitions.clear();
    }

    /// Return a definition by case-insensitive name.
    pub fn get(&self, name: &str) -> Option<&CustomFormula> {
        self.definitions.get(&name.to_ascii_uppercase())
    }

    /// Return component names in deterministic order.
    pub fn names(&self) -> Vec<String> {
        let mut names: Vec<_> = self.definitions.keys().cloned().collect();
        names.sort();
        names
    }

    /// Expand all registered calls in an AST.
    pub fn expand(&self, ast: &AstNode) -> Result<AstNode, String> {
        let mut nodes = 0;
        self.expand_node(ast, &HashMap::new(), 0, &mut nodes)
    }

    fn expand_node(
        &self,
        node: &AstNode,
        substitutions: &HashMap<String, AstNode>,
        depth: usize,
        nodes: &mut usize,
    ) -> Result<AstNode, String> {
        *nodes = nodes.saturating_add(1);
        if *nodes > self.max_nodes {
            return Err(format!(
                "custom formula expansion exceeded {} AST nodes",
                self.max_nodes
            ));
        }
        if depth > self.max_depth {
            return Err(format!(
                "custom formula expansion exceeded depth {}",
                self.max_depth
            ));
        }

        if let AstNode::Variable(name) = node {
            if let Some(value) = substitutions.get(&name.to_ascii_uppercase()) {
                return Ok(value.clone());
            }
            return Ok(node.clone());
        }

        match node {
            AstNode::FunctionCall { name, args } => {
                let expanded_args: Vec<_> = args
                    .iter()
                    .map(|arg| self.expand_node(arg, substitutions, depth, nodes))
                    .collect::<Result<_, _>>()?;
                let canonical = name.to_ascii_uppercase();
                let Some(definition) = self.definitions.get(&canonical) else {
                    return Ok(AstNode::FunctionCall {
                        name: name.clone(),
                        args: expanded_args,
                    });
                };
                if expanded_args.len() != definition.parameters.len() {
                    return Err(format!(
                        "custom formula `{canonical}` expects {} arguments, got {}",
                        definition.parameters.len(),
                        expanded_args.len()
                    ));
                }
                let local: HashMap<_, _> = definition
                    .parameters
                    .iter()
                    .cloned()
                    .zip(expanded_args)
                    .collect();
                self.expand_node(&definition.body, &local, depth + 1, nodes)
            }
            AstNode::Number(_) | AstNode::StringLit(_) | AstNode::Variable(_) => Ok(node.clone()),
            AstNode::BinaryOp { op, left, right } => Ok(AstNode::BinaryOp {
                op: op.clone(),
                left: Box::new(self.expand_node(left, substitutions, depth, nodes)?),
                right: Box::new(self.expand_node(right, substitutions, depth, nodes)?),
            }),
            AstNode::UnaryOp { op, expr } => Ok(AstNode::UnaryOp {
                op: op.clone(),
                expr: Box::new(self.expand_node(expr, substitutions, depth, nodes)?),
            }),
            AstNode::IndexAccess { array, index } => Ok(AstNode::IndexAccess {
                array: Box::new(self.expand_node(array, substitutions, depth, nodes)?),
                index: Box::new(self.expand_node(index, substitutions, depth, nodes)?),
            }),
            AstNode::Assignment { name, expr } => Ok(AstNode::Assignment {
                name: name.clone(),
                expr: Box::new(self.expand_node(expr, substitutions, depth, nodes)?),
            }),
            AstNode::CompoundAssignment { name, op, expr } => Ok(AstNode::CompoundAssignment {
                name: name.clone(),
                op: op.clone(),
                expr: Box::new(self.expand_node(expr, substitutions, depth, nodes)?),
            }),
            AstNode::Output {
                name,
                expr,
                modifier,
            } => Ok(AstNode::Output {
                name: name.clone(),
                expr: Box::new(self.expand_node(expr, substitutions, depth, nodes)?),
                modifier: modifier.clone(),
            }),
            AstNode::Statements(statements) => Ok(AstNode::Statements(
                statements
                    .iter()
                    .map(|statement| self.expand_node(statement, substitutions, depth, nodes))
                    .collect::<Result<_, _>>()?,
            )),
            AstNode::ParamDecl {
                name,
                min,
                max,
                default,
            } => Ok(AstNode::ParamDecl {
                name: name.clone(),
                min: *min,
                max: *max,
                default: *default,
            }),
            AstNode::DrawText {
                cond,
                price,
                text,
                color,
            } => Ok(AstNode::DrawText {
                cond: Box::new(self.expand_node(cond, substitutions, depth, nodes)?),
                price: Box::new(self.expand_node(price, substitutions, depth, nodes)?),
                text: text.clone(),
                color: color.clone(),
            }),
            AstNode::DrawIcon {
                cond,
                price,
                icon,
                color,
            } => Ok(AstNode::DrawIcon {
                cond: Box::new(self.expand_node(cond, substitutions, depth, nodes)?),
                price: Box::new(self.expand_node(price, substitutions, depth, nodes)?),
                icon: Box::new(self.expand_node(icon, substitutions, depth, nodes)?),
                color: color.clone(),
            }),
            AstNode::StickLine {
                cond,
                price1,
                price2,
                width,
                empty,
                color,
            } => Ok(AstNode::StickLine {
                cond: Box::new(self.expand_node(cond, substitutions, depth, nodes)?),
                price1: Box::new(self.expand_node(price1, substitutions, depth, nodes)?),
                price2: Box::new(self.expand_node(price2, substitutions, depth, nodes)?),
                width: Box::new(self.expand_node(width, substitutions, depth, nodes)?),
                empty: *empty,
                color: color.clone(),
            }),
            AstNode::DrawGeneric {
                command,
                args,
                color,
            } => Ok(AstNode::DrawGeneric {
                command: command.clone(),
                args: args
                    .iter()
                    .map(|arg| self.expand_node(arg, substitutions, depth, nodes))
                    .collect::<Result<_, _>>()?,
                color: color.clone(),
            }),
            AstNode::IfThenElse {
                cond,
                then_branch,
                else_branch,
            } => Ok(AstNode::IfThenElse {
                cond: Box::new(self.expand_node(cond, substitutions, depth, nodes)?),
                then_branch: Box::new(self.expand_node(
                    then_branch,
                    substitutions,
                    depth,
                    nodes,
                )?),
                else_branch: Box::new(self.expand_node(
                    else_branch,
                    substitutions,
                    depth,
                    nodes,
                )?),
            }),
            AstNode::ForLoop {
                var,
                start,
                end,
                body,
            } => Ok(AstNode::ForLoop {
                var: var.clone(),
                start: Box::new(self.expand_node(start, substitutions, depth, nodes)?),
                end: Box::new(self.expand_node(end, substitutions, depth, nodes)?),
                body: body
                    .iter()
                    .map(|item| self.expand_node(item, substitutions, depth, nodes))
                    .collect::<Result<_, _>>()?,
            }),
            AstNode::WhileLoop { cond, body } => Ok(AstNode::WhileLoop {
                cond: Box::new(self.expand_node(cond, substitutions, depth, nodes)?),
                body: body
                    .iter()
                    .map(|item| self.expand_node(item, substitutions, depth, nodes))
                    .collect::<Result<_, _>>()?,
            }),
        }
    }
}

fn normalize_identifier(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty()
        || !value.chars().enumerate().all(|(index, ch)| {
            ch == '_' || ch.is_ascii_alphanumeric() && (index > 0 || ch.is_ascii_alphabetic())
        })
    {
        return Err(format!("invalid custom formula identifier `{value}`"));
    }
    Ok(value.to_ascii_uppercase())
}

fn is_expression_only(node: &AstNode) -> bool {
    match node {
        AstNode::Number(_) | AstNode::StringLit(_) | AstNode::Variable(_) => true,
        AstNode::BinaryOp { left, right, .. } => {
            is_expression_only(left) && is_expression_only(right)
        }
        AstNode::UnaryOp { expr, .. } => is_expression_only(expr),
        AstNode::FunctionCall { args, .. } => args.iter().all(is_expression_only),
        AstNode::IndexAccess { array, index } => {
            is_expression_only(array) && is_expression_only(index)
        }
        _ => false,
    }
}

fn contains_call(node: &AstNode, name: &str) -> bool {
    match node {
        AstNode::FunctionCall { name: call, args } => {
            call.eq_ignore_ascii_case(name) || args.iter().any(|arg| contains_call(arg, name))
        }
        AstNode::BinaryOp { left, right, .. } => {
            contains_call(left, name) || contains_call(right, name)
        }
        AstNode::UnaryOp { expr, .. } => contains_call(expr, name),
        AstNode::IndexAccess { array, index } => {
            contains_call(array, name) || contains_call(index, name)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formula::parse_formula;

    #[test]
    fn expands_nested_parameterized_components_case_insensitively() {
        let mut registry = FormulaRegistry::new();
        registry
            .register("zscore_ma", &["X", "N"], "(X - MA(X, N)) / STD(X, N)")
            .unwrap();
        registry
            .register("signal", &["PRICE"], "CROSS(PRICE, ZSCORE_MA(PRICE, 5))")
            .unwrap();
        let ast = registry
            .expand(&parse_formula("signal(CLOSE)").unwrap())
            .unwrap();
        let text = format!("{ast:?}");
        assert!(text.contains("CROSS"));
        assert!(!text.contains("signal"));
        assert!(!text.contains("ZSCORE_MA"));
    }

    #[test]
    fn rejects_shadowing_and_side_effect_bodies() {
        let mut registry = FormulaRegistry::new();
        assert!(registry.register("MA", &["X"], "X").is_err());
        assert!(registry.register("BAD", &["X"], "Y := X").is_err());
    }

    #[test]
    fn rejects_wrong_arity_and_expansion_blowups() {
        let mut registry = FormulaRegistry::new().with_max_nodes(8);
        registry.register("PAIR", &["X", "Y"], "X + Y").unwrap();
        assert!(registry
            .expand(&parse_formula("PAIR(CLOSE)").unwrap())
            .is_err());
        registry
            .register("DEEP", &["X"], "PAIR(X, X) + PAIR(X, X)")
            .unwrap();
        assert!(registry
            .expand(&parse_formula("DEEP(DEEP(CLOSE))").unwrap())
            .is_err());
    }
}
