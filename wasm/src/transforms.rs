use finkit::transforms::{
    Diff, DiffN, LogReturn, MinMaxScaler, PctChange, PercentileRank, Pipeline as CorePipeline,
    Rank, RollingMean, RollingStd, RollingSum, StandardScaler, Transform, ZScore,
};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Pipeline {
    inner: CorePipeline,
}

#[wasm_bindgen]
impl Pipeline {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: CorePipeline::new(),
        }
    }

    #[wasm_bindgen]
    pub fn add_log_return(mut self) -> Self {
        self.inner = self.inner.add(LogReturn);
        self
    }

    #[wasm_bindgen]
    pub fn add_pct_change(mut self) -> Self {
        self.inner = self.inner.add(PctChange);
        self
    }

    #[wasm_bindgen]
    pub fn add_zscore(mut self) -> Self {
        self.inner = self.inner.add(ZScore);
        self
    }

    #[wasm_bindgen]
    pub fn add_standard_scaler(mut self) -> Self {
        self.inner = self.inner.add(StandardScaler);
        self
    }

    #[wasm_bindgen]
    pub fn add_min_max_scaler(mut self) -> Self {
        self.inner = self.inner.add(MinMaxScaler);
        self
    }

    #[wasm_bindgen]
    pub fn add_rank(mut self) -> Self {
        self.inner = self.inner.add(Rank);
        self
    }

    #[wasm_bindgen]
    pub fn add_percentile_rank(mut self) -> Self {
        self.inner = self.inner.add(PercentileRank);
        self
    }

    #[wasm_bindgen]
    pub fn add_diff(mut self) -> Self {
        self.inner = self.inner.add(Diff);
        self
    }

    #[wasm_bindgen]
    pub fn add_diff_n(mut self, order: usize) -> Self {
        self.inner = self.inner.add(DiffN { order });
        self
    }

    #[wasm_bindgen]
    pub fn add_rolling_mean(mut self, window: usize) -> Self {
        self.inner = self.inner.add(RollingMean { window });
        self
    }

    #[wasm_bindgen]
    pub fn add_rolling_std(mut self, window: usize) -> Self {
        self.inner = self.inner.add(RollingStd { window });
        self
    }

    #[wasm_bindgen]
    pub fn add_rolling_sum(mut self, window: usize) -> Self {
        self.inner = self.inner.add(RollingSum { window });
        self
    }

    #[wasm_bindgen]
    pub fn transform(&self, data: Vec<f64>) -> Vec<f64> {
        self.inner.transform(&data)
    }
}

#[wasm_bindgen]
pub fn transform_log_return(data: Vec<f64>) -> Vec<f64> {
    LogReturn.transform(&data)
}

#[wasm_bindgen]
pub fn transform_zscore(data: Vec<f64>) -> Vec<f64> {
    ZScore.transform(&data)
}

#[wasm_bindgen]
pub fn transform_rank(data: Vec<f64>) -> Vec<f64> {
    Rank.transform(&data)
}

#[wasm_bindgen]
pub fn transform_diff(data: Vec<f64>) -> Vec<f64> {
    Diff.transform(&data)
}

#[wasm_bindgen]
pub fn transform_rolling_mean(data: Vec<f64>, window: usize) -> Vec<f64> {
    RollingMean { window }.transform(&data)
}
