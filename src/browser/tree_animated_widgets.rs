//! Full-world retained Tree components driven by the owner's single clock.
//!
//! Children consume actual property Bonds; the host applies camera and plot
//! clipping once and never reruns a placement closure on animation samples.
use glory::{Bond, Cage, Widget};

use super::*;

const XHTML: &str = "http://www.w3.org/1999/xhtml";
const SVG: &str = "http://www.w3.org/2000/svg";

#[derive(Clone, Debug)]
pub enum BrowserTreeAnimatedWidgetData {
    Node(TreeNodeEventData),
    Link(TreeLinkEventData),
    Label(TreeMotionLabelData),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TreeMotionWidgetProps {
    Node(TreeMotionNodeProps),
    Link(TreeMotionLinkProps),
    Label(TreeMotionLabelProps),
}

/// One adopted target plus its sampled pose. Leaving data deliberately belongs
/// to its old immutable source cohort; `data_generation` identifies that cohort.
#[derive(Clone, Debug)]
pub struct BrowserTreeAnimatedWidgetFrame {
    pub key: String,
    pub data: BrowserTreeAnimatedWidgetData,
    pub animated_props: TreeMotionWidgetProps,
    pub phase: TreeMotionPhase,
    pub instance: u64,
    pub revision: u64,
    pub source_generation: u64,
    pub data_generation: u64,
    pub motion: TreeMotionFrame,
    pub occurrence: BrowserTreeMouseOccurrence,
    pub view: ZoomTransform,
    pub hidden: BTreeSet<String>,
    pub scene_size: [f64; 2],
    pub plot: ClipRect,
    /// Hidden-series state follows the real source item's declared series.
    /// Transparent/no-Mark data does not itself hide an explicit custom UI.
    pub visible: bool,
    pub interactive: bool,
}
impl PartialEq for BrowserTreeAnimatedWidgetFrame {
    fn eq(&self, other: &Self) -> bool {
        // The immutable motion snapshot identifies the exact role data too.
        self.motion == other.motion
            && self.key == other.key
            && self.instance == other.instance
            && self.animated_props == other.animated_props
            && self.phase == other.phase
            && self.revision == other.revision
            && self.source_generation == other.source_generation
            && self.data_generation == other.data_generation
            && self.occurrence == other.occurrence
            && self.view == other.view
            && self.hidden == other.hidden
            && self.scene_size == other.scene_size
            && self.plot == other.plot
            && self.visible == other.visible
            && self.interactive == other.interactive
    }
}

/// Both Bonds derive from a single adopted snapshot. `initial` is tentative;
/// the frame starts at None and is published only with the complete Tree stage.
#[derive(Clone)]
pub struct BrowserTreeAnimatedWidgetContext {
    pub key: String,
    pub revision: u64,
    pub phase_instance: u64,
    pub initial: BrowserTreeAnimatedWidgetFrame,
    pub frame: Bond<Option<BrowserTreeAnimatedWidgetFrame>>,
    pub animated_props: Bond<Option<TreeMotionWidgetProps>>,
}

pub(super) type AnimatedFactory =
    Box<dyn FnMut(BrowserTreeAnimatedWidgetContext) -> Result<Box<dyn Widget>>>;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct RecordKey {
    occurrence: String,
    instance: u64,
    revision: u64,
}
struct Record {
    root: HtmlElement,
    web: Rc<WebApp>,
    frame: Cage<Option<BrowserTreeAnimatedWidgetFrame>>,
    retired: Cell<bool>,
    failed: Cell<bool>,
    updated: Cell<bool>,
    live: Rc<Cell<usize>>,
}
impl Record {
    fn retire(&self) -> Option<ChartError> {
        if self.retired.replace(true) {
            return None;
        }
        self.live.set(self.live.get().saturating_sub(1));
        let error = self.web.app().clear().err().map(host);
        self.root.remove();
        error
    }
}
impl Drop for Record {
    fn drop(&mut self) {
        let _ = self.retire();
    }
}
struct Lease {
    records: Vec<Rc<Record>>,
    adopted: Cell<bool>,
}
impl Drop for Lease {
    fn drop(&mut self) {
        if !self.adopted.get() {
            for record in &self.records {
                let _ = record.retire();
            }
        }
    }
}

pub(super) struct AnimatedWidgets {
    document: web_sys::Document,
    options: BrowserTreeWidgetOptions,
    factory: RefCell<Option<AnimatedFactory>>,
    accepted: RefCell<BTreeMap<RecordKey, Rc<Record>>>,
    all: RefCell<Vec<Weak<Record>>>,
    data_generations: RefCell<BTreeMap<u64, u64>>,
    live: Rc<Cell<usize>>,
    alive: Cell<bool>,
    busy: Cell<bool>,
    epoch: Cell<u64>,
}

struct Destination {
    record: Rc<Record>,
    parent: HtmlElement,
    frame: BrowserTreeAnimatedWidgetFrame,
    paint_view: ZoomTransform,
}
struct Undo {
    record: Rc<Record>,
    parent: Option<web_sys::Node>,
    next: Option<web_sys::Node>,
    style: Option<String>,
    inert: Option<String>,
}

pub(super) struct Stage {
    owner: Weak<AnimatedWidgets>,
    epoch: u64,
    records: BTreeMap<RecordKey, Rc<Record>>,
    rows: Vec<Destination>,
    data_generations: BTreeMap<u64, u64>,
    replacement_mark_ids: BTreeSet<String>,
    leases: Vec<Rc<Lease>>,
    undo: RefCell<Vec<Undo>>,
    retirements: RefCell<Vec<Rc<Record>>>,
    placed: Cell<bool>,
    adopted: Cell<bool>,
    focus: RefCell<Option<HtmlElement>>,
}

impl AnimatedWidgets {
    pub(super) fn new(
        document: web_sys::Document,
        options: BrowserTreeWidgetOptions,
        factory: AnimatedFactory,
    ) -> Rc<Self> {
        Rc::new(Self {
            document,
            options,
            factory: RefCell::new(Some(factory)),
            accepted: RefCell::new(BTreeMap::new()),
            all: RefCell::new(Vec::new()),
            data_generations: RefCell::new(BTreeMap::new()),
            live: Rc::new(Cell::new(0)),
            alive: Cell::new(true),
            busy: Cell::new(false),
            epoch: Cell::new(0),
        })
    }
    pub(super) fn live(&self) -> usize {
        self.live.get()
    }
    pub(super) fn estimate(
        &self,
        paint: &TreeMotionPaintFrame,
        plan: &TreeSvgPlan,
        revision: u64,
        prior: Option<&Rc<Stage>>,
    ) -> Result<usize> {
        let previous = prior
            .map(|stage| stage.records.clone())
            .unwrap_or_else(|| self.accepted.borrow().clone());
        let retained = retained_instances(&previous);
        role_count(paint.motion(), plan, &retained)?;
        let count = role_rows(paint.motion(), plan, &retained)
            .into_iter()
            .filter(|(occurrence, instance, ..)| {
                !previous.contains_key(&RecordKey {
                    occurrence: occurrence.layer.key.clone(),
                    instance: *instance,
                    revision,
                })
            })
            .count();
        self.live
            .get()
            .checked_add(count)
            .ok_or_else(|| ChartError("animated Tree component count overflow".into()))
    }
    #[allow(clippy::too_many_arguments)]
    pub(super) fn prepare_target(
        self: &Rc<Self>,
        paint: &TreeMotionPaintFrame,
        model: &SurfaceModel,
        plan: &TreeSvgPlan,
        revision: u64,
        generation: u64,
        publication: u64,
        prior: Option<&Rc<Stage>>,
    ) -> Result<(Rc<Stage>, BTreeMap<String, Element>)> {
        self.prepare(
            paint,
            model,
            plan,
            revision,
            generation,
            publication,
            prior,
            true,
        )
    }
    #[allow(clippy::too_many_arguments)]
    pub(super) fn prepare_sample(
        self: &Rc<Self>,
        paint: &TreeMotionPaintFrame,
        model: &SurfaceModel,
        plan: &TreeSvgPlan,
        revision: u64,
        generation: u64,
        publication: u64,
        prior: Option<&Rc<Stage>>,
    ) -> Result<(Rc<Stage>, BTreeMap<String, Element>)> {
        self.prepare(
            paint,
            model,
            plan,
            revision,
            generation,
            publication,
            prior,
            false,
        )
    }
    #[allow(clippy::too_many_arguments)]
    fn prepare(
        self: &Rc<Self>,
        paint: &TreeMotionPaintFrame,
        model: &SurfaceModel,
        plan: &TreeSvgPlan,
        revision: u64,
        generation: u64,
        publication: u64,
        prior: Option<&Rc<Stage>>,
        construct: bool,
    ) -> Result<(Rc<Stage>, BTreeMap<String, Element>)> {
        scene::ensure(self.alive.get(), "animated Tree components stopped")?;
        scene::ensure(
            self.epoch.get() < u64::MAX,
            "animated Tree component epoch exhausted",
        )?;
        scene::ensure(
            !self.busy.replace(true),
            "animated Tree components are already in use",
        )?;
        struct Busy<'a>(&'a Cell<bool>);
        impl Drop for Busy<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }
        let _busy = Busy(&self.busy);
        scene::ensure(
            self.options.source_ordered,
            "animated Tree components require source order",
        )?;
        scene::ensure(
            *paint.scene() == *model.scene(),
            "animated Tree component Scene differs",
        )?;
        let previous = prior
            .map(|s| s.records.clone())
            .unwrap_or_else(|| self.accepted.borrow().clone());
        let mut data_generations = prior
            .map(|s| s.data_generations.clone())
            .unwrap_or_else(|| self.data_generations.borrow().clone());
        let retained = retained_instances(&previous);
        role_count(paint.motion(), plan, &retained)?;
        let selected = role_rows(paint.motion(), plan, &retained);
        let needed = selected
            .iter()
            .filter(|(occurrence, instance, ..)| {
                !previous.contains_key(&RecordKey {
                    occurrence: occurrence.layer.key.clone(),
                    instance: *instance,
                    revision,
                })
            })
            .count();
        scene::ensure(
            construct || needed == 0,
            "animation sample requires an unprepared component",
        )?;
        let total = self
            .live
            .get()
            .checked_add(needed)
            .ok_or_else(|| ChartError("animated Tree component count overflow".into()))?;
        scene::ensure(
            total <= plan.budget().components && total <= crate::MAX_PLOT_WIDGET_SLOTS,
            "animated Tree live/exit/pending component budget exceeded",
        )?;
        let mut records = BTreeMap::new();
        let mut frames = BTreeMap::new();
        let mut new_records = Vec::new();
        let mut replacement_mark_ids = BTreeSet::new();
        let mut rows = Vec::new();
        let mut elements = BTreeMap::new();
        let result = (|| {
            for (occurrence, instance, key, data, props, phase) in selected {
                let id = RecordKey {
                    occurrence: occurrence.layer.key.clone(),
                    instance,
                    revision,
                };
                if phase == TreeMotionPhase::Live {
                    data_generations.insert(instance, generation);
                }
                let data_generation = *data_generations.get(&instance).ok_or_else(|| {
                    ChartError("outgoing Tree component has no accepted data generation".into())
                })?;
                let visible = match paint.item_for_instance(instance) {
                    Some(DrawItem::Mark(index)) => {
                        !model.hidden().contains(&paint.scene().marks[index].series)
                    }
                    Some(DrawItem::Label(index)) => paint
                        .scene()
                        .label_series
                        .get(&index)
                        .is_none_or(|series| !model.hidden().contains(series)),
                    None => true,
                };
                let frame = BrowserTreeAnimatedWidgetFrame {
                    key,
                    data,
                    animated_props: props,
                    phase,
                    instance,
                    revision,
                    source_generation: generation,
                    data_generation,
                    motion: paint.motion().clone(),
                    occurrence: BrowserTreeMouseOccurrence {
                        key: occurrence.layer.key.clone(),
                        index: occurrence.index,
                        publication,
                    },
                    view: model.view(),
                    hidden: model.hidden().clone(),
                    scene_size: [paint.scene().width, paint.scene().height],
                    plot: paint.target().layout().plot,
                    visible,
                    interactive: visible && phase == TreeMotionPhase::Live,
                };
                let record = if let Some(record) = previous.get(&id) {
                    scene::ensure(
                        !record.retired.get(),
                        "animation sample references a retired component",
                    )?;
                    record.clone()
                } else {
                    let record = self.construct(frame.clone())?;
                    new_records.push(record.clone());
                    record
                };
                frames.insert((occurrence.layer.key.clone(), instance), frame);
                records.insert(id, record);
            }
            for occurrence in plan.layers() {
                let items = match occurrence.layer.kind {
                    TreeSvgLayerKind::Nodes => paint.node_items(),
                    TreeSvgLayerKind::Links => paint.link_items(),
                    TreeSvgLayerKind::Labels => paint.label_items(),
                    _ => continue,
                };
                // A newly added occurrence never owned these outgoing
                // components. Do not turn a deliberately omitted Widget into
                // fresh native ghost ink through the planner's fallback.
                let items: Vec<_> = items
                    .iter()
                    .copied()
                    .filter(|item| {
                        paint.target_item(*item).is_some()
                            || paint.instance_for_item(*item).is_none_or(|instance| {
                                frames.contains_key(&(occurrence.layer.key.clone(), instance))
                            })
                    })
                    .collect();
                let (local, labels) =
                    plot_widgets::svg_occurrences::occurrence_scene(paint.scene(), &items);
                let mut slots = SceneWidgetsLayout {
                    plot: Some(paint.target().layout().plot),
                    slots: Vec::new(),
                };
                let mut replacements = BTreeSet::new();
                let mut used = BTreeSet::new();
                for item in &items {
                    let Some(instance) = paint.instance_for_item(*item) else {
                        continue;
                    };
                    if !frames.contains_key(&(occurrence.layer.key.clone(), instance)) {
                        continue;
                    }
                    let key = format!("motion-{instance}");
                    let target = match *item {
                        DrawItem::Mark(i) => {
                            let id = paint.scene().marks[i].id.clone();
                            if self.options.replace_ink {
                                replacement_mark_ids.insert(id.clone());
                            }
                            SceneWidgetTarget::Mark(id)
                        }
                        DrawItem::Label(i) => SceneWidgetTarget::Label(labels[&i]),
                    };
                    slots.slots.push(SceneWidgetSlot {
                        key: key.clone(),
                        target,
                        revision,
                        placement: PlotWidgetPlacement::Rect(LayoutRect {
                            x: 0.,
                            y: 0.,
                            width: local.width,
                            height: local.height,
                        }),
                    });
                    if self.options.replace_ink {
                        replacements.insert(key);
                    }
                    used.insert(instance);
                }
                let compiled =
                    scene_widget_paint_plan(&local, &slots, &replacements, &MarkBlends::default())?;
                let carrier = self
                    .document
                    .create_element_ns(Some(SVG), "foreignObject")
                    .map_err(host)?;
                carrier
                    .set_attribute(
                        "class",
                        "kartis-tree-svg-source kartis-tree-animated-source",
                    )
                    .map_err(host)?;
                carrier
                    .set_attribute("data-tree-svg-key", &occurrence.layer.key)
                    .map_err(host)?;
                for (name, value) in [
                    ("x", 0.),
                    ("y", 0.),
                    ("width", local.width),
                    ("height", local.height),
                ] {
                    carrier
                        .set_attribute(name, &value.to_string())
                        .map_err(host)?;
                }
                carrier
                    .set_attribute("style", "overflow:hidden;pointer-events:none")
                    .map_err(host)?;
                let content = html(&self.document)?;
                content
                    .set_attribute("class", "kartis-tree-animated-content")
                    .map_err(host)?;
                // These are fixed paint viewports, never scroll containers.
                // Focusing a partly clipped child must not pan its world pose.
                content.set_attribute("style", "position:relative;width:100%;height:100%;pointer-events:none;isolation:isolate;overflow:clip").map_err(host)?;
                carrier.append_child(&content).map_err(host)?;
                self.render_nodes(
                    compiled.nodes(),
                    &content,
                    occurrence,
                    &frames,
                    &records,
                    model,
                    &mut rows,
                )?;
                // An explicitly requested component may represent a zero/no-Mark
                // logical row. It is UI, never fabricated native source ink.
                for ((key, instance), frame) in &frames {
                    if key == &occurrence.layer.key && !used.contains(instance) {
                        let record = &records[&RecordKey {
                            occurrence: key.clone(),
                            instance: *instance,
                            revision,
                        }];
                        append_destination(
                            &self.document,
                            &content,
                            record.clone(),
                            frame.clone(),
                            model.paint_transform(),
                            &mut rows,
                        )?;
                    }
                }
                elements.insert(occurrence.layer.key.clone(), carrier);
            }
            scene::ensure(
                self.alive.get(),
                "Tree stopped during animated component construction",
            )?;
            Ok(())
        })();
        if let Err(error) = result {
            for record in new_records {
                let _ = record.retire();
            }
            return Err(error);
        }
        let wanted_instances: BTreeSet<_> = frames.keys().map(|(_, instance)| *instance).collect();
        data_generations.retain(|instance, _| wanted_instances.contains(instance));
        // Only queued, unadopted candidates need the shared construction
        // lease. Accepted records are owned by the accepted map; copying their
        // old leases on every sample would retain history indefinitely.
        let mut leases = prior
            .filter(|stage| !stage.was_adopted())
            .map(|stage| stage.leases.clone())
            .unwrap_or_default();
        if !new_records.is_empty() {
            leases.push(Rc::new(Lease {
                records: new_records,
                adopted: Cell::new(false),
            }));
        }
        let stage = Rc::new(Stage {
            owner: Rc::downgrade(self),
            epoch: self.epoch.get(),
            records,
            rows,
            data_generations,
            replacement_mark_ids,
            leases,
            undo: RefCell::new(Vec::new()),
            retirements: RefCell::new(Vec::new()),
            placed: Cell::new(false),
            adopted: Cell::new(false),
            focus: RefCell::new(None),
        });
        Ok((stage, elements))
    }
    #[allow(clippy::too_many_arguments)]
    fn render_nodes(
        &self,
        nodes: &[WidgetPaintNode],
        parent: &HtmlElement,
        occurrence: &TreeSvgOccurrence,
        frames: &BTreeMap<(String, u64), BrowserTreeAnimatedWidgetFrame>,
        records: &BTreeMap<RecordKey, Rc<Record>>,
        model: &SurfaceModel,
        rows: &mut Vec<Destination>,
    ) -> Result<()> {
        for node in nodes {
            match node {
                WidgetPaintNode::Paint(run) => {
                    let view = svg::native_view(
                        &self.document,
                        &run.scene().without_series(model.hidden()),
                        model.paint_transform(),
                        &model.source_ink_ids(),
                    )?;
                    view.set_attribute("style", "position:absolute;left:0;top:0;width:100%;height:100%;overflow:hidden;pointer-events:none").map_err(host)?;
                    parent.append_child(&view).map_err(host)?;
                }
                WidgetPaintNode::Widget(key) => {
                    let instance = key
                        .strip_prefix("motion-")
                        .and_then(|v| v.parse::<u64>().ok())
                        .ok_or_else(|| ChartError("invalid animated component paint key".into()))?;
                    let frame = &frames[&(occurrence.layer.key.clone(), instance)];
                    let record = &records[&RecordKey {
                        occurrence: occurrence.layer.key.clone(),
                        instance,
                        revision: frame.revision,
                    }];
                    append_destination(
                        &self.document,
                        parent,
                        record.clone(),
                        frame.clone(),
                        model.paint_transform(),
                        rows,
                    )?;
                }
                WidgetPaintNode::Group {
                    opacity, children, ..
                } => {
                    let group = html(&self.document)?;
                    group.set_attribute("style", &format!("position:absolute;left:0;top:0;width:100%;height:100%;pointer-events:none;opacity:{opacity}")).map_err(host)?;
                    parent.append_child(&group).map_err(host)?;
                    self.render_nodes(children, &group, occurrence, frames, records, model, rows)?;
                }
            }
        }
        Ok(())
    }
    fn construct(self: &Rc<Self>, initial: BrowserTreeAnimatedWidgetFrame) -> Result<Rc<Record>> {
        self.live.set(self.live.get() + 1);
        let built = (|| {
            let root = html(&self.document)?;
            root.set_attribute(
                "class",
                "kartis-tree-animated-widget kartis-plot-widget-slot",
            )
            .map_err(host)?;
            root.set_attribute("data-tree-motion-key", &initial.key)
                .map_err(host)?;
            root.set_attribute("data-tree-motion-instance", &initial.instance.to_string())
                .map_err(host)?;
            root.set_attribute("data-tree-svg-key", &initial.occurrence.key)
                .map_err(host)?;
            root.set_attribute("style", &format!("position:absolute;left:0;top:0;width:{}px;height:{}px;pointer-events:none;transform-origin:0 0", initial.scene_size[0], initial.scene_size[1])).map_err(host)?;
            let captured = Rc::new(RefCell::new(None));
            let output = captured.clone();
            let owner = Rc::downgrade(self);
            let web = glory::web::launch_with(root.clone().into(), move |scope| {
                let frame = scope.cage(None::<BrowserTreeAnimatedWidgetFrame>);
                *output.borrow_mut() = Some(frame);
                glory::ui::widget(move |scope| {
                    let bond = scope.bond(move || frame.get());
                    let animated_props = scope.bond(move || {
                        frame.with(|value| value.as_ref().map(|value| value.animated_props))
                    });
                    let owner = owner.upgrade().ok_or_else(|| {
                        glory::render::Error::Backend(
                            "animated Tree component owner retired".into(),
                        )
                    })?;
                    let mut factory = owner
                        .factory
                        .try_borrow_mut()
                        .map_err(|_| {
                            glory::render::Error::Backend(
                                "animated Tree factory is already in use".into(),
                            )
                        })?
                        .take()
                        .ok_or_else(|| {
                            glory::render::Error::Backend(
                                "animated Tree factory is already in use".into(),
                            )
                        })?;
                    let result = factory(BrowserTreeAnimatedWidgetContext {
                        key: initial.key.clone(),
                        revision: initial.revision,
                        phase_instance: initial.instance,
                        initial: initial.clone(),
                        frame: bond,
                        animated_props,
                    });
                    if owner.alive.get() {
                        *owner.factory.borrow_mut() = Some(factory);
                    }
                    let widget = result.map_err(|e| glory::render::Error::Backend(e.0))?;
                    if !owner.alive.get() {
                        return Err(glory::render::Error::Backend(
                            "Tree stopped during animated factory".into(),
                        )
                        .into());
                    }
                    widget.show_in(scope)
                })
            })
            .map_err(host)?;
            let frame = captured
                .borrow_mut()
                .take()
                .ok_or_else(|| ChartError("animated Tree component did not initialize".into()))?;
            Ok(Rc::new(Record {
                root,
                web: Rc::new(web),
                frame,
                retired: Cell::new(false),
                failed: Cell::new(false),
                updated: Cell::new(true),
                live: self.live.clone(),
            }))
        })();
        match built {
            Ok(record) => {
                self.all.borrow_mut().retain(|r| r.strong_count() > 0);
                self.all.borrow_mut().push(Rc::downgrade(&record));
                Ok(record)
            }
            Err(error) => {
                self.live.set(self.live.get().saturating_sub(1));
                Err(error)
            }
        }
    }
    pub(super) fn stop(&self) {
        if !self.alive.replace(false) {
            return;
        }
        self.factory.borrow_mut().take();
        for record in self.all.borrow_mut().drain(..).filter_map(|r| r.upgrade()) {
            let _ = record.retire();
        }
        self.accepted.borrow_mut().clear();
        self.data_generations.borrow_mut().clear();
    }
}

type RoleRow<'a> = (
    &'a TreeSvgOccurrence,
    u64,
    String,
    BrowserTreeAnimatedWidgetData,
    TreeMotionWidgetProps,
    TreeMotionPhase,
);
type RetainedInstances = BTreeMap<String, BTreeSet<u64>>;
fn retained_instances(records: &BTreeMap<RecordKey, Rc<Record>>) -> RetainedInstances {
    let mut retained = RetainedInstances::new();
    for (key, record) in records {
        if !record.retired.get() {
            retained
                .entry(key.occurrence.clone())
                .or_default()
                .insert(key.instance);
        }
    }
    retained
}
fn role_allowed(
    instance: u64,
    phase: TreeMotionPhase,
    occurrence: &str,
    retained: &RetainedInstances,
) -> bool {
    phase == TreeMotionPhase::Live
        || retained
            .get(occurrence)
            .is_some_and(|instances| instances.contains(&instance))
}
fn role_count(
    motion: &TreeMotionFrame,
    plan: &TreeSvgPlan,
    retained: &RetainedInstances,
) -> Result<usize> {
    let mut count = 0usize;
    for occurrence in plan.layers() {
        let allowed =
            |instance, phase| role_allowed(instance, phase, &occurrence.layer.key, retained);
        let next = match occurrence.layer.kind {
            TreeSvgLayerKind::Nodes => motion
                .nodes()
                .iter()
                .filter(|row| allowed(row.instance(), row.phase()))
                .count(),
            TreeSvgLayerKind::Links => motion
                .links()
                .iter()
                .filter(|row| {
                    row.data().frame().layout().options().style.links.enabled
                        && allowed(row.instance(), row.phase())
                })
                .count(),
            TreeSvgLayerKind::Labels => motion
                .labels()
                .iter()
                .filter(|row| allowed(row.instance(), row.phase()))
                .count(),
            _ => 0,
        };
        count = count
            .checked_add(next)
            .ok_or_else(|| ChartError("animated Tree component count overflow".into()))?;
        scene::ensure(
            count <= crate::MAX_PLOT_WIDGET_SLOTS && count <= plan.budget().components,
            "animated Tree live/exit/pending component budget exceeded",
        )?;
    }
    Ok(count)
}
fn role_rows<'a>(
    motion: &TreeMotionFrame,
    plan: &'a TreeSvgPlan,
    retained: &RetainedInstances,
) -> Vec<RoleRow<'a>> {
    let mut rows = Vec::new();
    for occurrence in plan.layers() {
        match occurrence.layer.kind {
            TreeSvgLayerKind::Nodes => rows.extend(
                motion
                    .nodes()
                    .iter()
                    .filter(|row| {
                        role_allowed(row.instance(), row.phase(), &occurrence.layer.key, retained)
                    })
                    .map(|row| {
                        (
                            occurrence,
                            row.instance(),
                            format!("node:{}", row.key()),
                            BrowserTreeAnimatedWidgetData::Node(row.data().clone()),
                            TreeMotionWidgetProps::Node(*row.props()),
                            row.phase(),
                        )
                    }),
            ),
            TreeSvgLayerKind::Links => rows.extend(
                motion
                    .links()
                    .iter()
                    .filter(|row| {
                        row.data().frame().layout().options().style.links.enabled
                            && role_allowed(
                                row.instance(),
                                row.phase(),
                                &occurrence.layer.key,
                                retained,
                            )
                    })
                    .map(|row| {
                        (
                            occurrence,
                            row.instance(),
                            format!("link:{}", row.key().target_id),
                            BrowserTreeAnimatedWidgetData::Link(row.data().clone()),
                            TreeMotionWidgetProps::Link(*row.props()),
                            row.phase(),
                        )
                    }),
            ),
            TreeSvgLayerKind::Labels => rows.extend(
                motion
                    .labels()
                    .iter()
                    .filter(|row| {
                        role_allowed(row.instance(), row.phase(), &occurrence.layer.key, retained)
                    })
                    .map(|row| {
                        (
                            occurrence,
                            row.instance(),
                            format!("label:{}", row.key()),
                            BrowserTreeAnimatedWidgetData::Label(row.data().clone()),
                            TreeMotionWidgetProps::Label(*row.props()),
                            row.phase(),
                        )
                    }),
            ),
            _ => {}
        }
    }
    rows
}
fn html(document: &web_sys::Document) -> Result<HtmlElement> {
    document
        .create_element_ns(Some(XHTML), "div")
        .map_err(host)?
        .dyn_into()
        .map_err(host)
}
fn append_destination(
    document: &web_sys::Document,
    parent: &HtmlElement,
    record: Rc<Record>,
    frame: BrowserTreeAnimatedWidgetFrame,
    paint_view: ZoomTransform,
    rows: &mut Vec<Destination>,
) -> Result<()> {
    let clip = html(document)?;
    clip.set_attribute("class", "kartis-tree-animated-clip")
        .map_err(host)?;
    clip.set_attribute("style", &format!("position:absolute;left:{}px;top:{}px;width:{}px;height:{}px;overflow:clip;pointer-events:none", frame.plot.x, frame.plot.y, frame.plot.width, frame.plot.height)).map_err(host)?;
    parent.append_child(&clip).map_err(host)?;
    rows.push(Destination {
        record,
        parent: clip,
        frame,
        paint_view,
    });
    Ok(())
}

impl Stage {
    pub(super) fn was_adopted(&self) -> bool {
        self.adopted.get()
    }
    pub(super) fn replacement_mark_ids(&self) -> &BTreeSet<String> {
        &self.replacement_mark_ids
    }
    pub(super) fn place(&self) -> Result<()> {
        if self.placed.get() {
            return Ok(());
        }
        if let Some(owner) = self.owner.upgrade()
            && let Some(focus) = owner.document.active_element()
            && self
                .rows
                .iter()
                .any(|row| row.record.root.contains(Some(&focus)))
        {
            *self.focus.borrow_mut() = focus.dyn_into::<HtmlElement>().ok();
        }
        for row in &self.rows {
            let root = &row.record.root;
            self.undo.borrow_mut().push(Undo {
                record: row.record.clone(),
                parent: root.parent_node(),
                next: root.next_sibling(),
                style: root.get_attribute("style"),
                inert: root.get_attribute("inert"),
            });
            let result = (|| {
                root.set_attribute("style", &format!("position:absolute;left:{}px;top:{}px;width:{}px;height:{}px;pointer-events:none;transform-origin:0 0;transform:matrix({},0,0,{},{},{})", -row.frame.plot.x, -row.frame.plot.y, row.frame.scene_size[0], row.frame.scene_size[1], row.paint_view.scale, row.paint_view.scale, row.paint_view.translate_x, row.paint_view.translate_y)).map_err(host)?;
                let theme = row.frame.motion.target().theme();
                let font = theme.font.resolve(&ResolvedFont::default());
                root.style()
                    .set_property("font-family", &font.css_family())
                    .map_err(host)?;
                root.style()
                    .set_property("font-size", &format!("{}px", theme.font_size))
                    .map_err(host)?;
                root.style()
                    .set_property("font-weight", &font.weight.to_string())
                    .map_err(host)?;
                root.style()
                    .set_property("color", &render::css_color(theme.text))
                    .map_err(host)?;
                if !row.frame.visible {
                    root.style().set_property("display", "none").map_err(host)?;
                }
                if !row.frame.interactive {
                    root.set_attribute("inert", "").map_err(host)?;
                } else {
                    root.remove_attribute("inert").map_err(host)?;
                }
                move_root(&row.parent, root)?;
                Ok(())
            })();
            if let Err(error) = result {
                self.rollback()?;
                return Err(error);
            }
        }
        self.placed.set(true);
        self.validate()
    }
    pub(super) fn validate(&self) -> Result<()> {
        let owner = self
            .owner
            .upgrade()
            .ok_or_else(|| ChartError("animated Tree component owner retired".into()))?;
        scene::ensure(
            owner.alive.get() && owner.epoch.get() == self.epoch && !self.adopted.get(),
            "animated Tree component stage is stale",
        )?;
        scene::ensure(
            self.placed.get()
                && self.rows.iter().all(|row| {
                    row.record
                        .root
                        .parent_node()
                        .is_some_and(|p| p.is_same_node(Some(row.parent.as_ref())))
                }),
            "animated Tree component placement is incomplete",
        )
    }
    pub(super) fn rollback(&self) -> Result<()> {
        if self.adopted.get() {
            return Ok(());
        }
        let mut errors = Vec::new();
        for undo in self.undo.borrow_mut().drain(..).rev() {
            let root = &undo.record.root;
            let result = match undo.parent {
                Some(parent) => parent
                    .insert_before(root, undo.next.as_ref())
                    .map(|_| ())
                    .map_err(host),
                None => {
                    root.remove();
                    Ok(())
                }
            };
            if let Err(e) = result {
                errors.push(e.0);
            }
            for (name, value) in [("style", undo.style), ("inert", undo.inert)] {
                let result = match value {
                    Some(value) => root.set_attribute(name, &value),
                    None => root.remove_attribute(name),
                };
                if let Err(e) = result {
                    errors.push(format!("{e:?}"));
                }
            }
        }
        self.placed.set(false);
        if let Some(focus) = self.focus.borrow().as_ref()
            && focus.is_connected()
            && let Err(error) = focus.focus()
        {
            errors.push(host(error).0);
        }
        scene::ensure(errors.is_empty(), &errors.join("; "))
    }
    pub(super) fn adopt(&self) {
        assert!(
            !self.adopted.replace(true),
            "animated Tree component stage adopted twice"
        );
        let owner = self.owner.upgrade().expect("validated animated owner");
        for lease in &self.leases {
            lease.adopted.set(true);
        }
        for row in &self.rows {
            let next = Some(row.frame.clone());
            if row.record.frame.get_untracked() != next {
                row.record.updated.set(true);
                row.record.frame.revise(|value| *value = next);
            }
        }
        let previous = owner.accepted.replace(self.records.clone());
        let mut retired = self.retirements.borrow_mut();
        for record in previous
            .values()
            .chain(self.leases.iter().flat_map(|l| l.records.iter()))
        {
            if !self.records.values().any(|live| Rc::ptr_eq(live, record))
                && !retired.iter().any(|old| Rc::ptr_eq(old, record))
            {
                retired.push(record.clone());
            }
        }
        *owner.data_generations.borrow_mut() = self.data_generations.clone();
        owner.epoch.set(
            owner
                .epoch
                .get()
                .checked_add(1)
                .expect("validated animation epoch"),
        );
        self.undo.borrow_mut().clear();
    }
    pub(super) fn finish(&self) -> Vec<ChartError> {
        let mut errors = Vec::new();
        for record in self.retirements.borrow_mut().drain(..) {
            if let Some(e) = record.retire() {
                errors.push(e);
            }
        }
        for record in self.records.values() {
            if record.retired.get() {
                continue;
            }
            let updated = record.updated.replace(false);
            let pumped = record.web.app().pump();
            if record.retired.get() || !self.owner.upgrade().is_some_and(|owner| owner.alive.get())
            {
                break;
            }
            if let Err(error) = pumped {
                errors.push(host(error));
                record.failed.set(true);
                if let Err(error) = record.root.style().set_property("display", "none") {
                    errors.push(host(error));
                }
            } else if updated || !record.failed.get() {
                let visible = record
                    .frame
                    .get_untracked()
                    .is_some_and(|frame| frame.visible);
                let result = if visible {
                    record.root.style().remove_property("display").map(|_| ())
                } else {
                    record.root.style().set_property("display", "none")
                };
                if let Err(error) = result {
                    errors.push(host(error));
                } else {
                    record.failed.set(false);
                }
            }
        }
        if let Some(focus) = self.focus.borrow_mut().take()
            && focus.is_connected()
            && self
                .rows
                .iter()
                .any(|row| row.frame.interactive && row.record.root.contains(Some(&focus)))
            && let Err(error) = focus.focus()
        {
            errors.push(host(error));
        }
        errors
    }
}
impl Drop for Stage {
    fn drop(&mut self) {
        if !self.adopted.get() {
            let _ = self.rollback();
        }
    }
}

fn move_root(parent: &HtmlElement, root: &HtmlElement) -> Result<()> {
    // Prefer the platform's state-preserving owned-carrier move. Moving this
    // host never changes any child's Glory placement bookkeeping.
    if let Ok(function) = web_sys::js_sys::Reflect::get(parent.as_ref(), &"moveBefore".into())
        && let Some(function) = function.dyn_ref::<web_sys::js_sys::Function>()
        && function
            .call2(
                parent.as_ref(),
                root.as_ref(),
                &web_sys::wasm_bindgen::JsValue::NULL,
            )
            .is_ok()
    {
        return Ok(());
    }
    parent.append_child(root).map_err(host)?;
    Ok(())
}
