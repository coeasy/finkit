//! Execution sandbox: configurable timeout, recursion depth, and memory limits.

use std::cell::RefCell;
use std::time::{Duration, Instant};

use crate::formula::types::FormulaError;

/// Configurable limits for formula execution. `None` means unlimited.
#[derive(Debug, Clone, Copy, Default)]
pub struct ExecSandboxConfig {
    /// Wall-clock timeout in milliseconds.
    pub timeout_ms: Option<u64>,
    /// Maximum AST evaluation recursion depth.
    pub max_recursion_depth: Option<usize>,
    /// Approximate heap budget in bytes (tracked on array materialization).
    pub max_memory_bytes: Option<usize>,
}

impl ExecSandboxConfig {
    /// No limits — default sandbox configuration.
    pub fn unlimited() -> Self {
        Self::default()
    }

    pub fn with_timeout_ms(mut self, ms: u64) -> Self {
        self.timeout_ms = Some(ms);
        self
    }

    pub fn with_max_recursion_depth(mut self, depth: usize) -> Self {
        self.max_recursion_depth = Some(depth);
        self
    }

    pub fn with_max_memory_bytes(mut self, bytes: usize) -> Self {
        self.max_memory_bytes = Some(bytes);
        self
    }
}

/// Per-execution mutable state (reset before each top-level eval).
#[derive(Debug, Default, Clone)]
pub struct ExecSandboxState {
    recursion_depth: usize,
    bytes_tracked: usize,
    started_at: Option<Instant>,
    active: bool,
}

impl ExecSandboxState {
    pub fn recursion_depth(&self) -> usize {
        self.recursion_depth
    }

    pub fn reset(&mut self) {
        self.recursion_depth = 0;
        self.bytes_tracked = 0;
        self.started_at = None;
        self.active = false;
    }
}

/// RAII guard that decrements recursion depth on drop.
pub struct SandboxDepthGuard<'a> {
    state: &'a RefCell<ExecSandboxState>,
}

impl<'a> std::fmt::Debug for SandboxDepthGuard<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SandboxDepthGuard").finish_non_exhaustive()
    }
}

impl<'a> Drop for SandboxDepthGuard<'a> {
    fn drop(&mut self) {
        let mut st = self.state.borrow_mut();
        st.recursion_depth = st.recursion_depth.saturating_sub(1);
    }
}

/// Begin (or continue) sandbox tracking for a top-level execution.
pub fn sandbox_reset(state: &RefCell<ExecSandboxState>) {
    state.borrow_mut().reset();
}

/// Enter one level of AST evaluation; returns `Err` when a limit is exceeded.
pub fn sandbox_enter<'a>(
    config: &ExecSandboxConfig,
    state: &'a RefCell<ExecSandboxState>,
) -> Result<SandboxDepthGuard<'a>, FormulaError> {
    sandbox_push_depth(config, state)?;
    Ok(SandboxDepthGuard { state })
}

fn sandbox_push_depth(
    config: &ExecSandboxConfig,
    state: &RefCell<ExecSandboxState>,
) -> Result<(), FormulaError> {
    let mut st = state.borrow_mut();
    if !st.active {
        st.active = true;
        st.started_at = Some(Instant::now());
    }

    if let Some(limit) = config.timeout_ms {
        if let Some(start) = st.started_at {
            if start.elapsed() > Duration::from_millis(limit) {
                return Err(FormulaError::RuntimeError(format!(
                    "Sandbox timeout exceeded ({} ms)",
                    limit
                )));
            }
        }
    }

    st.recursion_depth += 1;
    if let Some(max) = config.max_recursion_depth {
        if st.recursion_depth > max {
            return Err(FormulaError::RuntimeError(format!(
                "Sandbox recursion depth exceeded (max {})",
                max
            )));
        }
    }

    Ok(())
}

/// Push sandbox depth without holding an RAII guard across `ctx` mutation.
pub fn sandbox_push(
    config: &ExecSandboxConfig,
    state: &RefCell<ExecSandboxState>,
) -> Result<(), FormulaError> {
    sandbox_push_depth(config, state)
}

/// Pop sandbox depth after evaluation.
pub fn sandbox_pop(state: &RefCell<ExecSandboxState>) {
    let mut st = state.borrow_mut();
    st.recursion_depth = st.recursion_depth.saturating_sub(1);
}

/// Track approximate bytes allocated during execution.
pub fn sandbox_track_bytes(
    config: &ExecSandboxConfig,
    state: &RefCell<ExecSandboxState>,
    bytes: usize,
) -> Result<(), FormulaError> {
    let mut st = state.borrow_mut();
    st.bytes_tracked += bytes;
    if let Some(max) = config.max_memory_bytes {
        if st.bytes_tracked > max {
            return Err(FormulaError::RuntimeError(format!(
                "Sandbox memory limit exceeded ({} bytes > {} bytes)",
                st.bytes_tracked, max
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formula::engine::FormulaEngine;
    use crate::formula::types::FormulaContext;
    use ndarray::Array1;

    fn make_ctx(len: usize) -> FormulaContext {
        let open = Array1::from_vec((0..len).map(|i| 10.0 + i as f64 * 0.1).collect());
        let high = Array1::from_vec((0..len).map(|i| 11.0 + i as f64 * 0.2).collect());
        let low = Array1::from_vec((0..len).map(|i| 9.0 + i as f64 * 0.1).collect());
        let close = Array1::from_vec((0..len).map(|i| 10.0 + i as f64 * 0.15).collect());
        let volume = Array1::from_vec((0..len).map(|i| 1000.0 + i as f64 * 10.0).collect());
        FormulaContext::new(open, high, low, close, volume, None)
    }

    #[test]
    fn sandbox_limits_timeout_enforced() {
        let config = ExecSandboxConfig::default().with_timeout_ms(1);
        let state = RefCell::new(ExecSandboxState::default());
        {
            let mut st = state.borrow_mut();
            st.active = true;
            st.started_at = Some(Instant::now() - Duration::from_millis(5));
        }
        let result = sandbox_enter(&config, &state);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("timeout"),
            "expected timeout limit error, got: {err}"
        );
    }

    #[test]
    fn sandbox_enter_limits_at_depth() {
        let config = ExecSandboxConfig::default().with_max_recursion_depth(8);
        let state = RefCell::new(ExecSandboxState::default());
        let mut guards = Vec::new();
        for _ in 0..8 {
            guards.push(sandbox_enter(&config, &state).unwrap());
        }
        assert!(sandbox_enter(&config, &state).is_err());
    }

    #[test]
    fn sandbox_limits_recursion_depth_enforced() {
        let mut ctx = make_ctx(10);
        ctx.sandbox = ExecSandboxConfig::default().with_max_recursion_depth(8);
        let mut engine = FormulaEngine::new();
        // Deeply nested MA calls exceed recursion budget.
        let source = "MA(MA(MA(MA(MA(MA(MA(MA(MA(MA(CLOSE,2),2),2),2),2),2),2),2),2),2)";
        let result = engine.eval(source, &mut ctx);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("recursion depth"),
            "expected recursion limit error, got: {err}"
        );
    }

    #[test]
    fn sandbox_limits_memory_enforced() {
        let mut ctx = make_ctx(100);
        ctx.sandbox = ExecSandboxConfig::default().with_max_memory_bytes(64);
        let mut engine = FormulaEngine::new();
        let result = engine.eval("MA(CLOSE, 5)", &mut ctx);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("memory limit"),
            "expected memory limit error, got: {err}"
        );
    }

    #[test]
    fn sandbox_limits_unlimited_passes() {
        let mut ctx = make_ctx(30);
        ctx.sandbox = ExecSandboxConfig::unlimited();
        let mut engine = FormulaEngine::new();
        let result = engine.eval("MA(CLOSE, 5)", &mut ctx);
        assert!(result.is_ok());
    }

    #[test]
    fn sandbox_limits_config_defaults_are_unlimited() {
        let cfg = ExecSandboxConfig::default();
        assert!(cfg.timeout_ms.is_none());
        assert!(cfg.max_recursion_depth.is_none());
        assert!(cfg.max_memory_bytes.is_none());
    }
}
