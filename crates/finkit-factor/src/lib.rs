//! Factor abstraction layer.

use finkit_series::QuantSeries;

pub trait Factor {
    fn name(&self) -> &str;

    fn compute(&self, input: &QuantSeries) -> QuantSeries;
}
