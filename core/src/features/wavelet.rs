//! Discrete Wavelet Transform (DWT) features for time series.

use crate::error::{Result, TaError};

/// Wavelet basis function type.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WaveletBasis {
    /// Haar wavelet (simplest orthogonal wavelet)
    Haar,
    /// Daubechies-4 wavelet
    Db4,
}

/// DWT feature extraction result.
#[derive(Debug, Clone)]
pub struct DwtFeatures {
    /// Energy at each decomposition level
    pub energy: Vec<f64>,
    /// Relative energy distribution (normalized)
    pub energy_ratio: Vec<f64>,
    /// Standard deviation of detail coefficients at each level
    pub detail_std: Vec<f64>,
    /// Mean of detail coefficients at each level
    pub detail_mean: Vec<f64>,
    /// Number of decomposition levels
    pub levels: usize,
}

/// Extract DWT features from a time series.
///
/// Performs multi-level discrete wavelet decomposition and computes
/// statistical features from the detail and approximation coefficients.
///
/// # Arguments
/// * `data` - Input time series
/// * `basis` - Wavelet basis function (Haar or Db4)
/// * `max_level` - Maximum decomposition levels (0 = auto based on data length)
///
/// # Returns
/// DwtFeatures with energy, energy_ratio, detail_std, detail_mean per level
pub fn dwt_features(data: &[f64], basis: WaveletBasis, max_level: usize) -> Result<DwtFeatures> {
    let n = data.len();
    if n < 4 {
        return Err(TaError::InvalidParameter {
            name: "data".to_string(),
            constraint: "length must be >= 4".to_string(),
        });
    }

    let filter = get_filter(basis);
    let filter_len = filter.low_pass.len();

    let auto_levels = ((n as f64).log2().floor() as usize).saturating_sub(1);
    let levels = if max_level == 0 {
        auto_levels.min(6)
    } else {
        max_level.min(auto_levels)
    };

    if levels == 0 {
        return Err(TaError::InvalidParameter {
            name: "max_level".to_string(),
            constraint: "data too short for decomposition".to_string(),
        });
    }

    let mut approx = data.to_vec();
    let mut energy = Vec::with_capacity(levels);
    let mut detail_std = Vec::with_capacity(levels);
    let mut detail_mean = Vec::with_capacity(levels);

    for _ in 0..levels {
        if approx.len() < filter_len {
            break;
        }
        let (a, d) = dwt_single_level(&approx, &filter);

        // Detail coefficient statistics
        let d_energy: f64 = d.iter().map(|x| x * x).sum();
        energy.push(d_energy);

        let d_mean = if d.is_empty() {
            0.0
        } else {
            d.iter().sum::<f64>() / d.len() as f64
        };
        detail_mean.push(d_mean);

        let d_std = if d.len() < 2 {
            0.0
        } else {
            let var = d.iter().map(|x| (x - d_mean) * (x - d_mean)).sum::<f64>() / (d.len() - 1) as f64;
            var.sqrt()
        };
        detail_std.push(d_std);

        approx = a;
    }

    // Add approximation energy at last level
    let approx_energy: f64 = approx.iter().map(|x| x * x).sum();
    energy.push(approx_energy);

    let total_energy: f64 = energy.iter().sum();
    let energy_ratio = if total_energy > 1e-15 {
        energy.iter().map(|e| e / total_energy).collect()
    } else {
        vec![0.0; energy.len()]
    };

    let actual_levels = detail_std.len();

    Ok(DwtFeatures {
        energy,
        energy_ratio,
        detail_std,
        detail_mean,
        levels: actual_levels,
    })
}

struct WaveletFilter {
    low_pass: Vec<f64>,
    high_pass: Vec<f64>,
}

fn get_filter(basis: WaveletBasis) -> WaveletFilter {
    match basis {
        WaveletBasis::Haar => {
            let s = 1.0 / std::f64::consts::SQRT_2;
            WaveletFilter {
                low_pass: vec![s, s],
                high_pass: vec![s, -s],
            }
        }
        WaveletBasis::Db4 => {
            let h = vec![
                0.48296291314469025,
                0.8365163037378079,
                0.22414386804185735,
                -0.12940952255092145,
            ];
            let g = vec![
                -0.12940952255092145,
                -0.22414386804185735,
                0.8365163037378079,
                -0.48296291314469025,
            ];
            WaveletFilter {
                low_pass: h,
                high_pass: g,
            }
        }
    }
}

/// Single-level DWT decomposition.
fn dwt_single_level(data: &[f64], filter: &WaveletFilter) -> (Vec<f64>, Vec<f64>) {
    let n = data.len();
    let flen = filter.low_pass.len();
    let out_len = n / 2;

    let mut approx = Vec::with_capacity(out_len);
    let mut detail = Vec::with_capacity(out_len);

    for i in 0..out_len {
        let mut a = 0.0;
        let mut d = 0.0;
        for j in 0..flen {
            let idx = (2 * i + j) % n;
            a += filter.low_pass[j] * data[idx];
            d += filter.high_pass[j] * data[idx];
        }
        approx.push(a);
        detail.push(d);
    }

    (approx, detail)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dwt_features_haar() {
        let data: Vec<f64> = (0..64).map(|i| (i as f64 * 0.3).sin() * 10.0).collect();
        let result = dwt_features(&data, WaveletBasis::Haar, 4).unwrap();
        assert_eq!(result.levels, 4);
        assert_eq!(result.energy.len(), 5); // 4 detail + 1 approx
        assert_eq!(result.energy_ratio.len(), 5);
        assert_eq!(result.detail_std.len(), 4);
        assert_eq!(result.detail_mean.len(), 4);
        // Energy ratios sum to ~1.0
        let sum: f64 = result.energy_ratio.iter().sum();
        assert!((sum - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_dwt_features_db4() {
        let data: Vec<f64> = (0..64).map(|i| (i as f64 * 0.3).sin() * 10.0).collect();
        let result = dwt_features(&data, WaveletBasis::Db4, 4).unwrap();
        assert_eq!(result.levels, 4);
        let sum: f64 = result.energy_ratio.iter().sum();
        assert!((sum - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_dwt_features_auto_level() {
        let data: Vec<f64> = (0..128).map(|i| (i as f64 * 0.5).sin() * 5.0).collect();
        let result = dwt_features(&data, WaveletBasis::Haar, 0).unwrap();
        assert!(result.levels >= 3);
    }

    #[test]
    fn test_dwt_features_invalid() {
        assert!(dwt_features(&[1.0, 2.0, 3.0], WaveletBasis::Haar, 2).is_err());
    }

    #[test]
    fn test_dwt_constant_signal() {
        let data = vec![5.0; 32];
        let result = dwt_features(&data, WaveletBasis::Haar, 3).unwrap();
        // Constant signal: all energy in approximation, detail energy ~0
        for &e in result.detail_std.iter() {
            assert!(e < 1e-10, "Detail std should be ~0 for constant signal");
        }
    }

    #[test]
    fn test_wavelet_basis_types() {
        let data: Vec<f64> = (0..32).map(|i| i as f64).collect();
        let haar = dwt_features(&data, WaveletBasis::Haar, 3).unwrap();
        let db4 = dwt_features(&data, WaveletBasis::Db4, 3).unwrap();
        // Both should produce valid results
        assert!(haar.levels > 0);
        assert!(db4.levels > 0);
    }
}
