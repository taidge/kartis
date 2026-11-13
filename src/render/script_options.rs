//! Explicit main-thread CanvasScript staging. No worker or GPU backend.
use super::*;

/// Behavior when main-thread OffscreenCanvas construction or 2D context fails.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CanvasScriptFallback {
    #[default]
    Error,
    Direct,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CanvasScriptSurface {
    #[default]
    Direct,
    /// Replay the complete Scene into an OffscreenCanvas on the calling thread,
    /// then composite its physical pixels into the named HTMLCanvasElement.
    Offscreen2d { fallback: CanvasScriptFallback },
}

/// Standalone export options. These do not change Scene fonts or install events.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CanvasScriptOptions {
    pub surface: CanvasScriptSurface,
    /// Maximum simultaneously planned extra pixels: offscreen frame, distinct
    /// pattern tiles and the shared blend scratch buffer. Excludes destination.
    /// Valid range1..=134217728; each RGBA8 pixel is four bytes, plus host overhead.
    /// Existing pattern/work/blend/frame limits also apply independently.
    pub max_extra_buffer_pixels: usize,
}

impl Default for CanvasScriptOptions {
    fn default() -> Self {
        Self {
            surface: CanvasScriptSurface::Direct,
            max_extra_buffer_pixels: 67_108_864,
        }
    }
}
impl CanvasScriptOptions {
    pub(super) fn validate(&self) -> Result<()> {
        ensure(
            (1..=134_217_728).contains(&self.max_extra_buffer_pixels),
            "CanvasScript extra buffer budget must be in1..=134217728",
        )
    }
}

pub(super) fn begin(
    target: &mut ScriptCanvas,
    w: f64,
    h: f64,
    bg: Color,
    budget: &str,
    patterns: &str,
) {
    let fallback = match target.options.surface {
        CanvasScriptSurface::Offscreen2d { fallback } => fallback,
        _ => unreachable!(),
    };
    let reject = if fallback == CanvasScriptFallback::Error {
        "throw error;"
    } else {
        "ctx=visible;"
    };
    let missing = if fallback == CanvasScriptFallback::Error {
        "throw Error('OffscreenCanvas 2D unavailable');"
    } else {
        "ctx=visible;"
    };
    let metadata = format!(
        "canvas.width=frameWidth;canvas.height=frameHeight;canvas.style.width='{w}px';canvas.style.height='{h}px';canvas.setAttribute('role','img');canvas.setAttribute('aria-label',{});",
        js_string(&target.title)
    );
    write!(target.code,"(()=>{{const canvas=document.getElementById({});if(!canvas)throw Error('Canvas element missing');const visible=canvas.getContext('2d');if(!visible)throw Error('Canvas 2D unavailable');const dpr=Math.min(4,Math.max(1,window.devicePixelRatio||1),Math.sqrt(67000000/({w}*{h})));const frameWidth=Math.ceil({w}*dpr),frameHeight=Math.ceil({h}*dpr);{budget}const hasOffscreen=typeof OffscreenCanvas==='function';const extraPixels=(hasOffscreen?frameWidth*frameHeight:0)+patternPixels+({});if(!Number.isSafeInteger(extraPixels)||extraPixels>{})throw Error('CanvasScript extra buffer budget exceeded');let ctx,staging=null;if(hasOffscreen){{try{{staging=new OffscreenCanvas(frameWidth,frameHeight);ctx=staging.getContext('2d');if(!ctx)throw Error('OffscreenCanvas 2D unavailable');}}catch(error){{if(staging){{staging.width=0;staging.height=0;}}staging=null;{reject}}}}}else{{{missing}}}try{{if(staging){{const attributes=visible.getContextAttributes?.();if(attributes&&(attributes.alpha===false||attributes.colorSpace&&attributes.colorSpace!=='srgb'))throw Error('Offscreen CanvasScript requires alpha-enabled sRGB destination');}}{}{patterns}if(!staging){{{metadata}}}ctx.setTransform(dpr,0,0,dpr,0,0);ctx.save();ctx.direction='ltr';ctx.fillStyle={};ctx.fillRect(0,0,{w},{h});ctx.beginPath();ctx.rect(0,0,{w},{h});ctx.clip();",js_string(&target.id),target.offscreen_blend_pixels,target.options.max_extra_buffer_pixels,target.blends,js_string(&css_color(bg))).unwrap();
}

pub(super) fn finish(target: &mut ScriptCanvas, w: f64, h: f64) {
    write!(target.code,"ctx.restore();if(staging){{canvas.width=frameWidth;canvas.height=frameHeight;canvas.style.width='{w}px';canvas.style.height='{h}px';canvas.setAttribute('role','img');canvas.setAttribute('aria-label',{});visible.save();try{{visible.setTransform(1,0,0,1,0,0);visible.globalAlpha=1;visible.globalCompositeOperation='copy';visible.imageSmoothingEnabled=false;visible.drawImage(staging,0,0);}}finally{{visible.restore();}}visible.setTransform(dpr,0,0,dpr,0,0);}}}}finally{{if(staging){{staging.width=0;staging.height=0;}}}}}})();",js_string(&target.title)).unwrap();
}
