//! §16 semantic graph: the one graph model every frontend lowers into.
//!
//! The V4 plan observed that the crate had accumulated six overlapping graph
//! concepts — `Formula AST`, `ComputeIR`, `Operation`, `FactorGraph`,
//! `FactorPlan`, `HotExecutionPlan` — and that each new cross-cutting concern
//! (dependency dedup, lookback, `DirtyRange`, scheduling, CSE, buffer
//! allocation, metrics, artifact hash) had to be implemented once per concept.
//!
//! The target shape is:
//!
//! ```text
//! Frontend AST  ->  SemanticGraph  ->  ComputePlan
//! ```
//!
//! with `Formula`, `Factor`, `Feature`, and `Composite` reduced to a
//! [`NodeKind`](crate::semantic_graph::NodeKind) — a *label on a node*, not a
//! separate graph type. Everything
//! below the frontend therefore operates on one structure, and every
//! graph-level optimization has exactly one implementation site.
//!
//! # What `NodeKind` is, and is not
//!
//! A kind records **where a node came from**, never **what it computes**. Two
//! graphs that compute the same values but were declared by different
//! frontends are the same graph for every purpose this module serves: they
//! lower to identical plans, schedule into identical levels, and hash to the
//! same [`ArtifactHash`](crate::unified_runtime::ArtifactHash). That is why
//! [`SemanticGraph::content_hash`](crate::semantic_graph::SemanticGraph::content_hash)
//! excludes
//! the kind. If the kind leaked into artifact identity, a formula and a factor
//! computing the same series could never share a cached result, which is
//! precisely the duplication §16 exists to remove.
//!
//! # Ordering is a contract
//!
//! [`SemanticNode::inputs`](crate::semantic_graph::SemanticNode::inputs) is an
//! **ordered operand list**, not a set. Kernels
//! are dispatched positionally, so `A - B` and `B - A`, and `X + X` versus
//! `X`, are different graphs. Every transformation here preserves operand
//! order and operand multiplicity.

use crate::compute::{
    ComputeCapabilities, ComputeEffect, ComputeNode, ComputeNodeId, ComputePlan, ComputePlanError,
    DependencyShape, LookbackRequirement,
};
use crate::registry::FunctionSpec;
use crate::unified_runtime::ArtifactHash;
use std::collections::BTreeMap;

/// Which frontend produced a node.
///
/// See the module docs: a kind is provenance, not semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NodeKind {
    /// A raw input series or cross-sectional field.
    Input,
    /// A literal constant.
    Constant,
    /// A registered indicator or formula primitive.
    Indicator,
    /// A formula expression node.
    Formula,
    /// A factor definition node.
    Factor,
    /// A feature/transform node.
    Feature,
    /// A node spanning several outputs or factors.
    Composite,
}

impl NodeKind {
    /// Stable lowercase label, used by diagnostics and hashing.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Input => "input",
            Self::Constant => "constant",
            Self::Indicator => "indicator",
            Self::Formula => "formula",
            Self::Factor => "factor",
            Self::Feature => "feature",
            Self::Composite => "composite",
        }
    }
}

/// Stable identifier for a node inside one semantic graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SemanticNodeId(pub usize);

/// One node of a semantic graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticNode {
    /// Identifier, unique within the graph.
    pub id: SemanticNodeId,
    /// Which frontend declared this node.
    pub kind: NodeKind,
    /// Canonical operation name or semantic label.
    pub operation: String,
    /// Ordered operand list. Order and multiplicity are both significant.
    pub inputs: Vec<SemanticNodeId>,
    /// Planner-visible execution capabilities.
    pub capabilities: ComputeCapabilities,
}

/// Capabilities for a pure, dependency-free leaf — an input series or a
/// literal.
///
/// Frontends need this for every source node, and hand-writing the struct at
/// each call site is how the `lookback`/`dependency` pair drifts apart. The
/// leaf shape is `FixedLookback(0)` rather than `Dynamic` because a source node
/// genuinely reads no history: it is the one place where a fixed shape is
/// provable without binding anything.
pub fn leaf_capabilities() -> ComputeCapabilities {
    ComputeCapabilities {
        deterministic: true,
        streaming: true,
        stateful: false,
        lookback: LookbackRequirement::None,
        dependency: DependencyShape::FixedLookback(0),
        effect: ComputeEffect::Pure,
    }
}

/// Lower a semantic node into its compute-plan node.
fn lower_node(node: &SemanticNode) -> ComputeNode {
    ComputeNode::new(
        ComputeNodeId(node.id.0),
        node.operation.clone(),
        node.inputs
            .iter()
            .map(|input| ComputeNodeId(input.0))
            .collect(),
        node.capabilities.clone(),
    )
}

/// A directed acyclic graph of semantic operations.
///
/// Construct one through [`SemanticGraph::builder`]; the builder validates
/// structure (unknown operands, duplicated ids, cycles, empty operations) by
/// delegating to [`ComputePlan::compile`], so the graph and the plan cannot
/// disagree about what a valid DAG is.
#[derive(Debug, Clone)]
pub struct SemanticGraph {
    nodes: BTreeMap<SemanticNodeId, SemanticNode>,
    execution_order: Vec<SemanticNodeId>,
    targets: Vec<SemanticNodeId>,
}

impl SemanticGraph {
    /// Start building a graph.
    pub fn builder() -> SemanticGraphBuilder {
        SemanticGraphBuilder::new()
    }

    /// Number of nodes in the graph.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Whether the graph has no nodes.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Look up one node.
    pub fn node(&self, id: SemanticNodeId) -> Option<&SemanticNode> {
        self.nodes.get(&id)
    }

    /// Deterministic topological execution order.
    ///
    /// Inherited from [`ComputePlan::execution_order`], which is what makes it
    /// safe to fold over in order: every operand is visited before its user.
    pub fn execution_order(&self) -> &[SemanticNodeId] {
        &self.execution_order
    }

    /// Nodes the caller asked for, in declaration order and de-duplicated.
    pub fn targets(&self) -> &[SemanticNodeId] {
        &self.targets
    }

    /// Iterate over nodes in ascending id order.
    pub fn iter(&self) -> impl Iterator<Item = &SemanticNode> {
        self.nodes.values()
    }

    /// Lower the graph into an executable [`ComputePlan`].
    ///
    /// This is the whole point of the module: one graph, one plan, one
    /// executor. The lowering is total — every node and every operand order is
    /// carried across unchanged — so a difference between two plans is always a
    /// difference between the two graphs that produced them.
    pub fn lower(&self) -> Result<ComputePlan, ComputePlanError> {
        ComputePlan::compile(self.nodes.values().map(lower_node))
    }

    /// The combined dependency shape of a node *and everything it reads*.
    ///
    /// A node's own [`ComputeCapabilities::dependency`] says how much history
    /// the node itself needs. That is not enough to decide whether the node may
    /// be recomputed over a dirty range: the whole upstream cone has to be
    /// range-safe too, or the recompute would feed off a stale ancestor. This
    /// is the conservative fold §17 mandates, using the one combination rule in
    /// [`DependencyShape::combine`].
    ///
    /// Returns `None` for an id that is not in the graph.
    pub fn dependency_shape(&self, id: SemanticNodeId) -> Option<DependencyShape> {
        self.cone_shapes().get(&id).copied()
    }

    /// Whether the entire graph may be recomputed over a dirty range.
    ///
    /// True only when *every* node's upstream cone is a proven fixed lookback.
    /// One `Dynamic`, `Expanding`, `CrossSectional`, or `Global` node anywhere
    /// in a chain turns the answer to `false` for that chain — and therefore
    /// for the graph.
    pub fn can_execute_range(&self) -> bool {
        self.cone_shapes()
            .values()
            .all(|shape| shape.allows_range_execution())
    }

    /// Fold every node's own shape with its ancestors', in topological order.
    fn cone_shapes(&self) -> BTreeMap<SemanticNodeId, DependencyShape> {
        let mut shapes: BTreeMap<SemanticNodeId, DependencyShape> = BTreeMap::new();
        for &id in &self.execution_order {
            let Some(node) = self.nodes.get(&id) else {
                continue;
            };
            let mut combined = node.capabilities.dependency;
            for input in &node.inputs {
                if let Some(shape) = shapes.get(input) {
                    combined = combined.combine(*shape);
                }
            }
            shapes.insert(id, combined);
        }
        shapes
    }

    /// Remove duplicated pure sub-computations (§20, priority 1).
    ///
    /// Two nodes merge when they have the same operation, the same operands
    /// *after each operand has itself been canonicalized*, and interchangeable
    /// capabilities. The rewrite walks the topological order so the earliest
    /// node in a class becomes the representative, which makes the result
    /// independent of how the map is iterated.
    ///
    /// A node is only eligible when it is pure, stateless, and deterministic.
    /// Each refusal is deliberate and counted in the [`CseReport`]:
    ///
    /// * **impure** — `WriteVariable`, `EmitOutput`, `Draw`, `Stateful`. These
    ///   are observable, so dropping one changes what the caller sees.
    /// * **stateful** — two structurally identical stateful nodes would evolve
    ///   identical state and *could* be merged, but state slots are keyed by
    ///   node identity, so merging renumbers them. That is a real behaviour
    ///   change for a marginal win, so the conservative answer stands until the
    ///   state arena can key by content instead.
    /// * **non-deterministic** — the same inputs may legitimately produce
    ///   different values, so sharing one result would be wrong.
    ///
    /// The rewrite cannot fail: the input graph is already validated, and every
    /// canonicalized operand is emitted before its user because the walk is
    /// topological.
    pub fn eliminate_common_subexpressions(&self) -> CseOutcome {
        let mut report = CseReport {
            nodes_before: self.nodes.len(),
            ..CseReport::default()
        };
        let mut alias: BTreeMap<SemanticNodeId, SemanticNodeId> = BTreeMap::new();
        let mut representative: BTreeMap<String, SemanticNodeId> = BTreeMap::new();
        let mut builder = SemanticGraphBuilder::new();

        for &id in &self.execution_order {
            let Some(node) = self.nodes.get(&id) else {
                continue;
            };
            // Operands are canonicalized first, so `Y = X + X` and `Z = X + X`
            // still merge after `X` itself merged with an earlier twin.
            let inputs: Vec<SemanticNodeId> = node
                .inputs
                .iter()
                .map(|input| alias.get(input).copied().unwrap_or(*input))
                .collect();

            match eligibility(node) {
                Eligibility::Eligible => {
                    let key = cse_key(node, &inputs);
                    if let Some(existing) = representative.get(&key) {
                        alias.insert(id, *existing);
                        report.merged += 1;
                        continue;
                    }
                    let emitted = builder.push(
                        node.kind,
                        node.operation.clone(),
                        inputs,
                        node.capabilities.clone(),
                    );
                    representative.insert(key, emitted);
                    alias.insert(id, emitted);
                }
                Eligibility::Impure => {
                    report.refused_impure += 1;
                    let emitted = builder.push(
                        node.kind,
                        node.operation.clone(),
                        inputs,
                        node.capabilities.clone(),
                    );
                    alias.insert(id, emitted);
                }
                Eligibility::Stateful => {
                    report.refused_stateful += 1;
                    let emitted = builder.push(
                        node.kind,
                        node.operation.clone(),
                        inputs,
                        node.capabilities.clone(),
                    );
                    alias.insert(id, emitted);
                }
                Eligibility::NonDeterministic => {
                    report.refused_nondeterministic += 1;
                    let emitted = builder.push(
                        node.kind,
                        node.operation.clone(),
                        inputs,
                        node.capabilities.clone(),
                    );
                    alias.insert(id, emitted);
                }
            }
        }

        // A target that merged away resolves to its representative, and two
        // targets that merged into one collapse to a single request.
        for target in &self.targets {
            builder.target(alias.get(target).copied().unwrap_or(*target));
        }

        let graph = builder.build().expect(
            "CSE rewrites a validated DAG into a validated DAG: operands are \
             remapped only to already-emitted nodes and operations are unchanged",
        );
        report.nodes_after = graph.len();
        CseOutcome { graph, report }
    }

    /// Group nodes into dependency levels for the scheduler.
    ///
    /// A node's level is one past the deepest level among its operands, so all
    /// operands of a node sit in strictly earlier levels. Nodes in the same
    /// level share no dependency path and may therefore be dispatched together.
    ///
    /// This is a **grouping**, not a reordering. Within a level the graph's own
    /// topological order is preserved, which keeps the flattened schedule a
    /// valid topological order *and* keeps observable effects in their original
    /// relative order. Flattening the levels is how a caller that cannot use
    /// the parallelism still gets a correct order.
    pub fn levels(&self) -> Vec<Vec<SemanticNodeId>> {
        let mut level_of: BTreeMap<SemanticNodeId, usize> = BTreeMap::new();
        let mut depth = 0usize;

        for &id in &self.execution_order {
            let Some(node) = self.nodes.get(&id) else {
                continue;
            };
            let mut level = 0usize;
            for input in &node.inputs {
                if let Some(&input_level) = level_of.get(input) {
                    level = level.max(input_level + 1);
                }
            }
            level_of.insert(id, level);
            depth = depth.max(level + 1);
        }

        let mut levels = vec![Vec::new(); depth];
        for &id in &self.execution_order {
            if let Some(&level) = level_of.get(&id) {
                levels[level].push(id);
            }
        }
        levels
    }

    /// Flatten [`Self::levels`] back into one deterministic topological order.
    pub fn scheduled_order(&self) -> Vec<SemanticNodeId> {
        self.levels().into_iter().flatten().collect()
    }

    /// Deterministic content identity for the *values* this graph computes.
    ///
    /// Built from the topological order, each node's operation, its operands,
    /// and the capabilities that affect the result. The [`NodeKind`] is
    /// deliberately excluded: see the module docs. Node ids are included via
    /// the operand list, so the canonical insertion order is part of the
    /// identity — two graphs that are isomorphic but built in different orders
    /// are honestly reported as different, rather than being claimed equal by a
    /// canonicalization this module does not perform.
    pub fn content_hash(&self) -> ArtifactHash {
        let mut hash = ArtifactHash::empty();
        for &id in &self.execution_order {
            let Some(node) = self.nodes.get(&id) else {
                continue;
            };
            hash = hash.chain(node.operation.as_bytes()).chain(b"\x1f");
            for input in &node.inputs {
                hash = hash.chain(input.0.to_string().as_bytes()).chain(b",");
            }
            hash = hash.chain(b"\x1f");
            hash = hash.chain(dependency_key(&node.capabilities).as_bytes());
            hash = hash.chain(b"\x1e");
        }
        for target in &self.targets {
            hash = hash.chain(target.0.to_string().as_bytes()).chain(b",");
        }
        hash
    }
}

/// Whether a node may participate in CSE, and if not, why not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Eligibility {
    Eligible,
    Impure,
    Stateful,
    NonDeterministic,
}

fn eligibility(node: &SemanticNode) -> Eligibility {
    if !node.capabilities.effect.is_pure() {
        return Eligibility::Impure;
    }
    if node.capabilities.stateful {
        return Eligibility::Stateful;
    }
    if !node.capabilities.deterministic {
        return Eligibility::NonDeterministic;
    }
    Eligibility::Eligible
}

/// Canonical text for the capabilities that make two nodes non-interchangeable.
///
/// Everything except [`ComputeCapabilities::effect`] is included. Effect is
/// omitted because only pure nodes reach this encoding, so it would only ever
/// contribute the constant `Pure`; including it would advertise a distinction
/// the key cannot actually express.
fn cse_key(node: &SemanticNode, inputs: &[SemanticNodeId]) -> String {
    let mut key = String::with_capacity(node.operation.len() + 32 + inputs.len() * 4);
    key.push_str(&node.operation);
    key.push('\u{1f}');
    key.push_str(&dependency_key(&node.capabilities));
    key.push('\u{1f}');
    key.push_str(if node.capabilities.streaming {
        "s"
    } else {
        "-"
    });
    key.push('\u{1f}');
    for input in inputs {
        key.push_str(&input.0.to_string());
        key.push(',');
    }
    key
}

/// Canonical text for the lookback/dependency pair.
///
/// [`DependencyShape::label`] alone is not enough: it renders every
/// `FixedLookback` as `"fixed-lookback"`, so `FixedLookback(3)` and
/// `FixedLookback(5)` would collide and CSE would merge two nodes that read
/// different windows. The row count is appended for exactly that reason.
fn dependency_key(capabilities: &ComputeCapabilities) -> String {
    let mut key = String::with_capacity(32);
    key.push_str(match capabilities.lookback {
        LookbackRequirement::None => "none",
        LookbackRequirement::PeriodMinusOne => "period-minus-one",
        LookbackRequirement::Period => "period",
        LookbackRequirement::Fixed(_) => "fixed",
        LookbackRequirement::Dynamic => "dynamic",
    });
    if let LookbackRequirement::Fixed(rows) = capabilities.lookback {
        key.push(':');
        key.push_str(&rows.to_string());
    }
    key.push('|');
    key.push_str(capabilities.dependency.label());
    if let Some(rows) = capabilities.dependency.fixed_lookback() {
        key.push(':');
        key.push_str(&rows.to_string());
    }
    key
}

/// What a CSE pass changed, and what it declined to change.
///
/// The refusal counters exist so the gate can assert the pass is not vacuous:
/// a CSE that silently stopped merging anything would otherwise look identical
/// to a CSE that had nothing to merge.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CseReport {
    /// Nodes in the input graph.
    pub nodes_before: usize,
    /// Nodes in the output graph.
    pub nodes_after: usize,
    /// Nodes folded into an earlier equivalent node.
    pub merged: usize,
    /// Nodes kept because they carry an observable effect.
    pub refused_impure: usize,
    /// Nodes kept because they carry mutable state.
    pub refused_stateful: usize,
    /// Nodes kept because they are not deterministic.
    pub refused_nondeterministic: usize,
}

impl CseReport {
    /// Whether the pass removed anything.
    pub const fn changed(self) -> bool {
        self.merged > 0
    }

    /// Nodes the pass removed.
    pub const fn removed(self) -> usize {
        self.nodes_before.saturating_sub(self.nodes_after)
    }
}

/// The rewritten graph produced by [`SemanticGraph::eliminate_common_subexpressions`].
#[derive(Debug, Clone)]
pub struct CseOutcome {
    /// Graph with duplicated pure sub-computations folded together.
    pub graph: SemanticGraph,
    /// What the pass did.
    pub report: CseReport,
}

/// Incremental builder for a [`SemanticGraph`].
///
/// Ids are assigned in push order, so a frontend that walks its own AST
/// deterministically produces a deterministic graph.
#[derive(Debug, Default)]
pub struct SemanticGraphBuilder {
    nodes: Vec<SemanticNode>,
    targets: Vec<SemanticNodeId>,
}

impl SemanticGraphBuilder {
    /// Create an empty builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a node and return its id.
    pub fn push(
        &mut self,
        kind: NodeKind,
        operation: impl Into<String>,
        inputs: Vec<SemanticNodeId>,
        capabilities: ComputeCapabilities,
    ) -> SemanticNodeId {
        let id = SemanticNodeId(self.nodes.len());
        self.nodes.push(SemanticNode {
            id,
            kind,
            operation: operation.into(),
            inputs,
            capabilities,
        });
        id
    }

    /// Append a node whose capabilities come from the canonical registry.
    ///
    /// Routing through [`ComputeCapabilities::from_function_spec`] is what
    /// keeps a frontend from inventing its own lookback/dependency pair for an
    /// operation the registry already describes.
    pub fn push_function(
        &mut self,
        kind: NodeKind,
        operation: impl Into<String>,
        inputs: Vec<SemanticNodeId>,
        spec: &FunctionSpec,
    ) -> SemanticNodeId {
        self.push(
            kind,
            operation,
            inputs,
            ComputeCapabilities::from_function_spec(spec),
        )
    }

    /// Append a pure, dependency-free source node.
    pub fn push_leaf(&mut self, kind: NodeKind, operation: impl Into<String>) -> SemanticNodeId {
        self.push(kind, operation, Vec::new(), leaf_capabilities())
    }

    /// Declare a node as a caller-visible result. Duplicates are ignored.
    pub fn target(&mut self, id: SemanticNodeId) {
        if !self.targets.contains(&id) {
            self.targets.push(id);
        }
    }

    /// Validate and freeze the graph.
    ///
    /// Validation is [`ComputePlan::compile`] itself — unknown operands,
    /// duplicated ids, empty operations, and cycles are all reported with the
    /// same error type and the same messages the plan uses.
    pub fn build(self) -> Result<SemanticGraph, ComputePlanError> {
        let plan = ComputePlan::compile(self.nodes.iter().map(lower_node))?;
        let execution_order: Vec<SemanticNodeId> = plan
            .execution_order()
            .iter()
            .map(|id| SemanticNodeId(id.0))
            .collect();
        debug_assert_eq!(
            execution_order.len(),
            self.nodes.len(),
            "a compiled plan covers every node it was given"
        );

        let nodes: BTreeMap<SemanticNodeId, SemanticNode> =
            self.nodes.into_iter().map(|node| (node.id, node)).collect();
        let targets = self
            .targets
            .into_iter()
            .filter(|id| nodes.contains_key(id))
            .collect();

        Ok(SemanticGraph {
            nodes,
            execution_order,
            targets,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::builtin_function_registry;
    use std::collections::BTreeSet;

    /// A leaf input series.
    fn input(builder: &mut SemanticGraphBuilder, name: &str) -> SemanticNodeId {
        builder.push_leaf(NodeKind::Input, name)
    }

    /// A pure node with an explicit fixed lookback.
    fn fixed(
        builder: &mut SemanticGraphBuilder,
        kind: NodeKind,
        operation: &str,
        inputs: Vec<SemanticNodeId>,
        lookback: usize,
    ) -> SemanticNodeId {
        builder.push(
            kind,
            operation,
            inputs,
            ComputeCapabilities {
                deterministic: true,
                streaming: true,
                stateful: false,
                lookback: LookbackRequirement::Fixed(lookback),
                dependency: DependencyShape::FixedLookback(lookback),
                effect: ComputeEffect::Pure,
            },
        )
    }

    /// Render a plan into a comparable, id-agnostic fingerprint.
    fn plan_fingerprint(plan: &ComputePlan) -> Vec<String> {
        plan.execution_order()
            .iter()
            .map(|id| {
                let node = plan
                    .node(*id)
                    .expect("execution order only lists plan nodes");
                let operands: Vec<String> = node
                    .dependencies
                    .iter()
                    .map(|dep| dep.0.to_string())
                    .collect();
                format!("{} <- [{}]", node.operation, operands.join(","))
            })
            .collect()
    }

    /// `MA(C,20) + MA(C,20)` — the canonical shared intermediate.
    fn duplicated_intermediate_graph(
        kind: NodeKind,
    ) -> (SemanticGraph, SemanticNodeId, SemanticNodeId) {
        let mut builder = SemanticGraph::builder();
        let close = input(&mut builder, "CLOSE");
        let first = fixed(&mut builder, kind, "MA", vec![close], 19);
        let second = fixed(&mut builder, kind, "MA", vec![close], 19);
        let sum = fixed(&mut builder, kind, "ADD", vec![first, second], 19);
        builder.target(sum);
        let graph = builder.build().expect("the graph is a DAG");
        (graph, first, second)
    }

    #[test]
    fn graph_validates_unknown_operands_and_cycles() {
        let mut builder = SemanticGraph::builder();
        let dangling = SemanticNodeId(41);
        builder.push(
            NodeKind::Formula,
            "ADD",
            vec![dangling],
            leaf_capabilities(),
        );
        assert!(matches!(
            builder
                .build()
                .expect_err("an unknown operand must be rejected"),
            ComputePlanError::UnknownDependency { .. }
        ));

        // Self-reference is a cycle of length one.
        let mut builder = SemanticGraph::builder();
        let self_referential = SemanticNodeId(0);
        builder.push(
            NodeKind::Formula,
            "SUM",
            vec![self_referential],
            leaf_capabilities(),
        );
        assert!(matches!(
            builder.build().expect_err("a self-reference is a cycle"),
            ComputePlanError::DependencyCycle(_)
        ));

        let mut builder = SemanticGraph::builder();
        builder.push(NodeKind::Formula, "   ", Vec::new(), leaf_capabilities());
        assert!(matches!(
            builder.build().expect_err("a blank operation is rejected"),
            ComputePlanError::EmptyOperation(_)
        ));
    }

    /// §16: the frontend label is not a semantic difference.
    ///
    /// The same arithmetic declared as a formula, a factor, and a composite
    /// must lower to the same plan and hash to the same artifact identity.
    /// This is the gate that stops `NodeKind` from quietly becoming six graph
    /// types again.
    #[test]
    fn every_frontend_kind_lowers_to_the_same_plan_for_the_same_semantics() {
        let fingerprints: Vec<Vec<String>> = [
            NodeKind::Formula,
            NodeKind::Factor,
            NodeKind::Feature,
            NodeKind::Composite,
        ]
        .into_iter()
        .map(|kind| {
            let (graph, _, _) = duplicated_intermediate_graph(kind);
            let plan = graph.lower().expect("a validated graph lowers");
            plan_fingerprint(&plan)
        })
        .collect();

        for fingerprint in &fingerprints[1..] {
            assert_eq!(
                fingerprint, &fingerprints[0],
                "the same semantics must lower to the same plan regardless of NodeKind"
            );
        }

        let hashes: Vec<ArtifactHash> = [
            NodeKind::Formula,
            NodeKind::Factor,
            NodeKind::Feature,
            NodeKind::Composite,
        ]
        .into_iter()
        .map(|kind| duplicated_intermediate_graph(kind).0.content_hash())
        .collect();
        for hash in &hashes[1..] {
            assert_eq!(
                hash, &hashes[0],
                "artifact identity must not depend on which frontend declared the node"
            );
        }
    }

    #[test]
    fn cse_folds_duplicated_pure_intermediates() {
        let (graph, _, _) = duplicated_intermediate_graph(NodeKind::Factor);
        let outcome = graph.eliminate_common_subexpressions();

        assert_eq!(outcome.report.nodes_before, 4, "CLOSE, MA, MA, ADD");
        assert_eq!(outcome.report.nodes_after, 3, "one MA survives");
        assert_eq!(outcome.report.merged, 1);
        assert_eq!(outcome.report.removed(), 1);
        assert!(outcome.report.changed());

        // The surviving ADD still reads the shared MA twice, because operand
        // multiplicity is significant: `MA + MA` is not `MA`.
        let add = outcome
            .graph
            .iter()
            .find(|node| node.operation == "ADD")
            .expect("ADD survives CSE");
        assert_eq!(add.inputs.len(), 2);
        assert_eq!(add.inputs[0], add.inputs[1]);
    }

    #[test]
    fn cse_is_idempotent_and_preserves_validation() {
        let (graph, _, _) = duplicated_intermediate_graph(NodeKind::Factor);
        let once = graph.eliminate_common_subexpressions();
        let twice = once.graph.eliminate_common_subexpressions();

        assert!(
            !twice.report.changed(),
            "a second pass has nothing left to do"
        );
        assert_eq!(twice.graph.len(), once.graph.len());
        assert_eq!(
            twice.graph.content_hash(),
            once.graph.content_hash(),
            "a no-op pass must not disturb artifact identity"
        );
        assert!(
            once.graph.lower().is_ok(),
            "the rewritten graph still lowers"
        );
    }

    #[test]
    fn cse_refuses_impure_stateful_and_nondeterministic_nodes() {
        let pure_fixed = ComputeCapabilities {
            deterministic: true,
            streaming: true,
            stateful: false,
            lookback: LookbackRequirement::Fixed(19),
            dependency: DependencyShape::FixedLookback(19),
            effect: ComputeEffect::Pure,
        };

        let mut builder = SemanticGraph::builder();
        let close = input(&mut builder, "CLOSE");

        // Observable effect: two identical assignments must both survive, or
        // the caller loses one of two writes they asked for.
        for _ in 0..2 {
            builder.push(
                NodeKind::Formula,
                "MA",
                vec![close],
                ComputeCapabilities {
                    effect: ComputeEffect::WriteVariable("x".to_string()),
                    ..pure_fixed.clone()
                },
            );
        }

        // Mutable state: state slots are keyed by node identity.
        for _ in 0..2 {
            builder.push(
                NodeKind::Indicator,
                "EMA",
                vec![close],
                ComputeCapabilities {
                    stateful: true,
                    ..pure_fixed.clone()
                },
            );
        }

        // Non-deterministic: sharing one result would be wrong.
        for _ in 0..2 {
            builder.push(
                NodeKind::Feature,
                "RANDOM",
                vec![close],
                ComputeCapabilities {
                    deterministic: false,
                    ..pure_fixed.clone()
                },
            );
        }

        let graph = builder.build().expect("the graph is a DAG");
        let outcome = graph.eliminate_common_subexpressions();

        assert_eq!(outcome.report.refused_impure, 2);
        assert_eq!(outcome.report.refused_stateful, 2);
        assert_eq!(outcome.report.refused_nondeterministic, 2);
        assert_eq!(outcome.report.merged, 0);
        assert_eq!(outcome.graph.len(), graph.len());
        assert!(!outcome.report.changed());
    }

    #[test]
    fn cse_keeps_different_lookbacks_apart() {
        let mut builder = SemanticGraph::builder();
        let close = input(&mut builder, "CLOSE");
        fixed(&mut builder, NodeKind::Factor, "MA", vec![close], 5);
        fixed(&mut builder, NodeKind::Factor, "MA", vec![close], 20);
        let graph = builder.build().expect("the graph is a DAG");

        let outcome = graph.eliminate_common_subexpressions();
        assert_eq!(
            outcome.report.merged, 0,
            "MA(5) and MA(20) read different windows and must not merge"
        );
        assert_eq!(outcome.graph.len(), 3);
    }

    #[test]
    fn cse_remaps_targets_and_collapses_duplicate_requests() {
        let mut builder = SemanticGraph::builder();
        let close = input(&mut builder, "CLOSE");
        let left = fixed(&mut builder, NodeKind::Factor, "MA", vec![close], 19);
        let right = fixed(&mut builder, NodeKind::Factor, "MA", vec![close], 19);
        builder.target(left);
        builder.target(right);
        let graph = builder.build().expect("the graph is a DAG");
        assert_eq!(
            graph.targets().len(),
            2,
            "two distinct nodes were requested"
        );

        let outcome = graph.eliminate_common_subexpressions();
        assert_eq!(outcome.report.merged, 1);
        assert_eq!(
            outcome.graph.targets().len(),
            1,
            "two requests that folded into one node are one request"
        );
        let target = outcome.graph.targets()[0];
        assert!(
            outcome.graph.node(target).is_some(),
            "the remapped target must resolve inside the rewritten graph"
        );
    }

    #[test]
    fn cone_shape_is_conservative_over_the_whole_upstream_chain() {
        let mut builder = SemanticGraph::builder();
        let close = input(&mut builder, "CLOSE");
        let short = fixed(&mut builder, NodeKind::Factor, "MA", vec![close], 3);
        let long = fixed(&mut builder, NodeKind::Factor, "STD", vec![short], 5);
        let graph = builder.build().expect("the graph is a DAG");

        assert_eq!(
            graph.dependency_shape(short),
            Some(DependencyShape::FixedLookback(3))
        );
        assert_eq!(
            graph.dependency_shape(long),
            Some(DependencyShape::FixedLookback(5)),
            "a fixed chain folds to the widest window, not the sum: each node \
             reads its own window off its operand's already-computed row"
        );
        assert!(graph.can_execute_range());

        // One dynamic node anywhere in the chain disqualifies its whole cone.
        let mut builder = SemanticGraph::builder();
        let close = input(&mut builder, "CLOSE");
        let fixed_node = fixed(&mut builder, NodeKind::Factor, "MA", vec![close], 3);
        let dynamic_node = builder.push(
            NodeKind::Composite,
            "RANK",
            vec![fixed_node],
            ComputeCapabilities {
                lookback: LookbackRequirement::Dynamic,
                dependency: DependencyShape::CrossSectional,
                ..leaf_capabilities()
            },
        );
        let reader = fixed(&mut builder, NodeKind::Factor, "MA", vec![dynamic_node], 2);
        let graph = builder.build().expect("the graph is a DAG");

        assert_eq!(
            graph.dependency_shape(dynamic_node),
            Some(DependencyShape::Dynamic)
        );
        assert_eq!(
            graph.dependency_shape(reader),
            Some(DependencyShape::Dynamic),
            "a fixed node reading a cross-sectional parent is not range-safe"
        );
        assert!(!graph.can_execute_range());
        assert_eq!(graph.dependency_shape(SemanticNodeId(999)), None);
    }

    #[test]
    fn levels_are_a_valid_topological_schedule() {
        let mut builder = SemanticGraph::builder();
        let close = input(&mut builder, "CLOSE");
        let a = fixed(&mut builder, NodeKind::Factor, "MA", vec![close], 19);
        let b = fixed(&mut builder, NodeKind::Factor, "STD", vec![close], 19);
        let c = fixed(&mut builder, NodeKind::Factor, "ADD", vec![a, b], 19);
        let d = fixed(&mut builder, NodeKind::Factor, "MUL", vec![a], 19);
        let graph = builder.build().expect("the graph is a DAG");

        let levels = graph.levels();
        assert_eq!(
            levels.len(),
            3,
            "CLOSE | MA, STD | ADD, MUL — MUL reads MA, so it lands below it"
        );
        assert!(levels.iter().all(|level| !level.is_empty()));

        let scheduled = graph.scheduled_order();
        assert_eq!(
            scheduled.len(),
            graph.len(),
            "every node is scheduled exactly once"
        );
        let unique: BTreeSet<SemanticNodeId> = scheduled.iter().copied().collect();
        assert_eq!(unique.len(), graph.len(), "no node is scheduled twice");

        // Every operand must precede its user in the flattened schedule.
        let position: BTreeMap<SemanticNodeId, usize> = scheduled
            .iter()
            .enumerate()
            .map(|(index, id)| (*id, index))
            .collect();
        for node in graph.iter() {
            for operand in &node.inputs {
                assert!(
                    position[operand] < position[&node.id],
                    "{:?} must be scheduled before {:?}",
                    operand,
                    node.id
                );
            }
        }

        // Independent siblings share a level.
        let level_of = |needle: SemanticNodeId| {
            levels
                .iter()
                .position(|level| level.contains(&needle))
                .expect("every node sits in a level")
        };
        assert_eq!(level_of(a), level_of(b), "both read only CLOSE");
        assert_eq!(level_of(c), level_of(d), "both read only level-1 nodes");
        assert!(level_of(a) < level_of(c), "ADD reads MA");
        assert!(level_of(a) < level_of(d), "MUL reads MA");
    }

    #[test]
    fn scheduling_never_reorders_observable_effects() {
        let mut builder = SemanticGraph::builder();
        let close = input(&mut builder, "CLOSE");
        let pure_a = fixed(&mut builder, NodeKind::Factor, "MA", vec![close], 19);
        let pure_b = fixed(&mut builder, NodeKind::Factor, "STD", vec![close], 19);
        // Two effect nodes that are genuinely independent — the scheduler must
        // group them but must not swap them.
        let first_write = builder.push(
            NodeKind::Formula,
            "ASSIGN_X",
            vec![pure_a],
            ComputeCapabilities {
                effect: ComputeEffect::WriteVariable("x".to_string()),
                ..leaf_capabilities()
            },
        );
        let second_write = builder.push(
            NodeKind::Formula,
            "ASSIGN_Y",
            vec![pure_b],
            ComputeCapabilities {
                effect: ComputeEffect::WriteVariable("y".to_string()),
                ..leaf_capabilities()
            },
        );
        let graph = builder.build().expect("the graph is a DAG");
        let level_of = |needle: SemanticNodeId| {
            graph
                .levels()
                .iter()
                .position(|level| level.contains(&needle))
                .expect("every node sits in a level")
        };
        assert_eq!(
            level_of(first_write),
            level_of(second_write),
            "the two writes depend on disjoint operands and share a level"
        );

        let scheduled = graph.scheduled_order();
        let effects: Vec<SemanticNodeId> = scheduled
            .iter()
            .copied()
            .filter(|id| {
                graph
                    .node(*id)
                    .is_some_and(|node| !node.capabilities.effect.is_pure())
            })
            .collect();
        assert_eq!(
            effects,
            vec![first_write, second_write],
            "the scheduler must preserve the declared order of observable effects"
        );
    }

    #[test]
    fn content_hash_tracks_structure_not_labels() {
        let (baseline, _, _) = duplicated_intermediate_graph(NodeKind::Factor);
        let baseline_hash = baseline.content_hash();
        assert_eq!(baseline_hash, baseline.content_hash(), "hashing is stable");
        assert_eq!(baseline_hash.to_string().len(), 16);

        // A different window is a different value.
        let mut builder = SemanticGraph::builder();
        let close = input(&mut builder, "CLOSE");
        fixed(&mut builder, NodeKind::Factor, "MA", vec![close], 20);
        let wider = builder.build().expect("the graph is a DAG");
        assert_ne!(wider.content_hash(), baseline_hash);

        // A different operand order is a different value.
        let mut builder = SemanticGraph::builder();
        let close = input(&mut builder, "CLOSE");
        let a = fixed(&mut builder, NodeKind::Factor, "MA", vec![close], 19);
        let b = fixed(&mut builder, NodeKind::Factor, "STD", vec![close], 19);
        fixed(&mut builder, NodeKind::Factor, "SUB", vec![a, b], 19);
        let subtract = builder.build().expect("the graph is a DAG");

        let mut builder = SemanticGraph::builder();
        let close = input(&mut builder, "CLOSE");
        let a = fixed(&mut builder, NodeKind::Factor, "MA", vec![close], 19);
        let b = fixed(&mut builder, NodeKind::Factor, "STD", vec![close], 19);
        fixed(&mut builder, NodeKind::Factor, "SUB", vec![b, a], 19);
        let reversed = builder.build().expect("the graph is a DAG");

        assert_ne!(
            subtract.content_hash(),
            reversed.content_hash(),
            "SUB is not commutative, so swapping operands must change identity"
        );
    }

    #[test]
    fn registry_backed_nodes_inherit_the_canonical_capabilities() {
        let registry = builtin_function_registry();
        let spec = registry.get("SMA").expect("SMA is registered");

        let mut builder = SemanticGraph::builder();
        let close = input(&mut builder, "CLOSE");
        let sma = builder.push_function(NodeKind::Indicator, "SMA", vec![close], spec);
        let graph = builder.build().expect("the graph is a DAG");

        let node = graph.node(sma).expect("the node exists");
        assert_eq!(
            node.capabilities,
            ComputeCapabilities::from_function_spec(spec),
            "a registry-backed node must not invent its own capabilities"
        );
        assert_eq!(
            node.capabilities.dependency,
            spec.dependency(),
            "the §15 registry answer reaches the plan node"
        );
    }

    #[test]
    fn an_empty_graph_is_valid_and_has_a_stable_identity() {
        let graph = SemanticGraph::builder()
            .build()
            .expect("an empty graph is trivially a DAG");
        assert!(graph.is_empty());
        assert_eq!(graph.len(), 0);
        assert!(graph.execution_order().is_empty());
        assert!(graph.targets().is_empty());
        assert!(graph.levels().is_empty());
        assert!(graph.can_execute_range());
        assert!(graph.lower().is_ok());
        assert_eq!(graph.content_hash(), ArtifactHash::empty());
    }
}
