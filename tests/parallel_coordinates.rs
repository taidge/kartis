use std::collections::{BTreeMap, BTreeSet};

use glory::render::{GraphicPoint as P, PathCommand as C};
use kartis::*;
fn config() -> ChartConfig {
    ChartConfig {
        width: 600.,
        height: 360.,
        margin: Margin {
            left: 60.,
            right: 60.,
            top: 30.,
            bottom: 50.,
        },
        ..Default::default()
    }
}
fn dimensions() -> Vec<ParallelDimension> {
    ["cost", "speed", "quality"]
        .into_iter()
        .map(|id| {
            let mut d = ParallelDimension::new(id);
            d.domain = Some([0., 100.]);
            d
        })
        .collect()
}
fn rows() -> Vec<ParallelDatum> {
    vec![ParallelDatum::new(
        "row",
        vec![Some(25.), Some(50.), Some(75.)],
    )]
}
fn path<'a>(scene: &'a Scene, row: &str) -> &'a [C] {
    let Geometry::Path(path) = &scene
        .marks
        .iter()
        .find(|m| m.id == format!("parallel:{}:{row}", row.len()))
        .unwrap()
        .geometry
    else {
        panic!()
    };
    path
}
fn points(scene: &Scene, row: &str) -> Vec<[f64; 2]> {
    path(scene, row)
        .iter()
        .filter_map(|c| match c {
            C::Move(p) | C::Line(p) => Some([p.x, p.y]),
            _ => None,
        })
        .collect()
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-8, "{a} != {b}");
}

#[test]
fn independent_linear_log_symlog_reverse_scales_project_real_variable_values() {
    let mut dims = dimensions();
    dims[1].scale = ScaleKind::Log;
    dims[1].domain = Some([1., 1000.]);
    dims[2].scale = ScaleKind::SymLog { constant: 1. };
    dims[2].domain = Some([-100., 100.]);
    dims[2].reverse = true;
    let scene = parallel_coordinates_with_options(
        &config(),
        &dims,
        &[ParallelDatum::new(
            "r",
            vec![Some(25.), Some(10.), Some(10.)],
        )],
        &Default::default(),
    )
    .unwrap();
    let p = points(&scene, "r");
    assert_eq!(p.len(), 3);
    close(p[0][0], 60.);
    close(p[0][1], 240.);
    close(p[1][0], 300.);
    close(p[1][1], 310. - 280. / 3.);
    close(p[2][0], 540.);
    close(p[2][1], 170. + 140. * 11_f64.ln() / 101_f64.ln());
    assert!(
        scene
            .marks
            .iter()
            .filter(|m| m.id.starts_with("parallel:"))
            .all(|m| m.clip == Some(ClipRect::plot(&config())))
    );
}

#[test]
fn vertical_orientation_orders_axes_bottom_to_top_and_reverses_value_axis() {
    let mut dims = dimensions();
    dims[1].reverse = true;
    let scene = parallel_coordinates_with_options(
        &config(),
        &dims,
        &rows(),
        &ParallelCoordinatesOptions {
            layout: ParallelLayout::Vertical,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        points(&scene, "row"),
        vec![[180., 310.], [300., 170.], [420., 30.]]
    );
    let first = scene
        .marks
        .iter()
        .find(|m| m.id == "parallel-axis:4:cost")
        .unwrap();
    let Geometry::Path(commands) = &first.geometry else {
        panic!()
    };
    assert!(matches!(commands[0],C::Move(p) if p.x==60.&&p.y==310.));
    assert!(matches!(commands[1],C::Line(p) if p.x==540.&&p.y==310.));
}

#[test]
fn nullable_gap_connect_skip_error_policies_are_distinct_and_legacy_record_converts() {
    let legacy = ParallelRecord {
        id: "r".into(),
        values: vec![Some(20.), None, Some(80.)],
    };
    let data = vec![ParallelDatum::from(&legacy)];
    assert_eq!(data[0].values, legacy.values);
    let gap =
        parallel_coordinates_with_options(&config(), &dimensions(), &data, &Default::default())
            .unwrap();
    assert_eq!(
        path(&gap, "r")
            .iter()
            .filter(|c| matches!(c, C::Move(_)))
            .count(),
        2
    );
    let mut last = None;
    for command in path(&gap, "r") {
        match command {
            C::Move(p) => last = Some(p),
            C::Line(p) => assert_eq!(
                Some(p),
                last,
                "a gap must not acquire a cross-variable line"
            ),
            C::Close => {}
            _ => panic!("unexpected singleton geometry"),
        }
    }
    let connected = parallel_coordinates_with_options(
        &config(),
        &dimensions(),
        &data,
        &ParallelCoordinatesOptions {
            missing: ParallelMissing::Connect,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(matches!(path(&connected, "r"), [C::Move(_), C::Line(_)]));
    let skipped = parallel_coordinates_with_options(
        &config(),
        &dimensions(),
        &data,
        &ParallelCoordinatesOptions {
            missing: ParallelMissing::SkipRecord,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(!skipped.marks.iter().any(|m| m.id.starts_with("parallel:")));
    assert!(
        parallel_coordinates_with_options(
            &config(),
            &dimensions(),
            &data,
            &ParallelCoordinatesOptions {
                missing: ParallelMissing::Error,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        parallel_coordinates(
            &config(),
            &["cost".into(), "speed".into(), "quality".into()],
            &[legacy]
        )
        .is_ok()
    );
}

#[test]
fn grouping_palette_hidden_domains_row_labels_and_source_reordering_keep_identity() {
    let mut dims = dimensions();
    for d in &mut dims {
        d.domain = None;
    }
    let mut a = ParallelDatum::new("a", vec![Some(25.); 3]);
    a.group = Some("group".into());
    let mut b = ParallelDatum::new("b", vec![Some(50.); 3]);
    b.group = Some("group".into());
    let outlier = ParallelDatum::new("outlier", vec![Some(100.); 3]);
    let records = vec![a, b, outlier];
    let options = ParallelCoordinatesOptions {
        legend: true,
        row_labels: true,
        clip_row_labels: true,
        legend_labels: BTreeMap::from([("group".into(), "Group display".into())]),
        ..Default::default()
    };
    let scene = parallel_coordinates_with_options(&config(), &dims, &records, &options).unwrap();
    let mut reversed = records.clone();
    reversed.reverse();
    assert_eq!(
        scene,
        parallel_coordinates_with_options(&config(), &dims, &reversed, &options).unwrap()
    );
    assert_eq!(scene.legend.len(), 2);
    assert_eq!(scene.legend[0].label, "Group display");
    let hidden = parallel_coordinates_with_options(
        &config(),
        &dims,
        &records,
        &ParallelCoordinatesOptions {
            hidden: BTreeSet::from(["outlier".into()]),
            ..options
        },
    )
    .unwrap();
    assert_eq!(scene.legend, hidden.legend);
    close(points(&scene, "b")[0][1], 310. - 280. / 3.);
    close(points(&hidden, "b")[0][1], 30.);
    let original = scene.marks.iter().find(|m| m.id == "parallel:1:b").unwrap();
    let current = hidden
        .marks
        .iter()
        .find(|m| m.id == "parallel:1:b")
        .unwrap();
    assert_eq!(original.stroke, current.stroke);
    let only_axes = scene.without_series(&BTreeSet::from(["group".into(), "outlier".into()]));
    only_axes.validate().unwrap();
    assert!(only_axes.label_series.is_empty() && only_axes.label_clips.is_empty());
    assert!(only_axes.labels.iter().any(|l| l.text == "cost"));
}

#[test]
fn curves_are_shared_exact_geometry_and_styles_tooltips_and_ownership_are_real() {
    let mut dims = dimensions();
    dims[0].value_format = Some(ValueFormat::Affix {
        prefix: "$".into(),
        suffix: String::new(),
        precision: 1,
    });
    let options = ParallelCoordinatesOptions {
        curve: Curve::Natural,
        line: ParallelLineStyle {
            width: 5.,
            opacity: 0.8,
            color: Some(Color(200, 30, 60, 255)),
            dash: Some(StrokeDash {
                segments: vec![8., 4.],
                offset: 2.,
            }),
            ..Default::default()
        },
        row_labels: true,
        row_label_style: ParallelTextStyle {
            color: Some(Color(10, 40, 160, 255)),
            size: Some(19.),
        },
        row_label_offset: [-40., -20.],
        ..Default::default()
    };
    let scene = parallel_coordinates_with_options(&config(), &dims, &rows(), &options).unwrap();
    let mark = scene
        .marks
        .iter()
        .find(|m| m.id == "parallel:3:row")
        .unwrap();
    assert_eq!(mark.value, 25.);
    assert_eq!(mark.opacity, 0.8);
    assert_eq!(mark.stroke.unwrap().width, 5.);
    assert_eq!(mark.stroke.unwrap().color, Color(200, 30, 60, 255));
    assert_eq!(mark.stroke_dash, options.line.dash);
    assert!(
        path(&scene, "row")
            .iter()
            .any(|c| matches!(c, C::Cubic { .. }))
    );
    assert!(mark.label.contains("cost: $25.0") && mark.label.contains("quality: 75"));
    let slot = *scene
        .label_series
        .iter()
        .find(|(_, id)| *id == "row")
        .unwrap()
        .0;
    let label = &scene.labels[slot];
    assert_eq!((label.x, label.y, label.size), (500., 80., 19.));
    assert_eq!(label.color, Color(10, 40, 160, 255));
    let svg = render::to_svg(&scene).unwrap();
    assert!(
        svg.contains("stroke-width=\"5\"")
            && svg.contains("stroke-dasharray=\"8 4\"")
            && svg.contains("$25.0")
    );
}

#[test]
fn dimension_axes_ticks_before_after_grid_and_caption_styles_are_projected_in_place() {
    let mut dims = dimensions();
    for d in &mut dims {
        d.axis.tick_values = Some(vec![50.]);
        d.axis.tick_size = 7.;
        d.axis.tick_padding = 3.;
        d.axis.format = ValueFormat::Percent(0);
        d.axis.grid = true;
        d.style.line_color = Some(Color(10, 30, 80, 255));
        d.style.line_width = 3.;
        d.style.ticks = ParallelTextStyle {
            color: Some(Color(90, 40, 20, 255)),
            size: Some(18.),
        };
        d.style.caption = ParallelTextStyle {
            color: Some(Color(20, 90, 50, 255)),
            size: Some(20.),
        };
        d.style.caption_offset = 31.;
    }
    dims[1].ticks_position = ParallelTicksPosition::Before;
    dims[1].axis.legend = Some("Speed caption".into());
    dims[1].style.caption_rotation = -20.;
    let scene =
        parallel_coordinates_with_options(&config(), &dims, &rows(), &Default::default()).unwrap();
    let tick = scene
        .marks
        .iter()
        .find(|m| m.id == "parallel-axis:5:speed-tick-0")
        .unwrap();
    let Geometry::Path(commands) = &tick.geometry else {
        panic!()
    };
    assert_eq!(
        commands,
        &vec![C::Move(P::new(300., 170.)), C::Line(P::new(293., 170.))]
    );
    assert_eq!(tick.stroke.unwrap().width, 3.);
    let grid = scene
        .marks
        .iter()
        .find(|m| m.id == "parallel-axis:5:speed-grid-0")
        .unwrap();
    assert_eq!(
        grid.geometry,
        Geometry::Path(vec![
            C::Move(P::new(60., 170.)),
            C::Line(P::new(540., 170.))
        ])
    );
    let tick_label = scene
        .labels
        .iter()
        .find(|l| l.text == "5000%" && l.x < 300. && l.x > 280.)
        .unwrap();
    assert_eq!((tick_label.x, tick_label.size), (290., 18.));
    assert_eq!(tick_label.color, Color(90, 40, 20, 255));
    let caption = scene
        .labels
        .iter()
        .find(|l| l.text == "Speed caption")
        .unwrap();
    assert_eq!(
        (caption.x, caption.y, caption.size, caption.rotation),
        (300., 341., 20., -20.)
    );
    let painted = scene
        .paint_marks()
        .map(|m| m.id.as_str())
        .collect::<Vec<_>>();
    let row = painted
        .iter()
        .position(|id| *id == "parallel:3:row")
        .unwrap();
    assert!(painted[..row].iter().all(|id| id.contains("-grid-")));
    assert!(
        painted[row + 1..]
            .iter()
            .all(|id| id.starts_with("parallel-axis:"))
    );
}

#[test]
fn constant_and_empty_dimensions_center_and_hidden_negative_log_data_remains_valid() {
    let mut dims = dimensions();
    for d in &mut dims {
        d.domain = None;
    }
    let source = vec![ParallelDatum::new("r", vec![Some(3.), None, Some(5.)])];
    let scene =
        parallel_coordinates_with_options(&config(), &dims, &source, &Default::default()).unwrap();
    assert_eq!(points(&scene, "r"), vec![[60., 170.], [540., 170.]]);
    for d in &mut dims {
        d.scale = ScaleKind::Log;
    }
    let negative = vec![ParallelDatum::new("r", vec![Some(-3.); 3])];
    assert!(
        parallel_coordinates_with_options(&config(), &dims, &negative, &Default::default()).is_ok()
    );
    assert!(
        parallel_coordinates_with_options(
            &config(),
            &dims,
            &negative,
            &ParallelCoordinatesOptions {
                hidden: BTreeSet::from(["r".into()]),
                ..Default::default()
            }
        )
        .is_ok()
    );
}

#[test]
fn clamping_is_per_variable_and_does_not_rename_or_forge_endpoint_values() {
    let data = vec![ParallelDatum::new(
        "r",
        vec![Some(-50.), Some(50.), Some(150.)],
    )];
    let scene = parallel_coordinates_with_options(
        &config(),
        &dimensions(),
        &data,
        &ParallelCoordinatesOptions {
            clamp: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        points(&scene, "r"),
        vec![[60., 310.], [300., 170.], [540., 30.]]
    );
    assert!(
        scene
            .marks
            .iter()
            .find(|m| m.id == "parallel:1:r")
            .unwrap()
            .label
            .contains("quality: 150")
    );
}

#[test]
fn tooltip_missing_text_limit_and_safe_escaping_preserve_current_row_values() {
    let mut row = ParallelDatum::new("safe", vec![Some(25.), None, Some(75.)]);
    row.label = "<script>& row".into();
    let scene = parallel_coordinates_with_options(
        &config(),
        &dimensions(),
        &[row],
        &ParallelCoordinatesOptions {
            tooltip_dimensions: 2,
            missing_text: "N/A".into(),
            ..Default::default()
        },
    )
    .unwrap();
    let mark = scene
        .marks
        .iter()
        .find(|m| m.id == "parallel:4:safe")
        .unwrap();
    assert!(mark.label.contains("speed: N/A") && mark.label.contains("+1 dimensions"));
    let svg = render::to_svg(&scene).unwrap();
    assert!(svg.contains("&lt;script&gt;&amp; row") && !svg.contains("<script>"));
}

#[test]
fn malformed_values_domains_scales_styles_and_identity_profiles_fail_without_mutating_inputs() {
    let c = config();
    let dims = dimensions();
    let input = rows();
    let before = input.clone();
    let mut invalid = input.clone();
    invalid[0].values[0] = Some(f64::NAN);
    assert!(parallel_coordinates_with_options(&c, &dims, &invalid, &Default::default()).is_err());
    assert_eq!(input, before);
    let mut duplicate = dims.clone();
    duplicate[1].id = duplicate[0].id.clone();
    assert!(
        parallel_coordinates_with_options(&c, &duplicate, &input, &Default::default()).is_err()
    );
    for domain in [[100., 0.], [0., f64::INFINITY], [-f64::MAX, f64::MAX]] {
        let mut bad = dims.clone();
        bad[0].domain = Some(domain);
        assert!(parallel_coordinates_with_options(&c, &bad, &input, &Default::default()).is_err());
    }
    let mut log = dims.clone();
    log[0].scale = ScaleKind::Log;
    assert!(parallel_coordinates_with_options(&c, &log, &input, &Default::default()).is_err());
    let mut symlog = dims.clone();
    symlog[0].scale = ScaleKind::SymLog { constant: 0. };
    assert!(parallel_coordinates_with_options(&c, &symlog, &input, &Default::default()).is_err());
    let mut group = ParallelDatum::new("a", vec![Some(1.); 3]);
    group.group = Some("row".into());
    assert!(
        parallel_coordinates_with_options(
            &c,
            &dims,
            &[input[0].clone(), group],
            &Default::default()
        )
        .is_err()
    );
    assert!(
        parallel_coordinates_with_options(
            &c,
            &dims,
            &input,
            &ParallelCoordinatesOptions {
                line: ParallelLineStyle {
                    width: f64::NAN,
                    ..Default::default()
                },
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        parallel_coordinates_with_options(
            &c,
            &dims,
            &input,
            &ParallelCoordinatesOptions {
                series_styles: BTreeMap::from([("unknown".into(), Default::default())]),
                ..Default::default()
            }
        )
        .is_err()
    );
    let extreme = vec![ParallelDatum::new("huge", vec![Some(f64::MAX); 3])];
    assert!(parallel_coordinates_with_options(&c, &dims, &extreme, &Default::default()).is_err());
}

#[test]
fn dimensions_records_legends_ticks_and_tooltip_resources_are_bounded() {
    let c = config();
    let records = (0..2048)
        .map(|i| ParallelDatum::new(format!("r{i}"), vec![Some(50.); 3]))
        .collect::<Vec<_>>();
    assert!(
        parallel_coordinates_with_options(&c, &dimensions(), &records, &Default::default()).is_ok()
    );
    assert!(
        parallel_coordinates_with_options(
            &c,
            &dimensions(),
            &records,
            &ParallelCoordinatesOptions {
                legend: true,
                ..Default::default()
            }
        )
        .is_err()
    );
    let mut over = records;
    over.push(ParallelDatum::new("last", vec![Some(1.); 3]));
    assert!(
        parallel_coordinates_with_options(&c, &dimensions(), &over, &Default::default()).is_err()
    );
    let dims = (0..9)
        .map(|i| {
            let mut d = ParallelDimension::new(format!("d{i}"));
            d.axis.tick_values = Some((0..1000).map(f64::from).collect());
            d
        })
        .collect::<Vec<_>>();
    assert!(parallel_coordinates_with_options(&c, &dims, &[], &Default::default()).is_err());
    let mut row = rows()[0].clone();
    row.label = "x".repeat(1024);
    assert!(
        parallel_coordinates_with_options(&c, &dimensions(), &[row], &Default::default()).is_err()
    );
}
