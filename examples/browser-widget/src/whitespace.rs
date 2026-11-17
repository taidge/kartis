//! Mode-only updates exercised on real retained SVG and ordinary Glory Text.
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use glory::Cage;
use glory::web::WebApp;
use kartis::browser::{BrowserChart, BrowserOptions, BrowserRenderer, BrowserSemanticTextMeasurer};
use kartis::*;
use wasm_bindgen::prelude::*;

struct Fixture {
    web: WebApp,
    scene: Cage<Scene>,
    svg: BrowserChart,
    canvas: BrowserChart,
    measures: Rc<Cell<usize>>,
}
thread_local! { static FIXTURE: RefCell<Option<Fixture>> = const { RefCell::new(None) }; }
fn js(e: impl std::fmt::Display) -> JsValue {
    e.to_string().into()
}
fn source(preserve: bool) -> Scene {
    let mut scene = ChartConfig {
        width: 320.,
        height: 140.,
        ..Default::default()
    }
    .scene()
    .unwrap();
    scene.labels.push(Label {
        x: 40.,
        y: 70.,
        text: "  A \t B  C  ".into(),
        size: 30.,
        color: Color(20, 40, 70, 255),
        anchor: TextAnchor::Start,
        rotation: 0.,
    });
    if preserve {
        scene
            .typography
            .label_whitespace
            .insert(0, LabelWhitespace::Preserve);
    }
    scene
}
#[wasm_bindgen]
pub fn whitespace_mount() -> std::result::Result<(), JsValue> {
    let document = web_sys::window().unwrap().document().unwrap();
    source(false)
        .validate()
        .map_err(|e| js(format!("source: {e}")))?;
    let svg = BrowserChart::mount(
        document
            .get_element_by_id("retained")
            .ok_or_else(|| js("retained root"))?,
        source(false),
        BrowserOptions {
            renderer: BrowserRenderer::Svg,
            legend: false,
            data_controls: false,
            tooltip: false,
            ..Default::default()
        },
    )
    .map_err(|e| js(format!("retained: {e}")))?;
    let canvas = BrowserChart::mount(
        document
            .get_element_by_id("canvas")
            .ok_or_else(|| js("canvas root"))?,
        source(false),
        BrowserOptions {
            renderer: BrowserRenderer::Canvas,
            legend: false,
            data_controls: false,
            tooltip: false,
            ..Default::default()
        },
    )
    .map_err(js)?;
    let measurer = BrowserSemanticTextMeasurer::new(&document).map_err(js)?;
    let measures = Rc::new(Cell::new(0));
    let counter = measures.clone();
    let rejects = document.clone();
    let probe = document.clone();
    let mut cage = None;
    let web = glory::web::launch_with(
        document
            .get_element_by_id("semantic")
            .ok_or_else(|| js("semantic root"))?,
        |scope| {
            let data = scope.cage(source(false));
            cage = Some(data);
            chart(data)
                .on_reject(move |error| {
                    if let Some(root) = rejects.get_element_by_id("semantic") {
                        let _ = root.set_attribute("data-reject", &error.to_string());
                    }
                })
                .semantic_text_measurer(SemanticTextProfile::default(), move |request| {
                    counter.set(counter.get() + 1);
                    let result = measurer.measure(request);
                    if let Err(error) = &result
                        && let Some(root) = probe.get_element_by_id("semantic")
                    {
                        let _ = root.set_attribute("data-measure-error", &error.to_string());
                    }
                    result
                })
        },
    )
    .map_err(|e| js(format!("semantic: {e}")))?;
    FIXTURE.with(|f| {
        *f.borrow_mut() = Some(Fixture {
            web,
            scene: cage.unwrap(),
            svg,
            canvas,
            measures,
        })
    });
    Ok(())
}
#[wasm_bindgen]
pub fn whitespace_mode(preserve: bool) -> std::result::Result<(), JsValue> {
    FIXTURE.with(|slot| {
        let fixture = slot.borrow();
        let fixture = fixture.as_ref().ok_or_else(|| js("not mounted"))?;
        let next = source(preserve);
        fixture.scene.revise(|scene| *scene = next.clone());
        fixture.web.app().flush().map_err(js)?;
        fixture.svg.handle().set_scene(next.clone()).map_err(js)?;
        fixture.svg.handle().flush().map_err(js)?;
        fixture.canvas.handle().set_scene(next).map_err(js)?;
        fixture.canvas.handle().flush().map_err(js)?;
        Ok(())
    })
}
#[wasm_bindgen]
pub fn whitespace_measurements() -> usize {
    FIXTURE.with(|f| f.borrow().as_ref().unwrap().measures.get())
}
