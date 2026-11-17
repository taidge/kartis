//! Independent original Nivo function probes plus actual Rust factory consumers.
//! The frozen producer executes upstream bodies on a real Canvas2D. Force
//! algorithms and React lifecycles are explicitly outside this equivalence.
#![cfg(feature = "geojson")]

use std::collections::BTreeMap;

use glory::render::{GraphicPoint, PathCommand};
use kartis::surface::{Picking, SurfaceInput, SurfaceModel, SurfaceOptions};
use kartis::*;
use serde_json::Value;

fn fixture() -> Value {
    serde_json::from_str(include_str!("fixtures/node-link-custom/values.json")).unwrap()
}
fn array(v: &Value) -> &[Value] {
    v.as_array().unwrap()
}
fn number(v: &Value) -> f64 {
    v.as_f64().unwrap()
}
fn text(v: &Value) -> &str {
    v.as_str().unwrap()
}
fn named<'a>(values: &'a Value, name: &str) -> &'a Value {
    array(values).iter().find(|v| v["name"] == name).unwrap()
}
fn point(v: &Value) -> [f64; 2] {
    [number(&v[0]), number(&v[1])]
}
fn hex(v: &Value) -> Color {
    let s = text(v).strip_prefix('#').unwrap();
    let b = |i| u8::from_str_radix(&s[i..i + 2], 16).unwrap();
    Color(b(0), b(2), b(4), if s.len() == 8 { b(6) } else { 255 })
}
fn near(a: f64, b: f64) {
    assert!((a - b).abs() <= 1e-9, "{a} != {b}");
}
fn config() -> ChartConfig {
    ChartConfig {
        width: 220.,
        height: 160.,
        margin: Margin {
            left: 0.,
            right: 0.,
            top: 0.,
            bottom: 0.,
        },
        ..Default::default()
    }
}
fn simulation(f: &Value) -> ForceSimulation {
    let input = &f["networkCases"][0];
    // Physics radii and the visual node size are separate in both protocols.
    // Beta's zero computed visual size does not require an invalid zero Rust
    // physics radius; these probes compare link primitives, not force layout.
    let nodes = array(&input["nodes"])
        .iter()
        .map(|n| NetworkNode {
            id: text(&n["id"]).into(),
            label: text(&n["data"]["label"]).into(),
            radius: if n["id"] == "beta" {
                4.
            } else {
                number(&n["data"]["radius"])
            },
        })
        .collect::<Vec<_>>();
    let links = array(&input["links"])
        .iter()
        .map(|l| {
            NetworkLink::new(
                text(&l["source"]["id"]),
                text(&l["target"]["id"]),
                number(&l["data"]["value"]),
            )
        })
        .collect::<Vec<_>>();
    let mut s = ForceSimulation::new(
        &nodes,
        &links,
        ForceOptions {
            labels: false,
            ..Default::default()
        },
    )
    .unwrap();
    for n in array(&input["nodes"]) {
        s.set_position(
            text(&n["id"]),
            [number(&n["x"]) - 110., number(&n["y"]) - 80.],
        )
        .unwrap();
        s.set_velocity(text(&n["id"]), [number(&n["vx"]), number(&n["vy"])])
            .unwrap();
    }
    s
}
fn swarm_data(f: &Value) -> Vec<SwarmPoint> {
    array(&f["preparedSwarm"])
        .iter()
        .map(|p| SwarmPoint {
            id: text(&p["id"]).into(),
            group: text(&p["group"]).into(),
            value: number(&p["value"]),
            size: Some(number(&p["size"])),
            color: Some(hex(&p["data"]["color"])),
            label: None,
        })
        .collect()
}
fn swarm_options(orientation: SwarmOrientation, opacity: f64) -> SwarmLayoutOptions {
    SwarmLayoutOptions {
        orientation,
        groups: Some(vec![SwarmGroup::new("left"), SwarmGroup::new("right")]),
        group_gap: 8.,
        spacing: 0.,
        value_domain: Some([-5., 10.]),
        value_scale: ContinuousScaleOptions::default(),
        value_fit: SwarmValueFit::Clip,
        value_axis: None,
        group_axis: None,
        node: SwarmNodeStyle {
            opacity,
            ..Default::default()
        },
        ..Default::default()
    }
}
fn path_points(g: &Geometry) -> Vec<[f64; 2]> {
    let Geometry::Path(path) = g else {
        panic!("expected line")
    };
    path.iter()
        .map(|c| match c {
            PathCommand::Move(p) | PathCommand::Line(p) => [p.x, p.y],
            _ => panic!("default primitive is not linear"),
        })
        .collect()
}

#[test]
fn original_real_canvas_probes_retain_zero_ink_calls_order_and_applied_paint() {
    let f = fixture();
    assert_eq!(array(&f["native"]["network"]).len(), 10);
    assert_eq!(array(&f["native"]["swarm"]).len(), 6);
    for (kind, zero, full, count) in [
        ("network", "links-butt-alpha0", "links-round-alpha1", 4),
        ("swarm", "nodes-alpha0", "nodes-alpha1", 3),
    ] {
        let zero = named(&f["native"][kind], zero);
        let full = named(&f["native"][kind], full);
        assert_eq!(array(&zero["calls"]).len(), count);
        assert_eq!(array(&full["calls"]).len(), count);
        assert!(
            array(&zero["witnesses"])
                .iter()
                .all(|p| array(&p["rgba"]).iter().all(|c| number(c) == 0.))
        );
        let failing = named(
            &f["native"][kind],
            if kind == "network" {
                "links-first-failure-stops"
            } else {
                "nodes-first-failure-stops"
            },
        );
        assert_eq!(array(&failing["calls"]).len(), 2);
        assert_eq!(failing["thrown"], "explicit custom callback failure");
    }
    let links = named(&f["native"]["network"], "links-repeat-layer-and-order");
    assert_eq!(
        array(&links["calls"])
            .iter()
            .map(|l| l["index"].as_u64().unwrap())
            .collect::<Vec<_>>(),
        [3, 0, 2, 1, 3, 0, 2, 1]
    );
    let circles = named(&f["native"]["swarm"], "nodes-reordered-repeat-layer");
    assert_eq!(
        array(&circles["calls"])
            .iter()
            .map(|l| l["index"].as_u64().unwrap())
            .collect::<Vec<_>>(),
        [2, 0, 1, 2, 0, 1]
    );
    let native = named(&f["native"]["network"], "links-round-alpha1");
    let requested_zero = array(&native["events"])
        .iter()
        .find(|e| e[0] == "set" && e[1] == "lineWidth" && e[2] == 0)
        .unwrap();
    assert_eq!(
        number(&requested_zero[3]),
        2.,
        "real Canvas ignores lineWidth=0, retaining previous 2px; this is not a Rust default promise"
    );
    assert!(
        array(&native["events"])
            .iter()
            .filter(|e| e[0] == "stroke")
            .all(|e| e[1]["lineCap"] == "round" && e[1]["globalAlpha"] == 1)
    );
    let enters = array(&native["animation"]);
    for a in enters {
        assert_eq!(a["enter"]["x1"], a["enter"]["x2"]);
        assert_eq!(a["enter"]["opacity"], 0);
        assert_eq!(a["update"]["opacity"], 1);
    }
    let swarm = named(&f["native"]["swarm"], "nodes-alpha1");
    for a in array(&swarm["animation"]) {
        let index = a["index"].as_u64().unwrap() as usize;
        near(
            number(&a["update"]["radius"]),
            number(&f["preparedSwarm"][index]["size"]) / 2.,
        );
        assert_eq!(a["enter"]["radius"], 0);
        assert_eq!(a["leave"]["opacity"], 0);
    }
    assert_eq!(
        f["computedLinks"][0]["id"], f["computedLinks"][1]["id"],
        "retain actual upstream parallel-id collision instead of rewriting the oracle"
    );
}

#[test]
fn network_borrowed_context_and_real_default_path_follow_native_computed_links() {
    let f = fixture();
    let s = simulation(&f);
    let before = s.clone();
    let original = network_glyph_layout(&config(), &s, &Default::default()).unwrap();
    let native = named(&f["native"]["network"], "links-round-alpha1");
    let mut calls = vec![];
    let layout =
        network_glyph_layout_with_link_factory(&config(), &s, &Default::default(), &mut |ctx| {
            let reference = &native["calls"][ctx.link_index];
            calls.push(ctx.link_index);
            assert_eq!(ctx.source.id, text(&reference["source"]));
            assert_eq!(ctx.target.id, text(&reference["target"]));
            assert_eq!(ctx.edge.value, number(&reference["data"]["value"]));
            assert!(std::ptr::eq(ctx.edge, &s.edges()[ctx.link_index]));
            assert!(std::ptr::eq(ctx.source, &s.nodes()[ctx.edge.source]));
            assert!(std::ptr::eq(ctx.target, &s.nodes()[ctx.edge.target]));
            assert_eq!(ctx.source_position, point(&reference["sourcePosition"]));
            assert_eq!(ctx.target_position, point(&reference["targetPosition"]));
            assert_eq!(
                path_points(ctx.geometry),
                [ctx.source_position, ctx.target_position]
            );
            assert_eq!(ctx.world_offset, [110., 80.]);
            for i in 0..2 {
                near(
                    ctx.position[i],
                    (ctx.source_position[i] + ctx.target_position[i]) / 2.,
                );
            }
            Ok(ctx.inherit(ctx.geometry.clone()))
        })
        .unwrap();
    assert_eq!(calls, [0, 1, 2, 3]);
    assert_eq!(layout.scene(), original.scene());
    assert_eq!(layout.node_layout().sites(), original.sites());
    assert_eq!(layout.interaction_binding(), original.interaction_binding());
    assert_eq!(s.nodes(), before.nodes());
    assert_eq!(s.edges(), before.edges());
    assert_eq!(s.total_ticks(), before.total_ticks());
    for (meta, reference) in layout.links().iter().zip(array(&native["calls"])) {
        assert_eq!(meta.source_id, text(&reference["source"]));
        assert_eq!(meta.target_id, text(&reference["target"]));
        assert_eq!(meta.value, number(&reference["data"]["value"]));
        assert_eq!(meta.source_position, point(&reference["sourcePosition"]));
        assert_eq!(meta.target_position, point(&reference["targetPosition"]));
    }
    // A second actual frame must read current simulation endpoints and velocities.
    let mut moved = s.clone();
    moved.set_position("alpha", [-70., -35.]).unwrap();
    moved.set_velocity("alpha", [8., 9.]).unwrap();
    let mut count = 0;
    network_glyph_layout_with_link_factory(&config(), &moved, &Default::default(), &mut |ctx| {
        count += 1;
        if ctx.source.id == "alpha" {
            assert_eq!(ctx.source.position, [-70., -35.]);
            assert_eq!(ctx.source.velocity, [8., 9.]);
            assert_eq!(ctx.source_position, [40., 45.]);
        }
        Ok(ctx.inherit(ctx.geometry.clone()))
    })
    .unwrap();
    assert_eq!(count, 4);
}

#[test]
fn swarm_factory_matches_original_circle_units_and_group_centers_without_claiming_force_positions()
{
    let f = fixture();
    let data = swarm_data(&f);
    let snapshot = data.clone();
    let native = named(&f["native"]["swarm"], "nodes-alpha1");
    let arcs = array(&native["events"])
        .iter()
        .filter(|e| e[0] == "arc")
        .collect::<Vec<_>>();
    assert_eq!(arcs.len(), 3);
    for orientation in [SwarmOrientation::Horizontal, SwarmOrientation::Vertical] {
        for opacity in [0., 1.] {
            let options = swarm_options(orientation, opacity);
            let old = swarm_layout(&config(), &data, &options).unwrap();
            let mut order = vec![];
            let ordinal = array(&f["ordinalCases"])
                .iter()
                .find(|c| {
                    c["input"]["axis"]
                        == if orientation == SwarmOrientation::Horizontal {
                            "x"
                        } else {
                            "y"
                        }
                        && array(&c["domain"]).len() == 2
                })
                .unwrap();
            let next = swarm_layout_with_node_factory(&config(), &data, &options, |ctx| {
                order.push(ctx.source_index);
                assert!(std::ptr::eq(ctx.datum, &data[ctx.source_index]));
                assert_eq!(
                    ctx.datum.id,
                    text(&f["preparedSwarm"][ctx.source_index]["id"])
                );
                assert_eq!(
                    ctx.datum.value,
                    number(&f["preparedSwarm"][ctx.source_index]["value"])
                );
                assert_eq!(
                    ctx.size,
                    number(&f["preparedSwarm"][ctx.source_index]["size"])
                );
                let Geometry::Circle { x, y, radius } = ctx.geometry else {
                    panic!("default circle expected")
                };
                assert_eq!([*x, *y], ctx.position);
                near(*radius, number(&arcs[ctx.source_index][3]));
                near(
                    ctx.group.center,
                    number(&ordinal["mapped"][ctx.group_index]),
                );
                assert_eq!(ctx.opacity, opacity);
                assert_eq!(
                    ctx.fill,
                    Some(hex(&f["preparedSwarm"][ctx.source_index]["data"]["color"]))
                );
                assert_eq!(ctx.site.source_index, ctx.source_index);
                assert_eq!(ctx.site.group_index, ctx.group_index);
                Ok(ctx.inherit(ctx.geometry.clone()))
            })
            .unwrap();
            assert_eq!(
                order,
                old.sites.iter().map(|s| s.source_index).collect::<Vec<_>>()
            );
            assert_eq!(
                order.len(),
                array(&native["calls"]).len(),
                "zero size, transparent and zero opacity retain the original call eligibility"
            );
            assert_eq!(next.scene, old.scene);
            assert_eq!(next.sites, old.sites);
            assert_eq!(next.metadata, old.metadata);
            assert_eq!(next.groups, old.groups);
            assert_eq!(next.anchors, old.anchors);
            assert_eq!(next.interaction_binding(), old.interaction_binding());
            assert_eq!(data, snapshot);
        }
    }
}

#[test]
fn source_identity_and_nearest_anchor_survive_custom_ink_and_failures_stop_both_factories() {
    let f = fixture();
    let data = swarm_data(&f);
    let options = swarm_options(SwarmOrientation::Horizontal, 1.);
    let old = swarm_layout(&config(), &data, &options).unwrap();
    let custom = swarm_layout_with_node_factory(&config(), &data, &options, |ctx| {
        let [x, y] = ctx.position;
        let mut ink = ctx.inherit(Geometry::Path(vec![
            PathCommand::Move(GraphicPoint::new(x + 12., y + 8.)),
            PathCommand::Line(GraphicPoint::new(x + 20., y + 8.)),
            PathCommand::Line(GraphicPoint::new(x + 20., y + 14.)),
            PathCommand::Line(GraphicPoint::new(x + 12., y + 14.)),
            PathCommand::Close,
        ]));
        ink.fill = Some(Color(180, 30, 90, 255));
        ink.stroke = None;
        Ok(ink)
    })
    .unwrap();
    for site in &custom.sites {
        let original = old
            .sites
            .iter()
            .find(|s| s.source_index == site.source_index)
            .unwrap();
        assert_eq!(site.position, original.position);
        assert_eq!(site.ink_radius, original.ink_radius);
        assert_eq!(site.raw_value, original.raw_value);
        for (point, picking, hit) in [
            (site.position, Picking::Nearest { distance: 1. }, true),
            (site.position, Picking::Geometry, false),
            (
                [site.position[0] + 16., site.position[1] + 11.],
                Picking::Geometry,
                true,
            ),
        ] {
            let mut host = SurfaceModel::new_with_interaction(
                custom.scene.clone(),
                SurfaceOptions {
                    plot: Some(custom.plot),
                    picking,
                    ..Default::default()
                },
                Some(custom.interaction_binding()),
            )
            .unwrap();
            host.apply(SurfaceInput::Move {
                id: 1,
                point: (point[0], point[1]),
            })
            .unwrap();
            assert_eq!(host.hovered().contains(&site.mark_id), hit);
        }
    }
    let expected = array(&named(&f["native"]["swarm"], "nodes-first-failure-stops")["calls"]).len();
    let mut calls = vec![];
    assert!(
        swarm_layout_with_node_factory(&config(), &data, &options, |ctx| {
            calls.push(ctx.source_index);
            if calls.len() == 2 {
                return Err(ChartError("independent swarm error".into()));
            }
            Ok(ctx.inherit(ctx.geometry.clone()))
        })
        .is_err()
    );
    assert_eq!(calls.len(), expected);
    assert_eq!(
        calls,
        old.sites
            .iter()
            .take(2)
            .map(|s| s.source_index)
            .collect::<Vec<_>>()
    );
    let sim = simulation(&f);
    let before = sim.clone();
    let mut links = vec![];
    assert!(
        network_glyph_layout_with_link_factory(&config(), &sim, &Default::default(), &mut |ctx| {
            links.push(ctx.link_index);
            if links.len() == 2 {
                return Err(ChartError("independent link error".into()));
            }
            Ok(ctx.inherit(ctx.geometry.clone()))
        })
        .is_err()
    );
    assert_eq!(links, [0, 1]);
    assert_eq!(sim.nodes(), before.nodes());
    assert_eq!(sim.edges(), before.edges());
    let ids = ["parallel-one", "parallel-two", "self", "reverse"].map(String::from);
    let baseline = network_glyph_relation_layout(
        &config(),
        &sim,
        &Default::default(),
        &ids,
        &Default::default(),
    )
    .unwrap();
    let relation = network_glyph_relation_layout_with_link_factory(
        &config(),
        &sim,
        &Default::default(),
        &ids,
        &Default::default(),
        &mut |ctx| {
            let [a, b] = ctx.source_position;
            let [c, d] = ctx.target_position;
            Ok(ctx.inherit(Geometry::Path(vec![
                PathCommand::Move(GraphicPoint::new(a, b)),
                PathCommand::Cubic {
                    first: GraphicPoint::new(a, b - 12.),
                    second: GraphicPoint::new(c, d - 12.),
                    end: GraphicPoint::new(c, d),
                },
            ])))
        },
    )
    .unwrap();
    assert_eq!(relation.graph(), baseline.graph());
    for id in ids {
        assert_eq!(relation.graph().edge(&id), baseline.graph().edge(&id));
    }
    let original_nodes = baseline
        .authored_scene()
        .marks
        .iter()
        .filter(|m| m.id.starts_with("force-node:"))
        .map(|m| (&m.id, &m.geometry))
        .collect::<BTreeMap<_, _>>();
    let nodes = relation
        .authored_scene()
        .marks
        .iter()
        .filter(|m| m.id.starts_with("force-node:"))
        .map(|m| (&m.id, &m.geometry))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(nodes, original_nodes);
}
