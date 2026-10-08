//! Forming-bar rollback: the single home of the repaint discipline.
//!
//! A streaming feed delivers a bar, and may then re-deliver *the same bar*
//! with a fresher close while it is still forming. An indicator that already
//! folded the first copy into its state has to undo that fold before folding
//! the new one in — otherwise its output silently depends on how many times
//! the feed happened to call it.
//!
//! Ten indicators used to hand-write this: an `Option<SnapshotState>` plus an
//! `i64`, with the `t != 0 && t == self.last_open_time` test spelled out in
//! each. The failure mode of a hand-written copy is not a compile error. It is
//! restoring the indicator's own fields but forgetting `last_open_time`, after
//! which the *next* re-feed no longer matches and the bar is folded in twice.
//! So the whole discipline lives in [`FormingBar`] and an indicator
//! contributes only the state that is actually its own.
//!
//! ```
//! use finkit::streaming::forming_bar::FormingBar;
//!
//! let mut bar: FormingBar<u32> = FormingBar::new();
//! assert_eq!(bar.take_rollback(100), None); // first sighting of bar 100
//! bar.begin(100, 7);
//! assert_eq!(bar.open_time(), 100);
//!
//! // The feed re-delivers bar 100 with a fresher close: the pre-bar state
//! // comes back so the indicator can fold the new copy in cleanly.
//! assert_eq!(bar.take_rollback(100), Some(7));
//! // ...and consuming it means a third delivery has nothing to roll back to.
//! assert_eq!(bar.take_rollback(100), None);
//!
//! // A different bar is never a re-delivery.
//! bar.begin(101, 9);
//! assert_eq!(bar.take_rollback(102), None);
//! ```

/// Tracks the bar currently in flight and the state to restore when that same
/// `open_time` arrives again.
///
/// `S` is moved in and out rather than copied, so a snapshot that owns heap
/// state (`StreamingMacdExt` clones three moving-average states) needs no
/// `Copy` bound of its own.
#[derive(Debug, Clone, Copy)]
pub struct FormingBar<S> {
    /// `open_time` of the bar in flight; `0` before the first bar.
    open_time: i64,
    /// Pre-bar state, present only while a bar is in flight.
    snapshot: Option<S>,
}

impl<S> FormingBar<S> {
    /// A tracker with no bar in flight.
    pub const fn new() -> Self {
        Self {
            open_time: 0,
            snapshot: None,
        }
    }

    /// The `open_time` of the bar in flight, or `0` when there is none.
    ///
    /// `0` is treated as "no bar" throughout, matching the convention the
    /// hand-written copies used: feeds that do not stamp their bars can never
    /// trigger a rollback, which is the safe direction — a fold that cannot be
    /// undone is still one fold, not two.
    pub fn open_time(&self) -> i64 {
        self.open_time
    }

    /// Take the pre-bar state, but only when `open_time` re-delivers the bar
    /// already in flight.
    ///
    /// `None` means "this is a new bar, fold it straight in". The returned
    /// state is *taken*, not copied, so a second rollback for the same bar
    /// correctly finds nothing — the indicator is then mid-way through
    /// recomputing that bar and must not roll back twice.
    pub fn take_rollback(&mut self, open_time: i64) -> Option<S> {
        if open_time != 0 && open_time == self.open_time {
            self.snapshot.take()
        } else {
            None
        }
    }

    /// Record `state` as the pre-bar state and adopt `open_time` as in flight.
    ///
    /// Call this *after* any rollback for the same bar has been applied, so the
    /// recorded state is the one that precedes the fold about to happen.
    pub fn begin(&mut self, open_time: i64, state: S) {
        self.snapshot = Some(state);
        self.open_time = open_time;
    }

    /// Adopt `open_time` as in flight **without** recording a rollback.
    ///
    /// Only for restoring a state that was captured elsewhere — a composed
    /// indicator saving its child's progress across a repaint bar, say. A
    /// caller that is about to fold the bar in should use [`Self::begin`]
    /// instead, which is the only way to make the fold undoable.
    pub fn set_open_time(&mut self, open_time: i64) {
        self.open_time = open_time;
    }

    /// Forget the in-flight bar. Call from `reset`.
    pub fn clear(&mut self) {
        self.snapshot = None;
        self.open_time = 0;
    }
}

impl<S> Default for FormingBar<S> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_sighting_of_a_bar_is_not_a_rollback() {
        let mut bar: FormingBar<u32> = FormingBar::new();
        assert_eq!(bar.open_time(), 0);
        assert_eq!(bar.take_rollback(42), None);
    }

    #[test]
    fn redelivering_the_forming_bar_yields_the_state_exactly_once() {
        let mut bar: FormingBar<u32> = FormingBar::new();
        bar.begin(42, 7);
        assert_eq!(bar.take_rollback(42), Some(7));
        // Consuming it must not leave a second copy behind: an indicator that
        // is midway through recomputing bar 42 cannot roll back again.
        assert_eq!(bar.take_rollback(42), None);
    }

    #[test]
    fn a_different_open_time_is_never_a_rollback() {
        let mut bar: FormingBar<u32> = FormingBar::new();
        bar.begin(42, 7);
        assert_eq!(bar.take_rollback(43), None);
        // The in-flight bar is untouched by a non-matching probe.
        assert_eq!(bar.open_time(), 42);
        assert_eq!(bar.take_rollback(42), Some(7));
    }

    #[test]
    fn an_unstamped_bar_never_rolls_back() {
        // `open_time == 0` means "no timestamp"; folding it twice is still one
        // fold, so it must not be treated as a re-delivery.
        let mut bar: FormingBar<u32> = FormingBar::new();
        bar.begin(0, 7);
        assert_eq!(bar.take_rollback(0), None);
    }

    #[test]
    fn begin_after_a_rollback_re_arms_the_bar() {
        let mut bar: FormingBar<u32> = FormingBar::new();
        bar.begin(42, 7);
        let restored = bar.take_rollback(42).expect("re-delivery");
        // The indicator restored the snapshot, recomputed, and re-armed the bar
        // with the state that now precedes the new fold.
        assert_eq!(restored, 7);
        bar.begin(42, 11);
        assert_eq!(bar.take_rollback(42), Some(11));
    }

    #[test]
    fn clear_returns_the_tracker_to_its_initial_state() {
        let mut bar: FormingBar<u32> = FormingBar::new();
        bar.begin(42, 7);
        bar.clear();
        assert_eq!(bar.open_time(), 0);
        assert_eq!(bar.take_rollback(42), None);
    }

    #[test]
    fn set_open_time_adopts_the_bar_without_armoring_a_rollback() {
        let mut bar: FormingBar<u32> = FormingBar::new();
        bar.set_open_time(42);
        assert_eq!(bar.open_time(), 42);
        // Deliberately no rollback record: this is a restore, not a fold.
        assert_eq!(bar.take_rollback(42), None);
    }
}
