//! Authoritative Funnel SVG data, source-bound annotations and literal paint
//! occurrences. Bound annotations and authored Scene extras are separate layers.
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use crate::scene::ensure;
use crate::*;

pub const MAX_FUNNEL_SVG_LAYERS: usize = 128;
pub const MAX_FUNNEL_SVG_WORK: usize = 200_000;
pub const MAX_FUNNEL_SVG_SOURCE_COMMANDS: usize = 2_000_000;
pub const MAX_FUNNEL_SVG_COMPONENTS: usize = 128;
pub const MAX_FUNNEL_SVG_KEY_BYTES: usize = 128;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FunnelSvgMarkRole {
    Body { part_index: usize },
    Border { part_index: usize },
    Separator { separator_index: usize },
}

#[derive(Clone, Debug)]
struct Frame {
    config: ChartConfig,
    source: Vec<FunnelDatum>,
    options: FunnelLayoutOptions,
    layout: FunnelLayout,
    by_id: BTreeMap<String, usize>,
    bodies: Vec<Option<usize>>,
    borders: Vec<Option<usize>>,
    labels: Vec<Option<usize>>,
    separators: Vec<Option<usize>>,
    mark_roles: BTreeMap<usize, FunnelSvgMarkRole>,
    label_parts: Vec<Option<usize>>,
    part_items: Vec<DrawItem>,
    separator_items: Vec<DrawItem>,
    label_items: Vec<DrawItem>,
    extras: Vec<DrawItem>,
    annotations: Option<FunnelAnnotations>,
    annotation_items: Vec<DrawItem>,
    annotation_base: Option<FunnelSvgFrame>,
}

/// One actual constructor cohort, with no retained user functions. Equality is
/// capture identity, preserving raw numeric bits and absent-role style changes.
/// Constructor profile helpers and current Scene Mark getters are separate:
/// legal custom/edited ink does not rewrite the original four-point profile.
#[derive(Clone, Debug)]
pub struct FunnelSvgFrame(Arc<Frame>);
impl PartialEq for FunnelSvgFrame {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for FunnelSvgFrame {}

impl FunnelSvgFrame {
    /// Apply a borrowed geometry factory once for every source part, including
    /// collapsed parts without a body Mark. This additive pass preserves the
    /// existing resolved style/label results and never replays their factories.
    /// An inactive result is captured for motion only: it does not create ink,
    /// a logical site, or a body target. Existing stage constructors retain
    /// their original drawable-only callback contract.
    pub fn with_all_stage_factory(
        &self,
        factory: &mut impl FnMut(&FunnelStageContext<'_>) -> Result<FunnelStageGeometry>,
    ) -> Result<Self> {
        Self::from_layout(
            self.annotation_source()
                .layout()
                .all_stage_factory(factory)?,
        )
    }
    pub(crate) fn motion_capture(
        &self,
        index: usize,
    ) -> Result<&crate::funnel_layout::FunnelPartMotionCapture> {
        self.0.layout.motion_capture(index)
    }
    pub fn new(
        config: &ChartConfig,
        source: &[FunnelDatum],
        options: &FunnelLayoutOptions,
    ) -> Result<Self> {
        Self::from_layout(funnel_layout(config, source, options)?)
    }
    pub fn with_factories(
        config: &ChartConfig,
        source: &[FunnelDatum],
        options: &FunnelLayoutOptions,
        factories: &mut FunnelFactories<'_>,
    ) -> Result<Self> {
        Self::from_layout(funnel_layout_with_factories(
            config, source, options, factories,
        )?)
    }
    pub fn with_size(
        config: &ChartConfig,
        source: &[FunnelDatum],
        options: &FunnelLayoutOptions,
        size: impl FnMut(FunnelSizeContext<'_>) -> Result<f64>,
    ) -> Result<Self> {
        Self::from_layout(funnel_layout_with_size(config, source, options, size)?)
    }
    pub fn with_size_and_factories(
        config: &ChartConfig,
        source: &[FunnelDatum],
        options: &FunnelLayoutOptions,
        size: impl FnMut(FunnelSizeContext<'_>) -> Result<f64>,
        factories: &mut FunnelFactories<'_>,
    ) -> Result<Self> {
        Self::from_layout(funnel_layout_with_size_and_factories(
            config, source, options, size, factories,
        )?)
    }
    pub fn with_stage_factory(
        config: &ChartConfig,
        source: &[FunnelDatum],
        options: &FunnelLayoutOptions,
        stage: &mut impl FnMut(&FunnelStageContext<'_>) -> Result<FunnelStageGeometry>,
    ) -> Result<Self> {
        Self::from_layout(funnel_layout_with_stage_factory(
            config, source, options, stage,
        )?)
    }
    pub fn with_factories_and_stage_factory(
        config: &ChartConfig,
        source: &[FunnelDatum],
        options: &FunnelLayoutOptions,
        factories: &mut FunnelFactories<'_>,
        stage: &mut impl FnMut(&FunnelStageContext<'_>) -> Result<FunnelStageGeometry>,
    ) -> Result<Self> {
        Self::from_layout(funnel_layout_with_factories_and_stage_factory(
            config, source, options, factories, stage,
        )?)
    }
    pub fn with_size_factories_and_stage_factory(
        config: &ChartConfig,
        source: &[FunnelDatum],
        options: &FunnelLayoutOptions,
        size: impl FnMut(FunnelSizeContext<'_>) -> Result<f64>,
        factories: &mut FunnelFactories<'_>,
        stage: &mut impl FnMut(&FunnelStageContext<'_>) -> Result<FunnelStageGeometry>,
    ) -> Result<Self> {
        Self::from_layout(funnel_layout_with_size_factories_and_stage_factory(
            config, source, options, size, factories, stage,
        )?)
    }
    /// Consume a layout bearing a real constructor's private provenance. This
    /// does not infer config/options from geometry or replay any function.
    /// Current Scene decoration/paint may differ; original source/part metadata
    /// must still match that constructor (or its real stage-factory result).
    pub fn from_layout(layout: FunnelLayout) -> Result<Self> {
        let (config, source, options) = layout.svg_source()?;
        layout.clone().into_scene_interaction()?;
        let marks: BTreeMap<_, _> = layout
            .scene
            .marks
            .iter()
            .enumerate()
            .map(|(i, m)| (m.id.as_str(), i))
            .collect();
        let mut by_id = BTreeMap::new();
        let mut mark_roles = BTreeMap::new();
        let mut bodies = Vec::with_capacity(source.len());
        let mut borders = Vec::with_capacity(source.len());
        let mut series = BTreeMap::new();
        for (index, part) in layout.parts.iter().enumerate() {
            ensure(
                part.source_index == index,
                "Funnel SVG source index changed",
            )?;
            ensure(
                by_id.insert(part.datum.id.identity()?, index).is_none(),
                "duplicate Funnel SVG source",
            )?;
            series.insert(part.series.as_str(), index);
            let body = marks.get(part.mark_id.as_str()).copied();
            if let Some(i) = body {
                let m = &layout.scene.marks[i];
                ensure(
                    m.series == part.series && m.value.to_bits() == part.datum.value.to_bits(),
                    "Funnel SVG body identity changed",
                )?;
                ensure(
                    mark_roles
                        .insert(i, FunnelSvgMarkRole::Body { part_index: index })
                        .is_none(),
                    "ambiguous Funnel SVG body",
                )?;
            }
            bodies.push(body);
            let border = marks
                .get(format!("funnel:border:{}", part.datum.id.identity()?).as_str())
                .copied();
            if let Some(i) = border {
                let mark = &layout.scene.marks[i];
                ensure(
                    mark.series == part.series && mark.label.is_empty() && mark.value == 0.,
                    "Funnel SVG border decorative identity changed",
                )?;
                ensure(
                    mark_roles
                        .insert(i, FunnelSvgMarkRole::Border { part_index: index })
                        .is_none(),
                    "ambiguous Funnel SVG border",
                )?;
            }
            borders.push(border);
        }
        let mut separators = Vec::with_capacity(layout.separators.len());
        for (index, s) in layout.separators.iter().enumerate() {
            let side = match s.side {
                FunnelSeparatorSide::Before => "before",
                FunnelSeparatorSide::After => "after",
            };
            let id = if let Some(id) = &s.part_id {
                format!("funnel:separator:{side}:stage:{}", id.identity()?)
            } else {
                format!("funnel:separator:{side}:tail")
            };
            let mark = marks.get(id.as_str()).copied();
            if let Some(i) = mark {
                let mark = &layout.scene.marks[i];
                ensure(
                    mark.series.is_empty() && mark.label.is_empty() && mark.value == 0.,
                    "Funnel SVG separator decorative identity changed",
                )?;
                ensure(
                    mark_roles
                        .insert(
                            i,
                            FunnelSvgMarkRole::Separator {
                                separator_index: index,
                            },
                        )
                        .is_none(),
                    "ambiguous Funnel SVG separator",
                )?;
            }
            separators.push(mark);
        }
        for (i, mark) in layout.scene.marks.iter().enumerate() {
            ensure(
                mark_roles.contains_key(&i)
                    || !(mark.id.starts_with("funnel:stage:")
                        || mark.id.starts_with("funnel:border:")
                        || mark.id.starts_with("funnel:separator:")),
                "orphan managed Funnel SVG Mark",
            )?;
        }
        let mut labels = vec![None; source.len()];
        let mut label_parts = vec![None; layout.scene.labels.len()];
        for (&index, owner) in &layout.scene.label_series {
            let part = *series
                .get(owner.as_str())
                .ok_or_else(|| ChartError("Funnel SVG label owner is not a part".into()))?;
            ensure(
                labels[part].replace(index).is_none(),
                "duplicate Funnel SVG part label",
            )?;
            label_parts[index] = Some(part);
        }
        let mut part_items = Vec::new();
        let mut separator_items = Vec::new();
        let mut label_items = Vec::new();
        let mut extras = Vec::new();
        for item in layout.scene.draw_items() {
            match item {
                DrawItem::Mark(index) => match mark_roles.get(&index) {
                    Some(FunnelSvgMarkRole::Separator { .. }) => separator_items.push(item),
                    Some(_) => part_items.push(item),
                    None => extras.push(item),
                },
                DrawItem::Label(index) => {
                    if label_parts[index].is_some() {
                        label_items.push(item)
                    } else {
                        extras.push(item)
                    }
                }
            }
        }
        Ok(Self(Arc::new(Frame {
            config,
            source,
            options,
            layout,
            by_id,
            bodies,
            borders,
            labels,
            separators,
            mark_roles,
            label_parts,
            part_items,
            separator_items,
            label_items,
            extras,
            annotations: None,
            annotation_items: Vec::new(),
            annotation_base: None,
        })))
    }
    /// The constructor cohort before managed bound-annotation paint. Rebinding
    /// replaces this layer rather than accumulating prior annotations or user
    /// closures, and always uses the original profile's center and dimensions.
    pub fn annotation_source(&self) -> &Self {
        self.0.annotation_base.as_ref().unwrap_or(self)
    }
    pub fn annotations(&self) -> Option<&FunnelAnnotations> {
        self.0.annotations.as_ref()
    }
    pub fn with_annotations(
        &self,
        specs: &[FunnelAnnotationSpec],
        factories: &mut FunnelAnnotationFactories<'_>,
        style: &FunnelAnnotationStyle,
    ) -> Result<Self> {
        self.with_annotations_with_budget(
            specs,
            factories,
            style,
            FunnelAnnotationBudget::default(),
        )
    }
    pub fn with_annotations_with_budget(
        &self,
        specs: &[FunnelAnnotationSpec],
        factories: &mut FunnelAnnotationFactories<'_>,
        style: &FunnelAnnotationStyle,
        budget: FunnelAnnotationBudget,
    ) -> Result<Self> {
        let bundle = funnel_annotations_with_budget(
            self.annotation_source(),
            specs,
            factories,
            style,
            budget,
        )?;
        self.with_annotation_bundle(bundle)
    }
    /// Adopt a precomputed cohort without replaying match or note factories.
    /// A bundle from a different constructor capture is rejected, even when its
    /// raw values compare equal, so stale annotations cannot attach to new data.
    pub fn with_annotation_bundle(&self, annotations: FunnelAnnotations) -> Result<Self> {
        let base = self.annotation_source();
        ensure(
            annotations.source() == base,
            "Funnel annotations belong to a different source capture",
        )?;
        let paint = annotations.scene();
        paint.validate()?;
        ensure(
            paint.width == base.scene().width && paint.height == base.scene().height,
            "Funnel annotation viewport differs from its source",
        )?;
        ensure(
            paint.marks.iter().all(|m| m.series.is_empty()) && paint.label_series.is_empty(),
            "Funnel annotation paint cannot own business sites",
        )?;
        let mut next = base.0.as_ref().clone();
        let mark_start = next.layout.scene.marks.len();
        let label_start = next.layout.scene.labels.len();
        let remap = |item| match item {
            DrawItem::Mark(i) => DrawItem::Mark(i + mark_start),
            DrawItem::Label(i) => DrawItem::Label(i + label_start),
        };
        next.annotation_items = paint.draw_items().into_iter().map(remap).collect();
        let mut order = next.layout.scene.draw_items();
        order.extend(next.annotation_items.iter().copied());
        let scene = &mut next.layout.scene;
        scene.marks.extend(paint.marks.iter().cloned());
        scene
            .opacity_groups
            .extend(paint.opacity_groups.iter().cloned());
        scene.labels.extend(paint.labels.iter().cloned());
        for (i, _) in paint.labels.iter().enumerate() {
            let font = paint.typography.label_options(i);
            if !font.is_empty() {
                scene.typography.labels.insert(label_start + i, font);
            }
        }
        for (&i, mode) in &paint.typography.label_whitespace {
            scene
                .typography
                .label_whitespace
                .insert(label_start + i, *mode);
        }
        for (&i, stroke) in &paint.typography.label_strokes {
            scene
                .typography
                .label_strokes
                .insert(label_start + i, *stroke);
        }
        for (&i, clip) in &paint.label_clips {
            scene.label_clips.insert(label_start + i, clip.clone());
        }
        scene.draw_order = order;
        scene.validate()?;
        next.label_parts.resize(scene.labels.len(), None);
        next.annotation_base = Some(base.clone());
        next.annotations = Some(annotations);
        Ok(Self(Arc::new(next)))
    }
    pub fn config(&self) -> &ChartConfig {
        &self.0.config
    }
    pub fn source(&self) -> &[FunnelDatum] {
        &self.0.source
    }
    pub fn options(&self) -> &FunnelLayoutOptions {
        &self.0.options
    }
    pub fn layout(&self) -> &FunnelLayout {
        &self.0.layout
    }
    pub fn scene(&self) -> &Scene {
        &self.0.layout.scene
    }
    pub fn styles(&self) -> &[FunnelComputedPartStyle] {
        self.0.layout.computed_part_styles()
    }
    pub fn parts(&self) -> &[FunnelPart] {
        &self.0.layout.parts
    }
    pub fn separators(&self) -> &[FunnelSeparator] {
        &self.0.layout.separators
    }
    pub fn plot(&self) -> ClipRect {
        self.0.layout.plot()
    }
    pub fn inner_size(&self) -> [f64; 2] {
        [self.plot().width, self.plot().height]
    }
    pub fn inner_position(&self, p: [f64; 2]) -> Result<[f64; 2]> {
        ensure(
            p.iter().all(|v| v.is_finite()),
            "invalid Funnel SVG position",
        )?;
        let p = [p[0] - self.plot().x, p[1] - self.plot().y];
        ensure(
            p.iter().all(|v| v.is_finite()),
            "Funnel SVG position overflow",
        )?;
        Ok(p)
    }
    pub fn part_by_id(&self, id: &CategoryKey) -> Result<Option<usize>> {
        Ok(self.0.by_id.get(&id.identity()?).copied())
    }
    pub fn mark_role(&self, index: usize) -> Result<Option<FunnelSvgMarkRole>> {
        ensure(
            index < self.scene().marks.len(),
            "Funnel SVG Mark index is missing",
        )?;
        Ok(self.0.mark_roles.get(&index).copied())
    }
    pub fn label_part_index(&self, index: usize) -> Result<Option<usize>> {
        self.0
            .label_parts
            .get(index)
            .copied()
            .ok_or_else(|| ChartError("Funnel SVG label index is missing".into()))
    }
    pub fn body_mark(&self, index: usize) -> Result<Option<&Mark>> {
        Ok(self
            .0
            .bodies
            .get(index)
            .ok_or_else(|| ChartError("Funnel SVG part index is missing".into()))?
            .map(|i| &self.scene().marks[i]))
    }
    pub fn border_mark(&self, index: usize) -> Result<Option<&Mark>> {
        Ok(self
            .0
            .borders
            .get(index)
            .ok_or_else(|| ChartError("Funnel SVG part index is missing".into()))?
            .map(|i| &self.scene().marks[i]))
    }
    pub fn label(&self, index: usize) -> Result<Option<&Label>> {
        Ok(self
            .0
            .labels
            .get(index)
            .ok_or_else(|| ChartError("Funnel SVG part index is missing".into()))?
            .map(|i| &self.scene().labels[i]))
    }
    pub fn separator_mark(&self, index: usize) -> Result<Option<&Mark>> {
        Ok(self
            .0
            .separators
            .get(index)
            .ok_or_else(|| ChartError("Funnel SVG separator index is missing".into()))?
            .map(|i| &self.scene().marks[i]))
    }
    pub fn original_body_geometry(&self, index: usize) -> Result<Option<Geometry>> {
        funnel_part_geometry(
            self.parts()
                .get(index)
                .ok_or_else(|| ChartError("Funnel SVG part index is missing".into()))?,
            self.options().interpolation,
        )
    }
    pub fn original_border_geometry(&self, index: usize) -> Result<Geometry> {
        funnel_part_border_geometry(
            self.parts()
                .get(index)
                .ok_or_else(|| ChartError("Funnel SVG part index is missing".into()))?,
            self.options().interpolation,
        )
    }
    pub fn extras(&self) -> &[DrawItem] {
        &self.0.extras
    }
    fn items(&self, kind: &FunnelSvgLayerKind) -> &[DrawItem] {
        match kind {
            FunnelSvgLayerKind::Parts => &self.0.part_items,
            FunnelSvgLayerKind::Separators => &self.0.separator_items,
            FunnelSvgLayerKind::Labels => &self.0.label_items,
            FunnelSvgLayerKind::Annotations => &self.0.annotation_items,
            FunnelSvgLayerKind::SceneExtras => &self.0.extras,
            FunnelSvgLayerKind::Custom(_) => &[],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FunnelSvgLayerKind {
    Separators,
    Parts,
    Labels,
    Annotations,
    SceneExtras,
    Custom(String),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunnelSvgLayer {
    pub key: String,
    pub kind: FunnelSvgLayerKind,
}
impl FunnelSvgLayer {
    pub fn new(key: impl Into<String>, kind: FunnelSvgLayerKind) -> Result<Self> {
        let layer = Self {
            key: key.into(),
            kind,
        };
        layer.validate()?;
        Ok(layer)
    }
    fn validate(&self) -> Result<()> {
        let valid = |s: &str| !s.trim().is_empty() && s.len() <= MAX_FUNNEL_SVG_KEY_BYTES;
        ensure(valid(&self.key), "invalid Funnel SVG occurrence key")?;
        if let FunnelSvgLayerKind::Custom(key) = &self.kind {
            ensure(valid(key), "invalid Funnel SVG registry key")?;
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FunnelSvgExtrasPolicy {
    #[default]
    RequireLayer,
    Omit,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FunnelSvgBudget {
    pub layers: usize,
    pub work: usize,
    pub source_commands: usize,
    pub components: usize,
}
impl Default for FunnelSvgBudget {
    fn default() -> Self {
        Self {
            layers: MAX_FUNNEL_SVG_LAYERS,
            work: MAX_FUNNEL_SVG_WORK,
            source_commands: MAX_FUNNEL_SVG_SOURCE_COMMANDS,
            components: MAX_FUNNEL_SVG_COMPONENTS,
        }
    }
}
impl FunnelSvgBudget {
    pub fn validate(self) -> Result<()> {
        ensure(
            self.layers <= MAX_FUNNEL_SVG_LAYERS
                && self.work <= MAX_FUNNEL_SVG_WORK
                && self.source_commands <= MAX_FUNNEL_SVG_SOURCE_COMMANDS
                && self.components <= MAX_FUNNEL_SVG_COMPONENTS,
            "invalid Funnel SVG budget",
        )
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunnelSvgOccurrence {
    pub index: usize,
    pub layer: FunnelSvgLayer,
    items: Vec<DrawItem>,
}
impl FunnelSvgOccurrence {
    pub fn items(&self) -> &[DrawItem] {
        &self.items
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunnelSvgPlan {
    frame: FunnelSvgFrame,
    layers: Vec<FunnelSvgOccurrence>,
    budget: FunnelSvgBudget,
    work: usize,
    visible_bodies: BTreeSet<usize>,
}
fn geometry_cost(g: &Geometry) -> usize {
    match g {
        Geometry::Path(p) => p.len(),
        Geometry::Circle { .. } => 6,
        Geometry::Rect { .. } => 5,
    }
}
impl FunnelSvgPlan {
    pub fn default_layers() -> Vec<FunnelSvgLayer> {
        [
            ("separators", FunnelSvgLayerKind::Separators),
            ("parts", FunnelSvgLayerKind::Parts),
            ("labels", FunnelSvgLayerKind::Labels),
            ("annotations", FunnelSvgLayerKind::Annotations),
            ("scene-extras", FunnelSvgLayerKind::SceneExtras),
        ]
        .into_iter()
        .map(|(key, kind)| FunnelSvgLayer {
            key: key.into(),
            kind,
        })
        .collect()
    }
    pub fn defaults(frame: &FunnelSvgFrame) -> Result<Self> {
        Self::new(
            frame,
            Self::default_layers(),
            FunnelSvgBudget::default(),
            FunnelSvgExtrasPolicy::RequireLayer,
        )
    }
    pub fn validate_layers(layers: &[FunnelSvgLayer], budget: FunnelSvgBudget) -> Result<()> {
        budget.validate()?;
        ensure(
            layers.len() <= budget.layers,
            "Funnel SVG layer budget exceeded",
        )?;
        let mut keys = BTreeSet::new();
        let mut custom = 0;
        for layer in layers {
            layer.validate()?;
            ensure(
                keys.insert(&layer.key),
                "duplicate Funnel SVG occurrence key",
            )?;
            if matches!(layer.kind, FunnelSvgLayerKind::Custom(_)) {
                custom += 1;
            }
        }
        ensure(
            custom <= budget.components,
            "Funnel SVG custom layer budget exceeded",
        )?;
        ensure(
            custom <= budget.work,
            "Funnel SVG custom work budget exceeded",
        )
    }
    pub fn new(
        frame: &FunnelSvgFrame,
        layers: Vec<FunnelSvgLayer>,
        budget: FunnelSvgBudget,
        extras: FunnelSvgExtrasPolicy,
    ) -> Result<Self> {
        Self::validate_layers(&layers, budget)?;
        ensure(
            frame.extras().is_empty()
                || extras == FunnelSvgExtrasPolicy::Omit
                || layers
                    .iter()
                    .any(|l| l.kind == FunnelSvgLayerKind::SceneExtras),
            "Funnel SVG must explicitly include or omit Scene extras",
        )?;
        let mut work = 0usize;
        let mut commands = 0usize;
        let mut occurrences = Vec::with_capacity(layers.len());
        let mut visible_bodies = BTreeSet::new();
        for (index, layer) in layers.into_iter().enumerate() {
            let items = frame.items(&layer.kind).to_vec();
            let positions: BTreeMap<_, _> = items
                .iter()
                .enumerate()
                .filter_map(|(i, item)| {
                    if let DrawItem::Mark(m) = item {
                        Some((frame.scene().marks[*m].id.as_str(), i))
                    } else {
                        None
                    }
                })
                .collect();
            for group in &frame.scene().opacity_groups {
                let found: Vec<_> = group
                    .members
                    .iter()
                    .filter_map(|id| positions.get(id.as_str()).copied())
                    .collect();
                ensure(
                    found.is_empty()
                        || (found.len() == group.members.len()
                            && found.windows(2).all(|p| p[1] == p[0] + 1)),
                    "Funnel SVG occurrence splits or reorders an opacity group",
                )?;
            }
            work = work
                .checked_add(if matches!(layer.kind, FunnelSvgLayerKind::Custom(_)) {
                    1
                } else {
                    items.len()
                })
                .ok_or_else(|| ChartError("Funnel SVG work overflow".into()))?;
            ensure(work <= budget.work, "Funnel SVG work budget exceeded")?;
            for item in &items {
                let cost = match item {
                    DrawItem::Mark(i) => {
                        let mark = &frame.scene().marks[*i];
                        let mut n = geometry_cost(&mark.geometry);
                        for extra in [
                            usize::from(mark.clip.is_some()) * 5,
                            mark.clip_path
                                .as_ref()
                                .map_or(0, |c| geometry_cost(&c.geometry)),
                            mark.stroke_dash.as_ref().map_or(0, |d| d.segments.len()),
                            mark.fill_paint
                                .as_ref()
                                .map_or(0, |p| p.stops().len().max(1)),
                        ] {
                            n = n
                                .checked_add(extra)
                                .ok_or_else(|| ChartError("Funnel SVG resource overflow".into()))?;
                        }
                        if matches!(
                            frame.0.mark_roles.get(i),
                            Some(FunnelSvgMarkRole::Body { .. })
                        ) {
                            visible_bodies.insert(*i);
                        }
                        n
                    }
                    DrawItem::Label(i) => {
                        1 + frame
                            .scene()
                            .label_clips
                            .get(i)
                            .map_or(0, |c| geometry_cost(&c.geometry))
                    }
                };
                commands = commands
                    .checked_add(cost)
                    .ok_or_else(|| ChartError("Funnel SVG command overflow".into()))?;
                ensure(
                    commands <= budget.source_commands,
                    "Funnel SVG source command budget exceeded",
                )?;
            }
            occurrences.push(FunnelSvgOccurrence {
                index,
                layer,
                items,
            });
        }
        Ok(Self {
            frame: frame.clone(),
            layers: occurrences,
            budget,
            work,
            visible_bodies,
        })
    }
    pub fn frame(&self) -> &FunnelSvgFrame {
        &self.frame
    }
    pub fn layers(&self) -> &[FunnelSvgOccurrence] {
        &self.layers
    }
    pub fn budget(&self) -> FunnelSvgBudget {
        self.budget
    }
    pub fn work(&self) -> usize {
        self.work
    }
    /// Actual original body Mark indices referenced by the literal union.
    /// This is a source projection, not an alpha/clip/occlusion oracle. Rich
    /// paint and zero-opacity native SVG path hit behavior remain renderer-owned.
    pub fn body_mark_indices(&self) -> &BTreeSet<usize> {
        &self.visible_bodies
    }
    pub fn mark_indices(&self) -> Vec<usize> {
        self.layers
            .iter()
            .flat_map(|o| o.items())
            .filter_map(|item| {
                if let DrawItem::Mark(i) = item {
                    Some(*i)
                } else {
                    None
                }
            })
            .collect()
    }
}
