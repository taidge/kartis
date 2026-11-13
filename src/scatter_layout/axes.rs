use super::*;
use glory::render::{GraphicPoint as P, PathCommand as C, StrokeCap};

impl ScatterAxisScale {
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Piecewise { options } => {
                crate::line_scales::resolve_plot_piecewise(options, [0., 1.])?;
                Ok(())
            }
            Self::Continuous { options, min, max } => {
                options.validate()?;
                for bound in [min, max] {
                    if let ScatterBound::Fixed(v) = bound {
                        ensure(v.is_finite(), "scatter axis endpoint must be finite")?;
                        if options.kind == ScaleKind::Utc {
                            ensure(
                                v.fract() == 0. && v.abs() <= MAX_TIMESTAMP_MS as f64,
                                "scatter UTC endpoint must be integral and bounded",
                            )?;
                        }
                    }
                }
                Ok(())
            }
            Self::Point {
                options,
                domain,
                window,
            } => {
                options.validate()?;
                if let Some(domain) = domain {
                    ensure(
                        domain.len() <= MAX_MARKS,
                        "scatter point category budget exceeded",
                    )?;
                    let mut bytes = 0usize;
                    for key in domain {
                        key.validate()?;
                        if let CategoryKey::String(s) = key {
                            bytes += s.len();
                            ensure(bytes <= 4_000_000, "scatter category text budget exceeded")?;
                        }
                    }
                }
                if let Some(w) = window {
                    ensure(
                        w.iter().all(|v| v.is_finite() && v.abs() <= 1e6)
                            && w[0] < w[1]
                            && (w[1] - w[0]).is_finite(),
                        "invalid scatter ordinal window",
                    )?;
                }
                Ok(())
            }
        }
    }
    pub(super) fn resolve(
        &self,
        visible: Vec<CategoryKey>,
        all: Vec<CategoryKey>,
        range: [f64; 2],
    ) -> Result<ResolvedScatterAxis> {
        match self {
            Self::Piecewise { options } => {
                let keys = if visible.is_empty() { &all } else { &visible };
                ensure(
                    keys.iter().all(|key| matches!(key, CategoryKey::Number(_))),
                    "piecewise scatter axis requires numeric keys",
                )?;
                Ok(ResolvedScatterAxis::Continuous(
                    crate::line_scales::resolve_plot_piecewise(options, range)?,
                ))
            }
            Self::Continuous { options, min, max } => {
                let numbers = |keys: &[CategoryKey]| -> Result<Vec<f64>> {
                    keys.iter()
                        .map(|key| match key {
                            CategoryKey::Number(v) => Ok(*v),
                            _ => Err(ChartError(
                                "continuous scatter axis requires numeric keys".into(),
                            )),
                        })
                        .collect()
                };
                let source = numbers(&visible)?;
                let source = if source.is_empty() {
                    numbers(&all)?
                } else {
                    source
                };
                let fallback = match options.kind {
                    kind if kind.requires_nonzero_domain() => [1., 10.],
                    ScaleKind::Utc => [0., 86_400_000.],
                    _ => [0., 1.],
                };
                let lo = source
                    .iter()
                    .copied()
                    .reduce(f64::min)
                    .unwrap_or(fallback[0]);
                let hi = source
                    .iter()
                    .copied()
                    .reduce(f64::max)
                    .unwrap_or(fallback[1]);
                let domain = [
                    if let ScatterBound::Fixed(v) = min {
                        *v
                    } else {
                        lo
                    },
                    if let ScatterBound::Fixed(v) = max {
                        *v
                    } else {
                        hi
                    },
                ];
                Ok(ResolvedScatterAxis::Continuous(
                    ResolvedContinuousScale::new(domain, range, options)?,
                ))
            }
            Self::Point {
                options,
                domain,
                window,
            } => Ok(ResolvedScatterAxis::Point(ScatterPointScale {
                base: PointScale::new(domain.clone().unwrap_or(visible), range, options)?,
                window: *window,
                round: options.round,
            })),
        }
    }
    pub(super) fn validate_raw(&self, key: &CategoryKey) -> Result<()> {
        key.validate()?;
        let numeric_kind = match self {
            Self::Continuous { options, .. } => Some(options.kind),
            Self::Piecewise { options } => Some(options.continuous.kind),
            Self::Point { .. } => None,
        };
        if let Some(kind) = numeric_kind {
            let CategoryKey::Number(value) = key else {
                return Err(ChartError(
                    "continuous scatter axis requires numeric keys".into(),
                ));
            };
            if kind == ScaleKind::Utc {
                ensure(
                    value.fract() == 0. && value.abs() <= MAX_TIMESTAMP_MS as f64,
                    "scatter UTC datum must be integral and bounded",
                )?;
            }
        }
        Ok(())
    }
}
impl ScatterPointScale {
    pub fn domain(&self) -> &[CategoryKey] {
        self.base.domain()
    }
    pub fn range(&self) -> [f64; 2] {
        self.base.range()
    }
    fn full_window(&self) -> [f64; 2] {
        match self.domain().len() {
            0 => [0., 1.],
            1 => [-0.5, 0.5],
            n => [0., (n - 1) as f64],
        }
    }
    pub fn window(&self) -> [f64; 2] {
        self.window.unwrap_or_else(|| self.full_window())
    }
    pub fn original_scale(&self) -> &PointScale {
        &self.base
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
        let p = crate::scale_options::interpolate(window, range, logical, false);
        Some(if self.round {
            crate::scale_options::js_round(p)
        } else {
            p
        })
    }
}
impl ResolvedScatterAxis {
    pub fn map(&self, key: &CategoryKey) -> Result<Option<f64>> {
        match self {
            Self::Continuous(scale) => {
                let CategoryKey::Number(v) = key else {
                    return Err(ChartError(
                        "continuous scatter axis requires numeric keys".into(),
                    ));
                };
                ensure(
                    scale.chart_scale().accepts(*v),
                    "scatter datum is invalid for continuous scale",
                )?;
                let p = scale.map(*v);
                ensure(p.is_finite(), "scatter projection overflow")?;
                Ok(Some(p))
            }
            Self::Point(scale) => {
                key.validate()?;
                let p = scale.map(key);
                ensure(
                    p.is_none_or(f64::is_finite),
                    "scatter point projection overflow",
                )?;
                Ok(p)
            }
        }
    }
    pub fn range(&self) -> [f64; 2] {
        match self {
            Self::Continuous(s) => s.range(),
            Self::Point(s) => s.range(),
        }
    }
    pub fn chart_scale(&self) -> Option<&ChartScale> {
        if let Self::Continuous(s) = self {
            Some(s.chart_scale())
        } else {
            None
        }
    }
    pub fn point_scale(&self) -> Option<&PointScale> {
        if let Self::Point(s) = self {
            Some(s.original_scale())
        } else {
            None
        }
    }
    pub fn source_domain(&self) -> Option<[f64; 2]> {
        if let Self::Continuous(s) = self {
            Some(s.source_domain())
        } else {
            None
        }
    }
    pub fn display_domain(&self) -> Option<[f64; 2]> {
        if let Self::Continuous(s) = self {
            Some(s.display_domain())
        } else {
            None
        }
    }
    pub fn category_domain(&self) -> Option<&[CategoryKey]> {
        if let Self::Point(s) = self {
            Some(s.domain())
        } else {
            None
        }
    }
    pub fn window(&self) -> Option<[f64; 2]> {
        if let Self::Point(s) = self {
            Some(s.window())
        } else {
            None
        }
    }
    /// Continuous data coordinates, or explicit ordinal window coordinates.
    /// This never replaces the typed raw values in sites/tooltips.
    pub fn coordinate_domain(&self) -> [f64; 2] {
        match self {
            Self::Continuous(s) => s.display_domain(),
            Self::Point(s) => s.window(),
        }
    }
    pub fn invert_coordinate(&self, pixel: f64) -> Result<f64> {
        ensure(pixel.is_finite(), "invalid scatter inverse pixel")?;
        match self {
            Self::Continuous(s) => s.invert(pixel),
            Self::Point(s) => {
                let v = crate::scale_options::interpolate(s.range(), s.window(), pixel, false);
                ensure(v.is_finite(), "scatter ordinal inverse overflow")?;
                Ok(v)
            }
        }
    }
    pub(super) fn format(&self, key: &CategoryKey, format: &ValueFormat) -> Result<String> {
        match key {
            CategoryKey::String(s) => {
                ensure(
                    s.len() <= MAX_LABEL_BYTES,
                    "scatter coordinate label budget exceeded",
                )?;
                Ok(s.clone())
            }
            CategoryKey::Number(n) => {
                if self
                    .chart_scale()
                    .is_some_and(|s| s.kind() == ScaleKind::Utc)
                    && *format == ValueFormat::Auto
                {
                    ValueFormat::Utc.format(*n)
                } else {
                    format.format(*n)
                }
            }
        }
    }
    pub(super) fn append_axis(
        &self,
        scene: &mut Scene,
        c: &ChartConfig,
        side: AxisSide,
        o: &AxisOptions,
        id: &str,
    ) -> Result<()> {
        if let Self::Continuous(s) = self {
            return axis(scene, c, s.chart_scale(), side, o, id);
        }
        ensure(
            o.tick_values.is_none() && o.tick_calendar.is_none(),
            "point scatter axes require category domain/tick count, not numeric tick values or calendars",
        )?;
        let Self::Point(scale) = self else {
            unreachable!()
        };
        let horizontal = matches!(side, AxisSide::Top | AxisSide::Bottom);
        let direction = if matches!(side, AxisSide::Top | AxisSide::Left) {
            -1.
        } else {
            1.
        };
        let position = match side {
            AxisSide::Top => c.margin.top,
            AxisSide::Bottom => c.height - c.margin.bottom,
            AxisSide::Left => c.margin.left,
            AxisSide::Right => c.width - c.margin.right,
        };
        let stride = scale.domain().len().div_ceil(o.tick_count.max(1)).max(1);
        if o.tick_count > 0 {
            for key in scale.domain().iter().step_by(stride) {
                let pixel = scale.map(key).unwrap();
                let lo = if horizontal {
                    c.margin.left
                } else {
                    c.margin.top
                };
                let hi = if horizontal {
                    c.width - c.margin.right
                } else {
                    c.height - c.margin.bottom
                };
                if pixel < lo || pixel > hi {
                    continue;
                }
                let identity = key.identity()?;
                ensure(
                    identity.len() + id.len() + 12 <= MAX_LABEL_BYTES,
                    "scatter point tick ID budget exceeded",
                )?;
                if o.grid {
                    path(
                        scene,
                        format!("{id}-grid:{identity}"),
                        if horizontal {
                            [pixel, c.margin.top]
                        } else {
                            [c.margin.left, pixel]
                        },
                        if horizontal {
                            [pixel, c.height - c.margin.bottom]
                        } else {
                            [c.width - c.margin.right, pixel]
                        },
                        c.theme.grid,
                    )?;
                }
                if o.visible {
                    path(
                        scene,
                        format!("{id}-tick:{identity}"),
                        if horizontal {
                            [pixel, position]
                        } else {
                            [position, pixel]
                        },
                        if horizontal {
                            [pixel, position + direction * o.tick_size]
                        } else {
                            [position + direction * o.tick_size, pixel]
                        },
                        c.theme.text,
                    )?;
                    ensure(
                        scene.labels.len() < MAX_MARKS,
                        "scatter label budget exceeded",
                    )?;
                    scene.labels.push(Label {
                        x: if horizontal {
                            pixel
                        } else {
                            position + direction * (o.tick_size + o.tick_padding)
                        },
                        y: if horizontal {
                            position
                                + direction * (o.tick_size + o.tick_padding)
                                + if direction > 0. {
                                    c.theme.font_size
                                } else {
                                    0.
                                }
                        } else {
                            pixel + c.theme.font_size * 0.33
                        },
                        text: self.format(key, &o.format)?,
                        size: c.theme.font_size,
                        color: c.theme.text,
                        anchor: if horizontal {
                            TextAnchor::Middle
                        } else if direction < 0. {
                            TextAnchor::End
                        } else {
                            TextAnchor::Start
                        },
                        rotation: o.tick_rotation,
                    });
                }
            }
        }
        if o.visible {
            path(
                scene,
                id.into(),
                if horizontal {
                    [c.margin.left, position]
                } else {
                    [position, c.margin.top]
                },
                if horizontal {
                    [c.width - c.margin.right, position]
                } else {
                    [position, c.height - c.margin.bottom]
                },
                c.theme.text,
            )?;
            if let Some(text) = &o.legend {
                ensure(
                    scene.labels.len() < MAX_MARKS,
                    "scatter label budget exceeded",
                )?;
                scene.labels.push(Label {
                    x: if horizontal {
                        (c.margin.left + c.width - c.margin.right) / 2.
                    } else {
                        position
                            + direction * (o.tick_size + o.tick_padding + c.theme.font_size * 2.5)
                    },
                    y: if horizontal {
                        position
                            + direction * (o.tick_size + o.tick_padding + c.theme.font_size * 2.5)
                    } else {
                        (c.margin.top + c.height - c.margin.bottom) / 2.
                    },
                    text: text.clone(),
                    size: c.theme.font_size,
                    color: c.theme.text,
                    anchor: TextAnchor::Middle,
                    rotation: if horizontal { 0. } else { -90. },
                });
            }
        }
        Ok(())
    }
}
fn path(scene: &mut Scene, id: String, a: [f64; 2], b: [f64; 2], color: Color) -> Result<()> {
    ensure(
        scene.marks.len() < MAX_MARKS,
        "scatter axis mark budget exceeded",
    )?;
    let mut m = Mark::new(
        id,
        Geometry::Path(vec![
            C::Move(P::new(a[0], a[1])),
            C::Line(P::new(b[0], b[1])),
        ]),
        color,
    );
    m.fill = None;
    m.label.clear();
    m.stroke = Some(GraphicStroke {
        color,
        width: 1.,
        cap: StrokeCap::Butt,
    });
    scene.marks.push(m);
    Ok(())
}
