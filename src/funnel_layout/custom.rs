//! Borrowed stage-body customization; profile borders remain explicit decoration.
use super::*;
use crate::ScatterNodeGeometry;

/// A resolved original stage before custom ink. Coordinates use full Scene pixels.
#[derive(Clone, Copy, Debug)]
pub struct FunnelStageContext<'a> {
    pub datum: &'a FunnelDatum,
    pub source_index: usize,
    pub part: &'a FunnelPart,
    pub geometry: &'a Geometry,
    pub fill: Option<Color>,
    pub fill_paint: Option<&'a FillPaint>,
    pub opacity: f64,
    pub plot: ClipRect,
}

impl FunnelStageContext<'_> {
    /// Inherit stage paint and retain its original open-sided profile border.
    /// Disable `retain_profile_border` for an unrelated replacement silhouette.
    pub fn inherit(&self, geometry: Geometry) -> FunnelStageGeometry {
        FunnelStageGeometry {
            body: ScatterNodeGeometry {
                geometry,
                fill: self.fill,
                fill_paint: self.fill_paint.cloned(),
                stroke: None,
                stroke_dash: None,
                opacity: self.opacity,
                anchor_commands: None,
            },
            retain_profile_border: true,
        }
    }
}

/// One replacement body. Optional body strokes follow the replacement geometry;
/// retained profile borders keep their original geometry, paint and opacity.
#[derive(Clone, Debug, PartialEq)]
pub struct FunnelStageGeometry {
    pub body: ScatterNodeGeometry,
    pub retain_profile_border: bool,
}

/// Build a Funnel with one borrowed callback per originally drawable body, in
/// source order. Zero-width stages without a body do not invoke the callback.
/// The callback is never retained. A failure returns no candidate frame;
/// application-owned callback side effects cannot be rolled back.
///
/// Rectangles/circles must be centered on `part.center`; use Path for offset ink.
/// Metadata bounds, labels, separators and logical sites keep their original
/// placement. Exact picking follows the new ink, nearest follows the data site.
pub fn funnel_layout_with_stage_factory(
    c: &ChartConfig,
    data: &[FunnelDatum],
    o: &FunnelLayoutOptions,
    factory: &mut impl FnMut(&FunnelStageContext<'_>) -> Result<FunnelStageGeometry>,
) -> Result<FunnelLayout> {
    let candidate = funnel_layout(c, data, o)?;
    customize_stages(c, data, o, candidate, factory, false)
}

pub(super) fn customize_stages(
    c: &ChartConfig,
    data: &[FunnelDatum],
    o: &FunnelLayoutOptions,
    candidate: FunnelLayout,
    factory: &mut impl FnMut(&FunnelStageContext<'_>) -> Result<FunnelStageGeometry>,
    preserve_label_color: bool,
) -> Result<FunnelLayout> {
    customize(c, data, o, candidate, factory, preserve_label_color, false)
}
pub(super) fn customize_all_stages(
    c: &ChartConfig,
    data: &[FunnelDatum],
    o: &FunnelLayoutOptions,
    candidate: FunnelLayout,
    factory: &mut impl FnMut(&FunnelStageContext<'_>) -> Result<FunnelStageGeometry>,
) -> Result<FunnelLayout> {
    // This additive pass follows already resolved style roles. Their results,
    // including explicit label colors, are not inferred or replayed.
    customize(c, data, o, candidate, factory, true, true)
}
fn customize(
    c: &ChartConfig,
    data: &[FunnelDatum],
    o: &FunnelLayoutOptions,
    mut candidate: FunnelLayout,
    factory: &mut impl FnMut(&FunnelStageContext<'_>) -> Result<FunnelStageGeometry>,
    preserve_label_color: bool,
    all_sources: bool,
) -> Result<FunnelLayout> {
    let commands = |geometry: &Geometry| match geometry {
        Geometry::Path(p) => p.len(),
        _ => 0,
    };
    // Budget the final frame, excluding every old body that will be replaced.
    let mut replaced = BTreeSet::new();
    for part in &candidate.parts {
        if candidate.scene.marks.iter().any(|m| m.id == part.mark_id) {
            replaced.insert(part.mark_id.clone());
            replaced.insert(format!("funnel:border:{}", part.datum.id.identity()?));
        }
    }
    let mut count: usize = candidate
        .scene
        .marks
        .iter()
        .filter(|m| !replaced.contains(&m.id))
        .map(|m| commands(&m.geometry))
        .sum();
    let stops = |paint: Option<&FillPaint>| paint.map_or(0, |p| p.stops().len());
    let mut paint_stops: usize = candidate
        .scene
        .marks
        .iter()
        .filter(|m| !replaced.contains(&m.id))
        .map(|m| stops(m.fill_paint.as_ref()))
        .sum();
    let mut remove = BTreeSet::new();
    for part in &mut candidate.parts {
        let Some(slot) = candidate
            .scene
            .marks
            .iter()
            .position(|m| m.id == part.mark_id)
        else {
            let capture = &mut candidate.motion_capture[part.source_index];
            capture.origin = FunnelMotionGeometryOrigin::UncalledStageFactory;
            if all_sources {
                let original = capture.body.as_ref().ok_or_else(|| {
                    crate::ChartError("Funnel stage source geometry is missing".into())
                })?;
                let context = FunnelStageContext {
                    datum: &data[part.source_index],
                    source_index: part.source_index,
                    part,
                    geometry: &original.geometry,
                    fill: original.fill,
                    fill_paint: original.fill_paint.as_ref(),
                    opacity: original.opacity,
                    plot: candidate.plot,
                };
                let custom = factory(&context)?;
                let explicit_fill = custom.body.fill;
                let prepared =
                    custom
                        .body
                        .prepare_at(&part.mark_id, part.center, part.color, 200_000)?;
                // prepare_at validates anchoring, not the complete Mark paint.
                // An inactive result still needs full constructor admission;
                // this temporary validation never enters the returned Scene.
                let mut validation = c.scene()?;
                validation.marks.push(prepared.mark.clone());
                validation.validate()?;
                count = count
                    .checked_add(commands(&prepared.mark.geometry))
                    .ok_or_else(|| {
                        crate::ChartError("custom Funnel latent geometry budget overflow".into())
                    })?;
                ensure(
                    count <= 200_000,
                    "custom Funnel latent geometry budget exceeded",
                )?;
                paint_stops = paint_stops
                    .checked_add(stops(prepared.mark.fill_paint.as_ref()))
                    .ok_or_else(|| {
                        crate::ChartError("custom Funnel latent paint budget overflow".into())
                    })?;
                ensure(
                    paint_stops <= crate::MAX_SCENE_GRADIENT_STOPS,
                    "custom Funnel latent paint budget exceeded",
                )?;
                capture.body = Some(FunnelMotionInk {
                    geometry: prepared.mark.geometry,
                    // prepare_at omits alpha-zero solid paint. Its explicitly
                    // returned numeric color remains a valid spring target.
                    fill: explicit_fill,
                    fill_paint: prepared.mark.fill_paint,
                    opacity: prepared.mark.opacity,
                    stroke: prepared.mark.stroke,
                });
                capture.origin = FunnelMotionGeometryOrigin::StageFactory;
                part.color = prepared.color;
                for legend in &mut candidate.scene.legend {
                    if legend.id == part.series {
                        legend.color = prepared.color;
                    }
                }
                if !custom.retain_profile_border {
                    remove.insert(format!("funnel:border:{}", part.datum.id.identity()?));
                }
            }
            continue;
        };
        let original = &candidate.scene.marks[slot];
        let context = FunnelStageContext {
            datum: &data[part.source_index],
            source_index: part.source_index,
            part,
            geometry: &original.geometry,
            fill: original.fill,
            fill_paint: original.fill_paint.as_ref(),
            opacity: original.opacity,
            plot: candidate.plot,
        };
        let custom = factory(&context)?;
        let capture = &mut candidate.motion_capture[part.source_index];
        capture.origin = FunnelMotionGeometryOrigin::StageFactory;
        capture.body = Some(FunnelMotionInk {
            geometry: custom.body.geometry.clone(),
            fill: custom.body.fill,
            fill_paint: custom.body.fill_paint.clone(),
            opacity: custom.body.opacity,
            stroke: custom.body.stroke,
        });
        // Preserve legacy anchors and source colors for exact inheritance,
        // including gradients whose representative ink differs from datum color.
        let inherited = custom == context.inherit(context.geometry.clone());
        if let Some(paint) = &custom.body.fill_paint {
            paint.validate()?;
        }
        paint_stops = paint_stops
            .checked_add(stops(custom.body.fill_paint.as_ref()))
            .ok_or_else(|| ChartError("custom Funnel paint budget overflow".into()))?;
        ensure(
            paint_stops <= crate::MAX_SCENE_GRADIENT_STOPS,
            "custom Funnel paint budget exceeded",
        )?;
        let border_id = format!("funnel:border:{}", part.datum.id.identity()?);
        if custom.retain_profile_border {
            if let Some(border) = candidate.scene.marks.iter().find(|m| m.id == border_id) {
                count += commands(&border.geometry);
            }
        } else {
            remove.insert(border_id);
        }
        count = count
            .checked_add(commands(&custom.body.geometry))
            .ok_or_else(|| ChartError("custom Funnel command count overflow".into()))?;
        ensure(count <= 200_000, "custom Funnel geometry budget exceeded")?;
        if inherited {
            continue;
        }
        let prepared = custom
            .body
            .prepare_at(&part.mark_id, part.center, part.color, 200_000)?;
        let mut mark = prepared.mark;
        // Keep the explicit factory style captured above, including a Some
        // transparent fill normalized away by prepare_at. Actual Scene paint
        // still controls whether any of these channels can draw ink.
        mark.series = original.series.clone();
        mark.label = original.label.clone();
        mark.value = original.value;
        mark.clip = original.clip;
        mark.clip_path = original.clip_path.clone();
        // A fresh anchor must describe custom geometry, never the old profile.
        candidate.interaction.anchors.remove(&part.mark_id);
        if let Some(anchor) = prepared.anchor {
            candidate
                .interaction
                .anchors
                .insert(part.mark_id.clone(), anchor);
        }
        for site in &mut candidate.interaction.sites {
            if site.mark_id == part.mark_id {
                site.color = prepared.color;
            }
        }
        part.color = prepared.color;
        for legend in &mut candidate.scene.legend {
            if legend.id == part.series {
                legend.color = prepared.color;
            }
        }
        if !preserve_label_color && let Some(labels) = &o.labels {
            let color = labels.color.resolve_validated(InheritedColorContext {
                inherited: prepared.color,
                theme: c.theme.background,
                surface: prepared.color,
                backdrop: c.theme.background,
            })?;
            for (&slot, series) in &candidate.scene.label_series {
                if series == &part.series {
                    candidate.scene.labels[slot].color = color;
                }
            }
        }
        candidate.scene.marks[slot] = mark;
    }
    candidate.scene.marks.retain(|m| !remove.contains(&m.id));
    candidate.interaction.validate(&candidate.scene)?;
    candidate.refresh_svg_capture();
    Ok(candidate)
}
