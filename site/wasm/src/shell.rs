use std::cell::RefCell;

use glory::Widget;
use glory::render::{SelectEntry, SelectOption, SelectProps, SelectSelection};
use glory::ui::{self, FieldPart, Role, SemanticId, Semantics, TextKind};
use glory::web::WebApp;
use wasm_bindgen::JsValue;
use web_sys::{Document, Element};

use crate::{catalog, navigation};

thread_local! {
    static SHELL: RefCell<Option<WebApp>> = const { RefCell::new(None) };
    static CONFIGURATION: RefCell<Option<(WebApp, WebApp)>> = const { RefCell::new(None) };
}

const DOCS: &str = "https://docs.rs/kartis/0.2.0/kartis/";
const REPO: &str = "https://github.com/taidge/kartis";
const SPRINGS: &[(&str, &str)] = &[
    ("default", "Default · 170 / 26"),
    ("gentle", "Gentle · 120 / 14"),
    ("wobbly", "Wobbly · 180 / 12"),
    ("stiff", "Stiff · 210 / 20"),
    ("slow", "Slow · 100 / 28"),
    ("molasses", "Molasses · 280 / 120"),
    ("custom", "Custom"),
];

fn id(key: &str) -> SemanticId {
    SemanticId::new(key).expect("showcase IDs are valid ASCII tokens")
}

fn identified(key: &str) -> Option<Box<Semantics>> {
    (!key.is_empty()).then(|| {
        Box::new(Semantics {
            id: Some(id(key)),
            ..Default::default()
        })
    })
}

fn block(key: &str, class: &str) -> ui::View {
    ui::view()
        .semantics(identified(key))
        .class(class.to_owned())
}

fn text(key: &str, content: &str) -> ui::Text {
    ui::text()
        .content(content.to_owned())
        .semantics(identified(key))
        .selectable(true)
}

fn heading(level: u8, key: &str, content: &str) -> ui::Text {
    text(key, content).kind(TextKind::Heading(level))
}

fn paragraph(key: &str, content: &str) -> ui::Text {
    text(key, content).kind(TextKind::Paragraph)
}

fn button(key: &str, class: &str, content: &str) -> ui::Button {
    ui::button()
        .semantics(identified(key))
        .class(class.to_owned())
        .label(content.to_owned())
}

fn link(key: &str, class: &str, target: &str, content: &str) -> ui::Link {
    ui::link()
        .target(target.to_owned())
        .semantics(identified(key))
        .class(class.to_owned())
        .label(content.to_owned())
}

fn label(control: &str) -> ui::View {
    ui::view().semantics(Some(Box::new(Semantics {
        field_part: Some(FieldPart::Label),
        label_for: Some(id(control)),
        ..Default::default()
    })))
}

fn picker(key: &str, choices: &[(&str, &str)], selected: &str) -> ui::Select {
    let mut entries = Vec::with_capacity(choices.len());
    let mut keys = Vec::new();
    for (index, &(value, caption)) in choices.iter().enumerate() {
        let key = id(&format!("choice-{index}"));
        if value == selected {
            keys.push(key.clone());
        }
        entries.push(SelectEntry::Option(SelectOption {
            key,
            value: value.into(),
            label: caption.into(),
            disabled: false,
        }));
    }
    ui::select()
        .semantics(identified(key))
        .configuration(SelectProps {
            entries: entries.into(),
            selection: SelectSelection { keys, revision: 0 },
            ..Default::default()
        })
}

fn toolbar() -> impl Widget {
    block("", "topbar").role(Role::Banner).fill((
        block("", "topbar-start").fill((
            button("menu-toggle", "icon-button mobile-only", "☰").name("Open chart navigation"),
            link("", "brand", "#", "").name("Kartis home").fill((
                ui::image().source("/favicon.svg").decorative(),
                text("", "Kartis"),
            )),
            text("", "v0.2.0").class("version"),
        )),
        ui::view()
            .role(Role::Navigation)
            .name("Main navigation")
            .fill((
                link("", "", DOCS, "Documentation ↗"),
                link("", "", REPO, "GitHub ↗"),
                button("theme-toggle", "icon-button", "◐").name("Switch to dark theme"),
            )),
    ))
}

fn sidebar() -> impl Widget {
    block("sidebar", "sidebar").name("Chart navigation").fill((
        block("", "sidebar-head").fill((
            block("", "index-heading").fill((text("", "Chart index"), text("type-count", "35"))),
            block("", "search-box").fill((
                text("", "").class("lens"),
                ui::input()
                    .purpose(ui::Purpose::Search)
                    .name("Search chart types")
                    .semantics(Some(Box::new(Semantics {
                        id: Some(id("search")),
                        placeholder: Some("Find a chart".into()),
                        ..Default::default()
                    }))),
                text("", "/").kind(TextKind::Kbd),
            )),
        )),
        navigation::sidebar(),
    ))
}

fn hero_title() -> impl Widget {
    use glory::display::{Template, attr, element, span, text, view};
    static TITLE: Template = Template::new(&[element(
        "h1",
        &[attr("id", "page-title")],
        &[
            text("Chart"),
            span(&[attr("class", "hero-em")], &[text("explorer.")]),
        ],
    )]);
    view(&TITLE)
}

fn home() -> impl Widget {
    let description = format!(
        "Explore {} chart types for Rust and Glory. Change the data and see how each chart moves in SVG or Canvas.",
        catalog::KINDS.len()
    );
    block("catalog-view", "").role(Role::Section).fill((
        block("", "live-hero").fill((
            block("", "hero-copy").fill((
                hero_title(),
                paragraph("", &description),
                button("browse-collection", "primary-button", "Browse charts ↓"),
            )),
            block("", "live-example")
                .role(Role::Section)
                .name("Live line chart")
                .fill((
                    block("", "example-head").fill((
                        heading(2, "", "Live line chart"),
                        button("hero-open", "text-button", "Open playground ↗"),
                    )),
                    block("hero-mount", ""),
                )),
        )),
        block("collection", "").role(Role::Section).fill((
            block("category-title", "catalog-heading").fill((
                heading(2, "category-label", "All charts"),
                text("result-count", "35"),
            )),
            navigation::categories(),
            navigation::cards(),
            paragraph(
                "no-results",
                "No charts match your search. Try another name or category.",
            )
            .class("empty-state"),
        )),
    ))
}

fn check(key: &str, caption: &str) -> ui::Check {
    ui::check()
        .class("toggle-row")
        .checked(true)
        .label(caption.to_owned())
        .semantics(identified(key))
}

struct Slider {
    key: &'static str,
    caption: &'static str,
    output: &'static str,
    value: &'static str,
    initial_text: &'static str,
    min: &'static str,
    max: &'static str,
    step: &'static str,
}

const WIDTH: Slider = Slider {
    key: "bump-width",
    caption: "Line width",
    output: "width-value",
    value: "2",
    initial_text: "2 px",
    min: "1",
    max: "12",
    step: "0.5",
};
const STIFFNESS: Slider = Slider {
    key: "spring-stiffness",
    caption: "Stiffness",
    output: "stiffness-value",
    value: "170",
    initial_text: "170",
    min: "80",
    max: "320",
    step: "1",
};
const DAMPING: Slider = Slider {
    key: "spring-damping",
    caption: "Damping",
    output: "damping-value",
    value: "26",
    initial_text: "26",
    min: "10",
    max: "120",
    step: "1",
};
const SPACING: Slider = Slider {
    key: "plot-spacing",
    caption: "Plot margins",
    output: "spacing-value",
    value: "100",
    initial_text: "100%",
    min: "50",
    max: "150",
    step: "5",
};

impl Slider {
    fn widget(&self) -> impl Widget + use<> {
        ui::fragment((
            label(self.key).fill((
                text("", self.caption),
                text(self.output, self.initial_text).class("range-output"),
            )),
            ui::input()
                .value(self.value)
                .semantics(identified(self.key)),
        ))
    }
}

fn settings() -> impl Widget {
    block("chart-settings", "settings")
        .name("Chart settings")
        .fill((
            block("", "settings-heading").fill((
                heading(2, "", "Chart settings"),
                button("jump-preview", "text-button", "Back to chart ↑"),
            )),
            block("", "settings-grid").fill((
                block("", "properties-section").role(Role::Section).fill((
                    heading(3, "presentation-title", "Presentation"),
                    block("variant-field", ""),
                    block("bump-controls", "")
                        .fill((WIDTH.widget(), check("bump-labels", "End labels"))),
                    block("chart-controls", ""),
                )),
                block("", "secondary-settings").fill((
                    ui::view().role(Role::Section).fill((
                        heading(3, "motion-title", "Motion"),
                        check("animation-enabled", "Animate updates"),
                        label("spring-preset").fill(text("", "Spring preset")),
                        picker("spring-preset", SPRINGS, "default"),
                        STIFFNESS.widget(),
                        DAMPING.widget(),
                        button("replay-entrance", "small-button", "Replay entrance"),
                        paragraph("motion-note", ""),
                    )),
                    ui::view().role(Role::Section).fill((
                        heading(3, "frame-title", "Display & export"),
                        SPACING.widget(),
                        block("", "property-row").fill((
                            text("", "Theme"),
                            button("theme-setting", "small-button", "Paper ↔ Midnight"),
                        )),
                        button("download-svg", "download-button", "↓ Download SVG"),
                    )),
                )),
            )),
        ))
}

fn detail() -> impl Widget {
    block("detail-view", "").role(Role::Section).fill((
        block("", "detail-breadcrumb").fill((
            button("back-to-gallery", "text-button", "← All charts"),
            text("", "/"),
            text("chart-category", ""),
        )),
        block("", "detail-heading").fill((
            ui::view().fill((
                heading(1, "chart-title", "Line"),
                paragraph("chart-description", ""),
                block("map-note", "").fill((
                    text("", "World sample · 175 non-Antarctic regions · "),
                    link("", "", "/THIRD_PARTY_NOTICES.txt", "Data attribution ↗"),
                )),
            )),
            link(
                "catalog-source",
                "source-link",
                "https://github.com/taidge/kartis/tree/main/site/wasm/src/catalog",
                "Sample source ↗",
            ),
        )),
        block("", "detail-layout").fill((
            block("", "plot-column").fill((
                block("detail-mount", ""),
                block("", "detail-foot").fill((
                    text("mark-count", ""),
                    button("jump-settings", "text-button", "Chart settings ↓"),
                )),
            )),
            settings(),
        )),
    ))
}

fn source_panel(panel: &str, title: &str) -> impl Widget + use<> {
    block(&format!("panel-{panel}"), "source-panel")
        .role(Role::TabPanel)
        .fill((
            block("", "source-caption").fill((
                text("", title),
                button(&format!("copy-{panel}"), "text-button", "Copy"),
            )),
            text(&format!("{panel}-output"), "")
                .kind(TextKind::Code)
                .class("source-code"),
        ))
}

fn stage() -> impl Widget {
    block("stage-shell", "stage-shell overview-stage").fill((
        block("", "stage-toolbar").fill((
            block("", "plot-tabs")
                .role(Role::TabList)
                .name("Playground panels")
                .fill((
                    button("tab-chart", "", "Chart"),
                    button("tab-code", "", "Code"),
                    button("tab-data", "", "Data"),
                )),
            block("", "stage-actions").fill((
                block("", "renderer-switch")
                    .role(Role::Group)
                    .name("Chart renderer")
                    .fill((
                        button("renderer-svg", "", "SVG"),
                        button("renderer-canvas", "", "Canvas"),
                    )),
                button("new-data", "small-button", "↻ New data"),
                button("auto-demo", "small-button play-button", "Auto-update data"),
            )),
        )),
        block("panel-chart", "stage-panel")
            .role(Role::TabPanel)
            .fill(block("chart-stage", "").name("Interactive Kartis chart")),
        source_panel("code", "Core layout call · Rust"),
        source_panel("data", "Input data · JSON"),
        block("", "stage-status").fill((
            block("motion-status", "").fill((
                text("", "").class("motion-dot"),
                text("motion-text", "Ready"),
            )),
            text("dataset-status", "Dataset 1"),
            text("", "SVG · Rust / WASM").class("render-label"),
        )),
    ))
}

fn page() -> impl Widget {
    ui::fragment((
        link("skip-to-main", "skip-link", "#main", "Skip to charts"),
        toolbar(),
        sidebar(),
        block("nav-backdrop", ""),
        block("main", "").role(Role::Main).fill((
            block("error", "error").role(Role::Alert),
            paragraph("status", "Loading the chart explorer…").class("boot-status"),
            home(),
            detail(),
            ui::view().role(Role::ContentInfo).fill((
                link("", "footer-brand", "#", "Kartis"),
                link("", "", DOCS, "Documentation ↗"),
            )),
        )),
        stage(),
    ))
}

fn err(error: impl ToString) -> JsValue {
    JsValue::from_str(&error.to_string())
}

fn by_id(doc: &Document, key: &str) -> Result<Element, JsValue> {
    doc.get_element_by_id(key)
        .ok_or_else(|| err(format!("Missing showcase element {key}")))
}

fn attributes(doc: &Document, key: &str, pairs: &[(&str, &str)]) -> Result<(), JsValue> {
    let element = by_id(doc, key)?;
    for &(name, value) in pairs {
        element.set_attribute(name, value)?;
    }
    Ok(())
}

fn remove_placeholders(root: &Element) -> Result<(), JsValue> {
    while let Some(option) = root.query_selector("option[data-gly-placeholder]")? {
        option.remove();
    }
    Ok(())
}

pub(crate) fn mount(root: Element) -> Result<(), JsValue> {
    let doc = root
        .owner_document()
        .ok_or_else(|| err("Showcase needs a document"))?;
    let app = glory::web::launch_with(root.clone(), |_| page()).map_err(err)?;
    app.app().pump().map_err(err)?;
    for key in [
        "error",
        "nav-backdrop",
        "detail-view",
        "map-note",
        "no-results",
        "bump-controls",
        "panel-code",
        "panel-data",
    ] {
        attributes(&doc, key, &[("hidden", "")])?;
    }
    for (key, attrs) in [
        (
            "menu-toggle",
            &[("aria-expanded", "false"), ("aria-controls", "sidebar")][..],
        ),
        ("theme-toggle", &[("aria-pressed", "false")]),
        ("search", &[("autocomplete", "off")]),
        ("sidebar", &[("role", "complementary")]),
        ("chart-settings", &[("role", "complementary")]),
        ("status", &[("role", "status"), ("aria-live", "polite")]),
        ("catalog-view", &[("aria-labelledby", "page-title")]),
        ("collection", &[("aria-labelledby", "category-label")]),
        ("detail-view", &[("aria-labelledby", "chart-title")]),
        ("new-data", &[("title", "Generate the next dataset")]),
        (
            "auto-demo",
            &[
                ("aria-pressed", "false"),
                ("title", "Automatically update the data every 2.8 seconds"),
            ],
        ),
    ] {
        attributes(&doc, key, attrs)?;
    }
    if let Some(brand) = root.query_selector(".brand img")? {
        brand.set_attribute("width", "34")?;
        brand.set_attribute("height", "34")?;
    }
    for key in ["chart", "code", "data"] {
        let button = by_id(&doc, &format!("tab-{key}"))?;
        button.set_attribute("role", "tab")?;
        button.set_attribute("data-tab", key)?;
        button.set_attribute("aria-controls", &format!("panel-{key}"))?;
        button.set_attribute(
            "aria-selected",
            if key == "chart" { "true" } else { "false" },
        )?;
    }
    for renderer in ["svg", "canvas"] {
        attributes(
            &doc,
            &format!("renderer-{renderer}"),
            &[
                ("data-renderer", renderer),
                (
                    "aria-pressed",
                    if renderer == "svg" { "true" } else { "false" },
                ),
            ],
        )?;
    }
    for slider in [&WIDTH, &STIFFNESS, &DAMPING, &SPACING] {
        attributes(
            &doc,
            slider.key,
            &[
                ("type", "range"),
                ("min", slider.min),
                ("max", slider.max),
                ("step", slider.step),
                ("value", slider.value),
            ],
        )?;
    }
    for key in ["animation-enabled", "bump-labels"] {
        let element = by_id(&doc, key)?;
        if element.tag_name() == "LABEL" {
            let input = element
                .query_selector("input")?
                .ok_or_else(|| err("Checkbox lacks a native input"))?;
            element.remove_attribute("id")?;
            input.set_attribute("id", key)?;
        }
        // The browser exposes the live checked property to assistive technology.
        by_id(&doc, key)?.remove_attribute("aria-checked")?;
    }
    remove_placeholders(&root)?;
    let variant =
        glory::web::launch(by_id(&doc, "variant-field")?, ui::fragment(())).map_err(err)?;
    let controls =
        glory::web::launch(by_id(&doc, "chart-controls")?, ui::fragment(())).map_err(err)?;
    CONFIGURATION.with(|slot| slot.replace(Some((variant, controls))));
    SHELL.with(|slot| slot.replace(Some(app)));
    Ok(())
}

fn groups(slug: &str, settings: &catalog::Settings) -> impl Widget + use<> {
    let mut groups = Vec::new();
    for (name, controls) in catalog::control_groups(slug) {
        if controls.is_empty() {
            continue;
        }
        let mut fields: Vec<Box<dyn Widget>> = vec![Box::new(
            ui::view()
                .field_part(FieldPart::Legend)
                .fill(text("", name)),
        )];
        for control in controls {
            let key = format!("chart-option-{}", control.key);
            let selected = settings
                .0
                .get(control.key)
                .map_or(control.choices[0].0, String::as_str);
            fields.push(Box::new(ui::view().fill((
                label(&key).fill(text("", control.label)),
                picker(&key, control.choices, selected),
            ))));
        }
        groups.push(
            ui::view()
                .field_part(FieldPart::Fieldset)
                .class("chart-properties")
                .fill(fields),
        );
    }
    ui::fragment(groups)
}

pub(crate) fn configure(
    slug: &str,
    variant: &str,
    settings: &catalog::Settings,
) -> Result<(), JsValue> {
    let doc = web_sys::window()
        .and_then(|w| w.document())
        .ok_or_else(|| err("Showcase needs a document"))?;
    let choices = catalog::variants(slug);
    CONFIGURATION.with(|slot| {
        let slot = slot.borrow();
        let (variants, controls) = slot
            .as_ref()
            .ok_or_else(|| err("Showcase settings are not mounted"))?;
        variants.app().clear().map_err(err)?;
        variants
            .app()
            .mount(ui::fragment((
                label("variant-select").fill(text("", "Layout / shape")),
                picker("variant-select", choices, variant),
            )))
            .map_err(err)?;
        controls.app().clear().map_err(err)?;
        controls.app().mount(groups(slug, settings)).map_err(err)?;
        Ok::<_, JsValue>(())
    })?;
    for (key, hidden) in [
        ("bump-controls", slug != "bump"),
        ("variant-field", choices.is_empty()),
    ] {
        let el = by_id(&doc, key)?;
        if hidden {
            el.set_attribute("hidden", "")?;
        } else {
            el.remove_attribute("hidden")?;
        }
    }
    let controls = by_id(&doc, "chart-controls")?;
    remove_placeholders(&by_id(&doc, "variant-field")?)?;
    remove_placeholders(&controls)?;
    for control in catalog::controls(slug) {
        attributes(
            &doc,
            &format!("chart-option-{}", control.key),
            &[("data-setting", control.key)],
        )?;
    }
    for (group, entries) in catalog::control_groups(slug) {
        if let Some(first) = entries.first() {
            let field = by_id(&doc, &format!("chart-option-{}", first.key))?;
            if let Some(set) = field.closest("fieldset")? {
                set.set_attribute("data-group", group)?;
            }
        }
    }
    Ok(())
}
