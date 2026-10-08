//! Calendar component extractors for the TDX date/time functions.

use super::prelude::*;

// ======================== TIME / BAR FUNCTIONS (TDX) ========================

pub(crate) fn datetime_component(ctx: &FormulaContext, extract: fn(i64) -> f64) -> Array1<f64> {
    let len = ctx.data_len;
    match &ctx.datetime {
        Some(dt) => dt.mapv(extract),
        None => nan_vec(len),
    }
}

pub(crate) fn ts_to_date_parts(ts: i64) -> (i32, u32, u32, u32, u32, u32) {
    const SECONDS_PER_DAY: i64 = 86_400;
    let total_days = ts.div_euclid(SECONDS_PER_DAY);
    let time_of_day = ts.rem_euclid(SECONDS_PER_DAY) as u32;
    let hour = time_of_day / 3600;
    let minute = (time_of_day % 3600) / 60;
    let second = time_of_day % 60;

    // Civil date conversion from days since 1970-01-01. This is constant-time,
    // supports dates before the Unix epoch, and avoids narrowing i64 day counts.
    // The 719468 offset changes the origin to 0000-03-01 for leap-year math.
    let z = total_days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_part = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_part + 2) / 5 + 1;
    let month = month_part + if month_part < 10 { 3 } else { -9 };
    let year = year + if month <= 2 { 1 } else { 0 };

    // The public formula API exposes a 32-bit year. Saturate only for the
    // physically unreachable i64-second extremes that cannot fit that type.
    let year = year.clamp(i32::MIN as i64, i32::MAX as i64) as i32;
    (year, month as u32, day as u32, hour, minute, second)
}

pub(crate) fn fn_year(
    ctx: &FormulaContext,
    _args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    Ok(datetime_component(ctx, |ts| ts_to_date_parts(ts).0 as f64))
}

pub(crate) fn fn_month(
    ctx: &FormulaContext,
    _args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    Ok(datetime_component(ctx, |ts| ts_to_date_parts(ts).1 as f64))
}

pub(crate) fn fn_day(
    ctx: &FormulaContext,
    _args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    Ok(datetime_component(ctx, |ts| ts_to_date_parts(ts).2 as f64))
}

pub(crate) fn fn_hour(
    ctx: &FormulaContext,
    _args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    Ok(datetime_component(ctx, |ts| ts_to_date_parts(ts).3 as f64))
}

pub(crate) fn fn_minute(
    ctx: &FormulaContext,
    _args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    Ok(datetime_component(ctx, |ts| ts_to_date_parts(ts).4 as f64))
}

pub(crate) fn fn_time(
    ctx: &FormulaContext,
    _args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    Ok(datetime_component(ctx, |ts| {
        let (_, _, _, h, m, _s) = ts_to_date_parts(ts);
        (h * 100 + m) as f64
    }))
}

pub(crate) fn fn_weekday(
    ctx: &FormulaContext,
    _args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    Ok(datetime_component(ctx, |ts| {
        let days = ts / 86400;
        ((days % 7 + 4) % 7) as f64 // 1970-01-01 was Thursday(4), 0=Sun
    }))
}

pub(crate) fn fn_date_tdx(
    ctx: &FormulaContext,
    _args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    Ok(datetime_component(ctx, |ts| {
        let (y, m, d, _, _, _) = ts_to_date_parts(ts);
        ((y - 1900) * 10000 + m as i32 * 100 + d as i32) as f64
    }))
}
