//! Actual Scatter node components in SVG/Canvas source paint order.
use glory::Widget;
use glory::render::{Event, EventKind};
use glory::style::{Length, Style};

use super::*;
#[path = "../../../shared/scatter_widgets.rs"]
mod model;

struct Fixture {
    root: web_sys::Element,
    css: web_sys::Element,
    chart: BrowserChart,
    renderer: &'static str,
    calls: Rc<Cell<u32>>,
    cleanups: Rc<Cell<u32>>,
    failure: Rc<Cell<u8>>,
    errors: Rc<RefCell<Vec<String>>>,
}
thread_local! {static FIXTURES: RefCell<Vec<Fixture>>=const {RefCell::new(Vec::new())};}
fn color(value: Option<f64>) -> Color {
    match value {
        None => Color(255, 128, 192, 255),
        Some(0.) => Color(0, 200, 0, 255),
        Some(v) if v < 0. => Color(255, 220, 0, 255),
        Some(_) => Color(0, 100, 255, 255),
    }
}
fn rgba(Color(r, g, b, a): Color) -> [u8; 4] {
    [r, g, b, a]
}
fn rect(r: LayoutRect) -> Value {
    json!([r.x, r.y, r.width, r.height])
}
fn category(c: &CategoryKey) -> Value {
    match c {
        CategoryKey::Number(n) => json!({"kind":"number","value":n}),
        CategoryKey::String(s) => json!({"kind":"string","value":s}),
    }
}
fn datum(d: &ScatterNodeWidgetData) -> Value {
    let source = &d.series.data[d.site.source_index];
    json!({"series":{"id":d.series.id,"label":d.series.label,"color":d.series.color.map(rgba),"data":d.series.data.iter().map(|v|json!({"id":v.id,"x":category(&v.x),"y":category(&v.y),"sizeValue":v.size_value,"label":v.label,"color":v.color.map(rgba)})).collect::<Vec<_>>()},
        "seriesIndex":d.site.series_index,"sourceIndex":d.site.source_index,"globalIndex":d.site.global_index,
        "nodeId":d.site.node_id,"label":d.site.label,"sourceLabel":source.label,"sourceColor":source.color.map(rgba),
        "x":category(&d.site.raw_x),"y":category(&d.site.raw_y),"formattedX":d.site.formatted_x,"formattedY":d.site.formatted_y,
        "sizeValue":d.site.size_value,"nodeSize":d.site.node_size,"sizeUnit":format!("{:?}",d.site.size_unit),
        "siteColor":rgba(d.site.color),"position":d.site.position,
        "bounds":[d.site.bounds.x,d.site.bounds.y,d.site.bounds.width,d.site.bounds.height],
        "plot":[d.plot.x,d.plot.y,d.plot.width,d.plot.height]})
}
fn frame(f: &TypedPlotWidgetFrame<ScatterNodeWidgetData>, count: u32, focus: bool) -> String {
    json!({"id":f.frame.mark.id,"data":datum(&f.data),"rect":rect(f.frame.rect),
        "visible":f.frame.visible.map(rect),"interactive":f.frame.interactive,
        "paintValue":f.frame.mark.value,"paintFill":f.frame.mark.fill.map(rgba),"opacity":f.frame.mark.opacity,
        "geometry":format!("{:?}",f.frame.mark.geometry),"count":count,"focus":focus}).to_string()
}
#[wasm_bindgen]
pub fn scatter_widgets_mount(renderer: &str) -> std::result::Result<(), JsValue> {
    let (renderer, backend) = match renderer {
        "svg" => ("svg", BrowserRenderer::Svg),
        "canvas" => ("canvas", BrowserRenderer::Canvas),
        _ => return Err(js("unknown renderer")),
    };
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div")?;
    root.set_id(&format!("scatter-widgets-{renderer}"));
    root.set_attribute(
        "style",
        "width:600px;max-width:none;margin:12px 0;transform-origin:0 0",
    )?;
    let css = document.create_element("style")?;
    css.set_text_content(Some(&format!("#scatter-widgets-{renderer} .scatter-node-widget{{width:100%!important;height:100%!important;min-width:0!important;border:0!important;border-radius:0!important;padding:0!important;margin:0!important;box-shadow:none!important;outline:none!important;appearance:none!important;font:8px monospace!important;overflow:hidden!important}}")));
    document
        .query_selector("head")?
        .unwrap()
        .append_child(&css)?;
    document.body().unwrap().append_child(&root)?;
    let source = model::source(0, 600., 0).map_err(js)?;
    let calls = Rc::new(Cell::new(0));
    let cleanups = Rc::new(Cell::new(0));
    let failure = Rc::new(Cell::new(0));
    let errors = Rc::new(RefCell::new(vec![]));
    let (called, retired, fail) = (calls.clone(), cleanups.clone(), failure.clone());
    let chart = BrowserChart::mount_with_plot_widgets_in_scene(
        root.clone(),
        source.frame.scene,
        BrowserOptions {
            renderer: backend,
            height: Some(260.),
            legend: false,
            interaction_data: source.frame.interaction,
            interaction: surface::SurfaceOptions {
                plot: source.frame.plot,
                ..Default::default()
            },
            motion: browser::MotionPreference::Reduced,
            highlight: browser::BrowserHighlight::Controlled,
            ..Default::default()
        },
        source.composition,
        move |context: PlotWidgetCompositionContext<ScatterNodeWidgetData>| {
            called.set(called.get() + 1);
            let frame = context.frame;
            let selected =
                frame.with(|f| f.data.site.series_index == 0 && f.data.site.source_index == 2);
            if fail.get() == 1 && selected {
                return Err(ChartError("Scatter node factory rejected".into()));
            }
            let build_failure = fail.get() == 2 && selected;
            let retired = retired.clone();
            Ok(glory::ui::widget(move |scope| {
                let retired = retired.clone();
                scope.on_cleanup(move || retired.set(retired.get() + 1))?;
                let count = scope.cage(0u32);
                let focus = scope.cage(false);
                glory::ui::button()
                    .class("scatter-node-widget")
                    .style(scope.bond(move || {
                        frame.with(|f| {
                            let c = color(match f.data.site.raw_y {
                                CategoryKey::Number(n) => Some(n),
                                CategoryKey::String(_) => None,
                            });
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
                        scope
                            .bond(move || frame.with(|f| self::frame(f, count.get(), focus.get()))),
                    )
                    .on(EventKind::Activate, move |_| count.revise(|v| *v += 1))
                    .on(EventKind::FocusChanged, move |event| {
                        if let Event::FocusChanged(v) = event {
                            focus.revise(|f| *f = v);
                        }
                    })
                    .show_in(scope)?;
                if build_failure {
                    return Err(glory::render::Error::InvalidProps(
                        "Scatter node child build rejected",
                    )
                    .into());
                }
                Ok(())
            }))
        },
    );
    let chart = match chart {
        Ok(chart) => chart,
        Err(error) => {
            root.remove();
            css.remove();
            return Err(js(error));
        }
    };
    let rejected = errors.clone();
    chart.on_event(move |event| {
        if let BrowserEvent::Error(error) = event {
            rejected.borrow_mut().push(error.0);
        }
    });
    chart.flush().map_err(js)?;
    FIXTURES.with(|f| {
        f.borrow_mut().push(Fixture {
            root,
            css,
            chart,
            renderer,
            calls,
            cleanups,
            failure,
            errors,
        })
    });
    Ok(())
}
#[wasm_bindgen]
pub fn scatter_widgets_update(
    renderer: &str,
    phase: u8,
    width: f64,
    revision: u32,
    flags: u16,
) -> std::result::Result<(), JsValue> {
    FIXTURES.with(|f| {
        let f = f.borrow();
        let entry = f
            .iter()
            .find(|f| f.renderer == renderer)
            .ok_or_else(|| js("missing Scatter host"))?;
        entry.failure.set((flags & 255) as u8);
        let mut source = model::source(phase, width, u64::from(revision)).map_err(js)?;
        match flags >> 8 {
            1 => {
                source.composition.widgets.data.pop_first();
            }
            2 => {
                source.composition.widgets.layout.slots[0].placement =
                    PlotWidgetPlacement::Rect(LayoutRect {
                        x: f64::NAN,
                        y: 20.,
                        width: 12.,
                        height: 12.,
                    });
            }
            3 => source.frame.scene.width = f64::NAN,
            4 => {
                source.composition.widgets.layout.slots.clear();
                source.composition.widgets.data.clear();
                source.composition.replace_ink.clear();
            }
            5 => {
                source
                    .composition
                    .replace_ink
                    .insert("scatter-node:missing".into());
            }
            6 => {
                source.frame.interaction.as_mut().unwrap().sites[0].value = Some(991.);
            }
            _ => {}
        }
        entry
            .chart
            .set_layout_with_plot_widgets_in_scene(source.frame, source.composition)
            .map_err(js)
    })
}
#[wasm_bindgen]
pub fn scatter_widgets_hidden(renderer: &str, hidden: bool) -> std::result::Result<(), JsValue> {
    FIXTURES.with(|f| {
        let f = f.borrow();
        let e = f
            .iter()
            .find(|f| f.renderer == renderer)
            .ok_or_else(|| js("missing Scatter host"))?;
        e.chart
            .handle()
            .set_hidden(if hidden {
                ["north".into(), "south".into()].into()
            } else {
                Default::default()
            })
            .map_err(js)
    })
}
#[wasm_bindgen]
pub fn scatter_widgets_flush() -> std::result::Result<(), JsValue> {
    FIXTURES.with(|f| {
        for e in f.borrow().iter() {
            e.chart.flush().map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn scatter_widgets_snapshot(renderer: &str) -> std::result::Result<String, JsValue> {
    FIXTURES.with(|f|{let f=f.borrow();let e=f.iter().find(|f|f.renderer==renderer).ok_or_else(||js("missing Scatter host"))?;let s=e.chart.snapshot().map_err(js)?;
        Ok(json!({"active":s.active,"width":s.width,"height":s.height,"revision":s.revision,"selected":s.selected,"hovered":s.hovered,"hidden":s.hidden,
            "interaction":s.interaction.as_ref().map(|a|a.rows.iter().map(|r|json!({"id":r.mark_id,"value":r.value,"formatted":r.formatted_value})).collect::<Vec<_>>()),
            "calls":e.calls.get(),"cleanups":e.cleanups.get(),"errors":e.errors.borrow().clone()}).to_string())
    })
}
#[wasm_bindgen]
pub fn scatter_widgets_source(phase: u8, width: f64) -> std::result::Result<String, JsValue> {
    let source = model::source(phase, width, 0).map_err(js)?;
    let scene = &source.frame.scene;
    let packet = &source.composition.widgets;
    let slots = packet
        .layout
        .slots
        .iter()
        .map(|s| {
            let d = &packet.data[&s.mark_id];
            let [x, y] = d.site.position;
            json!({"id":s.mark_id,"rect":[x-16.,y-11.,32.,22.],"data":datum(d)})
        })
        .collect::<Vec<_>>();
    Ok(json!({"width":scene.width,"height":scene.height,"plot":[source.frame.plot.unwrap().x,source.frame.plot.unwrap().y,source.frame.plot.unwrap().width,source.frame.plot.unwrap().height],
        "svg":kartis::render::to_svg(scene).map_err(js)?,"slots":slots,
        "marks":scene.marks.iter().map(|m|json!({"id":m.id,"label":m.label,"value":m.value,"fill":m.fill.map(rgba),"opacity":m.opacity,"geometry":format!("{:?}",m.geometry)})).collect::<Vec<_>>(),
        "sites":source.frame.interaction.as_ref().unwrap().sites.iter().map(|s|json!({"id":s.mark_id,"value":s.value,"position":s.position,"formatted":s.formatted_value})).collect::<Vec<_>>()}).to_string())
}
#[wasm_bindgen]
pub fn scatter_widgets_stop() -> u32 {
    let mut cleaned = 0;
    for e in FIXTURES.with(|f| std::mem::take(&mut *f.borrow_mut())) {
        e.chart.stop();
        cleaned += e.cleanups.get();
        e.root.remove();
        e.css.remove();
    }
    cleaned
}
