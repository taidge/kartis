use std::collections::BTreeSet;

use super::*;

#[derive(Clone)]
pub(super) enum Layout {
    Line(PiecewiseLineLayout),
    Scatter(ScatterLayout),
}
pub(super) fn config(width: f64, height: f64) -> ChartConfig {
    ChartConfig {
        width,
        height,
        title: "Piecewise numeric scales".into(),
        margin: Margin {
            left: 76.,
            right: 30.,
            top: 35.,
            bottom: 65.,
        },
        ..Default::default()
    }
}
pub(super) fn profiles(case: u8, phase: u8) -> Result<PiecewiseLineScaleOptions> {
    if case > 6 || phase > 1 {
        return Err(ChartError("invalid piecewise fixture profile".into()));
    }
    let (kind, domain) = match case {
        0 | 6 => (ScaleKind::Linear, vec![-10., -1., 4., 20.]),
        1 => (ScaleKind::Log, vec![1., 2., 15., 128.]),
        2 => (ScaleKind::SymLog { constant: 2. }, vec![-30., -2., 3., 80.]),
        3 if phase == 1 => (ScaleKind::Pow { exponent: -1. }, vec![-32., -8., -2., -1.]),
        3 => (ScaleKind::Pow { exponent: 2. }, vec![-4., -1., 1., 9.]),
        4 => (ScaleKind::Sqrt, vec![-9., -1., 4., 25.]),
        _ => (
            ScaleKind::Utc,
            vec![
                1_708_992_000_000.,
                1_709_078_400_000.,
                1_709_251_200_000.,
                1_709_510_400_000.,
            ],
        ),
    };
    let mut x = PiecewiseScaleOptions {
        domain,
        range: if case == 6 {
            vec![0., 1., 0.2, 1.]
        } else {
            vec![0., 0.18, 0.73, 1.]
        },
        continuous: ContinuousScaleOptions {
            kind,
            reverse: phase == 1 && matches!(case, 1 | 2 | 3 | 5),
            round: phase == 1,
            clamp: phase == 1,
            ..Default::default()
        },
    };
    if phase == 1 && case != 2 {
        x.domain.reverse();
    }
    let mut y = PiecewiseScaleOptions {
        domain: vec![0., 2., 7., 12.],
        range: vec![0., 0.53, 0.81, 1.],
        continuous: ContinuousScaleOptions {
            reverse: phase == 1,
            round: phase == 1,
            clamp: phase == 1,
            ..Default::default()
        },
    };
    if phase == 1 {
        y.domain.reverse();
    }
    Ok(PiecewiseLineScaleOptions {
        x: Some(x),
        y: Some(y),
        ..Default::default()
    })
}
fn data(case: u8, phase: u8) -> Result<Vec<Series>> {
    let mut domain = profiles(case, phase)?.x.unwrap().domain;
    domain.sort_by(f64::total_cmp);
    Ok(["alpha", "beta"]
        .into_iter()
        .enumerate()
        .map(|(series, id)| {
            Series::new(
                id,
                (0..3)
                    .flat_map(|segment| (0..7).map(move |i| (segment, i)))
                    .enumerate()
                    .map(|(index, (segment, i))| {
                        let fraction = (f64::from(i) + 0.37) / 7.;
                        let x = domain[segment] * (1. - fraction) + domain[segment + 1] * fraction;
                        Point::new(
                            if case == 5 { x.round() } else { x },
                            1. + ((index * 3 + series * 5) % 11) as f64,
                        )
                    })
                    .collect(),
            )
        })
        .collect())
}
pub(super) fn proposal(
    case: u8,
    phase: u8,
    scatter: bool,
    c: &ChartConfig,
    hidden: BTreeSet<String>,
    viewport: Option<(&Layout, &BrowserViewportLayout)>,
) -> Result<Layout> {
    let mut frame = c.clone();
    if case == 5 {
        // Leave room for the outer date label as well as spacing the ticks.
        frame.margin.right = 44.;
    }
    let c = &frame;
    let data = data(case, phase)?;
    let mut scales = profiles(case, phase)?;
    if let Some((Layout::Line(base), request)) = viewport {
        scales = base.viewport_scale_options(&request.piecewise_domains)?;
    }
    if !scatter {
        let mut options = LineOptions {
            hidden,
            ..Default::default()
        };
        if case == 5 {
            options.axes.x.format = ValueFormat::UtcDate;
            options.axes.x.tick_count = 4;
        }
        if let Some((_, request)) = viewport {
            options.x_domain = Some(request.domains.x);
            options.y_domain = request.domains.left;
        }
        return Ok(Layout::Line(line_layout_with_piecewise_scale_options(
            c,
            &data,
            &options,
            &Default::default(),
            &scales,
        )?));
    }
    let scales = if let Some((Layout::Scatter(base), request)) = viewport {
        base.viewport_scale_options_with_piecewise(request.domains, &request.piecewise_domains)?
    } else {
        ScatterScaleOptions {
            x: ScatterAxisScale::Piecewise {
                options: scales.x.unwrap(),
            },
            y: ScatterAxisScale::Piecewise {
                options: scales.y.unwrap(),
            },
        }
    };
    let data = data
        .into_iter()
        .map(|s| {
            ScatterSeries::new(
                s.id,
                s.data
                    .into_iter()
                    .enumerate()
                    .map(|(i, p)| ScatterDatum::new(i.to_string(), p.x, p.y.unwrap()))
                    .collect(),
            )
        })
        .collect::<Vec<_>>();
    let mut options = ScatterLayoutOptions {
        scales,
        hidden,
        ..Default::default()
    };
    if case == 5
        && let Some(axis) = options.axes.bottom.as_mut()
    {
        axis.format = ValueFormat::UtcDate;
        axis.tick_count = 4;
    }
    Ok(Layout::Scatter(scatter_layout(c, &data, &options)?))
}
impl Layout {
    pub(super) fn scene(&self) -> &Scene {
        match self {
            Self::Line(l) => &l.layout.scene,
            Self::Scatter(l) => &l.scene,
        }
    }
    pub(super) fn plot(&self) -> ClipRect {
        match self {
            Self::Line(l) => l.layout.plot,
            Self::Scatter(l) => l.plot,
        }
    }
    pub(super) fn controller(&self) -> Result<DomainViewport> {
        match self {
            Self::Line(l) => l.viewport_controller(ViewportLimits {
                max_zoom: 8.,
                ..Default::default()
            }),
            Self::Scatter(l) => l.viewport_controller(ViewportLimits {
                max_zoom: 8.,
                ..Default::default()
            }),
        }
    }
    pub(super) fn interaction(&self) -> SceneInteraction {
        SceneInteraction {
            scene: self.scene().clone(),
            plot: Some(self.plot()),
            interaction: Some(match self {
                Self::Line(l) => l.interaction_binding(),
                Self::Scatter(l) => l.interaction_binding(),
            }),
        }
    }
    pub(super) fn scale(&self, vertical: bool) -> &ResolvedContinuousScale {
        match self {
            Self::Line(l) => {
                if vertical {
                    &l.y_scale
                } else {
                    &l.x_scale
                }
            }
            Self::Scatter(l) => match if vertical { &l.y_scale } else { &l.x_scale } {
                ResolvedScatterAxis::Continuous(s) => s,
                _ => unreachable!(),
            },
        }
    }
    pub(super) fn sites(&self) -> Vec<Value> {
        match self {
        Self::Line(l)=>l.layout.sites.iter().map(|s|json!({"id":s.mark_id,"series":s.series_id,"source":s.source_index,
            "raw":[s.raw_x,s.raw_y],"position":s.position,"formattedX":s.formatted_x,"formattedY":s.formatted_y})).collect(),
        Self::Scatter(l)=>l.sites.iter().map(|s|json!({"id":s.mark_id,"series":s.series_id,"source":s.source_index,
            "raw":[number(&s.raw_x),number(&s.raw_y)],"position":s.position,"formattedX":s.formatted_x,"formattedY":s.formatted_y})).collect()}
    }
}
fn number(key: &CategoryKey) -> f64 {
    match key {
        CategoryKey::Number(n) => *n,
        _ => unreachable!(),
    }
}
pub(super) fn scale_json(s: &ResolvedContinuousScale) -> Value {
    let (kind, parameter) = match s.chart_scale().kind() {
        ScaleKind::Linear => ("linear", 1.),
        ScaleKind::Log => ("log", 10.),
        ScaleKind::SymLog { constant } => ("symlog", constant),
        ScaleKind::Pow { exponent } => ("pow", exponent),
        ScaleKind::Sqrt => ("sqrt", 0.5),
        ScaleKind::Utc => ("utc", 1.),
    };
    json!({"kind":kind,"parameter":parameter,"domain":s.display_domain_stops(),"range":s.range_stops(),
        "round":s.chart_scale().is_rounded(),"clamp":s.chart_scale().is_clamped(),"ticks":s.ticks(5)})
}
