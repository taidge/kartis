//! Shared scene rendering. Inspired by Nivo's compute/render split in
//! `packages/bar/src/BarCanvas.tsx` and `renderBar.ts`; no JavaScript runtime
//! or d3 dependency is required. Canvas and SVG replay identical cubic paths.
use crate::{ClipPath, ClipRule, FillPaint, LabelWhitespace, ResolvedFont, StrokeDash};
#[cfg(all(feature = "canvas", target_arch = "wasm32"))]
mod pattern_browser;
use std::fmt::Write;

use glory::render::{GraphicStroke, PathCommand, StrokeCap};
use glory::style::Color;

use crate::scene::*;
#[cfg(feature = "raster")]
pub mod raster;

pub fn css_color(Color(r, g, b, a): Color) -> String {
    format!("rgba({r},{g},{b},{})", f64::from(a) / 255.)
}
fn escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            ch if matches!(ch, '\0' | '\u{fffe}' | '\u{ffff}')
                || ch.is_control() && !matches!(ch, '\t' | '\n' | '\r') =>
            {
                out.push('\u{fffd}')
            }
            ch => out.push(ch),
        }
    }
    out
}
fn paint_definition(paint: &FillPaint, namespace: &str, out: &mut String) {
    if let FillPaint::Pattern(pattern) = paint {
        out.push_str(
            &pattern
                .svg_definition_with_namespace(namespace)
                .expect("validated pattern"),
        );
        return;
    }
    let (tag, attributes) = match paint {
        FillPaint::Linear { start, end, .. } => (
            "linearGradient",
            format!(
                "x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\"",
                start[0], start[1], end[0], end[1]
            ),
        ),
        FillPaint::Radial { center, radius, .. } => (
            "radialGradient",
            format!("cx=\"{}\" cy=\"{}\" r=\"{}\"", center[0], center[1], radius),
        ),
        FillPaint::Pattern(_) => unreachable!(),
    };
    write!(out,"<{tag} id=\"{namespace}{}\" gradientUnits=\"userSpaceOnUse\" color-interpolation=\"sRGB\" {attributes}>",paint.svg_id()).unwrap();
    for stop in paint.stops() {
        write!(
            out,
            "<stop offset=\"{}\" stop-color=\"{}\"/>",
            stop.offset,
            css_color(stop.color)
        )
        .unwrap();
    }
    write!(out, "</{tag}>").unwrap();
}
fn clip_definition(clip: &ClipPath, namespace: &str, out: &mut String) {
    write!(out,"<clipPath id=\"{namespace}{}\" clipPathUnits=\"userSpaceOnUse\"><path d=\"{}\" clip-rule=\"{}\"/></clipPath>",clip.svg_id(),path_data(&clip.geometry.commands()),clip_rule(clip.rule)).unwrap();
}
pub(crate) fn clip_rule(rule: ClipRule) -> &'static str {
    match rule {
        ClipRule::NonZero => "nonzero",
        ClipRule::EvenOdd => "evenodd",
    }
}
pub(crate) fn path_data(commands: &[PathCommand]) -> String {
    let mut out = String::new();
    for command in commands {
        match command {
            PathCommand::Move(p) => write!(out, "M{} {}", p.x, p.y),
            PathCommand::Line(p) => write!(out, "L{} {}", p.x, p.y),
            PathCommand::Cubic { first, second, end } => write!(
                out,
                "C{} {} {} {} {} {}",
                first.x, first.y, second.x, second.y, end.x, end.y
            ),
            PathCommand::Close => write!(out, "Z"),
        }
        .expect("String writes are infallible");
    }
    out
}
pub(crate) fn anchor(anchor: TextAnchor) -> &'static str {
    match anchor {
        TextAnchor::Start => "start",
        TextAnchor::Middle => "middle",
        TextAnchor::End => "end",
    }
}
pub(crate) fn cap(cap: StrokeCap) -> &'static str {
    match cap {
        StrokeCap::Butt => "butt",
        StrokeCap::Round => "round",
        StrokeCap::Square => "square",
    }
}

/// A standalone, accessible SVG document. Host markup in all labels and keys
/// is escaped. The root clips overflowing geometry to the scene viewport.
pub fn to_svg(scene: &Scene) -> Result<String> {
    to_svg_inner(scene, "", &crate::blend::MarkBlends::default())
}
/// SVG for embedding beside other inline SVGs. Use a different, stable
/// namespace for each embedding: only generated definition IDs and references
/// change. Scene identities, labels, user text and standalone exports survive.
/// Namespaces contain 1..=128 ASCII letters, digits, `_`, `-`, or `.`, beginning
/// with a letter or `_`. BrowserChart automatically isolates its own SVG owner.
pub fn to_svg_with_namespace(scene: &Scene, namespace: &str) -> Result<String> {
    to_svg_with_namespace_and_blends(scene, namespace, &crate::blend::MarkBlends::default())
}

/// Render into a chart-isolated SVG using the given mark blend modes.
pub fn to_svg_with_blends(scene: &Scene, blends: &crate::blend::MarkBlends) -> Result<String> {
    to_svg_inner(scene, "", blends)
}

/// Namespace SVG definitions and isolate this chart's artistic blend backdrop.
pub fn to_svg_with_namespace_and_blends(
    scene: &Scene,
    namespace: &str,
    blends: &crate::blend::MarkBlends,
) -> Result<String> {
    crate::scene::ensure(
        !namespace.is_empty()
            && namespace.len() <= 128
            && namespace.bytes().enumerate().all(|(i, b)| {
                b.is_ascii_alphabetic()
                    || b == b'_'
                    || i > 0 && (b.is_ascii_digit() || b == b'-' || b == b'.')
            }),
        "invalid SVG namespace",
    )?;
    to_svg_inner(scene, &format!("kartis-ns-{namespace}-"), blends)
}
fn to_svg_inner(
    scene: &Scene,
    namespace: &str,
    blends: &crate::blend::MarkBlends,
) -> Result<String> {
    scene.validate()?;
    blends.validate(scene)?;
    let paint_marks = scene
        .marks
        .iter()
        .map(paint_mark)
        .collect::<Result<Vec<_>>>()?;
    let mut out = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\" role=\"img\" direction=\"ltr\" aria-label=\"{}\" overflow=\"hidden\"{}><title>{}</title><desc>",
        scene.width,
        scene.height,
        scene.width,
        scene.height,
        escape(&scene.title),
        if blends.is_empty() {
            ""
        } else {
            " style=\"isolation:isolate\""
        },
        escape(&scene.title)
    );
    for item in &scene.legend {
        write!(out, "{}; ", escape(&item.label)).expect("String write");
    }
    out.push_str("</desc>");
    let mut clips = std::collections::BTreeSet::new();
    for clip in scene
        .marks
        .iter()
        .filter_map(|m| m.clip_path.as_deref())
        .chain(scene.label_clips.values())
    {
        if clips.insert(clip.svg_id()) {
            out.push_str("<defs>");
            clip_definition(clip, namespace, &mut out);
            out.push_str("</defs>");
        }
    }
    let mut paints = std::collections::BTreeSet::new();
    for paint in paint_marks.iter().filter_map(|m| m.fill_paint.as_ref()) {
        if paints.insert(paint.svg_id()) {
            out.push_str("<defs>");
            paint_definition(paint, namespace, &mut out);
            out.push_str("</defs>");
        }
    }
    write!(
        out,
        "<rect width=\"100%\" height=\"100%\" fill=\"{}\"/>",
        css_color(scene.background)
    )
    .expect("String write");
    let (group_starts, group_ends) = scene.opacity_group_events();
    for (position, item) in scene.draw_items().into_iter().enumerate() {
        if let Some(&index) = group_starts.get(&position) {
            let group = &scene.opacity_groups[index];
            write!(
                out,
                "<g data-opacity-group=\"{}\" opacity=\"{}\">",
                escape(&group.id),
                group.opacity
            )
            .unwrap();
        }
        match item {
            DrawItem::Mark(mark_index) => {
                let mark = &paint_marks[mark_index];
                let mode = blends.mode(&mark.id);
                if mode != crate::blend::BlendMode::Normal {
                    write!(out, "<g style=\"mix-blend-mode:{}\">", mode.css_name()).unwrap();
                }
                if let Some(clip) = &mark.clip_path {
                    write!(out, "<g clip-path=\"url(#{namespace}{})\">", clip.svg_id()).unwrap();
                }
                if let Some(c) = mark.clip {
                    write!(out,"<svg x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" viewBox=\"{} {} {} {}\" overflow=\"hidden\">",c.x,c.y,c.width,c.height,c.x,c.y,c.width,c.height).expect("String write");
                }
                let fill = mark
                    .fill_paint
                    .as_ref()
                    .map(|p| format!("url(#{namespace}{})", p.svg_id()))
                    .unwrap_or_else(|| mark.fill.map(css_color).unwrap_or_else(|| "none".into()));
                // A final normal-blend solid fill with no stroke is one paint:
                // fill-opacity is the same single alpha factor without an SVG
                // object-opacity intermediate surface. Keep multi-paint and
                // non-solid/blended marks on their existing element opacity.
                let opacity_attribute = if mark.fill.is_some()
                    && mark.fill_paint.is_none()
                    && mark.stroke.is_none()
                    && mode == crate::blend::BlendMode::Normal
                    && mark.opacity != 1.
                {
                    "fill-opacity"
                } else {
                    "opacity"
                };
                write!(out,"<path data-id=\"{}\" data-series=\"{}\" data-value=\"{}\" d=\"{}\" fill=\"{}\" {opacity_attribute}=\"{}\"",escape(&mark.id),escape(&mark.series),mark.value,path_data(&mark.geometry.commands()),fill,mark.opacity).expect("String write");
                if let Some(stroke) = mark.stroke {
                    write!(
                        out,
                        " stroke=\"{}\" stroke-width=\"{}\" stroke-linecap=\"{}\"",
                        css_color(stroke.color),
                        stroke.width,
                        cap(stroke.cap)
                    )
                    .expect("String write");
                    if let Some(dash) = &mark.stroke_dash {
                        let dash = dash.native_normalized()?;
                        write!(
                            out,
                            " stroke-dasharray=\"{}\" stroke-dashoffset=\"{}\"",
                            if dash.segments.is_empty() {
                                "none".into()
                            } else {
                                dash.segments
                                    .iter()
                                    .map(f64::to_string)
                                    .collect::<Vec<_>>()
                                    .join(" ")
                            },
                            dash.offset
                        )
                        .unwrap();
                    }
                }
                write!(out, "><title>{}</title></path>", escape(&mark.label))
                    .expect("String write");
                if mark.clip.is_some() {
                    out.push_str("</svg>");
                }
                if mark.clip_path.is_some() {
                    out.push_str("</g>");
                }
                if mode != crate::blend::BlendMode::Normal {
                    out.push_str("</g>");
                }
            }
            DrawItem::Label(index) => {
                let whitespace = scene.typography.label_whitespace(index);
                let drawable = whitespace.drawable_label(&scene.labels[index]);
                let label = drawable.as_ref();
                let font = scene.resolved_label_font(index);

                if let Some(clip) = scene.label_clips.get(&index) {
                    write!(out, "<g clip-path=\"url(#{namespace}{})\">", clip.svg_id()).unwrap();
                }
                write!(out,"<text x=\"{}\" y=\"{}\" fill=\"{}\" font-family=\"{}\" font-size=\"{}\" text-anchor=\"{}\"",label.x,label.y,css_color(label.color),escape(&font.css_family()),label.size,anchor(label.anchor)).expect("String write");
                write!(out, " font-weight=\"{}\"", font.weight).expect("String write");
                if let Some(stroke) = scene.typography.label_strokes.get(&index) {
                    write!(out, " stroke=\"{}\" stroke-width=\"{}\" stroke-linejoin=\"round\" paint-order=\"fill stroke\"", css_color(stroke.color), stroke.width).expect("String write");
                }
                if whitespace == LabelWhitespace::Preserve {
                    out.push_str(" style=\"white-space:pre\"");
                }
                if label.rotation != 0. {
                    write!(
                        out,
                        " transform=\"rotate({} {} {})\"",
                        label.rotation, label.x, label.y
                    )
                    .expect("String write");
                }
                write!(out, ">{}</text>", escape(&label.text)).expect("String write");
                if scene.label_clips.contains_key(&index) {
                    out.push_str("</g>");
                }
            }
        }
        if group_ends.contains_key(&position) {
            out.push_str("</g>");
        }
    }
    out.push_str("</svg>");
    Ok(out)
}

/// Adapter contract for a stateful raster backend. `begin` must establish a
/// clean, clipped logical-coordinate viewport. `finish` releases its state,
/// including when replay fails. Implementations must report failed host calls.
pub trait Canvas {
    /// Support one object-opacity composite after a mark's fill and stroke.
    /// Normal marks can require this even with an empty MarkBlends profile.
    /// This capability is independent of artistic blend-mode support.
    /// Backends opting in must implement the isolated-layer hooks below.
    /// The default rejects such frames before preparation or begin touches
    /// the destination, including adapters overriding prepare_mark_blends.
    fn supports_group_opacity(&self) -> bool {
        false
    }
    /// Validate isolation capabilities and scratch work before begin touches the
    /// target. Supporting backends override all three layer hooks. Existing
    /// adapters explicitly refuse modes or groups requiring isolation.
    fn prepare_mark_blends(
        &mut self,
        scene: &Scene,
        blends: &crate::blend::MarkBlends,
    ) -> Result<()> {
        ensure(
            !crate::blend::has_isolated_marks(scene, blends),
            "Canvas backend does not support mark isolation",
        )
    }
    /// Enter a transparent source layer, leaving the target's clips on the
    /// destination. False means the mark cannot affect the viewport.
    fn begin_mark_blend(&mut self, _mark_index: usize) -> Result<bool> {
        Err(ChartError(
            "Canvas backend does not support mark blending".into(),
        ))
    }
    /// Restore the target. On commit, apply opacity and the selected blend
    /// once under its active clips. On failure, discard the source layer.
    fn end_mark_blend(
        &mut self,
        _mode: crate::blend::BlendMode,
        _opacity: f64,
        _commit: bool,
    ) -> Result<()> {
        Err(ChartError(
            "Canvas backend does not support mark blending".into(),
        ))
    }
    fn supports_clip_paths(&self) -> bool {
        false
    }
    fn push_clip_path(&mut self, _clip: &ClipPath) -> Result<()> {
        Err(ChartError(
            "Canvas backend does not support clip paths".into(),
        ))
    }
    fn pop_clip_path(&mut self) -> Result<()> {
        Err(ChartError(
            "Canvas backend does not support clip paths".into(),
        ))
    }
    fn stroke_dashed(
        &mut self,
        geometry: &Geometry,
        stroke: GraphicStroke,
        dash: &StrokeDash,
        opacity: f64,
    ) -> Result<()> {
        let detailed = crate::dash_geometry_detailed(geometry, dash, &Default::default())?;
        ensure(
            self.supports_translucent_dash_dots()
                || detailed.dots.is_empty()
                || stroke.cap == StrokeCap::Butt
                || opacity == 1. && stroke.color.3 == 255,
            "Canvas backend cannot union translucent zero-dash caps",
        )?;
        replay_path(self, &detailed.stroke_geometry())?;
        self.stroke(stroke, opacity)?;
        for dot in detailed.dots {
            replay_path(
                self,
                &crate::dash_dot_geometry(&dot, stroke.width, stroke.cap)?,
            )?;
            self.fill(stroke.color, opacity)?;
        }
        Ok(())
    }
    /// Opt in when zero-length caps and ordinary dash strokes are composited
    /// as one ink footprint. The default replay paints them separately.
    fn supports_translucent_dash_dots(&self) -> bool {
        false
    }
    fn supports_fill_paint(&self) -> bool {
        false
    }
    fn fill_paint(&mut self, _paint: &FillPaint, _opacity: f64) -> Result<()> {
        Err(ChartError(
            "Canvas backend does not support gradient fills".into(),
        ))
    }
    /// A backend must opt in before receiving nonzero `Label.rotation`.
    fn supports_text_rotation(&self) -> bool {
        false
    }
    /// Opt in to the resolved family/weight protocol. Legacy backends receive
    /// only the original default font, and unsupported profiles fail before
    /// `begin` can mutate the target.
    fn supports_text_fonts(&self) -> bool {
        false
    }
    /// Opt in only if literal leading, trailing and repeated single-line spaces
    /// are painted without collapsing. Preserve input is normalized before replay.
    fn supports_text_whitespace(&self) -> bool {
        false
    }
    fn supports_text_stroke(&self) -> bool {
        false
    }
    fn supports_clipping(&self) -> bool {
        false
    }
    fn push_clip(&mut self, _clip: ClipRect) -> Result<()> {
        Err(ChartError(
            "Canvas backend does not support plot clipping".into(),
        ))
    }
    fn pop_clip(&mut self) -> Result<()> {
        Err(ChartError(
            "Canvas backend does not support plot clipping".into(),
        ))
    }
    fn begin(&mut self, width: f64, height: f64, background: Color) -> Result<()>;
    fn begin_path(&mut self) -> Result<()>;
    fn move_to(&mut self, x: f64, y: f64) -> Result<()>;
    fn line_to(&mut self, x: f64, y: f64) -> Result<()>;
    fn cubic_to(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, x: f64, y: f64) -> Result<()>;
    fn close_path(&mut self) -> Result<()>;
    fn fill(&mut self, color: Color, opacity: f64) -> Result<()>;
    fn stroke(&mut self, stroke: GraphicStroke, opacity: f64) -> Result<()>;
    fn text(&mut self, label: &Label) -> Result<()>;
    fn text_with_font(&mut self, label: &Label, font: &ResolvedFont) -> Result<()> {
        ensure(
            font == &ResolvedFont::default(),
            "Canvas backend does not support text fonts",
        )?;
        self.text(label)
    }
    /// `explicit_font` preserves registration-order font selection for raster
    /// clients that never authored a family/weight declaration.
    fn text_with_stroke(
        &mut self,
        _label: &Label,
        _font: &ResolvedFont,
        _explicit_font: bool,
        _stroke: crate::LabelStroke,
    ) -> Result<()> {
        Err(ChartError(
            "Canvas backend does not support label strokes".into(),
        ))
    }
    fn finish(&mut self) -> Result<()>;
}

/// Validate completely before mutating the target, then replay in scene order.
pub fn draw(scene: &Scene, target: &mut impl Canvas) -> Result<()> {
    draw_with_blends(scene, target, &crate::blend::MarkBlends::default())
}

/// Replay a Scene using chart-isolated artistic blending for selected marks.
pub fn draw_with_blends(
    scene: &Scene,
    target: &mut impl Canvas,
    blends: &crate::blend::MarkBlends,
) -> Result<()> {
    scene.validate()?;
    blends.validate(scene)?;
    crate::blend::validate_opacity_group_layers(scene, blends)?;
    // Native Canvas engines omit caps on a closed all-coincident cubic circle.
    // Resolve one actual ink footprint before touching the backend; this also
    // keeps repeated zero-length subpaths and translucent dash caps one paint.
    let point_ink = scene
        .marks
        .iter()
        .map(point_stroke_footprint)
        .collect::<Result<Vec<_>>>()?;
    if !target.supports_translucent_dash_dots() {
        for (index, mark) in scene.marks.iter().enumerate() {
            if point_ink[index].is_some() {
                continue;
            }
            if let (Some(dash), Some(stroke)) = (&mark.stroke_dash, mark.stroke)
                && stroke.cap != StrokeCap::Butt
                && (mark.opacity < 1. || stroke.color.3 < 255)
            {
                ensure(
                    crate::dash_geometry_detailed(&mark.geometry, dash, &Default::default())?
                        .dots
                        .is_empty(),
                    "Canvas backend cannot union translucent zero-dash caps",
                )?;
            }
        }
    }
    ensure(
        scene.label_clips.is_empty() && !scene.marks.iter().any(|m| m.clip_path.is_some())
            || target.supports_clip_paths(),
        "Canvas backend does not support clip paths",
    )?;
    ensure(
        !scene.marks.iter().any(|m| m.fill_paint.is_some()) || target.supports_fill_paint(),
        "Canvas backend does not support gradient fills",
    )?;
    ensure(
        scene.labels.iter().all(|label| label.rotation == 0.) || target.supports_text_rotation(),
        "Canvas backend does not support rotated text",
    )?;
    ensure(
        target.supports_text_fonts()
            || (0..scene.labels.len())
                .all(|index| scene.resolved_label_font(index) == ResolvedFont::default()),
        "Canvas backend does not support text fonts",
    )?;
    ensure(
        target.supports_text_whitespace()
            || (0..scene.labels.len())
                .all(|index| scene.typography.label_whitespace(index) == LabelWhitespace::Legacy),
        "Canvas backend does not support preserved text whitespace",
    )?;
    ensure(
        scene.typography.label_strokes.is_empty() || target.supports_text_stroke(),
        "Canvas backend does not support label strokes",
    )?;
    ensure(
        !scene.marks.iter().any(|m| m.clip.is_some()) || target.supports_clipping(),
        "Canvas backend does not support plot clipping",
    )?;
    ensure(
        target.supports_group_opacity()
            || (!scene
                .opacity_groups
                .iter()
                .any(|g| g.opacity > 0. && g.opacity < 1.)
                && !scene.marks.iter().any(|mark| {
                    blends.mode(&mark.id) == crate::blend::BlendMode::Normal
                        && crate::blend::requires_isolation(mark, crate::blend::BlendMode::Normal)
                })),
        "Canvas backend does not support object opacity groups",
    )?;
    target.prepare_mark_blends(scene, blends)?;
    target.begin(scene.width, scene.height, scene.background)?;
    let (group_starts, group_ends) = scene.opacity_group_events();
    let mut active_group = None;
    let mut skipped_group = false;
    let painted = (|| {
        for (position, item) in scene.draw_items().into_iter().enumerate() {
            if let Some(&index) = group_starts.get(&position) {
                let group = &scene.opacity_groups[index];
                skipped_group = group.opacity == 0.;
                if group.opacity > 0. && group.opacity < 1. {
                    if target.begin_mark_blend(scene.marks.len() + index)? {
                        active_group = Some(index);
                    } else {
                        skipped_group = true;
                    }
                }
            }
            if skipped_group {
                if group_ends.contains_key(&position) {
                    skipped_group = false;
                }
                continue;
            }
            match item {
                DrawItem::Mark(mark_index) => {
                    let mark = &scene.marks[mark_index];
                    if let Some(c) = mark.clip {
                        target.push_clip(c)?;
                    }
                    if let Some(clip) = &mark.clip_path
                        && let Err(error) = target.push_clip_path(clip)
                    {
                        if mark.clip.is_some() {
                            target.pop_clip()?;
                        }
                        return Err(error);
                    }
                    let painted_mark = (|| {
                        let mode = blends.mode(&mark.id);
                        let point = point_ink[mark_index].as_ref();
                        if !crate::blend::requires_isolation(mark, mode) {
                            return draw_mark(target, mark, point, mark.opacity);
                        }
                        if !target.begin_mark_blend(mark_index)? {
                            return Ok(());
                        }
                        let painted = draw_mark(target, mark, point, 1.);
                        let ended = target.end_mark_blend(mode, mark.opacity, painted.is_ok());
                        painted.and(ended)
                    })();
                    let restored_path = if mark.clip_path.is_some() {
                        target.pop_clip_path()
                    } else {
                        Ok(())
                    };
                    let restored = if mark.clip.is_some() {
                        target.pop_clip()
                    } else {
                        Ok(())
                    };
                    painted_mark.and(restored_path).and(restored)?;
                }
                DrawItem::Label(index) => {
                    let whitespace = scene.typography.label_whitespace(index);
                    let drawable = whitespace.drawable_label(&scene.labels[index]);
                    let label = drawable.as_ref();
                    let font = scene.resolved_label_font(index);
                    let explicit = scene.typography.has_label_font(index);

                    if let Some(clip) = scene.label_clips.get(&index) {
                        target.push_clip_path(clip)?;
                        let painted =
                            if let Some(stroke) = scene.typography.label_strokes.get(&index) {
                                target.text_with_stroke(label, &font, explicit, *stroke)
                            } else if explicit {
                                target.text_with_font(label, &font)
                            } else {
                                target.text(label)
                            };
                        let restored = target.pop_clip_path();
                        painted.and(restored)?;
                    } else if let Some(stroke) = scene.typography.label_strokes.get(&index) {
                        target.text_with_stroke(label, &font, explicit, *stroke)?;
                    } else if explicit {
                        target.text_with_font(label, &font)?;
                    } else {
                        target.text(label)?;
                    }
                }
            }
            if group_ends.contains_key(&position)
                && let Some(index) = active_group.take()
            {
                target.end_mark_blend(
                    crate::BlendMode::Normal,
                    scene.opacity_groups[index].opacity,
                    true,
                )?;
            }
        }
        Ok(())
    })();
    let restored = if let Some(index) = active_group.take() {
        target.end_mark_blend(
            crate::BlendMode::Normal,
            scene.opacity_groups[index].opacity,
            false,
        )
    } else {
        Ok(())
    };
    let finished = target.finish();
    painted.and(restored).and(finished)
}

fn draw_mark(
    target: &mut impl Canvas,
    mark: &Mark,
    point_ink: Option<&Geometry>,
    opacity: f64,
) -> Result<()> {
    replay_path(target, &mark.geometry)?;
    if let Some(paint) = &mark.fill_paint {
        target.fill_paint(paint, opacity)?;
    } else if let Some(fill) = mark.fill {
        target.fill(fill, opacity)?;
    }
    if let Some(stroke) = mark.stroke {
        if let Some(ink) = point_ink {
            if !matches!(ink, Geometry::Path(commands) if commands.is_empty()) {
                replay_path(target, ink)?;
                target.fill(stroke.color, opacity)?;
            }
        } else if let Some(dash) = &mark.stroke_dash {
            target.stroke_dashed(&mark.geometry, stroke, dash, opacity)?;
        } else {
            target.stroke(stroke, opacity)?;
        }
    }
    Ok(())
}
/// Pure geometry recognition shared with isolation admission. A normalized
/// point stroke has one paint, regardless of the authored zero-area fill.
pub(crate) fn point_stroke_position(geometry: &Geometry) -> Option<glory::render::GraphicPoint> {
    match geometry {
        Geometry::Circle { x, y, radius: 0. }
        | Geometry::Rect {
            x,
            y,
            width: 0.,
            height: 0.,
        } => Some(glory::render::GraphicPoint::new(*x, *y)),
        Geometry::Path(commands) => {
            let mut point = None;
            let mut drawable = false;
            for command in commands {
                match command {
                    PathCommand::Move(p) => {
                        if point.is_some_and(|q| q != *p) {
                            return None;
                        }
                        point = Some(*p);
                    }
                    PathCommand::Line(p) => {
                        if point != Some(*p) {
                            return None;
                        }
                        drawable = true;
                    }
                    PathCommand::Cubic { first, second, end } => {
                        if [first, second, end].iter().any(|p| point != Some(**p)) {
                            return None;
                        }
                        drawable = true;
                    }
                    PathCommand::Close => {
                        drawable |= point.is_some();
                    }
                }
            }
            point.filter(|_| drawable)
        }
        _ => None,
    }
}

/// An actual single-point stroke footprint, including the dash's start phase.
/// None means ordinary geometry. Some(empty) is a handled stroke with no ink.
/// Shared with typed Exact picking; bare Scene legacy picking is unchanged.
pub(crate) fn point_stroke_footprint(mark: &Mark) -> Result<Option<Geometry>> {
    let Some(stroke) = mark.stroke else {
        return Ok(None);
    };
    let point = point_stroke_position(&mark.geometry);
    let Some(point) = point else {
        return Ok(None);
    };
    if stroke.cap == StrokeCap::Butt
        || stroke.width == 0.
        || stroke.color.3 == 0
        || mark.opacity == 0.
    {
        return Ok(Some(Geometry::Path(vec![])));
    }
    if let Some(dash) = &mark.stroke_dash {
        let normalized = dash.normalized()?;
        if !normalized.segments.is_empty()
            && crate::dash_geometry_detailed(&mark.geometry, dash, &Default::default())?
                .dots
                .is_empty()
        {
            return Ok(Some(Geometry::Path(vec![])));
        }
    }
    crate::dash_dot_geometry(
        &crate::DashDot {
            point,
            incoming: [1., 0.],
            outgoing: [1., 0.],
        },
        stroke.width,
        stroke.cap,
    )
    .map(Some)
}
/// Normalize single-point strokes to one actual filled ink footprint. Browsers
/// disagree on native zero-length dashed paths, so static and retained SVG use
/// the same SVG2 endpoint dash rule as Canvas, CPU and typed Exact picking. The
/// original Scene/business geometry and its anchor remain unchanged.
pub(crate) fn paint_mark(mark: &Mark) -> Result<std::borrow::Cow<'_, Mark>> {
    let Some(geometry) = point_stroke_footprint(mark)? else {
        return Ok(std::borrow::Cow::Borrowed(mark));
    };
    let mut paint = mark.clone();
    paint.geometry = geometry;
    paint.fill = mark.stroke.map(|stroke| stroke.color);
    paint.fill_paint = None;
    paint.stroke = None;
    paint.stroke_dash = None;
    Ok(std::borrow::Cow::Owned(paint))
}
fn replay_path<T: Canvas + ?Sized>(target: &mut T, geometry: &Geometry) -> Result<()> {
    target.begin_path()?;
    for c in geometry.commands() {
        match c {
            PathCommand::Move(p) => target.move_to(p.x, p.y)?,
            PathCommand::Line(p) => target.line_to(p.x, p.y)?,
            PathCommand::Cubic { first, second, end } => {
                target.cubic_to(first.x, first.y, second.x, second.y, end.x, end.y)?
            }
            PathCommand::Close => target.close_path()?,
        }
    }
    Ok(())
}

fn js_string(value: &str) -> String {
    let mut out = String::from("\"");
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '<' => out.push_str("\\u003c"),
            '>' => out.push_str("\\u003e"),
            '&' => out.push_str("\\u0026"),
            '\u{2028}' => out.push_str("\\u2028"),
            '\u{2029}' => out.push_str("\\u2029"),
            ch if ch.is_control() => {
                write!(out, "\\u{:04x}", ch as u32).expect("String write");
            }
            ch => out.push(ch),
        }
    }
    out.push('"');
    out
}

mod script_options;
pub use script_options::{CanvasScriptFallback, CanvasScriptOptions, CanvasScriptSurface};

struct ScriptCanvas {
    options: CanvasScriptOptions,
    frame: [f64; 2],
    offscreen_blend_pixels: String,
    code: String,
    id: String,
    title: String,
    patterns: std::collections::BTreeMap<String, (crate::PatternFill, usize)>,
    blends: String,
}
impl Canvas for ScriptCanvas {
    fn supports_group_opacity(&self) -> bool {
        true
    }
    fn prepare_mark_blends(
        &mut self,
        scene: &Scene,
        blends: &crate::blend::MarkBlends,
    ) -> Result<()> {
        self.blends = blend_script::preflight(scene, blends);
        if matches!(
            self.options.surface,
            CanvasScriptSurface::Offscreen2d { .. }
        ) {
            self.offscreen_blend_pixels = blend_script::scratch_pixels(scene, blends);
        }
        Ok(())
    }
    fn begin_mark_blend(&mut self, mark_index: usize) -> Result<bool> {
        write!(self.code, "{{const layer=blendRegions[{mark_index}];if(layer){{const [x,y,w,h]=layer;const source=blendSource;source.save();try{{source.setTransform(1,0,0,1,0,0);source.globalAlpha=1;source.globalCompositeOperation='source-over';source.clearRect(0,0,w,h);source.beginPath();source.rect(0,0,w,h);source.clip();source.setTransform(dpr,0,0,dpr,-x,-y);ctx=source;").unwrap();
        Ok(true)
    }
    fn end_mark_blend(
        &mut self,
        mode: crate::blend::BlendMode,
        opacity: f64,
        commit: bool,
    ) -> Result<()> {
        self.code
            .push_str("}finally{ctx=blendDestination;source.restore();}");
        if commit {
            write!(self.code, "ctx.save();try{{ctx.setTransform(1,0,0,1,0,0);ctx.globalCompositeOperation='{}';ctx.globalAlpha={opacity};ctx.imageSmoothingEnabled=false;ctx.drawImage(blendScratch,0,0,w,h,x,y,w,h);}}finally{{ctx.restore();}}", mode.canvas_name()).unwrap();
        }
        self.code.push_str("}}");
        Ok(())
    }
    fn supports_translucent_dash_dots(&self) -> bool {
        true
    }
    fn supports_clip_paths(&self) -> bool {
        true
    }
    fn push_clip_path(&mut self, clip: &ClipPath) -> Result<()> {
        self.code.push_str("ctx.save();");
        replay_path(self, &clip.geometry)?;
        write!(self.code, "ctx.clip('{}');", clip_rule(clip.rule)).unwrap();
        Ok(())
    }
    fn pop_clip_path(&mut self) -> Result<()> {
        self.code.push_str("ctx.restore();");
        Ok(())
    }
    fn stroke_dashed(
        &mut self,
        _: &Geometry,
        stroke: GraphicStroke,
        dash: &StrokeDash,
        opacity: f64,
    ) -> Result<()> {
        let d = dash.native_normalized()?;
        write!(
            self.code,
            "ctx.setLineDash([{}]);ctx.lineDashOffset={};",
            d.segments
                .iter()
                .map(f64::to_string)
                .collect::<Vec<_>>()
                .join(","),
            d.offset
        )
        .unwrap();
        self.stroke(stroke, opacity)?;
        self.code
            .push_str("ctx.setLineDash([]);ctx.lineDashOffset=0;");
        Ok(())
    }
    fn supports_fill_paint(&self) -> bool {
        true
    }
    fn fill_paint(&mut self, paint: &FillPaint, opacity: f64) -> Result<()> {
        if let FillPaint::Pattern(pattern) = paint {
            let slot = self.patterns[&pattern.deterministic_id()].1;
            write!(
                self.code,
                "{{ctx.globalAlpha={opacity};ctx.fillStyle=pattern{slot};ctx.fill();}}"
            )
            .unwrap();
            return Ok(());
        }
        self.code.push_str("{const gradient=");
        match paint {
            FillPaint::Linear { start, end, .. } => write!(
                self.code,
                "ctx.createLinearGradient({},{},{},{});",
                start[0], start[1], end[0], end[1]
            ),
            FillPaint::Radial { center, radius, .. } => write!(
                self.code,
                "ctx.createRadialGradient({},{},0,{},{},{});",
                center[0], center[1], center[0], center[1], radius
            ),
            FillPaint::Pattern(_) => unreachable!(),
        }
        .unwrap();
        for stop in paint.stops() {
            write!(
                self.code,
                "gradient.addColorStop({},{});",
                stop.offset,
                js_string(&css_color(stop.color))
            )
            .unwrap();
        }
        write!(
            self.code,
            "ctx.globalAlpha={opacity};ctx.fillStyle=gradient;ctx.fill();}}"
        )
        .unwrap();
        Ok(())
    }
    fn supports_text_rotation(&self) -> bool {
        true
    }
    fn supports_text_whitespace(&self) -> bool {
        true
    }
    fn supports_clipping(&self) -> bool {
        true
    }
    fn push_clip(&mut self, c: ClipRect) -> Result<()> {
        write!(
            self.code,
            "ctx.save();ctx.beginPath();ctx.rect({},{},{},{});ctx.clip();",
            c.x, c.y, c.width, c.height
        )
        .expect("String write");
        Ok(())
    }
    fn pop_clip(&mut self) -> Result<()> {
        self.code.push_str("ctx.restore();");
        Ok(())
    }
    fn begin(&mut self, w: f64, h: f64, bg: Color) -> Result<()> {
        let mut preflight = String::from("let patternPixels=0,patternWork=0;");
        for (p, _) in self.patterns.values() {
            let tile = p.tile()?;
            write!(preflight, "{{const w=Math.max(1,Math.ceil({}*dpr)),h=Math.max(1,Math.ceil({}*dpr));patternPixels+=w*h;patternWork+=w*h*{};if(w>4096||h>4096||w*h>4194304||w*h*{}>67108864||patternPixels>16777216||patternWork>268435456)throw Error('canvas scene pattern budget exceeded');}}", tile.size[0],tile.size[1],tile.shapes.len()+1,tile.shapes.len()+1).unwrap();
        }
        let pattern_budget_len = preflight.len();
        if !self.patterns.is_empty() {
            for (_, slot) in self.patterns.values() {
                write!(preflight, "let pattern{slot};").unwrap();
            }
            preflight.push_str("ctx.save();try{ctx.setTransform(dpr,0,0,dpr,0,0);");
            for (p, slot) in self.patterns.values() {
                write!(
                    preflight,
                    "pattern{slot}={};",
                    p.canvas_script_current_transform()?
                )
                .unwrap();
            }
            preflight.push_str("}finally{ctx.restore();}");
        }
        self.frame = [w, h];
        if matches!(
            self.options.surface,
            CanvasScriptSurface::Offscreen2d { .. }
        ) {
            script_options::begin(
                self,
                w,
                h,
                bg,
                &preflight[..pattern_budget_len],
                &preflight[pattern_budget_len..],
            );
            return Ok(());
        }
        let binding = if self.blends.is_empty() {
            "const"
        } else {
            "let"
        };
        write!(self.code,"(()=>{{const canvas=document.getElementById({});if(!canvas)throw Error('Canvas element missing');{binding} ctx=canvas.getContext('2d');if(!ctx)throw Error('Canvas 2D unavailable');const dpr=Math.min(4,Math.max(1,window.devicePixelRatio||1),Math.sqrt(67000000/({w}*{h})));{}{preflight}canvas.width=Math.ceil({w}*dpr);canvas.height=Math.ceil({h}*dpr);canvas.style.width='{w}px';canvas.style.height='{h}px';canvas.setAttribute('role','img');canvas.setAttribute('aria-label',{});ctx.setTransform(dpr,0,0,dpr,0,0);ctx.save();ctx.direction='ltr';ctx.fillStyle={};ctx.fillRect(0,0,{w},{h});ctx.beginPath();ctx.rect(0,0,{w},{h});ctx.clip();",js_string(&self.id),self.blends,js_string(&self.title),js_string(&css_color(bg))).expect("String write");
        Ok(())
    }
    fn begin_path(&mut self) -> Result<()> {
        self.code.push_str("ctx.beginPath();");
        Ok(())
    }
    fn move_to(&mut self, x: f64, y: f64) -> Result<()> {
        write!(self.code, "ctx.moveTo({x},{y});").expect("String write");
        Ok(())
    }
    fn line_to(&mut self, x: f64, y: f64) -> Result<()> {
        write!(self.code, "ctx.lineTo({x},{y});").expect("String write");
        Ok(())
    }
    fn cubic_to(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, x: f64, y: f64) -> Result<()> {
        write!(self.code, "ctx.bezierCurveTo({x1},{y1},{x2},{y2},{x},{y});").expect("String write");
        Ok(())
    }
    fn close_path(&mut self) -> Result<()> {
        self.code.push_str("ctx.closePath();");
        Ok(())
    }
    fn fill(&mut self, color: Color, opacity: f64) -> Result<()> {
        write!(
            self.code,
            "ctx.globalAlpha={opacity};ctx.fillStyle={};ctx.fill();",
            js_string(&css_color(color))
        )
        .expect("String write");
        Ok(())
    }
    fn stroke(&mut self, stroke: GraphicStroke, opacity: f64) -> Result<()> {
        write!(self.code,"ctx.globalAlpha={opacity};ctx.strokeStyle={};ctx.lineWidth={};ctx.lineCap={};ctx.miterLimit=4;ctx.stroke();",js_string(&css_color(stroke.color)),stroke.width,js_string(cap(stroke.cap))).expect("String write");
        Ok(())
    }
    fn text(&mut self, label: &Label) -> Result<()> {
        self.text_with_font(label, &ResolvedFont::default())
    }
    fn supports_text_fonts(&self) -> bool {
        true
    }
    fn text_with_font(&mut self, label: &Label, font: &ResolvedFont) -> Result<()> {
        if label.rotation != 0. {
            write!(
                self.code,
                "ctx.save();ctx.translate({},{});ctx.rotate({});",
                label.x,
                label.y,
                label.rotation.to_radians()
            )
            .expect("String write");
        }
        write!(self.code,"ctx.globalAlpha=1;ctx.fillStyle={};ctx.font={};ctx.textAlign={};if('fontKerning' in Object.getPrototypeOf(ctx))ctx.fontKerning='normal';ctx.textBaseline='alphabetic';ctx.fillText({},{},{});",js_string(&css_color(label.color)),js_string(&font.canvas_font(label.size)),js_string(match label.anchor{TextAnchor::Middle=>"center",TextAnchor::Start=>"start",TextAnchor::End=>"end"}),js_string(&label.text),if label.rotation == 0. { label.x } else { 0. },if label.rotation == 0. { label.y } else { 0. }).expect("String write");
        if label.rotation != 0. {
            self.code.push_str("ctx.restore();");
        }
        Ok(())
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
        write!(self.code,"ctx.save();ctx.translate({},{});ctx.rotate({});ctx.globalAlpha=1;ctx.font={};ctx.textAlign={};ctx.textBaseline='alphabetic';if('fontKerning' in Object.getPrototypeOf(ctx))ctx.fontKerning='normal';ctx.fillStyle={};ctx.fillText({},0,0);ctx.strokeStyle={};if({}>0){{ctx.lineWidth={};ctx.lineJoin='round';ctx.strokeText({},0,0);}}ctx.restore();",label.x,label.y,label.rotation.to_radians(),js_string(&font.canvas_font(label.size)),js_string(match label.anchor{TextAnchor::Middle=>"center",TextAnchor::Start=>"start",TextAnchor::End=>"end"}),js_string(&css_color(label.color)),js_string(&label.text),js_string(&css_color(stroke.color)),stroke.width,stroke.width,js_string(&label.text)).expect("String write");
        Ok(())
    }
    fn finish(&mut self) -> Result<()> {
        if matches!(
            self.options.surface,
            CanvasScriptSurface::Offscreen2d { .. }
        ) {
            script_options::finish(self, self.frame[0], self.frame[1]);
        } else {
            self.code.push_str("ctx.restore();})();");
        }
        Ok(())
    }
}

/// Export a standalone Canvas replay for a native-generated gallery. The
/// returned script is safe to embed in an HTML script body: text is JSON
/// escaped, including `<`, `&`, and JavaScript line separators. The application
/// must create the named canvas first. This export does not install events.
pub fn to_canvas_script(scene: &Scene, canvas_id: &str) -> Result<String> {
    to_canvas_script_with_blends(scene, canvas_id, &crate::blend::MarkBlends::default())
}

/// Generate a Canvas replay with the given chart-isolated mark blend modes.
pub fn to_canvas_script_with_blends(
    scene: &Scene,
    canvas_id: &str,
    blends: &crate::blend::MarkBlends,
) -> Result<String> {
    to_canvas_script_with_blends_and_options(
        scene,
        canvas_id,
        blends,
        &CanvasScriptOptions::default(),
    )
}

/// Export with explicit main-thread staging and unavailable-backend behavior.
/// Font family strings remain unchanged. Offscreen font language/inheritance can
/// differ from HTML Canvas; load fonts before executing the synchronous replay.
pub fn to_canvas_script_with_options(
    scene: &Scene,
    canvas_id: &str,
    options: &CanvasScriptOptions,
) -> Result<String> {
    to_canvas_script_with_blends_and_options(
        scene,
        canvas_id,
        &crate::blend::MarkBlends::default(),
        options,
    )
}

/// Export explicit surface options with chart-isolated mark blends.
pub fn to_canvas_script_with_blends_and_options(
    scene: &Scene,
    canvas_id: &str,
    blends: &crate::blend::MarkBlends,
    options: &CanvasScriptOptions,
) -> Result<String> {
    options.validate()?;
    ensure(
        !canvas_id.is_empty() && canvas_id.len() <= 1024,
        "canvas id must be nonempty and at most 1024 bytes",
    )?;
    let mut target = ScriptCanvas {
        options: *options,
        frame: [0., 0.],
        offscreen_blend_pixels: String::from("0"),
        code: String::new(),
        id: canvas_id.into(),
        title: scene.title.clone(),
        patterns: Default::default(),
        blends: String::new(),
    };
    for mark in &scene.marks {
        if let Some(FillPaint::Pattern(p)) = &mark.fill_paint {
            let slot = target.patterns.len();
            target
                .patterns
                .entry(p.deterministic_id())
                .or_insert_with(|| (p.clone(), slot));
        }
    }
    draw_with_blends(scene, &mut target, blends)?;
    Ok(target.code)
}

mod blend_script;

#[cfg(all(feature = "canvas", target_arch = "wasm32"))]
mod browser;
#[cfg(all(feature = "web", target_arch = "wasm32"))]
pub(crate) use browser::{PreparedCanvas, prepare_canvas};
#[cfg(all(feature = "canvas", target_arch = "wasm32"))]
pub use browser::{
    draw_canvas, draw_canvas_with_blends, draw_canvas_with_transform,
    draw_canvas_with_transform_and_blends,
};
#[cfg(all(feature = "canvas", target_arch = "wasm32"))]
pub(crate) use browser::{tree_canvas_label, tree_canvas_mark};
