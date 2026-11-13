use kartis::*;
fn source() -> Vec<BulletDatum> {
    vec![
        BulletDatum {
            id: "A".into(),
            ranges: vec![10., -10., 0.],
            measures: vec![8., -4.],
            markers: vec![25.],
        },
        BulletDatum {
            id: "B".into(),
            ranges: vec![5., 30.],
            measures: vec![20.],
            markers: vec![0.],
        },
    ]
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} != {b}");
}
#[test]
fn shared_auto_domain_uses_all_rows_ranges_measures_and_markers() {
    let l = bullet_layout(&ChartConfig::default(), &source(), &Default::default()).unwrap();
    assert_eq!(l.shared_domain, Some([-10., 30.]));
    assert!(l.rows.iter().all(|r| r.domain == [-10., 30.]));
    let a = &l.rows[0];
    let ranges = a
        .parts
        .iter()
        .filter(|p| p.kind == BulletPartKind::Range)
        .collect::<Vec<_>>();
    assert_eq!(
        ranges.iter().map(|p| p.raw_value).collect::<Vec<_>>(),
        vec![-10., 0., 10., 30.]
    );
    assert_eq!(ranges[1].source_index, Some(2));
    assert_eq!(ranges[1].interval, [-10., 0.]);
    assert_eq!(ranges[3].source_index, None);
    assert_eq!(ranges[3].interval, [10., 30.]);
    let measures = a
        .parts
        .iter()
        .filter(|p| p.kind == BulletPartKind::Measure)
        .collect::<Vec<_>>();
    assert_eq!(measures[0].source_index, Some(1));
    assert_eq!(measures[0].interval, [-10., -4.]);
    assert_eq!(measures[1].interval, [-4., 8.]);
    let m = l
        .scene
        .marks
        .iter()
        .find(|m| m.id == measures[1].id)
        .unwrap();
    assert_eq!(m.value, 8.);
    assert!(m.label.contains("-4 → 8"));
    assert_eq!(m.series, "A");
}
#[test]
fn explicit_bounds_and_per_row_profile_crop_without_fake_marker_at_edges() {
    let c = ChartConfig::default();
    let d = source();
    let per = bullet_layout(
        &c,
        &d,
        &BulletLayoutOptions {
            domain_mode: BulletDomainMode::PerRow,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(per.shared_domain, None);
    assert_eq!(per.rows[0].domain, [-10., 25.]);
    assert_eq!(per.rows[1].domain, [0., 30.]);
    let l = bullet_layout(
        &c,
        &d,
        &BulletLayoutOptions {
            min: Some(-5.),
            max: Some(15.),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(l.shared_domain, Some([-5., 15.]));
    let p = l.rows[0]
        .parts
        .iter()
        .find(|p| p.kind == BulletPartKind::Measure && p.source_index == Some(1))
        .unwrap();
    assert_eq!(p.interval, [-5., -4.]);
    assert_eq!(p.visible_interval, [-5., -4.]);
    let marker = l.rows[0]
        .parts
        .iter()
        .find(|p| p.kind == BulletPartKind::Marker)
        .unwrap();
    assert_eq!(marker.raw_value, 25.);
    assert!(!l.scene.marks.iter().any(|m| m.id == marker.id));
    let last = l.rows[1]
        .parts
        .iter()
        .find(|p| p.kind == BulletPartKind::Measure)
        .unwrap();
    assert_eq!(last.raw_value, 20.);
    assert_eq!(last.visible_interval, [-5., 15.]);
}
#[test]
fn horizontal_vertical_reverse_real_coordinates_sizes_and_owned_titles() {
    let c = ChartConfig::default();
    let d = source();
    let mut o = BulletLayoutOptions {
        min: Some(-10.),
        max: Some(30.),
        measure_size: 0.25,
        marker_size: 0.75,
        title_rotation: 30.,
        ..Default::default()
    };
    let a = bullet_layout(&c, &d, &o).unwrap();
    o.reverse = true;
    let b = bullet_layout(&c, &d, &o).unwrap();
    for (p, q) in a.rows[0].parts.iter().zip(&b.rows[0].parts) {
        if let (
            Geometry::Rect {
                x, width, height, ..
            },
            Geometry::Rect {
                x: rx, width: rw, ..
            },
        ) = (&p.geometry, &q.geometry)
        {
            close(*width, *rw);
            close(*x + *rx + *width, 2. * c.margin.left + c.inner_width());
            if p.kind == BulletPartKind::Measure {
                close(*height, a.rows[0].track.height * 0.25);
            }
        }
    }
    o.orientation = StatisticsOrientation::Vertical;
    let v = bullet_layout(&c, &d, &o).unwrap();
    let p = v.rows[0]
        .parts
        .iter()
        .find(|p| p.kind == BulletPartKind::Measure)
        .unwrap();
    assert!(
        matches!(p.geometry,Geometry::Rect{width,..} if (width-v.rows[0].track.width*0.25).abs()<1e-9)
    );
    assert!(v.scene.labels.iter().enumerate().any(|(i, l)| l.text == "A"
        && l.rotation == 30.
        && v.scene.label_series.get(&i) == Some(&"A".into())));
}
#[test]
fn hidden_rows_relayout_and_source_reorder_keeps_identity() {
    let c = ChartConfig::default();
    let d = source();
    let mut o = BulletLayoutOptions {
        order: Some(vec!["B".into(), "A".into()]),
        ..Default::default()
    };
    let l = bullet_layout(&c, &d, &o).unwrap();
    assert_eq!(l.rows[0].id, "B");
    let old = l.rows[0].parts.clone();
    let filtered = l
        .scene
        .without_series(&std::collections::BTreeSet::from(["A".into()]));
    assert!(
        !filtered
            .marks
            .iter()
            .any(|m| m.id.starts_with("bullet-axis:1:A"))
    );
    assert!(
        l.scene
            .marks
            .iter()
            .any(|m| m.id.starts_with("bullet-axis:1:A") && m.series == "A" && m.label.is_empty())
    );
    o.hidden.insert("A".into());
    let h = bullet_layout(&c, &d, &o).unwrap();
    assert_eq!(h.rows.len(), 1);
    close(h.rows[0].track.height, c.inner_height());
    assert_eq!(
        h.rows[0].parts.iter().map(|p| &p.id).collect::<Vec<_>>(),
        old.iter().map(|p| &p.id).collect::<Vec<_>>()
    );
    assert!(!h.scene.label_series.values().any(|s| s == "A"));
    let mut reordered = d.clone();
    reordered.reverse();
    assert_eq!(h, bullet_layout(&c, &reordered, &o).unwrap());
    o.hidden.insert("B".into());
    assert!(bullet_layout(&c, &d, &o).unwrap().rows.is_empty());
}
#[test]
fn empty_zero_negative_invalid_limits_and_budgets_are_explicit_and_atomic() {
    let c = ChartConfig::default();
    let d = vec![
        BulletDatum {
            id: "empty".into(),
            ranges: vec![],
            measures: vec![],
            markers: vec![],
        },
        BulletDatum {
            id: "zero".into(),
            ranges: vec![0.],
            measures: vec![0.],
            markers: vec![0.],
        },
    ];
    let l = bullet_layout(&c, &d, &Default::default()).unwrap();
    assert_eq!(l.shared_domain, Some([0., 0.]));
    assert!(
        l.rows[1]
            .parts
            .iter()
            .any(|p| p.kind == BulletPartKind::Marker && p.raw_value == 0.)
    );
    let saved = l.scene.clone();
    assert!(
        bullet_layout(
            &c,
            &d,
            &BulletLayoutOptions {
                min: Some(2.),
                max: Some(1.),
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        bullet_layout(
            &c,
            &d,
            &BulletLayoutOptions {
                spacing: 1000.,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        bullet_layout(
            &c,
            &d,
            &BulletLayoutOptions {
                measure_size: f64::NAN,
                ..Default::default()
            }
        )
        .is_err()
    );
    let mut bad = d.clone();
    bad[0].ranges.push(f64::INFINITY);
    assert!(bullet_layout(&c, &bad, &Default::default()).is_err());
    let mut bad = d.clone();
    bad[0].id = "zero".into();
    assert!(bullet_layout(&c, &bad, &Default::default()).is_err());
    let mut bad = d.clone();
    bad[0].ranges = vec![0.; 65];
    assert!(bullet_layout(&c, &bad, &Default::default()).is_err());
    assert_eq!(l.scene, saved);
    let huge = [BulletDatum {
        id: "huge".into(),
        ranges: vec![f64::MAX],
        measures: vec![f64::MAX],
        markers: vec![f64::MAX],
    }];
    assert!(bullet_layout(&c, &huge, &Default::default()).is_ok());
}
#[cfg(feature = "raster")]
#[test]
fn cpu_adjacent_segments_measure_size_marker_and_reverse_pixels() {
    use kartis::render::raster::*;
    let c = ChartConfig {
        width: 200.,
        height: 120.,
        margin: Margin {
            left: 20.,
            right: 20.,
            top: 20.,
            bottom: 20.,
        },
        ..Default::default()
    };
    let d = [BulletDatum {
        id: "a".into(),
        ranges: vec![5., 10.],
        measures: vec![3.],
        markers: vec![7.],
    }];
    let mut o = BulletLayoutOptions {
        min: Some(0.),
        max: Some(10.),
        range_colors: vec![Color(200, 200, 200, 255), Color(100, 100, 100, 255)],
        measure_colors: vec![Color(0, 0, 255, 255)],
        marker_colors: vec![Color(255, 0, 0, 255)],
        axis: AxisOptions {
            visible: false,
            ..Default::default()
        },
        ..Default::default()
    };
    for reverse in [false, true] {
        o.reverse = reverse;
        let mut s = bullet_with_options(&c, &d, &o).unwrap();
        s.labels.clear();
        s.label_series.clear();
        s.legend.clear();
        let im = render_raster(&s, &FontRegistry::default(), &Default::default()).unwrap();
        let pixel = |x: usize, y: usize| {
            let x = if reverse { 199 - x } else { x };
            let i = (y * im.width as usize + x) * 4;
            &im.rgba[i..i + 4]
        };
        assert_eq!(pixel(40, 30), &[200, 200, 200, 255]);
        assert_eq!(pixel(140, 30), &[100, 100, 100, 255]);
        assert_eq!(pixel(40, 60), &[0, 0, 255, 255]);
        assert_eq!(pixel(132, 60), &[255, 0, 0, 255]);
    }
}
