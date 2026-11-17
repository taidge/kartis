//! Actual Hierarchy node components in SVG/Canvas source paint order.
use glory::Widget;
use glory::render::{Event, EventKind};
use glory::style::{Length, Style};

use super::*;
#[path = "../../../shared/hierarchy_widgets.rs"]
mod model;

struct Fixture {
    key: String,
    kind: u8,
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
fn center(g: &Geometry) -> [f64; 2] {
    match g {
        Geometry::Rect {
            x,
            y,
            width,
            height,
        } => [x + width / 2., y + height / 2.],
        Geometry::Circle { x, y, .. } => [*x, *y],
        _ => unreachable!(),
    }
}
fn datum(d: &HierarchyNodeWidgetData) -> Value {
    let raw = &d.source[d.source_index];
    json!({"source":d.source.iter().map(|v|json!({"id":v.id,"label":v.label,"parent":v.parent,"value":v.value,"valueBits":v.value.to_bits().to_string()})).collect::<Vec<_>>(),
        "sourceIndex":d.source_index,"nodeId":d.node.id,"rawValue":raw.value,"rawBits":raw.value.to_bits().to_string(),"value":d.node.value,
        "kind":format!("{:?}",d.kind),"rootId":d.root_id,"focusId":d.focus_id,"focusAncestors":*d.focus_ancestors,
        "parent":d.node.parent,"children":d.node.children,"depth":d.node.depth,"relativeDepth":d.node.relative_depth,"leaf":d.node.leaf,"label":d.node.label,"labelSlots":d.node.label_slots,
        "formattedValue":d.formatted_value,"position":center(d.node.geometry.as_ref().unwrap()),"labels":d.labels.iter().map(|v|json!({"text":v.label.text,"position":[v.label.x,v.label.y],"clip":format!("{:?}",v.clip)})).collect::<Vec<_>>(),
        "plot":[d.plot.x,d.plot.y,d.plot.width,d.plot.height]})
}
fn frame(f: &TypedPlotWidgetFrame<HierarchyNodeWidgetData>, count: u32, focus: bool) -> String {
    json!({"id":f.frame.mark.id,"data":datum(&f.data),"rect":rect(f.frame.rect),
        "visible":f.frame.visible.map(rect),"interactive":f.frame.interactive,
        "paintValue":f.frame.mark.value,"paintFill":f.frame.mark.fill.map(rgba),"opacity":f.frame.mark.opacity,
        "geometry":format!("{:?}",f.frame.mark.geometry),"count":count,"focus":focus}).to_string()
}
#[wasm_bindgen]
pub fn hierarchy_widgets_mount(key: &str) -> std::result::Result<(), JsValue> {
    let (kind, renderer) = key
        .split_once('-')
        .ok_or_else(|| js("invalid hierarchy fixture key"))?;
    let kind: u8 = kind.parse().map_err(js)?;
    let (renderer, backend) = match renderer {
        "svg" => ("svg", BrowserRenderer::Svg),
        "canvas" => ("canvas", BrowserRenderer::Canvas),
        _ => return Err(js("unknown renderer")),
    };
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div")?;
    root.set_id(&format!("hierarchy-widgets-{key}"));
    root.set_attribute(
        "style",
        "width:600px;max-width:none;margin:12px 0;transform-origin:0 0",
    )?;
    let css = document.create_element("style")?;
    css.set_text_content(Some(&format!("#hierarchy-widgets-{key} .hierarchy-node-widget{{width:100%!important;height:100%!important;min-width:0!important;border:0!important;border-radius:0!important;padding:0!important;margin:0!important;box-shadow:none!important;outline:none!important;appearance:none!important;font:0px monospace!important;display:block!important;overflow:hidden!important}}")));
    document
        .query_selector("head")?
        .unwrap()
        .append_child(&css)?;
    document.body().unwrap().append_child(&root)?;
    let source = model::source(kind, 0, 600., 0).map_err(js)?;
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
        move |context: PlotWidgetCompositionContext<HierarchyNodeWidgetData>| {
            called.set(called.get() + 1);
            let frame = context.frame;
            let selected = frame.with(|f| f.data.node.id == "b");
            if fail.get() == 1 && selected {
                return Err(ChartError("Hierarchy node factory rejected".into()));
            }
            let build_failure = fail.get() == 2 && selected;
            let retired = retired.clone();
            Ok(glory::ui::widget(move |scope| {
                let retired = retired.clone();
                scope.on_cleanup(move || retired.set(retired.get() + 1))?;
                let count = scope.cage(0u32);
                let focus = scope.cage(false);
                glory::ui::button()
                    .class("hierarchy-node-widget")
                    .style(scope.bond(move || {
                        frame.with(|f| {
                            let c = color(Some(f.data.source[f.data.source_index].value));
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
                        "Hierarchy node child build rejected",
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
            key: key.to_owned(),
            kind,
            calls,
            cleanups,
            failure,
            errors,
        })
    });
    Ok(())
}
#[wasm_bindgen]
pub fn hierarchy_widgets_update(
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
            .find(|f| f.key == renderer)
            .ok_or_else(|| js("missing Hierarchy host"))?;
        entry.failure.set((flags & 255) as u8);
        let mut source =
            model::source(entry.kind, phase, width, u64::from(revision)).map_err(js)?;
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
                source.composition.replace_ink.insert("node:missing".into());
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
pub fn hierarchy_widgets_hidden(renderer: &str, hidden: bool) -> std::result::Result<(), JsValue> {
    FIXTURES.with(|f| {
        let f = f.borrow();
        let e = f
            .iter()
            .find(|f| f.key == renderer)
            .ok_or_else(|| js("missing Hierarchy host"))?;
        e.chart
            .handle()
            .set_hidden(if hidden {
                ["root", "north", "south", "a", "b", "c", "d", "e"]
                    .into_iter()
                    .map(String::from)
                    .collect()
            } else {
                Default::default()
            })
            .map_err(js)
    })
}
#[wasm_bindgen]
pub fn hierarchy_widgets_flush() -> std::result::Result<(), JsValue> {
    FIXTURES.with(|f| {
        for e in f.borrow().iter() {
            e.chart.flush().map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn hierarchy_widgets_snapshot(renderer: &str) -> std::result::Result<String, JsValue> {
    FIXTURES.with(|f|{let f=f.borrow();let e=f.iter().find(|f|f.key==renderer).ok_or_else(||js("missing Hierarchy host"))?;let s=e.chart.snapshot().map_err(js)?;
        Ok(json!({"active":s.active,"kind":e.kind,"renderer":e.renderer,"width":s.width,"height":s.height,"revision":s.revision,"selected":s.selected,"hovered":s.hovered,"hidden":s.hidden,
            "interaction":s.interaction.as_ref().map(|a|a.rows.iter().map(|r|json!({"id":r.mark_id,"value":r.value,"formatted":r.formatted_value})).collect::<Vec<_>>()),
            "calls":e.calls.get(),"cleanups":e.cleanups.get(),"errors":e.errors.borrow().clone()}).to_string())
    })
}
#[wasm_bindgen]
pub fn hierarchy_widgets_source(
    kind: u8,
    phase: u8,
    width: f64,
) -> std::result::Result<String, JsValue> {
    let source = model::source(kind, phase, width, 0).map_err(js)?;
    let scene = &source.frame.scene;
    let packet = &source.composition.widgets;
    let slots = packet
        .layout
        .slots
        .iter()
        .map(|s| {
            let d = &packet.data[&s.mark_id];
            let [x, y] = center(d.node.geometry.as_ref().unwrap());
            json!({"id":s.mark_id,"rect":[x-16.,y-11.,32.,22.],"data":datum(d)})
        })
        .collect::<Vec<_>>();
    Ok(json!({"width":scene.width,"height":scene.height,"plot":[source.frame.plot.unwrap().x,source.frame.plot.unwrap().y,source.frame.plot.unwrap().width,source.frame.plot.unwrap().height],
        "svg":kartis::render::to_svg(scene).map_err(js)?,"slots":slots,
        "marks":scene.marks.iter().map(|m|json!({"id":m.id,"label":m.label,"value":m.value,"fill":m.fill.map(rgba),"opacity":m.opacity,"geometry":format!("{:?}",m.geometry)})).collect::<Vec<_>>(),
        "sites":source.frame.interaction.as_ref().unwrap().sites.iter().map(|s|json!({"id":s.mark_id,"value":s.value,"position":s.position,"formatted":s.formatted_value})).collect::<Vec<_>>()}).to_string())
}
#[wasm_bindgen]
pub fn hierarchy_widgets_stop() -> u32 {
    let mut cleaned = 0;
    for e in FIXTURES.with(|f| std::mem::take(&mut *f.borrow_mut())) {
        e.chart.stop();
        cleaned += e.cleanups.get();
        e.root.remove();
        e.css.remove();
    }
    cleaned
}
