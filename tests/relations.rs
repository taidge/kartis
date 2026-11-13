use kartis::*;
use std::collections::BTreeSet;
fn set(ids: &[&str]) -> BTreeSet<String> {
    ids.iter().map(|s| (*s).into()).collect()
}
#[test]
fn network_relation_edges_are_real_surface_pointer_and_keyboard_data() {
    use kartis::surface::{SurfaceEvent, SurfaceInput, SurfaceModel, SurfaceOptions};
    let nodes = ["a", "b"].map(|id| NetworkNode {
        id: id.into(),
        label: id.into(),
        radius: 8.,
    });
    let links = [NetworkLink {
        source: "a".into(),
        target: "b".into(),
        value: 4.,
    }];
    let mut simulation = ForceSimulation::new(&nodes, &links, Default::default()).unwrap();
    simulation.set_position("a", [-60., 0.]).unwrap();
    simulation.set_position("b", [60., 0.]).unwrap();
    let mut layout = network_relation_layout(
        &ChartConfig::default(),
        &simulation,
        &["ab".into()],
        &Default::default(),
    )
    .unwrap();
    let edge_id = &layout.graph().edge("ab").unwrap().mark_ids[0];
    let edge = layout
        .scene()
        .marks
        .iter()
        .find(|mark| &mark.id == edge_id)
        .unwrap();
    assert_eq!(edge.series, "a");
    assert_eq!(edge.label, "a → b");
    let Geometry::Path(path) = &edge.geometry else {
        panic!("edge path")
    };
    let (glory::render::PathCommand::Move(a), glory::render::PathCommand::Line(b)) =
        (&path[0], &path[1])
    else {
        panic!("edge endpoints")
    };
    let point = ((a.x + b.x) / 2., (a.y + b.y) / 2.);
    let mut model = SurfaceModel::new(layout.scene().clone(), SurfaceOptions::default()).unwrap();
    model.apply(SurfaceInput::Move { id: 1, point }).unwrap();
    assert_eq!(model.hovered(), std::slice::from_ref(edge_id));
    assert_eq!(
        layout.graph().focus_for_mark(edge_id),
        Some(&RelationFocus::Edge("ab".into()))
    );
    model.apply(SurfaceInput::First).unwrap();
    assert_eq!(model.selected(), Some(edge_id.as_str()));
    assert!(
        model
            .apply(SurfaceInput::Activate)
            .unwrap()
            .contains(&SurfaceEvent::Activated(edge_id.clone()))
    );
    // Relation hiding suppresses both endpoint sides, independent of series.
    layout.set_hidden(set(&["b"])).unwrap();
    model.set_scene(layout.scene().clone()).unwrap();
    model.apply(SurfaceInput::Move { id: 1, point }).unwrap();
    assert!(model.hovered().is_empty());
}
fn graph(tree: bool) -> RelationGraph {
    let ids = if tree {
        vec!["r", "a", "b", "a1", "a2", "b1"]
    } else {
        vec!["a", "b", "c", "isolated"]
    };
    let edges = if tree {
        vec![
            ("a", "r", "a"),
            ("b", "r", "b"),
            ("a1", "a", "a1"),
            ("a2", "a", "a2"),
            ("b1", "b", "b1"),
        ]
    } else {
        vec![
            ("ab", "a", "b"),
            ("ab2", "a", "b"),
            ("ba", "b", "a"),
            ("bc", "b", "c"),
            ("aa", "a", "a"),
            ("ca", "c", "a"),
        ]
    };
    RelationGraph::new(
        ids.into_iter().map(RelationNode::new).collect(),
        edges
            .into_iter()
            .map(|(id, a, b)| RelationEdge::new(id, a, b))
            .collect(),
    )
    .unwrap()
}
fn flags(bits: u8) -> TreeRelationOptions {
    TreeRelationOptions {
        ancestor_nodes: bits & 1 != 0,
        descendant_nodes: bits & 2 != 0,
        ancestor_links: bits & 4 != 0,
        descendant_links: bits & 8 != 0,
    }
}
#[test]
fn selection_matches_134_actual_nivo_tree_chord_and_sankey_queries() {
    let (tree, other) = (graph(true), graph(false));
    let mut count = 0;
    for line in include_str!("fixtures/relations-nivo.csv").lines().skip(2) {
        let f = line.split('|').collect::<Vec<_>>();
        let mode = match f[0] {
            "tree" => RelationMode::Tree(flags(f[3].parse().unwrap())),
            "chord" => RelationMode::Chord,
            _ => RelationMode::Sankey,
        };
        let focus = match f[1] {
            "node" => Some(RelationFocus::Node(f[2].into())),
            "edge" => Some(RelationFocus::Edge(f[2].into())),
            _ => None,
        };
        let g = if f[0] == "tree" { &tree } else { &other };
        let r = g
            .related(focus.as_ref(), &mode, &BTreeSet::new(), Default::default())
            .unwrap();
        let ids = |s: &str| {
            s.split(',')
                .filter(|v| !v.is_empty())
                .map(str::to_owned)
                .collect::<BTreeSet<_>>()
        };
        assert_eq!(r.nodes, ids(f[4]), "{line}");
        assert_eq!(r.edges, ids(f[5]), "{line}");
        assert_eq!(r.highlight, f[6] == "1", "{line}");
        count += 1;
    }
    assert_eq!(count, 134);
}
#[test]
fn hidden_tree_topology_and_independent_link_flags_are_explicit() {
    let g = graph(true);
    let mode = RelationMode::Tree(flags(5));
    let hidden = set(&["a"]);
    let focus = RelationFocus::Node("a1".into());
    let visible = g
        .related(Some(&focus), &mode, &hidden, RelationTraversal::Visible)
        .unwrap();
    assert_eq!(visible.nodes, set(&["a1"]));
    assert!(visible.edges.is_empty());
    let through = g
        .related(
            Some(&focus),
            &mode,
            &hidden,
            RelationTraversal::ThroughHidden,
        )
        .unwrap();
    assert_eq!(through.nodes, set(&["a1", "r"]));
    assert!(through.edges.is_empty());
    let descendant_links = g
        .related(
            Some(&RelationFocus::Node("a".into())),
            &RelationMode::Tree(flags(8)),
            &BTreeSet::new(),
            Default::default(),
        )
        .unwrap();
    assert_eq!(descendant_links.nodes, set(&["a"]));
    assert_eq!(descendant_links.edges, set(&["a1", "a2"]));
    assert_eq!(
        g.related(
            Some(&RelationFocus::Node("a".into())),
            &mode,
            &hidden,
            Default::default()
        )
        .unwrap(),
        RelationSet::default()
    );
    assert!(
        g.related(
            Some(&RelationFocus::Edge("a".into())),
            &mode,
            &BTreeSet::new(),
            Default::default()
        )
        .is_err()
    );
}
#[test]
fn cycles_self_links_parallel_links_and_chord_isolation_keep_distinct_semantics() {
    let g = graph(false);
    assert!(!g.is_forest());
    let focus = RelationFocus::Node("b".into());
    let neighbors = g
        .related(
            Some(&focus),
            &RelationMode::Neighbors,
            &BTreeSet::new(),
            Default::default(),
        )
        .unwrap();
    assert_eq!(neighbors.nodes, set(&["a", "b", "c"]));
    assert_eq!(neighbors.edges, set(&["ab", "ab2", "ba", "bc"]));
    let parallel = g
        .related(
            Some(&RelationFocus::Edge("ab2".into())),
            &RelationMode::Sankey,
            &BTreeSet::new(),
            Default::default(),
        )
        .unwrap();
    assert_eq!(parallel.nodes, set(&["a", "b"]));
    assert_eq!(parallel.edges, set(&["ab", "ab2"]));
    let chord = g
        .related(
            Some(&focus),
            &RelationMode::Chord,
            &BTreeSet::new(),
            Default::default(),
        )
        .unwrap();
    assert_eq!(chord.nodes, set(&["b"]));
    assert_eq!(chord.edges, neighbors.edges);
    let isolated = g
        .related(
            Some(&RelationFocus::Node("isolated".into())),
            &RelationMode::Chord,
            &BTreeSet::new(),
            Default::default(),
        )
        .unwrap();
    assert_eq!(isolated.nodes, set(&["isolated"]));
    assert!(!isolated.highlight);
    assert!(
        g.related(
            Some(&focus),
            &RelationMode::Tree(Default::default()),
            &BTreeSet::new(),
            Default::default()
        )
        .is_err()
    );
    assert_eq!(g.incoming("b").unwrap().len(), 2);
    assert_eq!(g.outgoing("a").unwrap().len(), 3);
    assert!(g.incoming("unknown").is_none());
}
fn layout() -> RelationLayout {
    use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as C, StrokeCap};
    let mut scene = ChartConfig {
        width: 400.,
        height: 240.,
        margin: Margin {
            top: 10.,
            right: 10.,
            bottom: 10.,
            left: 10.,
        },
        ..Default::default()
    }
    .scene()
    .unwrap();
    let mut nodes = vec![];
    for (i, id) in ["a", "b", "c", "isolated"].into_iter().enumerate() {
        let x = 25. + i as f64 * 90.;
        let mid = format!("n:{id}");
        let mut mark = Mark::new(
            &mid,
            Geometry::Rect {
                x,
                y: 80.,
                width: 50.,
                height: 40.,
            },
            Color(200, 20, 30, 255),
        );
        mark.series = "all-shared".into();
        mark.value = (i + 1) as f64;
        mark.opacity = 0.8;
        scene.marks.push(mark);
        let mut node = RelationNode::new(id);
        node.mark_ids.push(mid);
        node.label_indices.push(scene.labels.len());
        scene
            .label_series
            .insert(scene.labels.len(), "all-shared".into());
        scene.label_clips.insert(
            scene.labels.len(),
            ClipPath {
                geometry: Geometry::Rect {
                    x: 0.,
                    y: 0.,
                    width: 400.,
                    height: 240.,
                },
                rule: ClipRule::NonZero,
            },
        );
        scene.labels.push(Label {
            x,
            y: 150.,
            text: id.into(),
            color: Color(20, 30, 40, 255),
            size: 12.,
            anchor: TextAnchor::Start,
            rotation: 0.,
        });
        nodes.push(node);
    }
    let mut edges = vec![];
    for (id, a, b, y) in [
        ("ab", "a", "b", 35.),
        ("ab2", "a", "b", 45.),
        ("bc", "b", "c", 55.),
    ] {
        let mut mark = Mark::new(
            format!("e:{id}"),
            Geometry::Path(vec![C::Move(P::new(50., y)), C::Line(P::new(250., y))]),
            Color(0, 0, 0, 255),
        );
        mark.fill = None;
        mark.stroke = Some(GraphicStroke {
            color: Color(50, 60, 70, 255),
            width: 3.,
            cap: StrokeCap::Butt,
        });
        mark.stroke_dash = Some(StrokeDash {
            segments: vec![8., 4.],
            offset: 0.,
        });
        mark.series = "all-shared".into();
        scene.marks.push(mark);
        let mut edge = RelationEdge::new(id, a, b);
        edge.mark_ids.push(format!("e:{id}"));
        edges.push(edge);
    }
    let mut decoration = Mark::new(
        "custom decoration",
        Geometry::Rect {
            x: 8.,
            y: 8.,
            width: 5.,
            height: 5.,
        },
        Color(0, 160, 60, 255),
    );
    decoration.label.clear();
    scene.marks.push(decoration);
    scene.draw_order = scene.draw_items().into_iter().rev().collect();
    RelationLayout::new(
        scene,
        RelationGraph::new(nodes, edges).unwrap(),
        RelationOptions::default(),
    )
    .unwrap()
}
#[test]
fn controlled_paint_uses_bindings_not_series_and_restores_authored_styles() {
    let mut l = layout();
    let authored = l.scene().clone();
    l.set_focus(Some(RelationFocus::Node("a".into()))).unwrap();
    assert_eq!(l.selection().nodes, set(&["a", "b"]));
    let c = l.scene().marks.iter().find(|m| m.id == "n:c").unwrap();
    assert!((c.opacity - 0.16).abs() < 1e-14);
    assert_eq!(
        l.scene()
            .marks
            .iter()
            .find(|m| m.id == "n:a")
            .unwrap()
            .opacity,
        0.8
    );
    assert_eq!(l.scene().labels[2].color.3, 51);
    assert_eq!(
        l.scene().marks.last().unwrap(),
        authored.marks.last().unwrap()
    );
    l.set_focus(Some(RelationFocus::Node("a".into()))).unwrap();
    assert!(
        (l.scene()
            .marks
            .iter()
            .find(|m| m.id == "n:c")
            .unwrap()
            .opacity
            - 0.16)
            .abs()
            < 1e-14
    );
    let mut options = l.options().clone();
    options.active_node.absolute_opacity = Some(1.);
    options.active_node.fill = Some(Color(10, 90, 200, 255));
    options.active_edge.stroke_width = Some(0.);
    l.set_options(options).unwrap();
    let active = l.scene().marks.iter().find(|m| m.id == "n:a").unwrap();
    assert_eq!(active.opacity, 1.);
    assert_eq!(active.fill, Some(Color(10, 90, 200, 255)));
    let edge = l.scene().marks.iter().find(|m| m.id == "e:ab").unwrap();
    assert!(edge.stroke.is_none() && edge.stroke_dash.is_none());
    l.set_focus(None).unwrap();
    assert_eq!(l.scene(), &authored);
    let mut gradient = authored.clone();
    gradient
        .marks
        .iter_mut()
        .find(|m| m.id == "n:a")
        .unwrap()
        .fill_paint = Some(FillPaint::Linear {
        start: [25., 80.],
        end: [75., 120.],
        stops: vec![
            GradientStop::new(0., Color(250, 0, 20, 255)),
            GradientStop::new(1., Color(20, 80, 240, 255)),
        ],
    });
    let mut g = RelationLayout::new(
        gradient.clone(),
        l.graph().clone(),
        RelationOptions {
            active_node: RelationPaint {
                fill: Some(Color(20, 100, 80, 255)),
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .unwrap();
    g.set_focus(Some(RelationFocus::Node("a".into()))).unwrap();
    assert!(
        g.scene()
            .marks
            .iter()
            .find(|m| m.id == "n:a")
            .unwrap()
            .fill_paint
            .is_none()
    );
    g.set_focus(None).unwrap();
    assert_eq!(g.scene(), &gradient);
}
#[test]
fn hidden_endpoints_remove_both_sides_and_remap_label_clips_and_draw_order() {
    let mut l = layout();
    l.set_focus(Some(RelationFocus::Edge("ab".into()))).unwrap();
    l.set_hidden(set(&["b"])).unwrap();
    assert!(l.focus().is_none());
    assert!(
        l.scene()
            .marks
            .iter()
            .all(|m| !matches!(m.id.as_str(), "n:b" | "e:ab" | "e:ab2" | "e:bc"))
    );
    assert_eq!(
        l.scene()
            .labels
            .iter()
            .map(|l| l.text.as_str())
            .collect::<Vec<_>>(),
        ["a", "c", "isolated"]
    );
    assert_eq!(l.scene().label_clips.len(), 3);
    assert_eq!(
        l.scene().label_series.keys().copied().collect::<Vec<_>>(),
        [0, 1, 2]
    );
    l.scene().validate().unwrap();
    let once = l.clone();
    l.set_hidden(set(&["b"])).unwrap();
    assert_eq!(l, once);
    l.set_hidden(BTreeSet::new()).unwrap();
    assert_eq!(l.scene(), l.authored_scene());
}
#[test]
fn bad_updates_reject_atomically_and_replacement_rebinds_current_geometry() {
    let mut l = layout();
    l.set_focus(Some(RelationFocus::Node("a".into()))).unwrap();
    let before = l.clone();
    assert!(
        l.set_focus(Some(RelationFocus::Node("unknown".into())))
            .is_err()
    );
    assert_eq!(l, before);
    assert!(l.set_hidden(set(&["unknown"])).is_err());
    assert_eq!(l, before);
    let mut invalid = l.options().clone();
    invalid.active_node.opacity = f64::NAN;
    assert!(l.set_options(invalid).is_err());
    assert_eq!(l, before);
    let next = layout();
    let mut source = next.authored_scene().clone();
    let m = source.marks.iter_mut().find(|m| m.id == "n:a").unwrap();
    m.value = 99.;
    m.label = "A current 99".into();
    m.geometry = Geometry::Rect {
        x: 60.,
        y: 80.,
        width: 50.,
        height: 40.,
    };
    let next = RelationLayout::new(source, next.graph().clone(), next.options().clone()).unwrap();
    l.replace(next).unwrap();
    assert_eq!(l.focus(), Some(&RelationFocus::Node("a".into())));
    let m = l.scene().marks.iter().find(|m| m.id == "n:a").unwrap();
    assert_eq!(m.value, 99.);
    assert_eq!(m.label, "A current 99");
    assert_eq!(
        m.geometry,
        Geometry::Rect {
            x: 60.,
            y: 80.,
            width: 50.,
            height: 40.
        }
    );
    l.set_hidden(set(&["isolated"])).unwrap();
    l.replace(
        RelationLayout::new(
            ChartConfig::default().scene().unwrap(),
            RelationGraph::new(vec![], vec![]).unwrap(),
            RelationOptions::default(),
        )
        .unwrap(),
    )
    .unwrap();
    assert!(l.focus().is_none() && l.hidden().is_empty() && l.scene().marks.is_empty());
    let empty = ChartConfig::default().scene().unwrap();
    let mut tree = RelationLayout::new(
        empty.clone(),
        graph(true),
        RelationOptions {
            mode: RelationMode::Tree(Default::default()),
            ..Default::default()
        },
    )
    .unwrap();
    let before = tree.clone();
    let cyclic = RelationLayout::new(empty, graph(false), Default::default()).unwrap();
    assert!(tree.replace(cyclic).is_err());
    assert_eq!(tree, before);
}

#[test]
fn adapters_use_real_tree_force_chord_and_sankey_producer_ids() {
    let c = ChartConfig::default();
    let data = vec![
        HierarchyDatum::new("r", None, 0.),
        HierarchyDatum::new("a", Some("r".into()), 2.),
        HierarchyDatum::new("b", Some("r".into()), 3.),
    ];
    let mut tree = tree_relation_layout(
        &c,
        &data,
        &TreeOptions::default(),
        &RelationOptions {
            mode: RelationMode::Tree(Default::default()),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        tree.graph().focus_for_mark("node:a"),
        Some(&RelationFocus::Node("a".into()))
    );
    assert_eq!(tree.graph().edge("a").unwrap().source, "r");
    tree.set_focus(Some(RelationFocus::Node("a".into())))
        .unwrap();
    assert_eq!(tree.selection().nodes, set(&["a", "r"]));
    let nodes = vec![
        NetworkNode {
            id: "a".into(),
            label: "A".into(),
            radius: 8.,
        },
        NetworkNode {
            id: "b".into(),
            label: "B".into(),
            radius: 8.,
        },
        NetworkNode {
            id: "c".into(),
            label: "C".into(),
            radius: 8.,
        },
    ];
    let links = vec![
        NetworkLink {
            source: "a".into(),
            target: "b".into(),
            value: 2.,
        },
        NetworkLink {
            source: "b".into(),
            target: "c".into(),
            value: 3.,
        },
    ];
    let mut sim = ForceSimulation::new(&nodes, &links, Default::default()).unwrap();
    let edge_ids = vec!["ab".into(), "bc".into()];
    let mut net = network_relation_layout(&c, &sim, &edge_ids, &Default::default()).unwrap();
    assert_eq!(
        net.graph().focus_for_mark("force-node:b"),
        Some(&RelationFocus::Node("b".into()))
    );
    net.set_focus(Some(RelationFocus::Node("b".into())))
        .unwrap();
    sim.set_position("b", [30., 40.]).unwrap();
    net.replace(
        sim.scene_with_relations(&c, &edge_ids, &Default::default())
            .unwrap(),
    )
    .unwrap();
    assert_eq!(net.selection().nodes, set(&["a", "b", "c"]));
    assert!(network_relation_layout(&c, &sim, &[], &Default::default()).is_err());
    let ids = vec!["a:b".into(), "c".into(), "a".into(), "b:c".into()];
    let matrix = vec![
        vec![0., 2., 0., 0.],
        vec![3., 0., 0., 0.],
        vec![0., 0., 0., 4.],
        vec![0., 0., 5., 0.],
    ];
    let mut chord = chord_relation_layout(
        &c,
        &ids,
        &matrix,
        0.03,
        &RelationOptions {
            mode: RelationMode::Chord,
            ..Default::default()
        },
    )
    .unwrap();
    assert_ne!(
        chord_relation_edge_id("a:b", "c"),
        chord_relation_edge_id("a", "b:c")
    );
    chord
        .set_focus(Some(RelationFocus::Edge(chord_relation_edge_id(
            "a:b", "c",
        ))))
        .unwrap();
    assert_eq!(chord.selection().nodes, set(&["a:b", "c"]));
    chord.set_hidden(set(&["c"])).unwrap();
    assert!(
        chord
            .scene()
            .marks
            .iter()
            .all(|m| m.id != format!("relation-chord-link:{}", chord_relation_edge_id("a:b", "c")))
    );
    let sn = vec![FlowNode::new("a"), FlowNode::new("b"), FlowNode::new("c")];
    let sl = vec![
        SankeyStyleLink::new("ab", "a", "b", 2.),
        SankeyStyleLink::new("ab2", "a", "b", 3.),
        SankeyStyleLink::new("bc", "b", "c", 5.),
    ];
    let solved = sankey_with_style(&c, &sn, &sl, &Default::default(), &Default::default()).unwrap();
    let mut related = solved
        .relation_layout(&RelationOptions {
            mode: RelationMode::Sankey,
            ..Default::default()
        })
        .unwrap();
    related
        .set_focus(Some(RelationFocus::Edge("ab".into())))
        .unwrap();
    assert_eq!(related.selection().edges, set(&["ab", "ab2"]));
    related.set_hidden(set(&["b"])).unwrap();
    assert!(related.scene().marks.iter().all(|m| {
        !related
            .graph()
            .edges()
            .iter()
            .any(|e| e.mark_ids.contains(&m.id))
    }));
    let mut cycle = sl.clone();
    cycle.push(SankeyStyleLink::new("ca", "c", "a", 3.));
    let solved =
        circular_sankey_with_style(&c, &sn, &cycle, &Default::default(), &Default::default())
            .unwrap();
    let mut relations = solved
        .relation_layout(&RelationOptions {
            mode: RelationMode::Sankey,
            ..Default::default()
        })
        .unwrap();
    assert!(!relations.graph().is_forest());
    relations
        .set_focus(Some(RelationFocus::Node("a".into())))
        .unwrap();
    assert_eq!(relations.selection().nodes, set(&["a", "b", "c"]));
    assert_eq!(relations.selection().edges, set(&["ab", "ab2", "ca"]));
    relations
        .set_focus(Some(RelationFocus::Edge("ca".into())))
        .unwrap();
    assert_eq!(relations.selection().edges, set(&["ca"]));
}

#[test]
fn graph_admission_limits_and_ambiguous_bindings_reject_before_publication() {
    assert!(
        RelationGraph::new(vec![RelationNode::new("x"), RelationNode::new("x")], vec![]).is_err()
    );
    assert!(
        RelationGraph::new(
            vec![RelationNode::new("x")],
            vec![RelationEdge::new("e", "x", "missing")]
        )
        .is_err()
    );
    let mut a = RelationNode::new("a");
    a.mark_ids = vec!["same".into()];
    let mut b = RelationNode::new("b");
    b.mark_ids = a.mark_ids.clone();
    assert!(RelationGraph::new(vec![a, b], vec![]).is_err());
    let mut missing = RelationNode::new("x");
    missing.mark_ids = vec!["absent".into()];
    assert!(
        RelationLayout::new(
            ChartConfig::default().scene().unwrap(),
            RelationGraph::new(vec![missing], vec![]).unwrap(),
            Default::default()
        )
        .is_err()
    );
    assert!(
        RelationGraph::new(
            (0..=MAX_RELATION_NODES)
                .map(|i| RelationNode::new(format!("n{i}")))
                .collect(),
            vec![]
        )
        .is_err()
    );
    let n = MAX_RELATION_NODES;
    let chain = RelationGraph::new(
        (0..n).map(|i| RelationNode::new(format!("n{i}"))).collect(),
        (1..n)
            .map(|i| RelationEdge::new(format!("e{i}"), format!("n{}", i - 1), format!("n{i}")))
            .collect(),
    )
    .unwrap();
    let r = chain
        .related(
            Some(&RelationFocus::Node(format!("n{}", n - 1))),
            &RelationMode::Tree(flags(5)),
            &BTreeSet::new(),
            Default::default(),
        )
        .unwrap();
    assert_eq!(r.nodes.len(), n);
    assert_eq!(r.edges.len(), n - 1);
}

#[cfg(feature = "raster")]
#[test]
fn actual_cpu_highlight_and_hidden_remove_owned_ink_and_preserve_custom_layer() {
    use kartis::render::raster::{FontRegistry, RasterOptions, render_raster};
    let mut l = layout();
    let mut source = l.authored_scene().clone();
    source.labels.clear();
    source.label_clips.clear();
    source.label_series.clear();
    source.draw_order.clear();
    let mut nodes = l.graph().nodes().to_vec();
    for n in &mut nodes {
        n.label_indices.clear();
    }
    l = RelationLayout::new(
        source,
        RelationGraph::new(nodes, l.graph().edges().to_vec()).unwrap(),
        Default::default(),
    )
    .unwrap();
    let render = |l: &RelationLayout| {
        render_raster(l.scene(), &FontRegistry::new(), &RasterOptions::default()).unwrap()
    };
    let base = render(&l);
    l.set_focus(Some(RelationFocus::Node("a".into()))).unwrap();
    let focused = render(&l);
    let pixel = |image: &kartis::render::raster::RenderedImage, x: usize, y: usize| {
        let i = (y * image.width as usize + x) * 4;
        image.rgba[i..i + 4].to_vec()
    };
    assert_eq!(pixel(&base, 45, 100), pixel(&focused, 45, 100));
    let dim = pixel(&focused, 225, 100);
    assert!(dim[1] > 210 && dim[2] > 210);
    assert_eq!(pixel(&base, 10, 10), pixel(&focused, 10, 10));
    l.set_hidden(set(&["c"])).unwrap();
    let hidden = render(&l);
    assert_eq!(pixel(&hidden, 225, 100), [255, 255, 255, 255]);
}
