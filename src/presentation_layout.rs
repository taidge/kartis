//! Bounded presentation placement with application-supplied text measurements.
//! No font lookup, DOM reads or approximate character-width heuristics occur.
use crate::presentation::{LegendDirection, PresentationAlign};
use crate::{ChartError, MAX_LABEL_BYTES, Result, scene::ensure};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TextSize {
    pub width: f64,
    pub height: f64,
}
impl TextSize {
    pub fn validate(self) -> Result<()> {
        ensure(
            [self.width, self.height]
                .into_iter()
                .all(|v| v.is_finite() && (0. ..=1_000_000.).contains(&v)),
            "invalid measured text size",
        )
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LayoutRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
impl LayoutRect {
    pub fn validate(self) -> Result<()> {
        TextSize {
            width: self.width,
            height: self.height,
        }
        .validate()?;
        ensure(
            [self.x, self.y, self.x + self.width, self.y + self.height]
                .into_iter()
                .all(|v| v.is_finite() && v.abs() <= 1_000_000.),
            "invalid presentation rectangle",
        )
    }
    pub fn overlap(self, other: Self) -> f64 {
        (self.x + self.width)
            .min(other.x + other.width)
            .sub(self.x.max(other.x))
            .max(0.)
            * (self.y + self.height)
                .min(other.y + other.height)
                .sub(self.y.max(other.y))
                .max(0.)
    }
    pub fn contains(self, other: Self) -> bool {
        other.x >= self.x
            && other.y >= self.y
            && other.x + other.width <= self.x + self.width
            && other.y + other.height <= self.y + self.height
    }
    fn inset(self, padding: f64) -> Result<Self> {
        let r = Self {
            x: self.x + padding,
            y: self.y + padding,
            width: self.width - padding * 2.,
            height: self.height - padding * 2.,
        };
        r.validate()?;
        Ok(r)
    }
}
// Keep arithmetic readable without importing an operator into consumers.
use std::ops::Sub;
fn spacing(value: f64) -> Result<()> {
    ensure(
        value.is_finite() && (0. ..=4096.).contains(&value),
        "invalid presentation layout spacing",
    )
}
fn identity(id: &str) -> Result<()> {
    ensure(
        !id.is_empty() && id.len() <= MAX_LABEL_BYTES,
        "invalid presentation layout identity",
    )
}
#[derive(Clone, Debug, PartialEq)]
pub struct LegendMeasuredItem {
    pub id: String,
    pub size: TextSize,
}
#[derive(Clone, Debug, PartialEq)]
pub struct PlacedLegendItem {
    pub id: String,
    pub rect: LayoutRect,
}
#[derive(Clone, Debug, PartialEq)]
pub struct LegendLayout {
    pub items: Vec<PlacedLegendItem>,
    pub bounds: LayoutRect,
}
#[derive(Clone, Debug, PartialEq)]
pub struct LegendLayoutOptions {
    pub bounds: LayoutRect,
    pub direction: LegendDirection,
    pub wrap: bool,
    /// Horizontal flow caps items per row; vertical flow caps columns.
    pub max_columns: usize,
    pub row_gap: f64,
    pub column_gap: f64,
    pub align: PresentationAlign,
}
impl Default for LegendLayoutOptions {
    fn default() -> Self {
        Self {
            bounds: LayoutRect {
                x: 0.,
                y: 0.,
                width: 640.,
                height: 200.,
            },
            direction: LegendDirection::Horizontal,
            wrap: true,
            max_columns: 256,
            row_gap: 8.,
            column_gap: 16.,
            align: PresentationAlign::Start,
        }
    }
}
impl LegendLayoutOptions {
    pub fn validate(&self) -> Result<()> {
        self.bounds.validate()?;
        spacing(self.row_gap)?;
        spacing(self.column_gap)?;
        ensure(
            (1..=256).contains(&self.max_columns),
            "invalid legend column limit",
        )
    }
}
pub fn layout_legend(
    items: &[LegendMeasuredItem],
    options: &LegendLayoutOptions,
) -> Result<LegendLayout> {
    options.validate()?;
    ensure(items.len() <= 256, "legend layout item budget exceeded")?;
    let mut ids = BTreeSet::new();
    for item in items {
        identity(&item.id)?;
        ensure(ids.insert(&item.id), "duplicate legend layout identity")?;
        item.size.validate()?;
        ensure(
            item.size.width <= options.bounds.width && item.size.height <= options.bounds.height,
            "legend item exceeds layout bounds",
        )?;
    }
    let horizontal = options.direction == LegendDirection::Horizontal;
    let (main_limit, cross_limit, main_gap, cross_gap) = if horizontal {
        (
            options.bounds.width,
            options.bounds.height,
            options.column_gap,
            options.row_gap,
        )
    } else {
        (
            options.bounds.height,
            options.bounds.width,
            options.row_gap,
            options.column_gap,
        )
    };
    let mut groups: Vec<Vec<&LegendMeasuredItem>> = Vec::new();
    let mut group = Vec::new();
    let mut extent = 0.;
    for item in items {
        let main = if horizontal {
            item.size.width
        } else {
            item.size.height
        };
        let next = if group.is_empty() {
            main
        } else {
            extent + main_gap + main
        };
        if !group.is_empty()
            && options.wrap
            && (next > main_limit || horizontal && group.len() >= options.max_columns)
        {
            groups.push(std::mem::take(&mut group));
            extent = 0.;
        }
        extent = if group.is_empty() {
            main
        } else {
            extent + main_gap + main
        };
        ensure(
            extent <= main_limit,
            "unwrapped legend exceeds layout bounds",
        )?;
        group.push(item);
    }
    if !group.is_empty() {
        groups.push(group);
    }
    ensure(
        horizontal || groups.len() <= options.max_columns,
        "legend exceeds maximum columns",
    )?;
    let mut placed = Vec::new();
    let mut cross = 0.;
    for group in groups {
        let main_extent = group
            .iter()
            .map(|i| {
                if horizontal {
                    i.size.width
                } else {
                    i.size.height
                }
            })
            .sum::<f64>()
            + main_gap * (group.len().saturating_sub(1)) as f64;
        let cross_extent = group
            .iter()
            .map(|i| {
                if horizontal {
                    i.size.height
                } else {
                    i.size.width
                }
            })
            .fold(0., f64::max);
        ensure(
            cross + cross_extent <= cross_limit,
            "legend rows/columns exceed layout bounds",
        )?;
        let mut main = match options.align {
            PresentationAlign::Start => 0.,
            PresentationAlign::Center => (main_limit - main_extent) / 2.,
            PresentationAlign::End => main_limit - main_extent,
        };
        for item in group {
            let rect = LayoutRect {
                x: options.bounds.x + if horizontal { main } else { cross },
                y: options.bounds.y + if horizontal { cross } else { main },
                width: item.size.width,
                height: item.size.height,
            };
            rect.validate()?;
            placed.push(PlacedLegendItem {
                id: item.id.clone(),
                rect,
            });
            main += if horizontal {
                item.size.width
            } else {
                item.size.height
            };
            main += main_gap;
        }
        cross += cross_extent + cross_gap;
    }
    Ok(LegendLayout {
        items: placed,
        bounds: options.bounds,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TooltipAnchor {
    Top,
    Bottom,
    Left,
    Right,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}
#[derive(Clone, Debug, PartialEq)]
pub struct TooltipLayoutOptions {
    pub viewport: LayoutRect,
    pub preferred: Vec<TooltipAnchor>,
    pub gap: f64,
    pub viewport_padding: f64,
    pub forbidden: Vec<LayoutRect>,
}
impl Default for TooltipLayoutOptions {
    fn default() -> Self {
        Self {
            viewport: LayoutRect {
                x: 0.,
                y: 0.,
                width: 640.,
                height: 400.,
            },
            preferred: vec![
                TooltipAnchor::BottomRight,
                TooltipAnchor::TopRight,
                TooltipAnchor::BottomLeft,
                TooltipAnchor::TopLeft,
            ],
            gap: 12.,
            viewport_padding: 4.,
            forbidden: Vec::new(),
        }
    }
}
impl TooltipLayoutOptions {
    pub fn validate(&self) -> Result<()> {
        self.viewport.validate()?;
        spacing(self.gap)?;
        spacing(self.viewport_padding)?;
        self.viewport.inset(self.viewport_padding)?;
        ensure(
            !self.preferred.is_empty() && self.preferred.len() <= 8 && self.forbidden.len() <= 256,
            "tooltip candidate/obstacle budget exceeded",
        )?;
        for r in &self.forbidden {
            r.validate()?;
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TooltipPlacement {
    pub rect: LayoutRect,
    pub anchor: TooltipAnchor,
    pub overlap_area: f64,
    pub clamped: bool,
}
/// Prefers the first collision-free unclamped anchor, then obstacle-edge
/// alternatives. If no clear location exists, reports the least overlap rather
/// than pretending an impossible avoidance succeeded. Oversized content is
/// bounded to the viewport; hosts should scroll/clip its inner content.
pub fn place_tooltip(
    anchor: (f64, f64),
    size: TextSize,
    options: &TooltipLayoutOptions,
) -> Result<TooltipPlacement> {
    options.validate()?;
    size.validate()?;
    ensure(
        [anchor.0, anchor.1]
            .into_iter()
            .all(|v| v.is_finite() && v.abs() <= 1_000_000.),
        "invalid tooltip anchor point",
    )?;
    let view = options.viewport.inset(options.viewport_padding)?;
    let w = size.width.min(view.width);
    let h = size.height.min(view.height);
    let mut candidates = Vec::new();
    for kind in &options.preferred {
        let (x, y) = match kind {
            TooltipAnchor::Top => (anchor.0 - w / 2., anchor.1 - options.gap - h),
            TooltipAnchor::Bottom => (anchor.0 - w / 2., anchor.1 + options.gap),
            TooltipAnchor::Left => (anchor.0 - options.gap - w, anchor.1 - h / 2.),
            TooltipAnchor::Right => (anchor.0 + options.gap, anchor.1 - h / 2.),
            TooltipAnchor::TopLeft => (anchor.0 - options.gap - w, anchor.1 - options.gap - h),
            TooltipAnchor::TopRight => (anchor.0 + options.gap, anchor.1 - options.gap - h),
            TooltipAnchor::BottomLeft => (anchor.0 - options.gap - w, anchor.1 + options.gap),
            TooltipAnchor::BottomRight => (anchor.0 + options.gap, anchor.1 + options.gap),
        };
        candidates.push((x, y, *kind));
    }
    for r in &options.forbidden {
        for (x, y) in [
            (r.x - w - options.gap, r.y),
            (r.x + r.width + options.gap, r.y),
            (r.x, r.y - h - options.gap),
            (r.x, r.y + r.height + options.gap),
        ] {
            candidates.push((x, y, options.preferred[0]));
        }
    }
    let mut best: Option<(TooltipPlacement, f64)> = None;
    for (x, y, kind) in candidates {
        let rect = LayoutRect {
            x: x.clamp(view.x, view.x + view.width - w),
            y: y.clamp(view.y, view.y + view.height - h),
            width: w,
            height: h,
        };
        rect.validate()?;
        let overlap = options
            .forbidden
            .iter()
            .map(|r| rect.overlap(*r))
            .sum::<f64>();
        let displacement = (rect.x - x).abs() + (rect.y - y).abs();
        let placement = TooltipPlacement {
            rect,
            anchor: kind,
            overlap_area: overlap,
            clamped: displacement > 0. || w != size.width || h != size.height,
        };
        if best.is_none_or(|(old, d)| {
            overlap < old.overlap_area || overlap == old.overlap_area && displacement < d
        }) {
            best = Some((placement, displacement));
        }
        if overlap == 0. && displacement == 0. {
            return Ok(placement);
        }
    }
    Ok(best.expect("nonempty validated candidates").0)
}

#[derive(Clone, Debug, PartialEq)]
pub struct LabelCandidate {
    pub id: String,
    pub rect: LayoutRect,
    pub priority: i32,
    /// Ordered alternative displacements; (0,0) is tried first automatically.
    pub offsets: Vec<(f64, f64)>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct LabelCollisionOptions {
    pub viewport: LayoutRect,
    pub forbidden: Vec<LayoutRect>,
    pub padding: f64,
    pub max_checks: usize,
}
impl Default for LabelCollisionOptions {
    fn default() -> Self {
        Self {
            viewport: LayoutRect {
                x: 0.,
                y: 0.,
                width: 640.,
                height: 400.,
            },
            forbidden: Vec::new(),
            padding: 2.,
            max_checks: 1_000_000,
        }
    }
}
impl LabelCollisionOptions {
    pub fn validate(&self) -> Result<()> {
        self.viewport.validate()?;
        spacing(self.padding)?;
        ensure(
            self.forbidden.len() <= 256 && (1..=10_000_000).contains(&self.max_checks),
            "label collision resource budget invalid",
        )?;
        for r in &self.forbidden {
            r.validate()?;
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct LabelPlacement {
    pub id: String,
    pub rect: Option<LayoutRect>,
}
pub fn layout_labels(
    labels: &[LabelCandidate],
    options: &LabelCollisionOptions,
) -> Result<Vec<LabelPlacement>> {
    options.validate()?;
    ensure(
        labels.len() <= crate::MAX_MARKS,
        "label layout input budget exceeded",
    )?;
    let mut ids = BTreeSet::new();
    for label in labels {
        identity(&label.id)?;
        ensure(ids.insert(&label.id), "duplicate collision label identity")?;
        label.rect.validate()?;
        ensure(
            label.offsets.len() <= 16
                && label.offsets.iter().all(|(x, y)| {
                    x.is_finite() && y.is_finite() && x.abs() <= 1_000_000. && y.abs() <= 1_000_000.
                }),
            "invalid label collision offsets",
        )?;
    }
    let mut order = (0..labels.len()).collect::<Vec<_>>();
    order.sort_by_key(|i| std::cmp::Reverse(labels[*i].priority));
    let mut result = labels
        .iter()
        .map(|l| LabelPlacement {
            id: l.id.clone(),
            rect: None,
        })
        .collect::<Vec<_>>();
    let mut occupied = options.forbidden.clone();
    let mut checks = 0usize;
    for index in order {
        let label = &labels[index];
        for (dx, dy) in std::iter::once((0., 0.)).chain(label.offsets.iter().copied()) {
            let rect = LayoutRect {
                x: label.rect.x + dx,
                y: label.rect.y + dy,
                ..label.rect
            };
            rect.validate()?;
            if !options.viewport.contains(rect) {
                continue;
            }
            let padded = LayoutRect {
                x: rect.x - options.padding,
                y: rect.y - options.padding,
                width: rect.width + options.padding * 2.,
                height: rect.height + options.padding * 2.,
            };
            padded.validate()?;
            let mut collision = false;
            for old in &occupied {
                checks += 1;
                if checks > options.max_checks {
                    return Err(ChartError("label collision work budget exceeded".into()));
                }
                if padded.overlap(*old) > 0. {
                    collision = true;
                    break;
                }
            }
            if !collision {
                occupied.push(rect);
                result[index].rect = Some(rect);
                break;
            }
        }
    }
    Ok(result)
}
