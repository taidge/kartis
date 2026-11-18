use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;

use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{Document, Element, HtmlElement, Window};

use crate::{SELECTION, Selection, catalog, js, navigation, shell};

mod controls;
mod events;
pub(crate) use events::listen;

type Result<T = ()> = std::result::Result<T, JsValue>;

#[derive(Clone)]
struct Layout {
    variant: String,
    spacing: u32,
    settings: catalog::Settings,
}

thread_local! {
    static LAYOUTS: RefCell<BTreeMap<String, Layout>> = const { RefCell::new(BTreeMap::new()) };
    static DETAIL: Cell<bool> = const { Cell::new(false) };
    static SUSPENDED: Cell<bool> = const { Cell::new(false) };
    static AUTO: Cell<bool> = const { Cell::new(false) };
    static STAGE_HEIGHT: Cell<f64> = const { Cell::new(0.0) };
}

fn window() -> Window {
    web_sys::window().expect("browser window")
}
fn document() -> Document {
    window().document().expect("browser document")
}
fn el(id: &str) -> Result<Element> {
    document()
        .get_element_by_id(id)
        .ok_or_else(|| js(format!("Missing element #{id}")))
}
fn html(id: &str) -> Result<HtmlElement> {
    el(id)?.dyn_into().map_err(Into::into)
}
fn selection() -> Selection {
    SELECTION.with(|s| s.borrow().clone())
}
fn attr(node: &Element, key: &str, value: &str) -> Result {
    if node.get_attribute(key).as_deref() != Some(value) {
        node.set_attribute(key, value)?;
    }
    Ok(())
}
fn text(id: &str, value: &str) -> Result {
    let node = el(id)?;
    if node.text_content().as_deref() != Some(value) {
        node.set_text_content(Some(value));
    }
    Ok(())
}
fn hidden(id: &str, hide: bool) -> Result {
    html(id)?.set_hidden(hide);
    Ok(())
}
fn toggle_class(node: &Element, class: &str, on: bool) -> Result {
    node.class_list().toggle_with_force(class, on)?;
    Ok(())
}
fn apply_nodes(selector: &str, mut apply: impl FnMut(Element) -> Result) -> Result {
    let nodes = document().query_selector_all(selector)?;
    for i in 0..nodes.length() {
        if let Some(node) = nodes.item(i) {
            apply(node.unchecked_into())?;
        }
    }
    Ok(())
}

pub(super) fn report(error: JsValue) {
    let _ = events::set_auto(false);
    web_sys::console::error_1(&error);
    if let Ok(node) = html("error") {
        node.set_hidden(false);
        let message = error.as_string().unwrap_or_else(|| format!("{error:?}"));
        node.set_text_content(Some(&format!("Unable to update the chart. {message}")));
    }
}
fn panels() -> Result {
    text("code-output", &crate::source_code()?)?;
    text("data-output", &crate::input_json()?)
}
fn tab(name: &str) -> Result {
    apply_nodes("[data-tab]", |node| {
        attr(
            &node,
            "aria-selected",
            if node.get_attribute("data-tab").as_deref() == Some(name) {
                "true"
            } else {
                "false"
            },
        )
    })?;
    for panel in ["chart", "code", "data"] {
        hidden(&format!("panel-{panel}"), panel != name)?;
    }
    if name != "chart" {
        panels()?;
    }
    Ok(())
}
fn reduced() -> bool {
    window()
        .match_media("(prefers-reduced-motion: reduce)")
        .ok()
        .flatten()
        .is_some_and(|query| query.matches())
}
fn sync() -> Result {
    if SUSPENDED.get() {
        return Ok(());
    }
    let Some((frame, marks, error)) = crate::OWNER.with(|cell| -> Result<_> {
        let owner = cell.borrow();
        let Some(owner) = owner.as_ref() else {
            return Ok(None);
        };
        Ok(Some((
            owner.chart.snapshot().map_err(js)?,
            owner.sample.borrow().scene.marks.len(),
            owner.errors.borrow().last().cloned(),
        )))
    })?
    else {
        return Ok(());
    };
    if let Some(error) = error {
        return Err(js(error));
    }
    let s = selection();
    let root = document()
        .document_element()
        .ok_or_else(|| js("Missing document root"))?;
    let dataset = s.seed + 1;
    for (key, value) in [
        ("data-ready", "true"),
        ("data-renderer", s.renderer.as_str()),
        ("data-chart", s.slug.as_str()),
        (
            "data-animating",
            if frame.animating { "true" } else { "false" },
        ),
        ("data-dataset", &dataset.to_string()),
    ] {
        attr(&root, key, value)?;
    }
    text("dataset-status", &format!("Dataset {dataset}"))?;
    let reduced_motion = reduced();
    let instant = reduced_motion || !s.animate;
    let replay = el("replay-entrance")?.unchecked_into::<web_sys::HtmlButtonElement>();
    if replay.disabled() != instant {
        replay.set_disabled(instant);
    }
    toggle_class(&el("motion-status")?, "moving", frame.animating)?;
    let motion = if frame.animating {
        "Spring in motion"
    } else if instant {
        "Instant updates"
    } else {
        "Spring at rest"
    };
    text("motion-text", motion)?;
    text(
        "mark-count",
        &format!(
            "{marks} rendered marks · {} × {}",
            frame.width.round(),
            frame.height.round()
        ),
    )?;
    for mode in ["svg", "canvas"] {
        attr(
            &el(&format!("renderer-{mode}"))?,
            "aria-pressed",
            if s.renderer == mode { "true" } else { "false" },
        )?;
    }
    if let Some(node) = document().query_selector(".render-label")? {
        let value = format!("{} · RUST / WASM", s.renderer.to_uppercase());
        if node.text_content().as_deref() != Some(&value) {
            node.set_text_content(Some(&value));
        }
    }
    text(
        "motion-note",
        if reduced_motion {
            "Reduced motion is enabled on your device. Data updates instantly."
        } else {
            "Adjust the spring or replay the entrance to see how the chart moves."
        },
    )
}

fn mount(s: &Selection) -> Result {
    hidden("error", true)?;
    STAGE_HEIGHT.set(el("chart-stage")?.get_bounding_client_rect().height());
    crate::mount_chart(
        &s.slug,
        &s.renderer,
        s.animate,
        &s.preset,
        &s.theme,
        &serde_json::json!([s.variant, s.settings.0]).to_string(),
        s.spacing,
    )?;
    events::observe_controls()?;
    panels()?;
    sync()
}
pub(super) fn navigate(slug: Option<&str>, push: bool) -> Result {
    let detail = slug.is_some();
    let slug = slug.unwrap_or("line");
    let kind = catalog::KINDS
        .iter()
        .find(|kind| kind.slug == slug)
        .ok_or_else(|| js(format!("Unknown chart {slug}")))?;
    let mut s = selection();
    let entering = detail && (!DETAIL.get() || s.slug != slug);
    if detail {
        events::set_auto(false)?;
    }
    DETAIL.set(detail);
    s.slug = slug.into();
    let layout = if detail {
        LAYOUTS.with(|cell| cell.borrow().get(slug).cloned())
    } else {
        None
    };
    let layout = layout.unwrap_or_else(|| Layout {
        variant: catalog::variants(slug).first().map_or("", |v| v.0).into(),
        spacing: 100,
        settings: Default::default(),
    });
    s.variant = layout.variant;
    s.spacing = layout.spacing;
    s.settings = layout.settings;
    hidden("catalog-view", detail)?;
    hidden("detail-view", !detail)?;
    let stage = el("stage-shell")?;
    el(if detail { "detail-mount" } else { "hero-mount" })?.append_child(&stage)?;
    toggle_class(&stage, "overview-stage", !detail)?;
    if detail {
        text("chart-title", kind.name)?;
        text("chart-category", kind.category)?;
        text("chart-description", kind.description)?;
        hidden("map-note", !matches!(slug, "choropleth" | "geo-map"))?;
        shell::configure(slug, &s.variant, &s.settings)?;
        controls::input("plot-spacing")?.set_value(&s.spacing.to_string());
        text("spacing-value", &format!("{}%", s.spacing))?;
    } else {
        navigation::previews(&s.theme)?;
    }
    tab("chart")?;
    mount(&s)?;
    if entering && s.animate && !reduced() {
        crate::replay_entrance()?;
    }
    navigation::selected(detail.then_some(slug), push)?;
    text(
        "status",
        if detail {
            kind.name
        } else {
            "Explore the chart collection"
        },
    )?;
    if detail {
        window().scroll_to_with_x_and_y(0., 0.);
    }
    Ok(())
}

#[wasm_bindgen]
pub fn open_chart(slug: &str) -> Result {
    navigate(Some(slug), true)
}
#[wasm_bindgen]
pub fn open_gallery() -> Result {
    navigate(None, true)
}
#[wasm_bindgen]
pub fn pause_updates() -> Result {
    events::set_auto(false)
}
#[wasm_bindgen]
pub fn shuffle_data() -> Result {
    crate::next_dataset()?;
    panels()?;
    sync()
}

#[wasm_bindgen]
pub fn start_showcase() -> Result {
    shell::mount(el("showcase")?)?;
    navigation::install()?;
    events::install()?;
    navigation::route()?;
    toggle_class(&el("status")?, "ready", true)?;
    if !DETAIL.get() && !reduced() {
        events::start_home_updates()?;
    }
    Ok(())
}
