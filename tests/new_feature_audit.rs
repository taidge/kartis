//! Independent export/identity checks for the new extension boundaries.
use std::collections::BTreeMap;

use kartis::*;

#[test]
fn funnel_paint_budget_counts_the_completed_replacement_frame() {
    let gradient = |n: usize| FillPaint::Linear {
        start: [0., 0.],
        end: [10., 10.],
        stops: (0..n)
            .map(|i| GradientStop::new(i as f64 / (n - 1) as f64, Color(255, 0, 0, 255)))
            .collect(),
    };
    let rows = (0..33)
        .map(|i| {
            let mut row = FunnelDatum::new(format!("row-{i}"), 1.);
            row.paint = Some(gradient(if i < 2 { 2 } else { 512 }));
            row
        })
        .collect::<Vec<_>>();
    let output = funnel_layout_with_stage_factory(
        &ChartConfig::default(),
        &rows,
        &Default::default(),
        &mut |context| {
            let mut result = context.inherit(context.geometry.clone());
            if context.source_index == 0 {
                result.body.fill_paint = Some(gradient(512));
            } else if context.source_index == 1 {
                result.body.fill_paint = None;
            }
            Ok(result)
        },
    )
    .expect("the finished frame has exactly 32 * 512 = 16384 stops");
    output.scene.validate().unwrap();
}

#[test]
fn inherited_funnel_gradient_keeps_export_and_business_identity() {
    let config = ChartConfig::default();
    let mut first = FunnelDatum::new("a", 8.);
    first.color = Some(Color(5, 6, 7, 255));
    first.paint = Some(FillPaint::Linear {
        start: [20., 20.],
        end: [180., 180.],
        stops: vec![
            GradientStop::new(0., Color(255, 0, 0, 100)),
            GradientStop::new(1., Color(0, 0, 255, 200)),
        ],
    });
    let source = [first, FunnelDatum::new("b", 3.)];
    for direction in [FunnelDirection::Vertical, FunnelDirection::Horizontal] {
        let options = FunnelLayoutOptions {
            direction,
            fill_opacity: 0.4,
            border_opacity: 0.7,
            current: Some("a".into()),
            current_size_extension: 2.,
            ..Default::default()
        };
        let base = funnel_layout(&config, &source, &options).unwrap();
        let custom = funnel_layout_with_stage_factory(&config, &source, &options, &mut |c| {
            Ok(c.inherit(c.geometry.clone()))
        })
        .unwrap();
        assert_eq!(base.scene, custom.scene);
        assert_eq!(base.parts, custom.parts);
        assert_eq!(base.interaction_binding(), custom.interaction_binding());
        assert_eq!(
            render::to_svg(&base.scene).unwrap(),
            render::to_svg(&custom.scene).unwrap()
        );
        assert_eq!(
            render::to_canvas_script(&base.scene, "audit").unwrap(),
            render::to_canvas_script(&custom.scene, "audit").unwrap()
        );
    }
}

#[test]
fn filtering_one_group_member_preserves_palette_then_restores_identical_export() {
    let config = ChartConfig::default();
    let dims = [ParallelDimension::new("x"), ParallelDimension::new("y")];
    let rows = [
        ParallelDatum {
            group: Some("same".into()),
            ..ParallelDatum::new("z", vec![Some(100.), Some(0.)])
        },
        ParallelDatum {
            group: Some("same".into()),
            ..ParallelDatum::new("a", vec![Some(0.), Some(100.)])
        },
    ];
    let options = ParallelCoordinatesOptions::default();
    let mut state =
        ParallelFilterController::new(&config, &dims, &rows, &options, &BTreeMap::new()).unwrap();
    let original = state.layout().clone();
    let svg = render::to_svg(&original.scene).unwrap();
    state
        .set_filter("x", Some(ParallelFilterRange::new([90., 100.])))
        .unwrap();
    assert_eq!(state.layout().scene.legend, original.scene.legend);
    assert_eq!(state.layout().rows.len(), 2);
    let z = state
        .layout()
        .rows
        .iter()
        .find(|r| r.datum.id == "z")
        .unwrap();
    let a = state
        .layout()
        .rows
        .iter()
        .find(|r| r.datum.id == "a")
        .unwrap();
    assert!(z.drawn && !a.drawn);
    assert_eq!(z.source_index, 0);
    assert_eq!(a.source_index, 1);
    assert_eq!(state.layout().dimensions[0].source_domain, [0., 100.]);
    state.clear().unwrap();
    assert_eq!(state.layout().scene, original.scene);
    assert_eq!(state.layout().interaction, original.interaction);
    assert_eq!(render::to_svg(&state.layout().scene).unwrap(), svg);
}

#[test]
fn rejected_source_and_filter_proposals_preserve_accepted_export_and_query_sites() {
    let config = ChartConfig::default();
    let dims = [ParallelDimension::new("x"), ParallelDimension::new("y")];
    let rows = [
        ParallelDatum::new("a", vec![Some(2.), Some(4.)]),
        ParallelDatum::new("b", vec![Some(4.), Some(2.)]),
    ];
    let mut state =
        ParallelFilterController::new(&config, &dims, &rows, &Default::default(), &BTreeMap::new())
            .unwrap();
    state
        .set_filter("x", Some(ParallelFilterRange::new([2., 2.])))
        .unwrap();
    let frame = state.layout().clone();
    let filters = state.filters().clone();
    let mut bad = rows.to_vec();
    bad[1].values[0] = Some(f64::NAN);
    assert!(state.replace_data(&bad).is_err());
    assert!(
        state
            .set_pixel_filter("y", Some([f64::NAN, 0.]), ParallelFilterMissing::Include)
            .is_err()
    );
    assert!(
        state
            .replace_filters(BTreeMap::from([(
                "unknown".into(),
                ParallelFilterRange::new([0., 10.])
            )]))
            .is_err()
    );
    assert_eq!(state.filters(), &filters);
    assert_eq!(state.layout().scene, frame.scene);
    assert_eq!(state.layout().interaction, frame.interaction);
    assert_eq!(state.layout().rows, frame.rows);
}
