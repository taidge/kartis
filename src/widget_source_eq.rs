//! Exact original-source comparisons for typed live Widget payloads.
//! Business-key identity and ordinary layout/paint numeric equality are separate.
use crate::{
    BarSeries, BumpSeries, CategoryKey, FillPaint, GradientStop, PatternShape, ScatterSeries,
};

pub(crate) fn number(a: f64, b: f64) -> bool {
    a == b && a.to_bits() == b.to_bits()
}
pub(crate) fn optional(a: Option<f64>, b: Option<f64>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => number(a, b),
        _ => false,
    }
}
pub(crate) fn category(a: &CategoryKey, b: &CategoryKey) -> bool {
    match (a, b) {
        (CategoryKey::Number(a), CategoryKey::Number(b)) => number(*a, *b),
        (CategoryKey::String(a), CategoryKey::String(b)) => a == b,
        _ => false,
    }
}

/// Original authored paint, including every nested floating-point bit pattern.
/// Ordinary paint equality still determines variants, colors and other fields;
/// source identity additionally distinguishes otherwise equal signed zeros.
pub(crate) fn paint(a: Option<&FillPaint>, b: Option<&FillPaint>) -> bool {
    if a != b {
        return false;
    }
    let point = |a: [f64; 2], b: [f64; 2]| a.into_iter().zip(b).all(|(a, b)| number(a, b));
    let stops = |a: &[GradientStop], b: &[GradientStop]| {
        a.len() == b.len() && a.iter().zip(b).all(|(a, b)| number(a.offset, b.offset))
    };
    match (a, b) {
        (None, None) => true,
        (
            Some(FillPaint::Linear {
                start,
                end,
                stops: left,
            }),
            Some(FillPaint::Linear {
                start: other_start,
                end: other_end,
                stops: right,
            }),
        ) => point(*start, *other_start) && point(*end, *other_end) && stops(left, right),
        (
            Some(FillPaint::Radial {
                center,
                radius,
                stops: left,
            }),
            Some(FillPaint::Radial {
                center: other_center,
                radius: other_radius,
                stops: right,
            }),
        ) => point(*center, *other_center) && number(*radius, *other_radius) && stops(left, right),
        (Some(FillPaint::Pattern(left)), Some(FillPaint::Pattern(right))) => {
            let shape = match (left.shape, right.shape) {
                (
                    PatternShape::Dots { diameter: a, .. },
                    PatternShape::Dots { diameter: b, .. },
                ) => number(a, b),
                (PatternShape::Squares { size: a, .. }, PatternShape::Squares { size: b, .. }) => {
                    number(a, b)
                }
                (PatternShape::Lines { width: a }, PatternShape::Lines { width: b }) => {
                    number(a, b)
                }
                _ => false,
            };
            shape
                && number(left.spacing, right.spacing)
                && number(left.rotation, right.rotation)
                && point(left.offset, right.offset)
        }
        _ => false,
    }
}
pub(crate) fn bar_series(a: &BarSeries, b: &BarSeries) -> bool {
    let BarSeries { id, values } = a;
    let BarSeries {
        id: other_id,
        values: other_values,
    } = b;
    id == other_id
        && values.len() == other_values.len()
        && values
            .iter()
            .zip(other_values)
            .all(|(a, b)| optional(*a, *b))
}
pub(crate) fn bump_series(a: &BumpSeries, b: &BumpSeries) -> bool {
    let BumpSeries { id, label, data } = a;
    let BumpSeries {
        id: other_id,
        label: other_label,
        data: other_data,
    } = b;
    id == other_id
        && label == other_label
        && data.len() == other_data.len()
        && data
            .iter()
            .zip(other_data)
            .all(|(a, b)| category(&a.x, &b.x) && optional(a.rank, b.rank))
}
pub(crate) fn scatter_series(a: &ScatterSeries, b: &ScatterSeries) -> bool {
    let ScatterSeries {
        id,
        label,
        data,
        color,
    } = a;
    let ScatterSeries {
        id: other_id,
        label: other_label,
        data: other_data,
        color: other_color,
    } = b;
    id == other_id
        && label == other_label
        && color == other_color
        && data.len() == other_data.len()
        && data.iter().zip(other_data).all(|(a, b)| {
            a.id == b.id
                && category(&a.x, &b.x)
                && category(&a.y, &b.y)
                && optional(a.size_value, b.size_value)
                && a.color == b.color
                && a.label == b.label
        })
}
