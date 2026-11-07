use finkit::state_arena::StateArena;
use std::hint::black_box;
use std::time::{Duration, Instant};

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}

fn time_per_call(mut operation: impl FnMut(), duration: Duration) -> f64 {
    let started = Instant::now();
    let mut calls = 0_u64;
    while started.elapsed() < duration {
        operation();
        calls += 1;
    }
    started.elapsed().as_secs_f64() * 1e9 / calls as f64
}

fn main() {
    const SLOTS: usize = 4096;
    let mut arena = StateArena::new();
    arena.reserve_slots(SLOTS);
    assert!(arena.is_empty());
    assert_eq!(arena.slot_capacity(), SLOTS);

    // Reproduce the previous clear loop over a retained, entirely empty slot
    // vector. `black_box` keeps the scan visible to the optimizer.
    let mut prior_slots = vec![None::<Box<u64>>; SLOTS];
    let mut arena_samples = [0.0; 7];
    let mut prior_samples = [0.0; 7];
    for turn in 0..7 {
        if turn % 2 == 0 {
            arena_samples[turn] = time_per_call(
                || {
                    arena.clear();
                    black_box(&arena);
                },
                Duration::from_millis(150),
            );
            prior_samples[turn] = time_per_call(
                || {
                    for slot in &mut prior_slots {
                        *slot = None;
                    }
                    black_box(&prior_slots);
                },
                Duration::from_millis(150),
            );
        } else {
            prior_samples[turn] = time_per_call(
                || {
                    for slot in &mut prior_slots {
                        *slot = None;
                    }
                    black_box(&prior_slots);
                },
                Duration::from_millis(150),
            );
            arena_samples[turn] = time_per_call(
                || {
                    arena.clear();
                    black_box(&arena);
                },
                Duration::from_millis(150),
            );
        }
    }

    let previous = median(&mut prior_samples);
    let current = median(&mut arena_samples);
    println!(
        "empty state clear slots={SLOTS}: previous_scan={previous:.1}ns current={current:.1}ns speedup={:.2}x",
        previous / current
    );
    assert!(arena.is_empty());
    assert_eq!(arena.slot_capacity(), SLOTS);
}
