//! Typed undirected chord layout following Nivo 0.99 and d3-chord 1.0.6.
//! Angles are radians clockwise from twelve o'clock. Ribbon sides are exact
//! quadratic-to-cubic conversions. Circular boundaries use the shared kernel.
use crate::{
    CategoryKey, ChartConfig, ChartError, ClipRect, Color, ColorModifier, Geometry, InheritedColor,
    InheritedColorContext, InteractionAnchor, InteractionBinding, InteractionSite, Label,
    LegendItem, Mark, OrdinalColorOptions, OrdinalColorScale, RelationEdge, RelationGraph,
    RelationLayout, RelationNode, RelationOptions, Result, Scene, SceneInteraction, TextAnchor,
    TooltipDatum, TooltipOptions, ValueFormat, chord_relation_edge_id, pie_layout::pie_arc,
    polar_axes::arc_commands, presentation::tooltip_data_text, scene::ensure,
};
use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as C, StrokeCap};
use std::{
    collections::{BTreeMap, BTreeSet},
    f64::consts::TAU,
};

pub const MAX_CHORD_GROUPS: usize = 64;

#[derive(Clone, Debug, PartialEq)]
pub struct ChordPaintOptions {
    pub opacity: f64,
    pub border_width: f64,
    pub border_color: InheritedColor,
}

impl Default for ChordPaintOptions {
    fn default() -> Self {
        Self {
            opacity: 1.,
            border_width: 1.,
            border_color: InheritedColor::FromContext(vec![ColorModifier::Darker(0.4)]),
        }
    }
}

impl ChordPaintOptions {
    fn validate(&self) -> Result<()> {
        ensure(
            (0. ..=1.).contains(&self.opacity)
                && (self.border_width == 0. || (0.000001..=10000.).contains(&self.border_width)),
            "invalid Chord opacity or border width",
        )?;
        self.border_color.validate()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ChordLabelText {
    #[default]
    Label,
    Id,
    FormattedValue,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ChordLabelOrientation {
    /// Nivo's rotated polar labels, including alignment changes across 180°.
    #[default]
    Polar,
    Horizontal,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChordLabelOptions {
    pub text: ChordLabelText,
    pub offset: f64,
    pub rotation: f64,
    pub orientation: ChordLabelOrientation,
    pub color: InheritedColor,
    pub size: Option<f64>,
}

impl Default for ChordLabelOptions {
    fn default() -> Self {
        Self {
            text: Default::default(),
            offset: 12.,
            rotation: 0.,
            orientation: Default::default(),
            color: InheritedColor::FromContext(vec![ColorModifier::Darker(1.)]),
            size: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChordLayoutOptions {
    pub pad_angle: f64,
    pub inner_radius_ratio: f64,
    /// Ribbon radius is outer_radius * (inner_radius_ratio - this offset).
    pub inner_radius_offset: f64,
    pub arc_style: ChordPaintOptions,
    pub ribbon_style: ChordPaintOptions,
    pub colors: OrdinalColorOptions,
    pub colors_by_id: BTreeMap<String, Color>,
    pub labels_by_id: BTreeMap<String, String>,
    pub value_format: ValueFormat,
    pub labels: Option<ChordLabelOptions>,
    /// Omit hidden arcs and every ribbon incident to a hidden endpoint, while
    /// preserving the complete matrix's angular intervals and color domain.
    pub hidden: BTreeSet<String>,
    pub legend: bool,
}

impl Default for ChordLayoutOptions {
    fn default() -> Self {
        Self {
            pad_angle: 0.,
            inner_radius_ratio: 0.9,
            inner_radius_offset: 0.,
            arc_style: Default::default(),
            ribbon_style: ChordPaintOptions {
                opacity: 0.5,
                ..Default::default()
            },
            colors: Default::default(),
            colors_by_id: Default::default(),
            labels_by_id: Default::default(),
            value_format: Default::default(),
            labels: Some(Default::default()),
            hidden: Default::default(),
            legend: false,
        }
    }
}

impl ChordLayoutOptions {
    fn formatted(&self, value: f64) -> Result<String> {
        let text = self.value_format.format(value)?;
        ensure(
            text.len() <= 1024,
            "Chord formatted value exceeds the text budget",
        )?;
        Ok(text)
    }

    fn validate(&self, groups: usize) -> Result<()> {
        ensure(
            self.pad_angle.is_finite()
                && self.pad_angle >= 0.
                && self.pad_angle * (groups as f64) < TAU
                && (0. ..=1.).contains(&self.inner_radius_ratio)
                && (0. ..=self.inner_radius_ratio).contains(&self.inner_radius_offset),
            "invalid Chord padding or radii",
        )?;
        self.arc_style.validate()?;
        self.ribbon_style.validate()?;
        self.value_format.validate()?;
        ensure(
            self.colors_by_id.len() + self.labels_by_id.len() + self.hidden.len() <= 768
                && self
                    .colors_by_id
                    .keys()
                    .chain(self.labels_by_id.keys())
                    .chain(self.hidden.iter())
                    .all(|id| !id.trim().is_empty() && id.len() <= 512)
                && self.labels_by_id.values().all(|label| label.len() <= 1024),
            "invalid Chord style identity, label or budget",
        )?;
        if let Some(labels) = &self.labels {
            ensure(
                labels.offset.is_finite()
                    && labels.offset.abs() <= 16384.
                    && labels.rotation.is_finite()
                    && labels.rotation.abs() <= 180.
                    && labels.size.is_none_or(|size| (1. ..=256.).contains(&size)),
                "invalid Chord label style",
            )?;
            labels.color.validate()?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChordSubgroup {
    pub id: String,
    pub label: String,
    /// Source row and counterpart column in the authored matrix.
    pub index: usize,
    pub subindex: usize,
    pub value: f64,
    pub formatted_value: String,
    pub angles: [f64; 2],
    pub color: Color,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChordGroup {
    pub id: String,
    pub label: String,
    pub index: usize,
    pub value: f64,
    pub formatted_value: String,
    pub angles: [f64; 2],
    pub color: Color,
    pub anchor: [f64; 2],
    pub mark_id: String,
    pub mark_index: Option<usize>,
    pub label_index: Option<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChordRibbon {
    /// Collision-safe identity for the unordered pair of raw endpoint IDs.
    pub id: String,
    /// The larger matrix direction is the source. Ties preserve row order.
    pub source: ChordSubgroup,
    pub target: ChordSubgroup,
    /// Sum of the two directions, counting a self ribbon's value once.
    pub value: f64,
    pub formatted_value: String,
    pub anchor: [f64; 2],
    pub mark_id: String,
    pub mark_index: Option<usize>,
}

impl ChordRibbon {
    /// Business rows for a two-endpoint tooltip. A self ribbon has two equal
    /// rows, as in Nivo. This data does not create additional hit-test marks.
    pub fn tooltip_rows(&self) -> [TooltipDatum<'_>; 2] {
        [&self.source, &self.target].map(|endpoint| TooltipDatum {
            id: &endpoint.id,
            label: &endpoint.label,
            series: &endpoint.id,
            value: Some(endpoint.value),
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChordLayout {
    pub scene: Scene,
    pub center: [f64; 2],
    pub radius: f64,
    pub inner_radius: f64,
    pub ribbon_radius: f64,
    pub total: f64,
    /// Groups and the full subgroup matrix retain authored index order.
    pub groups: Vec<ChordGroup>,
    pub subgroups: Vec<Vec<ChordSubgroup>>,
    pub ribbons: Vec<ChordRibbon>,
    pub color_scale: OrdinalColorScale,
    group_by_id: BTreeMap<String, usize>,
    group_by_mark: BTreeMap<String, usize>,
    ribbon_by_id: BTreeMap<String, usize>,
    ribbon_by_mark: BTreeMap<String, usize>,
    hidden: BTreeSet<String>,
    value_format: ValueFormat,
    plot: ClipRect,
}

impl ChordLayout {
    pub fn plot(&self) -> ClipRect {
        self.plot
    }

    pub fn group(&self, id: &str) -> Option<&ChordGroup> {
        self.group_by_id
            .get(id)
            .and_then(|&i| self.groups.get(i))
            .filter(|group| group.id == id)
    }

    pub fn ribbon(&self, id: &str) -> Option<&ChordRibbon> {
        self.ribbon_by_id
            .get(id)
            .and_then(|&i| self.ribbons.get(i))
            .filter(|ribbon| ribbon.id == id)
    }

    pub fn group_for_mark(&self, id: &str) -> Option<&ChordGroup> {
        self.group_by_mark
            .get(id)
            .and_then(|&i| self.groups.get(i))
            .filter(|group| group.mark_id == id && group.mark_index.is_some())
    }

    pub fn ribbon_for_mark(&self, id: &str) -> Option<&ChordRibbon> {
        self.ribbon_by_mark
            .get(id)
            .and_then(|&i| self.ribbons.get(i))
            .filter(|ribbon| ribbon.mark_id == id && ribbon.mark_index.is_some())
    }

    /// Render the current group's total or both raw ribbon directions. An
    /// explicit tooltip format overrides the layout's stored value format.
    pub fn tooltip_text(&self, mark_id: &str, options: &TooltipOptions) -> Result<String> {
        let mut options = options.clone();
        if options.value_format == ValueFormat::Auto {
            options.value_format = self.value_format.clone();
        }
        if let Some(group) = self.group_for_mark(mark_id) {
            tooltip_data_text(
                &options,
                &[TooltipDatum {
                    id: &group.id,
                    label: &group.label,
                    series: &group.id,
                    value: Some(group.value),
                }],
            )
        } else if let Some(ribbon) = self.ribbon_for_mark(mark_id) {
            tooltip_data_text(&options, &ribbon.tooltip_rows())
        } else {
            Err(ChartError("unknown Chord tooltip mark".into()))
        }
    }

    pub fn interaction_binding(&self) -> Result<InteractionBinding> {
        let mut binding = InteractionBinding::default();
        for mark in &self.scene.marks {
            let (anchor, formatted) = if let Some(group) = self.group_for_mark(&mark.id) {
                (group.anchor, &group.formatted_value)
            } else if let Some(ribbon) = self.ribbon_for_mark(&mark.id) {
                (ribbon.anchor, &ribbon.formatted_value)
            } else {
                continue;
            };
            let Geometry::Path(commands) = &mark.geometry else {
                return Err(ChartError("Chord binding requires a path".into()));
            };
            let indices = commands
                .iter()
                .enumerate()
                .filter_map(|(i, command)| (!matches!(command, C::Close)).then_some(i))
                .collect::<Vec<_>>();
            binding.anchors.insert(
                mark.id.clone(),
                InteractionAnchor::path_offset_following(&mark.geometry, anchor, &indices)?,
            );
            binding.sites.push(InteractionSite {
                mark_id: mark.id.clone(),
                series: mark.series.clone(),
                label: mark.label.clone(),
                value: Some(mark.value),
                formatted_value: Some(formatted.clone()),
                color: mark.fill.unwrap_or(Color(0, 0, 0, 0)),
                position: anchor,
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

    /// Bind the current authored ink to shared topology-based highlighting.
    /// Layout-level hidden groups are source-excluded for this snapshot.
    pub fn relation_layout(&self, options: &RelationOptions) -> Result<RelationLayout> {
        let nodes = self
            .groups
            .iter()
            .map(|group| {
                let mut node = RelationNode::new(&group.id);
                node.visible = !self.hidden.contains(&group.id);
                if group.mark_index.is_some() {
                    node.mark_ids.push(group.mark_id.clone());
                }
                if let Some(index) = group.label_index {
                    node.label_indices.push(index);
                }
                node
            })
            .collect();
        let edges = self
            .ribbons
            .iter()
            .map(|ribbon| {
                let mut edge = RelationEdge::new(&ribbon.id, &ribbon.source.id, &ribbon.target.id);
                edge.visible = !self.hidden.contains(&ribbon.source.id)
                    && !self.hidden.contains(&ribbon.target.id);
                if ribbon.mark_index.is_some() {
                    edge.mark_ids.push(ribbon.mark_id.clone());
                }
                edge
            })
            .collect();
        RelationLayout::new(
            self.scene.clone(),
            RelationGraph::new(nodes, edges)?,
            options.clone(),
        )
    }
}

fn context(c: &ChartConfig, color: Color, theme: Color) -> InheritedColorContext {
    InheritedColorContext {
        inherited: color,
        theme,
        surface: color,
        backdrop: c.theme.background,
    }
}

fn point(center: [f64; 2], radius: f64, angle: f64) -> P {
    P::new(
        center[0] + radius * angle.sin(),
        center[1] - radius * angle.cos(),
    )
}

fn quadratic(out: &mut Vec<C>, start: P, control: P, end: P) {
    out.push(C::Cubic {
        first: P::new(
            start.x + (control.x - start.x) * (2. / 3.),
            start.y + (control.y - start.y) * (2. / 3.),
        ),
        second: P::new(
            end.x + (control.x - end.x) * (2. / 3.),
            end.y + (control.y - end.y) * (2. / 3.),
        ),
        end,
    });
}

fn ribbon_geometry(
    center: [f64; 2],
    radius: f64,
    source: &ChordSubgroup,
    target: &ChordSubgroup,
) -> (Geometry, [f64; 2]) {
    let [a, b] = source.angles;
    let [c, d] = target.angles;
    let [s0, s1, t0, t1] = [a, b, c, d].map(|angle| point(center, radius, angle));
    let control = P::new(center[0], center[1]);
    let mut path = vec![C::Move(s0)];
    arc_commands(center, radius, a, b, &mut path);
    let anchor = if source.index == target.index {
        quadratic(&mut path, s1, control, s0);
        let p = point(
            center,
            radius * (0.5 + 0.25 * ((b - a) / 2.).cos()),
            (a + b) / 2.,
        );
        [p.x, p.y]
    } else {
        quadratic(&mut path, s1, control, t0);
        if c != d {
            arc_commands(center, radius, c, d, &mut path);
        }
        quadratic(&mut path, t1, control, s0);
        [
            center[0]
                + [s0.x, s1.x, t0.x, t1.x]
                    .iter()
                    .map(|v| (v - center[0]) / 8.)
                    .sum::<f64>(),
            center[1]
                + [s0.y, s1.y, t0.y, t1.y]
                    .iter()
                    .map(|v| (v - center[1]) / 8.)
                    .sum::<f64>(),
        ]
    };
    path.push(C::Close);
    (Geometry::Path(path), anchor)
}

fn paint_mark(
    mark: &mut Mark,
    c: &ChartConfig,
    color: Color,
    style: &ChordPaintOptions,
) -> Result<()> {
    mark.opacity = style.opacity;
    if style.border_width > 0. {
        mark.stroke = Some(GraphicStroke {
            width: style.border_width,
            color: style
                .border_color
                .resolve_validated(context(c, color, c.theme.grid))?,
            cap: StrokeCap::Butt,
        });
    }
    Ok(())
}

fn append_label(
    scene: &mut Scene,
    c: &ChartConfig,
    group: &ChordGroup,
    center: [f64; 2],
    radius: f64,
    options: &ChordLabelOptions,
) -> Result<usize> {
    let angle = (group.angles[0] + group.angles[1]) / 2.;
    let p = point(center, radius + options.offset, angle);
    let mut rotation = 0.;
    let mut anchor = TextAnchor::Middle;
    if options.orientation == ChordLabelOrientation::Polar {
        rotation = angle.to_degrees();
        anchor = if options.rotation > 0. {
            TextAnchor::End
        } else if options.rotation < 0. {
            TextAnchor::Start
        } else {
            TextAnchor::Middle
        };
        if options.rotation != 0. && rotation > 180. {
            rotation -= 180.;
            anchor = if anchor == TextAnchor::Start {
                TextAnchor::End
            } else {
                TextAnchor::Start
            };
        }
        rotation += options.rotation;
        rotation = (rotation + 180.).rem_euclid(360.) - 180.;
    }
    let size = options.size.unwrap_or(c.theme.font_size);
    let baseline = if options.orientation == ChordLabelOrientation::Polar && options.rotation == 0.
    {
        0.
    } else {
        size * 0.35
    };
    let index = scene.labels.len();
    scene.label_series.insert(index, group.id.clone());
    scene.labels.push(Label {
        x: p.x,
        y: p.y + baseline,
        text: match options.text {
            ChordLabelText::Label => group.label.clone(),
            ChordLabelText::Id => group.id.clone(),
            ChordLabelText::FormattedValue => group.formatted_value.clone(),
        },
        color: options
            .color
            .resolve_validated(context(c, group.color, c.theme.text))?,
        size,
        anchor,
        rotation,
    });
    Ok(index)
}

/// Build a Nivo-style undirected Chord from a square matrix. Each row reserves
/// its full outgoing angular extent. Hidden groups keep their slots. All-zero
/// matrices have finite, evenly spaced zero-width metadata and produce no ink.
pub fn chord_layout(
    c: &ChartConfig,
    ids: &[String],
    matrix: &[Vec<f64>],
    o: &ChordLayoutOptions,
) -> Result<ChordLayout> {
    let scene = c.scene()?;
    let n = ids.len();
    ensure(
        n <= MAX_CHORD_GROUPS && matrix.len() == n && matrix.iter().all(|row| row.len() == n),
        "Chord needs a square matrix with at most 64 groups",
    )?;
    let mut seen = BTreeSet::new();
    ensure(
        ids.iter()
            .all(|id| !id.trim().is_empty() && id.len() <= 512 && seen.insert(id)),
        "invalid or duplicate Chord group identity",
    )?;
    ensure(
        matrix
            .iter()
            .flatten()
            .all(|value| value.is_finite() && *value >= 0.),
        "Chord values must be finite and nonnegative",
    )?;
    o.validate(n)?;
    let sums = matrix
        .iter()
        .map(|row| row.iter().sum::<f64>())
        .collect::<Vec<_>>();
    let total = sums.iter().sum::<f64>();
    ensure(total.is_finite(), "Chord total overflow")?;
    let color_scale = o
        .colors
        .resolve(ids.iter().cloned().map(CategoryKey::String))?;
    let center = [
        c.margin.left + c.inner_width() / 2.,
        c.margin.top + c.inner_height() / 2.,
    ];
    let radius = c.inner_width().min(c.inner_height()) / 2.;
    let inner_radius = radius * o.inner_radius_ratio;
    let ribbon_radius = radius * (o.inner_radius_ratio - o.inner_radius_offset);
    let mut layout = ChordLayout {
        scene,
        center,
        radius,
        inner_radius,
        ribbon_radius,
        total,
        groups: Vec::with_capacity(n),
        subgroups: Vec::with_capacity(n),
        ribbons: vec![],
        color_scale,
        group_by_id: Default::default(),
        group_by_mark: Default::default(),
        ribbon_by_id: Default::default(),
        ribbon_by_mark: Default::default(),
        hidden: o.hidden.clone(),
        value_format: o.value_format.clone(),
        plot: ClipRect::plot(c),
    };
    let available = TAU - o.pad_angle * n as f64;
    let mut angle = 0.;
    for (i, id) in ids.iter().enumerate() {
        if total == 0. {
            angle = TAU * i as f64 / n as f64;
        }
        let start = angle;
        let label = o
            .labels_by_id
            .get(id)
            .filter(|label| !label.is_empty())
            .unwrap_or(id)
            .clone();
        let palette_color = layout
            .color_scale
            .map_or_insert(CategoryKey::String(id.clone()))?;
        let color = o
            .colors_by_id
            .get(id)
            .copied()
            .or(palette_color)
            .unwrap_or(Color(0, 0, 0, 0));
        let mut subgroups = Vec::with_capacity(n);
        for (j, &value) in matrix[i].iter().enumerate() {
            let end = if total == 0. {
                angle
            } else {
                angle + (value / total) * available
            };
            ensure(
                value == 0. || end > angle,
                "positive Chord subgroup collapsed during angular projection",
            )?;
            subgroups.push(ChordSubgroup {
                id: id.clone(),
                label: label.clone(),
                index: i,
                subindex: j,
                value,
                formatted_value: o.formatted(value)?,
                angles: [angle, end],
                color,
            });
            angle = end;
        }
        let anchor = point(center, (radius + inner_radius) / 2., (start + angle) / 2.);
        let mark_id = format!("chord-arc:{}:{}", id.len(), id);
        layout.group_by_id.insert(id.clone(), i);
        layout.group_by_mark.insert(mark_id.clone(), i);
        layout.groups.push(ChordGroup {
            id: id.clone(),
            label,
            index: i,
            value: sums[i],
            formatted_value: o.formatted(sums[i])?,
            angles: [start, angle],
            color,
            anchor: [anchor.x, anchor.y],
            mark_id,
            mark_index: None,
            label_index: None,
        });
        layout.subgroups.push(subgroups);
        angle += o.pad_angle;
    }
    for (i, row) in matrix.iter().enumerate() {
        for (j, &forward) in row.iter().enumerate().skip(i) {
            let reverse = matrix[j][i];
            if forward == 0. && reverse == 0. {
                continue;
            }
            let (source, target) = if forward < reverse {
                (&layout.subgroups[j][i], &layout.subgroups[i][j])
            } else {
                (&layout.subgroups[i][j], &layout.subgroups[j][i])
            };
            let id = chord_relation_edge_id(&source.id, &target.id);
            let mark_id = format!("chord-ribbon:{id}");
            let label = format!("{} ↔ {}", source.label, target.label);
            ensure(
                mark_id.len() <= 1024 && label.len() <= 1024,
                "Chord ribbon identity or label exceeds the Scene budget",
            )?;
            let value = if i == j { forward } else { forward + reverse };
            let formatted_value = o.formatted(value)?;
            let (geometry, anchor) = ribbon_geometry(center, ribbon_radius, source, target);
            let mut mark_index = None;
            if ribbon_radius > 0.
                && o.ribbon_style.opacity > 0.
                && !o.hidden.contains(&source.id)
                && !o.hidden.contains(&target.id)
            {
                let mut mark = Mark::new(&mark_id, geometry, source.color);
                mark.label = label;
                mark.series = source.id.clone();
                mark.value = value;
                paint_mark(&mut mark, c, source.color, &o.ribbon_style)?;
                mark_index = Some(layout.scene.marks.len());
                layout.scene.marks.push(mark);
            }
            layout.ribbon_by_id.insert(id.clone(), layout.ribbons.len());
            layout
                .ribbon_by_mark
                .insert(mark_id.clone(), layout.ribbons.len());
            layout.ribbons.push(ChordRibbon {
                id,
                source: source.clone(),
                target: target.clone(),
                value,
                formatted_value,
                anchor,
                mark_id,
                mark_index,
            });
        }
    }
    for group in &mut layout.groups {
        if total == 0. || o.hidden.contains(&group.id) {
            continue;
        }
        if o.arc_style.opacity > 0.
            && (group.value > 0. || (o.arc_style.border_width > 0. && inner_radius < radius))
        {
            let arc = pie_arc::build(pie_arc::ArcInput {
                center,
                inner: inner_radius,
                outer: radius,
                start: group.angles[0],
                end: group.angles[1],
                pad: 0.,
                pad_radius: None,
                corner: 0.,
            })?;
            if !arc.outer_collapsed || o.arc_style.border_width > 0. {
                let mut mark = Mark::new(&group.mark_id, arc.geometry, group.color);
                mark.label = group.label.clone();
                mark.series = group.id.clone();
                mark.value = group.value;
                paint_mark(&mut mark, c, group.color, &o.arc_style)?;
                group.mark_index = Some(layout.scene.marks.len());
                layout.scene.marks.push(mark);
            }
        }
        if let Some(options) = &o.labels {
            group.label_index = Some(append_label(
                &mut layout.scene,
                c,
                group,
                center,
                radius,
                options,
            )?);
        }
    }
    if o.legend {
        layout.scene.legend = layout
            .groups
            .iter()
            .map(|group| LegendItem {
                id: group.id.clone(),
                label: group.label.clone(),
                color: group.color,
            })
            .collect();
    }
    layout.scene.validate()?;
    Ok(layout)
}

pub fn chord_with_options(
    c: &ChartConfig,
    ids: &[String],
    matrix: &[Vec<f64>],
    options: &ChordLayoutOptions,
) -> Result<Scene> {
    Ok(chord_layout(c, ids, matrix, options)?.scene)
}
