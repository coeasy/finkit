use crate::error::{Result, TaError};
use crate::math::linear::{linreg, linreg_angle, linreg_intercept, linreg_slope};
use crate::utils::{init_output, validate_input};
use ndarray::Array1;

/// Mean Absolute Deviation (AVGDEV)
///
/// Computes the mean absolute deviation of each value from the rolling mean.
///
/// # 公式
/// AVGDEV_i = mean(|x_{i-k} - mean(window)|) for k in [0, period)
///
/// # 参数
/// * `input` - 输入数据序列
/// * `timeperiod` - 滚动窗口大小
///
/// # 返回值
/// AVGDEV 数组（前 `timeperiod - 1` 个值为 NaN；`timeperiod == 1` 时全部为 0）
///
/// # 示例
/// ```rust
/// use finkit::indicators::statistics::avgdev;
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
/// let result = avgdev(&data, 5).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
pub fn avgdev(input: &[f64], timeperiod: usize) -> Result<Array1<f64>> {
    if timeperiod < 1 {
        return Err(TaError::InvalidParameter {
            name: "timeperiod".to_string(),
            constraint: "at least 1".to_string(),
        });
    }
    validate_input(input.len(), timeperiod)?;

    let len = input.len();
    let mut output = init_output(len);
    if timeperiod == 1 {
        // No deviation: window of size 1 always has mean == x.
        for i in 0..len {
            output[i] = 0.0;
        }
        return Ok(output);
    }

    let n = timeperiod as f64;
    let inv_n = 1.0 / n;

    // Rolling window sum → recompute mean → recompute sum of |x - mean| per step.
    // O(1) rolling mean update, O(period) abs-deviation per step. For typical
    // periods (<= 200) this is fast enough; a strict O(1) streaming version
    // is implemented in `streaming::StreamingAvgdev`.
    let mut buf: Vec<f64> = input[..timeperiod].to_vec();
    let mut sum: f64 = buf.iter().sum();

    // Helper closure
    let dev_sum = |buf: &[f64], mean: f64| -> f64 { buf.iter().map(|&x| (x - mean).abs()).sum() };

    let mean = sum * inv_n;
    output[timeperiod - 1] = dev_sum(&buf, mean) * inv_n;

    for i in timeperiod..len {
        let oldest = input[i - timeperiod];
        let newest = input[i];
        sum += newest - oldest;
        // O(period) sliding-window update: drop oldest from front, push newest
        // to back. Cheap because period is bounded and the vec is tiny.
        buf.remove(0);
        buf.push(newest);
        let mean = sum * inv_n;
        output[i] = dev_sum(&buf, mean) * inv_n;
        let _ = oldest; // keep `oldest` for clarity / debug
    }

    Ok(output)
}

/// Z-Score (Z分数/标准化)
///
/// 计算每个数据点相对于滚动窗口的标准分数，表示该点偏离均值多少个标准差。
///
/// # 公式
/// Z = (X - μ) / σ
///
/// # 参数
/// * `input` - 输入数据序列
/// * `timeperiod` - 滚动窗口大小
///
/// # 返回值
/// Z-Score 数组（前 `timeperiod - 1` 个值为 NaN）
///
/// # 示例
/// ```rust
/// use finkit::indicators::zscore;
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
/// let result = zscore(&data, 5).unwrap();
/// ```
pub fn zscore(input: &[f64], timeperiod: usize) -> Result<Array1<f64>> {
    if timeperiod < 2 {
        return Err(TaError::InvalidParameter {
            name: "timeperiod".to_string(),
            constraint: "at least 2".to_string(),
        });
    }
    validate_input(input.len(), timeperiod)?;

    let len = input.len();
    let mut output = init_output(len);
    let n = timeperiod as f64;
    let inv_n = 1.0 / n;
    let inv_n_minus_1 = 1.0 / (n - 1.0);

    // Initialize accumulators with first window
    let mut sum: f64 = 0.0;
    let mut sum_sq: f64 = 0.0;
    for j in 0..timeperiod {
        sum += input[j];
        sum_sq += input[j] * input[j];
    }

    // First window
    let mean = sum * inv_n;
    let var = ((sum_sq - sum * mean) * inv_n_minus_1).max(0.0);
    let std_dev = var.sqrt();
    if std_dev > 1e-15 {
        output[timeperiod - 1] = (input[timeperiod - 1] - mean) / std_dev;
    }

    // Subsequent windows — incremental O(1) update per step
    for i in timeperiod..len {
        let old = input[i - timeperiod];
        let new = input[i];
        sum += new - old;
        sum_sq += new * new - old * old;
        let mean = sum * inv_n;
        let var = ((sum_sq - sum * mean) * inv_n_minus_1).max(0.0);
        let std_dev = var.sqrt();
        if std_dev > 1e-15 {
            output[i] = (input[i] - mean) / std_dev;
        }
    }

    Ok(output)
}

/// Percent Rank (百分比排名)
///
/// 计算当前值在过去窗口中的百分比排名，表示有多少比例的值低于当前值。
///
/// # 公式
/// PercentRank = (Count(values < current) / Count(valid values)) * 100
///
/// # 参数
/// * `input` - 输入数据序列
/// * `timeperiod` - 滚动窗口大小
///
/// # 返回值
/// 百分比排名数组（前 `timeperiod - 1` 个值为 NaN）
///
/// # 示例
/// ```rust
/// use finkit::indicators::percent_rank;
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
/// let result = percent_rank(&data, 3).unwrap();
/// ```
pub fn percent_rank(input: &[f64], timeperiod: usize) -> Result<Array1<f64>> {
    if timeperiod < 1 {
        return Err(TaError::InvalidParameter {
            name: "timeperiod".to_string(),
            constraint: "at least 1".to_string(),
        });
    }
    validate_input(input.len(), timeperiod)?;

    let len = input.len();
    let mut output = init_output(len);

    // Maintain a sorted window for O(log n) rank lookup
    let mut sorted: Vec<f64> = input[..timeperiod].to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    // First window
    let current = input[timeperiod - 1];
    let count_less = sorted.partition_point(|&x| x < current);
    output[timeperiod - 1] = (count_less as f64 / timeperiod as f64) * 100.0;

    // Subsequent windows — incremental sorted-vec update
    for i in timeperiod..len {
        // Remove the evicted value
        let evicted = input[i - timeperiod];
        let pos = sorted.partition_point(|&x| x < evicted);
        sorted.remove(pos);

        // Insert the new value in sorted order
        let new_val = input[i];
        let insert_pos = sorted.partition_point(|&x| x < new_val);
        sorted.insert(insert_pos, new_val);

        // Compute rank via binary search
        let count_less = sorted.partition_point(|&x| x < new_val);
        output[i] = (count_less as f64 / timeperiod as f64) * 100.0;
    }

    Ok(output)
}

/// Percent Rank (PR) — TA-Lib 0.6.4 compatible short alias.
///
/// Returns `(count(input[i-period+1..=i] < input[i]) / period) * 100`,
/// which matches TA-Lib's `TA_PERCENTRANK` semantics (1.0 normalisation, *100
/// to express as a percentage in `[0, 100]`).
///
/// Added 2026-06-06 to provide parity with TA-Lib 0.6.4's `PERCENTRANK`.
#[inline]
pub fn pr(input: &[f64], timeperiod: usize) -> Result<Array1<f64>> {
    percent_rank(input, timeperiod)
}

/// Beta 系数 (Beta Coefficient)
///
/// 计算两只股票/资产之间的 Beta 系数，衡量相对波动率。
/// Beta 表示资产收益相对于基准资产收益的敏感度。
///
/// # 公式
/// Beta = Cov(asset, benchmark) / Var(benchmark)
///
/// # 参数
/// * `asset` - 资产价格序列（如个股）
/// * `benchmark` - 基准价格序列（如大盘指数）
/// * `timeperiod` - 滚动窗口大小
///
/// # 返回值
/// Beta 系数数组（前 `timeperiod - 1` 个值为 NaN）
///
/// # 说明
/// - Beta > 1: 资产波动性大于基准
/// - Beta = 1: 资产波动性与基准相同
/// - Beta < 1: 资产波动性小于基准
/// - Beta < 0: 资产与基准呈反向变动
///
/// # 示例
/// ```rust
/// use finkit::indicators::beta;
/// let stock = vec![100.0, 101.0, 102.0, 103.0, 104.0, 105.0];
/// let market = vec![3000.0, 3010.0, 3020.0, 3015.0, 3030.0, 3040.0];
/// let result = beta(&stock, &market, 5).unwrap();
/// ```
pub fn beta(asset: &[f64], benchmark: &[f64], timeperiod: usize) -> Result<Array1<f64>> {
    if asset.len() != benchmark.len() {
        return Err(TaError::InvalidParameter {
            name: "asset and benchmark".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    if timeperiod < 2 {
        return Err(TaError::InvalidParameter {
            name: "timeperiod".to_string(),
            constraint: "at least 2".to_string(),
        });
    }
    validate_input(asset.len(), timeperiod)?;

    let len = asset.len();
    let mut output = init_output(len);

    let n = timeperiod as f64;

    // Initialize accumulators with first window
    // TA-Lib uses raw prices, not returns
    let mut sum_a: f64 = 0.0;
    let mut sum_b: f64 = 0.0;
    let mut sum_ab: f64 = 0.0;
    let mut sum_b2: f64 = 0.0;
    for j in 0..timeperiod {
        let a = asset[j];
        let b = benchmark[j];
        sum_a += a;
        sum_b += b;
        sum_ab += a * b;
        sum_b2 += b * b;
    }

    // beta = Cov(asset, benchmark) / Var(benchmark)
    // Using population variance (÷n) to match TA-Lib
    // beta = (sum_ab - sum_a*sum_b/n) / (n * variance_b)
    // where variance_b = (sum_b2 - sum_b*sum_b/n) / n
    let variance_b = (sum_b2 - sum_b * sum_b / n) / n;
    if variance_b.abs() > 1e-15 {
        let covariance = (sum_ab - sum_a * sum_b / n) / n;
        output[timeperiod - 1] = covariance / variance_b;
    }

    // Subsequent windows — incremental O(1) update per step
    for i in timeperiod..len {
        let old_a = asset[i - timeperiod];
        let old_b = benchmark[i - timeperiod];
        let new_a = asset[i];
        let new_b = benchmark[i];
        sum_a += new_a - old_a;
        sum_b += new_b - old_b;
        sum_ab += new_a * new_b - old_a * old_b;
        sum_b2 += new_b * new_b - old_b * old_b;

        let variance_b = (sum_b2 - sum_b * sum_b / n) / n;
        if variance_b.abs() > 1e-15 {
            let covariance = (sum_ab - sum_a * sum_b / n) / n;
            output[i] = covariance / variance_b;
        }
    }

    Ok(output)
}

/// Correlation (相关系数 - Pearson)
///
/// 计算两个价格序列之间的滚动 Pearson 相关系数。
///
/// # 公式
/// r = Cov(X, Y) / (σ_X * σ_Y)
///
/// # 参数
/// * `input_a` - 第一个数据序列
/// * `input_b` - 第二个数据序列
/// * `timeperiod` - 滚动窗口大小
///
/// # 返回值
/// 相关系数数组，范围 [-1, 1]（前 `timeperiod - 1` 个值为 NaN）
///
/// # 说明
/// - r = 1: 完全正相关
/// - r = 0: 无相关性
/// - r = -1: 完全负相关
///
/// # 示例
/// ```rust
/// use finkit::indicators::correlation;
/// let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
/// let y = vec![2.0, 4.0, 6.0, 8.0, 10.0];
/// let result = correlation(&x, &y, 3).unwrap();
/// ```
pub fn correlation(input_a: &[f64], input_b: &[f64], timeperiod: usize) -> Result<Array1<f64>> {
    if input_a.len() != input_b.len() {
        return Err(TaError::InvalidParameter {
            name: "input_a and input_b".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    if timeperiod < 2 {
        return Err(TaError::InvalidParameter {
            name: "timeperiod".to_string(),
            constraint: "at least 2".to_string(),
        });
    }
    validate_input(input_a.len(), timeperiod)?;

    let len = input_a.len();
    let mut output = init_output(len);

    let n = timeperiod as f64;

    // Initialize accumulators with first window
    let mut sum_a: f64 = 0.0;
    let mut sum_b: f64 = 0.0;
    let mut sum_ab: f64 = 0.0;
    let mut sum_a2: f64 = 0.0;
    let mut sum_b2: f64 = 0.0;
    for j in 0..timeperiod {
        let a = input_a[j];
        let b = input_b[j];
        sum_a += a;
        sum_b += b;
        sum_ab += a * b;
        sum_a2 += a * a;
        sum_b2 += b * b;
    }

    // correlation = (n*sum_ab - sum_a*sum_b) / sqrt((n*sum_a2 - sum_a^2)*(n*sum_b2 - sum_b^2))
    let numerator = n * sum_ab - sum_a * sum_b;
    let denom_a = n * sum_a2 - sum_a * sum_a;
    let denom_b = n * sum_b2 - sum_b * sum_b;
    if denom_a > 1e-15 && denom_b > 1e-15 {
        let corr = numerator / (denom_a * denom_b).sqrt();
        output[timeperiod - 1] = corr.clamp(-1.0, 1.0);
    }

    // Subsequent windows — incremental O(1) update per step
    for i in timeperiod..len {
        let old_a = input_a[i - timeperiod];
        let old_b = input_b[i - timeperiod];
        let new_a = input_a[i];
        let new_b = input_b[i];
        sum_a += new_a - old_a;
        sum_b += new_b - old_b;
        sum_ab += new_a * new_b - old_a * old_b;
        sum_a2 += new_a * new_a - old_a * old_a;
        sum_b2 += new_b * new_b - old_b * old_b;

        let numerator = n * sum_ab - sum_a * sum_b;
        let denom_a = n * sum_a2 - sum_a * sum_a;
        let denom_b = n * sum_b2 - sum_b * sum_b;
        if denom_a > 1e-15 && denom_b > 1e-15 {
            let corr = numerator / (denom_a * denom_b).sqrt();
            output[i] = corr.clamp(-1.0, 1.0);
        }
    }

    Ok(output)
}

/// StdDev (标准差)
///
/// 计算滚动窗口的标准差（样本标准差）。
///
/// # 参数
/// * `input` - 输入数据序列
/// * `timeperiod` - 滚动窗口大小
/// * `nb_dev` - 标准差倍数（为了 API 兼容性保留，当前不使用）
///
/// # 返回值
/// 标准差数组（前 `timeperiod - 1` 个值为 NaN）
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
/// let result = indicators::std_dev(&data, 5, 1.0).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
pub fn std_dev(input: &[f64], timeperiod: usize, nb_dev: f64) -> Result<Array1<f64>> {
    if timeperiod < 2 {
        return Err(TaError::InvalidParameter {
            name: "timeperiod".to_string(),
            constraint: "at least 2".to_string(),
        });
    }
    validate_input(input.len(), timeperiod)?;

    let len = input.len();
    let mut output = init_output(len);
    let n = timeperiod as f64;
    let inv_n = 1.0 / n;

    // 总体标准差（÷n），匹配 TA-Lib TA_STDDEV.c
    let mut sum: f64 = 0.0;
    let mut sum_sq: f64 = 0.0;
    for i in 0..timeperiod {
        let x = input[i];
        sum += x;
        sum_sq += x * x;
    }
    let mean = sum * inv_n;
    let m2 = sum_sq - sum * mean;
    output[timeperiod - 1] = (m2 * inv_n).max(0.0).sqrt() * nb_dev;

    for i in timeperiod..len {
        let old = input[i - timeperiod];
        let new = input[i];
        sum += new - old;
        sum_sq += new * new - old * old;
        let m = sum * inv_n;
        let m2 = sum_sq - sum * m;
        output[i] = (m2 * inv_n).max(0.0).sqrt() * nb_dev;
    }

    Ok(output)
}

/// Var (方差)
///
/// 计算滚动窗口的总体方差，匹配 TA-Lib TA_VAR.c: variance = m2 / n
///
/// # 参数
/// * `input` - 输入数据序列
/// * `period` - 滚动窗口大小
/// * `nb_dev` - 未使用（为 API 兼容性保留，TA-Lib 中此参数无效）
///
/// # 返回值
/// 方差数组（前 `period - 1` 个值为 NaN）
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
/// let result = indicators::var(&data, 5, 1.0).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
pub fn var(input: &[f64], period: usize, nb_dev: f64) -> Result<Array1<f64>> {
    if period < 1 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "greater than 0".to_string(),
        });
    }
    validate_input(input.len(), period)?;

    let len = input.len();
    let mut output = init_output(len);

    let n = period as f64;
    // 总体方差（除以 n），匹配 TA-Lib TA_VAR.c: variance = m2 / n
    // nb_dev 作为缩放因子作用于结果（与 TA-Lib 一致）。
    let inv_n = 1.0 / n;

    let mut sum: f64 = 0.0;
    let mut sum_sq: f64 = 0.0;
    for i in 0..period {
        let x = input[i];
        sum += x;
        sum_sq += x * x;
    }
    let mean = sum * inv_n;
    // m2 = sum_sq - sum * mean 等价于 sum((x - mean)^2)
    let m2 = sum_sq - sum * mean;
    output[period - 1] = (m2 * inv_n * nb_dev).max(0.0);

    // 滑动窗口：O(1) 增量更新
    for i in period..len {
        let old = input[i - period];
        let new = input[i];
        sum += new - old;
        sum_sq += new * new - old * old;
        let m = sum * inv_n;
        let m2 = sum_sq - sum * m;
        output[i] = (m2 * inv_n * nb_dev).max(0.0);
    }

    Ok(output)
}

/// 线性回归 (Linear Regression)
///
/// 使用最小二乘法计算滚动线性回归的预测值。
///
/// # 参数
/// * `input` - 输入数据序列
/// * `timeperiod` - 滚动窗口大小
///
/// # 返回值
/// 线性回归预测值数组（前 `timeperiod - 1` 个值为 NaN）
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
/// let result = indicators::linear_reg(&data, 5).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
///
/// Legacy spelling retained for source compatibility in the 0.1.x line.
/// New internal code and generated bindings use [`linearreg`].
pub fn linear_reg(input: &[f64], timeperiod: usize) -> Result<Array1<f64>> {
    linreg(input, timeperiod)
}

/// Time Series Forecast (时间序列预测)
///
/// 使用线性回归预测下一个时间点的值。
/// TSF 是当前线性回归拟合曲线外推一个时间单位的预测值。
///
/// # 公式
/// TSF = intercept + slope * timeperiod
///
/// # 参数
/// * `input` - 输入数据序列
/// * `timeperiod` - 滚动窗口大小
///
/// # 返回值
/// 时间序列预测值数组（前 `timeperiod - 1` 个值为 NaN）
///
/// # 示例
/// ```rust
/// use finkit::indicators::tsf;
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
/// let result = tsf(&data, 5).unwrap();
/// ```
pub fn tsf(input: &[f64], timeperiod: usize) -> Result<Array1<f64>> {
    if timeperiod < 2 {
        return Err(TaError::InvalidParameter {
            name: "timeperiod".to_string(),
            constraint: "at least 2".to_string(),
        });
    }
    validate_input(input.len(), timeperiod)?;

    let len = input.len();
    let mut output = init_output(len);
    let p = timeperiod as f64;

    // Precompute constants for x = [0, 1, ..., timeperiod-1]
    let sum_x = p * (p - 1.0) / 2.0;
    let sum_x2 = p * (p - 1.0) * (2.0 * p - 1.0) / 6.0;
    let denom = p * sum_x2 - sum_x * sum_x;
    let last_x = (timeperiod - 1) as f64;

    if denom.abs() < 1e-15 {
        return Ok(output);
    }

    // Initialize accumulators with first window
    let mut sum_y: f64 = 0.0;
    let mut sum_xy: f64 = 0.0;
    for (j, &val) in input[..timeperiod].iter().enumerate() {
        sum_y += val;
        sum_xy += j as f64 * val;
    }

    // First window
    let slope = (p * sum_xy - sum_x * sum_y) / denom;
    let intercept = (sum_y - slope * sum_x) / p;
    output[timeperiod - 1] = slope * p + intercept;

    // Subsequent windows — incremental O(1) update per step
    for i in timeperiod..len {
        let old_val = input[i - timeperiod];
        let new_val = input[i];
        sum_xy += last_x * new_val - (sum_y - old_val);
        sum_y += new_val - old_val;
        let slope = (p * sum_xy - sum_x * sum_y) / denom;
        let intercept = (sum_y - slope * sum_x) / p;
        output[i] = slope * p + intercept;
    }

    Ok(output)
}

// ============================================================
// TA-Lib 命名别名 (Linear Regression 系列)
// ============================================================

/// Linear Regression 预测值 (TA-Lib 命名别名)
///
/// 滚动线性回归在当前点的预测值。这是 TA-Lib C 库 `LINEARREG` 的命名兼容函数。
///
/// # 参数
/// * `input` - 输入数据序列
/// * `period` - 滚动窗口大小
///
/// # 返回值
/// 线性回归预测值数组（前 `period - 1` 个值为 NaN）
///
/// # Examples
///
/// ```
/// use finkit::indicators::statistics::linearreg;
///
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
/// let result = linearreg(&data, 3).unwrap();
/// assert_eq!(result.len(), 5);
/// ```
pub fn linearreg(input: &[f64], period: usize) -> Result<Array1<f64>> {
    linreg(input, period)
}

/// Linear Regression Angle 角度 (TA-Lib 命名别名)
///
/// 滚动线性回归斜率对应的角度（度数）。这是 TA-Lib C 库 `LINEARREG_ANGLE` 的命名兼容函数。
///
/// # 参数
/// * `input` - 输入数据序列
/// * `period` - 滚动窗口大小
///
/// # 返回值
/// 线性回归角度（度数）数组（前 `period - 1` 个值为 NaN）
///
/// # Examples
///
/// ```
/// use finkit::indicators::statistics::linearreg_angle;
///
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
/// let result = linearreg_angle(&data, 3).unwrap();
/// assert_eq!(result.len(), 5);
/// ```
pub fn linearreg_angle(input: &[f64], period: usize) -> Result<Array1<f64>> {
    linreg_angle(input, period)
}

/// Linear Regression Intercept 截距 (TA-Lib 命名别名)
///
/// 滚动线性回归的截距。这是 TA-Lib C 库 `LINEARREG_INTERCEPT` 的命名兼容函数。
///
/// # 参数
/// * `input` - 输入数据序列
/// * `period` - 滚动窗口大小
///
/// # 返回值
/// 线性回归截距数组（前 `period - 1` 个值为 NaN）
///
/// # Examples
///
/// ```
/// use finkit::indicators::statistics::linearreg_intercept;
///
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
/// let result = linearreg_intercept(&data, 3).unwrap();
/// assert_eq!(result.len(), 5);
/// ```
pub fn linearreg_intercept(input: &[f64], period: usize) -> Result<Array1<f64>> {
    linreg_intercept(input, period)
}

/// Linear Regression Slope 斜率 (TA-Lib 命名别名)
///
/// 滚动线性回归的斜率。这是 TA-Lib C 库 `LINEARREG_SLOPE` 的命名兼容函数。
///
/// # 参数
/// * `input` - 输入数据序列
/// * `period` - 滚动窗口大小
///
/// # 返回值
/// 线性回归斜率数组（前 `period - 1` 个值为 NaN）
///
/// # Examples
///
/// ```
/// use finkit::indicators::statistics::linearreg_slope;
///
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
/// let result = linearreg_slope(&data, 3).unwrap();
/// assert_eq!(result.len(), 5);
/// ```
pub fn linearreg_slope(input: &[f64], period: usize) -> Result<Array1<f64>> {
    linreg_slope(input, period)
}

// ============================================================
// TA-Lib SKEWNESS / KURTOSIS (新增 2026-07-07)
// ============================================================

/// Rolling Skewness (Fisher-Pearson 偏度) — TA-Lib `TA_SKEWNESS` 兼容
///
/// 度量滚动窗口内数据分布的不对称性。正偏度表示右尾较长（大多数值在均值左侧），
/// 负偏度表示左尾较长。零偏度表示对称分布（接近正态）。
///
/// # 公式
/// `g1 = (1/n) * Σ((x_i - μ)/σ)^3`，其中 μ 是窗口均值，σ 是窗口标准差。
///
/// # 参数
/// * `input` - 输入数据序列
/// * `timeperiod` - 滚动窗口大小（必须 `>= 3`）
///
/// # 返回值
/// 偏度数组（前 `timeperiod - 1` 个值为 `NaN`）
///
/// # 示例
/// ```rust
/// use finkit::indicators::statistics::skewness;
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
/// let r = skewness(&data, 5).unwrap();
/// assert_eq!(r.len(), 10);
/// assert!(r.as_slice().unwrap().iter().take(4).all(|x| x.is_nan()));
/// ```
pub fn skewness(input: &[f64], timeperiod: usize) -> Result<Array1<f64>> {
    if timeperiod < 3 {
        return Err(TaError::InvalidParameter {
            name: "timeperiod".to_string(),
            constraint: "at least 3".to_string(),
        });
    }
    validate_input(input.len(), timeperiod)?;

    let len = input.len();
    let mut output = init_output(len);
    let n = timeperiod as f64;
    let inv_n = 1.0 / n;
    // population variance divisor (matches TA-Lib behavior)
    let inv_n_pop = 1.0 / n;

    // Sliding window: maintain sum, sum_sq, sum_cu in O(1) per step
    let mut sum: f64 = 0.0;
    let mut sum_sq: f64 = 0.0;
    let mut sum_cu: f64 = 0.0;
    for j in 0..timeperiod {
        let v = input[j];
        sum += v;
        sum_sq += v * v;
        sum_cu += v * v * v;
    }

    for i in (timeperiod - 1)..len {
        if i >= timeperiod {
            let old = input[i - timeperiod];
            let new_v = input[i];
            sum += new_v - old;
            sum_sq += new_v * new_v - old * old;
            sum_cu += new_v * new_v * new_v - old * old * old;
        }
        let mean = sum * inv_n;
        let m2 = sum_sq * inv_n - mean * mean; // 2nd central moment
        let m3 = sum_cu * inv_n_pop - 3.0 * mean * sum_sq * inv_n_pop + 2.0 * mean * mean * mean; // 3rd central moment
                                                                                                  // skewness = m3 / m2^1.5
        let denom = m2 * m2.sqrt();
        output[i] = if denom > 1e-15 { m3 / denom } else { 0.0 };
    }
    Ok(output)
}

/// Rolling Kurtosis (超出度) — TA-Lib `TA_KURTOSIS` 兼容
///
/// 度量滚动窗口内数据分布的尾部厚度。正超出度表示重尾（极端值多于正态分布），
/// 负超出度表示轻尾。零值与正态分布一致。
///
/// # 公式
/// `g2 = (1/n) * Σ((x_i - μ)/σ)^4 - 3`，即 excess kurtosis。
///
/// # 参数
/// * `input` - 输入数据序列
/// * `timeperiod` - 滚动窗口大小（必须 `>= 4`）
///
/// # 返回值
/// 超出度数组（前 `timeperiod - 1` 个值为 `NaN`）
///
/// # 示例
/// ```rust
/// use finkit::indicators::statistics::kurtosis;
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
/// let r = kurtosis(&data, 5).unwrap();
/// assert_eq!(r.len(), 10);
/// ```
pub fn kurtosis(input: &[f64], timeperiod: usize) -> Result<Array1<f64>> {
    if timeperiod < 4 {
        return Err(TaError::InvalidParameter {
            name: "timeperiod".to_string(),
            constraint: "at least 4".to_string(),
        });
    }
    validate_input(input.len(), timeperiod)?;

    let len = input.len();
    let mut output = init_output(len);
    let n = timeperiod as f64;
    let inv_n = 1.0 / n;
    let inv_n_pop = 1.0 / n;

    let mut sum: f64 = 0.0;
    let mut sum_sq: f64 = 0.0;
    let mut sum_cu: f64 = 0.0;
    let mut sum_qu: f64 = 0.0;
    for j in 0..timeperiod {
        let v = input[j];
        let v2 = v * v;
        sum += v;
        sum_sq += v2;
        sum_cu += v2 * v;
        sum_qu += v2 * v2;
    }

    for i in (timeperiod - 1)..len {
        if i >= timeperiod {
            let old = input[i - timeperiod];
            let new_v = input[i];
            let old2 = old * old;
            let new2 = new_v * new_v;
            sum += new_v - old;
            sum_sq += new2 - old2;
            sum_cu += new2 * new_v - old2 * old;
            sum_qu += new2 * new2 - old2 * old2;
        }
        let mean = sum * inv_n;
        let m2 = sum_sq * inv_n - mean * mean;
        let m4 = sum_qu * inv_n_pop - 4.0 * mean * sum_cu * inv_n_pop
            + 6.0 * mean * mean * sum_sq * inv_n_pop
            - 3.0 * mean.powi(4);
        // excess kurtosis = m4 / m2^2 - 3
        let denom = m2 * m2;
        output[i] = if denom > 1e-15 { m4 / denom - 3.0 } else { 0.0 };
    }
    Ok(output)
}

// ============================================================
// Internal helper functions
// ============================================================

#[allow(dead_code)]
fn calculate_returns(prices: &[f64]) -> Result<Vec<f64>> {
    if prices.len() < 2 {
        return Err(TaError::InsufficientData {
            length: prices.len(),
            required: 2,
        });
    }

    let mut returns = Vec::with_capacity(prices.len());
    returns.push(0.0);

    for i in 1..prices.len() {
        if prices[i - 1].abs() > 1e-15 {
            returns.push((prices[i] - prices[i - 1]) / prices[i - 1]);
        } else {
            returns.push(0.0);
        }
    }

    Ok(returns)
}

// ============================================================
// Unit tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_zscore_basic() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let result = zscore(&data, 5).unwrap();

        for i in 0..4 {
            assert!(result[i].is_nan());
        }

        assert!(!result[4].is_nan());
        assert_relative_eq!(result[4], 1.2649110640673518, epsilon = 1e-6);
    }

    #[test]
    fn test_zscore_constant() {
        let data = vec![5.0; 10];
        let result = zscore(&data, 3).unwrap();

        assert!(result[2].is_nan());
    }

    #[test]
    fn test_zscore_invalid_period() {
        let data = vec![1.0, 2.0, 3.0];
        assert!(zscore(&data, 0).is_err());
        assert!(zscore(&data, 1).is_err());
    }

    #[test]
    fn test_percent_rank_basic() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = percent_rank(&data, 3).unwrap();

        assert!(result[0].is_nan());
        assert!(result[1].is_nan());

        assert_relative_eq!(result[2], 66.66666666666666, epsilon = 1e-6);
        assert_relative_eq!(result[3], 66.66666666666666, epsilon = 1e-6);
    }

    #[test]
    fn test_percent_rank_minimum() {
        let data = vec![5.0, 4.0, 3.0, 2.0, 1.0];
        let result = percent_rank(&data, 3).unwrap();

        assert_relative_eq!(result[2], 0.0, epsilon = 1e-6);
    }

    #[test]
    fn test_percent_rank_period_1() {
        let data = vec![1.0, 2.0, 3.0];
        let result = percent_rank(&data, 1).unwrap();

        assert_relative_eq!(result[0], 0.0, epsilon = 1e-6);
        assert_relative_eq!(result[1], 0.0, epsilon = 1e-6);
    }

    #[test]
    fn test_beta_basic() {
        let benchmark = vec![100.0, 101.0, 102.0, 103.0, 104.0, 105.0, 106.0];
        let asset = vec![100.0, 102.0, 104.0, 106.0, 108.0, 110.0, 112.0];

        let result = beta(&asset, &benchmark, 5).unwrap();

        assert!(!result[4].is_nan());
        assert!(result[4] > 1.9);
        assert!(result[4] < 2.1);
    }

    #[test]
    fn test_beta_same_movement() {
        let benchmark = vec![100.0, 101.0, 102.0, 103.0, 104.0, 105.0];
        let asset = benchmark.clone();

        let result = beta(&asset, &benchmark, 5).unwrap();

        assert!(!result[4].is_nan());
        assert_relative_eq!(result[4], 1.0, epsilon = 1e-3);
    }

    #[test]
    fn test_beta_mismatched_length() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0];
        assert!(beta(&a, &b, 2).is_err());
    }

    #[test]
    fn test_correlation_perfect_positive() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![2.0, 4.0, 6.0, 8.0, 10.0];

        let result = correlation(&x, &y, 3).unwrap();

        assert_relative_eq!(result[2], 1.0, epsilon = 1e-6);
        assert_relative_eq!(result[3], 1.0, epsilon = 1e-6);
        assert_relative_eq!(result[4], 1.0, epsilon = 1e-6);
    }

    #[test]
    fn test_correlation_perfect_negative() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![10.0, 8.0, 6.0, 4.0, 2.0];

        let result = correlation(&x, &y, 3).unwrap();

        assert_relative_eq!(result[2], -1.0, epsilon = 1e-6);
    }

    #[test]
    fn test_correlation_no_correlation() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![5.0, 3.0, 4.0, 1.0, 2.0];

        let result = correlation(&x, &y, 5).unwrap();

        assert!(!result[4].is_nan());
        assert!(result[4] < 0.0);
        assert!(result[4] > -1.0);
    }

    #[test]
    fn test_correlation_mismatched_length() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0];
        assert!(correlation(&a, &b, 2).is_err());
    }

    #[test]
    fn test_linear_reg_basic() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = linear_reg(&data, 3).unwrap();

        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert_relative_eq!(result[2], 3.0, epsilon = 1e-6);
    }

    #[test]
    fn test_std_dev_basic() {
        let data = vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        let result = std_dev(&data, 5, 1.0).unwrap();

        assert!(result[0].is_nan());
        assert!(result[3].is_nan());

        assert!(!result[4].is_nan());
    }

    #[test]
    fn test_var() {
        let data = vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        let result = var(&data, 5, 1.0).unwrap();

        for i in 0..4 {
            assert!(result[i].is_nan());
        }

        // TA-Lib uses population variance (÷n): m2=4.8, var=4.8/5=0.96
        assert_relative_eq!(result[4], 0.96, epsilon = 1e-6);

        let scaled = var(&data, 5, 2.0).unwrap();
        assert_relative_eq!(scaled[4], 1.92, epsilon = 1e-6);

        assert!(var(&data, 0, 1.0).is_err());
        assert!(var(&[1.0, 2.0], 5, 1.0).is_err());
    }

    #[test]
    fn test_tsf_basic() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let result = tsf(&data, 5).unwrap();

        for i in 0..4 {
            assert!(result[i].is_nan());
        }

        assert_relative_eq!(result[4], 6.0, epsilon = 1e-6);
    }

    #[test]
    fn test_tsf_flat_series() {
        let data = vec![5.0; 10];
        let result = tsf(&data, 5).unwrap();

        assert!(!result[4].is_nan());
        assert_relative_eq!(result[4], 5.0, epsilon = 1e-6);
    }

    #[test]
    fn test_tsf_invalid_period() {
        let data = vec![1.0, 2.0, 3.0];
        assert!(tsf(&data, 0).is_err());
        assert!(tsf(&data, 1).is_err());
    }

    #[test]
    fn test_zscore_insufficient_data() {
        let data = vec![1.0, 2.0];
        assert!(zscore(&data, 5).is_err());
    }

    #[test]
    fn test_percent_rank_insufficient_data() {
        let data = vec![1.0, 2.0];
        assert!(percent_rank(&data, 5).is_err());
    }

    #[test]
    fn test_beta_insufficient_data() {
        let a = vec![1.0, 2.0];
        let b = vec![3.0, 4.0];
        assert!(beta(&a, &b, 5).is_err());
    }

    #[test]
    fn test_linearreg_basic() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = linearreg(&data, 3).unwrap();

        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert_relative_eq!(result[2], 3.0, epsilon = 1e-6);
    }

    #[test]
    fn test_linearreg_slope_basic() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = linearreg_slope(&data, 3).unwrap();

        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert_relative_eq!(result[2], 1.0, epsilon = 1e-6);
        assert_relative_eq!(result[3], 1.0, epsilon = 1e-6);
        assert_relative_eq!(result[4], 1.0, epsilon = 1e-6);
    }

    #[test]
    fn test_linearreg_intercept_basic() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = linearreg_intercept(&data, 3).unwrap();

        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert_relative_eq!(result[2], 1.0, epsilon = 1e-6);
    }

    #[test]
    fn test_linearreg_angle_basic() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = linearreg_angle(&data, 3).unwrap();

        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        let expected = 1.0_f64.atan() * 180.0 / std::f64::consts::PI;
        assert_relative_eq!(result[2], expected, epsilon = 1e-6);
    }

    #[test]
    fn test_avgdev_basic_14() {
        // 14-period AVGDEV on simple integer sequence [1..=14]
        let data: Vec<f64> = (1..=14).map(|x| x as f64).collect();
        let result = avgdev(&data, 14).unwrap();

        // First 13 values should be NaN
        for i in 0..13 {
            assert!(result[i].is_nan(), "expected NaN at index {}", i);
        }

        // Hand-computed: window is [1..=14], mean = 7.5
        // sum |x - 7.5| = 6.5 + 5.5 + 4.5 + 3.5 + 2.5 + 1.5 + 0.5 + 0.5 + 1.5 + 2.5 + 3.5 + 4.5 + 5.5 + 6.5
        // = 49.0
        // AVGDEV = 49.0 / 14 = 3.5
        let mean: f64 = data.iter().sum::<f64>() / 14.0;
        let dev_sum: f64 = data.iter().map(|&x| (x - mean).abs()).sum();
        let expected = dev_sum / 14.0;
        assert_relative_eq!(result[13], expected, epsilon = 1e-10);
        assert_relative_eq!(result[13], 3.5, epsilon = 1e-10);
    }

    #[test]
    fn test_avgdev_period_one() {
        let data = vec![3.0, 7.0, 1.0, 5.0, 9.0];
        let result = avgdev(&data, 1).unwrap();
        // period=1: deviation is always zero
        for i in 0..data.len() {
            assert_eq!(result[i], 0.0, "expected 0 at index {}", i);
        }
    }

    #[test]
    fn test_avgdev_invalid_period() {
        let data = vec![1.0, 2.0, 3.0];
        assert!(avgdev(&data, 0).is_err());
    }

    #[test]
    fn test_avgdev_rolling() {
        // Verify rolling behaviour: known window of [1..=5] gives
        // mean=3, dev_sum = 2+1+0+1+2 = 6, avgdev = 6/5 = 1.2
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
        let result = avgdev(&data, 5).unwrap();
        for i in 0..4 {
            assert!(result[i].is_nan(), "expected NaN at index {}", i);
        }
        assert_relative_eq!(result[4], 1.2, epsilon = 1e-10);
        // Window [2..=6]: mean=4, dev_sum = 2+1+0+1+2 = 6, avgdev = 1.2
        assert_relative_eq!(result[5], 1.2, epsilon = 1e-10);
        // Window [3..=7]: mean=5, dev_sum = 2+1+0+1+2 = 6, avgdev = 1.2
        assert_relative_eq!(result[6], 1.2, epsilon = 1e-10);
    }

    // ---------- SKEWNESS / KURTOSIS (新增 2026-07-07) ----------

    #[test]
    fn test_skewness_symmetric_zero() {
        // Symmetric distribution → skewness ≈ 0
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 4.0, 3.0, 2.0, 1.0];
        let r = skewness(&data, 5).unwrap();
        // First 4 NaN, then valid values
        for i in 0..4 {
            assert!(r[i].is_nan());
        }
        assert!(!r[4].is_nan());
        assert!(
            r[4].abs() < 1e-9,
            "expected ~0 symmetric skew, got {}",
            r[4]
        );
    }

    #[test]
    fn test_skewness_right_tail_positive() {
        // Right-skewed: long right tail (one big outlier)
        let data = vec![1.0, 2.0, 3.0, 4.0, 100.0, 1.0, 2.0, 3.0, 4.0, 100.0];
        let r = skewness(&data, 5).unwrap();
        // window [1,2,3,4,100] is right-skewed → positive skewness
        assert!(r[4] > 0.5, "expected positive skew, got {}", r[4]);
    }

    #[test]
    fn test_skewness_invalid_period() {
        let data = vec![1.0; 10];
        assert!(skewness(&data, 0).is_err());
        assert!(skewness(&data, 2).is_err());
        // valid minimum is 3
        let r = skewness(&data, 3).unwrap();
        assert_eq!(r.len(), 10);
    }

    #[test]
    fn test_kurtosis_normal_zero() {
        // Uniform-ish data → near-zero excess kurtosis
        let data: Vec<f64> = (1..=20).map(|x| x as f64).collect();
        let r = kurtosis(&data, 10).unwrap();
        // First 9 NaN
        for i in 0..9 {
            assert!(r[i].is_nan());
        }
        assert!(!r[9].is_nan());
        // Linear data is bounded → finite kurtosis, but not necessarily 0
        assert!(r[9].is_finite());
    }

    #[test]
    fn test_kurtosis_heavy_tail_positive() {
        // Heavy-tailed: most values small, two big outliers
        let data = vec![1.0, 1.0, 1.0, 100.0, 1.0, 1.0, 1.0, 100.0, 1.0, 1.0];
        let r = kurtosis(&data, 5).unwrap();
        // window [1,1,1,100,1] is heavy-tailed → positive excess kurtosis
        assert!(r[4] > 0.0, "expected positive kurtosis, got {}", r[4]);
    }

    #[test]
    fn test_kurtosis_invalid_period() {
        let data = vec![1.0; 10];
        assert!(kurtosis(&data, 0).is_err());
        assert!(kurtosis(&data, 3).is_err());
        // valid minimum is 4
        let r = kurtosis(&data, 4).unwrap();
        assert_eq!(r.len(), 10);
    }
}
