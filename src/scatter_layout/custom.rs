use glory::render::{GraphicPoint, GraphicStroke, PathCommand};

use super::{ScatterDatum, ScatterSizeUnit};
use crate::scene::ensure;
use crate::{
    ClipRect, Color, FillPaint, Geometry, InteractionAnchor, Mark, PatternShape, Result, StrokeDash,
};

/// Source and resolved layout data supplied to a custom scatter node factory.
#[derive(Clone, Copy, Debug)]
pub struct ScatterNodeContext<'a> {
    pub datum: &'a ScatterDatum,
    pub series_id: &'a str,
    pub series_label: &'a str,
    pub mark_id: &'a str,
    pub series_index: usize,
    pub source_index: usize,
    pub global_index: usize,
    pub formatted_x: &'a str,
    pub formatted_y: &'a str,
    /// The exact mapped data coordinate, in full Scene pixels.
    pub position: [f64; 2],
    pub plot: ClipRect,
    pub size: f64,
    pub size_unit: ScatterSizeUnit,
    pub color: Color,
    pub fill: Option<Color>,
    pub fill_paint: Option<&'a FillPaint>,
    pub stroke: Option<GraphicStroke>,
    pub opacity: f64,
}

/// A single caller-authored mark in logical Scene coordinates. Paths may have
/// several subpaths. Rectangles and circles must be centered at the context's
/// mapped position. Rects built by subtracting half-extents may round on center
/// reconstruction; those keep the exact data position. Use a Path for an
/// offset visual around a separate data site.
#[derive(Clone, Debug, PartialEq)]
pub struct ScatterNodeGeometry {
    pub geometry: Geometry,
    pub fill: Option<Color>,
    pub fill_paint: Option<FillPaint>,
    pub stroke: Option<GraphicStroke>,
    pub stroke_dash: Option<StrokeDash>,
    pub opacity: f64,
    /// Optional unique Move/Line/Cubic endpoint indices in the authored Path.
    /// The default uses endpoints of drawable subpaths, or the first Move for
    /// a Move-only Path. Accepted frames use the exact mapped position. During
    /// compatible animation the anchor follows the mean displacement of these
    /// endpoints. Translation is exact; for a resize or rotation, choose
    /// supports centered on the intended moving origin to keep it centered.
    pub anchor_commands: Option<Vec<usize>>,
}
impl ScatterNodeGeometry {
    /// Retain the inherited node paint while replacing its geometry.
    pub fn inherit(geometry: Geometry, context: &ScatterNodeContext<'_>) -> Self {
        Self {
            geometry,
            fill: context.fill,
            fill_paint: context.fill_paint.cloned(),
            stroke: context.stroke,
            stroke_dash: None,
            opacity: context.opacity,
            anchor_commands: None,
        }
    }

    pub(crate) fn prepare_at(
        self,
        mark_id: &str,
        position: [f64; 2],
        fallback_color: Color,
        remaining_commands: usize,
    ) -> Result<PreparedNode> {
        if let Geometry::Path(path) = &self.geometry {
            ensure(
                path.len() <= remaining_commands,
                "custom point geometry budget exceeded",
            )?;
        } else {
            ensure(
                self.anchor_commands.is_none(),
                "custom point anchor commands require Path geometry",
            )?;
        }
        let visible_solid = self.fill.filter(|color| color.3 > 0);
        let fill_color = self
            .fill_paint
            .as_ref()
            .map_or(visible_solid, representative_fill);
        let color = fill_color
            .or_else(|| {
                self.stroke
                    .map(|stroke| stroke.color)
                    .filter(|color| color.3 > 0)
            })
            .unwrap_or(fallback_color);
        let mut mark = Mark::new(mark_id, self.geometry, color);
        mark.fill = visible_solid;
        mark.fill_paint = self.fill_paint;
        mark.stroke = self.stroke;
        mark.stroke_dash = self.stroke_dash;
        mark.opacity = self.opacity;
        let (bounds, mut anchor) = match &mark.geometry {
            Geometry::Circle { x, y, radius } => (
                ClipRect {
                    x: x - radius,
                    y: y - radius,
                    width: 2. * radius,
                    height: 2. * radius,
                },
                None,
            ),
            Geometry::Rect {
                x,
                y,
                width,
                height,
            } => (
                ClipRect {
                    x: *x,
                    y: *y,
                    width: *width,
                    height: *height,
                },
                None,
            ),
            Geometry::Path(path) => {
                let (bounds, default_support) = path_support(path, position);
                let support = self.anchor_commands.as_deref().unwrap_or(&default_support);
                let anchor =
                    InteractionAnchor::path_offset_following(&mark.geometry, position, support)?;
                (bounds, Some(anchor))
            }
        };
        bounds.validate()?;
        if anchor.is_none() && crate::crosshair::mark_center(&mark) != Some(position) {
            ensure(
                matches!(mark.geometry, Geometry::Rect { .. }),
                "custom Circle must be centered at its data position; use a Path for offset geometry",
            )?;
            anchor = Some(InteractionAnchor::rounded_rect_center(
                &mark.geometry,
                position,
            )?);
        }
        Ok(PreparedNode {
            mark,
            bounds,
            anchor,
            color,
        })
    }
}

fn representative_fill(paint: &FillPaint) -> Option<Color> {
    if let FillPaint::Pattern(pattern) = paint {
        let size = match pattern.shape {
            PatternShape::Dots { diameter, .. } => diameter,
            PatternShape::Squares { size, .. } => size,
            PatternShape::Lines { width } => width,
        };
        if size > 0. && pattern.foreground.3 > 0 {
            Some(pattern.foreground)
        } else {
            Some(pattern.background).filter(|color| color.3 > 0)
        }
    } else {
        paint
            .stops()
            .iter()
            .map(|stop| stop.color)
            .find(|color| color.3 > 0)
    }
}

pub(crate) struct PreparedNode {
    pub mark: Mark,
    pub bounds: ClipRect,
    pub anchor: Option<InteractionAnchor>,
    pub color: Color,
}

/// Conservative control hull bounds of drawable subpaths. An isolated Move
/// establishes no painted extent. A Move-only Path may still be a logical site.
fn path_support(path: &[PathCommand], position: [f64; 2]) -> (ClipRect, Vec<usize>) {
    let mut support = Vec::new();
    let mut first_move = None;
    let mut pending_move = None;
    let mut lo = [f64::INFINITY; 2];
    let mut hi = [f64::NEG_INFINITY; 2];
    let mut include = |p: GraphicPoint| {
        lo[0] = lo[0].min(p.x);
        lo[1] = lo[1].min(p.y);
        hi[0] = hi[0].max(p.x);
        hi[1] = hi[1].max(p.y);
    };
    for (index, command) in path.iter().enumerate() {
        if let PathCommand::Move(point) = command {
            first_move.get_or_insert(index);
            pending_move = Some((index, *point));
            continue;
        }
        if let Some((start, point)) = pending_move.take() {
            support.push(start);
            include(point);
        }
        match command {
            PathCommand::Line(point) => {
                include(*point);
                support.push(index);
            }
            PathCommand::Cubic { first, second, end } => {
                include(*first);
                include(*second);
                include(*end);
                support.push(index);
            }
            PathCommand::Close => {}
            PathCommand::Move(_) => unreachable!("moves handled above"),
        }
    }
    if support.is_empty() {
        support.extend(first_move);
        lo = position;
        hi = position;
    }
    (
        ClipRect {
            x: lo[0],
            y: lo[1],
            width: hi[0] - lo[0],
            height: hi[1] - lo[1],
        },
        support,
    )
}
