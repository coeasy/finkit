use finkit::math::moving_avg::{sma, wma};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;

#[derive(Deserialize)]
struct G {
    metadata: Gm,
    results: HashMap<String, Dr>,
}
#[derive(Deserialize)]
struct Gm {
    outputs: Vec<String>,
}
#[derive(Deserialize)]
struct Dr {
    fixture_path: String,
    outputs: HashMap<String, Vec<Option<f64>>>,
}

fn read_csv(path: &str) -> Vec<f64> {
    let content = fs::read_to_string(path).unwrap();
    let lines: Vec<&str> = content
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .collect();
    let header: Vec<&str> = lines[0].split(',').map(|s| s.trim()).collect();
    let ci = header
        .iter()
        .position(|h| h.eq_ignore_ascii_case("close"))
        .unwrap();
    lines[1..]
        .iter()
        .filter_map(|l| l.split(',').nth(ci).and_then(|s| s.trim().parse().ok()))
        .collect()
}

fn main() {
    let root = env!("CARGO_MANIFEST_DIR");
    let ws = format!("{}/..", root);
    for name in ["sma", "wma"] {
        let golden: G = serde_json::from_str(
            &fs::read_to_string(format!("{}/tests/golden/talib/{}.json", ws, name)).unwrap(),
        )
        .unwrap();
        for (ds_id, (_k, dr)) in golden.results.into_iter().enumerate() {
            let fp = format!("{}/{}", ws, dr.fixture_path);
            let input = read_csv(&fp);
            let outkey = &golden.metadata.outputs[0];
            let expected: &Vec<Option<f64>> = dr.outputs.get(outkey).unwrap();
            let computed = match name {
                "sma" => sma(&input, 10).unwrap(),
                _ => wma(&input, 10).unwrap(),
            };
            let computed: Vec<f64> = computed.iter().copied().collect();
            let mut maxd = 0f64;
            let mut maxi = 0usize;
            let mut over = 0usize;
            let mut over1e6 = 0usize;
            let mut n = 0;
            for (i, e) in expected.iter().enumerate() {
                if let Some(e) = e {
                    let a = computed[i];
                    let d = (e - a).abs();
                    if d > maxd {
                        maxd = d;
                        maxi = i;
                    }
                    if d > 1e-10 {
                        over += 1;
                    }
                    if d > 1e-6 {
                        over1e6 += 1;
                    }
                    n += 1;
                }
            }
            println!(
                "{} ds#{} len={} compared={} maxdiff={:.3e} @idx{} >1e-10:{}/{} >1e-6:{}",
                name,
                ds_id,
                input.len(),
                n,
                maxd,
                maxi,
                over,
                n,
                over1e6
            );
        }
    }
}
