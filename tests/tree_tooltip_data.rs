use std::cell::Cell;

use kartis::*;

fn source(zero: f64) -> Vec<HierarchyDatum> {
    let mut data = vec![
        HierarchyDatum::new("empty", Some("root".into()), zero),
        HierarchyDatum::new("root", None, -0.),
        HierarchyDatum::new("leaf", Some("root".into()), 5.),
    ];
    data[0].label.clear();
    data[1].label = "<root>&".into();
    data
}

fn build(c: &ChartConfig, raw: &[HierarchyDatum], o: &TreeOptions) -> TreeActivityLayout {
    tree_layout_with_activity(
        c,
        raw,
        o,
        &mut TreeFactories::default(),
        &TreeActivityOptions::default(),
        &mut TreeActivityFactories::default(),
    )
    .unwrap()
}

#[test]
fn data_preserves_unpaintable_nodes_source_bits_and_full_original_pool() {
    let c = ChartConfig::default();
    let raw = source(-0.);
    let mut options = TreeOptions::default();
    options.style.nodes.radius = Some(0.);
    options.style.nodes.opacity = 0.;
    options.hierarchy.labels = false;
    let layout = build(&c, &raw, &options);
    let data = TreeTooltipData::new(c.clone(), layout.clone(), "empty").unwrap();
    assert_eq!(data.source_index(), 0);
    assert_eq!(data.datum().label, "");
    assert_eq!(data.datum().value.to_bits(), (-0_f64).to_bits());
    assert_eq!(data.node().raw_value.to_bits(), (-0_f64).to_bits());
    assert_eq!(data.node().radius, 0.);
    assert_eq!(data.anchor(), layout.layout.nodes[0].position);
    assert_eq!(data.source(), raw);
    assert_eq!(data.activity().focus_id, None);
    assert_eq!(data.is_active(), None);
    assert_eq!(data.activity_layout(), &layout);
    assert_eq!(data.config(), &c);
    let root = TreeTooltipData::new(c.clone(), layout, "root").unwrap();
    assert_eq!(root.datum().value.to_bits(), (-0_f64).to_bits());
    assert_eq!(root.node().value, 5.);
    assert_eq!(root.text(&TooltipOptions::default()).unwrap(), "<root>&: 5");
    let template = TooltipOptions {
        lines: vec![vec![TooltipPart::Id]],
        ..Default::default()
    };
    assert_eq!(data.text(&template).unwrap(), "empty");
    let positive_root =
        TreeTooltipData::new(c.clone(), build(&c, &source(0.), &options), "root").unwrap();
    assert_eq!(
        root.datum().value.to_bits(),
        positive_root.datum().value.to_bits()
    );
    assert_ne!(
        root, positive_root,
        "An unselected sibling's raw bits remain observable"
    );
    let positive =
        TreeTooltipData::new(c.clone(), build(&c, &source(0.), &options), "empty").unwrap();
    assert_ne!(data, positive, "Raw zero bits are observable typed source");
}

#[test]
fn fixed_anchor_is_the_computed_center_not_edited_path_bounds() {
    let c = ChartConfig::default();
    let mut layout = build(&c, &source(-0.), &TreeOptions::default());
    let original = layout.layout.nodes[2].position;
    let id = layout.layout.nodes[2].mark_id.clone().unwrap();
    let mark = layout
        .layout
        .scene
        .marks
        .iter_mut()
        .find(|mark| mark.id == id)
        .unwrap();
    mark.geometry = Geometry::Path(vec![
        glory::render::PathCommand::Move(glory::render::GraphicPoint::new(900., 20.)),
        glory::render::PathCommand::Line(glory::render::GraphicPoint::new(930., 20.)),
        glory::render::PathCommand::Line(glory::render::GraphicPoint::new(915., 40.)),
        glory::render::PathCommand::Close,
    ]);
    let data = TreeTooltipData::new(c, layout, "leaf").unwrap();
    assert_eq!(data.anchor(), original);
    assert_ne!(data.anchor(), [915., 30.]);
}

#[test]
fn activity_data_uses_final_nodes_without_replaying_factories() {
    let c = ChartConfig::default();
    let raw = source(-0.);
    let sizes = Cell::new(0);
    let modifiers = Cell::new(0);
    let mut size = |_: &TreeNodeFunctionContext<'_>| {
        sizes.set(sizes.get() + 1);
        Ok(10.)
    };
    let mut active = |_: &TreeNodeModifierContext<'_>| {
        modifiers.set(modifiers.get() + 1);
        Ok(30.)
    };
    let layout = tree_layout_with_activity(
        &c,
        &raw,
        &TreeOptions::default(),
        &mut TreeFactories {
            node_size: Some(&mut size),
            ..Default::default()
        },
        &TreeActivityOptions {
            focus_id: Some("leaf".into()),
            ..Default::default()
        },
        &mut TreeActivityFactories {
            active_node_size: Some(&mut active),
            ..Default::default()
        },
    )
    .unwrap();
    let before = (sizes.get(), modifiers.get());
    let selected = TreeTooltipData::new(c.clone(), layout.clone(), "leaf").unwrap();
    let sibling = TreeTooltipData::new(c.clone(), layout.clone(), "empty").unwrap();
    assert_eq!(selected.is_active(), Some(true));
    assert_eq!(selected.node().radius, 15.);
    assert_eq!(selected.anchor(), layout.layout.nodes[2].position);
    assert_eq!(sibling.is_active(), Some(false));
    assert_eq!(sibling.node().radius, 5.);
    assert_eq!(
        selected.text(&TooltipOptions::default()).unwrap(),
        "leaf: 5"
    );
    assert_eq!(selected, selected.clone());
    assert_eq!((sizes.get(), modifiers.get()), before);
}

#[test]
fn invalid_config_capture_or_source_id_cannot_manufacture_tooltip_data() {
    let c = ChartConfig::default();
    let layout = build(&c, &source(-0.), &TreeOptions::default());
    assert!(TreeTooltipData::new(c.clone(), layout.clone(), "node:leaf").is_err());
    assert!(TreeTooltipData::new(c.clone(), layout.clone(), "unknown").is_err());
    let mut config = c.clone();
    config.margin.left += 1.;
    assert!(TreeTooltipData::new(config, layout.clone(), "root").is_err());
    let mut edited = layout.clone();
    edited.layout.nodes[0].raw_value = 3.;
    assert!(TreeTooltipData::new(c.clone(), edited, "empty").is_err());
    let other = build(&c, &source(0.), &TreeOptions::default());
    let mut grafted = layout;
    grafted.layout = other.layout;
    assert!(TreeTooltipData::new(c, grafted, "root").is_err());
}

#[test]
fn equality_observes_complete_computed_caches_omitted_from_legacy_layout_equality() {
    let c = ChartConfig::default();
    let raw = source(-0.);
    let mut options = TreeOptions::default();
    options.style.links.enabled = false;
    options.hierarchy.labels = false;
    let make = |text: &str, width: f64| {
        let mut label = |_: &TreeLabelFunctionContext<'_>| Ok(text.to_owned());
        let mut thickness = |_: &TreeLinkFunctionContext<'_>| Ok(width);
        tree_canvas_layout_with_activity(
            &c,
            &raw,
            &options,
            &mut TreeFactories {
                label: Some(&mut label),
                link_thickness: Some(&mut thickness),
                ..Default::default()
            },
            &TreeActivityOptions::default(),
            &mut TreeActivityFactories::default(),
        )
        .unwrap()
    };
    let first = make("one", 3.);
    let labels_changed = make("two", 3.);
    let links_changed = make("one", 4.);
    assert_eq!(first, labels_changed);
    assert_eq!(first, links_changed);
    let first = TreeTooltipData::new(c.clone(), first, "root").unwrap();
    assert_ne!(
        first,
        TreeTooltipData::new(c.clone(), labels_changed, "root").unwrap()
    );
    assert_ne!(
        first,
        TreeTooltipData::new(c, links_changed, "root").unwrap()
    );
}
