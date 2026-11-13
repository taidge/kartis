use glory::render::{GraphicStroke, PathCommand, StrokeCap};
use kartis::*;
fn config() -> ChartConfig {
    ChartConfig {
        width: 200.,
        height: 200.,
        margin: Margin {
            top: 10.,
            right: 10.,
            bottom: 10.,
            left: 10.,
        },
        ..Default::default()
    }
}
fn layout() -> LineOptions {
    let mut options = LineOptions {
        x_domain: Some([0., 1.]),
        y_domain: Some([0., 1.]),
        ..Default::default()
    };
    options.axes.x.visible = false;
    options.axes.y.visible = false;
    options.axes.x.grid = false;
    options.axes.y.grid = false;
    options
}
fn rows() -> Vec<Series> {
    vec![
        Series::new(
            "a:point-0",
            vec![Point::new(0.2, 0.4), Point::gap(0.5), Point::new(0.8, 0.7)],
        ),
        Series::new("b", vec![Point::new(0.2, 0.8), Point::new(0.8, 0.2)]),
    ]
}
#[test]
fn default_paint_geometry_matches_existing_pipeline_and_point_ids_survive_reorder_and_gap_connection()
 {
    let data = rows();
    let old = line(&config(), &data, &layout()).unwrap();
    let styled = line_with_style(&config(), &data, &layout(), &Default::default()).unwrap();
    assert_eq!(old.marks.len(), styled.marks.len());
    for (a, b) in old.marks.iter().zip(&styled.marks) {
        assert_eq!(a.geometry, b.geometry);
        assert_eq!(a.fill, b.fill);
        assert_eq!(a.stroke, b.stroke);
        assert_eq!(a.opacity, b.opacity);
        assert_eq!(a.series, b.series);
    }
    let mut reverse = data.clone();
    reverse.reverse();
    let reordered = line_with_style(&config(), &reverse, &layout(), &Default::default()).unwrap();
    for point in styled
        .marks
        .iter()
        .filter(|m| m.id.starts_with("line-point:"))
    {
        assert_eq!(
            point.geometry,
            reordered
                .marks
                .iter()
                .find(|m| m.id == point.id)
                .unwrap()
                .geometry
        );
    }
    let connected = line_with_style(
        &config(),
        &data,
        &layout(),
        &LineStyleOptions {
            connect_missing: true,
            ..Default::default()
        },
    )
    .unwrap();
    let point = styled
        .marks
        .iter()
        .find(|m| m.series == data[0].id && m.id.ends_with(":2"))
        .unwrap();
    assert_eq!(
        point.geometry,
        connected
            .marks
            .iter()
            .find(|m| m.id == point.id)
            .unwrap()
            .geometry
    );
    let gap_strokes = styled
        .marks
        .iter()
        .filter(|m| m.series == data[0].id && m.id.starts_with("line-stroke:"))
        .count();
    let joined_strokes = connected
        .marks
        .iter()
        .filter(|m| m.series == data[0].id && m.id.starts_with("line-stroke:"))
        .count();
    assert_eq!((gap_strokes, joined_strokes), (2, 1));
}
#[test]
fn stroke_points_and_area_have_independent_real_paint_and_gradient() {
    let mut options = layout();
    options.area = true;
    let style = LineStyleOptions {
        series: LineSeriesStyle {
            color: Some(Color(230, 30, 40, 255)),
            width: 5.,
            line_opacity: 0.7,
            dash: Some(StrokeDash {
                segments: vec![8., 4.],
                offset: 2.,
            }),
            point_radius: 7.,
            point_color: Some(Color(20, 150, 70, 255)),
            point_border: Some(GraphicStroke {
                color: Color(10, 20, 30, 255),
                width: 2.,
                cap: StrokeCap::Round,
            }),
            area_opacity: 0.18,
            area_paint: Some(FillPaint::Linear {
                start: [10., 10.],
                end: [10., 190.],
                stops: vec![
                    GradientStop::new(0., Color(20, 100, 180, 255)),
                    GradientStop::new(1., Color(230, 240, 250, 255)),
                ],
            }),
            ..Default::default()
        },
        connect_missing: true,
        ..Default::default()
    };
    let scene = line_with_style(&config(), &rows(), &options, &style).unwrap();
    let stroke = scene
        .marks
        .iter()
        .find(|m| m.id.starts_with("line-stroke:"))
        .unwrap();
    assert_eq!(stroke.stroke.unwrap().width, 5.);
    assert_eq!(stroke.opacity, 0.7);
    assert_eq!(stroke.stroke_dash, style.series.dash);
    let area = scene
        .marks
        .iter()
        .find(|m| m.id.starts_with("line-area:"))
        .unwrap();
    assert_eq!(area.opacity, 0.18);
    assert_eq!(area.fill_paint, style.series.area_paint);
    assert!(area.stroke.is_none());
    let point = scene
        .marks
        .iter()
        .find(|m| m.id.starts_with("line-point:"))
        .unwrap();
    assert!(matches!(
        point.geometry,
        Geometry::Circle { radius: 7., .. }
    ));
    assert_eq!(point.opacity, 1.);
    assert_eq!(point.stroke, style.series.point_border);
    assert_eq!(scene.legend[0].color, Color(230, 30, 40, 255));
    let svg = render::to_svg(&scene).unwrap();
    assert_eq!(svg.matches("<linearGradient ").count(), 1);
    assert!(svg.contains("stroke-dasharray=\"8 4\""));
}
#[test]
fn invisible_points_keep_explicit_mesh_sites_without_painted_or_geometric_hits() {
    let mut o = layout();
    o.points = false;
    let scene = line_with_style(
        &config(),
        &rows(),
        &o,
        &LineStyleOptions {
            interactive_sites: true,
            ..Default::default()
        },
    )
    .unwrap();
    let mesh = mesh::PointMesh::from_scene(&scene).unwrap();
    assert_eq!(mesh.points().len(), 4);
    let point = &mesh.points()[0];
    assert_eq!(mesh.nearest(point.x, point.y, 0.).unwrap().id, point.id);
    let mut only_points = scene.clone();
    only_points
        .marks
        .retain(|m| m.id.starts_with("line-point:"));
    for m in &only_points.marks {
        assert_eq!(m.fill, Some(Color(0, 0, 0, 0)));
        assert!(matches!(m.geometry, Geometry::Circle { radius: 0., .. }));
    }
    assert!(only_points.hit_test(point.x, point.y).is_none());
    let ordinary = line_with_style(&config(), &rows(), &o, &Default::default()).unwrap();
    assert!(
        mesh::PointMesh::from_scene(&ordinary)
            .unwrap()
            .points()
            .is_empty()
    );
}
#[test]
fn labels_are_formatted_owned_clipped_and_can_exist_without_visible_points() {
    let mut o = layout();
    o.points = false;
    let style = LineStyleOptions {
        series: LineSeriesStyle {
            point_labels: Some(LinePointLabels {
                text: LinePointLabelText::Coordinates {
                    x: ValueFormat::Fixed(1),
                    y: ValueFormat::Percent(0),
                },
                offset: [2., -8.],
                rotation: 20.,
                clip_to_plot: true,
                ..Default::default()
            }),
            ..Default::default()
        },
        ..Default::default()
    };
    let scene = line_with_style(&config(), &rows(), &o, &style).unwrap();
    assert_eq!(scene.labels.len(), 4);
    assert_eq!(scene.label_clips.len(), 4);
    assert_eq!(scene.labels[0].text, "0.2, 40%");
    assert_eq!(scene.labels[0].rotation, 20.);
    assert!(!scene.marks.iter().any(|m| m.id.starts_with("line-point:")));
    let hidden = scene.without_series(&[rows()[0].id.clone()].into_iter().collect());
    assert_eq!(hidden.labels.len(), 2);
    assert_eq!(hidden.label_clips.len(), 2);
    assert!(hidden.label_series.values().all(|id| id == "b"));
}
#[test]
fn series_overrides_do_not_inherit_global_fields_and_hidden_legend_identity_remains() {
    let mut o = layout();
    o.hidden.insert("b".into());
    let mut style = LineStyleOptions {
        series: LineSeriesStyle {
            width: 9.,
            point_radius: 8.,
            ..Default::default()
        },
        ..Default::default()
    };
    style.series_styles.insert(
        rows()[0].id.clone(),
        LineSeriesStyle {
            color: Some(Color(1, 2, 3, 255)),
            ..Default::default()
        },
    );
    let scene = line_with_style(&config(), &rows(), &o, &style).unwrap();
    assert_eq!(scene.legend.len(), 2);
    assert!(scene.marks.iter().all(|m| m.series != "b"));
    assert!(
        scene
            .marks
            .iter()
            .filter_map(|m| m.stroke)
            .all(|s| s.width == 2.)
    );
    assert!(
        scene
            .marks
            .iter()
            .filter_map(|m| if let Geometry::Circle { radius, .. } = m.geometry {
                Some(radius)
            } else {
                None
            })
            .all(|r| r == 3.5)
    );
}
#[test]
fn rejected_styles_and_invalid_original_gap_inputs_do_not_change_callers() {
    let mut data = rows();
    let original = data.clone();
    for width in [f64::NAN, -1., 257.] {
        let style = LineStyleOptions {
            series: LineSeriesStyle {
                width,
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(line_with_style(&config(), &data, &layout(), &style).is_err());
    }
    assert_eq!(data, original);
    data[0].data[1].x = f64::NAN;
    assert!(
        line_with_style(
            &config(),
            &data,
            &layout(),
            &LineStyleOptions {
                connect_missing: true,
                ..Default::default()
            }
        )
        .is_err()
    );
    let mut style = LineStyleOptions::default();
    style.series.point_labels = Some(LinePointLabels {
        offset: [f64::INFINITY, 0.],
        ..Default::default()
    });
    assert!(line_with_style(&config(), &original, &layout(), &style).is_err());
}
#[cfg(feature = "raster")]
#[test]
fn actual_cpu_area_alpha_does_not_fade_point_fill_or_border() {
    use render::raster::*;
    let mut o = layout();
    o.area = true;
    let data = [Series::new(
        "a",
        vec![
            Point::new(0.2, 0.4),
            Point::new(0.5, 0.4),
            Point::new(0.8, 0.4),
        ],
    )];
    let style = LineStyleOptions {
        series: LineSeriesStyle {
            color: Some(Color(255, 0, 0, 255)),
            line_opacity: 0.4,
            point_radius: 7.,
            point_color: Some(Color(0, 150, 60, 255)),
            point_border: Some(GraphicStroke {
                color: Color(0, 0, 0, 255),
                width: 4.,
                cap: StrokeCap::Round,
            }),
            area_opacity: 0.2,
            ..Default::default()
        },
        ..Default::default()
    };
    let scene = line_with_style(&config(), &data, &o, &style).unwrap();
    let image = render_raster(&scene, &FontRegistry::new(), &Default::default()).unwrap();
    let pixel = |x: usize, y: usize| {
        &image.rgba[(y * image.width as usize + x) * 4..(y * image.width as usize + x) * 4 + 4]
    };
    assert_eq!(pixel(100, 118), [0, 150, 60, 255]);
    assert_eq!(pixel(107, 118), [0, 0, 0, 255]);
    assert_eq!(pixel(100, 160), [255, 204, 204, 255]);
}
#[test]
fn curve_and_clipping_topology_are_not_reconstructed_by_the_paint_adapter() {
    for curve in [
        Curve::Linear,
        Curve::Step,
        Curve::Basis,
        Curve::Natural,
        Curve::Cardinal,
    ] {
        let mut o = layout();
        o.curve = curve;
        let data = [Series::new(
            "a",
            vec![
                Point::new(-0.5, 0.2),
                Point::new(0.3, 0.8),
                Point::new(0.6, 0.3),
                Point::new(1.2, 0.7),
            ],
        )];
        let old = line(&config(), &data, &o).unwrap();
        let new = line_with_style(&config(), &data, &o, &Default::default()).unwrap();
        let paths = |scene: Scene| {
            scene
                .marks
                .into_iter()
                .filter(|m| matches!(m.geometry, Geometry::Path(_)) && m.series == "a")
                .map(|m| m.geometry.commands())
                .collect::<Vec<Vec<PathCommand>>>()
        };
        assert_eq!(paths(old), paths(new));
    }
}
#[test]
fn connecting_gaps_preserves_original_automatic_x_domain_and_default_labels_use_axis_format() {
    let mut o = layout();
    o.x_domain = None;
    o.axes.y.format = ValueFormat::Percent(1);
    let data = [Series::new(
        "s",
        vec![
            Point::gap(-10.),
            Point::new(0., 0.4),
            Point::new(1., 0.7),
            Point::gap(10.),
        ],
    )];
    let a = line_with_style(&config(), &data, &o, &Default::default()).unwrap();
    let b = line_with_style(
        &config(),
        &data,
        &o,
        &LineStyleOptions {
            connect_missing: true,
            series: LineSeriesStyle {
                point_labels: Some(Default::default()),
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .unwrap();
    for point in a.marks.iter().filter(|m| m.id.starts_with("line-point:")) {
        assert_eq!(
            point.geometry,
            b.marks.iter().find(|m| m.id == point.id).unwrap().geometry
        );
    }
    assert_eq!(b.labels[0].text, "40.0%");
}
