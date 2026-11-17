//! Three-region polygon streaming and fitting. Postclip contours retain their
//! native nonzero winding; list position never assigns exterior/hole roles.
use super::*;

const WORLD: [f64; 4] = [-0.455, -0.238, 0.455, 0.238];

#[derive(Clone, Debug, PartialEq)]
pub struct GeoUsaMapOptions {
    pub fit: GeoFit,
    pub padding: f64,
    pub scale: Option<f64>,
    /// Pixel position of the fitted bounds' center, matching GeoMapOptions.
    /// With World fit its normalized center is zero and this is the D3 translate.
    pub translate: Option<[f64; 2]>,
    pub fill: Option<Color>,
    pub border: Option<Color>,
    pub border_width: f64,
    pub path: GeoPathOptions,
}
impl Default for GeoUsaMapOptions {
    fn default() -> Self {
        let old = GeoMapOptions::default();
        Self {
            fit: old.fit,
            padding: old.padding,
            scale: old.scale,
            translate: old.translate,
            fill: old.fill,
            border: old.border,
            border_width: old.border_width,
            path: Default::default(),
        }
    }
}

/// Shared admission across every component and the optional fitting pass.
/// Points include adaptive emissions and postclip output. Work includes a
/// conservative preparation/topology reservation, adaptive evaluations, and
/// actual rectangle clipping. Neither counter resets per region or pass.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GeoUsaMapStats {
    pub points: usize,
    pub work: usize,
    pub deepest: u8,
    pub passes: usize,
}

struct Budget<'a> {
    path: &'a GeoPathOptions,
    stats: GeoUsaMapStats,
}
impl Budget<'_> {
    fn work(&mut self, amount: usize) -> Result<()> {
        self.stats.work = self
            .stats
            .work
            .checked_add(amount)
            .ok_or_else(|| ChartError("USA geographic work overflow".into()))?;
        ensure(
            self.stats.work <= self.path.max_work,
            "USA geographic shared work budget exceeded",
        )
    }
    fn sampler(&self, scale: f64, projection: GeoProjection) -> Result<path::Sampler<'_>> {
        let mut sampler = path::Sampler::new(self.path, scale, projection)?;
        sampler.stats = GeoPathStats {
            points: self.stats.points,
            work: self.stats.work,
            deepest: self.stats.deepest,
        };
        Ok(sampler)
    }
    fn absorb_sampling(&mut self, stats: GeoPathStats) {
        self.stats.points = stats.points;
        self.stats.work = stats.work;
        self.stats.deepest = stats.deepest;
    }
    fn clip_options(&self) -> Result<GeoRectClipOptions> {
        ensure(
            self.stats.points < self.path.max_points && self.stats.work < self.path.max_work,
            "USA geographic shared clipping budget exhausted",
        )?;
        Ok(GeoRectClipOptions {
            max_points: self.path.max_points - self.stats.points,
            max_work: self.path.max_work - self.stats.work,
        })
    }
    fn absorb_clip(&mut self, stats: GeoRectClipStats) -> Result<()> {
        self.stats.points = self
            .stats
            .points
            .checked_add(stats.output_points)
            .ok_or_else(|| ChartError("USA geographic point overflow".into()))?;
        ensure(
            self.stats.points <= self.path.max_points,
            "USA geographic shared point budget exceeded",
        )?;
        self.work(stats.work)
    }
}

fn view(o: &GeoUsaMapOptions) -> GeoMapOptions {
    GeoMapOptions {
        projection: GeoProjection::Albers,
        fit: o.fit,
        padding: o.padding,
        scale: o.scale,
        translate: o.translate,
        fill: o.fill,
        border: o.border,
        border_width: o.border_width,
        ..Default::default()
    }
}

fn context(
    c: &ChartConfig,
    o: &GeoUsaMapOptions,
    mut bounds: [f64; 4],
) -> Result<GeoProjectionContext> {
    if bounds[0] == f64::INFINITY {
        bounds = WORLD;
    }
    let mut v = view(o);
    // The existing affine fitter is reused with explicit composite bounds.
    // Its ordinary World branch must not substitute an Albers cone footprint.
    v.fit = GeoFit::Data;
    let mut result = fitted_context(c, &v, bounds, Some(&o.path))?;
    result.view.fit = o.fit;
    let native_translate = [
        result.translate[0] - result.center[0] * result.scale,
        result.translate[1] + result.center[1] * result.scale,
    ];
    result.usa = Some(GeoAlbersUsaProjection::new(result.scale, native_translate)?);
    Ok(result)
}

fn validate_source(features: &[GeoFeature], budget: &mut Budget<'_>) -> Result<([f64; 2], bool)> {
    ensure(
        features.len() <= 4096,
        "USA geographic feature budget exceeded",
    )?;
    let mut ids = BTreeSet::new();
    let mut count = 0usize;
    let mut values = [f64::INFINITY, f64::NEG_INFINITY];
    let mut geometry = false;
    for feature in features {
        budget.work(1)?;
        ensure(
            !feature.id.trim().is_empty()
                && feature.id.len() <= 1000
                && feature.label.len() <= 1024
                && ids.insert(&feature.id)
                && feature.value.is_none_or(f64::is_finite),
            "invalid USA geographic feature identity/value",
        )?;
        if let Some(value) = feature.value {
            values[0] = values[0].min(value);
            values[1] = values[1].max(value);
        }
        for polygon in &feature.polygons {
            budget.work(1)?;
            ensure(!polygon.is_empty(), "USA polygon needs an exterior ring")?;
            geometry = true;
            for ring in polygon {
                budget.work(1)?;
                ensure(
                    (3..=4096).contains(&ring.len()),
                    "USA geographic rings require 3..=4096 points",
                )?;
                count = count
                    .checked_add(ring.len())
                    .ok_or_else(|| ChartError("USA geographic source point overflow".into()))?;
                ensure(
                    count <= MAX_POINTS,
                    "USA geographic source point budget exceeded",
                )?;
                budget.work(ring.len())?;
                for point in ring {
                    valid_position(*point)?;
                }
            }
        }
    }
    ensure(
        values[0] == f64::INFINITY || (values[1] - values[0]).is_finite(),
        "USA geographic color domain span overflow",
    )?;
    Ok((values, geometry))
}

fn preparation_work(rings: &[Vec<V>]) -> Result<usize> {
    let mut lengths = Vec::with_capacity(rings.len());
    for ring in rings {
        let mut length = 1usize;
        for i in 0..ring.len() {
            let a = ring[i];
            let b = ring[(i + 1) % ring.len()];
            let angle = a.cross(b).norm().atan2(a.dot(b));
            // Existing two-degree sampling can add latitude extrema as well
            // as regular fractions. Reserve both before unwrapping allocates.
            let steps = (angle / PRECISION).ceil().max(1.) as usize;
            length = length
                .checked_add(steps + 2)
                .ok_or_else(|| ChartError("USA geographic preparation overflow".into()))?;
        }
        lengths.push(length);
    }
    let mut work = lengths
        .iter()
        .try_fold(0usize, |sum, n| sum.checked_add(*n))
        .ok_or_else(|| ChartError("USA geographic preparation overflow".into()))?;
    for hole in lengths.iter().skip(1) {
        work = work
            .checked_add(
                lengths[0]
                    .checked_mul(*hole)
                    .ok_or_else(|| ChartError("USA geographic containment overflow".into()))?,
            )
            .ok_or_else(|| ChartError("USA geographic containment overflow".into()))?;
    }
    Ok(work)
}

fn projected(
    features: &[GeoFeature],
    projection: &GeoProjectionContext,
    budget: &mut Budget<'_>,
) -> Result<Vec<Vec<Ring>>> {
    budget.work(1)?;
    budget.stats.passes += 1;
    let usa = projection.usa.unwrap();
    let children = GeoUsaRegion::ALL
        .map(|region| projection.usa_component(region))
        .into_iter()
        .collect::<Result<Vec<_>>>()?;
    let mut out = Vec::with_capacity(features.len());
    for feature in features {
        budget.work(1)?;
        let mut contours = vec![];
        for polygon in &feature.polygons {
            for (region, child) in GeoUsaRegion::ALL.into_iter().zip(&children) {
                budget.work(1 + polygon.len())?;
                let rings = polygon
                    .iter()
                    .map(|ring| vectors(ring, &child.view))
                    .collect::<Result<Vec<_>>>()?;
                budget.work(preparation_work(&rings)?)?;
                let (pieces, stats) = {
                    let mut sampler = budget.sampler(child.scale, child.view.projection)?;
                    let mut ignored = [
                        f64::INFINITY,
                        f64::INFINITY,
                        f64::NEG_INFINITY,
                        f64::NEG_INFINITY,
                    ];
                    let pieces = planar_polygon(
                        &rings,
                        child.view.projection,
                        Some(&mut sampler),
                        &mut ignored,
                    )?;
                    (pieces, sampler.stats)
                };
                budget.absorb_sampling(stats);
                for mut piece in pieces {
                    let dx = child.translate[0] - child.scale * child.center[0];
                    let dy = child.translate[1] + child.scale * child.center[1];
                    for ring in &mut piece {
                        for p in ring {
                            *p = [dx + child.scale * p[0], dy - child.scale * p[1]];
                        }
                    }
                    let result = clip_geo_planar_polygon(
                        &piece,
                        usa.region_clip(region),
                        &budget.clip_options()?,
                    )?;
                    budget.absorb_clip(result.stats)?;
                    contours.extend(result.rings);
                }
            }
        }
        out.push(contours);
    }
    Ok(out)
}

fn data_bounds(polygons: &[Vec<Ring>]) -> [f64; 4] {
    let mut bounds = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for p in polygons.iter().flatten().flatten() {
        merge_geo_bounds(
            &mut bounds,
            [p[0] / 150., -p[1] / 150., p[0] / 150., -p[1] / 150.],
        );
    }
    bounds
}

fn render_usa(
    c: &ChartConfig,
    features: &[GeoFeature],
    o: &GeoUsaMapOptions,
    choropleth: bool,
) -> Result<GeoMapLayout> {
    let mut scene = c.scene()?;
    o.path.validate()?;
    options(&view(o))?;
    ensure(
        o.padding * 2. < c.inner_width().min(c.inner_height()),
        "USA geographic padding exhausts viewport",
    )?;
    let mut budget = Budget {
        path: &o.path,
        stats: Default::default(),
    };
    let (values, has_geometry) = validate_source(features, &mut budget)?;
    let bounds = if o.fit == GeoFit::Data && has_geometry {
        let temporary = GeoUsaMapOptions {
            fit: GeoFit::World,
            padding: 0.,
            scale: Some(150.),
            translate: Some([0., 0.]),
            ..o.clone()
        };
        data_bounds(&projected(
            features,
            &context(c, &temporary, WORLD)?,
            &mut budget,
        )?)
    } else {
        WORLD
    };
    let mut projection = context(c, o, bounds)?;
    let data = projected(features, &projection, &mut budget)?;
    let mut command_count = 0usize;
    for (feature, rings) in features.iter().zip(data) {
        let mut commands = vec![];
        for ring in rings {
            if let Some(first) = ring.first() {
                command_count = command_count
                    .checked_add(ring.len() + 1)
                    .ok_or_else(|| ChartError("USA map command count overflow".into()))?;
                ensure(command_count <= 200_000, "USA map command budget exceeded")?;
                commands.push(Cmd::Move(P::new(first[0], first[1])));
                commands.extend(ring.iter().skip(1).map(|p| Cmd::Line(P::new(p[0], p[1]))));
                commands.push(Cmd::Close);
            }
        }
        if commands.is_empty() {
            continue;
        }
        let fill = if choropleth {
            feature
                .value
                .map(|value| {
                    let t = if values[0] == values[1] {
                        0.5
                    } else {
                        (value - values[0]) / (values[1] - values[0])
                    };
                    Color(
                        (230. - 190. * t) as u8,
                        (240. - 120. * t) as u8,
                        (250. - 60. * t) as u8,
                        255,
                    )
                })
                .or(o.fill)
                .unwrap_or(c.theme.grid)
        } else {
            o.fill.unwrap_or(c.theme.grid)
        };
        let mut mark = Mark::new(
            format!("map:{}", feature.id),
            Geometry::Path(commands),
            fill,
        );
        mark.series = feature.id.clone();
        mark.label = feature.label.clone();
        mark.value = feature.value.unwrap_or(0.);
        if let Some(color) = o.border
            && o.border_width > 0.
        {
            mark.stroke = Some(GraphicStroke {
                color,
                width: o.border_width,
                cap: StrokeCap::Butt,
            });
        }
        scene.marks.push(mark);
    }
    scene.validate()?;
    projection.usa_stats = Some(budget.stats);
    Ok(GeoMapLayout { scene, projection })
}

pub fn geographic_usa_map_layout(
    c: &ChartConfig,
    features: &[GeoFeature],
    o: &GeoUsaMapOptions,
) -> Result<GeoMapLayout> {
    render_usa(c, features, o, false)
}
pub fn choropleth_usa_layout(
    c: &ChartConfig,
    features: &[GeoFeature],
    o: &GeoUsaMapOptions,
) -> Result<GeoMapLayout> {
    render_usa(c, features, o, true)
}
pub fn geographic_usa_map(
    c: &ChartConfig,
    features: &[GeoFeature],
    o: &GeoUsaMapOptions,
) -> Result<Scene> {
    Ok(geographic_usa_map_layout(c, features, o)?.scene)
}
pub fn choropleth_usa_map(
    c: &ChartConfig,
    features: &[GeoFeature],
    o: &GeoUsaMapOptions,
) -> Result<Scene> {
    Ok(choropleth_usa_layout(c, features, o)?.scene)
}
