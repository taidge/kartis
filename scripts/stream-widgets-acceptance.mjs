// Real source-ordered Stream dot Widgets. This is not a React renderer equivalence oracle.
import assert from 'node:assert/strict';
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {createServer} from 'node:http';
import {execFileSync} from 'node:child_process';
import {resolve,relative,isAbsolute,extname,join} from 'node:path';
import {chromium,firefox,webkit} from 'playwright';
import {png,pixel} from './acceptance-png.mjs';
const meta=JSON.parse(execFileSync('cargo',['metadata','--no-deps','--format-version','1'],{encoding:'utf8'}));
const root=resolve('.'),out=resolve(process.argv.slice(2).find(x=>!x.startsWith('--'))||join(meta.target_directory,'stream-widgets-browser'));
await mkdir(out,{recursive:true});
const hash=b=>createHash('sha256').update(b).digest('hex');
const files={script:new URL(import.meta.url),pngHelper:new URL('./acceptance-png.mjs',import.meta.url),wasm:'examples/browser-host/pkg/host_bg.wasm',glue:'examples/browser-host/pkg/host.js'};
const hashes=async()=>Object.fromEntries(await Promise.all(Object.entries(files).map(async([k,p])=>[k,hash(await readFile(p))]))),initial=await hashes();
async function bounded(label,fn,ms=15000){let timer;try{return await Promise.race([Promise.resolve().then(fn),new Promise((_,reject)=>{timer=setTimeout(()=>reject(Error(label+' timed out')),ms);})]);}finally{clearTimeout(timer);}}
const server=createServer(async(req,res)=>{try{const file=resolve(root,'.'+new URL(req.url,'http://localhost').pathname),p=relative(root,file);assert(!p.startsWith('..')&&!isAbsolute(p));res.setHeader('Content-Type',{'.html':'text/html','.js':'application/javascript','.wasm':'application/wasm','.json':'application/json'}[extname(file)]||'application/octet-stream');res.end(await readFile(file));}catch{res.writeHead(404);res.end();}});
await new Promise(done=>server.listen(0,'127.0.0.1',done));
const engines=process.argv.includes('--chromium-only')?[['Chromium',chromium,{}]]:[['Chrome',chromium,{channel:'chrome'}],['Firefox',firefox,{}],['WebKit',webkit,{}],['Edge',chromium,{channel:'msedge'}]];
const report={passed:false,startedAt:new Date().toISOString(),scope:'Stream actual dot-role component metadata, source-ordered SVG/Canvas paint, trusted child/source input and transactional candidate rollback. No automatic spring, arbitrary export or native OS input claim. Disabled zero-height area boundary pixels are separate diagnostics, not exact AA equivalence.',pixelTolerance:1,pixelToleranceScope:'Widget/hidden pixels, non-degenerate area interiors and restored actual dots including zero values',geometryTolerance:1/32,provenance:{start:initial},contexts:[]};
const color=v=>v===0?[0,200,0,255]:v<0?[255,220,0,255]:[0,100,255,255];
const contains=(r,p)=>r&&p[0]>=r[0]&&p[0]<r[0]+r[2]&&p[1]>=r[1]&&p[1]<r[1]+r[3];
const intersect=(a,b)=>{const x=Math.max(a[0],b[0]),y=Math.max(a[1],b[1]),w=Math.min(a[0]+a[2],b[0]+b[2])-x,h=Math.min(a[1]+a[3],b[1]+b[3])-y;return w>0&&h>0?[x,y,w,h]:null;};
const mid=r=>[r[0]+r[2]/2,r[1]+r[3]/2];
try{for(const[name,engine,options]of engines){const browser=await bounded('launch '+name,()=>engine.launch({headless:true,...options}));try{for(const dpr of[1,2]){
 const record={name,dpr,passed:false,frames:[],inputs:[],rejections:[],lifecycle:[],steps:[]};report.contexts.push(record);
 const step=async(label,fn)=>{record.phase=label;return bounded(`${name} DPR${dpr} ${label}`,fn);};
 const context=await step('context',()=>browser.newContext({viewport:{width:1400,height:1100},deviceScaleFactor:dpr}));context.setDefaultTimeout(15000);
 const page=await step('page',()=>context.newPage()),errors=[];page.on('pageerror',e=>errors.push(e.message));
 try{
 await step('navigate',()=>page.goto(`http://127.0.0.1:${server.address().port}/examples/browser-host/index.html`));await step('host ready',()=>page.waitForFunction(()=>window.hostReady===true));
 await step('hide unrelated panels',()=>page.evaluate(()=>{document.querySelector('.panels').style.display='none';window.received=[];window.receivedClicks=[];window.addEventListener('pointermove',e=>window.received.push({x:e.clientX,y:e.clientY,trusted:e.isTrusted,target:e.target.tagName}),true);window.addEventListener('click',e=>window.receivedClicks.push({x:e.clientX,y:e.clientY,trusted:e.isTrusted,target:e.target.tagName,widget:e.target.closest('.stream-dot-widget')?JSON.parse(e.target.closest('.stream-dot-widget').textContent).id:null}),true);}));
 const flush=async()=>{await step('two frames',()=>page.evaluate(()=>new Promise(done=>requestAnimationFrame(()=>requestAnimationFrame(done)))));await step('flush hosts',()=>page.evaluate(()=>window.hostFixture.stream_widgets_flush()));};
 const state=r=>step('snapshot '+r,()=>page.evaluate(r=>JSON.parse(window.hostFixture.stream_widgets_snapshot(r)),r));
 const source=(phase,width=600)=>step('original source',()=>page.evaluate(([p,w])=>JSON.parse(window.hostFixture.stream_widgets_source(p,w)),[phase,width]));
 const surface=r=>page.locator(`#stream-widgets-${r} .kartis-surface`),children=r=>page.locator(`#stream-widgets-${r} .stream-dot-widget`);
 const live=r=>step('accepted child frames '+r,()=>children(r).evaluateAll(ns=>ns.map(n=>JSON.parse(n.textContent))));
 const child=(r,id)=>children(r).filter({hasText:'"id":'+JSON.stringify(id)});
 const update=async(phase,width=600,revision=0,flags=0)=>{for(const r of['svg','canvas'])await step('candidate '+r,()=>page.evaluate(a=>window.hostFixture.stream_widgets_update(...a),[r,phase,width,revision,flags]));await flush();};
 const move=async(r,p)=>{await step('reveal '+r,()=>surface(r).scrollIntoViewIfNeeded());const b=await step('DOM viewport '+r,()=>surface(r).evaluate(n=>{const b=n.getBoundingClientRect();return{x:b.x,y:b.y,width:b.width,height:b.height};})),s=await state(r);await step('trusted move '+r,()=>page.mouse.move(b.x+p[0]/s.width*b.width,b.y+p[1]/s.height*b.height));await flush();const received=await step('received input',()=>page.evaluate(()=>window.received.at(-1)));assert.equal(received.trusted,true);record.inputs.push({renderer:r,requested:p,received});return received;};
 const remember=()=>step('remember child identities',()=>page.evaluate(()=>{window.streamNodes=new Map(['svg','canvas'].flatMap(r=>[...document.querySelectorAll(`#stream-widgets-${r} .stream-dot-widget`)].map(n=>[r+JSON.parse(n.textContent).id,n])));}));
 const retained=()=>step('verify retained identities',()=>page.evaluate(()=>[...window.streamNodes].every(([key,n])=>n.isConnected&&[...document.querySelectorAll('.stream-dot-widget')].some(now=>now===n&&key.endsWith(JSON.parse(now.textContent).id)))));
 const witness=()=>step('complete accepted witness',()=>page.evaluate(()=>['svg','canvas'].map(r=>{const root=document.getElementById('stream-widgets-'+r),s=JSON.parse(window.hostFixture.stream_widgets_snapshot(r));return{renderer:r,accepted:{active:s.active,width:s.width,height:s.height,revision:s.revision,selected:s.selected,hovered:s.hovered,hidden:s.hidden,interaction:s.interaction},html:root.outerHTML,canvas:[...root.querySelectorAll('canvas')].map(n=>n.toDataURL()),focus:document.activeElement?.outerHTML};})));
 const capture=async(label,phase,width=600,{empty=false,hidden=false}={})=>{
  const expected=await source(phase,width);
  for(const r of['svg','canvas']){
   await step('move outside plots',()=>page.mouse.move(1300,1050));await flush();const s=await state(r),actual=await live(r);
   assert.equal(actual.length,empty?0:expected.slots.length);assert.equal(await step('surface role',()=>surface(r).getAttribute('role')),'group');
   const placements=[];
   for(const slot of empty?[]:expected.slots){
    const a=actual.find(a=>a.id===slot.id),m=expected.marks.find(m=>m.id===slot.id);assert(a);assert.deepEqual(a.data,slot.data);assert.deepEqual(a.rect,slot.rect);assert.equal(a.geometry,m.geometry);assert.equal(a.paintValue,m.value);assert.deepEqual(a.paintFill,m.fill);assert.equal(a.opacity,m.opacity);assert.notEqual(a.data.sample.value,null);
    const visible=hidden||m.opacity===0?null:intersect(slot.rect,expected.plot);assert.deepEqual(a.visible,visible);
    assert.equal(a.interactive,visible!==null&&visible.every((x,i)=>x===slot.rect[i]));
    if(visible){const box=await step('real child bounds',()=>child(r,slot.id).evaluate(n=>{const b=n.getBoundingClientRect();return[b.x,b.y,b.width,b.height];})),b=await step('real surface bounds',()=>surface(r).boundingBox());
     const want=[b.x+slot.rect[0]/s.width*b.width,b.y+slot.rect[1]/s.height*b.height,slot.rect[2]/s.width*b.width,slot.rect[3]/s.height*b.height];assert(box.every((x,i)=>Math.abs(x-want[i])<=1/32),'actual child geometry follows sample anchor');placements.push({id:slot.id,actual:box,expected:want});
    }
   }
   await step('reveal screenshot surface',()=>surface(r).scrollIntoViewIfNeeded());const file=join(out,`${name}-dpr${dpr}-${label}-${r}.png`),bytes=await step('physical surface screenshot',()=>surface(r).screenshot({path:file})),image=png(bytes),samples=[];
   if(!empty&&!hidden){for(const slot of expected.slots){const m=expected.marks.find(m=>m.id===slot.id),visible=m.opacity===0?null:intersect(slot.rect,expected.plot);if(!visible)continue;const point=mid(visible);
     const top=[...expected.marks].reverse().map(m=>({m,slot:expected.slots.find(s=>s.id===m.id)})).find(x=>x.slot&&x.m.opacity!==0&&contains(intersect(x.slot.rect,expected.plot),point));assert(top);
     const rgba=pixel(image,...point,width,240),want=color(top.slot.data.sample.value);assert(rgba.every((v,i)=>Math.abs(v-want[i])<=1),JSON.stringify({label,r,id:slot.id,point,rgba,want}));samples.push({id:slot.id,point,actual:rgba,expected:want});
   }}
   if(hidden){for(const slot of expected.slots){const visible=intersect(slot.rect,expected.plot);if(!visible)continue;const point=mid(visible),rgba=pixel(image,...point,width,240);assert.deepEqual(rgba,[255,255,255,255]);samples.push({id:slot.id,hidden:true,point,actual:rgba,expected:[255,255,255,255]});}}
   if(empty){
    const points=expected.sites.filter(s=>s.value!==null&&s.position[0]>expected.plot[0]+1&&s.position[0]<expected.plot[0]+expected.plot[2]-1).map(s=>({id:s.id,point:s.position}));
    const reference=await step('independent standalone SVG image',()=>page.evaluate(async({svg,width,dpr,points})=>{
     // Rasterize the exported vector at the actual physical output size. A
     // 1x SVG image subsequently stretched to 2x is a different edge oracle.
     const doc=new DOMParser().parseFromString(svg,'image/svg+xml');doc.documentElement.setAttribute('width',String(width*dpr));doc.documentElement.setAttribute('height',String(240*dpr));
     const image=new Image();image.src='data:image/svg+xml;charset=utf-8,'+encodeURIComponent(new XMLSerializer().serializeToString(doc));await image.decode();
     const canvas=document.createElement('canvas');canvas.width=width*dpr;canvas.height=240*dpr;assertImageSize();
     function assertImageSize(){if(image.naturalWidth!==canvas.width||image.naturalHeight!==canvas.height)throw Error('SVG reference must decode at native physical size');}
     const ctx=canvas.getContext('2d');ctx.drawImage(image,0,0);
     return points.map(p=>({...p,color:[...ctx.getImageData(Math.floor(p.point[0]*dpr),Math.floor(p.point[1]*dpr),1,1).data]}));
    },{svg:expected.svg,width,dpr,points}));
    // A native SVG element preserves the same fractional CSS raster origin as
    // the displayed surface. Keep the SVG-image readback as a separate carrier
    // diagnostic, rather than assuming its offscreen edge coverage is identical.
    await step('native standalone source reference',()=>page.evaluate(({svg,renderer,width,dpr})=>{
     const surface=document.querySelector(`#stream-widgets-${renderer} .kartis-surface`),b=surface.getBoundingClientRect(),reference=document.createElement('div');
     reference.id='stream-source-reference';reference.style.cssText=`position:absolute;left:${b.left+scrollX+b.width+40}px;top:${b.top+scrollY}px;width:${b.width}px;height:${b.height}px;overflow:hidden;background:white;pointer-events:none`;
     reference.innerHTML=svg;const vector=reference.querySelector('svg');vector.style.cssText='display:block;width:100%;height:100%';
     if(renderer==='canvas'){
      // This test profile has only solid paths/circles/rectangles and rectangular
      // SVG viewports. Replay the standalone SVG through independent Canvas
      // primitives, not through Kartis or a pre-rasterized SVG image.
      const canvas=document.createElement('canvas');canvas.width=width*dpr;canvas.height=240*dpr;canvas.style.cssText='display:block;width:100%;height:100%';const ctx=canvas.getContext('2d');ctx.scale(dpr,dpr);
      const number=(e,key,fallback=0,extent=1)=>{const v=e.getAttribute(key);return v===null?fallback:v.endsWith('%')?parseFloat(v)*extent/100:Number(v);};
      function paint(e,isRoot=false){ctx.save();const tag=e.localName;
       if(['title','desc','defs'].includes(tag)){if(tag==='defs'&&e.children.length)throw Error('unexpected source reference defs');ctx.restore();return;}
       if(e.hasAttribute('transform'))throw Error('source reference profile requires untransformed exported primitives');
       if(e.hasAttribute('opacity'))ctx.globalAlpha*=Number(e.getAttribute('opacity'));
       if(e.hasAttribute('fill'))ctx.fillStyle=e.getAttribute('fill');
       if(tag==='svg'&&!isRoot){const x=number(e,'x'),y=number(e,'y'),w=number(e,'width'),h=number(e,'height'),v=e.getAttribute('viewBox').split(/\s+/).map(Number);ctx.beginPath();ctx.rect(x,y,w,h);ctx.clip();ctx.translate(x,y);ctx.scale(w/v[2],h/v[3]);ctx.translate(-v[0],-v[1]);}
       if(tag==='path'){if(e.getAttribute('stroke')&&e.getAttribute('stroke')!=='none')throw Error('unexpected source reference path stroke');if(e.getAttribute('fill')!=='none')ctx.fill(new Path2D(e.getAttribute('d')),e.getAttribute('fill-rule')||'nonzero');}
       else if(tag==='circle'){const radius=number(e,'r');if(radius>0&&e.getAttribute('fill')!=='none'){ctx.beginPath();ctx.arc(number(e,'cx'),number(e,'cy'),radius,0,Math.PI*2);ctx.fill();}}
       else if(tag==='rect'){if(e.getAttribute('fill')!=='none')ctx.fillRect(number(e,'x'),number(e,'y'),number(e,'width',0,width),number(e,'height',0,240));}
       else if(tag!=='svg'&&tag!=='g')throw Error('unexpected source reference primitive '+tag);
       for(const node of e.children)paint(node);ctx.restore();
      }
      paint(vector,true);reference.replaceChildren(canvas);
     }
     document.body.appendChild(reference);
    },{svg:expected.svg,renderer:r,width,dpr}));
    const referenceFile=join(out,`${name}-dpr${dpr}-${label}-${r}-standalone.png`);let native;
    try{native=png(await step('native standalone SVG screenshot',()=>page.locator('#stream-source-reference').screenshot({path:referenceFile})));}
    finally{await step('remove reference carrier',()=>page.evaluate(()=>document.getElementById('stream-source-reference')?.remove()));}
    for(const p of reference){
     const rgba=pixel(image,...p.point,width,240),want=pixel(native,...p.point,width,240),gated=phase!==7||expected.sites.find(s=>s.id===p.id).value!==0;
     const check={label,renderer:r,id:p.id,sourceOnly:true,point:p.point,actual:rgba,expected:want,svgImageReadback:p.color,nativeReference:referenceFile,carrier:r==='svg'?'native-svg':'independent-native-canvas-primitives',gated,passesPixelGate:rgba.every((v,i)=>Math.abs(v-want[i])<=1)};
     (record.sourceSamples??=[]).push(check);
     if(gated)assert(check.passesPixelGate,'restored dot or non-degenerate source interior mismatch '+JSON.stringify(check));
     else (record.edgeDiagnostics??=[]).push({...check,reason:'Dots disabled: zero-height sample has no dot ink or area interior. Existing strict boundary mismatches remain unaccepted AA diagnostics.'});
     samples.push(check);
    }
   }
   assert(await step('accessible child ancestry',()=>children(r).evaluateAll(ns=>ns.every(n=>!n.closest('[aria-hidden="true"]')))));
   record.frames.push({label,renderer:r,phase,width,slots:actual.length,placements,samples,png:file,sha256:hash(bytes)});
  }
 };
 for(const r of['svg','canvas'])await step('mount '+r,()=>page.evaluate(r=>window.hostFixture.stream_widgets_mount(r),r));await flush();await capture('initial',0);await remember();
 const initialSource=await source(0),stable=initialSource.slots.find(s=>s.data.seriesIndex===0&&s.data.sample.sourceIndex===2),missing=initialSource.sites.find(s=>s.value===null);assert(stable&&missing);assert(!initialSource.slots.some(s=>s.id===missing.id));
 const zero=initialSource.slots.find(s=>s.data.seriesIndex===0&&s.data.sample.sourceIndex===1);assert.equal(zero.data.sample.value,0);
 for(const r of['svg','canvas']){
  await step('reveal zero-valued child',()=>child(r,zero.id).scrollIntoViewIfNeeded());
  const box=await step('zero child native bounds',()=>child(r,zero.id).boundingBox()),at=[box.x+box.width/2,box.y+box.height-3];
  assert(await step('uncovered real zero child point',()=>child(r,zero.id).evaluate((n,p)=>{const target=document.elementFromPoint(...p);return target===n||n.contains(target);},at)));
  await step('trusted zero-valued child click',()=>page.mouse.click(...at));await flush();assert.equal((await live(r)).find(a=>a.id===zero.id).count,1);
  const zeroClick=await step('zero child click evidence',()=>page.evaluate(()=>window.receivedClicks.at(-1)));assert.equal(zeroClick.trusted,true);assert.equal(zeroClick.widget,zero.id);record.inputs.push({renderer:r,kind:'zero-child',received:zeroClick});
  await step('trusted child click',()=>child(r,stable.id).click());await step('focus actual child',()=>child(r,stable.id).focus());await flush();const a=(await live(r)).find(a=>a.id===stable.id);assert.equal(a.count,1);assert.equal(a.focus,true);assert(!(await state(r)).selected,'child click must not synthesize source selection');
  const received=await step('child click evidence',()=>page.evaluate(()=>window.receivedClicks.at(-1)));assert.equal(received.trusted,true);assert.equal(received.widget,stable.id);record.inputs.push({renderer:r,kind:'child',received});
 }
 await update(1);assert(await retained());await capture('none-becomes-real-zero',1);for(const r of['svg','canvas']){const a=await live(r);assert.equal(a.length,8);assert(a.some(a=>a.id===missing.id&&a.data.sample.value===0));assert.equal(a.find(a=>a.id===stable.id).count,1);assert(a.every(a=>a.data.series.values[0]===0||a.data.series.id==='south'));}
 for(const [mode,invalid] of[[1,0],[2,0],[0,1],[0,2],[0,3],[0,5],[0,6]]){
  for(const r of['svg','canvas']){
   await step('focus rollback child',()=>child(r,stable.id).focus());await flush();const beforePNG=await step('before rollback screenshot',()=>surface(r).screenshot()),before=await witness();let reason='';
   try{await step('reject actual candidate',()=>page.evaluate(a=>window.hostFixture.stream_widgets_update(...a),[r,3,600,1,mode|(invalid<<8)]));}catch(e){reason=String(e);}
   // A rejected synchronous setter already retains the accepted publication.
   // An explicit flush would request an unrelated new paint revision.
   const afterPNG=await step('after rollback screenshot',()=>surface(r).screenshot()),after=await witness();
   const stem=join(out,`${name}-dpr${dpr}-reject-${r}-${mode}-${invalid}`);await writeFile(stem+'.json',JSON.stringify({reason,before,after,beforePNG:hash(beforePNG),afterPNG:hash(afterPNG)},null,2));await writeFile(stem+'-before.png',beforePNG);await writeFile(stem+'-after.png',afterPNG);
   assert(reason,'candidate must reject');assert.deepEqual(after,before);assert.equal(hash(afterPNG),hash(beforePNG));assert(await retained());
   assert.equal(await step('no leaked staging children',()=>page.locator('.stream-dot-widget').count()),(await live('svg')).length+(await live('canvas')).length);
   record.rejections.push({renderer:r,mode,invalid,witness:stem+'.json'});
  }
 }
 await update(3);assert(await retained());await capture('offset-path-anchor',3);for(const r of['svg','canvas'])assert.equal((await live(r)).find(a=>a.id===stable.id).count,1);
 await update(4);assert(await retained());await capture('enabled-size-zero',4);await update(5);assert(await retained());await capture('transparent-dot-role',5);
 for(const r of['svg','canvas'])await step('hide all series',()=>page.evaluate(r=>window.hostFixture.stream_widgets_hidden(r,true),r));await flush();assert(await retained());await capture('host-hidden-retains-scopes',5,600,{hidden:true});
 for(const r of['svg','canvas'])await step('restore all series',()=>page.evaluate(r=>window.hostFixture.stream_widgets_hidden(r,false),r));await flush();await capture('host-visible-again',5);
 await update(0,480);for(const r of['svg','canvas'])await step('resize actual root',()=>page.locator('#stream-widgets-'+r).evaluate(n=>n.style.width='480px'));await flush();await capture('resize',0,480);
 await update(2,480);await capture('layout-hidden-removes-layer',2,480);for(const r of['svg','canvas'])assert((await live(r)).every(a=>a.data.layerId==='north'));
 await update(6,480);await capture('selective-component',6,480);
 await update(7,480);await capture('disabled-placeholders-only',7,480,{empty:true});for(const r of['svg','canvas'])assert.equal(await step('disabled role count',()=>children(r).count()),0);
 await update(0,480,0,4<<8);await capture('clear-restores-source-ink',0,480,{empty:true});
 const restored=await source(0,480),target=restored.slots.find(s=>s.id===stable.id);
 for(const r of['svg','canvas']){
  const pointer=await move(r,target.data.sample.position);let s=await state(r);assert(s.hovered.includes(stable.id));assert.equal(s.interaction.find(a=>a.id===stable.id).value,3);
  await step('trusted restored source click',()=>page.mouse.click(pointer.x,pointer.y));await flush();s=await state(r);assert.equal(s.selected,stable.id);
  const received=await step('source click evidence',()=>page.evaluate(()=>window.receivedClicks.at(-1)));assert.equal(received.trusted,true);assert.equal(received.widget,null);record.inputs.push({renderer:r,kind:'restored-source',received});
 }
 await update(0,480,3);await capture('new-children-after-clear',0,480);
 const beforeStop=await Promise.all(['svg','canvas'].map(async r=>({renderer:r,cleanups:(await state(r)).cleanups,children:await step('live children before stop',()=>children(r).count())}))),expectedCleanups=beforeStop.reduce((n,s)=>n+s.cleanups+s.children,0);
 const cleanups=await step('stop hosts',()=>page.evaluate(()=>window.hostFixture.stream_widgets_stop()));assert.equal(cleanups,expectedCleanups);assert.equal(await step('disposed roots',()=>page.locator('[id^="stream-widgets-"]').count()),0);assert.equal(await step('no children after stop',()=>page.locator('.stream-dot-widget').count()),0);record.lifecycle.push({beforeStop,stopCleanups:cleanups});
 for(const r of['svg','canvas'])await step('remount '+r,()=>page.evaluate(r=>window.hostFixture.stream_widgets_mount(r),r));await flush();for(const r of['svg','canvas'])assert((await live(r)).every(a=>a.count===0));await step('stop remounted hosts',()=>page.evaluate(()=>window.hostFixture.stream_widgets_stop()));
 assert.deepEqual(errors,[]);record.passed=true;console.log(`${name} DPR${dpr}: Stream actual roles, Path anchors, pixels, state and rollback passed`);
 }catch(e){record.error=e.stack;await bounded('failure screenshot',()=>page.screenshot({path:join(out,`${name}-dpr${dpr}-failure.png`)}),5000).catch(()=>{});await bounded('failure html',async()=>writeFile(join(out,`${name}-dpr${dpr}-failure.html`),await page.content()),5000).catch(()=>{});throw e;}
 finally{await bounded('close context',()=>context.close(),5000).catch(()=>{});}
 }}finally{await bounded('close browser',()=>browser.close(),5000).catch(()=>{});}}
 report.passed=report.contexts.length===engines.length*2&&report.contexts.every(c=>c.passed);
}finally{server.close();report.provenance.end=await hashes();report.provenance.unchanged=JSON.stringify(initial)===JSON.stringify(report.provenance.end);if(!report.provenance.unchanged)report.passed=false;report.finishedAt=new Date().toISOString();await writeFile(join(out,'report.json'),JSON.stringify(report,null,2));assert(report.provenance.unchanged,'executed driver/helper/WASM/glue changed');}
console.log(JSON.stringify({passed:report.passed,contexts:report.contexts.length,out}));
