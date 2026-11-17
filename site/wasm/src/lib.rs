//! One real, application-owned chart. Thumbnails use the same Rust catalog.
#![cfg(target_arch = "wasm32")]
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use kartis::browser::{
    BrowserAnimation, BrowserChart, BrowserEvent, BrowserOptions, BrowserRenderer, MotionPreference,
};
use kartis::motion::{PropertyMotionOptions, SpringConfig};
use kartis::*;
use serde_json::json;
use wasm_bindgen::prelude::*;
pub mod catalog;
#[derive(Clone)]
struct Selection {
    slug: String,
    seed: u32,
    renderer: String,
    animate: bool,
    preset: String,
    theme: String,
    variant: String,
}
impl Default for Selection {
    fn default() -> Self {
        Self {
            slug: "line".into(),
            seed: 0,
            renderer: "svg".into(),
            animate: true,
            preset: "default".into(),
            theme: "light".into(),
            variant: String::new(),
        }
    }
}
struct Playground {
    chart: BrowserChart,
    sample: Rc<RefCell<catalog::Sample>>,
    errors: Rc<RefCell<Vec<String>>>,
}
thread_local! {
    static OWNER:RefCell<Option<Playground>>=const{RefCell::new(None)};
    // Author state survives pagehide/BFCache; scoped owners and RAFs do not.
    static SELECTION:Rc<RefCell<Selection>>=Rc::new(RefCell::new(Selection::default()));
    static SEEDS:RefCell<BTreeMap<String,u32>>=const{RefCell::new(BTreeMap::new())};
}
fn js(error: impl ToString) -> JsValue {
    JsValue::from_str(&error.to_string())
}
fn themed(mut sample: catalog::Sample, theme: &str) -> catalog::Sample {
    if theme == "dark" {
        sample.scene.background = Color(24, 34, 32, 255);
        for label in &mut sample.scene.labels {
            if label.color.0 != 255 || label.color.1 != 255 || label.color.2 != 255 {
                label.color = Color(178, 195, 184, label.color.3);
            }
        }
        for mark in &mut sample.scene.marks {
            if let Some(stroke) = &mut mark.stroke {
                let Color(r, g, b, a) = stroke.color;
                if r.abs_diff(g) < 16 && g.abs_diff(b) < 16 {
                    stroke.color = if r > 190 {
                        Color(53, 72, 65, a)
                    } else {
                        Color(143, 166, 150, a)
                    };
                }
            }
        }
    }
    sample
}
fn sample(selection: &Selection, width: f64, height: f64) -> Result<catalog::Sample> {
    catalog::sample(
        &selection.slug,
        width.max(240.),
        height.max(220.),
        selection.seed,
        &selection.variant,
    )
    .map(|s| themed(s, &selection.theme))
}
fn spring(preset: &str) -> Result<SpringConfig> {
    let (stiffness, damping) = match preset {
        "default" => (170., 26.),
        "gentle" => (120., 14.),
        "wobbly" => (180., 12.),
        "stiff" => (210., 20.),
        "slow" => (100., 28.),
        _ => return Err(ChartError("Unknown spring preset".into())),
    };
    Ok(SpringConfig {
        stiffness,
        damping,
        ..Default::default()
    })
}
#[wasm_bindgen]
pub fn catalog_json() -> String {
    json!(catalog::KINDS.iter().map(|kind|json!({"slug":kind.slug,"name":kind.name,"category":kind.category,"description":kind.description,"variants":catalog::variants(kind.slug).iter().map(|(value,label)|json!({"value":value,"label":label})).collect::<Vec<_>>()})).collect::<Vec<_>>()).to_string()
}
#[wasm_bindgen]
pub fn thumbnail_svg(slug: &str, theme: &str) -> std::result::Result<String, JsValue> {
    let variant = catalog::variants(slug).first().map_or("", |v| v.0);
    let source = themed(
        catalog::sample(slug, 360., 230., 0, variant).map_err(js)?,
        theme,
    );
    render::to_svg_with_namespace(&source.scene, &format!("preview-{slug}")).map_err(js)
}
#[wasm_bindgen]
pub fn mount_chart(
    slug: &str,
    renderer: &str,
    animate: bool,
    preset: &str,
    theme: &str,
    variant: &str,
) -> std::result::Result<(), JsValue> {
    if !catalog::KINDS.iter().any(|kind| kind.slug == slug) {
        return Err(js("Unknown chart type"));
    }
    let render_mode = match renderer {
        "svg" => BrowserRenderer::Svg,
        "canvas" => BrowserRenderer::Canvas,
        _ => return Err(js("Unknown renderer")),
    };
    if !["light", "dark"].contains(&theme) {
        return Err(js("Unknown theme"));
    }
    let config = spring(preset).map_err(js)?;
    let seed = SEEDS.with(|seeds| seeds.borrow().get(slug).copied().unwrap_or(0));
    let selection = Selection {
        slug: slug.into(),
        seed,
        renderer: renderer.into(),
        animate,
        preset: preset.into(),
        theme: theme.into(),
        variant: variant.into(),
    };
    let root = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id("chart-stage"))
        .ok_or_else(|| js("Missing chart stage"))?;
    let bounds = root.get_bounding_client_rect();
    let height = (bounds.height() - 54.).clamp(220., 720.);
    let source = sample(&selection, bounds.width(), height).map_err(js)?;
    stop_gallery();
    let source = Rc::new(RefCell::new(source));
    let errors = Rc::new(RefCell::new(Vec::new()));
    let chart = BrowserChart::mount(
        root,
        source.borrow().scene.clone(),
        BrowserOptions {
            renderer: render_mode,
            data_controls: false,
            height: Some(height),
            motion: if animate {
                MotionPreference::System
            } else {
                MotionPreference::Reduced
            },
            animation: BrowserAnimation::Properties(PropertyMotionOptions {
                spring: config,
                presence_spring: config,
                ..Default::default()
            }),
            ..Default::default()
        },
    )
    .map_err(js)?;
    SELECTION.with(|s| *s.borrow_mut() = selection);
    let current = SELECTION.with(Rc::clone);
    let target = source.clone();
    chart.on_resize(move |w, h| {
        let next = sample(&current.borrow(), w, h)?;
        let scene = next.scene.clone();
        *target.borrow_mut() = next;
        Ok(scene)
    });
    let log = errors.clone();
    chart.on_event(move |event| {
        if let BrowserEvent::Error(error) = event {
            let mut log = log.borrow_mut();
            if log.len() == 32 {
                log.remove(0);
            }
            log.push(error.to_string());
        }
    });
    OWNER.with(|owner| {
        *owner.borrow_mut() = Some(Playground {
            chart,
            sample: source,
            errors,
        })
    });
    Ok(())
}
#[wasm_bindgen]
pub fn next_dataset() -> std::result::Result<(), JsValue> {
    OWNER.with(|owner| {
        let owner = owner.borrow();
        let owner = owner.as_ref().ok_or_else(|| js("Chart is not mounted"))?;
        let mut selection = SELECTION.with(|s| s.borrow().clone());
        selection.seed = selection.seed.wrapping_add(1);
        let state = owner.chart.snapshot().map_err(js)?;
        let next = sample(&selection, state.width, state.height).map_err(js)?;
        if selection.animate {
            owner.chart.animate_to(next.scene.clone())
        } else {
            owner.chart.set_scene(next.scene.clone())
        }
        .map_err(js)?;
        *owner.sample.borrow_mut() = next;
        SEEDS.with(|seeds| {
            seeds
                .borrow_mut()
                .insert(selection.slug.clone(), selection.seed);
        });
        SELECTION.with(|s| *s.borrow_mut() = selection);
        Ok(())
    })
}
#[wasm_bindgen]
pub fn gallery_snapshot() -> String {
    let selection = SELECTION.with(|s| s.borrow().clone());
    OWNER.with(|owner|{let owner=owner.borrow();let Some(owner)=owner.as_ref() else{return json!({"active":false,"slug":selection.slug,"seed":selection.seed}).to_string()};match owner.chart.snapshot(){Ok(state)=>json!({"active":state.active,"slug":selection.slug,"seed":selection.seed,"dataset":selection.seed+1,"renderer":selection.renderer,"animate":selection.animate,"preset":selection.preset,"theme":selection.theme,"variant":selection.variant,"animating":state.animating,"animationKind":state.animation_kind.map(|kind|format!("{kind:?}")),"revision":state.revision,"pending":state.pending,"width":state.width,"height":state.height,"marks":owner.sample.borrow().scene.marks.len(),"errors":*owner.errors.borrow()}).to_string(),Err(error)=>json!({"active":false,"error":error.to_string()}).to_string()}})
}
#[wasm_bindgen]
pub fn input_json() -> std::result::Result<String, JsValue> {
    OWNER.with(|owner| {
        owner
            .borrow()
            .as_ref()
            .ok_or_else(|| js("Chart is not mounted"))
            .and_then(|owner| serde_json::to_string_pretty(&owner.sample.borrow().data).map_err(js))
    })
}
#[wasm_bindgen]
pub fn source_code() -> std::result::Result<String, JsValue> {
    OWNER.with(|owner| {
        owner
            .borrow()
            .as_ref()
            .map(|owner| owner.sample.borrow().code.to_owned())
            .ok_or_else(|| js("Chart is not mounted"))
    })
}
#[wasm_bindgen]
pub fn export_svg() -> std::result::Result<String, JsValue> {
    OWNER.with(|owner| {
        owner
            .borrow()
            .as_ref()
            .ok_or_else(|| js("Chart is not mounted"))
            .and_then(|owner| render::to_svg(&owner.sample.borrow().scene).map_err(js))
    })
}
#[wasm_bindgen]
pub fn stop_gallery() {
    let old = OWNER.with(|owner| owner.borrow_mut().take());
    drop(old);
}
