use glory::render::PathCommand as Cmd;
use kartis::*;
use std::collections::BTreeMap;
fn nodes(ids: &[&str]) -> Vec<FlowNode> {
    ids.iter().map(|id| FlowNode::new(*id)).collect()
}
fn edges(data: &[(&str, &str, &str, f64)]) -> Vec<CircularFlowLink> {
    data.iter()
        .map(|(id, s, t, v)| CircularFlowLink::new(*id, *s, *t, *v))
        .collect()
}
fn options() -> CircularSankeyOptions {
    CircularSankeyOptions {
        appearance: SankeyOptions {
            labels: false,
            ..Default::default()
        },
        ..Default::default()
    }
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-8, "{a} != {b}");
}
fn assert_ports(layout: &CircularSankeyLayout) {
    for node in &layout.nodes {
        close(node.value, node.inflow.max(node.outflow));
        for incoming in [false, true] {
            let mut ports = layout
                .links
                .iter()
                .filter(|e| {
                    if incoming {
                        e.target == node.id
                    } else {
                        e.source == node.id
                    }
                })
                .map(|e| {
                    let p = if incoming {
                        e.target_port
                    } else {
                        e.source_port
                    };
                    close(
                        p[0],
                        node.rect.x + if incoming { 0. } else { node.rect.width },
                    );
                    assert!(p[1] - e.width / 2. >= node.rect.y - 1e-8);
                    assert!(p[1] + e.width / 2. <= node.rect.y + node.rect.height + 1e-8);
                    (p[1] - e.width / 2., p[1] + e.width / 2., e.width)
                })
                .collect::<Vec<_>>();
            ports.sort_by(|a, b| a.0.total_cmp(&b.0));
            for pair in ports.windows(2) {
                assert!(
                    pair[0].1 <= pair[1].0 + 1e-8,
                    "overlapping node ports: {}",
                    node.id
                );
            }
            let flow = if incoming { node.inflow } else { node.outflow };
            if flow == node.value {
                close(ports.iter().map(|p| p.2).sum(), node.rect.height);
            }
        }
    }
    for a in &layout.links {
        for b in &layout.links {
            if a.value > 0. && b.value > 0. {
                close(a.width / a.value, b.width / b.value);
            }
        }
    }
}
#[test]
fn actual_cycles_self_links_and_parallel_links_are_kept_with_conserved_ports() {
    let c = ChartConfig::default();
    let n = nodes(&["A", "B", "C", "D", "isolate"]);
    let e = edges(&[
        ("ab", "A", "B", 8.),
        ("bc", "B", "C", 6.),
        ("ca", "C", "A", 4.),
        ("aa", "A", "A", 2.),
        ("aa2", "A", "A", 1.),
        ("bc2", "B", "C", 2.),
        ("cd", "C", "D", 4.),
    ]);
    let layout = layout_sankey_circular(&c, &n, &e, &options()).unwrap();
    assert_eq!(layout.links.len(), e.len());
    assert_eq!(layout.nodes.len(), n.len());
    assert!(layout.metrics.return_links >= 3);
    let map = layout
        .nodes
        .iter()
        .map(|n| (n.id.as_str(), n))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(map["A"].component, map["B"].component);
    assert_eq!(map["B"].component, map["C"].component);
    assert_ne!(map["C"].component, map["D"].component);
    for e in layout.links.iter().filter(|e| e.source == e.target) {
        assert!(matches!(e.route, CircularLinkRoute::Return { .. }));
        assert_eq!(e.centerline.len(), 6);
        assert!(e.centerline[1][0] > e.source_port[0]);
        assert!(e.centerline[4][0] < e.target_port[0]);
    }
    assert_ports(&layout);
    layout.scene.validate().unwrap();
    assert!(
        render::to_svg(&layout.scene)
            .unwrap()
            .contains("circular-link:2:aa")
    );
    assert!(
        render::to_canvas_script(&layout.scene, "cycle")
            .unwrap()
            .contains("ctx.lineTo(")
    );
    assert!(
        layout
            .scene
            .marks
            .iter()
            .all(|m| m.geometry.commands().len() <= 256)
    );
}
fn segment_hits_rect(a: [f64; 2], b: [f64; 2], r: ClipRect, half: f64) -> bool {
    let low_x = a[0].min(b[0]) - half;
    let high_x = a[0].max(b[0]) + half;
    let low_y = a[1].min(b[1]) - half;
    let high_y = a[1].max(b[1]) + half;
    low_x < r.x + r.width - 1e-8
        && high_x > r.x + 1e-8
        && low_y < r.y + r.height - 1e-8
        && high_y > r.y + 1e-8
}
#[test]
fn return_lanes_and_vertical_strips_have_clearance_and_do_not_cross_node_bodies() {
    let mut o = options();
    o.route_padding = 4.;
    let n = nodes(&["A", "B", "C", "D"]);
    let e = edges(&[
        ("ab", "A", "B", 8.),
        ("bc", "B", "C", 8.),
        ("cd", "C", "D", 8.),
        ("da", "D", "A", 5.),
        ("ca", "C", "A", 2.),
        ("bb", "B", "B", 1.),
        ("dc", "D", "C", 1.),
    ]);
    let c = ChartConfig {
        width: 1200.,
        height: 700.,
        ..Default::default()
    };
    let layout = layout_sankey_circular(&c, &n, &e, &o).unwrap();
    let returns = layout
        .links
        .iter()
        .filter(|e| matches!(e.route, CircularLinkRoute::Return { .. }) && e.width > 0.)
        .collect::<Vec<_>>();
    for (i, a) in returns.iter().enumerate() {
        let CircularLinkRoute::Return { side, center_y, .. } = a.route else {
            panic!()
        };
        for b in &returns[i + 1..] {
            if let CircularLinkRoute::Return {
                side: other,
                center_y: y,
                ..
            } = b.route
                && side == other
            {
                assert!((y - center_y).abs() + 1e-8 >= (a.width + b.width) / 2. + o.route_padding);
            }
        }
        for pair in a.centerline[1..5].windows(2) {
            for node in &layout.nodes {
                assert!(
                    !segment_hits_rect(pair[0], pair[1], node.rect, a.width / 2.),
                    "return {} intersects node {}",
                    a.id,
                    node.id
                );
            }
        }
        for m in layout
            .scene
            .marks
            .iter()
            .filter(|m| m.id.ends_with(&format!(":{}", a.id)))
        {
            for command in m.geometry.commands() {
                if let Cmd::Move(p) | Cmd::Line(p) = command {
                    assert!(p.x >= c.margin.left - 1e-8 && p.x <= c.width - c.margin.right + 1e-8);
                    assert!(p.y >= c.margin.top - 1e-8 && p.y <= c.height - c.margin.bottom + 1e-8);
                }
            }
        }
    }
    assert_eq!(layout.metrics.return_lane_collisions, 0);
    assert_ports(&layout);
}
#[test]
fn permutation_keeps_metadata_ids_palette_and_routing_bit_for_bit() {
    let mut n = nodes(&["A", "B", "C", "D"]);
    let mut e = edges(&[
        ("ab", "A", "B", 8.),
        ("bc", "B", "C", 6.),
        ("ca", "C", "A", 4.),
        ("aa", "A", "A", 2.),
        ("cd", "C", "D", 4.),
    ]);
    let first = layout_sankey_circular(&ChartConfig::default(), &n, &e, &options()).unwrap();
    n.reverse();
    e.rotate_left(2);
    e.reverse();
    assert_eq!(
        first,
        layout_sankey_circular(&ChartConfig::default(), &n, &e, &options()).unwrap()
    );
}
#[test]
fn all_alignment_policies_keep_forward_edges_monotonic_and_disconnected_nodes() {
    let n = nodes(&[
        "Supply", "A", "B", "C", "Out", "Late", "Early", "Short", "ShortOut", "isolate",
    ]);
    let e = edges(&[
        ("sa", "Supply", "A", 80.),
        ("ab", "A", "B", 60.),
        ("bc", "B", "C", 60.),
        ("co", "C", "Out", 90.),
        ("lc", "Late", "C", 30.),
        ("ae", "A", "Early", 20.),
        ("ss", "Supply", "Short", 20.),
        ("so", "Short", "ShortOut", 20.),
    ]);
    let mut signatures = vec![];
    for alignment in [
        SankeyAlignment::Left,
        SankeyAlignment::Right,
        SankeyAlignment::Center,
        SankeyAlignment::Justify,
    ] {
        let layout = layout_sankey_circular(
            &ChartConfig::default(),
            &n,
            &e,
            &CircularSankeyOptions {
                alignment,
                ..options()
            },
        )
        .unwrap();
        assert_eq!(layout.nodes.len(), n.len());
        assert_eq!(layout.metrics.return_links, 0);
        let cols = layout
            .nodes
            .iter()
            .map(|n| (n.id.as_str(), n.column))
            .collect::<BTreeMap<_, _>>();
        for edge in &layout.links {
            assert!(cols[edge.source.as_str()] < cols[edge.target.as_str()]);
        }
        signatures.push([cols["Late"], cols["Early"], cols["Short"]]);
        assert_ports(&layout);
    }
    signatures.sort();
    signatures.dedup();
    assert_eq!(signatures.len(), 4);
}
#[test]
fn sweeps_reduce_known_crossing_and_never_worsen_other_graphs() {
    let n = nodes(&["A", "B", "C", "D"]);
    let e = edges(&[("ad", "A", "D", 3.), ("bc", "B", "C", 3.)]);
    let improved = layout_sankey_circular(&ChartConfig::default(), &n, &e, &options()).unwrap();
    assert_eq!(improved.metrics.forward_crossings_before, 1);
    assert_eq!(improved.metrics.forward_crossings_after, 0);
    for seed in 0..12 {
        let n = (0..8)
            .map(|i| FlowNode::new(format!("n{i}")))
            .collect::<Vec<_>>();
        let e = (0..12)
            .map(|i| {
                CircularFlowLink::new(
                    format!("e{i}"),
                    format!("n{}", (i + seed) % 8),
                    format!("n{}", (i * 3 + seed + 1) % 8),
                    (i % 3 + 1) as f64,
                )
            })
            .collect::<Vec<_>>();
        let c = ChartConfig {
            width: 1600.,
            height: 900.,
            ..Default::default()
        };
        let layout = layout_sankey_circular(&c, &n, &e, &options()).unwrap();
        assert!(layout.metrics.forward_crossings_after <= layout.metrics.forward_crossings_before);
        assert_ports(&layout);
    }
}
#[test]
fn zero_flows_empty_graph_and_extreme_magnitudes_have_explicit_semantics() {
    let c = ChartConfig::default();
    let empty = layout_sankey_circular(&c, &[], &[], &options()).unwrap();
    assert!(empty.nodes.is_empty());
    let n = nodes(&["A", "B"]);
    let e = edges(&[
        ("ab", "A", "B", 0.),
        ("ba", "B", "A", 0.),
        ("aa", "A", "A", 0.),
    ]);
    let zero = layout_sankey_circular(&c, &n, &e, &options()).unwrap();
    assert_eq!(zero.links.len(), 3);
    assert!(zero.links.iter().all(|e| e.width == 0.));
    assert!(zero.nodes.iter().all(|n| n.rect.height == 0.));
    zero.scene.validate().unwrap();
    let e = edges(&[
        ("ab", "A", "B", f64::MAX / 8.),
        ("ba", "B", "A", f64::MAX / 16.),
    ]);
    assert_ports(&layout_sankey_circular(&c, &n, &e, &options()).unwrap());
    let tiny = edges(&[
        ("ab", "A", "B", f64::from_bits(1)),
        ("ba", "B", "A", f64::from_bits(1)),
    ]);
    let tiny = layout_sankey_circular(&c, &n, &tiny, &options()).unwrap();
    assert!(
        tiny.links
            .iter()
            .all(|e| e.width.is_finite() && e.width > 0.)
    );
    assert_eq!(tiny.metrics.pixels_per_unit, None);
    assert_eq!(tiny.metrics.flow_normalization, f64::from_bits(1));
}
#[test]
fn invalid_graphs_and_spacing_budgets_reject_without_mutating_inputs() {
    let n = nodes(&["A", "B"]);
    let base = edges(&[("ab", "A", "B", 1.), ("ba", "B", "A", 1.)]);
    for e in [
        edges(&[("bad", "A", "missing", 1.)]),
        edges(&[("bad", "A", "B", -1.)]),
        edges(&[("bad", "A", "B", f64::NAN)]),
        edges(&[("same", "A", "B", 1.), ("same", "B", "A", 1.)]),
    ] {
        assert!(layout_sankey_circular(&ChartConfig::default(), &n, &e, &options()).is_err());
    }
    let before = (n.clone(), base.clone());
    let tiny = ChartConfig {
        width: 100.,
        height: 100.,
        margin: Margin {
            left: 0.,
            right: 0.,
            top: 0.,
            bottom: 0.,
        },
        ..Default::default()
    };
    assert!(
        layout_sankey_circular(
            &tiny,
            &n,
            &base,
            &CircularSankeyOptions {
                route_padding: 80.,
                ..options()
            }
        )
        .is_err()
    );
    assert_eq!(before, (n.clone(), base));
    let too_many = (0..=MAX_CIRCULAR_LINKS)
        .map(|i| CircularFlowLink::new(format!("e{i}"), "A", "A", 0.))
        .collect::<Vec<_>>();
    assert!(layout_sankey_circular(&ChartConfig::default(), &n, &too_many, &options()).is_err());
}

#[test]
fn balanced_return_lanes_reduce_one_sided_band_load_and_large_ring_is_bounded() {
    let c = ChartConfig {
        width: 1800.,
        height: 1000.,
        ..Default::default()
    };
    let n = nodes(&["A"]);
    let e = (0..6)
        .map(|i| CircularFlowLink::new(format!("self{i}"), "A", "A", 1.))
        .collect::<Vec<_>>();
    let balanced = layout_sankey_circular(&c, &n, &e, &options()).unwrap();
    let (top, bottom) = balanced
        .links
        .iter()
        .fold((0, 0), |(a, b), e| match e.route {
            CircularLinkRoute::Return {
                side: ReturnSide::Top,
                ..
            } => (a + 1, b),
            CircularLinkRoute::Return {
                side: ReturnSide::Bottom,
                ..
            } => (a, b + 1),
            _ => panic!(),
        });
    assert_eq!((top, bottom), (3, 3));
    let top = layout_sankey_circular(
        &c,
        &n,
        &e,
        &CircularSankeyOptions {
            return_side: CircularReturnSide::Top,
            ..options()
        },
    )
    .unwrap();
    assert!(top.links.iter().all(|e| matches!(
        e.route,
        CircularLinkRoute::Return {
            side: ReturnSide::Top,
            ..
        }
    )));
    let n = (0..MAX_CIRCULAR_NODES)
        .map(|i| FlowNode::new(format!("n{i:03}")))
        .collect::<Vec<_>>();
    let e = (0..MAX_CIRCULAR_NODES)
        .map(|i| {
            CircularFlowLink::new(
                format!("e{i:03}"),
                format!("n{i:03}"),
                format!("n{:03}", (i + 1) % MAX_CIRCULAR_NODES),
                1.,
            )
        })
        .collect::<Vec<_>>();
    let large = ChartConfig {
        width: 16384.,
        height: 16384.,
        ..Default::default()
    };
    let result = layout_sankey_circular(&large, &n, &e, &options()).unwrap();
    assert_eq!(result.nodes.len(), 256);
    assert_eq!(result.links.len(), 256);
    assert_eq!(result.metrics.components, 1);
    assert!(result.metrics.return_links >= 1);
    result.scene.validate().unwrap();
}

#[cfg(feature = "raster")]
#[test]
fn cyclic_ribbons_replay_as_real_cpu_canvas_paint_with_opacity() {
    use kartis::render::raster::{FontRegistry, RasterOptions, render_raster};
    let n = nodes(&["A", "B", "C"]);
    let e = edges(&[
        ("ab", "A", "B", 8.),
        ("bc", "B", "C", 8.),
        ("ca", "C", "A", 4.),
        ("aa", "A", "A", 2.),
    ]);
    let c = ChartConfig::default();
    let layout = layout_sankey_circular(&c, &n, &e, &options()).unwrap();
    let image = render_raster(
        &layout.scene,
        &FontRegistry::new(),
        &RasterOptions::default(),
    )
    .unwrap();
    let ink = image
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|p| p[0] < 240 || p[1] < 240 || p[2] < 240)
        .count();
    assert!(
        ink > 10_000,
        "the graph must replay actual closed ribbon interiors"
    );
    let back = layout
        .links
        .iter()
        .find(|e| matches!(e.route, CircularLinkRoute::Return { .. }) && e.width > 4.)
        .unwrap();
    let a = back.centerline[2];
    let b = back.centerline[3];
    let (x, y) = (((a[0] + b[0]) / 2.).floor() as usize, a[1].floor() as usize);
    let offset = (y * image.width as usize + x) * 4;
    assert_ne!(
        &image.rgba[offset..offset + 3],
        &[255, 255, 255],
        "return lane must visibly retain its flow"
    );
}
