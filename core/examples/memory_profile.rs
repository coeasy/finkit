/// Memory profiling for Alpha-TA core library
/// Run with: cargo run --example memory_profile --release
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

struct TrackingAllocator;

static ALLOCATED: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
static TRACKING: AtomicBool = AtomicBool::new(false);

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: Delegating to System allocator which is safe.
        let ptr = unsafe { System.alloc(layout) };
        if TRACKING.load(Ordering::SeqCst) {
            let size = layout.size();
            let prev = ALLOCATED.fetch_add(size, Ordering::SeqCst);
            let new_total = prev + size;
            let mut peak = PEAK.load(Ordering::SeqCst);
            while new_total > peak {
                match PEAK.compare_exchange_weak(
                    peak,
                    new_total,
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                ) {
                    Ok(_) => break,
                    Err(current) => peak = current,
                }
            }
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if TRACKING.load(Ordering::SeqCst) {
            ALLOCATED.fetch_sub(layout.size(), Ordering::SeqCst);
        }
        // SAFETY: Delegating to System allocator which is safe.
        unsafe {
            System.dealloc(ptr, layout);
        }
    }
}

#[global_allocator]
static A: TrackingAllocator = TrackingAllocator;

fn start_tracking() {
    ALLOCATED.store(0, Ordering::SeqCst);
    PEAK.store(0, Ordering::SeqCst);
    TRACKING.store(true, Ordering::SeqCst);
}

fn stop_tracking() -> (usize, usize) {
    TRACKING.store(false, Ordering::SeqCst);
    let current = ALLOCATED.load(Ordering::SeqCst);
    let peak = PEAK.load(Ordering::SeqCst);
    (current, peak)
}

fn profile_indicator<F>(name: &str, _n: usize, mut func: F)
where
    F: FnMut(),
{
    // Warmup
    for _ in 0..3 {
        func();
    }

    let mut total_time = 0.0f64;
    let mut peak_mem = 0usize;

    for _ in 0..5 {
        start_tracking();
        let t0 = std::time::Instant::now();
        func();
        let elapsed = t0.elapsed();
        let (_, peak) = stop_tracking();
        total_time += elapsed.as_secs_f64() * 1000.0;
        peak_mem += peak;
    }

    let avg_time = total_time / 5.0;
    let avg_peak = peak_mem as f64 / 5.0 / 1024.0; // KB

    println!(
        "{:<20} time={:>8.3}ms  peak_mem={:>8.1}KB",
        name, avg_time, avg_peak
    );
}

fn generate_ohlcv(n: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut close = Vec::with_capacity(n);
    let mut high = Vec::with_capacity(n);
    let mut low = Vec::with_capacity(n);
    let mut open = Vec::with_capacity(n);
    let mut volume = Vec::with_capacity(n);
    let mut price = 100.0;
    for i in 0..n {
        let change = (i as f64 * 0.1).sin() * 2.0 + (i as f64 * 0.05).cos() * 1.5;
        price += change;
        high.push(price + (i as f64 * 0.03).sin().abs() * 3.0);
        low.push(price - (i as f64 * 0.04).cos().abs() * 3.0);
        open.push(price + (i as f64 * 0.02).sin() * 1.0);
        volume.push(1_000_000.0 + (i as f64 * 0.1).sin() * 500_000.0);
        close.push(price);
    }
    (open, high, low, close, volume)
}

fn main() {
    println!("===== AlphaTA Memory Profile (10,000 data points) =====");
    println!("{:<20} {:<16} {}", "Indicator", "Time", "Peak Memory");
    println!("{}", "-".repeat(60));

    let (_open, high, low, close, volume) = generate_ohlcv(10000);

    // Overlap indicators
    profile_indicator("SMA(20)", 10000, || {
        finkit::math::moving_avg::sma(&close, 20).unwrap();
    });

    profile_indicator("EMA(20)", 10000, || {
        finkit::math::moving_avg::ema(&close, 20).unwrap();
    });

    profile_indicator("WMA(20)", 10000, || {
        finkit::math::moving_avg::wma(&close, 20).unwrap();
    });

    profile_indicator("DEMA(20)", 10000, || {
        finkit::math::moving_avg::dema(&close, 20).unwrap();
    });

    profile_indicator("TEMA(20)", 10000, || {
        finkit::math::moving_avg::tema(&close, 20).unwrap();
    });

    // Momentum indicators
    profile_indicator("RSI(14)", 10000, || {
        finkit::indicators::rsi(&close, 14).unwrap();
    });

    profile_indicator("MACD(12,26,9)", 10000, || {
        finkit::indicators::macd(&close, 12, 26, 9).unwrap();
    });

    profile_indicator("MOM(10)", 10000, || {
        finkit::indicators::mom(&close, 10).unwrap();
    });

    profile_indicator("ROC(10)", 10000, || {
        finkit::indicators::roc(&close, 10).unwrap();
    });

    profile_indicator("CCI(14)", 10000, || {
        finkit::indicators::cci(&high, &low, &close, 14).unwrap();
    });

    profile_indicator("WILLR(14)", 10000, || {
        finkit::indicators::willr(&high, &low, &close, 14).unwrap();
    });

    profile_indicator("ADX(14)", 10000, || {
        finkit::indicators::adx(&high, &low, &close, 14).unwrap();
    });

    profile_indicator("TRIX(14)", 10000, || {
        finkit::indicators::trix(&close, 14).unwrap();
    });

    // Volatility indicators
    profile_indicator("BBANDS(20,2)", 10000, || {
        finkit::indicators::bbands(&close, 20, 2.0, 2.0).unwrap();
    });

    profile_indicator("ATR(14)", 10000, || {
        finkit::indicators::atr(&high, &low, &close, 14).unwrap();
    });

    profile_indicator("NATR(14)", 10000, || {
        finkit::indicators::natr(&high, &low, &close, 14).unwrap();
    });

    // Volume indicators
    profile_indicator("OBV", 10000, || {
        finkit::indicators::obv(&close, &volume).unwrap();
    });

    profile_indicator("AD", 10000, || {
        finkit::indicators::ad(&high, &low, &close, &volume).unwrap();
    });

    profile_indicator("ADOSC", 10000, || {
        finkit::indicators::adosc(&high, &low, &close, &volume, 3, 10).unwrap();
    });

    // Statistical indicators
    profile_indicator("STDDEV(20)", 10000, || {
        finkit::indicators::std_dev(&close, 20, 1.0).unwrap();
    });

    profile_indicator("VAR(20)", 10000, || {
        finkit::indicators::var(&close, 20, 1.0).unwrap();
    });

    // Zero-alloc variants
    println!();
    println!("--- Zero-Allocation Variants ---");
    let mut out = vec![0.0; close.len()];

    profile_indicator("SMA_INTO(20)", 10000, || {
        finkit::math::moving_avg::sma_into(&close, 20, &mut out).unwrap();
    });

    profile_indicator("EMA_INTO(20)", 10000, || {
        finkit::math::moving_avg::ema_into(&close, 20, &mut out).unwrap();
    });

    println!("{}", "-".repeat(60));
    println!(
        "Note: Peak memory measured via custom global allocator tracking all alloc/dealloc calls."
    );
    println!("(Does not include pre-allocated output buffer for zero-alloc variants)");
}
