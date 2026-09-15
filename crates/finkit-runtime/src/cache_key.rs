//! Cache identity for factor execution.

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FactorCacheKey {
    pub symbol: String,
    pub factor_name: String,
    pub params: String,
}

impl FactorCacheKey {
    pub fn new(symbol: impl Into<String>, factor_name: impl Into<String>, params: impl Into<String>) -> Self {
        Self {
            symbol: symbol.into(),
            factor_name: factor_name.into(),
            params: params.into(),
        }
    }
}
