//! Feature matrix export: CSV, JSON Lines, Arrow IPC.

use super::FeatureMatrix;
use std::io::Write;
use std::path::Path;

/// Export FeatureMatrix to CSV format.
pub fn to_csv(matrix: &FeatureMatrix, path: impl AsRef<Path>) -> std::io::Result<()> {
    let mut file = std::fs::File::create(path)?;

    // Header
    let names = matrix.column_names();
    writeln!(file, "{}", names.join(","))?;

    // Rows
    for row in 0..matrix.rows() {
        let values: Vec<String> = (0..matrix.cols())
            .map(|col| {
                let v = matrix.get(row, col);
                if v.is_nan() { String::new() } else { format!("{}", v) }
            })
            .collect();
        writeln!(file, "{}", values.join(","))?;
    }
    Ok(())
}

/// Export FeatureMatrix to JSON Lines format.
pub fn to_json_lines(matrix: &FeatureMatrix, path: impl AsRef<Path>) -> std::io::Result<()> {
    let mut file = std::fs::File::create(path)?;
    let names = matrix.column_names();

    for row in 0..matrix.rows() {
        let mut entries = Vec::new();
        for (col, name) in names.iter().enumerate() {
            let v = matrix.get(row, col);
            if v.is_nan() {
                entries.push(format!("\"{}\":null", name));
            } else {
                entries.push(format!("\"{}\":{}", name, v));
            }
        }
        writeln!(file, "{{{}}}", entries.join(","))?;
    }
    Ok(())
}

/// Export FeatureMatrix to Arrow IPC format (simplified column-based binary).
///
/// Note: This is a simplified implementation. For production use with
/// full Arrow compatibility, enable the `finkit-polars` feature.
pub fn to_arrow_ipc(matrix: &FeatureMatrix, path: impl AsRef<Path>) -> std::io::Result<()> {
    let mut file = std::fs::File::create(path)?;

    // Simple binary format: header + column data
    let num_cols = matrix.cols() as u32;
    let num_rows = matrix.rows() as u32;
    file.write_all(&num_cols.to_le_bytes())?;
    file.write_all(&num_rows.to_le_bytes())?;

    // Column names (length-prefixed strings)
    for name in matrix.column_names() {
        let name_bytes = name.as_bytes();
        let name_len = name_bytes.len() as u32;
        file.write_all(&name_len.to_le_bytes())?;
        file.write_all(name_bytes)?;
    }

    // Column data (raw f64 arrays)
    for col in 0..matrix.cols() {
        let data = matrix.column(col);
        for &v in data {
            file.write_all(&v.to_le_bytes())?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::Feature;
    use std::io::Read;

    #[test]
    fn test_to_csv() {
        let mut m = FeatureMatrix::new();
        m.add_column(Feature::new("a", "cat", 0), vec![1.0, 2.0, 3.0]);
        m.add_column(Feature::new("b", "cat", 0), vec![4.0, f64::NAN, 6.0]);

        let path = std::env::temp_dir().join("test_fta_export.csv");
        to_csv(&m, &path).unwrap();

        let mut content = String::new();
        std::fs::File::open(&path).unwrap().read_to_string(&mut content).unwrap();
        assert!(content.contains("a,b"));
        assert!(content.contains("1,4"));
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn test_to_json_lines() {
        let mut m = FeatureMatrix::new();
        m.add_column(Feature::new("x", "cat", 0), vec![1.5, 2.5]);
        m.add_column(Feature::new("y", "cat", 0), vec![3.0, f64::NAN]);

        let path = std::env::temp_dir().join("test_fta_export.jsonl");
        to_json_lines(&m, &path).unwrap();

        let mut content = String::new();
        std::fs::File::open(&path).unwrap().read_to_string(&mut content).unwrap();
        assert!(content.contains("\"x\":1.5"));
        assert!(content.contains("\"y\":null"));
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn test_to_arrow_ipc() {
        let mut m = FeatureMatrix::new();
        m.add_column(Feature::new("val", "cat", 0), vec![1.0, 2.0, 3.0]);

        let path = std::env::temp_dir().join("test_fta_export.arrow");
        to_arrow_ipc(&m, &path).unwrap();

        let metadata = std::fs::metadata(&path).unwrap();
        assert!(metadata.len() > 0);
        std::fs::remove_file(path).ok();
    }
}
