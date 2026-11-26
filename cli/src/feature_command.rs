use super::{indicators, output_multi, LogReturn, OutputFormat, Transform, ZScore};
use crate::csv_io::{ohlcv_series, OhlcvData};
use std::path::PathBuf;

pub(super) fn run(
    pack: &str,
    ohlcv: &OhlcvData,
    format: &OutputFormat,
    output: &Option<String>,
) -> Result<(), String> {
    let missing = [
        ("open", ohlcv.has_open),
        ("high", ohlcv.has_high),
        ("low", ohlcv.has_low),
        ("vwap", ohlcv.has_vwap),
        ("volume", ohlcv.has_volume),
    ]
    .into_iter()
    .filter_map(|(name, present)| (!present).then_some(name))
    .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(format!(
            "{pack} requires CSV columns: {}",
            missing.join(", ")
        ));
    }

    let matrix = match pack {
        "alpha360" => finkit::features::alpha360(
            &ohlcv.open,
            &ohlcv.high,
            &ohlcv.low,
            &ohlcv.close,
            &ohlcv.vwap,
            &ohlcv.volume,
        )
        .map_err(|error| error.to_string()),
        "alpha158" => finkit::features::alpha158_features(
            &ohlcv.open,
            &ohlcv.high,
            &ohlcv.low,
            &ohlcv.close,
            &ohlcv.vwap,
            &ohlcv.volume,
        )
        .map_err(|error| error.to_string()),
        _ => return Err(format!("unsupported feature set: {pack}")),
    }
    .map_err(|error| format!("could not compute {pack}: {error}"))?;

    let names = matrix.column_names();
    let columns = matrix
        .columns()
        .iter()
        .map(Vec::as_slice)
        .collect::<Vec<_>>();
    output_multi(&names, &columns, format, output);
    Ok(())
}

pub(super) fn run_features(
    pack: &str,
    input: &PathBuf,
    period: usize,
    output: Option<String>,
    format: OutputFormat,
) {
    use finkit::math::moving_avg;
    let ohlcv = ohlcv_series(Some(input));
    let close = &ohlcv.close;
    let high = &ohlcv.high;
    let low = &ohlcv.low;
    let volume = &ohlcv.volume;
    match pack {
        "alpha360" | "alpha158" => {
            if let Err(error) = run(pack, &ohlcv, &format, &output) {
                eprintln!("{error}");
                std::process::exit(1);
            }
        }
        "alpha_pack" => {
            // Build a default alpha factor pack: sma, ema, rsi, macd_hist, atr, obv, returns, zscore
            let sma = moving_avg::sma(close, period)
                .expect("sma")
                .as_slice()
                .expect("unexpected None/Err in CLI handler (see cli/src/main.rs)")
                .to_vec();
            let ema = moving_avg::ema(close, period)
                .expect("ema")
                .as_slice()
                .expect("unexpected None/Err in CLI handler (see cli/src/main.rs)")
                .to_vec();
            let rsi = indicators::rsi(close, period)
                .expect("rsi")
                .as_slice()
                .expect("unexpected None/Err in CLI handler (see cli/src/main.rs)")
                .to_vec();
            let macd = indicators::macd(close, 12, 26, 9).expect("macd");
            let atr_v = indicators::atr(high, low, close, period)
                .expect("atr")
                .as_slice()
                .expect("unexpected None/Err in CLI handler (see cli/src/main.rs)")
                .to_vec();
            let obv_v = indicators::obv(close, volume)
                .expect("obv")
                .as_slice()
                .expect("unexpected None/Err in CLI handler (see cli/src/main.rs)")
                .to_vec();
            let returns: Vec<f64> = LogReturn.transform(close);
            let zscore: Vec<f64> = ZScore.transform(close);
            // Align all columns to the same length (the minimum, so log_return fits)
            let len = sma
                .len()
                .min(ema.len())
                .min(rsi.len())
                .min(macd.macd.len())
                .min(macd.signal.len())
                .min(macd.hist.len())
                .min(atr_v.len())
                .min(obv_v.len())
                .min(returns.len() + 1)
                .min(zscore.len());
            let sma = &sma[..len];
            let ema = &ema[..len];
            let rsi = &rsi[..len];
            let macd_line = &macd
                .macd
                .as_slice()
                .expect("unexpected None/Err in CLI handler (see cli/src/main.rs)")[..len];
            let macd_sig = &macd
                .signal
                .as_slice()
                .expect("unexpected None/Err in CLI handler (see cli/src/main.rs)")[..len];
            let macd_hist = &macd
                .hist
                .as_slice()
                .expect("unexpected None/Err in CLI handler (see cli/src/main.rs)")[..len];
            let atr_v = &atr_v[..len];
            let obv_v = &obv_v[..len];
            // log_return is shorter by 1; prepend a NaN so length matches.
            let mut returns_aligned: Vec<f64> = Vec::with_capacity(len);
            returns_aligned.push(f64::NAN);
            returns_aligned.extend_from_slice(&returns[..returns.len().min(len.saturating_sub(1))]);
            while returns_aligned.len() < len {
                returns_aligned.push(f64::NAN);
            }
            let zscore = &zscore[..len];
            let names = [
                "sma",
                "ema",
                "rsi",
                "macd",
                "macd_signal",
                "macd_hist",
                "atr",
                "obv",
                "log_return",
                "zscore",
            ];
            let cols: Vec<&[f64]> = vec![
                sma,
                ema,
                rsi,
                macd_line,
                macd_sig,
                macd_hist,
                atr_v,
                obv_v,
                &returns_aligned,
                zscore,
            ];
            output_multi(&names, &cols, &format, &output);
        }
        other => {
            eprintln!("Unknown feature pack: {other}. Available: alpha158, alpha360, alpha_pack");
            std::process::exit(1);
        }
    }
}
