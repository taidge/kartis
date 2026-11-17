//! Actual function-resolved Stream frames on Memory and CPU image hosts.
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use glory::render::{Event, EventKind, GraphicPoint as P, NodeId, PathCommand as C, Props};
use glory::{App, Widget};
use kartis::stack::{StackOffset, StackOptions};
use kartis::*;

fn source(
    phase: u8,
    width: f64,
    revision: u64,
) -> (
    SceneInteraction,
    PlotWidgetsComposition<StreamDotWidgetData>,
) {
    let c = ChartConfig {
        width,
        height: 240.,
        margin: Margin {
            left: 31.,
            right: 23.,
            top: 25.,
            bottom: 21.,
        },
        ..Default::default()
    };
    let raw = [
        BarSeries::new(
            "a",
            vec![
                Some(if phase == 3 { 6. } else { 4. }),
                None,
                Some(if phase == 0 { -0. } else { 0. }),
            ],
        ),
        BarSeries::new("b", vec![Some(2.), Some(3.), Some(5.)]),
    ];
    let mut o = StreamLayoutOptions {
        stack: StackOptions {
            offset: StackOffset::Zero,
            ..Default::default()
        },
        curve: Curve::Linear,
        dots: Some(StreamDotOptions {
            size: 12.,
            ..Default::default()
        }),
        ..Default::default()
    };
    o.axes.x.visible = false;
    o.axes.x.grid = false;
    o.axes.y.visible = false;
    o.axes.y.grid = false;
    let mut label = |s: &StreamLayerPathContext<'_>| Ok(format!("Resolved-{}", s.id));
    let mut color = |_: &StreamLayerPathContext<'_>| Ok(Color(50, 110, 100, 255));
    let mut fmt = |_: f64| Ok("MMMM".into());
    let mut dot = |ctx: &StreamDotStyleContext<'_>| {
        Ok(if ctx.series.id == "b" {
            Color(180, 70, 90, 255)
        } else {
            Color(20, 90, 140, 255)
        })
    };
    let mut layout = stream_layout_with_factories_and_dot_factory(
        &c,
        &["one".into(), "two".into(), "three".into()],
        &raw,
        &o,
        &mut StreamFactories {
            label: Some(&mut label),
            color: Some(&mut color),
            formatted_value: Some(&mut fmt),
            dot_color: Some(&mut dot),
            ..Default::default()
        },
        |ctx| {
            if ctx.series.id == "a" && ctx.sample.source_index == 0 {
                let [x, y] = ctx.position;
                Ok(ctx.inherit(Geometry::Path(vec![
                    C::Move(P::new(x + 12., y - 3.)),
                    C::Line(P::new(x + 20., y - 3.)),
                    C::Line(P::new(x + 20., y + 3.)),
                    C::Close,
                ])))
            } else {
                Ok(ctx.inherit(Geometry::Circle {
                    x: ctx.position[0],
                    y: ctx.position[1],
                    radius: 6.,
                }))
            }
        },
    )
    .unwrap();
    for mark in &mut layout.scene.marks {
        if mark.value == 0. {
            mark.value = 0.;
        }
    }
    let widgets = stream_dot_widgets(&layout, revision, |ctx| {
        Ok(
            (ctx.series.id == "a" && ctx.sample.source_index == 0).then_some(
                PlotWidgetPlacement::AtDatum {
                    offset: [1., -4.],
                    measured: TextSize {
                        width: 12.,
                        height: 8.,
                    },
                },
            ),
        )
    })
    .unwrap();
    let mut frame = layout.into_scene_interaction().unwrap();
    frame.interaction.as_mut().unwrap().picking_profile = PickingProfile::PaintedGeometry;
    frame.validate().unwrap();
    (
        frame,
        PlotWidgetsComposition {
            replace_ink: widgets.data.keys().cloned().collect(),
            widgets,
        },
    )
}
fn nodes(app: &App, class: &str) -> Vec<NodeId> {
    let tree = app.tree();
    let mut todo = tree.roots().to_vec();
    let mut result = vec![];
    while let Some(id) = todo.pop() {
        let n = tree.node(id).unwrap();
        if n.descriptor.view().classes.iter().any(|c| c == class) {
            result.push(id);
        }
        todo.extend(n.children.iter().copied());
    }
    result
}
fn text(app: &App, id: NodeId) -> String {
    let tree = app.tree();
    let Props::Button(p) = tree.node(id).unwrap().descriptor.view().props else {
        panic!("realButton")
    };
    p.label.clone()
}
fn child(
    ctx: PlotWidgetCompositionContext<StreamDotWidgetData>,
    calls: Rc<Cell<usize>>,
) -> Result<impl Widget> {
    calls.set(calls.get() + 1);
    if ctx.revision == 90 {
        return Err(ChartError(
            "independent function child factory denied".into(),
        ));
    }
    let reject = ctx.revision == 91;
    let frame = ctx.frame;
    Ok(glory::ui::widget(move |scope| {
        let count = scope.cage(0u32);
        let focus = scope.cage(false);
        glory::ui::button()
            .class("stream-function-child")
            .label(scope.bond(move || {
                frame.with(|f| {
                    assert_eq!(f.data.layer_label, "Resolved-a");
                    assert_eq!(f.data.sample.formatted_value.as_deref(), Some("MMMM"));
                    assert!((f.frame.rect.x - 1. - f.data.sample.position[0]).abs() < 1e-9);
                    format!(
                        "raw={:016x};value={};label={};format={};count={};focus={}",
                        f.data.series.values[2].unwrap().to_bits(),
                        f.data.sample.value.unwrap(),
                        f.data.layer_label,
                        f.data.sample.formatted_value.as_deref().unwrap(),
                        count.get(),
                        focus.get()
                    )
                })
            }))
            .on(EventKind::Activate, move |_| count.revise(|c| *c += 1))
            .on(EventKind::FocusChanged, move |e| {
                if let Event::FocusChanged(v) = e {
                    focus.revise(|f| *f = v);
                }
            })
            .show_in(scope)?;
        if reject {
            return Err(glory::render::Error::InvalidProps(
                "independent function child build denied",
            )
            .into());
        }
        Ok(())
    }))
}

#[test]
fn function_resolved_typed_path_child_retains_id_focus_count_and_atomic_source_updates() {
    let old = source(0, 360., 7);
    let next = source(1, 360., 7);
    assert_eq!(old.0, next.0);
    assert_eq!(
        render::to_svg(&old.0.scene).unwrap(),
        render::to_svg(&next.0.scene).unwrap()
    );
    assert_eq!(old.1.widgets.layout, next.1.widgets.layout);
    assert_ne!(old.1.widgets, next.1.widgets);
    let app = App::memory();
    let input = app.scope().cage(old);
    let frame = app.scope().bond(move || input.with(|s| s.0.clone()));
    let slots = app.scope().bond(move || input.with(|s| s.1.clone()));
    let calls = Rc::new(Cell::new(0));
    let called = calls.clone();
    let errors = Rc::new(RefCell::new(vec![]));
    let failed = errors.clone();
    app.mount(
        chart_layout(frame)
            .legend(false)
            .tooltip(false)
            .on_reject(move |e| failed.borrow_mut().push(e.0))
            .plot_widgets_composed(slots, move |c| child(c, called.clone())),
    )
    .unwrap();
    let id = nodes(&app, "stream-function-child")[0];
    let controls = nodes(&app, "kartis-datum");
    app.dispatch(id, Event::Activate).unwrap();
    app.dispatch(id, Event::FocusChanged(true)).unwrap();
    assert!(text(&app, id).starts_with("raw=8000000000000000;"));
    input.revise(|s| *s = next);
    app.flush().unwrap();
    assert!(text(&app, id).starts_with("raw=0000000000000000;"));
    assert!(text(&app, id).ends_with("count=1;focus=true"));
    assert_eq!(calls.get(), 1);
    let previous = text(&app, id);
    let total = app.stats().nodes;
    for rev in [90, 91] {
        input.revise(|s| *s = source(3, 400., rev));
        app.flush().unwrap();
        assert_eq!(text(&app, id), previous);
        assert_eq!(nodes(&app, "stream-function-child"), [id]);
        assert_eq!(nodes(&app, "kartis-datum"), controls);
        assert_eq!(app.stats().nodes, total);
    }
    let mut broken = source(3, 400., 7);
    broken.1.widgets.data.clear();
    input.revise(|s| *s = broken);
    app.flush().unwrap();
    assert_eq!(text(&app, id), previous);
    assert_eq!(errors.borrow().len(), 3);
    input.revise(|s| *s = source(3, 410., 7));
    app.flush().unwrap();
    assert!(text(&app, id).contains("value=6;"));
    assert!(text(&app, id).ends_with("count=1;focus=true"));
    assert_eq!(nodes(&app, "stream-function-child"), [id]);
    app.clear().unwrap();
    assert_eq!(app.stats().nodes, 0);
}

#[cfg(feature = "raster")]
fn image(app: &App) -> String {
    let tree = app.tree();
    let id = nodes(app, "kartis-raster-image")[0];
    let Props::Image(p) = tree.node(id).unwrap().descriptor.view().props else {
        panic!("Image")
    };
    p.source.clone()
}
#[cfg(feature = "raster")]
fn pixel(uri: &str, p: [usize; 2]) -> [u8; 4] {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(uri.split_once(',').unwrap().1)
        .unwrap();
    let mut r = png::Decoder::new(std::io::Cursor::new(bytes))
        .read_info()
        .unwrap();
    let mut out = vec![0; r.output_buffer_size().unwrap()];
    let info = r.next_frame(&mut out).unwrap();
    let i = 4 * (p[1] * info.width as usize + p[0]);
    out[i..i + 4].try_into().unwrap()
}
#[cfg(feature = "raster")]
#[test]
fn real_function_colored_png_and_owned_typed_frame_rollback_on_both_dpr_publishers() {
    use kartis::render::raster::{FontRegistry, RasterOptions};
    for dpr in [1., 2.] {
        let initial = source(0, 360., 7);
        let position = initial
            .0
            .interaction
            .as_ref()
            .unwrap()
            .sites
            .iter()
            .find(|s| {
                s.series == "b" && s.mark_id.starts_with("stream:sample") && s.value == Some(3.)
            })
            .unwrap()
            .position;
        let app = App::memory();
        let input = app.scope().cage(initial);
        let frame = app.scope().bond(move || input.with(|s| s.0.clone()));
        let packet = app.scope().bond(move || input.with(|s| s.1.clone()));
        let deny = Rc::new(Cell::new(false));
        let denied = deny.clone();
        let calls = Rc::new(Cell::new(0));
        let called = calls.clone();
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
                    if denied.get() {
                        Err(ChartError("independent function PNG denied".into()))
                    } else {
                        image.data_url()
                    }
                })
                .on_reject(move |e| failed.borrow_mut().push(e.0))
                .plot_widgets_composed(packet, move |c| child(c, called.clone())),
        )
        .unwrap();
        let id = nodes(&app, "stream-function-child")[0];
        app.dispatch(id, Event::Activate).unwrap();
        app.dispatch(id, Event::FocusChanged(true)).unwrap();
        let old = text(&app, id);
        let png = image(&app);
        assert_eq!(
            pixel(
                &png,
                [(position[0] * dpr) as usize, (position[1] * dpr) as usize]
            ),
            [180, 70, 90, 255]
        );
        let controls = nodes(&app, "kartis-datum");
        let n = publications.get();
        deny.set(true);
        input.revise(|s| *s = source(3, 400., 7));
        app.flush().unwrap();
        assert_eq!(publications.get(), n + 1);
        assert_eq!(image(&app), png);
        assert_eq!(text(&app, id), old);
        assert_eq!(nodes(&app, "kartis-datum"), controls);
        assert_eq!(errors.borrow().len(), 1);
        deny.set(false);
        input.revise(|s| *s = source(3, 410., 7));
        app.flush().unwrap();
        assert_ne!(image(&app), png);
        assert!(text(&app, id).contains("value=6;"));
        assert!(text(&app, id).ends_with("count=1;focus=true"));
        assert_eq!(nodes(&app, "stream-function-child"), [id]);
        assert_eq!(calls.get(), 1);
        app.clear().unwrap();
        assert_eq!(app.stats().nodes, 0);
    }
}
