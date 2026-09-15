//! Runtime factor registry foundation.

#[derive(Clone, Debug)]
pub struct FactorDescriptor {
    pub name: String,
}

#[derive(Clone, Debug, Default)]
pub struct FactorRegistry {
    factors: Vec<FactorDescriptor>,
}

impl FactorRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, name: impl Into<String>) {
        self.factors.push(FactorDescriptor { name: name.into() });
    }

    pub fn contains(&self, name: &str) -> bool {
        self.factors.iter().any(|item| item.name == name)
    }
}
