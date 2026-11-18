//! Path operations shared by the showcase motion presets.
use Cmd::{Close, Cubic, Line, Move};
use glory::render::{GraphicPoint as Point, PathCommand as Cmd};
use kartis::Geometry;

pub(super) fn mix(a: Point, b: Point, t: f64) -> Point {
    Point::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
}

pub(super) fn split([a, b, c, d]: [Point; 4], t: f64) -> ([Point; 4], [Point; 4]) {
    let (ab, bc, cd) = (mix(a, b, t), mix(b, c, t), mix(c, d, t));
    let (abc, bcd) = (mix(ab, bc, t), mix(bc, cd, t));
    let pivot = mix(abc, bcd, t);
    ([a, ab, abc, pivot], [pivot, bcd, cd, d])
}

// Twelve equal parameter intervals pair all possible authored banks (1–4
// quarter-circle cubics). De Casteljau subdivision preserves the exact curve.
const ARC_STEPS: usize = 12;

fn arc_bank(out: &mut Vec<Cmd>, mut start: Point, bank: &[Cmd]) {
    if bank.is_empty() {
        out.extend(
            [Cmd::Cubic {
                first: start,
                second: start,
                end: start,
            }; ARC_STEPS],
        );
        return;
    }
    for &cmd in bank {
        let Cmd::Cubic { first, second, end } = cmd else {
            unreachable!()
        };
        let mut curve = [start, first, second, end];
        for remaining in (1..=ARC_STEPS / bank.len()).rev() {
            let segment = if remaining == 1 {
                curve
            } else {
                let (left, right) = split(curve, 1. / remaining as f64);
                curve = right;
                left
            };
            let [_, first, second, end] = segment;
            out.push(Cmd::Cubic { first, second, end });
        }
        start = end;
    }
}

pub(super) fn normalize_sector(geometry: &mut Geometry) {
    let Geometry::Path(path) = geometry else {
        return;
    };
    let [Cmd::Move(start), edges @ .., Cmd::Close] = &path[..] else {
        return;
    };
    let Some(cap) = edges.iter().position(|c| matches!(c, Cmd::Line(_))) else {
        return;
    };
    let (outer, rest) = edges.split_at(cap);
    let [Cmd::Line(inner_start), inner @ ..] = rest else {
        return;
    };
    if !(1..=4).contains(&outer.len())
        || inner.len() > 4
        || !outer
            .iter()
            .chain(inner)
            .all(|c| matches!(c, Cmd::Cubic { .. }))
    {
        return;
    }
    let mut result = Vec::with_capacity(2 * ARC_STEPS + 3);
    result.push(Cmd::Move(*start));
    arc_bank(&mut result, *start, outer);
    result.push(Cmd::Line(*inner_start));
    arc_bank(&mut result, *inner_start, inner);
    result.push(Cmd::Close);
    *path = result;
}

pub(super) fn transform(path: &mut [Cmd], map: impl Fn(Point) -> Point) {
    for command in path {
        match command {
            Cmd::Move(p) | Cmd::Line(p) => *p = map(*p),
            Cmd::Cubic { first, second, end } => {
                *first = map(*first);
                *second = map(*second);
                *end = map(*end);
            }
            Cmd::Close => {}
        }
    }
}

// Starting the outer boundary on the reversed inner boundary keeps the sector
// circular as its thickness grows. A root disk starts at its center instead.
pub(super) fn collapse_sector(path: &mut [Cmd]) -> bool {
    let Some(divider) = path.iter().position(|cmd| matches!(cmd, Line(_))) else {
        return false;
    };
    let (outer, inner) = path.split_at_mut(divider);
    let [Line(inner_end), arc @ .., Close] = inner else {
        return false;
    };
    if arc.is_empty() {
        transform(outer, |_| *inner_end);
        return true;
    }
    let [Move(start), segments @ ..] = outer else {
        return false;
    };
    if segments.len() != arc.len() {
        return false;
    }
    let mut previous = *inner_end;
    for (out, &cmd) in segments.iter_mut().rev().zip(arc.iter()) {
        let Cubic { first, second, end } = cmd else {
            return false;
        };
        *out = Cubic {
            first: second,
            second: first,
            end: previous,
        };
        previous = end;
    }
    *start = previous;
    true
}

// D3's basis curve has a short straight lead-in/out and one cubic per
// interval. Express each linear interval with the same command slots so
// the spring can keep the original curve control points even mid-retarget.
pub(super) fn linear_basis_edge(points: &[Point], start: Cmd, out: &mut Vec<Cmd>) {
    out.push(start);
    out.push(Cmd::Line(points[0]));
    for pair in points.windows(2) {
        let [a, b] = [pair[0], pair[1]];
        out.push(Cmd::Cubic {
            first: mix(a, b, 1. / 3.),
            second: mix(a, b, 2. / 3.),
            end: b,
        });
    }
    out.push(Cmd::Line(points[points.len() - 1]));
}

/// Use the rounded rectangle command slots at radius zero, respecting the layout winding.
pub(super) fn square_corners(geometry: &mut Geometry, reverse: bool) {
    let Geometry::Rect {
        x,
        y,
        width,
        height,
    } = *geometry
    else {
        return;
    };
    let start = Point::new(x, y);
    let right = Point::new(x + width, y);
    let bottom = Point::new(x, y + height);
    let opposite = Point::new(x + width, y + height);
    let corners = if reverse {
        [bottom, opposite, right, start]
    } else {
        [right, opposite, bottom, start]
    };
    let mut commands = vec![Cmd::Move(start)];
    for end in corners {
        commands.push(Cmd::Line(end));
        commands.push(Cmd::Cubic {
            first: end,
            second: end,
            end,
        });
    }
    commands.push(Cmd::Close);
    *geometry = Geometry::Path(commands);
}
