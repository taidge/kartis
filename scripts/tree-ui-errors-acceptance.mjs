// Application UI binding faults after a retained typed composition is adopted.
import assert from 'node:assert/strict';
import {readFile,writeFile,mkdir,readdir} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {createServer} from 'node:http';
import {execFileSync} from 'node:child_process';
import {resolve,relative,isAbsolute,extname,join} from 'node:path';
import {pathToFileURL,fileURLToPath} from 'node:url';
import {createRequire} from 'node:module';

const root=resolve('.'),require=createRequire(pathToFileURL(join(root,'package.json')));
const playwright=await import(pathToFileURL(require.resolve('playwright')).href);
const {chromium,webkit}=playwright.default??playwright;
const {captureViewport,rgbaAtLogical}=await import(pathToFileURL(join(root,'scripts/acceptance-viewport.mjs')).href);
const meta=JSON.parse(execFileSync('cargo',['metadata','--no-deps','--format-version','1'],{cwd:root,encoding:'utf8'}));
const arg=n=>process.argv.find(v=>v.startsWith(n+'='))?.slice(n.length+1);
const name=arg('--browser')||(process.argv.includes('--chromium-only')?'Chromium':'Chrome');assert(['Chrome','Chromium','WebKit'].includes(name));
const out=resolve(arg('--out')||join(meta.target_directory,'tree-ui-errors-browser',name));
await mkdir(out,{recursive:true});try{await readFile(join(out,'report.json'));throw Error('Refusing to overwrite prior report');}catch(e){if(e.code!=='ENOENT')throw e;}
const sha=b=>createHash('sha256').update(b).digest('hex');
const step=async(label,promise,ms=30000)=>{let timer;try{return await Promise.race([promise,new Promise((_,reject)=>timer=setTimeout(()=>reject(Error('bounded browser operation: '+label)),ms))]);}finally{clearTimeout(timer);}};
async function rs(dir){return(await Promise.all((await readdir(dir,{withFileTypes:true})).map(e=>e.isDirectory()?rs(join(dir,e.name)):e.name.endsWith('.rs')?[join(dir,e.name)]:[]))).flat();}
const files=[...(await Promise.all(['src','examples/shared','examples/browser-host/src'].map(rs))).flat(),'examples/browser-host/pkg/host.js','examples/browser-host/pkg/host_bg.wasm','scripts/acceptance-viewport.mjs','scripts/acceptance-png.mjs',fileURLToPath(import.meta.url)];
const fence=async()=>Object.fromEntries(await Promise.all(files.map(async p=>[relative(root,resolve(p)).replaceAll('\\','/'),sha(await readFile(p))])));
const report={passed:false,browser:name,dpr:1,scope:'Four renderer/order representatives. A real retained Widget Style binding rejects NaN after typed data adoption; the frame remains accepted, the held old gesture is cancelled, all live Bonds agree, and disabling the application fault restores actual styles and fresh native activation. No OS cancellation or full rendering parity claim.',stopScope:'Public owner inactive return, root removed, and registry inaccessible. This fixture does not export post-stop Rust cleanup counters.',cases:[],provenance:{start:await fence()}};
const server=createServer(async(req,res)=>{try{const p=resolve(root,'.'+new URL(req.url,'http://localhost').pathname),r=relative(root,p);assert(!r.startsWith('..')&&!isAbsolute(r));res.setHeader('Content-Type',{'.html':'text/html','.js':'application/javascript','.wasm':'application/wasm'}[extname(p)]||'application/octet-stream');res.end(await readFile(p));}catch{res.writeHead(404);res.end();}});
await new Promise(done=>server.listen(0,'127.0.0.1',done));
const ids=['a','c','r','b','d'],parents=['r','a',null,'r','a'];
const bit=n=>{const d=new DataView(new ArrayBuffer(8));d.setFloat64(0,n);return d.getBigUint64(0).toString();};
function expectedRaw(phase){const values=phase?[0,35,0,5,10]:[0,30,0,-0,10];return ids.map((id,i)=>({id,parent:parents[i],label:'source-'+id,value:values[i],bits:bit(values[i])}));}
function check(s,phase,focus){
 assert.equal(s.owner.state.active,true);assert.equal(s.owner.state.revision,0);assert.equal(s.owner.state.selected,null);
 assert.deepEqual(s.owner.accepted.raw,expectedRaw(phase),'independent original full source, including exact bits');
 assert.deepEqual(s.live.owner,s.owner.accepted,'actual Frame Bond reader and public accepted metadata');
 assert.equal(s.owner.accepted.activity.focus,focus);
 assert.deepEqual(Object.keys(s.live.rows).sort(),['link:c','node:b']);
 for(const [id,row]of Object.entries(s.live.rows)){
  assert.equal(row.frameId,id);assert.equal(row.revision,0);assert.equal(row.data.index,id==='node:b'?3:2);
  for(const k of['raw','nodes','links','activity'])assert.deepEqual(row.data[k],s.owner.accepted[k],'actual retained live Bond: '+id+'/'+k);
 }
 assert.deepEqual(s.rows.map(r=>r.id).sort(),['link:c','node:b']);
}
const stable=(before,after)=>{assert.deepEqual(after.rows.map(r=>[r.id,r.node,r.count]).sort(),before.rows.map(r=>[r.id,r.node,r.count]).sort(),'same real child NodeIds/counts');assert.equal(after.owner.diagnostics.calls,before.owner.diagnostics.calls,'no new child factories');assert.equal(after.owner.diagnostics.cleanups,before.owner.diagnostics.cleanups,'no child retirements');};
const restoredText=s=>{for(const row of s.rows)for(const k of['raw','nodes','links','activity'])assert.deepEqual(row.data[k],s.live.rows[row.id].data[k],'restored actual Widget text reads live accepted '+k);};
let browser;
try{
 browser=await step('launch',name==='WebKit'?webkit.launch({headless:true}):chromium.launch({headless:true,...(name==='Chrome'?{channel:'chrome'}:{})}),60000);
 report.version=browser.version();
 for(const ordered of[true,false])for(const renderer of['svg','canvas']){
  const c={renderer,ordered,profile:1,flags:5,passed:false,snapshots:[]};report.cases.push(c);
  const context=await step('context',browser.newContext({viewport:{width:1600,height:1200},deviceScaleFactor:1}),60000),page=await step('page',context.newPage(),60000),pageErrors=[];page.on('pageerror',e=>pageErrors.push(e.message));
  try{
   await step('goto',page.goto(`http://127.0.0.1:${server.address().port}/examples/browser-host/index.html`));await step('ready',page.waitForFunction(()=>window.hostReady===true));
   await step('setup',page.evaluate(r=>{document.querySelector('.panels').style.display='none';window.uiRenderer=r;window.uiEvents=[];for(const type of['pointerdown','pointerup','click'])window.addEventListener(type,e=>{if(e.target instanceof Node&&document.querySelector('#tree-activity-'+r)?.contains(e.target))window.uiEvents.push({type,trusted:e.isTrusted,x:e.clientX,y:e.clientY,pointerId:e.pointerId??null,detail:e.detail??null,target:e.target.getAttribute?.('data-gly-node')??null});},true);},renderer));
   const api=(fn,...args)=>step('api '+fn,page.evaluate(([fn,r,...args])=>window.hostFixture['tree_activity_'+fn](r,...args),[fn,renderer,...args]));
   const raf=()=>step('two RAF',page.evaluate(()=>new Promise(done=>requestAnimationFrame(()=>requestAnimationFrame(done)))));
   const settle=async()=>{await raf();await api('flush');await raf();await api('flush');};
   const read=async label=>{const s=await step('read '+label,page.evaluate(r=>({owner:JSON.parse(window.hostFixture.tree_activity_snapshot(r)),live:JSON.parse(window.hostFixture.tree_activity_live_data_read(r)),policy:JSON.parse(window.hostFixture.tree_activity_input_snapshot(r)),rows:[...document.querySelectorAll('#tree-activity-'+r+' .tree-activity-widget')].map(n=>{const m=n.textContent.match(/^(.*)\|count=(\d+)\|focus=(true|false)\|born=(\d+)$/s);if(!m)throw Error('actual Widget label format');const d=JSON.parse(m[1]),s=getComputedStyle(n),b=n.getBoundingClientRect();return{id:d.frameId,node:n.getAttribute('data-gly-node'),count:Number(m[2]),focused:m[3]==='true',born:Number(m[4]),background:s.backgroundColor,width:s.width,height:s.height,rect:[b.x,b.y,b.width,b.height],data:d};}),events:window.uiEvents.slice()}),renderer));s.label=label;c.snapshots.push(s);return s;};
   const surface=page.locator('#tree-activity-'+renderer+' .kartis-surface');
   const shot=async(label,expected)=>{const capture=await step('viewport '+label,captureViewport(page,surface));await writeFile(join(out,`${ordered?'ordered':'overlay'}-${renderer}-${label}.png`),capture.bytes);const samples=[];for(const dx of[-1,0,1])for(const dy of[-1,0,1]){const point=[395+dx,274.6+dy],sample=rgbaAtLogical(capture,{point,size:[720,340]}),delta=sample.rgba.map((v,i)=>Math.abs(v-expected[i]));samples.push({point,...sample,expected,delta});assert(delta.every(v=>v<=1),'actual Widget foreground RGBA1 '+label+' '+sample.rgba);}return{file:`${ordered?'ordered':'overlay'}-${renderer}-${label}.png`,sha256:sha(capture.bytes),geometry:capture.geometry,samples};};
   await step('outside',page.mouse.move(1550,20));await api('mount_input',1,ordered,0,5,false);await settle();await step('reveal',surface.scrollIntoViewIfNeeded());await settle();
   let initial=await read('initial');check(initial,0,null);assert.equal(initial.owner.diagnostics.calls,2);assert.equal(initial.owner.diagnostics.cleanups,0);assert(initial.rows.every(r=>r.count===0&&r.background==='rgb(100, 20, 180)'));
   c.initialPaint=await shot('initial',[100,20,180,255]);
   await api('set_ui_fault',true);await settle();const armed=await read('phase0-armed-valid');check(armed,0,null);stable(initial,armed);assert.deepEqual(armed.policy.eventLog,initial.policy.eventLog,'arming phase0 causes no UI validation error');
   const row=armed.rows.find(r=>r.id==='node:b'),point=[row.rect[0]+row.rect[2]/2,row.rect[1]+row.rect[3]/2].map(Math.round);c.pressPoint=point;
   // Deterministic UI-fault setup, separate from the rapid-input matrix: reveal
   // the target, clear activity explicitly without another pointer move, then
   // press the retained purple Widget in a known neutral accepted frame.
   await step('native Move',page.mouse.move(...point));await settle();await api('clear');await settle();const neutral=await read('neutral-before-down');check(neutral,0,null);stable(armed,neutral);
   await step('native Down',page.mouse.down());await settle();const held=await read('held-phase0');check(held,0,null);stable(neutral,held);const down=held.events.filter(e=>e.type==='pointerdown').at(-1);assert(down?.trusted&&down.pointerId>0,'actual trusted primary contact');assert(Math.abs(down.x-point[0])<=1&&Math.abs(down.y-point[1])<=1);c.down=down;
   await api('input_source');const accepted=await read('phase1-postcommit-error');check(accepted,1,null);stable(held,accepted);
   c.postcommitErrors=accepted.policy.eventLog.slice(held.policy.eventLog.length);assert(c.postcommitErrors.some(e=>e.includes('Error(')&&e.includes('sizes must be finite and nonnegative')),'postcommit BrowserEvent::Error exact recoverable finite-size reason');assert(!c.postcommitErrors.some(e=>/TypeError|RuntimeError/.test(e)));
   assert(accepted.owner.state.error?.includes('sizes must be finite and nonnegative'),'reported error is observable without rejecting adopted phase1');
   await step('old real Up',page.mouse.up());await settle();const released=await read('old-release-cancelled');check(released,1,null);stable(accepted,released);assert.equal(released.rows.find(r=>r.id==='node:b').count,0,'accepted source cancelled old real Widget press');const up=released.events.filter(e=>e.type==='pointerup').at(-1);assert(up?.trusted&&up.pointerId===down.pointerId);assert(Math.abs(up.x-point[0])<=1&&Math.abs(up.y-point[1])<=1);c.up=up;
   await step('outside recover',page.mouse.move(1550,20));await api('set_ui_fault',false);await api('recover');await settle();const restored=await read('fault-off-restored');check(restored,1,null);stable(initial,restored);restoredText(restored);assert(restored.rows.every(r=>r.background==='rgb(100, 20, 180)'));
   const errorCount=restored.policy.eventLog.filter(e=>e.includes('sizes must be finite and nonnegative')).length;
   const box=await step('surface box',surface.boundingBox());assert(box);const hover=[box.x+360*box.width/720,box.y+233.6*box.height/340].map(Math.round);c.actualHover={box,requested:hover,world:[360,233.6]};await step('native source b hover',page.mouse.move(...hover));await settle();const blue=await read('recovered-live-b-active');check(blue,1,'b');stable(initial,blue);assert.equal(blue.live.rows['node:b'].data.active,true);assert.equal(blue.rows.find(r=>r.id==='node:b').background,'rgb(0, 100, 255)');assert.equal(blue.policy.eventLog.filter(e=>e.includes('sizes must be finite and nonnegative')).length,errorCount,'no new validation error after fault disabled');
   restoredText(blue);c.restoredPaint=await shot('blue-restored',[0,100,255,255]);
   const fresh=blue.rows.find(r=>r.id==='node:b'),freshPoint=[fresh.rect[0]+fresh.rect[2]/2,fresh.rect[1]+fresh.rect[3]/2].map(Math.round);await step('fresh hover',page.mouse.move(...freshPoint));await settle();const ready=await read('fresh-down-baseline');const clickBefore=ready.events.filter(e=>e.type==='click'&&e.trusted).length;await step('fresh click',page.mouse.click(...freshPoint));await settle();const clicked=await read('fresh-activation');check(clicked,1,'b');assert.equal(clicked.rows.find(r=>r.id==='node:b').node,fresh.node);assert.equal(clicked.rows.find(r=>r.id==='node:b').count,1);assert.equal(clicked.owner.diagnostics.calls,2);const clicks=clicked.events.filter(e=>e.type==='click'&&e.trusted);assert.equal(clicks.length,clickBefore+1);assert.equal(clicks.at(-1).target,fresh.node);c.freshPoint=freshPoint;
   assert.deepEqual(pageErrors,[],'no uncaught browser errors or WASM trap');c.stopActive=await api('stop');assert.equal(c.stopActive,false);assert.equal(await step('root removed',page.locator('#tree-activity-'+renderer).count()),0);const stopped=await step('stopped registry',page.evaluate(r=>{try{window.hostFixture.tree_activity_snapshot(r);return{accepted:true};}catch(e){if(e instanceof TypeError||e instanceof WebAssembly.RuntimeError)throw e;return{accepted:false,error:String(e)};}},renderer));assert.equal(stopped.accepted,false);assert.equal(stopped.error,'missing Tree activity owner');c.stopped=stopped;c.passed=true;
  }catch(error){c.error=error.stack||String(error);try{c.failure=await step('failure state',page.evaluate(r=>({owner:JSON.parse(window.hostFixture.tree_activity_snapshot(r)),live:JSON.parse(window.hostFixture.tree_activity_live_data_read(r)),policy:JSON.parse(window.hostFixture.tree_activity_input_snapshot(r)),events:window.uiEvents}),renderer));await step('failure screenshot',page.screenshot({path:join(out,`${ordered?'ordered':'overlay'}-${renderer}-failure.png`)}));}catch(e){c.captureError=String(e);}}
  finally{c.pageErrors=pageErrors;await step('close context',context.close());await writeFile(join(out,`${ordered?'ordered':'overlay'}-${renderer}.json`),JSON.stringify(c,null,2)+'\n');}
  console.log(JSON.stringify({browser:name,renderer,ordered,passed:c.passed,error:c.error?.split('\n')[0]}));
 }
 report.passed=report.cases.length===4&&report.cases.every(c=>c.passed);
}catch(error){report.error=error.stack||String(error);}
finally{if(browser)await step('browser close',browser.close());await new Promise(done=>server.close(done));report.provenance.end=await fence();report.provenance.unchanged=JSON.stringify(report.provenance.start)===JSON.stringify(report.provenance.end);assert(report.provenance.unchanged,'protected Rust/package/helpers/this script changed');await writeFile(join(out,'report.json'),JSON.stringify(report,null,2)+'\n');}
console.log(JSON.stringify({passed:report.passed,cases:report.cases.length,out}));if(!report.passed)process.exitCode=1;
