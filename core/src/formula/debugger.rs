use crate::formula::ast::AstNode;
use crate::formula::executor::FormulaExecutor;
use crate::formula::types::*;
use ndarray::Array1;
use std::sync::Arc;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "type", rename_all = "camelCase"))]
pub enum DebugEvent {
    StepStart {
        line: usize,
        node_type: String,
    },
    VariableSet {
        name: String,
        values: Vec<f64>,
    },
    FunctionCall {
        name: String,
        arg_count: usize,
    },
    FunctionReturn {
        name: String,
        result_len: usize,
    },
    Error {
        message: String,
        line: usize,
        column: usize,
    },
}

pub struct FormulaDebugger {
    events: Vec<DebugEvent>,
    watch_variables: Vec<String>,
    trace_enabled: bool,
}

impl Default for FormulaDebugger {
    fn default() -> Self {
        Self::new()
    }
}

impl FormulaDebugger {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            watch_variables: Vec::new(),
            trace_enabled: false,
        }
    }

    pub fn add_watch(&mut self, name: &str) {
        if !self.watch_variables.contains(&name.to_string()) {
            self.watch_variables.push(name.to_string());
        }
    }

    pub fn enable_trace(&mut self) {
        self.trace_enabled = true;
    }

    pub fn disable_trace(&mut self) {
        self.trace_enabled = false;
    }

    pub fn step(
        &mut self,
        ast: &AstNode,
        ctx: &mut FormulaContext,
        executor: &FormulaExecutor,
    ) -> Result<Array1<f64>, FormulaError> {
        if self.trace_enabled {
            self.record_step_start(ast, 0);
        }

        let result = executor.execute(ast, ctx);

        match &result {
            Ok(_values) => {
                if self.trace_enabled {
                    self.record_variable_changes(ctx);
                }
            }
            Err(e) => {
                if self.trace_enabled {
                    self.events.push(DebugEvent::Error {
                        message: e.to_string(),
                        line: 0,
                        column: 0,
                    });
                }
            }
        }

        result
    }

    pub fn run_with_debug(
        &mut self,
        ast: &AstNode,
        ctx: &mut FormulaContext,
        executor: &FormulaExecutor,
    ) -> Result<Array1<f64>, FormulaError> {
        if self.trace_enabled {
            self.collect_debug_events(ast, ctx, executor)
        } else {
            executor.execute(ast, ctx)
        }
    }

    pub fn get_events(&self) -> &[DebugEvent] {
        &self.events
    }

    pub fn clear_events(&mut self) {
        self.events.clear();
    }

    pub fn get_variable_snapshot<'a>(
        &self,
        name: &str,
        ctx: &'a FormulaContext,
    ) -> Option<&'a Array1<f64>> {
        ctx.get_data(name)
    }

    pub fn get_last_event(&self) -> Option<&DebugEvent> {
        self.events.last()
    }

    pub fn get_events_by_type(&self, event_type: &str) -> Vec<&DebugEvent> {
        self.events
            .iter()
            .filter(|e| {
                matches!(
                    (e, event_type),
                    (DebugEvent::StepStart { .. }, "StepStart")
                        | (DebugEvent::VariableSet { .. }, "VariableSet")
                        | (DebugEvent::FunctionCall { .. }, "FunctionCall")
                        | (DebugEvent::FunctionReturn { .. }, "FunctionReturn")
                        | (DebugEvent::Error { .. }, "Error")
                )
            })
            .collect()
    }

    pub fn is_watching(&self, name: &str) -> bool {
        self.watch_variables
            .iter()
            .any(|v| v.eq_ignore_ascii_case(name))
    }

    fn record_step_start(&mut self, ast: &AstNode, line: usize) {
        let node_type = self.get_node_type_name(ast);
        self.events.push(DebugEvent::StepStart { line, node_type });
    }

    fn record_variable_changes(&mut self, ctx: &FormulaContext) {
        for var_name in &self.watch_variables {
            if let Some(values) = ctx.get_data(var_name) {
                self.events.push(DebugEvent::VariableSet {
                    name: var_name.clone(),
                    values: values.iter().copied().collect(),
                });
            }
        }
    }

    fn collect_debug_events(
        &mut self,
        ast: &AstNode,
        ctx: &mut FormulaContext,
        executor: &FormulaExecutor,
    ) -> Result<Array1<f64>, FormulaError> {
        self.record_step_start(ast, 0);

        let result = match ast {
            AstNode::Statements(stmts) => {
                let mut last_result = Array1::zeros(ctx.data_len);
                for stmt in stmts {
                    self.record_step_start(stmt, 0);
                    let r = self.collect_debug_events(stmt, ctx, executor)?;
                    last_result = r;
                }
                Ok(last_result)
            }
            AstNode::Assignment { name, expr } => {
                let value = self.collect_debug_events(expr, ctx, executor)?;
                ctx.variables.insert(Arc::from(name.clone()), value.clone());
                self.events.push(DebugEvent::VariableSet {
                    name: name.clone(),
                    values: value.iter().copied().take(10).collect(),
                });
                Ok(value)
            }
            AstNode::Output {
                name,
                expr,
                modifier: _,
            } => {
                let value = self.collect_debug_events(expr, ctx, executor)?;
                ctx.variables.insert(Arc::from(name.clone()), value.clone());
                Ok(value)
            }
            AstNode::FunctionCall { name, args } => {
                self.events.push(DebugEvent::FunctionCall {
                    name: name.clone(),
                    arg_count: args.len(),
                });
                let result = executor.execute(ast, ctx);
                if let Ok(ref values) = result {
                    self.events.push(DebugEvent::FunctionReturn {
                        name: name.clone(),
                        result_len: values.len(),
                    });
                }
                result
            }
            _ => executor.execute(ast, ctx),
        };

        if self.trace_enabled {
            self.record_variable_changes(ctx);
        }

        result
    }

    fn get_node_type_name(&self, ast: &AstNode) -> String {
        match ast {
            AstNode::Number(_) => "Number".to_string(),
            AstNode::StringLit(_) => "StringLit".to_string(),
            AstNode::Variable(_) => "Variable".to_string(),
            AstNode::BinaryOp { op, .. } => format!("BinaryOp({:?})", op),
            AstNode::UnaryOp { op, .. } => format!("UnaryOp({:?})", op),
            AstNode::FunctionCall { name, .. } => format!("FunctionCall({})", name),
            AstNode::IndexAccess { .. } => "IndexAccess".to_string(),
            AstNode::Assignment { name, .. } => format!("Assignment({})", name),
            AstNode::CompoundAssignment { name, op, .. } => {
                format!("CompoundAssignment({},{:?})", name, op)
            }
            AstNode::Output { name, .. } => format!("Output({})", name),
            AstNode::Statements(s) => format!("Statements({} items)", s.len()),
            AstNode::ParamDecl { name, .. } => format!("ParamDecl({})", name),
            AstNode::DrawText { .. } => "DrawText".to_string(),
            AstNode::DrawIcon { .. } => "DrawIcon".to_string(),
            AstNode::StickLine { .. } => "StickLine".to_string(),
            AstNode::DrawGeneric { command, .. } => format!("DrawGeneric({})", command),
            AstNode::IfThenElse { .. } => "IfThenElse".to_string(),
            AstNode::ForLoop { .. } => "ForLoop".to_string(),
            AstNode::WhileLoop { .. } => "WhileLoop".to_string(),
        }
    }
}

pub struct FormulaErrorWithLocation {
    pub error: FormulaError,
    pub line: usize,
    pub column: usize,
}

impl std::fmt::Display for FormulaErrorWithLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}:{}] {}", self.line, self.column, self.error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formula::parser::parse_formula;

    fn make_ctx(len: usize) -> FormulaContext {
        let open = Array1::from_vec((0..len).map(|i| 10.0 + i as f64 * 0.1).collect());
        let high = Array1::from_vec((0..len).map(|i| 11.0 + i as f64 * 0.2).collect());
        let low = Array1::from_vec((0..len).map(|i| 9.0 + i as f64 * 0.1).collect());
        let close = Array1::from_vec((0..len).map(|i| 10.0 + i as f64 * 0.15).collect());
        let volume = Array1::from_vec((0..len).map(|i| 1000.0 + i as f64 * 10.0).collect());
        FormulaContext::new(open, high, low, close, volume, None)
    }

    fn parse(source: &str) -> AstNode {
        parse_formula(source).expect("Failed to parse formula")
    }

    #[test]
    fn test_debugger_new() {
        let debugger = FormulaDebugger::new();
        assert!(debugger.get_events().is_empty());
        assert!(!debugger.trace_enabled);
        assert!(debugger.watch_variables.is_empty());
    }

    #[test]
    fn test_debugger_add_watch() {
        let mut debugger = FormulaDebugger::new();
        debugger.add_watch("CLOSE");
        assert!(debugger.is_watching("CLOSE"));
        assert!(!debugger.is_watching("OPEN"));
    }

    #[test]
    fn test_debugger_add_watch_duplicate() {
        let mut debugger = FormulaDebugger::new();
        debugger.add_watch("CLOSE");
        debugger.add_watch("CLOSE");
        assert_eq!(debugger.watch_variables.len(), 1);
    }

    #[test]
    fn test_debugger_enable_disable_trace() {
        let mut debugger = FormulaDebugger::new();
        assert!(!debugger.trace_enabled);
        debugger.enable_trace();
        assert!(debugger.trace_enabled);
        debugger.disable_trace();
        assert!(!debugger.trace_enabled);
    }

    #[test]
    fn test_debugger_step_basic() {
        let mut debugger = FormulaDebugger::new();
        let mut ctx = make_ctx(5);
        let executor = FormulaExecutor::new();
        let ast = parse("10 + 20");

        let result = debugger.step(&ast, &mut ctx, &executor).unwrap();
        for i in 0..5 {
            assert!((result[i] - 30.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_debugger_step_with_trace() {
        let mut debugger = FormulaDebugger::new();
        debugger.enable_trace();
        let mut ctx = make_ctx(5);
        let executor = FormulaExecutor::new();
        let ast = parse("10 + 20");

        let _result = debugger.step(&ast, &mut ctx, &executor).unwrap();
        assert!(!debugger.get_events().is_empty());

        let has_step_start = debugger
            .get_events()
            .iter()
            .any(|e| matches!(e, DebugEvent::StepStart { .. }));
        assert!(has_step_start);
    }

    #[test]
    fn test_debugger_run_with_debug() {
        let mut debugger = FormulaDebugger::new();
        debugger.enable_trace();
        debugger.add_watch("CLOSE");
        let mut ctx = make_ctx(5);
        let executor = FormulaExecutor::new();
        let ast = parse("CLOSE + 1");

        let result = debugger.run_with_debug(&ast, &mut ctx, &executor).unwrap();
        assert_eq!(result.len(), 5);
        assert!(!debugger.get_events().is_empty());
    }

    #[test]
    fn test_debugger_get_variable_snapshot() {
        let debugger = FormulaDebugger::new();
        let ctx = make_ctx(5);

        let snapshot = debugger.get_variable_snapshot("CLOSE", &ctx);
        assert!(snapshot.is_some());
        assert_eq!(snapshot.unwrap().len(), 5);

        let snapshot_none = debugger.get_variable_snapshot("NONEXISTENT", &ctx);
        assert!(snapshot_none.is_none());
    }

    #[test]
    fn test_debugger_get_events_by_type() {
        let mut debugger = FormulaDebugger::new();
        debugger.enable_trace();
        let mut ctx = make_ctx(5);
        let executor = FormulaExecutor::new();
        let ast = parse("MA(CLOSE, 3)");

        let _ = debugger.run_with_debug(&ast, &mut ctx, &executor);

        let step_events = debugger.get_events_by_type("StepStart");
        assert!(!step_events.is_empty());
    }

    #[test]
    fn test_debugger_clear_events() {
        let mut debugger = FormulaDebugger::new();
        debugger.enable_trace();
        let mut ctx = make_ctx(5);
        let executor = FormulaExecutor::new();
        let ast = parse("1 + 2");

        let _ = debugger.run_with_debug(&ast, &mut ctx, &executor);
        assert!(!debugger.get_events().is_empty());

        debugger.clear_events();
        assert!(debugger.get_events().is_empty());
    }

    #[test]
    fn test_debugger_get_last_event() {
        let mut debugger = FormulaDebugger::new();
        assert!(debugger.get_last_event().is_none());

        debugger.events.push(DebugEvent::StepStart {
            line: 1,
            node_type: "Number".to_string(),
        });
        assert!(debugger.get_last_event().is_some());
    }

    #[test]
    fn test_debugger_assignment_tracking() {
        let mut debugger = FormulaDebugger::new();
        debugger.enable_trace();
        debugger.add_watch("UP");
        let mut ctx = make_ctx(5);
        let executor = FormulaExecutor::new();
        let ast = parse("UP := CLOSE + 1");

        let result = debugger.run_with_debug(&ast, &mut ctx, &executor).unwrap();
        assert_eq!(result.len(), 5);

        let var_set_events = debugger.get_events_by_type("VariableSet");
        assert!(!var_set_events.is_empty());
    }

    #[test]
    fn test_debugger_function_call_events() {
        let mut debugger = FormulaDebugger::new();
        debugger.enable_trace();
        let mut ctx = make_ctx(10);
        let executor = FormulaExecutor::new();
        let ast = parse("MA(CLOSE, 3)");

        let _ = debugger.run_with_debug(&ast, &mut ctx, &executor);

        let func_call_events = debugger.get_events_by_type("FunctionCall");
        assert!(!func_call_events.is_empty());

        let func_return_events = debugger.get_events_by_type("FunctionReturn");
        assert!(!func_return_events.is_empty());
    }

    #[test]
    fn test_debugger_error_event() {
        let mut debugger = FormulaDebugger::new();
        debugger.enable_trace();
        let mut ctx = make_ctx(5);
        let executor = FormulaExecutor::new();
        let ast = parse("UNKNOWN_FUNCTION(CLOSE)");

        let result = debugger.step(&ast, &mut ctx, &executor);
        assert!(result.is_err());

        let error_events = debugger.get_events_by_type("Error");
        assert!(!error_events.is_empty());
    }

    #[test]
    fn test_debugger_multiple_statements() {
        let mut debugger = FormulaDebugger::new();
        debugger.enable_trace();
        debugger.add_watch("MA5");
        debugger.add_watch("MA10");
        let mut ctx = make_ctx(30);
        let executor = FormulaExecutor::new();
        let ast = parse("MA5 := MA(CLOSE, 5); MA10 := MA(CLOSE, 10); MA5 > MA10");

        let result = debugger.run_with_debug(&ast, &mut ctx, &executor).unwrap();
        assert_eq!(result.len(), 30);

        let events = debugger.get_events();
        assert!(events.len() > 3);
    }
}
