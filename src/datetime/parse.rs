use std::collections::BTreeMap;

use super::*;
use crate::zoned_time::{FoldPolicy, GapPolicy, TimeCalendar, resolve_local};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ParseGap {
    #[default]
    Reject,
    /// Resolve a nonexistent local time to the transition's first valid instant,
    /// e.g. NY 02:30 -> 03:00. This does not preserve minutes across the gap.
    NextValid,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ParseFold {
    #[default]
    Reject,
    Earlier,
    Later,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DateTimeParseOptions {
    pub gap: ParseGap,
    pub fold: ParseFold,
}

impl DateTimeFormat {
    /// Strictly consume the complete input using this formatter's compiled
    /// locale tokens. Defaults: 1900-01-01 00:00:00.000, two-digit years use
    /// D3's 00..68 -> 2000..2068, 69..99 -> 1969..1999. Duplicates must agree;
    /// invalid dates, inconsistent weekday/week/epoch fields and DST ambiguity
    /// reject. %Z supplies an explicit offset and overrides the configured zone.
    /// %f requires millisecond-exact microseconds (last three digits zero).
    /// Names match Unicode lowercase, literals exactly. No trailing whitespace.
    pub fn parse(&self, text: &str) -> Result<i64> {
        self.parse_with(text, DateTimeParseOptions::default())
    }
    pub fn parse_with(&self, text: &str, options: DateTimeParseOptions) -> Result<i64> {
        ensure(
            text.len() <= MAX_LABEL_BYTES,
            "date-time parse input byte budget exceeded",
        )?;
        let mut cursor = Cursor { text, index: 0 };
        let mut fields = BTreeMap::new();
        for (index, token) in self.tokens.iter().enumerate() {
            match token {
                Token::Literal(literal) => cursor.literal(literal)?,
                Token::Directive { code, padding } => {
                    let (key, value) = match code {
                        '%' => {
                            cursor.literal("%")?;
                            continue;
                        }
                        'a' => ('w', cursor.name(&self.locale.short_days)? as i64),
                        'A' => ('w', cursor.name(&self.locale.days)? as i64),
                        'b' => ('m', cursor.name(&self.locale.short_months)? as i64 + 1),
                        'B' => ('m', cursor.name(&self.locale.months)? as i64 + 1),
                        'p' => {
                            if self.locale.periods[0] == self.locale.periods[1] {
                                cursor.literal(&self.locale.periods[0])?;
                                ('P', 0)
                            } else {
                                ('p', cursor.name(&self.locale.periods)? as i64)
                            }
                        }
                        'Z' => ('Z', cursor.offset()?),
                        code => {
                            let adjacent=self.tokens.get(index+1).is_some_and(|t|matches!(t,Token::Directive{code,..} if "defgGHIjLmMqQsSuUVwWyY".contains(*code)));
                            let width = match code {
                                'Y' | 'G' => {
                                    if adjacent {
                                        4
                                    } else {
                                        6
                                    }
                                }
                                'Q' | 's' => 17,
                                'j' => 3,
                                'L' => 3,
                                'f' => 6,
                                'q' | 'u' | 'w' => 1,
                                _ => 2,
                            };
                            let signed = matches!(code, 'Y' | 'G' | 'y' | 'g' | 'Q' | 's');
                            let mut value = cursor.number(
                                width,
                                signed,
                                *padding == Padding::Space || *code == 'e',
                            )?;
                            let key = match code {
                                'e' => 'd',
                                'u' => {
                                    ensure((1..=7).contains(&value), "invalid ISO weekday")?;
                                    value %= 7;
                                    'w'
                                }
                                'y' | 'g' => {
                                    ensure((-99..=99).contains(&value), "invalid two-digit year")?;
                                    *code
                                }
                                'f' => {
                                    ensure(
                                        (0..=999999).contains(&value) && value % 1000 == 0,
                                        "submillisecond precision is unsupported",
                                    )?;
                                    value /= 1000;
                                    'L'
                                }
                                _ => *code,
                            };
                            (key, value)
                        }
                    };
                    if let Some(previous) = fields.insert(key, value) {
                        ensure(previous == value, "conflicting date-time fields")?;
                    }
                }
            }
        }
        ensure(cursor.index == text.len(), "unconsumed date-time input")?;
        resolve(self, &fields, options)
    }
}
struct Cursor<'a> {
    text: &'a str,
    index: usize,
}
impl Cursor<'_> {
    fn literal(&mut self, literal: &str) -> Result<()> {
        ensure(
            self.text[self.index..].starts_with(literal),
            "date-time literal mismatch",
        )?;
        self.index += literal.len();
        Ok(())
    }
    fn number(&mut self, width: usize, signed: bool, spaces: bool) -> Result<i64> {
        if spaces {
            while self.text.as_bytes().get(self.index) == Some(&b' ') {
                self.index += 1;
            }
        }
        let start = self.index;
        if signed
            && self
                .text
                .as_bytes()
                .get(self.index)
                .is_some_and(|b| matches!(b, b'+' | b'-'))
        {
            self.index += 1;
        }
        let digits = self.index;
        while self.index - digits < width
            && self
                .text
                .as_bytes()
                .get(self.index)
                .is_some_and(u8::is_ascii_digit)
        {
            self.index += 1;
        }
        ensure(self.index > digits, "missing date-time numeric field")?;
        self.text[start..self.index]
            .parse()
            .map_err(|_| ChartError("date-time number overflow".into()))
    }
    fn name(&mut self, names: &[String]) -> Result<usize> {
        // Longest matching name prevents e.g. French 'mars'/'mar' prefix
        // consumption. Equal matching names with different values reject.
        let rest = &self.text[self.index..];
        let mut matches = Vec::new();
        for (index, name) in names.iter().enumerate() {
            let count = name.chars().count();
            let end = rest
                .char_indices()
                .nth(count)
                .map_or(rest.len(), |(i, _)| i);
            let candidate = &rest[..end];
            if candidate.to_lowercase() == name.to_lowercase() {
                matches.push((end, index));
            }
        }
        matches.sort_unstable();
        let (length, index) = matches
            .pop()
            .ok_or_else(|| ChartError("unknown date-time locale name".into()))?;
        ensure(
            matches.last().is_none_or(|(len, _)| *len != length),
            "ambiguous date-time locale name",
        )?;
        self.index += length;
        Ok(index)
    }
    fn offset(&mut self) -> Result<i64> {
        if self.text[self.index..].starts_with('Z') {
            self.index += 1;
            return Ok(0);
        }
        let sign = match self.text.as_bytes().get(self.index) {
            Some(b'+') => 1,
            Some(b'-') => -1,
            _ => return Err(ChartError("missing time zone offset".into())),
        };
        self.index += 1;
        let hour_start = self.index;
        let hour = self.number(2, false, false)?;
        ensure(
            self.index - hour_start == 2,
            "zone offset hours need two digits",
        )?;
        let colon = self.text.as_bytes().get(self.index) == Some(&b':');
        if colon {
            self.index += 1;
        }
        let minute = if self
            .text
            .as_bytes()
            .get(self.index)
            .is_some_and(u8::is_ascii_digit)
        {
            let start = self.index;
            let value = self.number(2, false, false)?;
            ensure(
                self.index - start == 2,
                "zone offset minutes need two digits",
            )?;
            value
        } else {
            ensure(!colon, "missing zone offset minutes")?;
            0
        };
        ensure(hour < 24 && minute < 60, "invalid time zone offset")?;
        Ok(sign * (hour * 3600 + minute * 60))
    }
}
fn resolve(
    format: &DateTimeFormat,
    p: &BTreeMap<char, i64>,
    options: DateTimeParseOptions,
) -> Result<i64> {
    let get = |key, default| p.get(&key).copied().unwrap_or(default);
    for (key, lo, hi) in [
        ('Y', -300000, 300000),
        ('G', -300000, 300000),
        ('m', 1, 12),
        ('d', 1, 31),
        ('H', 0, 23),
        ('I', 1, 12),
        ('M', 0, 59),
        ('S', 0, 59),
        ('L', 0, 999),
        ('j', 1, 366),
        ('w', 0, 6),
        ('V', 1, 53),
        ('U', 0, 53),
        ('W', 0, 53),
        ('q', 1, 4),
        ('p', 0, 1),
    ] {
        ensure(
            p.get(&key).is_none_or(|v| (lo..=hi).contains(v)),
            "date-time field out of range",
        )?;
    }
    ensure(
        !p.contains_key(&'p') && !p.contains_key(&'P')
            || ['I', 'H', 'Q', 's'].iter().any(|key| p.contains_key(key)),
        "period needs an hour field",
    )?;
    ensure(
        !p.contains_key(&'P') || ['H', 'Q', 's'].iter().any(|key| p.contains_key(key)),
        "identical locale periods require a 24-hour or epoch field",
    )?;
    let explicit_zone = p.get(&'Z').copied();
    let instant = if let Some(q) = p.get(&'Q') {
        Some(*q)
    } else if let Some(s) = p.get(&'s') {
        Some(
            s.checked_mul(1000)
                .and_then(|v| v.checked_add(get('L', 0)))
                .ok_or_else(|| ChartError("parsed epoch overflow".into()))?,
        )
    } else {
        None
    };
    if let Some(instant) = instant {
        let offset = if let Some(offset) = explicit_zone {
            offset
        } else {
            i64::from(format.zone.offset_seconds_at(instant)?)
        };
        let local = instant
            .checked_add(offset * 1000)
            .ok_or_else(|| ChartError("parsed local overflow".into()))?;
        verify(p, &Fields::new(local), instant)?;
        ensure(
            (-MAX_TIMESTAMP_MS..=MAX_TIMESTAMP_MS).contains(&instant),
            "parsed timestamp out of range",
        )?;
        return Ok(instant);
    }
    let short_year = |key| {
        p.get(&key).map(|v| {
            if *v < 0 {
                *v
            } else {
                *v + if *v > 68 { 1900 } else { 2000 }
            }
        })
    };
    let year = get('Y', short_year('y').unwrap_or(1900));
    let month = get('m', (get('q', 1) - 1) * 3 + 1);
    let day = get('d', 1);
    let calendar_days = days_from_civil(year, month, day);
    ensure(
        civil_from_days(calendar_days) == (year, month, day),
        "invalid Gregorian date",
    )?;
    let iso_default = (p.contains_key(&'G') || p.contains_key(&'g'))
        && !['Y', 'y', 'm', 'd', 'j', 'U', 'W']
            .iter()
            .any(|key| p.contains_key(key));
    let days = if p.contains_key(&'V') || iso_default {
        let week = get('V', 1);
        let iso_year = get('G', short_year('g').unwrap_or(year));
        let jan4 = days_from_civil(iso_year, 1, 4);
        let monday = jan4 - (jan4 + 3).rem_euclid(7);
        monday + (week - 1) * 7 + (get('w', 1) + 6) % 7
    } else if let Some(ordinal) = p.get(&'j') {
        days_from_civil(year, 1, 1) + ordinal - 1
    } else if p.contains_key(&'U') || p.contains_key(&'W') {
        let monday = p.contains_key(&'W');
        let week = get(if monday { 'W' } else { 'U' }, 0);
        let jan1 = days_from_civil(year, 1, 1);
        let first = jan1 + (if monday { 1 } else { 0 } - (jan1 + 4).rem_euclid(7)).rem_euclid(7);
        first
            + (week - 1) * 7
            + if monday {
                (get('w', 1) + 6) % 7
            } else {
                get('w', 0)
            }
    } else {
        calendar_days
    };
    let hour = get(
        'H',
        if p.contains_key(&'I') {
            get('I', 12) % 12 + get('p', 0) * 12
        } else {
            0
        },
    );
    let local =
        days * DAY + hour * 3_600_000 + get('M', 0) * 60_000 + get('S', 0) * 1000 + get('L', 0);
    // Verify desired civil fields before applying an explicitly selected gap
    // normalization. This still rejects impossible dates/ISO week 53/conflicts.
    verify(p, &Fields::new(local), 0)?;
    let result = if let Some(offset) = explicit_zone {
        local - offset * 1000
    } else {
        let calendar = TimeCalendar {
            zone: format.zone.clone(),
            interval: None,
            gap: match options.gap {
                ParseGap::Reject => GapPolicy::Reject,
                ParseGap::NextValid => GapPolicy::NextValid,
            },
            fold: match options.fold {
                ParseFold::Reject => FoldPolicy::Reject,
                ParseFold::Earlier => FoldPolicy::Earlier,
                ParseFold::Later => FoldPolicy::Later,
            },
        };
        let candidates = resolve_local(local, &calendar)?;
        *candidates
            .first()
            .ok_or_else(|| ChartError("parsed local date outside supported zone range".into()))?
    };
    ensure(
        (-MAX_TIMESTAMP_MS..=MAX_TIMESTAMP_MS).contains(&result),
        "parsed timestamp out of range",
    )?;
    Ok(result)
}
fn verify(p: &BTreeMap<char, i64>, f: &Fields, instant: i64) -> Result<()> {
    for (key, value) in [
        ('Y', f.year),
        ('y', f.year % 100),
        ('m', f.month),
        ('d', f.day),
        ('H', f.hour),
        ('I', if f.hour % 12 == 0 { 12 } else { f.hour % 12 }),
        ('p', i64::from(f.hour >= 12)),
        ('M', f.minute),
        ('S', f.second),
        ('L', f.millisecond),
        ('j', f.ordinal),
        ('w', f.weekday),
        ('G', f.iso_year),
        ('g', f.iso_year % 100),
        ('V', f.iso_week),
        ('U', (f.ordinal - 1 + 7 - f.weekday) / 7),
        ('W', (f.ordinal - 1 + 7 - f.weekday_monday) / 7),
        ('q', (f.month - 1) / 3 + 1),
        ('Q', instant),
        ('s', instant.div_euclid(1000)),
    ] {
        ensure(
            p.get(&key).is_none_or(|v| *v == value),
            "conflicting parsed calendar/epoch fields",
        )?;
    }
    Ok(())
}
