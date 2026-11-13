//! Variable-width, collision-packed and polar chart layouts.
//!
//! Source comparisons: nivo/packages/marimekko/src/hooks.ts (thickness and
//! stack scales), swarmplot/src/compute.ts (value/group scales and collision
//! forces), polar-bar/src/hooks.ts (angular bands and stacked radial scales).
//! Beeswarm packing here solves tangent intervals directly and deterministically,
//! rather than running D3's iterative force simulation.
use std::collections::{BTreeMap, BTreeSet};
use std::f64::consts::{FRAC_PI_2, TAU};

use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as Cmd, StrokeCap};

use crate::scales::LinearScale;
use crate::scene::{MAX_MARKS, ensure};
use crate::{
    BarLayout, BarSeries, ChartConfig, ChartError, Geometry, Label, LegendItem, Mark, Result,
    Scene, TextAnchor,
};

fn key(id: &str) -> Result<()> {
    ensure(
        !id.trim().is_empty() && id.len() <= 256,
        "ids must be nonempty and at most 256 bytes",
    )
}
fn matrix(categories: &[String], series: &[BarSeries]) -> Result<Vec<f64>> {
    ensure(
        categories.len() <= MAX_MARKS
            && series.len() <= 256
            && categories
                .len()
                .checked_mul(series.len())
                .is_some_and(|n| n <= MAX_MARKS - 16),
        "matrix exceeds chart budgets",
    )?;
    let mut seen = BTreeSet::new();
    for c in categories {
        key(c)?;
        ensure(seen.insert(c), "duplicate category id")?;
    }
    seen.clear();
    let mut totals = vec![0.; categories.len()];
    for s in series {
        key(&s.id)?;
        ensure(seen.insert(&s.id), "duplicate series id")?;
        ensure(
            s.values.len() == categories.len(),
            "matrix series must align with categories",
        )?;
        for (i, v) in s.values.iter().enumerate() {
            if let Some(v) = v {
                ensure(
                    v.is_finite() && *v >= 0.,
                    "matrix values must be finite and nonnegative",
                )?;
                totals[i] += v;
                ensure(totals[i].is_finite(), "matrix total overflow")?;
            }
        }
    }
    Ok(totals)
}
fn legends(scene: &mut Scene, c: &ChartConfig, series: &[BarSeries]) {
    for (i, s) in series.iter().enumerate() {
        scene.legend.push(LegendItem {
            id: s.id.clone(),
            label: s.id.clone(),
            color: c.theme.color(i),
        });
    }
}
fn label(c: &ChartConfig, x: f64, y: f64, text: String, anchor: TextAnchor) -> Label {
    Label {
        rotation: 0.,
        x,
        y,
        text,
        color: c.theme.text,
        size: c.theme.font_size,
        anchor,
    }
}
fn finish(scene: Scene) -> Result<Scene> {
    scene.validate()?;
    Ok(scene)
}

#[derive(Clone, Debug, PartialEq)]
pub struct MarimekkoOptions {
    /// Transpose category widths into category heights.
    pub horizontal: bool,
    /// Logical pixels between nonzero categories. Zero preserves area exactly.
    pub padding: f64,
    pub labels: bool,
}
impl Default for MarimekkoOptions {
    fn default() -> Self {
        Self {
            horizontal: false,
            padding: 0.,
            labels: false,
        }
    }
}
/// Category thickness is proportional to its total; each category is normalized
/// to a full-height stack. With zero padding, datum area / plot area equals its
/// value / grand total. Null/zero entries consume no area and make no mark.
pub fn marimekko(
    c: &ChartConfig,
    categories: &[String],
    series: &[BarSeries],
    o: &MarimekkoOptions,
) -> Result<Scene> {
    let mut scene = c.scene()?;
    let totals = matrix(categories, series)?;
    ensure(
        o.padding.is_finite() && (0. ..=64.).contains(&o.padding),
        "Marimekko padding must be in 0..=64",
    )?;
    let grand_total = totals.iter().sum::<f64>();
    ensure(grand_total.is_finite(), "Marimekko grand total overflow")?;
    legends(&mut scene, c, series);
    if grand_total == 0. {
        return finish(scene);
    }
    let nonzero = totals.iter().filter(|v| **v > 0.).count();
    let thickness = if o.horizontal {
        c.inner_height()
    } else {
        c.inner_width()
    };
    let available = thickness - o.padding * nonzero.saturating_sub(1) as f64;
    ensure(available > 0., "Marimekko padding exhausts plot area")?;
    let mut cursor = 0.;
    for (ci, total) in totals.iter().copied().enumerate() {
        if total == 0. {
            continue;
        }
        let size = total / grand_total * available;
        let mut stack = 0.;
        for (si, s) in series.iter().enumerate() {
            let Some(v) = s.values[ci] else {
                continue;
            };
            if v == 0. {
                continue;
            }
            let fraction = v / total;
            let geometry = if o.horizontal {
                Geometry::Rect {
                    x: c.margin.left + stack * c.inner_width(),
                    y: c.margin.top + cursor,
                    width: fraction * c.inner_width(),
                    height: size,
                }
            } else {
                Geometry::Rect {
                    x: c.margin.left + cursor,
                    y: c.height - c.margin.bottom - (stack + fraction) * c.inner_height(),
                    width: size,
                    height: fraction * c.inner_height(),
                }
            };
            let mut m = Mark::new(format!("mekko-{si}-{ci}"), geometry, c.theme.color(si));
            m.series = s.id.clone();
            m.label = format!("{} · {}: {}", categories[ci], s.id, v);
            m.value = v;
            if o.labels
                && let Geometry::Rect {
                    x,
                    y,
                    width,
                    height,
                } = m.geometry
                && width > c.theme.font_size * 2.
                && height > c.theme.font_size * 1.3
            {
                scene.labels.push(label(
                    c,
                    x + width / 2.,
                    y + height / 2. + c.theme.font_size * 0.35,
                    format!("{v}"),
                    TextAnchor::Middle,
                ));
            }
            scene.marks.push(m);
            stack += fraction;
        }
        if nonzero <= 40 {
            scene.labels.push(if o.horizontal {
                label(
                    c,
                    c.margin.left - 9.,
                    c.margin.top + cursor + size / 2. + 4.,
                    categories[ci].clone(),
                    TextAnchor::End,
                )
            } else {
                label(
                    c,
                    c.margin.left + cursor + size / 2.,
                    c.height - c.margin.bottom + 22.,
                    categories[ci].clone(),
                    TextAnchor::Middle,
                )
            });
        }
        cursor += size + o.padding;
    }
    finish(scene)
}

#[derive(Clone, Debug, PartialEq)]
pub struct SwarmDatum {
    pub id: String,
    pub group: String,
    pub value: f64,
    pub radius: f64,
}
impl SwarmDatum {
    pub fn new(id: impl Into<String>, group: impl Into<String>, value: f64) -> Self {
        Self {
            id: id.into(),
            group: group.into(),
            value,
            radius: 4.,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct SwarmOptions {
    /// Values run left-to-right when true, bottom-to-top otherwise.
    pub horizontal: bool,
    pub gap: f64,
    pub group_padding: f64,
    pub value_domain: Option<[f64; 2]>,
}
impl Default for SwarmOptions {
    fn default() -> Self {
        Self {
            horizontal: true,
            gap: 1.,
            group_padding: 8.,
            value_domain: None,
        }
    }
}
/// Collision-free deterministic beeswarm. Value coordinates remain exact; only
/// the cross-axis coordinate moves. A typed error rejects a group that cannot
/// fit in its band, rather than returning overlapping or silently dropped nodes.
/// Work is bounded to 2,000 points and 256 groups.
pub fn swarmplot(c: &ChartConfig, data: &[SwarmDatum], o: &SwarmOptions) -> Result<Scene> {
    let mut scene = c.scene()?;
    ensure(
        data.len() <= 2000,
        "swarmplot exceeds 2000 point packing budget",
    )?;
    ensure(
        o.gap.is_finite()
            && (0. ..=64.).contains(&o.gap)
            && o.group_padding.is_finite()
            && (0. ..=256.).contains(&o.group_padding),
        "invalid swarm spacing",
    )?;
    let mut ids = BTreeSet::new();
    let mut groups = BTreeMap::<&str, Vec<&SwarmDatum>>::new();
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    let mut max_radius = 0_f64;
    for d in data {
        key(&d.id)?;
        key(&d.group)?;
        ensure(ids.insert(&d.id), "duplicate swarm datum id")?;
        ensure(
            d.value.is_finite() && d.radius.is_finite() && (0.000001..=256.).contains(&d.radius),
            "invalid swarm value or radius",
        )?;
        groups.entry(&d.group).or_default().push(d);
        min = min.min(d.value);
        max = max.max(d.value);
        max_radius = max_radius.max(d.radius);
    }
    ensure(groups.len() <= 256, "swarmplot exceeds 256 group budget")?;
    if data.is_empty() {
        return finish(scene);
    }
    let domain = o.value_domain.unwrap_or([min, max]);
    ensure(
        domain[0] <= domain[1] && domain.iter().all(|v| v.is_finite()),
        "invalid swarm value domain",
    )?;
    ensure(
        data.iter()
            .all(|d| d.value >= domain[0] && d.value <= domain[1]),
        "swarm datum lies outside value domain",
    )?;
    let primary_extent = if o.horizontal {
        c.inner_width()
    } else {
        c.inner_height()
    };
    ensure(
        primary_extent > 2. * max_radius,
        "swarm circles exceed value-axis space",
    )?;
    let range = if o.horizontal {
        [
            c.margin.left + max_radius,
            c.width - c.margin.right - max_radius,
        ]
    } else {
        [
            c.height - c.margin.bottom - max_radius,
            c.margin.top + max_radius,
        ]
    };
    let scale = LinearScale::new(domain, range)?;
    let cross_extent = if o.horizontal {
        c.inner_height()
    } else {
        c.inner_width()
    };
    let group_size = (cross_extent - o.group_padding * groups.len().saturating_sub(1) as f64)
        / groups.len() as f64;
    ensure(group_size > 0., "swarm group padding exhausts plot area")?;
    for (gi, (group, mut points)) in groups.into_iter().enumerate() {
        points.sort_by(|a, b| a.value.total_cmp(&b.value).then_with(|| a.id.cmp(&b.id)));
        let center = if o.horizontal {
            c.margin.top
        } else {
            c.margin.left
        } + gi as f64 * (group_size + o.group_padding)
            + group_size / 2.;
        let mut placed = Vec::<(f64, f64, f64)>::new();
        for d in points {
            let primary = scale.map(d.value);
            let bound = group_size / 2. - d.radius;
            ensure(bound >= 0., "swarm radius exceeds group band")?;
            let mut intervals = Vec::<(f64, f64)>::new();
            for &(p, q, r) in &placed {
                let distance = (primary - p).abs();
                let separation = d.radius + r + o.gap;
                if distance < separation {
                    let offset = (separation * separation - distance * distance)
                        .max(0.)
                        .sqrt();
                    intervals.push((q - offset, q + offset));
                }
            }
            intervals.sort_by(|a, b| a.0.total_cmp(&b.0));
            let mut merged = Vec::<(f64, f64)>::new();
            for (a, b) in intervals {
                if let Some(last) = merged.last_mut()
                    && a <= last.1 + 1e-9
                {
                    last.1 = last.1.max(b);
                    continue;
                }
                merged.push((a, b));
            }
            let containing = merged.iter().find(|(a, b)| *a < 0. && *b > 0.);
            let cross = if let Some(&(a, b)) = containing {
                let epsilon = 1e-7;
                let candidates = [a - epsilon, b + epsilon];
                candidates
                    .into_iter()
                    .filter(|v| v.abs() <= bound && !merged.iter().any(|(a, b)| *v > *a && *v < *b))
                    .min_by(|a, b| a.abs().total_cmp(&b.abs()).then_with(|| a.total_cmp(b)))
                    .ok_or_else(|| {
                        ChartError(format!(
                            "swarm group '{group}' cannot fit without collisions"
                        ))
                    })?
            } else {
                0.
            };
            let (x, y) = if o.horizontal {
                (primary, center + cross)
            } else {
                (center + cross, primary)
            };
            let mut m = Mark::new(
                format!("swarm-{}", d.id),
                Geometry::Circle {
                    x,
                    y,
                    radius: d.radius,
                },
                c.theme.color(gi),
            );
            m.series = group.into();
            m.label = format!("{} · {}: {}", group, d.id, d.value);
            m.value = d.value;
            scene.marks.push(m);
            placed.push((primary, cross, d.radius));
        }
        scene.legend.push(LegendItem {
            id: group.into(),
            label: group.into(),
            color: c.theme.color(gi),
        });
        scene.labels.push(if o.horizontal {
            label(
                c,
                c.margin.left - 8.,
                center + 4.,
                group.into(),
                TextAnchor::End,
            )
        } else {
            label(
                c,
                center,
                c.height - c.margin.bottom + 22.,
                group.into(),
                TextAnchor::Middle,
            )
        });
    }
    for v in scale.ticks(5) {
        let p = scale.map(v);
        scene.labels.push(if o.horizontal {
            label(
                c,
                p,
                c.height - c.margin.bottom + 22.,
                format!("{v}"),
                TextAnchor::Middle,
            )
        } else {
            label(
                c,
                c.margin.left - 8.,
                p + 4.,
                format!("{v}"),
                TextAnchor::End,
            )
        });
    }
    finish(scene)
}

#[derive(Clone, Debug, PartialEq)]
pub struct PolarBarOptions {
    pub layout: BarLayout,
    /// Inner radius as a fraction of the available outer radius.
    pub inner_radius: f64,
    /// Radians, clockwise from twelve o'clock; span must be at most one turn.
    pub start_angle: f64,
    pub end_angle: f64,
    pub pad_angle: f64,
    /// Use square-root radii so sector area is proportional to value.
    pub area_scale: bool,
    pub labels: bool,
}
impl Default for PolarBarOptions {
    fn default() -> Self {
        Self {
            layout: BarLayout::Stacked,
            inner_radius: 0.15,
            start_angle: 0.,
            end_angle: TAU,
            pad_angle: 0.025,
            area_scale: false,
            labels: true,
        }
    }
}
fn polar(cx: f64, cy: f64, r: f64, a: f64) -> P {
    P::new(cx + r * a.sin(), cy - r * a.cos())
}
fn arc(commands: &mut Vec<Cmd>, cx: f64, cy: f64, r: f64, start: f64, end: f64) {
    let count = ((end - start).abs() / FRAC_PI_2).ceil().max(1.) as usize;
    let delta = (end - start) / count as f64;
    for i in 0..count {
        let a = start + i as f64 * delta;
        let b = a + delta;
        let k = 4. / 3. * (delta / 4.).tan();
        let p = polar(cx, cy, r, a);
        let q = polar(cx, cy, r, b);
        commands.push(Cmd::Cubic {
            first: P::new(p.x + k * r * a.cos(), p.y + k * r * a.sin()),
            second: P::new(q.x - k * r * b.cos(), q.y - k * r * b.sin()),
            end: q,
        });
    }
}
fn annular(cx: f64, cy: f64, inner: f64, outer: f64, start: f64, end: f64) -> Geometry {
    let mut commands = vec![Cmd::Move(polar(cx, cy, outer, start))];
    arc(&mut commands, cx, cy, outer, start, end);
    commands.push(Cmd::Line(polar(cx, cy, inner, end)));
    if inner > 0. {
        arc(&mut commands, cx, cy, inner, end, start);
    }
    commands.push(Cmd::Close);
    Geometry::Path(commands)
}
/// Grouped, stacked or percentage annular bars with explicit linear-radius or
/// area-preserving scaling. Negative values are rejected instead of being folded
/// into a misleading positive radius.
pub fn polar_bar(
    c: &ChartConfig,
    categories: &[String],
    series: &[BarSeries],
    o: &PolarBarOptions,
) -> Result<Scene> {
    let mut scene = c.scene()?;
    let totals = matrix(categories, series)?;
    ensure(
        o.inner_radius.is_finite() && (0. ..1.).contains(&o.inner_radius),
        "polar inner radius must be in 0..1",
    )?;
    ensure(
        [o.start_angle, o.end_angle, o.pad_angle]
            .iter()
            .all(|v| v.is_finite())
            && o.start_angle.abs() <= 100. * TAU
            && o.end_angle > o.start_angle
            && o.end_angle - o.start_angle <= TAU + 1e-12
            && o.pad_angle >= 0.,
        "invalid polar angular range",
    )?;
    legends(&mut scene, c, series);
    if categories.is_empty() || series.is_empty() {
        return finish(scene);
    }
    let maximum = match o.layout {
        BarLayout::Grouped => series
            .iter()
            .flat_map(|s| s.values.iter().flatten())
            .copied()
            .fold(0., f64::max),
        BarLayout::Stacked => totals.iter().copied().fold(0., f64::max),
        BarLayout::Percent => 1.,
    };
    let maximum = if maximum > 0. { maximum } else { 1. };
    let cx = c.margin.left + c.inner_width() / 2.;
    let cy = c.margin.top + c.inner_height() / 2.;
    let outer = c.inner_width().min(c.inner_height()) / 2.;
    let inner = outer * o.inner_radius;
    let radius = |v: f64| {
        let t = (v / maximum).clamp(0., 1.);
        if o.area_scale {
            (inner * inner + (outer * outer - inner * inner) * t).sqrt()
        } else {
            inner + (outer - inner) * t
        }
    };
    let step = (o.end_angle - o.start_angle) / categories.len() as f64;
    let subdivisions = if o.layout == BarLayout::Grouped {
        series.len()
    } else {
        1
    };
    let angle_width = step / subdivisions as f64;
    ensure(
        o.pad_angle < angle_width,
        "polar padding must be smaller than each bar angle",
    )?;
    for i in 1..=4 {
        let r = inner + (outer - inner) * i as f64 / 4.;
        let mut m = Mark::new(
            format!("polar-grid-{i}"),
            Geometry::Circle {
                x: cx,
                y: cy,
                radius: r,
            },
            c.theme.grid,
        );
        m.fill = None;
        m.stroke = Some(GraphicStroke {
            color: c.theme.grid,
            width: 1.,
            cap: StrokeCap::Butt,
        });
        m.label.clear();
        scene.marks.push(m);
    }
    let mut stack = vec![0.; categories.len()];
    for (si, s) in series.iter().enumerate() {
        for (ci, raw) in s.values.iter().enumerate() {
            let Some(raw) = raw else {
                continue;
            };
            if *raw == 0. {
                continue;
            }
            let value = if o.layout == BarLayout::Percent {
                if totals[ci] == 0. {
                    0.
                } else {
                    raw / totals[ci]
                }
            } else {
                *raw
            };
            let base = if o.layout == BarLayout::Grouped {
                0.
            } else {
                stack[ci]
            };
            let end = base + value;
            stack[ci] = end;
            let start = o.start_angle
                + ci as f64 * step
                + if o.layout == BarLayout::Grouped {
                    si as f64 * angle_width
                } else {
                    0.
                }
                + o.pad_angle / 2.;
            let stop = start + angle_width - o.pad_angle;
            let mut m = Mark::new(
                format!("polar-bar-{si}-{ci}"),
                annular(cx, cy, radius(base), radius(end), start, stop),
                c.theme.color(si),
            );
            m.series = s.id.clone();
            m.value = *raw;
            m.label = format!("{} · {}: {}", categories[ci], s.id, raw);
            scene.marks.push(m);
        }
    }
    if o.labels && categories.len() <= 40 {
        for (i, category) in categories.iter().enumerate() {
            let p = polar(cx, cy, outer + 16., o.start_angle + (i as f64 + 0.5) * step);
            scene.labels.push(label(
                c,
                p.x,
                p.y + 4.,
                category.clone(),
                TextAnchor::Middle,
            ));
        }
    }
    finish(scene)
}
