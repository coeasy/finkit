//! Built-in factor factories.

pub mod ema_factory;
pub mod macd_factory;
pub mod rsi_factory;
pub mod sma_factory;

pub use ema_factory::EmaFactory;
pub use macd_factory::MacdFactory;
pub use rsi_factory::RsiFactory;
pub use sma_factory::SmaFactory;
