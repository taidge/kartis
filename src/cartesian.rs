//! Shared computed geometry for SVG, Canvas and Glory hosts.
//!
//! References inspected locally: nivo/packages/bar/src/compute/{grouped,stacked}.ts,
//! nivo/packages/line/src/hooks.ts, scatterplot/src/compute.ts and heatmap/src/compute.ts.
//! Rust algorithms are independent implementations; no JavaScript runtime is required.
use std::collections::BTreeSet;

use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as Cmd, StrokeCap};
use glory::style::Color;

use crate::axes::{CartesianAxes, ChartScale, ScaleKind, ValueFormat, cartesian_axes};
use crate::scales::{BandScale, LinearScale};
use crate::scene::*;

#[derive(Clone, Debug, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: Option<f64>,
}
impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y: Some(y) }
    }
    pub fn gap(x: f64) -> Self {
        Self { x, y: None }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct Series {
    pub id: String,
    pub data: Vec<Point>,
}
impl Series {
    pub fn new(id: impl Into<String>, data: Vec<Point>) -> Self {
        Self {
            id: id.into(),
            data,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Curve {
    #[default]
    Linear,
    LinearClosed,
    Step,
    Monotone,
    /// D3 Steffen interpolation along strictly monotonic x (alias of `Monotone`).
    MonotoneX,
    /// D3 Steffen interpolation along strictly monotonic y, independently per run.
    MonotoneY,
    StepBefore,
    StepAfter,
    Cardinal,
    CardinalOpen,
    CardinalClosed,
    CardinalTension {
        tension: f64,
    },
    CardinalOpenTension {
        tension: f64,
    },
    CardinalClosedTension {
        tension: f64,
    },
    Natural,
    Basis,
    BasisOpen,
    BasisClosed,
    CatmullRom {
        alpha: f64,
    },
    CatmullRomOpen {
        alpha: f64,
    },
    CatmullRomClosed {
        alpha: f64,
    },
    BumpX,
    BumpY,
    Bundle {
        beta: f64,
    },
}
impl Curve {
    pub fn cardinal(tension: f64) -> Self {
        Self::CardinalTension { tension }
    }
    pub fn cardinal_open(tension: f64) -> Self {
        Self::CardinalOpenTension { tension }
    }
    pub fn cardinal_closed(tension: f64) -> Self {
        Self::CardinalClosedTension { tension }
    }
    fn closed(self) -> bool {
        matches!(
            self,
            Self::LinearClosed
                | Self::BasisClosed
                | Self::CardinalClosed
                | Self::CardinalClosedTension { .. }
                | Self::CatmullRomClosed { .. }
        )
    }
    pub fn validate(self) -> Result<()> {
        match self {
            Self::CatmullRom { alpha }
            | Self::CatmullRomOpen { alpha }
            | Self::CatmullRomClosed { alpha } => ensure(
                alpha.is_finite() && (0. ..=1.).contains(&alpha),
                "Catmull-Rom alpha must be in 0..=1",
            ),
            Self::CardinalTension { tension }
            | Self::CardinalOpenTension { tension }
            | Self::CardinalClosedTension { tension } => ensure(
                tension.is_finite() && (0. ..=1.).contains(&tension),
                "cardinal tension must be in 0..=1",
            ),
            Self::Bundle { beta } => ensure(
                beta.is_finite() && (0. ..=1.).contains(&beta),
                "bundle beta must be in 0..=1",
            ),
            _ => Ok(()),
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct LineOptions {
    pub curve: Curve,
    pub area: bool,
    pub points: bool,
    pub baseline: f64,
    pub x_domain: Option<[f64; 2]>,
    pub y_domain: Option<[f64; 2]>,
    pub hidden: BTreeSet<String>,
    pub x_scale: ScaleKind,
    pub y_scale: ScaleKind,
    pub axes: CartesianAxes,
}
impl Default for LineOptions {
    fn default() -> Self {
        Self {
            curve: Curve::Linear,
            area: false,
            points: true,
            baseline: 0.,
            x_domain: None,
            y_domain: None,
            hidden: BTreeSet::new(),
            x_scale: ScaleKind::Linear,
            y_scale: ScaleKind::Linear,
            axes: CartesianAxes::default(),
        }
    }
}
fn id(value: &str) -> Result<()> {
    ensure(
        !value.is_empty() && value.len() <= 256,
        "ids must be nonempty and at most 256 bytes",
    )
}
pub(crate) fn validate_series(series: &[Series]) -> Result<()> {
    ensure(series.len() <= 256, "too many series (maximum 256)")?;
    let mut ids = BTreeSet::new();
    let mut total = 0usize;
    for s in series {
        id(&s.id)?;
        ensure(ids.insert(&s.id), "duplicate series id")?;
        total = total
            .checked_add(s.data.len())
            .ok_or_else(|| ChartError("point budget overflow".into()))?;
        ensure(total <= MAX_MARKS / 2, "point budget exceeded")?;
        ensure(
            s.data
                .iter()
                .all(|p| p.x.is_finite() && p.y.is_none_or(f64::is_finite)),
            "point coordinates must be finite",
        )?;
    }
    Ok(())
}
fn domain(values: impl Iterator<Item = f64>, zero: bool) -> Result<[f64; 2]> {
    let mut lo = if zero { 0. } else { f64::INFINITY };
    let mut hi = if zero { 0. } else { f64::NEG_INFINITY };
    for v in values {
        ensure(v.is_finite(), "domain contains nonfinite values")?;
        lo = lo.min(v);
        hi = hi.max(v);
    }
    if lo == f64::INFINITY {
        return Ok([0., 1.]);
    }
    if lo == hi {
        let padding = (lo.abs() * 0.05).max(1.);
        lo -= padding;
        hi += padding;
    }
    ensure(
        lo.is_finite() && hi.is_finite() && (hi - lo).is_finite(),
        "domain overflow",
    )?;
    Ok([lo, hi])
}
fn scale(d: [f64; 2], r: [f64; 2]) -> Result<LinearScale> {
    ensure(d[0] <= d[1], "chart domains must be ascending")?;
    LinearScale::new(d, r)
}
pub(crate) fn scale_domain(values: impl Iterator<Item = f64>, kind: ScaleKind) -> Result<[f64; 2]> {
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for v in values {
        ensure(v.is_finite(), "nonfinite scale input")?;
        lo = lo.min(v);
        hi = hi.max(v);
    }
    if lo == f64::INFINITY {
        return Ok(if kind.requires_nonzero_domain() {
            [1., 10.]
        } else {
            [0., 1.]
        });
    }
    if lo == hi {
        match kind {
            kind if kind.requires_nonzero_domain() => {
                ensure(lo != 0., "log values cannot be zero")?;
                let a = lo / 10.;
                let b = lo * 10.;
                lo = a.min(b);
                hi = a.max(b);
            }
            ScaleKind::Utc => {
                lo = (lo - 86_400_000.).max(-crate::MAX_TIMESTAMP_MS as f64);
                hi = (hi + 86_400_000.).min(crate::MAX_TIMESTAMP_MS as f64);
            }
            _ => {
                let pad = (lo.abs() * 0.05).max(1.);
                lo -= pad;
                hi += pad;
            }
        }
    }
    ensure(
        lo.is_finite() && hi.is_finite() && (hi - lo).is_finite(),
        "scale domain overflow",
    )?;
    Ok([lo, hi])
}
fn label(config: &ChartConfig, x: f64, y: f64, text: String, anchor: TextAnchor) -> Label {
    Label {
        rotation: 0.,
        x,
        y,
        text,
        color: config.theme.text,
        size: config.theme.font_size,
        anchor,
    }
}
fn fmt(v: f64) -> String {
    if v == 0. {
        "0".into()
    } else if v.abs() >= 1e6 || v.abs() < 0.001 {
        format!("{v:.2e}")
    } else {
        let s = format!("{v:.3}");
        s.trim_end_matches('0').trim_end_matches('.').into()
    }
}
fn stroke(color: Color, width: f64) -> GraphicStroke {
    GraphicStroke {
        color,
        width,
        cap: StrokeCap::Round,
    }
}
fn grid(scene: &mut Scene, config: &ChartConfig, x: &LinearScale, y: &LinearScale) {
    let l = config.margin.left;
    let r = config.width - config.margin.right;
    let t = config.margin.top;
    let b = config.height - config.margin.bottom;
    for (i, v) in y.ticks(5).into_iter().enumerate() {
        let py = y.map(v);
        let mut mark = Mark::new(
            format!("grid-y-{i}"),
            Geometry::Path(vec![Cmd::Move(P::new(l, py)), Cmd::Line(P::new(r, py))]),
            config.theme.grid,
        );
        mark.fill = None;
        mark.label.clear();
        mark.stroke = Some(stroke(config.theme.grid, 1.));
        scene.marks.push(mark);
        scene
            .labels
            .push(label(config, l - 9., py + 4., fmt(v), TextAnchor::End));
    }
    for v in x.ticks(6) {
        scene
            .labels
            .push(label(config, x.map(v), b + 22., fmt(v), TextAnchor::Middle));
    }
    let mut axes = Mark::new(
        "axes",
        Geometry::Path(vec![
            Cmd::Move(P::new(l, t)),
            Cmd::Line(P::new(l, b)),
            Cmd::Line(P::new(r, b)),
        ]),
        config.theme.text,
    );
    axes.fill = None;
    axes.label.clear();
    axes.stroke = Some(stroke(config.theme.text, 1.));
    scene.marks.push(axes);
}
fn legend(scene: &mut Scene, config: &ChartConfig, index: usize, id: &str) {
    scene.legend.push(LegendItem {
        id: id.into(),
        label: id.into(),
        color: config.theme.color(index),
    });
}
pub(crate) fn path(points: &[P], curve: Curve) -> Vec<Cmd> {
    let Some(first) = points.first() else {
        return vec![];
    };
    let mut commands = vec![Cmd::Move(*first)];
    match curve {
        Curve::Basis => return basis(points),
        Curve::Bundle { beta } => {
            if points.len() == 1 {
                commands.push(Cmd::Close);
                return commands;
            }
            let last = *points.last().unwrap();
            let adjusted = points
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    let t = i as f64 / (points.len() - 1) as f64;
                    P::new(
                        beta * p.x + (1. - beta) * (first.x * (1. - t) + last.x * t),
                        beta * p.y + (1. - beta) * (first.y * (1. - t) + last.y * t),
                    )
                })
                .collect::<Vec<_>>();
            return basis(&adjusted);
        }
        Curve::BasisOpen => return basis_variant(points, false),
        Curve::BasisClosed => return basis_variant(points, true),
        Curve::Cardinal => return cardinal(points, 0., EndMode::Clamped),
        Curve::CardinalOpen => return cardinal(points, 0., EndMode::Open),
        Curve::CardinalClosed => return cardinal(points, 0., EndMode::Closed),
        Curve::CardinalTension { tension } => return cardinal(points, tension, EndMode::Clamped),
        Curve::CardinalOpenTension { tension } => return cardinal(points, tension, EndMode::Open),
        Curve::CardinalClosedTension { tension } => {
            return cardinal(points, tension, EndMode::Closed);
        }
        Curve::CatmullRom { alpha } => return catmull(points, alpha, EndMode::Clamped),
        Curve::CatmullRomOpen { alpha } => return catmull(points, alpha, EndMode::Open),
        Curve::CatmullRomClosed { alpha } => return catmull(points, alpha, EndMode::Closed),
        Curve::Monotone | Curve::MonotoneX => return monotone(points, false),
        Curve::MonotoneY => return monotone(points, true),
        Curve::LinearClosed => {
            commands.extend(points.iter().skip(1).copied().map(Cmd::Line));
            commands.push(Cmd::Close);
            return commands;
        }
        Curve::BumpX | Curve::BumpY => {
            for pair in points.windows(2) {
                let a = pair[0];
                let b = pair[1];
                let (first, second) = if curve == Curve::BumpX {
                    let mid = a.x * 0.5 + b.x * 0.5;
                    (P::new(mid, a.y), P::new(mid, b.y))
                } else {
                    let mid = a.y * 0.5 + b.y * 0.5;
                    (P::new(a.x, mid), P::new(b.x, mid))
                };
                commands.push(Cmd::Cubic {
                    first,
                    second,
                    end: b,
                });
            }
        }
        Curve::Linear => commands.extend(points.iter().skip(1).copied().map(Cmd::Line)),
        Curve::Step => {
            for pair in points.windows(2) {
                let x = pair[0].x + (pair[1].x - pair[0].x) * 0.5;
                commands.extend([
                    Cmd::Line(P::new(x, pair[0].y)),
                    Cmd::Line(P::new(x, pair[1].y)),
                    Cmd::Line(pair[1]),
                ]);
            }
        }
        Curve::StepBefore | Curve::StepAfter => {
            for pair in points.windows(2) {
                let p = if curve == Curve::StepBefore {
                    P::new(pair[0].x, pair[1].y)
                } else {
                    P::new(pair[1].x, pair[0].y)
                };
                commands.extend([Cmd::Line(p), Cmd::Line(pair[1])]);
            }
        }
        Curve::Natural => {
            if points.len() == 2 {
                commands.push(Cmd::Line(points[1]));
            } else if points.len() > 2 {
                let controls = |values: Vec<f64>| {
                    let n = values.len() - 1;
                    let mut a = vec![0.; n];
                    let mut b = vec![0.; n];
                    let mut rhs = vec![0.; n];
                    b[0] = 2.;
                    rhs[0] = values[0] + 2. * values[1];
                    for i in 1..n - 1 {
                        a[i] = 1.;
                        b[i] = 4.;
                        rhs[i] = 4. * values[i] + 2. * values[i + 1];
                    }
                    a[n - 1] = 2.;
                    b[n - 1] = 7.;
                    rhs[n - 1] = 8. * values[n - 1] + values[n];
                    for i in 1..n {
                        let m = a[i] / b[i - 1];
                        b[i] -= m;
                        rhs[i] -= m * rhs[i - 1];
                    }
                    let mut first = vec![0.; n];
                    first[n - 1] = rhs[n - 1] / b[n - 1];
                    for i in (0..n - 1).rev() {
                        first[i] = (rhs[i] - first[i + 1]) / b[i];
                    }
                    let mut second = vec![0.; n];
                    for i in 0..n - 1 {
                        second[i] = 2. * values[i + 1] - first[i + 1];
                    }
                    second[n - 1] = (values[n] + first[n - 1]) / 2.;
                    (first, second)
                };
                let (x1, x2) = controls(points.iter().map(|p| p.x).collect());
                let (y1, y2) = controls(points.iter().map(|p| p.y).collect());
                for i in 0..points.len() - 1 {
                    commands.push(Cmd::Cubic {
                        first: P::new(x1[i], y1[i]),
                        second: P::new(x2[i], y2[i]),
                        end: points[i + 1],
                    });
                }
            }
        }
    }
    if points.len() == 1 {
        commands.push(Cmd::Close);
    }
    commands
}
fn basis(points: &[P]) -> Vec<Cmd> {
    let Some(first) = points.first() else {
        return vec![];
    };
    let mut out = vec![Cmd::Move(*first)];
    if points.len() < 3 {
        if points.len() == 2 {
            out.push(Cmd::Line(points[1]));
        } else {
            out.push(Cmd::Close);
        }
        return out;
    }
    let blend = |a: P, b: P, wa: f64, wb: f64, denominator: f64| {
        P::new(
            (a.x * wa + b.x * wb) / denominator,
            (a.y * wa + b.y * wb) / denominator,
        )
    };
    out.push(Cmd::Line(blend(points[0], points[1], 5., 1., 6.)));
    for i in 0..points.len() - 1 {
        let a = points[i];
        let b = points[i + 1];
        let c = points[(i + 2).min(points.len() - 1)];
        out.push(Cmd::Cubic {
            first: blend(a, b, 2., 1., 3.),
            second: blend(a, b, 1., 2., 3.),
            end: P::new((a.x + 4. * b.x + c.x) / 6., (a.y + 4. * b.y + c.y) / 6.),
        });
    }
    out.push(Cmd::Line(*points.last().unwrap()));
    out
}
// D3-shape 3.2.0 src/curve/{basisOpen,basisClosed,cardinal*,catmullRom*,
// monotone,bump}.js, also documented by E:/Repos/d3/docs/d3-shape/curve.md.
// The periodic variants use cyclic neighbors, not a straight closing edge.
#[derive(Clone, Copy, PartialEq)]
enum EndMode {
    Clamped,
    Open,
    Closed,
}
fn basis_variant(points: &[P], closed: bool) -> Vec<Cmd> {
    let n = points.len();
    if n == 0 || !closed && n < 3 {
        return vec![];
    }
    if closed && n == 1 {
        return vec![Cmd::Move(points[0]), Cmd::Close];
    }
    if closed && n == 2 {
        let blend = |a: P, b: P| P::new((a.x + 2. * b.x) / 3., (a.y + 2. * b.y) / 3.);
        return vec![
            Cmd::Move(blend(points[0], points[1])),
            Cmd::Line(blend(points[1], points[0])),
            Cmd::Close,
        ];
    }
    let blend = |a: P, b: P, c: P| P::new((a.x + 4. * b.x + c.x) / 6., (a.y + 4. * b.y + c.y) / 6.);
    let mut out = vec![Cmd::Move(blend(points[0], points[1], points[2]))];
    for i in 0..if closed { n } else { n - 3 } {
        let a = points[(i + 1) % n];
        let b = points[(i + 2) % n];
        let c = points[(i + 3) % n];
        out.push(Cmd::Cubic {
            first: P::new((2. * a.x + b.x) / 3., (2. * a.y + b.y) / 3.),
            second: P::new((a.x + 2. * b.x) / 3., (a.y + 2. * b.y) / 3.),
            end: blend(a, b, c),
        });
    }
    if !closed && n == 3 {
        out.push(Cmd::Close);
    }
    out
}
fn spline_segments(
    points: &[P],
    mode: EndMode,
    mut segment: impl FnMut(P, P, P, P, bool, bool) -> Cmd,
) -> Vec<Cmd> {
    let n = points.len();
    if n == 0 || mode == EndMode::Open && n < 3 {
        return vec![];
    }
    if n == 1 {
        return vec![Cmd::Move(points[0]), Cmd::Close];
    }
    if mode == EndMode::Closed && n == 2 {
        return vec![Cmd::Move(points[1]), Cmd::Line(points[0]), Cmd::Close];
    }
    let start = usize::from(mode != EndMode::Clamped);
    if mode == EndMode::Open && n == 3 {
        return vec![Cmd::Move(points[1]), Cmd::Close];
    }
    let count = match mode {
        EndMode::Clamped => n - 1,
        EndMode::Open => n - 3,
        EndMode::Closed => n,
    };
    let mut out = vec![Cmd::Move(points[start])];
    if mode == EndMode::Clamped && n == 2 {
        out.push(Cmd::Line(points[1]));
        return out;
    }
    for j in 0..count {
        let i = start + j;
        let a = points[i % n];
        let b = points[(i + 1) % n];
        let prev = if mode == EndMode::Clamped {
            points[i.saturating_sub(1)]
        } else {
            points[(i + n - 1) % n]
        };
        let next = if mode == EndMode::Clamped {
            points[(i + 2).min(n - 1)]
        } else {
            points[(i + 2) % n]
        };
        out.push(segment(
            prev,
            a,
            b,
            next,
            mode == EndMode::Clamped && i == 0,
            mode == EndMode::Clamped && i + 2 == n,
        ));
    }
    out
}
fn cardinal(points: &[P], tension: f64, mode: EndMode) -> Vec<Cmd> {
    let k = (1. - tension) / 6.;
    spline_segments(points, mode, |prev, a, b, next, first, last| Cmd::Cubic {
        first: if first {
            a
        } else {
            P::new(a.x + k * (b.x - prev.x), a.y + k * (b.y - prev.y))
        },
        second: if last {
            b
        } else {
            P::new(b.x + k * (a.x - next.x), b.y + k * (a.y - next.y))
        },
        end: b,
    })
}
fn catmull(points: &[P], alpha: f64, mode: EndMode) -> Vec<Cmd> {
    if alpha == 0. {
        return cardinal(points, 0., mode);
    }
    spline_segments(points, mode, |prev, a, b, next, _, _| {
        let l01 = (a.x - prev.x).hypot(a.y - prev.y).powf(alpha);
        let l12 = (b.x - a.x).hypot(b.y - a.y).powf(alpha);
        let l23 = (next.x - b.x).hypot(next.y - b.y).powf(alpha);
        let mut first = a;
        let mut second = b;
        if l01 > 1e-6 {
            let weight = 2. * l01 * l01 + 3. * l01 * l12 + l12 * l12;
            let divisor = 3. * l01 * (l01 + l12);
            first = P::new(
                (a.x * weight - prev.x * l12 * l12 + b.x * l01 * l01) / divisor,
                (a.y * weight - prev.y * l12 * l12 + b.y * l01 * l01) / divisor,
            );
        }
        if l23 > 1e-6 {
            let weight = 2. * l23 * l23 + 3. * l23 * l12 + l12 * l12;
            let divisor = 3. * l23 * (l23 + l12);
            second = P::new(
                (b.x * weight + a.x * l23 * l23 - next.x * l12 * l12) / divisor,
                (b.y * weight + a.y * l23 * l23 - next.y * l12 * l12) / divisor,
            );
        }
        Cmd::Cubic {
            first,
            second,
            end: b,
        }
    })
}
fn monotone(points: &[P], reflected: bool) -> Vec<Cmd> {
    // Steffen's weighted interior slopes and one-sided endpoint estimates,
    // as used by D3; the earlier Fritsch-Carlson path had different controls.
    let p = points
        .iter()
        .map(|p| if reflected { P::new(p.y, p.x) } else { *p })
        .collect::<Vec<_>>();
    let Some(first) = p.first() else {
        return vec![];
    };
    let reflect = |p: P| if reflected { P::new(p.y, p.x) } else { p };
    let mut out = vec![Cmd::Move(reflect(*first))];
    if p.len() == 1 {
        out.push(Cmd::Close);
        return out;
    }
    if p.len() == 2 {
        out.push(Cmd::Line(reflect(p[1])));
        return out;
    }
    let mut tangents = vec![0.; p.len()];
    for (i, w) in p.windows(3).enumerate() {
        let h0 = w[1].x - w[0].x;
        let h1 = w[2].x - w[1].x;
        let s0 = (w[1].y - w[0].y) / h0;
        let s1 = (w[2].y - w[1].y) / h1;
        let weighted = (s0 * h1 + s1 * h0) / (h0 + h1);
        let sign = |s: f64| if s < 0. { -1. } else { 1. };
        tangents[i + 1] = (sign(s0) + sign(s1)) * s0.abs().min(s1.abs()).min(0.5 * weighted.abs());
    }
    let last = p.len() - 1;
    tangents[0] = (3. * (p[1].y - p[0].y) / (p[1].x - p[0].x) - tangents[1]) / 2.;
    tangents[last] =
        (3. * (p[last].y - p[last - 1].y) / (p[last].x - p[last - 1].x) - tangents[last - 1]) / 2.;
    for (i, w) in p.windows(2).enumerate() {
        let dx = (w[1].x - w[0].x) / 3.;
        out.push(Cmd::Cubic {
            first: reflect(P::new(w[0].x + dx, w[0].y + dx * tangents[i])),
            second: reflect(P::new(w[1].x - dx, w[1].y - dx * tangents[i + 1])),
            end: reflect(w[1]),
        });
    }
    out
}
fn path_chunks(points: &[P], curve: Curve) -> Vec<Vec<Cmd>> {
    let commands = path(points, curve);
    if commands.is_empty() {
        return vec![];
    }
    let mut chunks = vec![];
    let Cmd::Move(start) = commands[0] else {
        unreachable!()
    };
    let mut current = start;
    let mut chunk = vec![Cmd::Move(current)];
    for mut command in commands.into_iter().skip(1) {
        if chunk.len() >= 240 {
            chunks.push(std::mem::take(&mut chunk));
            chunk.push(Cmd::Move(current));
        }
        // Close refers to the full run's Move, never a budget chunk's Move.
        if matches!(command, Cmd::Close) && !chunks.is_empty() {
            command = Cmd::Line(start);
        }
        current = match command {
            Cmd::Move(p) | Cmd::Line(p) => p,
            Cmd::Cubic { end, .. } => end,
            Cmd::Close => start,
        };
        chunk.push(command);
    }
    chunks.push(chunk);
    chunks
}
/// Line or baseline area chart; `None` values split runs without bridging gaps.
/// Open splines retain D3's interior endpoints and minimum point counts.
/// Closed area curves fill their periodic top contour: the constant baseline
/// is a separate degenerate contour, as in D3. Such an area run must fit in
/// the Scene command budget and retains one complete path. Direct SVG/Canvas
/// support it; the semantic Glory Widget applies its own per-node limit.
/// Monotone curves require a strictly monotonic independent axis in each run.
pub fn line(config: &ChartConfig, series: &[Series], options: &LineOptions) -> Result<Scene> {
    line_impl(config, series, options, None)
}
/// Shared producer for additive configured scales. Axis, geometry and sites
/// consume the very same checked mapping; the legacy entry retains its profile.
pub(crate) fn line_with_scales(
    config: &ChartConfig,
    series: &[Series],
    options: &LineOptions,
    x: &ChartScale,
    y: &ChartScale,
) -> Result<Scene> {
    line_impl(config, series, options, Some((x, y)))
}
fn line_impl(
    config: &ChartConfig,
    series: &[Series],
    options: &LineOptions,
    resolved: Option<(&ChartScale, &ChartScale)>,
) -> Result<Scene> {
    options.curve.validate()?;
    let mut scene = config.scene()?;
    validate_series(series)?;
    ensure(options.baseline.is_finite(), "baseline must be finite")?;
    let visible = series
        .iter()
        .enumerate()
        .filter(|(_, s)| !options.hidden.contains(&s.id))
        .collect::<Vec<_>>();
    let xd = match resolved.map(|(x, _)| x.domain()).or(options.x_domain) {
        Some(d) => d,
        None => scale_domain(
            visible.iter().flat_map(|(_, s)| s.data.iter().map(|p| p.x)),
            options.x_scale,
        )?,
    };
    let yd = match resolved.map(|(_, y)| y.domain()).or(options.y_domain) {
        Some(d) => d,
        None => scale_domain(
            visible
                .iter()
                .flat_map(|(_, s)| s.data.iter().filter_map(|p| p.y))
                .chain(options.area.then_some(options.baseline)),
            options.y_scale,
        )?,
    };
    let x = if let Some((x, _)) = resolved {
        x.clone()
    } else {
        ChartScale::new(
            options.x_scale,
            xd,
            [config.margin.left, config.width - config.margin.right],
        )?
    };
    let y = if let Some((_, y)) = resolved {
        y.clone()
    } else {
        ChartScale::new(
            options.y_scale,
            yd,
            [config.height - config.margin.bottom, config.margin.top],
        )?
    };
    let xb = [xd[0].min(xd[1]), xd[0].max(xd[1])];
    let yb = [yd[0].min(yd[1]), yd[0].max(yd[1])];
    cartesian_axes(&mut scene, config, &x, &y, &options.axes)?;
    let clip = ClipRect::plot(config);
    for (si, s) in visible {
        legend(&mut scene, config, si, &s.id);
        let series_start = scene.marks.len();
        let mut runs = Vec::<Vec<P>>::new();
        let mut run: Vec<P> = vec![];
        let mut previous = None;
        let constant_x = resolved.is_some() && xd[0] == xd[1];
        let constant_y = resolved.is_some() && yd[0] == yd[1];
        // A D3 constant scale maps every finite input to the range midpoint.
        // Give clipping a nonempty virtual interval instead of dropping runs.
        let tx = if constant_x {
            [0., 1.]
        } else {
            xd.map(|v| x.transformed(v))
        };
        let ty = if constant_y {
            [0., 1.]
        } else {
            yd.map(|v| y.transformed(v))
        };
        let transform_x = LinearScale::new(tx, x.range())?;
        let transform_y = LinearScale::new(ty, y.range())?;
        let data_clip = ClipRect {
            x: tx[0].min(tx[1]),
            y: ty[0].min(ty[1]),
            width: (tx[1] - tx[0]).abs(),
            height: (ty[1] - ty[0]).abs(),
        };
        for (pi, point) in s.data.iter().enumerate() {
            let Some(value) = point.y else {
                if !run.is_empty() {
                    runs.push(std::mem::take(&mut run));
                }
                previous = None;
                continue;
            };
            ensure(
                x.accepts(point.x) && y.accepts(value),
                "point is invalid for selected scale",
            )?;
            let px_value = if resolved.is_some() && x.is_clamped() {
                point.x.clamp(xb[0], xb[1])
            } else {
                point.x
            };
            let py_value = if resolved.is_some() && y.is_clamped() {
                value.clamp(yb[0], yb[1])
            } else {
                value
            };
            let inside = (constant_x || px_value >= xb[0] && px_value <= xb[1])
                && (constant_y || py_value >= yb[0] && py_value <= yb[1]);
            let raw = (
                if constant_x {
                    0.5
                } else {
                    x.transformed(px_value)
                },
                if constant_y {
                    0.5
                } else {
                    y.transformed(py_value)
                },
            );
            ensure(
                raw.0.is_finite() && raw.1.is_finite(),
                "scale transform overflow",
            )?;
            if options.curve == Curve::Linear {
                if let Some(prev) = previous {
                    if let Some([a, b]) = crate::clipping::clip_segment(prev, raw, data_clip)? {
                        let project = |p: (f64, f64)| {
                            if resolved.is_some() {
                                P::new(x.map_transformed(p.0), y.map_transformed(p.1))
                            } else {
                                P::new(transform_x.map(p.0), transform_y.map(p.1))
                            }
                        };
                        let a = project(a);
                        let b = project(b);
                        if run
                            .last()
                            .is_some_and(|p| (p.x - a.x).abs() > 1e-8 || (p.y - a.y).abs() > 1e-8)
                        {
                            runs.push(std::mem::take(&mut run));
                        }
                        if run.is_empty() {
                            run.push(a);
                        }
                        run.push(b);
                    } else if !run.is_empty() {
                        runs.push(std::mem::take(&mut run));
                    }
                } else if inside {
                    run.push(P::new(x.map(point.x), y.map(value)));
                }
                previous = Some(raw);
            } else {
                let p = P::new(x.map(point.x), y.map(value));
                ensure(
                    p.x.is_finite()
                        && p.y.is_finite()
                        && p.x.abs() <= 900_000.
                        && p.y.abs() <= 900_000.,
                    "curve projection exceeds coordinate budget; reduce the input domain before smoothing",
                )?;
                run.push(p);
            }
            if options.points && inside {
                let p = P::new(x.map(point.x), y.map(value));
                let mut mark = Mark::new(
                    format!("point-{si}-{pi}"),
                    Geometry::Circle {
                        x: p.x,
                        y: p.y,
                        radius: 3.5,
                    },
                    config.theme.color(si),
                );
                mark.series = s.id.clone();
                let xf = if options.x_scale == ScaleKind::Utc
                    && options.axes.x.format == ValueFormat::Auto
                {
                    &ValueFormat::Utc
                } else {
                    &options.axes.x.format
                };
                let yf = if options.y_scale == ScaleKind::Utc
                    && options.axes.y.format == ValueFormat::Auto
                {
                    &ValueFormat::Utc
                } else {
                    &options.axes.y.format
                };
                mark.label = format!("{}: {}, {}", s.id, xf.format(point.x)?, yf.format(value)?);
                mark.value = value;
                mark.clip = Some(clip);
                scene.marks.push(mark);
            }
        }
        if !run.is_empty() {
            runs.push(run);
        }
        for (ri, run) in runs.iter().enumerate() {
            if matches!(
                options.curve,
                Curve::Monotone | Curve::MonotoneX | Curve::MonotoneY
            ) {
                let independent = |p: &P| {
                    if options.curve == Curve::MonotoneY {
                        p.y
                    } else {
                        p.x
                    }
                };
                let ascending = run.len() < 2 || independent(&run[0]) < independent(&run[1]);
                ensure(
                    run.windows(2).all(|w| {
                        if ascending {
                            independent(&w[0]) < independent(&w[1])
                        } else {
                            independent(&w[0]) > independent(&w[1])
                        }
                    }),
                    "monotone curve requires a strictly monotonic independent axis in each defined run",
                )?;
            }
            // A periodic area is a closed top contour and a degenerate constant
            // baseline contour (D3 closed-area semantics). Splitting its fill
            // into independent Graphic paths would change winding/opacity, so
            // preserve it intact; individual hosts own their admission limits.
            let chunks = if options.area && options.curve.closed() {
                vec![path(run, options.curve)]
            } else {
                path_chunks(run, options.curve)
            };
            for (ci, commands) in chunks.into_iter().enumerate() {
                let color = config.theme.color(si);
                if options.area {
                    let mut area = commands.clone();
                    if area.last() == Some(&Cmd::Close) {
                        area.pop();
                    }
                    let first = match commands[0] {
                        Cmd::Move(p) => p,
                        _ => unreachable!(),
                    };
                    let last = match commands.last().unwrap() {
                        Cmd::Move(p) | Cmd::Line(p) => *p,
                        Cmd::Cubic { end, .. } => *end,
                        Cmd::Close => first,
                    };
                    if !options.curve.closed() {
                        let baseline = y.map(options.baseline.clamp(yb[0], yb[1]));
                        area.extend([
                            Cmd::Line(P::new(last.x, baseline)),
                            Cmd::Line(P::new(first.x, baseline)),
                        ]);
                    }
                    area.push(Cmd::Close);
                    let mut fill =
                        Mark::new(format!("area-{si}-{ri}-{ci}"), Geometry::Path(area), color);
                    fill.series = s.id.clone();
                    fill.label.clear();
                    fill.opacity = 0.35;
                    fill.clip = Some(clip);
                    scene.marks.push(fill);
                }
                let mut mark = Mark::new(
                    format!("line-{si}-{ri}-{ci}"),
                    Geometry::Path(commands),
                    color,
                );
                mark.series = s.id.clone();
                mark.label.clear();
                mark.fill = None;
                mark.stroke = Some(stroke(color, 2.));
                mark.clip = Some(clip);
                scene.marks.push(mark);
            }
        }
        // Paint fill chunks without outlines, then continuous outlines and points.
        // This avoids vertical seam strokes at semantic path-budget boundaries.
        scene.marks[series_start..].sort_by_key(|m| {
            if m.id.starts_with("area-") {
                0
            } else if m.id.starts_with("line-") {
                1
            } else {
                2
            }
        });
    }
    ensure(scene.marks.len() <= MAX_MARKS, "scene mark budget exceeded")?;
    scene.validate()?;
    Ok(scene)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum YAxisSide {
    Left,
    Right,
}
#[derive(Clone, Debug, PartialEq)]
pub struct DualAxisSeries {
    pub series: Series,
    pub axis: YAxisSide,
    pub area: bool,
}
impl DualAxisSeries {
    pub fn new(series: Series, axis: YAxisSide) -> Self {
        Self {
            series,
            axis,
            area: false,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct YAxisOptions {
    pub domain: Option<[f64; 2]>,
    pub scale: ScaleKind,
    pub axis: crate::axes::AxisOptions,
}
impl Default for YAxisOptions {
    fn default() -> Self {
        Self {
            domain: None,
            scale: ScaleKind::Linear,
            axis: crate::axes::AxisOptions::default(),
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct DualAxisOptions {
    pub x_domain: Option<[f64; 2]>,
    pub x_scale: ScaleKind,
    pub x_axis: crate::axes::AxisOptions,
    pub x_side: crate::axes::AxisSide,
    pub left: YAxisOptions,
    pub right: YAxisOptions,
    pub curve: Curve,
    pub points: bool,
    pub baseline: f64,
    pub hidden: BTreeSet<String>,
}
impl Default for DualAxisOptions {
    fn default() -> Self {
        Self {
            x_domain: None,
            x_scale: ScaleKind::Linear,
            x_axis: Default::default(),
            x_side: crate::axes::AxisSide::Bottom,
            left: YAxisOptions {
                axis: crate::axes::AxisOptions {
                    grid: true,
                    ..Default::default()
                },
                ..Default::default()
            },
            right: Default::default(),
            curve: Curve::Linear,
            points: true,
            baseline: 0.,
            hidden: BTreeSet::new(),
        }
    }
}
/// Compose lines/areas against independent left/right value scales and one
/// shared x scale. Series IDs determine paint identities and colors retain the
/// original input order, including when the series switch axis or are hidden.
pub fn dual_axis(
    config: &ChartConfig,
    data: &[DualAxisSeries],
    o: &DualAxisOptions,
) -> Result<Scene> {
    let mut out = config.scene()?;
    o.curve.validate()?;
    ensure(o.baseline.is_finite(), "dual-axis baseline must be finite")?;
    ensure(data.len() <= 256, "dual-axis series budget exceeded")?;
    let mut count = 0usize;
    let mut ids = BTreeSet::new();
    for item in data {
        id(&item.series.id)?;
        ensure(ids.insert(&item.series.id), "duplicate dual-axis series id")?;
        count = count
            .checked_add(item.series.data.len())
            .ok_or_else(|| ChartError("dual-axis point budget overflow".into()))?;
        ensure(count <= MAX_MARKS / 2, "dual-axis point budget exceeded")?;
        ensure(
            item.series
                .data
                .iter()
                .all(|p| p.x.is_finite() && p.y.is_none_or(f64::is_finite)),
            "dual-axis points must be finite",
        )?;
    }
    ensure(
        matches!(
            o.x_side,
            crate::axes::AxisSide::Top | crate::axes::AxisSide::Bottom
        ),
        "dual-axis x side must be top/bottom",
    )?;
    let xd = match o.x_domain {
        Some(d) => d,
        None => scale_domain(
            data.iter()
                .filter(|d| !o.hidden.contains(&d.series.id))
                .flat_map(|d| d.series.data.iter().map(|p| p.x)),
            o.x_scale,
        )?,
    };
    for side in [YAxisSide::Left, YAxisSide::Right] {
        let selected = data
            .iter()
            .enumerate()
            .filter(|(_, d)| d.axis == side)
            .collect::<Vec<_>>();
        let y = if side == YAxisSide::Left {
            &o.left
        } else {
            &o.right
        };
        let yd = match y.domain {
            Some(d) => d,
            None => scale_domain(
                selected
                    .iter()
                    .filter(|(_, d)| !o.hidden.contains(&d.series.id))
                    .flat_map(|(_, d)| {
                        d.series
                            .data
                            .iter()
                            .filter_map(|p| p.y)
                            .chain(d.area.then_some(o.baseline))
                    }),
                y.scale,
            )?,
        };
        let xs = ChartScale::new(
            o.x_scale,
            xd,
            [config.margin.left, config.width - config.margin.right],
        )?;
        let ys = ChartScale::new(
            y.scale,
            yd,
            [config.height - config.margin.bottom, config.margin.top],
        )?;
        if side == YAxisSide::Left {
            crate::axes::axis(&mut out, config, &xs, o.x_side, &o.x_axis, "dual-axis-x")?;
        }
        crate::axes::axis(
            &mut out,
            config,
            &ys,
            if side == YAxisSide::Left {
                crate::axes::AxisSide::Left
            } else {
                crate::axes::AxisSide::Right
            },
            &y.axis,
            if side == YAxisSide::Left {
                "dual-axis-left"
            } else {
                "dual-axis-right"
            },
        )?;
        for (original, item) in selected {
            if o.hidden.contains(&item.series.id) {
                continue;
            }
            let mut local = config.clone();
            local.theme.colors = vec![config.theme.color(original)];
            let mut disabled_x = crate::axes::AxisOptions {
                visible: false,
                grid: false,
                tick_count: 0,
                ..Default::default()
            };
            disabled_x.format = o.x_axis.format.clone();
            let mut disabled_y = disabled_x.clone();
            disabled_y.format = y.axis.format.clone();
            let layer = line(
                &local,
                std::slice::from_ref(&item.series),
                &LineOptions {
                    curve: o.curve,
                    area: item.area,
                    points: o.points,
                    baseline: o.baseline,
                    x_domain: Some(xd),
                    y_domain: Some(yd),
                    x_scale: o.x_scale,
                    y_scale: y.scale,
                    axes: CartesianAxes {
                        x: disabled_x,
                        y: disabled_y,
                        x_side: o.x_side,
                        y_side: if side == YAxisSide::Left {
                            crate::axes::AxisSide::Left
                        } else {
                            crate::axes::AxisSide::Right
                        },
                    },
                    ..Default::default()
                },
            )?;
            for mut mark in layer.marks {
                if mark.series.is_empty() {
                    continue;
                }
                let mut parts = mark.id.splitn(3, '-');
                let kind = parts.next().unwrap_or("mark");
                parts.next();
                let suffix = parts.next().unwrap_or("0");
                mark.id = format!(
                    "dual:{}:{}:{kind}:{suffix}",
                    item.series.id.len(),
                    item.series.id
                );
                out.marks.push(mark);
            }
        }
    }
    for (index, item) in data
        .iter()
        .enumerate()
        .filter(|(_, d)| !o.hidden.contains(&d.series.id))
    {
        out.legend.push(LegendItem {
            id: item.series.id.clone(),
            label: item.series.id.clone(),
            color: config.theme.color(index),
        });
    }
    out.validate()?;
    Ok(out)
}

#[derive(Clone, Debug, PartialEq)]
pub struct BarSeries {
    pub id: String,
    pub values: Vec<Option<f64>>,
}
impl BarSeries {
    pub fn new(id: impl Into<String>, values: Vec<Option<f64>>) -> Self {
        Self {
            id: id.into(),
            values,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BarLayout {
    #[default]
    Grouped,
    Stacked,
    Percent,
}
#[derive(Clone, Debug, PartialEq)]
pub struct BarOptions {
    pub corner_radius: f64,
    pub border: Option<GraphicStroke>,
    /// Optional ordinary border role. None preserves the fixed stroke. Some
    /// changes only an existing border's color, using the pre-factory fill as
    /// inherited/surface, the fixed border color as Theme, and chart background
    /// as backdrop. Hidden/cropped Some sources preflight before factories.
    pub border_color_profile: Option<crate::InheritedColor>,
    pub category_window: Option<[f64; 2]>,
    pub value_scale: ScaleKind,
    pub value_axis: crate::axes::AxisOptions,
    pub category_axis: crate::axes::CategoryAxisOptions,
    pub layout: BarLayout,
    pub horizontal: bool,
    pub padding: f64,
    pub hidden: BTreeSet<String>,
    pub value_domain: Option<[f64; 2]>,
}
impl Default for BarOptions {
    fn default() -> Self {
        Self {
            corner_radius: 0.,
            border: None,
            border_color_profile: None,
            category_window: None,
            value_scale: ScaleKind::Linear,
            value_axis: crate::axes::AxisOptions {
                grid: true,
                tick_count: 5,
                ..Default::default()
            },
            category_axis: crate::axes::CategoryAxisOptions::default(),
            layout: BarLayout::Grouped,
            horizontal: false,
            padding: 0.2,
            hidden: BTreeSet::new(),
            value_domain: None,
        }
    }
}
pub(crate) fn validate_bars(categories: &[String], series: &[BarSeries]) -> Result<()> {
    ensure(
        categories
            .len()
            .checked_mul(series.len())
            .is_some_and(|n| n <= MAX_MARKS - 32),
        "bar data budget exceeded",
    )?;
    ensure(
        categories.len() <= MAX_MARKS && series.len() <= 256,
        "bar dimension budget exceeded",
    )?;
    let mut ids = BTreeSet::new();
    for c in categories {
        id(c)?;
        ensure(ids.insert(c), "duplicate category id")?;
    }
    ids.clear();
    for s in series {
        id(&s.id)?;
        ensure(ids.insert(&s.id), "duplicate series id")?;
        ensure(
            s.values.len() == categories.len(),
            "bar series length differs from categories",
        )?;
        ensure(
            s.values.iter().flatten().all(|v| v.is_finite()),
            "bar values must be finite",
        )?;
    }
    Ok(())
}
fn bar_axes(
    scene: &mut Scene,
    c: &ChartConfig,
    categories: &[String],
    bands: &BandScale,
    value: &ChartScale,
    o: &BarOptions,
) -> Result<()> {
    let centers = (0..categories.len())
        .map(|i| bands.at(i).unwrap_or(0.) + bands.bandwidth / 2.)
        .collect::<Vec<_>>();
    bar_axes_positions(scene, c, categories, &centers, value, o)
}
pub(crate) fn bar_axes_positions(
    scene: &mut Scene,
    c: &ChartConfig,
    categories: &[String],
    centers: &[f64],
    value: &ChartScale,
    o: &BarOptions,
) -> Result<()> {
    o.category_axis.validate()?;
    crate::axes::axis(
        scene,
        c,
        value,
        if o.horizontal {
            crate::axes::AxisSide::Bottom
        } else {
            crate::axes::AxisSide::Left
        },
        &o.value_axis,
        "axis-bar-value",
    )?;
    let ticks = o
        .category_axis
        .tick_values
        .as_ref()
        .map(|ticks| ticks.iter().collect::<BTreeSet<_>>());
    if let Some(ticks) = &ticks {
        ensure(
            ticks.iter().all(|tick| categories.contains(tick)),
            "unknown category tick",
        )?;
    }
    let range = if o.horizontal {
        [c.margin.top, c.height - c.margin.bottom]
    } else {
        [c.margin.left, c.width - c.margin.right]
    };
    let visible = (0..categories.len())
        .filter(|i| {
            centers.get(*i).is_some_and(|center| {
                *center >= range[0].min(range[1]) && *center <= range[0].max(range[1])
            })
        })
        .collect::<Vec<_>>();
    let first = visible.first().copied().unwrap_or(0);
    let stride = visible
        .len()
        .div_ceil(o.category_axis.tick_count.max(1))
        .max(1);
    for (i, category) in categories.iter().enumerate() {
        if let Some(ticks) = &ticks {
            if !ticks.contains(category) {
                continue;
            }
        } else if o.category_axis.tick_count == 0 || i.saturating_sub(first) % stride != 0 {
            continue;
        }
        let pixel = centers[i];
        let [start, end] = range;
        if pixel < start.min(end) || pixel > start.max(end) {
            continue;
        }
        if o.category_axis.grid {
            let p = if o.horizontal {
                vec![
                    Cmd::Move(P::new(c.margin.left, pixel)),
                    Cmd::Line(P::new(c.width - c.margin.right, pixel)),
                ]
            } else {
                vec![
                    Cmd::Move(P::new(pixel, c.margin.top)),
                    Cmd::Line(P::new(pixel, c.height - c.margin.bottom)),
                ]
            };
            let mut mark = Mark::new(
                format!("category-grid-{i}"),
                Geometry::Path(p),
                c.theme.grid,
            );
            mark.fill = None;
            mark.label.clear();
            mark.stroke = Some(stroke(c.theme.grid, 1.));
            scene.marks.push(mark);
        }
        if o.category_axis.visible {
            let mut tick = if o.horizontal {
                label(
                    c,
                    c.margin.left - o.category_axis.tick_padding,
                    pixel + c.theme.font_size / 3.,
                    category.clone(),
                    TextAnchor::End,
                )
            } else {
                label(
                    c,
                    pixel,
                    c.height - c.margin.bottom + o.category_axis.tick_padding + c.theme.font_size,
                    category.clone(),
                    TextAnchor::Middle,
                )
            };
            tick.rotation = o.category_axis.tick_rotation;
            scene.labels.push(tick);
        }
    }
    if o.category_axis.visible
        && let Some(legend) = &o.category_axis.legend
    {
        scene.labels.push(if o.horizontal {
            label(
                c,
                c.margin.left,
                c.margin.top - c.theme.font_size,
                legend.clone(),
                TextAnchor::Middle,
            )
        } else {
            label(
                c,
                c.margin.left + c.inner_width() / 2.,
                c.height - c.margin.bottom + o.category_axis.tick_padding + c.theme.font_size * 2.5,
                legend.clone(),
                TextAnchor::Middle,
            )
        });
    }
    Ok(())
}
/// Grouped or signed diverging stacked bars, with optional independent positive/
/// negative percentage normalization. Missing values create no mark.
pub fn bar(
    config: &ChartConfig,
    categories: &[String],
    series: &[BarSeries],
    options: &BarOptions,
) -> Result<Scene> {
    let mut scene = config.scene()?;
    validate_bars(categories, series)?;
    validate_bar_style(options.corner_radius, options.border)?;
    if let Some(profile) = &options.border_color_profile {
        profile.validate()?;
    }
    // Preflight actual source roles, including hidden/cropped data. Missing
    // values have no fabricated color context; the profile is still validated.
    let borders = if options.border_color_profile.is_some() {
        Some(
            series
                .iter()
                .enumerate()
                .map(|(i, series)| {
                    if series.values.iter().any(Option::is_some) {
                        resolve_bar_border(config, options, config.theme.color(i))
                    } else {
                        Ok(options.border)
                    }
                })
                .collect::<Result<Vec<_>>>()?,
        )
    } else {
        None
    };
    ensure(
        options.value_scale.supports_zero_baseline(),
        "zero-baseline bars require linear, symlog, sqrt or positive power scale",
    )?;
    let visible = series
        .iter()
        .enumerate()
        .filter(|(_, s)| !options.hidden.contains(&s.id))
        .collect::<Vec<_>>();
    let mut positive = vec![0.; categories.len()];
    let mut negative = positive.clone();
    if options.layout != BarLayout::Grouped {
        for (_, s) in &visible {
            for (i, v) in s.values.iter().enumerate() {
                if let Some(v) = v {
                    if *v >= 0. {
                        positive[i] += v;
                    } else {
                        negative[i] += v;
                    }
                }
            }
        }
    }
    ensure(
        positive.iter().chain(&negative).all(|v| v.is_finite()),
        "stack total overflow",
    )?;
    let totals = (positive.clone(), negative.clone());
    let auto = match options.layout {
        BarLayout::Grouped => domain(
            visible
                .iter()
                .flat_map(|(_, s)| s.values.iter().flatten().copied()),
            true,
        )?,
        BarLayout::Stacked => domain(positive.iter().chain(&negative).copied(), true)?,
        BarLayout::Percent => [
            if negative.iter().any(|v| *v < 0.) {
                -1.
            } else {
                0.
            },
            1.,
        ],
    };
    let d = options.value_domain.unwrap_or(auto);
    ensure(d[0] <= d[1], "bar value domain must be ascending")?;
    let value = ChartScale::new(
        options.value_scale,
        d,
        if options.horizontal {
            [config.margin.left, config.width - config.margin.right]
        } else {
            [config.height - config.margin.bottom, config.margin.top]
        },
    )?
    .clamped(true);
    let unclamped_value = value.clone().clamped(false);
    let mut bands = BandScale::new(
        categories.to_vec(),
        if options.horizontal {
            [config.margin.top, config.height - config.margin.bottom]
        } else {
            [config.margin.left, config.width - config.margin.right]
        },
        options.padding,
    )?;
    if let Some(window) = options.category_window {
        bands = bands.windowed(window)?;
    }
    bar_axes(&mut scene, config, categories, &bands, &value, options)?;
    positive.fill(0.);
    negative.fill(0.);
    for (vi, (si, s)) in visible.iter().enumerate() {
        legend(&mut scene, config, *si, &s.id);
        let border = borders.as_ref().map_or(options.border, |b| b[*si]);
        for (i, raw) in s.values.iter().enumerate() {
            let Some(raw) = raw else {
                continue;
            };
            let v = if options.layout == BarLayout::Percent {
                if *raw >= 0. {
                    if totals.0[i] == 0. {
                        0.
                    } else {
                        raw / totals.0[i]
                    }
                } else if totals.1[i] == 0. {
                    0.
                } else {
                    raw / -totals.1[i]
                }
            } else {
                *raw
            };
            let base = if options.layout == BarLayout::Grouped {
                0.
            } else if v >= 0. {
                positive[i]
            } else {
                negative[i]
            };
            let end = base + v;
            if v >= 0. {
                positive[i] = end;
            } else {
                negative[i] = end;
            }
            // An explicit view can exclude the original zero baseline. Keep
            // stacking in data coordinates and crop only the final rectangle;
            // wholly off-screen bars must not remain as zero-area hit targets.
            if options.value_domain.is_some()
                && (base.max(end) < d[0]
                    || base.min(end) > d[1]
                    || base != end && (base.max(end) == d[0] || base.min(end) == d[1]))
            {
                continue;
            }
            let group_count = if options.layout == BarLayout::Grouped {
                visible.len().max(1)
            } else {
                1
            };
            let width = bands.bandwidth / group_count as f64;
            let position = bands.at(i).unwrap_or(0.)
                + if options.layout == BarLayout::Grouped {
                    vi as f64 * width
                } else {
                    0.
                };
            let [lo, hi] = bands.range();
            let start = position.max(lo.min(hi));
            let finish = (position + width).min(lo.max(hi));
            if finish <= start {
                continue;
            }
            let original_band = [position, width];
            let position = start;
            let width = finish - start;
            let a = value.map(base);
            let b = value.map(end);
            let geometry = if options.horizontal {
                Geometry::Rect {
                    x: a.min(b),
                    y: position,
                    width: (a - b).abs(),
                    height: width,
                }
            } else {
                Geometry::Rect {
                    x: position,
                    y: a.min(b),
                    width,
                    height: (a - b).abs(),
                }
            };
            let mut mark = Mark::new(format!("bar-{si}-{i}"), geometry, config.theme.color(*si));
            if options.corner_radius > 0. {
                mark.geometry = rounded_bar_projection(
                    &unclamped_value,
                    [base, end],
                    original_band,
                    ClipRect::plot(config),
                    options.horizontal,
                    options.corner_radius,
                    border,
                );
            }
            mark.stroke = border;
            mark.series = s.id.clone();
            mark.label = format!(
                "{} · {}: {}",
                categories[i],
                s.id,
                options.value_axis.format.format(*raw)?
            );
            mark.value = *raw;
            mark.clip = Some(ClipRect::plot(config));
            scene.marks.push(mark);
        }
    }
    scene.validate()?;
    Ok(scene)
}

/// Resolve a validated role even when border=None, so configuration validity
/// does not depend on whether a real source is currently painted.
pub(crate) fn resolve_bar_border(
    config: &ChartConfig,
    options: &BarOptions,
    inherited: Color,
) -> Result<Option<GraphicStroke>> {
    let Some(profile) = &options.border_color_profile else {
        return Ok(options.border);
    };
    let color = profile.resolve_validated(crate::InheritedColorContext {
        inherited,
        surface: inherited,
        theme: options
            .border
            .map_or(config.theme.text, |stroke| stroke.color),
        backdrop: config.theme.background,
    })?;
    Ok(options
        .border
        .map(|stroke| GraphicStroke { color, ..stroke }))
}

pub(crate) fn validate_bar_style(radius: f64, border: Option<GraphicStroke>) -> Result<()> {
    ensure(
        radius.is_finite() && (0. ..=16384.).contains(&radius),
        "invalid bar corner radius",
    )?;
    ensure(
        border.is_none_or(|s| s.width.is_finite() && (0.000001..=10000.).contains(&s.width)),
        "invalid bar border width",
    )
}
pub(crate) fn round_bar(geometry: Geometry, radius: f64) -> Geometry {
    let Geometry::Rect {
        x,
        y,
        width: w,
        height: h,
    } = geometry
    else {
        return geometry;
    };
    let r = radius.min(w / 2.).min(h / 2.);
    if r <= 0. {
        return Geometry::Rect {
            x,
            y,
            width: w,
            height: h,
        };
    }
    let k = r * 0.5522847498307936;
    Geometry::Path(vec![
        Cmd::Move(P::new(x + r, y)),
        Cmd::Line(P::new(x + w - r, y)),
        Cmd::Cubic {
            first: P::new(x + w - r + k, y),
            second: P::new(x + w, y + r - k),
            end: P::new(x + w, y + r),
        },
        Cmd::Line(P::new(x + w, y + h - r)),
        Cmd::Cubic {
            first: P::new(x + w, y + h - r + k),
            second: P::new(x + w - r + k, y + h),
            end: P::new(x + w - r, y + h),
        },
        Cmd::Line(P::new(x + r, y + h)),
        Cmd::Cubic {
            first: P::new(x + r - k, y + h),
            second: P::new(x, y + h - r + k),
            end: P::new(x, y + h - r),
        },
        Cmd::Line(P::new(x, y + r)),
        Cmd::Cubic {
            first: P::new(x, y + r - k),
            second: P::new(x + r - k, y),
            end: P::new(x + r, y),
        },
        Cmd::Close,
    ])
}

/// Keep the true rounded rectangle through the visible clip. Synthetic crop
/// boundaries stay at least two corner radii plus border width outside it:
/// one radius hides the synthetic cap, the second prevents re-clamping the
/// radius on a short visible stem with its other, genuine endpoint inside.
pub(crate) fn rounded_bar_projection(
    scale: &ChartScale,
    interval: [f64; 2],
    band: [f64; 2],
    clip: ClipRect,
    horizontal: bool,
    radius: f64,
    border: Option<GraphicStroke>,
) -> Geometry {
    let extent = 2. * radius.min(band[1] / 2.) + border.map_or(0., |stroke| stroke.width);
    let pixel = |value: f64| {
        let range = scale.range();
        bounded_bar_pixel(
            scale,
            value,
            [
                range[0].min(range[1]) - extent,
                range[0].max(range[1]) + extent,
            ],
        )
    };
    let a = pixel(interval[0]);
    let b = pixel(interval[1]);
    let (lo, hi) = if horizontal {
        (clip.y, clip.y + clip.height)
    } else {
        (clip.x, clip.x + clip.width)
    };
    let start = band[0].clamp(lo - extent, hi + extent);
    let finish = (band[0] + band[1]).clamp(lo - extent, hi + extent);
    let geometry = if horizontal {
        Geometry::Rect {
            x: a.min(b),
            y: start,
            width: (a - b).abs(),
            height: (finish - start).max(0.),
        }
    } else {
        Geometry::Rect {
            x: start,
            y: a.min(b),
            width: (finish - start).max(0.),
            height: (a - b).abs(),
        }
    };
    round_bar(geometry, radius)
}

pub(crate) fn bounded_bar_pixel(scale: &ChartScale, value: f64, bounds: [f64; 2]) -> f64 {
    let mapped = scale.map(value);
    if mapped.is_finite() {
        return mapped.clamp(bounds[0], bounds[1]);
    }
    // Extrapolation can overflow even for finite input. A value outside the
    // domain still has a known side, including on a descending/reversed axis.
    let [a, b] = scale.domain();
    let [first, last] = scale.range();
    let increasing = (a <= b) == (first <= last);
    if value < a.min(b) {
        bounds[usize::from(!increasing)]
    } else if value > a.max(b) {
        bounds[usize::from(increasing)]
    } else {
        mapped
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScatterOptions {
    pub radius: f64,
    pub x_domain: Option<[f64; 2]>,
    pub y_domain: Option<[f64; 2]>,
    pub hidden: BTreeSet<String>,
    pub x_scale: ScaleKind,
    pub y_scale: ScaleKind,
    pub axes: CartesianAxes,
}
impl Default for ScatterOptions {
    fn default() -> Self {
        Self {
            radius: 5.,
            x_domain: None,
            y_domain: None,
            hidden: BTreeSet::new(),
            x_scale: ScaleKind::Linear,
            y_scale: ScaleKind::Linear,
            axes: CartesianAxes::default(),
        }
    }
}
pub fn scatter(config: &ChartConfig, series: &[Series], options: &ScatterOptions) -> Result<Scene> {
    config.validate()?;
    validate_series(series)?;
    ensure(
        options.radius.is_finite() && (0. ..=256.).contains(&options.radius),
        "scatter radius must be in 0..=256",
    )?;
    let mut scene = line(
        config,
        series,
        &LineOptions {
            x_domain: options.x_domain,
            y_domain: options.y_domain,
            hidden: options.hidden.clone(),
            x_scale: options.x_scale,
            y_scale: options.y_scale,
            axes: options.axes.clone(),
            ..LineOptions::default()
        },
    )?;
    scene.marks.retain(|m| !m.id.starts_with("line-"));
    for m in &mut scene.marks {
        if let Geometry::Circle { radius, .. } = &mut m.geometry {
            *radius = options.radius;
        }
    }
    scene.validate()?;
    Ok(scene)
}

#[derive(Clone, Debug, PartialEq)]
pub struct HeatCell {
    pub row: String,
    pub column: String,
    pub value: Option<f64>,
}
impl HeatCell {
    pub fn new(row: impl Into<String>, column: impl Into<String>, value: f64) -> Self {
        Self {
            row: row.into(),
            column: column.into(),
            value: Some(value),
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct HeatmapOptions {
    pub padding: f64,
    pub square: bool,
    pub labels: bool,
    pub domain: Option<[f64; 2]>,
    pub low: Color,
    pub high: Color,
    pub missing: Color,
}
impl Default for HeatmapOptions {
    fn default() -> Self {
        Self {
            padding: 0.05,
            square: false,
            labels: false,
            domain: None,
            low: Color(235, 244, 251, 255),
            high: Color(31, 99, 171, 255),
            missing: Color(238, 238, 238, 255),
        }
    }
}
fn blend(a: Color, b: Color, t: f64) -> Color {
    let f = |a: u8, b: u8| ((a as f64) * (1. - t) + (b as f64) * t).round() as u8;
    Color(f(a.0, b.0), f(a.1, b.1), f(a.2, b.2), f(a.3, b.3))
}
pub fn heatmap(
    config: &ChartConfig,
    cells: &[HeatCell],
    options: &HeatmapOptions,
) -> Result<Scene> {
    let mut scene = config.scene()?;
    ensure(cells.len() <= MAX_MARKS, "heatmap cell budget exceeded")?;
    let mut rows = vec![];
    let mut columns = vec![];
    let mut seen_rows = BTreeSet::new();
    let mut seen_cols = BTreeSet::new();
    let mut pairs = BTreeSet::new();
    for c in cells {
        id(&c.row)?;
        id(&c.column)?;
        ensure(
            c.value.is_none_or(f64::is_finite),
            "heatmap values must be finite",
        )?;
        ensure(pairs.insert((&c.row, &c.column)), "duplicate heatmap cell")?;
        if seen_rows.insert(&c.row) {
            rows.push(c.row.clone());
        }
        if seen_cols.insert(&c.column) {
            columns.push(c.column.clone());
        }
    }
    let mut w = config.inner_width();
    let mut h = config.inner_height();
    if options.square && !rows.is_empty() && !columns.is_empty() {
        let size = (w / columns.len() as f64).min(h / rows.len() as f64);
        w = size * columns.len() as f64;
        h = size * rows.len() as f64;
    }
    let left = config.margin.left + (config.inner_width() - w) / 2.;
    let top = config.margin.top + (config.inner_height() - h) / 2.;
    let x = BandScale::new(columns.clone(), [left, left + w], options.padding)?;
    let y = BandScale::new(rows.clone(), [top, top + h], options.padding)?;
    let d = match options.domain {
        Some(domain) => domain,
        None => domain(cells.iter().filter_map(|c| c.value), false)?,
    };
    let colors = scale(d, [0., 1.])?.clamped(true);
    for (i, c) in cells.iter().enumerate() {
        let px = x.map(&c.column).unwrap_or(left);
        let py = y.map(&c.row).unwrap_or(top);
        let color = c.value.map_or(options.missing, |v| {
            blend(options.low, options.high, colors.map(v))
        });
        let mut m = Mark::new(
            format!("cell-{i}"),
            Geometry::Rect {
                x: px,
                y: py,
                width: x.bandwidth,
                height: y.bandwidth,
            },
            color,
        );
        m.series = c.row.clone();
        m.value = c.value.unwrap_or(0.);
        m.label = format!(
            "{} · {}: {}",
            c.row,
            c.column,
            c.value.map_or_else(|| "missing".into(), fmt)
        );
        scene.marks.push(m);
        if options.labels {
            scene.label_series.insert(scene.labels.len(), c.row.clone());
            scene.labels.push(label(
                config,
                px + x.bandwidth / 2.,
                py + y.bandwidth / 2. + 4.,
                c.value.map_or_else(|| "–".into(), fmt),
                TextAnchor::Middle,
            ));
        }
    }
    for (i, c) in columns
        .iter()
        .enumerate()
        .step_by(columns.len().div_ceil(40).max(1))
    {
        scene.labels.push(label(
            config,
            x.at(i).unwrap() + x.bandwidth / 2.,
            top + h + 22.,
            c.clone(),
            TextAnchor::Middle,
        ));
    }
    for (i, r) in rows
        .iter()
        .enumerate()
        .step_by(rows.len().div_ceil(40).max(1))
    {
        scene.labels.push(label(
            config,
            left - 9.,
            y.at(i).unwrap() + y.bandwidth / 2. + 4.,
            r.clone(),
            TextAnchor::End,
        ));
    }
    scene.validate()?;
    Ok(scene)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StreamOffset {
    #[default]
    Zero,
    Center,
    Expand,
    Wiggle,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StreamOptions {
    pub offset: StreamOffset,
    pub order: crate::stack::StackOrder,
    pub hidden: BTreeSet<String>,
}
/// Aligned nonnegative series stacked into areas. Center uses a silhouette
/// baseline; Expand normalizes each column to one. Null cells contribute zero.
pub fn stream(
    config: &ChartConfig,
    categories: &[String],
    series: &[BarSeries],
    options: &StreamOptions,
) -> Result<Scene> {
    let mut scene = config.scene()?;
    validate_bars(categories, series)?;
    let visible = series
        .iter()
        .enumerate()
        .filter(|(_, s)| !options.hidden.contains(&s.id))
        .collect::<Vec<_>>();
    let inputs: Vec<_> = visible.iter().map(|(_, s)| (*s).clone()).collect();
    let stack = crate::stack::stack(
        &inputs,
        &crate::stack::StackOptions {
            order: options.order,
            offset: match options.offset {
                StreamOffset::Zero => crate::stack::StackOffset::Zero,
                StreamOffset::Center => crate::stack::StackOffset::Silhouette,
                StreamOffset::Expand => crate::stack::StackOffset::Expand,
                StreamOffset::Wiggle => crate::stack::StackOffset::Wiggle,
            },
        },
    )?;
    let d = if stack.domain[0] == stack.domain[1] {
        [stack.domain[0], stack.domain[0] + 1.]
    } else {
        stack.domain
    };
    let x = LinearScale::new(
        [0., categories.len().saturating_sub(1).max(1) as f64],
        [config.margin.left, config.width - config.margin.right],
    )?;
    let y = LinearScale::new(d, [config.height - config.margin.bottom, config.margin.top])?;
    grid(&mut scene, config, &x, &y);
    scene
        .labels
        .retain(|l| l.y < config.height - config.margin.bottom + 10.);
    for (i, c) in categories
        .iter()
        .enumerate()
        .step_by(categories.len().div_ceil(40).max(1))
    {
        scene.labels.push(label(
            config,
            x.map(i as f64),
            config.height - config.margin.bottom + 22.,
            c.clone(),
            TextAnchor::Middle,
        ));
    }
    for &layer_index in &stack.order {
        let (si, s) = visible[layer_index];
        legend(&mut scene, config, si, &s.id);
        let layer = &stack.layers[layer_index];
        let lower: Vec<_> = layer.points.iter().map(|p| p.y0).collect();
        let baseline: Vec<_> = layer.points.iter().map(|p| p.y1).collect();
        // 120 points each direction fit Glory's 256-command semantic path budget.
        let mut start = 0;
        let mut chunk = 0;
        while start < categories.len() {
            let end = (start + 120).min(categories.len());
            let mut commands = vec![Cmd::Move(P::new(
                x.map(start as f64),
                y.map(baseline[start]),
            ))];
            for (i, v) in baseline.iter().enumerate().take(end).skip(start + 1) {
                commands.push(Cmd::Line(P::new(x.map(i as f64), y.map(*v))));
            }
            for (i, v) in lower.iter().enumerate().take(end).skip(start).rev() {
                commands.push(Cmd::Line(P::new(x.map(i as f64), y.map(*v))));
            }
            commands.push(Cmd::Close);
            let mut mark = Mark::new(
                format!("stream-{si}-{chunk}"),
                Geometry::Path(commands),
                config.theme.color(si),
            );
            mark.series = s.id.clone();
            mark.label.clear();
            mark.clip = Some(ClipRect::plot(config));
            scene.marks.push(mark);
            if end == categories.len() {
                break;
            }
            start = end - 1;
            chunk += 1;
        }
    }
    scene.validate()?;
    Ok(scene)
}
/// Rank lines. Input y values are explicit ranks (one is the best), matching
/// nivo's bump data contract. Axis direction places rank one at the top.
pub fn bump(config: &ChartConfig, series: &[Series], options: &LineOptions) -> Result<Scene> {
    validate_series(series)?;
    ensure(
        series
            .iter()
            .flat_map(|s| &s.data)
            .filter_map(|p| p.y)
            .all(|v| v >= 1. && v.fract() == 0.),
        "bump ranks must be positive integers",
    )?;
    let mut scene = line(config, series, options)?;
    let top = config.margin.top;
    let bottom = config.height - config.margin.bottom;
    for mark in &mut scene.marks {
        if mark.id == "axes" || mark.id.starts_with("axis-x") {
            continue;
        }
        match &mut mark.geometry {
            Geometry::Circle { y, .. } => *y = top + bottom - *y,
            Geometry::Path(commands) => {
                for cmd in commands {
                    match cmd {
                        Cmd::Move(p) | Cmd::Line(p) => p.y = top + bottom - p.y,
                        Cmd::Cubic { first, second, end } => {
                            first.y = top + bottom - first.y;
                            second.y = top + bottom - second.y;
                            end.y = top + bottom - end.y;
                        }
                        Cmd::Close => {}
                    }
                }
            }
            Geometry::Rect { y, height, .. } => *y = top + bottom - *y - *height,
        }
    }
    for l in &mut scene.labels {
        if matches!(l.anchor, TextAnchor::Start | TextAnchor::End) {
            l.y = top + bottom - l.y + config.theme.font_size * 2. / 3.;
        }
    }
    scene.validate()?;
    Ok(scene)
}
