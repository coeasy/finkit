use napi_derive::napi;
use alpha_ta_core::transforms::{
    Diff, DiffN, LogReturn, MinMaxScaler, PctChange, PercentileRank, Pipeline as CorePipeline,
    Rank, RollingMean, RollingStd, RollingSum, StandardScaler, Transform, ZScore,
};

#[napi]
pub struct Pipeline {
    inner: CorePipeline,
}

#[napi]
impl Pipeline {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            inner: CorePipeline::new(),
        }
    }

    #[napi]
    pub fn add_log_return(&mut self) -> &Self {
        let pipeline = std::mem::take(&mut self.inner);
        self.inner = pipeline.add(LogReturn);
        self
    }

    #[napi]
    pub fn add_pct_change(&mut self) -> &Self {
        let pipeline = std::mem::take(&mut self.inner);
        self.inner = pipeline.add(PctChange);
        self
    }

    #[napi]
    pub fn add_zscore(&mut self) -> &Self {
        let pipeline = std::mem::take(&mut self.inner);
        self.inner = pipeline.add(ZScore);
        self
    }

    #[napi]
    pub fn add_standard_scaler(&mut self) -> &Self {
        let pipeline = std::mem::take(&mut self.inner);
        self.inner = pipeline.add(StandardScaler);
        self
    }

    #[napi]
    pub fn add_min_max_scaler(&mut self) -> &Self {
        let pipeline = std::mem::take(&mut self.inner);
        self.inner = pipeline.add(MinMaxScaler);
        self
    }

    #[napi]
    pub fn add_rank(&mut self) -> &Self {
        let pipeline = std::mem::take(&mut self.inner);
        self.inner = pipeline.add(Rank);
        self
    }

    #[napi]
    pub fn add_percentile_rank(&mut self) -> &Self {
        let pipeline = std::mem::take(&mut self.inner);
        self.inner = pipeline.add(PercentileRank);
        self
    }

    #[napi]
    pub fn add_diff(&mut self) -> &Self {
        let pipeline = std::mem::take(&mut self.inner);
        self.inner = pipeline.add(Diff);
        self
    }

    #[napi]
    pub fn add_diff_n(&mut self, order: u32) -> &Self {
        let pipeline = std::mem::take(&mut self.inner);
        self.inner = pipeline.add(DiffN { order: order as usize });
        self
    }

    #[napi]
    pub fn add_rolling_mean(&mut self, window: u32) -> &Self {
        let pipeline = std::mem::take(&mut self.inner);
        self.inner = pipeline.add(RollingMean { window: window as usize });
        self
    }

    #[napi]
    pub fn add_rolling_std(&mut self, window: u32) -> &Self {
        let pipeline = std::mem::take(&mut self.inner);
        self.inner = pipeline.add(RollingStd { window: window as usize });
        self
    }

    #[napi]
    pub fn add_rolling_sum(&mut self, window: u32) -> &Self {
        let pipeline = std::mem::take(&mut self.inner);
        self.inner = pipeline.add(RollingSum { window: window as usize });
        self
    }

    #[napi]
    pub fn transform(&self, data: Vec<f64>) -> Vec<f64> {
        self.inner.transform(&data)
    }
}

#[napi]
pub fn transform_log_return(data: Vec<f64>) -> Vec<f64> {
    LogReturn.transform(&data)
}

#[napi]
pub fn transform_zscore(data: Vec<f64>) -> Vec<f64> {
    ZScore.transform(&data)
}

#[napi]
pub fn transform_rank(data: Vec<f64>) -> Vec<f64> {
    Rank.transform(&data)
}

#[napi]
pub fn transform_diff(data: Vec<f64>) -> Vec<f64> {
    Diff.transform(&data)
}

#[napi]
pub fn transform_rolling_mean(data: Vec<f64>, window: u32) -> Vec<f64> {
    RollingMean { window: window as usize }.transform(&data)
}
