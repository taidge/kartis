//! Shared polar ticks and real Scene geometry. References: local Nivo
//! packages/polar-axes/src/{RadialAxis,CircularAxis,RadialGrid,CircularGrid}.tsx.
//! Angles are unwrapped radians, clockwise from twelve o'clock; either winding
//! is accepted. Radial ticks carry radii; circular ticks carry angles.
use crate::{scene::ensure, *};
use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as C, StrokeCap};
use std::{
    collections::{BTreeMap, BTreeSet},
    f64::consts::{FRAC_PI_2, TAU},
};
pub const MAX_POLAR_TICKS: usize = 1000;
#[derive(Clone, Debug, PartialEq)]
pub struct PolarFrame {
    pub center: [f64; 2],
    pub inner_radius: f64,
    pub outer_radius: f64,
    pub start_angle: f64,
    pub end_angle: f64,
}
impl PolarFrame {
    pub fn from_config(
        c: &ChartConfig,
        inner_ratio: f64,
        start_angle: f64,
        end_angle: f64,
    ) -> Result<Self> {
        c.validate()?;
        ensure(
            inner_ratio.is_finite() && (0. ..1.).contains(&inner_ratio),
            "invalid polar inner ratio",
        )?;
        let outer = c.inner_width().min(c.inner_height()) / 2.;
        let frame = Self {
            center: [
                c.margin.left + c.inner_width() / 2.,
                c.margin.top + c.inner_height() / 2.,
            ],
            inner_radius: outer * inner_ratio,
            outer_radius: outer,
            start_angle,
            end_angle,
        };
        frame.validate()?;
        Ok(frame)
    }
    pub fn validate(&self) -> Result<()> {
        ensure(
            self.center
                .iter()
                .all(|v| v.is_finite() && v.abs() <= 900000.)
                && self.inner_radius.is_finite()
                && self.outer_radius.is_finite()
                && self.inner_radius >= 0.
                && self.outer_radius > self.inner_radius
                && self.outer_radius <= 16384.
                && [self.start_angle, self.end_angle]
                    .into_iter()
                    .all(|a| a.is_finite() && a.abs() <= 1e6)
                && self.span().abs() > 0.
                && self.span().abs() <= TAU + self.turn_tolerance(),
            "invalid polar frame",
        )
    }
    pub fn span(&self) -> f64 {
        self.end_angle - self.start_angle
    }
    // The span subtracts two supplied f64 endpoints. Two endpoint roundings
    // contribute at most epsilon*(|start|+|end|), bounded here by 2*epsilon*max.
    // This recognizes one-turn endpoint roundoff, never excess beyond that
    // uncertainty or an additional revolution.
    pub(crate) fn turn_tolerance(&self) -> f64 {
        (2. * f64::EPSILON * self.start_angle.abs().max(self.end_angle.abs())).max(1e-12)
    }
    pub fn is_full_turn(&self) -> bool {
        (self.span().abs() - TAU).abs() <= self.turn_tolerance()
    }
    pub fn point(&self, radius: f64, angle: f64) -> [f64; 2] {
        [
            self.center[0] + radius * angle.sin(),
            self.center[1] - radius * angle.cos(),
        ]
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct PolarTick {
    pub id: String,
    pub label: String,
    pub position: f64,
}
impl PolarTick {
    pub fn new(id: impl Into<String>, label: impl Into<String>, position: f64) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            position,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct PolarTickOptions {
    pub count: usize,
    pub values: Option<Vec<f64>>,
    pub format: ValueFormat,
}
impl Default for PolarTickOptions {
    fn default() -> Self {
        Self {
            count: 5,
            values: None,
            format: ValueFormat::Auto,
        }
    }
}
impl PolarTickOptions {
    pub fn validate(&self) -> Result<()> {
        self.format.validate()?;
        ensure(
            self.count <= MAX_POLAR_TICKS
                && self
                    .values
                    .as_ref()
                    .is_none_or(|v| v.len() <= MAX_POLAR_TICKS && v.iter().all(|x| x.is_finite())),
            "polar tick budget exceeded or invalid values",
        )
    }
}
/// Uses the actual scale's domain and range; callers may convert the resulting
/// radius afterwards (for example square-root area scaling).
pub fn polar_numeric_ticks(scale: &ChartScale, o: &PolarTickOptions) -> Result<Vec<PolarTick>> {
    o.validate()?;
    let values = o.values.clone().unwrap_or_else(|| scale.ticks(o.count));
    ensure(
        values.len() <= MAX_POLAR_TICKS,
        "polar tick output budget exceeded",
    )?;
    let mut seen = BTreeSet::new();
    let mut result = vec![];
    for v in values {
        ensure(scale.accepts(v), "polar tick invalid for scale")?;
        let v = if v == 0. { 0. } else { v };
        if v < scale.domain()[0] || v > scale.domain()[1] || !seen.insert(v.to_bits()) {
            continue;
        }
        let position = scale.map(v);
        ensure(
            position.is_finite() && position.abs() <= 1e6,
            "polar tick projection overflow",
        )?;
        let label = if scale.kind() == ScaleKind::Utc && o.format == ValueFormat::Auto {
            ValueFormat::Utc.format(v)?
        } else {
            o.format.format(v)?
        };
        result.push(PolarTick::new(
            format!("{:016x}", v.to_bits()),
            label,
            position,
        ));
    }
    Ok(result)
}
/// Equal categorical bands, centered in the supplied directed range. Labels
/// are explicit safe text; category IDs remain unchanged.
pub fn polar_category_ticks(
    ids: &[String],
    range: [f64; 2],
    labels: &BTreeMap<String, String>,
) -> Result<Vec<PolarTick>> {
    ensure(
        ids.len() <= MAX_POLAR_TICKS
            && range.iter().all(|v| v.is_finite() && v.abs() <= 1e6)
            && labels.len() <= ids.len(),
        "invalid polar category tick input",
    )?;
    let mut seen = BTreeSet::new();
    ensure(
        ids.iter()
            .all(|id| !id.is_empty() && id.len() <= 256 && seen.insert(id))
            && labels
                .iter()
                .all(|(id, l)| seen.contains(id) && l.len() <= 1024),
        "invalid polar category identity or label",
    )?;
    Ok(ids
        .iter()
        .enumerate()
        .map(|(i, id)| {
            PolarTick::new(
                id,
                labels.get(id).unwrap_or(id),
                (range[0] * (1. - (i as f64 + 0.5) / ids.len() as f64))
                    + range[1] * (i as f64 + 0.5) / ids.len() as f64,
            )
        })
        .collect())
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PolarTextRotation {
    Fixed(f64),
    Radial,
    Tangential,
}
impl Default for PolarTextRotation {
    fn default() -> Self {
        Self::Fixed(0.)
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct PolarAxisStyle {
    pub width: f64,
    pub color: Option<Color>,
    pub tick_size: f64,
    pub tick_padding: f64,
    pub labels: bool,
    pub text_color: Option<Color>,
    pub text_size: Option<f64>,
    pub rotation: PolarTextRotation,
}
impl Default for PolarAxisStyle {
    fn default() -> Self {
        Self {
            width: 1.,
            color: None,
            tick_size: 5.,
            tick_padding: 5.,
            labels: true,
            text_color: None,
            text_size: None,
            rotation: Default::default(),
        }
    }
}
impl PolarAxisStyle {
    fn validate(&self) -> Result<()> {
        ensure(
            [self.width, self.tick_size, self.tick_padding]
                .iter()
                .all(|v| v.is_finite() && (0. ..=1024.).contains(v))
                && self
                    .text_size
                    .is_none_or(|v| v.is_finite() && (1. ..=256.).contains(&v))
                && match self.rotation {
                    PolarTextRotation::Fixed(v) => v.is_finite() && (-360. ..=360.).contains(&v),
                    _ => true,
                },
            "invalid polar axis style",
        )
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct PolarGridStyle {
    pub radial: bool,
    pub circular: bool,
    pub color: Option<Color>,
    pub width: f64,
    pub dash: Option<StrokeDash>,
}
impl Default for PolarGridStyle {
    fn default() -> Self {
        Self {
            radial: true,
            circular: true,
            color: None,
            width: 1.,
            dash: None,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct PolarAxesOptions {
    pub id: String,
    pub radial_start: Option<PolarAxisStyle>,
    pub radial_end: Option<PolarAxisStyle>,
    pub circular_inner: Option<PolarAxisStyle>,
    pub circular_outer: Option<PolarAxisStyle>,
    pub grid: PolarGridStyle,
}
impl Default for PolarAxesOptions {
    fn default() -> Self {
        Self {
            id: "polar".into(),
            radial_start: Some(Default::default()),
            radial_end: None,
            circular_inner: None,
            circular_outer: Some(Default::default()),
            grid: Default::default(),
        }
    }
}
impl PolarAxesOptions {
    pub fn validate(&self) -> Result<()> {
        ensure(
            !self.id.is_empty()
                && self.id.len() <= 128
                && self.grid.width.is_finite()
                && (0. ..=256.).contains(&self.grid.width),
            "invalid polar axes identity or grid width",
        )?;
        if let Some(d) = &self.grid.dash {
            d.validate()?;
        }
        for s in [
            &self.radial_start,
            &self.radial_end,
            &self.circular_inner,
            &self.circular_outer,
        ]
        .into_iter()
        .flatten()
        {
            s.validate()?;
        }
        Ok(())
    }
}
pub(crate) fn arc_commands(center: [f64; 2], radius: f64, start: f64, end: f64, out: &mut Vec<C>) {
    let n = ((end - start).abs() / FRAC_PI_2).ceil().max(1.) as usize;
    let delta = (end - start) / n as f64;
    for i in 0..n {
        let a = start + delta * i as f64;
        let b = a + delta;
        let k = 4. / 3. * (delta / 4.).tan();
        let p = P::new(center[0] + radius * a.sin(), center[1] - radius * a.cos());
        let q = P::new(center[0] + radius * b.sin(), center[1] - radius * b.cos());
        out.push(C::Cubic {
            first: P::new(p.x + k * radius * a.cos(), p.y + k * radius * a.sin()),
            second: P::new(q.x - k * radius * b.cos(), q.y - k * radius * b.sin()),
            end: q,
        });
    }
}
fn path_mark(id: String, commands: Vec<C>, color: Color, width: f64) -> Mark {
    let mut m = Mark::new(id, Geometry::Path(commands), color);
    m.fill = None;
    m.label.clear();
    m.stroke = (width > 0.).then_some(GraphicStroke {
        color,
        width,
        cap: StrokeCap::Butt,
    });
    m
}
fn p(v: [f64; 2]) -> P {
    P::new(v[0], v[1])
}
fn rotation(style: &PolarAxisStyle, angle: f64) -> f64 {
    let raw = match style.rotation {
        PolarTextRotation::Fixed(v) => return v,
        PolarTextRotation::Radial => angle.to_degrees() - 90.,
        PolarTextRotation::Tangential => angle.to_degrees(),
    };
    let mut a = (raw + 180.).rem_euclid(360.) - 180.;
    if a > 90. {
        a -= 180.;
    }
    if a < -90. {
        a += 180.;
    }
    a
}
fn text(
    c: &ChartConfig,
    style: &PolarAxisStyle,
    at: [f64; 2],
    direction: [f64; 2],
    angle: f64,
    value: &str,
) -> Label {
    let r = rotation(style, angle);
    let size = style.text_size.unwrap_or(c.theme.font_size);
    let along = direction[0] * r.to_radians().cos() + direction[1] * r.to_radians().sin();
    Label {
        x: at[0] - r.to_radians().sin() * size / 3.,
        y: at[1] + r.to_radians().cos() * size / 3.,
        text: value.into(),
        size,
        color: style.text_color.unwrap_or(c.theme.text),
        anchor: if along > 1e-9 {
            TextAnchor::Start
        } else if along < -1e-9 {
            TextAnchor::End
        } else {
            TextAnchor::Middle
        },
        rotation: r,
    }
}
fn checked_ticks(ticks: &[PolarTick]) -> Result<()> {
    let mut ids = BTreeSet::new();
    ensure(
        ticks.len() <= MAX_POLAR_TICKS
            && ticks.iter().all(|t| {
                !t.id.is_empty()
                    && t.id.len() <= 256
                    && t.label.len() <= 1024
                    && t.position.is_finite()
                    && ids.insert(&t.id)
            }),
        "invalid polar ticks or duplicate identity",
    )
}
/// Standalone axis/grid scene; use append_polar_axes to combine it with an
/// existing checked chart without losing explicit paint order or label slots.
pub fn polar_axes(
    c: &ChartConfig,
    f: &PolarFrame,
    radial: &[PolarTick],
    circular: &[PolarTick],
    o: &PolarAxesOptions,
) -> Result<Scene> {
    let mut scene = c.scene()?;
    append_polar_axes(&mut scene, c, f, radial, circular, o)?;
    Ok(scene)
}
/// Append grid behind the accepted scene and axes/text in front. Existing
/// labels and ownership slots are retained. Failure leaves the scene unchanged.
pub fn append_polar_axes(
    scene: &mut Scene,
    c: &ChartConfig,
    f: &PolarFrame,
    radial: &[PolarTick],
    circular: &[PolarTick],
    o: &PolarAxesOptions,
) -> Result<()> {
    scene.validate()?;
    c.validate()?;
    f.validate()?;
    o.validate()?;
    checked_ticks(radial)?;
    checked_ticks(circular)?;
    ensure(
        scene.width == c.width && scene.height == c.height,
        "polar axes config differs from scene",
    )?;
    ensure(
        radial
            .iter()
            .all(|t| t.position >= f.inner_radius && t.position <= f.outer_radius),
        "polar radius tick outside frame",
    )?;
    let dir = f.span().signum();
    let mut angles = vec![];
    let mut seam_seen = false;
    for tick in circular {
        let offset = (tick.position - f.start_angle) * dir;
        ensure(
            (-1e-10..=f.span().abs() + 1e-10).contains(&offset),
            "polar angle tick outside directed frame",
        )?;
        if f.is_full_turn()
            && (offset.abs() <= f.turn_tolerance()
                || (offset - f.span().abs()).abs() <= f.turn_tolerance())
        {
            if seam_seen {
                continue;
            }
            seam_seen = true;
        }
        angles.push(tick);
    }
    let mut marks = vec![];
    let mut labels = vec![];
    let grid = o.grid.color.unwrap_or(c.theme.grid);
    if o.grid.width > 0. {
        if o.grid.radial {
            for tick in radial {
                let mut path = vec![C::Move(p(f.point(tick.position, f.start_angle)))];
                arc_commands(
                    f.center,
                    tick.position,
                    f.start_angle,
                    f.end_angle,
                    &mut path,
                );
                let mut m = path_mark(
                    format!("{}:grid-radius:{}:{}", o.id, tick.id.len(), tick.id),
                    path,
                    grid,
                    o.grid.width,
                );
                m.stroke_dash = o.grid.dash.clone();
                marks.push(m);
            }
        }
        if o.grid.circular {
            for tick in &angles {
                let mut m = path_mark(
                    format!("{}:grid-angle:{}:{}", o.id, tick.id.len(), tick.id),
                    vec![
                        C::Move(p(f.point(f.inner_radius, tick.position))),
                        C::Line(p(f.point(f.outer_radius, tick.position))),
                    ],
                    grid,
                    o.grid.width,
                );
                m.stroke_dash = o.grid.dash.clone();
                marks.push(m);
            }
        }
    }
    let grids = marks.len();
    for (name, angle, side, style) in [
        ("start", f.start_angle, -dir, &o.radial_start),
        ("end", f.end_angle, dir, &o.radial_end),
    ] {
        let Some(style) = style else {
            continue;
        };
        let color = style.color.unwrap_or(c.theme.text);
        marks.push(path_mark(
            format!("{}:axis-{name}", o.id),
            vec![
                C::Move(p(f.point(f.inner_radius, angle))),
                C::Line(p(f.point(f.outer_radius, angle))),
            ],
            color,
            style.width,
        ));
        for tick in radial {
            let at = f.point(tick.position, angle);
            let normal = [side * angle.cos(), side * angle.sin()];
            marks.push(path_mark(
                format!("{}:tick-{name}:{}:{}", o.id, tick.id.len(), tick.id),
                vec![
                    C::Move(p(at)),
                    C::Line(P::new(
                        at[0] + normal[0] * style.tick_size,
                        at[1] + normal[1] * style.tick_size,
                    )),
                ],
                color,
                style.width,
            ));
            if style.labels {
                labels.push(text(
                    c,
                    style,
                    [
                        at[0] + normal[0] * (style.tick_size + style.tick_padding),
                        at[1] + normal[1] * (style.tick_size + style.tick_padding),
                    ],
                    normal,
                    angle,
                    &tick.label,
                ));
            }
        }
    }
    for (name, radius, side, style) in [
        ("inner", f.inner_radius, -1., &o.circular_inner),
        ("outer", f.outer_radius, 1., &o.circular_outer),
    ] {
        let Some(style) = style else {
            continue;
        };
        ensure(
            radius + side * (style.tick_size + style.tick_padding) >= 0.,
            "inner polar ticks cross the origin",
        )?;
        let color = style.color.unwrap_or(c.theme.text);
        let mut path = vec![C::Move(p(f.point(radius, f.start_angle)))];
        arc_commands(f.center, radius, f.start_angle, f.end_angle, &mut path);
        marks.push(path_mark(
            format!("{}:axis-{name}", o.id),
            path,
            color,
            style.width,
        ));
        for tick in &angles {
            marks.push(path_mark(
                format!("{}:tick-{name}:{}:{}", o.id, tick.id.len(), tick.id),
                vec![
                    C::Move(p(f.point(radius, tick.position))),
                    C::Line(p(f.point(radius + side * style.tick_size, tick.position))),
                ],
                color,
                style.width,
            ));
            if style.labels {
                labels.push(text(
                    c,
                    style,
                    f.point(
                        radius + side * (style.tick_size + style.tick_padding),
                        tick.position,
                    ),
                    [side * tick.position.sin(), -side * tick.position.cos()],
                    tick.position,
                    &tick.label,
                ));
            }
        }
    }
    let mut next = scene.clone();
    let before = scene.draw_items();
    let mi = next.marks.len();
    let li = next.labels.len();
    let added = marks.len();
    let texts = labels.len();
    next.marks.extend(marks);
    next.labels.extend(labels);
    next.draw_order = (mi..mi + grids)
        .map(DrawItem::Mark)
        .chain(before)
        .chain((mi + grids..mi + added).map(DrawItem::Mark))
        .chain((li..li + texts).map(DrawItem::Label))
        .collect();
    next.validate()?;
    *scene = next;
    Ok(())
}
/// Existing polar-bar geometry plus the common value/category axes. The wrapper
/// owns tick labels; legacy PolarBarOptions.labels is superseded by axis styles.
pub fn polar_bar_with_axes(
    c: &ChartConfig,
    categories: &[String],
    series: &[BarSeries],
    layout: &PolarBarOptions,
    axes: &PolarAxesOptions,
    ticks: &PolarTickOptions,
) -> Result<Scene> {
    let mut o = layout.clone();
    o.labels = false;
    let mut scene = crate::polar_bar(c, categories, series, &o)?;
    scene.marks.retain(|m| {
        !matches!(
            m.id.as_str(),
            "polar-grid-1" | "polar-grid-2" | "polar-grid-3" | "polar-grid-4"
        )
    });
    let f = PolarFrame::from_config(c, layout.inner_radius, layout.start_angle, layout.end_angle)?;
    let maximum = match layout.layout {
        BarLayout::Percent => 1.,
        BarLayout::Grouped => series
            .iter()
            .flat_map(|s| s.values.iter().flatten())
            .copied()
            .fold(0., f64::max),
        BarLayout::Stacked => (0..categories.len())
            .map(|i| series.iter().filter_map(|s| s.values[i]).sum::<f64>())
            .fold(0., f64::max),
    };
    let maximum = if maximum > 0. { maximum } else { 1. };
    let scale = ChartScale::new(ScaleKind::Linear, [0., maximum], [0., 1.])?;
    let mut radial = polar_numeric_ticks(&scale, ticks)?;
    for t in &mut radial {
        t.position = if layout.area_scale {
            (f.inner_radius * f.inner_radius
                + (f.outer_radius * f.outer_radius - f.inner_radius * f.inner_radius) * t.position)
                .sqrt()
        } else {
            f.inner_radius + (f.outer_radius - f.inner_radius) * t.position
        };
    }
    let circular =
        polar_category_ticks(categories, [f.start_angle, f.end_angle], &BTreeMap::new())?;
    append_polar_axes(&mut scene, c, &f, &radial, &circular, axes)?;
    Ok(scene)
}
