//! Borrowed Waffle style functions over real source groups and an empty-style
//! template. No synthetic datum or physical empty-cell index is invented.
use super::*;

#[derive(Clone, Copy, Debug)]
pub struct WaffleGroupColorContext<'a> {
    pub datum: &'a RadialDatum,
    pub source_index: usize,
    pub default_color: Color,
    pub plot: ClipRect,
    pub grid: ClipRect,
    pub total: f64,
    pub unit: f64,
}

/// One shared empty-cell style, evaluated once even when every cell is occupied.
/// This is not an allocated cell and has no row, column, or business datum.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaffleEmptyStyle {
    pub color: Color,
    pub opacity: f64,
}

#[derive(Clone, Copy, Debug)]
pub enum WaffleBorderTarget<'a> {
    Group(&'a WaffleGroup),
    Empty(WaffleEmptyStyle),
}

#[derive(Clone, Copy, Debug)]
pub struct WaffleBorderColorContext<'a> {
    pub target: WaffleBorderTarget<'a>,
    /// None when the configured inherited color requires an opaque effective
    /// backdrop which this role does not have. An explicit function may handle
    /// that case; without a function the original resolution error is returned.
    pub default_color: Option<Color>,
    pub plot: ClipRect,
    pub grid: ClipRect,
    pub total: f64,
    pub unit: f64,
}

pub type WaffleGroupColorFactory<'a> =
    dyn FnMut(&WaffleGroupColorContext<'_>) -> Result<Color> + 'a;
pub type WaffleValueFormatter<'a> = dyn FnMut(f64) -> Result<String> + 'a;
pub type WaffleBorderColorFactory<'a> =
    dyn FnMut(&WaffleBorderColorContext<'_>) -> Result<Color> + 'a;

#[derive(Default)]
pub struct WaffleFactories<'a> {
    pub color: Option<&'a mut WaffleGroupColorFactory<'a>>,
    pub formatted_value: Option<&'a mut WaffleValueFormatter<'a>>,
    pub border_color: Option<&'a mut WaffleBorderColorFactory<'a>>,
}

/// Build complete ordinary geometry and validate all static inputs/default
/// sources before calling application functions. Each source group (including
/// hidden, zero, and overflow groups) calls color, formatter, then border in
/// source order, followed by exactly one empty-style border call. Border group
/// input has resolved color/format, its original ranges, empty polygons, and a
/// temporary border_color equal to the newly resolved color.
///
/// Callback errors return no candidate; application-owned side effects cannot
/// be rolled back. Functions are borrowed only for this call and never stored.
pub fn waffle_layout_with_factories(
    config: &ChartConfig,
    data: &[RadialDatum],
    options: &WaffleLayoutOptions,
    factories: &mut WaffleFactories<'_>,
) -> Result<WaffleLayout> {
    build_styles(config, data, options, factories, false)
}
pub(crate) fn waffle_svg_layout_with_factories(
    config: &ChartConfig,
    data: &[RadialDatum],
    options: &WaffleLayoutOptions,
    factories: &mut WaffleFactories<'_>,
) -> Result<WaffleLayout> {
    build_styles(config, data, options, factories, true)
}
fn build_styles(
    config: &ChartConfig,
    data: &[RadialDatum],
    options: &WaffleLayoutOptions,
    factories: &mut WaffleFactories<'_>,
    nivo: bool,
) -> Result<WaffleLayout> {
    let mut candidate = layout_base(
        config,
        data,
        options,
        factories.border_color.is_some(),
        nivo,
    )?;
    if factories.color.is_none()
        && factories.formatted_value.is_none()
        && factories.border_color.is_none()
    {
        return Ok(candidate);
    }
    let resolve_border = |fill| {
        options
            .border_color
            .resolve_validated(InheritedColorContext {
                inherited: fill,
                surface: fill,
                backdrop: candidate.scene.background,
                theme: config.theme.text,
            })
    };
    for group in &mut candidate.groups {
        let color = if let Some(function) = &mut factories.color {
            function(&WaffleGroupColorContext {
                datum: &group.source,
                source_index: group.source_index,
                default_color: group.color,
                plot: candidate.plot,
                grid: candidate.grid,
                total: candidate.total,
                unit: candidate.unit,
            })?
        } else {
            group.color
        };
        let formatted_value = if let Some(function) = &mut factories.formatted_value {
            let text = function(group.source.value)?;
            ensure(
                text.len() <= crate::MAX_LABEL_BYTES,
                "Waffle formatted value exceeds 1024 bytes",
            )?;
            text
        } else {
            group.formatted_value.clone()
        };
        let mut input = WaffleGroup {
            source: group.source.clone(),
            source_index: group.source_index,
            formatted_value,
            hidden: group.hidden,
            color,
            border_color: color,
            requested: group.requested.clone(),
            occupied: group.occupied.clone(),
            polygons: Vec::new(),
            area_mark_id: None,
        };
        let default = resolve_border(color);
        input.border_color = if let Some(function) = &mut factories.border_color {
            function(&WaffleBorderColorContext {
                target: WaffleBorderTarget::Group(&input),
                default_color: default.ok(),
                plot: candidate.plot,
                grid: candidate.grid,
                total: candidate.total,
                unit: candidate.unit,
            })?
        } else {
            default?
        };
        input.polygons = std::mem::take(&mut group.polygons);
        input.area_mark_id = group.area_mark_id.take();
        *group = input;
    }
    let default = resolve_border(options.empty_color);
    let empty_border = if let Some(function) = &mut factories.border_color {
        function(&WaffleBorderColorContext {
            target: WaffleBorderTarget::Empty(WaffleEmptyStyle {
                color: options.empty_color,
                opacity: options.empty_opacity,
            }),
            default_color: default.ok(),
            plot: candidate.plot,
            grid: candidate.grid,
            total: candidate.total,
            unit: candidate.unit,
        })?
    } else {
        default?
    };
    let mark_slots: BTreeMap<_, _> = candidate
        .scene
        .marks
        .iter()
        .enumerate()
        .map(|(index, mark)| (mark.id.clone(), index))
        .collect();
    for cell in &candidate.cells {
        if let Some(id) = &cell.mark_id {
            let mark = &mut candidate.scene.marks[mark_slots[id]];
            let group = cell.owner.map(|owner| &candidate.groups[owner]);
            mark.fill = Some(group.map_or(options.empty_color, |g| g.color));
            mark.stroke = border(
                options.border_width,
                group.map_or(empty_border, |g| g.border_color),
            );
        }
    }
    let groups: BTreeMap<_, _> = candidate
        .groups
        .iter()
        .map(|group| (group.source.id.as_str(), group))
        .collect();
    for group in &candidate.groups {
        if let Some(id) = &group.area_mark_id {
            let mark = &mut candidate.scene.marks[mark_slots[id]];
            mark.fill = Some(group.color);
            mark.stroke = border(options.border_width, group.border_color);
        }
    }
    for item in &mut candidate.scene.legend {
        item.color = groups[item.id.as_str()].color;
    }
    for site in &mut candidate.interaction.sites {
        let group = groups[site.series.as_str()];
        site.color = group.color;
        site.formatted_value = Some(group.formatted_value.clone());
    }
    candidate.interaction.validate(&candidate.scene)?;
    Ok(candidate)
}
