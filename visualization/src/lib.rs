#![allow(missing_docs)]
#![allow(missing_debug_implementations)]

pub mod config;
pub mod data;
pub mod error;
pub mod language;
pub mod renderer;

pub mod chart;
pub mod decimate;
pub mod geometry;
pub mod interaction;
pub mod layout;
pub mod primitive;
pub mod render;
pub mod text;

pub use error::{Result, VisualizationError};
