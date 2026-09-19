//! Factor execution engine.

use finkit_series::QuantSeries;
use std::collections::HashMap;
use std::sync::Arc;

use crate::{
    ExecutionPlan, FactorCache, FactorCacheKey, FactorConfig, FactorFactoryRequest, FactorNode,
    FactorOutput, FactorProvider, FactorRegistry, FactorRegistryError, Scheduler, SchedulerError,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeError {
    InvalidInput(String),
    Registry(FactorRegistryError),
    Scheduler(SchedulerError),
    MultipleDependencies { node: String, count: usize },
    MissingDependencyOutput { node: String, dependency: String },
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput(reason) => write!(f, "invalid factor input: {reason}"),
            Self::Registry(error) => error.fmt(f),
            Self::Scheduler(error) => error.fmt(f),
            Self::MultipleDependencies { node, count } => {
                write!(
                    f,
                    "node {node} has {count} dependencies; one input series is required"
                )
            }
            Self::MissingDependencyOutput { node, dependency } => {
                write!(f, "node {node} has no output for dependency {dependency}")
            }
        }
    }
}

impl std::error::Error for RuntimeError {}

impl From<FactorRegistryError> for RuntimeError {
    fn from(error: FactorRegistryError) -> Self {
        Self::Registry(error)
    }
}

impl From<SchedulerError> for RuntimeError {
    fn from(error: SchedulerError) -> Self {
        Self::Scheduler(error)
    }
}

#[derive(Clone, Debug)]
pub struct Executor {
    pub plan: ExecutionPlan,
    pub registry: FactorRegistry,
    pub cache: FactorCache,
}

impl Executor {
    pub fn new(plan: ExecutionPlan, registry: FactorRegistry, cache: FactorCache) -> Self {
        Self {
            plan,
            registry,
            cache,
        }
    }

    pub fn node_count(&self) -> usize {
        self.plan.nodes.len()
    }

    pub fn resolve_factor(
        &self,
        request: &FactorFactoryRequest,
    ) -> Result<crate::DynFactor, RuntimeError> {
        self.registry.create_factor(request).map_err(Into::into)
    }

    pub fn resolve_factory(&self, name: &str) -> Option<Arc<dyn FactorProvider>> {
        self.registry.get_provider(name)
    }

    pub fn resolve_config(&self, request: &FactorFactoryRequest) -> Option<FactorConfig> {
        let mut config = FactorConfig::new(request.name.clone());
        for (key, value) in &request.params {
            config = config.with_param(key.clone(), value.clone());
        }
        self.registry.contains(&config.name).then_some(config)
    }

    /// Execute every graph node in dependency order and return one output per
    /// node. A node without dependencies consumes the external input; a node
    /// with one dependency consumes that dependency's output.
    pub fn execute(&mut self, input: &QuantSeries) -> Result<Vec<FactorOutput>, RuntimeError> {
        let ordered = Scheduler::topological_order(&self.plan.nodes)?;
        let mut outputs = HashMap::with_capacity(ordered.len());
        let mut ordered_outputs = Vec::with_capacity(ordered.len());

        for node in ordered {
            let source = match node.dependencies.as_slice() {
                [] => input,
                [dependency] => outputs
                    .get(dependency)
                    .map(|output: &FactorOutput| &output.series)
                    .ok_or_else(|| RuntimeError::MissingDependencyOutput {
                        node: node.id.clone(),
                        dependency: dependency.clone(),
                    })?,
                dependencies => {
                    return Err(RuntimeError::MultipleDependencies {
                        node: node.id,
                        count: dependencies.len(),
                    });
                }
            };
            let output = self.execute_node(&node, source)?;
            outputs.insert(node.id, output.clone());
            ordered_outputs.push(output);
        }
        Ok(ordered_outputs)
    }

    /// Execute a single node through provider construction and the shared
    /// cache. This is also the extension point for future typed multi-input
    /// and multi-output operation adapters.
    pub fn execute_node(
        &mut self,
        node: &FactorNode,
        input: &QuantSeries,
    ) -> Result<FactorOutput, RuntimeError> {
        if !input.is_valid() {
            return Err(RuntimeError::InvalidInput(
                "timestamps and values must have equal length and strictly increasing timestamps"
                    .to_string(),
            ));
        }
        let mut request = FactorFactoryRequest::new(node.factor_name.clone());
        let mut params: Vec<_> = node.params.iter().collect();
        params.sort_by(|left, right| left.0.cmp(right.0));
        for (key, value) in params {
            request = request.with_param(key.clone(), value.clone());
        }

        let factor = self.registry.create_factor(&request)?;
        let mut key = FactorCacheKey::new(
            input.symbol(),
            node.factor_name.to_ascii_uppercase(),
            request.canonical_params(),
        );
        if let (Some(first), Some(last)) = (input.timestamps().first(), input.timestamps().last()) {
            key = key.with_time_range(format!("{first}:{last}:{}", input.len()));
        }
        self.cache
            .get_or_compute(key, || -> Result<FactorOutput, RuntimeError> {
                Ok(factor.compute(input))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FactorCache, FactorGraph, FactorNode};
    use finkit_array::FloatArray;

    fn input() -> QuantSeries {
        QuantSeries::new(
            "TEST",
            (0..10).collect(),
            FloatArray::new((0..10).map(f64::from).collect()),
        )
    }

    #[test]
    fn executes_provider_graph_and_reuses_cache() {
        let mut graph = FactorGraph::new();
        graph.add_node(FactorNode::new("sma", "SMA").with_param("period", "3"));
        graph.add_node(
            FactorNode::new("ema", "EMA")
                .with_param("period", "2")
                .depends_on("sma"),
        );
        let plan = ExecutionPlan::from_graph("test", &graph);
        let mut executor = Executor::new(plan, FactorRegistry::with_builtin(), FactorCache::new());

        let first = executor.execute(&input()).unwrap();
        assert_eq!(first.len(), 2);
        assert_eq!(first[0].factor_name, "SMA");
        assert_eq!(first[0].series.timestamps(), &[2, 3, 4, 5, 6, 7, 8, 9]);
        assert_eq!(first[1].factor_name, "EMA");
        assert_eq!(first[1].series.timestamps(), &[3, 4, 5, 6, 7, 8, 9]);
        assert_eq!(executor.cache.stats().misses, 2);

        let second = executor.execute(&input()).unwrap();
        assert_eq!(second[1].series.values(), first[1].series.values());
        assert_eq!(executor.cache.stats().hits, 2);
    }

    #[test]
    fn rejects_unknown_and_invalid_factor_requests() {
        let mut executor = Executor::new(
            ExecutionPlan::new("test"),
            FactorRegistry::with_builtin(),
            FactorCache::new(),
        );
        let unknown = FactorNode::new("x", "NOPE");
        assert!(matches!(
            executor.execute_node(&unknown, &input()),
            Err(RuntimeError::Registry(FactorRegistryError::UnknownFactor(
                _
            )))
        ));

        let invalid = FactorNode::new("x", "SMA").with_param("period", "0");
        assert!(matches!(
            executor.execute_node(&invalid, &input()),
            Err(RuntimeError::Registry(FactorRegistryError::Factory(_)))
        ));

        let duplicate = FactorFactoryRequest::new("SMA")
            .with_param("period", "3")
            .with_param("period", "4");
        assert!(matches!(
            executor.resolve_factor(&duplicate),
            Err(RuntimeError::Registry(FactorRegistryError::Factory(_)))
        ));

        let malformed = QuantSeries::new("TEST", vec![0, 0], FloatArray::new(vec![1.0, 2.0]));
        assert!(matches!(
            executor.execute_node(&FactorNode::new("x", "SMA"), &malformed),
            Err(RuntimeError::InvalidInput(_))
        ));
    }
}
