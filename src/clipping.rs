//! Robust line clipping against logical/data-space rectangles.
use crate::scene::ensure;
use crate::{ClipRect, Result};
/// Cohen–Sutherland clipping with an implicit line equation. Normalized line
/// coefficients avoid overflowing endpoint differences for opposite f64 maxima.
/// Each axis is normalized independently so tiny y values survive huge x values
/// (and vice versa); a common normalization would erase subnormal coordinates.
pub fn clip_segment(
    a: (f64, f64),
    b: (f64, f64),
    rect: ClipRect,
) -> Result<Option<[(f64, f64); 2]>> {
    ensure(
        [
            a.0,
            a.1,
            b.0,
            b.1,
            rect.x,
            rect.y,
            rect.width,
            rect.height,
            rect.x + rect.width,
            rect.y + rect.height,
        ]
        .into_iter()
        .all(f64::is_finite)
            && rect.width >= 0.
            && rect.height >= 0.,
        "invalid segment or clipping rectangle",
    )?;
    let code = |p: (f64, f64)| {
        u8::from(p.0 < rect.x)
            | (u8::from(p.0 > rect.x + rect.width) * 2)
            | (u8::from(p.1 < rect.y) * 4)
            | (u8::from(p.1 > rect.y + rect.height) * 8)
    };
    let scale_x = a.0.abs().max(b.0.abs());
    let scale_y = a.1.abs().max(b.1.abs());
    let scale_x = if scale_x == 0. { 1. } else { scale_x };
    let scale_y = if scale_y == 0. { 1. } else { scale_y };
    let aa = b.1 / scale_y - a.1 / scale_y;
    let bb = a.0 / scale_x - b.0 / scale_x;
    let cc = (a.0 / scale_x).mul_add(b.1 / scale_y, -(b.0 / scale_x) * (a.1 / scale_y));
    let (mut p, mut q) = (a, b);
    for _ in 0..12 {
        let cp = code(p);
        let cq = code(q);
        if cp | cq == 0 {
            return Ok(Some([p, q]));
        }
        if cp & cq != 0 {
            return Ok(None);
        }
        let outside = if cp != 0 { cp } else { cq };
        let next = if outside & 8 != 0 {
            let y = rect.y + rect.height;
            if aa == 0. {
                return Ok(None);
            }
            (((cc - bb * (y / scale_y)) / aa) * scale_x, y)
        } else if outside & 4 != 0 {
            let y = rect.y;
            if aa == 0. {
                return Ok(None);
            }
            (((cc - bb * (y / scale_y)) / aa) * scale_x, y)
        } else if outside & 2 != 0 {
            let x = rect.x + rect.width;
            if bb == 0. {
                return Ok(None);
            }
            (x, ((cc - aa * (x / scale_x)) / bb) * scale_y)
        } else {
            let x = rect.x;
            if bb == 0. {
                return Ok(None);
            }
            (x, ((cc - aa * (x / scale_x)) / bb) * scale_y)
        };
        ensure(
            next.0.is_finite() && next.1.is_finite(),
            "segment clipping arithmetic overflow",
        )?;
        // Axis normalization can leave a corner one rounding step from its
        // other boundary. Snap only relative to this rectangle's axis scale;
        // an absolute epsilon would erase legitimate subnormal plot domains.
        let snap = |value: f64, low: f64, high: f64| {
            let tolerance = ((high - low).abs().max(low.abs()).max(high.abs()) * f64::EPSILON) * 4.;
            if (value - low).abs() <= tolerance {
                low
            } else if (value - high).abs() <= tolerance {
                high
            } else {
                value
            }
        };
        let next = (
            snap(next.0, rect.x, rect.x + rect.width),
            snap(next.1, rect.y, rect.y + rect.height),
        );
        if cp != 0 {
            p = next;
        } else {
            q = next;
        }
    }
    Err(crate::ChartError(
        "segment clipping did not converge".into(),
    ))
}
