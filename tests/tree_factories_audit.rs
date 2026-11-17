//! Independent five-function Tree configuration audit, not Nivo numeric parity.
use std::cell::{Cell, RefCell};

use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 400.,
        height: 280.,
        margin: Margin {
            left: 31.,
            right: 25.,
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

#[test]
fn callbacks_use_bfs_original_source_indices_and_links_receive_all_final_node_styles() {
    let mut o = TreeOptions::default();
    o.style.nodes.fill = TreeNodeFill::None;
    let base = tree_layout(&config(), &raw(), &o).unwrap();
    let log = RefCell::new(vec![]);
    let mut size = |c: &TreeNodeFunctionContext<'_>| {
        assert_eq!(c.source, &raw());
        assert_eq!(c.datum, &raw()[c.source_index]);
        assert_eq!(c.plot, base.plot);
        let n = &base.nodes[c.source_index];
        assert_eq!(
            (
                c.parent_index,
                c.children,
                c.depth,
                c.height,
                c.value,
                c.position
            ),
            (
                n.parent_index,
                n.children.as_slice(),
                n.depth,
                n.height,
                n.value,
                n.position
            )
        );
        log.borrow_mut().push(format!("size:{}", c.datum.id));
        Ok(10. + c.source_index as f64 * 2.)
    };
    let mut color = |c: &TreeNodeFunctionContext<'_>| {
        log.borrow_mut().push(format!("color:{}", c.datum.id));
        Ok(Color(20 + c.source_index as u8, 90, 140, 255))
    };
    let mut width = |c: &TreeLinkFunctionContext<'_>| {
        assert_eq!(c.source, &raw());
        assert_eq!(c.source_node.source_index, c.link.source_index);
        assert_eq!(c.target_node.source_index, c.link.target_index);
        for n in [c.source_node, c.target_node] {
            assert_eq!(n.radius, 5. + n.source_index as f64);
            assert_eq!(
                n.color,
                Some(Color(20 + n.source_index as u8, 90, 140, 255))
            );
            assert_eq!(n.style.radius, Some(n.radius));
            assert_eq!(n.style.fill, TreeNodeFill::Fixed(n.color.unwrap()));
        }
        log.borrow_mut()
            .push(format!("width:{}", c.source[c.link.target_index].id));
        Ok(if c.source[c.link.target_index].id == "zero" {
            0.
        } else {
            3.
        })
    };
    let mut link_color = |c: &TreeLinkFunctionContext<'_>| {
        log.borrow_mut()
            .push(format!("linkcolor:{}", c.source[c.link.target_index].id));
        Ok(Color(230, 110, 20, 255))
    };
    let mut label = |c: &TreeLabelFunctionContext<'_>| {
        assert_eq!(c.node.radius, 5. + c.source_index as f64);
        assert_eq!(
            c.node.color,
            Some(Color(20 + c.source_index as u8, 90, 140, 255))
        );
        log.borrow_mut().push(format!("label:{}", c.datum.id));
        Ok(if c.datum.id == "zero" {
            String::new()
        } else {
            format!("paint-{}", c.datum.id)
        })
    };
    let actual = tree_layout_with_factories(
        &config(),
        &raw(),
        &o,
        &mut TreeFactories {
            node_size: Some(&mut size),
            node_color: Some(&mut color),
            link_thickness: Some(&mut width),
            link_color: Some(&mut link_color),
            label: Some(&mut label),
        },
    )
    .unwrap();
    let mut expected = vec![];
    for id in ["root", "branch", "zero", "leaf", "other"] {
        expected.push(format!("size:{id}"));
        expected.push(format!("color:{id}"));
    }
    for id in ["branch", "zero", "leaf", "other"] {
        expected.push(format!("width:{id}"));
        expected.push(format!("linkcolor:{id}"));
    }
    for id in ["root", "branch", "zero", "leaf", "other"] {
        expected.push(format!("label:{id}"));
    }
    assert_eq!(*log.borrow(), expected);
    assert_eq!(
        actual.nodes.iter().map(|n| n.position).collect::<Vec<_>>(),
        base.nodes.iter().map(|n| n.position).collect::<Vec<_>>(),
        "dynamic diameter does not mutate static projection inset"
    );
    for n in &actual.nodes {
        let mark = actual
            .scene
            .marks
            .iter()
            .find(|m| Some(&m.id) == n.mark_id.as_ref())
            .unwrap();
        assert_eq!(mark.fill, n.color);
        assert_eq!(mark.label, raw()[n.source_index].label);
    }
    let zero = actual
        .scene
        .marks
        .iter()
        .find(|m| m.id == "link:zero")
        .unwrap();
    assert!(zero.stroke.is_none());
    assert!(
        actual
            .links
            .iter()
            .find(|l| l.mark_id.as_deref() == Some("link:zero"))
            .is_some()
    );
    let label_index = actual
        .scene
        .label_series
        .iter()
        .find(|(_, owner)| owner.as_str() == "zero")
        .unwrap()
        .0;
    assert_eq!(actual.scene.labels[*label_index].text, "");
    let packet = tree_node_widgets(&actual, 0, |_| {
        Ok(Some(PlotWidgetPlacement::AtDatum {
            offset: [0., 0.],
            measured: TextSize {
                width: 5.,
                height: 5.,
            },
        }))
    })
    .unwrap();
    assert_eq!(packet.data.len(), 5);
    assert!(
        packet
            .data
            .values()
            .all(|d| d.labels.labels == actual.scene.labels)
    );
    assert_eq!(
        tree_widget_frame(&actual)
            .unwrap()
            .interaction
            .unwrap()
            .sites
            .len(),
        5,
        "painted empty label does not remove original business label"
    );
}

#[test]
fn default_bundle_is_exact_and_only_actual_scene_label_eligibility_schedules_text_callbacks() {
    for mode in [TreeMode::Tidy, TreeMode::Cluster] {
        for orientation in [
            TreeOrientation::LeftToRight,
            TreeOrientation::RightToLeft,
            TreeOrientation::TopToBottom,
            TreeOrientation::BottomToTop,
            TreeOrientation::Radial,
        ] {
            let o = TreeOptions {
                mode,
                orientation,
                ..Default::default()
            };
            let old = tree_layout(&config(), &raw(), &o).unwrap();
            let new =
                tree_layout_with_factories(&config(), &raw(), &o, &mut TreeFactories::default())
                    .unwrap();
            assert_eq!(old, new);
            assert_eq!(
                render::to_svg(&old.scene).unwrap(),
                render::to_svg(&new.scene).unwrap()
            );
        }
    }
    for (labels, visibility, count) in [
        (false, TreeLabels::All, 0),
        (true, TreeLabels::None, 0),
        (true, TreeLabels::Leaves, 3),
        (true, TreeLabels::Internal, 2),
        (true, TreeLabels::All, 5),
    ] {
        let mut o = TreeOptions::default();
        o.hierarchy.labels = labels;
        o.style.labels.visibility = visibility;
        o.style.links.enabled = false;
        let label_calls = Cell::new(0);
        let edge_calls = Cell::new(0);
        let mut text = |_: &TreeLabelFunctionContext<'_>| {
            label_calls.set(label_calls.get() + 1);
            Ok("text".into())
        };
        let mut width = |_: &TreeLinkFunctionContext<'_>| {
            edge_calls.set(edge_calls.get() + 1);
            Ok(0.)
        };
        let result = tree_layout_with_factories(
            &config(),
            &raw(),
            &o,
            &mut TreeFactories {
                link_thickness: Some(&mut width),
                label: Some(&mut text),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(label_calls.get(), count);
        assert_eq!(result.scene.labels.len(), count);
        assert_eq!(edge_calls.get(), 4, "disabled ink still has computed links");
        assert!(result.links.iter().all(|l| l.mark_id.is_none()));
    }
    let calls = Cell::new(0);
    let mut size = |_: &TreeNodeFunctionContext<'_>| {
        calls.set(calls.get() + 1);
        Ok(0.)
    };
    let empty = tree_layout_with_factories(
        &config(),
        &[],
        &TreeOptions::default(),
        &mut TreeFactories {
            node_size: Some(&mut size),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(empty.nodes.is_empty());
    assert_eq!(calls.get(), 0);
}

#[test]
fn all_source_and_static_options_preflight_then_user_errors_and_results_stop_exactly() {
    let calls = Cell::new(0);
    let mut size = |_: &TreeNodeFunctionContext<'_>| {
        calls.set(calls.get() + 1);
        Ok(10.)
    };
    let mut bad = raw();
    bad[4].value = f64::NAN;
    assert!(
        tree_layout_with_factories(
            &config(),
            &bad,
            &TreeOptions::default(),
            &mut TreeFactories {
                node_size: Some(&mut size),
                ..Default::default()
            }
        )
        .is_err()
    );
    assert_eq!(calls.get(), 0);
    let mut bad = raw();
    bad[4].parent = Some("missing".into());
    assert!(
        tree_layout_with_factories(
            &config(),
            &bad,
            &TreeOptions::default(),
            &mut TreeFactories {
                node_size: Some(&mut size),
                ..Default::default()
            }
        )
        .is_err()
    );
    assert_eq!(calls.get(), 0);
    let mut o = TreeOptions::default();
    o.style.links.width = f64::NAN;
    assert!(
        tree_layout_with_factories(
            &config(),
            &raw(),
            &o,
            &mut TreeFactories {
                node_size: Some(&mut size),
                ..Default::default()
            }
        )
        .is_err()
    );
    assert_eq!(calls.get(), 0);
    let mut size = |_: &TreeNodeFunctionContext<'_>| {
        calls.set(calls.get() + 1);
        if calls.get() == 3 {
            Err(ChartError("independent size stop".into()))
        } else {
            Ok(10.)
        }
    };
    assert!(
        tree_layout_with_factories(
            &config(),
            &raw(),
            &TreeOptions::default(),
            &mut TreeFactories {
                node_size: Some(&mut size),
                ..Default::default()
            }
        )
        .is_err()
    );
    assert_eq!(calls.get(), 3);
    for value in [f64::NAN, -1., 1e12] {
        let mut size = |_: &TreeNodeFunctionContext<'_>| Ok(value);
        assert!(
            tree_layout_with_factories(
                &config(),
                &raw(),
                &TreeOptions::default(),
                &mut TreeFactories {
                    node_size: Some(&mut size),
                    ..Default::default()
                }
            )
            .is_err()
        );
        let mut width = |_: &TreeLinkFunctionContext<'_>| Ok(value);
        assert!(
            tree_layout_with_factories(
                &config(),
                &raw(),
                &TreeOptions::default(),
                &mut TreeFactories {
                    link_thickness: Some(&mut width),
                    ..Default::default()
                }
            )
            .is_err()
        );
    }
    let max = "界".repeat(341) + "a";
    assert_eq!(max.len(), 1024);
    let mut label = |_: &TreeLabelFunctionContext<'_>| Ok(max.clone());
    assert!(
        tree_layout_with_factories(
            &config(),
            &raw(),
            &TreeOptions::default(),
            &mut TreeFactories {
                label: Some(&mut label),
                ..Default::default()
            }
        )
        .is_ok()
    );
    let mut label = |_: &TreeLabelFunctionContext<'_>| Ok(max.clone() + "b");
    assert!(
        tree_layout_with_factories(
            &config(),
            &raw(),
            &TreeOptions::default(),
            &mut TreeFactories {
                label: Some(&mut label),
                ..Default::default()
            }
        )
        .is_err()
    );
}

#[test]
fn source_and_target_link_defaults_follow_resolved_color_even_when_source_node_has_zero_size() {
    for inherit in [TreeLinkColor::Source, TreeLinkColor::Target] {
        let mut o = TreeOptions::default();
        o.style.links.color = inherit;
        let mut size = |_: &TreeNodeFunctionContext<'_>| Ok(0.);
        let mut color =
            |c: &TreeNodeFunctionContext<'_>| Ok(Color(20 + c.source_index as u8, 80, 150, 255));
        let layout = tree_layout_with_factories(
            &config(),
            &raw(),
            &o,
            &mut TreeFactories {
                node_size: Some(&mut size),
                node_color: Some(&mut color),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(layout.nodes.iter().all(|n| n.radius == 0.));
        for link in &layout.links {
            let selected = if inherit == TreeLinkColor::Source {
                link.source_index
            } else {
                link.target_index
            };
            let mark = layout
                .scene
                .marks
                .iter()
                .find(|m| Some(&m.id) == link.mark_id.as_ref())
                .unwrap();
            assert_eq!(
                mark.stroke.unwrap().color,
                layout.nodes[selected].color.unwrap()
            );
        }
        assert_eq!(
            tree_node_widgets(&layout, 0, |_| Ok(Some(PlotWidgetPlacement::Rect(
                LayoutRect {
                    x: 100.,
                    y: 100.,
                    width: 10.,
                    height: 10.
                }
            ))))
            .unwrap()
            .data
            .len(),
            5
        );
    }
}
