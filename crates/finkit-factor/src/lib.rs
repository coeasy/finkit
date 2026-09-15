//! Factor abstraction layer.

use finkit_series::QuantSeries;

mod ema;
mod sma;

pub use ema::Ema;
pub use sma::Sma;

pub trait Factor {
    fn name(&self) -> &str;

    fn compute(&self, input: &QuantSeries) -> QuantSeries;
}
