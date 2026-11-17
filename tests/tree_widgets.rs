#[cfg(feature = "raster")]
use std::cell::Cell;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use glory::render::{Event, EventKind, GraphicPoint as P, NodeId, PathCommand as C, Props};
use glory::{App, Widget};
use kartis::*;

fn config(width: f64) -> ChartConfig {
    ChartConfig {
        width,
        height: 280.,
        margin: Margin {
            left: 30.,
            right: 20.,
            top: 18.,
            bottom: 26.,
        },
        ..Default::default()
    }
}
fn source() -> Vec<HierarchyDatum> {
    let mut raw = vec![
        HierarchyDatum::new("a", Some("branch".into()), 2.),
        HierarchyDatum::new("root", None, -0.),
        HierarchyDatum::new("branch", Some("root".into()), -0.),
        HierarchyDatum::new("zero", Some("branch".into()), -0.),
        HierarchyDatum::new("b", Some("root".into()), 3.),
    ];
    raw[3].label.clear();
    raw
}
fn options() -> TreeOptions {
    TreeOptions {
        hierarchy: HierarchyOptions {
            labels: false,
            ..Default::default()
        },
        ..Default::default()
    }
}
fn placement() -> PlotWidgetPlacement {
    PlotWidgetPlacement::AtDatum {
        offset: [-8., -4.],
        measured: TextSize {
            width: 16.,
            height: 8.,
        },
    }
}
fn nodes(app: &App, class: &str) -> Vec<NodeId> {
    let tree = app.tree();
    let mut queue = tree.roots().to_vec();
    let mut out = vec![];
    while let Some(id) = queue.pop() {
        let n = tree.node(id).unwrap();
        if n.descriptor.view().classes.iter().any(|c| c == class) {
            out.push(id);
        }
        queue.extend(n.children.iter().copied());
    }
    out.sort();
    out
}
fn text(app: &App, id: NodeId) -> String {
    let tree = app.tree();
    let Props::Button(p) = tree.node(id).unwrap().descriptor.view().props else {
        panic!("real Button")
    };
    p.label.clone()
}

#[test]
fn captured_source_topology_aggregate_and_true_scene_order_share_complete_pools() {
    let mut raw = source();
    let layout = tree_layout(&config(420.), &raw, &options()).unwrap();
    assert_eq!(layout.root_index(), Some(1));
    assert_eq!(layout.source(), raw);
    assert!(!layout.options().hierarchy.include_root);
    assert_eq!(
        layout
            .nodes
            .iter()
            .map(|n| (n.parent_index, n.depth, n.height, n.value))
            .collect::<Vec<_>>(),
        [
            (Some(2), 2, 0, 2.),
            (None, 0, 2, 5.),
            (Some(1), 1, 1, 2.),
            (Some(2), 2, 0, -0.),
            (Some(1), 1, 0, 3.)
        ]
    );
    assert_eq!(layout.nodes[1].children, [2, 4]);
    assert_eq!(layout.nodes[2].children, [0, 3]);
    assert_eq!(
        layout
            .links
            .iter()
            .map(|l| (l.source_index, l.target_index))
            .collect::<Vec<_>>(),
        [(1, 2), (1, 4), (2, 0), (2, 3)]
    );
    let before = layout.clone();
    let mut order = vec![];
    let packet = tree_widgets(&layout, 7, |c| {
        let id = match c {
            TreeWidgetContext::Node(c) => {
                assert!(std::ptr::eq(c.datum, &layout.source()[c.source_index]));
                assert!(std::ptr::eq(c.node, &layout.nodes[c.source_index]));
                c.mark.id.clone()
            }
            TreeWidgetContext::Link(c) => {
                assert!(std::ptr::eq(
                    c.source_datum,
                    &layout.source()[c.link.source_index]
                ));
                assert!(std::ptr::eq(
                    c.target_datum,
                    &layout.source()[c.link.target_index]
                ));
                c.mark.id.clone()
            }
        };
        order.push(id);
        Ok(Some(placement()))
    })
    .unwrap();
    assert_eq!(layout, before);
    assert_eq!(
        order,
        layout
            .scene
            .paint_marks()
            .map(|m| m.id.clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(packet.data.len(), 9);
    let mut pools = None;
    for data in packet.data.values() {
        let (source, nodes, links, labels, position) = match data {
            TreeWidgetData::Node(d) => (&d.source, &d.nodes, &d.links, &d.labels, d.node.position),
            TreeWidgetData::Link(d) => (&d.source, &d.nodes, &d.links, &d.labels, d.link.position),
        };
        if let Some((s, n, l, ls)) = pools {
            assert!(
                Arc::ptr_eq(source, s)
                    && Arc::ptr_eq(nodes, n)
                    && Arc::ptr_eq(links, l)
                    && Arc::ptr_eq(labels, ls)
            );
        } else {
            pools = Some((source, nodes, links, labels));
        }
        assert_eq!(source[3].value.to_bits(), (-0_f64).to_bits());
        let id = match data {
            TreeWidgetData::Node(d) => d.node.mark_id.as_ref().unwrap(),
            TreeWidgetData::Link(d) => d.link.mark_id.as_ref().unwrap(),
        };
        assert_eq!(
            packet
                .layout
                .slots
                .iter()
                .find(|s| &s.mark_id == id)
                .unwrap()
                .placement,
            PlotWidgetPlacement::Rect(LayoutRect {
                x: position[0] - 8.,
                y: position[1] - 4.,
                width: 16.,
                height: 8.
            })
        );
    }
    raw[0].value = 99.;
    assert_eq!(layout.source()[0].value, 2.);
    let frame = tree_widget_frame(&layout).unwrap();
    assert_eq!(frame.scene, layout.scene);
    let binding = frame.interaction.unwrap();
    assert_eq!(
        binding.sites.len(),
        4,
        "empty-label node and links do not invent business controls"
    );
    assert!(binding.sites.iter().all(|s| s.mark_id.starts_with("node:")));
}

#[test]
fn all_directions_curves_modes_zero_radius_transparent_and_disabled_roles() {
    use kartis::surface::{SurfaceModel, SurfaceOptions};
    let raw = source();
    for orientation in [
        TreeOrientation::LeftToRight,
        TreeOrientation::RightToLeft,
        TreeOrientation::TopToBottom,
        TreeOrientation::BottomToTop,
        TreeOrientation::Radial,
    ] {
        for mode in [TreeMode::Tidy, TreeMode::Cluster] {
            for shape in [
                TreeNodeShape::Circle,
                TreeNodeShape::Square,
                TreeNodeShape::Diamond,
            ] {
                let mut o = options();
                o.orientation = orientation;
                o.mode = mode;
                o.node_radius = 0.;
                o.style.nodes.shape = shape;
                o.style.nodes.radius = Some(0.);
                o.style.nodes.fill = TreeNodeFill::None;
                o.style.nodes.opacity = 0.;
                o.style.links.enabled = false;
                let layout = tree_layout(&config(420.), &raw, &o).unwrap();
                let p = tree_widgets(&layout, 0, |_| Ok(Some(placement()))).unwrap();
                assert_eq!(p.data.len(), raw.len());
                assert_eq!(layout.links.len(), 4);
                assert!(layout.links.iter().all(|l| l.mark_id.is_none()));
                assert_eq!(
                    tree_link_widgets(&layout, 0, |_| panic!("disabled"))
                        .unwrap()
                        .data
                        .len(),
                    0
                );
                let frame = tree_widget_frame(&layout).unwrap();
                let model = SurfaceModel::new_with_interaction(
                    frame.scene,
                    SurfaceOptions {
                        plot: frame.plot,
                        ..Default::default()
                    },
                    frame.interaction,
                )
                .unwrap();
                assert!(
                    model
                        .physical_hit_at((layout.nodes[0].position[0], layout.nodes[0].position[1]))
                        .unwrap()
                        .is_none()
                );
                assert_eq!(
                    tree_node_widgets(&layout, 0, |_| Ok(Some(placement())))
                        .unwrap()
                        .data
                        .len(),
                    5
                );
            }
            for curve in [
                TreeLinkCurve::Straight,
                TreeLinkCurve::Bump,
                TreeLinkCurve::Step,
                TreeLinkCurve::StepBefore,
                TreeLinkCurve::StepAfter,
            ] {
                let mut o = options();
                o.orientation = orientation;
                o.mode = mode;
                o.style.links.curve = curve;
                o.style.links.opacity = 0.;
                let l = tree_layout(&config(420.), &raw, &o).unwrap();
                let p = tree_link_widgets(&l, 0, |c| {
                    assert_eq!(c.mark.opacity, 0.);
                    Ok(Some(placement()))
                })
                .unwrap();
                assert_eq!(p.data.len(), 4);
                assert!(p.data.values().all(|d| d.link.position
                    == [
                        d.link.source_position[0] * 0.5 + d.link.target_position[0] * 0.5,
                        d.link.source_position[1] * 0.5 + d.link.target_position[1] * 0.5
                    ]));
            }
        }
    }
    let mut o = options();
    o.style.nodes.radius = Some(1e-9);
    assert!(
        tree_layout(&config(420.), &raw, &o).is_err(),
        "only exactly zero was added to existing style admission"
    );
}

#[test]
fn current_offset_paths_paint_and_measured_labels_preserve_original_anchors() {
    let raw = source();
    let mut o = options();
    o.hierarchy.labels = true;
    let mut l = tree_layout(&config(420.), &raw, &o).unwrap();
    let p = l.nodes[0].position;
    let body = l.scene.marks.iter_mut().find(|m| m.id == "node:a").unwrap();
    body.geometry = Geometry::Path(vec![
        C::Move(P::new(p[0] + 20., p[1] - 5.)),
        C::Line(P::new(p[0] + 30., p[1] - 5.)),
        C::Line(P::new(p[0] + 30., p[1] + 5.)),
        C::Line(P::new(p[0] + 20., p[1] + 5.)),
        C::Close,
    ]);
    body.fill = Some(Color(200, 20, 120, 255));
    let link = l.links[0].clone();
    let mark = l
        .scene
        .marks
        .iter_mut()
        .find(|m| m.id == link.mark_id.as_deref().unwrap())
        .unwrap();
    mark.geometry = Geometry::Path(vec![
        C::Move(P::new(link.position[0] + 100., link.position[1])),
        C::Line(P::new(link.position[0] + 110., link.position[1])),
    ]);
    l.scene.labels[0].x += 40.;
    l.scene.labels[0].text = "measured current text".into();
    let extra = l.scene.labels[0].clone();
    let i = l.scene.labels.len();
    l.scene.labels.push(extra);
    l.scene
        .label_series
        .insert(i, l.scene.label_series[&0].clone());
    l.scene.label_clips.insert(
        i,
        ClipPath {
            geometry: Geometry::Rect {
                x: l.plot.x,
                y: l.plot.y,
                width: l.plot.width,
                height: l.plot.height,
            },
            rule: ClipRule::NonZero,
        },
    );
    let frame = tree_widget_frame(&l).unwrap();
    let binding = frame.interaction.as_ref().unwrap();
    assert_eq!(
        binding
            .anchor_for(l.scene.marks.iter().find(|m| m.id == "node:a").unwrap())
            .unwrap(),
        Some(p)
    );
    let packet = tree_widgets(&l, 0, |_| Ok(Some(placement()))).unwrap();
    let TreeWidgetData::Node(d) = &packet.data["node:a"] else {
        panic!("node")
    };
    assert_eq!(d.node.position, p);
    assert_eq!(d.labels.labels[0].text, "measured current text");
    assert_eq!(d.labels.clips[&i], l.scene.label_clips[&i]);
    assert_eq!(
        packet
            .layout
            .slots
            .iter()
            .find(|s| s.mark_id == "node:a")
            .unwrap()
            .placement,
        PlotWidgetPlacement::Rect(LayoutRect {
            x: p[0] - 8.,
            y: p[1] - 4.,
            width: 16.,
            height: 8.
        })
    );
    let TreeWidgetData::Link(d) = &packet.data[link.mark_id.as_ref().unwrap()] else {
        panic!("link")
    };
    assert_eq!(d.link.position, link.position);
    let mut input = source();
    input[0].value = 100.;
    assert_eq!(d.source[0].value, 2.);
}

#[test]
fn complete_frame_conflicts_fail_before_first_of_either_callback_role() {
    type Mutation = fn(&mut TreeLayout);
    let cases: &[(&str, Mutation)] = &[
        ("source index", |l| l.nodes[0].source_index = 3),
        ("parent", |l| l.nodes[0].parent_index = None),
        ("children", |l| l.nodes[1].children.clear()),
        ("depth", |l| l.nodes[4].depth = 9),
        ("height", |l| l.nodes[1].height = 9),
        ("raw bits", |l| l.nodes[3].raw_value = 0.),
        ("aggregate", |l| l.nodes[2].value = 0.),
        ("center", |l| l.nodes[4].position[0] += 1.),
        ("radius", |l| l.nodes[4].radius += 1.),
        ("style", |l| l.nodes[4].style.opacity = 0.),
        ("color", |l| l.nodes[4].color = None),
        ("node role", |l| l.nodes[4].mark_id = None),
        ("link index", |l| l.links[3].link_index = 0),
        ("link source", |l| l.links[3].source_index = 0),
        ("link target", |l| l.links[3].target_index = 0),
        ("link endpoint", |l| l.links[3].source_position[0] += 1.),
        ("link midpoint", |l| l.links[3].position[0] += 1.),
        ("link role", |l| l.links[3].mark_id = None),
        ("plot", |l| l.plot.width += 1.),
        ("scene size", |l| l.scene.width += 1.),
        ("node scalar", |l| {
            l.scene
                .marks
                .iter_mut()
                .find(|m| m.id == "node:b")
                .unwrap()
                .value = 0.
        }),
        ("node series", |l| {
            l.scene
                .marks
                .iter_mut()
                .find(|m| m.id == "node:b")
                .unwrap()
                .series = "a".into()
        }),
        ("node label", |l| {
            l.scene
                .marks
                .iter_mut()
                .find(|m| m.id == "node:b")
                .unwrap()
                .label = "wrong".into()
        }),
        ("node center", |l| {
            l.scene
                .marks
                .iter_mut()
                .find(|m| m.id == "node:b")
                .unwrap()
                .geometry = Geometry::Circle {
                x: 1.,
                y: 1.,
                radius: 2.,
            }
        }),
        ("missing node", |l| {
            l.scene.marks.retain(|m| m.id != "node:b")
        }),
        ("missing link", |l| {
            l.scene.marks.retain(|m| m.id != "link:b")
        }),
        ("link business", |l| {
            l.scene
                .marks
                .iter_mut()
                .find(|m| m.id == "link:b")
                .unwrap()
                .label = "fake".into()
        }),
        ("orphan role", |l| {
            l.scene.marks.push(Mark::new(
                "node:fake",
                Geometry::Circle {
                    x: 20.,
                    y: 20.,
                    radius: 0.,
                },
                Color(0, 0, 0, 0),
            ))
        }),
        ("label owner", |l| {
            l.scene.label_series.insert(0, "fake".into());
        }),
    ];
    let mut o = options();
    o.hierarchy.labels = true;
    let base = tree_layout(&config(420.), &source(), &o).unwrap();
    for (name, mutation) in cases {
        let mut l = base.clone();
        mutation(&mut l);
        let mut callbacks = 0;
        assert!(
            tree_widgets(&l, 0, |_| {
                callbacks += 1;
                Ok(Some(placement()))
            })
            .is_err(),
            "{name}"
        );
        assert_eq!(callbacks, 0, "{name}");
        assert!(
            tree_node_widgets(&l, 0, |_| {
                callbacks += 1;
                Ok(Some(placement()))
            })
            .is_err(),
            "{name}"
        );
        assert!(
            tree_link_widgets(&l, 0, |_| {
                callbacks += 1;
                Ok(Some(placement()))
            })
            .is_err(),
            "{name}"
        );
        assert_eq!(callbacks, 0);
        assert!(tree_widget_frame(&l).is_err(), "{name}");
    }
}

#[test]
fn empty_singleton_skip_error_and_one_mixed_budget() {
    let empty = tree_layout(&config(420.), &[], &options()).unwrap();
    assert_eq!(empty.root_index(), None);
    assert!(
        tree_widgets(&empty, 0, |_| panic!("no role"))
            .unwrap()
            .data
            .is_empty()
    );
    assert_eq!(tree_widget_frame(&empty).unwrap().plot, Some(empty.plot));
    let single = tree_layout(
        &config(420.),
        &[HierarchyDatum::new("root", None, 0.)],
        &options(),
    )
    .unwrap();
    assert_eq!(single.nodes.len(), 1);
    assert!(single.links.is_empty());
    assert_eq!(
        tree_widgets(&single, 0, |_| Ok(Some(placement())))
            .unwrap()
            .data
            .len(),
        1,
        "include_root false still preserves existing Tree root"
    );
    let l = tree_layout(&config(420.), &source(), &options()).unwrap();
    let before = l.clone();
    let mut count = 0;
    assert!(
        tree_widgets(&l, 0, |_| {
            count += 1;
            if count == 8 {
                Err(ChartError("late placement refused".into()))
            } else {
                Ok(Some(placement()))
            }
        })
        .is_err()
    );
    assert_eq!(count, 8);
    assert_eq!(l, before);
    let mut raw = vec![HierarchyDatum::new("root", None, 0.)];
    raw.extend((0..64).map(|i| HierarchyDatum::new(format!("leaf{i}"), Some("root".into()), 1.)));
    let l = tree_layout(&config(420.), &raw, &options()).unwrap();
    let mut count = 0;
    assert!(
        tree_widgets(&l, 0, |_| {
            count += 1;
            Ok(Some(placement()))
        })
        .is_err()
    );
    assert_eq!(count, 129);
    let p = tree_widgets(&l, 0, |c| {
        Ok(match c {
            TreeWidgetContext::Node(c) if c.datum.id == "root" => None,
            _ => Some(placement()),
        })
    })
    .unwrap();
    assert_eq!(p.data.len(), 128);
}

type Input = (SceneInteraction, PlotWidgetsComposition<TreeWidgetData>);
fn proposal(positive: bool, width: f64, revision: u64) -> Input {
    let mut raw = source();
    raw[3].value = if positive { 0. } else { -0. };
    let mut l = tree_layout(&config(width), &raw, &options()).unwrap();
    l.scene
        .marks
        .iter_mut()
        .find(|m| m.id == "node:zero")
        .unwrap()
        .value = 0.;
    let widgets = tree_widgets(&l, revision, |c| {
        Ok(match c {
            TreeWidgetContext::Node(c) if c.datum.id == "a" => Some(PlotWidgetPlacement::AtDatum {
                offset: [-20., -4.],
                measured: TextSize {
                    width: 16.,
                    height: 8.,
                },
            }),
            TreeWidgetContext::Link(c) if c.target_datum.id == "a" => Some(placement()),
            _ => None,
        })
    })
    .unwrap();
    (
        tree_widget_frame(&l).unwrap(),
        PlotWidgetsComposition {
            replace_ink: widgets.data.keys().cloned().collect(),
            widgets,
        },
    )
}
fn child(c: PlotWidgetCompositionContext<TreeWidgetData>) -> Result<impl Widget> {
    if c.revision == 90 {
        return Err(ChartError("Tree child factory denied".into()));
    }
    let reject = c.revision == 91;
    let frame = c.frame;
    Ok(glory::ui::widget(move |scope| {
        let count = scope.cage(0usize);
        let focus = scope.cage(false);
        let is_node = frame.with(|f| matches!(f.data, TreeWidgetData::Node(_)));
        glory::ui::button()
            .class(if is_node {
                "tree-node-role"
            } else {
                "tree-link-role"
            })
            .label(scope.bond(move || {
                frame.with(|f| {
                    let (source, kind) = match &f.data {
                        TreeWidgetData::Node(d) => (&d.source, "node"),
                        TreeWidgetData::Link(d) => (&d.source, "link"),
                    };
                    format!(
                        "{kind} bit={} count={} focus={}",
                        source[3].value.to_bits(),
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
            .show_in(scope)?;
        if reject {
            Err(glory::render::Error::InvalidProps("Tree child build denied").into())
        } else {
            Ok(())
        }
    }))
}

#[test]
fn real_memory_bit_only_updates_retain_both_roles_and_strict_rejections_keep_accepted_data() {
    let a = proposal(false, 420., 0);
    let b = proposal(true, 420., 0);
    assert_eq!(a.0, b.0);
    assert_eq!(
        render::to_svg(&a.0.scene).unwrap(),
        render::to_svg(&b.0.scene).unwrap()
    );
    assert_ne!(a.1, b.1);
    let app = App::memory();
    let input = app.scope().cage(a);
    let frame = app.scope().bond(move || input.with(|s| s.0.clone()));
    let data = app.scope().bond(move || input.with(|s| s.1.clone()));
    let errors = Rc::new(RefCell::new(vec![]));
    let err = errors.clone();
    app.mount(
        chart_layout(frame)
            .legend(false)
            .on_reject(move |e| err.borrow_mut().push(e.0))
            .plot_widgets_composed(data, child),
    )
    .unwrap();
    let node = nodes(&app, "tree-node-role")[0];
    let link = nodes(&app, "tree-link-role")[0];
    let controls = nodes(&app, "kartis-datum");
    assert_eq!(controls.len(), 4);
    for id in [node, link] {
        app.dispatch(id, Event::Activate).unwrap();
        app.dispatch(id, Event::FocusChanged(true)).unwrap();
        assert!(text(&app, id).contains("bit=9223372036854775808 count=1 focus=true"));
    }
    input.revise(|s| *s = b);
    app.flush().unwrap();
    for id in [node, link] {
        assert!(text(&app, id).contains("bit=0 count=1 focus=true"));
    }
    assert_eq!(nodes(&app, "tree-node-role"), [node]);
    assert_eq!(nodes(&app, "tree-link-role"), [link]);
    let accepted = [text(&app, node), text(&app, link)];
    for revision in [90, 91] {
        input.revise(|s| *s = proposal(false, 430., revision));
        app.flush().unwrap();
        assert_eq!([text(&app, node), text(&app, link)], accepted);
        assert_eq!(nodes(&app, "kartis-datum"), controls);
    }
    assert_eq!(errors.borrow().len(), 2);
    let mut bad = proposal(false, 440., 0);
    bad.1.widgets.data.pop_first();
    input.revise(|s| *s = bad);
    app.flush().unwrap();
    assert_eq!(errors.borrow().len(), 3);
    assert_eq!([text(&app, node), text(&app, link)], accepted);
    input.revise(|s| *s = proposal(true, 440., 1));
    app.flush().unwrap();
    assert_ne!(nodes(&app, "tree-node-role"), [node]);
    assert!(text(&app, nodes(&app, "tree-node-role")[0]).contains("count=0 focus=false"));
    app.clear().unwrap();
    assert_eq!(app.stats().nodes, 0);
}

#[cfg(feature = "raster")]
#[test]
fn cpu_two_dpr_real_png_publisher_rejection_preserves_mixed_typed_children_and_source() {
    use base64::Engine;
    use kartis::render::raster::{FontRegistry, RasterOptions};
    for dpr in [1., 2.] {
        let app = App::memory();
        let initial = proposal(false, 420., 0);
        let p = initial
            .0
            .interaction
            .as_ref()
            .unwrap()
            .sites
            .iter()
            .find(|s| s.mark_id == "node:b")
            .unwrap()
            .position;
        let expected = initial
            .0
            .scene
            .marks
            .iter()
            .find(|m| m.id == "node:b")
            .unwrap()
            .fill
            .unwrap();
        let input = app.scope().cage(initial);
        let frame = app.scope().bond(move || input.with(|s| s.0.clone()));
        let packet = app.scope().bond(move || input.with(|s| s.1.clone()));
        let denied = Rc::new(Cell::new(false));
        let deny = denied.clone();
        let errors = Rc::new(RefCell::new(vec![]));
        let err = errors.clone();
        app.mount(
            chart_layout(frame)
                .legend(false)
                .tooltip(false)
                .raster(
                    FontRegistry::new(),
                    RasterOptions {
                        pixel_ratio: dpr,
                        ..Default::default()
                    },
                )
                .raster_image_source(move |image| {
                    if deny.get() {
                        Err(ChartError("Tree PNG publisher denied".into()))
                    } else {
                        image.data_url()
                    }
                })
                .on_reject(move |e| err.borrow_mut().push(e.0))
                .plot_widgets_composed(packet, child),
        )
        .unwrap();
        let image = || {
            let tree = app.tree();
            let id = nodes(&app, "kartis-raster-image")[0];
            let Props::Image(p) = tree.node(id).unwrap().descriptor.view().props else {
                panic!("actual Image")
            };
            p.source.clone()
        };
        let old = image();
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(old.split_once(',').unwrap().1)
            .unwrap();
        let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
            .read_info()
            .unwrap();
        let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
        let info = reader.next_frame(&mut pixels).unwrap();
        assert_eq!(
            [info.width, info.height],
            [(420. * dpr) as u32, (280. * dpr) as u32]
        );
        let offset = 4 * ((p[1] * dpr) as usize * info.width as usize + (p[0] * dpr) as usize);
        assert_eq!(
            &pixels[offset..offset + 4],
            &[expected.0, expected.1, expected.2, expected.3]
        );
        let node = nodes(&app, "tree-node-role")[0];
        let link = nodes(&app, "tree-link-role")[0];
        app.dispatch(node, Event::Activate).unwrap();
        app.dispatch(node, Event::FocusChanged(true)).unwrap();
        let texts = [text(&app, node), text(&app, link)];
        let controls = nodes(&app, "kartis-datum");
        denied.set(true);
        input.revise(|s| *s = proposal(true, 430., 0));
        app.flush().unwrap();
        assert_eq!(image(), old);
        assert_eq!([text(&app, node), text(&app, link)], texts);
        assert_eq!(nodes(&app, "kartis-datum"), controls);
        assert_eq!(errors.borrow().len(), 1);
        denied.set(false);
        input.revise(|s| *s = proposal(true, 440., 0));
        app.flush().unwrap();
        assert_ne!(image(), old);
        assert!(text(&app, node).contains("bit=0 count=1 focus=true"));
        assert_eq!(nodes(&app, "tree-link-role"), [link]);
        app.clear().unwrap();
        assert_eq!(app.stats().nodes, 0);
    }
}
