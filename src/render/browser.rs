use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

use super::*;
use crate::blend::{BlendMode, MarkBlends};
mod blend_layer;
#[cfg(feature = "web")]
mod retained;
#[cfg(feature = "web")]
pub(crate) use retained::RetainedCanvas;
mod tree_canvas;
pub(crate) use tree_canvas::{tree_canvas_label, tree_canvas_mark};
#[wasm_bindgen::prelude::wasm_bindgen]
extern "C" {
    // Optional reset support preserves the historical clean context state
    // without reallocating an unchanged backing store. Older hosts retain
    // the previous width/height-reset path.
    type CanvasResetSupport;
    // SVG's small-text kerning policy is explicit in Canvas too. This typed
    // setter fills a web-sys API gap without property lookup or new features.
    #[wasm_bindgen::prelude::wasm_bindgen(extends = CanvasRenderingContext2d)]
    type CanvasTextKerning;
    #[wasm_bindgen::prelude::wasm_bindgen(method, setter, js_name = fontKerning)]
    fn set_font_kerning(this: &CanvasTextKerning, value: &str);
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = Object, js_name = getPrototypeOf)]
    fn context_prototype(context: &CanvasRenderingContext2d) -> wasm_bindgen::JsValue;
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = Reflect, js_name = has)]
    fn prototype_has_property(prototype: &wasm_bindgen::JsValue, property: &str) -> bool;

    #[wasm_bindgen::prelude::wasm_bindgen(method, getter, catch, js_name = reset)]
    fn reset_member(
        this: &CanvasResetSupport,
    ) -> std::result::Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue>;
    #[wasm_bindgen::prelude::wasm_bindgen(method, catch, js_name = reset)]
    fn reset_state(this: &CanvasResetSupport) -> std::result::Result<(), wasm_bindgen::JsValue>;
    #[wasm_bindgen::prelude::wasm_bindgen(js_name = Array)]
    type DashArray;
    #[wasm_bindgen::prelude::wasm_bindgen(constructor, js_class = Array)]
    fn new() -> DashArray;
    #[wasm_bindgen::prelude::wasm_bindgen(method, js_name = push)]
    fn push(this: &DashArray, value: f64);
}

fn host_error(value: wasm_bindgen::JsValue) -> ChartError {
    ChartError(format!("canvas host call failed: {value:?}"))
}
struct BrowserCanvas {
    context: CanvasRenderingContext2d,
    view: crate::annotations::ZoomTransform,
    ratio: f64,
    size: [u32; 2],
    density: f64,
    patterns: std::collections::BTreeMap<String, web_sys::CanvasPattern>,
    blend: Option<blend_layer::Layers>,
    /// Integer device rectangles cleared before a retained pass.
    damage: Option<Vec<[u32; 4]>>,
    clip_damage: bool,
}
impl Canvas for BrowserCanvas {
    fn supports_group_opacity(&self) -> bool {
        true
    }
    fn prepare_mark_blends(&mut self, scene: &Scene, blends: &MarkBlends) -> Result<()> {
        if crate::blend::has_isolated_marks(scene, blends) && self.blend.is_none() {
            self.blend = Some(blend_layer::Layers::prepare(
                &self.context,
                scene,
                blends,
                self.size,
                self.ratio,
                self.view,
            )?);
        }
        Ok(())
    }
    fn begin_mark_blend(&mut self, index: usize) -> Result<bool> {
        self.blend.as_mut().expect("prepared blend mode").begin(
            &mut self.context,
            index,
            self.ratio,
            self.view,
        )
    }
    fn end_mark_blend(&mut self, mode: BlendMode, opacity: f64, commit: bool) -> Result<()> {
        self.blend.as_mut().expect("prepared blend mode").end(
            &mut self.context,
            mode,
            opacity,
            commit,
        )
    }
    fn supports_translucent_dash_dots(&self) -> bool {
        true
    }
    fn stroke_dashed(
        &mut self,
        _: &Geometry,
        stroke: GraphicStroke,
        dash: &StrokeDash,
        opacity: f64,
    ) -> Result<()> {
        let dash = dash.native_normalized()?;
        let values = DashArray::new();
        for value in dash.segments {
            values.push(value);
        }
        self.context
            .set_line_dash(values.as_ref())
            .map_err(host_error)?;
        self.context.set_line_dash_offset(dash.offset);
        let painted = self.stroke(stroke, opacity);
        self.context.set_line_dash_offset(0.);
        let restored = self
            .context
            .set_line_dash(DashArray::new().as_ref())
            .map_err(host_error);
        painted.and(restored)
    }
    fn supports_clip_paths(&self) -> bool {
        true
    }
    fn push_clip_path(&mut self, clip: &ClipPath) -> Result<()> {
        self.context.save();
        super::replay_path(self, &clip.geometry)?;
        self.context.clip_with_canvas_winding_rule(match clip.rule {
            ClipRule::NonZero => web_sys::CanvasWindingRule::Nonzero,
            ClipRule::EvenOdd => web_sys::CanvasWindingRule::Evenodd,
        });
        Ok(())
    }
    fn pop_clip_path(&mut self) -> Result<()> {
        self.context.restore();
        Ok(())
    }
    fn supports_fill_paint(&self) -> bool {
        true
    }
    fn fill_paint(&mut self, paint: &FillPaint, opacity: f64) -> Result<()> {
        if let FillPaint::Pattern(pattern) = paint {
            let key = pattern.deterministic_id();
            if !self.patterns.contains_key(&key) {
                let p = super::pattern_browser::make(&self.context, pattern, self.density)?;
                self.patterns.insert(key.clone(), p);
            }
            let p = &self.patterns[&key];
            self.context.set_global_alpha(opacity);
            self.context.set_fill_style_canvas_pattern(p);
            self.context.fill();
            return Ok(());
        }
        let gradient = match paint {
            FillPaint::Linear { start, end, .. } => self
                .context
                .create_linear_gradient(start[0], start[1], end[0], end[1]),
            FillPaint::Radial { center, radius, .. } => self
                .context
                .create_radial_gradient(center[0], center[1], 0., center[0], center[1], *radius)
                .map_err(host_error)?,
            FillPaint::Pattern(_) => unreachable!(),
        };
        for stop in paint.stops() {
            gradient
                .add_color_stop(stop.offset as f32, &css_color(stop.color))
                .map_err(host_error)?;
        }
        self.context.set_global_alpha(opacity);
        self.context.set_fill_style_canvas_gradient(&gradient);
        self.context.fill();
        Ok(())
    }
    fn supports_text_rotation(&self) -> bool {
        true
    }
    fn supports_clipping(&self) -> bool {
        true
    }
    fn push_clip(&mut self, c: ClipRect) -> Result<()> {
        self.context.save();
        self.context.begin_path();
        self.context.rect(c.x, c.y, c.width, c.height);
        self.context.clip();
        Ok(())
    }
    fn pop_clip(&mut self) -> Result<()> {
        self.context.restore();
        Ok(())
    }
    fn begin(&mut self, w: f64, h: f64, bg: Color) -> Result<()> {
        let c = &self.context;
        let canvas = c
            .canvas()
            .ok_or_else(|| ChartError("Canvas has no destination".into()))?;
        let reset: &CanvasResetSupport = c.unchecked_ref();
        if self.damage.is_some() {
            debug_assert_eq!([canvas.width(), canvas.height()], self.size);
        } else if reset.reset_member().map_err(host_error)?.is_function() {
            if canvas.width() != self.size[0] {
                canvas.set_width(self.size[0]);
            }
            if canvas.height() != self.size[1] {
                canvas.set_height(self.size[1]);
            }
            reset.reset_state().map_err(host_error)?;
        } else {
            canvas.set_width(self.size[0]);
            canvas.set_height(self.size[1]);
        }
        c.set_transform(self.ratio, 0., 0., self.ratio, 0., 0.)
            .map_err(host_error)?;
        c.save();
        c.set_global_alpha(1.);
        if let Some(rects) = &self.damage {
            if !self.clip_damage {
                c.save();
            }
            c.set_transform(1., 0., 0., 1., 0., 0.)
                .map_err(host_error)?;
            c.begin_path();
            for &[left, top, right, bottom] in rects {
                let [x, y, width, height] = [left, top, right - left, bottom - top].map(f64::from);
                c.clear_rect(x, y, width, height);
                c.rect(x, y, width, height);
            }
            c.clip();
            c.set_transform(self.ratio, 0., 0., self.ratio, 0., 0.)
                .map_err(host_error)?;
            c.set_fill_style_str(&css_color(bg));
            c.fill_rect(0., 0., w, h);
            if !self.clip_damage {
                c.restore();
            }
        } else {
            c.clear_rect(0., 0., w, h);
            c.set_fill_style_str(&css_color(bg));
            c.fill_rect(0., 0., w, h);
        }
        c.begin_path();
        c.rect(0., 0., w, h);
        c.clip();
        c.translate(self.view.translate_x, self.view.translate_y)
            .map_err(host_error)?;
        c.scale(self.view.scale, self.view.scale)
            .map_err(host_error)?;
        Ok(())
    }
    fn begin_path(&mut self) -> Result<()> {
        self.context.begin_path();
        Ok(())
    }
    fn move_to(&mut self, x: f64, y: f64) -> Result<()> {
        self.context.move_to(x, y);
        Ok(())
    }
    fn line_to(&mut self, x: f64, y: f64) -> Result<()> {
        self.context.line_to(x, y);
        Ok(())
    }
    fn cubic_to(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, x: f64, y: f64) -> Result<()> {
        self.context.bezier_curve_to(x1, y1, x2, y2, x, y);
        Ok(())
    }
    fn close_path(&mut self) -> Result<()> {
        self.context.close_path();
        Ok(())
    }
    fn fill(&mut self, color: Color, opacity: f64) -> Result<()> {
        self.context.set_global_alpha(opacity);
        self.context.set_fill_style_str(&css_color(color));
        self.context.fill();
        Ok(())
    }
    fn stroke(&mut self, stroke: GraphicStroke, opacity: f64) -> Result<()> {
        self.context.set_global_alpha(opacity);
        self.context.set_stroke_style_str(&css_color(stroke.color));
        self.context.set_line_width(stroke.width);
        self.context.set_line_cap(cap(stroke.cap));
        self.context.set_miter_limit(4.);
        self.context.stroke();
        Ok(())
    }
    fn text(&mut self, label: &Label) -> Result<()> {
        self.text_with_font(label, &ResolvedFont::default())
    }
    fn supports_text_whitespace(&self) -> bool {
        true
    }
    fn supports_text_fonts(&self) -> bool {
        true
    }
    fn text_with_font(&mut self, label: &Label, font: &ResolvedFont) -> Result<()> {
        self.context.save();
        let painted = (|| {
            self.context
                .translate(label.x, label.y)
                .map_err(host_error)?;
            self.context
                .rotate(label.rotation.to_radians())
                .map_err(host_error)?;
            self.context.set_global_alpha(1.);
            self.context.set_fill_style_str(&css_color(label.color));
            self.context.set_font(&font.canvas_font(label.size));
            self.context.set_text_align(match label.anchor {
                TextAnchor::Middle => "center",
                TextAnchor::Start => "left",
                TextAnchor::End => "right",
            });
            // Inspect the native prototype chain, not caller-owned expandos.
            // Unsupported engines keep their original drawing/state behavior.
            let prototype = context_prototype(&self.context);
            if prototype_has_property(&prototype, "fontKerning") {
                self.context
                    .unchecked_ref::<CanvasTextKerning>()
                    .set_font_kerning("normal");
            }
            self.context.set_text_baseline("alphabetic");
            self.context
                .fill_text(&label.text, 0., 0.)
                .map_err(host_error)
        })();
        self.context.restore();
        painted
    }
    fn supports_text_stroke(&self) -> bool {
        true
    }
    fn text_with_stroke(
        &mut self,
        label: &Label,
        font: &ResolvedFont,
        _explicit_font: bool,
        stroke: crate::LabelStroke,
    ) -> Result<()> {
        self.context.save();
        let painted = (|| {
            self.context
                .translate(label.x, label.y)
                .map_err(host_error)?;
            self.context
                .rotate(label.rotation.to_radians())
                .map_err(host_error)?;
            self.context.set_global_alpha(1.);
            self.context.set_font(&font.canvas_font(label.size));
            self.context.set_text_align(match label.anchor {
                TextAnchor::Start => "left",
                TextAnchor::Middle => "center",
                TextAnchor::End => "right",
            });
            self.context.set_text_baseline("alphabetic");
            let prototype = context_prototype(&self.context);
            if prototype_has_property(&prototype, "fontKerning") {
                self.context
                    .unchecked_ref::<CanvasTextKerning>()
                    .set_font_kerning("normal");
            }
            self.context.set_fill_style_str(&css_color(label.color));
            self.context
                .fill_text(&label.text, 0., 0.)
                .map_err(host_error)?;
            if stroke.width > 0. && stroke.color.3 > 0 {
                self.context.set_stroke_style_str(&css_color(stroke.color));
                self.context.set_line_width(stroke.width);
                self.context.set_line_join("round");
                self.context
                    .stroke_text(&label.text, 0., 0.)
                    .map_err(host_error)?;
            }
            Ok(())
        })();
        self.context.restore();
        painted
    }
    fn finish(&mut self) -> Result<()> {
        self.context.restore();
        Ok(())
    }
}

/// Draw on an application-owned browser canvas. Set its CSS width/height to the
/// scene's logical dimensions; this function sizes the backing store for DPR.
/// Pixel coordinates for hit testing must be converted to the scene viewBox.
pub fn draw_canvas(scene: &Scene, canvas: &HtmlCanvasElement, pixel_ratio: f64) -> Result<()> {
    draw_canvas_with_transform(
        scene,
        canvas,
        pixel_ratio,
        crate::annotations::ZoomTransform::default(),
    )
}
/// Paint a viewport transform after clearing/clipping the backing viewport;
/// geometry, text and per-mark clips share the same scene transform.
pub fn draw_canvas_with_transform(
    scene: &Scene,
    canvas: &HtmlCanvasElement,
    pixel_ratio: f64,
    view: crate::annotations::ZoomTransform,
) -> Result<()> {
    draw_canvas_with_transform_and_blends(scene, canvas, pixel_ratio, view, &MarkBlends::default())
}

/// Paint a chart-isolated blend profile on the application canvas.
pub fn draw_canvas_with_blends(
    scene: &Scene,
    canvas: &HtmlCanvasElement,
    pixel_ratio: f64,
    blends: &MarkBlends,
) -> Result<()> {
    draw_canvas_with_transform_and_blends(
        scene,
        canvas,
        pixel_ratio,
        crate::ZoomTransform::default(),
        blends,
    )
}

/// Paint artistic blending after applying the viewport transform to marks and
/// clips. Resource admission and scratch allocation precede destination resize.
pub fn draw_canvas_with_transform_and_blends(
    scene: &Scene,
    canvas: &HtmlCanvasElement,
    pixel_ratio: f64,
    view: crate::annotations::ZoomTransform,
    blends: &MarkBlends,
) -> Result<()> {
    prepare_canvas(scene, canvas, pixel_ratio, view, blends)?.paint(scene, blends)
}

pub(crate) struct PreparedCanvas(BrowserCanvas);

impl PreparedCanvas {
    pub(crate) fn paint(mut self, scene: &Scene, blends: &MarkBlends) -> Result<()> {
        draw_with_blends(scene, &mut self.0, blends)
    }
}

fn context_2d(canvas: &HtmlCanvasElement) -> Result<CanvasRenderingContext2d> {
    canvas
        .get_context("2d")
        .map_err(host_error)?
        .ok_or_else(|| ChartError("2d canvas unavailable".into()))?
        .dyn_into::<CanvasRenderingContext2d>()
        .map_err(|value| host_error(value.into()))
}

pub(crate) fn prepare_canvas(
    scene: &Scene,
    canvas: &HtmlCanvasElement,
    pixel_ratio: f64,
    view: crate::annotations::ZoomTransform,
    blends: &MarkBlends,
) -> Result<PreparedCanvas> {
    scene.validate()?;
    blends.validate(scene)?;
    let isolated = crate::blend::has_isolated_marks(scene, blends);
    for mark in &scene.marks {
        if let Some(dash) = &mark.stroke_dash {
            dash.native_normalized()?;
        }
        if isolated {
            super::point_stroke_footprint(mark)?;
        }
    }
    view.validate()?;
    ensure(
        pixel_ratio.is_finite() && (0.25..=8.).contains(&pixel_ratio),
        "pixel ratio must be in 0.25..=8",
    )?;
    let w = (scene.width * pixel_ratio).ceil();
    let h = (scene.height * pixel_ratio).ceil();
    ensure(
        w * h <= 67_108_864.,
        "canvas backing store exceeds 64 megapixels",
    )?;
    let density = pixel_ratio * view.scale;
    let mut patterns = std::collections::BTreeSet::new();
    let mut pixels = 0u64;
    let mut operations = 0usize;
    for mark in &scene.marks {
        if let Some(FillPaint::Pattern(p)) = &mark.fill_paint
            && patterns.insert(p.deterministic_id())
        {
            let [w, h] = p.tile_pixels(density)?;
            pixels += u64::from(w) * u64::from(h);
            operations += p.tile_operation_cost(density)?;
            ensure(
                pixels <= 16_777_216 && operations <= 268_435_456,
                "canvas scene pattern budget exceeded",
            )?;
        }
    }
    let context = context_2d(canvas)?;
    let mut prepared_patterns = std::collections::BTreeMap::new();
    for mark in &scene.marks {
        if let Some(FillPaint::Pattern(p)) = &mark.fill_paint {
            let key = p.deterministic_id();
            if let std::collections::btree_map::Entry::Vacant(entry) = prepared_patterns.entry(key)
            {
                entry.insert(super::pattern_browser::make(&context, p, density)?);
            }
        }
    }
    let mut prepared = BrowserCanvas {
        context,
        view,
        ratio: pixel_ratio,
        size: [w as u32, h as u32],
        density,
        patterns: prepared_patterns,
        blend: None,
        damage: None,
        clip_damage: false,
    };
    prepared.prepare_mark_blends(scene, blends)?;
    Ok(PreparedCanvas(prepared))
}
