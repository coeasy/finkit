//! Polars DataFrame and Apache Arrow zero-copy integration.
//!
//! Enabled via the `alpha-ta-polars` Cargo feature.
//!
//! Provides extension traits on `Series` and `DataFrame` for
//! computing technical analysis indicators directly on Polars data
//! without copying the underlying Arrow buffers.

#[cfg(feature = "alpha-ta-polars")]
mod series_ops;
#[cfg(feature = "alpha-ta-polars")]
mod dataframe_ext;

#[cfg(feature = "alpha-ta-polars")]
pub use series_ops::TaSeries;
#[cfg(feature = "alpha-ta-polars")]
pub use dataframe_ext::{TaDataFrame, TaAccessor};
