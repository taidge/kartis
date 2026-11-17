//! Actual retained Memory children and CPU-image publication transactions.
//! Caller-owned force proposals are rebuilt explicitly; no browser/OS claim.
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use glory::render::{Event, EventKind, NodeId, Props};
use glory::{App, Widget};
use kartis::*;

fn config(width: f64) -> ChartConfig {
    ChartConfig {
        width,
        height: 230.,
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
                id: "selected".into(),
                label: String::new(),
                radius: 3.,
            },
            NetworkNode {
                id: "other".into(),
                label: "Other".into(),
                radius: 7.,
            },
        ],
        &[],
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
    s.set_position("selected", [-0., 10.]).unwrap();
    s.set_position("other", [50., 15.]).unwrap();
    s.set_velocity("selected", [-0., 0.]).unwrap();
    s
}
fn glyphs() -> NetworkGlyphOptions {
    let mut g = NetworkGlyphOptions::default();
    g.nodes.insert(
        "selected".into(),
        NetworkNodeGlyph {
            glyph: GlyphOptions {
                shape: GlyphShape::Filled(FilledGlyph::Triangle),
                size: 128.,
                rotation: 0.,
            },
            fill: Some(Color(10, 100, 180, 255)),
            stroke: None,
        },
    );
    g.nodes.insert(
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
    g
}
fn source(
    s: &ForceSimulation,
    width: f64,
    revision: u64,
) -> (
    SceneInteraction,
    PlotWidgetsComposition<NetworkNodeWidgetData>,
) {
    let layout = network_glyph_layout(&config(width), s, &glyphs()).unwrap();
    let widgets = network_node_widgets(&layout, s, revision, |c| {
        Ok(
            (c.node.id == "selected").then_some(PlotWidgetPlacement::AtDatum {
                offset: [-6., -4.],
                measured: TextSize {
                    width: 12.,
                    height: 8.,
                },
            }),
        )
    })
    .unwrap();
    let replace_ink = widgets.data.keys().cloned().collect();
    (
        network_widget_frame(&layout).unwrap(),
        PlotWidgetsComposition {
            widgets,
            replace_ink,
        },
    )
}
fn nodes(app: &App, class: &str) -> Vec<NodeId> {
    let tree = app.tree();
    let mut q = tree.roots().to_vec();
    let mut out = vec![];
    while let Some(id) = q.pop() {
        let n = tree.node(id).unwrap();
        if n.descriptor.view().classes.iter().any(|c| c == class) {
            out.push(id);
        }
        q.extend(n.children.iter().copied());
    }
    out
}
fn text(app: &App, id: NodeId) -> String {
    let tree = app.tree();
    let Props::Button(p) = tree.node(id).unwrap().descriptor.view().props else {
        panic!("actual Button expected")
    };
    p.label.clone()
}
fn child(
    c: PlotWidgetCompositionContext<NetworkNodeWidgetData>,
    calls: Rc<Cell<usize>>,
) -> Result<impl Widget> {
    calls.set(calls.get() + 1);
    if c.revision == 90 {
        return Err(ChartError("network child factory rejected".into()));
    }
    let fail = c.revision == 91;
    let frame = c.frame;
    Ok(glory::ui::widget(move |scope| {
        let count = scope.cage(0);
        let focus = scope.cage(false);
        glory::ui::button().class("network-owned-child").label(scope.bond(move||frame.with(|f|{let d=&f.data;let n=&d.source[d.source_index];assert!((f.frame.rect.x+6.-d.site.position[0]).abs()<1e-10);assert!((f.frame.rect.y+4.-d.site.position[1]).abs()<1e-10);assert_eq!(d.site.world_position,n.position);let fixed=n.fixed.map(|p|format!("{:016x},{:016x}",p[0].to_bits(),p[1].to_bits())).unwrap_or_else(||"None".into());format!("velocity={:016x},{:016x};fixed={fixed};ticks={};alpha={:016x};empty={};count={};focus={}",n.velocity[0].to_bits(),n.velocity[1].to_bits(),d.ticks,d.alpha.to_bits(),n.label.is_empty(),count.get(),focus.get())}))).on(EventKind::Activate,move|_|count.revise(|v|*v+=1)).on(EventKind::FocusChanged,move|e|if let Event::FocusChanged(v)=e{focus.revise(|f|*f=v);}).show_in(scope)?;
        if fail {
            return Err(glory::render::Error::InvalidProps("network child build rejected").into());
        }
        Ok(())
    }))
}

#[test]
fn force_snapshots_update_retained_empty_label_path_child_and_preserve_real_controls() {
    let mut owned = simulation();
    let initial = source(&owned, 360., 8);
    assert_eq!(initial.0.interaction.as_ref().unwrap().sites.len(), 1);
    assert_eq!(
        initial.0.interaction.as_ref().unwrap().sites[0].mark_id,
        "force-node:other"
    );
    let snapshot = initial.1.widgets.data["force-node:selected"].source.clone();
    let mut metadata = owned.clone();
    metadata.set_velocity("selected", [0., -0.]).unwrap();
    let next = source(&metadata, 360., 8);
    assert_eq!(initial.0, next.0);
    assert_eq!(
        render::to_svg(&initial.0.scene).unwrap(),
        render::to_svg(&next.0.scene).unwrap()
    );
    assert_eq!(snapshot[0].velocity[0].to_bits(), (-0_f64).to_bits());
    assert_ne!(initial.1.widgets, next.1.widgets);
    let app = App::memory();
    let input = app.scope().cage(initial);
    let frame = app.scope().bond(move || input.with(|s| s.0.clone()));
    let slots = app.scope().bond(move || input.with(|s| s.1.clone()));
    let calls = Rc::new(Cell::new(0));
    let called = calls.clone();
    let errors = Rc::new(RefCell::new(vec![]));
    let error = errors.clone();
    app.mount(
        chart_layout(frame)
            .legend(false)
            .tooltip(false)
            .on_reject(move |e| error.borrow_mut().push(e.0))
            .plot_widgets_composed(slots, move |c| child(c, called.clone())),
    )
    .unwrap();
    let id = nodes(&app, "network-owned-child")[0];
    let controls = nodes(&app, "kartis-datum");
    assert_eq!(controls.len(), 1);
    app.dispatch(id, Event::Activate).unwrap();
    app.dispatch(id, Event::FocusChanged(true)).unwrap();
    assert!(
        text(&app, id)
            .starts_with("velocity=8000000000000000,0000000000000000;fixed=None;ticks=0;")
    );
    assert!(text(&app, id).ends_with("empty=true;count=1;focus=true"));
    input.revise(|s| *s = next);
    app.flush().unwrap();
    owned = metadata;
    assert!(text(&app, id).starts_with("velocity=0000000000000000,8000000000000000;fixed=None;"));
    assert_eq!(nodes(&app, "network-owned-child"), [id]);
    assert_eq!(nodes(&app, "kartis-datum"), controls);
    owned.set_fixed("selected", Some([-0., 10.])).unwrap();
    input.revise(|s| *s = source(&owned, 360., 8));
    app.flush().unwrap();
    assert!(text(&app, id).contains("fixed=8000000000000000,4024000000000000;"));
    owned.set_fixed("selected", None).unwrap();
    owned.set_velocity("selected", [2., 0.]).unwrap();
    owned.step(1).unwrap();
    assert_eq!(owned.total_ticks(), 1);
    input.revise(|s| *s = source(&owned, 380., 8));
    app.flush().unwrap();
    assert!(text(&app, id).contains("fixed=None;ticks=1;"));
    assert_eq!(nodes(&app, "network-owned-child"), [id]);
    assert_eq!(calls.get(), 1);
    let old = text(&app, id);
    let n = app.stats().nodes;
    for revision in [90, 91] {
        let mut candidate = owned.clone();
        candidate.set_position("selected", [70., 20.]).unwrap();
        input.revise(|s| *s = source(&candidate, 400., revision));
        app.flush().unwrap();
        assert_eq!(text(&app, id), old);
        assert_eq!(nodes(&app, "network-owned-child"), [id]);
        assert_eq!(nodes(&app, "kartis-datum"), controls);
        assert_eq!(app.stats().nodes, n);
    }
    assert_eq!(errors.borrow().len(), 2);
    let before_alpha = source(&owned, 380., 8);
    owned.reheat(1.).unwrap();
    let after_alpha = source(&owned, 380., 8);
    assert_eq!(before_alpha.0, after_alpha.0);
    assert_eq!(before_alpha.1.widgets.layout, after_alpha.1.widgets.layout);
    assert_ne!(before_alpha.1.widgets, after_alpha.1.widgets);
    input.revise(|s| *s = after_alpha);
    app.flush().unwrap();
    assert!(text(&app, id).contains("alpha=3ff0000000000000;"));
    assert!(text(&app, id).ends_with("count=1;focus=true"));
    assert_eq!(nodes(&app, "network-owned-child"), [id]);
    app.clear().unwrap();
    assert_eq!(app.stats().nodes, 0);
}

#[cfg(feature = "raster")]
fn image(app: &App) -> String {
    let tree = app.tree();
    let ids = nodes(app, "kartis-raster-image");
    assert_eq!(ids.len(), 1);
    let Props::Image(p) = tree.node(ids[0]).unwrap().descriptor.view().props else {
        panic!("actual Image expected")
    };
    p.source.clone()
}
#[cfg(feature = "raster")]
fn pixel(uri: &str, p: [usize; 2]) -> [u8; 4] {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(uri.split_once(',').unwrap().1)
        .unwrap();
    let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
        .read_info()
        .unwrap();
    let mut data = vec![0; reader.output_buffer_size().unwrap()];
    let i = reader.next_frame(&mut data).unwrap();
    let offset = 4 * (p[1] * i.width as usize + p[0]);
    data[offset..offset + 4].try_into().unwrap()
}
#[cfg(feature = "raster")]
#[test]
fn cpu_publication_refusal_keeps_accepted_force_snapshot_image_and_child_at_both_dprs() {
    use kartis::render::raster::{FontRegistry, RasterOptions};
    for dpr in [1., 2.] {
        let owned = simulation();
        let initial = source(&owned, 360., 8);
        let other = initial.0.interaction.as_ref().unwrap().sites[0].position;
        let app = App::memory();
        let input = app.scope().cage(initial);
        let frame = app.scope().bond(move || input.with(|s| s.0.clone()));
        let slots = app.scope().bond(move || input.with(|s| s.1.clone()));
        let calls = Rc::new(Cell::new(0));
        let called = calls.clone();
        let deny = Rc::new(Cell::new(false));
        let reject = deny.clone();
        let publications = Rc::new(Cell::new(0));
        let publish = publications.clone();
        let errors = Rc::new(RefCell::new(vec![]));
        let error = errors.clone();
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
                    publish.set(publish.get() + 1);
                    if reject.get() {
                        Err(ChartError("network PNG publisher denied".into()))
                    } else {
                        image.data_url()
                    }
                })
                .on_reject(move |e| error.borrow_mut().push(e.0))
                .plot_widgets_composed(slots, move |c| child(c, called.clone())),
        )
        .unwrap();
        let id = nodes(&app, "network-owned-child")[0];
        let controls = nodes(&app, "kartis-datum");
        app.dispatch(id, Event::Activate).unwrap();
        app.dispatch(id, Event::FocusChanged(true)).unwrap();
        let old = text(&app, id);
        let png = image(&app);
        assert_eq!(
            pixel(&png, [(other[0] * dpr) as usize, (other[1] * dpr) as usize]),
            [180, 70, 90, 255]
        );
        let mut candidate = owned.clone();
        candidate.set_velocity("selected", [2., 0.]).unwrap();
        candidate.step(1).unwrap();
        let count = publications.get();
        deny.set(true);
        input.revise(|s| *s = source(&candidate, 400., 8));
        app.flush().unwrap();
        assert_eq!(publications.get(), count + 1);
        assert_eq!(errors.borrow().len(), 1);
        assert_eq!(image(&app), png);
        assert_eq!(text(&app, id), old);
        assert_eq!(nodes(&app, "network-owned-child"), [id]);
        assert_eq!(nodes(&app, "kartis-datum"), controls);
        assert_eq!(owned.total_ticks(), 0, "caller proposal was not committed");
        deny.set(false);
        input.revise(|s| *s = source(&candidate, 410., 8));
        app.flush().unwrap();
        assert_ne!(image(&app), png);
        assert!(text(&app, id).contains("fixed=None;ticks=1;"));
        assert!(text(&app, id).ends_with("empty=true;count=1;focus=true"));
        assert_eq!(nodes(&app, "network-owned-child"), [id]);
        assert_eq!(nodes(&app, "kartis-datum"), controls);
        assert_eq!(calls.get(), 1);
        app.clear().unwrap();
        assert_eq!(app.stats().nodes, 0);
    }
}
