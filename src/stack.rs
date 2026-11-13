//! Reusable aligned stack computation, independent of rendering.
//!
//! Nivo's stream/src/hooks.ts uses d3-shape order/offset functions. The wiggle
//! recurrence and inside-out peak balancing here follow the documented D3
//! algorithms (d3-shape/src/offset/wiggle.js, src/order/insideOut.js), using
//! normalized weights to avoid overflowing intermediate value products.
use crate::scene::ensure;
use crate::{BarSeries, Result};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StackOrder {
    #[default]
    None,
    Reverse,
    Ascending,
    Descending,
    InsideOut,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StackOffset {
    #[default]
    Zero,
    Diverging,
    Expand,
    Silhouette,
    Wiggle,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StackOptions {
    pub order: StackOrder,
    pub offset: StackOffset,
}
#[derive(Clone, Debug, PartialEq)]
pub struct StackPoint {
    pub column: usize,
    pub value: Option<f64>,
    pub y0: f64,
    pub y1: f64,
}
#[derive(Clone, Debug, PartialEq)]
pub struct StackLayer {
    pub id: String,
    pub original_index: usize,
    pub stack_index: usize,
    pub points: Vec<StackPoint>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct StackResult {
    /// Layers remain in input order; use `order` for bottom-to-top painting.
    pub layers: Vec<StackLayer>,
    pub order: Vec<usize>,
    /// Bottom baseline per column; zero for diverging stacks.
    pub baseline: Vec<f64>,
    /// Exact extrema of all pairs. Empty/zero data has domain `[0, 0]`.
    pub domain: [f64; 2],
}
fn finite(value: f64) -> Result<f64> {
    ensure(value.is_finite(), "stack arithmetic overflow")?;
    Ok(value)
}
/// Align nullable series by column. Missing values retain `None` metadata but
/// contribute zero thickness. All rows must have the same length. Expand,
/// Silhouette and Wiggle require nonnegative values; signed inputs choose Zero
/// (ordinary cumulative) or Diverging (independent positive/negative baselines).
pub fn stack(series: &[BarSeries], options: &StackOptions) -> Result<StackResult> {
    ensure(series.len() <= 256, "stack exceeds 256 series budget")?;
    let columns = series.first().map_or(0, |s| s.values.len());
    ensure(
        columns <= 20_000
            && series
                .len()
                .checked_mul(columns)
                .is_some_and(|n| n <= 100_000),
        "stack exceeds 100000 cell budget",
    )?;
    let mut ids = BTreeSet::new();
    let mut maximum = 0_f64;
    for s in series {
        ensure(
            !s.id.trim().is_empty() && s.id.len() <= 256 && ids.insert(&s.id),
            "invalid or duplicate stack series id",
        )?;
        ensure(s.values.len() == columns, "stack series lengths must align")?;
        for v in s.values.iter().flatten() {
            ensure(v.is_finite(), "stack values must be finite")?;
            if !matches!(options.offset, StackOffset::Zero | StackOffset::Diverging) {
                ensure(
                    *v >= 0.,
                    "selected stack offset requires nonnegative values",
                )?;
            }
            maximum = maximum.max(v.abs());
        }
    }
    let normalizer = if maximum > 0. { maximum } else { 1. };
    let sums = series
        .iter()
        .map(|s| {
            s.values
                .iter()
                .flatten()
                .map(|v| v / normalizer)
                .sum::<f64>()
        })
        .collect::<Vec<_>>();
    let mut order = (0..series.len()).collect::<Vec<_>>();
    match options.order {
        StackOrder::None => {}
        StackOrder::Reverse => order.reverse(),
        StackOrder::Ascending => {
            order.sort_by(|a, b| sums[*a].total_cmp(&sums[*b]).then_with(|| a.cmp(b)))
        }
        StackOrder::Descending => {
            order.sort_by(|a, b| sums[*b].total_cmp(&sums[*a]).then_with(|| a.cmp(b)))
        }
        StackOrder::InsideOut => {
            let peaks = series
                .iter()
                .map(|s| {
                    s.values
                        .iter()
                        .enumerate()
                        .max_by(|(ai, a), (bi, b)| {
                            a.unwrap_or(0.)
                                .total_cmp(&b.unwrap_or(0.))
                                .then_with(|| bi.cmp(ai))
                        })
                        .map_or(0, |(i, _)| i)
                })
                .collect::<Vec<_>>();
            order.sort_by(|a, b| peaks[*a].cmp(&peaks[*b]).then_with(|| a.cmp(b)));
            let mut top = 0.;
            let mut bottom = 0.;
            let mut tops = vec![];
            let mut bottoms = vec![];
            for i in order {
                if top < bottom {
                    top += sums[i];
                    tops.push(i);
                } else {
                    bottom += sums[i];
                    bottoms.push(i);
                }
            }
            bottoms.reverse();
            bottoms.extend(tops);
            order = bottoms;
        }
    }
    let mut totals = vec![0.; columns];
    if !matches!(options.offset, StackOffset::Zero | StackOffset::Diverging) {
        for s in series {
            for (column, v) in s.values.iter().enumerate() {
                totals[column] = finite(totals[column] + v.unwrap_or(0.))?;
            }
        }
    }
    let mut baseline = vec![0.; columns];
    match options.offset {
        StackOffset::Silhouette => {
            for (base, total) in baseline.iter_mut().zip(&totals) {
                *base = -total / 2.;
            }
        }
        StackOffset::Wiggle => {
            let mut y = 0.;
            for column in 1..columns {
                let mut prefix_delta = 0.;
                let mut slope = 0.;
                for &si in &order {
                    let current = series[si].values[column].unwrap_or(0.);
                    let previous = series[si].values[column - 1].unwrap_or(0.);
                    let delta = current - previous;
                    if totals[column] > 0. {
                        slope = finite(
                            slope
                                + finite((delta / 2. + prefix_delta) * (current / totals[column]))?,
                        )?;
                    }
                    prefix_delta = finite(prefix_delta + delta)?;
                }
                if totals[column] > 0. {
                    y = finite(y - slope)?;
                }
                baseline[column] = y;
            }
        }
        _ => {}
    }
    let mut layers = series
        .iter()
        .enumerate()
        .map(|(i, s)| StackLayer {
            id: s.id.clone(),
            original_index: i,
            stack_index: 0,
            points: Vec::with_capacity(columns),
        })
        .collect::<Vec<_>>();
    let mut positive = vec![0.; columns];
    let mut negative = positive.clone();
    let mut cumulative = baseline.clone();
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for (stack_index, &si) in order.iter().enumerate() {
        layers[si].stack_index = stack_index;
        for column in 0..columns {
            let value = series[si].values[column];
            let raw = value.unwrap_or(0.);
            let v = if options.offset == StackOffset::Expand && totals[column] != 0. {
                raw / totals[column]
            } else {
                raw
            };
            let (y0, y1) = if options.offset == StackOffset::Diverging {
                if v > 0. {
                    let base = positive[column];
                    positive[column] = finite(base + v)?;
                    (base, positive[column])
                } else if v < 0. {
                    let base = negative[column];
                    negative[column] = finite(base + v)?;
                    (negative[column], base)
                } else {
                    (0., 0.)
                }
            } else {
                let base = cumulative[column];
                cumulative[column] = finite(base + v)?;
                (base, cumulative[column])
            };
            lo = lo.min(y0.min(y1));
            hi = hi.max(y0.max(y1));
            layers[si].points.push(StackPoint {
                column,
                value,
                y0,
                y1,
            });
        }
    }
    let domain = if lo == f64::INFINITY {
        [0., 0.]
    } else {
        ensure((hi - lo).is_finite(), "stack domain span overflow")?;
        [lo, hi]
    };
    Ok(StackResult {
        layers,
        order,
        baseline,
        domain,
    })
}
