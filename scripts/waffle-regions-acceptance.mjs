// Independent logical regions retain real ink picking and unmodified exports.
import assert from 'node:assert/strict';
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {createServer} from 'node:http';
import {execFileSync} from 'node:child_process';
import {resolve,relative,isAbsolute,extname,join} from 'node:path';
import {chromium,firefox,webkit} from 'playwright';
import {png,pixel} from './acceptance-png.mjs';
const meta=JSON.parse(execFileSync('cargo',['metadata','--no-deps','--format-version','1'],{encoding:'utf8'}));
const root=resolve('.'),out=resolve(process.argv.slice(2).find(x=>!x.startsWith('--'))||join(meta.target_directory,'waffle-regions-browser'));
await mkdir(out,{recursive:true});
const sha=b=>createHash('sha256').update(b).digest('hex');
const files={driver:new URL(import.meta.url),helper:new URL('./acceptance-png.mjs',import.meta.url),wasm:'examples/browser-host/pkg/host_bg.wasm',glue:'examples/browser-host/pkg/host.js'};
const hashes=async()=>Object.fromEntries(await Promise.all(Object.entries(files).map(async([k,p])=>[k,sha(await readFile(p))]))),initial=await hashes();
async function bounded(label,fn,ms=15000){let timer;try{return await Promise.race([Promise.resolve().then(fn),new Promise((_,reject)=>{timer=setTimeout(()=>reject(Error(label+' timed out')),ms);})]);}finally{clearTimeout(timer);}}
const server=createServer(async(req,res)=>{try{const p=resolve(root,'.'+new URL(req.url,'http://localhost').pathname),r=relative(root,p);assert(!r.startsWith('..')&&!isAbsolute(r));res.setHeader('Content-Type',{'.html':'text/html','.js':'application/javascript','.wasm':'application/wasm'}[extname(p)]||'application/octet-stream');res.end(await readFile(p));}catch{res.writeHead(404);res.end();}});
await new Promise(done=>server.listen(0,'127.0.0.1',done));
const availableEngines=process.argv.includes('--chromium-only')?[['Chromium',chromium,{}]]:[['Chrome',chromium,{channel:'chrome'}],['Firefox',firefox,{}],['WebKit',webkit,{}],['Edge',chromium,{channel:'msedge'}]];
const selectedEngine=process.argv.find(x=>x.startsWith('--browser='))?.split('=')[1];
const engines=availableEngines.filter(([name])=>!selectedEngine||name===selectedEngine);assert(engines.length>0,'unknown browser selection');
const report={passed:false,scope:'Waffle explicit full-slot business regions, one actual canonical group control, trusted SVG/Canvas pointer/keyboard input and exact invalid-candidate rollback. Physical ink is observed separately. No React lifecycle, arbitrary export, native OS transport or shared-edge Nivo equivalence claim.',pixelTolerance:1,inputCoordinateTolerance:1,physicalScenePointTolerance:1/32,inputCoordinateScope:'Native OS/backend screen request quantization only; accepted logical picking uses actual received client coordinates. A blank Firefox control independently reproduces up to 0.817 CSS px error.',provenance:{start:initial},contexts:[]};
try{for(const[name,engine,options]of engines){const browser=await bounded('launch '+name,()=>engine.launch({headless:true,...options}));try{for(const dpr of[1,2]){
 const record={name,dpr,passed:false,frames:[],inputs:[],rejections:[]};report.contexts.push(record);
 const step=(label,fn)=>{record.phase=label;return bounded(`${name} DPR${dpr} ${label}`,fn);};
 const context=await step('context',()=>browser.newContext({viewport:{width:1200,height:1000},deviceScaleFactor:dpr}));context.setDefaultTimeout(15000);
 const page=await step('page',()=>context.newPage()),errors=[];page.on('pageerror',e=>errors.push(e.message));
 try{
 await step('navigate',()=>page.goto(`http://127.0.0.1:${server.address().port}/examples/browser-host/index.html`));await step('ready',()=>page.waitForFunction(()=>window.hostReady===true));
 await step('mount',()=>page.evaluate(()=>{document.querySelector('.panels').style.display='none';window.moves=[];window.clicks=[];window.addEventListener('pointermove',e=>window.moves.push({trusted:e.isTrusted,x:e.clientX,y:e.clientY}),true);window.addEventListener('click',e=>window.clicks.push({trusted:e.isTrusted,x:e.clientX,y:e.clientY}),true);window.hostFixture.waffle_regions_mount();}));
 const flush=async()=>{await step('frames',()=>page.evaluate(()=>new Promise(done=>requestAnimationFrame(()=>requestAnimationFrame(done)))));await step('flush',()=>page.evaluate(()=>window.hostFixture.waffle_regions_flush()));};
 const state=r=>step('state '+r,()=>page.evaluate(r=>JSON.parse(window.hostFixture.waffle_regions_snapshot(r)),r));
 const surface=r=>page.locator(`#waffle-regions-${r} .kartis-surface`),controls=r=>page.locator(`#waffle-regions-${r} .kartis-datum`);
 const physical=s=>s.events.findLast(e=>e.kind==='physical')?.hit??null;
 const move=async(r,p)=>{await step('reveal',()=>surface(r).scrollIntoViewIfNeeded());const b=await step('bounds',()=>surface(r).boundingBox()),s=await state(r);const requested=[b.x+p[0]/s.width*b.width,b.y+p[1]/s.height*b.height].map(Math.round);await step('trusted move',()=>page.mouse.move(...requested));await flush();const received=await step('actual move',()=>page.evaluate(()=>window.moves.at(-1)));assert.equal(received.trusted,true);record.lastMove={renderer:r,sourcePoint:p,requested,received};assert(Math.abs(received.x-requested[0])<=1&&Math.abs(received.y-requested[1])<=1,JSON.stringify(record.lastMove));const accepted=await state(r),actualSourcePoint=[(received.x-b.x)*s.width/b.width,(received.y-b.y)*s.height/b.height],expectedRegion=[...accepted.regions].reverse().find(region=>!accepted.hidden.includes(accepted.sites.find(site=>site.id===region.id).series)&&region.bounds.some(q=>actualSourcePoint[0]>=q[0]&&actualSourcePoint[0]<q[0]+q[2]&&actualSourcePoint[1]>=q[1]&&actualSourcePoint[1]<q[1]+q[3]));assert.deepEqual(accepted.hovered,expectedRegion?[expectedRegion.id]:[],'business picking follows actual received coordinates');const ink=physical(accepted);if(ink)assert(ink.point.every((v,i)=>Math.abs(v-actualSourcePoint[i])<=1/32),'physical scene coordinates use actual received input');return{sourcePoint:p,requested,received,actualSourcePoint,state:accepted};};
 const outside=async()=>{await step('outside',()=>page.mouse.move(1100,900));await flush();};
 await flush();
 for(const phase of[0,1,2,3,4,5,6,7]){
  await outside();await step('update',()=>page.evaluate(p=>window.hostFixture.waffle_regions_update(p,320,0),phase));await flush();
  for(const r of['svg','canvas']){
   const s=await state(r),region=s.regions[0],site=s.sites.find(x=>x.id===region.id),b=region.bounds[0],gap=[b[0]+1,b[1]+b[3]/2],center=[b[0]+b[2]/2,b[1]+b[3]/2];
   assert.equal(await step('group controls',()=>controls(r).count()),phase===2?1:2);assert.equal(s.regions.length,phase===2?1:2);
   await outside();const hit=await move(r,center);assert.deepEqual(hit.state.hovered,[region.id]);
   if(phase===0){assert.equal(physical(hit.state)?.physical,region.id);assert.equal(physical(hit.state)?.logical,region.id);}
   if(phase===1||phase===4)assert.equal(physical(hit.state),null);
   const atGap=await move(r,gap);assert.deepEqual(atGap.state.hovered,[region.id]);assert.equal(atGap.state.interaction.rows[0].value,site.value);assert.equal(atGap.state.interaction.rows[0].series,site.series);
   if(phase!==3)assert.equal(physical(atGap.state),null,'padding creates a logical hit without physical ink');
   const activations=atGap.state.events.filter(e=>e.kind==='activated').length,clicksBefore=await step('click count before',()=>page.evaluate(()=>window.clicks.length));
   await step('trusted click',()=>page.mouse.click(...atGap.requested));await flush();const clicked=await state(r);assert.equal(clicked.selected,region.id);assert.equal(clicked.events.filter(e=>e.kind==='activated').length-activations,1,'this exact trusted click activates once');assert.equal(clicked.events.findLast(e=>e.kind==='activated').id,region.id);assert.equal(await step('click delta',()=>page.evaluate(()=>window.clicks.length)),clicksBefore+1);assert.equal((await step('click receipt',()=>page.evaluate(()=>window.clicks.at(-1)))).trusted,true);
   const second=region.bounds[1],aliasPoint=[second[0]+second[2]/2,second[1]+second[3]/2];
   if(phase===0){const alias=await move(r,aliasPoint);assert.deepEqual(alias.state.hovered,[region.id]);assert.notEqual(physical(alias.state)?.physical,region.id);assert.equal(physical(alias.state)?.logical,region.id);record.inputs.push({renderer:r,phase,kind:'alias',...alias});}
   if(phase===4){const offset=await move(r,[center[0]+16,center[1]]);assert.equal(physical(offset.state)?.physical,region.id);assert.equal(physical(offset.state)?.logical,region.id);assert.deepEqual(offset.state.hovered,[region.id]);record.inputs.push({renderer:r,phase,kind:'offset-path-ink',...offset});}
   const empty=s.cells.find(c=>c.owner===null).bounds,miss=await move(r,[empty[0]+empty[2]/2,empty[1]+empty[3]/2]);assert.deepEqual(miss.state.hovered,[]);assert.equal(physical(miss.state),null);
   record.inputs.push({renderer:r,phase,kind:'group',center:hit,gap:atGap});
   await outside();await step('reveal screenshot',()=>surface(r).scrollIntoViewIfNeeded());const bytes=await step('screenshot',()=>surface(r).screenshot()),image=png(bytes),samples=[];
   // Solid interiors only: group first-center and padding, plus actual empty paint.
   const points=[center,gap,[empty[0]+empty[2]/2,empty[1]+empty[3]/2]];if(phase===4)points.push([center[0]+16,center[1]]);
   const expected=await step('independent source colors',()=>page.evaluate(({svg,phase,id})=>{
    const doc=new DOMParser().parseFromString(svg,'image/svg+xml'),paths=[...doc.querySelectorAll('path,rect,circle')].filter(n=>n.getAttribute('data-id')?.startsWith('waffle-'));
    // The known source palette and independent fixture geometry determine samples.
    const ink=paths.find(n=>n.getAttribute('data-id')===id);
    const color=n=>{const c=n?.getAttribute('fill');if(!c)throw Error('missing source fill');const m=/^rgba\((\d+),(\d+),(\d+),([0-9.]+)\)$/.exec(c);if(!m)throw Error('unsupported source color '+c);if(Number(m[4])!==1)throw Error('unexpected source alpha');return m.slice(1,4).map(Number).concat(255);};
    const fill=color(ink),result=[phase===1||phase===4?[255,255,255,255]:fill,phase===3?fill:[255,255,255,255],phase===1||phase===4?[255,255,255,255]:[204,204,204,255]];if(phase===4)result.push(fill);return result;
   },{svg:s.svg,phase,id:region.id}));
   for(let i=0;i<points.length;i++){const actual=pixel(image,...points[i],s.width,s.height),delta=actual.map((x,j)=>Math.abs(x-expected[i][j]));assert(delta.every(x=>x<=1),`${name}/${dpr}/${r}/${phase}/${i}: ${actual} vs ${expected[i]}`);samples.push({point:points[i],actual,expected:expected[i],delta});}
   record.frames.push({renderer:r,phase,samples,pngSha256:sha(bytes)});
  }
 }
 // Host visibility suppresses logical regions while preserving their binding.
 await step('initial again',()=>page.evaluate(()=>window.hostFixture.waffle_regions_update(0,320,0)));await flush();
 for(const r of['svg','canvas']){const s=await state(r),b=s.regions[0].bounds[0];await step('hide alpha',()=>page.evaluate(()=>window.hostFixture.waffle_regions_hidden(true)));await flush();const h=await move(r,[b[0]+1,b[1]+b[3]/2]);assert.deepEqual(h.state.hovered,[]);await step('restore alpha',()=>page.evaluate(()=>window.hostFixture.waffle_regions_hidden(false)));await flush();}
 // Invalid synchronous setters must preserve revision, focus, raw DOM and pixels.
 for(let invalid=1;invalid<=4;invalid++){
  await step('settled focus control',()=>controls('canvas').first().focus());await flush();
  const before={};for(const r of['svg','canvas']){before[r]={state:await state(r),dom:await step('DOM',()=>page.locator(`#waffle-regions-${r}`).evaluate(n=>n.outerHTML)),png:sha(await step('before PNG',()=>surface(r).screenshot()))};}
  const focused=await step('focused identity',()=>page.evaluate(()=>{window.regionFocus=document.activeElement;return document.activeElement.outerHTML;}));
  await step('rejected candidate',()=>page.evaluate(i=>window.hostFixture.waffle_regions_update(0,320,i),invalid));
  for(const r of['svg','canvas']){const after=await state(r);assert.deepEqual(after,before[r].state);assert.equal(await step('DOM preserved',()=>page.locator(`#waffle-regions-${r}`).evaluate(n=>n.outerHTML)),before[r].dom);assert.equal(sha(await step('after PNG',()=>surface(r).screenshot())),before[r].png);record.rejections.push({renderer:r,invalid,revision:after.revision});}
  assert(await step('focus preserved',()=>page.evaluate(()=>window.regionFocus===document.activeElement)));assert.equal(await step('focus text',()=>page.evaluate(()=>document.activeElement.outerHTML)),focused);
 }
 for(const r of['svg','canvas']){await step('keyboard focus',()=>controls(r).last().focus());const s=await state(r);await step('keyboard activate',()=>page.keyboard.press('Enter'));await flush();const after=await state(r);const expected=s.sites.find(site=>site.series==='beta').id;assert.equal(after.events.filter(e=>e.kind==='activated').length-s.events.filter(e=>e.kind==='activated').length,1);assert.equal(after.events.findLast(e=>e.kind==='activated').id,expected);assert.equal(after.selected,expected);}
 await step('stop',()=>page.evaluate(()=>window.hostFixture.waffle_regions_stop()));assert.equal(await step('removed',()=>page.locator('[id^="waffle-regions-"]').count()),0);assert.deepEqual(errors,[]);record.passed=true;
 }catch(error){record.error=error.stack;await writeFile(join(out,`${name}-${dpr}-failure.html`),await page.content());throw error;}finally{await context.close();}
}}finally{await browser.close();}}
report.provenance.end=await hashes();assert.deepEqual(report.provenance.end,initial);report.passed=true;
}catch(error){report.error=error.stack;throw error;}finally{report.finishedAt=new Date().toISOString();await writeFile(join(out,'report.json'),JSON.stringify(report,null,2));await new Promise(done=>server.close(done));}
console.log(JSON.stringify({passed:report.passed,contexts:report.contexts.length,frames:report.contexts.reduce((n,c)=>n+c.frames.length,0),report:join(out,'report.json')}));
