//! Independent Tree activity provenance and computed-function contracts.
use std::cell::{Cell, RefCell};

use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 420.,
        height: 280.,
        margin: Margin {
            left: 31.,
            right: 29.,
            top: 23.,
            bottom: 27.,
        },
        ..Default::default()
    }
}
fn raw() -> Vec<HierarchyDatum> {
    vec![
        HierarchyDatum::new("leaf", Some("branch".into()), 4.),
        HierarchyDatum::new("root", None, 0.),
        HierarchyDatum::new("branch", Some("root".into()), 0.),
        HierarchyDatum::new("zero", Some("root".into()), -0.),
        HierarchyDatum::new("other", Some("branch".into()), 3.),
    ]
}
fn neutral(focus: Option<&str>) -> TreeActivityLayout {
    tree_layout_with_activity(
        &config(),
        &raw(),
        &TreeOptions::default(),
        &mut TreeFactories::default(),
        &TreeActivityOptions {
            focus_id: focus.map(str::to_owned),
            ..Default::default()
        },
        &mut TreeActivityFactories::default(),
    )
    .unwrap()
}

#[test]
fn whole_valid_layout_graft_must_not_reuse_an_unrelated_activity_index_space() {
    let initial = neutral(Some("branch"));
    let mut reordered = raw();
    reordered.reverse();
    let mut topology = raw();
    topology[0].parent = Some("zero".into());
    for source in [reordered, topology] {
        let candidate = tree_layout(&config(), &source, &TreeOptions::default()).unwrap();
        assert!(
            tree_widget_frame(&candidate).is_ok(),
            "graft is a valid independent layout"
        );
        let mut graft = initial.clone();
        graft.layout = candidate;
        let callbacks = Cell::new(0);
        let result = tree_activity_widgets(&graft, 1, |_| {
            callbacks.set(callbacks.get() + 1);
            Ok(None)
        });
        assert!(
            result.is_err(),
            "independent valid layout graft was accepted, callbacks={}",
            callbacks.get()
        );
        assert_eq!(callbacks.get(), 0);
        assert!(tree_activity_frame(&graft).is_err());
    }
}

#[test]
fn all_highlight_flags_keep_node_and_link_selection_independent_and_clear_is_neutral() {
    // Independently authored ancestor/descendant memberships for raw(). Edge
    // order is root->branch, root->zero, branch->leaf, branch->other.
    let ancestors = [vec![1, 2], vec![], vec![1], vec![1], vec![1, 2]];
    let descendants = [vec![], vec![0, 2, 3, 4], vec![0, 4], vec![], vec![]];
    let edges = [(1, 2), (1, 3), (2, 0), (2, 4)];
    for flags in 0u8..16 {
        for focus in [None, Some(0usize), Some(1), Some(2), Some(3), Some(4)] {
            for enabled in [false, true] {
                let mut o = TreeOptions::default();
                o.style.links.enabled = enabled;
                let options = TreeActivityOptions {
                    focus_id: focus.map(|i| raw()[i].id.clone()),
                    highlight: TreeRelationOptions {
                        ancestor_nodes: flags & 1 != 0,
                        descendant_nodes: flags & 2 != 0,
                        ancestor_links: flags & 4 != 0,
                        descendant_links: flags & 8 != 0,
                    },
                    ..Default::default()
                };
                let calls = Cell::new(0);
                let mut node = |c: &TreeNodeModifierContext<'_>| {
                    calls.set(calls.get() + 1);
                    assert_eq!(c.activity.focus_index, focus);
                    Ok(c.size)
                };
                let mut inactive_node = |c: &TreeNodeModifierContext<'_>| {
                    calls.set(calls.get() + 1);
                    assert!(!c.is_active);
                    Ok(c.size)
                };
                let mut link = |c: &TreeLinkModifierContext<'_>| {
                    calls.set(calls.get() + 1);
                    assert!(c.is_active);
                    Ok(c.thickness)
                };
                let mut inactive_link = |c: &TreeLinkModifierContext<'_>| {
                    calls.set(calls.get() + 1);
                    assert!(!c.is_active);
                    Ok(c.thickness)
                };
                let current = tree_layout_with_activity(
                    &config(),
                    &raw(),
                    &o,
                    &mut TreeFactories::default(),
                    &options,
                    &mut TreeActivityFactories {
                        active_node_size: Some(&mut node),
                        inactive_node_size: Some(&mut inactive_node),
                        active_link_thickness: Some(&mut link),
                        inactive_link_thickness: Some(&mut inactive_link),
                    },
                )
                .unwrap();
                let expected_nodes: Vec<_> = (0..5)
                    .map(|i| {
                        focus.map(|f| {
                            i == f
                                || flags & 1 != 0 && ancestors[f].contains(&i)
                                || flags & 2 != 0 && descendants[f].contains(&i)
                        })
                    })
                    .collect();
                let expected_links: Vec<_> = edges
                    .iter()
                    .map(|&(s, t)| {
                        focus.map(|f| {
                            flags & 4 != 0 && (t == f || ancestors[f].contains(&t))
                                || flags & 8 != 0 && (s == f || descendants[f].contains(&s))
                        })
                    })
                    .collect();
                assert_eq!(current.activity.nodes, expected_nodes);
                assert_eq!(current.activity.links, expected_links);
                assert_eq!(calls.get(), if focus.is_some() { 9 } else { 0 });
                let base = tree_layout(&config(), &raw(), &o).unwrap();
                assert_eq!(current.layout.scene, base.scene);
                assert_eq!(current.layout.source(), base.source());
                assert_eq!(current.layout.options(), base.options());
                for (current, base) in current.layout.nodes.iter().zip(&base.nodes) {
                    assert_eq!(current.position, base.position);
                    assert_eq!(current.radius, base.radius);
                    // An invoked modifier explicitly materializes the radius
                    // override even if it returns the inherited base size.
                    assert_eq!(current.style.radius, focus.map(|_| current.radius));
                }
                assert!(tree_activity_frame(&current).is_ok());
                assert_eq!(
                    tree_activity_widgets(&current, 1, |c| {
                        let expected = match c.tree {
                            TreeWidgetContext::Node(n) => expected_nodes[n.source_index],
                            TreeWidgetContext::Link(l) => expected_links[l.link_index],
                        };
                        assert_eq!(c.is_active, expected);
                        Ok(None)
                    })
                    .unwrap()
                    .data
                    .len(),
                    0
                );
            }
        }
    }
}

#[test]
fn modifiers_see_complete_base_styles_then_final_nodes_flow_to_links_labels_and_live_roles() {
    let log = RefCell::new(vec![]);
    let mut size = |c: &TreeNodeFunctionContext<'_>| {
        log.borrow_mut().push(format!("size:{}", c.datum.id));
        Ok(12. + c.source_index as f64 * 2.)
    };
    let mut color = |c: &TreeNodeFunctionContext<'_>| {
        log.borrow_mut().push(format!("color:{}", c.datum.id));
        Ok(Color(10 + c.source_index as u8, 50, 90, 255))
    };
    let mut active = |c: &TreeNodeModifierContext<'_>| {
        assert!(c.is_active);
        assert_eq!(c.node.radius * 2., c.size);
        assert_eq!(c.node.color, Some(Color(11, 50, 90, 255)));
        assert_eq!(c.datum, &raw()[c.node.source_index]);
        log.borrow_mut().push(format!("active:{}", c.datum.id));
        Ok(c.size + 8.)
    };
    let mut inactive = |c: &TreeNodeModifierContext<'_>| {
        assert!(!c.is_active);
        assert_eq!(c.node.radius * 2., c.size);
        assert_eq!(c.node.color, Some(Color(11, 50, 90, 255)));
        log.borrow_mut().push(format!("inactive:{}", c.datum.id));
        Ok(c.size / 2.)
    };
    let radius = |index: usize| {
        if index == 1 || index == 2 {
            10. + index as f64
        } else {
            3. + index as f64 / 2.
        }
    };
    let mut thickness = |c: &TreeLinkFunctionContext<'_>| {
        assert!(c.activity.is_some());
        assert_eq!(c.source_node.radius, radius(c.link.source_index));
        assert_eq!(c.target_node.radius, radius(c.link.target_index));
        log.borrow_mut()
            .push(format!("width:{}", c.link.link_index));
        Ok(3.)
    };
    let mut linkcolor = |c: &TreeLinkFunctionContext<'_>| {
        assert_eq!(c.source_node.color, Some(Color(11, 50, 90, 255)));
        log.borrow_mut()
            .push(format!("linkcolor:{}", c.link.link_index));
        Ok(Color(230, 110, 20, 255))
    };
    let mut active_link = |c: &TreeLinkModifierContext<'_>| {
        assert!(c.is_active);
        assert_eq!(c.thickness, 3.);
        assert_eq!(c.color, Color(230, 110, 20, 255));
        assert_eq!(c.target_node.radius, radius(c.link.target_index));
        log.borrow_mut()
            .push(format!("active-link:{}", c.link.link_index));
        Ok(7.)
    };
    let mut inactive_link = |c: &TreeLinkModifierContext<'_>| {
        assert!(!c.is_active);
        assert_eq!(c.thickness, 3.);
        log.borrow_mut()
            .push(format!("inactive-link:{}", c.link.link_index));
        Ok(0.)
    };
    let mut label = |c: &TreeLabelFunctionContext<'_>| {
        assert_eq!(c.node.radius, radius(c.source_index));
        assert!(c.activity.is_some());
        log.borrow_mut().push(format!("label:{}", c.datum.id));
        Ok(format!("paint:{}", c.datum.id))
    };
    let layout = tree_layout_with_activity(
        &config(),
        &raw(),
        &TreeOptions::default(),
        &mut TreeFactories {
            node_size: Some(&mut size),
            node_color: Some(&mut color),
            link_thickness: Some(&mut thickness),
            link_color: Some(&mut linkcolor),
            label: Some(&mut label),
        },
        &TreeActivityOptions {
            focus_id: Some("branch".into()),
            fix_node_color_at_depth: Some(0),
            ..Default::default()
        },
        &mut TreeActivityFactories {
            active_node_size: Some(&mut active),
            inactive_node_size: Some(&mut inactive),
            active_link_thickness: Some(&mut active_link),
            inactive_link_thickness: Some(&mut inactive_link),
        },
    )
    .unwrap();
    let mut expected = vec![];
    for (id, on) in [
        ("root", true),
        ("branch", true),
        ("zero", false),
        ("leaf", false),
        ("other", false),
    ] {
        expected.push(format!("size:{id}"));
        if id == "root" {
            expected.push(format!("color:{id}"));
        }
        expected.push(format!("{}:{id}", if on { "active" } else { "inactive" }));
    }
    for i in 0..4 {
        expected.push(format!("width:{i}"));
        expected.push(format!("linkcolor:{i}"));
        expected.push(format!(
            "{}:{i}",
            if i == 0 {
                "active-link"
            } else {
                "inactive-link"
            }
        ));
    }
    expected.extend(["root", "branch", "zero", "leaf", "other"].map(|s| format!("label:{s}")));
    assert_eq!(*log.borrow(), expected);
    let packet = tree_activity_widgets(&layout, 1, |_| {
        Ok(Some(PlotWidgetPlacement::AtDatum {
            offset: [1., 1.],
            measured: TextSize {
                width: 4.,
                height: 4.,
            },
        }))
    })
    .unwrap();
    assert_eq!(packet.data.len(), 9);
    for node in &layout.layout.nodes {
        assert_eq!(node.radius, radius(node.source_index));
        assert_eq!(node.color, Some(Color(11, 50, 90, 255)));
    }
    for (i, edge) in layout.layout.links.iter().enumerate() {
        let mark = layout
            .layout
            .scene
            .marks
            .iter()
            .find(|m| Some(&m.id) == edge.mark_id.as_ref())
            .unwrap();
        assert_eq!(
            mark.stroke.map(|s| s.width),
            if i == 0 { Some(7.) } else { None }
        );
    }
    for label in &layout.layout.scene.labels {
        assert!(label.text.starts_with("paint:"));
    }
    for mark in layout
        .layout
        .scene
        .marks
        .iter()
        .filter(|m| m.id.starts_with("node:"))
    {
        assert!(!mark.label.starts_with("paint:"));
    }
}

#[test]
fn activity_forgery_preflights_before_placement_but_current_scene_edits_remain_legal() {
    let original = neutral(Some("branch"));
    assert_eq!(
        original,
        neutral(Some("branch")),
        "allocation identity is not presentation equality"
    );
    for mutation in 0..5 {
        let mut changed = original.clone();
        let state = std::sync::Arc::make_mut(&mut changed.activity);
        match mutation {
            0 => state.nodes.reverse(),
            1 => state.links.clear(),
            2 => state.focus_id = Some("leaf".into()),
            3 => state.highlight.descendant_links = true,
            _ => state.fix_node_color_at_depth = Some(1),
        }
        let calls = Cell::new(0);
        assert!(
            tree_activity_widgets(&changed, 1, |_| {
                calls.set(calls.get() + 1);
                Ok(None)
            })
            .is_err()
        );
        assert_eq!(calls.get(), 0);
        assert!(tree_activity_frame(&changed).is_err());
    }
    let mut edited = original.clone();
    edited.layout.scene.marks[0].opacity = 0.25;
    edited.layout.scene.labels[0].text = "current presentation".into();
    assert!(tree_activity_frame(&edited).is_ok());
    assert!(tree_activity_widgets(&edited, 1, |_| Ok(None)).is_ok());
}

#[test]
fn unknown_focus_bad_source_and_invalid_modifier_results_stop_without_a_candidate() {
    let calls = Cell::new(0);
    let mut size = |_: &TreeNodeFunctionContext<'_>| {
        calls.set(calls.get() + 1);
        Ok(12.)
    };
    let mut bad = raw();
    bad[4].value = f64::NAN;
    for (source, focus) in [(raw(), Some("unknown")), (bad, None)] {
        assert!(
            tree_layout_with_activity(
                &config(),
                &source,
                &TreeOptions::default(),
                &mut TreeFactories {
                    node_size: Some(&mut size),
                    ..Default::default()
                },
                &TreeActivityOptions {
                    focus_id: focus.map(str::to_owned),
                    ..Default::default()
                },
                &mut TreeActivityFactories::default()
            )
            .is_err()
        );
        assert_eq!(calls.get(), 0);
    }
    for active in [false, true] {
        for value in [f64::NAN, -1., 1e12] {
            let mut modifier = |_: &TreeNodeModifierContext<'_>| Ok(value);
            let mut f = TreeActivityFactories::default();
            if active {
                f.active_node_size = Some(&mut modifier);
            } else {
                f.inactive_node_size = Some(&mut modifier);
            }
            assert!(
                tree_layout_with_activity(
                    &config(),
                    &raw(),
                    &TreeOptions::default(),
                    &mut TreeFactories::default(),
                    &TreeActivityOptions {
                        focus_id: Some("branch".into()),
                        ..Default::default()
                    },
                    &mut f
                )
                .is_err()
            );
            let mut modifier = |_: &TreeLinkModifierContext<'_>| Ok(value);
            let mut f = TreeActivityFactories::default();
            if active {
                f.active_link_thickness = Some(&mut modifier);
            } else {
                f.inactive_link_thickness = Some(&mut modifier);
            }
            assert!(
                tree_layout_with_activity(
                    &config(),
                    &raw(),
                    &TreeOptions::default(),
                    &mut TreeFactories::default(),
                    &TreeActivityOptions {
                        focus_id: Some("branch".into()),
                        ..Default::default()
                    },
                    &mut f
                )
                .is_err()
            );
        }
    }
    let mut no_fill = TreeOptions::default();
    no_fill.style.nodes.fill = TreeNodeFill::None;
    let current = tree_layout_with_activity(
        &config(),
        &raw(),
        &no_fill,
        &mut TreeFactories::default(),
        &TreeActivityOptions {
            fix_node_color_at_depth: Some(0),
            ..Default::default()
        },
        &mut TreeActivityFactories::default(),
    )
    .unwrap();
    assert!(
        current
            .layout
            .nodes
            .iter()
            .all(|n| n.color.is_none() && n.style.fill == TreeNodeFill::None)
    );
}

#[test]
fn zero_modifier_results_and_zero_opacity_keep_typed_roles_without_inventing_ink() {
    let mut node = |_: &TreeNodeModifierContext<'_>| Ok(0.);
    let mut inactive_node = |_: &TreeNodeModifierContext<'_>| Ok(0.);
    let mut link = |_: &TreeLinkModifierContext<'_>| Ok(0.);
    let mut inactive_link = |_: &TreeLinkModifierContext<'_>| Ok(0.);
    let mut layout = tree_layout_with_activity(
        &config(),
        &raw(),
        &TreeOptions::default(),
        &mut TreeFactories::default(),
        &TreeActivityOptions {
            focus_id: Some("branch".into()),
            ..Default::default()
        },
        &mut TreeActivityFactories {
            active_node_size: Some(&mut node),
            inactive_node_size: Some(&mut inactive_node),
            active_link_thickness: Some(&mut link),
            inactive_link_thickness: Some(&mut inactive_link),
        },
    )
    .unwrap();
    assert_eq!(layout.layout.scene.marks.len(), 9);
    assert!(layout.layout.nodes.iter().all(|n| n.radius == 0.));
    for mark in &mut layout.layout.scene.marks {
        if mark.id.starts_with("link:") {
            assert!(mark.stroke.is_none());
        }
        mark.opacity = 0.;
    }
    let frame = tree_activity_frame(&layout).unwrap();
    assert_eq!(frame.interaction.unwrap().sites.len(), 5);
    let calls = Cell::new(0);
    let packet = tree_activity_widgets(&layout, 1, |_| {
        calls.set(calls.get() + 1);
        Ok(Some(PlotWidgetPlacement::AtDatum {
            offset: [1., 1.],
            measured: TextSize {
                width: 2.,
                height: 2.,
            },
        }))
    })
    .unwrap();
    assert_eq!(calls.get(), 9);
    assert_eq!(packet.data.len(), 9);
}
