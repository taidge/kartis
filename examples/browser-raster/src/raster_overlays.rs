use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::rc::Rc;

use glory::render::EventKind;
use glory::web::WebApp;
use glory::{Bond, Cage, Widget};
use kartis::render::raster::*;
use kartis::*;
use serde_json::json;
use wasm_bindgen::prelude::*;

struct Fixture {
    web: WebApp,
    scene: Cage<Scene>,
    options: Cage<RasterOptions>,
    hidden: Cage<BTreeSet<String>>,
    frame: Rc<RefCell<Option<Bond<PlotWidgetFrame>>>>,
    count: Rc<Cell<u32>>,
    errors: Rc<RefCell<Vec<String>>>,
    fail: Rc<Cell<bool>>,
    publications: Rc<Cell<u32>>,
    factories: Rc<Cell<u32>>,
    children_cleaned: Rc<Cell<u32>>,
    publisher_drops: Rc<Cell<u32>>,
    factory_drops: Rc<Cell<u32>>,
    cleared: Cell<bool>,
}
struct Capture(Rc<Cell<u32>>);
impl Drop for Capture {
    fn drop(&mut self) {
        self.0.set(self.0.get() + 1);
    }
}
thread_local! {static FIXTURE:RefCell<Option<Fixture>>=const{RefCell::new(None)};}
fn js(error: impl std::fmt::Display) -> JsValue {
    error.to_string().into()
}
fn scene(phase: u8, value: f64) -> Result<Scene> {
    let mut scene = ChartConfig {
        width: 300.,
        height: 160.,
        ..Default::default()
    }
    .scene()?;
    let (x, y, color) = if phase == 0 {
        (115., 70., Color(210, 40, 50, 255))
    } else {
        (150. + value - 20., 80., Color(30, 90, 190, 255))
    };
    let mut mark = Mark::new("alpha", Geometry::Circle { x, y, radius: 28. }, color);
    mark.value = value;
    mark.label = "Alpha overlay datum".into();
    mark.series = "alpha".into();
    if phase == 2 {
        mark.clip = Some(ClipRect {
            x: 140.,
            y: 70.,
            width: 10.,
            height: 20.,
        });
    }
    scene.marks.push(mark);
    Ok(scene)
}
fn options(phase: u8) -> RasterOptions {
    if phase == 0 {
        RasterOptions::default()
    } else {
        RasterOptions {
            view: ZoomTransform::new(1.2, 0., 0.).unwrap(),
            pixel_ratio: 2.,
            ..Default::default()
        }
    }
}
fn layout() -> PlotWidgetsLayout {
    PlotWidgetsLayout {
        plot: Some(ClipRect {
            x: 0.,
            y: 0.,
            width: 300.,
            height: 160.,
        }),
        slots: vec![PlotWidgetSlot {
            mark_id: "alpha".into(),
            revision: 0,
            placement: PlotWidgetPlacement::AtDatum {
                offset: [-60., -16.],
                measured: TextSize {
                    width: 120.,
                    height: 32.,
                },
            },
        }],
    }
}
fn with_fixture<T>(
    read: impl FnOnce(&Fixture) -> std::result::Result<T, JsValue>,
) -> std::result::Result<T, JsValue> {
    FIXTURE.with(|slot| {
        read(
            slot.borrow()
                .as_ref()
                .ok_or_else(|| js("raster overlay stopped"))?,
        )
    })
}

#[wasm_bindgen]
pub fn mount_raster_overlay_fixture() -> std::result::Result<(), JsValue> {
    super::browser::stop_fixture()?;
    super::typed_surface::stop_typed_surface_fixture()?;
    super::color_fonts::stop_color_fixture()?;
    stop_raster_overlay_fixture()?;
    let root = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id("app"))
        .ok_or_else(|| js("missing app"))?;
    root.set_inner_html("");
    root.remove_attribute("data-gly-app")?;
    let frame = Rc::new(RefCell::new(None));
    let count = Rc::new(Cell::new(0));
    let errors = Rc::new(RefCell::new(vec![]));
    let fail = Rc::new(Cell::new(false));
    let publications = Rc::new(Cell::new(0));
    let factories = Rc::new(Cell::new(0));
    let children_cleaned = Rc::new(Cell::new(0));
    let publisher_drops = Rc::new(Cell::new(0));
    let factory_drops = Rc::new(Cell::new(0));
    let pub_capture = Capture(publisher_drops.clone());
    let factory_capture = Capture(factory_drops.clone());
    let observed_frame = frame.clone();
    let observed_count = count.clone();
    let rejected = errors.clone();
    let failure = fail.clone();
    let published = publications.clone();
    let built = factories.clone();
    let cleaned = children_cleaned.clone();
    let mut inputs = None;
    let web = glory::web::launch_with(root, |scope| {
        let scene = scope.cage(scene(0, 10.).unwrap());
        let options = scope.cage(options(0));
        let hidden = scope.cage(BTreeSet::new());
        inputs = Some((scene, options, hidden));
        glory::ui::view().class("raster-overlay-fixture").fill((
            raster_chart(scene, FontRegistry::new(), options)
                .hidden(hidden)
                .legend(false)
                .data_controls(false)
                .tooltip(false)
                .on_reject(move |e| rejected.borrow_mut().push(e.0))
                .raster_image_source(move |image| {
                    let _ = &pub_capture;
                    published.set(published.get() + 1);
                    if failure.get() {
                        Err(ChartError("raster overlay publication rejected".into()))
                    } else {
                        image.data_url()
                    }
                })
                .plot_widgets(layout(), move |context| {
                    let _ = &factory_capture;
                    built.set(built.get() + 1);
                    *observed_frame.borrow_mut() = Some(context.frame);
                    let frame = context.frame;
                    let count = observed_count.clone();
                    let cleaned = cleaned.clone();
                    Ok(glory::ui::widget(move |scope| {
                        let cleaned = cleaned.clone();
                        scope.on_cleanup(move || cleaned.set(cleaned.get() + 1))?;
                        let local = scope.cage(0u32);
                        let count = count.clone();
                        glory::ui::button()
                            .class("raster-overlay-alpha")
                            .label(scope.bond(move || {
                                frame.with(|f| format!("{} / {}", f.mark.value, local.get()))
                            }))
                            .on(EventKind::Activate, move |_| {
                                local.revise(|n| *n += 1);
                                count.set(local.get_untracked());
                            })
                            .show_in(scope)
                    }))
                }),
            glory::ui::button()
                .class("raster-overlay-toggle")
                .label("Toggle overlay")
                .on(EventKind::Activate, move |_| {
                    hidden.revise(|h| {
                        if !h.remove("alpha") {
                            h.insert("alpha".into());
                        }
                    })
                }),
        ))
    })
    .map_err(js)?;
    let (scene, options, hidden) = inputs.unwrap();
    FIXTURE.with(|slot| {
        *slot.borrow_mut() = Some(Fixture {
            web,
            scene,
            options,
            hidden,
            frame,
            count,
            errors,
            fail,
            publications,
            factories,
            children_cleaned,
            publisher_drops,
            factory_drops,
            cleared: Cell::new(false),
        })
    });
    Ok(())
}

#[wasm_bindgen]
pub fn raster_overlay_phase(phase: u8) -> std::result::Result<(), JsValue> {
    if phase > 2 {
        return Err(js("invalid raster overlay phase"));
    }
    with_fixture(|f| {
        f.fail.set(false);
        f.scene
            .revise(|s| *s = scene(phase, if phase == 0 { 10. } else { 20. }).unwrap());
        f.options.revise(|o| *o = options(phase));
        f.web.app().flush().map_err(js)
    })
}
#[wasm_bindgen]
pub fn raster_overlay_update(value: f64, failure: bool) -> std::result::Result<(), JsValue> {
    if !value.is_finite() {
        return Err(js("finite value required"));
    }
    with_fixture(|f| {
        f.fail.set(failure);
        f.scene.revise(|s| *s = scene(1, value).unwrap());
        f.options.revise(|o| *o = options(1));
        f.web.app().flush().map_err(js)
    })
}
#[wasm_bindgen]
pub fn raster_overlay_hidden(hidden: bool) -> std::result::Result<(), JsValue> {
    with_fixture(|f| {
        f.hidden.revise(|h| {
            if hidden {
                h.insert("alpha".into());
            } else {
                h.remove("alpha");
            }
        });
        f.web.app().flush().map_err(js)
    })
}
#[wasm_bindgen]
pub fn raster_overlay_snapshot() -> String {
    FIXTURE.with(|slot|match slot.borrow().as_ref() {
        Some(f)=> {
            let frame=if f.cleared.get() {None} else {f.frame.borrow().as_ref().map(|frame|frame.with(|frame|json!({
                "value":frame.mark.value,"rect":[frame.rect.x,frame.rect.y,frame.rect.width,frame.rect.height],
                "visible":frame.visible.map(|r|[r.x,r.y,r.width,r.height]),"interactive":frame.interactive,
            })))};
            json!({"active":!f.cleared.get(),"frame":frame,"count":f.count.get(),"errors":f.errors.borrow().clone(),
                "publications":f.publications.get(),"factories":f.factories.get(),"childrenCleaned":f.children_cleaned.get(),
                "publisherDrops":f.publisher_drops.get(),"factoryDrops":f.factory_drops.get(),
                "nodes":f.web.app().stats().nodes,"handlers":f.web.app().stats().handlers}).to_string()
        },
        None=>json!({"active":false}).to_string(),
    })
}
#[wasm_bindgen]
pub fn clear_raster_overlay_fixture() -> std::result::Result<(), JsValue> {
    with_fixture(|f| {
        f.web.app().clear().map_err(js)?;
        f.cleared.set(true);
        f.frame.borrow_mut().take();
        Ok(())
    })
}
#[wasm_bindgen]
pub fn stop_raster_overlay_fixture() -> std::result::Result<(), JsValue> {
    if let Some(f) = FIXTURE.with(|slot| slot.borrow_mut().take()) {
        f.web.app().clear().map_err(js)?;
    }
    Ok(())
}
