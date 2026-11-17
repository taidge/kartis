use super::*;
fn finite(value: f64) -> bool {
    value.is_finite() && value.abs() <= 1_000_000.
}
fn nonnegative(value: f64) -> bool {
    finite(value) && value >= 0.
}
pub(super) fn preflight(
    frame: &FunnelSvgFrame,
    specs: &[FunnelAnnotationSpec],
    factories: &FunnelAnnotationFactories<'_>,
    theme: &FunnelAnnotationTheme,
    budget: FunnelAnnotationBudget,
) -> Result<()> {
    ensure(
        budget.specs <= 1024
            && budget.bindings <= MAX_MARKS
            && budget.match_work <= 2_000_000
            && budget.marks <= MAX_MARKS
            && budget.labels <= MAX_MARKS
            && budget.path_commands <= 200_000,
        "invalid Funnel annotation budget",
    )?;
    ensure(
        specs.len() <= budget.specs,
        "Funnel annotation spec budget exceeded",
    )?;
    ensure(
        specs
            .len()
            .checked_mul(frame.parts().len())
            .is_some_and(|n| n <= budget.match_work),
        "Funnel annotation match work budget exceeded",
    )?;
    for style in [&theme.outline, &theme.symbol, &theme.link] {
        ensure(
            [style.stroke_width, style.outline_width]
                .into_iter()
                .all(|v| v.is_finite() && (0. ..=10_000.).contains(&v))
                && style.stroke_width + 2. * style.outline_width <= 10_000.
                && [style.stroke_width, 2. * style.outline_width]
                    .into_iter()
                    .all(|v| v == 0. || v >= 0.000001)
                && [style.opacity, style.outline_opacity]
                    .into_iter()
                    .all(|v| v.is_finite() && (0. ..=1.).contains(&v)),
            "invalid Funnel annotation paint style",
        )?;
    }
    theme.text.font.validate()?;
    ensure(
        theme.text.font_size.is_finite()
            && (1. ..=256.).contains(&theme.text.font_size)
            && theme.text.outline_width.is_finite()
            && (0. ..=2048.).contains(&theme.text.outline_width),
        "invalid Funnel annotation text style",
    )?;
    for spec in specs {
        match &spec.matcher {
            FunnelAnnotationMatch::Id(id) => id.validate()?,
            FunnelAnnotationMatch::Predicate(key) => {
                ensure(
                    !key.is_empty() && key.len() <= MAX_LABEL_BYTES,
                    "invalid Funnel annotation predicate key",
                )?;
                ensure(
                    factories.matcher.is_some(),
                    "Funnel annotation predicate factory is missing",
                )?;
            }
            FunnelAnnotationMatch::All => {}
        }
        match &spec.note {
            FunnelAnnotationNote::Text(text) => ensure(
                text.len() <= MAX_LABEL_BYTES,
                "Funnel annotation note text budget exceeded",
            )?,
            FunnelAnnotationNote::Custom(key) => {
                ensure(
                    !key.is_empty() && key.len() <= MAX_LABEL_BYTES,
                    "invalid Funnel annotation note key",
                )?;
                ensure(
                    factories.note.is_some(),
                    "Funnel annotation note renderer is missing",
                )?;
            }
        }
        for position in [spec.note_x, spec.note_y] {
            let (FunnelAnnotationPosition::Relative(v) | FunnelAnnotationPosition::Absolute(v)) =
                position;
            ensure(finite(v), "invalid Funnel annotation note position")?;
        }
        ensure(
            [spec.x, spec.y].into_iter().flatten().all(finite)
                && [spec.width, spec.height, spec.size]
                    .into_iter()
                    .flatten()
                    .all(nonnegative)
                && finite(spec.offset)
                && nonnegative(spec.note_width)
                && finite(spec.note_text_offset)
                && nonnegative(spec.border_radius),
            "invalid Funnel annotation specification",
        )?;
    }
    Ok(())
}
pub(super) fn bind(
    frame: &FunnelSvgFrame,
    specs: &[FunnelAnnotationSpec],
    factories: &mut FunnelAnnotationFactories<'_>,
    budget: FunnelAnnotationBudget,
) -> Result<Vec<BoundFunnelAnnotation>> {
    let mut bindings = Vec::new();
    for (spec_index, spec) in specs.iter().enumerate() {
        for (part_index, part) in frame.parts().iter().enumerate() {
            let selected = match &spec.matcher {
                FunnelAnnotationMatch::All => true,
                FunnelAnnotationMatch::Id(id) => id == &part.datum.id,
                FunnelAnnotationMatch::Predicate(key) => {
                    factories.matcher.as_mut().ok_or_else(|| {
                        ChartError("Funnel annotation predicate factory is missing".into())
                    })?(FunnelAnnotationMatchContext {
                        key,
                        spec_index,
                        spec,
                        frame,
                        part_index,
                        source_index: part.source_index,
                        part,
                        collection: frame.parts(),
                    })?
                }
            };
            if !selected {
                continue;
            }
            ensure(
                bindings.len() < budget.bindings,
                "Funnel annotation binding budget exceeded",
            )?;
            let offset = if spec.kind == FunnelAnnotationKind::Dot {
                0.
            } else {
                spec.offset * 2.
            };
            let width = part.bounds.width + offset;
            let height = part.bounds.height + offset;
            let derived = part.bounds.width.max(part.bounds.height) + offset;
            let size = spec.size.filter(|v| *v != 0.).unwrap_or(derived);
            ensure(
                [size, width, height].into_iter().all(nonnegative),
                "invalid bound Funnel annotation dimensions",
            )?;
            let position = |value: FunnelAnnotationPosition, axis: usize| match value {
                FunnelAnnotationPosition::Relative(value) => part.center[axis] + value,
                FunnelAnnotationPosition::Absolute(value) => {
                    [frame.plot().x, frame.plot().y][axis] + value
                }
            };
            let note = [position(spec.note_x, 0), position(spec.note_y, 1)];
            let computed = compute(part.center, note, spec, size, width, height)?;
            bindings.push(BoundFunnelAnnotation {
                spec_index,
                part_index,
                source_index: part.source_index,
                id: part.datum.id.clone(),
                kind: spec.kind,
                note: spec.note.clone(),
                center: part.center,
                size,
                width,
                height,
                border_radius: spec.border_radius,
                note_width: spec.note_width,
                note_text_offset: spec.note_text_offset,
                computed,
            });
        }
    }
    Ok(bindings)
}
fn compute(
    center: [f64; 2],
    note: [f64; 2],
    spec: &FunnelAnnotationSpec,
    size: f64,
    width: f64,
    height: f64,
) -> Result<FunnelAnnotationInstructions> {
    let mut angle =
        (180. * (note[1] - center[1]).atan2(note[0] - center[0])) / std::f64::consts::PI % 360.;
    if angle < 0. {
        angle += 360.;
    }
    let mut start = center;
    match spec.kind {
        FunnelAnnotationKind::Circle => {
            let radians = (angle * std::f64::consts::PI) / 180.;
            start[0] += radians.cos() * size / 2.;
            start[1] += radians.sin() * size / 2.;
        }
        FunnelAnnotationKind::Rect => {
            let eighth = (((angle + 90.) / 45. + 0.5).floor() as usize) % 8;
            let dx = [0., 1., 1., 1., 0., -1., -1., -1.][eighth];
            let dy = [-1., -1., 0., 1., 1., 1., 0., -1.][eighth];
            start[0] += dx * width / 2.;
            start[1] += dy * height / 2.;
        }
        FunnelAnnotationKind::Dot => {}
    }
    let left = (angle + 90.).rem_euclid(360.) > 180.;
    let text = [
        note[0] - if left { spec.note_width } else { 0. },
        note[1] - spec.note_text_offset,
    ];
    let tail = [
        note[0]
            + if left {
                -spec.note_width
            } else {
                spec.note_width
            },
        note[1],
    ];
    ensure(
        start
            .into_iter()
            .chain(note)
            .chain(tail)
            .chain(text)
            .all(finite),
        "Funnel annotation coordinates exceed Scene limits",
    )?;
    Ok(FunnelAnnotationInstructions {
        points: [start, note, tail],
        text,
        angle: angle + 90.,
    })
}
