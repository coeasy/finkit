use finkit::factors::panel::FactorPanel;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::Path;

#[derive(Debug)]
struct PanelCsv {
    timestamps: Vec<i64>,
    symbols: Vec<String>,
    fields: BTreeMap<String, Vec<f64>>,
}

#[derive(Debug)]
struct CsvRow {
    timestamp: i64,
    symbol: String,
    values: Vec<String>,
}

pub(super) fn run(
    input: &Path,
    library_name: Option<&str>,
    manifest_path: Option<&Path>,
    factor_name: Option<&str>,
    expression: Option<&str>,
    output: Option<&str>,
) -> Result<(), String> {
    if factor_name.is_some() == expression.is_some() {
        return Err("provide exactly one of --factor or --expr".to_string());
    }
    if manifest_path.is_some() && expression.is_some() {
        return Err("--manifest must be used with --factor, not --expr".to_string());
    }
    let data = read_panel_csv(input)?;
    let timestamp_axis = data.timestamps.as_slice();
    let symbol_axis: Vec<&str> = data.symbols.iter().map(String::as_str).collect();
    let fields: Vec<(&str, &[f64])> = data
        .fields
        .iter()
        .map(|(name, values)| (name.as_str(), values.as_slice()))
        .collect();
    let panel = FactorPanel::new(timestamp_axis, &symbol_axis, &fields)
        .map_err(|error| error.to_string())?;

    let result = if let Some(expression) = expression {
        panel.evaluate(expression)
    } else if let Some(manifest_path) = manifest_path {
        let source = std::fs::read_to_string(manifest_path).map_err(|error| {
            format!(
                "could not read factor manifest {}: {error}",
                manifest_path.display()
            )
        })?;
        let definitions: BTreeMap<String, String> = serde_json::from_str(&source)
            .map_err(|error| format!("invalid factor manifest JSON: {error}"))?;
        let owned: Vec<(String, String)> = definitions.into_iter().collect();
        let entries: Vec<(&str, &str, finkit::factors::FactorDirection)> = owned
            .iter()
            .map(|(name, expression)| {
                (
                    name.as_str(),
                    expression.as_str(),
                    finkit::factors::FactorDirection::Neutral,
                )
            })
            .collect();
        let library = finkit::factors::builtin::FactorLibrary::from_named_expressions(
            manifest_path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("user-manifest"),
            &entries,
        )
        .map_err(|error| error.to_string())?;
        library.evaluate_panel(factor_name.expect("validated factor name"), &panel)
    } else {
        let library =
            finkit::factors::builtin::factor_library(library_name.unwrap_or("worldquant101"))
                .map_err(|error| error.to_string())?;
        library.evaluate_panel(factor_name.expect("validated factor name"), &panel)
    }
    .map_err(|error| error.to_string())?;

    let mut writer = csv::Writer::from_writer(Vec::new());
    writer
        .write_record(["timestamp", "symbol", "value"])
        .map_err(|error| error.to_string())?;
    for (row, timestamp) in result.timestamps.iter().enumerate() {
        for (column, symbol) in result.symbols.iter().enumerate() {
            let value = result.values[row * result.cols() + column];
            writer
                .write_record([timestamp.to_string(), symbol.clone(), value.to_string()])
                .map_err(|error| error.to_string())?;
        }
    }
    let bytes = writer.into_inner().map_err(|error| error.to_string())?;
    let text = String::from_utf8(bytes).map_err(|error| error.to_string())?;
    if let Some(path) = output {
        std::fs::write(path, text).map_err(|error| format!("could not write {path}: {error}"))?;
    } else {
        print!("{text}");
    }
    Ok(())
}

fn read_panel_csv(path: &Path) -> Result<PanelCsv, String> {
    let mut reader = csv::ReaderBuilder::new()
        .flexible(false)
        .from_path(path)
        .map_err(|error| format!("could not read panel CSV {}: {error}", path.display()))?;
    let headers = reader.headers().map_err(|error| error.to_string())?.clone();
    let header_names: Vec<String> = headers
        .iter()
        .map(|value| value.trim().to_ascii_lowercase())
        .collect();
    let timestamp_index = header_names
        .iter()
        .position(|name| name == "timestamp")
        .ok_or_else(|| "panel CSV requires a timestamp column".to_string())?;
    let symbol_index = header_names
        .iter()
        .position(|name| name == "symbol")
        .ok_or_else(|| "panel CSV requires a symbol column".to_string())?;
    let value_indices: Vec<usize> = (0..header_names.len())
        .filter(|index| *index != timestamp_index && *index != symbol_index)
        .collect();
    if value_indices.is_empty() {
        return Err("panel CSV requires at least one numeric field".to_string());
    }

    let mut rows = Vec::new();
    let mut timestamps = BTreeSet::new();
    let mut symbols = BTreeSet::new();
    let mut keys = HashSet::new();
    for record in reader.records() {
        let record = record.map_err(|error| error.to_string())?;
        let timestamp_text = record
            .get(timestamp_index)
            .ok_or_else(|| "missing timestamp cell".to_string())?;
        let timestamp = timestamp_text.trim().parse::<i64>().map_err(|_| {
            format!("timestamp `{timestamp_text}` must be an integer Unix timestamp")
        })?;
        let symbol = record
            .get(symbol_index)
            .unwrap_or_default()
            .trim()
            .to_string();
        if symbol.is_empty() {
            return Err("panel CSV contains an empty symbol".to_string());
        }
        if !keys.insert((timestamp, symbol.clone())) {
            return Err(format!(
                "panel CSV contains duplicate timestamp/symbol row: {timestamp}/{symbol}"
            ));
        }
        timestamps.insert(timestamp);
        symbols.insert(symbol.clone());
        let values = value_indices
            .iter()
            .map(|index| record.get(*index).unwrap_or_default().trim().to_string())
            .collect();
        rows.push(CsvRow {
            timestamp,
            symbol,
            values,
        });
    }
    if rows.is_empty() {
        return Err("panel CSV contains no data rows".to_string());
    }

    let timestamps: Vec<i64> = timestamps.into_iter().collect();
    let symbols: Vec<String> = symbols.into_iter().collect();
    let timestamp_indices: BTreeMap<i64, usize> = timestamps
        .iter()
        .enumerate()
        .map(|(index, value)| (*value, index))
        .collect();
    let symbol_indices: BTreeMap<&str, usize> = symbols
        .iter()
        .enumerate()
        .map(|(index, value)| (value.as_str(), index))
        .collect();
    let cells = timestamps
        .len()
        .checked_mul(symbols.len())
        .ok_or_else(|| "panel dimensions overflow address space".to_string())?;
    let mut raw_fields: Vec<Vec<Option<String>>> = vec![vec![None; cells]; value_indices.len()];
    for row in rows {
        let row_index = timestamp_indices[&row.timestamp];
        let col_index = symbol_indices[row.symbol.as_str()];
        let cell = row_index * symbols.len() + col_index;
        for (field_index, value) in row.values.into_iter().enumerate() {
            if !value.is_empty() {
                raw_fields[field_index][cell] = Some(value);
            }
        }
    }

    let mut fields = BTreeMap::new();
    for (field_index, csv_index) in value_indices.iter().enumerate() {
        let field_name = header_names[*csv_index].to_ascii_uppercase();
        let raw = &raw_fields[field_index];
        let numeric = raw
            .iter()
            .flatten()
            .map(|value| value.parse::<f64>())
            .collect::<Result<Vec<_>, _>>();
        let values = match numeric {
            Ok(parsed) => {
                let mut cursor = parsed.into_iter();
                raw.iter()
                    .map(|cell| cell.as_ref().map_or(f64::NAN, |_| cursor.next().unwrap()))
                    .collect()
            }
            Err(_) if is_group_field(&field_name) => {
                let mut categories: Vec<String> = raw.iter().flatten().cloned().collect();
                categories.sort();
                categories.dedup();
                raw.iter()
                    .map(|cell| {
                        cell.as_ref().map_or(f64::NAN, |category| {
                            categories
                                .binary_search(category)
                                .map_or(f64::NAN, |index| index as f64 + 1.0)
                        })
                    })
                    .collect()
            }
            Err(_) => {
                return Err(format!(
                    "panel field `{field_name}` contains non-numeric values; only sector/industry group fields accept labels"
                ));
            }
        };
        fields.insert(field_name, values);
    }

    Ok(PanelCsv {
        timestamps,
        symbols,
        fields,
    })
}

fn is_group_field(name: &str) -> bool {
    matches!(
        name.rsplit('.').next().unwrap_or(name),
        "SECTOR" | "INDUSTRY" | "SUBINDUSTRY"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_csv_aligns_unsorted_rows_and_encodes_group_labels() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("panel.csv");
        std::fs::write(
            &path,
            "timestamp,symbol,close,sector\n2,B,20,tech\n1,A,10,finance\n1,B,11,tech\n",
        )
        .unwrap();
        let panel = read_panel_csv(&path).unwrap();
        assert_eq!(panel.timestamps, vec![1, 2]);
        assert_eq!(panel.symbols, vec!["A", "B"]);
        assert_eq!(&panel.fields["CLOSE"][..2], &[10.0, 11.0]);
        assert!(panel.fields["CLOSE"][2].is_nan());
        assert_eq!(panel.fields["CLOSE"][3], 20.0);
        assert_eq!(panel.fields["SECTOR"][1], panel.fields["SECTOR"][3]);
        assert_ne!(panel.fields["SECTOR"][0], panel.fields["SECTOR"][1]);
    }

    #[test]
    fn panel_csv_rejects_duplicate_asset_dates() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("panel.csv");
        std::fs::write(&path, "timestamp,symbol,close\n1,A,10\n1,A,11\n").unwrap();
        assert!(read_panel_csv(&path).unwrap_err().contains("duplicate"));
    }
}
