use std::sync::Arc;

use glory::render::{GraphicPoint as P, PathCommand as C};
use kartis::*;
fn config() -> ChartConfig {
    ChartConfig {
        width: 420.,
        height: 280.,
        margin: Margin {
            left: 24.,
            right: 16.,
            top: 20.,
            bottom: 30.,
        },
        ..Default::default()
    }
}
fn simulation(values: [f64; 3], reversed: bool) -> ForceSimulation {
    let mut nodes = vec![
        NetworkNode::new("a"),
        NetworkNode::new("b"),
        NetworkNode::new("c"),
    ];
    nodes[1].label.clear();
    let mut links = vec![
        NetworkLink::new("a", "b", values[0]),
        NetworkLink::new("a", "b", values[1]),
        NetworkLink::new("c", "c", values[2]),
    ];
    if reversed {
        links.reverse();
    }
    let mut s = ForceSimulation::new(
        &nodes,
        &links,
        ForceOptions {
            labels: false,
            repulsion: 0.,
            center_strength: 0.,
            link_strength: 0.,
            collision_strength: 0.,
            ..Default::default()
        },
    )
    .unwrap();
    for (id, p) in [("a", [-40., -0.]), ("b", [60., 30.]), ("c", [20., -50.])] {
        s.set_position(id, p).unwrap();
    }
    s
}
fn layout(s: &ForceSimulation) -> NetworkLinkLayout {
    network_glyph_layout_with_link_factory(&config(), s, &Default::default(), &mut |c| {
        Ok(c.inherit(c.geometry.clone()))
    })
    .unwrap()
}
fn placement() -> PlotWidgetPlacement {
    PlotWidgetPlacement::AtDatum {
        offset: [-16., -10.],
        measured: TextSize {
            width: 32.,
            height: 20.,
        },
    }
}

#[test]
fn default_scene_and_canonical_binding_are_exact_and_raw_sources_shared() {
    let sim = simulation([-0., 4., 0.], false);
    let l = layout(&sim);
    let original = sim
        .scene_with_glyphs(&config(), &Default::default())
        .unwrap();
    assert_eq!(l.scene(), original.scene());
    assert_eq!(l.interaction_binding(), original.interaction_binding());
    let before = l.scene().clone();
    let mut calls = vec![];
    let packet = network_link_widgets(&l, &sim, 7, |c| {
        assert!(std::ptr::eq(c.edge, &sim.edges()[c.link_index]));
        assert!(std::ptr::eq(c.source, &sim.nodes()[c.edge.source]));
        assert!(std::ptr::eq(c.target, &sim.nodes()[c.edge.target]));
        assert_eq!(c.mark.id, c.site.mark_id);
        assert_eq!(c.position, c.site.position);
        calls.push((c.link_index, c.source.id.clone(), c.target.id.clone()));
        Ok(Some(placement()))
    })
    .unwrap();
    assert_eq!(
        calls,
        [
            (0, "a".into(), "b".into()),
            (1, "a".into(), "b".into()),
            (2, "c".into(), "c".into())
        ]
    );
    assert_eq!(l.scene(), &before);
    assert_eq!(packet.data.len(), 3);
    let first = packet.data.values().next().unwrap();
    for d in packet.data.values() {
        assert!(Arc::ptr_eq(&d.source, &first.source));
        assert!(Arc::ptr_eq(&d.edges, &first.edges));
        assert_eq!(
            d.edges[d.source_index].source,
            sim.edges()[d.source_index].source
        );
        assert_eq!(
            d.site.value.to_bits(),
            sim.edges()[d.source_index].value.to_bits()
        );
    }
    assert_eq!(first.edges[0].value.to_bits(), (-0_f64).to_bits());
    assert!(packet.layout.slots.iter().all(|s| s.revision == 7));
    assert_eq!(packet.layout.plot, None);
    let frame = network_link_widget_frame(&l).unwrap();
    assert_eq!(frame.scene, before);
    assert_eq!(frame.plot, None);
    let binding = frame.interaction.unwrap();
    assert_eq!(binding.sites, original.interaction_binding().sites);
    assert!(
        binding
            .sites
            .iter()
            .all(|s| s.mark_id.starts_with("force-node:"))
    );
    assert_eq!(binding.picking_profile, PickingProfile::PaintedGeometry);
    let mut detached = first.source.clone();
    Arc::make_mut(&mut detached)[0].label = "external".into();
    assert_eq!(first.source[0].label, "a");
}
#[test]
fn mixed_callback_order_one_node_snapshot_and_selective_roles() {
    let sim = simulation([1., 2., 0.], false);
    let l = layout(&sim);
    let mut order = vec![];
    let packet = network_widgets(&l, &sim, 9, |c| {
        order.push(match c {
            NetworkWidgetContext::Link(c) => c.mark.id.clone(),
            NetworkWidgetContext::Node(c) => c.mark.id.clone(),
        });
        Ok(Some(placement()))
    })
    .unwrap();
    assert_eq!(
        order,
        vec![
            "force-link:0",
            "force-link:1",
            "force-link:2",
            "force-node:a",
            "force-node:b",
            "force-node:c"
        ]
    );
    let sources: Vec<_> = packet
        .data
        .values()
        .map(|d| match d {
            NetworkWidgetData::Link(d) => &d.source,
            NetworkWidgetData::Node(d) => &d.source,
        })
        .collect();
    assert!(sources.iter().all(|s| Arc::ptr_eq(s, sources[0])));
    let selected = network_widgets(&l, &sim, 0, |c| {
        Ok(match c {
            NetworkWidgetContext::Link(c) if c.link_index == 1 => Some(placement()),
            NetworkWidgetContext::Node(c) if c.node.id == "b" => Some(placement()),
            _ => None,
        })
    })
    .unwrap();
    assert_eq!(
        selected
            .layout
            .slots
            .iter()
            .map(|s| s.mark_id.as_str())
            .collect::<Vec<_>>(),
        ["force-link:1", "force-node:b"]
    );
    assert_eq!(selected.data.len(), 2);
    assert_eq!(
        l.interaction_binding().sites.len(),
        2,
        "empty-label node role does not invent a canonical control"
    );
}
#[test]
fn opaque_or_no_ink_custom_paths_keep_original_midpoint_in_real_memory_chart() {
    use std::cell::RefCell;
    use std::rc::Rc;

    use glory::App;
    let sim = simulation([1., 2., 0.], false);
    for invisible in [false, true] {
        let l = network_glyph_layout_with_link_factory(
            &config(),
            &sim,
            &Default::default(),
            &mut |c| {
                let [x, y] = c.position;
                let mut ink = c.inherit(Geometry::Path(vec![
                    C::Move(P::new(x + 25., y - 3.)),
                    C::Line(P::new(x + 35., y - 3.)),
                    C::Line(P::new(x + 35., y + 3.)),
                    C::Close,
                ]));
                ink.fill = Some(Color(0, 100, 255, 255));
                ink.stroke = None;
                if invisible {
                    ink.opacity = 0.;
                    ink.fill = None;
                }
                Ok(ink)
            },
        )
        .unwrap();
        let packet = network_link_widgets(&l, &sim, 0, |_| Ok(Some(placement()))).unwrap();
        assert_eq!(packet.data.len(), 3);
        for slot in &packet.layout.slots {
            let d = &packet.data[&slot.mark_id];
            let PlotWidgetPlacement::Rect(r) = slot.placement else {
                panic!("real midpoint should be explicit")
            };
            assert_eq!(
                [r.x, r.y],
                [d.site.position[0] - 16., d.site.position[1] - 10.]
            );
            assert!(d.site.bounds.x > d.site.position[0]);
        }
        let seen = Rc::new(RefCell::new(vec![]));
        let out = seen.clone();
        let app = App::memory();
        app.mount(
            chart_layout(network_link_widget_frame(&l).unwrap())
                .legend(false)
                .plot_widgets_with_data(packet, move |c| {
                    out.borrow_mut().push(
                        c.frame
                            .with(|f| (f.frame.rect, f.data.site.position, f.frame.mark.clone())),
                    );
                    Ok(glory::ui::button().label("real link component"))
                }),
        )
        .unwrap();
        assert_eq!(seen.borrow().len(), 3);
        for (r, p, m) in seen.borrow().iter() {
            assert_eq!([r.x, r.y], [p[0] - 16., p[1] - 10.]);
            assert_eq!(m.opacity, if invisible { 0. } else { 1. });
        }
        app.clear().unwrap();
        assert_eq!(app.stats().nodes, 0);
    }
}
#[test]
fn complete_mismatched_frame_rejects_before_link_or_node_callbacks() {
    let sim = simulation([-0., 4., 0.], false);
    let l = layout(&sim);
    let mut candidates = vec![
        simulation([0., 4., 0.], false),
        simulation([-0., 4., 1.], false),
        simulation([-0., 4., 0.], true),
    ];
    let mut changed = sim.clone();
    changed.set_position("c", [21., -50.]).unwrap();
    candidates.push(changed);
    let mut changed = sim.clone();
    changed.set_position("a", [-40., 0.]).unwrap();
    candidates.push(changed);
    candidates.push(
        ForceSimulation::new(
            &[NetworkNode::new("a")],
            &[],
            ForceOptions {
                labels: false,
                ..Default::default()
            },
        )
        .unwrap(),
    );
    for changed in candidates {
        let mut calls = 0;
        assert!(
            network_widgets(&l, &changed, 0, |_| {
                calls += 1;
                Ok(None)
            })
            .is_err()
        );
        assert_eq!(calls, 0);
        let mut calls = 0;
        assert!(
            network_link_widgets(&l, &changed, 0, |_| {
                calls += 1;
                Ok(None)
            })
            .is_err()
        );
        assert_eq!(calls, 0);
    }
    // Velocities/fixedness are a current snapshot, not unretained original layout options.
    let mut current = sim.clone();
    current.set_velocity("c", [-0., 0.]).unwrap();
    current.set_fixed("c", Some([20., -50.])).unwrap();
    assert!(network_widgets(&l, &current, 0, |_| Ok(Some(placement()))).is_ok());
}
#[test]
fn exact_equality_observes_all_node_and_edge_siblings_and_detached_ownership() {
    let sim = simulation([-0., 4., 0.], false);
    let l = layout(&sim);
    let p = network_link_widgets(&l, &sim, 0, |c| Ok((c.link_index == 1).then(placement))).unwrap();
    let a = p.data.values().next().unwrap().clone();
    let mut b = a.clone();
    b.source = Arc::new((*a.source).clone());
    b.edges = Arc::new((*a.edges).clone());
    assert_eq!(a, b);
    Arc::make_mut(&mut b.edges)[0].value = 0.;
    assert_ne!(a, b);
    assert_eq!(a.edges[0].value.to_bits(), (-0_f64).to_bits());
    b = a.clone();
    Arc::make_mut(&mut b.source)[2].velocity[0] = -0.;
    assert_ne!(a, b);
    b = a.clone();
    Arc::make_mut(&mut b.source)[2].fixed = Some([20., -50.]);
    assert_ne!(a, b);
    b = a.clone();
    b.site.value = -0.;
    let mut c = b.clone();
    c.site.value = 0.;
    assert_ne!(b, c);
    b = a.clone();
    b.site.position[0] = -0.;
    let mut c = b.clone();
    c.site.position[0] = 0.;
    assert_eq!(b, c, "computed geometry keeps numeric equality");
    let mut ticked = sim.clone();
    ticked.tick().unwrap();
    let new = layout(&ticked);
    let data = network_link_widgets(
        &new,
        &ticked,
        0,
        |c| Ok((c.link_index == 1).then(placement)),
    )
    .unwrap();
    assert_ne!(p.data, data.data);
}
#[test]
fn edge_index_identity_reuses_original_index_after_reordering() {
    let a = simulation([1., 2., 3.], false);
    let b = simulation([1., 2., 3.], true);
    let la = layout(&a);
    let lb = layout(&b);
    let pa = network_link_widgets(&la, &a, 0, |_| Ok(Some(placement()))).unwrap();
    let pb = network_link_widgets(&lb, &b, 0, |_| Ok(Some(placement()))).unwrap();
    assert_eq!(
        pa.layout
            .slots
            .iter()
            .map(|s| &s.mark_id)
            .collect::<Vec<_>>(),
        pb.layout
            .slots
            .iter()
            .map(|s| &s.mark_id)
            .collect::<Vec<_>>()
    );
    assert_eq!(pa.data["force-link:0"].source_index, 0);
    assert_eq!(pb.data["force-link:0"].source_index, 0);
    assert_eq!(pa.data["force-link:0"].site.source_id, "a");
    assert_eq!(pb.data["force-link:0"].site.source_id, "c");
}
#[test]
fn empty_skip_error_and_shared_selected_role_budget() {
    let empty = ForceSimulation::new(
        &[],
        &[],
        ForceOptions {
            labels: false,
            ..Default::default()
        },
    )
    .unwrap();
    let l = layout(&empty);
    let mut calls = 0;
    assert!(
        network_widgets(&l, &empty, 0, |_| {
            calls += 1;
            Ok(None)
        })
        .unwrap()
        .data
        .is_empty()
    );
    assert_eq!(calls, 0);
    assert_eq!(network_link_widget_frame(&l).unwrap().scene.width, 420.);
    let sim = simulation([1., 2., 0.], false);
    let l = layout(&sim);
    let before = l.scene().clone();
    let mut calls = 0;
    assert!(
        network_widgets(&l, &sim, 0, |_| {
            calls += 1;
            if calls == 5 {
                Err(ChartError("late callback".into()))
            } else {
                Ok(Some(placement()))
            }
        })
        .is_err()
    );
    assert_eq!(calls, 5);
    assert_eq!(l.scene(), &before);
    let nodes = [NetworkNode::new("a"), NetworkNode::new("b")];
    let links: Vec<_> = (0..127).map(|_| NetworkLink::new("a", "b", 0.)).collect();
    let many = ForceSimulation::new(
        &nodes,
        &links,
        ForceOptions {
            labels: false,
            ..Default::default()
        },
    )
    .unwrap();
    let l = layout(&many);
    let mut calls = 0;
    assert!(
        network_widgets(&l, &many, 0, |_| {
            calls += 1;
            Ok(Some(placement()))
        })
        .is_err()
    );
    assert_eq!(calls, 129);
    let p = network_widgets(&l, &many, 0, |c| {
        Ok(match c {
            NetworkWidgetContext::Node(c) if c.node.id == "b" => None,
            _ => Some(placement()),
        })
    })
    .unwrap();
    assert_eq!(p.data.len(), 128);
    assert_eq!(
        network_link_widgets(&l, &many, 0, |_| Ok(Some(placement())))
            .unwrap()
            .data
            .len(),
        127
    );
}

#[test]
fn real_retained_link_child_refreshes_endpoints_and_revision_rebuilds() {
    use glory::render::{Event, EventKind, Props};
    use glory::{App, Widget};
    #[derive(Clone)]
    struct Source {
        frame: SceneInteraction,
        composition: PlotWidgetsComposition<NetworkLinkWidgetData>,
    }
    let candidate = |reversed: bool, revision: u64| {
        let sim = simulation([1., 2., 3.], reversed);
        let l = layout(&sim);
        let widgets = network_link_widgets(&l, &sim, revision, |c| {
            Ok((c.link_index == 0).then(placement))
        })
        .unwrap();
        Source {
            frame: network_link_widget_frame(&l).unwrap(),
            composition: PlotWidgetsComposition {
                replace_ink: widgets.data.keys().cloned().collect(),
                widgets,
            },
        }
    };
    let app = App::memory();
    let source = app.scope().cage(candidate(false, 0));
    let frame = app.scope().bond(move || source.with(|s| s.frame.clone()));
    let packet = app
        .scope()
        .bond(move || source.with(|s| s.composition.clone()));
    app.mount(
        chart_layout(frame)
            .legend(false)
            .plot_widgets_composed(packet, |context| {
                let frame = context.frame;
                Ok(glory::ui::widget(move |scope| {
                    let count = scope.cage(0usize);
                    let focus = scope.cage(false);
                    glory::ui::button()
                        .class("retained-link-role")
                        .label(scope.bond(move || {
                            frame.with(|f| {
                                format!(
                                    "{}->{} value={} count={} focus={}",
                                    f.data.site.source_id,
                                    f.data.site.target_id,
                                    f.data.edges[f.data.source_index].value,
                                    count.get(),
                                    focus.get()
                                )
                            })
                        }))
                        .on(EventKind::Activate, move |_| count.revise(|n| *n += 1))
                        .on(EventKind::FocusChanged, move |e| {
                            if let Event::FocusChanged(v) = e {
                                focus.revise(|f| *f = v);
                            }
                        })
                        .show_in(scope)
                }))
            }),
    )
    .unwrap();
    let find = || {
        let tree = app.tree();
        let mut queue = tree.roots().to_vec();
        let mut found = vec![];
        while let Some(id) = queue.pop() {
            let n = tree.node(id).unwrap();
            if n.descriptor
                .view()
                .classes
                .iter()
                .any(|c| c == "retained-link-role")
            {
                found.push(id);
            }
            queue.extend(n.children.iter().copied());
        }
        assert_eq!(found.len(), 1);
        found[0]
    };
    let label = |id| {
        let tree = app.tree();
        let Props::Button(p) = tree.node(id).unwrap().descriptor.view().props else {
            panic!("actual Button required")
        };
        p.label.clone()
    };
    let id = find();
    app.dispatch(id, Event::Activate).unwrap();
    app.dispatch(id, Event::FocusChanged(true)).unwrap();
    assert_eq!(label(id), "a->b value=1 count=1 focus=true");
    source.revise(|s| *s = candidate(true, 0));
    app.flush().unwrap();
    assert_eq!(find(), id);
    assert_eq!(
        label(id),
        "c->c value=3 count=1 focus=true",
        "edge index identity intentionally reuses retained child for changed source/target"
    );
    source.revise(|s| *s = candidate(true, 1));
    app.flush().unwrap();
    assert_ne!(find(), id);
    assert_eq!(label(find()), "c->c value=3 count=0 focus=false");
    app.clear().unwrap();
    assert_eq!(app.stats().nodes, 0);
}
