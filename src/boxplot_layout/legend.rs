use super::*;

/// Legend topology is explicit: default Keys does not merge captions.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BoxPlotLegendMode {
    #[default]
    Keys,
    /// Nivo's exact-label deduplication, first representative, stable index sort.
    ByLabel,
}

/// A view of one real computed summary and its original input cell.
/// Indices use complete configured domains, including reserved/hidden keys.
/// observation_indices includes nullable records, in caller input order;
/// computed_box.count/statistics/source_ids still omit None values.
#[derive(Clone, Copy, Debug)]
pub struct BoxPlotLegendContext<'a> {
    pub computed_box: &'a ComputedBox,
    pub data: &'a [BoxObservation],
    pub observation_indices: &'a [usize],
    pub summary_index: usize,
    pub group_index: usize,
    pub subgroup_index: usize,
    pub cell_index: usize,
    pub color_index: usize,
    pub color_by: BoxPlotColorBy,
    /// Configured probabilities, not the computed statistic values.
    pub quantile_probabilities: [f64; 5],
    pub raw_color_key: &'a str,
    /// Source color before role modifiers or opacity treatment. The legacy
    /// legend-only entry points use the numeric palette; combined factories
    /// supply the resolved per-summary color.
    pub color: Color,
    /// Map caption when present, otherwise the selected raw key, including "".
    pub default_text: &'a str,
}

/// Borrowed only for this builder/source-replacement call; never stored.
pub type BoxPlotLegendLabelFactory<'f> =
    dyn for<'a> FnMut(&BoxPlotLegendContext<'a>) -> Result<String> + 'f;

struct Row {
    index: usize,
    id: String,
    text: String,
    color: Color,
    target: LegendTarget,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn build(
    scene: &mut Scene,
    c: &ChartConfig,
    data: &[BoxObservation],
    o: &BoxPlotLayoutOptions,
    boxes: &[ComputedBox],
    groups: &[String],
    subgroups: &[String],
    mut factory: Option<&mut BoxPlotLegendLabelFactory<'_>>,
    source_colors: Option<&BTreeMap<String, Color>>,
) -> Result<()> {
    if !o.legend {
        return Ok(());
    }
    let group_indices: BTreeMap<_, _> = groups
        .iter()
        .enumerate()
        .map(|(i, k)| (k.as_str(), i))
        .collect();
    let subgroup_indices: BTreeMap<_, _> = subgroups
        .iter()
        .enumerate()
        .map(|(i, k)| (k.as_str(), i))
        .collect();
    // One pass preserves original nullable input order, independently of the
    // sample sorting used for quantiles. Storage is bounded by source length.
    let mut cell_indices: BTreeMap<(&str, &str), Vec<usize>> = BTreeMap::new();
    for (index, datum) in data.iter().enumerate() {
        cell_indices
            .entry((&datum.group, &datum.subgroup))
            .or_default()
            .push(index);
    }
    let mut seen = BTreeSet::new();
    let mut rows = Vec::new();
    for (summary_index, summary) in boxes.iter().enumerate() {
        let group_index = group_indices[summary.group.as_str()];
        let subgroup_index = subgroup_indices[summary.subgroup.as_str()];
        let (raw_color_key, color_index, tag, target) = match o.color_by {
            BoxPlotColorBy::SubGroup => (
                summary.subgroup.as_str(),
                subgroup_index,
                "s",
                LegendTarget::Series(boxplot_series_id(&summary.subgroup)),
            ),
            BoxPlotColorBy::Group => (
                summary.group.as_str(),
                group_index,
                "g",
                LegendTarget::Category(summary.group.clone()),
            ),
        };
        let color = source_colors
            .and_then(|colors| colors.get(&summary.id))
            .copied()
            .unwrap_or_else(|| {
                o.colors
                    .get(raw_color_key)
                    .copied()
                    .unwrap_or(c.theme.color(color_index))
            });
        let default_text = o
            .legend_labels
            .as_ref()
            .and_then(|labels| labels.get(raw_color_key))
            .map_or(raw_color_key, String::as_str);
        let context = BoxPlotLegendContext {
            computed_box: summary,
            data,
            observation_indices: &cell_indices
                [&(summary.group.as_str(), summary.subgroup.as_str())],
            summary_index,
            group_index,
            subgroup_index,
            cell_index: group_index * subgroups.len() + subgroup_index,
            color_index,
            color_by: o.color_by,
            quantile_probabilities: o.quantiles,
            raw_color_key,
            color,
            default_text,
        };
        let text = if let Some(callback) = factory.as_mut() {
            callback(&context)?
        } else {
            default_text.to_string()
        };
        ensure(
            text.len() <= MAX_LABEL_BYTES,
            "boxplot legend label exceeds byte budget",
        )?;
        // Never budget all candidates as rows: 2048 summaries can become one
        // row. The first 257th distinct caption rejects immediately.
        if !seen.insert(text.clone()) {
            continue;
        }
        ensure(
            rows.len() < 256,
            "boxplot distinct legend label budget exceeded",
        )?;
        rows.push(Row {
            index: color_index,
            // Injective representative source encoding, independent of text.
            // Unique summary IDs permit multiple rows targeting one series.
            id: format!("boxplot-legend:{tag}:{}:{}", summary.id.len(), summary.id),
            text,
            color,
            target,
        });
    }
    // sort_by_key is stable: equal full-domain indices retain source order.
    rows.sort_by_key(|row| row.index);
    for row in rows {
        scene.legend_targets.insert(row.id.clone(), row.target);
        scene.legend.push(LegendItem {
            id: row.id,
            label: row.text,
            color: row.color,
        });
    }
    Ok(())
}
