use kartis::{Color, ColorScheme};
use std::collections::BTreeSet;

fn rgb(hex: &str) -> Color {
    let color = u32::from_str_radix(hex, 16).unwrap();
    Color((color >> 16) as u8, (color >> 8) as u8, color as u8, 255)
}

#[test]
fn every_named_scheme_matches_actual_d3_samples() {
    let mut names = BTreeSet::new();
    let mut count = 0;
    for row in include_str!("fixtures/color-schemes-d3.csv")
        .lines()
        .skip(2)
    {
        let mut fields = row.split('|');
        let name = fields.next().unwrap();
        let scheme = name.parse::<ColorScheme>().unwrap();
        let parameter = fields.next().unwrap();
        let t = parameter.parse().unwrap();
        let expected = rgb(fields.next().unwrap());
        assert_eq!(scheme.sample(t).unwrap(), expected, "{name} at {parameter}");
        assert_eq!(fields.next(), None);
        names.insert(name);
        count += 1;
    }
    assert_eq!(count, 5690);
    assert_eq!(names.len(), 38);
    assert_eq!(names, ColorScheme::ALL.iter().map(|s| s.id()).collect());
}

#[test]
fn every_official_brewer_palette_preserves_exact_colors_and_order() {
    let mut palettes = BTreeSet::new();
    for row in include_str!("fixtures/color-scheme-palettes-d3.csv")
        .lines()
        .skip(2)
    {
        let mut fields = row.split('|');
        let scheme = ColorScheme::from_name(fields.next().unwrap()).unwrap();
        let size = fields.next().unwrap().parse().unwrap();
        let expected: Vec<_> = fields
            .next()
            .unwrap()
            .split_ascii_whitespace()
            .map(rgb)
            .collect();
        assert_eq!(
            scheme.palette(size).unwrap(),
            expected,
            "{scheme} at {size}"
        );
        assert_eq!(expected.len(), size);
        assert_eq!(fields.next(), None);
        assert!(palettes.insert((scheme, size)));
    }
    assert_eq!(palettes.len(), 207);
    let mut supported = 0;
    for &scheme in ColorScheme::ALL {
        if let Some(sizes) = scheme.palette_sizes() {
            assert_eq!(*sizes.start(), 3);
            assert!(scheme.palette(*sizes.end() + 1).is_err());
            for size in sizes {
                assert!(palettes.contains(&(scheme, size)));
            }
            supported += 1;
        } else {
            assert!(scheme.palette(5).is_err());
        }
        assert!(scheme.palette(0).is_err());
        assert!(scheme.palette(2).is_err());
        assert!(scheme.palette(usize::MAX).is_err());
    }
    assert_eq!(supported, 27);
}

#[test]
fn catalog_is_case_sensitive_and_sampling_has_a_bounded_parameter_contract() {
    assert_eq!(
        ColorScheme::from_name("brown_blueGreen"),
        Some(ColorScheme::BrownBlueGreen)
    );
    assert_eq!(
        ColorScheme::from_name("cubehelixDefault"),
        Some(ColorScheme::CubehelixDefault)
    );
    assert_eq!(ColorScheme::from_name("brown_bluegreen"), None);
    assert!(ColorScheme::from_name("Viridis").is_none());
    assert!("".parse::<ColorScheme>().is_err());
    for &scheme in ColorScheme::ALL {
        assert_eq!(scheme.to_string(), scheme.id());
        assert_eq!(scheme.id().parse::<ColorScheme>().unwrap(), scheme);
        assert_eq!(scheme.sample(-f64::MAX), scheme.sample(0.));
        assert_eq!(scheme.sample(-0.), scheme.sample(0.));
        assert_eq!(scheme.sample(f64::MAX), scheme.sample(1.));
        for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(scheme.sample(invalid).is_err());
        }
    }
}

#[test]
fn both_cyclical_profiles_close_and_colorbrewer_keeps_distinct_discrete_sizes() {
    for scheme in [ColorScheme::Rainbow, ColorScheme::Sinebow] {
        assert_eq!(scheme.sample(0.), scheme.sample(1.));
        assert_ne!(scheme.sample(0.), scheme.sample(0.5));
    }
    let blues = ColorScheme::Blues;
    assert_eq!(blues.sample(0.).unwrap(), rgb("f7fbff"));
    assert_eq!(blues.sample(1.).unwrap(), rgb("08306b"));
    assert_eq!(blues.palette(3).unwrap()[0], rgb("deebf7"));
    assert_ne!(blues.palette(3).unwrap()[0], blues.sample(0.).unwrap());
}
