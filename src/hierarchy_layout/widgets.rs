//! Shared live node roles for the three flat-source hierarchy layouts.
use std::sync::Arc;

use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HierarchyWidgetKind {
    Treemap,
    CirclePacking,
    Icicle,
}

/// One current owned Scene label. Placement can remove or move labels without
/// changing the original datum or replacing its live node component.
#[derive(Clone, Debug, PartialEq)]
pub struct HierarchyNodeWidgetLabel {
    pub label: Label,
    pub clip: Option<ClipPath>,
}

/// Original flat source, shared once across the packet. `datum.value` is read
/// from `source[source_index]`, whereas `node.value` is the aggregate weight.
/// Current geometry/paint belongs to the accepted host Mark. Labels remain
/// independently owned and drawn by the source Scene.
#[derive(Clone, Debug)]
pub struct HierarchyNodeWidgetData {
    pub source: Arc<Vec<HierarchyDatum>>,
    pub source_index: usize,
    pub node: HierarchyLayoutNode,
    pub kind: HierarchyWidgetKind,
    pub root_id: Option<String>,
    pub focus_id: Option<String>,
    pub focus_ancestors: Arc<Vec<String>>,
    pub plot: ClipRect,
    pub formatted_value: String,
    pub labels: Vec<HierarchyNodeWidgetLabel>,
}

fn source_eq(a: &Arc<Vec<HierarchyDatum>>, b: &Arc<Vec<HierarchyDatum>>) -> bool {
    Arc::ptr_eq(a, b)
        || (a.len() == b.len()
            && a.iter().zip(b.iter()).all(|(a, b)| {
                a.id == b.id
                    && a.label == b.label
                    && a.parent == b.parent
                    && a.value.to_bits() == b.value.to_bits()
            }))
}

impl PartialEq for HierarchyNodeWidgetData {
    fn eq(&self, other: &Self) -> bool {
        source_eq(&self.source, &other.source)
            && self.source_index == other.source_index
            && self.node == other.node
            && self.kind == other.kind
            && self.root_id == other.root_id
            && self.focus_id == other.focus_id
            && self.focus_ancestors == other.focus_ancestors
            && self.plot == other.plot
            && self.formatted_value == other.formatted_value
            && self.labels == other.labels
    }
}

#[derive(Clone, Copy, Debug)]
pub struct HierarchyNodeWidgetContext<'a> {
    pub layout: &'a HierarchyLayout,
    pub source: &'a [HierarchyDatum],
    pub datum: &'a HierarchyDatum,
    pub source_index: usize,
    pub node: &'a HierarchyLayoutNode,
    pub mark: &'a Mark,
}

/// Only raw source and construction eligibility are captured. Scene paint and
/// the managed label placement state are deliberately not snapshotted here.
#[derive(Clone, Debug)]
pub(super) struct HierarchyWidgetSource {
    source: Arc<Vec<HierarchyDatum>>,
    kind: HierarchyWidgetKind,
    focus: Option<usize>,
    hidden: BTreeSet<String>,
    leaves_only: bool,
    include_focus: bool,
    clip_plot: bool,
}

impl PartialEq for HierarchyWidgetSource {
    fn eq(&self, other: &Self) -> bool {
        source_eq(&self.source, &other.source)
            && self.kind == other.kind
            && self.focus == other.focus
            && self.hidden == other.hidden
            && self.leaves_only == other.leaves_only
            && self.include_focus == other.include_focus
            && self.clip_plot == other.clip_plot
    }
}

impl HierarchyWidgetSource {
    pub(super) fn new(
        model: &HierarchyModel,
        focus: Option<usize>,
        style: &HierarchyStyleOptions,
        kind: HierarchyWidgetKind,
    ) -> Self {
        Self {
            source: Arc::new(model.data.clone()),
            kind,
            focus,
            hidden: style.hidden.clone(),
            leaves_only: style.leaves_only,
            include_focus: style.include_focus,
            clip_plot: style.clip_plot,
        }
    }
}

struct Checked<'a> {
    marks: BTreeMap<&'a str, &'a Mark>,
    indices: BTreeMap<&'a str, usize>,
}

fn geometry_bounds(kind: HierarchyWidgetKind, geometry: &Geometry) -> Result<ClipRect> {
    let bounds = match (kind, geometry) {
        (HierarchyWidgetKind::CirclePacking, Geometry::Circle { x, y, radius }) if *radius > 0. => {
            ClipRect {
                x: x - radius,
                y: y - radius,
                width: radius * 2.,
                height: radius * 2.,
            }
        }
        (
            HierarchyWidgetKind::Treemap | HierarchyWidgetKind::Icicle,
            Geometry::Rect {
                x,
                y,
                width,
                height,
            },
        ) if *width > 0. && *height > 0. => ClipRect {
            x: *x,
            y: *y,
            width: *width,
            height: *height,
        },
        _ => {
            return Err(ChartError(
                "hierarchy Widget geometry differs from its family".into(),
            ));
        }
    };
    bounds.validate()?;
    Ok(bounds)
}

fn preflight(layout: &HierarchyLayout) -> Result<Checked<'_>> {
    layout.scene.validate()?;
    crate::surface::validate_plot(Some(layout.plot), &layout.scene)?;
    layout.value_format.validate()?;
    let state = &layout.widget_source;
    let source = state.source.as_slice();
    let hierarchy = radial::hierarchy(source)?;
    let indices = source
        .iter()
        .enumerate()
        .map(|(i, d)| (d.id.as_str(), i))
        .collect::<BTreeMap<_, _>>();
    let marks = layout
        .scene
        .marks
        .iter()
        .map(|m| (m.id.as_str(), m))
        .collect::<BTreeMap<_, _>>();
    ensure(
        layout.nodes.len() == source.len()
            && layout.by_id.len() == source.len()
            && layout.by_mark.len() == source.len(),
        "hierarchy Widget source/index count changed",
    )?;
    let Some(mut h) = hierarchy else {
        ensure(
            state.focus.is_none()
                && layout.root_id.is_none()
                && layout.focus_id.is_none()
                && layout.focus_ancestors.is_empty()
                && !layout.scene.marks.iter().any(|m| m.id.starts_with("node:")),
            "empty hierarchy Widget has invented source roles",
        )?;
        layout.validate_managed_labels()?;
        return Ok(Checked { marks, indices });
    };
    let focus = state
        .focus
        .ok_or_else(|| ChartError("hierarchy Widget original focus is missing".into()))?;
    ensure(
        focus < source.len()
            && layout.root_id.as_deref() == Some(source[h.root].id.as_str())
            && layout.focus_id.as_deref() == Some(source[focus].id.as_str()),
        "hierarchy Widget root/focus differs from its original source",
    )?;
    let mut ancestors = vec![];
    let mut ancestor = h.parent[focus];
    while let Some(i) = ancestor {
        ancestors.push(source[i].id.clone());
        ancestor = h.parent[i];
    }
    ancestors.reverse();
    ensure(
        layout.focus_ancestors == ancestors,
        "hierarchy Widget focus ancestry changed",
    )?;
    for children in &mut h.children {
        children.sort_by(|a, b| source[*a].id.cmp(&source[*b].id));
    }
    let mut order = vec![];
    let mut queue = VecDeque::from([h.root]);
    while let Some(i) = queue.pop_front() {
        order.push(i);
        queue.extend(h.children[i].iter().copied());
    }
    let mut members = BTreeSet::new();
    let mut queue = VecDeque::from([focus]);
    while let Some(i) = queue.pop_front() {
        members.insert(i);
        queue.extend(h.children[i].iter().copied());
    }
    let mut owned = BTreeSet::new();
    for (slot, &i) in order.iter().enumerate() {
        let node = &layout.nodes[slot];
        let datum = &source[i];
        let relative = members.contains(&i).then(|| h.depth[i] - h.depth[focus]);
        let children = h.children[i]
            .iter()
            .map(|j| source[*j].id.clone())
            .collect::<Vec<_>>();
        ensure(
            node.id == datum.id
                && node.label == datum.label
                && node.parent == h.parent[i].map(|j| source[j].id.clone())
                && node.children == children
                && node.depth == h.depth[i]
                && node.relative_depth == relative
                && node.value.to_bits() == h.totals[i].to_bits()
                && node.leaf == h.children[i].is_empty()
                && node.mark_id == format!("node:{}", datum.id)
                && layout.by_id.get(&node.id) == Some(&slot)
                && layout.by_mark.get(&node.mark_id) == Some(&slot),
            "hierarchy Widget computed node differs from raw source topology/totals",
        )?;
        let geometry_expected = relative.is_some()
            && node.value > 0.
            && !(state.kind == HierarchyWidgetKind::Icicle && !state.include_focus && i == focus);
        ensure(
            node.geometry.is_some() == geometry_expected,
            "hierarchy Widget allocation eligibility changed",
        )?;
        if let Some(geometry) = &node.geometry {
            let bounds = geometry_bounds(state.kind, geometry)?;
            let needs_content = state.kind == HierarchyWidgetKind::Treemap && !node.leaf;
            ensure(
                node.content_rect.is_some() == needs_content,
                "hierarchy Widget parent content metadata changed",
            )?;
            if let Some(content) = node.content_rect {
                content.validate()?;
                ensure(
                    content.width > 0.
                        && content.height > 0.
                        && content.x >= bounds.x
                        && content.y >= bounds.y
                        && content.x + content.width <= bounds.x + bounds.width
                        && content.y + content.height <= bounds.y + bounds.height,
                    "hierarchy Widget parent content escapes its allocation",
                )?;
            }
        } else {
            ensure(
                node.content_rect.is_none(),
                "unallocated hierarchy Widget owns parent content",
            )?;
        }
        let visible = geometry_expected
            && !state.hidden.contains(&node.id)
            && (!state.leaves_only || node.leaf)
            && (state.include_focus || i != focus);
        ensure(
            node.visible == visible,
            "hierarchy Widget visible eligibility changed",
        )?;
        if visible {
            let mark = marks
                .get(node.mark_id.as_str())
                .ok_or_else(|| ChartError("hierarchy Widget actual node mark is missing".into()))?;
            let text = if node.label.is_empty() {
                &node.id
            } else {
                &node.label
            };
            let expected_label = format!("{text}: {}", layout.value_format.format(node.value)?);
            ensure(
                owned.insert(node.mark_id.as_str())
                    && mark.geometry == *node.geometry.as_ref().unwrap()
                    && mark.series == node.id
                    && mark.value == node.value
                    && mark.label == expected_label
                    && mark.clip == state.clip_plot.then_some(layout.plot),
                "hierarchy Widget actual mark differs from its node/source",
            )?;
        } else {
            ensure(
                !marks.contains_key(node.mark_id.as_str()),
                "ineligible hierarchy Widget has an actual mark",
            )?;
        }
    }
    ensure(
        layout
            .scene
            .marks
            .iter()
            .all(|m| !m.id.starts_with("node:") || owned.contains(m.id.as_str())),
        "orphan hierarchy Widget node mark",
    )?;
    layout.validate_managed_labels()?;
    Ok(Checked { marks, indices })
}

/// Exact current source Scene/plot with explicit painted picking and one typed
/// business site for every actual node, including a no-fill/no-stroke Mark.
/// This intentionally extends the old `interaction_binding()` ink-only filter:
/// empty ink creates no physical hit, but its actual component keeps its role.
pub fn hierarchy_widget_frame(layout: &HierarchyLayout) -> Result<SceneInteraction> {
    let checked = preflight(layout)?;
    let mut binding = InteractionBinding {
        picking_profile: PickingProfile::PaintedGeometry,
        ..Default::default()
    };
    for node in &layout.nodes {
        if !node.visible {
            continue;
        }
        let mark = checked.marks[node.mark_id.as_str()];
        binding.sites.push(InteractionSite {
            mark_id: mark.id.clone(),
            series: node.id.clone(),
            label: if node.label.is_empty() {
                node.id.clone()
            } else {
                node.label.clone()
            },
            value: Some(node.value),
            formatted_value: Some(layout.value_format.format(node.value)?),
            color: mark
                .fill
                .or_else(|| mark.stroke.map(|s| s.color))
                .unwrap_or(Color(0, 0, 0, 0)),
            position: crate::crosshair::mark_center(mark)
                .ok_or_else(|| ChartError("missing hierarchy Widget node anchor".into()))?,
        });
    }
    let frame = SceneInteraction {
        scene: layout.scene.clone(),
        interaction: Some(binding),
        plot: Some(layout.plot),
    };
    frame.validate()?;
    Ok(frame)
}

/// Borrow placement for actual node Marks after whole-frame source, topology,
/// aggregate, focus, eligibility, geometry, index and managed-label preflight.
/// Original raw weights are preserved independently of aggregate values. The
/// flat input is shared by Arc, not cloned per slot; at most 128 selected slots.
/// None skips a role; errors return no packet. Callback side effects cannot roll
/// back. Paint may be coherently edited; managed labels use the existing
/// measurement/placement APIs and remain separately drawn by the source Scene.
///
/// Hidden/zero-weight/out-of-focus/leaves-only/include-focus omissions have no
/// actual Mark and no callback. Upstream Nivo can mount zero-area/radius nodes;
/// this adapter follows Rust's actual-Mark eligibility. Opacity0/no-fill actual
/// Marks stay eligible. Use an accepted static layout or a motion target_layout;
/// do not pair its packet with a sampled intermediate Scene. Automatic springs,
/// React animatedProps and arbitrary Widget export are not supplied here.
pub fn hierarchy_node_widgets(
    layout: &HierarchyLayout,
    revision: u64,
    mut placement: impl FnMut(&HierarchyNodeWidgetContext<'_>) -> Result<Option<PlotWidgetPlacement>>,
) -> Result<PlotWidgetsData<HierarchyNodeWidgetData>> {
    let checked = preflight(layout)?;
    let state = &layout.widget_source;
    let mut packet = PlotWidgetsData {
        layout: PlotWidgetsLayout {
            plot: Some(layout.plot),
            slots: vec![],
        },
        data: BTreeMap::new(),
    };
    let ancestors = Arc::new(layout.focus_ancestors.clone());
    for node in &layout.nodes {
        if !node.visible {
            continue;
        }
        let source_index = checked.indices[node.id.as_str()];
        let context = HierarchyNodeWidgetContext {
            layout,
            source: state.source.as_slice(),
            datum: &state.source[source_index],
            source_index,
            node,
            mark: checked.marks[node.mark_id.as_str()],
        };
        if let Some(placement) = placement(&context)? {
            ensure(
                packet.layout.slots.len() < MAX_PLOT_WIDGET_SLOTS,
                "plot Widget slot budget exceeded",
            )?;
            packet.layout.slots.push(PlotWidgetSlot {
                mark_id: node.mark_id.clone(),
                revision,
                placement,
            });
            packet.data.insert(
                node.mark_id.clone(),
                HierarchyNodeWidgetData {
                    source: state.source.clone(),
                    source_index,
                    node: node.clone(),
                    kind: state.kind,
                    root_id: layout.root_id.clone(),
                    focus_id: layout.focus_id.clone(),
                    focus_ancestors: ancestors.clone(),
                    plot: layout.plot,
                    formatted_value: layout.value_format.format(node.value)?,
                    labels: node
                        .label_slots
                        .iter()
                        .map(|i| HierarchyNodeWidgetLabel {
                            label: layout.scene.labels[*i].clone(),
                            clip: layout.scene.label_clips.get(i).cloned(),
                        })
                        .collect(),
                },
            );
        }
    }
    Ok(packet)
}
