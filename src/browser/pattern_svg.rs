//! Retained SVG pattern construction using typed DOM calls, never innerHTML.
use wasm_bindgen::JsValue;
use web_sys::{Document, Element};

use crate::{ChartError, Geometry, PatternFill, Result};
fn host(value: JsValue) -> ChartError {
    ChartError(format!("pattern SVG host call failed: {value:?}"))
}
fn node(document: &Document, name: &str) -> Result<Element> {
    document
        .create_element_ns(Some("http://www.w3.org/2000/svg"), name)
        .map_err(host)
}
fn attr(node: &Element, name: &str, value: impl ToString) -> Result<()> {
    node.set_attribute(name, &value.to_string()).map_err(host)
}
pub(super) fn make(document: &Document, pattern: &PatternFill) -> Result<Element> {
    let tile = pattern.tile()?;
    let [a, b, c, d, e, f] = pattern.logical_transform()?;
    let output = node(document, "pattern")?;
    attr(&output, "id", pattern.deterministic_id())?;
    attr(&output, "patternUnits", "userSpaceOnUse")?;
    attr(&output, "patternContentUnits", "userSpaceOnUse")?;
    attr(&output, "overflow", "hidden")?;
    attr(&output, "width", tile.size[0])?;
    attr(&output, "height", tile.size[1])?;
    attr(
        &output,
        "patternTransform",
        format!("matrix({a} {b} {c} {d} {e} {f})"),
    )?;
    let background = node(document, "rect")?;
    attr(&background, "width", tile.size[0])?;
    attr(&background, "height", tile.size[1])?;
    attr(
        &background,
        "fill",
        crate::render::css_color(tile.background),
    )?;
    output.append_child(&background).map_err(host)?;
    for geometry in tile.shapes {
        let child = match geometry {
            Geometry::Circle { x, y, radius } => {
                let child = node(document, "circle")?;
                attr(&child, "cx", x)?;
                attr(&child, "cy", y)?;
                attr(&child, "r", radius)?;
                child
            }
            Geometry::Rect {
                x,
                y,
                width,
                height,
            } => {
                let child = node(document, "rect")?;
                attr(&child, "x", x)?;
                attr(&child, "y", y)?;
                attr(&child, "width", width)?;
                attr(&child, "height", height)?;
                child
            }
            Geometry::Path(_) => {
                unreachable!("pattern tiles contain only circle/rectangle primitives")
            }
        };
        attr(&child, "fill", crate::render::css_color(tile.foreground))?;
        output.append_child(&child).map_err(host)?;
    }
    Ok(output)
}
