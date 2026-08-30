//! CSV I/O helpers for the finkit CLI.
//!
//! Provides:
//! - [`OhlcvData`] — in-memory OHLCV container parsed from CSV.
//! - [`read_ohlcv_csv`] — read an OHLCV CSV file from disk.
//! - [`read_ohlcv_stdin`] — read an OHLCV CSV stream from stdin (pipe support).
//! - [`read_ohlcv_input`] — read from `Option<Path>`: file or stdin.
//!
//! Header columns are matched case-insensitively by the standard names
//! `open`, `high`, `low`, `close`, and `volume` (or `vol`). Missing
//! optional columns are filled with `f64::NAN` (volume defaults to `0.0`).

use std::fs::File;
use std::io::{self, BufRead, Read};
use std::path::Path;

/// In-memory OHLCV dataset.
#[derive(Debug, Default, Clone)]
pub struct OhlcvData {
    pub open: Vec<f64>,
    pub high: Vec<f64>,
    pub low: Vec<f64>,
    pub close: Vec<f64>,
    pub volume: Vec<f64>,
}

/// Read OHLCV data from a CSV file path.
pub fn read_ohlcv_csv<P: AsRef<Path>>(path: P) -> io::Result<OhlcvData> {
    let file = File::open(path)?;
    parse_ohlcv_reader(file)
}

/// Read OHLCV data from stdin (used for shell pipe support).
pub fn read_ohlcv_stdin() -> io::Result<OhlcvData> {
    let stdin = io::stdin();
    let mut buf = Vec::new();
    stdin.lock().read_to_end(&mut buf)?;
    parse_ohlcv_reader(&buf[..])
}

/// Read OHLCV data from `Some(path)` or stdin if `None`.
pub fn read_ohlcv_input<P: AsRef<Path>>(path: Option<P>) -> io::Result<OhlcvData> {
    match path {
        Some(p) => read_ohlcv_csv(p),
        None => read_ohlcv_stdin(),
    }
}

fn parse_ohlcv_reader<R: Read>(reader: R) -> io::Result<OhlcvData> {
    let mut rdr = csv::ReaderBuilder::new()
        .flexible(true)
        .has_headers(true)
        .from_reader(reader);

    let headers = rdr.headers()?.clone();
    let header_lower: Vec<String> = headers.iter().map(|h| h.to_lowercase()).collect();

    let find_col = |names: &[&str]| -> Option<usize> {
        for name in names {
            if let Some(idx) = header_lower.iter().position(|h| h.contains(name)) {
                return Some(idx);
            }
        }
        None
    };

    let open_idx = find_col(&["open"]);
    let high_idx = find_col(&["high"]);
    let low_idx = find_col(&["low"]);
    let close_idx = find_col(&["close"]);
    let volume_idx = find_col(&["volume", "vol"]);

    let close_idx = close_idx.ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "CSV must have a 'close' column")
    })?;

    let mut data = OhlcvData::default();

    for result in rdr.records() {
        let record = result?;
        let parse = |idx: usize| -> f64 {
            record
                .get(idx)
                .and_then(|v| v.trim().parse().ok())
                .unwrap_or(f64::NAN)
        };

        data.close.push(parse(close_idx));
        data.open.push(open_idx.map_or(f64::NAN, parse));
        data.high.push(high_idx.map_or(f64::NAN, parse));
        data.low.push(low_idx.map_or(f64::NAN, parse));
        data.volume.push(volume_idx.map_or(0.0, parse));
    }

    Ok(data)
}

/// Read a single column (close-style) CSV from a file path.
pub fn read_close_csv<P: AsRef<Path>>(path: P) -> io::Result<Vec<f64>> {
    let content = std::fs::read_to_string(path)?;
    parse_close_lines(&content)
}

/// Read a single column (close-style) CSV from stdin (pipe support).
pub fn read_close_stdin() -> io::Result<Vec<f64>> {
    let stdin = io::stdin();
    let mut buf = String::new();
    for line in stdin.lock().lines() {
        let line = line?;
        buf.push_str(&line);
        buf.push('\n');
    }
    parse_close_lines(&buf)
}

/// Read a single column (close-style) CSV from `Some(path)` or stdin if `None`.
pub fn read_close_input<P: AsRef<Path>>(path: Option<P>) -> io::Result<Vec<f64>> {
    match path {
        Some(p) => read_close_csv(p),
        None => read_close_stdin(),
    }
}

fn parse_close_lines(content: &str) -> io::Result<Vec<f64>> {
    let values: Result<Vec<f64>, _> = content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.trim().parse::<f64>())
        .collect();
    values.map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_parse_close_lines() {
        let csv = "1.0\n2.5\n3.7\n";
        let result = parse_close_lines(csv).unwrap();
        assert_eq!(result, vec![1.0, 2.5, 3.7]);
    }

    #[test]
    fn test_parse_close_lines_skip_empty() {
        let csv = "1.0\n\n2.5\n  \n3.7\n";
        let result = parse_close_lines(csv).unwrap();
        assert_eq!(result, vec![1.0, 2.5, 3.7]);
    }

    #[test]
    fn test_parse_close_lines_invalid() {
        let csv = "1.0\nnot_a_number\n3.7\n";
        assert!(parse_close_lines(csv).is_err());
    }

    #[test]
    fn test_read_close_csv() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        write!(tmp, "1.0\n2.0\n3.0\n").unwrap();
        let result = read_close_csv(tmp.path()).unwrap();
        assert_eq!(result, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_parse_ohlcv_reader() {
        let csv = "open,high,low,close,volume\n100.0,105.0,99.0,102.0,10000\n101.0,106.0,100.0,103.0,11000\n";
        let data = parse_ohlcv_reader(csv.as_bytes()).unwrap();
        assert_eq!(data.close, vec![102.0, 103.0]);
        assert_eq!(data.open, vec![100.0, 101.0]);
        assert_eq!(data.high, vec![105.0, 106.0]);
        assert_eq!(data.low, vec![99.0, 100.0]);
        assert_eq!(data.volume, vec![10000.0, 11000.0]);
    }

    #[test]
    fn test_parse_ohlcv_close_only() {
        // CSV with only close column should fill others with NaN/0.0
        let csv = "close\n10.0\n20.0\n";
        let data = parse_ohlcv_reader(csv.as_bytes()).unwrap();
        assert_eq!(data.close, vec![10.0, 20.0]);
        assert!(data.open.iter().all(|v| v.is_nan()));
        assert_eq!(data.volume, vec![0.0, 0.0]);
    }

    #[test]
    fn test_parse_ohlcv_no_close_column() {
        let csv = "open,high\n100.0,105.0\n";
        assert!(parse_ohlcv_reader(csv.as_bytes()).is_err());
    }

    #[test]
    fn test_ohlcv_data_default() {
        let data = OhlcvData::default();
        assert!(data.open.is_empty());
        assert!(data.close.is_empty());
    }

    #[test]
    fn test_read_close_stdin_mocked() {
        // read_close_stdin reads from actual stdin; we cannot mock it
        // without external crate. This is a placeholder documenting the gap.
        // In practice, users pipe data: `echo "1.0\n2.0" | finkit sma 3`
    }

    #[test]
    fn test_parse_ohlcv_volume_alias() {
        // "vol" should also be recognized as volume column
        let csv = "close,vol\n10.0,500\n20.0,600\n";
        let data = parse_ohlcv_reader(csv.as_bytes()).unwrap();
        assert_eq!(data.close, vec![10.0, 20.0]);
        assert_eq!(data.volume, vec![500.0, 600.0]);
    }
}
