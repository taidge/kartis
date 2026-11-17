use std::cell::Cell;
use std::collections::BTreeMap;
use std::sync::Arc;

use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 520.,
        height: 360.,
        margin: Margin {
            left: 32.,
            right: 24.,
            top: 20.,
            bottom: 28.,
        },
        ..Default::default()
    }
}

fn source() -> Vec<HierarchyDatum> {
    let mut source = vec![
        HierarchyDatum::new("zero", Some("branch".into()), -0.),
        HierarchyDatum::new("root", None, -0.),
        HierarchyDatum::new("branch", Some("root".into()), 0.),
        HierarchyDatum::new("positive", Some("branch".into()), 2.),
    ];
    source[0].label.clear();
    source
}

fn placement() -> PlotWidgetPlacement {
    PlotWidgetPlacement::AtDatum {
        offset: [-9., -5.],
        measured: TextSize {
            width: 18.,
            height: 10.,
        },
    }
}

type PoolRefs<'a> = (
    &'a Arc<Vec<HierarchyDatum>>,
    &'a Arc<Vec<TreeNode>>,
    &'a Arc<Vec<TreeLink>>,
    &'a Arc<TreeWidgetLabels>,
);

fn pools(data: &TreeComponentData) -> PoolRefs<'_> {
    match data {
        TreeComponentData::Node(d) => (&d.source, &d.nodes, &d.links, &d.labels),
        TreeComponentData::Link(d) => (&d.source, &d.nodes, &d.links, &d.labels),
        TreeComponentData::Label(d) => (&d.source, &d.nodes, &d.links, &d.labels),
    }
}

#[test]
fn actual_scene_items_drive_three_roles_with_one_shared_original_source() {
    let mut raw = source();
    let mut layout = tree_layout(&config(), &raw, &TreeOptions::default()).unwrap();
    let annotation = layout.scene.labels[0].clone();
    layout.scene.labels.push(annotation);
    let mut order = layout.scene.draw_items();
    let first_label = order
        .iter()
        .position(|item| matches!(item, DrawItem::Label(0)))
        .unwrap();
    let label = order.remove(first_label);
    order.insert(0, label);
    layout.scene.draw_order = order;
    let before = layout.clone();
    let mut calls = vec![];
    let packet = tree_components(&layout, 17, |c| {
        let key = match c {
            TreeComponentContext::Node(c) => {
                assert!(std::ptr::eq(c.datum, &layout.source()[c.source_index]));
                c.mark.id.clone()
            }
            TreeComponentContext::Link(c) => {
                assert!(std::ptr::eq(
                    c.source_datum,
                    &layout.source()[c.link.source_index]
                ));
                c.mark.id.clone()
            }
            TreeComponentContext::Label(c) => {
                assert!(std::ptr::eq(c.label, &layout.scene.labels[c.label_index]));
                assert!(std::ptr::eq(c.node, &layout.nodes[c.source_index]));
                assert!(std::ptr::eq(c.datum, &layout.source()[c.source_index]));
                format!("tree-label:{}", c.datum.id)
            }
        };
        calls.push(key);
        Ok(Some(placement()))
    })
    .unwrap();
    let expected = layout
        .scene
        .draw_items()
        .into_iter()
        .filter_map(|item| match item {
            DrawItem::Mark(index) => Some(layout.scene.marks[index].id.clone()),
            DrawItem::Label(index) => layout
                .scene
                .label_series
                .get(&index)
                .map(|owner| format!("tree-label:{owner}")),
        })
        .collect::<Vec<_>>();
    assert_eq!(calls, expected);
    assert_eq!(
        packet
            .layout
            .slots
            .iter()
            .map(|slot| slot.key.clone())
            .collect::<Vec<_>>(),
        expected
    );
    assert_eq!(packet.data.len(), 11);
    assert_eq!(layout, before);
    let first = pools(packet.data.values().next().unwrap());
    for data in packet.data.values() {
        let current = pools(data);
        assert!(Arc::ptr_eq(first.0, current.0));
        assert!(Arc::ptr_eq(first.1, current.1));
        assert!(Arc::ptr_eq(first.2, current.2));
        assert!(Arc::ptr_eq(first.3, current.3));
        assert_eq!(current.0[0].value.to_bits(), (-0_f64).to_bits());
    }
    raw[0].value = 99.;
    assert_eq!(first.0[0].value.to_bits(), (-0_f64).to_bits());
    assert_eq!(tree_widget_frame(&layout).unwrap().scene, layout.scene);
}

#[test]
fn empty_function_text_zero_radius_transparent_node_and_label_remain_real_roles() {
    let calls = Cell::new(0);
    let mut size = |_: &TreeNodeFunctionContext<'_>| {
        calls.set(calls.get() + 1);
        Ok(0.)
    };
    let mut label = |_: &TreeLabelFunctionContext<'_>| {
        calls.set(calls.get() + 1);
        Ok(String::new())
    };
    let mut options = TreeOptions::default();
    options.style.nodes.opacity = 0.;
    options.style.nodes.fill = TreeNodeFill::None;
    options.style.labels.color = Some(Color(12, 34, 56, 0));
    let layout = tree_layout_with_factories(
        &config(),
        &source(),
        &options,
        &mut TreeFactories {
            node_size: Some(&mut size),
            label: Some(&mut label),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(layout.scene.labels.len(), 4);
    assert!(layout.scene.labels.iter().all(|l| l.text.is_empty()));
    let before_calls = calls.get();
    let packet = tree_label_widgets(&layout, 0, |c| {
        assert_eq!(c.node.radius, 0.);
        assert!(c.label.text.is_empty());
        assert_eq!(c.label.color.3, 0);
        Ok(Some(placement()))
    })
    .unwrap();
    assert_eq!(
        calls.get(),
        before_calls,
        "adapters do not replay functions"
    );
    assert_eq!(packet.data.len(), 4);
    assert_eq!(packet.data["tree-label:zero"].node.radius, 0.);
    assert!(packet.data["tree-label:zero"].source[0].label.is_empty());
    assert_eq!(
        tree_widget_frame(&layout)
            .unwrap()
            .interaction
            .unwrap()
            .sites
            .len(),
        3
    );
    for slot in packet.layout.slots {
        assert!(matches!(slot.target, SceneWidgetTarget::Label(_)));
        assert_eq!(
            slot.placement,
            placement(),
            "real label anchor is left to its host"
        );
    }
}

#[test]
fn label_gates_keep_the_existing_scene_and_no_fake_role() {
    let empty = tree_layout(&config(), &[], &TreeOptions::default()).unwrap();
    let empty_calls = Cell::new(0);
    let packet = tree_components(&empty, 0, |_| {
        empty_calls.set(empty_calls.get() + 1);
        Ok(Some(placement()))
    })
    .unwrap();
    assert_eq!(empty_calls.get(), 0);
    assert!(packet.data.is_empty());
    assert_eq!(packet.layout.plot, Some(ClipRect::plot(&config())));
    for (hierarchy_labels, visibility, expected) in [
        (false, TreeLabels::All, 0),
        (true, TreeLabels::None, 0),
        (true, TreeLabels::Leaves, 2),
        (true, TreeLabels::Internal, 2),
        (true, TreeLabels::All, 4),
    ] {
        let mut options = TreeOptions::default();
        options.hierarchy.labels = hierarchy_labels;
        options.style.labels.visibility = visibility;
        let layout = tree_layout(&config(), &source(), &options).unwrap();
        let before = layout.scene.clone();
        let calls = Cell::new(0);
        let packet = tree_label_widgets(&layout, 1, |_| {
            calls.set(calls.get() + 1);
            Ok(Some(placement()))
        })
        .unwrap();
        assert_eq!(calls.get(), expected);
        assert_eq!(packet.layout.slots.len(), expected);
        assert_eq!(layout.scene, before);
    }
}

#[test]
fn current_label_font_clip_and_whitespace_follow_owner_across_scene_reindex() {
    let mut layout = tree_layout(&config(), &source(), &TreeOptions::default()).unwrap();
    let index = *layout
        .scene
        .label_series
        .iter()
        .find(|(_, owner)| owner.as_str() == "positive")
        .unwrap()
        .0;
    layout.scene.labels[index].text = "current\tβ\nlabel".into();
    layout.scene.labels[index].rotation = -90.;
    layout.scene.labels[index].color = Color(91, 17, 123, 255);
    layout.scene.typography.labels.insert(
        index,
        FontOptions {
            family: Some(vec![FontFamily::Mono]),
            weight: Some(612.5),
        },
    );
    layout
        .scene
        .typography
        .label_whitespace
        .insert(index, LabelWhitespace::Preserve);
    let clip = ClipPath {
        geometry: Geometry::Rect {
            x: layout.plot.x,
            y: layout.plot.y,
            width: layout.plot.width,
            height: layout.plot.height,
        },
        rule: ClipRule::NonZero,
    };
    layout.scene.label_clips.insert(index, clip.clone());
    let before = tree_label_widgets(&layout, 7, |_| Ok(Some(placement()))).unwrap();
    let old = &before.data["tree-label:positive"];
    assert_eq!(old.label, layout.scene.labels[index]);
    assert_eq!(old.font.family, [FontFamily::Mono]);
    assert_eq!(old.font.weight, 612.5);
    assert_eq!(old.whitespace, LabelWhitespace::Preserve);
    assert_eq!(old.clip.as_ref(), Some(&clip));
    assert_ne!(old.label.text, old.source[old.source_index].label);
    let count = layout.scene.labels.len();
    layout.scene.labels.reverse();
    layout.scene.label_series = layout
        .scene
        .label_series
        .into_iter()
        .map(|(i, owner)| (count - 1 - i, owner))
        .collect();
    layout.scene.label_clips = BTreeMap::from([(count - 1 - index, clip)]);
    layout.scene.typography.labels = BTreeMap::from([(
        count - 1 - index,
        layout.scene.typography.labels.remove(&index).unwrap(),
    )]);
    layout.scene.typography.label_whitespace =
        BTreeMap::from([(count - 1 - index, LabelWhitespace::Preserve)]);
    layout.scene.draw_order.clear();
    let after = tree_label_widgets(&layout, 7, |_| Ok(Some(placement()))).unwrap();
    assert_eq!(
        before.data.keys().collect::<Vec<_>>(),
        after.data.keys().collect::<Vec<_>>()
    );
    let new = &after.data["tree-label:positive"];
    assert_eq!(new.label_index, count - 1 - old.label_index);
    assert_eq!(new.source_index, old.source_index);
    assert_eq!(new.label, old.label);
    assert_eq!(new.font, old.font);
    assert_eq!(new.clip, old.clip);
    assert_ne!(
        new, old,
        "current physical index updates without changing slot identity"
    );
}

#[test]
fn complete_preflight_rejects_late_forgery_and_duplicate_owners_before_any_callback() {
    let layout = tree_layout(&config(), &source(), &TreeOptions::default()).unwrap();
    let mut mutations = vec![];
    let mut x = layout.clone();
    x.nodes.last_mut().unwrap().source_index = usize::MAX;
    mutations.push(x);
    let mut x = layout.clone();
    x.scene.labels.last_mut().unwrap().x = f64::NAN;
    mutations.push(x);
    let mut x = layout.clone();
    x.scene
        .label_series
        .insert(x.scene.labels.len() - 1, "unknown".into());
    mutations.push(x);
    let mut x = layout.clone();
    x.scene.typography.labels.insert(
        x.scene.labels.len() - 1,
        FontOptions {
            weight: Some(f64::NAN),
            ..Default::default()
        },
    );
    mutations.push(x);
    let mut x = layout.clone();
    let first_owner = x.scene.label_series[&0].clone();
    x.scene
        .label_series
        .insert(x.scene.labels.len() - 1, first_owner);
    mutations.push(x);
    for x in mutations {
        let calls = Cell::new(0);
        assert!(
            tree_components(&x, 0, |_| {
                calls.set(calls.get() + 1);
                Ok(Some(placement()))
            })
            .is_err()
        );
        assert_eq!(calls.get(), 0);
    }
    let calls = Cell::new(0);
    let result = tree_components(&layout, 0, |_| {
        calls.set(calls.get() + 1);
        Err(ChartError("stop placement".into()))
    });
    assert_eq!(result.unwrap_err().0, "stop placement");
    assert_eq!(calls.get(), 1, "Err stops all later user calls");
}

#[test]
fn source_bit_only_changes_reach_label_data_and_fresh_equal_allocations_compare_equal() {
    let a = tree_layout(&config(), &source(), &TreeOptions::default()).unwrap();
    let same = tree_layout(&config(), &source(), &TreeOptions::default()).unwrap();
    let mut raw = source();
    raw[0].value = 0.;
    let b = tree_layout(&config(), &raw, &TreeOptions::default()).unwrap();
    assert_eq!(
        a.scene, b.scene,
        "paint compatibility equality remains numeric"
    );
    let select = |c: &TreeLabelWidgetContext<'_>| Ok((c.datum.id == "positive").then(placement));
    let pa = tree_label_widgets(&a, 0, select).unwrap();
    let ps = tree_label_widgets(&same, 0, select).unwrap();
    let pb = tree_label_widgets(&b, 0, select).unwrap();
    assert_eq!(pa, ps, "fresh source allocations have value equality");
    assert_ne!(
        pa.data, pb.data,
        "original unselected signed-zero source is meaningful"
    );
    assert_eq!(
        pa.data["tree-label:positive"].source[0].value.to_bits(),
        (-0_f64).to_bits()
    );
    assert_eq!(
        pb.data["tree-label:positive"].source[0].value.to_bits(),
        0_f64.to_bits()
    );
}

#[test]
fn node_link_and_label_roles_share_exactly_one_128_slot_budget() {
    for (count, expected) in [(43, Some(128)), (44, None)] {
        let mut raw = vec![HierarchyDatum::new("root", None, 0.)];
        raw.extend(
            (1..count).map(|i| HierarchyDatum::new(format!("leaf-{i}"), Some("root".into()), 1.)),
        );
        let layout = tree_layout(&config(), &raw, &TreeOptions::default()).unwrap();
        let calls = Cell::new(0);
        let packet = tree_components(&layout, 0, |_| {
            calls.set(calls.get() + 1);
            Ok(Some(placement()))
        });
        if let Some(expected) = expected {
            assert_eq!(packet.unwrap().data.len(), expected);
            assert_eq!(calls.get(), expected);
        } else {
            assert!(packet.is_err());
            assert_eq!(
                calls.get(),
                129,
                "only selected slots consume the shared limit"
            );
        }
        assert_eq!(
            tree_label_widgets(&layout, 0, |_| Ok(Some(placement())))
                .unwrap()
                .data
                .len(),
            count
        );
    }
}

#[test]
fn activity_labels_borrow_the_accepted_node_state_and_reject_opaque_grafts() {
    let mut layout = tree_layout_with_activity(
        &config(),
        &source(),
        &TreeOptions::default(),
        &mut TreeFactories::default(),
        &TreeActivityOptions {
            focus_id: Some("positive".into()),
            ..Default::default()
        },
        &mut TreeActivityFactories::default(),
    )
    .unwrap();
    let packet = tree_activity_components(&layout, 3, |c| {
        let expected = match c.component {
            TreeComponentContext::Node(n) => layout.activity.nodes[n.source_index],
            TreeComponentContext::Link(l) => layout.activity.links[l.link_index],
            TreeComponentContext::Label(l) => layout.activity.nodes[l.source_index],
        };
        assert_eq!(c.is_active, expected);
        Ok(Some(placement()))
    })
    .unwrap();
    let label = &packet.data["tree-label:positive"];
    assert_eq!(label.is_active, Some(true));
    assert!(Arc::ptr_eq(&label.activity, &layout.activity));
    layout.layout.scene.labels[0].text = "legal current text".into();
    assert!(tree_activity_components(&layout, 3, |_| Ok(None)).is_ok());
    layout.layout = tree_layout(&config(), &source(), &TreeOptions::default()).unwrap();
    let calls = Cell::new(0);
    assert!(
        tree_activity_components(&layout, 3, |_| {
            calls.set(calls.get() + 1);
            Ok(Some(placement()))
        })
        .is_err()
    );
    assert_eq!(calls.get(), 0);
}

#[test]
fn actual_hierarchy_identity_byte_limit_keeps_label_keys_inside_the_host_budget() {
    for id in ["x".repeat(1000), "é".repeat(500)] {
        assert_eq!(id.len(), 1000);
        let raw = [HierarchyDatum::new(id.clone(), None, -0.)];
        let layout = tree_layout(&config(), &raw, &TreeOptions::default()).unwrap();
        let calls = Cell::new(0);
        let packet = tree_label_widgets(&layout, 0, |_| {
            calls.set(calls.get() + 1);
            Ok(Some(placement()))
        })
        .unwrap();
        assert_eq!(calls.get(), 1);
        let slot = &packet.layout.slots[0];
        assert_eq!(slot.key, format!("tree-label:{id}"));
        assert_eq!(slot.key.len(), 1011);
        assert!(slot.key.len() <= MAX_LABEL_BYTES);
        let oversized = [HierarchyDatum::new(format!("{id}x"), None, -0.)];
        assert_eq!(oversized[0].id.len(), 1001);
        assert!(tree_layout(&config(), &oversized, &TreeOptions::default()).is_err());
    }
}
