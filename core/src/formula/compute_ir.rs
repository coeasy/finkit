//! Semantic lowering from formula AST nodes into the unified compute plan.
//!
//! The formula AST describes syntax. This module adds planner-visible data
//! dependencies and observable effects so optimizers can reason about formula
//! semantics without guessing whether an assignment, output, drawing command,
//! or context mutation is safe to remove.

use super::ast::AstNode;
use crate::compute::{
    ComputeCapabilities, ComputeEffect, ComputeNode, ComputeNodeId, ComputePlan, ComputePlanError,
    LookbackRequirement,
};
use crate::registry::{builtin_function_registry, FunctionRegistry};
use std::collections::BTreeMap;

/// Validated semantic compute plan derived from one formula AST.
#[derive(Debug, Clone)]
pub struct FormulaComputePlan {
    plan: ComputePlan,
    root: ComputeNodeId,
}

impl FormulaComputePlan {
    /// Lower an AST using the canonical built-in function registry.
    pub fn compile(ast: &AstNode) -> Result<Self, ComputePlanError> {
        let registry = builtin_function_registry();
        Self::compile_with_registry(ast, &registry)
    }

    /// Lower an AST using an explicit function registry.
    pub fn compile_with_registry(
        ast: &AstNode,
        registry: &FunctionRegistry,
    ) -> Result<Self, ComputePlanError> {
        let mut lowerer = FormulaLowerer::new(registry);
        let root = lowerer.lower(ast);
        let plan = ComputePlan::compile(lowerer.nodes)?;
        Ok(Self { plan, root })
    }

    /// Unified compute plan containing dependencies and effects.
    pub const fn plan(&self) -> &ComputePlan {
        &self.plan
    }

    /// Node representing the formula's final value.
    pub const fn root(&self) -> ComputeNodeId {
        self.root
    }
}

/// Convenience wrapper around [`FormulaComputePlan::compile`].
pub fn lower_formula_ast(ast: &AstNode) -> Result<FormulaComputePlan, ComputePlanError> {
    FormulaComputePlan::compile(ast)
}

/// Lower a formula AST with an explicit canonical function registry.
pub fn lower_formula_ast_with_registry(
    ast: &AstNode,
    registry: &FunctionRegistry,
) -> Result<FormulaComputePlan, ComputePlanError> {
    FormulaComputePlan::compile_with_registry(ast, registry)
}

struct FormulaLowerer<'a> {
    registry: &'a FunctionRegistry,
    nodes: Vec<ComputeNode>,
    next_id: usize,
    last_write: BTreeMap<String, ComputeNodeId>,
    last_effect: Option<ComputeNodeId>,
}

impl<'a> FormulaLowerer<'a> {
    fn new(registry: &'a FunctionRegistry) -> Self {
        Self {
            registry,
            nodes: Vec::new(),
            next_id: 0,
            last_write: BTreeMap::new(),
            last_effect: None,
        }
    }

    fn lower(&mut self, ast: &AstNode) -> ComputeNodeId {
        match ast {
            AstNode::Number(_) => self.add_pure("NUMBER", Vec::new()),
            AstNode::StringLit(_) => self.add_effect(
                "STRING_LITERAL",
                Vec::new(),
                ComputeCapabilities {
                    deterministic: true,
                    streaming: false,
                    stateful: true,
                    lookback: LookbackRequirement::None,
                    // The executor appends literals to FormulaContext::string_table.
                    effect: ComputeEffect::Stateful,
                },
            ),
            AstNode::Variable(name) => self.lower_variable(name),
            AstNode::BinaryOp { op, left, right } => {
                let left = self.lower(left);
                let right = self.lower(right);
                self.add_pure(format!("BINARY:{op:?}"), vec![left, right])
            }
            AstNode::UnaryOp { op, expr } => {
                let expr = self.lower(expr);
                self.add_pure(format!("UNARY:{op:?}"), vec![expr])
            }
            AstNode::FunctionCall { name, args } => {
                let dependencies = args.iter().map(|arg| self.lower(arg)).collect();
                let capabilities = self.function_capabilities(name);
                if capabilities.effect.is_pure() {
                    self.add_node(
                        format!("CALL:{}", canonical_name(name)),
                        dependencies,
                        capabilities,
                    )
                } else {
                    self.add_effect(
                        format!("CALL:{}", canonical_name(name)),
                        dependencies,
                        capabilities,
                    )
                }
            }
            AstNode::IndexAccess { array, index } => {
                let array = self.lower(array);
                let index = self.lower(index);
                self.add_pure("INDEX", vec![array, index])
            }
            AstNode::Assignment { name, expr } => {
                let expr = self.lower(expr);
                let id = self.add_effect(
                    format!("ASSIGN:{}", canonical_name(name)),
                    vec![expr],
                    ComputeCapabilities {
                        deterministic: true,
                        streaming: true,
                        stateful: false,
                        lookback: LookbackRequirement::None,
                        effect: ComputeEffect::WriteVariable(name.clone()),
                    },
                );
                self.last_write.insert(canonical_name(name), id);
                id
            }
            AstNode::CompoundAssignment { name, op, expr } => {
                let current = self.lower_variable(name);
                let expr = self.lower(expr);
                let id = self.add_effect(
                    format!("COMPOUND:{name}:{op:?}"),
                    vec![current, expr],
                    ComputeCapabilities {
                        deterministic: true,
                        streaming: true,
                        stateful: false,
                        lookback: LookbackRequirement::None,
                        effect: ComputeEffect::WriteVariable(name.clone()),
                    },
                );
                self.last_write.insert(canonical_name(name), id);
                id
            }
            AstNode::Output { name, expr, .. } => {
                let expr = self.lower(expr);
                let id = self.add_effect(
                    format!("OUTPUT:{}", canonical_name(name)),
                    vec![expr],
                    ComputeCapabilities {
                        deterministic: true,
                        streaming: true,
                        stateful: false,
                        lookback: LookbackRequirement::None,
                        effect: ComputeEffect::EmitOutput(name.clone()),
                    },
                );
                // FormulaExecutor stores outputs in FormulaContext::variables, so
                // a later reference to the output must depend on this node.
                self.last_write.insert(canonical_name(name), id);
                id
            }
            AstNode::Statements(statements) => {
                let dependencies = statements
                    .iter()
                    .map(|statement| self.lower(statement))
                    .collect();
                self.add_pure("STATEMENTS", dependencies)
            }
            AstNode::ParamDecl { name, .. } => self.add_node(
                format!("PARAM:{}", canonical_name(name)),
                Vec::new(),
                ComputeCapabilities {
                    deterministic: true,
                    streaming: false,
                    stateful: false,
                    lookback: LookbackRequirement::None,
                    effect: ComputeEffect::Pure,
                },
            ),
            AstNode::DrawText { cond, price, .. } => {
                let cond = self.lower(cond);
                let price = self.lower(price);
                self.add_draw("DRAW_TEXT", vec![cond, price])
            }
            AstNode::DrawIcon {
                cond, price, icon, ..
            } => {
                let cond = self.lower(cond);
                let price = self.lower(price);
                let icon = self.lower(icon);
                self.add_draw("DRAW_ICON", vec![cond, price, icon])
            }
            AstNode::StickLine {
                cond,
                price1,
                price2,
                width,
                ..
            } => {
                let cond = self.lower(cond);
                let price1 = self.lower(price1);
                let price2 = self.lower(price2);
                let width = self.lower(width);
                self.add_draw("STICK_LINE", vec![cond, price1, price2, width])
            }
            AstNode::DrawGeneric { command, args, .. } => {
                let dependencies = args.iter().map(|arg| self.lower(arg)).collect();
                self.add_draw(format!("DRAW:{}", canonical_name(command)), dependencies)
            }
            AstNode::IfThenElse {
                cond,
                then_branch,
                else_branch,
            } => {
                // FormulaExecutor currently evaluates both branches before
                // selecting the result, so lowering both branches preserves its
                // existing observable side-effect order.
                let cond = self.lower(cond);
                let then_branch = self.lower(then_branch);
                let else_branch = self.lower(else_branch);
                self.add_pure("IF_THEN_ELSE", vec![cond, then_branch, else_branch])
            }
            AstNode::ForLoop {
                var, start, end, ..
            } => {
                let start = self.lower(start);
                let end = self.lower(end);
                let id = self.add_effect(
                    format!("FOR_LOOP:{}", canonical_name(var)),
                    vec![start, end],
                    opaque_control_flow_capabilities(),
                );
                // Loop bodies remain opaque at this planning layer because
                // representing loop-carried dependencies in an acyclic plan
                // requires a dedicated control-flow IR. The stateful barrier
                // prevents unsafe elimination/reordering in the meantime.
                self.last_write.insert(canonical_name(var), id);
                id
            }
            AstNode::WhileLoop { cond, .. } => {
                let cond = self.lower(cond);
                self.add_effect("WHILE_LOOP", vec![cond], opaque_control_flow_capabilities())
            }
        }
    }

    fn lower_variable(&mut self, name: &str) -> ComputeNodeId {
        let key = canonical_name(name);
        let dependencies = self.last_write.get(&key).copied().into_iter().collect();
        self.add_pure(format!("VARIABLE:{key}"), dependencies)
    }

    fn function_capabilities(&self, name: &str) -> ComputeCapabilities {
        self.registry.get(name).map_or_else(
            || ComputeCapabilities {
                // Unknown/custom formula functions are deliberately conservative.
                // Once registered in the SSOT they regain precise capabilities.
                deterministic: false,
                streaming: false,
                stateful: true,
                lookback: LookbackRequirement::Dynamic,
                effect: ComputeEffect::Stateful,
            },
            ComputeCapabilities::from_function_spec,
        )
    }

    fn add_pure(
        &mut self,
        operation: impl Into<String>,
        dependencies: Vec<ComputeNodeId>,
    ) -> ComputeNodeId {
        self.add_node(
            operation,
            dependencies,
            ComputeCapabilities {
                deterministic: true,
                streaming: true,
                stateful: false,
                lookback: LookbackRequirement::None,
                effect: ComputeEffect::Pure,
            },
        )
    }

    fn add_draw(
        &mut self,
        operation: impl Into<String>,
        dependencies: Vec<ComputeNodeId>,
    ) -> ComputeNodeId {
        self.add_effect(
            operation,
            dependencies,
            ComputeCapabilities {
                deterministic: true,
                streaming: true,
                stateful: false,
                lookback: LookbackRequirement::None,
                effect: ComputeEffect::Draw,
            },
        )
    }

    fn add_effect(
        &mut self,
        operation: impl Into<String>,
        mut dependencies: Vec<ComputeNodeId>,
        capabilities: ComputeCapabilities,
    ) -> ComputeNodeId {
        if let Some(previous) = self.last_effect {
            dependencies.push(previous);
        }
        let id = self.add_node(operation, dependencies, capabilities);
        self.last_effect = Some(id);
        id
    }

    fn add_node(
        &mut self,
        operation: impl Into<String>,
        dependencies: Vec<ComputeNodeId>,
        capabilities: ComputeCapabilities,
    ) -> ComputeNodeId {
        let id = ComputeNodeId(self.next_id);
        self.next_id += 1;
        self.nodes
            .push(ComputeNode::new(id, operation, dependencies, capabilities));
        id
    }
}

fn canonical_name(name: &str) -> String {
    name.trim().to_ascii_uppercase()
}

fn opaque_control_flow_capabilities() -> ComputeCapabilities {
    ComputeCapabilities {
        deterministic: false,
        streaming: false,
        stateful: true,
        lookback: LookbackRequirement::Dynamic,
        effect: ComputeEffect::Stateful,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formula::parse_formula;

    #[test]
    fn assignment_and_output_are_observable_effects() {
        let ast = parse_formula("MA5:=MA(CLOSE,5);SELL:CROSS(CLOSE,MA5);").unwrap();
        let formula_plan = FormulaComputePlan::compile(&ast).unwrap();
        let plan = formula_plan.plan();

        let mut saw_assignment = false;
        let mut saw_output = false;
        for &id in plan.execution_order() {
            match &plan.node(id).unwrap().capabilities.effect {
                ComputeEffect::WriteVariable(name) if name == "MA5" => saw_assignment = true,
                ComputeEffect::EmitOutput(name) if name == "SELL" => saw_output = true,
                _ => {}
            }
        }

        assert!(saw_assignment);
        assert!(saw_output);
        assert!(plan.has_observable_effects());
    }

    #[test]
    fn later_variable_read_depends_on_latest_assignment() {
        let ast = parse_formula("MA5:=MA(CLOSE,5);MA5+1;").unwrap();
        let formula_plan = FormulaComputePlan::compile(&ast).unwrap();
        let plan = formula_plan.plan();

        let assignment = plan
            .execution_order()
            .iter()
            .copied()
            .find(|&id| plan.node(id).unwrap().operation == "ASSIGN:MA5")
            .unwrap();
        let read = plan
            .execution_order()
            .iter()
            .copied()
            .filter(|&id| plan.node(id).unwrap().operation == "VARIABLE:MA5")
            .last()
            .unwrap();

        assert!(plan.node(read).unwrap().dependencies.contains(&assignment));
    }

    #[test]
    fn registered_function_uses_ssot_capabilities() {
        let ast = parse_formula("MA(CLOSE,5)").unwrap();
        let formula_plan = FormulaComputePlan::compile(&ast).unwrap();
        let plan = formula_plan.plan();
        let call = plan
            .execution_order()
            .iter()
            .copied()
            .find(|&id| plan.node(id).unwrap().operation == "CALL:MA")
            .unwrap();
        let capabilities = &plan.node(call).unwrap().capabilities;

        assert!(capabilities.deterministic);
        assert!(capabilities.streaming);
        assert!(capabilities.effect.is_pure());
        assert_eq!(capabilities.lookback, LookbackRequirement::PeriodMinusOne);
    }

    #[test]
    fn unknown_function_is_conservative_until_registered() {
        let ast = AstNode::FunctionCall {
            name: "CUSTOM_FN".to_string(),
            args: vec![AstNode::Variable("CLOSE".to_string())],
        };
        let formula_plan = FormulaComputePlan::compile(&ast).unwrap();
        let root = formula_plan.root();
        let capabilities = &formula_plan.plan().node(root).unwrap().capabilities;

        assert!(!capabilities.deterministic);
        assert!(!capabilities.streaming);
        assert!(capabilities.stateful);
        assert_eq!(capabilities.effect, ComputeEffect::Stateful);
        assert_eq!(capabilities.lookback, LookbackRequirement::Dynamic);
    }

    #[test]
    fn drawing_commands_are_effectful_and_ordered() {
        let ast = AstNode::Statements(vec![
            AstNode::Assignment {
                name: "X".to_string(),
                expr: Box::new(AstNode::Number(1.0)),
            },
            AstNode::DrawText {
                cond: Box::new(AstNode::Variable("X".to_string())),
                price: Box::new(AstNode::Variable("CLOSE".to_string())),
                text: "signal".to_string(),
                color: None,
            },
        ]);
        let formula_plan = FormulaComputePlan::compile(&ast).unwrap();
        let plan = formula_plan.plan();
        let assignment = plan
            .execution_order()
            .iter()
            .copied()
            .find(|&id| plan.node(id).unwrap().operation == "ASSIGN:X")
            .unwrap();
        let draw = plan
            .execution_order()
            .iter()
            .copied()
            .find(|&id| plan.node(id).unwrap().operation == "DRAW_TEXT")
            .unwrap();

        assert_eq!(
            plan.node(draw).unwrap().capabilities.effect,
            ComputeEffect::Draw
        );
        assert!(plan.node(draw).unwrap().dependencies.contains(&assignment));
    }
}
