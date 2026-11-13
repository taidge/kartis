//! Real persistent brush editing on independently owned SVG and Canvas charts.
use super::*;
use kartis::brush::{BrushAxis, BrushPhase, BrushReconfigure, BrushSource, EditableBrushOptions};
use std::{collections::BTreeSet, rc::Weak};
mod viewport;

struct Fixture {
    chart: BrowserChart,
    root: web_sys::Element,
    renderer: String,
    events: Rc<RefCell<Vec<Value>>>,
    reenter: Rc<Cell<bool>>,
    callback: Weak<()>,
}
thread_local! {
    static FIXTURES: RefCell<Vec<Fixture>> = const { RefCell::new(Vec::new()) };
    static RETIRED: RefCell<Vec<(String, BrowserHandle, Weak<()>)>> = const { RefCell::new(Vec::new()) };
}

fn axis(value: u8) -> Result<BrushAxis> {
    match value {
        0 => Ok(BrushAxis::XY),
        1 => Ok(BrushAxis::X),
        2 => Ok(BrushAxis::Y),
        _ => Err(ChartError("invalid editable brush axis".into())),
    }
}
fn bounds(rect: Brush) -> [f64; 4] {
    [rect.x0, rect.y0, rect.x1, rect.y1]
}
fn source(width: f64, height: f64) -> Result<Scene> {
    let mut scene = ChartConfig {
        width,
        height,
        title: "Editable selection".into(),
        ..Default::default()
    }
    .scene()?;
    for (series, color, points) in [
        (
            "alpha",
            Color(65, 125, 190, 255),
            [(100., 100.), (240., 180.), (400., 280.)],
        ),
        (
            "beta",
            Color(225, 125, 40, 255),
            [(180., 120.), (420., 280.), (540., 100.)],
        ),
    ] {
        for (index, (x, y)) in points.into_iter().enumerate() {
            let id = format!("point-{series}-{index}");
            let mut mark = Mark::new(id.clone(), Geometry::Circle { x, y, radius: 9. }, color);
            mark.series = series.into();
            mark.label = id;
            mark.value = index as f64;
            scene.marks.push(mark);
        }
        scene.legend.push(LegendItem {
            id: series.into(),
            label: series.into(),
            color,
        });
    }
    scene.validate()?;
    Ok(scene)
}
fn options(axis: BrushAxis, width: f64, height: f64) -> EditableBrushOptions {
    EditableBrushOptions {
        axis,
        extent: Brush {
            x0: 40.,
            y0: 30.,
            x1: 600_f64.min(width),
            y1: 350_f64.min(height),
        },
        handle_size: 10.,
        modifiers: true,
    }
}

#[wasm_bindgen]
pub fn editable_brush_mount(mode: u8) -> std::result::Result<(), JsValue> {
    let axis = axis(mode).map_err(js)?;
    editable_brush_stop();
    RETIRED.with(|r| r.borrow_mut().clear());
    let document = web_sys::window().unwrap().document().unwrap();
    for (name, renderer) in [
        ("svg", BrowserRenderer::Svg),
        ("canvas", BrowserRenderer::Canvas),
    ] {
        let root = document.create_element("div")?;
        root.set_id(&format!("editable-brush-{name}"));
        root.set_attribute(
            "style",
            "width:640px;max-width:none;position:relative;margin:12px 0",
        )?;
        document.body().unwrap().append_child(&root)?;
        let chart = match BrowserChart::mount(
            root.clone(),
            source(640., 400.).map_err(js)?,
            BrowserOptions {
                renderer,
                editable_brush: Some(options(axis, 640., 400.)),
                height: Some(400.),
                highlight: browser::BrowserHighlight::Controlled,
                motion: browser::MotionPreference::Reduced,
                ..Default::default()
            },
        ) {
            Ok(chart) => chart,
            Err(error) => {
                root.remove();
                editable_brush_stop();
                return Err(js(error));
            }
        };
        chart.on_resize(source);
        let events = Rc::new(RefCell::new(Vec::new()));
        let callback_events = events.clone();
        let reenter = Rc::new(Cell::new(false));
        let callback_reenter = reenter.clone();
        let handle = chart.handle();
        let owner = Rc::new(());
        let callback = Rc::downgrade(&owner);
        chart.on_event(move |event| {
            let _keep_owner = &owner;
            let value = match event {
                BrowserEvent::Interaction(SurfaceEvent::EditableBrush { event, ids }) => {
                    if callback_reenter.get() && event.phase == BrushPhase::Change && event.source == BrushSource::User {
                        callback_reenter.set(false);
                        if let Err(error) = handle.set_brush_selection(Some(Brush { x0:350., y0:60., x1:500., y1:160. })) {
                            callback_events.borrow_mut().push(json!({"callbackError":error.0}));
                        }
                    }
                    json!({"phase":format!("{:?}",event.phase),"source":format!("{:?}",event.source),"mode":format!("{:?}",event.mode),"selection":event.selection.map(bounds),"session":event.session.map(|s|format!("{s:?}")),"target":event.target.map(|t|format!("{t:?}")),"ids":ids})
                }
                BrowserEvent::Error(error) => json!({"error":error.0}),
                other => json!({"other":format!("{other:?}")}),
            };
            let mut events = callback_events.borrow_mut();
            if events.len() == 2048 { events.remove(0); }
            events.push(value);
        });
        chart.flush().map_err(js)?;
        FIXTURES.with(|slot| {
            slot.borrow_mut().push(Fixture {
                chart,
                root,
                renderer: name.into(),
                events,
                reenter,
                callback,
            })
        });
    }
    Ok(())
}

#[wasm_bindgen]
pub fn editable_brush_snapshot(renderer: &str) -> std::result::Result<String, JsValue> {
    if let Some(snapshot) = viewport::snapshot(renderer)? {
        return Ok(snapshot);
    }
    FIXTURES.with(|slot| {
        let entries = slot.borrow();
        let Some(f) = entries.iter().find(|f|f.renderer == renderer) else {
            return RETIRED.with(|slot| {
                let retired = slot.borrow();
                let stale = retired.iter().find(|(name,_,_)|name == renderer);
                Ok(json!({"active":false,"callbackCleanup":stale.is_none_or(|(_,_,weak)|weak.upgrade().is_none()),"staleHandleRejected":stale.is_none_or(|(_,handle,_)|handle.set_brush_selection(None).is_err())}).to_string())
            });
        };
        let state = f.chart.snapshot().map_err(js)?;
        let editor = state.editable_brush.map(|b| json!({"selection":b.selection.map(bounds),"ids":b.ids,"extent":bounds(b.options.extent),"axis":format!("{:?}",b.options.axis),"handleSize":b.options.handle_size,"active":b.active,"mode":b.mode.map(|m|format!("{m:?}")),"session":b.session.map(|s|format!("{s:?}")),"captured":b.captured_pointer,"touch":b.touch}));
        Ok(json!({"active":state.active,"renderer":f.renderer,"width":state.width,"height":state.height,"view":[state.view.scale,state.view.translate_x,state.view.translate_y],"revision":state.revision,"pending":state.pending,"dropped":state.dropped,"hidden":state.hidden,"editor":editor,"events":f.events.borrow().clone(),"callbackAlive":f.callback.upgrade().is_some()}).to_string())
    })
}

#[wasm_bindgen]
pub fn editable_brush_set(
    renderer: &str,
    selection: Option<Vec<f64>>,
) -> std::result::Result<(), JsValue> {
    let selection = selection
        .map(|values| {
            if values.len() != 4 {
                return Err(ChartError("brush fixture expects four coordinates".into()));
            }
            let rect = Brush {
                x0: values[0],
                y0: values[1],
                x1: values[2],
                y1: values[3],
            };
            rect.validate()?;
            Ok(rect)
        })
        .transpose()
        .map_err(js)?;
    with_fixture(renderer, |f| {
        f.chart.handle().set_brush_selection(selection)
    })
}

fn with_fixture(
    renderer: &str,
    action: impl FnOnce(&Fixture) -> Result<()>,
) -> std::result::Result<(), JsValue> {
    FIXTURES.with(|slot| {
        let entries = slot.borrow();
        let fixture = entries
            .iter()
            .find(|f| f.renderer == renderer)
            .ok_or_else(|| js(ChartError("editable brush fixture is stopped".into())))?;
        action(fixture).map_err(js)
    })
}

#[wasm_bindgen]
pub fn editable_brush_configure(
    renderer: &str,
    mode: u8,
    policy: u8,
    enabled: bool,
) -> std::result::Result<(), JsValue> {
    let axis = axis(mode).map_err(js)?;
    let policy = match policy {
        0 => BrushReconfigure::Clamp,
        1 => BrushReconfigure::Clear,
        2 => BrushReconfigure::Preserve,
        _ => return Err(js(ChartError("invalid brush fixture policy".into()))),
    };
    with_fixture(renderer, |f| {
        let state = f.chart.snapshot()?;
        f.chart.handle().configure_editable_brush(
            enabled.then(|| options(axis, state.width, state.height)),
            policy,
        )
    })
}

#[wasm_bindgen]
pub fn editable_brush_profile(
    renderer: &str,
    mode: u8,
    extent: Vec<f64>,
    modifiers: bool,
) -> std::result::Result<(), JsValue> {
    if extent.len() != 4 {
        return Err(js(ChartError(
            "brush fixture expects four extent coordinates".into(),
        )));
    }
    let options = EditableBrushOptions {
        axis: axis(mode).map_err(js)?,
        extent: Brush {
            x0: extent[0],
            y0: extent[1],
            x1: extent[2],
            y1: extent[3],
        },
        handle_size: 10.,
        modifiers,
    };
    options.validate().map_err(js)?;
    with_fixture(renderer, |f| {
        f.chart
            .handle()
            .configure_editable_brush(Some(options), BrushReconfigure::Clear)
    })
}

#[wasm_bindgen]
pub fn editable_brush_reenter(renderer: &str) -> std::result::Result<(), JsValue> {
    with_fixture(renderer, |f| {
        f.reenter.set(true);
        Ok(())
    })
}

#[wasm_bindgen]
pub fn editable_brush_hidden(renderer: &str, hide: bool) -> std::result::Result<(), JsValue> {
    with_fixture(renderer, |f| {
        f.chart.handle().set_hidden(if hide {
            BTreeSet::from(["alpha".into()])
        } else {
            BTreeSet::new()
        })
    })
}

#[wasm_bindgen]
pub fn editable_brush_rejected(renderer: &str) -> std::result::Result<bool, JsValue> {
    FIXTURES.with(|slot| {
        let entries = slot.borrow();
        let f = entries
            .iter()
            .find(|f| f.renderer == renderer)
            .ok_or_else(|| js(ChartError("brush fixture is stopped".into())))?;
        let handle = f.chart.handle();
        let state = handle.snapshot().map_err(js)?;
        let selection = handle
            .set_brush_selection(Some(Brush {
                x0: f64::NAN,
                y0: 40.,
                x1: 100.,
                y1: 100.,
            }))
            .is_err();
        let mut invalid = options(BrushAxis::XY, state.width, state.height);
        invalid.handle_size = -1.;
        let config = handle
            .configure_editable_brush(Some(invalid), BrushReconfigure::Clamp)
            .is_err();
        let mut scene = source(state.width, state.height).map_err(js)?;
        scene.marks[0].opacity = f64::NAN;
        let scene = handle.set_scene(scene).is_err();
        let after = handle.snapshot().map_err(js)?;
        Ok(selection
            && config
            && scene
            && after.revision == state.revision
            && after.pending == state.pending
            && after.editable_brush == state.editable_brush)
    })
}

#[wasm_bindgen]
pub fn editable_brush_flush() -> std::result::Result<(), JsValue> {
    viewport::flush()?;
    FIXTURES.with(|slot| {
        for f in slot.borrow().iter() {
            f.chart.flush().map_err(js)?;
        }
        Ok(())
    })
}

#[wasm_bindgen]
pub fn editable_brush_stop() {
    viewport::stop();
    let entries = FIXTURES.with(|slot| std::mem::take(&mut *slot.borrow_mut()));
    let mut retired = Vec::with_capacity(entries.len());
    for f in entries {
        let handle = f.chart.handle();
        f.chart.stop();
        f.root.remove();
        retired.push((f.renderer, handle, f.callback));
    }
    RETIRED.with(|slot| *slot.borrow_mut() = retired);
}
