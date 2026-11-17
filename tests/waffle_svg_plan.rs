use std::cell::Cell;

use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 500.,
        height: 340.,
        margin: Margin {
            left: 41.,
            right: 31.,
            top: 23.,
            bottom: 37.,
        },
        ..Default::default()
    }
}
fn options() -> WaffleLayoutOptions {
    WaffleLayoutOptions {
        rows: 2,
        columns: 2,
        total: Some(100.),
        padding: 8.,
        ..Default::default()
    }
}
fn source() -> Vec<RadialDatum> {
    vec![
        RadialDatum::new("a", 50.),
        RadialDatum::new("b", 25.),
        RadialDatum::new("zero", -0.),
    ]
}
fn layer(key: &str, kind: WaffleSvgLayerKind) -> WaffleSvgLayer {
    WaffleSvgLayer::new(key, kind).unwrap()
}
fn plan(frame: &WaffleSvgFrame, layers: Vec<WaffleSvgLayer>) -> WaffleSvgPlan {
    WaffleSvgPlan::new(frame, layers, vec![], Default::default()).unwrap()
}

#[test]
fn literal_program_omits_repeats_and_interleaves_without_rebuilding_source_roles() {
    use WaffleSvgLayerKind::*;
    let frame = WaffleSvgFrame::new(&config(), &source(), &options()).unwrap();
    let default = WaffleSvgPlan::defaults(&frame).unwrap();
    assert_eq!(default.frame(), &frame);
    assert_eq!(
        default
            .layers()
            .iter()
            .map(|l| &l.layer.kind)
            .collect::<Vec<_>>(),
        vec![&Cells, &Areas, &Legends]
    );
    assert!(
        default.layers()[2].items().is_empty(),
        "source legend metadata does not invent configured legends"
    );
    assert!(default.legends().is_empty());
    let empty = plan(&frame, vec![]);
    assert!(empty.layers().is_empty());
    assert_eq!(empty.work(), 0);
    assert_eq!(empty.source_commands(), 0);
    assert_eq!(empty.frame(), &frame);
    let ordered = plan(
        &frame,
        vec![
            layer("under", Custom("application".into())),
            layer("regions-first", Areas),
            layer("paint", Cells),
            layer("regions-again", Areas),
            layer("over", Custom("application".into())),
        ],
    );
    assert_eq!(
        ordered
            .layers()
            .iter()
            .map(|l| l.layer.key.as_str())
            .collect::<Vec<_>>(),
        vec!["under", "regions-first", "paint", "regions-again", "over"]
    );
    assert_eq!(
        ordered.layers().iter().map(|l| l.index).collect::<Vec<_>>(),
        vec![0, 1, 2, 3, 4]
    );
    assert_eq!(ordered.layers()[1].items(), ordered.layers()[3].items());
    assert_eq!(ordered.layers()[2].items().len(), 4);
    assert_eq!(ordered.layers()[1].items().len(), 2);
    assert_eq!(ordered.work(), 10);
    for i in [0, 4] {
        assert!(ordered.layers()[i].items().is_empty());
    }
    let areas = ordered.native_scene(1).unwrap();
    assert_eq!((areas.width, areas.height), (500., 340.));
    assert_eq!(areas.marks.len(), 2);
    assert!(
        areas
            .marks
            .iter()
            .all(|m| m.fill == Some(Color(0, 0, 0, 0)))
    );
    assert_eq!(
        ordered.native_hit_ids(1).unwrap().len(),
        2,
        "transparent Areas remain native input/occlusion leaves"
    );
    assert_eq!(
        ordered.native_hit_ids(2).unwrap().len(),
        4,
        "Cells are not made passive because they lack group handlers"
    );
    assert!(ordered.native_hit_ids(0).unwrap().is_empty());
    assert!(ordered.native_scene(5).is_err());
    assert!(ordered.native_hit_ids(5).is_err());
    assert!(
        frame
            .scene()
            .marks
            .iter()
            .all(|m| !m.id.starts_with("waffle-svg-area:")),
        "transparent area leaves do not become canonical source Marks"
    );
}

#[test]
fn repeated_native_and_custom_occurrences_consume_real_work_and_resource_budgets() {
    use WaffleSvgLayerKind::*;
    let frame = WaffleSvgFrame::new(&config(), &source(), &options()).unwrap();
    let once = plan(&frame, vec![layer("once", Cells)]);
    let twice_layers = vec![layer("first", Cells), layer("second", Cells)];
    let twice = plan(&frame, twice_layers.clone());
    assert_eq!(twice.work(), once.work() * 2);
    assert_eq!(twice.source_commands(), once.source_commands() * 2);
    for budget in [
        WaffleSvgBudget {
            layers: 1,
            ..Default::default()
        },
        WaffleSvgBudget {
            work: twice.work() - 1,
            ..Default::default()
        },
        WaffleSvgBudget {
            source_commands: twice.source_commands() - 1,
            ..Default::default()
        },
    ] {
        assert!(WaffleSvgPlan::new(&frame, twice_layers.clone(), vec![], budget).is_err());
    }
    assert!(
        WaffleSvgPlan::new(
            &frame,
            twice_layers,
            vec![],
            WaffleSvgBudget {
                layers: 2,
                work: twice.work(),
                source_commands: twice.source_commands(),
                components: 0,
            }
        )
        .is_ok()
    );
    let custom = vec![
        layer("a", Custom("shared".into())),
        layer("b", Custom("shared".into())),
    ];
    assert!(
        WaffleSvgPlan::validate_layers(
            &custom,
            WaffleSvgBudget {
                components: 1,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        WaffleSvgPlan::validate_layers(
            &custom,
            WaffleSvgBudget {
                components: 2,
                ..Default::default()
            }
        )
        .is_ok()
    );
    assert!(
        WaffleSvgPlan::new(
            &frame,
            vec![layer("same", Cells), layer("same", Areas)],
            vec![],
            Default::default()
        )
        .is_err()
    );
    assert!(WaffleSvgLayer::new(" ", Cells).is_err());
    assert!(WaffleSvgLayer::new("x", Custom("".into())).is_err());
    assert!(
        WaffleSvgBudget {
            work: usize::MAX,
            ..Default::default()
        }
        .validate()
        .is_err()
    );
}

#[test]
fn actual_custom_cell_ink_is_captured_once_while_group_area_keeps_full_slot_geometry() {
    use WaffleSvgLayerKind::*;
    let calls = Cell::new(0);
    let base = WaffleSvgFrame::new(&config(), &source(), &options()).unwrap();
    let custom = WaffleSvgFrame::with_cell_factory(&config(), &source(), &options(), &mut |c| {
        calls.set(calls.get() + 1);
        Ok(c.inherit(Geometry::Circle {
            x: c.position[0],
            y: c.position[1],
            radius: 3.,
        }))
    })
    .unwrap();
    assert_eq!(calls.get(), 4);
    assert_eq!(custom.groups(), base.groups());
    assert_eq!(
        custom
            .areas()
            .iter()
            .map(|a| a.geometry())
            .collect::<Vec<_>>(),
        base.areas()
            .iter()
            .map(|a| a.geometry())
            .collect::<Vec<_>>()
    );
    for i in 0..4 {
        assert!(matches!(
            custom.cell_mark(i).unwrap().unwrap().geometry,
            Geometry::Circle { radius: 3., .. }
        ));
    }
    let repeated = plan(
        &custom,
        vec![
            layer("cells-1", Cells),
            layer("areas", Areas),
            layer("cells-2", Cells),
        ],
    );
    for i in [0, 2] {
        let scene = repeated.native_scene(i).unwrap();
        assert!(
            scene
                .marks
                .iter()
                .all(|m| matches!(m.geometry, Geometry::Circle { radius: 3., .. }))
        );
    }
    assert_eq!(
        calls.get(),
        4,
        "plans/getters/repeated layers never replay the application cell function"
    );
    let area_mode = WaffleSvgFrame::new(
        &config(),
        &source(),
        &WaffleLayoutOptions {
            mode: WaffleMode::Areas,
            ..options()
        },
    )
    .unwrap();
    assert!(WaffleSvgPlan::defaults(&area_mode).is_err());
    let mut editable = base.layout().clone();
    editable.cells[0].bounds.x += 1.;
    let attempted = Cell::new(0);
    assert!(
        waffle_cell_widgets(&editable, 0, |_| {
            attempted.set(attempted.get() + 1);
            Ok(None)
        })
        .is_err()
    );
    assert_eq!(
        attempted.get(),
        0,
        "public edited metadata is not silently repaired into source authority"
    );
}

#[test]
fn explicitly_measured_legends_keep_outer_scene_dimensions_and_literal_identity() {
    use WaffleSvgLayerKind::*;
    let frame = WaffleSvgFrame::new(&config(), &source(), &options()).unwrap();
    let spec = WaffleSvgLegendSpec::new(
        "outer",
        LegendOptions::default(),
        LegendLayoutOptions {
            bounds: LayoutRect {
                x: 0.,
                y: 0.,
                width: 500.,
                height: 340.,
            },
            ..Default::default()
        },
        TextSize {
            width: 120.,
            height: 24.,
        },
    )
    .unwrap();
    let p = WaffleSvgPlan::new(
        &frame,
        vec![
            layer("legend-before", Legends),
            layer("native", Cells),
            layer("legend-after", Legends),
        ],
        vec![spec.clone()],
        Default::default(),
    )
    .unwrap();
    assert_eq!(p.layers()[0].items(), p.layers()[2].items());
    assert_eq!(p.layers()[0].items().len(), source().len());
    assert_eq!(
        p.legends()[0].items().len(),
        source().len(),
        "zero/hidden source legend records are not allocation-filtered"
    );
    for occurrence in [0, 2] {
        let scene = p.native_scene(occurrence).unwrap();
        assert_eq!((scene.width, scene.height), (500., 340.));
        assert_eq!(scene.labels.len(), 3);
        assert_eq!(
            p.native_hit_ids(occurrence).unwrap().len(),
            3,
            "only real legend item backgrounds get native eligibility"
        );
    }
    assert!(
        WaffleSvgPlan::new(
            &frame,
            vec![layer("legends", Legends)],
            vec![spec.clone(), spec],
            Default::default()
        )
        .is_err()
    );
}
