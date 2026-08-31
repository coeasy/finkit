//! Same-binary before/after benchmark for the optimized indicators.
//! Times the ORIGINAL HEAD algorithms (copied inline) vs the CURRENT
//! optimized `indicators::*` functions, eliminating compiler/alignment variance.

use finkit::indicators::{apo, aroon, willr};
use finkit::math::moving_avg::sma;
use finkit::utils::init_output;
use ndarray::Array1;

const N: usize = 10_000;
const ITERS: usize = 200;

fn ohlcv(len: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut open = Vec::with_capacity(len);
    let mut high = Vec::with_capacity(len);
    let mut low = Vec::with_capacity(len);
    let mut close = Vec::with_capacity(len);
    let mut volume = Vec::with_capacity(len);
    for i in 0..len {
        let t = i as f64;
        let noise = (t * 0.37).sin() * 2.0 + (t * 1.13).cos() * 1.5 + (t * 3.71).sin() * 0.8;
        let price = 100.0 + t * 0.01 + noise;
        open.push(price - 0.3);
        high.push(price + 1.0 + ((t * 0.7).sin().abs() * 0.5));
        low.push(price - 1.0 - ((t * 0.5).cos().abs() * 0.5));
        close.push(price);
        volume.push(10000.0 + (t * 10.0).sin() * 3000.0 + 2000.0 * (t * 2.3).cos().abs());
    }
    (open, high, low, close, volume)
}

fn time<F: FnMut()>(mut f: F) -> f64 {
    for _ in 0..20 {
        f();
    }
    let start = std::time::Instant::now();
    for _ in 0..ITERS {
        f();
    }
    start.elapsed().as_secs_f64() * 1000.0 / ITERS as f64
}

// ---- ORIGINAL HEAD willr (deque-based) ----
fn orig_willr(high: &[f64], low: &[f64], close: &[f64], period: usize) -> Array1<f64> {
    let len = close.len();
    let start = period - 1;
    let mut out = vec![f64::NAN; len];
    let high_ptr = high.as_ptr();
    let low_ptr = low.as_ptr();
    let close_ptr = close.as_ptr();
    let out_ptr = out.as_mut_ptr();
    let mut h_buf: Vec<usize> = Vec::with_capacity(period);
    let mut l_buf: Vec<usize> = Vec::with_capacity(period);
    let mut h_head: usize = 0;
    let mut l_head: usize = 0;
    unsafe {
        h_buf.push(0);
        l_buf.push(0);
        for k in 1..period {
            let h = *high_ptr.add(k);
            let l = *low_ptr.add(k);
            while h_buf.len() > h_head && *high_ptr.add(*h_buf.last().unwrap_unchecked()) <= h {
                h_buf.pop();
            }
            h_buf.push(k);
            while l_buf.len() > l_head && *low_ptr.add(*l_buf.last().unwrap_unchecked()) >= l {
                l_buf.pop();
            }
            l_buf.push(k);
        }
        let highest = *high_ptr.add(*h_buf.get_unchecked(h_head));
        let lowest = *low_ptr.add(*l_buf.get_unchecked(l_head));
        let denom = highest - lowest;
        *out_ptr.add(start) = if denom > 1e-15 {
            (highest - *close_ptr.add(start)) / denom * -100.0
        } else {
            0.0
        };
        for i in period..len {
            let ws = i + 1 - period;
            let new_h = *high_ptr.add(i);
            let new_l = *low_ptr.add(i);
            while h_buf.len() > h_head && *high_ptr.add(*h_buf.last().unwrap_unchecked()) <= new_h {
                h_buf.pop();
            }
            h_buf.push(i);
            while h_buf[h_head] < ws {
                h_head += 1;
            }
            let highest_v = *high_ptr.add(*h_buf.get_unchecked(h_head));
            while l_buf.len() > l_head && *low_ptr.add(*l_buf.last().unwrap_unchecked()) >= new_l {
                l_buf.pop();
            }
            l_buf.push(i);
            while l_buf[l_head] < ws {
                l_head += 1;
            }
            let lowest_v = *low_ptr.add(*l_buf.get_unchecked(l_head));
            let denom = highest_v - lowest_v;
            *out_ptr.add(i) = if denom > 1e-15 {
                (highest_v - *close_ptr.add(i)) / denom * -100.0
            } else {
                0.0
            };
        }
    }
    Array1::from_vec(out)
}

// ---- ORIGINAL HEAD aroon (deque-based) ----
fn orig_aroon(high: &[f64], low: &[f64], period: usize) -> (Vec<f64>, Vec<f64>) {
    let len = high.len();
    let mut up_out = vec![f64::NAN; len];
    let mut dn_out = vec![f64::NAN; len];
    let inv_period = 100.0 / period as f64;
    let high_ptr = high.as_ptr();
    let low_ptr = low.as_ptr();
    let mut h_buf: Vec<usize> = Vec::with_capacity(period + 1);
    let mut l_buf: Vec<usize> = Vec::with_capacity(period + 1);
    let mut h_head: usize = 0;
    let mut l_head: usize = 0;
    unsafe {
        h_buf.push(1);
        l_buf.push(1);
        for k in 2..=period {
            let h = *high_ptr.add(k);
            let l = *low_ptr.add(k);
            while h_buf.len() > h_head && *high_ptr.add(*h_buf.last().unwrap_unchecked()) <= h {
                h_buf.pop();
            }
            h_buf.push(k);
            while l_buf.len() > l_head && *low_ptr.add(*l_buf.last().unwrap_unchecked()) >= l {
                l_buf.pop();
            }
            l_buf.push(k);
        }
        let highest_idx = *h_buf.get_unchecked(h_head);
        let lowest_idx = *l_buf.get_unchecked(l_head);
        *up_out.get_unchecked_mut(period) = highest_idx as f64 * inv_period;
        *dn_out.get_unchecked_mut(period) = lowest_idx as f64 * inv_period;
        for i in (period + 1)..len {
            let new_h = *high_ptr.add(i);
            let new_l = *low_ptr.add(i);
            let ws = i + 1 - period;
            while h_buf.len() > h_head && *high_ptr.add(*h_buf.last().unwrap_unchecked()) <= new_h {
                h_buf.pop();
            }
            h_buf.push(i);
            while h_buf[h_head] < ws {
                h_head += 1;
            }
            let highest_idx_i = *h_buf.get_unchecked(h_head);
            while l_buf.len() > l_head && *low_ptr.add(*l_buf.last().unwrap_unchecked()) >= new_l {
                l_buf.pop();
            }
            l_buf.push(i);
            while l_buf[l_head] < ws {
                l_head += 1;
            }
            let lowest_idx_i = *l_buf.get_unchecked(l_head);
            *up_out.get_unchecked_mut(i) = (period - (i - highest_idx_i)) as f64 * inv_period;
            *dn_out.get_unchecked_mut(i) = (period - (i - lowest_idx_i)) as f64 * inv_period;
        }
    }
    (up_out, dn_out)
}

fn main() {
    let (_open, high, low, close, _volume) = ohlcv(N);

    let t_orig_willr = time(|| {
        let _ = orig_willr(&high, &low, &close, 14);
    });
    let t_new_willr = time(|| {
        let _ = willr(&high, &low, &close, 14).unwrap();
    });

    let t_orig_aroon = time(|| {
        let _ = orig_aroon(&high, &low, 14);
    });
    let t_new_aroon = time(|| {
        let _ = aroon(&high, &low, 14).unwrap();
    });

    println!("=== WILLR (14) ===");
    println!("  original : {:>9.4} ms", t_orig_willr);
    println!("  optimized: {:>9.4} ms", t_new_willr);
    println!("  speedup  : {:.2}x", t_orig_willr / t_new_willr);

    println!("=== AROON (14) ===");
    println!("  original : {:>9.4} ms", t_orig_aroon);
    println!("  optimized: {:>9.4} ms", t_new_aroon);
    println!("  speedup  : {:.2}x", t_orig_aroon / t_new_aroon);

    // Sanity: optimized output must match original output numerically.
    let o_w = orig_willr(&high, &low, &close, 14);
    let n_w = willr(&high, &low, &close, 14).unwrap();
    let mut maxdiff = 0.0f64;
    for i in 0..N {
        let d = (o_w[i] - n_w[i]).abs();
        if d > maxdiff {
            maxdiff = d;
        }
    }
    println!("  WILLR max|orig-opt| diff: {:.3e}", maxdiff);

    let (o_u, o_d) = orig_aroon(&high, &low, 14);
    let n_a = aroon(&high, &low, 14).unwrap();
    let mut maxdiff = 0.0f64;
    for i in 0..N {
        let d = (o_u[i] - n_a.aroon_up[i])
            .abs()
            .max((o_d[i] - n_a.aroon_down[i]).abs());
        if d > maxdiff {
            maxdiff = d;
        }
    }
    println!("  AROON max|orig-opt| diff: {:.3e}", maxdiff);

    // APO: original = two full SMA passes + diff; optimized = fused single pass.
    let fast = 12usize;
    let slow = 26usize;
    let t_orig_apo = time(|| {
        let f = sma(&close, fast).unwrap();
        let s = sma(&close, slow).unwrap();
        let mut out = init_output(N);
        for i in 0..N {
            if !f[i].is_nan() && !s[i].is_nan() {
                out[i] = f[i] - s[i];
            }
        }
        let _ = out;
    });
    let t_fused_apo = time(|| {
        let _ = apo(&close, fast, slow).unwrap();
    });
    println!("=== APO ({},{}) ===", fast, slow);
    println!("  original : {:>9.4} ms", t_orig_apo);
    println!("  fused    : {:>9.4} ms", t_fused_apo);
    println!("  speedup  : {:.2}x", t_orig_apo / t_fused_apo);
}
