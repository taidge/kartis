use super::*;

/// A real visible nonempty summary, before source-color resolution.
///
/// Full-domain indices include reserved and hidden slots. The original input
/// indices include nullable records in caller order; statistics omit None.
/// `quantile_probabilities` are configuration, while `computed_box.quantiles`
/// are statistics. Position/geometry are the genuine ordinary projected layout,
/// including collapsed or cropped boxes, rather than fabricated hit geometry.
#[derive(Clone, Copy, Debug)]
pub struct BoxPlotColorContext<'a> {
    pub computed_box: &'a ComputedBox,
    pub data: &'a [BoxObservation],
    pub observation_indices: &'a [usize],
    pub summary_index: usize,
    pub group_index: usize,
    pub subgroup_index: usize,
    pub cell_index: usize,
    pub color_index: usize,
    pub color_by: BoxPlotColorBy,
    pub raw_color_key: &'a str,
    pub quantile_probabilities: [f64; 5],
    /// Original fixed/palette color, before this callback or inherited roles.
    pub default_color: Color,
}

/// Borrowed during construction/replacement only; never retained or re-called
/// during hover. Returns typed straight RGBA, without premultiplying opacity.
pub type BoxPlotColorFactory<'f> =
    dyn for<'a> FnMut(&BoxPlotColorContext<'a>) -> Result<Color> + 'f;

/// Per-build callbacks. Every eligible color runs before any legend callback.
///
/// Color runs once per actual visible nonempty summary, including zero extent
/// and cropped summaries, even with legend=false. Legend text retains its
/// ByLabel eligibility and first-representative semantics. Neither callback is
/// retained. Failure preserves inputs/accepted frames, not external side effects.
///
/// Unlike Nivo's function scale, this interface does not call partial-domain
/// priming objects or re-run colors during hover. It does not coerce None to 0,
/// expose arbitrary object-property access, or erase RGB at typed alpha zero.
#[derive(Default)]
pub struct BoxPlotFactories<'a> {
    pub color: Option<&'a mut BoxPlotColorFactory<'a>>,
    pub legend_label: Option<&'a mut BoxPlotLegendLabelFactory<'a>>,
}

/// Build with borrowed source-color and/or ByLabel text factories.
///
/// Both None use the exact legacy path. Known configuration/source/scale/paint
/// and projected geometry are preflighted before any callback. Inherited role
/// contexts depend on the returned color and may reject after a callback.
///
/// ```
/// use kartis::*;
/// let c = ChartConfig::default();
/// let data = [BoxObservation::new("a", "A", "x", Some(2.))];
/// let mut color = |ctx: &BoxPlotColorContext<'_>| {
///     Ok(if ctx.computed_box.mean > 1. { Color(20, 90, 140, 255) }
///        else { ctx.default_color })
/// };
/// let layout = boxplot_layout_with_factories(&c, &data,
///     &BoxPlotLayoutOptions::default(),
///     BoxPlotFactories { color: Some(&mut color), legend_label: None })?;
/// # Ok::<(), ChartError>(())
/// ```
pub fn boxplot_layout_with_factories(
    c: &ChartConfig,
    data: &[BoxObservation],
    o: &BoxPlotLayoutOptions,
    factories: BoxPlotFactories<'_>,
) -> Result<BoxPlotLayout> {
    let Some(color) = factories.color else {
        return match factories.legend_label {
            Some(label) => boxplot_layout_with_legend_label_factory(c, data, o, label),
            None => boxplot_layout(c, data, o),
        };
    };
    validate_factories(o, factories.legend_label.is_some())?;
    let preflight = boxplot_layout(c, data, &preflight_options(o))?;
    let colors = resolve_colors(c, data, o, &preflight, color)?;
    let (layout, ()) = build_boxplot_layout(
        c,
        data,
        o,
        o.scale,
        false,
        factories.legend_label,
        Some(&colors),
        |domain, range| Ok((ChartScale::new(o.scale, domain, range)?, ())),
    )?;
    Ok(layout)
}

/// The same per-summary factories with a configured continuous value scale.
/// Known scale errors reject before callbacks; the configured scale and source
/// projection remain identical to the non-factory builder.
pub fn boxplot_layout_with_scale_options_and_factories(
    c: &ChartConfig,
    data: &[BoxObservation],
    o: &BoxPlotLayoutOptions,
    value_scale: &ContinuousScaleOptions,
    factories: BoxPlotFactories<'_>,
) -> Result<ScaledBoxPlotLayout> {
    let Some(color) = factories.color else {
        return match factories.legend_label {
            Some(label) => boxplot_layout_with_scale_options_and_legend_label_factory(
                c,
                data,
                o,
                value_scale,
                label,
            ),
            None => boxplot_layout_with_scale_options(c, data, o, value_scale),
        };
    };
    validate_factories(o, factories.legend_label.is_some())?;
    let preflight = boxplot_layout_with_scale_options(c, data, &preflight_options(o), value_scale)?;
    let colors = resolve_colors(c, data, o, &preflight.layout, color)?;
    let (layout, value_scale) = build_boxplot_layout(
        c,
        data,
        o,
        value_scale.kind,
        true,
        factories.legend_label,
        Some(&colors),
        |domain, range| {
            let resolved = ResolvedContinuousScale::new(domain, range, value_scale)?;
            Ok((resolved.chart_scale().clone(), resolved))
        },
    )?;
    Ok(ScaledBoxPlotLayout {
        layout,
        value_scale,
    })
}

fn validate_factories(o: &BoxPlotLayoutOptions, has_label: bool) -> Result<()> {
    ensure(
        !has_label || o.legend_mode == BoxPlotLegendMode::ByLabel,
        "BoxPlot legend factory requires ByLabel mode",
    )?;
    for profile in [
        &o.border_color_profile,
        &o.median_color_profile,
        &o.whisker_color_profile,
    ]
    .into_iter()
    .flatten()
    {
        profile.validate()?;
    }
    Ok(())
}

fn preflight_options(o: &BoxPlotLayoutOptions) -> BoxPlotLayoutOptions {
    // Intrinsic profiles were checked above. Context-dependent Contrast must
    // use the factory's actual returned surface, not this provisional palette.
    let mut preflight = o.clone();
    preflight.border_color_profile = None;
    preflight.median_color_profile = None;
    preflight.whisker_color_profile = None;
    preflight
}

fn resolve_colors(
    c: &ChartConfig,
    data: &[BoxObservation],
    o: &BoxPlotLayoutOptions,
    preflight: &BoxPlotLayout,
    factory: &mut BoxPlotColorFactory<'_>,
) -> Result<BTreeMap<String, Color>> {
    let groups: BTreeMap<_, _> = preflight
        .groups
        .iter()
        .enumerate()
        .map(|(i, key)| (key.as_str(), i))
        .collect();
    let subgroups: BTreeMap<_, _> = preflight
        .subgroups
        .iter()
        .enumerate()
        .map(|(i, key)| (key.as_str(), i))
        .collect();
    let mut cells: BTreeMap<(&str, &str), Vec<usize>> = BTreeMap::new();
    for (index, datum) in data.iter().enumerate() {
        cells
            .entry((&datum.group, &datum.subgroup))
            .or_default()
            .push(index);
    }
    let mut colors = BTreeMap::new();
    for (summary_index, summary) in preflight.boxes.iter().enumerate() {
        let group_index = groups[summary.group.as_str()];
        let subgroup_index = subgroups[summary.subgroup.as_str()];
        let (raw_color_key, color_index) = match o.color_by {
            BoxPlotColorBy::SubGroup => (summary.subgroup.as_str(), subgroup_index),
            BoxPlotColorBy::Group => (summary.group.as_str(), group_index),
        };
        let context = BoxPlotColorContext {
            computed_box: summary,
            data,
            observation_indices: &cells[&(summary.group.as_str(), summary.subgroup.as_str())],
            summary_index,
            group_index,
            subgroup_index,
            cell_index: group_index * preflight.subgroups.len() + subgroup_index,
            color_index,
            color_by: o.color_by,
            raw_color_key,
            quantile_probabilities: o.quantiles,
            default_color: o
                .colors
                .get(raw_color_key)
                .copied()
                .unwrap_or(c.theme.color(color_index)),
        };
        colors.insert(summary.id.clone(), factory(&context)?);
    }
    Ok(colors)
}
