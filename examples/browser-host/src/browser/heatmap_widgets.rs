//! Actual source-ordered HeatMap cells with nullable accepted Widget data.
use glory::Widget;
use glory::render::{Event, EventKind};
use glory::style::{Length, Style};

use super::*;
#[path = "../../../shared/heatmap_widgets.rs"]
mod model;

struct Fixture {
    root: web_sys::Element,
    css: web_sys::Element,
    chart: BrowserChart,
    renderer: &'static str,
    calls: Rc<Cell<u32>>,
    cleanups: Rc<Cell<u32>>,
    mode: Rc<Cell<u8>>,
    errors: Rc<RefCell<Vec<String>>>,
}
thread_local! { static FIXTURES: RefCell<Vec<Fixture>> = const { RefCell::new(Vec::new()) }; }

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
fn geometry(g: &Geometry) -> Value {
    match g {
        Geometry::Rect {
            x,
            y,
            width,
            height,
        } => json!({"kind":"rect","rect":[x,y,width,height]}),
        Geometry::Circle { x, y, radius } => {
            json!({"kind":"circle","center":[x,y],"radius":radius})
        }
        Geometry::Path(commands) => {
            json!({"kind":"path","commands":commands.len(),"debug":format!("{commands:?}")})
        }
    }
}
fn data(d: &HeatmapCellWidgetData) -> Value {
    json!({"row":d.datum.row,"column":d.datum.column,"value":d.datum.value,
        "sourceIndex":d.source_index,"cellValue":d.cell.value,"formatted":d.cell.formatted_value,
        "sourceLabel":d.cell.label,"palette":rgba(d.cell.color),"opacity":d.cell.opacity,
        "geometry":d.cell.geometry.as_ref().map(geometry),"center":d.cell.center,
        "band":d.cell.band.map(|r|[r.x,r.y,r.width,r.height])})
}
fn frame(f: &TypedPlotWidgetFrame<HeatmapCellWidgetData>, count: u32, focus: bool) -> String {
    json!({"id":f.frame.mark.id,"data":data(&f.data),"markLabel":f.frame.mark.label,
        "markValue":f.frame.mark.value,"markOpacity":f.frame.mark.opacity,
        "actualFill":f.frame.mark.fill.map(rgba),"markGeometry":geometry(&f.frame.mark.geometry),
        "rect":rect(f.frame.rect),"visible":f.frame.visible.map(rect),"interactive":f.frame.interactive,
        "count":count,"focus":focus}).to_string()
}

#[wasm_bindgen]
pub fn heatmap_widgets_mount(renderer: &str, initial_mode: u8) -> std::result::Result<(), JsValue> {
    let (renderer, backend) = match renderer {
        "svg" => ("svg", BrowserRenderer::Svg),
        "canvas" => ("canvas", BrowserRenderer::Canvas),
        _ => return Err(JsValue::from_str("unknown HeatMap renderer")),
    };
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div")?;
    root.set_id(&format!("heatmap-widgets-{renderer}"));
    root.set_attribute(
        "style",
        "width:600px;max-width:none;margin:12px 0;transform-origin:0 0",
    )?;
    let css = document.create_element("style")?;
    css.set_text_content(Some(&format!("#heatmap-widgets-{renderer} .heatmap-widget-cell{{width:100%!important;height:100%!important;border:0!important;border-radius:0!important;padding:0!important;margin:0!important;box-shadow:none!important;outline:none!important;appearance:none!important;overflow:hidden!important;font:8px monospace!important;}}")));
    document
        .query_selector("head")?
        .unwrap()
        .append_child(&css)?;
    document.body().unwrap().append_child(&root)?;
    let (source, packet) = model::source(0, 600., 0).map_err(js)?;
    let calls = Rc::new(Cell::new(0));
    let cleanups = Rc::new(Cell::new(0));
    let mode = Rc::new(Cell::new(initial_mode));
    let errors = Rc::new(RefCell::new(vec![]));
    let (called, retired, fail) = (calls.clone(), cleanups.clone(), mode.clone());
    let chart = BrowserChart::mount_with_plot_widgets_in_scene(
        root.clone(),
        source.scene,
        BrowserOptions {
            renderer: backend,
            interaction_data: source.interaction,
            interaction: kartis::surface::SurfaceOptions {
                plot: source.plot,
                ..Default::default()
            },
            height: Some(320.),
            motion: browser::MotionPreference::Reduced,
            highlight: browser::BrowserHighlight::Controlled,
            ..Default::default()
        },
        packet,
        move |context: PlotWidgetCompositionContext<HeatmapCellWidgetData>| {
            called.set(called.get() + 1);
            let frame = context.frame;
            let is_null = frame.with(|f| f.data.datum.column == "null");
            if fail.get() == 1 && is_null {
                return Err(ChartError("HeatMap cell factory rejected".into()));
            }
            let build_failure = fail.get() == 2 && is_null;
            let retired = retired.clone();
            Ok(glory::ui::widget(move |scope| {
                let retired = retired.clone();
                scope.on_cleanup(move || retired.set(retired.get() + 1))?;
                let count = scope.cage(0u32);
                let focus = scope.cage(false);
                glory::ui::button()
                    .class("heatmap-widget-cell")
                    .style(scope.bond(move || {
                        frame.with(|f| {
                            let c = color(f.data.datum.value);
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
                    return Err(
                        glory::render::Error::InvalidProps("HeatMap cell build rejected").into(),
                    );
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
    let observed = errors.clone();
    chart.on_event(move |event| {
        if let BrowserEvent::Error(e) = event {
            observed.borrow_mut().push(e.0);
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
            mode,
            errors,
        })
    });
    Ok(())
}

#[wasm_bindgen]
pub fn heatmap_widgets_update(
    renderer: &str,
    phase: u8,
    width: f64,
    revision: u32,
    flags: u16,
) -> std::result::Result<(), JsValue> {
    FIXTURES.with(|f| {
        let f = f.borrow();
        let e = f
            .iter()
            .find(|f| f.renderer == renderer)
            .ok_or_else(|| JsValue::from_str("missing HeatMap fixture"))?;
        e.mode.set((flags & 255) as u8);
        let (mut source, mut packet) =
            model::source(phase, width, u64::from(revision)).map_err(js)?;
        match flags >> 8 {
            1 => {
                packet.widgets.data.pop_first();
            }
            2 => {
                packet.widgets.layout.slots[0].placement = PlotWidgetPlacement::Rect(LayoutRect {
                    x: f64::NAN,
                    y: 20.,
                    width: 10.,
                    height: 10.,
                });
            }
            3 => source.scene.width = f64::NAN,
            4 => {
                packet.widgets.layout.slots.clear();
                packet.widgets.data.clear();
                packet.replace_ink.clear();
            }
            5 => {
                packet.replace_ink.insert("heatmap-cell:missing".into());
            }
            _ => {}
        }
        e.chart
            .set_layout_with_plot_widgets_in_scene(source, packet)
            .map_err(js)
    })
}
#[wasm_bindgen]
pub fn heatmap_widgets_hidden(renderer: &str, hidden: bool) -> std::result::Result<(), JsValue> {
    FIXTURES.with(|f| {
        let f = f.borrow();
        let e = f
            .iter()
            .find(|f| f.renderer == renderer)
            .ok_or_else(|| JsValue::from_str("missing HeatMap fixture"))?;
        e.chart
            .handle()
            .set_hidden(if hidden {
                ["north:温度".into(), "south".into()].into()
            } else {
                Default::default()
            })
            .map_err(js)
    })
}
#[wasm_bindgen]
pub fn heatmap_widgets_flush() -> std::result::Result<(), JsValue> {
    FIXTURES.with(|f| {
        for e in f.borrow().iter() {
            e.chart.flush().map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn heatmap_widgets_snapshot(renderer: &str) -> std::result::Result<String, JsValue> {
    FIXTURES.with(|f| {
        let f = f.borrow();
        let e = f
            .iter()
            .find(|f| f.renderer == renderer)
            .ok_or_else(|| JsValue::from_str("missing HeatMap fixture"))?;
        let s = e.chart.snapshot().map_err(js)?;
        Ok(json!({"active":s.active,"width":s.width,"height":s.height,
            "revision":s.revision,"selected":s.selected,"hovered":s.hovered,"hidden":s.hidden,
            "interaction":s.interaction.as_ref().map(|a|a.rows.iter().map(|r|json!({"id":r.mark_id,"value":r.value,"formatted":r.formatted_value})).collect::<Vec<_>>()),
            "calls":e.calls.get(),"cleanups":e.cleanups.get(),"errors":e.errors.borrow().clone()})
        .to_string())
    })
}
#[wasm_bindgen]
pub fn heatmap_widgets_source(phase: u8, width: f64) -> std::result::Result<String, JsValue> {
    let (s, p) = model::source(phase, width, 0).map_err(js)?;
    Ok(json!({"scene":format!("{:?}",s.scene),"labels":s.scene.labels.len(),"width":s.scene.width,"height":s.scene.height,
        "marks":s.scene.marks.iter().map(|m|json!({"id":m.id,"label":m.label,"value":m.value,"fill":m.fill.map(rgba),"opacity":m.opacity,"geometry":geometry(&m.geometry)})).collect::<Vec<_>>(),
        "slots":p.widgets.layout.slots.iter().map(|slot|json!({"id":slot.mark_id,"rect":match slot.placement {PlotWidgetPlacement::Rect(r)=>rect(r),_=>Value::Null},"data":data(&p.widgets.data[&slot.mark_id])})).collect::<Vec<_>>()}).to_string())
}
#[wasm_bindgen]
pub fn heatmap_widgets_stop() -> u32 {
    let mut cleaned = 0;
    for e in FIXTURES.with(|f| std::mem::take(&mut *f.borrow_mut())) {
        e.chart.stop();
        cleaned += e.cleanups.get();
        e.root.remove();
        e.css.remove();
    }
    cleaned
}
