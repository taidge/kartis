//! Owner-scoped Glory charts. Interactive legends filter a computed scene;
//! applications that need axis reflow accept the proposal and recompute layout.
//! Legacy pointer/focus interaction belongs to accessible data buttons. The
//! explicit chart_layout entry binds finite plot coordinates to painted ink.
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use glory::render::{
    Action, Event, EventKind, FocusMode, GraphicProps, MAX_GRAPHIC_COMMANDS, Navigation, NodeId,
    Orientation, PointerPhase, Semantics,
};
use glory::style::{
    Display, Justify, Length, Overflow, PointerEvents, Position, Style, WhiteSpace,
};
use glory::{Bond, Cage, Scope, Vary, Widget};

use crate::presentation::{
    LegendActivation, LegendOptions, LegendRenderContext, PresentationText, TooltipOptions,
    TooltipRenderContext, legend_control_label, legend_item_style, legend_label, legend_style,
    legend_symbol_style, tooltip_style, tooltip_text,
};
use crate::presentation_layout::*;
use crate::scene::*;
use crate::{
    SceneInteraction, SceneWidgetCompositionContext, SceneWidgetsComposition, SurfacePhysicalHit,
    ZoomTransform,
};
pub(crate) mod plot_widgets;
use plot_widgets::composition::{CompositionManager, CompositionStage};
pub use plot_widgets::{
    MAX_PLOT_WIDGET_SLOTS, PlotWidgetCompositionContext, PlotWidgetContext, PlotWidgetFrame,
    PlotWidgetPlacement, PlotWidgetSlot, PlotWidgetsComposition, PlotWidgetsData,
    PlotWidgetsLayout, TypedPlotWidgetContext, TypedPlotWidgetFrame,
};
use plot_widgets::{
    PlotWidgetsInput, PlotWidgetsPacket, PlotWidgetsSource, PlotWidgetsState, PreparedSlot,
};
pub(crate) mod plot_pointer;
mod semantic_labels;
use semantic_labels::{
    SemanticLabelsFrame, SemanticLabelsInput, SemanticLabelsState, ordinary_container,
    ordinary_content, ordinary_text_style, rectangular_clip, relative_to_clip,
};

type CallbackFn<T> = Box<dyn FnMut(T)>;
type RenderFn<A, R> = Box<dyn FnMut(A) -> Result<R>>;
struct Renderer<A, R> {
    callback: Rc<RefCell<Option<RenderFn<A, R>>>>,
    alive: Rc<Cell<bool>>,
}
impl<A, R> Clone for Renderer<A, R> {
    fn clone(&self) -> Self {
        Self {
            callback: self.callback.clone(),
            alive: self.alive.clone(),
        }
    }
}
impl<A, R> Renderer<A, R> {
    fn new(callback: Option<RenderFn<A, R>>) -> Self {
        Self {
            callback: Rc::new(RefCell::new(callback)),
            alive: Rc::new(Cell::new(true)),
        }
    }
    fn configured(&self) -> bool {
        self.callback.borrow().is_some()
    }
    fn call(&self, arg: A) -> Result<R> {
        ensure(self.alive.get(), "presentation owner retired")?;
        let mut callback = self
            .callback
            .borrow_mut()
            .take()
            .ok_or_else(|| ChartError("missing or reentrant presentation renderer".into()))?;
        let result = callback(arg);
        if self.alive.get() {
            *self.callback.borrow_mut() = Some(callback);
        }
        ensure(self.alive.get(), "presentation owner retired")?;
        result
    }
    fn clear(&self) {
        self.alive.set(false);
        self.callback.borrow_mut().take();
    }
}
type WidgetRenderer<A> = Renderer<A, Box<dyn Widget>>;
#[derive(Clone, Debug, Default, PartialEq)]
struct PresentationLayout {
    legend: BTreeMap<String, LayoutRect>,
    labels: BTreeMap<usize, Option<LayoutRect>>,
}
struct Callback<T> {
    value: Rc<RefCell<Option<CallbackFn<T>>>>,
    alive: Rc<Cell<bool>>,
}
impl<T> Clone for Callback<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            alive: self.alive.clone(),
        }
    }
}
impl<T> Callback<T> {
    fn new(value: Option<CallbackFn<T>>) -> Self {
        Self {
            value: Rc::new(RefCell::new(value)),
            alive: Rc::new(Cell::new(true)),
        }
    }
    fn clear(&self) {
        self.alive.set(false);
        self.value.borrow_mut().take();
    }
    fn emit(&self, value: T) {
        if !self.alive.get() {
            return;
        }
        let callback = self.value.borrow_mut().take();
        if let Some(mut callback) = callback {
            callback(value);
            if self.alive.get() {
                *self.value.borrow_mut() = Some(callback);
            }
        }
    }
}
/// A fixed-logical-size Glory scene Widget. Recompute the scene on host resize.
/// Legends filter marks, owned labels and data controls without rescaling axes.
/// Shared labels remain visible. Datum buttons supply pointer,
/// keyboard and focus interaction; decorative plot paths do not intercept hits.
/// Canvas rendering uses the explicit renderer adapter.
/// Semantic dashed strokes pack complete runs into Graphics. A single run must
/// fit 256 commands; expanded drawings have a 200,000-command budget. Partial-
/// alpha stroke colors which require several ink Graphics are rejected, since
/// that split cannot preserve one stroke's alpha union. Mark opacity is applied
/// once to their common wrapper. Arbitrary masks and fill paints require the
/// external SVG/Canvas or raster backend. Rectangular label clips use real
/// overflow-hidden containers without changing their source text or placement.
pub struct Chart {
    #[cfg(feature = "raster")]
    outlined: Option<(
        crate::render::raster::FontRegistry,
        crate::text_outline::OutlineLabelOptions,
    )>,
    #[cfg(feature = "raster")]
    painted: Option<(
        crate::render::raster::FontRegistry,
        crate::text_outline::PaintedLabelOptions,
    )>,
    #[cfg(feature = "raster")]
    pub(crate) raster: Option<crate::raster_widget::RasterInput>,
    #[cfg(feature = "raster")]
    pub(crate) raster_source: Option<crate::raster_widget::ImageSource>,
    scene: Vary<Scene>,
    bound_layout: Option<Vary<SceneInteraction>>,
    data_control_pointer: Vary<bool>,
    physical_hover: Option<CallbackFn<Option<SurfacePhysicalHit>>>,
    plot_widgets: Option<PlotWidgetsInput>,
    composition: Option<CompositionConfig>,
    hidden: Option<Vary<BTreeSet<String>>>,
    default_hidden: BTreeSet<String>,
    selected: Option<Vary<Option<String>>>,
    hovered: Option<Vary<Option<String>>>,
    tooltip_marks: Option<Vary<Vec<String>>>,
    data_labels: Option<Vary<BTreeMap<String, String>>>,
    default_selected: Option<String>,
    selection: Option<CallbackFn<Mark>>,
    selection_change: Option<CallbackFn<Option<Mark>>>,
    hidden_change: Option<CallbackFn<BTreeSet<String>>>,
    legend_activate: Option<CallbackFn<LegendActivation>>,
    legend_toggle: Option<CallbackFn<LegendActivation>>,
    hover: Option<CallbackFn<Option<Mark>>>,
    focus: Option<CallbackFn<Option<Mark>>>,
    navigate: Option<CallbackFn<Mark>>,
    rejection: Option<CallbackFn<ChartError>>,
    data_controls: bool,
    basic_data_controls: bool,
    legend: bool,
    tooltip: bool,
    plot: bool,
    highlight: bool,
    legend_options: LegendOptions,
    tooltip_options: TooltipOptions,
    legend_layout: Option<LegendLayoutOptions>,
    tooltip_layout: Option<TooltipLayoutOptions>,
    label_collision: Option<LabelCollisionOptions>,
    label_priorities: BTreeMap<usize, i32>,
    measurer: Option<RenderFn<(String, PresentationText), TextSize>>,
    semantic_labels: Option<SemanticLabelsInput>,
    tooltip_formatter: Option<RenderFn<Vec<Mark>, String>>,
    legend_renderer: Option<RenderFn<LegendRenderContext, Box<dyn Widget>>>,
    symbol_renderer: Option<RenderFn<LegendRenderContext, Box<dyn Widget>>>,
    tooltip_renderer: Option<RenderFn<TooltipRenderContext, Box<dyn Widget>>>,
}
pub fn chart(scene: impl Into<Vary<Scene>>) -> Chart {
    Chart {
        #[cfg(feature = "raster")]
        outlined: None,
        #[cfg(feature = "raster")]
        painted: None,
        #[cfg(feature = "raster")]
        raster: None,
        #[cfg(feature = "raster")]
        raster_source: None,
        scene: scene.into(),
        bound_layout: None,
        data_control_pointer: Vary::Value(true),
        physical_hover: None,
        plot_widgets: None,
        composition: None,
        hidden: None,
        default_hidden: BTreeSet::new(),
        selected: None,
        hovered: None,
        tooltip_marks: None,
        data_labels: None,
        default_selected: None,
        selection: None,
        selection_change: None,
        hidden_change: None,
        legend_activate: None,
        legend_toggle: None,
        hover: None,
        focus: None,
        navigate: None,
        rejection: None,
        data_controls: true,
        basic_data_controls: false,
        legend: true,
        tooltip: true,
        plot: true,
        highlight: true,
        legend_options: LegendOptions::default(),
        tooltip_options: TooltipOptions::default(),
        legend_layout: None,
        tooltip_layout: None,
        label_collision: None,
        label_priorities: BTreeMap::new(),
        measurer: None,
        semantic_labels: None,
        tooltip_formatter: None,
        legend_renderer: None,
        symbol_renderer: None,
        tooltip_renderer: None,
    }
}
/// Atomically bind a strict painted-geometry layout to a semantic Graphic chart
/// or an optional CPU image chart. One site remains one keyboard control;
/// parts retain their physical ink identity.
/// This opt-in requires a host supporting finite PlotPointer observations.
pub fn chart_layout(layout: impl Into<Vary<SceneInteraction>>) -> Chart {
    // This placeholder is never read or painted on the bound-layout path.
    let mut chart = chart(
        crate::ChartConfig::default()
            .scene()
            .expect("the default chart configuration is valid"),
    );
    chart.bound_layout = Some(layout.into());
    chart
}
impl Chart {
    /// Control logical data-button pointer behavior independently of keyboard
    /// and focus. False never makes button bounds physical plot ink.
    pub fn data_control_pointer(mut self, enabled: impl Into<Vary<bool>>) -> Self {
        self.data_control_pointer = enabled.into();
        self
    }
    /// Physical-part observations from the current accepted strict layout.
    /// Original outlier values are owned source_data, never live Mark scalars.
    pub fn on_physical_hover(
        mut self,
        callback: impl FnMut(Option<SurfacePhysicalHit>) + 'static,
    ) -> Self {
        self.physical_hover = Some(Box::new(callback));
        self
    }

    /// Render selected labels as true shaped glyph outlines. This opt-in uses
    /// application font bytes and retains the original safe semantic text.
    #[cfg(feature = "raster")]
    pub fn outline_labels(
        mut self,
        fonts: crate::render::raster::FontRegistry,
        options: crate::text_outline::OutlineLabelOptions,
    ) -> Self {
        self.outlined = Some((fonts, options));
        self.painted = None;
        self
    }
    /// Paint shaped font layers with their authored colors. Registered fonts
    /// and an explicit color-glyph profile determine palette selection. The
    /// original label remains available as safe semantic text.
    #[cfg(feature = "raster")]
    pub fn painted_labels(
        mut self,
        fonts: crate::render::raster::FontRegistry,
        options: crate::text_outline::PaintedLabelOptions,
    ) -> Self {
        self.painted = Some((fonts, options));
        self.outlined = None;
        self
    }
    fn accepts_rotated_labels(&self) -> bool {
        #[cfg(feature = "raster")]
        {
            self.outlined.is_some() || self.painted.is_some()
        }
        #[cfg(not(feature = "raster"))]
        {
            false
        }
    }
    /// All automatic placement uses this explicit application measurement.
    pub fn text_measurer(
        mut self,
        mut measure: impl FnMut(&str, &PresentationText) -> Result<TextSize> + 'static,
    ) -> Self {
        self.measurer = Some(Box::new(move |(text, style)| measure(&text, &style)));
        self
    }
    /// Position ordinary Text using actual host line metrics. The profile's
    /// resource revision invalidates cached measurements without replacing nodes.
    /// Labels and collision placement publish together only after all metrics
    /// have been validated against the current text/font/profile inputs.
    pub fn semantic_text_measurer(
        mut self,
        profile: impl Into<Vary<crate::SemanticTextProfile>>,
        mut measure: impl FnMut(&crate::SemanticTextRequest) -> Result<crate::SemanticTextMeasurement>
        + 'static,
    ) -> Self {
        self.semantic_labels = Some(SemanticLabelsInput {
            profile: profile.into(),
            measure: Box::new(move |request| measure(&request)),
        });
        self
    }
    /// Measure with the exact family list and weight used for this accepted
    /// scene's label, legend, or tooltip. Size and color remain in `style`.
    /// The last measurement builder wins.
    pub fn font_text_measurer(
        self,
        mut measure: impl FnMut(&str, &PresentationText, &crate::ResolvedFont) -> Result<TextSize>
        + 'static,
    ) -> Self {
        self.text_measurer(move |text, style| {
            let font = style.font.resolve(&crate::ResolvedFont::default());
            measure(text, style, &font)
        })
    }
    pub fn legend_layout(mut self, options: LegendLayoutOptions) -> Self {
        self.legend_layout = Some(options);
        self
    }
    pub fn tooltip_layout(mut self, options: TooltipLayoutOptions) -> Self {
        self.tooltip_layout = Some(options);
        self
    }
    pub fn label_collision(mut self, options: LabelCollisionOptions) -> Self {
        self.label_collision = Some(options);
        self
    }
    /// Larger values win label collisions; equal priorities preserve scene order.
    pub fn label_priorities(mut self, priorities: BTreeMap<usize, i32>) -> Self {
        self.label_priorities = priorities;
        self
    }
    pub fn tooltip_formatter(
        mut self,
        mut format: impl FnMut(&[Mark]) -> Result<String> + 'static,
    ) -> Self {
        self.tooltip_formatter = Some(Box::new(move |marks| format(&marks)));
        self
    }
    pub fn legend_item_renderer<W: Widget>(
        mut self,
        mut render: impl FnMut(LegendRenderContext) -> W + 'static,
    ) -> Self {
        self.legend_renderer = Some(Box::new(move |ctx| Ok(Box::new(render(ctx)))));
        self
    }
    pub fn legend_symbol_renderer<W: Widget>(
        mut self,
        mut render: impl FnMut(LegendRenderContext) -> W + 'static,
    ) -> Self {
        self.symbol_renderer = Some(Box::new(move |ctx| Ok(Box::new(render(ctx)))));
        self
    }
    pub fn tooltip_renderer<W: Widget>(
        mut self,
        mut render: impl FnMut(TooltipRenderContext) -> W + 'static,
    ) -> Self {
        self.tooltip_renderer = Some(Box::new(move |ctx| Ok(Box::new(render(ctx)))));
        self
    }
    /// Place real Glory Widgets in explicit, caller-measured slots above the
    /// semantic plot. A slot's datum ID and revision retain its child across
    /// data/geometry changes. A failed child build preserves the prior child
    /// and is reported through the chart's normal rejection handler.
    /// Returning to the prior accepted revision keeps that child. Use a
    /// different revision to retry. Side effects the factory performs outside
    /// the child scope belong to the caller.
    ///
    /// Slot input belongs to the child Widget. Hidden or fully clipped slots
    /// retain state, and partially clipped slots are visible but inert.
    /// Use inline child content. A child that opens a host top-layer popup
    /// uses Glory's separate popup placement and input policy.
    /// Scene and slot sources are validated together; remove a slot when its
    /// datum is removed. Requires a semantic plot or a raster chart image.
    /// Raster view transforms apply to slot rectangles independently of DPR.
    /// The optional plot rectangle stays in output logical coordinates. Live
    /// child Widgets are separate from exported Scene PNG pixels.
    pub fn plot_widgets<W: Widget>(
        mut self,
        layout: impl Into<Vary<PlotWidgetsLayout>>,
        factory: impl FnMut(PlotWidgetContext) -> Result<W> + 'static,
    ) -> Self {
        self.plot_widgets = Some(PlotWidgetsInput {
            source: plot_widgets::PlotWidgetsSource::plain(layout.into()),
            factory: plot_widgets::plain_factory(factory),
        });
        self
    }

    /// Mount typed Widgets whose source data and geometry are accepted as one
    /// packet. Data keys must exactly match slot mark IDs. Changing metadata
    /// updates the live frame without recreating the child or changing its key.
    /// Rejected Scene/slot/metadata proposals preserve the preceding accepted
    /// frame. A child factory/build error still follows `plot_widgets`' local
    /// revision retention policy; it does not roll back the chart's whole frame.
    pub fn plot_widgets_with_data<T: Clone + PartialEq + 'static, W: Widget>(
        mut self,
        source: impl Into<Vary<PlotWidgetsData<T>>>,
        factory: impl FnMut(TypedPlotWidgetContext<T>) -> Result<W> + 'static,
    ) -> Self {
        self.plot_widgets = Some(PlotWidgetsInput {
            source: plot_widgets::PlotWidgetsSource::typed(source.into()),
            factory: plot_widgets::typed_factory(factory),
        });
        self
    }

    /// Finish a chart with transactional live Widget composition. New or revised
    /// children are built hidden before paint publication; any factory, build,
    /// placement or publisher error rejects the whole candidate. Original Scene
    /// data and labels remain; only `replace_ink` mark bodies and physical picking
    /// are suppressed. Candidate-build contexts are tentative until commit.
    pub fn plot_widgets_composed<T: Clone + PartialEq + 'static, W: Widget>(
        mut self,
        source: impl Into<Vary<PlotWidgetsComposition<T>>>,
        factory: impl FnMut(PlotWidgetCompositionContext<T>) -> Result<W> + 'static,
    ) -> impl Widget {
        self.plot_widgets = None;
        ComposedChart {
            chart: Some(self),
            source: PlotWidgetsSource::composition(source.into()),
            factory: Some(plot_widgets::composition_factory(factory)),
        }
    }

    /// Compose live Glory children at real Scene mark and label targets.
    /// Slot keys retain child identity independently of label ordinals. All
    /// factories, placement and paint changes share one transaction. Replacing
    /// label ink preserves its source metadata and does not remove node picking.
    /// Native hosts accept axis-aligned slots; automatic rotated label placement
    /// requires the BrowserChart affine carrier.
    pub fn scene_widgets_composed<T: Clone + PartialEq + 'static, W: Widget>(
        mut self,
        source: impl Into<Vary<SceneWidgetsComposition<T>>>,
        factory: impl FnMut(SceneWidgetCompositionContext<T>) -> Result<W> + 'static,
    ) -> impl Widget {
        self.plot_widgets = None;
        ComposedChart {
            chart: Some(self),
            source: PlotWidgetsSource::scene_composition(source.into()),
            factory: Some(plot_widgets::scene_composition_factory(factory)),
        }
    }

    fn semantic_plot(&self) -> bool {
        #[cfg(feature = "raster")]
        if self.raster.is_some() {
            return false;
        }
        self.plot
    }
    pub fn on_select(mut self, callback: impl FnMut(Mark) + 'static) -> Self {
        self.selection = Some(Box::new(callback));
        self
    }
    /// Controlled state: legend activation proposes a value for the caller to
    /// accept or reject. Matching marks/data hide without recomputing scales.
    pub fn hidden(mut self, value: impl Into<Vary<BTreeSet<String>>>) -> Self {
        self.hidden = Some(value.into());
        self
    }
    pub fn default_hidden(mut self, value: BTreeSet<String>) -> Self {
        self.default_hidden = value;
        self
    }
    /// Observe accepted legend activations. Categories become ordinary action
    /// buttons only when configured; series retain their hidden-state toggle.
    /// The callback observes activation intent before a series toggle, not a
    /// successful hidden-state commit. A reentrant Scene update is rechecked.
    pub fn on_legend_activate(mut self, callback: impl FnMut(LegendActivation) + 'static) -> Self {
        self.legend_activate = Some(Box::new(callback));
        self
    }
    // Adapters retain row/target intent across their own publication queues.
    // This internal series channel does not make static categories actionable.
    #[cfg(any(feature = "raster", all(target_arch = "wasm32", feature = "web")))]
    pub(crate) fn on_legend_toggle(
        mut self,
        callback: impl FnMut(LegendActivation) + 'static,
    ) -> Self {
        self.legend_toggle = Some(Box::new(callback));
        self
    }
    pub fn on_hidden_change(mut self, callback: impl FnMut(BTreeSet<String>) + 'static) -> Self {
        self.hidden_change = Some(Box::new(callback));
        self
    }
    pub fn selected(mut self, value: impl Into<Vary<Option<String>>>) -> Self {
        self.selected = Some(value.into());
        self
    }
    /// Controlled plot-hover identity. Data-button pointer observations propose
    /// through on_hover; the caller decides when to publish the accepted value.
    pub fn hovered(mut self, value: impl Into<Vary<Option<String>>>) -> Self {
        self.hovered = Some(value.into());
        self
    }
    /// Suppress semantic vector plot nodes while retaining Glory legend, data
    /// and tooltip controls for an application-owned SVG or Canvas surface.
    pub fn plot(mut self, show: bool) -> Self {
        self.plot = show;
        self
    }
    /// Control the built-in single-datum visual dimming (default true).
    /// False keeps hover/focus/selection callbacks, tooltips and accessibility;
    /// callers can instead publish a controlled association through the Scene.
    pub fn highlight(mut self, enabled: bool) -> Self {
        self.highlight = enabled;
        self
    }
    pub fn default_selected(mut self, value: Option<String>) -> Self {
        self.default_selected = value;
        self
    }
    pub fn on_selection_change(mut self, callback: impl FnMut(Option<Mark>) + 'static) -> Self {
        self.selection_change = Some(Box::new(callback));
        self
    }
    pub fn on_hover(mut self, callback: impl FnMut(Option<Mark>) + 'static) -> Self {
        self.hover = Some(Box::new(callback));
        self
    }
    pub fn on_focus(mut self, callback: impl FnMut(Option<Mark>) + 'static) -> Self {
        self.focus = Some(Box::new(callback));
        self
    }
    /// Destination proposal plus a host focus intent. on_focus reports the
    /// actual host confirmation separately, including refusal by headless hosts.
    pub fn on_navigate(mut self, callback: impl FnMut(Mark) + 'static) -> Self {
        self.navigate = Some(Box::new(callback));
        self
    }
    pub fn on_reject(mut self, callback: impl FnMut(ChartError) + 'static) -> Self {
        self.rejection = Some(Box::new(callback));
        self
    }
    pub fn data_controls(mut self, show: bool) -> Self {
        self.data_controls = show;
        self
    }
    /// Override data-button text by stable mark ID. Unspecified IDs use the
    /// mark's label and scalar value. Each label may contain up to 2048 bytes.
    pub fn data_control_labels(
        mut self,
        labels: impl Into<Vary<BTreeMap<String, String>>>,
    ) -> Self {
        self.data_labels = Some(labels.into());
        self
    }
    /// Use standard focusable data buttons on hosts with basic input support.
    /// Tab, focus and activation remain available. Arrow navigation and data
    /// button pointer-hover observations require the default enhanced controls.
    pub fn basic_data_controls(mut self) -> Self {
        self.basic_data_controls = true;
        self
    }
    pub fn legend(mut self, show: bool) -> Self {
        self.legend = show;
        self
    }
    pub fn tooltip(mut self, show: bool) -> Self {
        self.tooltip = show;
        self
    }
    pub fn legend_options(mut self, options: LegendOptions) -> Self {
        self.legend_options = options;
        self
    }
    pub fn tooltip_options(mut self, options: TooltipOptions) -> Self {
        self.tooltip_options = options;
        self
    }
    /// Controlled slice identities, in the caller's desired row order. Each
    /// render resolves current visible marks; stale or hidden IDs disappear.
    /// Without this option, tooltip content follows datum hover/focus.
    pub fn tooltip_marks(mut self, marks: impl Into<Vary<Vec<String>>>) -> Self {
        self.tooltip_marks = Some(marks.into());
        self
    }
}
fn validate_tooltip_marks(ids: &[String]) -> Result<()> {
    ensure(
        ids.len() <= MAX_MARKS
            && ids
                .iter()
                .all(|id| !id.is_empty() && id.len() <= MAX_LABEL_BYTES)
            && ids.iter().collect::<BTreeSet<_>>().len() == ids.len(),
        "invalid tooltip mark identities",
    )
}
fn validate_legend(scene: &Scene, options: &LegendOptions) -> Result<()> {
    for item in &scene.legend {
        legend_label(item, options)?;
    }
    Ok(())
}
fn ui_error(_: ChartError) -> glory::ui::Error {
    glory::render::Error::InvalidProps("invalid kartis input; inspect on_reject diagnostic").into()
}
fn validate(scene: &Scene, plot: bool, outlined: bool) -> Result<()> {
    scene.validate()?;
    ensure(
        !plot || scene.typography.label_strokes.is_empty(),
        "Glory semantic text does not support label strokes; use SVG/Canvas/raster",
    )?;
    ensure(
        !plot || scene.marks.iter().all(|mark| mark.fill_paint.is_none()),
        "Glory semantic Graphic has no gradient or pattern fill; use SVG/Canvas/raster",
    )?;
    ensure(
        !plot
            || scene
                .label_clips
                .values()
                .all(|clip| matches!(clip.geometry, Geometry::Rect { .. }))
                && scene.marks.iter().all(|mark| mark.clip_path.is_none()),
        "Glory semantic Graphic/text supports only rectangular label clips, not arbitrary clip masks; use SVG/Canvas/raster",
    )?;
    ensure(
        !plot || outlined || scene.labels.iter().all(|label| label.rotation == 0.),
        "Glory semantic text has no rotation; explicitly outline_labels with fonts or use SVG/Canvas/raster",
    )?;
    ensure(
        !plot
            || scene
                .marks
                .iter()
                .all(|m| m.geometry.commands().len() <= MAX_GRAPHIC_COMMANDS),
        "Glory Graphic allows 256 commands per mark; use SVG/Canvas export for longer paths",
    )
}
fn validate_hidden(value: &BTreeSet<String>) -> Result<()> {
    ensure(
        value.len() <= 256
            && value
                .iter()
                .all(|id| !id.is_empty() && id.len() <= MAX_LABEL_BYTES),
        "invalid hidden-series state",
    )
}
fn validate_selected(value: &Option<String>) -> Result<()> {
    ensure(
        value
            .as_ref()
            .is_none_or(|id| !id.is_empty() && id.len() <= MAX_LABEL_BYTES),
        "invalid selected identity",
    )
}
fn validate_hovered(value: &Option<String>) -> Result<()> {
    ensure(
        value
            .as_ref()
            .is_none_or(|id| !id.is_empty() && id.len() <= MAX_LABEL_BYTES),
        "invalid hovered identity",
    )
}
fn validate_data_labels(labels: &BTreeMap<String, String>) -> Result<()> {
    ensure(
        labels.len() <= MAX_MARKS
            && labels.iter().all(|(id, label)| {
                !id.is_empty() && id.len() <= MAX_LABEL_BYTES && label.len() <= 2 * MAX_LABEL_BYTES
            }),
        "invalid data control label identity or budget",
    )
}
fn local<T>(value: &Vary<T>, scope: &Scope) -> bool {
    match value {
        Vary::Value(_) => true,
        Vary::Cage(v) => v.runtime() == scope.runtime(),
        Vary::Bond(v) => v.runtime() == scope.runtime(),
    }
}
fn datum(mark: &Mark, hidden: &BTreeSet<String>, groups: &BTreeMap<String, f64>) -> bool {
    !mark.label.is_empty()
        && mark.opacity * groups.get(&mark.id).copied().unwrap_or(1.) > 0.
        && !hidden.contains(&mark.series)
}
type NodeRef = Rc<Cage<Option<NodeId>>>;
type Registry = Rc<RefCell<BTreeMap<String, NodeRef>>>;
#[derive(Clone)]
struct CompositionConfig {
    source: PlotWidgetsSource,
    manager: CompositionManager,
    dimensions: Cage<[f64; 2]>,
    profile: glory::style::Profile,
}
#[derive(Clone)]
struct CompositionState {
    config: CompositionConfig,
    packet: Cage<PlotWidgetsPacket>,
}
struct CompositionCandidate {
    stage: Rc<CompositionStage>,
    packet: PlotWidgetsPacket,
}
struct ComposedChart {
    chart: Option<Chart>,
    source: PlotWidgetsSource,
    factory: Option<plot_widgets::PlotWidgetFactory>,
}
impl Widget for ComposedChart {
    fn build(&mut self, scope: &Scope) -> glory::ui::Result {
        ensure(
            self.source.is_local(scope),
            "composition source belongs to a foreign runtime",
        )
        .map_err(ui_error)?;
        let mut chart = self
            .chart
            .take()
            .ok_or_else(|| ui_error(ChartError("composition chart mounts once".into())))?;
        let source = self.source.clone();
        ensure(
            local(&chart.scene, scope)
                && chart
                    .bound_layout
                    .as_ref()
                    .is_none_or(|source| local(source, scope)),
            "chart input belongs to a foreign runtime",
        )
        .map_err(ui_error)?;
        let mut factory = self.factory.take();
        glory::ui::view()
            .class("kartis-composed-chart")
            .style(Style::new().position(Position::Relative))
            .fill(glory::ui::widget(move |scope| {
                let initial = if let Some(bound) = &chart.bound_layout {
                    bound.try_get()?.scene
                } else {
                    chart.scene.try_get()?
                };
                let dimensions = scope.cage([initial.width, initial.height]);
                let style = scope.bond(move || {
                    dimensions.with(|[width, height]| {
                        Style::new()
                            .position(Position::Absolute)
                            .left(Length::Px(0.))
                            .top(Length::Px(0.))
                            .width(Length::Px(*width))
                            .height(Length::Px(*height))
                            .pointer_events(PointerEvents::None)
                    })
                });
                let profile = scope.style_profile()?;
                let manager = CompositionManager::new(
                    scope,
                    profile,
                    factory.take().ok_or_else(|| {
                        ui_error(ChartError("composition factory retired".into()))
                    })?,
                    style.into(),
                    None,
                )
                .map_err(ui_error)?;
                chart.composition = Some(CompositionConfig {
                    source: source.clone(),
                    manager: manager.clone(),
                    dimensions,
                    profile,
                });
                chart.build(scope)?;
                manager.raise().map_err(ui_error)
            }))
            .show_in(scope)
    }
}
struct AcceptedInputs {
    bound_plot: Option<plot_pointer::Prepared>,
    data_pointer: bool,
    semantic_labels: Option<SemanticLabelsFrame>,
    drawings: Option<PreparedDrawings>,
    #[cfg(feature = "raster")]
    outlines: Option<crate::text_outline::OutlinedLabels>,
    #[cfg(feature = "raster")]
    paints: Option<crate::text_outline::PaintedLabels>,
    scene: Scene,
    paint_scene: Scene,
    composition: Option<CompositionCandidate>,
    plot_widgets: Option<Vec<PreparedSlot>>,
    hidden: Option<BTreeSet<String>>,
    selected: Option<Option<String>>,
    hovered: Option<Option<String>>,
    tooltip: Option<Vec<String>>,
    data_labels: Option<BTreeMap<String, String>>,
    layout: Option<PresentationLayout>,
    #[cfg(feature = "raster")]
    raster_update: Option<(
        crate::raster_widget::RasterFrame,
        crate::render::raster::RasterOptions,
    )>,
}
#[derive(Clone)]
struct State {
    bound_plot: Option<plot_pointer::BoundPlot>,
    data_pointer: Cage<bool>,
    data_pointer_style: bool,
    physical_hover: Callback<Option<SurfacePhysicalHit>>,
    semantic_labels: Option<SemanticLabelsState>,
    data_labels: Cage<BTreeMap<String, String>>,
    drawings: Cage<PreparedDrawings>,
    #[cfg(feature = "raster")]
    outlined: Option<
        Rc<(
            crate::render::raster::FontRegistry,
            crate::text_outline::OutlineLabelOptions,
        )>,
    >,
    #[cfg(feature = "raster")]
    outlines: Cage<crate::text_outline::OutlinedLabels>,
    #[cfg(feature = "raster")]
    painted: Option<
        Rc<(
            crate::render::raster::FontRegistry,
            crate::text_outline::PaintedLabelOptions,
        )>,
    >,
    #[cfg(feature = "raster")]
    paints: Cage<crate::text_outline::PaintedLabels>,
    legend_layout: Option<Rc<LegendLayoutOptions>>,
    tooltip_layout: Option<Rc<TooltipLayoutOptions>>,
    label_collision: Option<Rc<LabelCollisionOptions>>,
    label_priorities: Rc<BTreeMap<usize, i32>>,
    measurer: Renderer<(String, PresentationText), TextSize>,
    formatter: Renderer<Vec<Mark>, String>,
    legend_renderer: WidgetRenderer<LegendRenderContext>,
    symbol_renderer: WidgetRenderer<LegendRenderContext>,
    tooltip_renderer: WidgetRenderer<TooltipRenderContext>,
    layout: Cage<PresentationLayout>,
    tooltip_rect: Cage<Option<LayoutRect>>,
    #[cfg(feature = "raster")]
    raster: Option<crate::raster_widget::RasterState>,
    scene: Cage<Scene>,
    paint_scene: Cage<Scene>,
    composition: Option<CompositionState>,
    // One sparse index per accepted Scene; source changes replace it before
    // publishing the reactive Scene. Hidden-only updates reuse the same index.
    group_opacities: Rc<RefCell<BTreeMap<String, f64>>>,
    plot_widgets: Option<PlotWidgetsState>,
    hidden: Cage<BTreeSet<String>>,
    selected: Cage<Option<String>>,
    hovered: Cage<Option<String>>,
    focused: Cage<Option<String>>,
    controlled_hidden: bool,
    controlled_selection: bool,
    controlled_hover: bool,
    highlight: bool,
    selection: Callback<Mark>,
    selection_change: Callback<Option<Mark>>,
    hidden_change: Callback<BTreeSet<String>>,
    legend_activate: Callback<LegendActivation>,
    legend_toggle: Callback<LegendActivation>,
    has_legend_activate: bool,
    hover: Callback<Option<Mark>>,
    focus: Callback<Option<Mark>>,
    navigate: Callback<Mark>,
    rejection: Callback<ChartError>,
    has_rejection: bool,
    owner: glory::ui::WeakScope,
    registry: Registry,
    legend_options: Rc<LegendOptions>,
    tooltip_options: Rc<TooltipOptions>,
    tooltip_marks: Option<Cage<Vec<String>>>,
    tooltip_value: Cage<String>,
}
impl State {
    fn lookup(&self, id: &str) -> Option<Mark> {
        if let Some(bound) = &self.bound_plot {
            return bound.lookup(id);
        }
        self.hidden.with(|hidden| {
            self.scene.with(|s| {
                let groups = self.group_opacities.borrow();
                s.marks
                    .iter()
                    .find(|m| m.id == id && datum(m, hidden, &groups))
                    .cloned()
            })
        })
    }
    fn toggle(&self, id: &str, expected: &LegendTarget) {
        let activation = self.scene.with(|scene| {
            let target = scene.legend_target(id)?;
            if &target != expected || target.role() != LegendRole::Series {
                return None;
            }
            Some(LegendActivation {
                item: scene.legend.iter().find(|item| item.id == id)?.clone(),
                role: LegendRole::Series,
                target: scene.legend_targets.get(id).cloned(),
            })
        });
        let Some(activation) = activation else { return };
        let LegendTarget::Series(series) = activation.resolved_target() else {
            return;
        };
        let mut hidden = self.hidden.get_untracked();
        if !hidden.remove(&series) {
            hidden.insert(series);
        }
        if let Err(error) = validate_hidden(&hidden) {
            if self.has_rejection {
                self.rejection.emit(error);
            } else if let Some(scope) = self.owner.upgrade() {
                scope.report_error(ui_error(error));
            }
            return;
        }
        if !self.controlled_hidden {
            let prepared = (|| -> Result<_> {
                let scene = self.scene.get_untracked();
                let profile = self
                    .semantic_labels
                    .as_ref()
                    .map(|state| state.frame.get_untracked().profile)
                    .unwrap_or_default();
                let paint_scene = self.paint_scene.get_untracked();
                let packet = self
                    .composition
                    .as_ref()
                    .map(|state| state.packet.get_untracked());
                let semantic = self.prepare_semantic_labels(&paint_scene, &hidden, profile)?;
                let layout = if self.legend_layout.is_some() || self.label_collision.is_some() {
                    Some(self.prepare_layout(
                        &paint_scene,
                        &hidden,
                        semantic.as_ref(),
                        packet.as_ref(),
                    )?)
                } else {
                    None
                };
                let bound = self
                    .bound_plot
                    .as_ref()
                    .map(|state| state.prepare_hidden(&hidden))
                    .transpose()?;
                Ok((semantic, layout, bound, scene))
            })();
            let (semantic, layout, bound, accepted_scene) = match prepared {
                Ok(prepared) => prepared,
                Err(error) => {
                    if self.has_rejection {
                        self.rejection.emit(error);
                    } else if let Some(scope) = self.owner.upgrade() {
                        scope.report_error(ui_error(error));
                    }
                    return;
                }
            };
            if !self.selection.alive.get() || self.scene.with(|scene| scene != &accepted_scene) {
                return;
            }
            #[cfg(feature = "raster")]
            if let Some(raster) = &self.raster {
                let options = raster.options.get_untracked();
                let result = raster.prepare(&self.paint_scene.get_untracked(), &hidden, &options);
                if !self.selection.alive.get() || self.scene.with(|scene| scene != &accepted_scene)
                {
                    return;
                }
                match result {
                    Ok(frame) => raster.commit(frame, options),
                    Err(error) => {
                        if self.has_rejection {
                            self.rejection.emit(error);
                        } else if let Some(scope) = self.owner.upgrade() {
                            scope.report_error(ui_error(error));
                        }
                        return;
                    }
                }
            }
            if let Some(layout) = layout {
                self.layout.revise(|old| *old = layout);
            }
            if let (Some(state), Some(frame)) = (&self.semantic_labels, semantic) {
                state.frame.revise(|old| *old = frame);
            }
            let events = if let (Some(state), Some(next)) = (&self.bound_plot, bound) {
                state.commit(next)
            } else {
                vec![]
            };
            self.hidden.revise(|old| *old = hidden.clone());
            self.reconcile();
            self.bound_events(events);
        }
        self.hidden_change.emit(hidden);
        self.legend_toggle.emit(activation);
    }
    fn activate_legend(&self, id: &str, expected: &LegendTarget) {
        let activation = self.scene.with(|scene| {
            let target = scene.legend_target(id)?;
            if &target != expected {
                return None;
            }
            let role = target.role();
            let item = scene.legend.iter().find(|item| item.id == id)?.clone();
            Some(LegendActivation {
                item,
                role,
                target: scene.legend_targets.get(id).cloned(),
            })
        });
        let Some(activation) = activation else { return };
        let series = activation.role == LegendRole::Series;
        let target = activation.resolved_target();
        if self.has_legend_activate {
            self.legend_activate.emit(activation);
        }
        // Like Nivo's onClick-before-toggle order, notify intent first. A
        // callback can retire the owner or publish a new role/identity; toggle
        // resolves the current accepted Scene again before proposing Hidden.
        if series && self.selection.alive.get() {
            self.toggle(id, &target);
        }
    }
    fn activate(&self, id: &str) {
        if let Some(bound) = &self.bound_plot {
            match bound.activate(id) {
                Ok(events) => self.bound_events(events),
                Err(error) => self.bound_error(error),
            }
            return;
        }
        if let Some(mark) = self.lookup(id) {
            if !self.controlled_selection {
                self.selected.revise(|value| *value = Some(mark.id.clone()));
            }
            self.selection_change.emit(Some(mark.clone()));
            self.selection.emit(mark);
        }
    }
    fn hover(&self, id: &str, active: bool) {
        if !self.data_pointer.get_untracked() {
            return;
        }
        let next = if active {
            self.lookup(id).map(|m| m.id)
        } else {
            None
        };
        if !active && self.hovered.get_untracked().as_deref() != Some(id) {
            return;
        }
        if self.hovered.get_untracked() != next {
            if !self.controlled_hover {
                self.hovered.revise(|value| *value = next.clone());
            }
            self.hover
                .emit(next.as_deref().and_then(|id| self.lookup(id)));
        }
    }
    fn focus(&self, id: &str, active: bool) {
        if active && let Some(bound) = &self.bound_plot {
            match bound.clear_pointer() {
                Ok(events) => self.bound_events(events),
                Err(error) => {
                    self.bound_error(error);
                    return;
                }
            }
        }
        let next = if active {
            self.lookup(id).map(|m| m.id)
        } else {
            None
        };
        if !active && self.focused.get_untracked().as_deref() != Some(id) {
            return;
        }
        if self.focused.get_untracked() != next {
            self.focused.revise(|value| *value = next.clone());
            self.focus
                .emit(next.as_deref().and_then(|id| self.lookup(id)));
        }
    }
    fn active(&self) -> Option<String> {
        self.hovered
            .get()
            .or_else(|| self.focused.get())
            .or_else(|| self.selected.get())
            .filter(|id| self.lookup(id).is_some())
    }
    fn tooltip(&self) -> String {
        self.tooltip_value.get()
    }
    fn compute_tooltip(&self) -> Result<String> {
        if !self.formatter.configured()
            && let Some(bound) = &self.bound_plot
        {
            return bound.active_interaction(&self.tooltip_ids()).map_or_else(
                || Ok(String::new()),
                |active| crate::interaction_tooltip_text(&self.tooltip_options, &active),
            );
        }
        let marks = self.tooltip_data();
        if marks.is_empty() {
            return Ok(String::new());
        }
        let text = if self.formatter.configured() {
            self.formatter.call(marks)?
        } else {
            tooltip_text(&self.tooltip_options, &marks.iter().collect::<Vec<_>>())?
        };
        ensure(text.len() <= 16_384, "custom tooltip text budget exceeded")?;
        Ok(text)
    }
    fn tooltip_data(&self) -> Vec<Mark> {
        let ids = self.tooltip_ids();
        if let Some(bound) = &self.bound_plot {
            return ids.iter().filter_map(|id| bound.lookup(id)).collect();
        }
        self.hidden.with(|hidden| {
            self.scene.with(|scene| {
                let groups = self.group_opacities.borrow();
                let current = scene
                    .marks
                    .iter()
                    .filter(|mark| datum(mark, hidden, &groups))
                    .map(|mark| (mark.id.as_str(), mark))
                    .collect::<BTreeMap<_, _>>();
                let marks = ids
                    .iter()
                    .filter_map(|id| current.get(id.as_str()).copied())
                    .collect::<Vec<_>>();
                marks.into_iter().cloned().collect()
            })
        })
    }
    fn tooltip_ids(&self) -> Vec<String> {
        self.tooltip_marks
            .map(|source| source.get())
            .unwrap_or_else(|| {
                self.hovered
                    .get()
                    .or_else(|| self.focused.get())
                    .into_iter()
                    .collect()
            })
    }
    fn measure(&self, text: String, style: PresentationText) -> Result<TextSize> {
        let size = self.measurer.call((text, style))?;
        size.validate()?;
        Ok(size)
    }
    fn prepare_layout(
        &self,
        scene: &Scene,
        hidden: &BTreeSet<String>,
        semantic: Option<&SemanticLabelsFrame>,
        packet: Option<&PlotWidgetsPacket>,
    ) -> Result<PresentationLayout> {
        let label_priorities: BTreeMap<usize, i32> = self
            .label_priorities
            .iter()
            .filter_map(|(&index, &priority)| {
                packet
                    .map_or(Some(index), |packet| packet.paint_label_index(index))
                    .map(|index| (index, priority))
            })
            .collect();
        let mut result = PresentationLayout::default();
        if let Some(options) = &self.legend_layout {
            let mut items = Vec::new();
            for item in &scene.legend {
                let label = legend_control_label(
                    item,
                    scene.legend_role(&item.id).expect("accepted legend role"),
                    scene.legend_hidden(&item.id, hidden),
                    &self.legend_options,
                )?;
                let font = scene.typography.legend_font(
                    &self.legend_options.text.font,
                    self.legend_options.text.weight,
                );
                let text =
                    self.measure(label, self.legend_options.text.with_resolved_font(&font))?;
                items.push(LegendMeasuredItem {
                    id: item.id.clone(),
                    size: TextSize {
                        width: self.legend_options.item_width.unwrap_or(
                            text.width
                                + self.legend_options.item_gap
                                + if self.legend_options.symbol
                                    == crate::presentation::LegendSymbol::None
                                {
                                    0.
                                } else {
                                    self.legend_options.size
                                },
                        ),
                        height: text.height.max(self.legend_options.size),
                    },
                });
            }
            result.legend = layout_legend(&items, options)?
                .items
                .into_iter()
                .map(|item| (item.id, item.rect))
                .collect();
        }
        if let Some(options) = &self.label_collision {
            ensure(
                semantic.is_some()
                    || !scene
                        .typography
                        .label_whitespace
                        .values()
                        .any(|mode| *mode == crate::LabelWhitespace::Preserve),
                "Preserve label collision requires a semantic text measurer",
            )?;
            let mut labels = Vec::new();
            for (index, label) in scene.labels.iter().enumerate() {
                if label.color.3 == 0
                    || scene
                        .label_series
                        .get(&index)
                        .is_some_and(|id| hidden.contains(id))
                {
                    continue;
                }
                if let Some(measurement) = semantic.and_then(|frame| frame.labels.get(&index)) {
                    let placement = measurement.place_label(label)?;
                    if let Some(bounds) = placement.ink_bounds {
                        let step = bounds.height + options.padding + 2.;
                        let line_step = measurement.metrics().line_height + options.padding + 2.;
                        let mut offsets = vec![(0., -step), (0., step)];
                        if line_step != step {
                            offsets.extend([(0., -line_step), (0., line_step)]);
                        }
                        offsets.retain(|(_, y)| y.abs() <= 1_000_000.);
                        labels.push(LabelCandidate {
                            id: index.to_string(),
                            rect: bounds,
                            priority: *label_priorities.get(&index).unwrap_or(&0),
                            offsets,
                        });
                    }
                    continue;
                }
                if self.semantic_labels.is_some() {
                    continue;
                }
                let size = self.measure(
                    label.text.clone(),
                    PresentationText {
                        size: label.size,
                        color: Some(label.color),
                        ..Default::default()
                    }
                    .with_resolved_font(&scene.resolved_label_font(index)),
                )?;
                let left = label.x
                    - match label.anchor {
                        TextAnchor::Start => 0.,
                        TextAnchor::Middle => size.width / 2.,
                        TextAnchor::End => size.width,
                    };
                labels.push(LabelCandidate {
                    id: index.to_string(),
                    rect: LayoutRect {
                        x: left,
                        y: label.y - label.size * 0.8,
                        width: size.width,
                        height: size.height,
                    },
                    priority: *label_priorities.get(&index).unwrap_or(&0),
                    offsets: vec![(0., -size.height - 2.), (0., size.height + 2.)],
                });
            }
            result.labels = layout_labels(&labels, options)?
                .into_iter()
                .map(|label| (label.id.parse().expect("index identity"), label.rect))
                .collect();
            if let Some(frame) = semantic {
                for (&index, bounds) in &result.labels {
                    if let Some(bounds) = bounds
                        && let Some(measurement) = frame.labels.get(&index)
                    {
                        measurement
                            .place_label(&scene.labels[index])?
                            .with_placed_ink_bounds(*bounds)?;
                    }
                }
            }
        }
        Ok(result)
    }
    fn place_tooltip(&self, text: &str) -> Result<Option<LayoutRect>> {
        let Some(options) = &self.tooltip_layout else {
            return Ok(None);
        };
        if text.is_empty() {
            return Ok(None);
        }
        let marks = self.tooltip_data();
        let anchor = marks
            .first()
            .map(|mark| match &mark.geometry {
                Geometry::Rect {
                    x,
                    y,
                    width,
                    height,
                } => (*x + width / 2., *y + height / 2.),
                Geometry::Circle { x, y, .. } => (*x, *y),
                Geometry::Path(commands) => commands
                    .iter()
                    .find_map(|command| match command {
                        glory::render::PathCommand::Move(p)
                        | glory::render::PathCommand::Line(p) => Some((p.x, p.y)),
                        glory::render::PathCommand::Cubic { end, .. } => Some((end.x, end.y)),
                        _ => None,
                    })
                    .unwrap_or((options.viewport.x, options.viewport.y)),
            })
            .unwrap_or((options.viewport.x, options.viewport.y));
        let anchor = if let Some(bound) = &self.bound_plot {
            let ids = marks.iter().map(|mark| mark.id.clone()).collect::<Vec<_>>();
            bound.tooltip_anchor(&ids)?.unwrap_or(anchor)
        } else {
            anchor
        };
        let font = self.scene.with(|scene| {
            scene.typography.tooltip_font(
                &self.tooltip_options.text.font,
                self.tooltip_options.text.weight,
            )
        });
        let mut size = self.measure(
            text.to_owned(),
            self.tooltip_options.text.with_resolved_font(&font),
        )?;
        size.width += self.tooltip_options.padding * 2.;
        size.height += self.tooltip_options.padding * 2.;
        Ok(Some(place_tooltip(anchor, size, options)?.rect))
    }
    fn reconcile(&self) {
        for (source, callback, controlled) in [
            (self.hovered, &self.hover, self.controlled_hover),
            (self.focused, &self.focus, false),
        ] {
            if !self.selection.alive.get() {
                return;
            }
            if !controlled
                && source
                    .get_untracked()
                    .as_ref()
                    .is_some_and(|id| self.lookup(id).is_none())
            {
                source.revise(|value| *value = None);
                callback.emit(None);
            }
        }
        if self.selection.alive.get()
            && !self.controlled_selection
            && self
                .selected
                .get_untracked()
                .as_ref()
                .is_some_and(|id| self.lookup(id).is_none())
        {
            self.selected.revise(|value| *value = None);
            self.selection_change.emit(None);
        }
    }
    fn data_marks(&self) -> Vec<Mark> {
        if let Some(bound) = &self.bound_plot {
            return bound.marks();
        }
        self.hidden.with(|hidden| {
            self.scene.with(|s| {
                let groups = self.group_opacities.borrow();
                s.paint_marks()
                    .filter(|m| datum(m, hidden, &groups))
                    .cloned()
                    .collect::<Vec<_>>()
            })
        })
    }
    fn destination(&self, from: NodeId, command: Navigation) -> Option<(Mark, NodeId)> {
        let marks = self.data_marks();
        let registry = self.registry.borrow();
        let index = marks.iter().position(|m| {
            registry
                .get(&m.id)
                .is_some_and(|r| r.try_get().ok().flatten() == Some(from))
        })?;
        let last = marks.len().checked_sub(1)?;
        let index = match command {
            Navigation::First => 0,
            Navigation::Last => last,
            Navigation::Next => (index + 1).min(last),
            Navigation::Previous => index.saturating_sub(1),
            Navigation::PageNext => (index + 10).min(last),
            Navigation::PagePrevious => index.saturating_sub(10),
            Navigation::Reveal => index,
        };
        let mark = marks[index].clone();
        let node = registry.get(&mark.id)?.try_get().ok().flatten()?;
        Some((mark, node))
    }
}
impl Widget for Chart {
    fn build(&mut self, scope: &Scope) -> glory::ui::Result {
        if !local(&self.scene, scope)
            || self.bound_layout.as_ref().is_some_and(|v| !local(v, scope))
            || !local(&self.data_control_pointer, scope)
            || self
                .plot_widgets
                .as_ref()
                .is_some_and(|input| !input.source.is_local(scope))
            || self
                .composition
                .as_ref()
                .is_some_and(|input| !input.source.is_local(scope))
            || self.hidden.as_ref().is_some_and(|v| !local(v, scope))
            || self.selected.as_ref().is_some_and(|v| !local(v, scope))
            || self.hovered.as_ref().is_some_and(|v| !local(v, scope))
            || self
                .tooltip_marks
                .as_ref()
                .is_some_and(|v| !local(v, scope))
            || self.data_labels.as_ref().is_some_and(|v| !local(v, scope))
        {
            return Err(glory::render::Error::InvalidProps(
                "kartis input belongs to a foreign runtime",
            )
            .into());
        }
        #[cfg(feature = "raster")]
        {
            if self
                .raster
                .as_ref()
                .is_some_and(|input| !local(&input.options, scope))
            {
                return Err(glory::render::Error::InvalidProps(
                    "raster options belong to a foreign runtime",
                )
                .into());
            }
            ensure(
                self.raster.is_some() || self.raster_source.is_none(),
                "raster_image_source requires a raster chart",
            )
            .map_err(ui_error)?;
        }
        let initial_bound = self
            .bound_layout
            .as_ref()
            .map(|v| glory::core::untracked(|| v.try_get()))
            .transpose()?;
        if let Some(frame) = &initial_bound {
            plot_pointer::validate(frame).map_err(ui_error)?;
            let supported = self.semantic_plot();
            #[cfg(feature = "raster")]
            let supported = supported || self.raster.is_some();
            ensure(
                supported,
                "chart_layout requires a semantic Graphic plot or raster image",
            )
            .map_err(ui_error)?;
            if scope.capabilities()?.input && !scope.supports_plot_pointer()? {
                return Err(glory::render::Error::Unsupported("finite chart plot pointer").into());
            }
        }
        let initial = match &initial_bound {
            Some(frame) => frame.scene.clone(),
            None => glory::core::untracked(|| self.scene.try_get())?,
        };
        validate(
            &initial,
            self.semantic_plot() && self.composition.is_none(),
            self.accepts_rotated_labels(),
        )
        .map_err(ui_error)?;
        let initial_hidden = match &self.hidden {
            Some(value) => glory::core::untracked(|| value.try_get())?,
            None => self.default_hidden.clone(),
        };
        validate_hidden(&initial_hidden).map_err(ui_error)?;
        let initial_composition = if let Some(config) = &self.composition {
            let has_plot = self.semantic_plot();
            #[cfg(feature = "raster")]
            let has_plot = has_plot || self.raster.is_some();
            ensure(
                has_plot,
                "composed Widgets require a semantic plot or raster image",
            )
            .map_err(ui_error)?;
            let packet = glory::core::untracked(|| config.source.read()).map_err(ui_error)?;
            ensure(
                packet.is_composition(),
                "composition chart requires a composition packet",
            )
            .map_err(ui_error)?;
            let view = ZoomTransform::default();
            #[cfg(feature = "raster")]
            let view = self
                .raster
                .as_ref()
                .map(|input| glory::core::untracked(|| input.options.try_get()))
                .transpose()?
                .map_or(view, |options| options.view);
            let rows = plot_widgets::prepare_packet(
                &initial,
                &packet,
                config.profile,
                view,
                initial_bound
                    .as_ref()
                    .and_then(|frame| frame.interaction.as_ref()),
            )
            .map_err(ui_error)?;
            let stage = config
                .manager
                .prepare(rows, initial_hidden.clone())
                .map_err(ui_error)?;
            Some(CompositionCandidate { stage, packet })
        } else {
            None
        };
        let initial_paint = initial_composition.as_ref().map_or_else(
            || initial.clone(),
            |candidate| candidate.packet.project_scene(&initial),
        );
        validate(
            &initial_paint,
            self.semantic_plot(),
            self.accepts_rotated_labels(),
        )
        .map_err(ui_error)?;
        let plot_widgets_source = self.plot_widgets.as_ref().map(|input| input.source.clone());
        let plot_widgets = if let Some(input) = self.plot_widgets.take() {
            let has_plot = self.semantic_plot();
            #[cfg(feature = "raster")]
            let has_plot = has_plot || self.raster.is_some();
            if !has_plot {
                return Err(glory::render::Error::Unsupported(
                    "plot Widgets require a semantic plot or raster image",
                )
                .into());
            }
            let profile = scope.style_profile()?;
            let packet = glory::core::untracked(|| input.source.read())
                .map_err(|error| glory::render::Error::Backend(error.0))?;
            let view = ZoomTransform::default();
            #[cfg(feature = "raster")]
            let view = if let Some(input) = &self.raster {
                glory::core::untracked(|| input.options.try_get())?.view
            } else {
                view
            };
            let rows = plot_widgets::prepare_packet(
                &initial,
                &packet,
                profile,
                view,
                initial_bound
                    .as_ref()
                    .and_then(|frame| frame.interaction.as_ref()),
            )
            .map_err(|error| glory::render::Error::Backend(error.0))?;
            Some(PlotWidgetsState::new(scope, rows, profile, input.factory))
        } else {
            None
        };
        let initial_drawings = if self.semantic_plot() {
            prepare_drawings(&initial_paint).map_err(ui_error)?
        } else {
            Default::default()
        };
        #[cfg(feature = "raster")]
        let initial_outlines = if let Some((fonts, options)) = &self.outlined {
            ensure(
                self.semantic_plot(),
                "outline_labels requires the semantic plot",
            )
            .map_err(ui_error)?;
            ensure(self.label_collision.is_none(),"outline_labels requires explicit placement; automatic semantic-text collision metrics do not describe rotated glyph bounds").map_err(ui_error)?;
            crate::text_outline::outline_labels(&initial_paint, fonts, options).map_err(ui_error)?
        } else {
            Default::default()
        };
        #[cfg(feature = "raster")]
        let initial_paints = if let Some((fonts, options)) = &self.painted {
            ensure(
                self.semantic_plot(),
                "painted_labels requires the semantic plot",
            )
            .map_err(ui_error)?;
            ensure(
                self.label_collision.is_none(),
                "painted_labels requires explicit measured placement",
            )
            .map_err(ui_error)?;
            crate::text_outline::painted_labels(&initial_paint, fonts, options).map_err(ui_error)?
        } else {
            Default::default()
        };
        ensure(self.label_collision.is_none()||self.semantic_plot(),"widget label collision requires the semantic plot; apply pure layout_labels to external/raster scenes explicitly").map_err(ui_error)?;
        if let Some(options) = &self.legend_layout {
            options.validate().map_err(ui_error)?;
        }
        if let Some(options) = &self.tooltip_layout {
            options.validate().map_err(ui_error)?;
        }
        if let Some(options) = &self.label_collision {
            options.validate().map_err(ui_error)?;
        }
        ensure(
            self.measurer.is_some()
                || self.legend_layout.is_none()
                    && self.tooltip_layout.is_none()
                    && (self.label_collision.is_none() || self.semantic_labels.is_some()),
            "automatic presentation layout requires an explicit text measurer",
        )
        .map_err(ui_error)?;
        ensure(
            self.semantic_labels.is_none() || self.semantic_plot(),
            "semantic text metrics require the semantic plot",
        )
        .map_err(ui_error)?;
        ensure(
            self.label_priorities.len() <= MAX_MARKS
                && self.label_priorities.keys().all(|i| *i < MAX_MARKS),
            "label priority slot budget exceeded",
        )
        .map_err(ui_error)?;
        self.legend_options.validate().map_err(ui_error)?;
        self.tooltip_options.validate().map_err(ui_error)?;
        validate_legend(&initial, &self.legend_options).map_err(ui_error)?;
        let tooltip_marks = self
            .tooltip_marks
            .as_ref()
            .map(|v| glory::core::untracked(|| v.try_get()))
            .transpose()?;
        if let Some(ids) = &tooltip_marks {
            validate_tooltip_marks(ids).map_err(ui_error)?;
        }
        let mut data_labels = self
            .data_labels
            .as_ref()
            .map(|value| glory::core::untracked(|| value.try_get()))
            .transpose()?
            .unwrap_or_default();
        validate_data_labels(&data_labels).map_err(ui_error)?;
        let hidden = initial_hidden;
        validate_hidden(&hidden).map_err(ui_error)?;
        #[cfg(not(feature = "raster"))]
        let image_view: Option<ZoomTransform> = None;
        #[cfg(feature = "raster")]
        let image_view = self
            .raster
            .as_ref()
            .map(|input| glory::core::untracked(|| input.options.try_get()))
            .transpose()?
            .map(|options| options.view);
        let bound_plot = initial_bound
            .map(|frame| {
                if image_view.is_some() {
                    return plot_pointer::BoundPlot::new_projected(
                        scope,
                        frame,
                        &hidden,
                        initial_composition
                            .as_ref()
                            .map_or_else(BTreeSet::new, |candidate| {
                                candidate.packet.replace_ink().clone()
                            }),
                        image_view,
                    );
                }
                if let Some(candidate) = &initial_composition {
                    plot_pointer::BoundPlot::new_masked(
                        scope,
                        frame,
                        &hidden,
                        candidate.packet.replace_ink().clone(),
                    )
                } else {
                    plot_pointer::BoundPlot::new(scope, frame, &hidden)
                }
            })
            .transpose()
            .map_err(ui_error)?;
        if let Some(bound) = &bound_plot {
            let mut labels = bound.data_labels();
            labels.extend(data_labels);
            validate_data_labels(&labels).map_err(ui_error)?;
            data_labels = labels;
        }
        let data_pointer_style =
            bound_plot.is_some() || !matches!(&self.data_control_pointer, Vary::Value(true));
        let data_pointer =
            bound_plot.is_none() && glory::core::untracked(|| self.data_control_pointer.try_get())?;
        let selected = match &self.selected {
            Some(v) => glory::core::untracked(|| v.try_get())?,
            None => self.default_selected.clone(),
        };
        validate_selected(&selected).map_err(ui_error)?;
        let hovered = match &self.hovered {
            Some(v) => glory::core::untracked(|| v.try_get())?,
            None => None,
        };
        validate_hovered(&hovered).map_err(ui_error)?;
        #[cfg(feature = "raster")]
        let raster = if let Some(input) = &self.raster {
            let options = glory::core::untracked(|| input.options.try_get())?;
            Some(
                crate::raster_widget::RasterState::new(
                    scope,
                    input,
                    options,
                    &initial_paint,
                    &hidden,
                    self.raster_source.take(),
                )
                .map_err(ui_error)?,
            )
        } else {
            None
        };
        let rejection = Callback::new(self.rejection.take());
        let has_rejection = rejection.value.borrow().is_some();
        let semantic_labels_input = self.semantic_labels.take();
        let semantic_labels_source = semantic_labels_input
            .as_ref()
            .map(|input| input.profile.clone());
        let initial_semantic_profile = semantic_labels_source
            .as_ref()
            .map(|source| glory::core::untracked(|| source.try_get()))
            .transpose()?
            .unwrap_or_default();
        let state = State {
            bound_plot,
            data_pointer: scope.cage(data_pointer),
            data_pointer_style,
            physical_hover: Callback::new(self.physical_hover.take()),
            semantic_labels: semantic_labels_input
                .map(|input| SemanticLabelsState::new(scope, input.measure)),
            data_labels: scope.cage(data_labels),
            drawings: scope.cage(initial_drawings),
            #[cfg(feature = "raster")]
            outlined: self.outlined.clone().map(Rc::new),
            #[cfg(feature = "raster")]
            outlines: scope.cage(initial_outlines),
            #[cfg(feature = "raster")]
            painted: self.painted.clone().map(Rc::new),
            #[cfg(feature = "raster")]
            paints: scope.cage(initial_paints),
            legend_layout: self.legend_layout.clone().map(Rc::new),
            tooltip_layout: self.tooltip_layout.clone().map(Rc::new),
            label_collision: self.label_collision.clone().map(Rc::new),
            label_priorities: Rc::new(self.label_priorities.clone()),
            measurer: Renderer::new(self.measurer.take()),
            formatter: Renderer::new(self.tooltip_formatter.take()),
            legend_renderer: Renderer::new(self.legend_renderer.take()),
            symbol_renderer: Renderer::new(self.symbol_renderer.take()),
            tooltip_renderer: Renderer::new(self.tooltip_renderer.take()),
            layout: scope.cage(PresentationLayout::default()),
            tooltip_rect: scope.cage(None),
            #[cfg(feature = "raster")]
            raster,
            group_opacities: Rc::new(RefCell::new(initial.mark_group_opacities())),
            paint_scene: scope.cage(initial_paint),
            composition: self.composition.clone().map(|config| CompositionState {
                config,
                packet: scope.cage(initial_composition.as_ref().unwrap().packet.clone()),
            }),
            scene: scope.cage(initial),
            plot_widgets,
            hidden: scope.cage(hidden),
            selected: scope.cage(selected),
            hovered: scope.cage(hovered),
            focused: scope.cage(None),
            controlled_hidden: self.hidden.is_some(),
            controlled_selection: self.selected.is_some(),
            controlled_hover: self.hovered.is_some(),
            highlight: self.highlight,
            selection: Callback::new(self.selection.take()),
            selection_change: Callback::new(self.selection_change.take()),
            hidden_change: Callback::new(self.hidden_change.take()),
            has_legend_activate: self.legend_activate.is_some(),
            legend_activate: Callback::new(self.legend_activate.take()),
            legend_toggle: Callback::new(self.legend_toggle.take()),
            hover: Callback::new(self.hover.take()),
            focus: Callback::new(self.focus.take()),
            navigate: Callback::new(self.navigate.take()),
            rejection: rejection.clone(),
            has_rejection,
            owner: scope.downgrade(),
            registry: Rc::new(RefCell::new(BTreeMap::new())),
            legend_options: Rc::new(self.legend_options.clone()),
            tooltip_options: Rc::new(self.tooltip_options.clone()),
            tooltip_marks: tooltip_marks.map(|ids| scope.cage(ids)),
            tooltip_value: scope.cage(String::new()),
        };
        let cleanup = state.clone();
        let clean_rejection = rejection.clone();
        scope.on_cleanup(move || {
            if let Some(labels) = &cleanup.semantic_labels {
                labels.clear();
            }
            cleanup.measurer.clear();
            cleanup.formatter.clear();
            cleanup.legend_renderer.clear();
            cleanup.symbol_renderer.clear();
            cleanup.tooltip_renderer.clear();
            if let Some(widgets) = &cleanup.plot_widgets {
                widgets.clear();
            }
            cleanup.selection.clear();
            cleanup.selection_change.clear();
            cleanup.hidden_change.clear();
            cleanup.legend_activate.clear();
            cleanup.legend_toggle.clear();
            cleanup.hover.clear();
            cleanup.physical_hover.clear();
            cleanup.focus.clear();
            cleanup.navigate.clear();
            cleanup.registry.borrow_mut().clear();
            #[cfg(feature = "raster")]
            if let Some(raster) = &cleanup.raster {
                raster.clear();
            }
            clean_rejection.clear();
        })?;
        let initial_semantic = state
            .prepare_semantic_labels(
                &state.paint_scene.get_untracked(),
                &state.hidden.get_untracked(),
                initial_semantic_profile,
            )
            .map_err(ui_error)?;
        let initial_layout = state
            .prepare_layout(
                &state.paint_scene.get_untracked(),
                &state.hidden.get_untracked(),
                initial_semantic.as_ref(),
                initial_composition
                    .as_ref()
                    .map(|candidate| &candidate.packet),
            )
            .map_err(ui_error)?;
        if !state.selection.alive.get() {
            return Err(ui_error(ChartError("presentation owner retired".into())));
        }
        state.layout.revise(|old| *old = initial_layout);
        if let (Some(state), Some(frame)) = (&state.semantic_labels, initial_semantic) {
            state.frame.revise(|old| *old = frame);
        }
        if let (Some(config), Some(candidate)) = (&state.composition, initial_composition) {
            candidate.stage.validate().map_err(ui_error)?;
            config
                .config
                .manager
                .commit(&candidate.stage)
                .map_err(ui_error)?;
        }
        let source = self.scene.clone();
        let bound_source = self.bound_layout.clone();
        let data_pointer_source = self.data_control_pointer.clone();
        let hidden_source = self.hidden.clone();
        let selected_source = self.selected.clone();
        let hovered_source = self.hovered.clone();
        let tooltip_source = self.tooltip_marks.clone();
        let data_labels_source = self.data_labels.clone();
        let show_plot = self.semantic_plot();
        let allow_outlined = self.accepts_rotated_labels();
        #[cfg(feature = "raster")]
        let raster_source = self.raster.as_ref().map(|input| input.options.clone());
        let observed = state.clone();
        let weak = scope.downgrade();
        scope.structural(move || {
            let Some(scope) = weak.upgrade() else { return };
            let read = || -> Result<_> {
                let bound_layout = bound_source
                    .as_ref()
                    .map(Vary::try_get)
                    .transpose()
                    .map_err(|error| ChartError(error.to_string()))?;
                if let Some(frame) = &bound_layout {
                    plot_pointer::validate(frame)?;
                }
                let scene = match &bound_layout {
                    Some(frame) => frame.scene.clone(),
                    None => source
                        .try_get()
                        .map_err(|error| ChartError(error.to_string()))?,
                };
                let data_pointer = bound_layout.is_none()
                    && data_pointer_source
                        .try_get()
                        .map_err(|error| ChartError(error.to_string()))?;
                validate(
                    &scene,
                    show_plot && observed.composition.is_none(),
                    allow_outlined,
                )?;
                #[cfg(feature = "raster")]
                let raster_options = raster_source
                    .as_ref()
                    .map(|source| {
                        source
                            .try_get()
                            .map_err(|error| ChartError(error.to_string()))
                    })
                    .transpose()?;
                let plot_widgets = if let (Some(source), Some(widgets)) =
                    (&plot_widgets_source, &observed.plot_widgets)
                {
                    let packet = source.read()?;
                    let view = ZoomTransform::default();
                    #[cfg(feature = "raster")]
                    let view = raster_options.as_ref().map_or(view, |options| options.view);
                    Some(plot_widgets::prepare_packet(
                        &scene,
                        &packet,
                        widgets.profile,
                        view,
                        bound_layout
                            .as_ref()
                            .and_then(|frame| frame.interaction.as_ref()),
                    )?)
                } else {
                    None
                };
                validate_legend(&scene, &observed.legend_options)?;
                let hidden = hidden_source
                    .as_ref()
                    .map(Vary::try_get)
                    .transpose()
                    .map_err(|error| ChartError(error.to_string()))?;
                if let Some(value) = &hidden {
                    validate_hidden(value)?;
                }
                let selected = selected_source
                    .as_ref()
                    .map(Vary::try_get)
                    .transpose()
                    .map_err(|error| ChartError(error.to_string()))?;
                if let Some(value) = &selected {
                    validate_selected(value)?;
                }
                let hovered = hovered_source
                    .as_ref()
                    .map(Vary::try_get)
                    .transpose()
                    .map_err(|error| ChartError(error.to_string()))?;
                if let Some(value) = &hovered {
                    validate_hovered(value)?;
                }
                let tooltip = tooltip_source
                    .as_ref()
                    .map(Vary::try_get)
                    .transpose()
                    .map_err(|error| ChartError(error.to_string()))?;
                if let Some(ids) = &tooltip {
                    validate_tooltip_marks(ids)?;
                }
                let mut data_labels = data_labels_source
                    .as_ref()
                    .map(Vary::try_get)
                    .transpose()
                    .map_err(|error| ChartError(error.to_string()))?;
                if let Some(labels) = &data_labels {
                    validate_data_labels(labels)?;
                }
                let candidate_hidden = hidden.as_ref().cloned().unwrap_or_else(|| {
                    if observed.composition.is_some() {
                        observed.hidden.get()
                    } else {
                        observed.hidden.get_untracked()
                    }
                });
                let composition = if let Some(config) = &observed.composition {
                    let packet = config.config.source.read()?;
                    ensure(
                        packet.is_composition(),
                        "composition chart requires a composition packet",
                    )?;
                    let view = ZoomTransform::default();
                    #[cfg(feature = "raster")]
                    let view = raster_options.as_ref().map_or(view, |options| options.view);
                    let rows = plot_widgets::prepare_packet(
                        &scene,
                        &packet,
                        config.config.profile,
                        view,
                        bound_layout
                            .as_ref()
                            .and_then(|frame| frame.interaction.as_ref()),
                    )?;
                    let stage = config
                        .config
                        .manager
                        .prepare(rows, candidate_hidden.clone())?;
                    Some(CompositionCandidate { stage, packet })
                } else {
                    None
                };
                let paint_scene = composition.as_ref().map_or_else(
                    || scene.clone(),
                    |candidate| candidate.packet.project_scene(&scene),
                );
                validate(&paint_scene, show_plot, allow_outlined)?;
                let drawings = if show_plot && observed.paint_scene.get_untracked() != paint_scene {
                    Some(prepare_drawings(&paint_scene)?)
                } else {
                    None
                };
                let bound_plot =
                    if let (Some(bound), Some(frame)) = (&observed.bound_plot, bound_layout) {
                        #[cfg(not(feature = "raster"))]
                        let image_view: Option<ZoomTransform> = None;
                        #[cfg(feature = "raster")]
                        let image_view = raster_options.as_ref().map(|options| options.view);
                        Some(if image_view.is_some() {
                            bound.prepare_projected(
                                frame,
                                &candidate_hidden,
                                composition
                                    .as_ref()
                                    .map_or_else(BTreeSet::new, |candidate| {
                                        candidate.packet.replace_ink().clone()
                                    }),
                                image_view,
                            )?
                        } else if let Some(candidate) = &composition {
                            bound.prepare_masked(
                                frame,
                                &candidate_hidden,
                                candidate.packet.replace_ink().clone(),
                            )?
                        } else {
                            bound.prepare(frame, &candidate_hidden)?
                        })
                    } else {
                        None
                    };
                if let Some(bound) = &bound_plot {
                    let mut labels = bound.data_labels();
                    if let Some(custom) = data_labels {
                        labels.extend(custom);
                    }
                    validate_data_labels(&labels)?;
                    data_labels = Some(labels);
                }
                let profile = semantic_labels_source
                    .as_ref()
                    .map(|source| {
                        source
                            .try_get()
                            .map_err(|error| ChartError(error.to_string()))
                    })
                    .transpose()?
                    .unwrap_or_default();
                // Mark-only replacement keeps the old presentation callback
                // policy. Only a changed source or label projection needs new
                // semantic/collision caches; labels use paint-pool ordinals.
                let label_projection_changed = composition.as_ref().is_some_and(|candidate| {
                    observed
                        .composition
                        .as_ref()
                        .unwrap()
                        .packet
                        .get_untracked()
                        .replaced_label_indices()
                        != candidate.packet.replaced_label_indices()
                });
                let presentation_changed =
                    observed.scene.get_untracked() != scene || label_projection_changed;
                let semantic_labels = if observed
                    .semantic_labels
                    .as_ref()
                    .is_some_and(|state| state.frame.get_untracked().profile != profile)
                    || presentation_changed
                    || observed.hidden.get_untracked() != candidate_hidden
                {
                    observed.prepare_semantic_labels(&paint_scene, &candidate_hidden, profile)?
                } else {
                    None
                };
                let layout = if (observed.legend_layout.is_some()
                    || observed.label_collision.is_some())
                    && (presentation_changed
                        || observed.hidden.get_untracked() != candidate_hidden
                        || semantic_labels.is_some())
                {
                    Some(observed.prepare_layout(
                        &paint_scene,
                        &candidate_hidden,
                        semantic_labels.as_ref(),
                        composition.as_ref().map(|candidate| &candidate.packet),
                    )?)
                } else {
                    None
                };
                #[cfg(feature = "raster")]
                let raster_update =
                    if let (Some(raster), Some(options)) = (&observed.raster, &raster_options) {
                        let candidate_hidden = hidden
                            .as_ref()
                            .cloned()
                            .unwrap_or_else(|| observed.hidden.get_untracked());
                        if observed.scene.get_untracked() != scene
                            || observed.paint_scene.get_untracked() != paint_scene
                            || composition.as_ref().is_some_and(|candidate| {
                                observed
                                    .composition
                                    .as_ref()
                                    .unwrap()
                                    .packet
                                    .get_untracked()
                                    != candidate.packet
                            })
                            || observed.hidden.get_untracked() != candidate_hidden
                            || &raster.options.get_untracked() != options
                        {
                            Some((
                                raster.prepare(&paint_scene, &candidate_hidden, options)?,
                                options.clone(),
                            ))
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                #[cfg(feature = "raster")]
                let outlines = if let Some(config) = &observed.outlined {
                    if presentation_changed {
                        Some(crate::text_outline::outline_labels(
                            &paint_scene,
                            &config.0,
                            &config.1,
                        )?)
                    } else {
                        None
                    }
                } else {
                    None
                };
                #[cfg(feature = "raster")]
                let paints = if let Some(config) = &observed.painted {
                    if presentation_changed {
                        Some(crate::text_outline::painted_labels(
                            &paint_scene,
                            &config.0,
                            &config.1,
                        )?)
                    } else {
                        None
                    }
                } else {
                    None
                };
                // Publisher/measurement callbacks may stop or supersede a
                // candidate. Retarget validates its epoch without changing any
                // accepted child Cage or running its factory again.
                let composition = composition
                    .map(|candidate| {
                        let manager = &observed.composition.as_ref().unwrap().config.manager;
                        let stage = manager.retarget(
                            &candidate.stage,
                            candidate.stage.rows().to_vec(),
                            candidate_hidden.clone(),
                        )?;
                        Ok::<_, ChartError>(CompositionCandidate {
                            stage,
                            packet: candidate.packet,
                        })
                    })
                    .transpose()?;
                Ok(AcceptedInputs {
                    bound_plot,
                    data_pointer,
                    semantic_labels,
                    drawings,
                    #[cfg(feature = "raster")]
                    outlines,
                    #[cfg(feature = "raster")]
                    paints,
                    scene,
                    paint_scene,
                    composition,
                    plot_widgets,
                    hidden,
                    selected,
                    hovered,
                    tooltip,
                    data_labels,
                    layout,
                    #[cfg(feature = "raster")]
                    raster_update,
                })
            };
            match read() {
                Ok(AcceptedInputs {
                    bound_plot,
                    data_pointer,
                    semantic_labels,
                    drawings,
                    #[cfg(feature = "raster")]
                    outlines,
                    #[cfg(feature = "raster")]
                    paints,
                    scene,
                    paint_scene,
                    composition,
                    plot_widgets,
                    hidden,
                    selected,
                    hovered,
                    tooltip,
                    data_labels,
                    layout,
                    #[cfg(feature = "raster")]
                    raster_update,
                }) => glory::core::untracked(|| {
                    if !observed.selection.alive.get() {
                        return;
                    }
                    if let (Some(config), Some(candidate)) = (&observed.composition, composition) {
                        if let Err(error) = config.config.manager.commit(&candidate.stage) {
                            if has_rejection {
                                rejection.emit(error);
                            } else {
                                scope.report_error(ui_error(error));
                            }
                            return;
                        }
                        config.packet.revise(|old| *old = candidate.packet);
                        let dimensions = [scene.width, scene.height];
                        if config.config.dimensions.get_untracked() != dimensions {
                            config.config.dimensions.revise(|old| *old = dimensions);
                        }
                    }
                    let bound_events = if let (Some(bound), Some(candidate)) =
                        (&observed.bound_plot, bound_plot)
                    {
                        bound.commit(candidate)
                    } else {
                        vec![]
                    };
                    if observed.data_pointer.get_untracked() != data_pointer {
                        observed.data_pointer.revise(|old| *old = data_pointer);
                    }
                    if let Some(drawings) = drawings {
                        observed.drawings.revise(|old| *old = drawings);
                    }
                    if let (Some(state), Some(frame)) = (&observed.semantic_labels, semantic_labels)
                    {
                        state.frame.revise(|old| *old = frame);
                    }
                    #[cfg(feature = "raster")]
                    if let Some(outlines) = outlines {
                        observed.outlines.revise(|old| *old = outlines);
                    }
                    #[cfg(feature = "raster")]
                    if let Some(paints) = paints {
                        observed.paints.revise(|old| *old = paints);
                    }
                    if let Some(layout) = layout {
                        observed.layout.revise(|old| *old = layout);
                    }
                    #[cfg(feature = "raster")]
                    if let (Some(raster), Some((frame, options))) =
                        (&observed.raster, raster_update)
                    {
                        raster.commit(frame, options);
                    }
                    if observed.scene.with(|old| old != &scene) {
                        *observed.group_opacities.borrow_mut() = scene.mark_group_opacities();
                        observed.scene.revise(|old| *old = scene);
                    }
                    if observed.paint_scene.with(|old| old != &paint_scene) {
                        observed.paint_scene.revise(|old| *old = paint_scene);
                    }
                    if let Some(labels) = data_labels
                        && observed.data_labels.with(|old| old != &labels)
                    {
                        observed.data_labels.revise(|old| *old = labels);
                    }
                    if let (Some(widgets), Some(rows)) = (&observed.plot_widgets, plot_widgets)
                        && widgets.rows.with(|old| old != &rows)
                    {
                        widgets.rows.revise(|old| *old = rows);
                    }
                    if let Some(hidden) = hidden
                        && observed.hidden.with(|old| old != &hidden)
                    {
                        observed.hidden.revise(|old| *old = hidden);
                    }
                    if let Some(selected) = selected
                        && observed.selected.get_untracked() != selected
                    {
                        observed.selected.revise(|old| *old = selected);
                    }
                    if let Some(hovered) = hovered
                        && observed.hovered.get_untracked() != hovered
                    {
                        observed.hovered.revise(|old| *old = hovered);
                    }
                    if let (Some(source), Some(ids)) = (observed.tooltip_marks, tooltip)
                        && source.with(|old| old != &ids)
                    {
                        source.revise(|old| *old = ids);
                    }
                    observed.reconcile();
                    observed.bound_events(bound_events);
                }),
                Err(error) => {
                    if !observed.selection.alive.get() {
                        return;
                    }
                    if has_rejection {
                        rejection.emit(error);
                    } else {
                        scope.report_error(ui_error(error));
                    }
                }
            }
        });
        let show_data = self.data_controls;
        let basic_data_controls = self.basic_data_controls;
        let show_legend = self.legend;
        let show_tooltip = self.tooltip;
        if show_tooltip {
            let observed = state.clone();
            scope.structural(move || {
                let result = observed
                    .compute_tooltip()
                    .and_then(|text| observed.place_tooltip(&text).map(|rect| (text, rect)));
                glory::core::untracked(|| match result {
                    Ok((text, rect)) => {
                        if !observed.selection.alive.get() {
                            return;
                        }
                        if observed.tooltip_rect.get_untracked() != rect {
                            observed.tooltip_rect.revise(|old| *old = rect);
                        }
                        if observed.tooltip_value.get_untracked() != text {
                            observed.tooltip_value.revise(|old| *old = text);
                        }
                    }
                    Err(error) => {
                        if !observed.selection.alive.get() {
                            return;
                        }
                        // Clear on formatting failure so another datum's old
                        // tooltip cannot be mistaken for the current identity.
                        if !observed.tooltip_value.get_untracked().is_empty() {
                            observed.tooltip_value.revise(String::clear);
                        }
                        if observed.has_rejection {
                            observed.rejection.emit(error);
                        } else if let Some(scope) = observed.owner.upgrade() {
                            scope.report_error(ui_error(error));
                        }
                    }
                });
            });
        }
        let accepted = state.scene;
        glory::ui::view()
            .class("kartis-chart")
            .style(Style::new().position(Position::Relative))
            .name(scope.bond(move || accepted.with(|s| s.title.clone())))
            .fill(glory::ui::widget(move |scope| {
                #[cfg(feature = "raster")]
                if let Some(raster) = &state.raster {
                    let frame = raster.frame;
                    let scene = state.scene;
                    let image = glory::ui::image()
                        .class("kartis-raster-image")
                        .source(scope.bond(move || frame.with(|frame| frame.source.clone())))
                        .alt(scope.bond(move || scene.with(|scene| scene.title.clone())))
                        .style(scope.bond(move || {
                            frame.with(|frame| {
                                Style::new()
                                    .width(Length::Px(frame.width))
                                    .height(Length::Px(frame.height))
                            })
                        }));
                    if state.bound_plot.is_some() {
                        let pointer = state.clone();
                        glory::ui::view()
                            .class("kartis-raster-plot")
                            .style(scope.bond(move || {
                                frame.with(|frame| Style {
                                    overflow_x: Some(Overflow::Hidden),
                                    overflow_y: Some(Overflow::Hidden),
                                    ..Style::new()
                                        .position(Position::Relative)
                                        .width(Length::Px(frame.width))
                                        .height(Length::Px(frame.height))
                                })
                            }))
                            .on(EventKind::PlotPointer, move |event| {
                                if let Event::PlotPointer { pointer: point, .. } = event {
                                    pointer.bound_pointer(point);
                                }
                            })
                            .fill(
                                image.style(
                                    Style::new()
                                        .position(Position::Absolute)
                                        .left(Length::Px(0.))
                                        .top(Length::Px(0.))
                                        .width(Length::Percent(100.))
                                        .height(Length::Percent(100.)),
                                ),
                            )
                            .show_in(scope)?;
                    } else {
                        image.show_in(scope)?;
                    }
                }
                if show_plot {
                    build_plot(scope, state.clone())?;
                }
                if let Some(widgets) = &state.plot_widgets {
                    let scene = state.scene;
                    let reject = state.rejection.clone();
                    let has_rejection = state.has_rejection;
                    let weak = scope.downgrade();
                    widgets.clone().show(
                        scope,
                        scope.bond(move || scene.with(|s| plot_widgets::overlay_style(s, false))),
                        state.hidden,
                        move |error| {
                            if has_rejection {
                                reject.emit(error);
                            } else if let Some(scope) = weak.upgrade() {
                                scope.report_error(ui_error(error));
                            }
                        },
                    )?;
                }
                if show_legend {
                    build_legend(scope, state.clone())?;
                }
                if show_data {
                    build_data(scope, state.clone(), basic_data_controls)?;
                }
                if show_tooltip {
                    build_tooltip(scope, state.clone())?;
                }
                Ok(())
            }))
            .show_in(scope)
    }
}
fn drawing(mark: &Mark, scene: &Scene) -> GraphicProps {
    GraphicProps {
        width: scene.width,
        height: scene.height,
        commands: mark.geometry.commands(),
        fill: mark.fill,
        stroke: mark.stroke,
        alternative: None,
        ..Default::default()
    }
}
type PreparedDrawings = BTreeMap<String, Vec<GraphicProps>>;
/// Pack whole stroked subpaths. Splitting inside one painted run would change
/// its joins/caps, so that run must satisfy Glory's own command admission.
fn graphic_chunks(
    commands: Vec<glory::render::PathCommand>,
) -> Result<Vec<Vec<glory::render::PathCommand>>> {
    use glory::render::PathCommand;
    let mut contours = vec![];
    let mut contour = vec![];
    for command in commands {
        if matches!(command, PathCommand::Move(_)) && !contour.is_empty() {
            contours.push(std::mem::take(&mut contour));
        }
        let closed = matches!(command, PathCommand::Close);
        contour.push(command);
        if closed {
            contours.push(std::mem::take(&mut contour));
        }
    }
    if !contour.is_empty() {
        contours.push(contour);
    }
    let mut chunks = vec![];
    let mut chunk = vec![];
    for contour in contours {
        ensure(
            contour.len() <= MAX_GRAPHIC_COMMANDS,
            "one dashed stroke run exceeds Glory's 256 command budget; use SVG/Canvas/raster",
        )?;
        if chunk.len() + contour.len() > MAX_GRAPHIC_COMMANDS {
            chunks.push(std::mem::take(&mut chunk));
        }
        chunk.extend(contour);
    }
    if !chunk.is_empty() {
        chunks.push(chunk);
    }
    Ok(chunks)
}
fn prepare_drawings(scene: &Scene) -> Result<PreparedDrawings> {
    use crate::stroke_dash::{StrokeDashLimits, dash_dot_geometry, dash_geometry_detailed};
    let mut output = BTreeMap::new();
    let mut prepared_commands = 0usize;
    for mark in &scene.marks {
        let (Some(dash), Some(stroke)) = (&mark.stroke_dash, mark.stroke) else {
            continue;
        };
        if dash.normalized()?.segments.is_empty() {
            continue;
        }
        let detailed = dash_geometry_detailed(&mark.geometry, dash, &StrokeDashLimits::default())?;
        let mut parts = vec![];
        if mark.fill.is_some() {
            let mut fill = drawing(mark, scene);
            fill.stroke = None;
            parts.push(fill);
        }
        for commands in graphic_chunks(detailed.stroke_geometry().commands())? {
            parts.push(GraphicProps {
                width: scene.width,
                height: scene.height,
                commands,
                fill: None,
                stroke: Some(stroke),
                ..Default::default()
            });
        }
        let mut dots = vec![];
        for dot in &detailed.dots {
            let footprint = dash_dot_geometry(dot, stroke.width, stroke.cap)?.commands();
            ensure(
                dots.len() + footprint.len() <= crate::stroke_dash::MAX_DASH_COMMANDS,
                "semantic dash footprints exceed the command budget; use SVG/Canvas/raster",
            )?;
            dots.extend(footprint);
        }
        for commands in graphic_chunks(dots)? {
            parts.push(GraphicProps {
                width: scene.width,
                height: scene.height,
                commands,
                fill: Some(stroke.color),
                stroke: None,
                ..Default::default()
            });
        }
        let ink_parts = parts.len() - usize::from(mark.fill.is_some());
        ensure(
            stroke.color.3 == 0 || stroke.color.3 == 255 || ink_parts <= 1,
            "semantic semi-transparent dashed stroke requires one ink Graphic; use SVG/Canvas/raster for overlapping cap/dot footprints",
        )?;
        prepared_commands += parts.iter().map(|p| p.commands.len()).sum::<usize>();
        ensure(
            prepared_commands <= crate::stroke_dash::MAX_DASH_COMMANDS,
            "semantic dash drawings exceed the command budget; use SVG/Canvas/raster",
        )?;
        output.insert(mark.id.clone(), parts);
    }
    Ok(output)
}
fn mark_style(mark: &Mark, scene: &Scene, active: Option<&str>) -> Style {
    let clip = mark.clip;
    let dim = active.is_some_and(|id| id != mark.id) && !mark.series.is_empty();
    Style::new()
        .position(Position::Absolute)
        .left(Length::Px(-clip.map_or(0., |c| c.x)))
        .top(Length::Px(-clip.map_or(0., |c| c.y)))
        .width(Length::Px(scene.width))
        .height(Length::Px(scene.height))
        .opacity(mark.opacity * if dim { 0.35 } else { 1. })
        .pointer_events(PointerEvents::None)
}
fn clip_style(mark: &Mark, scene: &Scene, hidden: &BTreeSet<String>) -> Style {
    let clip = mark.clip.unwrap_or(ClipRect {
        x: 0.,
        y: 0.,
        width: scene.width,
        height: scene.height,
    });
    Style {
        display: Some(if hidden.contains(&mark.series) {
            Display::None
        } else {
            Display::Block
        }),
        overflow_x: Some(Overflow::Hidden),
        overflow_y: Some(Overflow::Hidden),
        ..Style::new()
            .position(Position::Absolute)
            .left(Length::Px(clip.x))
            .top(Length::Px(clip.y))
            .width(Length::Px(clip.width))
            .height(Length::Px(clip.height))
            .pointer_events(PointerEvents::None)
    }
}
fn label_container(label: &Label, scene: &Scene) -> Style {
    let (left, justify) = match label.anchor {
        TextAnchor::Start => (label.x, Justify::Start),
        TextAnchor::Middle => (label.x - scene.width / 2., Justify::Center),
        TextAnchor::End => (label.x - scene.width, Justify::End),
    };
    Style {
        justify: Some(justify),
        ..Style::new()
            .position(Position::Absolute)
            .left(Length::Px(left))
            .top(Length::Px(label.y - label.size * 0.8))
            .width(Length::Px(scene.width))
            .display(Display::Flex)
            .pointer_events(PointerEvents::None)
    }
}
#[derive(Clone, Debug, PartialEq)]
enum PaintRow {
    Mark(Box<Mark>),
    Group {
        group: MarkOpacityGroup,
        marks: Vec<Mark>,
    },
    Label {
        index: usize,
        label: Label,
        series: Option<String>,
        clip: Option<ClipRect>,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum PaintKey {
    Mark(String),
    Group(String),
    Label(usize, bool),
}
impl PaintRow {
    fn key(&self) -> PaintKey {
        match self {
            Self::Mark(mark) => PaintKey::Mark(mark.id.clone()),
            Self::Group { group, .. } => PaintKey::Group(group.id.clone()),
            Self::Label { index, clip, .. } => PaintKey::Label(*index, clip.is_some()),
        }
    }
}
// The empty profile produces exactly the original flat Mark/Label rows.
// Nonempty groups have already passed Scene's disjoint contiguous admission.
fn paint_rows(scene: &Scene) -> Vec<PaintRow> {
    let first = scene
        .opacity_groups
        .iter()
        .map(|group| (group.members[0].as_str(), group))
        .collect::<BTreeMap<_, _>>();
    let members = scene
        .opacity_groups
        .iter()
        .flat_map(|group| group.members.iter().map(String::as_str))
        .collect::<BTreeSet<_>>();
    let marks = scene
        .marks
        .iter()
        .map(|mark| (mark.id.as_str(), mark))
        .collect::<BTreeMap<_, _>>();
    scene
        .draw_items()
        .into_iter()
        .filter_map(|item| match item {
            DrawItem::Mark(index) => {
                let mark = &scene.marks[index];
                if let Some(group) = first.get(mark.id.as_str()) {
                    Some(PaintRow::Group {
                        group: (**group).clone(),
                        marks: group
                            .members
                            .iter()
                            .map(|id| (*marks[id.as_str()]).clone())
                            .collect(),
                    })
                } else if members.contains(mark.id.as_str()) {
                    None
                } else {
                    Some(PaintRow::Mark(Box::new(mark.clone())))
                }
            }
            DrawItem::Label(index) => Some(PaintRow::Label {
                index,
                label: scene.labels[index].clone(),
                series: scene.label_series.get(&index).cloned(),
                clip: rectangular_clip(scene, index),
            }),
        })
        .collect()
}
fn build_opacity_group(scope: &Scope, item: Cage<PaintRow>, state: State) -> glory::ui::Result {
    let scene = state.scene;
    let rows = state.clone();
    glory::ui::view()
        .class("kartis-opacity-group kartis-paint-item")
        .style(scope.bond(move || {
            item.with(|row| {
                let PaintRow::Group { group, .. } = row else {
                    unreachable!("paint row key preserves group variant")
                };
                scene.with(|scene| {
                    Style::new()
                        .position(Position::Absolute)
                        .left(Length::Px(0.))
                        .top(Length::Px(0.))
                        .width(Length::Px(scene.width))
                        .height(Length::Px(scene.height))
                        .opacity(group.opacity)
                        .pointer_events(PointerEvents::None)
                })
            })
        }))
        .fill(glory::ui::widget(move |scope| {
            let marks = scope.bond(move || {
                item.with(|row| match row {
                    PaintRow::Group { marks, .. } => marks.clone(),
                    _ => unreachable!("paint row key preserves group variant"),
                })
            });
            let state = rows.clone();
            glory::ui::each(marks)
                .key(|mark| mark.id.clone())
                .children(move |entry| {
                    let mark = entry.value;
                    let state = state.clone();
                    glory::ui::widget(move |scope| {
                        let mark = scope.bond(move || mark.get());
                        build_mark(scope, mark, state.clone())
                    })
                })
                .show_in(scope)
        }))
        .show_in(scope)
}
fn build_plot(scope: &Scope, state: State) -> glory::ui::Result {
    let scene = state.paint_scene;
    let plot = glory::ui::view()
        .class("kartis-plot")
        .style(scope.bond(move || {
            scene.with(|s| Style {
                background: Some(s.background),
                overflow_x: Some(Overflow::Hidden),
                overflow_y: Some(Overflow::Hidden),
                ..Style::new()
                    .position(Position::Relative)
                    .width(Length::Px(s.width))
                    .height(Length::Px(s.height))
            })
        }));
    let plot = if state.bound_plot.is_some() {
        let pointer = state.clone();
        plot.on(EventKind::PlotPointer, move |event| {
            if let Event::PlotPointer { pointer: point, .. } = event {
                pointer.bound_pointer(point);
            }
        })
    } else {
        plot
    };
    plot.fill(glory::ui::widget(move |scope| {
        let items = scope.bond(move || scene.with(paint_rows));
        let rows = state.clone();
        glory::ui::each(items)
            .key(PaintRow::key)
            .children(move |entry| {
                let item = entry.value;
                let is_mark = matches!(item.get_untracked(), PaintRow::Mark(_));
                let is_group = matches!(item.get_untracked(), PaintRow::Group { .. });
                let state = rows.clone();
                glory::ui::widget(move |scope| {
                    if is_group {
                        build_opacity_group(scope, item, state.clone())
                    } else if is_mark {
                        let mark = scope.bond(move || {
                            item.with(|row| match row {
                                PaintRow::Mark(mark) => mark.as_ref().clone(),
                                _ => unreachable!("paint row key preserves variant"),
                            })
                        });
                        build_mark(scope, mark, state.clone())
                    } else {
                        let clipped =
                            item.with(|row| matches!(row, PaintRow::Label { clip: Some(_), .. }));
                        let label = scope.bond(move || {
                            item.with(|row| match row {
                                PaintRow::Label {
                                    index,
                                    label,
                                    series,
                                    ..
                                } => (*index, label.clone(), series.clone()),
                                _ => unreachable!("paint row key preserves variant"),
                            })
                        });
                        build_label(scope, label, state.clone(), clipped)
                    }
                })
            })
            .show_in(scope)
    }))
    .show_in(scope)
}
fn build_mark(scope: &Scope, mark: Bond<Mark>, state: State) -> glory::ui::Result {
    let scene = state.scene;
    let hidden = state.hidden;
    let active = state.clone();
    let drawings = state.drawings;
    let dashed = state.clone();
    glory::ui::view()
        .class("kartis-mark-clip kartis-paint-item")
        .style(
            scope.bond(move || mark.with(|m| scene.with(|s| hidden.with(|h| clip_style(m, s, h))))),
        )
        .fill(
            glory::ui::graphic()
                .class("kartis-mark")
                .drawing(scope.bond(move || mark.with(|m| scene.with(|s| drawing(m, s)))))
                .style(scope.bond(move || {
                    let active = active.highlight.then(|| active.active()).flatten();
                    mark.with(|m| {
                        scene.with(|s| {
                            let mut style = mark_style(m, s, active.as_deref());
                            if drawings.with(|parts| parts.contains_key(&m.id)) {
                                style.display = Some(Display::None);
                            }
                            style
                        })
                    })
                })),
        )
        .fill(
            glory::ui::view()
                .class("kartis-mark-parts")
                .style(scope.bond(move || {
                    let active = dashed.highlight.then(|| dashed.active()).flatten();
                    mark.with(|m| {
                        scene.with(|s| {
                            let mut style = mark_style(m, s, active.as_deref());
                            style.display =
                                Some(if drawings.with(|parts| parts.contains_key(&m.id)) {
                                    Display::Block
                                } else {
                                    Display::None
                                });
                            style
                        })
                    })
                }))
                .fill(glory::ui::widget(move |scope| {
                    let parts = scope.bond(move || {
                        mark.with(|m| {
                            drawings.with(|all| {
                                all.get(&m.id)
                                    .cloned()
                                    .unwrap_or_default()
                                    .into_iter()
                                    .enumerate()
                                    .collect::<Vec<_>>()
                            })
                        })
                    });
                    glory::ui::each(parts)
                        .key(|(index, _)| *index)
                        .children(move |entry| {
                            let part = entry.value;
                            glory::ui::widget(move |scope| {
                                glory::ui::graphic()
                                    .class("kartis-mark kartis-dash-part")
                                    .drawing(
                                        scope.bond(move || {
                                            part.with(|(_, drawing)| drawing.clone())
                                        }),
                                    )
                                    .style(scope.bond(move || {
                                        scene.with(|s| {
                                            Style::new()
                                                .position(Position::Absolute)
                                                .left(Length::Px(0.))
                                                .top(Length::Px(0.))
                                                .width(Length::Px(s.width))
                                                .height(Length::Px(s.height))
                                                .pointer_events(PointerEvents::None)
                                        })
                                    }))
                                    .show_in(scope)
                            })
                        })
                        .show_in(scope)
                })),
        )
        .show_in(scope)
}
fn build_label(
    scope: &Scope,
    label: Bond<(usize, Label, Option<String>)>,
    state: State,
    clipped: bool,
) -> glory::ui::Result {
    if !clipped {
        return build_label_contents(scope, label, state, false);
    }
    let inside = state.clone();
    let hidden = state.hidden;
    glory::ui::view()
        .class("kartis-label-clip kartis-paint-item")
        .style(scope.bond(move || {
            label.with(|(index, _, series)| {
                state.paint_scene.with(|scene| {
                    // On removal, the keyed list replaces this wrapper. A
                    // transient read of the new Scene still has valid bounds.
                    let rect = rectangular_clip(scene, *index).unwrap_or(ClipRect {
                        x: 0.,
                        y: 0.,
                        width: scene.width,
                        height: scene.height,
                    });
                    Style {
                        display: Some(
                            if series
                                .as_ref()
                                .is_some_and(|id| hidden.with(|h| h.contains(id)))
                            {
                                Display::None
                            } else {
                                Display::Block
                            },
                        ),
                        overflow_x: Some(Overflow::Hidden),
                        overflow_y: Some(Overflow::Hidden),
                        ..Style::new()
                            .position(Position::Absolute)
                            .left(Length::Px(rect.x))
                            .top(Length::Px(rect.y))
                            .width(Length::Px(rect.width))
                            .height(Length::Px(rect.height))
                            .pointer_events(PointerEvents::None)
                    }
                })
            })
        }))
        .fill(glory::ui::widget(move |scope| {
            build_label_contents(scope, label, inside.clone(), true)
        }))
        .show_in(scope)
}
fn build_label_contents(
    scope: &Scope,
    label: Bond<(usize, Label, Option<String>)>,
    state: State,
    clipped: bool,
) -> glory::ui::Result {
    #[cfg(feature = "raster")]
    if state.outlined.is_some() || state.painted.is_some() {
        return build_outline_label(scope, label, state, clipped);
    }
    let text_state = state.clone();
    let content_state = state.clone();
    let hidden = state.hidden;
    glory::ui::view()
        .class(if clipped {
            "kartis-label"
        } else {
            "kartis-label kartis-paint-item"
        })
        .style(scope.bond(move || {
            label.with(|(index, label, series)| {
                let mut style = relative_to_clip(
                    *index,
                    ordinary_container(*index, label, &state),
                    &state,
                    clipped,
                );
                if series
                    .as_ref()
                    .is_some_and(|id| hidden.with(|h| h.contains(id)))
                {
                    style.display = Some(Display::None);
                }
                style
            })
        }))
        .fill(
            glory::ui::text()
                .content(scope.bond(move || {
                    label.with(|(index, label, _)| ordinary_content(*index, label, &content_state))
                }))
                .style(scope.bond(move || {
                    label.with(|(index, label, _)| ordinary_text_style(*index, label, &text_state))
                })),
        )
        .show_in(scope)
}
#[cfg(feature = "raster")]
fn build_outline_label(
    scope: &Scope,
    label: Bond<(usize, Label, Option<String>)>,
    state: State,
    clipped: bool,
) -> glory::ui::Result {
    let scene = state.paint_scene;
    let text_state = state.clone();
    let content_state = state.clone();
    let hidden = state.hidden;
    let outlines = state.outlines;
    let paints = state.paints;
    let painted = state.painted.is_some();
    let glyphs = scope.bond(move || {
        label.with(|(index, label, _)| {
            if painted {
                return paints.with(|cache| {
                    cache
                        .labels
                        .get(index)
                        .map(|text| {
                            text.layers
                                .iter()
                                .map(|layer| {
                                    (
                                        ((layer.glyph as u64) << 32) | layer.layer as u64,
                                        layer.commands.clone(),
                                        layer.color,
                                    )
                                })
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default()
                });
            }
            outlines.with(|cache| {
                cache
                    .labels
                    .get(index)
                    .map(|text| {
                        text.glyphs
                            .iter()
                            .enumerate()
                            .map(|(slot, commands)| {
                                ((slot as u64) << 32, commands.clone(), label.color)
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default()
            })
        })
    });
    glory::ui::view()
        .class(if painted && clipped {
            "kartis-label kartis-outline-label kartis-painted-label"
        } else if painted {
            "kartis-label kartis-paint-item kartis-outline-label kartis-painted-label"
        } else if clipped {
            "kartis-label kartis-outline-label"
        } else {
            "kartis-label kartis-paint-item kartis-outline-label"
        })
        .style(scope.bond(move || {
            label.with(|(index, label, series)| {
                let outlined = if painted {
                    paints.with(|cache| cache.labels.contains_key(index))
                } else {
                    outlines.with(|cache| cache.labels.contains_key(index))
                };
                let mut style = if outlined {
                    scene.with(|s| {
                        Style::new()
                            .position(Position::Absolute)
                            .left(Length::Px(0.))
                            .top(Length::Px(0.))
                            .width(Length::Px(s.width))
                            .height(Length::Px(s.height))
                            .pointer_events(PointerEvents::None)
                    })
                } else {
                    ordinary_container(*index, label, &state)
                };
                style = relative_to_clip(*index, style, &state, clipped);
                if series
                    .as_ref()
                    .is_some_and(|id| hidden.with(|h| h.contains(id)))
                {
                    style.display = Some(Display::None);
                }
                style
            })
        }))
        .fill((
            glory::ui::text()
                .class("kartis-outline-text")
                .content(scope.bond(move || {
                    label.with(|(index, label, _)| ordinary_content(*index, label, &content_state))
                }))
                .style(scope.bond(move || {
                    label.with(|(index, label, _)| {
                        if if painted {
                            paints.with(|cache| cache.labels.contains_key(index))
                        } else {
                            outlines.with(|cache| cache.labels.contains_key(index))
                        } {
                            Style {
                                color: Some(glory::style::Color(0, 0, 0, 0)),
                                font_size: Some(1.),
                                line_height: Some(1.),
                                overflow_x: Some(Overflow::Hidden),
                                overflow_y: Some(Overflow::Hidden),
                                ..Style::new()
                                    .position(Position::Absolute)
                                    .width(Length::Px(1.))
                                    .height(Length::Px(1.))
                            }
                        } else {
                            ordinary_text_style(*index, label, &text_state)
                        }
                    })
                })),
            glory::ui::each(glyphs)
                .key(|glyph| glyph.0)
                .children(move |entry| {
                    let glyph = entry.value;
                    glory::ui::widget(move |scope| {
                        glory::ui::graphic()
                            .class(if painted {
                                "kartis-outline-glyph kartis-painted-glyph"
                            } else {
                                "kartis-outline-glyph"
                            })
                            .drawing(scope.bond(move || {
                                glyph.with(|(_, commands, color)| {
                                    scene.with(|scene| GraphicProps {
                                        width: scene.width,
                                        height: scene.height,
                                        commands: commands.clone(),
                                        fill: Some(*color),
                                        alternative: None,
                                        ..Default::default()
                                    })
                                })
                            }))
                            .style(scope.bond(move || {
                                scene.with(|s| {
                                    Style::new()
                                        .position(Position::Absolute)
                                        .left(Length::Px(0.))
                                        .top(Length::Px(0.))
                                        .width(Length::Px(s.width))
                                        .height(Length::Px(s.height))
                                        .pointer_events(PointerEvents::None)
                                })
                            }))
                            .show_in(scope)
                    })
                }),
        ))
        .show_in(scope)
}
fn build_legend(scope: &Scope, state: State) -> glory::ui::Result {
    let scene = state.scene;
    // Include role in the values, not only the key closure: an otherwise
    // identical LegendItem must dirty the list when its role changes.
    let legend = scope.bond(move || {
        scene.with(|s| {
            s.legend
                .iter()
                .map(|item| {
                    (
                        item.clone(),
                        s.legend_role(&item.id).expect("accepted legend role"),
                        s.legend_target(&item.id).expect("accepted legend target"),
                    )
                })
                .collect::<Vec<_>>()
        })
    });
    glory::ui::view()
        .class("kartis-legend")
        .name("Chart legend".to_string())
        .style(if let Some(layout) = &state.legend_layout {
            Style::new()
                .position(Position::Relative)
                .width(Length::Px(layout.bounds.width))
                .height(Length::Px(layout.bounds.height))
        } else {
            legend_style(&state.legend_options)
        })
        .fill(
            glory::ui::each(legend)
                .key(|row| (row.0.id.clone(), row.1 == LegendRole::Series, row.2.clone()))
                .children(move |entry| {
                    let row = entry.value;
                    let state = state.clone();
                    glory::ui::widget(move |scope| {
                        let item = scope.bond(move || row.with(|r| r.0.clone()));
                        let role = row.with(|r| r.1);
                        let target = scope.bond(move || row.with(|r| r.2.clone()));
                        let expected_target = row.with(|r| r.2.clone());
                        let actionable = role == LegendRole::Series || state.has_legend_activate;
                        let hidden = state.hidden;
                        let toggle = state.clone();
                        let label_options = state.legend_options.clone();
                        let symbol_options = state.legend_options.clone();
                        let name_options = state.legend_options.clone();
                        let placed = state.clone();
                        let default_style = legend_item_style(&state.legend_options);
                        let custom = state.legend_renderer.configured();
                        let context = || {
                            let label_options = state.legend_options.clone();
                            LegendRenderContext {
                                item: scope.bond(move || item.get()),
                                hidden: scope.bond(move || {
                                    role == LegendRole::Series
                                        && target.with(|t| hidden.with(|h| matches!(t, LegendTarget::Series(id) if h.contains(id))))
                                }),
                                role: scope.bond(move || row.with(|r| r.1)),
                                target,
                                label: scope.bond(move || {
                                    item.with(|i| {
                                        legend_label(i, &label_options)
                                            .expect("accepted legend label")
                                    })
                                }),
                            }
                        };
                        let mut custom_item = if custom {
                            Some(state.legend_renderer.call(context()).map_err(ui_error)?)
                        } else {
                            None
                        };
                        let mut custom_symbol = if state.symbol_renderer.configured() {
                            Some(state.symbol_renderer.call(context()).map_err(ui_error)?)
                        } else {
                            None
                        };
                        let style = scope.bond(move || {
                            let mut style = default_style.clone();
                            let font = placed.scene.with(|scene| {
                                scene.typography.legend_font(
                                    &placed.legend_options.text.font,
                                    placed.legend_options.text.weight,
                                )
                            });
                            style.font_family = Some(font.family);
                            style.font_weight = Some(font.weight);
                            if let Some(options) = &placed.legend_layout
                                && let Some(rect) = placed
                                    .layout
                                    .with(|l| item.with(|i| l.legend.get(&i.id).copied()))
                            {
                                style.position = Some(Position::Absolute);
                                style.left = Some(Length::Px(rect.x - options.bounds.x));
                                style.top = Some(Length::Px(rect.y - options.bounds.y));
                                style.width = Some(Length::Px(rect.width));
                                style.height = Some(Length::Px(rect.height));
                            }
                            style
                        });
                        let display_label = scope.bond(move || {
                            item.with(|i| {
                                legend_control_label(
                                    i,
                                    role,
                                    role == LegendRole::Series
                                        && target.with(|t| hidden.with(|h| matches!(t, LegendTarget::Series(id) if h.contains(id)))),
                                    &label_options,
                                )
                                .expect("accepted legend label")
                            })
                        });
                        let name = scope.bond(move || {
                            item.with(|i| {
                                legend_control_label(
                                    i,
                                    role,
                                    role == LegendRole::Series
                                        && target.with(|t| hidden.with(|h| matches!(t, LegendTarget::Series(id) if h.contains(id)))),
                                    &name_options,
                                )
                                .expect("accepted legend label")
                            })
                        });
                        let content = glory::ui::widget(move |scope| {
                            if let Some(custom_item) = custom_item.as_mut() {
                                custom_item.build(scope)?;
                            }
                            if let Some(custom_symbol) = custom_symbol.as_mut() {
                                return custom_symbol.build(scope);
                            }
                            if custom {
                                return Ok(());
                            }
                            let symbol_options = symbol_options.clone();
                            glory::ui::text()
                                .class("kartis-legend-symbol")
                                .content(String::new())
                                .style(scope.bond(move || {
                                    item.with(|i| {
                                        legend_symbol_style(
                                            &symbol_options,
                                            i.color,
                                            role == LegendRole::Series
                                                && target.with(|t| hidden.with(|h| matches!(t, LegendTarget::Series(id) if h.contains(id)))),
                                        )
                                    })
                                }))
                                .show_in(scope)
                        });
                        if actionable {
                            glory::ui::button()
                                .class("kartis-legend-item")
                                .style(style)
                                .label(scope.bond(move || {
                                    if custom {
                                        String::new()
                                    } else {
                                        display_label.get()
                                    }
                                }))
                                .name(name)
                                .semantics(scope.bond(move || {
                                    Some(Box::new(Semantics {
                                        pressed: (role == LegendRole::Series).then(|| {
                                            !target.with(|t| hidden.with(|h| matches!(t, LegendTarget::Series(id) if h.contains(id))))
                                        }),
                                        ..Default::default()
                                    }))
                                }))
                                .on(EventKind::Activate, move |_| {
                                    toggle.activate_legend(&item.get_untracked().id, &expected_target)
                                })
                                .fill(content)
                                .show_in(scope)
                        } else {
                            glory::ui::view()
                                .class("kartis-legend-item")
                                .style(style)
                                .name(name)
                                .fill((
                                    glory::ui::text().content(scope.bond(move || {
                                        if custom {
                                            String::new()
                                        } else {
                                            display_label.get()
                                        }
                                    })),
                                    content,
                                ))
                                .show_in(scope)
                        }
                    })
                }),
        )
        .show_in(scope)
}
fn build_tooltip(scope: &Scope, state: State) -> glory::ui::Result {
    if state.tooltip_renderer.configured() {
        let marks = state.clone();
        let text = state.clone();
        let widget = state
            .tooltip_renderer
            .call(TooltipRenderContext {
                marks: scope.bond(move || marks.tooltip_data()),
                text: scope.bond(move || text.tooltip()),
            })
            .map_err(ui_error)?;
        return glory::ui::view()
            .class("kartis-tooltip")
            .style(scope.bond(move || tooltip_position_style(&state)))
            .fill(widget)
            .show_in(scope);
    }
    let text_state = state.clone();
    glory::ui::text()
        .class("kartis-tooltip")
        .content(scope.bond(move || text_state.tooltip()))
        .style(scope.bond(move || tooltip_position_style(&state)))
        .show_in(scope)
}
fn tooltip_position_style(state: &State) -> Style {
    let mut style = tooltip_style(&state.tooltip_options, !state.tooltip().is_empty());
    let font = state.scene.with(|scene| {
        scene.typography.tooltip_font(
            &state.tooltip_options.text.font,
            state.tooltip_options.text.weight,
        )
    });
    style.font_family = Some(font.family);
    style.font_weight = Some(font.weight);
    if let Some(rect) = state.tooltip_rect.get() {
        style.position = Some(Position::Absolute);
        style.left = Some(Length::Px(rect.x));
        style.top = Some(Length::Px(rect.y));
        style.width = Some(Length::Px(rect.width));
        style.height = Some(Length::Px(rect.height));
        style.overflow_x = Some(Overflow::Auto);
        style.overflow_y = Some(Overflow::Auto);
    }
    style
}
fn build_data(scope: &Scope, state: State, basic: bool) -> glory::ui::Result {
    let source = state.clone();
    let marks = scope.bond(move || source.data_marks());
    let nav_state = state.clone();
    let weak = scope.downgrade();
    let rows_state = state.clone();
    let view = glory::ui::view()
        .class("kartis-data")
        .name("Chart data".to_string());
    let view = if basic {
        view
    } else {
        view.semantics(Some(Box::new(Semantics {
            navigation: Some(Orientation::Vertical),
            ..Default::default()
        })))
        .on(EventKind::Navigate, move |event| {
            if let Event::Navigate { from, command } = event
                && let Some((mark, node)) = nav_state.destination(from, command)
            {
                if let Some(scope) = weak.upgrade() {
                    match scope.action(Action::Focus(node)) {
                        Ok(())
                        | Err(glory::ui::Error::Render(glory::render::Error::Unsupported(_)))
                        | Err(glory::ui::Error::Gone) => {}
                        Err(error) => scope.report_error(error),
                    }
                }
                nav_state.navigate.emit(mark);
            }
        })
    };
    view.fill(
        glory::ui::each(marks)
            .key(|m| m.id.clone())
            .children(move |entry| {
                let mark = entry.value;
                let state = rows_state.clone();
                glory::ui::widget(move |scope| {
                    let id = mark.get_untracked().id;
                    let reference = Rc::new(scope.cage(None));
                    state
                        .registry
                        .borrow_mut()
                        .insert(id.clone(), reference.clone());
                    let cleanup = state.registry.clone();
                    let clean_id = id.clone();
                    let clean_ref = reference.clone();
                    scope.on_cleanup(move || {
                        let mut map = cleanup.borrow_mut();
                        if map
                            .get(&clean_id)
                            .is_some_and(|value| Rc::ptr_eq(value, &clean_ref))
                        {
                            map.remove(&clean_id);
                        }
                    })?;
                    let active = state.clone();
                    let selection = state.clone();
                    let pointer = state.clone();
                    let focus = state.clone();
                    let labels = state.data_labels;
                    let activate_id = id.clone();
                    let hover_id = id.clone();
                    let focus_id = id;
                    let button = glory::ui::button()
                        .class("kartis-datum")
                        .focus(FocusMode::TabStop)
                        .node_ref(*reference)
                        .label(scope.bond(move || {
                            mark.with(|m| {
                                labels
                                    .with(|labels| labels.get(&m.id).cloned())
                                    .unwrap_or_else(|| format!("{}: {}", m.label, m.value))
                            })
                        }))
                        .semantics(scope.bond(move || {
                            mark.with(|m| {
                                Some(Box::new(Semantics {
                                    pressed: Some(active.selected.get().as_deref() == Some(&m.id)),
                                    navigation_item: !basic,
                                    ..Default::default()
                                }))
                            })
                        }))
                        .on(EventKind::Activate, move |_| {
                            selection.activate(&activate_id)
                        })
                        .on(EventKind::FocusChanged, move |event| {
                            if let Event::FocusChanged(focused) = event {
                                focus.focus(&focus_id, focused)
                            }
                        });
                    let button = if state.data_pointer_style {
                        let enabled = state.data_pointer;
                        button.style(scope.bond(move || {
                            if enabled.get() {
                                Style::new()
                            } else {
                                Style::new().pointer_events(PointerEvents::None)
                            }
                        }))
                    } else {
                        button
                    };
                    let button = if basic {
                        button
                    } else {
                        button.on(EventKind::Pointer, move |event| {
                            if let Event::Pointer { phase, .. } = event {
                                pointer.hover(&hover_id, phase != PointerPhase::Leave)
                            }
                        })
                    };
                    button.show_in(scope)
                })
            }),
    )
    .show_in(scope)
}
