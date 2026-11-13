//! Line layouts with shared, explicitly resolved D3 scale profiles.
use std::collections::{BTreeMap, BTreeSet};

use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as C, StrokeCap};

use crate::line_styles::{LinePointFactory, LinePointRender};
use crate::scene::ensure;
use crate::*;
mod piecewise;
pub(crate) use piecewise::resolve_plot_piecewise;
pub use piecewise::{
    PiecewiseLineLayout, PiecewiseLineScaleOptions, line_layout_with_piecewise_scale_options,
    line_layout_with_piecewise_scale_options_and_point_factory,
};
fn line_options_valid(options: &LineOptions) -> Result<()> {
    options.curve.validate()?;
    ensure(
        options.hidden.len() <= MAX_MARKS
            && options
                .hidden
                .iter()
                .all(|s| !s.is_empty() && s.len() <= 256),
        "invalid hidden line identities",
    )
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct LineScaleOptions {
    pub x: ContinuousScaleOptions,
    pub y: ContinuousScaleOptions,
}
#[derive(Clone, Debug)]
pub struct ScaledLineLayout {
    pub layout: LineLayout,
    pub x_scale: ResolvedContinuousScale,
    pub y_scale: ResolvedContinuousScale,
    pub scale_options: LineScaleOptions,
}
impl ScaledLineLayout {
    /// Canonical viewport domains are ascending. Preserve the original visual
    /// direction even when the authored domain was descending, and do not nice
    /// an already anchored gesture window a second time.
    pub fn viewport_scale_options(&self) -> LineScaleOptions {
        let mut options = self.scale_options.clone();
        let x = self.x_scale.display_domain();
        let y = self.y_scale.display_domain();
        let xr = self.x_scale.range();
        let yr = self.y_scale.range();
        options.x.reverse = (x[1] < x[0]) ^ (xr[1] < xr[0]);
        options.y.reverse = (y[1] < y[0]) ^ (yr[1] > yr[0]);
        options.x.nice = ScaleNice::Off;
        options.y.nice = ScaleNice::Off;
        options
    }
    pub fn interaction_binding(&self) -> InteractionBinding {
        self.layout.interaction_binding()
    }
    /// Navigate canonical data domains using the actual displayed range
    /// directions. Constant domains have no navigable span and return Err.
    pub fn viewport_controller(&self, limits: ViewportLimits) -> Result<DomainViewport> {
        let x = self.x_scale.display_domain();
        let y = self.y_scale.display_domain();
        let xr = self.x_scale.range();
        let yr = self.y_scale.range();
        DomainViewport::new(
            ViewportAxes {
                x: ViewportAxis::new(
                    [x[0].min(x[1]), x[0].max(x[1])],
                    self.x_scale.chart_scale().kind(),
                ),
                left: Some(ViewportAxis::new(
                    [y[0].min(y[1]), y[0].max(y[1])],
                    self.y_scale.chart_scale().kind(),
                )),
                right: None,
            },
            self.layout.plot,
            limits,
        )?
        .with_directions(ViewportDirections {
            x: (x[1] < x[0]) ^ (xr[1] < xr[0]),
            left: (y[1] < y[0]) ^ (yr[1] > yr[0]),
            right: false,
        })
    }
}
impl From<ScaledLineLayout> for SceneInteraction {
    fn from(layout: ScaledLineLayout) -> Self {
        layout.layout.into()
    }
}
/// The resolved display domains drive axes, clipping, paths and actual sites.
/// Auto domains retain the existing visible-line data policy; explicit domains
/// may be constant/descending. Nice does not overwrite the recorded source.
pub fn line_layout_with_scale_options(
    config: &ChartConfig,
    data: &[Series],
    layout: &LineOptions,
    style: &LineStyleOptions,
    scales: &LineScaleOptions,
) -> Result<ScaledLineLayout> {
    scaled_line_layout(config, data, layout, style, scales, Default::default())
}

/// Configure the value scales and the ordinal series palette together.
pub fn line_layout_with_scale_options_and_colors(
    config: &ChartConfig,
    data: &[Series],
    layout: &LineOptions,
    style: &LineStyleOptions,
    scales: &LineScaleOptions,
    colors: &OrdinalColorScale,
) -> Result<ScaledLineLayout> {
    scaled_line_layout(
        config,
        data,
        layout,
        style,
        scales,
        LinePointRender {
            colors: Some(colors),
            ..Default::default()
        },
    )
}

/// Combine resolved value scales, an optional ordinal palette and custom
/// point geometry. See [`line_layout_with_point_factory`].
pub fn line_layout_with_scale_options_and_point_factory(
    config: &ChartConfig,
    data: &[Series],
    layout: &LineOptions,
    style: &LineStyleOptions,
    scales: &LineScaleOptions,
    colors: Option<&OrdinalColorScale>,
    mut factory: impl FnMut(&LinePointContext<'_>) -> Result<LinePointGeometry>,
) -> Result<ScaledLineLayout> {
    scaled_line_layout(
        config,
        data,
        layout,
        style,
        scales,
        LinePointRender {
            colors,
            factory: Some(&mut factory),
            categories: None,
        },
    )
}

fn scaled_line_layout(
    config: &ChartConfig,
    data: &[Series],
    layout: &LineOptions,
    style: &LineStyleOptions,
    scales: &LineScaleOptions,
    render: LinePointRender<'_>,
) -> Result<ScaledLineLayout> {
    config.validate()?;
    crate::cartesian::validate_series(data)?;
    style.validate()?;
    scales.x.validate()?;
    scales.y.validate()?;
    layout.axes.validate()?;
    line_options_valid(layout)?;
    ensure(layout.baseline.is_finite(), "invalid line baseline")?;
    let visible = || data.iter().filter(|row| !layout.hidden.contains(&row.id));
    let xd = match layout.x_domain {
        Some(domain) => domain,
        None => crate::cartesian::scale_domain(
            visible().flat_map(|row| row.data.iter().map(|p| p.x)),
            scales.x.kind,
        )?,
    };
    let yd = match layout.y_domain {
        Some(domain) => domain,
        None => crate::cartesian::scale_domain(
            visible()
                .flat_map(|row| row.data.iter().filter_map(|p| p.y))
                .chain(layout.area.then_some(layout.baseline)),
            scales.y.kind,
        )?,
    };
    let x_scale = ResolvedContinuousScale::new(
        xd,
        [config.margin.left, config.width - config.margin.right],
        &scales.x,
    )?;
    let y_scale = ResolvedContinuousScale::new(
        yd,
        [config.height - config.margin.bottom, config.margin.top],
        &scales.y,
    )?;
    let mut configured = layout.clone();
    configured.x_scale = scales.x.kind;
    configured.y_scale = scales.y.kind;
    let layout = crate::line_styles::line_layout_with_resolved_scales(
        config,
        data,
        &configured,
        style,
        (x_scale.chart_scale(), y_scale.chart_scale()),
        render,
    )?;
    Ok(ScaledLineLayout {
        layout,
        x_scale,
        y_scale,
        scale_options: scales.clone(),
    })
}

#[derive(Clone, Debug, PartialEq)]
pub struct PointLineDatum {
    pub x: CategoryKey,
    pub y: Option<f64>,
}
impl PointLineDatum {
    pub fn new(x: impl Into<CategoryKey>, y: f64) -> Self {
        Self {
            x: x.into(),
            y: Some(y),
        }
    }
    pub fn gap(x: impl Into<CategoryKey>) -> Self {
        Self {
            x: x.into(),
            y: None,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct PointLineSeries {
    pub id: String,
    pub data: Vec<PointLineDatum>,
}
impl PointLineSeries {
    pub fn new(id: impl Into<String>, data: Vec<PointLineDatum>) -> Self {
        Self {
            id: id.into(),
            data,
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PointLineOptions {
    /// Numeric X settings are not used: x_scale must be Linear and x_domain,
    /// x tick_values and tick_calendar must be None. Other line styles apply.
    pub line: LineOptions,
    pub point: PointScaleOptions,
    pub y_scale: ContinuousScaleOptions,
    /// None gathers first occurrences from visible series, including gaps.
    /// Explicit omitted keys become gaps, preserving original source indices.
    pub domain: Option<Vec<CategoryKey>>,
    pub tick_values: Option<Vec<CategoryKey>>,
    /// Optional ascending window in ordinal index coordinates. Default is
    /// [0,n-1], or [-.5,.5] for one key. A window affine-projects the full D3
    /// lattice, then optionally rounds pixels; it never substitutes band slots.
    pub window: Option<[f64; 2]>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedLinePointScale {
    base: PointScale,
    window: Option<[f64; 2]>,
    round: bool,
}
impl ResolvedLinePointScale {
    pub fn domain(&self) -> &[CategoryKey] {
        self.base.domain()
    }
    pub fn range(&self) -> [f64; 2] {
        self.base.range()
    }
    pub fn original_scale(&self) -> &PointScale {
        &self.base
    }
    pub fn window(&self) -> [f64; 2] {
        self.window.unwrap_or_else(|| self.full_window())
    }
    fn full_window(&self) -> [f64; 2] {
        match self.domain().len() {
            0 => [0., 1.],
            1 => [-0.5, 0.5],
            n => [0., (n - 1) as f64],
        }
    }
    pub fn map(&self, key: &CategoryKey) -> Option<f64> {
        let pixel = self.base.map(key)?;
        let Some(window) = self.window else {
            return Some(pixel);
        };
        if window == self.full_window() {
            return Some(pixel);
        }
        let range = self.range();
        let fraction = if range[0] == range[1] {
            0.5
        } else {
            (pixel - range[0]) / (range[1] - range[0])
        };
        let logical = if self.domain().len() == 1 {
            fraction - 0.5
        } else {
            fraction * self.domain().len().saturating_sub(1) as f64
        };
        let value = crate::scale_options::interpolate(window, range, logical, false);
        Some(if self.round {
            crate::scale_options::js_round(value)
        } else {
            value
        })
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct PointLineSite {
    pub mark_id: String,
    pub series_id: String,
    pub source_index: usize,
    pub raw_x: CategoryKey,
    pub raw_y: f64,
    pub formatted_x: String,
    pub formatted_y: String,
    pub series_color: Color,
    pub position: [f64; 2],
    pub label_index: Option<usize>,
}
#[derive(Clone, Debug)]
pub struct PointLineLayout {
    pub scene: Scene,
    pub plot: ClipRect,
    pub sites: Vec<PointLineSite>,
    pub slices_x: Vec<LineSlice>,
    pub slices_y: Vec<LineSlice>,
    pub x_scale: ResolvedLinePointScale,
    pub y_scale: ResolvedContinuousScale,
    pub anchors: BTreeMap<String, InteractionAnchor>,
    y_options: ContinuousScaleOptions,
}
impl PointLineLayout {
    pub fn viewport_y_scale_options(&self) -> ContinuousScaleOptions {
        let mut options = self.y_options;
        let d = self.y_scale.display_domain();
        let r = self.y_scale.range();
        options.reverse = (d[1] < d[0]) ^ (r[1] > r[0]);
        options.nice = ScaleNice::Off;
        options
    }
    pub fn viewport_controller(&self, limits: ViewportLimits) -> Result<DomainViewport> {
        let x = self.x_scale.window();
        let y = self.y_scale.display_domain();
        let yr = self.y_scale.range();
        DomainViewport::new(
            ViewportAxes {
                x: ViewportAxis::new(x, ScaleKind::Linear),
                left: Some(ViewportAxis::new(
                    [y[0].min(y[1]), y[0].max(y[1])],
                    self.y_scale.chart_scale().kind(),
                )),
                right: None,
            },
            self.plot,
            limits,
        )?
        .with_directions(ViewportDirections {
            x: self.x_scale.range()[1] < self.x_scale.range()[0],
            left: (y[1] < y[0]) ^ (yr[1] > yr[0]),
            right: false,
        })?
        .with_x_max_zoom((x[1] - x[0]).clamp(1., 1e6))
    }
    pub fn interaction_binding(&self) -> InteractionBinding {
        InteractionBinding {
            sites: self
                .sites
                .iter()
                .map(|s| InteractionSite {
                    mark_id: s.mark_id.clone(),
                    series: s.series_id.clone(),
                    label: format!("{} · {}", s.series_id, s.formatted_x),
                    value: Some(s.raw_y),
                    formatted_value: Some(s.formatted_y.clone()),
                    color: s.series_color,
                    position: s.position,
                })
                .collect(),
            anchors: self.anchors.clone(),
            ..Default::default()
        }
    }
}
impl From<PointLineLayout> for SceneInteraction {
    fn from(layout: PointLineLayout) -> Self {
        let interaction = Some(layout.interaction_binding());
        Self {
            scene: layout.scene,
            plot: Some(layout.plot),
            interaction,
        }
    }
}
pub(crate) fn category_text(key: &CategoryKey, format: &ValueFormat) -> Result<String> {
    match key {
        CategoryKey::String(s) => Ok(s.clone()),
        CategoryKey::Number(n) => format.format(*n),
    }
}
/// Point X accepts real string/numeric keys. The internal numeric adapter is
/// private: exported sites and tooltips retain the exact original typed key.
/// Y auto domains use all visible source values, so panning X does not rescale Y.
pub fn point_line_layout(
    config: &ChartConfig,
    data: &[PointLineSeries],
    options: &PointLineOptions,
    style: &LineStyleOptions,
) -> Result<PointLineLayout> {
    point_line_layout_impl(config, data, options, style, None, None)
}

/// Replace enabled points on an ordinal X axis. The factory receives the
/// original typed category key. Omitted domain keys and points outside the
/// visible window do not call the factory.
pub fn point_line_layout_with_point_factory(
    config: &ChartConfig,
    data: &[PointLineSeries],
    options: &PointLineOptions,
    style: &LineStyleOptions,
    colors: Option<&OrdinalColorScale>,
    mut factory: impl FnMut(&LinePointContext<'_>) -> Result<LinePointGeometry>,
) -> Result<PointLineLayout> {
    point_line_layout_impl(config, data, options, style, colors, Some(&mut factory))
}

fn point_line_layout_impl<'a>(
    config: &ChartConfig,
    data: &'a [PointLineSeries],
    options: &'a PointLineOptions,
    style: &LineStyleOptions,
    colors: Option<&'a OrdinalColorScale>,
    factory: Option<&'a mut LinePointFactory<'a>>,
) -> Result<PointLineLayout> {
    config.validate()?;
    style.validate()?;
    options.point.validate()?;
    options.y_scale.validate()?;
    options.line.axes.validate()?;
    line_options_valid(&options.line)?;
    ensure(
        options.line.x_scale == ScaleKind::Linear
            && options.line.x_domain.is_none()
            && options.line.axes.x.tick_values.is_none()
            && options.line.axes.x.tick_calendar.is_none(),
        "point X requires typed domain/window/ticks, not continuous X options",
    )?;
    ensure(
        options.line.baseline.is_finite() && data.len() <= 256,
        "invalid point-line baseline/series budget",
    )?;
    let mut ids = BTreeSet::new();
    let mut total = 0usize;
    for row in data {
        ensure(
            !row.id.is_empty() && row.id.len() <= 256 && ids.insert(&row.id),
            "invalid point-line series identity",
        )?;
        total = total
            .checked_add(row.data.len())
            .ok_or_else(|| ChartError("point-line budget overflow".into()))?;
        ensure(total <= MAX_MARKS / 2, "point-line source budget exceeded")?;
        for p in &row.data {
            p.x.validate()?;
            ensure(p.y.is_none_or(f64::is_finite), "nonfinite point-line value")?;
        }
    }
    for domain in [options.domain.as_ref(), options.tick_values.as_ref()]
        .into_iter()
        .flatten()
    {
        ensure(
            domain.len() <= MAX_MARKS,
            "point-line category budget exceeded",
        )?;
        for key in domain {
            key.validate()?;
        }
    }
    if let Some(w) = options.window {
        ensure(
            w.iter().all(|v| v.is_finite() && v.abs() <= 1e6)
                && w[0] < w[1]
                && (w[1] - w[0]).is_finite(),
            "invalid ordinal point window",
        )?;
    }
    let domain = options.domain.clone().unwrap_or_else(|| {
        data.iter()
            .filter(|row| !options.line.hidden.contains(&row.id))
            .flat_map(|row| row.data.iter().map(|p| p.x.clone()))
            .collect()
    });
    let plot = ClipRect::plot(config);
    let x_scale = ResolvedLinePointScale {
        base: PointScale::new(domain, [plot.x, plot.x + plot.width], &options.point)?,
        window: options.window,
        round: options.point.round,
    };
    ensure(
        x_scale
            .domain()
            .iter()
            .all(|key| x_scale.map(key).is_some_and(f64::is_finite)),
        "ordinal point window projection overflow",
    )?;
    let yd = match options.line.y_domain {
        Some(d) => d,
        None => crate::cartesian::scale_domain(
            data.iter()
                .filter(|row| !options.line.hidden.contains(&row.id))
                .flat_map(|row| row.data.iter().filter_map(|p| p.y))
                .chain(options.line.area.then_some(options.line.baseline)),
            options.y_scale.kind,
        )?,
    };
    let y_scale =
        ResolvedContinuousScale::new(yd, [plot.y + plot.height, plot.y], &options.y_scale)?;
    let adapter = data
        .iter()
        .map(|row| {
            Series::new(
                &row.id,
                row.data
                    .iter()
                    .map(|p| {
                        if let Some(x) = x_scale.map(&p.x) {
                            Point { x, y: p.y }
                        } else {
                            Point::gap(0.)
                        }
                    })
                    .collect(),
            )
        })
        .collect::<Vec<_>>();
    let mut line = options.line.clone();
    line.x_scale = ScaleKind::Linear;
    line.y_scale = options.y_scale.kind;
    line.axes.x.visible = false;
    line.axes.x.grid = false;
    line.axes.x.legend = None;
    // Adapter X values are pixels. Only the original keys use the requested
    // category formatter, including formatters that require integral values.
    line.axes.x.format = ValueFormat::Auto;
    let identity = ChartScale::new(
        ScaleKind::Linear,
        [plot.x, plot.x + plot.width],
        [plot.x, plot.x + plot.width],
    )?;
    let mut numeric = crate::line_styles::line_layout_with_resolved_scales(
        config,
        &adapter,
        &line,
        style,
        (&identity, y_scale.chart_scale()),
        LinePointRender {
            colors,
            factory,
            categories: Some((data, &options.line.axes.x.format)),
        },
    )?;
    let rows = data
        .iter()
        .map(|row| (row.id.as_str(), row))
        .collect::<BTreeMap<_, _>>();
    let mut sites = Vec::with_capacity(numeric.sites.len());
    for site in &numeric.sites {
        let source = &rows[site.series_id.as_str()].data[site.source_index];
        sites.push(PointLineSite {
            mark_id: site.mark_id.clone(),
            series_id: site.series_id.clone(),
            source_index: site.source_index,
            raw_x: source.x.clone(),
            raw_y: site.raw_y,
            formatted_x: site.formatted_x.clone(),
            formatted_y: site.formatted_y.clone(),
            series_color: site.series_color,
            position: site.position,
            label_index: site.label_index,
        });
    }
    let site_labels = sites
        .iter()
        .map(|s| {
            (
                s.mark_id.as_str(),
                format!("{}: {}, {}", s.series_id, s.formatted_x, s.formatted_y),
            )
        })
        .collect::<BTreeMap<_, _>>();
    for mark in &mut numeric.scene.marks {
        if let Some(label) = site_labels.get(mark.id.as_str()) {
            mark.label = label.clone();
        }
    }
    let previous = numeric.scene.draw_items();
    let mark_start = numeric.scene.marks.len();
    let label_start = numeric.scene.labels.len();
    point_line_axis(
        &mut numeric.scene,
        config,
        &x_scale,
        &options.line.axes.x,
        options.line.axes.x_side,
        options.tick_values.as_deref(),
        "axis-x",
    )?;
    numeric.scene.draw_order = (mark_start..numeric.scene.marks.len())
        .map(DrawItem::Mark)
        .chain(previous)
        .chain((label_start..numeric.scene.labels.len()).map(DrawItem::Label))
        .collect();
    numeric.scene.validate()?;
    let result = PointLineLayout {
        scene: numeric.scene,
        plot,
        sites,
        slices_x: numeric.slices_x,
        slices_y: numeric.slices_y,
        x_scale,
        y_scale,
        anchors: numeric.anchors,
        y_options: options.y_scale,
    };
    result.interaction_binding().validate(&result.scene)?;
    Ok(result)
}
fn point_line_axis(
    scene: &mut Scene,
    c: &ChartConfig,
    scale: &ResolvedLinePointScale,
    o: &AxisOptions,
    side: AxisSide,
    explicit: Option<&[CategoryKey]>,
    id: &str,
) -> Result<()> {
    ensure(
        matches!(side, AxisSide::Top | AxisSide::Bottom),
        "point X axis must be top/bottom",
    )?;
    let y = if side == AxisSide::Top {
        c.margin.top
    } else {
        c.height - c.margin.bottom
    };
    let direction = if side == AxisSide::Top { -1. } else { 1. };
    let stride = scale.domain().len().div_ceil(o.tick_count.max(1)).max(1);
    let ticks = if let Some(ticks) = explicit {
        ensure(ticks.len() <= 1000, "point axis tick budget exceeded")?;
        ensure(
            ticks.iter().all(|key| scale.map(key).is_some()),
            "point axis tick outside typed domain",
        )?;
        ticks.iter().collect::<Vec<_>>()
    } else if o.tick_count == 0 {
        vec![]
    } else {
        scale.domain().iter().step_by(stride).collect()
    };
    let mut seen = BTreeSet::new();
    for key in ticks {
        let identity = key.identity()?;
        if !seen.insert(identity.clone()) {
            continue;
        }
        let x = scale.map(key).unwrap();
        if x < c.margin.left || x > c.width - c.margin.right {
            continue;
        }
        let mut path = |name: String, a: [f64; 2], b: [f64; 2], color: Color| {
            let mut mark = Mark::new(
                name,
                Geometry::Path(vec![
                    C::Move(P::new(a[0], a[1])),
                    C::Line(P::new(b[0], b[1])),
                ]),
                color,
            );
            mark.label.clear();
            mark.fill = None;
            mark.stroke = Some(GraphicStroke {
                color,
                width: 1.,
                cap: StrokeCap::Butt,
            });
            scene.marks.push(mark);
        };
        if o.grid {
            path(
                format!("{id}-grid:{identity}"),
                [x, c.margin.top],
                [x, c.height - c.margin.bottom],
                c.theme.grid,
            );
        }
        if o.visible {
            path(
                format!("{id}-tick:{identity}"),
                [x, y],
                [x, y + direction * o.tick_size],
                c.theme.text,
            );
            scene.labels.push(Label {
                x,
                y: y + direction * (o.tick_size + o.tick_padding)
                    + if direction > 0. {
                        c.theme.font_size
                    } else {
                        0.
                    },
                text: category_text(key, &o.format)?,
                size: c.theme.font_size,
                color: c.theme.text,
                anchor: TextAnchor::Middle,
                rotation: o.tick_rotation,
            });
        }
    }
    if o.visible {
        let mut mark = Mark::new(
            id,
            Geometry::Path(vec![
                C::Move(P::new(c.margin.left, y)),
                C::Line(P::new(c.width - c.margin.right, y)),
            ]),
            c.theme.text,
        );
        mark.label.clear();
        mark.fill = None;
        mark.stroke = Some(GraphicStroke {
            color: c.theme.text,
            width: 1.,
            cap: StrokeCap::Butt,
        });
        scene.marks.push(mark);
        if let Some(legend) = &o.legend {
            scene.labels.push(Label {
                x: (c.margin.left + c.width - c.margin.right) / 2.,
                y: y + direction * (o.tick_size + o.tick_padding + c.theme.font_size * 2.5),
                text: legend.clone(),
                size: c.theme.font_size,
                color: c.theme.text,
                anchor: TextAnchor::Middle,
                rotation: 0.,
            });
        }
    }
    Ok(())
}
