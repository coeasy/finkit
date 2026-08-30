// GENERATED FILE — do not edit by hand.
// Source of truth: docs/indicator_registry.json (ffi block).
// Regenerate with: python3 scripts/gen_binding.py --lang node --generate <path>


    #[napi]
    pub fn sma(input: Vec<f64>, period: i32) -> Result<Vec<f64>> {
        moving_avg::sma(&input, period as usize)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn ema(input: Vec<f64>, period: i32) -> Result<Vec<f64>> {
        moving_avg::ema(&input, period as usize)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn wma(input: Vec<f64>, period: i32) -> Result<Vec<f64>> {
        moving_avg::wma(&input, period as usize)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn dema(input: Vec<f64>, period: i32) -> Result<Vec<f64>> {
        moving_avg::dema(&input, period as usize)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn tema(input: Vec<f64>, period: i32) -> Result<Vec<f64>> {
        moving_avg::tema(&input, period as usize)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn kama(input: Vec<f64>, period: i32) -> Result<Vec<f64>> {
        moving_avg::kama(&input, period as usize, 2, 30)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn mama(input: Vec<f64>, fast_limit: i32, slow_limit: i32) -> Result<Vec<f64>> {
        indicators::mama(&input, fast_limit, slow_limit)
            .map(|arr| Ok((arr.mama.into_raw_vec_and_offset().0, arr.fama.into_raw_vec_and_offset().0)))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn t3(input: Vec<f64>, period: i32, vfactor: i32) -> Result<Vec<f64>> {
        indicators::t3(&input, period as usize, vfactor)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn bbands(input: Vec<f64>, period: i32, nbdevup: i32, nbdevdn: i32) -> Result<Vec<f64>> {
        indicators::bbands(&input, period as usize, nbdevup, nbdevdn)
            .map(|arr| Ok((arr.upper.into_raw_vec_and_offset().0, arr.middle.into_raw_vec_and_offset().0, arr.lower.into_raw_vec_and_offset().0)))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn midpoint(input: Vec<f64>, period: i32) -> Result<Vec<f64>> {
        indicators::midpoint(&input, period as usize)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn midprice(high: Vec<f64>, low: Vec<f64>, period: i32) -> Result<Vec<f64>> {
        indicators::midprice(&high, &low, period as usize)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn sar(high: Vec<f64>, low: Vec<f64>, acceleration: i32, maximum: i32) -> Result<Vec<f64>> {
        indicators::sar(&high, &low, acceleration, maximum)
            .map(|arr| Ok((arr.sar.into_raw_vec_and_offset().0)))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn rsi(input: Vec<f64>, period: i32) -> Result<Vec<f64>> {
        indicators::rsi(&input, period as usize)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn macd(input: Vec<f64>, fast_period: i32, slow_period: i32, signal_period: i32) -> Result<Vec<f64>> {
        indicators::macd(&input, fast_period as usize, slow_period as usize, signal_period as usize)
            .map(|arr| Ok((arr.macd.into_raw_vec_and_offset().0, arr.signal.into_raw_vec_and_offset().0, arr.hist.into_raw_vec_and_offset().0)))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn stoch(high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, fastk_period: i32, slowk_period: i32, slowd_period: i32) -> Result<Vec<f64>> {
        indicators::stoch(&high, &low, &close, fastk_period as usize, slowk_period as usize, slowd_period as usize)
            .map(|arr| Ok((arr.k.into_raw_vec_and_offset().0, arr.d.into_raw_vec_and_offset().0)))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn adx(high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, period: i32) -> Result<Vec<f64>> {
        indicators::adx(&high, &low, &close, period as usize)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn aroon(high: Vec<f64>, low: Vec<f64>, period: i32) -> Result<Vec<f64>> {
        indicators::aroon(&high, &low, period as usize)
            .map(|arr| Ok((arr.aroon_up.into_raw_vec_and_offset().0, arr.aroon_down.into_raw_vec_and_offset().0)))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn cci(high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, period: i32) -> Result<Vec<f64>> {
        indicators::cci(&high, &low, &close, period as usize)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn mom(input: Vec<f64>, period: i32) -> Result<Vec<f64>> {
        indicators::mom(&input, period as usize)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn roc(input: Vec<f64>, period: i32) -> Result<Vec<f64>> {
        indicators::roc(&input, period as usize)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn willr(high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, period: i32) -> Result<Vec<f64>> {
        indicators::willr(&high, &low, &close, period as usize)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn apo(input: Vec<f64>, fast_period: i32, slow_period: i32) -> Result<Vec<f64>> {
        indicators::apo(&input, fast_period as usize, slow_period as usize)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn bop(open: Vec<f64>, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>) -> Result<Vec<f64>> {
        indicators::bop(&open, &high, &low, &close)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn cmo(input: Vec<f64>, period: i32) -> Result<Vec<f64>> {
        indicators::cmo(&input, period as usize)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn mfi(high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, volume: Vec<f64>, period: i32) -> Result<Vec<f64>> {
        indicators::mfi(&high, &low, &close, &volume, period as usize)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn trix(input: Vec<f64>, period: i32) -> Result<Vec<f64>> {
        indicators::trix(&input, period as usize)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn vortex(high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, period: i32) -> Result<Vec<f64>> {
        indicators::vortex(&high, &low, &close, period as usize)
            .map(|arr| Ok((arr.vi_plus.into_raw_vec_and_offset().0, arr.vi_minus.into_raw_vec_and_offset().0)))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn vzo(close: Vec<f64>, volume: Vec<f64>, period: i32) -> Result<Vec<f64>> {
        indicators::vzo(&close, &volume, period as usize)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn volume_momentum(volume: Vec<f64>, period: i32) -> Result<Vec<f64>> {
        indicators::volume_momentum(&volume, period as usize)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn volume_roc(volume: Vec<f64>, period: i32) -> Result<Vec<f64>> {
        indicators::volume_roc(&volume, period as usize)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn chande_forecast(close: Vec<f64>, period: i32) -> Result<Vec<f64>> {
        indicators::chande_forecast_oscillator(&close, period as usize)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn twiggs_mf(high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, volume: Vec<f64>, period: i32) -> Result<Vec<f64>> {
        indicators::twiggs_money_flow(&high, &low, &close, &volume, period as usize)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn inertia(open: Vec<f64>, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, rvi_period: i32, linreg_period: i32) -> Result<Vec<f64>> {
        indicators::inertia(&open, &high, &low, &close, rvi_period as usize, linreg_period as usize)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn atr(high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, period: i32) -> Result<Vec<f64>> {
        indicators::atr(&high, &low, &close, period as usize)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn natr(high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, period: i32) -> Result<Vec<f64>> {
        indicators::natr(&high, &low, &close, period as usize)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn trange(high: Vec<f64>, low: Vec<f64>, close: Vec<f64>) -> Result<Vec<f64>> {
        indicators::trange(&high, &low, &close)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn obv(close: Vec<f64>, volume: Vec<f64>) -> Result<Vec<f64>> {
        indicators::obv(&close, &volume)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn ad(high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, volume: Vec<f64>) -> Result<Vec<f64>> {
        indicators::ad(&high, &low, &close, &volume)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn adosc(high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, volume: Vec<f64>, fast_period: i32, slow_period: i32) -> Result<Vec<f64>> {
        indicators::adosc(&high, &low, &close, &volume, fast_period as usize, slow_period as usize)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn ht_dcperiod(input: Vec<f64>) -> Result<Vec<f64>> {
        indicators::ht_dcperiod(&input)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn ht_dcphase(input: Vec<f64>) -> Result<Vec<f64>> {
        indicators::ht_dcphase(&input)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn ht_phasor(input: Vec<f64>) -> Result<Vec<f64>> {
        indicators::ht_phasor(&input)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn ht_sine(input: Vec<f64>) -> Result<Vec<f64>> {
        indicators::ht_sine(&input)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn ht_trendmode(input: Vec<f64>) -> Result<Vec<f64>> {
        indicators::ht_trendmode(&input)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn ht_trendline(input: Vec<f64>) -> Result<Vec<f64>> {
        indicators::ht_trendline(&input)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn zscore(input: Vec<f64>, period: i32) -> Result<Vec<f64>> {
        indicators::zscore(&input, period as usize)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn beta(asset: Vec<f64>, benchmark: Vec<f64>, period: i32) -> Result<Vec<f64>> {
        indicators::beta(&asset, &benchmark, period as usize)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn correlation(input_a: Vec<f64>, input_b: Vec<f64>, period: i32) -> Result<Vec<f64>> {
        indicators::correlation(&input_a, &input_b, period as usize)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn stddev(input: Vec<f64>, period: i32, nb_dev: i32) -> Result<Vec<f64>> {
        indicators::std_dev(&input, period as usize, nb_dev)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn tsf(input: Vec<f64>, period: i32) -> Result<Vec<f64>> {
        indicators::tsf(&input, period as usize)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn linear_reg(input: Vec<f64>, period: i32) -> Result<Vec<f64>> {
        indicators::linear_reg(&input, period as usize)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn percent_rank(input: Vec<f64>, period: i32) -> Result<Vec<f64>> {
        indicators::percent_rank(&input, period as usize)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn avgprice(open: Vec<f64>, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>) -> Result<Vec<f64>> {
        indicators::avgprice(&open, &high, &low, &close)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn medprice(high: Vec<f64>, low: Vec<f64>) -> Result<Vec<f64>> {
        indicators::medprice(&high, &low)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn typprice(high: Vec<f64>, low: Vec<f64>, close: Vec<f64>) -> Result<Vec<f64>> {
        indicators::typprice(&high, &low, &close)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn wclprice(high: Vec<f64>, low: Vec<f64>, close: Vec<f64>) -> Result<Vec<f64>> {
        indicators::wclprice(&high, &low, &close)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn cdl_doji(open: Vec<f64>, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, doji_pct: i32) -> Result<Vec<i32>> {
        candlestick::doji(&open, &high, &low, &close, doji_pct)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn cdl_dragonfly_doji(open: Vec<f64>, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, doji_pct: i32) -> Result<Vec<i32>> {
        candlestick::dragonfly_doji(&open, &high, &low, &close, doji_pct)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn cdl_gravestone_doji(open: Vec<f64>, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, doji_pct: i32) -> Result<Vec<i32>> {
        candlestick::gravestone_doji(&open, &high, &low, &close, doji_pct)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn cdl_long_legged_doji(open: Vec<f64>, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, doji_pct: i32) -> Result<Vec<i32>> {
        candlestick::long_legged_doji(&open, &high, &low, &close, doji_pct)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn cdl_hammer(open: Vec<f64>, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>) -> Result<Vec<i32>> {
        candlestick::hammer(&open, &high, &low, &close)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn cdl_inverted_hammer(open: Vec<f64>, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>) -> Result<Vec<i32>> {
        candlestick::inverted_hammer(&open, &high, &low, &close)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn cdl_hanging_man(open: Vec<f64>, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>) -> Result<Vec<i32>> {
        candlestick::hanging_man(&open, &high, &low, &close)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn cdl_shooting_star(open: Vec<f64>, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>) -> Result<Vec<i32>> {
        candlestick::shooting_star(&open, &high, &low, &close)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn cdl_engulfing(open: Vec<f64>, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>) -> Result<Vec<i32>> {
        candlestick::engulfing(&open, &high, &low, &close)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn cdl_harami(open: Vec<f64>, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>) -> Result<Vec<i32>> {
        candlestick::harami(&open, &high, &low, &close)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn cdl_morning_star(open: Vec<f64>, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>) -> Result<Vec<i32>> {
        candlestick::morning_star(&open, &high, &low, &close)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn cdl_evening_star(open: Vec<f64>, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>) -> Result<Vec<i32>> {
        candlestick::evening_star(&open, &high, &low, &close)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn cdl_three_white_soldiers(open: Vec<f64>, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>) -> Result<Vec<i32>> {
        candlestick::three_white_soldiers(&open, &high, &low, &close)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn cdl_three_black_crows(open: Vec<f64>, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>) -> Result<Vec<i32>> {
        candlestick::three_black_crows(&open, &high, &low, &close)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn cdl_marubozu(open: Vec<f64>, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, shadow_pct: i32) -> Result<Vec<i32>> {
        candlestick::marubozu(&open, &high, &low, &close, shadow_pct)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn darvas_box(high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, lookback: i32, confirmation: i32) -> Result<Vec<i32>> {
        indicators::darvas_box(&high, &low, &close, lb, conf) {
        Ok(r) => {
            if !out_top.is_null() {
                copy_result(out_top, &r.box_top, len);
            }
            if !out_bottom.is_null() {
                copy_result(out_bottom, &r.box_bottom, len);
            }
            if !out_signal.is_null( as usize)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn renko(high: Vec<f64>, low: Vec<f64>, box_size: i32) -> Result<Vec<i32>> {
        indicators::renko(&high, &low, box_size) {
        Ok(r) => {
            copy_result(out_bricks, &r.bricks, len);
            if !out_dir.is_null( as usize)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn kagi(close: Vec<f64>, reversal: i32) -> Result<Vec<i32>> {
        indicators::kagi(&close, reversal) {
        Ok(r) => {
            copy_result(out_kagi, &r.kagi, len);
            if !out_dir.is_null( as usize)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn point_and_figure(high: Vec<f64>, low: Vec<f64>, box_size: i32, reversal: i32) -> Result<Vec<i32>> {
        indicators::point_and_figure(&high, &low, box_size, rev) {
        Ok(r) => {
            copy_result(out_pnf, &r.pnf, len);
            if !out_col.is_null() {
                copy_int_result(out_col, &r.column_type, len);
            }
            if !out_new.is_null( as usize)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn three_line_break(close: Vec<f64>, lines: i32) -> Result<Vec<i32>> {
        indicators::three_line_break(&close, n) {
        Ok(r) => {
            copy_result(out_line, &r.line, len);
            if !out_dir.is_null( as usize)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn williams_alligator(close: Vec<f64>) -> Result<Vec<f64>> {
        indicators::williams_alligator(&close)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

    #[napi]
    pub fn heikin_ashi(open: Vec<f64>, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>) -> Result<Vec<f64>> {
        indicators::heikin_ashi(&open, &high, &low, c) {
        Ok(r) => {
            if !out_o.is_null() {
                copy_result(out_o, &r.ha_open, len);
            }
            if !out_h.is_null() {
                copy_result(out_h, &r.ha_high, len);
            }
            if !out_l.is_null() {
                copy_result(out_l, &r.ha_low, len);
            }
            if !out_c.is_null( as usize)
            .map(|arr| Ok(arr.into_raw_vec_and_offset().0))
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
    }

