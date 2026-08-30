//! Feature matrix: 2D storage for computed features.

use super::Feature;

/// A 2D feature matrix where rows represent time points and columns represent features.
///
/// This is the primary output of feature generation, designed for efficient
/// column-oriented access and zero-copy export to NumPy/Arrow/Polars.
#[derive(Debug, Clone)]
pub struct FeatureMatrix {
    data: Vec<Vec<f64>>,
    features: Vec<Feature>,
    num_rows: usize,
}

impl FeatureMatrix {
    /// Create a new empty FeatureMatrix.
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            features: Vec::new(),
            num_rows: 0,
        }
    }

    /// Create a FeatureMatrix with pre-allocated capacity.
    pub fn with_capacity(num_rows: usize, num_cols: usize) -> Self {
        Self {
            data: Vec::with_capacity(num_cols),
            features: Vec::with_capacity(num_cols),
            num_rows,
        }
    }

    /// Add a column of feature data.
    pub fn add_column(&mut self, feature: Feature, values: Vec<f64>) {
        if self.data.is_empty() {
            self.num_rows = values.len();
        }
        debug_assert!(values.len() == self.num_rows || self.num_rows == 0);
        self.num_rows = values.len();
        self.data.push(values);
        self.features.push(feature);
    }

    /// Number of time points (rows).
    pub fn rows(&self) -> usize {
        self.num_rows
    }

    /// Number of features (columns).
    pub fn cols(&self) -> usize {
        self.data.len()
    }

    /// Get column names.
    pub fn column_names(&self) -> Vec<&str> {
        self.features.iter().map(|f| f.name.as_str()).collect()
    }

    /// Get feature metadata.
    pub fn features(&self) -> &[Feature] {
        &self.features
    }

    /// Get a single value at (row, col).
    pub fn get(&self, row: usize, col: usize) -> f64 {
        self.data[col][row]
    }

    /// Get an entire column by index.
    pub fn column(&self, col: usize) -> &[f64] {
        &self.data[col]
    }

    /// Get a column by name.
    pub fn column_by_name(&self, name: &str) -> Option<&[f64]> {
        self.features
            .iter()
            .position(|f| f.name == name)
            .map(|idx| self.data[idx].as_slice())
    }

    /// Get all column data as slice of slices (column-major).
    pub fn columns(&self) -> &[Vec<f64>] {
        &self.data
    }

    /// Extract a row as a vector.
    pub fn row(&self, idx: usize) -> Vec<f64> {
        self.data.iter().map(|col| col[idx]).collect()
    }

    /// Select specific columns by name.
    pub fn select(&self, names: &[&str]) -> FeatureMatrix {
        let mut result = FeatureMatrix::new();
        for name in names {
            if let Some(pos) = self.features.iter().position(|f| f.name == *name) {
                result.add_column(self.features[pos].clone(), self.data[pos].clone());
            }
        }
        result
    }

    /// Filter rows by index range.
    pub fn slice_rows(&self, start: usize, end: usize) -> FeatureMatrix {
        let mut result = FeatureMatrix::with_capacity(end - start, self.cols());
        for (i, col) in self.data.iter().enumerate() {
            result.add_column(
                self.features[i].clone(),
                col[start..end].to_vec(),
            );
        }
        result
    }

    /// Merge another FeatureMatrix (must have same number of rows).
    pub fn merge(&mut self, other: FeatureMatrix) {
        debug_assert!(other.rows() == self.num_rows || self.num_rows == 0);
        if self.num_rows == 0 {
            self.num_rows = other.num_rows;
        }
        for (i, col) in other.data.into_iter().enumerate() {
            self.data.push(col);
            self.features.push(other.features[i].clone());
        }
    }

    /// Remove columns with all NaN values.
    pub fn drop_all_nan_columns(&mut self) {
        let mut keep = Vec::new();
        for (i, col) in self.data.iter().enumerate() {
            if !col.iter().all(|v| v.is_nan()) {
                keep.push(i);
            }
        }
        self.data = keep.iter().map(|&i| self.data[i].clone()).collect();
        self.features = keep.iter().map(|&i| self.features[i].clone()).collect();
    }
}

impl Default for FeatureMatrix {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_matrix_basic() {
        let mut m = FeatureMatrix::new();
        m.add_column(
            Feature::new("sma_5", "overlap", 5),
            vec![1.0, 2.0, 3.0, 4.0, 5.0],
        );
        m.add_column(
            Feature::new("rsi_14", "momentum", 14),
            vec![50.0, 55.0, 60.0, 45.0, 70.0],
        );

        assert_eq!(m.rows(), 5);
        assert_eq!(m.cols(), 2);
        assert_eq!(m.column_names(), vec!["sma_5", "rsi_14"]);
        assert_eq!(m.get(0, 0), 1.0);
        assert_eq!(m.get(2, 1), 60.0);
    }

    #[test]
    fn test_feature_matrix_column_by_name() {
        let mut m = FeatureMatrix::new();
        m.add_column(Feature::new("a", "cat", 0), vec![1.0, 2.0]);
        m.add_column(Feature::new("b", "cat", 0), vec![3.0, 4.0]);

        assert_eq!(m.column_by_name("b"), Some(&[3.0, 4.0][..]));
        assert_eq!(m.column_by_name("c"), None);
    }

    #[test]
    fn test_feature_matrix_select() {
        let mut m = FeatureMatrix::new();
        m.add_column(Feature::new("a", "cat", 0), vec![1.0, 2.0]);
        m.add_column(Feature::new("b", "cat", 0), vec![3.0, 4.0]);
        m.add_column(Feature::new("c", "cat", 0), vec![5.0, 6.0]);

        let selected = m.select(&["a", "c"]);
        assert_eq!(selected.cols(), 2);
        assert_eq!(selected.column_names(), vec!["a", "c"]);
    }

    #[test]
    fn test_feature_matrix_row() {
        let mut m = FeatureMatrix::new();
        m.add_column(Feature::new("x", "cat", 0), vec![1.0, 2.0, 3.0]);
        m.add_column(Feature::new("y", "cat", 0), vec![4.0, 5.0, 6.0]);

        assert_eq!(m.row(1), vec![2.0, 5.0]);
    }

    #[test]
    fn test_feature_matrix_merge() {
        let mut m1 = FeatureMatrix::new();
        m1.add_column(Feature::new("a", "cat", 0), vec![1.0, 2.0]);

        let mut m2 = FeatureMatrix::new();
        m2.add_column(Feature::new("b", "cat", 0), vec![3.0, 4.0]);

        m1.merge(m2);
        assert_eq!(m1.cols(), 2);
        assert_eq!(m1.column_names(), vec!["a", "b"]);
    }

    #[test]
    fn test_feature_matrix_slice_rows() {
        let mut m = FeatureMatrix::new();
        m.add_column(Feature::new("x", "cat", 0), vec![1.0, 2.0, 3.0, 4.0, 5.0]);
        let sliced = m.slice_rows(1, 4);
        assert_eq!(sliced.rows(), 3);
        assert_eq!(sliced.column(0), &[2.0, 3.0, 4.0]);
    }
}
