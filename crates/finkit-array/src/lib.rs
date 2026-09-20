//! Finkit array abstraction layer.
//!
//! This crate is the foundation for future SIMD, Arrow and zero-copy buffers.
//!
//! # Superseded -- do not add public API
//!
//! Part of the unconverged `crates/finkit-*` migration track, which has no
//! production dependents (`core`, `ffi`, `cli` and `wasm` never reference it).
//! [`FloatArray`] is fully covered by `core`'s `buffer_arena::BufferArena` plus
//! the `ndarray`-backed buffers, so this crate is scheduled for removal -- see
//! `docs/runtime-carrier-adoption-plan-2026-09-20.md`.

#[derive(Clone, Debug, PartialEq)]
pub struct FloatArray {
    data: Vec<f64>,
}

impl FloatArray {
    pub fn new(data: Vec<f64>) -> Self {
        Self { data }
    }

    pub fn as_slice(&self) -> &[f64] {
        &self.data
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}
