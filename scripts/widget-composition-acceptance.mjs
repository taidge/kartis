// Actual retained force-driver candidates, custom link paint and native input.
import assert from 'node:assert/strict';
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {createServer} from 'node:http';
import {execFileSync} from 'node:child_process';
import {resolve,relative,isAbsolute,extname,join} from 'node:path';
import {chromium,firefox,webkit} from 'playwright';

const root=resolve('.'),metadata=JSON.parse(execFileSync('cargo',['metadata','--offline','--no-deps','--format-version','1'],{encoding:'utf8',windowsHide:true}));
const out=resolve(process.argv[2]||join(metadata.target_directory,'widget-composition-host'));await mkdir(out,{recursive:true});
const sha=x=>createHash('sha256').update(x).digest('hex');
const report={passed:false,scope:'Strict Widget composition with prebuilt hidden child scopes, filtered original mark paint and pointer exclusion. Top-layer composition, not Nivo in-layer equivalence.',contexts:[]};
const server=createServer(async(req,res)=>{try{
  if(!['GET','HEAD'].includes(req.method))throw Error('Read only');
  const file=resolve(root,'.'+decodeURIComponent(new URL(req.url,'http://localhost').pathname)),child=relative(root,file);
  if(child.startsWith('..')||isAbsolute(child))throw Error('Outside root');
  res.setHeader('Content-Type',{'.html':'text/html','.js':'text/javascript','.wasm':'application/wasm','.json':'application/json'}[extname(file)]||'application/octet-stream');res.end(await readFile(file));
}catch{res.writeHead(404);res.end();}});
await new Promise(r=>server.listen(0,'127.0.0.1',r));
const engines=process.argv.includes('--chromium-only')?[['Chromium',chromium,{}]]:[['Chrome',chromium,{channel:'chrome'}],['Firefox',firefox,{}],['WebKit',webkit,{}],['Edge',chromium,{channel:'msedge'}]];
try{for(const [name,engine,opts]of engines){const browser=await engine.launch({headless:true,...opts});try{for(const dpr of [1,2]){
 const ctx=await browser.newContext({viewport:{width:1440,height:1100},deviceScaleFactor:dpr}),page=await ctx.newPage();
 const record={name,dpr,passed:false,frames:[],rejections:[],lifecycle:[]};report.contexts.push(record);
 try{
  const errors=[];page.on('pageerror',e=>errors.push(e.message));
  await page.goto(`http://127.0.0.1:${server.address().port}/examples/browser-host/index.html`);await page.waitForFunction(()=>window.hostReady===true);
  await page.evaluate(()=>{document.querySelector('.panels').style.display='none';if(typeof window.hostFixture.bullet_composition_update!=='function')throw Error('Missing strict composition export');const update=window.hostFixture.bullet_composition_update;window.__compositionUpdate=(r,p,w,v,n,k,m,i)=>update(r,p,w,v,n,k,m|(i<<8));});
  const mount=()=>page.evaluate(()=>{for(const r of ['svg','canvas'])window.hostFixture.bullet_composition_mount(r,0);});
  const flush=async()=>{await page.evaluate(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))));await page.evaluate(()=>window.hostFixture.bullet_composition_flush());};
  const state=r=>page.evaluate(r=>JSON.parse(window.hostFixture.bullet_composition_snapshot(r)),r);
  const buttons=r=>page.locator(`#bullet-composition-${r} [class^="bullet-widget-"]`);
  const update=async({phase=0,width=600,revision=0,note=0,replacement=1,mode=0,invalid=0}={})=>{for(const r of ['svg','canvas'])await page.evaluate(a=>window.__compositionUpdate(...a),[r,phase,width,revision,note,replacement,mode,invalid]);await flush();};
  await mount();await flush();
  const knownIds=['north:温度','south'].flatMap(row=>['range','measure','marker'].map(role=>`bullet:${new TextEncoder().encode(row).length}:${row}:${role}:${role==='marker'?0:1}`));
  const paint=async(label,replacement)=>{
   const p=await page.evaluate(async()=>{
    const svg=document.querySelector('#bullet-composition-svg svg'),canvas=document.querySelector('#bullet-composition-canvas canvas'),copy=svg.cloneNode(true);
    let lang=null;for(let n=svg;n;n=n.parentElement){const x=n.getAttributeNS('http://www.w3.org/XML/1998/namespace','lang')??n.getAttribute('xml:lang');if(x!==null){lang=x;break;}if(n.hasAttribute('lang')){lang=n.getAttribute('lang');break;}}
    lang??=document.documentElement.lang;copy.setAttribute('lang',lang);copy.setAttributeNS('http://www.w3.org/XML/1998/namespace','xml:lang',lang);copy.setAttribute('width',canvas.width/devicePixelRatio);copy.setAttribute('height',canvas.height/devicePixelRatio);
    const image=new Image(),url=URL.createObjectURL(new Blob([new XMLSerializer().serializeToString(copy)],{type:'image/svg+xml'}));
    try{image.src=url;await image.decode();const c=document.createElement('canvas');c.width=canvas.width;c.height=canvas.height;c.getContext('2d').drawImage(image,0,0,c.width,c.height);const a=c.getContext('2d').getImageData(0,0,c.width,c.height).data,b=canvas.getContext('2d').getImageData(0,0,c.width,c.height).data;let total=0;for(let i=0;i<a.length;i++)total+=Math.abs(a[i]-b[i]);const pixel=(x,y)=>Array.from(canvas.getContext('2d').getImageData(Math.floor(x*devicePixelRatio),Math.floor(y*devicePixelRatio),1,1).data);return{mean:total/a.length,width:canvas.width,height:canvas.height,range:pixel(40,30),measure:pixel(40,80),ids:[...svg.querySelectorAll('[data-id]')].map(n=>n.dataset.id),labels:[...svg.querySelectorAll('text')].map(n=>n.textContent).sort()};}finally{URL.revokeObjectURL(url);}
   });assert(p.mean<.5,JSON.stringify({label,p}));assert.deepEqual(p.range,replacement?[255,255,255,255]:[195,203,214,255]);assert.deepEqual(p.measure,replacement===1?[255,255,255,255]:[55,85,130,255]);
   for(const id of knownIds){const suppressed=replacement===1||(replacement===2&&id.includes(':range:'));assert.equal(p.ids.includes(id),!suppressed,`${label}/${id}`);}
   assert(p.labels.includes('north:温度')&&p.labels.includes('south'));record.frames.push({label,...p});
  };
  for(const r of ['svg','canvas']){assert.equal(await buttons(r).count(),6);assert.equal((await state(r)).calls,6);}
  await paint('initial-replacement',1);
  await page.evaluate(()=>{window.__compositionElements=new Map([...document.querySelectorAll('[class^="bullet-widget-"]')].map(b=>[b.closest('[id^="bullet-composition-"]').id+'/'+b.textContent.split('|').slice(0,2).join('|'),b]));});
  const retained=()=>page.evaluate(()=>[...window.__compositionElements].every(([key,node])=>node.isConnected&&node.closest('[id^="bullet-composition-"]').id+'/'+node.textContent.split('|').slice(0,2).join('|')===key));
  for(const r of ['svg','canvas']){const b=page.locator(`#bullet-composition-${r} .bullet-widget-range`).first();await b.click();await b.press('Enter');await flush();assert((await b.textContent()).endsWith('count=2'));}
  const focused=await page.locator('#bullet-composition-canvas .bullet-widget-range').first().elementHandle();
  await update({phase:1,note:41});assert(await retained());assert(await focused.evaluate(b=>document.activeElement===b),'source reorder keeps actual DOM focus');
  for(const r of ['svg','canvas']){assert.equal((await state(r)).calls,6);assert((await buttons(r).allTextContents()).every(t=>t.includes('note=41')));}
  await paint('source-reorder',1);
  // Same Scene and same children; only replacement policy changes.
  await update({phase:1,note:41,replacement:0});assert(await retained());await paint('restore-original-ink',0);
  for(const r of ['svg','canvas']){const surface=page.locator(`#bullet-composition-${r} .kartis-surface`);await surface.scrollIntoViewIfNeeded();const b=await surface.boundingBox(),s=await state(r);await page.mouse.move(b.x+40*b.width/s.width,b.y+30*b.height/s.height);await flush();const hovered=(await state(r)).hovered;assert.equal(hovered.length,1);assert(knownIds.includes(hovered[0])&&hovered[0].includes(':range:1'));record.lifecycle.push({renderer:r,positiveOriginalHit:hovered});}
  await update({phase:1,note:41,replacement:2});assert(await retained());await paint('range-only-replacement',2);
  await update({phase:1,note:42,replacement:1});assert(await retained());await paint('metadata-and-mask',1);
  const witness=()=>page.evaluate(()=>['svg','canvas'].map(r=>{const root=document.getElementById(`bullet-composition-${r}`),s=JSON.parse(window.hostFixture.bullet_composition_snapshot(r));return{width:s.width,height:s.height,revision:s.revision,selected:s.selected,hovered:s.hovered,controls:[...root.querySelectorAll('.kartis-datum')].map(n=>n.outerHTML),widgets:root.querySelector('.kartis-plot-widgets').outerHTML,paint:r==='svg'?root.querySelector('svg').outerHTML:root.querySelector('canvas').toDataURL()};}));
  for(const renderer of ['svg','canvas'])for(const [mode,invalid]of [[1,0],[2,0],[0,1],[0,2],[0,3]]){
   const before=await witness();const rejected=await page.evaluate(a=>{try{window.__compositionUpdate(...a);return false;}catch(error){if(error instanceof TypeError||error instanceof WebAssembly.RuntimeError)throw error;return String(error);}},[renderer,2,520,1,999,0,mode,invalid]);assert(rejected,JSON.stringify({renderer,mode,invalid}));assert.deepEqual(await witness(),before,`${renderer}/${mode}/${invalid} keeps accepted paint, data, children`);assert(await retained());record.rejections.push({renderer,mode,invalid,reason:rejected});
  }
  await flush();assert(await retained());
  // Replaced source rectangles no longer hit at their original location.
  for(const r of ['svg','canvas']){const surface=page.locator(`#bullet-composition-${r} .kartis-surface`);await surface.scrollIntoViewIfNeeded();const b=await surface.boundingBox(),s=await state(r);await page.mouse.move(b.x+40*b.width/s.width,b.y+30*b.height/s.height);await flush();assert.deepEqual((await state(r)).hovered,[]);}
  // A real Canvas publication failure occurs after preflight child construction.
  const rollback=await page.evaluate(()=>{
   const root=document.getElementById('bullet-composition-canvas'),witness=()=>({paint:root.querySelector('canvas').toDataURL(),widgets:root.querySelector('.kartis-plot-widgets').outerHTML,snapshot:(()=>{const s=JSON.parse(window.hostFixture.bullet_composition_snapshot('canvas'));return{active:s.active,width:s.width,height:s.height,revision:s.revision,selected:s.selected,hovered:s.hovered,view:s.view};})(),controls:[...root.querySelectorAll('.kartis-datum')].map(n=>n.outerHTML),errors:JSON.parse(window.hostFixture.bullet_composition_snapshot('canvas')).errors});
   const before=witness();window.__compositionUpdate('canvas',2,520,1,999,0,0,0);
   const original=HTMLCanvasElement.prototype.getContext;let injected=0;
   HTMLCanvasElement.prototype.getContext=function(...args){if(!injected&&this===root.querySelector('canvas')){injected++;throw Error('injected strict composition Canvas paint refusal');}return original.apply(this,args);};
   try{try{window.hostFixture.bullet_composition_flush();}catch{}}finally{HTMLCanvasElement.prototype.getContext=original;}
   const after=witness();return{injected,samePaint:before.paint===after.paint,sameWidgets:before.widgets===after.widgets,sameAcceptedModel:JSON.stringify(before.snapshot)===JSON.stringify(after.snapshot),sameControls:JSON.stringify(before.controls)===JSON.stringify(after.controls),reported:after.errors.some(e=>e.includes('injected strict composition Canvas paint refusal'))};
  });assert(rollback.injected&&rollback.samePaint&&rollback.sameWidgets&&rollback.sameAcceptedModel&&rollback.sameControls&&rollback.reported,JSON.stringify(rollback));assert(await retained());record.rejections.push({publication:rollback});
  await update({phase:2,width:520,note:52});assert(await retained());await paint('resize-recovery',1);
  await page.evaluate(()=>{for(const r of ['svg','canvas'])document.getElementById(`bullet-composition-${r}`).style.transform='scale(.8,1.1)';});await flush();assert(await retained());
  for(const r of ['svg','canvas']){const actual=await page.locator(`#bullet-composition-${r} .kartis-plot-widget-slot`).evaluateAll(a=>a.map(n=>{const b=n.getBoundingClientRect(),base=n.closest('.kartis-plot-widgets').getBoundingClientRect(),[row,role]=n.textContent.split('|'),band=['range','measure','marker'].indexOf(role),rowIndex=row==='north:温度'?0:1;return{actual:[b.x,b.y,b.width,b.height],expected:[base.x+(20+380*(band+1)/4)*base.width/520,base.y+(20+172*rowIndex+6+34*band)*base.height/360,100*base.width/520,26*base.height/360]}}));assert(actual.every(({actual,expected})=>actual.every((v,i)=>Math.abs(v-expected[i])<.1)),JSON.stringify(actual));record.lifecycle.push({renderer:r,css:actual});}
  await paint('nonuniform-css',1);
  await page.evaluate(()=>window.hostFixture.bullet_composition_camera(false));await flush();
  for(const r of ['svg','canvas']){const s=await state(r);assert(s.view[0]>1);const measured=await page.locator(`#bullet-composition-${r} .kartis-plot-widget-slot`).evaluateAll((a,view)=>a.map(n=>{const b=n.getBoundingClientRect(),base=n.closest('.kartis-plot-widgets').getBoundingClientRect(),[row,role]=n.textContent.split('|'),band=['range','measure','marker'].indexOf(role),i=row==='north:温度'?0:1,[k,tx,ty]=view;return{actual:[b.x,b.y,b.width,b.height],expected:[base.x+((20+380*(band+1)/4)*k+tx)*base.width/520,base.y+((20+172*i+6+34*band)*k+ty)*base.height/360,100*k*base.width/520,26*k*base.height/360]}}),s.view);assert(measured.every(({actual,expected})=>actual.every((v,i)=>Math.abs(v-expected[i])<.1)),JSON.stringify(measured));record.lifecycle.push({renderer:r,camera:s.view,measured});}
  assert(await retained());await paint('camera',1);await page.evaluate(()=>window.hostFixture.bullet_composition_camera(true));await flush();
  await update({phase:2,width:520,note:52,invalid:5});assert(await retained());
  for(const r of ['svg','canvas']){const b=page.locator(`#bullet-composition-${r} .bullet-widget-range`).first(),old=await b.textContent();const mask=await b.evaluate(n=>{const p=n.closest('.kartis-plot-widget-clip');return{inert:p.inert,clip:getComputedStyle(p).clipPath}});assert(mask.inert&&mask.clip!=='none',JSON.stringify(mask));await b.scrollIntoViewIfNeeded();const box=await b.boundingBox();await page.mouse.click(box.x+5,box.y+5);await flush();assert.equal(await b.textContent(),old);record.lifecycle.push({renderer:r,partialPathMask:mask});}
  await update({phase:2,width:520,note:52});assert(await retained());await paint('clip-restored',1);
  const beforeGroup=Object.fromEntries(await Promise.all(['svg','canvas'].map(async r=>[r,(await state(r)).calls])));
  await update({phase:2,width:520,note:52,invalid:6});assert(await retained());
  for(const r of ['svg','canvas']){assert.equal(await buttons(r).count(),6);assert.equal(await buttons(r).first().isVisible(),false);assert.equal((await state(r)).calls,beforeGroup[r]);const inert=await buttons(r).first().evaluate(n=>n.closest('.kartis-plot-widget-clip').inert);assert(inert);record.lifecycle.push({renderer:r,groupZeroHidden:true});}
  await update({phase:2,width:520,note:52,invalid:7});assert(await retained());for(const r of ['svg','canvas']){assert(await buttons(r).first().isVisible());assert.equal((await state(r)).calls,beforeGroup[r]);assert((await buttons(r).allTextContents()).some(t=>t.endsWith('count=2')));}
  await paint('positive-opacity-group',1);await update({phase:2,width:520,note:52});
  // New revision construction is allowed to read accepted handles, and nested
  // composition construction must reject before it can mutate the active host.
  await update({phase:3,width:520,revision:3,note:63,mode:3});for(const r of ['svg','canvas'])assert((await state(r)).reads>=6);
  await update({phase:3,width:520,revision:4,note:64,mode:4});for(const r of ['svg','canvas'])assert((await state(r)).reads>=12);
  await update({phase:3,width:520,revision:4,note:64,invalid:4,replacement:0});for(const r of ['svg','canvas']){assert.equal(await buttons(r).count(),0);assert((await state(r)).cleanupReads>0);}
  for(const renderer of ['svg','canvas'])for(const mode of [1,2]){const before=await witness();const rejected=await page.evaluate(a=>{try{window.__compositionUpdate(...a);return false;}catch(error){if(error instanceof TypeError||error instanceof WebAssembly.RuntimeError)throw error;return String(error);}},[renderer,0,600,6,999,1,mode,0]);assert(rejected);assert.deepEqual(await witness(),before);assert.equal(await buttons(renderer).count(),0);record.rejections.push({renderer,newKeys:true,mode,reason:rejected});}
  await update({phase:0,width:600});await flush();await paint('readded-children',1);
  for(const renderer of ['svg','canvas']){const burst=await page.evaluate(renderer=>{let accepted=0,reason=null;for(let revision=100;revision<200;revision++){try{window.__compositionUpdate(renderer,0,600,revision,0,1,0,0);accepted++;}catch(error){if(error instanceof TypeError||error instanceof WebAssembly.RuntimeError)throw error;reason=String(error);break;}}return{accepted,reason};},renderer);assert(burst.accepted>0&&burst.accepted<100&&burst.reason?.includes('pending composition Widget budget exceeded'),JSON.stringify(burst));record.lifecycle.push({renderer,burst});await flush();assert.equal(await buttons(renderer).count(),6);await page.evaluate(a=>window.__compositionUpdate(...a),[renderer,0,600,201,0,1,3,0]);await flush();assert.equal(await buttons(renderer).count(),6);}
  for(const r of ['svg','canvas']){const rejected=await page.evaluate(a=>{try{window.__compositionUpdate(...a);return false;}catch(error){if(error instanceof TypeError||error instanceof WebAssembly.RuntimeError)throw error;return String(error);}},[r,1,600,9,99,1,5,0]);assert(rejected);record.lifecycle.push({renderer:r,stopReason:rejected});assert.equal((await state(r)).active,false);}
  await page.evaluate(()=>window.hostFixture.bullet_composition_stop());assert.equal(await page.locator('[id^="bullet-composition-"]').count(),0);
  for(const r of ['svg','canvas'])for(const mode of [1,2]){const failed=await page.evaluate(([r,mode])=>{try{window.hostFixture.bullet_composition_mount(r,mode);return false;}catch(error){if(error instanceof TypeError||error instanceof WebAssembly.RuntimeError)throw error;return true;}},[r,mode]);assert(failed);assert.equal(await page.locator(`#bullet-composition-${r}`).count(),0);record.lifecycle.push({renderer:r,initialRejectedMode:mode});}
  await mount();await flush();await page.evaluate(()=>window.hostFixture.bullet_composition_stop());assert.deepEqual(errors,[]);record.passed=true;console.log(`${name} DPR${dpr}: strict composition paint/state/input/rollback/lifecycle passed`);
 }catch(error){record.error=error.stack;throw error;}finally{await ctx.close();}
}}finally{await browser.close();}}
report.passed=report.contexts.length===engines.length*2&&report.contexts.every(c=>c.passed);
}finally{server.close();report.wasmSha256=sha(await readFile('examples/browser-host/pkg/host_bg.wasm'));report.scriptSha256=sha(await readFile(new URL(import.meta.url)));await writeFile(join(out,'report.json'),JSON.stringify(report,null,2));}
console.log(JSON.stringify({passed:report.passed,contexts:report.contexts.length,out}));
