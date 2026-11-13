use kartis::{
    CategoricalColorScheme, CategoryKey, Color, ColorScheme, OrdinalColorOptions,
    OrdinalColorOverflow, OrdinalColorPalette, OrdinalColorScale, OrdinalColorUnknown,
};

const REFERENCE: &str = include_str!("fixtures/ordinal-color-d3.csv");

fn color(hex: &str) -> Color {
    let value = u32::from_str_radix(hex, 16).unwrap();
    Color((value >> 16) as u8, (value >> 8) as u8, value as u8, 255)
}

fn key(encoded: &str) -> CategoryKey {
    let (kind, value) = encoded.split_once(':').unwrap();
    match kind {
        "text" => value.into(),
        "number" => value.parse::<f64>().unwrap().into(),
        _ => panic!("unknown reference key type {kind}"),
    }
}

#[test]
fn every_nivo_categorical_and_default_brewer_palette_matches_the_catalog() {
    let mut categorical = 0;
    let mut brewer = 0;
    for line in REFERENCE.lines() {
        let columns: Vec<_> = line.split('|').collect();
        let palette = match columns[0] {
            "categorical" => {
                categorical += 1;
                let scheme = columns[1].parse::<CategoricalColorScheme>().unwrap();
                assert_eq!(scheme.to_string(), columns[1]);
                OrdinalColorPalette::Categorical(scheme)
            }
            "brewer" => {
                brewer += 1;
                OrdinalColorPalette::Brewer {
                    scheme: columns[1].parse().unwrap(),
                    size: None,
                }
            }
            _ => continue,
        };
        let expected: Vec<_> = columns[2].split_whitespace().map(color).collect();
        let options = OrdinalColorOptions {
            palette,
            ..Default::default()
        };
        let scale = options
            .resolve((0..=expected.len()).map(|index| CategoryKey::Number(index as f64)))
            .unwrap();
        assert_eq!(scale.palette(), expected, "{}", columns[1]);
        assert_eq!(scale.color_at(expected.len()), Some(expected[0]));
    }
    assert_eq!(categorical, CategoricalColorScheme::ALL.len());
    assert_eq!((categorical, brewer), (11, 27));
    assert!(CategoricalColorScheme::from_name("Category10").is_none());
    assert!("observable10".parse::<CategoricalColorScheme>().is_err());
}

#[test]
fn fixed_and_implicit_ordinal_queries_match_actual_d3() {
    let mut palette = Vec::new();
    let mut scale = None::<OrdinalColorScale>;
    let mut checked = 0;
    for line in REFERENCE.lines() {
        let columns: Vec<_> = line.split('|').collect();
        match columns[0] {
            "range" => palette = columns[1].split_whitespace().map(color).collect(),
            "domain" => {
                let unknown = match columns[1] {
                    "fixed" => OrdinalColorUnknown::Color(Color(1, 2, 3, 255)),
                    "extend" => OrdinalColorUnknown::Extend,
                    mode => panic!("unknown reference mode {mode}"),
                };
                scale = Some(
                    OrdinalColorOptions {
                        palette: OrdinalColorPalette::Custom(palette.clone()),
                        unknown,
                        ..Default::default()
                    }
                    .resolve(columns[2..].iter().map(|entry| key(entry)))
                    .unwrap(),
                );
            }
            "query" => {
                let scale = scale.as_mut().unwrap();
                let raw = key(columns[2]);
                let expected = Some(color(columns[3]));
                assert_eq!(
                    scale.map_or_insert(raw.clone()).unwrap(),
                    expected,
                    "{line}"
                );
                assert_eq!(scale.map(&raw), expected, "{line}");
                checked += 1;
            }
            "final" => {
                let expected: Vec<_> = columns[2..].iter().map(|entry| key(entry)).collect();
                assert_eq!(scale.as_ref().unwrap().domain(), expected);
            }
            _ => {}
        }
    }
    assert_eq!(checked, 30);
}

#[test]
fn fixed_domain_reserves_hidden_positions_and_keeps_unknown_out_of_the_domain() {
    let north = Color(100, 110, 120, 127);
    let hidden = Color(130, 140, 150, 96);
    let south = Color(160, 170, 180, 0);
    let other = Color(220, 210, 200, 63);
    let options = OrdinalColorOptions {
        palette: OrdinalColorPalette::Custom(vec![north, hidden, south]),
        domain: Some(
            ["north", "hidden", "south", "east"]
                .map(CategoryKey::from)
                .into(),
        ),
        unknown: OrdinalColorUnknown::Color(other),
        overflow: OrdinalColorOverflow::Unknown,
    };
    let before = options
        .resolve(["hidden", "north", "south", "east"].map(CategoryKey::from))
        .unwrap();
    let mut after = options
        .resolve(["south", "north"].map(CategoryKey::from))
        .unwrap();
    assert_eq!(before, after);
    for (id, expected) in [
        ("north", north),
        ("hidden", hidden),
        ("south", south),
        ("east", other),
        ("unseen", other),
    ] {
        assert_eq!(after.map_or_insert(id.into()).unwrap(), Some(expected));
    }
    assert_eq!(after, before);
    assert_eq!(after.color_at(4), None);
    assert_eq!(after.map(&CategoryKey::Number(f64::NAN)), None);
    assert!(after.map_or_insert(CategoryKey::Number(f64::NAN)).is_err());
    assert_eq!(after, before);
}

#[test]
fn explicit_lookup_stays_pure_and_invalid_configurations_are_rejected() {
    let mut scale = OrdinalColorOptions {
        unknown: OrdinalColorUnknown::Extend,
        ..Default::default()
    }
    .resolve([])
    .unwrap();
    let number = CategoryKey::Number(-0.);
    assert_eq!(scale.map(&number), None);
    assert!(scale.domain().is_empty());
    assert_eq!(
        scale.map_or_insert(number).unwrap(),
        Some(scale.palette()[0])
    );
    assert_eq!(scale.map(&0.0.into()), Some(scale.palette()[0]));
    assert_eq!(scale.map(&"0".into()), None);
    let CategoryKey::Number(zero) = scale.domain()[0] else {
        panic!("missing numeric zero");
    };
    assert_eq!(zero.to_bits(), 0);

    let size = 4;
    let brewer = OrdinalColorOptions {
        palette: OrdinalColorPalette::Brewer {
            scheme: ColorScheme::RedBlue,
            size: Some(size),
        },
        ..Default::default()
    };
    assert_eq!(
        brewer.resolve([]).unwrap().palette(),
        ColorScheme::RedBlue.palette(size).unwrap()
    );
    for invalid in [
        OrdinalColorPalette::Custom(Vec::new()),
        OrdinalColorPalette::Custom(vec![Color(0, 0, 0, 0); 257]),
        OrdinalColorPalette::Brewer {
            scheme: ColorScheme::Blues,
            size: Some(10),
        },
        OrdinalColorPalette::Brewer {
            scheme: ColorScheme::Viridis,
            size: None,
        },
    ] {
        assert!(
            OrdinalColorOptions {
                palette: invalid,
                ..Default::default()
            }
            .resolve([])
            .is_err()
        );
    }
    assert!(
        OrdinalColorOptions::default()
            .resolve([CategoryKey::Number(f64::INFINITY)])
            .is_err()
    );
}

#[test]
fn source_and_extension_budgets_are_checked_transactionally() {
    let options = OrdinalColorOptions {
        unknown: OrdinalColorUnknown::Extend,
        ..Default::default()
    };
    assert!(
        options
            .resolve(std::iter::repeat_n(CategoryKey::from("duplicate"), 20_001))
            .is_err()
    );
    assert!(
        options
            .resolve([CategoryKey::from("a".repeat(1025))])
            .is_err()
    );

    let body = "a".repeat(1020);
    let mut scale = options
        .resolve((0..4096).map(|index| CategoryKey::from(format!("{index:04}{body}"))))
        .unwrap();
    let last = scale.domain().last().unwrap().clone();
    let color = scale.map(&last);
    assert!(scale.map_or_insert("extra".into()).is_err());
    assert_eq!(scale.domain().len(), 4096);
    assert_eq!(scale.map_or_insert(last).unwrap(), color);
}
