//! Semantic lowering from formula AST nodes into the unified compute plan.
//!
//! The formula AST describes syntax. This module adds planner-visible data
//! dependencies and observable effects so optimizers can reason about formula
//! semantics without guessing whether an assignment, output, drawing command,
//! or context mutation is safe to remove.

use super::ast::{AstNode, BinaryOperator, OutputModifier, UnaryOperator};
use super::params::expand_implicit_price_args;
use crate::compute::{
    ComputeCapabilities, ComputeEffect, ComputeNode, ComputeNodeId, ComputePlan, ComputePlanError,
    LookbackRequirement,
};
use crate::registry::{builtin_function_registry, FunctionRegistry};
use std::collections::BTreeMap;

/// Ceiling on how many times a `for` loop body may be duplicated.
///
/// Unrolling is linear in the iteration count, so an unbounded loop would be a
/// memory-amplification vector: `for i = 0 to 1000000` would emit millions of
/// nodes. Exceeding this is a compile error rather than a truncation, because
/// a truncated loop silently computes a partial sum.
///
/// It deliberately reuses the interpreter's own iteration ceiling rather than
/// introducing a second one. A lower ceiling here would make loops that the
/// tree path executes perfectly well a hard compile error on the plan path,
/// which is exactly the kind of split that makes a "drop-in faster path" a lie.
use super::executor::MAX_LOOP_ITERATIONS as MAX_UNROLLED_LOOP_ITERATIONS;

/// Validated semantic compute plan derived from one formula AST.
#[derive(Debug, Clone)]
pub struct FormulaComputePlan {
    plan: ComputePlan,
    root: ComputeNodeId,
    /// Exact literal carried by each `NUMBER` node, keyed by node id.
    ///
    /// The lowerer records a literal at the moment it creates the node, so this
    /// map stays correct under loop unrolling and under any pass that later
    /// drops nodes. Binding literals by re-walking the AST cannot: the walker
    /// would have to reproduce every lowering decision (including how many
    /// times a loop body was duplicated), and any drift shows up as a plan that
    /// silently binds the wrong constant.
    number_literals: BTreeMap<ComputeNodeId, f64>,
    /// Deepest AST nesting seen while lowering. See [`Self::max_ast_depth`].
    max_ast_depth: usize,
    /// Chart styling per declared output, keyed by the name as written.
    ///
    /// The tree path publishes these into `ctx.output_modifiers` and the FFI
    /// layer reads them back to describe a series' colour and line style. They
    /// are not part of the numeric plan, so they travel beside it rather than in
    /// a [`ComputeEffect`].
    output_modifiers: BTreeMap<String, OutputModifier>,
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
        // Context-implicit price arguments cannot survive lowering. The plan's
        // input layout only carries series the source text *names*, so
        // `PLUS_DI(CLOSE, 14)` — which the tree path evaluates against
        // `ctx.high`/`ctx.low`/`ctx.close` — would reach the dispatcher one
        // operand short and be rejected with `ERR_ARITY`. Expanding the short
        // form into its explicit `HIGH`/`LOW`/`CLOSE` spelling here keeps the
        // plan self-contained and is value-preserving. See
        // [`crate::formula::params::expand_implicit_price_args`].
        let ast = expand_implicit_price_args(ast);
        let mut lowerer = FormulaLowerer::new(registry);
        let root = lowerer.lower(&ast);
        let plan = ComputePlan::compile(lowerer.nodes)?;
        if let Some(error) = lowerer.pending_error {
            return Err(error);
        }
        Ok(Self {
            plan,
            root,
            number_literals: lowerer.number_literals,
            max_ast_depth: lowerer.max_depth,
            output_modifiers: lowerer.output_modifiers,
        })
    }

    /// Unified compute plan containing dependencies and effects.
    pub const fn plan(&self) -> &ComputePlan {
        &self.plan
    }

    /// Node representing the formula's final value.
    pub const fn root(&self) -> ComputeNodeId {
        self.root
    }

    /// Literal carried by a `NUMBER` node, if that node is a literal.
    ///
    /// See the field documentation on [`FormulaComputePlan`] for why the value
    /// is recorded by the lowerer rather than recovered from the AST.
    pub fn number_literal(&self, node: ComputeNodeId) -> Option<f64> {
        self.number_literals.get(&node).copied()
    }

    /// Number of `NUMBER` literals recorded by the lowerer.
    pub fn number_literals_len(&self) -> usize {
        self.number_literals.len()
    }

    /// Deepest AST nesting the lowerer walked.
    ///
    /// Published so the execution sandbox can apply `max_recursion_depth` to
    /// the compiled plan. The limit protects against stack exhaustion, and on
    /// this path the recursive walk is lowering rather than interpretation, so
    /// the depth has to be measured there and carried out — by the time the plan
    /// executes, the stack has already been used.
    pub const fn max_ast_depth(&self) -> usize {
        self.max_ast_depth
    }

    /// Chart styling for each declared output, keyed by the name as written.
    ///
    /// Published so the plan path can leave `ctx.output_modifiers` in the same
    /// state the tree path does; see [`Self::output_modifiers`]'s field docs.
    pub const fn output_modifiers(&self) -> &BTreeMap<String, OutputModifier> {
        &self.output_modifiers
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
    last_control_flow: Option<ComputeNodeId>,
    /// Variables whose current value is a compile-time constant, so a later
    /// read can be folded.
    ///
    /// Invalidated wholesale at every control-flow boundary. An opaque loop or
    /// a conditional body can assign to a variable we believe is constant, so
    /// folding a read across one would replace a runtime value with a stale
    /// literal — silently, and only for the formulas that actually take the
    /// other branch.
    const_env: BTreeMap<String, f64>,
    /// The loop variable of the `for` loop currently being unrolled, if any.
    ///
    /// Kept apart from [`Self::const_env`] because it must survive a control-flow
    /// boundary inside the loop body: `if cond { x := x + y[i] }` still has to
    /// resolve `i`, even though the branch invalidates every other constant.
    loop_var: Option<(String, f64)>,
    /// Literal value of each `NUMBER` node, recorded at creation time.
    number_literals: BTreeMap<ComputeNodeId, f64>,
    /// Chart styling per declared output, keyed by the name as written.
    output_modifiers: BTreeMap<String, OutputModifier>,
    /// Current AST nesting depth of the walk.
    depth: usize,
    /// Deepest nesting the walk reached, i.e. the value the sandbox compares
    /// against `max_recursion_depth`.
    max_depth: usize,
    /// First error that made lowering impossible.
    ///
    /// `lower` returns a node id rather than a `Result`, so a failure that is
    /// only discovered mid-traversal is parked here and surfaced by
    /// [`FormulaComputePlan::compile`]. Lowering still completes, which keeps
    /// node ids stable for the rest of the pass.
    pending_error: Option<ComputePlanError>,
}

impl<'a> FormulaLowerer<'a> {
    fn new(registry: &'a FunctionRegistry) -> Self {
        Self {
            registry,
            nodes: Vec::new(),
            next_id: 0,
            last_write: BTreeMap::new(),
            last_effect: None,
            last_control_flow: None,
            const_env: BTreeMap::new(),
            loop_var: None,
            number_literals: BTreeMap::new(),
            output_modifiers: BTreeMap::new(),
            depth: 0,
            max_depth: 0,
            pending_error: None,
        }
    }

    /// Lower one AST node, tracking how deeply the walk nested.
    ///
    /// The nesting depth is recorded because the sandbox's recursion limit is
    /// checked against it on the plan path. Lowering is the plan path's
    /// equivalent of the tree path's recursive `execute_val`: it is where a
    /// deeply nested formula would consume stack, so it is also where the limit
    /// has to be applied.
    fn lower(&mut self, ast: &AstNode) -> ComputeNodeId {
        self.depth += 1;
        self.max_depth = self.max_depth.max(self.depth);
        let node = self.lower_inner(ast);
        self.depth -= 1;
        node
    }

    fn lower_inner(&mut self, ast: &AstNode) -> ComputeNodeId {
        match ast {
            AstNode::Number(value) => self.add_number(*value),
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
            AstNode::Variable(name) => match self.constant_of(name) {
                // A variable bound to a constant reads as that constant. This is
                // what makes an unrolled loop variable reachable inside the
                // body: `volume[i]` must resolve `i`, not bind an input series
                // named `i`.
                Some(value) => self.add_number(value),
                None => self.lower_variable(name),
            },
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
                let (operation_name, capabilities) = self.function_metadata(name);
                if capabilities.effect.is_pure() {
                    self.add_node(format!("CALL:{operation_name}"), dependencies, capabilities)
                } else {
                    self.add_effect(format!("CALL:{operation_name}"), dependencies, capabilities)
                }
            }
            AstNode::IndexAccess { array, index } => {
                let array = self.lower(array);
                let index = self.lower(index);
                self.add_pure("INDEX", vec![array, index])
            }
            AstNode::Assignment { name, expr } => {
                // Track constant variables so a later `for` bound such as
                // `lookback - 1` can still be folded after the parameter has
                // been applied. A reassignment to something non-constant must
                // drop the entry: a stale constant would fold a variable that
                // is no longer constant.
                match self.const_eval(expr) {
                    Some(value) => self.const_env.insert(canonical_name(name), value),
                    None => self.const_env.remove(&canonical_name(name)),
                };
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
            AstNode::Output {
                name,
                expr,
                modifier,
            } => {
                let expr = self.lower(expr);
                // A level marker (Pine `hline`) draws a horizontal price line.
                // It is observable like any other output, but it carries no data
                // series, so it must stay distinguishable here — otherwise the
                // plan path would select `hline(30)` as the whole result of a
                // script. See `AstNode::produces_value`.
                let effect = match modifier
                    .as_ref()
                    .is_some_and(OutputModifier::is_level_marker)
                {
                    true => ComputeEffect::EmitLevelMarker(name.clone()),
                    false => ComputeEffect::EmitOutput(name.clone()),
                };
                let id = self.add_effect(
                    format!("OUTPUT:{}", canonical_name(name)),
                    vec![expr],
                    ComputeCapabilities {
                        deterministic: true,
                        streaming: true,
                        stateful: false,
                        lookback: LookbackRequirement::None,
                        effect,
                    },
                );
                // FormulaExecutor stores outputs in FormulaContext::variables, so
                // a later reference to the output must depend on this node.
                self.last_write.insert(canonical_name(name), id);
                // Chart styling is host-visible state the tree path leaves in
                // `ctx.output_modifiers`, keyed by the name as written. It is
                // recorded here because lowering is the last point where the
                // `AstNode::Output` modifier is still in hand — the compute plan
                // itself only needs the level-marker distinction.
                if let Some(modifier) = modifier {
                    self.output_modifiers.insert(name.clone(), modifier.clone());
                }
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
            AstNode::DrawGeneric { args, .. } => {
                let dependencies = args.iter().map(|arg| self.lower(arg)).collect();
                // One operation name for *every* generic draw, deliberately. The
                // command is rendering information that the numeric plan does
                // not carry, exactly like the command-less `DRAW_TEXT`,
                // `DRAW_ICON` and `STICK_LINE` above.
                //
                // Emitting `DRAW:{command}` here instead made the plan path need
                // one kernel per command, and because `KernelId` is an opaque
                // hash the dispatcher had to enumerate them by hand — so eleven
                // of the twelve commands (including `DRAWLINE`, the most common
                // one) fell through unhandled while the dispatcher's list still
                // read as if it were complete. Collapsing removes that failure
                // mode structurally rather than by keeping two lists in sync.
                self.add_draw("DRAW_GENERIC", dependencies)
            }
            AstNode::IfThenElse {
                cond,
                then_branch,
                else_branch,
            } => {
                // FormulaExecutor currently evaluates both branches before
                // selecting the result, so lowering both branches preserves its
                // existing observable side-effect order.
                //
                // Emitted as `CALL:IF` rather than a bespoke `IF_THEN_ELSE`
                // node name: the operation is the same select that the `IF(...)`
                // function performs, and `IF` is a registered, pure function.
                // An unregistered node name would instead be demoted to a
                // stateful barrier by `function_metadata`, blocking CSE.
                let cond = self.lower(cond);
                let then_branch = self.lower(then_branch);
                let else_branch = self.lower(else_branch);
                let id = self.add_pure("CALL:IF", vec![cond, then_branch, else_branch]);
                // Either branch may assign, and which one runs is a runtime
                // fact, so nothing folded before this point is still a known
                // constant afterwards.
                self.invalidate_constants();
                id
            }
            AstNode::ForLoop {
                var,
                start,
                end,
                body,
            } => self.lower_for_loop(var, start, end, body),
            AstNode::WhileLoop { cond, .. } => {
                let cond = self.lower(cond);
                let id =
                    self.add_effect("WHILE_LOOP", vec![cond], opaque_control_flow_capabilities());
                self.last_control_flow = Some(id);
                // The body is opaque, so any number of iterations — including
                // zero — may have assigned to something we folded as constant.
                self.invalidate_constants();
                id
            }
        }
    }

    fn lower_variable(&mut self, name: &str) -> ComputeNodeId {
        let key = canonical_name(name);
        let mut dependencies = Vec::with_capacity(2);
        if let Some(write) = self.last_write.get(&key).copied() {
            dependencies.push(write);
        }
        if let Some(barrier) = self.last_control_flow {
            if !dependencies.contains(&barrier) {
                dependencies.push(barrier);
            }
        }
        self.add_pure(format!("VARIABLE:{key}"), dependencies)
    }

    fn function_metadata(&self, name: &str) -> (String, ComputeCapabilities) {
        self.registry.get(name).map_or_else(
            || {
                (
                    canonical_name(name),
                    ComputeCapabilities {
                        // Unknown/custom formula functions are deliberately conservative.
                        // Once registered in the SSOT they regain precise capabilities.
                        deterministic: false,
                        streaming: false,
                        stateful: true,
                        lookback: LookbackRequirement::Dynamic,
                        effect: ComputeEffect::Stateful,
                    },
                )
            },
            |spec| {
                (
                    canonical_name(spec.name),
                    ComputeCapabilities::from_function_spec(spec),
                )
            },
        )
    }

    /// Constant currently bound to a variable, if it has one.
    ///
    /// The unrolled loop variable wins over [`Self::const_env`], because the
    /// latter is cleared by control flow inside the loop body while the loop
    /// variable must stay visible for the whole iteration.
    fn constant_of(&self, name: &str) -> Option<f64> {
        let key = canonical_name(name);
        if let Some((variable, value)) = &self.loop_var {
            if *variable == key {
                return Some(*value);
            }
        }
        self.const_env.get(&key).copied()
    }

    /// Drop every folded constant.
    ///
    /// Called at control-flow boundaries: anything the body of an opaque loop
    /// or a conditional assigns may or may not have happened at runtime, so no
    /// read after one can be trusted to still hold a literal.
    fn invalidate_constants(&mut self) {
        self.const_env.clear();
    }

    /// Evaluate an expression whose value is already known at compile time.
    ///
    /// Only arithmetic over literals and over variables bound to literals is
    /// folded. Everything else yields `None`, which callers must treat as "not
    /// a constant" rather than as zero.
    fn const_eval(&self, ast: &AstNode) -> Option<f64> {
        match ast {
            AstNode::Number(value) => Some(*value),
            AstNode::Variable(name) => self.constant_of(name),
            AstNode::UnaryOp {
                op: UnaryOperator::Neg,
                expr,
            } => self.const_eval(expr).map(|value| -value),
            AstNode::BinaryOp { op, left, right } => {
                let left = self.const_eval(left)?;
                let right = self.const_eval(right)?;
                match op {
                    BinaryOperator::Add => Some(left + right),
                    BinaryOperator::Sub => Some(left - right),
                    BinaryOperator::Mul => Some(left * right),
                    BinaryOperator::Div => Some(left / right),
                    BinaryOperator::Mod => Some(left % right),
                    BinaryOperator::Pow => Some(left.powf(right)),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// Record the first lowering failure. Later failures do not overwrite it,
    /// so the reported cause is the earliest one in the formula.
    fn fail(&mut self, error: ComputePlanError) {
        if self.pending_error.is_none() {
            self.pending_error = Some(error);
        }
    }

    /// Lower a `for` loop by unrolling it, when its bounds are compile-time
    /// constants.
    ///
    /// An acyclic plan has nowhere to put a back edge, so the only way to give
    /// the loop its real semantics is to emit the body once per iteration.
    /// Loop-carried state then becomes an ordinary dependency chain: each
    /// iteration writes a variable and the next one reads it, which is exactly
    /// the order the tree-walking reference path evaluates them in.
    ///
    /// Bounds that are not constant cannot be unrolled, and that is a hard
    /// error rather than a silent skip. Dropping the body would leave every
    /// accumulator at its initial value, producing a wrong number instead of a
    /// missing one — the failure has to be loud so the caller sees it.
    fn lower_for_loop(
        &mut self,
        var: &str,
        start: &AstNode,
        end: &AstNode,
        body: &[AstNode],
    ) -> ComputeNodeId {
        let key = canonical_name(var);
        let bounds = match (self.const_eval(start), self.const_eval(end)) {
            (Some(from), Some(to))
                if from.fract() == 0.0 && to.fract() == 0.0 && from >= 0.0 && to >= 0.0 =>
            {
                Some((from as i64, to as i64))
            }
            _ => None,
        };

        let Some((from, to)) = bounds else {
            self.fail(ComputePlanError::UnsupportedLoop {
                variable: var.to_string(),
                reason: "bounds are not compile-time constants, so the loop cannot be unrolled"
                    .to_string(),
            });
            return self.opaque_for_loop(var, start, end);
        };

        if to < from {
            // The body never runs. This is a legal empty range, not an error.
            return self.lower(end);
        }

        let iterations = (to - from + 1) as usize;
        if iterations > MAX_UNROLLED_LOOP_ITERATIONS {
            self.fail(ComputePlanError::UnsupportedLoop {
                variable: var.to_string(),
                reason: format!(
                    "{iterations} iterations exceeds the unrolling limit of \
                     {MAX_UNROLLED_LOOP_ITERATIONS}"
                ),
            });
            return self.opaque_for_loop(var, start, end);
        }

        // Nested loops: an outer loop variable must still resolve inside an
        // inner body, so the binding is saved and restored rather than cleared.
        let outer_loop_var = self.loop_var.take();
        let mut last = None;
        for index in from..=to {
            self.loop_var = Some((key.clone(), index as f64));
            for statement in body {
                last = Some(self.lower(statement));
            }
        }
        self.loop_var = outer_loop_var;

        // Iterations may have assigned to anything, and how many ran is enough
        // to make no pre-loop constant trustworthy afterwards.
        self.invalidate_constants();
        // Pine leaves the loop variable at its final value once the loop ends,
        // so keep it bound instead of letting a later read fall through to an
        // input slot and fail as an unknown series.
        self.const_env.insert(key, to as f64);

        match last {
            Some(id) => id,
            // An empty body produces no value; fall back to the bound it ended on.
            None => self.add_number(to as f64),
        }
    }

    /// The pre-unrolling fallback: one opaque node standing in for the loop.
    ///
    /// Only reached on the error path, so the plan still has a node to return
    /// while [`FormulaComputePlan::compile`] reports the recorded failure.
    fn opaque_for_loop(&mut self, var: &str, start: &AstNode, end: &AstNode) -> ComputeNodeId {
        let start = self.lower(start);
        let end = self.lower(end);
        let id = self.add_effect(
            format!("FOR_LOOP:{}", canonical_name(var)),
            vec![start, end],
            opaque_control_flow_capabilities(),
        );
        self.last_write.insert(canonical_name(var), id);
        // The body was never lowered, so it may have assigned to anything.
        self.invalidate_constants();
        id
    }

    /// Create a `NUMBER` node and record the literal it carries.
    fn add_number(&mut self, value: f64) -> ComputeNodeId {
        let id = self.add_pure("NUMBER", Vec::new());
        self.number_literals.insert(id, value);
        id
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

/// Canonical form of a formula variable name.
///
/// The formula layer trims and upper-cases every name before it becomes an
/// operation label, so `fast`, `FAST` and `" fast "` all denote one variable.
/// Exposed crate-wide so a frontend that builds an AST directly (see
/// [`crate::factor_graph`]) can apply the same rule instead of mirroring it.
pub(crate) fn canonical_name(name: &str) -> String {
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
    fn ma_and_sma_keep_distinct_canonical_operations() {
        let ma = FormulaComputePlan::compile(&parse_formula("MA(CLOSE,5)").unwrap()).unwrap();
        let sma = FormulaComputePlan::compile(&parse_formula("SMA(CLOSE,5,1)").unwrap()).unwrap();

        let ma_node = ma.plan().node(ma.root()).unwrap();
        let sma_node = sma.plan().node(sma.root()).unwrap();
        assert_eq!(ma_node.operation, "CALL:MA");
        assert_eq!(sma_node.operation, "CALL:SMA");
        assert_ne!(
            ma_node.capabilities.lookback,
            sma_node.capabilities.lookback
        );
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
    fn reads_after_opaque_control_flow_depend_on_the_control_barrier() {
        let ast = AstNode::Statements(vec![
            AstNode::Assignment {
                name: "X".to_string(),
                expr: Box::new(AstNode::Number(0.0)),
            },
            AstNode::WhileLoop {
                cond: Box::new(AstNode::Number(0.0)),
                body: vec![AstNode::Assignment {
                    name: "X".to_string(),
                    expr: Box::new(AstNode::Number(1.0)),
                }],
            },
            AstNode::Variable("X".to_string()),
        ]);
        let formula_plan = FormulaComputePlan::compile(&ast).unwrap();
        let plan = formula_plan.plan();
        let barrier = plan
            .execution_order()
            .iter()
            .copied()
            .find(|&id| plan.node(id).unwrap().operation == "WHILE_LOOP")
            .unwrap();
        let read = plan
            .execution_order()
            .iter()
            .copied()
            .filter(|&id| plan.node(id).unwrap().operation == "VARIABLE:X")
            .last()
            .unwrap();

        assert!(plan.node(read).unwrap().dependencies.contains(&barrier));
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
