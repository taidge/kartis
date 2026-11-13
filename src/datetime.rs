//! Bounded, explicit-locale formatting of epoch milliseconds.
//!
//! Directive behavior follows D3 time-format (local E:/Repos/d3/docs/d3-time-format.md
//! and <https://github.com/d3/d3-time-format/blob/main/src/locale.js>). Calendar
//! fields use the proleptic Gregorian calendar and Euclidean day arithmetic.
//! There is no host timezone, leap second or locale global state. Parsing is
//! strict and bounded, with explicit gap/fold choices (reject by default).
//! The opt-in `timezone` feature adds named IANA zones and DST using a static
//! TZif database, including POSIX tail rules for future DST. The default build
//! has no time/date dependencies. No global Jiff database lookup is used.
//! `%Y`/`%G` retain extended signed years instead of D3's four-digit truncation.
use crate::scene::ensure;
use crate::time::{MAX_TIMESTAMP_MS, civil_from_days, days_from_civil};
use crate::{ChartError, MAX_LABEL_BYTES, Result};

const DAY: i64 = 86_400_000;
pub const MAX_TIME_FORMAT_BYTES: usize = 1024;
pub const MAX_TIME_FORMAT_TOKENS: usize = 512;
pub const MAX_TIME_TEMPLATE_DEPTH: usize = 8;
const MAX_LOCALE_BYTES: usize = 16_384;

mod locales;
pub use locales::{
    SUPPORTED_TIME_LOCALES, TIME_LOCALE_UPSTREAM_COMMIT, TIME_LOCALE_UPSTREAM_VERSION,
};
mod parse;
pub use parse::{DateTimeParseOptions, ParseFold, ParseGap};

/// An explicit wall-clock zone. Fixed offsets are minutes east of UTC, limited
/// to ±23:59. They never apply daylight-saving transitions or query the host.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum TimeZone {
    #[default]
    Utc,
    FixedOffset {
        minutes: i16,
    },
    /// Compiled IANA zone, including historical UTC offsets and DST rules.
    #[cfg(feature = "timezone")]
    Named(jiff::tz::TimeZone),
}
impl TimeZone {
    pub fn fixed_offset(minutes: i16) -> Result<Self> {
        let zone = Self::FixedOffset { minutes };
        zone.validate()?;
        Ok(zone)
    }
    pub fn validate(&self) -> Result<()> {
        if let Self::FixedOffset { minutes } = self {
            ensure(
                (-1439..=1439).contains(minutes),
                "time offset must be within ±23:59",
            )?;
        }
        Ok(())
    }
    /// Parse an exact IANA database name, without consulting the host timezone.
    #[cfg(feature = "timezone")]
    pub fn iana(name: &str) -> Result<Self> {
        ensure(
            !name.is_empty() && name.len() <= 128,
            "invalid IANA time zone name",
        )?;
        let (canonical, data) =
            jiff_tzdb::get(name).ok_or_else(|| ChartError("unknown IANA time zone".into()))?;
        ensure(
            canonical == name,
            "IANA time zone name must use canonical case",
        )?;
        jiff::tz::TimeZone::tzif(canonical, data)
            .map(Self::Named)
            .map_err(|error| ChartError(format!("invalid bundled IANA zone: {error}")))
    }
    /// Exact offset seconds east of UTC at this instant. Historical IANA zones
    /// can have sub-minute offsets. Named zones use Jiff's supported Timestamp
    /// range (approximately years -9999..=9999) and return Err outside it.
    /// POSIX tail rules continue future DST within that range. UTC/fixed offsets
    /// support the complete ECMAScript Date bound.
    pub fn offset_seconds_at(&self, timestamp_ms: i64) -> Result<i32> {
        self.validate()?;
        ensure(
            (-MAX_TIMESTAMP_MS..=MAX_TIMESTAMP_MS).contains(&timestamp_ms),
            "date-time timestamp out of range",
        )?;
        match self {
            Self::Utc => Ok(0),
            Self::FixedOffset { minutes } => Ok(i32::from(*minutes) * 60),
            #[cfg(feature = "timezone")]
            Self::Named(zone) => {
                // Direct construction also admits fractional milliseconds in
                // Jiff's final supported second.
                let timestamp = jiff::Timestamp::new(
                    timestamp_ms / 1000,
                    (timestamp_ms % 1000) as i32 * 1_000_000,
                )
                .map_err(|_| ChartError("IANA timestamp outside Jiff Timestamp range".into()))?;
                Ok(zone.to_offset(timestamp).seconds())
            }
        }
    }
}

/// Version of the opt-in compiled IANA database; never fetched at runtime.
#[cfg(feature = "timezone")]
pub fn iana_time_database_version() -> Option<&'static str> {
    jiff_tzdb::VERSION
}

/// Inclusive integral-millisecond bounds for named zone formatting. UTC and
/// explicit fixed offsets instead use ±MAX_TIMESTAMP_MS.
#[cfg(feature = "timezone")]
pub fn iana_timestamp_bounds() -> [i64; 2] {
    [
        jiff::Timestamp::MIN.as_millisecond(),
        jiff::Timestamp::MAX.as_millisecond(),
    ]
}

/// Locale names start at Sunday and January, respectively. Templates may use
/// `%c`, `%x`, `%X`; cycles, excessive expansion and unknown directives reject.
/// Fields are explicit strings; names are never interpreted as format strings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimeLocale {
    pub date_time: String,
    pub date: String,
    pub time: String,
    pub periods: [String; 2],
    pub days: [String; 7],
    pub short_days: [String; 7],
    pub months: [String; 12],
    pub short_months: [String; 12],
}
impl Default for TimeLocale {
    fn default() -> Self {
        Self::en_us()
    }
}
impl TimeLocale {
    pub fn en_us() -> Self {
        Self {
            date_time: "%x, %X".into(),
            date: "%-m/%-d/%Y".into(),
            time: "%-I:%M:%S %p".into(),
            periods: ["AM", "PM"].map(str::to_string),
            days: [
                "Sunday",
                "Monday",
                "Tuesday",
                "Wednesday",
                "Thursday",
                "Friday",
                "Saturday",
            ]
            .map(str::to_string),
            short_days: ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"].map(str::to_string),
            months: [
                "January",
                "February",
                "March",
                "April",
                "May",
                "June",
                "July",
                "August",
                "September",
                "October",
                "November",
                "December",
            ]
            .map(str::to_string),
            short_months: [
                "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
            ]
            .map(str::to_string),
        }
    }
    pub fn zh_cn() -> Self {
        Self {
            date_time: "%x %A %X".into(),
            date: "%Y年%-m月%-d日".into(),
            time: "%H:%M:%S".into(),
            periods: ["上午", "下午"].map(str::to_string),
            days: [
                "星期日",
                "星期一",
                "星期二",
                "星期三",
                "星期四",
                "星期五",
                "星期六",
            ]
            .map(str::to_string),
            short_days: ["周日", "周一", "周二", "周三", "周四", "周五", "周六"]
                .map(str::to_string),
            months: [
                "一月",
                "二月",
                "三月",
                "四月",
                "五月",
                "六月",
                "七月",
                "八月",
                "九月",
                "十月",
                "十一月",
                "十二月",
            ]
            .map(str::to_string),
            short_months: [
                "1月", "2月", "3月", "4月", "5月", "6月", "7月", "8月", "9月", "10月", "11月",
                "12月",
            ]
            .map(str::to_string),
        }
    }
    pub fn validate(&self) -> Result<()> {
        let templates = [&self.date_time, &self.date, &self.time];
        ensure(
            templates.iter().all(|t| t.len() <= MAX_TIME_FORMAT_BYTES),
            "time locale template byte budget exceeded",
        )?;
        // Official fr-CA intentionally has empty periods and uses a 24-hour
        // clock. Preserve that data; parsing an ambiguous 12-hour-only field
        // still rejects instead of silently choosing AM.
        ensure(
            self.periods.iter().all(|name| name.len() <= 256),
            "invalid time locale period",
        )?;
        let names = self
            .days
            .iter()
            .chain(&self.short_days)
            .chain(&self.months)
            .chain(&self.short_months);
        let mut bytes = templates.iter().map(|s| s.len()).sum::<usize>()
            + self.periods.iter().map(String::len).sum::<usize>();
        for name in names {
            ensure(
                !name.is_empty() && name.len() <= 256,
                "invalid time locale name",
            )?;
            bytes += name.len();
        }
        ensure(
            bytes <= MAX_LOCALE_BYTES,
            "time locale byte budget exceeded",
        )?;
        // Validate all templates, including currently unreferenced branches.
        for template in templates {
            compile(template, self)?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Padding {
    Default,
    Zero,
    Space,
    None,
}
#[derive(Clone, Debug, PartialEq, Eq)]
enum Token {
    Literal(String),
    Directive { code: char, padding: Padding },
}

/// A compiled, immutable formatter. Constructor work and output are bounded;
/// `format` accepts integral epoch milliseconds within the ECMAScript Date bound.
/// A fixed offset may put local civil fields up to one day beyond that bound.
/// `%Z` emits ±HHMM (UTC is +0000); historical offset seconds are truncated
/// toward zero in this textual field, while civil fields retain the exact offset.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DateTimeFormat {
    specifier: String,
    locale: TimeLocale,
    zone: TimeZone,
    tokens: Vec<Token>,
}
impl DateTimeFormat {
    pub fn new(specifier: &str, locale: &TimeLocale, zone: TimeZone) -> Result<Self> {
        zone.validate()?;
        locale.validate()?;
        let tokens = compile(specifier, locale)?;
        Ok(Self {
            specifier: specifier.into(),
            locale: locale.clone(),
            zone,
            tokens,
        })
    }
    pub fn utc(specifier: &str, locale: &TimeLocale) -> Result<Self> {
        Self::new(specifier, locale, TimeZone::Utc)
    }
    pub fn specifier(&self) -> &str {
        &self.specifier
    }
    pub fn locale(&self) -> &TimeLocale {
        &self.locale
    }
    pub fn zone(&self) -> TimeZone {
        self.zone.clone()
    }
    pub fn format(&self, timestamp_ms: i64) -> Result<String> {
        ensure(
            (-MAX_TIMESTAMP_MS..=MAX_TIMESTAMP_MS).contains(&timestamp_ms),
            "date-time timestamp out of range",
        )?;
        let offset_seconds = self.zone.offset_seconds_at(timestamp_ms)?;
        let shifted = timestamp_ms + i64::from(offset_seconds) * 1000;
        let fields = Fields::new(shifted);
        let mut out = String::new();
        for token in &self.tokens {
            let piece = match token {
                Token::Literal(literal) => literal.clone(),
                Token::Directive { code, padding } => {
                    self.directive(*code, *padding, timestamp_ms, offset_seconds, &fields)
                }
            };
            ensure(
                out.len() + piece.len() <= MAX_LABEL_BYTES,
                "date-time output byte budget exceeded",
            )?;
            out.push_str(&piece);
        }
        Ok(out)
    }
    fn directive(
        &self,
        code: char,
        padding: Padding,
        instant: i64,
        offset_seconds: i32,
        f: &Fields,
    ) -> String {
        let numeric = |value, width| pad(value, width, padding, code == 'e');
        match code {
            'a' => self.locale.short_days[f.weekday as usize].clone(),
            'A' => self.locale.days[f.weekday as usize].clone(),
            'b' => self.locale.short_months[(f.month - 1) as usize].clone(),
            'B' => self.locale.months[(f.month - 1) as usize].clone(),
            'd' | 'e' => numeric(f.day, 2),
            // D3 pads milliseconds, then appends three zeros: %-f for zero is 0000.
            'f' => format!("{}000", numeric(f.millisecond, 3)),
            'g' => numeric(f.iso_year % 100, 2),
            'G' => year(f.iso_year, padding),
            'H' => numeric(f.hour, 2),
            'I' => numeric(if f.hour % 12 == 0 { 12 } else { f.hour % 12 }, 2),
            'j' => numeric(f.ordinal, 3),
            'L' => numeric(f.millisecond, 3),
            'm' => numeric(f.month, 2),
            'M' => numeric(f.minute, 2),
            'p' => self.locale.periods[usize::from(f.hour >= 12)].clone(),
            'q' => ((f.month - 1) / 3 + 1).to_string(),
            'Q' => instant.to_string(),
            's' => instant.div_euclid(1000).to_string(),
            'S' => numeric(f.second, 2),
            'u' => (f.weekday_monday + 1).to_string(),
            'U' => numeric((f.ordinal - 1 + 7 - f.weekday) / 7, 2),
            'V' => numeric(f.iso_week, 2),
            'w' => f.weekday.to_string(),
            'W' => numeric((f.ordinal - 1 + 7 - f.weekday_monday) / 7, 2),
            'y' => numeric(f.year % 100, 2),
            'Y' => year(f.year, padding),
            'Z' => {
                let minutes = offset_seconds.abs() / 60;
                format!(
                    "{}{:02}{:02}",
                    if offset_seconds < 0 { '-' } else { '+' },
                    minutes / 60,
                    minutes % 60
                )
            }
            '%' => "%".into(),
            _ => unreachable!("constructor validates and expands all directives"),
        }
    }
}
struct Fields {
    year: i64,
    month: i64,
    day: i64,
    hour: i64,
    minute: i64,
    second: i64,
    millisecond: i64,
    weekday: i64,
    weekday_monday: i64,
    ordinal: i64,
    iso_year: i64,
    iso_week: i64,
}
impl Fields {
    fn new(timestamp_ms: i64) -> Self {
        let days = timestamp_ms.div_euclid(DAY);
        let clock = timestamp_ms.rem_euclid(DAY);
        let (year, month, day) = civil_from_days(days);
        let weekday = (days + 4).rem_euclid(7);
        let weekday_monday = (weekday + 6) % 7;
        let thursday = days + 3 - weekday_monday;
        let iso_year = civil_from_days(thursday).0;
        let iso_week = (thursday - days_from_civil(iso_year, 1, 1)).div_euclid(7) + 1;
        Self {
            year,
            month,
            day,
            hour: clock / 3_600_000,
            minute: clock / 60_000 % 60,
            second: clock / 1000 % 60,
            millisecond: clock % 1000,
            weekday,
            weekday_monday,
            ordinal: days - days_from_civil(year, 1, 1) + 1,
            iso_year,
            iso_week,
        }
    }
}
fn pad(value: i64, width: usize, padding: Padding, space_default: bool) -> String {
    let number = value.unsigned_abs().to_string();
    let fill = match padding {
        Padding::None => "",
        Padding::Space => " ",
        Padding::Zero => "0",
        Padding::Default if space_default => " ",
        Padding::Default => "0",
    };
    format!(
        "{}{}{}",
        if value < 0 { "-" } else { "" },
        fill.repeat(width.saturating_sub(number.len())),
        number
    )
}
fn year(value: i64, padding: Padding) -> String {
    if value > 9999 {
        format!("+{}", pad(value, 6, padding, false))
    } else {
        pad(value, 4, padding, false)
    }
}
fn compile(specifier: &str, locale: &TimeLocale) -> Result<Vec<Token>> {
    let mut tokens = vec![];
    let mut work = 0;
    expand(specifier, locale, &mut vec![], &mut work, &mut tokens)?;
    Ok(tokens)
}
fn expand(
    specifier: &str,
    locale: &TimeLocale,
    stack: &mut Vec<char>,
    work: &mut usize,
    tokens: &mut Vec<Token>,
) -> Result<()> {
    ensure(
        specifier.len() <= MAX_TIME_FORMAT_BYTES,
        "time format specifier byte budget exceeded",
    )?;
    *work += specifier.len();
    ensure(
        *work <= 8192 && stack.len() <= MAX_TIME_TEMPLATE_DEPTH,
        "time template expansion budget exceeded",
    )?;
    let mut chars = specifier.chars().peekable();
    let mut literal = String::new();
    while let Some(ch) = chars.next() {
        if ch != '%' {
            literal.push(ch);
            continue;
        }
        if !literal.is_empty() {
            tokens.push(Token::Literal(std::mem::take(&mut literal)));
        }
        let next = chars
            .next()
            .ok_or_else(|| ChartError("incomplete time directive".into()))?;
        let (padding, code) = match next {
            '0' | '_' | '-' => (
                match next {
                    '0' => Padding::Zero,
                    '_' => Padding::Space,
                    _ => Padding::None,
                },
                chars
                    .next()
                    .ok_or_else(|| ChartError("incomplete time padding directive".into()))?,
            ),
            code => (Padding::Default, code),
        };
        if matches!(code, 'c' | 'x' | 'X') {
            ensure(!stack.contains(&code), "cyclic time locale template")?;
            let template = match code {
                'c' => &locale.date_time,
                'x' => &locale.date,
                _ => &locale.time,
            };
            stack.push(code);
            expand(template, locale, stack, work, tokens)?;
            stack.pop();
        } else {
            ensure(
                "aAbBdefgGHIjLmMpqQsSuUVwWyYZ%".contains(code),
                "unknown time format directive",
            )?;
            tokens.push(Token::Directive { code, padding });
        }
        ensure(
            tokens.len() <= MAX_TIME_FORMAT_TOKENS,
            "time format token budget exceeded",
        )?;
    }
    if !literal.is_empty() {
        tokens.push(Token::Literal(literal));
    }
    ensure(
        tokens.len() <= MAX_TIME_FORMAT_TOKENS,
        "time format token budget exceeded",
    )
}
