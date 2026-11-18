//! Keep authored Bézier curves compatible as sector spans and ring sizes change.
use glory::render::{GraphicPoint as Point, PathCommand as Cmd};
use kartis::{Geometry, Scene};

use super::motion_geometry::{
    collapse_sector, linear_basis_edge, mix, normalize_sector, transform,
};

fn linear_funnel(geometry: &mut Geometry) {
    let Geometry::Path(path) = geometry else {
        return;
    };
    let closed = path.last() == Some(&Cmd::Close);
    let edges = &path[..path.len() - usize::from(closed)];
    if edges.len() != 8 {
        return;
    }
    let mut out = Vec::new();
    for edge in edges.as_chunks::<4>().0 {
        let Some(points) = edge
            .iter()
            .map(|cmd| match cmd {
                Cmd::Move(p) | Cmd::Line(p) => Some(*p),
                _ => None,
            })
            .collect::<Option<Vec<_>>>()
        else {
            return;
        };
        linear_basis_edge(&points, edge[0], &mut out);
    }
    if closed {
        out.push(Cmd::Close);
    }
    *path = out;
}

pub(super) fn prepare(kind: &str, variant: &str, scene: &mut Scene) {
    for mark in &mut scene.marks {
        match kind {
            "pie" | "radial-bar" | "polar-bar" => normalize_sector(&mut mark.geometry),
            "funnel" if variant == "linear" => linear_funnel(&mut mark.geometry),
            _ => {}
        }
    }
}

fn cell_center(path: &[Cmd]) -> Option<Point> {
    let points = path.iter().filter_map(|c| match c {
        Cmd::Move(p) | Cmd::Line(p) | Cmd::Cubic { end: p, .. } => Some(*p),
        Cmd::Close => None,
    });
    let mut bounds = None;
    for p in points {
        let (low, high) = bounds.get_or_insert((p, p));
        low.x = low.x.min(p.x);
        low.y = low.y.min(p.y);
        high.x = high.x.max(p.x);
        high.y = high.y.max(p.y);
    }
    bounds.map(|(low, high)| mix(low, high, 0.5))
}

pub(super) fn entrance(kind: &str, variant: &str, spacing: u32, target: &Scene, start: &mut Scene) {
    if !matches!(
        kind,
        "pie" | "radial-bar" | "polar-bar" | "waffle" | "funnel"
    ) {
        return;
    }
    let c = super::spaced_config(kind, target.width, target.height, spacing);
    let center = Point::new(
        c.margin.left + c.inner_width() / 2.,
        c.margin.top + c.inner_height() / 2.,
    );
    for (mark, original) in start.marks.iter_mut().zip(&target.marks) {
        let Geometry::Path(path) = &mut mark.geometry else {
            continue;
        };
        match kind {
            "waffle" if mark.id.starts_with("waffle-cell:") => {
                let Some(center) = cell_center(path) else {
                    continue;
                };
                transform(path, |_| center);
            }
            "funnel"
                if mark.id.starts_with("funnel:stage:")
                    || mark.id.starts_with("funnel:border:") =>
            {
                transform(path, |p| {
                    if variant == "horizontal" {
                        Point::new(p.x, center.y)
                    } else {
                        Point::new(center.x, p.y)
                    }
                });
            }
            "pie" if !mark.series.is_empty() => transform(path, |_| center),
            "radial-bar" | "polar-bar" if !mark.series.is_empty() => {
                // Pin the inside edge. The outside grows from its own ring.
                if !collapse_sector(path) {
                    continue;
                }
            }
            _ => continue,
        }
        mark.clip = original.clip;
        mark.opacity = original.opacity;
    }
}
