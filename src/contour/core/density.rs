//! Weighted Float32 density grid; independent d3-contour/d3-array oracle.
use super::*;

#[derive(Clone, Copy, Debug)]
pub struct WeightedPoint {
    pub position: Point,
    pub weight: f64,
}
#[derive(Clone, Copy, Debug)]
pub struct DensityOptions {
    pub size: [f64; 2],
    pub bandwidth: f64,
    pub cell_size: f64,
    pub limits: Limits,
}
impl Default for DensityOptions {
    fn default() -> Self {
        Self {
            size: [960., 500.],
            bandwidth: 420_f64.sqrt(),
            cell_size: 4.,
            limits: Limits::default(),
        }
    }
}
#[derive(Debug)]
pub struct DensityField {
    values: Vec<f32>,
    width: usize,
    height: usize,
    cell: usize,
    offset: f64,
    max: f64,
}
#[derive(Clone, Debug)]
pub enum DensityThresholds {
    Count(usize),
    Values(Vec<f64>),
}
impl Default for DensityThresholds {
    fn default() -> Self {
        Self::Count(20)
    }
}
impl DensityField {
    pub fn values(&self) -> &[f32] {
        &self.values
    }
    pub fn dimensions(&self) -> [usize; 2] {
        [self.width, self.height]
    }
    pub fn cell_size(&self) -> usize {
        self.cell
    }
    pub fn max(&self) -> f64 {
        self.max
    }
    fn field(&self) -> Field<'_> {
        Field {
            width: self.width,
            height: self.height,
            samples: Samples::Dense(&self.values),
        }
    }
    fn transform(&self, level: &mut Level) -> Result<()> {
        let cell = self.cell as f64;
        for polygon in &mut level.polygons {
            for ring in std::iter::once(&mut polygon.outer).chain(&mut polygon.holes) {
                for point in ring {
                    point[0] = point[0] * cell - self.offset;
                    point[1] = point[1] * cell - self.offset;
                    ensure(
                        point.iter().all(|v| v.is_finite()),
                        "density transformed coordinate overflow",
                    )?;
                }
            }
        }
        Ok(())
    }
    pub fn contour(&self, value: f64, limits: Limits) -> Result<Level> {
        limits.validate()?;
        ensure(
            self.values.len() <= limits.cells,
            "density contour cell budget exceeded",
        )?;
        ensure(limits.levels > 0, "density contour level budget exceeded")?;
        let mut budget = Budget {
            limits,
            work: 0,
            edges: 0,
            points: 0,
            rings: 0,
        };
        self.one(value, &mut budget)
    }
    fn one(&self, value: f64, budget: &mut Budget) -> Result<Level> {
        let scaled = value * (self.cell as f64).powi(2);
        ensure(
            value.is_finite() && scaled.is_finite(),
            "density threshold overflow",
        )?;
        let mut result = level(self.field(), scaled, true, budget)?;
        result.value = value;
        self.transform(&mut result)?;
        Ok(result)
    }
    pub fn contours(&self, thresholds: &DensityThresholds, limits: Limits) -> Result<Vec<Level>> {
        limits.validate()?;
        ensure(
            self.values.len() <= limits.cells,
            "density contour cell budget exceeded",
        )?;
        let values = match thresholds {
            DensityThresholds::Count(count) => {
                ensure(
                    *count <= limits.levels,
                    "density threshold count budget exceeded",
                )?;
                numeric_ticks(f64::from_bits(1), self.max, *count, limits.levels)?
            }
            DensityThresholds::Values(values) => {
                ensure(
                    values.len() <= limits.levels && values.iter().all(|v| v.is_finite()),
                    "invalid density thresholds or level budget",
                )?;
                let mut values = values.clone();
                values.sort_by(|a, b| a.partial_cmp(b).unwrap());
                values
            }
        };
        // Bind every geometry to its actual value. The original bulk D3 path
        // sorts geometry but can relabel it with unsorted source levels.
        let mut budget = Budget {
            limits,
            work: 0,
            edges: 0,
            points: 0,
            rings: 0,
        };
        values
            .into_iter()
            .map(|value| self.one(value, &mut budget))
            .collect()
    }
}

pub fn density(points: &[WeightedPoint], options: DensityOptions) -> Result<DensityField> {
    options.limits.validate()?;
    ensure(points.len() <= 20_000, "density sample budget exceeded")?;
    ensure(
        options
            .size
            .iter()
            .all(|v| v.is_finite() && *v > 0. && *v <= 1_000_000.)
            && options.bandwidth.is_finite()
            && (0. ..=100_000.).contains(&options.bandwidth)
            && options.cell_size.is_finite()
            && (1. ..=1_048_576.).contains(&options.cell_size),
        "invalid density size, bandwidth or cell size",
    )?;
    ensure(
        points
            .iter()
            .all(|p| p.position.iter().all(|v| v.is_finite()) && p.weight.is_finite()),
        "nonfinite density point or weight",
    )?;
    let radius = ((4. * options.bandwidth * options.bandwidth + 1.).sqrt() - 1.) / 2.;
    let offset = radius * 3.;
    let power = (options.cell_size.ln() / std::f64::consts::LN_2).floor() as u32;
    let cell = 1_usize << power;
    let width = ((options.size[0] + offset * 2.).floor() as usize) >> power;
    let height = ((options.size[1] + offset * 2.).floor() as usize) >> power;
    let count = width
        .checked_mul(height)
        .ok_or_else(|| Error("density grid overflow".into()))?;
    ensure(
        width > 0 && height > 0 && count <= options.limits.cells,
        "density grid dimensions or cell budget exceeded",
    )?;
    let mut budget = Budget {
        limits: options.limits,
        work: 0,
        edges: 0,
        points: 0,
        rings: 0,
    };
    budget.work(
        count
            .checked_add(points.len() * 4)
            .ok_or_else(|| Error("density work overflow".into()))?,
    )?;
    let mut values = vec![0_f32; count];
    let inverse = 2_f64.powi(-(power as i32));
    for p in points {
        let x = (p.position[0] + offset) * inverse;
        let y = (p.position[1] + offset) * inverse;
        if p.weight == 0. || x < 0. || y < 0. || x >= width as f64 || y >= height as f64 {
            continue;
        }
        let x0 = x.floor() as usize;
        let y0 = y.floor() as usize;
        let xt = x - x0 as f64 - 0.5;
        let yt = y - y0 as f64 - 0.5;
        let index = x0 + y0 * width;
        for (index, amount) in [
            (index, (1. - xt) * (1. - yt) * p.weight),
            (index + 1, xt * (1. - yt) * p.weight),
            (index + 1 + width, xt * yt * p.weight),
            (index + width, (1. - xt) * yt * p.weight),
        ] {
            if let Some(value) = values.get_mut(index) {
                let next = (f64::from(*value) + amount) as f32;
                ensure(next.is_finite(), "density Float32 accumulation overflow")?;
                *value = next;
            }
        }
    }
    let blur_radius = radius * inverse;
    if blur_radius > 0. {
        let r = blur_radius.floor() as usize;
        let blur_work = count
            .checked_mul(6)
            .and_then(|work| {
                (width + height).checked_mul(r).and_then(|extra| {
                    extra
                        .checked_mul(3)
                        .and_then(|extra| work.checked_add(extra))
                })
            })
            .ok_or_else(|| Error("density blur work overflow".into()))?;
        budget.work(blur_work)?;
        let mut temporary = values.clone();
        for pass in 0..3 {
            if pass % 2 == 0 {
                blur_horizontal(&mut temporary, &values, width, height, blur_radius)?;
            } else {
                blur_horizontal(&mut values, &temporary, width, height, blur_radius)?;
            }
        }
        for pass in 0..3 {
            if pass % 2 == 0 {
                blur_vertical(&mut values, &temporary, width, height, blur_radius)?;
            } else {
                blur_vertical(&mut temporary, &values, width, height, blur_radius)?;
            }
        }
    }
    let max = values.iter().copied().reduce(f32::max).unwrap_or(0.) as f64 / (cell as f64).powi(2);
    Ok(DensityField {
        values,
        width,
        height,
        cell,
        offset,
        max,
    })
}
fn blur_horizontal(
    target: &mut [f32],
    source: &[f32],
    width: usize,
    height: usize,
    radius: f64,
) -> Result<()> {
    for y in 0..height {
        blur_line(target, source, y * width, (y + 1) * width, 1, radius)?;
    }
    Ok(())
}
fn blur_vertical(
    target: &mut [f32],
    source: &[f32],
    width: usize,
    height: usize,
    radius: f64,
) -> Result<()> {
    for x in 0..width {
        blur_line(target, source, x, x + width * height, width, radius)?;
    }
    Ok(())
}
fn blur_line(
    target: &mut [f32],
    source: &[f32],
    start: usize,
    stop: usize,
    step: usize,
    radius: f64,
) -> Result<()> {
    let stop = stop - step;
    let r = radius.floor() as usize;
    let t = radius - r as f64;
    let weight = 2. * radius + 1.;
    let s0 = step * r;
    let s1 = s0 + step;
    let mut sum = r as f64 * f64::from(source[start]);
    for i in (start..start + s0).step_by(step) {
        sum += f64::from(source[stop.min(i)]);
    }
    for i in (start..=stop).step_by(step) {
        sum += f64::from(source[stop.min(i + s0)]);
        let value = if t == 0. {
            sum / weight
        } else {
            (sum + t
                * (f64::from(source[start.max(i.saturating_sub(s1))])
                    + f64::from(source[stop.min(i + s1)])))
                / weight
        };
        let value = value as f32;
        ensure(value.is_finite(), "density Float32 blur overflow")?;
        target[i] = value;
        sum -= f64::from(source[start.max(i.saturating_sub(s0))]);
    }
    Ok(())
}
