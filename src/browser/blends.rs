use web_sys::js_sys::{Function, Reflect};

use super::*;

pub(super) fn ratio(dpr: f64, scene: &Scene) -> f64 {
    dpr.clamp(0.25, 4.)
        .min((67_000_000. / (scene.width * scene.height)).sqrt())
}

pub(super) fn admit(
    renderer: BrowserRenderer,
    window: &Window,
    scene: &Scene,
    modes: &MarkBlends,
    view: ZoomTransform,
) -> Result<()> {
    if modes.is_empty() {
        return Ok(());
    }
    modes.validate(scene)?;
    match renderer {
        BrowserRenderer::Canvas => {
            let ratio = ratio(window.device_pixel_ratio(), scene);
            let size = [scene.width, scene.height].map(|v| (v * ratio).ceil() as u32);
            crate::blend::BlendPlan::new(scene, modes, size, ratio, view)?;
        }
        BrowserRenderer::Svg => {
            let css = Reflect::get(window, &"CSS".into()).map_err(host)?;
            let supports = Reflect::get(&css, &"supports".into())
                .map_err(host)?
                .dyn_into::<Function>()
                .map_err(host)?;
            for mode in modes
                .iter()
                .map(|(_, mode)| mode.css_name())
                .collect::<BTreeSet<_>>()
            {
                scene::ensure(
                    supports
                        .call2(&css, &"mix-blend-mode".into(), &mode.into())
                        .map_err(host)?
                        .as_bool()
                        == Some(true),
                    "SVG blend mode unavailable",
                )?;
            }
        }
    }
    Ok(())
}

pub(super) struct Frame {
    pub scene: Scene,
    pub tooltip_ids: Vec<String>,
    pub interaction: Option<ActiveInteraction>,
    pub modes: MarkBlends,
    pub canvas: Option<render::PreparedCanvas>,
}
