//! Source-order renderer planes and native, pixel-shaped pointer targets.
use glory::render::{Change, NodeId, Patch, Semantics};
use glory::style::{Length, Overflow, PointerEvents, Position, Style};
use glory::{Scope, Widget};

use super::*;
use crate::widget::plot_widgets::composition::{CompositionPlacement, CompositionRetirement};
use crate::widget_hit_mask::{AlphaRect, alpha_rectangles};

const MAX_PIXELS: u64 = 67_108_864;
const MAX_HIT_RECTS: usize = 200_000;
const MAX_PATTERN_PIXELS: u64 = 16_777_216;
const MAX_PATTERN_OPERATIONS: usize = 268_435_456;

#[derive(Default)]
struct Budget {
    pixels: Cell<u64>,
    rects: Cell<usize>,
    alive: Cell<bool>,
}
struct Lease {
    budget: Rc<Budget>,
    pixels: u64,
    rects: usize,
}
impl Lease {
    fn new(budget: Rc<Budget>) -> Self {
        Self {
            budget,
            pixels: 0,
            rects: 0,
        }
    }
    fn pixels(&mut self, count: u64) -> Result<()> {
        scene::ensure(self.budget.alive.get(), "ordered paint owner retired")?;
        let next = self
            .budget
            .pixels
            .get()
            .checked_add(count)
            .ok_or_else(|| ChartError("ordered paint pixel overflow".into()))?;
        scene::ensure(
            next <= MAX_PIXELS,
            "ordered paint live/pending pixel budget exceeded",
        )?;
        self.budget.pixels.set(next);
        self.pixels += count;
        Ok(())
    }
    fn rects(&mut self, count: usize) -> Result<()> {
        let next = self
            .budget
            .rects
            .get()
            .checked_add(count)
            .ok_or_else(|| ChartError("ordered mask count overflow".into()))?;
        scene::ensure(
            next <= MAX_HIT_RECTS,
            "ordered paint hit-mask rectangle budget exceeded",
        )?;
        self.budget.rects.set(next);
        self.rects += count;
        Ok(())
    }
}
impl Drop for Lease {
    fn drop(&mut self) {
        self.budget
            .pixels
            .set(self.budget.pixels.get() - self.pixels);
        self.budget.rects.set(self.budget.rects.get() - self.rects);
    }
}

#[derive(Clone, PartialEq)]
pub(super) struct Request {
    pub scene: Scene,
    pub slots: SceneWidgetsLayout,
    pub replacements: BTreeSet<String>,
    pub modes: MarkBlends,
    pub hidden: BTreeSet<String>,
    pub view: ZoomTransform,
    pub ratio: f64,
    pub renderer: BrowserRenderer,
    pub tree_labels_passive: bool,
    /// Runtime helpers follow the authored plan and do not intercept input.
    pub helpers: Vec<Mark>,
    pub language: Option<String>,
    /// New literal SVG mode admits only actual source Mark ink as native
    /// targets. Passive Label ink remains visible but never forms a hit mask.
    pub source_hit_ids: Option<BTreeSet<String>>,
}

type WidgetTargets = BTreeMap<String, (NodeId, Option<NodeId>)>;

struct Plane {
    class: String,
    outer: Element,
    _svg: Option<svg::RetainedSvg>,
}
pub(super) struct PaintStage {
    request: Request,
    root: RefCell<Option<glory::ui::Mounted>>,
    native_root: RefCell<Option<Element>>,
    scope: Scope,
    root_id: NodeId,
    targets: WidgetTargets,
    planes: Vec<Plane>,
    _lease: Lease,
}
impl PaintStage {
    fn focused(&self) -> Option<HtmlElement> {
        let root = self.native_root.borrow();
        let root = root.as_ref()?;
        root.owner_document()?
            .active_element()
            .filter(|active| root.contains(Some(active.as_ref())))?
            .dyn_into::<HtmlElement>()
            .ok()
    }
    fn activate(&self) -> Result<()> {
        self.set_active(true)
    }
    fn set_active(&self, active: bool) -> Result<()> {
        self.scope
            .patch(
                self.root_id,
                Patch::Style(Change::Set(Box::new(root_style(
                    self.request.scene.background,
                    active,
                )))),
            )
            .map_err(host)?;
        self.scope
            .patch(
                self.root_id,
                Patch::Semantics(Change::Set(Some(Box::new(Semantics {
                    hidden: !active,
                    inert: !active,
                    ..Default::default()
                })))),
            )
            .map_err(host)
    }
}
pub(super) struct Placement {
    stage: Rc<PaintStage>,
    previous: Option<Rc<PaintStage>>,
    widgets: CompositionPlacement,
    budget: Rc<Budget>,
    focus: Option<HtmlElement>,
    finished: Cell<bool>,
}
pub(super) struct Retirement {
    _widgets: CompositionRetirement,
    _paint: Option<Rc<PaintStage>>,
}
impl Placement {
    pub(super) fn validate(&self) -> Result<()> {
        scene::ensure(
            self.budget.alive.get(),
            "ordered paint owner retired during placement",
        )?;
        self.widgets.validate()
    }
    pub(super) fn rollback(&self) -> Result<()> {
        if self.finished.replace(true) {
            return Ok(());
        }
        self.widgets.rollback()?;
        if self
            .previous
            .as_ref()
            .is_none_or(|old| !Rc::ptr_eq(old, &self.stage))
        {
            self.stage.set_active(false)?;
            if let Some(old) = &self.previous {
                old.activate()?;
            }
        }
        if let Some(focus) = &self.focus {
            restore_dom_focus(focus)?;
        }
        Ok(())
    }
}
impl Drop for Placement {
    fn drop(&mut self) {
        let _ = self.rollback();
    }
}
impl Drop for PaintStage {
    fn drop(&mut self) {
        let root = self.root.get_mut().take();
        drop(root);
    }
}

pub(super) struct OrderedPaint {
    budget: Rc<Budget>,
    active: RefCell<Option<Rc<PaintStage>>>,
    content: Element,
}
impl OrderedPaint {
    pub fn new(content: Element) -> Result<Self> {
        content
            .dyn_ref::<HtmlElement>()
            .ok_or_else(|| ChartError("ordered paint requires an HTML root".into()))?
            .style()
            .set_property("isolation", "isolate")
            .map_err(host)?;
        Ok(Self {
            budget: Rc::new(Budget {
                alive: Cell::new(true),
                ..Default::default()
            }),
            active: RefCell::new(None),
            content,
        })
    }
    pub fn has_frame(&self) -> bool {
        self.active.borrow().is_some()
    }
    pub fn clear(&self) {
        self.budget.alive.set(false);
        let old = self.active.borrow_mut().take();
        drop(old);
    }

    /// No Engine state borrow may span this call: UI observation can run user code.
    pub fn prepare(&self, app: &WebApp, request: Request) -> Result<Rc<PaintStage>> {
        if let Some(active) = self
            .active
            .borrow()
            .as_ref()
            .filter(|s| s.request == request)
        {
            return Ok(active.clone());
        }
        scene::ensure(self.budget.alive.get(), "ordered paint owner retired")?;
        let plan = scene_widget_paint_plan(
            &request.scene,
            &request.slots,
            &request.replacements,
            &request.modes,
        )?;
        let document = self
            .content
            .owner_document()
            .ok_or_else(|| ChartError("ordered paint has no document".into()))?;
        let namespace = svg::owner_namespace(&document)?;
        let mut lease = Lease::new(self.budget.clone());
        let mut planes = vec![];
        let mut patterns = PatternCost::default();
        let context = PaintContext {
            request: &request,
            document: &document,
            namespace: &namespace,
        };
        let rendered = render_nodes(
            plan.nodes(),
            true,
            &context,
            &mut planes,
            &mut lease,
            &mut patterns,
        )?;
        let mut rendered = rendered;
        if !request.helpers.is_empty() {
            let mut helper = ChartConfig {
                width: request.scene.width,
                height: request.scene.height,
                ..Default::default()
            }
            .scene()?;
            helper.background = Color(0, 0, 0, 0);
            helper.marks = request.helpers.clone();
            let plane = render_plane(
                &helper,
                BlendMode::Normal,
                false,
                &context,
                planes.len(),
                &mut lease,
                &mut patterns,
            )?;
            rendered.push(Tree::Plane(planes.len()));
            planes.push(plane);
        }
        scene::ensure(
            self.budget.alive.get(),
            "ordered paint owner retired during rendering",
        )?;
        let capture = Rc::new(RefCell::new(None));
        let result = capture.clone();
        let root_class = format!("{namespace}-paint-root");
        let query_class = root_class.clone();
        let mut root_source = Some(rendered);
        let root = app
            .app()
            .scope()
            .mount_with(|| {
                glory::ui::widget(move |scope| {
                    let id = scope.cage(None);
                    let targets = Rc::new(RefCell::new(BTreeMap::new()));
                    let out = targets.clone();
                    let tree = root_source.take().expect("paint tree builds once");
                    let plane_classes = planes_classes(&tree, &namespace);
                    glory::ui::view()
                        .class(format!("kartis-ordered-paint {root_class}"))
                        .node_ref(id)
                        .style(root_style(request.scene.background, false))
                        .semantics(Some(Box::new(Semantics {
                            hidden: true,
                            inert: true,
                            ..Default::default()
                        })))
                        .fill(glory::ui::widget(move |scope| {
                            build_tree(scope, &tree, &plane_classes, &out)
                        }))
                        .show_in(scope)?;
                    *result.borrow_mut() = Some((
                        id.get().expect("paint root identity"),
                        targets.borrow().clone(),
                        scope.clone(),
                    ));
                    Ok(())
                })
            })
            .map_err(host)?;
        let (root_id, targets, scope) = capture
            .borrow_mut()
            .take()
            .ok_or_else(|| ChartError("ordered paint tree did not initialize".into()))?;
        let stage = Rc::new(PaintStage {
            request,
            root: RefCell::new(Some(root)),
            native_root: RefCell::new(None),
            scope,
            root_id,
            targets,
            planes,
            _lease: lease,
        });
        let result = (|| {
            app.app().pump().map_err(host)?;
            let root = self
                .content
                .query_selector(&format!(".{query_class}"))
                .map_err(host)?
                .ok_or_else(|| ChartError("ordered paint root missing".into()))?;
            stage.native_root.replace(Some(root.clone()));
            for plane in &stage.planes {
                let target = root
                    .query_selector(&format!(".{}", plane.class))
                    .map_err(host)?
                    .ok_or_else(|| ChartError("ordered renderer container missing".into()))?;
                target.append_child(&plane.outer).map_err(host)?;
            }
            scene::ensure(
                self.budget.alive.get(),
                "ordered paint owner retired during preparation",
            )
        })();
        if let Err(error) = result {
            drop(stage);
            let _ = app.app().pump();
            return Err(error);
        }
        Ok(stage)
    }
    pub fn commit(
        &self,
        manager: &crate::widget::plot_widgets::composition::CompositionManager,
        widget_stage: &Rc<crate::widget::plot_widgets::composition::CompositionStage>,
        stage: Rc<PaintStage>,
    ) -> Result<()> {
        let placed = self.place(manager, widget_stage, stage)?;
        manager.with_commit_guard(|| {
            placed.validate()?;
            drop(self.adopt_placed(&placed, &mut || {}));
            Ok(())
        })
    }
    pub fn commit_with_adopt(
        &self,
        manager: &crate::widget::plot_widgets::composition::CompositionManager,
        widget_stage: &Rc<crate::widget::plot_widgets::composition::CompositionStage>,
        stage: Rc<PaintStage>,
        adopt: &mut dyn FnMut(),
    ) -> Result<()> {
        let placed = self.place(manager, widget_stage, stage)?;
        manager.with_commit_guard(|| {
            placed.validate()?;
            drop(self.adopt_placed(&placed, adopt));
            Ok(())
        })
    }
    pub(super) fn place(
        &self,
        manager: &crate::widget::plot_widgets::composition::CompositionManager,
        widgets: &Rc<crate::widget::plot_widgets::composition::CompositionStage>,
        stage: Rc<PaintStage>,
    ) -> Result<Placement> {
        scene::ensure(self.budget.alive.get(), "ordered paint owner retired")?;
        let previous = self.active.borrow().clone();
        let changed = previous.as_ref().is_none_or(|old| !Rc::ptr_eq(old, &stage));
        // Hiding the old inert tree can blur a retained child before the
        // backend's managed-root move has an opportunity to preserve it.
        let focus = previous.as_ref().and_then(|old| old.focused());
        stage.activate()?;
        let result: Result<CompositionPlacement> = (|| {
            if changed && let Some(old) = &previous {
                old.set_active(false)?;
            }
            let placed = manager.place_placed(widgets, &stage.targets)?;
            if let Some(focus) = &focus {
                restore_dom_focus(focus)?;
            }
            Ok(placed)
        })();
        match result {
            Ok(widgets) => Ok(Placement {
                stage,
                previous,
                widgets,
                budget: self.budget.clone(),
                focus,
                finished: Cell::new(false),
            }),
            Err(error) => {
                let rollback = (|| -> Result<()> {
                    if changed {
                        stage.set_active(false)?;
                        if let Some(old) = previous {
                            old.activate()?;
                        }
                    }
                    if let Some(focus) = &focus {
                        restore_dom_focus(focus)?;
                    }
                    Ok(())
                })();
                match rollback {
                    Ok(()) => Err(error),
                    Err(rollback) => Err(ChartError(format!(
                        "{}; ordered paint rollback: {}",
                        error.0, rollback.0
                    ))),
                }
            }
        }
    }
    pub(super) fn adopt_placed(&self, placed: &Placement, adopt: &mut dyn FnMut()) -> Retirement {
        assert!(
            !placed.finished.replace(true),
            "ordered placement finalized twice"
        );
        let widgets = placed.widgets.adopt_with(adopt);
        let previous = self.active.replace(Some(placed.stage.clone()));
        Retirement {
            _widgets: widgets,
            _paint: previous,
        }
    }
}

fn root_style(background: Color, active: bool) -> Style {
    Style {
        background: Some(background),
        opacity: Some(if active { 1. } else { 0. }),
        overflow_x: Some(Overflow::Hidden),
        overflow_y: Some(Overflow::Hidden),
        ..full_style()
    }
}
fn full_style() -> Style {
    Style::new()
        .position(Position::Absolute)
        .left(Length::Px(0.))
        .top(Length::Px(0.))
        .width(Length::Percent(100.))
        .height(Length::Percent(100.))
        .pointer_events(PointerEvents::None)
}
#[derive(Clone)]
enum Tree {
    Plane(usize),
    Widget(String),
    Group(f64, Vec<Tree>),
}
fn planes_classes(nodes: &[Tree], namespace: &str) -> BTreeMap<usize, String> {
    let mut out = BTreeMap::new();
    for node in nodes {
        match node {
            Tree::Plane(i) => {
                out.insert(*i, format!("{namespace}-plane-{i}"));
            }
            Tree::Group(_, c) => out.extend(planes_classes(c, namespace)),
            Tree::Widget(_) => {}
        }
    }
    out
}
fn build_tree(
    scope: &Scope,
    nodes: &[Tree],
    classes: &BTreeMap<usize, String>,
    targets: &Rc<RefCell<WidgetTargets>>,
) -> glory::ui::Result {
    for node in nodes {
        match node {
            Tree::Plane(i) => glory::ui::view()
                .class(classes[i].clone())
                .style(full_style())
                .show_in(scope)?,
            Tree::Widget(key) => {
                let id = scope.cage(None);
                glory::ui::view()
                    .class("kartis-ordered-widget-anchor")
                    .node_ref(id)
                    .style(full_style())
                    .show_in(scope)?;
                targets.borrow_mut().insert(
                    key.clone(),
                    (id.get().expect("Widget anchor identity"), None),
                );
            }
            Tree::Group(opacity, children) => {
                let children = children.clone();
                let classes = classes.clone();
                let targets = targets.clone();
                glory::ui::view()
                    .class("kartis-ordered-opacity-group")
                    .style(Style {
                        opacity: Some(*opacity),
                        ..full_style()
                    })
                    .semantics(Some(Box::new(Semantics {
                        inert: *opacity == 0.,
                        hidden: *opacity == 0.,
                        ..Default::default()
                    })))
                    .fill(glory::ui::widget(move |scope| {
                        build_tree(scope, &children, &classes, &targets)
                    }))
                    .show_in(scope)?;
            }
        }
    }
    Ok(())
}

#[derive(Default)]
struct PatternCost {
    pixels: u64,
    operations: usize,
}
struct PaintContext<'a> {
    request: &'a Request,
    document: &'a web_sys::Document,
    namespace: &'a str,
}

fn render_nodes(
    nodes: &[WidgetPaintNode],
    visible: bool,
    context: &PaintContext<'_>,
    planes: &mut Vec<Plane>,
    lease: &mut Lease,
    patterns: &mut PatternCost,
) -> Result<Vec<Tree>> {
    let request = context.request;
    let mut tree = vec![];
    for node in nodes {
        match node {
            WidgetPaintNode::Widget(id) => tree.push(Tree::Widget(id.clone())),
            WidgetPaintNode::Paint(run) => {
                if !visible {
                    continue;
                }
                let scene = run.scene().without_series(&request.hidden);
                if scene.marks.is_empty() && scene.labels.is_empty() {
                    continue;
                }
                let plane = render_plane(
                    &scene,
                    run.blend(),
                    true,
                    context,
                    planes.len(),
                    lease,
                    patterns,
                )?;
                tree.push(Tree::Plane(planes.len()));
                planes.push(plane);
            }
            WidgetPaintNode::Group {
                opacity, children, ..
            } => {
                if visible && *opacity > 0. && *opacity < 1. {
                    lease.pixels(pixel_count(request)?)?;
                }
                tree.push(Tree::Group(
                    *opacity,
                    render_nodes(
                        children,
                        visible && *opacity > 0.,
                        context,
                        planes,
                        lease,
                        patterns,
                    )?,
                ));
            }
        }
    }
    Ok(tree)
}
fn pixel_count(request: &Request) -> Result<u64> {
    let w = (request.scene.width * request.ratio).ceil();
    let h = (request.scene.height * request.ratio).ceil();
    scene::ensure(
        w.is_finite() && h.is_finite() && w >= 1. && h >= 1. && w * h <= MAX_PIXELS as f64,
        "ordered paint dimensions exceed pixel budget",
    )?;
    Ok(w as u64 * h as u64)
}
fn render_plane(
    scene: &Scene,
    mode: BlendMode,
    occludes: bool,
    context: &PaintContext<'_>,
    index: usize,
    lease: &mut Lease,
    patterns: &mut PatternCost,
) -> Result<Plane> {
    let PaintContext {
        request,
        document,
        namespace,
    } = *context;
    let pixels = pixel_count(request)?;
    lease.pixels(pixels)?;
    if mode != BlendMode::Normal {
        lease.pixels(pixels)?;
    }
    let density = request.ratio * request.view.scale;
    let mut ids = BTreeSet::new();
    for mark in &scene.marks {
        if let Some(FillPaint::Pattern(p)) = &mark.fill_paint
            && ids.insert(p.deterministic_id())
        {
            let [w, h] = p.tile_pixels(density)?;
            patterns.pixels = patterns
                .pixels
                .checked_add(u64::from(w) * u64::from(h))
                .ok_or_else(|| ChartError("ordered pattern pixel overflow".into()))?;
            patterns.operations = patterns
                .operations
                .checked_add(p.tile_operation_cost(density)?)
                .ok_or_else(|| ChartError("ordered pattern operation overflow".into()))?;
            scene::ensure(
                patterns.pixels <= MAX_PATTERN_PIXELS
                    && patterns.operations <= MAX_PATTERN_OPERATIONS,
                "ordered cumulative pattern budget exceeded",
            )?;
        }
    }
    let outer = document.create_element("div").map_err(host)?;
    outer.set_attribute("aria-hidden", "true").map_err(host)?;
    outer
        .set_attribute("class", "kartis-ordered-renderer-plane")
        .map_err(host)?;
    outer
        .set_attribute(
            "style",
            &format!(
                "position:absolute;inset:0;pointer-events:none;mix-blend-mode:{}",
                mode.css_name()
            ),
        )
        .map_err(host)?;
    // SVG's visible plane and its temporary Canvas probe coexist until the
    // native SVG has been prepared and the probe backing store is released.
    let mut svg_probe = Lease::new(lease.budget.clone());
    if request.renderer == BrowserRenderer::Svg {
        svg_probe.pixels(pixels)?;
    }
    let canvas = document
        .create_element("canvas")
        .map_err(host)?
        .dyn_into::<HtmlCanvasElement>()
        .map_err(host)?;
    canvas.set_attribute("aria-hidden", "true").map_err(host)?;
    canvas
        .set_attribute(
            "style",
            "position:absolute;inset:0;width:100%;height:100%;pointer-events:none",
        )
        .map_err(host)?;
    if let Some(lang) = &request.language {
        canvas.set_attribute("lang", lang).map_err(host)?;
        outer.set_attribute("lang", lang).map_err(host)?;
    }
    canvas.set_attribute("dir", "ltr").map_err(host)?;
    outer.set_attribute("dir", "ltr").map_err(host)?;
    let hit_scene = request.source_hit_ids.as_ref().map(|ids| {
        let labels = (0..scene.labels.len()).collect();
        let excluded = scene
            .marks
            .iter()
            .filter(|m| !ids.contains(&m.id))
            .map(|m| m.id.clone())
            .collect();
        scene.without_label_ink(&labels).without_mark_ink(&excluded)
    });
    scene::ensure(
        request.source_hit_ids.is_none() || request.renderer == BrowserRenderer::Svg,
        "literal source hit projection requires SVG renderer",
    )?;
    render::draw_canvas_with_transform(
        hit_scene.as_ref().unwrap_or(scene),
        &canvas,
        request.ratio,
        request.view,
    )?;
    let native_tree = request.tree_labels_passive && request.renderer == BrowserRenderer::Svg;
    // Tree source SVG paths themselves own native input. A raster-shaped
    // companion rect would hide their actual node/link DOM provenance.
    let rects = if occludes && !native_tree {
        let mut temporary = Lease::new(lease.budget.clone());
        temporary.pixels(pixels)?;
        let context = canvas
            .get_context("2d")
            .map_err(host)?
            .ok_or_else(|| ChartError("ordered hit canvas unavailable".into()))?
            .dyn_into::<web_sys::CanvasRenderingContext2d>()
            .map_err(host)?;
        let data = context
            .get_image_data(
                0.,
                0.,
                f64::from(canvas.width()),
                f64::from(canvas.height()),
            )
            .map_err(host)?
            .data();
        let rects = alpha_rectangles(
            &data.0,
            canvas.width(),
            canvas.height(),
            MAX_HIT_RECTS - lease.budget.rects.get(),
        )?;
        lease.rects(rects.len())?;
        rects
    } else {
        vec![]
    };
    let retained = if request.renderer == BrowserRenderer::Svg {
        let mut svg = svg::RetainedSvg::new(document)?;
        if request.tree_labels_passive {
            svg.make_labels_passive();
        }
        svg.paint(
            outer.dyn_ref::<HtmlElement>().expect("HTML renderer root"),
            scene,
            request.view,
        )?;
        if native_tree {
            let mut ids = request.source_hit_ids.clone().unwrap_or_default();
            let opacity = scene.mark_group_opacities();
            let visible: BTreeSet<_> = scene
                .marks
                .iter()
                .filter(|m| {
                    m.opacity * opacity.get(&m.id).copied().unwrap_or(1.) > 0.
                        && (m.fill.is_some_and(|Color(_, _, _, a)| a > 0)
                            || m.stroke.is_some_and(|s| s.width > 0. && s.color.3 > 0))
                })
                .map(|m| m.id.as_str())
                .collect();
            ids.retain(|id| visible.contains(id.as_str()));
            svg.native_hit_ids(&ids)?;
        }
        Some(svg)
    } else {
        outer.append_child(&canvas).map_err(host)?;
        None
    };
    if !rects.is_empty() {
        let mask = hit_mask(
            document,
            namespace,
            index,
            request,
            [canvas.width(), canvas.height()],
            &rects,
        )?;
        outer.append_child(&mask).map_err(host)?;
    }
    if request.renderer == BrowserRenderer::Svg {
        canvas.set_width(0);
        canvas.set_height(0);
    }
    Ok(Plane {
        class: format!("{namespace}-plane-{index}"),
        outer,
        _svg: retained,
    })
}
pub(super) fn inherited(node: &Element, name: &str) -> Option<String> {
    let mut node = Some(node.clone());
    while let Some(current) = node {
        let value = if name == "lang" {
            current
                .get_attribute_ns(Some("http://www.w3.org/XML/1998/namespace"), "lang")
                .or_else(|| current.get_attribute("xml:lang"))
                .or_else(|| current.get_attribute(name))
        } else {
            current.get_attribute(name)
        };
        if value.is_some() {
            return value;
        }
        node = current.parent_element();
    }
    None
}
fn hit_mask(
    document: &web_sys::Document,
    namespace: &str,
    index: usize,
    request: &Request,
    backing: [u32; 2],
    rects: &[AlphaRect],
) -> Result<Element> {
    const NS: &str = "http://www.w3.org/2000/svg";
    let svg = document.create_element_ns(Some(NS), "svg").map_err(host)?;
    for (name, value) in [
        ("class", "kartis-ordered-hit-mask".to_string()),
        ("width", "100%".into()),
        ("height", "100%".into()),
        (
            "viewBox",
            format!("0 0 {} {}", request.scene.width, request.scene.height),
        ),
        ("preserveAspectRatio", "none".into()),
        ("aria-hidden", "true".into()),
        (
            "style",
            "position:absolute;inset:0;pointer-events:none;overflow:hidden".into(),
        ),
    ] {
        svg.set_attribute(name, &value).map_err(host)?;
    }
    let defs = document.create_element_ns(Some(NS), "defs").map_err(host)?;
    let clip = document
        .create_element_ns(Some(NS), "clipPath")
        .map_err(host)?;
    let id = format!("{namespace}-hit-{index}");
    clip.set_attribute("id", &id).map_err(host)?;
    clip.set_attribute("clipPathUnits", "userSpaceOnUse")
        .map_err(host)?;
    let mut path = String::new();
    use std::fmt::Write;
    let scales = if request.renderer == BrowserRenderer::Canvas {
        [
            request.scene.width / f64::from(backing[0]),
            request.scene.height / f64::from(backing[1]),
        ]
    } else {
        [1. / request.ratio; 2]
    };
    for r in rects {
        let x = f64::from(r.x) * scales[0];
        let y = f64::from(r.y) * scales[1];
        let w = f64::from(r.width) * scales[0];
        let h = f64::from(r.height) * scales[1];
        write!(path, "M{x} {y}h{w}v{h}h{}Z", -w).expect("String write");
    }
    let shape = document.create_element_ns(Some(NS), "path").map_err(host)?;
    shape.set_attribute("d", &path).map_err(host)?;
    clip.append_child(&shape).map_err(host)?;
    defs.append_child(&clip).map_err(host)?;
    svg.append_child(&defs).map_err(host)?;
    let target = document.create_element_ns(Some(NS), "rect").map_err(host)?;
    for (name, value) in [
        ("width", request.scene.width.to_string()),
        ("height", request.scene.height.to_string()),
        ("fill", "transparent".into()),
        ("pointer-events", "all".into()),
        ("clip-path", format!("url(#{id})")),
    ] {
        target.set_attribute(name, &value).map_err(host)?;
    }
    svg.append_child(&target).map_err(host)?;
    Ok(svg)
}

impl State {
    pub(super) fn ordered_request(&self, window: &Window) -> Result<Request> {
        let (mut scene, ..) = self.paint_scene_for(&self.model)?;
        let count = self.model.scene().marks.len();
        let helpers = scene.marks.split_off(count);
        scene
            .draw_order
            .retain(|item| !matches!(item,DrawItem::Mark(i) if *i>=count));
        scene.validate()?;
        let packet = self
            .plot_widgets
            .as_ref()
            .ok_or_else(|| ChartError("ordered Widget packet missing".into()))?;
        let modes = self.current_blends().for_scene(&scene);
        // Both renderer profiles use CSS blending across renderer/Widget planes.
        blends::admit(
            BrowserRenderer::Svg,
            window,
            &scene,
            &modes,
            self.model.paint_transform(),
        )?;
        let source_hit_ids =
            if self.tree_labels_passive && self.options.renderer == BrowserRenderer::Svg {
                let mut projected = self.model.clone();
                let order = self
                    .model
                    .scene()
                    .draw_items()
                    .into_iter()
                    .filter_map(|item| match item {
                        DrawItem::Mark(i) => Some(i),
                        DrawItem::Label(_) => None,
                    })
                    .collect();
                projected.set_source_paint_order(order)?;
                Some(projected.source_ink_ids())
            } else {
                None
            };
        Ok(Request {
            ratio: blends::ratio(window.device_pixel_ratio(), &scene),
            scene,
            slots: packet.scene_layout(),
            replacements: packet.replacement_keys().clone(),
            modes,
            hidden: self.model.hidden().clone(),
            view: self.model.paint_transform(),
            renderer: self.options.renderer,
            tree_labels_passive: self.tree_labels_passive,
            helpers,
            language: inherited(&self.surface, "lang"),
            source_hit_ids,
        })
    }
}
