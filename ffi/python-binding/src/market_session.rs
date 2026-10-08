//! Market-calendar / trading-session resolution.

use super::*;

#[pyfunction(name = "resolve_market_session")]
#[pyo3(signature = (market, timestamp, timezone=None, holidays=None, sessions=None, special_sessions=None))]
pub fn resolve_market_session_py(
    py: Python<'_>,
    market: &str,
    timestamp: i64,
    timezone: Option<&str>,
    holidays: Option<Vec<String>>,
    sessions: Option<Vec<(u32, u32)>>,
    special_sessions: Option<Vec<(String, Vec<(u32, u32)>)>>,
) -> PyResult<Option<Py<PyAny>>> {
    let preset = MarketCalendarPreset::parse(market)
        .map_err(|error| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}")))?;
    let mut calendar = TradingCalendar::for_market(preset);
    if let Some(timezone) = timezone {
        calendar = calendar.with_timezone(TimeZoneSpec::parse(timezone).map_err(|error| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}"))
        })?);
    }
    if let Some(holidays) = holidays {
        for holiday in holidays {
            calendar.add_holiday(&holiday).map_err(|error| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}"))
            })?;
        }
    }
    if let Some(sessions) = sessions {
        let sessions: Vec<SessionWindow> = sessions
            .into_iter()
            .map(|(open, close)| SessionWindow::new(open, close))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}")))?;
        calendar = calendar.with_sessions(&sessions);
    }
    if let Some(special_sessions) = special_sessions {
        for (date, sessions) in special_sessions {
            let sessions: Vec<SessionWindow> = sessions
                .into_iter()
                .map(|(open, close)| SessionWindow::new(open, close))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| {
                    PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}"))
                })?;
            calendar
                .set_special_sessions(&date, &sessions)
                .map_err(|error| {
                    PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}"))
                })?;
        }
    }
    let session = calendar
        .session_for_timestamp_local(timestamp)
        .map_err(|error| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}")))?;
    let Some(session) = session else {
        return Ok(None);
    };
    let output = pyo3::types::PyDict::new(py);
    output.set_item("session_day", session.session_day)?;
    output.set_item("session_index", session.session_index)?;
    output.set_item("open_timestamp", session.open_timestamp)?;
    output.set_item("close_timestamp", session.close_timestamp)?;
    output.set_item("market", preset.as_str())?;
    let timezone_name = match calendar.timezone() {
        TimeZoneSpec::Utc => "UTC".to_string(),
        TimeZoneSpec::AsiaShanghai => "Asia/Shanghai".to_string(),
        TimeZoneSpec::AsiaHongKong => "Asia/Hong_Kong".to_string(),
        TimeZoneSpec::AmericaNewYork => "America/New_York".to_string(),
        TimeZoneSpec::Fixed(offset_seconds) => format!("UTC{offset_seconds:+}"),
    };
    output.set_item("timezone", timezone_name)?;
    output.set_item("source", calendar.source())?;
    output.set_item("revision", calendar.revision())?;
    Ok(Some(output.into()))
}

#[pyfunction(name = "resolve_market_session_config")]
#[pyo3(signature = (config_json, timestamp))]
pub fn resolve_market_session_config_py(
    py: Python<'_>,
    config_json: &str,
    timestamp: i64,
) -> PyResult<Option<Py<PyAny>>> {
    let calendar = TradingCalendar::from_json(config_json)
        .map_err(|error| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}")))?;
    let session = calendar
        .session_for_timestamp_local(timestamp)
        .map_err(|error| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}")))?;
    let Some(session) = session else {
        return Ok(None);
    };
    let output = pyo3::types::PyDict::new(py);
    output.set_item("session_day", session.session_day)?;
    output.set_item("session_index", session.session_index)?;
    output.set_item("open_timestamp", session.open_timestamp)?;
    output.set_item("close_timestamp", session.close_timestamp)?;
    output.set_item("source", calendar.source())?;
    output.set_item("revision", calendar.revision())?;
    Ok(Some(output.into()))
}

#[pyfunction(name = "resolve_market_session_csv")]
#[pyo3(signature = (csv, market, timestamp, timezone=None))]
pub fn resolve_market_session_csv_py(
    py: Python<'_>,
    csv: &str,
    market: &str,
    timestamp: i64,
    timezone: Option<&str>,
) -> PyResult<Option<Py<PyAny>>> {
    let calendar = TradingCalendar::from_csv(csv, Some(market), timezone)
        .map_err(|error| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}")))?;
    let session = calendar
        .session_for_timestamp_local(timestamp)
        .map_err(|error| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}")))?;
    let Some(session) = session else {
        return Ok(None);
    };
    let output = pyo3::types::PyDict::new(py);
    output.set_item("session_day", session.session_day)?;
    output.set_item("session_index", session.session_index)?;
    output.set_item("open_timestamp", session.open_timestamp)?;
    output.set_item("close_timestamp", session.close_timestamp)?;
    output.set_item("source", calendar.source())?;
    output.set_item("revision", calendar.revision())?;
    Ok(Some(output.into()))
}
