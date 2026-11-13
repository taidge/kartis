//! Renderer-independent plot interaction with transactional pointer/zoom/brush state.
use crate::annotations::{Brush, ZoomConstraints, ZoomTransform};
use crate::mesh::{PointMesh, SliceAxis, SliceIndex};
use crate::scene::ensure;
use crate::{ClipRect, Mark, Result, Scene};
use std::{
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};
mod editable_brush;
mod painted_geometry;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Picking {
    #[default]
    Geometry,
    Nearest {
        distance: f64,
    },
    SliceX {
        distance: f64,
    },
    SliceY {
        distance: f64,
    },
}
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceOptions {
    pub plot: Option<ClipRect>,
    pub view_mode: SurfaceViewMode,
    pub picking: Picking,
    pub zoom: bool,
    pub pinch: bool,
    pub pan: bool,
    pub brush: bool,
    pub min_scale: f64,
    pub max_scale: f64,
}
impl Default for SurfaceOptions {
    fn default() -> Self {
        Self {
            plot: None,
            view_mode: SurfaceViewMode::Transform,
            picking: Picking::Geometry,
            zoom: true,
            pinch: true,
            pan: true,
            brush: true,
            min_scale: 1.,
            max_scale: 20.,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SurfaceViewMode {
    #[default]
    Transform,
    /// Scene geometry is already laid out for the accepted view's data domains.
    Relayout,
}
impl SurfaceOptions {
    pub fn validate(&self) -> Result<()> {
        let (minimum, maximum) = if self.view_mode == SurfaceViewMode::Relayout {
            (0.000001, 1_000_000.)
        } else {
            (1., 100.)
        };
        ensure(
            self.min_scale.is_finite()
                && self.max_scale.is_finite()
                && self.min_scale >= minimum
                && self.max_scale >= self.min_scale
                && self.max_scale <= maximum,
            "surface scale extent is outside the selected view mode's supported range",
        )?;
        let distance = match self.picking {
            Picking::Geometry => 0.,
            Picking::Nearest { distance }
            | Picking::SliceX { distance }
            | Picking::SliceY { distance } => distance,
        };
        ensure(
            distance.is_finite() && (0. ..=1e6).contains(&distance),
            "invalid picking distance",
        )
    }
}
/// Opaque accepted physical frame identity. A clone retains identity; a
/// successful paint/binding/hidden/view update creates a distinct token.
#[derive(Clone)]
pub struct SurfaceFrameToken(std::sync::Arc<()>);
impl PartialEq for SurfaceFrameToken {
    fn eq(&self, other: &Self) -> bool {
        std::sync::Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for SurfaceFrameToken {}
impl std::fmt::Debug for SurfaceFrameToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SurfaceFrameToken")
    }
}
/// A physical ink hit, its single logical identity and optional accepted raw
/// source snapshot. Live animated mark scalars never become source metadata.
#[derive(Clone, Debug, PartialEq)]
pub struct SurfacePhysicalHit {
    pub physical_mark_id: String,
    pub logical_site_id: String,
    pub scene_point: [f64; 2],
    pub frame_token: SurfaceFrameToken,
    /// Accepted original source captured with this hit, not a live mark scalar.
    pub source_data: Option<crate::InteractionPartDatum>,
}
#[derive(Clone, Debug, PartialEq)]
pub enum SurfaceEvent {
    Hover(Vec<String>),
    /// Only the explicit PaintedGeometry profile emits this event. Appended
    /// after all ordinary logical/view events of the accepted operation.
    PhysicalHover(Option<SurfacePhysicalHit>),
    Selected(Option<String>),
    Activated(String),
    View(ZoomTransform),
    Brush {
        bounds: Brush,
        ids: Vec<String>,
        complete: bool,
    },
    /// Editable selection coordinates are logical viewport pixels. IDs refer
    /// to the current visible data and resolve its accepted typed anchors.
    EditableBrush {
        event: crate::brush::EditableBrushEvent,
        ids: Vec<String>,
    },
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SurfaceInput {
    TouchStart {
        id: i32,
        point: (f64, f64),
    },
    TouchMove {
        id: i32,
        point: (f64, f64),
    },
    TouchEnd {
        id: i32,
        point: (f64, f64),
    },
    TouchCancel {
        id: i32,
    },
    Move {
        id: i32,
        point: (f64, f64),
    },
    Leave,
    Down {
        id: i32,
        point: (f64, f64),
        brush: bool,
    },
    Up {
        id: i32,
        point: (f64, f64),
    },
    Cancel {
        id: i32,
    },
    Wheel {
        point: (f64, f64),
        delta: f64,
    },
    Reset,
    Next,
    Previous,
    First,
    Last,
    Activate,
    Clear,
}
#[derive(Clone, Copy, Debug, PartialEq)]
enum DragKind {
    Pan,
    Brush,
    Click,
}
#[derive(Clone, Copy, Debug, PartialEq)]
struct Drag {
    id: i32,
    start: (f64, f64),
    last: (f64, f64),
    kind: DragKind,
    moved: bool,
}
#[derive(Clone, Copy)]
struct Pinch {
    center: (f64, f64),
    distance: f64,
    view: ZoomTransform,
}
#[derive(Clone)]
struct Index {
    mesh: PointMesh,
    slices: Option<SliceIndex>,
}
struct BoundInteraction {
    data: crate::InteractionBinding,
    lookup: BTreeMap<String, usize>,
}
impl BoundInteraction {
    fn new(data: crate::InteractionBinding, scene: &Scene) -> Result<Self> {
        data.validate(scene)?;
        let lookup = data
            .sites
            .iter()
            .enumerate()
            .map(|(i, s)| (s.mark_id.clone(), i))
            .collect();
        Ok(Self { data, lookup })
    }
}
#[derive(Clone)]
pub struct SurfaceModel {
    scene: Rc<Scene>,
    lookup: Rc<BTreeMap<String, usize>>,
    group_opacities: Rc<BTreeMap<String, f64>>,
    options: SurfaceOptions,
    index: Option<Rc<Index>>,
    interaction: Option<Rc<BoundInteraction>>,
    painted: Option<Rc<painted_geometry::PreparedPainted>>,
    frame_token: SurfaceFrameToken,
    physical_hit: Option<SurfacePhysicalHit>,
    hidden: BTreeSet<String>,
    hovered: Vec<String>,
    hover_point: Option<(f64, f64)>,
    selected: Option<String>,
    view: ZoomTransform,
    brush: Option<Brush>,
    editable_brush: Option<crate::brush::EditableBrush>,
    drag: Option<Drag>,
    touches: BTreeMap<i32, (f64, f64)>,
    pinch: Option<Pinch>,
}
impl SurfaceModel {
    pub fn new(scene: Scene, options: SurfaceOptions) -> Result<Self> {
        Self::new_with_interaction(scene, options, None)
    }
    pub fn new_with_interaction(
        scene: Scene,
        options: SurfaceOptions,
        binding: Option<crate::InteractionBinding>,
    ) -> Result<Self> {
        scene.validate()?;
        options.validate()?;
        validate_plot(options.plot, &scene)?;
        let interaction = binding
            .map(|data| BoundInteraction::new(data, &scene).map(Rc::new))
            .transpose()?;
        let mut model = Self {
            group_opacities: Rc::new(scene.mark_group_opacities()),
            lookup: Rc::new(
                scene
                    .marks
                    .iter()
                    .enumerate()
                    .map(|(i, m)| (m.id.clone(), i))
                    .collect(),
            ),
            scene: Rc::new(scene),
            options,
            index: None,
            interaction,
            painted: None,
            frame_token: SurfaceFrameToken(std::sync::Arc::new(())),
            physical_hit: None,
            hidden: BTreeSet::new(),
            hovered: vec![],
            hover_point: None,
            selected: None,
            view: ZoomTransform::default(),
            brush: None,
            editable_brush: None,
            drag: None,
            touches: BTreeMap::new(),
            pinch: None,
        };
        model.view = model.limits().constrain(&model.view)?;
        model.reindex()?;
        Ok(model)
    }
    pub fn scene(&self) -> &Scene {
        &self.scene
    }
    pub fn frame_token(&self) -> SurfaceFrameToken {
        self.frame_token.clone()
    }
    pub fn hovered_physical_hit(&self) -> Option<&SurfacePhysicalHit> {
        self.physical_hit.as_ref()
    }
    pub fn is_current_hit(&self, hit: &SurfacePhysicalHit) -> bool {
        hit.frame_token == self.frame_token
    }
    /// Query the current strict physical frame without changing hover state.
    /// The point is in viewport logical pixels, as for SurfaceInput::Move.
    pub fn physical_hit_at(&self, point: (f64, f64)) -> Result<Option<SurfacePhysicalHit>> {
        finite(point)?;
        let painted = self.painted.as_ref().ok_or_else(|| {
            crate::ChartError("physical ink query requires PaintedGeometry".into())
        })?;
        if !self.plot().contains(point.0, point.1) {
            return Ok(None);
        }
        let world = self.paint_transform().unproject(point)?;
        Ok(painted
            .hit(world, &self.hidden)
            .map(|(physical, logical)| SurfacePhysicalHit {
                physical_mark_id: physical.into(),
                logical_site_id: logical.into(),
                scene_point: [world.0, world.1],
                frame_token: self.frame_token.clone(),
                source_data: self
                    .interaction
                    .as_ref()
                    .and_then(|b| b.data.part_data.get(physical))
                    .cloned(),
            }))
    }
    fn physical_events(&self, previous: &Self, events: &mut Vec<SurfaceEvent>) {
        // Nested transactional helpers may prepare intermediate physical events.
        // Publish only the final accepted physical observation, after logical events.
        events.retain(|event| !matches!(event, SurfaceEvent::PhysicalHover(_)));
        if (self.painted.is_some() || previous.painted.is_some())
            && self.physical_hit != previous.physical_hit
        {
            events.push(SurfaceEvent::PhysicalHover(self.physical_hit.clone()));
        }
    }
    pub fn interaction_data(&self) -> Option<&crate::InteractionBinding> {
        self.interaction.as_ref().map(|b| &b.data)
    }
    /// Accessibility controls use accepted business values, while the plot may
    /// display intermediate spring coordinates and scalar paint properties.
    /// For PaintedGeometry this projection also removes group factors so an
    /// invisible logical site remains keyboard-addressable. Use only for
    /// plot(false) controls, never for physical rendering or hit testing.
    pub fn controls_scene(&self) -> Scene {
        let mut scene = self.scene.as_ref().clone();
        if let Some(binding) = &self.interaction {
            if binding.data.picking_profile == crate::PickingProfile::PaintedGeometry {
                scene.opacity_groups.clear();
            }
            for mark in &mut scene.marks {
                if let Some(&index) = binding.lookup.get(&mark.id) {
                    // This Scene feeds controls, never the application-owned
                    // plot. Business identity survives an entering zero-alpha
                    // frame without making that invisible frame pointer ink.
                    mark.opacity = 1.;
                    if let Some(value) = binding.data.sites[index].value {
                        mark.value = value;
                    }
                } else {
                    mark.label.clear();
                }
            }
        }
        scene
    }
    /// Labels for bound nullable data whose scalar Scene values cannot encode
    /// a missing value. Apply these with Chart::data_control_labels.
    pub fn data_control_labels(&self) -> BTreeMap<String, String> {
        let Some(binding) = &self.interaction else {
            return BTreeMap::new();
        };
        self.scene
            .marks
            .iter()
            .filter_map(|mark| {
                let site = &binding.data.sites[*binding.lookup.get(&mark.id)?];
                site.value
                    .is_none()
                    .then(|| (mark.id.clone(), format!("{}: missing", mark.label)))
            })
            .collect()
    }
    /// Current slice membership for pointer, focused datum or keyboard selection.
    /// Hidden rows are removed using the same accepted Scene as picking.
    pub fn interaction_ids(&self, ids: &[String]) -> Vec<String> {
        let Some(first) = ids.iter().find(|id| self.datum(id).is_some()) else {
            return vec![];
        };
        if let Some(polar) = self.interaction_data().and_then(|b| b.polar.as_ref()) {
            return polar
                .slice_for_mark(first)
                .map(|slice| {
                    slice
                        .member_ids
                        .iter()
                        .filter(|id| self.datum(id).is_some())
                        .cloned()
                        .collect()
                })
                .unwrap_or_default();
        }
        let mut result = if let Some(slices) = self.index.as_ref().and_then(|i| i.slices.as_ref()) {
            slices
                .slices()
                .iter()
                .find(|slice| slice.ids.contains(first))
                .map(|slice| {
                    slice
                        .ids
                        .iter()
                        .filter(|id| self.datum(id).is_some())
                        .cloned()
                        .collect()
                })
                .unwrap_or_default()
        } else {
            ids.iter()
                .filter(|id| self.datum(id).is_some())
                .cloned()
                .collect::<Vec<_>>()
        };
        if let Some(binding) = &self.interaction {
            // Nivo line slices reverse the source point order. Point picking
            // has one member; explicitly controlled multi-point data stays ordered.
            if self.index.as_ref().is_some_and(|i| i.slices.is_some()) {
                result.sort_by_key(|id| {
                    std::cmp::Reverse(binding.lookup.get(id).copied().unwrap_or(0))
                });
            }
        }
        result
    }
    pub fn active_interaction(&self, ids: &[String]) -> Option<crate::ActiveInteraction> {
        let binding = self.interaction.as_ref()?;
        let ids = self.interaction_ids(ids);
        let first = ids.first()?;
        let polar = binding
            .data
            .polar
            .as_ref()
            .and_then(|p| p.slice_for_mark(first));
        let rows = ids
            .iter()
            .filter_map(|id| {
                let site = &binding.data.sites[*binding.lookup.get(id)?];
                let position = self.current_anchor(self.datum(id)?)?;
                let mut current = site.clone();
                current.position = position;
                Some(current)
            })
            .collect::<Vec<_>>();
        let anchor = if let Some(slice) = polar {
            slice.anchor
        } else {
            self.current_anchor(self.datum(first)?)?
        };
        let slice_id = if let Some(slice) = polar {
            Some(slice.id.clone())
        } else {
            match self.options.picking {
                Picking::SliceX { .. } => {
                    Some(format!("line-slice:x:{:016x}", anchor[0].to_bits()))
                }
                Picking::SliceY { .. } => {
                    Some(format!("line-slice:y:{:016x}", anchor[1].to_bits()))
                }
                _ => None,
            }
        };
        Some(crate::ActiveInteraction {
            slice_id,
            label: polar.map(|p| p.label.clone()),
            anchor,
            ids,
            rows,
        })
    }
    pub fn interaction_overlays(
        &self,
        ids: &[String],
        crosshair: Option<&crate::CrosshairOptions>,
    ) -> Result<Vec<Mark>> {
        let ids = self.interaction_ids(ids);
        let Some(first) = ids.first() else {
            return Ok(vec![]);
        };
        let transform = self.paint_transform();
        let screen = self.plot();
        let a = transform.unproject((screen.x, screen.y))?;
        let b = transform.unproject((screen.x + screen.width, screen.y + screen.height))?;
        let plot = ClipRect {
            x: a.0,
            y: a.1,
            width: b.0 - a.0,
            height: b.1 - a.1,
        };
        if let Some(binding) = self.interaction_data()
            && let Some(polar) = &binding.polar
        {
            let Some(stroke) = binding.polar_spoke else {
                return Ok(vec![]);
            };
            let Some(slice) = polar.slice_for_mark(first) else {
                return Ok(vec![]);
            };
            let center = polar.center();
            let mut mark = Mark::new(
                crate::POLAR_SPOKE_ID,
                crate::Geometry::Path(vec![
                    glory::render::PathCommand::Move(glory::render::GraphicPoint::new(
                        center[0], center[1],
                    )),
                    glory::render::PathCommand::Line(glory::render::GraphicPoint::new(
                        slice.anchor[0],
                        slice.anchor[1],
                    )),
                ]),
                stroke.color,
            );
            mark.label.clear();
            mark.fill = None;
            mark.stroke = Some(glory::render::GraphicStroke {
                width: stroke.width / transform.scale,
                ..stroke
            });
            mark.clip = Some(plot);
            return Ok(vec![mark]);
        }
        let Some(options) = crosshair else {
            return Ok(vec![]);
        };
        let Some(anchor) = self.datum(first).and_then(|mark| self.current_anchor(mark)) else {
            return Ok(vec![]);
        };
        let mut options = options.clone();
        options.stroke.width /= transform.scale;
        if let Some(dash) = &mut options.dash {
            for segment in &mut dash.segments {
                *segment /= transform.scale;
            }
            dash.offset /= transform.scale;
        }
        let axis = match self.options.picking {
            Picking::SliceX { .. } => Some(SliceAxis::X),
            Picking::SliceY { .. } => Some(SliceAxis::Y),
            _ => None,
        };
        // crosshair_marks validates the configured width; keep validation in
        // CSS units and divide only the resulting physical paint afterwards.
        let mut source = options.clone();
        source.stroke.width *= transform.scale;
        let mut marks = crate::crosshair_marks(plot, anchor, &source, axis)?;
        for mark in &mut marks {
            mark.stroke = Some(options.stroke);
        }
        Ok(marks)
    }
    pub fn set_interaction_target(
        &mut self,
        target: &Scene,
        binding: Option<crate::InteractionBinding>,
    ) -> Result<Vec<SurfaceEvent>> {
        let data = binding
            .map(|b| BoundInteraction::new(b, target).map(Rc::new))
            .transpose()?;
        let mut next = self.clone();
        next.interaction = data;
        next.reindex()?;
        let mut events = next.reconcile();
        next.repick(&mut events)?;
        next.physical_events(self, &mut events);
        *self = next;
        Ok(events)
    }
    /// Install target business metadata and an already sampled live frame as
    /// one candidate. Crossfading a new shape must not momentarily bind the
    /// target's Path descriptor to the old shape under the same datum ID.
    pub fn set_interaction_frame(
        &mut self,
        target: &Scene,
        binding: Option<crate::InteractionBinding>,
        live_frame: Scene,
    ) -> Result<Vec<SurfaceEvent>> {
        let data = binding
            .map(|binding| BoundInteraction::new(binding, target).map(Rc::new))
            .transpose()?;
        if let Some(data) = &data {
            let ids = live_frame
                .marks
                .iter()
                .map(|mark| mark.id.as_str())
                .collect::<BTreeSet<_>>();
            ensure(
                data.data
                    .sites
                    .iter()
                    .all(|site| ids.contains(site.mark_id.as_str())),
                "live interaction frame is missing target datum IDs",
            )?;
        }
        if let Some(data) = &data {
            data.data.validate_frame_anchors(&live_frame)?;
        }
        self.replace_scene(live_frame, data)
    }
    pub fn options(&self) -> &SurfaceOptions {
        &self.options
    }
    pub fn hidden(&self) -> &BTreeSet<String> {
        &self.hidden
    }
    pub fn hovered(&self) -> &[String] {
        &self.hovered
    }
    #[cfg(all(feature = "web", target_arch = "wasm32"))]
    pub(crate) fn has_plot_hover(&self) -> bool {
        self.hover_point.is_some()
    }
    pub fn selected(&self) -> Option<&str> {
        self.selected.as_deref()
    }
    pub fn view(&self) -> ZoomTransform {
        self.view
    }
    /// Decompose an accepted-to-proposed virtual input transform for a data
    /// viewport. Pinch movement uses the previous contact centroid, keeping pan
    /// distinct from anchored scale when either data axis saturates.
    pub fn relayout_gesture(&self, previous: &Self) -> Result<crate::ViewportGesture> {
        ensure(
            self.plot() == previous.plot(),
            "relative surface gesture requires the same accepted plot",
        )?;
        let factor = self.view.scale / previous.view.scale;
        let dx = self.view.translate_x - factor * previous.view.translate_x;
        let dy = self.view.translate_y - factor * previous.view.translate_y;
        let anchor = if previous.touches.len() == 2 {
            previous.local(previous.pinch_baseline()?.center)
        } else {
            self.hover_point.map(|p| self.local(p)).unwrap_or((0., 0.))
        };
        let pan = (dx - (1. - factor) * anchor.0, dy - (1. - factor) * anchor.1);
        finite(anchor)?;
        finite(pan)?;
        ensure(
            factor.is_finite() && factor > 0.,
            "relative surface gesture overflow",
        )?;
        Ok(crate::ViewportGesture {
            factor,
            anchor,
            pan,
        })
    }
    pub fn plot(&self) -> ClipRect {
        self.options.plot.unwrap_or(ClipRect {
            x: 0.,
            y: 0.,
            width: self.scene.width,
            height: self.scene.height,
        })
    }
    pub fn paint_transform(&self) -> ZoomTransform {
        if self.options.view_mode == SurfaceViewMode::Relayout {
            return ZoomTransform::default();
        }
        let p = self.plot();
        ZoomTransform {
            scale: self.view.scale,
            translate_x: p.x + self.view.translate_x - p.x * self.view.scale,
            translate_y: p.y + self.view.translate_y - p.y * self.view.scale,
        }
    }
    fn local(&self, point: (f64, f64)) -> (f64, f64) {
        let p = self.plot();
        (point.0 - p.x, point.1 - p.y)
    }
    fn picking_scale(&self) -> f64 {
        if self.options.view_mode == SurfaceViewMode::Relayout {
            1.
        } else {
            self.view.scale
        }
    }
    pub fn brush(&self) -> Option<Brush> {
        self.brush
    }
    pub fn dragging(&self) -> bool {
        self.drag.is_some()
            || !self.touches.is_empty()
            || self
                .editable_brush
                .as_ref()
                .is_some_and(|brush| brush.active())
    }
    pub fn touch_count(&self) -> usize {
        self.touches.len()
    }
    pub fn has_touch(&self, id: i32) -> bool {
        self.touches.contains_key(&id)
    }
    pub fn pinching(&self) -> bool {
        self.pinch.is_some()
    }
    pub fn cancel_drag(&mut self) {
        self.drag = None;
        self.brush = None;
        self.touches.clear();
        self.pinch = None;
        if let Some(brush) = &mut self.editable_brush {
            // The rollback selection was accepted before Begin and remains
            // owned by the validated controller. Cleanup has no observers.
            brush.cancel().expect("validated editable brush rollback");
        }
    }
    pub fn datum(&self, id: &str) -> Option<&Mark> {
        self.lookup
            .get(id)
            .and_then(|i| self.scene.marks.get(*i))
            .filter(|m| self.data(m))
    }
    /// Stable typed business identity for controls and keyboard focus. Pointer
    /// geometry/hover still use `datum`, which requires positive live opacity.
    pub fn logical_datum(&self, id: &str) -> Option<&Mark> {
        let mark = self.lookup.get(id).and_then(|i| self.scene.marks.get(*i))?;
        if let Some(binding) = &self.interaction {
            let site = &binding.data.sites[*binding.lookup.get(id)?];
            if binding.data.picking_profile == crate::PickingProfile::PaintedGeometry {
                return (!self.hidden.contains(&mark.series) && mark.series == site.series)
                    .then_some(mark);
            }
            (!self.hidden.contains(&mark.series)
                && mark.series == site.series
                && !mark.label.is_empty()
                && (mark.has_fill() || mark.stroke.is_some()))
            .then_some(mark)
        } else {
            self.data(mark).then_some(mark)
        }
    }
    fn current_anchor(&self, mark: &Mark) -> Option<[f64; 2]> {
        self.interaction.as_ref().map_or_else(
            || crate::crosshair::mark_center(mark),
            |binding| binding.data.anchor_for(mark).ok().flatten(),
        )
    }
    fn data(&self, m: &Mark) -> bool {
        if let Some(painted) = &self.painted {
            return !self.hidden.contains(&m.series) && painted.has_owner(&m.id);
        }
        !m.series.is_empty()
            && !m.label.is_empty()
            && m.opacity > 0.
            && self
                .group_opacities
                .get(&m.id)
                .is_none_or(|opacity| *opacity > 0.)
            && (m.has_fill() || m.stroke.is_some())
            && !self.hidden.contains(&m.series)
            && self
                .interaction
                .as_ref()
                .is_none_or(|binding| binding.lookup.contains_key(&m.id))
    }
    fn limits(&self) -> ZoomConstraints {
        let plot = self.plot();
        ZoomConstraints {
            min_scale: self.options.min_scale,
            max_scale: self.options.max_scale,
            viewport_width: plot.width,
            viewport_height: plot.height,
            world: if self.options.view_mode == SurfaceViewMode::Relayout {
                Brush {
                    x0: -1_000_000.,
                    y0: -1_000_000.,
                    x1: 1_000_000.,
                    y1: 1_000_000.,
                }
            } else {
                Brush {
                    x0: 0.,
                    y0: 0.,
                    x1: plot.width,
                    y1: plot.height,
                }
            },
        }
    }
    fn reindex(&mut self) -> Result<()> {
        if let Some(binding) = &self.interaction {
            // Business metadata may target a new physical frame. Resolve every
            // present descriptor before accepting it, including Exact mode,
            // whose hit test deliberately bypasses the point index.
            binding.data.validate_frame_anchors(&self.scene)?;
        }
        let strict = self
            .interaction
            .as_ref()
            .is_some_and(|b| b.data.picking_profile == crate::PickingProfile::PaintedGeometry);
        let painted = if strict {
            ensure(
                self.options.picking == Picking::Geometry,
                "PaintedGeometry supports only Geometry picking",
            )?;
            let binding = &self.interaction.as_ref().expect("strict binding").data;
            ensure(
                binding.polar.is_none(),
                "PaintedGeometry cannot use a polar slice index",
            )?;
            Some(Rc::new(painted_geometry::PreparedPainted::prepare(
                &self.scene,
                binding,
            )?))
        } else {
            None
        };
        self.painted = painted;
        self.frame_token = SurfaceFrameToken(std::sync::Arc::new(()));
        self.physical_hit = None;
        self.index = if self.options.picking == Picking::Geometry {
            None
        } else {
            let mut source = self.scene.without_series(&self.hidden);
            let mesh = if self.interaction.is_some() {
                let positions = source
                    .marks
                    .iter()
                    .filter(|mark| self.data(mark))
                    .filter_map(|mark| {
                        self.current_anchor(mark)
                            .map(|anchor| (mark.id.clone(), anchor))
                    })
                    .collect();
                PointMesh::from_scene_at(&source, &positions)?
            } else {
                for mark in &mut source.marks {
                    if !self.data(mark) {
                        mark.label.clear();
                    }
                }
                PointMesh::from_scene(&source)?
            };
            let slices = match self.options.picking {
                Picking::SliceX { .. } => Some(SliceIndex::new(&mesh, SliceAxis::X)),
                Picking::SliceY { .. } => Some(SliceIndex::new(&mesh, SliceAxis::Y)),
                _ => None,
            };
            Some(Rc::new(Index { mesh, slices }))
        };
        Ok(())
    }
    pub fn set_scene(&mut self, scene: Scene) -> Result<Vec<SurfaceEvent>> {
        self.set_scene_with_interaction(scene, None)
    }
    pub fn set_scene_with_interaction(
        &mut self,
        scene: Scene,
        binding: Option<crate::InteractionBinding>,
    ) -> Result<Vec<SurfaceEvent>> {
        let interaction = binding
            .map(|data| BoundInteraction::new(data, &scene).map(Rc::new))
            .transpose()?;
        self.replace_scene(scene, interaction)
    }
    pub fn set_layout(&mut self, candidate: crate::SceneInteraction) -> Result<Vec<SurfaceEvent>> {
        let mut next = self.clone();
        if let Some(plot) = candidate.plot {
            next.options.plot = Some(plot);
        }
        let mut events = next.set_scene_with_interaction(candidate.scene, candidate.interaction)?;
        next.physical_events(self, &mut events);
        *self = next;
        Ok(events)
    }
    /// Hosts retaining property-motion settings can admit the next layout and
    /// its binding transactionally before publishing or creating new tracks.
    pub fn set_layout_for_motion(
        &mut self,
        candidate: crate::SceneInteraction,
        options: &crate::motion::PropertyMotionOptions,
        bindings: &crate::motion::ContourBindings,
    ) -> Result<Vec<SurfaceEvent>> {
        crate::validate_motion_scene(&candidate.scene, options, bindings)?;
        self.set_layout(candidate)
    }
    /// Animation frames retain the already admitted business metadata, while
    /// anchors and hit geometry always come from the actual delivered Scene.
    pub fn set_scene_preserving_interaction(&mut self, scene: Scene) -> Result<Vec<SurfaceEvent>> {
        self.replace_scene(scene, self.interaction.clone())
    }
    fn replace_scene(
        &mut self,
        scene: Scene,
        interaction: Option<Rc<BoundInteraction>>,
    ) -> Result<Vec<SurfaceEvent>> {
        scene.validate()?;
        validate_plot(self.options.plot, &scene)?;
        let mut next = self.clone();
        next.interaction = interaction;
        if scene.width != self.scene.width || scene.height != self.scene.height {
            next.hover_point = None;
        }
        next.lookup = Rc::new(
            scene
                .marks
                .iter()
                .enumerate()
                .map(|(i, m)| (m.id.clone(), i))
                .collect(),
        );
        next.group_opacities = Rc::new(scene.mark_group_opacities());
        next.scene = Rc::new(scene);
        next.view = next.limits().constrain(&next.view)?;
        let mut events = next.cancel_editable_brush()?;
        next.cancel_drag();
        events.extend(next.fit_editable_brush()?);
        next.reindex()?;
        events.extend(next.reconcile());
        next.repick(&mut events)?;
        if next.view != self.view {
            events.push(SurfaceEvent::View(next.view));
        }
        next.physical_events(self, &mut events);
        *self = next;
        Ok(events)
    }
    /// Accept freshly laid-out pixels for the same logical data view. Gesture
    /// baselines survive only when scene and plot dimensions are unchanged.
    pub fn relayout(
        &mut self,
        scene: Scene,
        plot: ClipRect,
        view: ZoomTransform,
        scale_extent: [f64; 2],
        preserve_gestures: bool,
    ) -> Result<Vec<SurfaceEvent>> {
        self.relayout_with_interaction(scene, plot, view, scale_extent, preserve_gestures, None)
    }
    pub fn relayout_with_interaction(
        &mut self,
        scene: Scene,
        plot: ClipRect,
        view: ZoomTransform,
        scale_extent: [f64; 2],
        preserve_gestures: bool,
        binding: Option<crate::InteractionBinding>,
    ) -> Result<Vec<SurfaceEvent>> {
        scene.validate()?;
        let interaction = binding
            .map(|data| BoundInteraction::new(data, &scene).map(Rc::new))
            .transpose()?;
        validate_plot(Some(plot), &scene)?;
        view.validate()?;
        let mut next = self.clone();
        next.interaction = interaction;
        let preserve = preserve_gestures
            && self.options.view_mode == SurfaceViewMode::Relayout
            && self.plot() == plot
            && self.scene.width == scene.width
            && self.scene.height == scene.height;
        next.options.plot = Some(plot);
        next.options.view_mode = SurfaceViewMode::Relayout;
        next.options.min_scale = scale_extent[0];
        next.options.max_scale = scale_extent[1];
        next.options.validate()?;
        next.lookup = Rc::new(
            scene
                .marks
                .iter()
                .enumerate()
                .map(|(i, m)| (m.id.clone(), i))
                .collect(),
        );
        next.group_opacities = Rc::new(scene.mark_group_opacities());
        next.scene = Rc::new(scene);
        next.view = next.limits().constrain(&view)?;
        let mut events = Vec::new();
        if !preserve {
            events.extend(next.cancel_editable_brush()?);
            next.cancel_drag();
        } else if next.pinch.is_some() && next.view != self.view {
            // Data hosts may rebase a virtual gesture transform after independently
            // constraining X/Y. Continue from the accepted contact geometry.
            next.pinch = Some(next.pinch_baseline()?);
        }
        events.extend(next.fit_editable_brush()?);
        next.reindex()?;
        events.extend(next.reconcile());
        next.repick(&mut events)?;
        next.physical_events(self, &mut events);
        *self = next;
        Ok(events)
    }
    pub fn set_hidden(&mut self, hidden: BTreeSet<String>) -> Result<Vec<SurfaceEvent>> {
        ensure(
            hidden.len() <= 256 && hidden.iter().all(|s| !s.is_empty() && s.len() <= 1024),
            "invalid surface hidden series",
        )?;
        let mut next = self.clone();
        next.hidden = hidden;
        let mut events = if next.hidden != self.hidden {
            next.cancel_editable_brush()?
        } else {
            Vec::new()
        };
        next.reindex()?;
        events.extend(next.reconcile());
        next.repick(&mut events)?;
        next.physical_events(self, &mut events);
        *self = next;
        Ok(events)
    }
    pub fn set_selected(&mut self, id: Option<String>) -> Result<Vec<SurfaceEvent>> {
        ensure(
            id.as_ref()
                .is_none_or(|s| s.len() <= 1024 && self.logical_datum(s).is_some()),
            "unknown selected datum",
        )?;
        let mut events = vec![];
        self.selection(id, &mut events);
        Ok(events)
    }
    pub fn set_hovered(&mut self, ids: Vec<String>) -> Result<Vec<SurfaceEvent>> {
        ensure(ids.len() <= crate::MAX_MARKS, "hover budget exceeded")?;
        let mut seen = BTreeSet::new();
        ensure(
            ids.iter()
                .all(|s| seen.insert(s) && self.datum(s).is_some()),
            "unknown or duplicate hovered datum",
        )?;
        let mut events = vec![];
        self.hover_point = None;
        if self.physical_hit.take().is_some() {
            self.hover(ids, &mut events);
            events.push(SurfaceEvent::PhysicalHover(None));
        } else {
            self.hover(ids, &mut events);
        }
        Ok(events)
    }
    pub fn activate(&mut self, id: &str) -> Result<Vec<SurfaceEvent>> {
        ensure(self.logical_datum(id).is_some(), "unknown activated datum")?;
        let mut events = vec![];
        self.selection(Some(id.into()), &mut events);
        events.push(SurfaceEvent::Activated(id.into()));
        Ok(events)
    }
    fn reconcile(&mut self) -> Vec<SurfaceEvent> {
        let mut events = vec![];
        let hover = self
            .hovered
            .iter()
            .filter(|id| self.datum(id).is_some())
            .cloned()
            .collect();
        self.hover(hover, &mut events);
        if self
            .selected
            .as_ref()
            .is_some_and(|id| self.logical_datum(id).is_none())
        {
            self.selection(None, &mut events);
        }
        events
    }
    fn hover(&mut self, next: Vec<String>, events: &mut Vec<SurfaceEvent>) {
        if next.is_empty() {
            self.physical_hit = None;
        }
        if self.hovered != next {
            self.hovered = next.clone();
            events.push(SurfaceEvent::Hover(next));
        }
    }
    fn repick(&mut self, events: &mut Vec<SurfaceEvent>) -> Result<()> {
        if !self.dragging()
            && let Some(point) = self.hover_point
        {
            let ids = self.pick(point)?;
            self.hover(ids, events);
        }
        Ok(())
    }
    fn selection(&mut self, next: Option<String>, events: &mut Vec<SurfaceEvent>) {
        if self.selected != next {
            self.selected = next.clone();
            events.push(SurfaceEvent::Selected(next));
        }
    }
    fn transform(&mut self, next: ZoomTransform, events: &mut Vec<SurfaceEvent>) {
        if self.view != next {
            self.view = next;
            self.frame_token = SurfaceFrameToken(std::sync::Arc::new(()));
            self.physical_hit = None;
            events.push(SurfaceEvent::View(next));
        }
    }
    fn pick(&mut self, point: (f64, f64)) -> Result<Vec<String>> {
        finite(point)?;
        if self.painted.is_some() {
            self.physical_hit = self.physical_hit_at(point)?;
            return Ok(self
                .physical_hit
                .as_ref()
                .map(|h| vec![h.logical_site_id.clone()])
                .unwrap_or_default());
        }
        if !self.plot().contains(point.0, point.1) {
            return Ok(vec![]);
        }
        let world = self.paint_transform().unproject(point)?;
        if let Some(polar) = self
            .interaction
            .as_ref()
            .and_then(|b| b.data.polar.as_ref())
        {
            return Ok(polar
                .pick(world.0, world.1)
                .map(|slice| {
                    slice
                        .member_ids
                        .iter()
                        .filter(|id| self.datum(id).is_some())
                        .cloned()
                        .collect()
                })
                .unwrap_or_default());
        }
        Ok(match self.options.picking {
            Picking::Geometry => {
                let mut picked = Vec::new();
                for m in self
                    .scene
                    .paint_marks()
                    .rev()
                    .filter(|mark| self.data(mark))
                {
                    if self.interaction.is_some()
                        && let Some(geometry) = crate::render::point_stroke_footprint(m)?
                    {
                        let mut ink = m.clone();
                        ink.geometry = geometry;
                        ink.fill = m.stroke.map(|stroke| stroke.color);
                        ink.fill_paint = None;
                        ink.stroke = None;
                        ink.stroke_dash = None;
                        if crate::scene::contains(&ink, world.0, world.1) {
                            picked.push(m.id.clone());
                            break;
                        }
                        continue;
                    }
                    if !(self.interaction.is_some()
                        && m.stroke
                            .is_some_and(|stroke| stroke.cap == glory::render::StrokeCap::Butt)
                        && zero_length_path(&m.geometry))
                        && crate::scene::contains(m, world.0, world.1)
                    {
                        picked.push(m.id.clone());
                        break;
                    }
                }
                picked
            }
            Picking::Nearest { distance } => self
                .index
                .as_ref()
                .and_then(|i| {
                    i.mesh
                        .nearest(world.0, world.1, distance / self.picking_scale())
                })
                .map(|p| vec![p.id.clone()])
                .unwrap_or_default(),
            Picking::SliceX { distance } | Picking::SliceY { distance } => {
                let coordinate = if matches!(self.options.picking, Picking::SliceX { .. }) {
                    world.0
                } else {
                    world.1
                };
                let Some(index) = self.index.as_ref().and_then(|i| i.slices.as_ref()) else {
                    return Ok(vec![]);
                };
                let eligible = |id: &&String| {
                    self.datum(id).is_some_and(|m| {
                        m.clip.is_none_or(|c| c.contains(world.0, world.1))
                            && m.clip_path
                                .as_ref()
                                .is_none_or(|c| c.contains(world.0, world.1))
                    })
                };
                let mut nearest = index.nearest(coordinate, distance / self.picking_scale());
                if nearest.is_some_and(|s| !s.ids.iter().any(|id| eligible(&id))) {
                    nearest = index
                        .slices()
                        .iter()
                        .filter(|s| {
                            (s.coordinate - coordinate).abs() <= distance / self.picking_scale()
                                && s.ids.iter().any(|id| eligible(&id))
                        })
                        .min_by(|a, b| {
                            (a.coordinate - coordinate)
                                .abs()
                                .total_cmp(&(b.coordinate - coordinate).abs())
                                .then_with(|| a.coordinate.total_cmp(&b.coordinate))
                        });
                }
                nearest
                    .map(|s| s.ids.iter().filter(eligible).cloned().collect())
                    .unwrap_or_default()
            }
        })
    }
    /// All inputs are logical viewport coordinates. No callbacks, DOM or global
    /// scheduling run here; rejected inputs retain the complete accepted state.
    pub fn apply(&mut self, input: SurfaceInput) -> Result<Vec<SurfaceEvent>> {
        let mut next = self.clone();
        let mut events = next.input(input)?;
        next.view.validate()?;
        next.physical_events(self, &mut events);
        *self = next;
        Ok(events)
    }
    fn input(&mut self, input: SurfaceInput) -> Result<Vec<SurfaceEvent>> {
        let mut events = vec![];
        match input {
            SurfaceInput::TouchStart { id, point } => {
                finite(point)?;
                if self
                    .editable_brush
                    .as_ref()
                    .is_some_and(|brush| brush.active())
                {
                    return Ok(events);
                }
                if !self.plot().contains(point.0, point.1) {
                    return Ok(events);
                }
                if self.touches.contains_key(&id)
                    || self.touches.len() == 2
                    || (self.touches.is_empty() && self.drag.is_some())
                {
                    return Ok(events);
                }
                if self.touches.len() == 1 && !(self.options.pinch && self.options.zoom) {
                    return Ok(events);
                }
                if self.touches.is_empty() {
                    events.extend(self.input(SurfaceInput::Down {
                        id,
                        point,
                        brush: false,
                    })?);
                }
                self.touches.insert(id, point);
                if self.touches.len() == 2 {
                    self.drag = None;
                    self.brush = None;
                    self.pinch = Some(self.pinch_baseline()?);
                }
                self.hover(vec![], &mut events);
                self.hover_point = None;
            }
            SurfaceInput::TouchMove { id, point } => {
                finite(point)?;
                if !self.touches.contains_key(&id) {
                    return Ok(events);
                }
                if self.touches.get(&id) == Some(&point) {
                    return Ok(events);
                }
                self.touches.insert(id, point);
                if let Some(pinch) = self.pinch {
                    let geometry = self.pinch_baseline()?;
                    if pinch.distance <= 1e-6 {
                        self.pinch = Some(geometry);
                    } else {
                        let ratio = (geometry.distance / pinch.distance).clamp(
                            self.options.min_scale / pinch.view.scale,
                            self.options.max_scale / pinch.view.scale,
                        );
                        let scale = pinch.view.scale * ratio;
                        let anchor = pinch.view.unproject(self.local(pinch.center))?;
                        let center = if self.options.pan {
                            geometry.center
                        } else {
                            pinch.center
                        };
                        let center = self.local(center);
                        let view = ZoomTransform::new(
                            scale,
                            center.0 - anchor.0 * scale,
                            center.1 - anchor.1 * scale,
                        )?;
                        self.transform(self.limits().constrain(&view)?, &mut events);
                    }
                } else {
                    events.extend(self.input(SurfaceInput::Move { id, point })?);
                }
            }
            SurfaceInput::TouchEnd { id, point } => {
                finite(point)?;
                if !self.touches.contains_key(&id) {
                    return Ok(events);
                }
                if self.pinch.is_some() {
                    events.extend(self.input(SurfaceInput::TouchMove { id, point })?);
                    self.touches.remove(&id);
                    self.resume_touch();
                } else {
                    events.extend(self.input(SurfaceInput::Up { id, point })?);
                    self.touches.remove(&id);
                }
            }
            SurfaceInput::TouchCancel { id } => {
                if self.touches.remove(&id).is_some() {
                    self.resume_touch();
                    self.hover(vec![], &mut events);
                }
            }
            SurfaceInput::Move { id, point } => {
                finite(point)?;
                if self
                    .editable_brush
                    .as_ref()
                    .is_some_and(|brush| brush.active())
                {
                    return Ok(events);
                }
                if self.pinch.is_some() {
                    return Ok(events);
                }
                if let Some(mut drag) = self.drag {
                    if drag.id != id {
                        return Ok(events);
                    }
                    drag.moved |= (point.0 - drag.start.0).hypot(point.1 - drag.start.1) > 3.;
                    match drag.kind {
                        DragKind::Pan if drag.moved => {
                            self.hover_point = None;
                            let view = self.view.pan_constrained(
                                (point.0 - drag.last.0, point.1 - drag.last.1),
                                &self.limits(),
                            )?;
                            self.transform(view, &mut events);
                            self.hover(vec![], &mut events);
                        }
                        DragKind::Brush => {
                            let plot = self.plot();
                            let clamp = |p: (f64, f64)| {
                                (
                                    p.0.clamp(plot.x, plot.x + plot.width),
                                    p.1.clamp(plot.y, plot.y + plot.height),
                                )
                            };
                            let a = self.paint_transform().unproject(clamp(drag.start))?;
                            let b = self.paint_transform().unproject(clamp(point))?;
                            self.brush = Brush::from_points(a, b)?
                                .clipped(self.scene.width, self.scene.height)?;
                            if let Some(bounds) = self.brush {
                                events.push(SurfaceEvent::Brush {
                                    bounds,
                                    ids: self.brush_ids(bounds)?,
                                    complete: false,
                                });
                            }
                        }
                        _ => {}
                    }
                    drag.last = point;
                    self.drag = Some(drag);
                } else {
                    self.hover_point = Some(point);
                    let picked = self.pick(point)?;
                    self.hover(picked, &mut events);
                }
            }
            SurfaceInput::Leave => {
                if self.drag.is_none() {
                    self.hover_point = None;
                    self.hover(vec![], &mut events);
                }
            }
            SurfaceInput::Down { id, point, brush } => {
                finite(point)?;
                if self
                    .editable_brush
                    .as_ref()
                    .is_some_and(|brush| brush.active())
                {
                    return Ok(events);
                }
                if !self.plot().contains(point.0, point.1) {
                    return Ok(events);
                }
                if self.drag.is_none() && self.touches.is_empty() {
                    self.drag = Some(Drag {
                        id,
                        start: point,
                        last: point,
                        kind: if brush && self.options.brush && self.editable_brush.is_none() {
                            DragKind::Brush
                        } else if self.options.pan {
                            DragKind::Pan
                        } else {
                            DragKind::Click
                        },
                        moved: false,
                    });
                    self.brush = None;
                }
            }
            SurfaceInput::Up { id, point } => {
                finite(point)?;
                if self.drag.is_some_and(|d| d.id == id) {
                    events.extend(self.input(SurfaceInput::Move { id, point })?);
                    let drag = self.drag.take().unwrap();
                    if drag.kind == DragKind::Brush {
                        if let Some(bounds) = self.brush {
                            events.push(SurfaceEvent::Brush {
                                bounds,
                                ids: self.brush_ids(bounds)?,
                                complete: true,
                            });
                        }
                    } else if !drag.moved {
                        self.hover_point = Some(point);
                        let picked = self.pick(point)?;
                        self.selection(picked.first().cloned(), &mut events);
                        if let Some(id) = picked.first() {
                            events.push(SurfaceEvent::Activated(id.clone()));
                        }
                        self.hover(picked, &mut events);
                    }
                }
            }
            SurfaceInput::Cancel { id } => {
                if self.drag.is_some_and(|d| d.id == id) {
                    self.drag = None;
                    self.brush = None;
                }
            }
            SurfaceInput::Wheel { point, delta } => {
                finite(point)?;
                ensure(delta.is_finite(), "nonfinite wheel delta")?;
                if self.options.zoom
                    && !self
                        .editable_brush
                        .as_ref()
                        .is_some_and(|brush| brush.active())
                    && self.drag.is_none()
                    && self.touches.is_empty()
                    && self.plot().contains(point.0, point.1)
                {
                    let view = self.view.zoom_constrained(
                        (-delta * 0.002).clamp(-20., 20.).exp(),
                        self.local(point),
                        &self.limits(),
                    )?;
                    self.transform(view, &mut events);
                    self.hover_point = Some(point);
                    let picked = self.pick(point)?;
                    self.hover(picked, &mut events);
                }
            }
            SurfaceInput::Reset => {
                events.extend(self.cancel_editable_brush()?);
                let view = self.limits().constrain(&ZoomTransform::default())?;
                self.transform(view, &mut events);
                self.cancel_drag();
                self.hover_point = None;
                self.hover(vec![], &mut events);
            }
            SurfaceInput::Next
            | SurfaceInput::Previous
            | SurfaceInput::First
            | SurfaceInput::Last => {
                let ids: Vec<_> = if let Some(polar) = self
                    .interaction
                    .as_ref()
                    .and_then(|b| b.data.polar.as_ref())
                {
                    polar
                        .slices()
                        .iter()
                        .filter_map(|slice| {
                            slice
                                .member_ids
                                .iter()
                                .find(|id| self.logical_datum(id).is_some())
                                .cloned()
                        })
                        .collect()
                } else {
                    self.scene
                        .paint_marks()
                        .filter(|m| self.logical_datum(&m.id).is_some())
                        .map(|m| m.id.clone())
                        .collect()
                };
                let index = self.selected.as_ref().and_then(|id| {
                    ids.iter().position(|v| {
                        v == id
                            || self
                                .interaction
                                .as_ref()
                                .and_then(|b| b.data.polar.as_ref())
                                .is_some_and(|p| {
                                    p.slice_for_mark(v).map(|s| &s.id)
                                        == p.slice_for_mark(id).map(|s| &s.id)
                                })
                    })
                });
                let next = if ids.is_empty() {
                    None
                } else {
                    Some(
                        ids[match input {
                            SurfaceInput::First => 0,
                            SurfaceInput::Last => ids.len() - 1,
                            SurfaceInput::Previous => {
                                index.map_or(ids.len() - 1, |i| (i + ids.len() - 1) % ids.len())
                            }
                            _ => index.map_or(0, |i| (i + 1) % ids.len()),
                        }]
                        .clone(),
                    )
                };
                self.selection(next, &mut events);
            }
            SurfaceInput::Activate => {
                if let Some(id) = self
                    .hovered
                    .first()
                    .cloned()
                    .or_else(|| self.selected.clone())
                {
                    events.extend(self.activate(&id)?);
                }
            }
            SurfaceInput::Clear => {
                if self.editable_brush.is_some() {
                    events.extend(self.apply_editable_brush(crate::EditableBrushInput::Clear)?);
                }
                self.cancel_drag();
                self.hover_point = None;
                self.hover(vec![], &mut events);
                self.selection(None, &mut events);
            }
        }
        Ok(events)
    }
    fn pinch_baseline(&self) -> Result<Pinch> {
        let mut points = self.touches.values();
        let a = points
            .next()
            .ok_or_else(|| crate::ChartError("pinch requires two contacts".into()))?;
        let b = points
            .next()
            .ok_or_else(|| crate::ChartError("pinch requires two contacts".into()))?;
        let center = (a.0 / 2. + b.0 / 2., a.1 / 2. + b.1 / 2.);
        let distance = (a.0 - b.0).hypot(a.1 - b.1);
        ensure(distance.is_finite(), "pinch distance overflow")?;
        finite(center)?;
        Ok(Pinch {
            center,
            distance,
            view: self.view,
        })
    }
    fn resume_touch(&mut self) {
        self.pinch = None;
        self.brush = None;
        self.drag = self.touches.iter().next().map(|(id, point)| Drag {
            id: *id,
            start: *point,
            last: *point,
            kind: if self.options.pan {
                DragKind::Pan
            } else {
                DragKind::Click
            },
            moved: true,
        });
    }
    fn brush_ids(&self, bounds: Brush) -> Result<Vec<String>> {
        bounds.select(&self.scene.without_series(&self.hidden))
    }
    pub fn transformed_brush(&self) -> Result<Option<Brush>> {
        self.brush
            .map(|b| {
                let a = self.paint_transform().project((b.x0, b.y0))?;
                let z = self.paint_transform().project((b.x1, b.y1))?;
                Brush::from_points(a, z)
            })
            .transpose()
    }
}
/// All drawable subpaths have zero extent. Butt caps paint no ink here, while
/// Round/Square zero-length strokes deliberately retain their real footprint.
fn zero_length_path(geometry: &crate::Geometry) -> bool {
    use glory::render::PathCommand;
    let crate::Geometry::Path(path) = geometry else {
        return false;
    };
    let mut current = None;
    let mut start = None;
    for command in path {
        match command {
            PathCommand::Move(p) => {
                current = Some(*p);
                start = current;
            }
            PathCommand::Line(p) => {
                if current != Some(*p) {
                    return false;
                }
                current = Some(*p);
            }
            PathCommand::Cubic { first, second, end } => {
                if [first, second, end].iter().any(|p| current != Some(**p)) {
                    return false;
                }
                current = Some(*end);
            }
            PathCommand::Close => {
                if current != start {
                    return false;
                }
                current = start;
            }
        }
    }
    true
}
fn finite(point: (f64, f64)) -> Result<()> {
    ensure(
        point.0.is_finite() && point.1.is_finite(),
        "nonfinite pointer coordinate",
    )
}
pub(crate) fn validate_plot(plot: Option<ClipRect>, scene: &Scene) -> Result<()> {
    if let Some(p) = plot {
        ensure(
            [p.x, p.y, p.width, p.height].iter().all(|v| v.is_finite())
                && p.x >= 0.
                && p.y >= 0.
                && p.width > 0.
                && p.height > 0.
                && p.x + p.width <= scene.width
                && p.y + p.height <= scene.height,
            "surface plot must fit inside scene",
        )?;
    }
    Ok(())
}
