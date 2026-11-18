use PathCommand::{Close, Cubic, Line, Move};
use glory::render::{GraphicPoint, PathCommand};
use kartis::{Geometry, Scene};

use super::super::motion_geometry::collapse_sector;

pub(in super::super) fn entrance(kind: &str, target: &Scene, start: &mut Scene) {
    if !matches!(
        kind,
        "tree"
            | "network"
            | "sunburst"
            | "chord"
            | "sankey"
            | "icicle"
            | "treemap"
            | "circle-packing"
    ) {
        return;
    }
    for (mark, goal) in start.marks.iter_mut().zip(&target.marks) {
        match (kind, &goal.geometry, &mut mark.geometry) {
            ("network" | "tree", _, Geometry::Path(path))
                if goal.id.starts_with("force-link:") || goal.id.starts_with("link:") =>
            {
                // Nivo network links extend from the source toward the target.
                if let Some(Move(source)) = path.first().copied() {
                    for command in path {
                        collapse(command, source);
                    }
                }
                mark.opacity = 0.;
                mark.clip = goal.clip;
            }
            ("circle-packing" | "network", _, Geometry::Circle { .. }) => mark.opacity = 0.,
            (
                "icicle",
                Geometry::Rect {
                    x,
                    y,
                    width,
                    height,
                },
                geometry,
            ) => {
                // Nivo rects' flow-down transition moves each complete partition by one row.
                *geometry = Geometry::Rect {
                    x: *x,
                    y: y - height,
                    width: *width,
                    height: *height,
                };
                mark.opacity = 0.;
            }
            ("sunburst" | "chord", _, Geometry::Path(path))
                if goal.id.starts_with("node:") || goal.id.starts_with("group:") =>
            {
                if !collapse_sector(path) {
                    continue;
                }
                mark.opacity = goal.opacity;
                mark.clip = goal.clip;
            }
            ("chord", _, Geometry::Path(path)) if goal.id.starts_with("ribbon:") => {
                collapse_chord(path);
            }
            ("sankey", _, Geometry::Path(path)) if goal.id.starts_with("flow:") => {
                // Open both curved boundaries outwards from the stream's centerline.
                if let [
                    Move(left),
                    Cubic { first, second, end },
                    Line(right_bottom),
                    Cubic {
                        first: right_control,
                        second: left_control,
                        end: left_bottom,
                    },
                    Close,
                ] = path.as_mut_slice()
                {
                    let half = (left_bottom.y - left.y) / 2.;
                    left.y += half;
                    first.y += half;
                    second.y += half;
                    end.y += half;
                    right_bottom.y -= half;
                    right_control.y -= half;
                    left_control.y -= half;
                    left_bottom.y -= half;
                    mark.opacity = goal.opacity;
                    mark.clip = goal.clip;
                }
            }
            _ => {}
        }
    }
    // Owned labels appear with their nodes. Unowned annotations stay in place.
    for &index in target.label_series.keys() {
        start.labels[index].color.3 = 0;
    }
}

fn collapse(command: &mut PathCommand, point: GraphicPoint) {
    match command {
        Move(p) | Line(p) => *p = point,
        Cubic { first, second, end } => {
            *first = point;
            *second = point;
            *end = point;
        }
        Close => {}
    }
}

// Ribbons consist of two circular runs joined by cubics through the center.
// Start with the same connecting curve in both directions and zero angular width.
fn collapse_chord(path: &mut [PathCommand]) {
    let Some(Move(mut anchor)) = path.first().copied() else {
        return;
    };
    for command in &mut path[1..] {
        if let Cubic { first, second, end } = command
            && first == second
        {
            anchor = *end;
        } else {
            collapse(command, anchor);
        }
    }
}
