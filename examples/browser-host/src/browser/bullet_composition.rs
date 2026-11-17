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
    mode: Rc<Cell<u8>>,
    reads: Rc<Cell<u32>>,
    cleanup_reads: Rc<Cell<u32>>,
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
pub fn bullet_composition_mount(
    only_renderer: &str,
    initial_mode: u8,
) -> std::result::Result<(), JsValue> {
    let document = web_sys::window().unwrap().document().unwrap();
    for (renderer, backend) in [
        ("svg", BrowserRenderer::Svg),
        ("canvas", BrowserRenderer::Canvas),
    ] {
        if only_renderer != renderer {
            continue;
        }
        let root = document.create_element("div")?;
        root.set_id(&format!("bullet-composition-{renderer}"));
        root.set_attribute("style", "width:600px;max-width:none;margin:12px 0")?;
        document.body().unwrap().append_child(&root)?;
        let (scene, data) = packet(0, 600., 0, 0).map_err(js)?;
        let calls = Rc::new(Cell::new(0));
        let cleanups = Rc::new(Cell::new(0));
        let errors = Rc::new(RefCell::new(vec![]));
        let mode = Rc::new(Cell::new(initial_mode));
        let reads = Rc::new(Cell::new(0));
        let cleanup_reads = Rc::new(Cell::new(0));
        let handle = Rc::new(RefCell::new(None::<BrowserHandle>));
        let owner = handle.clone();
        let (fail, read, cleaned) = (mode.clone(), reads.clone(), cleanup_reads.clone());
        let (called, retired) = (calls.clone(), cleanups.clone());
        let chart = BrowserChart::mount_with_plot_widgets_composed(
            root.clone(),
            scene,
            BrowserOptions {
                renderer: backend,
                height: Some(360.),
                motion: browser::MotionPreference::Reduced,
                highlight: browser::BrowserHighlight::Controlled,
                ..Default::default()
            },
            PlotWidgetsComposition {
                replace_ink: data
                    .layout
                    .slots
                    .iter()
                    .map(|s| s.mark_id.clone())
                    .collect(),
                widgets: data,
            },
            move |context: PlotWidgetCompositionContext<Payload>| {
                called.set(called.get() + 1);
                let frame = context.frame;
                let role = frame.with(|f| match f.data.part.part.kind {
                    BulletPartKind::Range => "range",
                    BulletPartKind::Measure => "measure",
                    BulletPartKind::Marker => "marker",
                });
                if fail.get() == 1 && role == "marker" {
                    return Err(ChartError("strict marker factory rejected".into()));
                }
                if let Some(h) = owner.borrow().clone() {
                    if fail.get() == 3 {
                        h.snapshot()?;
                        read.set(read.get() + 1);
                    }
                    if fail.get() == 4 {
                        let (_, data) = packet(0, 600., 0, 0)?;
                        let nested = PlotWidgetsComposition {
                            widgets: data,
                            replace_ink: BTreeSet::new(),
                        };
                        if h.set_plot_widgets_composed(nested).is_err() {
                            read.set(read.get() + 1);
                        } else {
                            return Err(ChartError(
                                "nested composition unexpectedly accepted".into(),
                            ));
                        }
                    }
                    if fail.get() == 5 {
                        h.stop();
                    }
                }
                let build_failure = fail.get() == 2 && role == "marker";
                let cleanup_handle = owner.clone();
                let cleaned = cleaned.clone();
                let retired = retired.clone();
                Ok(glory::ui::widget(move |scope| {
                    let retired = retired.clone();
                    let cleanup_handle = cleanup_handle.clone();
                    let cleaned = cleaned.clone();
                    scope.on_cleanup(move || {
                        retired.set(retired.get() + 1);
                        if let Some(h) = cleanup_handle.borrow().clone() {
                            let _ = h.snapshot();
                            cleaned.set(cleaned.get() + 1);
                        }
                    })?;
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
                        .show_in(scope)?;
                    if build_failure {
                        return Err(glory::render::Error::InvalidProps(
                            "strict marker Widget build rejected",
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
                return Err(js(error));
            }
        };
        *handle.borrow_mut() = Some(chart.handle());
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
                mode,
                reads,
                cleanup_reads,
            })
        });
    }
    Ok(())
}
#[wasm_bindgen]
pub fn bullet_composition_update(
    renderer: &str,
    phase: u8,
    width: f64,
    revision: u32,
    note: u32,
    replacement: u8,
    flags: u16,
) -> std::result::Result<(), JsValue> {
    let mode = (flags & 255) as u8;
    let invalid = (flags >> 8) as u8;
    FIXTURES.with(|f| {
        let f = f.borrow();
        let entry = f
            .iter()
            .find(|e| e.renderer == renderer)
            .ok_or_else(|| JsValue::from_str("missing composition fixture"))?;
        entry.mode.set(mode);
        let (mut scene, mut data) = packet(phase, width, u64::from(revision), note).map_err(js)?;
        let mut replace_ink: BTreeSet<_> = data
            .layout
            .slots
            .iter()
            .filter(|s| {
                replacement == 1
                    || (replacement == 2
                        && data.data[&s.mark_id].part.part.kind == BulletPartKind::Range)
            })
            .map(|s| s.mark_id.clone())
            .collect();
        match invalid {
            1 => {
                replace_ink.insert("unknown-physical-mark".into());
            }
            2 => scene.width = f64::NAN,
            3 => {
                data.data.remove(&data.layout.slots[0].mark_id);
            }
            4 => {
                data.layout.slots.clear();
                data.data.clear();
                replace_ink.clear();
            }
            5 => {
                let slot = &data.layout.slots[0];
                let PlotWidgetPlacement::Rect(r) = slot.placement else {
                    unreachable!()
                };
                let mut commands = Geometry::Rect {
                    x: r.x,
                    y: r.y,
                    width: r.width,
                    height: r.height,
                }
                .commands();
                commands.extend(
                    Geometry::Rect {
                        x: r.x + 35.,
                        y: r.y + 8.,
                        width: 30.,
                        height: 10.,
                    }
                    .commands(),
                );
                scene
                    .marks
                    .iter_mut()
                    .find(|m| m.id == slot.mark_id)
                    .unwrap()
                    .clip_path = Some(Box::new(ClipPath {
                    geometry: Geometry::Path(commands),
                    rule: ClipRule::EvenOdd,
                }));
            }
            6 | 7 => {
                scene.opacity_groups.push(MarkOpacityGroup {
                    id: "composition-visibility".into(),
                    members: scene.marks.iter().map(|m| m.id.clone()).collect(),
                    opacity: if invalid == 6 { 0. } else { 0.5 },
                });
            }
            _ => {}
        }
        entry
            .chart
            .set_layout_with_plot_widgets_composed(
                scene.into(),
                PlotWidgetsComposition {
                    widgets: data,
                    replace_ink,
                },
            )
            .map_err(js)
    })
}
#[wasm_bindgen]
pub fn bullet_composition_hidden(hidden: bool) -> std::result::Result<(), JsValue> {
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
pub fn bullet_composition_camera(reset: bool) -> std::result::Result<(), JsValue> {
    FIXTURES.with(|f| {
        for entry in f.borrow().iter() {
            entry
                .chart
                .handle()
                .dispatch(if reset {
                    surface::SurfaceInput::Reset
                } else {
                    surface::SurfaceInput::Wheel {
                        point: (260., 180.),
                        delta: -100.,
                    }
                })
                .map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn bullet_composition_flush() -> std::result::Result<(), JsValue> {
    FIXTURES.with(|f| {
        for entry in f.borrow().iter() {
            entry.chart.flush().map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn bullet_composition_snapshot(renderer: &str) -> std::result::Result<String, JsValue> {
    FIXTURES.with(|f| {let f=f.borrow();let entry=f.iter().find(|e|e.renderer==renderer).ok_or_else(||JsValue::from_str("no Bullet fixture"))?;if !entry.chart.handle().is_active(){return Ok(json!({"active":false,"calls":entry.calls.get(),"cleanups":entry.cleanups.get(),"reads":entry.reads.get(),"cleanupReads":entry.cleanup_reads.get()}).to_string());}let s=entry.chart.snapshot().map_err(js)?;
        Ok(json!({"active":s.active,"width":s.width,"height":s.height,"revision":s.revision,"selected":s.selected,"calls":entry.calls.get(),"cleanups":entry.cleanups.get(),"errors":entry.errors.borrow().clone(),"hovered":s.hovered,"view":[s.view.scale,s.view.translate_x,s.view.translate_y],"reads":entry.reads.get(),"cleanupReads":entry.cleanup_reads.get()}).to_string())})
}
#[wasm_bindgen]
pub fn bullet_composition_stop() {
    for entry in FIXTURES.with(|f| std::mem::take(&mut *f.borrow_mut())) {
        entry.chart.stop();
        entry.root.remove();
    }
}
