use std::cell::{Cell, RefCell};
use std::sync::Arc;

use glory::render::{GraphicPoint as P, PathCommand as C};
use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 460.,
        height: 320.,
        ..Default::default()
    }
}
fn source() -> Vec<HierarchyDatum> {
    vec![
        HierarchyDatum::new("a", Some("branch".into()), 2.),
        HierarchyDatum::new("root", None, 0.),
        HierarchyDatum::new("branch", Some("root".into()), -0.),
        HierarchyDatum::new("zero", Some("branch".into()), -0.),
        HierarchyDatum::new("b", Some("root".into()), 3.),
        HierarchyDatum::new("deep", Some("branch".into()), 0.),
        HierarchyDatum::new("c", Some("deep".into()), 5.),
    ]
}
fn activity(focus: Option<&str>, flags: TreeRelationOptions) -> TreeActivityOptions {
    TreeActivityOptions {
        focus_id: focus.map(str::to_owned),
        highlight: flags,
        ..Default::default()
    }
}
fn build(options: &TreeActivityOptions) -> TreeActivityLayout {
    tree_layout_with_activity(
        &config(),
        &source(),
        &TreeOptions::default(),
        &mut TreeFactories::default(),
        options,
        &mut TreeActivityFactories::default(),
    )
    .unwrap()
}
fn placement() -> PlotWidgetPlacement {
    PlotWidgetPlacement::AtDatum {
        offset: [-5., -4.],
        measured: TextSize {
            width: 10.,
            height: 8.,
        },
    }
}

#[test]
fn absent_focus_preserves_old_layout_debug_and_skips_all_modifiers() {
    let called = Cell::new(0);
    let mut active = |_: &TreeNodeModifierContext<'_>| {
        called.set(called.get() + 1);
        Err(ChartError("unused active".into()))
    };
    let mut inactive = |_: &TreeNodeModifierContext<'_>| {
        called.set(called.get() + 1);
        Err(ChartError("unused inactive".into()))
    };
    let mut active_link = |_: &TreeLinkModifierContext<'_>| {
        called.set(called.get() + 1);
        Err(ChartError("unused edge".into()))
    };
    let mut inactive_link = |_: &TreeLinkModifierContext<'_>| {
        called.set(called.get() + 1);
        Err(ChartError("unused edge".into()))
    };
    let old = tree_layout(&config(), &source(), &TreeOptions::default()).unwrap();
    let new = tree_layout_with_activity(
        &config(),
        &source(),
        &TreeOptions::default(),
        &mut TreeFactories::default(),
        &TreeActivityOptions::default(),
        &mut TreeActivityFactories {
            active_node_size: Some(&mut active),
            inactive_node_size: Some(&mut inactive),
            active_link_thickness: Some(&mut active_link),
            inactive_link_thickness: Some(&mut inactive_link),
        },
    )
    .unwrap();
    assert_eq!(called.get(), 0);
    assert_eq!(new.layout, old);
    assert_eq!(format!("{:?}", new.layout), format!("{old:?}"));
    assert_eq!(
        render::to_svg(&new.layout.scene).unwrap(),
        render::to_svg(&old.scene).unwrap()
    );
    assert_eq!(new.activity.nodes, vec![None; 7]);
    assert_eq!(new.activity.links, vec![None; 6]);
    assert_eq!(build(&TreeActivityOptions::default()), new);
}

#[test]
fn all_sixteen_flag_combinations_classify_nodes_and_links_independently() {
    for mask in 0..16 {
        let flags = TreeRelationOptions {
            ancestor_nodes: mask & 1 != 0,
            descendant_nodes: mask & 2 != 0,
            ancestor_links: mask & 4 != 0,
            descendant_links: mask & 8 != 0,
        };
        let layout = build(&activity(Some("deep"), flags.clone()));
        assert_eq!(layout.activity.focus_index, Some(5));
        for (i, node) in layout.layout.nodes.iter().enumerate() {
            let expected = i == 5
                || flags.ancestor_nodes && [1, 2].contains(&i)
                || flags.descendant_nodes && i == 6;
            assert_eq!(
                layout.activity.nodes[i],
                Some(expected),
                "mask={mask},node={i}"
            );
            assert_eq!(node.source_index, i);
        }
        for link in &layout.layout.links {
            let expected = flags.ancestor_links && [2, 5].contains(&link.target_index)
                || flags.descendant_links && link.target_index == 6;
            assert_eq!(
                layout.activity.links[link.link_index],
                Some(expected),
                "mask={mask},target={}",
                link.target_index
            );
        }
    }
    let all_false = TreeRelationOptions {
        ancestor_nodes: false,
        descendant_nodes: false,
        ancestor_links: false,
        descendant_links: false,
    };
    let layout = build(&activity(Some("deep"), all_false));
    assert_eq!(
        layout
            .activity
            .nodes
            .iter()
            .filter(|v| **v == Some(true))
            .count(),
        1
    );
    assert!(layout.activity.links.iter().all(|v| *v == Some(false)));
}

#[test]
fn fixed_depth_and_modifiers_resolve_in_stage_order_before_links_and_labels() {
    let raw = source();
    let options = TreeOptions::default();
    let original = tree_layout(&config(), &raw, &options).unwrap();
    let mut state = activity(Some("deep"), TreeRelationOptions::default());
    state.fix_node_color_at_depth = Some(1);
    let log = RefCell::new(Vec::new());
    let color = |i: usize| Color(i as u8 * 20, 70, 100, 255);
    let mut size = |c: &TreeNodeFunctionContext<'_>| {
        log.borrow_mut().push(format!("size:{}", c.datum.id));
        Ok(20. + c.source_index as f64)
    };
    let mut node_color = |c: &TreeNodeFunctionContext<'_>| {
        log.borrow_mut().push(format!("color:{}", c.datum.id));
        Ok(color(c.source_index))
    };
    let mut active = |c: &TreeNodeModifierContext<'_>| {
        assert!(c.is_active);
        assert_eq!(c.size, 20. + c.node.source_index as f64);
        assert_eq!(c.node.radius, c.size / 2.);
        assert_eq!(
            c.node.color,
            Some(color(if c.node.depth > 1 {
                2
            } else {
                c.node.source_index
            }))
        );
        log.borrow_mut().push(format!("active:{}", c.datum.id));
        Ok(c.size + 20.)
    };
    let mut inactive = |c: &TreeNodeModifierContext<'_>| {
        assert!(!c.is_active);
        assert_eq!(c.node.radius, c.size / 2.);
        log.borrow_mut().push(format!("inactive:{}", c.datum.id));
        Ok(c.size / 2.)
    };
    let mut thickness = |c: &TreeLinkFunctionContext<'_>| {
        let states = c.activity.unwrap();
        for n in [c.source_node, c.target_node] {
            let base = 20. + n.source_index as f64;
            let final_diameter = if states.nodes[n.source_index] == Some(true) {
                base + 20.
            } else {
                base / 2.
            };
            assert_eq!(n.radius, final_diameter / 2.);
        }
        log.borrow_mut()
            .push(format!("width:{}", c.link.link_index));
        Ok(3.)
    };
    let mut link_color = |c: &TreeLinkFunctionContext<'_>| {
        assert!(c.activity.is_some());
        log.borrow_mut()
            .push(format!("link-color:{}", c.link.link_index));
        Ok(Color(210, 100, 20, 255))
    };
    let mut active_link = |c: &TreeLinkModifierContext<'_>| {
        assert!(c.is_active);
        assert_eq!(c.thickness, 3.);
        assert_eq!(c.color, Color(210, 100, 20, 255));
        log.borrow_mut()
            .push(format!("active-edge:{}", c.link.link_index));
        Ok(9.)
    };
    let mut inactive_link = |c: &TreeLinkModifierContext<'_>| {
        assert!(!c.is_active);
        log.borrow_mut()
            .push(format!("inactive-edge:{}", c.link.link_index));
        Ok(0.)
    };
    let mut label = |c: &TreeLabelFunctionContext<'_>| {
        assert!(c.activity.is_some());
        log.borrow_mut().push(format!("label:{}", c.datum.id));
        Ok(format!("{}:{}", c.datum.id, c.node.radius))
    };
    let layout = tree_layout_with_activity(
        &config(),
        &raw,
        &options,
        &mut TreeFactories {
            node_size: Some(&mut size),
            node_color: Some(&mut node_color),
            link_thickness: Some(&mut thickness),
            link_color: Some(&mut link_color),
            label: Some(&mut label),
        },
        &state,
        &mut TreeActivityFactories {
            active_node_size: Some(&mut active),
            inactive_node_size: Some(&mut inactive),
            active_link_thickness: Some(&mut active_link),
            inactive_link_thickness: Some(&mut inactive_link),
        },
    )
    .unwrap();
    let mut expected = Vec::new();
    for i in [1, 2, 4, 0, 3, 5, 6] {
        expected.push(format!("size:{}", raw[i].id));
        if [1, 2, 4].contains(&i) {
            expected.push(format!("color:{}", raw[i].id));
        }
        expected.push(format!(
            "{}:{}",
            if [1, 2, 5].contains(&i) {
                "active"
            } else {
                "inactive"
            },
            raw[i].id
        ));
    }
    for link in &layout.layout.links {
        let i = link.link_index;
        expected.extend([
            format!("width:{i}"),
            format!("link-color:{i}"),
            format!(
                "{}:{i}",
                if layout.activity.links[i] == Some(true) {
                    "active-edge"
                } else {
                    "inactive-edge"
                }
            ),
        ]);
    }
    for i in [1, 2, 4, 0, 3, 5, 6] {
        expected.push(format!("label:{}", raw[i].id));
    }
    assert_eq!(*log.borrow(), expected);
    for node in &layout.layout.nodes {
        assert_eq!(node.position, original.nodes[node.source_index].position);
        assert_eq!(node.style.radius, Some(node.radius));
        let mark = layout
            .layout
            .scene
            .marks
            .iter()
            .find(|m| Some(&m.id) == node.mark_id.as_ref())
            .unwrap();
        assert_eq!(mark.fill, node.color);
        assert_eq!(
            mark.geometry,
            Geometry::Circle {
                x: node.position[0],
                y: node.position[1],
                radius: node.radius
            }
        );
    }
    for link in &layout.layout.links {
        let mark = layout
            .layout
            .scene
            .marks
            .iter()
            .find(|m| Some(&m.id) == link.mark_id.as_ref())
            .unwrap();
        assert_eq!(
            mark.stroke.map(|s| s.width),
            if layout.activity.links[link.link_index] == Some(true) {
                Some(9.)
            } else {
                None
            }
        );
    }
    let packet = tree_activity_widgets(&layout, 0, |_| Ok(Some(placement()))).unwrap();
    assert_eq!(packet.data.len(), 13);
    for d in packet.data.values() {
        assert!(Arc::ptr_eq(&d.activity, &layout.activity));
        let expected = match &d.tree {
            TreeWidgetData::Node(n) => layout.activity.nodes[n.source_index],
            TreeWidgetData::Link(l) => layout.activity.links[l.link_index],
        };
        assert_eq!(d.is_active, expected);
    }
    tree_activity_frame(&layout).unwrap();
}

#[test]
fn fixed_depth_skips_color_functions_and_propagates_parent_no_fill() {
    let mut options = TreeOptions::default();
    options.style.nodes.fill = TreeNodeFill::None;
    options.style.leaves = Some(TreeNodeStyle {
        fill: TreeNodeFill::Fixed(Color(255, 0, 0, 255)),
        ..Default::default()
    });
    let activity = TreeActivityOptions {
        fix_node_color_at_depth: Some(0),
        ..Default::default()
    };
    let layout = tree_layout_with_activity(
        &config(),
        &source(),
        &options,
        &mut TreeFactories::default(),
        &activity,
        &mut TreeActivityFactories::default(),
    )
    .unwrap();
    assert!(
        layout
            .layout
            .nodes
            .iter()
            .all(|n| n.color.is_none() && n.style.fill == TreeNodeFill::None)
    );
    assert!(
        layout
            .layout
            .scene
            .marks
            .iter()
            .filter(|m| m.id.starts_with("node:"))
            .all(|m| m.fill.is_none())
    );
    for depth in [Some(0), Some(1), Some(2), Some(usize::MAX), None] {
        let calls = RefCell::new(Vec::new());
        let mut color = |c: &TreeNodeFunctionContext<'_>| {
            calls.borrow_mut().push(c.source_index);
            Ok(Color(c.source_index as u8, 0, 0, 255))
        };
        let state = TreeActivityOptions {
            fix_node_color_at_depth: depth,
            ..Default::default()
        };
        let layout = tree_layout_with_activity(
            &config(),
            &source(),
            &options,
            &mut TreeFactories {
                node_color: Some(&mut color),
                ..Default::default()
            },
            &state,
            &mut TreeActivityFactories::default(),
        )
        .unwrap();
        let expected = layout
            .layout
            .nodes
            .iter()
            .filter(|n| depth.is_none_or(|d| n.depth <= d))
            .count();
        assert_eq!(calls.borrow().len(), expected);
        assert!(calls.borrow().contains(&1));
    }
}

#[test]
fn disabled_links_still_apply_inactive_modifier_and_clearing_restores_base() {
    let mut options = TreeOptions::default();
    options.style.links.enabled = false;
    let flags = TreeRelationOptions {
        ancestor_nodes: false,
        descendant_nodes: false,
        ancestor_links: false,
        descendant_links: false,
    };
    let calls = Cell::new(0);
    let mut modifier = |c: &TreeLinkModifierContext<'_>| {
        assert!(!c.is_active);
        assert_eq!(c.link.mark_id, None);
        calls.set(calls.get() + 1);
        Ok(0.)
    };
    let layout = tree_layout_with_activity(
        &config(),
        &source(),
        &options,
        &mut TreeFactories::default(),
        &activity(Some("deep"), flags),
        &mut TreeActivityFactories {
            inactive_link_thickness: Some(&mut modifier),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(calls.get(), 6);
    assert_eq!(layout.layout.scene.marks.len(), 7);
    let cleared = tree_layout_with_activity(
        &config(),
        &source(),
        &options,
        &mut TreeFactories::default(),
        &TreeActivityOptions::default(),
        &mut TreeActivityFactories {
            inactive_link_thickness: Some(&mut modifier),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(calls.get(), 6);
    assert_eq!(
        cleared.layout,
        tree_layout(&config(), &source(), &options).unwrap()
    );
}

#[test]
fn unknown_focus_and_invalid_source_run_no_functions() {
    let calls = Cell::new(0);
    let mut size = |_: &TreeNodeFunctionContext<'_>| {
        calls.set(calls.get() + 1);
        Ok(10.)
    };
    for raw in [source(), vec![]] {
        assert!(
            tree_layout_with_activity(
                &config(),
                &raw,
                &TreeOptions::default(),
                &mut TreeFactories {
                    node_size: Some(&mut size),
                    ..Default::default()
                },
                &activity(Some("missing"), TreeRelationOptions::default()),
                &mut TreeActivityFactories::default()
            )
            .is_err()
        );
        assert_eq!(calls.get(), 0);
    }
    let mut raw = source();
    raw[0].parent = Some("missing".into());
    assert!(
        tree_layout_with_activity(
            &config(),
            &raw,
            &TreeOptions::default(),
            &mut TreeFactories {
                node_size: Some(&mut size),
                ..Default::default()
            },
            &TreeActivityOptions::default(),
            &mut TreeActivityFactories::default()
        )
        .is_err()
    );
    assert_eq!(calls.get(), 0);
}

#[test]
fn invalid_modifier_outputs_stop_at_the_selected_stage() {
    for value in [f64::NAN, f64::INFINITY, -1., 32768.1] {
        let later = Cell::new(0);
        let mut modifier = |_: &TreeNodeModifierContext<'_>| Ok(value);
        let mut link = |_: &TreeLinkFunctionContext<'_>| {
            later.set(later.get() + 1);
            Ok(2.)
        };
        assert!(
            tree_layout_with_activity(
                &config(),
                &source(),
                &TreeOptions::default(),
                &mut TreeFactories {
                    link_thickness: Some(&mut link),
                    ..Default::default()
                },
                &activity(Some("root"), TreeRelationOptions::default()),
                &mut TreeActivityFactories {
                    active_node_size: Some(&mut modifier),
                    ..Default::default()
                }
            )
            .is_err()
        );
        assert_eq!(later.get(), 0);
    }
    for value in [f64::NAN, f64::INFINITY, -1., 1e-7, 10000.1] {
        let mut modifier = |_: &TreeLinkModifierContext<'_>| Ok(value);
        assert!(
            tree_layout_with_activity(
                &config(),
                &source(),
                &TreeOptions::default(),
                &mut TreeFactories::default(),
                &activity(Some("root"), TreeRelationOptions::default()),
                &mut TreeActivityFactories {
                    inactive_link_thickness: Some(&mut modifier),
                    ..Default::default()
                }
            )
            .is_err()
        );
    }
    let mut node =
        |_: &TreeNodeModifierContext<'_>| Err(ChartError("modifier application failure".into()));
    let e = tree_layout_with_activity(
        &config(),
        &source(),
        &TreeOptions::default(),
        &mut TreeFactories::default(),
        &activity(Some("root"), TreeRelationOptions::default()),
        &mut TreeActivityFactories {
            active_node_size: Some(&mut node),
            ..Default::default()
        },
    )
    .unwrap_err();
    assert_eq!(e.0, "modifier application failure");
}

#[test]
fn original_capture_allows_current_ink_edits_but_rejects_grafted_layout_or_state() {
    let mut layout = build(&activity(Some("deep"), TreeRelationOptions::default()));
    let same = build(&activity(Some("deep"), TreeRelationOptions::default()));
    assert_eq!(layout, same);
    let p = layout.layout.nodes[0].position;
    let mark = layout
        .layout
        .scene
        .marks
        .iter_mut()
        .find(|m| m.id == "node:a")
        .unwrap();
    mark.geometry = Geometry::Path(vec![
        C::Move(P::new(p[0] + 20., p[1] - 5.)),
        C::Line(P::new(p[0] + 30., p[1] - 5.)),
        C::Line(P::new(p[0] + 25., p[1] + 5.)),
        C::Close,
    ]);
    mark.fill = Some(Color(20, 200, 20, 255));
    tree_activity_frame(&layout).unwrap();
    let packet = tree_activity_widgets(&layout, 0, |c| {
        Ok(matches!(c.tree,TreeWidgetContext::Node(n)if n.source_index==0).then_some(placement()))
    })
    .unwrap();
    assert_eq!(
        packet.layout.slots[0].placement,
        PlotWidgetPlacement::Rect(LayoutRect {
            x: p[0] - 5.,
            y: p[1] - 4.,
            width: 10.,
            height: 8.
        })
    );
    assert_eq!(layout.node_id_from_mark("node:a"), Some("a"));
    assert_eq!(layout.mark_id_for_node("a"), Some("node:a"));
    assert_eq!(layout.node_id_from_mark("link:a"), None);
    let mut raw = source();
    raw.swap(0, 4);
    let mut changed = source();
    changed[5].parent = Some("root".into());
    for data in [source(), raw, changed] {
        let mut graft = layout.clone();
        graft.layout = tree_layout(&config(), &data, &TreeOptions::default()).unwrap();
        let count = Cell::new(0);
        assert!(tree_activity_frame(&graft).is_err());
        assert!(
            tree_activity_widgets(&graft, 0, |_| {
                count.set(count.get() + 1);
                Ok(Some(placement()))
            })
            .is_err()
        );
        assert_eq!(count.get(), 0);
    }
    let mut invalid = layout;
    Arc::make_mut(&mut invalid.activity).nodes[0] = Some(true);
    let count = Cell::new(0);
    assert!(
        tree_activity_widgets(&invalid, 0, |_| {
            count.set(count.get() + 1);
            Ok(Some(placement()))
        })
        .is_err()
    );
    assert_eq!(count.get(), 0);
}
