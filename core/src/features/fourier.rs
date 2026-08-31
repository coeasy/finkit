//! FFT-based features for time series analysis.

use crate::error::{Result, TaError};
use ndarray::Array1;
use std::f64::consts::PI;

/// FFT feature extraction result.
#[derive(Debug, Clone)]
pub struct FftFeatures {
    /// Dominant frequency (index of max power, excluding DC)
    pub dominant_freq: usize,
    /// Power spectrum (magnitude squared of FFT, first half)
    pub power_spectrum: Vec<f64>,
    /// Spectral energy distribution (normalized power in frequency bands)
    pub spectral_energy: Vec<f64>,
    /// Total spectral energy
    pub total_energy: f64,
    /// Spectral centroid (weighted average frequency)
    pub spectral_centroid: f64,
}

/// Extract FFT features from a time series.
///
/// Computes the discrete Fourier transform and extracts frequency-domain features.
///
/// # Arguments
/// * `data` - Input time series (length will be zero-padded to next power of 2)
/// * `num_bands` - Number of frequency bands for energy distribution (0 = auto)
///
/// # Returns
/// FftFeatures with dominant frequency, power spectrum, spectral energy, etc.
pub fn fft_features(data: &[f64], num_bands: usize) -> Result<FftFeatures> {
    let n = data.len();
    if n < 4 {
        return Err(TaError::InvalidParameter {
            name: "data".to_string(),
            constraint: "length must be >= 4".to_string(),
        });
    }

    // Zero-pad to next power of 2
    let fft_size = n.next_power_of_two();
    let mut real = vec![0.0; fft_size];
    let mut imag = vec![0.0; fft_size];
    real[..n].copy_from_slice(data);

    // In-place FFT
    fft_in_place(&mut real, &mut imag);

    // Power spectrum (first half only, excluding Nyquist)
    let half = fft_size / 2;
    let mut power = Vec::with_capacity(half);
    for i in 0..half {
        power.push(real[i] * real[i] + imag[i] * imag[i]);
    }

    // Dominant frequency (skip DC at index 0)
    let dominant_freq = if half > 1 {
        power[1..]
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i + 1)
            .unwrap_or(1)
    } else {
        1
    };

    let total_energy: f64 = power[1..].iter().sum();

    // Spectral centroid
    let spectral_centroid = if total_energy > 1e-15 {
        power[1..]
            .iter()
            .enumerate()
            .map(|(i, &p)| (i + 1) as f64 * p)
            .sum::<f64>()
            / total_energy
    } else {
        0.0
    };

    // Spectral energy in bands
    let bands = if num_bands == 0 { 5 } else { num_bands };
    let band_size = (half - 1).max(1) / bands;
    let mut spectral_energy = Vec::with_capacity(bands);
    for b in 0..bands {
        let start = 1 + b * band_size;
        let end = if b == bands - 1 {
            half
        } else {
            1 + (b + 1) * band_size
        };
        let band_energy: f64 = power[start..end].iter().sum();
        let ratio = if total_energy > 1e-15 {
            band_energy / total_energy
        } else {
            0.0
        };
        spectral_energy.push(ratio);
    }

    Ok(FftFeatures {
        dominant_freq,
        power_spectrum: power,
        spectral_energy,
        total_energy,
        spectral_centroid,
    })
}

/// Rolling FFT dominant frequency detection.
///
/// For each window, computes FFT and returns the dominant frequency index.
///
/// # Arguments
/// * `data` - Time series
/// * `window` - Rolling window size (>= 4)
///
/// # Returns
/// Array of dominant frequency indices (NaN during warm-up)
pub fn rolling_fft(data: &[f64], window: usize) -> Result<Array1<f64>> {
    if window < 4 {
        return Err(TaError::InvalidParameter {
            name: "window".to_string(),
            constraint: "must be >= 4".to_string(),
        });
    }
    let n = data.len();
    if n < window {
        return Err(TaError::InvalidParameter {
            name: "data".to_string(),
            constraint: "length must be >= window".to_string(),
        });
    }

    let mut output = Array1::from_elem(n, f64::NAN);
    for i in (window - 1)..n {
        let start = i + 1 - window;
        if let Ok(features) = fft_features(&data[start..=i], 0) {
            output[i] = features.dominant_freq as f64;
        }
    }

    Ok(output)
}

/// In-place Cooley-Tukey radix-2 FFT.
fn fft_in_place(real: &mut [f64], imag: &mut [f64]) {
    let n = real.len();
    assert!(n.is_power_of_two());

    // Bit-reversal permutation
    let mut j = 0;
    for i in 0..n {
        if i < j {
            real.swap(i, j);
            imag.swap(i, j);
        }
        let mut m = n >> 1;
        while m >= 1 && j >= m {
            j -= m;
            m >>= 1;
        }
        j += m;
    }

    // Butterfly operations
    let mut len = 2;
    while len <= n {
        let half = len / 2;
        let angle = -2.0 * PI / len as f64;

        for start in (0..n).step_by(len) {
            for k in 0..half {
                let w_angle = angle * k as f64;
                let wr = w_angle.cos();
                let wi = w_angle.sin();

                let even = start + k;
                let odd = start + k + half;

                let tr = wr * real[odd] - wi * imag[odd];
                let ti = wr * imag[odd] + wi * real[odd];

                real[odd] = real[even] - tr;
                imag[odd] = imag[even] - ti;
                real[even] += tr;
                imag[even] += ti;
            }
        }
        len <<= 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fft_features_sine() {
        // Single frequency signal
        let n = 64;
        let freq = 4; // cycles in the window
        let data: Vec<f64> = (0..n)
            .map(|i| (2.0 * PI * freq as f64 * i as f64 / n as f64).sin())
            .collect();
        let result = fft_features(&data, 5).unwrap();
        assert_eq!(result.dominant_freq, freq);
        assert!(result.total_energy > 0.0);
        assert_eq!(result.spectral_energy.len(), 5);
    }

    #[test]
    fn test_fft_features_dc() {
        // Constant signal: all power at DC, dominant should still be >= 1
        let data = vec![5.0; 32];
        let result = fft_features(&data, 3).unwrap();
        // Dominant freq excludes DC, so all power[1..] is ~0
        assert!(result.total_energy < 1e-10);
    }

    #[test]
    fn test_fft_features_invalid() {
        assert!(fft_features(&[1.0, 2.0, 3.0], 0).is_err());
    }

    #[test]
    fn test_rolling_fft_basic() {
        let data: Vec<f64> = (0..50)
            .map(|i| (2.0 * PI * 3.0 * i as f64 / 32.0).sin())
            .collect();
        let result = rolling_fft(&data, 32).unwrap();
        assert_eq!(result.len(), 50);
        assert!(result[30].is_nan());
        assert!(result[31].is_finite());
    }

    #[test]
    fn test_rolling_fft_invalid() {
        let data = vec![1.0; 10];
        assert!(rolling_fft(&data, 3).is_err());
        assert!(rolling_fft(&data, 20).is_err());
    }

    #[test]
    fn test_spectral_centroid() {
        let n = 64;
        let data: Vec<f64> = (0..n)
            .map(|i| (2.0 * PI * 8.0 * i as f64 / n as f64).sin())
            .collect();
        let result = fft_features(&data, 5).unwrap();
        // Centroid should be near the dominant frequency
        assert!(
            (result.spectral_centroid - 8.0).abs() < 2.0,
            "Centroid: {}",
            result.spectral_centroid
        );
    }
}
