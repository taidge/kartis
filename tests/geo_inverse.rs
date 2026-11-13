use kartis::*;

fn frame() -> ChartConfig {
    ChartConfig {
        width: 700.,
        height: 500.,
        margin: Margin {
            left: 35.,
            right: 25.,
            top: 25.,
            bottom: 35.,
        },
        ..Default::default()
    }
}
fn kind(value: &str) -> GeoProjection {
    match value {
        "equirectangular" => GeoProjection::Equirectangular,
        "mercator" => GeoProjection::Mercator,
        "transverse" => GeoProjection::TransverseMercator,
        "equalEarth" => GeoProjection::EqualEarth,
        "naturalEarth" => GeoProjection::NaturalEarth,
        "orthographic" => GeoProjection::Orthographic,
        "equalArea" => GeoProjection::AzimuthalEqualArea,
        "equidistant" => GeoProjection::AzimuthalEquidistant,
        "stereographic" => GeoProjection::Stereographic,
        "gnomonic" => GeoProjection::Gnomonic,
        _ => panic!("unknown original D3 projection"),
    }
}
fn kinds() -> [GeoProjection; 10] {
    [
        GeoProjection::Equirectangular,
        GeoProjection::Mercator,
        GeoProjection::TransverseMercator,
        GeoProjection::EqualEarth,
        GeoProjection::NaturalEarth,
        GeoProjection::Orthographic,
        GeoProjection::AzimuthalEqualArea,
        GeoProjection::AzimuthalEquidistant,
        GeoProjection::Stereographic,
        GeoProjection::Gnomonic,
    ]
}
fn vector([l, f]: [f64; 2]) -> [f64; 3] {
    let (l, f) = (l.to_radians(), f.to_radians());
    [f.cos() * l.cos(), f.cos() * l.sin(), f.sin()]
}
fn distance(a: [f64; 2], b: [f64; 2]) -> f64 {
    let (a, b) = (vector(a), vector(b));
    (a[0] - b[0]).hypot(a[1] - b[1]).hypot(a[2] - b[2])
}
fn near(a: f64, b: f64, tolerance: f64) {
    assert!(
        (a - b).abs() <= tolerance,
        "{a} differs from {b} (tolerance {tolerance})"
    );
}
struct Case {
    id: usize,
    options: GeoMapOptions,
    scale: f64,
    translate: [f64; 2],
    ring: Vec<[f64; 2]>,
    queries: Vec<([f64; 2], Option<[f64; 2]>, u8)>,
}
fn reference() -> Vec<Case> {
    let mut cases = Vec::<Case>::new();
    for line in include_str!("fixtures/geo-inverse-d3.csv")
        .lines()
        .filter(|line| !line.starts_with('#'))
    {
        let c = line.split('|').collect::<Vec<_>>();
        let number = |i: usize| c[i].parse::<f64>().unwrap();
        match c[0] {
            "C" => cases.push(Case {
                id: c[1].parse().unwrap(),
                options: GeoMapOptions {
                    projection: kind(c[2]),
                    center: [number(3), number(4)],
                    rotation: number(5),
                    clip_angle: if c[6] == "none" {
                        None
                    } else {
                        Some(number(6))
                    },
                    fit: if c[7] == "data" {
                        GeoFit::Data
                    } else {
                        GeoFit::World
                    },
                    padding: 7.,
                    scale: if c[8] == "auto" {
                        None
                    } else {
                        Some(number(8))
                    },
                    translate: if c[9] == "auto" {
                        None
                    } else {
                        Some([number(9), number(10)])
                    },
                    ..Default::default()
                },
                scale: number(11),
                translate: [number(12), number(13)],
                ring: Vec::new(),
                queries: Vec::new(),
            }),
            "G" => cases.last_mut().unwrap().ring.push([number(2), number(3)]),
            "Q" | "B" | "H" => cases.last_mut().unwrap().queries.push((
                [number(2), number(3)],
                if c[4] == "none" {
                    None
                } else {
                    Some([number(4), number(5)])
                },
                match c[0] {
                    "Q" => 0,
                    "B" => 1,
                    _ => 2,
                },
            )),
            _ => panic!("invalid original D3 inverse fixture"),
        }
    }
    cases
}
fn context(case: &Case) -> GeoProjectionContext {
    let features = if case.ring.is_empty() {
        Vec::new()
    } else {
        vec![GeoFeature {
            id: "region".into(),
            label: "Region".into(),
            value: None,
            polygons: vec![vec![case.ring.clone()]],
        }]
    };
    geographic_map_layout(&frame(), &features, &case.options)
        .unwrap_or_else(|e| panic!("case{}: {e}", case.id))
        .projection
}

#[test]
fn independent_original_d3_inverses_and_point_stream_visibility_cover_all_ten_projections() {
    let cases = reference();
    assert_eq!(cases.len(), 100);
    let (mut count, mut positive, mut negative, mut data) = (0, 0, 0, 0);
    for case in &cases {
        let ctx = context(case);
        near(ctx.scale(), case.scale, 1e-8);
        near(ctx.translate()[0], case.translate[0], 1e-8);
        near(ctx.translate()[1], case.translate[1], 1e-8);
        assert_eq!(ctx.dimensions(), [700., 500.]);
        if case.options.fit == GeoFit::Data {
            data += 1;
        }
        for &(point, expected, boundary) in &case.queries {
            if boundary != 0 {
                continue;
            }
            count += 1;
            let actual = ctx
                .invert(point)
                .unwrap_or_else(|e| panic!("case{} point{point:?}: {e}", case.id));
            match (expected, actual) {
                (None, None) => negative += 1,
                (Some(expected), Some(actual)) => {
                    positive += 1;
                    assert!(
                        distance(actual, expected) < 1e-9,
                        "case{} pixel{point:?}: inverse{actual:?} vs original D3{expected:?}",
                        case.id
                    );
                    let forward = ctx
                        .project(actual)
                        .unwrap()
                        .expect("inverse must remain forward visible");
                    assert!(
                        (forward[0] - point[0]).hypot(forward[1] - point[1]) < 2e-6,
                        "case{} roundtrip {point:?} -> {actual:?} -> {forward:?}",
                        case.id
                    );
                }
                (expected, actual) => panic!(
                    "case{} {:?} pixel{point:?}: original D3 expected{expected:?}, inverse{actual:?}",
                    case.id, case.options
                ),
            }
        }
    }
    assert_eq!((count, positive, negative, data), (11081, 5055, 6026, 30));
}

#[test]
fn original_d3_singular_boundaries_use_spherical_equivalence_and_forward_visibility() {
    let (mut queried, mut visible) = (0, 0);
    for case in reference() {
        let ctx = context(&case);
        for &(point, expected, boundary) in &case.queries {
            if boundary != 1 {
                continue;
            }
            queried += 1;
            if let Some(actual) = ctx.invert(point).unwrap() {
                visible += 1;
                assert!(
                    distance(actual, expected.unwrap()) < 2e-7,
                    "boundary case{} {point:?}: {actual:?} vs{expected:?}",
                    case.id
                );
                assert!(ctx.project(actual).unwrap().is_some());
            }
        }
    }
    assert_eq!(queried, 385);
    assert!(visible > 200);
}

#[test]
fn exact_cap_profile_keeps_original_d3_strict_visibility_and_existing_forward_tolerance() {
    let cases = reference();
    let mut count = 0;
    let mut d3_visible = 0;
    for line in include_str!("fixtures/geo-inverse-d3.csv")
        .lines()
        .filter(|s| s.starts_with("H|"))
    {
        let parts = line.split('|').collect::<Vec<_>>();
        let case = &cases[parts[1].parse::<usize>().unwrap()];
        let point = [parts[2].parse().unwrap(), parts[3].parse().unwrap()];
        let expected = [parts[4].parse().unwrap(), parts[5].parse().unwrap()];
        d3_visible += usize::from(parts[6] == "true");
        let c = vector(case.options.center);
        let p = vector(expected);
        let dot = c.iter().zip(p).map(|(a, b)| a * b).sum::<f64>();
        near(
            dot,
            case.options.clip_angle.unwrap().to_radians().cos(),
            1e-12,
        );
        let ctx = context(case);
        // Existing project deliberately accepts this exact cap within EPS.
        assert!(ctx.project(expected).unwrap().is_some());
        let actual = ctx
            .invert(point)
            .unwrap()
            .expect("inclusive forward cap must remain invertible");
        assert!(distance(actual, expected) < 2e-7);
        let forward = ctx.project(actual).unwrap().unwrap();
        assert!((forward[0] - point[0]).hypot(forward[1] - point[1]) < 2e-6);
        count += 1;
    }
    assert_eq!((count, d3_visible), (14, 5));
}

#[test]
fn plot_window_is_inclusive_and_separate_from_intrinsic_projection_visibility() {
    let ctx = geographic_map_layout(
        &frame(),
        &[],
        &GeoMapOptions {
            scale: Some(10000.),
            translate: Some([355., 245.]),
            ..Default::default()
        },
    )
    .unwrap()
    .projection;
    let plot = ctx.plot();
    for p in [
        [plot.x, plot.y],
        [plot.x + plot.width, plot.y],
        [plot.x, plot.y + plot.height],
        [plot.x + plot.width, plot.y + plot.height],
    ] {
        assert!(ctx.invert_in_plot(p).unwrap().is_some());
    }
    let point = [plot.x - 0.001, plot.y + plot.height / 2.];
    assert!(ctx.invert(point).unwrap().is_some());
    assert_eq!(ctx.invert_in_plot(point).unwrap(), None);
    for p in [
        [f64::NAN, 0.],
        [0., f64::INFINITY],
        [1_000_000.001, 0.],
        [0., -1_000_000.001],
    ] {
        assert!(ctx.invert(p).is_err());
        assert!(ctx.invert_in_plot(p).is_err());
    }
}

#[test]
fn accepted_affines_still_check_scene_budget_and_unrepresentable_inverse_arithmetic() {
    let ctx = geographic_map_layout(
        &frame(),
        &[],
        &GeoMapOptions {
            scale: Some(1e-320),
            translate: Some([0., 0.]),
            ..Default::default()
        },
    )
    .unwrap()
    .projection;
    assert!(ctx.invert([0., 0.]).unwrap().is_some());
    assert!(ctx.invert([1., 0.]).is_err());
    assert!(ctx.invert_in_plot([40., 40.]).is_err());
    // Budget checks precede plot clipping, so invalid far-out pixels are errors.
    assert!(ctx.invert_in_plot([1_000_001., 0.]).is_err());
    let ordinary = geographic_map_layout(&frame(), &[], &GeoMapOptions::default())
        .unwrap()
        .projection;
    assert_eq!(ordinary.invert([1_000_000., 1_000_000.]).unwrap(), None);
    assert!(
        geographic_map_layout(
            &frame(),
            &[],
            &GeoMapOptions {
                scale: Some(f64::NAN),
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        geographic_map_layout(
            &frame(),
            &[],
            &GeoMapOptions {
                center: [181., 0.],
                ..Default::default()
            }
        )
        .is_err()
    );
}

#[test]
fn explicit_azimuthal_caps_reject_outer_disk_without_resurrecting_the_back_hemisphere() {
    for projection in [
        GeoProjection::Orthographic,
        GeoProjection::AzimuthalEqualArea,
        GeoProjection::AzimuthalEquidistant,
        GeoProjection::Stereographic,
        GeoProjection::Gnomonic,
    ] {
        for cap in [0.01, 30., 75.] {
            let ctx = geographic_map_layout(
                &frame(),
                &[],
                &GeoMapOptions {
                    projection,
                    clip_angle: Some(cap),
                    scale: Some(90.),
                    translate: Some([355., 245.]),
                    ..Default::default()
                },
            )
            .unwrap()
            .projection;
            let disk = ctx.projection_disk().unwrap();
            let inside = [disk.center[0] + disk.radius * 0.99, disk.center[1]];
            assert!(ctx.invert(inside).unwrap().is_some());
            assert_eq!(
                ctx.invert([
                    disk.center[0] + disk.radius * 1.01 + 0.00001,
                    disk.center[1]
                ])
                .unwrap(),
                None,
                "{projection:?} cap{cap}"
            );
            assert_eq!(ctx.project([180., 0.]).unwrap(), None);
            let boundary = ctx.project([cap, 0.]).unwrap().unwrap();
            assert!(distance(ctx.invert(boundary).unwrap().unwrap(), [cap, 0.]) < 1e-8);
        }
    }
}

#[test]
fn asymmetric_data_fit_nonzero_raw_center_scale_overrides_and_large_roll_roundtrip() {
    for projection in kinds() {
        let opts = GeoMapOptions {
            projection,
            center: [35., 20.],
            rotation: 25.,
            fit: GeoFit::Data,
            scale: Some(75.),
            translate: Some([311., 207.]),
            ..Default::default()
        };
        let feature = GeoFeature {
            id: "offset".into(),
            label: "Offset".into(),
            value: None,
            polygons: vec![vec![vec![
                [40., 22.],
                [40., 30.],
                [52., 30.],
                [52., 22.],
                [40., 22.],
            ]]],
        };
        let ctx = geographic_map_layout(&frame(), &[feature], &opts)
            .unwrap()
            .projection;
        assert!(ctx.fitted_raw_center().iter().any(|v| v.abs() > 0.001));
        assert_eq!(ctx.scale(), 75.);
        assert_eq!(ctx.translate(), [311., 207.]);
        for p in [[40., 22.], [45., 25.], [52., 30.]] {
            let pixel = ctx.project(p).unwrap().unwrap();
            let world = ctx.invert(pixel).unwrap().unwrap();
            assert!(
                distance(world, p) < 1e-10,
                "{projection:?} asymmetric Data fit used wrong raw center"
            );
        }
        let world = geographic_map_layout(
            &frame(),
            &[],
            &GeoMapOptions {
                projection,
                center: [-120., -40.],
                rotation: 360_000.,
                ..Default::default()
            },
        )
        .unwrap()
        .projection;
        for p in [[-120., -40.], [-110., -35.]] {
            let pixel = world.project(p).unwrap().unwrap();
            assert!(distance(world.invert(pixel).unwrap().unwrap(), p) < 1e-9);
        }
    }
}
