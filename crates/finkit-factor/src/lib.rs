//! Factor abstraction layer.

use finkit_series::QuantSeries;

mod ema;
mod macd;
mod result;
mod rsi;
mod sma;

pub use ema::Ema;
pub use macd::Macd;
pub use result::FactorResult;
pub use rsi::Rsi;
pub use sma::Sma;

pub trait Factor {
    fn name(&self) -> &str;

    fn compute(&self, input: &QuantSeries) -> QuantSeries;
}
