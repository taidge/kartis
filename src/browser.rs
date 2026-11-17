//! Opt-in browser host for application-owned SVG/Canvas surfaces plus real Glory
//! controls. Native listeners enqueue bounded inputs; RAF performs model work,
//! painting and application callbacks. Drop/stop removes every owned listener.
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::rc::{Rc, Weak};

use glory::Cage;
use glory::web::WebApp;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use web_sys::{
    Element, Event, EventTarget, HtmlCanvasElement, HtmlElement, KeyboardEvent, PointerEvent,
    ResizeObserver, WheelEvent, Window,
};

use crate::annotations::ZoomTransform;
use crate::surface::{SurfaceEvent, SurfaceInput, SurfaceModel, SurfaceOptions};
use crate::*;
mod blend_motion;
mod blends;
mod editable_brush;
mod funnel;
mod funnel_tooltip;
mod waffle;
pub use waffle::{
    BrowserWaffle, BrowserWaffleBuildContext, BrowserWaffleData, BrowserWaffleDataBuildContext,
    BrowserWaffleDataHandle, BrowserWaffleDataMouseEvent, BrowserWaffleDataMouseHandlers,
    BrowserWaffleDataSnapshot, BrowserWaffleDataSvgHit, BrowserWaffleDataSvgLayerContext,
    BrowserWaffleDataSvgLayerFrame, BrowserWaffleDataSvgProgram,
    BrowserWaffleDataTooltipComponentContext, BrowserWaffleHandle, BrowserWaffleMouseEvent,
    BrowserWaffleMouseHandlers, BrowserWaffleMouseInput, BrowserWaffleMouseKind,
    BrowserWaffleMouseOccurrence, BrowserWaffleMousePointer, BrowserWaffleSnapshot,
    BrowserWaffleSvgHit, BrowserWaffleSvgLayerContext, BrowserWaffleSvgLayerFrame,
    BrowserWaffleSvgProgram, BrowserWaffleTooltipAnchor, BrowserWaffleTooltipComponentContext,
    BrowserWaffleTooltipOptions, WaffleDataMouseHandler, WaffleDataTooltipFrame,
    WaffleMouseHandler, WaffleTooltipFrame,
};
mod geo_coordinates;
pub use editable_brush::BrowserBrushSnapshot;
pub use funnel::{
    BrowserFunnel, BrowserFunnelBuildContext, BrowserFunnelCurrentPart, BrowserFunnelData,
    BrowserFunnelDataBuildContext, BrowserFunnelDataHandle, BrowserFunnelDataMotionMouseHandlers,
    BrowserFunnelDataMouseEvent, BrowserFunnelDataMouseHandlers, BrowserFunnelDataSnapshot,
    BrowserFunnelDataSvgHit, BrowserFunnelDataSvgLayerContext, BrowserFunnelDataSvgLayerFrame,
    BrowserFunnelDataSvgProgram, BrowserFunnelDataTooltipComponentContext, BrowserFunnelHandle,
    BrowserFunnelMotionClock, BrowserFunnelMotionFrame, BrowserFunnelMotionMouseHandlers,
    BrowserFunnelMotionSnapshot, BrowserFunnelMouseActivity, BrowserFunnelMouseEvent,
    BrowserFunnelMouseHandlers, BrowserFunnelMouseInput, BrowserFunnelMouseKind,
    BrowserFunnelMouseOccurrence, BrowserFunnelMousePointer, BrowserFunnelSnapshot,
    BrowserFunnelSvgHit, BrowserFunnelSvgLayerContext, BrowserFunnelSvgLayerFrame,
    BrowserFunnelSvgProgram, FunnelDataMotionMouseHandler, FunnelDataMouseHandler,
    FunnelDataTooltipFrame, FunnelMotionMouseHandler, FunnelMouseHandler,
};
pub use funnel_tooltip::{
    BrowserFunnelTooltipAnchor, BrowserFunnelTooltipComponentContext, BrowserFunnelTooltipOptions,
    FunnelTooltipFrame,
};

use crate::brush::{BrushReconfigure, EditableBrushOptions};
mod native_press;
mod network;
mod ordered_widgets;
mod pattern_svg;
mod plot_widgets;
pub use network::{
    BrowserNetwork, BrowserNetworkHandle, BrowserNetworkNodeWidgetOptions, BrowserNetworkOptions,
    BrowserNetworkSnapshot, BrowserNetworkWidgetOptions,
};
mod svg;
mod text_metrics;
pub use text_metrics::BrowserSemanticTextMeasurer;
mod tree;
mod tree_animated_widgets;
pub use tree_animated_widgets::{
    BrowserTreeAnimatedWidgetContext, BrowserTreeAnimatedWidgetData,
    BrowserTreeAnimatedWidgetFrame, TreeMotionWidgetProps,
};
mod tree_canvas_frame;
mod tree_svg_frame;
mod tree_svg_layers;
mod tree_tooltip_host;
mod tree_tooltip_panel;
pub use tree::{
    BrowserTree, BrowserTreeBuildContext, BrowserTreeCanvasProgram, BrowserTreeDoubleClickBehavior,
    BrowserTreeHandle, BrowserTreeLinkMouseEvent, BrowserTreeLinkMouseHandlers,
    BrowserTreeMotionClock, BrowserTreeMotionFrame, BrowserTreeMouseBinding,
    BrowserTreeMouseHandlers, BrowserTreeMouseInput, BrowserTreeMouseKind,
    BrowserTreeMouseOccurrence, BrowserTreeMouseOrigin, BrowserTreeMousePointer,
    BrowserTreeNodeMouseActivity, BrowserTreeNodeMouseEvent, BrowserTreeNodeMouseHandlers,
    BrowserTreeOptions, BrowserTreePicking, BrowserTreeSnapshot, BrowserTreeSvgBuildContext,
    BrowserTreeSvgCurrentNode, BrowserTreeWidgetOptions, TreeLinkMouseHandler,
    TreeNodeMouseHandler,
};
pub use tree_svg_layers::{
    BrowserTreeSvgAnimatedLayerContext, BrowserTreeSvgLayerContext, BrowserTreeSvgLayerFrame,
    BrowserTreeSvgProgram,
};
pub use tree_tooltip_host::{
    BrowserTreeLinkTooltipComponentContext, BrowserTreeLinkTooltipOptions,
    BrowserTreeTooltipAnchor, BrowserTreeTooltipComponentContext, BrowserTreeTooltipOptions,
    BrowserTreeTooltipPosition, TreeLinkTooltipFrame, TreeTooltipFrame,
};
mod viewport;
pub use viewport::BrowserViewportLayout;
use viewport::ViewportBinding;

const MAX_PENDING: usize = 256;
// web-sys' legacy MouseEvent accessors return i32. Browsers deliver fractional
// CSS pixels (especially under scaling/subpixel layout); truncating them loses
// thin marks before coordinate conversion. Read the actual Number as f64.
#[wasm_bindgen::prelude::wasm_bindgen]
extern "C" {
    #[wasm_bindgen(extends = web_sys::MouseEvent)]
    type PreciseMouseEvent;
    #[wasm_bindgen(method, getter, js_name = clientX)]
    fn precise_x(this: &PreciseMouseEvent) -> f64;
    #[wasm_bindgen(method, getter, js_name = clientY)]
    fn precise_y(this: &PreciseMouseEvent) -> f64;
}
fn client_coordinates(event: &web_sys::MouseEvent) -> (f64, f64) {
    let event: &PreciseMouseEvent = event.unchecked_ref();
    (event.precise_x(), event.precise_y())
}
type EventCallback = Box<dyn FnMut(BrowserEvent)>;
type LayoutCallback = Box<dyn FnMut(f64, f64) -> Result<SceneInteraction>>;
type RafCallback = Closure<dyn FnMut(f64)>;
type ResizeBinding = (ResizeObserver, Closure<dyn FnMut()>);
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BrowserRenderer {
    Svg,
    #[default]
    Canvas,
}
/// Whether BrowserChart adds its automatic series dimming and active stroke.
/// Controlled keeps authored Scene paint intact; input, focus, selection,
/// tooltip and controls still work and can drive a chart-specific relation set.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BrowserHighlight {
    #[default]
    Series,
    Controlled,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MotionPreference {
    #[default]
    System,
    Reduced,
    Animate,
}
/// Property springs preserve each coordinate's velocity across retargets.
/// Scene mode retains the earlier shared-progress interpolation contract.
#[derive(Clone, Debug, PartialEq)]
pub enum BrowserAnimation {
    Scene,
    Properties(motion::PropertyMotionOptions),
    /// Initial input contour keys, preserving the existing Properties tuple API.
    PropertiesWithContours {
        options: motion::PropertyMotionOptions,
        bindings: motion::ContourBindings,
    },
}
impl Default for BrowserAnimation {
    fn default() -> Self {
        Self::Properties(motion::PropertyMotionOptions::default())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrowserAnimationKind {
    Scene,
    Properties,
}
#[derive(Clone)]
enum RunningMotion {
    Scene(Box<motion::SpringMotion>),
    Properties(Box<motion::PropertyMotion>),
}
impl RunningMotion {
    fn new(
        scene: Scene,
        mode: &BrowserAnimation,
        bindings: motion::ContourBindings,
        blends: MarkBlends,
    ) -> Result<Self> {
        Ok(match mode {
            BrowserAnimation::Scene => {
                Self::Scene(Box::new(motion::SpringMotion::new_with_blends(
                    scene,
                    motion::SpringConfig::default(),
                    blends,
                )?))
            }
            BrowserAnimation::Properties(options)
            | BrowserAnimation::PropertiesWithContours { options, .. } => Self::Properties(
                Box::new(motion::PropertyMotion::new_with_contours_and_blends(
                    scene,
                    options.clone(),
                    bindings,
                    blends,
                )?),
            ),
        })
    }
    fn kind(&self) -> BrowserAnimationKind {
        match self {
            Self::Scene(_) => BrowserAnimationKind::Scene,
            Self::Properties(_) => BrowserAnimationKind::Properties,
        }
    }
    fn scene(&self) -> &Scene {
        match self {
            Self::Scene(m) => m.scene(),
            Self::Properties(m) => m.scene(),
        }
    }
    fn retarget_with_blends(
        &mut self,
        scene: Scene,
        bindings: Option<motion::ContourBindings>,
        blends: MarkBlends,
    ) -> Result<()> {
        match self {
            Self::Scene(_) if bindings.is_some() => Err(ChartError(
                "contour bindings require Properties animation".into(),
            )),
            Self::Scene(m) => m.retarget_with_blends(scene, blends),
            Self::Properties(m) => {
                m.retarget_with_contours_and_blends(scene, bindings.unwrap_or_default(), blends)
            }
        }
    }
    fn blends(&self) -> &MarkBlends {
        match self {
            Self::Scene(m) => m.blends(),
            Self::Properties(m) => m.blends(),
        }
    }
    fn blend_admission_frames(&self) -> Result<Option<[Scene; 2]>> {
        match self {
            Self::Scene(m) => m.blend_admission_frames(),
            Self::Properties(m) => m.blend_admission_frames(),
        }
    }
    fn paint_bindings(&self) -> motion::ContourBindings {
        match self {
            Self::Properties(m) if m.is_settled() => m.contour_bindings().clone(),
            _ => motion::ContourBindings::new(),
        }
    }
    fn tick(&mut self, seconds: f64) -> Result<()> {
        match self {
            Self::Scene(m) => m.tick(seconds).map(|_| ()),
            Self::Properties(m) => m.tick(seconds).map(|_| ()),
        }
    }
    fn is_settled(&self) -> bool {
        match self {
            Self::Scene(m) => m.is_settled(),
            Self::Properties(m) => m.is_settled(),
        }
    }
    fn finish(&mut self) {
        match self {
            Self::Scene(m) => m.finish(),
            Self::Properties(m) => m.finish(),
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct BrowserOptions {
    /// Opt-in persistent, editable viewport selection shared by SVG and Canvas.
    pub editable_brush: Option<EditableBrushOptions>,
    /// Opt-in owned crosshair, driven by the accepted point/slice state.
    pub crosshair: Option<CrosshairOptions>,
    pub interaction_data: Option<InteractionBinding>,
    pub legend_options: LegendOptions,
    pub tooltip_options: TooltipOptions,
    /// Surface-relative CSS pixel placement. The configured viewport is
    /// intersected with the current surface; forbidden rectangles share that
    /// coordinate system. None uses the whole surface and eight-way avoidance.
    pub tooltip_layout: Option<TooltipLayoutOptions>,
    pub renderer: BrowserRenderer,
    pub interaction: SurfaceOptions,
    pub highlight: BrowserHighlight,
    pub legend: bool,
    /// Enable typed legend action events. Default categories are static.
    /// Subscribe after mount with BrowserChart::on_event; subscription alone
    /// does not enable actions or alter the mounted control kind.
    pub legend_actions: bool,
    pub data_controls: bool,
    pub tooltip: bool,
    pub height: Option<f64>,
    pub motion: MotionPreference,
    pub animation: BrowserAnimation,
}
impl Default for BrowserOptions {
    fn default() -> Self {
        Self {
            editable_brush: None,
            crosshair: None,
            interaction_data: None,
            legend_options: LegendOptions::default(),
            tooltip_options: TooltipOptions {
                padding: 8.,
                background: Some(Color(32, 51, 78, 255)),
                text: PresentationText {
                    color: Some(Color(255, 255, 255, 255)),
                    ..Default::default()
                },
                ..Default::default()
            },
            tooltip_layout: None,
            renderer: BrowserRenderer::Canvas,
            interaction: SurfaceOptions::default(),
            highlight: BrowserHighlight::Series,
            legend: true,
            legend_actions: false,
            data_controls: true,
            tooltip: true,
            height: None,
            motion: MotionPreference::System,
            animation: BrowserAnimation::default(),
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum BrowserEvent {
    /// An explicitly enabled legend action; category actions never hide series.
    LegendActivated(LegendActivation),
    Interaction(SurfaceEvent),
    Resized {
        width: f64,
        height: f64,
    },
    Error(ChartError),
    Focus(Option<String>),
}
#[derive(Clone, Debug, PartialEq)]
pub struct BrowserSnapshot {
    pub editable_brush: Option<BrowserBrushSnapshot>,
    pub interaction: Option<ActiveInteraction>,
    pub active: bool,
    pub renderer: BrowserRenderer,
    pub width: f64,
    pub height: f64,
    pub hovered: Vec<String>,
    pub selected: Option<String>,
    pub hidden: BTreeSet<String>,
    pub view: ZoomTransform,
    pub revision: u64,
    pub pending: usize,
    pub dropped: u64,
    pub animating: bool,
    pub animation_kind: Option<BrowserAnimationKind>,
    pub touches: usize,
    pub pinching: bool,
    pub captured_touches: usize,
    pub domains: Option<ResolvedDomains>,
}
#[derive(Clone, Copy)]
enum PointerKind {
    Down,
    Move,
    Up,
    Cancel,
}
enum Input {
    EditableBrush(editable_brush::Command),
    Pointer {
        kind: PointerKind,
        id: i32,
        touch: bool,
        client: (f64, f64),
        brush: bool,
        source_mesh: bool,
    },
    Wheel {
        client: (f64, f64),
        delta: f64,
        mode: u32,
    },
    Action(SurfaceInput),
    Hidden(BTreeSet<String>),
    LegendActivate(LegendActivation),
    LegendToggle(LegendActivation),
    Selected(Option<String>),
    Hover(Option<String>),
    Focus(Option<String>),
    Activate(String),
    // Focus dispatched synchronously by UI work in an Engine frame is not
    // another native pointer/keyboard intent. Latch that delivery boundary
    // here; draining the queue happens in a later frame.
    PlotWidgetFocus {
        restoring: bool,
    },
    PlotWidgets(crate::widget::plot_widgets::PlotWidgetsPacket),
    PlotWidgetLayout(
        Box<(
            SceneInteraction,
            crate::widget::plot_widgets::PlotWidgetsPacket,
        )>,
    ),
    PlotWidgetComposition(
        Box<(
            Option<SceneInteraction>,
            crate::widget::plot_widgets::PlotWidgetsPacket,
            Rc<crate::widget::plot_widgets::composition::CompositionStage>,
        )>,
    ),
    TreeCanvas(Box<tree_canvas_frame::Candidate>),
    TreeSvg(Rc<tree_svg_frame::Stage>),
    TreeTooltip(Box<tree_tooltip_host::Candidate>),
    Scene(Scene, bool),
    InteractionScene(Box<(Scene, InteractionBinding, bool)>),
    Layout(Box<(SceneInteraction, MarkBlends)>),
    BlendedAnimation(Box<blend_motion::Animation>),
    ContourScene(Scene, motion::ContourBindings),
    Resize,
    ReducedMotion(bool),
    ViewportRefresh,
    ViewportRequest(ViewportRequest),
}
struct Listener {
    target: EventTarget,
    name: &'static str,
    callback: Closure<dyn FnMut(Event)>,
}
impl Drop for Listener {
    fn drop(&mut self) {
        let _ = self
            .target
            .remove_event_listener_with_callback(self.name, self.callback.as_ref().unchecked_ref());
    }
}
struct State {
    model: SurfaceModel,
    web: WebApp,
    source: Cage<Scene>,
    data_labels: Cage<BTreeMap<String, String>>,
    data_pointer: Cage<bool>,
    hidden: Cage<BTreeSet<String>>,
    selected: Cage<Option<String>>,
    surface: HtmlElement,
    canvas: Option<HtmlCanvasElement>,
    svg: Option<svg::RetainedSvg>,
    tree_labels_passive: bool,
    tooltip: HtmlElement,
    brush: HtmlElement,
    editable_host: Option<editable_brush::Host>,
    editable_transport: Option<(u64, i32)>,
    options: BrowserOptions,
    revision: u64,
    last_size: (f64, f64),
    motion: Option<RunningMotion>,
    last_time: Option<f64>,
    pointer: Option<(f64, f64)>,
    focused: Option<String>,
    keyboard: bool,
    reduced: bool,
    presentation_error: Option<ChartError>,
    contour_bindings: motion::ContourBindings,
    blends: Rc<MarkBlends>,
    plot_widgets: Option<Rc<crate::widget::plot_widgets::PlotWidgetsPacket>>,
    staged_widgets: Option<Rc<crate::widget::plot_widgets::composition::CompositionStage>>,
    tree_canvas: Option<PreparedTreeCanvas>,
    tree_tooltip: Option<Rc<tree_tooltip_host::Stage>>,
    tree_svg: Option<Rc<tree_svg_frame::Stage>>,
}
#[derive(Clone)]
struct GestureState {
    editable_focus: Option<HtmlElement>,
    options: BrowserOptions,
    revision: u64,
    last_size: (f64, f64),
    presentation_error: Option<ChartError>,
    editable_transport: Option<(u64, i32)>,
    model: SurfaceModel,
    motion: Option<RunningMotion>,
    last_time: Option<f64>,
    pointer: Option<(f64, f64)>,
    keyboard: bool,
    contour_bindings: motion::ContourBindings,
    blends: Rc<MarkBlends>,
    plot_widgets: Option<Rc<crate::widget::plot_widgets::PlotWidgetsPacket>>,
    staged_widgets: Option<Rc<crate::widget::plot_widgets::composition::CompositionStage>>,
    tree_canvas: Option<PreparedTreeCanvas>,
    tree_tooltip: Option<Rc<tree_tooltip_host::Stage>>,
    tree_svg: Option<Rc<tree_svg_frame::Stage>>,
}
struct Engine {
    viewport: RefCell<Option<ViewportBinding>>,
    viewport_generation: Cell<u64>,
    viewport_configured: Cell<bool>,
    viewport_domains: Cell<Option<ResolvedDomains>>,
    viewport_controller: RefCell<Option<DomainViewport>>,
    window: Window,
    root: Element,
    outer: HtmlElement,
    alive: Cell<bool>,
    processing: Cell<bool>,
    queue: RefCell<VecDeque<Input>>,
    resize_pending: Cell<bool>,
    pending_reduced: Cell<Option<bool>>,
    ticket: Cell<Option<i32>>,
    callback: RefCell<Option<RafCallback>>,
    listeners: RefCell<Vec<Listener>>,
    observer: RefCell<Option<ResizeBinding>>,
    plot_widgets: RefCell<Option<Rc<plot_widgets::Host>>>,
    tree_canvas_repaint: RefCell<Option<tree_canvas_frame::Repaint>>,
    tree_tooltips: RefCell<Option<Rc<tree_tooltip_host::Host>>>,
    tree_svg_host: RefCell<Option<Rc<tree_svg_layers::Host>>>,
    tree_svg_sources: RefCell<Option<Rc<plot_widgets::svg_occurrences::Sources>>>,
    tree_animated_widgets: RefCell<Option<Rc<tree_animated_widgets::AnimatedWidgets>>>,
    tree_svg_publication: Cell<u64>,
    tree_svg_retarget_model: RefCell<Option<SurfaceModel>>,
    tree_svg_mesh_contacts: RefCell<BTreeSet<i32>>,
    tree_mouse_double_click: RefCell<Option<Event>>,
    state: RefCell<Option<State>>,
    event: RefCell<Option<EventCallback>>,
    event_generation: Cell<u64>,
    layout: RefCell<Option<LayoutCallback>>,
    layout_generation: Cell<u64>,
    captured: Cell<Option<i32>>,
    native_press: Cell<Option<native_press::Press>>,
    retired_native_press: Cell<Option<i32>>,
    editable_capture: Cell<Option<editable_brush::Capture>>,
    editable_generation: Cell<u64>,
    editable_space: Cell<bool>,
    touches: RefCell<BTreeSet<i32>>,
    pointer_over_datum: Cell<bool>,
    dropped: Cell<u64>,
    fault: RefCell<Option<ChartError>>,
}
#[must_use = "retain BrowserChart for its mounted lifetime; dropping it stops frames and removes the chart"]
pub struct BrowserChart {
    engine: Rc<Engine>,
}
#[derive(Clone)]
pub struct BrowserHandle {
    engine: Weak<Engine>,
}
fn host(error: impl std::fmt::Debug) -> ChartError {
    ChartError(format!("browser chart host error: {error:?}"))
}
/// Restore an existing owned control after moving its native carrier, without
/// turning the restoration into a viewport scroll or reconstructing its Scope.
fn restore_dom_focus(element: &HtmlElement) -> Result<()> {
    let document = element
        .owner_document()
        .ok_or_else(|| ChartError("browser focus document missing".into()))?;
    if document
        .active_element()
        .is_some_and(|active| active.is_same_node(Some(element.as_ref())))
        || !element.is_connected()
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
fn css(element: &HtmlElement, name: &str, value: &str) -> Result<()> {
    element.style().set_property(name, value).map_err(host)
}
fn element(document: &web_sys::Document, class: &str) -> Result<HtmlElement> {
    let e = document
        .create_element("div")
        .map_err(host)?
        .dyn_into::<HtmlElement>()
        .map_err(host)?;
    e.set_class_name(class);
    Ok(e)
}
impl BrowserChart {
    /// Claim an empty application-owned root. This mounts Glory controls into
    /// a separate child; the surface is never inserted into Glory's owned tree.
    pub fn mount(root: Element, scene: Scene, options: BrowserOptions) -> Result<Self> {
        Self::mount_with_blends(root, scene, options, MarkBlends::default())
    }
    /// Mount a chart-isolated blend profile with the scene. Resize and
    /// viewport callbacks retain these modes by stable mark ID. Explicit
    /// plain scene/layout replacement clears them.
    pub fn mount_with_blends(
        root: Element,
        scene: Scene,
        options: BrowserOptions,
        blends: MarkBlends,
    ) -> Result<Self> {
        Self::mount_inner(root, scene, options, blends, None)
    }
    fn mount_inner(
        root: Element,
        scene: Scene,
        options: BrowserOptions,
        blends: MarkBlends,
        widget_mount: Option<plot_widgets::Mount>,
    ) -> Result<Self> {
        Self::mount_core(root, scene, options, blends, widget_mount, None)
    }
    fn mount_tree_scene(
        root: Element,
        scene: Scene,
        options: BrowserOptions,
        widget_mount: Option<plot_widgets::Mount>,
    ) -> Result<Self> {
        Self::mount_base(
            root,
            scene,
            options,
            MarkBlends::default(),
            widget_mount,
            None,
            None,
            true,
        )
    }
    fn mount_core(
        root: Element,
        scene: Scene,
        options: BrowserOptions,
        blends: MarkBlends,
        widget_mount: Option<plot_widgets::Mount>,
        tree_canvas: Option<PreparedTreeCanvas>,
    ) -> Result<Self> {
        let tree_labels_passive = tree_canvas.is_some();
        Self::mount_base(
            root,
            scene,
            options,
            blends,
            widget_mount,
            tree_canvas,
            None,
            tree_labels_passive,
        )
    }
    #[allow(clippy::too_many_arguments)]
    fn mount_base(
        root: Element,
        scene: Scene,
        mut options: BrowserOptions,
        blends: MarkBlends,
        widget_mount: Option<plot_widgets::Mount>,
        tree_canvas: Option<PreparedTreeCanvas>,
        tree_svg: Option<tree_svg_frame::Mount>,
        tree_labels_passive: bool,
    ) -> Result<Self> {
        scene.validate()?;
        blends.validate(&scene)?;
        options.interaction.validate()?;
        options.legend_options.validate()?;
        options.tooltip_options.validate()?;
        if let Some(layout) = &options.tooltip_layout {
            layout.validate()?;
        }
        for item in &scene.legend {
            legend_label(item, &options.legend_options)?;
        }
        if let Some(crosshair) = &options.crosshair {
            crosshair.validate()?;
            crosshair::overlay_scene_admission(&scene, crosshair)?;
        }
        crosshair::overlay_admission(
            &scene,
            options.crosshair.is_some()
                || options
                    .interaction_data
                    .as_ref()
                    .is_some_and(|b| b.polar_spoke.is_some() && b.polar.is_some()),
        )?;
        let mut model = SurfaceModel::new_with_interaction(
            scene.clone(),
            options.interaction.clone(),
            options.interaction_data.take(),
        )?;
        if let Some(brush) = options.editable_brush {
            model.configure_editable_brush(Some(brush), BrushReconfigure::Clamp)?;
        }
        if let Some(mount) = &widget_mount {
            mount.layout.validate()?;
            model.set_widget_replacements(mount.layout.replace_ink().clone())?;
        }
        if let Some(raw) = &tree_canvas {
            tree_canvas_frame::policy(&options, widget_mount.as_ref().map(|m| &m.layout))?;
            tree_canvas_frame::validate_bitmap(&scene, raw)?;
            scene::ensure(blends.is_empty(), "raw Tree Canvas does not support blends")?;
            scene::ensure(
                model.paint_transform() == raw.options().view
                    && raw.options().pixel_ratio
                        == blends::ratio(
                            web_sys::window()
                                .ok_or_else(|| ChartError("browser window unavailable".into()))?
                                .device_pixel_ratio(),
                            &scene,
                        ),
                "raw Tree Canvas initial view or DPR differs",
            )?;
        }
        let widget_initial = widget_mount
            .as_ref()
            .map(|mount| plot_widgets::prepare(&model, &mount.layout, true))
            .transpose()?;
        let contour_bindings = match &options.animation {
            BrowserAnimation::PropertiesWithContours { bindings, .. } => bindings.clone(),
            _ => motion::ContourBindings::new(),
        };
        match &options.animation {
            BrowserAnimation::Properties(properties)
            | BrowserAnimation::PropertiesWithContours {
                options: properties,
                ..
            } => {
                properties.validate()?;
                if !properties.geometry.contour_morphs.is_empty() || !contour_bindings.is_empty() {
                    motion::PropertyMotion::new_with_contours(
                        scene.clone(),
                        properties.clone(),
                        contour_bindings.clone(),
                    )?;
                }
            }
            BrowserAnimation::Scene => {}
        }
        scene::ensure(
            options
                .height
                .is_none_or(|h| h.is_finite() && (1. ..=16384.).contains(&h)),
            "invalid browser surface height",
        )?;
        scene::ensure(
            !root.has_child_nodes()
                && !root.has_attribute("data-gly-app")
                && !root.has_attribute("data-kartis-browser"),
            "browser chart needs an empty unclaimed root",
        )?;
        let document = root
            .owner_document()
            .ok_or_else(|| ChartError("root has no document".into()))?;
        let window = document
            .default_view()
            .ok_or_else(|| ChartError("root has no window".into()))?;
        blends::admit(
            options.renderer,
            &window,
            &scene,
            &blends,
            model.paint_transform(),
        )?;
        let outer = element(&document, "kartis-browser")?;
        let surface = element(&document, "kartis-surface")?;
        surface.set_attribute("tabindex", "0").map_err(host)?;
        surface
            .set_attribute(
                "role",
                if options.editable_brush.is_some() {
                    "group"
                } else {
                    "img"
                },
            )
            .map_err(host)?;
        surface
            .set_attribute("aria-label", &scene.title)
            .map_err(host)?;
        css(&outer, "position", "relative")?;
        css(&surface, "position", "relative")?;
        css(&surface, "overflow", "hidden")?;
        css(&surface, "width", "100%")?;
        css(
            &surface,
            "touch-action",
            if options.interaction.pan || options.interaction.brush || options.interaction.zoom {
                "none"
            } else {
                "auto"
            },
        )?;
        if let Some(height) = options.height {
            css(&surface, "height", &format!("{height}px"))?;
        } else {
            css(
                &surface,
                "aspect-ratio",
                &format!("{} / {}", scene.width, scene.height),
            )?;
        }
        let ordered_widgets = widget_mount
            .as_ref()
            .is_some_and(|mount| mount.layout.is_source_ordered());
        if ordered_widgets {
            surface.set_attribute("role", "group").map_err(host)?;
        }
        let canvas = if options.renderer == BrowserRenderer::Canvas && !ordered_widgets {
            let canvas = document
                .create_element("canvas")
                .map_err(host)?
                .dyn_into::<HtmlCanvasElement>()
                .map_err(host)?;
            canvas.set_attribute("aria-hidden", "true").map_err(host)?;
            surface.append_child(&canvas).map_err(host)?;
            Some(canvas)
        } else {
            None
        };
        let controls = element(&document, "kartis-controls")?;
        let tooltip = element(&document, "kartis-surface-tooltip")?;
        tooltip.set_attribute("role", "tooltip").map_err(host)?;
        tooltip.set_attribute("aria-live", "polite").map_err(host)?;
        for (name, value) in [
            ("position", "absolute"),
            ("pointer-events", "none"),
            ("display", "none"),
            ("z-index", "2"),
            ("background", "#20334e"),
            ("color", "white"),
            ("padding", "8px 12px"),
            ("border-radius", "5px"),
            ("max-width", "min(340px,90%)"),
            ("white-space", "pre-line"),
            ("overflow-wrap", "anywhere"),
            ("box-sizing", "border-box"),
            ("overflow", "auto"),
        ] {
            css(&tooltip, name, value)?;
        }
        let t = &options.tooltip_options;
        for (name, value) in [
            ("font-size", format!("{}px", t.text.size)),
            ("font-weight", t.text.weight.to_string()),
            ("line-height", t.text.line_height.to_string()),
            ("padding", format!("{}px", t.padding)),
        ] {
            css(&tooltip, name, &value)?;
        }
        if let Some(color) = t.text.color {
            css(&tooltip, "color", &render::css_color(color))?;
        }
        if let Some(color) = t.background {
            css(&tooltip, "background", &render::css_color(color))?;
        }
        let brush = element(&document, "kartis-surface-brush")?;
        for (name, value) in [
            ("position", "absolute"),
            ("pointer-events", "none"),
            ("display", "none"),
            ("border", "1px solid #3775b9"),
            ("background", "rgba(60,130,210,.18)"),
            ("z-index", "1"),
        ] {
            css(&brush, name, value)?;
        }
        let widget_root = if widget_mount.is_some() {
            let stack = element(&document, "kartis-browser-plot")?;
            css(&stack, "position", "relative")?;
            let layer = element(&document, "kartis-browser-plot-widgets")?;
            css(&layer, "position", "absolute")?;
            css(&layer, "inset", "0")?;
            css(&layer, "pointer-events", "none")?;
            stack.append_child(&surface).map_err(host)?;
            if ordered_widgets {
                surface.append_child(&layer).map_err(host)?;
            } else {
                stack.append_child(&layer).map_err(host)?;
            }
            outer.append_child(&stack).map_err(host)?;
            Some(layer)
        } else {
            outer.append_child(&surface).map_err(host)?;
            None
        };
        outer.append_child(&controls).map_err(host)?;
        outer.append_child(&tooltip).map_err(host)?;
        outer.append_child(&brush).map_err(host)?;
        root.append_child(&outer).map_err(host)?;
        root.set_attribute("data-kartis-browser", "active")
            .map_err(host)?;
        let engine = Rc::new(Engine {
            viewport: RefCell::new(None),
            viewport_generation: Cell::new(0),
            viewport_configured: Cell::new(false),
            viewport_domains: Cell::new(None),
            viewport_controller: RefCell::new(None),
            window,
            root,
            outer,
            alive: Cell::new(true),
            processing: Cell::new(false),
            queue: RefCell::new(VecDeque::new()),
            resize_pending: Cell::new(false),
            pending_reduced: Cell::new(None),
            ticket: Cell::new(None),
            callback: RefCell::new(None),
            listeners: RefCell::new(vec![]),
            observer: RefCell::new(None),
            plot_widgets: RefCell::new(None),
            tree_canvas_repaint: RefCell::new(None),
            tree_tooltips: RefCell::new(None),
            tree_svg_host: RefCell::new(None),
            tree_svg_sources: RefCell::new(None),
            tree_animated_widgets: RefCell::new(None),
            tree_svg_publication: Cell::new(0),
            tree_svg_retarget_model: RefCell::new(None),
            tree_svg_mesh_contacts: RefCell::new(BTreeSet::new()),
            tree_mouse_double_click: RefCell::new(None),
            state: RefCell::new(None),
            event: RefCell::new(None),
            event_generation: Cell::new(0),
            layout: RefCell::new(None),
            layout_generation: Cell::new(0),
            captured: Cell::new(None),
            native_press: Cell::new(None),
            retired_native_press: Cell::new(None),
            editable_capture: Cell::new(None),
            editable_generation: Cell::new(0),
            editable_space: Cell::new(false),
            touches: RefCell::new(BTreeSet::new()),
            pointer_over_datum: Cell::new(false),
            dropped: Cell::new(0),
            fault: RefCell::new(None),
        });
        let chart = Self {
            engine: engine.clone(),
        };
        let weak = Rc::downgrade(&engine);
        *engine.callback.borrow_mut() = Some(Closure::new(move |time: f64| {
            if let Some(engine) = weak.upgrade() {
                engine.ticket.set(None);
                if let Err(error) = engine.frame(time) {
                    engine.emit(BrowserEvent::Error(error));
                }
            }
        }));
        let mut cages = None;
        let weak = Rc::downgrade(&engine);
        let initial = model.controls_scene();
        let initial_labels = model.data_control_labels();
        let initial_data_pointer = model
            .interaction_data()
            .is_none_or(|binding| binding.picking_profile == crate::PickingProfile::LegacyGeometry);
        let show_legend = options.legend;
        let legend_actions = options.legend_actions;
        let show_data = options.data_controls;
        let legend_options = options.legend_options.clone();
        let web = glory::web::launch_with(controls.clone().into(), |scope| {
            let source = scope.cage(initial);
            let data_labels = scope.cage(initial_labels);
            let data_pointer = scope.cage(initial_data_pointer);
            let hidden = scope.cage(BTreeSet::new());
            let selected = scope.cage(None);
            cages = Some((source, hidden, selected, data_labels, data_pointer));
            let h = weak.clone();
            let s = weak.clone();
            let a = weak.clone();
            let p = weak.clone();
            let f = weak.clone();
            let l = weak.clone();
            let controls = crate::chart(source)
                .data_control_labels(data_labels)
                .data_control_pointer(data_pointer)
                .plot(false)
                .legend(show_legend)
                .legend_options(legend_options)
                .data_controls(show_data)
                .tooltip(false)
                .hidden(hidden)
                .selected(selected)
                .on_legend_toggle(move |activation| {
                    if let Some(e) = h.upgrade() {
                        e.enqueue(Input::LegendToggle(activation));
                    }
                })
                .on_selection_change(move |value| {
                    if let Some(e) = s.upgrade() {
                        e.enqueue(Input::Selected(value.map(|m| m.id)));
                    }
                })
                .on_select(move |mark| {
                    if let Some(e) = a.upgrade() {
                        e.enqueue(Input::Activate(mark.id));
                    }
                })
                .on_hover(move |value| {
                    if let Some(e) = p.upgrade() {
                        e.enqueue(Input::Hover(value.map(|m| m.id)));
                    }
                })
                .on_focus(move |value| {
                    if let Some(e) = f.upgrade() {
                        e.enqueue(Input::Focus(value.map(|m| m.id)));
                    }
                });
            if legend_actions {
                controls.on_legend_activate(move |activation| {
                    if let Some(e) = l.upgrade() {
                        e.enqueue(Input::LegendActivate(activation));
                    }
                })
            } else {
                controls
            }
        })
        .map_err(host)?;
        let (source, hidden, selected, data_labels, data_pointer) =
            cages.ok_or_else(|| ChartError("Glory controls did not initialize".into()))?;
        let reduced = options.motion == MotionPreference::Reduced
            || (options.motion == MotionPreference::System
                && engine
                    .window
                    .match_media("(prefers-reduced-motion: reduce)")
                    .map_err(host)?
                    .is_some_and(|m| m.matches()));
        if let Some(raw) = &tree_canvas {
            let destination = canvas
                .as_ref()
                .ok_or_else(|| ChartError("raw Tree Canvas primary surface missing".into()))?;
            raw.blit(destination)?;
            destination
                .style()
                .set_property("width", "100%")
                .map_err(host)?;
            destination
                .style()
                .set_property("height", "100%")
                .map_err(host)?;
            destination
                .style()
                .set_property("display", "block")
                .map_err(host)?;
        }
        let widget_layout = if let (Some(root), Some(mount), Some(initial)) =
            (widget_root, widget_mount, widget_initial)
        {
            let widgets = plot_widgets::Host::launch(
                root.clone().into(),
                model.scene(),
                mount.factory,
                initial,
            )?;
            *engine.plot_widgets.borrow_mut() = Some(Rc::new(widgets));
            let weak = Rc::downgrade(&engine);
            engine.add(root.into(), "focusin", move |_| {
                if let Some(engine) = weak.upgrade() {
                    engine.enqueue(Input::PlotWidgetFocus {
                        restoring: engine.processing.get(),
                    });
                }
            })?;
            Some(Rc::new(mount.layout))
        } else {
            None
        };
        let editable_host = options
            .editable_brush
            .as_ref()
            .map(|_| editable_brush::Host::new(&document))
            .transpose()?;
        *engine.state.borrow_mut() = Some(State {
            model,
            web,
            source,
            data_labels,
            data_pointer,
            hidden,
            selected,
            surface,
            canvas,
            svg: None,
            tree_labels_passive,
            tooltip,
            brush,
            editable_host,
            editable_transport: None,
            options,
            revision: 0,
            last_size: (0., 0.),
            motion: None,
            last_time: None,
            pointer: None,
            focused: None,
            keyboard: false,
            reduced,
            presentation_error: None,
            contour_bindings,
            blends: Rc::new(blends),
            plot_widgets: widget_layout,
            staged_widgets: None,
            tree_canvas,
            tree_tooltip: None,
            tree_svg: None,
        });
        let mut initial_svg = None;
        if let Some(initial) = tree_svg {
            let tree_svg_frame::Mount {
                layout,
                plan,
                program,
                picking,
                components,
                generation,
                publication,
                current,
                motion,
            } = initial;
            engine.tree_svg_publication.set(publication);
            let packet = components.as_ref().map(|m| m.layout.clone());
            if let Some(components) = components {
                *engine.tree_svg_sources.borrow_mut() = Some(
                    plot_widgets::svg_occurrences::Sources::new(document.clone(), components),
                );
            }
            let stage = if let Some(motion) = motion {
                let revision = motion
                    .widgets
                    .as_ref()
                    .map_or(0, |(options, _)| options.revision);
                if let Some((options, factory)) = motion.widgets {
                    *engine.tree_animated_widgets.borrow_mut() =
                        Some(tree_animated_widgets::AnimatedWidgets::new(
                            document.clone(),
                            options,
                            factory,
                        ));
                }
                engine.prepare_tree_motion_svg(
                    layout,
                    plan,
                    program,
                    picking,
                    generation,
                    publication,
                    current,
                    motion.paint,
                    motion.frame,
                    revision,
                    true,
                )?
            } else {
                engine.prepare_tree_svg(
                    layout,
                    plan,
                    program,
                    picking,
                    packet,
                    generation,
                    publication,
                    current,
                )?
            };
            initial_svg = Some(stage.clone());
            engine.try_enqueue(Input::TreeSvg(stage))?;
        }
        engine.listen(controls)?;
        engine.enqueue(Input::Resize);
        // Initial admission has no public observer yet. Frame errors normally
        // become BrowserEvents after restoring accepted output, but the plain
        // pre-mount model is not an accepted SVG-layer presentation.
        let initial_error = Rc::new(RefCell::new(None));
        if initial_svg.is_some() {
            let captured = initial_error.clone();
            *engine.event.borrow_mut() = Some(Box::new(move |event| {
                if let BrowserEvent::Error(error) = event {
                    let mut first = captured.borrow_mut();
                    if first.is_none() {
                        *first = Some(error);
                    }
                }
            }));
        }
        let painted = engine.frame(0.);
        if initial_svg.is_some() {
            engine.event.borrow_mut().take();
        }
        painted?;
        if let Some(initial) = initial_svg {
            let adopted = initial.was_adopted()
                && engine.tree_svg_stage().is_some_and(|accepted| {
                    accepted.was_adopted() && accepted.same_admission(&initial)
                });
            if !adopted {
                return Err(initial_error.borrow_mut().take().unwrap_or_else(|| {
                    ChartError("initial Tree SVG presentation was not adopted".into())
                }));
            }
        }
        Ok(chart)
    }
    pub fn handle(&self) -> BrowserHandle {
        BrowserHandle {
            engine: Rc::downgrade(&self.engine),
        }
    }
    pub fn on_event(&self, callback: impl FnMut(BrowserEvent) + 'static) {
        if !self.engine.alive.get() {
            return;
        }
        self.engine
            .event_generation
            .set(self.engine.event_generation.get().wrapping_add(1));
        *self.engine.event.borrow_mut() = Some(Box::new(callback));
    }
    /// Called in RAF, outside native listeners and state borrows. A failed
    /// layout retains the previous scene; callbacks may stop their weak handle.
    pub fn on_resize(&self, mut layout: impl FnMut(f64, f64) -> Result<Scene> + 'static) {
        self.on_resize_with_interaction(move |w, h| layout(w, h).map(Into::into));
    }
    pub fn on_resize_with_interaction(
        &self,
        layout: impl FnMut(f64, f64) -> Result<SceneInteraction> + 'static,
    ) {
        if !self.engine.alive.get() {
            return;
        }
        *self.engine.layout.borrow_mut() = Some(Box::new(layout));
        self.engine
            .layout_generation
            .set(self.engine.layout_generation.get().wrapping_add(1));
        if let Some(state) = self.engine.state.borrow_mut().as_mut() {
            state.last_size = (0., 0.);
        }
        self.engine.enqueue(Input::Resize);
    }
    pub fn set_scene(&self, scene: Scene) -> Result<()> {
        self.handle().set_scene(scene)
    }
    pub fn set_scene_with_interaction(
        &self,
        scene: Scene,
        binding: InteractionBinding,
    ) -> Result<()> {
        self.handle().set_scene_with_interaction(scene, binding)
    }
    pub fn set_layout(&self, candidate: SceneInteraction) -> Result<()> {
        self.handle().set_layout(candidate)
    }
    pub fn set_layout_with_blends(
        &self,
        candidate: SceneInteraction,
        blends: MarkBlends,
    ) -> Result<()> {
        self.handle().set_layout_with_blends(candidate, blends)
    }
    pub fn animate_to_with_interaction(
        &self,
        scene: Scene,
        binding: InteractionBinding,
    ) -> Result<()> {
        self.handle().animate_to_with_interaction(scene, binding)
    }
    pub fn animate_to(&self, scene: Scene) -> Result<()> {
        self.handle().animate_to(scene)
    }
    pub fn animate_to_with_contours(
        &self,
        scene: Scene,
        bindings: motion::ContourBindings,
    ) -> Result<()> {
        self.handle().animate_to_with_contours(scene, bindings)
    }
    pub fn snapshot(&self) -> Result<BrowserSnapshot> {
        self.handle().snapshot()
    }
    pub fn flush(&self) -> Result<()> {
        self.handle().flush()
    }
    pub fn stop(&self) {
        self.engine.stop();
    }
}
impl Drop for BrowserChart {
    fn drop(&mut self) {
        self.engine.stop();
    }
}
impl BrowserHandle {
    pub fn set_layout(&self, candidate: SceneInteraction) -> Result<()> {
        self.set_layout_with_blends(candidate, MarkBlends::default())
    }
    /// Atomically replace geometry, interaction metadata and the blend profile.
    pub fn set_layout_with_blends(
        &self,
        candidate: SceneInteraction,
        modes: MarkBlends,
    ) -> Result<()> {
        candidate.validate()?;
        let engine = self.engine()?;
        {
            let state = engine.state.borrow();
            let s = state
                .as_ref()
                .ok_or_else(|| ChartError("browser chart is stopped".into()))?;
            s.admit_interaction_scene(&candidate.scene, candidate.interaction.as_ref())?;
            surface::validate_plot(candidate.plot.or(s.model.options().plot), &candidate.scene)?;
            blends::admit(
                s.options.renderer,
                &engine.window,
                &candidate.scene,
                &modes,
                s.model.paint_transform(),
            )?;
        }
        engine.try_enqueue(Input::Layout(Box::new((candidate, modes))))
    }
    pub fn is_active(&self) -> bool {
        self.engine.upgrade().is_some_and(|e| e.alive.get())
    }
    fn engine(&self) -> Result<Rc<Engine>> {
        let engine = self
            .engine
            .upgrade()
            .ok_or_else(|| ChartError("browser chart was dropped".into()))?;
        scene::ensure(engine.alive.get(), "browser chart is stopped")?;
        Ok(engine)
    }
    pub fn stop(&self) {
        if let Some(e) = self.engine.upgrade() {
            e.stop();
        }
    }
    pub fn set_scene(&self, scene: Scene) -> Result<()> {
        scene.validate()?;
        let engine = self.engine()?;
        {
            let state = engine.state.borrow();
            state
                .as_ref()
                .ok_or_else(|| ChartError("browser chart is stopped".into()))?
                .admit_interaction_scene(&scene, None)?;
        }
        engine.try_enqueue(Input::Scene(scene, false))
    }
    pub fn set_scene_with_interaction(
        &self,
        scene: Scene,
        binding: InteractionBinding,
    ) -> Result<()> {
        binding.validate(&scene)?;
        let engine = self.engine()?;
        {
            let state = engine.state.borrow();
            let state = state
                .as_ref()
                .ok_or_else(|| ChartError("browser chart is stopped".into()))?;
            state.admit_interaction_scene(&scene, Some(&binding))?;
        }
        engine.try_enqueue(Input::InteractionScene(Box::new((scene, binding, false))))
    }
    pub fn animate_to_with_interaction(
        &self,
        scene: Scene,
        binding: InteractionBinding,
    ) -> Result<()> {
        binding.validate(&scene)?;
        let engine = self.engine()?;
        engine.admit_blend_animation()?;
        {
            let state = engine.state.borrow();
            let state = state
                .as_ref()
                .ok_or_else(|| ChartError("browser chart is stopped".into()))?;
            state.admit_interaction_scene(&scene, Some(&binding))?;
            state.admit_animation_overlay(&scene, None, Some(&binding))?;
        }
        engine.try_enqueue(Input::InteractionScene(Box::new((scene, binding, true))))
    }
    pub fn animate_to(&self, scene: Scene) -> Result<()> {
        scene.validate()?;
        let engine = self.engine()?;
        engine.admit_blend_animation()?;
        {
            let state = engine.state.borrow();
            let state = state
                .as_ref()
                .ok_or_else(|| ChartError("browser chart is stopped".into()))?;
            state.admit_interaction_scene(&scene, None)?;
            state.admit_animation_overlay(&scene, None, None)?;
        }
        engine.try_enqueue(Input::Scene(scene, true))
    }
    /// Explicit contour keys are available only to Properties animation. Native
    /// property tracks are retargeted in RAF; this does not pre-sample a Scene.
    pub fn animate_to_with_contours(
        &self,
        scene: Scene,
        bindings: motion::ContourBindings,
    ) -> Result<()> {
        scene.validate()?;
        let engine = self.engine()?;
        engine.admit_blend_animation()?;
        {
            let state = engine.state.borrow();
            let state = state
                .as_ref()
                .ok_or_else(|| ChartError("browser chart is stopped".into()))?;
            scene::ensure(
                !matches!(state.options.animation, BrowserAnimation::Scene),
                "contour bindings require Properties animation",
            )?;
            state.admit_scene_with_contours(&scene, &bindings)?;
            state.admit_animation_overlay(&scene, Some(&bindings), None)?;
        }
        engine.try_enqueue(Input::ContourScene(scene, bindings))
    }
    pub fn reset_view(&self) -> Result<()> {
        self.engine()?
            .try_enqueue(Input::Action(SurfaceInput::Reset))
    }
    pub fn dispatch(&self, input: SurfaceInput) -> Result<()> {
        self.engine()?.try_enqueue(Input::Action(input))
    }
    pub fn set_hidden(&self, hidden: BTreeSet<String>) -> Result<()> {
        scene::ensure(
            hidden.len() <= 256 && hidden.iter().all(|s| !s.is_empty() && s.len() <= 1024),
            "invalid hidden series",
        )?;
        self.engine()?.try_enqueue(Input::Hidden(hidden))
    }
    /// Validate and queue a brush configuration in viewport logical pixels.
    /// Reconfiguration retires the previous pointer session on acceptance.
    pub fn configure_editable_brush(
        &self,
        options: Option<EditableBrushOptions>,
        policy: BrushReconfigure,
    ) -> Result<()> {
        let engine = self.engine()?;
        {
            let state = engine.state.borrow();
            let state = state
                .as_ref()
                .ok_or_else(|| ChartError("browser chart is stopped".into()))?;
            let mut candidate = state.model.clone();
            candidate.configure_editable_brush(options, policy)?;
        }
        engine.try_enqueue(Input::EditableBrush(editable_brush::Command::Configure(
            options, policy,
        )))
    }
    /// Set or clear a persistent selection, retiring any older gesture.
    pub fn set_brush_selection(&self, selection: Option<Brush>) -> Result<()> {
        let engine = self.engine()?;
        {
            let state = engine.state.borrow();
            let state = state
                .as_ref()
                .ok_or_else(|| ChartError("browser chart is stopped".into()))?;
            let mut candidate = state.model.clone();
            candidate.set_brush_selection(selection)?;
        }
        engine.try_enqueue(Input::EditableBrush(editable_brush::Command::Selection(
            selection,
        )))
    }
    pub fn flush(&self) -> Result<()> {
        let e = self.engine()?;
        if let Some(ticket) = e.ticket.take() {
            e.window.cancel_animation_frame(ticket).map_err(host)?;
        }
        let now = e.window.performance().map_or(0., |p| p.now());
        e.frame(now)
    }
    pub fn snapshot(&self) -> Result<BrowserSnapshot> {
        let e = self.engine()?;
        let state = e.state.borrow();
        let s = state
            .as_ref()
            .ok_or_else(|| ChartError("browser chart is stopped".into()))?;
        let brush_ids = s.model.editable_brush_ids()?;
        Ok(BrowserSnapshot {
            editable_brush: s.model.editable_brush().map(|brush| BrowserBrushSnapshot {
                options: *brush.options(),
                selection: brush.selection(),
                ids: brush_ids,
                active: brush.active(),
                mode: brush.mode(),
                session: brush.session(),
                captured_pointer: e.editable_capture.get().map(|capture| capture.id),
                touch: e
                    .editable_capture
                    .get()
                    .is_some_and(|capture| capture.touch),
            }),
            interaction: s.model.active_interaction(&s.tooltip_ids(&s.model)),
            active: e.alive.get(),
            renderer: s.options.renderer,
            width: s.model.scene().width,
            height: s.model.scene().height,
            hovered: s.model.hovered().to_vec(),
            selected: s.model.selected().map(str::to_owned),
            hidden: s.model.hidden().clone(),
            view: s.model.view(),
            revision: s.revision,
            pending: e.queue.borrow().len()
                + usize::from(e.resize_pending.get())
                + usize::from(e.pending_reduced.get().is_some()),
            dropped: e.dropped.get(),
            animating: s.motion.is_some(),
            animation_kind: s.motion.as_ref().map(RunningMotion::kind),
            touches: s.model.touch_count(),
            pinching: s.model.pinching(),
            captured_touches: e.touches.borrow().len(),
            domains: e.viewport_domains.get(),
        })
    }
}
impl Engine {
    fn admit_blend_animation(&self) -> Result<()> {
        scene::ensure(
            self.state
                .borrow()
                .as_ref()
                .is_some_and(|s| s.current_blends().is_empty() && s.blends.is_empty())
                && !self.queue.borrow().iter().any(|input| match input {
                    Input::Layout(candidate) => !candidate.1.is_empty(),
                    Input::BlendedAnimation(request) => !request.modes.is_empty(),
                    _ => false,
                }),
            "animation is unavailable with an active or queued blend profile",
        )
    }
    fn request(&self) -> Result<()> {
        if self.alive.get() && self.ticket.get().is_none() {
            let callback = self.callback.borrow();
            let callback = callback
                .as_ref()
                .ok_or_else(|| ChartError("browser frame callback unavailable".into()))?;
            self.ticket.set(Some(
                self.window
                    .request_animation_frame(callback.as_ref().unchecked_ref())
                    .map_err(host)?,
            ));
        }
        Ok(())
    }
    fn enqueue(&self, input: Input) {
        if let Err(error) = self.try_enqueue(input)
            && self.fault.borrow().is_none()
        {
            *self.fault.borrow_mut() = Some(error);
        }
    }
    fn try_enqueue(&self, input: Input) -> Result<()> {
        scene::ensure(self.alive.get(), "browser chart is stopped")?;
        if let Input::ReducedMotion(value) = input {
            // A system preference is latest authoritative configuration, not
            // a lossy gesture. Reserve one coalesced slot outside the action
            // queue, admitting it only after RAF scheduling succeeds.
            self.request()?;
            self.pending_reduced.set(Some(value));
            return Ok(());
        }
        if matches!(input, Input::Resize) {
            self.resize_pending.set(true);
            return self.request();
        }
        let mut queue = self.queue.borrow_mut();
        let coalesced = match &input {
            Input::Pointer {
                kind: PointerKind::Move,
                id,
                ..
            } => {
                matches!(queue.back(),Some(Input::Pointer{kind:PointerKind::Move,id:previous,..})if previous==id)
            }
            Input::Resize => matches!(queue.back(), Some(Input::Resize)),
            _ => false,
        };
        if !coalesced && queue.len() >= MAX_PENDING {
            self.dropped.set(self.dropped.get().saturating_add(1));
            drop(queue);
            self.request()?;
            return Err(ChartError(
                "browser input queue exceeded its 256-event budget".into(),
            ));
        }
        self.request()?;
        if coalesced {
            queue.pop_back();
        }
        queue.push_back(input);
        drop(queue);
        Ok(())
    }
    fn emit(&self, event: BrowserEvent) {
        if !self.alive.get() {
            return;
        }
        let generation = self.event_generation.get();
        let callback = self.event.borrow_mut().take();
        if let Some(mut callback) = callback {
            callback(event);
            if self.alive.get() && self.event_generation.get() == generation {
                *self.event.borrow_mut() = Some(callback);
            }
        }
    }
    fn stop(&self) {
        if !self.alive.replace(false) {
            return;
        }
        self.native_press.set(None);
        self.retired_native_press.set(None);
        if let Some(ticket) = self.ticket.take() {
            let _ = self.window.cancel_animation_frame(ticket);
        }
        if let Some((observer, _)) = self.observer.borrow_mut().take() {
            observer.disconnect();
        }
        self.listeners.borrow_mut().clear();
        let queued = std::mem::take(&mut *self.queue.borrow_mut());
        drop(queued);
        self.resize_pending.set(false);
        self.pending_reduced.set(None);
        self.event.borrow_mut().take();
        self.layout.borrow_mut().take();
        self.viewport.borrow_mut().take();
        self.viewport_configured.set(false);
        self.viewport_domains.set(None);
        self.viewport_controller.borrow_mut().take();
        self.tree_canvas_repaint.borrow_mut().take();
        self.tree_svg_retarget_model.borrow_mut().take();
        self.tree_svg_mesh_contacts.borrow_mut().clear();
        self.tree_mouse_double_click.borrow_mut().take();
        if let Some(sources) = self.tree_svg_sources.borrow_mut().take() {
            sources.stop();
        }
        if let Some(animated) = self.tree_animated_widgets.borrow_mut().take() {
            animated.stop();
        }
        if let Some(host) = self.tree_svg_host.borrow_mut().take() {
            host.stop();
        }
        if let Some(host) = self.tree_tooltips.borrow_mut().take() {
            host.stop();
        }
        let state = self.state.borrow_mut().take();
        if let Some(state) = state {
            self.cancel_captures(&state.surface);
            let _ = state.web.app().clear();
            drop(state);
        }
        let widgets = self.plot_widgets.borrow_mut().take();
        if let Some(widgets) = widgets {
            widgets.stop();
        }
        self.outer.remove();
        let _ = self.root.remove_attribute("data-kartis-browser");
    }
    fn cancel_captures(&self, surface: &HtmlElement) {
        self.retire_editable_capture(surface);
        self.editable_space.set(false);
        let mut ids = std::mem::take(&mut *self.touches.borrow_mut());
        if let Some(id) = self.captured.take() {
            ids.insert(id);
        }
        for id in ids {
            if surface.has_pointer_capture(id) {
                let _ = surface.release_pointer_capture(id);
            }
        }
    }
    fn retire_terminal(&self, terminal: Option<(i32, bool)>, events: &mut Vec<BrowserEvent>) {
        let Some((id, touch)) = terminal else {
            return;
        };
        let mut state = self.state.borrow_mut();
        let Some(state) = state.as_mut() else {
            return;
        };
        let input = if touch {
            SurfaceInput::TouchCancel { id }
        } else {
            SurfaceInput::Cancel { id }
        };
        match state.model.apply(input) {
            Ok(changes) => events.extend(changes.into_iter().map(BrowserEvent::Interaction)),
            Err(error) => events.push(BrowserEvent::Error(error)),
        }
    }
    fn add(
        self: &Rc<Self>,
        target: EventTarget,
        name: &'static str,
        callback: impl FnMut(Event) + 'static,
    ) -> Result<()> {
        let mut callback = callback;
        let callback = Closure::<dyn FnMut(Event)>::new(move |event: Event| {
            if matches!(
                name,
                "pointerdown"
                    | "pointermove"
                    | "lostpointercapture"
                    | "wheel"
                    | "dblclick"
                    | "keydown"
            ) && event
                .target()
                .and_then(|target| target.dyn_into::<Element>().ok())
                .is_some_and(|element| {
                    element
                        .closest(".kartis-plot-widget-slot, .kartis-tree-svg-custom-host")
                        .ok()
                        .flatten()
                        .is_some()
                })
            {
                return;
            }
            callback(event);
        });
        let options = web_sys::AddEventListenerOptions::new();
        options.set_passive(false);
        target
            .add_event_listener_with_callback_and_add_event_listener_options(
                name,
                callback.as_ref().unchecked_ref(),
                &options,
            )
            .map_err(host)?;
        self.listeners.borrow_mut().push(Listener {
            target,
            name,
            callback,
        });
        Ok(())
    }
    fn listen(self: &Rc<Self>, controls: HtmlElement) -> Result<()> {
        // Glory delivers datum pointer events on its own frame. Observe the
        // native pointer's region synchronously so a delayed control enter
        // cannot take over after the pointer has already left the controls.
        for (name, entering) in [("pointerover", true), ("pointerout", false)] {
            let weak = Rc::downgrade(self);
            let controls = controls.clone();
            self.add(controls.clone().into(), name, move |event| {
                let Some(e) = weak.upgrade() else {
                    return;
                };
                let Some(pointer) = event.dyn_ref::<PointerEvent>() else {
                    return;
                };
                if !matches!(pointer.pointer_type().as_str(), "mouse" | "pen") {
                    return;
                }
                let target = if entering {
                    event.target()
                } else {
                    pointer.related_target()
                };
                let datum = target
                    .and_then(|target| target.dyn_into::<Element>().ok())
                    .and_then(|element| element.closest(".kartis-datum").ok().flatten());
                e.pointer_over_datum
                    .set(datum.is_some_and(|datum| controls.contains(Some(&datum))));
            })?;
        }
        let surface = self.state.borrow().as_ref().unwrap().surface.clone();
        let weak = Rc::downgrade(self);
        self.add(surface.clone().into(), "focusin", move |event| {
            if let Some(engine) = weak.upgrade()
                && let Some(stage) = engine.tree_svg_stage()
                && let Some(target) = event.target().and_then(|t| t.dyn_into::<Element>().ok())
                && stage.root().contains(Some(&target))
                && target
                    .closest(".kartis-plot-widget-slot, .kartis-tree-svg-custom-host")
                    .ok()
                    .flatten()
                    .is_some()
            {
                engine.enqueue(Input::PlotWidgetFocus {
                    restoring: engine.processing.get(),
                });
            }
        })?;
        let weak = Rc::downgrade(self);
        self.add(surface.clone().into(), "pointerover", move |event| {
            if event
                .target()
                .and_then(|target| target.dyn_into::<Element>().ok())
                .is_some_and(|element| {
                    element
                        .closest(".kartis-plot-widget-slot, .kartis-tree-svg-custom-host")
                        .ok()
                        .flatten()
                        .is_some()
                })
                && let Some(engine) = weak.upgrade()
            {
                engine.enqueue(Input::Action(SurfaceInput::Leave));
            }
        })?;
        let weak = Rc::downgrade(self);
        let target = surface.clone();
        self.add(surface.clone().into(), "pointerdown", move |event| {
            let Some(e) = weak.upgrade() else {
                return;
            };
            let Some(p) = event.dyn_ref::<PointerEvent>() else {
                return;
            };
            if p.button() != 0 {
                return;
            }
            if p.pointer_type() != "touch" && e.retired_native_press.get() == Some(p.pointer_id()) {
                return;
            }
            if e.editable_pointer(PointerKind::Down, p, &event) {
                return;
            }
            let source_mesh = e.tree_svg_mesh_target(&event);
            let touch = p.pointer_type() == "touch";
            if touch {
                if e.captured.get().is_some()
                    || e.touches.borrow().contains(&p.pointer_id())
                    || e.touches.borrow().len() >= 2
                {
                    return;
                }
                if e.touches.borrow().len() == 1 {
                    let state = e.state.borrow();
                    let options = &state.as_ref().unwrap().options.interaction;
                    if !options.pinch || !options.zoom {
                        return;
                    }
                }
                e.touches.borrow_mut().insert(p.pointer_id());
            } else {
                if !p.is_primary() || e.captured.get().is_some() || !e.touches.borrow().is_empty() {
                    return;
                }
                e.captured.set(Some(p.pointer_id()));
            }
            let _ = target.set_pointer_capture(p.pointer_id());
            if source_mesh {
                e.tree_svg_mesh_contacts.borrow_mut().insert(p.pointer_id());
            } else {
                e.tree_svg_mesh_contacts
                    .borrow_mut()
                    .remove(&p.pointer_id());
            }
            let _ = target.focus();
            event.prevent_default();
            e.enqueue(Input::Pointer {
                kind: PointerKind::Down,
                id: p.pointer_id(),
                touch,
                client: client_coordinates(p),
                brush: p.shift_key(),
                source_mesh,
            });
        })?;
        let weak = Rc::downgrade(self);
        self.add(surface.clone().into(), "pointermove", move |event| {
            let Some(e) = weak.upgrade() else {
                return;
            };
            let Some(p) = event.dyn_ref::<PointerEvent>() else {
                return;
            };
            if e.editable_pointer(PointerKind::Move, p, &event) {
                return;
            }
            let touch = p.pointer_type() == "touch" || e.touches.borrow().contains(&p.pointer_id());
            if touch {
                if !e.touches.borrow().contains(&p.pointer_id()) {
                    return;
                }
            } else if !p.is_primary() || !e.touches.borrow().is_empty() {
                return;
            }
            e.enqueue(Input::Pointer {
                kind: PointerKind::Move,
                id: p.pointer_id(),
                touch,
                client: client_coordinates(p),
                brush: false,
                source_mesh: e.tree_svg_mesh_contacts.borrow().contains(&p.pointer_id())
                    || e.tree_svg_mesh_target(&event),
            });
        })?;
        let weak = Rc::downgrade(self);
        self.add(surface.clone().into(), "pointerleave", move |_| {
            if let Some(e) = weak.upgrade() {
                e.enqueue(Input::Action(SurfaceInput::Leave));
            }
        })?;
        let weak = Rc::downgrade(self);
        self.add(surface.clone().into(), "lostpointercapture", move |event| {
            let Some(e) = weak.upgrade() else {
                return;
            };
            let Some(p) = event.dyn_ref::<PointerEvent>() else {
                return;
            };
            if e.editable_pointer(PointerKind::Cancel, p, &event) {
                return;
            }
            if e.captured.get() == Some(p.pointer_id()) {
                e.captured.set(None);
                e.enqueue(Input::Pointer {
                    kind: PointerKind::Cancel,
                    id: p.pointer_id(),
                    touch: false,
                    client: client_coordinates(p),
                    brush: false,
                    source_mesh: e
                        .tree_svg_mesh_contacts
                        .borrow_mut()
                        .remove(&p.pointer_id()),
                });
            } else if e.touches.borrow_mut().remove(&p.pointer_id()) {
                e.enqueue(Input::Pointer {
                    kind: PointerKind::Cancel,
                    id: p.pointer_id(),
                    touch: true,
                    client: (0., 0.),
                    brush: false,
                    source_mesh: e
                        .tree_svg_mesh_contacts
                        .borrow_mut()
                        .remove(&p.pointer_id()),
                });
            }
        })?;
        let weak = Rc::downgrade(self);
        let target = surface.clone();
        self.add(self.window.clone().into(), "pointermove", move |event| {
            let Some(e) = weak.upgrade() else {
                return;
            };
            let Some(p) = event.dyn_ref::<PointerEvent>() else {
                return;
            };
            let touch = e.touches.borrow().contains(&p.pointer_id());
            if e.captured.get() != Some(p.pointer_id()) && !touch {
                return;
            }
            if event
                .target()
                .and_then(|t| t.dyn_into::<web_sys::Node>().ok())
                .is_some_and(|node| target.contains(Some(&node)))
            {
                return;
            }
            e.enqueue(Input::Pointer {
                kind: PointerKind::Move,
                id: p.pointer_id(),
                touch,
                client: client_coordinates(p),
                brush: false,
                source_mesh: e.tree_svg_mesh_contacts.borrow().contains(&p.pointer_id()),
            });
        })?;
        for (name, kind) in [
            ("pointerup", PointerKind::Up),
            ("pointercancel", PointerKind::Cancel),
        ] {
            let weak = Rc::downgrade(self);
            let target = surface.clone();
            self.add(self.window.clone().into(), name, move |event| {
                let Some(e) = weak.upgrade() else {
                    return;
                };
                let Some(p) = event.dyn_ref::<PointerEvent>() else {
                    return;
                };
                if p.pointer_type() != "touch"
                    && e.retired_native_press.get() == Some(p.pointer_id())
                {
                    e.retired_native_press.set(None);
                    e.tree_svg_mesh_contacts
                        .borrow_mut()
                        .remove(&p.pointer_id());
                    return;
                }
                let touch = e.touches.borrow_mut().remove(&p.pointer_id());
                if e.captured.get() != Some(p.pointer_id()) && !touch {
                    return;
                }
                if !touch {
                    e.captured.set(None);
                }
                if target.has_pointer_capture(p.pointer_id()) {
                    let _ = target.release_pointer_capture(p.pointer_id());
                }
                e.enqueue(Input::Pointer {
                    kind,
                    id: p.pointer_id(),
                    touch,
                    client: client_coordinates(p),
                    brush: false,
                    source_mesh: e
                        .tree_svg_mesh_contacts
                        .borrow_mut()
                        .remove(&p.pointer_id()),
                });
            })?;
        }
        let weak = Rc::downgrade(self);
        let zoom = self
            .state
            .borrow()
            .as_ref()
            .unwrap()
            .options
            .interaction
            .zoom;
        self.add(surface.clone().into(), "wheel", move |event| {
            if !zoom {
                return;
            }
            let Some(e) = weak.upgrade() else {
                return;
            };
            let Some(w) = event.dyn_ref::<WheelEvent>() else {
                return;
            };
            event.prevent_default();
            e.enqueue(Input::Wheel {
                client: client_coordinates(w),
                delta: w.delta_y(),
                mode: w.delta_mode(),
            });
        })?;
        let weak = Rc::downgrade(self);
        self.add(surface.clone().into(), "dblclick", move |event| {
            if let Some(e) = weak.upgrade() {
                if e.tree_mouse_double_click
                    .borrow_mut()
                    .take()
                    .is_some_and(|owned| owned == event)
                {
                    return;
                }
                event.prevent_default();
                e.enqueue(Input::Action(SurfaceInput::Reset));
            }
        })?;
        let weak = Rc::downgrade(self);
        self.add(surface.clone().into(), "keydown", move |event| {
            let Some(e) = weak.upgrade() else {
                return;
            };
            let Some(k) = event.dyn_ref::<KeyboardEvent>() else {
                return;
            };
            if e.editable_key_owned(&event) {
                return;
            }
            if k.ctrl_key() || k.meta_key() || k.alt_key() {
                return;
            }
            let input = match k.key().as_str() {
                "ArrowRight" | "ArrowDown" => Some(SurfaceInput::Next),
                "ArrowLeft" | "ArrowUp" => Some(SurfaceInput::Previous),
                "Home" => Some(SurfaceInput::First),
                "End" => Some(SurfaceInput::Last),
                "Enter" | " " => Some(SurfaceInput::Activate),
                "Escape" => Some(SurfaceInput::Clear),
                "0" => Some(SurfaceInput::Reset),
                _ => None,
            };
            if let Some(input) = input {
                event.prevent_default();
                e.enqueue(Input::Action(input));
            }
        })?;
        let weak = Rc::downgrade(self);
        let callback = Closure::<dyn FnMut()>::new(move || {
            if let Some(e) = weak.upgrade() {
                e.enqueue(Input::Resize);
            }
        });
        let observer = ResizeObserver::new(callback.as_ref().unchecked_ref()).map_err(host)?;
        observer.observe(&surface);
        *self.observer.borrow_mut() = Some((observer, callback));
        let weak = Rc::downgrade(self);
        self.add(self.window.clone().into(), "resize", move |_| {
            if let Some(e) = weak.upgrade() {
                e.enqueue(Input::Resize);
            }
        })?;
        if self.state.borrow().as_ref().unwrap().options.motion == MotionPreference::System
            && let Some(media) = self
                .window
                .match_media("(prefers-reduced-motion: reduce)")
                .map_err(host)?
        {
            let weak = Rc::downgrade(self);
            let query = media.clone();
            self.add(media.into(), "change", move |_| {
                if let Some(e) = weak.upgrade() {
                    e.enqueue(Input::ReducedMotion(query.matches()));
                }
            })?;
        }
        self.install_editable_brush()?;
        Ok(())
    }
    fn frame(&self, time: f64) -> Result<()> {
        if !self.alive.get() {
            return Ok(());
        }
        scene::ensure(time.is_finite(), "nonfinite frame timestamp")?;
        scene::ensure(!self.processing.replace(true), "reentrant browser frame")?;
        struct Guard<'a>(&'a Cell<bool>);
        impl Drop for Guard<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }
        let _guard = Guard(&self.processing);
        let mut inputs = Vec::with_capacity(self.queue.borrow().len() + 2);
        if let Some(value) = self.pending_reduced.take() {
            // Apply current policy before actions, including animation requests
            // accepted while a media change and a full queue share one frame.
            inputs.push(Input::ReducedMotion(value));
        }
        inputs.extend(self.queue.borrow_mut().drain(..));
        if self.resize_pending.replace(false) {
            inputs.push(Input::Resize);
        }
        // A queued composition owns staged Widget scopes. Keep their last Rc
        // alive until all State borrows have ended, including rejected inputs
        // and paint checkpoint restoration: cleanup may read the weak handle.
        let mut staged_lifetimes: Vec<_> = inputs
            .iter()
            .filter_map(|input| match input {
                Input::PlotWidgetComposition(candidate) => Some(candidate.2.clone()),
                Input::TreeCanvas(candidate) => candidate.composition.as_ref().map(|c| c.1.clone()),
                Input::TreeTooltip(candidate) => candidate.source_stage.clone(),
                _ => None,
            })
            .collect();
        if let Some(stage) = self
            .state
            .borrow()
            .as_ref()
            .and_then(|s| s.staged_widgets.clone())
        {
            staged_lifetimes.push(stage);
        }
        let _tooltip_lifetimes: Vec<_> = inputs
            .iter()
            .filter_map(|input| match input {
                Input::TreeTooltip(c) => Some(c.stage.clone()),
                _ => None,
            })
            .collect();
        let _svg_lifetimes: Vec<_> = inputs
            .iter()
            .filter_map(|input| match input {
                Input::TreeSvg(stage) => Some(stage.clone()),
                Input::TreeTooltip(candidate) => match candidate.input.as_deref() {
                    Some(Input::TreeSvg(stage)) => Some(stage.clone()),
                    _ => None,
                },
                _ => None,
            })
            .collect();
        let tooltip_only = !inputs.is_empty()
            && inputs
                .iter()
                .all(|input| matches!(input,Input::TreeTooltip(c)if c.input.is_none()));
        let _staged_lifetimes = staged_lifetimes;
        let mut events = vec![];
        let blended = self
            .state
            .borrow()
            .as_ref()
            .is_some_and(|s| !s.current_blends().is_empty() || !s.blends.is_empty())
            || inputs.iter().any(|input| match input {
                Input::Layout(candidate) => !candidate.1.is_empty(),
                Input::BlendedAnimation(_) => true,
                _ => false,
            });
        let slots = self
            .state
            .borrow()
            .as_ref()
            .is_some_and(|s| s.plot_widgets.is_some());
        let editing = self
            .state
            .borrow()
            .as_ref()
            .is_some_and(|s| s.model.editable_brush().is_some())
            || inputs
                .iter()
                .any(|input| matches!(input, Input::EditableBrush(_)));
        let viewport_checkpoint = (blended || slots || editing).then(|| {
            (
                self.viewport_generation.get(),
                self.viewport_controller.borrow().clone(),
            )
        });
        // Retain the accepted input/model state until its owned decoration has
        // passed complete Scene admission, including cumulative dash work.
        let mut paint_checkpoint = self.state.borrow().as_ref().and_then(|s| {
            (s.tree_canvas.is_some()
                || s.tree_svg.is_some()
                || s.tree_tooltip.is_some()
                || blended
                || s.model.editable_brush().is_some()
                || inputs.iter().any(|i| matches!(i, Input::EditableBrush(_)))
                || s.transactional_geometry(s.model.interaction_data())
                || inputs.iter().any(|i| {
                    matches!(
                        i,
                        Input::InteractionScene(_)
                            | Input::Layout(_)
                            | Input::TreeCanvas(_)
                            | Input::TreeSvg(_)
                            | Input::TreeTooltip(_)
                            | Input::Resize
                            | Input::ViewportRequest(_)
                    )
                }))
            .then(|| (s.gesture(), s.focused.clone()))
        });
        let fault = self.fault.borrow_mut().take();
        let mut explicit_accepted = false;
        let mut forced_press_cancel = fault.is_some();
        let mut paint_accepted = true;
        let mut tree_canvas_rejected = false;
        for input in inputs {
            if !self.alive.get() {
                return Ok(());
            }
            let (input, tooltip_candidate) = match input {
                Input::TreeTooltip(candidate) => {
                    let tree_tooltip_host::Candidate { input, stage, .. } = *candidate;
                    if let Some(input) = input {
                        (*input, Some(stage))
                    } else {
                        let result = {
                            let mut state = self.state.borrow_mut();
                            let s = state.as_mut().expect("active tooltip chart");
                            stage
                                .validate(&s.tree_tooltip_model()?)
                                .and_then(|()| {
                                    scene::ensure(
                                        s.tree_tooltip.as_ref().is_none_or(|old| {
                                            old.source_generation == stage.source_generation
                                        }),
                                        "Tree tooltip-only source generation is stale",
                                    )
                                })
                                .map(|()| {
                                    s.tree_tooltip = Some(stage);
                                })
                        };
                        if let Err(error) = result {
                            paint_accepted = false;
                            events.push(BrowserEvent::Error(error));
                        }
                        continue;
                    }
                }
                input => (input, None),
            };
            if matches!(&input, Input::Hover(Some(_))) && !self.pointer_over_datum.get() {
                continue;
            }
            let request = match &input {
                Input::ViewportRequest(request) => Some(*request),
                Input::Action(SurfaceInput::Reset) if self.viewport_configured.get() => {
                    Some(ViewportRequest::Reset)
                }
                _ => None,
            };
            if let Some(request) = request {
                match self.relayout_viewport_request(None, None, Some(request)) {
                    Ok(Some(changes)) => {
                        explicit_accepted = true;
                        events.extend(changes.into_iter().map(BrowserEvent::Interaction))
                    }
                    Ok(None) => {}
                    Err(error) => events.push(BrowserEvent::Error(error)),
                }
                continue;
            }
            if let Input::LegendActivate(activation) = input {
                let state = self.state.borrow();
                let s = state.as_ref().unwrap();
                let scene = s.model.scene();
                if s.options.legend_actions
                    && scene.legend_role(&activation.item.id) == Some(activation.role)
                    && scene.legend_target(&activation.item.id)
                        == Some(activation.resolved_target())
                    && let Some(item) = scene.legend.iter().find(|i| i.id == activation.item.id)
                {
                    events.push(BrowserEvent::LegendActivated(LegendActivation {
                        item: item.clone(),
                        role: activation.role,
                        target: scene.legend_targets.get(&item.id).cloned(),
                    }));
                }
                continue;
            }
            if let Input::Focus(id) = input {
                let mut state = self.state.borrow_mut();
                let s = state.as_mut().unwrap();
                let next = id.filter(|id| s.model.logical_datum(id).is_some());
                if s.focused != next {
                    s.focused = next.clone();
                    if next.is_some() {
                        s.pointer = None;
                        events.extend(
                            s.model
                                .set_hovered(vec![])?
                                .into_iter()
                                .map(BrowserEvent::Interaction),
                        );
                    }
                    events.push(BrowserEvent::Focus(next));
                }
                continue;
            }
            if let Input::PlotWidgetFocus { restoring } = input {
                let mut state = self.state.borrow_mut();
                let s = state.as_mut().unwrap();
                if s.focused.take().is_some() {
                    events.push(BrowserEvent::Focus(None));
                }
                s.keyboard = false;
                if !restoring {
                    s.pointer = None;
                    events.extend(
                        s.model
                            .apply(SurfaceInput::Leave)?
                            .into_iter()
                            .map(BrowserEvent::Interaction),
                    );
                }
                continue;
            }
            if matches!(input, Input::ViewportRefresh) {
                let size = {
                    let state = self.state.borrow();
                    let s = state.as_ref().unwrap();
                    let (w, h) = s.last_size;
                    if w > 0.
                        && h > 0.
                        && (w != s.model.scene().width || h != s.model.scene().height)
                    {
                        Some((w, h))
                    } else {
                        None
                    }
                };
                match self.relayout_viewport(None, size) {
                    Ok(Some(changes)) => {
                        explicit_accepted = true;
                        events.extend(changes.into_iter().map(BrowserEvent::Interaction))
                    }
                    Ok(None) => {}
                    Err(error) => events.push(BrowserEvent::Error(error)),
                }
                continue;
            }
            if matches!(input, Input::Resize) {
                let size = {
                    let state = self.state.borrow();
                    let s = state.as_ref().unwrap();
                    let rect = s.surface.get_bounding_client_rect();
                    (rect.width(), rect.height())
                };
                let changed = self.state.borrow().as_ref().unwrap().last_size != size;
                if changed {
                    forced_press_cancel = true;
                    let mut state = self.state.borrow_mut();
                    let s = state.as_mut().unwrap();
                    s.model.cancel_drag();
                    self.cancel_captures(&s.surface);
                }
                if size.0 <= 0. || size.1 <= 0. {
                    self.state.borrow_mut().as_mut().unwrap().last_size = size;
                    continue;
                }
                if changed {
                    let data_viewport = self.viewport.borrow().is_some();
                    if data_viewport {
                        match self.relayout_viewport(None, Some(size)) {
                            Ok(Some(changes)) => {
                                events.extend(changes.into_iter().map(BrowserEvent::Interaction))
                            }
                            Ok(None) => {}
                            Err(error) => events.push(BrowserEvent::Error(error)),
                        }
                        if !self.alive.get() {
                            return Ok(());
                        }
                        self.state.borrow_mut().as_mut().unwrap().last_size = size;
                        events.push(BrowserEvent::Resized {
                            width: size.0,
                            height: size.1,
                        });
                        continue;
                    }
                    let generation = self.layout_generation.get();
                    let callback = self.layout.borrow_mut().take();
                    if let Some(mut callback) = callback {
                        let result = callback(size.0, size.1);
                        if !self.alive.get() {
                            return Ok(());
                        }
                        if self.layout_generation.get() != generation {
                            continue;
                        }
                        if self.layout.borrow().is_none() {
                            *self.layout.borrow_mut() = Some(callback);
                        }
                        match result {
                            Ok(candidate) => {
                                let mut state = self.state.borrow_mut();
                                let s = state.as_mut().unwrap();
                                match s
                                    .admit_interaction_scene(
                                        &candidate.scene,
                                        candidate.interaction.as_ref(),
                                    )
                                    .and_then(|_| s.admit_scene(&candidate.scene))
                                    .and_then(|_| {
                                        let modes = s.blends.for_scene(&candidate.scene);
                                        blends::admit(
                                            s.options.renderer,
                                            &self.window,
                                            &candidate.scene,
                                            &modes,
                                            s.model.paint_transform(),
                                        )
                                    })
                                    .and_then(|_| s.model.set_layout(candidate))
                                {
                                    Ok(changes) => {
                                        s.motion = None;
                                        s.last_time = None;
                                        s.contour_bindings.clear();
                                        events.extend(
                                            changes.into_iter().map(BrowserEvent::Interaction),
                                        );
                                    }
                                    Err(error) => events.push(BrowserEvent::Error(error)),
                                }
                            }
                            Err(error) => events.push(BrowserEvent::Error(error)),
                        }
                    }
                    self.state.borrow_mut().as_mut().unwrap().last_size = size;
                    events.push(BrowserEvent::Resized {
                        width: size.0,
                        height: size.1,
                    });
                }
                continue;
            }
            if let Input::LegendToggle(activation) = &input {
                // Discard a retired row/target before relayout can invoke any
                // application callback, not only before changing Hidden.
                let state = self.state.borrow();
                let scene = state.as_ref().unwrap().model.scene();
                let target = activation.resolved_target();
                if activation.role != LegendRole::Series
                    || !matches!(&target, LegendTarget::Series(_))
                    || scene.legend_target(&activation.item.id) != Some(target)
                {
                    continue;
                }
            }
            let refresh_layout = matches!(&input, Input::Hidden(_) | Input::LegendToggle(_));
            let explicit = native_press::explicit_change(&input)
                || matches!(&input, Input::TreeCanvas(_) | Input::TreeSvg(_));
            let raw_input = matches!(&input, Input::TreeCanvas(_) | Input::TreeSvg(_));
            let terminal = match &input {
                Input::Pointer {
                    kind: PointerKind::Up | PointerKind::Cancel,
                    id,
                    touch,
                    ..
                } => Some((*id, *touch)),
                Input::Action(SurfaceInput::Up { id, .. } | SurfaceInput::Cancel { id }) => {
                    Some((*id, false))
                }
                Input::Action(
                    SurfaceInput::TouchEnd { id, .. } | SurfaceInput::TouchCancel { id },
                ) => Some((*id, true)),
                _ => None,
            };
            let mut state = self.state.borrow_mut();
            let s = state.as_mut().unwrap();
            let previous = self.viewport.borrow().as_ref().map(|_| s.gesture());
            let tip_previous = tooltip_candidate.as_ref().map(|_| s.gesture());
            let source_replacement = matches!(
                &input,
                Input::Scene(..)
                    | Input::InteractionScene(_)
                    | Input::Layout(_)
                    | Input::TreeCanvas(_)
                    | Input::TreeSvg(_)
                    | Input::PlotWidgetLayout(_)
                    | Input::ContourScene(..)
            ) || matches!(&input,Input::PlotWidgetComposition(c)if c.0.is_some());
            match s.process(input, &self.window) {
                Ok(mut changes) => {
                    if source_replacement {
                        s.tree_tooltip = None;
                    }
                    if let Some(stage) = tooltip_candidate {
                        if let Err(error) = s
                            .tree_tooltip_model()
                            .and_then(|model| stage.validate(&model))
                        {
                            s.restore_gesture(tip_previous.expect("tooltip checkpoint"));
                            paint_accepted = false;
                            events.push(BrowserEvent::Error(error));
                            continue;
                        }
                        s.tree_tooltip = Some(stage);
                    }
                    if let Some(previous) = previous
                        && (refresh_layout
                            || changes
                                .iter()
                                .any(|change| matches!(change, SurfaceEvent::View(_))))
                    {
                        let proposed = s.gesture();
                        s.restore_gesture(previous);
                        drop(state);
                        match self.relayout_viewport(Some(proposed), None) {
                            Ok(Some(extra)) => {
                                explicit_accepted |= explicit;
                                changes.retain(|change| !matches!(change, SurfaceEvent::View(_)));
                                changes.extend(extra);
                                changes.push(SurfaceEvent::View(
                                    self.state.borrow().as_ref().unwrap().model.view(),
                                ));
                                events.extend(changes.into_iter().map(BrowserEvent::Interaction));
                            }
                            Ok(None) => {
                                self.retire_terminal(terminal, &mut events);
                            }
                            Err(error) => {
                                self.retire_terminal(terminal, &mut events);
                                events.push(BrowserEvent::Error(error));
                            }
                        }
                    } else {
                        explicit_accepted |= explicit;
                        events.extend(changes.into_iter().map(BrowserEvent::Interaction));
                    }
                }
                Err(error) => {
                    if raw_input {
                        tree_canvas_rejected = true;
                        paint_accepted = false;
                    }
                    events.push(BrowserEvent::Error(error));
                }
            }
        }
        if !self.alive.get() {
            return Ok(());
        }
        if tooltip_only && fault.is_none() {
            if !paint_accepted {
                // A rejected incoming tooltip still owns its pending lease.
                // Do not prepare a second stage from the accepted request and
                // hide the original validation error behind a budget failure.
                if let Some((baseline, focused)) = paint_checkpoint.take() {
                    let mut state = self.state.borrow_mut();
                    let s = state
                        .as_mut()
                        .ok_or_else(|| ChartError("Tree tooltip chart stopped".into()))?;
                    s.restore_gesture(baseline);
                    s.focused = focused;
                }
                // Cleanup runs only after the State borrow has ended; it may
                // observe the previous complete accepted source/presentation.
                drop(_tooltip_lifetimes);
                drop(_staged_lifetimes);
                drop(_svg_lifetimes);
                return self.finish_frame_events(events);
            }
            let stage = self.tooltip_stage_for_paint()?;
            let result: Result<()> = (|| {
                if let Some(stage) = &stage {
                    self.place_tree_tooltip(stage)?;
                }
                {
                    let mut state = self.state.borrow_mut();
                    let s = state
                        .as_mut()
                        .ok_or_else(|| ChartError("Tree tooltip chart stopped".into()))?;
                    let scene = s.model.scene().clone();
                    let ids = s.tooltip_ids(&s.model);
                    let interaction = s.model.active_interaction(&ids);
                    s.paint_tooltip(&scene, &ids, interaction.as_ref())?;
                    if let Some(stage) = &stage {
                        s.tree_tooltip = Some(stage.clone());
                    }
                }
                if let Some(stage) = &stage {
                    stage.adopt();
                }
                Ok(())
            })();
            if let Err(error) = result {
                if let Some(stage) = &stage {
                    stage.rollback()?;
                }
                if let Some((baseline, focused)) = paint_checkpoint.take() {
                    let mut state = self.state.borrow_mut();
                    let s = state
                        .as_mut()
                        .ok_or_else(|| ChartError("Tree tooltip chart stopped".into()))?;
                    s.restore_gesture(baseline);
                    s.focused = focused;
                    let scene = s.model.scene().clone();
                    let ids = s.tooltip_ids(&s.model);
                    let interaction = s.model.active_interaction(&ids);
                    s.paint_tooltip(&scene, &ids, interaction.as_ref())?;
                }
                events.push(BrowserEvent::Error(error));
            } else {
                events.extend(
                    self.pump_tree_tooltip()
                        .into_iter()
                        .map(BrowserEvent::Error),
                );
            }
            return self.finish_frame_events(events);
        }
        let mut restore_editable_focus = None;
        let ordered_mode = self
            .state
            .borrow()
            .as_ref()
            .is_some_and(State::source_ordered);
        let ordered_baseline = paint_checkpoint.clone();
        let mut paint_fault = None;
        let mut prepared_animation = None;
        {
            let mut state = self.state.borrow_mut();
            let s = state
                .as_mut()
                .ok_or_else(|| ChartError("browser chart stopped".into()))?;
            if let Some(error) = fault {
                s.model.cancel_drag();
                self.cancel_captures(&s.surface);
                events.push(BrowserEvent::Error(error));
            }
            if s.focused
                .as_ref()
                .is_some_and(|id| s.model.logical_datum(id).is_none())
            {
                s.focused = None;
                events.push(BrowserEvent::Focus(None));
            }
            if s.motion.is_some()
                && (!s.current_blends().is_empty()
                    || s.transactional_geometry(s.model.interaction_data()))
            {
                // A failed decorated frame must not advance the accepted
                // physical tracks, metadata, clock or published Scene.
                let candidate = (|| -> Result<_> {
                    let mut animation = s.motion.clone().expect("checked motion");
                    let delta = s
                        .last_time
                        .map_or(0., |old| ((time - old) / 1000.).clamp(0., 0.25));
                    animation.tick(delta)?;
                    crosshair::overlay_admission(
                        animation.scene(),
                        s.overlay_enabled(s.model.interaction_data()),
                    )?;
                    let mut model = s.model.clone();
                    let changes =
                        model.set_scene_preserving_interaction(animation.scene().clone())?;
                    let prepared = if s.current_blends().is_empty() {
                        s.paint_scene_for(&model)?;
                        None
                    } else {
                        Some(s.prepare_frame_for(&model, animation.blends(), &self.window, true)?)
                    };
                    Ok((animation, model, changes, prepared))
                })();
                match candidate {
                    Ok((animation, model, changes, prepared)) => {
                        prepared_animation = prepared;
                        s.model = model;
                        s.contour_bindings = animation.paint_bindings();
                        if animation.is_settled() {
                            s.motion = None;
                            s.last_time = None;
                        } else {
                            s.motion = Some(animation);
                            s.last_time = Some(time);
                        }
                        events.extend(changes.into_iter().map(BrowserEvent::Interaction));
                    }
                    Err(error) => paint_fault = Some(error),
                }
            } else if let Some(animation) = &mut s.motion {
                let delta = s
                    .last_time
                    .map_or(0., |old| ((time - old) / 1000.).clamp(0., 0.25));
                s.last_time = Some(time);
                animation.tick(delta)?;
                events.extend(
                    s.model
                        .set_scene_preserving_interaction(animation.scene().clone())?
                        .into_iter()
                        .map(BrowserEvent::Interaction),
                );
                s.contour_bindings = animation.paint_bindings();
                if animation.is_settled() {
                    s.motion = None;
                    s.last_time = None;
                }
            }
        }
        if !forced_press_cancel && self.hold_native_press_paint(explicit_accepted) {
            // Drain logical input and ordinary UI events, but keep the accepted
            // source/hit tree and Widget parents until native click dispatch.
            // In particular Series highlighting and crosshair helpers must not
            // swap the hit target underneath a pending click.
            if let Some(state) = self.state.borrow_mut().as_mut() {
                state.publish()?;
            }
            if let Some(host) = self.plot_widgets.borrow().clone()
                && let Err(error) = host.flush_cleanup()
            {
                events.push(BrowserEvent::Error(error));
            }
            drop(_staged_lifetimes);
            return self.finish_frame_events(events);
        }
        if let Err(error) = self.repaint_tree_canvas_if_needed(paint_checkpoint.as_ref()) {
            if !self.alive.get() {
                return Ok(());
            }
            tree_canvas_rejected = true;
            paint_accepted = false;
            paint_fault = Some(error);
        }
        let _svg_repaint_lifetime = match self.repaint_tree_svg_if_needed(paint_checkpoint.as_ref())
        {
            Ok(stage) => stage,
            Err(error) => {
                if !self.alive.get() {
                    return Ok(());
                }
                tree_canvas_rejected = true;
                paint_accepted = false;
                paint_fault = Some(error);
                None
            }
        };
        let mut ordered_frame = None;
        let mut ordered_rejected = false;
        if ordered_mode {
            let host = self
                .plot_widgets
                .borrow()
                .clone()
                .ok_or_else(|| ChartError("ordered Widget host missing".into()))?;
            if paint_fault.is_none() {
                let work = (|| -> Result<_> {
                    let state = self.state.borrow();
                    let s = state
                        .as_ref()
                        .ok_or_else(|| ChartError("ordered chart stopped".into()))?;
                    Ok((
                        s.ordered_request(&self.window)?,
                        s.prepare_widgets_for(&s.model)?
                            .ok_or_else(|| ChartError("ordered rows missing".into()))?,
                        s.staged_widgets.clone(),
                        s.gesture(),
                    ))
                })();
                let result = match work {
                    Ok((request, rows, stage, candidate)) => {
                        if let Some((baseline, _)) = &ordered_baseline {
                            self.state
                                .borrow_mut()
                                .as_mut()
                                .expect("active ordered chart")
                                .restore_gesture(baseline.clone());
                        }
                        let result = host.prepare_ordered(request, rows, stage);
                        if !self.alive.get() {
                            return Ok(());
                        }
                        if result.is_ok() {
                            self.state
                                .borrow_mut()
                                .as_mut()
                                .expect("active ordered chart")
                                .restore_gesture(candidate);
                        }
                        result
                    }
                    Err(error) => Err(error),
                };
                match result {
                    Ok(frame) => ordered_frame = Some(frame),
                    Err(error) => {
                        if !self.alive.get() {
                            return Ok(());
                        }
                        if !host.has_ordered_frame() {
                            return Err(error);
                        }
                        if let Some((baseline, focused)) = &ordered_baseline {
                            let mut state = self.state.borrow_mut();
                            restore_editable_focus = self.restore_paint_checkpoint(
                                state.as_mut().expect("active ordered chart"),
                                baseline.clone(),
                                focused.clone(),
                                viewport_checkpoint.as_ref(),
                            )?;
                        }
                        prepared_animation = None;
                        paint_accepted = false;
                        ordered_rejected = true;
                        events.clear();
                        events.push(BrowserEvent::Error(error));
                    }
                }
            } else {
                if !host.has_ordered_frame() {
                    return Err(paint_fault.take().expect("paint fault"));
                }
                ordered_rejected = true;
                paint_accepted = false;
            }
        }
        let mut widget_frame = {
            let mut state = self.state.borrow_mut();
            let s = state
                .as_mut()
                .ok_or_else(|| ChartError("browser chart stopped".into()))?;
            let (prepared, mut widgets) = match paint_fault.map_or_else(
                || {
                    if !tree_canvas_rejected {
                        s.validate_tree_canvas_paint(self.window.device_pixel_ratio())?;
                    }
                    prepared_animation.map_or_else(|| s.prepare_frame(&self.window, blended), Ok)
                },
                Err,
            ) {
                Ok(prepared) => prepared,
                Err(error) => {
                    paint_accepted = false;
                    ordered_frame = None;
                    ordered_rejected |= ordered_mode;
                    if let Some((accepted, focused)) = paint_checkpoint.take() {
                        restore_editable_focus = self.restore_paint_checkpoint(
                            s,
                            accepted,
                            focused,
                            viewport_checkpoint.as_ref(),
                        )?;
                        events.clear();
                        events.push(BrowserEvent::Error(error));
                        s.prepare_frame(&self.window, blended)?
                    } else {
                        return Err(error);
                    }
                }
            };
            let retired: Vec<_> = self
                .touches
                .borrow()
                .iter()
                .copied()
                .filter(|id| !s.model.has_touch(*id))
                .collect();
            for id in retired {
                self.touches.borrow_mut().remove(&id);
                if s.surface.has_pointer_capture(id) {
                    let _ = s.surface.release_pointer_capture(id);
                }
            }
            if let Err(error) = s.publish().and_then(|_| {
                if ordered_rejected || tree_canvas_rejected {
                    s.paint_owned(self.window.device_pixel_ratio(), prepared, false)
                } else {
                    s.paint(self.window.device_pixel_ratio(), prepared)
                }
            }) {
                paint_accepted = false;
                ordered_frame = None;
                if let Some((accepted, focused)) = paint_checkpoint.take() {
                    restore_editable_focus = self.restore_paint_checkpoint(
                        s,
                        accepted,
                        focused,
                        viewport_checkpoint.as_ref(),
                    )?;
                    let (previous, previous_widgets) = s.prepare_frame(&self.window, blended)?;
                    // Restore all owned output after a partial DOM write. A
                    // recovery repaint keeps the accepted revision, and a
                    // still-live brush retains its opaque session and capture.
                    s.publish()?;
                    s.paint_owned(self.window.device_pixel_ratio(), previous, false)?;
                    widgets = previous_widgets;
                    events.clear();
                    events.push(BrowserEvent::Error(error));
                } else {
                    return Err(error);
                }
            }
            if let Some(error) = s.presentation_error.take() {
                events.push(BrowserEvent::Error(error));
            }
            widgets.map(|widgets| (widgets, s.staged_widgets.take()))
        };
        let mut tooltip_stage = None;
        if paint_accepted {
            let prepared = self.tooltip_stage_for_paint().and_then(|stage| {
                if let Some(stage) = &stage
                    && let Err(error) = self.place_tree_tooltip(stage)
                {
                    stage.rollback()?;
                    return Err(error);
                }
                Ok(stage)
            });
            match prepared {
                Ok(stage) => {
                    tooltip_stage = stage;
                    if let Some(stage) = &tooltip_stage
                        && let Some(s) = self.state.borrow_mut().as_mut()
                    {
                        s.tree_tooltip = Some(stage.clone());
                    }
                }
                Err(error) => {
                    paint_accepted = false;
                    ordered_frame = None;
                    widget_frame = None;
                    if let Some((accepted, focused)) = paint_checkpoint.take() {
                        let mut state = self.state.borrow_mut();
                        let s = state
                            .as_mut()
                            .ok_or_else(|| ChartError("Tree tooltip chart stopped".into()))?;
                        restore_editable_focus = self.restore_paint_checkpoint(
                            s,
                            accepted,
                            focused,
                            viewport_checkpoint.as_ref(),
                        )?;
                        let (previous, _) = s.prepare_frame(&self.window, blended)?;
                        s.publish()?;
                        s.paint_owned(self.window.device_pixel_ratio(), previous, false)?;
                    }
                    events.clear();
                    events.push(BrowserEvent::Error(error));
                }
            }
        }
        if let Some(control) = restore_editable_focus.as_ref() {
            let _ = control.focus();
        }
        self.sync_editable_capture();
        let mut svg_stage = None;
        if paint_accepted {
            match self.place_tree_svg() {
                Ok(stage) => svg_stage = stage,
                Err(error) => {
                    paint_accepted = false;
                    ordered_frame = None;
                    widget_frame = None;
                    if let Some(tip) = &tooltip_stage {
                        tip.rollback()?;
                    }
                    if let Some((accepted, focused)) = paint_checkpoint.take() {
                        let mut state = self.state.borrow_mut();
                        let s = state
                            .as_mut()
                            .ok_or_else(|| ChartError("Tree SVG chart stopped".into()))?;
                        restore_editable_focus = self.restore_paint_checkpoint(
                            s,
                            accepted,
                            focused,
                            viewport_checkpoint.as_ref(),
                        )?;
                        let (previous, _) = s.prepare_frame(&self.window, blended)?;
                        s.publish()?;
                        s.paint_owned(self.window.device_pixel_ratio(), previous, false)?;
                    }
                    events.clear();
                    events.push(BrowserEvent::Error(error));
                }
            }
        }
        if let Some(stage) = &svg_stage {
            stage.adopt(tooltip_stage.as_ref());
            events.extend(stage.finish().into_iter().map(BrowserEvent::Error));
        }
        if ordered_mode {
            // Native input/effects may have stopped this chart during preparation.
            if !self.alive.get() {
                return Ok(());
            }
            if let Some(frame) = ordered_frame {
                let host = self.plot_widgets.borrow().clone().expect("ordered host");
                let mut adopt = || {
                    if let Some(stage) = &tooltip_stage {
                        stage.adopt();
                    }
                };
                let committed = if tooltip_stage.is_some() {
                    host.commit_ordered_with_adopt(frame, &mut adopt)
                } else {
                    host.commit_ordered(frame)
                };
                match committed {
                    Ok(errors) => events.extend(errors.into_iter().map(BrowserEvent::Error)),
                    Err(error) => {
                        paint_accepted = false;
                        if !self.alive.get() {
                            return Ok(());
                        }
                        if let Some((baseline, focused)) = &ordered_baseline {
                            let mut state = self.state.borrow_mut();
                            let s = state.as_mut().expect("active ordered chart");
                            restore_editable_focus = self.restore_paint_checkpoint(
                                s,
                                baseline.clone(),
                                focused.clone(),
                                viewport_checkpoint.as_ref(),
                            )?;
                            let (previous, _) = s.prepare_frame(&self.window, blended)?;
                            s.publish()?;
                            s.paint_owned(self.window.device_pixel_ratio(), previous, false)?;
                        }
                        events.clear();
                        events.push(BrowserEvent::Error(error));
                    }
                }
            }
            drop(widget_frame);
        } else if let Some((frame, stage)) = widget_frame {
            let host = self.plot_widgets.borrow().clone();
            if let Some(host) = host {
                let mut adopt = || {
                    if let Some(stage) = &tooltip_stage {
                        stage.adopt();
                    }
                };
                let committed = if tooltip_stage.is_some() {
                    host.publish_with_adopt(frame, stage, &mut adopt)
                } else {
                    host.publish(frame, stage)
                };
                match committed {
                    Ok(errors) => events.extend(errors.into_iter().map(BrowserEvent::Error)),
                    Err(error) => {
                        paint_accepted = false;
                        if !self.alive.get() {
                            return Ok(());
                        }
                        if (self.tree_canvas_bitmap().is_some()
                            || tooltip_stage.is_some()
                            || paint_checkpoint
                                .as_ref()
                                .is_some_and(|(s, _)| s.tree_canvas.is_some()))
                            && let Some((accepted, focused)) = paint_checkpoint.take()
                        {
                            let mut state = self.state.borrow_mut();
                            let s = state
                                .as_mut()
                                .ok_or_else(|| ChartError("browser chart stopped".into()))?;
                            restore_editable_focus = self.restore_paint_checkpoint(
                                s,
                                accepted,
                                focused,
                                viewport_checkpoint.as_ref(),
                            )?;
                            let (previous, _) = s.prepare_frame(&self.window, blended)?;
                            s.publish()?;
                            s.paint_owned(self.window.device_pixel_ratio(), previous, false)?;
                        }
                        events.push(BrowserEvent::Error(error));
                    }
                }
            }
        }
        if let Some(stage) = &tooltip_stage {
            if paint_accepted {
                if !stage.was_adopted() {
                    stage.adopt();
                }
                events.extend(
                    self.pump_tree_tooltip()
                        .into_iter()
                        .map(BrowserEvent::Error),
                );
            } else {
                stage.rollback()?;
            }
        }
        drop(_staged_lifetimes);
        if ordered_mode
            && let Some(host) = self.plot_widgets.borrow().clone()
            && let Err(error) = host.flush_cleanup()
        {
            events.push(BrowserEvent::Error(error));
        }
        if let Some(control) = restore_editable_focus {
            let _ = control.focus();
        }
        if forced_press_cancel || explicit_accepted && paint_accepted {
            self.tree_press_cancel()?;
        }
        self.finish_frame_events(events)
    }
    fn restore_paint_checkpoint(
        &self,
        state: &mut State,
        accepted: GestureState,
        focused: Option<String>,
        viewport: Option<&(u64, Option<DomainViewport>)>,
    ) -> Result<Option<HtmlElement>> {
        let focus = accepted.editable_focus.clone();
        state.restore_gesture(accepted);
        state.focused = focused;
        if let Some((generation, controller)) = viewport
            && self.viewport_generation.get() == *generation
        {
            if let (Some(binding), Some(controller)) =
                (self.viewport.borrow_mut().as_mut(), controller.as_ref())
            {
                binding.controller = controller.clone();
            }
            self.viewport_domains
                .set(controller.as_ref().map(DomainViewport::domains));
            *self.viewport_controller.borrow_mut() = controller.clone();
        }
        if !self.editable_contact_live(state) && !self.native_press_contact_live() {
            // An actual up/cancel already released by the browser cannot be
            // resurrected. A still-captured Tree mouse/pen contact can retain
            // its accepted gesture on a rejected candidate, but an Up-tail
            // paint lease and a Widget-only lease cannot revive that gesture.
            let selection = state.model.editable_brush().and_then(|b| b.selection());
            state.model.cancel_drag();
            if state.model.editable_brush().is_some() {
                state.model.set_brush_selection(selection)?;
            }
            state.editable_transport = None;
            state.freeze_motion();
            self.cancel_captures(&state.surface);
        }
        Ok(focus)
    }
}
impl State {
    fn transactional_geometry(&self, interaction: Option<&InteractionBinding>) -> bool {
        self.plot_widgets.is_some()
            || self.overlay_enabled(interaction)
            || interaction.is_some_and(|data| !data.anchors.is_empty())
    }
    fn overlay_enabled(&self, interaction: Option<&InteractionBinding>) -> bool {
        self.options.crosshair.is_some()
            || interaction.is_some_and(|b| b.polar.is_some() && b.polar_spoke.is_some())
    }
    fn motion_candidate(
        &self,
        scene: Scene,
        bindings: Option<motion::ContourBindings>,
        interaction: Option<&InteractionBinding>,
    ) -> Result<RunningMotion> {
        self.motion_candidate_with_blends(scene, bindings, interaction, MarkBlends::default())
    }
    fn motion_candidate_with_blends(
        &self,
        scene: Scene,
        bindings: Option<motion::ContourBindings>,
        interaction: Option<&InteractionBinding>,
        modes: MarkBlends,
    ) -> Result<RunningMotion> {
        let mut candidate = match &self.motion {
            Some(motion) => motion.clone(),
            None => RunningMotion::new(
                self.model.scene().clone(),
                &self.options.animation,
                self.contour_bindings.clone(),
                self.current_blends().for_scene(self.model.scene()),
            )?,
        };
        candidate.retarget_with_blends(scene, bindings, modes)?;
        if let Some(interaction) = interaction {
            interaction.validate_frame_anchors(candidate.scene())?;
        }
        // Both solvers paint their complete entering/exiting union at retarget.
        // Subsequent ticks only prune tracks; finish uses the separately admitted
        // target. Check the real union, including zero-opacity ghost geometry.
        crosshair::overlay_admission(candidate.scene(), self.overlay_enabled(interaction))?;
        if let Some(options) = &self.options.crosshair {
            crosshair::overlay_scene_admission(candidate.scene(), options)?;
        }
        Ok(candidate)
    }
    fn admit_animation_overlay(
        &self,
        scene: &Scene,
        bindings: Option<&motion::ContourBindings>,
        interaction: Option<&InteractionBinding>,
    ) -> Result<()> {
        if !self.reduced && self.transactional_geometry(interaction) {
            self.motion_candidate(scene.clone(), bindings.cloned(), interaction)?;
        }
        Ok(())
    }
    fn admit_interaction_scene(
        &self,
        scene: &Scene,
        binding: Option<&InteractionBinding>,
    ) -> Result<()> {
        self.admit_interaction_scene_with_contours(scene, binding, &motion::ContourBindings::new())
    }
    fn admit_interaction_scene_with_contours(
        &self,
        scene: &Scene,
        binding: Option<&InteractionBinding>,
        contours: &motion::ContourBindings,
    ) -> Result<()> {
        scene.validate()?;
        if let Some(binding) = binding {
            binding.validate(scene)?;
        }
        crosshair::overlay_admission(
            scene,
            self.options.crosshair.is_some()
                || binding.is_some_and(|b| b.polar.is_some() && b.polar_spoke.is_some()),
        )?;
        if let Some(options) = &self.options.crosshair {
            crosshair::overlay_scene_admission(scene, options)?;
        }
        for item in &scene.legend {
            legend_label(item, &self.options.legend_options)?;
        }
        self.admit_motion_scene(scene, contours)
    }
    fn admit_scene(&self, scene: &Scene) -> Result<()> {
        self.admit_scene_with_contours(scene, &motion::ContourBindings::new())
    }
    fn admit_scene_with_contours(
        &self,
        scene: &Scene,
        bindings: &motion::ContourBindings,
    ) -> Result<()> {
        scene.validate()?;
        crosshair::overlay_admission(
            scene,
            self.options.crosshair.is_some()
                || self
                    .model
                    .interaction_data()
                    .is_some_and(|b| b.polar.is_some() && b.polar_spoke.is_some()),
        )?;
        if let Some(options) = &self.options.crosshair {
            crosshair::overlay_scene_admission(scene, options)?;
        }
        for item in &scene.legend {
            legend_label(item, &self.options.legend_options)?;
        }
        self.admit_motion_scene(scene, bindings)
    }
    fn admit_motion_scene(&self, scene: &Scene, bindings: &motion::ContourBindings) -> Result<()> {
        match &self.options.animation {
            BrowserAnimation::Properties(options)
            | BrowserAnimation::PropertiesWithContours { options, .. }
                if !options.geometry.contour_morphs.is_empty() || !bindings.is_empty() =>
            {
                validate_motion_scene(scene, options, bindings)?;
            }
            _ => {}
        }
        Ok(())
    }
    fn gesture(&self) -> GestureState {
        GestureState {
            editable_focus: self
                .surface
                .owner_document()
                .and_then(|document| document.active_element())
                .filter(|element| self.surface.contains(Some(element)))
                .and_then(|element| {
                    element
                        .closest(".kartis-editable-brush-control")
                        .ok()
                        .flatten()
                })
                .and_then(|element| element.dyn_into::<HtmlElement>().ok()),
            options: self.options.clone(),
            revision: self.revision,
            last_size: self.last_size,
            presentation_error: self.presentation_error.clone(),
            editable_transport: self.editable_transport,
            model: self.model.clone(),
            motion: self.motion.clone(),
            last_time: self.last_time,
            pointer: self.pointer,
            keyboard: self.keyboard,
            contour_bindings: self.contour_bindings.clone(),
            blends: self.blends.clone(),
            plot_widgets: self.plot_widgets.clone(),
            staged_widgets: self.staged_widgets.clone(),
            tree_canvas: self.tree_canvas.clone(),
            tree_tooltip: self.tree_tooltip.clone(),
            tree_svg: self.tree_svg.clone(),
        }
    }
    fn restore_gesture(&mut self, state: GestureState) {
        self.options = state.options;
        self.revision = state.revision;
        self.last_size = state.last_size;
        self.presentation_error = state.presentation_error;
        self.editable_transport = state.editable_transport;
        self.model = state.model;
        self.motion = state.motion;
        self.last_time = state.last_time;
        self.pointer = state.pointer;
        self.keyboard = state.keyboard;
        self.contour_bindings = state.contour_bindings;
        self.blends = state.blends;
        self.plot_widgets = state.plot_widgets;
        self.staged_widgets = state.staged_widgets;
        self.tree_canvas = state.tree_canvas;
        self.tree_tooltip = state.tree_tooltip;
        self.tree_svg = state.tree_svg;
    }
    fn coordinates(&self, client: (f64, f64)) -> Result<(f64, f64)> {
        let r = self.surface.get_bounding_client_rect();
        crate::interaction::pointer_coordinates(
            self.model.scene(),
            client,
            (r.left(), r.top(), r.width(), r.height()),
        )
    }
    fn accept_scene(
        &mut self,
        scene: Scene,
        animate: bool,
        bindings: Option<motion::ContourBindings>,
    ) -> Result<Vec<SurfaceEvent>> {
        self.accept_interaction_scene(scene, animate, bindings, None)
    }
    fn accept_interaction_scene(
        &mut self,
        scene: Scene,
        animate: bool,
        bindings: Option<motion::ContourBindings>,
        interaction: Option<InteractionBinding>,
    ) -> Result<Vec<SurfaceEvent>> {
        let order = bindings.clone().unwrap_or_default();
        scene::ensure(
            !animate || self.current_blends().is_empty() && self.blends.is_empty(),
            "animation is unavailable with an active blend profile",
        )?;
        // Current metadata need not agree with new business values. Validate
        // the replacement binding only against its own candidate Scene.
        self.admit_interaction_scene_with_contours(&scene, interaction.as_ref(), &order)?;
        if animate && !self.reduced {
            // Recheck against the actual state after earlier queued commands.
            let next_motion =
                self.motion_candidate(scene.clone(), bindings.clone(), interaction.as_ref())?;
            let mut next_model = self.model.clone();
            let events = next_model.set_interaction_frame(
                &scene,
                interaction,
                next_motion.scene().clone(),
            )?;
            if self.motion.is_none() {
                self.last_time = None;
            }
            next_model.cancel_drag();
            self.motion = Some(next_motion);
            self.model = next_model;
            Ok(events)
        } else {
            let changes = self.model.set_scene_with_interaction(scene, interaction)?;
            self.tree_canvas = None;
            self.tree_svg = None;
            if !self.blends.is_empty() {
                self.blends = Rc::default();
            }
            self.contour_bindings = order;
            self.motion = None;
            self.last_time = None;
            Ok(changes)
        }
    }
    fn process(&mut self, input: Input, window: &Window) -> Result<Vec<SurfaceEvent>> {
        if self.tree_svg.is_some() {
            scene::ensure(
                !matches!(
                    &input,
                    Input::Scene(..)
                        | Input::InteractionScene(_)
                        | Input::Layout(_)
                        | Input::ContourScene(..)
                        | Input::BlendedAnimation(_)
                        | Input::TreeCanvas(_)
                        | Input::PlotWidgetLayout(_)
                        | Input::PlotWidgetComposition(_)
                ),
                "Tree SVG source changes require its unified owner",
            )?;
        }
        if self.tree_canvas.is_some() {
            let animated = match &input {
                Input::Scene(_, animate) => *animate,
                Input::InteractionScene(c) => c.2,
                Input::ContourScene(..) | Input::BlendedAnimation(_) => true,
                _ => false,
            };
            scene::ensure(!animated, "raw Tree Canvas does not support animation")?;
        }
        match input {
            Input::TreeTooltip(_) => Err(ChartError("Tree tooltip bundle was not unpacked".into())),
            Input::TreeCanvas(candidate) => self.process_tree_canvas(*candidate),
            Input::TreeSvg(stage) => self.process_tree_svg(stage),
            Input::EditableBrush(command) => self.process_editable_brush(command, window),
            Input::Pointer {
                kind,
                id,
                touch,
                client,
                brush,
                source_mesh,
            } => {
                if matches!(kind, PointerKind::Cancel) {
                    return self.model.apply(if touch {
                        SurfaceInput::TouchCancel { id }
                    } else {
                        SurfaceInput::Cancel { id }
                    });
                }
                let point = match self.coordinates(client) {
                    Ok(point) => point,
                    Err(_) if matches!(kind, PointerKind::Up) => {
                        return self.model.apply(if touch {
                            SurfaceInput::TouchCancel { id }
                        } else {
                            SurfaceInput::Cancel { id }
                        });
                    }
                    Err(error) => return Err(error),
                };
                let input = if touch {
                    match kind {
                        PointerKind::Down => SurfaceInput::TouchStart { id, point },
                        PointerKind::Move => SurfaceInput::TouchMove { id, point },
                        PointerKind::Up => SurfaceInput::TouchEnd { id, point },
                        PointerKind::Cancel => SurfaceInput::TouchCancel { id },
                    }
                } else {
                    match kind {
                        PointerKind::Down => SurfaceInput::Down { id, point, brush },
                        PointerKind::Move => SurfaceInput::Move { id, point },
                        PointerKind::Up => SurfaceInput::Up { id, point },
                        PointerKind::Cancel => SurfaceInput::Cancel { id },
                    }
                };
                let changes = if source_mesh {
                    self.model.apply_without_source_pick(input)?
                } else {
                    self.model.apply(input)?
                };
                self.pointer = Some(point);
                self.keyboard = false;
                if matches!(kind, PointerKind::Down) {
                    self.freeze_motion();
                }
                Ok(changes)
            }
            Input::Wheel {
                client,
                delta,
                mode,
            } => {
                let point = self.coordinates(client)?;
                let delta = delta
                    * match mode {
                        1 => 16.,
                        2 => self.surface.get_bounding_client_rect().height(),
                        _ => 1.,
                    };
                scene::ensure(delta.is_finite(), "nonfinite wheel delta")?;
                let changes = self.model.apply(SurfaceInput::Wheel { point, delta })?;
                self.pointer = Some(point);
                self.freeze_motion();
                Ok(changes)
            }
            Input::Action(action) => {
                let keyboard = matches!(
                    action,
                    SurfaceInput::Next
                        | SurfaceInput::Previous
                        | SurfaceInput::First
                        | SurfaceInput::Last
                        | SurfaceInput::Activate
                );
                if keyboard {
                    self.keyboard = true;
                    self.pointer = None;
                    let mut events = self.model.set_hovered(vec![])?;
                    events.extend(self.model.apply(action)?);
                    Ok(events)
                } else {
                    let was_dragging = self.model.dragging();
                    let changes = self.model.apply(action)?;
                    if matches!(action, SurfaceInput::Reset | SurfaceInput::Clear)
                        || matches!(
                            action,
                            SurfaceInput::Down { .. } | SurfaceInput::TouchStart { .. }
                        ) && !was_dragging
                            && self.model.dragging()
                        || matches!(action, SurfaceInput::Wheel { .. })
                            && self.options.interaction.zoom
                    {
                        self.freeze_motion();
                    }
                    Ok(changes)
                }
            }
            Input::Hidden(value) => self.model.set_hidden(value),
            Input::LegendToggle(activation) => {
                let target = activation.resolved_target();
                if activation.role != LegendRole::Series
                    || self.model.scene().legend_target(&activation.item.id) != Some(target.clone())
                {
                    return Ok(vec![]);
                }
                let LegendTarget::Series(series) = target else {
                    return Ok(vec![]);
                };
                let mut hidden = self.model.hidden().clone();
                if !hidden.remove(&series) {
                    hidden.insert(series);
                }
                self.model.set_hidden(hidden)
            }
            Input::Selected(value) => {
                if value
                    .as_ref()
                    .is_some_and(|id| self.model.logical_datum(id).is_none())
                {
                    Ok(vec![])
                } else {
                    self.model.set_selected(value)
                }
            }
            Input::Hover(value) => {
                // Strict plot hover comes only from the accepted ink resolver.
                // A control event queued before a profile change cannot replace it.
                if self.model.interaction_data().is_some_and(|binding| {
                    binding.picking_profile == crate::PickingProfile::PaintedGeometry
                }) {
                    return Ok(vec![]);
                }
                // The plot's geometry (including a miss) remains authoritative
                // until its own pointer leaves. A queued datum-button leave
                // must not clear a newer plot hit or its repick coordinates.
                if self.model.has_plot_hover() {
                    return Ok(vec![]);
                }
                let value = value.filter(|id| self.model.datum(id).is_some());
                if value.is_some() {
                    self.pointer = None;
                }
                self.model.set_hovered(value.into_iter().collect())
            }
            Input::Focus(_) | Input::PlotWidgetFocus { .. } | Input::LegendActivate(_) => {
                Ok(vec![])
            }
            Input::Activate(id) => {
                if self.model.logical_datum(&id).is_some() {
                    self.model.activate(&id)
                } else {
                    Ok(vec![])
                }
            }
            Input::Scene(scene, animate) => self.accept_scene(scene, animate, None),
            Input::InteractionScene(candidate) => {
                let (scene, binding, animate) = *candidate;
                self.accept_interaction_scene(scene, animate, None, Some(binding))
            }
            Input::Layout(candidate) => {
                let (candidate, modes) = *candidate;
                self.admit_interaction_scene(&candidate.scene, candidate.interaction.as_ref())?;
                modes.validate(&candidate.scene)?;
                let changes = self.model.set_layout(candidate)?;
                self.tree_canvas = None;
                self.blends = Rc::new(modes);
                self.motion = None;
                self.last_time = None;
                self.contour_bindings.clear();
                Ok(changes)
            }
            Input::PlotWidgetComposition(candidate) => {
                let (candidate, packet, stage) = *candidate;
                stage.validate()?;
                self.require_plot_widget_type(&packet)?;
                let (mut model, mut changes) = match candidate {
                    Some(candidate) => self.plot_widget_layout_candidate(candidate, &packet)?,
                    None => {
                        plot_widgets::prepare(&self.model, &packet, true)?;
                        (self.model.clone(), vec![])
                    }
                };
                changes.extend(model.set_widget_replacements(packet.replace_ink().clone())?);
                self.model = model;
                self.tree_canvas = None;
                self.plot_widgets = Some(Rc::new(packet));
                self.staged_widgets = Some(stage);
                self.blends = Rc::default();
                self.motion = None;
                self.last_time = None;
                self.contour_bindings.clear();
                Ok(changes)
            }
            Input::PlotWidgets(layout) => {
                self.require_plot_widget_type(&layout)?;
                plot_widgets::prepare(&self.model, &layout, true)?;
                self.plot_widgets = Some(Rc::new(layout));
                Ok(vec![])
            }
            Input::PlotWidgetLayout(candidate) => {
                let (candidate, layout) = *candidate;
                let (model, changes) = self.plot_widget_layout_candidate(candidate, &layout)?;
                self.model = model;
                self.tree_canvas = None;
                self.plot_widgets = Some(Rc::new(layout));
                self.blends = Rc::default();
                self.motion = None;
                self.last_time = None;
                self.contour_bindings.clear();
                Ok(changes)
            }
            Input::ContourScene(scene, bindings) => self.accept_scene(scene, true, Some(bindings)),
            Input::BlendedAnimation(request) => self.accept_blended_animation(window, *request),
            Input::Resize | Input::ViewportRefresh | Input::ViewportRequest(_) => Ok(vec![]),
            Input::ReducedMotion(value) => {
                self.reduced = value;
                if value && let Some(mut motion) = self.motion.take() {
                    motion.finish();
                    self.last_time = None;
                    let changes = self
                        .model
                        .set_scene_preserving_interaction(motion.scene().clone())?;
                    self.contour_bindings = motion.paint_bindings();
                    Ok(changes)
                } else {
                    Ok(vec![])
                }
            }
        }
    }
    fn publish(&self) -> Result<()> {
        let business = self
            .model
            .interaction_data()
            .map(|_| self.model.controls_scene());
        let source = business.as_ref().unwrap_or_else(|| self.model.scene());
        if self.source.with(|s| s != source) {
            self.source.revise(|s| *s = source.clone());
        }
        let data_pointer = self
            .model
            .interaction_data()
            .is_none_or(|binding| binding.picking_profile == crate::PickingProfile::LegacyGeometry);
        if self.data_pointer.get() != data_pointer {
            self.data_pointer.revise(|old| *old = data_pointer);
        }
        let labels = self.model.data_control_labels();
        if self.data_labels.with(|old| old != &labels) {
            self.data_labels.revise(|old| *old = labels);
        }
        if self.hidden.with(|s| s != self.model.hidden()) {
            self.hidden.revise(|s| *s = self.model.hidden().clone());
        }
        let selected = self.model.selected().map(str::to_owned);
        if self.selected.get() != selected {
            self.selected.revise(|s| *s = selected);
        }
        self.web.app().pump().map_err(host)?;
        Ok(())
    }
    fn tooltip_ids(&self, model: &SurfaceModel) -> Vec<String> {
        let ids = if !model.hovered().is_empty() {
            model.hovered().to_vec()
        } else if let Some(id) = self.focused.as_ref().filter(|id| model.datum(id).is_some()) {
            vec![id.clone()]
        } else if self.keyboard {
            model.selected().into_iter().map(str::to_owned).collect()
        } else {
            vec![]
        };
        model.interaction_ids(&ids)
    }
    fn paint_scene_for(
        &self,
        model: &SurfaceModel,
    ) -> Result<(Scene, Vec<String>, Option<ActiveInteraction>)> {
        let mut scene = if self.source_ordered() {
            model.scene().clone()
        } else {
            let projected = match &self.plot_widgets {
                Some(packet) => packet.project_scene(model.scene()),
                None => model.scene().without_mark_ink(model.widget_replacements()),
            };
            projected.without_series(model.hidden())
        };
        let tooltip_ids = self.tooltip_ids(model);
        let interaction = model.active_interaction(&tooltip_ids);
        let active: Vec<_> = if tooltip_ids.is_empty() {
            model.selected().into_iter().map(str::to_owned).collect()
        } else {
            tooltip_ids.clone()
        };
        if !active.is_empty() && self.options.highlight == BrowserHighlight::Series {
            let active_ids: BTreeSet<_> = active.iter().collect();
            let series: BTreeSet<_> = active
                .iter()
                .filter_map(|id| model.datum(id).map(|m| m.series.clone()))
                .collect();
            for mark in &mut scene.marks {
                if !mark.series.is_empty() && !series.contains(&mark.series) {
                    mark.opacity *= 0.35;
                }
                if active_ids.contains(&mark.id)
                    && !matches!(mark.geometry, Geometry::Circle { radius: 0., .. })
                {
                    mark.stroke = Some(glory::render::GraphicStroke {
                        color: Color(28, 43, 65, 255),
                        width: 2.,
                        cap: glory::render::StrokeCap::Round,
                    });
                }
            }
        }
        let overlays = model.interaction_overlays(&tooltip_ids, self.options.crosshair.as_ref())?;
        let first_overlay = scene.marks.len();
        if !scene.draw_order.is_empty() {
            scene
                .draw_order
                .extend((first_overlay..first_overlay + overlays.len()).map(DrawItem::Mark));
        }
        scene.marks.extend(overlays);
        scene.validate()?;
        Ok((scene, tooltip_ids, interaction))
    }
    fn paint_tooltip(
        &mut self,
        scene: &Scene,
        tooltip_ids: &[String],
        interaction: Option<&ActiveInteraction>,
    ) -> Result<()> {
        let r = self.surface.get_bounding_client_rect();
        let font = scene.typography.tooltip_font(
            &self.options.tooltip_options.text.font,
            self.options.tooltip_options.text.weight,
        );
        css(&self.tooltip, "font-family", &font.css_family())?;
        css(&self.tooltip, "font-weight", &font.weight.to_string())?;
        css(&self.tooltip, "display", "none")?;
        if self.options.tooltip
            && self.tree_tooltip.as_ref().is_none_or(|tip| !tip.managed())
            && !tooltip_ids.is_empty()
        {
            let marks: Vec<_> = tooltip_ids
                .iter()
                .filter_map(|id| self.model.datum(id))
                .collect();
            let text = match if let Some(active) = interaction {
                interaction_tooltip_text(&self.options.tooltip_options, active)
            } else {
                tooltip_text(&self.options.tooltip_options, &marks)
            } {
                Ok(text) => text,
                Err(error) => {
                    self.presentation_error = Some(error);
                    String::new()
                }
            };
            self.tooltip.set_text_content(Some(&text));
            if !text.is_empty() {
                css(&self.tooltip, "display", "block")?;
            }
            let pointer = self
                .pointer
                .or_else(|| {
                    interaction.as_ref().and_then(|active| {
                        self.model
                            .paint_transform()
                            .project((active.anchor[0], active.anchor[1]))
                            .ok()
                    })
                })
                .unwrap_or((scene.width / 2., scene.height / 2.));
            let surface = LayoutRect {
                x: 0.,
                y: 0.,
                width: r.width(),
                height: r.height(),
            };
            let mut layout =
                self.options
                    .tooltip_layout
                    .clone()
                    .unwrap_or_else(|| TooltipLayoutOptions {
                        viewport: surface,
                        preferred: vec![
                            TooltipAnchor::BottomRight,
                            TooltipAnchor::TopRight,
                            TooltipAnchor::BottomLeft,
                            TooltipAnchor::TopLeft,
                            TooltipAnchor::Right,
                            TooltipAnchor::Left,
                            TooltipAnchor::Bottom,
                            TooltipAnchor::Top,
                        ],
                        ..Default::default()
                    });
            let left = layout.viewport.x.max(0.);
            let top = layout.viewport.y.max(0.);
            let right = (layout.viewport.x + layout.viewport.width).min(surface.width);
            let bottom = (layout.viewport.y + layout.viewport.height).min(surface.height);
            if right - left <= layout.viewport_padding * 2.
                || bottom - top <= layout.viewport_padding * 2.
            {
                css(&self.tooltip, "display", "none")?;
            } else if !text.is_empty() {
                layout.viewport = LayoutRect {
                    x: left,
                    y: top,
                    width: right - left,
                    height: bottom - top,
                };
                css(
                    &self.tooltip,
                    "max-width",
                    &format!(
                        "{}px",
                        (layout.viewport.width - layout.viewport_padding * 2.).min(340.)
                    ),
                )?;
                css(
                    &self.tooltip,
                    "max-height",
                    &format!(
                        "{}px",
                        layout.viewport.height - layout.viewport_padding * 2.
                    ),
                )?;
                let measured = self.tooltip.get_bounding_client_rect();
                let placement = place_tooltip(
                    (
                        pointer.0 / scene.width * r.width(),
                        pointer.1 / scene.height * r.height(),
                    ),
                    TextSize {
                        width: measured.width(),
                        height: measured.height(),
                    },
                    &layout,
                )?;
                css(
                    &self.tooltip,
                    "left",
                    &format!("{}px", placement.rect.x + self.surface.offset_left() as f64),
                )?;
                css(
                    &self.tooltip,
                    "top",
                    &format!("{}px", placement.rect.y + self.surface.offset_top() as f64),
                )?;
            }
        } else {
            css(&self.tooltip, "display", "none")?;
        }
        Ok(())
    }
    fn prepare_frame(
        &self,
        window: &Window,
        blended: bool,
    ) -> Result<(blends::Frame, Option<plot_widgets::Prepared>)> {
        self.prepare_frame_for(&self.model, self.current_blends(), window, blended)
    }
    fn prepare_frame_for(
        &self,
        model: &SurfaceModel,
        profile: &MarkBlends,
        window: &Window,
        blended: bool,
    ) -> Result<(blends::Frame, Option<plot_widgets::Prepared>)> {
        if let Some(stage) = &self.staged_widgets {
            stage.validate()?;
        }
        let (scene, tooltip_ids, interaction) = self.paint_scene_for(model)?;
        let widgets = self.prepare_widgets_for(model)?;
        let modes = profile.for_scene(&scene);
        let view = model.paint_transform();
        let canvas = match &self.canvas {
            // A final Normal frame of a blend transition still participates in
            // the same publication transaction and must prepare its backing
            // store and pattern tiles before publishing.
            Some(canvas) if blended || !modes.is_empty() => Some(render::prepare_canvas(
                &scene,
                canvas,
                blends::ratio(window.device_pixel_ratio(), &scene),
                view,
                &modes,
            )?),
            None if !modes.is_empty() && !self.source_ordered() => {
                blends::admit(BrowserRenderer::Svg, window, &scene, &modes, view)?;
                None
            }
            _ => None,
        };
        Ok((
            blends::Frame {
                scene,
                tooltip_ids,
                interaction,
                modes,
                canvas,
            },
            widgets,
        ))
    }
    fn paint(&mut self, dpr: f64, prepared: blends::Frame) -> Result<()> {
        self.paint_owned(dpr, prepared, true)
    }
    fn paint_owned(
        &mut self,
        dpr: f64,
        prepared: blends::Frame,
        advance_revision: bool,
    ) -> Result<()> {
        if advance_revision {
            self.validate_tree_canvas_paint(dpr)?;
        }
        let blends::Frame {
            scene,
            tooltip_ids,
            interaction,
            modes,
            canvas: prepared_canvas,
        } = prepared;
        if self.surface.get_attribute("aria-label").as_deref() != Some(scene.title.as_str()) {
            self.surface
                .set_attribute("aria-label", &scene.title)
                .map_err(host)?;
        }
        if self.tree_svg.is_some() || self.source_ordered() {
            // The registered source-order plane tree is prepared and committed
            // outside State borrows, together with its real Widget scopes.
        } else if let Some(canvas) = &self.canvas {
            if let Some(raw) = &self.tree_canvas {
                raw.blit(canvas)?;
            } else if let Some(prepared) = prepared_canvas {
                prepared.paint(&scene, &modes)?;
            } else {
                render::draw_canvas_with_transform(
                    &scene,
                    canvas,
                    blends::ratio(dpr, &scene),
                    self.model.paint_transform(),
                )?;
            }
            canvas.style().set_property("width", "100%").map_err(host)?;
            canvas
                .style()
                .set_property("height", "100%")
                .map_err(host)?;
            canvas
                .style()
                .set_property("display", "block")
                .map_err(host)?;
        } else {
            if self.svg.is_none() {
                let document = self
                    .surface
                    .owner_document()
                    .ok_or_else(|| ChartError("SVG surface has no document".into()))?;
                let mut svg = svg::RetainedSvg::new(&document)?;
                if self.tree_labels_passive {
                    svg.make_labels_passive();
                }
                self.svg = Some(svg);
            }
            if modes.is_empty() {
                self.svg.as_mut().unwrap().paint(
                    &self.surface,
                    &scene,
                    self.model.paint_transform(),
                )?;
            } else {
                self.svg.as_mut().unwrap().paint_with_blends(
                    &self.surface,
                    &scene,
                    self.model.paint_transform(),
                    &modes,
                )?;
            }
        }
        let r = self.surface.get_bounding_client_rect();
        self.paint_tooltip(&scene, &tooltip_ids, interaction.as_ref())?;
        if let Some(b) = self
            .model
            .transformed_brush()?
            .filter(|_| self.model.editable_brush().is_none())
            .map(|b| b.clipped(scene.width, scene.height))
            .transpose()?
            .flatten()
        {
            css(&self.brush, "display", "block")?;
            for (name, value) in [
                ("left", b.x0 / scene.width * r.width()),
                ("top", b.y0 / scene.height * r.height()),
                ("width", (b.x1 - b.x0) / scene.width * r.width()),
                ("height", (b.y1 - b.y0) / scene.height * r.height()),
            ] {
                css(&self.brush, name, &format!("{value}px"))?;
            }
        } else {
            css(&self.brush, "display", "none")?;
        }
        if let Some(host) = &self.editable_host {
            host.publish(&self.surface, self.model.editable_brush(), &scene)?;
        }
        let role = if self.tree_svg.is_some()
            || self.source_ordered()
            || self.model.editable_brush().is_some()
        {
            "group"
        } else {
            "img"
        };
        if self.surface.get_attribute("role").as_deref() != Some(role) {
            self.surface.set_attribute("role", role).map_err(host)?;
        }
        self.revision = self
            .revision
            .checked_add(u64::from(advance_revision))
            .ok_or_else(|| ChartError("paint revision exhausted".into()))?;
        self.surface
            .set_attribute("data-revision", &self.revision.to_string())
            .map_err(host)?;
        let renderer = if self.canvas.is_some()
            || (self.source_ordered() && self.options.renderer == BrowserRenderer::Canvas)
        {
            "canvas"
        } else {
            "svg"
        };
        if self.surface.get_attribute("data-renderer").as_deref() != Some(renderer) {
            self.surface
                .set_attribute("data-renderer", renderer)
                .map_err(host)?;
        }
        Ok(())
    }
}
