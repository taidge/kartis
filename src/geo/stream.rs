//! Geographic line streams and major/minor graticules. Primary references:
//! d3-geo 3.1.1 graticule.js, clip/{circle,antimeridian}.js and
//! projection/{transverseMercator,resample}.js; ISC, Mike Bostock 2010-2024.
use super::{path::Sampler, *};
use crate::{ClipPath, ClipRule, DrawItem, StrokeDash, ValueFormat};

#[derive(Clone, Debug, PartialEq)]
pub struct GeoLineFeature {
    pub id: String,
    pub label: String,
    pub series: String,
    pub value: Option<f64>,
    /// Each component is an independent open line of minor great-circle arcs.
    pub lines: Vec<Vec<[f64; 2]>>,
    pub style: Option<GeoLineStyle>,
}
impl GeoLineFeature {
    pub fn new(id: impl Into<String>, positions: Vec<[f64; 2]>) -> Self {
        let id = id.into();
        Self {
            label: id.clone(),
            series: id.clone(),
            id,
            value: None,
            lines: vec![positions],
            style: None,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct GeoLineStyle {
    pub stroke: GraphicStroke,
    pub opacity: f64,
    pub dash: Option<StrokeDash>,
}
impl Default for GeoLineStyle {
    fn default() -> Self {
        Self {
            stroke: GraphicStroke {
                color: Color(50, 85, 140, 255),
                width: 1.5,
                cap: StrokeCap::Butt,
            },
            opacity: 1.,
            dash: None,
        }
    }
}
impl GeoLineStyle {
    fn validate(&self) -> Result<()> {
        ensure(
            self.stroke.width.is_finite()
                && (0.000001..=10000.).contains(&self.stroke.width)
                && self.opacity.is_finite()
                && (0. ..=1.).contains(&self.opacity),
            "invalid geographic line stroke or opacity",
        )?;
        if let Some(d) = &self.dash {
            d.validate()?;
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct GeoStreamOptions {
    /// None inherits the adaptive polygon context, or defaults to 0.5px.
    pub path: Option<GeoPathOptions>,
    pub style: GeoLineStyle,
    pub interactive: bool,
    pub clip_plot: bool,
    pub clip_projection: bool,
    pub behind: bool,
    pub value_format: ValueFormat,
}
impl Default for GeoStreamOptions {
    fn default() -> Self {
        Self {
            path: None,
            style: Default::default(),
            interactive: true,
            clip_plot: true,
            clip_projection: true,
            behind: false,
            value_format: ValueFormat::Auto,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ComputedGeoLine {
    pub id: String,
    pub mark_id: String,
    pub series: String,
    pub value: Option<f64>,
    pub geometry: Option<Geometry>,
    pub components: usize,
    pub visible: bool,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GeoStreamMetadata {
    pub lines: Vec<ComputedGeoLine>,
    pub sampling: GeoPathStats,
}
#[derive(Clone, Debug, PartialEq)]
pub struct GeoStreamLayout {
    pub scene: Scene,
    pub projection: GeoProjectionContext,
    pub metadata: GeoStreamMetadata,
}

#[derive(Clone, Copy)]
struct Sample {
    v: V,
    p: [f64; 2],
}
impl Sample {
    fn raw(self, projection: GeoProjection) -> [f64; 2] {
        if projection.hemisphere() {
            raw_vector(self.v, projection)
        } else {
            raw(self.p, projection)
        }
    }
}
fn at(a: V, tangent: V, t: f64) -> V {
    V {
        x: a.x * t.cos() + tangent.x * t.sin(),
        y: a.y * t.cos() + tangent.y * t.sin(),
        z: a.z * t.cos() + tangent.z * t.sin(),
    }
}
fn roots(a: f64, b: f64, level: f64, theta: f64, cuts: &mut Vec<f64>) {
    let r = a.hypot(b);
    if r <= 1e-15 || level.abs() > r + 1e-14 {
        return;
    }
    let phase = b.atan2(a);
    let offset = (level / r).clamp(-1., 1.).acos();
    for sign in [-1., 1.] {
        for turn in -1..=1 {
            let t = phase + sign * offset + f64::from(turn) * TAU;
            if t > 0. && t < theta {
                cuts.push(t);
            }
        }
    }
}
fn sample(v: V, hint: f64, boundary: bool) -> Sample {
    let mut p = spherical(v);
    if boundary && v.x.hypot(v.y) < 1e-12 {
        p[0] = hint;
    } else if boundary && v.x < 0. && v.y.abs() < 1e-12 {
        p[0] = if hint < 0. { -PI } else { PI };
    }
    Sample { v, p }
}
fn pieces(a: V, b: V, o: &GeoMapOptions) -> Result<Vec<Vec<Sample>>> {
    let cosine = a.dot(b).clamp(-1., 1.);
    let theta = a.cross(b).norm().atan2(cosine);
    ensure(
        theta < PI - 1e-8,
        "antipodal geographic line edge is ambiguous",
    )?;
    if theta == 0. {
        return Ok(vec![]);
    }
    let tangent = V {
        x: (b.x - a.x * cosine) / theta.sin(),
        y: (b.y - a.y * cosine) / theta.sin(),
        z: (b.z - a.z * cosine) / theta.sin(),
    };
    let mut cuts = vec![0., theta];
    if o.projection.hemisphere() {
        roots(a.x, tangent.x, clip_angle(o).cos(), theta, &mut cuts);
    } else {
        roots(a.y, tangent.y, 0., theta, &mut cuts);
        // Poles have no unique longitude. Split there and retain the longitude
        // of each adjacent arc; never connect opposite sides of a flat map.
        roots(a.x, tangent.x, 0., theta, &mut cuts);
        if o.projection.latitude_bounded() {
            roots(a.z, tangent.z, MERCATOR_LIMIT.sin(), theta, &mut cuts);
            roots(a.z, tangent.z, -MERCATOR_LIMIT.sin(), theta, &mut cuts);
        }
    }
    cuts.sort_by(f64::total_cmp);
    cuts.dedup_by(|a, b| (*a - *b).abs() < theta * 1e-12);
    let mut out = vec![];
    for pair in cuts.windows(2) {
        let mid = at(a, tangent, (pair[0] + pair[1]) / 2.);
        let visible = visible_vector(mid, o);
        if !visible {
            continue;
        }
        let n = ((pair[1] - pair[0]) / PRECISION).ceil().max(1.) as usize;
        let mut line = vec![];
        for i in 0..=n {
            let t = pair[0] + (pair[1] - pair[0]) * i as f64 / n as f64;
            let v = at(a, tangent, t);
            let hint = spherical(mid)[0];
            line.push(sample(
                v,
                hint,
                (i == 0 && pair[0] > 0.) || (i == n && pair[1] < theta),
            ));
        }
        out.push(line);
    }
    Ok(out)
}
type RawLines = Vec<Vec<Vec<Sample>>>;
fn source(data: &[GeoLineFeature], o: &GeoMapOptions, limit: usize) -> Result<RawLines> {
    ensure(
        data.len() <= 4096,
        "geographic line feature budget exceeded",
    )?;
    let mut ids = BTreeSet::new();
    let mut count = 0usize;
    let mut generated = 0usize;
    let mut output = vec![];
    for f in data {
        ensure(
            !f.id.trim().is_empty()
                && f.id.len() <= 1000
                && f.label.len() <= 1024
                && f.series.len() <= 1024
                && f.value.is_none_or(f64::is_finite)
                && ids.insert(&f.id),
            "invalid geographic line identity/value",
        )?;
        if let Some(s) = &f.style {
            s.validate()?;
        }
        ensure(
            f.lines.len() <= 4096,
            "geographic line component budget exceeded",
        )?;
        let mut result: Vec<Vec<Sample>> = vec![];
        for line in &f.lines {
            ensure(
                line.len() >= 2,
                "geographic line requires at least two positions",
            )?;
            count = count
                .checked_add(line.len())
                .ok_or_else(|| ChartError("line point count overflow".into()))?;
            ensure(
                count <= MAX_POINTS,
                "geographic line source budget exceeded",
            )?;
            for p in line {
                valid_position(*p)?;
            }
            let mut current: Vec<Sample> = vec![];
            for pair in line.windows(2) {
                if pair[0] == pair[1]
                    || (pair[0][1] == pair[1][1]
                        && ((pair[0][0] - pair[1][0]).abs() == 360. || pair[0][1].abs() == 90.))
                {
                    continue;
                }
                let a = rotate(pair[0], o);
                let b = rotate(pair[1], o);
                ensure(
                    a.cross(b).norm() > 0. || a.dot(b) < 0.,
                    "distinct geographic line positions collapse in spherical rotation",
                )?;
                let fragments = pieces(a, b, o)?;
                for fragment in fragments {
                    generated += fragment.len();
                    ensure(
                        generated <= limit,
                        "geographic line coarse point budget exceeded",
                    )?;
                    let connected = current.last().is_some_and(|last| {
                        let a = last.raw(o.projection);
                        let b = fragment[0].raw(o.projection);
                        (a[0] - b[0]).hypot(a[1] - b[1]) <= 1e-11
                    });
                    if !connected && !current.is_empty() {
                        result.push(std::mem::take(&mut current));
                    }
                    current.extend(fragment.into_iter().skip(usize::from(connected)));
                }
                let end = rotate(pair[1], o);
                let end_visible = visible_vector(end, o);
                if !end_visible && !current.is_empty() {
                    result.push(std::mem::take(&mut current));
                }
            }
            if !current.is_empty() {
                result.push(current);
            }
        }
        output.push(result);
    }
    Ok(output)
}
fn projected(
    data: &RawLines,
    projection: GeoProjection,
    scale: f64,
    options: &GeoPathOptions,
    capture_fit: bool,
) -> Result<(Vec<Vec<Ring>>, GeoPathStats, [f64; 4])> {
    let mut sampler = Sampler::new(options, scale, projection)?;
    sampler.capture_fit = capture_fit && projection.conic_kernel().is_some();
    let mut output = vec![];
    for feature in data {
        let mut paths = vec![];
        for line in feature {
            if line.len() < 2 {
                continue;
            }
            let mut points = vec![];
            sampler.start(&mut points, line[0].raw(projection))?;
            for pair in line.windows(2) {
                let a = pair[0];
                let b = pair[1];
                if projection.conic_kernel().is_some() {
                    let geo = |t| {
                        let v = interpolate_arc(a.v, b.v, t)?;
                        let hint = a.p[0] + (b.p[0] - a.p[0]) * t;
                        let mut p = spherical(v);
                        while p[0] - hint > PI {
                            p[0] -= TAU;
                        }
                        while p[0] - hint < -PI {
                            p[0] += TAU;
                        }
                        if v.x.hypot(v.y) < 1e-12 {
                            p[0] = hint;
                        }
                        Ok(p)
                    };
                    sampler.segment_geo(
                        a.raw(projection),
                        b.raw(projection),
                        &geo,
                        false,
                        a.v.cross(b.v).norm().atan2(a.v.dot(b.v)),
                        &mut points,
                    )?;
                    continue;
                }
                let curve = |t: f64| -> Result<[f64; 2]> {
                    let v = interpolate_arc(a.v, b.v, t)?;
                    let hint = a.p[0] + (b.p[0] - a.p[0]) * t;
                    let mut p = spherical(v);
                    while p[0] - hint > PI {
                        p[0] -= TAU;
                    }
                    while p[0] - hint < -PI {
                        p[0] += TAU;
                    }
                    if v.x.hypot(v.y) < 1e-12 {
                        p[0] = hint;
                    }
                    Ok(Sample { v, p }.raw(projection))
                };
                sampler.segment(
                    a.raw(projection),
                    b.raw(projection),
                    &curve,
                    a.v.cross(b.v).norm().atan2(a.v.dot(b.v)),
                    &mut points,
                )?;
            }
            paths.push(points);
        }
        output.push(paths);
    }
    Ok((output, sampler.stats, sampler.fit_bounds))
}
fn bounds(data: &RawLines, projection: GeoProjection) -> [f64; 4] {
    if projection.conic_kernel().is_some() {
        let mut b = [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ];
        for line in data.iter().flatten() {
            for pair in line.windows(2) {
                if let Some(next) = projection.conic_arc_bounds(pair[0].p, pair[1].p, false) {
                    merge_geo_bounds(&mut b, next);
                }
            }
        }
        return b;
    }
    point_bounds(data, projection)
}
fn point_bounds(data: &RawLines, projection: GeoProjection) -> [f64; 4] {
    data.iter()
        .flatten()
        .flatten()
        .map(|s| s.raw(projection))
        .fold(
            [
                f64::INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
            ],
            |b, p| {
                [
                    b[0].min(p[0]),
                    b[1].min(p[1]),
                    b[2].max(p[0]),
                    b[3].max(p[1]),
                ]
            },
        )
}
impl GeoProjectionContext {
    fn disk_covers_plot(&self, d: GeoProjectionDisk) -> bool {
        // Keep ordinary valid masks, including their antialiasing behavior.
        // This optimization only rescues a redundant disk outside mask bounds.
        if (ClipPath {
            geometry: Geometry::Circle {
                x: d.center[0],
                y: d.center[1],
                radius: d.radius,
            },
            rule: ClipRule::NonZero,
        })
        .validate()
        .is_ok()
        {
            return false;
        }
        let r = self.plot;
        let magnitude = [
            d.radius,
            d.center[0].abs(),
            d.center[1].abs(),
            self.scale.abs(),
            r.x.abs(),
            r.y.abs(),
            (r.x + r.width).abs(),
            (r.y + r.height).abs(),
            1.,
        ]
        .into_iter()
        .fold(0., f64::max);
        // Reserve many ulps for fitted-center cancellation, corner subtraction
        // and hypot. Touching/uncertain cases retain the actual circle mask.
        let guard = (magnitude * (64. * f64::EPSILON)).max(2.);
        d.radius.is_finite()
            && d.radius > guard
            && [r.x, r.x + r.width].into_iter().all(|x| {
                [r.y, r.y + r.height]
                    .into_iter()
                    .all(|y| (x - d.center[0]).hypot(y - d.center[1]) <= d.radius - guard)
            })
    }
    pub fn path_options(&self) -> Option<&GeoPathOptions> {
        self.path_options.as_ref()
    }
    /// Transactional overlay: fit/center/rotation/scale/translate are reused.
    pub fn add_lines(
        &self,
        scene: &mut Scene,
        data: &[GeoLineFeature],
        o: &GeoStreamOptions,
    ) -> Result<GeoStreamMetadata> {
        scene.validate()?;
        ensure(
            [scene.width, scene.height] == self.dimensions,
            "geographic line Scene dimensions differ from projection",
        )?;
        o.style.validate()?;
        o.value_format.validate()?;
        let mut path = o
            .path
            .as_ref()
            .or(self.path_options.as_ref())
            .cloned()
            .unwrap_or_default();
        path.validate()?;
        let used = scene
            .marks
            .iter()
            .map(|m| {
                if let Geometry::Path(p) = &m.geometry {
                    p.len()
                } else {
                    0
                }
            })
            .sum::<usize>();
        let remaining = 200000usize.saturating_sub(used);
        ensure(
            remaining >= 2,
            "geographic overlay command budget exhausted",
        )?;
        path.max_points = path.max_points.min(remaining);
        let raw = source(data, &self.view, path.max_points)?;
        let (paths, sampling, _) = projected(&raw, self.view.projection, self.scale, &path, false)?;
        self.append_projected(scene, data, o, paths, sampling)
    }
    fn append_projected(
        &self,
        scene: &mut Scene,
        data: &[GeoLineFeature],
        o: &GeoStreamOptions,
        paths: Vec<Vec<Ring>>,
        sampling: GeoPathStats,
    ) -> Result<GeoStreamMetadata> {
        let mut next = scene.clone();
        let mark_start = next.marks.len();
        let old_order = next.draw_items();
        let explicit = !next.draw_order.is_empty();
        let mut lines = vec![];
        for (f, paths) in data.iter().zip(paths) {
            let mut commands = vec![];
            let components = paths.len();
            for p in paths {
                ensure(
                    p.windows(2).any(|pair| pair[0] != pair[1]),
                    "geographic line collapses in projected coordinates",
                )?;
                if let Some(first) = p.first() {
                    let q = self.map_raw(*first);
                    commands.push(Cmd::Move(P::new(q[0], q[1])));
                    commands.extend(p.iter().skip(1).map(|p| {
                        let p = self.map_raw(*p);
                        Cmd::Line(P::new(p[0], p[1]))
                    }));
                }
            }
            let mark_id = format!("geo-line:{}", f.id);
            let geometry = (!commands.is_empty()).then_some(Geometry::Path(commands));
            let style = f.style.as_ref().unwrap_or(&o.style);
            if let Some(geometry) = &geometry {
                let mut m = Mark::new(&mark_id, geometry.clone(), style.stroke.color);
                m.fill = None;
                m.stroke = Some(style.stroke);
                m.stroke_dash = style.dash.clone();
                m.opacity = style.opacity;
                m.series = f.series.clone();
                m.value = f.value.unwrap_or(0.);
                m.label = if o.interactive {
                    if let Some(v) = f.value {
                        format!("{}: {}", f.label, o.value_format.format(v)?)
                    } else {
                        f.label.clone()
                    }
                } else {
                    String::new()
                };
                if o.clip_plot {
                    m.clip = Some(self.plot);
                }
                if o.clip_projection
                    && let Some(d) = self.projection_disk()
                    && !(o.clip_plot && self.disk_covers_plot(d))
                {
                    m.clip_path = Some(Box::new(ClipPath {
                        geometry: Geometry::Circle {
                            x: d.center[0],
                            y: d.center[1],
                            radius: d.radius,
                        },
                        rule: ClipRule::NonZero,
                    }));
                }
                next.marks.push(m);
            }
            lines.push(ComputedGeoLine {
                id: f.id.clone(),
                mark_id,
                series: f.series.clone(),
                value: f.value,
                visible: geometry.is_some(),
                geometry,
                components,
            });
        }
        ensure(
            next.marks.len() <= crate::MAX_MARKS,
            "geographic overlay mark budget exceeded",
        )?;
        if o.behind {
            next.draw_order = (mark_start..next.marks.len())
                .map(DrawItem::Mark)
                .chain(old_order)
                .collect();
        } else if explicit {
            next.draw_order
                .extend((mark_start..next.marks.len()).map(DrawItem::Mark));
        }
        next.validate()?;
        *scene = next;
        Ok(GeoStreamMetadata { lines, sampling })
    }
}
impl GeoMapLayout {
    pub fn add_lines(
        &mut self,
        data: &[GeoLineFeature],
        o: &GeoStreamOptions,
    ) -> Result<GeoStreamMetadata> {
        self.projection.add_lines(&mut self.scene, data, o)
    }
}
/// Standalone lines. A conservative coarse fit is refined in screen space,
/// then every refined point participates in the final Data fit. The scale can
/// only decrease, preserving the error criterion. Overlays never refit a map.
pub fn geographic_lines(
    c: &ChartConfig,
    data: &[GeoLineFeature],
    view: &GeoMapOptions,
    o: &GeoStreamOptions,
) -> Result<GeoStreamLayout> {
    let mut scene = c.scene()?;
    options(view)?;
    ensure(
        view.padding * 2. < c.inner_width().min(c.inner_height()),
        "geographic line padding exhausts viewport",
    )?;
    let path = o.path.clone().unwrap_or_default();
    path.validate()?;
    o.style.validate()?;
    o.value_format.validate()?;
    let raw = source(data, view, path.max_points)?;
    let coarse = fitted_context(c, view, bounds(&raw, view.projection), Some(&path))?;
    let sampling_scale = if view.projection.conic_kernel().is_some() {
        fitted_context(c, view, point_bounds(&raw, view.projection), Some(&path))?
            .scale
            .max(coarse.scale)
    } else {
        coarse.scale
    };
    let (paths, sampling, leaf_bounds) =
        projected(&raw, view.projection, sampling_scale, &path, true)?;
    let mut refined = paths.iter().flatten().flatten().fold(
        [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ],
        |b, p| {
            [
                b[0].min(p[0]),
                b[1].min(p[1]),
                b[2].max(p[0]),
                b[3].max(p[1]),
            ]
        },
    );
    if view.projection.conic_kernel().is_some() {
        merge_geo_bounds(&mut refined, leaf_bounds);
    }
    let projection = fitted_context(c, view, refined, Some(&path))?;
    let metadata = projection.append_projected(&mut scene, data, o, paths, sampling)?;
    Ok(GeoStreamLayout {
        scene,
        projection,
        metadata,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeoGraticuleAxis {
    Meridian,
    Parallel,
}
#[derive(Clone, Debug, PartialEq)]
pub struct GeoGraticuleLine {
    pub feature: GeoLineFeature,
    pub axis: GeoGraticuleAxis,
    pub coordinate: f64,
    pub major: bool,
}
#[derive(Clone, Debug, PartialEq)]
pub struct GeoGraticuleOptions {
    pub extent_major: [[f64; 2]; 2],
    pub extent_minor: [[f64; 2]; 2],
    pub step_major: [f64; 2],
    pub step_minor: [f64; 2],
    /// Source parallel polyline step in degrees, as d3.geoGraticule.precision.
    /// Independent from the projected-pixel GeoPathOptions error criterion.
    pub precision: f64,
}
impl Default for GeoGraticuleOptions {
    fn default() -> Self {
        Self {
            extent_major: [[-180., -89.999999], [180., 89.999999]],
            extent_minor: [[-180., -80.000001], [180., 80.000001]],
            step_major: [90., 360.],
            step_minor: [10., 10.],
            precision: 2.5,
        }
    }
}
pub fn geo_graticule(o: &GeoGraticuleOptions) -> Result<Vec<GeoGraticuleLine>> {
    for e in [o.extent_major, o.extent_minor] {
        valid_position(e[0])?;
        valid_position(e[1])?;
        ensure(
            e[0][0] < e[1][0] && e[0][1] < e[1][1],
            "graticule extents must ascend",
        )?;
    }
    ensure(
        o.step_major
            .into_iter()
            .chain(o.step_minor)
            .all(|v| v.is_finite() && (0.1..=360.).contains(&v))
            && o.precision.is_finite()
            && (0.01..=90.).contains(&o.precision),
        "invalid graticule step or precision",
    )?;
    let mut out = vec![];
    let mut count = 0usize;
    for (major, extent, step) in [
        (true, o.extent_major, o.step_major),
        (false, o.extent_minor, o.step_minor),
    ] {
        for axis in [GeoGraticuleAxis::Meridian, GeoGraticuleAxis::Parallel] {
            let dimension = usize::from(axis == GeoGraticuleAxis::Parallel);
            let first = (extent[0][dimension] / step[dimension]).ceil() as i32;
            let last = (extent[1][dimension] / step[dimension]).ceil() as i32;
            ensure(last - first <= 4096, "graticule line budget exceeded")?;
            for i in first..last {
                let coordinate = i as f64 * step[dimension];
                if !major
                    && coordinate
                        .rem_euclid(o.step_major[dimension])
                        .min((-coordinate).rem_euclid(o.step_major[dimension]))
                        < 1e-6
                {
                    continue;
                }
                let d = 1 - dimension;
                let spacing = if axis == GeoGraticuleAxis::Meridian {
                    90.
                } else {
                    o.precision
                };
                let n = ((extent[1][d] - extent[0][d] - 1e-6) / spacing).ceil() as usize;
                count = count.saturating_add(n + 1);
                ensure(
                    count <= MAX_POINTS && out.len() < 4096,
                    "graticule source budget exceeded",
                )?;
                let mut points = vec![];
                for j in 0..n {
                    let v = extent[0][d] + j as f64 * spacing;
                    points.push(if dimension == 0 {
                        [coordinate, v]
                    } else {
                        [v, coordinate]
                    });
                }
                points.push(if dimension == 0 {
                    [coordinate, extent[1][d]]
                } else {
                    [extent[1][d], coordinate]
                });
                let bit = if coordinate == 0. {
                    0
                } else {
                    coordinate.to_bits()
                };
                let id = format!(
                    "graticule:{}:{}:{bit:016x}",
                    if major { "major" } else { "minor" },
                    if dimension == 0 {
                        "meridian"
                    } else {
                        "parallel"
                    }
                );
                out.push(GeoGraticuleLine {
                    feature: GeoLineFeature::new(id, points),
                    axis,
                    coordinate,
                    major,
                });
            }
        }
    }
    Ok(out)
}
impl GeoProjectionContext {
    pub fn add_graticule(
        &self,
        scene: &mut Scene,
        grid: &GeoGraticuleOptions,
        o: &GeoStreamOptions,
    ) -> Result<GeoStreamMetadata> {
        let data = geo_graticule(grid)?
            .into_iter()
            .map(|g| g.feature)
            .collect::<Vec<_>>();
        let mut options = o.clone();
        options.interactive = false;
        self.add_lines(scene, &data, &options)
    }
}
impl GeoMapLayout {
    pub fn add_graticule(
        &mut self,
        grid: &GeoGraticuleOptions,
        o: &GeoStreamOptions,
    ) -> Result<GeoStreamMetadata> {
        self.projection.add_graticule(&mut self.scene, grid, o)
    }
}
