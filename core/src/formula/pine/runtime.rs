//! Pine Script runtime semantics — bar-by-bar series evaluation, na propagation,
//! and cross-timeframe `request.security` mapping.

use std::collections::HashMap;

/// A single bar's series value — may be na (NaN).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SeriesValue {
    pub value: f64,
}

impl SeriesValue {
    pub fn new(value: f64) -> Self {
        Self { value }
    }

    pub fn na() -> Self {
        Self { value: f64::NAN }
    }

    pub fn is_na(&self) -> bool {
        self.value.is_nan()
    }
}

/// Output plot channel from Pine evaluation.
#[derive(Debug, Clone)]
pub struct PlotOutput {
    pub name: String,
    pub values: Vec<SeriesValue>,
}

/// Cross-timeframe security request descriptor.
#[derive(Debug, Clone)]
pub struct SecurityRequest {
    pub symbol: String,
    pub timeframe: String,
    pub expression: String,
}

/// Runtime error during Pine series evaluation.
#[derive(Debug, Clone)]
pub struct PineRuntimeError {
    pub message: String,
    pub bar_index: usize,
}

impl std::fmt::Display for PineRuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Pine runtime error at bar {}: {}",
            self.bar_index, self.message
        )
    }
}

impl std::error::Error for PineRuntimeError {}

/// Barstate flags for the current bar.
#[derive(Debug, Clone, Copy)]
pub struct BarState {
    pub isconfirmed: bool,
    pub islast: bool,
    pub isnew: bool,
    pub isrealtime: bool,
    pub ishistory: bool,
    pub isfirst: bool,
}

impl BarState {
    pub fn get_field(&self, field: &str) -> f64 {
        let v = match field {
            "isconfirmed" => self.isconfirmed,
            "islast" => self.islast,
            "isnew" => self.isnew,
            "isrealtime" => self.isrealtime,
            "ishistory" => self.ishistory,
            "isfirst" => self.isfirst,
            _ => false,
        };
        if v {
            1.0
        } else {
            0.0
        }
    }
}

/// Pine Script bar-by-bar runtime evaluator.
///
/// Implements:
/// - `series` type bar-by-bar evaluation
/// - `na` propagation rules
/// - `nz()`, `na()`, `fixnan()` helpers
/// - Single / multi plot output collection
/// - `request.security` cross-timeframe mapping
/// - History operator `[]` for series lookback
/// - `barstate` variables
pub struct PineRuntime {
    /// Bar count
    data_len: usize,
    /// Named series storage (per-bar vectors)
    series: HashMap<String, Vec<SeriesValue>>,
    /// Plot outputs
    plots: Vec<PlotOutput>,
    /// Pending security requests
    security_requests: Vec<SecurityRequest>,
    /// Current bar index during evaluation
    current_bar: usize,
    /// Bar state flags
    barstate: BarState,
}

impl PineRuntime {
    pub fn new(data_len: usize) -> Self {
        Self {
            data_len,
            series: HashMap::new(),
            plots: Vec::new(),
            security_requests: Vec::new(),
            current_bar: 0,
            barstate: BarState {
                isconfirmed: true,
                islast: false,
                isnew: true,
                isrealtime: false,
                ishistory: true,
                isfirst: false,
            },
        }
    }

    /// Register a raw data series (e.g. CLOSE, OPEN).
    pub fn register_series(&mut self, name: &str, values: &[f64]) {
        let series: Vec<SeriesValue> = values.iter().map(|&v| SeriesValue::new(v)).collect();
        self.series.insert(name.to_uppercase(), series);
    }

    /// Get series value at current bar.
    pub fn get(&self, name: &str) -> SeriesValue {
        let key = name.to_uppercase();
        self.series
            .get(&key)
            .and_then(|s| s.get(self.current_bar))
            .copied()
            .unwrap_or(SeriesValue::na())
    }

    /// Set series value at current bar.
    pub fn set(&mut self, name: &str, value: SeriesValue) {
        let key = name.to_uppercase();
        let entry = self
            .series
            .entry(key)
            .or_insert_with(|| (0..self.data_len).map(|_| SeriesValue::na()).collect());
        if self.current_bar < entry.len() {
            entry[self.current_bar] = value;
        }
    }

    /// History operator: get series value at `offset` bars back from current bar.
    /// `close[1]` means offset=1, i.e. previous bar.
    pub fn get_history(&self, name: &str, offset: usize) -> SeriesValue {
        let key = name.to_uppercase();
        if offset > self.current_bar {
            return SeriesValue::na();
        }
        let idx = self.current_bar - offset;
        self.series
            .get(&key)
            .and_then(|s| s.get(idx))
            .copied()
            .unwrap_or(SeriesValue::na())
    }

    /// Get barstate field value.
    pub fn get_barstate(&self, field: &str) -> SeriesValue {
        SeriesValue::new(self.barstate.get_field(field))
    }

    /// Update barstate flags for the current bar during evaluation.
    pub fn update_barstate(&mut self) {
        self.barstate.isfirst = self.current_bar == 0;
        self.barstate.islast = self.current_bar + 1 >= self.data_len;
        self.barstate.isnew = true;
        self.barstate.isconfirmed = true;
        self.barstate.ishistory = true;
        self.barstate.isrealtime = false;
    }

    /// Advance to next bar.
    pub fn next_bar(&mut self) {
        if self.current_bar + 1 < self.data_len {
            self.current_bar += 1;
            self.update_barstate();
        }
    }

    pub fn current_bar(&self) -> usize {
        self.current_bar
    }

    pub fn plots(&self) -> &[PlotOutput] {
        &self.plots
    }

    pub fn security_requests(&self) -> &[SecurityRequest] {
        &self.security_requests
    }

    // --- na propagation rules ---

    /// Binary arithmetic with na propagation: any na operand → na result.
    pub fn arith_add(&self, a: SeriesValue, b: SeriesValue) -> SeriesValue {
        if a.is_na() || b.is_na() {
            return SeriesValue::na();
        }
        SeriesValue::new(a.value + b.value)
    }

    pub fn arith_sub(&self, a: SeriesValue, b: SeriesValue) -> SeriesValue {
        if a.is_na() || b.is_na() {
            return SeriesValue::na();
        }
        SeriesValue::new(a.value - b.value)
    }

    pub fn arith_mul(&self, a: SeriesValue, b: SeriesValue) -> SeriesValue {
        if a.is_na() || b.is_na() {
            return SeriesValue::na();
        }
        SeriesValue::new(a.value * b.value)
    }

    pub fn arith_div(&self, a: SeriesValue, b: SeriesValue) -> SeriesValue {
        if a.is_na() || b.is_na() || b.value == 0.0 {
            return SeriesValue::na();
        }
        SeriesValue::new(a.value / b.value)
    }

    /// Comparison: na compared to anything → na (not true/false).
    pub fn compare_eq(&self, a: SeriesValue, b: SeriesValue) -> SeriesValue {
        if a.is_na() || b.is_na() {
            return SeriesValue::na();
        }
        SeriesValue::new(if a.value == b.value { 1.0 } else { 0.0 })
    }

    pub fn compare_gt(&self, a: SeriesValue, b: SeriesValue) -> SeriesValue {
        if a.is_na() || b.is_na() {
            return SeriesValue::na();
        }
        SeriesValue::new(if a.value > b.value { 1.0 } else { 0.0 })
    }

    pub fn compare_gte(&self, a: SeriesValue, b: SeriesValue) -> SeriesValue {
        if a.is_na() || b.is_na() {
            return SeriesValue::na();
        }
        SeriesValue::new(if a.value >= b.value { 1.0 } else { 0.0 })
    }

    pub fn compare_lt(&self, a: SeriesValue, b: SeriesValue) -> SeriesValue {
        if a.is_na() || b.is_na() {
            return SeriesValue::na();
        }
        SeriesValue::new(if a.value < b.value { 1.0 } else { 0.0 })
    }

    pub fn compare_lte(&self, a: SeriesValue, b: SeriesValue) -> SeriesValue {
        if a.is_na() || b.is_na() {
            return SeriesValue::na();
        }
        SeriesValue::new(if a.value <= b.value { 1.0 } else { 0.0 })
    }

    // --- Pine na helpers ---

    /// `nz(x, y)` — replace na with y (default 0).
    pub fn nz(&self, x: SeriesValue, replacement: Option<SeriesValue>) -> SeriesValue {
        let fallback = replacement.unwrap_or(SeriesValue::new(0.0));
        if x.is_na() {
            fallback
        } else {
            x
        }
    }

    /// `na(x)` — returns 1.0 if x is na, 0.0 otherwise.
    pub fn na_check(&self, x: SeriesValue) -> SeriesValue {
        SeriesValue::new(if x.is_na() { 1.0 } else { 0.0 })
    }

    /// `fixnan(x)` — forward-fill na values from previous bars.
    pub fn fixnan(&self, name: &str) -> SeriesValue {
        let key = name.to_uppercase();
        if let Some(series) = self.series.get(&key) {
            let mut last_valid = SeriesValue::na();
            for i in 0..=self.current_bar {
                if let Some(v) = series.get(i) {
                    if !v.is_na() {
                        last_valid = *v;
                    }
                }
            }
            last_valid
        } else {
            SeriesValue::na()
        }
    }

    /// `fixnan` on a single value using previous bar's stored value.
    pub fn fixnan_value(&self, name: &str, current: SeriesValue) -> SeriesValue {
        if !current.is_na() {
            return current;
        }
        if self.current_bar == 0 {
            return SeriesValue::na();
        }
        let key = name.to_uppercase();
        if let Some(series) = self.series.get(&key) {
            series
                .get(self.current_bar - 1)
                .copied()
                .unwrap_or(SeriesValue::na())
        } else {
            SeriesValue::na()
        }
    }

    // --- plot output ---

    /// Record a plot value at current bar.
    pub fn plot(&mut self, name: &str, value: SeriesValue) {
        let plot_name = name.to_uppercase();
        let plots = &mut self.plots;
        if let Some(p) = plots.iter_mut().find(|p| p.name == plot_name) {
            if self.current_bar < p.values.len() {
                p.values[self.current_bar] = value;
            }
        } else {
            let mut values = (0..self.data_len)
                .map(|_| SeriesValue::na())
                .collect::<Vec<_>>();
            if self.current_bar < values.len() {
                values[self.current_bar] = value;
            }
            plots.push(PlotOutput {
                name: plot_name,
                values,
            });
        }
    }

    /// Add a multi-plot output channel (e.g. MACD DIF/DEA/MACD).
    pub fn plot_multi(&mut self, names: &[&str], values: &[SeriesValue]) {
        for (name, value) in names.iter().zip(values.iter()) {
            self.plot(*name, *value);
        }
    }

    // --- request.security cross-timeframe mapping ---

    /// Register a `request.security(sym, timeframe, expr)` call.
    ///
    /// Common pattern: same symbol, different timeframe (e.g. daily close on intraday chart).
    pub fn request_security(
        &mut self,
        symbol: &str,
        timeframe: &str,
        expression: &str,
    ) -> SecurityRequest {
        let req = SecurityRequest {
            symbol: symbol.to_string(),
            timeframe: timeframe.to_string(),
            expression: expression.to_string(),
        };
        self.security_requests.push(req.clone());
        req
    }

    /// Map a security request to AlphaTA cross-timeframe lookup key.
    ///
    /// Format: `SECURITY:{symbol}:{timeframe}:{expression}`
    pub fn security_key(req: &SecurityRequest) -> String {
        format!(
            "SECURITY:{}:{}:{}",
            req.symbol, req.timeframe, req.expression
        )
    }

    /// Evaluate a simple expression tree bar-by-bar over registered series.
    ///
    /// Supports variable lookup, numeric literals, unary and binary arithmetic,
    /// comparison and logical operators with full `na` propagation. Used by the
    /// bar-by-bar series evaluation path and runtime smoke tests.
    pub fn eval_bar(
        &self,
        op: &str,
        left_name: Option<&str>,
        right_name: Option<&str>,
    ) -> Result<SeriesValue, PineRuntimeError> {
        match op {
            "var" => {
                let name = left_name.unwrap_or("");
                Ok(self.get(name))
            }
            "num" | "lit" => {
                let raw = left_name.unwrap_or("").parse::<f64>().unwrap_or(f64::NAN);
                Ok(SeriesValue::new(raw))
            }
            "add" => {
                let a = self.get(left_name.unwrap_or(""));
                let b = self.get(right_name.unwrap_or(""));
                Ok(self.arith_add(a, b))
            }
            "sub" => {
                let a = self.get(left_name.unwrap_or(""));
                let b = self.get(right_name.unwrap_or(""));
                Ok(self.arith_sub(a, b))
            }
            "mul" => {
                let a = self.get(left_name.unwrap_or(""));
                let b = self.get(right_name.unwrap_or(""));
                Ok(self.arith_mul(a, b))
            }
            "div" => {
                let a = self.get(left_name.unwrap_or(""));
                let b = self.get(right_name.unwrap_or(""));
                Ok(self.arith_div(a, b))
            }
            "eq" => {
                let a = self.get(left_name.unwrap_or(""));
                let b = self.get(right_name.unwrap_or(""));
                Ok(self.compare_eq(a, b))
            }
            "gt" => {
                let a = self.get(left_name.unwrap_or(""));
                let b = self.get(right_name.unwrap_or(""));
                Ok(self.compare_gt(a, b))
            }
            "lt" => {
                let a = self.get(left_name.unwrap_or(""));
                let b = self.get(right_name.unwrap_or(""));
                Ok(self.compare_lt(a, b))
            }
            "gte" => {
                let a = self.get(left_name.unwrap_or(""));
                let b = self.get(right_name.unwrap_or(""));
                Ok(self.compare_gte(a, b))
            }
            "lte" => {
                let a = self.get(left_name.unwrap_or(""));
                let b = self.get(right_name.unwrap_or(""));
                Ok(self.compare_lte(a, b))
            }
            "and" => {
                let a = self.get(left_name.unwrap_or(""));
                let b = self.get(right_name.unwrap_or(""));
                Ok(self.logical_and(a, b))
            }
            "or" => {
                let a = self.get(left_name.unwrap_or(""));
                let b = self.get(right_name.unwrap_or(""));
                Ok(self.logical_or(a, b))
            }
            "neg" => {
                let a = self.get(left_name.unwrap_or(""));
                Ok(self.arith_sub(SeriesValue::new(0.0), a))
            }
            "not" => {
                let a = self.get(left_name.unwrap_or(""));
                Ok(self.logical_not(a))
            }
            _ => Err(PineRuntimeError {
                message: format!("Unknown eval op: {}", op),
                bar_index: self.current_bar,
            }),
        }
    }

    /// Logical AND (treats non-zero as true, na as na).
    pub fn logical_and(&self, a: SeriesValue, b: SeriesValue) -> SeriesValue {
        if a.is_na() || b.is_na() {
            return SeriesValue::na();
        }
        SeriesValue::new(if a.value != 0.0 && b.value != 0.0 {
            1.0
        } else {
            0.0
        })
    }

    /// Logical OR (treats non-zero as true, na as na).
    pub fn logical_or(&self, a: SeriesValue, b: SeriesValue) -> SeriesValue {
        if a.is_na() || b.is_na() {
            return SeriesValue::na();
        }
        SeriesValue::new(if a.value != 0.0 || b.value != 0.0 {
            1.0
        } else {
            0.0
        })
    }

    /// Logical NOT (na → na; 0 → 1; non-zero → 0).
    pub fn logical_not(&self, a: SeriesValue) -> SeriesValue {
        if a.is_na() {
            return SeriesValue::na();
        }
        SeriesValue::new(if a.value == 0.0 { 1.0 } else { 0.0 })
    }

    /// Run bar-by-bar evaluation over all bars for registered series.
    pub fn run_all_bars<F>(&mut self, mut per_bar: F)
    where
        F: FnMut(&mut PineRuntime, usize),
    {
        for bar in 0..self.data_len {
            self.current_bar = bar;
            per_bar(self, bar);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_na_propagation() {
        let rt = PineRuntime::new(3);
        let na = SeriesValue::na();
        let one = SeriesValue::new(1.0);
        assert!(rt.arith_add(na, one).is_na());
        assert!(rt.compare_eq(na, one).is_na());
    }

    #[test]
    fn test_nz() {
        let rt = PineRuntime::new(1);
        let na = SeriesValue::na();
        let result = rt.nz(na, Some(SeriesValue::new(5.0)));
        assert_eq!(result.value, 5.0);
        let result2 = rt.nz(SeriesValue::new(3.0), None);
        assert_eq!(result2.value, 3.0);
    }

    #[test]
    fn test_na_check() {
        let rt = PineRuntime::new(1);
        assert_eq!(rt.na_check(SeriesValue::na()).value, 1.0);
        assert_eq!(rt.na_check(SeriesValue::new(0.0)).value, 0.0);
    }

    #[test]
    fn test_plot_output() {
        let mut rt = PineRuntime::new(5);
        rt.plot("MA", SeriesValue::new(100.0));
        assert_eq!(rt.plots().len(), 1);
        assert_eq!(rt.plots()[0].values[0].value, 100.0);
    }

    #[test]
    fn test_security_request() {
        let mut rt = PineRuntime::new(10);
        let req = rt.request_security("AAPL", "D", "close");
        let key = PineRuntime::security_key(&req);
        assert_eq!(key, "SECURITY:AAPL:D:close");
        assert_eq!(rt.security_requests().len(), 1);
    }

    #[test]
    fn test_bar_by_bar() {
        let mut rt = PineRuntime::new(3);
        rt.register_series("CLOSE", &[10.0, 20.0, 30.0]);
        rt.run_all_bars(|rt, _bar| {
            let v = rt.get("CLOSE");
            rt.plot("C", v);
        });
        assert_eq!(rt.plots()[0].values[2].value, 30.0);
    }

    #[test]
    fn test_fixnan() {
        let mut rt = PineRuntime::new(3);
        rt.register_series("X", &[f64::NAN, f64::NAN, 5.0]);
        rt.current_bar = 2;
        let v = rt.fixnan("X");
        assert_eq!(v.value, 5.0);
    }

    #[test]
    fn test_history_lookback() {
        let mut rt = PineRuntime::new(5);
        rt.register_series("CLOSE", &[10.0, 20.0, 30.0, 40.0, 50.0]);
        rt.current_bar = 3;
        assert_eq!(rt.get_history("CLOSE", 0).value, 40.0);
        assert_eq!(rt.get_history("CLOSE", 1).value, 30.0);
        assert_eq!(rt.get_history("CLOSE", 2).value, 20.0);
        assert!(rt.get_history("CLOSE", 10).is_na());
    }

    #[test]
    fn test_barstate_fields() {
        let mut rt = PineRuntime::new(5);
        rt.current_bar = 0;
        rt.update_barstate();
        assert_eq!(rt.get_barstate("isfirst").value, 1.0);
        assert_eq!(rt.get_barstate("islast").value, 0.0);
        assert_eq!(rt.get_barstate("isconfirmed").value, 1.0);
        assert_eq!(rt.get_barstate("ishistory").value, 1.0);

        rt.current_bar = 4;
        rt.update_barstate();
        assert_eq!(rt.get_barstate("islast").value, 1.0);
        assert_eq!(rt.get_barstate("isfirst").value, 0.0);
    }

    #[test]
    fn test_eval_bar_comparison_ops() {
        let mut rt = PineRuntime::new(3);
        rt.register_series("CLOSE", &[10.0, 20.0, 30.0]);
        rt.current_bar = 1;
        let gt = rt.eval_bar("gt", Some("CLOSE"), Some("CLOSE")).unwrap();
        // 20 > 20 is false
        assert_eq!(gt.value, 0.0);
        let gte = rt.eval_bar("gte", Some("CLOSE"), Some("CLOSE")).unwrap();
        assert_eq!(gte.value, 1.0);
        let lt = rt.eval_bar("lt", Some("CLOSE"), Some("CLOSE")).unwrap();
        assert_eq!(lt.value, 0.0);
    }

    #[test]
    fn test_eval_bar_logical_and_neg_not() {
        let mut rt = PineRuntime::new(1);
        rt.register_series("A", &[1.0]);
        rt.register_series("B", &[0.0]);
        rt.current_bar = 0;
        let and = rt.eval_bar("and", Some("A"), Some("B")).unwrap();
        assert_eq!(and.value, 0.0);
        let or = rt.eval_bar("or", Some("A"), Some("B")).unwrap();
        assert_eq!(or.value, 1.0);
        let neg = rt.eval_bar("neg", Some("A"), None).unwrap();
        assert_eq!(neg.value, -1.0);
        let not = rt.eval_bar("not", Some("B"), None).unwrap();
        assert_eq!(not.value, 1.0);
    }

    #[test]
    fn test_eval_bar_num_literal_unknown_op() {
        let rt = PineRuntime::new(1);
        let num = rt.eval_bar("num", Some("42"), None).unwrap();
        assert_eq!(num.value, 42.0);
        // Unknown op must surface an error (not silently Ok).
        assert!(rt.eval_bar("frobnicate", Some("X"), None).is_err());
        // Arithmetic on an unregistered series yields na (Ok, not Err).
        let na = rt.eval_bar("add", Some("X"), None).unwrap();
        assert!(na.is_na());
    }
}
