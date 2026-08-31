//! Minimal memory-profile benchmark entry point for the optional profiling feature.
#![cfg(feature = "memory-profile")]

use dhat::{Alloc, Profiler};

#[global_allocator]
static ALLOC: Alloc = Alloc;

fn main() {
    let _profiler = Profiler::builder().testing().build();
    let values: Vec<f64> = (0..1024).map(f64::from).collect();
    std::hint::black_box(values);
}
