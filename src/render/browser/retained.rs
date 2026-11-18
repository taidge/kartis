use std::collections::{BTreeMap, BTreeSet};

use super::*;
use crate::ZoomTransform;
use crate::blend::BlendBounds;

// All rectangles are half-open backing-store pixels. Integer boundaries are
// essential when clearing or copying antialiased and partially transparent ink.
type Rect = [u32; 4];

fn intersects(a: Rect, b: Rect) -> bool {
    a[0] < b[2] && a[2] > b[0] && a[1] < b[3] && a[3] > b[1]
}

fn contains(a: Rect, b: Rect) -> bool {
    a[0] <= b[0] && a[1] <= b[1] && a[2] >= b[2] && a[3] >= b[3]
}

fn union(a: Rect, b: Rect) -> Rect {
    [
        a[0].min(b[0]),
        a[1].min(b[1]),
        a[2].max(b[2]),
        a[3].max(b[3]),
    ]
}

fn area(rect: Rect) -> u64 {
    u64::from(rect[2] - rect[0]) * u64::from(rect[3] - rect[1])
}

fn add_region(regions: &mut Vec<Rect>, mut next: Rect) {
    let mut i = 0;
    while i < regions.len() {
        let existing = regions[i];
        if contains(existing, next) {
            return;
        }
        let combined = union(existing, next);
        if intersects(existing, next) || area(combined) <= area(existing) + area(next) {
            next = combined;
            regions.swap_remove(i);
        } else {
            i += 1;
        }
    }
    // Bound intersection work on dense scenes with many tiny disjoint marks.
    if regions.len() == 128 {
        next = regions.drain(..).fold(next, union);
    }
    regions.push(next);
}

#[derive(Clone, Copy, PartialEq)]
struct View {
    transform: ZoomTransform,
    ratio: f64,
    size: [u32; 2],
}

impl View {
    fn pixels(self, bounds: BlendBounds) -> Option<Rect> {
        let mut result = [0; 4];
        for (axis, offset) in [self.transform.translate_x, self.transform.translate_y]
            .into_iter()
            .enumerate()
        {
            let project = |v| (v * self.transform.scale + offset) * self.ratio;
            let limit = f64::from(self.size[axis]);
            result[axis] = (project(bounds.min[axis]).floor() - 2.).clamp(0., limit) as u32;
            result[axis + 2] = (project(bounds.max[axis]).ceil() + 2.).clamp(0., limit) as u32;
        }
        (result[0] < result[2] && result[1] < result[3]).then_some(result)
    }
}

// Label slots may shift when axes, legends or annotations are inserted. Match
// still-identical glyph runs by their placement and text across these edits.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Key {
    Mark(String),
    Label(String, u64, u64, usize),
}

#[derive(Clone, Copy)]
struct Item {
    slot: DrawItem,
    bounds: Option<Rect>,
}

struct Frame {
    scene: Scene,
    text_environment: TextEnvironment,
    view: View,
    items: BTreeMap<Key, Item>,
    order: Vec<Key>,
}

#[derive(PartialEq)]
struct TextEnvironment {
    language: String,
    direction: String,
}

impl TextEnvironment {
    fn read(canvas: &HtmlCanvasElement) -> Result<Self> {
        let mut node: web_sys::Element = canvas.clone().into();
        let language = loop {
            if let Some(source) = node.closest("[lang]").map_err(host_error)? {
                break source.get_attribute("lang").unwrap_or_default();
            }
            let Ok(shadow) = node.get_root_node().dyn_into::<web_sys::ShadowRoot>() else {
                break String::new();
            };
            node = shadow.host();
        };
        let direction = canvas
            .owner_document()
            .and_then(|d| d.default_view())
            .ok_or_else(|| ChartError("Canvas has no window".into()))?
            .get_computed_style(canvas)
            .map_err(host_error)?
            .map(|s| s.get_property_value("direction").map_err(host_error))
            .transpose()?
            .unwrap_or_default();
        Ok(Self {
            language,
            direction,
        })
    }
}

fn same_ink(a: &Scene, ai: DrawItem, b: &Scene, bi: DrawItem) -> bool {
    match (ai, bi) {
        (DrawItem::Mark(i), DrawItem::Mark(j)) => {
            let (a, b) = (&a.marks[i], &b.marks[j]);
            a.geometry == b.geometry
                && a.opacity == b.opacity
                && a.fill_paint == b.fill_paint
                && (a.fill_paint.is_some() || a.fill == b.fill)
                && a.stroke == b.stroke
                && a.stroke_dash == b.stroke_dash
                && a.clip == b.clip
                && a.clip_path == b.clip_path
        }
        (DrawItem::Label(i), DrawItem::Label(j)) => {
            a.labels[i] == b.labels[j]
                && a.resolved_label_font(i) == b.resolved_label_font(j)
                && a.typography.label_strokes.get(&i) == b.typography.label_strokes.get(&j)
                && a.typography.label_whitespace(i) == b.typography.label_whitespace(j)
                && a.label_clips.get(&i) == b.label_clips.get(&j)
        }
        _ => false,
    }
}

fn label_bounds(
    context: &CanvasRenderingContext2d,
    scene: &Scene,
    i: usize,
    view: View,
) -> Result<Option<Rect>> {
    let label = scene.typography.drawable_label(i, &scene.labels[i]);
    let stroke = scene
        .typography
        .label_strokes
        .get(&i)
        .filter(|stroke| stroke.color.3 != 0);
    if label.text.is_empty() || label.color.3 == 0 && stroke.is_none() {
        return Ok(None);
    }
    context.save();
    context.set_font(&scene.resolved_label_font(i).canvas_font(label.size));
    context.set_text_align(match label.anchor {
        TextAnchor::Start => "left",
        TextAnchor::Middle => "center",
        TextAnchor::End => "right",
    });
    context.set_text_baseline("alphabetic");
    if prototype_has_property(&context_prototype(context), "fontKerning") {
        context
            .unchecked_ref::<CanvasTextKerning>()
            .set_font_kerning("normal");
    }
    let metrics = context.measure_text(&label.text);
    context.restore();
    let metrics = metrics.map_err(host_error)?;
    let padding = stroke.map_or(0., |s| s.width / 2.) + 1.;
    let x = [
        -metrics.actual_bounding_box_left() - padding,
        metrics.actual_bounding_box_right() + padding,
    ];
    let y = [
        -metrics.actual_bounding_box_ascent() - padding,
        metrics.actual_bounding_box_descent() + padding,
    ];
    if !x.into_iter().chain(y).all(f64::is_finite) {
        return Ok(Some([0, 0, view.size[0], view.size[1]]));
    }
    let (sin, cos) = label.rotation.to_radians().sin_cos();
    let mut bounds = BlendBounds {
        min: [f64::INFINITY; 2],
        max: [f64::NEG_INFINITY; 2],
    };
    for dx in x {
        for dy in y {
            let point = [label.x + dx * cos - dy * sin, label.y + dx * sin + dy * cos];
            for (axis, v) in point.into_iter().enumerate() {
                bounds.min[axis] = bounds.min[axis].min(v);
                bounds.max[axis] = bounds.max[axis].max(v);
            }
        }
    }
    Ok(view.pixels(bounds))
}

impl Frame {
    fn new(
        scene: &Scene,
        view: View,
        text_environment: TextEnvironment,
        context: &CanvasRenderingContext2d,
        previous: Option<&Frame>,
        fonts_changed: bool,
    ) -> Result<Self> {
        let mut items = BTreeMap::new();
        let mut order = Vec::new();
        let mut occurrences = BTreeMap::new();
        for slot in scene.draw_items() {
            let key = match slot {
                DrawItem::Mark(i) => Key::Mark(scene.marks[i].id.clone()),
                DrawItem::Label(i) => {
                    let l = &scene.labels[i];
                    let occurrence = occurrences
                        .entry((&l.text, l.x.to_bits(), l.y.to_bits()))
                        .or_insert(0);
                    *occurrence += 1;
                    Key::Label(l.text.clone(), l.x.to_bits(), l.y.to_bits(), *occurrence)
                }
            };
            let previous_bounds = previous.and_then(|prev| {
                let old = prev.items.get(&key)?;
                (prev.view == view
                    && !(fonts_changed && matches!(slot, DrawItem::Label(_)))
                    && same_ink(scene, slot, &prev.scene, old.slot))
                .then_some(old.bounds)
            });
            let bounds = match (previous_bounds, slot) {
                (Some(bounds), _) => bounds,
                (None, DrawItem::Mark(i)) => {
                    crate::blend::mark_blend_bounds(&scene.marks[i]).and_then(|b| view.pixels(b))
                }
                (None, DrawItem::Label(i)) => label_bounds(context, scene, i, view)?,
            };
            order.push(key.clone());
            items.insert(key, Item { slot, bounds });
        }
        Ok(Self {
            scene: scene.clone(),
            text_environment,
            view,
            items,
            order,
        })
    }

    fn damage(&self, old: &Self, fonts_changed: bool) -> Vec<Rect> {
        let mut regions = Vec::new();
        let mut add = |item: &Item| {
            if let Some(rect) = item.bounds {
                add_region(&mut regions, rect);
            }
        };
        for (key, item) in &self.items {
            match old.items.get(key) {
                None => add(item),
                Some(before)
                    if (fonts_changed && matches!(item.slot, DrawItem::Label(_)))
                        || !same_ink(&self.scene, item.slot, &old.scene, before.slot) =>
                {
                    add(before);
                    add(item);
                }
                _ => {}
            }
        }
        for (key, item) in &old.items {
            if !self.items.contains_key(key) {
                add(item);
            }
        }
        let old_order = old.order.iter().filter(|key| self.items.contains_key(*key));
        let new_order = self.order.iter().filter(|key| old.items.contains_key(*key));
        for (before, after) in old_order.zip(new_order) {
            if before != after {
                add(&old.items[before]);
                add(&self.items[after]);
            }
        }
        if old.scene.opacity_groups != self.scene.opacity_groups {
            for frame in [old, self] {
                for group in &frame.scene.opacity_groups {
                    for id in &group.members {
                        add(&frame.items[&Key::Mark(id.clone())]);
                    }
                }
            }
        }
        regions
    }

    fn affected(&self, regions: &[Rect]) -> BTreeSet<DrawItem> {
        let mut affected: BTreeSet<_> = self
            .items
            .values()
            .filter(|item| {
                item.bounds
                    .is_some_and(|bounds| regions.iter().any(|r| intersects(*r, bounds)))
            })
            .map(|item| item.slot)
            .collect();
        for group in &self.scene.opacity_groups {
            let slots = || {
                group
                    .members
                    .iter()
                    .map(|id| self.items[&Key::Mark(id.clone())].slot)
            };
            if slots().any(|slot| affected.contains(&slot)) {
                affected.extend(slots());
            }
        }
        affected
    }
}

#[derive(Default)]
pub(crate) struct RetainedCanvas {
    accepted: Option<Frame>,

    fonts_changed: bool,
}

impl RetainedCanvas {
    pub(crate) fn invalidate_fonts(&mut self) {
        self.fonts_changed = true;
    }

    pub(crate) fn invalidate(&mut self) {
        self.accepted = None;
    }

    pub(crate) fn paint(
        &mut self,
        scene: &Scene,
        canvas: &HtmlCanvasElement,
        ratio: f64,
        transform: ZoomTransform,
    ) -> Result<()> {
        let size = [
            (scene.width * ratio).ceil() as u32,
            (scene.height * ratio).ceil() as u32,
        ];
        let view = View {
            ratio,
            transform,
            size,
        };
        let text_environment = TextEnvironment::read(canvas)?;
        let fonts_changed = std::mem::take(&mut self.fonts_changed);
        if !fonts_changed
            && self.accepted.as_ref().is_some_and(|old| {
                old.view == view
                    && old.text_environment == text_environment
                    && [canvas.width(), canvas.height()] == size
                    && old.scene == *scene
            })
        {
            return Ok(());
        }
        // Validate the complete scene and reserve all pattern/blend resources
        // even if the accepted change will only touch a few destination pixels.
        let modes = MarkBlends::default();
        let prepared = prepare_canvas(scene, canvas, ratio, transform, &modes)?;
        let accepted = self.accepted.take();
        let comparable = accepted.as_ref().filter(|old| {
            old.view == view
                && old.text_environment == text_environment
                && old.scene.width == scene.width
                && old.scene.height == scene.height
                && old.scene.background == scene.background
                && [canvas.width(), canvas.height()] == size
        });
        let frame = Frame::new(
            scene,
            view,
            text_environment,
            &prepared.0.context,
            comparable,
            fonts_changed,
        )?;
        if let Some(previous) = comparable {
            let regions = frame.damage(previous, fonts_changed);
            if !regions.is_empty() {
                self.repaint(&frame, prepared, regions, &modes)?;
            }
        } else {
            prepared.paint(scene, &modes)?;
        }
        self.accepted = Some(frame);
        Ok(())
    }

    fn repaint(
        &mut self,
        frame: &Frame,
        prepared: PreparedCanvas,
        regions: Vec<Rect>,
        modes: &MarkBlends,
    ) -> Result<()> {
        // Clipping a crossing glyph or arbitrary path can change its native
        // antialiasing inside the damage boundary. Include such primitives as
        // a whole; axis-aligned rectangle fills can be clipped on pixel edges.
        let mut regions = regions;
        let mut passes = 0;
        let included = loop {
            let affected = frame.affected(&regions);
            let mut grew = false;
            for item in frame
                .items
                .values()
                .filter(|item| affected.contains(&item.slot))
            {
                let rectangular_fill = match item.slot {
                    DrawItem::Mark(i) => {
                        let mark = &frame.scene.marks[i];
                        matches!(mark.geometry, Geometry::Rect { .. })
                            && mark.stroke.is_none()
                            && mark.clip_path.is_none()
                    }
                    _ => false,
                };
                if !rectangular_fill
                    && let Some(bounds) = item.bounds
                    && !regions.iter().any(|r| contains(*r, bounds))
                {
                    add_region(&mut regions, bounds);
                    grew = true;
                }
            }
            if !grew {
                break affected;
            }
            passes += 1;
            // A dense chain of overlapping strokes should not cause unbounded
            // repeated scans across an otherwise linear full paint pass.
            if passes == 16 {
                return prepared.paint(&frame.scene, modes);
            }
        };
        // When the update already replays all visible ink, start a clean
        // recording. A wide chart may have generous empty margins around its
        // plot; that whitespace cannot prevent deferred edge coverage drift.
        if included.len() > 1
            && frame
                .items
                .values()
                .all(|item| item.bounds.is_none() || included.contains(&item.slot))
        {
            return prepared.paint(&frame.scene, modes);
        }
        let mut target = prepared.0;
        target.clip_damage = frame.items.values().any(|item| {
            included.contains(&item.slot)
                && item
                    .bounds
                    .is_some_and(|b| !regions.iter().any(|r| contains(*r, b)))
        });
        target.damage = Some(regions);
        super::super::draw_filtered(&frame.scene, &mut target, modes, |slot| {
            included.contains(&slot)
        })
    }
}
