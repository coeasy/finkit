use finkit::formula::{FormulaContext, FormulaDialect, FormulaEngine, FormulaExecutionMode};
use finkit::indicators::sweep::ema_sweep;
use finkit::math::moving_avg::ema;
use ndarray::Array1;
use std::hint::black_box;
use std::time::{Duration, Instant};

const SOURCE: &str = "EXP1:=EMA(CLOSE,12);EXP2:=EMA(CLOSE,50);EXP3:=EMA(CLOSE,20);";
const PERIODS: [usize; 3] = [12, 50, 20];

fn input(len: usize) -> Vec<f64> {
    (0..len)
        .map(|i| {
            let x = i as f64;
            3_200.0 + 0.13 * x + (x * 0.031).sin() * 41.0
        })
        .collect()
}

fn context(close: &[f64]) -> FormulaContext {
    FormulaContext::new(
        Array1::from_vec(vec![0.0; close.len()]),
        Array1::from_vec(close.iter().map(|v| v + 1.0).collect()),
        Array1::from_vec(close.iter().map(|v| v - 1.0).collect()),
        Array1::from_vec(close.to_vec()),
        Array1::from_vec(vec![1_000_000.0; close.len()]),
        None,
    )
}

fn time(mut f: impl FnMut(), duration: Duration) -> f64 {
    let start = Instant::now();
    let mut calls = 0u64;
    while start.elapsed() < duration {
        f();
        calls += 1;
    }
    start.elapsed().as_secs_f64() * 1e6 / calls as f64
}

fn median(samples: &mut [f64]) -> f64 {
    samples.sort_by(f64::total_cmp);
    samples[samples.len() / 2]
}

fn same(left: &[f64], right: &[f64]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(&a, &b)| (a.is_nan() && b.is_nan()) || (a - b).abs() <= 1e-10)
}

fn main() {
    for len in [250usize, 5_000] {
        let close = input(len);
        let sweep = || ema_sweep(&close, &PERIODS).unwrap();
        let repeated = || PERIODS.map(|period| ema(&close, period).unwrap().to_vec());
        let expected = repeated();
        let actual = sweep();
        assert_eq!(actual.len(), expected.len());
        for (left, right) in actual.iter().zip(&expected) {
            assert_eq!(left.len(), right.len());
            assert!(
                left.iter()
                    .zip(right)
                    .all(|(&a, &b)| { (a.is_nan() && b.is_nan()) || (a - b).abs() <= 1e-10 }),
                "EMA sweep exceeded the 1e-10 probe tolerance"
            );
        }

        let mut plan_context = context(&close);
        let mut plan = FormulaEngine::new().with_execution_mode(FormulaExecutionMode::Plan);
        let plan_primary = plan
            .eval_with_dialect(SOURCE, FormulaDialect::TongDaXin, &mut plan_context)
            .unwrap();
        assert!(same(plan_primary.as_slice().unwrap(), &expected[2]));
        for (name, values) in ["EXP1", "EXP2", "EXP3"].into_iter().zip(&expected) {
            assert!(same(
                plan_context.variables[name].as_slice().unwrap(),
                values
            ));
        }

        let mut tree_context = context(&close);
        let mut tree = FormulaEngine::new().with_execution_mode(FormulaExecutionMode::Tree);
        let tree_primary = tree
            .eval_with_dialect(SOURCE, FormulaDialect::TongDaXin, &mut tree_context)
            .unwrap();
        assert!(same(
            tree_primary.as_slice().unwrap(),
            plan_primary.as_slice().unwrap()
        ));
        for name in ["EXP1", "EXP2", "EXP3"] {
            assert!(same(
                tree_context.variables[name].as_slice().unwrap(),
                plan_context.variables[name].as_slice().unwrap()
            ));
        }

        let mut sweep_samples = [0.0; 7];
        let mut repeated_samples = [0.0; 7];
        let mut plan_samples = [0.0; 7];
        let mut tree_samples = [0.0; 7];
        for i in 0..7 {
            sweep_samples[i] = time(
                || {
                    black_box(sweep());
                },
                Duration::from_millis(100),
            );
            repeated_samples[i] = time(
                || {
                    black_box(repeated());
                },
                Duration::from_millis(100),
            );
            plan_samples[i] = time(
                || {
                    black_box(
                        plan.eval_with_dialect(
                            SOURCE,
                            FormulaDialect::TongDaXin,
                            &mut plan_context,
                        )
                        .unwrap(),
                    );
                },
                Duration::from_millis(100),
            );
            tree_samples[i] = time(
                || {
                    black_box(
                        tree.eval_with_dialect(
                            SOURCE,
                            FormulaDialect::TongDaXin,
                            &mut tree_context,
                        )
                        .unwrap(),
                    );
                },
                Duration::from_millis(100),
            );
        }
        println!(
            "len={len} sweep={:.3}us repeated={:.3}us plan={:.3}us tree={:.3}us sweep/repeated={:.3}x",
            median(&mut sweep_samples),
            median(&mut repeated_samples),
            median(&mut plan_samples),
            median(&mut tree_samples),
            median(&mut sweep_samples) / median(&mut repeated_samples),
        );
    }
}
