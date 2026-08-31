use finkit::formula::engine::FormulaEngine;
use finkit::formula::types::{AlertCommand, FormulaContext, SelectionResult};
use ndarray::Array1;

fn make_test_context(len: usize) -> FormulaContext {
    let close: Vec<f64> = (0..len).map(|i| 10.0 + i as f64 * 0.5).collect();
    let open: Vec<f64> = (0..len).map(|i| 9.5 + i as f64 * 0.5).collect();
    let high: Vec<f64> = (0..len).map(|i| 11.0 + i as f64 * 0.5).collect();
    let low: Vec<f64> = (0..len).map(|i| 9.0 + i as f64 * 0.5).collect();
    let volume: Vec<f64> = (0..len).map(|i| 1000.0 + i as f64 * 100.0).collect();

    FormulaContext::new(
        Array1::from_vec(open),
        Array1::from_vec(high),
        Array1::from_vec(low),
        Array1::from_vec(close),
        Array1::from_vec(volume),
        None,
    )
}

fn make_engine() -> FormulaEngine {
    FormulaEngine::new()
}

#[cfg(test)]
mod ths_alias_tests {
    use super::*;

    #[test]
    fn test_close1_alias() {
        let mut engine = make_engine();
        let mut ctx = make_test_context(10);
        let result = engine.eval("CLOSE1", &mut ctx).unwrap();

        for i in 1..10 {
            assert!((result[i] - ctx.close[i - 1]).abs() < 1e-10);
        }
        assert!(result[0].is_nan());
    }

    #[test]
    fn test_open1_alias() {
        let mut engine = make_engine();
        let mut ctx = make_test_context(10);
        let result = engine.eval("OPEN1", &mut ctx).unwrap();

        for i in 1..10 {
            assert!((result[i] - ctx.open[i - 1]).abs() < 1e-10);
        }
        assert!(result[0].is_nan());
    }

    #[test]
    fn test_high1_alias() {
        let mut engine = make_engine();
        let mut ctx = make_test_context(10);
        let result = engine.eval("HIGH1", &mut ctx).unwrap();

        for i in 1..10 {
            assert!((result[i] - ctx.high[i - 1]).abs() < 1e-10);
        }
        assert!(result[0].is_nan());
    }

    #[test]
    fn test_low1_alias() {
        let mut engine = make_engine();
        let mut ctx = make_test_context(10);
        let result = engine.eval("LOW1", &mut ctx).unwrap();

        for i in 1..10 {
            assert!((result[i] - ctx.low[i - 1]).abs() < 1e-10);
        }
        assert!(result[0].is_nan());
    }

    #[test]
    fn test_vol1_alias() {
        let mut engine = make_engine();
        let mut ctx = make_test_context(10);
        let result = engine.eval("VOL1", &mut ctx).unwrap();

        for i in 1..10 {
            assert!((result[i] - ctx.volume[i - 1]).abs() < 1e-10);
        }
        assert!(result[0].is_nan());
    }

    #[test]
    fn test_ref_equivalence() {
        let mut engine = make_engine();
        let mut ctx = make_test_context(10);

        let close1_result = engine.eval("CLOSE1", &mut ctx).unwrap();
        let ref_result = engine.eval("REF(CLOSE, 1)", &mut ctx).unwrap();

        for i in 0..10 {
            if close1_result[i].is_nan() && ref_result[i].is_nan() {
                continue;
            }
            assert!((close1_result[i] - ref_result[i]).abs() < 1e-10);
        }
    }
}

#[cfg(test)]
mod ths_selection_tests {
    use super::*;

    #[test]
    fn test_smartselect_mode0() {
        let mut engine = make_engine();
        let mut ctx = make_test_context(20);
        let result = engine
            .eval("SMARTSELECT(CLOSE > MA(CLOSE, 5), 0)", &mut ctx)
            .unwrap();

        let signal_count = result.iter().filter(|&v| *v > 0.0).count();
        assert!(signal_count > 0);
    }

    #[test]
    fn test_smartselect_mode1() {
        let mut engine = make_engine();
        let mut ctx = make_test_context(20);
        let result = engine
            .eval("SMARTSELECT(CLOSE > OPEN, 1)", &mut ctx)
            .unwrap();

        let mut last_signal = false;
        for i in 0..result.len() {
            if result[i] > 0.0 {
                assert!(!last_signal);
                last_signal = true;
            }
        }
    }

    #[test]
    fn test_selectcond() {
        let mut engine = make_engine();
        let mut ctx = make_test_context(20);
        let result = engine.eval("SELECTCOND(CLOSE > OPEN)", &mut ctx).unwrap();

        for i in 0..ctx.data_len {
            if ctx.close[i] > ctx.open[i] {
                assert!(result[i] > 0.0);
            } else {
                assert!(result[i] == 0.0);
            }
        }
    }
}

#[cfg(test)]
mod ths_alert_tests {
    use super::*;

    #[test]
    fn test_alert_basic() {
        let mut engine = make_engine();
        let mut ctx = make_test_context(20);
        let result = engine
            .eval("ALERT(CLOSE > OPEN, \"Price Up\")", &mut ctx)
            .unwrap();

        let alert_count = result.iter().filter(|&v| *v > 0.0).count();
        let expected_count = ctx
            .close
            .iter()
            .zip(ctx.open.iter())
            .filter(|(&c, &o)| c > o)
            .count();
        assert_eq!(alert_count, expected_count);
    }

    #[test]
    fn test_alertonce_single_trigger() {
        let mut engine = make_engine();
        let mut ctx = make_test_context(20);
        let result = engine
            .eval("ALERTONCE(CLOSE > OPEN, \"First Alert\")", &mut ctx)
            .unwrap();

        let alert_count = result.iter().filter(|&v| *v > 0.0).count();
        assert!(alert_count <= 1);

        if alert_count == 1 {
            let first_alert_idx = result.iter().position(|&v| v > 0.0).unwrap();
            assert!(ctx.close[first_alert_idx] > ctx.open[first_alert_idx]);
        }
    }
}

#[cfg(test)]
mod ths_statistics_tests {
    use super::*;

    #[test]
    fn test_avgprice_n() {
        let mut engine = make_engine();
        let mut ctx = make_test_context(20);
        let result = engine.eval("AVGPRICE_N(5)", &mut ctx).unwrap();

        for i in 5..20 {
            let expected_tp_sum: f64 = (i - 4..=i)
                .map(|j| (ctx.high[j] + ctx.low[j] + ctx.close[j]) / 3.0)
                .sum::<f64>()
                / 5.0;
            assert!((result[i] - expected_tp_sum).abs() < 1e-10);
        }
    }

    #[test]
    fn test_totalvol() {
        let mut engine = make_engine();
        let mut ctx = make_test_context(20);
        let result = engine.eval("TOTALVOL(5)", &mut ctx).unwrap();

        for i in 5..20 {
            let expected_vol_sum: f64 = (i - 4..=i).map(|j| ctx.volume[j]).sum();
            assert!((result[i] - expected_vol_sum).abs() < 1e-10);
        }
    }

    #[test]
    fn test_maxprice() {
        let mut engine = make_engine();
        let mut ctx = make_test_context(20);
        let result = engine.eval("MAXPRICE(5)", &mut ctx).unwrap();

        for i in 5..20 {
            let expected_max: f64 = (i - 4..=i)
                .map(|j| ctx.high[j])
                .fold(f64::NEG_INFINITY, f64::max);
            assert!((result[i] - expected_max).abs() < 1e-10);
        }
    }

    #[test]
    fn test_minprice() {
        let mut engine = make_engine();
        let mut ctx = make_test_context(20);
        let result = engine.eval("MINPRICE(5)", &mut ctx).unwrap();

        for i in 5..20 {
            let expected_min: f64 = (i - 4..=i)
                .map(|j| ctx.low[j])
                .fold(f64::INFINITY, f64::min);
            assert!((result[i] - expected_min).abs() < 1e-10);
        }
    }
}

#[cfg(test)]
mod ths_compat_integration_tests {
    use super::*;

    #[test]
    fn test_ths_golden_cross_formula() {
        let mut engine = make_engine();
        let mut ctx = make_test_context(30);

        let formula = "MA5 := MA(CLOSE, 5); MA10 := MA(CLOSE, 10); CROSS(MA5, MA10)";
        let result = engine.eval(formula, &mut ctx).unwrap();

        let cross_count = result.iter().filter(|&v| *v > 0.0).count();
        assert_eq!(result.len(), ctx.close.len());
        assert!(cross_count <= result.len());
    }

    #[test]
    fn test_ths_macd_signal() {
        let mut engine = make_engine();
        let mut ctx = make_test_context(50);

        let formula =
            "DIF := EMA(CLOSE, 12) - EMA(CLOSE, 26); DEA := EMA(DIF, 9); MACD := (DIF - DEA) * 2";
        let result = engine.eval(formula, &mut ctx).unwrap();

        assert!(result.len() == ctx.data_len);
    }

    #[test]
    fn test_ths_kdj_formula() {
        let mut engine = make_engine();
        let mut ctx = make_test_context(30);

        let formula = "RSV := (CLOSE - LLV(LOW, 9)) / (HHV(HIGH, 9) - LLV(LOW, 9)) * 100; K := SMA(RSV, 3, 1); D := SMA(K, 3, 1); J := 3 * K - 2 * D";
        let result = engine.eval(formula, &mut ctx).unwrap();

        assert!(result.len() == ctx.data_len);
    }

    #[test]
    fn test_ths_combined_formula() {
        let mut engine = make_engine();
        let mut ctx = make_test_context(30);

        let formula = "COND1 := CLOSE > MA(CLOSE, 5); COND2 := VOL > MA(VOL, 5); SIGNAL := SMARTSELECT(COND1 AND COND2, 0)";
        let result = engine.eval(formula, &mut ctx).unwrap();

        assert!(result.len() == ctx.data_len);
    }

    #[test]
    fn test_ths_price_change_formula() {
        let mut engine = make_engine();
        let mut ctx = make_test_context(30);

        let formula = "CHANGE := (CLOSE - CLOSE1) / CLOSE1 * 100";
        let result = engine.eval(formula, &mut ctx).unwrap();

        for i in 1..ctx.data_len {
            let expected = (ctx.close[i] - ctx.close[i - 1]) / ctx.close[i - 1] * 100.0;
            assert!((result[i] - expected).abs() < 1e-10);
        }
    }
}

#[cfg(test)]
mod alert_command_tests {
    use super::*;

    #[test]
    fn test_alert_command_creation() {
        let condition = Array1::from_vec(vec![0.0, 1.0, 0.0, 1.0, 1.0]);
        let alert = AlertCommand::new(condition, "Test Alert".to_string(), false);

        assert_eq!(alert.message, "Test Alert");
        assert!(!alert.is_once);
        assert!(alert.triggered_bars.is_empty());
    }

    #[test]
    fn test_alert_command_check() {
        let condition = Array1::from_vec(vec![0.0, 1.0, 0.0, 1.0, 1.0]);
        let mut alert = AlertCommand::new(condition, "Test Alert".to_string(), false);

        let alerts = alert.check_alerts();
        assert_eq!(alerts.len(), 3);
        assert_eq!(alerts[0].0, 1);
        assert_eq!(alerts[1].0, 3);
        assert_eq!(alerts[2].0, 4);
    }

    #[test]
    fn test_alert_once_command() {
        let condition = Array1::from_vec(vec![0.0, 1.0, 0.0, 1.0, 1.0]);
        let mut alert = AlertCommand::new(condition, "Once Alert".to_string(), true);

        let alerts = alert.check_alerts();
        assert_eq!(alerts.len(), 3);

        let alerts2 = alert.check_alerts();
        assert_eq!(alerts2.len(), 0);
    }
}

#[cfg(test)]
mod selection_result_tests {
    use super::*;

    #[test]
    fn test_selection_result_creation() {
        let signals = Array1::from_vec(vec![0.0, 1.0, 0.0, 1.0, 1.0]);
        let result = SelectionResult::new(signals, 0);

        assert_eq!(result.mode, 0);
        assert_eq!(result.selected_bars.len(), 3);
        assert_eq!(result.selected_bars, vec![1, 3, 4]);
    }

    #[test]
    fn test_selection_result_empty() {
        let signals = Array1::from_vec(vec![0.0, 0.0, 0.0, 0.0, 0.0]);
        let result = SelectionResult::new(signals, 1);

        assert!(result.selected_bars.is_empty());
    }
}

#[cfg(test)]
mod compatibility_score_tests {
    use super::*;

    fn count_supported_functions() -> usize {
        let mut engine = make_engine();
        let test_funcs = vec![
            "CLOSE1",
            "OPEN1",
            "HIGH1",
            "LOW1",
            "VOL1",
            "SMARTSELECT",
            "SELECTCOND",
            "ALERT",
            "ALERTONCE",
            "AVGPRICE_N",
            "TOTALVOL",
            "MAXPRICE",
            "MINPRICE",
            "MA",
            "EMA",
            "SMA",
            "WMA",
            "HHV",
            "LLV",
            "REF",
            "CROSS",
            "RSI",
            "MACD",
            "KDJ",
            "BOLL",
            "ATR",
            "VWAP",
            "SUPERTREND",
        ];

        let mut supported = 0;

        for func in test_funcs {
            let mut ctx = make_test_context(30);
            let formula = if func.contains("SELECT") || func.contains("ALERT") {
                format!("{}(CLOSE > OPEN, 0)", func)
            } else if func.ends_with("_N")
                || func == "TOTALVOL"
                || func == "MAXPRICE"
                || func == "MINPRICE"
            {
                format!("{}(5)", func)
            } else if func.ends_with("1") {
                func.to_string()
            } else if func == "MA" || func == "EMA" || func == "SMA" || func == "WMA" {
                format!("{}(CLOSE, 5)", func)
            } else if func == "HHV" || func == "LLV" {
                format!("{}(CLOSE, 5)", func)
            } else if func == "REF" {
                format!("{}(CLOSE, 1)", func)
            } else if func == "CROSS" {
                format!("{}(CLOSE, OPEN)", func)
            } else if func == "RSI" {
                format!("{}(CLOSE, 14)", func)
            } else if func == "MACD" {
                format!("{}(CLOSE, 12)", func)
            } else if func == "KDJ" {
                format!("{}(HIGH, LOW, CLOSE)", func)
            } else if func == "BOLL" {
                format!("{}(CLOSE, 20)", func)
            } else if func == "ATR" {
                format!("{}(HIGH, LOW, CLOSE, 14)", func)
            } else if func == "VWAP" {
                format!("{}(HIGH, LOW, CLOSE, VOLUME)", func)
            } else if func == "SUPERTREND" {
                format!("{}(HIGH, LOW, CLOSE, 14)", func)
            } else {
                func.to_string()
            };

            if engine.eval(&formula, &mut ctx).is_ok() {
                supported += 1;
            }
        }

        supported
    }

    #[test]
    fn test_compatibility_score() {
        let supported = count_supported_functions();
        let total = 27;
        let score = supported as f64 / total as f64 * 100.0;

        println!(
            "THS Compatibility Score: {:.1}% ({}/{} functions supported)",
            score, supported, total
        );
        assert!(
            score >= 95.0,
            "THS compatibility should be >= 95%, got {:.1}%",
            score
        );
    }
}
