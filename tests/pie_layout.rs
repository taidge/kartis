use glory::render::{GraphicStroke, PathCommand as C, StrokeCap};
use kartis::*;
use std::f64::consts::{FRAC_PI_2, PI, TAU};
fn config() -> ChartConfig {
    ChartConfig {
        width: 400.,
        height: 300.,
        margin: Margin {
            top: 20.,
            right: 20.,
            bottom: 20.,
            left: 20.,
        },
        ..Default::default()
    }
}
fn stroke() -> GraphicStroke {
    GraphicStroke {
        color: Color(20, 30, 50, 255),
        width: 1.,
        cap: StrokeCap::Butt,
    }
}
fn options(profile: usize) -> (Vec<RadialDatum>, PieLayoutOptions) {
    let (values, a, b, inner, pad, corner): (Vec<f64>, f64, f64, f64, f64, f64) = match profile {
        0 => (vec![1., 3.], 0., TAU, 0.5, 0.06, 12.),
        1 => (vec![0., 1., 2.], 0.2, 4.6, 0.2, 0.3, 8.),
        2 => (vec![1., 1., 1.], 0., PI, 0.82, 0.16, 30.),
        3 => (vec![1., 4., 2.], 5. * PI / 6., -PI / 6., 0.45, 0.08, 22.),
        4 => (vec![1., 1.], 0., 0.12, 0.15, 0.07, 15.),
        5 => (vec![0., 0.], 0., PI, 0.4, 0.5, 0.),
        6 => (vec![1.], 0.3, 0.3 - TAU, 0.6, 2., 30.),
        7 => (vec![1., 2., 0.], 0., FRAC_PI_2, 0., 1.2, 5.),
        _ => (vec![1., 2., 3.], PI / 6., 5. * PI / 6., 0.7, 0.03, 0.),
    };
    let mut o = PieLayoutOptions::default();
    o.pie.inner_radius = inner;
    o.pie.start_angle = a;
    o.pie.end_angle = b;
    o.pie.pad_angle = pad;
    o.pie.style.corner_radius = corner;
    o.pie.labels = false;
    o.pie.style.border = Some(stroke());
    (
        values
            .into_iter()
            .enumerate()
            .map(|(i, v)| RadialDatum::new(i.to_string(), v))
            .collect(),
        o,
    )
}
fn near(a: f64, b: f64) {
    assert!(
        (a - b).abs() <= 2e-8 * (1. + a.abs().max(b.abs())),
        "{a} != {b}"
    );
}
fn coordinates(c: &C) -> Vec<f64> {
    match c {
        C::Move(p) | C::Line(p) => vec![p.x, p.y],
        C::Cubic { first, second, end } => vec![first.x, first.y, second.x, second.y, end.x, end.y],
        C::Close => vec![],
    }
}
#[test]
fn actual_d3_pie_arc_and_nivo_fit_oracle_264_padded_rounded_paths() {
    let mut count = 0;
    for line in include_str!("fixtures/pie-layout-d3.csv")
        .lines()
        .filter(|l| !l.starts_with('#'))
        .skip(1)
    {
        let f = line.split('|').collect::<Vec<_>>();
        let (data, mut o) = options(f[0].parse().unwrap());
        let pr = if f[3] == "auto" {
            PiePadRadius::Auto
        } else {
            PiePadRadius::Fixed(f[3].parse().unwrap())
        };
        o.padding = if f[1] == "applied" {
            PiePadding::D3Applied { pad_radius: pr }
        } else {
            PiePadding::NivoRequested { pad_radius: pr }
        };
        o.fit = if f[2] == "1" {
            PieFit::NivoSector
        } else {
            PieFit::Centered
        };
        let layout = pie_layout(&config(), &data, &o).unwrap();
        let source = f[4].parse::<usize>().unwrap();
        let sl = layout
            .slices
            .iter()
            .find(|s| s.source_index == source)
            .unwrap();
        for (v, i) in [
            (sl.value, 5),
            (sl.start_angle, 6),
            (sl.end_angle, 7),
            (sl.applied_pad, 8),
            (sl.geometry_pad, 9),
            (layout.center[0], 10),
            (layout.center[1], 11),
            (layout.radius, 12),
            (layout.inner_radius, 13),
        ] {
            near(v, f[i].parse().unwrap());
        }
        let got = layout.scene.marks[sl.mark_index.unwrap()]
            .geometry
            .commands();
        let expected = f[14].split(';').collect::<Vec<_>>();
        assert_eq!(
            got.len(),
            expected.len(),
            "profile {} mode {} fit {} radius {} source {}",
            f[0],
            f[1],
            f[2],
            f[3],
            f[4]
        );
        for (command, reference) in got.iter().zip(expected) {
            let tokens = reference.split(',').collect::<Vec<_>>();
            assert_eq!(
                match command {
                    C::Move(_) => "M",
                    C::Line(_) => "L",
                    C::Cubic { .. } => "C",
                    C::Close => "Z",
                },
                tokens[0]
            );
            for (actual, expected) in coordinates(command).into_iter().zip(tokens.iter().skip(1)) {
                near(actual, expected.parse().unwrap());
            }
        }
        count += 1;
    }
    assert_eq!(count, 264);
}
#[test]
fn padding_profiles_distinguish_requested_applied_and_inner_outer_collapses() {
    let (data, mut o) = options(7);
    o.pie.style.border = None;
    o.padding = PiePadding::D3Applied {
        pad_radius: PiePadRadius::Fixed(40.),
    };
    let applied = pie_layout(&config(), &data, &o).unwrap();
    o.padding = PiePadding::NivoRequested {
        pad_radius: PiePadRadius::Fixed(40.),
    };
    let requested = pie_layout(&config(), &data, &o).unwrap();
    assert!(
        applied
            .slices
            .iter()
            .all(|s| s.applied_pad < s.requested_pad)
    );
    assert!(
        applied
            .slices
            .iter()
            .all(|s| s.geometry_pad == s.applied_pad)
    );
    assert!(
        requested
            .slices
            .iter()
            .all(|s| s.geometry_pad == s.requested_pad)
    );
    assert_ne!(applied.scene, requested.scene);
    let (d, mut o) = options(2);
    o.pie.style.border = None;
    let s = pie_layout(&config(), &d, &o).unwrap();
    assert!(
        s.slices
            .iter()
            .any(|x| (x.inner_angles[0] - x.outer_angles[0]).abs() > 0.001)
    );
    o.pie.inner_radius = 0.05;
    o.pie.pad_angle = 0.5;
    let s = pie_layout(&config(), &d, &o).unwrap();
    assert!(
        s.slices
            .iter()
            .all(|x| x.inner_collapsed && !x.outer_collapsed)
    );
}
#[test]
fn zero_padding_is_real_d3_geometry_and_legacy_remains_byte_identical() {
    let mut o = PieLayoutOptions {
        fit: PieFit::Centered,
        padding: PiePadding::LegacyAngularGap,
        ..Default::default()
    };
    o.pie.inner_radius = 0.4;
    o.pie.pad_angle = 0.2;
    o.pie.style.corner_radius = 7.;
    let d = [
        RadialDatum::new("zero", 0.),
        RadialDatum::new("a", 1.),
        RadialDatum::new("b", 2.),
    ];
    assert_eq!(
        pie_layout(&config(), &d, &o).unwrap().scene,
        pie(&config(), &d, &o.pie).unwrap()
    );
    o.padding = PiePadding::D3Applied {
        pad_radius: PiePadRadius::Auto,
    };
    let s = pie_layout(&config(), &d, &o).unwrap();
    assert_eq!(s.slices.len(), 3);
    assert!(s.slices[0].mark_index.is_none());
    assert!(s.slices[0].outer_collapsed);
    o.pie.style.border = Some(stroke());
    let stroked = pie_layout(&config(), &d, &o).unwrap();
    assert!(stroked.slices[0].mark_index.is_some());
    assert_eq!(stroked.scene.marks[0].value, 0.);
    o.pie.style.border = None;
    o.padding = PiePadding::D3Applied {
        pad_radius: PiePadRadius::Fixed(0.),
    };
    let no_trim = pie_layout(&config(), &d, &o).unwrap();
    assert!(!no_trim.slices[0].outer_collapsed);
    assert!(no_trim.slices[0].mark_index.is_some());
    let empty = pie_layout(&config(), &[], &o).unwrap();
    assert!(empty.slices.is_empty() && empty.scene.marks.is_empty());
}
#[test]
fn exact_partial_fit_preserves_base_frame_hidden_active_and_full_annulus_hole() {
    let mut o = PieLayoutOptions::default();
    o.pie.labels = false;
    o.pie.end_angle = PI;
    let d = [RadialDatum::new("a", 1.), RadialDatum::new("b", 1.)];
    let s = pie_layout(&config(), &d, &o).unwrap();
    near(s.radius, 130.);
    near(s.center[0], 135.);
    near(s.center[1], 150.);
    near(s.bounds.width, 130.);
    near(s.bounds.height, 260.);
    o.pie.hidden.insert("b".into());
    o.pie.style.active_ids.insert("a".into());
    o.pie.style.active_outer_radius_offset = 12.;
    let active = pie_layout(&config(), &d, &o).unwrap();
    assert_eq!(active.center, s.center);
    assert_eq!(active.radius, s.radius);
    assert_eq!(active.bounds, s.bounds);
    o.pie.hidden.clear();
    o.pie.style.active_ids.clear();
    o.pie.end_angle = TAU;
    o.pie.inner_radius = 0.5;
    let full = pie_layout(&config(), &d[..1], &o).unwrap();
    assert!(
        full.scene
            .hit_test(full.center[0], full.center[1])
            .is_none()
    );
    assert!(
        full.scene
            .hit_test(full.center[0] + full.radius * 0.75, full.center[1])
            .is_some()
    );
    o.pie.start_angle = PI / 2. + 0.0001;
    o.pie.end_angle = PI - 0.0001;
    let exact = pie_layout(&config(), &d, &o).unwrap();
    assert!(exact.bounds.width <= config().inner_width() + 1e-8);
    assert!(exact.bounds.height <= config().inner_height() + 1e-8);
    assert!((exact.radius - 260. / 0.0001_f64.cos()).abs() < 1e-10);
    assert!(
        exact.radius > 260. + 1e-6,
        "exact fit accidentally used Nivo's rounded degree cardinal overshoot"
    );
    near(exact.center[0], 70.);
    near(exact.center[1], 20.);
}
fn crowded() -> PieLayout {
    let d = (0..12)
        .map(|i| RadialDatum::new(format!("long-label-{i}"), 1.))
        .collect::<Vec<_>>();
    let mut o = PieLayoutOptions {
        fit: PieFit::Centered,
        ..Default::default()
    };
    o.pie.start_angle = 0.3;
    o.pie.end_angle = 1.;
    o.pie.labels = false;
    o.pie.style.link_labels.enabled = true;
    o.pie.style.link_labels.diagonal_length = 8.;
    o.pie.style.link_labels.straight_length = 8.;
    o.pie.style.link_labels.style.size = Some(15.);
    pie_layout(&config(), &d, &o).unwrap()
}
fn measurements(s: &PieLayout) -> Vec<PieMeasuredLinkLabel> {
    s.link_candidates
        .iter()
        .map(|c| {
            c.measured(LayoutRect {
                x: 0.,
                y: -12.,
                width: 70.,
                height: 16.,
            })
        })
        .collect()
}
fn placement() -> PieLinkPlacementOptions {
    PieLinkPlacementOptions {
        collision: LabelCollisionOptions {
            viewport: LayoutRect {
                x: 0.,
                y: 0.,
                width: 400.,
                height: 300.,
            },
            padding: 2.,
            ..Default::default()
        },
        y_offsets: vec![],
    }
}
#[test]
fn measured_collision_hides_leaders_together_and_moves_real_owned_endpoints() {
    let s = crowded();
    let measured = measurements(&s);
    let saved = s.clone();
    let hidden = s.place_link_labels(&measured, &placement()).unwrap();
    let kept = hidden
        .link_placements
        .iter()
        .filter(|p| p.bounds.is_some())
        .collect::<Vec<_>>();
    assert!(!kept.is_empty() && kept.len() < s.link_candidates.len());
    for a in &kept {
        let ra = a.bounds.unwrap();
        for b in &kept {
            if a.id != b.id {
                assert_eq!(ra.overlap(b.bounds.unwrap()), 0.);
            }
        }
    }
    for p in &hidden.link_placements {
        let c = s.link_candidates.iter().find(|c| c.id == p.id).unwrap();
        if p.bounds.is_none() {
            assert!(!hidden.scene.marks.iter().any(|m| m.id == c.id));
            assert!(!hidden.scene.labels.iter().any(|l| l == &c.label));
        } else {
            let idx = p.label_index.unwrap();
            assert_eq!(hidden.scene.label_series[&idx], c.series);
            assert_eq!(hidden.scene.marks[p.leader_index.unwrap()].series, c.series);
        }
    }
    let shifted = s
        .place_link_labels(
            &measured,
            &PieLinkPlacementOptions {
                y_offsets: vec![20., 40., 60., 80., 100.],
                ..placement()
            },
        )
        .unwrap();
    assert!(
        shifted
            .link_placements
            .iter()
            .filter(|p| p.bounds.is_some())
            .count()
            > kept.len()
    );
    for p in shifted
        .link_placements
        .iter()
        .filter(|p| p.bounds.is_some())
    {
        let label = &shifted.scene.labels[p.label_index.unwrap()];
        let c = s.link_candidates.iter().find(|c| c.id == p.id).unwrap();
        let commands = shifted.scene.marks[p.leader_index.unwrap()]
            .geometry
            .commands();
        let C::Line(end) = commands[2] else { panic!() };
        near(end.y, label.y);
        near(end.x + c.text_offset, label.x);
    }
    assert_eq!(s, saved);
    let repeated = hidden.place_link_labels(&measured, &placement()).unwrap();
    assert_eq!(hidden, repeated);
}
#[test]
fn stale_measurements_budgets_hidden_raw_identity_and_invalid_input_are_atomic() {
    let s = crowded();
    let saved = s.clone();
    let mut custom = s.clone();
    let mut mark = Mark::new(
        "custom-layer",
        Geometry::Circle {
            x: 12.,
            y: 12.,
            radius: 5.,
        },
        Color(1, 2, 3, 255),
    );
    mark.label.clear();
    custom.scene.marks.push(mark);
    let edited = custom.scene.clone();
    assert!(
        custom
            .place_link_labels(&measurements(&custom), &placement())
            .is_err()
    );
    assert_eq!(custom.scene, edited);
    let mut styled = s.clone();
    styled.scene.marks[0].opacity = 0.5;
    let authored = styled.scene.clone();
    assert!(
        styled
            .place_link_labels(&measurements(&styled), &placement())
            .is_err()
    );
    assert_eq!(styled.scene, authored);
    let mut m = measurements(&s);
    m[0].signature.push('x');
    assert!(s.place_link_labels(&m, &placement()).is_err());
    let mut m = measurements(&s);
    m[0].bounds.width = f64::NAN;
    assert!(s.place_link_labels(&m, &placement()).is_err());
    assert!(s.place_link_labels(&[], &placement()).is_err());
    let o = PieLinkPlacementOptions {
        collision: LabelCollisionOptions {
            max_checks: 1,
            ..placement().collision
        },
        ..placement()
    };
    assert!(s.place_link_labels(&measurements(&s), &o).is_err());
    assert_eq!(s, saved);
    let d = [
        RadialDatum::new("a:1", 2.),
        RadialDatum::new("pie-layout-link:3:a:1", 1.),
        RadialDatum::new("zero", 0.),
    ];
    let mut o = PieLayoutOptions::default();
    o.pie.style.link_labels.enabled = true;
    let initial = pie_layout(&config(), &d, &o).unwrap();
    o.pie.hidden.insert(d[0].id.clone());
    let hidden = pie_layout(&config(), &d, &o).unwrap();
    assert_eq!(hidden.slices.len(), 2);
    assert_eq!(hidden.slices[0].source_index, 1);
    assert_eq!(hidden.scene.legend, initial.scene.legend);
    let ids = initial
        .scene
        .marks
        .iter()
        .map(|m| &m.id)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(ids.len(), initial.scene.marks.len());
    for i in 0..6 {
        let mut bad = o.clone();
        match i {
            0 => bad.pie.pad_angle = f64::NAN,
            1 => bad.pie.end_angle = bad.pie.start_angle,
            2 => bad.pie.inner_radius = 1.,
            3 => {
                bad.padding = PiePadding::D3Applied {
                    pad_radius: PiePadRadius::Fixed(-1.),
                }
            }
            4 => bad.pie.style.corner_radius = f64::INFINITY,
            _ => bad.pie.style.fill_opacity = 2.,
        };
        assert!(pie_layout(&config(), &d, &bad).is_err());
    }
    let duplicate = [RadialDatum::new("a", 1.), RadialDatum::new("a", 2.)];
    assert!(pie_layout(&config(), &duplicate, &o).is_err());
    assert!(pie_layout(&config(), &vec![RadialDatum::new("a", 1.); 257], &o).is_err());
}
#[cfg(feature = "raster")]
#[test]
fn cpu_raster_proves_annular_hole_padding_cut_and_actual_measured_text_placement() {
    use kartis::render::raster::*;
    let c = ChartConfig {
        width: 240.,
        height: 240.,
        margin: Margin {
            left: 20.,
            right: 20.,
            top: 20.,
            bottom: 20.,
        },
        ..Default::default()
    };
    let mut o = PieLayoutOptions {
        fit: PieFit::Centered,
        ..Default::default()
    };
    o.pie.labels = false;
    o.pie.inner_radius = 0.5;
    o.pie.pad_angle = 0.2;
    let d = [RadialDatum::new("a", 1.), RadialDatum::new("b", 1.)];
    let mut s = pie_layout(&c, &d, &o).unwrap();
    s.scene.legend.clear();
    let im = render_raster(&s.scene, &FontRegistry::default(), &Default::default()).unwrap();
    let px = |x: usize, y: usize| {
        &im.rgba[(y * im.width as usize + x) * 4..(y * im.width as usize + x) * 4 + 4]
    };
    assert_eq!(px(120, 120), &[255, 255, 255, 255]);
    assert_eq!(px(120, 30), &[255, 255, 255, 255]);
    assert_ne!(px(190, 120), &[255, 255, 255, 255]);
    let zero_data = [
        RadialDatum::new("zero", 0.),
        RadialDatum::new("nonzero", 1.),
    ];
    o.padding = PiePadding::D3Applied {
        pad_radius: PiePadRadius::Auto,
    };
    let mut fill_only = pie_layout(&c, &zero_data, &o).unwrap().scene;
    fill_only.legend.clear();
    let plain = render_raster(&fill_only, &FontRegistry::default(), &Default::default()).unwrap();
    o.pie.style.border = Some(GraphicStroke {
        color: Color(0, 0, 0, 255),
        width: 4.,
        cap: StrokeCap::Butt,
    });
    let mut stroked = pie_layout(&c, &zero_data, &o).unwrap().scene;
    stroked.legend.clear();
    let stroke_image =
        render_raster(&stroked, &FontRegistry::default(), &Default::default()).unwrap();
    let index = (45 * stroke_image.width as usize + 127) * 4;
    assert_eq!(&plain.rgba[index..index + 4], &[255, 255, 255, 255]);
    assert!(
        stroke_image.rgba[index] < 100,
        "D3 collapsed zero slice lost its real radial stroke"
    );
    let mut fonts = FontRegistry::new();
    fonts
        .register(
            std::sync::Arc::<[u8]>::from(include_bytes!("fonts/Lato-Regular.ttf").as_slice()),
            0,
        )
        .unwrap();
    let raw = crowded();
    let measured = raw
        .link_candidates
        .iter()
        .map(|c| {
            let outline = fonts
                .outline_label(&c.label, MissingGlyphPolicy::Error)
                .unwrap();
            let coords = outline
                .glyphs
                .iter()
                .flatten()
                .flat_map(coordinates)
                .collect::<Vec<_>>();
            let pairs = coords.as_chunks::<2>().0;
            let xmin = pairs.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min);
            let xmax = pairs.iter().map(|p| p[0]).fold(f64::NEG_INFINITY, f64::max);
            let ymin = pairs.iter().map(|p| p[1]).fold(f64::INFINITY, f64::min);
            let ymax = pairs.iter().map(|p| p[1]).fold(f64::NEG_INFINITY, f64::max);
            c.measured(LayoutRect {
                x: xmin - c.label.x,
                y: ymin - c.label.y,
                width: xmax - xmin,
                height: ymax - ymin,
            })
        })
        .collect::<Vec<_>>();
    let mut placed = raw.place_link_labels(&measured, &placement()).unwrap();
    placed.scene.legend.clear();
    let im = render_raster(&placed.scene, &fonts, &Default::default()).unwrap();
    assert!(
        im.rgba
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|p| p[0] < 100 && p[1] < 100 && p[2] < 100)
            .count()
            > 50
    );
}
#[test]
fn finite_extreme_values_are_normalized_and_mutated_measurement_candidates_reject() {
    let mut o = PieLayoutOptions::default();
    o.pie.labels = false;
    o.pie.pad_angle = 0.;
    let d = [
        RadialDatum::new("a", f64::MAX),
        RadialDatum::new("b", f64::MAX),
    ];
    let s = pie_layout(&config(), &d, &o).unwrap();
    assert_eq!(s.slices[0].fraction, 0.5);
    assert_eq!(s.slices[1].fraction, 0.5);
    near(s.slices[0].end_angle, PI);
    s.scene.validate().unwrap();
    assert!(
        pie_layout(
            &config(),
            &[d[0].clone(), RadialDatum::new("tiny", f64::from_bits(1))],
            &o
        )
        .is_err()
    );
    let mut c = crowded();
    let current_measurements = measurements(&c);
    c.link_candidates[0].label.x += 1.;
    assert!(
        c.place_link_labels(&current_measurements, &placement())
            .is_err()
    );
    let mut expanded = crowded();
    expanded
        .link_candidates
        .resize(257, expanded.link_candidates[0].clone());
    assert!(expanded.place_link_labels(&[], &placement()).is_err());
    let mut oversized = crowded();
    oversized.link_candidates[0].label.text = "x".repeat(MAX_LABEL_BYTES + 1);
    assert!(
        oversized
            .place_link_labels(&measurements(&oversized), &placement())
            .is_err()
    );
    let d = (0..256)
        .map(|i| RadialDatum::new(i.to_string(), 1.))
        .collect::<Vec<_>>();
    let many = pie_layout(&config(), &d, &o).unwrap();
    assert_eq!(many.scene.marks.len(), 256);
    assert_eq!(many.scene.legend.len(), 256);
}
