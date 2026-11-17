//! Owned SVG carriers: prepare, journaled place, common adoption, then pump.
//! Managed Glory children are never moved through an unjournaled Scope.place.
use super::*;

const SVG_NS: &str = "http://www.w3.org/2000/svg";
pub(super) const XHTML_NS: &str = "http://www.w3.org/1999/xhtml";
pub(super) fn svg_node(document: &web_sys::Document, name: &str) -> Result<Element> {
    document.create_element_ns(Some(SVG_NS), name).map_err(host)
}
fn attr(node: &Element, name: &str, value: &str) -> Result<()> {
    node.set_attribute(name, value).map_err(host)
}
fn css(node: &Element, name: &str, value: &str) -> Result<()> {
    let style = web_sys::js_sys::Reflect::get(node, &"style".into())
        .map_err(host)?
        .dyn_into::<web_sys::CssStyleDeclaration>()
        .map_err(host)?;
    style.set_property(name, value).map_err(host)
}
fn restore(node: &Element, name: &str, value: Option<&str>) -> Result<()> {
    match value {
        Some(value) => attr(node, name, value),
        None => node.remove_attribute(name).map_err(host),
    }
}
struct Shell(Element);
impl Drop for Shell {
    fn drop(&mut self) {
        self.0.remove();
    }
}
struct Move {
    node: Element,
    parent: Option<web_sys::Node>,
    next: Option<web_sys::Node>,
}
struct Placement {
    moves: Vec<Move>,
    attributes: Vec<(Element, String, Option<String>)>,
    focus: Option<Element>,
    leaves: Vec<LeafJournal>,
}
type LeafKey = (String, String, String);
struct NativeReuse {
    node: Element,
    desired: Element,
}
struct LeafJournal {
    node: Element,
    parent: Option<web_sys::Node>,
    next: Option<web_sys::Node>,
    attributes: BTreeMap<String, String>,
    children: Vec<web_sys::Node>,
}
fn leaf_attributes(node: &Element) -> Result<BTreeMap<String, String>> {
    node.get_attribute_names()
        .iter()
        .map(|name| {
            let name = name
                .as_string()
                .ok_or_else(|| ChartError("Funnel native attribute name invalid".into()))?;
            let value = node
                .get_attribute(&name)
                .ok_or_else(|| ChartError("Funnel native attribute disappeared".into()))?;
            Ok((name, value))
        })
        .collect()
}
fn copy_attributes(node: &Element, desired: &BTreeMap<String, String>) -> Result<()> {
    for name in leaf_attributes(node)?.keys() {
        if !desired.contains_key(name) {
            node.remove_attribute(name).map_err(host)?;
        }
    }
    for (name, value) in desired {
        if node.get_attribute(name).as_ref() != Some(value) {
            attr(node, name, value)?;
        }
    }
    Ok(())
}
fn replace_children(node: &Element, children: &[web_sys::Node]) -> Result<()> {
    while let Some(child) = node.first_child() {
        node.remove_child(&child).map_err(host)?;
    }
    for child in children {
        node.append_child(child).map_err(host)?;
    }
    Ok(())
}
fn native_leaves(node: &Element, paths: &mut Vec<Element>, texts: &mut Vec<Element>) {
    if node.local_name() == "path" && node.has_attribute("data-id") {
        paths.push(node.clone());
    }
    if node.local_name() == "text" {
        texts.push(node.clone());
    }
    let mut child = node.first_element_child();
    while let Some(node) = child {
        child = node.next_element_sibling();
        native_leaves(&node, paths, texts);
    }
}
fn restore_native_focus(element: &Element) -> Result<()> {
    if let Some(html) = element.dyn_ref::<HtmlElement>() {
        return restore_dom_focus(html);
    }
    if !element.is_connected()
        || element
            .owner_document()
            .and_then(|d| d.active_element())
            .is_some_and(|active| active.is_same_node(Some(element.as_ref())))
    {
        return Ok(());
    }
    let options = web_sys::js_sys::Object::new();
    web_sys::js_sys::Reflect::set(&options, &"preventScroll".into(), &true.into()).map_err(host)?;
    let focus = web_sys::js_sys::Reflect::get(element, &"focus".into())
        .map_err(host)?
        .dyn_into::<web_sys::js_sys::Function>()
        .map_err(host)?;
    focus.call1(element, &options).map_err(host)?;
    Ok(())
}
struct NativeTarget {
    path: Element,
    mark_id: String,
    part_index: usize,
    key: String,
    index: usize,
}
pub(super) struct Stage {
    owner: Weak<Driver>,
    plan: FunnelSvgPlan,
    shell: Shell,
    pub frames: BTreeMap<String, BrowserFunnelSvgLayerFrame>,
    pub records: BTreeMap<String, Rc<layers::Record>>,
    carriers: Vec<(Element, Element)>,
    hits: Vec<NativeTarget>,
    leaves: BTreeMap<LeafKey, Element>,
    reuse: Vec<NativeReuse>,
    generation: u64,
    publication: u64,
    data: FunnelSvgFrame,
    pub source: SourceEnvelope,
    view: ZoomTransform,
    interactive: bool,
    previous: Option<Weak<Stage>>,
    placement: RefCell<Option<Placement>>,
    adopted: Cell<bool>,
}

impl Stage {
    pub fn prepare(
        owner: &Rc<Driver>,
        request: &Request,
        source: SourceEnvelope,
        generation: u64,
        publication: u64,
        previous: Option<Rc<Stage>>,
    ) -> Result<Rc<Self>> {
        source.validate_type(owner.raw_type)?;
        let data = source.projection.clone();
        let physical = source.paint_scene();
        layers::preflight(owner, &request.program, previous.as_deref())?;
        request.view.validate()?;
        let plan = FunnelSvgPlan::new(
            &data,
            request.program.layers.clone(),
            request.program.budget,
            request.program.extras,
        )?;
        let document = owner
            .content
            .owner_document()
            .ok_or_else(|| ChartError("Funnel document missing".into()))?;
        let root = svg_node(&document, "svg")?;
        attr(&root, "class", "kartis-funnel-svg-layers")?;
        attr(&root, "data-funnel-svg-active", "false")?;
        attr(&root, "data-funnel-svg-state", "prepared")?;
        attr(
            &root,
            "data-funnel-svg-publication",
            &publication.to_string(),
        )?;
        attr(&root, "width", &data.scene().width.to_string())?;
        attr(&root, "height", &data.scene().height.to_string())?;
        attr(
            &root,
            "viewBox",
            &format!("0 0 {} {}", data.scene().width, data.scene().height),
        )?;
        attr(&root, "role", "img")?;
        attr(&root, "aria-label", &data.scene().title)?;
        attr(
            &root,
            "style",
            "position:absolute;left:0;top:0;width:100%;height:100%;visibility:hidden;opacity:0;overflow:hidden;pointer-events:none",
        )?;
        let background = svg_node(&document, "rect")?;
        for (name, value) in [
            ("x", 0.),
            ("y", 0.),
            ("width", data.scene().width),
            ("height", data.scene().height),
        ] {
            attr(&background, name, &value.to_string())?;
        }
        attr(
            &background,
            "fill",
            &render::css_color(data.scene().background),
        )?;
        attr(&background, "pointer-events", "none")?;
        root.append_child(&background).map_err(host)?;
        let shell = Shell(root);
        // Connecting only an inert, hidden owned shell is reversible and does
        // not publish the candidate or subscribe an old subtree to new data.
        owner.content.append_child(&shell.0).map_err(host)?;
        let mut frames = BTreeMap::new();
        let mut records = BTreeMap::new();
        let mut carriers = Vec::new();
        let mut hits = Vec::new();
        let mut leaves = BTreeMap::new();
        let mut reuse = Vec::new();
        let mut body_parts = BTreeMap::new();
        for index in plan.body_mark_indices() {
            if let Some(FunnelSvgMarkRole::Body { part_index }) = data.mark_role(*index)? {
                body_parts.insert(data.scene().marks[*index].id.clone(), part_index);
            }
        }
        let paint = paint_view(data.plot(), request.view);
        for occurrence in plan.layers() {
            let key = &occurrence.layer.key;
            let group = svg_node(&document, "g")?;
            attr(&group, "class", "kartis-funnel-svg-occurrence")?;
            attr(&group, "data-funnel-svg-key", key)?;
            attr(
                &group,
                "data-funnel-svg-index",
                &occurrence.index.to_string(),
            )?;
            attr(&group, "data-funnel-svg-kind", kind(&occurrence.layer.kind))?;
            shell.0.append_child(&group).map_err(host)?;
            let frame = BrowserFunnelSvgLayerFrame {
                data: data.clone(),
                source_generation: generation,
                publication,
                key: key.clone(),
                index: occurrence.index,
                view: request.view,
                hidden: request.hidden.clone(),
                interactive: request.program.is_interactive(),
            };
            if let FunnelSvgLayerKind::Custom(registry) = &occurrence.layer.kind {
                let component = request.program.custom[registry].clone();
                let retained = previous
                    .as_ref()
                    .and_then(|s| s.records.get(key))
                    .filter(|r| {
                        r.registry == *registry && r.component.revision == component.revision
                    })
                    .cloned();
                let record = match retained {
                    Some(record) => record,
                    None => layers::Record::construct(
                        owner,
                        registry,
                        component,
                        frame.clone(),
                        source.clone(),
                    )?,
                };
                owner.require_alive()?;
                carriers.push((group, record.carrier.clone()));
                records.insert(key.clone(), record);
            } else {
                let (mut scene, _) =
                    plot_widgets::svg_occurrences::occurrence_scene(physical, occurrence.items());
                for mark in &mut scene.marks {
                    if request.hidden.contains(&mark.series) {
                        mark.opacity = 0.;
                    }
                }
                for (index, label) in scene.labels.iter_mut().enumerate() {
                    if scene
                        .label_series
                        .get(&index)
                        .is_some_and(|series| request.hidden.contains(series))
                    {
                        label.color.3 = 0;
                    }
                }
                let ids: BTreeSet<_> = occurrence
                    .items()
                    .iter()
                    .filter_map(|item| match item {
                        DrawItem::Mark(index)
                            if plan.body_mark_indices().contains(index)
                                && !request.hidden.contains(&data.scene().marks[*index].series)
                                && request.program.is_interactive() =>
                        {
                            Some(data.scene().marks[*index].id.clone())
                        }
                        _ => None,
                    })
                    .collect();
                let view = if occurrence.layer.kind == FunnelSvgLayerKind::Annotations {
                    svg::native_view_with_decoration_input(&document, &scene, paint, &ids, true)?
                } else {
                    svg::native_view(&document, &scene, paint, &ids)?
                };
                attr(&view, "class", "kartis-funnel-svg-native-view")?;
                if occurrence.layer.kind == FunnelSvgLayerKind::Parts {
                    parts_paint_opacity(&view, &scene)?;
                }
                let mut paths = Vec::new();
                let mut texts = Vec::new();
                native_leaves(&view, &mut paths, &mut texts);
                let mut desired_leaves = Vec::new();
                for node in paths {
                    if let Some(id) = node.get_attribute("data-id") {
                        desired_leaves.push((format!("mark:{id}"), node));
                    }
                }
                let mut texts = texts.into_iter();
                for item in occurrence.items() {
                    if let DrawItem::Label(index) = *item {
                        let node = texts
                            .next()
                            .ok_or_else(|| ChartError("Funnel native label slot missing".into()))?;
                        if let Some(part) = data.label_part_index(index)? {
                            desired_leaves.push((
                                format!("label:{}", data.parts()[part].datum.id.identity()?),
                                node,
                            ));
                        }
                    }
                }
                for (role, desired) in desired_leaves {
                    let leaf_key = (key.clone(), kind(&occurrence.layer.kind).into(), role);
                    let retained = previous
                        .as_ref()
                        .filter(|old| source.motion.is_some() || old.source.motion.is_some())
                        .and_then(|old| old.leaves.get(&leaf_key))
                        .filter(|old| {
                            old.is_connected()
                                && old.namespace_uri() == desired.namespace_uri()
                                && old.local_name() == desired.local_name()
                        });
                    let actual = retained.cloned().unwrap_or_else(|| desired.clone());
                    if retained.is_some() {
                        reuse.push(NativeReuse {
                            node: actual.clone(),
                            desired: desired.clone(),
                        });
                    }
                    leaves.insert(leaf_key, actual);
                }
                let hit_start = hits.len();
                collect_hits(&view, &ids, &body_parts, key, occurrence.index, &mut hits);
                for hit in &mut hits[hit_start..] {
                    if let Some(actual) = leaves.get(&(
                        key.clone(),
                        kind(&occurrence.layer.kind).into(),
                        format!("mark:{}", hit.mark_id),
                    )) {
                        hit.path = actual.clone();
                    }
                }
                let carrier = svg_node(&document, "foreignObject")?;
                attr(&carrier, "class", "kartis-funnel-svg-native")?;
                for (name, value) in [
                    ("x", 0.),
                    ("y", 0.),
                    ("width", data.scene().width),
                    ("height", data.scene().height),
                ] {
                    attr(&carrier, name, &value.to_string())?;
                }
                attr(&carrier, "style", "overflow:hidden;pointer-events:none")?;
                let content = document
                    .create_element_ns(Some(XHTML_NS), "div")
                    .map_err(host)?;
                attr(&content, "class", "kartis-funnel-svg-native-content")?;
                attr(
                    &content,
                    "style",
                    "position:relative;width:100%;height:100%;overflow:clip;pointer-events:none;isolation:isolate",
                )?;
                content.append_child(&view).map_err(host)?;
                carrier.append_child(&content).map_err(host)?;
                group.append_child(&carrier).map_err(host)?;
            }
            frames.insert(key.clone(), frame);
        }
        Ok(Rc::new(Self {
            owner: Rc::downgrade(owner),
            plan,
            shell,
            frames,
            records,
            carriers,
            hits,
            leaves,
            reuse,
            generation,
            publication,
            data,
            source,
            view: request.view,
            interactive: request.program.is_interactive(),
            previous: previous.as_ref().map(Rc::downgrade),
            placement: RefCell::new(None),
            adopted: Cell::new(false),
        }))
    }
    pub fn place(&self) -> Result<()> {
        let owner = self
            .owner
            .upgrade()
            .ok_or_else(|| ChartError("Funnel owner dropped".into()))?;
        owner.require_alive()?;
        scene::ensure(!self.adopted.get(), "Funnel stage already adopted")?;
        if self.placement.borrow().is_some() {
            return Ok(());
        }
        let moves = self
            .carriers
            .iter()
            .map(|(_, node)| Move {
                node: node.clone(),
                parent: node.parent_node(),
                next: node.next_sibling(),
            })
            .collect();
        let mut attributes = vec![
            (
                self.shell.0.clone(),
                "style".into(),
                self.shell.0.get_attribute("style"),
            ),
            (
                self.shell.0.clone(),
                "data-funnel-svg-state".into(),
                self.shell.0.get_attribute("data-funnel-svg-state"),
            ),
        ];
        if let Some(previous) = self.previous.as_ref().and_then(Weak::upgrade) {
            attributes.push((
                previous.shell.0.clone(),
                "style".into(),
                previous.shell.0.get_attribute("style"),
            ));
        }
        for record in self.records.values() {
            for name in ["x", "y", "width", "height", "style"] {
                attributes.push((
                    record.carrier.clone(),
                    name.into(),
                    record.carrier.get_attribute(name),
                ));
            }
            for node in [&record.clip, &record.root] {
                for name in ["style", "inert"] {
                    attributes.push((node.clone().into(), name.into(), node.get_attribute(name)));
                }
            }
        }
        let focus = owner
            .content
            .owner_document()
            .and_then(|d| d.active_element())
            .filter(|active| owner.content.contains(Some(active.as_ref())));
        *self.placement.borrow_mut() = Some(Placement {
            moves,
            attributes,
            focus: focus.clone(),
            leaves: Vec::new(),
        });
        let result = (|| {
            for reuse in &self.reuse {
                let mut children = Vec::new();
                let mut child = reuse.node.first_child();
                while let Some(node) = child {
                    child = node.next_sibling();
                    children.push(node);
                }
                self.placement
                    .borrow_mut()
                    .as_mut()
                    .expect("placed journal")
                    .leaves
                    .push(LeafJournal {
                        node: reuse.node.clone(),
                        parent: reuse.node.parent_node(),
                        next: reuse.node.next_sibling(),
                        attributes: leaf_attributes(&reuse.node)?,
                        children,
                    });
                copy_attributes(&reuse.node, &leaf_attributes(&reuse.desired)?)?;
                let mut desired_children = Vec::new();
                let mut child = reuse.desired.first_child();
                while let Some(node) = child {
                    child = node.next_sibling();
                    desired_children.push(node.clone_node_with_deep(true).map_err(host)?);
                }
                replace_children(&reuse.node, &desired_children)?;
                let parent = reuse
                    .desired
                    .parent_node()
                    .ok_or_else(|| ChartError("Funnel desired native leaf disconnected".into()))?;
                parent
                    .insert_before(&reuse.node, Some(reuse.desired.as_ref()))
                    .map_err(host)?;
                parent.remove_child(&reuse.desired).map_err(host)?;
            }
            for (parent, carrier) in &self.carriers {
                parent.append_child(carrier).map_err(host)?;
            }
            for (key, record) in &self.records {
                pose(
                    &record.carrier,
                    &record.clip,
                    &record.root,
                    &self.frames[key],
                )?;
                if !record.is_failed() {
                    record.root.remove_attribute("inert").map_err(host)?;
                }
            }
            if let Some(previous) = self.previous.as_ref().and_then(Weak::upgrade) {
                css(&previous.shell.0, "visibility", "hidden")?;
                css(&previous.shell.0, "opacity", "0")?;
            }
            css(&self.shell.0, "visibility", "visible")?;
            css(&self.shell.0, "opacity", "1")?;
            attr(&self.shell.0, "data-funnel-svg-state", "placed")?;
            // Successful replacement must not refocus a superseded child. Its
            // identity is still journaled for a later pre-adoption rollback.
            if let Some(focus) = focus
                && (self
                    .carriers
                    .iter()
                    .any(|(_, carrier)| carrier.contains(Some(focus.as_ref())))
                    || self
                        .reuse
                        .iter()
                        .any(|reuse| reuse.node.is_same_node(Some(focus.as_ref()))))
            {
                restore_native_focus(&focus)?;
            }
            owner.require_alive()
        })();
        match result {
            Ok(()) => Ok(()),
            Err(error) => match self.rollback() {
                Ok(()) => Err(error),
                Err(rollback) => Err(ChartError(format!(
                    "{}; Funnel rollback: {}",
                    error.0, rollback.0
                ))),
            },
        }
    }
    pub fn validate(&self, owner: &Driver) -> Result<()> {
        owner.require_alive()?;
        self.source.validate_type(owner.raw_type)?;
        scene::ensure(
            self.source.motion.as_ref().is_none_or(|motion| {
                motion.source_generation == self.generation
                    && motion.publication == self.publication
            }),
            "Funnel motion presentation epoch differs",
        )?;
        scene::ensure(
            self.placement.borrow().is_some() && !self.adopted.get(),
            "Funnel stage is not placed",
        )?;
        scene::ensure(
            owner.content.contains(Some(self.shell.0.as_ref())),
            "Funnel shell disconnected during placement",
        )?;
        for (key, record) in &self.records {
            scene::ensure(
                record.key == *key
                    && self.shell.0.contains(Some(record.carrier.as_ref()))
                    && self.frames.contains_key(key),
                "Funnel component carrier is not placed",
            )?;
        }
        for hit in &self.hits {
            scene::ensure(
                self.shell.0.contains(Some(hit.path.as_ref()))
                    && hit.path.get_attribute("data-id").as_deref() == Some(hit.mark_id.as_str()),
                "Funnel actual native hit leaf is not placed",
            )?;
        }
        Ok(())
    }
    pub fn rollback(&self) -> Result<()> {
        if self.adopted.get() {
            return Ok(());
        }
        let Some(placement) = self.placement.borrow_mut().take() else {
            return Ok(());
        };
        let mut errors = Vec::new();
        for leaf in placement.leaves.into_iter().rev() {
            let restored = (|| {
                match leaf.parent {
                    Some(parent) => {
                        parent
                            .insert_before(&leaf.node, leaf.next.as_ref())
                            .map_err(host)?;
                    }
                    None => leaf.node.remove(),
                }
                copy_attributes(&leaf.node, &leaf.attributes)?;
                replace_children(&leaf.node, &leaf.children)
            })();
            if let Err(error) = restored {
                errors.push(error.0);
            }
        }
        for movement in placement.moves.into_iter().rev() {
            let result = match movement.parent {
                Some(parent) => parent
                    .insert_before(&movement.node, movement.next.as_ref())
                    .map(|_| ())
                    .map_err(host),
                None => {
                    movement.node.remove();
                    Ok(())
                }
            };
            if let Err(error) = result {
                errors.push(error.0);
            }
        }
        for (node, name, value) in placement.attributes {
            if let Err(error) = restore(&node, &name, value.as_deref()) {
                errors.push(error.0);
            }
        }
        if let Some(focus) = placement.focus
            && let Err(error) = restore_native_focus(&focus)
        {
            errors.push(error.0);
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(ChartError(errors.join("; ")))
        }
    }
    pub fn adopt(&self, previous: Option<&Stage>) {
        if self.adopted.replace(true) {
            return;
        }
        if let Some(previous) = previous {
            for (key, record) in &previous.records {
                if self
                    .records
                    .get(key)
                    .is_none_or(|new| !Rc::ptr_eq(new, record))
                {
                    record.live.set(false);
                    if record.frame.get_untracked().is_some() {
                        record.frame.revise(|frame| *frame = None);
                    }
                }
            }
        }
        for (key, record) in &self.records {
            let value = Some(layers::LayerSnapshot {
                frame: self.frames[key].clone(),
                source: self.source.clone(),
            });
            if record.frame.get_untracked() != value {
                record.frame.revise(|frame| *frame = value);
            }
            record.live.set(true);
        }
        self.placement.borrow_mut().take();
    }
    pub fn publish(&self) -> Vec<ChartError> {
        let mut errors = Vec::new();
        for (name, value) in [
            ("data-funnel-svg-active", "true"),
            ("data-funnel-svg-state", "accepted"),
        ] {
            if let Err(error) = attr(&self.shell.0, name, value) {
                errors.push(error);
            }
        }
        if let Some(previous) = self.previous.as_ref().and_then(Weak::upgrade)
            && let Err(error) = attr(&previous.shell.0, "data-funnel-svg-active", "false")
        {
            errors.push(error);
        }
        errors
    }
    pub fn finish(&self, previous: Option<&Stage>) -> Vec<ChartError> {
        let mut errors = Vec::new();
        for record in self.records.values() {
            if let Some(error) = record.pump() {
                errors.push(error);
            }
        }
        if let Some(previous) = previous {
            for (key, record) in &previous.records {
                if self
                    .records
                    .get(key)
                    .is_none_or(|new| !Rc::ptr_eq(new, record))
                {
                    if let Some(error) = record.pump() {
                        errors.push(error);
                    }
                    if let Some(error) = record.retire() {
                        errors.push(error);
                    }
                }
            }
            previous.shell.0.remove();
        }
        errors
    }
    pub fn contains_body(&self, role: &HoverRole) -> bool {
        self.hits
            .iter()
            .any(|target| target.key == role.0 && target.mark_id == role.1)
    }
    pub fn plan(&self) -> &FunnelSvgPlan {
        &self.plan
    }
    pub fn same_body(&self, element: &Element, role: &HoverRole) -> bool {
        self.owned_target(element)
            .is_some_and(|target| target.key == role.0 && target.mark_id == role.1)
    }
    pub fn leave_role(&self, event: &Event) -> Option<HoverRole> {
        let element = event.target()?.dyn_into::<Element>().ok()?;
        self.owned_target(&element)
            .map(|target| (target.key.clone(), target.mark_id.clone()))
    }
    fn owned_target(&self, element: &Element) -> Option<&NativeTarget> {
        if !self.adopted.get()
            || !self.interactive
            || !element.is_connected()
            || !self.shell.0.contains(Some(element.as_ref()))
        {
            return None;
        }
        self.hits.iter().rev().find(|target| {
            target.path.is_same_node(Some(element.as_ref()))
                && element.get_attribute("data-id").as_deref() == Some(target.mark_id.as_str())
        })
    }
    pub fn hit(
        &self,
        event: &Event,
        mouse: &web_sys::MouseEvent,
    ) -> Result<Option<BrowserFunnelSvgHit>> {
        self.mouse_hit(event, mouse, false)
    }
    pub fn mouse_hit(
        &self,
        event: &Event,
        mouse: &web_sys::MouseEvent,
        allow_outside: bool,
    ) -> Result<Option<BrowserFunnelSvgHit>> {
        if !self.adopted.get() || !self.interactive {
            return Ok(None);
        }
        let Some(target) = event.target().and_then(|n| n.dyn_into::<Element>().ok()) else {
            return Ok(None);
        };
        let Some(native) = self.owned_target(&target) else {
            return Ok(None);
        };
        let owner = self
            .owner
            .upgrade()
            .ok_or_else(|| ChartError("Funnel owner dropped".into()))?;
        // A nested chart's actual path is never an owned Funnel body element.
        let (client_x, client_y) = client_coordinates(mouse);
        let rect = self.shell.0.get_bounding_client_rect();
        if rect.width() <= 0. || rect.height() <= 0. {
            return Ok(None);
        }
        let viewport = [
            (client_x - rect.left()) * self.data.scene().width / rect.width(),
            (client_y - rect.top()) * self.data.scene().height / rect.height(),
        ];
        if viewport.iter().any(|v| !v.is_finite())
            || !allow_outside
                && (viewport[0] < 0.
                    || viewport[1] < 0.
                    || viewport[0] > self.data.scene().width
                    || viewport[1] > self.data.scene().height)
        {
            return Ok(None);
        }
        let paint = paint_view(self.data.plot(), self.view);
        let point = paint.unproject((viewport[0], viewport[1]))?;
        scene::ensure(owner.alive.get(), "Funnel owner stopped")?;
        Ok(Some(BrowserFunnelSvgHit {
            data: self.data.clone(),
            source_generation: self.generation,
            publication: self.publication,
            key: native.key.clone(),
            index: native.index,
            part_index: native.part_index,
            source_index: self.data.parts()[native.part_index].source_index,
            mark_id: native.mark_id.clone(),
            client: [client_x, client_y],
            viewport,
            scene: [point.0, point.1],
            trusted: event.is_trusted(),
            native_type: event.type_(),
        }))
    }
}
impl Drop for Stage {
    fn drop(&mut self) {
        if !self.adopted.get() {
            let _ = self.rollback();
        }
    }
}

/// Nivo's separate body and border paths use fillOpacity and strokeOpacity.
/// For one actual paint operation these equal the source scalar opacity, while
/// avoiding a needless whole-path compositing boundary in the browser. Two
/// paint operations must keep whole-path opacity: moving it to both brushes
/// would change their overlap. This adapter never edits the canonical Scene.
fn parts_paint_opacity(root: &Element, scene: &Scene) -> Result<()> {
    let mut single_pens = BTreeMap::new();
    for mark in &scene.marks {
        // Use exactly the native renderer's existing point-cap normalization.
        // A stroke-only point may actually paint as one filled footprint.
        let painted = render::paint_mark(mark)?;
        let filled = painted.fill.is_some() || painted.fill_paint.is_some();
        let attribute = match (filled, painted.stroke.is_some()) {
            (true, false) => Some("fill-opacity"),
            (false, true) => Some("stroke-opacity"),
            _ => None,
        };
        if let Some(attribute) = attribute {
            single_pens.insert(mark.id.as_str(), (attribute, painted.opacity));
        }
    }
    apply_parts_opacity(root, &single_pens)
}
fn apply_parts_opacity(node: &Element, single_pens: &BTreeMap<&str, (&str, f64)>) -> Result<()> {
    if node.tag_name().eq_ignore_ascii_case("path")
        && let Some(id) = node.get_attribute("data-id")
        && let Some((attribute, opacity)) = single_pens.get(id.as_str())
    {
        attr(node, attribute, &opacity.to_string())?;
        attr(node, "opacity", "1")?;
    }
    let mut child = node.first_element_child();
    while let Some(node) = child {
        child = node.next_element_sibling();
        apply_parts_opacity(&node, single_pens)?;
    }
    Ok(())
}

fn collect_hits(
    node: &Element,
    ids: &BTreeSet<String>,
    body_parts: &BTreeMap<String, usize>,
    key: &str,
    index: usize,
    hits: &mut Vec<NativeTarget>,
) {
    if node.tag_name().eq_ignore_ascii_case("path")
        && let Some(id) = node.get_attribute("data-id")
        && ids.contains(&id)
        && let Some(part_index) = body_parts.get(&id)
    {
        hits.push(NativeTarget {
            path: node.clone(),
            mark_id: id,
            part_index: *part_index,
            key: key.into(),
            index,
        });
    }
    let mut child = node.first_element_child();
    while let Some(node) = child {
        child = node.next_element_sibling();
        collect_hits(&node, ids, body_parts, key, index, hits);
    }
}
fn paint_view(plot: ClipRect, view: ZoomTransform) -> ZoomTransform {
    ZoomTransform {
        scale: view.scale,
        translate_x: plot.x + view.translate_x - plot.x * view.scale,
        translate_y: plot.y + view.translate_y - plot.y * view.scale,
    }
}
fn kind(kind: &FunnelSvgLayerKind) -> &'static str {
    match kind {
        FunnelSvgLayerKind::Parts => "parts",
        FunnelSvgLayerKind::Separators => "separators",
        FunnelSvgLayerKind::Labels => "labels",
        FunnelSvgLayerKind::Annotations => "annotations",
        FunnelSvgLayerKind::SceneExtras => "scene-extras",
        FunnelSvgLayerKind::Custom(_) => "custom",
    }
}
pub(super) fn pose(
    carrier: &Element,
    clip: &HtmlElement,
    root: &HtmlElement,
    frame: &BrowserFunnelSvgLayerFrame,
) -> Result<()> {
    let scene = frame.data.scene();
    let plot = frame.data.plot();
    for (name, value) in [
        ("x", 0.),
        ("y", 0.),
        ("width", scene.width),
        ("height", scene.height),
    ] {
        attr(carrier, name, &value.to_string())?;
    }
    for (name, value) in [
        ("left", plot.x),
        ("top", plot.y),
        ("width", plot.width),
        ("height", plot.height),
    ] {
        clip.style()
            .set_property(name, &format!("{value}px"))
            .map_err(host)?;
    }
    root.style()
        .set_property("width", &format!("{}px", plot.width))
        .map_err(host)?;
    root.style()
        .set_property("height", &format!("{}px", plot.height))
        .map_err(host)?;
    root.style()
        .set_property(
            "transform",
            &format!(
                "matrix({},0,0,{},{},{})",
                frame.view.scale, frame.view.scale, frame.view.translate_x, frame.view.translate_y
            ),
        )
        .map_err(host)?;
    let font = scene.typography.default.resolve(&ResolvedFont::default());
    for (name, value) in [
        ("font-family", font.css_family()),
        (
            "font-size",
            format!("{}px", frame.data.config().theme.font_size),
        ),
        ("font-weight", font.weight.to_string()),
        ("color", render::css_color(frame.data.config().theme.text)),
    ] {
        root.style().set_property(name, &value).map_err(host)?;
    }
    Ok(())
}
