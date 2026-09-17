//! Factor abstraction layer.

use finkit_series::QuantSeries;

mod ema;
mod sma;
mod rsi;
mod macd;
mod result;

pub use ema::Ema;
pub use sma::Sma;
pub use rsi::Rsi;
pub use macd::Macd;
pub use result::FactorResult;

pub trait Factor {
    fn name(&self) -> &str;

    fn compute(&self, input: &QuantSeries) -> QuantSeries;
}
