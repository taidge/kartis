use glory::render::{GraphicPoint as P, PathCommand as C};

use super::*;

/// A configured native path generator, not a current-edge bounds approximation.
/// A bound context receives path commands; an unbound generator returns SVG.
/// It never begins/strokes a path or resets the caller's native transform.
#[derive(Clone, Debug)]
pub struct TreeCanvasLinkGenerator {
    curve: TreeLinkCurve,
    orientation: TreeOrientation,
    #[cfg(all(feature = "canvas", target_arch = "wasm32"))]
    context: Option<web_sys::CanvasRenderingContext2d>,
}
impl TreeCanvasLinkGenerator {
    pub fn new(curve: TreeLinkCurve, orientation: TreeOrientation) -> Self {
        Self {
            curve,
            orientation,
            #[cfg(all(feature = "canvas", target_arch = "wasm32"))]
            context: None,
        }
    }
    pub fn curve(&self) -> TreeLinkCurve {
        self.curve
    }
    pub fn set_curve(&mut self, curve: TreeLinkCurve) {
        self.curve = curve;
    }
    pub fn orientation(&self) -> TreeOrientation {
        self.orientation
    }
    pub fn set_orientation(&mut self, orientation: TreeOrientation) {
        self.orientation = orientation;
    }
    #[cfg(all(feature = "canvas", target_arch = "wasm32"))]
    pub fn context(&self) -> Option<web_sys::CanvasRenderingContext2d> {
        self.context.clone()
    }
    #[cfg(all(feature = "canvas", target_arch = "wasm32"))]
    pub fn set_context(&mut self, context: Option<web_sys::CanvasRenderingContext2d>) {
        self.context = context;
    }

    /// Arbitrary Cartesian endpoints. Radial layouts use `polar_geometry`
    /// explicitly, rather than secretly interpreting these x/y values as polar.
    pub fn geometry(&self, source: [f64; 2], target: [f64; 2]) -> Result<Geometry> {
        valid_point(source)?;
        valid_point(target)?;
        ensure(
            self.orientation != TreeOrientation::Radial,
            "Cartesian Tree Canvas generator requires Cartesian orientation",
        )?;
        let p = P::new(source[0], source[1]);
        let q = P::new(target[0], target[1]);
        let mut path = vec![C::Move(p)];
        let horizontal = matches!(
            self.orientation,
            TreeOrientation::LeftToRight | TreeOrientation::RightToLeft
        );
        match self.curve {
            TreeLinkCurve::Straight => path.push(C::Line(q)),
            TreeLinkCurve::Bump => {
                let (first, second) = if horizontal {
                    let x = (p.x + q.x) / 2.;
                    (P::new(x, p.y), P::new(x, q.y))
                } else {
                    let y = (p.y + q.y) / 2.;
                    (P::new(p.x, y), P::new(q.x, y))
                };
                path.push(C::Cubic {
                    first,
                    second,
                    end: q,
                });
            }
            curve => {
                if horizontal {
                    let x = match curve {
                        TreeLinkCurve::StepBefore => p.x,
                        TreeLinkCurve::StepAfter => q.x,
                        _ => (p.x + q.x) / 2.,
                    };
                    path.extend([C::Line(P::new(x, p.y)), C::Line(P::new(x, q.y)), C::Line(q)]);
                } else {
                    let y = match curve {
                        TreeLinkCurve::StepBefore => p.y,
                        TreeLinkCurve::StepAfter => q.y,
                        _ => (p.y + q.y) / 2.,
                    };
                    path.extend([C::Line(P::new(p.x, y)), C::Line(P::new(q.x, y)), C::Line(q)]);
                }
            }
        }
        Ok(Geometry::Path(path))
    }
    /// Explicit Rust radial extension: endpoints are `[radius, radians]`.
    pub fn polar_geometry(
        &self,
        source: [f64; 2],
        target: [f64; 2],
        center: [f64; 2],
    ) -> Result<Geometry> {
        valid_point(center)?;
        ensure(
            source
                .into_iter()
                .chain(target)
                .all(|v| v.is_finite() && v.abs() <= 1e6)
                && source[0] >= 0.
                && target[0] >= 0.,
            "invalid Tree Canvas polar endpoints",
        )?;
        let point = |r: f64, a: f64| [center[0] + r * a.cos(), center[1] + r * a.sin()];
        let p = point(source[0], source[1]);
        let q = point(target[0], target[1]);
        valid_point(p)?;
        valid_point(q)?;
        let mut path = vec![C::Move(P::new(p[0], p[1]))];
        if self.curve == TreeLinkCurve::Straight {
            path.push(C::Line(P::new(q[0], q[1])));
        } else if self.curve == TreeLinkCurve::Bump {
            let middle = (source[0] + target[0]) / 2.;
            let a = point(middle, source[1]);
            let b = point(middle, target[1]);
            valid_point(a)?;
            valid_point(b)?;
            path.push(C::Cubic {
                first: P::new(a[0], a[1]),
                second: P::new(b[0], b[1]),
                end: P::new(q[0], q[1]),
            });
        } else {
            let radius = match self.curve {
                TreeLinkCurve::StepBefore => source[0],
                TreeLinkCurve::StepAfter => target[0],
                _ => (source[0] + target[0]) / 2.,
            };
            let delta = (target[1] - source[1])
                .sin()
                .atan2((target[1] - source[1]).cos());
            let start = point(radius, source[1]);
            valid_point(start)?;
            path.push(C::Line(P::new(start[0], start[1])));
            if radius > 0. && delta != 0. {
                let count = (delta.abs() / std::f64::consts::FRAC_PI_2).ceil() as usize;
                for i in 0..count {
                    let a = source[1] + delta * i as f64 / count as f64;
                    let b = source[1] + delta * (i + 1) as f64 / count as f64;
                    let k = (4. / 3.) * ((b - a) / 4.).tan();
                    let first = [
                        center[0] + radius * (a.cos() - k * a.sin()),
                        center[1] + radius * (a.sin() + k * a.cos()),
                    ];
                    let second = [
                        center[0] + radius * (b.cos() + k * b.sin()),
                        center[1] + radius * (b.sin() - k * b.cos()),
                    ];
                    let end = point(radius, b);
                    valid_point(first)?;
                    valid_point(second)?;
                    valid_point(end)?;
                    path.push(C::Cubic {
                        first: P::new(first[0], first[1]),
                        second: P::new(second[0], second[1]),
                        end: P::new(end[0], end[1]),
                    });
                }
            }
            path.push(C::Line(P::new(q[0], q[1])));
        }
        Ok(Geometry::Path(path))
    }
    pub fn svg_path(&self, source: [f64; 2], target: [f64; 2]) -> Result<String> {
        Ok(crate::render::path_data(
            &self.geometry(source, target)?.commands(),
        ))
    }
    pub fn generate(&mut self, source: [f64; 2], target: [f64; 2]) -> Result<Option<String>> {
        let geometry = self.geometry(source, target)?;
        #[cfg(all(feature = "canvas", target_arch = "wasm32"))]
        if let Some(context) = &self.context {
            append_geometry(context, &geometry)?;
            return Ok(None);
        }
        Ok(Some(crate::render::path_data(&geometry.commands())))
    }
    pub fn generate_polar(
        &mut self,
        source: [f64; 2],
        target: [f64; 2],
        center: [f64; 2],
    ) -> Result<Option<String>> {
        let geometry = self.polar_geometry(source, target, center)?;
        #[cfg(all(feature = "canvas", target_arch = "wasm32"))]
        if let Some(context) = &self.context {
            append_geometry(context, &geometry)?;
            return Ok(None);
        }
        Ok(Some(crate::render::path_data(&geometry.commands())))
    }
    #[cfg(all(feature = "canvas", target_arch = "wasm32"))]
    pub fn append_path(
        &self,
        context: &web_sys::CanvasRenderingContext2d,
        source: [f64; 2],
        target: [f64; 2],
    ) -> Result<()> {
        append_geometry(context, &self.geometry(source, target)?)
    }
}
fn valid_point(p: [f64; 2]) -> Result<()> {
    ensure(
        p.into_iter().all(|v| v.is_finite() && v.abs() <= 1e6),
        "invalid Tree Canvas generator endpoint",
    )
}
#[cfg(all(feature = "canvas", target_arch = "wasm32"))]
pub(super) fn append_geometry(
    context: &web_sys::CanvasRenderingContext2d,
    geometry: &Geometry,
) -> Result<()> {
    for command in geometry.commands() {
        match command {
            C::Move(p) => context.move_to(p.x, p.y),
            C::Line(p) => context.line_to(p.x, p.y),
            C::Cubic { first, second, end } => {
                context.bezier_curve_to(first.x, first.y, second.x, second.y, end.x, end.y)
            }
            C::Close => context.close_path(),
        }
    }
    Ok(())
}
