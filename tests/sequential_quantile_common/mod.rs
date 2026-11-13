#![allow(dead_code)]
use kartis::*;

#[derive(Clone, Debug)]
pub enum Token<'a> {
    Number(f64),
    Missing,
    JsOnly(&'a str),
}
pub fn token(raw: &str) -> Token<'_> {
    if let Some(value) = raw.strip_prefix("n:") {
        return Token::Number(match value {
            "NaN" => f64::NAN,
            "+Infinity" => f64::INFINITY,
            "-Infinity" => f64::NEG_INFINITY,
            _ => value.parse().unwrap(),
        });
    }
    if matches!(raw, "u" | "z") {
        Token::Missing
    } else {
        assert!(raw.starts_with("s:") || raw.starts_with("b:") || raw.starts_with("o:"));
        Token::JsOnly(raw)
    }
}
pub fn numbers(raw: &str) -> Vec<f64> {
    if raw.is_empty() {
        return vec![];
    }
    raw.split(',')
        .map(|raw| match token(raw) {
            Token::Number(value) => value,
            _ => panic!("Expected original numeric token: {raw}"),
        })
        .collect()
}
pub fn bits(actual: &[f64], expected: &[f64], context: &str) {
    assert_eq!(actual.len(), expected.len(), "{context}");
    for (index, (a, b)) in actual.iter().zip(expected).enumerate() {
        assert_eq!(
            a.to_bits(),
            b.to_bits(),
            "{context}[{index}]: {a:?} vs {b:?}"
        );
    }
}
pub fn options() -> ColorScaleOptions {
    ColorScaleOptions {
        kind: ColorScaleKind::SequentialQuantile,
        colors: vec![Color(0, 0, 0, 255), Color(255, 255, 255, 255)],
        unknown: Color(23, 71, 109, 137),
        ..Default::default()
    }
}
pub fn resolve(values: &[f64]) -> Result<ColorScale> {
    options().resolve(values.iter().copied().map(Some))
}
// An unambiguous grayscale output adapter. Rank itself comes exclusively from
// original D3 CSV, not from another implementation of the rank algorithm.
pub fn gray(parameter: f64) -> Color {
    assert!(parameter.is_finite() && (0. ..=1.).contains(&parameter));
    let channel = (parameter * 255.).round() as u8;
    Color(channel, channel, channel, 255)
}
#[derive(Debug)]
pub struct Query<'a> {
    pub input: &'a str,
    pub output: &'a str,
    pub profile: &'a str,
    pub arguments: &'a str,
}
#[derive(Debug)]
pub struct Case<'a> {
    pub id: &'a str,
    pub profile: &'a str,
    pub source: &'a str,
    pub domain: &'a str,
    pub queries: Vec<Query<'a>>,
    pub range: &'a str,
    pub quantiles: Vec<(usize, &'a str)>,
    pub mutated_getter: &'a str,
    pub fresh_getter: &'a str,
    pub copied_domain: &'a str,
    pub copied_range: &'a str,
    pub copy_queries: Vec<(&'a str, &'a str)>,
    sections_seen: u8,
}
pub fn cases() -> Vec<Case<'static>> {
    let mut cases = Vec::<Case<'static>>::new();
    for line in include_str!("../fixtures/sequential-quantile-d3.csv").lines() {
        if line.starts_with('#') {
            continue;
        }
        let fields = line.split('|').collect::<Vec<_>>();
        if fields[0] == "C" {
            assert_eq!(fields.len(), 5);
            cases.push(Case {
                id: fields[1],
                profile: fields[2],
                source: fields[3],
                domain: fields[4],
                queries: vec![],
                range: "",
                quantiles: vec![],
                mutated_getter: "",
                fresh_getter: "",
                copied_domain: "",
                copied_range: "",
                copy_queries: vec![],
                sections_seen: 0,
            });
            continue;
        }
        let case = cases.last_mut().expect("Record before C");
        assert_eq!(fields[1], case.id);
        match fields[0] {
            "Q" => {
                assert_eq!(fields.len(), 7);
                assert_eq!(fields[2].parse::<usize>().unwrap(), case.queries.len());
                case.queries.push(Query {
                    input: fields[4],
                    output: fields[5],
                    profile: fields[3],
                    arguments: fields[6],
                });
            }
            "R" => {
                assert_eq!(fields.len(), 3);
                assert_eq!(case.sections_seen & 1, 0);
                case.sections_seen |= 1;
                case.range = fields[2];
            }
            "P" => {
                assert_eq!(fields.len(), 4);
                case.quantiles.push((fields[2].parse().unwrap(), fields[3]));
            }
            "G" => {
                assert_eq!(fields.len(), 4);
                assert_eq!(case.sections_seen & 2, 0);
                case.sections_seen |= 2;
                case.mutated_getter = fields[2];
                case.fresh_getter = fields[3];
            }
            "Y" => {
                assert_eq!(fields.len(), 4);
                assert_eq!(case.sections_seen & 4, 0);
                case.sections_seen |= 4;
                case.copied_domain = fields[2];
                case.copied_range = fields[3];
            }
            "M" => {
                assert_eq!(fields.len(), 5);
                assert_eq!(fields[2].parse::<usize>().unwrap(), case.copy_queries.len());
                case.copy_queries.push((fields[3], fields[4]));
            }
            kind => panic!("Unclassified original oracle record: {kind}"),
        }
    }
    for case in &cases {
        assert_eq!(case.sections_seen, 7);
        assert_eq!(case.quantiles.len(), 9);
        assert_eq!(
            case.quantiles.iter().map(|row| row.0).collect::<Vec<_>>(),
            [0, 1, 2, 3, 4, 7, 16, 31, 256]
        );
        assert_eq!(case.copy_queries.len(), case.queries.len());
        assert_eq!(case.fresh_getter, case.domain);
        assert_eq!(case.copied_domain, case.domain);
        assert_eq!(case.copied_range, case.range);
        assert!(!case.mutated_getter.is_empty());
        for ((input, output), query) in case.copy_queries.iter().zip(&case.queries) {
            assert_eq!(*input, query.input);
            assert_eq!(*output, query.output);
        }
    }
    cases
}

pub fn query(scale: &ColorScale, row: &Query<'_>, context: &str) -> bool {
    match token(row.input) {
        Token::Number(value) if value.is_finite() => {
            assert_eq!(row.profile, "finite-number");
            let expected = numbers(row.output)[0];
            assert_eq!(
                scale.rank(value).unwrap().to_bits(),
                expected.to_bits(),
                "{context}"
            );
            assert_eq!(scale.map(Some(value)), gray(expected), "{context}");
            assert_eq!(
                row.arguments, row.output,
                "Original interpolator argument: {context}"
            );
            true
        }
        Token::Number(value) => {
            assert!(matches!(
                row.profile,
                "native-missing" | "native-query-coercion-or-infinity"
            ));
            assert_eq!(scale.rank(value), None, "{context}");
            assert_eq!(scale.map(Some(value)), options().unknown, "{context}");
            if value.is_nan() {
                assert_eq!(row.output, "u");
                assert!(row.arguments.is_empty());
            } else {
                assert!(matches!(row.output, "n:0" | "n:1"));
                assert_eq!(row.arguments, row.output);
            }
            false
        }
        Token::Missing => {
            assert_eq!(row.profile, "native-missing");
            assert_eq!(row.output, "u");
            assert!(row.arguments.is_empty());
            assert_eq!(scale.map(None), options().unknown, "{context}");
            false
        }
        Token::JsOnly(raw) => {
            // The typed Rust API has no string/bool/object query entrance.
            // These original native rows are explicitly accounted for, not
            // parsed as f64 or credited to the finite numeric equality scope.
            assert_eq!(row.profile, "native-query-coercion-or-infinity");
            assert!(raw.starts_with("s:") || raw.starts_with("b:"));
            assert!(
                row.output == "u"
                    || matches!(token(row.output),Token::Number(value) if value.is_finite())
            );
            if row.output == "u" {
                assert!(row.arguments.is_empty());
            } else {
                assert_eq!(row.arguments, row.output);
            }
            false
        }
    }
}

pub fn statistics(scale: &ColorScale, case: &Case<'_>) -> (usize, usize) {
    let mut finite = 0;
    let mut overflow = 0;
    for &(segments, raw) in &case.quantiles {
        if segments == 0 {
            assert!(scale.quantiles(segments).is_err());
            continue;
        }
        let expected = numbers(raw);
        assert_eq!(expected.len(), segments + 1);
        if expected.iter().any(|value| !value.is_finite()) {
            assert!(
                scale.quantiles(segments).is_err(),
                "{} / {segments}: original nonfinite R7 must reject",
                case.id
            );
            overflow += 1;
        } else {
            bits(
                &scale.quantiles(segments).unwrap().unwrap(),
                &expected,
                &format!("{} R7 {segments}", case.id),
            );
            finite += 1;
        }
    }
    (finite, overflow)
}

pub fn getter_and_copy(scale: &ColorScale, case: &Case<'_>) {
    let domain = numbers(case.domain);
    bits(scale.sample_domain().unwrap(), &domain, case.id);
    let clone = scale.clone();
    assert_eq!(
        clone.sample_domain().unwrap().as_ptr(),
        scale.sample_domain().unwrap().as_ptr(),
        "Clone shares immutable Arc: {}",
        case.id
    );
    let mut getter = scale.sample_domain().unwrap().to_vec();
    if let Some(first) = getter.first_mut() {
        *first = -123456.25;
    }
    getter.push(765432.5);
    bits(
        &getter,
        &numbers(case.mutated_getter),
        "D3 getter copy mutation",
    );
    bits(
        scale.sample_domain().unwrap(),
        &numbers(case.fresh_getter),
        "Original private samples unchanged",
    );
    let replacement = resolve(&[37., 41.]).unwrap();
    assert_ne!(replacement.sample_domain(), clone.sample_domain());
    bits(
        clone.sample_domain().unwrap(),
        &numbers(case.copied_domain),
        "Cloned distribution unchanged after replacement",
    );
    assert_eq!(clone.rank_colors(), scale.rank_colors());
    if domain.len() >= 2 {
        let colors = clone.rank_colors().unwrap();
        let range = numbers(case.copied_range);
        assert_eq!(colors.len(), range.len());
        for (actual, parameter) in colors.iter().zip(range) {
            assert_eq!(*actual, gray(parameter), "{} D3 ordinal range", case.id);
        }
        for (index, (input, output)) in case.copy_queries.iter().enumerate() {
            let row = &case.queries[index];
            assert_eq!(*input, row.input);
            assert_eq!(*output, row.output);
            query(&clone, row, &format!("{} copy {index}", case.id));
        }
    }
}

/// Strict reader for the small native-profile tail schema: strings, arrays and
/// objects only. Unknown value types, duplicate fields and trailing data reject.
/// Whitespace/object-field order do not alter values; quoted contents stay exact.
#[derive(Debug, PartialEq, Eq)]
pub enum NativeJson {
    String(String),
    Array(Vec<Self>),
    Object(std::collections::BTreeMap<String, Self>),
}
pub fn native_json(source: &str) -> NativeJson {
    struct Reader<'a> {
        bytes: &'a [u8],
        at: usize,
    }
    impl Reader<'_> {
        fn whitespace(&mut self) {
            while self.bytes.get(self.at).is_some_and(u8::is_ascii_whitespace) {
                self.at += 1;
            }
        }
        fn take(&mut self, expected: u8) {
            self.whitespace();
            assert_eq!(self.bytes.get(self.at), Some(&expected));
            self.at += 1;
        }
        fn hex4(&mut self) -> u32 {
            let text = std::str::from_utf8(&self.bytes[self.at..self.at + 4]).unwrap();
            let value = u32::from_str_radix(text, 16).unwrap();
            self.at += 4;
            value
        }
        fn string(&mut self) -> String {
            self.take(b'"');
            let mut bytes = Vec::new();
            loop {
                let character = *self.bytes.get(self.at).expect("Unclosed JSON string");
                self.at += 1;
                match character {
                    b'"' => break,
                    b'\\' => {
                        let escape = self.bytes[self.at];
                        self.at += 1;
                        match escape {
                            b'"' | b'\\' | b'/' => bytes.push(escape),
                            b'b' => bytes.push(8),
                            b'f' => bytes.push(12),
                            b'n' => bytes.push(b'\n'),
                            b'r' => bytes.push(b'\r'),
                            b't' => bytes.push(b'\t'),
                            b'u' => {
                                let mut code = self.hex4();
                                if (0xD800..=0xDBFF).contains(&code) {
                                    assert_eq!(&self.bytes[self.at..self.at + 2], b"\\u");
                                    self.at += 2;
                                    let low = self.hex4();
                                    assert!((0xDC00..=0xDFFF).contains(&low));
                                    code = 0x10000 + ((code - 0xD800) << 10) + (low - 0xDC00);
                                }
                                let value = char::from_u32(code).unwrap();
                                let mut buffer = [0; 4];
                                bytes.extend_from_slice(value.encode_utf8(&mut buffer).as_bytes());
                            }
                            _ => panic!("Invalid JSON escape"),
                        }
                    }
                    value => {
                        assert!(value >= 32, "Unescaped JSON control");
                        bytes.push(value);
                    }
                }
            }
            String::from_utf8(bytes).unwrap()
        }
        fn value(&mut self) -> NativeJson {
            self.whitespace();
            match self.bytes.get(self.at) {
                Some(b'"') => NativeJson::String(self.string()),
                Some(b'[') => {
                    self.at += 1;
                    let mut values = Vec::new();
                    self.whitespace();
                    if self.bytes.get(self.at) != Some(&b']') {
                        loop {
                            values.push(self.value());
                            self.whitespace();
                            if self.bytes.get(self.at) != Some(&b',') {
                                break;
                            }
                            self.at += 1;
                        }
                    }
                    self.take(b']');
                    NativeJson::Array(values)
                }
                Some(b'{') => {
                    self.at += 1;
                    let mut fields = std::collections::BTreeMap::new();
                    self.whitespace();
                    if self.bytes.get(self.at) != Some(&b'}') {
                        loop {
                            let key = self.string();
                            self.take(b':');
                            assert!(
                                fields.insert(key, self.value()).is_none(),
                                "Duplicate JSON field"
                            );
                            self.whitespace();
                            if self.bytes.get(self.at) != Some(&b',') {
                                break;
                            }
                            self.at += 1;
                        }
                    }
                    self.take(b'}');
                    NativeJson::Object(fields)
                }
                other => panic!("Value outside native-profile schema: {other:?}"),
            }
        }
    }
    let mut reader = Reader {
        bytes: source.as_bytes(),
        at: 0,
    };
    let result = reader.value();
    reader.whitespace();
    assert_eq!(reader.at, reader.bytes.len());
    result
}
