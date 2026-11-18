//! The grouped index and gallery share the catalog, filter and browser route.
use std::cell::RefCell;

use glory::Widget;
use glory::display::{
    Click, Field, Outlet, Template, attr, bind_attr, button, children, class, div, field, on_click,
    span, text, view,
};
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{HtmlElement, HtmlInputElement, KeyboardEvent};

use crate::catalog::{ChartKind, KINDS};
use crate::page;

const ALL: &str = "All charts";
const ITEMS: Outlet = Outlet("items");
const LABEL: Field = Field("label");
const CLASS: Field = Field("class");
const COUNT: Field = Field("count");
const OPEN: Click = Click("open");

struct State {
    category: &'static str,
    query: String,
    current: Option<String>,
    routed_url: Option<String>,
    preview_theme: Option<String>,
}
thread_local! {
    static STATE: RefCell<State> = const { RefCell::new(State {
        category: ALL, query: String::new(), current: None, routed_url: None, preview_theme: None,
    }) };
}

fn families() -> Vec<&'static str> {
    let mut groups = vec![ALL];
    for chart in KINDS {
        if !groups.contains(&chart.category) {
            groups.push(chart.category);
        }
    }
    groups
}

fn checked(result: Result<(), JsValue>) {
    if let Err(error) = result {
        page::report(error);
    }
}

fn chart_id(prefix: &str, chart: &ChartKind) -> String {
    format!("{prefix}-{}", chart.slug)
}

pub(crate) fn categories() -> impl Widget {
    static FILTERS: Template = Template::new(&[div(
        &[
            attr("id", "category-list"),
            class("categories"),
            attr("role", "group"),
            attr("aria-label", "Filter by chart family"),
            children(ITEMS),
        ],
        &[],
    )]);
    static FILTER: Template = Template::new(&[button(
        &[
            attr("type", "button"),
            bind_attr("class", CLASS),
            attr("aria-pressed", "false"),
            on_click(OPEN),
        ],
        &[
            span(&[], &[field(LABEL)]),
            span(&[class("category-count")], &[field(COUNT)]),
        ],
    )]);
    let filters = families()
        .into_iter()
        .enumerate()
        .map(|(i, category)| {
            let count = KINDS
                .iter()
                .filter(|chart| category == ALL || chart.category == category)
                .count();
            view(&FILTER)
                .bind(CLASS, format!("filter-{i}"))
                .bind(LABEL, category)
                .bind(COUNT, count)
                .on_click(OPEN, move || checked(choose_category(category)))
        })
        .collect::<Vec<_>>();
    view(&FILTERS).child(ITEMS, filters)
}

pub(crate) fn sidebar() -> impl Widget {
    static NAV: Template = Template::new(&[div(
        &[
            attr("id", "chart-nav"),
            attr("role", "navigation"),
            attr("aria-label", "Chart types"),
            children(ITEMS),
        ],
        &[],
    )]);
    static GROUP: Template = Template::new(&[div(
        &[bind_attr("class", CLASS)],
        &[
            div(
                &[
                    class("nav-heading"),
                    attr("role", "heading"),
                    attr("aria-level", "3"),
                ],
                &[field(LABEL)],
            ),
            glory::display::outlet(ITEMS),
        ],
    )]);
    static ITEM: Template = Template::new(&[button(
        &[
            attr("type", "button"),
            bind_attr("class", CLASS),
            attr("aria-current", "false"),
            on_click(OPEN),
        ],
        &[field(LABEL)],
    )]);
    let groups = families()
        .into_iter()
        .enumerate()
        .skip(1)
        .map(|(i, category)| {
            let items = KINDS
                .iter()
                .filter(|chart| chart.category == category)
                .map(|chart| {
                    view(&ITEM)
                        .bind(CLASS, chart_id("nav", chart))
                        .bind(LABEL, chart.name)
                        .on_click(OPEN, move || {
                            checked(page::navigate(Some(chart.slug), true))
                        })
                })
                .collect::<Vec<_>>();
            view(&GROUP)
                .bind(CLASS, format!("nav-group family-{i}"))
                .bind(LABEL, category)
                .child(ITEMS, items)
        })
        .collect::<Vec<_>>();
    view(&NAV).child(ITEMS, groups)
}

pub(crate) fn cards() -> impl Widget {
    static GRID: Template = Template::new(&[div(
        &[
            attr("id", "catalog-grid"),
            class("catalog-grid"),
            attr("aria-label", "Chart gallery"),
            children(ITEMS),
        ],
        &[],
    )]);
    static CARD: Template = Template::new(&[button(
        &[
            attr("type", "button"),
            bind_attr("class", CLASS),
            bind_attr("aria-label", Field("accessible")),
            on_click(OPEN),
        ],
        &[
            span(&[class("thumbnail")], &[]),
            span(
                &[class("card-meta")],
                &[
                    span(&[class("card-category")], &[field(Field("category"))]),
                    span(
                        &[class("card-heading")],
                        &[
                            span(
                                &[
                                    class("card-title"),
                                    attr("role", "heading"),
                                    attr("aria-level", "3"),
                                ],
                                &[field(LABEL)],
                            ),
                            span(
                                &[class("card-arrow"), attr("aria-hidden", "true")],
                                &[text("↗")],
                            ),
                        ],
                    ),
                    span(&[class("card-description")], &[field(Field("description"))]),
                ],
            ),
        ],
    )]);
    let cards = KINDS
        .iter()
        .map(|chart| {
            view(&CARD)
                .bind(CLASS, format!("catalog-card card-{}", chart.slug))
                .bind(Field("accessible"), format!("Explore {} chart", chart.name))
                .bind(Field("category"), chart.category)
                .bind(LABEL, chart.name)
                .bind(Field("description"), chart.description)
                .on_click(OPEN, move || {
                    checked(page::navigate(Some(chart.slug), true))
                })
        })
        .collect::<Vec<_>>();
    view(&GRID).child(ITEMS, cards)
}

fn document() -> web_sys::Document {
    web_sys::window()
        .expect("browser window")
        .document()
        .expect("browser document")
}
fn element(id: &str) -> Result<HtmlElement, JsValue> {
    document()
        .get_element_by_id(id)
        .and_then(|e| e.dyn_into().ok())
        .ok_or_else(|| crate::js(format!("Missing navigation element #{id}")))
}
// Repeated widget instances use a stable class binding in Glory's display profile.
fn keyed(key: &str) -> Result<HtmlElement, JsValue> {
    document()
        .get_elements_by_class_name(key)
        .item(0)
        .and_then(|e| e.dyn_into().ok())
        .ok_or_else(|| crate::js(format!("Missing catalog widget .{key}")))
}
fn show(node: HtmlElement, visible: bool) {
    if node.hidden() == visible {
        node.set_hidden(!visible);
    }
}
fn hidden(id: &str, hide: bool) -> Result<(), JsValue> {
    let node = element(id)?;
    if hide != node.hidden() {
        node.set_hidden(hide);
    }
    Ok(())
}

fn filter() -> Result<(), JsValue> {
    let (category, query) = STATE.with_borrow(|s| (s.category, s.query.clone()));
    let mut count = 0;
    let mut matched = Vec::new();
    for chart in KINDS {
        let searchable = format!(
            "{} {} {} {}",
            chart.name, chart.slug, chart.category, chart.description
        );
        let visible = (category == ALL || category == chart.category)
            && searchable.to_lowercase().contains(&query);
        show(keyed(&chart_id("card", chart))?, visible);
        show(keyed(&chart_id("nav", chart))?, visible);
        if visible {
            count += 1;
            if !matched.contains(&chart.category) {
                matched.push(chart.category);
            }
        }
    }
    for (i, name) in families().into_iter().enumerate() {
        keyed(&format!("filter-{i}"))?.set_attribute(
            "aria-pressed",
            if category == name { "true" } else { "false" },
        )?;
        if i > 0 {
            show(keyed(&format!("family-{i}"))?, matched.contains(&name));
        }
    }
    element("result-count")?.set_text_content(Some(&count.to_string()));
    element("category-label")?.set_text_content(Some(&format!("{category} ")));
    hidden("no-results", count != 0)
}

fn to_gallery_if_needed() -> Result<(), JsValue> {
    if STATE.with_borrow(|state| state.current.is_some()) {
        page::navigate(None, true)?;
    }
    Ok(())
}
fn choose_category(category: &'static str) -> Result<(), JsValue> {
    STATE.with_borrow_mut(|s| s.category = category);
    to_gallery_if_needed()?;
    filter()?;
    set_menu(false)
}

fn drawer() -> Result<web_sys::MediaQueryList, JsValue> {
    web_sys::window()
        .unwrap()
        .match_media("(max-width: 900px)")?
        .ok_or_else(|| crate::js("Cannot observe navigation viewport"))
}
fn set_menu(open: bool) -> Result<(), JsValue> {
    let show = open && drawer()?.matches();
    element("sidebar")?
        .class_list()
        .toggle_with_force("open", show)?;
    hidden("nav-backdrop", !show)?;
    element("menu-toggle")?.set_attribute("aria-expanded", if show { "true" } else { "false" })
}
fn close_and_focus() -> Result<(), JsValue> {
    set_menu(false)?;
    element("menu-toggle")?.focus()
}

pub(crate) fn previews(theme: &str) -> Result<(), JsValue> {
    if STATE.with_borrow(|s| s.preview_theme.as_deref() == Some(theme)) {
        return Ok(());
    }
    for chart in KINDS {
        let preview = keyed(&chart_id("card", chart))?
            .query_selector(".thumbnail")?
            .ok_or_else(|| crate::js("Missing chart preview container"))?;
        // SVG is generated by Kartis from the catalog's own typed scene.
        preview.set_inner_html(&crate::thumbnail_svg(chart.slug, theme)?);
        let svg = preview
            .first_element_child()
            .ok_or_else(|| crate::js("Missing preview SVG"))?;
        svg.set_attribute("aria-hidden", "true")?;
        svg.set_attribute("focusable", "false")?;
    }
    STATE.with_borrow_mut(|s| s.preview_theme = Some(theme.to_owned()));
    Ok(())
}

pub(crate) fn selected(slug: Option<&str>, push_history: bool) -> Result<(), JsValue> {
    let previous =
        STATE.with_borrow_mut(|s| std::mem::replace(&mut s.current, slug.map(str::to_owned)));
    if previous.as_deref() != slug {
        if let Some(old) = previous {
            keyed(&format!("nav-{old}"))?.set_attribute("aria-current", "false")?;
        }
        if let Some(chart) = slug {
            keyed(&format!("nav-{chart}"))?.set_attribute("aria-current", "true")?;
        }
    }
    set_menu(false)?;
    if let Some(chart) = slug {
        let nav = element("chart-nav")?;
        let panel = nav.get_bounding_client_rect();
        let active = keyed(&format!("nav-{chart}"))?.get_bounding_client_rect();
        if active.top() < panel.top() {
            nav.set_scroll_top(nav.scroll_top() + (active.top() - panel.top()).floor() as i32 - 8);
        } else if active.bottom() > panel.bottom() {
            nav.set_scroll_top(
                nav.scroll_top() + (active.bottom() - panel.bottom()).ceil() as i32 + 8,
            );
        }
    }
    let window = web_sys::window().unwrap();
    let location = window.location();
    if push_history {
        let hash = slug.map_or(String::new(), |s| format!("#chart/{s}"));
        if location.hash()? != hash {
            let url = format!("{}{}{hash}", location.pathname()?, location.search()?);
            window
                .history()?
                .push_state_with_url(&JsValue::NULL, "", Some(&url))?;
        }
    }
    let href = location.href()?;
    STATE.with_borrow_mut(|s| s.routed_url = Some(href));
    Ok(())
}

/// Both history events may fire for a single back/forward traversal.
pub(crate) fn route() -> Result<(), JsValue> {
    let location = web_sys::window().unwrap().location();
    let href = location.href()?;
    if STATE.with_borrow(|s| s.routed_url.as_deref() == Some(&href)) {
        return Ok(());
    }
    let hash = location.hash()?;
    let slug = hash.strip_prefix("#chart/").filter(|slug| !slug.is_empty());
    let Some(requested) = slug else {
        return page::navigate(None, false);
    };
    let decoded = js_sys::decode_uri_component(requested)
        .ok()
        .map(String::from);
    if let Some(chart) = KINDS
        .iter()
        .find(|chart| Some(chart.slug) == decoded.as_deref())
    {
        return page::navigate(Some(chart.slug), false);
    }
    // A stale bookmark still opens a usable gallery, including on the first boot.
    // Replace this history entry so Back does not reopen the invalid location.
    let gallery = format!("{}{}", location.pathname()?, location.search()?);
    web_sys::window()
        .unwrap()
        .history()?
        .replace_state_with_url(&JsValue::NULL, "", Some(&gallery))?;
    page::navigate(None, false)
}

pub(crate) fn install() -> Result<(), JsValue> {
    // The Glory web template profile reserves data attributes for its renderer.
    // Publish the stable chart keys on the already-mounted widget elements.
    for chart in KINDS {
        for prefix in ["nav", "card"] {
            keyed(&chart_id(prefix, chart))?.set_attribute("data-slug", chart.slug)?;
        }
    }
    for (i, category) in families().into_iter().enumerate() {
        keyed(&format!("filter-{i}"))?.set_attribute("data-category", category)?;
    }
    element("type-count")?.set_text_content(Some(&KINDS.len().to_string()));
    filter()?;
    let search = element("search")?.dyn_into::<HtmlInputElement>()?;
    let search_value = search.clone();
    page::listen(search.as_ref(), "input", move |_| {
        STATE.with_borrow_mut(|s| s.query = search_value.value().trim().to_lowercase());
        to_gallery_if_needed()?;
        filter()?;
        if drawer()?.matches() {
            set_menu(true)?;
            search_value.focus()?;
        }
        Ok(())
    })?;
    page::listen(element("menu-toggle")?.as_ref(), "click", |_| {
        let open = !element("sidebar")?.class_list().contains("open");
        set_menu(open)?;
        if open {
            element("search")?.focus()?;
        }
        Ok(())
    })?;
    page::listen(element("nav-backdrop")?.as_ref(), "click", |_| {
        close_and_focus()
    })?;
    page::listen(drawer()?.as_ref(), "change", |_| set_menu(false))?;
    page::listen(element("hero-open")?.as_ref(), "click", |_| {
        page::navigate(Some("line"), true)
    })?;
    page::listen(element("back-to-gallery")?.as_ref(), "click", |_| {
        page::navigate(None, true)
    })?;
    let window = web_sys::window().unwrap();
    page::listen(window.as_ref(), "keydown", |raw| {
        let event = raw.dyn_into::<KeyboardEvent>()?;
        match event.key().as_str() {
            "/" if !document().active_element().is_some_and(|e| {
                matches!(e.tag_name().as_str(), "INPUT" | "TEXTAREA" | "SELECT")
                    || e.dyn_ref::<HtmlElement>()
                        .is_some_and(HtmlElement::is_content_editable)
            }) =>
            {
                event.prevent_default();
                set_menu(true)?;
                element("search")?.focus()?;
            }
            "Escape" if element("sidebar")?.class_list().contains("open") => close_and_focus()?,
            _ => {}
        }
        Ok(())
    })?;
    for name in ["popstate", "hashchange"] {
        page::listen(window.as_ref(), name, |_| route())?;
    }
    Ok(())
}
