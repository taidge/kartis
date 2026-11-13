#[cfg(feature = "raster")]
use glory::render::PathCommand as Cmd;
use glory::render::{GraphicStroke, StrokeCap};
use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 640.,
        height: 420.,
        margin: Margin {
            left: 42.,
            right: 54.,
            top: 32.,
            bottom: 46.,
        },
        ..Default::default()
    }
}
fn data(cyclic: bool) -> (Vec<FlowNode>, Vec<SankeyStyleLink>) {
    let nodes = ["source", "middle", "sink", "isolate"]
        .map(FlowNode::new)
        .to_vec();
    let mut links = vec![
        SankeyStyleLink::new("first", "source", "middle", 8.),
        SankeyStyleLink::new("parallel", "source", "middle", 2.),
        SankeyStyleLink::new("last", "middle", "sink", 10.),
    ];
    if cyclic {
        links.push(SankeyStyleLink::new("return", "sink", "source", 3.));
        links.push(SankeyStyleLink::new("self", "middle", "middle", 1.));
    }
    (nodes, links)
}
fn solve(cyclic: bool, style: &SankeyStyleOptions) -> SankeyStyledLayout {
    let (n, e) = data(cyclic);
    if cyclic {
        circular_sankey_with_style(&config(), &n, &e, &CircularSankeyOptions::default(), style)
    } else {
        sankey_with_style(&config(), &n, &e, &SankeyLayoutOptions::default(), style)
    }
    .unwrap()
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-8, "{a} != {b}");
}
fn ports(s: &SankeyStyledLayout, vertical: bool) {
    for node in &s.nodes {
        close(node.value, node.inflow.max(node.outflow));
        for incoming in [false, true] {
            let mut spans = vec![];
            for link in s.links.iter().filter(|l| {
                if incoming {
                    l.target == node.id
                } else {
                    l.source == node.id
                }
            }) {
                let p = if incoming {
                    link.target_port
                } else {
                    link.source_port
                };
                let r = node.rect;
                let (flow, cross, base, breadth) = if vertical {
                    (p[1], p[0], r.x, r.width)
                } else {
                    (p[0], p[1], r.y, r.height)
                };
                close(
                    flow,
                    if vertical {
                        r.y + if incoming { 0. } else { r.height }
                    } else {
                        r.x + if incoming { 0. } else { r.width }
                    },
                );
                assert!(
                    cross - link.width / 2. >= base - 1e-8
                        && cross + link.width / 2. <= base + breadth + 1e-8
                );
                spans.push((cross - link.width / 2., cross + link.width / 2., link.width));
            }
            spans.sort_by(|a, b| a.0.total_cmp(&b.0));
            for w in spans.windows(2) {
                assert!(w[0].1 <= w[1].0 + 1e-8, "overlapping ports on {}", node.id);
            }
            if node.value > 0. && (if incoming { node.inflow } else { node.outflow }) == node.value
            {
                close(
                    spans.iter().map(|p| p.2).sum(),
                    if vertical {
                        node.rect.width
                    } else {
                        node.rect.height
                    },
                );
            }
        }
    }
    for a in &s.links {
        for b in &s.links {
            close(a.width / a.value, b.width / b.value);
        }
    }
}
#[test]
fn both_real_orientations_conserve_widths_and_ports_in_dags_and_cycles() {
    for cyclic in [false, true] {
        for vertical in [false, true] {
            let s = solve(
                cyclic,
                &SankeyStyleOptions {
                    orientation: if vertical {
                        SankeyOrientation::Vertical
                    } else {
                        SankeyOrientation::Horizontal
                    },
                    ..Default::default()
                },
            );
            s.scene.validate().unwrap();
            ports(&s, vertical);
            for n in &s.nodes {
                close(
                    if vertical {
                        n.rect.height
                    } else {
                        n.rect.width
                    },
                    16.,
                );
            }
            assert!(s.scene.labels.iter().all(|l| l.rotation == 0.));
            if cyclic {
                assert_eq!(
                    s.circular_metrics.as_ref().unwrap().return_lane_collisions,
                    0
                );
                assert!(
                    s.links
                        .iter()
                        .any(|l| matches!(l.route, SankeyStyledRoute::Return { .. }))
                );
                for l in &s.links {
                    if let SankeyStyledRoute::Return { side, .. } = l.route {
                        assert_eq!(
                            matches!(side, SankeyReturnSide::Left | SankeyReturnSide::Right),
                            vertical
                        );
                    }
                }
            }
        }
    }
}
#[test]
fn vertical_solver_uses_swapped_flow_space_before_transposing_geometry() {
    let c = config();
    let (nodes, edges) = data(false);
    let v = sankey_with_style(
        &c,
        &nodes,
        &edges,
        &SankeyLayoutOptions::default(),
        &SankeyStyleOptions {
            orientation: SankeyOrientation::Vertical,
            ..Default::default()
        },
    )
    .unwrap();
    let canonical = ChartConfig {
        width: c.height,
        height: c.width,
        margin: Margin {
            left: c.margin.top,
            right: c.margin.bottom,
            top: c.margin.left,
            bottom: c.margin.right,
        },
        ..c.clone()
    };
    let h = sankey_with_style(
        &canonical,
        &nodes,
        &edges,
        &SankeyLayoutOptions::default(),
        &SankeyStyleOptions::default(),
    )
    .unwrap();
    for (a, b) in v.nodes.iter().zip(&h.nodes) {
        assert_eq!(a.id, b.id);
        close(a.rect.x, b.rect.y);
        close(a.rect.y, b.rect.x);
        close(a.rect.width, b.rect.height);
        close(a.rect.height, b.rect.width);
    }
    assert!(v.nodes.iter().any(|n| n.rect.width > 100.));
}
#[test]
fn explicit_ids_keep_geometry_palette_and_styles_when_input_is_reordered() {
    let (mut n, mut e) = data(true);
    n[0].id = "source:node:link".into();
    for l in &mut e {
        if l.source == "source" {
            l.source = n[0].id.clone();
        }
        if l.target == "source" {
            l.target = n[0].id.clone();
        }
    }
    e[0].id = "node:4:sink".into();
    let options = SankeyStyleOptions::default();
    let a = circular_sankey_with_style(
        &config(),
        &n,
        &e,
        &CircularSankeyOptions::default(),
        &options,
    )
    .unwrap();
    n.reverse();
    e.reverse();
    let b = circular_sankey_with_style(
        &config(),
        &n,
        &e,
        &CircularSankeyOptions::default(),
        &options,
    )
    .unwrap();
    assert_eq!(a, b);
    let (mut n, mut e) = data(false);
    let a =
        sankey_with_style(&config(), &n, &e, &SankeyLayoutOptions::default(), &options).unwrap();
    n.reverse();
    e.reverse();
    assert_eq!(
        a,
        sankey_with_style(&config(), &n, &e, &SankeyLayoutOptions::default(), &options).unwrap()
    );
}
#[test]
fn colors_gradients_borders_opacity_and_rounding_are_actual_scene_properties() {
    let red = Color(240, 20, 30, 255);
    let blue = Color(20, 40, 230, 255);
    let mut style = SankeyStyleOptions::default();
    style.node.corner_radius = 12.;
    style.node.border = Some(GraphicStroke {
        color: Color(0, 0, 0, 255),
        width: 2.,
        cap: StrokeCap::Round,
    });
    style.nodes.insert(
        "source".into(),
        SankeyNodeStyle {
            color: Some(red),
            opacity: 0.6,
            ..style.node.clone()
        },
    );
    style.nodes.insert(
        "middle".into(),
        SankeyNodeStyle {
            color: Some(blue),
            ..style.node.clone()
        },
    );
    style.link.color = SankeyLinkColor::Gradient;
    style.link.opacity = Some(0.8);
    style.links.insert(
        "parallel".into(),
        SankeyLinkStyle {
            color: SankeyLinkColor::Target,
            opacity: Some(0.3),
        },
    );
    for cyclic in [false, true] {
        let s = solve(cyclic, &style);
        let first = &s.links.iter().find(|l| l.id == "first").unwrap();
        let mark = s
            .scene
            .marks
            .iter()
            .find(|m| m.id == first.mark_id)
            .unwrap();
        assert_eq!(
            mark.fill_paint,
            Some(FillPaint::Linear {
                start: first.source_port,
                end: first.target_port,
                stops: vec![GradientStop::new(0., red), GradientStop::new(1., blue)]
            })
        );
        assert_eq!(mark.opacity, 0.8);
        let parallel = s.links.iter().find(|l| l.id == "parallel").unwrap();
        let mark = s
            .scene
            .marks
            .iter()
            .find(|m| m.id == parallel.mark_id)
            .unwrap();
        assert_eq!(mark.fill, Some(blue));
        assert_eq!(mark.opacity, 0.3);
        assert_eq!(mark.fill_paint, None);
        let node = s.nodes.iter().find(|n| n.id == "source").unwrap();
        let mark = s.scene.marks.iter().find(|m| m.id == node.mark_id).unwrap();
        assert!(matches!(mark.geometry, Geometry::Path(_)));
        assert_eq!(mark.opacity, 0.6);
        assert_eq!(mark.stroke, style.node.border);
        assert_eq!(mark.fill, Some(red));
    }
}
#[test]
fn curvature_changes_only_forward_ribbons_and_leaves_return_lanes_and_ports_exact() {
    for cyclic in [false, true] {
        let a = solve(
            cyclic,
            &SankeyStyleOptions {
                forward_curvature: 0.,
                ..Default::default()
            },
        );
        let b = solve(
            cyclic,
            &SankeyStyleOptions {
                forward_curvature: 0.9,
                ..Default::default()
            },
        );
        assert_eq!(a.nodes, b.nodes);
        for (x, y) in a.links.iter().zip(&b.links) {
            assert_eq!(x.source_port, y.source_port);
            assert_eq!(x.target_port, y.target_port);
            assert_eq!(x.width, y.width);
            assert_eq!(x.route, y.route);
            let m = |s: &SankeyStyledLayout| {
                s.scene
                    .marks
                    .iter()
                    .find(|m| m.id == x.mark_id)
                    .unwrap()
                    .geometry
                    .clone()
            };
            if x.route == SankeyStyledRoute::Forward {
                assert_ne!(m(&a), m(&b));
            } else {
                assert_eq!(m(&a), m(&b));
            }
        }
    }
}
#[test]
fn owned_labels_have_configured_positions_and_independent_text_orientation() {
    for vertical in [false, true] {
        for inside in [false, true] {
            let options = SankeyStyleOptions {
                orientation: if vertical {
                    SankeyOrientation::Vertical
                } else {
                    SankeyOrientation::Horizontal
                },
                label: SankeyLabelStyle {
                    position: if inside {
                        SankeyLabelPosition::Inside
                    } else {
                        SankeyLabelPosition::Outside
                    },
                    color: Some(Color(90, 10, 100, 255)),
                    size: Some(19.),
                    padding: 7.,
                    orientation: if vertical {
                        SankeyLabelOrientation::Vertical
                    } else {
                        SankeyLabelOrientation::Horizontal
                    },
                    offset: [2., 3.],
                },
                ..Default::default()
            };
            let s = solve(false, &options);
            let n = s.nodes.iter().find(|n| n.id == "source").unwrap();
            let i = *s
                .scene
                .label_series
                .iter()
                .find(|(_, id)| id.as_str() == "source")
                .unwrap()
                .0;
            let label = &s.scene.labels[i];
            assert_eq!(label.size, 19.);
            assert_eq!(label.color, Color(90, 10, 100, 255));
            assert_eq!(label.rotation, if vertical { -90. } else { 0. });
            if vertical {
                close(
                    label.y,
                    if inside {
                        n.rect.y + n.rect.height + 7. + 3.
                    } else {
                        n.rect.y - 7. + 3.
                    },
                );
            } else {
                close(
                    label.x,
                    if inside {
                        n.rect.x + n.rect.width + 7. + 2.
                    } else {
                        n.rect.x - 7. + 2.
                    },
                );
            }
            assert!(matches!(label.anchor, TextAnchor::Start | TextAnchor::End));
        }
    }
}
#[test]
fn hidden_nodes_filter_incident_ink_and_owned_labels_without_moving_any_flow() {
    for cyclic in [false, true] {
        let a = solve(cyclic, &SankeyStyleOptions::default());
        let b = solve(
            cyclic,
            &SankeyStyleOptions {
                hidden: ["middle".into()].into(),
                ..Default::default()
            },
        );
        assert_eq!(a.scene.legend, b.scene.legend);
        assert!(!b.scene.label_series.values().any(|s| s == "middle"));
        assert!(!b.scene.marks.iter().any(|m| m.series == "middle"));
        for (x, y) in a.links.iter().zip(&b.links) {
            assert_eq!(x.source_port, y.source_port);
            assert_eq!(x.target_port, y.target_port);
            assert_eq!(x.width, y.width);
            assert_eq!(y.visible, y.source != "middle" && y.target != "middle");
            assert_eq!(b.scene.marks.iter().any(|m| m.id == y.mark_id), y.visible);
        }
        for (x, y) in a.nodes.iter().zip(&b.nodes) {
            assert_eq!(x.rect, y.rect);
        }
        let filtered = a.scene.without_series(&["source".into()].into());
        assert!(!filtered.marks.iter().any(|m| m.series == "source"));
        assert!(!filtered.label_series.values().any(|s| s == "source"));
    }
}
#[test]
fn invalid_styles_topologies_and_values_reject_without_mutating_inputs() {
    let (nodes, edges) = data(false);
    let snapshot = (nodes.clone(), edges.clone());
    let check = |o: &SankeyStyleOptions, e: &[SankeyStyleLink]| {
        sankey_with_style(&config(), &nodes, e, &SankeyLayoutOptions::default(), o).is_err()
    };
    for n in [f64::NAN, f64::INFINITY, -1., 1.1] {
        assert!(check(
            &SankeyStyleOptions {
                forward_curvature: n,
                ..Default::default()
            },
            &edges
        ));
    }
    for n in [f64::NAN, -1., 1.1] {
        assert!(check(
            &SankeyStyleOptions {
                node: SankeyNodeStyle {
                    opacity: n,
                    ..Default::default()
                },
                ..Default::default()
            },
            &edges
        ));
    }
    assert!(check(
        &SankeyStyleOptions {
            hidden: ["unknown".into()].into(),
            ..Default::default()
        },
        &edges
    ));
    let mut bad = edges.clone();
    bad.push(bad[0].clone());
    assert!(check(&SankeyStyleOptions::default(), &bad));
    bad = edges.clone();
    bad[0].value = f64::MAX;
    bad[1].value = f64::MAX;
    assert!(check(&SankeyStyleOptions::default(), &bad));
    bad = edges.clone();
    bad.push(SankeyStyleLink::new("cycle", "sink", "source", 1.));
    assert!(check(&SankeyStyleOptions::default(), &bad));
    let mut labels = SankeyStyleOptions::default();
    labels.label.offset = [1_000_000., 1_000_000.];
    assert!(check(&labels, &edges));
    assert_eq!((nodes, edges), snapshot);
}
#[test]
fn subnormal_and_large_finite_flows_have_finite_true_widths() {
    for value in [f64::from_bits(1), 1e-300, 1e300] {
        for cyclic in [false, true] {
            let nodes = [FlowNode::new("a"), FlowNode::new("b")];
            let mut edges = vec![SankeyStyleLink::new("ab", "a", "b", value)];
            if cyclic {
                edges.push(SankeyStyleLink::new("ba", "b", "a", value));
            }
            let s = if cyclic {
                circular_sankey_with_style(
                    &config(),
                    &nodes,
                    &edges,
                    &CircularSankeyOptions::default(),
                    &SankeyStyleOptions::default(),
                )
            } else {
                sankey_with_style(
                    &config(),
                    &nodes,
                    &edges,
                    &SankeyLayoutOptions::default(),
                    &SankeyStyleOptions::default(),
                )
            }
            .unwrap();
            s.scene.validate().unwrap();
            assert!(s.links.iter().all(|l| l.width.is_finite() && l.width > 0.));
        }
    }
}

#[test]
fn empty_zero_and_large_legendless_layouts_have_explicit_bounded_profiles() {
    let empty = sankey_with_style(
        &config(),
        &[],
        &[],
        &SankeyLayoutOptions::default(),
        &SankeyStyleOptions::default(),
    )
    .unwrap();
    assert!(empty.scene.marks.is_empty());
    assert!(empty.nodes.is_empty());
    let nodes = [FlowNode::new("a"), FlowNode::new("b")];
    let edges = [
        SankeyStyleLink::new("a", "a", "b", 2.),
        SankeyStyleLink::new("b", "a", "b", 0.),
        SankeyStyleLink::new("c", "a", "b", 3.),
    ];
    let s = sankey_with_style(
        &config(),
        &nodes,
        &edges,
        &SankeyLayoutOptions::default(),
        &SankeyStyleOptions::default(),
    )
    .unwrap();
    let zero = &s.links[1];
    assert_eq!(zero.width, 0.);
    assert!(!zero.visible);
    assert!(!s.scene.marks.iter().any(|m| m.id == zero.mark_id));
    close(
        zero.source_port[1],
        s.links[0].source_port[1] + s.links[0].width / 2.,
    );
    close(
        zero.target_port[1],
        s.links[0].target_port[1] + s.links[0].width / 2.,
    );
    let many: Vec<_> = (0..257)
        .map(|i| FlowNode::new(format!("n{i:04}")))
        .collect();
    let layout = SankeyLayoutOptions {
        appearance: SankeyOptions {
            node_padding: 0.,
            labels: false,
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(
        sankey_with_style(
            &config(),
            &many,
            &[],
            &layout,
            &SankeyStyleOptions::default()
        )
        .is_err()
    );
    let accepted = sankey_with_style(
        &config(),
        &many,
        &[],
        &layout,
        &SankeyStyleOptions {
            legend: false,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(accepted.nodes.len(), 257);
    assert!(accepted.scene.legend.is_empty());
    let mut over = many;
    over.resize_with(2001, || FlowNode::new("over"));
    assert!(
        sankey_with_style(
            &config(),
            &over,
            &[],
            &layout,
            &SankeyStyleOptions {
                legend: false,
                ..Default::default()
            }
        )
        .is_err()
    );
}

#[cfg(feature = "raster")]
#[test]
fn cpu_pixels_render_real_linear_gradient_and_rounded_node_corner() {
    use kartis::render::raster::{FontRegistry, RasterOptions, render_raster};
    for vertical in [false, true] {
        for cyclic in [false, true] {
            let mut style = SankeyStyleOptions {
                orientation: if vertical {
                    SankeyOrientation::Vertical
                } else {
                    SankeyOrientation::Horizontal
                },
                ..Default::default()
            };
            style.node.corner_radius = 8.;
            style.node.color = Some(Color(10, 30, 240, 255));
            style.nodes.insert(
                "source".into(),
                SankeyNodeStyle {
                    color: Some(Color(240, 20, 10, 255)),
                    ..style.node.clone()
                },
            );
            style.link = SankeyLinkStyle {
                color: SankeyLinkColor::Gradient,
                opacity: Some(1.),
            };
            let mut s = solve(cyclic, &style);
            s.scene.labels.clear();
            s.scene.label_series.clear();
            let frame =
                render_raster(&s.scene, &FontRegistry::new(), &RasterOptions::default()).unwrap();
            let pixel = |x: f64, y: f64| {
                let i = (y.floor() as usize * frame.width as usize + x.floor() as usize) * 4;
                &frame.rgba[i..i + 4]
            };
            let node = s.nodes.iter().find(|n| n.id == "source").unwrap();
            let r = node.rect;
            // Return ribbons can legitimately pass behind a rounded corner.
            // Isolate the actual node paint to test its own corner coverage.
            let mut node_scene = s.scene.clone();
            node_scene.marks.retain(|m| m.id == node.mark_id);
            let rounded =
                render_raster(&node_scene, &FontRegistry::new(), &RasterOptions::default())
                    .unwrap();
            let corner = (r.y.ceil() as usize * rounded.width as usize + r.x.ceil() as usize) * 4;
            assert_eq!(
                &rounded.rgba[corner..corner + 4],
                &[255, 255, 255, 255],
                "round corner must remove actual fill"
            );
            assert_eq!(
                pixel(r.x + r.width / 2., r.y + r.height / 2.),
                &[240, 20, 10, 255]
            );
            let link = s.links.iter().find(|l| l.id == "first").unwrap();
            let Geometry::Path(cmds) = &link.centerline else {
                panic!()
            };
            let (
                Cmd::Move(a),
                Cmd::Cubic {
                    first: b,
                    second: d,
                    end: e,
                },
            ) = (&cmds[0], &cmds[1])
            else {
                panic!()
            };
            let p = |t: f64| {
                let q = 1. - t;
                [
                    q * q * q * a.x + 3. * q * q * t * b.x + 3. * q * t * t * d.x + t * t * t * e.x,
                    q * q * q * a.y + 3. * q * q * t * b.y + 3. * q * t * t * d.y + t * t * t * e.y,
                ]
            };
            let start = p(0.2);
            let end = p(0.8);
            let a = pixel(start[0], start[1]);
            let b = pixel(end[0], end[1]);
            assert!(
                a[0] > b[0] + 60 && b[2] > a[2] + 60,
                "not a source→target gradient: {a:?}, {b:?}, cyclic={cyclic}, vertical={vertical}"
            );
        }
    }
}
