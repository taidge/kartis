use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{Blob, BlobPropertyBag, HtmlAnchorElement, HtmlInputElement, HtmlSelectElement, Url};

use super::*;

pub(super) fn input(id: &str) -> Result<HtmlInputElement> {
    el(id)?.dyn_into().map_err(Into::into)
}
fn select(id: &str) -> Result<HtmlSelectElement> {
    el(id)?.dyn_into().map_err(Into::into)
}
fn layout() -> Result {
    let current = selection();
    let variant = select("variant-select")?.value();
    let spacing = input("plot-spacing")?.value().parse::<u32>().map_err(js)?;
    let mut settings = catalog::Settings::default();
    for control in catalog::controls(&current.slug) {
        let value = select(&format!("chart-option-{}", control.key))?.value();
        if value != control.choices[0].0 {
            settings.0.insert(control.key.into(), value);
        }
    }
    crate::configure_chart(
        &serde_json::json!([variant, settings.0]).to_string(),
        spacing,
        input("bump-width")?.value_as_number(),
        input("bump-labels")?.checked(),
    )?;
    LAYOUTS.with(|cell| {
        cell.borrow_mut().insert(
            current.slug,
            Layout {
                variant,
                spacing,
                settings,
            },
        )
    });
    text("spacing-value", &format!("{spacing}%"))?;
    events::fit_controls()?;
    panels()?;
    sync()
}
fn change_spring(custom: bool) -> Result {
    let selector = select("spring-preset")?;
    if custom {
        selector.set_value("custom");
    }
    let mut preset = selector.value();
    let stiffness = input("spring-stiffness")?;
    let damping = input("spring-damping")?;
    if preset == "custom" {
        preset = format!("custom/{}/{}", stiffness.value(), damping.value());
    } else {
        let spring = crate::spring(&preset).map_err(js)?;
        stiffness.set_value(&spring.stiffness.to_string());
        damping.set_value(&spring.damping.to_string());
    }
    text("stiffness-value", &stiffness.value())?;
    text("damping-value", &damping.value())?;
    crate::configure_motion(selection().animate, &preset)?;
    sync()
}
pub(super) fn change(node: &Element) -> Result {
    match node.id().as_str() {
        "animation-enabled" => {
            crate::configure_motion(input("animation-enabled")?.checked(), &selection().preset)?;
            sync()
        }
        "spring-preset" => change_spring(false),
        "spring-stiffness" | "spring-damping" => change_spring(true),
        "variant-select" | "plot-spacing" | "bump-width" | "bump-labels" => layout(),
        _ if node.has_attribute("data-setting") => layout(),
        _ => Ok(()),
    }
}
pub(super) fn range(id: &str) -> Result {
    let (output, unit) = match id {
        "spring-stiffness" => ("stiffness-value", ""),
        "spring-damping" => ("damping-value", ""),
        "plot-spacing" => ("spacing-value", "%"),
        "bump-width" => ("width-value", " px"),
        _ => return Ok(()),
    };
    text(output, &format!("{}{unit}", input(id)?.value()))
}
fn theme() -> Result {
    let theme = if selection().theme == "light" {
        "dark"
    } else {
        "light"
    };
    let root = document()
        .document_element()
        .ok_or_else(|| js("Missing document root"))?;
    attr(&root, "data-theme", theme)?;
    let button = el("theme-toggle")?;
    attr(
        &button,
        "aria-pressed",
        if theme == "dark" { "true" } else { "false" },
    )?;
    attr(
        &button,
        "aria-label",
        if theme == "dark" {
            "Switch to light theme"
        } else {
            "Switch to dark theme"
        },
    )?;
    if !DETAIL.get() {
        navigation::previews(theme)?;
    }
    crate::configure_theme(theme)?;
    sync()
}
fn download() -> Result {
    let parts = js_sys::Array::new();
    parts.push(&JsValue::from_str(&crate::export_svg()?));
    let options = BlobPropertyBag::new();
    options.set_type("image/svg+xml");
    let blob = Blob::new_with_str_sequence_and_options(&parts, &options)?;
    let url = Url::create_object_url_with_blob(&blob)?;
    let link: HtmlAnchorElement = document().create_element("a")?.unchecked_into();
    link.set_href(&url);
    link.set_download(&format!("kartis-{}.svg", selection().slug));
    link.click();
    events::later(1000, move || Url::revoke_object_url(&url))
}
fn copy(panel: &'static str) -> Result {
    let value = el(&format!("{panel}-output"))?
        .text_content()
        .unwrap_or_default();
    let task = window().navigator().clipboard().write_text(&value);
    spawn_local(async move {
        let id = format!("copy-{panel}");
        if JsFuture::from(task).await.is_ok() {
            let _ = text(&id, "Copied");
            let _ = events::later(1200, move || text(&id, "Copy"));
        } else {
            let _ = text(&id, "Select text to copy");
        }
    });
    Ok(())
}
pub(super) fn click(node: &Element, event: &web_sys::Event) -> Result {
    let jump_target = match node.id().as_str() {
        "skip-to-main" => Some("main"),
        "browse-collection" => Some("collection"),
        "jump-settings" => Some("chart-settings"),
        "jump-preview" => Some("detail-mount"),
        _ => None,
    };
    if let Some(id) = jump_target {
        event.prevent_default();
        let section = html(id)?;
        section.set_tab_index(-1);
        section.focus()?;
        section.scroll_into_view();
        return Ok(());
    }
    if let Some(name) = node.get_attribute("data-tab") {
        return tab(&name);
    }
    if let Some(mode) = node.get_attribute("data-renderer") {
        let mut s = selection();
        if s.renderer != mode {
            s.renderer = mode;
            mount(&s)?;
        }
        let options = web_sys::FocusOptions::new();
        options.set_prevent_scroll(true);
        return node
            .unchecked_ref::<HtmlElement>()
            .focus_with_options(&options);
    }
    match node.id().as_str() {
        "new-data" => shuffle_data(),
        "auto-demo" => events::set_auto(!AUTO.get()),
        "replay-entrance" => {
            crate::replay_entrance()?;
            sync()
        }
        "theme-toggle" | "theme-setting" => theme(),
        "download-svg" => download(),
        "copy-code" => copy("code"),
        "copy-data" => copy("data"),
        _ => Ok(()),
    }
}
