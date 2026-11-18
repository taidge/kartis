//! Single-item defaults for a native Tree paint pass. Unlike a Scene replay,
//! these helpers never begin/clear/reset the caller's Canvas context.
use web_sys::js_sys::{Function, Reflect};

use super::*;
use crate::ZoomTransform;

fn transform(context: &CanvasRenderingContext2d) -> Result<[f64; 6]> {
    let method = Reflect::get(context, &"getTransform".into())
        .map_err(host_error)?
        .dyn_into::<Function>()
        .map_err(host_error)?;
    let matrix = method.call0(context).map_err(host_error)?;
    let mut values = [0.; 6];
    for (index, key) in ["a", "b", "c", "d", "e", "f"].iter().enumerate() {
        values[index] = Reflect::get(&matrix, &(*key).into())
            .map_err(host_error)?
            .as_f64()
            .filter(|value| value.is_finite())
            .ok_or_else(|| ChartError("Tree Canvas transform is not finite".into()))?;
    }
    Ok(values)
}

fn adapter(
    context: &CanvasRenderingContext2d,
    pixel_ratio: f64,
    view: ZoomTransform,
    matrix: [f64; 6],
) -> Result<BrowserCanvas> {
    view.validate()?;
    ensure(
        pixel_ratio.is_finite() && (0.25..=8.).contains(&pixel_ratio),
        "pixel ratio must be in 0.25..=8",
    )?;
    let canvas = context
        .canvas()
        .ok_or_else(|| ChartError("Tree Canvas has no destination".into()))?;
    // Use the actual affine density, including callback-authored shear/scale,
    // rather than assuming the context still has the initial camera matrix.
    let aa = matrix[0] * matrix[0] + matrix[1] * matrix[1];
    let bb = matrix[2] * matrix[2] + matrix[3] * matrix[3];
    let ab = matrix[0] * matrix[2] + matrix[1] * matrix[3];
    let density = ((aa + bb + (aa - bb).hypot(2. * ab)) / 2.).sqrt();
    ensure(density.is_finite(), "Tree Canvas density is not finite")?;
    Ok(BrowserCanvas {
        context: context.clone(),
        view,
        ratio: pixel_ratio,
        size: [canvas.width(), canvas.height()],
        // A singular context has no area, but native patterns still need a
        // valid tile allocation before the host processes the empty geometry.
        density: if density > 0. {
            density
        } else {
            pixel_ratio * view.scale
        },
        patterns: Default::default(),
        blend: None,
        damage: None,
        clip_damage: false,
    })
}

fn mark_clip(target: &mut BrowserCanvas, mark: &Mark) -> Result<()> {
    // The enclosing helper's single save owns restoration of both clips.
    // Do not push another save: a failed path replay must also restore fully.
    if let Some(clip) = mark.clip {
        target.context.begin_path();
        target.context.rect(clip.x, clip.y, clip.width, clip.height);
        target.context.clip();
    }
    if let Some(clip) = &mark.clip_path {
        super::super::replay_path(target, &clip.geometry)?;
        target
            .context
            .clip_with_canvas_winding_rule(match clip.rule {
                ClipRule::NonZero => web_sys::CanvasWindingRule::Nonzero,
                ClipRule::EvenOdd => web_sys::CanvasWindingRule::Evenodd,
            });
    }
    Ok(())
}

/// Draw one complete Scene Mark under the caller's existing transform, clip,
/// alpha and composite operation. Default styles are restored on return.
pub(crate) fn tree_canvas_mark(
    context: &CanvasRenderingContext2d,
    mark: &Mark,
    pixel_ratio: f64,
    view: ZoomTransform,
    scratch_pixel_budget: u64,
) -> Result<()> {
    let matrix = transform(context)?;
    let mut target = adapter(context, pixel_ratio, view, matrix)?;
    let point = super::super::point_stroke_footprint(mark)?;
    if let Some(dash) = &mark.stroke_dash {
        dash.native_normalized()?;
    }
    let pattern_pixels = if let Some(FillPaint::Pattern(pattern)) = &mark.fill_paint {
        let [w, h] = pattern.tile_pixels(target.density)?;
        pattern.tile_operation_cost(target.density)?;
        u64::from(w) * u64::from(h)
    } else {
        0
    };
    ensure(
        pattern_pixels <= scratch_pixel_budget,
        "Tree Canvas default pattern live pixel budget exceeded",
    )?;
    context.save();
    let painted = (|| {
        mark_clip(&mut target, mark)?;
        if mark.stroke.is_some() {
            context.set_line_join("miter");
            if mark.stroke_dash.is_none() {
                context
                    .set_line_dash(DashArray::new().as_ref())
                    .map_err(host_error)?;
                context.set_line_dash_offset(0.);
            }
        }
        if crate::blend::requires_isolation(mark, BlendMode::Normal) {
            isolated_mark(
                &mut target,
                mark,
                point.as_ref(),
                matrix,
                scratch_pixel_budget - pattern_pixels,
            )
        } else {
            let alpha = context.global_alpha() * mark.opacity;
            super::super::draw_mark(&mut target, mark, point.as_ref(), alpha)
        }
    })();
    context.restore();
    painted
}

fn isolated_mark(
    target: &mut BrowserCanvas,
    mark: &Mark,
    point: Option<&Geometry>,
    matrix: [f64; 6],
    scratch_pixel_budget: u64,
) -> Result<()> {
    // Parent/source clips apply at final composition, not while building the
    // isolated source. Include the entire primitive, even outside the backing
    // viewport: its parent shadow/filter can project ink back into the frame.
    let mut unbounded = mark.clone();
    unbounded.clip = None;
    unbounded.clip_path = None;
    let Some(bounds) = crate::blend::mark_blend_bounds(&unbounded) else {
        return Ok(());
    };
    let mut lo = [f64::INFINITY; 2];
    let mut hi = [f64::NEG_INFINITY; 2];
    for x in [bounds.min[0], bounds.max[0]] {
        for y in [bounds.min[1], bounds.max[1]] {
            let p = [
                matrix[0] * x + matrix[2] * y + matrix[4],
                matrix[1] * x + matrix[3] * y + matrix[5],
            ];
            for i in 0..2 {
                lo[i] = lo[i].min(p[i]);
                hi[i] = hi[i].max(p[i]);
            }
        }
    }
    // Integer device-space origin makes the final blit pixel aligned. The
    // transparent guard also preserves the complete antialiasing fringe.
    let origin = lo.map(|v| v.floor() - 2.);
    let dimensions = [0, 1].map(|i| hi[i].ceil() + 2. - origin[i]);
    ensure(
        origin.into_iter().all(f64::is_finite)
            && dimensions
                .into_iter()
                .all(|v| v.is_finite() && v >= 1. && v <= f64::from(crate::blend::MAX_BLEND_SIDE))
            && dimensions[0] * dimensions[1] <= crate::blend::MAX_BLEND_SCRATCH_PIXELS as f64,
        "Tree Canvas single-mark scratch budget exceeded",
    )?;
    ensure(
        dimensions[0] * dimensions[1] <= scratch_pixel_budget as f64,
        "Tree Canvas default scratch live pixel budget exceeded",
    )?;
    let document = target
        .context
        .canvas()
        .and_then(|canvas| canvas.owner_document())
        .ok_or_else(|| ChartError("Tree Canvas has no owner document".into()))?;
    let canvas = document
        .create_element("canvas")
        .map_err(host_error)?
        .dyn_into::<HtmlCanvasElement>()
        .map_err(|value| host_error(value.into()))?;
    canvas.set_width(dimensions[0] as u32);
    canvas.set_height(dimensions[1] as u32);
    let context = canvas
        .get_context("2d")
        .map_err(host_error)?
        .ok_or_else(|| ChartError("Tree Canvas mark scratch unavailable".into()))?
        .dyn_into::<CanvasRenderingContext2d>()
        .map_err(|value| host_error(value.into()))?;
    // A fresh scratch has no shadow/filter and source-over/alpha=1. Those
    // parent effects are applied exactly once to the completed object below.
    context
        .set_transform(
            matrix[0],
            matrix[1],
            matrix[2],
            matrix[3],
            matrix[4] - origin[0],
            matrix[5] - origin[1],
        )
        .map_err(host_error)?;
    let mut scratch = adapter(&context, target.ratio, target.view, matrix)?;
    super::super::draw_mark(&mut scratch, mark, point, 1.)?;
    target
        .context
        .set_transform(1., 0., 0., 1., 0., 0.)
        .map_err(host_error)?;
    target
        .context
        .set_global_alpha(target.context.global_alpha() * mark.opacity);
    target.context.set_image_smoothing_enabled(false);
    target
        .context
        .draw_image_with_html_canvas_element(&canvas, origin[0], origin[1])
        .map_err(host_error)
}

/// Draw one resolved Scene Label without changing the parent's alpha, native
/// composite, transform or clip. Source font/whitespace are explicit inputs.
#[allow(clippy::too_many_arguments)]
pub(crate) fn tree_canvas_label(
    context: &CanvasRenderingContext2d,
    label: &Label,
    font: &ResolvedFont,
    whitespace: LabelWhitespace,
    stroke: Option<crate::LabelStroke>,
    clip: Option<&ClipPath>,
    pixel_ratio: f64,
    view: ZoomTransform,
) -> Result<()> {
    let matrix = transform(context)?;
    let mut target = adapter(context, pixel_ratio, view, matrix)?;
    context.save();
    let painted = (|| {
        if let Some(clip) = clip {
            super::super::replay_path(&mut target, &clip.geometry)?;
            context.clip_with_canvas_winding_rule(match clip.rule {
                ClipRule::NonZero => web_sys::CanvasWindingRule::Nonzero,
                ClipRule::EvenOdd => web_sys::CanvasWindingRule::Evenodd,
            });
        }
        context.translate(label.x, label.y).map_err(host_error)?;
        context
            .rotate(label.rotation.to_radians())
            .map_err(host_error)?;
        context.set_fill_style_str(&css_color(label.color));
        context.set_font(&font.canvas_font(label.size));
        context.set_text_align(match label.anchor {
            TextAnchor::Middle => "center",
            TextAnchor::Start => "left",
            TextAnchor::End => "right",
        });
        if prototype_has_property(&context_prototype(context), "fontKerning") {
            context
                .unchecked_ref::<CanvasTextKerning>()
                .set_font_kerning("normal");
        }
        context.set_text_baseline("alphabetic");
        let text = whitespace.normalize(&label.text);
        context.fill_text(&text, 0., 0.).map_err(host_error)?;
        if let Some(stroke) = stroke
            && stroke.width > 0.
            && stroke.color.3 > 0
        {
            context.set_stroke_style_str(&css_color(stroke.color));
            context.set_line_width(stroke.width);
            context.set_line_join("round");
            context.stroke_text(&text, 0., 0.).map_err(host_error)?;
        }
        Ok(())
    })();
    context.restore();
    painted
}
