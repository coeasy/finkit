use crate::error::{Result, TaError};
use crate::utils::{init_output, validate_input};
use ndarray::Array1;

/// Pivot Points calculation method
pub enum PivotMethod {
    /// Standard (Floor) Pivots
    Standard,
    /// Fibonacci Pivots
    Fibonacci,
    /// Woodie's Pivots
    Woodie,
    /// Camarilla Pivots
    Camarilla,
    /// DeMark Pivots
    DeMark,
}

/// Pivot Points Result
///
/// Contains pivot point and support/resistance levels.
#[derive(Debug, Clone)]
pub struct PivotResult {
    /// Pivot Point
    pub pivot: Array1<f64>,
    /// Resistance Level 1
    pub r1: Array1<f64>,
    /// Resistance Level 2
    pub r2: Array1<f64>,
    /// Resistance Level 3
    pub r3: Array1<f64>,
    /// Support Level 1
    pub s1: Array1<f64>,
    /// Support Level 2
    pub s2: Array1<f64>,
    /// Support Level 3
    pub s3: Array1<f64>,
}

/// Pivot Points (PIVOT)
///
/// Calculates pivot points and support/resistance levels based on previous period's
/// high, low, and close prices. Different methods produce different level calculations.
///
/// # Methods
/// - **Standard**: Traditional floor trader pivots
///   - P = (H + L + C) / 3
///   - R1 = 2*P - L, S1 = 2*P - H
///   - R2 = P + (H - L), S2 = P - (H - L)
///   - R3 = H + 2*(P - L), S3 = L - 2*(H - P)
///
/// - **Fibonacci**: Uses Fibonacci ratios
///   - P = (H + L + C) / 3
///   - R1 = P + 0.382*(H - L), S1 = P - 0.382*(H - L)
///   - R2 = P + 0.618*(H - L), S2 = P - 0.618*(H - L)
///   - R3 = P + 1.0*(H - L), S3 = P - 1.0*(H - L)
///
/// - **Woodie**: Gives more weight to the close
///   - P = (H + L + 2*C) / 4
///   - Same R/S formulas as Standard
///
/// - **Camarilla**: Tighter ranges using Fibonacci ratios
///   - P = (H + L + C) / 3
///   - R1 = C + (H - L) * 1.1/12, S1 = C - (H - L) * 1.1/12
///   - R2 = C + (H - L) * 1.1/6, S2 = C - (H - L) * 1.1/6
///   - R3 = C + (H - L) * 1.1/4, S3 = C - (H - L) * 1.1/4
///
/// - **DeMark**: Simplified calculation
///   - If C < O: P = H + 2*L + C, else if C > O: P = 2*H + L + C, else: P = H + L + 2*C
///   - R1 = P/2 - L, S1 = P/2 - H
///
/// # Arguments
/// * `high` - High prices (previous period highs)
/// * `low` - Low prices (previous period lows)
/// * `close` - Close prices (previous period closes)
/// * `method` - Calculation method (0=Standard, 1=Fibonacci, 2=Woodie, 3=Camarilla, 4=DeMark)
///
/// # Returns
/// PivotResult containing pivot point and R1-R3, S1-S3 levels
///
/// # Example
/// ```rust
/// use alpha_ta_core::indicators::{pivot_points, PivotMethod};
///
/// let high = vec![105.0, 106.0, 107.0, 108.0];
/// let low = vec![100.0, 101.0, 102.0, 103.0];
/// let close = vec![103.0, 104.0, 105.0, 106.0];
///
/// let result = pivot_points(&high, &low, &close, PivotMethod::Standard).unwrap();
/// assert_eq!(result.pivot.len(), 4);
/// ```
pub fn pivot_points(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    method: PivotMethod,
) -> Result<PivotResult> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(high.len(), 1)?;

    let len = high.len();
    let mut pivot = init_output(len);
    let mut r1 = init_output(len);
    let mut r2 = init_output(len);
    let mut r3 = init_output(len);
    let mut s1 = init_output(len);
    let mut s2 = init_output(len);
    let mut s3 = init_output(len);

    for i in 0..len {
        let h = high[i];
        let l = low[i];
        let c = close[i];
        let range = h - l;

        match method {
            PivotMethod::Standard => {
                let p = (h + l + c) / 3.0;
                pivot[i] = p;
                r1[i] = 2.0 * p - l;
                s1[i] = 2.0 * p - h;
                r2[i] = p + range;
                s2[i] = p - range;
                r3[i] = h + 2.0 * (p - l);
                s3[i] = l - 2.0 * (h - p);
            }
            PivotMethod::Fibonacci => {
                let p = (h + l + c) / 3.0;
                pivot[i] = p;
                r1[i] = p + 0.382 * range;
                s1[i] = p - 0.382 * range;
                r2[i] = p + 0.618 * range;
                s2[i] = p - 0.618 * range;
                r3[i] = p + 1.0 * range;
                s3[i] = p - 1.0 * range;
            }
            PivotMethod::Woodie => {
                let p = (h + l + 2.0 * c) / 4.0;
                pivot[i] = p;
                r1[i] = 2.0 * p - l;
                s1[i] = 2.0 * p - h;
                r2[i] = p + range;
                s2[i] = p - range;
                r3[i] = h + 2.0 * (p - l);
                s3[i] = l - 2.0 * (h - p);
            }
            PivotMethod::Camarilla => {
                let p = (h + l + c) / 3.0;
                pivot[i] = p;
                r1[i] = c + range * 1.1 / 12.0;
                s1[i] = c - range * 1.1 / 12.0;
                r2[i] = c + range * 1.1 / 6.0;
                s2[i] = c - range * 1.1 / 6.0;
                r3[i] = c + range * 1.1 / 4.0;
                s3[i] = c - range * 1.1 / 4.0;
            }
            PivotMethod::DeMark => {
                let x = if c < *high.get(i.wrapping_sub(1)).unwrap_or(&c) {
                    h + 2.0 * l + c
                } else if c > *high.get(i.wrapping_sub(1)).unwrap_or(&c) {
                    2.0 * h + l + c
                } else {
                    h + l + 2.0 * c
                };
                pivot[i] = x / 4.0;
                r1[i] = x / 2.0 - l;
                s1[i] = x / 2.0 - h;
                // DeMark only has R1/S1, fill R2/S2/R3/S3 with NaN
                r2[i] = f64::NAN;
                s2[i] = f64::NAN;
                r3[i] = f64::NAN;
                s3[i] = f64::NAN;
            }
        }
    }

    Ok(PivotResult {
        pivot,
        r1,
        r2,
        r3,
        s1,
        s2,
        s3,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_pivot_standard_basic() {
        let high = vec![105.0, 106.0, 107.0];
        let low = vec![100.0, 101.0, 102.0];
        let close = vec![103.0, 104.0, 105.0];

        let result = pivot_points(&high, &low, &close, PivotMethod::Standard).unwrap();

        assert_eq!(result.pivot.len(), 3);

        // First bar: H=105, L=100, C=103
        // P = (105+100+103)/3 = 102.667
        assert_relative_eq!(
            result.pivot[0],
            (105.0 + 100.0 + 103.0) / 3.0,
            epsilon = 1e-10
        );
        // R1 = 2*P - L = 205.333 - 100 = 105.333
        assert_relative_eq!(result.r1[0], 2.0 * result.pivot[0] - 100.0, epsilon = 1e-10);
        // S1 = 2*P - H = 205.333 - 105 = 100.333
        assert_relative_eq!(result.s1[0], 2.0 * result.pivot[0] - 105.0, epsilon = 1e-10);
    }

    #[test]
    fn test_pivot_fibonacci() {
        let high = vec![110.0];
        let low = vec![100.0];
        let close = vec![105.0];

        let result = pivot_points(&high, &low, &close, PivotMethod::Fibonacci).unwrap();

        let p = (110.0 + 100.0 + 105.0) / 3.0;
        assert_relative_eq!(result.pivot[0], p, epsilon = 1e-10);
        assert_relative_eq!(result.r1[0], p + 0.382 * 10.0, epsilon = 1e-10);
        assert_relative_eq!(result.s1[0], p - 0.382 * 10.0, epsilon = 1e-10);
        assert_relative_eq!(result.r2[0], p + 0.618 * 10.0, epsilon = 1e-10);
        assert_relative_eq!(result.s2[0], p - 0.618 * 10.0, epsilon = 1e-10);
    }

    #[test]
    fn test_pivot_woodie() {
        let high = vec![110.0];
        let low = vec![100.0];
        let close = vec![105.0];

        let result = pivot_points(&high, &low, &close, PivotMethod::Woodie).unwrap();

        let p = (110.0 + 100.0 + 2.0 * 105.0) / 4.0;
        assert_relative_eq!(result.pivot[0], p, epsilon = 1e-10);
    }

    #[test]
    fn test_pivot_camarilla() {
        let high = vec![110.0];
        let low = vec![100.0];
        let close = vec![105.0];

        let result = pivot_points(&high, &low, &close, PivotMethod::Camarilla).unwrap();

        let p = (110.0 + 100.0 + 105.0) / 3.0;
        assert_relative_eq!(result.pivot[0], p, epsilon = 1e-10);
        assert_relative_eq!(result.r1[0], 105.0 + 10.0 * 1.1 / 12.0, epsilon = 1e-10);
        assert_relative_eq!(result.s1[0], 105.0 - 10.0 * 1.1 / 12.0, epsilon = 1e-10);
    }

    #[test]
    fn test_pivot_invalid_lengths() {
        let high = vec![105.0, 106.0];
        let low = vec![100.0];
        let close = vec![103.0, 104.0];

        assert!(pivot_points(&high, &low, &close, PivotMethod::Standard).is_err());
    }

    #[test]
    fn test_pivot_insufficient_data() {
        let high: Vec<f64> = vec![];
        let low: Vec<f64> = vec![];
        let close: Vec<f64> = vec![];

        assert!(pivot_points(&high, &low, &close, PivotMethod::Standard).is_err());
    }

    #[test]
    fn test_pivot_r_s_relationship() {
        let high = vec![110.0];
        let low = vec![100.0];
        let close = vec![105.0];

        let result = pivot_points(&high, &low, &close, PivotMethod::Standard).unwrap();

        // Resistance levels should be above pivot
        assert!(result.r1[0] > result.pivot[0]);
        assert!(result.r2[0] > result.pivot[0]);
        assert!(result.r3[0] > result.pivot[0]);

        // Support levels should be below pivot
        assert!(result.s1[0] < result.pivot[0]);
        assert!(result.s2[0] < result.pivot[0]);
        assert!(result.s3[0] < result.pivot[0]);

        // R levels should be ordered
        assert!(result.r3[0] > result.r2[0]);
        assert!(result.r2[0] > result.r1[0]);

        // S levels should be ordered
        assert!(result.s1[0] > result.s2[0]);
        assert!(result.s2[0] > result.s3[0]);
    }
}
