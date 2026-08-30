#![allow(missing_docs)]

mod csv_io;

use clap::{Parser, Subcommand, ValueEnum};
use csv_io::{read_close_input, read_ohlcv_input};
use alpha_ta_core::indicators;
use alpha_ta_core::math::moving_avg;
use alpha_ta_core::patterns::{candlestick, chart};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "finkit")]
#[command(about = "Finkit financial computation CLI — indicators, formulas, streaming, and features")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Clone, ValueEnum)]
enum OutputFormat {
    Plain,
    Json,
    Csv,
}

#[derive(Subcommand)]
enum Commands {
    /// Simple Moving Average
    Sma {
        #[arg(short, long)]
        input: Option<String>,
        #[arg(short, long, default_value_t = 14)]
        period: usize,
        #[arg(short, long)]
        output: Option<String>,
        #[arg(short, long, value_enum, default_value_t = OutputFormat::Plain)]
        format: OutputFormat,
    },
    /// Exponential Moving Average
    Ema {
        #[arg(short, long)]
        input: Option<String>,
        #[arg(short, long, default_value_t = 14)]
        period: usize,
        #[arg(short, long)]
        output: Option<String>,
        #[arg(short, long, value_enum, default_value_t = OutputFormat::Plain)]
        format: OutputFormat,
    },
    /// Relative Strength Index
    Rsi {
        #[arg(short, long)]
        input: Option<String>,
        #[arg(short, long, default_value_t = 14)]
        period: usize,
        #[arg(short, long)]
        output: Option<String>,
        #[arg(short, long, value_enum, default_value_t = OutputFormat::Plain)]
        format: OutputFormat,
    },
    /// Moving Average Convergence Divergence
    Macd {
        #[arg(short, long)]
        input: Option<String>,
        #[arg(long, default_value_t = 12)]
        fast: usize,
        #[arg(long, default_value_t = 26)]
        slow: usize,
        #[arg(long, default_value_t = 9)]
        signal: usize,
        #[arg(short, long)]
        output: Option<String>,
        #[arg(short, long, value_enum, default_value_t = OutputFormat::Plain)]
        format: OutputFormat,
    },
    /// Bollinger Bands
    Bbands {
        /// 输入 CSV 文件路径（含 open,high,low,close 列）
        #[arg(short, long)]
        input: PathBuf,
        /// 周期
        #[arg(short, long, default_value_t = 20)]
        period: usize,
        /// 标准差倍数（同时作用于 upper/lower）
        #[arg(long, default_value_t = 2.0)]
        stddev: f64,
        /// 上轨标准差倍数（与 --stddev 二选一）
        #[arg(long, default_value_t = 2.0)]
        nbdevup: f64,
        /// 下轨标准差倍数（与 --stddev 二选一）
        #[arg(long, default_value_t = 2.0)]
        nbdevdn: f64,
        #[arg(short, long)]
        output: Option<String>,
        #[arg(short, long, value_enum, default_value_t = OutputFormat::Json)]
        format: OutputFormat,
    },
    /// Average True Range (requires OHLCV input)
    Atr {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long, default_value_t = 14)]
        period: usize,
        #[arg(short, long)]
        output: Option<String>,
        #[arg(short, long, value_enum, default_value_t = OutputFormat::Json)]
        format: OutputFormat,
    },
    /// Stochastic Oscillator (requires OHLCV input)
    Stoch {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(long, default_value_t = 14)]
        fastk_period: usize,
        #[arg(long, default_value_t = 3)]
        slowk_period: usize,
        #[arg(long, default_value_t = 3)]
        slowd_period: usize,
        #[arg(short, long)]
        output: Option<String>,
        #[arg(short, long, value_enum, default_value_t = OutputFormat::Plain)]
        format: OutputFormat,
    },
    /// Average Directional Index (requires OHLCV input)
    Adx {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long, default_value_t = 14)]
        period: usize,
        #[arg(short, long)]
        output: Option<String>,
        #[arg(short, long, value_enum, default_value_t = OutputFormat::Plain)]
        format: OutputFormat,
    },
    /// Commodity Channel Index (requires OHLCV input)
    Cci {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long, default_value_t = 14)]
        period: usize,
        #[arg(short, long)]
        output: Option<String>,
        #[arg(short, long, value_enum, default_value_t = OutputFormat::Plain)]
        format: OutputFormat,
    },
    /// On Balance Volume (requires OHLCV input)
    Obv {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long)]
        output: Option<String>,
        #[arg(short, long, value_enum, default_value_t = OutputFormat::Plain)]
        format: OutputFormat,
    },
    /// Williams %R (requires OHLCV input)
    Willr {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long, default_value_t = 14)]
        period: usize,
        #[arg(short, long)]
        output: Option<String>,
        #[arg(short, long, value_enum, default_value_t = OutputFormat::Plain)]
        format: OutputFormat,
    },
    /// Weighted Moving Average
    Wma {
        #[arg(short, long)]
        input: Option<String>,
        #[arg(short, long, default_value_t = 14)]
        period: usize,
        #[arg(short, long)]
        output: Option<String>,
        #[arg(short, long, value_enum, default_value_t = OutputFormat::Plain)]
        format: OutputFormat,
    },
    /// Detect candlestick or chart patterns (requires OHLCV input)
    Pattern {
        #[arg(short, long)]
        input: PathBuf,
        /// Pattern type: candlestick or chart
        #[arg(long, value_enum)]
        kind: PatternKind,
        /// Specific pattern name (e.g., doji, hammer, engulfing, head_shoulders)
        #[arg(long)]
        name: Option<String>,
        #[arg(short, long, value_enum, default_value_t = OutputFormat::Json)]
        format: OutputFormat,
    },
    /// Execute a TongDaXin-compatible formula (requires OHLCV input)
    ///
    /// Usage: finkit formula "MA(CLOSE, 5)" --input data.csv
    Formula {
        /// 公式字符串，例如 `MA(CLOSE, 5)` 或 `MA(C, 5)`
        formula: Option<String>,
        /// 输入 CSV 路径（OHLCV 必填）
        #[arg(short, long)]
        input: PathBuf,
        /// 公式表达式（与位置参数互斥）
        #[arg(long, conflicts_with = "formula")]
        expr: Option<String>,
        /// 公式方言：alpha_ta（通达信，默认）或 pine（Pine Script v5）
        #[arg(long, default_value = "alpha_ta")]
        dialect: String,
        #[arg(short, long, value_enum, default_value_t = OutputFormat::Plain)]
        format: OutputFormat,
    },
    /// Streaming/incremental indicator computation (O(1) per bar)
    ///
    /// Usage: finkit streaming sma --input data.csv --period 14
    Streaming {
        /// Streaming indicator name: sma, ema, rsi, atr, macd, boll, vwap, obv, adx, stoch, supertrend, ...
        indicator: String,
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long, default_value_t = 14)]
        period: usize,
        /// Optional fast period (macd/ppo/apo)
        #[arg(long, default_value_t = 12)]
        fast_period: usize,
        /// Optional slow period (macd/ppo/apo)
        #[arg(long, default_value_t = 26)]
        slow_period: usize,
        /// Optional signal period (macd)
        #[arg(long, default_value_t = 9)]
        signal_period: usize,
        /// Standard deviation multiplier (bollinger bands)
        #[arg(long, default_value_t = 2.0)]
        nb_dev: f64,
        #[arg(short, long)]
        output: Option<String>,
        #[arg(short, long, value_enum, default_value_t = OutputFormat::Plain)]
        format: OutputFormat,
    },
    /// Apply data transformations (log-return, z-score, scaling, etc.)
    ///
    /// Usage: finkit transform log_return --input data.csv
    Transform {
        /// Transform name: log_return, pct_change, zscore, standard_scaler, minmax_scaler, rank, diff
        transform: String,
        #[arg(short, long)]
        input: Option<String>,
        /// Period (for rolling transforms)
        #[arg(short, long, default_value_t = 14)]
        period: usize,
        #[arg(short, long)]
        output: Option<String>,
        #[arg(short, long, value_enum, default_value_t = OutputFormat::Plain)]
        format: OutputFormat,
    },
    /// Feature engineering pipeline (alpha factors, cross features, etc.)
    ///
    /// Usage: finkit features alpha_pack --input data.csv
    Features {
        /// Feature pack name: alpha_pack (default pack of alpha factors)
        pack: String,
        #[arg(short, long)]
        input: PathBuf,
        /// Period for the underlying indicators
        #[arg(short, long, default_value_t = 14)]
        period: usize,
        #[arg(short, long)]
        output: Option<String>,
        #[arg(short, long, value_enum, default_value_t = OutputFormat::Csv)]
        format: OutputFormat,
    },
    /// Parameter sweep over an indicator
    ///
    /// Usage: finkit sweep sma --input data.csv --period-min 5 --period-max 50
    Sweep {
        /// Indicator to sweep: sma, ema, rsi, atr, wma
        indicator: String,
        #[arg(short, long)]
        input: Option<String>,
        #[arg(long, default_value_t = 5)]
        period_min: usize,
        #[arg(long, default_value_t = 50)]
        period_max: usize,
        #[arg(long, default_value_t = 1)]
        period_step: usize,
        /// Metric to compute per period: mean, std, min, max, last, slope
        #[arg(long, default_value = "mean")]
        metric: String,
        #[arg(short, long)]
        output: Option<String>,
        #[arg(short, long, value_enum, default_value_t = OutputFormat::Csv)]
        format: OutputFormat,
    },
    /// Generate a chart from OHLCV data (SVG/HTML/JSON)
    ///
    /// Usage: finkit chart --input data.csv --format svg --output chart.svg
    Chart {
        #[arg(short, long)]
        input: PathBuf,
        /// Output format: svg, html, json
        #[arg(long, default_value = "svg")]
        chart_format: String,
        /// Chart title
        #[arg(long)]
        title: Option<String>,
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Generic indicator calculator
    ///
    /// Usage: finkit calc SMA --period 20 --input data.csv
    Calc {
        /// Indicator name: SMA, EMA, RSI, MACD, ATR, BBANDS, ADX, CCI, OBV, WILLR, WMA, STOCH
        indicator: String,
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long, default_value_t = 14)]
        period: usize,
        #[arg(long, default_value_t = 12)]
        fast: usize,
        #[arg(long, default_value_t = 26)]
        slow: usize,
        #[arg(long, default_value_t = 9)]
        signal: usize,
        #[arg(long, default_value_t = 2.0)]
        stddev: f64,
        #[arg(short, long)]
        output: Option<String>,
        #[arg(short, long, value_enum, default_value_t = OutputFormat::Plain)]
        format: OutputFormat,
    },
    /// Browse, search, and render formula templates
    ///
    /// Usage: finkit template list | search macd | render <name> --input data.csv
    Template {
        /// Action: list, search, render, info
        action: String,
        /// Template name or keyword (for search/info/render)
        name: Option<String>,
        #[arg(short, long)]
        input: Option<PathBuf>,
        #[arg(short, long)]
        output: Option<String>,
        #[arg(short, long, value_enum, default_value_t = OutputFormat::Plain)]
        format: OutputFormat,
    },
}

#[derive(Clone, ValueEnum)]
enum PatternKind {
    Candlestick,
    Chart,
}

/// Backward-compatible close-input reader.
/// Supports both a file path and stdin (when `path` is `None`).
fn read_close_input_legacy(path: &Option<String>) -> io::Result<Vec<f64>> {
    read_close_input(path.as_deref())
}

fn output_single(name: &str, data: &[f64], format: &OutputFormat, output: &Option<String>) {
    let text = match format {
        OutputFormat::Plain => data.iter().map(|v| format!("{v}")).collect::<Vec<_>>().join("\n"),
        OutputFormat::Csv => {
            let mut out = format!("{name}\n");
            for v in data {
                out.push_str(&format!("{v}\n"));
            }
            out
        }
        OutputFormat::Json => {
            let items: Vec<String> = data
                .iter()
                .map(|v| if v.is_nan() { "null".to_string() } else { format!("{v}") })
                .collect();
            format!("{{\"{name}\":[{}]}}", items.join(","))
        }
    };

    if let Some(path) = output {
        fs::write(path, &text).expect("Failed to write output");
    } else {
        println!("{text}");
    }
}

fn output_multi(names: &[&str], columns: &[&[f64]], format: &OutputFormat, output: &Option<String>) {
    let text = match format {
        OutputFormat::Plain => {
            let mut out = String::new();
            for (i, name) in names.iter().enumerate() {
                if i > 0 { out.push_str("\n\n"); }
                out.push_str(&format!("{name}:\n"));
                for v in columns[i] {
                    out.push_str(&format!("{v}\n"));
                }
            }
            out
        }
        OutputFormat::Csv => {
            let mut out = names.join(",");
            out.push('\n');
            let len = columns[0].len();
            for row in 0..len {
                let vals: Vec<String> = columns.iter().map(|col| format!("{}", col[row])).collect();
                out.push_str(&vals.join(","));
                out.push('\n');
            }
            out
        }
        OutputFormat::Json => {
            let mut parts = Vec::new();
            for (i, name) in names.iter().enumerate() {
                let items: Vec<String> = columns[i]
                    .iter()
                    .map(|v| if v.is_nan() { "null".to_string() } else { format!("{v}") })
                    .collect();
                parts.push(format!("\"{name}\":[{}]", items.join(",")));
            }
            format!("{{{}}}", parts.join(","))
        }
    };

    if let Some(path) = output {
        fs::write(path, &text).expect("Failed to write output");
    } else {
        println!("{text}");
    }
}

/// Resolve `--stddev` vs legacy `--nbdevup/--nbdevdn`.
/// If the user explicitly customised nbdevup/nbdevdn from defaults, prefer them.
fn resolve_bbands_stddev(stddev: f64, nbdevup: f64, nbdevdn: f64) -> (f64, f64) {
    let up_default = (nbdevup - 2.0).abs() < f64::EPSILON;
    let dn_default = (nbdevdn - 2.0).abs() < f64::EPSILON;
    let stddev_default = (stddev - 2.0).abs() < f64::EPSILON;
    if !stddev_default {
        (stddev, stddev)
    } else if !up_default || !dn_default {
        (nbdevup, nbdevdn)
    } else {
        (stddev, stddev)
    }
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Sma { input, period, output, format } => {
            let data = read_close_input_legacy(&input).expect("Failed to read input");
            let result = moving_avg::sma(&data, period).expect("SMA calculation failed");
            output_single("sma", result.as_slice().expect("unexpected None/Err in CLI handler (see cli/src/main.rs)"), &format, &output);
        }
        Commands::Ema { input, period, output, format } => {
            let data = read_close_input_legacy(&input).expect("Failed to read input");
            let result = moving_avg::ema(&data, period).expect("EMA calculation failed");
            output_single("ema", result.as_slice().expect("unexpected None/Err in CLI handler (see cli/src/main.rs)"), &format, &output);
        }
        Commands::Wma { input, period, output, format } => {
            let data = read_close_input_legacy(&input).expect("Failed to read input");
            let result = moving_avg::wma(&data, period).expect("WMA calculation failed");
            output_single("wma", result.as_slice().expect("unexpected None/Err in CLI handler (see cli/src/main.rs)"), &format, &output);
        }
        Commands::Rsi { input, period, output, format } => {
            let data = read_close_input_legacy(&input).expect("Failed to read input");
            let result = indicators::rsi(&data, period).expect("RSI calculation failed");
            output_single("rsi", result.as_slice().expect("unexpected None/Err in CLI handler (see cli/src/main.rs)"), &format, &output);
        }
        Commands::Macd { input, fast, slow, signal, output, format } => {
            let data = read_close_input_legacy(&input).expect("Failed to read input");
            let result = indicators::macd(&data, fast, slow, signal).expect("MACD calculation failed");
            let macd_s = result.macd.as_slice().expect("unexpected None/Err in CLI handler (see cli/src/main.rs)");
            let signal_s = result.signal.as_slice().expect("unexpected None/Err in CLI handler (see cli/src/main.rs)");
            let hist_s = result.hist.as_slice().expect("unexpected None/Err in CLI handler (see cli/src/main.rs)");
            output_multi(&["macd", "signal", "histogram"], &[macd_s, signal_s, hist_s], &format, &output);
        }
        Commands::Bbands { input, period, stddev, nbdevup, nbdevdn, output, format } => {
            let ohlcv = read_ohlcv_input(Some(&input)).expect("Failed to read OHLCV input");
            let (dev_up, dev_dn) = resolve_bbands_stddev(stddev, nbdevup, nbdevdn);
            let result = indicators::bbands(&ohlcv.close, period, dev_up, dev_dn)
                .expect("BBANDS calculation failed");
            let upper = result.upper.as_slice().expect("unexpected None/Err in CLI handler (see cli/src/main.rs)");
            let middle = result.middle.as_slice().expect("unexpected None/Err in CLI handler (see cli/src/main.rs)");
            let lower = result.lower.as_slice().expect("unexpected None/Err in CLI handler (see cli/src/main.rs)");
            output_multi(&["upper", "middle", "lower"], &[upper, middle, lower], &format, &output);
        }
        Commands::Atr { input, period, output, format } => {
            let ohlcv = read_ohlcv_input(Some(&input)).expect("Failed to read OHLCV input");
            let result = indicators::atr(&ohlcv.high, &ohlcv.low, &ohlcv.close, period)
                .expect("ATR calculation failed");
            output_single("atr", result.as_slice().expect("unexpected None/Err in CLI handler (see cli/src/main.rs)"), &format, &output);
        }
        Commands::Stoch { input, fastk_period, slowk_period, slowd_period, output, format } => {
            let ohlcv = read_ohlcv_input(Some(&input)).expect("Failed to read OHLCV input");
            let result = indicators::stoch(&ohlcv.high, &ohlcv.low, &ohlcv.close, fastk_period, slowk_period, slowd_period)
                .expect("STOCH calculation failed");
            let k = result.k.as_slice().expect("unexpected None/Err in CLI handler (see cli/src/main.rs)");
            let d = result.d.as_slice().expect("unexpected None/Err in CLI handler (see cli/src/main.rs)");
            output_multi(&["k", "d"], &[k, d], &format, &output);
        }
        Commands::Adx { input, period, output, format } => {
            let ohlcv = read_ohlcv_input(Some(&input)).expect("Failed to read OHLCV input");
            let result = indicators::adx(&ohlcv.high, &ohlcv.low, &ohlcv.close, period)
                .expect("ADX calculation failed");
            output_single("adx", result.as_slice().expect("unexpected None/Err in CLI handler (see cli/src/main.rs)"), &format, &output);
        }
        Commands::Cci { input, period, output, format } => {
            let ohlcv = read_ohlcv_input(Some(&input)).expect("Failed to read OHLCV input");
            let result = indicators::cci(&ohlcv.high, &ohlcv.low, &ohlcv.close, period)
                .expect("CCI calculation failed");
            output_single("cci", result.as_slice().expect("unexpected None/Err in CLI handler (see cli/src/main.rs)"), &format, &output);
        }
        Commands::Obv { input, output, format } => {
            let ohlcv = read_ohlcv_input(Some(&input)).expect("Failed to read OHLCV input");
            let result = indicators::obv(&ohlcv.close, &ohlcv.volume)
                .expect("OBV calculation failed");
            output_single("obv", result.as_slice().expect("unexpected None/Err in CLI handler (see cli/src/main.rs)"), &format, &output);
        }
        Commands::Willr { input, period, output, format } => {
            let ohlcv = read_ohlcv_input(Some(&input)).expect("Failed to read OHLCV input");
            let result = indicators::willr(&ohlcv.high, &ohlcv.low, &ohlcv.close, period)
                .expect("WILLR calculation failed");
            output_single("willr", result.as_slice().expect("unexpected None/Err in CLI handler (see cli/src/main.rs)"), &format, &output);
        }
        Commands::Pattern { input, kind, name, format } => {
            let ohlcv = read_ohlcv_input(Some(&input)).expect("Failed to read OHLCV input");
            match kind {
                PatternKind::Candlestick => {
                    let pattern_name = name.as_deref().unwrap_or("doji");
                    let result = match pattern_name {
                        "doji" => candlestick::doji(&ohlcv.open, &ohlcv.high, &ohlcv.low, &ohlcv.close, 0.1),
                        "hammer" => candlestick::hammer(&ohlcv.open, &ohlcv.high, &ohlcv.low, &ohlcv.close),
                        "engulfing" => candlestick::engulfing(&ohlcv.open, &ohlcv.high, &ohlcv.low, &ohlcv.close),
                        "morning_star" => candlestick::morning_star(&ohlcv.open, &ohlcv.high, &ohlcv.low, &ohlcv.close),
                        "evening_star" => candlestick::evening_star(&ohlcv.open, &ohlcv.high, &ohlcv.low, &ohlcv.close),
                        "shooting_star" => candlestick::shooting_star(&ohlcv.open, &ohlcv.high, &ohlcv.low, &ohlcv.close),
                        "hanging_man" => candlestick::hanging_man(&ohlcv.open, &ohlcv.high, &ohlcv.low, &ohlcv.close),
                        "inverted_hammer" => candlestick::inverted_hammer(&ohlcv.open, &ohlcv.high, &ohlcv.low, &ohlcv.close),
                        "dark_cloud" => candlestick::dark_cloud_cover(&ohlcv.open, &ohlcv.high, &ohlcv.low, &ohlcv.close),
                        "piercing" => candlestick::piercing(&ohlcv.open, &ohlcv.high, &ohlcv.low, &ohlcv.close),
                        other => {
                            eprintln!("Unknown candlestick pattern: {other}. Available: doji, hammer, engulfing, morning_star, evening_star, shooting_star, hanging_man, inverted_hammer, dark_cloud, piercing");
                            std::process::exit(1);
                        }
                    };
                    match result {
                        Ok(arr) => {
                            let signals: Vec<i32> = arr.to_vec();
                            match format {
                                OutputFormat::Json => {
                                    let items: Vec<String> = signals.iter().map(|v| format!("{v}")).collect();
                                    println!("{{\"pattern\":\"{pattern_name}\",\"signals\":[{}]}}", items.join(","));
                                }
                                _ => {
                                    for v in &signals { println!("{v}"); }
                                }
                            }
                        }
                        Err(e) => { eprintln!("Pattern error: {e}"); std::process::exit(1); }
                    }
                }
                PatternKind::Chart => {
                    let pattern_name = name.as_deref().unwrap_or("double_top");
                    let result = match pattern_name {
                        "double_top" => chart::double_top(&ohlcv.high, 20, 0.03),
                        "double_bottom" => chart::double_bottom(&ohlcv.low, 20, 0.03),
                        "head_shoulders" => chart::head_and_shoulders_top(&ohlcv.high, 30, 0.05),
                        "head_shoulders_bottom" => chart::head_and_shoulders_bottom(&ohlcv.low, 30, 0.05),
                        "ascending_triangle" => chart::ascending_triangle(&ohlcv.high, &ohlcv.low, 20, 0.03),
                        "descending_triangle" => chart::descending_triangle(&ohlcv.high, &ohlcv.low, 20, 0.03),
                        other => {
                            eprintln!("Unknown chart pattern: {other}. Available: double_top, double_bottom, head_shoulders, head_shoulders_bottom, ascending_triangle, descending_triangle");
                            std::process::exit(1);
                        }
                    };
                    match result {
                        Ok(arr) => {
                            let signals: Vec<i32> = arr.to_vec();
                            match format {
                                OutputFormat::Json => {
                                    let items: Vec<String> = signals.iter().map(|v| format!("{v}")).collect();
                                    println!("{{\"pattern\":\"{pattern_name}\",\"signals\":[{}]}}", items.join(","));
                                }
                                _ => {
                                    for v in &signals { println!("{v}"); }
                                }
                            }
                        }
                        Err(e) => { eprintln!("Pattern error: {e}"); std::process::exit(1); }
                    }
                }
            }
        }
        Commands::Formula { formula, input, expr, dialect, format } => {
            let expr_str = match (formula, expr) {
                (Some(f), _) => f,
                (None, Some(e)) => e,
                (None, None) => {
                    eprintln!("Formula expression required: positional `alpha-ta-cli formula \"MA(C,5)\" -i data.csv` or `--expr MA(C,5)`");
                    std::process::exit(1);
                }
            };
            let ohlcv = read_ohlcv_input(Some(&input)).expect("Failed to read OHLCV input");
            let open_arr = ndarray::Array1::from_vec(ohlcv.open);
            let high_arr = ndarray::Array1::from_vec(ohlcv.high);
            let low_arr = ndarray::Array1::from_vec(ohlcv.low);
            let close_arr = ndarray::Array1::from_vec(ohlcv.close);
            let volume_arr = ndarray::Array1::from_vec(ohlcv.volume);

            let mut ctx = alpha_ta_core::formula::FormulaContext::new(
                open_arr, high_arr, low_arr, close_arr, volume_arr, None,
            );
            let mut engine = alpha_ta_core::formula::FormulaEngine::new();

            let dialect = alpha_ta_core::formula::FormulaDialect::from_str(&dialect)
                .unwrap_or(alpha_ta_core::formula::FormulaDialect::AlphaTA);
            let result = match dialect {
                alpha_ta_core::formula::FormulaDialect::AlphaTA => {
                    engine.eval(&expr_str, &mut ctx)
                }
                alpha_ta_core::formula::FormulaDialect::Pine => {
                    let ast = alpha_ta_core::formula::parse_formula_with_dialect(
                        &expr_str,
                        alpha_ta_core::formula::FormulaDialect::Pine,
                    )
                    .map_err(|e| {
                        eprintln!("Pine parse/map error: {e}");
                        std::process::exit(1);
                    })
                    .unwrap();
                    engine.eval_ast(&ast, &mut ctx)
                }
            };
            match result {
                Ok(result) => {
                    output_single("result", result.as_slice().expect("unexpected None/Err in CLI handler (see cli/src/main.rs)"), &format, &None);
                }
                Err(e) => {
                    eprintln!("Formula error: {e}");
                    std::process::exit(1);
                }
            }
        }
        Commands::Streaming { indicator, input, period, fast_period, slow_period, signal_period, nb_dev, output, format } => {
            run_streaming(&indicator, &input, period, fast_period, slow_period, signal_period, nb_dev, output, format);
        }
        Commands::Transform { transform, input, period, output, format } => {
            run_transform(&transform, input.as_deref(), period, output, format);
        }
        Commands::Features { pack, input, period, output, format } => {
            run_features(&pack, &input, period, output, format);
        }
        Commands::Sweep { indicator, input, period_min, period_max, period_step, metric, output, format } => {
            run_sweep(&indicator, input.as_deref(), period_min, period_max, period_step, &metric, output, format);
        }
        Commands::Chart { input, chart_format, title, output } => {
            run_chart(&input, &chart_format, title.as_deref(), output);
        }
        Commands::Calc { indicator, input, period, fast, slow, signal, stddev, output, format } => {
            run_calc(&indicator, &input, period, fast, slow, signal, stddev, output, format);
        }
        Commands::Template { action, name, input, output, format } => {
            run_template(&action, name.as_deref(), input.as_deref(), output, format);
        }
    }
}

// ─────────────────────── CLI subcommand implementations ───────────────────────

use alpha_ta_core::streaming::{StreamingIndicator, OhlcvBar};
use alpha_ta_core::transforms::{LogReturn, PctChange, ZScore, StandardScaler, MinMaxScaler, Rank, PercentileRank, Diff, DiffN, RollingMean, RollingStd, RollingSum, Transform};

fn run_streaming(
    indicator: &str,
    input: &PathBuf,
    period: usize,
    fast_period: usize,
    slow_period: usize,
    signal_period: usize,
    nb_dev: f64,
    output: Option<String>,
    format: OutputFormat,
) {
    use alpha_ta_core::streaming::indicators::*;
    let ohlcv = read_ohlcv_input(Some(input)).expect("Failed to read OHLCV input");
    let close = ohlcv.close.clone();
    let high = ohlcv.high.clone();
    let low = ohlcv.low.clone();
    let volume = ohlcv.volume.clone();
    match indicator {
        "sma" => {
            let mut ind = StreamingSma::new(period);
            let vals: Vec<f64> = close.iter().map(|&v| ind.next(v).unwrap_or(f64::NAN)).collect();
            output_single("streaming_sma", &vals, &format, &output);
        }
        "ema" => {
            let mut ind = StreamingEma::new(period);
            let vals: Vec<f64> = close.iter().map(|&v| ind.next(v).unwrap_or(f64::NAN)).collect();
            output_single("streaming_ema", &vals, &format, &output);
        }
        "rsi" => {
            let mut ind = StreamingRsi::new(period);
            let vals: Vec<f64> = close.iter().map(|&v| ind.next(v).unwrap_or(f64::NAN)).collect();
            output_single("streaming_rsi", &vals, &format, &output);
        }
        "atr" => {
            let mut ind = StreamingAtr::new(period);
            let vals: Vec<f64> = high.iter().zip(low.iter()).zip(close.iter())
                .map(|((&h, &l), &c)| ind.next((h, l, c)).unwrap_or(f64::NAN))
                .collect();
            output_single("streaming_atr", &vals, &format, &output);
        }
        "adx" => {
            let mut ind = StreamingAdx::new(period);
            let vals: Vec<f64> = high.iter().zip(low.iter()).zip(close.iter())
                .map(|((&h, &l), &c)| ind.next((h, l, c)).unwrap_or(f64::NAN))
                .collect();
            output_single("streaming_adx", &vals, &format, &output);
        }
        "stoch" => {
            let mut ind = StreamingStoch::new(period, 3, 3);
            let vals: Vec<f64> = high.iter().zip(low.iter()).zip(close.iter())
                .map(|((&h, &l), &c)| ind.next((h, l, c)).map(|o| o.k).unwrap_or(f64::NAN))
                .collect();
            output_single("streaming_stoch_k", &vals, &format, &output);
        }
        "macd" => {
            let mut ind = StreamingMacd::new(fast_period, slow_period, signal_period);
            let macd_line: Vec<f64> = close.iter().map(|&v| ind.next(v).map(|o| o.macd).unwrap_or(f64::NAN)).collect();
            let signal_line: Vec<f64> = close.iter().map(|&v| ind.next(v).map(|o| o.signal).unwrap_or(f64::NAN)).collect();
            output_multi(&["macd", "signal"], &[&macd_line, &signal_line], &format, &output);
        }
        "boll" => {
            let mut ind = StreamingBoll::new(period, nb_dev, nb_dev);
            let uppers: Vec<f64> = close.iter().map(|&v| ind.next(v).map(|o| o.upper).unwrap_or(f64::NAN)).collect();
            let middles: Vec<f64> = close.iter().map(|&v| ind.next(v).map(|o| o.middle).unwrap_or(f64::NAN)).collect();
            let lowers: Vec<f64> = close.iter().map(|&v| ind.next(v).map(|o| o.lower).unwrap_or(f64::NAN)).collect();
            output_multi(&["upper", "middle", "lower"], &[&uppers, &middles, &lowers], &format, &output);
        }
        "vwap" => {
            let mut ind = StreamingVwap::new();
            let vals: Vec<f64> = high.iter().zip(low.iter()).zip(close.iter()).zip(volume.iter())
                .map(|(((&h, &l), &c), &v)| {
                    let bar = OhlcvBar::new(0.0, h, l, c, v);
                    ind.next(&bar).unwrap_or(f64::NAN)
                })
                .collect();
            output_single("streaming_vwap", &vals, &format, &output);
        }
        "obv" => {
            let mut ind = StreamingObv::new();
            let vals: Vec<f64> = close.iter().zip(volume.iter())
                .map(|(&c, &v)| {
                    let bar = OhlcvBar::new(0.0, 0.0, 0.0, c, v);
                    ind.next(&bar).unwrap_or(f64::NAN)
                })
                .collect();
            output_single("streaming_obv", &vals, &format, &output);
        }
        "supertrend" => {
            let mut ind = StreamingSuperTrend::new(period, nb_dev);
            let vals: Vec<f64> = high.iter().zip(low.iter()).zip(close.iter())
                .map(|((&h, &l), &c)| {
                    let bar = OhlcvBar::new(0.0, h, l, c, 0.0);
                    ind.next(&bar).map(|o| o.supertrend).unwrap_or(f64::NAN)
                })
                .collect();
            output_single("streaming_supertrend", &vals, &format, &output);
        }
        other => {
            eprintln!("Unknown streaming indicator: {other}. Available: sma, ema, rsi, atr, adx, stoch, macd, boll, vwap, obv, supertrend");
            std::process::exit(1);
        }
    }
}

fn run_transform(
    transform: &str,
    input: Option<&str>,
    period: usize,
    output: Option<String>,
    format: OutputFormat,
) {
    let data = read_close_input(input).expect("Failed to read input");
    let result: Vec<f64> = match transform {
        "log_return" => LogReturn.transform(&data),
        "pct_change" => PctChange.transform(&data),
        "zscore" => ZScore.transform(&data),
        "standard_scaler" => StandardScaler.transform(&data),
        "minmax_scaler" => MinMaxScaler.transform(&data),
        "rank" => Rank.transform(&data),
        "percentile_rank" => PercentileRank.transform(&data),
        "diff" => Diff.transform(&data),
        "diff2" => DiffN { order: 2 }.transform(&data),
        "rolling_mean" => RollingMean { window: period }.transform(&data),
        "rolling_std" => RollingStd { window: period }.transform(&data),
        "rolling_sum" => RollingSum { window: period }.transform(&data),
        other => {
            eprintln!("Unknown transform: {other}. Available: log_return, pct_change, zscore, standard_scaler, minmax_scaler, rank, percentile_rank, diff, diff2, rolling_mean, rolling_std, rolling_sum");
            std::process::exit(1);
        }
    };
    output_single(transform, &result, &format, &output);
}

fn run_features(
    pack: &str,
    input: &PathBuf,
    period: usize,
    output: Option<String>,
    format: OutputFormat,
) {
    use alpha_ta_core::math::moving_avg;
    let ohlcv = read_ohlcv_input(Some(input)).expect("Failed to read OHLCV input");
    let close = &ohlcv.close;
    let high = &ohlcv.high;
    let low = &ohlcv.low;
    let volume = &ohlcv.volume;
    match pack {
        "alpha_pack" => {
            // Build a default alpha factor pack: sma, ema, rsi, macd_hist, atr, obv, returns, zscore
            let sma = moving_avg::sma(close, period).expect("sma").as_slice().expect("unexpected None/Err in CLI handler (see cli/src/main.rs)").to_vec();
            let ema = moving_avg::ema(close, period).expect("ema").as_slice().expect("unexpected None/Err in CLI handler (see cli/src/main.rs)").to_vec();
            let rsi = indicators::rsi(close, period).expect("rsi").as_slice().expect("unexpected None/Err in CLI handler (see cli/src/main.rs)").to_vec();
            let macd = indicators::macd(close, 12, 26, 9).expect("macd");
            let atr_v = indicators::atr(high, low, close, period).expect("atr").as_slice().expect("unexpected None/Err in CLI handler (see cli/src/main.rs)").to_vec();
            let obv_v = indicators::obv(close, volume).expect("obv").as_slice().expect("unexpected None/Err in CLI handler (see cli/src/main.rs)").to_vec();
            let returns: Vec<f64> = LogReturn.transform(close);
            let zscore: Vec<f64> = ZScore.transform(close);
            // Align all columns to the same length (the minimum, so log_return fits)
            let len = sma.len().min(ema.len()).min(rsi.len())
                .min(macd.macd.len()).min(macd.signal.len()).min(macd.hist.len())
                .min(atr_v.len()).min(obv_v.len())
                .min(returns.len() + 1).min(zscore.len());
            let sma = &sma[..len];
            let ema = &ema[..len];
            let rsi = &rsi[..len];
            let macd_line = &macd.macd.as_slice().expect("unexpected None/Err in CLI handler (see cli/src/main.rs)")[..len];
            let macd_sig = &macd.signal.as_slice().expect("unexpected None/Err in CLI handler (see cli/src/main.rs)")[..len];
            let macd_hist = &macd.hist.as_slice().expect("unexpected None/Err in CLI handler (see cli/src/main.rs)")[..len];
            let atr_v = &atr_v[..len];
            let obv_v = &obv_v[..len];
            // log_return is shorter by 1; prepend a NaN so length matches.
            let mut returns_aligned: Vec<f64> = Vec::with_capacity(len);
            returns_aligned.push(f64::NAN);
            returns_aligned.extend_from_slice(&returns[..returns.len().min(len.saturating_sub(1))]);
            while returns_aligned.len() < len { returns_aligned.push(f64::NAN); }
            let zscore = &zscore[..len];
            let names = ["sma", "ema", "rsi", "macd", "macd_signal", "macd_hist", "atr", "obv", "log_return", "zscore"];
            let cols: Vec<&[f64]> = vec![
                sma, ema, rsi, macd_line, macd_sig, macd_hist, atr_v, obv_v, &returns_aligned, zscore,
            ];
            output_multi(&names, &cols, &format, &output);
        }
        other => {
            eprintln!("Unknown feature pack: {other}. Available: alpha_pack");
            std::process::exit(1);
        }
    }
}

fn run_sweep(
    indicator: &str,
    input: Option<&str>,
    period_min: usize,
    period_max: usize,
    period_step: usize,
    metric: &str,
    output: Option<String>,
    format: OutputFormat,
) {
    use alpha_ta_core::math::moving_avg;
    let data = read_close_input(input).expect("Failed to read input");
    if period_step == 0 {
        eprintln!("--period-step must be > 0");
        std::process::exit(1);
    }
    if period_min > period_max {
        eprintln!("--period-min must be <= --period-max");
        std::process::exit(1);
    }
    let compute_metric = |vals: &[f64]| -> f64 {
        let valid: Vec<f64> = vals.iter().copied().filter(|v| v.is_finite()).collect();
        if valid.is_empty() { return f64::NAN; }
        match metric {
            "mean" => valid.iter().sum::<f64>() / valid.len() as f64,
            "std" => {
                let m = valid.iter().sum::<f64>() / valid.len() as f64;
                let var = valid.iter().map(|v| (v - m).powi(2)).sum::<f64>() / valid.len() as f64;
                var.sqrt()
            }
            "min" => valid.iter().cloned().fold(f64::INFINITY, f64::min),
            "max" => valid.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
            "last" => *valid.last().expect("unexpected None/Err in CLI handler (see cli/src/main.rs)"),
            "slope" => {
                // OLS slope of valid sequence on its index
                let n = valid.len() as f64;
                let xs: Vec<f64> = (0..valid.len()).map(|i| i as f64).collect();
                let x_mean = xs.iter().sum::<f64>() / n;
                let y_mean = valid.iter().sum::<f64>() / n;
                let num: f64 = xs.iter().zip(valid.iter()).map(|(x, y)| (x - x_mean) * (y - y_mean)).sum();
                let den: f64 = xs.iter().map(|x| (x - x_mean).powi(2)).sum();
                if den == 0.0 { 0.0 } else { num / den }
            }
            other => {
                eprintln!("Unknown metric: {other}. Available: mean, std, min, max, last, slope");
                std::process::exit(1);
            }
        }
    };
    let text = match format {
        OutputFormat::Csv => {
            let mut out = String::from("period,value\n");
            let mut p = period_min;
            while p <= period_max {
                let vals = match indicator {
                    "sma" => moving_avg::sma(&data, p).ok().map(|a| a.into_raw_vec_and_offset().0),
                    "ema" => moving_avg::ema(&data, p).ok().map(|a| a.into_raw_vec_and_offset().0),
                    "wma" => moving_avg::wma(&data, p).ok().map(|a| a.into_raw_vec_and_offset().0),
                    "rsi" => indicators::rsi(&data, p).ok().map(|a| a.into_raw_vec_and_offset().0),
                    "atr" => {
                        let ohlcv = read_ohlcv_input(None::<&str>).ok();
                        ohlcv.and_then(|d| indicators::atr(&d.high, &d.low, &d.close, p).ok().map(|a| a.into_raw_vec_and_offset().0))
                    }
                    other => {
                        eprintln!("Unknown sweep indicator: {other}. Available: sma, ema, wma, rsi, atr");
                        std::process::exit(1);
                    }
                };
                let v = vals.as_deref().map(compute_metric).unwrap_or(f64::NAN);
                out.push_str(&format!("{p},{v}\n"));
                p = match p.checked_add(period_step) {
                    Some(v) => v,
                    None => break,
                };
            }
            out
        }
        _ => {
            let mut out = String::new();
            let mut p = period_min;
            while p <= period_max {
                let vals = match indicator {
                    "sma" => moving_avg::sma(&data, p).ok().map(|a| a.into_raw_vec_and_offset().0),
                    "ema" => moving_avg::ema(&data, p).ok().map(|a| a.into_raw_vec_and_offset().0),
                    "wma" => moving_avg::wma(&data, p).ok().map(|a| a.into_raw_vec_and_offset().0),
                    "rsi" => indicators::rsi(&data, p).ok().map(|a| a.into_raw_vec_and_offset().0),
                    _ => None,
                };
                let v = vals.as_deref().map(compute_metric).unwrap_or(f64::NAN);
                out.push_str(&format!("period={p} {metric}={v}\n"));
                p = match p.checked_add(period_step) {
                    Some(v) => v,
                    None => break,
                };
            }
            out
        }
    };
    if let Some(path) = output {
        fs::write(&path, &text).expect("Failed to write output");
    } else {
        print!("{text}");
    }
}

fn run_chart(
    input: &PathBuf,
    chart_format: &str,
    title: Option<&str>,
    output: Option<String>,
) {
    use alpha_ta_visualization::config::ChartConfig;
    use alpha_ta_visualization::data::KlineData;
    use alpha_ta_visualization::renderer::{ChartRenderer, Renderer};
    let ohlcv = read_ohlcv_input(Some(input)).expect("Failed to read OHLCV input");
    let mut cfg = ChartConfig::default();
    if let Some(t) = title {
        cfg.title = t.to_string();
    }
    let renderer = ChartRenderer::new(cfg);
    let dates: Vec<String> = (0..ohlcv.close.len())
        .map(|i| format!("bar_{i}"))
        .collect();
    let kline = KlineData::new(
        dates,
        ohlcv.open.clone(),
        ohlcv.high.clone(),
        ohlcv.low.clone(),
        ohlcv.close.clone(),
        ohlcv.volume.clone(),
    );
    let payload = match chart_format {
        "json" => {
            renderer.render(&kline, &[]).expect("render failed")
        }
        "svg" | "html" => {
            // Generate a minimal SVG with the closing line as a quick visual preview.
            let mut svg = String::new();
            let w = 800usize;
            let h = 400usize;
            let closes = &kline.closes;
            let min_v = closes.iter().cloned().fold(f64::INFINITY, f64::min);
            let max_v = closes.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let range = (max_v - min_v).max(1e-12);
            svg.push_str(&format!(
                "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">\n"
            ));
            svg.push_str(&format!(
                "<rect width=\"100%\" height=\"100%\" fill=\"#ffffff\"/>\n"
            ));
            if let Some(t) = title {
                svg.push_str(&format!(
                    "<text x=\"10\" y=\"20\" font-size=\"14\" fill=\"#000\">{t}</text>\n"
                ));
            }
            let mut points = String::new();
            for (i, v) in closes.iter().enumerate() {
                if v.is_nan() { continue; }
                let x = (i as f64 / (closes.len().max(1) - 1).max(1) as f64) * w as f64;
                let y = h as f64 - ((v - min_v) / range) * h as f64;
                if i > 0 { points.push(' '); }
                points.push_str(&format!("{:.2},{:.2}", x, y));
            }
            svg.push_str(&format!(
                "<polyline points=\"{points}\" fill=\"none\" stroke=\"#1f77b4\" stroke-width=\"1.5\"/>\n"
            ));
            svg.push_str("</svg>\n");
            if chart_format == "html" {
                format!(
                    "<!doctype html><html><head><meta charset=\"utf-8\"><title>{}</title></head><body>{}</body></html>\n",
                    title.unwrap_or("AlphaTA Chart"), svg
                )
            } else {
                svg
            }
        }
        other => {
            eprintln!("Unknown chart format: {other}. Available: svg, html, json");
            std::process::exit(1);
        }
    };
    if let Some(path) = output {
        fs::write(&path, &payload).expect("Failed to write output");
    } else {
        print!("{payload}");
    }
}

fn run_calc(
    indicator: &str,
    input: &PathBuf,
    period: usize,
    fast: usize,
    slow: usize,
    signal: usize,
    stddev: f64,
    output: Option<String>,
    format: OutputFormat,
) {
    let indicator_upper = indicator.to_uppercase();
    match indicator_upper.as_str() {
        "SMA" => {
            let ohlcv = read_ohlcv_input(Some(input)).expect("Failed to read input");
            let result = moving_avg::sma(&ohlcv.close, period).expect("SMA calculation failed");
            output_single("sma", result.as_slice().expect("unexpected None/Err in CLI handler (see cli/src/main.rs)"), &format, &output);
        }
        "EMA" => {
            let ohlcv = read_ohlcv_input(Some(input)).expect("Failed to read input");
            let result = moving_avg::ema(&ohlcv.close, period).expect("EMA calculation failed");
            output_single("ema", result.as_slice().expect("unexpected None/Err in CLI handler (see cli/src/main.rs)"), &format, &output);
        }
        "WMA" => {
            let ohlcv = read_ohlcv_input(Some(input)).expect("Failed to read input");
            let result = moving_avg::wma(&ohlcv.close, period).expect("WMA calculation failed");
            output_single("wma", result.as_slice().expect("unexpected None/Err in CLI handler (see cli/src/main.rs)"), &format, &output);
        }
        "RSI" => {
            let ohlcv = read_ohlcv_input(Some(input)).expect("Failed to read input");
            let result = indicators::rsi(&ohlcv.close, period).expect("RSI calculation failed");
            output_single("rsi", result.as_slice().expect("unexpected None/Err in CLI handler (see cli/src/main.rs)"), &format, &output);
        }
        "MACD" => {
            let ohlcv = read_ohlcv_input(Some(input)).expect("Failed to read input");
            let result = indicators::macd(&ohlcv.close, fast, slow, signal).expect("MACD calculation failed");
            let macd_s = result.macd.as_slice().expect("unexpected None/Err in CLI handler (see cli/src/main.rs)");
            let signal_s = result.signal.as_slice().expect("unexpected None/Err in CLI handler (see cli/src/main.rs)");
            let hist_s = result.hist.as_slice().expect("unexpected None/Err in CLI handler (see cli/src/main.rs)");
            output_multi(&["macd", "signal", "histogram"], &[macd_s, signal_s, hist_s], &format, &output);
        }
        "ATR" => {
            let ohlcv = read_ohlcv_input(Some(input)).expect("Failed to read input");
            let result = indicators::atr(&ohlcv.high, &ohlcv.low, &ohlcv.close, period)
                .expect("ATR calculation failed");
            output_single("atr", result.as_slice().expect("unexpected None/Err in CLI handler (see cli/src/main.rs)"), &format, &output);
        }
        "BBANDS" | "BOLL" => {
            let ohlcv = read_ohlcv_input(Some(input)).expect("Failed to read input");
            let result = indicators::bbands(&ohlcv.close, period, stddev, stddev)
                .expect("BBANDS calculation failed");
            let upper = result.upper.as_slice().expect("unexpected None/Err in CLI handler (see cli/src/main.rs)");
            let middle = result.middle.as_slice().expect("unexpected None/Err in CLI handler (see cli/src/main.rs)");
            let lower = result.lower.as_slice().expect("unexpected None/Err in CLI handler (see cli/src/main.rs)");
            output_multi(&["upper", "middle", "lower"], &[upper, middle, lower], &format, &output);
        }
        "ADX" => {
            let ohlcv = read_ohlcv_input(Some(input)).expect("Failed to read input");
            let result = indicators::adx(&ohlcv.high, &ohlcv.low, &ohlcv.close, period)
                .expect("ADX calculation failed");
            output_single("adx", result.as_slice().expect("unexpected None/Err in CLI handler (see cli/src/main.rs)"), &format, &output);
        }
        "CCI" => {
            let ohlcv = read_ohlcv_input(Some(input)).expect("Failed to read input");
            let result = indicators::cci(&ohlcv.high, &ohlcv.low, &ohlcv.close, period)
                .expect("CCI calculation failed");
            output_single("cci", result.as_slice().expect("unexpected None/Err in CLI handler (see cli/src/main.rs)"), &format, &output);
        }
        "OBV" => {
            let ohlcv = read_ohlcv_input(Some(input)).expect("Failed to read input");
            let result = indicators::obv(&ohlcv.close, &ohlcv.volume)
                .expect("OBV calculation failed");
            output_single("obv", result.as_slice().expect("unexpected None/Err in CLI handler (see cli/src/main.rs)"), &format, &output);
        }
        "WILLR" => {
            let ohlcv = read_ohlcv_input(Some(input)).expect("Failed to read input");
            let result = indicators::willr(&ohlcv.high, &ohlcv.low, &ohlcv.close, period)
                .expect("WILLR calculation failed");
            output_single("willr", result.as_slice().expect("unexpected None/Err in CLI handler (see cli/src/main.rs)"), &format, &output);
        }
        "STOCH" => {
            let ohlcv = read_ohlcv_input(Some(input)).expect("Failed to read input");
            let result = indicators::stoch(&ohlcv.high, &ohlcv.low, &ohlcv.close, period, 3, 3)
                .expect("STOCH calculation failed");
            let k = result.k.as_slice().expect("unexpected None/Err in CLI handler (see cli/src/main.rs)");
            let d = result.d.as_slice().expect("unexpected None/Err in CLI handler (see cli/src/main.rs)");
            output_multi(&["k", "d"], &[k, d], &format, &output);
        }
        other => {
            eprintln!("Unknown indicator: {other}. Available: SMA, EMA, WMA, RSI, MACD, ATR, BBANDS, ADX, CCI, OBV, WILLR, STOCH");
            std::process::exit(1);
        }
    }
}

fn run_template(
    action: &str,
    name: Option<&str>,
    input: Option<&Path>,
    output: Option<String>,
    format: OutputFormat,
) {
    use alpha_ta_core::formula::{FormulaEngine, FormulaTemplates};
    let mut engine = FormulaEngine::new();
    let templates = FormulaTemplates::new();
    match action {
        "list" => {
            let categories = FormulaTemplates::categories();
            let text = match format {
                OutputFormat::Json => {
                    let mut parts = Vec::new();
                    for c in categories {
                        let items: Vec<String> = templates.get_by_category(&c).iter()
                            .map(|t| format!("{{\"name\":\"{}\",\"description\":\"{}\"}}", t.name, t.description))
                            .collect();
                        parts.push(format!("\"{:?}\":[{}]", c, items.join(",")));
                    }
                    format!("{{{}}}", parts.join(","))
                }
                _ => {
                    let mut out = String::new();
                    for c in categories {
                        out.push_str(&format!("\n=== {:?} ===\n", c));
                        for t in templates.get_by_category(&c) {
                            out.push_str(&format!("- {}: {}\n", t.name, t.description));
                        }
                    }
                    out
                }
            };
            if let Some(path) = output {
                fs::write(&path, &text).expect("Failed to write output");
            } else {
                print!("{text}");
            }
        }
        "search" => {
            let kw = name.unwrap_or("");
            let results = engine.search_templates(kw);
            let text = match format {
                OutputFormat::Json => {
                    let items: Vec<String> = results.iter().map(|t|
                        format!("{{\"name\":\"{}\",\"description\":\"{}\",\"category\":\"{:?}\"}}", t.name, t.description, t.category)
                    ).collect();
                    format!("[{}]", items.join(","))
                }
                _ => {
                    let mut out = String::new();
                    for t in results {
                        out.push_str(&format!("- {} [{:?}]: {}\n", t.name, t.category, t.description));
                    }
                    out
                }
            };
            if let Some(path) = output {
                fs::write(&path, &text).expect("Failed to write output");
            } else {
                print!("{text}");
            }
        }
        "info" => {
            let n = name.unwrap_or_else(|| {
                eprintln!("template info requires a template name");
                std::process::exit(1);
            });
            let tmpl = engine.get_template(n).unwrap_or_else(|| {
                eprintln!("Template not found: {n}");
                std::process::exit(1);
            });
            let text = match format {
                OutputFormat::Json => format!(
                    "{{\"name\":\"{}\",\"description\":\"{}\",\"category\":\"{:?}\",\"source\":\"{}\"}}",
                    tmpl.name, tmpl.description, tmpl.category,
                    tmpl.source.replace('\n', "\\n").replace('"', "\\\"")
                ),
                _ => format!(
                    "Name: {}\nCategory: {:?}\nDescription: {}\n\nSource:\n{}\n",
                    tmpl.name, tmpl.category, tmpl.description, tmpl.source
                ),
            };
            if let Some(path) = output {
                fs::write(&path, &text).expect("Failed to write output");
            } else {
                print!("{text}");
            }
        }
        "render" => {
            let n = name.unwrap_or_else(|| {
                eprintln!("template render requires a template name");
                std::process::exit(1);
            });
            let inp = input.unwrap_or_else(|| {
                eprintln!("template render requires --input");
                std::process::exit(1);
            });
            let source = {
                let tmpl = engine.get_template(n).unwrap_or_else(|| {
                    eprintln!("Template not found: {n}");
                    std::process::exit(1);
                });
                (tmpl.source.clone(), tmpl.name.clone())
            };
            let ohlcv = read_ohlcv_input(Some(inp)).expect("Failed to read OHLCV input");
            let open_arr = ndarray::Array1::from_vec(ohlcv.open);
            let high_arr = ndarray::Array1::from_vec(ohlcv.high);
            let low_arr = ndarray::Array1::from_vec(ohlcv.low);
            let close_arr = ndarray::Array1::from_vec(ohlcv.close);
            let volume_arr = ndarray::Array1::from_vec(ohlcv.volume);
            let mut ctx = alpha_ta_core::formula::FormulaContext::new(
                open_arr, high_arr, low_arr, close_arr, volume_arr, None,
            );
            match engine.eval(&source.0, &mut ctx) {
                Ok(result) => {
                    let vals = result.as_slice().expect("unexpected None/Err in CLI handler (see cli/src/main.rs)");
                    output_single(&source.1, vals, &format, &output);
                }
                Err(e) => {
                    eprintln!("Template render error: {e}");
                    std::process::exit(1);
                }
            }
        }
        other => {
            eprintln!("Unknown template action: {other}. Available: list, search, info, render");
            std::process::exit(1);
        }
    }
}
