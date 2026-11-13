//! Single-line HTML baseline measurements for ordinary Glory Text labels.
use super::*;
use wasm_bindgen::JsValue;

#[wasm_bindgen::prelude::wasm_bindgen]
extern "C" {
    #[wasm_bindgen(extends = web_sys::CanvasRenderingContext2d)]
    type DirectionCanvas;
    #[wasm_bindgen(method, setter, js_name = direction)]
    fn set_direction(this: &DirectionCanvas, value: &str);
}

/// Owns one invisible, noninteractive HTML line and one reusable Canvas context.
/// Text is assigned through `textContent`. Baselines and advances come from HTML
/// layout; Canvas only supplies conservative alphabetic ink bounds.
///
/// Wait for the document's fonts before measuring and increment the semantic
/// profile revision after font resources or inherited text styles change. The
/// probe is removed on Drop. Named families may use the browser's normal fallback;
/// this adapter does not discover or load font assets on the application's behalf.
pub struct BrowserSemanticTextMeasurer {
    document: web_sys::Document,
    row: HtmlElement,
    text: HtmlElement,
    marker: HtmlElement,
    axes: [HtmlElement; 4],
    canvas: web_sys::CanvasRenderingContext2d,
}

fn metric_host(error: JsValue) -> ChartError {
    ChartError(format!("HTML text measurement failed: {error:?}"))
}

fn css_pixels(style: &web_sys::CssStyleDeclaration, property: &str) -> Result<f64> {
    let value = style.get_property_value(property).map_err(metric_host)?;
    let pixels = value
        .trim()
        .strip_suffix("px")
        .and_then(|number| number.parse::<f64>().ok());
    scene::ensure(
        pixels.is_some_and(|pixels| pixels.is_finite() && pixels >= 0.),
        "HTML text used dimensions are unavailable",
    )?;
    Ok(pixels.unwrap())
}

fn none(value: &str) -> bool {
    value.is_empty() || value == "none"
}

fn axis_transform(style: &web_sys::CssStyleDeclaration) -> Result<()> {
    let value = style.get_property_value("transform").map_err(metric_host)?;
    if !none(value.trim()) {
        let entries = value
            .trim()
            .strip_prefix("matrix(")
            .and_then(|v| v.strip_suffix(')'))
            .map(|v| {
                v.split(',')
                    .map(|number| number.trim().parse::<f64>())
                    .collect::<std::result::Result<Vec<_>, _>>()
            });
        scene::ensure(
            entries.as_ref().is_some_and(|entries| {
                entries.as_ref().is_ok_and(|v| {
                    v.len() == 6
                        && v.iter().all(|v| v.is_finite())
                        && v[0] > 0.
                        && v[3] > 0.
                        && v[1] == 0.
                        && v[2] == 0.
                })
            }),
            "HTML text measurement supports positive axis-aligned 2D transforms only",
        )?;
    }
    let rotate = style.get_property_value("rotate").map_err(metric_host)?;
    scene::ensure(
        none(rotate.trim()) || matches!(rotate.trim(), "0deg" | "0rad" | "0grad" | "0turn"),
        "HTML text measurement does not support ancestor rotation",
    )?;
    let scale = style.get_property_value("scale").map_err(metric_host)?;
    if !none(scale.trim()) {
        let scale = scale
            .split_whitespace()
            .map(|part| {
                if let Some(number) = part.strip_suffix('%') {
                    number.parse::<f64>().map(|n| n / 100.)
                } else {
                    part.parse::<f64>()
                }
            })
            .collect::<std::result::Result<Vec<_>, _>>();
        scene::ensure(
            scale.as_ref().is_ok_and(|v| {
                (1..=3).contains(&v.len())
                    && v.iter().all(|v| v.is_finite() && *v > 0.)
                    && (v.len() < 3 || v[2] == 1.)
            }),
            "HTML text measurement does not support reflected, collapsed or 3D scale",
        )?;
    }
    let translate = style.get_property_value("translate").map_err(metric_host)?;
    if !none(translate.trim()) {
        let parts = translate.split_whitespace().collect::<Vec<_>>();
        scene::ensure(
            parts.len() <= 2 || parts.len() == 3 && matches!(parts[2], "0" | "0px" | "0%"),
            "HTML text measurement does not support 3D translation",
        )?;
    }
    scene::ensure(
        none(
            style
                .get_property_value("perspective")
                .map_err(metric_host)?
                .trim(),
        ),
        "HTML text measurement does not support perspective",
    )?;
    scene::ensure(
        none(
            style
                .get_property_value("offset-path")
                .map_err(metric_host)?
                .trim(),
        ),
        "HTML text measurement does not support CSS motion paths",
    )?;
    let zoom = style.get_property_value("zoom").map_err(metric_host)?;
    let zoom_value = zoom
        .trim()
        .strip_suffix('%')
        .map(|v| v.parse::<f64>().map(|v| v / 100.))
        .unwrap_or_else(|| zoom.trim().parse::<f64>());
    scene::ensure(
        matches!(zoom.trim(), "" | "normal") || zoom_value.is_ok_and(|v| v.is_finite() && v > 0.),
        "HTML text measurement requires positive CSS zoom",
    )
}

fn validate_transform_ancestry(document: &web_sys::Document, row: &HtmlElement) -> Result<()> {
    let window = document
        .default_view()
        .ok_or_else(|| ChartError("text measurement document has no window".into()))?;
    let root = document
        .document_element()
        .ok_or_else(|| ChartError("text measurement document has no root".into()))?;
    let mut node: Element = row.clone().unchecked_into();
    for _ in 0..256 {
        let style = window
            .get_computed_style(&node)
            .map_err(metric_host)?
            .ok_or_else(|| ChartError("text measurement ancestor style unavailable".into()))?;
        axis_transform(&style)?;
        if let Some(slot) = node.assigned_slot() {
            node = slot.unchecked_into();
        } else if let Some(parent) = node.parent_element() {
            node = parent;
        } else if let Ok(shadow) = node.get_root_node().dyn_into::<web_sys::ShadowRoot>() {
            node = shadow.host();
        } else {
            return scene::ensure(
                node.is_same_node(Some(root.as_ref())),
                "HTML text measurement requires inspectable document ancestry",
            );
        }
    }
    Err(ChartError("HTML text ancestor budget exceeded".into()))
}

impl BrowserSemanticTextMeasurer {
    /// Create a probe in the document body. Use `new_in` when the chart inherits
    /// text styles from a more specific application container.
    pub fn new(document: &web_sys::Document) -> Result<Self> {
        let parent = document
            .body()
            .ok_or_else(|| ChartError("text measurement document has no body".into()))?;
        Self::new_in(&parent)
    }

    /// The chart and probe should inherit the same portable host text styles.
    /// The parent must remain connected and participate in layout while measuring.
    /// Positive axis-aligned 2D scaling and translation return local CSS metrics.
    /// Positive CSS zoom and inspectable composed Shadow DOM ancestry are
    /// supported. Rotation, skew, reflection, perspective and 3D are rejected
    /// instead of returning screen-space values.
    pub fn new_in(parent: &HtmlElement) -> Result<Self> {
        scene::ensure(parent.is_connected(), "text measurement parent is detached")?;
        let document = parent
            .owner_document()
            .ok_or_else(|| ChartError("text measurement parent has no document".into()))?;
        let make = |name| {
            document
                .create_element(name)
                .map_err(metric_host)?
                .dyn_into::<HtmlElement>()
                .map_err(|_| ChartError("text measurement HTML element unavailable".into()))
        };
        let row = make("div")?;
        row.set_class_name("kartis-semantic-text-probe");
        row.set_attribute("aria-hidden", "true")
            .map_err(metric_host)?;
        row.set_attribute("inert", "").map_err(metric_host)?;
        row.set_attribute("style", "position:absolute;left:-1000000px;top:0;display:inline-block;visibility:visible;opacity:0;pointer-events:none;width:max-content;height:auto;min-width:0;max-width:none;margin:0;padding:0;border:0;letter-spacing:0;white-space:nowrap;text-transform:none;text-indent:0")
            .map_err(metric_host)?;
        let text = make("span")?;
        text.set_attribute(
            "style",
            "display:inline-block;vertical-align:baseline;margin:0;padding:0;border:0;letter-spacing:0;white-space:nowrap",
        )
        .map_err(metric_host)?;
        let marker = make("span")?;
        marker.set_attribute("style", "display:inline-block;vertical-align:baseline;width:0;height:0;margin:0;padding:0;border:0;line-height:0;font-size:0")
            .map_err(metric_host)?;
        row.append_child(&text).map_err(metric_host)?;
        row.append_child(&marker).map_err(metric_host)?;
        let axes = [make("span")?, make("span")?, make("span")?, make("span")?];
        for (axis, [x, y]) in axes.iter().zip([[0, 0], [100, 0], [0, 100], [100, 100]]) {
            axis.set_attribute("style", &format!("position:absolute;display:block;left:{x}px;top:{y}px;width:0;height:0;margin:0;padding:0;border:0;font-size:0;line-height:0")).map_err(metric_host)?;
            row.append_child(axis).map_err(metric_host)?;
        }
        let canvas = document
            .create_element("canvas")
            .map_err(metric_host)?
            .dyn_into::<HtmlCanvasElement>()
            .map_err(|_| ChartError("text measurement Canvas unavailable".into()))?
            .get_context("2d")
            .map_err(metric_host)?
            .ok_or_else(|| ChartError("text measurement Canvas 2D unavailable".into()))?
            .dyn_into::<web_sys::CanvasRenderingContext2d>()
            .map_err(|_| ChartError("text measurement Canvas 2D type unavailable".into()))?;
        parent.append_child(&row).map_err(metric_host)?;
        Ok(Self {
            document,
            row,
            text,
            marker,
            axes,
            canvas,
        })
    }

    /// Measure a bounded request against the current, fully loaded font resources.
    /// The response retains the complete request, including its resource revision.
    pub fn measure(&self, request: &SemanticTextRequest) -> Result<SemanticTextMeasurement> {
        request.validate()?;
        scene::ensure(
            self.row.is_connected(),
            "text measurement probe is detached",
        )?;
        validate_transform_ancestry(&self.document, &self.row)?;
        // Confirm the actual composed geometry too. This catches a hidden
        // closed-slot transform that DOM ancestor getters cannot expose.
        // Four zero-size points must map to one positively oriented rectangle;
        // no angle/scale estimate or arbitrary error tolerance is involved.
        let [origin, x, y, xy] = self
            .axes
            .each_ref()
            .map(|point| point.get_bounding_client_rect());
        scene::ensure(
            x.left() > origin.left()
                && y.top() > origin.top()
                && x.top() == origin.top()
                && y.left() == origin.left()
                && xy.left() == x.left()
                && xy.top() == y.top(),
            "HTML text composed geometry is not a measurable positive 2D axis transform",
        )?;
        let fonts = self.document.fonts();
        scene::ensure(
            fonts.status() == web_sys::FontFaceSetLoadStatus::Loaded,
            "text measurement font resources are loading",
        )?;
        for element in [&self.row, &self.text] {
            let style = element.style();
            for (name, value) in [
                ("font-family", request.font.css_family()),
                ("font-weight", request.font.weight.to_string()),
                ("font-size", format!("{}px", request.size)),
                ("line-height", format!("{}px", request.line_height)),
            ] {
                style.set_property(name, &value).map_err(metric_host)?;
            }
        }
        self.text.set_text_content(Some(&request.text));
        let span = self.text.get_bounding_client_rect();
        let marker = self.marker.get_bounding_client_rect();
        let computed = self
            .document
            .default_view()
            .ok_or_else(|| ChartError("text measurement document has no window".into()))?
            .get_computed_style(&self.text)
            .map_err(metric_host)?
            .ok_or_else(|| ChartError("text measurement style is unavailable".into()))?;
        let slant = computed
            .get_property_value("font-style")
            .map_err(metric_host)?;
        // Independent formatting-context used dimensions stay in the local CSS
        // coordinate system even when an ancestor transforms its screen bounds.
        let advance = css_pixels(&computed, "width")?;
        let line_height = css_pixels(&computed, "height")?;
        let scale_y = if line_height > 0. {
            span.height() / line_height
        } else {
            1.
        };
        scene::ensure(
            scale_y.is_finite() && scale_y > 0.,
            "HTML text positive scale cannot be measured",
        )?;
        let font = format!("{slant} {}", request.font.canvas_font(request.size));
        scene::ensure(
            fonts.status() == web_sys::FontFaceSetLoadStatus::Loaded
                && fonts
                    .check_with_text(&font, &request.text)
                    .map_err(metric_host)?,
            "text measurement font resources are not ready",
        )?;
        // An independent inline-block has the same single-line formatting as
        // Glory's block Text. The external marker cannot preserve trailing
        // spaces or fabricate a line box for an empty Text block.
        let rendered = self.text.inner_text();
        scene::ensure(
            rendered.is_empty() || span.height() > 0.,
            "text measurement parent does not participate in layout",
        )?;
        self.canvas.set_font(&font);
        self.canvas.set_text_align("left");
        self.canvas.set_text_baseline("alphabetic");
        self.canvas
            .unchecked_ref::<DirectionCanvas>()
            .set_direction(
                &computed
                    .get_property_value("direction")
                    .map_err(metric_host)?,
            );
        let ink = self.canvas.measure_text(&rendered).map_err(metric_host)?;
        let left = -ink.actual_bounding_box_left();
        let right = ink.actual_bounding_box_right();
        let top = -ink.actual_bounding_box_ascent();
        let bottom = ink.actual_bounding_box_descent();
        scene::ensure(
            [ink.width(), left, right, top, bottom]
                .into_iter()
                .all(f64::is_finite),
            "Canvas actual text ink metrics are unavailable",
        )?;
        // Browser engines quantize HTML and Canvas font sizes independently.
        // Cover the HTML advance plus actual Canvas overhang, including drift
        // accumulated over long runs. One logical pixel outward covers glyph
        // edge quantization. HTML remains authoritative for baseline/line box.
        let ink_left = left.min(0.).floor() - 1.;
        let ink_right = (advance + (right - ink.width()).max(0.)).ceil() + 1.;
        let ink_bounds = (right > left && bottom > top).then_some(LayoutRect {
            x: ink_left,
            y: top.floor() - 1.,
            width: ink_right - ink_left,
            height: bottom.ceil() - top.floor() + 2.,
        });
        request.measured(SemanticTextMetrics {
            advance,
            line_height,
            // An empty block has no line box and therefore no baseline.
            // Engines disagree on the empty inline-block's rectangle origin;
            // canonical zero preserves that actual absence without using the
            // calibration row's strut as a fictitious Text baseline.
            baseline_from_top: if rendered.is_empty() && line_height == 0. {
                0.
            } else {
                (marker.top() - span.top()) / scale_y
            },
            ink_bounds,
        })
    }

    /// Check the entire batch budget and requests before any probe mutation.
    pub fn measure_many(
        &self,
        requests: &[SemanticTextRequest],
    ) -> Result<Vec<SemanticTextMeasurement>> {
        scene::ensure(
            requests.len() <= MAX_MARKS,
            "HTML text measurement batch budget exceeded",
        )?;
        for request in requests {
            request.validate()?;
        }
        requests
            .iter()
            .map(|request| self.measure(request))
            .collect()
    }
}

impl Drop for BrowserSemanticTextMeasurer {
    fn drop(&mut self) {
        self.row.remove();
    }
}
