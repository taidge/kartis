use glory::render::PathCommand;
use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 400.,
        height: 400.,
        margin: Margin {
            left: 40.,
            right: 40.,
            top: 40.,
            bottom: 40.,
        },
        ..Default::default()
    }
}

fn options() -> RadarLayoutOptions {
    RadarLayoutOptions {
        radius_ratio: 0.8,
        ..Default::default()
    }
}

fn axes(n: usize) -> Vec<String> {
    (0..n).map(|j| format!("axis{j}")).collect()
}

fn near(a: f64, b: f64) {
    assert!((a - b).abs() <= 1e-10 * b.abs().max(1.), "{a} != {b}");
}

fn list(s: &str) -> Vec<f64> {
    s.split(',').map(|v| v.parse().unwrap()).collect()
}

fn log(base: f64) -> RadarValueScaleOptions {
    RadarValueScaleOptions {
        kind: RadarValueScaleKind::Log { base },
        ..Default::default()
    }
}

#[test]
fn eight_actual_d3_profiles_match_domains_points_inverse_ticks_and_ink() {
    let c = config();
    let mut count = 0;
    for row in include_str!("fixtures/radar-scale-d3.csv").lines().skip(2) {
        let row: Vec<_> = row.split('|').collect();
        let parameter = row[1].parse().unwrap();
        let kind = match row[0] {
            "linear" => RadarValueScaleKind::Linear,
            "log" => RadarValueScaleKind::Log { base: parameter },
            "symlog" => RadarValueScaleKind::SymLog {
                constant: parameter,
            },
            _ => unreachable!(),
        };
        let scale = RadarValueScaleOptions {
            kind,
            nice: match row[3].parse::<i32>().unwrap() {
                -1 => ScaleNice::Off,
                0 => ScaleNice::Default,
                n => ScaleNice::Count(n as usize),
            },
            grid_ticks: if row[4] == "even" {
                RadarGridTickMode::EqualRadius
            } else {
                RadarGridTickMode::ScaleTicks
            },
        };
        let values = list(row[6]);
        let axes = axes(values.len());
        let data = [RadarSeries::new("data", values.clone())];
        let mut o = options();
        o.domain = Some(list(row[2]).try_into().unwrap());
        o.grid.levels = row[5].parse().unwrap();
        let chart =
            radar_layout_with_value_scale(&c, &axes, &data, &o, &scale, &Default::default())
                .unwrap();
        let expected_display = list(row[7]);
        for (actual, expected) in chart.layout.domain.into_iter().zip(expected_display) {
            near(actual, expected);
        }
        assert_eq!(chart.value_scale.display_domain(), chart.layout.domain);
        assert_eq!(chart.value_scale.source_domain(), o.domain.unwrap());
        assert_eq!(chart.value_scale.outer_radius(), 128.);
        for ((slice, value), expected) in chart.layout.slices.iter().zip(values).zip(list(row[8])) {
            let p = slice.rows[0].position;
            near(p[0], 200. + expected * slice.angle.sin());
            near(p[1], 200. - expected * slice.angle.cos());
            near(chart.value_scale.radius_at(value).unwrap(), expected);
            near(chart.value_scale.value_at(expected).unwrap(), value);
        }
        let expected_ticks = list(row[9]);
        assert_eq!(chart.ticks.len(), expected_ticks.len());
        for (i, ((tick, value), radius)) in chart
            .ticks
            .iter()
            .zip(expected_ticks)
            .zip(list(row[10]))
            .enumerate()
        {
            near(tick.value, value);
            near(tick.radius, radius);
            let id = format!("radar-grid:{i}");
            let mark = chart.layout.scene.marks.iter().find(|m| m.id == id);
            if radius == 0. {
                assert!(mark.is_none());
            } else {
                let Geometry::Circle { radius: ink, .. } = mark.unwrap().geometry else {
                    panic!("default grid must paint circles");
                };
                near(ink, radius);
            }
        }
        assert_eq!(
            radar_with_value_scale(&c, &axes, &data, &o, &scale).unwrap(),
            chart.layout.scene
        );
        count += 1;
    }
    assert_eq!(count, 8);
}

#[test]
fn signed_zero_ring_preserves_source_order_and_matches_geometry_in_both_directions() {
    let c = config();
    let axes = axes(4);
    let data = [
        RadarSeries::new("A", vec![-40., 0., 10., 40.]),
        RadarSeries::new("B", vec![0., -10., 10., 40.]),
    ];
    let mut o = options();
    o.grid.values = Some(vec![-40., 0., 40.]);
    o.grid.tick_labels = true;
    o.grid.tick_label_axis = 1;
    o.grid.tick_label_offset = [0., 0.];
    o.grid.shape = RadarGridShape::Polygon;
    o.points.enabled = false;
    let a = RadarSeriesStyle {
        points: Some(RadarPointOptions {
            shape: RadarPointShape::Triangle,
            labels: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    o.series_styles.insert("A".into(), a);
    let scale = RadarValueScaleOptions {
        kind: RadarValueScaleKind::SymLog { constant: 2. },
        ..Default::default()
    };
    for direction in [
        PolarSliceDirection::Clockwise,
        PolarSliceDirection::CounterClockwise,
    ] {
        let chart = radar_layout_with_value_scale(
            &c,
            &axes,
            &data,
            &o,
            &scale,
            &RadarInteractionOptions {
                direction,
                ..Default::default()
            },
        )
        .unwrap();
        let layout = &chart.layout;
        assert_eq!(chart.value_scale.zero_radius(), Some(64.));
        assert_eq!(chart.value_scale.display_domain(), [-40., 40.]);
        assert_eq!(chart.ticks[0].radius, 0.);
        assert_eq!(chart.ticks[1].radius, 64.);
        let sign = if direction == PolarSliceDirection::Clockwise {
            1.
        } else {
            -1.
        };
        let zero = layout.slices[1]
            .rows
            .iter()
            .find(|r| r.series_id == "A")
            .unwrap();
        near(zero.position[0], 200. + 64. * sign);
        near(zero.position[1], 200.);
        assert_eq!(zero.raw_value, 0.);
        assert!(layout.anchors.contains_key(&zero.point_mark_id));
        let b = &layout.slices[1].rows[1];
        assert_eq!(b.series_id, "B");
        assert_eq!(b.raw_value, -10.);
        assert!(b.glyph_mark_id.is_none());
        near(
            (b.position[0] - 200.).abs(),
            chart.value_scale.radius_at(-10.).unwrap(),
        );
        let label = &layout.scene.labels[1];
        assert_eq!(label.text, "0");
        near(label.x, zero.position[0]);
        near(label.y, zero.position[1]);
        let grid = layout
            .scene
            .marks
            .iter()
            .find(|m| m.id == "radar-grid:1")
            .unwrap();
        let Geometry::Path(path) = &grid.geometry else {
            panic!("polygon ring expected")
        };
        let PathCommand::Move(p) = path[0] else {
            panic!("missing grid start")
        };
        near(p.x, 200.);
        near(p.y, 136.);
        assert!(layout.scene.label_series.values().all(|owner| owner == "A"));
        layout
            .interaction_binding()
            .validate(&layout.scene)
            .unwrap();
        assert_eq!(
            layout.polar.pick(200. + sign * 97., 200.).unwrap().axis_id,
            axes[1]
        );
    }
}

#[test]
fn log_domain_admission_hidden_extent_and_numeric_limits_are_checked() {
    let c = config();
    let axes = axes(3);
    let source = [
        RadarSeries::new("shown", vec![-40., -10., -3.]),
        RadarSeries::new("hidden", vec![-100., -2., -1.]),
    ];
    let mut o = options();
    o.hidden.insert("hidden".into());
    let chart =
        radar_layout_with_value_scale(&c, &axes, &source, &o, &log(10.), &Default::default())
            .unwrap();
    assert_eq!(chart.value_scale.source_domain(), [-100., -1.]);
    assert_eq!(chart.value_scale.zero_radius(), None);
    assert_eq!(chart.value_scale.radius_at(-100.).unwrap(), 0.);
    assert_eq!(chart.value_scale.radius_at(-1.).unwrap(), 128.);
    assert!(chart.value_scale.radius_at(0.).is_err());
    assert!(chart.value_scale.value_at(-1.).is_err());
    assert!(chart.value_scale.value_at(f64::NAN).is_err());
    assert!(chart.layout.slices.iter().all(|s| s.rows.len() == 1));

    for values in [vec![-1., 1., 2.], vec![0., 1., 2.], vec![-3., -3., -3.]] {
        assert!(
            radar_with_value_scale(
                &c,
                &axes,
                &[RadarSeries::new("invalid", values)],
                &o,
                &log(10.),
            )
            .is_err()
        );
    }
    assert!(radar_with_value_scale(&c, &axes, &[], &o, &log(10.)).is_err());
    assert!(radar_with_value_scale(&c, &axes, &source, &o, &log(1.)).is_err());
    o.domain = Some([-97., -1.]);
    let nice = RadarValueScaleOptions {
        nice: ScaleNice::Default,
        ..log(10.)
    };
    assert!(radar_with_value_scale(&c, &axes, &source, &o, &nice).is_err());
    let lo: f64 = 1e200;
    o.domain = Some([lo, f64::from_bits(lo.to_bits() + 1)]);
    assert!(radar_with_value_scale(&c, &axes, &[], &o, &log(10.)).is_err());
    o.domain = Some([1., 1e8]);
    o.grid.levels = 64;
    let ticks = RadarValueScaleOptions {
        grid_ticks: RadarGridTickMode::ScaleTicks,
        ..log(10.)
    };
    assert!(radar_with_value_scale(&c, &axes, &[], &o, &ticks).is_err());
    o.grid.values = Some(vec![1., 1e4, 1e8]);
    assert!(radar_with_value_scale(&c, &axes, &[], &o, &ticks).is_ok());
    o.grid.values = Some(vec![1e4, 1e4]);
    assert!(radar_with_value_scale(&c, &axes, &[], &o, &ticks).is_err());
}

#[test]
fn surface_reads_current_raw_values_on_nonlinear_radius_and_rejects_stale_binding() {
    use kartis::surface::{SurfaceInput, SurfaceModel, SurfaceOptions};
    let c = config();
    let axes = axes(3);
    let mut o = options();
    o.domain = Some([1., 100.]);
    o.points.enabled = false;
    let mut data = [RadarSeries::new("A", vec![10., 1., 100.])];
    let layout = |data: &[RadarSeries]| {
        radar_layout_with_value_scale(&c, &axes, data, &o, &log(10.), &Default::default())
            .unwrap()
            .layout
    };
    let old = layout(&data);
    let mut model = SurfaceModel::new_with_interaction(
        old.scene.clone(),
        SurfaceOptions::default(),
        Some(old.interaction_binding()),
    )
    .unwrap();
    model
        .apply(SurfaceInput::Wheel {
            point: (200., 200.),
            delta: -75.,
        })
        .unwrap();
    let point = model.view().project((200., 105.)).unwrap();
    model.apply(SurfaceInput::Move { id: 1, point }).unwrap();
    let hover = model.hovered().to_vec();
    let interaction = model.active_interaction(&hover).unwrap();
    assert_eq!(interaction.rows[0].value, Some(10.));
    assert_eq!(interaction.rows[0].position, [200., 136.]);
    data[0].values[0] = 25.;
    let new = layout(&data);
    assert!(
        model
            .set_scene_with_interaction(new.scene.clone(), Some(old.interaction_binding()))
            .is_err()
    );
    assert_eq!(model.scene(), &old.scene);
    assert_eq!(model.hovered(), hover);
    model
        .set_scene_with_interaction(new.scene.clone(), Some(new.interaction_binding()))
        .unwrap();
    model.apply(SurfaceInput::Move { id: 1, point }).unwrap();
    let interaction = model.active_interaction(model.hovered()).unwrap();
    assert_eq!(interaction.rows[0].value, Some(25.));
    near(interaction.rows[0].position[1], 200. - 64. * 25_f64.log10());
}

#[test]
fn original_linear_profile_keeps_marks_and_grid_with_no_scale_configuration() {
    let c = config();
    let axes = axes(4);
    let data = [RadarSeries::new("A", vec![0., 4., 8., 16.])];
    let mut o = options();
    o.grid.levels = 4;
    let legacy = radar_with_options(&c, &axes, &data, &o).unwrap();
    assert_eq!(
        legacy,
        radar_with_value_scale(&c, &axes, &data, &o, &Default::default()).unwrap()
    );
    let negative = [RadarSeries::new("A", vec![-2., -1., 0., 2.])];
    assert!(radar_with_options(&c, &axes, &negative, &o).is_err());
    assert!(radar_with_value_scale(&c, &axes, &negative, &o, &Default::default()).is_ok());
}

#[cfg(feature = "raster")]
#[test]
fn cpu_marks_use_log_radius_instead_of_linear_positions() {
    use render::raster::{FontRegistry, render_raster};
    let c = config();
    let axes = axes(4);
    let data = [RadarSeries::new("A", vec![10., 100., 1., 10.])];
    let mut o = options();
    o.domain = Some([1., 100.]);
    o.axis_labels = false;
    o.grid.line.enabled = false;
    o.grid.spokes.enabled = false;
    o.series.fill_enabled = false;
    o.series.border.enabled = false;
    o.points.fill = Some(Color(210, 30, 60, 255));
    let scene = radar_with_value_scale(&c, &axes, &data, &o, &log(10.)).unwrap();
    let image = render_raster(&scene, &FontRegistry::new(), &Default::default()).unwrap();
    let pixel = |x: usize, y: usize| {
        let i = (y * image.width as usize + x) * 4;
        &image.rgba[i..i + 4]
    };
    assert_eq!(pixel(200, 136), [210, 30, 60, 255]);
    assert_eq!(pixel(200, 200), [210, 30, 60, 255]);
    assert_eq!(pixel(200, 188), [255, 255, 255, 255]);
}
