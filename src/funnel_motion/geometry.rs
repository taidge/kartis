use glory::render::{GraphicPoint as P, PathCommand as C};

use super::*;

pub(super) fn numbers(g: &Geometry) -> Vec<f64> {
    match g {
        Geometry::Circle { x, y, radius } => vec![*x, *y, *radius],
        Geometry::Rect {
            x,
            y,
            width,
            height,
        } => vec![*x, *y, *width, *height],
        Geometry::Path(path) => path
            .iter()
            .flat_map(|c| match c {
                C::Move(p) | C::Line(p) => vec![p.x, p.y],
                C::Cubic { first, second, end } => {
                    vec![first.x, first.y, second.x, second.y, end.x, end.y]
                }
                C::Close => vec![],
            })
            .collect(),
    }
}
/// Inactive descriptors obey the same composed coordinate bounds as paint,
/// without inventing a Scene slot or granting interaction eligibility. Command
/// syntax is inherited from the validated constructor/target template.
pub(super) fn validate(g: &Geometry) -> Result<()> {
    let coordinate = |v: f64| v.is_finite() && v.abs() <= 1_000_000.;
    let valid = match g {
        Geometry::Rect {
            x,
            y,
            width,
            height,
        } => {
            [*x, *y, *width, *height, x + width, y + height]
                .into_iter()
                .all(coordinate)
                && *width >= 0.
                && *height >= 0.
        }
        Geometry::Circle { x, y, radius } => {
            [
                *x,
                *y,
                *radius,
                x + radius,
                x - radius,
                y + radius,
                y - radius,
            ]
            .into_iter()
            .all(coordinate)
                && *radius >= 0.
        }
        Geometry::Path(_) => numbers(g).into_iter().all(coordinate),
    };
    ensure(
        valid,
        "Funnel motion dormant geometry exceeds Scene coordinate limits",
    )
}
pub(super) fn same_kind(a: &Geometry, b: &Geometry) -> bool {
    matches!(
        (a, b),
        (Geometry::Circle { .. }, Geometry::Circle { .. })
            | (Geometry::Rect { .. }, Geometry::Rect { .. })
            | (Geometry::Path(_), Geometry::Path(_))
    )
}
pub(super) fn replace(template: &Geometry, numbers: &[f64]) -> Result<Geometry> {
    let mut iter = numbers.iter().copied();
    let mut take = || {
        iter.next()
            .ok_or_else(|| ChartError("Funnel motion geometry channel missing".into()))
    };
    Ok(match template {
        Geometry::Circle { .. } => Geometry::Circle {
            x: take()?,
            y: take()?,
            radius: take()?.max(0.),
        },
        Geometry::Rect { .. } => Geometry::Rect {
            x: take()?,
            y: take()?,
            width: take()?.max(0.),
            height: take()?.max(0.),
        },
        Geometry::Path(commands) => Geometry::Path(
            commands
                .iter()
                .map(|c| {
                    Ok(match c {
                        C::Move(_) => C::Move(P::new(take()?, take()?)),
                        C::Line(_) => C::Line(P::new(take()?, take()?)),
                        C::Cubic { .. } => C::Cubic {
                            first: P::new(take()?, take()?),
                            second: P::new(take()?, take()?),
                            end: P::new(take()?, take()?),
                        },
                        C::Close => C::Close,
                    })
                })
                .collect::<Result<_>>()?,
        ),
    })
}

#[derive(Clone, Debug)]
pub(super) enum Shape {
    Circle([f64; 3]),
    Rect { values: [f64; 4], radius: f64 },
}
impl Shape {
    fn values(&self) -> Vec<f64> {
        match self {
            Self::Circle(v) => v.to_vec(),
            Self::Rect { values, .. } => values.to_vec(),
        }
    }
    fn compatible(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (Self::Circle(_), Self::Circle(_)) | (Self::Rect { .. }, Self::Rect { .. })
        )
    }
    fn geometry(&self, v: &[f64]) -> Geometry {
        match self {
            Self::Circle(_) => {
                if v[2] <= 0. {
                    Geometry::Path(vec![])
                } else {
                    Geometry::Circle {
                        x: v[0],
                        y: v[1],
                        radius: v[2],
                    }
                }
            }
            Self::Rect { radius, .. } => {
                if v[2] <= 0. || v[3] <= 0. {
                    Geometry::Path(vec![])
                } else {
                    crate::cartesian::round_bar(
                        Geometry::Rect {
                            x: v[0],
                            y: v[1],
                            width: v[2],
                            height: v[3],
                        },
                        *radius,
                    )
                }
            }
        }
    }
}
#[derive(Clone, Debug)]
pub(super) struct GeometryTrack {
    target: Geometry,
    previous_numbers: Vec<f64>,
    values: Vec<SpringValue>,
    progress: Option<SpringValue>,
    shape: Option<Shape>,
}
impl GeometryTrack {
    pub fn new(
        target: Geometry,
        config: SpringConfig,
        scalar_path: bool,
        reduced: bool,
        shape: Option<Shape>,
    ) -> Result<Self> {
        let numbers = shape
            .as_ref()
            .map_or_else(|| numbers(&target), Shape::values);
        let progress = if shape.is_none() && scalar_path && matches!(target, Geometry::Path(_)) {
            Some(SpringValue::new(1., config)?)
        } else {
            None
        };
        let values = if progress.is_none() {
            numbers
                .iter()
                .map(|&v| SpringValue::new(v, config))
                .collect::<Result<_>>()?
        } else {
            vec![]
        };
        let mut out = Self {
            target,
            previous_numbers: numbers,
            values,
            progress,
            shape,
        };
        out.set_reduced(reduced);
        Ok(out)
    }
    pub fn retarget(
        &mut self,
        target: Geometry,
        config: SpringConfig,
        scalar_path: bool,
        reduced: bool,
        shape: Option<Shape>,
    ) -> Result<()> {
        if let (Some(old), Some(new)) = (&self.shape, &shape)
            && old.compatible(new)
        {
            for (spring, value) in self.values.iter_mut().zip(new.values()) {
                spring.retarget(value)?;
            }
            self.target = target;
            self.shape = shape;
            return Ok(());
        }
        if self.shape.is_some()
            || shape.is_some()
            || !same_kind(&self.target, &target)
            || (self.progress.is_some() != (scalar_path && matches!(target, Geometry::Path(_))))
        {
            *self = Self::new(target, config, scalar_path, reduced, shape)?;
            return Ok(());
        }
        if self.progress.is_some() {
            if self.target != target {
                self.previous_numbers = numbers(&self.target);
                let mut progress = SpringValue::new(0., config)?;
                progress.retarget(1.)?;
                progress.set_reduced_motion(reduced);
                self.progress = Some(progress);
            }
        } else {
            let n = numbers(&target);
            if self.values.len() != n.len() {
                *self = Self::new(target, config, scalar_path, reduced, shape)?;
                return Ok(());
            }
            for (spring, value) in self.values.iter_mut().zip(n) {
                spring.retarget(value)?;
            }
        }
        self.target = target;
        Ok(())
    }
    pub fn scene(&self) -> Result<Geometry> {
        if let Some(shape) = &self.shape {
            return Ok(shape.geometry(
                &self
                    .values
                    .iter()
                    .map(SpringValue::value)
                    .collect::<Vec<_>>(),
            ));
        }
        if let Some(progress) = &self.progress {
            let t = progress.value();
            // D3 pairs source/target numbers in ordinal order, retaining every
            // target token and leaving unmatched target numbers constant.
            let n = numbers(&self.target)
                .into_iter()
                .enumerate()
                .map(|(i, b)| {
                    self.previous_numbers
                        .get(i)
                        .map_or(b, |a| a * (1. - t) + b * t)
                })
                .collect::<Vec<_>>();
            replace(&self.target, &n)
        } else {
            replace(
                &self.target,
                &self
                    .values
                    .iter()
                    .map(SpringValue::value)
                    .collect::<Vec<_>>(),
            )
        }
    }
    pub fn settled(&self) -> bool {
        self.progress
            .iter()
            .chain(self.values.iter())
            .all(SpringValue::is_settled)
    }
    pub fn tick(&mut self, dt: f64) -> Result<()> {
        for v in self.progress.iter_mut().chain(self.values.iter_mut()) {
            v.tick(dt)?;
        }
        Ok(())
    }
    pub fn set_config(&mut self, c: SpringConfig) -> Result<()> {
        for v in self.progress.iter_mut().chain(self.values.iter_mut()) {
            v.set_config(c)?;
        }
        Ok(())
    }
    pub fn set_reduced(&mut self, r: bool) {
        for v in self.progress.iter_mut().chain(self.values.iter_mut()) {
            v.set_reduced_motion(r);
        }
    }
    pub fn finish(&mut self) {
        for v in self.progress.iter_mut().chain(self.values.iter_mut()) {
            v.finish();
        }
    }
    pub fn states(&self) -> Vec<PropertyState> {
        self.progress
            .iter()
            .chain(self.values.iter())
            .map(state)
            .collect()
    }
    pub fn progress(&self) -> Option<PropertyState> {
        self.progress.as_ref().map(state)
    }
}
