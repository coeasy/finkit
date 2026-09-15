//! Factor abstraction layer.

use finkit_series::QuantSeries;

mod ema;
mod sma;
mod result;

pub use ema::Ema;
pub use sma::Sma;
pub use result::FactorResult;

pub trait Factor {
    fn name(&self) -> &str;

    fn compute(&self, input: &QuantSeries) -> QuantSeries;
}
