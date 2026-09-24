//! Truthiness of a numeric condition in the formula language.
//!
//! This is the single source of truth for "when does a condition select the
//! *then* branch". Every evaluation path calls it instead of restating the
//! comparison.
//!
//! ## The rule
//!
//! A condition is **true when it is not exactly zero** (`!= 0.0`). Two
//! consequences are worth stating because they are easy to get wrong:
//!
//! - A **negative** condition is true: `-1.0` is non-zero, so
//!   `IF(-1, a, b)` selects `a`.
//! - A **NaN** condition is true, because `NaN != 0.0` is `true`.
//!
//! Both fall out of comparing against zero rather than testing the sign.
//!
//! ## Why this rule and not `> 0.0`
//!
//! The rule used to be `> 0.0` at the hand-written sites while
//! [`crate::formula::simd::SimdOps::select`] used `!= 0.0`. Because the SIMD
//! path only engages from 16 elements up, the *same formula returned different
//! answers depending on series length*:
//!
//! ```text
//! IF(-1, 10, 20)  ->  20 with 4, 8 or 15 bars
//!                 ->  10 with 16, 17 or 32 bars
//! ```
//!
//! Unifying on `!= 0.0` fixes that and leaves every series of 16 or more
//! elements — that is, essentially all real data — producing exactly the values
//! it produced before. Unifying on `> 0.0` would instead have changed the
//! results of real data and preserved the short-series anomaly.
//!
//! `!= 0.0` is also the domestic convention: 通达信 and 同花顺 treat a non-zero
//! condition as true, which is what `IF(CLOSE - OPEN, a, b)` relies on when the
//! bar is down.
//!
//! ## Deliberately out of scope
//!
//! These test [`is_logical_true`] instead, and are left that way on purpose:
//!
//! - **Logical operators** (`And` / `Or` / `Xor` / `Not`). They fold a value to
//!   `1.0` / `0.0` rather than selecting between two branches, so their
//!   behaviour is a separate question from branch selection.
//! - **`while` loop continuation** (`executor.rs`). Changing it would alter how
//!   many times loops run rather than which value is picked.
//!
//! Neither is length-dependent, which is the property this module exists to
//! guarantee. Deciding to unify the two rules on `!= 0.0` would be a *semantic
//! change* to every logical operator on negative and NaN operands — it would
//! move the plan's `BINARY:And/Or/Xor` kernels and the JIT as well — so it must
//! not be done as a "consistency cleanup".

/// Whether a numeric condition selects the *then* branch.
///
/// See the module documentation for the rule and why it is not `> 0.0`.
///
/// # Examples
///
/// ```
/// use finkit::formula::truth::is_true;
///
/// assert!(is_true(1.0));
/// assert!(is_true(-1.0)); // non-zero, so true
/// assert!(!is_true(0.0));
/// ```
#[inline]
#[must_use]
pub const fn is_true(condition: f64) -> bool {
    condition != 0.0
}

/// Truthiness for the **logical operators** (`And` / `Or` / `Xor` / `Not`).
///
/// Deliberately **not** [`is_true`]: a value counts as true only when it is
/// strictly positive, so a negative operand is false. See the module
/// documentation for why the two rules are separate.
///
/// Every implementation of the logical operators must call this — the tree
/// scalar and array kernels, the bytecode VM, the JIT, the plan's
/// `BINARY:And/Or/Xor` and `UNARY:Not` kernels, the streaming path, the
/// optimizer's constant folder, and the `SimdOps::logical_*` helpers.
///
/// Those helpers were the one holdout: they compared against zero
/// (`!= 0.0`), and they are what the *array* legs call. So the same expression
/// family answered two ways depending on whether an operand happened to be a
/// scalar — `CLOSE AND 0` was `0.0` while `CLOSE AND (0 - CLOSE)` was `1.0`.
/// Negative operands are the only thing that separates the two comparisons, and
/// no test used any.
#[inline]
#[must_use]
pub const fn is_logical_true(value: f64) -> bool {
    value > 0.0
}

/// Fold a [`is_logical_true`] verdict into the `1.0` / `0.0` a logical operator
/// produces.
#[inline]
#[must_use]
pub const fn logical_bool(value: bool) -> f64 {
    if value {
        1.0
    } else {
        0.0
    }
}
