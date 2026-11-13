use glory::render::PathCommand;
use kartis::{
    surface::{SurfaceInput, SurfaceModel, SurfaceOptions},
    *,
};
use std::collections::{BTreeMap, BTreeSet};

fn frame() -> ChartConfig {
    ChartConfig {
        width: 400.,
        height: 240.,
        margin: Margin {
            left: 40.,
            right: 20.,
            top: 20.,
            bottom: 40.,
        },
        ..Default::default()
    }
}

fn near(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1e-9, "{actual} != {expected}");
}

#[test]
fn d3_scales_share_resolved_domains_ticks_and_projected_vertices() {
    const FEB_28: f64 = 1_709_078_400_000.;
    const FEB_29: f64 = 1_709_164_800_000.;
    const MAR_02: f64 = 1_709_337_600_000.;
    let dimensions = ["linear", "log", "date"].map(ParallelDimension::new);
    let data = [
        ParallelDatum::new("middle", vec![Some(2.2), Some(12.), Some(FEB_29)]),
        ParallelDatum::new("low", vec![None, Some(2.2), Some(FEB_28)]),
        ParallelDatum::new("high", vec![Some(8.3), Some(90.), Some(MAR_02)]),
    ];
    let profiles = BTreeMap::from([
        (
            "linear".into(),
            ParallelScaleOptions {
                bounds: [Some(0.), None],
                scale: ContinuousScaleOptions {
                    nice: ScaleNice::Count(4),
                    round: true,
                    ..Default::default()
                },
            },
        ),
        (
            "log".into(),
            ParallelScaleOptions {
                scale: ContinuousScaleOptions {
                    kind: ScaleKind::Log,
                    log_base: 2.,
                    reverse: true,
                    nice: ScaleNice::Default,
                    ..Default::default()
                },
                ..Default::default()
            },
        ),
        (
            "date".into(),
            ParallelScaleOptions {
                scale: ContinuousScaleOptions {
                    kind: ScaleKind::Utc,
                    ..Default::default()
                },
                ..Default::default()
            },
        ),
    ]);
    let layout = parallel_layout_with_scale_options(
        &frame(),
        &dimensions,
        &data,
        &Default::default(),
        &profiles,
    )
    .unwrap();
    let [linear, log, date] = layout.dimensions.as_slice() else {
        panic!("missing dimensions")
    };
    // d3-scale 4.0.2 scaleLinear().rangeRound().nice(4), scaleLog().base(2).nice(), scaleUtc().
    assert_eq!(linear.source_domain, [0., 8.3]);
    assert_eq!(linear.scale.domain(), [0., 10.]);
    assert_eq!(linear.scale.ticks(4), [0., 2., 4., 6., 8., 10.]);
    assert_eq!(log.source_domain, [2.2, 90.]);
    assert_eq!(log.scale.domain(), [2., 128.]);
    assert_eq!(log.scale.ticks(4), [4., 16., 64.]);
    assert_eq!(
        date.scale.ticks(4),
        [FEB_28, FEB_29, 1_709_251_200_000., MAR_02]
    );
    let expected = [[40., 160.], [210., 97.5488750216347], [380., 140.]];
    assert_eq!(layout.rows[0].datum, data[0]);
    for (actual, expected) in layout.rows[0].points.iter().zip(expected) {
        let actual = actual.unwrap();
        near(actual[0], expected[0]);
        near(actual[1], expected[1]);
    }
    let mark = layout
        .scene
        .marks
        .iter()
        .find(|mark| mark.id == layout.rows[0].mark_id)
        .unwrap();
    let commands = mark.geometry.commands();
    for (command, expected) in commands.iter().zip(expected) {
        let (PathCommand::Move(point) | PathCommand::Line(point)) = command else {
            panic!("unexpected curve command")
        };
        near(point.x, expected[0]);
        near(point.y, expected[1]);
    }
    assert!(mark.label.contains("date: 2024-02-29"));
    let tick_text = ValueFormat::Utc.format(FEB_29).unwrap();
    let leap_tick = layout
        .scene
        .labels
        .iter()
        .find(|label| label.text == tick_text)
        .unwrap();
    near(leap_tick.y, date.scale.map(FEB_29) + leap_tick.size / 3.);
}

#[test]
fn typed_rows_keep_missing_slots_and_domains_across_hide_and_skip() {
    let dimensions = ["a", "b"].map(ParallelDimension::new);
    let data = [
        ParallelDatum::new("z-hidden", vec![Some(20.), Some(20.)]),
        ParallelDatum::new("b-live", vec![Some(4.), Some(5.)]),
        ParallelDatum::new("a-incomplete", vec![Some(1.), None]),
    ];
    let opts = ParallelCoordinatesOptions {
        hidden: BTreeSet::from(["z-hidden".into()]),
        missing: ParallelMissing::SkipRecord,
        ..Default::default()
    };
    let layout =
        parallel_layout_with_scale_options(&frame(), &dimensions, &data, &opts, &BTreeMap::new())
            .unwrap();
    assert_eq!(layout.dimensions[0].source_domain, [1., 20.]);
    assert_eq!(layout.dimensions[1].source_domain, [5., 20.]);
    assert_eq!(
        layout
            .rows
            .iter()
            .map(|row| (row.source_index, row.drawn))
            .collect::<Vec<_>>(),
        [(0, false), (1, true), (2, false)]
    );
    assert_eq!(layout.rows[2].points[1], None);
    assert!(layout.rows[0].points.iter().all(Option::is_some));
    assert_eq!(layout.interaction.sites.len(), 1);
    assert_eq!(layout.interaction.sites[0].mark_id, layout.rows[1].mark_id);
    let old = parallel_coordinates_with_options(&frame(), &dimensions, &data, &opts).unwrap();
    assert_ne!(old, layout.scene);
}

#[test]
fn vertical_clamp_keeps_original_values_and_exact_path_picking() {
    let dimensions = ["a", "b"].map(ParallelDimension::new);
    let data = [ParallelDatum::new("live", vec![Some(12.), Some(-5.)])];
    let mut opts = ParallelCoordinatesOptions {
        layout: ParallelLayout::Vertical,
        ..Default::default()
    };
    opts.line.width = 5.;
    let profile = ParallelScaleOptions {
        bounds: [Some(0.), Some(10.)],
        scale: ContinuousScaleOptions {
            clamp: true,
            ..Default::default()
        },
    };
    let layout = parallel_layout_with_scale_options(
        &frame(),
        &dimensions,
        &data,
        &opts,
        &BTreeMap::from([("a".into(), profile), ("b".into(), profile)]),
    )
    .unwrap();
    assert_eq!(
        layout.rows[0].points,
        [Some([380., 200.]), Some([40., 20.])]
    );
    assert_eq!(layout.rows[0].datum.values, [Some(12.), Some(-5.)]);
    let mark_id = layout.rows[0].mark_id.clone();
    let candidate = layout.into_scene_interaction();
    let mut surface =
        SurfaceModel::new(candidate.scene.clone(), SurfaceOptions::default()).unwrap();
    surface.set_layout(candidate).unwrap();
    surface
        .apply(SurfaceInput::Move {
            id: 1,
            point: (210., 110.),
        })
        .unwrap();
    assert_eq!(surface.hovered(), &[mark_id]);
    assert_eq!(
        surface.interaction_data().unwrap().sites[0].value,
        Some(12.)
    );
    surface
        .apply(SurfaceInput::Move {
            id: 1,
            point: (210., 160.),
        })
        .unwrap();
    assert!(surface.hovered().is_empty());
}

#[test]
fn profiles_reject_unknown_dimensions_and_invalid_hidden_values() {
    let dimensions = ["a", "b"].map(ParallelDimension::new);
    let data = [ParallelDatum::new("hidden", vec![Some(-1.), Some(1.)])];
    let opts = ParallelCoordinatesOptions {
        hidden: BTreeSet::from(["hidden".into()]),
        ..Default::default()
    };
    let profile = ParallelScaleOptions {
        bounds: [Some(1.), Some(10.)],
        scale: ContinuousScaleOptions {
            kind: ScaleKind::Log,
            ..Default::default()
        },
    };
    for key in ["a", "unknown"] {
        assert!(
            parallel_layout_with_scale_options(
                &frame(),
                &dimensions,
                &data,
                &opts,
                &BTreeMap::from([(key.into(), profile)])
            )
            .is_err()
        );
    }
}

#[test]
fn actual_calendar_ticks_count_toward_the_chart_budget() {
    let day = 86_400_000.;
    let dimensions = (0..10)
        .map(|index| {
            let mut dimension = ParallelDimension::new(format!("day-{index}"));
            dimension.domain = Some([0., 900. * day]);
            dimension.scale = ScaleKind::Utc;
            dimension.axis.tick_calendar = Some(TimeCalendar {
                interval: Some(TimeInterval::new(TimeUnit::Day, 1).unwrap()),
                ..Default::default()
            });
            dimension
        })
        .collect::<Vec<_>>();
    let err = parallel_layout_with_scale_options(
        &frame(),
        &dimensions,
        &[],
        &Default::default(),
        &BTreeMap::new(),
    )
    .unwrap_err();
    assert_eq!(err.0, "parallel axis tick budget exceeded");
}
