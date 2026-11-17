use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use glory::App;
use glory::render::{GraphicPoint as P, PathCommand as C};
use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 400.,
        height: 280.,
        margin: Margin {
            left: 30.,
            right: 20.,
            top: 20.,
            bottom: 30.,
        },
        ..Default::default()
    }
}
fn options() -> SwarmLayoutOptions {
    SwarmLayoutOptions {
        value_axis: None,
        group_axis: None,
        value_domain: Some([-2., 10.]),
        value_scale: ContinuousScaleOptions {
            nice: ScaleNice::Off,
            ..Default::default()
        },
        groups: Some(vec![
            SwarmGroup::new("north"),
            SwarmGroup::new("south"),
            SwarmGroup::new("ghost"),
        ]),
        hidden: ["ghost".into()].into(),
        ..Default::default()
    }
}
fn source() -> Vec<SwarmPoint> {
    let mut zero = SwarmPoint::new("zero", "north", -0.);
    zero.size = Some(-0.);
    zero.color = Some(Color(0, 100, 255, 255));
    zero.label = Some("zero original".into());
    vec![
        SwarmPoint::new("high", "north", 6.),
        zero,
        SwarmPoint::new("low", "north", 2.),
        SwarmPoint::new("other", "south", 3.),
        SwarmPoint::new("hidden", "ghost", 9.),
    ]
}
fn placement(_: &SwarmNodeWidgetContext<'_>) -> Result<Option<PlotWidgetPlacement>> {
    Ok(Some(PlotWidgetPlacement::AtDatum {
        offset: [-8., -4.],
        measured: TextSize {
            width: 16.,
            height: 8.,
        },
    }))
}

#[test]
fn original_source_is_borrowed_then_shared_once_in_packed_order() {
    let mut raw = source();
    let layout = swarm_layout(&config(), &raw, &options()).unwrap();
    let scene = layout.scene.clone();
    let svg = render::to_svg(&scene).unwrap();
    let mut order = vec![];
    let packet = swarm_node_widgets(&layout, &raw, 7, |c| {
        assert!(std::ptr::eq(c.source.as_ptr(), raw.as_ptr()));
        assert!(std::ptr::eq(c.datum, &raw[c.source_index]));
        assert!(std::ptr::eq(c.site, &layout.sites[c.site_index]));
        assert_eq!(c.mark.id, c.site.mark_id);
        assert_eq!(c.group.id, c.datum.group);
        order.push(c.source_index);
        placement(c)
    })
    .unwrap();
    assert_eq!(order, [0, 2, 1, 3]);
    assert_eq!(layout.scene, scene);
    assert_eq!(render::to_svg(&layout.scene).unwrap(), svg);
    assert_eq!(packet.layout.plot, Some(layout.plot));
    assert!(packet.layout.slots.iter().all(|s| s.revision == 7));
    let rows: Vec<_> = packet.data.values().collect();
    assert_eq!(rows.len(), 4);
    assert!(rows.iter().all(|d| Arc::ptr_eq(&d.source, &rows[0].source)));
    assert_eq!(rows[0].source.len(), 5);
    assert_eq!(rows[0].source[1].value.to_bits(), (-0_f64).to_bits());
    assert_eq!(
        rows[0].source[1].size.unwrap().to_bits(),
        (-0_f64).to_bits()
    );
    raw[1].label = Some("external change".into());
    assert_eq!(rows[0].source[1].label.as_deref(), Some("zero original"));
    let mut detached = rows[0].source.clone();
    Arc::make_mut(&mut detached)[1].value = 99.;
    assert_eq!(rows[0].source[1].value, 0.);
    let frame = swarm_widget_frame(&layout).unwrap();
    assert_eq!(frame.scene, scene);
    assert_eq!(frame.plot, Some(layout.plot));
    assert_eq!(
        frame.interaction.unwrap().picking_profile,
        PickingProfile::PaintedGeometry
    );
}

#[test]
fn directions_reverse_round_and_actual_zero_or_transparent_roles() {
    for orientation in [SwarmOrientation::Vertical, SwarmOrientation::Horizontal] {
        for reverse in [false, true] {
            for round in [false, true] {
                for value_fit in [SwarmValueFit::Inset, SwarmValueFit::Clip] {
                    let raw = source();
                    let mut o = options();
                    o.orientation = orientation;
                    o.value_fit = value_fit;
                    o.value_scale.reverse = reverse;
                    o.value_scale.round = round;
                    o.node.opacity = 0.;
                    let mut layout = swarm_layout(&config(), &raw, &o).unwrap();
                    for s in &layout.sites {
                        let m = &mut layout.scene.marks[s.mark_index];
                        m.fill = None;
                        m.stroke = None;
                    }
                    assert_eq!(
                        swarm_node_widgets(&layout, &raw, 0, placement)
                            .unwrap()
                            .data
                            .len(),
                        4
                    );
                    assert!(layout.sites.iter().all(|s| s.ink_radius == 0.));
                    assert!(swarm_widget_frame(&layout).is_ok());
                }
            }
        }
    }
    let mut raw = source();
    raw.push(SwarmPoint::new("culled", "south", 100.));
    let mut o = options();
    o.value_fit = SwarmValueFit::Clip;
    let layout = swarm_layout(&config(), &raw, &o).unwrap();
    assert_eq!(layout.metadata[5].site_index, None);
    let mut calls = vec![];
    swarm_node_widgets(&layout, &raw, 0, |c| {
        calls.push(c.source_index);
        placement(c)
    })
    .unwrap();
    assert_eq!(calls, [0, 2, 1, 3]);
}

#[test]
fn complete_frame_preflight_rejects_late_forgery_before_any_callback() {
    let raw = source();
    let base = swarm_layout(&config(), &raw, &options()).unwrap();
    type Mutation = (&'static str, fn(&mut SwarmLayout));
    let changes: Vec<Mutation> = vec![
        ("meta identity", |l| l.metadata[3].node_id = "wrong".into()),
        ("meta source index", |l| l.metadata[3].source_index = 0),
        ("meta group", |l| l.metadata[3].group_index = usize::MAX),
        ("meta raw", |l| l.metadata[3].raw_value = 4.),
        ("meta size", |l| l.metadata[1].size = 1.),
        ("meta hidden", |l| l.metadata[4].hidden = false),
        ("meta duplicate site", |l| {
            l.metadata[3].site_index = Some(0)
        }),
        ("meta absent visible", |l| {
            l.metadata[3].site_index = None;
            l.sites.pop();
            l.scene.marks.pop();
        }),
        ("site raw bits", |l| l.sites[2].raw_value = 0.),
        ("site source", |l| l.sites[3].source_index = usize::MAX),
        ("site group", |l| l.sites[3].group_id = "north".into()),
        ("site mark index", |l| l.sites[3].mark_index = 0),
        ("site position", |l| l.sites[3].position[1] += 1.),
        ("site radius", |l| l.sites[3].ink_radius = f64::NAN),
        ("site order", |l| l.sites.swap(0, 1)),
        ("site orphan", |l| l.sites.push(l.sites[0].clone())),
        ("group center", |l| l.groups[1].center += 1.),
        ("group duplicate", |l| l.groups[1].id = "north".into()),
        ("group bounds", |l| l.groups[1].band[1] = f64::NAN),
        ("mark owner", |l| {
            let i = l.sites[3].mark_index;
            l.scene.marks[i].series = "north".into();
        }),
        ("mark label", |l| {
            let i = l.sites[3].mark_index;
            l.scene.marks[i].label = "wrong".into();
        }),
        ("mark scalar", |l| {
            let i = l.sites[3].mark_index;
            l.scene.marks[i].value = 7.;
        }),
        ("mark center", |l| {
            let i = l.sites[3].mark_index;
            if let Geometry::Circle { x, .. } = &mut l.scene.marks[i].geometry {
                *x += 1.;
            }
        }),
        ("mark clip", |l| {
            let i = l.sites[3].mark_index;
            l.scene.marks[i].clip = None;
        }),
        ("mark orphan", |l| {
            let mut m = l.scene.marks[l.sites[3].mark_index].clone();
            m.id = "swarm-node:orphan".into();
            l.scene.marks.push(m);
        }),
        ("scale mapping", |l| {
            l.value_scale = ResolvedContinuousScale::new(
                [-2., 20.],
                l.value_scale.range(),
                &ContinuousScaleOptions {
                    nice: ScaleNice::Off,
                    ..Default::default()
                },
            )
            .unwrap()
        }),
    ];
    for (name, change) in changes {
        let mut l = base.clone();
        change(&mut l);
        let mut calls = 0;
        assert!(
            swarm_node_widgets(&l, &raw, 0, |_| {
                calls += 1;
                Ok(None)
            })
            .is_err(),
            "{name}"
        );
        assert_eq!(calls, 0, "{name}");
    }
    for k in 0..5 {
        let mut changed = raw.clone();
        match k {
            0 => changed[4].id = "wrong".into(),
            1 => changed[4].group = "north".into(),
            2 => changed[1].value = 0.,
            3 => changed[1].size = Some(0.),
            _ => changed[4].value = f64::NAN,
        };
        let mut calls = 0;
        assert!(
            swarm_node_widgets(&base, &changed, 0, |_| {
                calls += 1;
                Ok(None)
            })
            .is_err()
        );
        assert_eq!(calls, 0);
    }
}

#[test]
fn offset_path_uses_packed_anchor_and_real_owned_data_in_memory_chart() {
    let raw = source();
    let layout = swarm_layout_with_node_factory(&config(), &raw, &options(), |c| {
        let [x, y] = c.position;
        Ok(c.inherit(Geometry::Path(vec![
            C::Move(P::new(x + 20., y - 3.)),
            C::Line(P::new(x + 30., y - 3.)),
            C::Line(P::new(x + 30., y + 3.)),
            C::Close,
        ])))
    })
    .unwrap();
    let frame = swarm_widget_frame(&layout).unwrap();
    let packet = swarm_node_widgets(&layout, &raw, 9, placement).unwrap();
    let observed = Rc::new(RefCell::new(vec![]));
    let out = observed.clone();
    let app = App::memory();
    app.mount(
        chart_layout(frame.clone())
            .legend(false)
            .plot_widgets_with_data(packet, move |c| {
                out.borrow_mut().push(
                    c.frame
                        .with(|f| (f.frame.rect, f.frame.mark.clone(), f.data.clone())),
                );
                Ok(glory::ui::button().label("actual Swarm child"))
            }),
    )
    .unwrap();
    assert_eq!(observed.borrow().len(), 4);
    for (rect, mark, data) in observed.borrow().iter() {
        assert_eq!(
            [rect.x, rect.y],
            [data.site.position[0] - 8., data.site.position[1] - 4.]
        );
        assert_eq!(
            frame
                .interaction
                .as_ref()
                .unwrap()
                .anchor_for(mark)
                .unwrap(),
            Some(data.site.position)
        );
        assert_eq!(
            data.site.ink_radius,
            layout.sites[data.site_index].ink_radius
        );
        assert_eq!(data.source[data.source_index].id, data.site.node_id);
    }
    app.clear().unwrap();
    assert_eq!(app.stats().nodes, 0);
    let mut forged = layout.clone();
    let s = forged.sites.last().unwrap();
    let m = &forged.scene.marks[s.mark_index];
    forged.anchors.insert(
        s.mark_id.clone(),
        InteractionAnchor::path_points(
            &m.geometry,
            [s.position[0] + 20., s.position[1] - 3.],
            &[0],
        )
        .unwrap(),
    );
    let mut calls = 0;
    assert!(
        swarm_node_widgets(&forged, &raw, 0, |_| {
            calls += 1;
            Ok(None)
        })
        .is_err()
    );
    assert_eq!(calls, 0);
}

#[test]
fn exact_source_equality_includes_unselected_signed_zero_but_not_geometry_policy() {
    let raw = source();
    let l = swarm_layout(&config(), &raw, &options()).unwrap();
    let p = swarm_node_widgets(&l, &raw, 0, |c| {
        Ok((c.datum.id == "high").then(|| placement(c).unwrap().unwrap()))
    })
    .unwrap();
    let a = p.data.values().next().unwrap().clone();
    let mut b = a.clone();
    assert_eq!(a, b);
    b.source = Arc::new((*a.source).clone());
    assert_eq!(a, b);
    Arc::make_mut(&mut b.source)[1].value = 0.;
    assert_ne!(a, b);
    b = a.clone();
    Arc::make_mut(&mut b.source)[1].size = Some(0.);
    assert_ne!(a, b);
    b = a.clone();
    Arc::make_mut(&mut b.source)[4].label = Some("hidden changed".into());
    assert_ne!(a, b);
    b = a.clone();
    b.site.raw_value = -0.;
    let mut c = b.clone();
    c.site.raw_value = 0.;
    assert_ne!(b, c);
    b = a.clone();
    b.site.size = -0.;
    let mut c = b.clone();
    c.site.size = 0.;
    assert_eq!(b, c);
    let mut o = l.clone();
    for s in &o.sites {
        o.scene.marks[s.mark_index].opacity = 0.;
    }
    assert!(swarm_node_widgets(&o, &raw, 0, placement).is_ok());
}

#[test]
fn empty_hidden_skips_errors_and_selected_slot_budget_are_bounded() {
    let empty = swarm_layout(&config(), &[], &options()).unwrap();
    let mut calls = 0;
    let packet = swarm_node_widgets(&empty, &[], 1, |_| {
        calls += 1;
        Ok(None)
    })
    .unwrap();
    assert_eq!(calls, 0);
    assert_eq!(packet.layout.plot, Some(empty.plot));
    assert!(packet.data.is_empty());
    assert_eq!(swarm_widget_frame(&empty).unwrap().plot, Some(empty.plot));
    let raw = source();
    let layout = swarm_layout(&config(), &raw, &options()).unwrap();
    assert!(
        swarm_node_widgets(&layout, &raw, 0, |_| Ok(None))
            .unwrap()
            .data
            .is_empty()
    );
    let mut order = vec![];
    assert!(
        swarm_node_widgets(&layout, &raw, 0, |c| {
            order.push(c.site_index);
            if c.site_index == 2 {
                Err(ChartError("factory fail".into()))
            } else {
                placement(c)
            }
        })
        .is_err()
    );
    assert_eq!(order, [0, 1, 2]);
    let mut o = options();
    o.hidden = ["north".into(), "south".into(), "ghost".into()].into();
    let hidden = swarm_layout(&config(), &raw, &o).unwrap();
    assert!(
        swarm_node_widgets(&hidden, &raw, 0, placement)
            .unwrap()
            .data
            .is_empty()
    );
    let raw: Vec<_> = (0..129)
        .map(|i| SwarmPoint::new(format!("n{i:03}"), "north", i as f64))
        .collect();
    let mut o = options();
    o.value_domain = Some([0., 128.]);
    o.node.shape = SwarmNodeShape::Circle { diameter: 0. };
    let many = swarm_layout(&config(), &raw, &o).unwrap();
    let mut calls = 0;
    assert!(
        swarm_node_widgets(&many, &raw, 0, |c| {
            calls += 1;
            placement(c)
        })
        .is_err()
    );
    assert_eq!(calls, 129);
    assert_eq!(
        swarm_node_widgets(&many, &raw, 0, |c| Ok(
            (c.site_index < 128).then(|| placement(c).unwrap().unwrap())
        ))
        .unwrap()
        .data
        .len(),
        128
    );
}

#[test]
fn bit_only_unselected_source_update_reaches_retained_real_child() {
    use glory::Widget;
    use glory::render::{Event, EventKind, Props};
    #[derive(Clone)]
    struct Source {
        frame: SceneInteraction,
        packet: PlotWidgetsData<SwarmNodeWidgetData>,
    }
    let candidate = |positive: bool| {
        let mut raw = source();
        raw[4].value = if positive { 0. } else { -0. };
        raw[4].size = Some(if positive { 0. } else { -0. });
        let l = swarm_layout(&config(), &raw, &options()).unwrap();
        Source {
            frame: swarm_widget_frame(&l).unwrap(),
            packet: swarm_node_widgets(&l, &raw, 0, |c| {
                Ok((c.datum.id == "high").then(|| placement(c).unwrap().unwrap()))
            })
            .unwrap(),
        }
    };
    let first = candidate(false);
    let second = candidate(true);
    assert_eq!(first.frame, second.frame);
    assert_eq!(
        render::to_svg(&first.frame.scene).unwrap(),
        render::to_svg(&second.frame.scene).unwrap()
    );
    assert_eq!(first.packet.layout, second.packet.layout);
    assert_ne!(first.packet, second.packet);
    let app = App::memory();
    let input = app.scope().cage(first);
    let frame = app.scope().bond(move || input.with(|s| s.frame.clone()));
    let packet = app.scope().bond(move || input.with(|s| s.packet.clone()));
    app.mount(
        chart_layout(frame)
            .legend(false)
            .plot_widgets_with_data(packet, |context| {
                let frame = context.frame;
                Ok(glory::ui::widget(move |scope| {
                    let count = scope.cage(0usize);
                    let focus = scope.cage(false);
                    glory::ui::button()
                        .class("swarm-bits-child")
                        .label(scope.bond(move || {
                            frame.with(|f| {
                                format!(
                                    "{}:{} count={} focus={}",
                                    f.data.source[4].value.to_bits(),
                                    f.data.source[4].size.unwrap().to_bits(),
                                    count.get(),
                                    focus.get()
                                )
                            })
                        }))
                        .on(EventKind::Activate, move |_| count.revise(|v| *v += 1))
                        .on(EventKind::FocusChanged, move |event| {
                            if let Event::FocusChanged(v) = event {
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
                .any(|c| c == "swarm-bits-child")
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
    assert_eq!(
        label(id),
        "9223372036854775808:9223372036854775808 count=1 focus=true"
    );
    input.revise(|s| *s = second);
    app.flush().unwrap();
    assert_eq!(find(), id);
    assert_eq!(label(id), "0:0 count=1 focus=true");
    app.clear().unwrap();
    assert_eq!(app.stats().nodes, 0);
}
