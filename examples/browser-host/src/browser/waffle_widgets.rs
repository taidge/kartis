//! Real Waffle cell/empty-cell Widgets in authored SVG/Canvas paint order.
use std::collections::BTreeSet;

use glory::Widget;
use glory::render::{Event, EventKind};
use glory::style::{Length, Style};

use super::*;
#[path = "../../../shared/waffle_widgets.rs"]
mod model;

struct Fixture {
    root: web_sys::Element,
    css: web_sys::Element,
    chart: BrowserChart,
    renderer: &'static str,
    calls: Rc<Cell<u32>>,
    cleanups: Rc<Cell<u32>>,
    mode: Rc<Cell<u8>>,
    reads: Rc<Cell<u32>>,
    errors: Rc<RefCell<Vec<String>>>,
}
thread_local! {static FIXTURES:RefCell<Vec<Fixture>>=const{RefCell::new(Vec::new())};}

#[wasm_bindgen]
pub fn waffle_widgets_mount(renderer: &str, initial_mode: u8) -> std::result::Result<(), JsValue> {
    let backend = match renderer {
        "svg" => BrowserRenderer::Svg,
        "canvas" => BrowserRenderer::Canvas,
        _ => return Err(JsValue::from_str("unknown renderer")),
    };
    let renderer = if renderer == "svg" { "svg" } else { "canvas" };
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div")?;
    root.set_id(&format!("waffle-widgets-{renderer}"));
    root.set_attribute(
        "style",
        "width:340px;max-width:none;margin:12px 0;transform-origin:0 0",
    )?;
    let css = document.create_element("style")?;
    css.set_text_content(Some(&format!("#waffle-widgets-{renderer} .waffle-widget{{width:100%!important;height:100%!important;border:0!important;border-radius:0!important;padding:0!important;margin:0!important;box-shadow:none!important;outline:none!important;appearance:none!important;font:8px monospace!important}}")));
    document
        .query_selector("head")?
        .unwrap()
        .append_child(&css)?;
    document.body().unwrap().append_child(&root)?;
    let (layout, composition) = model::source(0, 340., 0, 0, false).map_err(js)?;
    let calls = Rc::new(Cell::new(0));
    let cleanups = Rc::new(Cell::new(0));
    let reads = Rc::new(Cell::new(0));
    let errors = Rc::new(RefCell::new(vec![]));
    let mode = Rc::new(Cell::new(initial_mode));
    let handle = Rc::new(RefCell::new(None::<BrowserHandle>));
    let (called, retired, read, owner, fail) = (
        calls.clone(),
        cleanups.clone(),
        reads.clone(),
        handle.clone(),
        mode.clone(),
    );
    let chart = BrowserChart::mount_with_plot_widgets_in_scene(
        root.clone(),
        layout.scene.clone(),
        BrowserOptions {
            renderer: backend,
            height: Some(240.),
            motion: browser::MotionPreference::Reduced,
            highlight: browser::BrowserHighlight::Controlled,
            interaction_data: Some(layout.interaction.clone()),
            interaction: surface::SurfaceOptions {
                plot: composition.widgets.layout.plot,
                ..Default::default()
            },
            ..Default::default()
        },
        composition,
        move |context: PlotWidgetCompositionContext<model::Data>| {
            called.set(called.get() + 1);
            let frame = context.frame;
            let index = frame.with(|f| f.data.cell.cell.index);
            if fail.get() == 1 && index == 4 {
                return Err(ChartError("Waffle Widget factory rejected".into()));
            }
            if let Some(h) = owner.borrow().clone() {
                if fail.get() == 3 {
                    h.snapshot()?;
                    read.set(read.get() + 1);
                }
                if fail.get() == 4 {
                    let (_, nested) = model::source(0, 340., 0, 0, false)?;
                    if h.set_plot_widgets_in_scene(nested).is_err() {
                        read.set(read.get() + 1);
                    } else {
                        return Err(ChartError("nested Waffle construction accepted".into()));
                    }
                }
                if fail.get() == 5 {
                    h.stop();
                }
            }
            let build_failure = fail.get() == 2 && index == 4;
            let retired = retired.clone();
            Ok(glory::ui::widget(move |scope| {
                let retired = retired.clone();
                scope.on_cleanup(move || retired.set(retired.get() + 1))?;
                let count = scope.cage(0u32);
                let focus = scope.cage(false);
                let color = move || {
                    frame.with(
                        |f| match f.data.cell.group.as_ref().map(|g| g.source.id.as_str()) {
                            Some("one") => Color(255, 0, 0, 255),
                            Some(_) => Color(0, 200, 0, 255),
                            None => Color(255, 255, 0, 255),
                        },
                    )
                };
                glory::ui::button()
                .class(frame.with(|f|format!("waffle-widget waffle-widget-{}-{}",f.data.cell.cell.row,f.data.cell.cell.column)))
                .style(scope.bond(move||Style{background:Some(color()),color:Some(color()),..Style::new().width(Length::Percent(100.)).height(Length::Percent(100.))}))
                .label(scope.bond(move||frame.with(|f|format!("{}:{}|owner={}|raw={:?}|source={:?}|formatted={:?}|index={}|note={}|ink={}|count={}|focus={}",f.data.cell.cell.row,f.data.cell.cell.column,f.data.cell.group.as_ref().map_or("empty",|g|g.source.id.as_str()),f.data.cell.group.as_ref().map(|g|g.source.value),f.data.cell.group.as_ref().map(|g|g.source_index),f.data.cell.group.as_ref().map(|g|g.formatted_value.as_str()),f.data.cell.cell.index,f.data.note,if matches!(f.frame.mark.geometry,Geometry::Circle{..}){"circle"}else{"rect"},count.get(),focus.get()))))
                .on(EventKind::Activate,move|_|count.revise(|v|*v+=1)).on(EventKind::FocusChanged,move|event|{if let Event::FocusChanged(value)=event{focus.revise(|v|*v=value);}}).show_in(scope)?;
                if build_failure {
                    return Err(
                        glory::render::Error::InvalidProps("Waffle Widget build rejected").into(),
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
    *handle.borrow_mut() = Some(chart.handle());
    let reported = errors.clone();
    chart.on_event(move |event| {
        if let BrowserEvent::Error(error) = event {
            reported.borrow_mut().push(error.0);
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
            reads,
            errors,
        })
    });
    Ok(())
}

#[wasm_bindgen]
pub fn waffle_widgets_update(
    renderer: &str,
    phase: u8,
    width: f64,
    revision: u32,
    note: u32,
    cover_first: bool,
    flags: u16,
) -> std::result::Result<(), JsValue> {
    FIXTURES.with(|f| {
        let f = f.borrow();
        let entry = f
            .iter()
            .find(|e| e.renderer == renderer)
            .ok_or_else(|| JsValue::from_str("missing Waffle fixture"))?;
        entry.mode.set((flags & 255) as u8);
        let (mut layout, mut composition) =
            model::source(phase, width, u64::from(revision), note, cover_first).map_err(js)?;
        match flags >> 8 {
            1 => {
                composition
                    .replace_ink
                    .insert("missing-Waffle-source".into());
            }
            2 => layout.scene.width = f64::NAN,
            3 => {
                if let Some(id) = composition
                    .widgets
                    .layout
                    .slots
                    .first()
                    .map(|s| s.mark_id.clone())
                {
                    composition.widgets.data.remove(&id);
                }
            }
            4 => {
                if let Some(slot) = composition.widgets.layout.slots.first_mut() {
                    slot.placement = PlotWidgetPlacement::Rect(LayoutRect {
                        x: 0.,
                        y: 0.,
                        width: f64::NAN,
                        height: 20.,
                    });
                }
            }
            5 => {
                composition.replace_ink.clear();
            }
            _ => {}
        }
        entry
            .chart
            .set_layout_with_plot_widgets_in_scene(layout.into_scene_interaction(), composition)
            .map_err(js)
    })
}
#[wasm_bindgen]
pub fn waffle_widgets_flush() -> std::result::Result<(), JsValue> {
    FIXTURES.with(|f| {
        for e in f.borrow().iter() {
            e.chart.flush().map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn waffle_widgets_hidden(hidden: bool) -> std::result::Result<(), JsValue> {
    FIXTURES.with(|f| {
        for e in f.borrow().iter() {
            e.chart
                .handle()
                .set_hidden(if hidden {
                    BTreeSet::from(["one".into()])
                } else {
                    BTreeSet::new()
                })
                .map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn waffle_widgets_camera(reset: bool) -> std::result::Result<(), JsValue> {
    FIXTURES.with(|f| {
        for e in f.borrow().iter() {
            e.chart
                .handle()
                .dispatch(if reset {
                    SurfaceInput::Reset
                } else {
                    SurfaceInput::Wheel {
                        point: (170., 120.),
                        // Keep every measured cell wholly inside its real
                        // plot, so the normal partial-slot inert rule stays
                        // separate from this finite camera/input check.
                        delta: -10.,
                    }
                })
                .map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn waffle_widgets_snapshot(renderer: &str) -> std::result::Result<String, JsValue> {
    FIXTURES.with(|f|{let f=f.borrow();let e=f.iter().find(|e|e.renderer==renderer).ok_or_else(||JsValue::from_str("missing Waffle fixture"))?;
    if !e.chart.handle().is_active(){return Ok(json!({"active":false,"calls":e.calls.get(),"cleanups":e.cleanups.get(),"reads":e.reads.get()}).to_string());}let s=e.chart.snapshot().map_err(js)?;
    Ok(json!({"active":s.active,"width":s.width,"height":s.height,"revision":s.revision,"view":[s.view.scale,s.view.translate_x,s.view.translate_y],"hovered":s.hovered,"selected":s.selected,"calls":e.calls.get(),"cleanups":e.cleanups.get(),"reads":e.reads.get(),"errors":e.errors.borrow().clone()}).to_string())})
}
#[wasm_bindgen]
pub fn waffle_widgets_stop() {
    for e in FIXTURES.with(|f| std::mem::take(&mut *f.borrow_mut())) {
        e.chart.stop();
        e.root.remove();
        e.css.remove();
    }
}
