use finkit::streaming::{
    indicators::{
        StreamingAtr, StreamingBoll, StreamingCci, StreamingEma, StreamingMacd, StreamingMom,
        StreamingRoc, StreamingRsi, StreamingSma,
    },
    Ohlcv, OhlcvBar, StreamingIndicator,
};

fn make_bar(close: f64) -> OhlcvBar {
    OhlcvBar::new(close, close + 1.0, close - 1.0, close, 1000.0)
}

fn bar_hlc(bar: &OhlcvBar) -> (f64, f64, f64) {
    (bar.high(), bar.low(), bar.close())
}

#[test]
fn test_sma_clone_snapshot_restore() {
    let mut sma = StreamingSma::new(3);
    sma.next(1.0);
    sma.next(2.0);
    let snapshot = sma.clone();
    sma.next(3.0);
    let val = sma.value();
    let mut restored = snapshot;
    restored.next(3.0);
    assert_eq!(restored.value(), val);
}

#[test]
fn test_ema_clone_snapshot_restore() {
    let mut ema = StreamingEma::new(3);
    ema.next(10.0);
    ema.next(20.0);
    let snapshot = ema.clone();
    ema.next(30.0);
    let val = ema.value();
    let mut restored = snapshot;
    restored.next(30.0);
    assert_eq!(restored.value(), val);
}

#[test]
fn test_rsi_clone_snapshot_restore() {
    let mut rsi = StreamingRsi::new(5);
    let data = vec![44.0, 44.25, 44.5, 43.75, 44.5, 44.25, 44.0];
    for &v in &data {
        rsi.next(v);
    }
    let snapshot = rsi.clone();
    rsi.next(45.0);
    let val = rsi.value();
    let mut restored = snapshot;
    restored.next(45.0);
    assert_eq!(restored.value(), val);
}

#[test]
fn test_macd_clone_snapshot_restore() {
    let mut macd = StreamingMacd::new(12, 26, 9);
    let data: Vec<f64> = (1..=30).map(|x| x as f64).collect();
    for &v in &data {
        macd.next(v);
    }
    let snapshot = macd.clone();
    macd.next(31.0);
    let val = macd.value();
    let mut restored = snapshot;
    restored.next(31.0);
    assert_eq!(
        restored.value().map(|v| v.macd),
        val.map(|v| v.macd)
    );
}

#[test]
fn test_boll_clone_snapshot_restore() {
    let mut boll = StreamingBoll::new(5, 2.0, 2.0);
    let data = vec![10.0, 11.0, 12.0, 11.5, 10.5, 11.0, 12.5];
    for &v in &data {
        boll.next(v);
    }
    let snapshot = boll.clone();
    boll.next(13.0);
    let val = boll.value();
    let mut restored = snapshot;
    restored.next(13.0);
    assert_eq!(
        restored.value().map(|v| v.upper),
        val.map(|v| v.upper)
    );
}

#[test]
fn test_atr_clone_snapshot_restore() {
    let mut atr = StreamingAtr::new(5);
    let data: Vec<f64> = vec![44.0, 44.25, 44.5, 43.75, 44.5, 44.25, 44.0];
    for &v in &data {
        let bar = make_bar(v);
        atr.next(bar_hlc(&bar));
    }
    let snapshot = atr.clone();
    atr.next(bar_hlc(&make_bar(45.0)));
    let val = atr.value();
    let mut restored = snapshot;
    restored.next(bar_hlc(&make_bar(45.0)));
    assert_eq!(restored.value(), val);
}

#[test]
fn test_cci_clone_snapshot_restore() {
    let mut cci = StreamingCci::new(5);
    let data: Vec<f64> = vec![44.0, 44.25, 44.5, 43.75, 44.5, 44.25, 44.0];
    for &v in &data {
        let bar = make_bar(v);
        cci.next(bar_hlc(&bar));
    }
    let snapshot = cci.clone();
    cci.next(bar_hlc(&make_bar(45.0)));
    let val = cci.value();
    let mut restored = snapshot;
    restored.next(bar_hlc(&make_bar(45.0)));
    assert_eq!(restored.value(), val);
}

#[test]
fn test_roc_clone_snapshot_restore() {
    let mut roc = StreamingRoc::new(3);
    for v in [10.0, 11.0, 12.0, 13.0, 14.0] {
        roc.next(v);
    }
    let snapshot = roc.clone();
    roc.next(15.0);
    let val = roc.value();
    let mut restored = snapshot;
    restored.next(15.0);
    assert_eq!(restored.value(), val);
}

#[test]
fn test_mom_clone_snapshot_restore() {
    let mut mom = StreamingMom::new(3);
    for v in [10.0, 11.0, 12.0, 13.0, 14.0] {
        mom.next(v);
    }
    let snapshot = mom.clone();
    mom.next(15.0);
    let val = mom.value();
    let mut restored = snapshot;
    restored.next(15.0);
    assert_eq!(restored.value(), val);
}

#[test]
fn test_clone_reset_independence() {
    let mut sma = StreamingSma::new(3);
    sma.next(1.0);
    sma.next(2.0);
    let mut clone = sma.clone();
    sma.reset();
    assert_eq!(sma.value(), None);
    clone.next(3.0);
    assert!(clone.value().is_some());
}

// ============================================================================
// next_with_time repaint tests
// ============================================================================

#[test]
fn test_sma_next_with_time_basic_repaint() {
    let mut sma = StreamingSma::new(3);
    sma.next_with_time(1.0, 1000);
    sma.next_with_time(2.0, 2000);
    sma.next_with_time(10.0, 3000);
    sma.next_with_time(20.0, 3000); // repaint
    let result = sma.next_with_time(3.0, 3000); // repaint again

    let mut clean = StreamingSma::new(3);
    clean.next(1.0);
    clean.next(2.0);
    let expected = clean.next(3.0);

    assert_eq!(result, expected);
}

#[test]
fn test_sma_next_with_time_no_repaint_different_times() {
    let mut sma = StreamingSma::new(3);
    sma.next_with_time(1.0, 1000);
    sma.next_with_time(2.0, 2000);
    let r1 = sma.next_with_time(3.0, 3000);
    let r2 = sma.next_with_time(4.0, 4000); // different time, not repaint

    let mut clean = StreamingSma::new(3);
    clean.next(1.0);
    clean.next(2.0);
    clean.next(3.0);
    let expected = clean.next(4.0);

    assert_eq!(r1, Some(2.0));
    assert_eq!(r2, expected);
}

#[test]
fn test_sma_next_with_time_zero_timestamp_no_repaint() {
    let mut sma = StreamingSma::new(3);
    sma.next_with_time(1.0, 0);
    sma.next_with_time(2.0, 0);
    let result = sma.next_with_time(3.0, 0);
    assert_eq!(result, Some(2.0));
    let result2 = sma.next_with_time(4.0, 0);
    assert_eq!(result2, Some(3.0));
}

#[test]
fn test_ema_compute_bar_repaint() {
    let mut ema = StreamingEma::new(3);
    ema.compute_bar(&OhlcvBar::new_with_time(0.0, 0.0, 0.0, 1.0, 0.0, 1000));
    ema.compute_bar(&OhlcvBar::new_with_time(0.0, 0.0, 0.0, 2.0, 0.0, 2000));
    ema.compute_bar(&OhlcvBar::new_with_time(0.0, 0.0, 0.0, 99.0, 0.0, 3000));
    let result = ema.compute_bar(&OhlcvBar::new_with_time(0.0, 0.0, 0.0, 3.0, 0.0, 3000));

    let mut clean = StreamingEma::new(3);
    clean.next(1.0);
    clean.next(2.0);
    let expected = clean.next(3.0);
    assert_eq!(result, expected);
}

#[test]
fn test_rsi_compute_bar_repaint() {
    let mut rsi = StreamingRsi::new(5);
    let bars: Vec<f64> = vec![44.0, 44.25, 44.5, 43.75, 44.5, 44.25];
    for (i, &v) in bars.iter().enumerate() {
        rsi.compute_bar(&OhlcvBar::new_with_time(0.0, 0.0, 0.0, v, 0.0, (i + 1) as i64 * 1000));
    }
    rsi.compute_bar(&OhlcvBar::new_with_time(0.0, 0.0, 0.0, 99.0, 0.0, 7000));
    let result = rsi.compute_bar(&OhlcvBar::new_with_time(0.0, 0.0, 0.0, 44.0, 0.0, 7000));

    let mut clean = StreamingRsi::new(5);
    for &v in &bars {
        clean.next(v);
    }
    let expected = clean.next(44.0);
    assert!((result.unwrap() - expected.unwrap()).abs() < 1e-10);
}

#[test]
fn test_sma_repaint_multiple_consecutive() {
    let mut sma = StreamingSma::new(3);
    sma.next_with_time(10.0, 1000);
    sma.next_with_time(20.0, 2000);
    // 5 consecutive repaints on same bar
    for i in 1..=5 {
        sma.next_with_time(i as f64 * 100.0, 3000);
    }
    let result = sma.next_with_time(30.0, 3000);

    let mut clean = StreamingSma::new(3);
    clean.next(10.0);
    clean.next(20.0);
    let expected = clean.next(30.0);
    assert_eq!(result, expected);
}

#[test]
fn test_sma_repaint_then_new_bar() {
    let mut sma = StreamingSma::new(3);
    sma.next_with_time(1.0, 1000);
    sma.next_with_time(2.0, 2000);
    sma.next_with_time(3.0, 3000); // forming
    sma.next_with_time(3.5, 3000); // repaint
    let _ = sma.next_with_time(3.0, 3000); // repaint final
    let result = sma.next_with_time(4.0, 4000); // new bar

    let mut clean = StreamingSma::new(3);
    clean.next(1.0);
    clean.next(2.0);
    clean.next(3.0);
    let expected = clean.next(4.0);
    assert_eq!(result, expected);
}
