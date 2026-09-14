//! Exchange/session calendar primitives for timestamped market data.
//!
//! The calendar deliberately has no external timezone database dependency.
//! It provides deterministic adapters for the supported exchange families and
//! accepts user-supplied holiday/session overrides, while applications that
//! need a full IANA database can resolve a zone at the feed boundary. This
//! keeps the core suitable for Rust, Python, Node and WASM adapters alike.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Calendar validation and lookup failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CalendarError {
    InvalidDate(String),
    InvalidSession(String),
    InvalidDuration(String),
    InvalidMarket(String),
    InvalidTimezone(String),
}

impl fmt::Display for CalendarError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDate(value) => write!(formatter, "invalid calendar date: {value}"),
            Self::InvalidSession(value) => write!(formatter, "invalid trading session: {value}"),
            Self::InvalidDuration(value) => write!(formatter, "invalid calendar duration: {value}"),
            Self::InvalidMarket(value) => write!(formatter, "invalid market calendar: {value}"),
            Self::InvalidTimezone(value) => write!(formatter, "invalid timezone: {value}"),
        }
    }
}

impl std::error::Error for CalendarError {}

pub type CalendarResult<T> = core::result::Result<T, CalendarError>;

/// Fixed-name timezone adapters used by exchange calendars.
///
/// The core intentionally keeps this set small and deterministic. Applications
/// that need a full IANA timezone database can resolve their zone externally
/// and pass [`TimeZoneSpec::Fixed`] or implement the same local-to-UTC
/// contract at the feed boundary. `AmericaNewYork` is included because its
/// DST transition changes U.S. equity session timestamps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum TimeZoneSpec {
    Utc,
    AsiaShanghai,
    AsiaHongKong,
    AmericaNewYork,
    Fixed(i32),
}

impl Default for TimeZoneSpec {
    fn default() -> Self {
        Self::Utc
    }
}

impl TimeZoneSpec {
    pub fn fixed(offset_seconds: i32) -> CalendarResult<Self> {
        if !(-86_400..=86_400).contains(&offset_seconds) {
            return Err(CalendarError::InvalidTimezone(offset_seconds.to_string()));
        }
        Ok(Self::Fixed(offset_seconds))
    }

    /// Parse common IANA names and fixed UTC offsets such as `UTC+08:00`.
    pub fn parse(value: &str) -> CalendarResult<Self> {
        match value.trim() {
            "UTC" | "Etc/UTC" | "GMT" => Ok(Self::Utc),
            "Asia/Shanghai" | "PRC" => Ok(Self::AsiaShanghai),
            "Asia/Hong_Kong" | "Hongkong" => Ok(Self::AsiaHongKong),
            "America/New_York" | "US/Eastern" => Ok(Self::AmericaNewYork),
            value if value.starts_with("UTC") || value.starts_with("GMT") => {
                let suffix = &value[3..];
                let sign = match suffix.as_bytes().first() {
                    Some(b'+') => 1_i32,
                    Some(b'-') => -1_i32,
                    _ => {
                        return Err(CalendarError::InvalidTimezone(value.to_string()));
                    }
                };
                let digits = &suffix[1..];
                if digits.contains(':') {
                    let parts = digits.split_once(':').expect("checked colon");
                    if !parts.0.as_bytes().iter().all(u8::is_ascii_digit)
                        || !parts.1.as_bytes().iter().all(u8::is_ascii_digit)
                    {
                        return Err(CalendarError::InvalidTimezone(value.to_string()));
                    }
                } else if !digits.as_bytes().iter().all(u8::is_ascii_digit) {
                    return Err(CalendarError::InvalidTimezone(value.to_string()));
                }
                let (hours, minutes) = if let Some((hours, minutes)) = digits.split_once(':') {
                    (
                        hours
                            .parse::<i32>()
                            .map_err(|_| CalendarError::InvalidTimezone(value.to_string()))?,
                        minutes
                            .parse::<i32>()
                            .map_err(|_| CalendarError::InvalidTimezone(value.to_string()))?,
                    )
                } else if (1..=4).contains(&digits.len()) {
                    let hours_len = if digits.len() <= 2 { digits.len() } else { 2 };
                    let hours = digits[0..hours_len]
                        .parse::<i32>()
                        .map_err(|_| CalendarError::InvalidTimezone(value.to_string()))?;
                    let minutes = if digits.len() > 2 {
                        digits[hours_len..]
                            .parse::<i32>()
                            .map_err(|_| CalendarError::InvalidTimezone(value.to_string()))?
                    } else {
                        0
                    };
                    (hours, minutes)
                } else {
                    return Err(CalendarError::InvalidTimezone(value.to_string()));
                };
                if minutes >= 60 {
                    return Err(CalendarError::InvalidTimezone(value.to_string()));
                }
                Self::fixed(sign * (hours * 3_600 + minutes * 60))
            }
            value => Err(CalendarError::InvalidTimezone(value.to_string())),
        }
    }

    pub fn identifier(self) -> String {
        match self {
            Self::Utc => "UTC".to_string(),
            Self::AsiaShanghai => "Asia/Shanghai".to_string(),
            Self::AsiaHongKong => "Asia/Hong_Kong".to_string(),
            Self::AmericaNewYork => "America/New_York".to_string(),
            Self::Fixed(offset) => {
                let sign = if offset >= 0 { '+' } else { '-' };
                let absolute = offset.unsigned_abs();
                format!(
                    "UTC{sign}{:02}:{:02}",
                    absolute / 3_600,
                    absolute % 3_600 / 60
                )
            }
        }
    }

    pub fn offset_at_utc(self, timestamp: i64) -> i32 {
        match self {
            Self::Utc => 0,
            Self::AsiaShanghai | Self::AsiaHongKong => 8 * 3_600,
            Self::Fixed(offset) => offset,
            Self::AmericaNewYork => {
                let standard = -5 * 3_600;
                let approximate_day = (timestamp + standard as i64).div_euclid(86_400);
                let (year, _, _) = civil_from_days(approximate_day);
                let start_day = nth_weekday_of_month(year, 3, 6, 2);
                let end_day = nth_weekday_of_month(year, 11, 6, 1);
                let dst_start = start_day * 86_400 + 2 * 3_600 - standard as i64;
                let daylight = -4 * 3_600;
                let dst_end = end_day * 86_400 + 2 * 3_600 - daylight as i64;
                if timestamp >= dst_start && timestamp < dst_end {
                    daylight
                } else {
                    standard
                }
            }
        }
    }

    fn standard_offset(self) -> i32 {
        match self {
            Self::AmericaNewYork => -5 * 3_600,
            _ => self.offset_at_utc(0),
        }
    }

    fn local_day_seconds(self, timestamp: i64) -> (i64, u32) {
        let offset = self.offset_at_utc(timestamp);
        let local = timestamp.saturating_add(offset as i64);
        (local.div_euclid(86_400), local.rem_euclid(86_400) as u32)
    }

    fn timestamp_from_local(self, day: i64, seconds: u32) -> i64 {
        let local = day * 86_400 + seconds as i64;
        let mut offset = self.standard_offset();
        for _ in 0..3 {
            let timestamp = local - offset as i64;
            offset = self.offset_at_utc(timestamp);
        }
        local - offset as i64
    }
}

/// Regular-session presets for the supported market families.
///
/// Presets intentionally describe exchange session structure and timezone.
/// Exact holiday files, temporary closures and product-specific futures
/// sessions are supplied through [`TradingCalendar::with_holiday`] and
/// [`TradingCalendar::with_special_sessions`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum MarketCalendarPreset {
    Ashare,
    ChinaFutures,
    HongKong,
    UsEquity,
    Crypto,
}

impl MarketCalendarPreset {
    pub fn parse(value: &str) -> CalendarResult<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "a_share" | "a-share" | "ashare" | "cn_stock" | "cn-stocks" | "a股" | "中国a股" => {
                Ok(Self::Ashare)
            }
            "china_futures" | "china-futures" | "cn_futures" | "cn-futures" | "期货"
            | "中国期货" => Ok(Self::ChinaFutures),
            "hong_kong" | "hong-kong" | "hk" | "hk_stock" | "hk-stocks" | "港股" => {
                Ok(Self::HongKong)
            }
            "us_equity" | "us-equity" | "us_stock" | "us-stocks" | "nyse" | "nasdaq" => {
                Ok(Self::UsEquity)
            }
            "美股" => Ok(Self::UsEquity),
            "crypto" | "crypto_24x7" | "crypto-24x7" | "加密货币" => Ok(Self::Crypto),
            value => Err(CalendarError::InvalidMarket(value.to_string())),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ashare => "a_share",
            Self::ChinaFutures => "china_futures",
            Self::HongKong => "hong_kong",
            Self::UsEquity => "us_equity",
            Self::Crypto => "crypto",
        }
    }
}

/// An intraday session expressed as seconds after local midnight.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SessionWindow {
    pub open_seconds: u32,
    pub close_seconds: u32,
}

/// A date-specific replacement for the regular sessions.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CalendarSessionOverride {
    pub date: String,
    #[cfg_attr(feature = "serde", serde(default))]
    pub sessions: Vec<SessionWindow>,
}

impl CalendarSessionOverride {
    pub fn new(date: impl Into<String>, sessions: Vec<SessionWindow>) -> Self {
        Self {
            date: date.into(),
            sessions,
        }
    }
}

/// Versioned, serializable calendar input used by exchange adapters.
///
/// `market` selects one of the built-in session templates. `holidays` and
/// `special_sessions` are intentionally explicit so an exchange-published
/// annual file can override the template without changing indicator code.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CalendarDefinition {
    #[cfg_attr(feature = "serde", serde(default))]
    pub market: Option<String>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub timezone: Option<String>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub weekend_mask: Option<u8>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub holidays: Vec<String>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub sessions: Option<Vec<SessionWindow>>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub special_sessions: Vec<CalendarSessionOverride>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub source: Option<String>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub revision: Option<String>,
}

impl CalendarDefinition {
    pub fn for_market(market: impl Into<String>) -> Self {
        Self {
            market: Some(market.into()),
            ..Self::default()
        }
    }

    pub fn with_source(mut self, source: impl Into<String>, revision: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self.revision = Some(revision.into());
        self
    }

    #[cfg(feature = "serde")]
    pub fn from_json(json: &str) -> CalendarResult<Self> {
        serde_json::from_str(json)
            .map_err(|error| CalendarError::InvalidDate(format!("calendar JSON: {error}")))
    }

    /// Parse a compact exchange-published annual CSV file.
    ///
    /// The accepted columns are `date,status,sessions`. The header is
    /// optional. `status` accepts `open`/`trading`/`1` and
    /// `closed`/`holiday`/`0`; an open row with a non-empty `sessions` column
    /// becomes a date-specific session override. Sessions use
    /// `HH:MM-HH:MM;HH:MM-HH:MM`, which also supports cross-midnight futures
    /// sessions. Blank lines and `#` comments are ignored.
    pub fn from_csv(
        csv: &str,
        market: Option<&str>,
        timezone: Option<&str>,
    ) -> CalendarResult<Self> {
        let mut definition = Self {
            market: market.map(str::to_string),
            timezone: timezone.map(str::to_string),
            ..Self::default()
        };
        for (line_number, raw_line) in csv.lines().enumerate() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let fields: Vec<&str> = line.split(',').map(str::trim).collect();
            if fields
                .first()
                .is_some_and(|field| field.eq_ignore_ascii_case("date"))
            {
                continue;
            }
            let date = fields.first().copied().ok_or_else(|| {
                CalendarError::InvalidDate(format!(
                    "calendar CSV line {} is empty",
                    line_number + 1
                ))
            })?;
            parse_date(date).map_err(|_| {
                CalendarError::InvalidDate(format!(
                    "calendar CSV line {} has invalid date: {date}",
                    line_number + 1
                ))
            })?;
            let status = fields
                .get(1)
                .copied()
                .unwrap_or_default()
                .to_ascii_lowercase();
            let sessions = fields.get(2).copied().unwrap_or_default();
            let closed = matches!(status.as_str(), "closed" | "holiday" | "0" | "false");
            let open = matches!(status.as_str(), "open" | "trading" | "1" | "true");
            if closed || status.is_empty() && sessions.is_empty() {
                definition.holidays.push(date.to_string());
                continue;
            }
            if !open && !status.is_empty() {
                return Err(CalendarError::InvalidSession(format!(
                    "calendar CSV line {} has unknown status: {}",
                    line_number + 1,
                    status
                )));
            }
            if !sessions.is_empty() {
                definition
                    .special_sessions
                    .push(CalendarSessionOverride::new(
                        date,
                        parse_session_list(sessions).map_err(|error| {
                            CalendarError::InvalidSession(format!(
                                "calendar CSV line {}: {error}",
                                line_number + 1
                            ))
                        })?,
                    ));
            }
        }
        Ok(definition)
    }
}

impl SessionWindow {
    pub fn new(open_seconds: u32, close_seconds: u32) -> CalendarResult<Self> {
        if open_seconds >= 86_400 || close_seconds >= 86_400 {
            return Err(CalendarError::InvalidSession(
                "session endpoints must be below 86400 seconds".to_string(),
            ));
        }
        if open_seconds == close_seconds {
            return Err(CalendarError::InvalidSession(
                "session open and close must differ".to_string(),
            ));
        }
        Ok(Self {
            open_seconds,
            close_seconds,
        })
    }

    pub fn crosses_midnight(self) -> bool {
        self.close_seconds < self.open_seconds
    }
}

fn parse_session_list(value: &str) -> CalendarResult<Vec<SessionWindow>> {
    let mut sessions = Vec::new();
    for range in value
        .split(';')
        .map(str::trim)
        .filter(|range| !range.is_empty())
    {
        let (open, close) = range.split_once('-').ok_or_else(|| {
            CalendarError::InvalidSession(format!("session must be OPEN-CLOSE: {range}"))
        })?;
        sessions.push(SessionWindow::new(parse_clock(open)?, parse_clock(close)?)?);
    }
    if sessions.is_empty() {
        return Err(CalendarError::InvalidSession(
            "session list must contain at least one range".to_string(),
        ));
    }
    Ok(sessions)
}

fn parse_clock(value: &str) -> CalendarResult<u32> {
    let parts: Vec<&str> = value.trim().split(':').collect();
    if !(2..=3).contains(&parts.len()) {
        return Err(CalendarError::InvalidSession(format!(
            "invalid clock value: {value}"
        )));
    }
    let hour = parts[0]
        .parse::<u32>()
        .map_err(|_| CalendarError::InvalidSession(format!("invalid clock value: {value}")))?;
    let minute = parts[1]
        .parse::<u32>()
        .map_err(|_| CalendarError::InvalidSession(format!("invalid clock value: {value}")))?;
    let second = parts
        .get(2)
        .map(|part| {
            part.parse::<u32>()
                .map_err(|_| CalendarError::InvalidSession(format!("invalid clock value: {value}")))
        })
        .transpose()?
        .unwrap_or(0);
    if hour >= 24 || minute >= 60 || second >= 60 {
        return Err(CalendarError::InvalidSession(format!(
            "clock value out of range: {value}"
        )));
    }
    Ok(hour * 3_600 + minute * 60 + second)
}

/// A resolved session occurrence for a timestamp.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionMatch {
    /// Calendar date on which the session opened, in `YYYY-MM-DD` form only
    /// through the query API; the numeric day is exposed for efficient joins.
    pub session_day: i64,
    pub session_index: usize,
    pub open_timestamp: i64,
    pub close_timestamp: i64,
}

/// Deterministic, configurable market calendar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TradingCalendar {
    market: Option<MarketCalendarPreset>,
    timezone: TimeZoneSpec,
    /// Bit mask where bit 0 is Monday and bit 6 is Sunday.
    weekend_mask: u8,
    holidays: BTreeSet<i64>,
    sessions: Vec<SessionWindow>,
    /// Per-date overrides. An empty vector means fully closed; non-empty
    /// vectors replace the regular sessions and therefore model half days or
    /// product-specific temporary sessions.
    special_sessions: BTreeMap<i64, Vec<SessionWindow>>,
    source: Option<String>,
    revision: Option<String>,
}

impl Default for TradingCalendar {
    fn default() -> Self {
        Self::weekdays()
    }
}

impl TradingCalendar {
    /// A conventional Monday-Friday calendar with no intraday session split.
    pub fn weekdays() -> Self {
        Self {
            market: None,
            timezone: TimeZoneSpec::Utc,
            weekend_mask: (1 << 5) | (1 << 6),
            holidays: BTreeSet::new(),
            sessions: Vec::new(),
            special_sessions: BTreeMap::new(),
            source: None,
            revision: None,
        }
    }

    /// A 24/7 calendar useful for crypto and always-on feeds.
    pub fn always_open() -> Self {
        Self {
            market: None,
            timezone: TimeZoneSpec::Utc,
            weekend_mask: 0,
            holidays: BTreeSet::new(),
            sessions: Vec::new(),
            special_sessions: BTreeMap::new(),
            source: None,
            revision: None,
        }
    }

    /// Build a regular-session preset for a supported market family.
    pub fn for_market(market: MarketCalendarPreset) -> Self {
        let mut calendar = match market {
            MarketCalendarPreset::Ashare => Self::weekdays()
                .with_timezone(TimeZoneSpec::AsiaShanghai)
                .with_sessions(&[
                    SessionWindow::new(9 * 3_600 + 30 * 60, 11 * 3_600 + 30 * 60)
                        .expect("valid A-share morning session"),
                    SessionWindow::new(13 * 3_600, 15 * 3_600)
                        .expect("valid A-share afternoon session"),
                ]),
            MarketCalendarPreset::ChinaFutures => Self::weekdays()
                .with_timezone(TimeZoneSpec::AsiaShanghai)
                .with_sessions(&[
                    SessionWindow::new(9 * 3_600, 10 * 3_600 + 15 * 60)
                        .expect("valid China futures morning session"),
                    SessionWindow::new(10 * 3_600 + 30 * 60, 11 * 3_600 + 30 * 60)
                        .expect("valid China futures late morning session"),
                    SessionWindow::new(13 * 3_600 + 30 * 60, 15 * 3_600)
                        .expect("valid China futures afternoon session"),
                    SessionWindow::new(21 * 3_600, 23 * 3_600)
                        .expect("valid China futures night session"),
                ]),
            MarketCalendarPreset::HongKong => Self::weekdays()
                .with_timezone(TimeZoneSpec::AsiaHongKong)
                .with_sessions(&[
                    SessionWindow::new(9 * 3_600 + 30 * 60, 12 * 3_600)
                        .expect("valid Hong Kong morning session"),
                    SessionWindow::new(13 * 3_600, 16 * 3_600)
                        .expect("valid Hong Kong afternoon session"),
                ]),
            MarketCalendarPreset::UsEquity => Self::weekdays()
                .with_timezone(TimeZoneSpec::AmericaNewYork)
                .with_sessions(&[SessionWindow::new(9 * 3_600 + 30 * 60, 16 * 3_600)
                    .expect("valid U.S. equity session")]),
            MarketCalendarPreset::Crypto => Self::always_open(),
        };
        calendar.market = Some(market);
        calendar
    }

    /// Build a calendar from an exchange adapter's versioned definition.
    pub fn from_definition(definition: &CalendarDefinition) -> CalendarResult<Self> {
        let mut calendar = match definition.market.as_deref() {
            Some(market) => Self::for_market_name(market)?,
            None => Self::weekdays(),
        };
        if let Some(timezone) = definition.timezone.as_deref() {
            calendar = calendar.with_timezone(TimeZoneSpec::parse(timezone)?);
        }
        if let Some(weekend_mask) = definition.weekend_mask {
            calendar = calendar.with_weekend_mask(weekend_mask);
        }
        if let Some(sessions) = definition.sessions.as_deref() {
            for session in sessions {
                if session.open_seconds >= 86_400 || session.close_seconds >= 86_400 {
                    return Err(CalendarError::InvalidSession(format!(
                        "{}-{}",
                        session.open_seconds, session.close_seconds
                    )));
                }
            }
            calendar = calendar.with_sessions(sessions);
        }
        for holiday in &definition.holidays {
            calendar.add_holiday(holiday)?;
        }
        for special in &definition.special_sessions {
            calendar.set_special_sessions(&special.date, &special.sessions)?;
        }
        calendar.source = definition.source.clone();
        calendar.revision = definition.revision.clone();
        Ok(calendar)
    }

    #[cfg(feature = "serde")]
    pub fn from_json(json: &str) -> CalendarResult<Self> {
        Self::from_definition(&CalendarDefinition::from_json(json)?)
    }

    /// Build a calendar directly from an exchange-published annual CSV file.
    pub fn from_csv(
        csv: &str,
        market: Option<&str>,
        timezone: Option<&str>,
    ) -> CalendarResult<Self> {
        Self::from_definition(&CalendarDefinition::from_csv(csv, market, timezone)?)
    }

    pub fn for_market_name(value: &str) -> CalendarResult<Self> {
        Ok(Self::for_market(MarketCalendarPreset::parse(value)?))
    }

    pub fn market(&self) -> Option<MarketCalendarPreset> {
        self.market
    }

    pub fn timezone(&self) -> TimeZoneSpec {
        self.timezone
    }

    pub fn source(&self) -> Option<&str> {
        self.source.as_deref()
    }

    pub fn revision(&self) -> Option<&str> {
        self.revision.as_deref()
    }

    pub fn with_metadata(
        mut self,
        source: Option<impl Into<String>>,
        revision: Option<impl Into<String>>,
    ) -> Self {
        self.source = source.map(Into::into);
        self.revision = revision.map(Into::into);
        self
    }

    /// Replace the timezone adapter while preserving sessions and overrides.
    pub fn with_timezone(mut self, timezone: TimeZoneSpec) -> Self {
        self.timezone = timezone;
        self
    }

    /// Replace regular sessions. This is useful for a specific futures
    /// product whose night session differs from the generic preset.
    pub fn with_sessions(mut self, sessions: &[SessionWindow]) -> Self {
        self.sessions = sessions.to_vec();
        self.sessions.sort_unstable();
        self
    }

    /// Replace the weekend mask. Bit 0 is Monday, bit 6 is Sunday.
    pub fn with_weekend_mask(mut self, weekend_mask: u8) -> Self {
        self.weekend_mask = weekend_mask & 0x7f;
        self
    }

    /// Add an ISO holiday and return the updated calendar.
    pub fn with_holiday(mut self, date: &str) -> CalendarResult<Self> {
        self.add_holiday(date)?;
        Ok(self)
    }

    /// Add one holiday without changing existing session rules.
    pub fn add_holiday(&mut self, date: &str) -> CalendarResult<()> {
        self.holidays.insert(parse_date(date)?);
        Ok(())
    }

    /// Add many holiday dates from an exchange-published holiday file.
    pub fn with_holidays(mut self, dates: &[&str]) -> CalendarResult<Self> {
        for date in dates {
            self.add_holiday(date)?;
        }
        Ok(self)
    }

    /// Replace the sessions for one date. Pass an empty slice for a closure.
    pub fn with_special_sessions(
        mut self,
        date: &str,
        sessions: &[SessionWindow],
    ) -> CalendarResult<Self> {
        self.set_special_sessions(date, sessions)?;
        Ok(self)
    }

    pub fn set_special_sessions(
        &mut self,
        date: &str,
        sessions: &[SessionWindow],
    ) -> CalendarResult<()> {
        let day = parse_date(date)?;
        let mut sessions = sessions.to_vec();
        sessions.sort_unstable();
        self.special_sessions.insert(day, sessions);
        Ok(())
    }

    pub fn close_on(&mut self, date: &str) -> CalendarResult<()> {
        self.set_special_sessions(date, &[])
    }

    /// Add an intraday session. Sessions are kept in opening-time order.
    pub fn add_session(&mut self, session: SessionWindow) {
        self.sessions.push(session);
        self.sessions.sort_unstable();
    }

    pub fn sessions(&self) -> &[SessionWindow] {
        &self.sessions
    }

    pub fn special_sessions(&self, date: &str) -> CalendarResult<Option<&[SessionWindow]>> {
        let day = parse_date(date)?;
        Ok(self.special_sessions.get(&day).map(Vec::as_slice))
    }

    pub fn is_trading_day(&self, date: &str) -> CalendarResult<bool> {
        Ok(self.is_trading_day_number(parse_date(date)?))
    }

    /// Resolve an ISO date by adding or subtracting trading days.
    pub fn shift_trading_day(&self, date: &str, offset: i32) -> CalendarResult<String> {
        let mut day = parse_date(date)?;
        let step = if offset < 0 { -1 } else { 1 };
        let mut remaining = offset.unsigned_abs();
        while remaining > 0 {
            day += step;
            if self.is_trading_day_number(day) {
                remaining -= 1;
            }
        }
        Ok(format_date(day))
    }

    /// Return the session containing a Unix timestamp.
    ///
    /// `utc_offset_seconds` converts the feed timestamp into local exchange
    /// time. A cross-midnight session is assigned to the day on which it
    /// opened, so night bars remain grouped with the correct trading day.
    pub fn session_for_timestamp(
        &self,
        timestamp: i64,
        utc_offset_seconds: i32,
    ) -> CalendarResult<Option<SessionMatch>> {
        self.session_for_timestamp_with_timezone(timestamp, TimeZoneSpec::Fixed(utc_offset_seconds))
    }

    /// Resolve a timestamp using the calendar's configured timezone adapter.
    pub fn session_for_timestamp_local(
        &self,
        timestamp: i64,
    ) -> CalendarResult<Option<SessionMatch>> {
        self.session_for_timestamp_with_timezone(timestamp, self.timezone)
    }

    pub(crate) fn session_for_timestamp_for_analysis(
        &self,
        timestamp: i64,
        fallback_offset_seconds: i32,
    ) -> CalendarResult<Option<SessionMatch>> {
        if self.timezone == TimeZoneSpec::Utc {
            self.session_for_timestamp(timestamp, fallback_offset_seconds)
        } else {
            self.session_for_timestamp_local(timestamp)
        }
    }

    /// Test a timestamp using the calendar's configured timezone adapter.
    pub fn is_trading_timestamp_local(&self, timestamp: i64) -> CalendarResult<bool> {
        Ok(self.session_for_timestamp_local(timestamp)?.is_some())
    }

    pub fn is_trading_timestamp(
        &self,
        timestamp: i64,
        utc_offset_seconds: i32,
    ) -> CalendarResult<bool> {
        Ok(self
            .session_for_timestamp(timestamp, utc_offset_seconds)?
            .is_some())
    }

    fn session_for_timestamp_with_timezone(
        &self,
        timestamp: i64,
        timezone: TimeZoneSpec,
    ) -> CalendarResult<Option<SessionMatch>> {
        let (local_day, seconds) = timezone.local_day_seconds(timestamp);
        for open_day in [local_day, local_day - 1] {
            let Some(sessions) = self.sessions_for_day_number(open_day) else {
                continue;
            };
            if sessions.is_empty() {
                if open_day == local_day {
                    return Ok(Some(SessionMatch {
                        session_day: open_day,
                        session_index: 0,
                        open_timestamp: timezone.timestamp_from_local(open_day, 0),
                        close_timestamp: timezone.timestamp_from_local(open_day + 1, 0),
                    }));
                }
                continue;
            }
            for (index, session) in sessions.iter().copied().enumerate() {
                let contains = if session.crosses_midnight() {
                    (local_day == open_day && seconds >= session.open_seconds)
                        || (local_day == open_day + 1 && seconds < session.close_seconds)
                } else {
                    local_day == open_day
                        && seconds >= session.open_seconds
                        && seconds < session.close_seconds
                };
                if !contains {
                    continue;
                }
                let close_day = if session.crosses_midnight() {
                    open_day + 1
                } else {
                    open_day
                };
                return Ok(Some(SessionMatch {
                    session_day: open_day,
                    session_index: index,
                    open_timestamp: timezone.timestamp_from_local(open_day, session.open_seconds),
                    close_timestamp: timezone
                        .timestamp_from_local(close_day, session.close_seconds),
                }));
            }
        }
        Ok(None)
    }

    fn sessions_for_day_number(&self, day: i64) -> Option<&[SessionWindow]> {
        if let Some(sessions) = self.special_sessions.get(&day) {
            return (!sessions.is_empty()).then_some(sessions.as_slice());
        }
        if !self.is_base_trading_day_number(day) {
            return None;
        }
        Some(self.sessions.as_slice())
    }

    fn is_trading_day_number(&self, day: i64) -> bool {
        self.sessions_for_day_number(day).is_some()
    }

    fn is_base_trading_day_number(&self, day: i64) -> bool {
        let weekday = ((day + 3).rem_euclid(7)) as u8;
        self.weekend_mask & (1 << weekday) == 0
            && !self.holidays.contains(&day)
            && !self.is_builtin_holiday(day)
    }

    fn is_builtin_holiday(&self, day: i64) -> bool {
        match self.market {
            Some(MarketCalendarPreset::UsEquity) => is_us_equity_holiday(day),
            _ => false,
        }
    }
}

fn parse_date(date: &str) -> CalendarResult<i64> {
    let bytes = date.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return Err(CalendarError::InvalidDate(date.to_string()));
    }
    let year = date[0..4]
        .parse::<i64>()
        .map_err(|_| CalendarError::InvalidDate(date.to_string()))?;
    let month = date[5..7]
        .parse::<u32>()
        .map_err(|_| CalendarError::InvalidDate(date.to_string()))?;
    let day = date[8..10]
        .parse::<u32>()
        .map_err(|_| CalendarError::InvalidDate(date.to_string()))?;
    if !(1..=12).contains(&month) || day == 0 || day > days_in_month(year, month) {
        return Err(CalendarError::InvalidDate(date.to_string()));
    }
    Ok(days_from_civil(year, month, day))
}

fn format_date(day: i64) -> String {
    let (year, month, date) = civil_from_days(day);
    format!("{year:04}-{month:02}-{date:02}")
}

fn days_in_month(year: i64, month: u32) -> u32 {
    match month {
        2 if year.rem_euclid(4) == 0
            && (year.rem_euclid(100) != 0 || year.rem_euclid(400) == 0) =>
        {
            29
        }
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

fn weekday_of(day: i64) -> u8 {
    ((day + 3).rem_euclid(7)) as u8
}

fn nth_weekday_of_month(year: i64, month: u32, weekday: u8, nth: u32) -> i64 {
    let first = days_from_civil(year, month, 1);
    first + (weekday as i64 - weekday_of(first) as i64).rem_euclid(7) + 7 * (nth as i64 - 1)
}

fn last_weekday_of_month(year: i64, month: u32, weekday: u8) -> i64 {
    let last = days_from_civil(year, month, days_in_month(year, month));
    last - (weekday_of(last) as i64 - weekday as i64).rem_euclid(7)
}

fn observed_fixed_holiday(year: i64, month: u32, date: u32) -> i64 {
    let day = days_from_civil(year, month, date);
    match weekday_of(day) {
        5 => day - 1,
        6 => day + 1,
        _ => day,
    }
}

fn easter_sunday(year: i64) -> i64 {
    // Gregorian computus, valid for the exchange years covered by this crate.
    let a = year.rem_euclid(19);
    let b = year.div_euclid(100);
    let c = year.rem_euclid(100);
    let d = b.div_euclid(4);
    let e = b.rem_euclid(4);
    let f = (b + 8).div_euclid(25);
    let g = (b - f + 1).div_euclid(3);
    let h = (19 * a + b - d - g + 15).rem_euclid(30);
    let i = c.div_euclid(4);
    let k = c.rem_euclid(4);
    let l = (32 + 2 * e + 2 * i - h - k).rem_euclid(7);
    let m = (a + 11 * h + 22 * l).div_euclid(451);
    let month = (h + l - 7 * m + 114).div_euclid(31) as u32;
    let date = ((h + l - 7 * m + 114).rem_euclid(31) + 1) as u32;
    days_from_civil(year, month, date)
}

fn is_us_equity_holiday(day: i64) -> bool {
    let (year, month, date) = civil_from_days(day);
    for candidate_year in [year - 1, year, year + 1] {
        let fixed = [
            observed_fixed_holiday(candidate_year, 1, 1),
            observed_fixed_holiday(candidate_year, 7, 4),
            observed_fixed_holiday(candidate_year, 12, 25),
        ];
        if fixed.contains(&day)
            || (candidate_year >= 2022 && day == observed_fixed_holiday(candidate_year, 6, 19))
            || day == easter_sunday(candidate_year) - 2
        {
            return true;
        }
    }
    day == nth_weekday_of_month(year, 1, 0, 3)
        || day == nth_weekday_of_month(year, 2, 0, 3)
        || day == last_weekday_of_month(year, 5, 0)
        || day == nth_weekday_of_month(year, 9, 0, 1)
        || day == nth_weekday_of_month(year, 11, 3, 4)
        || (month == 1 && date == 1 && weekday_of(day) >= 5)
}

// Howard Hinnant's proleptic Gregorian civil-date conversion, kept local to
// avoid pulling a timezone/date database into the numerical core.
fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let year = year - i64::from(month <= 2);
    let era = (if year >= 0 { year } else { year - 399 }).div_euclid(400);
    let year_of_era = year - era * 400;
    let month = i64::from(month);
    let day_of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + i64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let days = days + 719_468;
    let era = (if days >= 0 { days } else { days - 146_096 }).div_euclid(146_097);
    let day_of_era = days - era * 146_097;
    let year_of_era = (day_of_era - day_of_era / 1_460 + day_of_era / 36_524
        - day_of_era / 146_096)
        .div_euclid(365);
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_part = (5 * day_of_year + 2).div_euclid(153);
    let day = day_of_year - (153 * month_part + 2).div_euclid(5) + 1;
    let month = month_part + if month_part < 10 { 3 } else { -9 };
    let year = year + i64::from(month <= 2);
    (year, month as u32, day as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weekdays_and_holidays_are_deterministic() {
        let mut calendar = TradingCalendar::weekdays();
        assert!(calendar.is_trading_day("2024-01-05").unwrap());
        assert!(!calendar.is_trading_day("2024-01-06").unwrap());
        calendar.add_holiday("2024-01-05").unwrap();
        assert!(!calendar.is_trading_day("2024-01-05").unwrap());
        assert_eq!(
            calendar.shift_trading_day("2024-01-04", 1).unwrap(),
            "2024-01-08"
        );
    }

    #[test]
    fn session_lookup_handles_breaks_and_night_sessions() {
        let mut calendar = TradingCalendar::weekdays();
        calendar.add_session(SessionWindow::new(9 * 3600 + 30 * 60, 11 * 3600 + 30 * 60).unwrap());
        calendar.add_session(SessionWindow::new(13 * 3600, 15 * 3600).unwrap());
        calendar.add_session(SessionWindow::new(21 * 3600, 2 * 3600).unwrap());

        let morning = 1_704_447_000_i64; // 2024-01-05 09:30 UTC
        assert!(calendar.is_trading_timestamp(morning, 0).unwrap());
        let break_time = morning + 3 * 3600;
        assert!(!calendar.is_trading_timestamp(break_time, 0).unwrap());
        let night = 1_704_488_400_i64; // 2024-01-05 21:00 UTC
        let found = calendar.session_for_timestamp(night, 0).unwrap();
        assert!(found.is_some());
        assert!(found.unwrap().close_timestamp > found.unwrap().open_timestamp);
    }

    #[test]
    fn rejects_invalid_dates_and_sessions() {
        assert!(TradingCalendar::weekdays()
            .is_trading_day("2024-02-30")
            .is_err());
        assert!(SessionWindow::new(86_400, 1).is_err());
        assert!(SessionWindow::new(1, 1).is_err());
    }

    #[test]
    fn market_presets_and_configured_overrides_are_explicit() {
        let mut calendar = TradingCalendar::for_market(MarketCalendarPreset::Ashare);
        assert_eq!(calendar.market(), Some(MarketCalendarPreset::Ashare));
        assert_eq!(calendar.timezone(), TimeZoneSpec::AsiaShanghai);
        let morning = TimeZoneSpec::AsiaShanghai
            .timestamp_from_local(days_from_civil(2024, 1, 5), 9 * 3_600 + 30 * 60);
        assert!(calendar.is_trading_timestamp_local(morning).unwrap());

        calendar.close_on("2024-01-05").unwrap();
        assert!(!calendar.is_trading_day("2024-01-05").unwrap());
        calendar
            .set_special_sessions(
                "2024-01-08",
                &[SessionWindow::new(9 * 3_600 + 30 * 60, 12 * 3_600).unwrap()],
            )
            .unwrap();
        let half_day = TimeZoneSpec::AsiaShanghai
            .timestamp_from_local(days_from_civil(2024, 1, 8), 11 * 3_600 + 59 * 60);
        assert!(calendar.is_trading_timestamp_local(half_day).unwrap());
        let after_close = TimeZoneSpec::AsiaShanghai
            .timestamp_from_local(days_from_civil(2024, 1, 8), 12 * 3_600);
        assert!(!calendar.is_trading_timestamp_local(after_close).unwrap());
    }

    #[test]
    fn us_equity_timezone_tracks_dst_and_holidays() {
        let calendar = TradingCalendar::for_market(MarketCalendarPreset::UsEquity);
        let before_dst = TimeZoneSpec::AmericaNewYork
            .timestamp_from_local(days_from_civil(2024, 3, 8), 9 * 3_600 + 30 * 60);
        let after_dst = TimeZoneSpec::AmericaNewYork
            .timestamp_from_local(days_from_civil(2024, 3, 11), 9 * 3_600 + 30 * 60);
        assert_eq!(after_dst - before_dst, 3 * 86_400 - 3_600);
        assert_eq!(
            TimeZoneSpec::AmericaNewYork.offset_at_utc(before_dst),
            -5 * 3_600
        );
        assert_eq!(
            TimeZoneSpec::AmericaNewYork.offset_at_utc(after_dst),
            -4 * 3_600
        );
        assert!(!calendar.is_trading_day("2024-07-04").unwrap());
        assert!(calendar.is_trading_day("2024-07-05").unwrap());
    }

    #[test]
    fn market_and_timezone_names_are_parseable() {
        assert_eq!(
            MarketCalendarPreset::parse("hk").unwrap(),
            MarketCalendarPreset::HongKong
        );
        assert_eq!(
            TimeZoneSpec::parse("UTC+08:00").unwrap(),
            TimeZoneSpec::Fixed(8 * 3_600)
        );
        assert!(TimeZoneSpec::parse("Not/AZone").is_err());
        assert!(MarketCalendarPreset::parse("unknown").is_err());
    }

    #[test]
    fn annual_csv_import_supports_closures_and_special_sessions() {
        let csv =
            "date,status,sessions\n2026-01-01,closed,\n2026-01-02,open,09:30-11:30;13:00-15:00\n";
        let calendar =
            TradingCalendar::from_csv(csv, Some("a_share"), Some("Asia/Shanghai")).unwrap();

        assert!(!calendar.is_trading_day("2026-01-01").unwrap());
        assert_eq!(
            calendar
                .special_sessions("2026-01-02")
                .unwrap()
                .unwrap()
                .len(),
            2
        );
        assert!(calendar
            .is_trading_timestamp_local(
                TimeZoneSpec::AsiaShanghai
                    .timestamp_from_local(days_from_civil(2026, 1, 2), 14 * 3_600)
            )
            .unwrap());
    }

    #[test]
    fn annual_csv_import_rejects_unknown_status_and_clock() {
        assert!(TradingCalendar::from_csv(
            "date,status,sessions\n2026-01-02,maybe,\n",
            Some("a_share"),
            None,
        )
        .is_err());
        assert!(TradingCalendar::from_csv(
            "date,status,sessions\n2026-01-02,open,25:00-26:00\n",
            Some("a_share"),
            None,
        )
        .is_err());
    }

    #[test]
    fn annual_csv_import_preserves_cross_midnight_sessions() {
        let calendar = TradingCalendar::from_csv(
            "date,status,sessions\n2026-01-05,open,21:00-01:00\n",
            Some("china_futures"),
            Some("Asia/Shanghai"),
        )
        .unwrap();
        let after_midnight =
            TimeZoneSpec::AsiaShanghai.timestamp_from_local(days_from_civil(2026, 1, 6), 30 * 60);
        let session = calendar
            .session_for_timestamp_local(after_midnight)
            .unwrap()
            .expect("overnight session should include next local day");
        assert_eq!(session.session_day, days_from_civil(2026, 1, 5));
        assert!(session.close_timestamp > session.open_timestamp);
    }

    #[test]
    fn versioned_definition_applies_metadata_and_overrides() {
        let definition = CalendarDefinition {
            market: Some("a_share".to_string()),
            timezone: Some("Asia/Shanghai".to_string()),
            weekend_mask: None,
            holidays: vec!["2026-01-01".to_string()],
            sessions: None,
            special_sessions: vec![CalendarSessionOverride::new(
                "2026-01-02",
                vec![SessionWindow::new(9 * 3600 + 30 * 60, 12 * 3600).unwrap()],
            )],
            source: Some("sse-official".to_string()),
            revision: Some("2026.1".to_string()),
        };
        let calendar = TradingCalendar::from_definition(&definition).unwrap();
        assert_eq!(calendar.source(), Some("sse-official"));
        assert_eq!(calendar.revision(), Some("2026.1"));
        assert!(!calendar.is_trading_day("2026-01-01").unwrap());
        assert_eq!(
            calendar
                .special_sessions("2026-01-02")
                .unwrap()
                .unwrap()
                .len(),
            1
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn versioned_definition_roundtrips_from_json() {
        let calendar = TradingCalendar::from_json(
            r#"{
                "market":"us_equity",
                "timezone":"America/New_York",
                "holidays":["2026-07-03"],
                "source":"nyse-official",
                "revision":"2026.1"
            }"#,
        )
        .unwrap();
        assert_eq!(calendar.source(), Some("nyse-official"));
        assert!(!calendar.is_trading_day("2026-07-03").unwrap());
    }
}
