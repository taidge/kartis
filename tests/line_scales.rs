use glory::render::PathCommand as C;
use kartis::{surface::*, *};
use std::collections::BTreeSet;
fn config() -> ChartConfig {
    ChartConfig {
        width: 420.5,
        height: 300.5,
        margin: Margin {
            left: 60.25,
            right: 20.25,
            top: 20.25,
            bottom: 40.25,
        },
        ..Default::default()
    }
}
fn options() -> LineOptions {
    LineOptions {
        x_domain: Some([0., 10.]),
        y_domain: Some([-10., 10.]),
        ..Default::default()
    }
}
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-8, "{a} != {b}");
}
fn paths(s: &Scene, prefix: &str) -> Vec<Vec<[f64; 2]>> {
    s.marks
        .iter()
        .filter(|m| m.id.starts_with(prefix))
        .map(|m| {
            m.geometry
                .commands()
                .into_iter()
                .filter_map(|c| match c {
                    C::Move(p) | C::Line(p) => Some([p.x, p.y]),
                    _ => None,
                })
                .collect()
        })
        .collect()
}
#[test]
fn line_axes_paths_sites_share_round_clamp_and_reverse_mapping() {
    let data = [Series::new(
        "a",
        vec![
            Point::new(-1., -20.),
            Point::new(2.5, 0.),
            Point::new(11., 20.),
        ],
    )];
    let scales = LineScaleOptions {
        x: ContinuousScaleOptions {
            reverse: true,
            round: true,
            clamp: true,
            ..Default::default()
        },
        y: ContinuousScaleOptions {
            round: true,
            clamp: true,
            ..Default::default()
        },
    };
    let result =
        line_layout_with_scale_options(&config(), &data, &options(), &Default::default(), &scales)
            .unwrap();
    assert_eq!(
        result
            .layout
            .sites
            .iter()
            .map(|s| s.position)
            .collect::<Vec<_>>(),
        vec![[400., 260.], [315., 140.], [60., 20.]]
    );
    assert_eq!(result.layout.sites[0].raw_y, -20.);
    assert_eq!(result.layout.sites[2].raw_x, 11.);
    let points = paths(&result.layout.scene, "line-stroke:").concat();
    for site in &result.layout.sites {
        assert!(
            points.contains(&site.position),
            "path bypassed the shared rounded map"
        );
    }
    for p in paths(&result.layout.scene, "axis-x-tick-").concat() {
        assert_eq!(p[0].fract(), 0.);
    }
    result
        .interaction_binding()
        .validate(&result.layout.scene)
        .unwrap();
}
#[test]
fn nice_records_source_and_uses_display_domain_for_real_geometry() {
    let data = [Series::new(
        "a",
        vec![Point::new(0.13, 2.), Point::new(0.87, 7.)],
    )];
    let mut o = options();
    o.x_domain = Some([0.13, 0.87]);
    let s = LineScaleOptions {
        x: ContinuousScaleOptions {
            nice: ScaleNice::Count(5),
            ..Default::default()
        },
        ..Default::default()
    };
    let r = line_layout_with_scale_options(&config(), &data, &o, &Default::default(), &s).unwrap();
    assert_eq!(r.x_scale.source_domain(), [0.13, 0.87]);
    assert_eq!(r.x_scale.display_domain(), [0., 1.]);
    near(r.layout.sites[0].position[0], 60.25 + 340. * 0.13);
    near(r.layout.sites[1].position[0], 60.25 + 340. * 0.87);
    let tick_x = paths(&r.layout.scene, "axis-x-tick-").concat();
    assert!(tick_x.iter().any(|p| (p[0] - 60.25).abs() < 1e-9));
}
#[test]
fn constant_and_descending_domains_keep_paths_points_and_y_areas() {
    let data = [Series::new(
        "a",
        vec![Point::new(0., 1.), Point::new(5., 5.), Point::new(10., 9.)],
    )];
    let mut o = options();
    o.x_domain = Some([3., 3.]);
    let r = line_layout_with_scale_options(
        &config(),
        &data,
        &o,
        &Default::default(),
        &Default::default(),
    )
    .unwrap();
    assert_eq!(r.layout.sites.len(), 3);
    assert!(r.layout.sites.iter().all(|s| s.position[0] == 230.25));
    assert!(paths(&r.layout.scene, "line-stroke:").concat().len() >= 3);
    o.x_domain = Some([10., 0.]);
    o.y_domain = Some([10., 0.]);
    o.area = true;
    let d = line_layout_with_scale_options(
        &config(),
        &data,
        &o,
        &Default::default(),
        &Default::default(),
    )
    .unwrap();
    near(d.layout.sites[0].position[0], 400.25);
    near(d.layout.sites[2].position[0], 60.25);
    assert!(!paths(&d.layout.scene, "line-area:").is_empty());
    d.layout.scene.validate().unwrap();
}
#[test]
fn configured_log_clamps_raw_zero_and_opposite_sign_before_transform() {
    let data = [Series::new(
        "a",
        vec![Point::new(-10., 2.), Point::new(0., 4.), Point::new(8., 6.)],
    )];
    let mut o = options();
    o.x_domain = Some([1., 8.]);
    let scale = LineScaleOptions {
        x: ContinuousScaleOptions {
            kind: ScaleKind::Log,
            log_base: 2.,
            clamp: true,
            ..Default::default()
        },
        ..Default::default()
    };
    let r =
        line_layout_with_scale_options(&config(), &data, &o, &Default::default(), &scale).unwrap();
    assert_eq!(r.layout.sites.len(), 3);
    near(r.layout.sites[0].position[0], 60.25);
    near(r.layout.sites[1].position[0], 60.25);
    near(r.layout.sites[2].position[0], 400.25);
    assert_eq!(r.x_scale.ticks(4), vec![1., 2., 4., 8.]);
    assert_eq!(
        r.x_scale.ticks(10),
        (0..15).map(|i| 1. + f64::from(i) * 0.5).collect::<Vec<_>>()
    );
}
fn point_options() -> PointLineOptions {
    PointLineOptions {
        line: LineOptions {
            y_domain: Some([0., 10.]),
            ..Default::default()
        },
        ..Default::default()
    }
}
#[test]
fn actual_point_categories_keep_number_string_identity_gaps_and_source_indices() {
    let data = [PointLineSeries::new(
        "a",
        vec![
            PointLineDatum::new(1., 2.),
            PointLineDatum::new("1", 4.),
            PointLineDatum::gap("empty"),
            PointLineDatum::new("last", 8.),
        ],
    )];
    let r = point_line_layout(&config(), &data, &point_options(), &Default::default()).unwrap();
    assert_eq!(r.x_scale.domain().len(), 4);
    assert_eq!(r.sites.len(), 3);
    assert!(r.sites[0].position[0] < r.sites[1].position[0]);
    assert_eq!(r.sites[2].source_index, 3);
    assert_eq!(r.sites[0].raw_x, CategoryKey::Number(1.));
    assert_eq!(r.sites[1].raw_x, CategoryKey::String("1".into()));
    near(r.sites[0].position[0], 60.25);
    near(r.sites[2].position[0], 400.25);
    assert!(r.scene.labels.iter().any(|s| s.text == "empty"));
    let ids = r
        .scene
        .marks
        .iter()
        .filter(|m| m.id.starts_with("axis-x-tick:"))
        .map(|m| &m.id)
        .collect::<BTreeSet<_>>();
    assert_eq!(ids.len(), 4);
    let binding = r.interaction_binding();
    assert!(binding.sites[2].label.contains("last"));
    binding.validate(&r.scene).unwrap();
}
#[test]
fn point_window_clips_full_lattice_without_reindexing_or_rescaling_y() {
    let data = [PointLineSeries::new(
        "s",
        vec![
            PointLineDatum::new("a", 2.),
            PointLineDatum::new("b", 4.),
            PointLineDatum::new("c", 6.),
        ],
    )];
    let mut o = point_options();
    o.window = Some([0.5, 1.5]);
    let r = point_line_layout(&config(), &data, &o, &Default::default()).unwrap();
    assert_eq!(r.sites.len(), 1);
    assert_eq!(r.sites[0].source_index, 1);
    assert_eq!(r.sites[0].raw_x, CategoryKey::String("b".into()));
    near(r.sites[0].position[0], 230.25);
    let p = paths(&r.scene, "line-stroke:").concat();
    assert!(
        p.iter()
            .any(|v| (v[0] - 60.25).abs() < 1e-8 && (v[1] - 188.25).abs() < 1e-8)
    );
    assert!(
        p.iter()
            .any(|v| (v[0] - 400.25).abs() < 1e-8 && (v[1] - 140.25).abs() < 1e-8)
    );
    let hidden = PointLineOptions {
        line: LineOptions {
            hidden: BTreeSet::from(["s".into()]),
            ..o.line.clone()
        },
        ..o.clone()
    };
    let h = point_line_layout(&config(), &data, &hidden, &Default::default()).unwrap();
    assert!(h.sites.is_empty());
    assert_eq!(h.scene.legend.len(), 1);
}
#[test]
fn ordinal_labels_retain_raw_keys_even_when_points_and_interaction_are_disabled() {
    let data = [PointLineSeries::new(
        "s",
        vec![PointLineDatum::new("beta", 4.)],
    )];
    let mut o = point_options();
    o.line.points = false;
    let style = LineStyleOptions {
        series: LineSeriesStyle {
            points: Some(false),
            point_labels: Some(LinePointLabels {
                text: LinePointLabelText::Coordinates {
                    x: ValueFormat::Auto,
                    y: ValueFormat::Auto,
                },
                ..Default::default()
            }),
            ..Default::default()
        },
        ..Default::default()
    };
    let r = point_line_layout(&config(), &data, &o, &style).unwrap();
    assert!(r.sites.is_empty());
    assert!(r.scene.labels.iter().any(|s| s.text == "beta, 4"));
    near(r.x_scale.map(&CategoryKey::from("beta")).unwrap(), 230.25);
}
#[test]
fn line_glyph_uses_real_path_anchor_and_keeps_scalar_business_values() {
    let data = [Series::new("s", vec![Point::new(5., 0.)])];
    let style = LineStyleOptions {
        series: LineSeriesStyle {
            point_glyph: Some(GlyphOptions {
                shape: GlyphShape::Filled(FilledGlyph::Triangle),
                size: 400.,
                rotation: 0.4,
            }),
            ..Default::default()
        },
        ..Default::default()
    };
    let r =
        line_layout_with_scale_options(&config(), &data, &options(), &style, &Default::default())
            .unwrap();
    let site = &r.layout.sites[0];
    let mark = r
        .layout
        .scene
        .marks
        .iter()
        .find(|m| m.id == site.mark_id)
        .unwrap();
    assert!(matches!(mark.geometry, Geometry::Path(_)));
    assert_eq!(r.layout.anchors.len(), 1);
    let mut model = SurfaceModel::new_with_interaction(
        r.layout.scene.clone(),
        SurfaceOptions {
            picking: Picking::Geometry,
            ..Default::default()
        },
        Some(r.interaction_binding()),
    )
    .unwrap();
    model
        .apply(SurfaceInput::Move {
            id: 1,
            point: (site.position[0], site.position[1]),
        })
        .unwrap();
    assert_eq!(model.hovered(), std::slice::from_ref(&site.mark_id));
    let active = model.active_interaction(model.hovered()).unwrap();
    assert_eq!(active.anchor, site.position);
    assert_eq!(active.rows[0].value, Some(0.));
}
