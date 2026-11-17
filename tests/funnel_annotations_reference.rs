use kartis::*;

fn frame(profile: usize, phase: usize, current: i32) -> FunnelSvgFrame {
    let mut data = vec![
        FunnelDatum::new("1", if phase == 1 { 10. } else { 8. }),
        FunnelDatum::new(1., 4.),
        FunnelDatum::new("zero", if phase == 1 { 0. } else { -0. }),
        FunnelDatum::new("transparent", 2.),
        FunnelDatum::new("tail", 0.),
    ];
    for (i, d) in data.iter_mut().enumerate() {
        d.size = Some([180., 120., 0., 90., 60.][i]);
    }
    if phase == 2 {
        data.swap(0, 1);
    }
    if phase == 3 {
        data.remove(1);
    }
    let config = ChartConfig {
        width: 600.,
        height: 420.,
        margin: Margin {
            left: 40.,
            right: 30.,
            top: 28.,
            bottom: 32.,
        },
        ..Default::default()
    };
    let options = FunnelLayoutOptions {
        direction: if profile.is_multiple_of(2) {
            FunnelDirection::Vertical
        } else {
            FunnelDirection::Horizontal
        },
        interpolation: if profile < 2 {
            FunnelInterpolation::Linear
        } else {
            FunnelInterpolation::Smooth
        },
        size: FunnelSize::DatumPixels,
        shape_blending: 0.5,
        before: None,
        after: None,
        labels: None,
        legend: false,
        border_width: 0.,
        current_size_extension: 12.,
        current: match current {
            0 => Some("1".into()),
            1 => Some(1.0.into()),
            _ => None,
        },
        ..Default::default()
    };
    FunnelSvgFrame::new(&config, &data, &options).unwrap()
}
fn recipes(recipe: usize) -> Vec<FunnelAnnotationSpec> {
    use FunnelAnnotationKind as K;
    use FunnelAnnotationMatch as M;
    use FunnelAnnotationNote as N;
    use FunnelAnnotationPosition as P;
    let make = |matcher, kind, note, x, y| {
        FunnelAnnotationSpec::new(matcher, kind, note, P::Relative(x), P::Relative(y))
    };
    if recipe == 0 {
        return vec![];
    }
    if recipe == 2 {
        return [
            (0., -100.),
            (100., -100.),
            (100., 0.),
            (100., 100.),
            (0., 100.),
            (-100., 100.),
            (-100., 0.),
            (-100., -100.),
        ]
        .into_iter()
        .map(|(x, y)| {
            let mut s = make(M::Id("1".into()), K::Rect, N::Text("eight".into()), x, y);
            s.offset = 4.;
            s.note_width = 24.;
            s
        })
        .collect();
    }
    let mut dot = make(
        M::Id("1".into()),
        K::Dot,
        N::Text("dot <&>".into()),
        60.,
        -10.,
    );
    dot.size = Some(12.);
    dot.offset = 91.;
    dot.note_width = 48.;
    let mut circle = make(
        M::Id(1.0.into()),
        K::Circle,
        N::Text("circle".into()),
        -70.,
        -12.,
    );
    circle.size = Some(46.);
    circle.offset = 3.;
    circle.note_width = 42.;
    let mut rect = make(
        M::Id("zero".into()),
        K::Rect,
        N::Text("rect".into()),
        60.,
        14.,
    );
    rect.offset = 6.;
    rect.width = Some(777.);
    rect.height = Some(555.);
    rect.x = Some(999.);
    rect.y = Some(888.);
    rect.border_radius = 3.;
    rect.note_width = 36.;
    let mut custom = make(
        M::Predicate("target".into()),
        K::Dot,
        N::Custom("note".into()),
        -80.,
        -16.,
    );
    custom.size = Some(0.);
    custom.note_width = 32.;
    let mut absolute = make(
        M::Id("tail".into()),
        K::Circle,
        N::Text(String::new()),
        0.,
        0.,
    );
    absolute.note_x = P::Absolute(500.);
    absolute.note_y = P::Absolute(370.);
    absolute.size = Some(0.);
    absolute.offset = 2.;
    absolute.note_width = 22.;
    let missing = make(
        M::Predicate("none".into()),
        K::Dot,
        N::Text("absent".into()),
        30.,
        30.,
    );
    let mut multi = make(
        M::Predicate("multi".into()),
        K::Dot,
        N::Text("multi".into()),
        30.,
        20.,
    );
    multi.size = Some(4.);
    multi.note_width = 24.;
    vec![dot, circle, rect, custom, absolute, missing, multi]
}
#[test]
fn complete_upstream_binding_and_connector_reference() {
    let input = include_str!("fixtures/funnel-annotations/reference.tsv");
    let mut lines = input.lines().filter(|l| !l.starts_with('#'));
    let mut cases = 0;
    let mut bindings = 0;
    while let Some(header) = lines.next() {
        let h: Vec<_> = header.split('\t').collect();
        assert_eq!(h[0], "case");
        let profile = h[1].parse().unwrap();
        let phase = h[2].parse().unwrap();
        let current = h[3].parse().unwrap();
        let recipe = h[4].parse().unwrap();
        let count: usize = h[5].parse().unwrap();
        let frame = frame(profile, phase, current);
        let mut matcher = |c: FunnelAnnotationMatchContext<'_>| {
            Ok(match c.key {
                "target" => c.part.datum.id == CategoryKey::from("transparent"),
                "multi" => c.source_index < 2,
                _ => false,
            })
        };
        let mut note = |c: FunnelAnnotationNoteContext<'_>| c.scene();
        let mut factories = FunnelAnnotationFactories {
            matcher: Some(&mut matcher),
            note: Some(&mut note),
        };
        let bound = funnel_annotations(
            &frame,
            &recipes(recipe),
            &mut factories,
            &Default::default(),
        )
        .unwrap();
        assert_eq!(bound.bindings().len(), count, "{header}");
        for b in bound.bindings() {
            let row: Vec<_> = lines.next().unwrap().split('\t').collect();
            assert_eq!(b.spec_index, row[0].parse::<usize>().unwrap());
            assert_eq!(b.part_index, row[1].parse::<usize>().unwrap());
            assert_eq!(
                b.kind,
                match row[2] {
                    "circle" => FunnelAnnotationKind::Circle,
                    "dot" => FunnelAnnotationKind::Dot,
                    "rect" => FunnelAnnotationKind::Rect,
                    _ => panic!("shape"),
                }
            );
            let actual = [
                b.center[0],
                b.center[1],
                b.size,
                b.width,
                b.height,
                b.computed.points[0][0],
                b.computed.points[0][1],
                b.computed.points[1][0],
                b.computed.points[1][1],
                b.computed.points[2][0],
                b.computed.points[2][1],
                b.computed.text[0],
                b.computed.text[1],
                b.computed.angle,
            ];
            assert_eq!(row.len(), actual.len() + 3);
            for (i, got) in actual.iter().enumerate() {
                let want: f64 = row[i + 3].parse().unwrap();
                assert!(
                    (got - want).abs() <= 1e-9,
                    "{header} binding{} field{i}: {got} != {want}",
                    b.spec_index
                );
            }
            bindings += 1;
        }
        cases += 1;
    }
    assert_eq!((cases, bindings), (144, 708));
}
