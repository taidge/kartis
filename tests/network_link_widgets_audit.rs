//! Independent live-link qualification, source proof and placement contracts.
use std::sync::Arc;

use glory::render::{GraphicPoint, PathCommand};
use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 370.,
        height: 250.,
        margin: Margin {
            left: 31.,
            right: 23.,
            top: 19.,
            bottom: 29.,
        },
        ..Default::default()
    }
}
fn raw() -> Vec<NetworkNode> {
    vec![
        NetworkNode {
            id: "a".into(),
            label: String::new(),
            radius: 3.,
        },
        NetworkNode {
            id: "b".into(),
            label: "B".into(),
            radius: 5.,
        },
        NetworkNode {
            id: "z".into(),
            label: "Z".into(),
            radius: 7.,
        },
    ]
}
fn edges() -> Vec<NetworkLink> {
    vec![
        NetworkLink::new("a", "b", 0.),
        NetworkLink::new("a", "b", 9.),
        NetworkLink::new("b", "b", -0.),
        NetworkLink::new("z", "a", 1.),
    ]
}
fn build(nodes: &[NetworkNode], links: &[NetworkLink]) -> ForceSimulation {
    let mut s = ForceSimulation::new(
        nodes,
        links,
        ForceOptions {
            repulsion: 0.,
            center_strength: 0.,
            collision_strength: 0.,
            link_strength: 0.,
            labels: false,
            ..Default::default()
        },
    )
    .unwrap();
    for (n, position) in nodes.iter().zip([[-0., 0.], [40., 20.], [-50., -10.]]) {
        s.set_position(&n.id, position).unwrap();
    }
    if let Some(node) = nodes.last() {
        s.set_velocity(&node.id, [-0., 0.]).unwrap();
    }
    s
}
fn simulation() -> ForceSimulation {
    build(&raw(), &edges())
}
fn layout(s: &ForceSimulation) -> NetworkLinkLayout {
    network_glyph_layout_with_link_factory(&config(), s, &Default::default(), &mut |c| {
        let mut ink = c.inherit(c.geometry.clone());
        if c.link_index == 0 {
            ink.geometry = Geometry::Path(vec![
                PathCommand::Move(GraphicPoint::new(240., 80.)),
                PathCommand::Line(GraphicPoint::new(260., 80.)),
                PathCommand::Line(GraphicPoint::new(250., 100.)),
                PathCommand::Close,
            ]);
            ink.fill = Some(Color(20, 90, 140, 255));
            ink.stroke = None;
        }
        if c.link_index == 1 {
            ink.opacity = 0.;
        }
        if c.link_index == 2 {
            ink.fill = None;
            ink.fill_paint = None;
            ink.stroke = None;
        }
        Ok(ink)
    })
    .unwrap()
}
fn placement() -> PlotWidgetPlacement {
    PlotWidgetPlacement::AtDatum {
        offset: [-7., -5.],
        measured: TextSize {
            width: 14.,
            height: 10.,
        },
    }
}

#[test]
fn actual_links_keep_zero_self_parallel_noink_and_original_midpoint_without_controls() {
    let s = simulation();
    let l = layout(&s);
    let authored = l.scene().clone();
    let mut order = vec![];
    let p = network_link_widgets(&l, &s, 11, |c| {
        order.push(c.link_index);
        assert!(std::ptr::eq(c.edge, &s.edges()[c.link_index]));
        assert!(std::ptr::eq(c.source, &s.nodes()[c.edge.source]));
        assert!(std::ptr::eq(c.target, &s.nodes()[c.edge.target]));
        assert_eq!(
            c.position,
            [
                c.source_position[0] * 0.5 + c.target_position[0] * 0.5,
                c.source_position[1] * 0.5 + c.target_position[1] * 0.5
            ]
        );
        assert_eq!(c.mark.id, format!("force-link:{}", c.link_index));
        Ok(Some(placement()))
    })
    .unwrap();
    assert_eq!(order, [0, 1, 2, 3]);
    assert_eq!(p.data.len(), 4);
    for slot in &p.layout.slots {
        let d = &p.data[&slot.mark_id];
        let PlotWidgetPlacement::Rect(r) = slot.placement else {
            panic!("Path midpoint must be explicit")
        };
        assert_eq!([r.x + 7., r.y + 5.], d.site.position);
        assert_eq!(slot.revision, 11);
        assert_eq!(
            d.edges[d.source_index].value.to_bits(),
            s.edges()[d.source_index].value.to_bits()
        );
        assert!(Arc::ptr_eq(&d.source, &p.data["force-link:0"].source));
        assert!(Arc::ptr_eq(&d.edges, &p.data["force-link:0"].edges));
    }
    let first = &p.data["force-link:0"].site;
    assert_ne!(
        first.position,
        [
            first.bounds.x + first.bounds.width / 2.,
            first.bounds.y + first.bounds.height / 2.
        ]
    );
    assert_eq!(
        p.data["force-link:2"].edges[2].value.to_bits(),
        (-0_f64).to_bits()
    );
    let frame = network_link_widget_frame(&l).unwrap();
    assert_eq!(frame.scene, authored);
    assert_eq!(l.scene(), &authored);
    assert!(
        frame
            .interaction
            .unwrap()
            .sites
            .iter()
            .all(|site| site.mark_id.starts_with("force-node:"))
    );
}

#[test]
fn mixed_roles_share_current_source_and_follow_links_then_nodes_with_one_budget() {
    let s = simulation();
    let l = layout(&s);
    let mut order = vec![];
    let p = network_widgets(&l, &s, 4, |c| {
        let id = match c {
            NetworkWidgetContext::Link(c) => c.mark.id.clone(),
            NetworkWidgetContext::Node(c) => c.mark.id.clone(),
        };
        order.push(id);
        Ok(Some(placement()))
    })
    .unwrap();
    let expected: Vec<_> = l.scene().marks.iter().map(|m| m.id.clone()).collect();
    assert_eq!(order, expected);
    assert_eq!(
        p.layout
            .slots
            .iter()
            .map(|x| &x.mark_id)
            .collect::<Vec<_>>(),
        expected.iter().collect::<Vec<_>>()
    );
    let NetworkWidgetData::Link(first) = &p.data["force-link:0"] else {
        panic!("link role")
    };
    for d in p.data.values() {
        let source = match d {
            NetworkWidgetData::Link(d) => &d.source,
            NetworkWidgetData::Node(d) => &d.source,
        };
        assert!(Arc::ptr_eq(source, &first.source));
    }
    let many = vec![NetworkLink::new("a", "b", 1.); 127];
    let s = build(&raw()[..2], &many);
    let l = layout(&s);
    let mut calls = 0;
    assert!(
        network_widgets(&l, &s, 0, |_| {
            calls += 1;
            Ok(Some(placement()))
        })
        .is_err()
    );
    assert_eq!(calls, 129);
    let accepted = network_widgets(&l, &s, 0, |c| {
        Ok(match c {
            NetworkWidgetContext::Link(_) => Some(placement()),
            NetworkWidgetContext::Node(c) => (c.node.id == "a").then(placement),
        })
    })
    .unwrap();
    assert_eq!(accepted.data.len(), 128);
}

#[test]
fn all_late_graph_source_conflicts_reject_before_the_first_link_or_node_callback() {
    let s = simulation();
    let l = layout(&s);
    let mut cases = vec![];
    let mut moved = s.clone();
    moved.set_position("z", [-49., -10.]).unwrap();
    cases.push(("late position", moved));
    let mut sign = s.clone();
    sign.set_position("a", [0., 0.]).unwrap();
    cases.push(("world signed zero", sign));
    let mut links = edges();
    links[2].value = 0.;
    cases.push(("edge signed zero", build(&raw(), &links)));
    let mut links = edges();
    links[3].value = 4.;
    cases.push(("late edge value", build(&raw(), &links)));
    let mut links = edges();
    links[3].target = "b".into();
    cases.push(("late target", build(&raw(), &links)));
    let mut links = edges();
    links.swap(0, 1);
    cases.push(("parallel edge order", build(&raw(), &links)));
    cases.push(("edge count", build(&raw(), &edges()[..3])));
    let mut nodes = raw();
    nodes[2].radius = 8.;
    cases.push(("late radius", build(&nodes, &edges())));
    let mut nodes = raw();
    nodes[2].label = "Changed".into();
    cases.push(("late label", build(&nodes, &edges())));
    let mut nodes = raw();
    nodes[2].id = "renamed".into();
    let mut links = edges();
    links[3].source = "renamed".into();
    cases.push(("late identity", build(&nodes, &links)));
    for (name, candidate) in cases {
        let mut calls = 0;
        assert!(
            network_widgets(&l, &candidate, 0, |_| {
                calls += 1;
                Ok(Some(placement()))
            })
            .is_err(),
            "{name}"
        );
        assert_eq!(calls, 0, "{name}");
    }
}

#[test]
fn current_metadata_bits_and_index_identity_have_explicit_not_frozen_edge_semantics() {
    let s = simulation();
    let l = layout(&s);
    let p = network_link_widgets(&l, &s, 0, |_| Ok(Some(placement()))).unwrap();
    let mut next = s.clone();
    next.set_velocity("z", [0., -0.]).unwrap();
    next.set_fixed("b", Some([40., 20.])).unwrap();
    let next = network_link_widgets(&l, &next, 0, |_| Ok(Some(placement()))).unwrap();
    assert_eq!(p.layout, next.layout);
    assert_ne!(p, next);
    assert_eq!(
        p.data["force-link:0"].source[2].velocity[0].to_bits(),
        (-0_f64).to_bits()
    );
    assert_eq!(
        next.data["force-link:0"].source[2].velocity[0].to_bits(),
        0_f64.to_bits()
    );
    let mut links = edges();
    links[2].value = 0.;
    let current = build(&raw(), &links);
    let current_l = layout(&current);
    let current_p =
        network_link_widgets(&current_l, &current, 0, |_| Ok(Some(placement()))).unwrap();
    assert_eq!(l.scene(), current_l.scene());
    assert_eq!(p.layout, current_p.layout);
    assert_ne!(p.data["force-link:0"], current_p.data["force-link:0"]);
    let mut links = edges();
    links.swap(0, 1);
    let current = build(&raw(), &links);
    let current_l = layout(&current);
    let reordered =
        network_link_widgets(&current_l, &current, 0, |_| Ok(Some(placement()))).unwrap();
    assert_eq!(p.layout.slots[0].mark_id, reordered.layout.slots[0].mark_id);
    assert_eq!(reordered.data["force-link:0"].site.value, 9.);
    assert_eq!(p.data["force-link:0"].site.value, 0.);
}

#[test]
fn placement_error_stops_before_node_callbacks_and_empty_links_create_no_role() {
    let s = simulation();
    let l = layout(&s);
    let mut calls = 0;
    let e = network_widgets(&l, &s, 0, |_| {
        calls += 1;
        if calls == 3 {
            Err(ChartError("independent placement stop".into()))
        } else {
            Ok(Some(placement()))
        }
    })
    .unwrap_err();
    assert_eq!(e.0, "independent placement stop");
    assert_eq!(calls, 3);
    let s = build(&raw(), &[]);
    let l = layout(&s);
    let mut calls = 0;
    let empty = network_link_widgets(&l, &s, 0, |_| {
        calls += 1;
        Ok(Some(placement()))
    })
    .unwrap();
    assert_eq!(calls, 0);
    assert!(empty.data.is_empty());
    assert_eq!(network_link_widget_frame(&l).unwrap().scene, *l.scene());
}
