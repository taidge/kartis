use glory::web::WebApp;
use kartis::{render::raster::*, surface::SurfaceInput, *};
use serde_json::json;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use wasm_bindgen::prelude::*;

struct Fixture {
    web: WebApp,
    surface: RasterSurface,
    fail: Rc<Cell<bool>>,
    publications: Rc<Cell<u32>>,
}
thread_local! { static FIXTURE: RefCell<Option<Fixture>> = const { RefCell::new(None) }; }
fn js(error: impl std::fmt::Display) -> JsValue {
    error.to_string().into()
}

fn layout(phase: u8) -> Result<SceneInteraction> {
    let mut scene = ChartConfig {
        width: 300.,
        height: 150.,
        ..Default::default()
    }
    .scene()?;
    for (id, label, x, value, color) in [
        (
            "a",
            "Alpha",
            65. + f64::from(phase) * 25.,
            0.,
            if phase == 0 {
                Color(210, 40, 50, 255)
            } else {
                Color(30, 90, 190, 255)
            },
        ),
        ("b", "Beta", 170., 7., Color(20, 150, 70, 255)),
        (
            "decoration",
            "Decoration",
            250.,
            12.,
            Color(80, 80, 80, 255),
        ),
    ] {
        let mut mark = Mark::new(
            id,
            Geometry::Circle {
                x,
                y: 55.,
                radius: 20.,
            },
            color,
        );
        mark.label = label.into();
        mark.series = id.into();
        mark.value = value;
        scene.marks.push(mark);
    }
    let binding = InteractionBinding {
        sites: scene.marks[..2]
            .iter()
            .map(|mark| InteractionSite {
                mark_id: mark.id.clone(),
                series: mark.series.clone(),
                label: format!("source {}", mark.label),
                value: if mark.id == "a" && phase == 0 {
                    None
                } else {
                    Some(mark.value)
                },
                formatted_value: Some(format!("raw {}", mark.value)),
                color: mark.fill.unwrap(),
                position: if let Geometry::Circle { x, y, .. } = mark.geometry {
                    [x, y]
                } else {
                    unreachable!()
                },
            })
            .collect(),
        ..Default::default()
    };
    let layout = SceneInteraction {
        scene,
        interaction: Some(binding),
        plot: Some(ClipRect {
            x: 10.,
            y: 10.,
            width: 280.,
            height: 130.,
        }),
    };
    layout.validate()?;
    Ok(layout)
}

#[wasm_bindgen]
pub fn mount_typed_surface_fixture() -> std::result::Result<(), JsValue> {
    super::browser::stop_fixture()?;
    stop_typed_surface_fixture()?;
    let root = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id("app"))
        .ok_or_else(|| js("missing app"))?;
    root.set_inner_html("");
    root.remove_attribute("data-gly-app")?;
    let fail = Rc::new(Cell::new(false));
    let failure = fail.clone();
    let publications = Rc::new(Cell::new(0));
    let published = publications.clone();
    let mut installed = None;
    let web = glory::web::launch_with(root, |scope| {
        let surface = RasterSurface::mount_layout_published(
            scope,
            layout(0).expect("valid typed fixture layout"),
            FontRegistry::new(),
            RasterSurfaceOptions {
                highlight: false,
                tooltip: TooltipOptions {
                    missing: MissingValue::Placeholder("missing".into()),
                    ..Default::default()
                },
                ..Default::default()
            },
            move |image| {
                published.set(published.get() + 1);
                if failure.get() {
                    Err(ChartError("typed fixture publication rejected".into()))
                } else {
                    image.data_url()
                }
            },
        )
        .expect("typed fixture raster admission");
        let widget = surface.widget();
        installed = Some(surface);
        widget
    })
    .map_err(js)?;
    FIXTURE.with(|slot| {
        *slot.borrow_mut() = Some(Fixture {
            web,
            surface: installed.expect("mounted surface"),
            fail,
            publications,
        })
    });
    Ok(())
}
fn with_fixture<T>(
    read: impl FnOnce(&Fixture) -> std::result::Result<T, JsValue>,
) -> std::result::Result<T, JsValue> {
    FIXTURE.with(|slot| {
        read(
            slot.borrow()
                .as_ref()
                .ok_or_else(|| js("typed surface stopped"))?,
        )
    })
}
#[wasm_bindgen]
pub fn typed_surface_update(phase: u8, failure: bool) -> std::result::Result<bool, JsValue> {
    with_fixture(|f| {
        f.fail.set(failure);
        let result = f.surface.handle().set_layout(layout(phase).map_err(js)?);
        f.web.app().flush().map_err(js)?;
        Ok(result.is_ok())
    })
}
#[wasm_bindgen]
pub fn typed_surface_reject_binding() -> std::result::Result<bool, JsValue> {
    with_fixture(|f| {
        let mut invalid = layout(1).map_err(js)?;
        invalid.interaction.as_mut().unwrap().sites[0].position[0] += 1.;
        let rejected = f.surface.handle().set_layout(invalid).is_err();
        f.web.app().flush().map_err(js)?;
        Ok(rejected)
    })
}
#[wasm_bindgen]
pub fn typed_surface_pointer(x: f64, y: f64, bounds: Vec<f64>) -> std::result::Result<(), JsValue> {
    with_fixture(|f| {
        if bounds.len() != 4 {
            return Err(js("four bounds required"));
        }
        f.surface
            .handle()
            .dispatch_host(
                SurfaceInput::Move {
                    id: 1,
                    point: (x, y),
                },
                RasterSurfaceBounds {
                    x: bounds[0],
                    y: bounds[1],
                    width: bounds[2],
                    height: bounds[3],
                },
            )
            .map_err(js)?;
        f.web.app().flush().map_err(js)
    })
}
#[wasm_bindgen]
pub fn typed_surface_snapshot() -> std::result::Result<String, JsValue> {
    with_fixture(|f| {
        let handle = f.surface.handle();
        let s = handle.snapshot().map_err(js)?;
        let active = handle.active_interaction().map_err(js)?;
        Ok(json!({ "revision":s.revision,"selected":s.selected,"hovered":s.hovered,
            "publications":f.publications.get(),"nodes":f.web.app().stats().nodes,
            "interaction":active.map(|a| json!({"ids":a.ids,"anchor":a.anchor,"rows":a.rows.iter().map(|r| json!({"id":r.mark_id,"value":r.value,"formatted":r.formatted_value,"label":r.label})).collect::<Vec<_>>()})) }).to_string())
    })
}
#[wasm_bindgen]
pub fn stop_typed_surface_fixture() -> std::result::Result<(), JsValue> {
    if let Some(f) = FIXTURE.with(|slot| slot.borrow_mut().take()) {
        f.web.app().clear().map_err(js)?;
    }
    Ok(())
}
