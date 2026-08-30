//! Library-wide metrics façade (O-2).
//!
//! Centralises the metric names + labels used across the library so that the
//! Prometheus exporter (or any other recorder) sees a consistent schema.
//!
//! The macros are no-ops unless the user installs a recorder — see
//! [`metrics::set_global_recorder`]. With the `metrics-prometheus` feature,
//! the `prometheus_recorder()` helper below installs a default recorder and
//! returns a handle whose `render()` produces the scrape text.
//!
//! ## Metric inventory
//!
//! | Name                              | Kind      | Labels                |
//! |-----------------------------------|-----------|-----------------------|
//! | `indicator_compute_total`         | counter   | `name`                |
//! | `indicator_duration_seconds`      | histogram | `name`                |
//! | `indicator_input_rejected_total`  | counter   | `name`, `reason`      |
//! | `formula_eval_total`              | counter   | `result` (ok/error)  |
//! | `formula_duration_seconds`        | histogram |                       |
//! | `formula_errors_total`            | counter   | `kind`                |
//! | `streaming_next_total`            | counter   | `name`, `ready`       |
//!
//! All of the macros below are gated on the `metrics` feature so that
//! downstream `no_std` builds don't pull in the crate.

/// Increment the per-indicator call counter.
///
/// Use [`record_indicator_duration`] to log the elapsed time in the same call.
#[cfg(feature = "metrics")]
#[inline]
pub fn indicator_called(name: &'static str) {
    metrics::counter!("indicator_compute_total", "name" => name).increment(1);
}

/// Record how long a batch indicator call took, in seconds.
#[cfg(feature = "metrics")]
#[inline]
pub fn record_indicator_duration(name: &'static str, seconds: f64) {
    metrics::histogram!("indicator_duration_seconds", "name" => name).record(seconds);
}

/// Increment the per-formula eval counter. `ok = true` on success, `false`
/// when the engine returned any error.
#[cfg(feature = "metrics")]
#[inline]
pub fn formula_eval(ok: bool) {
    metrics::counter!("formula_eval_total", "result" => if ok { "ok" } else { "error" })
        .increment(1);
}

/// Record how long a formula evaluation took, in seconds.
#[cfg(feature = "metrics")]
#[inline]
pub fn record_formula_duration(seconds: f64) {
    metrics::histogram!("formula_duration_seconds").record(seconds);
}

/// Increment the formula-error counter with a categorical `kind` label.
#[cfg(feature = "metrics")]
#[inline]
pub fn formula_error(kind: &'static str) {
    metrics::counter!("formula_errors_total", "kind" => kind).increment(1);
}

/// Increment the rejected-input counter. `reason` should be one of a small
/// set of stable labels (`nan_input`, `inf_input`, `zero_period`,
/// `dimension_mismatch`).
#[cfg(feature = "metrics")]
#[inline]
pub fn input_rejected(name: &'static str, reason: &'static str) {
    metrics::counter!("indicator_input_rejected_total", "name" => name, "reason" => reason)
        .increment(1);
}

/// Increment the streaming-next counter. `ready = true` once the indicator
/// has produced its first value.
#[cfg(feature = "metrics")]
#[inline]
pub fn streaming_next(name: &'static str, ready: bool) {
    metrics::counter!(
        "streaming_next_total",
        "name" => name,
        "ready" => if ready { "true" } else { "false" }
    )
    .increment(1);
}

/// Wrap the body of a streaming `next()` call: starts a timer, executes the
/// body, records duration + counter + ready flag.  No-op when the `metrics`
/// feature is disabled (the body is still evaluated so callers don't need
/// to special-case build configurations).
///
/// # Example
///
/// ```ignore
/// use finkit::metrics::streaming_measure;
/// fn next(&mut self, input: f64) -> Option<f64> {
///     streaming_measure!("sma", self.count, {
///         self.count += 1;
///         self.sum += input;
///         if self.len == self.period { Some(self.sum / self.period as f64) } else { None }
///     })
/// }
/// ```
#[cfg(feature = "metrics")]
#[macro_export]
macro_rules! streaming_measure {
    ($name:expr, $count:expr, $body:expr) => {{
        let __start = $crate::metrics::__std::time::Instant::now();
        let __result = { $body };
        $crate::metrics::streaming_next($name, $crate::metrics::option_is_some(&__result));
        $crate::metrics::record_indicator_duration(
            $name,
            __start.elapsed().as_secs_f64(),
        );
        let _ = $count; // currently unused; reserved for per-period metrics
        __result
    }};
}

/// No-op fallback when the `metrics` feature is disabled. Always defined
/// (not feature-gated) so that downstream targets without `std::time` —
/// notably `wasm32-unknown-unknown` — still see a `crate::streaming_measure!`
/// symbol at the crate root.
#[cfg(not(feature = "metrics"))]
#[macro_export]
macro_rules! streaming_measure {
    ($name:expr, $count:expr, $body:expr) => {{
        let _ = $count;
        $body
    }};
}

/// Helper used by `streaming_measure!` to convert `Option<T>` to a `bool`.
/// Always available (no feature gate) because the macro is.
#[doc(hidden)]
#[inline]
pub fn option_is_some<T>(o: &Option<T>) -> bool {
    o.is_some()
}

/// Time the execution of `$body`, increment the per-name counter, and record
/// the duration histogram. No-op when the `metrics` feature is disabled.
///
/// # Example
///
/// ```ignore
/// use finkit::metrics::timed;
/// let r = timed("rsi", || { /* computation */ 42.0 });
/// ```
#[cfg(feature = "metrics")]
#[macro_export]
macro_rules! timed {
    ($name:expr, $body:expr) => {{
        let __start = $crate::metrics::__std::time::Instant::now();
        let __result = { $body };
        $crate::metrics::indicator_called($name);
        $crate::metrics::record_indicator_duration(
            $name,
            __start.elapsed().as_secs_f64(),
        );
        __result
    }};
}

/// No-op fallback when the `metrics` feature is disabled. The expression is
/// still evaluated so callers don't need to special-case build configurations.
#[cfg(not(feature = "metrics"))]
#[macro_export]
macro_rules! timed {
    ($name:expr, $body:expr) => {{ $body }};
}

/// Statically-resolved path to `std` for use by the `timed!` macro. When the
/// `metrics` feature is disabled this resolves to `core`; otherwise it
/// resolves to `std`.
#[doc(hidden)]
pub mod __std {
    #[cfg(feature = "metrics")]
    pub use std::time;

    // Provide a no-op `time::Instant` shape when `metrics` is off so the
    // macro is still type-correct. The `timed!` macro is gated on the
    // `metrics` feature, so this path is never actually executed.
    #[cfg(not(feature = "metrics"))]
    pub mod time {
        pub struct Instant;
        impl Instant {
            pub fn now() -> Self { Instant }
            pub fn elapsed(&self) -> core::time::Duration { core::time::Duration::from_secs(0) }
        }
    }
}

/// Install a Prometheus recorder as the global default. Returns a handle
/// whose `render()` produces the scrape text. Requires the
/// `metrics-prometheus` feature.
#[cfg(feature = "metrics-prometheus")]
pub fn prometheus_recorder() -> Result<
    metrics_exporter_prometheus::PrometheusHandle,
    metrics_exporter_prometheus::BuildError,
> {
    use std::sync::OnceLock;
    static HANDLE: OnceLock<metrics_exporter_prometheus::PrometheusHandle> = OnceLock::new();
    if let Some(h) = HANDLE.get() {
        return Ok(h.clone());
    }
    let builder = metrics_exporter_prometheus::PrometheusBuilder::new();
    let handle = builder.install_recorder()?;
    let _ = HANDLE.set(handle.clone());
    Ok(handle)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Smoke test: the macro site is callable even without an installed
    // recorder (it silently no-ops).
    #[test]
    #[cfg(feature = "metrics")]
    fn macro_sites_compile_without_recorder() {
        indicator_called("sma");
        record_indicator_duration("sma", 0.001);
        formula_eval(true);
        record_formula_duration(0.002);
        formula_error("parse");
        input_rejected("sma", "nan_input");
        streaming_next("sma", true);
    }

    // The streaming_measure! macro should evaluate its body and return it
    // unchanged even without a recorder installed.
    #[test]
    fn streaming_measure_returns_body() {
        // Path 1: no `metrics` feature — body is the only thing evaluated.
        #[cfg(not(feature = "metrics"))]
        {
            let r: Option<f64> = streaming_measure!("x", 0usize, { Some(42.0) });
            assert_eq!(r, Some(42.0));
        }
        // Path 2: with `metrics` feature, body + counter (no-op without
        // recorder) are both evaluated.
        #[cfg(feature = "metrics")]
        {
            let r: Option<f64> = streaming_measure!("x", 0usize, Some(42.0));
            assert_eq!(r, Some(42.0));
        }
    }
}
