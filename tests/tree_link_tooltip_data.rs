use std::cell::Cell;

use kartis::*;

fn source(zero: f64) -> Vec<HierarchyDatum> {
    let mut rows = vec![
        HierarchyDatum::new("leaf", Some("branch".into()), zero),
        HierarchyDatum::new("root", None, -0.),
        HierarchyDatum::new("branch", Some("root".into()), 0.),
        HierarchyDatum::new("other", Some("root".into()), 5.),
    ];
    rows[1].label = "<root>&".into();
    rows[2].label = "<branch>".into();
    rows
}

fn build(config: &ChartConfig, zero: f64) -> TreeActivityLayout {
    tree_layout_with_activity(
        config,
        &source(zero),
        &TreeOptions::default(),
        &mut TreeFactories::default(),
        &TreeActivityOptions::default(),
        &mut TreeActivityFactories::default(),
    )
    .unwrap()
}

#[test]
fn two_endpoint_text_uses_business_labels_and_separate_computed_aggregates() {
    let c = ChartConfig::default();
    let layout = build(&c, -0.);
    let branch = TreeLinkTooltipData::new(c.clone(), layout.clone(), "branch").unwrap();
    assert_eq!(branch.link_index(), 0);
    assert_eq!((branch.source_index(), branch.target_index()), (1, 2));
    assert_eq!(branch.source_datum().value.to_bits(), (-0_f64).to_bits());
    assert_eq!(branch.source_node().value, 5.);
    assert_eq!(branch.target_node().value, 0.);
    assert_eq!(
        branch.text(&TooltipOptions::default()).unwrap(),
        "<root>&: 5\n<branch>: 0"
    );
    let leaf = TreeLinkTooltipData::new(c, layout, "leaf").unwrap();
    assert_eq!(leaf.link_index(), 2);
    assert_eq!((leaf.source_index(), leaf.target_index()), (2, 0));
    assert_eq!(leaf.target_datum().value.to_bits(), (-0_f64).to_bits());
    let template = TooltipOptions {
        lines: vec![vec![TooltipPart::Id]],
        heading: Some("Endpoints".into()),
        ..Default::default()
    };
    assert_eq!(leaf.text(&template).unwrap(), "Endpoints\nbranch\nleaf");
    assert_eq!(leaf.config().theme, *leaf.theme());
}

#[test]
fn cloned_or_selected_data_shares_one_pool_and_equality_does_not_hide_raw_zero_changes() {
    let c = ChartConfig::default();
    let frame = TreeEventFrame::with_activity(c.clone(), build(&c, -0.)).unwrap();
    let data = TreeLinkTooltipData::from_link(frame.link(2).unwrap());
    let same: TreeLinkTooltipData = frame.link(2).unwrap().into();
    let cloned = data.clone();
    assert_eq!(data, same);
    assert_eq!(data, cloned);
    assert!(std::ptr::eq(data.frame().scene(), frame.scene()));
    assert!(std::ptr::eq(data.style(), same.style()));
    assert!(std::ptr::eq(data.source(), frame.source()));
    assert!(std::ptr::eq(
        data.link_data().target_node(),
        &frame.nodes()[0]
    ));
    assert_ne!(data, TreeLinkTooltipData::from_link(frame.link(0).unwrap()));
    let positive = TreeLinkTooltipData::new(c.clone(), build(&c, 0.), "leaf").unwrap();
    assert_eq!(
        data.text(&TooltipOptions::default()).unwrap(),
        positive.text(&TooltipOptions::default()).unwrap()
    );
    assert_ne!(
        data.target_datum().value.to_bits(),
        positive.target_datum().value.to_bits()
    );
    assert_ne!(
        data, positive,
        "reactive equality cannot hide source changes"
    );
    drop(frame);
    drop(data);
    assert_eq!(cloned.target_datum().id, "leaf");
    assert_eq!(cloned.source().len(), 4);
    assert!(std::ptr::eq(cloned.source(), same.source()));
}

#[test]
fn disabled_link_keeps_full_style_and_activity_without_fabricating_paint_or_replaying_functions() {
    let c = ChartConfig::default();
    let mut options = TreeOptions::default();
    options.style.links.enabled = false;
    let calls = Cell::new(0);
    let mut width = |ctx: &TreeLinkFunctionContext<'_>| {
        calls.set(calls.get() + 1);
        Ok(ctx.link.link_index as f64 + 2.)
    };
    let mut color = |_: &TreeLinkFunctionContext<'_>| {
        calls.set(calls.get() + 1);
        Ok(Color(12, 34, 56, 0))
    };
    let mut inactive = |_: &TreeLinkModifierContext<'_>| Ok(-0.);
    let layout = tree_layout_with_activity(
        &c,
        &source(-0.),
        &options,
        &mut TreeFactories {
            link_thickness: Some(&mut width),
            link_color: Some(&mut color),
            ..Default::default()
        },
        &TreeActivityOptions {
            focus_id: Some("leaf".into()),
            ..Default::default()
        },
        &mut TreeActivityFactories {
            inactive_link_thickness: Some(&mut inactive),
            ..Default::default()
        },
    )
    .unwrap();
    let before = calls.get();
    assert_eq!(before, 6);
    let scene = layout.layout.scene.clone();
    let selected = TreeLinkTooltipData::new(c.clone(), layout.clone(), "leaf").unwrap();
    let sibling = TreeLinkTooltipData::new(c, layout, "other").unwrap();
    assert_eq!(selected.is_active(), Some(true));
    assert_eq!(selected.style().thickness, 4.);
    assert_eq!(sibling.is_active(), Some(false));
    assert_eq!(sibling.style().thickness.to_bits(), (-0_f64).to_bits());
    assert_eq!(selected.style().color, Color(12, 34, 56, 0));
    assert_eq!(
        selected.activity().unwrap().focus_id.as_deref(),
        Some("leaf")
    );
    assert!(selected.mark().is_none());
    assert!(sibling.mark().is_none());
    assert_eq!(selected.layout().scene, scene);
    assert!(
        !selected
            .text(&TooltipOptions::default())
            .unwrap()
            .is_empty()
    );
    assert_eq!(calls.get(), before);
}

#[test]
fn source_ids_and_capture_validation_cannot_invent_a_root_link_or_invalid_tooltip() {
    let c = ChartConfig::default();
    let layout = build(&c, -0.);
    for invalid in ["root", "link:leaf", "missing"] {
        assert!(TreeLinkTooltipData::new(c.clone(), layout.clone(), invalid).is_err());
    }
    let mut wrong_config = c.clone();
    wrong_config.margin.left += 1.;
    assert!(TreeLinkTooltipData::new(wrong_config, layout.clone(), "leaf").is_err());
    let mut wrong_capture = layout.clone();
    wrong_capture.layout.nodes[0].source_index = 3;
    assert!(TreeLinkTooltipData::new(c.clone(), wrong_capture, "leaf").is_err());
    let data = TreeLinkTooltipData::new(c, layout, "leaf").unwrap();
    let invalid = TooltipOptions {
        max_rows: 0,
        ..Default::default()
    };
    assert!(data.text(&invalid).is_err());
    assert_eq!(
        data.text(&TooltipOptions::default()).unwrap(),
        "<branch>: 0\nleaf: 0"
    );
}
