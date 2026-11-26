//! Two-axis factor evaluation for aligned `time × symbols` market panels.
//!
//! Time-series operators run independently down each symbol column. Explicit
//! cross-sectional operators (`RANK(x)`, `CS_RANK(x)`, `SCALE(x)`, and
//! `INDNEUTRALIZE(x, group)`) run across each timestamp row. This keeps
//! cross-sectional ranking distinct from the rolling `RANK(x, period)`
//! function used by the single-series formula API.

use std::collections::{BTreeMap, HashMap};
use std::sync::OnceLock;

use ndarray::Array1;

use crate::data_contract::CrossSectionView;
use crate::factors::{FactorError, FactorResult};
use crate::formula::functions::FormulaFn;
use crate::formula::{
    get_builtin_functions, parse_formula, AstNode, BinaryOperator, FormulaContext, UnaryOperator,
};

static BUILTIN_FUNCTIONS: OnceLock<HashMap<String, FormulaFn>> = OnceLock::new();

/// Aligned named fields for a panel. Each field is row-major with shape
/// `(timestamps.len(), symbols.len())`.
#[derive(Debug, Clone)]
pub struct FactorPanel<'a> {
    timestamps: &'a [i64],
    symbols: &'a [&'a str],
    fields: BTreeMap<String, CrossSectionView<'a>>,
}

impl<'a> FactorPanel<'a> {
    /// Construct a panel from named row-major fields.
    pub fn new(
        timestamps: &'a [i64],
        symbols: &'a [&'a str],
        fields: &[(&str, &'a [f64])],
    ) -> FactorResult<Self> {
        if timestamps.is_empty() || symbols.is_empty() {
            return Err(FactorError::InvalidParameter(
                "panel timestamps and symbols must both be non-empty".to_string(),
            ));
        }
        if fields.is_empty() {
            return Err(FactorError::InvalidParameter(
                "panel requires at least one input field".to_string(),
            ));
        }
        let expected = timestamps.len().checked_mul(symbols.len()).ok_or_else(|| {
            FactorError::InvalidParameter("panel dimensions overflow".to_string())
        })?;
        let mut mapped = BTreeMap::new();
        for (index, (name, values)) in fields.iter().enumerate() {
            let key = name.trim().to_ascii_uppercase();
            if key.is_empty() {
                return Err(FactorError::InvalidParameter(
                    "panel field names must not be empty".to_string(),
                ));
            }
            if mapped.contains_key(&key) {
                return Err(FactorError::InvalidParameter(format!(
                    "duplicate panel field: {name}"
                )));
            }
            let view = if index == 0 {
                CrossSectionView::new(timestamps, symbols, values).map_err(|error| {
                    FactorError::InvalidParameter(format!("invalid panel field {name}: {error}"))
                })?
            } else {
                if values.len() != expected {
                    return Err(FactorError::LengthMismatch {
                        name: name.to_string(),
                        expected,
                        actual: values.len(),
                    });
                }
                CrossSectionView {
                    timestamps,
                    symbols,
                    values,
                }
            };
            mapped.insert(key, view);
        }
        Ok(Self {
            timestamps,
            symbols,
            fields: mapped,
        })
    }

    /// Ordered timestamps.
    pub fn timestamps(&self) -> &[i64] {
        self.timestamps
    }

    /// Ordered symbol columns.
    pub fn symbols(&self) -> &[&'a str] {
        self.symbols
    }

    /// Field names in deterministic, canonical uppercase order.
    pub fn field_names(&self) -> impl Iterator<Item = &str> {
        self.fields.keys().map(String::as_str)
    }

    /// Evaluate an expression over the panel and return one value per cell.
    pub fn evaluate(&self, expression: &str) -> FactorResult<FactorPanelResult> {
        let ast = parse_formula(expression)
            .map_err(|error| FactorError::Compute(format!("formula parse failed: {error}")))?;
        let functions = BUILTIN_FUNCTIONS.get_or_init(get_builtin_functions);
        let values = evaluate_node(&ast, self, &functions)?;
        Ok(FactorPanelResult {
            timestamps: self.timestamps.to_vec(),
            symbols: self
                .symbols
                .iter()
                .map(|value| (*value).to_string())
                .collect(),
            values: values.data,
        })
    }
}

/// One computed panel, row-major in `(time, symbols)` order.
#[derive(Debug, Clone, PartialEq)]
pub struct FactorPanelResult {
    /// Timestamp axis.
    pub timestamps: Vec<i64>,
    /// Symbol axis.
    pub symbols: Vec<String>,
    /// Row-major values. Cell `(time, symbol)` is `values[time * symbols.len() + symbol]`.
    pub values: Vec<f64>,
}

impl FactorPanelResult {
    /// Number of timestamp rows.
    pub fn rows(&self) -> usize {
        self.timestamps.len()
    }

    /// Number of symbols.
    pub fn cols(&self) -> usize {
        self.symbols.len()
    }

    /// Read a cell by axis index.
    pub fn get(&self, row: usize, column: usize) -> Option<f64> {
        if row >= self.rows() || column >= self.cols() {
            return None;
        }
        self.values.get(row * self.cols() + column).copied()
    }
}

#[derive(Debug, Clone)]
struct PanelValues {
    rows: usize,
    cols: usize,
    data: Vec<f64>,
}

impl PanelValues {
    fn new(rows: usize, cols: usize, data: Vec<f64>) -> FactorResult<Self> {
        let expected = rows.checked_mul(cols).ok_or_else(|| {
            FactorError::InvalidParameter("panel dimensions overflow address space".to_string())
        })?;
        if data.len() != expected {
            return Err(FactorError::LengthMismatch {
                name: "panel result".to_string(),
                expected,
                actual: data.len(),
            });
        }
        Ok(Self { rows, cols, data })
    }

    fn constant(rows: usize, cols: usize, value: f64) -> Self {
        Self {
            rows,
            cols,
            data: vec![value; rows * cols],
        }
    }

    fn column(&self, col: usize) -> Array1<f64> {
        Array1::from_iter((0..self.rows).map(|row| self.data[row * self.cols + col]))
    }

    fn from_columns(rows: usize, cols: usize, columns: Vec<Array1<f64>>) -> FactorResult<Self> {
        let mut data = vec![f64::NAN; rows * cols];
        for (col, values) in columns.into_iter().enumerate() {
            if values.len() != rows {
                return Err(FactorError::LengthMismatch {
                    name: format!("symbol column {col}"),
                    expected: rows,
                    actual: values.len(),
                });
            }
            for (row, value) in values.into_iter().enumerate() {
                data[row * cols + col] = value;
            }
        }
        Self::new(rows, cols, data)
    }
}

fn evaluate_node(
    node: &AstNode,
    panel: &FactorPanel<'_>,
    functions: &HashMap<String, FormulaFn>,
) -> FactorResult<PanelValues> {
    let rows = panel.timestamps.len();
    let cols = panel.symbols.len();
    match node {
        AstNode::Number(value) => Ok(PanelValues::constant(rows, cols, *value)),
        AstNode::Variable(name) => resolve_variable(name, panel),
        AstNode::BinaryOp { op, left, right } => {
            let left = evaluate_node(left, panel, functions)?;
            let right = evaluate_node(right, panel, functions)?;
            let values = left
                .data
                .iter()
                .zip(&right.data)
                .map(|(a, b)| binary_value(op, *a, *b))
                .collect();
            PanelValues::new(rows, cols, values)
        }
        AstNode::UnaryOp { op, expr } => {
            let values = evaluate_node(expr, panel, functions)?;
            let data = values
                .data
                .into_iter()
                .map(|value| match op {
                    UnaryOperator::Neg => -value,
                    UnaryOperator::Not => f64::from(value == 0.0),
                })
                .collect();
            PanelValues::new(rows, cols, data)
        }
        AstNode::FunctionCall { name, args } => {
            let evaluated = args
                .iter()
                .map(|arg| evaluate_node(arg, panel, functions))
                .collect::<FactorResult<Vec<_>>>()?;
            evaluate_call(name, &evaluated, panel, functions)
        }
        AstNode::IfThenElse {
            cond,
            then_branch,
            else_branch,
        } => {
            let cond = evaluate_node(cond, panel, functions)?;
            let then_values = evaluate_node(then_branch, panel, functions)?;
            let else_values = evaluate_node(else_branch, panel, functions)?;
            let data = cond
                .data
                .iter()
                .zip(then_values.data.iter().zip(&else_values.data))
                .map(|(condition, (yes, no))| if *condition != 0.0 { *yes } else { *no })
                .collect();
            PanelValues::new(rows, cols, data)
        }
        AstNode::Assignment { expr, .. }
        | AstNode::Output { expr, .. }
        | AstNode::CompoundAssignment { expr, .. } => evaluate_node(expr, panel, functions),
        AstNode::Statements(statements) => {
            let last = statements.last().ok_or_else(|| {
                FactorError::InvalidParameter(
                    "formula has no value-producing statement".to_string(),
                )
            })?;
            evaluate_node(last, panel, functions)
        }
        _ => Err(FactorError::Compute(
            "panel formulas accept numeric expressions, assignments, and function calls"
                .to_string(),
        )),
    }
}

fn resolve_variable(name: &str, panel: &FactorPanel<'_>) -> FactorResult<PanelValues> {
    let canonical = name.to_ascii_uppercase();
    let rows = panel.timestamps.len();
    let cols = panel.symbols.len();
    if let Some(field) = panel.fields.get(&canonical) {
        return PanelValues::new(rows, cols, field.values.to_vec());
    }

    if canonical == "RETURNS" {
        let close = panel.fields.get("CLOSE").ok_or_else(|| {
            FactorError::MissingInput("close (required to derive returns)".to_string())
        })?;
        let mut output = vec![f64::NAN; rows * cols];
        for row in 1..rows {
            for col in 0..cols {
                let current = close.values[row * cols + col];
                let previous = close.values[(row - 1) * cols + col];
                if current.is_finite() && previous.is_finite() && previous != 0.0 {
                    output[row * cols + col] = current / previous - 1.0;
                }
            }
        }
        return PanelValues::new(rows, cols, output);
    }

    if let Some(period) = canonical
        .strip_prefix("ADV")
        .filter(|suffix| !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit()))
        .and_then(|suffix| suffix.parse::<usize>().ok())
    {
        if period == 0 {
            return Err(FactorError::InvalidParameter(
                "ADV period must be positive".to_string(),
            ));
        }
        let dollar_volume = if let Some(field) = panel
            .fields
            .get("DOLLAR_VOLUME")
            .or_else(|| panel.fields.get("AMOUNT"))
        {
            field.values.to_vec()
        } else {
            let close = panel.fields.get("CLOSE").ok_or_else(|| {
                FactorError::MissingInput(
                    "dollar_volume, or close and volume to derive dollar_volume".to_string(),
                )
            })?;
            let volume = panel.fields.get("VOLUME").ok_or_else(|| {
                FactorError::MissingInput(
                    "dollar_volume, or close and volume to derive dollar_volume".to_string(),
                )
            })?;
            close
                .values
                .iter()
                .zip(volume.values)
                .map(|(price, shares)| {
                    if price.is_finite() && shares.is_finite() {
                        price * shares
                    } else {
                        f64::NAN
                    }
                })
                .collect()
        };
        let mut output = vec![f64::NAN; rows * cols];
        for col in 0..cols {
            for row in period.saturating_sub(1)..rows {
                let start = row + 1 - period;
                let mut sum = 0.0;
                let mut valid = true;
                for index in start..=row {
                    let value = dollar_volume[index * cols + col];
                    if !value.is_finite() {
                        valid = false;
                        break;
                    }
                    sum += value;
                }
                if valid {
                    output[row * cols + col] = sum / period as f64;
                }
            }
        }
        return PanelValues::new(rows, cols, output);
    }

    Err(FactorError::MissingInput(name.to_string()))
}

fn evaluate_call(
    name: &str,
    args: &[PanelValues],
    panel: &FactorPanel<'_>,
    functions: &HashMap<String, FormulaFn>,
) -> FactorResult<PanelValues> {
    let canonical = name.to_ascii_uppercase();
    let rows = panel.timestamps.len();
    let cols = panel.symbols.len();

    if matches!(canonical.as_str(), "RANK" | "CS_RANK") && args.len() == 1 {
        return cross_sectional_unary(&args[0], |row| crate::formula::ops::percentile_rank(row));
    }
    if matches!(canonical.as_str(), "CS_ZSCORE" | "ZSCORE") && args.len() == 1 {
        return cross_sectional_unary(&args[0], |row| crate::formula::ops::zscore(row));
    }
    if matches!(canonical.as_str(), "SCALE" | "CS_SCALE") && (1..=2).contains(&args.len()) {
        let multiplier = if args.len() == 2 {
            args[1].data[0]
        } else {
            1.0
        };
        return cross_sectional_unary(&args[0], |row| crate::formula::ops::scale(row, multiplier));
    }
    if matches!(canonical.as_str(), "INDNEUTRALIZE" | "CS_INDNEUTRALIZE") && args.len() == 2 {
        let mut output = vec![f64::NAN; rows * cols];
        for row in 0..rows {
            let start = row * cols;
            let values = ndarray::ArrayView1::from(&args[0].data[start..start + cols]);
            let groups = ndarray::ArrayView1::from(&args[1].data[start..start + cols]);
            let neutralized = crate::formula::ops::indneutralize(values, groups);
            output[start..start + cols].copy_from_slice(neutralized.as_slice().unwrap_or(&[]));
        }
        return PanelValues::new(rows, cols, output);
    }
    if matches!(canonical.as_str(), "SIGNEDPOWER" | "SIGNED_POWER") && args.len() == 2 {
        // Paper spelling retained as a panel alias. The function registry's
        // CS_SIGNED_POWER kernel is element-wise; applying it per symbol keeps
        // that kernel's NaN and exponent semantics consistent with 1D formulas.
        let function = functions.get("CS_SIGNED_POWER").ok_or_else(|| {
            FactorError::Compute("unsupported panel formula function: SIGNEDPOWER".to_string())
        })?;
        let mut output_columns = Vec::with_capacity(cols);
        for symbol_index in 0..cols {
            let input = args[0].column(symbol_index);
            let exponent = args[1].column(symbol_index);
            output_columns.push(
                function(&context_for_symbol(panel, symbol_index), &[input, exponent]).map_err(
                    |error| FactorError::Compute(format!("panel function {name}: {error}")),
                )?,
            );
        }
        return PanelValues::from_columns(rows, cols, output_columns);
    }
    if canonical == "DELTA" && args.len() == 2 {
        let period = positive_period(args[1].data[0], "DELTA")?;
        let mut output = Vec::with_capacity(rows * cols);
        for index in 0..rows * cols {
            let row = index / cols;
            output.push(if row < period {
                f64::NAN
            } else {
                args[0].data[index] - args[0].data[index - period * cols]
            });
        }
        return PanelValues::new(rows, cols, output);
    }
    if matches!(canonical.as_str(), "TS_ARGMAX" | "TS_ARGMIN") && args.len() == 2 {
        let period = positive_period(args[1].data[0], name)?;
        let output = (0..cols)
            .map(|col| {
                let input = args[0].column(col);
                if canonical == "TS_ARGMAX" {
                    crate::formula::ops::ts_argmax(input.view(), period)
                } else {
                    crate::formula::ops::ts_argmin(input.view(), period)
                }
            })
            .collect();
        return PanelValues::from_columns(rows, cols, output);
    }
    if matches!(canonical.as_str(), "PRODUCT" | "TS_PRODUCT") && args.len() == 2 {
        let period = positive_period(args[1].data[0], name)?;
        let mut output = vec![f64::NAN; rows * cols];
        for col in 0..cols {
            for row in period.saturating_sub(1)..rows {
                let start = row + 1 - period;
                let window = (start..=row).map(|i| args[0].data[i * cols + col]);
                let mut product = 1.0;
                let mut valid = true;
                for value in window {
                    if !value.is_finite() {
                        valid = false;
                        break;
                    }
                    product *= value;
                }
                if valid {
                    output[row * cols + col] = product;
                }
            }
        }
        return PanelValues::new(rows, cols, output);
    }
    if matches!(canonical.as_str(), "TS_COVARIANCE" | "COVARIANCE") && args.len() == 3 {
        let period = positive_period(args[2].data[0], name)?;
        let output = (0..cols)
            .map(|col| {
                crate::formula::ops::covariance(
                    args[0].column(col).view(),
                    args[1].column(col).view(),
                    period,
                )
            })
            .collect();
        return PanelValues::from_columns(rows, cols, output);
    }

    let constant_window = |arg: &PanelValues| {
        let first = *arg.data.first()?;
        (first.is_finite() && arg.data.iter().all(|value| *value == first)).then_some(first)
    };
    let function_name = match canonical.as_str() {
        // The paper defines min(x, d) and max(x, d) as rolling time-series
        // reducers. Keep the general formula engine's element-wise behavior
        // when the second panel argument is data rather than a scalar window.
        "MIN" if args.len() == 2 && constant_window(&args[1]).is_some() => "LLV",
        "MAX" if args.len() == 2 && constant_window(&args[1]).is_some() => "HHV",
        // Match the sample rolling deviation used by the existing formula
        // transcription; the paper leaves the estimator convention unstated.
        "STDDEV" => "STDDEV_SAMPLE",
        "TS_RANK" => "RANK_PCT",
        "TS_MEAN" => "MA",
        "TS_MIN" => "LLV",
        "TS_MAX" => "HHV",
        "TS_STDDEV" => "STDDEV_SAMPLE",
        "CORRELATION" | "TS_CORRELATION" => "CORREL",
        "TS_SUM" => "SUM",
        "LOG" => "LN",
        "DELAY" => "REF",
        "DECAY_LINEAR" => "WMA",
        other => other,
    };
    let function = functions.get(function_name).ok_or_else(|| {
        FactorError::Compute(format!("unsupported panel formula function: {name}"))
    })?;

    let mut result_columns = Vec::with_capacity(cols);
    for symbol_index in 0..cols {
        let arg_columns: Vec<Array1<f64>> =
            args.iter().map(|arg| arg.column(symbol_index)).collect();
        let context = context_for_symbol(panel, symbol_index);
        let output = function(&context, &arg_columns)
            .map_err(|error| FactorError::Compute(format!("panel function {name}: {error}")))?;
        result_columns.push(output);
    }
    PanelValues::from_columns(rows, cols, result_columns)
}

fn context_for_symbol(panel: &FactorPanel<'_>, symbol: usize) -> FormulaContext {
    let series = |name: &str| -> Array1<f64> {
        panel
            .fields
            .get(name)
            .map(|view| {
                Array1::from_iter(
                    (0..panel.timestamps.len())
                        .map(|row| view.values[row * panel.symbols.len() + symbol]),
                )
            })
            .unwrap_or_else(|| Array1::from_elem(panel.timestamps.len(), f64::NAN))
    };
    let mut context = FormulaContext::new(
        series("OPEN"),
        series("HIGH"),
        series("LOW"),
        series("CLOSE"),
        series("VOLUME"),
        None,
    );
    for (name, view) in &panel.fields {
        let values = Array1::from_iter(
            (0..panel.timestamps.len()).map(|row| view.values[row * panel.symbols.len() + symbol]),
        );
        if name == "AMOUNT" {
            context.amount = Some(values);
        } else if !matches!(name.as_str(), "OPEN" | "HIGH" | "LOW" | "CLOSE" | "VOLUME") {
            context.set_variable(name.clone(), values);
        }
    }
    context
}

fn cross_sectional_unary(
    input: &PanelValues,
    operation: impl Fn(ndarray::ArrayView1<'_, f64>) -> Array1<f64>,
) -> FactorResult<PanelValues> {
    let mut output = Vec::with_capacity(input.rows * input.cols);
    for row in 0..input.rows {
        let start = row * input.cols;
        let result = operation(ndarray::ArrayView1::from(
            &input.data[start..start + input.cols],
        ));
        output.extend(result);
    }
    PanelValues::new(input.rows, input.cols, output)
}

fn positive_period(value: f64, name: &str) -> FactorResult<usize> {
    if !value.is_finite() || value < 1.0 {
        return Err(FactorError::InvalidParameter(format!(
            "{name} period must be a positive integer"
        )));
    }
    Ok(value.floor() as usize)
}

fn binary_value(op: &BinaryOperator, left: f64, right: f64) -> f64 {
    match op {
        BinaryOperator::Add => left + right,
        BinaryOperator::Sub => left - right,
        BinaryOperator::Mul => left * right,
        BinaryOperator::Div => left / right,
        BinaryOperator::Mod => left % right,
        BinaryOperator::Pow => left.powf(right),
        BinaryOperator::Gt => f64::from(left > right),
        BinaryOperator::Lt => f64::from(left < right),
        BinaryOperator::Gte => f64::from(left >= right),
        BinaryOperator::Lte => f64::from(left <= right),
        BinaryOperator::Eq => f64::from(left == right),
        BinaryOperator::Neq => f64::from(left != right),
        BinaryOperator::And => f64::from(left != 0.0 && right != 0.0),
        BinaryOperator::Or => f64::from(left != 0.0 || right != 0.0),
        BinaryOperator::Xor => f64::from((left != 0.0) ^ (right != 0.0)),
        BinaryOperator::StringConcat => f64::NAN,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn panel<'a>(
        timestamps: &'a [i64],
        symbols: &'a [&'a str],
        fields: &[(&str, &'a [f64])],
    ) -> FactorPanel<'a> {
        FactorPanel::new(timestamps, symbols, fields).unwrap()
    }

    #[test]
    fn rolling_operators_run_down_symbol_columns_and_rank_runs_across_rows() {
        let dates = [1, 2, 3];
        let symbols = ["A", "B"];
        let close = [1.0, 10.0, 2.0, 20.0, 3.0, 30.0];
        let data = panel(&dates, &symbols, &[("close", &close)]);

        let delayed = data.evaluate("REF(CLOSE, 1)").unwrap();
        assert!(delayed.get(0, 0).unwrap().is_nan());
        assert_eq!(delayed.get(1, 0), Some(1.0));
        assert_eq!(delayed.get(1, 1), Some(10.0));

        let ranked = data.evaluate("RANK(CLOSE)").unwrap();
        assert_eq!(ranked.get(0, 0), Some(0.0));
        assert_eq!(ranked.get(0, 1), Some(1.0));
        assert_eq!(ranked.get(2, 1), Some(1.0));
    }

    #[test]
    fn nested_cross_section_and_time_series_calls_compose_in_one_expression() {
        let dates = [1, 2, 3];
        let symbols = ["A", "B", "C"];
        let close = [1., 10., 100., 2., 20., 200., 3., 30., 300.];
        let data = panel(&dates, &symbols, &[("close", &close)]);
        let result = data.evaluate("MA(CS_RANK(CLOSE), 2)").unwrap();
        assert!(result.get(0, 0).unwrap().is_nan());
        assert!(result.get(2, 0).unwrap().abs() < 1e-12);
        assert!((result.get(2, 2).unwrap() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn group_neutralization_uses_each_date_and_group_independently() {
        let dates = [1, 2];
        let symbols = ["A", "B", "C", "D"];
        let close = [10., 20., 30., 40., 20., 30., 40., 50.];
        let sector = [1., 1., 2., 2., 1., 1., 2., 2.];
        let data = panel(&dates, &symbols, &[("close", &close), ("sector", &sector)]);
        let result = data.evaluate("INDNEUTRALIZE(CLOSE, SECTOR)").unwrap();
        assert_eq!(result.get(0, 0), Some(-5.0));
        assert_eq!(result.get(0, 1), Some(5.0));
        assert_eq!(result.get(0, 2), Some(-5.0));
        assert_eq!(result.get(1, 0), Some(-5.0));
        assert_eq!(result.get(1, 3), Some(5.0));
    }

    #[test]
    fn common_market_variables_are_derived_with_per_symbol_history() {
        let dates = [1, 2, 3];
        let symbols = ["A", "B"];
        let close = [2., 10., 4., 10., 6., 10.];
        let volume = [100., 1., 100., 1., 100., 1.];
        let data = panel(&dates, &symbols, &[("close", &close), ("volume", &volume)]);

        let adv = data.evaluate("ADV2").unwrap();
        assert!(adv.get(0, 0).unwrap().is_nan());
        assert_eq!(adv.get(1, 0), Some(300.0));
        assert_eq!(adv.get(2, 0), Some(500.0));

        let returns = data.evaluate("RETURNS").unwrap();
        assert!(returns.get(0, 0).unwrap().is_nan());
        assert_eq!(returns.get(1, 0), Some(1.0));
        assert_eq!(returns.get(2, 0), Some(0.5));
    }

    #[test]
    fn built_in_library_factors_are_evaluable_over_panel_axes() {
        let dates = [1, 2, 3];
        let symbols = ["A", "B"];
        let open = [10., 20., 11., 21., 12., 22.];
        let high = [11., 21., 12., 22., 13., 23.];
        let low = [9., 19., 10., 20., 11., 21.];
        let close = [10.5, 20.5, 11.5, 21.5, 12.5, 22.5];
        let volume = [100., 200., 110., 210., 120., 220.];
        let data = panel(
            &dates,
            &symbols,
            &[
                ("open", &open),
                ("high", &high),
                ("low", &low),
                ("close", &close),
                ("volume", &volume),
            ],
        );
        let library = crate::factors::builtin::factor_library("worldquant101").unwrap();
        let result = library.evaluate_panel("Alpha6", &data).unwrap();
        assert_eq!(result.rows(), 3);
        assert_eq!(result.cols(), 2);
        assert!(result.get(0, 0).unwrap().is_nan());
    }

    #[test]
    fn panel_rejects_misaligned_field_sizes_and_duplicate_fields() {
        let dates = [1, 2];
        let symbols = ["A", "B"];
        assert!(FactorPanel::new(&dates, &symbols, &[("close", &[1.0])]).is_err());
        let values = [1.0; 4];
        assert!(
            FactorPanel::new(&dates, &symbols, &[("close", &values), ("CLOSE", &values)]).is_err()
        );
    }
}
