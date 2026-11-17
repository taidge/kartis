//! Occurrence-local strict managers. Public component keys and data stay canonical.
use super::*;
use crate::{DrawItem, TreeSvgOccurrence, TreeSvgPlan};

type SharedFactory = Rc<RefCell<Option<PlotWidgetFactory>>>;
struct Carrier {
    element: Element,
    host: Rc<Host>,
    specs: RefCell<BTreeMap<String, (u64, bool)>>,
    retired: Cell<bool>,
    hidden_widgets: RefCell<Vec<(Element, Option<String>)>>,
}
impl Carrier {
    fn retire(&self) {
        if !self.retired.replace(true) {
            self.host.stop();
            self.element.remove();
        }
    }
    fn hide_widgets(&self) {
        if !self.hidden_widgets.borrow().is_empty() {
            return;
        }
        let mut stack = Vec::new();
        if let Some(root) = self.element.first_element_child() {
            stack.push(root);
        }
        while let Some(node) = stack.pop() {
            if node.get_attribute("class").is_some_and(|c| {
                c.split_ascii_whitespace()
                    .any(|c| c == "kartis-plot-widget-clip")
            }) {
                self.hidden_widgets
                    .borrow_mut()
                    .push((node.clone(), node.get_attribute("style")));
                if let Some(html) = node.dyn_ref::<HtmlElement>() {
                    let _ = html.style().set_property("display", "none");
                }
            } else {
                let mut child = node.first_element_child();
                while let Some(next) = child {
                    child = next.next_element_sibling();
                    stack.push(next);
                }
            }
        }
    }
}
impl Drop for Carrier {
    fn drop(&mut self) {
        self.retire();
    }
}
pub(crate) struct Sources {
    document: web_sys::Document,
    factory: SharedFactory,
    alive: Rc<Cell<bool>>,
    live: Rc<Cell<usize>>,
    data_type: Option<std::any::TypeId>,
    accepted: RefCell<BTreeMap<String, Rc<Carrier>>>,
    all: RefCell<Vec<Weak<Carrier>>>,
}
struct Row {
    key: String,
    carrier: Rc<Carrier>,
    frame: OrderedFrame,
    specs: BTreeMap<String, (u64, bool)>,
    size: [f64; 2],
}
type PreviousSizeAttributes = (Element, Option<String>, Option<String>);
pub(crate) struct Stage {
    source: Weak<Sources>,
    rows: Vec<Row>,
    placements: RefCell<Vec<super::super::ordered_widgets::Placement>>,
    retirements: RefCell<Vec<super::super::ordered_widgets::Retirement>>,
    retired: RefCell<Vec<Rc<Carrier>>>,
    sizes: RefCell<Vec<PreviousSizeAttributes>>,
    restored_widgets: RefCell<Vec<(Element, Option<String>)>>,
    adopted: Cell<bool>,
}
impl Sources {
    pub(crate) fn new(document: web_sys::Document, mount: Mount) -> Rc<Self> {
        Rc::new(Self {
            document,
            data_type: mount.layout.data_type(),
            factory: Rc::new(RefCell::new(Some(mount.factory))),
            alive: Rc::new(Cell::new(true)),
            live: Rc::new(Cell::new(0)),
            accepted: RefCell::new(BTreeMap::new()),
            all: RefCell::new(Vec::new()),
        })
    }
    pub(crate) fn live(&self) -> usize {
        self.live.get()
    }
    pub(crate) fn validate_packet(&self, packet: &PlotWidgetsPacket) -> Result<()> {
        packet.validate()?;
        scene::ensure(self.alive.get(), "Tree SVG source components stopped")?;
        scene::ensure(
            packet.is_composition() && packet.is_source_ordered(),
            "Tree SVG source components require source-ordered strict composition",
        )?;
        scene::ensure(
            packet.data_type() == self.data_type,
            "Tree SVG source component payload type changed",
        )
    }
    pub(crate) fn estimate(&self, plan: &TreeSvgPlan, packet: &PlotWidgetsPacket) -> Result<usize> {
        self.validate_packet(packet)?;
        let layout = packet.scene_layout();
        let accepted = self.accepted.borrow();
        let mut needed = 0usize;
        for occurrence in plan.layers() {
            let selected = selected_slots(plan, occurrence, &layout);
            let old = accepted.get(&occurrence.layer.key);
            for slot in selected {
                let same = old.is_some_and(|old| {
                    old.specs.borrow().get(&slot.key)
                        == Some(&(
                            slot.revision,
                            matches!(slot.target, SceneWidgetTarget::Label(_)),
                        ))
                });
                needed += usize::from(!same);
            }
        }
        self.live
            .get()
            .checked_add(needed)
            .ok_or_else(|| ChartError("Tree SVG source component count overflow".into()))
    }
    fn carrier(&self, key: &str, scene: &Scene, hidden: &BTreeSet<String>) -> Result<Rc<Carrier>> {
        if let Some(old) = self.accepted.borrow().get(key) {
            return Ok(old.clone());
        }
        const SVG: &str = "http://www.w3.org/2000/svg";
        let element = self
            .document
            .create_element_ns(Some(SVG), "foreignObject")
            .map_err(host)?;
        element
            .set_attribute("class", "kartis-tree-svg-source")
            .map_err(host)?;
        element
            .set_attribute("data-tree-svg-key", key)
            .map_err(host)?;
        for (n, v) in [
            ("x", "0"),
            ("y", "0"),
            ("style", "overflow:hidden;pointer-events:none"),
        ] {
            element.set_attribute(n, v).map_err(host)?;
        }
        element
            .set_attribute("width", &scene.width.to_string())
            .map_err(host)?;
        element
            .set_attribute("height", &scene.height.to_string())
            .map_err(host)?;
        let root = self
            .document
            .create_element_ns(Some("http://www.w3.org/1999/xhtml"), "div")
            .map_err(host)?;
        root.set_attribute("class", "kartis-tree-svg-source-content")
            .map_err(host)?;
        root.set_attribute("data-tree-svg-key", key).map_err(host)?;
        root.set_attribute(
            "style",
            "position:relative;width:100%;height:100%;pointer-events:none;overflow:hidden",
        )
        .map_err(host)?;
        element.append_child(&root).map_err(host)?;
        let shared = self.factory.clone();
        let alive = self.alive.clone();
        let live = self.live.clone();
        let factory: PlotWidgetFactory = Box::new(move |context| {
            scene::ensure(alive.get(), "Tree SVG source components stopped")?;
            let next = live
                .get()
                .checked_add(1)
                .ok_or_else(|| ChartError("Tree SVG source count overflow".into()))?;
            scene::ensure(
                next <= crate::MAX_TREE_SVG_COMPONENTS,
                "Tree SVG source live/pending budget exceeded",
            )?;
            live.set(next);
            let cleanup = live.clone();
            if let Err(error) = context
                .scope
                .on_cleanup(move || cleanup.set(cleanup.get().saturating_sub(1)))
            {
                live.set(live.get().saturating_sub(1));
                return Err(ChartError(error.to_string()));
            }
            let mut factory = shared
                .try_borrow_mut()
                .map_err(|_| ChartError("Tree SVG source factory is already in use".into()))?
                .take()
                .ok_or_else(|| ChartError("Tree SVG source factory is already in use".into()))?;
            let result = factory(context);
            if alive.get() {
                *shared.borrow_mut() = Some(factory);
            }
            scene::ensure(
                alive.get(),
                "Tree SVG stopped during source component construction",
            )?;
            result
        });
        let host = Host::launch(
            root,
            scene,
            factory,
            Prepared {
                rows: Vec::new(),
                hidden: hidden.clone(),
                composed: true,
                ordered: true,
            },
        )?;
        let carrier = Rc::new(Carrier {
            element,
            host: Rc::new(host),
            specs: RefCell::new(BTreeMap::new()),
            retired: Cell::new(false),
            hidden_widgets: RefCell::new(Vec::new()),
        });
        self.all.borrow_mut().retain(|c| c.strong_count() > 0);
        self.all.borrow_mut().push(Rc::downgrade(&carrier));
        Ok(carrier)
    }
    pub(crate) fn prepare(
        self: &Rc<Self>,
        plan: &TreeSvgPlan,
        packet: &PlotWidgetsPacket,
        model: &SurfaceModel,
        ratio: f64,
        language: Option<String>,
        prior: Option<&Rc<Stage>>,
    ) -> Result<(Rc<Stage>, BTreeMap<String, Element>)> {
        self.validate_packet(packet)?;
        let canonical = super::prepare(model, packet, true)?;
        let layout = packet.scene_layout();
        let mut rows = Vec::new();
        let mut elements = BTreeMap::new();
        for occurrence in plan.layers() {
            let selected = selected_slots(plan, occurrence, &layout);
            if selected.is_empty() {
                continue;
            }
            let keys: BTreeSet<_> = selected.iter().map(|s| s.key.as_str()).collect();
            let rows_for_occurrence = layout
                .slots
                .iter()
                .zip(&canonical.rows)
                .filter(|(slot, _)| keys.contains(slot.key.as_str()))
                .map(|(_, row)| row.clone())
                .collect();
            let carrier = prior
                .and_then(|s| {
                    s.rows
                        .iter()
                        .find(|r| r.key == occurrence.layer.key)
                        .map(|r| r.carrier.clone())
                })
                .map(Ok)
                .unwrap_or_else(|| {
                    self.carrier(&occurrence.layer.key, model.scene(), model.hidden())
                })?;
            let (scene, labels) = occurrence_scene(model.scene(), occurrence.items());
            let mut local = SceneWidgetsLayout {
                plot: layout.plot,
                slots: selected.into_iter().cloned().collect(),
            };
            for slot in &mut local.slots {
                if let SceneWidgetTarget::Label(index) = &mut slot.target {
                    *index = labels[index];
                }
            }
            let request = super::super::ordered_widgets::Request {
                scene,
                slots: local,
                replacements: packet
                    .replacement_keys()
                    .iter()
                    .filter(|k| keys.contains(k.as_str()))
                    .cloned()
                    .collect(),
                modes: MarkBlends::default(),
                hidden: model.hidden().clone(),
                view: model.paint_transform(),
                ratio,
                renderer: BrowserRenderer::Svg,
                tree_labels_passive: true,
                helpers: Vec::new(),
                language: language.clone(),
                source_hit_ids: Some(model.source_ink_ids()),
            };
            let prepared = Prepared {
                rows: rows_for_occurrence,
                hidden: model.hidden().clone(),
                composed: true,
                ordered: true,
            };
            let staged = prior.filter(|s| !s.adopted.get()).and_then(|s| {
                s.rows
                    .iter()
                    .find(|r| r.key == occurrence.layer.key)
                    .map(|r| r.frame.widgets.clone())
            });
            let frame = carrier.host.prepare_ordered(request, prepared, staged)?;
            let specs = layout
                .slots
                .iter()
                .filter(|s| keys.contains(s.key.as_str()))
                .map(|s| {
                    (
                        s.key.clone(),
                        (s.revision, matches!(s.target, SceneWidgetTarget::Label(_))),
                    )
                })
                .collect();
            elements.insert(occurrence.layer.key.clone(), carrier.element.clone());
            rows.push(Row {
                key: occurrence.layer.key.clone(),
                carrier,
                frame,
                specs,
                size: [model.scene().width, model.scene().height],
            });
        }
        Ok((
            Rc::new(Stage {
                source: Rc::downgrade(self),
                rows,
                placements: RefCell::new(Vec::new()),
                retirements: RefCell::new(Vec::new()),
                retired: RefCell::new(Vec::new()),
                sizes: RefCell::new(Vec::new()),
                restored_widgets: RefCell::new(Vec::new()),
                adopted: Cell::new(false),
            }),
            elements,
        ))
    }
    pub(crate) fn stop(&self) {
        self.alive.set(false);
        self.factory.borrow_mut().take();
        let old = std::mem::take(&mut *self.accepted.borrow_mut());
        let all = std::mem::take(&mut *self.all.borrow_mut());
        for carrier in all.into_iter().filter_map(|c| c.upgrade()) {
            carrier.retire();
        }
        drop(old);
    }
}
impl Stage {
    pub(crate) fn place(&self) -> Result<()> {
        for row in &self.rows {
            for (widget, style) in row.carrier.hidden_widgets.borrow().iter() {
                self.restored_widgets
                    .borrow_mut()
                    .push((widget.clone(), widget.get_attribute("style")));
                match style {
                    Some(style) => widget.set_attribute("style", style).map_err(host)?,
                    None => widget.remove_attribute("style").map_err(host)?,
                }
            }
            let element = &row.carrier.element;
            self.sizes.borrow_mut().push((
                element.clone(),
                element.get_attribute("width"),
                element.get_attribute("height"),
            ));
            element
                .set_attribute("width", &row.size[0].to_string())
                .map_err(host)?;
            element
                .set_attribute("height", &row.size[1].to_string())
                .map_err(host)?;
            let host = &row.carrier.host;
            host.masks.publish(&row.frame.rows)?;
            host.publishing.set(true);
            let result = host.ordered.as_ref().expect("ordered source").place(
                host.composition.as_ref().expect("source manager"),
                &row.frame.widgets,
                row.frame.paint.clone(),
            );
            match result {
                Ok(p) => self.placements.borrow_mut().push(p),
                Err(error) => {
                    self.rollback()?;
                    return Err(error);
                }
            }
        }
        self.validate()
    }
    pub(crate) fn validate(&self) -> Result<()> {
        let source = self
            .source
            .upgrade()
            .ok_or_else(|| ChartError("Tree SVG source owner retired".into()))?;
        scene::ensure(source.alive.get(), "Tree SVG source owner retired")?;
        scene::ensure(
            self.placements.borrow().len() == self.rows.len(),
            "Tree SVG source placement is incomplete",
        )?;
        for p in self.placements.borrow().iter() {
            p.validate()?;
        }
        Ok(())
    }
    pub(crate) fn rollback(&self) -> Result<()> {
        if self.adopted.get() {
            return Ok(());
        }
        for p in self.placements.borrow().iter().rev() {
            p.rollback()?;
        }
        for row in &self.rows {
            row.carrier
                .host
                .masks
                .publish(&row.carrier.host.accepted_masks.borrow())?;
            row.carrier.host.publishing.set(false);
        }
        self.placements.borrow_mut().clear();
        for (widget, style) in self.restored_widgets.borrow_mut().drain(..).rev() {
            match style {
                Some(style) => widget.set_attribute("style", &style).map_err(host)?,
                None => widget.remove_attribute("style").map_err(host)?,
            }
        }
        for (element, w, h) in self.sizes.borrow_mut().drain(..).rev() {
            for (name, value) in [("width", w), ("height", h)] {
                match value {
                    Some(value) => element.set_attribute(name, &value).map_err(host)?,
                    None => element.remove_attribute(name).map_err(host)?,
                }
            }
        }
        Ok(())
    }
    pub(crate) fn adopt(&self) {
        assert!(!self.adopted.replace(true), "Tree SVG source adopted twice");
        let source = self.source.upgrade().expect("validated source owner");
        for (row, p) in self.rows.iter().zip(self.placements.borrow().iter()) {
            self.retirements.borrow_mut().push(
                row.carrier
                    .host
                    .ordered
                    .as_ref()
                    .expect("source paint")
                    .adopt_placed(p, &mut || {}),
            );
            *row.carrier.host.accepted_masks.borrow_mut() = row.frame.rows.clone();
            row.carrier.host.publishing.set(false);
            *row.carrier.specs.borrow_mut() = row.specs.clone();
            row.carrier.hidden_widgets.borrow_mut().clear();
        }
        let next: BTreeMap<_, _> = self
            .rows
            .iter()
            .map(|r| (r.key.clone(), r.carrier.clone()))
            .collect();
        let old = source.accepted.replace(next.clone());
        self.retired.borrow_mut().extend(
            old.into_iter()
                .filter(|(key, c)| next.get(key).is_none_or(|n| !Rc::ptr_eq(n, c)))
                .map(|(_, c)| c),
        );
    }
    pub(crate) fn finish(&self) -> Vec<ChartError> {
        self.retirements.borrow_mut().clear();
        for carrier in self.retired.borrow_mut().drain(..) {
            carrier.retire();
        }
        let mut errors = Vec::new();
        for row in &self.rows {
            if let Err(e) = row.carrier.host.web.app().pump() {
                errors.push(host(e));
                row.carrier.hide_widgets();
            }
            errors.extend(std::mem::take(&mut *row.carrier.host.errors.borrow_mut()));
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

fn selected_slots<'a>(
    plan: &TreeSvgPlan,
    occurrence: &TreeSvgOccurrence,
    layout: &'a SceneWidgetsLayout,
) -> Vec<&'a SceneWidgetSlot> {
    let targets: BTreeSet<_> = occurrence
        .items()
        .iter()
        .map(|i| match *i {
            DrawItem::Mark(i) => {
                SceneWidgetTarget::Mark(plan.frame().layout().scene.marks[i].id.clone())
            }
            DrawItem::Label(i) => SceneWidgetTarget::Label(i),
        })
        .collect();
    layout
        .slots
        .iter()
        .filter(|s| targets.contains(&s.target))
        .collect()
}
pub(crate) fn occurrence_scene(
    source: &Scene,
    items: &[DrawItem],
) -> (Scene, BTreeMap<usize, usize>) {
    let mut out = source.clone();
    out.background = Color(0, 0, 0, 0);
    out.marks.clear();
    out.labels.clear();
    out.draw_order.clear();
    out.legend.clear();
    out.legend_roles.clear();
    out.legend_targets.clear();
    out.label_series.clear();
    out.label_clips.clear();
    out.typography.labels.clear();
    out.typography.label_whitespace.clear();
    out.typography.label_strokes.clear();
    let marks: BTreeSet<_> = items
        .iter()
        .filter_map(|i| {
            if let DrawItem::Mark(i) = i {
                Some(source.marks[*i].id.as_str())
            } else {
                None
            }
        })
        .collect();
    out.opacity_groups
        .retain(|g| g.members.iter().all(|m| marks.contains(m.as_str())));
    let mut labels = BTreeMap::new();
    for item in items {
        match *item {
            DrawItem::Mark(i) => {
                out.draw_order.push(DrawItem::Mark(out.marks.len()));
                out.marks.push(source.marks[i].clone());
            }
            DrawItem::Label(i) => {
                let n = out.labels.len();
                labels.insert(i, n);
                out.draw_order.push(DrawItem::Label(n));
                out.labels.push(source.labels[i].clone());
                if let Some(v) = source.label_series.get(&i) {
                    out.label_series.insert(n, v.clone());
                }
                if let Some(v) = source.label_clips.get(&i) {
                    out.label_clips.insert(n, v.clone());
                }
                if let Some(v) = source.typography.labels.get(&i) {
                    out.typography.labels.insert(n, v.clone());
                }
                if let Some(v) = source.typography.label_whitespace.get(&i) {
                    out.typography.label_whitespace.insert(n, *v);
                }
                if let Some(stroke) = source.typography.label_strokes.get(&i) {
                    out.typography.label_strokes.insert(n, *stroke);
                }
            }
        }
    }
    (out, labels)
}
