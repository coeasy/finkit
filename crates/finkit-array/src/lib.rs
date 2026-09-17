//! Finkit array abstraction layer.
//!
//! This crate is the foundation for future SIMD, Arrow and zero-copy buffers.

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
