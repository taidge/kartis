//! Equal-area D3 partition and configurable Sunburst presentation. Nivo uses
//! squared radial coordinates and computes node data in breadth-first order.
use crate::{
    CategoryKey, ChartConfig, ChartError, ClipRect, Color, FillPaint, Geometry, InheritedColor,
    InheritedColorContext, InteractionAnchor, InteractionBinding, InteractionSite, Label, Mark,
    OrdinalColorOptions, OrdinalColorScale, PiePadRadius, PolarFrame, Result, Scene,
    SceneInteraction, TextAnchor, ValueFormat,
    pie_layout::pie_arc,
    radial::{Hierarchy, HierarchyDatum, hierarchy_with_self_values},
    scene::{MAX_LABEL_BYTES, ensure},
};
use glory::render::{GraphicStroke, PathCommand, StrokeCap};
use std::{
    collections::{BTreeMap, BTreeSet},
    f64::consts::TAU,
};

mod labels;
pub use labels::{
    MAX_SUNBURST_LABEL_MASK_WORK, SunburstLabelCandidate, SunburstLabelClip,
    SunburstLabelPlacementOptions, SunburstLabelPlacementStats, SunburstMeasuredLabel,
    SunburstPlacedLabel,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SunburstColorBy {
    #[default]
    Id,
    Depth,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SunburstLabelText {
    #[default]
    FormattedValue,
    Label,
    Id,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SunburstLabelOptions {
    pub text: SunburstLabelText,
    /// Fraction across the ring, from the inner edge to the outer edge.
    pub radius_offset: f64,
    /// Minimum unpadded angular span in radians.
    pub skip_angle: f64,
    /// Minimum thickness in chart pixels.
    pub skip_radius: f64,
    pub color: InheritedColor,
    pub size: Option<f64>,
}

impl Default for SunburstLabelOptions {
    fn default() -> Self {
        Self {
            text: SunburstLabelText::FormattedValue,
            radius_offset: 0.5,
            skip_angle: 0.,
            skip_radius: 0.,
            color: InheritedColor::Theme,
            size: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SunburstLayoutOptions {
    pub start_angle: f64,
    pub end_angle: f64,
    /// Fraction of the available radius reserved before the first ring.
    pub inner_ratio: f64,
    /// Include the root as an arc. Its band is reserved either way.
    pub include_root: bool,
    pub corner_radius: f64,
    pub pad_angle: f64,
    pub pad_radius: PiePadRadius,
    pub opacity: f64,
    pub border_width: f64,
    pub border_color: InheritedColor,
    pub colors: OrdinalColorOptions,
    pub color_by: SunburstColorBy,
    pub inherit_color_from_parent: bool,
    pub child_color: InheritedColor,
    /// Override the node color used for painting and inheritance.
    pub colors_by_id: BTreeMap<String, Color>,
    pub paints_by_id: BTreeMap<String, FillPaint>,
    /// Hide exact nodes without changing their allocated angular intervals.
    pub hidden: BTreeSet<String>,
    /// Defaults to the percentage of the entire hierarchy.
    pub value_format: Option<ValueFormat>,
    pub labels: Option<SunburstLabelOptions>,
}

impl Default for SunburstLayoutOptions {
    fn default() -> Self {
        Self {
            start_angle: 0.,
            end_angle: TAU,
            inner_ratio: 0.,
            include_root: false,
            corner_radius: 0.,
            pad_angle: 0.,
            pad_radius: PiePadRadius::Auto,
            opacity: 1.,
            border_width: 1.,
            border_color: InheritedColor::Fixed(Color(255, 255, 255, 255)),
            colors: OrdinalColorOptions::default(),
            color_by: SunburstColorBy::Id,
            inherit_color_from_parent: true,
            child_color: InheritedColor::FromContext(vec![]),
            colors_by_id: BTreeMap::new(),
            paints_by_id: BTreeMap::new(),
            hidden: BTreeSet::new(),
            value_format: None,
            labels: None,
        }
    }
}

impl SunburstLayoutOptions {
    fn validate(&self) -> Result<()> {
        ensure(
            (0. ..=16384.).contains(&self.corner_radius)
                && (0. ..=TAU).contains(&self.pad_angle)
                && match self.pad_radius {
                    PiePadRadius::Auto => true,
                    PiePadRadius::Fixed(radius) => (0. ..=1e6).contains(&radius),
                }
                && (0. ..=1.).contains(&self.opacity)
                && (self.border_width == 0. || (0.000001..=10000.).contains(&self.border_width)),
            "invalid Sunburst arc or border style",
        )?;
        self.border_color.validate()?;
        self.child_color.validate()?;
        ensure(
            self.colors_by_id.len() + self.paints_by_id.len() + self.hidden.len() <= 24_000
                && self
                    .colors_by_id
                    .keys()
                    .chain(self.paints_by_id.keys())
                    .chain(self.hidden.iter())
                    .all(|id| !id.trim().is_empty() && id.len() <= 1000),
            "invalid Sunburst style identity or budget",
        )?;
        for paint in self.paints_by_id.values() {
            paint.validate()?;
        }
        if let Some(format) = &self.value_format {
            format.validate()?;
        }
        if let Some(labels) = &self.labels {
            ensure(
                (0. ..=1.).contains(&labels.radius_offset)
                    && (0. ..=TAU).contains(&labels.skip_angle)
                    && (0. ..=16384.).contains(&labels.skip_radius)
                    && labels.size.is_none_or(|size| (1. ..=256.).contains(&size)),
                "invalid Sunburst label style",
            )?;
            labels.color.validate()?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SunburstArc {
    /// Untrimmed clockwise angles from twelve o'clock, in radians.
    pub angles: [f64; 2],
    pub inner_radius: f64,
    pub outer_radius: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SunburstLayoutNode {
    pub data: HierarchyDatum,
    pub source_index: usize,
    pub depth: usize,
    pub height: usize,
    /// Sum of this node's own weight and its descendants' weights.
    pub value: f64,
    /// Share of the entire hierarchy, in [0, 100].
    pub percentage: f64,
    pub formatted_value: String,
    pub color: Color,
    pub arc: SunburstArc,
    pub anchor: [f64; 2],
    pub mark_id: String,
    pub mark_index: Option<usize>,
    pub label_index: Option<usize>,
}

/// Original source and computed geometry for one eligible owned arc label.
/// A label slot has not yet been assigned when the callback runs. The computed
/// node's data is a copy; datum borrows the original input at source_index.
#[derive(Clone, Copy, Debug)]
pub struct SunburstLabelContext<'a> {
    pub datum: &'a HierarchyDatum,
    pub node: &'a SunburstLayoutNode,
    pub default_text: &'a str,
    /// Actual text anchor, including the existing baseline adjustment.
    /// node.anchor separately retains the logical arc interaction origin.
    pub label_position: [f64; 2],
    pub plot: ClipRect,
}

pub type SunburstLabelFactory<'a> = dyn FnMut(&SunburstLabelContext<'_>) -> Result<String> + 'a;

#[derive(Clone, Debug, PartialEq)]
pub struct SunburstLayout {
    pub scene: Scene,
    pub frame: PolarFrame,
    pub root_id: Option<String>,
    pub total: f64,
    /// Breadth-first order, preserving authored order among siblings.
    pub nodes: Vec<SunburstLayoutNode>,
    pub color_scale: OrdinalColorScale,
    by_id: BTreeMap<String, usize>,
    plot: ClipRect,
    label_layout: labels::SunburstLabelState,
}

impl SunburstLayout {
    pub fn plot(&self) -> ClipRect {
        self.plot
    }

    pub fn node(&self, id: &str) -> Option<&SunburstLayoutNode> {
        self.by_id
            .get(id)
            .and_then(|index| self.nodes.get(*index))
            .filter(|node| node.data.id == id)
    }

    /// The node followed by its ancestors, ending at the root.
    pub fn path(&self, id: &str) -> Option<Vec<&SunburstLayoutNode>> {
        let mut node = self.node(id)?;
        let mut path = vec![node];
        while let Some(parent) = node.data.parent.as_deref() {
            let parent = self.node(parent)?;
            if parent.depth.checked_add(1) != Some(node.depth) {
                return None;
            }
            node = parent;
            path.push(node);
        }
        Some(path)
    }

    pub fn node_for_mark(&self, id: &str) -> Option<&SunburstLayoutNode> {
        self.node(id.strip_prefix("node:")?)
            .filter(|node| node.mark_id == id && node.mark_index.is_some())
    }

    pub fn hit_test(&self, x: f64, y: f64) -> Option<&SunburstLayoutNode> {
        self.scene
            .hit_test(x, y)
            .and_then(|mark| self.node_for_mark(&mark.id))
    }

    pub fn interaction_binding(&self) -> Result<InteractionBinding> {
        let mut binding = InteractionBinding::default();
        for mark in &self.scene.marks {
            let Some(node) = self.node_for_mark(&mark.id) else {
                continue;
            };
            let Geometry::Path(commands) = &mark.geometry else {
                return Err(ChartError("Sunburst binding requires an arc path".into()));
            };
            let points = commands
                .iter()
                .enumerate()
                .filter_map(|(i, command)| (!matches!(command, PathCommand::Close)).then_some(i))
                .collect::<Vec<_>>();
            let anchor =
                InteractionAnchor::path_offset_following(&mark.geometry, node.anchor, &points)?;
            binding.anchors.insert(mark.id.clone(), anchor);
            binding.sites.push(InteractionSite {
                mark_id: mark.id.clone(),
                series: node.data.id.clone(),
                label: mark.label.clone(),
                value: Some(node.value),
                formatted_value: Some(node.formatted_value.clone()),
                color: node.color,
                position: node.anchor,
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
}

fn color_key(data: &HierarchyDatum, depth: usize, o: &SunburstLayoutOptions) -> CategoryKey {
    match o.color_by {
        SunburstColorBy::Id => CategoryKey::String(data.id.clone()),
        SunburstColorBy::Depth => CategoryKey::Number(depth as f64),
    }
}

fn color_context(c: &ChartConfig, color: Color, theme: Color) -> InheritedColorContext {
    InheritedColorContext {
        inherited: color,
        theme,
        surface: color,
        backdrop: c.theme.background,
    }
}

fn partition(data: &[HierarchyDatum], h: &Hierarchy, frame: &PolarFrame) -> Result<Vec<[f64; 2]>> {
    let mut angles = vec![[frame.start_angle; 2]; data.len()];
    angles[h.root] = [frame.start_angle, frame.end_angle];
    for &i in &h.order {
        let [start, end] = angles[i];
        let span = end - start;
        let mut previous = start;
        for (j, &child) in h.children[i].iter().enumerate() {
            let next = if h.totals[child] == 0. {
                previous
            } else if j + 1 == h.children[i].len() && data[i].value == 0. {
                end
            } else {
                (previous + span * (h.totals[child] / h.totals[i]))
                    .clamp(start.min(end), start.max(end))
            };
            ensure(
                h.totals[child] == 0. || next != previous,
                "positive Sunburst node collapsed during angular projection",
            )?;
            angles[child] = [previous, next];
            previous = next;
        }
    }
    Ok(angles)
}

fn append_node(
    layout: &mut SunburstLayout,
    node: &mut SunburstLayoutNode,
    datum: &HierarchyDatum,
    c: &ChartConfig,
    o: &SunburstLayoutOptions,
    factory: &mut Option<&mut SunburstLabelFactory<'_>>,
) -> Result<()> {
    let id = &node.data.id;
    if node.value == 0.
        || o.opacity == 0.
        || o.hidden.contains(id)
        || (node.depth == 0 && !o.include_root)
    {
        return Ok(());
    }
    let [start, end] = node.arc.angles;
    let arc = pie_arc::build(pie_arc::ArcInput {
        center: layout.frame.center,
        inner: node.arc.inner_radius,
        outer: node.arc.outer_radius,
        start,
        end,
        pad: o.pad_angle,
        pad_radius: match o.pad_radius {
            PiePadRadius::Auto => None,
            PiePadRadius::Fixed(radius) => Some(radius),
        },
        corner: o.corner_radius,
    })?;
    let border = if o.border_width == 0. {
        None
    } else {
        let color = o
            .border_color
            .resolve_validated(color_context(c, node.color, c.theme.grid))?;
        (color.3 != 0).then_some(GraphicStroke {
            color,
            width: o.border_width,
            cap: StrokeCap::Butt,
        })
    };
    let paint = o.paints_by_id.get(id);
    if border.is_none() && (arc.outer_collapsed || (node.color.3 == 0 && paint.is_none())) {
        return Ok(());
    }
    let mut mark = Mark::new(&node.mark_id, arc.geometry, node.color);
    let label = if node.data.label.is_empty() {
        id
    } else {
        &node.data.label
    };
    mark.label = label.clone();
    mark.series = id.clone();
    mark.value = node.value;
    mark.fill_paint = paint.cloned();
    mark.stroke = border;
    mark.opacity = o.opacity;
    let scene = &mut layout.scene;
    node.mark_index = Some(scene.marks.len());
    scene.marks.push(mark);

    let thickness = node.arc.outer_radius - node.arc.inner_radius;
    if let Some(labels) = &o.labels
        && !arc.outer_collapsed
        && (end - start).abs() >= labels.skip_angle
        && thickness >= labels.skip_radius
    {
        let default_text = match labels.text {
            SunburstLabelText::FormattedValue => node.formatted_value.clone(),
            SunburstLabelText::Label => label.clone(),
            SunburstLabelText::Id => id.clone(),
        };
        let [x, y] =
            if node.depth == 0 && node.arc.inner_radius == 0. && layout.frame.is_full_turn() {
                layout.frame.center
            } else {
                layout.frame.point(
                    node.arc.inner_radius + thickness * labels.radius_offset,
                    (start + end) / 2.,
                )
            };
        let size = labels.size.unwrap_or(c.theme.font_size);
        let text = if let Some(factory) = factory.as_deref_mut() {
            let text = factory(&SunburstLabelContext {
                datum,
                node,
                default_text: &default_text,
                label_position: [x, y + size * 0.35],
                plot: layout.plot,
            })?;
            ensure(
                text.len() <= MAX_LABEL_BYTES,
                "Sunburst label factory text exceeds 1024 bytes",
            )?;
            text
        } else {
            default_text
        };
        node.label_index = Some(scene.labels.len());
        scene.label_series.insert(scene.labels.len(), id.clone());
        scene.labels.push(Label {
            x,
            y: y + size * 0.35,
            text,
            color: labels
                .color
                .resolve_validated(color_context(c, node.color, c.theme.text))?,
            size,
            anchor: TextAnchor::Middle,
            rotation: 0.,
        });
    }
    Ok(())
}

/// Allocate each sibling in source order, with separate angular and squared
/// radial coordinates. A parent's own weight reserves its unassigned trailing
/// angle in the child ring. The root is available in metadata even when hidden.
pub fn sunburst_layout(
    c: &ChartConfig,
    data: &[HierarchyDatum],
    o: &SunburstLayoutOptions,
) -> Result<SunburstLayout> {
    build_sunburst_layout(c, data, o, None)
}

/// Generate owned arc-label text inside the numeric builder, before measured
/// label signatures and managed state are captured. Only labels passing the
/// existing root/hidden/zero/ink/angle/radius rules invoke the callback, in
/// breadth-first order with authored sibling order. Empty text retains a slot.
/// No callback is retained; failures discard the new frame, but do not undo
/// callback-owned external side effects. Raw data, values and tooltips remain.
pub fn sunburst_layout_with_label_factory(
    c: &ChartConfig,
    data: &[HierarchyDatum],
    o: &SunburstLayoutOptions,
    mut factory: impl FnMut(&SunburstLabelContext<'_>) -> Result<String>,
) -> Result<SunburstLayout> {
    build_sunburst_layout(c, data, o, Some(&mut factory))
}

fn build_sunburst_layout(
    c: &ChartConfig,
    data: &[HierarchyDatum],
    o: &SunburstLayoutOptions,
    mut factory: Option<&mut SunburstLabelFactory<'_>>,
) -> Result<SunburstLayout> {
    let scene = c.scene()?;
    o.validate()?;
    let frame = PolarFrame::from_config(c, o.inner_ratio, o.start_angle, o.end_angle)?;
    let hierarchy = hierarchy_with_self_values(data)?;
    let color_scale = o.colors.resolve(hierarchy.iter().flat_map(|h| {
        h.order
            .iter()
            .copied()
            .filter(|&i| {
                let depth = h.depth[i];
                depth > 0 && (!o.inherit_color_from_parent || depth == 1)
            })
            .map(|i| color_key(&data[i], h.depth[i], o))
    }))?;
    let mut layout = SunburstLayout {
        scene,
        frame,
        root_id: None,
        total: 0.,
        nodes: Vec::with_capacity(data.len()),
        color_scale,
        by_id: BTreeMap::new(),
        plot: ClipRect::plot(c),
        label_layout: Default::default(),
    };
    let Some(h) = hierarchy else {
        return Ok(layout);
    };
    layout.root_id = Some(data[h.root].id.clone());
    layout.total = h.totals[h.root];
    let angles = partition(data, &h, &layout.frame)?;
    let levels = (h.max_depth + 1) as f64;
    let start_radius_squared = layout.frame.inner_radius.powi(2);
    let band = (layout.frame.outer_radius.powi(2) - start_radius_squared) / levels;
    let radius = |level: usize| (start_radius_squared + band * level as f64).sqrt();
    let mut heights = vec![0; data.len()];
    for &i in h.order.iter().rev() {
        if let Some(parent) = h.parent[i] {
            heights[parent] = heights[parent].max(heights[i] + 1);
        }
    }
    let mut colors = vec![c.theme.grid; data.len()];
    for i in h.order {
        let datum = &data[i];
        let depth = h.depth[i];
        let color = if let Some(color) = o.colors_by_id.get(&datum.id) {
            *color
        } else if depth > 1 && o.inherit_color_from_parent {
            let parent = colors[h.parent[i].expect("non-root Sunburst node")];
            o.child_color
                .resolve_validated(color_context(c, parent, c.theme.grid))?
        } else if depth > 0 {
            layout
                .color_scale
                .map_or_insert(color_key(datum, depth, o))?
                .unwrap_or(Color(0, 0, 0, 0))
        } else {
            c.theme.grid
        };
        colors[i] = color;
        let arc = SunburstArc {
            angles: angles[i],
            inner_radius: radius(depth),
            outer_radius: radius(depth + 1),
        };
        ensure(
            arc.outer_radius > arc.inner_radius,
            "Sunburst radial band collapsed during projection",
        )?;
        let value = h.totals[i];
        let fraction = if layout.total > 0. {
            value / layout.total
        } else {
            0.
        };
        let formatted_value = if let Some(format) = &o.value_format {
            format.format(value)?
        } else {
            ValueFormat::Percent(2).format(fraction)?
        };
        ensure(
            formatted_value.len() <= MAX_LABEL_BYTES,
            "Sunburst formatted value exceeds the label budget",
        )?;
        let anchor = if depth == 0 && arc.inner_radius == 0. && layout.frame.is_full_turn() {
            layout.frame.center
        } else {
            layout.frame.point(
                (arc.inner_radius + arc.outer_radius) / 2.,
                (angles[i][0] + angles[i][1]) / 2.,
            )
        };
        let mut node = SunburstLayoutNode {
            data: datum.clone(),
            source_index: i,
            depth,
            height: heights[i],
            value,
            percentage: fraction * 100.,
            formatted_value,
            color,
            arc,
            anchor,
            mark_id: format!("node:{}", datum.id),
            mark_index: None,
            label_index: None,
        };
        append_node(&mut layout, &mut node, datum, c, o, &mut factory)?;
        layout.by_id.insert(datum.id.clone(), layout.nodes.len());
        layout.nodes.push(node);
    }
    layout.scene.validate()?;
    layout.label_layout = labels::SunburstLabelState::capture(&layout)?;
    Ok(layout)
}

pub fn sunburst_with_options(
    c: &ChartConfig,
    data: &[HierarchyDatum],
    o: &SunburstLayoutOptions,
) -> Result<Scene> {
    Ok(sunburst_layout(c, data, o)?.scene)
}
