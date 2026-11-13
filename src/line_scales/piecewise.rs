//! Line consumers of full-breakpoint numeric scales.
use super::*;

/// Piecewise ranges are plot fractions: the first stop is 0 and the last is 1.
/// Interior stops may turn or leave that interval; drawing still uses the
/// normal plot clip. Such a range cannot provide a unique data viewport.
/// Ordinary axes use the unchanged continuous options.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PiecewiseLineScaleOptions {
    pub x: Option<PiecewiseScaleOptions>,
    pub y: Option<PiecewiseScaleOptions>,
    pub continuous: LineScaleOptions,
}

#[derive(Clone, Debug)]
pub struct PiecewiseLineLayout {
    pub layout: LineLayout,
    pub x_scale: ResolvedContinuousScale,
    pub y_scale: ResolvedContinuousScale,
    pub scale_options: PiecewiseLineScaleOptions,
}

pub(crate) fn resolve_plot_piecewise(
    profile: &PiecewiseScaleOptions,
    range: [f64; 2],
) -> Result<ResolvedContinuousScale> {
    profile.validate()?;
    ensure(
        profile.range.first() == Some(&0.) && profile.range.last() == Some(&1.),
        "chart piecewise range must begin at fraction 0 and end at fraction 1",
    )?;
    let mut pixels = profile.clone();
    pixels.range = profile
        .range
        .iter()
        .map(|&t| range[0] * (1. - t) + range[1] * t)
        .collect();
    ResolvedContinuousScale::piecewise(&pixels)
}

fn source_domain(profile: &PiecewiseScaleOptions, explicit: Option<[f64; 2]>) -> Result<[f64; 2]> {
    profile.validate()?;
    let domain = [profile.domain[0], profile.domain[profile.domain.len() - 1]];
    if let Some(explicit) = explicit {
        ensure(
            explicit[0].min(explicit[1]) == domain[0].min(domain[1])
                && explicit[0].max(explicit[1]) == domain[0].max(domain[1]),
            "line endpoint domain conflicts with its piecewise domain stops",
        )?;
    }
    Ok(domain)
}

pub fn line_layout_with_piecewise_scale_options(
    config: &ChartConfig,
    data: &[Series],
    layout: &LineOptions,
    style: &LineStyleOptions,
    scales: &PiecewiseLineScaleOptions,
) -> Result<PiecewiseLineLayout> {
    build(config, data, layout, style, scales, Default::default())
}

pub fn line_layout_with_piecewise_scale_options_and_point_factory(
    config: &ChartConfig,
    data: &[Series],
    layout: &LineOptions,
    style: &LineStyleOptions,
    scales: &PiecewiseLineScaleOptions,
    colors: Option<&OrdinalColorScale>,
    mut factory: impl FnMut(&LinePointContext<'_>) -> Result<LinePointGeometry>,
) -> Result<PiecewiseLineLayout> {
    build(
        config,
        data,
        layout,
        style,
        scales,
        LinePointRender {
            colors,
            factory: Some(&mut factory),
            ..Default::default()
        },
    )
}

fn build(
    config: &ChartConfig,
    data: &[Series],
    layout: &LineOptions,
    style: &LineStyleOptions,
    scales: &PiecewiseLineScaleOptions,
    render: LinePointRender<'_>,
) -> Result<PiecewiseLineLayout> {
    config.validate()?;
    crate::cartesian::validate_series(data)?;
    style.validate()?;
    layout.axes.validate()?;
    line_options_valid(layout)?;
    ensure(layout.baseline.is_finite(), "invalid line baseline")?;
    let x_kind = scales
        .x
        .as_ref()
        .map_or(scales.continuous.x.kind, |s| s.continuous.kind);
    let y_kind = scales
        .y
        .as_ref()
        .map_or(scales.continuous.y.kind, |s| s.continuous.kind);
    let visible = || data.iter().filter(|row| !layout.hidden.contains(&row.id));
    let xd = if let Some(profile) = &scales.x {
        source_domain(profile, layout.x_domain)?
    } else if let Some(domain) = layout.x_domain {
        domain
    } else {
        crate::cartesian::scale_domain(visible().flat_map(|s| s.data.iter().map(|p| p.x)), x_kind)?
    };
    let yd = if let Some(profile) = &scales.y {
        source_domain(profile, layout.y_domain)?
    } else if let Some(domain) = layout.y_domain {
        domain
    } else {
        crate::cartesian::scale_domain(
            visible()
                .flat_map(|s| s.data.iter().filter_map(|p| p.y))
                .chain(layout.area.then_some(layout.baseline)),
            y_kind,
        )?
    };
    let xr = [config.margin.left, config.width - config.margin.right];
    let yr = [config.height - config.margin.bottom, config.margin.top];
    let x_scale = if let Some(profile) = &scales.x {
        resolve_plot_piecewise(profile, xr)?
    } else {
        ResolvedContinuousScale::new(xd, xr, &scales.continuous.x)?
    };
    let y_scale = if let Some(profile) = &scales.y {
        resolve_plot_piecewise(profile, yr)?
    } else {
        ResolvedContinuousScale::new(yd, yr, &scales.continuous.y)?
    };
    let mut configured = layout.clone();
    configured.x_scale = x_kind;
    configured.y_scale = y_kind;
    let layout = crate::line_styles::line_layout_with_resolved_scales(
        config,
        data,
        &configured,
        style,
        (x_scale.chart_scale(), y_scale.chart_scale()),
        render,
    )?;
    Ok(PiecewiseLineLayout {
        layout,
        x_scale,
        y_scale,
        scale_options: scales.clone(),
    })
}

impl PiecewiseLineLayout {
    pub fn interaction_binding(&self) -> InteractionBinding {
        self.layout.interaction_binding()
    }

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
        })?
        .with_piecewise_scales(
            self.x_scale
                .is_piecewise()
                .then_some(self.x_scale.chart_scale()),
            self.y_scale
                .is_piecewise()
                .then_some(self.y_scale.chart_scale()),
            None,
        )
    }

    /// Retain all authored-order stops from the accepted viewport request.
    /// Nice is disabled after the baseline is established. Ordinary axes keep
    /// their canonical viewport orientation.
    pub fn viewport_scale_options(
        &self,
        domains: &PiecewiseViewportDomains,
    ) -> Result<PiecewiseLineScaleOptions> {
        let mut next = self.scale_options.clone();
        let update =
            |profile: &mut Option<PiecewiseScaleOptions>, stops: &Option<Vec<f64>>| -> Result<()> {
                ensure(
                    profile.is_some() == stops.is_some(),
                    "piecewise viewport axis profile is missing or unexpected",
                )?;
                if let (Some(profile), Some(stops)) = (profile, stops) {
                    profile.domain = stops.clone();
                    profile.continuous.nice = ScaleNice::Off;
                    profile.validate()?;
                }
                Ok(())
            };
        update(&mut next.x, &domains.x)?;
        update(&mut next.y, &domains.left)?;
        ensure(domains.right.is_none(), "line has no right value axis")?;
        let x = self.x_scale.display_domain();
        let y = self.y_scale.display_domain();
        let xr = self.x_scale.range();
        let yr = self.y_scale.range();
        next.continuous.x.reverse = (x[1] < x[0]) ^ (xr[1] < xr[0]);
        next.continuous.y.reverse = (y[1] < y[0]) ^ (yr[1] > yr[0]);
        next.continuous.x.nice = ScaleNice::Off;
        next.continuous.y.nice = ScaleNice::Off;
        Ok(next)
    }
}

impl From<PiecewiseLineLayout> for SceneInteraction {
    fn from(layout: PiecewiseLineLayout) -> Self {
        layout.layout.into()
    }
}
