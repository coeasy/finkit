use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum VisualizationError {
    #[error("No data available for rendering")]
    EmptyData,

    #[error("Insufficient data: need at least {required} points, got {length}")]
    InsufficientData { length: usize, required: usize },

    #[error("Invalid configuration: {field} - {reason}")]
    InvalidConfig { field: String, reason: String },

    #[error("Rendering failed: {message}")]
    RenderError { message: String },

    #[error("Data conversion error: {message}")]
    ConversionError { message: String },

    #[error("Unsupported indicator type: {type_name}")]
    UnsupportedIndicator { type_name: String },

    #[error("Theme loading failed: {message}")]
    ThemeError { message: String },

    #[error("Serialization error: {message}")]
    SerializationError { message: String },

    #[error("Rasterization error: {message}")]
    RasterizeError { message: String },

    #[error("Font error: {message}")]
    FontError { message: String },

    #[error("PNG encoding error: {message}")]
    PngEncodeError { message: String },

    #[error("Layout error: {message}")]
    LayoutError { message: String },

    #[error("Geometry error: {message}")]
    GeometryError { message: String },

    #[error("Decimation error: {message}")]
    DecimationError { message: String },
}

pub type Result<T> = std::result::Result<T, VisualizationError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = VisualizationError::EmptyData;
        assert_eq!(format!("{}", err), "No data available for rendering");

        let err = VisualizationError::InsufficientData {
            length: 5,
            required: 20,
        };
        assert_eq!(
            format!("{}", err),
            "Insufficient data: need at least 20 points, got 5"
        );

        let err = VisualizationError::InvalidConfig {
            field: "title".to_string(),
            reason: "cannot be empty".to_string(),
        };
        assert_eq!(
            format!("{}", err),
            "Invalid configuration: title - cannot be empty"
        );
    }

    #[test]
    fn test_result_type() {
        let ok: Result<String> = Ok("success".to_string());
        assert!(ok.is_ok());

        let err: Result<String> = Err(VisualizationError::EmptyData);
        assert!(err.is_err());
    }

    #[test]
    fn test_new_error_variants() {
        let err = VisualizationError::RasterizeError {
            message: "canvas alloc failed".to_string(),
        };
        assert_eq!(
            format!("{}", err),
            "Rasterization error: canvas alloc failed"
        );

        let err = VisualizationError::FontError {
            message: "glyph not found".to_string(),
        };
        assert_eq!(format!("{}", err), "Font error: glyph not found");

        let err = VisualizationError::PngEncodeError {
            message: "io error".to_string(),
        };
        assert_eq!(format!("{}", err), "PNG encoding error: io error");

        let err = VisualizationError::LayoutError {
            message: "overflow".to_string(),
        };
        assert_eq!(format!("{}", err), "Layout error: overflow");

        let err = VisualizationError::GeometryError {
            message: "invalid polygon".to_string(),
        };
        assert_eq!(format!("{}", err), "Geometry error: invalid polygon");

        let err = VisualizationError::DecimationError {
            message: "threshold too low".to_string(),
        };
        assert_eq!(format!("{}", err), "Decimation error: threshold too low");
    }
}
