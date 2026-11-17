use std::fmt::Write;

use super::*;

/// SVG object-box masks are scoped to one browser chart. The normalized
/// coordinates follow the CSS surface independently of either axis's resize.
pub(super) struct MaskLayer {
    pub owner: Rc<str>,
    root: Element,
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
                root: root.clone(),
                defs,
                style,
                accepted: RefCell::new(Default::default()),
            },
            content,
        ))
    }

    pub fn publish(&self, rows: &[PreparedSlot]) -> Result<()> {
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
        let mut host_size = None;
        for (index, frame, relative_to) in rows.iter().filter_map(PreparedSlot::label_style) {
            let class = format!("{}-label-slot-{index}", self.owner);
            // The carrier is passive. Application children may explicitly opt
            // into pointer events without becoming built-in datum controls.
            writeln!(
                rules,
                ".{class}>.kartis-plot-widget-slot{{pointer-events:none!important}}"
            )
            .expect("String writes are infallible");
            // Keep Label content in its exact local pixel box even when its
            // Scene pose is identity. Nested percentages magnify layout-unit
            // rounding for a partially clipped Label (notably in WebKit).
            // The public frame still reports no Scene transform in this case.
            let transform = frame.transform.unwrap_or(SceneWidgetTransform {
                matrix: [1., 0., 0., 1., frame.rect.x, frame.rect.y],
                measured: TextSize {
                    width: frame.rect.width,
                    height: frame.rect.height,
                },
            });
            let scale = match relative_to {
                Some(source) => {
                    let size = match host_size {
                        Some(size) => size,
                        None => {
                            let size = self.computed_size()?;
                            host_size = Some(size);
                            size
                        }
                    };
                    [size[0] / source[0], size[1] / source[1]]
                }
                None => [1., 1.],
            };
            let [a, b, c, d, e, f] = transform.matrix;
            let visible = frame.visible.unwrap_or(frame.rect);
            let [a, b, c, d, e, f] = [
                a * scale[0],
                b * scale[1],
                c * scale[0],
                d * scale[1],
                (e - visible.x) * scale[0],
                (f - visible.y) * scale[1],
            ];
            scene::ensure(
                [a, b, c, d, e, f].iter().all(|v| v.is_finite()),
                "label component CSS transform is not finite",
            )?;
            // The child lays out in its caller-measured local pixel box. The
            // source/view matrix then maps that box into the clip wrapper.
            // Computed CSS dimensions exclude ancestor CSS transforms: those
            // naturally apply once, after this matrix, rather than twice.
            writeln!(
                rules,
                ".{class}>.kartis-plot-widget-slot{{left:0px!important;top:0px!important;width:{}px!important;height:{}px!important;transform-origin:0 0;transform:matrix({a},{b},{c},{d},{e},{f})}}",
                transform.measured.width, transform.measured.height,
            )
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
        Ok(())
    }

    fn computed_size(&self) -> Result<[f64; 2]> {
        let window = self
            .root
            .owner_document()
            .and_then(|document| document.default_view())
            .ok_or_else(|| ChartError("label component host has no window".into()))?;
        let style = window
            .get_computed_style(&self.root)
            .map_err(host)?
            .ok_or_else(|| ChartError("label component host has no computed style".into()))?;
        let mut size = [0.; 2];
        for (index, property) in ["width", "height"].into_iter().enumerate() {
            let value = style.get_property_value(property).map_err(host)?;
            size[index] = value
                .trim()
                .strip_suffix("px")
                .and_then(|value| value.parse::<f64>().ok())
                .filter(|value| value.is_finite() && *value >= 0.)
                .ok_or_else(|| {
                    ChartError("label component host requires resolved CSS dimensions".into())
                })?;
        }
        Ok(size)
    }
}
