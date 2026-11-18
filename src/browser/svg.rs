//! Retained, application-owned SVG. Marks are keyed by stable Scene IDs;
//! labels retain their slot or follow an unambiguously matched tick across slots.
use std::collections::BTreeMap;

use super::*;

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
/// A geometry or color change normally leaves the child list untouched. When
/// it changes, keep the longest already ordered subsequence. For example,
/// moving the first path to the end requires a single DOM move.
fn reconcile_children(
    parent: &Element,
    previous: &mut Vec<Element>,
    children: Vec<Element>,
) -> Result<()> {
    if *previous == children {
        return Ok(());
    }
    let ranks = web_sys::js_sys::Map::new();
    let mut child = parent.first_child();
    let mut rank = 0u32;
    while let Some(node) = child {
        ranks.set(node.as_ref(), &rank.into());
        child = node.next_sibling();
        rank += 1;
    }
    let mut tails: Vec<(u32, usize)> = vec![];
    let mut predecessors = vec![None; children.len()];
    for (i, child) in children.iter().enumerate() {
        let Some(rank) = ranks.get(child.as_ref()).as_f64() else {
            continue;
        };
        let rank = rank as u32;
        let position = tails.partition_point(|&(end, _)| end < rank);
        if position > 0 {
            predecessors[i] = Some(tails[position - 1].1);
        }
        if position == tails.len() {
            tails.push((rank, i));
        } else {
            tails[position] = (rank, i);
        }
    }
    let mut keep = tails.last().map(|&(_, i)| i);
    for (i, child) in children.iter().enumerate().rev() {
        if keep == Some(i) {
            keep = predecessors[i];
        } else {
            parent
                .insert_before(child, children.get(i + 1).map(AsRef::as_ref))
                .map_err(host)?;
        }
    }
    *previous = children;
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
        let accepted = self.accepted.take();
        let old = accepted.as_ref();
        if old.is_none() {
            attr(&self.path, "data-id", &mark.id)?;
        }
        if old.is_none_or(|old| old.series != mark.series) {
            attr(&self.path, "data-series", &mark.series)?;
        }
        if old.is_none_or(|old| old.value != mark.value) {
            attr(&self.path, "data-value", &mark.value.to_string())?;
        }
        if old.is_none_or(|old| old.geometry != mark.geometry) {
            attr(
                &self.path,
                "d",
                &render::path_data(&mark.geometry.commands()),
            )?;
        }
        if old.is_none_or(|old| old.fill != mark.fill || old.fill_paint != mark.fill_paint) {
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
        }
        if old.is_none_or(|old| old.opacity != mark.opacity) {
            attr(&self.path, "opacity", &mark.opacity.to_string())?;
        }
        if old.is_none_or(|old| old.stroke != mark.stroke) {
            if let Some(stroke) = mark.stroke {
                attr(&self.path, "stroke", &render::css_color(stroke.color))?;
                attr(&self.path, "stroke-width", &stroke.width.to_string())?;
                attr(&self.path, "stroke-linecap", render::cap(stroke.cap))?;
            } else {
                for name in ["stroke", "stroke-width", "stroke-linecap"] {
                    unset(&self.path, name)?;
                }
            }
        }
        if old.is_none_or(|old| old.stroke_dash != mark.stroke_dash) {
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
        }
        if old.is_none_or(|old| old.label != mark.label) {
            text(&self.title, &mark.label);
        }
        if old.is_none_or(|old| old.clip != mark.clip) {
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
        }
        if old.is_none_or(|old| {
            old.clip_path != mark.clip_path || old.clip.is_some() != mark.clip.is_some()
        }) {
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
    owner: Option<String>,
    accepted: Option<Label>,
    accepted_font: Option<crate::ResolvedFont>,
    accepted_whitespace: Option<crate::LabelWhitespace>,
    accepted_stroke: Option<crate::LabelStroke>,
    mask: Option<Element>,
    accepted_clip: Option<ClipPath>,
}
impl LabelNode {
    fn outer(&self) -> Element {
        self.mask.as_ref().unwrap_or(&self.node).clone()
    }
    #[allow(clippy::too_many_arguments)]
    fn patch(
        &mut self,
        document: &web_sys::Document,
        label: &Label,
        font: &crate::ResolvedFont,
        whitespace: crate::LabelWhitespace,
        stroke: Option<crate::LabelStroke>,
        clip: Option<&ClipPath>,
        namespace: &str,
    ) -> Result<()> {
        if self.accepted.as_ref() == Some(label)
            && self.accepted_font.as_ref() == Some(font)
            && self.accepted_whitespace == Some(whitespace)
            && self.accepted_stroke == stroke
            && self.accepted_clip.as_ref() == clip
        {
            return Ok(());
        }
        // As with marks, only a fully successful patch may populate the cache.
        self.accepted = None;
        self.accepted_whitespace = None;
        self.accepted_stroke = None;
        for (name, value) in [("x", label.x), ("y", label.y), ("font-size", label.size)] {
            attr(&self.node, name, &value.to_string())?;
        }
        attr(&self.node, "fill", &render::css_color(label.color))?;
        attr(&self.node, "font-family", &font.css_family())?;
        attr(&self.node, "font-weight", &font.weight.to_string())?;
        attr(&self.node, "text-anchor", render::anchor(label.anchor))?;
        if let Some(stroke) = stroke {
            attr(&self.node, "stroke", &render::css_color(stroke.color))?;
            attr(&self.node, "stroke-width", &stroke.width.to_string())?;
            attr(&self.node, "stroke-linejoin", "round")?;
            attr(&self.node, "paint-order", "fill stroke")?;
        } else {
            for name in ["stroke", "stroke-width", "stroke-linejoin", "paint-order"] {
                unset(&self.node, name)?;
            }
        }
        if label.rotation != 0. {
            attr(
                &self.node,
                "transform",
                &format!("rotate({} {} {})", label.rotation, label.x, label.y),
            )?;
        } else {
            unset(&self.node, "transform")?;
        }
        if whitespace == crate::LabelWhitespace::Preserve {
            attr(&self.node, "style", "white-space:pre")?;
        } else {
            unset(&self.node, "style")?;
        }
        text(&self.node, &whitespace.normalize(&label.text));
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
        self.accepted_whitespace = Some(whitespace);
        self.accepted_stroke = stroke;
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
    group_order: BTreeMap<String, Vec<Element>>,
    top_order: Vec<Element>,
    labels: BTreeMap<usize, LabelNode>,
    labels_passive: bool,
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
            group_order: BTreeMap::new(),
            top_order: vec![],
            labels: BTreeMap::new(),
            labels_passive: false,
        })
    }
    pub(super) fn make_labels_passive(&mut self) {
        self.labels_passive = true;
    }
    /// Actual retained paths, never companion marks, own Tree SVG input.
    pub(super) fn native_hit_ids(&self, ids: &BTreeSet<String>) -> Result<()> {
        attr(&self.root, "pointer-events", "none")?;
        attr(&self.background, "pointer-events", "none")?;
        for (id, row) in &self.marks {
            attr(
                &row.path,
                "pointer-events",
                if ids.contains(id) {
                    "visiblePainted"
                } else {
                    "none"
                },
            )?;
        }
        Ok(())
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
        let result = self.patch_scene(surface, scene, view, blends);
        if result.is_err() {
            // A wrapper may have been detached before a later DOM write failed.
            // The retry must reconcile the actual children of the accepted frame.
            self.group_order.clear();
            self.top_order.clear();
        }
        result
    }
    fn patch_scene(
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
        let moves = scene::shifted_label_slots(
            self.labels.iter().filter_map(|(slot, row)| {
                row.accepted
                    .as_ref()
                    .map(|label| (*slot, label, row.owner.as_deref()))
            }),
            scene,
        );
        if !moves.is_empty() {
            self.labels = std::mem::take(&mut self.labels)
                .into_iter()
                .map(|(i, row)| (moves.get(&i).copied().unwrap_or(i), row))
                .collect();
        }
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
                    owner: None,
                    accepted: None,
                    accepted_font: None,
                    accepted_whitespace: None,
                    accepted_stroke: None,
                    mask: None,
                    accepted_clip: None,
                });
            }
            let row = self.labels.get_mut(&index).unwrap();
            row.patch(
                &document,
                label,
                &scene.resolved_label_font(index),
                scene.typography.label_whitespace(index),
                scene.typography.label_strokes.get(&index).copied(),
                scene.label_clips.get(&index),
                &self.namespace,
            )?;
            let owner = scene.label_series.get(&index);
            if row.owner.as_ref() != owner {
                row.owner = owner.cloned();
            }
            if self.labels_passive {
                attr(&self.labels[&index].node, "pointer-events", "none")?;
            }
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
        self.group_order
            .retain(|id, _| group_ids.contains(id.as_str()));
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
        let mut top_children = Vec::with_capacity(scene.marks.len() + scene.labels.len());
        for item in scene.draw_items() {
            let child = match item {
                DrawItem::Mark(i) => {
                    let id = &scene.marks[i].id;
                    if let Some(group_id) = member_groups.get(id.as_str()) {
                        let children = group_children.entry(group_id).or_default();
                        children.push(self.marks[id].outer());
                        if children.len() > 1 {
                            continue;
                        }
                        self.opacity_groups[*group_id].clone()
                    } else {
                        self.marks[id].outer()
                    }
                }
                DrawItem::Label(i) => self.labels[&i].outer(),
            };
            top_children.push(child);
        }
        for (id, children) in group_children {
            reconcile_children(
                &self.opacity_groups[id],
                self.group_order.entry(id.to_owned()).or_default(),
                children,
            )?;
        }
        reconcile_children(&self.group, &mut self.top_order, top_children)?;
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

/// A new SVG occurrence over an already validated source projection. Its paint
/// resources receive an independent namespace, while data-id values continue
/// to name the original business marks. The outer Tree host paints background
/// once; labels are passive and hit eligibility belongs to the projection.
pub(super) fn native_view(
    document: &web_sys::Document,
    source: &Scene,
    view: ZoomTransform,
    hit_ids: &BTreeSet<String>,
) -> Result<Element> {
    native_view_with_decoration_input(document, source, view, hit_ids, false)
}
/// Native annotation ink can intercept input without acquiring a business role.
/// Other existing literal views retain their original passive decoration.
pub(super) fn native_view_with_decoration_input(
    document: &web_sys::Document,
    source: &Scene,
    view: ZoomTransform,
    hit_ids: &BTreeSet<String>,
    decoration_input: bool,
) -> Result<Element> {
    let mut scene = source.clone();
    scene.background = Color(0, 0, 0, 0);
    let scratch: HtmlElement = document
        .create_element("div")
        .map_err(host)?
        .dyn_into()
        .map_err(host)?;
    let mut retained = RetainedSvg::new(document)?;
    retained.paint(&scratch, &scene, view)?;
    attr(&retained.root, "role", "presentation")?;
    attr(&retained.root, "aria-hidden", "true")?;
    attr(&retained.root, "pointer-events", "none")?;
    attr(&retained.background, "pointer-events", "none")?;
    for (id, row) in &retained.marks {
        attr(
            &row.path,
            "pointer-events",
            if hit_ids.contains(id) || decoration_input {
                "visiblePainted"
            } else {
                "none"
            },
        )?;
    }
    for row in retained.labels.values() {
        attr(
            &row.node,
            "pointer-events",
            if decoration_input {
                "visiblePainted"
            } else {
                "none"
            },
        )?;
    }
    retained.root.remove();
    Ok(retained.root.clone())
}
