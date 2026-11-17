use std::any::{Any, TypeId};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::rc::Rc;

use glory::render::Semantics;
use glory::style::{Display, Length, Overflow, PointerEvents, Position, Profile, Style};
use glory::{Bond, Cage, Scope, Vary, Widget};

use super::WidgetRenderer;
use crate::clip_path::RectCoverage;
use crate::scene::ensure;
use crate::{
    ChartError, ClipPath, ClipRect, Geometry, LayoutRect, Mark, Result, Scene,
    SceneLabelWidgetFrame, SceneWidgetContext, SceneWidgetFrame, SceneWidgetTarget,
    SceneWidgetTransform, SceneWidgetsComposition, TextSize, ZoomTransform,
};
#[cfg(any(test, all(feature = "web", target_arch = "wasm32")))]
use crate::{SceneWidgetSlot, SceneWidgetsLayout};

mod factories;
pub(crate) use factories::{
    composition_factory, plain_factory, scene_composition_factory, typed_factory,
};

/// Maximum simultaneous measured slots for a semantic chart.
pub const MAX_PLOT_WIDGET_SLOTS: usize = 128;

/// A caller-measured overlay placement in Scene coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlotWidgetPlacement {
    Rect(LayoutRect),
    /// Offset of the slot's top-left corner from its datum anchor.
    /// Paths require an accepted interaction anchor on a bound Chart or BrowserChart,
    /// or an explicit rectangle instead.
    AtDatum {
        offset: [f64; 2],
        measured: TextSize,
    },
}

/// Attach one measured Widget slot to a real chart datum.
#[derive(Clone, Debug, PartialEq)]
pub struct PlotWidgetSlot {
    pub mark_id: String,
    /// A new revision asks for a new child while retaining the keyed wrapper.
    pub revision: u64,
    pub placement: PlotWidgetPlacement,
}

/// An application-owned set of measured slots in paint order. The optional
/// rectangle constrains slots to the plot; omission uses the whole Scene.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlotWidgetsLayout {
    pub plot: Option<ClipRect>,
    pub slots: Vec<PlotWidgetSlot>,
}

/// Current accepted context for one slot. Partial slots paint through their
/// clip and are inert, so clipped controls cannot take focus.
#[derive(Clone, Debug, PartialEq)]
pub struct PlotWidgetFrame {
    pub mark: Mark,
    /// Rendered Scene coordinates. BrowserChart applies its current camera
    /// before this rectangle is normalized to the responsive surface box.
    pub rect: LayoutRect,
    /// Visible rectangle, or a conservative rectangular envelope of a
    /// partial BrowserChart path clip. The host draws the actual mask.
    pub visible: Option<LayoutRect>,
    pub interactive: bool,
}

/// The read-only frame updates without replacing the mounted Widget.
#[derive(Clone)]
pub struct PlotWidgetContext {
    pub mark_id: String,
    pub revision: u64,
    pub frame: Bond<PlotWidgetFrame>,
}

/// Measured slots and owned source metadata proposed as one accepted packet.
/// The data keys must exactly equal the slot mark IDs, including empty packets.
/// Metadata is compared by value and updates without rebuilding retained children.
/// Explicit typed packets may attach to an actual decorative Mark whose label
/// is empty, such as a Waffle empty cell. This does not create a logical datum,
/// source control or picking site. The plain slot API keeps its data-Mark rule.
#[derive(Clone, Debug, PartialEq)]
pub struct PlotWidgetsData<T: Clone + PartialEq + 'static> {
    pub layout: PlotWidgetsLayout,
    pub data: BTreeMap<String, T>,
}

/// A strict composition candidate. Replaced mark ink is omitted only when its
/// real child has been built successfully. Labels and source metadata remain.
#[derive(Clone, Debug, PartialEq)]
pub struct PlotWidgetsComposition<T: Clone + PartialEq + 'static> {
    pub widgets: PlotWidgetsData<T>,
    pub replace_ink: BTreeSet<String>,
}

/// Geometry, visibility and typed source from the same accepted slot row.
#[derive(Clone, Debug, PartialEq)]
pub struct TypedPlotWidgetFrame<T: Clone + PartialEq + 'static> {
    pub frame: PlotWidgetFrame,
    pub data: T,
}

/// A live accepted typed frame, independent of the unaccepted input source.
#[derive(Clone)]
pub struct TypedPlotWidgetContext<T: Clone + PartialEq + 'static> {
    pub mark_id: String,
    pub revision: u64,
    pub frame: Bond<TypedPlotWidgetFrame<T>>,
}

/// A tentative child-build frame which becomes live after strict composition
/// commits. Rejection disposes this candidate's scope and Bond. Existing
/// mounted children continue reading their preceding accepted frame until commit.
#[derive(Clone)]
pub struct PlotWidgetCompositionContext<T: Clone + PartialEq + 'static> {
    pub mark_id: String,
    pub revision: u64,
    pub frame: Bond<TypedPlotWidgetFrame<T>>,
}

pub(crate) mod composition;

trait ErasedSlotValue {
    fn as_any(&self) -> &dyn Any;
    fn value_eq(&self, other: &dyn ErasedSlotValue) -> bool;
}
impl<T: Clone + PartialEq + 'static> ErasedSlotValue for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn value_eq(&self, other: &dyn ErasedSlotValue) -> bool {
        other
            .as_any()
            .downcast_ref::<T>()
            .is_some_and(|other| self == other)
    }
}

#[derive(Clone, Default)]
pub(crate) struct SlotPayload(Option<Rc<dyn ErasedSlotValue>>);
#[derive(Clone, PartialEq)]
struct SceneSlotPayload<T: Clone + PartialEq + 'static>(T);
impl SlotPayload {
    fn new<T: Clone + PartialEq + 'static>(value: T) -> Self {
        Self(Some(Rc::new(value)))
    }
    fn get<T: Clone + PartialEq + 'static>(&self) -> Result<T> {
        self.0
            .as_ref()
            .and_then(|value| value.as_any().downcast_ref::<T>())
            .cloned()
            .ok_or_else(|| ChartError("plot Widget metadata type mismatch".into()))
    }
    fn scene_new<T: Clone + PartialEq + 'static>(value: T) -> Self {
        Self::new(SceneSlotPayload(value))
    }
    fn scene_get<T: Clone + PartialEq + 'static>(&self) -> Result<T> {
        self.get::<SceneSlotPayload<T>>().map(|value| value.0)
    }
}
impl PartialEq for SlotPayload {
    fn eq(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (None, None) => true,
            (Some(a), Some(b)) => a.value_eq(b.as_ref()),
            _ => false,
        }
    }
}
impl fmt::Debug for SlotPayload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("SlotPayload")
            .field(&self.0.as_ref().map(|value| value.as_any().type_id()))
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PlotWidgetsPacket {
    pub(crate) layout: PlotWidgetsLayout,
    data: BTreeMap<String, SlotPayload>,
    data_type: Option<TypeId>,
    composition_mode: bool,
    source_ordered: bool,
    replacement_set: BTreeSet<String>,
    source_targets: BTreeMap<String, SceneWidgetTarget>,
    source_replacements: BTreeSet<String>,
    general: bool,
}
impl PlotWidgetsPacket {
    pub(crate) fn plain(layout: PlotWidgetsLayout) -> Self {
        Self {
            layout,
            data: BTreeMap::new(),
            data_type: None,
            composition_mode: false,
            source_ordered: false,
            replacement_set: BTreeSet::new(),
            source_targets: BTreeMap::new(),
            source_replacements: BTreeSet::new(),
            general: false,
        }
    }
    pub(crate) fn typed<T: Clone + PartialEq + 'static>(packet: PlotWidgetsData<T>) -> Self {
        Self {
            layout: packet.layout,
            data: packet
                .data
                .into_iter()
                .map(|(key, value)| (key, SlotPayload::new(value)))
                .collect(),
            data_type: Some(TypeId::of::<T>()),
            composition_mode: false,
            source_ordered: false,
            replacement_set: BTreeSet::new(),
            source_targets: BTreeMap::new(),
            source_replacements: BTreeSet::new(),
            general: false,
        }
    }
    pub(crate) fn composition<T: Clone + PartialEq + 'static>(
        value: PlotWidgetsComposition<T>,
    ) -> Self {
        let mut packet = Self::typed(value.widgets);
        packet.composition_mode = true;
        packet.source_replacements = value.replace_ink.clone();
        packet.replacement_set = value.replace_ink;
        packet
    }
    pub(crate) fn scene_composition<T: Clone + PartialEq + 'static>(
        value: SceneWidgetsComposition<T>,
    ) -> Self {
        let mut packet = Self::plain(PlotWidgetsLayout {
            plot: value.widgets.layout.plot,
            slots: value
                .widgets
                .layout
                .slots
                .iter()
                .map(|slot| PlotWidgetSlot {
                    mark_id: slot.key.clone(),
                    revision: slot.revision,
                    placement: slot.placement,
                })
                .collect(),
        });
        packet.general = true;
        packet.composition_mode = true;
        packet.data_type = Some(TypeId::of::<SceneSlotPayload<T>>());
        packet.data = value
            .widgets
            .data
            .into_iter()
            .map(|(key, value)| (key, SlotPayload::scene_new(value)))
            .collect();
        packet.source_targets = value
            .widgets
            .layout
            .slots
            .into_iter()
            .map(|slot| (slot.key, slot.target))
            .collect();
        packet.source_replacements = value.replace_ink;
        packet.replacement_set = packet
            .source_replacements
            .iter()
            .filter_map(|key| match packet.source_targets.get(key) {
                Some(SceneWidgetTarget::Mark(id)) => Some(id.clone()),
                _ => None,
            })
            .collect();
        packet
    }
    #[cfg(all(feature = "web", target_arch = "wasm32"))]
    pub(crate) fn scene_in_scene<T: Clone + PartialEq + 'static>(
        value: SceneWidgetsComposition<T>,
    ) -> Self {
        let mut packet = Self::scene_composition(value);
        packet.source_ordered = true;
        packet
    }
    #[cfg(all(feature = "web", target_arch = "wasm32"))]
    pub(crate) fn scene_layout(&self) -> SceneWidgetsLayout {
        SceneWidgetsLayout {
            plot: self.layout.plot,
            slots: self
                .layout
                .slots
                .iter()
                .map(|slot| SceneWidgetSlot {
                    key: slot.mark_id.clone(),
                    revision: slot.revision,
                    placement: slot.placement,
                    target: self.target(&slot.mark_id),
                })
                .collect(),
        }
    }
    fn target(&self, key: &str) -> SceneWidgetTarget {
        self.source_targets
            .get(key)
            .cloned()
            .unwrap_or_else(|| SceneWidgetTarget::Mark(key.to_owned()))
    }
    #[cfg(all(feature = "web", target_arch = "wasm32"))]
    pub(crate) fn replacement_keys(&self) -> &BTreeSet<String> {
        &self.source_replacements
    }
    pub(crate) fn project_scene(&self, scene: &Scene) -> Scene {
        scene
            .without_mark_ink(&self.replacement_set)
            .without_label_ink(&self.replaced_label_indices())
    }
    pub(super) fn replaced_label_indices(&self) -> BTreeSet<usize> {
        self.source_replacements
            .iter()
            .filter_map(|key| match self.source_targets.get(key) {
                Some(SceneWidgetTarget::Label(index)) => Some(*index),
                _ => None,
            })
            .collect()
    }
    // Presentation priorities are authored using original Scene label indices.
    pub(super) fn paint_label_index(&self, source_index: usize) -> Option<usize> {
        let removed = self.replaced_label_indices();
        (!removed.contains(&source_index))
            .then(|| source_index - removed.range(..source_index).count())
    }
    pub(crate) fn replace_ink(&self) -> &BTreeSet<String> {
        &self.replacement_set
    }
    #[cfg(any(test, all(feature = "web", target_arch = "wasm32")))]
    pub(crate) fn in_scene<T: Clone + PartialEq + 'static>(
        value: PlotWidgetsComposition<T>,
    ) -> Self {
        let mut packet = Self::composition(value);
        packet.source_ordered = true;
        packet
    }
    #[cfg(any(test, all(feature = "web", target_arch = "wasm32")))]
    pub(crate) fn is_source_ordered(&self) -> bool {
        self.source_ordered
    }
    pub(crate) fn is_composition(&self) -> bool {
        self.composition_mode
    }
    pub(crate) fn data_type(&self) -> Option<TypeId> {
        self.data_type
    }
    pub(crate) fn validate(&self) -> Result<()> {
        ensure(
            self.layout.slots.len() <= MAX_PLOT_WIDGET_SLOTS,
            "plot Widget slot budget exceeded",
        )?;
        if let Some(expected) = self.data_type {
            let keys: BTreeSet<_> = self
                .layout
                .slots
                .iter()
                .map(|slot| slot.mark_id.as_str())
                .collect();
            ensure(
                keys.len() == self.layout.slots.len(),
                "duplicate plot Widget datum",
            )?;
            ensure(
                keys.len() == self.data.len()
                    && self.data.iter().all(|(key, value)| {
                        keys.contains(key.as_str())
                            && value
                                .0
                                .as_ref()
                                .is_some_and(|value| value.as_any().type_id() == expected)
                    }),
                "plot Widget metadata keys or type do not match slots",
            )?;
        } else {
            ensure(
                self.data.is_empty(),
                "plain plot Widget packet cannot contain metadata",
            )?;
        }
        ensure(
            self.composition_mode || self.replacement_set.is_empty(),
            "overlay packet cannot replace mark ink",
        )?;
        ensure(
            !self.source_ordered || self.composition_mode,
            "source ordered Widgets require strict composition",
        )?;
        ensure(
            self.source_replacements
                .iter()
                .all(|id| self.layout.slots.iter().any(|slot| &slot.mark_id == id)),
            "replacement mark requires a matching Widget slot",
        )?;
        if self.general {
            let keys: BTreeSet<_> = self
                .layout
                .slots
                .iter()
                .map(|slot| slot.mark_id.as_str())
                .collect();
            ensure(
                keys.len() == self.layout.slots.len()
                    && self.source_targets.len() == keys.len()
                    && self
                        .source_targets
                        .keys()
                        .all(|key| keys.contains(key.as_str())),
                "Scene component keys do not match targets",
            )?;
            let unique: BTreeSet<_> = self.source_targets.values().collect();
            ensure(
                unique.len() == self.source_targets.len(),
                "duplicate Scene component source target",
            )?;
        }
        Ok(())
    }
}

pub(crate) struct PlotWidgetBuildContext {
    pub(crate) base: SceneWidgetContext,
    pub(crate) payload: Bond<SlotPayload>,
    pub(crate) scope: Scope,
}
pub(crate) type PlotWidgetFactory =
    Box<dyn FnMut(PlotWidgetBuildContext) -> Result<Box<dyn Widget>>>;
#[derive(Clone)]
pub(super) struct PlotWidgetsSource {
    read: Rc<dyn Fn() -> Result<PlotWidgetsPacket>>,
    local: Rc<dyn Fn(&Scope) -> bool>,
}
impl PlotWidgetsSource {
    pub(super) fn plain(source: Vary<PlotWidgetsLayout>) -> Self {
        let check = source.clone();
        Self {
            read: Rc::new(move || {
                source
                    .try_get()
                    .map(PlotWidgetsPacket::plain)
                    .map_err(|error| ChartError(error.to_string()))
            }),
            local: Rc::new(move |scope| super::local(&check, scope)),
        }
    }
    pub(super) fn typed<T: Clone + PartialEq + 'static>(source: Vary<PlotWidgetsData<T>>) -> Self {
        let check = source.clone();
        Self {
            read: Rc::new(move || {
                source
                    .try_get()
                    .map(PlotWidgetsPacket::typed)
                    .map_err(|error| ChartError(error.to_string()))
            }),
            local: Rc::new(move |scope| super::local(&check, scope)),
        }
    }
    pub(super) fn composition<T: Clone + PartialEq + 'static>(
        source: Vary<PlotWidgetsComposition<T>>,
    ) -> Self {
        let check = source.clone();
        Self {
            read: Rc::new(move || {
                source
                    .try_get()
                    .map(PlotWidgetsPacket::composition)
                    .map_err(|error| ChartError(error.to_string()))
            }),
            local: Rc::new(move |scope| super::local(&check, scope)),
        }
    }
    pub(super) fn scene_composition<T: Clone + PartialEq + 'static>(
        source: Vary<SceneWidgetsComposition<T>>,
    ) -> Self {
        let check = source.clone();
        Self {
            read: Rc::new(move || {
                source
                    .try_get()
                    .map(PlotWidgetsPacket::scene_composition)
                    .map_err(|error| ChartError(error.to_string()))
            }),
            local: Rc::new(move |scope| super::local(&check, scope)),
        }
    }
    pub(super) fn read(&self) -> Result<PlotWidgetsPacket> {
        (self.read)()
    }
    pub(super) fn is_local(&self, scope: &Scope) -> bool {
        (self.local)(scope)
    }
}

pub(super) struct PlotWidgetsInput {
    pub source: PlotWidgetsSource,
    pub factory: PlotWidgetFactory,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PreparedSlot {
    revision: u64,
    key: String,
    source: SceneWidgetFrame,
    style_index: usize,
    rect: LayoutRect,
    clip: Option<LayoutRect>,
    // In BrowserChart, percentages resolve against the actual (possibly
    // nonuniformly resized) CSS paint box independently of canvas DPR.
    relative_to: Option<[f64; 2]>,
    mask: Option<PreparedSlotMask>,
    payload: SlotPayload,
    group_opacity: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct PreparedSlotMask {
    index: usize,
    /// Scale and translation into the clip wrapper's object bounding box.
    transform: [f64; 4],
}

impl PreparedSlot {
    pub(crate) fn is_label(&self) -> bool {
        self.source.is_label()
    }
    pub(crate) fn frame(&self, hidden: &BTreeSet<String>) -> SceneWidgetFrame {
        let mut frame = self.source.clone();
        match &mut frame {
            SceneWidgetFrame::Mark(frame) => {
                frame.rect = self.rect;
                frame.visible = self.clip.filter(|_| {
                    frame.mark.opacity * self.group_opacity > 0.
                        && !hidden.contains(&frame.mark.series)
                });
                frame.interactive = self.mask.is_none() && frame.visible == Some(self.rect);
            }
            SceneWidgetFrame::Label(frame) => {
                frame.rect = self.rect;
                frame.visible = self.clip.filter(|_| {
                    self.group_opacity > 0.
                        && frame
                            .owner
                            .as_ref()
                            .is_none_or(|owner| !hidden.contains(owner))
                });
                frame.interactive = self.mask.is_none()
                    && frame.visible == Some(self.rect)
                    && self.rect.width > 0.
                    && self.rect.height > 0.;
            }
        }
        frame
    }
    #[cfg(all(feature = "web", target_arch = "wasm32"))]
    pub(crate) fn path_mask(&self) -> Option<(usize, [f64; 4], &ClipPath)> {
        let mask = self.mask?;
        let clip = match &self.source {
            SceneWidgetFrame::Mark(frame) => frame.mark.clip_path.as_deref(),
            SceneWidgetFrame::Label(frame) => frame.clip.as_ref(),
        }?;
        Some((mask.index, mask.transform, clip))
    }
    #[cfg(all(feature = "web", target_arch = "wasm32"))]
    pub(crate) fn label_style(&self) -> Option<(usize, SceneLabelWidgetFrame, Option<[f64; 2]>)> {
        let SceneWidgetFrame::Label(frame) = self.frame(&BTreeSet::new()) else {
            return None;
        };
        Some((self.style_index, frame, self.relative_to))
    }
    fn clip_class(&self, owner: Option<&str>) -> String {
        let mut value = match (self.mask, owner) {
            (Some(mask), Some(owner)) => {
                format!("kartis-plot-widget-clip {owner}-slot-{}", mask.index)
            }
            _ => "kartis-plot-widget-clip".into(),
        };
        if self.is_label() {
            value.push_str(" kartis-scene-label-widget-clip");
            if let Some(owner) = owner {
                value.push_str(&format!(" {owner}-label-slot-{}", self.style_index));
            }
        }
        value
    }
}

#[derive(Clone)]
pub(crate) struct PlotWidgetsState {
    pub rows: Cage<Vec<PreparedSlot>>,
    pub profile: Profile,
    factory: WidgetRenderer<PlotWidgetBuildContext>,
    mask_owner: Option<Rc<str>>,
}

/// Invoke the application factory inside the candidate revision's real child
/// scope. Typed Bonds then retire with that revision, including failed builds.
struct FactoryWidget {
    factory: WidgetRenderer<PlotWidgetBuildContext>,
    context: SceneWidgetContext,
    payload: Bond<SlotPayload>,
    child: Option<Box<dyn Widget>>,
}
impl Widget for FactoryWidget {
    fn build(&mut self, scope: &Scope) -> glory::ui::Result {
        let mut child = self
            .factory
            .call(PlotWidgetBuildContext {
                base: self.context.clone(),
                payload: self.payload,
                scope: scope.clone(),
            })
            .map_err(|error| glory::render::Error::Backend(error.0))?;
        child.build(scope)?;
        self.child = Some(child);
        Ok(())
    }
}
impl PlotWidgetsState {
    pub fn new(
        scope: &Scope,
        rows: Vec<PreparedSlot>,
        profile: Profile,
        factory: PlotWidgetFactory,
    ) -> Self {
        Self {
            rows: scope.cage(rows),
            profile,
            factory: WidgetRenderer::new(Some(factory)),
            mask_owner: None,
        }
    }

    #[cfg(all(feature = "web", target_arch = "wasm32"))]
    pub fn with_mask_owner(mut self, owner: Rc<str>) -> Self {
        self.mask_owner = Some(owner);
        self
    }

    pub fn clear(&self) {
        self.factory.clear();
    }

    pub fn show(
        self,
        scope: &Scope,
        style: impl Into<Vary<Style>>,
        hidden: Cage<BTreeSet<String>>,
        reject: impl Fn(ChartError) + Clone + 'static,
    ) -> glory::ui::Result {
        let rows = self.rows;
        let factory = self.factory;
        let mask_owner = self.mask_owner;
        glory::ui::view()
            .class("kartis-plot-widgets")
            .style(style)
            .fill(
                glory::ui::each(rows)
                    .key(|row| row.key.clone())
                    .children(move |row| {
                        let factory = factory.clone();
                        let reject = reject.clone();
                        let mask_owner = mask_owner.clone();
                        glory::ui::widget(move |scope| {
                            let id = row.key().clone();
                            let datum = row.value;
                            let frame = scope.bond(move || {
                                datum.with(|datum| hidden.with(|hidden| datum.frame(hidden)))
                            });
                            let payload =
                                scope.bond(move || datum.with(|datum| datum.payload.clone()));
                            let child_factory = factory.clone();
                            let child_rejection = reject.clone();
                            let mask_owner = mask_owner.clone();
                            glory::ui::view()
                                .class(scope.bond(move || {
                                    datum.with(|slot| slot.clip_class(mask_owner.as_deref()))
                                }))
                                .style(scope.bond(move || {
                                    frame.with(|frame| {
                                        datum.with(|slot| clip_style(frame, slot.relative_to))
                                    })
                                }))
                                .semantics(scope.bond(move || {
                                    frame.with(|frame| {
                                        Some(Box::new(Semantics {
                                            hidden: frame.visible().is_none(),
                                            inert: !frame.interactive(),
                                            ..Default::default()
                                        }))
                                    })
                                }))
                                .fill(
                                    glory::ui::view()
                                        .class("kartis-plot-widget-slot")
                                        .style(scope.bond(move || {
                                            frame.with(|frame| {
                                                datum.with(|slot| {
                                                    content_style(frame, slot.relative_to.is_some())
                                                })
                                            })
                                        }))
                                        .fill(glory::ui::widget(move |scope| {
                                            let weak = scope.downgrade();
                                            let mut attempted_revision = None;
                                            let mut mounted = None;
                                            let child_factory = child_factory.clone();
                                            let child_rejection = child_rejection.clone();
                                            let id = id.clone();
                                            scope.structural(move || {
                                                let revision = datum.with(|datum| datum.revision);
                                                if attempted_revision == Some(revision) {
                                                    return;
                                                }
                                                attempted_revision = Some(revision);
                                                if mounted.as_ref().is_some_and(|(accepted, _)| {
                                                    *accepted == revision
                                                }) {
                                                    return;
                                                }
                                                let Some(scope) = weak.upgrade() else { return };
                                                let result = glory::core::untracked(|| {
                                                    scope
                                                        .mount_with(|| FactoryWidget {
                                                            factory: child_factory.clone(),
                                                            context: SceneWidgetContext {
                                                                key: id.clone(),
                                                                revision,
                                                                frame,
                                                            },
                                                            payload,
                                                            child: None,
                                                        })
                                                        .map_err(|error| {
                                                            ChartError(error.to_string())
                                                        })
                                                });
                                                match result {
                                                    Ok(next) => {
                                                        drop(mounted.replace((revision, next)))
                                                    }
                                                    Err(error) => child_rejection(error),
                                                }
                                            });
                                            Ok(())
                                        })),
                                )
                                .show_in(scope)
                        })
                    }),
            )
            .show_in(scope)
    }
}

pub(super) fn prepare(
    scene: &Scene,
    layout: &PlotWidgetsLayout,
    profile: Profile,
    view: ZoomTransform,
) -> Result<Vec<PreparedSlot>> {
    view.validate()?;
    prepare_in_space(
        scene,
        layout,
        profile,
        SlotSpace {
            plot: None,
            transform: view,
            responsive: false,
            require_current_data: true,
        },
        |mark| Ok(crate::crosshair::mark_center(mark)),
        |_| None,
    )
}

pub(crate) fn prepare_packet(
    scene: &Scene,
    packet: &PlotWidgetsPacket,
    profile: Profile,
    view: ZoomTransform,
    binding: Option<&crate::InteractionBinding>,
) -> Result<Vec<PreparedSlot>> {
    packet.validate()?;
    if packet.data_type().is_none() && binding.is_none() {
        return prepare(scene, &packet.layout, profile, view);
    }
    view.validate()?;
    if let Some(binding) = binding {
        binding.validate(scene)?;
    }
    prepare_in_space_packet(
        scene,
        packet,
        profile,
        SlotSpace {
            plot: None,
            transform: view,
            responsive: false,
            require_current_data: true,
        },
        |mark| match binding {
            Some(binding) => binding.anchor_for(mark),
            None => Ok(crate::crosshair::mark_center(mark)),
        },
        |_| None,
    )
}

pub(crate) struct SlotSpace {
    pub plot: Option<ClipRect>,
    pub transform: ZoomTransform,
    pub responsive: bool,
    pub require_current_data: bool,
}

pub(crate) fn prepare_in_space(
    scene: &Scene,
    layout: &PlotWidgetsLayout,
    profile: Profile,
    space: SlotSpace,
    anchor: impl Fn(&Mark) -> Result<Option<[f64; 2]>>,
    value: impl Fn(&Mark) -> Option<f64>,
) -> Result<Vec<PreparedSlot>> {
    prepare_in_space_packet(
        scene,
        &PlotWidgetsPacket::plain(layout.clone()),
        profile,
        space,
        anchor,
        value,
    )
}

pub(crate) fn prepare_in_space_packet(
    scene: &Scene,
    packet: &PlotWidgetsPacket,
    profile: Profile,
    space: SlotSpace,
    anchor: impl Fn(&Mark) -> Result<Option<[f64; 2]>>,
    value: impl Fn(&Mark) -> Option<f64>,
) -> Result<Vec<PreparedSlot>> {
    packet.validate()?;
    let layout = &packet.layout;
    ensure(
        layout.slots.len() <= MAX_PLOT_WIDGET_SLOTS,
        "plot Widget slot budget exceeded",
    )?;
    crate::surface::validate_plot(layout.plot, scene)?;
    let relative_to = space.responsive.then_some([scene.width, scene.height]);
    profile_style(profile, &overlay_style(scene, space.responsive))?;
    let screen = LayoutRect {
        x: 0.,
        y: 0.,
        width: scene.width,
        height: scene.height,
    };
    let plot = space.plot.map(layout_rect).unwrap_or(screen);
    let view = layout
        .plot
        .map_or(Some(plot), |clip| intersect(plot, layout_rect(clip)));
    let marks: BTreeMap<_, _> = scene
        .marks
        .iter()
        .map(|mark| (mark.id.as_str(), mark))
        .collect();
    let group_opacities = scene.mark_group_opacities();
    let mut ids = BTreeSet::new();
    let mut rows = Vec::with_capacity(layout.slots.len());
    for (index, slot) in layout.slots.iter().enumerate() {
        ensure(
            !slot.mark_id.is_empty() && slot.mark_id.len() <= 1024,
            "invalid plot Widget datum ID",
        )?;
        ensure(
            ids.insert(slot.mark_id.as_str()),
            "duplicate plot Widget datum",
        )?;
        let target = packet.target(&slot.mark_id);
        let (source, rect, clip, mask, group_opacity) = match target {
            SceneWidgetTarget::Mark(id) => {
                let Some(mark) = marks
                    .get(id.as_str())
                    .filter(|mark| packet.data_type.is_some() || !mark.label.is_empty())
                else {
                    ensure(
                        !space.require_current_data,
                        "plot Widget slot requires a current mark eligible for its packet type",
                    )?;
                    continue;
                };
                let rect = match slot.placement {
                    PlotWidgetPlacement::Rect(rect) => rect,
                    PlotWidgetPlacement::AtDatum { offset, measured } => {
                        measured.validate()?;
                        let [x,y]=anchor(mark)?.ok_or_else(||ChartError("a Path datum without an interaction anchor requires an explicit plot Widget rectangle".into()))?;
                        LayoutRect {
                            x: x + offset[0],
                            y: y + offset[1],
                            width: measured.width,
                            height: measured.height,
                        }
                    }
                };
                rect.validate()?;
                ensure(
                    rect.width > 0.
                        && rect.height > 0.
                        && rect.x + rect.width > rect.x
                        && rect.y + rect.height > rect.y,
                    "plot Widget slot requires positive representable measured dimensions",
                )?;
                let rect = project_rect(rect, space.transform)?;
                let clip = view.and_then(|view| intersect(rect, view));
                let clip = if let Some(mark_clip) = mark.clip {
                    let mark_clip = project_rect(layout_rect(mark_clip), space.transform)?;
                    clip.and_then(|visible| intersect(visible, mark_clip))
                } else {
                    clip
                };
                let (clip, mask) =
                    if let (Some(clip), Some(mask)) = (clip, mark.clip_path.as_deref()) {
                        path_mask(clip, mask, space.transform, index)?
                    } else {
                        (clip, None)
                    };
                let mut mark = (*mark).clone();
                if let Some(value) = value(&mark) {
                    mark.value = value;
                }
                let opacity = group_opacities.get(&id).copied().unwrap_or(1.);
                (
                    SceneWidgetFrame::Mark(PlotWidgetFrame {
                        mark,
                        rect,
                        visible: clip,
                        interactive: false,
                    }),
                    rect,
                    clip,
                    mask,
                    opacity,
                )
            }
            SceneWidgetTarget::Label(label_index) => {
                let label = scene
                    .labels
                    .get(label_index)
                    .ok_or_else(|| ChartError("Scene component label target is absent".into()))?;
                let (rect, transform) = label_placement(label, slot.placement, space.transform)?;
                ensure(
                    space.responsive || transform.is_none(),
                    "transformed Label component requires the browser affine host",
                )?;
                let clip = view.and_then(|view| intersect(rect, view));
                let source_clip = scene.label_clips.get(&label_index).cloned();
                let (clip, mask) = if let (Some(clip), Some(mask)) = (clip, source_clip.as_ref()) {
                    path_mask(clip, mask, space.transform, index)?
                } else {
                    (clip, None)
                };
                let frame = SceneLabelWidgetFrame {
                    label_index,
                    label: label.clone(),
                    owner: scene.label_series.get(&label_index).cloned(),
                    font: scene.resolved_label_font(label_index),
                    whitespace: scene.typography.label_whitespace(label_index),
                    clip: source_clip,
                    rect,
                    visible: clip,
                    interactive: false,
                    transform,
                };
                (SceneWidgetFrame::Label(frame), rect, clip, mask, 1.)
            }
        };
        ensure(
            space.responsive || mask.is_none(),
            "partial plot Widget path masks require the browser mask host",
        )?;
        let row = PreparedSlot {
            key: slot.mark_id.clone(),
            revision: slot.revision,
            source,
            style_index: index,
            rect,
            clip,
            relative_to,
            mask,
            payload: packet.data.get(&slot.mark_id).cloned().unwrap_or_default(),
            group_opacity,
        };
        let frame = row.frame(&BTreeSet::new());
        profile_style(profile, &clip_style(&frame, relative_to))?;
        profile_style(profile, &content_style(&frame, space.responsive))?;
        rows.push(row);
    }
    // Hidden/offscreen transitions use the same finite style properties.
    let hidden = LayoutRect::default();
    profile_style(profile, &positioned(hidden, None).display(Display::None))?;
    Ok(rows)
}

fn label_placement(
    label: &crate::Label,
    placement: PlotWidgetPlacement,
    view: ZoomTransform,
) -> Result<(LayoutRect, Option<SceneWidgetTransform>)> {
    if let PlotWidgetPlacement::Rect(rect) = placement {
        rect.validate()?;
        return Ok((project_rect(rect, view)?, None));
    }
    let PlotWidgetPlacement::AtDatum { offset, measured } = placement else {
        unreachable!()
    };
    measured.validate()?;
    ensure(
        offset
            .iter()
            .all(|v| v.is_finite() && v.abs() <= 1_000_000.),
        "invalid Label component offset",
    )?;
    let radians = label.rotation.to_radians();
    let (sin, cos) = radians.sin_cos();
    let origin = [
        label.x + cos * offset[0] - sin * offset[1],
        label.y + sin * offset[0] + cos * offset[1],
    ];
    let (x, y) = view.project((origin[0], origin[1]))?;
    let matrix = [
        view.scale * cos,
        view.scale * sin,
        -view.scale * sin,
        view.scale * cos,
        x,
        y,
    ];
    ensure(
        matrix.iter().all(|v| v.is_finite() && v.abs() <= 1e9),
        "invalid Label component affine transform",
    )?;
    let corners = [
        [0., 0.],
        [measured.width, 0.],
        [0., measured.height],
        [measured.width, measured.height],
    ];
    let points = corners.map(|p| {
        [
            matrix[0] * p[0] + matrix[2] * p[1] + x,
            matrix[1] * p[0] + matrix[3] * p[1] + y,
        ]
    });
    let minx = points.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min);
    let maxx = points
        .iter()
        .map(|p| p[0])
        .fold(f64::NEG_INFINITY, f64::max);
    let miny = points.iter().map(|p| p[1]).fold(f64::INFINITY, f64::min);
    let maxy = points
        .iter()
        .map(|p| p[1])
        .fold(f64::NEG_INFINITY, f64::max);
    let rect = LayoutRect {
        x: minx,
        y: miny,
        width: maxx - minx,
        height: maxy - miny,
    };
    rect.validate()?;
    let transform = (label.rotation != 0. || view.scale != 1.)
        .then_some(SceneWidgetTransform { matrix, measured });
    Ok((rect, transform))
}

fn path_mask(
    clip: LayoutRect,
    mask: &ClipPath,
    view: ZoomTransform,
    index: usize,
) -> Result<(Option<LayoutRect>, Option<PreparedSlotMask>)> {
    if let Geometry::Rect {
        x,
        y,
        width,
        height,
    } = mask.geometry
    {
        let bounds = project_rect(
            LayoutRect {
                x,
                y,
                width,
                height,
            },
            view,
        )?;
        return Ok((intersect(clip, bounds), None));
    }
    let (x, y) = view.unproject((clip.x, clip.y))?;
    let world = LayoutRect {
        x,
        y,
        width: clip.width / view.scale,
        height: clip.height / view.scale,
    };
    match mask.rect_coverage(world)? {
        RectCoverage::Outside => Ok((None, None)),
        RectCoverage::Full => Ok((Some(clip), None)),
        RectCoverage::Partial => {
            let transform = [
                view.scale / clip.width,
                view.scale / clip.height,
                (view.translate_x - clip.x) / clip.width,
                (view.translate_y - clip.y) / clip.height,
            ];
            ensure(
                transform
                    .iter()
                    .all(|value| value.is_finite() && value.abs() <= 1e9),
                "plot Widget path mask normalization exceeds coordinate budget",
            )?;
            Ok((Some(clip), Some(PreparedSlotMask { index, transform })))
        }
    }
}

fn project_rect(rect: LayoutRect, transform: ZoomTransform) -> Result<LayoutRect> {
    let (x, y) = transform.project((rect.x, rect.y))?;
    let width = rect.width * transform.scale;
    let height = rect.height * transform.scale;
    ensure(
        width.is_finite()
            && height.is_finite()
            && (x + width).is_finite()
            && (y + height).is_finite(),
        "invalid projected plot Widget rectangle",
    )?;
    Ok(LayoutRect {
        x,
        y,
        width,
        height,
    })
}

fn profile_style(profile: Profile, style: &Style) -> Result<()> {
    style.validate().map_err(|error| ChartError(error.into()))?;
    profile.inline.check(style).map_err(|error| {
        ChartError(format!(
            "plot Widget layout is unsupported on {} ({error:?})",
            profile.name
        ))
    })
}

fn layout_rect(rect: ClipRect) -> LayoutRect {
    LayoutRect {
        x: rect.x,
        y: rect.y,
        width: rect.width,
        height: rect.height,
    }
}

fn intersect(a: LayoutRect, b: LayoutRect) -> Option<LayoutRect> {
    if b.contains(a) {
        return Some(a);
    }
    let x = a.x.max(b.x);
    let y = a.y.max(b.y);
    let right = (a.x + a.width).min(b.x + b.width);
    let bottom = (a.y + a.height).min(b.y + b.height);
    (right > x && bottom > y).then_some(LayoutRect {
        x,
        y,
        width: right - x,
        height: bottom - y,
    })
}

pub(crate) fn overlay_style(scene: &Scene, responsive: bool) -> Style {
    let [width, height] = if responsive {
        [Length::Percent(100.), Length::Percent(100.)]
    } else {
        [Length::Px(scene.width), Length::Px(scene.height)]
    };
    Style::new()
        .position(Position::Absolute)
        .left(Length::Px(0.))
        .top(Length::Px(0.))
        .width(width)
        .height(height)
        .pointer_events(PointerEvents::None)
}

fn positioned(rect: LayoutRect, relative_to: Option<[f64; 2]>) -> Style {
    let [left, top, width, height] = if let Some([width, height]) = relative_to {
        [
            Length::Percent(rect.x / width * 100.),
            Length::Percent(rect.y / height * 100.),
            Length::Percent(rect.width / width * 100.),
            Length::Percent(rect.height / height * 100.),
        ]
    } else {
        [
            Length::Px(rect.x),
            Length::Px(rect.y),
            Length::Px(rect.width),
            Length::Px(rect.height),
        ]
    };
    Style::new()
        .position(Position::Absolute)
        .left(left)
        .top(top)
        .width(width)
        .height(height)
}

fn clip_style(frame: &SceneWidgetFrame, relative_to: Option<[f64; 2]>) -> Style {
    positioned(frame.visible().unwrap_or_default(), relative_to)
        .display(if frame.visible().is_some() {
            Display::Block
        } else {
            Display::None
        })
        .overflow_x(Overflow::Hidden)
        .overflow_y(Overflow::Hidden)
        .pointer_events(PointerEvents::None)
}

fn content_style(frame: &SceneWidgetFrame, responsive: bool) -> Style {
    let rect = frame.rect();
    let visible = frame.visible().unwrap_or(rect);
    let pointer = if !frame.is_label() && frame.interactive() {
        PointerEvents::Auto
    } else {
        PointerEvents::None
    };
    if let Some(transform) = frame.transform() {
        return Style::new()
            .position(Position::Absolute)
            .left(Length::Px(0.))
            .top(Length::Px(0.))
            .width(Length::Px(transform.measured.width))
            .height(Length::Px(transform.measured.height))
            .overflow_x(Overflow::Hidden)
            .overflow_y(Overflow::Hidden)
            .pointer_events(pointer);
    }
    positioned(
        LayoutRect {
            x: rect.x - visible.x,
            y: rect.y - visible.y,
            ..rect
        },
        (responsive && visible.width > 0. && visible.height > 0.)
            .then_some([visible.width, visible.height]),
    )
    .overflow_x(Overflow::Hidden)
    .overflow_y(Overflow::Hidden)
    .pointer_events(pointer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ChartConfig, Color, Geometry};

    #[test]
    fn path_mask_classifies_holes_and_conservatively_admits_interaction() {
        let rectangle = |x, y, width, height| LayoutRect {
            x,
            y,
            width,
            height,
        };
        let path = |rect: LayoutRect| {
            Geometry::Rect {
                x: rect.x,
                y: rect.y,
                width: rect.width,
                height: rect.height,
            }
            .commands()
        };
        let mut commands = path(rectangle(10., 10., 90., 90.));
        commands.extend(path(rectangle(40., 40., 20., 20.)));
        let mask = ClipPath {
            geometry: Geometry::Path(commands),
            rule: crate::ClipRule::EvenOdd,
        };
        let view = ZoomTransform::new(2., -10., 5.).unwrap();
        let project = |r| project_rect(r, view).unwrap();
        let outside = project(rectangle(46., 44., 8., 10.));
        assert_eq!(path_mask(outside, &mask, view, 0).unwrap(), (None, None));
        let whole = project(rectangle(25., 25., 12., 12.));
        assert_eq!(
            path_mask(whole, &mask, view, 0).unwrap(),
            (Some(whole), None)
        );
        let partial = project(rectangle(33., 45., 20., 10.));
        let (clip, normalized) = path_mask(partial, &mask, view, 2).unwrap();
        assert_eq!(clip, Some(partial));
        assert_eq!(normalized.unwrap().transform, [0.05, 0.1, -1.65, -4.5]);
        assert_eq!(normalized.unwrap().index, 2);

        let circle = ClipPath::circle(20., 20., 10.);
        let disjoint = project(rectangle(28., 28., 2., 2.));
        assert_eq!(path_mask(disjoint, &circle, view, 0).unwrap(), (None, None));
        let crossing = project(rectangle(24., 24., 5., 5.));
        assert!(path_mask(crossing, &circle, view, 0).unwrap().1.is_some());
    }

    #[test]
    fn responsive_slots_project_the_mark_clip_and_use_a_fixed_plot_aperture() {
        let mut scene = ChartConfig {
            width: 320.,
            height: 160.,
            ..Default::default()
        }
        .scene()
        .unwrap();
        let mut mark = Mark::new(
            "alpha",
            Geometry::Circle {
                x: 80.,
                y: 60.,
                radius: 10.,
            },
            Color(30, 70, 90, 255),
        );
        mark.clip = Some(ClipRect {
            x: 72.,
            y: 53.,
            width: 20.,
            height: 14.,
        });
        scene.marks.push(mark);
        let layout = PlotWidgetsLayout {
            plot: None,
            slots: vec![PlotWidgetSlot {
                mark_id: "alpha".into(),
                revision: 0,
                placement: PlotWidgetPlacement::AtDatum {
                    offset: [-10., -5.],
                    measured: TextSize {
                        width: 20.,
                        height: 10.,
                    },
                },
            }],
        };
        let rows = prepare_in_space(
            &scene,
            &layout,
            Profile::HTML,
            SlotSpace {
                plot: Some(ClipRect {
                    x: 120.,
                    y: 70.,
                    width: 90.,
                    height: 75.,
                }),
                transform: ZoomTransform::new(2., -30., -20.).unwrap(),
                responsive: true,
                require_current_data: true,
            },
            |mark| Ok(crate::crosshair::mark_center(mark)),
            |_| Some(7.),
        )
        .unwrap();
        let row = &rows[0];
        let generic_frame = row.frame(&BTreeSet::new());
        let frame = generic_frame.mark_frame().unwrap();
        assert_eq!(frame.mark.value, 7.);
        assert_eq!(
            frame.rect,
            LayoutRect {
                x: 110.,
                y: 90.,
                width: 40.,
                height: 20.
            }
        );
        assert_eq!(
            frame.visible,
            Some(LayoutRect {
                x: 120.,
                y: 90.,
                width: 30.,
                height: 20.
            })
        );
        assert!(!frame.interactive);
        let clip = clip_style(&generic_frame, row.relative_to);
        assert_eq!(clip.left, Some(Length::Percent(37.5)));
        assert_eq!(clip.width, Some(Length::Percent(9.375)));
        assert_eq!(clip.overflow_x, Some(Overflow::Hidden));
        let content = content_style(&generic_frame, true);
        assert_eq!(content.left, Some(Length::Percent(-10. / 30. * 100.)));
        assert_eq!(content.width, Some(Length::Percent(40. / 30. * 100.)));
        assert_eq!(content.pointer_events, Some(PointerEvents::None));
    }
}

#[cfg(test)]
mod scene_tests;
