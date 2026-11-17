//! Real typed Bullet role Widgets above retained SVG and Canvas charts.
use std::collections::BTreeSet;

use glory::Widget;
use glory::render::EventKind;
use glory::style::{Length, Style};

use super::*;
use crate::bullet_widget_model as model;

#[derive(Clone, Debug, PartialEq)]
struct Payload {
    part: BulletPartWidgetData,
    note: u32,
}
struct Fixture {
    root: web_sys::Element,
    chart: BrowserChart,
    renderer: &'static str,
    calls: Rc<Cell<u32>>,
    cleanups: Rc<Cell<u32>>,
    errors: Rc<RefCell<Vec<String>>>,
}
thread_local! {static FIXTURES:RefCell<Vec<Fixture>>=const{RefCell::new(Vec::new())};}
fn packet(
    phase: u8,
    width: f64,
    revision: u64,
    note: u32,
) -> Result<(Scene, PlotWidgetsData<Payload>)> {
    let (scene, data) = model::source(phase, width, revision)?;
    Ok((
        scene,
        PlotWidgetsData {
            layout: data.layout,
            data: data
                .data
                .into_iter()
                .map(|(k, part)| (k, Payload { part, note }))
                .collect(),
        },
    ))
}
#[wasm_bindgen]
pub fn bullet_widgets_mount() -> std::result::Result<(), JsValue> {
    bullet_widgets_stop();
    let document = web_sys::window().unwrap().document().unwrap();
    for (renderer, backend) in [
        ("svg", BrowserRenderer::Svg),
        ("canvas", BrowserRenderer::Canvas),
    ] {
        let root = document.create_element("div")?;
        root.set_id(&format!("bullet-widgets-{renderer}"));
        root.set_attribute("style", "width:600px;max-width:none;margin:12px 0")?;
        document.body().unwrap().append_child(&root)?;
        let (scene, data) = packet(0, 600., 0, 0).map_err(js)?;
        let calls = Rc::new(Cell::new(0));
        let cleanups = Rc::new(Cell::new(0));
        let errors = Rc::new(RefCell::new(vec![]));
        let (called, retired) = (calls.clone(), cleanups.clone());
        let chart = BrowserChart::mount_with_plot_widgets_data(
            root.clone(),
            scene,
            BrowserOptions {
                renderer: backend,
                height: Some(360.),
                motion: browser::MotionPreference::Reduced,
                highlight: browser::BrowserHighlight::Controlled,
                ..Default::default()
            },
            data,
            move |context: TypedPlotWidgetContext<Payload>| {
                called.set(called.get() + 1);
                let frame = context.frame;
                let role = frame.with(|f| match f.data.part.part.kind {
                    BulletPartKind::Range => "range",
                    BulletPartKind::Measure => "measure",
                    BulletPartKind::Marker => "marker",
                });
                if context.revision == 2 && role == "range" {
                    return Err(ChartError("range Widget revision rejected".into()));
                }
                let retired = retired.clone();
                Ok(glory::ui::widget(move |scope| {
                    let retired = retired.clone();
                    scope.on_cleanup(move || retired.set(retired.get() + 1))?;
                    let count = scope.cage(0u32);
                    glory::ui::button()
                        .class(format!("bullet-widget-{role}"))
                        .style(
                            Style::new()
                                .width(Length::Percent(100.))
                                .height(Length::Percent(100.)),
                        )
                        .label(scope.bond(move || {
                            frame.with(|f| {
                                format!(
                                    "{}|{}|v={}|rank={}|row={}|note={}|count={}",
                                    f.data.part.part.row,
                                    role,
                                    f.data.part.part.raw_value,
                                    f.data.part.rank,
                                    f.data.part.row_index,
                                    f.data.note,
                                    count.get()
                                )
                            })
                        }))
                        .on(EventKind::Activate, move |_| count.revise(|v| *v += 1))
                        .show_in(scope)
                }))
            },
        )
        .map_err(js)?;
        let events = errors.clone();
        chart.on_event(move |event| {
            if let browser::BrowserEvent::Error(error) = event {
                events.borrow_mut().push(error.0)
            }
        });
        chart.flush().map_err(js)?;
        FIXTURES.with(|f| {
            f.borrow_mut().push(Fixture {
                root,
                chart,
                renderer,
                calls,
                cleanups,
                errors,
            })
        });
    }
    Ok(())
}
#[wasm_bindgen]
pub fn bullet_widgets_update(
    phase: u8,
    width: f64,
    revision: u32,
    note: u32,
    invalid: u8,
) -> std::result::Result<(), JsValue> {
    update(None, phase, width, revision, note, invalid)
}

#[wasm_bindgen]
pub fn bullet_widgets_update_for(
    renderer: &str,
    phase: u8,
    width: f64,
    revision: u32,
    note: u32,
    invalid: u8,
) -> std::result::Result<(), JsValue> {
    update(Some(renderer), phase, width, revision, note, invalid)
}

fn update(
    renderer: Option<&str>,
    phase: u8,
    width: f64,
    revision: u32,
    note: u32,
    invalid: u8,
) -> std::result::Result<(), JsValue> {
    FIXTURES.with(|f| {
        for entry in f.borrow().iter() {
            if renderer.is_some_and(|renderer| renderer != entry.renderer) {
                continue;
            }
            let (mut scene, mut data) =
                packet(phase, width, u64::from(revision), note).map_err(js)?;
            match invalid {
                1 => {
                    let key = data.data.keys().next().unwrap().clone();
                    data.data.remove(&key);
                }
                2 => {
                    data.layout.slots[0].placement = PlotWidgetPlacement::Rect(LayoutRect {
                        x: 0.,
                        y: 0.,
                        width: f64::NAN,
                        height: 20.,
                    })
                }
                3 => scene.width = f64::NAN,
                4 => {
                    let wrong = PlotWidgetsData {
                        layout: data.layout.clone(),
                        data: data.data.keys().map(|k| (k.clone(), 1u32)).collect(),
                    };
                    entry.chart.set_plot_widgets_data(wrong).map_err(js)?;
                    continue;
                }
                5 => {
                    entry.chart.set_plot_widgets(data.layout).map_err(js)?;
                    continue;
                }
                6 => {
                    entry.chart.set_plot_widgets_data(data).map_err(js)?;
                    continue;
                }
                7 => {
                    let wrong = PlotWidgetsData::<u32> {
                        layout: PlotWidgetsLayout::default(),
                        data: BTreeMap::new(),
                    };
                    entry.chart.set_plot_widgets_data(wrong).map_err(js)?;
                    continue;
                }
                8 => {
                    data.layout.slots.clear();
                    data.data.clear();
                }
                _ => {}
            }
            entry
                .chart
                .set_layout_with_plot_widgets_data(scene.into(), data)
                .map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn bullet_widgets_hidden(hidden: bool) -> std::result::Result<(), JsValue> {
    FIXTURES.with(|f| {
        for entry in f.borrow().iter() {
            entry
                .chart
                .handle()
                .set_hidden(if hidden {
                    ["north:温度".into()].into_iter().collect()
                } else {
                    BTreeSet::new()
                })
                .map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn bullet_widgets_flush() -> std::result::Result<(), JsValue> {
    FIXTURES.with(|f| {
        for entry in f.borrow().iter() {
            entry.chart.flush().map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn bullet_widgets_snapshot(renderer: &str) -> std::result::Result<String, JsValue> {
    FIXTURES.with(|f| {let f=f.borrow();let entry=f.iter().find(|e|e.renderer==renderer).ok_or_else(||JsValue::from_str("no Bullet fixture"))?;let s=entry.chart.snapshot().map_err(js)?;
        Ok(json!({"active":s.active,"width":s.width,"height":s.height,"revision":s.revision,"selected":s.selected,"calls":entry.calls.get(),"cleanups":entry.cleanups.get(),"errors":entry.errors.borrow().clone()}).to_string())})
}
#[wasm_bindgen]
pub fn bullet_widgets_stop() {
    for entry in FIXTURES.with(|f| std::mem::take(&mut *f.borrow_mut())) {
        entry.chart.stop();
        entry.root.remove();
    }
}
