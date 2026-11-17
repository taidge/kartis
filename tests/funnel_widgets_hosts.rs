//! Real Memory and CPU-image custom-layer publication, not native OS input.
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use glory::render::{Event, EventKind, GraphicPoint as P, NodeId, PathCommand as Cmd, Props};
use glory::{App, Widget};
use kartis::*;

type Input = (SceneInteraction, PlotWidgetsComposition<FunnelWidgetData>);
fn raw() -> Vec<FunnelDatum> {
    let mut d = vec![
        FunnelDatum::new(-0., 10.),
        FunnelDatum::new("middle", 4.),
        FunnelDatum::new("last", -0.),
    ];
    d[0].label = "First".into();
    d[0].color = Some(Color(20, 130, 180, 255));
    d[1].color = Some(Color(180, 70, 90, 255));
    d[2].size = Some(-0.);
    d
}
fn source(data: &[FunnelDatum], width: f64, revision: u64, labels: bool) -> Input {
    let c = ChartConfig {
        width,
        height: 290.,
        margin: Margin {
            left: 31.,
            right: 23.,
            top: 19.,
            bottom: 27.,
        },
        ..Default::default()
    };
    let o = FunnelLayoutOptions {
        labels: labels.then(FunnelLabelOptions::default),
        legend: false,
        before: Some(FunnelSeparatorOptions {
            length: 18.,
            offset: 4.,
            ..Default::default()
        }),
        after: Some(FunnelSeparatorOptions {
            length: 14.,
            offset: 5.,
            ..Default::default()
        }),
        ..Default::default()
    };
    let l = funnel_layout_with_stage_factory(&c, data, &o, &mut |ctx| {
        if ctx.source_index != 0 {
            return Ok(ctx.inherit(ctx.geometry.clone()));
        }
        let [x, y] = ctx.part.center;
        let mut ink = ctx.inherit(Geometry::Path(vec![
            Cmd::Move(P::new(x + 30., y + 10.)),
            Cmd::Line(P::new(x + 42., y + 10.)),
            Cmd::Line(P::new(x + 36., y + 22.)),
            Cmd::Close,
        ]));
        ink.retain_profile_border = false;
        Ok(ink)
    })
    .unwrap();
    let widgets = funnel_widgets(&l, data, revision, |ctx| {
        let selected = (ctx.role == FunnelWidgetRole::Body && ctx.part_index == Some(0))
            || (ctx.role == FunnelWidgetRole::BeforeSeparator && ctx.source_index.is_none());
        Ok(selected.then_some(PlotWidgetPlacement::AtDatum {
            offset: [-5., -3.],
            measured: TextSize {
                width: 10.,
                height: 6.,
            },
        }))
    })
    .unwrap();
    (
        funnel_widget_frame(&l).unwrap(),
        PlotWidgetsComposition {
            replace_ink: widgets.data.keys().cloned().collect(),
            widgets,
        },
    )
}
fn nodes(app: &App, class: &str) -> Vec<NodeId> {
    let tree = app.tree();
    let mut todo = tree.roots().to_vec();
    let mut found = vec![];
    while let Some(id) = todo.pop() {
        let n = tree.node(id).unwrap();
        if n.descriptor.view().classes.iter().any(|c| c == class) {
            found.push(id);
        }
        todo.extend(n.children.iter().copied());
    }
    found
}
fn text(app: &App, id: NodeId) -> String {
    let tree = app.tree();
    let Props::Button(p) = tree.node(id).unwrap().descriptor.view().props else {
        panic!("actual Button expected")
    };
    p.label.clone()
}
fn child(
    ctx: PlotWidgetCompositionContext<FunnelWidgetData>,
    calls: Rc<Cell<usize>>,
) -> Result<impl Widget> {
    calls.set(calls.get() + 1);
    let frame = ctx.frame;
    let body = frame.with(|f| f.data.role == FunnelWidgetRole::Body);
    if body && ctx.revision == 90 {
        return Err(ChartError("independent Funnel factory rejected".into()));
    }
    let fail = body && ctx.revision == 91;
    Ok(glory::ui::widget(move |scope| {
        let count = scope.cage(0u32);
        let focus = scope.cage(false);
        glory::ui::button()
            .class(if body {
                "independent-funnel-body"
            } else {
                "independent-funnel-terminal"
            })
            .label(scope.bond(move || {
                frame.with(|f| {
                    let d = &f.data;
                    assert_eq!([f.frame.rect.x + 5., f.frame.rect.y + 3.], d.position);
                    let CategoryKey::Number(id) = d.source[0].id else {
                        panic!("numeric raw key")
                    };
                    if !body {
                        assert!(d.source_index.is_none() && d.part_index.is_none());
                        assert!(d.separators[d.separator_index.unwrap()].part_id.is_none());
                    }
                    format!(
                        "key={:016x};size={:016x};source={};parts={};labels={};count={};focus={}",
                        id.to_bits(),
                        d.source[2].size.unwrap().to_bits(),
                        d.source.len(),
                        d.parts.len(),
                        d.labels.labels.len(),
                        count.get(),
                        focus.get()
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
            return Err(
                glory::render::Error::InvalidProps("independent Funnel build rejected").into(),
            );
        }
        Ok(())
    }))
}

#[test]
fn raw_key_and_unselected_size_bits_reach_retained_body_and_real_terminal_without_fake_controls() {
    let data = raw();
    let initial = source(&data, 410., 8, true);
    let mut changed = data.clone();
    changed[0].id = CategoryKey::Number(0.);
    changed[2].size = Some(0.);
    let next = source(&changed, 410., 8, true);
    assert_eq!(initial.0, next.0);
    assert_eq!(
        render::to_svg(&initial.0.scene).unwrap(),
        render::to_svg(&next.0.scene).unwrap()
    );
    assert_eq!(initial.1.widgets.layout, next.1.widgets.layout);
    assert_ne!(initial.1.widgets, next.1.widgets);
    let app = App::memory();
    let input = app.scope().cage(initial);
    let frame = app.scope().bond(move || input.with(|i| i.0.clone()));
    let packet = app.scope().bond(move || input.with(|i| i.1.clone()));
    let calls = Rc::new(Cell::new(0));
    let called = calls.clone();
    let errors = Rc::new(RefCell::new(vec![]));
    let error = errors.clone();
    app.mount(
        chart_layout(frame)
            .legend(false)
            .tooltip(false)
            .on_reject(move |e| error.borrow_mut().push(e.0))
            .plot_widgets_composed(packet, move |c| child(c, called.clone())),
    )
    .unwrap();
    let body = nodes(&app, "independent-funnel-body")[0];
    let terminal = nodes(&app, "independent-funnel-terminal")[0];
    let controls = nodes(&app, "kartis-datum");
    assert_eq!(controls.len(), 2);
    app.dispatch(body, Event::Activate).unwrap();
    app.dispatch(body, Event::FocusChanged(true)).unwrap();
    assert!(text(&app, body).starts_with("key=8000000000000000;size=8000000000000000;"));
    input.revise(|old| *old = next);
    app.flush().unwrap();
    assert!(text(&app, body).starts_with("key=0000000000000000;size=0000000000000000;"));
    assert!(text(&app, terminal).starts_with("key=0000000000000000;size=0000000000000000;"));
    assert!(text(&app, body).ends_with("count=1;focus=true"));
    assert_eq!(calls.get(), 2);
    let old = [text(&app, body), text(&app, terminal)];
    let count = app.stats().nodes;
    for revision in [90, 91] {
        let mut next = changed.clone();
        next[0].value = 15.;
        input.revise(|old| *old = source(&next, 430., revision, true));
        app.flush().unwrap();
        assert_eq!([text(&app, body), text(&app, terminal)], old);
        assert_eq!(nodes(&app, "kartis-datum"), controls);
        assert_eq!(app.stats().nodes, count);
    }
    let mut bad = source(&changed, 430., 8, true);
    bad.1.widgets.data.pop_first();
    input.revise(|old| *old = bad);
    app.flush().unwrap();
    assert_eq!([text(&app, body), text(&app, terminal)], old);
    assert_eq!(errors.borrow().len(), 3);
    input.revise(|old| *old = source(&changed, 440., 8, true));
    app.flush().unwrap();
    assert_eq!(nodes(&app, "independent-funnel-body"), [body]);
    assert_eq!(nodes(&app, "independent-funnel-terminal"), [terminal]);
    assert!(text(&app, body).ends_with("count=1;focus=true"));
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
    let i = r.next_frame(&mut bytes).unwrap();
    let offset = 4 * (p[1] * i.width as usize + p[0]);
    bytes[offset..offset + 4].try_into().unwrap()
}
#[cfg(feature = "raster")]
#[test]
fn two_dpr_png_rejection_preserves_both_owned_layer_roles_and_recovery_accepts_raw_bits() {
    use kartis::render::raster::{FontRegistry, RasterOptions};
    for dpr in [1., 2.] {
        let data = raw();
        let initial = source(&data, 410., 8, false);
        let point = initial
            .0
            .interaction
            .as_ref()
            .unwrap()
            .sites
            .iter()
            .find(|s| s.label == "middle")
            .unwrap()
            .position;
        let app = App::memory();
        let input = app.scope().cage(initial);
        let frame = app.scope().bond(move || input.with(|i| i.0.clone()));
        let packet = app.scope().bond(move || input.with(|i| i.1.clone()));
        let calls = Rc::new(Cell::new(0));
        let called = calls.clone();
        let deny = Rc::new(Cell::new(false));
        let rejected = deny.clone();
        let publications = Rc::new(Cell::new(0));
        let published = publications.clone();
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
                    published.set(published.get() + 1);
                    if rejected.get() {
                        Err(ChartError("independent Funnel PNG denied".into()))
                    } else {
                        image.data_url()
                    }
                })
                .on_reject(move |e| error.borrow_mut().push(e.0))
                .plot_widgets_composed(packet, move |c| child(c, called.clone())),
        )
        .unwrap();
        let body = nodes(&app, "independent-funnel-body")[0];
        let terminal = nodes(&app, "independent-funnel-terminal")[0];
        let controls = nodes(&app, "kartis-datum");
        app.dispatch(body, Event::Activate).unwrap();
        app.dispatch(body, Event::FocusChanged(true)).unwrap();
        let old = [text(&app, body), text(&app, terminal)];
        let png = image(&app);
        assert_eq!(
            pixel(&png, [(point[0] * dpr) as usize, (point[1] * dpr) as usize]),
            [180, 70, 90, 255]
        );
        let mut next = data.clone();
        next[0].id = CategoryKey::Number(0.);
        next[2].size = Some(0.);
        let before = publications.get();
        deny.set(true);
        input.revise(|old| *old = source(&next, 440., 8, false));
        app.flush().unwrap();
        assert_eq!(publications.get(), before + 1);
        assert_eq!(errors.borrow().len(), 1);
        assert_eq!(image(&app), png);
        assert_eq!([text(&app, body), text(&app, terminal)], old);
        assert_eq!(nodes(&app, "kartis-datum"), controls);
        deny.set(false);
        input.revise(|old| *old = source(&next, 450., 8, false));
        app.flush().unwrap();
        assert_ne!(image(&app), png);
        assert!(text(&app, body).starts_with("key=0000000000000000;size=0000000000000000;"));
        assert!(text(&app, terminal).starts_with("key=0000000000000000;size=0000000000000000;"));
        assert!(text(&app, body).ends_with("count=1;focus=true"));
        assert_eq!(nodes(&app, "independent-funnel-body"), [body]);
        assert_eq!(nodes(&app, "independent-funnel-terminal"), [terminal]);
        assert_eq!(calls.get(), 2);
        app.clear().unwrap();
        assert_eq!(app.stats().nodes, 0);
    }
}
