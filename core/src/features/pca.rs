//! Incremental Principal Component Analysis for online dimensionality reduction.

/// Online PCA using incremental mean and covariance updates.
pub struct IncrementalPCA {
    n_components: usize,
    n_features: usize,
    n_samples_seen: usize,
    mean: Vec<f64>,
    cov: Vec<f64>,
    explained_variance_ratio_: Vec<f64>,
}

impl IncrementalPCA {
    /// Create a new incremental PCA reducer targeting `n_components` dimensions.
    pub fn new(n_components: usize) -> Self {
        Self {
            n_components,
            n_features: 0,
            n_samples_seen: 0,
            mean: Vec::new(),
            cov: Vec::new(),
            explained_variance_ratio_: Vec::new(),
        }
    }

    /// Update running statistics with a batch of row-major samples.
    pub fn partial_fit(&mut self, data: &[&[f64]], n_features: usize) {
        if data.is_empty() || n_features == 0 {
            return;
        }

        if self.n_features == 0 {
            self.n_features = n_features;
            self.mean = vec![0.0; n_features];
            self.cov = vec![0.0; n_features * n_features];
        } else if self.n_features != n_features {
            return;
        }

        let (batch_mean, batch_cov, n_batch) = batch_mean_cov(data, n_features);
        if n_batch == 0 {
            return;
        }

        let (total, merged_mean, merged_cov) = merge_statistics(
            self.n_samples_seen,
            &self.mean,
            &self.cov,
            n_batch,
            &batch_mean,
            &batch_cov,
        );

        self.n_samples_seen = total;
        self.mean = merged_mean;
        self.cov = merged_cov;
        self.update_explained_variance();
    }

    /// Project samples onto the top principal components (requires prior `partial_fit`).
    pub fn transform(&self, data: &[&[f64]], n_features: usize) -> Vec<Vec<f64>> {
        if !self.is_fitted() || data.is_empty() || n_features != self.n_features {
            return Vec::new();
        }

        let (_eigenvalues, eigenvectors) = symmetric_eigen(&self.cov, self.n_features);
        let k = self.n_components.min(self.n_features);

        data.iter()
            .filter(|row| row.len() == n_features)
            .map(|row| {
                let mut projected = vec![0.0; k];
                for c in 0..k {
                    let mut dot = 0.0;
                    for j in 0..n_features {
                        dot += (row[j] - self.mean[j]) * eigenvectors[j][c];
                    }
                    projected[c] = dot;
                }
                projected
            })
            .collect()
    }

    /// Explained variance ratio per retained component (updated on each `partial_fit`).
    pub fn explained_variance_ratio(&self) -> &[f64] {
        &self.explained_variance_ratio_
    }

    /// Whether enough samples have been seen to produce meaningful projections.
    pub fn is_fitted(&self) -> bool {
        self.n_samples_seen >= 2 && self.n_features > 0
    }

    fn update_explained_variance(&mut self) {
        if self.n_samples_seen < 2 {
            self.explained_variance_ratio_.clear();
            return;
        }

        let (eigenvalues, _) = symmetric_eigen(&self.cov, self.n_features);
        let total: f64 = eigenvalues.iter().sum();
        let k = self.n_components.min(self.n_features);

        if total <= 1e-15 {
            self.explained_variance_ratio_ = vec![0.0; k];
            return;
        }

        self.explained_variance_ratio_ = eigenvalues
            .iter()
            .take(k)
            .map(|&ev| (ev / total).max(0.0))
            .collect();
    }
}

fn batch_mean_cov(data: &[&[f64]], n_features: usize) -> (Vec<f64>, Vec<f64>, usize) {
    let mut mean = vec![0.0; n_features];

    for row in data {
        if row.len() != n_features {
            continue;
        }
        for (j, &v) in row.iter().enumerate() {
            mean[j] += v;
        }
    }

    let valid: Vec<&[f64]> = data
        .iter()
        .copied()
        .filter(|row| row.len() == n_features)
        .collect();
    let n_valid = valid.len();
    if n_valid == 0 {
        return (mean, vec![0.0; n_features * n_features], 0);
    }

    for v in &mut mean {
        *v /= n_valid as f64;
    }

    let mut cov = vec![0.0; n_features * n_features];
    if n_valid > 1 {
        for row in &valid {
            for i in 0..n_features {
                let di = row[i] - mean[i];
                for j in i..n_features {
                    let dj = row[j] - mean[j];
                    cov[i * n_features + j] += di * dj;
                    if i != j {
                        cov[j * n_features + i] += di * dj;
                    }
                }
            }
        }
        let denom = n_valid as f64;
        for val in &mut cov {
            *val /= denom;
        }
    }

    (mean, cov, n_valid)
}

fn merge_statistics(
    n_old: usize,
    mean_old: &[f64],
    cov_old: &[f64],
    n_new: usize,
    mean_new: &[f64],
    cov_new: &[f64],
) -> (usize, Vec<f64>, Vec<f64>) {
    let d = mean_old.len();
    let n_total = n_old + n_new;
    if n_total == 0 {
        return (0, vec![0.0; d], vec![0.0; d * d]);
    }

    let mut mean = vec![0.0; d];
    for i in 0..d {
        mean[i] = (n_old as f64 * mean_old[i] + n_new as f64 * mean_new[i]) / n_total as f64;
    }

    let mut cov = vec![0.0; d * d];
    for i in 0..d {
        for j in 0..d {
            let old_term = if n_old > 0 {
                let di = mean_old[i] - mean[i];
                let dj = mean_old[j] - mean[j];
                n_old as f64 * (cov_old[i * d + j] + di * dj)
            } else {
                0.0
            };
            let new_term = if n_new > 0 {
                let di = mean_new[i] - mean[i];
                let dj = mean_new[j] - mean[j];
                n_new as f64 * (cov_new[i * d + j] + di * dj)
            } else {
                0.0
            };
            cov[i * d + j] = (old_term + new_term) / n_total as f64;
        }
    }

    (n_total, mean, cov)
}

/// Jacobi eigenvalue decomposition for symmetric `n`×`n` matrix.
/// Returns eigenvalues (descending) and eigenvector matrix `V` where column `c` is eigenvector `c`.
#[allow(clippy::needless_range_loop)]
fn symmetric_eigen(cov: &[f64], n: usize) -> (Vec<f64>, Vec<Vec<f64>>) {
    let mut a = vec![vec![0.0; n]; n];
    let mut v = vec![vec![0.0; n]; n];

    for i in 0..n {
        for j in 0..n {
            a[i][j] = cov[i * n + j];
        }
        v[i][i] = 1.0;
    }

    const MAX_SWEEPS: usize = 50;
    const TOL: f64 = 1e-12;

    for _ in 0..MAX_SWEEPS {
        let mut max_off = 0.0f64;
        let mut p = 0usize;
        let mut q = 1usize;

        for i in 0..n {
            for j in (i + 1)..n {
                let off = a[i][j].abs();
                if off > max_off {
                    max_off = off;
                    p = i;
                    q = j;
                }
            }
        }

        if max_off < TOL {
            break;
        }

        let app = a[p][p];
        let aqq = a[q][q];
        let apq = a[p][q];

        let tau = (aqq - app) / (2.0 * apq);
        let t = if tau >= 0.0 {
            1.0 / (tau + (1.0 + tau * tau).sqrt())
        } else {
            -1.0 / (-tau + (1.0 + tau * tau).sqrt())
        };
        let c = 1.0 / (1.0 + t * t).sqrt();
        let s = t * c;

        let a_pp = c * c * app - 2.0 * s * c * apq + s * s * aqq;
        let a_qq = s * s * app + 2.0 * s * c * apq + c * c * aqq;
        a[p][p] = a_pp;
        a[q][q] = a_qq;
        a[p][q] = 0.0;
        a[q][p] = 0.0;

        for k in 0..n {
            if k != p && k != q {
                let akp = a[k][p];
                let akq = a[k][q];
                a[k][p] = c * akp - s * akq;
                a[p][k] = a[k][p];
                a[k][q] = s * akp + c * akq;
                a[q][k] = a[k][q];
            }
        }

        for k in 0..n {
            let vkp = v[k][p];
            let vkq = v[k][q];
            v[k][p] = c * vkp - s * vkq;
            v[k][q] = s * vkp + c * vkq;
        }
    }

    let mut eigenvalues: Vec<f64> = (0..n).map(|i| a[i][i]).collect();
    eigenvalues.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));

    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&i, &j| {
        a[j][j]
            .partial_cmp(&a[i][i])
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut eigenvectors = vec![vec![0.0; n]; n];
    for (col, &idx) in order.iter().enumerate() {
        for row in 0..n {
            eigenvectors[row][col] = v[row][idx];
        }
    }

    (eigenvalues, eigenvectors)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn structured_samples(n: usize, dim: usize) -> Vec<Vec<f64>> {
        (0..n)
            .map(|i| {
                let t = i as f64 * 0.1;
                let mut row = vec![0.0; dim];
                row[0] = t;
                row[1] = 2.0 * t;
                row[2] = -0.5 * t;
                for (j, val) in row.iter_mut().enumerate().skip(3) {
                    *val = (i as f64 * 0.37 + j as f64).sin() * 0.01;
                }
                row
            })
            .collect()
    }

    #[test]
    fn test_incremental_pca_basic() {
        let raw = structured_samples(200, 10);
        let refs: Vec<&[f64]> = raw.iter().map(|r| r.as_slice()).collect();

        let mut pca = IncrementalPCA::new(3);
        pca.partial_fit(&refs, 10);

        let transformed = pca.transform(&refs, 10);
        assert_eq!(transformed.len(), 200);
        assert_eq!(transformed[0].len(), 3);

        let evr: f64 = pca.explained_variance_ratio().iter().sum();
        assert!(
            evr > 0.8,
            "explained variance ratio sum should exceed 0.8, got {evr}"
        );
        assert!(pca.is_fitted());
    }

    #[test]
    fn test_pca_transform_dimensions() {
        let raw = structured_samples(50, 8);
        let refs: Vec<&[f64]> = raw.iter().map(|r| r.as_slice()).collect();

        let mut pca = IncrementalPCA::new(4);
        pca.partial_fit(&refs, 8);

        let out = pca.transform(&refs, 8);
        assert_eq!(out.len(), 50);
        for row in &out {
            assert_eq!(row.len(), 4);
        }

        let partial: Vec<&[f64]> = refs[..10].to_vec();
        let out_partial = pca.transform(&partial, 8);
        assert_eq!(out_partial.len(), 10);
        for row in &out_partial {
            assert_eq!(row.len(), 4);
        }
    }
}
