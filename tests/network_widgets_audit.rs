//! Independent node-role/source qualification; physics algorithms are not Nivo oracles.
use std::sync::Arc;

use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 360.,
        height: 240.,
        margin: Margin {
            left: 31.,
            right: 25.,
            top: 23.,
            bottom: 27.,
        },
        ..Default::default()
    }
}
fn nodes() -> Vec<NetworkNode> {
    vec![
        NetworkNode {
            id: "a".into(),
            label: "A".into(),
            radius: 3.,
        },
        NetworkNode {
            id: "empty".into(),
            label: String::new(),
            radius: 4.,
        },
        NetworkNode {
            id: "noink".into(),
            label: "No ink".into(),
            radius: 5.,
        },
        NetworkNode {
            id: "zero".into(),
            label: "Zero visual".into(),
            radius: 6.,
        },
    ]
}
fn make(raw: &[NetworkNode]) -> ForceSimulation {
    let mut s = ForceSimulation::new(
        raw,
        &[],
        ForceOptions {
            repulsion: 0.,
            center_strength: 0.,
            collision_strength: 0.,
            link_strength: 0.,
            labels: true,
            ..Default::default()
        },
    )
    .unwrap();
    for (i, n) in raw.iter().enumerate() {
        s.set_position(&n.id, [[-40., -10.], [-0., 0.], [40., 20.], [70., -20.]][i])
            .unwrap();
    }
    s
}
fn simulation() -> ForceSimulation {
    make(&nodes())
}
fn glyphs() -> NetworkGlyphOptions {
    let mut o = NetworkGlyphOptions::default();
    for (id, size, fill) in [
        ("a", 128., Color(10, 80, 150, 255)),
        ("empty", 64., Color(20, 120, 80, 255)),
        ("noink", 64., Color(30, 40, 50, 0)),
        ("zero", 0., Color(100, 60, 90, 255)),
    ] {
        o.nodes.insert(
            id.into(),
            NetworkNodeGlyph {
                glyph: GlyphOptions {
                    shape: GlyphShape::Filled(if id == "zero" {
                        FilledGlyph::Circle
                    } else {
                        FilledGlyph::Triangle
                    }),
                    size,
                    rotation: 0.,
                },
                fill: Some(fill),
                stroke: None,
            },
        );
    }
    o
}
fn placement() -> PlotWidgetPlacement {
    PlotWidgetPlacement::AtDatum {
        offset: [-6., -4.],
        measured: TextSize {
            width: 12.,
            height: 8.,
        },
    }
}

#[test]
fn node_roles_preserve_full_force_snapshot_and_do_not_invent_empty_or_noink_controls() {
    let s = simulation();
    let l = network_glyph_layout(&config(), &s, &glyphs()).unwrap();
    let scene = l.scene().clone();
    let mut calls = vec![];
    let p = network_node_widgets(&l, &s, 8, |c| {
        assert!(std::ptr::eq(c.node, &s.nodes()[c.source_index]));
        assert!(std::ptr::eq(c.layout, &l));
        assert!(std::ptr::eq(c.simulation, &s));
        assert_eq!(c.site.node_id, c.node.id);
        assert_eq!(c.site.physics_radius, c.node.radius);
        assert_eq!(c.site.world_position, c.node.position);
        assert_eq!(c.mark.id, c.site.mark_id);
        calls.push(c.source_index);
        Ok(Some(placement()))
    })
    .unwrap();
    assert_eq!(calls, [0, 1, 2, 3]);
    assert_eq!(p.data.len(), 4);
    let data = p.data.values().collect::<Vec<_>>();
    assert!(
        data.windows(2)
            .all(|p| Arc::ptr_eq(&p[0].source, &p[1].source))
    );
    assert_eq!(data[0].source.as_slice(), s.nodes());
    for slot in &p.layout.slots {
        let d = &p.data[&slot.mark_id];
        let PlotWidgetPlacement::Rect(rect) = slot.placement else {
            panic!("AtDatum was not normalized to actual origin")
        };
        assert_eq!([rect.x + 6., rect.y + 4.], d.site.position);
    }
    let frame = network_widget_frame(&l).unwrap();
    assert_eq!(frame.scene, scene);
    assert_eq!(frame.scene.labels.len(), 4);
    assert!(frame.scene.labels.iter().any(|label| label.text.is_empty()));
    let binding = frame.interaction.unwrap();
    assert_eq!(binding.sites, l.interaction_binding().sites);
    assert!(
        !binding
            .sites
            .iter()
            .any(|site| site.mark_id == "force-node:empty" || site.mark_id == "force-node:noink")
    );
    assert_eq!(binding.picking_profile, PickingProfile::PaintedGeometry);
    assert!(binding.sites.iter().all(|site| site.value.is_none()));
    let a = p.data["force-node:a"].site.clone();
    assert_ne!(a.bounds.width, 2. * a.physics_radius);
    assert_ne!(a.bounds.y + a.bounds.height / 2., a.position[1]);
    assert_eq!(network_widget_frame(&l).unwrap().scene, scene);
}

#[test]
fn all_unrelated_or_late_simulation_conflicts_reject_before_any_callback() {
    type Mutation = (&'static str, fn(&mut ForceSimulation));
    let cases: &[Mutation] = &[
        ("late position", |s| {
            s.set_position("zero", [90., -20.]).unwrap()
        }),
        ("source world sign", |s| {
            s.set_position("empty", [0., 0.]).unwrap()
        }),
        ("late label", |s| {
            let mut n = nodes();
            n[3].label = "invented".into();
            *s = make(&n);
        }),
        ("late radius", |s| {
            let mut n = nodes();
            n[3].radius = 7.;
            *s = make(&n);
        }),
        ("late id", |s| {
            let mut n = nodes();
            n[3].id = "invented".into();
            *s = make(&n);
        }),
        ("source order", |s| {
            let mut n = nodes();
            n.swap(0, 1);
            *s = make(&n);
        }),
        ("source count", |s| {
            let mut n = nodes();
            n.pop();
            *s = make(&n);
        }),
    ];
    let original = simulation();
    let l = network_glyph_layout(&config(), &original, &glyphs()).unwrap();
    for (name, mutate) in cases {
        let mut bad = original.clone();
        mutate(&mut bad);
        let mut calls = 0;
        assert!(
            network_node_widgets(&l, &bad, 8, |_| {
                calls += 1;
                Ok(None)
            })
            .is_err(),
            "accepted {name}"
        );
        assert_eq!(calls, 0, "callback before complete preflight {name}");
    }
}

#[test]
fn metadata_only_velocity_fixed_and_tick_snapshots_do_not_require_fake_geometry_changes() {
    let base = simulation();
    let l = network_glyph_layout(&config(), &base, &glyphs()).unwrap();
    let packet = |s: &ForceSimulation| {
        network_node_widgets(&l, s, 0, |c| Ok((c.node.id == "a").then(placement))).unwrap()
    };
    let p = packet(&base);
    let mut velocity = base.clone();
    velocity.set_velocity("a", [-0., 0.]).unwrap();
    let q = packet(&velocity);
    assert_ne!(p, q);
    assert_eq!(
        l.scene(),
        network_glyph_layout(&config(), &velocity, &glyphs())
            .unwrap()
            .scene()
    );
    assert_eq!(
        q.data["force-node:a"].source[0].velocity[0].to_bits(),
        (-0_f64).to_bits()
    );
    let mut fixed = base.clone();
    fixed.set_fixed("empty", Some([-0., 0.])).unwrap();
    let r = packet(&fixed);
    assert_ne!(p, r);
    assert_eq!(
        r.data["force-node:a"].source[1].fixed.unwrap()[0].to_bits(),
        (-0_f64).to_bits()
    );
    assert_eq!(
        l.scene(),
        network_glyph_layout(&config(), &fixed, &glyphs())
            .unwrap()
            .scene()
    );
    assert_eq!(p, packet(&base));
    let mut tick = base.clone();
    tick.step(1).unwrap();
    let next = network_glyph_layout(&config(), &tick, &glyphs()).unwrap();
    assert_eq!(next.scene(), l.scene());
    let t =
        network_node_widgets(&next, &tick, 0, |c| Ok((c.node.id == "a").then(placement))).unwrap();
    assert_ne!(p, t);
    assert_eq!(t.data["force-node:a"].ticks, 1);
}

#[test]
fn empty_label_path_role_uses_glyph_origin_without_adding_business_binding() {
    let s = simulation();
    let l = network_glyph_layout(&config(), &s, &glyphs()).unwrap();
    let site = l
        .sites()
        .iter()
        .find(|site| site.node_id == "empty")
        .unwrap();
    assert!(matches!(
        l.scene()
            .marks
            .iter()
            .find(|m| m.id == site.mark_id)
            .unwrap()
            .geometry,
        Geometry::Path(_)
    ));
    assert_ne!(
        [
            site.bounds.x + site.bounds.width / 2.,
            site.bounds.y + site.bounds.height / 2.
        ],
        site.position
    );
    let p =
        network_node_widgets(&l, &s, 1, |c| Ok((c.node.id == "empty").then(placement))).unwrap();
    let slot = &p.layout.slots[0];
    let PlotWidgetPlacement::Rect(rect) = slot.placement else {
        panic!("origin placement must be explicit")
    };
    assert_eq!(
        [rect.x + rect.width / 2., rect.y + rect.height / 2.],
        site.position
    );
    assert!(p.data[&slot.mark_id].source[1].label.is_empty());
    assert!(
        !network_widget_frame(&l)
            .unwrap()
            .interaction
            .unwrap()
            .sites
            .iter()
            .any(|datum| datum.mark_id == site.mark_id)
    );
}

#[test]
fn budget_skip_error_and_empty_source_do_not_publish_partial_node_packets() {
    let n = (0..129)
        .map(|i| NetworkNode {
            id: format!("n{i:03}"),
            label: String::new(),
            radius: 1.,
        })
        .collect::<Vec<_>>();
    let s = ForceSimulation::new(
        &n,
        &[],
        ForceOptions {
            labels: false,
            ..Default::default()
        },
    )
    .unwrap();
    let l = network_glyph_layout(&config(), &s, &Default::default()).unwrap();
    let mut count = 0;
    assert!(
        network_node_widgets(&l, &s, 0, |_| {
            count += 1;
            Ok(Some(placement()))
        })
        .is_err()
    );
    assert_eq!(count, 129);
    assert_eq!(
        network_node_widgets(&l, &s, 0, |c| Ok(c
            .source_index
            .is_multiple_of(2)
            .then(placement)))
        .unwrap()
        .data
        .len(),
        65
    );
    let mut count = 0;
    assert!(
        network_node_widgets(&l, &s, 0, |_| {
            count += 1;
            if count == 3 {
                Err(ChartError("stop nodes".into()))
            } else {
                Ok(Some(placement()))
            }
        })
        .is_err()
    );
    assert_eq!(count, 3);
    let s = ForceSimulation::new(&[], &[], Default::default()).unwrap();
    let l = network_glyph_layout(&config(), &s, &Default::default()).unwrap();
    let p = network_node_widgets(&l, &s, 0, |_| panic!("empty source has no callback")).unwrap();
    assert!(p.data.is_empty());
    assert!(p.layout.slots.is_empty());
    network_widget_frame(&l).unwrap();
}
