use std::collections::BTreeMap;

use kartis::*;

fn controller(layout: ParallelLayout, reverse: bool) -> ParallelFilterController {
    let dims = ["x", "y"].map(|id| {
        let mut d = ParallelDimension::new(id);
        d.reverse = reverse;
        d
    });
    let mut rows = vec![
        ParallelDatum::new("a", vec![Some(0.), Some(10.)]),
        ParallelDatum::new("b", vec![Some(5.), Some(5.)]),
        ParallelDatum::new("c", vec![Some(10.), None]),
    ];
    for row in &mut rows {
        row.group = Some("shared".into());
    }
    ParallelFilterController::new(
        &ChartConfig::default(),
        &dims,
        &rows,
        &ParallelCoordinatesOptions {
            layout,
            row_labels: true,
            legend: true,
            ..Default::default()
        },
        &BTreeMap::new(),
    )
    .unwrap()
}
fn drawn(c: &ParallelFilterController) -> Vec<&str> {
    c.layout()
        .rows
        .iter()
        .filter(|r| r.drawn)
        .map(|r| r.datum.id.as_str())
        .collect()
}

#[test]
fn and_filter_preserves_domains_groups_source_metadata_and_owned_labels() {
    let mut c = controller(ParallelLayout::Horizontal, false);
    let original = c.layout().clone();
    c.set_filter("x", Some(ParallelFilterRange::new([0., 5.])))
        .unwrap();
    c.set_filter("y", Some(ParallelFilterRange::new([5., 5.])))
        .unwrap();
    assert_eq!(drawn(&c), vec!["b"]);
    assert_eq!(c.layout().scene.legend, original.scene.legend);
    assert_eq!(
        c.layout().dimensions[0].scale.domain(),
        original.dimensions[0].scale.domain()
    );
    assert_eq!(c.layout().rows.len(), 3);
    assert_eq!(c.layout().rows[1].source_index, 1);
    assert_eq!(c.layout().rows[1].points, original.rows[1].points);
    assert_eq!(
        c.layout().scene.labels.len(),
        original.scene.labels.len() - 2
    );
    assert!(
        c.layout()
            .scene
            .marks
            .iter()
            .all(|m| m.id != original.rows[0].mark_id && m.id != original.rows[2].mark_id)
    );
    c.layout().interaction.validate(&c.layout().scene).unwrap();
    assert!(
        c.layout()
            .interaction
            .sites
            .iter()
            .all(|site| site.mark_id == original.rows[1].mark_id)
    );
    assert!(
        c.layout()
            .interaction
            .anchors
            .keys()
            .all(|id| id == &original.rows[1].mark_id)
    );
    assert!(render::to_svg(&c.layout().scene).unwrap().contains("<svg"));
    assert!(render::to_canvas_script(&c.layout().scene, "filtered").is_ok());
    c.clear().unwrap();
    assert_eq!(c.layout().scene, original.scene);
    assert_eq!(drawn(&c), vec!["a", "b", "c"]);
}
#[test]
fn missing_policy_and_invalid_candidates_are_transactional() {
    let mut c = controller(ParallelLayout::Horizontal, false);
    c.set_filter(
        "y",
        Some(ParallelFilterRange {
            bounds: [0., 10.],
            missing: ParallelFilterMissing::Include,
        }),
    )
    .unwrap();
    assert_eq!(drawn(&c), vec!["a", "b", "c"]);
    c.set_filter("y", Some(ParallelFilterRange::new([0., 10.])))
        .unwrap();
    assert_eq!(drawn(&c), vec!["a", "b"]);
    let scene = c.layout().scene.clone();
    let filters = c.filters().clone();
    for range in [[10., 0.], [f64::NAN, 5.], [0., f64::INFINITY]] {
        assert!(
            c.set_filter("x", Some(ParallelFilterRange::new(range)))
                .is_err()
        );
    }
    assert!(c.set_filter("typo", None).is_err());
    let mut replacements = filters.clone();
    replacements.insert("unknown".into(), ParallelFilterRange::new([0., 1.]));
    assert!(c.replace_filters(replacements).is_err());
    assert!(
        c.replace_data(&[ParallelDatum::new("invalid", vec![Some(1.)])])
            .is_err()
    );
    assert_eq!(c.layout().scene, scene);
    assert_eq!(c.filters(), &filters);
    c.replace_data(&[ParallelDatum::new("new", vec![Some(100.), Some(20.)])])
        .unwrap();
    assert!(drawn(&c).is_empty());
    assert_eq!(c.layout().rows[0].datum.id, "new");
}
#[test]
fn screen_intervals_use_orientation_and_reverse_and_keep_endpoint_rows() {
    for layout in [ParallelLayout::Horizontal, ParallelLayout::Vertical] {
        for reverse in [false, true] {
            let mut c = controller(layout, reverse);
            let scale = &c.layout().dimensions[0].scale;
            let endpoints = [scale.map(0.), scale.map(5.)];
            c.set_pixel_filter("x", Some(endpoints), ParallelFilterMissing::Exclude)
                .unwrap();
            assert_eq!(drawn(&c), vec!["a", "b"]);
            let old = c.layout().scene.clone();
            assert!(
                c.set_pixel_filter("x", Some([-1e6, 1e6]), ParallelFilterMissing::Exclude)
                    .is_err()
            );
            assert_eq!(c.layout().scene, old);
            let scale = &c.layout().dimensions[0].scale;
            let p = scale.map(10.);
            let brush = match layout {
                ParallelLayout::Horizontal => Brush {
                    x0: 0.,
                    x1: 0.,
                    y0: p,
                    y1: p,
                },
                ParallelLayout::Vertical => Brush {
                    x0: p,
                    x1: p,
                    y0: 0.,
                    y1: 0.,
                },
            };
            c.set_brush_filter("x", Some(brush), ParallelFilterMissing::Exclude)
                .unwrap();
            assert_eq!(drawn(&c), vec!["c"]);
        }
    }
}
#[test]
fn utc_fractional_brush_boundaries_do_not_select_truncated_milliseconds() {
    let dims = [
        ParallelDimension::new("time"),
        ParallelDimension::new("value"),
    ];
    let rows = [
        ParallelDatum::new("zero", vec![Some(0.), Some(1.)]),
        ParallelDatum::new("one", vec![Some(1.), Some(2.)]),
    ];
    let profiles = BTreeMap::from([(
        "time".into(),
        ParallelScaleOptions {
            bounds: [Some(0.), Some(10.)],
            scale: ContinuousScaleOptions {
                kind: ScaleKind::Utc,
                ..Default::default()
            },
        },
    )]);
    let mut c = ParallelFilterController::new(
        &ChartConfig::default(),
        &dims,
        &rows,
        &Default::default(),
        &profiles,
    )
    .unwrap();
    let s = &c.layout().dimensions[0].scale;
    let r = s.range();
    let p = |v: f64| r[0] + v / 10. * (r[1] - r[0]);
    c.set_pixel_filter(
        "time",
        Some([p(0.2), p(0.8)]),
        ParallelFilterMissing::Exclude,
    )
    .unwrap();
    assert!(drawn(&c).is_empty());
}

#[test]
fn logarithmic_brush_reversal_does_not_pick_clamped_outliers() {
    for reverse in [false, true] {
        let dims = [ParallelDimension::new("x"), ParallelDimension::new("y")];
        let rows = [0.1, 1., 10., 100.]
            .map(|v| ParallelDatum::new(v.to_string(), vec![Some(v), Some(1.)]));
        let profiles = BTreeMap::from([(
            "x".into(),
            ParallelScaleOptions {
                bounds: [Some(1.), Some(100.)],
                scale: ContinuousScaleOptions {
                    kind: ScaleKind::Log,
                    reverse,
                    clamp: true,
                    ..Default::default()
                },
            },
        )]);
        let mut c = ParallelFilterController::new(
            &ChartConfig::default(),
            &dims,
            &rows,
            &Default::default(),
            &profiles,
        )
        .unwrap();
        let s = &c.layout().dimensions[0].scale;
        let pixels = [s.map(1.), s.map(10.)];
        c.set_pixel_filter("x", Some(pixels), ParallelFilterMissing::Exclude)
            .unwrap();
        assert_eq!(drawn(&c), vec!["1", "10"]);
    }
}
#[test]
fn group_visibility_and_missing_record_policy_remain_independent() {
    let dims = [ParallelDimension::new("x"), ParallelDimension::new("y")];
    let mut hidden = ParallelDatum::new("hidden", vec![Some(1.), Some(1.)]);
    hidden.group = Some("group".into());
    let rows = [
        hidden,
        ParallelDatum::new("missing", vec![Some(1.), None]),
        ParallelDatum::new("visible", vec![Some(1.), Some(2.)]),
    ];
    let options = ParallelCoordinatesOptions {
        hidden: std::collections::BTreeSet::from(["group".into()]),
        missing: ParallelMissing::SkipRecord,
        ..Default::default()
    };
    let mut c = ParallelFilterController::new(
        &ChartConfig::default(),
        &dims,
        &rows,
        &options,
        &BTreeMap::new(),
    )
    .unwrap();
    c.set_filter(
        "y",
        Some(ParallelFilterRange {
            bounds: [0., 10.],
            missing: ParallelFilterMissing::Include,
        }),
    )
    .unwrap();
    assert_eq!(drawn(&c), vec!["visible"]);
    assert_eq!(c.layout().rows.len(), 3);
}
