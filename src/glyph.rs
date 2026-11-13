//! D3-shape 3.2.0 symbols, using the actual `src/symbol/*.js` formulas.
//! Filled sizes are area in logical px². Stroked sizes are D3's perceptually
//! calibrated parameter (designed for 1.5px strokes with round caps), not ink
//! area. No font, DOM, random source, or renderer-specific surrogate is used.
use crate::{scene::ensure, *};
use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as C};
use std::f64::consts::{PI, TAU};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FilledGlyph {
    #[default]
    Circle,
    Cross,
    Diamond,
    Square,
    Star,
    Triangle,
    Wye,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StrokedGlyph {
    #[default]
    Circle,
    Plus,
    Times,
    Triangle,
    Asterisk,
    Square,
    Diamond,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GlyphShape {
    Filled(FilledGlyph),
    Stroked(StrokedGlyph),
}
impl Default for GlyphShape {
    fn default() -> Self {
        Self::Filled(FilledGlyph::Circle)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GlyphPaintMode {
    Fill,
    Stroke,
}
#[derive(Clone, Debug, PartialEq)]
pub struct GlyphOptions {
    pub shape: GlyphShape,
    /// Filled: exact mathematical area. Stroked: D3 nominal size parameter.
    pub size: f64,
    /// Radians, clockwise in the usual screen (positive y down) coordinates.
    pub rotation: f64,
}
impl Default for GlyphOptions {
    fn default() -> Self {
        Self {
            shape: Default::default(),
            size: 64.,
            rotation: 0.,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct GlyphGeometry {
    pub geometry: Geometry,
    /// D3 origin after translation, independent of the path bounding box.
    pub anchor: [f64; 2],
    /// Geometric centerline/fill bounds, excluding stroke width and clipping.
    pub bounds: ClipRect,
    pub paint_mode: GlyphPaintMode,
    pub shape: GlyphShape,
    pub size: f64,
    pub rotation: f64,
    /// Every unique symmetric vertex once. Indices address Move/Line endpoints
    /// in Geometry::Path, suitable for InteractionAnchor::path_points. Circle
    /// uses its actual primitive center and therefore has no path selectors.
    pub anchor_indices: Vec<usize>,
}
impl GlyphGeometry {
    /// Reject a mutated or forged factory snapshot before producing a Mark.
    pub fn validate(&self) -> Result<()> {
        let expected = glyph_geometry(
            self.anchor,
            &GlyphOptions {
                shape: self.shape,
                size: self.size,
                rotation: self.rotation,
            },
        )?;
        ensure(
            *self == expected,
            "glyph geometry/metadata differs from its checked factory",
        )
    }
}
fn polygon(vertices: Vec<[f64; 2]>) -> Vec<C> {
    let mut path = vertices
        .into_iter()
        .enumerate()
        .map(|(i, p)| {
            if i == 0 {
                C::Move(P::new(p[0], p[1]))
            } else {
                C::Line(P::new(p[0], p[1]))
            }
        })
        .collect::<Vec<_>>();
    path.push(C::Close);
    path
}
fn local(shape: GlyphShape, size: f64) -> Geometry {
    if matches!(
        shape,
        GlyphShape::Filled(FilledGlyph::Circle) | GlyphShape::Stroked(StrokedGlyph::Circle)
    ) {
        return Geometry::Circle {
            x: 0.,
            y: 0.,
            radius: (size / PI).sqrt(),
        };
    }
    let path = match shape {
        GlyphShape::Filled(f) => polygon(match f {
            FilledGlyph::Cross => {
                let r = (size / 5.).sqrt() / 2.;
                [
                    [-3., -1.],
                    [-1., -1.],
                    [-1., -3.],
                    [1., -3.],
                    [1., -1.],
                    [3., -1.],
                    [3., 1.],
                    [1., 1.],
                    [1., 3.],
                    [-1., 3.],
                    [-1., 1.],
                    [-3., 1.],
                ]
                .map(|[x, y]| [x * r, y * r])
                .to_vec()
            }
            FilledGlyph::Diamond => {
                let tan30 = (1. / 3_f64).sqrt();
                let y = (size / (tan30 * 2.)).sqrt();
                let x = y * tan30;
                vec![[0., -y], [x, 0.], [0., y], [-x, 0.]]
            }
            FilledGlyph::Square => {
                let w = size.sqrt();
                let x = -w / 2.;
                vec![[x, x], [x + w, x], [x + w, x + w], [x, x + w]]
            }
            FilledGlyph::Triangle => {
                let y = -(size / (3_f64.sqrt() * 3.)).sqrt();
                vec![
                    [0., y * 2.],
                    [-3_f64.sqrt() * y, -y],
                    [3_f64.sqrt() * y, -y],
                ]
            }
            FilledGlyph::Star => {
                let r = (size * 0.890_813_091_529_285_2).sqrt();
                let kr = (PI / 10.).sin() / (7. * PI / 10.).sin();
                let x = (TAU / 10.).sin() * kr * r;
                let y = -(TAU / 10.).cos() * kr * r;
                let mut p = vec![[0., -r], [x, y]];
                for i in 1..5 {
                    let a = TAU * i as f64 / 5.;
                    let c = a.cos();
                    let s = a.sin();
                    p.push([s * r, -c * r]);
                    p.push([c * x - s * y, s * x + c * y]);
                }
                p
            }
            FilledGlyph::Wye => {
                let c = -0.5;
                let s = 3_f64.sqrt() / 2.;
                let k = 1. / 12_f64.sqrt();
                let a = (k / 2. + 1.) * 3.;
                let r = (size / a).sqrt();
                let x0 = r / 2.;
                let y0 = r * k;
                let x1 = x0;
                let y1 = r * k + r;
                let x2 = -x1;
                let y2 = y1;
                vec![
                    [x0, y0],
                    [x1, y1],
                    [x2, y2],
                    [c * x0 - s * y0, s * x0 + c * y0],
                    [c * x1 - s * y1, s * x1 + c * y1],
                    [c * x2 - s * y2, s * x2 + c * y2],
                    [c * x0 + s * y0, c * y0 - s * x0],
                    [c * x1 + s * y1, c * y1 - s * x1],
                    [c * x2 + s * y2, c * y2 - s * x2],
                ]
            }
            FilledGlyph::Circle => unreachable!(),
        }),
        GlyphShape::Stroked(s) => match s {
            StrokedGlyph::Plus => {
                let r = (size - (size / 7.).min(2.)).sqrt() * 0.87559;
                vec![
                    C::Move(P::new(-r, 0.)),
                    C::Line(P::new(r, 0.)),
                    C::Move(P::new(0., r)),
                    C::Line(P::new(0., -r)),
                ]
            }
            StrokedGlyph::Times => {
                let r = (size - (size / 6.).min(1.7)).sqrt() * 0.6189;
                vec![
                    C::Move(P::new(-r, -r)),
                    C::Line(P::new(r, r)),
                    C::Move(P::new(-r, r)),
                    C::Line(P::new(r, -r)),
                ]
            }
            StrokedGlyph::Asterisk => {
                let r = (size + (size / 28.).min(0.75)).sqrt() * 0.59436;
                let t = r / 2.;
                let u = t * 3_f64.sqrt();
                vec![
                    C::Move(P::new(0., r)),
                    C::Line(P::new(0., -r)),
                    C::Move(P::new(-u, -t)),
                    C::Line(P::new(u, t)),
                    C::Move(P::new(-u, t)),
                    C::Line(P::new(u, -t)),
                ]
            }
            StrokedGlyph::Triangle => {
                let a = size.sqrt() * 0.6824;
                let t = a / 2.;
                let u = a * 3_f64.sqrt() / 2.;
                polygon(vec![[0., -a], [u, t], [-u, t]])
            }
            StrokedGlyph::Square => {
                let r = size.sqrt() * 0.4431;
                polygon(vec![[r, r], [r, -r], [-r, -r], [-r, r]])
            }
            StrokedGlyph::Diamond => {
                let r = size.sqrt() * 0.62625;
                polygon(vec![[0., -r], [r, 0.], [0., r], [-r, 0.]])
            }
            StrokedGlyph::Circle => unreachable!(),
        },
    };
    Geometry::Path(path)
}
/// Produce real symbol geometry. Size zero is deliberately retained: filled or
/// closed zero glyphs have no area, while open zero centerlines may paint a dot
/// with round/square caps. No hidden Circle interaction proxy is introduced.
/// Positive geometry that collapses after world translation is rejected.
pub fn glyph_geometry(center: [f64; 2], o: &GlyphOptions) -> Result<GlyphGeometry> {
    ensure(
        center.iter().all(|v| v.is_finite() && v.abs() <= 1e6)
            && o.size.is_finite()
            && o.size >= 0.
            && o.rotation.is_finite()
            && o.rotation.abs() <= 1e6,
        "invalid glyph center, size or rotation",
    )?;
    let mode = match o.shape {
        GlyphShape::Filled(_) => GlyphPaintMode::Fill,
        GlyphShape::Stroked(_) => GlyphPaintMode::Stroke,
    };
    let c = o.rotation.cos();
    let s = o.rotation.sin();
    let transform = |p: P| P::new(center[0] + p.x * c - p.y * s, center[1] + p.x * s + p.y * c);
    let mut indices = vec![];
    let (geometry, bounds) = match local(o.shape, o.size) {
        Geometry::Circle { radius, .. } => {
            ensure(
                radius.is_finite() && (o.size == 0. || radius > 0.),
                "positive glyph size has no representable radius",
            )?;
            (
                Geometry::Circle {
                    x: center[0],
                    y: center[1],
                    radius,
                },
                ClipRect {
                    x: center[0] - radius,
                    y: center[1] - radius,
                    width: 2. * radius,
                    height: 2. * radius,
                },
            )
        }
        Geometry::Path(path) => {
            let mut commands = Vec::with_capacity(path.len());
            let mut lo = [f64::INFINITY; 2];
            let mut hi = [f64::NEG_INFINITY; 2];
            let mut previous = None;
            let mut start = None;
            for cmd in path {
                let (p, is_move) = match cmd {
                    C::Move(p) => (p, true),
                    C::Line(p) => (p, false),
                    C::Close => {
                        ensure(
                            o.size == 0. || previous != start,
                            "positive glyph closing edge collapsed",
                        )?;
                        commands.push(C::Close);
                        continue;
                    }
                    C::Cubic { .. } => unreachable!(),
                };
                let p = transform(p);
                ensure(
                    p.x.is_finite() && p.y.is_finite() && p.x.abs() <= 1e6 && p.y.abs() <= 1e6,
                    "glyph point outside coordinate budget",
                )?;
                if is_move {
                    start = Some(p);
                } else {
                    ensure(
                        o.size == 0. || previous != Some(p),
                        "positive glyph edge collapsed after translation",
                    )?;
                }
                previous = Some(p);
                lo[0] = lo[0].min(p.x);
                lo[1] = lo[1].min(p.y);
                hi[0] = hi[0].max(p.x);
                hi[1] = hi[1].max(p.y);
                indices.push(commands.len());
                commands.push(if is_move { C::Move(p) } else { C::Line(p) });
            }
            (
                Geometry::Path(commands),
                ClipRect {
                    x: lo[0],
                    y: lo[1],
                    width: hi[0] - lo[0],
                    height: hi[1] - lo[1],
                },
            )
        }
        Geometry::Rect { .. } => unreachable!(),
    };
    bounds.validate()?;
    ensure(
        o.size == 0.
            || bounds.width > 0.
                && bounds.height > 0.
                && bounds.x + bounds.width > bounds.x
                && bounds.y + bounds.height > bounds.y,
        "positive glyph has no representable world extent",
    )?;
    if matches!(geometry, Geometry::Path(_)) {
        InteractionAnchor::path_points(&geometry, center, &indices)?;
    }
    Ok(GlyphGeometry {
        geometry,
        anchor: center,
        bounds,
        paint_mode: mode,
        shape: o.shape,
        size: o.size,
        rotation: o.rotation,
        anchor_indices: indices,
    })
}
/// Check the fill/stroke contract before creating a normal Scene Mark. Its
/// caller supplies series/label/value and can bind the explicit anchor metadata.
/// Stroked variants require no fill and an explicit positive-width stroke.
pub fn glyph_mark(
    id: impl Into<String>,
    glyph: &GlyphGeometry,
    fill: Option<Color>,
    stroke: Option<GraphicStroke>,
) -> Result<Mark> {
    glyph.validate()?;
    let id = id.into();
    ensure(
        !id.trim().is_empty() && id.len() <= MAX_LABEL_BYTES,
        "invalid glyph mark identity",
    )?;
    ensure(
        match glyph.paint_mode {
            GlyphPaintMode::Fill => fill.is_some(),
            GlyphPaintMode::Stroke => fill.is_none() && stroke.is_some(),
        },
        "glyph paint mode requires filled ink or explicit stroke-only ink",
    )?;
    ensure(
        stroke.is_none_or(|s| s.width.is_finite() && (0.000001..=10000.).contains(&s.width)),
        "invalid glyph stroke width",
    )?;
    let mut mark = Mark::new(
        id,
        glyph.geometry.clone(),
        fill.or(stroke.map(|s| s.color)).unwrap(),
    );
    mark.fill = fill;
    mark.stroke = stroke;
    Ok(mark)
}
