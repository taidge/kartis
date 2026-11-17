//! Renderer-independent radial and hierarchy layouts.
//!
//! The decomposition follows Nivo's pie/radar/waffle/funnel and hierarchy hooks
//! (E:/Repos/nivo/packages/*/src/hooks.ts), with native Rust geometry instead of
//! React hooks and d3-shape generators. D3's local src/index.js identifies the
//! shape/hierarchy modules; its aggregate checkout does not include their code.
//! All angles are radians, clockwise from twelve o'clock.
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::f64::consts::{FRAC_PI_2, TAU};
use std::sync::Arc;

use glory::render::{GraphicPoint as Point, GraphicStroke, PathCommand as Cmd, StrokeCap};
use glory::style::Color;

use crate::scene::*;

#[derive(Clone, Debug, PartialEq)]
pub struct RadialDatum {
    pub id: String,
    pub label: String,
    pub value: f64,
}
impl RadialDatum {
    pub fn new(id: impl Into<String>, value: f64) -> Self {
        let id = id.into();
        Self {
            label: id.clone(),
            id,
            value,
        }
    }
}
fn valid_key(id: &str) -> bool {
    !id.trim().is_empty() && id.len() <= MAX_LABEL_BYTES
}
fn positive_max(values: impl Iterator<Item = f64>) -> f64 {
    let maximum = values.fold(0., f64::max);
    if maximum > 0. { maximum } else { 1. }
}
pub(crate) fn validate_data(data: &[RadialDatum]) -> Result<()> {
    ensure(data.len() <= 256, "radial data exceeds 256 legend entries")?;
    let mut ids = BTreeSet::new();
    for d in data {
        ensure(
            valid_key(&d.id) && d.id.len() <= 1000 && d.label.len() <= MAX_LABEL_BYTES,
            "invalid radial id/label",
        )?;
        ensure(ids.insert(&d.id), "duplicate radial id")?;
        ensure(
            d.value.is_finite() && d.value >= 0.,
            "radial values must be finite and nonnegative",
        )?;
    }
    Ok(())
}
fn center(c: &ChartConfig) -> (f64, f64, f64) {
    (
        c.margin.left + c.inner_width() / 2.,
        c.margin.top + c.inner_height() / 2.,
        c.inner_width().min(c.inner_height()) / 2.,
    )
}
fn point(cx: f64, cy: f64, r: f64, a: f64) -> Point {
    Point::new(cx + r * a.sin(), cy - r * a.cos())
}
fn arc_segment(out: &mut Vec<Cmd>, cx: f64, cy: f64, r: f64, start: f64, end: f64) {
    let n = ((end - start).abs() / FRAC_PI_2).ceil().max(1.) as usize;
    let delta = (end - start) / n as f64;
    for i in 0..n {
        let a = start + delta * i as f64;
        let b = a + delta;
        let k = 4. / 3. * (delta / 4.).tan();
        let p = point(cx, cy, r, a);
        let q = point(cx, cy, r, b);
        out.push(Cmd::Cubic {
            first: Point::new(p.x + k * r * a.cos(), p.y + k * r * a.sin()),
            second: Point::new(q.x - k * r * b.cos(), q.y - k * r * b.sin()),
            end: q,
        });
    }
}
/// One nonzero-winding annular sector, including complete circles and holes.
fn sector(cx: f64, cy: f64, inner: f64, outer: f64, start: f64, end: f64) -> Geometry {
    let mut path = vec![Cmd::Move(point(cx, cy, outer, start))];
    arc_segment(&mut path, cx, cy, outer, start, end);
    if inner > 0. {
        path.push(Cmd::Line(point(cx, cy, inner, end)));
        arc_segment(&mut path, cx, cy, inner, end, start);
    } else {
        path.push(Cmd::Line(Point::new(cx, cy)));
    }
    path.push(Cmd::Close);
    Geometry::Path(path)
}
fn polygon(points: impl IntoIterator<Item = Point>) -> Geometry {
    let mut iter = points.into_iter();
    let mut path = vec![];
    if let Some(p) = iter.next() {
        path.push(Cmd::Move(p));
        path.extend(iter.map(Cmd::Line));
        path.push(Cmd::Close);
    }
    Geometry::Path(path)
}
fn label(scene: &mut Scene, c: &ChartConfig, x: f64, y: f64, text: &str) {
    scene.labels.push(Label {
        rotation: 0.,
        x,
        y,
        text: text.into(),
        color: c.theme.text,
        size: c.theme.font_size,
        anchor: TextAnchor::Middle,
    });
}
fn owned_label(scene: &mut Scene, c: &ChartConfig, x: f64, y: f64, text: &str, series: &str) {
    let index = scene.labels.len();
    label(scene, c, x, y, text);
    scene.label_series.insert(index, series.into());
}
fn legend(scene: &mut Scene, c: &ChartConfig, data: &[RadialDatum]) {
    scene.legend = data
        .iter()
        .enumerate()
        .map(|(i, d)| LegendItem {
            id: d.id.clone(),
            label: d.label.clone(),
            color: c.theme.color(i),
        })
        .collect();
}
fn datum_mark(d: &RadialDatum, geometry: Geometry, c: &ChartConfig, i: usize) -> Mark {
    let mut m = Mark::new(d.id.clone(), geometry, c.theme.color(i));
    m.label = d.label.clone();
    m.value = d.value;
    m.series = d.id.clone();
    m
}

#[path = "pie_styles.rs"]
mod pie_styles;
pub(crate) use pie_styles::rounded_sector as legacy_pie_sector;
pub use pie_styles::*;

#[derive(Clone, Debug, PartialEq)]
pub struct PieOptions {
    pub inner_radius: f64,
    pub start_angle: f64,
    pub end_angle: f64,
    pub pad_angle: f64,
    pub sort_by_value: bool,
    pub labels: bool,
    pub hidden: BTreeSet<String>,
    pub style: PieStyleOptions,
}
impl Default for PieOptions {
    fn default() -> Self {
        Self {
            inner_radius: 0.,
            start_angle: 0.,
            end_angle: TAU,
            pad_angle: 0.,
            sort_by_value: false,
            labels: true,
            hidden: BTreeSet::new(),
            style: PieStyleOptions::default(),
        }
    }
}
/// Pie/donut sectors. Zero-valued slices remain in the legend but emit no mark.
pub fn pie(c: &ChartConfig, data: &[RadialDatum], o: &PieOptions) -> Result<Scene> {
    let mut s = c.scene()?;
    validate_data(data)?;
    o.style.validate(data)?;
    ensure(
        o.inner_radius.is_finite() && (0. ..1.).contains(&o.inner_radius),
        "inner radius ratio must be in [0,1)",
    )?;
    let span = o.end_angle - o.start_angle;
    ensure(
        o.start_angle.is_finite()
            && o.end_angle.is_finite()
            && span.abs() > 0.
            && span.abs() <= TAU + 1e-12
            && o.start_angle.abs() <= 1e6
            && o.end_angle.abs() <= 1e6,
        "pie absolute angular extent must be in (0,2pi]",
    )?;
    ensure(
        o.pad_angle.is_finite() && o.pad_angle >= 0. && o.pad_angle <= TAU,
        "invalid pie pad angle",
    )?;
    ensure(
        o.hidden.len() <= MAX_MARKS && o.hidden.iter().all(|id| valid_key(id)),
        "invalid hidden IDs",
    )?;
    legend(&mut s, c, data);
    let (cx, cy, r) = center(c);
    let mut order: Vec<usize> = (0..data.len())
        .filter(|i| !o.hidden.contains(&data[*i].id) && data[*i].value > 0.)
        .collect();
    if o.sort_by_value {
        order.sort_by(|a, b| data[*b].value.total_cmp(&data[*a].value));
    }
    let max = order.iter().map(|i| data[*i].value).fold(0., f64::max);
    if max == 0. {
        return finish(s);
    }
    let total: f64 = order.iter().map(|i| data[*i].value / max).sum();
    let direction = span.signum();
    let pad = o.pad_angle.min(span.abs() / order.len() as f64);
    let available = (span.abs() - pad * order.len() as f64).max(0.);
    let leader_ids = pie_styles::leader_ids(data);
    let mut angle = o.start_angle;
    for i in order {
        let d = &data[i];
        let extent = available * (d.value / max / total);
        let start = angle + direction * pad / 2.;
        let end = start + direction * extent;
        angle = end + direction * pad / 2.;
        if extent <= 0. {
            continue;
        }
        let active = o.style.active_ids.contains(&d.id);
        let inner = (r * o.inner_radius
            - if active {
                o.style.active_inner_radius_offset
            } else {
                0.
            })
        .max(0.);
        let outer = r + if active {
            o.style.active_outer_radius_offset
        } else {
            0.
        };
        ensure(
            outer > 0. && inner < outer,
            "active pie radii must form a positive annulus",
        )?;
        let mut mark = datum_mark(
            d,
            pie_styles::rounded_sector(cx, cy, inner, outer, start, end, o.style.corner_radius),
            c,
            i,
        );
        mark.stroke = o.style.border;
        if let Some(Color(red, green, blue, alpha)) = mark.fill {
            mark.fill = Some(Color(
                red,
                green,
                blue,
                (f64::from(alpha) * o.style.fill_opacity).round() as u8,
            ));
        }
        s.marks.push(mark);
        pie_styles::append_labels(
            &mut s,
            c,
            d,
            &o.style,
            pie_styles::PieLabelGeometry {
                center: [cx, cy],
                inner,
                outer,
                middle: (start + end) / 2.,
                extent,
                fraction: d.value / max / total,
                arc_enabled: o.labels,
                leader_id: &leader_ids[&d.id],
                slice_color: c.theme.color(i),
            },
        )?;
    }
    finish(s)
}

#[derive(Clone, Debug, PartialEq)]
pub struct RadialBarOptions {
    pub inner_radius: f64,
    pub max_value: Option<f64>,
    pub start_angle: f64,
    pub end_angle: f64,
    pub padding: f64,
    pub tracks: bool,
}
impl Default for RadialBarOptions {
    fn default() -> Self {
        Self {
            inner_radius: 0.15,
            max_value: None,
            start_angle: 0.,
            end_angle: TAU,
            padding: 2.,
            tracks: true,
        }
    }
}
pub fn radial_bar(c: &ChartConfig, data: &[RadialDatum], o: &RadialBarOptions) -> Result<Scene> {
    let mut s = c.scene()?;
    validate_data(data)?;
    ensure(
        o.inner_radius.is_finite()
            && (0. ..1.).contains(&o.inner_radius)
            && o.padding.is_finite()
            && o.padding >= 0.,
        "invalid radial bar geometry",
    )?;
    let span = o.end_angle - o.start_angle;
    ensure(
        o.start_angle.is_finite()
            && o.end_angle.is_finite()
            && o.start_angle.abs() <= 1e6
            && span > 0.
            && span <= TAU + 1e-12,
        "invalid radial bar angular extent",
    )?;
    let max = o
        .max_value
        .unwrap_or_else(|| positive_max(data.iter().map(|d| d.value)));
    ensure(
        max.is_finite() && max > 0. && data.iter().all(|d| d.value <= max),
        "radial bar maximum must include all values",
    )?;
    legend(&mut s, c, data);
    if data.is_empty() {
        return finish(s);
    }
    let (cx, cy, r) = center(c);
    let band = r * (1. - o.inner_radius) / data.len() as f64;
    ensure(o.padding < band, "radial bar padding exceeds band width")?;
    for (i, d) in data.iter().enumerate() {
        let outer = r - i as f64 * band;
        let inner = outer - band + o.padding;
        if o.tracks {
            let mut track = Mark::new(
                format!("track:{}", d.id),
                sector(cx, cy, inner, outer, o.start_angle, o.end_angle),
                c.theme.grid,
            );
            track.label = String::new();
            s.marks.push(track);
        }
        if d.value > 0. {
            let mut m = datum_mark(
                d,
                sector(
                    cx,
                    cy,
                    inner,
                    outer,
                    o.start_angle,
                    o.start_angle + span * (d.value / max),
                ),
                c,
                i,
            );
            m.id = format!("bar:{}", d.id);
            s.marks.push(m);
        }
    }
    ensure(s.marks.len() <= MAX_MARKS, "radial bar exceeds mark budget")?;
    finish(s)
}

#[derive(Clone, Debug, PartialEq)]
pub struct FunnelOptions {
    pub horizontal: bool,
    pub spacing: f64,
    pub labels: bool,
}
impl Default for FunnelOptions {
    fn default() -> Self {
        Self {
            horizontal: false,
            spacing: 2.,
            labels: true,
        }
    }
}
/// Each stage's two widths interpolate to the next stage; the final stage tapers
/// to the same final value, making a flat tail rather than inventing a zero.
pub fn funnel(c: &ChartConfig, data: &[RadialDatum], o: &FunnelOptions) -> Result<Scene> {
    let mut s = c.scene()?;
    validate_data(data)?;
    ensure(
        o.spacing.is_finite() && o.spacing >= 0.,
        "invalid funnel spacing",
    )?;
    legend(&mut s, c, data);
    if data.is_empty() {
        return finish(s);
    }
    let length = if o.horizontal {
        c.inner_width()
    } else {
        c.inner_height()
    };
    let cross = if o.horizontal {
        c.inner_height()
    } else {
        c.inner_width()
    };
    let band = (length - o.spacing * data.len().saturating_sub(1) as f64) / data.len() as f64;
    ensure(band > 0., "funnel spacing consumes plot")?;
    let max = data.iter().map(|d| d.value).fold(0., f64::max);
    if max == 0. {
        return finish(s);
    }
    for (i, d) in data.iter().enumerate() {
        let a = d.value / max * cross;
        let b = data.get(i + 1).unwrap_or(d).value / max * cross;
        let low = i as f64 * (band + o.spacing);
        let transform = |u: f64, v: f64| {
            if o.horizontal {
                Point::new(c.margin.left + u, c.margin.top + v)
            } else {
                Point::new(c.margin.left + v, c.margin.top + u)
            }
        };
        s.marks.push(datum_mark(
            d,
            polygon([
                transform(low, (cross - a) / 2.),
                transform(low, (cross + a) / 2.),
                transform(low + band, (cross + b) / 2.),
                transform(low + band, (cross - b) / 2.),
            ]),
            c,
            i,
        ));
        if o.labels {
            let p = transform(low + band / 2., cross / 2.);
            owned_label(&mut s, c, p.x, p.y, &d.label, &d.id);
        }
    }
    finish(s)
}

#[derive(Clone, Debug, PartialEq)]
pub struct WaffleOptions {
    pub rows: usize,
    pub columns: usize,
    pub total: Option<f64>,
    pub padding: f64,
    pub round_cells: bool,
}
impl Default for WaffleOptions {
    fn default() -> Self {
        Self {
            rows: 10,
            columns: 10,
            total: None,
            padding: 2.,
            round_cells: false,
        }
    }
}
/// Largest-remainder allocation preserves the exact rounded occupied-cell total.
/// Values smaller than the configured total leave empty cells.
pub fn waffle(c: &ChartConfig, data: &[RadialDatum], o: &WaffleOptions) -> Result<Scene> {
    let mut s = c.scene()?;
    validate_data(data)?;
    ensure(
        o.rows > 0
            && o.columns > 0
            && o.rows
                .checked_mul(o.columns)
                .is_some_and(|n| n <= MAX_MARKS),
        "invalid waffle grid budget",
    )?;
    ensure(
        o.padding.is_finite() && o.padding >= 0.,
        "invalid waffle padding",
    )?;
    let sum: f64 = data.iter().map(|d| d.value).sum();
    ensure(sum.is_finite(), "waffle value sum overflow")?;
    let total = o.total.unwrap_or(if sum > 0. { sum } else { 1. });
    ensure(
        total.is_finite() && total > 0. && sum <= total + total * 1e-12,
        "waffle total must be positive and include values",
    )?;
    let n = o.rows * o.columns;
    let mut allocations: Vec<usize> = data
        .iter()
        .map(|d| (d.value / total * n as f64).floor() as usize)
        .collect();
    let occupied = (sum / total * n as f64).round().min(n as f64) as usize;
    let mut order: Vec<usize> = (0..data.len()).collect();
    order.sort_by(|a, b| {
        let frac = |i: usize| (data[i].value / total * n as f64).fract();
        frac(*b).total_cmp(&frac(*a)).then(a.cmp(b))
    });
    let allocated: usize = allocations.iter().sum();
    for i in order.into_iter().take(occupied.saturating_sub(allocated)) {
        allocations[i] += 1;
    }
    let step = (c.inner_width() / o.columns as f64).min(c.inner_height() / o.rows as f64);
    ensure(o.padding < step, "waffle padding exceeds cell size")?;
    let size = step - o.padding;
    let x0 = c.margin.left + (c.inner_width() - step * o.columns as f64) / 2.;
    let y0 = c.margin.top + (c.inner_height() - step * o.rows as f64) / 2.;
    let mut owners = Vec::with_capacity(n);
    for (i, count) in allocations.iter().enumerate() {
        owners.extend(std::iter::repeat_n(Some(i), *count));
    }
    owners.resize(n, None);
    for (k, owner) in owners.into_iter().enumerate() {
        let x = x0 + (k % o.columns) as f64 * step + o.padding / 2.;
        let y = y0 + (o.rows - 1 - k / o.columns) as f64 * step + o.padding / 2.;
        let geometry = if o.round_cells {
            Geometry::Circle {
                x: x + size / 2.,
                y: y + size / 2.,
                radius: size / 2.,
            }
        } else {
            Geometry::Rect {
                x,
                y,
                width: size,
                height: size,
            }
        };
        let mut m = match owner {
            Some(i) => {
                let mut m = datum_mark(&data[i], geometry, c, i);
                m.id = format!("cell:{k}");
                m
            }
            None => Mark::new(format!("cell:{k}"), geometry, c.theme.grid),
        };
        if owner.is_none() {
            m.label = String::new();
        }
        s.marks.push(m);
    }
    legend(&mut s, c, data);
    finish(s)
}

#[derive(Clone, Debug, PartialEq)]
pub struct RadarSeries {
    pub id: String,
    pub label: String,
    pub values: Vec<f64>,
}
#[path = "radar_styles.rs"]
mod radar_styles;
pub use radar_styles::*;
impl RadarSeries {
    pub fn new(id: impl Into<String>, values: Vec<f64>) -> Self {
        let id = id.into();
        Self {
            label: id.clone(),
            id,
            values,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct RadarOptions {
    pub max_value: Option<f64>,
    pub levels: usize,
    pub rotation: f64,
    pub fill_opacity: f64,
}
impl Default for RadarOptions {
    fn default() -> Self {
        Self {
            max_value: None,
            levels: 5,
            rotation: 0.,
            fill_opacity: 0.2,
        }
    }
}
pub fn radar(
    c: &ChartConfig,
    axes: &[String],
    data: &[RadarSeries],
    o: &RadarOptions,
) -> Result<Scene> {
    let mut s = c.scene()?;
    ensure(
        (3..=128).contains(&axes.len())
            && axes.iter().all(|a| valid_key(a))
            && axes.iter().collect::<BTreeSet<_>>().len() == axes.len(),
        "radar requires 3..128 distinct axes",
    )?;
    ensure(
        data.len() <= 128
            && data.len() * axes.len() <= MAX_MARKS
            && data.iter().all(|d| {
                valid_key(&d.id)
                    && d.id.len() <= 1000
                    && d.label.len() <= MAX_LABEL_BYTES
                    && d.values.len() == axes.len()
                    && d.values.iter().all(|v| v.is_finite() && *v >= 0.)
            })
            && data.iter().map(|d| &d.id).collect::<BTreeSet<_>>().len() == data.len(),
        "invalid radar data",
    )?;
    ensure(
        (1..=32).contains(&o.levels)
            && o.rotation.is_finite()
            && o.rotation.abs() <= 1e6
            && o.fill_opacity.is_finite()
            && (0. ..=1.).contains(&o.fill_opacity),
        "invalid radar options",
    )?;
    let max = o
        .max_value
        .unwrap_or_else(|| positive_max(data.iter().flat_map(|d| &d.values).copied()));
    ensure(
        max.is_finite() && max > 0. && data.iter().flat_map(|d| &d.values).all(|v| *v <= max),
        "invalid radar maximum",
    )?;
    let (cx, cy, r) = center(c);
    let r = r * 0.85;
    let points = |radius: f64| {
        (0..axes.len()).map(move |j| {
            point(
                cx,
                cy,
                radius,
                o.rotation + TAU * j as f64 / axes.len() as f64,
            )
        })
    };
    for level in 1..=o.levels {
        let mut m = Mark::new(
            format!("grid:{level}"),
            polygon(points(r * level as f64 / o.levels as f64)),
            c.theme.grid,
        );
        m.fill = None;
        m.stroke = Some(GraphicStroke {
            color: c.theme.grid,
            width: 1.,
            cap: StrokeCap::Butt,
        });
        m.label = String::new();
        s.marks.push(m);
    }
    for (j, axis) in axes.iter().enumerate() {
        let p = point(
            cx,
            cy,
            r * 1.1,
            o.rotation + TAU * j as f64 / axes.len() as f64,
        );
        label(&mut s, c, p.x, p.y, axis);
    }
    for (i, d) in data.iter().enumerate() {
        let mut m = Mark::new(
            format!("series:{}", d.id),
            polygon(d.values.iter().enumerate().map(|(j, v)| {
                point(
                    cx,
                    cy,
                    r * (v / max),
                    o.rotation + TAU * j as f64 / axes.len() as f64,
                )
            })),
            c.theme.color(i),
        );
        m.series = d.id.clone();
        m.label = d.label.clone();
        m.opacity = o.fill_opacity;
        m.stroke = Some(GraphicStroke {
            color: c.theme.color(i),
            width: 2.,
            cap: StrokeCap::Round,
        });
        s.marks.push(m);
        for (j, v) in d.values.iter().enumerate() {
            let p = point(
                cx,
                cy,
                r * (v / max),
                o.rotation + TAU * j as f64 / axes.len() as f64,
            );
            let mut m = Mark::new(
                format!("point:{i}:{j}"),
                Geometry::Circle {
                    x: p.x,
                    y: p.y,
                    radius: 3.,
                },
                c.theme.color(i),
            );
            m.series = d.id.clone();
            m.label = format!("{} · {}", d.label, axes[j]);
            m.value = *v;
            s.marks.push(m);
        }
        s.legend.push(LegendItem {
            id: d.id.clone(),
            label: d.label.clone(),
            color: c.theme.color(i),
        });
    }
    finish(s)
}

/// Flat ownership avoids recursive input traversal and makes cycles/missing
/// parents explicit errors. Traditional layouts require zero-valued internal
/// nodes. The typed Sunburst layout also accepts internal-node weights.
#[derive(Clone, Debug, PartialEq)]
pub struct HierarchyDatum {
    pub id: String,
    pub label: String,
    pub parent: Option<String>,
    pub value: f64,
}
impl HierarchyDatum {
    pub fn new(id: impl Into<String>, parent: Option<String>, value: f64) -> Self {
        let id = id.into();
        Self {
            label: id.clone(),
            id,
            parent,
            value,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct HierarchyOptions {
    pub padding: f64,
    pub labels: bool,
    pub include_root: bool,
}
impl Default for HierarchyOptions {
    fn default() -> Self {
        Self {
            padding: 2.,
            labels: true,
            include_root: false,
        }
    }
}
#[derive(Clone, Debug)]
pub(crate) struct Hierarchy {
    pub(crate) children: Vec<Vec<usize>>,
    pub(crate) parent: Vec<Option<usize>>,
    pub(crate) totals: Vec<f64>,
    pub(crate) depth: Vec<usize>,
    pub(crate) order: Vec<usize>,
    pub(crate) root: usize,
    pub(crate) max_depth: usize,
}
pub(crate) fn hierarchy(data: &[HierarchyDatum]) -> Result<Option<Hierarchy>> {
    hierarchy_with_weights(data, false)
}

pub(crate) fn hierarchy_with_self_values(data: &[HierarchyDatum]) -> Result<Option<Hierarchy>> {
    hierarchy_with_weights(data, true)
}

fn hierarchy_with_weights(data: &[HierarchyDatum], self_values: bool) -> Result<Option<Hierarchy>> {
    ensure(data.len() <= 8_000, "hierarchy exceeds 8000 node budget")?;
    if data.is_empty() {
        return Ok(None);
    }
    let mut ids = BTreeMap::new();
    for (i, d) in data.iter().enumerate() {
        ensure(
            valid_key(&d.id) && d.id.len() <= 1000 && d.label.len() <= MAX_LABEL_BYTES,
            "invalid hierarchy identity/label",
        )?;
        ensure(
            d.value.is_finite() && d.value >= 0.,
            "hierarchy values must be finite and nonnegative",
        )?;
        ensure(
            ids.insert(d.id.as_str(), i).is_none(),
            "duplicate hierarchy identity",
        )?;
    }
    let mut children = vec![vec![]; data.len()];
    let mut parent = vec![None; data.len()];
    let mut roots = vec![];
    for (i, d) in data.iter().enumerate() {
        if let Some(p) = &d.parent {
            let index = *ids
                .get(p.as_str())
                .ok_or_else(|| ChartError("missing hierarchy parent".into()))?;
            ensure(index != i, "hierarchy self cycle")?;
            parent[i] = Some(index);
            children[index].push(i);
        } else {
            roots.push(i);
        }
    }
    ensure(roots.len() == 1, "hierarchy requires exactly one root")?;
    let root = roots[0];
    let mut depth = vec![0; data.len()];
    let mut queue = VecDeque::from([root]);
    let mut order = vec![];
    while let Some(i) = queue.pop_front() {
        order.push(i);
        ensure(depth[i] <= 128, "hierarchy depth exceeds 128")?;
        for &child in &children[i] {
            depth[child] = depth[i] + 1;
            queue.push_back(child);
        }
    }
    ensure(
        order.len() == data.len(),
        "hierarchy contains a cycle or disconnected component",
    )?;
    let mut totals: Vec<f64> = data.iter().map(|d| d.value).collect();
    for &i in order.iter().rev() {
        if !children[i].is_empty() {
            ensure(
                self_values || data[i].value == 0.,
                "internal hierarchy nodes must have value zero",
            )?;
            totals[i] = if self_values {
                // D3 adds each child to the node's own value in reverse order.
                children[i]
                    .iter()
                    .rev()
                    .fold(data[i].value, |sum, j| sum + totals[*j])
            } else {
                children[i].iter().map(|j| totals[*j]).sum()
            };
            ensure(totals[i].is_finite(), "hierarchy sum overflow")?;
        }
    }
    let max_depth = *depth.iter().max().unwrap_or(&0);
    Ok(Some(Hierarchy {
        children,
        parent,
        totals,
        depth,
        order,
        root,
        max_depth,
    }))
}
fn hierarchy_options(o: &HierarchyOptions) -> Result<()> {
    ensure(
        o.padding.is_finite() && o.padding >= 0. && o.padding <= 1024.,
        "invalid hierarchy padding",
    )
}
#[derive(Clone, Copy)]
pub(crate) struct Rect {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) w: f64,
    pub(crate) h: f64,
}
pub(crate) fn binary_tiles(indices: &[usize], totals: &[f64], rect: Rect) -> Vec<(usize, Rect)> {
    if indices.is_empty() {
        return vec![];
    }
    let mut stack = vec![(0, indices.len(), rect)];
    let mut out = vec![];
    while let Some((start, end, r)) = stack.pop() {
        if end - start == 1 {
            out.push((indices[start], r));
            continue;
        }
        let sum: f64 = indices[start..end].iter().map(|i| totals[*i]).sum();
        let mut accum = 0.;
        let mut split = start + 1;
        let mut best = f64::INFINITY;
        for (j, &i) in indices.iter().enumerate().take(end - 1).skip(start) {
            accum += totals[i];
            let distance = (accum - sum / 2.).abs();
            if distance < best {
                best = distance;
                split = j + 1;
            } else {
                break;
            }
        }
        let left: f64 = indices[start..split].iter().map(|i| totals[*i]).sum();
        let ratio = if sum > 0. {
            left / sum
        } else {
            (split - start) as f64 / (end - start) as f64
        };
        let (a, b) = if r.w >= r.h {
            let width = r.w * ratio;
            (
                Rect { w: width, ..r },
                Rect {
                    x: r.x + width,
                    w: r.w - width,
                    ..r
                },
            )
        } else {
            let height = r.h * ratio;
            (
                Rect { h: height, ..r },
                Rect {
                    y: r.y + height,
                    h: r.h - height,
                    ..r
                },
            )
        };
        stack.push((split, end, b));
        stack.push((start, split, a));
    }
    out
}
fn node_mark(
    data: &[HierarchyDatum],
    h: &Hierarchy,
    i: usize,
    g: Geometry,
    c: &ChartConfig,
) -> Mark {
    let mut m = Mark::new(format!("node:{}", data[i].id), g, c.theme.color(i));
    m.series = data[i].id.clone();
    m.label = data[i].label.clone();
    m.value = h.totals[i];
    m
}
/// Available hierarchy tiling strategies, matching Nivo's `tileByType` choices.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TreemapTile {
    Binary,
    #[default]
    Squarify,
    Slice,
    Dice,
    SliceDice,
}
pub(crate) fn slice_tiles(
    indices: &[usize],
    totals: &[f64],
    rect: Rect,
    horizontal: bool,
) -> Vec<(usize, Rect)> {
    let total: f64 = indices.iter().map(|i| totals[*i]).sum();
    let mut position = if horizontal { rect.x } else { rect.y };
    indices
        .iter()
        .map(|&i| {
            let fraction = totals[i] / total;
            let tile = if horizontal {
                let width = rect.w * fraction;
                let tile = Rect {
                    x: position,
                    w: width,
                    ..rect
                };
                position += width;
                tile
            } else {
                let height = rect.h * fraction;
                let tile = Rect {
                    y: position,
                    h: height,
                    ..rect
                };
                position += height;
                tile
            };
            (i, tile)
        })
        .collect()
}
/// Greedy squarification: accumulate a row while its worst aspect ratio improves,
/// then place that row along the shorter side of the remaining rectangle.
/// Reference: Nivo treemap/src/tiling.ts and the Bruls/Huizing/van Wijk algorithm;
/// D3's implementation uses the same row objective with a golden-ratio target.
fn squarify_tiles(indices: &[usize], totals: &[f64], mut rect: Rect) -> Vec<(usize, Rect)> {
    let total: f64 = indices.iter().map(|i| totals[*i]).sum();
    let area = rect.w * rect.h;
    let mut items: Vec<_> = indices
        .iter()
        .map(|&i| (i, area * (totals[i] / total)))
        .collect();
    items.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    let mut out = Vec::with_capacity(items.len());
    let mut start = 0;
    while start < items.len() {
        let side = rect.w.min(rect.h);
        let worst = |sum: f64, min: f64, max: f64| {
            if side <= 0. || min <= 0. || sum <= 0. {
                f64::INFINITY
            } else {
                ((side / sum) * (side / sum) * max).max((sum / side) * (sum / side) / min)
            }
        };
        let mut end = start + 1;
        let mut sum = items[start].1;
        let mut min = sum;
        let mut max = sum;
        let mut score = worst(sum, min, max);
        while end < items.len() {
            let value = items[end].1;
            let candidate = worst(sum + value, min.min(value), max.max(value));
            if candidate > score {
                break;
            }
            sum += value;
            min = min.min(value);
            max = max.max(value);
            score = candidate;
            end += 1;
        }
        if rect.w >= rect.h {
            let width = if rect.h > 0. {
                (sum / rect.h).min(rect.w)
            } else {
                0.
            };
            let mut y = rect.y;
            for &(i, area) in &items[start..end] {
                let height = if width > 0. { area / width } else { 0. };
                out.push((
                    i,
                    Rect {
                        x: rect.x,
                        y,
                        w: width,
                        h: height,
                    },
                ));
                y += height;
            }
            rect.x += width;
            rect.w = (rect.w - width).max(0.);
        } else {
            let height = if rect.w > 0. {
                (sum / rect.w).min(rect.h)
            } else {
                0.
            };
            let mut x = rect.x;
            for &(i, area) in &items[start..end] {
                let width = if height > 0. { area / height } else { 0. };
                out.push((
                    i,
                    Rect {
                        x,
                        y: rect.y,
                        w: width,
                        h: height,
                    },
                ));
                x += width;
            }
            rect.y += height;
            rect.h = (rect.h - height).max(0.);
        }
        start = end;
    }
    out
}
/// Binary treemap, retained as the original API's tiling default.
pub fn treemap(c: &ChartConfig, data: &[HierarchyDatum], o: &HierarchyOptions) -> Result<Scene> {
    treemap_with_tile(c, data, o, TreemapTile::Binary)
}
/// All positive leaf areas are proportional to weights. Padding is applied as a
/// visual inset after subdivision, so it does not change the allocated areas.
pub fn treemap_with_tile(
    c: &ChartConfig,
    data: &[HierarchyDatum],
    o: &HierarchyOptions,
    tile: TreemapTile,
) -> Result<Scene> {
    let mut s = c.scene()?;
    hierarchy_options(o)?;
    let Some(h) = hierarchy(data)? else {
        return finish(s);
    };
    if h.totals[h.root] == 0. {
        return finish(s);
    }
    let mut boxes = vec![
        Rect {
            x: 0.,
            y: 0.,
            w: 0.,
            h: 0.
        };
        data.len()
    ];
    boxes[h.root] = Rect {
        x: c.margin.left,
        y: c.margin.top,
        w: c.inner_width(),
        h: c.inner_height(),
    };
    for &i in &h.order {
        let r = boxes[i];
        if h.children[i].is_empty() {
            if h.totals[i] > 0. {
                let p = o.padding.min(r.w).min(r.h) / 2.;
                s.marks.push(node_mark(
                    data,
                    &h,
                    i,
                    Geometry::Rect {
                        x: r.x + p,
                        y: r.y + p,
                        width: (r.w - 2. * p).max(0.),
                        height: (r.h - 2. * p).max(0.),
                    },
                    c,
                ));
                if o.labels && r.w > c.theme.font_size * 3. && r.h > c.theme.font_size * 1.5 {
                    owned_label(
                        &mut s,
                        c,
                        r.x + r.w / 2.,
                        r.y + r.h / 2.,
                        &data[i].label,
                        &data[i].id,
                    );
                }
            }
        } else {
            let positive: Vec<usize> = h.children[i]
                .iter()
                .copied()
                .filter(|j| h.totals[*j] > 0.)
                .collect();
            let tiles = match tile {
                TreemapTile::Binary => binary_tiles(&positive, &h.totals, r),
                TreemapTile::Squarify => squarify_tiles(&positive, &h.totals, r),
                TreemapTile::Dice => slice_tiles(&positive, &h.totals, r, true),
                TreemapTile::Slice => slice_tiles(&positive, &h.totals, r, false),
                TreemapTile::SliceDice => slice_tiles(&positive, &h.totals, r, h.depth[i] % 2 == 0),
            };
            for (j, rect) in tiles {
                boxes[j] = rect;
            }
        }
    }
    finish(s)
}
/// Orientation of a cached squarification strip. Horizontal strips span the
/// parent's width and arrange children left-to-right; vertical strips span its
/// height and arrange children top-to-bottom.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TreemapRowOrientation {
    Horizontal,
    Vertical,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreemapRow {
    pub orientation: TreemapRowOrientation,
    pub children: Vec<String>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ResquarifyStats {
    pub reused_parents: usize,
    pub rebuilt_parents: usize,
}
/// Stateful resquarification, following d3-hierarchy's resquarify contract.
/// Accepted row membership and orientation survive weight, dimension and flat
/// input-order changes. A changed direct-child ID set rebuilds only that parent;
/// changing the target ratio invalidates all rows. Zero-valued children remain
/// in row topology. No prior scenes or removed parent IDs are retained.
#[derive(Clone, Debug, PartialEq)]
pub struct Resquarify {
    ratio: f64,
    pub(crate) parents: BTreeMap<String, Arc<[TreemapRow]>>,
    pub(crate) stats: ResquarifyStats,
}
impl Default for Resquarify {
    fn default() -> Self {
        Self {
            ratio: (1. + 5_f64.sqrt()) / 2.,
            parents: BTreeMap::new(),
            stats: ResquarifyStats::default(),
        }
    }
}
fn validate_treemap_ratio(ratio: f64) -> Result<()> {
    ensure(
        ratio.is_finite() && (1. ..=1_000_000.).contains(&ratio),
        "treemap target ratio must be finite and in 1..=1000000",
    )
}
impl Resquarify {
    pub fn new(ratio: f64) -> Result<Self> {
        validate_treemap_ratio(ratio)?;
        Ok(Self {
            ratio,
            ..Self::default()
        })
    }
    pub fn ratio(&self) -> f64 {
        self.ratio
    }
    pub fn set_ratio(&mut self, ratio: f64) -> Result<()> {
        validate_treemap_ratio(ratio)?;
        if self.ratio != ratio {
            self.ratio = ratio;
            self.clear();
        }
        Ok(())
    }
    pub fn clear(&mut self) {
        self.parents.clear();
        self.stats = ResquarifyStats::default();
    }
    pub fn cached_parents(&self) -> usize {
        self.parents.len()
    }
    pub fn cached_children(&self) -> usize {
        self.parents
            .values()
            .flat_map(|rows| rows.iter())
            .map(|row| row.children.len())
            .sum()
    }
    pub fn last_stats(&self) -> ResquarifyStats {
        self.stats
    }
    pub fn rows(&self, parent: &str) -> Option<&[TreemapRow]> {
        self.parents.get(parent).map(|rows| rows.as_ref())
    }
    /// Publish the scene and new cache together only after full validation.
    pub fn layout(
        &mut self,
        c: &ChartConfig,
        data: &[HierarchyDatum],
        o: &HierarchyOptions,
    ) -> Result<Scene> {
        let mut scene = c.scene()?;
        hierarchy_options(o)?;
        let hierarchy = hierarchy(data)?;
        let mut parents = BTreeMap::new();
        let mut stats = ResquarifyStats::default();
        if let Some(h) = hierarchy {
            let index: BTreeMap<&str, usize> = data
                .iter()
                .enumerate()
                .map(|(i, node)| (node.id.as_str(), i))
                .collect();
            let mut boxes = vec![
                Rect {
                    x: 0.,
                    y: 0.,
                    w: 0.,
                    h: 0.
                };
                data.len()
            ];
            boxes[h.root] = Rect {
                x: c.margin.left,
                y: c.margin.top,
                w: c.inner_width(),
                h: c.inner_height(),
            };
            for &i in &h.order {
                let rect = boxes[i];
                if h.children[i].is_empty() {
                    if h.totals[i] > 0. {
                        let pad = o.padding.min(rect.w).min(rect.h) / 2.;
                        scene.marks.push(node_mark(
                            data,
                            &h,
                            i,
                            Geometry::Rect {
                                x: rect.x + pad,
                                y: rect.y + pad,
                                width: (rect.w - 2. * pad).max(0.),
                                height: (rect.h - 2. * pad).max(0.),
                            },
                            c,
                        ));
                        if o.labels
                            && rect.w > c.theme.font_size * 3.
                            && rect.h > c.theme.font_size * 1.5
                        {
                            owned_label(
                                &mut scene,
                                c,
                                rect.x + rect.w / 2.,
                                rect.y + rect.h / 2.,
                                &data[i].label,
                                &data[i].id,
                            );
                        }
                    }
                    continue;
                }
                let children: BTreeSet<&str> =
                    h.children[i].iter().map(|j| data[*j].id.as_str()).collect();
                let reusable = self.parents.get(&data[i].id).filter(|rows| {
                    rows.iter().map(|row| row.children.len()).sum::<usize>() == children.len()
                        && rows
                            .iter()
                            .flat_map(|row| row.children.iter())
                            .all(|id| children.contains(id.as_str()))
                });
                let rows: Arc<[TreemapRow]> = if let Some(rows) = reusable {
                    stats.reused_parents += 1;
                    rows.clone()
                } else {
                    stats.rebuilt_parents += 1;
                    initial_treemap_rows(&h.children[i], &h.totals, data, rect, self.ratio).into()
                };
                for (child, tile) in layout_treemap_rows(&rows, &index, &h.totals, rect) {
                    boxes[child] = tile;
                }
                parents.insert(data[i].id.clone(), rows);
            }
        }
        scene.validate()?;
        self.parents = parents;
        self.stats = stats;
        Ok(scene)
    }
}
fn row_orientation(rect: Rect) -> TreemapRowOrientation {
    if rect.w >= rect.h {
        TreemapRowOrientation::Vertical
    } else {
        TreemapRowOrientation::Horizontal
    }
}
fn remaining_rect(
    mut rect: Rect,
    orientation: TreemapRowOrientation,
    fraction: f64,
) -> (Rect, Rect) {
    match orientation {
        TreemapRowOrientation::Horizontal => {
            let height = rect.h * fraction;
            let strip = Rect { h: height, ..rect };
            rect.y += height;
            rect.h = (rect.h - height).max(0.);
            (strip, rect)
        }
        TreemapRowOrientation::Vertical => {
            let width = rect.w * fraction;
            let strip = Rect { w: width, ..rect };
            rect.x += width;
            rect.w = (rect.w - width).max(0.);
            (strip, rect)
        }
    }
}
/// Initial row construction uses the D3 target-ratio objective. It is never
/// called for a cache hit. References: local Nivo treemap/src/tiling.ts and
/// d3-hierarchy/src/treemap/{squarify,resquarify}.js (no local JS runtime).
pub(crate) fn initial_treemap_rows(
    children: &[usize],
    totals: &[f64],
    data: &[HierarchyDatum],
    mut rect: Rect,
    ratio: f64,
) -> Vec<TreemapRow> {
    let mut order = children.to_vec();
    order.sort_by(|a, b| {
        totals[*b]
            .total_cmp(&totals[*a])
            .then_with(|| data[*a].id.cmp(&data[*b].id))
    });
    let maximum = positive_max(order.iter().map(|i| totals[*i]));
    let masses: Vec<_> = order.iter().map(|i| totals[*i] / maximum).collect();
    let mut suffix = vec![0.; masses.len() + 1];
    for i in (0..masses.len()).rev() {
        suffix[i] = suffix[i + 1] + masses[i];
    }
    let mut rows = vec![];
    let mut start = 0;
    while start < order.len() {
        let orientation = row_orientation(rect);
        let remaining = suffix[start];
        if masses[start] == 0. || rect.w == 0. || rect.h == 0. {
            rows.push(TreemapRow {
                orientation,
                children: order[start..].iter().map(|i| data[*i].id.clone()).collect(),
            });
            break;
        }
        let aspect = (rect.w / rect.h).max(rect.h / rect.w);
        let worst = |sum: f64, min: f64, max: f64| {
            let beta = sum * (sum / remaining) * aspect / ratio;
            if !beta.is_finite() || beta <= 0. || min <= 0. {
                f64::INFINITY
            } else {
                (max / beta).max(beta / min)
            }
        };
        let mut end = start + 1;
        let (mut sum, mut min, mut max) = (masses[start], masses[start], masses[start]);
        let mut score = worst(sum, min, max);
        while end < order.len() && masses[end] > 0. {
            let value = masses[end];
            let next = worst(sum + value, min.min(value), max.max(value));
            if next > score {
                break;
            }
            sum += value;
            min = min.min(value);
            max = max.max(value);
            score = next;
            end += 1;
        }
        rows.push(TreemapRow {
            orientation,
            children: order[start..end]
                .iter()
                .map(|i| data[*i].id.clone())
                .collect(),
        });
        rect = remaining_rect(rect, orientation, (sum / remaining).clamp(0., 1.)).1;
        start = end;
    }
    rows
}
pub(crate) fn layout_treemap_rows(
    rows: &[TreemapRow],
    index: &BTreeMap<&str, usize>,
    totals: &[f64],
    mut rect: Rect,
) -> Vec<(usize, Rect)> {
    let maximum = positive_max(
        rows.iter()
            .flat_map(|row| &row.children)
            .map(|id| totals[index[id.as_str()]]),
    );
    let row_masses: Vec<f64> = rows
        .iter()
        .map(|row| {
            row.children
                .iter()
                .map(|id| totals[index[id.as_str()]] / maximum)
                .sum()
        })
        .collect();
    let mut suffix = vec![0.; rows.len() + 1];
    for i in (0..rows.len()).rev() {
        suffix[i] = suffix[i + 1] + row_masses[i];
    }
    let mut tiles = vec![];
    for (r, row) in rows.iter().enumerate() {
        let fraction = if suffix[r] > 0. {
            (row_masses[r] / suffix[r]).clamp(0., 1.)
        } else {
            0.
        };
        let (strip, remaining) = remaining_rect(rect, row.orientation, fraction);
        rect = remaining;
        let mut prefix = 0.;
        for id in &row.children {
            let i = index[id.as_str()];
            let mass = totals[i] / maximum;
            let low = if row_masses[r] > 0. {
                (prefix / row_masses[r]).clamp(0., 1.)
            } else {
                0.
            };
            prefix += mass;
            let high = if row_masses[r] > 0. {
                (prefix / row_masses[r]).clamp(low, 1.)
            } else {
                low
            };
            let tile = match row.orientation {
                TreemapRowOrientation::Horizontal => Rect {
                    x: strip.x + strip.w * low,
                    w: strip.w * (high - low),
                    ..strip
                },
                TreemapRowOrientation::Vertical => Rect {
                    y: strip.y + strip.h * low,
                    h: strip.h * (high - low),
                    ..strip
                },
            };
            tiles.push((i, tile));
        }
    }
    tiles
}
/// Stateless target-ratio squarification. Use a retained Resquarify instance
/// when subsequent frames should preserve the initial row topology.
pub fn squarified_treemap(
    c: &ChartConfig,
    data: &[HierarchyDatum],
    o: &HierarchyOptions,
    ratio: f64,
) -> Result<Scene> {
    Resquarify::new(ratio)?.layout(c, data, o)
}
fn partition(h: &Hierarchy) -> Vec<(f64, f64)> {
    let mut ranges = vec![(0., 0.); h.children.len()];
    ranges[h.root] = (0., 1.);
    for &i in &h.order {
        let (a, b) = ranges[i];
        let mut x = a;
        for &child in &h.children[i] {
            let extent = if h.totals[i] > 0. {
                (b - a) * h.totals[child] / h.totals[i]
            } else {
                0.
            };
            ranges[child] = (x, x + extent);
            x += extent;
        }
    }
    ranges
}
/// Equal-area radial partition, using square-root radial boundaries like Nivo's
/// sunburst d3 partition size [2pi, radius²]. The root disk is optional.
pub fn sunburst(c: &ChartConfig, data: &[HierarchyDatum], o: &HierarchyOptions) -> Result<Scene> {
    let mut s = c.scene()?;
    hierarchy_options(o)?;
    let Some(h) = hierarchy(data)? else {
        return finish(s);
    };
    if h.totals[h.root] == 0. {
        return finish(s);
    }
    let ranges = partition(&h);
    let (cx, cy, r) = center(c);
    let bands = (h.max_depth + 1) as f64;
    for &i in &h.order {
        if (!o.include_root && i == h.root) || h.totals[i] == 0. {
            continue;
        }
        let (a, b) = ranges[i];
        let inner = r * (h.depth[i] as f64 / bands).sqrt();
        let outer = r * ((h.depth[i] + 1) as f64 / bands).sqrt();
        let gap = (o.padding / r.max(1.)).min((b - a) * TAU / 2.);
        let start = a * TAU + gap / 2.;
        let end = b * TAU - gap / 2.;
        if end <= start {
            continue;
        }
        s.marks.push(node_mark(
            data,
            &h,
            i,
            sector(cx, cy, inner, outer, start, end),
            c,
        ));
        if o.labels && (end - start) * (inner + outer) / 2. > c.theme.font_size * 3. {
            let p = point(cx, cy, (inner + outer) / 2., (start + end) / 2.);
            owned_label(&mut s, c, p.x, p.y, &data[i].label, &data[i].id);
        }
    }
    finish(s)
}
pub fn icicle(c: &ChartConfig, data: &[HierarchyDatum], o: &HierarchyOptions) -> Result<Scene> {
    let mut s = c.scene()?;
    hierarchy_options(o)?;
    let Some(h) = hierarchy(data)? else {
        return finish(s);
    };
    if h.totals[h.root] == 0. {
        return finish(s);
    }
    let ranges = partition(&h);
    let first = usize::from(!o.include_root);
    let levels = (h.max_depth + 1).saturating_sub(first).max(1);
    let band = c.inner_height() / levels as f64;
    for &i in &h.order {
        if h.depth[i] < first || h.totals[i] == 0. {
            continue;
        }
        let (a, b) = ranges[i];
        let width = (b - a) * c.inner_width();
        let p = o.padding.min(width).min(band) / 2.;
        let x = c.margin.left + a * c.inner_width();
        let y = c.margin.top + (h.depth[i] - first) as f64 * band;
        s.marks.push(node_mark(
            data,
            &h,
            i,
            Geometry::Rect {
                x: x + p,
                y: y + p,
                width: (width - 2. * p).max(0.),
                height: (band - 2. * p).max(0.),
            },
            c,
        ));
        if o.labels && width > c.theme.font_size * 3. && band > c.theme.font_size * 1.5 {
            owned_label(
                &mut s,
                c,
                x + width / 2.,
                y + band / 2.,
                &data[i].label,
                &data[i].id,
            );
        }
    }
    finish(s)
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TreeMode {
    #[default]
    Tidy,
    Cluster,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TreeOrientation {
    #[default]
    LeftToRight,
    RightToLeft,
    TopToBottom,
    BottomToTop,
    Radial,
}
#[path = "tree_styles.rs"]
mod tree_styles;
pub use tree_styles::*;
#[path = "tree_layout.rs"]
mod tree_layout;
pub use tree_layout::*;
#[derive(Clone, Debug, PartialEq)]
pub struct TreeOptions {
    pub hierarchy: HierarchyOptions,
    pub mode: TreeMode,
    pub orientation: TreeOrientation,
    pub node_radius: f64,
    pub sibling_separation: f64,
    pub subtree_separation: f64,
    pub rotation: f64,
    pub style: TreeStyleOptions,
}
impl Default for TreeOptions {
    fn default() -> Self {
        Self {
            hierarchy: HierarchyOptions::default(),
            mode: TreeMode::Tidy,
            orientation: TreeOrientation::LeftToRight,
            node_radius: 4.,
            sibling_separation: 1.,
            subtree_separation: 2.,
            rotation: 0.,
            style: TreeStyleOptions::default(),
        }
    }
}
// Reingold–Tilford tidy layout with Buchheim's deferred subtree shifts/threads.
// The primary reference is d3-hierarchy/src/tree.js, also used by Nivo's tree
// hooks. Typed index arrays keep input ownership flat; no recursive traversal.
struct Tidy<'a> {
    children: Vec<Vec<usize>>,
    parent: Vec<usize>,
    index: Vec<usize>,
    prelim: Vec<f64>,
    modifier: Vec<f64>,
    change: Vec<f64>,
    shift: Vec<f64>,
    thread: Vec<Option<usize>>,
    ancestor: Vec<usize>,
    default_ancestor: Vec<Option<usize>>,
    options: &'a TreeOptions,
}
impl<'a> Tidy<'a> {
    fn new(h: &Hierarchy, options: &'a TreeOptions) -> Self {
        let n = h.children.len();
        let mut children = h.children.clone();
        children.push(vec![h.root]);
        let mut parent: Vec<_> = h.parent.iter().map(|p| p.unwrap_or(n)).collect();
        parent.push(n);
        let mut index = vec![0; n + 1];
        for list in &children {
            for (i, &v) in list.iter().enumerate() {
                index[v] = i;
            }
        }
        Self {
            children,
            parent,
            index,
            prelim: vec![0.; n + 1],
            modifier: vec![0.; n + 1],
            change: vec![0.; n + 1],
            shift: vec![0.; n + 1],
            thread: vec![None; n + 1],
            ancestor: (0..=n).collect(),
            default_ancestor: vec![None; n + 1],
            options,
        }
    }
    fn separation(&self, a: usize, b: usize) -> f64 {
        if self.parent[a] == self.parent[b] {
            self.options.sibling_separation
        } else {
            self.options.subtree_separation
        }
    }
    fn left(&self, v: usize) -> Option<usize> {
        self.children[v].first().copied().or(self.thread[v])
    }
    fn right(&self, v: usize) -> Option<usize> {
        self.children[v].last().copied().or(self.thread[v])
    }
    fn move_subtree(&mut self, left: usize, right: usize, distance: f64) {
        let change = distance / (self.index[right] - self.index[left]) as f64;
        self.change[right] -= change;
        self.shift[right] += distance;
        self.change[left] += change;
        self.prelim[right] += distance;
        self.modifier[right] += distance;
    }
    fn apportion(
        &mut self,
        v: usize,
        previous: Option<usize>,
        mut ancestor: usize,
    ) -> Result<usize> {
        let Some(w) = previous else {
            return Ok(ancestor);
        };
        let (mut inner_left, mut inner_right) = (w, v);
        let (mut outer_left, mut outer_right) = (self.children[self.parent[v]][0], v);
        let (mut sum_left, mut sum_right) = (self.modifier[w], self.modifier[v]);
        let (mut outside_left, mut outside_right) = (self.modifier[outer_left], self.modifier[v]);
        let (remaining_left, remaining_right) = loop {
            let left = self.right(inner_left);
            let right = self.left(inner_right);
            let (Some(left), Some(right)) = (left, right) else {
                break (left, right);
            };
            inner_left = left;
            inner_right = right;
            outer_left = self
                .left(outer_left)
                .ok_or_else(|| ChartError("invalid tidy left contour".into()))?;
            outer_right = self
                .right(outer_right)
                .ok_or_else(|| ChartError("invalid tidy right contour".into()))?;
            self.ancestor[outer_right] = v;
            let distance =
                self.prelim[inner_left] + sum_left - self.prelim[inner_right] - sum_right
                    + self.separation(inner_left, inner_right);
            if distance > 0. {
                let candidate = self.ancestor[inner_left];
                let a = if self.parent[candidate] == self.parent[v] {
                    candidate
                } else {
                    ancestor
                };
                ensure(self.index[a] < self.index[v], "invalid tidy ancestor order")?;
                self.move_subtree(a, v, distance);
                sum_right += distance;
                outside_right += distance;
            }
            sum_left += self.modifier[inner_left];
            sum_right += self.modifier[inner_right];
            outside_left += self.modifier[outer_left];
            outside_right += self.modifier[outer_right];
        };
        if remaining_left.is_some() && self.right(outer_right).is_none() {
            self.thread[outer_right] = remaining_left;
            self.modifier[outer_right] += sum_left - outside_right;
        }
        if remaining_right.is_some() && self.left(outer_left).is_none() {
            self.thread[outer_left] = remaining_right;
            self.modifier[outer_left] += sum_right - outside_left;
            ancestor = v;
        }
        Ok(ancestor)
    }
    fn layout(mut self, root: usize) -> Result<Vec<f64>> {
        let mut stack = vec![(root, false)];
        let mut postorder = vec![];
        while let Some((v, visited)) = stack.pop() {
            if visited {
                postorder.push(v);
            } else {
                stack.push((v, true));
                stack.extend(self.children[v].iter().rev().map(|v| (*v, false)));
            }
        }
        for v in postorder {
            let parent = self.parent[v];
            let previous = self.index[v]
                .checked_sub(1)
                .map(|i| self.children[parent][i]);
            if !self.children[v].is_empty() {
                let (mut shift, mut change) = (0., 0.);
                for &w in self.children[v].iter().rev() {
                    self.prelim[w] += shift;
                    self.modifier[w] += shift;
                    change += self.change[w];
                    shift += self.shift[w] + change;
                }
                let midpoint = (self.prelim[self.children[v][0]]
                    + self.prelim[*self.children[v].last().expect("nonempty")])
                    / 2.;
                if let Some(w) = previous {
                    self.prelim[v] = self.prelim[w] + self.separation(v, w);
                    self.modifier[v] = self.prelim[v] - midpoint;
                } else {
                    self.prelim[v] = midpoint;
                }
            } else if let Some(w) = previous {
                self.prelim[v] = self.prelim[w] + self.separation(v, w);
            }
            let ancestor = self.default_ancestor[parent].unwrap_or(self.children[parent][0]);
            self.default_ancestor[parent] = Some(self.apportion(v, previous, ancestor)?);
        }
        self.modifier[self.parent[root]] = -self.prelim[root];
        let mut breadth = vec![0.; self.children.len() - 1];
        let mut stack = vec![root];
        while let Some(v) = stack.pop() {
            breadth[v] = self.prelim[v] + self.modifier[self.parent[v]];
            self.modifier[v] += self.modifier[self.parent[v]];
            stack.extend(self.children[v].iter().rev());
        }
        Ok(breadth)
    }
}
fn cluster_breadth(h: &Hierarchy) -> Vec<f64> {
    let mut stack = vec![h.root];
    let mut leaves = vec![];
    while let Some(v) = stack.pop() {
        if h.children[v].is_empty() {
            leaves.push(v);
        } else {
            stack.extend(h.children[v].iter().rev());
        }
    }
    let mut breadth = vec![0.; h.children.len()];
    for (i, &v) in leaves.iter().enumerate() {
        breadth[v] = i as f64;
    }
    for &v in h.order.iter().rev() {
        if !h.children[v].is_empty() {
            breadth[v] =
                h.children[v].iter().map(|w| breadth[*w]).sum::<f64>() / h.children[v].len() as f64;
        }
    }
    breadth
}
/// Tidy/cluster node-link trees in four Cartesian directions or a radial layout.
/// Tidy mode uses Buchheim's linear-time Reingold–Tilford algorithm, including
/// subtree apportionment and threads, rather than averaging leaf coordinates.
pub fn tree_with_options(
    c: &ChartConfig,
    data: &[HierarchyDatum],
    o: &TreeOptions,
) -> Result<Scene> {
    tree_layout(c, data, o).map(|layout| layout.scene)
}
/// Ordered cluster tree. Leaves have equal spacing; a parent's vertical position
/// is the mean of its immediate children and depth determines horizontal position.
pub fn tree(c: &ChartConfig, data: &[HierarchyDatum], o: &HierarchyOptions) -> Result<Scene> {
    let mut s = c.scene()?;
    hierarchy_options(o)?;
    let Some(h) = hierarchy(data)? else {
        return finish(s);
    };
    let mut leaves = vec![];
    let mut stack = vec![h.root];
    while let Some(i) = stack.pop() {
        if h.children[i].is_empty() {
            leaves.push(i);
        } else {
            stack.extend(h.children[i].iter().rev().copied());
        }
    }
    let mut ys = vec![0.; data.len()];
    for (k, &i) in leaves.iter().enumerate() {
        ys[i] = c.margin.top + c.inner_height() * (k as f64 + 0.5) / leaves.len() as f64;
    }
    for &i in h.order.iter().rev() {
        if !h.children[i].is_empty() {
            ys[i] = h.children[i].iter().map(|j| ys[*j]).sum::<f64>() / h.children[i].len() as f64;
        }
    }
    let x = |i: usize| {
        c.margin.left
            + if h.max_depth == 0 {
                c.inner_width() / 2.
            } else {
                c.inner_width() * h.depth[i] as f64 / h.max_depth as f64
            }
    };
    for &i in &h.order {
        if let Some(p) = h.parent[i] {
            let mid = (x(p) + x(i)) / 2.;
            let mut m = Mark::new(
                format!("link:{}", data[i].id),
                Geometry::Path(vec![
                    Cmd::Move(Point::new(x(p), ys[p])),
                    Cmd::Cubic {
                        first: Point::new(mid, ys[p]),
                        second: Point::new(mid, ys[i]),
                        end: Point::new(x(i), ys[i]),
                    },
                ]),
                c.theme.grid,
            );
            m.fill = None;
            m.stroke = Some(GraphicStroke {
                color: c.theme.grid,
                width: 1.5,
                cap: StrokeCap::Round,
            });
            m.label = String::new();
            s.marks.push(m);
        }
    }
    for &i in &h.order {
        s.marks.push(node_mark(
            data,
            &h,
            i,
            Geometry::Circle {
                x: x(i),
                y: ys[i],
                radius: 4.,
            },
            c,
        ));
        if o.labels {
            owned_label(&mut s, c, x(i), ys[i] - 9., &data[i].label, &data[i].id);
        }
    }
    finish(s)
}

#[derive(Clone, Debug, PartialEq)]
pub struct FlowNode {
    pub id: String,
    pub label: String,
}
impl FlowNode {
    pub fn new(id: impl Into<String>) -> Self {
        let id = id.into();
        Self {
            label: id.clone(),
            id,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct FlowLink {
    pub source: String,
    pub target: String,
    pub value: f64,
}
#[derive(Clone, Debug, PartialEq)]
pub struct SankeyOptions {
    pub node_width: f64,
    pub node_padding: f64,
    pub link_opacity: f64,
    pub labels: bool,
}
impl Default for SankeyOptions {
    fn default() -> Self {
        Self {
            node_width: 16.,
            node_padding: 12.,
            link_opacity: 0.45,
            labels: true,
        }
    }
}
/// Column assignment corresponding to d3-sankey's four alignment policies.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SankeyAlignment {
    #[default]
    Left,
    Right,
    Center,
    Justify,
}
#[derive(Clone, Debug, PartialEq)]
pub struct SankeyLayoutOptions {
    pub appearance: SankeyOptions,
    pub alignment: SankeyAlignment,
    pub iterations: usize,
}
impl Default for SankeyLayoutOptions {
    fn default() -> Self {
        Self {
            appearance: SankeyOptions::default(),
            alignment: SankeyAlignment::Left,
            iterations: 6,
        }
    }
}
/// Align an acyclic network before iterative breadth relaxation. Source depth
/// and sink height count edges, including structural links with value zero.
pub fn sankey_with_layout(
    c: &ChartConfig,
    nodes: &[FlowNode],
    links: &[FlowLink],
    options: &SankeyLayoutOptions,
) -> Result<Scene> {
    compute_sankey(
        c,
        nodes,
        links,
        &options.appearance,
        options.iterations,
        options.alignment,
    )
}
/// Deterministic left-aligned DAG Sankey. Links share one global pixels-per-unit
/// scale; each node has max(inflow,outflow) height and links use disjoint ports.
/// Cyclic graphs are rejected rather than silently dropping links.
pub fn sankey(
    c: &ChartConfig,
    nodes: &[FlowNode],
    links: &[FlowLink],
    o: &SankeyOptions,
) -> Result<Scene> {
    sankey_with_iterations(c, nodes, links, o, 6)
}
struct SankeyRelaxation<'a> {
    edges: &'a [(usize, usize, f64)],
    depth: &'a [usize],
    heights: &'a [f64],
    incoming: &'a [f64],
    outgoing: &'a [f64],
    top: f64,
    bottom: f64,
    padding: f64,
}
impl SankeyRelaxation<'_> {
    fn quality(&self, groups: &[Vec<usize>], ys: &[f64]) -> (u64, f64) {
        let mut ranks = vec![0usize; ys.len()];
        for group in groups {
            let mut order = group.clone();
            order.sort_by(|a, b| ys[*a].total_cmp(&ys[*b]).then(a.cmp(b)));
            for (rank, i) in order.into_iter().enumerate() {
                ranks[i] = rank;
            }
        }
        let mut pairs: BTreeMap<(usize, usize), Vec<(usize, usize)>> = BTreeMap::new();
        let maximum = positive_max(self.edges.iter().map(|(_, _, v)| *v));
        let mut energy = 0.;
        for &(a, b, v) in self.edges {
            if v > 0. {
                pairs
                    .entry((self.depth[a], self.depth[b]))
                    .or_default()
                    .push((ranks[a], ranks[b]));
                let d = ys[a] + self.heights[a] / 2. - ys[b] - self.heights[b] / 2.;
                energy += (v / maximum) * d * d;
            }
        }
        let mut crossings = 0u64;
        for list in pairs.values_mut() {
            if list.len() < 2 {
                continue;
            }
            list.sort_unstable();
            let size = list.iter().map(|(_, target)| target + 2).max().unwrap_or(1);
            let mut fenwick = vec![0u64; size + 1];
            let mut seen = 0u64;
            let mut start = 0;
            while start < list.len() {
                let mut end = start + 1;
                while end < list.len() && list[end].0 == list[start].0 {
                    end += 1;
                }
                for &(_, target) in &list[start..end] {
                    let mut at = target + 1;
                    let mut below = 0;
                    while at > 0 {
                        below += fenwick[at];
                        at &= at - 1;
                    }
                    crossings += seen - below;
                }
                for &(_, target) in &list[start..end] {
                    let mut at = target + 1;
                    while at < fenwick.len() {
                        fenwick[at] += 1;
                        at += at & at.wrapping_neg();
                    }
                    seen += 1;
                }
                start = end;
            }
        }
        (crossings, energy)
    }
    fn collide(&self, group: &[usize], ys: &mut [f64]) {
        let mut edge = self.top;
        for &i in group {
            ys[i] = ys[i].max(edge);
            edge = ys[i] + self.heights[i] + self.padding;
        }
        edge = self.bottom;
        for &i in group.iter().rev() {
            ys[i] = ys[i].min(edge - self.heights[i]);
            edge = ys[i] - self.padding;
        }
    }
    fn run(&self, groups: &mut [Vec<usize>], ys: &mut [f64], iterations: usize) {
        let mut before = vec![vec![]; ys.len()];
        let mut after = vec![vec![]; ys.len()];
        for &(a, b, v) in self.edges {
            if v > 0. {
                before[b].push((a, v));
                after[a].push((b, v));
            }
        }
        let mut best = ys.to_vec();
        let mut quality = self.quality(groups, ys);
        for iteration in 0..iterations {
            let alpha = 0.9_f64.powi(iteration as i32);
            for forward in [false, true] {
                for step in 0..groups.len() {
                    let column = if forward {
                        step
                    } else {
                        groups.len() - 1 - step
                    };
                    let group = &mut groups[column];
                    let mut desired = Vec::with_capacity(group.len());
                    for &i in group.iter() {
                        let neighbours = if forward { &before[i] } else { &after[i] };
                        let total = if forward {
                            self.incoming[i]
                        } else {
                            self.outgoing[i]
                        };
                        let center = if total > 0. {
                            neighbours
                                .iter()
                                .map(|(j, v)| (v / total) * (ys[*j] + self.heights[*j] / 2.))
                                .sum::<f64>()
                        } else {
                            ys[i] + self.heights[i] / 2.
                        };
                        desired.push((i, center));
                    }
                    desired.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
                    *group = desired.iter().map(|(i, _)| *i).collect();
                    for (i, center) in desired {
                        let target = (center - self.heights[i] / 2.)
                            .clamp(self.top, (self.bottom - self.heights[i]).max(self.top));
                        ys[i] = ys[i] * (1. - alpha) + target * alpha;
                    }
                    self.collide(group, ys);
                }
            }
            let candidate = self.quality(groups, ys);
            if candidate.0 < quality.0 || (candidate.0 == quality.0 && candidate.1 < quality.1) {
                best.copy_from_slice(ys);
                quality = candidate;
            }
        }
        ys.copy_from_slice(&best);
    }
}
/// Alternating weighted-barycenter sweeps reorder and relax DAG layers, followed
/// by exact collision resolution. The best layer-pair crossing count is retained
/// (then minimum vertical link energy on ties), so more sweeps cannot return an
/// arrangement worse than the initial drawing under that objective. Zero
/// iterations preserves input node order. Link ports follow the opposite layer.
pub fn sankey_with_iterations(
    c: &ChartConfig,
    nodes: &[FlowNode],
    links: &[FlowLink],
    o: &SankeyOptions,
    iterations: usize,
) -> Result<Scene> {
    compute_sankey(c, nodes, links, o, iterations, SankeyAlignment::Left)
}
fn compute_sankey(
    c: &ChartConfig,
    nodes: &[FlowNode],
    links: &[FlowLink],
    o: &SankeyOptions,
    iterations: usize,
    alignment: SankeyAlignment,
) -> Result<Scene> {
    let mut s = c.scene()?;
    ensure(
        iterations <= 128,
        "Sankey relaxation exceeds 128 iterations",
    )?;
    ensure(
        nodes.len() <= 2_000 && links.len() <= 8_000 && nodes.len() + links.len() <= MAX_MARKS,
        "sankey input exceeds budget",
    )?;
    ensure(
        o.node_width.is_finite()
            && o.node_width > 0.
            && o.node_width < c.inner_width()
            && o.node_padding.is_finite()
            && o.node_padding >= 0.
            && o.link_opacity.is_finite()
            && (0. ..=1.).contains(&o.link_opacity),
        "invalid sankey options",
    )?;
    let mut ids = BTreeMap::new();
    for (i, n) in nodes.iter().enumerate() {
        ensure(
            valid_key(&n.id)
                && n.id.len() <= 1000
                && n.label.len() <= MAX_LABEL_BYTES
                && ids.insert(n.id.as_str(), i).is_none(),
            "invalid or duplicate sankey node",
        )?;
    }
    let mut incoming = vec![0.; nodes.len()];
    let mut outgoing = vec![0.; nodes.len()];
    let mut adj = vec![vec![]; nodes.len()];
    let mut indegree = vec![0usize; nodes.len()];
    let mut edges = vec![];
    for l in links {
        ensure(
            l.value.is_finite() && l.value >= 0.,
            "sankey values must be finite and nonnegative",
        )?;
        let a = *ids
            .get(l.source.as_str())
            .ok_or_else(|| ChartError("missing sankey source".into()))?;
        let b = *ids
            .get(l.target.as_str())
            .ok_or_else(|| ChartError("missing sankey target".into()))?;
        ensure(a != b, "sankey self cycle")?;
        adj[a].push(b);
        indegree[b] += 1;
        outgoing[a] += l.value;
        incoming[b] += l.value;
        ensure(
            outgoing[a].is_finite() && incoming[b].is_finite(),
            "sankey sum overflow",
        )?;
        edges.push((a, b, l.value));
    }
    let mut queue: VecDeque<usize> = indegree
        .iter()
        .enumerate()
        .filter_map(|(i, n)| (*n == 0).then_some(i))
        .collect();
    let source_nodes: Vec<_> = indegree.iter().map(|count| *count == 0).collect();
    let mut depth = vec![0usize; nodes.len()];
    let mut topological = Vec::with_capacity(nodes.len());
    let mut visited = 0;
    while let Some(a) = queue.pop_front() {
        visited += 1;
        topological.push(a);
        for &b in &adj[a] {
            depth[b] = depth[b].max(depth[a] + 1);
            indegree[b] -= 1;
            if indegree[b] == 0 {
                queue.push_back(b);
            }
        }
    }
    ensure(visited == nodes.len(), "sankey requires an acyclic graph")?;
    if nodes.is_empty() {
        return finish(s);
    }
    let layers = depth.iter().max().copied().unwrap_or(0) + 1;
    let mut sink_height = vec![0usize; nodes.len()];
    for &a in topological.iter().rev() {
        sink_height[a] = adj[a]
            .iter()
            .map(|b| sink_height[*b] + 1)
            .max()
            .unwrap_or(0);
    }
    let columns: Vec<_> = (0..nodes.len())
        .map(|i| match alignment {
            SankeyAlignment::Left => depth[i],
            SankeyAlignment::Right => layers - 1 - sink_height[i],
            SankeyAlignment::Center => {
                if !source_nodes[i] {
                    depth[i]
                } else {
                    adj[i]
                        .iter()
                        .map(|j| depth[*j])
                        .min()
                        .map(|d| d.saturating_sub(1))
                        .unwrap_or(0)
                }
            }
            SankeyAlignment::Justify => {
                if adj[i].is_empty() {
                    layers - 1
                } else {
                    depth[i]
                }
            }
        })
        .collect();
    ensure(
        edges.iter().all(|(a, b, _)| columns[*a] < columns[*b]),
        "Sankey alignment must preserve forward links",
    )?;
    let mut groups = vec![vec![]; layers];
    for (i, d) in columns.iter().enumerate() {
        groups[*d].push(i);
    }
    let values: Vec<f64> = incoming
        .iter()
        .zip(&outgoing)
        .map(|(a, b)| a.max(*b))
        .collect();
    let maximum = positive_max(values.iter().copied());
    let mut scale = f64::INFINITY;
    for group in &groups {
        let space = c.inner_height() - o.node_padding * group.len().saturating_sub(1) as f64;
        ensure(space > 0., "sankey node padding consumes layer")?;
        let total: f64 = group.iter().map(|i| values[*i] / maximum).sum();
        ensure(total.is_finite(), "sankey layer sum overflow")?;
        if total > 0. {
            scale = scale.min(space / total);
        }
    }
    if !scale.is_finite() {
        scale = 1.;
    }
    let heights: Vec<_> = values.iter().map(|v| (v / maximum) * scale).collect();
    let mut xs = vec![0.; nodes.len()];
    let mut ys = vec![0.; nodes.len()];
    for (d, group) in groups.iter().enumerate() {
        let total: f64 = group.iter().map(|i| heights[*i]).sum::<f64>()
            + o.node_padding * group.len().saturating_sub(1) as f64;
        let mut y = c.margin.top + (c.inner_height() - total) / 2.;
        for &i in group {
            xs[i] = c.margin.left
                + if layers == 1 {
                    (c.inner_width() - o.node_width) / 2.
                } else {
                    (c.inner_width() - o.node_width) * d as f64 / (layers - 1) as f64
                };
            ys[i] = y;
            y += heights[i] + o.node_padding;
        }
    }
    SankeyRelaxation {
        edges: &edges,
        depth: &columns,
        heights: &heights,
        incoming: &incoming,
        outgoing: &outgoing,
        top: c.margin.top,
        bottom: c.height - c.margin.bottom,
        padding: o.node_padding,
    }
    .run(&mut groups, &mut ys, iterations);
    let mut source_links = vec![vec![]; nodes.len()];
    let mut target_links = vec![vec![]; nodes.len()];
    for (k, &(a, b, _)) in edges.iter().enumerate() {
        source_links[a].push(k);
        target_links[b].push(k);
    }
    let center = |i: usize| ys[i] + heights[i] / 2.;
    for edges_at_node in &mut source_links {
        edges_at_node.sort_by(|a, b| {
            center(edges[*a].1)
                .total_cmp(&center(edges[*b].1))
                .then(a.cmp(b))
        });
    }
    for edges_at_node in &mut target_links {
        edges_at_node.sort_by(|a, b| {
            center(edges[*a].0)
                .total_cmp(&center(edges[*b].0))
                .then(a.cmp(b))
        });
    }
    let mut ports_in = ys.clone();
    let mut ports_out = ys.clone();
    let mut edge_ports = vec![(0., 0.); edges.len()];
    for (i, list) in source_links.iter().enumerate() {
        for &k in list {
            edge_ports[k].0 = ports_out[i];
            ports_out[i] += (edges[k].2 / maximum) * scale;
        }
    }
    for (i, list) in target_links.iter().enumerate() {
        for &k in list {
            edge_ports[k].1 = ports_in[i];
            ports_in[i] += (edges[k].2 / maximum) * scale;
        }
    }
    for (k, (a, b, v)) in edges.into_iter().enumerate() {
        if v == 0. {
            continue;
        }
        let width = (v / maximum) * scale;
        let x0 = xs[a] + o.node_width;
        let x1 = xs[b];
        let mid = (x0 + x1) / 2.;
        let (y0, y1) = edge_ports[k];
        let mut m = Mark::new(
            format!("flow:{k}"),
            Geometry::Path(vec![
                Cmd::Move(Point::new(x0, y0)),
                Cmd::Cubic {
                    first: Point::new(mid, y0),
                    second: Point::new(mid, y1),
                    end: Point::new(x1, y1),
                },
                Cmd::Line(Point::new(x1, y1 + width)),
                Cmd::Cubic {
                    first: Point::new(mid, y1 + width),
                    second: Point::new(mid, y0 + width),
                    end: Point::new(x0, y0 + width),
                },
                Cmd::Close,
            ]),
            c.theme.color(a),
        );
        m.series = nodes[a].id.clone();
        m.label = format!("{} → {}", nodes[a].label, nodes[b].label);
        m.value = v;
        m.opacity = o.link_opacity;
        s.marks.push(m);
    }
    for (i, n) in nodes.iter().enumerate() {
        let mut m = Mark::new(
            format!("node:{}", n.id),
            Geometry::Rect {
                x: xs[i],
                y: ys[i],
                width: o.node_width,
                height: heights[i],
            },
            c.theme.color(i),
        );
        m.series = n.id.clone();
        m.label = n.label.clone();
        m.value = values[i];
        s.marks.push(m);
        if o.labels {
            owned_label(
                &mut s,
                c,
                xs[i] + o.node_width / 2.,
                ys[i] - 5.,
                &n.label,
                &n.id,
            );
        }
    }
    finish(s)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CirclePackingAlgorithm {
    #[default]
    FrontChain,
    Shelf,
}
#[derive(Clone, Copy)]
struct Disk {
    x: f64,
    y: f64,
    r: f64,
}
fn tangent_disk(a: Disk, b: Disk, r: f64, left: bool) -> Disk {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let distance = dx.hypot(dy);
    if distance == 0. {
        return Disk {
            x: a.x + a.r + r,
            y: a.y,
            r,
        };
    }
    let ar = a.r + r;
    let br = b.r + r;
    let along = (distance * distance + ar * ar - br * br) / (2. * distance);
    let perpendicular = (ar * ar - along * along).max(0.).sqrt() * if left { 1. } else { -1. };
    Disk {
        x: a.x + (along * dx - perpendicular * dy) / distance,
        y: a.y + (along * dy + perpendicular * dx) / distance,
        r,
    }
}
fn disks_intersect(a: Disk, b: Disk) -> bool {
    (a.x - b.x).hypot(a.y - b.y) < (a.r + b.r) * (1. - 1e-10)
}
/// One disk for exact minimum-enclosure geometry. Radius zero represents a point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PackingCircle {
    pub x: f64,
    pub y: f64,
    pub radius: f64,
}
/// Shared across every enclosing-circle solve in one hierarchical packing frame.
pub const MAX_ENCLOSURE_TESTS: usize = 2_000_000;
struct EnclosureWork {
    remaining: usize,
}
impl EnclosureWork {
    fn new() -> Self {
        Self {
            remaining: MAX_ENCLOSURE_TESTS,
        }
    }
    fn check(&mut self) -> Result<()> {
        ensure(
            self.remaining > 0,
            "minimum enclosing circle work budget exceeded",
        )?;
        self.remaining -= 1;
        Ok(())
    }
}
fn enclosure_epsilon(a: Disk, b: Disk) -> f64 {
    128. * f64::EPSILON * a.r.max(b.r).max(1.)
}
fn disk_encloses(a: Disk, b: Disk) -> bool {
    (a.x - b.x).hypot(a.y - b.y) + b.r <= a.r + enclosure_epsilon(a, b)
}
fn support_pair(a: Disk, b: Disk) -> Disk {
    let distance = (a.x - b.x).hypot(a.y - b.y);
    if a.r >= distance + b.r {
        return a;
    }
    if b.r >= distance + a.r {
        return b;
    }
    let r = (distance + a.r + b.r) / 2.;
    let fraction = (r - a.r) / distance;
    Disk {
        x: a.x + (b.x - a.x) * fraction,
        y: a.y + (b.y - a.y) * fraction,
        r,
    }
}
fn polish_tangency(mut circle: Disk, disks: [Disk; 3]) -> Disk {
    for _ in 0..12 {
        let mut matrix = [[0.; 4]; 3];
        let mut residual = 0f64;
        for (row, disk) in matrix.iter_mut().zip(disks) {
            let dx = circle.x - disk.x;
            let dy = circle.y - disk.y;
            let distance = dx.hypot(dy);
            if distance == 0. {
                return circle;
            }
            let error = distance + disk.r - circle.r;
            residual = residual.max(error.abs());
            *row = [dx / distance, dy / distance, -1., -error];
        }
        if residual <= 2. * f64::EPSILON {
            return circle;
        }
        for column in 0..3 {
            let pivot = (column..3)
                .max_by(|a, b| {
                    matrix[*a][column]
                        .abs()
                        .total_cmp(&matrix[*b][column].abs())
                })
                .expect("three rows");
            if matrix[pivot][column].abs() < f64::EPSILON * f64::EPSILON {
                return circle;
            }
            matrix.swap(column, pivot);
            let value = matrix[column][column];
            for entry in &mut matrix[column][column..] {
                *entry /= value;
            }
            let pivot_row = matrix[column];
            for (row_index, row) in matrix.iter_mut().enumerate() {
                if row_index != column {
                    let factor = row[column];
                    for j in column..4 {
                        row[j] -= factor * pivot_row[j];
                    }
                }
            }
        }
        let next = Disk {
            x: circle.x + matrix[0][3],
            y: circle.y + matrix[1][3],
            r: circle.r + matrix[2][3],
        };
        if ![next.x, next.y, next.r].iter().all(|v| v.is_finite()) || next.r < 0. {
            return circle;
        }
        circle = next;
    }
    circle
}
fn is_three_support_minimum(circle: Disk, disks: [Disk; 3]) -> bool {
    let mut angles = [0.; 3];
    for (angle, disk) in angles.iter_mut().zip(disks) {
        let distance = (circle.x - disk.x).hypot(circle.y - disk.y);
        if distance == 0. || (distance + disk.r - circle.r).abs() > enclosure_epsilon(circle, disk)
        {
            return false;
        }
        *angle = (circle.y - disk.y).atan2(circle.x - disk.x);
    }
    angles.sort_by(f64::total_cmp);
    let gap = (angles[1] - angles[0])
        .max(angles[2] - angles[1])
        .max(angles[0] + TAU - angles[2]);
    gap <= std::f64::consts::PI + 128. * f64::EPSILON
}
/// Solve the external Apollonius tangency equations. Translation removes the
/// absolute coordinates; stable quadratic roots avoid subtractive cancellation.
fn support_three(a: Disk, b: Disk, c: Disk) -> Option<Disk> {
    let (bx, by, cx, cy) = (b.x - a.x, b.y - a.y, c.x - a.x, c.y - a.y);
    let determinant = bx * cy - by * cx;
    let determinant_scale = bx.hypot(by) * cx.hypot(cy);
    if determinant.abs() <= 32. * f64::EPSILON * determinant_scale {
        return None;
    }
    let qb = (bx * bx + by * by + a.r * a.r - b.r * b.r) / 2.;
    let qc = (cx * cx + cy * cy + a.r * a.r - c.r * c.r) / 2.;
    let rb = b.r - a.r;
    let rc = c.r - a.r;
    let ux = (qb * cy - by * qc) / determinant;
    let uy = (bx * qc - qb * cx) / determinant;
    let vx = (rb * cy - by * rc) / determinant;
    let vy = (bx * rc - rb * cx) / determinant;
    let aa = vx * vx + vy * vy - 1.;
    let bb = 2. * (ux * vx + uy * vy + a.r);
    let cc = ux * ux + uy * uy - a.r * a.r;
    let mut roots = [f64::NAN; 2];
    if aa.abs() <= 32. * f64::EPSILON * (vx * vx + vy * vy + 1.) {
        if bb != 0. {
            roots[0] = -cc / bb;
        }
    } else {
        let discriminant = bb * bb - 4. * aa * cc;
        let tolerance =
            64. * f64::EPSILON * (bb * bb + (4. * aa * cc).abs()).max(f64::MIN_POSITIVE);
        if discriminant < -tolerance {
            return None;
        }
        let root = discriminant.max(0.).sqrt();
        let q = -0.5 * (bb + if bb >= 0. { root } else { -root });
        if q == 0. {
            roots[0] = -bb / (2. * aa);
        } else {
            roots[0] = q / aa;
            roots[1] = cc / q;
        }
    }
    roots
        .into_iter()
        .filter_map(|r| {
            let circle = polish_tangency(
                Disk {
                    x: a.x + ux + vx * r,
                    y: a.y + uy + vy * r,
                    r,
                },
                [a, b, c],
            );
            (circle.r.is_finite()
                && circle.r >= 0.
                && circle.x.is_finite()
                && circle.y.is_finite()
                && [a, b, c]
                    .into_iter()
                    .all(|disk| disk_encloses(circle, disk))
                && is_three_support_minimum(circle, [a, b, c]))
            .then_some(circle)
        })
        .min_by(|a, b| a.r.total_cmp(&b.r))
}
fn feasible_three(radius: f64, disks: [Disk; 3], work: &mut EnclosureWork) -> Result<Option<Disk>> {
    let mut candidates: Vec<_> = disks.iter().map(|d| (d.x, d.y)).collect();
    for i in 0..3 {
        for j in i + 1..3 {
            work.check()?;
            let (a, b) = (disks[i], disks[j]);
            let dx = b.x - a.x;
            let dy = b.y - a.y;
            let distance = dx.hypot(dy);
            if distance == 0. {
                continue;
            }
            let ra = (radius - a.r).max(0.);
            let rb = (radius - b.r).max(0.);
            let epsilon = 128. * f64::EPSILON;
            if distance > ra + rb + epsilon || distance < (ra - rb).abs() - epsilon {
                continue;
            }
            let along = (distance * distance + (ra - rb) * (ra + rb)) / (2. * distance);
            let square = (ra - along) * (ra + along);
            if square < -epsilon {
                continue;
            }
            let perpendicular = square.max(0.).sqrt();
            let x = a.x + along * dx / distance;
            let y = a.y + along * dy / distance;
            candidates.push((
                x - perpendicular * dy / distance,
                y + perpendicular * dx / distance,
            ));
            candidates.push((
                x + perpendicular * dy / distance,
                y - perpendicular * dx / distance,
            ));
        }
    }
    for (x, y) in candidates {
        let mut required = 0f64;
        for disk in disks {
            work.check()?;
            required = required.max((x - disk.x).hypot(y - disk.y) + disk.r);
        }
        if required <= radius + 128. * f64::EPSILON {
            return Ok(Some(Disk { x, y, r: required }));
        }
    }
    Ok(None)
}
/// Interval-certified fallback for ill-conditioned analytic tangencies. It
/// intersects the three exact center-feasibility disks and narrows the radius
/// bracket to adjacent floating values; no approximate center search is used.
fn support_three_interval(disks: [Disk; 3], work: &mut EnclosureWork) -> Result<Disk> {
    let mut lower = disks.iter().map(|d| d.r).fold(0., f64::max);
    let mut best = Disk {
        x: 0.,
        y: 0.,
        r: f64::INFINITY,
    };
    for i in 0..3 {
        for j in i + 1..3 {
            let pair = support_pair(disks[i], disks[j]);
            lower = lower.max(pair.r);
            let required = disks
                .iter()
                .map(|d| (pair.x - d.x).hypot(pair.y - d.y) + d.r)
                .fold(0., f64::max);
            if required < best.r {
                best = Disk {
                    r: required,
                    ..pair
                };
            }
        }
    }
    let mut upper = best.r;
    for _ in 0..80 {
        work.check()?;
        let middle = lower + (upper - lower) / 2.;
        if middle == lower || middle == upper {
            break;
        }
        if let Some(circle) = feasible_three(middle, disks, work)? {
            upper = middle;
            best = circle;
        } else {
            lower = middle;
        }
    }
    Ok(best)
}
fn consider_enclosure_basis(
    pool: &[Disk],
    best: &mut Option<(Disk, Vec<Disk>)>,
    circle: Disk,
    support: &[usize],
    work: &mut EnclosureWork,
) -> Result<()> {
    work.check()?;
    for &disk in pool {
        work.check()?;
        if !disk_encloses(circle, disk) {
            return Ok(());
        }
    }
    if best.as_ref().is_none_or(|(old, _)| circle.r < old.r) {
        *best = Some((circle, support.iter().map(|i| pool[*i]).collect()));
    }
    Ok(())
}
fn extend_enclosure_basis(pool: &[Disk], work: &mut EnclosureWork) -> Result<(Disk, Vec<Disk>)> {
    let mut best: Option<(Disk, Vec<Disk>)> = None;
    for (i, &disk) in pool.iter().enumerate() {
        consider_enclosure_basis(pool, &mut best, disk, &[i], work)?;
    }
    for i in 0..pool.len() {
        for j in i + 1..pool.len() {
            consider_enclosure_basis(
                pool,
                &mut best,
                support_pair(pool[i], pool[j]),
                &[i, j],
                work,
            )?;
        }
    }
    for i in 0..pool.len() {
        for j in i + 1..pool.len() {
            for k in j + 1..pool.len() {
                let disks = [pool[i], pool[j], pool[k]];
                let pair_contains = [(0, 1), (0, 2), (1, 2)].into_iter().any(|(a, b)| {
                    let circle = support_pair(disks[a], disks[b]);
                    disks.iter().all(|d| disk_encloses(circle, *d))
                });
                if !pair_contains {
                    let circle = if let Some(circle) = support_three(disks[0], disks[1], disks[2]) {
                        circle
                    } else {
                        support_three_interval(disks, work)?
                    };
                    consider_enclosure_basis(pool, &mut best, circle, &[i, j, k], work)?;
                }
            }
        }
    }
    best.ok_or_else(|| ChartError("degenerate minimum enclosing circle support".into()))
}
/// Deterministic randomized incremental support solver. A Euclidean minimum
/// enclosing disk has at most three supporting disks; each violated constraint
/// replaces the support with the exact minimum of a set of at most four disks.
/// Coordinate normalization and canonical pseudo-shuffling make the result
/// stable under input permutation. Only floating-point rounding is tolerated.
fn enclosing_disks(disks: &[Disk], work: &mut EnclosureWork) -> Result<Disk> {
    if disks.is_empty() {
        return Ok(Disk {
            x: 0.,
            y: 0.,
            r: 0.,
        });
    }
    ensure(
        disks
            .iter()
            .all(|d| d.x.is_finite() && d.y.is_finite() && d.r.is_finite() && d.r >= 0.),
        "invalid enclosing disk geometry",
    )?;
    let minx = disks.iter().map(|d| d.x).fold(f64::INFINITY, f64::min);
    let maxx = disks.iter().map(|d| d.x).fold(f64::NEG_INFINITY, f64::max);
    let miny = disks.iter().map(|d| d.y).fold(f64::INFINITY, f64::min);
    let maxy = disks.iter().map(|d| d.y).fold(f64::NEG_INFINITY, f64::max);
    let origin = [minx / 2. + maxx / 2., miny / 2. + maxy / 2.];
    let scale = (maxx - minx)
        .max(maxy - miny)
        .max(disks.iter().map(|d| d.r).fold(0., f64::max));
    ensure(scale.is_finite(), "enclosing disk span overflow")?;
    if scale == 0. {
        return Ok(Disk {
            x: minx,
            y: miny,
            r: 0.,
        });
    }
    let mut ordered: Vec<_> = disks
        .iter()
        .map(|d| Disk {
            x: (d.x - origin[0]) / scale,
            y: (d.y - origin[1]) / scale,
            r: d.r / scale,
        })
        .collect();
    ordered.sort_by(|a, b| {
        a.x.total_cmp(&b.x)
            .then(a.y.total_cmp(&b.y))
            .then(a.r.total_cmp(&b.r))
    });
    let mut random = 0x4d595df4d0f33173u64;
    for i in (1..ordered.len()).rev() {
        random = random
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ordered.swap(i, (random % (i as u64 + 1)) as usize);
    }
    let mut basis = Vec::with_capacity(3);
    let mut enclosing = None;
    let mut i = 0;
    while i < ordered.len() {
        work.check()?;
        let disk = ordered[i];
        if enclosing.is_some_and(|outer| disk_encloses(outer, disk)) {
            i += 1;
        } else {
            let mut pool = basis;
            pool.push(disk);
            let (circle, support) = extend_enclosure_basis(&pool, work)?;
            enclosing = Some(circle);
            basis = support;
            i = 0;
        }
    }
    let circle = enclosing.expect("nonempty disks have an enclosing circle");
    let mut result = Disk {
        x: origin[0] + circle.x * scale,
        y: origin[1] + circle.y * scale,
        r: circle.r * scale,
    };
    // Widen only by final arithmetic roundoff, never by a search approximation.
    for &disk in disks {
        work.check()?;
        result.r = result
            .r
            .max((result.x - disk.x).hypot(result.y - disk.y) + disk.r);
    }
    Ok(result)
}
/// Exact smallest enclosing circle for up to 2000 disks, within floating-point
/// rounding. Empty input yields None. Invalid geometry or exhausted work budgets
/// return an error; there is no silent approximate fallback.
pub fn enclosing_circle(disks: &[PackingCircle]) -> Result<Option<PackingCircle>> {
    ensure(
        disks.len() <= 2000
            && disks.iter().all(|d| {
                [d.x, d.y]
                    .iter()
                    .all(|v| v.is_finite() && v.abs() <= 1_000_000.)
                    && d.radius.is_finite()
                    && (0. ..=1_000_000.).contains(&d.radius)
            }),
        "invalid enclosing circle input or budget",
    )?;
    if disks.is_empty() {
        return Ok(None);
    }
    let circles: Vec<_> = disks
        .iter()
        .map(|d| Disk {
            x: d.x,
            y: d.y,
            r: d.radius,
        })
        .collect();
    let circle = enclosing_disks(&circles, &mut EnclosureWork::new())?;
    Ok(Some(PackingCircle {
        x: circle.x,
        y: circle.y,
        radius: circle.r,
    }))
}
/// Tangent front-chain placement as used by d3-hierarchy pack/siblings.js.
/// Circular linked-list indices remove obscured boundary disks on collision.
fn front_chain(radii: &[f64], work: &mut EnclosureWork) -> Result<(Vec<Disk>, Disk)> {
    let mut disks: Vec<_> = radii
        .iter()
        .map(|r| Disk {
            x: 0.,
            y: 0.,
            r: *r,
        })
        .collect();
    let n = disks.len();
    if n < 2 {
        let enclosing = enclosing_disks(&disks, work)?;
        return Ok((disks, enclosing));
    }
    disks[0].x = -disks[1].r;
    disks[1].x = disks[0].r;
    if n < 3 {
        let enclosing = enclosing_disks(&disks, work)?;
        return Ok((disks, enclosing));
    }
    disks[2] = tangent_disk(disks[0], disks[1], disks[2].r, true);
    let mut next = vec![0usize; n];
    let mut previous = vec![0usize; n];
    next[0] = 1;
    next[1] = 2;
    next[2] = 0;
    previous[0] = 2;
    previous[1] = 0;
    previous[2] = 1;
    let (mut a, mut b) = (0usize, 1usize);
    let mut operations = 0usize;
    for i in 3..n {
        loop {
            operations += 1;
            ensure(
                operations <= n * n * 4,
                "front-chain placement budget exceeded",
            )?;
            let c = tangent_disk(disks[a], disks[b], disks[i].r, false);
            let (mut j, mut k) = (next[b], previous[a]);
            let (mut forward, mut backward) = (disks[b].r, disks[a].r);
            let mut retry = false;
            loop {
                if forward <= backward {
                    if disks_intersect(disks[j], c) {
                        b = j;
                        next[a] = b;
                        previous[b] = a;
                        retry = true;
                        break;
                    }
                    forward += disks[j].r;
                    j = next[j];
                } else {
                    if disks_intersect(disks[k], c) {
                        a = k;
                        next[a] = b;
                        previous[b] = a;
                        retry = true;
                        break;
                    }
                    backward += disks[k].r;
                    k = previous[k];
                }
                if j == next[k] {
                    break;
                }
            }
            if retry {
                continue;
            }
            disks[i] = c;
            previous[i] = a;
            next[i] = b;
            next[a] = i;
            previous[b] = i;
            b = i;
            let score = |v: usize| {
                let p = disks[v];
                let q = disks[next[v]];
                let sum = p.r + q.r;
                let x = (p.x * q.r + q.x * p.r) / sum;
                let y = (p.y * q.r + q.y * p.r) / sum;
                x * x + y * y
            };
            let mut best = score(a);
            let mut cursor = next[b];
            while cursor != b {
                let value = score(cursor);
                if value < best {
                    a = cursor;
                    best = value;
                }
                cursor = next[cursor];
            }
            b = next[a];
            break;
        }
    }
    let enclosing = enclosing_disks(&disks, work)?;
    Ok((disks, enclosing))
}
/// Hierarchical front-chain packing. Sibling disks are tangent and nonoverlapping;
/// enclosing circles use exact support geometry and leaf area is proportional to
/// weight. Use `circle_packing_with_algorithm` for the original shelf layout.
pub fn circle_packing(
    c: &ChartConfig,
    data: &[HierarchyDatum],
    o: &HierarchyOptions,
) -> Result<Scene> {
    circle_packing_with_algorithm(c, data, o, CirclePackingAlgorithm::FrontChain)
}
/// Unprojected, parent-relative coordinates. Keeping these offsets avoids
/// subtracting nearly equal screen coordinates when focusing a tiny subtree.
pub(crate) struct PackingLocal {
    pub(crate) radii: Vec<f64>,
    pub(crate) offsets: Vec<(f64, f64)>,
}
pub(crate) fn packing_local(
    data: &[HierarchyDatum],
    h: &Hierarchy,
    viewport_radius: f64,
    padding: f64,
    algorithm: CirclePackingAlgorithm,
    stable: bool,
) -> Result<PackingLocal> {
    let mut enclosure_work = EnclosureWork::new();
    let mut pack = |gap: f64| -> Result<_> {
        let mut radii = vec![0.; data.len()];
        let mut offsets = vec![(0., 0.); data.len()];
        for &i in h.order.iter().rev() {
            if h.children[i].is_empty() {
                radii[i] = if stable {
                    h.totals[i].sqrt() / h.totals[h.root].sqrt()
                } else {
                    (h.totals[i] / h.totals[h.root]).sqrt()
                };
                continue;
            }
            let mut children: Vec<usize> = h.children[i]
                .iter()
                .copied()
                .filter(|j| radii[*j] > 0.)
                .collect();
            children.sort_by(|a, b| {
                radii[*b]
                    .total_cmp(&radii[*a])
                    .then_with(|| data[*a].id.cmp(&data[*b].id))
            });
            if children.is_empty() {
                continue;
            }
            // FrontChain's tangency formulas and Shelf's area squares are
            // homogeneous. Normalize each local sibling problem before those
            // squares, then restore its parent-relative units. Legacy callers
            // use basis=1 and retain their existing operation sequence.
            let basis = if stable {
                radii[children[0]].max(gap)
            } else {
                1.
            };
            let local_gap = gap / basis;
            if algorithm == CirclePackingAlgorithm::FrontChain {
                let padded: Vec<_> = children
                    .iter()
                    .map(|j| radii[*j] / basis + local_gap / 2.)
                    .collect();
                let (disks, enclosing) = front_chain(&padded, &mut enclosure_work)?;
                for (&j, disk) in children.iter().zip(disks) {
                    offsets[j] = (
                        (disk.x - enclosing.x) * basis,
                        (disk.y - enclosing.y) * basis,
                    );
                }
                radii[i] = (enclosing.r + local_gap / 2.) * basis;
                continue;
            }
            let area: f64 = children.iter().map(|j| (radii[*j] / basis).powi(2)).sum();
            let target = (area.sqrt() * 2.).max(radii[children[0]] / basis * 2.);
            let (mut x, mut y, mut row_height, mut max_width) = (0f64, 0f64, 0f64, 0f64);
            for &j in &children {
                let radius = radii[j] / basis;
                let diameter = radius * 2.;
                if x > 0. && x + diameter > target {
                    y += row_height + local_gap;
                    x = 0.;
                    row_height = 0.;
                }
                offsets[j] = (x + radius, y + radius);
                x += diameter + local_gap;
                row_height = row_height.max(diameter);
                max_width = max_width.max(x - local_gap);
            }
            let midx = max_width / 2.;
            let midy = (y + row_height) / 2.;
            let mut enclosing = 0f64;
            for &j in &children {
                offsets[j].0 -= midx;
                offsets[j].1 -= midy;
                enclosing = enclosing.max(offsets[j].0.hypot(offsets[j].1) + radii[j] / basis);
                offsets[j].0 *= basis;
                offsets[j].1 *= basis;
            }
            radii[i] = (enclosing + local_gap / 2.) * basis;
        }
        ensure(
            radii.iter().all(|r| r.is_finite() && *r >= 0.)
                && offsets.iter().all(|(x, y)| x.is_finite() && y.is_finite()),
            "circle packing local geometry overflow",
        )?;
        Ok((radii, offsets))
    };
    let (first, _) = pack(0.)?;
    let initial_scale = viewport_radius / first[h.root];
    let (radii, offsets) = pack(padding / initial_scale)?;
    Ok(PackingLocal { radii, offsets })
}
pub fn circle_packing_with_algorithm(
    c: &ChartConfig,
    data: &[HierarchyDatum],
    o: &HierarchyOptions,
    algorithm: CirclePackingAlgorithm,
) -> Result<Scene> {
    let mut s = c.scene()?;
    hierarchy_options(o)?;
    let Some(h) = hierarchy(data)? else {
        return finish(s);
    };
    ensure(
        data.len() <= 2_000,
        "circle packing exceeds 2000 node budget",
    )?;
    if h.totals[h.root] == 0. {
        return finish(s);
    }
    let (cx, cy, r) = center(c);
    let PackingLocal { radii, offsets } = packing_local(data, &h, r, o.padding, algorithm, false)?;
    let scale = r / radii[h.root];
    let mut positions = vec![(cx, cy); data.len()];
    for &i in &h.order {
        if let Some(p) = h.parent[i] {
            positions[i] = (
                positions[p].0 + offsets[i].0 * scale,
                positions[p].1 + offsets[i].1 * scale,
            );
        }
        if (!o.include_root && i == h.root) || radii[i] == 0. {
            continue;
        }
        let (x, y) = positions[i];
        let mut m = node_mark(
            data,
            &h,
            i,
            Geometry::Circle {
                x,
                y,
                radius: radii[i] * scale,
            },
            c,
        );
        if !h.children[i].is_empty() {
            m.opacity = 0.12;
            m.stroke = Some(GraphicStroke {
                color: c.theme.grid,
                width: 1.,
                cap: StrokeCap::Butt,
            });
        }
        s.marks.push(m);
        if o.labels && h.children[i].is_empty() && radii[i] * scale > c.theme.font_size * 2. {
            owned_label(&mut s, c, x, y, &data[i].label, &data[i].id);
        }
    }
    finish(s)
}
fn finish(scene: Scene) -> Result<Scene> {
    scene.validate()?;
    Ok(scene)
}
