use super::{Geometry, GraphicPoint, Mark, PathCommand, Scene};

/// Affine changes retain the command sequence, including the corners of rounded calendar cells.
fn scale(geometry: &mut Geometry, origin: GraphicPoint, sx: f64, sy: f64) {
    match geometry {
        Geometry::Path(commands) => {
            super::super::motion_geometry::transform(commands, |p| {
                GraphicPoint::new(
                    origin.x + (p.x - origin.x) * sx,
                    origin.y + (p.y - origin.y) * sy,
                )
            });
        }
        Geometry::Rect {
            x,
            y,
            width,
            height,
        } => {
            *x = origin.x + (*x - origin.x) * sx;
            *y = origin.y + (*y - origin.y) * sy;
            *width *= sx;
            *height *= sy;
        }
        Geometry::Circle { x, y, radius } => {
            *x = origin.x + (*x - origin.x) * sx;
            *y = origin.y + (*y - origin.y) * sy;
            *radius *= sx.min(sy);
        }
    }
}

fn segment(geometry: &Geometry) -> Option<(GraphicPoint, GraphicPoint)> {
    if let Geometry::Path(commands) = geometry
        && let [PathCommand::Move(a), PathCommand::Line(b), ..] = commands.as_slice()
    {
        return Some((*a, *b));
    }
    None
}

fn bullet(target: &Scene, start: &mut Scene) {
    // A marker is perpendicular to the value axis. The highest range touches the
    // upper bound of the track, including when the domain direction is reversed.
    let Some((a, b)) = target
        .marks
        .iter()
        .find(|m| m.id.contains(":marker:"))
        .and_then(|m| segment(&m.geometry))
    else {
        return;
    };
    let horizontal = a.x == b.x;
    let Some(high) = target
        .marks
        .iter()
        .filter(|m| m.id.contains(":range:"))
        .max_by(|a, b| a.value.total_cmp(&b.value))
    else {
        return;
    };
    let (
        Some(track),
        Geometry::Rect {
            x,
            y,
            width,
            height,
        },
    ) = (high.clip, &high.geometry)
    else {
        return;
    };
    let origin = GraphicPoint::new(
        if x + width / 2. < track.x + track.width / 2. {
            track.x + track.width
        } else {
            track.x
        },
        if y + height / 2. < track.y + track.height / 2. {
            track.y + track.height
        } else {
            track.y
        },
    );
    for (mark, authored) in start.marks.iter_mut().zip(&target.marks) {
        mark.clone_from(authored);
        if mark.id.starts_with("bullet:") {
            scale(
                &mut mark.geometry,
                origin,
                if horizontal { 0. } else { 1. },
                if horizontal { 1. } else { 0. },
            );
        }
    }
}

fn boxplot(target: &Scene, start: &mut Scene) {
    for median in &target.marks {
        let Some(group) = median.id.strip_suffix(":median") else {
            continue;
        };
        let Some((a, b)) = segment(&median.geometry) else {
            continue;
        };
        let horizontal = a.x == b.x;
        for (mark, authored) in start.marks.iter_mut().zip(&target.marks) {
            if let Some(suffix) = mark.id.strip_prefix(group)
                && (suffix.is_empty() || suffix.starts_with(':'))
            {
                mark.clone_from(authored);
                scale(
                    &mut mark.geometry,
                    a,
                    if horizontal { 0. } else { 1. },
                    if horizontal { 1. } else { 0. },
                );
            }
        }
    }
}

fn path_center(mark: &Mark) -> Option<GraphicPoint> {
    let Geometry::Path(commands) = &mark.geometry else {
        return None;
    };
    // The mean of a convex cell's vertices is inside the cell. Rounded day cells
    // have pairs of endpoints symmetric about the center of their rectangle.
    let mut sum = GraphicPoint::new(0., 0.);
    let mut count = 0.;
    for (i, command) in commands.iter().enumerate() {
        let p = match command {
            PathCommand::Move(p) | PathCommand::Line(p) => p,
            PathCommand::Cubic { end, .. } => end,
            PathCommand::Close => continue,
        };
        if i > 0 && commands.first() == Some(&PathCommand::Move(*p)) {
            continue;
        }
        sum.x += p.x;
        sum.y += p.y;
        count += 1.;
    }
    (count > 0.).then(|| GraphicPoint::new(sum.x / count, sum.y / count))
}

pub(super) fn apply(kind: &str, target: &Scene, start: &mut Scene) {
    match kind {
        "bullet" => bullet(target, start),
        "boxplot" => boxplot(target, start),
        "voronoi" => {
            for (mark, authored) in start.marks.iter_mut().zip(&target.marks) {
                if let Some(id) = mark.id.strip_prefix("cell:") {
                    let site = target.marks.iter().find_map(|candidate| {
                        if candidate.id.strip_prefix("point:") == Some(id)
                            && let Geometry::Circle { x, y, .. } = candidate.geometry
                        {
                            return Some(GraphicPoint::new(x, y));
                        }
                        None
                    });
                    if let Some(origin) = site.or_else(|| path_center(authored)) {
                        mark.clone_from(authored);
                        scale(&mut mark.geometry, origin, 0., 0.);
                    }
                } else if mark.id.starts_with("point:")
                    && let Geometry::Circle { radius, .. } = &mut mark.geometry
                {
                    *radius = 0.;
                }
            }
        }
        "delaunay" => {
            let origin = GraphicPoint::new(target.width / 2., target.height / 2.);
            for (mark, authored) in start.marks.iter_mut().zip(&target.marks) {
                if mark.id.starts_with("delaunay:") {
                    mark.clone_from(authored);
                    scale(&mut mark.geometry, origin, 0.2, 0.2);
                    mark.opacity = 0.;
                }
            }
            for (index, label) in start.labels.iter_mut().enumerate() {
                if target
                    .label_series
                    .get(&index)
                    .is_some_and(|id| id == "sites")
                {
                    label.x = origin.x + (label.x - origin.x) * 0.2;
                    label.y = origin.y + (label.y - origin.y) * 0.2;
                    label.color.3 = 0;
                }
            }
        }
        "calendar" | "time-range" => {
            for (mark, authored) in start.marks.iter_mut().zip(&target.marks) {
                if mark.series.starts_with("year:")
                    && let Some(origin) = path_center(authored)
                {
                    mark.clone_from(authored);
                    scale(&mut mark.geometry, origin, 0., 0.);
                }
            }
        }
        _ => {}
    }
}
