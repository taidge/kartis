//! Retained Glory Memory/CPU-image acceptance, independent of browser fixtures.
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use glory::render::{Event, EventKind, NodeId, Props};
use glory::{App, Widget};
use kartis::*;

fn config(width: f64) -> ChartConfig {
    ChartConfig {
        width,
        height: 220.,
        margin: Margin {
            left: 28.,
            right: 24.,
            top: 24.,
            bottom: 24.,
        },
        ..Default::default()
    }
}
fn source(
    phase: u8,
    revision: u64,
    width: f64,
) -> (
    SceneInteraction,
    PlotWidgetsComposition<SwarmNodeWidgetData>,
) {
    let mut selected = SwarmPoint::new("selected", "A", if phase == 3 { 2. } else { 1. });
    selected.label = Some(String::new());
    let mut hidden = SwarmPoint::new("hidden", "H", if phase == 0 { -0. } else { 0. });
    hidden.size = Some(if phase == 0 { -0. } else { 0. });
    let mut other = SwarmPoint::new("other", "B", 4.);
    other.color = Some(Color(180, 70, 90, 255));
    let mut raw = vec![selected, hidden, other];
    if phase == 2 {
        raw.rotate_left(1);
    }
    let o = SwarmLayoutOptions {
        orientation: SwarmOrientation::Horizontal,
        groups: Some(vec![
            SwarmGroup::new("A"),
            SwarmGroup::new("H"),
            SwarmGroup::new("B"),
        ]),
        hidden: ["H".into()].into(),
        value_domain: Some([0., 5.]),
        value_scale: ContinuousScaleOptions {
            nice: ScaleNice::Off,
            ..Default::default()
        },
        value_axis: None,
        group_axis: None,
        ..Default::default()
    };
    let layout = if phase == 4 {
        swarm_layout_with_node_factory(&config(width), &raw, &o, |c| {
            if c.datum.id != "selected" {
                return Ok(c.inherit(c.geometry.clone()));
            }
            use glory::render::{GraphicPoint as P, PathCommand as C};
            let [x, y] = c.position;
            Ok(c.inherit(Geometry::Path(vec![
                C::Move(P::new(x + 12., y + 8.)),
                C::Line(P::new(x + 20., y + 8.)),
                C::Line(P::new(x + 20., y + 16.)),
                C::Line(P::new(x + 12., y + 16.)),
                C::Close,
            ])))
        })
        .unwrap()
    } else {
        swarm_layout(&config(width), &raw, &o).unwrap()
    };
    let widgets = swarm_node_widgets(&layout, &raw, revision, |c| {
        Ok(
            (c.datum.id == "selected").then_some(PlotWidgetPlacement::AtDatum {
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
        swarm_widget_frame(&layout).unwrap(),
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
fn label(app: &App, id: NodeId) -> String {
    let tree = app.tree();
    let Props::Button(p) = tree.node(id).unwrap().descriptor.view().props else {
        panic!("real Button required")
    };
    p.label.clone()
}
fn child(
    c: PlotWidgetCompositionContext<SwarmNodeWidgetData>,
    calls: Rc<Cell<usize>>,
) -> Result<impl Widget> {
    calls.set(calls.get() + 1);
    if c.revision == 90 {
        return Err(ChartError("audit factory refusal".into()));
    }
    let fail = c.revision == 91;
    let frame = c.frame;
    Ok(glory::ui::widget(move |scope| {
        let count = scope.cage(0);
        let focus = scope.cage(false);
        glory::ui::button()
            .class("swarm-owned-child")
            .label(scope.bond(move || {
                frame.with(|f| {
                    assert!((f.frame.rect.x + 6. - f.data.site.position[0]).abs() < 1e-10);
                    assert!((f.frame.rect.y + 4. - f.data.site.position[1]).abs() < 1e-10);
                    let h = f.data.source.iter().find(|p| p.id == "hidden").unwrap();
                    format!(
                        "index={};raw={:016x}/{:016x};value={};count={};focus={};empty={};path={}",
                        f.data.source_index,
                        h.value.to_bits(),
                        h.size.unwrap().to_bits(),
                        f.data.site.raw_value,
                        count.get(),
                        focus.get(),
                        f.data.source[f.data.source_index].label.as_deref() == Some(""),
                        matches!(f.frame.mark.geometry, Geometry::Path(_))
                    )
                })
            }))
            .on(EventKind::Activate, move |_| count.revise(|v| *v += 1))
            .on(EventKind::FocusChanged, move |e| {
                if let Event::FocusChanged(v) = e {
                    focus.revise(|f| *f = v);
                }
            })
            .show_in(scope)?;
        if fail {
            return Err(glory::render::Error::InvalidProps("audit child build refusal").into());
        }
        Ok(())
    }))
}

#[test]
fn actual_retained_child_reads_raw_bits_reorder_path_anchors_and_atomic_rejections() {
    let before = source(0, 7, 340.);
    let bit = source(1, 7, 340.);
    let reordered = source(2, 7, 340.);
    assert_eq!(before.0, bit.0);
    assert_eq!(before.0, reordered.0);
    assert_eq!(
        render::to_svg(&before.0.scene).unwrap(),
        render::to_svg(&bit.0.scene).unwrap()
    );
    assert_eq!(before.1.widgets.layout, bit.1.widgets.layout);
    assert_ne!(before.1.widgets, bit.1.widgets);
    let app = App::memory();
    let input = app.scope().cage(before);
    let frame = app.scope().bond(move || input.with(|s| s.0.clone()));
    let slots = app.scope().bond(move || input.with(|s| s.1.clone()));
    let calls = Rc::new(Cell::new(0));
    let called = calls.clone();
    let errors = Rc::new(RefCell::new(vec![]));
    let err = errors.clone();
    app.mount(
        chart_layout(frame)
            .legend(false)
            .data_controls(false)
            .tooltip(false)
            .on_reject(move |e| err.borrow_mut().push(e.0))
            .plot_widgets_composed(slots, move |c| child(c, called.clone())),
    )
    .unwrap();
    let id = nodes(&app, "swarm-owned-child")[0];
    app.dispatch(id, Event::Activate).unwrap();
    app.dispatch(id, Event::FocusChanged(true)).unwrap();
    assert!(label(&app, id).contains(
        "raw=8000000000000000/8000000000000000;value=1;count=1;focus=true;empty=true;path=false"
    ));
    input.revise(|s| *s = bit);
    app.flush().unwrap();
    assert_eq!(nodes(&app, "swarm-owned-child"), [id]);
    assert!(
        label(&app, id)
            .contains("raw=0000000000000000/0000000000000000;value=1;count=1;focus=true")
    );
    input.revise(|s| *s = reordered);
    app.flush().unwrap();
    assert!(label(&app, id).starts_with("index=2;"));
    assert_eq!(nodes(&app, "swarm-owned-child"), [id]);
    input.revise(|s| *s = source(4, 7, 360.));
    app.flush().unwrap();
    assert_eq!(nodes(&app, "swarm-owned-child"), [id]);
    assert!(label(&app, id).ends_with("count=1;focus=true;empty=true;path=true"));
    assert_eq!(calls.get(), 1);
    for revision in [90, 91] {
        let old = label(&app, id);
        let n = app.stats().nodes;
        input.revise(|s| *s = source(0, revision, 400.));
        app.flush().unwrap();
        assert_eq!(nodes(&app, "swarm-owned-child"), [id]);
        assert_eq!(label(&app, id), old);
        assert_eq!(app.stats().nodes, n);
    }
    assert_eq!(errors.borrow().len(), 2);
    input.revise(|s| *s = source(3, 7, 380.));
    app.flush().unwrap();
    assert_eq!(nodes(&app, "swarm-owned-child"), [id]);
    assert!(label(&app, id).contains("value=2;count=1;focus=true"));
    app.clear().unwrap();
    assert_eq!(app.stats().nodes, 0);
}

#[cfg(feature = "raster")]
fn image(app: &App) -> String {
    let ids = nodes(app, "kartis-raster-image");
    assert_eq!(ids.len(), 1);
    let tree = app.tree();
    let Props::Image(p) = tree.node(ids[0]).unwrap().descriptor.view().props else {
        panic!("actual Image required")
    };
    p.source.clone()
}
#[cfg(feature = "raster")]
fn pixel(uri: &str, x: usize, y: usize) -> [u8; 4] {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(uri.split_once(',').unwrap().1)
        .unwrap();
    let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
        .read_info()
        .unwrap();
    let mut out = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut out).unwrap();
    out[4 * (y * info.width as usize + x)..4 * (y * info.width as usize + x) + 4]
        .try_into()
        .unwrap()
}
#[cfg(feature = "raster")]
#[test]
fn cpu_dpr_publisher_refusal_preserves_real_image_and_typed_source_then_recovers() {
    use kartis::render::raster::{FontRegistry, RasterOptions};
    for dpr in [1., 2.] {
        let app = App::memory();
        let initial = source(0, 7, 340.);
        let other = initial
            .0
            .interaction
            .as_ref()
            .unwrap()
            .sites
            .iter()
            .find(|s| s.series == "B")
            .unwrap()
            .position;
        let input = app.scope().cage(initial);
        let frame = app.scope().bond(move || input.with(|s| s.0.clone()));
        let slots = app.scope().bond(move || input.with(|s| s.1.clone()));
        let deny = Rc::new(Cell::new(false));
        let denied = deny.clone();
        let publish = Rc::new(Cell::new(0));
        let publications = publish.clone();
        let errors = Rc::new(RefCell::new(vec![]));
        let err = errors.clone();
        let calls = Rc::new(Cell::new(0));
        let called = calls.clone();
        app.mount(
            chart_layout(frame)
                .legend(false)
                .data_controls(false)
                .tooltip(false)
                .raster(
                    FontRegistry::new(),
                    RasterOptions {
                        pixel_ratio: dpr,
                        ..Default::default()
                    },
                )
                .raster_image_source(move |image| {
                    publications.set(publications.get() + 1);
                    if denied.get() {
                        Err(ChartError("independent Swarm publisher denied".into()))
                    } else {
                        image.data_url()
                    }
                })
                .on_reject(move |e| err.borrow_mut().push(e.0))
                .plot_widgets_composed(slots, move |c| child(c, called.clone())),
        )
        .unwrap();
        let id = nodes(&app, "swarm-owned-child")[0];
        app.dispatch(id, Event::Activate).unwrap();
        app.dispatch(id, Event::FocusChanged(true)).unwrap();
        let old = label(&app, id);
        let png = image(&app);
        assert_eq!(
            pixel(&png, (other[0] * dpr) as usize, (other[1] * dpr) as usize),
            [180, 70, 90, 255]
        );
        let n = app.stats().nodes;
        let count = publish.get();
        deny.set(true);
        input.revise(|s| *s = source(1, 7, 400.));
        app.flush().unwrap();
        assert_eq!(publish.get(), count + 1);
        assert_eq!(errors.borrow().len(), 1);
        assert_eq!(image(&app), png);
        assert_eq!(label(&app, id), old);
        assert_eq!(nodes(&app, "swarm-owned-child"), [id]);
        assert_eq!(app.stats().nodes, n);
        deny.set(false);
        input.revise(|s| *s = source(1, 7, 410.));
        app.flush().unwrap();
        assert_ne!(image(&app), png);
        assert!(
            label(&app, id)
                .contains("raw=0000000000000000/0000000000000000;value=1;count=1;focus=true")
        );
        assert_eq!(nodes(&app, "swarm-owned-child"), [id]);
        assert_eq!(calls.get(), 1);
        app.clear().unwrap();
        assert_eq!(app.stats().nodes, 0);
    }
}
