use crate::data::ResearchFrame;
use crate::error::{ResearchError, ResearchResult};
use finkit::features::IncrementalPCA;
use finkit::math::information::pairwise_pearson;
use finkit::math::regression::ols;
use serde::{Deserialize, Serialize};

/// Pairwise Pearson correlation matrix for named factor columns.
pub fn factor_correlation_matrix(frame: &ResearchFrame, factors: &[&str]) -> ResearchResult<Vec<Vec<f64>>> {
    let columns: Vec<&[f64]> = factors.iter().map(|name| frame.column(name)).collect::<ResearchResult<_>>()?;
    let mut matrix = vec![vec![0.0; columns.len()]; columns.len()];
    for i in 0..columns.len() {
        matrix[i][i] = 1.0;
        for j in (i + 1)..columns.len() {
            let corr = pairwise_pearson(columns[i], columns[j]);
            matrix[i][j] = corr;
            matrix[j][i] = corr;
        }
    }
    Ok(matrix)
}

/// Variance inflation factors for the requested factor set.
pub fn variance_inflation_factors(frame: &ResearchFrame, factors: &[&str]) -> ResearchResult<Vec<f64>> {
    let columns: Vec<&[f64]> = factors.iter().map(|name| frame.column(name)).collect::<ResearchResult<_>>()?;
    let mut out = Vec::with_capacity(columns.len());
    for i in 0..columns.len() {
        let x: Vec<&[f64]> = columns.iter().enumerate().filter_map(|(j, col)| (j != i).then_some(*col)).collect();
        if x.is_empty() { out.push(1.0); continue; }
        let fit = ols(columns[i], &x)?;
        out.push(if fit.r_squared >= 1.0 - 1e-12 { f64::INFINITY } else { 1.0 / (1.0 - fit.r_squared) });
    }
    Ok(out)
}

/// Cross-sectional residualization against numeric exposures, independently by date.
pub fn neutralize_cross_sectional(
    frame: &ResearchFrame,
    target: &str,
    exposures: &[&str],
) -> ResearchResult<Vec<f64>> {
    if exposures.is_empty() { return Ok(frame.column(target)?.to_vec()); }
    let y = frame.column(target)?;
    let x: Vec<&[f64]> = exposures.iter().map(|name| frame.column(name)).collect::<ResearchResult<_>>()?;
    let mut out = vec![f64::NAN; y.len()];
    for segment in 0..frame.index().date_segments().len() {
        let range = frame.index().date_segments().range(segment).unwrap();
        let rows: Vec<usize> = range.collect();
        let local_y: Vec<f64> = rows.iter().map(|&row| y[row]).collect();
        let local_x_owned: Vec<Vec<f64>> = x.iter().map(|col| rows.iter().map(|&row| col[row]).collect()).collect();
        let local_x: Vec<&[f64]> = local_x_owned.iter().map(Vec::as_slice).collect();
        if let Ok(fit) = ols(&local_y, &local_x) {
            for (&row, residual) in rows.iter().zip(fit.residuals) { out[row] = residual; }
        }
    }
    Ok(out)
}

/// PCA projection using the existing incremental PCA implementation.
pub fn pca_project(frame: &ResearchFrame, factors: &[&str], components: usize) -> ResearchResult<Vec<Vec<f64>>> {
    if factors.is_empty() || components == 0 {
        return Err(ResearchError::InvalidConfig("PCA requires factors and components".to_string()));
    }
    let columns: Vec<&[f64]> = factors.iter().map(|name| frame.column(name)).collect::<ResearchResult<_>>()?;
    let rows: Vec<Vec<f64>> = (0..frame.index().len()).map(|row| columns.iter().map(|col| col[row]).collect()).collect();
    if rows.iter().any(|row| row.iter().any(|value| !value.is_finite())) {
        return Err(ResearchError::InvalidConfig("PCA input must be finite; clean the research frame first".to_string()));
    }
    let row_refs: Vec<&[f64]> = rows.iter().map(Vec::as_slice).collect();
    let mut pca = IncrementalPCA::new(components);
    pca.partial_fit(&row_refs, factors.len());
    Ok(pca.transform(&row_refs, factors.len()))
}

/// Fama-MacBeth cross-sectional factor-premium summary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FamaMacBethResult {
    pub mean_premia: Vec<f64>,
    pub standard_errors: Vec<f64>,
    pub t_stats: Vec<f64>,
    pub valid_dates: usize,
}

pub fn fama_macbeth(
    frame: &ResearchFrame,
    return_column: &str,
    factors: &[&str],
) -> ResearchResult<FamaMacBethResult> {
    if factors.is_empty() { return Err(ResearchError::InvalidConfig("Fama-MacBeth requires at least one factor".to_string())); }
    let y = frame.column(return_column)?;
    let x: Vec<&[f64]> = factors.iter().map(|name| frame.column(name)).collect::<ResearchResult<_>>()?;
    let mut premia: Vec<Vec<f64>> = Vec::new();
    for segment in 0..frame.index().date_segments().len() {
        let range = frame.index().date_segments().range(segment).unwrap();
        let rows: Vec<usize> = range.collect();
        let local_y: Vec<f64> = rows.iter().map(|&row| y[row]).collect();
        let local_x_owned: Vec<Vec<f64>> = x.iter().map(|col| rows.iter().map(|&row| col[row]).collect()).collect();
        let local_x: Vec<&[f64]> = local_x_owned.iter().map(Vec::as_slice).collect();
        if let Ok(fit) = ols(&local_y, &local_x) {
            if fit.coefficients.len() == factors.len() { premia.push(fit.coefficients); }
        }
    }
    if premia.is_empty() {
        return Ok(FamaMacBethResult { mean_premia: vec![f64::NAN; factors.len()], standard_errors: vec![f64::NAN; factors.len()], t_stats: vec![f64::NAN; factors.len()], valid_dates: 0 });
    }
    let n = premia.len() as f64;
    let mut means = vec![0.0; factors.len()];
    for row in &premia { for (j, value) in row.iter().enumerate() { means[j] += value; } }
    for mean in &mut means { *mean /= n; }
    let mut ses = vec![0.0; factors.len()];
    for j in 0..factors.len() {
        let variance = if premia.len() > 1 {
            premia.iter().map(|row| (row[j] - means[j]).powi(2)).sum::<f64>() / (n - 1.0)
        } else { 0.0 };
        ses[j] = (variance / n).sqrt();
    }
    let t_stats = means.iter().zip(&ses).map(|(&mean, &se)| if se > 1e-15 { mean / se } else { f64::NAN }).collect();
    Ok(FamaMacBethResult { mean_premia: means, standard_errors: ses, t_stats, valid_dates: premia.len() })
}
