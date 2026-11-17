use std::cell::Cell;

use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 360.,
        height: 360.,
        margin: Margin {
            left: 30.,
            right: 30.,
            top: 30.,
            bottom: 30.,
        },
        ..Default::default()
    }
}

fn options(rows: usize, columns: usize, total: f64) -> WaffleLayoutOptions {
    WaffleLayoutOptions {
        rows,
        columns,
        total: Some(total),
        padding: 0.,
        ..Default::default()
    }
}

fn layer(key: &str, kind: WaffleSvgLayerKind) -> WaffleSvgLayer {
    WaffleSvgLayer::new(key, kind).unwrap()
}

#[test]
fn portable_request_ceiling_does_not_allocate_oversubscribed_slots() {
    let limit = f64::from(u32::MAX);
    let frame = WaffleSvgFrame::new(
        &config(),
        &[RadialDatum::new("capacity", limit)],
        &options(1, 1, 1.),
    )
    .unwrap();
    assert_eq!(frame.cells().len(), 1);
    assert_eq!(frame.scene().marks.len(), 1);
    assert_eq!(frame.groups()[0].requested, 0..u32::MAX as usize);
    assert_eq!(frame.groups()[0].occupied, 0..1);
    assert_eq!(frame.areas().len(), 1);
    assert!(waffle_area_interaction(frame.layout()).is_ok());
    assert_eq!(
        waffle_cell_widgets(frame.layout(), 0, |_| Ok(None))
            .unwrap()
            .data
            .len(),
        0
    );
    assert!(
        waffle_layout(
            &config(),
            &[RadialDatum::new("capacity", limit)],
            &options(1, 1, 1.)
        )
        .is_err(),
        "the new SVG quota domain must not relax the old public layout domain"
    );
}

#[test]
fn individual_and_combined_quota_overflows_fail_before_style_or_cell_callbacks() {
    let limit = f64::from(u32::MAX);
    for source in [
        vec![RadialDatum::new("a", limit + 1.)],
        vec![RadialDatum::new("a", limit), RadialDatum::new("b", 1.)],
        vec![RadialDatum::new("a", f64::MAX)],
    ] {
        let calls = Cell::new(0);
        let mut color = |_: &WaffleGroupColorContext<'_>| {
            calls.set(calls.get() + 1);
            Ok(Color(1, 2, 3, 255))
        };
        let mut cell = |c: &WaffleCellContext<'_>| {
            calls.set(calls.get() + 1);
            Ok(c.inherit(c.geometry.clone()))
        };
        let result = WaffleSvgFrame::with_factories_and_cell_factory(
            &config(),
            &source,
            &options(1, 1, 1.),
            &mut WaffleFactories {
                color: Some(&mut color),
                ..Default::default()
            },
            &mut cell,
        );
        assert!(result.is_err());
        assert_eq!(calls.get(), 0);
    }
}

#[test]
fn hidden_huge_values_keep_source_styles_without_consuming_request_budget() {
    let source = [
        RadialDatum::new("hidden", f64::MAX),
        RadialDatum::new("visible", 1.),
    ];
    let mut opts = options(1, 1, 1.);
    opts.hidden.insert("hidden".into());
    let mut visited = vec![];
    let frame = WaffleSvgFrame::with_factories(
        &config(),
        &source,
        &opts,
        &mut WaffleFactories {
            color: Some(&mut |c: &WaffleGroupColorContext<'_>| {
                visited.push((c.source_index, c.datum.value.to_bits()));
                Ok(Color(4, 5, 6, 255))
            }),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(visited, vec![(0, f64::MAX.to_bits()), (1, 1_f64.to_bits())]);
    assert_eq!(frame.groups()[0].requested, 0..0);
    assert_eq!(frame.groups()[1].requested, 0..1);
    assert_eq!(frame.cells()[0].owner, Some(1));
    assert_eq!(frame.scene().legend.len(), 2);
    assert!(frame.group_by_id("hidden").is_some());
}

#[test]
fn corner_contact_provenance_survives_both_allocators_and_legacy_adapters() {
    let source = [
        RadialDatum::new("first", 2.),
        RadialDatum::new("corner", 3.),
    ];
    for direction in [WaffleFillDirection::Right, WaffleFillDirection::Left] {
        for allocation in [
            WaffleAllocation::IndependentRound,
            WaffleAllocation::LargestRemainder,
        ] {
            let opts = WaffleLayoutOptions {
                direction,
                allocation,
                ..options(3, 3, 9.)
            };
            let legacy = waffle_layout(&config(), &source, &opts).unwrap();
            let svg = WaffleSvgFrame::new(&config(), &source, &opts).unwrap();
            assert_eq!(legacy.groups[1].polygons.len(), 2);
            assert_eq!(svg.groups()[1].polygons.len(), 1);
            let polygon = &svg.groups()[1].polygons[0];
            assert!(
                polygon
                    .iter()
                    .enumerate()
                    .any(|(i, vertex)| polygon[i + 1..].contains(vertex)),
                "a corner-connected outline must retain its repeated contact vertex"
            );
            for layout in [&legacy, svg.layout()] {
                let calls = Cell::new(0);
                waffle_cell_widgets(layout, 0, |_| {
                    calls.set(calls.get() + 1);
                    Ok(None)
                })
                .unwrap();
                assert_eq!(calls.get(), 9);
                assert!(waffle_area_interaction(layout).is_ok());
            }
        }
    }
}

#[test]
fn repeated_gradient_cells_charge_stop_resources_and_keep_paint_definitions() {
    let source = [RadialDatum::new("a", 1.)];
    let calls = Cell::new(0);
    let paint = FillPaint::Linear {
        start: [30., 30.],
        end: [330., 330.],
        stops: vec![
            GradientStop::new(0., Color(1, 2, 3, 255)),
            GradientStop::new(0.5, Color(4, 5, 6, 120)),
            GradientStop::new(1., Color(7, 8, 9, 255)),
        ],
    };
    let frame =
        WaffleSvgFrame::with_cell_factory(&config(), &source, &options(1, 1, 1.), &mut |c| {
            calls.set(calls.get() + 1);
            let mut result = c.inherit(c.geometry.clone());
            result.fill_paint = Some(paint.clone());
            Ok(result)
        })
        .unwrap();
    let layers = vec![
        layer("one", WaffleSvgLayerKind::Cells),
        layer("two", WaffleSvgLayerKind::Cells),
    ];
    let plan = WaffleSvgPlan::new(&frame, layers.clone(), vec![], Default::default()).unwrap();
    let geometry_commands = frame.scene().marks[0].geometry.commands().len();
    assert_eq!(plan.source_commands(), 2 * (geometry_commands + 4));
    for i in 0..2 {
        assert_eq!(
            plan.native_scene(i).unwrap().marks[0].fill_paint,
            Some(paint.clone())
        );
    }
    assert!(
        WaffleSvgPlan::new(
            &frame,
            layers,
            vec![],
            WaffleSvgBudget {
                source_commands: plan.source_commands() - 1,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert_eq!(calls.get(), 1);
}

#[test]
fn vertical_legend_fixed_boxes_preserve_reverse_source_mapping_and_backgrounds() {
    let source = [RadialDatum::new("a", 1.), RadialDatum::new("b", 0.)];
    let frame = WaffleSvgFrame::new(&config(), &source, &options(1, 1, 1.)).unwrap();
    let mut spec = WaffleSvgLegendSpec {
        options: LegendOptions {
            direction: LegendDirection::Vertical,
            gap: 9.,
            symbol: LegendSymbol::None,
            label_prefix: "<".into(),
            label_suffix: ">".into(),
            series_labels: [("a".into(), "雪".into())].into_iter().collect(),
            ..Default::default()
        },
        layout: LegendLayoutOptions {
            direction: LegendDirection::Vertical,
            row_gap: 9.,
            ..Default::default()
        },
        ..Default::default()
    };
    let plan = WaffleSvgPlan::new(
        &frame,
        vec![layer("legend", WaffleSvgLayerKind::Legends)],
        vec![spec.clone()],
        Default::default(),
    )
    .unwrap();
    let legend = &plan.legends()[0];
    assert_eq!(legend.items()[0].source_index(), 1);
    assert_eq!(legend.items()[1].source_index(), 0);
    assert_eq!(legend.scene().labels[1].text, "<雪>");
    assert!(
        legend
            .items()
            .iter()
            .all(|row| row.symbol_index().is_none())
    );
    let first = legend.items()[0].rect();
    let second = legend.items()[1].rect();
    assert_eq!(second.y - first.y, first.height + 9.);
    let scene = plan.native_scene(0).unwrap();
    assert_eq!(scene.marks.len(), 2);
    assert_eq!(scene.labels.len(), 2);
    assert!(
        scene
            .marks
            .iter()
            .all(|m| m.fill == Some(Color(0, 0, 0, 0)))
    );
    assert_eq!(plan.native_hit_ids(0).unwrap().len(), 2);
    spec.layout.row_gap = 8.;
    assert!(spec.validate().is_err());
    spec.layout.row_gap = 9.;
    spec.item_size.height = 1.;
    assert!(spec.validate().is_err());
}
