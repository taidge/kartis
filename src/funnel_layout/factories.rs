//! Borrowed source-color, value-format and computed-stage color roles.
use super::*;

#[derive(Clone, Copy, Debug)]
pub struct FunnelColorContext<'a> {
    pub datum: &'a FunnelDatum,
    pub source_index: usize,
    pub default_color: Color,
    pub plot: ClipRect,
}
/// Current typed Rust stage geometry and style inputs. Unlike Nivo's temporary
/// pre-profile object, `part.profile` is already computed here. Label color sees
/// the resolved border color, including an explicit border-color factory.
#[derive(Clone, Copy, Debug)]
pub struct FunnelPartStyleContext<'a> {
    pub part: &'a FunnelPart,
    pub plot: ClipRect,
    pub default_color: Color,
    pub fill_opacity: f64,
    pub border_width: f64,
    pub border_opacity: f64,
    pub border_color: Color,
}
pub type FunnelColorFactory<'a> = dyn FnMut(&FunnelColorContext<'_>) -> Result<Color> + 'a;
pub type FunnelStyleColorFactory<'a> = dyn FnMut(&FunnelPartStyleContext<'_>) -> Result<Color> + 'a;
pub type FunnelValueFormatter<'a> = dyn FnMut(f64) -> Result<String> + 'a;

/// Functions are borrowed for one build, never retained. Color and formatter
/// run once per source stage; border/label factories run once per computed stage
/// even when its body, border or label is not painted. All source validation and
/// size evaluation precedes these roles. Errors return no candidate; external
/// callback side effects cannot be rolled back.
#[derive(Default)]
pub struct FunnelFactories<'a> {
    /// Overrides the palette/explicit datum-color result without changing data.
    pub color: Option<&'a mut FunnelColorFactory<'a>>,
    pub formatted_value: Option<&'a mut FunnelValueFormatter<'a>>,
    pub border_color: Option<&'a mut FunnelStyleColorFactory<'a>>,
    pub label_color: Option<&'a mut FunnelStyleColorFactory<'a>>,
}
pub(super) struct ResolvedColors {
    pub border: Color,
    pub label: Option<Color>,
    pub computed_label: Option<Color>,
}
impl FunnelFactories<'_> {
    pub(super) fn colors(
        &mut self,
        c: &ChartConfig,
        o: &FunnelLayoutOptions,
        part: &FunnelPart,
    ) -> Result<ResolvedColors> {
        let context = InheritedColorContext {
            inherited: part.color,
            theme: c.theme.text,
            surface: part.color,
            backdrop: c.theme.background,
        };
        let border_width = if part.is_current {
            o.current_border_width.unwrap_or(o.border_width)
        } else {
            o.border_width
        };
        // Existing disabled roles must keep their previous admission rules:
        // an unused Contrast border need not resolve on a transparent surface.
        // Explicit style callbacks request the computed default context.
        let needs_border = border_width > 0. && o.border_opacity > 0.
            || self.border_color.is_some()
            || self.label_color.is_some();
        let default_border = if needs_border {
            o.border_color.resolve_validated(context)?
        } else {
            part.color
        };
        let mut stage = FunnelPartStyleContext {
            part,
            plot: ClipRect::plot(c),
            default_color: default_border,
            fill_opacity: o.fill_opacity,
            border_width,
            border_opacity: o.border_opacity,
            border_color: default_border,
        };
        let border = match &mut self.border_color {
            Some(factory) => factory(&stage)?,
            None => default_border,
        };
        let computed_label = if o.labels.is_some() || self.label_color.is_some() {
            let labels = o.labels.clone().unwrap_or_default();
            let default = labels.color.resolve_validated(InheritedColorContext {
                theme: c.theme.background,
                ..context
            })?;
            stage.default_color = default;
            stage.border_color = border;
            let color = match &mut self.label_color {
                Some(factory) => factory(&stage)?,
                None => default,
            };
            Some(color)
        } else {
            None
        };
        Ok(ResolvedColors {
            border,
            label: o.labels.as_ref().and(computed_label),
            computed_label,
        })
    }
}
pub fn funnel_layout_with_factories(
    c: &ChartConfig,
    data: &[FunnelDatum],
    o: &FunnelLayoutOptions,
    factories: &mut FunnelFactories<'_>,
) -> Result<FunnelLayout> {
    funnel_layout_with_size_and_factories(
        c,
        data,
        o,
        |ctx| {
            Ok(match &o.size {
                FunnelSize::Value => ctx.scaled_size,
                FunnelSize::DatumPixels => ctx.datum.size.unwrap_or(0.),
                FunnelSize::OrdinalPixels(sizes) => sizes[ctx.source_index % sizes.len()],
            })
        },
        factories,
    )
}
/// Existing size-only entry, with the same default output and validation.
pub fn funnel_layout_with_size(
    c: &ChartConfig,
    data: &[FunnelDatum],
    o: &FunnelLayoutOptions,
    size: impl FnMut(FunnelSizeContext<'_>) -> Result<f64>,
) -> Result<FunnelLayout> {
    super::build_with_factories(c, data, o, size, &mut FunnelFactories::default())
}
pub fn funnel_layout_with_size_and_factories(
    c: &ChartConfig,
    data: &[FunnelDatum],
    o: &FunnelLayoutOptions,
    size: impl FnMut(FunnelSizeContext<'_>) -> Result<f64>,
    factories: &mut FunnelFactories<'_>,
) -> Result<FunnelLayout> {
    super::build_with_factories(c, data, o, size, factories)
}
/// Style/format factories resolve before custom ink. Explicit label-color output
/// is retained; without it, custom body color still updates inherited labels as
/// in the original stage-geometry API. Color roles are never replayed per body.
pub fn funnel_layout_with_factories_and_stage_factory(
    c: &ChartConfig,
    data: &[FunnelDatum],
    o: &FunnelLayoutOptions,
    factories: &mut FunnelFactories<'_>,
    stage: &mut impl FnMut(&FunnelStageContext<'_>) -> Result<FunnelStageGeometry>,
) -> Result<FunnelLayout> {
    let preserve_label_color = factories.label_color.is_some();
    let layout = funnel_layout_with_factories(c, data, o, factories)?;
    super::custom::customize_stages(c, data, o, layout, stage, preserve_label_color)
}

/// Compose all borrowed size/style roles with real stage geometry once.
pub fn funnel_layout_with_size_factories_and_stage_factory(
    c: &ChartConfig,
    data: &[FunnelDatum],
    o: &FunnelLayoutOptions,
    size: impl FnMut(FunnelSizeContext<'_>) -> Result<f64>,
    factories: &mut FunnelFactories<'_>,
    stage: &mut impl FnMut(&FunnelStageContext<'_>) -> Result<FunnelStageGeometry>,
) -> Result<FunnelLayout> {
    let preserve_label_color = factories.label_color.is_some();
    let layout = funnel_layout_with_size_and_factories(c, data, o, size, factories)?;
    super::custom::customize_stages(c, data, o, layout, stage, preserve_label_color)
}
