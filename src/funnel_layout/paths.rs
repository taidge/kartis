//! Explicit geometry adapters for the original four-sample Funnel profile.
use super::*;

fn sides(part: &FunnelPart) -> Result<([P; 4], [P; 4])> {
    ensure(
        part.profile.iter().all(|sample| {
            sample
                .before
                .iter()
                .chain(sample.after.iter())
                .all(|v| v.is_finite() && v.abs() <= 1e6)
        }),
        "invalid Funnel profile path coordinate",
    )?;
    Ok((
        part.profile.map(|sample| graphic_point(sample.before)),
        part.profile.map(|sample| graphic_point(sample.after)),
    ))
}

/// Generate the original closed stage profile with an explicit interpolation.
/// Equal sides return `None`, matching the original absent zero-width body.
/// The input always has four samples per side, with finite coordinates bounded
/// by the Scene coordinate budget. No layout or input is modified.
///
/// This adapts the profile role of Nivo's custom-layer area generator. It does
/// not recover unretained interpolation settings or return the current ink of
/// `funnel_layout_with_stage_factory`: that actual Mark may be an offset Path
/// or a different shape. Read the Scene Mark for current custom geometry.
pub fn funnel_part_geometry(
    part: &FunnelPart,
    interpolation: FunnelInterpolation,
) -> Result<Option<Geometry>> {
    let (mut low, high) = sides(part)?;
    if low == high {
        return Ok(None);
    }
    low.reverse();
    let curve = interpolation.into();
    let mut area = path(&high, curve);
    area.extend(path(&low, curve).into_iter().map(|command| match command {
        Cmd::Move(point) => Cmd::Line(point),
        command => command,
    }));
    area.push(Cmd::Close);
    Ok(Some(Geometry::Path(area)))
}

/// Generate the two original open profile sides, leaving both stage ends open.
/// The explicit interpolation and coordinate admission match
/// [`funnel_part_geometry`]. A collapsed profile still returns its two sides;
/// whether a border is painted depends on the caller's style configuration.
/// Custom stage ink can differ from these original profile borders. No React
/// generator object, animation state, paint or event handlers are synthesized.
pub fn funnel_part_border_geometry(
    part: &FunnelPart,
    interpolation: FunnelInterpolation,
) -> Result<Geometry> {
    let (low, mut high) = sides(part)?;
    high.reverse();
    let curve = interpolation.into();
    Ok(Geometry::Path(
        path(&low, curve)
            .into_iter()
            .chain(path(&high, curve))
            .collect(),
    ))
}
