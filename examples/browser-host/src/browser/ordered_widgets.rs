//! Actual source-ordered live Widgets and native paint planes.
use std::collections::BTreeSet;

use glory::Widget;
use glory::render::{Event, EventKind};
use glory::style::{Length, Style};

use super::*;
#[path = "../../../shared/ordered_widgets.rs"]
mod model;

struct Fixture {
    root: web_sys::Element,
    css: web_sys::Element,
    chart: BrowserChart,
    renderer: &'static str,
    calls: Rc<Cell<u32>>,
    cleanups: Rc<Cell<u32>>,
    reads: Rc<Cell<u32>>,
    errors: Rc<RefCell<Vec<String>>>,
    mode: Rc<Cell<u8>>,
}
thread_local! { static FIXTURES: RefCell<Vec<Fixture>> = const { RefCell::new(Vec::new()) }; }

#[wasm_bindgen]
pub fn ordered_widgets_mount(renderer: &str, initial_mode: u8) -> std::result::Result<(), JsValue> {
    let backend = match renderer {
        "svg" => BrowserRenderer::Svg,
        "canvas" => BrowserRenderer::Canvas,
        _ => return Err(JsValue::from_str("unknown renderer")),
    };
    let renderer = if renderer == "svg" { "svg" } else { "canvas" };
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div")?;
    root.set_id(&format!("ordered-widgets-{renderer}"));
    root.set_attribute(
        "style",
        "width:360px;max-width:none;margin:12px 0;transform-origin:0 0",
    )?;
    let css = document.create_element("style")?;
    css.set_text_content(Some(&format!("#ordered-widgets-{renderer} .ordered-widget {{ width:100%!important;height:100%!important;background:rgb(255,0,0)!important;color:rgb(255,0,0)!important;border:0!important;border-radius:0!important;padding:0!important;margin:0!important;box-shadow:none!important;outline:none!important;appearance:none!important;font:8px monospace!important; }} #ordered-widgets-{renderer} .ordered-widget-secondary {{background:rgb(255,255,0)!important;color:rgb(255,255,0)!important;}}")));
    document
        .query_selector("head")?
        .unwrap()
        .append_child(&css)?;
    document.body().unwrap().append_child(&root)?;
    let (scene, composition) = model::source(0, 360., 0, 0, 0).map_err(js)?;
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
        scene,
        BrowserOptions {
            renderer: backend,
            height: Some(260.),
            motion: browser::MotionPreference::Reduced,
            highlight: browser::BrowserHighlight::Controlled,
            ..Default::default()
        },
        composition,
        move |context: PlotWidgetCompositionContext<model::Data>| {
            called.set(called.get() + 1);
            let id = context.mark_id;
            let frame = context.frame;
            if fail.get() == 1 && id == "stage" {
                return Err(ChartError("ordered factory rejected".into()));
            }
            if let Some(h) = owner.borrow().clone() {
                if fail.get() == 3 {
                    h.snapshot()?;
                    read.set(read.get() + 1);
                }
                if fail.get() == 4 {
                    let (_, packet) = model::source(0, 360., 0, 0, 0)?;
                    if h.set_plot_widgets_in_scene(packet).is_err() {
                        read.set(read.get() + 1);
                    } else {
                        return Err(ChartError("nested ordered construction accepted".into()));
                    }
                }
                if fail.get() == 5 {
                    h.stop();
                }
            }
            let build_failure = fail.get() == 2 && id == "stage";
            let retired = retired.clone();
            Ok(glory::ui::widget(move |scope| {
                let retired = retired.clone();
                scope.on_cleanup(move || retired.set(retired.get() + 1))?;
                let count = scope.cage(0u32);
                let focus = scope.cage(false);
                glory::ui::button()
                    .class(format!(
                        "ordered-widget ordered-widget-{}",
                        if id == "stage" { "main" } else { "secondary" }
                    ))
                    .style(
                        Style::new()
                            .width(Length::Percent(100.))
                            .height(Length::Percent(100.)),
                    )
                    .label(scope.bond(move || {
                        frame.with(|f| {
                            format!(
                                "{}|note={}|value={}|count={}|focus={}",
                                f.data.source,
                                f.data.note,
                                f.frame.mark.value,
                                count.get(),
                                focus.get()
                            )
                        })
                    }))
                    .on(EventKind::Activate, move |_| {
                        count.revise(|value| *value += 1)
                    })
                    .on(EventKind::FocusChanged, move |event| {
                        if let Event::FocusChanged(value) = event {
                            focus.revise(|v| *v = value);
                        }
                    })
                    .show_in(scope)?;
                if build_failure {
                    return Err(glory::render::Error::InvalidProps(
                        "ordered Widget build rejected",
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
    *handle.borrow_mut() = Some(chart.handle());
    let reported = errors.clone();
    chart.on_event(move |event| {
        if let BrowserEvent::Error(error) = event {
            reported.borrow_mut().push(error.0);
        }
    });
    chart.flush().map_err(js)?;
    FIXTURES.with(|fixtures| {
        fixtures.borrow_mut().push(Fixture {
            root,
            css,
            chart,
            renderer,
            calls,
            cleanups,
            reads,
            errors,
            mode,
        })
    });
    Ok(())
}

#[wasm_bindgen]
pub fn ordered_widgets_update(
    renderer: &str,
    order: u8,
    width: f64,
    revision: u32,
    note: u32,
    variant: u8,
    flags: u16,
) -> std::result::Result<(), JsValue> {
    FIXTURES.with(|fixtures| {
        let fixtures = fixtures.borrow();
        let entry = fixtures
            .iter()
            .find(|f| f.renderer == renderer)
            .ok_or_else(|| JsValue::from_str("missing ordered fixture"))?;
        entry.mode.set((flags & 255) as u8);
        let (mut scene, mut composition) =
            model::source(order, width, u64::from(revision), note, variant).map_err(js)?;
        match flags >> 8 {
            1 => {
                composition.replace_ink.insert("missing-source".into());
            }
            2 => scene.width = f64::NAN,
            3 => {
                composition.widgets.data.remove("stage");
            }
            4 => {
                composition.widgets.layout.slots.clear();
                composition.widgets.data.clear();
                composition.replace_ink.clear();
            }
            _ => {}
        }
        entry
            .chart
            .set_layout_with_plot_widgets_in_scene(scene.into(), composition)
            .map_err(js)
    })
}

#[wasm_bindgen]
pub fn ordered_widgets_camera(reset: bool) -> std::result::Result<(), JsValue> {
    FIXTURES.with(|fixtures| {
        for entry in fixtures.borrow().iter() {
            entry
                .chart
                .handle()
                .dispatch(if reset {
                    SurfaceInput::Reset
                } else {
                    SurfaceInput::Wheel {
                        point: (180., 130.),
                        delta: -70.,
                    }
                })
                .map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn ordered_widgets_blend(renderer: &str, enabled: bool) -> std::result::Result<(), JsValue> {
    FIXTURES.with(|fixtures| {
        let fixtures = fixtures.borrow();
        let entry = fixtures
            .iter()
            .find(|f| f.renderer == renderer)
            .ok_or_else(|| JsValue::from_str("missing ordered fixture"))?;
        let (scene, _) = model::source(0, 360., 0, 31, 0).map_err(js)?;
        let modes = if enabled {
            MarkBlends::new(&scene, [("decorative-blue", BlendMode::Multiply)]).map_err(js)?
        } else {
            MarkBlends::default()
        };
        entry
            .chart
            .set_layout_with_blends(scene.into(), modes)
            .map_err(js)
    })
}
#[wasm_bindgen]
pub fn ordered_widgets_hidden(hidden: bool) -> std::result::Result<(), JsValue> {
    FIXTURES.with(|fixtures| {
        for entry in fixtures.borrow().iter() {
            entry
                .chart
                .handle()
                .set_hidden(if hidden {
                    BTreeSet::from(["ordered".into()])
                } else {
                    BTreeSet::new()
                })
                .map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn ordered_widgets_flush() -> std::result::Result<(), JsValue> {
    FIXTURES.with(|fixtures| {
        for entry in fixtures.borrow().iter() {
            entry.chart.flush().map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn ordered_widgets_snapshot(renderer: &str) -> std::result::Result<String, JsValue> {
    FIXTURES.with(|fixtures| {
        let fixtures = fixtures.borrow();
        let entry = fixtures.iter().find(|f| f.renderer == renderer).ok_or_else(|| JsValue::from_str("missing ordered fixture"))?;
        if !entry.chart.handle().is_active() { return Ok(json!({"active":false,"calls":entry.calls.get(),"cleanups":entry.cleanups.get(),"reads":entry.reads.get()}).to_string()); }
        let state = entry.chart.snapshot().map_err(js)?;
        Ok(json!({"active":state.active,"width":state.width,"height":state.height,"revision":state.revision,"selected":state.selected,"hovered":state.hovered,"view":[state.view.scale,state.view.translate_x,state.view.translate_y],"calls":entry.calls.get(),"cleanups":entry.cleanups.get(),"reads":entry.reads.get(),"errors":entry.errors.borrow().clone()}).to_string())
    })
}
#[wasm_bindgen]
pub fn ordered_widgets_stop() {
    for entry in FIXTURES.with(|fixtures| std::mem::take(&mut *fixtures.borrow_mut())) {
        entry.chart.stop();
        entry.root.remove();
        entry.css.remove();
    }
}
