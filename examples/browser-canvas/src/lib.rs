//! Application-owned Canvas: Rust computes layout, draws, and handles interaction.
#![cfg(target_arch = "wasm32")]
use std::cell::RefCell;
use std::rc::Rc;

use kartis::interaction::{Interaction, pointer_coordinates};
use kartis::{BarOptions, BarSeries, ChartConfig, Color, Scene, bar};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{
    Document, Element, Event, EventTarget, HtmlCanvasElement, KeyboardEvent, MouseEvent, Window,
};

const SERIES: [&str; 3] = ["Desktop", "Mobile", "Tablet"];
thread_local! { static DEMO: RefCell<Option<Demo>> = const { RefCell::new(None) }; }
struct Demo {
    _state: Rc<RefCell<State>>,
    _listeners: Vec<Listener>,
}
struct Listener {
    target: EventTarget,
    name: &'static str,
    callback: Closure<dyn FnMut(Event)>,
}
impl Drop for Listener {
    fn drop(&mut self) {
        let _ = self
            .target
            .remove_event_listener_with_callback(self.name, self.callback.as_ref().unchecked_ref());
    }
}
fn listen(
    target: &EventTarget,
    name: &'static str,
    callback: impl FnMut(Event) + 'static,
) -> Result<Listener, JsValue> {
    let callback = Closure::wrap(Box::new(callback) as Box<dyn FnMut(Event)>);
    target.add_event_listener_with_callback(name, callback.as_ref().unchecked_ref())?;
    Ok(Listener {
        target: target.clone(),
        name,
        callback,
    })
}
fn fail(message: impl ToString) -> JsValue {
    JsValue::from_str(&message.to_string())
}
fn element<T: JsCast>(document: &Document, id: &str) -> Result<T, JsValue> {
    document
        .get_element_by_id(id)
        .ok_or_else(|| fail(format!("missing #{id}")))?
        .dyn_into()
        .map_err(|_| fail(format!("unexpected element type for #{id}")))
}
struct State {
    window: Window,
    container: Element,
    canvas: HtmlCanvasElement,
    status: Element,
    legend: Vec<Element>,
    scene: Scene,
    interaction: Interaction,
    revision: u64,
}
impl State {
    fn redraw(&mut self) -> Result<(), JsValue> {
        let width = self
            .container
            .get_bounding_client_rect()
            .width()
            .clamp(240., 1600.);
        let config = ChartConfig {
            width,
            height: (width * 0.52).clamp(300., 480.),
            title: "Quarterly sessions by device".into(),
            ..Default::default()
        };
        let categories = ["Q1", "Q2", "Q3", "Q4"].map(str::to_owned);
        let values = [
            [64., 72., 81., 76.],
            [48., 61., 69., 88.],
            [22., 27., 31., 35.],
        ];
        let series: Vec<_> = SERIES
            .iter()
            .zip(values)
            .map(|(id, values)| BarSeries::new(*id, values.into_iter().map(Some).collect()))
            .collect();
        self.scene = bar(
            &config,
            &categories,
            &series,
            &BarOptions {
                hidden: self.interaction.hidden.clone(),
                value_domain: Some([0., 100.]),
                ..Default::default()
            },
        )
        .map_err(fail)?;
        self.interaction.reconcile(&self.scene);
        for (button, id) in self.legend.iter().zip(SERIES) {
            button.set_attribute(
                "aria-pressed",
                if self.interaction.hidden.contains(id) {
                    "false"
                } else {
                    "true"
                },
            )?;
        }
        self.paint()
    }
    fn paint(&mut self) -> Result<(), JsValue> {
        let mut painted = self.scene.clone();
        for mark in &mut painted.marks {
            if self.interaction.hovered.as_ref() == Some(&mark.id)
                || self.interaction.selected.as_ref() == Some(&mark.id)
            {
                mark.fill = Some(Color(33, 49, 78, 255));
            }
        }
        self.canvas
            .style()
            .set_property("width", &format!("{}px", painted.width))?;
        self.canvas
            .style()
            .set_property("height", &format!("{}px", painted.height))?;
        kartis::render::draw_canvas(
            &painted,
            &self.canvas,
            self.window.device_pixel_ratio().clamp(0.25, 4.),
        )
        .map_err(fail)?;
        self.revision = self.revision.wrapping_add(1);
        self.canvas
            .set_attribute("data-scene-width", &self.scene.width.to_string())?;
        self.canvas
            .set_attribute("data-scene-height", &self.scene.height.to_string())?;
        self.canvas
            .set_attribute("data-mark-count", &self.scene.marks.len().to_string())?;
        self.canvas
            .set_attribute("data-render-revision", &self.revision.to_string())?;
        self.canvas.set_attribute(
            "data-hidden-series",
            &self
                .interaction
                .hidden
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(","),
        )?;
        self.canvas.set_attribute(
            "data-hovered-mark",
            self.interaction.hovered.as_deref().unwrap_or(""),
        )?;
        self.canvas.set_attribute(
            "data-selected-mark",
            self.interaction.selected.as_deref().unwrap_or(""),
        )?;
        let current = self
            .interaction
            .hovered
            .as_ref()
            .or(self.interaction.selected.as_ref());
        let message = current
            .and_then(|id| self.scene.marks.iter().find(|m| &m.id == id))
            .map(|mark| format!("{} — {} sessions", mark.label, mark.value))
            .unwrap_or_else(|| "Hover a bar, select it, or use the arrow keys to explore.".into());
        self.status.set_text_content(Some(&message));
        Ok(())
    }
    fn point(&mut self, event: &MouseEvent) -> Result<(), JsValue> {
        let bounds = self.canvas.get_bounding_client_rect();
        let (x, y) = pointer_coordinates(
            &self.scene,
            (event.client_x() as f64, event.client_y() as f64),
            (bounds.left(), bounds.top(), bounds.width(), bounds.height()),
        )
        .map_err(fail)?;
        self.interaction.pointer(&self.scene, x, y);
        Ok(())
    }
    fn report(&self, error: JsValue) {
        self.status
            .set_text_content(Some(&format!("Chart error: {error:?}")));
        web_sys::console::error_1(&error);
    }
}
/// Release every event listener. The exported function also supports hot reload.
#[wasm_bindgen]
pub fn stop() {
    DEMO.with(|demo| {
        demo.borrow_mut().take();
    });
    if let Some(root) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.document_element())
    {
        let _ = root.set_attribute("data-canvas-ready", "false");
    }
}
#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    stop();
    let window = web_sys::window().ok_or_else(|| fail("window unavailable"))?;
    let document = window
        .document()
        .ok_or_else(|| fail("document unavailable"))?;
    let legend = (0..3)
        .map(|i| element(&document, &format!("legend-{i}")))
        .collect::<Result<Vec<Element>, _>>()?;
    let state = Rc::new(RefCell::new(State {
        window: window.clone(),
        container: element(&document, "plot")?,
        canvas: element(&document, "chart")?,
        status: element(&document, "status")?,
        legend: legend.clone(),
        scene: ChartConfig::default().scene().map_err(fail)?,
        interaction: Interaction::default(),
        revision: 0,
    }));
    state.borrow_mut().redraw()?;
    let target: EventTarget = state.borrow().canvas.clone().into();
    let mut listeners = vec![];
    for name in ["pointermove", "click"] {
        let weak = Rc::downgrade(&state);
        listeners.push(listen(&target, name, move |event| {
            let Some(state) = weak.upgrade() else {
                return;
            };
            let Some(pointer) = event.dyn_ref::<MouseEvent>() else {
                return;
            };
            let mut state = state.borrow_mut();
            let result = state.point(pointer).and_then(|()| {
                if name == "click" {
                    state.interaction.activate();
                }
                state.paint()
            });
            if let Err(error) = result {
                state.report(error);
            }
        })?);
    }
    let weak = Rc::downgrade(&state);
    listeners.push(listen(&target, "pointerleave", move |_| {
        if let Some(state) = weak.upgrade() {
            let mut state = state.borrow_mut();
            state.interaction.hovered = None;
            if let Err(error) = state.paint() {
                state.report(error);
            }
        }
    })?);
    let weak = Rc::downgrade(&state);
    listeners.push(listen(&target, "keydown", move |event| {
        let Some(state) = weak.upgrade() else {
            return;
        };
        let Some(key) = event.dyn_ref::<KeyboardEvent>() else {
            return;
        };
        let mut state = state.borrow_mut();
        let forward = match key.key().as_str() {
            "ArrowRight" | "ArrowDown" | "Home" => Some(true),
            "ArrowLeft" | "ArrowUp" | "End" => Some(false),
            "Escape" => {
                state.interaction.selected = None;
                None
            }
            _ => return,
        };
        key.prevent_default();
        state.interaction.hovered = None;
        if let Some(forward) = forward {
            if key.key() == "Home" || key.key() == "End" {
                state.interaction.selected = None;
            }
            let scene = state.scene.clone();
            state.interaction.step(&scene, forward);
        }
        if let Err(error) = state.paint() {
            state.report(error);
        }
    })?);
    for (button, id) in legend.into_iter().zip(SERIES) {
        let weak = Rc::downgrade(&state);
        listeners.push(listen(&button.into(), "click", move |_| {
            if let Some(state) = weak.upgrade() {
                let mut state = state.borrow_mut();
                state.interaction.toggle_series(id);
                if let Err(error) = state.redraw() {
                    state.report(error);
                }
            }
        })?);
    }
    let weak = Rc::downgrade(&state);
    listeners.push(listen(&window.into(), "resize", move |_| {
        if let Some(state) = weak.upgrade() {
            let mut state = state.borrow_mut();
            if let Err(error) = state.redraw() {
                state.report(error);
            }
        }
    })?);
    DEMO.with(|demo| {
        *demo.borrow_mut() = Some(Demo {
            _state: state,
            _listeners: listeners,
        });
    });
    document
        .document_element()
        .ok_or_else(|| fail("document root unavailable"))?
        .set_attribute("data-canvas-ready", "true")?;
    Ok(())
}
