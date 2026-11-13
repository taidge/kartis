//! Retained, application-owned SVG. Marks are keyed by stable Scene IDs;
//! labels by explicit slots, matching the public Scene identity contract.
use super::*;
use std::collections::BTreeMap;

const NS: &str = "http://www.w3.org/2000/svg";
thread_local! {
    static NEXT_OWNER: Cell<u64> = const { Cell::new(0) };
}
pub(super) fn owner_namespace(document: &web_sys::Document) -> Result<String> {
    // The checked counter reserves detached instances in this module. The DOM
    // lookup also avoids attached owners from other copies of the library.
    for _ in 0..1024 {
        let sequence = NEXT_OWNER
            .with(|counter| {
                let next = counter.get().checked_add(1)?;
                counter.set(next);
                Some(next)
            })
            .ok_or_else(|| ChartError("SVG owner namespace counter exhausted".into()))?;
        let id = format!("kartis-svg-owner-{sequence}");
        if document.get_element_by_id(&id).is_none() {
            return Ok(id);
        }
    }
    Err(ChartError(
        "SVG owner namespace collision budget exceeded".into(),
    ))
}
fn scoped(namespace: &str, id: &str) -> String {
    format!("{namespace}-{id}")
}
fn node(document: &web_sys::Document, name: &str) -> Result<Element> {
    document.create_element_ns(Some(NS), name).map_err(host)
}
fn attr(node: &Element, name: &str, value: &str) -> Result<()> {
    if node.get_attribute(name).as_deref() != Some(value) {
        node.set_attribute(name, value).map_err(host)?;
    }
    Ok(())
}
fn unset(node: &Element, name: &str) -> Result<()> {
    if node.has_attribute(name) {
        node.remove_attribute(name).map_err(host)?;
    }
    Ok(())
}
fn ensure_child(parent: &Element, child: &Element) -> Result<()> {
    if child
        .parent_node()
        .as_ref()
        .is_none_or(|current| !current.is_same_node(Some(parent.as_ref())))
    {
        parent.append_child(child).map_err(host)?;
    }
    Ok(())
}
fn text(node: &Element, value: &str) {
    let clean: String = value
        .chars()
        .map(|ch| {
            if matches!(ch, '\0' | '\u{fffe}' | '\u{ffff}')
                || ch.is_control() && !matches!(ch, '\t' | '\n' | '\r')
            {
                '\u{fffd}'
            } else {
                ch
            }
        })
        .collect();
    if node.text_content().as_deref() != Some(&clean) {
        node.set_text_content(Some(&clean));
    }
}
fn gradient(document: &web_sys::Document, paint: &FillPaint, id: &str) -> Result<Element> {
    let (name, values) = match paint {
        FillPaint::Pattern(pattern) => {
            let output = super::pattern_svg::make(document, pattern)?;
            attr(&output, "id", id)?;
            return Ok(output);
        }
        FillPaint::Linear { start, end, .. } => (
            "linearGradient",
            vec![
                ("x1", start[0]),
                ("y1", start[1]),
                ("x2", end[0]),
                ("y2", end[1]),
            ],
        ),
        FillPaint::Radial { center, radius, .. } => (
            "radialGradient",
            vec![("cx", center[0]), ("cy", center[1]), ("r", *radius)],
        ),
    };
    let output = node(document, name)?;
    attr(&output, "id", id)?;
    attr(&output, "gradientUnits", "userSpaceOnUse")?;
    attr(&output, "color-interpolation", "sRGB")?;
    for (name, value) in values {
        attr(&output, name, &value.to_string())?;
    }
    for stop in paint.stops() {
        let child = node(document, "stop")?;
        attr(&child, "offset", &stop.offset.to_string())?;
        attr(&child, "stop-color", &render::css_color(stop.color))?;
        output.append_child(&child).map_err(host)?;
    }
    Ok(output)
}
fn clip_definition(document: &web_sys::Document, clip: &ClipPath, id: &str) -> Result<Element> {
    let definition = node(document, "clipPath")?;
    attr(&definition, "id", id)?;
    attr(&definition, "clipPathUnits", "userSpaceOnUse")?;
    let path = node(document, "path")?;
    attr(&path, "d", &render::path_data(&clip.geometry.commands()))?;
    attr(&path, "clip-rule", render::clip_rule(clip.rule))?;
    attr(&path, "fill-rule", render::clip_rule(clip.rule))?;
    definition.append_child(&path).map_err(host)?;
    Ok(definition)
}
struct MarkNode {
    path: Element,
    title: Element,
    clip: Option<Element>,
    mask: Option<Element>,
    blend: Option<Element>,
    blend_mode: BlendMode,
    accepted: Option<Mark>,
}
impl MarkNode {
    fn new(document: &web_sys::Document) -> Result<Self> {
        let path = node(document, "path")?;
        let title = node(document, "title")?;
        path.append_child(&title).map_err(host)?;
        Ok(Self {
            path,
            title,
            clip: None,
            mask: None,
            blend: None,
            blend_mode: BlendMode::Normal,
            accepted: None,
        })
    }
    fn outer(&self) -> Element {
        self.blend.clone().unwrap_or_else(|| self.clipped())
    }
    fn clipped(&self) -> Element {
        self.mask
            .as_ref()
            .or(self.clip.as_ref())
            .unwrap_or(&self.path)
            .clone()
    }
    fn patch(
        &mut self,
        document: &web_sys::Document,
        mark: &Mark,
        namespace: &str,
        mode: BlendMode,
    ) -> Result<()> {
        if self.accepted.as_ref() == Some(mark) && self.blend_mode == mode {
            return Ok(());
        }
        // A fallible DOM write can leave this node partially patched. Invalidate
        // the cache before writing so repainting the accepted scene repairs it.
        let geometry_changed = self
            .accepted
            .take()
            .is_none_or(|old| old.geometry != mark.geometry);
        attr(&self.path, "data-id", &mark.id)?;
        attr(&self.path, "data-series", &mark.series)?;
        attr(&self.path, "data-value", &mark.value.to_string())?;
        if geometry_changed {
            attr(
                &self.path,
                "d",
                &render::path_data(&mark.geometry.commands()),
            )?;
        }
        attr(
            &self.path,
            "fill",
            &mark
                .fill_paint
                .as_ref()
                .map(|p| format!("url(#{})", scoped(namespace, &p.svg_id())))
                .unwrap_or_else(|| {
                    mark.fill
                        .map(render::css_color)
                        .unwrap_or_else(|| "none".into())
                }),
        )?;
        attr(&self.path, "opacity", &mark.opacity.to_string())?;
        if let Some(stroke) = mark.stroke {
            attr(&self.path, "stroke", &render::css_color(stroke.color))?;
            attr(&self.path, "stroke-width", &stroke.width.to_string())?;
            attr(&self.path, "stroke-linecap", render::cap(stroke.cap))?;
        } else {
            for name in ["stroke", "stroke-width", "stroke-linecap"] {
                unset(&self.path, name)?;
            }
        }
        if let Some(dash) = &mark.stroke_dash {
            let dash = dash.native_normalized()?;
            attr(
                &self.path,
                "stroke-dasharray",
                &if dash.segments.is_empty() {
                    "none".into()
                } else {
                    dash.segments
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(" ")
                },
            )?;
            attr(&self.path, "stroke-dashoffset", &dash.offset.to_string())?;
        } else {
            unset(&self.path, "stroke-dasharray")?;
            unset(&self.path, "stroke-dashoffset")?;
        }
        text(&self.title, &mark.label);
        if let Some(c) = mark.clip {
            if self.clip.is_none() {
                let clip = node(document, "svg")?;
                attr(&clip, "overflow", "hidden")?;
                clip.append_child(&self.path).map_err(host)?;
                if let Some(mask) = &self.mask {
                    mask.append_child(&clip).map_err(host)?;
                }
                self.clip = Some(clip);
            }
            let clip = self.clip.as_ref().unwrap();
            ensure_child(clip, &self.path)?;
            for (name, value) in [
                ("x", c.x),
                ("y", c.y),
                ("width", c.width),
                ("height", c.height),
            ] {
                attr(clip, name, &value.to_string())?;
            }
            attr(
                clip,
                "viewBox",
                &format!("{} {} {} {}", c.x, c.y, c.width, c.height),
            )?;
        } else if let Some(clip) = self.clip.clone() {
            // Keep the structural checkpoint until every fallible operation succeeds.
            clip.remove_child(&self.path).map_err(host)?;
            if let Some(mask) = &self.mask {
                mask.append_child(&self.path).map_err(host)?;
            }
            clip.remove();
            self.clip = None;
        }
        if let Some(clip) = &mark.clip_path {
            if self.mask.is_none() {
                let mask = node(document, "g")?;
                mask.append_child(self.clip.as_ref().unwrap_or(&self.path))
                    .map_err(host)?;
                self.mask = Some(mask);
            }
            ensure_child(
                self.mask.as_ref().unwrap(),
                self.clip.as_ref().unwrap_or(&self.path),
            )?;
            attr(
                self.mask.as_ref().unwrap(),
                "clip-path",
                &format!("url(#{})", scoped(namespace, &clip.svg_id())),
            )?;
        } else if let Some(mask) = self.mask.clone() {
            mask.remove_child(self.clip.as_ref().unwrap_or(&self.path))
                .map_err(host)?;
            mask.remove();
            self.mask = None;
        }
        // Keep a newly allocated wrapper private until all of its DOM writes
        // succeed. A failed first style write must not leave it in the cache.
        let blender = match &self.blend {
            Some(blender) => Some(blender.clone()),
            None if mode != BlendMode::Normal => Some(node(document, "g")?),
            None => None,
        };
        if let Some(blender) = blender {
            let clipped = self.clipped();
            if !blender
                .first_child()
                .is_some_and(|child| child.is_same_node(Some(clipped.as_ref())))
            {
                blender.append_child(&clipped).map_err(host)?;
            }
            if mode == BlendMode::Normal {
                unset(&blender, "style")?;
            } else {
                attr(
                    &blender,
                    "style",
                    &format!("mix-blend-mode:{}", mode.css_name()),
                )?;
            }
            self.blend = Some(blender);
        }
        self.blend_mode = mode;
        self.accepted = Some(mark.clone());
        Ok(())
    }
}
struct LabelNode {
    node: Element,
    accepted: Option<Label>,
    accepted_font: Option<crate::ResolvedFont>,
    mask: Option<Element>,
    accepted_clip: Option<ClipPath>,
}
impl LabelNode {
    fn outer(&self) -> Element {
        self.mask.as_ref().unwrap_or(&self.node).clone()
    }
    fn patch(
        &mut self,
        document: &web_sys::Document,
        label: &Label,
        font: &crate::ResolvedFont,
        clip: Option<&ClipPath>,
        namespace: &str,
    ) -> Result<()> {
        if self.accepted.as_ref() == Some(label)
            && self.accepted_font.as_ref() == Some(font)
            && self.accepted_clip.as_ref() == clip
        {
            return Ok(());
        }
        // As with marks, only a fully successful patch may populate the cache.
        self.accepted = None;
        for (name, value) in [("x", label.x), ("y", label.y), ("font-size", label.size)] {
            attr(&self.node, name, &value.to_string())?;
        }
        attr(&self.node, "fill", &render::css_color(label.color))?;
        attr(&self.node, "font-family", &font.css_family())?;
        attr(&self.node, "font-weight", &font.weight.to_string())?;
        attr(&self.node, "text-anchor", render::anchor(label.anchor))?;
        if label.rotation != 0. {
            attr(
                &self.node,
                "transform",
                &format!("rotate({} {} {})", label.rotation, label.x, label.y),
            )?;
        } else {
            unset(&self.node, "transform")?;
        }
        text(&self.node, &label.text);
        if let Some(clip) = clip {
            if self.mask.is_none() {
                let mask = node(document, "g")?;
                mask.append_child(&self.node).map_err(host)?;
                self.mask = Some(mask);
            }
            ensure_child(self.mask.as_ref().unwrap(), &self.node)?;
            attr(
                self.mask.as_ref().unwrap(),
                "clip-path",
                &format!("url(#{})", scoped(namespace, &clip.svg_id())),
            )?;
        } else if let Some(mask) = self.mask.clone() {
            mask.remove_child(&self.node).map_err(host)?;
            mask.remove();
            self.mask = None;
        }
        self.accepted = Some(label.clone());
        self.accepted_font = Some(font.clone());
        self.accepted_clip = clip.cloned();
        Ok(())
    }
}
pub(super) struct RetainedSvg {
    namespace: String,
    root: Element,
    title: Element,
    description: Element,
    definitions: Element,
    paints: BTreeMap<String, Element>,
    clips: BTreeMap<String, Element>,
    background: Element,
    group: Element,
    marks: BTreeMap<String, MarkNode>,
    opacity_groups: BTreeMap<String, Element>,
    labels: BTreeMap<usize, LabelNode>,
}
impl RetainedSvg {
    pub fn new(document: &web_sys::Document) -> Result<Self> {
        let namespace = owner_namespace(document)?;
        let root = node(document, "svg")?;
        attr(&root, "id", &namespace)?;
        let title = node(document, "title")?;
        let description = node(document, "desc")?;
        let definitions = node(document, "defs")?;
        let background = node(document, "rect")?;
        let group = node(document, "g")?;
        for (name, value) in [
            ("width", "100%"),
            ("height", "100%"),
            ("preserveAspectRatio", "none"),
            ("role", "img"),
            ("direction", "ltr"),
            ("overflow", "hidden"),
        ] {
            attr(&root, name, value)?;
        }
        attr(&background, "width", "100%")?;
        attr(&background, "height", "100%")?;
        for child in [&title, &description, &definitions, &background, &group] {
            root.append_child(child).map_err(host)?;
        }
        Ok(Self {
            namespace,
            root,
            title,
            description,
            definitions,
            paints: BTreeMap::new(),
            clips: BTreeMap::new(),
            background,
            group,
            marks: BTreeMap::new(),
            opacity_groups: BTreeMap::new(),
            labels: BTreeMap::new(),
        })
    }
    pub fn paint(
        &mut self,
        surface: &HtmlElement,
        scene: &Scene,
        view: ZoomTransform,
    ) -> Result<()> {
        self.paint_with_blends(surface, scene, view, &MarkBlends::default())
    }
    pub fn paint_with_blends(
        &mut self,
        surface: &HtmlElement,
        scene: &Scene,
        view: ZoomTransform,
        blends: &MarkBlends,
    ) -> Result<()> {
        scene.validate()?;
        blends.validate(scene)?;
        let paint_marks = scene
            .marks
            .iter()
            .map(render::paint_mark)
            .collect::<Result<Vec<_>>>()?;
        for mark in &paint_marks {
            if let Some(dash) = &mark.stroke_dash {
                dash.native_normalized()?;
            }
        }
        view.validate()?;
        let document = surface
            .owner_document()
            .ok_or_else(|| ChartError("SVG surface has no document".into()))?;
        attr(
            &self.root,
            "viewBox",
            &format!("0 0 {} {}", scene.width, scene.height),
        )?;
        attr(&self.root, "aria-label", &scene.title)?;
        text(&self.title, &scene.title);
        let description: String = scene
            .legend
            .iter()
            .map(|item| format!("{}; ", item.label))
            .collect();
        text(&self.description, &description);
        attr(
            &self.background,
            "fill",
            &render::css_color(scene.background),
        )?;
        attr(
            &self.group,
            "transform",
            &format!(
                "translate({} {}) scale({})",
                view.translate_x, view.translate_y, view.scale
            ),
        )?;
        let background_in_group = self
            .background
            .parent_node()
            .is_some_and(|p| p.is_same_node(Some(self.group.as_ref())));
        if blends.is_empty() {
            unset(&self.root, "style")?;
            if background_in_group {
                self.root
                    .insert_before(&self.background, Some(self.group.as_ref()))
                    .map_err(host)?;
                unset(&self.background, "x")?;
                unset(&self.background, "y")?;
                attr(&self.background, "width", "100%")?;
                attr(&self.background, "height", "100%")?;
            }
        } else {
            attr(&self.root, "style", "isolation:isolate")?;
            // Keep the backdrop and every mark in the same transformed
            // stacking context. Gecko otherwise drops an outside backdrop
            // even under an identity transform. Undo view on the background.
            if !background_in_group {
                self.group
                    .insert_before(&self.background, self.group.first_child().as_ref())
                    .map_err(host)?;
            }
            for (name, value) in [
                ("x", -view.translate_x / view.scale),
                ("y", -view.translate_y / view.scale),
                ("width", scene.width / view.scale),
                ("height", scene.height / view.scale),
            ] {
                attr(&self.background, name, &value.to_string())?;
            }
        }
        let ids: BTreeSet<_> = scene.marks.iter().map(|m| m.id.as_str()).collect();
        let needed: BTreeSet<_> = paint_marks
            .iter()
            .filter_map(|m| m.fill_paint.as_ref())
            .map(FillPaint::svg_id)
            .collect();
        self.paints.retain(|id, node| {
            if needed.contains(id) {
                true
            } else {
                node.remove();
                false
            }
        });
        for paint in paint_marks.iter().filter_map(|m| m.fill_paint.as_ref()) {
            let id = paint.svg_id();
            if !self.paints.contains_key(&id) {
                let node = gradient(&document, paint, &scoped(&self.namespace, &id))?;
                self.definitions.append_child(&node).map_err(host)?;
                self.paints.insert(id, node);
            }
        }
        let clip_values = || {
            scene
                .marks
                .iter()
                .filter_map(|m| m.clip_path.as_deref())
                .chain(scene.label_clips.values())
        };
        let needed_clips: BTreeSet<_> = clip_values().map(ClipPath::svg_id).collect();
        self.clips.retain(|id, node| {
            if needed_clips.contains(id) {
                true
            } else {
                node.remove();
                false
            }
        });
        for clip in clip_values() {
            let id = clip.svg_id();
            if !self.clips.contains_key(&id) {
                let definition = clip_definition(&document, clip, &scoped(&self.namespace, &id))?;
                self.definitions.append_child(&definition).map_err(host)?;
                self.clips.insert(id, definition);
            }
        }
        self.marks.retain(|id, row| {
            if ids.contains(id.as_str()) {
                true
            } else {
                row.outer().remove();
                false
            }
        });
        self.labels.retain(|index, row| {
            if *index < scene.labels.len() {
                true
            } else {
                row.outer().remove();
                false
            }
        });
        for mark in &paint_marks {
            if !self.marks.contains_key(&mark.id) {
                self.marks
                    .insert(mark.id.clone(), MarkNode::new(&document)?);
            }
            self.marks.get_mut(&mark.id).unwrap().patch(
                &document,
                mark,
                &self.namespace,
                blends.mode(&mark.id),
            )?;
        }
        for (index, label) in scene.labels.iter().enumerate() {
            if let std::collections::btree_map::Entry::Vacant(entry) = self.labels.entry(index) {
                entry.insert(LabelNode {
                    node: node(&document, "text")?,
                    accepted: None,
                    accepted_font: None,
                    mask: None,
                    accepted_clip: None,
                });
            }
            self.labels.get_mut(&index).unwrap().patch(
                &document,
                label,
                &scene.resolved_label_font(index),
                scene.label_clips.get(&index),
                &self.namespace,
            )?;
        }
        // Keep the source opacity on a real parent, never multiply it into
        // the individual member paths. Stable MarkNode outers retain their
        // handlers, identity, blend wrapper and clip/mask chain.
        let group_ids: BTreeSet<_> = scene
            .opacity_groups
            .iter()
            .map(|group| group.id.as_str())
            .collect();
        self.opacity_groups.retain(|id, wrapper| {
            if group_ids.contains(id.as_str()) {
                true
            } else {
                wrapper.remove();
                false
            }
        });
        for group in &scene.opacity_groups {
            if let std::collections::btree_map::Entry::Vacant(entry) =
                self.opacity_groups.entry(group.id.clone())
            {
                let wrapper = node(&document, "g")?;
                // Publish the cache entry only after the new node's fallible
                // writes succeed. No mark has moved into it at this point.
                attr(&wrapper, "data-opacity-group", &group.id)?;
                attr(&wrapper, "opacity", &group.opacity.to_string())?;
                entry.insert(wrapper);
            } else {
                let wrapper = &self.opacity_groups[&group.id];
                attr(wrapper, "data-opacity-group", &group.id)?;
                attr(wrapper, "opacity", &group.opacity.to_string())?;
            }
        }
        let member_groups: BTreeMap<_, _> = scene
            .opacity_groups
            .iter()
            .flat_map(|group| {
                group
                    .members
                    .iter()
                    .map(move |member| (member.as_str(), group.id.as_str()))
            })
            .collect();
        let mut group_children = BTreeMap::<&str, Vec<Element>>::new();
        for item in scene.draw_items() {
            if let DrawItem::Mark(index) = item {
                let id = &scene.marks[index].id;
                if let Some(group_id) = member_groups.get(id.as_str()) {
                    group_children
                        .entry(group_id)
                        .or_default()
                        .push(self.marks[id].outer());
                }
            }
        }
        // Reparent every member before taking the top-level cursor. Taking it
        // first could leave insertBefore pointing at a mark now inside a group.
        // The validated paint order, rather than the metadata list order,
        // determines the exact child order inside each contiguous group.
        for (id, children) in group_children {
            let parent = &self.opacity_groups[id];
            let mut cursor = parent.first_child();
            for child in children {
                if cursor
                    .as_ref()
                    .is_some_and(|current| current.is_same_node(Some(child.as_ref())))
                {
                    cursor = child.next_sibling();
                } else {
                    parent
                        .insert_before(&child, cursor.as_ref())
                        .map_err(host)?;
                }
            }
        }
        let mut emitted_groups = BTreeSet::new();
        // Keep matching children in place. insertBefore moves only the items
        // whose actual paint order changed; it does not recreate any DOM nodes.
        let mut cursor = if blends.is_empty() {
            self.group.first_child()
        } else {
            self.background.next_sibling()
        };
        for item in scene.draw_items() {
            let child = match item {
                DrawItem::Mark(i) => {
                    let id = &scene.marks[i].id;
                    if let Some(group_id) = member_groups.get(id.as_str()) {
                        if !emitted_groups.insert(*group_id) {
                            continue;
                        }
                        self.opacity_groups[*group_id].clone()
                    } else {
                        self.marks[id].outer()
                    }
                }
                DrawItem::Label(i) => self.labels[&i].outer(),
            };
            if cursor
                .as_ref()
                .is_some_and(|current| current.is_same_node(Some(child.as_ref())))
            {
                cursor = child.next_sibling();
            } else {
                self.group
                    .insert_before(&child, cursor.as_ref())
                    .map_err(host)?;
            }
        }
        if self
            .root
            .parent_node()
            .as_ref()
            .is_none_or(|parent| !parent.is_same_node(Some(surface.as_ref())))
        {
            surface.append_child(&self.root).map_err(host)?;
        }
        Ok(())
    }
}
