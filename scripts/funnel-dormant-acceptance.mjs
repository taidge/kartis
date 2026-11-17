// Dormant Funnel springs: independent scalar/path algebra and real SVG paint.
import assert from 'node:assert/strict';
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {resolve,join,relative} from 'node:path';
import {createHash} from 'node:crypto';
import {createServer} from 'node:http';
import {chromium,firefox,webkit} from 'playwright';
import {spring,pathInterpolate} from './funnel-motion-reference.mjs';
import {computeShapeGenerators} from '../tests/fixtures/funnel-motion/shape-generators.mjs';
import {createDormantOracle} from './funnel-dormant-oracle.mjs';

const flags=Object.fromEntries(process.argv.slice(2).map(v=>{const i=v.indexOf('=');assert(i>2);return[v.slice(2,i),v.slice(i+1)];}));
assert(flags.pkg&&flags.out,'--pkg=<source-bound package> --out=<new evidence directory>');
const root=resolve('.'),pkg=resolve(flags.pkg),out=resolve(flags.out),sha=b=>createHash('sha256').update(b).digest('hex');
await mkdir(out,{recursive:false});
const producerPath=resolve(pkg,'../report.json'),producerBytes=await readFile(producerPath),producer=JSON.parse(producerBytes);
assert(producer.passed&&producer.sourceUnchanged);
for(const[p,h]of Object.entries(producer.sourceEnd))assert.equal(sha(await readFile(resolve(p))),h,'compiled source '+p);
for(const[p,h]of Object.entries(producer.package))assert.equal(sha(await readFile(join(pkg,p))),h,'package '+p);
const helpers=['scripts/funnel-dormant-acceptance.mjs','scripts/funnel-dormant-oracle.mjs','scripts/funnel-motion-reference.mjs','tests/fixtures/funnel-motion/shape-generators.mjs'];
for(const name of helpers)await writeFile(join(out,'executed-'+name.split('/').at(-1)),await readFile(name),{flag:'wx'});
const inputs=[...new Set([...Object.keys(producer.sourceEnd),...helpers,'package.json','package-lock.json'])];
const fence=async()=>Object.fromEntries(await Promise.all(inputs.map(async p=>[p,sha(await readFile(p))])));
const report={passed:false,featureAccepted:false,broadGoalComplete:false,scope:'Source-key dormant state and D3 path algebra; real native SVG path membership/attributes; absent-body negative input. PNGs are saved diagnostics. Pixel/cohort acceptance belongs to the same-package complete motion regression. Chromium GPU explicitly disabled.',producer:{path:relative(root,producerPath).replaceAll('\\','/'),sha256:sha(producerBytes),package:producer.package},sourceStart:await fence(),contexts:[]};
const html='<!doctype html><html><meta charset="utf-8"><style>body{margin:0;background:white}</style><div style="position:absolute;left:2000px"><div id="svg-root" style="width:520px;height:340px"></div><div id="canvas-root" style="width:520px;height:340px"></div></div><script type="module">import init,*as host from "/pkg/host.js";await init();window.host=host;window.ready=true;</script></html>';
const server=createServer(async(req,res)=>{try{const name=decodeURIComponent(new URL(req.url,'http://localhost').pathname);if(name==='/'){res.setHeader('Content-Type','text/html');res.end(html);return;}assert(name.startsWith('/pkg/')&&!name.includes('..'));const bytes=await readFile(join(pkg,name.slice(5)));res.setHeader('Content-Type',name.endsWith('.wasm')?'application/wasm':'text/javascript');res.end(bytes);}catch{res.statusCode=404;res.end();}});
await new Promise(r=>server.listen(0,'127.0.0.1',r));
const url=`http://127.0.0.1:${server.address().port}`,close=(a,b,label)=>assert(Number.isFinite(a)&&Number.isFinite(b)&&Math.abs(a-b)<=1e-7,`${label}: ${a} != ${b}`);
const equalState=(a,b,label)=>{assert(a&&b);a.forEach((v,i)=>close(v,b[i],label+' '+i));};
const commands=s=>{const t=s.match(/[MLCZ]|[-+]?(?:\d*\.\d+|\d+\.?\d*)(?:[eE][-+]?\d+)?/g)??[],out=[];for(let i=0;i<t.length;){const op=t[i++],n={M:2,L:2,C:6,Z:0}[op];assert(n!==undefined);out.push([op,...t.slice(i,i+n).map(Number)]);i+=n;}return out;};
const pathText=g=>g.commands.map(c=>c[0]+c.slice(1).join(',')).join('');
const pathEqual=(actual,expected,label)=>{assert.equal(actual.length,expected.length,label);actual.forEach((c,i)=>{assert.equal(c[0],expected[i][0]);assert.equal(c.length,expected[i].length);c.slice(1).forEach((v,j)=>close(v,expected[i][j+1],label));});};
function targetPaths(part,profile){
 const vertical=profile%2===0,[body,border]=computeShapeGenerators(profile<2?'linear':'smooth',vertical?'vertical':'horizontal');body.digits(15);border.digits(15);
 const points=part.profile.map(p=>vertical?{x0:p.before[0],x1:p.after[0],y:p.before[1]}:{y0:p.before[1],y1:p.after[1],x:p.before[0]});
 const borders=[...part.profile.map(p=>({x:p.before[0],y:p.before[1]})),null,...part.profile.toReversed().map(p=>({x:p.after[0],y:p.after[1]}))];
 return {Body:body(points),Border:border(borders)};
}
function inspect(owner,dom){
 assert(owner.active&&owner.motion&&owner.motionState.clock==='Manual'&&!owner.motionState.rafPending);
 const m=owner.motion;assert.equal(m.generation,owner.generation);assert.equal(m.publication,owner.publication);assert.deepEqual(m.targetScene,owner.data.scene);assert.deepEqual(m.target,owner.data.computed);
 assert.equal(m.dormant.length,m.target.parts.length);
 m.dormant.forEach((part,i)=>{
  const style=m.target.styles[i];
  for(const [role,keys,targets] of [['Body',['FillRed','FillGreen','FillBlue','FillAlpha'],style.color],['Border',['StrokeRed','StrokeGreen','StrokeBlue','StrokeAlpha','StrokeWidth'],[...style.borderColor,style.borderWidth]]]){
   const row=part[role];assert.deepEqual(Object.keys(row.channels).sort(),keys.toSorted(),'complete source-owned channel set');
   keys.forEach((key,j)=>{assert(row.channels[key].length===3&&row.channels[key].every(Number.isFinite));close(row.channels[key][2],targets[j],'independently captured style target');});
   assert(row.progress?.length===3&&row.progress.every(Number.isFinite));close(row.progress[2],1,'path spring target');
  }
 });
 const actual=dom.paths,expected=m.scene.marks.filter(x=>x.id.startsWith('funnel:stage:')||x.id.startsWith('funnel:border:'));
 assert.deepEqual(actual.map(x=>x.id).sort(),expected.map(x=>x.id).sort(),'exact native role membership');
 for(const p of actual){const mark=expected.find(m=>m.id===p.id);pathEqual(commands(p.d),mark.geometry.commands,'actual native sampled path');if(mark.stroke)close(Number(p.width),mark.stroke.width,'native width');}
 assert.equal(m.scene.marks.length,m.targetScene.marks.length,'no ghost marks');
 return m;
}
function verifyStep(before,after,dt){
 assert.equal(before.generation,after.generation);assert.equal(before.motion.targetSerial,after.motion.targetSerial);
 for(let i=0;i<before.motion.dormant.length;i++)for(const role of ['Body','Border']){
  const b=before.motion.dormant[i][role],a=after.motion.dormant[i][role];
  for(const[k,s]of Object.entries(b.channels)){const x=spring({value:s[0],velocity:s[1]},s[2],dt);let e=[x.value,x.velocity,s[2]];if(Math.abs(e[0]-e[2])<=.0001&&Math.abs(e[1])<=.0001)e=[e[2],0,e[2]];equalState(a.channels[k],e,'independent '+role+' '+k);}
  if(b.progress){const s=b.progress,x=spring({value:s[0],velocity:s[1]},s[2],dt);let e=[x.value,x.velocity,s[2]];if(Math.abs(e[0]-e[2])<=.0001&&Math.abs(e[1])<=.0001)e=[e[2],0,e[2]];equalState(a.progress,e,'independent '+role+' path progress');}
 }
}
const engines=flags.browser?[flags.browser]:['Chrome','Edge','Chromium','Firefox','WebKit'];
try{
 for(const engine of engines){const type=engine==='Firefox'?firefox:engine==='WebKit'?webkit:chromium,browser=await type.launch({headless:true,...(['Chrome','Edge','Chromium'].includes(engine)?{args:['--disable-gpu']}:{}),...(engine==='Chrome'?{channel:'chrome'}:engine==='Edge'?{channel:'msedge'}:{})});
 try{for(const dpr of flags.dpr?[Number(flags.dpr)]:[1,2]){const context=await browser.newContext({viewport:{width:1300,height:900},deviceScaleFactor:dpr}),page=await context.newPage(),entry={engine,dpr,profiles:[],passed:false};report.contexts.push(entry);
 try{await page.goto(url);await page.waitForFunction(()=>window.ready);for(const profile of flags.profile?[Number(flags.profile)]:[0,1,2,3]){
  const key=`dormant-${engine}-${dpr}-${profile}`,p={profile,frames:[],operations:[],passed:false},oracle=createDormantOracle(profile);entry.profiles.push(p);
  await page.evaluate(([k,p])=>window.host.funnel_motion_mount(k,p),[key,profile]);
  await page.evaluate(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))));
  const read=async()=>{const r=await page.evaluate(k=>{const snapshot=JSON.parse(window.host.funnel_motion_read(k));return{owner:snapshot.owner,builds:snapshot.builds,factories:snapshot.factories,events:JSON.parse(window.host.funnel_motion_drain(k)),paths:[...document.querySelectorAll(`#funnel-motion-${k} .kartis-funnel-svg-native-content path[data-id]`)].filter(n=>n.getAttribute('data-id').startsWith('funnel:stage:')||n.getAttribute('data-id').startsWith('funnel:border:')).map(n=>({id:n.getAttribute('data-id'),d:n.getAttribute('d'),width:n.getAttribute('stroke-width')}))};},key);return r;};
  const capture=async label=>{const r=await read(),stem=key+'-'+p.frames.length,bytes=await page.screenshot({scale:'device'});await writeFile(join(out,stem+'.png'),bytes,{flag:'wx'});await writeFile(join(out,stem+'.json'),JSON.stringify({...r,label,png:stem+'.png',pngSha256:sha(bytes)},null,2)+'\n',{flag:'wx'});p.frames.push({label,json:stem+'.json',png:stem+'.png',pngSha256:sha(bytes)});inspect(r.owner,r);oracle.verify(r.owner);return r.owner;};
  const op=async(name,a=0,b=0)=>{const before=await read(),result=await page.evaluate(([k,n,a,b])=>JSON.parse(window.host.funnel_motion_operation(k,n,a,b)),[key,name,a,b]);await page.evaluate(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))));const after=await read();
   const json=key+'-operation-'+p.operations.length+'.json';await writeFile(join(out,json),JSON.stringify({name,a,b,before,result,after},null,2)+'\n',{flag:'wx'});p.operations.push({name,a,b,json});assert(result.Ok,result.error);inspect(after.owner,after);oracle.verify(after.owner,{name,a,b});if(name==='step'){verifyStep(before.owner,after.owner,a);assert.equal(after.builds,before.builds,'physics does not replay source builder');assert.equal(after.factories,before.factories,'physics does not replay application factories');}return after.owner;};
  await capture('mounted');await op('dormant-border',0,1);await op('step',.05);let o=await capture('border-absent-ticking');assert(o.motion.target.parts.every(x=>x.border===null));assert(!o.motion.settled);
  const invisible=o;await op('dormant-border',6,1);o=await capture('border-restored-before-tick');for(let i=0;i<o.motion.dormant.length;i++){const s=invisible.motion.dormant[i].Border.channels.StrokeWidth;equalState(o.motion.dormant[i].Border.channels.StrokeWidth,[s[0],s[1],6],'restore preserves velocity');}
  await op('step',.05);await capture('border-restored-inertia');await op('preference',1);await op('dormant-border',0,1);await op('preference',2);await op('dormant-border',6,1);o=await op('step',.05);close(o.motion.dormant[0].Border.channels.StrokeWidth[0],.8365095561160942,'zero-to-six exact analytic sample');await capture('zero-to-six');
  await op('dormant-border',6,0);await op('step',.05);o=await capture('opacity-zero-still-ticking');assert(o.motion.target.parts.every(x=>x.border===null));
  await op('source',1,1);await op('step',.04);o=await capture('invisible-border-new-path');for(const x of o.motion.dormant)assert(x.Border.progress,'hidden border keeps path track');assert(!o.motion.settled);
  await op('dormant-border',6,1);await capture('opacity-restored');await op('source',2,4);await op('step',.04);o=await capture('all-zero-body-absent');assert(o.motion.target.parts.every(x=>x.body===null));assert(o.motion.dormant.every(x=>x.Body.progress));
  const collapsed=o;
  await page.mouse.move(1200,880);await read();
  const center=o.motion.target.parts[0].center,box=await page.locator(`#funnel-motion-${key}`).boundingBox();
  await page.mouse.click(box.x+center[0],box.y+center[1]);
  const noBodyInput=await read();p.absentBodyInput={point:center,box,before:collapsed,after:noBodyInput};await writeFile(join(out,key+'-absent-input.json'),JSON.stringify(p.absentBodyInput,null,2)+'\n',{flag:'wx'});assert(!noBodyInput.events.some(e=>e.kind==='native'),'absent body never grants business input, even over decorative border');
  await op('source',3,1);o=await capture('body-restored-from-collapsed-target');
  for(let i=0;i<o.motion.target.parts.length;i++){const m=o.motion.targetScene.marks.find(x=>x.id===o.motion.target.parts[i].body),previous=targetPaths(collapsed.motion.target.parts[i],profile).Body;pathEqual(o.motion.scene.marks.find(x=>x.id===m.id).geometry.commands,commands(pathInterpolate(previous,pathText(m.geometry),0)),'restored body begins from last collapsed TARGET');equalState(o.motion.dormant[i].Body.progress,[0,0,1],'restored progress reset');}
  await op('step',.05);await capture('restored-body-progress');await op('source',4,7);o=await capture('source-key-removed');assert(!o.motion.target.parts.some(x=>x.datum.id.identity==='s1'));
  await op('source',5,1);o=await capture('source-key-readded-fresh');const freshIndex=o.motion.target.parts.findIndex(p=>p.datum.id.identity==='s1');assert(freshIndex>=0);const fresh=o.motion.dormant[freshIndex];for(const role of ['Body','Border']){equalState(fresh[role].progress,[1,0,1],'new key fresh path');for(const s of Object.values(fresh[role].channels))equalState(s,[s[2],0,s[2]],'new key fresh style');}
  await op('source',6,4);await op('preference',1);o=await capture('reduced-finishes-dormant');assert(o.motion.settled);for(const x of o.motion.dormant)for(const role of ['Body','Border'])for(const s of Object.values(x[role].channels))equalState(s,[s[2],0,s[2]],'reduced dormant exact target');
  await page.evaluate(k=>window.host.funnel_motion_drop(k),key);p.passed=true;const json=key+'-profile.json';await writeFile(join(out,json),JSON.stringify(p,null,2)+'\n',{flag:'wx'});p.json=json;
 }}finally{await context.close();}entry.passed=true;}}
 finally{await browser.close();}}
 report.sourceEnd=await fence();report.sourceUnchanged=JSON.stringify(report.sourceStart)===JSON.stringify(report.sourceEnd);assert(report.sourceUnchanged);report.passed=true;
}catch(error){report.error=error.stack??String(error);throw error;}
finally{await new Promise(r=>server.close(r));await writeFile(join(out,'report.json'),JSON.stringify(report,null,2)+'\n',{flag:'wx'});}
console.log(JSON.stringify({passed:true,contexts:report.contexts.length,profiles:report.contexts.flatMap(c=>c.profiles).length}));
