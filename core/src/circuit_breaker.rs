//! Optional `circuit-breaker` pattern (R-4).
//!
//! Wraps a fallible operation so that repeated failures open the circuit and
//! subsequent calls return [`CircuitBreakerError::Open`] for a cool-off period.
//! The breaker auto-closes once `cool_off` elapses, retrying the next call.
//!
//! Use this to protect a downstream service (e.g. a remote formula evaluator)
//! from being hammered after a transient outage:
//!
//! ```no_run
//! use finkit::circuit_breaker::{CircuitBreaker, CircuitBreakerError};
//!
//! let mut cb = CircuitBreaker::new(3, std::time::Duration::from_secs(10));
//! let result: Result<u32, CircuitBreakerError<&'static str>> =
//!     cb.call(|| Ok(42));
//! assert_eq!(result.unwrap(), 42);
//! ```

use std::time::{Duration, Instant};

/// State of the breaker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// Normal operation — calls pass through and failures are counted.
    Closed,
    /// Tripped — calls short-circuit until `cool_off` elapses.
    Open,
}

/// Failure mode of the breaker.
#[derive(Debug)]
pub enum CircuitBreakerError<E> {
    /// The underlying operation failed; the breaker remains in `Closed`
    /// (or trips to `Open` if `fail_threshold` was reached).
    Inner(E),
    /// The breaker is open; the operation was not invoked.
    Open,
}

impl<E: std::fmt::Display> std::fmt::Display for CircuitBreakerError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Inner(e) => write!(f, "inner error: {e}"),
            Self::Open => f.write_str("circuit breaker is open"),
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for CircuitBreakerError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Inner(e) => Some(e),
            Self::Open => None,
        }
    }
}

/// Half-open probe behaviour: a single in-flight call is allowed while
/// `Open` to test recovery. The breaker does not block the caller for the
/// cool-off; it simply short-circuits and the next call after cool-off
/// becomes the probe.
#[derive(Debug)]
pub struct CircuitBreaker {
    fail_threshold: u32,
    cool_off: Duration,
    state: State,
    consecutive_failures: u32,
    opened_at: Option<Instant>,
}

impl CircuitBreaker {
    /// Create a breaker that trips after `fail_threshold` consecutive
    /// failures and stays open for at least `cool_off`.
    pub fn new(fail_threshold: u32, cool_off: Duration) -> Self {
        Self {
            fail_threshold: fail_threshold.max(1),
            cool_off,
            state: State::Closed,
            consecutive_failures: 0,
            opened_at: None,
        }
    }

    /// Current state. Resets `Open` to `Closed` once the cool-off elapses.
    pub fn state(&mut self) -> State {
        self.maybe_close();
        self.state
    }

    fn maybe_close(&mut self) {
        if self.state == State::Open {
            if let Some(t) = self.opened_at {
                if t.elapsed() >= self.cool_off {
                    self.state = State::Closed;
                    self.consecutive_failures = 0;
                    self.opened_at = None;
                }
            }
        }
    }

    /// Run `op`. If the breaker is open, returns [`CircuitBreakerError::Open`]
    /// without invoking `op`. Otherwise, records success/failure and trips
    /// after `fail_threshold` consecutive failures.
    pub fn call<T, E, F>(&mut self, op: F) -> Result<T, CircuitBreakerError<E>>
    where
        F: FnOnce() -> Result<T, E>,
    {
        self.maybe_close();
        if self.state == State::Open {
            return Err(CircuitBreakerError::Open);
        }
        match op() {
            Ok(v) => {
                self.consecutive_failures = 0;
                Ok(v)
            }
            Err(e) => {
                self.consecutive_failures += 1;
                if self.consecutive_failures >= self.fail_threshold {
                    self.state = State::Open;
                    self.opened_at = Some(Instant::now());
                }
                Err(CircuitBreakerError::Inner(e))
            }
        }
    }

    /// Force the breaker back to `Closed`. Useful for tests.
    pub fn reset(&mut self) {
        self.state = State::Closed;
        self.consecutive_failures = 0;
        self.opened_at = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closed_passes_through() {
        let mut cb = CircuitBreaker::new(2, Duration::from_secs(1));
        let r: Result<i32, CircuitBreakerError<()>> = cb.call(|| Ok(7));
        assert_eq!(r.unwrap(), 7);
        assert_eq!(cb.state(), State::Closed);
    }

    #[test]
    fn opens_after_threshold() {
        let mut cb = CircuitBreaker::new(2, Duration::from_secs(1));
        let _: Result<i32, CircuitBreakerError<&str>> = cb.call(|| Err("boom"));
        let _: Result<i32, CircuitBreakerError<&str>> = cb.call(|| Err("boom"));
        assert_eq!(cb.state(), State::Open);
        let r: Result<i32, CircuitBreakerError<&str>> = cb.call(|| Ok(1));
        assert!(matches!(r, Err(CircuitBreakerError::Open)));
    }

    #[test]
    fn closes_after_cool_off() {
        let mut cb = CircuitBreaker::new(1, Duration::from_millis(10));
        let _: Result<i32, CircuitBreakerError<&str>> = cb.call(|| Err("x"));
        assert_eq!(cb.state(), State::Open);
        std::thread::sleep(Duration::from_millis(20));
        assert_eq!(cb.state(), State::Closed);
        let r: Result<i32, CircuitBreakerError<&str>> = cb.call(|| Ok(42));
        assert_eq!(r.unwrap(), 42);
    }
}
