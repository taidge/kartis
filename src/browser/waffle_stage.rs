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
                .ok_or_else(|| ChartError("Waffle native attribute name invalid".into()))?;
            let value = node
                .get_attribute(&name)
                .ok_or_else(|| ChartError("Waffle native attribute disappeared".into()))?;
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
    area_id: String,
    source_index: usize,
    polygon_index: usize,
    key: String,
    index: usize,
}
pub(super) struct Stage {
    owner: Weak<Driver>,
    plan: WaffleSvgPlan,
    shell: Shell,
    pub frames: BTreeMap<String, BrowserWaffleSvgLayerFrame>,
    pub records: BTreeMap<String, Rc<layers::Record>>,
    carriers: Vec<(Element, Element)>,
    hits: Vec<NativeTarget>,
    leaves: BTreeMap<LeafKey, Element>,
    reuse: Vec<NativeReuse>,
    generation: u64,
    publication: u64,
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
        let data = &source.projection;
        let plan = WaffleSvgPlan::new(
            data,
            request.program.layers.clone(),
            request.program.legends.clone(),
            request.program.budget,
        )?;
        let document = owner
            .content
            .owner_document()
            .ok_or_else(|| ChartError("Waffle document missing".into()))?;
        let root = svg_node(&document, "svg")?;
        for (key, value) in [
            ("class", "kartis-waffle-svg-layers"),
            ("data-waffle-svg-active", "false"),
            ("data-waffle-svg-state", "prepared"),
            (
                "style",
                "position:absolute;left:0;top:0;visibility:hidden;overflow:hidden",
            ),
        ] {
            attr(&root, key, value)?;
        }
        attr(&root, "xmlns", SVG_NS)?;
        for (key, value) in [
            ("width", data.scene().width),
            ("height", data.scene().height),
        ] {
            attr(&root, key, &value.to_string())?;
        }
        attr(
            &root,
            "viewBox",
            &format!("0 0 {} {}", data.scene().width, data.scene().height),
        )?;
        let shell = Shell(root);
        let background = svg_node(&document, "rect")?;
        for (key, value) in [
            ("x", "0"),
            ("y", "0"),
            ("width", "100%"),
            ("height", "100%"),
            ("pointer-events", "none"),
        ] {
            attr(&background, key, value)?;
        }
        attr(
            &background,
            "fill",
            &render::css_color(data.scene().background),
        )?;
        shell.0.append_child(&background).map_err(host)?;
        owner.content.append_child(&shell.0).map_err(host)?;
        let mut records = BTreeMap::new();
        let mut frames = BTreeMap::new();
        let mut carriers = Vec::new();
        let mut hits = Vec::new();
        let mut leaves = BTreeMap::new();
        let mut reuse = Vec::new();
        for occurrence in plan.layers() {
            let group = svg_node(&document, "g")?;
            attr(&group, "class", "kartis-waffle-svg-occurrence")?;
            attr(&group, "data-waffle-svg-key", &occurrence.layer.key)?;
            attr(&group, "data-waffle-svg-kind", kind(&occurrence.layer.kind))?;
            shell.0.append_child(&group).map_err(host)?;
            let frame = BrowserWaffleSvgLayerFrame {
                data: data.clone(),
                presentation: plan.clone(),
                source_generation: generation,
                publication,
                key: occurrence.layer.key.clone(),
                index: occurrence.index,
                view: request.view,
                hidden: request.options.hidden.clone(),
                interactive: request.program.interactive,
            };
            frames.insert(frame.key.clone(), frame.clone());
            if let WaffleSvgLayerKind::Custom(registry) = &occurrence.layer.kind {
                let component = request.program.custom[registry].clone();
                let old = previous
                    .as_ref()
                    .and_then(|p| p.records.get(&frame.key))
                    .filter(|r| {
                        r.registry == *registry && r.component.revision == component.revision
                    });
                let record = match old {
                    Some(r) => r.clone(),
                    None => layers::Record::construct(
                        owner,
                        registry,
                        component,
                        frame,
                        source.clone(),
                    )?,
                };
                owner.require_alive()?;
                carriers.push((group, record.carrier.clone()));
                records.insert(record.key.clone(), record);
            } else {
                let native = plan.native_scene(occurrence.index)?;
                let ids = plan.native_hit_ids(occurrence.index)?;
                let view = svg::native_view(
                    &document,
                    &native,
                    paint_view(data.plot(), request.view),
                    &ids,
                )?;
                attr(&view, "class", "kartis-waffle-svg-native-view")?;
                let mut paths = Vec::new();
                let mut texts = Vec::new();
                native_leaves(&view, &mut paths, &mut texts);
                let mut area_roles = BTreeMap::new();
                for item in occurrence.items() {
                    if let WaffleSvgItem::Area {
                        source_index,
                        polygon_index,
                    } = *item
                    {
                        let area = data.area(source_index, polygon_index)?;
                        area_roles.insert(area.id().to_owned(), (source_index, polygon_index));
                    }
                }
                for node in paths {
                    let id = node
                        .get_attribute("data-id")
                        .ok_or_else(|| ChartError("Waffle native path ID missing".into()))?;
                    let role = area_roles.get(&id).copied();
                    // Upstream Areas are keyed by datum ID, not source ordinal.
                    // Reordering changes the candidate data-id but preserves the
                    // group's actual leaf; place journals those attributes.
                    let leaf = role.map_or_else(
                        || format!("mark:{id}"),
                        |(source_index, polygon_index)| {
                            format!("area:{}:{polygon_index}", data.source()[source_index].id)
                        },
                    );
                    let key = (
                        frame.key.clone(),
                        kind(&occurrence.layer.kind).to_owned(),
                        leaf,
                    );
                    let actual =
                        if let Some(old) = previous.as_ref().and_then(|p| p.leaves.get(&key)) {
                            reuse.push(NativeReuse {
                                node: old.clone(),
                                desired: node.clone(),
                            });
                            old.clone()
                        } else {
                            node.clone()
                        };
                    if occurrence.layer.kind == WaffleSvgLayerKind::Areas {
                        attr(&node, "fill", "#000000")?;
                        attr(&node, "fill-opacity", "0")?;
                        attr(&node, "opacity", "1")?;
                    }
                    leaves.insert(key, actual.clone());
                    if let Some((source_index, polygon_index)) = role {
                        hits.push(NativeTarget {
                            path: actual.clone(),
                            area_id: id,
                            source_index,
                            polygon_index,
                            key: frame.key.clone(),
                            index: frame.index,
                        });
                    }
                }
                for (index, node) in texts.into_iter().enumerate() {
                    let key = (
                        frame.key.clone(),
                        kind(&occurrence.layer.kind).to_owned(),
                        format!("label:{index}"),
                    );
                    let actual =
                        if let Some(old) = previous.as_ref().and_then(|p| p.leaves.get(&key)) {
                            reuse.push(NativeReuse {
                                node: old.clone(),
                                desired: node.clone(),
                            });
                            old.clone()
                        } else {
                            node.clone()
                        };
                    leaves.insert(key, actual);
                }
                let carrier = svg_node(&document, "foreignObject")?;
                attr(&carrier, "class", "kartis-waffle-svg-native")?;
                // Only explicitly painted native leaves receive input. The
                // full-Scene carrier must not block lower layers on empty ink.
                attr(&carrier, "pointer-events", "none")?;
                for (key, value) in [
                    ("x", 0.),
                    ("y", 0.),
                    ("width", data.scene().width),
                    ("height", data.scene().height),
                ] {
                    attr(&carrier, key, &value.to_string())?;
                }
                let content = document
                    .create_element_ns(Some(XHTML_NS), "div")
                    .map_err(host)?;
                attr(&content, "class", "kartis-waffle-svg-native-content")?;
                // Keep a stable native SVG paint boundary when the HTML
                // tooltip appears or disappears above the accepted backdrop.
                // This avoids stale edge antialiasing without changing ink
                // geometry or attributing the behavior to a specific GPU.
                attr(
                    &content,
                    "style",
                    "position:relative;width:100%;height:100%;overflow:clip;isolation:isolate;pointer-events:none;will-change:transform",
                )?;
                content.append_child(&view).map_err(host)?;
                carrier.append_child(&content).map_err(host)?;
                group.append_child(&carrier).map_err(host)?;
            }
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
            source,
            view: request.view,
            interactive: request.program.interactive,
            previous: previous.as_ref().map(Rc::downgrade),
            placement: RefCell::new(None),
            adopted: Cell::new(false),
        }))
    }
    pub fn place(&self) -> Result<()> {
        let owner = self
            .owner
            .upgrade()
            .ok_or_else(|| ChartError("Waffle owner dropped".into()))?;
        owner.require_alive()?;
        scene::ensure(!self.adopted.get(), "Waffle stage already adopted")?;
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
                "data-waffle-svg-state".into(),
                self.shell.0.get_attribute("data-waffle-svg-state"),
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
                    .ok_or_else(|| ChartError("Waffle desired native leaf disconnected".into()))?;
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
            attr(&self.shell.0, "data-waffle-svg-state", "placed")?;
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
                    "{}; Waffle rollback: {}",
                    error.0, rollback.0
                ))),
            },
        }
    }
    pub fn validate(&self, owner: &Driver) -> Result<()> {
        owner.require_alive()?;
        self.source.validate_type(owner.raw_type)?;
        scene::ensure(
            self.plan.frame() == &self.source.projection,
            "Waffle plan source differs",
        )?;
        for (key, frame) in &self.frames {
            scene::ensure(
                frame.key == *key
                    && frame.data == self.source.projection
                    && frame.source_generation == self.generation
                    && frame.publication == self.publication,
                "Waffle frame cohort differs",
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
            ("data-waffle-svg-active", "true"),
            ("data-waffle-svg-state", "accepted"),
        ] {
            if let Err(error) = attr(&self.shell.0, name, value) {
                errors.push(error);
            }
        }
        if let Some(previous) = self.previous.as_ref().and_then(Weak::upgrade)
            && let Err(error) = attr(&previous.shell.0, "data-waffle-svg-active", "false")
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
    pub fn plan(&self) -> &WaffleSvgPlan {
        &self.plan
    }
    pub fn contains_area(&self, role: &HoverRole) -> bool {
        self.hits.iter().any(|h| {
            h.key == role.0
                && self.source.projection.source()[h.source_index].id == role.1
                && h.polygon_index == role.2
        })
    }
    fn target(&self, element: &Element) -> Option<&NativeTarget> {
        if !self.adopted.get()
            || !element.is_connected()
            || !self.shell.0.contains(Some(element.as_ref()))
        {
            return None;
        }
        self.hits.iter().rev().find(|h| {
            h.path.is_same_node(Some(element.as_ref()))
                && element.get_attribute("data-id").as_deref() == Some(h.area_id.as_str())
        })
    }
    pub fn owns(&self, event: &Event) -> bool {
        event
            .target()
            .and_then(|v| v.dyn_into::<Element>().ok())
            .is_some_and(|e| self.target(&e).is_some())
    }
    pub fn same_area(&self, element: &Element, role: &HoverRole) -> bool {
        self.target(element).is_some_and(|h| {
            h.key == role.0
                && self.source.projection.source()[h.source_index].id == role.1
                && h.polygon_index == role.2
        })
    }
    pub fn hit(
        &self,
        event: &Event,
        mouse: &web_sys::MouseEvent,
        allow_outside: bool,
    ) -> Result<Option<BrowserWaffleSvgHit>> {
        if !self.interactive {
            return Ok(None);
        }
        let Some(element) = event.target().and_then(|v| v.dyn_into::<Element>().ok()) else {
            return Ok(None);
        };
        let Some(h) = self.target(&element) else {
            return Ok(None);
        };
        let (x, y) = client_coordinates(mouse);
        let rect = self.shell.0.get_bounding_client_rect();
        let data = &self.source.projection;
        if rect.width() <= 0. || rect.height() <= 0. {
            return Ok(None);
        }
        let viewport = [
            (x - rect.left()) * data.scene().width / rect.width(),
            (y - rect.top()) * data.scene().height / rect.height(),
        ];
        if viewport.iter().any(|v| !v.is_finite())
            || !allow_outside
                && (viewport[0] < 0.
                    || viewport[1] < 0.
                    || viewport[0] > data.scene().width
                    || viewport[1] > data.scene().height)
        {
            return Ok(None);
        }
        let point = paint_view(data.plot(), self.view).unproject((viewport[0], viewport[1]))?;
        Ok(Some(BrowserWaffleSvgHit {
            data: data.clone(),
            source_generation: self.generation,
            publication: self.publication,
            key: h.key.clone(),
            index: h.index,
            source_index: h.source_index,
            polygon_index: h.polygon_index,
            area_id: h.area_id.clone(),
            client: [x, y],
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
fn paint_view(plot: ClipRect, view: ZoomTransform) -> ZoomTransform {
    ZoomTransform {
        scale: view.scale,
        translate_x: plot.x + view.translate_x - plot.x * view.scale,
        translate_y: plot.y + view.translate_y - plot.y * view.scale,
    }
}
fn kind(kind: &WaffleSvgLayerKind) -> &'static str {
    match kind {
        WaffleSvgLayerKind::Cells => "cells",
        WaffleSvgLayerKind::Areas => "areas",
        WaffleSvgLayerKind::Legends => "legends",
        WaffleSvgLayerKind::Custom(_) => "custom",
    }
}
pub(super) fn pose(
    carrier: &Element,
    clip: &HtmlElement,
    root: &HtmlElement,
    frame: &BrowserWaffleSvgLayerFrame,
) -> Result<()> {
    let scene = frame.data.scene();
    for (key, value) in [
        ("x", 0.),
        ("y", 0.),
        ("width", scene.width),
        ("height", scene.height),
    ] {
        attr(carrier, key, &value.to_string())?;
    }
    for (key, value) in [
        ("left", 0.),
        ("top", 0.),
        ("width", scene.width),
        ("height", scene.height),
    ] {
        clip.style()
            .set_property(key, &format!("{value}px"))
            .map_err(host)?;
    }
    root.style()
        .set_property("width", &format!("{}px", scene.width))
        .map_err(host)?;
    root.style()
        .set_property("height", &format!("{}px", scene.height))
        .map_err(host)?;
    let view = paint_view(frame.data.plot(), frame.view);
    root.style()
        .set_property(
            "transform",
            &format!(
                "matrix({},0,0,{},{},{})",
                view.scale, view.scale, view.translate_x, view.translate_y
            ),
        )
        .map_err(host)?;
    let font = scene.typography.default.resolve(&ResolvedFont::default());
    for (key, value) in [
        ("font-family", font.css_family()),
        (
            "font-size",
            format!("{}px", frame.data.config().theme.font_size),
        ),
        ("font-weight", font.weight.to_string()),
        ("color", render::css_color(frame.data.config().theme.text)),
    ] {
        root.style().set_property(key, &value).map_err(host)?;
    }
    Ok(())
}
