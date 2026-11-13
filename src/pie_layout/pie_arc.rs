//! D3-shape 3.2.0 `src/arc.js` construction, including distinct inner/outer
//! asin padding and tangent-circle intersections after padding. Circular arcs
//! are lowered to <=90 degree cubics (radial error <=0.000273 * radius).
use std::f64::consts::{FRAC_PI_2, PI, TAU};

use glory::render::{GraphicPoint as P, PathCommand as C};

use crate::scene::ensure;
use crate::{Geometry, Result};
const EPS: f64 = 1e-12;
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ArcInput {
    pub center: [f64; 2],
    pub inner: f64,
    pub outer: f64,
    pub start: f64,
    pub end: f64,
    pub pad: f64,
    pub pad_radius: Option<f64>,
    pub corner: f64,
}
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ArcOutput {
    pub geometry: Geometry,
    pub inner_angles: [f64; 2],
    pub outer_angles: [f64; 2],
    pub inner_collapsed: bool,
    pub outer_collapsed: bool,
}
#[derive(Default)]
struct Path {
    out: Vec<C>,
    current: Option<P>,
    first: Option<P>,
    center: [f64; 2],
}
impl Path {
    fn p(&self, x: f64, y: f64) -> P {
        P::new(x + self.center[0], y + self.center[1])
    }
    fn mov(&mut self, x: f64, y: f64) {
        let p = self.p(x, y);
        self.out.push(C::Move(p));
        self.current = Some(p);
        self.first = Some(p);
    }
    fn line(&mut self, x: f64, y: f64) {
        let p = self.p(x, y);
        self.out.push(C::Line(p));
        self.current = Some(p);
    }
    fn arc(&mut self, x: f64, y: f64, r: f64, a: f64, b: f64, ccw: bool) {
        let start = self.p(x + r * a.cos(), y + r * a.sin());
        match self.current {
            None => self.mov(x + r * a.cos(), y + r * a.sin()),
            Some(p) if (p.x - start.x).hypot(p.y - start.y) > 1e-9 => {
                self.line(x + r * a.cos(), y + r * a.sin())
            }
            _ => {}
        }
        if r <= 0. {
            return;
        }
        let raw = if ccw { a - b } else { b - a };
        let sweep = if raw >= TAU - EPS {
            TAU
        } else {
            raw.rem_euclid(TAU)
        };
        if sweep < EPS {
            return;
        }
        let delta = if ccw { -sweep } else { sweep };
        let n = (delta.abs() / FRAC_PI_2).ceil().max(1.) as usize;
        let step = delta / n as f64;
        for i in 0..n {
            let a = a + i as f64 * step;
            let b = a + step;
            let k = 4. / 3. * (step / 4.).tan();
            let p = self.p(x + r * a.cos(), y + r * a.sin());
            let q = self.p(x + r * b.cos(), y + r * b.sin());
            self.out.push(C::Cubic {
                first: P::new(p.x - k * r * a.sin(), p.y + k * r * a.cos()),
                second: P::new(q.x + k * r * b.sin(), q.y - k * r * b.cos()),
                end: q,
            });
            self.current = Some(q);
        }
    }
    fn close(&mut self) {
        self.out.push(C::Close);
        self.current = self.first;
    }
}
fn intersect(a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2]) -> Option<[f64; 2]> {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let ex = d[0] - c[0];
    let ey = d[1] - c[1];
    let t = ey * dx - ex * dy;
    if t * t < EPS {
        return None;
    }
    let u = (ex * (a[1] - c[1]) - ey * (a[0] - c[0])) / t;
    Some([a[0] + u * dx, a[1] + u * dy])
}
#[derive(Clone, Copy)]
struct Tangent {
    cx: f64,
    cy: f64,
    x01: f64,
    y01: f64,
    x11: f64,
    y11: f64,
}
fn tangent(x0: f64, y0: f64, x1: f64, y1: f64, r1: f64, rc: f64, cw: bool) -> Tangent {
    let dx = x0 - x1;
    let dy = y0 - y1;
    let lo = (if cw { rc } else { -rc }) / dx.hypot(dy);
    let ox = lo * dy;
    let oy = -lo * dx;
    let ax = x0 + ox;
    let ay = y0 + oy;
    let bx = x1 + ox;
    let by = y1 + oy;
    let mx = (ax + bx) / 2.;
    let my = (ay + by) / 2.;
    let dx = bx - ax;
    let dy = by - ay;
    let d2 = dx * dx + dy * dy;
    let r = r1 - rc;
    let determinant = ax * by - bx * ay;
    let d =
        (if dy < 0. { -1. } else { 1. }) * (r * r * d2 - determinant * determinant).max(0.).sqrt();
    let mut cx = (determinant * dy - dx * d) / d2;
    let mut cy = (-determinant * dx - dy * d) / d2;
    let other_x = (determinant * dy + dx * d) / d2;
    let other_y = (-determinant * dx + dy * d) / d2;
    if (cx - mx).hypot(cy - my) > (other_x - mx).hypot(other_y - my) {
        cx = other_x;
        cy = other_y;
    }
    Tangent {
        cx,
        cy,
        x01: -ox,
        y01: -oy,
        x11: cx * (r1 / r - 1.),
        y11: cy * (r1 / r - 1.),
    }
}
fn angle(y: f64, x: f64) -> f64 {
    y.atan2(x)
}
fn rounded(
    path: &mut Path,
    tangents: [Tangent; 2],
    r: f64,
    rc: f64,
    merged: bool,
    cw: bool,
    inner: bool,
) {
    let [t0, t1] = tangents;
    if inner {
        path.line(t0.cx + t0.x01, t0.cy + t0.y01);
    } else {
        path.mov(t0.cx + t0.x01, t0.cy + t0.y01);
    }
    if merged {
        path.arc(
            t0.cx,
            t0.cy,
            rc,
            angle(t0.y01, t0.x01),
            angle(t1.y01, t1.x01),
            !cw,
        );
    } else {
        path.arc(
            t0.cx,
            t0.cy,
            rc,
            angle(t0.y01, t0.x01),
            angle(t0.y11, t0.x11),
            !cw,
        );
        path.arc(
            0.,
            0.,
            r,
            angle(t0.cy + t0.y11, t0.cx + t0.x11),
            angle(t1.cy + t1.y11, t1.cx + t1.x11),
            if inner { cw } else { !cw },
        );
        path.arc(
            t1.cx,
            t1.cy,
            rc,
            angle(t1.y11, t1.x11),
            angle(t1.y01, t1.x01),
            !cw,
        );
    }
}
pub(crate) fn build(input: ArcInput) -> Result<ArcOutput> {
    let ArcInput {
        center,
        inner: r0,
        outer: r1,
        start,
        end,
        pad,
        pad_radius,
        corner,
    } = input;
    let a0 = start - FRAC_PI_2;
    let a1 = end - FRAC_PI_2;
    let da = (a1 - a0).abs();
    let cw = a1 > a0;
    let mut p = Path {
        center,
        ..Default::default()
    };
    let (mut a00, mut a10, mut a01, mut a11) = (a0, a1, a0, a1);
    let (mut da0, mut da1) = (da, da);
    if r1 <= EPS {
        p.mov(0., 0.);
    } else if da > TAU - EPS {
        p.mov(r1 * a0.cos(), r1 * a0.sin());
        p.arc(0., 0., r1, a0, a1, !cw);
        if r0 > EPS {
            p.mov(r0 * a1.cos(), r0 * a1.sin());
            p.arc(0., 0., r0, a1, a0, cw);
        }
    } else {
        let ap = pad / 2.;
        let rp = if ap > EPS {
            pad_radius.unwrap_or(r0.hypot(r1))
        } else {
            0.
        };
        let rc = corner.min((r1 - r0).abs() / 2.);
        let (mut rc0, mut rc1) = (rc, rc);
        if rp > EPS {
            let p0 = (rp / r0 * ap.sin()).clamp(-1., 1.).asin();
            let p1 = (rp / r1 * ap.sin()).clamp(-1., 1.).asin();
            da0 -= p0 * 2.;
            if da0 > EPS {
                let shift = p0 * if cw { 1. } else { -1. };
                a00 += shift;
                a10 -= shift;
            } else {
                da0 = 0.;
                a00 = (a0 + a1) / 2.;
                a10 = a00;
            }
            da1 -= p1 * 2.;
            if da1 > EPS {
                let shift = p1 * if cw { 1. } else { -1. };
                a01 += shift;
                a11 -= shift;
            } else {
                da1 = 0.;
                a01 = (a0 + a1) / 2.;
                a11 = a01;
            }
        }
        let x01 = r1 * a01.cos();
        let y01 = r1 * a01.sin();
        let x10 = r0 * a10.cos();
        let y10 = r0 * a10.sin();
        let x11 = r1 * a11.cos();
        let y11 = r1 * a11.sin();
        let x00 = r0 * a00.cos();
        let y00 = r0 * a00.sin();
        if rc > EPS && da < PI {
            if let Some(oc) = intersect([x01, y01], [x00, y00], [x11, y11], [x10, y10]) {
                let ax = x01 - oc[0];
                let ay = y01 - oc[1];
                let bx = x11 - oc[0];
                let by = y11 - oc[1];
                let kc = 1.
                    / (((ax * bx + ay * by) / (ax.hypot(ay) * bx.hypot(by)))
                        .clamp(-1., 1.)
                        .acos()
                        / 2.)
                        .sin();
                let lc = oc[0].hypot(oc[1]);
                rc0 = rc.min((r0 - lc) / (kc - 1.));
                rc1 = rc.min((r1 - lc) / (kc + 1.));
            } else {
                rc0 = 0.;
                rc1 = 0.;
            }
        }
        if da1 <= EPS {
            p.mov(x01, y01);
        } else if rc1 > EPS {
            let t0 = tangent(x00, y00, x01, y01, r1, rc1, cw);
            let t1 = tangent(x11, y11, x10, y10, r1, rc1, cw);
            rounded(&mut p, [t0, t1], r1, rc1, rc1 < rc, cw, false);
        } else {
            p.mov(x01, y01);
            p.arc(0., 0., r1, a01, a11, !cw);
        }
        if r0 <= EPS || da0 <= EPS {
            p.line(x10, y10);
        } else if rc0 > EPS {
            let t0 = tangent(x10, y10, x11, y11, r0, -rc0, cw);
            let t1 = tangent(x01, y01, x00, y00, r0, -rc0, cw);
            rounded(&mut p, [t0, t1], r0, rc0, rc0 < rc, cw, true);
        } else {
            p.arc(0., 0., r0, a10, a00, cw);
        }
    }
    p.close();
    let geometry = Geometry::Path(p.out);
    ensure(
        geometry.commands().len() <= 128,
        "pie arc command budget exceeded",
    )?;
    Ok(ArcOutput {
        geometry,
        inner_angles: [a00 + FRAC_PI_2, a10 + FRAC_PI_2],
        outer_angles: [a01 + FRAC_PI_2, a11 + FRAC_PI_2],
        inner_collapsed: r0 <= EPS || da0 <= EPS,
        outer_collapsed: r1 <= EPS || da1 <= EPS,
    })
}
