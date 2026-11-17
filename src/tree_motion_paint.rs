//! Validated physical paint over an immutable Tree motion/target cohort.
//!
//! No source/layout factory is replayed. Default native recipes preserve the
//! configured Tree shapes and curves; an edited geometry requires an explicit
//! per-frame renderer instead of silently restoring a constructor shape.
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use glory::render::{GraphicPoint as Point, GraphicStroke, PathCommand as Command};

use crate::scene::ensure;
use crate::*;

pub type TreeMotionNodeRenderer<'a> = dyn FnMut(&TreeMotionNode, &Mark) -> Result<Mark> + 'a;
pub type TreeMotionLinkRenderer<'a> =
    dyn FnMut(&TreeMotionLink, &Mark, &TreeSvgLinkGenerator) -> Result<Mark> + 'a;
pub type TreeMotionLabelRenderer<'a> = dyn FnMut(&TreeMotionLabel, &Label) -> Result<Label> + 'a;

/// Explicit per-attempt render functions, separate from target layout factories.
/// Application side effects are not rolled back when a later renderer rejects.
#[derive(Default)]
pub struct TreeMotionRenderers<'a> {
    pub node: Option<&'a mut TreeMotionNodeRenderer<'a>>,
    pub link: Option<&'a mut TreeMotionLinkRenderer<'a>>,
    pub label: Option<&'a mut TreeMotionLabelRenderer<'a>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TreeMotionPaintRole {
    Node,
    Link,
    Label,
}

#[derive(Clone, Debug)]
struct PaintData {
    motion: TreeMotionFrame,
    scene: Scene,
    nodes: Vec<DrawItem>,
    links: Vec<DrawItem>,
    labels: Vec<DrawItem>,
    extras: Vec<DrawItem>,
    source_marks: BTreeMap<usize, usize>,
    source_labels: BTreeMap<usize, usize>,
    target_marks: BTreeMap<usize, usize>,
    target_labels: BTreeMap<usize, usize>,
    instances: BTreeMap<u64, DrawItem>,
    mark_rows: BTreeMap<usize, (TreeMotionPaintRole, u64)>,
    label_rows: BTreeMap<usize, u64>,
    live_marks: Vec<usize>,
    ghost_marks: BTreeSet<String>,
    ghost_labels: BTreeSet<usize>,
}

/// Shared target provenance and actual sampled paint, with explicit role maps.
/// Ghost ink has no current business identity or automatic input eligibility.
#[derive(Clone, Debug)]
pub struct TreeMotionPaintFrame(Arc<PaintData>);
impl PartialEq for TreeMotionPaintFrame {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for TreeMotionPaintFrame {}
impl TreeMotionPaintFrame {
    pub fn motion(&self) -> &TreeMotionFrame {
        &self.0.motion
    }
    pub fn target(&self) -> &TreeEventFrame {
        self.0.motion.target()
    }
    pub fn scene(&self) -> &Scene {
        &self.0.scene
    }
    pub fn node_items(&self) -> &[DrawItem] {
        &self.0.nodes
    }
    pub fn link_items(&self) -> &[DrawItem] {
        &self.0.links
    }
    pub fn label_items(&self) -> &[DrawItem] {
        &self.0.labels
    }
    pub fn extras(&self) -> &[DrawItem] {
        &self.0.extras
    }
    pub fn live_mark_indices(&self) -> &[usize] {
        &self.0.live_marks
    }
    pub fn ghost_mark_ids(&self) -> &BTreeSet<String> {
        &self.0.ghost_marks
    }
    pub fn ghost_label_indices(&self) -> &BTreeSet<usize> {
        &self.0.ghost_labels
    }
    /// Map a DrawItem from the current target Scene to sampled paint.
    pub fn source_item(&self, item: DrawItem) -> Option<DrawItem> {
        match item {
            DrawItem::Mark(i) => self.0.source_marks.get(&i).copied().map(DrawItem::Mark),
            DrawItem::Label(i) => self.0.source_labels.get(&i).copied().map(DrawItem::Label),
        }
    }
    /// Ghost items have no current-target index.
    pub fn target_item(&self, item: DrawItem) -> Option<DrawItem> {
        match item {
            DrawItem::Mark(i) => self.0.target_marks.get(&i).copied().map(DrawItem::Mark),
            DrawItem::Label(i) => self.0.target_labels.get(&i).copied().map(DrawItem::Label),
        }
    }
    pub fn instance_for_item(&self, item: DrawItem) -> Option<u64> {
        match item {
            DrawItem::Mark(i) => self.0.mark_rows.get(&i).map(|r| r.1),
            DrawItem::Label(i) => self.0.label_rows.get(&i).copied(),
        }
    }
    pub fn role_for_item(&self, item: DrawItem) -> Option<TreeMotionPaintRole> {
        match item {
            DrawItem::Mark(i) => self.0.mark_rows.get(&i).map(|r| r.0),
            DrawItem::Label(i) => self
                .0
                .label_rows
                .contains_key(&i)
                .then_some(TreeMotionPaintRole::Label),
        }
    }
    pub fn item_for_instance(&self, instance: u64) -> Option<DrawItem> {
        self.0.instances.get(&instance).copied()
    }
    pub fn mark_for_instance(&self, instance: u64) -> Option<usize> {
        match self.item_for_instance(instance)? {
            DrawItem::Mark(i) => Some(i),
            _ => None,
        }
    }
    pub fn label_for_instance(&self, instance: u64) -> Option<usize> {
        match self.item_for_instance(instance)? {
            DrawItem::Label(i) => Some(i),
            _ => None,
        }
    }
}

pub fn tree_motion_paint(frame: &TreeMotionFrame) -> Result<TreeMotionPaintFrame> {
    tree_motion_paint_with_renderers(frame, &mut TreeMotionRenderers::default())
}

/// Build and validate the complete candidate before returning it. Default
/// native projection supports the four Cartesian Tree orientations. Radial
/// animation and multi-Mark opacity groups need separate presentation policies.
pub fn tree_motion_paint_with_renderers(
    frame: &TreeMotionFrame,
    renderers: &mut TreeMotionRenderers<'_>,
) -> Result<TreeMotionPaintFrame> {
    // Reject unsupported historical cohorts before invoking any application
    // renderer. Departures can still refer to a different target Scene.
    paint_policy(frame.target())?;
    for row in frame.nodes() {
        paint_policy(row.data().frame())?;
    }
    for row in frame.links() {
        paint_policy(row.data().frame())?;
    }
    for row in frame.labels() {
        paint_policy(row.data().frame())?;
    }
    let mut builder = PaintBuilder::new(frame)?;
    for row in frame.links() {
        let Some(source) = row.data().mark() else {
            continue;
        };
        let options = row.data().frame().layout().options();
        ensure(
            options.orientation != TreeOrientation::Radial,
            "Tree motion departure requires Cartesian orientation",
        )?;
        let generator = TreeSvgLinkGenerator::new(options.style.links.curve, options.orientation);
        let mark = if let Some(renderer) = &mut renderers.link {
            renderer(row, source, &generator)?
        } else {
            link_mark(row, source, &generator)?
        };
        renderer_identity(source, &mark)?;
        let item = builder.mark(
            mark,
            source,
            row.phase(),
            row.instance(),
            TreeMotionPaintRole::Link,
        )?;
        builder.data.links.push(item);
    }
    for row in frame.nodes() {
        let Some(source) = row.data().mark() else {
            continue;
        };
        let mark = if let Some(renderer) = &mut renderers.node {
            renderer(row, source)?
        } else {
            node_mark(row, source)?
        };
        renderer_identity(source, &mark)?;
        let item = builder.mark(
            mark,
            source,
            row.phase(),
            row.instance(),
            TreeMotionPaintRole::Node,
        )?;
        builder.data.nodes.push(item);
    }
    for row in frame.labels() {
        let source = row.data().label();
        let label = if let Some(renderer) = &mut renderers.label {
            renderer(row, source)?
        } else {
            let mut label = source.clone();
            label.x = row.props().x;
            label.y = row.props().y;
            label.rotation = row.props().rotation;
            label
        };
        let item = builder.label(label, row)?;
        builder.data.labels.push(item);
    }
    builder.extras()?;
    builder.order()?;
    builder.data.scene.validate()?;
    Ok(TreeMotionPaintFrame(Arc::new(builder.data)))
}

fn paint_policy(frame: &TreeEventFrame) -> Result<()> {
    ensure(
        frame.layout().options().orientation != TreeOrientation::Radial,
        "Tree motion paint requires Cartesian orientation",
    )?;
    ensure(
        frame.scene().opacity_groups.is_empty(),
        "Tree motion paint does not support multi-Mark opacity groups",
    )
}

fn renderer_identity(source: &Mark, rendered: &Mark) -> Result<()> {
    ensure(
        source.id == rendered.id
            && source.series == rendered.series
            && source.label == rendered.label
            && source.value.to_bits() == rendered.value.to_bits(),
        "Tree motion renderer changed business Mark metadata",
    )
}

fn node_geometry(shape: TreeNodeShape, position: [f64; 2], radius: f64) -> Geometry {
    let [x, y] = position;
    match shape {
        TreeNodeShape::Circle => Geometry::Circle { x, y, radius },
        TreeNodeShape::Square => Geometry::Rect {
            x: x - radius,
            y: y - radius,
            width: radius * 2.,
            height: radius * 2.,
        },
        TreeNodeShape::Diamond => Geometry::Path(vec![
            Command::Move(Point::new(x, y - radius)),
            Command::Line(Point::new(x + radius, y)),
            Command::Line(Point::new(x, y + radius)),
            Command::Line(Point::new(x - radius, y)),
            Command::Close,
        ]),
    }
}
fn node_mark(row: &TreeMotionNode, source: &Mark) -> Result<Mark> {
    let data = row.data().node();
    ensure(
        source.geometry == node_geometry(data.style.shape, data.position, data.radius),
        "Tree motion edited node geometry requires an explicit renderer",
    )?;
    let mut mark = source.clone();
    mark.geometry = node_geometry(
        data.style.shape,
        [row.props().x, row.props().y],
        row.props().diameter / 2.,
    );
    // Other authored paints remain target properties. A canonical solid fill
    // routes through the animated color; gradients/explicit overrides stay authored.
    if mark.fill_paint.is_none() && mark.fill.is_some() && mark.fill == data.color {
        mark.fill = Some(row.props().color);
    }
    Ok(mark)
}
fn link_mark(
    row: &TreeMotionLink,
    source: &Mark,
    generator: &TreeSvgLinkGenerator,
) -> Result<Mark> {
    let data = row.data();
    let target = data.link();
    ensure(
        source.geometry == generator.geometry(target.source_position, target.target_position)?,
        "Tree motion edited link geometry requires an explicit renderer",
    )?;
    let style = data.style();
    let options = &data.frame().layout().options().style.links;
    let expected = (style.thickness > 0.).then_some(GraphicStroke {
        width: style.thickness,
        color: style.color,
        cap: options.cap,
    });
    ensure(
        source.stroke == expected
            && source.stroke_dash
                == (style.thickness > 0.)
                    .then(|| options.dash.clone())
                    .flatten(),
        "Tree motion edited link stroke requires an explicit renderer",
    )?;
    let mut mark = source.clone();
    mark.geometry = generator.geometry(row.props().source, row.props().target)?;
    mark.stroke = (row.props().thickness > 0.).then_some(GraphicStroke {
        width: row.props().thickness,
        color: row.props().color,
        cap: options.cap,
    });
    mark.stroke_dash = if mark.stroke.is_some() {
        options.dash.clone()
    } else {
        None
    };
    Ok(mark)
}

struct PaintBuilder<'a> {
    data: PaintData,
    target: &'a Scene,
    target_mark_ids: BTreeMap<&'a str, usize>,
    reserved: BTreeSet<String>,
}
impl<'a> PaintBuilder<'a> {
    fn new(motion: &'a TreeMotionFrame) -> Result<Self> {
        let target = motion.target().scene();
        ensure(
            target.opacity_groups.is_empty(),
            "Tree motion paint does not support multi-Mark opacity groups",
        )?;
        let mut scene = target.clone();
        scene.marks.clear();
        scene.labels.clear();
        scene.draw_order.clear();
        scene.label_series.clear();
        scene.label_clips.clear();
        scene.typography.labels.clear();
        scene.typography.label_whitespace.clear();
        scene.typography.label_strokes.clear();
        let data = PaintData {
            motion: motion.clone(),
            scene,
            nodes: vec![],
            links: vec![],
            labels: vec![],
            extras: vec![],
            source_marks: BTreeMap::new(),
            source_labels: BTreeMap::new(),
            target_marks: BTreeMap::new(),
            target_labels: BTreeMap::new(),
            instances: BTreeMap::new(),
            mark_rows: BTreeMap::new(),
            label_rows: BTreeMap::new(),
            live_marks: vec![],
            ghost_marks: BTreeSet::new(),
            ghost_labels: BTreeSet::new(),
        };
        Ok(Self {
            data,
            target,
            target_mark_ids: target
                .marks
                .iter()
                .enumerate()
                .map(|(i, m)| (m.id.as_str(), i))
                .collect(),
            reserved: target.marks.iter().map(|m| m.id.clone()).collect(),
        })
    }
    fn mark(
        &mut self,
        mut mark: Mark,
        source: &Mark,
        phase: TreeMotionPhase,
        instance: u64,
        role: TreeMotionPaintRole,
    ) -> Result<DrawItem> {
        ensure(
            self.data.scene.marks.len() < MAX_MARKS,
            "Tree motion paint Mark union budget exceeded",
        )?;
        if phase == TreeMotionPhase::Leaving {
            // Other departures retiring must not rename this instance.
            let mut ghost_serial = 0_u64;
            loop {
                let id = format!("kartis-tree-motion-exit-{instance}-{ghost_serial}");
                ghost_serial = ghost_serial
                    .checked_add(1)
                    .ok_or_else(|| ChartError("Tree motion ghost identity exhausted".into()))?;
                if self.reserved.insert(id.clone()) {
                    mark.id = id;
                    break;
                }
            }
            mark.label.clear();
            mark.value = 0.;
            self.data.ghost_marks.insert(mark.id.clone());
        } else {
            let original = *self
                .target_mark_ids
                .get(source.id.as_str())
                .ok_or_else(|| ChartError("Tree motion current source Mark missing".into()))?;
            ensure(
                self.data
                    .source_marks
                    .insert(original, self.data.scene.marks.len())
                    .is_none(),
                "Tree motion current Mark ownership is ambiguous",
            )?;
            self.data
                .target_marks
                .insert(self.data.scene.marks.len(), original);
            self.data.live_marks.push(self.data.scene.marks.len());
        }
        let index = self.data.scene.marks.len();
        let item = DrawItem::Mark(index);
        ensure(
            self.data.instances.insert(instance, item).is_none(),
            "Tree motion instance is ambiguous",
        )?;
        self.data.mark_rows.insert(index, (role, instance));
        self.data.scene.marks.push(mark);
        Ok(item)
    }
    fn label(&mut self, label: Label, row: &TreeMotionLabel) -> Result<DrawItem> {
        ensure(
            self.data.scene.labels.len() < MAX_MARKS,
            "Tree motion paint Label union budget exceeded",
        )?;
        let index = self.data.scene.labels.len();
        let item = DrawItem::Label(index);
        let data = row.data();
        if row.phase() == TreeMotionPhase::Live {
            ensure(
                self.data
                    .source_labels
                    .insert(data.label_index(), index)
                    .is_none(),
                "Tree motion current Label ownership is ambiguous",
            )?;
            self.data.target_labels.insert(index, data.label_index());
        } else {
            self.data.ghost_labels.insert(index);
        }
        self.data
            .scene
            .label_series
            .insert(index, data.datum().id.clone());
        self.data.scene.typography.labels.insert(
            index,
            crate::FontOptions {
                family: Some(data.font().family),
                weight: Some(data.font().weight),
            },
        );
        self.data
            .scene
            .typography
            .label_whitespace
            .insert(index, data.whitespace());
        if let Some(stroke) = data
            .frame()
            .scene()
            .typography
            .label_strokes
            .get(&data.label_index())
        {
            self.data
                .scene
                .typography
                .label_strokes
                .insert(index, *stroke);
        }
        if let Some(clip) = data.clip() {
            self.data.scene.label_clips.insert(index, clip.clone());
        }
        ensure(
            self.data.instances.insert(row.instance(), item).is_none(),
            "Tree motion label instance is ambiguous",
        )?;
        self.data.label_rows.insert(index, row.instance());
        self.data.scene.labels.push(label);
        Ok(item)
    }
    fn extras(&mut self) -> Result<()> {
        for item in self.target.draw_items() {
            match item {
                DrawItem::Mark(i) if !self.data.source_marks.contains_key(&i) => {
                    ensure(
                        self.data.scene.marks.len() < MAX_MARKS,
                        "Tree motion extra Mark budget exceeded",
                    )?;
                    let index = self.data.scene.marks.len();
                    self.data.scene.marks.push(self.target.marks[i].clone());
                    self.data.source_marks.insert(i, index);
                    self.data.target_marks.insert(index, i);
                    self.data.live_marks.push(index);
                    self.data.extras.push(DrawItem::Mark(index));
                }
                DrawItem::Label(i) if !self.data.source_labels.contains_key(&i) => {
                    ensure(
                        self.data.scene.labels.len() < MAX_MARKS,
                        "Tree motion extra Label budget exceeded",
                    )?;
                    let index = self.data.scene.labels.len();
                    self.data.scene.labels.push(self.target.labels[i].clone());
                    self.data.source_labels.insert(i, index);
                    self.data.target_labels.insert(index, i);
                    if let Some(series) = self.target.label_series.get(&i) {
                        self.data.scene.label_series.insert(index, series.clone());
                    }
                    if let Some(clip) = self.target.label_clips.get(&i) {
                        self.data.scene.label_clips.insert(index, clip.clone());
                    }
                    self.data
                        .scene
                        .typography
                        .labels
                        .insert(index, self.target.typography.label_options(i));
                    self.data
                        .scene
                        .typography
                        .label_whitespace
                        .insert(index, self.target.typography.label_whitespace(i));
                    if let Some(stroke) = self.target.typography.label_strokes.get(&i) {
                        self.data
                            .scene
                            .typography
                            .label_strokes
                            .insert(index, *stroke);
                    }
                    self.data.extras.push(DrawItem::Label(index));
                }
                _ => {}
            }
        }
        Ok(())
    }
    fn order(&mut self) -> Result<()> {
        // Preserve all current mixed order. Departures attach at their retained
        // role neighbor; no arbitrary extra or current item is moved.
        let mut before = BTreeMap::<(bool, usize), Vec<DrawItem>>::new();
        let mut after = BTreeMap::<(bool, usize), Vec<DrawItem>>::new();
        let mut tail = Vec::new();
        for family in [&self.data.links, &self.data.nodes, &self.data.labels] {
            let mut pending = Vec::new();
            let mut previous = None;
            for &item in family {
                let live = match item {
                    DrawItem::Mark(i) => self.data.target_marks.contains_key(&i),
                    DrawItem::Label(i) => self.data.target_labels.contains_key(&i),
                };
                if live {
                    if !pending.is_empty() {
                        if let Some(anchor) = previous {
                            after.entry(anchor).or_default().append(&mut pending);
                        } else {
                            before
                                .entry(item_key(item))
                                .or_default()
                                .append(&mut pending);
                        }
                    }
                    previous = Some(item_key(item));
                } else {
                    pending.push(item);
                }
            }
            if !pending.is_empty() {
                if let Some(anchor) = previous {
                    after.entry(anchor).or_default().append(&mut pending);
                } else {
                    tail.append(&mut pending);
                }
            }
        }
        let mut order = Vec::new();
        for source in self.target.draw_items() {
            let item = match source {
                DrawItem::Mark(i) => self.data.source_marks.get(&i).copied().map(DrawItem::Mark),
                DrawItem::Label(i) => self
                    .data
                    .source_labels
                    .get(&i)
                    .copied()
                    .map(DrawItem::Label),
            }
            .ok_or_else(|| ChartError("Tree motion target item mapping missing".into()))?;
            if let Some(items) = before.remove(&item_key(item)) {
                order.extend(items);
            }
            order.push(item);
            if let Some(items) = after.remove(&item_key(item)) {
                order.extend(items);
            }
        }
        ensure(
            before.is_empty() && after.is_empty(),
            "Tree motion exit order anchor missing",
        )?;
        order.extend(tail);
        self.data.scene.draw_order = order;
        Ok(())
    }
}
fn item_key(item: DrawItem) -> (bool, usize) {
    match item {
        DrawItem::Mark(i) => (false, i),
        DrawItem::Label(i) => (true, i),
    }
}
