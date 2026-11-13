//! Typed current-datum bindings and renderer-independent Nivo crosshairs.
//! Crosshair geometry follows @nivo/tooltip/src/Crosshair.tsx's eleven types.
use std::collections::{BTreeMap, BTreeSet};

use glory::render::{GraphicPoint, GraphicStroke, PathCommand, StrokeCap};

use crate::scene::ensure;
use crate::*;

pub const CROSSHAIR_X_ID: &str = "kartis-crosshair:x";
pub const CROSSHAIR_Y_ID: &str = "kartis-crosshair:y";
pub const POLAR_SPOKE_ID: &str = "kartis-crosshair:polar";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CrosshairType {
    X,
    Y,
    Cross,
    TopLeft,
    Top,
    TopRight,
    Right,
    BottomRight,
    Bottom,
    #[default]
    BottomLeft,
    Left,
}
#[derive(Clone, Debug, PartialEq)]
pub struct CrosshairOptions {
    pub kind: CrosshairType,
    pub stroke: GraphicStroke,
    pub dash: Option<StrokeDash>,
    pub opacity: f64,
}
impl Default for CrosshairOptions {
    fn default() -> Self {
        Self {
            kind: CrosshairType::BottomLeft,
            stroke: GraphicStroke {
                color: Color(65, 75, 90, 255),
                width: 1.,
                cap: StrokeCap::Butt,
            },
            dash: Some(StrokeDash {
                segments: vec![6., 6.],
                offset: 0.,
            }),
            opacity: 0.75,
        }
    }
}
impl CrosshairOptions {
    pub fn validate(&self) -> Result<()> {
        ensure(
            self.stroke.width.is_finite()
                && (0.0001..=256.).contains(&self.stroke.width)
                && self.opacity.is_finite()
                && (0. ..=1.).contains(&self.opacity),
            "invalid crosshair stroke or opacity",
        )?;
        if let Some(dash) = &self.dash {
            dash.validate()?;
        }
        Ok(())
    }
}
/// World coordinates. X/Y slice callers override `kind` with a full X/Y line.
/// Marks have no data label or series and never participate in datum picking.
pub fn crosshair_marks(
    plot: ClipRect,
    anchor: [f64; 2],
    options: &CrosshairOptions,
    axis: Option<mesh::SliceAxis>,
) -> Result<Vec<Mark>> {
    options.validate()?;
    plot.validate()?;
    ensure(
        anchor.iter().all(|v| v.is_finite() && v.abs() <= 1e6),
        "invalid crosshair anchor",
    )?;
    let kind = match axis {
        Some(mesh::SliceAxis::X) => CrosshairType::X,
        Some(mesh::SliceAxis::Y) => CrosshairType::Y,
        None => options.kind,
    };
    let visible = match kind {
        CrosshairType::X => anchor[0] >= plot.x && anchor[0] <= plot.x + plot.width,
        CrosshairType::Y => anchor[1] >= plot.y && anchor[1] <= plot.y + plot.height,
        _ => plot.contains(anchor[0], anchor[1]),
    };
    if !visible {
        return Ok(vec![]);
    }
    let [x, y] = anchor;
    let left = plot.x;
    let top = plot.y;
    let right = left + plot.width;
    let bottom = top + plot.height;
    let mut segments = vec![];
    match kind {
        CrosshairType::X => segments.push((CROSSHAIR_X_ID, [x, top], [x, bottom])),
        CrosshairType::Y => segments.push((CROSSHAIR_Y_ID, [left, y], [right, y])),
        CrosshairType::Cross => {
            segments.push((CROSSHAIR_X_ID, [x, top], [x, bottom]));
            segments.push((CROSSHAIR_Y_ID, [left, y], [right, y]));
        }
        CrosshairType::Top => segments.push((CROSSHAIR_X_ID, [x, top], [x, y])),
        CrosshairType::Bottom => segments.push((CROSSHAIR_X_ID, [x, y], [x, bottom])),
        CrosshairType::Left => segments.push((CROSSHAIR_Y_ID, [left, y], [x, y])),
        CrosshairType::Right => segments.push((CROSSHAIR_Y_ID, [x, y], [right, y])),
        CrosshairType::TopLeft
        | CrosshairType::TopRight
        | CrosshairType::BottomLeft
        | CrosshairType::BottomRight => {
            let upper = matches!(kind, CrosshairType::TopLeft | CrosshairType::TopRight);
            let before = matches!(kind, CrosshairType::TopLeft | CrosshairType::BottomLeft);
            segments.push((
                CROSSHAIR_X_ID,
                [x, if upper { top } else { y }],
                [x, if upper { y } else { bottom }],
            ));
            segments.push((
                CROSSHAIR_Y_ID,
                [if before { left } else { x }, y],
                [if before { x } else { right }, y],
            ));
        }
    }
    Ok(segments
        .into_iter()
        .map(|(id, a, b)| {
            let mut mark = Mark::new(
                id,
                Geometry::Path(vec![
                    PathCommand::Move(GraphicPoint::new(a[0], a[1])),
                    PathCommand::Line(GraphicPoint::new(b[0], b[1])),
                ]),
                options.stroke.color,
            );
            mark.label.clear();
            mark.fill = None;
            mark.stroke = Some(options.stroke);
            mark.stroke_dash = options.dash.clone();
            mark.opacity = options.opacity;
            mark.clip = Some(plot);
            mark
        })
        .collect())
}
/// Append one owned decorative layer as a whole candidate. Caller Scenes stay
/// unchanged on invalid style, namespace collision or budget rejection.
pub fn append_crosshair(
    scene: &mut Scene,
    plot: ClipRect,
    anchor: [f64; 2],
    options: &CrosshairOptions,
    axis: Option<mesh::SliceAxis>,
) -> Result<()> {
    scene.validate()?;
    overlay_admission(scene, true)?;
    let marks = crosshair_marks(plot, anchor, options, axis)?;
    let mut candidate = scene.clone();
    let start = candidate.marks.len();
    if !candidate.draw_order.is_empty() {
        candidate
            .draw_order
            .extend((start..start + marks.len()).map(DrawItem::Mark));
    }
    candidate.marks.extend(marks);
    candidate.validate()?;
    *scene = candidate;
    Ok(())
}

/// Business data is owned and bounded; positions are validated against the
/// candidate Scene. Current animation positions are resolved from live marks.
#[derive(Clone, Debug, PartialEq)]
pub struct InteractionSite {
    pub mark_id: String,
    pub series: String,
    pub label: String,
    pub value: Option<f64>,
    pub formatted_value: Option<String>,
    pub color: Color,
    pub position: [f64; 2],
}
/// Explicit Geometry-picking contract. Other Surface picking modes require the
/// legacy profile; physical ink ownership never selects a profile implicitly.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PickingProfile {
    #[default]
    LegacyGeometry,
    /// Pick actual visible painted geometry, then resolve a logical owner.
    /// Host support is independent of whether the alias map contains entries.
    PaintedGeometry,
}
/// Original source attached to one actual physical part. Values are independent
/// of the live mark scalar used by geometry/property animation.
#[derive(Clone, Debug, PartialEq)]
pub struct InteractionPartDatum {
    pub source_id: String,
    pub value: f64,
}
#[derive(Clone, Default, PartialEq)]
pub struct InteractionBinding {
    pub sites: Vec<InteractionSite>,
    /// Explicit anchors for real Path datums and checked factory-Rect rounding.
    /// Ordinary Circle/Rect sites use their geometric centers.
    pub anchors: BTreeMap<String, InteractionAnchor>,
    pub polar: Option<crate::PolarSliceIndex>,
    pub polar_spoke: Option<GraphicStroke>,
    /// Optional physical ink mark IDs mapped directly to existing logical site
    /// mark IDs. This metadata does not create paint, sites or hit geometry.
    /// Visibility and actual ink must be checked by the host before resolving it.
    pub ink_owners: BTreeMap<String, String>,
    /// Explicit opt-in painted-ink picking, including a body-only binding.
    pub picking_profile: PickingProfile,
    /// Optional accepted original source for an actual site/alias mark. This
    /// metadata does not synthesize a logical datum or change source geometry.
    pub part_data: BTreeMap<String, InteractionPartDatum>,
}
// Preserve the legacy compact/pretty representation when aliases are absent.
impl std::fmt::Debug for InteractionBinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug = f.debug_struct("InteractionBinding");
        debug
            .field("sites", &self.sites)
            .field("anchors", &self.anchors)
            .field("polar", &self.polar)
            .field("polar_spoke", &self.polar_spoke);
        if !self.ink_owners.is_empty() {
            debug.field("ink_owners", &self.ink_owners);
        }
        if self.picking_profile != PickingProfile::LegacyGeometry {
            debug.field("picking_profile", &self.picking_profile);
        }
        if !self.part_data.is_empty() {
            debug.field("part_data", &self.part_data);
        }
        debug.finish()
    }
}
/// A path anchor follows the mean displacement of explicitly chosen command
/// endpoints. The reference position is exact at construction. Choosing every
/// unique symmetric glyph vertex makes translation, rotation and size changes
/// follow the actual delivered path, including spring intermediates.
///
/// This is an application-declared coordinate functional, not a bounding-box
/// center or a second hit shape. A changed command topology is rejected.
#[derive(Clone, Debug, PartialEq)]
pub struct InteractionAnchor {
    position: [f64; 2],
    reference: AnchorReference,
}
#[derive(Clone, Debug, PartialEq)]
enum AnchorReference {
    Path {
        points: Vec<(usize, [f64; 2])>,
        topology: Vec<u8>,
    },
    RectCenter([f64; 2]),
}
impl InteractionAnchor {
    pub fn path_points(
        geometry: &Geometry,
        position: [f64; 2],
        command_indices: &[usize],
    ) -> Result<Self> {
        Self::path_support(geometry, position, command_indices, true)
    }

    /// Follow the mean displacement of authored endpoints from an exact
    /// logical origin, which may differ from their reference centroid.
    pub(crate) fn path_offset_following(
        geometry: &Geometry,
        position: [f64; 2],
        command_indices: &[usize],
    ) -> Result<Self> {
        Self::path_support(geometry, position, command_indices, false)
    }

    fn path_support(
        geometry: &Geometry,
        position: [f64; 2],
        command_indices: &[usize],
        require_centroid: bool,
    ) -> Result<Self> {
        ensure(
            position.iter().all(|v| v.is_finite() && v.abs() <= 1e6),
            "invalid explicit interaction anchor",
        )?;
        let Geometry::Path(path) = geometry else {
            return Err(ChartError(
                "explicit interaction anchors require Path geometry".into(),
            ));
        };
        ensure(
            !path.is_empty()
                && path.len() <= 200_000
                && !command_indices.is_empty()
                && command_indices.len() <= path.len(),
            "invalid interaction anchor point budget",
        )?;
        ensure(
            valid_anchor_path(path),
            "invalid interaction anchor path geometry",
        )?;
        let mut seen = BTreeSet::new();
        let mut points = Vec::with_capacity(command_indices.len());
        for &index in command_indices {
            ensure(seen.insert(index), "duplicate interaction anchor command")?;
            let point = path
                .get(index)
                .and_then(anchor_endpoint)
                .ok_or_else(|| ChartError("interaction anchor command has no endpoint".into()))?;
            ensure(
                point.iter().all(|v| v.is_finite() && v.abs() <= 1e6),
                "invalid interaction anchor command point",
            )?;
            points.push((index, point));
        }
        if require_centroid {
            let origin = points[0].1;
            let mut mean = origin;
            let mut correction = [0., 0.];
            for &(_, point) in &points {
                for axis in 0..2 {
                    let term = (point[axis] - origin[axis]) / points.len() as f64;
                    let corrected = term - correction[axis];
                    let next = mean[axis] + corrected;
                    correction[axis] = (next - mean[axis]) - corrected;
                    mean[axis] = next;
                }
            }
            let magnitude = points
                .iter()
                .flat_map(|(_, p)| p)
                .chain(&position)
                .fold(1_f64, |m, v| m.max(v.abs()));
            // Endpoint coordinates have already rounded after glyph translation.
            // This tolerance covers bounded f64 summation, not an arbitrary offset.
            let tolerance = 32. * f64::EPSILON * magnitude;
            ensure(
                (0..2).all(|axis| (mean[axis] - position[axis]).abs() <= tolerance),
                "interaction anchor is not the declared support centroid",
            )?;
        }
        Ok(Self {
            position,
            reference: AnchorReference::Path {
                points,
                topology: path.iter().map(anchor_command_kind).collect(),
            },
        })
    }

    /// Preserve the exact data position when x/y were computed by subtracting
    /// half-extents and addition rounds to a different center.
    pub(crate) fn rounded_rect_center(geometry: &Geometry, position: [f64; 2]) -> Result<Self> {
        let bounds = anchor_rect(geometry)?;
        let center = rect_center(bounds);
        ensure(
            position.iter().all(|v| v.is_finite() && v.abs() <= 1e6)
                && (bounds.x == position[0] - bounds.width / 2. || center[0] == position[0])
                && (bounds.y == position[1] - bounds.height / 2. || center[1] == position[1]),
            "custom Rect must be centered at its data position; use a Path for offset geometry",
        )?;
        Ok(Self {
            position,
            reference: AnchorReference::RectCenter(center),
        })
    }

    fn command_count(&self) -> usize {
        match &self.reference {
            AnchorReference::Path { topology, .. } => topology.len(),
            AnchorReference::RectCenter(_) => 0,
        }
    }

    pub fn position(&self) -> [f64; 2] {
        self.position
    }
    /// Resolve against current physical geometry, never the target datum's
    /// saved screen coordinate. Non-affine factories can select their own
    /// coordinate functional; topology-changing resampling is unsupported.
    pub fn resolve(&self, geometry: &Geometry) -> Result<[f64; 2]> {
        let (points, topology) = match &self.reference {
            AnchorReference::Path { points, topology } => (points, topology),
            AnchorReference::RectCenter(reference) => {
                let current = rect_center(anchor_rect(geometry)?);
                return self.displaced([current[0] - reference[0], current[1] - reference[1]]);
            }
        };
        let Geometry::Path(path) = geometry else {
            return Err(ChartError(
                "interaction anchor lost its Path geometry".into(),
            ));
        };
        ensure(
            path.len() == topology.len()
                && path
                    .iter()
                    .zip(topology)
                    .all(|(c, kind)| anchor_command_kind(c) == *kind),
            "interaction anchor path topology changed",
        )?;
        ensure(
            valid_anchor_path(path),
            "invalid current interaction anchor geometry",
        )?;
        let mut delta = [0., 0.];
        let mut correction = [0., 0.];
        for &(index, reference) in points {
            let point = anchor_endpoint(&path[index]).expect("validated endpoint topology");
            for axis in 0..2 {
                let term = (point[axis] - reference[axis]) / points.len() as f64;
                let corrected = term - correction[axis];
                let next = delta[axis] + corrected;
                correction[axis] = (next - delta[axis]) - corrected;
                delta[axis] = next;
            }
        }
        self.displaced(delta)
    }

    fn displaced(&self, delta: [f64; 2]) -> Result<[f64; 2]> {
        let position = [self.position[0] + delta[0], self.position[1] + delta[1]];
        ensure(
            position.iter().all(|v| v.is_finite() && v.abs() <= 1e6),
            "resolved interaction anchor is out of bounds",
        )?;
        Ok(position)
    }
}
fn anchor_rect(geometry: &Geometry) -> Result<ClipRect> {
    let Geometry::Rect {
        x,
        y,
        width,
        height,
    } = *geometry
    else {
        return Err(ChartError(
            "interaction anchor lost its Rect geometry".into(),
        ));
    };
    let bounds = ClipRect {
        x,
        y,
        width,
        height,
    };
    bounds.validate()?;
    Ok(bounds)
}

fn rect_center(bounds: ClipRect) -> [f64; 2] {
    [bounds.x + bounds.width / 2., bounds.y + bounds.height / 2.]
}

fn anchor_endpoint(command: &PathCommand) -> Option<[f64; 2]> {
    match command {
        PathCommand::Move(p) | PathCommand::Line(p) | PathCommand::Cubic { end: p, .. } => {
            Some([p.x, p.y])
        }
        PathCommand::Close => None,
    }
}
fn anchor_command_kind(command: &PathCommand) -> u8 {
    match command {
        PathCommand::Move(_) => 0,
        PathCommand::Line(_) => 1,
        PathCommand::Cubic { .. } => 2,
        PathCommand::Close => 3,
    }
}
fn valid_anchor_path(path: &[PathCommand]) -> bool {
    let coordinate = |p: &glory::render::GraphicPoint| {
        [p.x, p.y].iter().all(|v| v.is_finite() && v.abs() <= 1e6)
    };
    let mut open = false;
    path.iter().all(|command| match command {
        PathCommand::Move(p) => {
            open = true;
            coordinate(p)
        }
        PathCommand::Line(p) => open && coordinate(p),
        PathCommand::Cubic { first, second, end } => {
            open && [first, second, end].into_iter().all(coordinate)
        }
        PathCommand::Close => {
            let valid = open;
            open = false;
            valid
        }
    })
}
/// A layout callback's Scene and business metadata form one candidate.
#[derive(Clone, Debug, PartialEq)]
pub struct SceneInteraction {
    pub scene: Scene,
    pub interaction: Option<InteractionBinding>,
    /// A resized layout may atomically replace the interaction plot as well.
    pub plot: Option<ClipRect>,
}
impl SceneInteraction {
    pub fn validate(&self) -> Result<()> {
        self.scene.validate()?;
        if let Some(data) = &self.interaction {
            data.validate(&self.scene)?;
        }
        crate::surface::validate_plot(self.plot, &self.scene)
    }
}
/// Prepare a candidate with the actual property-motion geometry/profile rules,
/// without advancing a clock or changing an accepted Scene.
pub fn validate_motion_scene(
    scene: &Scene,
    options: &motion::PropertyMotionOptions,
    bindings: &motion::ContourBindings,
) -> Result<()> {
    scene.validate()?;
    options.validate()?;
    motion::PropertyMotion::new_with_contours(scene.clone(), options.clone(), bindings.clone())
        .map(|_| ())
}
impl From<Scene> for SceneInteraction {
    fn from(scene: Scene) -> Self {
        Self {
            scene,
            interaction: None,
            plot: None,
        }
    }
}
impl InteractionBinding {
    fn validate_ink_owners(&self, scene: &Scene) -> Result<()> {
        if self.picking_profile == PickingProfile::LegacyGeometry
            && self.ink_owners.is_empty()
            && self.part_data.is_empty()
        {
            return Ok(());
        }
        ensure(
            self.picking_profile == PickingProfile::PaintedGeometry,
            "interaction ink owners require explicit PaintedGeometry profile",
        )?;
        ensure(
            self.sites
                .len()
                .checked_add(self.ink_owners.len())
                .is_some_and(|count| count <= MAX_MARKS)
                && self.part_data.len() <= MAX_MARKS,
            "interaction ink owner count budget exceeded",
        )?;
        let sites = self
            .sites
            .iter()
            .map(|site| (site.mark_id.as_str(), site))
            .collect::<BTreeMap<_, _>>();
        let marks = scene
            .marks
            .iter()
            .map(|mark| (mark.id.as_str(), mark))
            .collect::<BTreeMap<_, _>>();
        // Strict live frames retain every logical owner, even when the map is
        // empty. Geometry/value may animate; original canonical series may not.
        for site in &self.sites {
            ensure(
                marks
                    .get(site.mark_id.as_str())
                    .is_some_and(|m| m.series == site.series),
                "strict physical frame is missing or reassigning a logical owner",
            )?;
        }
        let mut bytes = 0usize;
        for (physical, owner) in &self.ink_owners {
            ensure(
                !physical.is_empty()
                    && physical.len() <= MAX_LABEL_BYTES
                    && !owner.is_empty()
                    && owner.len() <= MAX_LABEL_BYTES
                    && physical != owner
                    && !sites.contains_key(physical.as_str()),
                "invalid interaction ink owner identity",
            )?;
            bytes = bytes
                .checked_add(physical.len())
                .and_then(|sum| sum.checked_add(owner.len()))
                .ok_or_else(|| ChartError("interaction ink owner byte overflow".into()))?;
            ensure(
                bytes <= 4_000_000,
                "interaction ink owner byte budget exceeded",
            )?;
            let site = sites
                .get(owner.as_str())
                .ok_or_else(|| ChartError("interaction ink owner is not a logical site".into()))?;
            let mark = marks
                .get(physical.as_str())
                .ok_or_else(|| ChartError("interaction ink refers to missing mark".into()))?;
            let target = marks
                .get(owner.as_str())
                .ok_or_else(|| ChartError("interaction ink owner refers to missing mark".into()))?;
            ensure(
                mark.series == site.series && target.series == site.series,
                "interaction ink crosses logical series",
            )?;
        }
        for (physical, datum) in &self.part_data {
            ensure(
                !physical.is_empty()
                    && physical.len() <= MAX_LABEL_BYTES
                    && (sites.contains_key(physical.as_str())
                        || self.ink_owners.contains_key(physical))
                    && marks.contains_key(physical.as_str())
                    && !datum.source_id.is_empty()
                    && datum.source_id.len() <= MAX_LABEL_BYTES
                    && datum.value.is_finite(),
                "invalid interaction physical source metadata",
            )?;
            let logical = self.ink_owners.get(physical).unwrap_or(physical);
            ensure(
                sites.get(logical.as_str()).is_some_and(|s| {
                    marks
                        .get(physical.as_str())
                        .is_some_and(|m| m.series == s.series)
                }),
                "interaction physical source crosses logical series",
            )?;
            bytes = bytes
                .checked_add(physical.len())
                .and_then(|sum| sum.checked_add(datum.source_id.len()))
                .ok_or_else(|| ChartError("interaction physical source byte overflow".into()))?;
            ensure(
                bytes <= 4_000_000,
                "interaction ink owner/source byte budget exceeded",
            )?;
        }
        Ok(())
    }
    pub(crate) fn validate_frame_anchors(&self, scene: &Scene) -> Result<()> {
        // Aliases may not silently survive removal or reassignment of a live
        // physical part. Unlike optional entering anchors, both ends must exist.
        self.validate_ink_owners(scene)?;
        if self.anchors.is_empty() {
            return Ok(());
        }
        let marks = scene
            .marks
            .iter()
            .map(|mark| (mark.id.as_str(), mark))
            .collect::<BTreeMap<_, _>>();
        for (id, anchor) in &self.anchors {
            // New target IDs may not exist in the previous physical frame.
            // Every present datum must retain its declared topology.
            if let Some(mark) = marks.get(id.as_str()) {
                anchor.resolve(&mark.geometry)?;
            }
        }
        Ok(())
    }
    pub fn anchor_for(&self, mark: &Mark) -> Result<Option<[f64; 2]>> {
        if let Some(anchor) = self.anchors.get(&mark.id) {
            anchor.resolve(&mark.geometry).map(Some)
        } else {
            Ok(mark_center(mark))
        }
    }
    pub fn validate(&self, scene: &Scene) -> Result<()> {
        scene.validate()?;
        ensure(
            self.sites.len() <= MAX_MARKS,
            "interaction site budget exceeded",
        )?;
        let marks = scene
            .marks
            .iter()
            .map(|m| (m.id.as_str(), m))
            .collect::<std::collections::BTreeMap<_, _>>();
        let mut ids = BTreeSet::new();
        ensure(
            self.anchors.len() <= self.sites.len()
                && self
                    .anchors
                    .values()
                    .map(InteractionAnchor::command_count)
                    .sum::<usize>()
                    <= 200_000,
            "interaction anchor descriptor budget exceeded",
        )?;
        for site in &self.sites {
            ensure(
                !site.mark_id.is_empty()
                    && site.mark_id.len() <= 1024
                    && ids.insert(&site.mark_id)
                    && !site.series.is_empty()
                    && site.series.len() <= 1024
                    && site.label.len() <= 1024
                    && site
                        .formatted_value
                        .as_ref()
                        .is_none_or(|s| s.len() <= 1024)
                    && site.value.is_none_or(f64::is_finite),
                "invalid interaction site identity/value",
            )?;
            let mark = marks
                .get(site.mark_id.as_str())
                .ok_or_else(|| ChartError("interaction site refers to missing mark".into()))?;
            ensure(
                mark.series == site.series
                    && !mark.label.is_empty()
                    && site.value.is_none_or(|v| v == mark.value)
                    && self.anchor_for(mark)? == Some(site.position),
                "interaction site disagrees with candidate Scene",
            )?;
        }
        ensure(
            self.anchors.keys().all(|id| ids.contains(id)),
            "interaction anchor refers to an unbound datum",
        )?;
        self.validate_ink_owners(scene)?;
        if let Some(polar) = &self.polar {
            polar.validate()?;
            for slice in polar.slices() {
                ensure(
                    slice.member_ids.iter().all(|id| ids.contains(id)),
                    "polar slice refers to an unbound datum",
                )?;
            }
        }
        if let Some(stroke) = self.polar_spoke {
            ensure(
                stroke.width.is_finite() && (0.0001..=256.).contains(&stroke.width),
                "invalid polar spoke stroke",
            )?;
        }
        overlay_admission(scene, self.polar.is_some() && self.polar_spoke.is_some())?;
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ActiveInteraction {
    pub slice_id: Option<String>,
    pub label: Option<String>,
    pub anchor: [f64; 2],
    pub ids: Vec<String>,
    pub rows: Vec<InteractionSite>,
}
pub(crate) fn mark_center(mark: &Mark) -> Option<[f64; 2]> {
    match mark.geometry {
        Geometry::Circle { x, y, .. } => Some([x, y]),
        Geometry::Rect {
            x,
            y,
            width,
            height,
        } => Some([x + width / 2., y + height / 2.]),
        _ => None,
    }
}
/// Uses the common safe-text formatter and templates, honoring an explicit
/// value_format override. Auto uses the layout's per-row formatted value.
pub fn interaction_tooltip_text(
    options: &TooltipOptions,
    active: &ActiveInteraction,
) -> Result<String> {
    options.validate()?;
    ensure(
        active.label.as_ref().is_none_or(|s| s.len() <= 1024),
        "interaction tooltip heading budget exceeded",
    )?;
    ensure(
        active.rows.len() <= MAX_MARKS,
        "interaction tooltip row budget exceeded",
    )?;
    if active.rows.iter().any(|r| r.value.is_none())
        && matches!(options.missing, MissingValue::HideTooltip)
    {
        return Ok(String::new());
    }
    let mut lines = Vec::new();
    let mut shown = 0;
    for row in &active.rows {
        if row.value.is_none() && matches!(options.missing, MissingValue::OmitRow) {
            continue;
        }
        if shown == options.max_rows {
            if let Some(text) = &options.overflow_text {
                lines.push(text.clone());
            }
            break;
        }
        let mut local = options.clone();
        local.heading = None;
        local.overflow_text = None;
        if options.value_format == ValueFormat::Auto
            && row.value.is_some()
            && let Some(formatted) = &row.formatted_value
        {
            ensure(
                formatted.len() <= 1024,
                "formatted tooltip value budget exceeded",
            )?;
            for line in &mut local.lines {
                for part in line {
                    if matches!(part, TooltipPart::Value) {
                        *part = TooltipPart::Text(formatted.clone());
                    }
                }
            }
        }
        let text = tooltip_data_text(
            &local,
            &[TooltipDatum {
                id: &row.mark_id,
                label: &row.label,
                series: &row.series,
                value: row.value,
            }],
        )?;
        if !text.is_empty() {
            lines.push(text);
            shown += 1;
        }
    }
    if shown == 0 {
        return Ok(String::new());
    }
    if let Some(heading) = options.heading.as_ref().or(active.label.as_ref()) {
        lines.insert(0, heading.clone());
    }
    let output = lines.join("\n");
    ensure(
        output.len() <= 16384,
        "interaction tooltip text budget exceeded",
    )?;
    Ok(output)
}

pub(crate) fn overlay_admission(scene: &Scene, enabled: bool) -> Result<()> {
    if enabled {
        ensure(
            !scene
                .marks
                .iter()
                .any(|m| [CROSSHAIR_X_ID, CROSSHAIR_Y_ID, POLAR_SPOKE_ID].contains(&m.id.as_str())),
            "crosshair overlay ID conflicts with business mark",
        )?;
        ensure(
            scene.marks.len() <= MAX_MARKS - 2,
            "crosshair needs two reserved mark slots",
        )?;
        let commands = scene
            .marks
            .iter()
            .map(|m| match &m.geometry {
                Geometry::Path(p) => p.len(),
                _ => 0,
            })
            .sum::<usize>();
        ensure(
            commands <= 199996,
            "crosshair needs four reserved path commands",
        )?;
    }
    Ok(())
}

/// Reserve a full-frame, two-axis decorated Scene, including the cumulative
/// dash expansion/work budgets. A real plot is contained in this frame and
/// partial crosshair types cannot have longer spans than these two axes.
/// This does not publish, paint or mutate the caller's accepted Scene.
#[cfg_attr(not(all(feature = "web", target_arch = "wasm32")), allow(dead_code))]
pub(crate) fn overlay_scene_admission(scene: &Scene, options: &CrosshairOptions) -> Result<()> {
    scene.validate()?;
    options.validate()?;
    overlay_admission(scene, true)?;
    let plot = ClipRect {
        x: 0.,
        y: 0.,
        width: scene.width,
        height: scene.height,
    };
    let worst = CrosshairOptions {
        kind: CrosshairType::Cross,
        ..options.clone()
    };
    let marks = crosshair_marks(plot, [scene.width / 2., scene.height / 2.], &worst, None)?;
    let mut candidate = scene.clone();
    let start = candidate.marks.len();
    if !candidate.draw_order.is_empty() {
        candidate
            .draw_order
            .extend((start..start + marks.len()).map(DrawItem::Mark));
    }
    candidate.marks.extend(marks);
    candidate.validate()
}

#[cfg(test)]
mod overlay_scene_admission_tests {
    use super::*;

    fn scene() -> Scene {
        ChartConfig {
            width: 400.,
            height: 300.,
            ..Default::default()
        }
        .scene()
        .unwrap()
    }

    #[test]
    fn unrenderable_tiny_dash_rejects_without_mutating_source() {
        let source = scene();
        let old = source.clone();
        let options = CrosshairOptions {
            kind: CrosshairType::Left,
            dash: Some(StrokeDash {
                segments: vec![1e-8, 1e-8],
                offset: 0.,
            }),
            ..Default::default()
        };
        options.validate().unwrap();
        assert_eq!(
            crosshair_marks(
                ClipRect {
                    x: 0.,
                    y: 0.,
                    width: 400.,
                    height: 300.
                },
                [200., 150.],
                &options,
                None,
            )
            .unwrap()
            .len(),
            1,
        );
        assert!(overlay_scene_admission(&source, &options).is_err());
        assert_eq!(source, old);
    }

    #[test]
    fn valid_styles_preserve_mixed_order_and_original_scene() {
        let mut source = scene();
        source.marks.push(Mark::new(
            "datum",
            Geometry::Circle {
                x: 60.,
                y: 70.,
                radius: 5.,
            },
            Color(20, 50, 100, 255),
        ));
        source.labels.push(Label {
            x: 60.,
            y: 55.,
            text: "Current".into(),
            color: Color(20, 50, 100, 255),
            size: 12.,
            anchor: TextAnchor::Middle,
            rotation: 0.,
        });
        source.draw_order = vec![DrawItem::Label(0), DrawItem::Mark(0)];
        let old = source.clone();
        for dash in [
            None,
            Some(StrokeDash {
                segments: vec![6., 6.],
                offset: -2.,
            }),
            Some(StrokeDash {
                segments: vec![0., 8.],
                offset: 0.,
            }),
        ] {
            overlay_scene_admission(
                &source,
                &CrosshairOptions {
                    dash,
                    ..Default::default()
                },
            )
            .unwrap();
            assert_eq!(source, old);
        }
    }

    #[test]
    fn overlay_uses_remaining_global_dash_budget_not_per_mark_budget() {
        let mut source = scene();
        for i in 0..2 {
            let mut mark = Mark::new(
                format!("source-{i}"),
                Geometry::Path(vec![
                    PathCommand::Move(GraphicPoint::new(10., 20. + i as f64 * 20.)),
                    PathCommand::Line(GraphicPoint::new(109.98, 20. + i as f64 * 20.)),
                ]),
                Color(30, 60, 100, 255),
            );
            mark.label.clear();
            mark.fill = None;
            mark.stroke = Some(GraphicStroke {
                color: Color(30, 60, 100, 255),
                width: 1.,
                cap: StrokeCap::Butt,
            });
            mark.stroke_dash = Some(StrokeDash {
                segments: vec![0.001, 0.001],
                offset: 0.,
            });
            source.marks.push(mark);
        }
        source.validate().unwrap();
        let old = source.clone();
        let standalone = scene();
        overlay_scene_admission(&standalone, &CrosshairOptions::default()).unwrap();
        // Both sources together are legal, as is the overlay by itself, but
        // their combined dash command count is over the shared 200k limit.
        assert!(overlay_scene_admission(&source, &CrosshairOptions::default()).is_err());
        assert_eq!(source, old);
    }
}
