//! Declarative factor graphs, lowered onto the compiled formula plan path.
//!
//! [`crate::factors::FactorDefinition`] describes a factor that is *already*
//! registered, and [`crate::factor_provider`] turns a declarative request into
//! one. This module covers the layer in between: a **multi-node graph** — `SMA`
//! feeding `EMA` feeding a ratio — assembled from ordinary function names rather
//! than a formula string.
//!
//! # Why a graph at all
//!
//! A formula string is the right interface for a human. A graph is the right
//! interface for a *program*: its nodes have stable identity, so a caller can
//! name an intermediate series, read it back by id after execution, and diff two
//! graphs structurally. Crucially, this does **not** introduce a second execution
//! engine. [`FactorGraph::build`](crate::factor_graph::FactorGraph::build) lowers the graph to an [`AstNode::Statements`](crate::formula::ast::AstNode::Statements)
//! block and hands it to [`FormulaHotPlan::compile`](crate::formula::hot_plan::FormulaHotPlan::compile), so a graph and the
//! equivalent formula string share one lowering contract, one optimizer, one CSE
//! pass, one dead-code elimination pass and one kernel dispatcher. A graph
//! therefore cannot drift from a formula — and the differential test at the
//! bottom of this file holds it to that.
//!
//! # Three node kinds, because the plan path has two call conventions
//!
//! [`crate::formula::unified_dispatch`] dispatches a named function through
//! `CALL:<name>` and an arithmetic operator through `BINARY:<op>`. Those are
//! genuinely different kernels: `CALL:SUB` is not dispatched, so a graph that
//! modelled `fast - slow` as a call to the registry's `SUB` function would
//! compile and then fail at execution. [`FactorOperation`](crate::factor_graph::FactorOperation) therefore keeps the
//! distinction explicit rather than inferring it from a function name:
//!
//! * [`FactorOperation::Call`](crate::factor_graph::FactorOperation::Call) — a registry function, with numeric parameters;
//! * [`FactorOperation::Binary`](crate::factor_graph::FactorOperation::Binary) — one of the `BINARY:*` operators, two operands;
//! * [`FactorOperation::Constant`](crate::factor_graph::FactorOperation::Constant) — a scalar, which is what lets a factor scale
//!   or offset a series.
//!
//! # Emission shape is load-bearing
//!
//! Two properties of the plumbing pass (`hot_plan::lower_formula_plumbing`)
//! decide how the graph must be emitted, and neither is obvious from the AST:
//!
//! * Nodes are emitted in **topological order**, because lowering resolves a
//!   variable against the *most recent prior write* (see `compute_ir`'s
//!   `lower_variable`). A reference to a node that has not been emitted yet does
//!   not fail — it silently becomes an **external input** of the same name. A
//!   cycle is therefore detected *here*, by Kahn's algorithm, and reported as
//!   [`FactorGraphError::Cycle`](crate::factor_graph::FactorGraphError::Cycle); delegating that to the compute plan would not
//!   work, because the plan never observes a cycle, only a set of variables that
//!   quietly resolve to external inputs.
//! * Every node is defined by an **`Assignment`**, not only published as an
//!   `Output`. The plumbing pass records only `ASSIGN:` nodes in its
//!   `local_writes` table, so `VARIABLE:<id>` resolves to the node's definition
//!   *only* when that definition is an assignment. Publishing a node purely as
//!   an `Output` would leave every reference to it classified as an external
//!   input by [`crate::execution_plan::InputLayout`] — a plan that compiles and
//!   then demands a data series named after your node. Each node is published as
//!   an `Output` as well, but *after* it has been assigned.
//!
//! One naming rule follows from the same layer: node ids are canonicalized the
//! way the formula layer canonicalizes variables — trimmed and upper-cased — so
//! `fast`, `FAST` and `" fast "` denote one node. They collide at declaration
//! ([`FactorGraphError::DuplicateNode`](crate::factor_graph::FactorGraphError::DuplicateNode)) instead of the later one silently
//! shadowing the earlier in the plan.
//!
//! # Provenance
//!
//! Ported from the `crates/finkit-runtime` migration track, whose `FactorGraph`
//! and `Scheduler` formed a parallel execution engine — see
//! `docs/runtime-carrier-adoption-plan-2026-09-20.md`. Two deliberate departures:
//!
//! * The track's `Scheduler` is **not** ported. Cycle reporting is already owned
//!   by [`crate::compute::ComputePlanError::DependencyCycle`], and this module
//!   reports a cycle before a plan is ever built.
//! * The track allowed **one dependency per node**. That limit is not carried
//!   over: a node takes any number of inputs, which is what makes
//!   `(EMA(CLOSE,12) - EMA(CLOSE,26)) / CLOSE` expressible at all.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::formula::compute_ir::canonical_name;
use crate::formula::hot_plan::FormulaOutputBinding;
use crate::formula::{
    unified_formula_executor, AstNode, BinaryOperator, FormulaContext, FormulaHotPlan,
    FormulaHotPlanError, FormulaInputBinding, UnaryOperator,
};
use crate::unified_executor::ExecuteError;

/// What a [`FactorNode`] computes from its inputs.
#[derive(Clone, Debug, PartialEq)]
pub enum FactorOperation {
    /// Apply a registry function, lowering to `CALL:<name>`.
    ///
    /// The node's numeric parameters are appended after its inputs, matching the
    /// argument order every formula function uses (`SMA(CLOSE, 20)`).
    Call {
        /// Registry function name.
        function: String,
    },
    /// Apply an arithmetic or comparison operator, lowering to `BINARY:<op>`.
    ///
    /// Takes exactly two inputs and no numeric parameters.
    Binary(BinaryOperator),
    /// Emit a scalar constant, lowering to a `NUMBER` node.
    ///
    /// Takes no inputs and exactly one parameter.
    Constant,
}

/// One node of a [`FactorGraph`].
///
/// `inputs` are resolved at build time — a name matching another node's id
/// becomes a graph edge, and a name matching a declared external input becomes a
/// bound data series. Node ids are also the names the intermediate series are
/// published under, so a node id shadows an external input of the same name.
#[derive(Clone, Debug, PartialEq)]
pub struct FactorNode {
    id: String,
    operation: FactorOperation,
    inputs: Vec<String>,
    params: Vec<f64>,
}

impl FactorNode {
    /// Start a node calling the registry function `function`.
    #[must_use]
    pub fn new(id: impl Into<String>, function: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            operation: FactorOperation::Call {
                function: function.into(),
            },
            inputs: Vec::new(),
            params: Vec::new(),
        }
    }

    /// Start a node applying a binary operator to two inputs.
    #[must_use]
    pub fn binary(id: impl Into<String>, op: BinaryOperator) -> Self {
        Self {
            id: id.into(),
            operation: FactorOperation::Binary(op),
            inputs: Vec::new(),
            params: Vec::new(),
        }
    }

    /// Start a node holding a scalar constant.
    #[must_use]
    pub fn constant(id: impl Into<String>, value: f64) -> Self {
        Self {
            id: id.into(),
            operation: FactorOperation::Constant,
            inputs: Vec::new(),
            params: vec![value],
        }
    }

    /// Append one input, which may name another node or an external series.
    #[must_use]
    pub fn input(mut self, name: impl Into<String>) -> Self {
        self.inputs.push(name.into());
        self
    }

    /// Append one numeric parameter.
    #[must_use]
    pub fn param(mut self, value: f64) -> Self {
        self.params.push(value);
        self
    }

    /// Node id, which is also the published series name.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// What this node computes.
    #[must_use]
    pub const fn operation(&self) -> &FactorOperation {
        &self.operation
    }

    /// Inputs in argument order.
    #[must_use]
    pub fn inputs(&self) -> &[String] {
        &self.inputs
    }

    /// Numeric parameters in argument order.
    #[must_use]
    pub fn params(&self) -> &[f64] {
        &self.params
    }
}

/// Errors raised while validating and lowering a [`FactorGraph`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FactorGraphError {
    /// Two nodes share an id, so their series could not be told apart.
    DuplicateNode {
        /// The repeated id.
        id: String,
    },
    /// The requested primary output is not a node in the graph.
    UnknownPrimary {
        /// The requested primary id.
        id: String,
    },
    /// A node input is neither another node nor a declared external input.
    ///
    /// This is the typo guard: without it a misspelled node id would be
    /// indistinguishable from an external series and would silently become a
    /// data input.
    UnknownInput {
        /// Node holding the offending input.
        node: String,
        /// The unresolved input name.
        input: String,
    },
    /// A binary or constant node was given the wrong number of inputs.
    InputArity {
        /// Offending node.
        node: String,
        /// How many inputs the node kind requires.
        expected: usize,
        /// How many inputs it was given.
        inputs: usize,
    },
    /// A binary node was given numeric parameters, which it cannot consume.
    UnexpectedParams {
        /// Offending node.
        node: String,
        /// How many parameters it was given.
        params: usize,
    },
    /// A constant node was given the wrong number of numeric parameters.
    ParamArity {
        /// Offending node.
        node: String,
        /// How many parameters the node kind requires.
        expected: usize,
        /// How many parameters it was given.
        params: usize,
    },
    /// [`FactorGraph::from_expression`] met a construct that has no factor-graph
    /// equivalent.
    ///
    /// The supported subset is deliberately narrow: numbers, variable
    /// references, binary operators, and function calls whose arguments are
    /// either sub-expressions or numeric literals. Statements, assignments,
    /// loops, drawing directives and index access are formula-language features
    /// with side effects or control flow, and a factor graph is a pure dataflow
    /// DAG — admitting them here would mean the graph could no longer be
    /// reordered or deduplicated.
    UnsupportedExpression {
        /// The offending construct, in the canonical rendering.
        construct: String,
    },
    /// The expression could not be parsed.
    Parse {
        /// Parser message.
        message: String,
    },
    /// The graph is not acyclic, so no valid evaluation order exists.
    Cycle {
        /// Every node left with an unsatisfied dependency, in declaration order.
        nodes: Vec<String>,
    },
    /// Lowering succeeded but the formula plan compiler rejected the result.
    Plan(FormulaHotPlanError),
    /// A series the compiled plan reads is not available in the supplied context.
    ///
    /// Raised at execution time rather than at build time: a graph declares its
    /// external inputs, but only the context can say whether a series is present.
    MissingInput {
        /// The series the plan asked for and the context could not supply.
        name: String,
    },
    /// A node id was asked for at execution time but is not an output of the plan.
    UnknownNode {
        /// The requested node id.
        id: String,
    },
    /// The compiled plan failed while executing.
    Execution(ExecuteError),
}

impl fmt::Display for FactorGraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateNode { id } => write!(f, "duplicate factor node id `{id}`"),
            Self::UnknownPrimary { id } => write!(f, "primary output `{id}` is not a factor node"),
            Self::UnknownInput { node, input } => write!(
                f,
                "factor node `{node}` references `{input}`, which is neither a node nor a declared input"
            ),
            Self::InputArity {
                node,
                expected,
                inputs,
            } => write!(
                f,
                "factor node `{node}` takes {expected} input(s) but was given {inputs}"
            ),
            Self::UnexpectedParams { node, params } => write!(
                f,
                "factor node `{node}` does not accept numeric parameters but was given {params}"
            ),
            Self::ParamArity {
                node,
                expected,
                params,
            } => write!(
                f,
                "factor node `{node}` takes {expected} parameter(s) but was given {params}"
            ),
            Self::UnsupportedExpression { construct } => write!(
                f,
                "`{construct}` has no factor-graph equivalent: a factor graph is a pure dataflow DAG, \
                 so statements, control flow and drawing directives are out of scope"
            ),
            Self::Parse { message } => write!(f, "factor expression did not parse: {message}"),
            Self::Cycle { nodes } => write!(
                f,
                "factor graph has a dependency cycle among: {}",
                nodes.join(", ")
            ),
            Self::Plan(error) => write!(f, "factor graph plan error: {error}"),
            Self::MissingInput { name } => write!(
                f,
                "factor graph needs input series `{name}`, which the context does not supply"
            ),
            Self::UnknownNode { id } => write!(f, "`{id}` is not an output node of this factor graph"),
            Self::Execution(error) => write!(f, "factor graph execution failed: {error}"),
        }
    }
}

impl std::error::Error for FactorGraphError {}

impl From<FormulaHotPlanError> for FactorGraphError {
    fn from(value: FormulaHotPlanError) -> Self {
        Self::Plan(value)
    }
}

/// Canonical, whitespace-free rendering of an expression, used as a node id.
///
/// Rendering rather than an opaque counter is what makes the generated graph
/// legible: a node is named by the text a human would have written for it, so
/// the deduplication below collapses `Ref($close,1)` written four times into one
/// node *and* leaves the result addressable as `Ref($close,1)`.
fn render_expression(node: &AstNode) -> String {
    match node {
        AstNode::Number(value) => format!("{value}"),
        AstNode::Variable(name) => canonical_name(name),
        AstNode::BinaryOp { op, left, right } => format!(
            "({}{}{})",
            render_expression(left),
            binary_operator_text(op),
            render_expression(right)
        ),
        AstNode::UnaryOp { op, expr } => {
            let symbol = match op {
                UnaryOperator::Neg => "-",
                UnaryOperator::Not => "!",
            };
            format!("({symbol}{})", render_expression(expr))
        }
        AstNode::FunctionCall { name, args } => {
            let rendered: Vec<String> = args.iter().map(render_expression).collect();
            format!("{}({})", canonical_name(name), rendered.join(","))
        }
        // Only reachable on the error path: the emitter rejects these before any
        // rendering is used as an id, and the debug form is what the resulting
        // `UnsupportedExpression` message reports.
        other => format!("{other:?}"),
    }
}

/// Source spelling of a binary operator, for [`render_expression`].
fn binary_operator_text(op: &BinaryOperator) -> &'static str {
    match op {
        BinaryOperator::Add => "+",
        BinaryOperator::Sub => "-",
        BinaryOperator::Mul => "*",
        BinaryOperator::Div => "/",
        BinaryOperator::Mod => "%",
        BinaryOperator::Pow => "^",
        BinaryOperator::StringConcat => "&",
        BinaryOperator::Gt => ">",
        BinaryOperator::Lt => "<",
        BinaryOperator::Gte => ">=",
        BinaryOperator::Lte => "<=",
        BinaryOperator::Eq => "==",
        BinaryOperator::Neq => "!=",
        BinaryOperator::And => "AND",
        BinaryOperator::Or => "OR",
        BinaryOperator::Xor => "XOR",
    }
}

/// Lowers a parsed expression into [`FactorGraph`] nodes.
///
/// Kept as a separate struct rather than a set of free functions because the
/// dedup table has to be threaded through the whole walk.
struct ExpressionEmitter<'a> {
    graph: &'a mut FactorGraph,
    /// Renderings already turned into a node, so a repeated sub-expression is
    /// emitted once and reused by id.
    emitted: BTreeSet<String>,
}

impl ExpressionEmitter<'_> {
    /// Emit `node`, returning the id of the graph node that computes it.
    fn emit(&mut self, node: &AstNode) -> Result<String, FactorGraphError> {
        match node {
            AstNode::Variable(name) => {
                // A bare variable is external data, never a node: the graph has
                // nothing to compute for it, and `build` needs it declared so a
                // typo in a node id is not silently read as a data series.
                let id = canonical_name(name);
                self.graph.declare_input(id.clone());
                Ok(id)
            }
            AstNode::Number(value) => {
                // Only reachable as a whole expression (`"5"`), because a
                // numeric call argument is consumed as a parameter by the
                // `FunctionCall` arm below.
                let id = render_expression(node);
                if self.emitted.insert(id.clone()) {
                    self.graph
                        .add_node(FactorNode::constant(id.clone(), *value))?;
                }
                Ok(id)
            }
            AstNode::BinaryOp { op, left, right } => {
                let id = render_expression(node);
                if self.emitted.contains(&id) {
                    return Ok(id);
                }
                let left_id = self.emit(left)?;
                let right_id = self.emit(right)?;
                self.emitted.insert(id.clone());
                self.graph.add_node(
                    FactorNode::binary(id.clone(), op.clone())
                        .input(left_id)
                        .input(right_id),
                )?;
                Ok(id)
            }
            AstNode::UnaryOp { op, expr } => match op {
                // `-x` has no unary node, so it is stated as `0 - x`. The
                // constant is shared across every negation in the graph.
                UnaryOperator::Neg => {
                    let id = render_expression(node);
                    if self.emitted.contains(&id) {
                        return Ok(id);
                    }
                    let inner = self.emit(expr)?;
                    let zero_id = "0".to_string();
                    if self.emitted.insert(zero_id.clone()) {
                        self.graph
                            .add_node(FactorNode::constant(zero_id.clone(), 0.0))?;
                    }
                    self.emitted.insert(id.clone());
                    self.graph.add_node(
                        FactorNode::binary(id.clone(), BinaryOperator::Sub)
                            .input(zero_id)
                            .input(inner),
                    )?;
                    Ok(id)
                }
                UnaryOperator::Not => Err(FactorGraphError::UnsupportedExpression {
                    construct: render_expression(node),
                }),
            },
            AstNode::FunctionCall { name, args } => {
                let id = render_expression(node);
                if self.emitted.contains(&id) {
                    return Ok(id);
                }
                let mut factor_node = FactorNode::new(id.clone(), canonical_name(name));
                for arg in args {
                    match arg {
                        // A numeric literal is this node's parameter, matching
                        // the `SMA(CLOSE, 20)` argument convention.
                        AstNode::Number(value) => factor_node = factor_node.param(*value),
                        other => {
                            let argument_id = self.emit(other)?;
                            factor_node = factor_node.input(argument_id);
                        }
                    }
                }
                self.emitted.insert(id.clone());
                self.graph.add_node(factor_node)?;
                Ok(id)
            }
            other => Err(FactorGraphError::UnsupportedExpression {
                construct: render_expression(other),
            }),
        }
    }
}

/// A declarative factor graph.
///
/// External data series must be declared before they can be referenced
/// ([`Self::declare_input`]). Declaring them is what lets [`Self::build`] tell a
/// node edge apart from a data binding, and report [`FactorGraphError::UnknownInput`]
/// on a typo instead of quietly compiling it into a new input slot.
#[derive(Clone, Debug, Default)]
pub struct FactorGraph {
    nodes: Vec<FactorNode>,
    inputs: BTreeSet<String>,
    /// Id of the node computing the whole expression, set by
    /// [`Self::from_expression`]. Hand-built graphs leave this `None` and pass
    /// the primary to [`Self::build`] explicitly, because a graph may publish
    /// several outputs and nothing in the structure says which one is wanted.
    expression_primary: Option<String>,
}

impl FactorGraph {
    /// Create an empty graph.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Declare one external data series, such as `CLOSE` or `VOLUME`.
    pub fn declare_input(&mut self, name: impl Into<String>) -> &mut Self {
        self.inputs.insert(canonical_name(&name.into()));
        self
    }

    /// Declare several external data series at once.
    pub fn declare_inputs<I, S>(&mut self, names: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for name in names {
            self.inputs.insert(canonical_name(&name.into()));
        }
        self
    }

    /// Build a graph from a formula expression.
    ///
    /// Every **distinct** sub-expression becomes one node, and the node's id is
    /// that sub-expression's canonical rendering. Two consequences follow, and
    /// both are the point:
    ///
    /// * shared sub-expressions are computed once. Alpha158's `CORD{d}` contains
    ///   `Ref($volume,1)` once and `$volume` twice; a hand-written graph would
    ///   have to remember to share them, whereas this cannot forget.
    /// * every intermediate series stays addressable by the text a human would
    ///   have written for it, so [`FactorGraphPlan::execute_node`] can return
    ///   `Ref($close,1)` without the caller inventing an id.
    ///
    /// Only the pure-dataflow subset of the language is accepted — see
    /// [`FactorGraphError::UnsupportedExpression`]. A `Variable` leaf becomes a
    /// declared external input; a `Number` argument to a call becomes that
    /// node's numeric parameter rather than a constant node, matching the
    /// `SMA(CLOSE, 20)` argument convention every formula function uses.
    ///
    /// # Errors
    ///
    /// [`FactorGraphError::Parse`] if `expression` does not parse, and
    /// [`FactorGraphError::UnsupportedExpression`] for a construct with no
    /// dataflow equivalent.
    pub fn from_expression(expression: &str) -> Result<Self, FactorGraphError> {
        let ast = crate::formula::parse_formula(expression)
            .map_err(|error| FactorGraphError::Parse { message: error })?;
        let mut graph = Self::new();
        let primary = render_expression(&ast);
        let mut emitter = ExpressionEmitter {
            graph: &mut graph,
            emitted: BTreeSet::new(),
        };
        emitter.emit(&ast)?;
        graph.expression_primary = Some(primary);
        Ok(graph)
    }

    /// Id of the node computing the whole expression, when this graph came from
    /// [`Self::from_expression`].
    ///
    /// Hand-built graphs return `None`; they name their primary explicitly.
    #[must_use]
    pub fn expression_primary(&self) -> Option<&str> {
        self.expression_primary.as_deref()
    }

    /// Add a node.
    ///
    /// Declaration order does not need to be a valid evaluation order; nodes are
    /// topologically sorted at build time. It does act as the deterministic
    /// tie-break between nodes that are ready at the same time.
    ///
    /// Ids are compared in canonical form, so `fast` and `FAST` collide here
    /// rather than silently overwriting one another in the plan.
    ///
    /// # Errors
    ///
    /// [`FactorGraphError::DuplicateNode`] if `node`'s canonical id is already
    /// present.
    pub fn add_node(&mut self, node: FactorNode) -> Result<&mut Self, FactorGraphError> {
        let canonical = canonical_name(&node.id);
        if self
            .nodes
            .iter()
            .any(|existing| canonical_name(&existing.id) == canonical)
        {
            return Err(FactorGraphError::DuplicateNode { id: node.id });
        }
        self.nodes.push(node);
        Ok(self)
    }

    /// Look up a node by id.
    #[must_use]
    pub fn node(&self, id: &str) -> Option<&FactorNode> {
        self.nodes.iter().find(|node| node.id == id)
    }

    /// Number of nodes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Whether the graph has no nodes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Declared external inputs the graph references, in canonical form, sorted.
    ///
    /// These are the same names [`FactorGraphPlan::external_inputs`] reports, so
    /// a caller can prepare its data bindings before compiling.
    #[must_use]
    pub fn used_inputs(&self) -> Vec<String> {
        let node_ids: BTreeSet<String> = self
            .nodes
            .iter()
            .map(|node| canonical_name(&node.id))
            .collect();
        let referenced: BTreeSet<String> = self
            .nodes
            .iter()
            .flat_map(|node| node.inputs.iter())
            .map(|input| canonical_name(input))
            .filter(|name| !node_ids.contains(name))
            .collect();
        referenced.into_iter().collect()
    }

    /// Lower the graph to the [`AstNode`] block a formula parser would produce.
    ///
    /// The block defines every node with an assignment, then publishes every
    /// node as a named output — so each series is addressable by id after
    /// execution — and ends with a bare reference to `primary`, which makes
    /// `primary` the formula's result.
    ///
    /// # Errors
    ///
    /// Returns [`FactorGraphError`] for a duplicate id, an unknown primary, an
    /// unresolved input, a node-kind arity violation, or a cycle.
    pub fn to_ast(&self, primary: &str) -> Result<AstNode, FactorGraphError> {
        let (_order, statements) = self.lower(primary)?;
        Ok(AstNode::Statements(statements))
    }

    /// Validate the graph and compile it into a [`FactorGraphPlan`].
    ///
    /// # Errors
    ///
    /// Returns [`FactorGraphError`] for the same reasons as [`Self::to_ast`],
    /// plus [`FactorGraphError::Plan`] if the plan compiler rejects the lowered
    /// AST.
    pub fn build(&self, primary: &str) -> Result<FactorGraphPlan, FactorGraphError> {
        let (order, statements) = self.lower(primary)?;
        let ast = AstNode::Statements(statements);
        let plan = FormulaHotPlan::compile(&ast)?;
        Ok(FactorGraphPlan {
            plan,
            order,
            primary: primary.to_string(),
        })
    }

    /// Shared validation + lowering tail for [`Self::to_ast`] and [`Self::build`].
    ///
    /// Returns the node ids in topological order and the emitted statements.
    fn lower(&self, primary: &str) -> Result<(Vec<String>, Vec<AstNode>), FactorGraphError> {
        let index = self.index_by_id()?;
        if !index.contains_key(&canonical_name(primary)) {
            return Err(FactorGraphError::UnknownPrimary {
                id: primary.to_string(),
            });
        }
        self.validate_inputs(&index)?;
        let order = self.topological_order(&index)?;

        // Each node is defined by an `Assignment`, and that is not cosmetic:
        // `hot_plan::lower_formula_plumbing` records only `ASSIGN:` nodes in its
        // `local_writes` table, so a `VARIABLE:` read of a node resolves to its
        // definition *only* when the definition is an assignment. Publishing a
        // node as an `Output` alone would leave every reference to it looking
        // like an external input — see the module docs.
        let mut statements: Vec<AstNode> = order
            .iter()
            .map(|position| {
                let node = &self.nodes[*position];
                Ok(AstNode::Assignment {
                    name: node.id.clone(),
                    expr: Box::new(Self::node_expression(node)?),
                })
            })
            .collect::<Result<_, FactorGraphError>>()?;

        // Then publish every node as a named output, so each series stays
        // addressable by id after execution rather than only the primary.
        statements.extend(order.iter().map(|position| {
            let id = &self.nodes[*position].id;
            AstNode::Output {
                name: id.clone(),
                expr: Box::new(AstNode::Variable(id.clone())),
                modifier: None,
            }
        }));

        // Finally name the result. This is a trailing bare reference rather than
        // the primary's own output statement, because topological order does not
        // put `primary` last — a node nothing depends on can precede it.
        statements.push(AstNode::Variable(primary.to_string()));

        let ids = order
            .iter()
            .map(|position| self.nodes[*position].id.clone())
            .collect();
        Ok((ids, statements))
    }

    /// Map every node id to its declaration position, rejecting duplicates.
    ///
    /// Keys are canonical, matching the formula layer: two ids that differ only
    /// in case or surrounding whitespace would become one variable in the plan,
    /// so allowing both would let the later definition silently shadow the
    /// earlier one.
    fn index_by_id(&self) -> Result<BTreeMap<String, usize>, FactorGraphError> {
        let mut index = BTreeMap::new();
        for (position, node) in self.nodes.iter().enumerate() {
            if index.insert(canonical_name(&node.id), position).is_some() {
                return Err(FactorGraphError::DuplicateNode {
                    id: node.id.clone(),
                });
            }
        }
        Ok(index)
    }

    /// Reject any input that is neither a node nor a declared external series.
    fn validate_inputs(&self, index: &BTreeMap<String, usize>) -> Result<(), FactorGraphError> {
        for node in &self.nodes {
            for input in &node.inputs {
                if index.contains_key(&canonical_name(input))
                    || self.inputs.contains(&canonical_name(input))
                {
                    continue;
                }
                return Err(FactorGraphError::UnknownInput {
                    node: node.id.clone(),
                    input: input.clone(),
                });
            }
        }
        Ok(())
    }

    /// Order node positions so every dependency precedes its dependents.
    ///
    /// Kahn's algorithm with a `BTreeSet` ready-queue, so the result is a
    /// deterministic function of declaration order rather than of hash order.
    fn topological_order(
        &self,
        index: &BTreeMap<String, usize>,
    ) -> Result<Vec<usize>, FactorGraphError> {
        let count = self.nodes.len();
        let mut dependents: Vec<Vec<usize>> = vec![Vec::new(); count];
        let mut indegree: Vec<usize> = vec![0; count];

        for (position, node) in self.nodes.iter().enumerate() {
            for input in &node.inputs {
                if let Some(&dependency) = index.get(&canonical_name(input)) {
                    dependents[dependency].push(position);
                    indegree[position] += 1;
                }
            }
        }

        let mut ready: BTreeSet<usize> = (0..count)
            .filter(|&position| indegree[position] == 0)
            .collect();
        let mut order = Vec::with_capacity(count);
        while let Some(position) = ready.pop_first() {
            order.push(position);
            for &dependent in &dependents[position] {
                indegree[dependent] -= 1;
                if indegree[dependent] == 0 {
                    ready.insert(dependent);
                }
            }
        }

        if order.len() != count {
            let nodes = (0..count)
                .filter(|&position| indegree[position] > 0)
                .map(|position| self.nodes[position].id.clone())
                .collect();
            return Err(FactorGraphError::Cycle { nodes });
        }
        Ok(order)
    }

    /// Build the expression that computes one node's value.
    fn node_expression(node: &FactorNode) -> Result<AstNode, FactorGraphError> {
        let expression = match &node.operation {
            FactorOperation::Call { function } => {
                let mut args: Vec<AstNode> = node
                    .inputs
                    .iter()
                    .map(|input| AstNode::Variable(input.clone()))
                    .collect();
                args.extend(node.params.iter().map(|param| AstNode::Number(*param)));
                AstNode::FunctionCall {
                    name: function.clone(),
                    args,
                }
            }
            FactorOperation::Binary(op) => {
                let (left, right) = Self::binary_operands(node)?;
                AstNode::BinaryOp {
                    op: op.clone(),
                    left: Box::new(AstNode::Variable(left.clone())),
                    right: Box::new(AstNode::Variable(right.clone())),
                }
            }
            FactorOperation::Constant => {
                if !node.inputs.is_empty() {
                    return Err(FactorGraphError::InputArity {
                        node: node.id.clone(),
                        expected: 0,
                        inputs: node.inputs.len(),
                    });
                }
                let [value] = node.params.as_slice() else {
                    return Err(FactorGraphError::ParamArity {
                        node: node.id.clone(),
                        expected: 1,
                        params: node.params.len(),
                    });
                };
                AstNode::Number(*value)
            }
        };
        Ok(expression)
    }

    /// Validate and destructure the two operands of a binary node.
    fn binary_operands(node: &FactorNode) -> Result<(&String, &String), FactorGraphError> {
        if !node.params.is_empty() {
            return Err(FactorGraphError::UnexpectedParams {
                node: node.id.clone(),
                params: node.params.len(),
            });
        }
        let [left, right] = node.inputs.as_slice() else {
            return Err(FactorGraphError::InputArity {
                node: node.id.clone(),
                expected: 2,
                inputs: node.inputs.len(),
            });
        };
        Ok((left, right))
    }
}

/// A compiled [`FactorGraph`].
///
/// Owns the [`FormulaHotPlan`] the graph lowered to, plus the node metadata a
/// caller needs to address results: the evaluation order and the external series
/// that must be bound before execution.
#[derive(Clone, Debug)]
pub struct FactorGraphPlan {
    plan: FormulaHotPlan,
    order: Vec<String>,
    primary: String,
}

impl FactorGraphPlan {
    /// The compiled plan, ready for [`crate::formula::unified_formula_executor`].
    #[must_use]
    pub const fn plan(&self) -> &FormulaHotPlan {
        &self.plan
    }

    /// Node ids in the evaluation order the graph lowered to.
    #[must_use]
    pub fn order(&self) -> &[String] {
        &self.order
    }

    /// The node whose series is the plan's primary result.
    #[must_use]
    pub fn primary(&self) -> &str {
        &self.primary
    }

    /// External series this plan reads, sorted. Every one must be bound.
    #[must_use]
    pub fn external_inputs(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self
            .plan
            .input_bindings()
            .iter()
            .map(FormulaInputBinding::name)
            .collect();
        names.sort_unstable();
        names.dedup();
        names
    }

    /// Position of a node's series inside an execution result, by node id.
    ///
    /// An [`crate::unified_executor::ExecutionOutput`] returns one buffer per
    /// retained output in plan order, so this index is what a caller needs to
    /// read an intermediate node rather than only the primary result. The id is
    /// canonicalized, so the spelling used at declaration works here.
    ///
    /// Both sides are canonicalized: the binding carries the name *as written*,
    /// because that is the spelling the tree path publishes into
    /// `ctx.output_names`, while a node id may be declared in any case.
    #[must_use]
    pub fn node_index(&self, id: &str) -> Option<usize> {
        let canonical = canonical_name(id);
        let slot = self
            .plan
            .outputs()
            .iter()
            .find(|output| canonical_name(output.name()) == canonical)
            .map(FormulaOutputBinding::slot)?;
        self.plan
            .hot()
            .output_layout()
            .outputs()
            .iter()
            .position(|(_, candidate)| *candidate == slot)
    }

    /// Execute the compiled plan against a formula context.
    ///
    /// This is the seam the module used to be missing: [`Self::plan`] handed the
    /// caller a compiled plan and no way to run it, so every caller had to
    /// re-derive the input-binding loop from this file's own tests. Binding is
    /// therefore **explicit and total**: every input slot declared by
    /// [`Self::external_inputs`] must be present in the context, and an unbound
    /// slot is an error rather than a silently substituted series.
    ///
    /// The returned series are in the plan's retained-output order, which is the
    /// order [`Self::node_index`] indexes — so `values[node_index(id)]` is node
    /// `id`. Prefer [`Self::execute_node`] when only one node is wanted.
    pub fn execute(&self, ctx: &FormulaContext) -> Result<Vec<Vec<f64>>, FactorGraphError> {
        let mut slots: Vec<Option<&[f64]>> = vec![None; self.plan.hot().input_layout().len()];
        for binding in self.plan.input_bindings() {
            let values =
                ctx.get_data(binding.name())
                    .ok_or_else(|| FactorGraphError::MissingInput {
                        name: binding.name().to_string(),
                    })?;
            slots[binding.slot().0] = Some(values);
        }
        let inputs: Vec<&[f64]> = slots
            .into_iter()
            .enumerate()
            .map(|(index, slot)| {
                slot.ok_or_else(|| FactorGraphError::MissingInput {
                    name: format!("input slot {index}"),
                })
            })
            .collect::<Result<_, FactorGraphError>>()?;

        let mut executor = unified_formula_executor(&self.plan);
        let output = executor
            .execute(&inputs)
            .map_err(FactorGraphError::Execution)?;
        Ok(output.values)
    }

    /// Execute the plan and return one node's series by id.
    ///
    /// Reading an intermediate node is the point of a graph rather than a
    /// formula string, so it is a first-class operation instead of a two-step
    /// `execute` + [`Self::node_index`] dance the caller has to get right.
    pub fn execute_node(
        &self,
        ctx: &FormulaContext,
        id: &str,
    ) -> Result<Vec<f64>, FactorGraphError> {
        let index = self
            .node_index(id)
            .ok_or_else(|| FactorGraphError::UnknownNode { id: id.to_string() })?;
        let values = self.execute(ctx)?;
        values
            .into_iter()
            .nth(index)
            .ok_or_else(|| FactorGraphError::UnknownNode { id: id.to_string() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formula::{parse_formula, unified_formula_executor, FormulaContext, FormulaHotPlan};
    use ndarray::Array1;

    /// Deterministic synthetic OHLCV: a rising, oscillating close so that
    /// smoothing kernels produce distinguishable values rather than a constant.
    #[allow(clippy::cast_precision_loss)] // bar counts here are far below 2^53
    fn context(len: usize) -> FormulaContext {
        let series = |offset: f64| {
            Array1::from_iter((0..len).map(|index| {
                let step = index as f64;
                offset + step * 0.5 + (step * 0.37).sin() * 3.0
            }))
        };
        let close = series(100.0);
        FormulaContext::new(
            series(99.0),
            close.clone() + 2.0,
            series(98.0),
            close,
            Array1::from_iter((0..len).map(|index| 1000.0 + index as f64)),
            None,
        )
    }

    /// Drive a compiled plan to its primary series, binding every input slot.
    fn plan_series(plan: &FormulaHotPlan, ctx: &FormulaContext) -> Vec<f64> {
        let mut slots: Vec<Option<&[f64]>> = vec![None; plan.hot().input_layout().len()];
        for binding in plan.input_bindings() {
            let values = ctx
                .get_data(binding.name())
                .unwrap_or_else(|| panic!("missing input series `{}`", binding.name()));
            slots[binding.slot().0] = Some(values);
        }
        let inputs: Vec<&[f64]> = slots
            .into_iter()
            .enumerate()
            .map(|(index, slot)| slot.unwrap_or_else(|| panic!("input slot {index} unbound")))
            .collect();
        let mut executor = unified_formula_executor(plan);
        let output = executor.execute(&inputs).expect("plan executes");
        output
            .values
            .into_iter()
            .next()
            .expect("plan produces a primary series")
    }

    /// Compare two series element-wise, treating two NaNs as equal.
    ///
    /// Warm-up bars are legitimately NaN — `EMA(CLOSE, 26)` has none defined for
    /// its first 25 samples — so a plain `assert_eq!` on the vectors would fail
    /// on a correct result.
    fn assert_same_series(left: &[f64], right: &[f64], tolerance: f64) {
        assert_eq!(
            left.len(),
            right.len(),
            "length mismatch: {} vs {}",
            left.len(),
            right.len()
        );
        for (index, (left, right)) in left.iter().zip(right.iter()).enumerate() {
            let equal = (left.is_nan() && right.is_nan()) || (left - right).abs() <= tolerance;
            assert!(equal, "index {index}: left={left} right={right}");
        }
    }

    /// The reference graph: `(EMA(CLOSE,12) - EMA(CLOSE,26)) / CLOSE`.
    ///
    /// Exercises a call node with a parameter, a two-operand binary node, and a
    /// constant-free ratio — the shape the migration track's single-dependency
    /// scheduler could not express.
    fn macd_like_graph() -> FactorGraph {
        let mut graph = FactorGraph::new();
        graph.declare_input("CLOSE");
        graph
            .add_node(FactorNode::new("fast", "EMA").input("CLOSE").param(12.0))
            .unwrap()
            .add_node(FactorNode::new("slow", "EMA").input("CLOSE").param(26.0))
            .unwrap()
            .add_node(
                FactorNode::binary("diff", BinaryOperator::Sub)
                    .input("fast")
                    .input("slow"),
            )
            .unwrap()
            .add_node(
                FactorNode::binary("primary", BinaryOperator::Div)
                    .input("diff")
                    .input("CLOSE"),
            )
            .unwrap();
        graph
    }

    #[test]
    fn graph_and_equivalent_formula_produce_the_same_series() {
        let ctx = context(160);
        let graph_plan = macd_like_graph().build("primary").unwrap();
        assert_eq!(graph_plan.order(), &["fast", "slow", "diff", "primary"]);
        assert_eq!(graph_plan.external_inputs(), vec!["CLOSE"]);

        let from_graph = plan_series(graph_plan.plan(), &ctx);

        let ast =
            parse_formula("FAST:=EMA(CLOSE,12);SLOW:=EMA(CLOSE,26);(FAST-SLOW)/CLOSE").unwrap();
        let formula_plan = FormulaHotPlan::compile(&ast).unwrap();
        let from_formula = plan_series(&formula_plan, &ctx);

        assert_eq!(from_graph.len(), from_formula.len());
        assert_same_series(&from_graph, &from_formula, 1e-12);
    }

    #[test]
    fn every_node_series_is_addressable_by_id() {
        let ctx = context(160);
        let graph_plan = macd_like_graph().build("primary").unwrap();
        let inputs = [ctx.get_data("CLOSE").unwrap()];
        let mut executor = unified_formula_executor(graph_plan.plan());
        let output = executor.execute(&inputs).unwrap();

        // The primary result is the first retained output...
        assert_eq!(graph_plan.node_index("primary"), Some(0));
        // ...and the intermediates are readable too, which is the point of
        // emitting one named output per node.
        for id in ["fast", "slow", "diff", "primary"] {
            let index = graph_plan
                .node_index(id)
                .unwrap_or_else(|| panic!("{id} not addressable"));
            assert_eq!(output.values[index].len(), ctx.data_len);
        }

        // The published channel really is that node's value, not just a buffer
        // of the right length: `diff` is `fast - slow`.
        let channel = |id: &str| &output.values[graph_plan.node_index(id).unwrap()];
        let expected: Vec<f64> = channel("fast")
            .iter()
            .zip(channel("slow").iter())
            .map(|(fast, slow)| fast - slow)
            .collect();
        assert_same_series(channel("diff"), &expected, 0.0);
    }

    #[test]
    fn declaration_order_does_not_affect_the_result() {
        let ctx = context(120);

        // The same graph, but every node declared before its dependencies.
        let mut graph = FactorGraph::new();
        graph.declare_input("CLOSE");
        graph
            .add_node(
                FactorNode::binary("primary", BinaryOperator::Div)
                    .input("diff")
                    .input("CLOSE"),
            )
            .unwrap()
            .add_node(
                FactorNode::binary("diff", BinaryOperator::Sub)
                    .input("fast")
                    .input("slow"),
            )
            .unwrap()
            .add_node(FactorNode::new("slow", "EMA").input("CLOSE").param(26.0))
            .unwrap()
            .add_node(FactorNode::new("fast", "EMA").input("CLOSE").param(12.0))
            .unwrap();

        let reordered = graph.build("primary").unwrap();
        let reference = macd_like_graph().build("primary").unwrap();

        // Declaration order changes the *emission* order: the tie-break between
        // two simultaneously-ready nodes is declaration index, so `slow`
        // (declared first) precedes `fast` here and follows it in the reference
        // graph. Both orders are valid topological orders.
        assert_eq!(reordered.order(), &["slow", "fast", "diff", "primary"]);
        assert_eq!(reference.order(), &["fast", "slow", "diff", "primary"]);

        // The compiled graph is the same either way, so the series agree.
        assert_same_series(
            &plan_series(reordered.plan(), &ctx),
            &plan_series(reference.plan(), &ctx),
            0.0,
        );
    }

    #[test]
    fn a_cycle_is_reported_by_node_name() {
        let mut graph = FactorGraph::new();
        graph.declare_input("CLOSE");
        graph
            .add_node(
                FactorNode::binary("a", BinaryOperator::Add)
                    .input("b")
                    .input("CLOSE"),
            )
            .unwrap()
            .add_node(
                FactorNode::binary("b", BinaryOperator::Add)
                    .input("a")
                    .input("CLOSE"),
            )
            .unwrap();

        let error = graph.build("a").unwrap_err();
        assert_eq!(
            error,
            FactorGraphError::Cycle {
                nodes: vec!["a".to_string(), "b".to_string()],
            }
        );
        assert!(error.to_string().contains("a, b"));
    }

    #[test]
    fn an_undeclared_input_is_rejected_rather_than_becoming_a_data_series() {
        let mut graph = FactorGraph::new();
        graph.declare_input("CLOSE");
        graph
            .add_node(FactorNode::new("fast", "EMA").input("CLOSE").param(12.0))
            .unwrap()
            // `CLOES` is a typo: without declaration checking it would compile
            // into a new input slot and fail only at binding time, or worse.
            .add_node(FactorNode::new("slow", "EMA").input("CLOES").param(26.0))
            .unwrap();

        assert_eq!(
            graph.build("fast").unwrap_err(),
            FactorGraphError::UnknownInput {
                node: "slow".to_string(),
                input: "CLOES".to_string(),
            }
        );
    }

    #[test]
    fn duplicate_ids_and_unknown_primaries_are_rejected() {
        let mut graph = FactorGraph::new();
        graph.declare_input("CLOSE");
        graph
            .add_node(FactorNode::new("fast", "EMA").input("CLOSE").param(12.0))
            .unwrap();
        assert_eq!(
            graph
                .add_node(FactorNode::new("fast", "SMA").input("CLOSE").param(5.0))
                .unwrap_err(),
            FactorGraphError::DuplicateNode {
                id: "fast".to_string(),
            }
        );
        assert_eq!(
            graph.build("missing").unwrap_err(),
            FactorGraphError::UnknownPrimary {
                id: "missing".to_string(),
            }
        );
    }

    #[test]
    fn node_kind_arity_is_enforced() {
        // Each case needs its own graph: `lower` validates and emits *every*
        // node, so one malformed node fails the build regardless of which
        // primary was requested.
        let mut one_input = FactorGraph::new();
        one_input.declare_input("CLOSE");
        one_input
            .add_node(FactorNode::binary("half", BinaryOperator::Sub).input("CLOSE"))
            .unwrap();
        assert_eq!(
            one_input.build("half").unwrap_err(),
            FactorGraphError::InputArity {
                node: "half".to_string(),
                expected: 2,
                inputs: 1,
            }
        );

        let mut params = FactorGraph::new();
        params.declare_input("CLOSE");
        params
            .add_node(
                FactorNode::binary("shifted", BinaryOperator::Sub)
                    .input("CLOSE")
                    .input("CLOSE")
                    .param(2.0),
            )
            .unwrap();
        assert_eq!(
            params.build("shifted").unwrap_err(),
            FactorGraphError::UnexpectedParams {
                node: "shifted".to_string(),
                params: 1,
            }
        );

        let mut constant = FactorGraph::new();
        constant
            .add_node(FactorNode::constant("k", 1.0).param(2.0))
            .unwrap();
        assert_eq!(
            constant.build("k").unwrap_err(),
            FactorGraphError::ParamArity {
                node: "k".to_string(),
                expected: 1,
                params: 2,
            }
        );
    }

    #[test]
    fn a_constant_node_scales_a_series() {
        let ctx = context(120);
        let mut graph = FactorGraph::new();
        graph.declare_input("CLOSE");
        graph
            .add_node(FactorNode::constant("two", 2.0))
            .unwrap()
            .add_node(FactorNode::new("ma", "SMA").input("CLOSE").param(5.0))
            .unwrap()
            .add_node(
                FactorNode::binary("scaled", BinaryOperator::Mul)
                    .input("two")
                    .input("ma"),
            )
            .unwrap();

        let plan = graph.build("scaled").unwrap();
        assert_eq!(graph.used_inputs(), vec!["CLOSE".to_string()]);
        assert_eq!(plan.external_inputs(), vec!["CLOSE"]);
        assert_eq!(plan.order(), &["two", "ma", "scaled"]);

        let scaled = plan_series(plan.plan(), &ctx);
        let expected: Vec<f64> = plan_series(graph.build("ma").unwrap().plan(), &ctx)
            .iter()
            .map(|value| value * 2.0)
            .collect();
        assert_same_series(&scaled, &expected, 1e-9);
    }

    #[test]
    fn a_node_shadows_an_external_input_of_the_same_name() {
        let ctx = context(120);
        let mut graph = FactorGraph::new();
        graph.declare_input("CLOSE");
        // `base` is a node, so `base` in `squared` is an edge rather than an
        // external series of the same name.
        graph
            .add_node(FactorNode::new("base", "SMA").input("CLOSE").param(10.0))
            .unwrap()
            .add_node(
                FactorNode::binary("squared", BinaryOperator::Mul)
                    .input("base")
                    .input("base"),
            )
            .unwrap();

        let plan = graph.build("squared").unwrap();
        assert_eq!(plan.external_inputs(), vec!["CLOSE"]);
        let squared = plan_series(plan.plan(), &ctx);
        let expected: Vec<f64> = plan_series(graph.build("base").unwrap().plan(), &ctx)
            .iter()
            .map(|value| value * value)
            .collect();
        assert_same_series(&squared, &expected, 1e-9);
    }

    #[test]
    fn ids_that_differ_only_in_case_are_the_same_node() {
        let mut graph = FactorGraph::new();
        graph.declare_input("CLOSE");
        graph
            .add_node(FactorNode::new("fast", "EMA").input("CLOSE").param(12.0))
            .unwrap();
        // The formula layer upper-cases every name, so `FAST` would overwrite
        // `fast`'s definition in the plan rather than adding a second node.
        assert_eq!(
            graph
                .add_node(FactorNode::new("FAST", "SMA").input("CLOSE").param(5.0))
                .unwrap_err(),
            FactorGraphError::DuplicateNode {
                id: "FAST".to_string(),
            }
        );

        // An input reference is canonicalized too, so either spelling resolves.
        let mut lower = FactorGraph::new();
        lower.declare_input("close");
        lower
            .add_node(
                FactorNode::binary("doubled", BinaryOperator::Add)
                    .input("close")
                    .input("CLOSE"),
            )
            .unwrap();
        assert_eq!(
            lower.build("doubled").unwrap().external_inputs(),
            vec!["CLOSE"]
        );
    }
}
