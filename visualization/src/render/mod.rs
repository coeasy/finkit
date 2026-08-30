pub mod json;
pub mod svg;

#[cfg(feature = "png")]
pub mod png;

#[cfg(feature = "html")]
pub mod html;

use crate::config::ChartConfig;
use crate::error::Result;
use crate::primitive::DrawList;

pub trait Renderer {
    fn render(&self, draw_list: &DrawList, config: &ChartConfig) -> Result<String>;
}

pub use json::JsonRenderer;
pub use svg::SvgRenderer;

#[cfg(feature = "png")]
pub use png::PngRenderer;

#[cfg(feature = "html")]
pub use html::HtmlRenderer;
