//! Checked locale-aware D3 number and explicit string formatting.
//!
//! Grammar: `[[fill]align][sign][symbol][0][width][,][.precision][~][type]`.
//! Supports f/e/g/r/s/%/p/d/b/o/x/X/n, explicit string c, and fixed SI prefixes.
//! Decimal rounding uses the exact binary value, with JavaScript's ties upward
//! rule for positive magnitudes. Precision is bounded to 20 fixed / 21 significant
//! digits; width to 256, and output to the scene label budget. Nonfinite inputs,
//! implicit JavaScript coercion, and unknown-type fallback are rejected.
//! Behavior follows released d3-format 3.1.0, including its fixed-SI suffix
//! placement (the unreleased upstream main branch differs).
use crate::scene::ensure;
use crate::{ChartError, MAX_LABEL_BYTES, Result};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormatLocale {
    pub decimal: String,
    pub thousands: String,
    pub grouping: Vec<usize>,
    pub currency: (String, String),
    pub minus: String,
    pub percent: String,
    pub numerals: Option<[String; 10]>,
}
impl Default for FormatLocale {
    fn default() -> Self {
        Self {
            decimal: ".".into(),
            thousands: ",".into(),
            grouping: vec![3],
            currency: ("$".into(), String::new()),
            minus: "−".into(),
            percent: "%".into(),
            numerals: None,
        }
    }
}
impl FormatLocale {
    pub fn validate(&self) -> Result<()> {
        for text in [
            &self.decimal,
            &self.thousands,
            &self.currency.0,
            &self.currency.1,
            &self.minus,
            &self.percent,
        ] {
            ensure(
                text.len() <= 64 && !text.chars().any(char::is_control),
                "invalid locale token",
            )?;
        }
        ensure(
            !self.decimal.is_empty()
                && !self.minus.is_empty()
                && self.grouping.len() <= 8
                && self.grouping.iter().all(|n| (1..=9).contains(n)),
            "invalid locale decimal/minus/grouping",
        )?;
        if let Some(numerals) = &self.numerals {
            ensure(
                numerals
                    .iter()
                    .all(|s| !s.is_empty() && s.len() <= 16 && !s.chars().any(char::is_control)),
                "invalid locale numerals",
            )?;
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Align {
    Left,
    Right,
    Center,
    Sign,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Sign {
    Minus,
    Plus,
    Space,
    Parentheses,
}
#[derive(Clone, Debug, PartialEq)]
pub struct NumberFormat {
    source: String,
    locale: FormatLocale,
    fill: char,
    align: Align,
    sign: Sign,
    symbol: Option<char>,
    zero: bool,
    width: usize,
    grouping: bool,
    precision: usize,
    trim: bool,
    kind: char,
}
fn invalid() -> ChartError {
    ChartError("invalid or unsupported numeric format specifier".into())
}
fn alignment(c: char) -> Option<Align> {
    match c {
        '<' => Some(Align::Left),
        '>' => Some(Align::Right),
        '^' => Some(Align::Center),
        '=' => Some(Align::Sign),
        _ => None,
    }
}
impl NumberFormat {
    pub fn new(spec: &str, locale: &FormatLocale) -> Result<Self> {
        locale.validate()?;
        ensure(
            spec.len() <= 128 && !spec.chars().any(char::is_control),
            "format specifier byte budget exceeded or control character",
        )?;
        let chars = spec.chars().collect::<Vec<_>>();
        let mut i = 0;
        let mut fill = ' ';
        let mut align = Align::Right;
        if chars.len() >= 2
            && let Some(a) = alignment(chars[1])
        {
            fill = chars[0];
            align = a;
            i = 2;
        } else if let Some(a) = chars.first().and_then(|c| alignment(*c)) {
            align = a;
            i = 1;
        }
        let sign = match chars.get(i) {
            Some('+') => {
                i += 1;
                Sign::Plus
            }
            Some('(') => {
                i += 1;
                Sign::Parentheses
            }
            Some(' ') => {
                i += 1;
                Sign::Space
            }
            Some('-') => {
                i += 1;
                Sign::Minus
            }
            _ => Sign::Minus,
        };
        let symbol = if chars.get(i).is_some_and(|c| matches!(c, '$' | '#')) {
            let c = chars[i];
            i += 1;
            Some(c)
        } else {
            None
        };
        let mut zero = chars.get(i) == Some(&'0');
        if zero {
            i += 1;
            fill = '0';
            align = Align::Sign;
        }
        if fill == '0' && align == Align::Sign {
            zero = true;
        }
        let mut width = 0usize;
        while let Some(c) = chars.get(i).filter(|c| c.is_ascii_digit()) {
            width = width
                .checked_mul(10)
                .and_then(|v| v.checked_add(*c as usize - '0' as usize))
                .ok_or_else(invalid)?;
            ensure(width <= 256, "format width exceeds 256")?;
            i += 1;
        }
        let mut grouping = chars.get(i) == Some(&',');
        if grouping {
            i += 1;
        }
        let mut precision = None;
        if chars.get(i) == Some(&'.') {
            i += 1;
            let start = i;
            let mut p = 0usize;
            while let Some(c) = chars.get(i).filter(|c| c.is_ascii_digit()) {
                p = p
                    .checked_mul(10)
                    .and_then(|v| v.checked_add(*c as usize - '0' as usize))
                    .ok_or_else(invalid)?;
                ensure(p <= 21, "format precision exceeds 21")?;
                i += 1;
            }
            ensure(i > start, "format precision requires digits")?;
            precision = Some(p);
        }
        let mut trim = chars.get(i) == Some(&'~');
        if trim {
            i += 1;
        }
        let mut kind = chars.get(i).copied().unwrap_or('\0');
        if i < chars.len() {
            i += 1;
        }
        ensure(i == chars.len(), "trailing numeric format characters")?;
        if kind == 'n' {
            grouping = true;
            kind = 'g';
        }
        let default = kind == '\0';
        if default {
            kind = 'g';
            trim = true;
        }
        ensure(
            matches!(
                kind,
                'f' | 'e' | 'g' | 'r' | 's' | '%' | 'p' | 'd' | 'b' | 'o' | 'x' | 'X' | 'c'
            ),
            "unsupported numeric format type",
        )?;
        let mut precision = precision.unwrap_or(if default { 12 } else { 6 });
        if matches!(kind, 'g' | 'r' | 's' | 'p') {
            precision = precision.max(1);
        }
        ensure(
            matches!(kind, 'g' | 'r' | 's' | 'p') || precision <= 20,
            "fixed precision exceeds 20",
        )?;
        Ok(Self {
            source: spec.into(),
            locale: locale.clone(),
            fill,
            align,
            sign,
            symbol,
            zero,
            width,
            grouping,
            precision,
            trim,
            kind,
        })
    }
    pub fn specifier(&self) -> &str {
        &self.source
    }
    /// Whether this specifier requires an explicit string rather than a number.
    pub fn is_text(&self) -> bool {
        self.kind == 'c'
    }
    /// D3's `c` preserves the entire supplied string; it is not a character code.
    /// Numeric signs, precision and grouping do not modify the string.
    pub fn format_text(&self, value: &str) -> Result<String> {
        ensure(self.is_text(), "string input requires format type c")?;
        ensure(
            value.len() <= MAX_LABEL_BYTES,
            "string format input budget exceeded",
        )?;
        let (prefix, suffix) = if self.symbol == Some('$') {
            (&self.locale.currency.0[..], &self.locale.currency.1[..])
        } else {
            ("", "")
        };
        self.assemble(prefix, "", &format!("{value}{suffix}"))
    }
    pub fn format(&self, value: f64) -> Result<String> {
        ensure(!self.is_text(), "format type c requires an explicit string")?;
        ensure(
            value.is_finite(),
            "numeric formatter requires a finite value",
        )?;
        let mut negative = value.is_sign_negative();
        let mut value = value.abs();
        let mut suffix = String::new();
        let mut body = match self.kind {
            'f' => fixed(value, self.precision),
            '%' => {
                value *= 100.;
                ensure(value.is_finite(), "formatted percentage overflow")?;
                suffix = self.locale.percent.clone();
                fixed(value, self.precision)
            }
            'e' => scientific(value, self.precision),
            'g' => significant(value, self.precision, false),
            'r' => significant(value, self.precision, true),
            'p' => {
                value *= 100.;
                ensure(value.is_finite(), "formatted percentage overflow")?;
                suffix = self.locale.percent.clone();
                significant(value, self.precision, true)
            }
            's' => {
                let (coefficient, exponent) = decimal_parts(value, self.precision);
                let prefix = (exponent.div_euclid(3) * 3).clamp(-24, 24);
                suffix = prefix_symbol(prefix).into();
                if exponent - prefix < 0 {
                    let digits = (self.precision as i32 + exponent - prefix).max(0) as usize;
                    let (coefficient, _) = if digits == 0 {
                        shortest_parts(value)
                    } else {
                        decimal_parts(value, digits)
                    };
                    format!(
                        "0.{}{}",
                        "0".repeat((prefix - exponent - 1) as usize),
                        coefficient.replace('.', "")
                    )
                } else {
                    expand(&coefficient, exponent - prefix)
                }
            }
            'd' | 'b' | 'o' | 'x' | 'X' => {
                let rounded = value.round();
                match self.kind {
                    'b' => radix(rounded, 2, false),
                    'o' => radix(rounded, 8, false),
                    'x' => radix(rounded, 16, false),
                    'X' => radix(rounded, 16, true),
                    _ => {
                        let (coefficient, e) = shortest_parts(rounded);
                        expand(&coefficient, e)
                    }
                }
            }
            _ => unreachable!(),
        };
        if self.trim {
            body = trim(&body);
        }
        // Hide the minus sign on a rounded zero unless '+' explicitly requests it.
        if negative && self.sign != Sign::Plus && body.parse::<f64>().is_ok_and(|n| n == 0.) {
            negative = false;
        }
        let mut prefix = if negative {
            if self.sign == Sign::Parentheses {
                "(".into()
            } else {
                self.locale.minus.clone()
            }
        } else {
            match self.sign {
                Sign::Plus => "+".into(),
                Sign::Space => " ".into(),
                _ => String::new(),
            }
        };
        if self.symbol == Some('$') {
            prefix.push_str(&self.locale.currency.0);
            // D3 currency takes precedence over the percentage suffix.
            if matches!(self.kind, '%' | 'p') {
                suffix.clear();
            }
            suffix.push_str(&self.locale.currency.1);
        } else if self.symbol == Some('#') && matches!(self.kind, 'b' | 'o' | 'x' | 'X') {
            prefix.push_str(match self.kind {
                'b' => "0b",
                'o' => "0o",
                _ => "0x",
            });
        }
        if negative && self.sign == Sign::Parentheses {
            suffix.push(')');
        }
        let split = if matches!(self.kind, 'd' | 'e' | 'f' | 'g' | 'p' | 'r' | 's' | '%') {
            body.find(['.', 'e']).unwrap_or(body.len())
        } else {
            body.len()
        };
        let integer = body[..split].to_owned();
        let tail = &body[split..];
        let tail = if let Some(fraction) = tail.strip_prefix('.') {
            format!("{}{fraction}", self.locale.decimal)
        } else {
            tail.into()
        };
        suffix = format!("{tail}{suffix}");
        self.assemble(&prefix, &integer, &suffix)
    }
    fn assemble(&self, prefix: &str, integer: &str, suffix: &str) -> Result<String> {
        let mut integer = integer.to_owned();
        if self.grouping && !self.zero && !self.is_text() {
            integer = group(&integer, &self.locale, usize::MAX);
        }
        let length = utf16_len(prefix) + utf16_len(&integer) + utf16_len(suffix);
        let padding = self.width.saturating_sub(length);
        let padding = std::iter::repeat_n(self.fill, padding).collect::<String>();
        let output = if self.zero && self.grouping {
            let digits = format!("{padding}{integer}");
            let width = if padding.is_empty() {
                usize::MAX
            } else {
                self.width.saturating_sub(utf16_len(suffix))
            };
            format!("{prefix}{}{suffix}", group(&digits, &self.locale, width))
        } else {
            match self.align {
                Align::Left => format!("{prefix}{integer}{suffix}{padding}"),
                Align::Right => format!("{padding}{prefix}{integer}{suffix}"),
                Align::Sign => format!("{prefix}{padding}{integer}{suffix}"),
                Align::Center => {
                    let count = padding.chars().count();
                    let before = std::iter::repeat_n(self.fill, count / 2).collect::<String>();
                    let after =
                        std::iter::repeat_n(self.fill, count - count / 2).collect::<String>();
                    format!("{before}{prefix}{integer}{suffix}{after}")
                }
            }
        };
        let output = if let Some(numerals) = &self.locale.numerals {
            let mut translated = String::new();
            for c in output.chars() {
                if c.is_ascii_digit() {
                    translated.push_str(&numerals[c as usize - '0' as usize]);
                } else {
                    translated.push(c);
                }
            }
            translated
        } else {
            output
        };
        ensure(
            output.len() <= MAX_LABEL_BYTES,
            "formatted numeric label budget exceeded",
        )?;
        Ok(output)
    }
}
pub fn format_number(spec: &str, value: f64, locale: &FormatLocale) -> Result<String> {
    NumberFormat::new(spec, locale)?.format(value)
}
fn utf16_len(s: &str) -> usize {
    s.encode_utf16().count()
}
const PREFIXES: [&str; 17] = [
    "y", "z", "a", "f", "p", "n", "µ", "m", "", "k", "M", "G", "T", "P", "E", "Z", "Y",
];
// Use the correctly rounded decimal constants, avoiding accumulated rounding
// from integer-power evaluation (observable at 20 fractional digits).
const PREFIX_SCALE: [f64; 17] = [
    1e24, 1e21, 1e18, 1e15, 1e12, 1e9, 1e6, 1e3, 1., 1e-3, 1e-6, 1e-9, 1e-12, 1e-15, 1e-18, 1e-21,
    1e-24,
];
fn prefix_symbol(exponent: i32) -> &'static str {
    PREFIXES[((exponent + 24) / 3) as usize]
}

/// A fixed SI unit chosen once from a finite reference value (zero uses no unit).
/// Unlike `s`, precision counts fractional digits and the unit never changes.
/// As in released d3-format 3.1, the unit is appended after padding/alignment.
#[derive(Clone, Debug, PartialEq)]
pub struct PrefixFormat {
    format: NumberFormat,
    exponent: i32,
}
impl PrefixFormat {
    pub fn new(spec: &str, reference: f64, locale: &FormatLocale) -> Result<Self> {
        ensure(reference.is_finite(), "SI prefix reference must be finite")?;
        // Parse the original grammar, then replace its type before precision
        // defaults/clamping: `.0s` is zero fractional digits in formatPrefix.
        NumberFormat::new(spec, locale)?;
        let mut fixed_spec = spec.to_owned();
        if fixed_spec.ends_with(|c: char| c.is_ascii_alphabetic() || c == '%') {
            fixed_spec.pop();
        }
        fixed_spec.push('f');
        let format = NumberFormat::new(&fixed_spec, locale)?;
        let (_, e) = shortest_parts(reference.abs());
        Ok(Self {
            format,
            exponent: (e.div_euclid(3) * 3).clamp(-24, 24),
        })
    }
    pub fn exponent(&self) -> i32 {
        self.exponent
    }
    pub fn format(&self, value: f64) -> Result<String> {
        let scaled = value * PREFIX_SCALE[((self.exponent + 24) / 3) as usize];
        let mut output = self.format.format(scaled)?;
        output.push_str(prefix_symbol(self.exponent));
        ensure(
            output.len() <= MAX_LABEL_BYTES,
            "formatted SI label budget exceeded",
        )?;
        Ok(output)
    }
}
pub fn format_prefix(spec: &str, reference: f64, locale: &FormatLocale) -> Result<PrefixFormat> {
    PrefixFormat::new(spec, reference, locale)
}
pub fn format_text(spec: &str, value: &str, locale: &FormatLocale) -> Result<String> {
    NumberFormat::new(spec, locale)?.format_text(value)
}
fn group(digits: &str, locale: &FormatLocale, width: usize) -> String {
    if locale.grouping.is_empty() || locale.thousands.is_empty() {
        return digits.into();
    }
    let mut groups = vec![];
    let mut end = digits.len();
    let mut index = 0;
    let mut length: usize = 0;
    while end > 0 {
        let mut size = locale.grouping[index % locale.grouping.len()];
        if length.saturating_add(size + 1) > width {
            size = width.saturating_sub(length).max(1);
        }
        let start = end.saturating_sub(size);
        groups.push(&digits[start..end]);
        end = start;
        length = length.saturating_add(size + 1);
        if length > width {
            break;
        }
        index += 1;
    }
    groups.reverse();
    groups.join(&locale.thousands)
}
fn scientific(value: f64, precision: usize) -> String {
    let (coefficient, exponent) = decimal_parts(value, precision + 1);
    format!("{coefficient}e{exponent:+}")
}
fn shortest_parts(value: f64) -> (String, i32) {
    let text = format!("{value:e}");
    let (coefficient, exponent) = text.split_once('e').unwrap();
    (coefficient.into(), exponent.parse().unwrap())
}
// Exact finite IEEE-754 magnitude as decimal integer digits times 10^scale.
// This uses at most 1075 multiplications of at most 767 decimal digits.
fn exact_parts(value: f64) -> (String, i32) {
    let bits = value.to_bits();
    let field = ((bits >> 52) & 0x7ff) as i32;
    let mut mantissa = (bits & ((1_u64 << 52) - 1)) | if field == 0 { 0 } else { 1_u64 << 52 };
    if mantissa == 0 {
        return ("0".into(), 0);
    }
    let mut exponent = if field == 0 { -1074 } else { field - 1023 - 52 };
    // Cancel denominator factors so ordinary tick values avoid manufacturing
    // dozens of insignificant decimal zeros.
    if exponent < 0 {
        let cancelled = mantissa.trailing_zeros().min((-exponent) as u32);
        mantissa >>= cancelled;
        exponent += cancelled as i32;
    }
    let mut digits = mantissa
        .to_string()
        .bytes()
        .rev()
        .map(|b| b - b'0')
        .collect::<Vec<_>>();
    let factor = if exponent >= 0 { 2 } else { 5 };
    for _ in 0..exponent.unsigned_abs() {
        let mut carry = 0;
        for digit in &mut digits {
            let n = *digit * factor + carry;
            *digit = n % 10;
            carry = n / 10;
        }
        if carry > 0 {
            digits.push(carry);
        }
    }
    let text = digits.iter().rev().map(|d| (b'0' + *d) as char).collect();
    (text, exponent.min(0))
}
// Keep `count` decimal integer positions, round the discarded part toward +∞
// at exact half. Negative counts necessarily round to zero.
fn round_digits(digits: &str, count: i32) -> String {
    if count < 0 {
        return "0".into();
    }
    let count = count as usize;
    if count >= digits.len() {
        return format!("{digits}{}", "0".repeat(count - digits.len()));
    }
    let mut kept = digits.as_bytes()[..count].to_vec();
    if digits.as_bytes()[count] >= b'5' {
        let mut index = kept.len();
        loop {
            if index == 0 {
                kept.insert(0, b'1');
                break;
            }
            index -= 1;
            if kept[index] != b'9' {
                kept[index] += 1;
                break;
            }
            kept[index] = b'0';
        }
    }
    if kept.is_empty() {
        "0".into()
    } else {
        String::from_utf8(kept).unwrap()
    }
}
fn decimal_parts(value: f64, digits: usize) -> (String, i32) {
    if value == 0. {
        return (
            if digits == 1 {
                "0".into()
            } else {
                format!("0.{}", "0".repeat(digits - 1))
            },
            0,
        );
    }
    let (exact, scale) = exact_parts(value);
    let mut exponent = exact.len() as i32 + scale - 1;
    let mut rounded = round_digits(&exact, digits as i32);
    if rounded.len() > digits {
        rounded.pop();
        exponent += 1;
    }
    if digits > 1 {
        rounded.insert(1, '.');
    }
    (rounded, exponent)
}
fn significant(value: f64, digits: usize, decimal: bool) -> String {
    let (coefficient, exponent) = decimal_parts(value, digits);
    if !decimal && (exponent < -6 || exponent >= digits as i32) {
        format!("{coefficient}e{exponent:+}")
    } else {
        expand(&coefficient, exponent)
    }
}
fn expand(coefficient: &str, exponent: i32) -> String {
    let digits = coefficient.replace('.', "");
    let point = 1 + exponent;
    if point <= 0 {
        format!("0.{}{digits}", "0".repeat((-point) as usize))
    } else if point as usize >= digits.len() {
        format!("{digits}{}", "0".repeat(point as usize - digits.len()))
    } else {
        format!(
            "{}.{}",
            &digits[..point as usize],
            &digits[point as usize..]
        )
    }
}
fn trim(text: &str) -> String {
    let (coefficient, exponent) = text
        .split_once('e')
        .map_or((text, None), |(a, b)| (a, Some(b)));
    let coefficient = if coefficient.contains('.') {
        coefficient.trim_end_matches('0').trim_end_matches('.')
    } else {
        coefficient
    };
    match exponent {
        Some(e) => format!("{coefficient}e{e}"),
        None => coefficient.into(),
    }
}
fn fixed(value: f64, precision: usize) -> String {
    if value >= 1e21 {
        let (coefficient, e) = shortest_parts(value);
        format!("{coefficient}e{e:+}")
    } else {
        fixed_exact(value, precision)
    }
}
fn fixed_exact(value: f64, precision: usize) -> String {
    let (digits, scale) = exact_parts(value);
    let mut rounded = round_digits(&digits, digits.len() as i32 + scale + precision as i32);
    if precision == 0 {
        return rounded;
    }
    if rounded.len() <= precision {
        rounded = format!("{}{rounded}", "0".repeat(precision + 1 - rounded.len()));
    }
    rounded.insert(rounded.len() - precision, '.');
    rounded
}
fn radix(value: f64, base: u8, upper: bool) -> String {
    if value == 0. {
        return "0".into();
    }
    let bits = value.to_bits();
    let e = ((bits >> 52) & 0x7ff) as i32 - 1023;
    let mantissa = (bits & ((1_u64 << 52) - 1)) | (1_u64 << 52);
    let mut digits = vec![0u8];
    for index in (0..=e).rev() {
        let bit = if index >= e - 52 {
            ((mantissa >> (index - (e - 52))) & 1) as u8
        } else {
            0
        };
        let mut carry = bit;
        for digit in &mut digits {
            let n = *digit * 2 + carry;
            *digit = n % base;
            carry = n / base;
        }
        if carry > 0 {
            digits.push(carry);
        }
    }
    let alphabet = if upper {
        b"0123456789ABCDEF"
    } else {
        b"0123456789abcdef"
    };
    digits
        .iter()
        .rev()
        .map(|d| alphabet[*d as usize] as char)
        .collect()
}
