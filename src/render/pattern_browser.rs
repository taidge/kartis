//! Typed application-owned CanvasPattern construction. No eval, global document,
//! SVG serialization, system I/O or application-provided JavaScript is involved.
use crate::{ChartError, Geometry, PatternFill, Result};
use wasm_bindgen::{JsCast, JsValue, prelude::wasm_bindgen};
use web_sys::{CanvasPattern, CanvasRenderingContext2d, DomMatrix2dInit, HtmlCanvasElement};

// The pinned web-sys 0.3.107 CanvasPattern.set_transform takes legacy SvgMatrix
// and omits `catch`. The actual HTML API accepts a DOMMatrix2DInit dictionary.
// This minimal typed binding uses that signature and reports host exceptions.
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "CanvasPattern")]
    type PatternTransformTarget;
    #[wasm_bindgen(method,catch,js_name=setTransform)]
    fn set_transform(
        this: &PatternTransformTarget,
        matrix: &DomMatrix2dInit,
    ) -> std::result::Result<(), JsValue>;
}
fn host(value: JsValue) -> ChartError {
    ChartError(format!("pattern canvas host call failed: {value:?}"))
}
/// Build one bounded repeat tile. Density includes DPR and the current view's
/// scale; the result's matrix corrects ceil backing sizes to retain exact Scene
/// pitch. Creation/drawing occurs off-screen; the caller's state is unchanged.
pub(crate) fn make(
    context: &CanvasRenderingContext2d,
    pattern: &PatternFill,
    density: f64,
) -> Result<CanvasPattern> {
    let dimensions = pattern.tile_pixels(density)?;
    pattern.tile_operation_cost(density)?;
    let tile = pattern.tile()?;
    let transform = pattern.pixel_transform(dimensions)?;
    let document = context
        .canvas()
        .and_then(|c| c.owner_document())
        .ok_or_else(|| ChartError("pattern canvas has no owner document".into()))?;
    let canvas = document
        .create_element("canvas")
        .map_err(host)?
        .dyn_into::<HtmlCanvasElement>()
        .map_err(|value| host(value.into()))?;
    canvas.set_width(dimensions[0]);
    canvas.set_height(dimensions[1]);
    let tile_context = canvas
        .get_context("2d")
        .map_err(host)?
        .ok_or_else(|| ChartError("pattern 2d context unavailable".into()))?
        .dyn_into::<CanvasRenderingContext2d>()
        .map_err(|value| host(value.into()))?;
    tile_context
        .scale(
            f64::from(dimensions[0]) / tile.size[0],
            f64::from(dimensions[1]) / tile.size[1],
        )
        .map_err(host)?;
    tile_context.set_fill_style_str(&crate::render::css_color(tile.background));
    tile_context.fill_rect(0., 0., tile.size[0], tile.size[1]);
    tile_context.set_fill_style_str(&crate::render::css_color(tile.foreground));
    for shape in tile.shapes {
        match shape {
            Geometry::Circle { x, y, radius } => {
                tile_context.begin_path();
                tile_context
                    .arc(x, y, radius, 0., std::f64::consts::TAU)
                    .map_err(host)?;
                tile_context.fill();
            }
            Geometry::Rect {
                x,
                y,
                width,
                height,
            } => tile_context.fill_rect(x, y, width, height),
            Geometry::Path(_) => {
                unreachable!("pattern tiles contain only circle/rectangle primitives")
            }
        }
    }
    let output = context
        .create_pattern_with_html_canvas_element(&canvas, "repeat")
        .map_err(host)?
        .ok_or_else(|| ChartError("repeat pattern unavailable".into()))?;
    let matrix = DomMatrix2dInit::new();
    matrix.set_a(transform[0]);
    matrix.set_b(transform[1]);
    matrix.set_c(transform[2]);
    matrix.set_d(transform[3]);
    matrix.set_e(transform[4]);
    matrix.set_f(transform[5]);
    output
        .unchecked_ref::<PatternTransformTarget>()
        .set_transform(&matrix)
        .map_err(host)?;
    Ok(output)
}
