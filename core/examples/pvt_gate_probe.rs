use finkit::indicators::volume_ext::pvt;
use finkit::talib_ffi::{TA_Initialize, TA_Shutdown, TA_SUCCESS};
use ndarray::Array1;
use std::hint::black_box;
use std::time::{Duration, Instant};

unsafe extern "C" {
    fn TA_PVT(
        start_idx: i32,
        end_idx: i32,
        input: *const f64,
        volume: *const f64,
        out_begin: *mut i32,
        out_count: *mut i32,
        output: *mut f64,
    ) -> i32;
}

fn data(len: usize) -> (Vec<f64>, Vec<f64>) {
    let mut close = Vec::with_capacity(len);
    let mut volume = Vec::with_capacity(len);
    for i in 0..len {
        let x = i as f64;
        close.push(100.0 + 0.006 * x + (x * 0.017).sin() * 2.1);
        volume.push(1_000_000.0 + (x % 997.0) * 170.0 + (x * 0.011).sin().abs() * 50_000.0);
    }
    (close, volume)
}

fn pvt_pointer_write(close: &[f64], volume: &[f64]) -> Array1<f64> {
    let len = close.len();
    let mut output = Vec::with_capacity(len);
    output.push(0.0);
    let output_ptr = output.as_mut_ptr();
    let mut accumulated = 0.0;
    for index in 1..len {
        let current = close[index];
        let previous = close[index - 1];
        if current.is_nan() || volume[index].is_nan() {
            return pvt(close, volume).unwrap();
        }
        let inv_cp = 1.0 / previous;
        accumulated += volume[index] * (current - previous) * inv_cp;
        unsafe { output_ptr.add(index).write(accumulated) };
    }
    unsafe { output.set_len(len) };
    Array1::from_vec(output)
}

fn pvt_zipped(close: &[f64], volume: &[f64]) -> Array1<f64> {
    let len = close.len();
    let mut output = Vec::with_capacity(len);
    output.push(0.0);
    let mut accumulated = 0.0;
    let output_ptr = output.as_mut_ptr();
    for (index, ((&previous, &current), &current_volume)) in
        close.iter().zip(&close[1..]).zip(&volume[1..]).enumerate()
    {
        if current.is_nan() || current_volume.is_nan() {
            return pvt(close, volume).unwrap();
        }
        let inv_cp = 1.0 / previous;
        accumulated += current_volume * (current - previous) * inv_cp;
        unsafe { output_ptr.add(index + 1).write(accumulated) };
    }
    unsafe { output.set_len(len) };
    Array1::from_vec(output)
}

fn talib(close: &[f64], volume: &[f64]) -> Array1<f64> {
    let mut output = vec![0.0; close.len()];
    let (mut begin, mut count) = (0, 0);
    let status = unsafe {
        TA_PVT(
            0,
            (close.len() - 1) as i32,
            close.as_ptr(),
            volume.as_ptr(),
            &mut begin,
            &mut count,
            output.as_mut_ptr(),
        )
    };
    assert_eq!(status, TA_SUCCESS);
    let begin = begin as usize;
    let count = count as usize;
    output.copy_within(0..count, begin);
    output[..begin].fill(f64::NAN);
    output[begin + count..].fill(f64::NAN);
    Array1::from_vec(output)
}

fn time(mut f: impl FnMut() -> Array1<f64>, duration: Duration) -> f64 {
    let started = Instant::now();
    let mut calls = 0u64;
    while started.elapsed() < duration {
        black_box(f());
        calls += 1;
    }
    started.elapsed().as_secs_f64() * 1e6 / calls as f64
}

fn same(left: &[f64], right: &[f64]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(&a, &b)| (a.is_nan() && b.is_nan()) || a == b || (a - b).abs() <= 1e-8)
}

fn main() {
    unsafe { TA_Initialize() };
    for len in [1_000usize, 100_000, 1_000_000] {
        let (close, volume) = data(len);
        let current = || pvt(&close, &volume).unwrap();
        let pointer_write = || pvt_pointer_write(&close, &volume);
        let zipped = || pvt_zipped(&close, &volume);
        let c = || talib(&close, &volume);
        let expected = current();
        let actual_c = c();
        let actual_pointer_write = pointer_write();
        let actual_zipped = zipped();
        assert!(same(
            expected.as_slice().unwrap(),
            actual_c.as_slice().unwrap()
        ));
        assert!(same(
            expected.as_slice().unwrap(),
            actual_pointer_write.as_slice().unwrap()
        ));
        assert!(same(
            expected.as_slice().unwrap(),
            actual_zipped.as_slice().unwrap()
        ));
        let (mut current_times, mut pointer_write_times, mut zipped_times, mut c_times) =
            (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        for turn in 0..7 {
            let measure = |slot: &mut Vec<f64>, f: &mut dyn FnMut() -> Array1<f64>| {
                slot.push(time(f, Duration::from_millis(100)));
            };
            if turn % 2 == 0 {
                measure(&mut current_times, &mut { || current() });
                measure(&mut pointer_write_times, &mut { || pointer_write() });
                measure(&mut zipped_times, &mut { || zipped() });
                measure(&mut c_times, &mut { || c() });
            } else {
                measure(&mut c_times, &mut { || c() });
                measure(&mut zipped_times, &mut { || zipped() });
                measure(&mut pointer_write_times, &mut { || pointer_write() });
                measure(&mut current_times, &mut { || current() });
            }
        }
        for samples in [
            &mut current_times,
            &mut pointer_write_times,
            &mut zipped_times,
            &mut c_times,
        ] {
            samples.sort_by(f64::total_cmp);
        }
        println!(
            "PVT n={len}: public={:.2}us pointer_write={:.2}us zipped={:.2}us TA-Lib={:.2}us speedup public={:.3}x pointer_write={:.3}x zipped={:.3}x parity=true",
            current_times[3],
            pointer_write_times[3],
            zipped_times[3],
            c_times[3],
            c_times[3] / current_times[3],
            c_times[3] / pointer_write_times[3],
            c_times[3] / zipped_times[3],
        );
    }
    unsafe { TA_Shutdown() };
}
