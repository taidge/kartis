//! Independent typed Tree provenance and physical-role acceptance.
use std::sync::Arc;

use glory::render::{GraphicPoint as P, PathCommand as C};
use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 420.,
        height: 280.,
        margin: Margin {
            left: 31.,
            right: 27.,
            top: 23.,
            bottom: 29.,
        },
        ..Default::default()
    }
}
fn raw() -> Vec<HierarchyDatum> {
    vec![
        HierarchyDatum::new("leaf-b", Some("branch".into()), 3.),
        HierarchyDatum::new("root", None, -0.),
        HierarchyDatum::new("branch", Some("root".into()), 0.),
        HierarchyDatum::new("zero", Some("root".into()), -0.),
        HierarchyDatum::new("leaf-c", Some("branch".into()), 4.),
    ]
}
fn place() -> PlotWidgetPlacement {
    PlotWidgetPlacement::AtDatum {
        offset: [-6., -4.],
        measured: TextSize {
            width: 12.,
            height: 8.,
        },
    }
}
fn all(layout: &TreeLayout) -> PlotWidgetsData<TreeWidgetData> {
    tree_widgets(layout, 17, |_| Ok(Some(place()))).unwrap()
}

#[test]
fn complete_original_source_aggregate_topology_and_shared_pools_follow_actual_scene_order() {
    let data = raw();
    let mut layout = tree_layout(&config(), &data, &TreeOptions::default()).unwrap();
    assert_eq!(layout.root_index(), Some(1));
    assert_eq!(
        layout
            .nodes
            .iter()
            .map(|n| n.source_index)
            .collect::<Vec<_>>(),
        [0, 1, 2, 3, 4]
    );
    assert_eq!(layout.nodes[1].raw_value.to_bits(), (-0f64).to_bits());
    assert_eq!(layout.nodes[1].value, 7.);
    assert_eq!(layout.nodes[1].children, [2, 3]);
    assert_eq!(layout.nodes[2].children, [0, 4]);
    assert_eq!(layout.nodes[1].height, 2);
    assert_eq!(layout.nodes[0].parent_index, Some(2));
    assert_eq!(layout.nodes[0].depth, 2);
    // Current labels are actual Scene-owned output, not regenerated from raw.
    layout.scene.labels[0].text = "Current label".into();
    let scene = layout.scene.clone();
    let packet = all(&layout);
    assert_eq!(packet.layout.slots.len(), 9);
    assert_eq!(
        packet
            .layout
            .slots
            .iter()
            .map(|s| s.mark_id.as_str())
            .collect::<Vec<_>>(),
        scene
            .paint_marks()
            .map(|m| m.id.as_str())
            .collect::<Vec<_>>()
    );
    let TreeWidgetData::Link(first) = packet.data.get("link:branch").unwrap() else {
        panic!("link expected")
    };
    assert_eq!(&*first.source, &data);
    assert_eq!(first.source[1].value.to_bits(), (-0f64).to_bits());
    assert_eq!(first.labels.labels, scene.labels);
    assert_eq!(first.labels.clips, scene.label_clips);
    assert_eq!(first.labels.series, scene.label_series);
    for value in packet.data.values() {
        let (source, nodes, links, labels, position) = match value {
            TreeWidgetData::Node(d) => {
                assert_eq!(d.node, layout.nodes[d.source_index]);
                (&d.source, &d.nodes, &d.links, &d.labels, d.node.position)
            }
            TreeWidgetData::Link(d) => {
                assert_eq!(d.link, layout.links[d.link_index]);
                assert_eq!(
                    d.link.source_position,
                    layout.nodes[d.link.source_index].position
                );
                assert_eq!(
                    d.link.target_position,
                    layout.nodes[d.link.target_index].position
                );
                (&d.source, &d.nodes, &d.links, &d.labels, d.link.position)
            }
        };
        assert!(Arc::ptr_eq(source, &first.source));
        assert!(Arc::ptr_eq(nodes, &first.nodes));
        assert!(Arc::ptr_eq(links, &first.links));
        assert!(Arc::ptr_eq(labels, &first.labels));
        let id = match value {
            TreeWidgetData::Node(d) => d.node.mark_id.as_ref().unwrap(),
            TreeWidgetData::Link(d) => d.link.mark_id.as_ref().unwrap(),
        };
        let slot = packet
            .layout
            .slots
            .iter()
            .find(|s| &s.mark_id == id)
            .unwrap();
        assert_eq!(
            slot.placement,
            PlotWidgetPlacement::Rect(LayoutRect {
                x: position[0] - 6.,
                y: position[1] - 4.,
                width: 12.,
                height: 8.
            })
        );
    }
    let frame = tree_widget_frame(&layout).unwrap();
    assert_eq!(frame.scene, scene);
    assert_eq!(frame.plot, Some(ClipRect::plot(&config())));
    assert_eq!(
        frame.interaction.as_ref().unwrap().sites.len(),
        5,
        "only canonical nodes, never decorative links"
    );
    assert_eq!(
        frame.interaction.unwrap().picking_profile,
        PickingProfile::PaintedGeometry
    );
    assert_eq!(layout.scene, scene);
}

#[test]
fn public_metadata_mark_and_late_scene_conflicts_reject_before_every_callback() {
    type Mutation = Box<dyn Fn(&mut TreeLayout)>;
    let mutations: Vec<Mutation> = vec![
        Box::new(|l| l.plot.x += 1.),
        Box::new(|l| l.nodes.swap(0, 4)),
        Box::new(|l| l.nodes[4].source_index = 0),
        Box::new(|l| l.nodes[4].parent_index = None),
        Box::new(|l| l.nodes[2].children.clear()),
        Box::new(|l| l.nodes[4].depth += 1),
        Box::new(|l| l.nodes[2].height += 1),
        Box::new(|l| l.nodes[3].raw_value = 0.),
        Box::new(|l| l.nodes[4].value += 1.),
        Box::new(|l| l.nodes[4].position[0] += 1.),
        Box::new(|l| l.nodes[4].radius += 1.),
        Box::new(|l| l.nodes[4].style.opacity = 0.5),
        Box::new(|l| l.nodes[4].color = Some(Color(1, 2, 3, 255))),
        Box::new(|l| l.nodes[4].mark_id = None),
        Box::new(|l| l.links[3].link_index = 0),
        Box::new(|l| l.links[3].source_index = 0),
        Box::new(|l| l.links[3].target_index = 0),
        Box::new(|l| l.links[3].source_position[0] += 1.),
        Box::new(|l| l.links[3].target_position[1] += 1.),
        Box::new(|l| l.links[3].position[0] += 1.),
        Box::new(|l| l.links[3].mark_id = None),
        Box::new(|l| {
            l.scene
                .marks
                .iter_mut()
                .find(|m| m.id == "node:leaf-c")
                .unwrap()
                .value += 1.
        }),
        Box::new(|l| {
            l.scene
                .marks
                .iter_mut()
                .find(|m| m.id == "node:leaf-c")
                .unwrap()
                .series = "wrong".into()
        }),
        Box::new(|l| {
            l.scene
                .marks
                .iter_mut()
                .find(|m| m.id == "node:leaf-c")
                .unwrap()
                .label = "wrong".into()
        }),
        Box::new(|l| {
            l.scene
                .marks
                .iter_mut()
                .find(|m| m.id == "link:leaf-c")
                .unwrap()
                .label = "fake business owner".into()
        }),
        Box::new(|l| {
            l.scene
                .marks
                .iter_mut()
                .find(|m| m.id == "node:leaf-c")
                .unwrap()
                .geometry = Geometry::Circle {
                x: 10.,
                y: 10.,
                radius: 4.,
            }
        }),
        Box::new(|l| {
            l.scene.label_series.insert(0, "missing-owner".into());
        }),
        Box::new(|l| l.scene.marks.last_mut().unwrap().opacity = f64::NAN),
        Box::new(|l| l.scene.width += 1.),
        Box::new(|l| {
            l.scene.marks.push(Mark::new(
                "node:orphan",
                Geometry::Circle {
                    x: 100.,
                    y: 100.,
                    radius: 3.,
                },
                Color(1, 2, 3, 255),
            ))
        }),
    ];
    for (index, mutate) in mutations.iter().enumerate() {
        let mut layout = tree_layout(&config(), &raw(), &TreeOptions::default()).unwrap();
        mutate(&mut layout);
        let mut calls = 0;
        assert!(
            tree_widgets(&layout, 0, |_| {
                calls += 1;
                Ok(Some(place()))
            })
            .is_err(),
            "mutation {index}"
        );
        assert_eq!(calls, 0, "late mutation {index} must not reach callback");
        assert!(
            tree_widget_frame(&layout).is_err(),
            "frame mutation {index}"
        );
    }
}

#[test]
fn real_zero_no_ink_empty_label_and_disabled_links_keep_distinct_role_and_control_eligibility() {
    let mut source = raw();
    source[3].label.clear();
    for mode in [TreeMode::Tidy, TreeMode::Cluster] {
        for orientation in [
            TreeOrientation::LeftToRight,
            TreeOrientation::RightToLeft,
            TreeOrientation::TopToBottom,
            TreeOrientation::BottomToTop,
            TreeOrientation::Radial,
        ] {
            let mut options = TreeOptions {
                mode,
                orientation,
                node_radius: 0.,
                ..Default::default()
            };
            options.style.links.enabled = false;
            options.style.nodes.fill = TreeNodeFill::None;
            options.style.nodes.opacity = 0.;
            let layout = tree_layout(&config(), &source, &options).unwrap();
            assert_eq!(layout.nodes.len(), 5);
            assert_eq!(layout.links.len(), 4);
            assert!(layout.links.iter().all(|l| l.mark_id.is_none()));
            assert!(
                layout
                    .nodes
                    .iter()
                    .all(|n| n.mark_id.is_some() && n.radius == 0.)
            );
            let nodes = tree_node_widgets(&layout, 0, |_| Ok(Some(place()))).unwrap();
            assert_eq!(
                nodes.data.len(),
                5,
                "actual zero/noink nodes remain component roles"
            );
            let mut callbacks = 0;
            let links = tree_link_widgets(&layout, 0, |_| {
                callbacks += 1;
                Ok(Some(place()))
            })
            .unwrap();
            assert!(links.data.is_empty());
            assert_eq!(callbacks, 0);
            assert_eq!(
                tree_widget_frame(&layout)
                    .unwrap()
                    .interaction
                    .unwrap()
                    .sites
                    .len(),
                4,
                "empty label adds no fake canonical control"
            );
        }
    }
    let empty = tree_layout(&config(), &[], &TreeOptions::default()).unwrap();
    assert!(all(&empty).data.is_empty());
    assert_eq!(empty.root_index(), None);
}

#[test]
fn custom_paths_keep_original_origins_and_mixed_slots_share_one_budget() {
    let mut layout = tree_layout(&config(), &raw(), &TreeOptions::default()).unwrap();
    for id in ["node:zero", "link:zero"] {
        let mark = layout.scene.marks.iter_mut().find(|m| m.id == id).unwrap();
        mark.geometry = Geometry::Path(vec![
            C::Move(P::new(210., 190.)),
            C::Line(P::new(230., 190.)),
            C::Line(P::new(220., 205.)),
            C::Close,
        ]);
        mark.fill = Some(Color(90, 30, 180, 255));
        mark.stroke = None;
    }
    let packet = all(&layout);
    let TreeWidgetData::Node(node) = &packet.data["node:zero"] else {
        panic!("node")
    };
    assert_eq!(node.node.position, layout.nodes[3].position);
    let frame = tree_widget_frame(&layout).unwrap();
    let mark = frame
        .scene
        .marks
        .iter()
        .find(|m| m.id == "node:zero")
        .unwrap();
    let anchor = frame
        .interaction
        .as_ref()
        .unwrap()
        .anchor_for(mark)
        .unwrap()
        .unwrap();
    assert_eq!(anchor, layout.nodes[3].position);
    let mut calls = 0;
    assert!(
        tree_widgets(&layout, 0, |_| {
            calls += 1;
            if calls == 3 {
                Err(ChartError("independent stop".into()))
            } else {
                Ok(Some(place()))
            }
        })
        .is_err()
    );
    assert_eq!(calls, 3);
    let mut large = vec![HierarchyDatum::new("r", None, 0.)];
    large.extend((0..64).map(|i| HierarchyDatum::new(format!("n{i}"), Some("r".into()), 1.)));
    let layout = tree_layout(&config(), &large, &TreeOptions::default()).unwrap();
    let mut calls = 0;
    assert!(
        tree_widgets(&layout, 0, |_| {
            calls += 1;
            Ok(Some(place()))
        })
        .is_err()
    );
    assert_eq!(calls, 129);
    let selected = tree_widgets(&layout, 0, |c| {
        Ok((!matches!(c, TreeWidgetContext::Node(n) if n.datum.id == "r")).then_some(place()))
    })
    .unwrap();
    assert_eq!(selected.layout.slots.len(), 128);
}
