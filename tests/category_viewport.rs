use kartis::scales::BandScale;
use kartis::*;
#[test]
fn value_view_excluding_zero_crops_after_stacking_and_removes_invisible_hits() {
    let c = config();
    let categories = vec!["A".into(), "B".into()];
    let series = [
        BarSeries::new("base", vec![Some(20.), Some(-20.)]),
        BarSeries::new("tip", vec![Some(10.), Some(-10.)]),
    ];
    for horizontal in [false, true] {
        for domain in [[25., 40.], [-40., -25.]] {
            let s = bar(
                &c,
                &categories,
                &series,
                &BarOptions {
                    layout: BarLayout::Stacked,
                    horizontal,
                    value_domain: Some(domain),
                    ..Default::default()
                },
            )
            .unwrap();
            let marks = s
                .marks
                .iter()
                .filter(|m| !m.series.is_empty())
                .collect::<Vec<_>>();
            assert_eq!(marks.len(), 1);
            assert_eq!(marks[0].series, "tip");
            assert_eq!(marks[0].value, domain[0].signum() * 10.);
            if let Geometry::Rect { width, height, .. } = marks[0].geometry {
                let extent = if horizontal {
                    c.inner_width()
                } else {
                    c.inner_height()
                };
                let visible = if horizontal { width } else { height };
                assert!((visible / extent - 1. / 3.).abs() < 1e-10);
            }
        }
    }
}
fn categories() -> Vec<String> {
    (0..8).map(|i| format!("Category {i}")).collect()
}
fn config() -> ChartConfig {
    ChartConfig {
        width: 600.,
        height: 360.,
        ..Default::default()
    }
}
fn rect() -> ClipRect {
    ClipRect::plot(&config())
}
fn data() -> Vec<BarSeries> {
    vec![
        BarSeries::new("positive", vec![Some(10.); 8]),
        BarSeries::new("negative", vec![Some(-4.); 8]),
    ]
}
#[test]
fn fractional_bands_preserve_original_ids_and_inverse_in_both_directions() {
    for range in [[20., 420.], [420., 20.]] {
        let original = BandScale::new(categories(), range, 0.2).unwrap();
        let zoom = original.clone().windowed([0.25, 0.75]).unwrap();
        for i in 0..8 {
            let direction = (range[1] - range[0]).signum();
            let x = original.at(i).unwrap();
            let expected = if direction > 0. {
                range[0] + ((x - range[0]) - 400. * 0.25) * 2.
            } else {
                range[0] - ((range[0] - x - original.bandwidth) - 400. * 0.25) * 2. - zoom.bandwidth
            };
            assert!((zoom.at(i).unwrap() - expected).abs() < 1e-10);
            let center = zoom.at(i).unwrap() + zoom.bandwidth / 2.;
            if (20. ..=420.).contains(&center) {
                assert_eq!(zoom.invert(center), Some(categories()[i].as_str()));
            }
        }
        assert_eq!(zoom.invert(19.), None);
        assert_eq!(zoom.invert(421.), None);
        let reset = zoom.windowed([0., 1.]).unwrap();
        for i in 0..8 {
            assert!((reset.at(i).unwrap() - original.at(i).unwrap()).abs() < 1e-10);
        }
    }
}
#[test]
fn category_navigation_brushes_fractional_windows_and_resize_is_exact() {
    let mut v = CategoryViewport::new(categories(), rect(), ViewportLimits::default()).unwrap();
    assert_eq!(v.controller().limits().max_zoom, 8.);
    let p = rect();
    let w = v
        .apply(ViewportRequest::Brush {
            bounds: Brush::from_points((p.width / 4., 0.), (p.width * 0.75, p.height)).unwrap(),
            axes: ViewportSelection::X,
        })
        .unwrap();
    assert_eq!(w.domain, [2., 6.]);
    assert_eq!(w.normalized, [0.25, 0.75]);
    assert_eq!(w.categories, categories()[2..6]);
    let old = v.controller().domains();
    v.resize(ClipRect {
        x: 30.,
        y: 20.,
        width: 900.,
        height: 600.,
    })
    .unwrap();
    assert_eq!(old, v.controller().domains());
    let before = v.clone();
    assert!(
        v.apply(ViewportRequest::Pan {
            delta: (f64::NAN, 0.),
            axes: ViewportSelection::X
        })
        .is_err()
    );
    assert_eq!(before, v);
    assert_eq!(
        v.apply(ViewportRequest::Reset).unwrap().categories,
        categories()
    );
    assert!(
        CategoryViewport::new(
            categories(),
            rect(),
            ViewportLimits {
                min_zoom: 20.,
                max_zoom: 10.,
                bounded: true
            }
        )
        .is_err()
    );
    assert!(CategoryViewport::resolve(&["a".into(), "a".into()], [0., 1.]).is_err());
}
#[test]
fn zoomed_bars_crop_category_edges_without_changing_stacks_colors_or_ids() {
    for horizontal in [false, true] {
        for layout in [BarLayout::Grouped, BarLayout::Stacked, BarLayout::Percent] {
            let c = config();
            let base = bar(
                &c,
                &categories(),
                &data(),
                &BarOptions {
                    horizontal,
                    layout,
                    ..Default::default()
                },
            )
            .unwrap();
            let zoom = bar(
                &c,
                &categories(),
                &data(),
                &BarOptions {
                    horizontal,
                    layout,
                    category_window: Some([0.28, 0.72]),
                    category_axis: CategoryAxisOptions {
                        tick_count: 20,
                        ..Default::default()
                    },
                    ..Default::default()
                },
            )
            .unwrap();
            let data_marks = zoom
                .marks
                .iter()
                .filter(|m| !m.series.is_empty())
                .collect::<Vec<_>>();
            assert!(!data_marks.is_empty());
            assert!(data_marks.len() < base.marks.iter().filter(|m| !m.series.is_empty()).count());
            for m in data_marks {
                let old = base.marks.iter().find(|old| old.id == m.id).unwrap();
                assert_eq!(m.fill, old.fill);
                assert_eq!(m.value, old.value);
                if let Geometry::Rect {
                    x,
                    y,
                    width,
                    height,
                } = m.geometry
                {
                    let (s, e, lo, hi) = if horizontal {
                        (y, y + height, c.margin.top, c.height - c.margin.bottom)
                    } else {
                        (x, x + width, c.margin.left, c.width - c.margin.right)
                    };
                    assert!(s >= lo - 1e-9 && e <= hi + 1e-9);
                }
            }
            let ticks = zoom
                .labels
                .iter()
                .filter(|l| l.text.starts_with("Category "))
                .collect::<Vec<_>>();
            assert!(ticks.len() >= 3);
            zoom.validate().unwrap();
        }
    }
}
#[test]
fn categorical_combo_window_clips_real_paths_and_preserves_point_identity() {
    let values = categories();
    let data = [
        ComboSeries::new(
            "bars",
            ComboKind::Bar,
            values
                .iter()
                .map(|v| ComboPoint::new(v.as_str(), 10.))
                .collect(),
        ),
        ComboSeries::new(
            "line",
            ComboKind::Line,
            values
                .iter()
                .enumerate()
                .map(|(i, v)| ComboPoint::new(v.as_str(), i as f64 + 3.))
                .collect(),
        ),
    ];
    let options = ComboOptions {
        x: ComboXAxis::Categories {
            values,
            padding: 0.1,
        },
        category_window: Some([0.25, 0.75]),
        ..Default::default()
    };
    let scene = combo(&config(), &data, &options).unwrap();
    scene.validate().unwrap();
    assert!(
        scene
            .marks
            .iter()
            .any(|m| m.series == "line" && matches!(m.geometry, Geometry::Path(_)))
    );
    assert_eq!(
        scene
            .marks
            .iter()
            .filter(|m| m.series == "line" && matches!(m.geometry, Geometry::Circle { .. }))
            .count(),
        4
    );
    assert!(
        combo(
            &config(),
            &[],
            &ComboOptions {
                category_window: Some([0., 1.]),
                ..Default::default()
            }
        )
        .is_err()
    );
}
#[test]
fn independent_transform_roundtrips_and_invalid_proposals_are_atomic() {
    let mut v = DomainViewport::new(
        ViewportAxes {
            x: ViewportAxis::new([0., 8.], ScaleKind::Linear),
            left: Some(ViewportAxis::new([0., 100.], ScaleKind::Linear)),
            right: None,
        },
        rect(),
        ViewportLimits::default(),
    )
    .unwrap();
    let p = rect();
    let t = ViewportTransform {
        scale_x: 4.,
        scale_y: 2.,
        translate_x: -p.width,
        translate_y: -p.height / 2.,
    };
    let d = v
        .apply(ViewportRequest::TransformXY {
            transform: t,
            axes: ViewportSelection::Both,
        })
        .unwrap();
    assert_eq!(d.x, [2., 4.]);
    assert_eq!(d.left, Some([25., 75.]));
    assert_eq!(v.transform_xy(), t);
    assert!(v.transform_for(ViewportSelection::Both).is_err());
    let old = v.clone();
    assert!(
        v.apply(ViewportRequest::TransformXY {
            transform: ViewportTransform {
                scale_y: f64::NAN,
                ..t
            },
            axes: ViewportSelection::Both
        })
        .is_err()
    );
    assert_eq!(old, v);
    let d2 = v
        .apply(ViewportRequest::TransformXY {
            transform: t,
            axes: ViewportSelection::Both,
        })
        .unwrap();
    assert_eq!(d, d2);
}
