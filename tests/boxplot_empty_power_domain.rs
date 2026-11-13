use kartis::{
    surface::{Picking, SurfaceInput, SurfaceModel, SurfaceOptions},
    *,
};

fn frame() -> ChartConfig {
    ChartConfig {
        width: 360.,
        height: 280.,
        margin: Margin {
            top: 20.,
            right: 20.,
            bottom: 30.,
            left: 45.,
        },
        ..Default::default()
    }
}
fn observations(values: &[f64]) -> Vec<BoxObservation> {
    values
        .iter()
        .enumerate()
        .map(|(i, v)| BoxObservation::new(i.to_string(), "Group", "sensor", Some(*v)))
        .collect()
}
fn profile(exponent: f64) -> ContinuousScaleOptions {
    ContinuousScaleOptions {
        kind: ScaleKind::Pow { exponent },
        ..Default::default()
    }
}
fn no_fake_data(layout: &BoxPlotLayout) {
    assert!(layout.boxes.is_empty());
    assert!(layout.scene.marks.iter().all(|m| m.series.is_empty()));
    assert!(!layout.scene.marks.iter().any(|m| m.id.starts_with("box:")));
    layout.scene.validate().unwrap();
    let mut model = SurfaceModel::new_with_interaction(
        layout.scene.clone(),
        SurfaceOptions {
            picking: Picking::Nearest { distance: 1000. },
            ..Default::default()
        },
        Some(InteractionBinding::default()),
    )
    .unwrap();
    assert!(model.interaction_data().unwrap().sites.is_empty());
    assert!(model.data_control_labels().is_empty());
    model
        .apply(SurfaceInput::Move {
            id: 1,
            point: (160., 120.),
        })
        .unwrap();
    assert!(model.hovered().is_empty());
}

#[test]
fn completely_empty_inverse_power_boxplots_use_nonzero_defaults_in_both_entry_points() {
    for orientation in [
        StatisticsOrientation::Vertical,
        StatisticsOrientation::Horizontal,
    ] {
        for exponent in [-1., -0.5, -2.] {
            let o = BoxPlotLayoutOptions {
                orientation,
                scale: ScaleKind::Pow { exponent },
                ..Default::default()
            };
            let legacy = boxplot_layout(&frame(), &[], &o);
            assert!(legacy.is_ok(), "empty legacy Pow({exponent}): {legacy:?}");
            let legacy = legacy.unwrap();
            assert_eq!(legacy.domain, [1., 10.]);
            no_fake_data(&legacy);
            let configured =
                boxplot_layout_with_scale_options(&frame(), &[], &o, &profile(exponent));
            assert!(
                configured.is_ok(),
                "empty configured Pow({exponent}): {configured:?}"
            );
            let configured = configured.unwrap();
            assert_eq!(configured.value_scale.source_domain(), [1., 10.]);
            assert_eq!(configured.layout.domain, [1., 10.]);
            no_fake_data(&configured.layout);
            assert_eq!(configured.layout, legacy);
        }
    }
}

#[test]
fn null_only_and_all_hidden_samples_preserve_category_slots_and_nonzero_sign() {
    for orientation in [
        StatisticsOrientation::Vertical,
        StatisticsOrientation::Horizontal,
    ] {
        for (values, expected) in [
            (vec![2., 4., 8.], [1., 10.]),
            (vec![-8., -4., -2.], [-10., -1.]),
        ] {
            let data = observations(&values);
            let o = BoxPlotLayoutOptions {
                orientation,
                scale: ScaleKind::Pow { exponent: -1. },
                hidden: [boxplot_series_id("sensor")].into(),
                ..Default::default()
            };
            let layout =
                boxplot_layout_with_scale_options(&frame(), &data, &o, &profile(-1.)).unwrap();
            assert_eq!(layout.value_scale.source_domain(), expected);
            no_fake_data(&layout.layout);
            assert_eq!(layout.layout.groups, ["Group"]);
            assert_eq!(layout.layout.subgroups, ["sensor"]);
            assert_eq!(
                layout.layout.scene.legend[0].id,
                boxplot_series_id("sensor")
            );
            let legacy = boxplot_layout(&frame(), &data, &o).unwrap();
            assert_eq!(layout.layout, legacy);
        }
        let data = [BoxObservation::new("missing", "Group", "sensor", None)];
        let o = BoxPlotLayoutOptions {
            orientation,
            groups: Some(vec!["Group".into(), "reserved".into()]),
            subgroups: Some(vec!["sensor".into()]),
            ..Default::default()
        };
        let layout = boxplot_layout_with_scale_options(&frame(), &data, &o, &profile(-1.)).unwrap();
        assert_eq!(layout.value_scale.source_domain(), [1., 10.]);
        no_fake_data(&layout.layout);
        assert_eq!(layout.layout.groups, ["Group", "reserved"]);
    }
}

#[test]
fn empty_fallback_uses_author_tick_sign_but_rejects_zero_and_cross_sign_inputs() {
    let mut o = BoxPlotLayoutOptions {
        scale: ScaleKind::Pow { exponent: -1. },
        ..Default::default()
    };
    for (ticks, expected) in [
        (vec![-8., -4., -1.], [-10., -1.]),
        (vec![1., 4., 8.], [1., 10.]),
    ] {
        o.value_axis.tick_values = Some(ticks);
        let result = boxplot_layout_with_scale_options(&frame(), &[], &o, &profile(-1.)).unwrap();
        assert_eq!(result.value_scale.source_domain(), expected);
        no_fake_data(&result.layout);
        assert_eq!(boxplot_layout(&frame(), &[], &o).unwrap().domain, expected);
    }
    for ticks in [vec![0.], vec![-1., 1.], vec![-1., 0.]] {
        o.value_axis.tick_values = Some(ticks);
        assert!(boxplot_layout_with_scale_options(&frame(), &[], &o, &profile(-1.)).is_err());
        assert!(boxplot_layout(&frame(), &[], &o).is_err());
    }
    o.value_axis.tick_values = None;
    o.hidden.insert(boxplot_series_id("sensor"));
    for values in [vec![0.], vec![-1., 1.], vec![-1., 0.]] {
        assert!(
            boxplot_layout_with_scale_options(&frame(), &observations(&values), &o, &profile(-1.))
                .is_err()
        );
        assert!(boxplot_layout(&frame(), &observations(&values), &o).is_err());
    }
}

#[test]
fn explicit_author_domains_take_precedence_and_remain_strict_on_empty_or_hidden_samples() {
    for domain in [
        [2., 8.],
        [-8., -2.],
        [8., 2.],
        [-2., -8.],
        [2., 2.],
        [-2., -2.],
    ] {
        let o = BoxPlotLayoutOptions {
            domain: Some(domain),
            hidden: [boxplot_series_id("sensor")].into(),
            scale: ScaleKind::Pow { exponent: -1. },
            ..Default::default()
        };
        for data in [vec![], observations(&[0., -4., 4.])] {
            let result =
                boxplot_layout_with_scale_options(&frame(), &data, &o, &profile(-1.)).unwrap();
            assert_eq!(result.value_scale.source_domain(), domain);
            assert_eq!(result.layout.domain, domain);
            no_fake_data(&result.layout);
            if domain[0] <= domain[1] {
                assert_eq!(boxplot_layout(&frame(), &data, &o).unwrap().domain, domain);
            } else {
                assert!(boxplot_layout(&frame(), &data, &o).is_err());
            }
        }
    }
    for domain in [
        [0., 1.],
        [-1., 0.],
        [-1., 1.],
        [0., 0.],
        [f64::NAN, 1.],
        [1., f64::INFINITY],
    ] {
        let o = BoxPlotLayoutOptions {
            domain: Some(domain),
            scale: ScaleKind::Pow { exponent: -1. },
            ..Default::default()
        };
        assert!(boxplot_layout_with_scale_options(&frame(), &[], &o, &profile(-1.)).is_err());
        assert!(boxplot_layout(&frame(), &[], &o).is_err());
    }
}

#[cfg(feature = "raster")]
#[test]
fn actual_cpu_empty_inverse_power_draws_only_authored_axes_and_no_statistical_ink() {
    use kartis::render::raster::*;
    let mut fonts = FontRegistry::new();
    fonts
        .register(include_bytes!("fonts/ABeeZee-Regular.ttf").as_slice(), 0)
        .unwrap();
    for orientation in [
        StatisticsOrientation::Vertical,
        StatisticsOrientation::Horizontal,
    ] {
        for ticks in [vec![1., 4., 10.], vec![-10., -4., -1.]] {
            let o = BoxPlotLayoutOptions {
                orientation,
                legend: false,
                group_axis: CategoryAxisOptions {
                    visible: false,
                    ..Default::default()
                },
                value_axis: AxisOptions {
                    tick_values: Some(ticks),
                    grid: false,
                    ..Default::default()
                },
                ..Default::default()
            };
            let result =
                boxplot_layout_with_scale_options(&frame(), &[], &o, &profile(-1.)).unwrap();
            no_fake_data(&result.layout);
            let raster = render_raster(&result.layout.scene, &fonts, &Default::default()).unwrap();
            let ink = raster
                .rgba
                .as_chunks::<4>()
                .0
                .iter()
                .filter(|p| p[3] > 0 && p[..3] != [255, 255, 255])
                .count();
            assert!(ink > 100, "empty inverse-power axis missing actual CPU ink");
            let quiet = BoxPlotLayoutOptions {
                value_axis: AxisOptions {
                    visible: false,
                    grid: false,
                    ..Default::default()
                },
                ..o
            };
            let quiet =
                boxplot_layout_with_scale_options(&frame(), &[], &quiet, &profile(-1.)).unwrap();
            assert!(quiet.layout.scene.marks.is_empty());
            assert!(quiet.layout.scene.labels.is_empty());
            let blank = render_raster(&quiet.layout.scene, &fonts, &Default::default()).unwrap();
            assert!(
                blank
                    .rgba
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .all(|p| p[..3] == [255, 255, 255])
            );
        }
    }
}
