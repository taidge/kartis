use glory::render::{GraphicPoint as P, PathCommand as C};
use motion_geometry::{mix, split, transform};

use super::*;

fn straight(a: P, b: P) -> C {
    C::Cubic {
        first: mix(a, b, 1. / 3.),
        second: mix(a, b, 2. / 3.),
        end: b,
    }
}

/// Each observation interval occupies three cubic segments (the three legs of
/// a step). Exact de Casteljau subdivision preserves smooth curves at rest and
/// lets every preset interpolate corresponding intervals without an exit ghost.
pub(super) fn line_curve(mark: &mut Mark, variant: &str) {
    let Geometry::Path(path) = &mut mark.geometry else {
        return;
    };
    let Some(C::Move(mut current)) = path.first().copied() else {
        return;
    };
    let area = mark.id.starts_with("area-");
    let Some(top) = path.len().checked_sub(if area { 3 } else { 0 }) else {
        return;
    };
    let basis = variant == "basis";
    if top < if basis { 4 } else { 2 } {
        return;
    }
    let mut out = Vec::with_capacity(path.len());
    out.push(C::Move(current));
    if basis {
        let C::Line(lead) = path[1] else { return };
        out.push(C::Line(lead));
        current = lead;
    } else {
        out.push(C::Line(current));
    }
    let end = top - usize::from(basis);
    for cmd in &path[1 + usize::from(basis)..end] {
        match *cmd {
            C::Line(b) if variant == "step" => {
                out.push(straight(current, b));
                current = b;
            }
            C::Line(b) => {
                let first = mix(current, b, 1. / 3.);
                let second = mix(current, b, 2. / 3.);
                out.extend([
                    straight(current, first),
                    straight(first, second),
                    straight(second, b),
                ]);
                current = b;
            }
            C::Cubic { first, second, end } => {
                let (a, rest) = split([current, first, second, end], 1. / 3.);
                let (b, c) = split(rest, 0.5);
                for [_, first, second, end] in [a, b, c] {
                    out.push(C::Cubic { first, second, end });
                }
                current = end;
            }
            _ => return,
        }
    }
    out.push(if basis { path[end] } else { C::Line(current) });
    out.extend_from_slice(&path[top..]);
    *path = out;
}

fn endpoints(path: &[C]) -> impl Iterator<Item = P> + '_ {
    path.iter().filter_map(|cmd| match cmd {
        C::Move(p) | C::Line(p) | C::Cubic { end: p, .. } => Some(*p),
        C::Close => None,
    })
}

pub(super) fn entrance(kind: &str, variant: &str, spacing: u32, target: &Scene, start: &mut Scene) {
    if kind == "area" {
        for (mark, original) in start.marks.iter_mut().zip(&target.marks) {
            if mark.series.is_empty() {
                continue;
            }
            let Some(clip) = original.clip else { continue };
            let baseline = clip.y + clip.height;
            match &mut mark.geometry {
                Geometry::Path(path) => transform(path, |p| P::new(p.x, baseline)),
                Geometry::Circle { y, .. } => *y = baseline,
                _ => continue,
            }
            mark.clip = original.clip;
        }
    } else if matches!(kind, "line" | "stream" | "bump") {
        // The reveal mask advances along the data axis. Isolated points and
        // stream dots must not appear ahead of their own line or band.
        for mark in &mut start.marks {
            if !mark.series.is_empty()
                && matches!(mark.geometry, Geometry::Circle { .. })
                && let Some(clip) = &mut mark.clip
            {
                clip.width = 0.;
            }
        }
    } else if kind == "parallel-coordinates" && variant == "vertical" {
        for (mark, original) in start.marks.iter_mut().zip(&target.marks) {
            if mark.id.starts_with("parallel:")
                && let Some(mut clip) = original.clip
            {
                clip.height = 0.;
                mark.clip = Some(clip);
            }
        }
    } else if kind == "radar" {
        let c = spaced_config(kind, target.width, target.height, spacing);
        let center = P::new(
            c.margin.left + c.inner_width() / 2.,
            c.margin.top + c.inner_height() / 2.,
        );
        for (mark, source) in start.marks.iter_mut().zip(&target.marks) {
            if mark.series.is_empty() {
                continue;
            }
            match &mut mark.geometry {
                Geometry::Path(path) => transform(path, |_| center),
                Geometry::Circle { x, y, .. } => {
                    *x = center.x;
                    *y = center.y;
                }
                _ => continue,
            }
            mark.opacity = source.opacity;
        }
    } else if matches!(kind, "bar" | "marimekko") {
        let horizontal = matches!(variant, "horizontal" | "horizontal-stacked");
        for (mark, source) in start.marks.iter_mut().zip(&target.marks) {
            if !(mark.id.starts_with("bar-") || mark.id.starts_with("mekko-"))
                || mark.series.is_empty()
            {
                continue;
            }
            mark.geometry = source.geometry.clone();
            mark.opacity = source.opacity;
            mark.clip = source.clip;
            match &mut mark.geometry {
                Geometry::Rect {
                    x: _,
                    y,
                    width,
                    height,
                } => {
                    if horizontal {
                        *width = 0.;
                    } else {
                        *y += *height;
                        *height = 0.;
                    }
                }
                Geometry::Path(path) => {
                    // Rounded corners meet the leading and trailing edges so
                    // their endpoints include the true bounds of the bar.
                    if horizontal {
                        let left = endpoints(path).map(|p| p.x).fold(f64::INFINITY, f64::min);
                        transform(path, |p| P::new(left, p.y));
                    } else {
                        let bottom = endpoints(path)
                            .map(|p| p.y)
                            .fold(f64::NEG_INFINITY, f64::max);
                        transform(path, |p| P::new(p.x, bottom));
                    }
                }
                _ => {}
            }
        }
    }
}
