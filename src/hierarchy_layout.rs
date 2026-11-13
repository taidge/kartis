//! Typed hierarchical presentation and data-space focus. Sources inspected:
//! Nivo treemap/src/hooks.ts (inner/outer and parent-label reservation),
//! circle-packing/src/hooks.ts::useCirclePackingZoom, icicle/src/hooks.ts.
//! Existing radial algorithms remain authoritative; no Scene camera is used.
use crate::{
    radial::{self, Hierarchy, Rect},
    scene::ensure,
    *,
};
use glory::render::GraphicStroke;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

mod labels;
pub use labels::{
    HierarchyLabelCandidate, HierarchyLabelPlacementOptions, HierarchyMeasuredLabel,
    HierarchyPlacedLabel,
};

#[derive(Clone, Debug)]
pub(crate) struct HierarchyModel {
    pub(crate) data: Vec<HierarchyDatum>,
    pub(crate) hierarchy: Option<Hierarchy>,
    pub(crate) index: BTreeMap<String, usize>,
}
impl HierarchyModel {
    pub(crate) fn new(data: &[HierarchyDatum]) -> Result<Self> {
        let mut hierarchy = radial::hierarchy(data)?;
        if let Some(h) = &mut hierarchy {
            for children in &mut h.children {
                children.sort_by(|a, b| data[*a].id.cmp(&data[*b].id));
            }
            let mut queue = VecDeque::from([h.root]);
            h.order.clear();
            while let Some(i) = queue.pop_front() {
                h.order.push(i);
                queue.extend(h.children[i].iter().copied());
            }
        }
        Ok(Self {
            data: data.to_vec(),
            hierarchy,
            index: data
                .iter()
                .enumerate()
                .map(|(i, d)| (d.id.clone(), i))
                .collect(),
        })
    }
    pub(crate) fn root(&self) -> Option<&str> {
        self.hierarchy
            .as_ref()
            .map(|h| self.data[h.root].id.as_str())
    }
    pub(crate) fn focus(&self, id: Option<&str>) -> Result<Option<usize>> {
        match (id, &self.hierarchy) {
            (Some(id), _) => self
                .index
                .get(id)
                .copied()
                .map(Some)
                .ok_or_else(|| ChartError("unknown hierarchy focus ID".into())),
            (None, Some(h)) => Ok(Some(h.root)),
            (None, None) => Ok(None),
        }
    }
    pub(crate) fn branch(&self, focus: usize) -> Vec<usize> {
        let h = self.hierarchy.as_ref().unwrap();
        let mut queue = VecDeque::from([focus]);
        let mut out = vec![];
        while let Some(i) = queue.pop_front() {
            out.push(i);
            queue.extend(h.children[i].iter().copied());
        }
        out
    }
    pub(crate) fn ancestors(&self, id: &str) -> Result<Vec<String>> {
        let mut i = *self
            .index
            .get(id)
            .ok_or_else(|| ChartError("unknown hierarchy node".into()))?;
        let h = self.hierarchy.as_ref().unwrap();
        let mut result = vec![];
        while let Some(p) = h.parent[i] {
            result.push(self.data[p].id.clone());
            i = p;
        }
        result.reverse();
        Ok(result)
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HierarchyFill {
    #[default]
    Palette,
    Fixed(Color),
    None,
}
#[derive(Clone, Debug, PartialEq)]
pub struct HierarchyNodeStyle {
    pub fill: HierarchyFill,
    pub paint: Option<FillPaint>,
    pub border: Option<GraphicStroke>,
    pub opacity: f64,
}
impl Default for HierarchyNodeStyle {
    fn default() -> Self {
        Self {
            fill: Default::default(),
            paint: None,
            border: None,
            opacity: 1.,
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub enum HierarchyLabelText {
    #[default]
    Label,
    Id,
    Value(ValueFormat),
}
#[derive(Clone, Debug, PartialEq)]
pub struct HierarchyLabelStyle {
    pub enabled: bool,
    pub text: HierarchyLabelText,
    pub color: Option<Color>,
    pub size: Option<f64>,
    pub rotation: f64,
    pub offset: [f64; 2],
    pub skip_width: f64,
    pub skip_height: f64,
    pub skip_radius: f64,
    pub orient_to_rect: bool,
    pub clip_to_node: bool,
}
impl Default for HierarchyLabelStyle {
    fn default() -> Self {
        Self {
            enabled: true,
            text: Default::default(),
            color: None,
            size: None,
            rotation: 0.,
            offset: [0., 0.],
            skip_width: 0.,
            skip_height: 0.,
            skip_radius: 0.,
            orient_to_rect: false,
            clip_to_node: true,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct HierarchyStyleOptions {
    pub leaf: HierarchyNodeStyle,
    pub parent: HierarchyNodeStyle,
    /// Complete per-ID node styles. Unused keys may survive dynamic datasets.
    pub nodes: BTreeMap<String, HierarchyNodeStyle>,
    pub leaf_labels: HierarchyLabelStyle,
    pub parent_labels: HierarchyLabelStyle,
    /// Exact node ink and its labels only; children retain their allocation.
    pub hidden: BTreeSet<String>,
    pub leaves_only: bool,
    pub include_focus: bool,
    pub clip_plot: bool,
    pub value_format: ValueFormat,
}
impl Default for HierarchyStyleOptions {
    fn default() -> Self {
        Self {
            leaf: Default::default(),
            parent: Default::default(),
            nodes: Default::default(),
            leaf_labels: Default::default(),
            parent_labels: HierarchyLabelStyle {
                enabled: false,
                ..Default::default()
            },
            hidden: Default::default(),
            leaves_only: false,
            include_focus: true,
            clip_plot: true,
            value_format: ValueFormat::Auto,
        }
    }
}
fn bounded(v: f64) -> bool {
    v.is_finite() && (0. ..=16384.).contains(&v)
}
impl HierarchyStyleOptions {
    fn validate(&self) -> Result<()> {
        self.value_format.validate()?;
        ensure(
            self.nodes.len() <= 8000
                && self.hidden.len() <= 8000
                && self
                    .nodes
                    .keys()
                    .chain(self.hidden.iter())
                    .all(|id| !id.trim().is_empty() && id.len() <= 1000),
            "hierarchy style identity budget exceeded",
        )?;
        for s in std::iter::once(&self.leaf)
            .chain(std::iter::once(&self.parent))
            .chain(self.nodes.values())
        {
            ensure(
                s.opacity.is_finite()
                    && (0. ..=1.).contains(&s.opacity)
                    && s.border.is_none_or(|b| {
                        b.width.is_finite() && (0.000001..=10000.).contains(&b.width)
                    }),
                "invalid hierarchy node style",
            )?;
            if let Some(p) = &s.paint {
                p.validate()?;
            }
        }
        for l in [&self.leaf_labels, &self.parent_labels] {
            if let HierarchyLabelText::Value(f) = &l.text {
                f.validate()?;
            }
            ensure(
                l.size
                    .is_none_or(|s| s.is_finite() && (1. ..=256.).contains(&s))
                    && l.rotation.is_finite()
                    && (-360. ..=360.).contains(&l.rotation)
                    && l.offset.iter().all(|v| v.is_finite() && v.abs() <= 16384.)
                    && [l.skip_width, l.skip_height, l.skip_radius]
                        .into_iter()
                        .all(bounded),
                "invalid hierarchy label style",
            )?;
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HierarchyTreemapTile {
    Static(TreemapTile),
    Resquarify,
}
impl Default for HierarchyTreemapTile {
    fn default() -> Self {
        Self::Static(TreemapTile::Squarify)
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TreemapParentLabelPosition {
    #[default]
    Top,
    Right,
    Bottom,
    Left,
}
#[derive(Clone, Debug, PartialEq)]
pub struct TreemapLayoutOptions {
    pub tile: HierarchyTreemapTile,
    pub ratio: f64,
    pub inner_padding: f64,
    pub outer_padding: f64,
    pub parent_label_size: f64,
    pub parent_label_padding: f64,
    pub parent_label_position: TreemapParentLabelPosition,
    pub style: HierarchyStyleOptions,
}
impl Default for TreemapLayoutOptions {
    fn default() -> Self {
        let mut style = HierarchyStyleOptions::default();
        style.parent_labels.enabled = true;
        Self {
            tile: Default::default(),
            ratio: (1. + 5_f64.sqrt()) / 2.,
            inner_padding: 2.,
            outer_padding: 3.,
            parent_label_size: 20.,
            parent_label_padding: 4.,
            parent_label_position: Default::default(),
            style,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct CirclePackingLayoutOptions {
    pub algorithm: CirclePackingAlgorithm,
    pub padding: f64,
    pub viewport_padding: f64,
    pub style: HierarchyStyleOptions,
}
impl Default for CirclePackingLayoutOptions {
    fn default() -> Self {
        let mut style = HierarchyStyleOptions::default();
        style.parent.opacity = 0.12;
        Self {
            algorithm: CirclePackingAlgorithm::FrontChain,
            padding: 2.,
            viewport_padding: 2.,
            style,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum IcicleDirection {
    #[default]
    Down,
    Up,
    Right,
    Left,
}
#[derive(Clone, Debug, PartialEq)]
pub struct IcicleLayoutOptions {
    pub direction: IcicleDirection,
    pub padding: f64,
    pub style: HierarchyStyleOptions,
}
impl Default for IcicleLayoutOptions {
    fn default() -> Self {
        let mut style = HierarchyStyleOptions::default();
        style.parent_labels.enabled = true;
        Self {
            direction: Default::default(),
            padding: 2.,
            style,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum HierarchyView {
    Treemap(TreemapLayoutOptions),
    CirclePacking(CirclePackingLayoutOptions),
    Icicle(IcicleLayoutOptions),
}
#[derive(Clone, Debug, PartialEq)]
pub struct HierarchyLayoutNode {
    pub id: String,
    pub label: String,
    pub parent: Option<String>,
    pub children: Vec<String>,
    pub depth: usize,
    pub relative_depth: Option<usize>,
    pub value: f64,
    pub leaf: bool,
    pub geometry: Option<Geometry>,
    pub content_rect: Option<ClipRect>,
    pub visible: bool,
    pub mark_id: String,
    pub label_slots: Vec<usize>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct HierarchyLayout {
    pub scene: Scene,
    pub root_id: Option<String>,
    pub focus_id: Option<String>,
    pub focus_ancestors: Vec<String>,
    pub nodes: Vec<HierarchyLayoutNode>,
    pub resquarify_stats: Option<ResquarifyStats>,
    by_id: BTreeMap<String, usize>,
    by_mark: BTreeMap<String, usize>,
    plot: ClipRect,
    value_format: ValueFormat,
    label_layout: labels::HierarchyLabelState,
}
impl HierarchyLayout {
    pub fn plot(&self) -> ClipRect {
        self.plot
    }
    /// Visible hierarchy marks with their business values and layout anchors.
    pub fn interaction_binding(&self) -> Result<InteractionBinding> {
        let mut binding = InteractionBinding::default();
        for mark in &self.scene.marks {
            let Some(node) = self.node_for_mark(&mark.id).filter(|node| node.visible) else {
                continue;
            };
            if !mark.has_fill() && mark.stroke.is_none() {
                continue;
            }
            let Some(position) = crate::crosshair::mark_center(mark) else {
                return Err(ChartError(
                    "unsupported hierarchy interaction geometry".into(),
                ));
            };
            binding.sites.push(InteractionSite {
                mark_id: mark.id.clone(),
                series: node.id.clone(),
                label: if node.label.is_empty() {
                    node.id.clone()
                } else {
                    node.label.clone()
                },
                value: Some(node.value),
                formatted_value: Some(self.value_format.format(node.value)?),
                color: mark
                    .fill
                    .or_else(|| mark.stroke.map(|stroke| stroke.color))
                    .unwrap_or(Color(0, 0, 0, 0)),
                position,
            });
        }
        binding.validate(&self.scene)?;
        Ok(binding)
    }
    pub fn into_scene_interaction(self) -> Result<SceneInteraction> {
        Ok(SceneInteraction {
            interaction: Some(self.interaction_binding()?),
            scene: self.scene,
            plot: Some(self.plot),
        })
    }
    pub fn node(&self, id: &str) -> Option<&HierarchyLayoutNode> {
        self.by_id
            .get(id)
            .and_then(|i| self.nodes.get(*i))
            .filter(|n| n.id == id)
    }
    pub fn node_for_mark(&self, id: &str) -> Option<&HierarchyLayoutNode> {
        self.by_mark
            .get(id)
            .and_then(|i| self.nodes.get(*i))
            .filter(|n| n.mark_id == id)
    }
    pub fn hit_test(&self, x: f64, y: f64) -> Option<&HierarchyLayoutNode> {
        self.scene
            .hit_test(x, y)
            .and_then(|m| self.node_for_mark(&m.id))
    }
    /// Root-to-parent order; paths are queried rather than cloned per node.
    pub fn ancestors(&self, id: &str) -> Result<Vec<&HierarchyLayoutNode>> {
        let mut n = self
            .node(id)
            .ok_or_else(|| ChartError("unknown hierarchy node".into()))?;
        let mut out = vec![];
        let mut seen = BTreeSet::new();
        while let Some(parent) = &n.parent {
            ensure(
                out.len() < 128 && seen.insert(parent.as_str()),
                "invalid hierarchy ancestor metadata",
            )?;
            n = self
                .node(parent)
                .ok_or_else(|| ChartError("missing hierarchy ancestor metadata".into()))?;
            out.push(n);
        }
        out.reverse();
        Ok(out)
    }
    pub fn descendants(&self, id: &str, include_self: bool) -> Result<Vec<&HierarchyLayoutNode>> {
        let root = self
            .node(id)
            .ok_or_else(|| ChartError("unknown hierarchy node".into()))?;
        let mut queue = VecDeque::from([root]);
        let mut seen = BTreeSet::new();
        let mut out = vec![];
        while let Some(n) = queue.pop_front() {
            ensure(
                seen.insert(n.id.as_str()) && seen.len() <= 8000,
                "invalid hierarchy descendant metadata",
            )?;
            if include_self || n.id != id {
                out.push(n);
            }
            for child in &n.children {
                ensure(queue.len() < 8000, "hierarchy descendant budget exceeded")?;
                queue.push_back(
                    self.node(child)
                        .ok_or_else(|| ChartError("missing hierarchy child metadata".into()))?,
                );
            }
        }
        Ok(out)
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TreemapLayoutState {
    cache: Resquarify,
}
impl TreemapLayoutState {
    pub fn cached_parents(&self) -> usize {
        self.cache.cached_parents()
    }
    pub fn rows(&self, parent: &str) -> Option<&[TreemapRow]> {
        self.cache.rows(parent)
    }
    pub fn last_stats(&self) -> ResquarifyStats {
        self.cache.last_stats()
    }
    pub fn clear(&mut self) {
        self.cache.clear();
    }
    pub fn layout(
        &mut self,
        c: &ChartConfig,
        data: &[HierarchyDatum],
        focus: Option<&str>,
        o: &TreemapLayoutOptions,
    ) -> Result<HierarchyLayout> {
        let model = HierarchyModel::new(data)?;
        self.layout_model(c, &model, focus, o)
    }
    pub(crate) fn reconcile(&mut self, model: &HierarchyModel) {
        let Some(h) = &model.hierarchy else {
            self.clear();
            return;
        };
        self.cache.parents.retain(|id, rows| {
            let Some(&i) = model.index.get(id) else {
                return false;
            };
            let children = h.children[i]
                .iter()
                .map(|j| model.data[*j].id.as_str())
                .collect::<BTreeSet<_>>();
            rows.iter()
                .flat_map(|r| r.children.iter())
                .map(String::as_str)
                .collect::<BTreeSet<_>>()
                == children
                && rows.iter().map(|r| r.children.len()).sum::<usize>() == children.len()
                && !children.is_empty()
        });
    }
    pub(crate) fn layout_model(
        &mut self,
        c: &ChartConfig,
        model: &HierarchyModel,
        focus: Option<&str>,
        o: &TreemapLayoutOptions,
    ) -> Result<HierarchyLayout> {
        let mut candidate = self.clone();
        candidate.cache.set_ratio(o.ratio)?;
        candidate.reconcile(model);
        candidate.cache.stats = Default::default();
        let result = treemap_build(c, model, focus, o, &mut candidate)?;
        *self = candidate;
        Ok(result)
    }
}
pub fn treemap_layout(
    c: &ChartConfig,
    data: &[HierarchyDatum],
    focus: Option<&str>,
    o: &TreemapLayoutOptions,
) -> Result<HierarchyLayout> {
    ensure(
        o.tile != HierarchyTreemapTile::Resquarify,
        "Resquarify requires a retained TreemapLayoutState",
    )?;
    TreemapLayoutState::default().layout(c, data, focus, o)
}
fn initial(
    c: &ChartConfig,
    model: &HierarchyModel,
    focus: Option<&str>,
    style: &HierarchyStyleOptions,
) -> Result<(HierarchyLayout, Option<usize>, Vec<usize>)> {
    let scene = c.scene()?;
    style.validate()?;
    let focus = model.focus(focus)?;
    let branch = focus.map(|i| model.branch(i)).unwrap_or_default();
    let members = branch.iter().copied().collect::<BTreeSet<_>>();
    let mut nodes = vec![];
    if let Some(h) = &model.hierarchy {
        for &i in &h.order {
            nodes.push(HierarchyLayoutNode {
                id: model.data[i].id.clone(),
                label: model.data[i].label.clone(),
                parent: h.parent[i].map(|p| model.data[p].id.clone()),
                children: h.children[i]
                    .iter()
                    .map(|j| model.data[*j].id.clone())
                    .collect(),
                depth: h.depth[i],
                relative_depth: focus
                    .filter(|_| members.contains(&i))
                    .map(|f| h.depth[i] - h.depth[f]),
                value: h.totals[i],
                leaf: h.children[i].is_empty(),
                geometry: None,
                content_rect: None,
                visible: false,
                mark_id: format!("node:{}", model.data[i].id),
                label_slots: vec![],
            });
        }
    }
    let by_id = nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.id.clone(), i))
        .collect();
    let by_mark = nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.mark_id.clone(), i))
        .collect();
    let focus_id = focus.map(|i| model.data[i].id.clone());
    let ancestors = focus_id
        .as_ref()
        .map(|id| model.ancestors(id))
        .transpose()?
        .unwrap_or_default();
    Ok((
        HierarchyLayout {
            scene,
            root_id: model.root().map(String::from),
            focus_id,
            focus_ancestors: ancestors,
            nodes,
            resquarify_stats: None,
            by_id,
            by_mark,
            plot: ClipRect::plot(c),
            value_format: style.value_format.clone(),
            label_layout: Default::default(),
        },
        focus,
        branch,
    ))
}
fn box_of(r: Rect) -> ClipRect {
    ClipRect {
        x: r.x,
        y: r.y,
        width: r.w,
        height: r.h,
    }
}
fn inset(r: Rect, p: f64) -> Rect {
    Rect {
        x: r.x + p,
        y: r.y + p,
        w: (r.w - 2. * p).max(0.),
        h: (r.h - 2. * p).max(0.),
    }
}
fn label_text(l: &HierarchyLabelStyle, n: &HierarchyLayoutNode) -> Result<String> {
    match &l.text {
        HierarchyLabelText::Label => Ok(n.label.clone()),
        HierarchyLabelText::Id => Ok(n.id.clone()),
        HierarchyLabelText::Value(f) => f.format(n.value),
    }
}
fn emit(
    c: &ChartConfig,
    result: &mut HierarchyLayout,
    style: &HierarchyStyleOptions,
    headers: &BTreeMap<String, ClipRect>,
    parent_position: Option<(TreemapParentLabelPosition, f64)>,
) -> Result<()> {
    let ranks = result
        .by_id
        .keys()
        .enumerate()
        .map(|(i, id)| (id.clone(), i))
        .collect::<BTreeMap<_, _>>();
    for node in &mut result.nodes {
        let Some(geometry) = &node.geometry else {
            continue;
        };
        let visible = node.relative_depth.is_some()
            && node.value > 0.
            && !style.hidden.contains(&node.id)
            && (!style.leaves_only || node.leaf)
            && (style.include_focus || result.focus_id.as_ref() != Some(&node.id));
        node.visible = visible;
        if !visible {
            continue;
        }
        let ns = style.nodes.get(&node.id).unwrap_or(if node.leaf {
            &style.leaf
        } else {
            &style.parent
        });
        let color = match ns.fill {
            HierarchyFill::Palette => Some(c.theme.color(ranks[&node.id])),
            HierarchyFill::Fixed(color) => Some(color),
            HierarchyFill::None => None,
        };
        let mut mark = Mark::new(
            &node.mark_id,
            geometry.clone(),
            color.unwrap_or(c.theme.grid),
        );
        mark.fill = color;
        mark.fill_paint = ns.paint.clone();
        mark.stroke = ns.border;
        mark.opacity = ns.opacity;
        mark.series = node.id.clone();
        mark.value = node.value;
        mark.label = format!(
            "{}: {}",
            if node.label.is_empty() {
                &node.id
            } else {
                &node.label
            },
            style.value_format.format(node.value)?
        );
        if style.clip_plot {
            mark.clip = Some(ClipRect::plot(c));
        }
        result.scene.marks.push(mark);
        let label = if node.leaf {
            &style.leaf_labels
        } else {
            &style.parent_labels
        };
        if !label.enabled {
            continue;
        }
        let (bounds, radius) = match geometry {
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
                width.min(*height) / 2.,
            ),
            Geometry::Circle { x, y, radius } => (
                ClipRect {
                    x: x - radius,
                    y: y - radius,
                    width: 2. * radius,
                    height: 2. * radius,
                },
                *radius,
            ),
            _ => return Err(ChartError("unsupported hierarchy node geometry".into())),
        };
        let bounds = headers.get(&node.id).copied().unwrap_or(bounds);
        if bounds.width < label.skip_width
            || bounds.height < label.skip_height
            || radius < label.skip_radius
        {
            continue;
        }
        let size = label.size.unwrap_or(c.theme.font_size);
        let mut at = [bounds.x + bounds.width / 2., bounds.y + bounds.height / 2.];
        let mut rotation = label.rotation;
        let mut anchor = TextAnchor::Middle;
        if let Some((position, padding)) =
            parent_position.filter(|_| !node.leaf && headers.contains_key(&node.id))
        {
            anchor = TextAnchor::Start;
            match position {
                TreemapParentLabelPosition::Top | TreemapParentLabelPosition::Bottom => {
                    at = [bounds.x + padding, at[1] + size / 3.];
                }
                TreemapParentLabelPosition::Left | TreemapParentLabelPosition::Right => {
                    at = [at[0] + size / 3., bounds.y + bounds.height - padding];
                    rotation -= 90.;
                }
            }
        } else {
            if label.orient_to_rect && bounds.height > bounds.width {
                rotation -= 90.;
            }
            at[0] -= rotation.to_radians().sin() * size / 3.;
            at[1] += rotation.to_radians().cos() * size / 3.;
        }
        rotation = (rotation + 180.).rem_euclid(360.) - 180.;
        let slot = result.scene.labels.len();
        result.scene.label_series.insert(slot, node.id.clone());
        node.label_slots.push(slot);
        result.scene.labels.push(Label {
            x: at[0] + label.offset[0],
            y: at[1] + label.offset[1],
            text: label_text(label, node)?,
            size,
            color: label.color.unwrap_or(c.theme.text),
            anchor,
            rotation,
        });
        if label.clip_to_node {
            result.scene.label_clips.insert(
                slot,
                ClipPath {
                    geometry: if headers.contains_key(&node.id) {
                        Geometry::Rect {
                            x: bounds.x,
                            y: bounds.y,
                            width: bounds.width,
                            height: bounds.height,
                        }
                    } else {
                        geometry.clone()
                    },
                    rule: ClipRule::NonZero,
                },
            );
        } else if style.clip_plot {
            let r = ClipRect::plot(c);
            result.scene.label_clips.insert(
                slot,
                ClipPath {
                    geometry: Geometry::Rect {
                        x: r.x,
                        y: r.y,
                        width: r.width,
                        height: r.height,
                    },
                    rule: ClipRule::NonZero,
                },
            );
        }
    }
    result.scene.validate()?;
    result.label_layout = labels::HierarchyLabelState::capture(result)?;
    Ok(())
}
fn treemap_build(
    c: &ChartConfig,
    model: &HierarchyModel,
    focus: Option<&str>,
    o: &TreemapLayoutOptions,
    state: &mut TreemapLayoutState,
) -> Result<HierarchyLayout> {
    ensure(
        [
            o.inner_padding,
            o.outer_padding,
            o.parent_label_size,
            o.parent_label_padding,
        ]
        .into_iter()
        .all(bounded),
        "invalid treemap padding or parent label reservation",
    )?;
    let (mut result, focus, branch) = initial(c, model, focus, &o.style)?;
    let Some(focus) = focus else {
        return Ok(result);
    };
    let h = model.hierarchy.as_ref().unwrap();
    if h.totals[focus] == 0. {
        return Ok(result);
    }
    let mut boxes = vec![None; model.data.len()];
    boxes[focus] = Some(Rect {
        x: c.margin.left,
        y: c.margin.top,
        w: c.inner_width(),
        h: c.inner_height(),
    });
    let mut headers = BTreeMap::new();
    let index = model
        .index
        .iter()
        .map(|(id, i)| (id.as_str(), *i))
        .collect::<BTreeMap<_, _>>();
    for i in branch {
        if h.totals[i] == 0. {
            continue;
        }
        let r = boxes[i].ok_or_else(|| ChartError("missing treemap allocation".into()))?;
        ensure(
            r.w > 0. && r.h > 0.,
            "treemap padding exhausted a positive node",
        )?;
        let slot = result.by_id[&model.data[i].id];
        result.nodes[slot].geometry = Some(Geometry::Rect {
            x: r.x,
            y: r.y,
            width: r.w,
            height: r.h,
        });
        if h.children[i].is_empty() {
            continue;
        }
        let mut content = inset(r, o.outer_padding);
        let reserve = o.style.parent_labels.enabled
            && !o.style.leaves_only
            && (o.style.include_focus || i != focus)
            && o.parent_label_size > 0.;
        if reserve {
            let band = o.parent_label_size;
            let header = match o.parent_label_position {
                TreemapParentLabelPosition::Top => {
                    let header = Rect { h: band, ..content };
                    content.y += band;
                    content.h -= band;
                    header
                }
                TreemapParentLabelPosition::Bottom => {
                    content.h -= band;
                    Rect {
                        y: content.y + content.h,
                        h: band,
                        ..content
                    }
                }
                TreemapParentLabelPosition::Left => {
                    let header = Rect { w: band, ..content };
                    content.x += band;
                    content.w -= band;
                    header
                }
                TreemapParentLabelPosition::Right => {
                    content.w -= band;
                    Rect {
                        x: content.x + content.w,
                        w: band,
                        ..content
                    }
                }
            };
            headers.insert(model.data[i].id.clone(), box_of(header));
        }
        ensure(
            content.w > 0. && content.h > 0.,
            "treemap outer/header padding exhausted parent content",
        )?;
        result.nodes[slot].content_rect = Some(box_of(content));
        let half = o.inner_padding / 2.;
        let tilespace = Rect {
            x: content.x - half,
            y: content.y - half,
            w: content.w + o.inner_padding,
            h: content.h + o.inner_padding,
        };
        let positive = h.children[i]
            .iter()
            .copied()
            .filter(|j| h.totals[*j] > 0.)
            .collect::<Vec<_>>();
        let tiles = match o.tile {
            HierarchyTreemapTile::Static(TreemapTile::Binary) => {
                radial::binary_tiles(&positive, &h.totals, tilespace)
            }
            HierarchyTreemapTile::Static(TreemapTile::Slice) => {
                radial::slice_tiles(&positive, &h.totals, tilespace, false)
            }
            HierarchyTreemapTile::Static(TreemapTile::Dice) => {
                radial::slice_tiles(&positive, &h.totals, tilespace, true)
            }
            HierarchyTreemapTile::Static(TreemapTile::SliceDice) => radial::slice_tiles(
                &positive,
                &h.totals,
                tilespace,
                (h.depth[i] - h.depth[focus]).is_multiple_of(2),
            ),
            _ => {
                let rows = if o.tile == HierarchyTreemapTile::Resquarify {
                    if let Some(rows) = state.cache.parents.get(&model.data[i].id) {
                        state.cache.stats.reused_parents += 1;
                        rows.clone()
                    } else {
                        state.cache.stats.rebuilt_parents += 1;
                        radial::initial_treemap_rows(
                            &h.children[i],
                            &h.totals,
                            &model.data,
                            tilespace,
                            o.ratio,
                        )
                        .into()
                    }
                } else {
                    radial::initial_treemap_rows(
                        &h.children[i],
                        &h.totals,
                        &model.data,
                        tilespace,
                        o.ratio,
                    )
                    .into()
                };
                let tiles = radial::layout_treemap_rows(&rows, &index, &h.totals, tilespace);
                if o.tile == HierarchyTreemapTile::Resquarify {
                    state.cache.parents.insert(model.data[i].id.clone(), rows);
                }
                tiles
            }
        };
        for (j, tile) in tiles {
            boxes[j] = Some(inset(tile, half));
        }
    }
    if o.tile == HierarchyTreemapTile::Resquarify {
        result.resquarify_stats = Some(state.cache.last_stats());
    }
    emit(
        c,
        &mut result,
        &o.style,
        &headers,
        Some((o.parent_label_position, o.parent_label_padding)),
    )?;
    Ok(result)
}
pub fn circle_packing_layout(
    c: &ChartConfig,
    data: &[HierarchyDatum],
    focus: Option<&str>,
    o: &CirclePackingLayoutOptions,
) -> Result<HierarchyLayout> {
    circle_packing_model(c, &HierarchyModel::new(data)?, focus, o)
}
pub(crate) fn circle_packing_model(
    c: &ChartConfig,
    model: &HierarchyModel,
    focus: Option<&str>,
    o: &CirclePackingLayoutOptions,
) -> Result<HierarchyLayout> {
    ensure(
        model.data.len() <= 2000,
        "circle packing exceeds 2000 node budget",
    )?;
    ensure(
        o.padding.is_finite() && (0. ..=1024.).contains(&o.padding) && bounded(o.viewport_padding),
        "invalid focused packing padding",
    )?;
    let (mut result, focus, branch) = initial(c, model, focus, &o.style)?;
    ensure(
        o.viewport_padding * 2. < c.inner_width().min(c.inner_height()),
        "packing viewport padding leaves no radius",
    )?;
    let Some(focus) = focus else {
        return Ok(result);
    };
    let h = model.hierarchy.as_ref().unwrap();
    if h.totals[h.root] == 0. {
        return Ok(result);
    }
    let packed = radial::packing_local(
        &model.data,
        h,
        c.inner_width().min(c.inner_height()) / 2.,
        o.padding,
        o.algorithm,
        true,
    )?;
    let fr = packed.radii[focus];
    ensure(
        fr.is_finite() && fr > 0.,
        "focus has no positive representable packing circle",
    )?;
    let radius = c.inner_width().min(c.inner_height()) / 2. - o.viewport_padding;
    let center = [
        c.margin.left + c.inner_width() / 2.,
        c.margin.top + c.inner_height() / 2.,
    ];
    let mut projected = vec![None; model.data.len()];
    projected[focus] = Some((center[0], center[1], radius));
    for &i in &branch {
        if h.totals[i] == 0. {
            continue;
        }
        if i != focus {
            let (px, py, _) = projected[h.parent[i].unwrap()]
                .ok_or_else(|| ChartError("missing focused packing parent".into()))?;
            // Ratios are taken BEFORE multiplication and only parent-relative
            // offsets are accumulated. No subtraction of rounded screen
            // centers and no enormous global zoom scalar are necessary.
            projected[i] = Some((
                px + packed.offsets[i].0 / fr * radius,
                py + packed.offsets[i].1 / fr * radius,
                packed.radii[i] / fr * radius,
            ));
        }
        let (x, y, r) = projected[i].unwrap();
        ensure(
            x.is_finite() && y.is_finite() && r.is_finite() && r > 0.,
            "focused packing projection overflow",
        )?;
        result.nodes[result.by_id[&model.data[i].id]].geometry =
            Some(Geometry::Circle { x, y, radius: r });
    }
    // The two original packing passes keep their enclosure-work budget. An
    // additional bounded admission pass catches genuinely unrepresentable
    // projected gaps/containment instead of publishing overlapping ink.
    let mut checks = 0usize;
    for &parent in &branch {
        let Some((px, py, pr)) = projected[parent] else {
            continue;
        };
        let children = h.children[parent]
            .iter()
            .filter_map(|i| projected[*i])
            .collect::<Vec<_>>();
        let epsilon = 1e-8 * pr.max(1.) + 128. * f64::EPSILON * px.abs().max(py.abs());
        for (j, &(x, y, r)) in children.iter().enumerate() {
            checks += 1;
            ensure(
                checks <= MAX_ENCLOSURE_TESTS && ((x - px).hypot(y - py) + r) <= pr + epsilon,
                "focused packing containment or work budget exceeded",
            )?;
            for &(xx, yy, rr) in &children[j + 1..] {
                checks += 1;
                ensure(
                    checks <= MAX_ENCLOSURE_TESTS && (x - xx).hypot(y - yy) + epsilon >= r + rr,
                    "focused packing separation or work budget exceeded",
                )?;
            }
        }
    }
    emit(c, &mut result, &o.style, &BTreeMap::new(), None)?;
    Ok(result)
}
pub fn icicle_layout(
    c: &ChartConfig,
    data: &[HierarchyDatum],
    focus: Option<&str>,
    o: &IcicleLayoutOptions,
) -> Result<HierarchyLayout> {
    icicle_model(c, &HierarchyModel::new(data)?, focus, o)
}
pub(crate) fn icicle_model(
    c: &ChartConfig,
    model: &HierarchyModel,
    focus: Option<&str>,
    o: &IcicleLayoutOptions,
) -> Result<HierarchyLayout> {
    ensure(bounded(o.padding), "invalid icicle padding")?;
    let (mut result, focus, branch) = initial(c, model, focus, &o.style)?;
    let Some(focus) = focus else {
        return Ok(result);
    };
    let h = model.hierarchy.as_ref().unwrap();
    if h.totals[focus] == 0. {
        return Ok(result);
    }
    let first = usize::from(!o.style.include_focus);
    let max_depth = branch
        .iter()
        .map(|i| h.depth[*i] - h.depth[focus])
        .max()
        .unwrap_or(0);
    let levels = (max_depth + 1).saturating_sub(first).max(1);
    let horizontal = matches!(o.direction, IcicleDirection::Left | IcicleDirection::Right);
    let depth_span = if horizontal {
        c.inner_width()
    } else {
        c.inner_height()
    };
    let band = depth_span / levels as f64;
    let mut ranges = vec![(0., 0.); model.data.len()];
    ranges[focus] = (0., 1.);
    for i in branch {
        let (a, b) = ranges[i];
        let children = h.children[i]
            .iter()
            .copied()
            .filter(|j| h.totals[*j] > 0.)
            .collect::<Vec<_>>();
        let maximum = children.iter().map(|j| h.totals[*j]).fold(0., f64::max);
        if maximum > 0. {
            let sum: f64 = children.iter().map(|j| h.totals[*j] / maximum).sum();
            let mut position = a;
            for (n, &j) in children.iter().enumerate() {
                let end = if n + 1 == children.len() {
                    b
                } else {
                    position + (b - a) * (h.totals[j] / maximum / sum)
                };
                ranges[j] = (position, end);
                position = end;
            }
        }
        let depth = h.depth[i] - h.depth[focus];
        if depth < first || h.totals[i] == 0. {
            continue;
        }
        let d = (depth - first) as f64;
        let r = match o.direction {
            IcicleDirection::Down => Rect {
                x: c.margin.left + a * c.inner_width(),
                y: c.margin.top + d * band,
                w: (b - a) * c.inner_width(),
                h: band,
            },
            IcicleDirection::Up => Rect {
                x: c.margin.left + a * c.inner_width(),
                y: c.height - c.margin.bottom - (d + 1.) * band,
                w: (b - a) * c.inner_width(),
                h: band,
            },
            IcicleDirection::Right => Rect {
                x: c.margin.left + d * band,
                y: c.margin.top + a * c.inner_height(),
                w: band,
                h: (b - a) * c.inner_height(),
            },
            IcicleDirection::Left => Rect {
                x: c.width - c.margin.right - (d + 1.) * band,
                y: c.margin.top + a * c.inner_height(),
                w: band,
                h: (b - a) * c.inner_height(),
            },
        };
        let r = inset(r, o.padding / 2.);
        ensure(
            r.w > 0. && r.h > 0.,
            "icicle padding or projection collapsed a positive node",
        )?;
        result.nodes[result.by_id[&model.data[i].id]].geometry = Some(Geometry::Rect {
            x: r.x,
            y: r.y,
            width: r.w,
            height: r.h,
        });
    }
    emit(c, &mut result, &o.style, &BTreeMap::new(), None)?;
    Ok(result)
}
