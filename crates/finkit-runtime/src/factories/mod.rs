//! Built-in factor factories.

pub mod ema_factory;
pub mod sma_factory;
pub mod rsi_factory;
pub mod macd_factory;

pub use ema_factory::EmaFactory;
pub use sma_factory::SmaFactory;
pub use rsi_factory::RsiFactory;
pub use macd_factory::MacdFactory;
