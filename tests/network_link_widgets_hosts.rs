//! Actual Memory children and CPU-image publication; no browser/OS equivalence.
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use glory::render::{Event, EventKind, GraphicPoint, NodeId, PathCommand, Props};
use glory::{App, Widget};
use kartis::*;

fn config(width: f64) -> ChartConfig {
    ChartConfig {
        width,
        height: 240.,
        margin: Margin {
            left: 30.,
            right: 24.,
            top: 22.,
            bottom: 26.,
        },
        ..Default::default()
    }
}
fn simulation() -> ForceSimulation {
    let mut s = ForceSimulation::new(
        &[
            NetworkNode {
                id: "a".into(),
                label: String::new(),
                radius: 3.,
            },
            NetworkNode {
                id: "b".into(),
                label: "B".into(),
                radius: 6.,
            },
            NetworkNode {
                id: "other".into(),
                label: "Other".into(),
                radius: 7.,
            },
        ],
        &[
            NetworkLink::new("a", "b", 4.),
            NetworkLink::new("other", "b", 1.),
        ],
        ForceOptions {
            repulsion: 0.,
            center_strength: 0.,
            collision_strength: 0.,
            link_strength: 0.,
            velocity_decay: 0.,
            labels: false,
            ..Default::default()
        },
    )
    .unwrap();
    s.set_position("a", [-20., 0.]).unwrap();
    s.set_position("b", [20., 20.]).unwrap();
    s.set_position("other", [-60., 35.]).unwrap();
    s.set_velocity("other", [-0., 0.]).unwrap();
    s
}
fn source(
    s: &ForceSimulation,
    width: f64,
    revision: u64,
) -> (SceneInteraction, PlotWidgetsComposition<NetworkWidgetData>) {
    let mut glyphs = NetworkGlyphOptions::default();
    glyphs.nodes.insert(
        "other".into(),
        NetworkNodeGlyph {
            glyph: GlyphOptions {
                shape: GlyphShape::Filled(FilledGlyph::Circle),
                size: 100.,
                rotation: 0.,
            },
            fill: Some(Color(180, 70, 90, 255)),
            stroke: None,
        },
    );
    let layout = network_glyph_layout_with_link_factory(&config(width), s, &glyphs, &mut |c| {
        let mut ink = c.inherit(c.geometry.clone());
        if c.link_index == 0 {
            ink.geometry = Geometry::Path(vec![
                PathCommand::Move(GraphicPoint::new(255., 150.)),
                PathCommand::Line(GraphicPoint::new(275., 150.)),
                PathCommand::Line(GraphicPoint::new(265., 165.)),
                PathCommand::Close,
            ]);
            ink.fill = Some(Color(20, 90, 140, 255));
            ink.stroke = None;
        }
        Ok(ink)
    })
    .unwrap();
    let widgets = network_widgets(&layout, s, revision, |c| {
        let selected = match c {
            NetworkWidgetContext::Link(c) => c.link_index == 0,
            NetworkWidgetContext::Node(c) => c.node.id == "a",
        };
        Ok(selected.then_some(PlotWidgetPlacement::AtDatum {
            offset: [-6., -4.],
            measured: TextSize {
                width: 12.,
                height: 8.,
            },
        }))
    })
    .unwrap();
    (
        network_link_widget_frame(&layout).unwrap(),
        PlotWidgetsComposition {
            replace_ink: widgets.data.keys().cloned().collect(),
            widgets,
        },
    )
}
fn nodes(app: &App, class: &str) -> Vec<NodeId> {
    let tree = app.tree();
    let mut pending = tree.roots().to_vec();
    let mut ids = vec![];
    while let Some(id) = pending.pop() {
        let n = tree.node(id).unwrap();
        if n.descriptor.view().classes.iter().any(|c| c == class) {
            ids.push(id);
        }
        pending.extend(n.children.iter().copied());
    }
    ids
}
fn text(app: &App, id: NodeId) -> String {
    let tree = app.tree();
    let Props::Button(p) = tree.node(id).unwrap().descriptor.view().props else {
        panic!("actual Button expected")
    };
    p.label.clone()
}
fn child(
    context: PlotWidgetCompositionContext<NetworkWidgetData>,
    calls: Rc<Cell<usize>>,
) -> Result<impl Widget> {
    calls.set(calls.get() + 1);
    let frame = context.frame;
    let link = frame.with(|f| matches!(f.data, NetworkWidgetData::Link(_)));
    if link && context.revision == 90 {
        return Err(ChartError("independent link child factory rejected".into()));
    }
    let fail = link && context.revision == 91;
    Ok(glory::ui::widget(move |scope| {
        let count = scope.cage(0u32);
        let focus = scope.cage(false);
        glory::ui::button().class(if link { "independent-link-child" } else { "independent-node-child" }).label(scope.bond(move || frame.with(|f| {
            let (kind, source, ticks, alpha, position, value) = match &f.data {
                NetworkWidgetData::Link(d) => { assert_eq!(d.source_index, 0); assert_eq!(d.edges[d.source_index].value, 4.); ("link", &d.source, d.ticks, d.alpha, d.site.position, d.edges[0].value) },
                NetworkWidgetData::Node(d) => ("node", &d.source, d.ticks, d.alpha, d.site.position, 0.),
            };
            assert!((f.frame.rect.x + 6. - position[0]).abs() < 1e-10); assert!((f.frame.rect.y + 4. - position[1]).abs() < 1e-10);
            format!("kind={kind};other={:016x},{:016x};ticks={ticks};alpha={:016x};edge={value};x={:.3};count={};focus={}", source[2].velocity[0].to_bits(), source[2].velocity[1].to_bits(), alpha.to_bits(), position[0], count.get(), focus.get())
        }))).on(EventKind::Activate, move |_| count.revise(|v| *v += 1)).on(EventKind::FocusChanged, move |e| if let Event::FocusChanged(v) = e { focus.revise(|f| *f = v); }).show_in(scope)?;
        if fail {
            return Err(glory::render::Error::InvalidProps(
                "independent link child build rejected",
            )
            .into());
        }
        Ok(())
    }))
}

#[test]
fn mixed_path_midpoint_and_node_children_retain_identity_bits_count_focus_and_atomic_rejection() {
    let mut owned = simulation();
    let initial = source(&owned, 360., 8);
    let mut metadata = owned.clone();
    metadata.set_velocity("other", [0., -0.]).unwrap();
    let next = source(&metadata, 360., 8);
    assert_eq!(initial.0, next.0);
    assert_eq!(
        render::to_svg(&initial.0.scene).unwrap(),
        render::to_svg(&next.0.scene).unwrap()
    );
    assert_eq!(initial.1.widgets.layout, next.1.widgets.layout);
    assert_ne!(initial.1.widgets, next.1.widgets);
    let app = App::memory();
    let input = app.scope().cage(initial);
    let frame = app.scope().bond(move || input.with(|s| s.0.clone()));
    let packet = app.scope().bond(move || input.with(|s| s.1.clone()));
    let calls = Rc::new(Cell::new(0));
    let counted = calls.clone();
    let errors = Rc::new(RefCell::new(vec![]));
    let failed = errors.clone();
    app.mount(
        chart_layout(frame)
            .legend(false)
            .tooltip(false)
            .on_reject(move |e| failed.borrow_mut().push(e.0))
            .plot_widgets_composed(packet, move |c| child(c, counted.clone())),
    )
    .unwrap();
    let link = nodes(&app, "independent-link-child")[0];
    let node = nodes(&app, "independent-node-child")[0];
    let controls = nodes(&app, "kartis-datum");
    assert_eq!(controls.len(), 2, "only existing nonempty node controls");
    assert_eq!(calls.get(), 2);
    app.dispatch(link, Event::Activate).unwrap();
    app.dispatch(link, Event::FocusChanged(true)).unwrap();
    assert!(text(&app, link).contains("other=8000000000000000,0000000000000000"));
    assert!(text(&app, link).ends_with("count=1;focus=true"));
    input.revise(|old| *old = next);
    app.flush().unwrap();
    assert!(text(&app, link).contains("other=0000000000000000,8000000000000000"));
    assert!(text(&app, node).contains("other=0000000000000000,8000000000000000"));
    assert_eq!(nodes(&app, "independent-link-child"), [link]);
    assert_eq!(nodes(&app, "independent-node-child"), [node]);
    assert_eq!(calls.get(), 2);
    owned = metadata;
    owned.set_velocity("a", [2., 0.]).unwrap();
    owned.step(1).unwrap();
    input.revise(|old| *old = source(&owned, 380., 8));
    app.flush().unwrap();
    assert!(text(&app, link).contains("ticks=1;"));
    assert!(text(&app, link).ends_with("count=1;focus=true"));
    assert_eq!(nodes(&app, "kartis-datum"), controls);
    let previous = [text(&app, link), text(&app, node)];
    let size = app.stats().nodes;
    for revision in [90, 91] {
        let mut candidate = owned.clone();
        candidate.set_position("a", [60., 10.]).unwrap();
        input.revise(|old| *old = source(&candidate, 400., revision));
        app.flush().unwrap();
        assert_eq!([text(&app, link), text(&app, node)], previous);
        assert_eq!(nodes(&app, "independent-link-child"), [link]);
        assert_eq!(nodes(&app, "independent-node-child"), [node]);
        assert_eq!(nodes(&app, "kartis-datum"), controls);
        assert_eq!(app.stats().nodes, size);
    }
    let mut bad = source(&owned, 400., 8);
    bad.1.widgets.data.remove("force-link:0");
    input.revise(|old| *old = bad);
    app.flush().unwrap();
    assert_eq!([text(&app, link), text(&app, node)], previous);
    assert_eq!(errors.borrow().len(), 3);
    input.revise(|old| *old = source(&owned, 410., 8));
    app.flush().unwrap();
    assert_eq!(nodes(&app, "independent-link-child"), [link]);
    assert!(text(&app, link).ends_with("count=1;focus=true"));
    app.clear().unwrap();
    assert_eq!(app.stats().nodes, 0);
}

#[cfg(feature = "raster")]
fn image(app: &App) -> String {
    let tree = app.tree();
    let id = nodes(app, "kartis-raster-image")[0];
    let Props::Image(p) = tree.node(id).unwrap().descriptor.view().props else {
        panic!("actual Image expected")
    };
    p.source.clone()
}
#[cfg(feature = "raster")]
fn pixel(uri: &str, p: [usize; 2]) -> [u8; 4] {
    use base64::Engine;
    let b = base64::engine::general_purpose::STANDARD
        .decode(uri.split_once(',').unwrap().1)
        .unwrap();
    let mut r = png::Decoder::new(std::io::Cursor::new(b))
        .read_info()
        .unwrap();
    let mut bytes = vec![0; r.output_buffer_size().unwrap()];
    let info = r.next_frame(&mut bytes).unwrap();
    let offset = 4 * (p[1] * info.width as usize + p[0]);
    bytes[offset..offset + 4].try_into().unwrap()
}
#[cfg(feature = "raster")]
#[test]
fn both_dpr_cpu_publish_failures_keep_real_png_node_and_link_payloads_then_recover_together() {
    use kartis::render::raster::{FontRegistry, RasterOptions};
    for dpr in [1., 2.] {
        let owned = simulation();
        let initial = source(&owned, 360., 8);
        let other = initial
            .0
            .interaction
            .as_ref()
            .unwrap()
            .sites
            .iter()
            .find(|s| s.mark_id == "force-node:other")
            .unwrap()
            .position;
        let app = App::memory();
        let input = app.scope().cage(initial);
        let frame = app.scope().bond(move || input.with(|s| s.0.clone()));
        let packet = app.scope().bond(move || input.with(|s| s.1.clone()));
        let calls = Rc::new(Cell::new(0));
        let counted = calls.clone();
        let deny = Rc::new(Cell::new(false));
        let rejected = deny.clone();
        let publications = Rc::new(Cell::new(0));
        let published = publications.clone();
        let errors = Rc::new(RefCell::new(vec![]));
        let failed = errors.clone();
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
                    published.set(published.get() + 1);
                    if rejected.get() {
                        Err(ChartError(
                            "independent mixed PNG publication denied".into(),
                        ))
                    } else {
                        image.data_url()
                    }
                })
                .on_reject(move |e| failed.borrow_mut().push(e.0))
                .plot_widgets_composed(packet, move |c| child(c, counted.clone())),
        )
        .unwrap();
        let link = nodes(&app, "independent-link-child")[0];
        let node = nodes(&app, "independent-node-child")[0];
        let controls = nodes(&app, "kartis-datum");
        app.dispatch(link, Event::Activate).unwrap();
        app.dispatch(link, Event::FocusChanged(true)).unwrap();
        let previous = [text(&app, link), text(&app, node)];
        let png = image(&app);
        assert_eq!(
            pixel(&png, [(other[0] * dpr) as usize, (other[1] * dpr) as usize]),
            [180, 70, 90, 255]
        );
        let mut candidate = owned.clone();
        candidate.set_velocity("a", [2., 0.]).unwrap();
        candidate.step(1).unwrap();
        let count = publications.get();
        deny.set(true);
        input.revise(|old| *old = source(&candidate, 400., 8));
        app.flush().unwrap();
        assert_eq!(publications.get(), count + 1);
        assert_eq!(errors.borrow().len(), 1);
        assert_eq!(image(&app), png);
        assert_eq!([text(&app, link), text(&app, node)], previous);
        assert_eq!(nodes(&app, "independent-link-child"), [link]);
        assert_eq!(nodes(&app, "independent-node-child"), [node]);
        assert_eq!(nodes(&app, "kartis-datum"), controls);
        assert_eq!(owned.total_ticks(), 0);
        deny.set(false);
        input.revise(|old| *old = source(&candidate, 410., 8));
        app.flush().unwrap();
        assert_ne!(image(&app), png);
        assert!(text(&app, link).contains("ticks=1;"));
        assert!(text(&app, node).contains("ticks=1;"));
        assert!(text(&app, link).ends_with("count=1;focus=true"));
        assert_eq!(calls.get(), 2);
        assert_eq!(nodes(&app, "kartis-datum"), controls);
        app.clear().unwrap();
        assert_eq!(app.stats().nodes, 0);
    }
}
