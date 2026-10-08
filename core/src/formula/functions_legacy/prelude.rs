//! Shared prelude for the `functions_legacy` family.
//!
//! Every bucket here was split out of one file and composes with the same
//! imports and helpers, which live in the parent module. This is the single
//! place that surface is named: buckets write `use super::prelude::*;` rather
//! than each carrying a copy, and a module called `prelude` is explicitly
//! exempt from `clippy::wildcard_imports`.

pub(crate) use super::*;
