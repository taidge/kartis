use super::{CanvasRenderingContext2d, HtmlCanvasElement, JsCast, host_error};
use crate::{
    ChartError, Result, Scene, ZoomTransform,
    blend::{BlendMode, BlendPlan, BlendRegion, MarkBlends},
    scene::ensure,
};
use std::collections::BTreeSet;
use web_sys::js_sys::{Function, Reflect};

pub(super) struct Layers {
    plan: BlendPlan,
    scratch: Option<CanvasRenderingContext2d>,
    active: Option<(CanvasRenderingContext2d, BlendRegion)>,
}

fn context_profile(context: &CanvasRenderingContext2d) -> Result<()> {
    let method = Reflect::get(context, &"getContextAttributes".into()).map_err(host_error)?;
    if let Some(get) = method.dyn_ref::<Function>() {
        let attributes = get.call0(context).map_err(host_error)?;
        let alpha = Reflect::get(&attributes, &"alpha".into()).map_err(host_error)?;
        let color = Reflect::get(&attributes, &"colorSpace".into()).map_err(host_error)?;
        ensure(
            alpha.as_bool() != Some(false) && color.as_string().is_none_or(|s| s == "srgb"),
            "Canvas blend requires an alpha-enabled sRGB context",
        )?;
    }
    Ok(())
}

fn probe_scratch(context: &CanvasRenderingContext2d) -> Result<()> {
    // Browsers can allocate a canvas backing store lazily. A tiny read after
    // paint forces allocation and proves this target before resizing the
    // caller's visible canvas. This source context is detached and untainted.
    context.set_fill_style_str("#ffffff");
    context.fill_rect(0., 0., 1., 1.);
    let read = Reflect::get(context, &"getImageData".into())
        .map_err(host_error)?
        .dyn_into::<Function>()
        .map_err(host_error)?;
    let pixel = read
        .call4(context, &0.into(), &0.into(), &1.into(), &1.into())
        .map_err(host_error)?;
    let data = Reflect::get(&pixel, &"data".into()).map_err(host_error)?;
    ensure(
        Reflect::get(&data, &3.into()).map_err(host_error)?.as_f64() == Some(255.),
        "Canvas blend scratch backing store unavailable",
    )
}

impl Layers {
    pub(super) fn prepare(
        context: &CanvasRenderingContext2d,
        scene: &Scene,
        blends: &MarkBlends,
        size: [u32; 2],
        ratio: f64,
        view: ZoomTransform,
    ) -> Result<Self> {
        let plan = BlendPlan::new(scene, blends, size, ratio, view)?;
        context_profile(context)?;
        context.save();
        let admitted = (|| {
            for mode in scene
                .marks
                .iter()
                .filter(|mark| crate::blend::requires_isolation(mark, blends.mode(&mark.id)))
                .map(|mark| blends.mode(&mark.id).canvas_name())
                .chain(
                    scene
                        .opacity_groups
                        .iter()
                        .filter(|g| g.opacity > 0. && g.opacity < 1.)
                        .map(|_| "source-over"),
                )
                .collect::<BTreeSet<_>>()
            {
                context
                    .set_global_composite_operation("source-over")
                    .map_err(host_error)?;
                context
                    .set_global_composite_operation(mode)
                    .map_err(host_error)?;
                ensure(
                    context.global_composite_operation().map_err(host_error)? == mode,
                    "Canvas blend mode unavailable",
                )?;
            }
            Ok(())
        })();
        context.restore();
        admitted?;
        let scratch = if plan.scratch[0] == 0 {
            None
        } else {
            let document = context
                .canvas()
                .and_then(|canvas| canvas.owner_document())
                .ok_or_else(|| ChartError("Canvas has no owner document".into()))?;
            let canvas = document
                .create_element("canvas")
                .map_err(host_error)?
                .dyn_into::<HtmlCanvasElement>()
                .map_err(|e| host_error(e.into()))?;
            canvas.set_width(plan.scratch[0]);
            canvas.set_height(plan.scratch[1]);
            let scratch = canvas
                .get_context("2d")
                .map_err(host_error)?
                .ok_or_else(|| ChartError("Canvas blend scratch unavailable".into()))?
                .dyn_into::<CanvasRenderingContext2d>()
                .map_err(|value| host_error(value.into()))?;
            context_profile(&scratch)?;
            probe_scratch(&scratch)?;
            Some(scratch)
        };
        Ok(Self {
            plan,
            scratch,
            active: None,
        })
    }

    pub(super) fn begin(
        &mut self,
        context: &mut CanvasRenderingContext2d,
        index: usize,
        ratio: f64,
        view: ZoomTransform,
    ) -> Result<bool> {
        let Some(region) = self.plan.regions[index] else {
            return Ok(false);
        };
        let scratch = self.scratch.as_ref().expect("nonempty blend scratch");
        let [w, h] = [region.width, region.height].map(f64::from);
        scratch.save();
        let staged = (|| {
            scratch
                .set_transform(1., 0., 0., 1., 0., 0.)
                .map_err(host_error)?;
            scratch.set_global_alpha(1.);
            scratch
                .set_global_composite_operation("source-over")
                .map_err(host_error)?;
            scratch.clear_rect(0., 0., w, h);
            scratch.begin_path();
            scratch.rect(0., 0., w, h);
            scratch.clip();
            scratch
                .set_transform(
                    ratio * view.scale,
                    0.,
                    0.,
                    ratio * view.scale,
                    ratio * view.translate_x - f64::from(region.x),
                    ratio * view.translate_y - f64::from(region.y),
                )
                .map_err(host_error)?;
            Ok(())
        })();
        if let Err(error) = staged {
            scratch.restore();
            return Err(error);
        }
        self.active = Some((std::mem::replace(context, scratch.clone()), region));
        Ok(true)
    }

    pub(super) fn end(
        &mut self,
        context: &mut CanvasRenderingContext2d,
        mode: BlendMode,
        opacity: f64,
        commit: bool,
    ) -> Result<()> {
        let (dest, region) = self.active.take().expect("active Canvas blend layer");
        let source = std::mem::replace(context, dest);
        source.restore();
        if !commit {
            return Ok(());
        }
        let source = source
            .canvas()
            .ok_or_else(|| ChartError("Canvas blend source disappeared".into()))?;
        let [x, y, w, h] = [region.x, region.y, region.width, region.height].map(f64::from);
        context.save();
        let painted = (|| {
            // The destination keeps its clip in device coordinates as the
            // transform switches to identity for a pixel-aligned blit.
            context
                .set_transform(1., 0., 0., 1., 0., 0.)
                .map_err(host_error)?;
            context
                .set_global_composite_operation(mode.canvas_name())
                .map_err(host_error)?;
            context.set_global_alpha(opacity);
            context.set_image_smoothing_enabled(false);
            context
                .draw_image_with_html_canvas_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                    &source, 0., 0., w, h, x, y, w, h,
                )
                .map_err(host_error)
        })();
        context.restore();
        painted
    }
}
