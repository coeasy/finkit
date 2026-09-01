use ndarray::{Array1, ArrayView1};
use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use crate::formula::ast::OutputModifier;
use crate::formula::drawing::DrawResult;
use crate::formula::sandbox::{sandbox_reset, ExecSandboxConfig, ExecSandboxState};

pub type FormulaResult = std::result::Result<Array1<f64>, FormulaError>;
pub type VarName = Arc<str>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BuiltinVar {
    Close,
    High,
    Low,
    Open,
    Volume,
    Amount,
    BarsCount,
    BarPos,
    Capital,
    DrawNull,
}

#[inline]
pub fn classify_builtin_var(name: &str) -> Option<BuiltinVar> {
    if name.eq_ignore_ascii_case("C") || name.eq_ignore_ascii_case("CLOSE") {
        Some(BuiltinVar::Close)
    } else if name.eq_ignore_ascii_case("H") || name.eq_ignore_ascii_case("HIGH") {
        Some(BuiltinVar::High)
    } else if name.eq_ignore_ascii_case("L") || name.eq_ignore_ascii_case("LOW") {
        Some(BuiltinVar::Low)
    } else if name.eq_ignore_ascii_case("O") || name.eq_ignore_ascii_case("OPEN") {
        Some(BuiltinVar::Open)
    } else if name.eq_ignore_ascii_case("V")
        || name.eq_ignore_ascii_case("VOL")
        || name.eq_ignore_ascii_case("VOLUME")
    {
        Some(BuiltinVar::Volume)
    } else if name.eq_ignore_ascii_case("AMOUNT") {
        Some(BuiltinVar::Amount)
    } else if name.eq_ignore_ascii_case("BARSCOUNT") {
        Some(BuiltinVar::BarsCount)
    } else if name.eq_ignore_ascii_case("BARPOS") {
        Some(BuiltinVar::BarPos)
    } else if name.eq_ignore_ascii_case("CAPITAL") {
        Some(BuiltinVar::Capital)
    } else if name.eq_ignore_ascii_case("DRAWNULL") {
        Some(BuiltinVar::DrawNull)
    } else {
        None
    }
}

/// 变量名缓存，用于避免热路径中重复创建 Arc<str>
#[derive(Debug, Default)]
pub struct VarNameCache {
    cache: HashMap<String, VarName>,
}

impl VarNameCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// 获取或创建 Arc<str>，避免重复分配
    pub fn get_or_create(&mut self, name: &str) -> VarName {
        if let Some(arc) = self.cache.get(name) {
            arc.clone()
        } else {
            let arc: VarName = Arc::from(name.to_string());
            self.cache.insert(name.to_string(), arc.clone());
            arc
        }
    }

    /// 预缓存常用变量名
    pub fn pre_cache_common(&mut self) {
        for name in [
            "MA5", "MA10", "MA20", "MA60", "DIF", "DEA", "MACD", "RSI", "K", "D", "J",
        ] {
            self.get_or_create(name);
        }
    }

    /// 清空缓存
    pub fn clear(&mut self) {
        self.cache.clear();
    }

    /// 缓存大小
    pub fn len(&self) -> usize {
        self.cache.len()
    }
}

#[derive(Clone, Debug)]
pub enum FormulaValue {
    Scalar(f64),
    Array(Array1<f64>),
}

/// 零拷贝值类型，返回数组视图而非克隆
#[derive(Debug, Clone, Copy)]
pub enum FormulaValueRef<'a> {
    Scalar(f64),
    Array(ArrayView1<'a, f64>),
}

impl<'a> FormulaValueRef<'a> {
    pub fn to_owned(&self, len: usize) -> Array1<f64> {
        match self {
            FormulaValueRef::Scalar(v) => Array1::from_elem(len, *v),
            FormulaValueRef::Array(a) => a.to_owned(),
        }
    }

    pub fn is_scalar(&self) -> bool {
        matches!(self, FormulaValueRef::Scalar(_))
    }

    pub fn as_scalar(&self) -> Option<f64> {
        match self {
            FormulaValueRef::Scalar(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<ArrayView1<'a, f64>> {
        match self {
            FormulaValueRef::Array(a) => Some(*a),
            _ => None,
        }
    }
}

impl FormulaValue {
    pub fn to_array(&self, len: usize) -> Array1<f64> {
        match self {
            FormulaValue::Scalar(v) => Array1::from_elem(len, *v),
            FormulaValue::Array(a) => a.clone(),
        }
    }

    /// 获取数组视图，避免克隆
    pub fn as_view(&self) -> FormulaValueRef<'_> {
        match self {
            FormulaValue::Scalar(v) => FormulaValueRef::Scalar(*v),
            FormulaValue::Array(a) => FormulaValueRef::Array(a.view()),
        }
    }

    pub fn is_scalar(&self) -> bool {
        matches!(self, FormulaValue::Scalar(_))
    }

    pub fn as_scalar(&self) -> Option<f64> {
        match self {
            FormulaValue::Scalar(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&Array1<f64>> {
        match self {
            FormulaValue::Array(a) => Some(a),
            _ => None,
        }
    }

    pub fn len(&self) -> usize {
        match self {
            FormulaValue::Scalar(_) => 0,
            FormulaValue::Array(a) => a.len(),
        }
    }
}

/// 预警指令（THS ALERT/ALERTONCE）
#[derive(Debug, Clone)]
pub struct AlertCommand {
    pub condition: Array1<f64>,
    pub message: String,
    pub is_once: bool,
    pub triggered_bars: Vec<usize>,
}

impl AlertCommand {
    pub fn new(condition: Array1<f64>, message: String, is_once: bool) -> Self {
        Self {
            condition,
            message,
            is_once,
            triggered_bars: Vec::new(),
        }
    }

    pub fn check_alerts(&mut self) -> Vec<(usize, String)> {
        let mut alerts = Vec::new();
        for i in 0..self.condition.len() {
            if self.condition[i] > 0.0 && !self.condition[i].is_nan() {
                if self.is_once && self.triggered_bars.contains(&i) {
                    continue;
                }
                alerts.push((i, self.message.clone()));
                self.triggered_bars.push(i);
            }
        }
        alerts
    }
}

/// 选股信号结果（THS SMARTSELECT/SELECTCOND）
#[derive(Debug, Clone)]
pub struct SelectionResult {
    pub signals: Array1<f64>,
    pub mode: u8,
    pub selected_bars: Vec<usize>,
}

impl SelectionResult {
    pub fn new(signals: Array1<f64>, mode: u8) -> Self {
        let selected_bars: Vec<usize> = signals
            .iter()
            .enumerate()
            .filter(|(_, &v)| v > 0.0 && !v.is_nan())
            .map(|(i, _)| i)
            .collect();
        Self {
            signals,
            mode,
            selected_bars,
        }
    }
}

pub use crate::error::FormulaError;

/// 多输出结果（支持函数返回多个命名序列，如 MACD 返回 DIF/DEA/MACD）
#[derive(Debug, Clone)]
pub struct MultiOutput {
    pub outputs: HashMap<String, Array1<f64>>,
    pub final_value: Array1<f64>,
}

impl MultiOutput {
    pub fn new(final_value: Array1<f64>) -> Self {
        Self {
            outputs: HashMap::new(),
            final_value,
        }
    }

    pub fn get(&self, name: &str) -> Option<&Array1<f64>> {
        self.outputs.get(name)
    }

    pub fn names(&self) -> Vec<&String> {
        self.outputs.keys().collect()
    }

    pub fn len(&self) -> usize {
        self.outputs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.outputs.is_empty()
    }
}

/// 大盘指数数据
#[derive(Clone, Default)]
pub struct IndexData {
    pub open: Option<Array1<f64>>,
    pub high: Option<Array1<f64>>,
    pub low: Option<Array1<f64>>,
    pub close: Option<Array1<f64>>,
    pub volume: Option<Array1<f64>>,
    pub amount: Option<Array1<f64>>,
}

/// 财务数据（通达信 FINANCE(N) 字段映射）
#[derive(Clone, Default)]
pub struct FinanceData {
    pub fields: HashMap<usize, f64>,
}

/// 筹码分布数据（用于 WINNER/COST/LWINNER 函数）
/// 筹码分布是一个价格-成交量的映射表
#[derive(Clone, Default)]
pub struct ChipData {
    /// 价格区间列表（每个价格点）
    pub price_levels: Vec<f64>,
    /// 对应的筹码成交量占比（累计）
    pub volume_ratios: Vec<f64>,
    /// 总成交量
    pub total_volume: f64,
}

impl ChipData {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_data(price_levels: Vec<f64>, volume_ratios: Vec<f64>, total_volume: f64) -> Self {
        Self {
            price_levels,
            volume_ratios,
            total_volume,
        }
    }

    pub fn winner(&self, price: f64) -> f64 {
        if self.price_levels.is_empty() || self.volume_ratios.is_empty() {
            return f64::NAN;
        }
        let mut cum_ratio = 0.0;
        for (i, &p) in self.price_levels.iter().enumerate() {
            if p <= price {
                cum_ratio = self.volume_ratios[i];
            } else {
                break;
            }
        }
        cum_ratio
    }

    pub fn cost(&self, ratio: f64) -> f64 {
        if self.price_levels.is_empty() || self.volume_ratios.is_empty() {
            return f64::NAN;
        }
        for (i, &r) in self.volume_ratios.iter().enumerate() {
            if r >= ratio {
                return self.price_levels[i];
            }
        }
        self.price_levels.last().copied().unwrap_or(f64::NAN)
    }
}

/// 动态行情数据（用于 DYNAINFO 函数）
#[derive(Clone, Default)]
pub struct DynaInfo {
    pub fields: HashMap<usize, f64>,
}

/// 跨周期数据（如周线、月线）
#[derive(Clone)]
pub struct PeriodData {
    pub open: Array1<f64>,
    pub high: Array1<f64>,
    pub low: Array1<f64>,
    pub close: Array1<f64>,
    pub volume: Array1<f64>,
}

/// 板块数据（大智慧 BLOCKDATA/BLOCKINDEX/BLOCKAVG 支持）
/// 用于引用板块指数、均价等数据
#[derive(Clone, Default)]
pub struct BlockData {
    pub index_close: HashMap<String, Array1<f64>>,
    pub avg_price: HashMap<String, Array1<f64>>,
    pub pct_change: HashMap<String, Array1<f64>>,
    pub volume: HashMap<String, Array1<f64>>,
    pub amount: HashMap<String, Array1<f64>>,
    pub leader_stock: HashMap<String, String>,
    pub custom_fields: HashMap<String, HashMap<String, Array1<f64>>>,
}

/// 东方财富数据（EM DKCOL/EM_REF/EM_ZLCCV 支持）
#[derive(Clone, Default)]
pub struct EmData {
    pub dkcol_data: HashMap<String, Array1<f64>>,
    pub external_data: HashMap<String, Array1<f64>>,
}

/// 资金流向数据（大智慧 MONEYFLOW/NETINFLOW/BIGORDER/SMALLORDER 支持）
#[derive(Clone)]
pub struct MoneyFlowData {
    /// 主力净流入序列 (万元)
    pub main_inflow: Array1<f64>,
    /// 超大单净流入序列 (万元)
    pub super_big_inflow: Array1<f64>,
    /// 大单净流入序列 (万元)
    pub big_inflow: Array1<f64>,
    /// 中单净流入序列 (万元)
    pub medium_inflow: Array1<f64>,
    /// 小单净流入序列 (万元)
    pub small_inflow: Array1<f64>,
    /// 主力净流入占比序列 (%)
    pub main_inflow_pct: Array1<f64>,
    /// 大单成交占比序列 (%)
    pub big_order_pct: Array1<f64>,
    /// 小单成交占比序列 (%)
    pub small_order_pct: Array1<f64>,
    /// 资金流向总序列 (万元)
    pub money_flow: Array1<f64>,
}

impl Default for MoneyFlowData {
    fn default() -> Self {
        Self {
            main_inflow: Array1::zeros(0),
            super_big_inflow: Array1::zeros(0),
            big_inflow: Array1::zeros(0),
            medium_inflow: Array1::zeros(0),
            small_inflow: Array1::zeros(0),
            main_inflow_pct: Array1::zeros(0),
            big_order_pct: Array1::zeros(0),
            small_order_pct: Array1::zeros(0),
            money_flow: Array1::zeros(0),
        }
    }
}

/// A one-dimensional f64 input that can either own its storage or borrow a
/// caller-owned contiguous buffer for the duration of a synchronous evaluation.
///
/// The borrowed constructor is crate-private so the raw pointer cannot escape
/// the engine call that established the NumPy borrow. Cloning a borrowed series
/// materializes an owned copy, which keeps existing parallel/context-clone
/// semantics safe.
#[derive(Debug)]
pub struct FormulaSeries {
    owned: Option<Vec<f64>>,
    borrowed_ptr: usize,
    len: usize,
}

impl FormulaSeries {
    pub fn from_vec(values: Vec<f64>) -> Self {
        let len = values.len();
        Self {
            owned: Some(values),
            borrowed_ptr: 0,
            len,
        }
    }

    pub(crate) fn from_slice(values: &[f64]) -> Self {
        Self {
            owned: None,
            borrowed_ptr: values.as_ptr() as usize,
            len: values.len(),
        }
    }

    pub fn as_slice(&self) -> &[f64] {
        if let Some(values) = &self.owned {
            values.as_slice()
        } else {
            // Safety: from_slice is crate-private and its caller keeps the
            // source slice alive for the complete synchronous evaluation.
            unsafe { std::slice::from_raw_parts(self.borrowed_ptr as *const f64, self.len) }
        }
    }

    pub fn as_ptr(&self) -> *const f64 {
        self.as_slice().as_ptr()
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn reserve(&mut self, additional: usize) {
        self.make_owned();
        self.owned
            .as_mut()
            .expect("FormulaSeries must be owned after make_owned")
            .reserve(additional);
        self.len = self.owned.as_ref().map_or(0, Vec::len);
    }

    pub fn push(&mut self, value: f64) {
        self.make_owned();
        self.owned
            .as_mut()
            .expect("FormulaSeries must be owned after make_owned")
            .push(value);
        self.len = self.owned.as_ref().map_or(0, Vec::len);
    }

    fn make_owned(&mut self) {
        if self.owned.is_none() {
            self.owned = Some(self.as_slice().to_vec());
            self.borrowed_ptr = 0;
        }
    }
}

impl Clone for FormulaSeries {
    fn clone(&self) -> Self {
        Self::from_vec(self.as_slice().to_vec())
    }
}

impl Deref for FormulaSeries {
    type Target = [f64];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl DerefMut for FormulaSeries {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.make_owned();
        self.owned
            .as_mut()
            .expect("FormulaSeries must be owned after make_owned")
            .as_mut_slice()
    }
}

/// 公式上下文，存储数据绑定和变量
pub struct FormulaContext {
    /// OHLCV数据
    pub open: FormulaSeries,
    pub high: FormulaSeries,
    pub low: FormulaSeries,
    pub close: FormulaSeries,
    pub volume: FormulaSeries,
    pub amount: Option<Array1<f64>>,
    /// 时间序列（Unix timestamp per bar, seconds）
    pub datetime: Option<Array1<i64>>,
    /// 大盘指数数据
    pub index_data: Option<IndexData>,
    /// 财务数据
    pub finance_data: Option<FinanceData>,
    /// 筹码分布数据
    pub chip_data: Option<ChipData>,
    /// 动态行情数据
    pub dynainfo: Option<DynaInfo>,
    /// 流通股本（CAPITAL）
    pub capital: Option<f64>,
    /// 板块数据（大智慧扩展）
    pub block_data: Option<BlockData>,
    /// 资金流向数据（大智慧扩展）
    pub money_flow_data: Option<MoneyFlowData>,
    /// 东方财富数据（EM扩展）
    pub em_data: Option<EmData>,
    /// 字符串映射表（用于函数参数传递，索引 -> 字符串）
    pub string_table: Vec<String>,
    /// 变量存储（使用 Arc<str> 避免字符串克隆）
    pub variables: HashMap<VarName, Array1<f64>>,
    /// 输出修饰符
    pub output_modifiers: HashMap<String, OutputModifier>,
    /// 绘图命令
    pub draw_commands: RefCell<DrawResult>,
    /// 数据长度
    pub data_len: usize,
    /// 多周期数据源 (e.g. "WEEK" -> OHLCV, "MONTH" -> OHLCV)
    pub period_data: HashMap<String, PeriodData>,
    /// 当前周期类型 (0=日线, 1=周线, 2=月线, 3=分钟线)
    pub period_type: u8,
    /// 执行沙箱限制（超时 / 递归深度 / 内存）
    pub sandbox: ExecSandboxConfig,
    /// 沙箱运行时状态（每次顶层 eval 前重置）
    pub(crate) sandbox_state: std::cell::RefCell<ExecSandboxState>,
}

impl Clone for FormulaContext {
    fn clone(&self) -> Self {
        Self {
            open: self.open.clone(),
            high: self.high.clone(),
            low: self.low.clone(),
            close: self.close.clone(),
            volume: self.volume.clone(),
            amount: self.amount.clone(),
            datetime: self.datetime.clone(),
            index_data: self.index_data.clone(),
            finance_data: self.finance_data.clone(),
            chip_data: self.chip_data.clone(),
            dynainfo: self.dynainfo.clone(),
            capital: self.capital,
            block_data: self.block_data.clone(),
            money_flow_data: self.money_flow_data.clone(),
            em_data: self.em_data.clone(),
            string_table: self.string_table.clone(),
            variables: self.variables.clone(),
            output_modifiers: self.output_modifiers.clone(),
            draw_commands: RefCell::new(self.draw_commands.borrow().clone()),
            data_len: self.data_len,
            period_data: self.period_data.clone(),
            period_type: self.period_type,
            sandbox: self.sandbox,
            sandbox_state: std::cell::RefCell::new(self.sandbox_state.borrow().clone()),
        }
    }
}

impl FormulaContext {
    /// 创建新的公式上下文
    pub fn new(
        open: Array1<f64>,
        high: Array1<f64>,
        low: Array1<f64>,
        close: Array1<f64>,
        volume: Array1<f64>,
        amount: Option<Array1<f64>>,
    ) -> Self {
        let data_len = open.len();
        Self {
            open: FormulaSeries::from_vec(open.into_raw_vec()),
            high: FormulaSeries::from_vec(high.into_raw_vec()),
            low: FormulaSeries::from_vec(low.into_raw_vec()),
            close: FormulaSeries::from_vec(close.into_raw_vec()),
            volume: FormulaSeries::from_vec(volume.into_raw_vec()),
            amount,
            datetime: None,
            index_data: None,
            finance_data: None,
            chip_data: None,
            dynainfo: None,
            capital: None,
            block_data: None,
            money_flow_data: None,
            em_data: None,
            string_table: Vec::new(),
            variables: HashMap::new(),
            output_modifiers: HashMap::new(),
            draw_commands: RefCell::new(DrawResult::new()),
            data_len,
            period_data: HashMap::new(),
            period_type: 0,
            sandbox: ExecSandboxConfig::default(),
            sandbox_state: std::cell::RefCell::new(ExecSandboxState::default()),
        }
    }

    /// Create a context that borrows contiguous OHLCV slices.
    ///
    /// This is used by the synchronous Python/NumPy zero-copy path. The
    /// returned context must not outlive the borrowed slices; the constructor
    /// is crate-private to keep that lifetime invariant inside the engine.
    pub(crate) fn from_borrowed_ohlcv(
        open: &[f64],
        high: &[f64],
        low: &[f64],
        close: &[f64],
        volume: &[f64],
        amount: Option<Array1<f64>>,
    ) -> Self {
        let data_len = close.len();
        Self {
            open: FormulaSeries::from_slice(open),
            high: FormulaSeries::from_slice(high),
            low: FormulaSeries::from_slice(low),
            close: FormulaSeries::from_slice(close),
            volume: FormulaSeries::from_slice(volume),
            amount,
            datetime: None,
            index_data: None,
            finance_data: None,
            chip_data: None,
            dynainfo: None,
            capital: None,
            block_data: None,
            money_flow_data: None,
            em_data: None,
            string_table: Vec::new(),
            variables: HashMap::new(),
            output_modifiers: HashMap::new(),
            draw_commands: RefCell::new(DrawResult::new()),
            data_len,
            period_data: HashMap::new(),
            period_type: 0,
            sandbox: ExecSandboxConfig::default(),
            sandbox_state: std::cell::RefCell::new(ExecSandboxState::default()),
        }
    }

    /// Reset sandbox runtime counters before a new top-level evaluation.
    pub fn reset_sandbox(&mut self) {
        sandbox_reset(&self.sandbox_state);
    }

    /// 创建带时间数据的公式上下文
    pub fn with_datetime(mut self, datetime: Array1<i64>) -> Self {
        self.datetime = Some(datetime);
        self
    }

    /// 设置大盘数据
    pub fn with_index_data(mut self, index_data: IndexData) -> Self {
        self.index_data = Some(index_data);
        self
    }

    /// 设置财务数据
    pub fn with_finance_data(mut self, finance_data: FinanceData) -> Self {
        self.finance_data = Some(finance_data);
        self
    }

    /// 设置筹码分布数据
    pub fn with_chip_data(mut self, chip_data: ChipData) -> Self {
        self.chip_data = Some(chip_data);
        self
    }

    /// 设置动态行情数据
    pub fn with_dynainfo(mut self, dynainfo: DynaInfo) -> Self {
        self.dynainfo = Some(dynainfo);
        self
    }

    /// 设置流通股本
    pub fn with_capital(mut self, capital: f64) -> Self {
        self.capital = Some(capital);
        self
    }

    /// 设置板块数据（大智慧扩展）
    pub fn with_block_data(mut self, block_data: BlockData) -> Self {
        self.block_data = Some(block_data);
        self
    }

    /// 设置资金流向数据（大智慧扩展）
    pub fn with_money_flow_data(mut self, money_flow_data: MoneyFlowData) -> Self {
        self.money_flow_data = Some(money_flow_data);
        self
    }

    /// 设置东方财富数据（EM扩展）
    pub fn with_em_data(mut self, em_data: EmData) -> Self {
        self.em_data = Some(em_data);
        self
    }

    /// 设置周期数据（如 "WEEK", "MONTH"）
    pub fn with_period_data(mut self, period: &str, data: PeriodData) -> Self {
        self.period_data.insert(period.to_uppercase(), data);
        self
    }

    /// 设置当前周期类型
    pub fn with_period_type(mut self, period_type: u8) -> Self {
        self.period_type = period_type;
        self
    }

    /// Reserve capacity for streaming bars before appending.
    pub fn reserve_bars(&mut self, additional: usize) {
        self.open.reserve(additional);
        self.high.reserve(additional);
        self.low.reserve(additional);
        self.close.reserve(additional);
        self.volume.reserve(additional);
    }

    /// Append a new bar in amortized O(1) time.
    ///
    /// OHLCV is backed by Vec, so appending no longer concatenates the entire
    /// history on every call. Call reserve_bars first for predictable capacity.
    pub fn append_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) {
        self.open.push(open);
        self.high.push(high);
        self.low.push(low);
        self.close.push(close);
        self.volume.push(volume);
        self.data_len = self.close.len();
    }

    /// 获取数据数组
    pub fn get_data(&self, name: &str) -> Option<&[f64]> {
        match classify_builtin_var(name) {
            Some(BuiltinVar::Open) => Some(&self.open),
            Some(BuiltinVar::High) => Some(&self.high),
            Some(BuiltinVar::Low) => Some(&self.low),
            Some(BuiltinVar::Close) => Some(&self.close),
            Some(BuiltinVar::Volume) => Some(&self.volume),
            Some(BuiltinVar::Amount) => self.amount.as_ref().and_then(|value| value.as_slice().ok()),
            _ => {
                if name.eq_ignore_ascii_case("A") {
                    self.amount.as_ref().and_then(|value| value.as_slice().ok())
                } else {
                    self.variables.get(name).and_then(|value| value.as_slice().ok())
                }
            }
        }
    }

    /// 设置变量（零拷贝路径）
    pub fn set_variable(&mut self, name: String, value: Array1<f64>) {
        self.variables.insert(Arc::from(name), value);
    }

    /// 设置变量（使用 Arc<str> 键，避免字符串克隆）
    pub fn set_variable_arc(&mut self, name: VarName, value: Array1<f64>) {
        self.variables.insert(name, value);
    }

    /// 获取变量（返回引用，避免克隆）
    pub fn get_variable(&self, name: &str) -> Option<&Array1<f64>> {
        self.variables.get(name)
    }

    /// 获取变量（使用 Arc<str> 键）
    pub fn get_variable_arc(&self, name: &VarName) -> Option<&Array1<f64>> {
        self.variables.get(name)
    }

    pub fn close_view(&self) -> ArrayView1<'_, f64> {
        ArrayView1::from(self.close.as_slice())
    }

    pub fn open_view(&self) -> ArrayView1<'_, f64> {
        ArrayView1::from(self.open.as_slice())
    }

    pub fn high_view(&self) -> ArrayView1<'_, f64> {
        ArrayView1::from(self.high.as_slice())
    }

    pub fn low_view(&self) -> ArrayView1<'_, f64> {
        ArrayView1::from(self.low.as_slice())
    }

    pub fn volume_view(&self) -> ArrayView1<'_, f64> {
        ArrayView1::from(self.volume.as_slice())
    }

    /// Create a context containing only the requested half-open bar range.
    pub fn window(&self, start: usize, end: usize) -> Result<Self, FormulaError> {
        if start > end || end > self.data_len {
            return Err(FormulaError::InvalidParameter(format!(
                "invalid formula window [{start}, {end}) for data_len {}",
                self.data_len
            )));
        }
        let mut result = self.clone();
        result.open = FormulaSeries::from_vec(self.open[start..end].to_vec());
        result.high = FormulaSeries::from_vec(self.high[start..end].to_vec());
        result.low = FormulaSeries::from_vec(self.low[start..end].to_vec());
        result.close = FormulaSeries::from_vec(self.close[start..end].to_vec());
        result.volume = FormulaSeries::from_vec(self.volume[start..end].to_vec());
        result.amount = self.amount.as_ref().map(|a| a.slice(ndarray::s![start..end]).to_owned());
        result.datetime = self.datetime.as_ref().map(|a| a.slice(ndarray::s![start..end]).to_owned());
        result.variables.clear();
        result.data_len = end - start;
        Ok(result)
    }

    pub fn copy_array(arr: &Array1<f64>) -> Array1<f64> {
        arr.view().to_owned()
    }

    pub fn assign_var(&mut self, name: &str, value: Array1<f64>) -> Array1<f64> {
        let copy = Self::copy_array(&value);
        self.variables.insert(Arc::from(name.to_string()), copy);
        value
    }

    /// 零拷贝赋值变量（避免字符串克隆）
    pub fn assign_var_arc(&mut self, name: VarName, value: Array1<f64>) -> Array1<f64> {
        let copy = Self::copy_array(&value);
        self.variables.insert(name, copy);
        value
    }

    /// 零拷贝赋值变量（直接存储，不复制）
    pub fn assign_var_no_copy(&mut self, name: VarName, value: Array1<f64>) {
        self.variables.insert(name, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ctx(len: usize) -> FormulaContext {
        let open = Array1::from_vec((0..len).map(|i| 10.0 + i as f64 * 0.1).collect());
        let high = Array1::from_vec((0..len).map(|i| 11.0 + i as f64 * 0.2).collect());
        let low = Array1::from_vec((0..len).map(|i| 9.0 + i as f64 * 0.1).collect());
        let close = Array1::from_vec((0..len).map(|i| 10.0 + i as f64 * 0.15).collect());
        let volume = Array1::from_vec((0..len).map(|i| 1000.0 + i as f64 * 10.0).collect());
        FormulaContext::new(open, high, low, close, volume, None)
    }

    #[test]
    fn test_close_view_returns_correct_data() {
        let ctx = make_ctx(5);
        let view = ctx.close_view();
        for i in 0..5 {
            let expected = 10.0 + i as f64 * 0.15;
            assert!((view[i] - expected).abs() < 1e-10);
        }
    }

    #[test]
    fn test_open_view_returns_correct_data() {
        let ctx = make_ctx(5);
        let view = ctx.open_view();
        for i in 0..5 {
            let expected = 10.0 + i as f64 * 0.1;
            assert!((view[i] - expected).abs() < 1e-10);
        }
    }

    #[test]
    fn test_high_view_returns_correct_data() {
        let ctx = make_ctx(5);
        let view = ctx.high_view();
        for i in 0..5 {
            let expected = 11.0 + i as f64 * 0.2;
            assert!((view[i] - expected).abs() < 1e-10);
        }
    }

    #[test]
    fn test_low_view_returns_correct_data() {
        let ctx = make_ctx(5);
        let view = ctx.low_view();
        for i in 0..5 {
            let expected = 9.0 + i as f64 * 0.1;
            assert!((view[i] - expected).abs() < 1e-10);
        }
    }

    #[test]
    fn test_volume_view_returns_correct_data() {
        let ctx = make_ctx(5);
        let view = ctx.volume_view();
        for i in 0..5 {
            let expected = 1000.0 + i as f64 * 10.0;
            assert!((view[i] - expected).abs() < 1e-10);
        }
    }

    #[test]
    fn test_views_are_zero_copy() {
        let ctx = make_ctx(5);
        assert_eq!(ctx.close_view().as_ptr(), ctx.close.as_ptr());
        assert_eq!(ctx.open_view().as_ptr(), ctx.open.as_ptr());
        assert_eq!(ctx.high_view().as_ptr(), ctx.high.as_ptr());
        assert_eq!(ctx.low_view().as_ptr(), ctx.low.as_ptr());
        assert_eq!(ctx.volume_view().as_ptr(), ctx.volume.as_ptr());
    }

    #[test]
    fn test_var_name_cache_new() {
        let cache = VarNameCache::new();
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_var_name_cache_get_or_create() {
        let mut cache = VarNameCache::new();
        let name1 = cache.get_or_create("MA5");
        let name2 = cache.get_or_create("MA5");
        assert_eq!(name1, name2);
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_var_name_cache_different_names() {
        let mut cache = VarNameCache::new();
        let name1 = cache.get_or_create("MA5");
        let name2 = cache.get_or_create("MA10");
        assert_ne!(name1, name2);
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn test_var_name_cache_pre_cache_common() {
        let mut cache = VarNameCache::new();
        cache.pre_cache_common();
        assert!(cache.len() >= 11);
        assert!(cache.get_or_create("MA5") == cache.get_or_create("MA5"));
        assert!(cache.get_or_create("DIF") == cache.get_or_create("DIF"));
    }

    #[test]
    fn test_var_name_cache_clear() {
        let mut cache = VarNameCache::new();
        cache.get_or_create("MA5");
        cache.get_or_create("MA10");
        assert_eq!(cache.len(), 2);
        cache.clear();
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_formula_value_ref_scalar() {
        let val = FormulaValue::Scalar(42.0);
        let ref_val = val.as_view();
        assert!(ref_val.is_scalar());
        assert_eq!(ref_val.as_scalar(), Some(42.0));
    }

    #[test]
    fn test_formula_value_ref_array() {
        let arr = Array1::from_vec(vec![1.0, 2.0, 3.0]);
        let val = FormulaValue::Array(arr);
        let ref_val = val.as_view();
        assert!(!ref_val.is_scalar());
        let view = ref_val.as_array().unwrap();
        assert_eq!(view.len(), 3);
        assert_eq!(view[0], 1.0);
    }

    #[test]
    fn test_formula_value_ref_to_owned_scalar() {
        let val = FormulaValue::Scalar(42.0);
        let ref_val = val.as_view();
        let owned = ref_val.to_owned(5);
        assert_eq!(owned.len(), 5);
        for i in 0..5 {
            assert_eq!(owned[i], 42.0);
        }
    }

    #[test]
    fn test_formula_value_ref_to_owned_array() {
        let arr = Array1::from_vec(vec![1.0, 2.0, 3.0]);
        let val = FormulaValue::Array(arr.clone());
        let ref_val = val.as_view();
        let owned = ref_val.to_owned(3);
        assert_eq!(owned.len(), 3);
        for i in 0..3 {
            assert_eq!(owned[i], arr[i]);
        }
    }

    #[test]
    fn test_set_variable_arc() {
        let mut ctx = make_ctx(5);
        let name: VarName = Arc::from("TEST_VAR");
        let value = Array1::from_vec(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
        ctx.set_variable_arc(name.clone(), value.clone());
        assert!(ctx.variables.contains_key(&name));
        let retrieved = ctx.get_variable_arc(&name).unwrap();
        for i in 0..5 {
            assert_eq!(retrieved[i], value[i]);
        }
    }

    #[test]
    fn test_assign_var_no_copy() {
        let mut ctx = make_ctx(5);
        let name: VarName = Arc::from("TEST_VAR");
        let value = Array1::from_vec(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
        ctx.assign_var_no_copy(name.clone(), value);
        assert!(ctx.variables.contains_key(&name));
        let retrieved = ctx.get_variable_arc(&name).unwrap();
        assert_eq!(retrieved.len(), 5);
    }
    #[test]
    fn test_borrowed_series_keeps_input_pointer_and_clones_safely() {
        let values = vec![1.0, 2.0, 3.0];
        let borrowed = FormulaSeries::from_slice(&values);
        assert_eq!(borrowed.as_ptr(), values.as_ptr());
        assert_eq!(borrowed.as_slice(), values.as_slice());

        let cloned = borrowed.clone();
        assert_ne!(cloned.as_ptr(), values.as_ptr());
        assert_eq!(cloned.as_slice(), values.as_slice());
    }

    #[test]
    fn test_append_bar_is_amortized_and_keeps_data() {
        let mut ctx = make_ctx(2);
        ctx.reserve_bars(128);
        let open_ptr = ctx.open.as_ptr();
        ctx.append_bar(12.0, 13.0, 11.0, 12.5, 2000.0);
        ctx.append_bar(12.5, 13.5, 11.5, 13.0, 2100.0);
        assert_eq!(ctx.data_len, 4);
        assert_eq!(ctx.close.as_slice(), &[10.0, 10.15, 12.5, 13.0]);
        assert_eq!(ctx.volume.as_slice(), &[1000.0, 1010.0, 2000.0, 2100.0]);
        assert_eq!(ctx.open.as_ptr(), open_ptr);
    }

}
