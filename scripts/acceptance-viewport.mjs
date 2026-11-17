// Read-only viewport capture. Element screenshots can contain integer padding;
// their encoded dimensions are not a scale factor for logical Scene positions.
import assert from 'node:assert/strict';
import {png} from './acceptance-png.mjs';

function positive(value,label){assert(Number.isFinite(value)&&value>0,label+' must be finite and positive');}
function validRect(rect){assert(rect&&Number.isFinite(rect.x)&&Number.isFinite(rect.y),'invalid surface origin');positive(rect.width,'surface width');positive(rect.height,'surface height');}

async function geometry(page,surface){
 const viewport=page.viewportSize();assert(viewport,'an explicit browser viewport is required');
 const rect=await surface.evaluate(node=>{const r=node.getBoundingClientRect();return{x:r.x,y:r.y,width:r.width,height:r.height};});validRect(rect);
 const window=await page.evaluate(()=>({dpr:devicePixelRatio,width:innerWidth,height:innerHeight,scrollX,scrollY,visual:visualViewport?{scale:visualViewport.scale,offsetLeft:visualViewport.offsetLeft,offsetTop:visualViewport.offsetTop,width:visualViewport.width,height:visualViewport.height}:null}));positive(window.dpr,'device pixel ratio');
 if(window.visual){assert.equal(window.visual.scale,1,'visual viewport zoom is unsupported');assert.equal(window.visual.offsetLeft,0,'visual viewport horizontal offset is unsupported');assert.equal(window.visual.offsetTop,0,'visual viewport vertical offset is unsupported');}
 assert.equal(window.width,viewport.width,'viewport emulation changed');assert.equal(window.height,viewport.height,'viewport emulation changed');
 return{viewport,rect,...window};
}

/** Capture without scrolling, altering CSS, hiding a caret or masking content.
 * Callers settle application input/animation and explicitly reveal their surface.
 * Stable geometry is mandatory; the image may show only part of the surface.
 * Callers provide a positive axis-aligned Scene-to-DOM mapping; a DOM rectangle
 * alone cannot describe rotation/skew/perspective. Geometry stability does not
 * prove ink/frame stability.
 * PNG decoding retains the existing bounded acceptance-png profile.
 */
export async function captureViewport(page,surface){
 const before=await geometry(page,surface);
 const bytes=await page.screenshot({type:'png',scale:'device',caret:'initial',animations:'allow',timeout:15000});
 const after=await geometry(page,surface);assert.deepEqual(after,before,'geometry changed during viewport capture');
 const image=png(bytes);
 assert.equal(image.width,before.viewport.width*before.dpr,'unexpected viewport PNG width');
 assert.equal(image.height,before.viewport.height*before.dpr,'unexpected viewport PNG height');
 return{bytes,image,geometry:before};
}

/** Map logical surface coordinates, after the application's accepted paint
 * camera projection, through the surface DOM rectangle.
 * A sample outside the actual viewport is rejected, even if it lies in Scene.
 * Floor is applied once in physical viewport pixels, never to element padding.
 */
export function rgbaAtLogical(capture,{point,size}){
 const{image,geometry:g}=capture;validRect(g.rect);positive(g.dpr,'device pixel ratio');
 assert(Array.isArray(point)&&point.length===2&&Array.isArray(size)&&size.length===2,'expected two-dimensional point and size');
 size.forEach((n,i)=>positive(n,'logical dimension '+i));
 point.forEach((n,i)=>assert(Number.isFinite(n)&&n>=0&&n<size[i],'logical sample outside Scene'));
 assert.equal(image.width,g.viewport.width*g.dpr);assert.equal(image.height,g.viewport.height*g.dpr);
 const client=[g.rect.x+point[0]*g.rect.width/size[0],g.rect.y+point[1]*g.rect.height/size[1]];
 assert(client[0]>=0&&client[0]<g.viewport.width&&client[1]>=0&&client[1]<g.viewport.height,'sample outside captured viewport');
 const index=client.map(n=>Math.floor(n*g.dpr));
 const offset=4*(index[1]*image.width+index[0]);
 return{client,index,rgba:[...image.data.subarray(offset,offset+4)]};
}
