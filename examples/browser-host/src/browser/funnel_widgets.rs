//! Actual SVG/Canvas custom-layer roles with typed source and style functions.
use glory::Widget;
use glory::render::{Event as UiEvent, EventKind};
use glory::style::{Length, Style};

use super::*;
#[path = "../../../shared/funnel_widgets.rs"]
mod model;
struct Fixture {
    key: String,
    root: web_sys::Element,
    css: web_sys::Element,
    chart: BrowserChart,
    profile: u8,
    calls: Rc<Cell<u32>>,
    cleanups: Rc<Cell<u32>>,
    registered: Rc<Cell<u32>>,
    failure: Rc<Cell<u8>>,
}
thread_local! {static FIXTURES:RefCell<Vec<Fixture>>=const {RefCell::new(Vec::new())};}
fn rgba(Color(r, g, b, a): Color) -> [u8; 4] {
    [r, g, b, a]
}
fn rect(r: ClipRect) -> [f64; 4] {
    [r.x, r.y, r.width, r.height]
}
fn slot(r: LayoutRect) -> [f64; 4] {
    [r.x, r.y, r.width, r.height]
}
fn key(k: &CategoryKey) -> Value {
    match k {
        CategoryKey::String(s) => json!({"kind":"string","value":s}),
        CategoryKey::Number(n) => json!({"kind":"number","value":n,"bits":n.to_bits().to_string()}),
    }
}
fn datum(d: &FunnelDatum) -> Value {
    json!({"id":key(&d.id),"label":d.label,"value":d.value,"valueBits":d.value.to_bits().to_string(),"size":d.size,"sizeBits":d.size.map(|s|s.to_bits().to_string()),"color":d.color.map(rgba),"paint":d.paint.as_ref().map(|p|format!("{p:?}"))})
}
fn part(p: &FunnelPart) -> Value {
    json!({"datum":datum(&p.datum),"sourceIndex":p.source_index,"markId":p.mark_id,"series":p.series,"bounds":rect(p.bounds),"center":p.center,"color":rgba(p.color),"formatted":p.formatted_value,"current":p.is_current,"profile":p.profile.iter().map(|p|json!({"before":p.before,"after":p.after})).collect::<Vec<_>>()})
}
fn data(d: &FunnelWidgetData) -> Value {
    json!({"source":d.source.iter().map(datum).collect::<Vec<_>>(),"parts":d.parts.iter().map(part).collect::<Vec<_>>(),
        "separators":d.separators.iter().map(|s|json!({"partId":s.part_id.as_ref().map(key),"side":format!("{:?}",s.side),"from":s.from,"to":s.to})).collect::<Vec<_>>(),
        "role":format!("{:?}",d.role),"sourceIndex":d.source_index,"partIndex":d.part_index,"separatorIndex":d.separator_index,"position":d.position,"plot":rect(d.plot),
        "labels":d.labels.labels.iter().enumerate().map(|(i,l)|json!({"position":[l.x,l.y],"text":l.text,"size":l.size,"color":rgba(l.color),"rotation":l.rotation,"anchor":format!("{:?}",l.anchor),"series":d.labels.series.get(&i),"clip":d.labels.clips.get(&i).map(|c|format!("{c:?}"))})).collect::<Vec<_>>()})
}
fn label(
    f: &TypedPlotWidgetFrame<FunnelWidgetData>,
    built: u64,
    count: u32,
    focus: bool,
) -> String {
    json!({"id":f.frame.mark.id,"data":data(&f.data),"rect":slot(f.frame.rect),"visible":f.frame.visible.map(slot),"interactive":f.frame.interactive,"paintValue":f.frame.mark.value,"paintSeries":f.frame.mark.series,"paintFill":f.frame.mark.fill.map(rgba),"paintStroke":f.frame.mark.stroke.map(|s|json!({"color":rgba(s.color),"width":s.width})),"opacity":f.frame.mark.opacity,"geometry":format!("{:?}",f.frame.mark.geometry),"builtRevision":built,"count":count,"focus":focus}).to_string()
}
#[wasm_bindgen]
pub fn funnel_widgets_mount(name: &str) -> std::result::Result<(), JsValue> {
    let (profile, renderer) = name
        .split_once('-')
        .ok_or_else(|| js("bad Funnel Widget key"))?;
    let profile: u8 = profile.parse().map_err(js)?;
    if profile > 3 {
        return Err(js("bad Funnel Widget profile"));
    }
    let renderer = match renderer {
        "svg" => BrowserRenderer::Svg,
        "canvas" => BrowserRenderer::Canvas,
        _ => return Err(js("bad renderer")),
    };
    let doc = web_sys::window().unwrap().document().unwrap();
    let root = doc.create_element("div")?;
    root.set_id(&format!("funnel-widgets-{name}"));
    root.set_attribute(
        "style",
        "width:720px;max-width:none;margin:12px 0;transform-origin:0 0",
    )?;
    let css = doc.create_element("style")?;
    css.set_text_content(Some(&format!("#funnel-widgets-{name} .funnel-live-widget{{display:block!important;width:100%!important;height:100%!important;min-width:0!important;border:0!important;border-radius:0!important;padding:0!important;margin:0!important;box-shadow:none!important;outline:none!important;appearance:none!important;font:0px monospace!important;overflow:hidden!important}}")));
    doc.query_selector("head")?.unwrap().append_child(&css)?;
    doc.body().unwrap().append_child(&root)?;
    let source = model::source(0, 720., 0, profile, 0).map_err(js)?;
    let calls = Rc::new(Cell::new(0));
    let cleanups = Rc::new(Cell::new(0));
    let registered = Rc::new(Cell::new(0));
    let failure = Rc::new(Cell::new(0));
    let (called, retired, reg, fail) = (
        calls.clone(),
        cleanups.clone(),
        registered.clone(),
        failure.clone(),
    );
    let factory = move |context: PlotWidgetCompositionContext<FunnelWidgetData>| {
        called.set(called.get() + 1);
        let frame = context.frame;
        let built = context.revision;
        if fail.get() == 1 && frame.with(|f| f.data.role == FunnelWidgetRole::Body) {
            return Err(ChartError("Funnel Widget factory rejected".into()));
        }
        let reject = fail.get() == 2 && frame.with(|f| f.data.role == FunnelWidgetRole::Border);
        let (retired, reg) = (retired.clone(), reg.clone());
        Ok(glory::ui::widget(move |scope| {
            let retired = retired.clone();
            scope.on_cleanup(move || retired.set(retired.get() + 1))?;
            reg.set(reg.get() + 1);
            let count = scope.cage(0u32);
            let focus = scope.cage(false);
            glory::ui::button()
                .class("funnel-live-widget")
                .style(scope.bond(move || {
                    frame.with(|f| {
                        let c = model::color(&f.data);
                        Style {
                            background: Some(c),
                            color: Some(c),
                            ..Style::new()
                                .width(Length::Percent(100.))
                                .height(Length::Percent(100.))
                        }
                    })
                }))
                .label(
                    scope.bond(move || frame.with(|f| label(f, built, count.get(), focus.get()))),
                )
                .on(EventKind::Activate, move |_| count.revise(|v| *v += 1))
                .on(EventKind::FocusChanged, move |e| {
                    if let UiEvent::FocusChanged(v) = e {
                        focus.revise(|f| *f = v);
                    }
                })
                .show_in(scope)?;
            if reject {
                return Err(
                    glory::render::Error::InvalidProps("Funnel Widget build rejected").into(),
                );
            }
            Ok(())
        }))
    };
    let options = BrowserOptions {
        renderer,
        height: Some(320.),
        legend: false,
        interaction_data: source.frame.interaction,
        interaction: surface::SurfaceOptions {
            plot: source.frame.plot,
            ..Default::default()
        },
        highlight: browser::BrowserHighlight::Controlled,
        motion: browser::MotionPreference::Reduced,
        ..Default::default()
    };
    let result = if profile < 2 {
        BrowserChart::mount_with_plot_widgets_in_scene(
            root.clone(),
            source.frame.scene,
            options,
            source.composition,
            factory,
        )
    } else {
        BrowserChart::mount_with_plot_widgets_composed(
            root.clone(),
            source.frame.scene,
            options,
            source.composition,
            factory,
        )
    };
    let chart = match result {
        Ok(c) => c,
        Err(e) => {
            root.remove();
            css.remove();
            return Err(js(e));
        }
    };
    chart.flush().map_err(js)?;
    FIXTURES.with(|all| {
        all.borrow_mut().push(Fixture {
            key: name.into(),
            root,
            css,
            chart,
            profile,
            calls,
            cleanups,
            registered,
            failure,
        })
    });
    Ok(())
}
fn with<T>(
    key: &str,
    f: impl FnOnce(&Fixture) -> std::result::Result<T, JsValue>,
) -> std::result::Result<T, JsValue> {
    FIXTURES.with(|all| {
        let all = all.borrow();
        f(all
            .iter()
            .find(|e| e.key == key)
            .ok_or_else(|| js("missing Funnel Widget host"))?)
    })
}
#[wasm_bindgen]
pub fn funnel_widgets_set(
    key: &str,
    phase: u8,
    width: f64,
    revision: u32,
    corrupt: u8,
) -> std::result::Result<(), JsValue> {
    with(key, |e| {
        let mut source =
            model::source(phase, width, revision.into(), e.profile, e.failure.get()).map_err(js)?;
        match corrupt {
            1 => {
                source.composition.widgets.data.pop_first();
            }
            2 => {
                source.composition.widgets.layout.slots[0].placement =
                    PlotWidgetPlacement::Rect(LayoutRect {
                        x: f64::NAN,
                        y: 20.,
                        width: 32.,
                        height: 22.,
                    });
            }
            3 => source.frame.scene.width = f64::NAN,
            4 => {
                source
                    .composition
                    .replace_ink
                    .insert("funnel:stage:missing".into());
            }
            5 => source.frame.interaction.as_mut().unwrap().sites[0].value = Some(991.),
            _ => {}
        }
        if e.profile < 2 {
            e.chart
                .set_layout_with_plot_widgets_in_scene(source.frame, source.composition)
        } else {
            e.chart
                .set_layout_with_plot_widgets_composed(source.frame, source.composition)
        }
        .map_err(js)
    })
}
#[wasm_bindgen]
pub fn funnel_widgets_failure(key: &str, mode: u8) -> std::result::Result<(), JsValue> {
    with(key, |e| {
        e.failure.set(mode);
        Ok(())
    })
}
#[wasm_bindgen]
pub fn funnel_widgets_flush(key: &str) -> std::result::Result<(), JsValue> {
    with(key, |e| e.chart.flush().map_err(js))
}
#[wasm_bindgen]
pub fn funnel_widgets_hidden(key: &str, hidden: bool) -> std::result::Result<(), JsValue> {
    with(key, |e| {
        e.chart
            .handle()
            .set_hidden(if hidden {
                [format!(
                    "funnel:{}",
                    CategoryKey::from("a").identity().map_err(js)?
                )]
                .into()
            } else {
                Default::default()
            })
            .map_err(js)
    })
}
#[wasm_bindgen]
pub fn funnel_widgets_zoom(key: &str, zoom: bool) -> std::result::Result<(), JsValue> {
    with(key, |e| {
        e.chart.handle().dispatch(SurfaceInput::Reset).map_err(js)?;
        if zoom {
            let s = e.chart.snapshot().map_err(js)?;
            e.chart
                .handle()
                .dispatch(SurfaceInput::Wheel {
                    point: (s.width / 2., s.height / 2.),
                    delta: -1.1f64.ln() / 0.002,
                })
                .map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn funnel_widgets_width(key: &str, width: f64) -> std::result::Result<(), JsValue> {
    with(key, |e| {
        e.root
            .unchecked_ref::<web_sys::HtmlElement>()
            .style()
            .set_property("width", &format!("{width}px"))
    })
}
#[wasm_bindgen]
pub fn funnel_widgets_snapshot(key: &str) -> std::result::Result<String, JsValue> {
    with(key, |e| {
        let s = e.chart.snapshot().map_err(js)?;
        Ok(json!({"active":s.active,"width":s.width,"height":s.height,"revision":s.revision,"view":[s.view.scale,s.view.translate_x,s.view.translate_y],"selected":s.selected,"hidden":s.hidden,"hovered":s.hovered,"calls":e.calls.get(),"cleanups":e.cleanups.get(),"registered":e.registered.get(),"interaction":s.interaction.as_ref().map(|i|i.rows.iter().map(|r|json!({"id":r.mark_id,"value":r.value,"formatted":r.formatted_value})).collect::<Vec<_>>())}).to_string())
    })
}
#[wasm_bindgen]
pub fn funnel_widgets_stop(key: &str) -> std::result::Result<String, JsValue> {
    FIXTURES.with(|all| {
        let mut all = all.borrow_mut();
        let index = all
            .iter()
            .position(|e| e.key == key)
            .ok_or_else(|| js("missing Funnel Widget host"))?;
        let e = all.remove(index);
        e.chart.stop();
        e.root.remove();
        e.css.remove();
        Ok(json!({"cleanups":e.cleanups.get(),"registered":e.registered.get()}).to_string())
    })
}
