//! Factor abstraction layer.

use finkit_series::QuantSeries;

mod sma;

pub use sma::Sma;

pub trait Factor {
    fn name(&self) -> &str;

    fn compute(&self, input: &QuantSeries) -> QuantSeries;
}
