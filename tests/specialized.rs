use kartis::*;
#[test]
fn network_is_repeatable_and_rejects_unknown_links() {
    let c = ChartConfig::default();
    let nodes = vec![
        NetworkNode::new("a"),
        NetworkNode::new("b"),
        NetworkNode::new("c"),
    ];
    let links = vec![
        NetworkLink::new("a", "b", 4.),
        NetworkLink::new("b", "c", 1.),
    ];
    let options = NetworkOptions::default();
    let s = network(&c, &nodes, &links, &options).unwrap();
    assert_eq!(s, network(&c, &nodes, &links, &options).unwrap());
    assert_eq!(s.marks.len(), 5);
    assert!(network(&c, &nodes, &[NetworkLink::new("z", "a", 1.)], &options).is_err());
}
#[test]
fn tukey_quartiles_and_outliers_are_computed_from_samples() {
    let s = boxplot(
        &ChartConfig::default(),
        &[BoxSample::new("A", vec![1., 2., 3., 4., 100.])],
    )
    .unwrap();
    let median = s.marks.iter().find(|m| m.id == "median:A").unwrap();
    assert_eq!(median.value, 3.);
    let outliers: Vec<_> = s
        .marks
        .iter()
        .filter(|m| m.id.starts_with("outlier:"))
        .collect();
    assert_eq!(outliers.len(), 1);
    assert_eq!(outliers[0].value, 100.);
}
#[test]
fn parallel_missing_values_break_paths_and_constant_dimensions_center() {
    let s = parallel_coordinates(
        &ChartConfig::default(),
        &["a".into(), "b".into(), "c".into()],
        &[ParallelRecord {
            id: "r".into(),
            values: vec![Some(3.), None, Some(5.)],
        }],
    )
    .unwrap();
    let Geometry::Path(p) = &s.marks.last().unwrap().geometry else {
        panic!()
    };
    assert_eq!(p.len(), 2);
    assert!(
        p.iter()
            .all(|p| matches!(p, glory::render::PathCommand::Move(_)))
    );
}
#[test]
fn geo_holes_keep_opposite_winding_for_canvas_and_svg() {
    let feature = GeoFeature {
        id: "land".into(),
        label: "land".into(),
        value: Some(10.),
        polygons: vec![vec![
            vec![[-30., -30.], [30., -30.], [30., 30.], [-30., 30.]],
            vec![[-10., -10.], [10., -10.], [10., 10.], [-10., 10.]],
        ]],
    };
    let c = ChartConfig::default();
    for projection in [Projection::Mercator, Projection::Equirectangular] {
        let s = choropleth(&c, std::slice::from_ref(&feature), projection).unwrap();
        let x = c.margin.left + c.inner_width() / 2.;
        let y = c.margin.top + c.inner_height() / 2.;
        assert!(s.hit_test(x, y).is_none());
        assert!(s.hit_test(x + c.inner_width() * 20. / 360., y).is_some());
    }
}
#[test]
fn chord_handles_zero_asymmetric_and_self_matrices() {
    let c = ChartConfig::default();
    let ids = vec!["a".into(), "b".into()];
    let s = chord(&c, &ids, &[vec![2., 5.], vec![1., 0.]], 0.04).unwrap();
    assert_eq!(
        s.marks
            .iter()
            .filter(|m| m.id.starts_with("ribbon:"))
            .count(),
        2
    );
    assert_eq!(
        s.marks.iter().find(|m| m.id == "ribbon:0:1").unwrap().value,
        6.
    );
    assert!(
        chord(&c, &ids, &[vec![0., 0.], vec![0., 0.]], 0.)
            .unwrap()
            .marks
            .is_empty()
    );
    assert!(chord(&c, &ids, &[vec![1.]], 0.).is_err());
}
#[test]
fn gregorian_calendar_checks_century_leap_rules_and_duplicate_days() {
    let c = ChartConfig::default();
    assert_eq!(calendar(&c, 2000, &[]).unwrap().marks.len(), 366);
    assert_eq!(calendar(&c, 1900, &[]).unwrap().marks.len(), 365);
    let leap = CalendarDay {
        month: 2,
        day: 29,
        value: 1.,
    };
    assert!(calendar(&c, 1900, std::slice::from_ref(&leap)).is_err());
    let s = calendar(&c, 2000, std::slice::from_ref(&leap)).unwrap();
    assert_eq!(
        s.marks.iter().find(|m| m.id == "2000-02-29").unwrap().value,
        1.
    );
    assert!(calendar(&c, 2000, &[leap.clone(), leap]).is_err());
}
#[test]
fn bullet_uses_largest_range_and_rejects_nonfinite_values() {
    let c = ChartConfig::default();
    let mut d = BulletDatum {
        id: "sales".into(),
        ranges: vec![100., 50.],
        measures: vec![75.],
        markers: vec![80.],
    };
    let s = bullet(&c, std::slice::from_ref(&d)).unwrap();
    assert_eq!(s.marks.len(), 4);
    let Geometry::Rect { width, .. } = s.marks[0].geometry else {
        panic!()
    };
    assert_eq!(width, c.inner_width());
    d.measures[0] = f64::NAN;
    assert!(bullet(&c, &[d]).is_err());
}

#[test]
fn bullet_fractional_domain_and_all_measures_stay_in_their_row() {
    let c = ChartConfig::default();
    let d = BulletDatum {
        id: "fraction".into(),
        ranges: vec![0.5],
        measures: vec![0.2; 32],
        markers: vec![],
    };
    let s = bullet(&c, &[d]).unwrap();
    let Geometry::Rect { width, .. } = s.marks[0].geometry else {
        panic!()
    };
    assert_eq!(width, c.inner_width());
    for m in s.marks.iter().filter(|m| m.id.starts_with("measure:")) {
        let Geometry::Rect { y, height, .. } = m.geometry else {
            panic!()
        };
        assert!(y >= c.margin.top && y + height <= c.height - c.margin.bottom);
    }
}
#[test]
fn network_zero_iterations_still_fits_seed_nodes() {
    let c = ChartConfig {
        width: 140.,
        height: 100.,
        margin: Margin {
            top: 0.,
            bottom: 0.,
            left: 0.,
            right: 0.,
        },
        ..Default::default()
    };
    let nodes: Vec<_> = (0..512).map(|i| NetworkNode::new(i.to_string())).collect();
    let s = network(
        &c,
        &nodes,
        &[],
        &NetworkOptions {
            iterations: 0,
            ..Default::default()
        },
    )
    .unwrap();
    for m in s.marks {
        let Geometry::Circle { x, y, radius } = m.geometry else {
            panic!()
        };
        assert!(
            x - radius >= 0. && x + radius <= c.width && y - radius >= 0. && y + radius <= c.height
        );
    }
}
#[test]
fn geo_rejects_antimeridian_and_overflowing_color_domain() {
    let c = ChartConfig::default();
    let mut f = GeoFeature {
        id: "a".into(),
        label: "a".into(),
        value: Some(0.),
        polygons: vec![vec![vec![
            [170., -10.],
            [-170., -10.],
            [-170., 10.],
            [170., 10.],
        ]]],
    };
    assert!(choropleth(&c, &[f.clone()], Projection::Equirectangular).is_err());
    f.polygons = vec![vec![vec![[0., 0.], [1., 0.], [1., 1.]]]];
    f.value = Some(-f64::MAX);
    let mut b = f.clone();
    b.id = "b".into();
    b.value = Some(f64::MAX);
    assert!(choropleth(&c, &[f, b], Projection::Equirectangular).is_err());
}
#[test]
fn chord_accepts_subnormal_weights_without_angle_overflow() {
    let c = ChartConfig::default();
    let s = chord(&c, &["a".into()], &[vec![f64::from_bits(1)]], 0.).unwrap();
    s.validate().unwrap();
    assert_eq!(s.marks.len(), 2);
}
