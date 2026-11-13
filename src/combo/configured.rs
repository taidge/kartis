//! Additive configured scale and typed metadata API for the shared Combo engine.
use super::*;
use crate::{InteractionAnchor, InteractionBinding, InteractionSite, SceneInteraction};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ComboScaleOptions {
    /// None inherits the numeric kind in ComboOptions. Category x requires None.
    pub x: Option<ContinuousScaleOptions>,
    /// None inherits the kind in ComboOptions::left, with other flags off.
    pub left: Option<ContinuousScaleOptions>,
    /// None inherits the kind in ComboOptions::right, with other flags off.
    pub right: Option<ContinuousScaleOptions>,
}

impl ComboScaleOptions {
    pub(super) fn validate(&self, o: &ComboOptions) -> Result<()> {
        ensure(
            self.x.is_none() || matches!(o.x, ComboXAxis::Numeric { .. }),
            "continuous combo x options require a numeric x axis",
        )?;
        for options in [self.x, self.left, self.right].into_iter().flatten() {
            options.validate()?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub enum ResolvedComboXScale {
    Numeric(ResolvedContinuousScale),
    /// The full category domain and the actual projected windowed band scale.
    Categories(BandScale),
}

impl ResolvedComboXScale {
    /// Resolve a raw numeric x or a category's band center. Grouped mark slots
    /// apply an additional offset available in ComboSite::value_position.
    pub fn map(&self, x: &ComboXValue) -> Option<f64> {
        let px = match (self, x) {
            (Self::Numeric(scale), ComboXValue::Number(v)) => scale.map(*v),
            (Self::Categories(scale), ComboXValue::Category(v)) => {
                scale.map(v)? + scale.bandwidth / 2.
            }
            _ => return None,
        };
        px.is_finite().then_some(px)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ComboSite {
    pub mark_id: String,
    pub mark_index: usize,
    pub series_id: String,
    pub series_index: usize,
    pub source_index: usize,
    pub point_id: Option<String>,
    pub kind: ComboKind,
    pub axis: YAxisSide,
    pub raw_x: ComboXValue,
    pub raw_y: f64,
    pub spread: Option<ComboSpread>,
    /// Authored start and plotted end, after stack/percent computation and
    /// before domain clipping. Negative stacks retain the negative direction.
    pub stack_interval: Option<[f64; 2]>,
    /// The raw datum's projected value, or the clipped plotted endpoint for
    /// a bar. For grouped bars/boxes the x coordinate includes the slot offset.
    /// Interval values are clamped to the visible domain for this coordinate.
    pub value_position: [f64; 2],
    /// Live nearest/keyboard anchor. Circle and rectangle marks use their
    /// center. Rounded bars use opposite path supports. Error bars and
    /// whisker-only boxes use the first drawn segment midpoint. Clipping
    /// remains on the actual mark.
    pub position: [f64; 2],
    pub label: String,
    pub formatted_value: String,
    pub color: Color,
}

#[derive(Clone, Debug)]
pub struct ComboLayout {
    pub scene: Scene,
    pub plot: ClipRect,
    pub x_scale: ResolvedComboXScale,
    pub left_scale: ResolvedContinuousScale,
    pub right_scale: ResolvedContinuousScale,
    /// Only emitted datum marks with real support, in final Scene mark order.
    /// Gaps, suppressed points and aggregate paths have no phantom sites.
    pub sites: Vec<ComboSite>,
    pub anchors: BTreeMap<String, InteractionAnchor>,
}

impl ComboLayout {
    pub fn interaction_binding(&self) -> InteractionBinding {
        InteractionBinding {
            sites: self
                .sites
                .iter()
                .map(|site| InteractionSite {
                    mark_id: site.mark_id.clone(),
                    series: site.series_id.clone(),
                    value: Some(site.raw_y),
                    formatted_value: Some(site.formatted_value.clone()),
                    label: site.label.clone(),
                    color: site.color,
                    position: site.position,
                })
                .collect(),
            anchors: self.anchors.clone(),
            ..Default::default()
        }
    }
}

impl From<ComboLayout> for SceneInteraction {
    fn from(layout: ComboLayout) -> Self {
        let interaction = Some(layout.interaction_binding());
        Self {
            scene: layout.scene,
            interaction,
            plot: Some(layout.plot),
        }
    }
}

/// Resolve the same continuous scale for axes, lines, bars, interval marks,
/// point labels and interaction. The original `combo` entry preserves its
/// historical scale profile. Value domains and category windows come from
/// ComboOptions, with explicit scale options overriding the authored kind.
/// Bars and areas require a numeric value axis with a defined zero baseline,
/// including positive powers and Sqrt. Numeric bar and box bands accept Linear,
/// UTC or power x. Scalar/interval UTC values are integral
/// epoch milliseconds. Rounded bars and boxes retain real clipped edges.
pub fn combo_layout_with_scale_options(
    config: &ChartConfig,
    data: &[ComboSeries],
    options: &ComboOptions,
    scales: &ComboScaleOptions,
) -> Result<ComboLayout> {
    let ComboOutput { scene, layout } = combo_impl(config, data, options, Some(scales))?;
    let LayoutParts {
        plot,
        x_scale,
        left_scale,
        right_scale,
        sites,
        anchors,
    } = layout.expect("configured combo metadata");
    let layout = ComboLayout {
        scene,
        plot,
        x_scale,
        left_scale,
        right_scale,
        sites,
        anchors,
    };
    layout.interaction_binding().validate(&layout.scene)?;
    Ok(layout)
}

pub(super) struct LayoutParts {
    pub(super) plot: ClipRect,
    pub(super) x_scale: ResolvedComboXScale,
    pub(super) left_scale: ResolvedContinuousScale,
    pub(super) right_scale: ResolvedContinuousScale,
    pub(super) sites: Vec<ComboSite>,
    pub(super) anchors: BTreeMap<String, InteractionAnchor>,
}

pub(super) fn utc_value(value: f64) -> Result<()> {
    ensure(
        value.fract() == 0. && value.abs() <= crate::MAX_TIMESTAMP_MS as f64,
        "configured combo UTC values must be bounded integral epoch milliseconds",
    )
}

pub(super) struct SiteSource<'a> {
    pub(super) data: &'a [ComboSeries],
    pub(super) rows: &'a [Vec<Row>],
    pub(super) y_scales: &'a [ChartScale],
    pub(super) x: &'a XEngine,
    pub(super) intervals: &'a BTreeMap<(usize, Key), [f64; 2]>,
    pub(super) grouped_slots: &'a [(bool, usize)],
    pub(super) band: f64,
    pub(super) options: &'a ComboOptions,
}

impl SiteSource<'_> {
    fn value_position(&self, si: usize, row: &Row, interval: Option<[f64; 2]>) -> Result<[f64; 2]> {
        let s = &self.data[si];
        let side = axis_index(s.axis);
        let ys = &self.y_scales[side];
        let mut value = interval.map_or(row.y.unwrap(), |i| i[1]);
        let boxed = s.kind == ComboKind::BoxPlot;
        let x = if interval.is_some() || boxed {
            let center = self
                .x
                .bar_pixel(row.x, true)
                .ok_or_else(|| ChartError("missing combo band projection".into()))?;
            let slot_key = if boxed || self.options.bar_layout == BarLayout::Grouped {
                (true, si)
            } else {
                (false, side)
            };
            let slot = self
                .grouped_slots
                .iter()
                .position(|key| *key == slot_key)
                .ok_or_else(|| ChartError("missing combo group slot".into()))?;
            let width = self.band / self.grouped_slots.len() as f64;
            center - self.band / 2. + (slot as f64 + 0.5) * width
        } else {
            self.x
                .pixel(row.x, true)
                .ok_or_else(|| ChartError("missing combo x projection".into()))?
        };
        if interval.is_some() || row.spread.is_some() {
            let [lo, hi] = ordered_domain(ys);
            value = value.clamp(lo, hi);
        }
        Ok([x, ys.map(value)])
    }
}

fn path_anchor(mark: &Mark, kind: ComboKind) -> Result<Option<InteractionAnchor>> {
    let Geometry::Path(path) = &mark.geometry else {
        return Ok(None);
    };
    let indices = match kind {
        // Opposite endpoints in cartesian::round_bar's fixed topology. The
        // extra closing cubic repeats the first support and is unnecessary.
        ComboKind::Bar => [0, 4],
        ComboKind::ErrorBars | ComboKind::BoxPlot => [0, 1],
        _ => return Err(ChartError("unexpected combo datum path".into())),
    };
    if path.is_empty() {
        return Ok(None);
    }
    let endpoint = |index| -> Result<P> {
        match path.get(index) {
            Some(Cmd::Move(p) | Cmd::Line(p) | Cmd::Cubic { end: p, .. }) => Ok(*p),
            _ => Err(ChartError("missing combo path anchor support".into())),
        }
    };
    let a = endpoint(indices[0])?;
    let b = endpoint(indices[1])?;
    let position = [a.x + (b.x - a.x) / 2., a.y + (b.y - a.y) / 2.];
    InteractionAnchor::path_points(&mark.geometry, position, &indices).map(Some)
}

type SitesAndAnchors = (Vec<ComboSite>, BTreeMap<String, InteractionAnchor>);

pub(super) fn sites(scene: &Scene, source: SiteSource<'_>) -> Result<SitesAndAnchors> {
    let mut by_mark = BTreeMap::new();
    for (si, rows) in source.rows.iter().enumerate() {
        for row in rows.iter().filter(|row| row.y.is_some()) {
            by_mark.insert(intervals::identity(&source.data[si], row), (si, row));
        }
    }
    let mut sites = Vec::new();
    let mut anchors = BTreeMap::new();
    for (mark_index, mark) in scene.marks.iter().enumerate() {
        let Some(&(si, row)) = by_mark.get(mark.id.as_str()) else {
            continue;
        };
        if !mark.has_fill() && mark.stroke.is_none() {
            continue;
        }
        let s = &source.data[si];
        let axis = axis_index(s.axis);
        let raw = &s.data[row.source_index];
        let raw_y = row.y.unwrap();
        let position = if let Some(center) = crate::crosshair::mark_center(mark) {
            center
        } else if let Some(anchor) = path_anchor(mark, s.kind)? {
            let position = anchor.position();
            anchors.insert(mark.id.clone(), anchor);
            position
        } else {
            continue;
        };
        let interval = source.intervals.get(&(si, row.key.clone())).copied();
        let value_position = source.value_position(si, row, interval)?;
        ensure(
            value_position
                .iter()
                .all(|v| v.is_finite() && v.abs() <= 1e6),
            "combo projected datum metadata exceeds coordinate budget",
        )?;
        let axis_options = if axis == 0 {
            &source.options.left
        } else {
            &source.options.right
        };
        let kind = source.y_scales[axis].kind();
        let formatted_value = if let Some([base, end]) =
            interval.filter(|_| source.options.bar_layout == BarLayout::Percent)
        {
            format!(
                "{} (raw {})",
                ValueFormat::Percent(1).format(end - base)?,
                ValueFormat::Auto.format(raw_y)?
            )
        } else {
            formatted(&axis_options.axis.format, kind, raw_y)?
        };
        sites.push(ComboSite {
            mark_id: mark.id.clone(),
            mark_index,
            series_id: s.id.clone(),
            series_index: si,
            source_index: row.source_index,
            point_id: raw.id.clone(),
            kind: s.kind,
            axis: s.axis,
            raw_x: raw.x.clone(),
            raw_y,
            spread: row.spread,
            stack_interval: interval,
            value_position,
            position,
            label: mark.label.clone(),
            formatted_value,
            color: mark
                .fill
                .or(mark.stroke.map(|stroke| stroke.color))
                .unwrap_or(scene.background),
        });
    }
    Ok((sites, anchors))
}
