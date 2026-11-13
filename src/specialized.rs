//! Additional deterministic layouts. Design references: local Nivo network/src/hooks.ts
//! (link, charge and centering forces), chord/src/compute.ts, geo/src/hooks.js,
//! boxplot/src and parallel-coordinates/src. Algorithms are independent Rust implementations.
use std::collections::{BTreeMap, BTreeSet};
use std::f64::consts::{PI, TAU};

use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as C, StrokeCap};

use crate::scales::LinearScale;
use crate::scene::*;

fn stroke(color: crate::Color, width: f64) -> GraphicStroke {
    GraphicStroke {
        color,
        width,
        cap: StrokeCap::Round,
    }
}
fn path(points: &[(f64, f64)], close: bool) -> Geometry {
    let mut commands = Vec::with_capacity(points.len() + 1);
    if let Some(&(x, y)) = points.first() {
        commands.push(C::Move(P::new(x, y)));
        commands.extend(points[1..].iter().map(|&(x, y)| C::Line(P::new(x, y))));
        if close {
            commands.push(C::Close);
        }
    }
    Geometry::Path(commands)
}
fn text(
    s: &mut Scene,
    c: &ChartConfig,
    x: f64,
    y: f64,
    value: impl Into<String>,
    anchor: TextAnchor,
) {
    s.labels.push(Label {
        rotation: 0.,
        x,
        y,
        text: value.into(),
        color: c.theme.text,
        size: c.theme.font_size,
        anchor,
    });
}
fn keys<'a>(values: impl IntoIterator<Item = &'a str>) -> Result<()> {
    let mut found = BTreeSet::new();
    for value in values {
        ensure(
            !value.trim().is_empty() && value.len() <= 512 && found.insert(value),
            "invalid or duplicate ID",
        )?;
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq)]
pub struct NetworkNode {
    pub id: String,
    pub label: String,
    pub radius: f64,
}
impl NetworkNode {
    pub fn new(id: impl Into<String>) -> Self {
        let id = id.into();
        Self {
            label: id.clone(),
            id,
            radius: 8.,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct NetworkLink {
    pub source: String,
    pub target: String,
    pub value: f64,
}
impl NetworkLink {
    pub fn new(source: impl Into<String>, target: impl Into<String>, value: f64) -> Self {
        Self {
            source: source.into(),
            target: target.into(),
            value,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct NetworkOptions {
    pub iterations: usize,
    pub repulsion: f64,
    pub link_distance: f64,
    pub labels: bool,
}
impl Default for NetworkOptions {
    fn default() -> Self {
        Self {
            iterations: 160,
            repulsion: 240.,
            link_distance: 70.,
            labels: true,
        }
    }
}
/// Bounded O(iterations*n²) force simulation with deterministic golden-angle seeds.
pub fn network(
    c: &ChartConfig,
    nodes: &[NetworkNode],
    links: &[NetworkLink],
    o: &NetworkOptions,
) -> Result<Scene> {
    let mut s = c.scene()?;
    ensure(
        nodes.len() <= 512 && links.len() <= 4096 && o.iterations <= 1000,
        "network simulation budget exceeded",
    )?;
    ensure(
        o.repulsion.is_finite()
            && (0. ..=100000.).contains(&o.repulsion)
            && o.link_distance.is_finite()
            && (1. ..=10000.).contains(&o.link_distance),
        "invalid force options",
    )?;
    keys(nodes.iter().map(|n| n.id.as_str()))?;
    let index: BTreeMap<_, _> = nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.id.as_str(), i))
        .collect();
    for n in nodes {
        ensure(
            n.radius.is_finite()
                && n.radius > 0.
                && n.radius <= c.inner_width().min(c.inner_height()) / 2.
                && n.label.len() <= 1024,
            "invalid network node",
        )?;
    }
    let mut edges = Vec::with_capacity(links.len());
    for l in links {
        ensure(l.value.is_finite() && l.value >= 0., "invalid network link")?;
        let a = index
            .get(l.source.as_str())
            .ok_or_else(|| ChartError("unknown link source".into()))?;
        let b = index
            .get(l.target.as_str())
            .ok_or_else(|| ChartError("unknown link target".into()))?;
        edges.push((*a, *b));
    }
    let w = c.inner_width();
    let h = c.inner_height();
    let mut positions: Vec<_> = nodes
        .iter()
        .enumerate()
        .map(|(i, node)| {
            let a = i as f64 * 2.399963229728653;
            let r = (i as f64 + 0.5).sqrt() * 5.;
            (
                (w / 2. + r * a.cos()).clamp(node.radius, w - node.radius),
                (h / 2. + r * a.sin()).clamp(node.radius, h - node.radius),
            )
        })
        .collect();
    let mut velocities = vec![(0., 0.); nodes.len()];
    for iteration in 0..o.iterations {
        let alpha = 1. - iteration as f64 / o.iterations.max(1) as f64;
        for a in 0..nodes.len() {
            for b in a + 1..nodes.len() {
                let dx = positions[b].0 - positions[a].0;
                let dy = positions[b].1 - positions[a].1;
                let distance = dx.hypot(dy).max(0.01);
                let overlap = (nodes[a].radius + nodes[b].radius + 3. - distance).max(0.);
                let force = (o.repulsion / distance.powi(2) + overlap * 0.15) * alpha;
                let (fx, fy) = (force * dx / distance, force * dy / distance);
                velocities[a].0 -= fx;
                velocities[a].1 -= fy;
                velocities[b].0 += fx;
                velocities[b].1 += fy;
            }
        }
        for &(a, b) in &edges {
            if a == b {
                continue;
            }
            let dx = positions[b].0 - positions[a].0;
            let dy = positions[b].1 - positions[a].1;
            let distance = dx.hypot(dy).max(0.01);
            let f = (distance - o.link_distance) * 0.025 * alpha;
            let (fx, fy) = (dx / distance * f, dy / distance * f);
            velocities[a].0 += fx;
            velocities[a].1 += fy;
            velocities[b].0 -= fx;
            velocities[b].1 -= fy;
        }
        for (i, ((x, y), (vx, vy))) in positions.iter_mut().zip(&mut velocities).enumerate() {
            *vx = (*vx + (w / 2. - *x) * 0.006 * alpha) * 0.75;
            *vy = (*vy + (h / 2. - *y) * 0.006 * alpha) * 0.75;
            *x = (*x + *vx).clamp(nodes[i].radius, w - nodes[i].radius);
            *y = (*y + *vy).clamp(nodes[i].radius, h - nodes[i].radius);
        }
    }
    for (i, &(a, b)) in edges.iter().enumerate() {
        let (x, y) = positions[a];
        let (xx, yy) = positions[b];
        let mut m = Mark::new(
            format!("link:{i}"),
            path(
                &[
                    (x + c.margin.left, y + c.margin.top),
                    (xx + c.margin.left, yy + c.margin.top),
                ],
                false,
            ),
            c.theme.grid,
        );
        m.fill = None;
        m.stroke = Some(stroke(c.theme.grid, links[i].value.sqrt().clamp(1., 12.)));
        m.label = format!("{} → {}", links[i].source, links[i].target);
        m.value = links[i].value;
        s.marks.push(m);
    }
    for (i, n) in nodes.iter().enumerate() {
        let (x, y) = (
            positions[i].0 + c.margin.left,
            positions[i].1 + c.margin.top,
        );
        let mut m = Mark::new(
            format!("node:{}", n.id),
            Geometry::Circle {
                x,
                y,
                radius: n.radius,
            },
            c.theme.color(i),
        );
        m.series = n.id.clone();
        m.label = n.label.clone();
        s.marks.push(m);
        if o.labels {
            s.label_series.insert(s.labels.len(), n.id.clone());
            text(
                &mut s,
                c,
                x,
                y - n.radius - 5.,
                &n.label,
                TextAnchor::Middle,
            );
        }
    }
    s.validate()?;
    Ok(s)
}

#[derive(Clone, Debug, PartialEq)]
pub struct BoxSample {
    pub id: String,
    pub values: Vec<f64>,
}
impl BoxSample {
    pub fn new(id: impl Into<String>, values: Vec<f64>) -> Self {
        Self {
            id: id.into(),
            values,
        }
    }
}
/// Tukey boxes: R-7 interpolated quartiles, 1.5 IQR whiskers and explicit outliers.
pub fn boxplot(c: &ChartConfig, data: &[BoxSample]) -> Result<Scene> {
    let mut s = c.scene()?;
    ensure(
        data.len() <= 256 && data.iter().map(|d| d.values.len()).sum::<usize>() <= 10000,
        "boxplot budget exceeded",
    )?;
    keys(data.iter().map(|d| d.id.as_str()))?;
    ensure(
        data.iter().flat_map(|d| &d.values).all(|v| v.is_finite()),
        "boxplot values must be finite",
    )?;
    let mut extent = (f64::INFINITY, f64::NEG_INFINITY);
    for v in data.iter().flat_map(|d| &d.values) {
        extent.0 = extent.0.min(*v);
        extent.1 = extent.1.max(*v);
    }
    if extent.0 == f64::INFINITY {
        return Ok(s);
    }
    let y = LinearScale::new(
        [extent.0, extent.1],
        [c.height - c.margin.bottom, c.margin.top],
    )?;
    let bw = c.inner_width() / data.len().max(1) as f64;
    for (i, d) in data.iter().enumerate() {
        if d.values.is_empty() {
            continue;
        }
        let mut v = d.values.clone();
        v.sort_by(f64::total_cmp);
        let q = |p: f64| {
            let z = (v.len() - 1) as f64 * p;
            let a = z.floor() as usize;
            v[a] * (1. - z.fract()) + v[z.ceil() as usize] * z.fract()
        };
        let (q1, med, q3) = (q(0.25), q(0.5), q(0.75));
        let iqr = q3 - q1;
        let lo = *v.iter().find(|n| **n >= q1 - 1.5 * iqr).unwrap();
        let hi = *v.iter().rev().find(|n| **n <= q3 + 1.5 * iqr).unwrap();
        let x = c.margin.left + (i as f64 + 0.5) * bw;
        let mut whisker = Mark::new(
            format!("whisker:{}", d.id),
            Geometry::Path(vec![
                C::Move(P::new(x, y.map(lo))),
                C::Line(P::new(x, y.map(hi))),
                C::Move(P::new(x - bw * 0.15, y.map(lo))),
                C::Line(P::new(x + bw * 0.15, y.map(lo))),
                C::Move(P::new(x - bw * 0.15, y.map(hi))),
                C::Line(P::new(x + bw * 0.15, y.map(hi))),
            ]),
            c.theme.text,
        );
        whisker.fill = None;
        whisker.stroke = Some(stroke(c.theme.text, 1.));
        whisker.series = d.id.clone();
        s.marks.push(whisker);
        let mut boxmark = Mark::new(
            format!("box:{}", d.id),
            Geometry::Rect {
                x: x - bw * 0.28,
                y: y.map(q3),
                width: bw * 0.56,
                height: (y.map(q1) - y.map(q3)).max(0.),
            },
            c.theme.color(i),
        );
        boxmark.series = d.id.clone();
        boxmark.value = med;
        boxmark.label = format!("{}: Q1 {q1}, median {med}, Q3 {q3}", d.id);
        s.marks.push(boxmark);
        let mut median = Mark::new(
            format!("median:{}", d.id),
            path(
                &[(x - bw * 0.28, y.map(med)), (x + bw * 0.28, y.map(med))],
                false,
            ),
            c.theme.text,
        );
        median.fill = None;
        median.stroke = Some(stroke(c.theme.text, 2.));
        median.series = d.id.clone();
        median.value = med;
        s.marks.push(median);
        for (j, value) in v
            .iter()
            .enumerate()
            .filter(|(_, value)| **value < lo || **value > hi)
        {
            let mut m = Mark::new(
                format!("outlier:{}:{j}", d.id),
                Geometry::Circle {
                    x,
                    y: y.map(*value),
                    radius: 3.,
                },
                c.theme.color(i),
            );
            m.series = d.id.clone();
            m.value = *value;
            s.marks.push(m);
        }
        s.label_series.insert(s.labels.len(), d.id.clone());
        text(
            &mut s,
            c,
            x,
            c.height - c.margin.bottom + 20.,
            &d.id,
            TextAnchor::Middle,
        );
    }
    s.validate()?;
    Ok(s)
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParallelRecord {
    pub id: String,
    pub values: Vec<Option<f64>>,
}
/// Missing dimensions interrupt a record; each axis has its own finite domain.
pub fn parallel_coordinates(
    c: &ChartConfig,
    dimensions: &[String],
    records: &[ParallelRecord],
) -> Result<Scene> {
    let mut s = c.scene()?;
    ensure(
        (2..=64).contains(&dimensions.len()) && records.len() <= 2048,
        "parallel coordinates budget exceeded",
    )?;
    keys(dimensions.iter().map(String::as_str))?;
    keys(records.iter().map(|r| r.id.as_str()))?;
    let mut domains = vec![(f64::INFINITY, f64::NEG_INFINITY); dimensions.len()];
    for r in records {
        ensure(
            r.values.len() == dimensions.len(),
            "record dimension count mismatch",
        )?;
        for (i, v) in r.values.iter().enumerate() {
            if let Some(v) = v {
                ensure(v.is_finite(), "nonfinite parallel value")?;
                domains[i].0 = domains[i].0.min(*v);
                domains[i].1 = domains[i].1.max(*v);
            }
        }
    }
    let mut scales = Vec::new();
    for (i, d) in domains.iter().enumerate() {
        let range = if d.0 == f64::INFINITY {
            [0., 1.]
        } else {
            [d.0, d.1]
        };
        let scale = LinearScale::new(range, [c.height - c.margin.bottom, c.margin.top])?;
        let x = c.margin.left + c.inner_width() * i as f64 / (dimensions.len() - 1) as f64;
        let mut axis = Mark::new(
            format!("axis:{i}"),
            path(&[(x, c.margin.top), (x, c.height - c.margin.bottom)], false),
            c.theme.grid,
        );
        axis.fill = None;
        axis.stroke = Some(stroke(c.theme.grid, 1.));
        s.marks.push(axis);
        text(
            &mut s,
            c,
            x,
            c.height - c.margin.bottom + 24.,
            &dimensions[i],
            TextAnchor::Middle,
        );
        for value in scale.ticks(4) {
            text(
                &mut s,
                c,
                x - 5.,
                scale.map(value),
                format!("{value:.1}"),
                TextAnchor::End,
            );
        }
        scales.push(scale);
    }
    for (i, r) in records.iter().enumerate() {
        let mut commands = Vec::new();
        let mut open = false;
        for (j, v) in r.values.iter().enumerate() {
            if let Some(v) = v {
                let p = P::new(
                    c.margin.left + c.inner_width() * j as f64 / (dimensions.len() - 1) as f64,
                    scales[j].map(*v),
                );
                commands.push(if open { C::Line(p) } else { C::Move(p) });
                open = true;
            } else {
                open = false;
            }
        }
        let mut m = Mark::new(
            format!("record:{}", r.id),
            Geometry::Path(commands),
            c.theme.color(i),
        );
        m.fill = None;
        m.stroke = Some(stroke(c.theme.color(i), 1.5));
        m.opacity = 0.7;
        m.series = r.id.clone();
        m.label = r.id.clone();
        s.marks.push(m);
    }
    s.validate()?;
    Ok(s)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Projection {
    #[default]
    Equirectangular,
    Mercator,
}
#[derive(Clone, Debug, PartialEq)]
pub struct GeoFeature {
    pub id: String,
    pub label: String,
    pub value: Option<f64>,
    pub polygons: Vec<Vec<Vec<[f64; 2]>>>,
}
/// Polygon/multipolygon choropleth, with explicitly typed longitude/latitude rings.
/// Coordinates must stay on one side of the antimeridian; no spherical clipping.
pub fn choropleth(
    c: &ChartConfig,
    features: &[GeoFeature],
    projection: Projection,
) -> Result<Scene> {
    let mut s = c.scene()?;
    ensure(features.len() <= 4096, "geo feature budget exceeded")?;
    keys(features.iter().map(|f| f.id.as_str()))?;
    let count: usize = features
        .iter()
        .flat_map(|f| &f.polygons)
        .flat_map(|p| p.iter())
        .map(Vec::len)
        .sum();
    ensure(count <= 100000, "geo coordinate budget exceeded")?;
    let project = |p: [f64; 2]| -> Result<(f64, f64)> {
        let [lon, lat] = p;
        ensure(
            lon.is_finite()
                && lat.is_finite()
                && (-180. ..=180.).contains(&lon)
                && (-90. ..=90.).contains(&lat),
            "invalid longitude/latitude",
        )?;
        let xx = (lon + 180.) / 360.;
        let yy = match projection {
            Projection::Equirectangular => (90. - lat) / 180.,
            Projection::Mercator => {
                let phi = lat.clamp(-85.05112878, 85.05112878).to_radians();
                (1. - ((PI / 4. + phi / 2.).tan()).ln() / PI) / 2.
            }
        };
        Ok((
            c.margin.left + xx * c.inner_width(),
            c.margin.top + yy * c.inner_height(),
        ))
    };
    let mut extent = (f64::INFINITY, f64::NEG_INFINITY);
    for f in features {
        ensure(
            f.label.len() <= 1024 && f.value.is_none_or(f64::is_finite),
            "invalid geo value/label",
        )?;
        if let Some(v) = f.value {
            extent.0 = extent.0.min(v);
            extent.1 = extent.1.max(v);
        }
    }
    ensure(
        extent.0 == f64::INFINITY || (extent.1 - extent.0).is_finite(),
        "geo color domain span overflow",
    )?;
    for f in features {
        let mut commands = vec![];
        for polygon in &f.polygons {
            for (ri, ring) in polygon.iter().enumerate() {
                ensure(ring.len() >= 3, "geo rings need at least three points")?;
                ensure(
                    (0..ring.len())
                        .all(|i| (ring[i][0] - ring[(i + 1) % ring.len()][0]).abs() <= 180.),
                    "antimeridian crossing requires spherical clipping",
                )?;
                let mut points = ring
                    .iter()
                    .map(|p| project(*p))
                    .collect::<Result<Vec<_>>>()?;
                // Canonicalize opposite hole winding for the shared nonzero fill rule.
                let area: f64 = (0..points.len())
                    .map(|i| {
                        let a = points[i];
                        let b = points[(i + 1) % points.len()];
                        a.0 * b.1 - b.0 * a.1
                    })
                    .sum();
                if (ri == 0 && area < 0.) || (ri > 0 && area > 0.) {
                    points.reverse();
                }
                if let Geometry::Path(p) = path(&points, true) {
                    commands.extend(p);
                }
            }
        }
        let color = if let Some(value) = f.value {
            let t = if extent.0 == extent.1 {
                0.5
            } else {
                ((value - extent.0) / (extent.1 - extent.0)).clamp(0., 1.)
            };
            crate::Color(
                (230. - 190. * t) as u8,
                (240. - 120. * t) as u8,
                (250. - 60. * t) as u8,
                255,
            )
        } else {
            c.theme.grid
        };
        let mut m = Mark::new(format!("geo:{}", f.id), Geometry::Path(commands), color);
        m.series = f.id.clone();
        m.label = f.label.clone();
        m.value = f.value.unwrap_or(0.);
        m.stroke = Some(stroke(c.theme.background, 0.75));
        s.marks.push(m);
    }
    s.validate()?;
    Ok(s)
}

#[derive(Clone, Debug, PartialEq)]
pub struct BulletDatum {
    pub id: String,
    pub ranges: Vec<f64>,
    pub measures: Vec<f64>,
    pub markers: Vec<f64>,
}
pub fn bullet(c: &ChartConfig, data: &[BulletDatum]) -> Result<Scene> {
    let mut s = c.scene()?;
    ensure(
        data.len() <= 256
            && data
                .iter()
                .all(|d| d.ranges.len() <= 32 && d.measures.len() <= 32 && d.markers.len() <= 32),
        "bullet budget exceeded",
    )?;
    keys(data.iter().map(|d| d.id.as_str()))?;
    let row = c.inner_height() / data.len().max(1) as f64;
    for (i, d) in data.iter().enumerate() {
        ensure(
            d.ranges
                .iter()
                .chain(&d.measures)
                .chain(&d.markers)
                .all(|v| v.is_finite() && *v >= 0.),
            "bullet values must be finite and nonnegative",
        )?;
        let max = d
            .ranges
            .iter()
            .chain(&d.measures)
            .chain(&d.markers)
            .copied()
            .fold(0., f64::max);
        let max = if max > 0. { max } else { 1. };
        let scale = LinearScale::new([0., max], [0., c.inner_width()])?;
        let y = c.margin.top + i as f64 * row;
        let mut ranges = d.ranges.clone();
        ranges.sort_by(|a, b| b.total_cmp(a));
        for (j, v) in ranges.iter().enumerate() {
            let shade = (210usize.saturating_sub(j * 18)).max(50) as u8;
            let mut m = Mark::new(
                format!("range:{}:{j}", d.id),
                Geometry::Rect {
                    x: c.margin.left,
                    y: y + row * 0.1,
                    width: scale.map(*v),
                    height: row * 0.8,
                },
                crate::Color(shade, shade, shade, 255),
            );
            m.series = d.id.clone();
            m.value = *v;
            s.marks.push(m);
        }
        for (j, v) in d.measures.iter().enumerate() {
            let height = row * 0.4 / (j + 1) as f64;
            let mut m = Mark::new(
                format!("measure:{}:{j}", d.id),
                Geometry::Rect {
                    x: c.margin.left,
                    y: y + (row - height) / 2.,
                    width: scale.map(*v),
                    height,
                },
                c.theme.color(j),
            );
            m.series = d.id.clone();
            m.value = *v;
            s.marks.push(m);
        }
        for (j, v) in d.markers.iter().enumerate() {
            let x = c.margin.left + scale.map(*v);
            let mut m = Mark::new(
                format!("marker:{}:{j}", d.id),
                path(&[(x, y + row * 0.2), (x, y + row * 0.8)], false),
                c.theme.text,
            );
            m.fill = None;
            m.stroke = Some(stroke(c.theme.text, 2.));
            m.value = *v;
            m.series = d.id.clone();
            s.marks.push(m);
        }
        s.label_series.insert(s.labels.len(), d.id.clone());
        text(
            &mut s,
            c,
            c.margin.left - 8.,
            y + row / 2.,
            &d.id,
            TextAnchor::End,
        );
    }
    s.validate()?;
    Ok(s)
}

fn polar(cx: f64, cy: f64, r: f64, a: f64) -> P {
    P::new(cx + r * a.sin(), cy - r * a.cos())
}
fn arc(out: &mut Vec<C>, cx: f64, cy: f64, r: f64, a: f64, b: f64) {
    let n = ((b - a).abs() / (PI / 2.)).ceil().max(1.) as usize;
    for i in 0..n {
        let aa = a + (b - a) * i as f64 / n as f64;
        let bb = a + (b - a) * (i + 1) as f64 / n as f64;
        let k = 4. / 3. * ((bb - aa) / 4.).tan();
        let p = polar(cx, cy, r, aa);
        let q = polar(cx, cy, r, bb);
        out.push(C::Cubic {
            first: P::new(p.x + k * r * aa.cos(), p.y + k * r * aa.sin()),
            second: P::new(q.x - k * r * bb.cos(), q.y - k * r * bb.sin()),
            end: q,
        });
    }
}
/// Undirected chord ribbons pair `matrix[i][j]` and `matrix[j][i]`; each row
/// controls one group's angular total. Zero rows and an all-zero matrix are valid.
pub fn chord(
    c: &ChartConfig,
    ids: &[String],
    matrix: &[Vec<f64>],
    pad_angle: f64,
) -> Result<Scene> {
    let mut s = c.scene()?;
    ensure(
        ids.len() <= 64 && matrix.len() == ids.len() && matrix.iter().all(|r| r.len() == ids.len()),
        "chord needs a square matrix, at most 64 groups",
    )?;
    keys(ids.iter().map(String::as_str))?;
    ensure(
        pad_angle.is_finite() && pad_angle >= 0. && pad_angle * (ids.len() as f64) < TAU,
        "invalid chord padding",
    )?;
    ensure(
        matrix.iter().flatten().all(|v| v.is_finite() && *v >= 0.),
        "chord values must be finite and nonnegative",
    )?;
    let total: f64 = matrix.iter().flatten().sum();
    ensure(total.is_finite(), "chord total overflow")?;
    if total == 0. {
        return Ok(s);
    }
    let available_angle = TAU - pad_angle * ids.len() as f64;
    let cx = c.margin.left + c.inner_width() / 2.;
    let cy = c.margin.top + c.inner_height() / 2.;
    let outer = c.inner_width().min(c.inner_height()) / 2.;
    let r = outer * 0.86;
    let mut intervals = vec![vec![(0., 0.); ids.len()]; ids.len()];
    let mut groups = Vec::new();
    let mut angle = 0.;
    for (i, row) in matrix.iter().enumerate() {
        let start = angle;
        for (j, value) in row.iter().enumerate() {
            let end = angle + (value / total) * available_angle;
            intervals[i][j] = (angle, end);
            angle = end;
        }
        groups.push((start, angle));
        angle += pad_angle;
    }
    for i in 0..ids.len() {
        for j in i..ids.len() {
            if matrix[i][j] + matrix[j][i] == 0. {
                continue;
            }
            let (a, b) = intervals[i][j];
            let (cc, d) = intervals[j][i];
            let mut commands = vec![C::Move(polar(cx, cy, r, a))];
            arc(&mut commands, cx, cy, r, a, b);
            commands.push(C::Cubic {
                first: P::new(cx, cy),
                second: P::new(cx, cy),
                end: polar(cx, cy, r, cc),
            });
            arc(&mut commands, cx, cy, r, cc, d);
            commands.push(C::Cubic {
                first: P::new(cx, cy),
                second: P::new(cx, cy),
                end: polar(cx, cy, r, a),
            });
            commands.push(C::Close);
            let mut m = Mark::new(
                format!("ribbon:{i}:{j}"),
                Geometry::Path(commands),
                c.theme.color(i),
            );
            m.opacity = 0.65;
            m.value = if i == j {
                matrix[i][j]
            } else {
                matrix[i][j] + matrix[j][i]
            };
            m.label = format!("{} ↔ {}", ids[i], ids[j]);
            m.series = ids[i].clone();
            s.marks.push(m);
        }
    }
    for (i, &(a, b)) in groups.iter().enumerate() {
        if a == b {
            continue;
        }
        let mut commands = vec![C::Move(polar(cx, cy, outer, a))];
        arc(&mut commands, cx, cy, outer, a, b);
        commands.push(C::Line(polar(cx, cy, r, b)));
        arc(&mut commands, cx, cy, r, b, a);
        commands.push(C::Close);
        let mut m = Mark::new(
            format!("group:{i}"),
            Geometry::Path(commands),
            c.theme.color(i),
        );
        m.value = matrix[i].iter().sum();
        m.label = ids[i].clone();
        m.series = ids[i].clone();
        s.marks.push(m);
        let p = polar(cx, cy, (outer + r) / 2., (a + b) / 2.);
        s.label_series.insert(s.labels.len(), ids[i].clone());
        text(&mut s, c, p.x, p.y, &ids[i], TextAnchor::Middle);
        s.legend.push(LegendItem {
            id: ids[i].clone(),
            label: ids[i].clone(),
            color: c.theme.color(i),
        });
    }
    s.validate()?;
    Ok(s)
}

#[derive(Clone, Debug, PartialEq)]
pub struct CalendarDay {
    pub month: u8,
    pub day: u8,
    pub value: f64,
}
/// A single Gregorian year, Monday-first weeks. Duplicate/invalid dates fail
/// before geometry generation; absent days remain visible with the grid color.
pub fn calendar(c: &ChartConfig, year: i32, data: &[CalendarDay]) -> Result<Scene> {
    let mut s = c.scene()?;
    ensure(
        (1..=9999).contains(&year) && data.len() <= 366,
        "invalid calendar year or budget",
    )?;
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let lengths = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut values = BTreeMap::new();
    for d in data {
        ensure(
            d.month >= 1
                && d.month <= 12
                && d.day >= 1
                && d.day <= lengths[(d.month - 1) as usize]
                && d.value.is_finite(),
            "invalid calendar day/value",
        )?;
        ensure(
            values.insert((d.month, d.day), d.value).is_none(),
            "duplicate calendar date",
        )?;
    }
    let prev = year - 1;
    let offset = (365 * prev + prev / 4 - prev / 100 + prev / 400) % 7; // 0001-01-01 was Monday.
    let days = if leap { 366 } else { 365 };
    let columns = (offset + days + 6) / 7;
    let cw = c.inner_width() / columns as f64;
    let ch = c.inner_height() / 7.;
    let max = data.iter().map(|d| d.value.abs()).fold(1., f64::max);
    let mut ordinal = 0;
    for (month, &length) in lengths.iter().enumerate() {
        for day in 1..=length {
            let pos = offset + ordinal;
            let value = values.get(&((month + 1) as u8, day));
            let color = if let Some(v) = value {
                let t = (*v / max).clamp(-1., 1.);
                if t >= 0. {
                    crate::Color(
                        (230. - 170. * t) as u8,
                        (245. - 100. * t) as u8,
                        (235. - 150. * t) as u8,
                        255,
                    )
                } else {
                    crate::Color(230, (235. + 150. * t) as u8, (230. + 130. * t) as u8, 255)
                }
            } else {
                c.theme.grid
            };
            let x = c.margin.left + (pos / 7) as f64 * cw;
            let y = c.margin.top + (pos % 7) as f64 * ch;
            let mut m = Mark::new(
                format!("{year:04}-{:02}-{day:02}", month + 1),
                Geometry::Rect {
                    x: x + cw * 0.08,
                    y: y + ch * 0.08,
                    width: cw * 0.84,
                    height: ch * 0.84,
                },
                color,
            );
            m.value = value.copied().unwrap_or(0.);
            m.label = if let Some(v) = value {
                format!("{}: {v}", m.id)
            } else {
                format!("{}: no data", m.id)
            };
            m.series = format!("month:{}", month + 1);
            s.marks.push(m);
            if day == 1 {
                text(
                    &mut s,
                    c,
                    x,
                    c.margin.top - 10.,
                    format!("{}", month + 1),
                    TextAnchor::Start,
                );
            }
            ordinal += 1;
        }
    }
    for (i, day) in ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]
        .iter()
        .enumerate()
    {
        text(
            &mut s,
            c,
            c.margin.left - 6.,
            c.margin.top + (i as f64 + 0.65) * ch,
            *day,
            TextAnchor::End,
        );
    }
    s.validate()?;
    Ok(s)
}
