use std::fmt;

use glory::render::{GraphicPoint, GraphicStroke, PathCommand};
use glory::style::Color;
mod label_slots;
mod opacity_groups;
pub(crate) use label_slots::shifted_label_slots;
pub use opacity_groups::MarkOpacityGroup;

pub const MAX_MARKS: usize = 20_000;
pub const MAX_LABEL_BYTES: usize = 1024;
// Two bounded identity strings per explicit target and at most 256 rows.
const MAX_LEGEND_TARGET_IDENTITY_BYTES: usize = 256 * 2 * MAX_LABEL_BYTES;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChartError(pub String);
impl fmt::Display for ChartError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for ChartError {}
pub type Result<T> = std::result::Result<T, ChartError>;
pub(crate) fn ensure(ok: bool, message: &str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(ChartError(message.into()))
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Margin {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}
impl Default for Margin {
    fn default() -> Self {
        Self {
            top: 32.,
            right: 24.,
            bottom: 52.,
            left: 64.,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct Theme {
    pub colors: Vec<Color>,
    pub background: Color,
    pub text: Color,
    pub grid: Color,
    pub font_size: f64,
    pub font: crate::FontOptions,
}
impl Default for Theme {
    fn default() -> Self {
        Self {
            colors: vec![
                Color(78, 121, 167, 255),
                Color(242, 142, 43, 255),
                Color(225, 87, 89, 255),
                Color(118, 183, 178, 255),
                Color(89, 161, 79, 255),
                Color(237, 201, 72, 255),
                Color(176, 122, 161, 255),
                Color(255, 157, 167, 255),
            ],
            background: Color(255, 255, 255, 255),
            text: Color(45, 52, 64, 255),
            grid: Color(225, 229, 235, 255),
            font_size: 12.,
            font: crate::FontOptions::default(),
        }
    }
}
impl Theme {
    pub fn color(&self, index: usize) -> Color {
        self.colors
            .get(index % self.colors.len().max(1))
            .copied()
            .unwrap_or(Color(78, 121, 167, 255))
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ChartConfig {
    pub width: f64,
    pub height: f64,
    pub margin: Margin,
    pub theme: Theme,
    pub title: String,
}
impl Default for ChartConfig {
    fn default() -> Self {
        Self {
            width: 800.,
            height: 440.,
            margin: Margin::default(),
            theme: Theme::default(),
            title: "Chart".into(),
        }
    }
}
impl ChartConfig {
    pub fn validate(&self) -> Result<()> {
        ensure(
            [self.width, self.height]
                .iter()
                .all(|v| v.is_finite() && (1. ..=16384.).contains(v)),
            "dimensions must be finite and in 1..=16384",
        )?;
        ensure(
            [
                self.margin.top,
                self.margin.right,
                self.margin.bottom,
                self.margin.left,
            ]
            .iter()
            .all(|v| v.is_finite() && *v >= 0.),
            "margins must be finite and nonnegative",
        )?;
        ensure(
            self.inner_width() > 0. && self.inner_height() > 0.,
            "margins must leave a positive plot area",
        )?;
        ensure(
            self.title.len() <= MAX_LABEL_BYTES && !self.title.trim().is_empty(),
            "title must be nonempty and at most 1024 bytes",
        )?;
        self.theme.font.validate()?;
        ensure(
            self.theme.font_size.is_finite()
                && (1. ..=256.).contains(&self.theme.font_size)
                && self.theme.colors.len() <= 256,
            "invalid theme",
        )
    }
    pub fn inner_width(&self) -> f64 {
        self.width - self.margin.left - self.margin.right
    }
    pub fn inner_height(&self) -> f64 {
        self.height - self.margin.top - self.margin.bottom
    }
    pub fn scene(&self) -> Result<Scene> {
        self.validate()?;
        Ok(Scene {
            typography: crate::SceneTypography {
                default: self.theme.font.clone(),
                ..Default::default()
            },
            label_clips: Default::default(),
            width: self.width,
            height: self.height,
            title: self.title.clone(),
            background: self.theme.background,
            marks: vec![],
            opacity_groups: vec![],
            labels: vec![],
            legend: vec![],
            legend_roles: Default::default(),
            legend_targets: Default::default(),
            draw_order: vec![],
            label_series: std::collections::BTreeMap::new(),
        })
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum Geometry {
    Rect {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    },
    Circle {
        x: f64,
        y: f64,
        radius: f64,
    },
    Path(Vec<PathCommand>),
}
#[derive(Clone, Debug, PartialEq)]
pub struct Mark {
    pub id: String,
    pub series: String,
    pub label: String,
    pub value: f64,
    pub geometry: Geometry,
    pub fill: Option<Color>,
    /// Overrides the solid fill in SVG/Canvas using Scene logical coordinates.
    pub fill_paint: Option<crate::FillPaint>,
    pub stroke: Option<GraphicStroke>,
    pub stroke_dash: Option<crate::StrokeDash>,
    pub clip_path: Option<Box<crate::ClipPath>>,
    pub opacity: f64,
    /// Logical plot clipping, shared by SVG, Canvas, hit testing and Widget.
    pub clip: Option<ClipRect>,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClipRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
impl ClipRect {
    pub fn validate(self) -> Result<()> {
        ensure(
            [
                self.x,
                self.y,
                self.width,
                self.height,
                self.x + self.width,
                self.y + self.height,
            ]
            .into_iter()
            .all(coordinate)
                && self.width >= 0.
                && self.height >= 0.,
            "invalid clip rectangle",
        )
    }
    pub fn contains(self, x: f64, y: f64) -> bool {
        self.width > 0.
            && self.height > 0.
            && x >= self.x
            && x <= self.x + self.width
            && y >= self.y
            && y <= self.y + self.height
    }
    pub fn plot(c: &ChartConfig) -> Self {
        Self {
            x: c.margin.left,
            y: c.margin.top,
            width: c.inner_width(),
            height: c.inner_height(),
        }
    }
}
impl Mark {
    pub fn clipped_to(mut self, path: crate::ClipPath) -> Self {
        self.clip_path = Some(Box::new(path));
        self
    }
    pub fn has_fill(&self) -> bool {
        self.fill.is_some() || self.fill_paint.is_some()
    }
    pub fn new(id: impl Into<String>, geometry: Geometry, fill: Color) -> Self {
        let id = id.into();
        Self {
            label: id.clone(),
            id,
            series: String::new(),
            value: 0.,
            geometry,
            fill: Some(fill),
            fill_paint: None,
            stroke: None,
            stroke_dash: None,
            clip_path: None,
            opacity: 1.,
            clip: None,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextAnchor {
    #[default]
    Start,
    Middle,
    End,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Label {
    /// Clockwise degrees about the text anchor `(x, y)`, in -360..=360.
    pub rotation: f64,
    pub x: f64,
    pub y: f64,
    pub text: String,
    pub color: Color,
    pub size: f64,
    pub anchor: TextAnchor,
}
#[derive(Clone, Debug, PartialEq)]
pub struct LegendItem {
    pub id: String,
    pub label: String,
    pub color: Color,
}
/// Legacy domain of a legend row without an explicit business target.
/// Missing role entries keep Series when no target is configured.
/// Without an explicit target, Category identities use `LegendItem::id`.
/// Activating a Category never implicitly changes hidden series.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LegendRole {
    #[default]
    Series,
    Category,
}
/// Business identity activated by an existing, independently keyed legend row.
/// The target is a scalar source identity, never a reference to another row.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LegendTarget {
    /// Canonical series visibility identity; it need not have visible marks.
    Series(String),
    /// Original category identity. An empty raw category is allowed.
    Category(String),
}
impl LegendTarget {
    /// Domain of this business identity, independent of its row's display text.
    pub fn role(&self) -> LegendRole {
        match self {
            Self::Series(_) => LegendRole::Series,
            Self::Category(_) => LegendRole::Category,
        }
    }

    /// Canonical series ID or original raw category key, not a transport row ID.
    pub fn id(&self) -> &str {
        match self {
            Self::Series(id) | Self::Category(id) => id,
        }
    }
}
#[derive(Clone, PartialEq)]
pub struct Scene {
    pub typography: crate::SceneTypography,
    pub width: f64,
    pub height: f64,
    pub title: String,
    pub background: Color,
    pub marks: Vec<Mark>,
    /// Flat, disjoint groups of consecutive painted marks. Members paint at
    /// their own opacity; the finished group composites once at this opacity.
    /// Labels cannot be members. Empty preserves the ordinary painting path.
    pub opacity_groups: Vec<MarkOpacityGroup>,
    pub labels: Vec<Label>,
    pub legend: Vec<LegendItem>,
    /// Sparse roles keyed by actual legend IDs. Category items are static
    /// unless a host explicitly enables independent activation callbacks.
    /// Roles do not reinterpret programmatic series visibility operations.
    pub legend_roles: std::collections::BTreeMap<String, LegendRole>,
    /// Sparse business targets keyed by actual, unique legend row IDs. Missing
    /// entries retain the old role plus row-ID identity. Repeated targets are
    /// allowed; multiple rows then share one business visibility state.
    pub legend_targets: std::collections::BTreeMap<String, LegendTarget>,
    /// Empty preserves the default geometry-then-text order. Otherwise every
    /// geometry/text slot occurs exactly once, allowing arbitrary layer order.
    pub draw_order: Vec<DrawItem>,
    /// Optional ownership for labels that should follow series visibility.
    pub label_series: std::collections::BTreeMap<usize, String>,
    pub label_clips: std::collections::BTreeMap<usize, crate::ClipPath>,
}
impl fmt::Debug for Scene {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Preserve the old default Debug representation, including pretty
        // formatting. Equality still includes roles even when they are empty.
        let mut out = f.debug_struct("Scene");
        out.field("typography", &self.typography)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("title", &self.title)
            .field("background", &self.background)
            .field("marks", &self.marks)
            .field("labels", &self.labels)
            .field("legend", &self.legend);
        if !self.legend_roles.is_empty() {
            out.field("legend_roles", &self.legend_roles);
        }
        if !self.legend_targets.is_empty() {
            out.field("legend_targets", &self.legend_targets);
        }
        if !self.opacity_groups.is_empty() {
            out.field("opacity_groups", &self.opacity_groups);
        }
        out.field("draw_order", &self.draw_order)
            .field("label_series", &self.label_series)
            .field("label_clips", &self.label_clips)
            .finish()
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DrawItem {
    Mark(usize),
    Label(usize),
}
#[derive(Clone, Copy, Debug)]
pub enum PaintItem<'a> {
    Mark(&'a Mark),
    Label(&'a Label),
}
impl Geometry {
    pub fn commands(&self) -> Vec<PathCommand> {
        use PathCommand::{Close, Cubic, Line, Move};
        let p = GraphicPoint::new;
        match self {
            Self::Path(commands) => commands.clone(),
            Self::Rect {
                x,
                y,
                width: w,
                height: h,
            } => vec![
                Move(p(*x, *y)),
                Line(p(x + w, *y)),
                Line(p(x + w, y + h)),
                Line(p(*x, y + h)),
                Close,
            ],
            Self::Circle { x, y, radius: r } => {
                let k = r * 0.5522847498307936;
                vec![
                    Move(p(x + r, *y)),
                    Cubic {
                        first: p(x + r, y + k),
                        second: p(x + k, y + r),
                        end: p(*x, y + r),
                    },
                    Cubic {
                        first: p(x - k, y + r),
                        second: p(x - r, y + k),
                        end: p(x - r, *y),
                    },
                    Cubic {
                        first: p(x - r, y - k),
                        second: p(x - k, y - r),
                        end: p(*x, y - r),
                    },
                    Cubic {
                        first: p(x + k, y - r),
                        second: p(x + r, y - k),
                        end: p(x + r, *y),
                    },
                    Close,
                ]
            }
        }
    }
}

fn coordinate(v: f64) -> bool {
    v.is_finite() && v.abs() <= 1_000_000.
}
impl Scene {
    /// Resolve an actual legend row to its business identity. Unknown rows
    /// return None even if a malformed sparse map contains their key. A target
    /// is not recursively interpreted as another legend row.
    pub fn legend_target(&self, id: &str) -> Option<LegendTarget> {
        self.legend.iter().any(|item| item.id == id).then(|| {
            self.legend_targets.get(id).cloned().unwrap_or_else(|| {
                match self.legend_roles.get(id).copied().unwrap_or_default() {
                    LegendRole::Series => LegendTarget::Series(id.to_owned()),
                    LegendRole::Category => LegendTarget::Category(id.to_owned()),
                }
            })
        })
    }

    /// Query the resolved domain of an actual legend row.
    pub fn legend_role(&self, id: &str) -> Option<LegendRole> {
        self.legend_target(id).map(|target| target.role())
    }

    /// Presentation visibility is independent for category legend items.
    /// Explicit `without_series` and host hidden-input APIs remain series APIs.
    pub fn legend_hidden(&self, id: &str, hidden: &std::collections::BTreeSet<String>) -> bool {
        matches!(self.legend_target(id), Some(LegendTarget::Series(target)) if hidden.contains(&target))
    }

    pub fn resolved_label_font(&self, index: usize) -> crate::ResolvedFont {
        self.typography.label_font(index)
    }
    pub fn draw_items(&self) -> Vec<DrawItem> {
        if self.draw_order.is_empty() {
            (0..self.marks.len())
                .map(DrawItem::Mark)
                .chain((0..self.labels.len()).map(DrawItem::Label))
                .collect()
        } else {
            self.draw_order.clone()
        }
    }
    pub fn paint_items(&self) -> impl DoubleEndedIterator<Item = PaintItem<'_>> {
        let count = if self.draw_order.is_empty() {
            self.marks.len() + self.labels.len()
        } else {
            self.draw_order.len()
        };
        (0..count).filter_map(move |index| {
            let item = if self.draw_order.is_empty() {
                if index < self.marks.len() {
                    DrawItem::Mark(index)
                } else {
                    DrawItem::Label(index - self.marks.len())
                }
            } else {
                self.draw_order[index]
            };
            match item {
                DrawItem::Mark(i) => self.marks.get(i).map(PaintItem::Mark),
                DrawItem::Label(i) => self.labels.get(i).map(PaintItem::Label),
            }
        })
    }
    pub fn paint_marks(&self) -> impl DoubleEndedIterator<Item = &Mark> {
        self.paint_items().filter_map(|item| match item {
            PaintItem::Mark(mark) => Some(mark),
            PaintItem::Label(_) => None,
        })
    }
    /// Validate the complete frame before publishing it to any renderer.
    pub fn validate(&self) -> Result<()> {
        self.typography.validate(self.labels.len())?;
        ensure(
            [self.width, self.height]
                .iter()
                .all(|v| v.is_finite() && (1. ..=16384.).contains(v)),
            "invalid scene dimensions",
        )?;
        ensure(
            !self.title.trim().is_empty() && self.title.len() <= MAX_LABEL_BYTES,
            "invalid scene title",
        )?;
        ensure(
            self.marks.len() <= MAX_MARKS
                && self.labels.len() <= MAX_MARKS
                && self.legend.len() <= 256
                && self.legend_roles.len() <= 256
                && self.legend_targets.len() <= 256,
            "scene budget exceeded",
        )?;
        ensure(
            self.label_series.len() <= self.labels.len()
                && self.label_series.iter().all(|(i, s)| {
                    *i < self.labels.len() && !s.is_empty() && s.len() <= MAX_LABEL_BYTES
                }),
            "invalid label series ownership",
        )?;
        if !self.draw_order.is_empty() {
            ensure(
                self.draw_order.len() == self.marks.len() + self.labels.len(),
                "custom paint order must reference every item exactly once",
            )?;
            let mut items = std::collections::BTreeSet::new();
            for item in &self.draw_order {
                ensure(
                    match item {
                        DrawItem::Mark(i) => *i < self.marks.len(),
                        DrawItem::Label(i) => *i < self.labels.len(),
                    } && items.insert(item),
                    "duplicate or invalid custom paint item",
                )?;
            }
        }
        let mut ids = std::collections::BTreeSet::new();
        let mut commands = 0usize;
        let mut paint_stops = 0usize;
        let mut clip_commands = 0usize;
        let mut clip_segments = 0usize;
        let mut clip_work = 0usize;
        let mut dash_commands = 0usize;
        let mut dash_work = 0usize;
        let mut dash_segments = 0usize;
        ensure(
            self.label_clips.len() <= self.labels.len(),
            "label clip budget exceeded",
        )?;
        for (index, clip) in &self.label_clips {
            ensure(
                *index < self.labels.len(),
                "label clip references an absent slot",
            )?;
            let cost = clip.picking_stats()?;
            clip_commands += clip.command_count();
            clip_segments += cost.flattened_segments;
            clip_work += cost.work;
            ensure(
                clip_commands <= 200_000 && clip_segments <= 200_000 && clip_work <= 4_000_000,
                "scene clip command/picking budget exceeded",
            )?;
        }
        ensure(
            clip_commands <= 200_000,
            "scene clip command budget exceeded",
        )?;
        for mark in &self.marks {
            if let Some(clip) = &mark.clip_path {
                let cost = clip.picking_stats()?;
                clip_commands += clip.command_count();
                clip_segments += cost.flattened_segments;
                clip_work += cost.work;
                ensure(
                    clip_segments <= 200_000 && clip_work <= 4_000_000,
                    "scene clip picking budget exceeded",
                )?;
            }
            ensure(
                clip_commands <= 200_000,
                "scene clip command budget exceeded",
            )?;
            if let Some(dash) = &mark.stroke_dash {
                dash.validate()?;
                ensure(mark.stroke.is_some(), "stroke dash requires a stroke")?;
            }
            if let Some(clip) = mark.clip {
                clip.validate()?;
            }
            ensure(
                !mark.id.is_empty() && mark.id.len() <= MAX_LABEL_BYTES && ids.insert(&mark.id),
                "empty, duplicate or oversized mark ID",
            )?;
            ensure(
                mark.label.len() <= MAX_LABEL_BYTES && mark.series.len() <= MAX_LABEL_BYTES,
                "mark label budget exceeded",
            )?;
            ensure(
                mark.value.is_finite()
                    && mark.opacity.is_finite()
                    && (0. ..=1.).contains(&mark.opacity),
                "invalid mark value or opacity",
            )?;
            if let Some(s) = mark.stroke {
                ensure(
                    s.width.is_finite() && (0.000001..=10000.).contains(&s.width),
                    "invalid stroke width",
                )?;
            }
            if let Some(paint) = &mark.fill_paint {
                paint.validate()?;
                paint_stops += paint.stops().len();
                ensure(
                    paint_stops <= crate::MAX_SCENE_GRADIENT_STOPS,
                    "scene gradient stop budget exceeded",
                )?;
            }
            match &mark.geometry {
                Geometry::Rect {
                    x,
                    y,
                    width,
                    height,
                } => ensure(
                    [*x, *y, *width, *height, x + width, y + height]
                        .into_iter()
                        .all(coordinate)
                        && *width >= 0.
                        && *height >= 0.,
                    "invalid rectangle",
                )?,
                Geometry::Circle { x, y, radius } => ensure(
                    [
                        *x,
                        *y,
                        *radius,
                        x + radius,
                        x - radius,
                        y + radius,
                        y - radius,
                    ]
                    .into_iter()
                    .all(coordinate)
                        && *radius >= 0.,
                    "invalid circle",
                )?,
                Geometry::Path(path) => {
                    commands = commands
                        .checked_add(path.len())
                        .ok_or_else(|| ChartError("path budget exceeded".into()))?;
                    ensure(commands <= 200_000, "path command budget exceeded")?;
                    let mut open = false;
                    for cmd in path {
                        match cmd {
                            PathCommand::Move(p) => {
                                ensure(coordinate(p.x) && coordinate(p.y), "invalid path point")?;
                                open = true;
                            }
                            PathCommand::Line(p) => ensure(
                                open && coordinate(p.x) && coordinate(p.y),
                                "invalid path segment",
                            )?,
                            PathCommand::Cubic { first, second, end } => ensure(
                                open && [first, second, end]
                                    .iter()
                                    .all(|p| coordinate(p.x) && coordinate(p.y)),
                                "invalid cubic segment",
                            )?,
                            PathCommand::Close => {
                                ensure(open, "close without open path")?;
                                open = false;
                            }
                        }
                    }
                }
            }
            if let Some(dash) = &mark.stroke_dash {
                let limits = crate::StrokeDashLimits {
                    max_output_commands: crate::MAX_DASH_COMMANDS - dash_commands,
                    max_flattened_segments: crate::MAX_DASH_COMMANDS - dash_segments,
                    max_work: crate::MAX_DASH_WORK.saturating_sub(dash_work),
                    ..Default::default()
                };
                let detailed = crate::dash_geometry_detailed(&mark.geometry, dash, &limits)?;
                dash_commands += detailed.geometry.commands().len();
                dash_segments += detailed.flattened_segments;
                dash_work += detailed.work;
            }
        }
        for label in &self.labels {
            ensure(
                coordinate(label.x)
                    && coordinate(label.y)
                    && label.rotation.is_finite()
                    && (-360. ..=360.).contains(&label.rotation)
                    && label.size.is_finite()
                    && (1. ..=256.).contains(&label.size)
                    && label.text.len() <= MAX_LABEL_BYTES,
                "invalid text label",
            )?;
        }
        let mut legends = std::collections::BTreeSet::new();
        for item in &self.legend {
            ensure(
                !item.id.is_empty()
                    && item.id.len() <= MAX_LABEL_BYTES
                    && item.label.len() <= MAX_LABEL_BYTES
                    && legends.insert(&item.id),
                "invalid legend ID or label",
            )?;
        }
        ensure(
            self.legend_roles.keys().all(|id| legends.contains(id)),
            "legend role references an unknown legend identity",
        )?;
        let mut target_identity_bytes = 0usize;
        for (row, target) in &self.legend_targets {
            ensure(
                legends.contains(row),
                "legend target references an unknown legend row",
            )?;
            ensure(
                target.id().len() <= MAX_LABEL_BYTES
                    && (!matches!(target, LegendTarget::Series(_)) || !target.id().is_empty()),
                "invalid legend target identity",
            )?;
            ensure(
                self.legend_roles
                    .get(row)
                    .is_none_or(|role| *role == target.role()),
                "legend target contradicts its explicit role",
            )?;
            target_identity_bytes = target_identity_bytes
                .checked_add(row.len())
                .and_then(|bytes| bytes.checked_add(target.id().len()))
                .ok_or_else(|| ChartError("legend target identity budget overflow".into()))?;
            ensure(
                target_identity_bytes <= MAX_LEGEND_TARGET_IDENTITY_BYTES,
                "legend target identity byte budget exceeded",
            )?;
        }
        self.validate_opacity_groups()?;
        Ok(())
    }
    /// Topmost painted mark. Cubics use 24 subdivisions for hit testing.
    /// Coordinates are in scene units; browser adapters must undo CSS/DPR scaling.
    pub fn hit_test(&self, x: f64, y: f64) -> Option<&Mark> {
        if !x.is_finite() || !y.is_finite() || x < 0. || y < 0. || x > self.width || y > self.height
        {
            return None;
        }
        let groups = self.mark_group_opacities();
        self.paint_marks().rev().find(|m| {
            m.opacity > 0. && groups.get(&m.id).is_none_or(|o| *o > 0.) && contains(m, x, y)
        })
    }
    /// Remove only the selected physical mark ink from a private host paint
    /// projection. Labels, typography, legends and authored metadata remain
    /// unchanged in the source Scene.
    pub(crate) fn without_mark_ink(&self, ids: &std::collections::BTreeSet<String>) -> Self {
        if ids.is_empty() {
            return self.clone();
        }
        let mut scene = self.clone();
        let mut indices = std::collections::BTreeMap::new();
        scene.marks.clear();
        for (index, mark) in self.marks.iter().enumerate() {
            if !ids.contains(&mark.id) {
                indices.insert(index, scene.marks.len());
                scene.marks.push(mark.clone());
            }
        }
        scene.draw_order = self
            .draw_order
            .iter()
            .filter_map(|item| match item {
                DrawItem::Mark(index) => indices.get(index).copied().map(DrawItem::Mark),
                DrawItem::Label(index) => Some(DrawItem::Label(*index)),
            })
            .collect();
        scene.opacity_groups = self
            .opacity_groups
            .iter()
            .filter_map(|group| {
                let members: Vec<_> = group
                    .members
                    .iter()
                    .filter(|id| !ids.contains(*id))
                    .cloned()
                    .collect();
                (!members.is_empty()).then(|| MarkOpacityGroup {
                    members,
                    ..group.clone()
                })
            })
            .collect();
        scene
    }
    /// Private paint projection for accepted Label components. Every sparse
    /// label index, including typography and authored draw order, is remapped.
    pub(crate) fn without_label_ink(&self, indices: &std::collections::BTreeSet<usize>) -> Self {
        if indices.is_empty() {
            return self.clone();
        }
        let mut scene = self.clone();
        let mut remap = std::collections::BTreeMap::new();
        scene.labels.clear();
        scene.label_clips.clear();
        scene.label_series.clear();
        scene.typography.labels.clear();
        scene.typography.label_whitespace.clear();
        scene.typography.label_strokes.clear();
        for (index, label) in self.labels.iter().enumerate() {
            if indices.contains(&index) {
                continue;
            }
            let next = scene.labels.len();
            remap.insert(index, next);
            scene.labels.push(label.clone());
            if let Some(value) = self.label_clips.get(&index) {
                scene.label_clips.insert(next, value.clone());
            }
            if let Some(value) = self.label_series.get(&index) {
                scene.label_series.insert(next, value.clone());
            }
            if let Some(value) = self.typography.labels.get(&index) {
                scene.typography.labels.insert(next, value.clone());
            }
            if let Some(value) = self.typography.label_whitespace.get(&index) {
                scene.typography.label_whitespace.insert(next, *value);
            }
            if let Some(stroke) = self.typography.label_strokes.get(&index) {
                scene.typography.label_strokes.insert(next, *stroke);
            }
        }
        scene.draw_order = self
            .draw_order
            .iter()
            .filter_map(|item| match item {
                DrawItem::Mark(index) => Some(DrawItem::Mark(*index)),
                DrawItem::Label(index) => remap.get(index).copied().map(DrawItem::Label),
            })
            .collect();
        scene
    }
    /// Keep the metadata and geometry shared by SVG and Canvas while hiding series.
    pub fn without_series(&self, hidden: &std::collections::BTreeSet<String>) -> Self {
        let mut scene = self.clone();
        let mut marks = std::collections::BTreeMap::new();
        scene.marks.clear();
        for (i, mark) in self.marks.iter().enumerate() {
            if !hidden.contains(&mark.series) {
                marks.insert(i, scene.marks.len());
                scene.marks.push(mark.clone());
            }
        }
        let mut labels = std::collections::BTreeMap::new();
        scene.labels.clear();
        scene.label_series.clear();
        scene.label_clips.clear();
        for (i, label) in self.labels.iter().enumerate() {
            if self
                .label_series
                .get(&i)
                .is_none_or(|s| !hidden.contains(s))
            {
                let next = scene.labels.len();
                labels.insert(i, next);
                scene.labels.push(label.clone());
                if let Some(clip) = self.label_clips.get(&i) {
                    scene.label_clips.insert(next, clip.clone());
                }
                if let Some(series) = self.label_series.get(&i) {
                    scene.label_series.insert(next, series.clone());
                }
            }
        }
        scene.draw_order = self
            .draw_order
            .iter()
            .filter_map(|item| match item {
                DrawItem::Mark(i) => marks.get(i).copied().map(DrawItem::Mark),
                DrawItem::Label(i) => labels.get(i).copied().map(DrawItem::Label),
            })
            .collect();
        scene.typography.remap_labels(&labels);
        let kept: std::collections::BTreeSet<_> =
            scene.marks.iter().map(|m| m.id.as_str()).collect();
        scene.opacity_groups = self
            .opacity_groups
            .iter()
            .filter_map(|group| {
                let members: Vec<_> = group
                    .members
                    .iter()
                    .filter(|id| kept.contains(id.as_str()))
                    .cloned()
                    .collect();
                (!members.is_empty()).then(|| MarkOpacityGroup {
                    members,
                    ..group.clone()
                })
            })
            .collect();
        scene
    }
    /// Interpolate compatible frames by stable mark identity. Topology changes are
    /// deliberately rejected so callers can choose a fade or immediate replacement.
    pub fn interpolate(&self, next: &Self, progress: f64) -> Result<Self> {
        self.validate()?;
        next.validate()?;
        ensure(
            progress.is_finite() && (0. ..=1.).contains(&progress),
            "animation progress must be in 0..=1",
        )?;
        ensure(
            self.width == next.width
                && self.height == next.height
                && self.marks.len() == next.marks.len(),
            "animation requires compatible frames",
        )?;
        let previous: std::collections::BTreeMap<_, _> =
            self.marks.iter().map(|m| (&m.id, m)).collect();
        let mut out = next.clone();
        out.opacity_groups = self.interpolate_opacity_groups(next, progress)?;
        for mark in &mut out.marks {
            let old = previous
                .get(&mark.id)
                .ok_or_else(|| ChartError("animation mark identity changed".into()))?;
            ensure(
                old.clip_path == mark.clip_path && old.stroke_dash == mark.stroke_dash,
                "animation clip/dash topology changed",
            )?;
            ensure(
                old.fill_paint == mark.fill_paint,
                "animation gradient definition changed; choose a topology fade",
            )?;
            mark.geometry = interpolate_geometry(&old.geometry, &mark.geometry, progress)?;
            mark.clip = match (old.clip, mark.clip) {
                (Some(a), Some(b)) => Some(ClipRect {
                    x: lerp(a.x, b.x, progress),
                    y: lerp(a.y, b.y, progress),
                    width: lerp(a.width, b.width, progress),
                    height: lerp(a.height, b.height, progress),
                }),
                (None, None) => None,
                _ => return Err(ChartError("animation clip topology changed".into())),
            };
            mark.opacity = lerp(old.opacity, mark.opacity, progress);
            mark.value = lerp(old.value, mark.value, progress);
            mark.fill = match (old.fill, mark.fill) {
                (Some(a), Some(b)) => Some(interpolate_color(a, b, progress)),
                (None, None) => None,
                _ => return Err(ChartError("animation fill topology changed".into())),
            };
            mark.stroke = match (old.stroke, mark.stroke) {
                (Some(a), Some(b)) if a.cap == b.cap => Some(GraphicStroke {
                    width: lerp(a.width, b.width, progress),
                    color: interpolate_color(a.color, b.color, progress),
                    cap: b.cap,
                }),
                (None, None) => None,
                _ => return Err(ChartError("animation stroke topology changed".into())),
            };
        }
        out.background = interpolate_color(self.background, next.background, progress);
        out.validate()?;
        Ok(out)
    }
}
fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a * (1. - t) + b * t
}
fn interpolate_color(a: Color, b: Color, t: f64) -> Color {
    let c = |a: u8, b: u8| lerp(a as f64, b as f64, t).round() as u8;
    Color(c(a.0, b.0), c(a.1, b.1), c(a.2, b.2), c(a.3, b.3))
}
fn interpolate_geometry(a: &Geometry, b: &Geometry, t: f64) -> Result<Geometry> {
    let point =
        |a: GraphicPoint, b: GraphicPoint| GraphicPoint::new(lerp(a.x, b.x, t), lerp(a.y, b.y, t));
    Ok(match (a, b) {
        (
            Geometry::Rect {
                x: ax,
                y: ay,
                width: aw,
                height: ah,
            },
            Geometry::Rect {
                x: bx,
                y: by,
                width: bw,
                height: bh,
            },
        ) => Geometry::Rect {
            x: lerp(*ax, *bx, t),
            y: lerp(*ay, *by, t),
            width: lerp(*aw, *bw, t),
            height: lerp(*ah, *bh, t),
        },
        (
            Geometry::Circle {
                x: ax,
                y: ay,
                radius: ar,
            },
            Geometry::Circle {
                x: bx,
                y: by,
                radius: br,
            },
        ) => Geometry::Circle {
            x: lerp(*ax, *bx, t),
            y: lerp(*ay, *by, t),
            radius: lerp(*ar, *br, t),
        },
        (Geometry::Path(a), Geometry::Path(b)) if a.len() == b.len() => Geometry::Path(
            a.iter()
                .zip(b)
                .map(|(a, b)| {
                    Ok(match (a, b) {
                        (PathCommand::Move(a), PathCommand::Move(b)) => {
                            PathCommand::Move(point(*a, *b))
                        }
                        (PathCommand::Line(a), PathCommand::Line(b)) => {
                            PathCommand::Line(point(*a, *b))
                        }
                        (
                            PathCommand::Cubic {
                                first: a,
                                second: c,
                                end: e,
                            },
                            PathCommand::Cubic {
                                first: b,
                                second: d,
                                end: f,
                            },
                        ) => PathCommand::Cubic {
                            first: point(*a, *b),
                            second: point(*c, *d),
                            end: point(*e, *f),
                        },
                        (PathCommand::Close, PathCommand::Close) => PathCommand::Close,
                        _ => return Err(ChartError("animation path topology changed".into())),
                    })
                })
                .collect::<Result<Vec<_>>>()?,
        ),
        _ => return Err(ChartError("animation geometry topology changed".into())),
    })
}
fn dash_unit(a: GraphicPoint, b: GraphicPoint) -> [f64; 2] {
    let length = (b.x - a.x).hypot(b.y - a.y);
    [(b.x - a.x) / length, (b.y - a.y) / length]
}
fn dash_rectangle_hit(
    point: GraphicPoint,
    origin: GraphicPoint,
    tangent: [f64; 2],
    lo: f64,
    hi: f64,
    half: f64,
) -> bool {
    let dx = point.x - origin.x;
    let dy = point.y - origin.y;
    let along = dx * tangent[0] + dy * tangent[1];
    let across = (-dx * tangent[1] + dy * tangent[0]).abs();
    (lo - along)
        .max(along - hi)
        .max(0.)
        .hypot((across - half).max(0.))
        <= 2.
}
fn dash_polygon_hit(point: GraphicPoint, polygon: &[GraphicPoint]) -> bool {
    let mut positive = false;
    let mut negative = false;
    let mut close = false;
    for (a, b) in polygon
        .iter()
        .zip(polygon.iter().cycle().skip(1))
        .take(polygon.len())
    {
        let dx = b.x - a.x;
        let dy = b.y - a.y;
        let cross = dx * (point.y - a.y) - dy * (point.x - a.x);
        positive |= cross > 0.;
        negative |= cross < 0.;
        let length = dx * dx + dy * dy;
        let t = if length == 0. {
            0.
        } else {
            ((point.x - a.x) * dx + (point.y - a.y) * dy) / length
        }
        .clamp(0., 1.);
        close |= (point.x - a.x - t * dx).hypot(point.y - a.y - t * dy) <= 2.;
    }
    close || (positive != negative)
}
fn dash_join_hit(
    a: GraphicPoint,
    b: GraphicPoint,
    c: GraphicPoint,
    half: f64,
    point: GraphicPoint,
) -> bool {
    let u = dash_unit(a, b);
    let v = dash_unit(b, c);
    let cross = u[0] * v[1] - u[1] * v[0];
    if cross.abs() < 1e-12 {
        return false;
    }
    let side = if cross > 0. { -1. } else { 1. };
    let n = [-u[1] * side, u[0] * side];
    let m = [-v[1] * side, v[0] * side];
    let p = GraphicPoint::new(b.x + n[0] * half, b.y + n[1] * half);
    let q = GraphicPoint::new(b.x + m[0] * half, b.y + m[1] * half);
    let denominator = 1. + (u[0] * v[0] + u[1] * v[1]).clamp(-1., 1.);
    // SVG/CPU default miterlimit=4; beyond this the join is a bevel. This is
    // bounded even at an almost exact reversal, without unbounded intersections.
    if denominator > 0. && (2. / denominator).sqrt() <= 4. {
        let tip = GraphicPoint::new(
            b.x + (n[0] + m[0]) * half / denominator,
            b.y + (n[1] + m[1]) * half / denominator,
        );
        dash_polygon_hit(point, &[b, p, tip, q])
    } else {
        dash_polygon_hit(point, &[b, p, q])
    }
}
fn dash_run_hit(
    points: &[GraphicPoint],
    closed: bool,
    stroke: GraphicStroke,
    point: GraphicPoint,
) -> bool {
    if points.len() < 2 {
        return false;
    }
    let half = stroke.width / 2.;
    for pair in points.windows(2) {
        let length = (pair[1].x - pair[0].x).hypot(pair[1].y - pair[0].y);
        if dash_rectangle_hit(
            point,
            pair[0],
            dash_unit(pair[0], pair[1]),
            0.,
            length,
            half,
        ) {
            return true;
        }
    }
    if closed {
        let a = *points.last().unwrap();
        let b = points[0];
        let length = (a.x - b.x).hypot(a.y - b.y);
        if length > 0. && dash_rectangle_hit(point, a, dash_unit(a, b), 0., length, half) {
            return true;
        }
    }
    for triple in points.windows(3) {
        if dash_join_hit(triple[0], triple[1], triple[2], half, point) {
            return true;
        }
    }
    let last = points.len() - 1;
    if closed {
        return dash_join_hit(points[last - 1], points[last], points[0], half, point)
            || dash_join_hit(points[last], points[0], points[1], half, point);
    }
    match stroke.cap {
        glory::render::StrokeCap::Butt => false,
        glory::render::StrokeCap::Round => {
            (point.x - points[0].x).hypot(point.y - points[0].y) <= half + 2.
                || (point.x - points[last].x).hypot(point.y - points[last].y) <= half + 2.
        }
        glory::render::StrokeCap::Square => {
            dash_rectangle_hit(
                point,
                points[0],
                dash_unit(points[0], points[1]),
                -half,
                0.,
                half,
            ) || dash_rectangle_hit(
                point,
                points[last],
                dash_unit(points[last - 1], points[last]),
                0.,
                half,
                half,
            )
        }
    }
}
fn dashed_path_hit(geometry: &Geometry, stroke: GraphicStroke, x: f64, y: f64) -> bool {
    let Geometry::Path(path) = geometry else {
        return false;
    };
    let mut points = Vec::new();
    let query = GraphicPoint::new(x, y);
    for command in path {
        match *command {
            PathCommand::Move(point) => {
                if dash_run_hit(&points, false, stroke, query) {
                    return true;
                }
                points.clear();
                points.push(point);
            }
            PathCommand::Line(point) => {
                if points.last() != Some(&point) {
                    points.push(point);
                }
            }
            PathCommand::Close => {
                if points.len() > 1 && points.last() == points.first() {
                    points.pop();
                }
                if dash_run_hit(&points, true, stroke, query) {
                    return true;
                }
                points.clear();
            }
            PathCommand::Cubic { .. } => return false, /* Only adaptive dash centerlines reach
                                                        * this helper. */
        }
    }
    dash_run_hit(&points, false, stroke, query)
}
pub(crate) fn contains(mark: &Mark, x: f64, y: f64) -> bool {
    if mark.clip_path.as_ref().is_some_and(|c| !c.contains(x, y)) {
        return false;
    }
    if mark.clip.is_some_and(|c| !c.contains(x, y)) {
        return false;
    }
    if let Some(dash) = &mark.stroke_dash {
        let mut filled = mark.clone();
        filled.stroke_dash = None;
        // Empty/all-zero arrays are the existing solid geometry, including its
        // legacy picking policy. Ordinary undashed picking remains untouched.
        if dash.normalized().is_ok_and(|dash| dash.segments.is_empty()) {
            return contains(&filled, x, y);
        }
        filled.clip_path = None;
        filled.clip = None; // The original point has already passed both clips.
        filled.stroke = None;
        if filled.has_fill() && contains(&filled, x, y) {
            return true;
        }
        let Some(stroke) = mark.stroke else {
            return false;
        };
        let Ok(detailed) = crate::dash_geometry_detailed(&mark.geometry, dash, &Default::default())
        else {
            return false;
        };
        filled.fill = None;
        filled.fill_paint = None;
        filled.stroke = Some(stroke);
        filled.geometry = detailed.stroke_geometry();
        if dashed_path_hit(&filled.geometry, stroke, x, y) {
            return true;
        }
        filled.stroke = None;
        filled.fill = Some(stroke.color);
        return detailed.dots.iter().any(|dot| {
            crate::dash_dot_geometry(dot, stroke.width, stroke.cap).is_ok_and(|g| {
                filled.geometry = g;
                contains(&filled, x, y)
            })
        });
    }
    let stroke = mark.stroke.map_or(0., |s| s.width / 2. + 2.);
    match &mark.geometry {
        Geometry::Rect {
            x: rx,
            y: ry,
            width: w,
            height: h,
        } => {
            let inside = x >= *rx && x <= rx + w && y >= *ry && y <= ry + h;
            (mark.has_fill() && *w > 0. && *h > 0. && inside)
                || (stroke > 0.
                    && x >= rx - stroke
                    && x <= rx + w + stroke
                    && y >= ry - stroke
                    && y <= ry + h + stroke
                    && (!inside
                        || x < rx + stroke
                        || x > rx + w - stroke
                        || y < ry + stroke
                        || y > ry + h - stroke))
        }
        Geometry::Circle {
            x: cx,
            y: cy,
            radius: r,
        } => {
            let distance = (x - cx).hypot(y - cy);
            (mark.has_fill() && *r > 0. && distance <= *r)
                || (stroke > 0. && (distance - r).abs() <= stroke)
        }
        Geometry::Path(commands) => {
            let mut winding = 0i32;
            let mut hit = false;
            let mut current = GraphicPoint::new(0., 0.);
            let mut start = current;
            let mut active = false;
            let mut segment = |a: GraphicPoint, b: GraphicPoint, paint_stroke: bool| {
                if a.y <= y {
                    if b.y > y && (b.x - a.x) * (y - a.y) - (x - a.x) * (b.y - a.y) > 0. {
                        winding += 1;
                    }
                } else if b.y <= y && (b.x - a.x) * (y - a.y) - (x - a.x) * (b.y - a.y) < 0. {
                    winding -= 1;
                }
                if paint_stroke && stroke > 0. {
                    let dx = b.x - a.x;
                    let dy = b.y - a.y;
                    let len = dx * dx + dy * dy;
                    let t = if len == 0. {
                        0.
                    } else {
                        ((x - a.x) * dx + (y - a.y) * dy) / len
                    }
                    .clamp(0., 1.);
                    hit |= (x - a.x - t * dx).hypot(y - a.y - t * dy) <= stroke;
                }
            };
            for cmd in commands {
                match *cmd {
                    PathCommand::Move(p) => {
                        if active {
                            segment(current, start, false);
                        }
                        current = p;
                        start = p;
                        active = true;
                    }
                    PathCommand::Line(p) => {
                        segment(current, p, true);
                        current = p;
                    }
                    PathCommand::Cubic { first, second, end } => {
                        let origin = current;
                        for i in 1..=24 {
                            let t = i as f64 / 24.;
                            let u = 1. - t;
                            let p = GraphicPoint::new(
                                u * u * u * origin.x
                                    + 3. * u * u * t * first.x
                                    + 3. * u * t * t * second.x
                                    + t * t * t * end.x,
                                u * u * u * origin.y
                                    + 3. * u * u * t * first.y
                                    + 3. * u * t * t * second.y
                                    + t * t * t * end.y,
                            );
                            segment(current, p, true);
                            current = p;
                        }
                    }
                    PathCommand::Close => {
                        segment(current, start, true);
                        current = start;
                        active = false;
                    }
                }
            }
            if active {
                segment(current, start, false);
            }
            hit || (mark.has_fill() && winding != 0)
        }
    }
}
