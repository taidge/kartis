//! Script replay plans physical scratch space before resizing its destination.
use std::collections::BTreeSet;
use std::fmt::Write as _;

use crate::Scene;
use crate::blend::{
    MAX_BLEND_FRAME_PIXELS, MAX_BLEND_SCRATCH_PIXELS, MAX_BLEND_SIDE, MarkBlends,
    group_blend_bounds, has_isolated_marks, mark_blend_bounds, requires_isolation,
};

// A plan only: no backing store or rendering context is created. The offscreen
// path uses this before any extra buffer allocation for aggregate admission.
pub(super) fn scratch_pixels(scene: &Scene, blends: &MarkBlends) -> String {
    if !has_isolated_marks(scene, blends) {
        return "0".into();
    }
    let mut out = String::from("(()=>{const bounds=[");
    let bounds = scene
        .marks
        .iter()
        .map(|mark| {
            requires_isolation(mark, blends.mode(&mark.id))
                .then(|| mark_blend_bounds(mark))
                .flatten()
        })
        .chain(group_blend_bounds(scene));
    for b in bounds.flatten() {
        write!(
            out,
            "[{},{},{},{}],",
            b.min[0], b.min[1], b.max[0], b.max[1]
        )
        .unwrap();
    }
    write!(out,"];const width=Math.ceil({}*dpr),height=Math.ceil({}*dpr);let mw=0,mh=0;for(const b of bounds){{const[x0,y0,x1,y1]=b.map(v=>v*dpr);if(x0>=width||y0>=height||x1<=0||y1<=0)continue;const x=Math.max(0,Math.floor(x0)-2),y=Math.max(0,Math.floor(y0)-2);mw=Math.max(mw,Math.min(width,Math.ceil(x1)+2)-x);mh=Math.max(mh,Math.min(height,Math.ceil(y1)+2)-y);}}return mw*mh;}})()",scene.width,scene.height).unwrap();
    out
}

pub(super) fn preflight(scene: &Scene, blends: &MarkBlends) -> String {
    if !has_isolated_marks(scene, blends) {
        return String::new();
    }
    let mut out = String::from("const blendDestination=ctx;const blendBounds=[");
    let bounds = scene
        .marks
        .iter()
        .map(|mark| {
            requires_isolation(mark, blends.mode(&mark.id))
                .then(|| mark_blend_bounds(mark))
                .flatten()
        })
        .chain(group_blend_bounds(scene));
    for bounds in bounds {
        if let Some(bounds) = bounds {
            let [x0, y0] = bounds.min;
            let [x1, y1] = bounds.max;
            write!(out, "[{x0},{y0},{x1},{y1}],").unwrap();
        } else {
            out.push_str("null,");
        }
    }
    out.push_str("];const blendModes=[");
    for mode in scene
        .marks
        .iter()
        .filter(|mark| requires_isolation(mark, blends.mode(&mark.id)))
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
        write!(out, "'{mode}',").unwrap();
    }
    out.push_str("];");
    write!(out, r#"
const blendAttributes=ctx.getContextAttributes?.();
if(blendAttributes&&(blendAttributes.alpha===false||blendAttributes.colorSpace&&blendAttributes.colorSpace!=='srgb'))throw Error('Canvas blend requires an alpha-enabled sRGB context');
ctx.save();try{{for(const mode of blendModes){{ctx.globalCompositeOperation='source-over';ctx.globalCompositeOperation=mode;if(ctx.globalCompositeOperation!==mode)throw Error('Canvas blend mode unavailable');}}}}finally{{ctx.restore();}}
const blendWidth=Math.ceil({}*dpr),blendHeight=Math.ceil({}*dpr);
let blendMaxW=0,blendMaxH=0,blendPixels=0;
const blendRegions=blendBounds.map(b=>{{
 if(!b)return null;
 const [x0,y0,x1,y1]=b.map(v=>v*dpr);
 if(x0>=blendWidth||y0>=blendHeight||x1<=0||y1<=0)return null;
 const x=Math.max(0,Math.floor(x0)-2),y=Math.max(0,Math.floor(y0)-2);
 const w=Math.min(blendWidth,Math.ceil(x1)+2)-x,h=Math.min(blendHeight,Math.ceil(y1)+2)-y;
 blendMaxW=Math.max(blendMaxW,w);blendMaxH=Math.max(blendMaxH,h);blendPixels+=w*h;
 if(blendPixels>{MAX_BLEND_FRAME_PIXELS})throw Error('blend frame pixel work budget exceeded');
 if(blendMaxW>{MAX_BLEND_SIDE}||blendMaxH>{MAX_BLEND_SIDE}||blendMaxW*blendMaxH>{MAX_BLEND_SCRATCH_PIXELS})throw Error('blend scratch pixel budget exceeded');
 return [x,y,w,h];
}});
let blendScratch=null,blendSource=null;
if(blendMaxW){{blendScratch=document.createElement('canvas');blendScratch.width=blendMaxW;blendScratch.height=blendMaxH;blendSource=blendScratch.getContext('2d');if(!blendSource)throw Error('Canvas blend scratch unavailable');blendSource.fillStyle='#ffffff';blendSource.fillRect(0,0,1,1);if(blendSource.getImageData(0,0,1,1).data[3]!==255)throw Error('Canvas blend scratch backing store unavailable');}}
"#, scene.width, scene.height).unwrap();
    out
}
