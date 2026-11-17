use glory::render::{GraphicPoint as P, PathCommand as C};
use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 420.,
        height: 300.,
        margin: Margin {
            left: 20.,
            right: 20.,
            top: 20.,
            bottom: 20.,
        },
        ..Default::default()
    }
}
fn simulation() -> ForceSimulation {
    let mut s = ForceSimulation::new(
        &[NetworkNode::new("a"), NetworkNode::new("b")],
        &[
            NetworkLink::new("a", "b", 9.),
            NetworkLink::new("a", "b", 0.),
            NetworkLink::new("b", "a", 4.),
            NetworkLink::new("a", "a", 1.),
        ],
        ForceOptions {
            labels: false,
            ..Default::default()
        },
    )
    .unwrap();
    s.set_position("a", [-60., 0.]).unwrap();
    s.set_position("b", [60., 0.]).unwrap();
    s
}
fn curve(ctx: &NetworkLinkContext<'_>) -> NetworkLinkGeometry {
    let [x0, y0] = ctx.source_position;
    let [x1, y1] = ctx.target_position;
    ctx.inherit(Geometry::Path(vec![
        C::Move(P::new(x0, y0)),
        C::Cubic {
            first: P::new(x0, y0 - 80.),
            second: P::new(x1, y1 - 80.),
            end: P::new(x1, y1),
        },
    ]))
}
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} != {b}");
}

#[test]
fn inherited_links_preserve_complete_scene_nodes_bindings_and_physics() {
    let c = config();
    let s = simulation();
    let old_nodes = s.nodes().to_vec();
    let old_edges = s.edges().to_vec();
    let options = NetworkGlyphOptions::default();
    let original = network_glyph_layout(&c, &s, &options).unwrap();
    let mut calls = vec![];
    let next = network_glyph_layout_with_link_factory(&c, &s, &options, &mut |ctx| {
        calls.push((
            ctx.link_index,
            ctx.source.id.clone(),
            ctx.target.id.clone(),
            ctx.edge.value,
        ));
        assert_eq!(
            ctx.source_position,
            [ctx.source.position[0] + 210., ctx.source.position[1] + 150.]
        );
        near(
            ctx.position[0],
            ctx.source_position[0] * 0.5 + ctx.target_position[0] * 0.5,
        );
        Ok(ctx.inherit(ctx.geometry.clone()))
    })
    .unwrap();
    assert_eq!(
        calls,
        [
            (0, "a".into(), "b".into(), 9.),
            (1, "a".into(), "b".into(), 0.),
            (2, "b".into(), "a".into(), 4.),
            (3, "a".into(), "a".into(), 1.)
        ]
    );
    assert_eq!(next.scene(), original.scene());
    assert_eq!(next.node_layout().sites(), original.sites());
    assert_eq!(next.interaction_binding(), original.interaction_binding());
    assert_eq!(s.nodes(), old_nodes);
    assert_eq!(s.edges(), old_edges);
    assert_eq!(next.links().len(), 4);
    assert_eq!(next.links()[0].position, [210., 150.]);
    assert_eq!(next.links()[0].mark_id, "force-link:0");
}

#[test]
fn new_ink_preserves_endpoints_and_raw_metadata_and_rebuilds_after_motion() {
    let c = config();
    let mut s = simulation();
    let glyphs = NetworkGlyphOptions::default();
    let next =
        network_glyph_layout_with_link_factory(&c, &s, &glyphs, &mut |ctx| Ok(curve(ctx))).unwrap();
    let mark = next
        .scene()
        .marks
        .iter()
        .find(|m| m.id == "force-link:0")
        .unwrap();
    let mut ink = next.scene().clone();
    ink.marks = vec![mark.clone()];
    ink.draw_order.clear();
    assert!(ink.hit_test(210., 90.).is_some());
    assert!(ink.hit_test(210., 150.).is_none());
    assert_eq!(mark.value, 9.);
    assert_eq!(mark.label, "a → b");
    assert_eq!(next.links()[0].source_position, [150., 150.]);
    assert_eq!(next.links()[0].target_position, [270., 150.]);
    assert_eq!(next.links()[0].position, [210., 150.]);
    // Source endpoints and physical force radii remain the original nodes.
    assert_eq!(next.node_layout().sites()[0].physics_radius, 8.);
    s.set_position("b", [100., 30.]).unwrap();
    let moved =
        network_glyph_layout_with_link_factory(&c, &s, &glyphs, &mut |ctx| Ok(curve(ctx))).unwrap();
    assert_eq!(moved.links()[0].target_position, [310., 180.]);
    assert_eq!(next.links()[0].target_position, [270., 150.]);
    let frame: SceneInteraction = moved.into();
    frame.validate().unwrap();
    assert_eq!(
        frame.interaction.unwrap().sites.len(),
        2,
        "links remain decorative in the node binding"
    );
}

#[test]
fn explicit_relation_ids_keep_parallel_and_self_edges_separate_with_custom_ink() {
    let c = config();
    let s = simulation();
    let ids = ["parallel:1", "parallel:2", "reverse", "self"].map(String::from);
    let glyphs = NetworkGlyphOptions::default();
    let options = RelationOptions::default();
    let original = network_glyph_relation_layout(&c, &s, &glyphs, &ids, &options).unwrap();
    let next = network_glyph_relation_layout_with_link_factory(
        &c,
        &s,
        &glyphs,
        &ids,
        &options,
        &mut |ctx| Ok(ctx.inherit(ctx.geometry.clone())),
    )
    .unwrap();
    assert_eq!(next, original);
    let mut curved = network_glyph_relation_layout_with_link_factory(
        &c,
        &s,
        &glyphs,
        &ids,
        &options,
        &mut |ctx| Ok(curve(ctx)),
    )
    .unwrap();
    assert_eq!(curved.graph(), original.graph());
    let old = curved.authored_scene().clone();
    curved
        .set_focus(Some(RelationFocus::Edge("parallel:1".into())))
        .unwrap();
    curved.set_focus(None).unwrap();
    assert_eq!(curved.scene(), &old);
}

#[test]
fn rejected_candidate_keeps_simulation_and_existing_frame_and_stops_callbacks() {
    let c = config();
    let s = simulation();
    let nodes = s.nodes().to_vec();
    let base = s.scene(&c).unwrap();
    let mut calls = vec![];
    let failed = network_glyph_layout_with_link_factory(&c, &s, &Default::default(), &mut |ctx| {
        calls.push(ctx.link_index);
        if ctx.link_index == 1 {
            return Err(ChartError("deliberate link failure".into()));
        }
        Ok(curve(ctx))
    });
    assert!(failed.is_err());
    assert_eq!(calls, [0, 1]);
    assert_eq!(s.nodes(), nodes);
    assert_eq!(s.scene(&c).unwrap(), base);
    assert!(
        network_glyph_layout_with_link_factory(&c, &s, &Default::default(), &mut |ctx| Ok(ctx
            .inherit(Geometry::Circle {
                x: ctx.position[0] + 1.,
                y: ctx.position[1],
                radius: 3.
            })))
        .is_err()
    );
    assert!(
        network_glyph_layout_with_link_factory(&c, &s, &Default::default(), &mut |ctx| {
            let mut g = curve(ctx);
            g.opacity = f64::NAN;
            Ok(g)
        })
        .is_err()
    );
}

#[test]
fn all_custom_links_share_the_scene_command_budget() {
    let c = config();
    let s = simulation();
    let mut calls = 0;
    let result = network_glyph_layout_with_link_factory(&c, &s, &Default::default(), &mut |ctx| {
        calls += 1;
        let mut p = vec![C::Move(P::new(ctx.position[0], ctx.position[1])); 100_001];
        *p.last_mut().unwrap() = C::Line(P::new(ctx.position[0], ctx.position[1]));
        Ok(ctx.inherit(Geometry::Path(p)))
    });
    assert!(result.unwrap_err().to_string().contains("budget"));
    assert_eq!(calls, 2);
}

#[test]
fn custom_links_share_the_paint_stop_budget_and_admit_the_exact_limit() {
    let c = config();
    let links = (0..33)
        .map(|_| NetworkLink::new("a", "b", 1.))
        .collect::<Vec<_>>();
    let s = ForceSimulation::new(
        &[NetworkNode::new("a"), NetworkNode::new("b")],
        &links,
        ForceOptions {
            labels: false,
            ..Default::default()
        },
    )
    .unwrap();
    let stops = (0..512)
        .map(|i| GradientStop::new(f64::from(i) / 511., Color(30, 60, 100, 255)))
        .collect::<Vec<_>>();
    for (limit, valid) in [(32, true), (33, false)] {
        let result =
            network_glyph_layout_with_link_factory(&c, &s, &Default::default(), &mut |ctx| {
                let mut g = ctx.inherit(ctx.geometry.clone());
                if ctx.link_index < limit {
                    g.fill_paint = Some(FillPaint::Linear {
                        start: [0., 0.],
                        end: [420., 0.],
                        stops: stops.clone(),
                    });
                }
                Ok(g)
            });
        assert_eq!(result.is_ok(), valid);
        if let Err(error) = result {
            assert!(error.0.contains("paint budget"));
        }
    }
}

#[test]
fn real_glory_node_controls_ignore_decorative_link_ink_and_reject_bad_binding_atomically() {
    use std::cell::RefCell;
    use std::rc::Rc;

    use glory::render::{Event, NodeId, PlotPointer, PlotPointerDevice, PlotPointerPhase, Rect};
    fn nodes(app: &glory::App, class: &str) -> Vec<NodeId> {
        let tree = app.tree();
        let mut stack = tree.roots().to_vec();
        let mut found = vec![];
        while let Some(id) = stack.pop() {
            let n = tree.node(id).unwrap();
            if n.descriptor.view().classes.iter().any(|c| c == class) {
                found.push(id);
            }
            stack.extend(n.children.iter().copied());
        }
        found
    }
    let c = config();
    let s = simulation();
    let frame = network_glyph_layout_with_link_factory(&c, &s, &Default::default(), &mut |ctx| {
        Ok(curve(ctx))
    })
    .unwrap()
    .into_painted_scene_interaction()
    .unwrap();
    let app = glory::App::memory();
    let input = app.scope().cage(frame.clone());
    let hits = Rc::new(RefCell::new(vec![]));
    let observed = hits.clone();
    let failures = Rc::new(RefCell::new(vec![]));
    let rejected = failures.clone();
    app.mount(
        chart_layout(input)
            .highlight(false)
            .on_physical_hover(move |hit| observed.borrow_mut().push(hit))
            .on_reject(move |e| rejected.borrow_mut().push(e)),
    )
    .unwrap();
    assert_eq!(nodes(&app, "kartis-datum").len(), 2);
    let plot = nodes(&app, "kartis-plot")[0];
    for point in [[150., 150.], [210., 90.]] {
        app.dispatch(
            plot,
            Event::PlotPointer {
                from: plot,
                pointer: PlotPointer {
                    pointer_id: 9,
                    device: PlotPointerDevice::Pen,
                    phase: PlotPointerPhase::Move,
                    x: point[0] * 2.,
                    y: point[1] * 2.,
                    viewport: Rect {
                        width: 840.,
                        height: 600.,
                        ..Default::default()
                    },
                },
            },
        )
        .unwrap();
        app.flush().unwrap();
    }
    let seen = hits.borrow();
    assert!(
        seen.iter()
            .flatten()
            .any(|h| h.logical_site_id == "force-node:a")
    );
    assert!(seen.last().unwrap().is_none());
    drop(seen);
    let old = format!("{:?}", app.tree());
    let mut bad = frame.clone();
    bad.interaction.as_mut().unwrap().sites[0].position[0] = f64::NAN;
    input.revise(|f| *f = bad);
    app.flush().unwrap();
    assert_eq!(failures.borrow().len(), 1);
    assert_eq!(format!("{:?}", app.tree()), old);
}

#[cfg(feature = "raster")]
#[test]
fn curved_link_ink_is_really_painted_at_both_dprs() {
    use render::raster::*;
    let c = config();
    let s = simulation();
    let color = Color(221, 32, 88, 255);
    let layout = network_glyph_layout_with_link_factory(&c, &s, &Default::default(), &mut |ctx| {
        let mut g = curve(ctx);
        g.stroke = Some(glory::render::GraphicStroke {
            color,
            width: 8.,
            cap: glory::render::StrokeCap::Round,
        });
        Ok(g)
    })
    .unwrap();
    for dpr in [1., 2.] {
        let image = render_raster(
            layout.scene(),
            &FontRegistry::new(),
            &RasterOptions {
                pixel_ratio: dpr,
                ..Default::default()
            },
        )
        .unwrap();
        let offset = (((90. * dpr) as usize) * image.width as usize + (210. * dpr) as usize) * 4;
        assert_eq!(&image.rgba[offset..offset + 4], [221, 32, 88, 255]);
    }
}
