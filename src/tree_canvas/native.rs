use std::rc::Rc;

use wasm_bindgen::{JsCast, JsValue};
use web_sys::{CanvasRenderingContext2d, Document, HtmlCanvasElement};

use super::*;

pub type TreeCanvasNodeRenderer<'a> =
    dyn FnMut(&CanvasRenderingContext2d, &TreeCanvasNodeContext<'_>) -> Result<()> + 'a;
pub type TreeCanvasLinkRenderer<'a> =
    dyn FnMut(&CanvasRenderingContext2d, &mut TreeCanvasLinkContext<'_>) -> Result<()> + 'a;
pub type TreeCanvasLabelRenderer<'a> =
    dyn FnMut(&CanvasRenderingContext2d, &TreeCanvasLabelContext<'_>) -> Result<()> + 'a;
pub type TreeCanvasLayerRenderer<'a> =
    dyn FnMut(&CanvasRenderingContext2d, &mut TreeCanvasLayerContext<'_>) -> Result<()> + 'a;

/// Borrowed for one paint. Every actual invocation shares one native context;
/// user callback state is intentionally not wrapped in save/restore.
#[derive(Default)]
pub struct TreeCanvasCallbacks<'a> {
    pub render_node: Option<&'a mut TreeCanvasNodeRenderer<'a>>,
    pub render_link: Option<&'a mut TreeCanvasLinkRenderer<'a>>,
    pub render_label: Option<&'a mut TreeCanvasLabelRenderer<'a>>,
    pub custom_layers: BTreeMap<String, &'a mut TreeCanvasLayerRenderer<'a>>,
    /// Owner lifecycle/generation guard, checked before and after user code.
    /// No guard or paint callback runs before complete static admission.
    pub checkpoint: Option<&'a mut (dyn FnMut() -> Result<()> + 'a)>,
}

/// A sealed owned bitmap. Its context was never supplied to a paint callback.
/// Snapshotting/export never reevaluates stateful render callbacks.
#[derive(Clone)]
pub struct PreparedTreeCanvas {
    sealed: HtmlCanvasElement,
    frame: TreeCanvasFrame,
    size: [u32; 2],
    options: TreeCanvasOptions,
    admission: Rc<()>,
}
struct PendingCanvas(HtmlCanvasElement);
impl Drop for PendingCanvas {
    fn drop(&mut self) {
        // A user may retain a clone of the genuine callback context. Retire
        // its host backing allocation on every success/error/stop exit.
        self.0.set_width(0);
        self.0.set_height(0);
    }
}
impl PreparedTreeCanvas {
    pub fn frame(&self) -> &TreeCanvasFrame {
        &self.frame
    }
    pub fn backing_size(&self) -> [u32; 2] {
        self.size
    }
    pub fn options(&self) -> &TreeCanvasOptions {
        &self.options
    }
    pub fn to_png_data_url(&self) -> Result<String> {
        self.sealed.to_data_url().map_err(host)
    }
    pub(crate) fn same_bitmap(&self, other: &Self) -> bool {
        self.sealed == other.sealed
    }
    pub(crate) fn same_admission(&self, other: &Self) -> bool {
        self.same_bitmap(other) || Rc::ptr_eq(&self.admission, &other.admission)
    }
    pub(crate) fn inherit_admission(&mut self, other: &Self) {
        self.admission = other.admission.clone();
    }
    /// Exact sealed backing transfer. This does not reinterpret a new camera,
    /// resize source coordinates or reexecute any user callback.
    pub(crate) fn blit(&self, destination: &HtmlCanvasElement) -> Result<()> {
        destination.set_width(self.size[0]);
        destination.set_height(self.size[1]);
        context(destination)?
            .draw_image_with_html_canvas_element(&self.sealed, 0., 0.)
            .map_err(host)
    }
}

/// Draw into the caller's actual native Canvas. Like the upstream low-level
/// API, an Err may leave this destination partially painted. Owners requiring
/// rollback must prepare/seal and atomically adopt instead.
pub fn draw_tree_canvas(
    canvas: &HtmlCanvasElement,
    frame: &TreeCanvasFrame,
    plan: &TreeCanvasPlan,
    options: &TreeCanvasOptions,
    callbacks: &mut TreeCanvasCallbacks<'_>,
) -> Result<()> {
    let mesh = preflight(frame, plan, options, callbacks, 1)?;
    let size = options.backing_size(frame, plan.budget(), 1)?;
    paint(
        canvas,
        size,
        frame,
        plan,
        options,
        callbacks,
        mesh.as_deref(),
    )
}

/// Execute against one private native Canvas, then copy to a second, unexposed
/// bitmap. Retaining the callback's raw context cannot mutate accepted pixels.
pub fn prepare_tree_canvas(
    document: &Document,
    frame: &TreeCanvasFrame,
    plan: &TreeCanvasPlan,
    options: &TreeCanvasOptions,
    callbacks: &mut TreeCanvasCallbacks<'_>,
) -> Result<PreparedTreeCanvas> {
    let mesh = preflight(frame, plan, options, callbacks, 2)?;
    let size = options.backing_size(frame, plan.budget(), 2)?;
    let pending = PendingCanvas(canvas(document, size)?);
    // The pending native canvas is intentionally not an accepted DOM node.
    // Explicit language/direction avoid depending on a detached default locale.
    if let Some(language) = document
        .document_element()
        .and_then(|e| e.get_attribute("lang"))
    {
        pending.0.set_attribute("lang", &language).map_err(host)?;
    }
    pending.0.set_attribute("dir", "ltr").map_err(host)?;
    paint(
        &pending.0,
        size,
        frame,
        plan,
        options,
        callbacks,
        mesh.as_deref(),
    )?;
    checkpoint(callbacks)?;
    // Reject a tainted pending bitmap before any owner publication.
    pending.0.to_data_url().map_err(host)?;
    let sealed = canvas(document, size)?;
    context(&sealed)?
        .draw_image_with_html_canvas_element(&pending.0, 0., 0.)
        .map_err(host)?;
    checkpoint(callbacks)?;
    Ok(PreparedTreeCanvas {
        sealed,
        frame: frame.clone(),
        size,
        options: options.clone(),
        admission: Rc::new(()),
    })
}

fn preflight(
    frame: &TreeCanvasFrame,
    plan: &TreeCanvasPlan,
    options: &TreeCanvasOptions,
    callbacks: &TreeCanvasCallbacks<'_>,
    copies: u64,
) -> Result<Option<Vec<crate::mesh::VoronoiCell>>> {
    ensure(
        frame == plan.frame(),
        "Tree Canvas plan belongs to a different domain frame",
    )?;
    options.backing_size(frame, plan.budget(), copies)?;
    for layer in plan.layers() {
        if let TreeCanvasLayer::Custom(key) = &layer.layer {
            ensure(
                callbacks.custom_layers.contains_key(key),
                "Tree Canvas custom layer callback is missing",
            )?;
        }
    }
    if options.mesh.debug
        && options.mesh.use_mesh
        && options.mesh.interactive
        && plan
            .layers()
            .iter()
            .any(|l| l.layer == TreeCanvasLayer::Mesh)
    {
        let points = frame
            .layout()
            .nodes
            .iter()
            .filter(|n| !options.hidden.contains(&frame.source()[n.source_index].id))
            .map(|n| crate::mesh::MeshPoint {
                id: frame.source()[n.source_index].id.clone(),
                x: n.position[0],
                y: n.position[1],
                clip: None,
            })
            .collect();
        let mesh = crate::mesh::PointMesh::new(points)?;
        Ok(Some(mesh.voronoi(ClipRect {
            x: 0.,
            y: 0.,
            width: frame.config().width,
            height: frame.config().height,
        })?))
    } else {
        Ok(None)
    }
}

#[allow(clippy::too_many_arguments)]
fn paint(
    canvas: &HtmlCanvasElement,
    size: [u32; 2],
    frame: &TreeCanvasFrame,
    plan: &TreeCanvasPlan,
    options: &TreeCanvasOptions,
    callbacks: &mut TreeCanvasCallbacks<'_>,
    mesh: Option<&[crate::mesh::VoronoiCell]>,
) -> Result<()> {
    checkpoint(callbacks)?;
    canvas.set_width(size[0]);
    canvas.set_height(size[1]);
    let ctx = context(canvas)?;
    ctx.set_transform(options.pixel_ratio, 0., 0., options.pixel_ratio, 0., 0.)
        .map_err(host)?;
    ctx.set_fill_style_str(&crate::render::css_color(frame.layout().scene.background));
    ctx.fill_rect(0., 0., frame.config().width, frame.config().height);
    ctx.translate(options.view.translate_x, options.view.translate_y)
        .map_err(host)?;
    ctx.scale(options.view.scale, options.view.scale)
        .map_err(host)?;
    let mut generator = TreeCanvasLinkGenerator::new(
        frame.layout().options().style.links.curve,
        frame.layout().options().orientation,
    );
    let scratch_budget = plan.budget().live_pixels.saturating_sub(
        options
            .reserved_pixels
            .saturating_add(u64::from(size[0]) * u64::from(size[1])),
    );
    for layer in plan.layers() {
        checkpoint(callbacks)?;
        // Nivo binds even an empty Links occurrence. A preceding custom layer
        // sees None; subsequent custom code may inspect or rebind this context.
        if layer.layer == TreeCanvasLayer::Links {
            generator.set_context(Some(ctx.clone()));
        }
        if layer.layer == TreeCanvasLayer::Labels && frame.layout().options().hierarchy.labels {
            ctx.set_font(
                &frame
                    .theme()
                    .font
                    .resolve(&ResolvedFont::default())
                    .canvas_font(frame.theme().font_size),
            );
        }
        for role in layer.roles() {
            checkpoint(callbacks)?;
            if frame.role_is_hidden(*role, &options.hidden)? {
                continue;
            }
            match *role {
                TreeCanvasRole::Node(index) => {
                    let c = frame.node_context(index)?;
                    if let Some(callback) = callbacks.render_node.as_deref_mut() {
                        callback(&ctx, &c)?;
                    } else if let Some(mark) = c.mark {
                        crate::render::tree_canvas_mark(
                            &ctx,
                            mark,
                            options.pixel_ratio,
                            options.view,
                            scratch_budget,
                        )?;
                    }
                }
                TreeCanvasRole::Link(index) => {
                    let link = &frame.layout().links[index];
                    let mark = link.mark_id.as_deref().and_then(|id| frame.source_mark(id));
                    let mut c = TreeCanvasLinkContext {
                        frame,
                        link_index: index,
                        link,
                        source_node: &frame.layout().nodes[link.source_index],
                        target_node: &frame.layout().nodes[link.target_index],
                        style: &frame.computed_link_styles()[index],
                        mark,
                        is_active: frame.activity().and_then(|a| a.links[index]),
                        generator: &mut generator,
                    };
                    if let Some(callback) = callbacks.render_link.as_deref_mut() {
                        callback(&ctx, &mut c)?;
                    } else if let Some(mark) = mark {
                        crate::render::tree_canvas_mark(
                            &ctx,
                            mark,
                            options.pixel_ratio,
                            options.view,
                            scratch_budget,
                        )?;
                    }
                }
                TreeCanvasRole::Label(index) => {
                    let c = frame.label_context(index)?;
                    if let Some(callback) = callbacks.render_label.as_deref_mut() {
                        callback(&ctx, &c)?;
                    } else {
                        crate::render::tree_canvas_label(
                            &ctx,
                            c.label,
                            &c.font,
                            c.whitespace,
                            c.stroke,
                            c.clip,
                            options.pixel_ratio,
                            options.view,
                        )?;
                    }
                }
                TreeCanvasRole::Custom => {
                    let TreeCanvasLayer::Custom(key) = &layer.layer else {
                        return Err(ChartError(
                            "Tree Canvas custom task has no layer key".into(),
                        ));
                    };
                    let callback = callbacks
                        .custom_layers
                        .get_mut(key)
                        .expect("preflight layer callback");
                    callback(
                        &ctx,
                        &mut TreeCanvasLayerContext {
                            frame,
                            layer_index: layer.layer_index,
                            layer: &layer.layer,
                            generator: &mut generator,
                        },
                    )?;
                }
                TreeCanvasRole::Mesh => {
                    if let Some(cells) = mesh {
                        paint_mesh(&ctx, frame, options, cells)?;
                    }
                }
            }
            ensure(
                canvas.width() == size[0] && canvas.height() == size[1],
                "Tree Canvas callback changed backing dimensions",
            )?;
            checkpoint(callbacks)?;
        }
    }
    checkpoint(callbacks)
}

fn paint_mesh(
    ctx: &CanvasRenderingContext2d,
    frame: &TreeCanvasFrame,
    options: &TreeCanvasOptions,
    cells: &[crate::mesh::VoronoiCell],
) -> Result<()> {
    // A builtin debug helper, like Nivo's builtin mesh, owns its own state save.
    // User node/link/label/custom callbacks do not receive this wrapper.
    ctx.save();
    ctx.set_stroke_style_str("rgba(0,0,0,0.35)");
    ctx.set_line_width(1.);
    for cell in cells {
        ctx.begin_path();
        for (i, &(x, y)) in cell.polygon.iter().enumerate() {
            if i == 0 {
                ctx.move_to(x, y);
            } else {
                ctx.line_to(x, y);
            }
        }
        ctx.close_path();
        ctx.stroke();
        if frame.activity().and_then(|a| a.focus_id.as_deref()) == Some(cell.id.as_str()) {
            ctx.set_fill_style_str("pink");
            ctx.fill();
        }
    }
    if options.mesh.detection_radius.is_finite() && options.mesh.detection_radius > 0. {
        for node in &frame.layout().nodes {
            if options
                .hidden
                .contains(&frame.source()[node.source_index].id)
            {
                continue;
            }
            ctx.begin_path();
            ctx.arc(
                node.position[0],
                node.position[1],
                options.mesh.detection_radius,
                0.,
                std::f64::consts::TAU,
            )
            .map_err(host)?;
            ctx.stroke();
        }
    }
    ctx.restore();
    Ok(())
}
fn checkpoint(callbacks: &mut TreeCanvasCallbacks<'_>) -> Result<()> {
    if let Some(check) = callbacks.checkpoint.as_deref_mut() {
        check()?;
    }
    Ok(())
}
fn canvas(document: &Document, size: [u32; 2]) -> Result<HtmlCanvasElement> {
    let c = document
        .create_element("canvas")
        .map_err(host)?
        .dyn_into::<HtmlCanvasElement>()
        .map_err(|error| host(error.into()))?;
    c.set_width(size[0]);
    c.set_height(size[1]);
    Ok(c)
}
fn context(canvas: &HtmlCanvasElement) -> Result<CanvasRenderingContext2d> {
    canvas
        .get_context("2d")
        .map_err(host)?
        .ok_or_else(|| ChartError("Tree Canvas native context unavailable".into()))?
        .dyn_into::<CanvasRenderingContext2d>()
        .map_err(|error| host(error.into()))
}
fn host(error: JsValue) -> ChartError {
    ChartError(format!("Tree Canvas host call failed: {error:?}"))
}
