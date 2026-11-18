//! One real, application-owned chart. Thumbnails use the same Rust catalog.
#![cfg(target_arch = "wasm32")]
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use kartis::browser::{
    BrowserAnimation, BrowserChart, BrowserEvent, BrowserOptions, BrowserRenderer,
};
use kartis::motion::{PropertyMotionOptions, SpringConfig};
use kartis::*;
use serde_json::json;
use wasm_bindgen::prelude::*;
pub mod catalog;
mod navigation;
mod page;
mod presentation;
mod shell;
#[derive(Clone)]
struct Selection {
    slug: String,
    seed: u32,
    renderer: String,
    animate: bool,
    preset: String,
    theme: String,
    variant: String,
    spacing: u32,
    bump: catalog::BumpControls,
    settings: catalog::Settings,
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
            spacing: 100,
            settings: Default::default(),
            bump: Default::default(),
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
    catalog::configured_sample(
        &selection.slug,
        (width.max(240.), height.max(220.)),
        selection.seed,
        &selection.variant,
        selection.spacing,
        selection.bump,
        &selection.settings,
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
        "molasses" => (280., 120.),
        custom if custom.starts_with("custom/") => {
            let values = custom
                .trim_start_matches("custom/")
                .split_once('/')
                .ok_or_else(|| ChartError("Expected stiffness and damping".into()))?;
            let parse = |value: &str| {
                value
                    .parse::<f64>()
                    .map_err(|_| ChartError("Invalid spring parameter".into()))
            };
            (parse(values.0)?, parse(values.1)?)
        }
        _ => return Err(ChartError("Unknown spring preset".into())),
    };
    let config = SpringConfig {
        stiffness,
        damping,
        ..Default::default()
    };
    config.validate()?;
    Ok(config)
}
#[wasm_bindgen]
pub fn catalog_json() -> String {
    let kinds: Vec<_> = catalog::KINDS.iter().map(|kind| {
        let controls:Vec<_> = catalog::control_groups(kind.slug).into_iter()
            .flat_map(|(group,controls)| controls.iter().map(move |c|json!({
                "group":group, "key":c.key, "label":c.label, "choices":c.choices
            }))).collect();
        let variants:Vec<_> = catalog::variants(kind.slug).iter()
            .map(|(value,label)|json!({"value":value,"label":label})).collect();
        json!({"slug":kind.slug,"name":kind.name,"category":kind.category,"description":kind.description,
            "controls":controls,"variants":variants})
    }).collect();
    json!(kinds).to_string()
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
    layout: &str,
    spacing: u32,
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
    let (variant, settings) = parse_layout(layout)?;
    let config = spring(preset).map_err(js)?;
    let seed = SEEDS.with(|seeds| seeds.borrow().get(slug).copied().unwrap_or(0));
    let selection = Selection {
        slug: slug.into(),
        seed,
        renderer: renderer.into(),
        animate,
        preset: preset.into(),
        theme: theme.into(),
        variant,
        settings,
        spacing,
        bump: SELECTION.with(|selection| selection.borrow().bump),
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
    let mut options = BrowserOptions {
        renderer: render_mode,
        data_controls: false,
        height: Some(height),
        animation: BrowserAnimation::Properties(PropertyMotionOptions {
            spring: config,
            presence_spring: config,
            ..Default::default()
        }),
        ..Default::default()
    };
    presentation::mount_options(&selection.settings, &mut options);
    let chart =
        BrowserChart::mount_with_datum_outline(root, source.borrow().scene.clone(), options)
            .map_err(js)?;
    presentation::set_stage(&selection.settings).map_err(js)?;
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
pub fn replay_entrance() -> std::result::Result<(), JsValue> {
    OWNER.with(|owner| {
        let owner = owner.borrow();
        let owner = owner.as_ref().ok_or_else(|| js("Chart is not mounted"))?;
        // Apply the initial resize before building the entrance from the sized scene.
        owner.chart.flush().map_err(js)?;
        let target = owner.sample.borrow().scene.clone();
        let start = SELECTION.with(|selection| {
            let selection = selection.borrow();
            catalog::entrance_scene(
                &selection.slug,
                &selection.variant,
                selection.spacing,
                &target,
            )
        });
        owner.chart.set_scene(start).map_err(js)?;
        owner.chart.flush().map_err(js)?;
        owner.chart.animate_to(target).map_err(js)
    })
}
#[wasm_bindgen]
pub fn next_dataset() -> std::result::Result<(), JsValue> {
    update_target(|selection| selection.seed = selection.seed.wrapping_add(1))
}
#[wasm_bindgen]
pub fn configure_theme(theme: &str) -> std::result::Result<(), JsValue> {
    if !["light", "dark"].contains(&theme) {
        return Err(js("Unknown theme"));
    }
    update_target(|selection| selection.theme = theme.to_owned())
}
#[wasm_bindgen]
pub fn configure_motion(animate: bool, preset: &str) -> std::result::Result<(), JsValue> {
    let config = spring(preset).map_err(js)?;
    OWNER.with(|owner| {
        let owner = owner.borrow();
        let owner = owner.as_ref().ok_or_else(|| js("Chart is not mounted"))?;
        SELECTION.with(|selection| {
            let mut selection = selection.borrow_mut();
            if preset != selection.preset {
                owner
                    .chart
                    .set_property_springs(config, config)
                    .map_err(js)?;
            }
            if !animate && selection.animate {
                let snapshot = owner.chart.snapshot().map_err(js)?;
                if snapshot.animating || snapshot.pending != 0 {
                    owner
                        .chart
                        .set_scene(owner.sample.borrow().scene.clone())
                        .map_err(js)?;
                }
            }
            selection.animate = animate;
            selection.preset = preset.to_owned();
            Ok(())
        })
    })
}
fn parse_layout(layout: &str) -> std::result::Result<(String, catalog::Settings), JsValue> {
    let (variant, options) = serde_json::from_str(layout).map_err(js)?;
    Ok((variant, catalog::Settings(options)))
}
#[wasm_bindgen]
pub fn configure_chart(
    layout: &str,
    spacing: u32,
    line_width: f64,
    end_labels: bool,
) -> std::result::Result<(), JsValue> {
    let (variant, settings) = parse_layout(layout)?;
    update_target(|selection| {
        selection.variant = variant;
        selection.settings = settings;
        selection.spacing = spacing;
        selection.bump = catalog::BumpControls {
            line_width,
            end_labels,
        };
    })
}
fn update_target(change: impl FnOnce(&mut Selection)) -> std::result::Result<(), JsValue> {
    OWNER.with(|owner| {
        let owner = owner.borrow();
        let owner = owner.as_ref().ok_or_else(|| js("Chart is not mounted"))?;
        let previous = SELECTION.with(|s| s.borrow().clone());
        let mut selection = previous.clone();
        change(&mut selection);
        selection.settings.validate(&selection.slug).map_err(js)?;
        if previous.variant == selection.variant
            && previous.theme == selection.theme
            && previous.spacing == selection.spacing
            && previous.bump == selection.bump
            && previous.seed == selection.seed
            && presentation::only_browser_changes(&previous.settings, &selection.settings)
        {
            presentation::update(&owner.chart, &previous.settings, &selection.settings)
                .map_err(js)?;
            SELECTION.with(|s| *s.borrow_mut() = selection);
            return Ok(());
        }
        let state = owner.chart.snapshot().map_err(js)?;
        let next = sample(&selection, state.width, state.height).map_err(js)?;
        if owner.sample.borrow().scene != next.scene {
            if selection.animate {
                owner.chart.animate_to(next.scene.clone())
            } else {
                owner.chart.set_scene(next.scene.clone())
            }
            .map_err(js)?;
        }
        presentation::update(&owner.chart, &previous.settings, &selection.settings).map_err(js)?;
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
    OWNER.with(|owner|{let owner=owner.borrow();let Some(owner)=owner.as_ref() else{return json!({"active":false,"slug":selection.slug,"seed":selection.seed}).to_string()};match owner.chart.snapshot(){Ok(state)=>json!({"active":state.active,"slug":selection.slug,"seed":selection.seed,"dataset":selection.seed+1,"renderer":selection.renderer,"animate":selection.animate,"preset":selection.preset,"theme":selection.theme,"variant":selection.variant,"settings":selection.settings.0,"animating":state.animating,"animationKind":state.animation_kind.map(|kind|format!("{kind:?}")),"revision":state.revision,"pending":state.pending,"width":state.width,"height":state.height,"marks":owner.sample.borrow().scene.marks.len(),"errors":*owner.errors.borrow()}).to_string(),Err(error)=>json!({"active":false,"error":error.to_string()}).to_string()}})
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
            .map(|owner| SELECTION.with(|selected| {
                let selected = selected.borrow();
                let mut code = String::new();
                if selected.spacing != 100 {
                    let scale = f64::from(selected.spacing) / 100.;
                    code.push_str(&format!("config.margin = Margin {{ top: {:.1}, right: {:.1}, bottom: {:.1}, left: {:.1} }};\n", 24.*scale, 28.*scale, 42.*scale, 48.*scale));
                }
                code.push_str(owner.sample.borrow().code);
                if selected.slug == "bump" {
                    code.push_str(&format!("\noptions.style.width = {:.1};\noptions.labels.end = {};\nbump_layout(&config, &rank_series, &options)?.scene",selected.bump.line_width,selected.bump.end_labels));
                }
                let overrides = catalog::control_code(&selected.slug, &selected.settings);
                if !overrides.is_empty() {
                    code.push_str("\n\n// Selected chart settings\n");
                    code.push_str(&overrides);
                }
                code
            }))
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
