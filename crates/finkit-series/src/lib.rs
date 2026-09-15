//! Quant series model for the Finkit factor engine.

use finkit_array::FloatArray;

#[derive(Clone, Debug)]
pub struct QuantSeries {
    pub symbol: String,
    pub timestamps: Vec<i64>,
    pub values: FloatArray,
}

impl QuantSeries {
    pub fn new(symbol: impl Into<String>, timestamps: Vec<i64>, values: FloatArray) -> Self {
        Self {
            symbol: symbol.into(),
            timestamps,
            values,
        }
    }

    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    pub fn timestamps(&self) -> &[i64] {
        &self.timestamps
    }

    pub fn values(&self) -> &[f64] {
        self.values.as_slice()
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}
