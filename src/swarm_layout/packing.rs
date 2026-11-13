use crate::scene::ensure;
use crate::{ChartError, Result};

#[derive(Clone, Copy)]
pub(super) struct Placed {
    pub primary: f64,
    pub cross: f64,
    pub radius: f64,
}

/// At a fixed primary coordinate, every older disc forbids an open interval
/// on the cross axis. Find the legal point nearest the lane center. Touching
/// interval endpoints are retained, since exact tangency is legal.
pub(super) fn place(
    primary: f64,
    radius: f64,
    spacing: f64,
    lane: [f64; 2],
    center: f64,
    placed: &[Placed],
    intervals: &mut Vec<[f64; 2]>,
) -> Result<f64> {
    let lo = lane[0] + radius;
    let hi = lane[1] - radius;
    ensure(lo <= hi, "swarm node envelope exceeds its group band")?;
    intervals.clear();
    for old in placed {
        let separation = radius + old.radius + spacing;
        let dx = (primary - old.primary).abs();
        if dx >= separation {
            continue;
        }
        // Factored form avoids cancellation in separation² − dx².
        let reach = ((separation - dx) * (separation + dx)).sqrt();
        let mut lower = old.cross - reach;
        let mut upper = old.cross + reach;
        // Repair an inward-rounded endpoint in world coordinates. There is
        // no absolute epsilon that would destroy a tight legal tangent slot.
        for _ in 0..4 {
            if dx.hypot(old.cross - lower) >= separation {
                break;
            }
            lower = lower.next_down();
        }
        for _ in 0..4 {
            if dx.hypot(upper - old.cross) >= separation {
                break;
            }
            upper = upper.next_up();
        }
        ensure(
            dx.hypot(old.cross - lower) >= separation && dx.hypot(upper - old.cross) >= separation,
            "swarm tangency cannot be represented at this coordinate",
        )?;
        if lower < hi && upper > lo {
            intervals.push([lower, upper]);
        }
    }
    intervals.sort_unstable_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
    let mut best = None::<f64>;
    let mut candidate = |start: f64, end: f64| {
        if start <= end {
            let q = center.clamp(start, end);
            if best.is_none_or(|old| {
                (q - center)
                    .abs()
                    .total_cmp(&(old - center).abs())
                    .then(q.total_cmp(&old))
                    .is_lt()
            }) {
                best = Some(q);
            }
        }
    };
    let mut cursor = lo;
    for &[start, end] in intervals.iter() {
        if end <= cursor {
            continue;
        }
        candidate(cursor, start.min(hi));
        cursor = cursor.max(end);
        if cursor > hi {
            break;
        }
    }
    candidate(cursor, hi);
    let cross =
        best.ok_or_else(|| ChartError("swarm group cannot fit without collisions".into()))?;
    ensure(
        placed.iter().all(|old| {
            (primary - old.primary).hypot(cross - old.cross) >= radius + old.radius + spacing
        }),
        "swarm candidate cannot preserve separation at this coordinate",
    )?;
    Ok(cross)
}
