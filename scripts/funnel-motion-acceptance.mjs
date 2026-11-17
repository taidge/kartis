// Actual Funnel property motion, immutable source cohorts and retained Glory UI.
import assert from 'node:assert/strict';
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {resolve,join,relative,extname} from 'node:path';
import {createHash} from 'node:crypto';
import {createServer} from 'node:http';
import {gzipSync} from 'node:zlib';
import {chromium,firefox,webkit} from 'playwright';
import {png} from './acceptance-png.mjs';
import {createOracle} from './funnel-motion-oracle.mjs';
import {selectFunnelPoint} from './funnel-motion-paint-reference.mjs';

const flags=Object.fromEntries(process.argv.slice(2).map(v=>{const i=v.indexOf('=');assert(i>2);return[v.slice(2,i),v.slice(i+1)];}));
assert(flags.pkg&&flags.out,'--pkg=<source-bound producer package> --out=<new directory>');
const root=resolve('.'),pkg=resolve(flags.pkg),out=resolve(flags.out),sha=b=>createHash('sha256').update(b).digest('hex');
await mkdir(out,{recursive:false});
const producerPath=resolve(pkg,'../report.json'),producerBytes=await readFile(producerPath),producer=JSON.parse(producerBytes);
assert(producer.passed&&producer.sourceUnchanged);
for(const[p,h]of Object.entries(producer.sourceEnd))assert.equal(sha(await readFile(resolve(p))),h,'compiled source '+p);
for(const[p,h]of Object.entries(producer.package))assert.equal(sha(await readFile(join(pkg,p))),h,'exact package '+p);
const helpers=['funnel-motion-acceptance.mjs','funnel-motion-oracle.mjs','funnel-motion-reference.mjs','funnel-motion-paint-reference.mjs','acceptance-png.mjs'];
for(const h of helpers)await writeFile(join(out,'executed-'+h),await readFile(resolve('scripts',h)),{flag:'wx'});
const inputs=[...new Set([...Object.keys(producer.sourceEnd),...helpers.map(n=>'scripts/'+n),'package.json','package-lock.json','README.zh-CN.md','tests/fixtures/funnel-motion/shape-generators.mjs'].map(p=>resolve(p)))];
const fence=async()=>Object.fromEntries(await Promise.all(inputs.map(async p=>[relative(root,p).replaceAll('\\','/'),sha(await readFile(p))])));
const report={passed:false,featureAccepted:false,broadGoalComplete:false,scope:'Owned Funnel property springs, actual sampled SVG/Canvas, retained application UI and native physical admission. Explicit --disable-gpu Chromium configurations; no React runtime or GPU/worker equivalence.',producer:{path:relative(root,producerPath),sha256:sha(producerBytes),package:producer.package},sourceStart:await fence(),contexts:[]};
const html='<!doctype html><html lang="en"><meta charset="utf-8"><style>body{margin:0;background:white}.fm-button,.fm-tip{padding:0!important;border:0!important;margin:0!important;border-radius:0!important;outline:none!important;box-shadow:none!important;appearance:none!important;box-sizing:border-box!important;min-width:0!important;min-height:0!important;font:0px sans-serif!important}.fm-button{background:rgb(15,160,95)!important}.fm-tip{background:rgb(170,60,200)!important}</style><div style="position:absolute;left:2000px"><div id="svg-root" style="width:520px;height:340px"></div><div id="canvas-root" style="width:520px;height:340px"></div></div><script type="module">import init,* as host from "/pkg/host.js";await init();window.host=host;window.ready=true;</script></html>';
const server=createServer(async(req,res)=>{try{const u=new URL(req.url,'http://localhost');if(u.pathname==='/'){res.setHeader('Content-Type','text/html');res.end(html);return;}assert(u.pathname.startsWith('/pkg/'));const p=resolve(pkg,u.pathname.slice(5));assert(!relative(pkg,p).startsWith('..'));res.setHeader('Content-Type',{'.js':'application/javascript','.wasm':'application/wasm'}[extname(p)]??'application/octet-stream');res.end(await readFile(p));}catch{res.writeHead(404);res.end();}});
await new Promise(done=>server.listen(0,'127.0.0.1',done));
const engines={Chrome:[chromium,{channel:'chrome',args:['--disable-gpu']}],Edge:[chromium,{channel:'msedge',args:['--disable-gpu']}],Chromium:[chromium,{args:['--disable-gpu']}],Firefox:[firefox,{}],WebKit:[webkit,{}]};
const near=(a,b,message)=>assert(Math.abs(a-b)<=1e-7*Math.max(1,Math.abs(a),Math.abs(b)),message+': '+a+' vs '+b);
const pathTokens=d=>(d.match(/[MLCZ]|[-+]?(?:\d*\.\d+|\d+\.?\d*)(?:[eE][-+]?\d+)?/g)??[]).map(v=>/^[MLCZ]$/.test(v)?v:Number(v));
const commandTokens=cs=>cs.flatMap(c=>c);
function pathEquals(d,commands){const a=pathTokens(d),b=commandTokens(commands);assert.equal(a.length,b.length,'actual path token count');a.forEach((v,i)=>typeof v==='string'?assert.equal(v,b[i],'actual path topology'):near(v,b[i],'actual path number'));}
function peer(row,o,kind){
 if(row.cleaned){assert(!row.location.connected);assert(row.frame.Err&&row.motion.Err);return;}
 if(kind==='tip'&&row.location.inTip&&!o.tooltip){assert.equal(row.frame.Ok,null);assert.equal(row.motion.Ok,null);return;}
 const active=kind==='layer'?row.location.accepted:row.location.inTip&&!!o.tooltip;
 if(active){assert(row.frame.Ok);assert.deepEqual(row.frame.Ok.data,o.data);assert.equal(row.frame.Ok.generation,o.generation);assert.equal(row.frame.Ok.publication,o.publication);assert.deepEqual(row.motion.Ok,o.motion,'retained peer physical and target cohort');}
}
function check(s,{allowError=false}={}){
 const o=s.owner;assert(o.active);assert(o.data);assert.deepEqual(o.uiErrors,[]);assert.deepEqual(o.eventErrors,[]);if(!allowError)assert.equal(o.error,null);
 if(o.motion){assert.deepEqual(o.motion.target,o.data.computed);assert.deepEqual(o.motion.targetScene,o.data.scene);assert.equal(o.motion.generation,o.generation);assert.equal(o.motion.publication,o.publication);assert.equal(o.motion.configuration,o.motionState.configuration);assert.equal(o.motion.scene.marks.length,o.data.scene.marks.length,'removed roles have no ghost ink');}
 s.peers.layers.forEach(r=>peer(r,o,'layer'));s.peers.tips.forEach(r=>peer(r,o,'tip'));
 if(o.tooltip){assert.deepEqual(o.tooltip.data,o.data);assert.equal(o.tooltip.generation,o.generation);assert.equal(o.tooltip.publication,o.publication);}
}
function entries(rows,native,initialOwner){for(const e of rows){const v=e.value;
 if(e.kind==='effect'){
  const active=v.role==='layer'?v.location.accepted:v.location.inTip&&!!v.owner?.tooltip;
  if(active&&v.owner){assert.deepEqual(v.own.data,v.owner.data);assert.equal(v.own.generation,v.owner.generation);assert.equal(v.own.publication,v.owner.publication);assert.deepEqual(v.motion,v.owner.motion);assert.deepEqual(v.cohort,{data:true,pool:true,motion:true,target:v.motion?true:null});v.peers.layers.forEach(r=>peer(r,v.owner,'layer'));v.peers.tips.forEach(r=>peer(r,v.owner,'tip'));}
  if(active&&!v.owner){assert.equal(v.motion,null);assert.equal(v.own.publication,0,'first mount effect precedes fixture handle');assert.equal(v.own.generation,0);assert.deepEqual(v.own.data,initialOwner.data,'initial mount source reconciles with returned owner');}
 }else if(e.kind==='native'){
  const n=native.get(v.seq);assert(n,'actual original Window capture');assert(n.trusted&&v.trusted);assert.equal(n.phase,1);assert.equal(v.eventPhase,3);assert.equal(n.type,v.type);assert.deepEqual(n.client,v.client);assert(n.active);assert.equal(n.occurrence.kind,'parts');
  assert.deepEqual(v.data,n.owner.data);assert.deepEqual(v.motion,n.owner.motion,'historical event physical certificate');assert.equal(v.publication,n.owner.publication);assert.equal(v.generation,n.owner.generation);assert.equal(v.occurrence.key,n.occurrence.key);assert.equal(v.occurrence.index,n.occurrence.index);assert.deepEqual(v.cohort,{data:true,pool:true,motion:true,target:v.motion?true:null});
  const part=v.data.computed.parts[v.partIndex];assert(part);assert.equal(n.target,part.body,'native target matches actual admitted business role');assert.equal(v.sourceIndex,part.index);
  const sample=(n.owner.motion?.scene??n.owner.data.scene).marks.find(m=>m.id===n.target);assert(sample);pathEquals(n.physicalPath,sample.geometry.commands);
  const [left,top,width,height]=n.surface,[k,tx,ty]=n.owner.view,viewport=[(v.client[0]-left)*600/width,(v.client[1]-top)*420/height],physical=[40+(viewport[0]-40-tx)/k,28+(viewport[1]-28-ty)/k];v.scene.forEach((x,i)=>near(x,physical[i],'capture-time physical input coordinates'));
 }else if(e.kind==='callbackStep'){assert(v.rejected&&v.unchanged);assert.match(v.error,/callback|dispatch|reentrant|inside mouse handler/i);}
}}
let failed=false;
try{for(const name of flags.browser?[flags.browser]:Object.keys(engines)){
 assert(engines[name]);const[engine,launch]=engines[name],browser=await engine.launch({headless:true,...launch});
 try{for(const dpr of flags.dpr?[Number(flags.dpr)]:[1,2]){
  const context=await browser.newContext({viewport:{width:1400,height:900},deviceScaleFactor:dpr}),page=await context.newPage(),errors=[],entry={engine:name,dpr,launch,version:browser.version(),passed:false,profiles:[]};report.contexts.push(entry);page.on('pageerror',e=>errors.push(String(e?.stack??e)));
  try{await page.goto('http://127.0.0.1:'+server.address().port);await page.waitForFunction(()=>window.ready);
   await page.evaluate(()=>{
    window.fmNative=[];window.fmSeq=0;window.fmNodes=new WeakMap();window.fmNext=0;
    for(const type of ['pointerover','pointermove','pointerout','click'])window.addEventListener(type,e=>{
     const root=e.target.closest?.('[id^="funnel-motion-"]');if(!root)return;
     const seq=++window.fmSeq;Object.defineProperty(e,'fmSeq',{value:seq});const g=e.target.closest('.kartis-funnel-svg-occurrence');
     const active=e.target.closest('.kartis-funnel-svg-layers[data-funnel-svg-active="true"]'),rect=active?.getBoundingClientRect();
     let owner;try{owner=JSON.parse(window.host.funnel_motion_owner(root.id.slice('funnel-motion-'.length)));}catch(error){owner={Err:String(error)};}
     window.fmNative.push({seq,type,trusted:e.isTrusted,phase:e.eventPhase,client:[e.clientX,e.clientY],root:root.id,target:e.target.getAttribute?.('data-id'),physicalPath:e.target.getAttribute?.('d'),
      surface:rect?[rect.x,rect.y,rect.width,rect.height]:null,active:!!active,
      occurrence:g?{key:g.getAttribute('data-funnel-svg-key'),index:Number(g.getAttribute('data-funnel-svg-index')),kind:g.getAttribute('data-funnel-svg-kind')}:null,owner});
    },true);
   });
   const settle=()=>page.evaluate(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))));
   for(const profile of flags.profile?[Number(flags.profile)]:[0,1,2,3]){
    const key=`${name}-${dpr}-${profile}`,selector='#funnel-motion-'+key,p={profile,passed:false,frames:[],negatives:[],steps:[],rejections:[],eventBatches:[]},native=new Map(),ledger=[];entry.profiles.push(p);let oracle,lastOwner,mode=0,config={mass:1,stiffness:170,damping:26};
    const read=async()=>{
     const r=await page.evaluate(key=>({snapshot:JSON.parse(window.host.funnel_motion_read(key)),entries:JSON.parse(window.host.funnel_motion_drain(key)),native:window.fmNative.splice(0)}),key);
     r.native.forEach(n=>{assert(!native.has(n.seq));native.set(n.seq,n);});
     if(r.entries.length){const bytes=Buffer.from(JSON.stringify(r.entries,null,2)+'\n'),compressed=gzipSync(bytes),json=key+'-events-'+p.eventBatches.length+'.json.gz';await writeFile(join(out,json),compressed,{flag:'wx'});r.eventBatch={json,entries:r.entries.length,sha256:sha(compressed),uncompressedSha256:sha(bytes),uncompressedBytes:bytes.length};p.eventBatches.push(r.eventBatch);}
     ledger.push(...r.entries.filter(e=>e.kind!=='effect'));
     try{entries(r.entries,native,r.snapshot.owner);}catch(error){await writeFile(join(out,key+'-failed-read.json'),JSON.stringify(r,null,2)+'\n',{flag:'wx'});throw error;}return r;
    };
    const sync=o=>{if(!o.motion){oracle=null;lastOwner=o;return;}if(!oracle){oracle=createOracle(o,config);}else if(lastOwner.motion&&o.motion.targetSerial!==lastOwner.motion.targetSerial){oracle.verifyRetarget(lastOwner,o);}oracle.verifyPaint(o);lastOwner=o;};
    const operation=async(op,a=0,b=0,{reject=false,backend=false}={})=>{
     const before=await read();sync(before.snapshot.owner);
     const proof=async()=>({dom:await page.evaluate(selector=>{const root=document.querySelector(selector),active=document.activeElement;return{html:root.outerHTML,canvas:document.querySelector('#fm-canvas-'+root.id.slice('funnel-motion-'.length))?.outerHTML,focus:active?.outerHTML};},selector),png:await page.screenshot({scale:'device',caret:'initial',animations:'allow'})});
     const previous=reject?await proof():null;
     const invocation=await page.evaluate(([k,op,a,b,backend])=>{const original=Element.prototype.setAttribute;let injected=false;if(backend)Element.prototype.setAttribute=function(name,value){if(!injected&&name==='data-funnel-svg-state'&&value==='placed'&&this.closest('#funnel-motion-'+k)){injected=true;throw new DOMException('authored failure after native leaf placement','InvalidStateError');}return original.call(this,name,value);};try{return{result:JSON.parse(window.host.funnel_motion_operation(k,op,a,b)),injected};}finally{if(backend)Element.prototype.setAttribute=original;}},[key,op,a,b,backend]);
     const r=invocation.result;if(backend)assert(invocation.injected,'backend fault reached actual post-leaf placement');
     const operationFile=key+'-operation-'+p.steps.length+'.json';
     p.steps.push({op,a,b,backend,Ok:r.Ok,error:r.error,before:before.snapshot.owner,owner:r.snapshot.owner,beforeBuilds:before.snapshot.builds,afterBuilds:r.snapshot.builds,beforeFactories:before.snapshot.factories,afterFactories:r.snapshot.factories,json:operationFile});
     await writeFile(join(out,operationFile),JSON.stringify({op,a,b,backend,injected:invocation.injected,before:before.snapshot,result:r},null,2)+'\n',{flag:'wx'});
     if(reject){const next=await proof(),stem=key+'-rejection-'+p.rejections.length,record={op,a,b,error:r.error,before:before.snapshot,after:r.snapshot,beforeDom:previous.dom,afterDom:next.dom,beforePng:stem+'-before.png',afterPng:stem+'-after.png',beforeSha256:sha(previous.png),afterSha256:sha(next.png)};p.rejections.push(record);
      await writeFile(join(out,record.beforePng),previous.png,{flag:'wx'});await writeFile(join(out,record.afterPng),next.png,{flag:'wx'});
      assert(!r.Ok,op+' must reject');const old=structuredClone(before.snapshot.owner),now=structuredClone(r.snapshot.owner);delete old.error;delete now.error;assert.deepEqual(now,old,'rejection preserves accepted data/model/policy/pose');assert.deepEqual(next.dom,previous.dom,'rejection restores whole DOM and focus');assert(previous.png.equals(next.png),'rejection preserves complete viewport pixels');}
     else{assert(r.Ok,r.error??op);const o=r.snapshot.owner;
      if(op==='step'&&o.motion){oracle.verifyStep(before.snapshot.owner,o,a,config);assert.equal(r.snapshot.builds,before.snapshot.builds,'physics does not replay business builder');assert.equal(r.snapshot.factories,before.snapshot.factories,'physics does not replay business factories');assert.equal(o.generation,before.snapshot.owner.generation);lastOwner=o;}
      else if(op==='preference'&&a===1&&o.motion){oracle.verifyFinish(before.snapshot.owner,o);lastOwner=o;}
      else{if(op==='spring')config={mass:1,stiffness:a,damping:b};if(op==='program')mode=a;sync(o);}
     }
     await settle();return r;
    };
    const capture=async(label,{allowError=false,negative=false}={})=>{
     const r=await read();await page.evaluate(k=>window.host.funnel_motion_canvas(k),key);
     const dom=await page.evaluate(selector=>{const root=document.querySelector(selector),surface=root.querySelector('.kartis-funnel-svg-layers[data-funnel-svg-active="true"]'),rect=n=>{const b=n.getBoundingClientRect();return[b.x,b.y,b.width,b.height];};
      const id=n=>{let id=window.fmNodes.get(n);if(!id){id=++window.fmNext;window.fmNodes.set(n,id);}return id;};
      return{surface:rect(surface),canvas:rect(document.querySelector('#fm-canvas-'+root.id.slice('funnel-motion-'.length))),focus:document.activeElement?.className,button:[...root.querySelectorAll('.fm-button')].map(n=>({id:id(n),text:n.textContent,box:rect(n)})),
       paths:[...surface.querySelectorAll('.kartis-funnel-svg-native-content path[data-id]')].map(n=>({id:n.getAttribute('data-id'),node:id(n),d:n.getAttribute('d'),fill:n.getAttribute('fill'),stroke:n.getAttribute('stroke'),width:n.getAttribute('stroke-width'),box:rect(n),occurrence:n.closest('.kartis-funnel-svg-occurrence').getAttribute('data-funnel-svg-key')})),
       texts:[...surface.querySelectorAll('.kartis-funnel-svg-native-content text')].map(n=>({node:id(n),text:n.textContent,x:n.getAttribute('x'),y:n.getAttribute('y'),fill:n.getAttribute('fill'),occurrence:n.closest('.kartis-funnel-svg-occurrence').getAttribute('data-funnel-svg-kind')}))};},selector);
     const bytes=await page.screenshot({scale:'device',caret:'initial',animations:'allow'}),im=png(bytes),series=negative?p.negatives:p.frames,stem=key+'-'+(negative?'negative':'frame')+'-'+series.length,row={label,mode,snapshot:r.snapshot,entries:r.entries,eventBatch:r.eventBatch??null,native:r.native,dom,png:stem+'.png',pngSha256:sha(bytes),samples:[],passed:false};
     series.push({json:stem+'.json',png:row.png,label});await writeFile(join(out,row.png),bytes,{flag:'wx'});
     try{
      check(r.snapshot,{allowError});sync(r.snapshot.owner);
      const o=r.snapshot.owner,s=o.motion?.scene??o.data.scene,byID=new Map(s.marks.map(m=>[m.id,m]));
      const partIDs=o.data.computed.parts.flatMap(p=>[p.body,p.border]).filter(Boolean).sort();
      for(const occurrence of ['parts','parts-echo']){const expected=(occurrence==='parts'?mode!==1:mode===2)?partIDs:[];assert.deepEqual(dom.paths.filter(p=>p.occurrence===occurrence).map(p=>p.id).sort(),expected,'exact native part role membership');}
      for(const [occurrence,prefix] of [['separators','funnel:separator:'],['annotations','funnel:annotation:']])assert.deepEqual(dom.paths.filter(p=>p.occurrence===occurrence).map(p=>p.id).sort(),s.marks.filter(m=>m.id.startsWith(prefix)).map(m=>m.id).sort(),'exact native decoration role membership');
      for(const path of dom.paths){const m=byID.get(path.id);assert(m,'actual path belongs to sampled target slot');pathEquals(path.d,m.geometry.commands);if(m.stroke){near(Number(path.width),m.stroke.width,'actual sampled stroke width');}}
      const labelBase=o.motion?.annotations?.baseLabels??o.data.computed.parts.filter(p=>p.label!==null).length;
      for(const kind of ['labels','annotations']){const labels=s.labels.filter((_,i)=>kind==='labels'?(o.motion?o.motion.labelParts[i]!=null:i<labelBase):i>=labelBase),actual=dom.texts.filter(t=>t.occurrence===kind);assert.equal(actual.length,labels.length,'actual text slots');actual.forEach((t,i)=>{const l=labels[i];assert.equal(t.text,l.text,'actual target text');near(Number(t.x),l.position[0],'actual animated text x');near(Number(t.y),l.position[1],'actual animated text y');assert.deepEqual(t.fill.match(/[\d.]+/g).map(Number).slice(0,3),l.color.slice(0,3),'actual animated text color');});}
      if(dom.surface[2]>0&&dom.button.length){const body=o.data.computed.parts[0].body,m=s.marks.find(m=>m.id===body),[k,tx,ty]=o.view;
       const sample=(logical,box,expected,why)=>{const client=[box[0]+logical[0]*box[2]/600,box[1]+logical[1]*box[3]/420],index=client.map(v=>Math.floor(v*dpr)),offset=4*(index[1]*im.width+index[0]),actual=[...im.data.subarray(offset,offset+4)],delta=Math.max(...actual.map((v,i)=>Math.abs(v-expected[i])));row.samples.push({why,logical,box,client,index,expected,actual,delta});assert(delta<=1,why);};
       const pose=v=>[40+tx+k*(v[0]-40),28+ty+k*(v[1]-28)];sample(pose([66,48]),dom.surface,o.motion?m.fill:[255,255,255,255],'actual animated Graphic paint');
       const selection=await page.evaluate(selectFunnelPoint,{scene:s,id:body,mode:'paint'});row.bodySelection=selection;
       if(selection){sample(selection.point,dom.canvas,m.fill,'actual sampled Scene Canvas interior');if(mode!==1)sample(pose(selection.point),dom.surface,o.hidden.includes(m.series)?s.background:m.fill,o.hidden.includes(m.series)?'matched hidden body has no SVG ink':'actual animated body SVG interior');}
      }
      row.passed=true;
     }catch(error){row.error=String(error?.stack??error);throw error;}finally{await writeFile(join(out,stem+'.json'),JSON.stringify(row,null,2)+'\n',{flag:'wx'});}return row;
    };
    try{
     await page.mouse.move(1350,850);await page.evaluate(([k,p])=>window.host.funnel_motion_mount(k,p),[key,profile]);await settle();await capture('initial settled source');
     await operation('source',1,1);await capture('new target preserves old displayed path and style');
     await operation('step',.08);const first=await capture('independent path/style/label/separator/annotation springs');
     await operation('source',0,0);await capture('interruption resets path from previous target, style keeps velocity');await operation('step',.04);await capture('interrupted independent sample');
     await operation('curve',profile<2?1:0);await capture('different topology keeps new commands and D3 ordinal numeric pairs');await operation('step',.06);await capture('different topology physical sample');
     await operation('source',2,1);await operation('step',.05);const beforeRaw=await capture('second accepted source');await operation('source',3,1);const raw=await capture('raw-only update retains progress and adopts owned source');assert.deepEqual(raw.snapshot.owner.data.computed,beforeRaw.snapshot.owner.data.computed);assert.notEqual(raw.snapshot.owner.data.pool,beforeRaw.snapshot.owner.data.pool);
     await operation('step',.07);const beforeConfig=await capture('pre-configuration physical velocity');await operation('spring',90,9);const afterConfig=await capture('configuration preserves physical pose and velocity');assert.deepEqual(afterConfig.snapshot.owner.motion.scene,beforeConfig.snapshot.owner.motion.scene);assert.deepEqual(afterConfig.snapshot.owner.motion.marks,beforeConfig.snapshot.owner.motion.marks,'configuration preserves all mark velocities');assert.deepEqual(afterConfig.snapshot.owner.motion.labels,beforeConfig.snapshot.owner.motion.labels,'configuration preserves all label velocities');await operation('step',.03);await capture('new spring configuration drives existing channels');
     await operation('budget',0,0,{reject:true});await capture('budget rejection preserves old model and paint',{allowError:true});await operation('refresh');
     await operation('step',-1,0,{reject:true});await capture('invalid interval rejection preserves old pose',{allowError:true});await operation('refresh');
     await operation('fault',1);await operation('source',99,0,{reject:true});await capture('failed source never poisons accepted raw pool',{allowError:true});await operation('fault',0);await operation('refresh');await capture('accepted-source refresh recovers');
     await operation('fault',2);await operation('program',0,1,{reject:true});await capture('retained constructor rejection preserves live model',{allowError:true});await operation('fault',0);await operation('refresh');
     await operation('preference',1);const finish=await capture('reduced motion snaps every channel to target');assert.deepEqual(finish.snapshot.owner.motion.scene,finish.snapshot.owner.data.scene);await operation('preference',2);
     await operation('source',4,2);await capture('reordered typed IDs retain keyed channels');await operation('step',.04);await capture('reordered physical sample');
     await operation('source',5,3);await capture('removed roles disappear immediately, fresh entries begin at target');await operation('step',.04);await capture('no exit ghost or stale business source');
     const button=page.locator(selector+' .fm-button').first();await button.evaluate(n=>n.focus());await page.keyboard.press('Enter');await settle();const focused=await capture('trusted keyboard preserves application local state');assert(focused.dom.button[0].text.endsWith(':1'));
     await operation('step',.02);const focusStep=await capture('physics keeps real retained button and focus');assert.equal(focusStep.dom.button[0].id,focused.dom.button[0].id);assert.equal(focusStep.dom.focus,focused.dom.focus);
     await operation('step',.01,0,{reject:true,backend:true});const rollback=await capture('post-placement native leaf failure restores old paths and focused component',{allowError:true});assert.equal(rollback.dom.button[0].id,focusStep.dom.button[0].id);assert.equal(rollback.dom.focus,focusStep.dom.focus);await operation('refresh');
     await operation('tooltip');await operation('callback',1);await operation('current',1);const controlled=await capture('controlled current avoids pointer-induced target replacement');
     const hitSelection=await page.evaluate(selectFunnelPoint,{scene:controlled.snapshot.owner.motion.scene,id:controlled.snapshot.owner.data.computed.parts[0].body,mode:'paint'});assert(hitSelection);p.hitSelection=hitSelection;
     const hit=hitSelection.point,box=controlled.dom.surface;await page.mouse.move(box[0]+hit[0]*box[2]/600,box[1]+hit[1]*box[3]/420);await settle();let hover=await capture('real native input carries target source and displayed physical frame');
     assert(hover.snapshot.owner.tooltip,'actual current tooltip selection');const bodyID=hover.snapshot.owner.data.computed.parts[0].body,oldNode=hover.dom.paths.find(p=>p.id===bodyID&&p.occurrence==='parts').node,builds=hover.snapshot.builds;
     for(const dt of[.016,.02,.03]){await operation('step',dt);hover=await capture('stationary native body survives physical publication');assert.equal(hover.dom.paths.find(p=>p.id===bodyID&&p.occurrence==='parts').node,oldNode,'animated body DOM identity');assert.equal(hover.snapshot.builds,builds,'stationary hover does not fabricate source rebuilds');}
     await page.mouse.click(box[0]+hit[0]*box[2]/600,box[1]+hit[1]*box[3]/420);await settle();await capture('step from native callback rejects before mutation');assert(ledger.some(e=>e.kind==='callbackStep'));
     await operation('source',8,0);await capture('prepare distinct physical publication for capture race');
     await page.evaluate(key=>{window.fmSwitch=null;window.addEventListener('click',e=>{if(window.fmSwitch!==key||!e.target.closest?.('#funnel-motion-'+key))return;window.fmSwitch=null;window.fmSwitchSequence=e.fmSeq;window.fmSwitchResult=JSON.parse(window.host.funnel_motion_operation(key,'step',.01,0));},true);window.fmSwitch=key;},key);
     await page.mouse.click(box[0]+hit[0]*box[2]/600,box[1]+hit[1]*box[3]/420);await settle();const switched=await read(),switchInfo=await page.evaluate(()=>({seq:window.fmSwitchSequence,result:window.fmSwitchResult}));
     assert(switchInfo.result.Ok);const admitted=native.get(switchInfo.seq);assert(admitted);oracle.verifyStep(admitted.owner,switchInfo.result.snapshot.owner,.01,config);lastOwner=switchInfo.result.snapshot.owner;
     assert(!ledger.some(e=>e.kind==='native'&&e.value.seq===switchInfo.seq),'capture-time physical A must not be delivered as adopted B');p.captureSwitch={...switchInfo,admission:admitted,delivered:false};await capture('physical publication changed between capture and bubble drops stale input');
     await page.mouse.move(box[0]+hit[0]*box[2]/600+1,box[1]+hit[1]*box[3]/420);await settle();await capture('new native observation admits the new physical publication');
     await page.mouse.move(1350,850);await settle();await capture('delivered native Leave clears presentation');
     p.physicalInput=[];
     await operation('current',1);const shrink=await operation('source',9,1),shrinkOwner=shrink.snapshot.owner;
     const sampleOnly=await page.evaluate(selectFunnelPoint,{scene:shrinkOwner.motion.scene,targetScene:shrinkOwner.motion.targetScene,id:shrinkOwner.data.computed.parts[0].body,bodyIDs:shrinkOwner.data.computed.parts.map(p=>p.body).filter(Boolean),mode:'sample-only'});assert(sampleOnly,'authored shrinking body has a sample-only interior');
     await page.mouse.click(box[0]+sampleOnly.point[0]*box[2]/600,box[1]+sampleOnly.point[1]*box[3]/420);await settle();const sampleInput=await capture('native sample-only geometry receives input');const positiveClick=[...native.values()].filter(n=>n.type==='click'&&n.root==='funnel-motion-'+key).at(-1),positiveCallback=ledger.find(e=>e.kind==='native'&&e.value.seq===positiveClick.seq&&e.value.kind==='Click');assert(positiveCallback);assert.equal(positiveCallback.value.partIndex,0);assert.equal(sampleInput.snapshot.owner.generation,shrinkOwner.generation,'selected current avoids reinterpreting sample-only input');p.physicalInput.push({kind:'sample-only',selection:sampleOnly,admission:positiveClick,callback:positiveCallback});
     await page.mouse.move(1350,850);await settle();await capture('clear sample-only pointer before growing target');await operation('current',1);const grow=await operation('source',10,0),growOwner=grow.snapshot.owner;
     const targetOnly=await page.evaluate(selectFunnelPoint,{scene:growOwner.motion.scene,targetScene:growOwner.motion.targetScene,id:growOwner.data.computed.parts[0].body,bodyIDs:growOwner.data.computed.parts.map(p=>p.body).filter(Boolean),mode:'target-only'});assert(targetOnly,'authored growing body has a target-only interior');
     await page.mouse.click(box[0]+targetOnly.point[0]*box[2]/600,box[1]+targetOnly.point[1]*box[3]/420);await settle();const targetInput=await capture('target-only geometry grants no native business input'),negativeClick=[...native.values()].filter(n=>n.type==='click'&&n.root==='funnel-motion-'+key).at(-1);assert(!ledger.some(e=>e.kind==='native'&&e.value.seq===negativeClick.seq));assert.equal(targetInput.snapshot.owner.generation,growOwner.generation);p.physicalInput.push({kind:'target-only',selection:targetOnly,admission:negativeClick,delivered:false});await page.mouse.move(1350,850);await settle();
     await operation('program',2,0);await capture('repeated literal body and retained layer occurrences');await operation('program',1,0);const omitted=await capture('omitted body has no native occurrence');assert(!omitted.dom.paths.some(p=>p.occurrence==='parts'));assert.equal(omitted.snapshot.owner.tooltip,null);await operation('program',0,0);
     await operation('view',1.1,8);await operation('css',.85,9);await capture('physical sampling under camera and CSS scale');await operation('hidden',1);await capture('hidden series keeps owned source');await operation('hidden',0);await operation('view',1,0);await operation('css',1,0);
     if(profile===0){const graphic=page.locator(selector+' .fm-graphic path').first(),old=await graphic.getAttribute('fill');await graphic.evaluate(n=>n.setAttribute('fill','rgb(240,0,0)'));let rejected=false;try{await capture('real Graphic mutation rejected by ordinary pixel gate',{negative:true});}catch(error){assert.match(String(error),/actual animated Graphic paint/);rejected=true;}assert(rejected);await graphic.evaluate((n,old)=>old===null?n.removeAttribute('fill'):n.setAttribute('fill',old),old);await capture('negative paint restored');}
     // Automatic runtime uses real RAF; exact math is separately checked at Manual intervals.
     await operation('source',6,0);const autoStart=(await read()).snapshot.owner.motion.sampleSerial;await operation('clock',1);await page.waitForFunction(({key,serial})=>JSON.parse(window.host.funnel_motion_owner(key)).motion.sampleSerial>serial,{key,serial:autoStart});await operation('clock',0);const automatic=await capture('automatic RAF advances real physical samples');assert(automatic.snapshot.owner.motion.sampleSerial>autoStart);p.automatic={beforeSerial:autoStart,afterSerial:automatic.snapshot.owner.motion.sampleSerial};
     await operation('visibility',0);await operation('source',7,1);await operation('clock',1);const hidden=await read();assert(!hidden.snapshot.owner.motion.settled,'hidden target is genuinely unsettled before restore');await page.waitForTimeout(80);const hiddenAfter=await read();assert.deepEqual(hiddenAfter.snapshot.owner.motion,hidden.snapshot.owner.motion,'zero-BCR automatic time is paused');assert(!hiddenAfter.snapshot.owner.rafPending);await operation('visibility',1);await page.waitForFunction(({key,serial})=>JSON.parse(window.host.funnel_motion_owner(key)).motion.sampleSerial>serial,{key,serial:hidden.snapshot.owner.motion.sampleSerial});await operation('clock',0);const restored=await capture('restored surface resumes without hidden-time catchup');p.visibility={hidden:hidden.snapshot.owner.motion,after:restored.snapshot.owner.motion};
     await page.emulateMedia({reducedMotion:'reduce'});await operation('preference',0);const reduced=await capture('system reduced motion');assert(reduced.snapshot.owner.motionState.reduced);assert(reduced.snapshot.owner.motion.settled);
     await operation('visibility',0);await page.emulateMedia({reducedMotion:'no-preference'});await settle();const hiddenMedia=await read();await operation('visibility',1);await settle();await operation('clock',0);const mediaRestored=await capture('settled hidden surface synchronizes restored system preference');assert(!mediaRestored.snapshot.owner.motionState.reduced);assert.equal(mediaRestored.snapshot.builds,reduced.snapshot.builds,'system and visibility policy never rebuild source');p.hiddenMedia={before:reduced.snapshot.owner.motionState,hidden:hiddenMedia.snapshot.owner.motionState,after:mediaRestored.snapshot.owner.motionState};
     await operation('disable');const disabled=await capture('disabling motion presents the target and preserves retained scopes');assert.equal(disabled.snapshot.owner.motion,null);await operation('enable');await capture('re-enabled source starts settled');
     const history=(await read()).snapshot.historical;const stopped=await operation('stop');p.stopped=stopped.snapshot;assert(!stopped.snapshot.owner.active);assert.deepEqual(stopped.snapshot.historical,history);assert.equal(stopped.snapshot.cleanups,stopped.snapshot.peers.layers.length,'every successfully mounted layer Scope retires');assert.equal(stopped.snapshot.tipCleanups,stopped.snapshot.peers.tips.length,'every successfully mounted tooltip Scope retires');
     for(const row of [...stopped.snapshot.peers.layers,...stopped.snapshot.peers.tips]){assert(row.cleaned&&!row.location.connected);assert(row.frame.Err&&row.motion.Err);}await read();p.passed=true;
    }catch(error){p.error=String(error?.stack??error);failed=true;}finally{p.ledger=ledger;p.native=[...native.values()];await page.evaluate(k=>window.host.funnel_motion_drop(k),key);p.json=key+'-profile.json';await writeFile(join(out,p.json),JSON.stringify(p,null,2)+'\n',{flag:'wx'});entry.profiles[entry.profiles.length-1]={profile,passed:p.passed,frames:p.frames.length,rejections:p.rejections.length,negatives:p.negatives.length,json:p.json,error:p.error};}
   }
   entry.pageErrors=errors;entry.passed=errors.length===0&&entry.profiles.every(p=>p.passed);
  }catch(error){entry.error=String(error?.stack??error);failed=true;}finally{entry.pageErrors=errors;entry.afterProfilesNative=await page.evaluate(()=>window.fmNative.splice(0)).catch(()=>[]);await context.close();}
 }}finally{await browser.close();}
}}finally{await new Promise(done=>server.close(done));report.sourceEnd=await fence();report.sourceUnchanged=JSON.stringify(report.sourceStart)===JSON.stringify(report.sourceEnd);report.passed=!failed&&report.sourceUnchanged&&report.contexts.every(c=>c.passed);await writeFile(join(out,'report.json'),JSON.stringify(report,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify({passed:report.passed,sourceUnchanged:report.sourceUnchanged,contexts:report.contexts.map(c=>({engine:c.engine,dpr:c.dpr,passed:c.passed,profiles:c.profiles.map(p=>({profile:p.profile,passed:p.passed,error:p.error}))}))}));}
if(!report.passed)process.exitCode=1;
