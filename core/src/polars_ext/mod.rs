//! Polars DataFrame and Apache Arrow zero-copy integration.
//!
//! Enabled via the `finkit-polars` Cargo feature.
//!
//! Provides extension traits on `Series` and `DataFrame` for
//! computing technical analysis indicators directly on Polars data
//! without copying the underlying Arrow buffers.

#[cfg(feature = "finkit-polars")]
mod series_ops;
#[cfg(feature = "finkit-polars")]
mod dataframe_ext;

#[cfg(feature = "finkit-polars")]
pub use series_ops::TaSeries;
#[cfg(feature = "finkit-polars")]
pub use dataframe_ext::{TaDataFrame, TaAccessor};
