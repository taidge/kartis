use kartis::*;
fn config() -> ChartConfig {
    ChartConfig {
        width: 320.,
        height: 180.,
        margin: Margin {
            left: 20.,
            right: 20.,
            top: 20.,
            bottom: 20.,
        },
        ..Default::default()
    }
}
fn options() -> HeatmapLayoutOptions {
    HeatmapLayoutOptions {
        axes: HeatmapAxes::none(),
        labels: true,
        hover_target: HeatmapHoverTarget::Cell,
        ..Default::default()
    }
}
fn data() -> Vec<HeatCell> {
    vec![HeatCell::new("a", "x", 1.), HeatCell::new("b", "x", 2.)]
}
#[test]
fn custom_binding_rejects_owned_label_reassigned_to_another_existing_row() {
    let mut layout = heatmap_layout_with_cell_factory(&config(), &data(), &options(), |ctx| {
        Ok(ctx.inherit(ctx.geometry.clone()))
    })
    .unwrap();
    assert_eq!(layout.scene.label_series.get(&0), Some(&"a".to_string()));
    layout.scene.label_series.insert(0, "b".into());
    let scene = layout.scene.clone();
    let cells = layout.cells.clone();
    let active = layout.active.clone();
    assert!(
        layout.interaction_binding().is_err(),
        "v1 accepts a real owned label reassigned a→b"
    );
    assert_eq!(layout.scene, scene);
    assert_eq!(layout.cells, cells);
    assert_eq!(layout.active, active);
}
#[test]
fn custom_hover_rejects_the_same_owned_label_corruption_atomically() {
    let mut layout = heatmap_layout_with_cell_factory(&config(), &data(), &options(), |ctx| {
        Ok(ctx.inherit(ctx.geometry.clone()))
    })
    .unwrap();
    layout.scene.label_series.insert(0, "b".into());
    let scene = layout.scene.clone();
    let cells = layout.cells.clone();
    let active = layout.active.clone();
    assert!(
        layout
            .set_active(Some(HeatmapCellKey::new("a", "x")))
            .is_err()
    );
    assert_eq!(layout.scene, scene);
    assert_eq!(layout.cells, cells);
    assert_eq!(layout.active, active);
}
#[test]
fn default_binding_rejects_null_forged_as_zero_despite_legacy_mark_zero() {
    let cells = [HeatCell {
        row: "a".into(),
        column: "x".into(),
        value: None,
    }];
    let mut layout = heatmap_layout(&config(), &cells, &options()).unwrap();
    assert_eq!(layout.cells[0].value, None);
    assert_eq!(layout.scene.marks[0].value, 0.);
    layout.cells[0].value = Some(0.);
    let scene = layout.scene.clone();
    let metadata = layout.cells.clone();
    assert!(
        layout.interaction_binding().is_err(),
        "v1 accepts None→Some0 because Scene scalar compatibility is0"
    );
    assert_eq!(layout.scene, scene);
    assert_eq!(layout.cells, metadata);
}
#[test]
fn legitimate_default_hover_and_scene_decorations_keep_new_binding_available() {
    let cells = [
        HeatCell {
            row: "a".into(),
            column: "x".into(),
            value: None,
        },
        HeatCell::new("b", "x", 0.),
    ];
    let mut layout = heatmap_layout(&config(), &cells, &options()).unwrap();
    layout.scene.marks[0].fill = Some(Color(200, 30, 40, 255));
    layout.scene.labels[0].size = 18.;
    layout.scene.marks[0].clip = Some(layout.plot);
    layout.scene.legend.push(LegendItem {
        id: "decorative".into(),
        label: "Extra display legend".into(),
        color: Color(40, 50, 60, 255),
    });
    let binding = layout.interaction_binding().unwrap();
    assert_eq!(
        binding.sites.iter().map(|s| s.value).collect::<Vec<_>>(),
        vec![None, Some(0.)]
    );
    layout
        .set_active(Some(HeatmapCellKey::new("a", "x")))
        .unwrap();
    layout.interaction_binding().unwrap();
    assert_eq!(layout.scene.labels[0].size, 18.);
    assert_eq!(layout.scene.marks[0].fill, Some(Color(200, 30, 40, 255)));
    assert_eq!(layout.scene.marks[0].clip, Some(layout.plot));
}
