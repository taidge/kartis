use std::cell::{Cell, RefCell};

use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 480.,
        height: 320.,
        ..Default::default()
    }
}

fn source() -> Vec<HierarchyDatum> {
    let mut raw = vec![
        HierarchyDatum::new("leaf", Some("branch".into()), 2.),
        HierarchyDatum::new("root", None, -0.),
        HierarchyDatum::new("branch", Some("root".into()), 0.),
        HierarchyDatum::new("other", Some("root".into()), -0.),
    ];
    raw[3].label.clear();
    raw
}

fn selected(visibility: TreeLabels, leaf: bool) -> bool {
    match visibility {
        TreeLabels::All => true,
        TreeLabels::Leaves => leaf,
        TreeLabels::Internal => !leaf,
        TreeLabels::None => false,
    }
}

#[test]
fn all_computed_calls_every_bfs_node_once_without_changing_scene_gates() {
    let c = config();
    let raw = source();
    for hierarchy_labels in [false, true] {
        for visibility in [
            TreeLabels::All,
            TreeLabels::Leaves,
            TreeLabels::Internal,
            TreeLabels::None,
        ] {
            let mut o = TreeOptions::default();
            o.hierarchy.labels = hierarchy_labels;
            o.style.labels.visibility = visibility;
            let old_calls = Cell::new(0);
            let mut old_label = |context: &TreeLabelFunctionContext<'_>| {
                old_calls.set(old_calls.get() + 1);
                Ok(format!("computed:{}", context.datum.id))
            };
            let old = tree_layout_with_factories(
                &c,
                &raw,
                &o,
                &mut TreeFactories {
                    label: Some(&mut old_label),
                    ..Default::default()
                },
            )
            .unwrap();
            assert!(old.computed_canvas_labels().is_none());
            let order = RefCell::new(Vec::new());
            let mut all_label = |context: &TreeLabelFunctionContext<'_>| {
                order.borrow_mut().push(context.source_index);
                assert_eq!(context.datum, &raw[context.source_index]);
                Ok(format!("computed:{}", context.datum.id))
            };
            let all = tree_canvas_layout_with_factories(
                &c,
                &raw,
                &o,
                &mut TreeFactories {
                    label: Some(&mut all_label),
                    ..Default::default()
                },
            )
            .unwrap();
            assert_eq!(*order.borrow(), [1, 2, 3, 0]);
            let pool = all.computed_canvas_labels().unwrap();
            assert_eq!(pool.len(), 4);
            assert_eq!(old, all);
            assert_eq!(format!("{old:?}"), format!("{all:?}"));
            assert_eq!(format!("{old:#?}"), format!("{all:#?}"));
            assert_eq!(
                render::to_svg(&old.scene).unwrap(),
                render::to_svg(&all.scene).unwrap()
            );
            let mut eligible = 0;
            for (entry, index) in pool.iter().zip([1, 2, 3, 0]) {
                assert_eq!(entry.source_index, index);
                let leaf = all.nodes[index].children.is_empty();
                let wanted = hierarchy_labels && selected(visibility, leaf);
                assert_eq!(entry.eligible, wanted);
                eligible += usize::from(wanted);
                assert_eq!(entry.label.text, format!("computed:{}", raw[index].id));
            }
            assert_eq!(old_calls.get(), eligible);
            assert_eq!(all.scene.labels.len(), eligible);
        }
    }
}

#[test]
fn hidden_empty_zero_transparent_rotated_labels_have_complete_constructor_data() {
    let mut c = config();
    c.theme.font.family = Some(vec![FontFamily::Mono]);
    c.theme.font.weight = Some(650.);
    let raw = source();
    let mut o = TreeOptions::default();
    o.hierarchy.labels = false;
    o.style.labels.visibility = TreeLabels::None;
    o.style.labels.rotation = TreeLabelRotation::Fixed(30.);
    o.style.labels.size = Some(18.);
    o.style.labels.color = Some(Color(12, 34, 56, 0));
    o.style.nodes.radius = Some(0.);
    o.style.nodes.opacity = 0.;
    o.style.clip_plot = true;
    let text = |id: &str| {
        if id == "other" {
            String::new()
        } else {
            format!("é叶:{id}")
        }
    };
    let mut label = |context: &TreeLabelFunctionContext<'_>| Ok(text(&context.datum.id));
    let all = tree_canvas_layout_with_factories(
        &c,
        &raw,
        &o,
        &mut TreeFactories {
            label: Some(&mut label),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(all.scene.labels.is_empty());
    let mut visible_options = o.clone();
    visible_options.hierarchy.labels = true;
    visible_options.style.labels.visibility = TreeLabels::All;
    let reference = tree_layout_with_factories(
        &c,
        &raw,
        &visible_options,
        &mut TreeFactories {
            label: Some(&mut label),
            ..Default::default()
        },
    )
    .unwrap();
    let pool = all.computed_canvas_labels().unwrap();
    assert_eq!(pool.len(), reference.scene.labels.len());
    for (index, entry) in pool.iter().enumerate() {
        assert!(!entry.eligible);
        assert_eq!(entry.label, reference.scene.labels[index]);
        assert_eq!(entry.font, reference.scene.resolved_label_font(index));
        assert_eq!(entry.font.family, [FontFamily::Mono]);
        assert_eq!(entry.font.weight, 650.);
        assert_eq!(entry.whitespace, LabelWhitespace::Legacy);
        assert_eq!(entry.clip.as_ref(), reference.scene.label_clips.get(&index));
        assert!(entry.clip.is_some());
        assert_eq!(entry.label.rotation, 30.);
        assert_eq!(entry.label.color.3, 0);
        assert_eq!(all.nodes[entry.source_index].radius, 0.);
    }
    assert!(pool.iter().any(|entry| entry.label.text.is_empty()));
    assert_eq!(all.source()[1].value.to_bits(), (-0f64).to_bits());
    assert_eq!(all.source()[3].value.to_bits(), (-0f64).to_bits());
}

#[test]
fn cached_text_is_not_replayed_or_rewritten_by_current_scene_edits() {
    let c = config();
    let calls = Cell::new(0);
    let mut label = |context: &TreeLabelFunctionContext<'_>| {
        calls.set(calls.get() + 1);
        Ok(format!("{}:{}", calls.get(), context.datum.id))
    };
    let mut l = tree_canvas_layout_with_factories(
        &c,
        &source(),
        &TreeOptions::default(),
        &mut TreeFactories {
            label: Some(&mut label),
            ..Default::default()
        },
    )
    .unwrap();
    let original = l.computed_canvas_labels().unwrap().to_vec();
    assert_eq!(
        original
            .iter()
            .map(|entry| entry.label.text.as_str())
            .collect::<Vec<_>>(),
        ["1:root", "2:branch", "3:other", "4:leaf"]
    );
    let cloned = l.clone();
    assert_eq!(
        l.computed_canvas_labels().unwrap().as_ptr(),
        cloned.computed_canvas_labels().unwrap().as_ptr()
    );
    l.scene.labels[0].text = "current Scene decoration".into();
    l.scene.labels[0].color = Color(255, 0, 0, 255);
    l.scene.typography.labels.insert(
        0,
        FontOptions {
            weight: Some(800.),
            ..Default::default()
        },
    );
    l.scene.validate().unwrap();
    for _ in 0..3 {
        assert_eq!(l.computed_canvas_labels().unwrap(), original);
        assert_eq!(cloned.computed_canvas_labels().unwrap(), original);
    }
    assert_eq!(calls.get(), 4);
    assert_eq!(l.scene.labels[0].text, "current Scene decoration");
}

#[test]
fn activity_cache_uses_final_sizes_and_complete_activity_even_with_labels_disabled() {
    let c = config();
    let raw = source();
    let mut o = TreeOptions::default();
    o.hierarchy.labels = false;
    o.style.labels.visibility = TreeLabels::None;
    o.style.labels.position = TreeLabelPosition::Below;
    o.style.labels.rotation = TreeLabelRotation::Fixed(0.);
    o.style.labels.size = Some(20.);
    o.style.labels.offset = 6.;
    let count = Cell::new(0);
    let mut label = |context: &TreeLabelFunctionContext<'_>| {
        count.set(count.get() + 1);
        let state = context.activity.unwrap();
        assert_eq!(
            context.node.radius,
            if state.nodes[context.source_index] == Some(true) {
                12.
            } else {
                0.
            }
        );
        Ok(format!(
            "{}:{:?}",
            context.datum.id, state.nodes[context.source_index]
        ))
    };
    let mut active = |_: &TreeNodeModifierContext<'_>| Ok(24.);
    let mut inactive = |_: &TreeNodeModifierContext<'_>| Ok(0.);
    let layout = tree_canvas_layout_with_activity(
        &c,
        &raw,
        &o,
        &mut TreeFactories {
            label: Some(&mut label),
            ..Default::default()
        },
        &TreeActivityOptions {
            focus_id: Some("leaf".into()),
            ..Default::default()
        },
        &mut TreeActivityFactories {
            active_node_size: Some(&mut active),
            inactive_node_size: Some(&mut inactive),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(count.get(), 4);
    assert!(layout.layout.scene.labels.is_empty());
    for entry in layout.layout.computed_canvas_labels().unwrap() {
        let node = &layout.layout.nodes[entry.source_index];
        assert!(!entry.eligible);
        assert_eq!(entry.label.x, node.position[0]);
        assert_eq!(entry.label.y, node.position[1] + (node.radius + 6.) + 16.);
        assert_eq!(
            entry.label.text,
            format!(
                "{}:{:?}",
                raw[entry.source_index].id, layout.activity.nodes[entry.source_index]
            )
        );
    }
    assert_eq!(count.get(), 4);
}

#[test]
fn optional_cache_has_no_effect_on_old_debug_eq_or_current_scene() {
    let c = config();
    let raw = source();
    let o = TreeOptions::default();
    let old = tree_layout(&c, &raw, &o).unwrap();
    let all = tree_canvas_layout(&c, &raw, &o).unwrap();
    assert!(old.computed_canvas_labels().is_none());
    assert_eq!(all.computed_canvas_labels().unwrap().len(), 4);
    assert_eq!(old, all);
    assert_eq!(format!("{old:?}"), format!("{all:?}"));
    assert_eq!(format!("{old:#?}"), format!("{all:#?}"));
    let mut disabled = o;
    disabled.hierarchy.labels = false;
    let build = |text: &str| {
        let mut label = |_: &TreeLabelFunctionContext<'_>| Ok(text.to_owned());
        tree_canvas_layout_with_factories(
            &c,
            &raw,
            &disabled,
            &mut TreeFactories {
                label: Some(&mut label),
                ..Default::default()
            },
        )
        .unwrap()
    };
    let a = build("unpainted first");
    let b = build("unpainted second");
    assert_eq!(a, b);
    assert_eq!(a.scene, b.scene);
    assert_eq!(format!("{a:?}"), format!("{b:?}"));
    assert_ne!(a.computed_canvas_labels(), b.computed_canvas_labels());
}

#[test]
fn empty_source_is_authoritative_empty_and_invalid_or_late_labels_return_no_layout() {
    let c = config();
    let o = TreeOptions::default();
    let empty = tree_canvas_layout(&c, &[], &o).unwrap();
    assert_eq!(empty.computed_canvas_labels(), Some([].as_slice()));
    let calls = Cell::new(0);
    let mut label = |_: &TreeLabelFunctionContext<'_>| {
        calls.set(calls.get() + 1);
        Ok("x".repeat(1025))
    };
    let mut hidden = o;
    hidden.hierarchy.labels = false;
    let old = tree_layout_with_factories(
        &c,
        &source(),
        &hidden,
        &mut TreeFactories {
            label: Some(&mut label),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(calls.get(), 0);
    assert!(old.computed_canvas_labels().is_none());
    assert!(
        tree_canvas_layout_with_factories(
            &c,
            &source(),
            &hidden,
            &mut TreeFactories {
                label: Some(&mut label),
                ..Default::default()
            },
        )
        .is_err()
    );
    assert_eq!(calls.get(), 1);
    calls.set(0);
    let mut bad = source();
    bad[0].value = f64::NAN;
    assert!(
        tree_canvas_layout_with_factories(
            &c,
            &bad,
            &hidden,
            &mut TreeFactories {
                label: Some(&mut label),
                ..Default::default()
            },
        )
        .is_err()
    );
    assert_eq!(calls.get(), 0);
    let mut late = |context: &TreeLabelFunctionContext<'_>| {
        calls.set(calls.get() + 1);
        if context.datum.id == "leaf" {
            Err(ChartError("last computed Canvas label rejected".into()))
        } else {
            Ok(context.datum.id.clone())
        }
    };
    assert!(
        tree_canvas_layout_with_factories(
            &c,
            &source(),
            &hidden,
            &mut TreeFactories {
                label: Some(&mut late),
                ..Default::default()
            },
        )
        .is_err()
    );
    assert_eq!(calls.get(), 4);
    calls.set(0);
    assert!(
        tree_canvas_layout_with_activity(
            &c,
            &[],
            &hidden,
            &mut TreeFactories {
                label: Some(&mut label),
                ..Default::default()
            },
            &TreeActivityOptions {
                focus_id: Some("absent".into()),
                ..Default::default()
            },
            &mut TreeActivityFactories::default(),
        )
        .is_err()
    );
    assert_eq!(calls.get(), 0);
}
