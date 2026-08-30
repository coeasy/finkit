//! Golden (regression) tests against pre-computed reference CSV fixtures.
//!
//! Reference values are self-golden: generated once by our library via the
//! ignored `generate_golden` test, then compared on every run.

use finkit::indicators::{
    adx, apo, aroon, bbands, cci, cmo, macd, mom, roc, rsi, stoch, stochrsi, trix, volatility::atr,
    volatility::{natr, trange}, volume::{obv, vwap}, willr,
    classic_patterns::{darvas_box, kagi, point_and_figure, renko, three_line_break, williams_alligator},
    heikin_ashi,
};
use finkit::indicators::StochResult;
use finkit::math::moving_avg::{dema, ema, sma, tema, wma};
use std::fs;
use std::path::{Path, PathBuf};

const TOLERANCE: f64 = 1e-8;

#[derive(Debug, Clone)]
struct Ohlcv {
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    volume: Vec<f64>,
}

fn golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/golden")
}

fn generate_input_data() -> Ohlcv {
    let mut open = Vec::with_capacity(50);
    let mut high = Vec::with_capacity(50);
    let mut low = Vec::with_capacity(50);
    let mut close = Vec::with_capacity(50);
    let mut volume = Vec::with_capacity(50);

    for i in 0..50 {
        let i_f = i as f64;
        let base = 40000.0 + i_f * 50.0 + 500.0 * (i_f * 0.3).sin();
        open.push(base);
        high.push(base + 200.0 + 50.0 * (i_f * 0.7).sin().abs());
        low.push(base - 200.0 - 50.0 * (i_f * 0.7).cos().abs());
        close.push(base + 100.0 * (i_f * 0.5).sin());
        volume.push(100_000.0 + 50_000.0 * (i_f * 0.2).sin().abs());
    }

    Ohlcv {
        open,
        high,
        low,
        close,
        volume,
    }
}

fn format_f64(v: f64) -> String {
    if v.is_nan() {
        "NaN".to_string()
    } else {
        format!("{v:.17}", v = v)
    }
}

fn parse_f64(s: &str) -> f64 {
    let trimmed = s.trim();
    if trimmed.eq_ignore_ascii_case("nan") {
        f64::NAN
    } else {
        trimmed
            .parse()
            .unwrap_or_else(|_| panic!("invalid float: {s}"))
    }
}

fn write_input_csv(path: &Path, data: &Ohlcv) {
    let mut lines = vec!["open,high,low,close,volume".to_string()];
    for i in 0..data.close.len() {
        lines.push(format!(
            "{},{},{},{},{}",
            format_f64(data.open[i]),
            format_f64(data.high[i]),
            format_f64(data.low[i]),
            format_f64(data.close[i]),
            format_f64(data.volume[i]),
        ));
    }
    fs::write(path, lines.join("\n") + "\n").expect("write input csv");
}

fn write_series_csv(path: &Path, header: &str, values: &[f64]) {
    let mut lines = vec![format!("index,{header}")];
    for (idx, value) in values.iter().enumerate() {
        lines.push(format!("{idx},{}", format_f64(*value)));
    }
    fs::write(path, lines.join("\n") + "\n").expect("write series csv");
}

fn write_multi_series_csv(path: &Path, headers: &[&str], columns: &[&[f64]]) {
    assert!(!headers.is_empty());
    assert!(columns.iter().all(|col| col.len() == columns[0].len()));

    let header = format!("index,{}", headers.join(","));
    let mut lines = vec![header];
    let len = columns[0].len();
    for idx in 0..len {
        let mut row = vec![idx.to_string()];
        for col in columns {
            row.push(format_f64(col[idx]));
        }
        lines.push(row.join(","));
    }
    fs::write(path, lines.join("\n") + "\n").expect("write multi series csv");
}

fn read_input_csv(path: &Path) -> Ohlcv {
    let content = fs::read_to_string(path).expect("read input csv");
    let mut lines = content.lines().filter(|l| !l.trim().is_empty());
    let header = lines.next().expect("input csv header");
    assert!(
        header.contains("open") && header.contains("close"),
        "unexpected input header: {header}"
    );

    let mut open = Vec::new();
    let mut high = Vec::new();
    let mut low = Vec::new();
    let mut close = Vec::new();
    let mut volume = Vec::new();

    for line in lines {
        let parts: Vec<&str> = line.split(',').collect();
        assert_eq!(parts.len(), 5, "expected 5 OHLCV columns");
        open.push(parse_f64(parts[0]));
        high.push(parse_f64(parts[1]));
        low.push(parse_f64(parts[2]));
        close.push(parse_f64(parts[3]));
        volume.push(parse_f64(parts[4]));
    }

    Ohlcv {
        open,
        high,
        low,
        close,
        volume,
    }
}

fn read_series_csv(path: &Path) -> Vec<f64> {
    let content = fs::read_to_string(path).expect("read series csv");
    let mut lines = content.lines().filter(|l| !l.trim().is_empty());
    let _header = lines.next().expect("series csv header");

    let mut values = Vec::new();
    for line in lines {
        let parts: Vec<&str> = line.split(',').collect();
        assert!(parts.len() >= 2, "expected index,value row");
        values.push(parse_f64(parts[1]));
    }
    values
}

fn read_multi_series_csv(path: &Path) -> Vec<Vec<f64>> {
    let content = fs::read_to_string(path).expect("read multi series csv");
    let mut lines = content.lines().filter(|l| !l.trim().is_empty());
    let header = lines.next().expect("multi series csv header");
    let col_count = header.split(',').count() - 1;

    let mut columns: Vec<Vec<f64>> = vec![Vec::new(); col_count];
    for line in lines {
        let parts: Vec<&str> = line.split(',').collect();
        assert_eq!(parts.len(), col_count + 1);
        for (col_idx, cell) in parts.iter().skip(1).enumerate() {
            columns[col_idx].push(parse_f64(cell));
        }
    }
    columns
}

fn values_close(a: f64, b: f64) -> bool {
    if a.is_nan() && b.is_nan() {
        return true;
    }
    if a.is_nan() || b.is_nan() {
        return false;
    }
    (a - b).abs() < TOLERANCE
}

fn assert_series_eq(computed: &[f64], expected: &[f64], label: &str) {
    assert_eq!(
        computed.len(),
        expected.len(),
        "{label}: length mismatch (computed {}, expected {})",
        computed.len(),
        expected.len()
    );
    for (idx, (actual, reference)) in computed.iter().zip(expected.iter()).enumerate() {
        assert!(
            values_close(*actual, *reference),
            "{label} index {idx}: computed {actual}, expected {reference}, diff {}",
            (actual - reference).abs()
        );
    }
}

fn load_ohlcv() -> Ohlcv {
    read_input_csv(&golden_dir().join("input_ohlcv.csv"))
}

#[test]
#[ignore = "run once to (re)generate golden reference CSV files"]
fn generate_golden() {
    let dir = golden_dir();
    fs::create_dir_all(&dir).expect("create golden dir");

    let data = generate_input_data();
    write_input_csv(&dir.join("input_ohlcv.csv"), &data);

    let close = &data.close;
    let sma10 = sma(close, 10).unwrap();
    write_series_csv(&dir.join("sma_10.csv"), "sma", sma10.as_slice().unwrap());

    let sma20 = sma(close, 20).unwrap();
    write_series_csv(&dir.join("sma_20.csv"), "sma", sma20.as_slice().unwrap());

    let ema10 = ema(close, 10).unwrap();
    write_series_csv(&dir.join("ema_10.csv"), "ema", ema10.as_slice().unwrap());

    let ema20 = ema(close, 20).unwrap();
    write_series_csv(&dir.join("ema_20.csv"), "ema", ema20.as_slice().unwrap());

    let rsi14 = rsi(close, 14).unwrap();
    write_series_csv(&dir.join("rsi_14.csv"), "rsi", rsi14.as_slice().unwrap());

    let macd_res = macd(close, 12, 26, 9).unwrap();
    write_multi_series_csv(
        &dir.join("macd_12_26_9.csv"),
        &["macd", "signal", "hist"],
        &[
            macd_res.macd.as_slice().unwrap(),
            macd_res.signal.as_slice().unwrap(),
            macd_res.hist.as_slice().unwrap(),
        ],
    );

    let bb = bbands(close, 20, 2.0, 2.0).unwrap();
    write_multi_series_csv(
        &dir.join("bbands_20_2.csv"),
        &["upper", "middle", "lower"],
        &[
            bb.upper.as_slice().unwrap(),
            bb.middle.as_slice().unwrap(),
            bb.lower.as_slice().unwrap(),
        ],
    );

    let atr14 = atr(&data.high, &data.low, close, 14).unwrap();
    write_series_csv(&dir.join("atr_14.csv"), "atr", atr14.as_slice().unwrap());

    let st = stoch(&data.high, &data.low, close, 14, 3, 3).unwrap();
    write_multi_series_csv(
        &dir.join("stoch_14_3.csv"),
        &["k", "d"],
        &[st.k.as_slice().unwrap(), st.d.as_slice().unwrap()],
    );

    let vwap_vals = vwap(&data.high, &data.low, close, &data.volume).unwrap();
    write_series_csv(&dir.join("vwap.csv"), "vwap", vwap_vals.as_slice().unwrap());

    let adx14 = adx(&data.high, &data.low, close, 14).unwrap();
    write_series_csv(&dir.join("adx_14.csv"), "adx", adx14.as_slice().unwrap());

    let willr14 = willr(&data.high, &data.low, close, 14).unwrap();
    write_series_csv(
        &dir.join("willr_14.csv"),
        "willr",
        willr14.as_slice().unwrap(),
    );

    let mom10 = mom(close, 10).unwrap();
    write_series_csv(&dir.join("mom_10.csv"), "mom", mom10.as_slice().unwrap());

    let roc10 = roc(close, 10).unwrap();
    write_series_csv(&dir.join("roc_10.csv"), "roc", roc10.as_slice().unwrap());

    let aroon14 = aroon(&data.high, &data.low, 14).unwrap();
    write_multi_series_csv(
        &dir.join("aroon_14.csv"),
        &["up", "down"],
        &[
            aroon14.aroon_up.as_slice().unwrap(),
            aroon14.aroon_down.as_slice().unwrap(),
        ],
    );

    let cci14 = cci(&data.high, &data.low, close, 14).unwrap();
    write_series_csv(&dir.join("cci_14.csv"), "cci", cci14.as_slice().unwrap());

    let apo12_26 = apo(close, 12, 26).unwrap();
    write_series_csv(
        &dir.join("apo_12_26.csv"),
        "apo",
        apo12_26.as_slice().unwrap(),
    );

    let cmo14 = cmo(close, 14).unwrap();
    write_series_csv(&dir.join("cmo_14.csv"), "cmo", cmo14.as_slice().unwrap());

    let trix14 = trix(close, 14).unwrap();
    write_series_csv(
        &dir.join("trix_14.csv"),
        "trix",
        trix14.as_slice().unwrap(),
    );

    let natr14 = natr(&data.high, &data.low, close, 14).unwrap();
    write_series_csv(
        &dir.join("natr_14.csv"),
        "natr",
        natr14.as_slice().unwrap(),
    );

    let trange_vals = trange(&data.high, &data.low, close).unwrap();
    write_series_csv(
        &dir.join("trange.csv"),
        "trange",
        trange_vals.as_slice().unwrap(),
    );

    let wma10 = wma(close, 10).unwrap();
    write_series_csv(&dir.join("wma_10.csv"), "wma", wma10.as_slice().unwrap());

    let dema10 = dema(close, 10).unwrap();
    write_series_csv(
        &dir.join("dema_10.csv"),
        "dema",
        dema10.as_slice().unwrap(),
    );

    let tema10 = tema(close, 10).unwrap();
    write_series_csv(
        &dir.join("tema_10.csv"),
        "tema",
        tema10.as_slice().unwrap(),
    );

    let obv_vals = obv(close, &data.volume).unwrap();
    write_series_csv(&dir.join("obv.csv"), "obv", obv_vals.as_slice().unwrap());

    // -------- Classic chart patterns (FTA-native) --------

    let ha = heikin_ashi(&data.open, &data.high, &data.low, close).unwrap();
    write_multi_series_csv(
        &dir.join("heikin_ashi.csv"),
        &["ha_open", "ha_high", "ha_low", "ha_close"],
        &[
            ha.ha_open.as_slice().unwrap(),
            ha.ha_high.as_slice().unwrap(),
            ha.ha_low.as_slice().unwrap(),
            ha.ha_close.as_slice().unwrap(),
        ],
    );

    let darvas = darvas_box(&data.high, &data.low, close, 5, 3).unwrap();
    write_multi_series_csv(
        &dir.join("darvas_box_5_3.csv"),
        &["box_top", "box_bottom", "signal"],
        &[
            darvas.box_top.as_slice().unwrap(),
            darvas.box_bottom.as_slice().unwrap(),
            &darvas.signal.iter().map(|v| *v as f64).collect::<Vec<_>>(),
        ],
    );

    let ren = renko(&data.high, &data.low, 5.0).unwrap();
    write_multi_series_csv(
        &dir.join("renko_5.csv"),
        &["brick", "direction"],
        &[
            ren.bricks.as_slice().unwrap(),
            &ren.direction.iter().map(|v| *v as f64).collect::<Vec<_>>(),
        ],
    );

    let kg = kagi(close, 50.0).unwrap();
    write_multi_series_csv(
        &dir.join("kagi_50.csv"),
        &["kagi", "direction"],
        &[
            kg.kagi.as_slice().unwrap(),
            &kg.direction.iter().map(|v| *v as f64).collect::<Vec<_>>(),
        ],
    );

    let pnf_res = point_and_figure(&data.high, &data.low, 5.0, 3).unwrap();
    write_multi_series_csv(
        &dir.join("pnf_5_3.csv"),
        &["pnf", "col", "new_col"],
        &[
            pnf_res.pnf.as_slice().unwrap(),
            &pnf_res.column_type.iter().map(|v| *v as f64).collect::<Vec<_>>(),
            &pnf_res.new_column.iter().map(|v| *v as f64).collect::<Vec<_>>(),
        ],
    );

    let tlb = three_line_break(close, 3).unwrap();
    write_multi_series_csv(
        &dir.join("three_line_break_3.csv"),
        &["line", "direction"],
        &[
            tlb.line.as_slice().unwrap(),
            &tlb.direction.iter().map(|v| *v as f64).collect::<Vec<_>>(),
        ],
    );

    let alligator = williams_alligator(close).unwrap();
    write_multi_series_csv(
        &dir.join("williams_alligator.csv"),
        &["jaw", "teeth", "lips"],
        &[
            alligator.jaw.as_slice().unwrap(),
            alligator.teeth.as_slice().unwrap(),
            alligator.lips.as_slice().unwrap(),
        ],
    );
}

#[test]
fn golden_sma_10() {
    let data = load_ohlcv();
    let computed = sma(&data.close, 10).unwrap();
    let expected = read_series_csv(&golden_dir().join("sma_10.csv"));
    assert_series_eq(computed.as_slice().unwrap(), &expected, "SMA(10)");
}

#[test]
fn golden_sma_20() {
    let data = load_ohlcv();
    let computed = sma(&data.close, 20).unwrap();
    let expected = read_series_csv(&golden_dir().join("sma_20.csv"));
    assert_series_eq(computed.as_slice().unwrap(), &expected, "SMA(20)");
}

#[test]
fn golden_ema_10() {
    let data = load_ohlcv();
    let computed = ema(&data.close, 10).unwrap();
    let expected = read_series_csv(&golden_dir().join("ema_10.csv"));
    assert_series_eq(computed.as_slice().unwrap(), &expected, "EMA(10)");
}

#[test]
fn golden_ema_20() {
    let data = load_ohlcv();
    let computed = ema(&data.close, 20).unwrap();
    let expected = read_series_csv(&golden_dir().join("ema_20.csv"));
    assert_series_eq(computed.as_slice().unwrap(), &expected, "EMA(20)");
}

#[test]
fn golden_rsi_14() {
    let data = load_ohlcv();
    let computed = rsi(&data.close, 14).unwrap();
    let expected = read_series_csv(&golden_dir().join("rsi_14.csv"));
    assert_series_eq(computed.as_slice().unwrap(), &expected, "RSI(14)");
}

#[test]
fn golden_macd_12_26_9() {
    let data = load_ohlcv();
    let computed = macd(&data.close, 12, 26, 9).unwrap();
    let expected = read_multi_series_csv(&golden_dir().join("macd_12_26_9.csv"));
    assert_series_eq(computed.macd.as_slice().unwrap(), &expected[0], "MACD line");
    assert_series_eq(
        computed.signal.as_slice().unwrap(),
        &expected[1],
        "MACD signal",
    );
    assert_series_eq(computed.hist.as_slice().unwrap(), &expected[2], "MACD hist");
}

#[test]
fn golden_bbands_20_2() {
    let data = load_ohlcv();
    let computed = bbands(&data.close, 20, 2.0, 2.0).unwrap();
    let expected = read_multi_series_csv(&golden_dir().join("bbands_20_2.csv"));
    assert_series_eq(computed.upper.as_slice().unwrap(), &expected[0], "BB upper");
    assert_series_eq(
        computed.middle.as_slice().unwrap(),
        &expected[1],
        "BB middle",
    );
    assert_series_eq(computed.lower.as_slice().unwrap(), &expected[2], "BB lower");
}

#[test]
fn golden_atr_14() {
    let data = load_ohlcv();
    let computed = atr(&data.high, &data.low, &data.close, 14).unwrap();
    let expected = read_series_csv(&golden_dir().join("atr_14.csv"));
    assert_series_eq(computed.as_slice().unwrap(), &expected, "ATR(14)");
}

#[test]
fn golden_stoch_14_3() {
    let data = load_ohlcv();
    let computed = stoch(&data.high, &data.low, &data.close, 14, 3, 3).unwrap();
    let expected = read_multi_series_csv(&golden_dir().join("stoch_14_3.csv"));
    assert_series_eq(computed.k.as_slice().unwrap(), &expected[0], "Stoch %K");
    assert_series_eq(computed.d.as_slice().unwrap(), &expected[1], "Stoch %D");
}

#[test]
fn golden_vwap() {
    let data = load_ohlcv();
    let computed = vwap(&data.high, &data.low, &data.close, &data.volume).unwrap();
    let expected = read_series_csv(&golden_dir().join("vwap.csv"));
    assert_series_eq(computed.as_slice().unwrap(), &expected, "VWAP");
}

#[test]
fn golden_adx_14() {
    let data = load_ohlcv();
    let computed = adx(&data.high, &data.low, &data.close, 14).unwrap();
    let expected = read_series_csv(&golden_dir().join("adx_14.csv"));
    assert_series_eq(computed.as_slice().unwrap(), &expected, "ADX(14)");
}

#[test]
fn golden_willr_14() {
    let data = load_ohlcv();
    let computed = willr(&data.high, &data.low, &data.close, 14).unwrap();
    let expected = read_series_csv(&golden_dir().join("willr_14.csv"));
    assert_series_eq(computed.as_slice().unwrap(), &expected, "WILLR(14)");
}

#[test]
fn golden_mom_10() {
    let data = load_ohlcv();
    let computed = mom(&data.close, 10).unwrap();
    let expected = read_series_csv(&golden_dir().join("mom_10.csv"));
    assert_series_eq(computed.as_slice().unwrap(), &expected, "MOM(10)");
}

#[test]
fn golden_roc_10() {
    let data = load_ohlcv();
    let computed = roc(&data.close, 10).unwrap();
    let expected = read_series_csv(&golden_dir().join("roc_10.csv"));
    assert_series_eq(computed.as_slice().unwrap(), &expected, "ROC(10)");
}

#[test]
fn golden_aroon_14() {
    let data = load_ohlcv();
    let computed = aroon(&data.high, &data.low, 14).unwrap();
    let expected = read_multi_series_csv(&golden_dir().join("aroon_14.csv"));
    assert_series_eq(
        computed.aroon_up.as_slice().unwrap(),
        &expected[0],
        "Aroon up",
    );
    assert_series_eq(
        computed.aroon_down.as_slice().unwrap(),
        &expected[1],
        "Aroon down",
    );
}

#[test]
fn golden_cci_14() {
    let data = load_ohlcv();
    let computed = cci(&data.high, &data.low, &data.close, 14).unwrap();
    let expected = read_series_csv(&golden_dir().join("cci_14.csv"));
    assert_series_eq(computed.as_slice().unwrap(), &expected, "CCI(14)");
}

#[test]
fn golden_apo_12_26() {
    let data = load_ohlcv();
    let computed = apo(&data.close, 12, 26).unwrap();
    let expected = read_series_csv(&golden_dir().join("apo_12_26.csv"));
    assert_series_eq(computed.as_slice().unwrap(), &expected, "APO(12,26)");
}

#[test]
fn golden_cmo_14() {
    let data = load_ohlcv();
    let computed = cmo(&data.close, 14).unwrap();
    let expected = read_series_csv(&golden_dir().join("cmo_14.csv"));
    assert_series_eq(computed.as_slice().unwrap(), &expected, "CMO(14)");
}

#[test]
fn golden_trix_14() {
    let data = load_ohlcv();
    let computed = trix(&data.close, 14).unwrap();
    let expected = read_series_csv(&golden_dir().join("trix_14.csv"));
    assert_series_eq(computed.as_slice().unwrap(), &expected, "TRIX(14)");
}

#[test]
fn golden_natr_14() {
    let data = load_ohlcv();
    let computed = natr(&data.high, &data.low, &data.close, 14).unwrap();
    let expected = read_series_csv(&golden_dir().join("natr_14.csv"));
    assert_series_eq(computed.as_slice().unwrap(), &expected, "NATR(14)");
}

#[test]
fn golden_trange() {
    let data = load_ohlcv();
    let computed = trange(&data.high, &data.low, &data.close).unwrap();
    let expected = read_series_csv(&golden_dir().join("trange.csv"));
    assert_series_eq(computed.as_slice().unwrap(), &expected, "TRANGE");
}

#[test]
fn golden_wma_10() {
    let data = load_ohlcv();
    let computed = wma(&data.close, 10).unwrap();
    let expected = read_series_csv(&golden_dir().join("wma_10.csv"));
    assert_series_eq(computed.as_slice().unwrap(), &expected, "WMA(10)");
}

#[test]
fn golden_dema_10() {
    let data = load_ohlcv();
    let computed = dema(&data.close, 10).unwrap();
    let expected = read_series_csv(&golden_dir().join("dema_10.csv"));
    assert_series_eq(computed.as_slice().unwrap(), &expected, "DEMA(10)");
}

#[test]
fn golden_tema_10() {
    let data = load_ohlcv();
    let computed = tema(&data.close, 10).unwrap();
    let expected = read_series_csv(&golden_dir().join("tema_10.csv"));
    assert_series_eq(computed.as_slice().unwrap(), &expected, "TEMA(10)");
}

#[test]
fn golden_obv() {
    let data = load_ohlcv();
    let computed = obv(&data.close, &data.volume).unwrap();
    let expected = read_series_csv(&golden_dir().join("obv.csv"));
    assert_series_eq(computed.as_slice().unwrap(), &expected, "OBV");
}

// -------- Classic chart pattern golden tests (FTA-native) --------

fn close_to_int(values: &[f64]) -> Vec<i32> {
    values
        .iter()
        .map(|v| if v.is_nan() { 0 } else { v.round() as i32 })
        .collect()
}

#[test]
fn golden_heikin_ashi() {
    let data = load_ohlcv();
    let computed = heikin_ashi(&data.open, &data.high, &data.low, &data.close).unwrap();
    let expected = read_multi_series_csv(&golden_dir().join("heikin_ashi.csv"));
    assert_series_eq(computed.ha_open.as_slice().unwrap(), &expected[0], "HA open");
    assert_series_eq(computed.ha_high.as_slice().unwrap(), &expected[1], "HA high");
    assert_series_eq(computed.ha_low.as_slice().unwrap(), &expected[2], "HA low");
    assert_series_eq(computed.ha_close.as_slice().unwrap(), &expected[3], "HA close");
}

#[test]
fn golden_darvas_box() {
    let data = load_ohlcv();
    let computed = darvas_box(&data.high, &data.low, &data.close, 5, 3).unwrap();
    let expected = read_multi_series_csv(&golden_dir().join("darvas_box_5_3.csv"));
    assert_series_eq(computed.box_top.as_slice().unwrap(), &expected[0], "Darvas top");
    assert_series_eq(computed.box_bottom.as_slice().unwrap(), &expected[1], "Darvas bottom");
    assert_eq!(close_to_int(&expected[2]), computed.signal.to_vec());
}

#[test]
fn golden_renko() {
    let data = load_ohlcv();
    let computed = renko(&data.high, &data.low, 5.0).unwrap();
    let expected = read_multi_series_csv(&golden_dir().join("renko_5.csv"));
    assert_series_eq(computed.bricks.as_slice().unwrap(), &expected[0], "Renko brick");
    assert_eq!(close_to_int(&expected[1]), computed.direction.to_vec());
}

#[test]
fn golden_kagi() {
    let data = load_ohlcv();
    let computed = kagi(&data.close, 50.0).unwrap();
    let expected = read_multi_series_csv(&golden_dir().join("kagi_50.csv"));
    assert_series_eq(computed.kagi.as_slice().unwrap(), &expected[0], "Kagi line");
    assert_eq!(close_to_int(&expected[1]), computed.direction.to_vec());
}

#[test]
fn golden_pnf() {
    let data = load_ohlcv();
    let computed = point_and_figure(&data.high, &data.low, 5.0, 3).unwrap();
    let expected = read_multi_series_csv(&golden_dir().join("pnf_5_3.csv"));
    assert_series_eq(computed.pnf.as_slice().unwrap(), &expected[0], "PnF value");
    assert_eq!(close_to_int(&expected[1]), computed.column_type.to_vec());
    assert_eq!(close_to_int(&expected[2]), computed.new_column.to_vec());
}

#[test]
fn golden_three_line_break() {
    let data = load_ohlcv();
    let computed = three_line_break(&data.close, 3).unwrap();
    let expected = read_multi_series_csv(&golden_dir().join("three_line_break_3.csv"));
    assert_series_eq(computed.line.as_slice().unwrap(), &expected[0], "TLB line");
    assert_eq!(close_to_int(&expected[1]), computed.direction.to_vec());
}

#[test]
fn golden_williams_alligator() {
    let data = load_ohlcv();
    let computed = williams_alligator(&data.close).unwrap();
    let expected = read_multi_series_csv(&golden_dir().join("williams_alligator.csv"));
    assert_series_eq(computed.jaw.as_slice().unwrap(), &expected[0], "Alligator jaw");
    assert_series_eq(computed.teeth.as_slice().unwrap(), &expected[1], "Alligator teeth");
    assert_series_eq(computed.lips.as_slice().unwrap(), &expected[2], "Alligator lips");
}

#[test]
fn golden_input_ohlcv_matches_generator() {
    let from_csv = load_ohlcv();
    let generated = generate_input_data();
    assert_series_eq(&from_csv.open, &generated.open, "input open");
    assert_series_eq(&from_csv.high, &generated.high, "input high");
    assert_series_eq(&from_csv.low, &generated.low, "input low");
    assert_series_eq(&from_csv.close, &generated.close, "input close");
    assert_series_eq(&from_csv.volume, &generated.volume, "input volume");
}

/// Snapshot (regression) guard for the `stochrsi` SIMD SMA refactor.
///
/// The baseline `%K`/`%D` values were captured from the ORIGINAL scalar
/// `sma_nan_as_zero_into` implementation (via a `--nocapture` capture run)
/// before swapping the two smoothing passes to the SIMD `simd_sma` kernel.
/// Locking these numbers ensures the SIMD path cannot silently drift from the
/// scalar semantics (NaN→0.0 warm-up handling preserved). Deterministic
/// synthetic input, no RNG.
#[test]
fn golden_stochrsi_simd_snapshot() {
    let n = 64usize;
    let input: Vec<f64> = (0..n)
        .map(|i| {
            let x = i as f64;
            100.0
                + 10.0 * (x * 0.35).sin()
                + 4.0 * (x * 0.11).cos()
                + 1.5 * ((x * 7.0) as i64 as f64 * 0.13).sin()
        })
        .collect();

    let res: StochResult = stochrsi(&input, 14, 14, 3, 3).unwrap();
    let k = res.k.as_slice().unwrap();
    let d = res.d.as_slice().unwrap();

    let nan = f64::NAN;
    let expected_k: Vec<f64> = vec![
        nan, nan, nan, nan, nan, nan, nan, nan, nan, nan, nan, nan, nan, nan, nan, nan, nan, nan,
        nan, nan, nan, nan, nan, nan, nan, nan, nan, nan, nan, 14.633402877996966,
        4.040600005430504, 1.1842378929335002e-15, 1.1842378929335002e-15, 2.773044096232221,
        13.20080246600195, 32.532907454834174, 56.94158982838073, 79.84716479194432,
        93.84839313644542, 100.0, 100.0, 100.0, 100.0, 97.64431767995325, 89.52551617881227,
        74.61714438584893, 55.101587417353855, 31.21104684274501, 12.786085302375007,
        2.8537439365836477, 5.8969785578966745, 13.07199011545687, 22.040125751659662,
        33.45684092468276, 49.43942696002972, 69.68160920588782, 87.23100148730151,
        97.40673722772767, 99.99999999999997, 96.35228170360503, 87.24802148653896,
        73.16304915055302, 57.818618947601635, 41.60051096537618,
    ];
    let expected_d: Vec<f64> = vec![
        nan, nan, nan, nan, nan, nan, nan, nan, nan, nan, nan, nan, nan, nan, nan, nan, nan, nan,
        nan, nan, nan, nan, nan, nan, nan, nan, nan, nan, nan, nan, nan, 6.224667627809156,
        1.3468666684768351, 0.9243480320774076, 5.324615520744723, 16.16891800568945,
        34.22509991640562, 56.440554025053075, 76.87904925225682, 91.23185264279658,
        97.94946437881515, 100.0, 100.0, 99.21477255998441, 95.72327795292183, 87.26232608153813,
        73.08141599400501, 53.64325954864927, 33.032906520824625, 15.616958693901227,
        7.178935932285115, 7.274237536645736, 13.669698141671073, 22.85631893059977,
        34.978797878790715, 50.85929236353344, 68.78401255107303, 84.77311597363901,
        94.87924623834306, 97.91967297711088, 94.53343439671465, 85.58778411356566,
        72.74322986156453, 57.527393021176934,
    ];

    assert_eq!(k.len(), expected_k.len());
    assert_eq!(d.len(), expected_d.len());
    assert_series_eq(k, &expected_k, "stochrsi %K (SIMD snapshot)");
    assert_series_eq(d, &expected_d, "stochrsi %D (SIMD snapshot)");
}
