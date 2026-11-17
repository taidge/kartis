//! Checked source-order paint runs for live Widget composition.
//!
//! This compiler preserves authored order and identity; it does not mount UI,
//! apply visibility state, resolve datum anchors or perform host I/O. Hosts
//! composite each Group once and each non-Normal run against the preceding
//! backdrop, including preceding Widgets. A run must not isolate that backdrop.
use std::collections::{BTreeMap, BTreeSet};

use crate::scene::ensure;
use crate::{
    BlendMode, Color, DrawItem, Geometry, MarkBlends, PlotWidgetPlacement, PlotWidgetsLayout,
    Result, Scene, SceneTypography,
};

/// Maximum transparent renderer surfaces in one resolved plan.
pub const MAX_WIDGET_PAINT_RUNS: usize = 512;
/// Paint, Widget and Group nodes share this cumulative budget.
pub const MAX_WIDGET_PAINT_NODES: usize = 1024;
/// Authored mark commands, including the ink being replaced, share this budget.
/// Rectangles cost five commands and circles six; Path costs its actual length.
pub const MAX_WIDGET_PAINT_SOURCE_COMMANDS: usize = 200_000;

/// One transparent source-over batch, or one independently blended mark.
/// The Scene contains only this run's items and needed label declarations.
#[derive(Clone, Debug, PartialEq)]
pub struct WidgetPaintRun {
    scene: Box<Scene>,
    blend: BlendMode,
    source_items: Vec<DrawItem>,
}
impl WidgetPaintRun {
    pub fn scene(&self) -> &Scene {
        &self.scene
    }
    pub fn blend(&self) -> BlendMode {
        self.blend
    }
    /// References into the original authored Scene, in this run's paint order.
    pub fn source_items(&self) -> &[DrawItem] {
        &self.source_items
    }
}

/// Ordered renderer and Widget siblings. Groups are disjoint and never nest.
#[derive(Clone, Debug, PartialEq)]
pub enum WidgetPaintNode {
    Paint(WidgetPaintRun),
    Widget(String),
    /// The host composites these children before applying `opacity` once.
    /// Child run Scenes deliberately contain no copy of this opacity group.
    Group {
        id: String,
        opacity: f64,
        children: Vec<WidgetPaintNode>,
    },
}

/// Immutable, frame-local output. The original background is painted once by
/// the host, below these nodes; every individual run background is transparent.
#[derive(Clone, Debug, PartialEq)]
pub struct WidgetPaintPlan {
    width: f64,
    height: f64,
    background: Color,
    nodes: Vec<WidgetPaintNode>,
    widget_count: usize,
    run_count: usize,
    node_count: usize,
    source_commands: usize,
}
impl WidgetPaintPlan {
    pub fn width(&self) -> f64 {
        self.width
    }
    pub fn height(&self) -> f64 {
        self.height
    }
    pub fn background(&self) -> Color {
        self.background
    }
    pub fn nodes(&self) -> &[WidgetPaintNode] {
        &self.nodes
    }
    pub fn widget_count(&self) -> usize {
        self.widget_count
    }
    pub fn run_count(&self) -> usize {
        self.run_count
    }
    pub fn node_count(&self) -> usize {
        self.node_count
    }
    pub fn source_commands(&self) -> usize {
        self.source_commands
    }
}

#[derive(Default)]
struct Budget {
    nodes: usize,
    runs: usize,
}
impl Budget {
    fn node(&mut self) -> Result<()> {
        self.nodes += 1;
        ensure(
            self.nodes <= MAX_WIDGET_PAINT_NODES,
            "Widget paint node budget exceeded",
        )
    }
    fn run(&mut self) -> Result<()> {
        self.runs += 1;
        ensure(
            self.runs <= MAX_WIDGET_PAINT_RUNS,
            "Widget paint run budget exceeded",
        )?;
        self.node()
    }
}

fn placement_valid(placement: PlotWidgetPlacement, allow_zero: bool) -> Result<()> {
    let (width, height) = match placement {
        PlotWidgetPlacement::Rect(rect) => {
            rect.validate()?;
            ensure(
                (rect.width == 0. && allow_zero || rect.x + rect.width > rect.x)
                    && (rect.height == 0. && allow_zero || rect.y + rect.height > rect.y),
                "Widget paint slot dimensions must be representable",
            )?;
            (rect.width, rect.height)
        }
        PlotWidgetPlacement::AtDatum { offset, measured } => {
            measured.validate()?;
            ensure(
                offset
                    .iter()
                    .all(|v| v.is_finite() && v.abs() <= 1_000_000.),
                "invalid Widget paint datum offset",
            )?;
            (measured.width, measured.height)
        }
    };
    ensure(
        allow_zero || width > 0. && height > 0.,
        "Widget paint slot requires positive dimensions",
    )
}

fn run_scene(source: &Scene, items: &[DrawItem]) -> Scene {
    let mut scene = Scene {
        typography: SceneTypography {
            default: source.typography.default.clone(),
            ..Default::default()
        },
        width: source.width,
        height: source.height,
        title: source.title.clone(),
        background: Color(0, 0, 0, 0),
        marks: Vec::new(),
        opacity_groups: Vec::new(),
        labels: Vec::new(),
        legend: Vec::new(),
        legend_roles: BTreeMap::new(),
        legend_targets: BTreeMap::new(),
        draw_order: Vec::with_capacity(items.len()),
        label_series: BTreeMap::new(),
        label_clips: BTreeMap::new(),
    };
    for item in items {
        match *item {
            DrawItem::Mark(index) => {
                scene.draw_order.push(DrawItem::Mark(scene.marks.len()));
                scene.marks.push(source.marks[index].clone());
            }
            DrawItem::Label(index) => {
                let next = scene.labels.len();
                scene.draw_order.push(DrawItem::Label(next));
                scene.labels.push(source.labels[index].clone());
                if let Some(series) = source.label_series.get(&index) {
                    scene.label_series.insert(next, series.clone());
                }
                if let Some(clip) = source.label_clips.get(&index) {
                    scene.label_clips.insert(next, clip.clone());
                }
                if let Some(font) = source.typography.labels.get(&index) {
                    scene.typography.labels.insert(next, font.clone());
                }
                if let Some(whitespace) = source.typography.label_whitespace.get(&index) {
                    scene.typography.label_whitespace.insert(next, *whitespace);
                }
                if let Some(stroke) = source.typography.label_strokes.get(&index) {
                    scene.typography.label_strokes.insert(next, *stroke);
                }
            }
        }
    }
    scene
}

fn push_run(
    source: &Scene,
    items: Vec<DrawItem>,
    blend: BlendMode,
    nodes: &mut Vec<WidgetPaintNode>,
    budget: &mut Budget,
) -> Result<()> {
    if items.is_empty() {
        return Ok(());
    }
    budget.run()?;
    let scene = run_scene(source, &items);
    scene.validate()?;
    nodes.push(WidgetPaintNode::Paint(WidgetPaintRun {
        scene: Box::new(scene),
        blend,
        source_items: items,
    }));
    Ok(())
}
fn flush(
    source: &Scene,
    pending: &mut Vec<DrawItem>,
    nodes: &mut Vec<WidgetPaintNode>,
    budget: &mut Budget,
) -> Result<()> {
    push_run(
        source,
        std::mem::take(pending),
        BlendMode::Normal,
        nodes,
        budget,
    )
}

/// Compile a complete authored paint sequence without modifying its source.
///
/// A replacement slot substitutes its Widget for its source Mark. Other slots
/// insert their Widget immediately after their Mark. Labels keep their original
/// positions. Slot-list order does not override Scene/LayerPlan paint order.
/// Zero-opacity and clipped sources retain their plan identity; the host applies
/// current visibility and input eligibility using the original slot frame.
///
/// Normal items batch until a Widget/group boundary. Each non-Normal Mark gets
/// a separate run. Replaced ink has no run, so its original blend is not applied
/// implicitly to the arbitrary Widget. A host must support cross-plane blending
/// or reject the plan before publication; this compiler does not fake blending.
///
/// Typed payload keys and resolved datum anchors are checked by the host's slot
/// packet protocol. This function checks the Scene, slot identities, structural
/// placements, replacement subset, source blend map and cumulative plan limits.
pub fn widget_paint_plan(
    scene: &Scene,
    slots: &PlotWidgetsLayout,
    replace_ink: &BTreeSet<String>,
    blends: &MarkBlends,
) -> Result<WidgetPaintPlan> {
    let slots = crate::SceneWidgetsLayout {
        plot: slots.plot,
        slots: slots
            .slots
            .iter()
            .map(|slot| crate::SceneWidgetSlot {
                key: slot.mark_id.clone(),
                target: crate::SceneWidgetTarget::Mark(slot.mark_id.clone()),
                revision: slot.revision,
                placement: slot.placement,
            })
            .collect(),
    };
    scene_widget_paint_plan(scene, &slots, replace_ink, blends)
}

/// Compile real Mark and Label components at their authored source positions.
/// Label replacement omits that Label only; no Mark or datum is fabricated.
/// Component keys are stable caller identities, independent of Label ordinals.
pub fn scene_widget_paint_plan(
    scene: &Scene,
    slots: &crate::SceneWidgetsLayout,
    replace_ink: &BTreeSet<String>,
    blends: &MarkBlends,
) -> Result<WidgetPaintPlan> {
    scene.validate()?;
    blends.validate(scene)?;
    crate::surface::validate_plot(slots.plot, scene)?;
    ensure(
        slots.slots.len() <= crate::MAX_PLOT_WIDGET_SLOTS,
        "Widget paint slot budget exceeded",
    )?;
    let marks: BTreeSet<_> = scene.marks.iter().map(|mark| mark.id.as_str()).collect();
    let mut widgets = BTreeMap::new();
    let mut keys = BTreeSet::new();
    for slot in &slots.slots {
        ensure(
            !slot.key.is_empty() && slot.key.len() <= 1024 && keys.insert(slot.key.as_str()),
            "invalid or duplicate Scene component key",
        )?;
        match &slot.target {
            crate::SceneWidgetTarget::Mark(id) => ensure(
                marks.contains(id.as_str()),
                "Widget paint slot requires a current physical mark",
            )?,
            crate::SceneWidgetTarget::Label(index) => ensure(
                *index < scene.labels.len(),
                "Widget paint slot requires a current label",
            )?,
        }
        ensure(
            widgets
                .insert(slot.target.clone(), slot.key.clone())
                .is_none(),
            "duplicate Scene component source target",
        )?;
        placement_valid(
            slot.placement,
            matches!(slot.target, crate::SceneWidgetTarget::Label(_)),
        )?;
    }
    ensure(
        replace_ink.iter().all(|key| keys.contains(key.as_str())),
        "Widget paint replacement requires a current slot",
    )?;
    let mut source_commands = 0usize;
    for mark in &scene.marks {
        source_commands += match &mark.geometry {
            Geometry::Path(path) => path.len(),
            Geometry::Rect { .. } => 5,
            Geometry::Circle { .. } => 6,
        };
        ensure(
            source_commands <= MAX_WIDGET_PAINT_SOURCE_COMMANDS,
            "Widget paint source command budget exceeded",
        )?;
    }
    let mut nodes = Vec::new();
    let mut pending = Vec::new();
    let mut group: Option<(usize, Vec<WidgetPaintNode>)> = None;
    let mut budget = Budget::default();
    let (starts, ends) = scene.opacity_group_events();
    for (position, item) in scene.draw_items().into_iter().enumerate() {
        if let Some(&index) = starts.get(&position) {
            flush(scene, &mut pending, &mut nodes, &mut budget)?;
            group = Some((index, Vec::new()));
        }
        let target = match &mut group {
            Some((_, children)) => children,
            None => &mut nodes,
        };
        let source_target = match item {
            DrawItem::Label(index) => crate::SceneWidgetTarget::Label(index),
            DrawItem::Mark(index) => crate::SceneWidgetTarget::Mark(scene.marks[index].id.clone()),
        };
        let key = widgets.get(&source_target);
        let replaced = key.is_some_and(|key| replace_ink.contains(key));
        if !replaced {
            let blend = match item {
                DrawItem::Mark(index) => blends.mode(&scene.marks[index].id),
                DrawItem::Label(_) => BlendMode::Normal,
            };
            if blend == BlendMode::Normal {
                pending.push(item);
            } else {
                flush(scene, &mut pending, target, &mut budget)?;
                push_run(scene, vec![item], blend, target, &mut budget)?;
            }
        }
        if let Some(key) = key {
            flush(scene, &mut pending, target, &mut budget)?;
            budget.node()?;
            target.push(WidgetPaintNode::Widget(key.clone()));
        }
        if ends.contains_key(&position)
            && let Some((index, mut children)) = group.take()
        {
            flush(scene, &mut pending, &mut children, &mut budget)?;
            budget.node()?;
            let source = &scene.opacity_groups[index];
            nodes.push(WidgetPaintNode::Group {
                id: source.id.clone(),
                opacity: source.opacity,
                children,
            });
        }
    }
    flush(scene, &mut pending, &mut nodes, &mut budget)?;
    Ok(WidgetPaintPlan {
        width: scene.width,
        height: scene.height,
        background: scene.background,
        nodes,
        widget_count: widgets.len(),
        run_count: budget.runs,
        node_count: budget.nodes,
        source_commands,
    })
}
