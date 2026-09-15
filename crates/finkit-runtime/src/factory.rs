//! Factor creation abstraction.

#[derive(Clone, Debug)]
pub struct FactorFactoryRequest {
    pub name: String,
    pub params: Vec<(String, String)>,
}

impl FactorFactoryRequest {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            params: Vec::new(),
        }
    }

    pub fn with_param(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.params.push((key.into(), value.into()));
        self
    }
}
