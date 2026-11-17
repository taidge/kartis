// Actual Glory Image/View input with live typed children. No application pointer bridge.
import assert from 'node:assert/strict';
import {readFile, writeFile, mkdir} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {createServer} from 'node:http';
import {execFileSync} from 'node:child_process';
import {resolve, join, relative, isAbsolute, extname} from 'node:path';
import {chromium, firefox, webkit} from 'playwright';

const metadata=JSON.parse(execFileSync('cargo',['metadata','--no-deps','--format-version','1'],{encoding:'utf8'}));
const out=resolve(process.argv[2]||join(metadata.target_directory,'raster-chart-bindings-browser'));
const root=resolve('examples/browser-widget');
await mkdir(out,{recursive:true});
const sha=b=>createHash('sha256').update(b).digest('hex');
const debug=process.argv.includes('--debug')||process.env.RASTER_BINDINGS_DEBUG==='1';
async function bounded(name,fn,ms=15000){
 let timer;const started=Date.now();if(debug)console.log(`[begin] ${name}`);
 try{
  // Install the Node timer before running any browser/protocol operation.
  const deadline=new Promise((_,reject)=>{timer=setTimeout(()=>reject(Error(`${name} timed out after ${ms}ms`)),ms);});
  const result=await Promise.race([Promise.resolve().then(fn),deadline]);
  if(debug)console.log(`[end] ${name} (${Date.now()-started}ms)`);return result;
 }catch(error){if(debug)console.error(`[failed] ${name} (${Date.now()-started}ms): ${error}`);throw error;}
 finally{clearTimeout(timer);}
}
const artifactFiles=['scripts/raster-chart-bindings-acceptance.mjs','examples/browser-widget/pkg/widget.js','examples/browser-widget/pkg/widget_bg.wasm'];
const hashes=async()=>Object.fromEntries(await Promise.all(artifactFiles.map(async f=>[f,sha(await readFile(f))])));
const initial=await hashes();
const html='<!doctype html><html><head><link rel="icon" href="data:,"><style>body{margin:24px}button{box-sizing:border-box}</style></head><body><div id="app"></div><pre id="events"></pre><script type="module">import init,*as f from "/pkg/widget.js";try{await init();f.stop_fixture();window.fixture=f;window.ready=true}catch(e){window.bootError=String(e)}</script></body></html>';
const server=createServer(async(req,res)=>{try{
  const pathname=new URL(req.url,'http://localhost').pathname;
  if(pathname==='/'){res.setHeader('Content-Type','text/html');res.end(html);return;}
  const file=resolve(root,'.'+pathname),child=relative(root,file);
  if(child.startsWith('..')||isAbsolute(child))throw Error('outside fixture');
  res.setHeader('Content-Type',{'.js':'application/javascript','.wasm':'application/wasm'}[extname(file)]||'application/octet-stream');
  res.end(await readFile(file));
}catch{res.writeHead(404);res.end();}});
await new Promise(done=>server.listen(0,'127.0.0.1',done));
const ci=process.argv.includes('--chromium-only');
const allEngines=[['Chrome',chromium,{channel:'chrome'}],['Firefox',firefox,{}],['WebKit',webkit,{}],['Edge',chromium,{channel:'msedge'}]];
const engines=ci?[['Chromium',chromium,{}]]:allEngines.filter(([name])=>!process.argv.includes('--firefox-only')||name==='Firefox');
const views=[[1,0,0],[1.25,-12,-8],[2,-40,-40],[.5,40,30]];
const project=(p,v)=>[p[0]*v[0]+v[1],p[1]*v[0]+v[2]];
const report={passed:false,startedAt:new Date().toISOString(),contexts:[],provenance:{initial},limits:['Real web input; Memory tests are separate. Native OS input is not claimed.']};
try{
for(const[name,engine,options]of engines){
 const browser=await bounded(`${name} launch`,()=>engine.launch({headless:true,...options}));
 try{for(const dpr of[1,2]){
  const record={name,dpr,passed:false,frames:[],inputs:[],rollback:[],steps:[]};report.contexts.push(record);
  const step=async(label,fn)=>{
   record.phase=label;const entry={label,startedAt:new Date().toISOString()};record.steps.push(entry);
   try{const result=await bounded(`${name} DPR${dpr} ${label}`,fn);entry.passed=true;return result;}
   catch(error){entry.passed=false;entry.error=String(error);throw error;}
   finally{entry.finishedAt=new Date().toISOString();}
  };
  let context,page;const errors=[];
  try{
   context=await step('create browser context',()=>browser.newContext({deviceScaleFactor:dpr,viewport:{width:1100,height:800}}));
   context.setDefaultTimeout(15000);
   context.setDefaultNavigationTimeout(15000);
   page=await step('create page',()=>context.newPage());page.on('pageerror',e=>errors.push(String(e)));
   console.log(`${name} DPR${dpr}: opening fixture`);
   await step('navigate fixture',()=>page.goto(`http://127.0.0.1:${server.address().port}/`));
   await step('wait fixture ready',()=>page.waitForFunction(()=>window.ready||window.bootError));
   assert.equal(await step('read boot error',()=>page.evaluate(()=>window.bootError)),undefined);
   await step('mount fixture',()=>page.evaluate(d=>window.fixture.raster_bindings_mount(d),dpr));
   const flush=async()=>{await step('model flush',()=>page.evaluate(()=>window.fixture.raster_bindings_flush()));await step('two animation frames',()=>page.evaluate(()=>Promise.race([new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))),new Promise((_,reject)=>setTimeout(()=>reject(Error('animation frame timeout')),10000))])));await step('image decode',()=>page.locator('#raster-chart-bindings img').evaluate(async n=>{await Promise.race([n.decode(),new Promise((_,reject)=>setTimeout(()=>reject(Error('image decode timeout')),10000))]);}));};
   const state=()=>step('accepted state',()=>page.evaluate(()=>JSON.parse(window.fixture.raster_bindings_state())));
   const hitState=async(physical,source=null)=>{
    const s=await state(),event=s.events.findLast(e=>e.event==='physical-hover');
    assert(event,'real View input must produce a physical observation');
    if(physical===null){assert.equal(event.hit,null);return s;}
    assert.equal(event.hit.physical,physical);assert.equal(event.hit.owner,'owner');
    assert.deepEqual(event.hit.source,source);
    const received=record.inputs.at(-1).received,b=await step('read exact DOM hit viewport',()=>plot.evaluate(n=>{const r=n.getBoundingClientRect();return{x:r.x,y:r.y,width:r.width,height:r.height};})),v=s.accepted.view;
    const expected=[((received.x-b.x)*240/b.width-v[1])/v[0],((received.y-b.y)*160/b.height-v[2])/v[0]];
    (record.coordinateChecks??=[]).push({received,viewport:b,expected,actual:event.hit.point,error:event.hit.point.map((x,i)=>Math.abs(x-expected[i]))});
    assert(event.hit.point.every((x,i)=>Math.abs(x-expected[i])<1e-6),'scenePoint must invert the received host coordinate exactly once');
    return s;
   };
   const update=async(phase,view,hidden=false,fail=false)=>{await step('propose input',()=>page.evaluate(a=>window.fixture.raster_bindings_update(...a),[phase,view,hidden,fail]));await flush();};
   const plot=page.locator('#raster-chart-bindings .kartis-raster-plot');
   const child=page.locator('#raster-chart-bindings .bound-raster-widget');
   await flush();
   await step('install trusted pointer observer',()=>page.evaluate(()=>{
    window.received=[];
    document.addEventListener('pointermove',e=>window.received.push({trusted:e.isTrusted,x:e.clientX,y:e.clientY,target:e.target.tagName}),true);
   }));
   const image=async()=>step('image pixels',()=>page.locator('#raster-chart-bindings img').evaluate(async n=>{
    await Promise.race([n.decode(),new Promise((_,reject)=>setTimeout(()=>reject(Error('image decode timeout')),10000))]);const c=document.createElement('canvas');c.width=n.naturalWidth;c.height=n.naturalHeight;const ctx=c.getContext('2d');ctx.drawImage(n,0,0);
    const r=n.getBoundingClientRect();return{src:n.src,width:c.width,height:c.height,rect:[r.x,r.y,r.width,r.height],pixels:[...ctx.getImageData(0,0,c.width,c.height).data]};
   }));
   const pixel=(im,p)=>{const x=Math.floor(p[0]*dpr),y=Math.floor(p[1]*dpr);return im.pixels.slice((y*im.width+x)*4,(y*im.width+x)*4+4);};
   const move=async(p)=>{
    const b=await step('read pointer plot bounds',()=>plot.boundingBox());assert(b);
    await step(`trusted pointer move ${p.join(',')}`,()=>page.mouse.move(b.x+p[0]*b.width/240,b.y+p[1]*b.height/160));await flush();
    record.inputs.push({at:p,received:await step('read trusted pointer',()=>page.evaluate(()=>window.received.at(-1)))});
    assert.equal(record.inputs.at(-1).received.trusted,true);
   };
   // The remaining checks read accepted fixture data, actual DOM, image pixels and trusted input.
   await step('capture retained Widget identity',()=>page.evaluate(()=>window.__rasterWidget=document.querySelector('.bound-raster-widget')));
   for(let view=0;view<views.length;view++){
    console.log(`${name} DPR${dpr}: camera ${view}`);
    await update(0,view);const im=await image();
    assert.equal(im.width,240*dpr);assert.equal(im.height,160*dpr);
    assert.deepEqual(pixel(im,project([110,80],views[view])),[20,180,100,255]);
    const tail=project([160,105],views[view]);
    const tailVisible=tail[0]>=0&&tail[0]<240&&tail[1]>=0&&tail[1]<160;
    if(tailVisible)assert.deepEqual(pixel(im,tail),[180,90,220,255]);
    await move(project([110,80],views[view]));
    const s=await hitState('long-alias');
    assert.equal(s.accepted.value,null);assert.deepEqual(s.accepted.view,views[view]);assert.equal(s.physicalPlots,1);
    assert.equal(s.controls.length,2);assert(s.controls.some(c=>c.label==='Nullable measurement: missing'));
    assert(s.tooltip.join(' ').includes('missing'));assert(!s.tooltip.join(' ').includes('-913'));
    record.frames.push({view,pngSha256:sha(im.src),state:s});
    if(tailVisible){await move(tail);record.frames.push({view,part:true,state:await hitState('tail-alias',{id:'raw-part-0',value:99})});}
    assert(await step('check retained Widget identity',()=>page.evaluate(()=>window.__rasterWidget===document.querySelector('.bound-raster-widget'))));
   }
   for(const view of[2,3])for(const delta of[2,4]){
    await update(0,view);await move(project([110,80],views[view]));
    const before=(await state()).events.filter(e=>e.event==='selected').length;
    await step('trusted threshold pointer down',()=>page.mouse.down());const b=await step('read threshold plot bounds',()=>plot.boundingBox()),p=project([110,80],views[view]);
    await step(`trusted threshold move ${delta}`,()=>page.mouse.move(b.x+(p[0]+delta)*b.width/240,b.y+p[1]*b.height/160));await step('trusted threshold pointer up',()=>page.mouse.up());await flush();
    const after=(await state()).events.filter(e=>e.event==='selected').length;
    assert.equal(after-before,delta<3?1:0,'visual movement threshold must not change with the image camera');
   }
   // Retained local Widget state and focus participate in the publication rollback.
   console.log(`${name} DPR${dpr}: publication rollback`);
   await update(0,1);console.log(`${name} DPR${dpr}: clicking retained child`);
   await step('trusted retained child click',()=>child.click());
   // Button clicks do not focus in every browser. Establish actual DOM focus
   // explicitly before verifying its retention; click state is checked separately.
   await step('focus retained child',()=>child.focus());
   assert(await step('read actual retained child focus',()=>child.evaluate(n=>document.activeElement===n)),'retained child must actually own DOM focus');
   console.log(`${name} DPR${dpr}: retained child focused`);await flush();
   const before=await image(),beforeState=await state(),beforeText=await step('read child text before rejection',()=>child.textContent());
   await update(1,3,false,true);
   const after=await image(),afterState=await state();
   assert.equal(after.src,before.src);assert.equal(await step('read child text after rejection',()=>child.textContent()),beforeText);assert(await step('read child focus after rejection',()=>child.evaluate(n=>document.activeElement===n)));
   for(const field of['accepted','controls','widget','physicalPlots'])assert.deepEqual(afterState[field],beforeState[field]);
   assert(afterState.events.slice(beforeState.events.length).some(e=>e.event==='publication-rejected'));
   assert(afterState.events.slice(beforeState.events.length).some(e=>e.event==='rejected'));
   assert(await step('check retained Widget identity',()=>page.evaluate(()=>window.__rasterWidget===document.querySelector('.bound-raster-widget'))));
   record.rollback.push({before:beforeState,after:afterState,pngSha256:sha(before.src)});
   await update(1,3);await move(project([138,80],views[3]));
   const accepted=await hitState('long-alias');assert.equal(accepted.accepted.value,0);assert(accepted.controls.some(c=>c.label==='Nullable measurement: 0'));
   assert((await step('read retained child local state',()=>child.textContent())).includes('count=1|focus=true'));record.frames.push({acceptedRetry:true,state:accepted});
   await step('apply nonuniform CSS viewport scale',()=>page.locator('#raster-chart-bindings').evaluate(n=>{n.style.transformOrigin='0 0';n.style.transform='scale(2,1.5)';}));
   // Glory retires the previous bounds lease on its first observation of a
   // changed viewport. Require that cancellation, then start a fresh pointer lease.
   await move(project([138,80],views[3]));await hitState(null);
   assert.equal(await step('verify outside-chart leave point',()=>page.evaluate(()=>{
    const chart=document.querySelector('#raster-chart-bindings'),target=document.elementFromPoint(8,8);
    return target!==null&&!chart.contains(target);
   })),true,'leave target must be real viewport content outside the chart');
   await step('trusted pointer leave scaled chart',()=>page.mouse.move(8,8));await flush();
   await move(project([138,80],views[3]));record.frames.push({nonuniformCssScale:true,state:await hitState('long-alias')});
   await update(1,3,true);assert.equal(await step('read hidden child visibility',()=>child.isVisible()),false);await move(project([138,80],views[3]));await hitState(null);
   await update(1,3,false);assert.equal(await step('read restored child visibility',()=>child.isVisible()),true);
   await step('capture successful frame screenshot',()=>page.screenshot({path:join(out,`${name}-dpr${dpr}.png`)}));
   await step('stop fixture',()=>page.evaluate(()=>window.fixture.raster_bindings_stop()));assert.equal(await step('count roots after stop',()=>page.locator('#raster-chart-bindings').count()),0);
   const stopped=await state();assert.equal(stopped.active,false);assert.equal(stopped.cleanups,1);
   await step('trusted pointer after stop',()=>page.mouse.move(50,50));assert.deepEqual((await state()).events,stopped.events,'disposed owner must not receive further input');
   await step('remount fixture',()=>page.evaluate(d=>window.fixture.raster_bindings_mount(d),dpr));await flush();assert((await step('read remounted child local state',()=>child.textContent())).includes('count=0|focus=false'));await step('stop remounted fixture',()=>page.evaluate(()=>window.fixture.raster_bindings_stop()));
   assert.deepEqual(errors,[]);record.passed=true;
   console.log(`${name} DPR${dpr}: typed CPU Image input and live Widget rollback passed`);
  }catch(error){
   record.error=error.stack;
   if(page){
    await bounded(`${name} DPR${dpr} failure screenshot`,()=>page.screenshot({path:join(out,`${name}-dpr${dpr}-failure.png`)}),5000).catch(()=>{});
    await bounded(`${name} DPR${dpr} failure HTML`,async()=>writeFile(join(out,`${name}-dpr${dpr}-failure.html`),await page.content()),5000).catch(()=>{});
   }
   throw error;
  }finally{if(context)await bounded(`${name} DPR${dpr} context close`,()=>context.close(),5000).catch(()=>{});}
 }}finally{await bounded(`${name} browser close`,()=>browser.close(),5000).catch(()=>{});}
}
report.passed=report.contexts.length===engines.length*2&&report.contexts.every(c=>c.passed);
}finally{
 server.close();report.provenance.end=await hashes();report.provenance.unchanged=JSON.stringify(initial)===JSON.stringify(report.provenance.end);if(!report.provenance.unchanged)report.passed=false;
 report.finishedAt=new Date().toISOString();await writeFile(join(out,'report.json'),JSON.stringify(report,null,2));assert(report.provenance.unchanged,'driver/WASM/glue changed during execution');
}
console.log(JSON.stringify({passed:report.passed,contexts:report.contexts.length,out}));
