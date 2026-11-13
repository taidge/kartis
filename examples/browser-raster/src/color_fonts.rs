use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;

use glory::Cage;
use glory::render::EventKind;
use glory::style::{Display, Style};
use glory::web::WebApp;
use kartis::render::raster::*;
use kartis::*;
use serde_json::json;
use wasm_bindgen::prelude::*;
struct Fixture {
    web: WebApp,
    scene: Cage<Scene>,
    errors: Rc<RefCell<Vec<String>>>,
}
thread_local! {static FIXTURE:RefCell<Option<Fixture>>=const{RefCell::new(None)};}
fn js(error: impl std::fmt::Display) -> JsValue {
    error.to_string().into()
}
fn fonts(palette: u16) -> FontRegistry {
    let mut fonts = FontRegistry::new().with_color_glyphs(ColorGlyphMode::ColrV0 { palette });
    fonts
        .register(
            include_bytes!("../../../tests/fonts/KartisFixtureColrV0.ttf").as_slice(),
            0,
        )
        .expect("fixture font");
    fonts
}
fn scene() -> Scene {
    let mut scene = ChartConfig {
        width: 180.,
        height: 180.,
        ..Default::default()
    }
    .scene()
    .unwrap();
    scene.labels.push(Label {
        text: "B".into(),
        x: 40.,
        y: 140.,
        size: 100.,
        color: Color(0, 255, 0, 255),
        anchor: TextAnchor::Start,
        rotation: 0.,
    });
    scene.label_series.insert(0, "glyph".into());
    scene
}
#[wasm_bindgen]
pub fn color_fixture_font() -> Vec<u8> {
    include_bytes!("../../../tests/fonts/KartisFixtureColrV0.ttf").to_vec()
}
fn export_scene(palette: u16, foreground_alpha: u8) -> kartis::Result<Scene> {
    let mut scene = scene();
    scene.labels[0].color = Color(0, 255, 0, foreground_alpha);
    scene_with_painted_labels(
        &scene,
        &fonts(palette),
        &PaintedLabelOptions {
            rotated_only: false,
            ..Default::default()
        },
        "color-export",
    )
}
#[wasm_bindgen]
pub fn color_fixture_svg(
    palette: u16,
    foreground_alpha: u8,
) -> std::result::Result<String, JsValue> {
    kartis::render::to_svg(&export_scene(palette, foreground_alpha).map_err(js)?).map_err(js)
}
#[wasm_bindgen]
pub fn color_fixture_canvas(
    palette: u16,
    foreground_alpha: u8,
    canvas: web_sys::HtmlCanvasElement,
) -> std::result::Result<(), JsValue> {
    kartis::render::draw_canvas(
        &export_scene(palette, foreground_alpha).map_err(js)?,
        &canvas,
        1.,
    )
    .map_err(js)
}
#[wasm_bindgen]
pub fn mount_color_fixture(palette: u16) -> std::result::Result<(), JsValue> {
    super::browser::stop_fixture()?;
    super::typed_surface::stop_typed_surface_fixture()?;
    stop_color_fixture()?;
    let root = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id("app"))
        .ok_or_else(|| js("missing root"))?;
    root.set_inner_html("");
    root.remove_attribute("data-gly-app")?;
    let mut source = None;
    let errors = Rc::new(RefCell::new(vec![]));
    let semantic_errors = errors.clone();
    let raster_errors = errors.clone();
    let fonts = fonts(palette);
    let web = glory::web::launch_with(root, |scope| {
        let scene = scope.cage(scene());
        source = Some(scene);
        let hidden = scope.cage(BTreeSet::new());
        glory::ui::view().class("color-font-fixture").fill((
            glory::ui::view()
                .style(Style::new().display(Display::Flex).gap(16.))
                .fill((
                    glory::ui::view().class("color-semantic").fill(
                        chart(scene)
                            .hidden(hidden)
                            .legend(false)
                            .data_controls(false)
                            .tooltip(false)
                            .painted_labels(
                                fonts.clone(),
                                PaintedLabelOptions {
                                    rotated_only: false,
                                    ..Default::default()
                                },
                            )
                            .on_reject(move |e| semantic_errors.borrow_mut().push(e.0)),
                    ),
                    glory::ui::view().class("color-raster").fill(
                        raster_chart(scene, fonts, RasterOptions::default())
                            .hidden(hidden)
                            .legend(false)
                            .data_controls(false)
                            .tooltip(false)
                            .on_reject(move |e| raster_errors.borrow_mut().push(e.0)),
                    ),
                )),
            glory::ui::button()
                .label("Toggle color glyph")
                .on(EventKind::Activate, move |_| {
                    hidden.revise(|set| {
                        if !set.remove("glyph") {
                            set.insert("glyph".into());
                        }
                    })
                }),
        ))
    })
    .map_err(js)?;
    FIXTURE.with(|slot| {
        *slot.borrow_mut() = Some(Fixture {
            web,
            scene: source.unwrap(),
            errors,
        })
    });
    Ok(())
}
#[wasm_bindgen]
pub fn color_fixture_update(text: &str, foreground_alpha: u8) -> std::result::Result<(), JsValue> {
    FIXTURE.with(|slot| {
        let slot = slot.borrow();
        let f = slot.as_ref().ok_or_else(|| js("color fixture stopped"))?;
        f.scene.revise(|scene| {
            scene.labels[0].text = text.into();
            scene.labels[0].color = Color(0, 255, 0, foreground_alpha);
        });
        f.web.app().flush().map_err(js)
    })
}
#[wasm_bindgen]
pub fn color_fixture_snapshot() -> String {
    FIXTURE.with(|slot|match slot.borrow().as_ref() {
        Some(f)=>json!({"active":true,"nodes":f.web.app().stats().nodes,"handlers":f.web.app().stats().handlers,"errors":f.errors.borrow().clone()}).to_string(),
        None=>json!({"active":false}).to_string(),
    })
}
#[wasm_bindgen]
pub fn stop_color_fixture() -> std::result::Result<(), JsValue> {
    if let Some(f) = FIXTURE.with(|slot| slot.borrow_mut().take()) {
        f.web.app().clear().map_err(js)?;
    }
    Ok(())
}
