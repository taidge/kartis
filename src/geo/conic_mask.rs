//! Finite Conic footprint in the same raw/context affine as map consumers.
use super::*;
impl GeoProjectionContext {
    /// Build a true finite projection footprint for Conic projections. Other
    /// families return None and keep their existing disk/mask paths. Precision
    /// bounds circular-arc sagitta in logical pixels; finite coordinate and
    /// caller point budgets are checked before returning the complete mask.
    pub fn projection_clip_path(
        &self,
        precision: f64,
        max_points: usize,
    ) -> Result<Option<crate::ClipPath>> {
        ensure(
            precision.is_finite()
                && (0.0001..=64.).contains(&precision)
                && (4..=180_000).contains(&max_points),
            "invalid projection footprint precision or budget",
        )?;
        ensure(
            self.usa_projection().is_none(),
            "Albers USA footprints require a selected component rectangle",
        )?;
        let projection = self.view.projection;
        let Some(kernel) = projection.conic_kernel() else {
            return Ok(None);
        };
        let limit = if projection.latitude_bounded() {
            MERCATOR_LIMIT
        } else {
            FRAC_PI_2
        };
        let mut commands = Vec::new();
        let mut append = |source: [f64; 2]| -> Result<()> {
            ensure(
                commands.len() + 1 < max_points,
                "projection footprint point budget exceeded",
            )?;
            let point = self.map_raw(projection.conic_project(source));
            ensure(
                point.iter().all(|v| v.is_finite() && v.abs() <= 1_000_000.),
                "projection footprint exceeds Scene coordinates",
            )?;
            commands.push(if commands.is_empty() {
                Cmd::Move(P::new(point[0], point[1]))
            } else {
                Cmd::Line(P::new(point[0], point[1]))
            });
            Ok(())
        };
        if let Some(n) = kernel.n() {
            for (latitude, direction) in [(-limit, 1.), (limit, -1.)] {
                // x(pi/2)=r*sin(n*pi/2). The resolved original kernel gives
                // the signed radius without putting its possibly huge center
                // into the Scene. Multiplying n first avoids radius*scale
                // overflow for a nearly cylindrical but finite cone.
                let radius = kernel.raw_project([FRAC_PI_2, latitude])[0] / (n * FRAC_PI_2).sin();
                let curvature = (radius.abs() * n.abs()) * n.abs() * self.scale;
                ensure(
                    curvature.is_finite(),
                    "projection footprint curvature is not representable",
                )?;
                // 1-cos(theta)<=theta^2/2 gives a conservative arc sagitta
                // bound R*n^2*delta_lambda^2/8. Also retain <=pi/2 arc steps.
                let step = if curvature > 0. {
                    (8. * precision / curvature).sqrt().min(FRAC_PI_2 / n.abs())
                } else {
                    TAU
                };
                ensure(
                    step.is_finite() && step > 0.,
                    "projection footprint precision is not representable",
                )?;
                let count = (TAU / step).ceil().max(1.);
                ensure(
                    count < max_points as f64,
                    "projection footprint point budget exceeded",
                )?;
                let count = count as usize;
                for i in 0..=count {
                    let lambda = direction * (-PI + TAU * i as f64 / count as f64);
                    append([lambda, latitude])?;
                }
            }
        } else {
            for source in [[-PI, -limit], [PI, -limit], [PI, limit], [-PI, limit]] {
                append(source)?;
            }
        }
        commands.push(Cmd::Close);
        let mask = crate::ClipPath {
            geometry: Geometry::Path(commands),
            rule: crate::ClipRule::NonZero,
        };
        mask.validate()?;
        Ok(Some(mask))
    }
}
