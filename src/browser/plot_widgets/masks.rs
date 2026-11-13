use std::fmt::Write;

use super::*;

/// SVG object-box masks are scoped to one browser chart. The normalized
/// coordinates follow the CSS surface independently of either axis's resize.
pub(super) struct MaskLayer {
    pub owner: Rc<str>,
    defs: Element,
    style: Element,
    accepted: RefCell<(String, String)>,
}

impl MaskLayer {
    pub fn mount(root: &Element) -> Result<(Self, Element)> {
        let document = root
            .owner_document()
            .ok_or_else(|| ChartError("plot Widget layer has no document".into()))?;
        let owner = Rc::<str>::from(svg::owner_namespace(&document)?);
        let content = document.create_element("div").map_err(host)?;
        let style = document.create_element("style").map_err(host)?;
        let svg = document
            .create_element_ns(Some("http://www.w3.org/2000/svg"), "svg")
            .map_err(host)?;
        let defs = document
            .create_element_ns(Some("http://www.w3.org/2000/svg"), "defs")
            .map_err(host)?;
        for (name, value) in [
            ("id", owner.as_ref()),
            ("width", "0"),
            ("height", "0"),
            ("aria-hidden", "true"),
            ("style", "position:absolute;pointer-events:none"),
        ] {
            svg.set_attribute(name, value).map_err(host)?;
        }
        svg.append_child(&defs).map_err(host)?;
        root.append_child(&svg).map_err(host)?;
        root.append_child(&style).map_err(host)?;
        root.append_child(&content).map_err(host)?;
        Ok((
            Self {
                owner,
                defs,
                style,
                accepted: RefCell::new(Default::default()),
            },
            content,
        ))
    }

    pub fn publish(&self, rows: &[PreparedSlot]) {
        let mut definitions = String::new();
        let mut rules = String::new();
        for (index, [sx, sy, tx, ty], clip) in rows.iter().filter_map(PreparedSlot::path_mask) {
            // Only the generated owner/index, validated numbers, path commands
            // and fixed rule names enter this markup. Datum IDs are excluded.
            let class = format!("{}-slot-{index}", self.owner);
            let rule = render::clip_rule(clip.rule);
            let path = render::path_data(&clip.geometry.commands());
            write!(
                definitions,
                "<clipPath id=\"{class}-clip\" clipPathUnits=\"objectBoundingBox\"><path transform=\"matrix({sx} 0 0 {sy} {tx} {ty})\" clip-rule=\"{rule}\" fill-rule=\"{rule}\" d=\"{path}\"/></clipPath>"
            )
            .expect("String writes are infallible");
            writeln!(rules, ".{class}{{clip-path:url(#{class}-clip)}}")
                .expect("String writes are infallible");
        }
        let mut accepted = self.accepted.borrow_mut();
        if definitions != accepted.0 {
            self.defs.set_inner_html(&definitions);
            accepted.0 = definitions;
        }
        if rules != accepted.1 {
            self.style.set_text_content(Some(&rules));
            accepted.1 = rules;
        }
    }
}
