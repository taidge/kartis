//! AreaBump: rank by value independently at each x, then draw variable-width
//! bands. Reference: Nivo 0.99 bump/src/area-bump/{compute,hooks,types}.ts.
//! Numeric and string x values remain distinct ordinal categories. Positive
//! data uses one shared height scale, never independent column normalization.
use std::collections::{BTreeMap, BTreeSet};

use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as C, StrokeCap};

use crate::scene::ensure;
use crate::*;

mod labels;
use labels::LabelBinding;
pub use labels::{
    AreaBumpLabelCandidate, AreaBumpLabelPlacementOptions, AreaBumpLabelSide,
    AreaBumpMeasuredLabel, AreaBumpPlacedLabel,
};

pub const MAX_AREA_BUMP_CELLS: usize = 16_384;
#[derive(Clone, Debug, PartialEq)]
pub enum AreaBumpX {
    Number(f64),
    Category(String),
}
impl From<f64> for AreaBumpX {
    fn from(v: f64) -> Self {
        Self::Number(v)
    }
}
impl From<i32> for AreaBumpX {
    fn from(v: i32) -> Self {
        Self::Number(f64::from(v))
    }
}
impl From<String> for AreaBumpX {
    fn from(v: String) -> Self {
        Self::Category(v)
    }
}
impl From<&str> for AreaBumpX {
    fn from(v: &str) -> Self {
        Self::Category(v.into())
    }
}
impl AreaBumpX {
    pub fn key(&self) -> String {
        match self {
            Self::Number(v) => format!("number:{:016x}", if *v == 0. { 0 } else { v.to_bits() }),
            Self::Category(v) => format!("category:{}:{v}", v.len()),
        }
    }
    fn validate(&self) -> Result<()> {
        match self {
            Self::Number(v) => ensure(v.is_finite(), "AreaBump numeric x must be finite"),
            Self::Category(v) => valid_id(v),
        }
    }
    fn label(&self, format: &ValueFormat) -> Result<String> {
        match self {
            Self::Number(v) => format.format(*v),
            Self::Category(v) => Ok(v.clone()),
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct AreaBumpDatum {
    pub x: AreaBumpX,
    pub value: f64,
}
impl AreaBumpDatum {
    pub fn new(x: impl Into<AreaBumpX>, value: f64) -> Self {
        Self { x: x.into(), value }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct AreaBumpSeries {
    pub id: String,
    pub label: String,
    pub data: Vec<AreaBumpDatum>,
}
impl AreaBumpSeries {
    pub fn new(id: impl Into<String>, data: Vec<AreaBumpDatum>) -> Self {
        let id = id.into();
        Self {
            label: id.clone(),
            id,
            data,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AreaBumpAlign {
    Start,
    #[default]
    Middle,
    End,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AreaBumpInterpolation {
    Linear,
    #[default]
    Smooth,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AreaBumpMissing {
    /// Nivo's missing-x behavior: join present samples.
    #[default]
    Connect,
    Gap,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AreaBumpTieBreak {
    #[default]
    InputOrder,
    SeriesId,
}
#[derive(Clone, Debug, PartialEq)]
pub struct AreaBumpSeriesStyle {
    pub color: Option<Color>,
    pub paint: Option<FillPaint>,
    pub fill_opacity: f64,
    pub border_color: Option<Color>,
    pub border_width: f64,
    pub border_opacity: f64,
}
impl Default for AreaBumpSeriesStyle {
    fn default() -> Self {
        Self {
            color: None,
            paint: None,
            fill_opacity: 0.8,
            border_color: None,
            border_width: 1.,
            border_opacity: 1.,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct AreaBumpFocusStyle {
    pub fill_opacity: f64,
    pub border_width: f64,
    pub border_opacity: f64,
}
#[derive(Clone, Debug, PartialEq)]
pub enum AreaBumpLabelText {
    Series,
    Endpoint(ValueFormat),
}
#[derive(Clone, Debug, PartialEq)]
pub struct AreaBumpLabels {
    pub enabled: bool,
    pub text: AreaBumpLabelText,
    pub padding: f64,
    pub offset: [f64; 2],
    pub skip_height: f64,
    pub color: Option<Color>,
    pub size: Option<f64>,
    pub rotation: f64,
}
impl Default for AreaBumpLabels {
    fn default() -> Self {
        Self {
            enabled: true,
            text: AreaBumpLabelText::Series,
            padding: 12.,
            offset: [0., 0.],
            skip_height: 0.,
            color: None,
            size: None,
            rotation: 0.,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct AreaBumpAxis {
    /// Explicit tick_values use AreaBumpX::key, disambiguating number 1 / "1".
    pub options: CategoryAxisOptions,
    pub numeric_format: ValueFormat,
    pub tick_size: f64,
    pub text_color: Option<Color>,
    pub text_size: Option<f64>,
}
impl Default for AreaBumpAxis {
    fn default() -> Self {
        Self {
            options: Default::default(),
            numeric_format: ValueFormat::Auto,
            tick_size: 4.,
            text_color: None,
            text_size: None,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct AreaBumpOptions {
    /// Explicit order may additionally reserve completely empty columns.
    pub x_order: Option<Vec<AreaBumpX>>,
    pub align: AreaBumpAlign,
    pub interpolation: AreaBumpInterpolation,
    pub missing: AreaBumpMissing,
    pub tie_break: AreaBumpTieBreak,
    pub spacing: f64,
    pub x_padding: f64,
    pub style: AreaBumpSeriesStyle,
    pub styles: BTreeMap<String, AreaBumpSeriesStyle>,
    pub start_labels: AreaBumpLabels,
    pub end_labels: AreaBumpLabels,
    pub axis_top: Option<AreaBumpAxis>,
    pub axis_bottom: Option<AreaBumpAxis>,
    pub grid: bool,
    /// Hidden rows are omitted before thickness/rank calculation; x identities
    /// and legends retain the full source domain. Re-layout after legend input.
    pub hidden: BTreeSet<String>,
    pub active_ids: BTreeSet<String>,
    pub active_style: AreaBumpFocusStyle,
    pub inactive_style: AreaBumpFocusStyle,
    pub legend: bool,
}
impl Default for AreaBumpOptions {
    fn default() -> Self {
        Self {
            x_order: None,
            align: Default::default(),
            interpolation: Default::default(),
            missing: Default::default(),
            tie_break: Default::default(),
            spacing: 0.,
            x_padding: 0.6,
            style: Default::default(),
            styles: Default::default(),
            start_labels: Default::default(),
            end_labels: Default::default(),
            axis_top: Some(Default::default()),
            axis_bottom: Some(Default::default()),
            grid: true,
            hidden: Default::default(),
            active_ids: Default::default(),
            active_style: AreaBumpFocusStyle {
                fill_opacity: 1.,
                border_width: 1.,
                border_opacity: 1.,
            },
            inactive_style: AreaBumpFocusStyle {
                fill_opacity: 0.15,
                border_width: 0.,
                border_opacity: 0.,
            },
            legend: true,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct AreaBumpCell {
    pub series_id: String,
    pub source_index: usize,
    pub x: AreaBumpX,
    pub x_key: String,
    pub column_index: usize,
    pub rank: usize,
    pub value: f64,
    pub x_pixel: f64,
    /// Logical y coordinates, top <= bottom. Zero values have zero thickness.
    pub y0: f64,
    pub y1: f64,
}
#[derive(Clone, Debug, PartialEq)]
pub struct AreaBumpSlice {
    pub x: AreaBumpX,
    pub key: String,
    pub x_pixel: f64,
    pub total: f64,
    pub offset: f64,
    pub cell_indices: Vec<usize>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct AreaBumpComputedSeries {
    pub id: String,
    pub label: String,
    pub cell_indices: Vec<usize>,
    pub mark_ids: Vec<String>,
    pub hidden: bool,
}
#[derive(Clone, Debug, PartialEq)]
struct PaintBinding {
    slot: usize,
    id: String,
    series: String,
    color: Color,
    style: AreaBumpSeriesStyle,
}
#[derive(Clone, Debug, PartialEq)]
pub struct AreaBumpLayout {
    pub scene: Scene,
    pub cells: Vec<AreaBumpCell>,
    pub slices: Vec<AreaBumpSlice>,
    pub series: Vec<AreaBumpComputedSeries>,
    pub max_sum: f64,
    pub available_height: f64,
    pub active_ids: BTreeSet<String>,
    known: BTreeSet<String>,
    visible: BTreeSet<String>,
    paints: Vec<PaintBinding>,
    endpoint_labels: Vec<AreaBumpLabelCandidate>,
    labels: Vec<LabelBinding>,
    endpoint_placements: Vec<AreaBumpPlacedLabel>,
    active_style: AreaBumpFocusStyle,
    inactive_style: AreaBumpFocusStyle,
}
fn valid_id(s: &str) -> Result<()> {
    ensure(
        !s.trim().is_empty() && s.len() <= 256,
        "invalid AreaBump identity",
    )
}
pub(crate) fn alpha(c: Color, opacity: f64) -> Color {
    Color(c.0, c.1, c.2, (f64::from(c.3) * opacity).round() as u8)
}
fn darker(c: Color, power: f64) -> Color {
    let f = 0.7_f64.powf(power);
    Color(
        (f64::from(c.0) * f).round() as u8,
        (f64::from(c.1) * f).round() as u8,
        (f64::from(c.2) * f).round() as u8,
        c.3,
    )
}
pub(crate) fn paint_alpha(p: &FillPaint, opacity: f64) -> FillPaint {
    let mut p = p.clone();
    match &mut p {
        FillPaint::Pattern(p) => {
            p.foreground = alpha(p.foreground, opacity);
            p.background = alpha(p.background, opacity);
        }
        FillPaint::Linear { stops, .. } | FillPaint::Radial { stops, .. } => {
            for s in stops {
                s.color = alpha(s.color, opacity);
            }
        }
    }
    p
}
fn visual_valid(fill: f64, width: f64, border: f64) -> Result<()> {
    ensure(
        fill.is_finite()
            && (0. ..=1.).contains(&fill)
            && border.is_finite()
            && (0. ..=1.).contains(&border)
            && width.is_finite()
            && (0. ..=10000.).contains(&width),
        "invalid AreaBump opacity or border width",
    )
}
fn labels_valid(l: &AreaBumpLabels) -> Result<()> {
    ensure(
        l.padding.is_finite()
            && (0. ..=16384.).contains(&l.padding)
            && l.offset.iter().all(|v| v.is_finite() && v.abs() <= 16384.)
            && l.skip_height.is_finite()
            && (0. ..=16384.).contains(&l.skip_height)
            && l.size
                .is_none_or(|v| v.is_finite() && (1. ..=256.).contains(&v))
            && l.rotation.is_finite()
            && (-360. ..=360.).contains(&l.rotation),
        "invalid AreaBump label style",
    )?;
    if let AreaBumpLabelText::Endpoint(f) = &l.text {
        f.validate()?;
    }
    Ok(())
}
fn style_valid(s: &AreaBumpSeriesStyle) -> Result<()> {
    visual_valid(s.fill_opacity, s.border_width, s.border_opacity)?;
    if let Some(p) = &s.paint {
        p.validate()?;
    }
    Ok(())
}
impl AreaBumpLayout {
    pub fn nearest_slice(&self, x: f64) -> Option<&AreaBumpSlice> {
        if !x.is_finite() {
            return None;
        }
        self.slices
            .iter()
            .min_by(|a, b| (a.x_pixel - x).abs().total_cmp(&(b.x_pixel - x).abs()))
    }
    /// Controlled paint only; geometry/ranks/domains remain the same. Other
    /// appended Scene marks and labels are preserved. Unknown identities reject.
    pub fn set_active(&mut self, ids: BTreeSet<String>) -> Result<()> {
        ensure(
            ids.len() <= 256 && ids.iter().all(|id| self.known.contains(id)),
            "unknown active AreaBump series",
        )?;
        let any = ids.iter().any(|id| self.visible.contains(id));
        let mut next = self.scene.clone();
        for b in &self.paints {
            let mark = next
                .marks
                .get_mut(b.slot)
                .ok_or_else(|| ChartError("AreaBump Scene slots were changed".into()))?;
            ensure(
                mark.id == b.id && mark.series == b.series,
                "AreaBump Scene identity was changed",
            )?;
            let focus = if !any {
                None
            } else if ids.contains(&b.series) {
                Some(&self.active_style)
            } else {
                Some(&self.inactive_style)
            };
            let (fill, width, border) = focus.map_or(
                (
                    b.style.fill_opacity,
                    b.style.border_width,
                    b.style.border_opacity,
                ),
                |f| (f.fill_opacity, f.border_width, f.border_opacity),
            );
            mark.fill = Some(alpha(b.color, fill));
            mark.fill_paint = b.style.paint.as_ref().map(|p| paint_alpha(p, fill));
            mark.opacity = 1.;
            mark.stroke = if width > 0. {
                Some(GraphicStroke {
                    color: alpha(
                        b.style.border_color.unwrap_or_else(|| darker(b.color, 0.4)),
                        border,
                    ),
                    width,
                    cap: StrokeCap::Butt,
                })
            } else {
                None
            };
        }
        self.update_label_paint(&mut next, &ids, any)?;
        next.validate()?;
        self.scene = next;
        self.active_ids = ids;
        Ok(())
    }
    pub fn with_active(&self, ids: BTreeSet<String>) -> Result<Self> {
        let mut next = self.clone();
        next.set_active(ids)?;
        Ok(next)
    }
}
fn axis(
    scene: &mut Scene,
    c: &ChartConfig,
    slices: &[AreaBumpSlice],
    o: &AreaBumpAxis,
    top: bool,
) -> Result<()> {
    o.options.validate()?;
    o.numeric_format.validate()?;
    ensure(
        o.tick_size.is_finite()
            && (0. ..=256.).contains(&o.tick_size)
            && o.text_size
                .is_none_or(|s| s.is_finite() && (1. ..=256.).contains(&s)),
        "invalid AreaBump axis style",
    )?;
    if let Some(keys) = &o.options.tick_values {
        ensure(
            keys.iter().all(|k| slices.iter().any(|s| &s.key == k))
                && keys.iter().enumerate().all(|(i, k)| !keys[..i].contains(k)),
            "unknown or duplicate AreaBump x tick",
        )?;
    }
    if !o.options.visible {
        return Ok(());
    }
    let color = o.text_color.unwrap_or(c.theme.text);
    let size = o.text_size.unwrap_or(c.theme.font_size);
    let y = if top {
        c.margin.top
    } else {
        c.margin.top + c.inner_height()
    };
    let direction = if top { -1. } else { 1. };
    let name = if top { "top" } else { "bottom" };
    let mut line = Mark::new(
        format!("area-bump-axis:{name}"),
        Geometry::Path(vec![
            C::Move(P::new(c.margin.left, y)),
            C::Line(P::new(c.margin.left + c.inner_width(), y)),
        ]),
        color,
    );
    line.label.clear();
    line.fill = None;
    line.stroke = Some(GraphicStroke {
        color,
        width: 1.,
        cap: StrokeCap::Butt,
    });
    scene.marks.push(line);
    let stride = slices.len().div_ceil(o.options.tick_count.max(1)).max(1);
    for (i, slice) in slices.iter().enumerate() {
        let selected = o
            .options
            .tick_values
            .as_ref()
            .map_or(o.options.tick_count > 0 && i % stride == 0, |v| {
                v.contains(&slice.key)
            });
        if !selected {
            continue;
        }
        let mut tick = Mark::new(
            format!("area-bump-tick:{name}:{i}"),
            Geometry::Path(vec![
                C::Move(P::new(slice.x_pixel, y)),
                C::Line(P::new(slice.x_pixel, y + direction * o.tick_size)),
            ]),
            color,
        );
        tick.label.clear();
        tick.fill = None;
        tick.stroke = Some(GraphicStroke {
            color,
            width: 1.,
            cap: StrokeCap::Butt,
        });
        scene.marks.push(tick);
        scene.labels.push(Label {
            x: slice.x_pixel,
            y: y + direction * (o.tick_size + o.options.tick_padding) + if top { 0. } else { size },
            text: slice.x.label(&o.numeric_format)?,
            color,
            size,
            anchor: TextAnchor::Middle,
            rotation: o.options.tick_rotation,
        });
    }
    if let Some(text) = &o.options.legend {
        scene.labels.push(Label {
            x: c.margin.left + c.inner_width() / 2.,
            y: y + direction * (o.tick_size + o.options.tick_padding + size * 2.5),
            text: text.clone(),
            color,
            size,
            anchor: TextAnchor::Middle,
            rotation: 0.,
        });
    }
    Ok(())
}
pub fn area_bump(c: &ChartConfig, data: &[AreaBumpSeries], o: &AreaBumpOptions) -> Result<Scene> {
    Ok(layout_area_bump(c, data, o)?.scene)
}
pub fn layout_area_bump(
    c: &ChartConfig,
    data: &[AreaBumpSeries],
    o: &AreaBumpOptions,
) -> Result<AreaBumpLayout> {
    c.validate()?;
    ensure(data.len() <= 256, "AreaBump series budget exceeded")?;
    ensure(o.styles.len() <= 256, "AreaBump style budget exceeded")?;
    if let Some(order) = &o.x_order {
        ensure(order.len() <= 4096, "AreaBump x domain budget exceeded")?;
        // Validate nested strings before cloning external configuration.
        for x in order {
            x.validate()?;
        }
    }
    let count = data.iter().try_fold(0_usize, |sum, s| {
        sum.checked_add(s.data.len())
            .ok_or_else(|| ChartError("AreaBump data budget exceeded".into()))
    })?;
    ensure(
        count <= MAX_AREA_BUMP_CELLS,
        "AreaBump data budget exceeded",
    )?;
    ensure(
        o.spacing.is_finite()
            && (0. ..=16384.).contains(&o.spacing)
            && o.x_padding.is_finite()
            && (0. ..=1.).contains(&o.x_padding),
        "invalid AreaBump spacing/padding",
    )?;
    style_valid(&o.style)?;
    for s in o.styles.values() {
        style_valid(s)?;
    }
    for f in [&o.active_style, &o.inactive_style] {
        visual_valid(f.fill_opacity, f.border_width, f.border_opacity)?;
    }
    labels_valid(&o.start_labels)?;
    labels_valid(&o.end_labels)?;
    let mut known = BTreeSet::new();
    let mut inferred = Vec::new();
    let mut seen_x = BTreeSet::new();
    let mut inputs = Vec::with_capacity(data.len());
    for s in data {
        valid_id(&s.id)?;
        ensure(known.insert(s.id.clone()), "duplicate AreaBump series ID")?;
        ensure(
            s.label.len() <= 1024,
            "AreaBump series label budget exceeded",
        )?;
        let mut samples = BTreeMap::new();
        for (i, p) in s.data.iter().enumerate() {
            p.x.validate()?;
            ensure(
                p.value.is_finite() && p.value >= 0.,
                "AreaBump requires finite nonnegative values",
            )?;
            let key = p.x.key();
            ensure(
                samples.insert(key.clone(), (i, p)).is_none(),
                "duplicate AreaBump x in series",
            )?;
            if seen_x.insert(key) {
                inferred.push(p.x.clone());
            }
        }
        inputs.push(samples);
    }
    ensure(
        o.hidden.len() <= 256
            && o.active_ids.len() <= 256
            && o.styles
                .keys()
                .chain(o.hidden.iter())
                .chain(o.active_ids.iter())
                .all(|id| known.contains(id)),
        "AreaBump options refer to unknown series",
    )?;
    let domain = o.x_order.clone().unwrap_or(inferred);
    ensure(domain.len() <= 4096, "AreaBump x domain budget exceeded")?;
    let mut domain_index = BTreeMap::new();
    for (i, x) in domain.iter().enumerate() {
        x.validate()?;
        ensure(
            domain_index.insert(x.key(), i).is_none(),
            "duplicate AreaBump domain x",
        )?;
    }
    ensure(
        seen_x.iter().all(|key| domain_index.contains_key(key)),
        "AreaBump x order omits a source category",
    )?;
    let mut columns = vec![Vec::new(); domain.len()];
    for (si, s) in data.iter().enumerate() {
        if o.hidden.contains(&s.id) {
            continue;
        }
        for (key, (source, p)) in &inputs[si] {
            columns[domain_index[key]].push((si, *source, p.value));
        }
    }
    for values in &mut columns {
        values.sort_by(|a, b| {
            b.2.total_cmp(&a.2).then_with(|| match o.tie_break {
                AreaBumpTieBreak::InputOrder => a.0.cmp(&b.0),
                AreaBumpTieBreak::SeriesId => data[a.0].id.cmp(&data[b.0].id),
            })
        });
    }
    let mut totals = Vec::with_capacity(columns.len());
    for values in &columns {
        let mut total = 0.;
        for (_, _, v) in values {
            total += v;
            ensure(total.is_finite(), "AreaBump column total overflow")?;
        }
        totals.push(total);
    }
    let max_sum = totals.iter().copied().fold(0., f64::max);
    let max_count = columns.iter().map(Vec::len).max().unwrap_or(0);
    let available = c.inner_height() - max_count as f64 * o.spacing;
    ensure(available >= 0., "AreaBump spacing leaves no band area")?;
    let step = if domain.len() > 1 {
        c.inner_width() / (domain.len() - 1) as f64
    } else {
        c.inner_width()
    };
    let mut slices = Vec::with_capacity(domain.len());
    let mut cells = Vec::with_capacity(count);
    let mut series = data
        .iter()
        .map(|s| AreaBumpComputedSeries {
            id: s.id.clone(),
            label: s.label.clone(),
            cell_indices: vec![],
            mark_ids: vec![],
            hidden: o.hidden.contains(&s.id),
        })
        .collect::<Vec<_>>();
    for (ci, values) in columns.iter().enumerate() {
        let x = c.margin.left
            + if domain.len() == 1 {
                c.inner_width() / 2.
            } else {
                ci as f64 * step
            };
        let height = if max_sum > 0. {
            totals[ci] / max_sum * available
        } else {
            0.
        } + values.len() as f64 * o.spacing;
        let offset = match o.align {
            AreaBumpAlign::Start => 0.,
            AreaBumpAlign::Middle => (c.inner_height() - height) / 2.,
            AreaBumpAlign::End => c.inner_height() - height,
        };
        let mut slice = AreaBumpSlice {
            x: domain[ci].clone(),
            key: domain[ci].key(),
            x_pixel: x,
            total: totals[ci],
            offset,
            cell_indices: vec![],
        };
        let mut prefix = 0.;
        for (rank, (si, source, value)) in values.iter().enumerate() {
            let y0 = c.margin.top
                + offset
                + if max_sum > 0. {
                    prefix / max_sum * available
                } else {
                    0.
                }
                + o.spacing * (rank as f64 + 0.5);
            let thickness = if max_sum > 0. {
                value / max_sum * available
            } else {
                0.
            };
            let y1 = y0 + thickness;
            ensure(
                *value == 0. || thickness > 0. && y1 > y0,
                "positive AreaBump thickness is not representable",
            )?;
            prefix += value;
            let index = cells.len();
            series[*si].cell_indices.push(index);
            slice.cell_indices.push(index);
            cells.push(AreaBumpCell {
                series_id: data[*si].id.clone(),
                source_index: *source,
                x: domain[ci].clone(),
                x_key: slice.key.clone(),
                column_index: ci,
                rank,
                value: *value,
                x_pixel: x,
                y0,
                y1,
            });
        }
        slices.push(slice);
    }
    let palette = known
        .iter()
        .enumerate()
        .map(|(i, id)| (id.clone(), c.theme.color(i)))
        .collect::<BTreeMap<_, _>>();
    let mut scene = c.scene()?;
    if o.grid
        || o.axis_top.as_ref().is_some_and(|a| a.options.grid)
        || o.axis_bottom.as_ref().is_some_and(|a| a.options.grid)
    {
        for (i, s) in slices.iter().enumerate() {
            let mut m = Mark::new(
                format!("area-bump-grid:{i}"),
                Geometry::Path(vec![
                    C::Move(P::new(s.x_pixel, c.margin.top)),
                    C::Line(P::new(s.x_pixel, c.margin.top + c.inner_height())),
                ]),
                c.theme.grid,
            );
            m.fill = None;
            m.label.clear();
            m.stroke = Some(GraphicStroke {
                color: c.theme.grid,
                width: 1.,
                cap: StrokeCap::Butt,
            });
            scene.marks.push(m);
        }
    }
    if let Some(a) = &o.axis_top {
        axis(&mut scene, c, &slices, a, true)?;
    }
    if let Some(a) = &o.axis_bottom {
        axis(&mut scene, c, &slices, a, false)?;
    }
    let mut paints = Vec::new();
    let mut labels = Vec::new();
    let mut endpoint_labels = Vec::new();
    let mut visible = BTreeSet::new();
    for (si, s) in data.iter().enumerate() {
        let style = o.styles.get(&s.id).unwrap_or(&o.style);
        let color = style.color.unwrap_or(palette[&s.id]);
        if o.legend {
            scene.legend.push(LegendItem {
                id: s.id.clone(),
                label: s.label.clone(),
                color,
            });
        }
        let computed = &mut series[si];
        let points = &computed.cell_indices;
        if points.is_empty() {
            continue;
        }
        visible.insert(s.id.clone());
        let mut runs: Vec<Vec<usize>> = vec![];
        for index in points {
            if o.missing == AreaBumpMissing::Gap
                && runs.last().and_then(|r| r.last()).is_some_and(|previous| {
                    cells[*index].column_index > cells[*previous].column_index + 1
                })
            {
                runs.push(vec![]);
            }
            if runs.is_empty() {
                runs.push(vec![]);
            }
            runs.last_mut().unwrap().push(*index);
        }
        for run in runs {
            if run.len() < 2 || !run.iter().any(|i| cells[*i].value > 0.) {
                continue;
            }
            let mut top = Vec::with_capacity(run.len() * 3);
            let mut bottom = Vec::with_capacity(run.len() * 3);
            for (i, index) in run.iter().enumerate() {
                let p = &cells[*index];
                let pad = step * o.x_padding / 2.;
                for x in [
                    if i > 0 { Some(p.x_pixel - pad) } else { None },
                    Some(p.x_pixel),
                    if i + 1 < run.len() {
                        Some(p.x_pixel + pad)
                    } else {
                        None
                    },
                ]
                .into_iter()
                .flatten()
                {
                    top.push(P::new(x, p.y0));
                    bottom.push(P::new(x, p.y1));
                }
            }
            let curve = if o.interpolation == AreaBumpInterpolation::Smooth {
                Curve::Basis
            } else {
                Curve::Linear
            };
            let mut commands = crate::cartesian::path(&bottom, curve);
            top.reverse();
            let mut upper = crate::cartesian::path(&top, curve);
            if let Some(C::Move(p)) = upper.first() {
                commands.push(C::Line(*p));
            }
            commands.extend(upper.drain(1..));
            commands.push(C::Close);
            let first = &cells[run[0]];
            let last = &cells[*run.last().unwrap()];
            let id = if o.missing == AreaBumpMissing::Connect {
                format!("area-bump:{}:{}", s.id.len(), s.id)
            } else {
                format!(
                    "area-bump:{}:{}:gap:{}:{}",
                    s.id.len(),
                    s.id,
                    first.x_key.len(),
                    first.x_key
                )
            };
            let mut mark = Mark::new(&id, Geometry::Path(commands), color);
            mark.series = s.id.clone();
            mark.label = s.label.clone();
            // Whole-band events carry the last present value of this run;
            // full slice values and source indices remain in layout metadata.
            mark.value = last.value;
            mark.clip = Some(ClipRect::plot(c));
            paints.push(PaintBinding {
                slot: scene.marks.len(),
                id: id.clone(),
                series: s.id.clone(),
                color,
                style: style.clone(),
            });
            scene.marks.push(mark);
            computed.mark_ids.push(id);
        }
        for (start, options) in [(true, &o.start_labels), (false, &o.end_labels)] {
            let p = &cells[if start {
                points[0]
            } else {
                *points.last().unwrap()
            }];
            if !options.enabled || p.y1 - p.y0 < options.skip_height {
                continue;
            }
            let text = match &options.text {
                AreaBumpLabelText::Series => s.label.clone(),
                AreaBumpLabelText::Endpoint(f) => f.format(p.value)?,
            };
            let label_color = options.color.unwrap_or_else(|| darker(color, 1.));
            let slot = scene.labels.len();
            scene.label_series.insert(slot, s.id.clone());
            let label = Label {
                x: p.x_pixel
                    + if start {
                        -options.padding
                    } else {
                        options.padding
                    }
                    + options.offset[0],
                y: (p.y0 + p.y1) / 2. + options.offset[1],
                text,
                color: label_color,
                size: options.size.unwrap_or(c.theme.font_size),
                anchor: if start {
                    TextAnchor::End
                } else {
                    TextAnchor::Start
                },
                rotation: options.rotation,
            };
            labels.push(LabelBinding::new(slot, &label, style.fill_opacity));
            endpoint_labels.push(AreaBumpLabelCandidate::new(
                &s.id,
                if start {
                    AreaBumpLabelSide::Start
                } else {
                    AreaBumpLabelSide::End
                },
                [p.x_pixel, (p.y0 + p.y1) / 2.],
                label.clone(),
                scene.resolved_label_font(slot),
            ));
            scene.labels.push(label);
        }
    }
    let mut layout = AreaBumpLayout {
        scene,
        cells,
        slices,
        series,
        max_sum,
        available_height: available,
        active_ids: BTreeSet::new(),
        known,
        visible,
        paints,
        endpoint_labels,
        labels,
        endpoint_placements: Vec::new(),
        active_style: o.active_style.clone(),
        inactive_style: o.inactive_style.clone(),
    };
    layout.set_active(o.active_ids.clone())?;
    Ok(layout)
}
