//! Interval geometry uses the parent combo's resolved scales and datum keys.
use super::*;

pub(super) fn validate_point(point: &ComboPoint, kind: ComboKind) -> Result<()> {
    let interval = matches!(
        kind,
        ComboKind::ErrorBars | ComboKind::BoxPlot | ComboKind::RangeBand
    );
    if point.y.is_none() {
        return ensure(
            point.spread.is_none(),
            "a combo gap cannot carry interval data",
        );
    }
    ensure(
        interval == point.spread.is_some(),
        "interval combo kinds require bounds; scalar kinds do not accept interval data",
    )?;
    if let Some(spread) = point.spread {
        let values = spread.values();
        ensure(
            values.iter().all(|v| v.is_finite()) && values.windows(2).all(|v| v[0] <= v[1]),
            "combo interval endpoints must be finite and ordered",
        )?;
        let y = point.y.unwrap();
        let [low, high] = spread.bounds();
        ensure(
            y >= low && y <= high,
            "combo datum value must lie within its interval",
        )?;
        ensure(
            matches!((kind,spread),(ComboKind::BoxPlot,ComboSpread::Box{median,..}) if median==y)
                || matches!(
                    (kind, spread),
                    (
                        ComboKind::RangeBand | ComboKind::ErrorBars,
                        ComboSpread::Range { .. }
                    )
                ),
            "combo interval summary does not match its series kind",
        )?;
    }
    Ok(())
}

pub(super) fn identity(series: &ComboSeries, row: &Row) -> String {
    format!(
        "combo:{}:{}:datum:{}",
        series.id.len(),
        series.id,
        row.datum_id
    )
}
fn tooltip(
    series: &ComboSeries,
    row: &Row,
    axis: &YAxisOptions,
    kind: ScaleKind,
) -> Result<String> {
    let fmt = |v| formatted(&axis.axis.format, kind, v);
    let spread = row.spread.unwrap();
    let text = match spread {
        ComboSpread::Range { low, high } => format!(
            "{} · {}: {} [{} – {}]",
            series.id,
            row.x_label,
            fmt(row.y.unwrap())?,
            fmt(low)?,
            fmt(high)?
        ),
        ComboSpread::Box {
            low,
            q1,
            median,
            q3,
            high,
        } => format!(
            "{} · {}: median {}; Q1 {}; Q3 {}; whiskers {} – {}",
            series.id,
            row.x_label,
            fmt(median)?,
            fmt(q1)?,
            fmt(q3)?,
            fmt(low)?,
            fmt(high)?
        ),
    };
    ensure(text.len() <= 1024, "combo tooltip byte budget exceeded")?;
    Ok(text)
}
fn with_stroke(width: f64, color: Color) -> Option<GraphicStroke> {
    (width > 0.).then(|| GraphicStroke {
        width,
        ..stroke(color)
    })
}
fn segment(commands: &mut Vec<Cmd>, a: [f64; 2], b: [f64; 2], clip: ClipRect) -> Result<()> {
    if let Some([a, b]) = crate::clipping::clip_segment((a[0], a[1]), (b[0], b[1]), clip)? {
        commands.extend([Cmd::Move(P::new(a.0, a.1)), Cmd::Line(P::new(b.0, b.1))]);
    }
    Ok(())
}

/// No curve approximation is used: this envelope has explicit linear edges.
fn range_band(
    series: &ComboSeries,
    rows: &[Row],
    x: &XEngine,
    ys: &ChartScale,
    clip: ClipRect,
    color: Color,
) -> Result<Vec<(usize, Mark)>> {
    let mut runs = Vec::<Vec<&Row>>::new();
    let mut run = vec![];
    let mut previous = None;
    for row in rows {
        let gap = row.y.is_none() || matches!((previous,row.ordinal),(Some(a),Some(b)) if b>a+1);
        if gap && !run.is_empty() {
            runs.push(std::mem::take(&mut run));
        }
        if row.y.is_some() {
            run.push(row);
        }
        previous = row.ordinal;
    }
    if !run.is_empty() {
        runs.push(run);
    }
    let mut marks = vec![];
    for run in runs.into_iter().filter(|run| run.len() > 1) {
        let point = |row: &&Row, upper: bool| -> Result<P> {
            let px = match x {
                XEngine::Numeric(scale) => scale.map(row.x),
                _ => row.x,
            };
            let bounds = row.spread.unwrap().bounds();
            let py = ys.map(bounds[usize::from(upper)]);
            ensure(
                [px, py]
                    .into_iter()
                    .all(|v| v.is_finite() && v.abs() <= 900000.),
                "combo range projection exceeds coordinate budget; narrow the input before rendering",
            )?;
            Ok(P::new(px, py))
        };
        let mut points = run
            .iter()
            .map(|row| point(row, true))
            .collect::<Result<Vec<_>>>()?;
        points.extend(
            run.iter()
                .rev()
                .map(|row| point(row, false))
                .collect::<Result<Vec<_>>>()?,
        );
        if points.iter().all(|p| p.x < clip.x)
            || points.iter().all(|p| p.x > clip.x + clip.width)
            || points.iter().all(|p| p.y < clip.y)
            || points.iter().all(|p| p.y > clip.y + clip.height)
        {
            continue;
        }
        let mut commands = vec![Cmd::Move(points[0])];
        commands.extend(points.into_iter().skip(1).map(Cmd::Line));
        commands.push(Cmd::Close);
        // Key by the first stable datum, never by source row ordinal.
        let mut mark = Mark::new(
            format!(
                "combo:{}:{}:band:{}",
                series.id.len(),
                series.id,
                run[0].datum_id
            ),
            Geometry::Path(commands),
            color,
        );
        mark.series = series.id.clone();
        mark.label.clear();
        mark.opacity = series.style.area_opacity * series.style.opacity;
        mark.stroke = with_stroke(series.style.line_width, color);
        mark.clip = Some(clip);
        marks.push((0, mark));
    }
    Ok(marks)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn render(
    c: &ChartConfig,
    series: &ComboSeries,
    rows: &[Row],
    x: &XEngine,
    ys: &ChartScale,
    axis: &YAxisOptions,
    clip: ClipRect,
    band: f64,
    width: f64,
    box_slot: Option<usize>,
    radius: f64,
    color: Color,
    configured: bool,
) -> Result<Vec<(usize, Mark)>> {
    let mut marks = if series.kind == ComboKind::RangeBand {
        range_band(series, rows, x, ys, clip, color)?
    } else {
        vec![]
    };
    let domain = ordered_domain(ys);
    let unclamped = (configured && ys.is_clamped() && series.kind == ComboKind::BoxPlot)
        .then(|| ys.clone().clamped(false));
    let box_projection = unclamped.as_ref().unwrap_or(ys);
    for row in rows {
        let Some(value) = row.y else {
            continue;
        };
        let center = if series.kind == ComboKind::BoxPlot {
            x.bar_pixel(row.x, configured)
        } else {
            x.pixel(row.x, configured)
        };
        let Some(center) = center else {
            continue;
        };
        let spread = row.spread.unwrap();
        let [low, high] = spread.bounds();
        if high < domain[0] || low > domain[1] {
            continue;
        }
        let id = identity(series, row);
        let text = tooltip(series, row, axis, ys.kind())?;
        let mapped = |v: f64| ys.map(v.clamp(domain[0], domain[1]));
        let mut datum = match spread {
            ComboSpread::Box { q1, median, q3, .. } => {
                let cx = center - band / 2. + (box_slot.unwrap() as f64 + 0.5) * width;
                if cx + width / 2. <= clip.x || cx - width / 2. >= clip.x + clip.width {
                    continue;
                }
                let body_visible = q3 >= domain[0] && q1 <= domain[1];
                // Keep the actual box edges. Cropping the rectangle itself
                // would add a horizontal border at an artificial domain edge.
                let a = if body_visible {
                    box_projection.map(q1)
                } else {
                    mapped(q1)
                };
                let b = if body_visible {
                    box_projection.map(q3)
                } else {
                    mapped(q3)
                };
                ensure(
                    [a, b]
                        .into_iter()
                        .all(|v| v.is_finite() && v.abs() <= 900000.),
                    "combo box projection exceeds coordinate budget; narrow the input before rendering",
                )?;
                let mut body = Mark::new(
                    id.clone(),
                    Geometry::Rect {
                        x: cx - width / 2.,
                        y: a.min(b),
                        width,
                        height: (a - b).abs(),
                    },
                    color,
                );
                body.stroke = with_stroke(series.style.line_width, color);
                body.opacity = series.style.opacity;
                if !body_visible {
                    body.fill = None;
                    body.stroke = None;
                }
                let body_has_ink = body_visible
                    && (a != b && width > 0. || body.stroke.is_some() && (a != b || width > 0.));
                let mut datum_fallback = None;
                // Each cap is emitted only if its actual endpoint is visible;
                // clipped domain boundaries do not acquire invented caps.
                let mut whiskers = vec![];
                for [a, b] in [[low, q1], [q3, high]] {
                    if configured && (b <= domain[0] || a >= domain[1]) {
                        continue;
                    }
                    let [a, b] = [mapped(a), mapped(b)];
                    if !configured || a != b {
                        segment(&mut whiskers, [cx, a], [cx, b], clip)?;
                    }
                }
                let half = width * series.style.whisker_end_size / 2.;
                if half > 0. {
                    for end in [low, high]
                        .into_iter()
                        .filter(|v| *v >= domain[0] && *v <= domain[1])
                    {
                        segment(
                            &mut whiskers,
                            [cx - half, mapped(end)],
                            [cx + half, mapped(end)],
                            clip,
                        )?;
                    }
                }
                if !whiskers.is_empty() && series.style.whisker_width > 0. {
                    let mut mark =
                        Mark::new(format!("{id}:whiskers"), Geometry::Path(whiskers), color);
                    mark.fill = None;
                    mark.stroke = with_stroke(series.style.whisker_width, color);
                    mark.series = series.id.clone();
                    mark.label = text.clone();
                    mark.value = median;
                    mark.opacity = series.style.opacity;
                    mark.clip = Some(clip);
                    if configured && !body_has_ink {
                        datum_fallback = Some(mark);
                    } else {
                        if configured {
                            mark.label.clear();
                        }
                        marks.push((2, mark));
                    }
                }
                if (domain[0]..=domain[1]).contains(&median) && series.style.median_width > 0. {
                    let mut commands = vec![];
                    segment(
                        &mut commands,
                        [cx - width / 2., mapped(median)],
                        [cx + width / 2., mapped(median)],
                        clip,
                    )?;
                    let mut mark =
                        Mark::new(format!("{id}:median"), Geometry::Path(commands), color);
                    mark.fill = None;
                    mark.stroke = with_stroke(
                        series.style.median_width,
                        series.style.median_color.unwrap_or(c.theme.text),
                    );
                    mark.series = series.id.clone();
                    mark.label.clear();
                    mark.value = median;
                    mark.opacity = series.style.opacity;
                    mark.clip = Some(clip);
                    if configured && !body_has_ink && datum_fallback.is_none() {
                        datum_fallback = Some(mark);
                    } else {
                        marks.push((3, mark));
                    }
                }
                if configured && !body_has_ink {
                    // Keep one stable business identity per raw summary. A
                    // clipped-away body delegates to real surviving ink.
                    let Some(mut fallback) = datum_fallback else {
                        continue;
                    };
                    fallback.id = id;
                    fallback
                } else {
                    body
                }
            }
            ComboSpread::Range { .. } if series.kind == ComboKind::ErrorBars => {
                if configured && series.style.whisker_width == 0. {
                    continue;
                }
                let mut commands = vec![];
                segment(
                    &mut commands,
                    [center, mapped(low)],
                    [center, mapped(high)],
                    clip,
                )?;
                let half = series.style.error_cap_width / 2.;
                if half > 0. {
                    for end in [low, high]
                        .into_iter()
                        .filter(|v| *v >= domain[0] && *v <= domain[1])
                    {
                        segment(
                            &mut commands,
                            [center - half, mapped(end)],
                            [center + half, mapped(end)],
                            clip,
                        )?;
                    }
                }
                let mut mark = Mark::new(id.clone(), Geometry::Path(commands), color);
                mark.fill = None;
                mark.stroke = with_stroke(series.style.whisker_width, color);
                mark.opacity = series.style.opacity;
                mark
            }
            ComboSpread::Range { .. } => {
                if !(domain[0]..=domain[1]).contains(&value) || radius == 0. {
                    continue;
                }
                let mut mark = Mark::new(
                    id.clone(),
                    Geometry::Circle {
                        x: center,
                        y: mapped(value),
                        radius,
                    },
                    series.style.point_color.unwrap_or(color),
                );
                mark.stroke = series.style.point_border;
                mark.opacity = series.style.opacity;
                mark
            }
        };
        datum.series = series.id.clone();
        datum.label = text;
        datum.value = value;
        datum.clip = Some(clip);
        marks.push((
            if series.kind == ComboKind::BoxPlot {
                if matches!(datum.geometry, Geometry::Path(_)) {
                    2
                } else {
                    1
                }
            } else {
                3
            },
            datum,
        ));
    }
    Ok(marks)
}

pub(super) fn label_at(
    scene: &mut Scene,
    c: &ChartConfig,
    series: &ComboSeries,
    row: &Row,
    px: f64,
    py: f64,
    kind: Option<ScaleKind>,
) -> Result<()> {
    let Some(options) = &series.style.point_labels else {
        return Ok(());
    };
    let mut label = label(
        c,
        px + options.offset[0],
        py + options.offset[1],
        formatted(
            &options.format,
            kind.unwrap_or(ScaleKind::Linear),
            row.y.unwrap(),
        )?,
        TextAnchor::Middle,
    );
    label.color = options.color.unwrap_or(c.theme.text);
    label.size = options.size.unwrap_or(c.theme.font_size);
    scene
        .label_series
        .insert(scene.labels.len(), series.id.clone());
    scene.labels.push(label);
    Ok(())
}
#[allow(clippy::too_many_arguments)]
pub(super) fn point_labels(
    scene: &mut Scene,
    c: &ChartConfig,
    series: &ComboSeries,
    rows: &[Row],
    x: &XEngine,
    ys: &ChartScale,
    shift: Option<f64>,
    configured: bool,
) -> Result<()> {
    if series.style.point_labels.is_none() {
        return Ok(());
    }
    for row in rows {
        if let (Some(y), Some(px)) = (row.y, x.pixel(row.x, configured))
            && scale_contains(ys, y, configured)
        {
            label_at(
                scene,
                c,
                series,
                row,
                px + shift.unwrap_or(0.),
                ys.map(y),
                configured.then_some(ys.kind()),
            )?;
        }
    }
    Ok(())
}
