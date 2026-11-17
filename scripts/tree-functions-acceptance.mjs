// Actual Tree function configuration through SVG and Canvas consumers.
import assert from 'node:assert/strict';
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {createServer} from 'node:http';
import {createHash} from 'node:crypto';
import {execFileSync} from 'node:child_process';
import {resolve,relative,isAbsolute,join,extname} from 'node:path';
import {chromium,firefox,webkit} from 'playwright';
import {captureViewport,rgbaAtLogical} from './acceptance-viewport.mjs';
import {png} from './acceptance-png.mjs';

const root=resolve('.');
const meta=JSON.parse(execFileSync('cargo',['metadata','--no-deps','--format-version','1'],{encoding:'utf8'}));
const out=resolve(process.argv.slice(2).find(v=>!v.startsWith('--'))||join(meta.target_directory,'tree-functions-browser'));
await mkdir(out,{recursive:true});
const hash=b=>createHash('sha256').update(b).digest('hex');
const protectedFiles=[
 'scripts/tree-functions-acceptance.mjs','scripts/acceptance-viewport.mjs','scripts/acceptance-png.mjs',
 'src/radial.rs','src/tree_styles.rs','src/tree_layout.rs','src/tree_layout/factories.rs',
 'examples/shared/tree_functions.rs','examples/browser-host/src/browser/tree_functions.rs',
 'examples/browser-host/src/lib.rs','examples/browser-host/pkg/host.js','examples/browser-host/pkg/host_bg.wasm',
];
const hashes=async()=>Object.fromEntries(await Promise.all(protectedFiles.map(async p=>[p,hash(await readFile(p))])));
const initial=await hashes();
const server=createServer(async(req,res)=>{
 try{
  const p=resolve(root,'.'+new URL(req.url,'http://localhost').pathname),r=relative(root,p);
  assert(!r.startsWith('..')&&!isAbsolute(r));
  res.setHeader('Content-Type',{'.html':'text/html','.js':'application/javascript','.wasm':'application/wasm'}[extname(p)]||'application/octet-stream');
  res.end(await readFile(p));
 }catch{res.writeHead(404);res.end();}
});
await new Promise(done=>server.listen(0,'127.0.0.1',done));
const filter=flag=>process.argv.find(v=>v.startsWith(flag+'='))?.split('=')[1];
const engines=(process.argv.includes('--chromium-only')?[['Chromium',chromium,{}]]:[['Chrome',chromium,{channel:'chrome'}],['Firefox',firefox,{}],['WebKit',webkit,{}],['Edge',chromium,{channel:'msedge'}]]).filter(([n])=>!filter('--browser')||filter('--browser')===n);
const dprs=[1,2].filter(v=>!filter('--dpr')||filter('--dpr')===String(v));
assert(engines.length&&dprs.length);
const report={passed:false,strictWholeViewportAccepted:false,pixelTolerance:1,nativePrimitivePixelTolerance:1,geometryTolerance:1e-9,
 scope:'Five borrowed Tree functions; complete raw/topology/computed function inputs and order; actual source geometry, labels, colors, widths, picking and candidate rollback. Declared static inset remains independent of function size. No arbitrary React/Spring, nested datum/path UID or D3 numerical equivalence.',
 contexts:[],provenance:{start:initial}};
const close=(a,b,t=1e-9)=>Number.isFinite(a)&&Number.isFinite(b)&&Math.abs(a-b)<=t;
const vec=(a,b)=>a.length===b.length&&a.every((v,i)=>close(v,b[i]));
const bit=n=>{const b=new ArrayBuffer(8),d=new DataView(b);d.setFloat64(0,n);return d.getBigUint64(0).toString();};
const white=[255,255,255,255],orange=[230,110,20,255],purple=[100,20,180,255];
const color=(id,phase)=>[...(id==='a'?[0,100,255]:id==='b'?[0,200,0]:[160,80,30]),phase===13?0:255];
function expected(phase,width){
 let raw=[{id:'a',parent:'r',value:phase===1?24:12},{id:'r',parent:null,value:0},{id:'b',parent:'r',value:phase===1?4:-0}];
 if(phase===2)raw.reverse();if(phase===14)raw=[];if(phase===15)raw=raw.filter(d=>d.parent===null);
 raw=raw.map(d=>({...d,label:'source-'+d.id,bits:bit(d.value)}));
 const index=new Map(raw.map((d,i)=>[d.id,i])),rootIndex=raw.findIndex(d=>d.parent===null);
 const children=raw.map(d=>raw.flatMap((r,i)=>r.parent===d.id?[i]:[]));
 const order=rootIndex<0?[]:[rootIndex];for(let i=0;i<order.length;i++)order.push(...children[order[i]]);
 const nodes=raw.map((d,i)=>{
  const depth=d.parent===null?0:1,height=children[i].length?1:0;
  const value=height?children[i].reduce((sum,j)=>sum+raw[j].value,0):d.value;
  const radius=phase===4?0:12+2*depth+(phase===1?4:0);
  const breadth=d.parent===null?.5:children[rootIndex].indexOf(i)===0?.25:.75;
  const x=v=>56+v*(width-112),y=v=>46+v*228;
  let position;
  if(phase===8)position=[x(1-depth),y(breadth)];
  else if(phase===9)position=[x(breadth),y(depth)];
  else if(phase===10)position=[x(breadth),y(1-depth)];
  else if(phase===11)position=[width/2+114*depth*Math.sin(2*Math.PI*breadth),160-114*depth*Math.cos(2*Math.PI*breadth)];
  else position=[x(depth),y(breadth)];
  return{sourceIndex:i,parentIndex:d.parent===null?null:index.get(d.parent),children:children[i],depth,height,
   rawValue:d.value,rawBits:bit(d.value),value,position,radius,color:color(d.id,phase),markId:'node:'+d.id};
 });
 const links=order.filter(i=>raw[i].parent!==null).map((i,linkIndex)=>{
  const sourceIndex=index.get(raw[i].parent),p=nodes[sourceIndex].position,q=nodes[i].position;
  return{linkIndex,sourceIndex,targetIndex:i,sourcePosition:p,targetPosition:q,
   position:[p[0]/2+q[0]/2,p[1]/2+q[1]/2],markId:phase===3?null:'link:'+raw[i].id};
 });
 const labels=phase===6||phase===7?[]:order.map(i=>({text:phase===16&&raw[i].id==='b'?'':'function-'+raw[i].id,
  owner:raw[i].id,color:purple,position:[nodes[i].position[0],nodes[i].position[1]+nodes[i].radius+5+18*.8]}));
 const log=[];
 for(const i of order){const n=nodes[i],d=raw[i];log.push(`size:${d.id}:${i}:${n.depth}:${n.height}:${bit(n.value)}:${d.bits}:${raw.length}`,`color:${d.id}:${n.parentIndex!==null}:${n.children.length}`);}
 for(const l of links){const p=nodes[l.sourceIndex],q=nodes[l.targetIndex];log.push(`thickness:${l.linkIndex}:${p.radius}:${q.radius}:${bit(p.value)}:${bit(q.value)}`,`link-color:${l.linkIndex}:${raw[l.sourceIndex].id}:${raw[l.targetIndex].id}`);}
 if(phase!==6&&phase!==7)for(const i of order){const n=nodes[i];log.push(`label:${raw[i].id}:${n.radius}:${bit(n.value)}`);}
 return{raw,nodes,links,labels,log,order,plot:[40,30,width-80,260]};
}
function geometry(mark,kind,numbers,commands){
 assert(mark.geometry.startsWith(kind),'actual source geometry type');
 const fields=kind==='Circle'?['x','y','radius']:kind==='Rect'?['x','y','width','height']:null;
 const actual=fields?fields.map(k=>Number(mark.geometry.match(new RegExp('\\b'+k+': ([^,}]+)'))[1])):[...mark.geometry.matchAll(/\b[xy]: ([^,}]+)/g)].map(m=>Number(m[1]));
 assert(vec(actual,numbers),'independent actual source geometry');
 if(commands)assert.deepEqual([...mark.geometry.matchAll(/\b(Move|Line|Close|Cubic)\b/g)].map(m=>m[1]),commands);
}
function validateAccepted(d,e,phase){
 assert.deepEqual(d.raw,e.raw,'independent raw values and exact IEEE bits');assert.deepEqual(d.log,e.log,'independent five-function scheduling and complete computed input values');
 assert.deepEqual(d.plot,e.plot);assert.equal(d.nodes.length,e.nodes.length);assert.equal(d.links.length,e.links.length);
 for(let i=0;i<e.nodes.length;i++){
  const p=d.nodes[i],q=e.nodes[i];
  for(const key of Object.keys(q).filter(k=>k!=='position'))assert.deepEqual(p[key],q[key],'node '+key);
  assert(vec(p.position,q.position));
 }
 for(let i=0;i<e.links.length;i++){
  const p=d.links[i],q=e.links[i];
  for(const key of ['linkIndex','sourceIndex','targetIndex','markId'])assert.deepEqual(p[key],q[key],'link '+key);
  for(const key of ['position','sourcePosition','targetPosition'])assert(vec(p[key],q[key]),'link '+key);
 }
 assert.equal(d.labels.length,e.labels.length);
 for(let i=0;i<e.labels.length;i++){
  for(const key of ['text','owner','color'])assert.deepEqual(d.labels[i][key],e.labels[i][key]);
  assert(vec(d.labels[i].position,e.labels[i].position),'label position follows resolved function size');
 }
 assert.equal(d.marks.length,e.nodes.length+e.links.filter(l=>l.markId!==null).length);
 assert.deepEqual(d.marks.map(m=>m.id),[...e.links.filter(l=>l.markId!==null).map(l=>l.markId),...e.order.map(i=>e.nodes[i].markId)],'BFS paint role order');
 const probes=[];
 for(const n of e.nodes){
  const source=e.raw[n.sourceIndex],m=d.marks.find(m=>m.id===n.markId),[x,y]=n.position,r=n.radius;
  assert.equal(m.series,source.id);assert.equal(m.label,source.label,'drawn label callback does not rewrite raw business identity');assert.equal(m.value,n.value);
  assert.deepEqual(m.fill,n.color);assert.equal(m.stroke,null);assert.equal(m.opacity,1);
  if(phase===12)geometry(m,'Rect',[x-r,y-r,2*r,2*r]);else geometry(m,'Circle',[x,y,r]);
  // Perpendicular to the neighboring straight links and away from Below labels.
  const axis=phase===9||phase===10?[1,0]:[0,-1];
  const point=distance=>[x+axis[0]*distance,y+axis[1]*distance];
  if(r>0&&phase!==13){
   for(const dx of [-2,0,2])for(const dy of [-2,0,2])probes.push({kind:'node-interior',id:n.markId,point:[x+dx,y+dy],expected:n.color});
   probes.push({kind:'node-size-interior',id:n.markId,point:point(r-4),expected:n.color});
   probes.push({kind:'node-size-exterior',id:n.markId,point:point(r+4),expected:white});
  }else probes.push({kind:'node-no-ink',id:n.markId,point:point(9),expected:white});
 }
 const thickness=phase===5?0:phase===1?10:6;
 for(const l of e.links){
  if(l.markId!==null){const m=d.marks.find(m=>m.id===l.markId);assert.equal(m.series,'');assert.equal(m.label,'');assert.equal(m.value,0);assert.equal(m.fill,null);assert.deepEqual(m.stroke,thickness===0?null:{color:orange,width:thickness});assert.equal(m.opacity,1);geometry(m,'Path',[...l.sourcePosition,...l.targetPosition],['Move','Line']);}
  const p=l.sourcePosition,q=l.targetPosition,dx=q[0]-p[0],dy=q[1]-p[1],length=Math.hypot(dx,dy),normal=[-dy/length,dx/length];
  const at=offset=>[p[0]*.55+q[0]*.45+normal[0]*offset,p[1]*.55+q[1]*.45+normal[1]*offset];
  const visible=phase!==3&&thickness>0;
  probes.push({kind:'link-center',id:'link:'+e.raw[l.targetIndex].id,point:at(0),expected:visible?orange:white});
  if(visible)for(const sign of [-1,1])probes.push({kind:'link-width-interior',id:l.markId,point:at(sign*thickness/4),expected:orange});
  for(const sign of [-1,1])probes.push({kind:'link-width-exterior',id:'link:'+e.raw[l.targetIndex].id,point:at(sign*(thickness/2+4)),expected:white});
 }
 assert.equal(d.binding.length,e.nodes.length);
 for(let i=0;i<e.nodes.length;i++){
  const n=e.nodes[i],s=d.binding[i],raw=e.raw[i];
  assert.equal(s.id,n.markId);assert.equal(s.series,raw.id);assert.equal(s.label,raw.label);assert.equal(s.value,n.value);assert.deepEqual(s.color,n.color);assert(vec(s.position,n.position));
 }
 return probes;
}

// Use the browser's independent standard line primitive, not metadata from the
// accepted source, to compare the complete AA stroke cross-section in backing
// pixels. The reference canvas is never attached to DOM or passed to Kartis.
async function nativeLineEvidence(page,rootId,e,phase,width,dpr,zoom){
 const camera=zoom?{k:1.1,tx:40-44-(width-80)*.05,ty:30-33-13}:{k:1,tx:0,ty:0};
 const thickness=phase===3||phase===5?0:phase===1?10:6;
 const lines=e.links.map(l=>({id:'link:'+e.raw[l.targetIndex].id,source:l.sourcePosition,target:l.targetPosition,width:thickness}));
 const raster=await page.evaluate(({rootId,width,dpr,lines,camera})=>{
  const actual=document.querySelector('#'+rootId).querySelectorAll('canvas');
  if(actual.length!==1)throw new Error('expected one actual Canvas backing');
  const canvas=actual[0];
  if(canvas.width!==width*dpr||canvas.height!==320*dpr)throw new Error('independent native backing dimensions');
  const reference=document.createElement('canvas');reference.width=canvas.width;reference.height=canvas.height;
  const ctx=reference.getContext('2d');ctx.setTransform(dpr,0,0,dpr,0,0);ctx.fillStyle='#ffffff';ctx.fillRect(0,0,width,320);
  ctx.translate(camera.tx,camera.ty);ctx.scale(camera.k,camera.k);ctx.strokeStyle='#e66e14';ctx.lineCap='round';
  for(const line of lines)if(line.width>0){ctx.lineWidth=line.width;ctx.beginPath();ctx.moveTo(...line.source);ctx.lineTo(...line.target);ctx.stroke();}
  if(reference.isConnected)throw new Error('reference Canvas must remain outside the DOM');
  const r=canvas.getBoundingClientRect();
  return{actual:canvas.toDataURL('image/png'),reference:reference.toDataURL('image/png'),size:[canvas.width,canvas.height],referenceAttached:reference.isConnected,canvasRect:[r.x,r.y,r.width,r.height]};
 },{rootId,width,dpr,lines,camera});
 const actualBytes=Buffer.from(raster.actual.split(',')[1],'base64'),referenceBytes=Buffer.from(raster.reference.split(',')[1],'base64');
 const actual=png(actualBytes),reference=png(referenceBytes);assert.deepEqual([actual.width,actual.height],raster.size);assert.deepEqual([reference.width,reference.height],raster.size);
 const rois=[];
 for(const line of lines){
  const project=p=>[(p[0]*camera.k+camera.tx)*dpr,(p[1]*camera.k+camera.ty)*dpr];
  const p=project(line.source),q=project(line.target),direction=[q[0]-p[0],q[1]-p[1]],length=Math.hypot(...direction);
  const axis=Math.abs(direction[0])>=Math.abs(direction[1])?0:1,cross=1-axis,dominantCos=Math.abs(direction[axis])/length;
  const center=Math.floor(p[axis]*.55+q[axis]*.45),radius=Math.ceil(line.width*dpr*camera.k/(2*dominantCos))+4;
  const roi={id:line.id,source:line.source,target:line.target,width:line.width,projectedSource:p,projectedTarget:q,majorAxis:axis,dominantCos,scanlineCount:9,crossRadius:radius,maxChannelDifference:0,actualMass:0,referenceMass:0,scanlines:[]};
  for(let major=center-4;major<=center+4;major++){
   const middle=p[cross]+(major+.5-p[axis])*direction[cross]/direction[axis],first=Math.floor(middle)-radius,last=Math.floor(middle)+radius;
   const scan={major,first,last,actualMass:0,referenceMass:0,maxChannelDifference:0};
   for(let minor=first;minor<=last;minor++){
    const at=[];at[axis]=major;at[cross]=minor;assert(at[0]>=0&&at[0]<actual.width&&at[1]>=0&&at[1]<actual.height,'native ROI stays in actual backing');
    const offset=4*(at[1]*actual.width+at[0]),a=[...actual.data.subarray(offset,offset+4)],b=[...reference.data.subarray(offset,offset+4)];
    const delta=Math.max(...a.map((v,i)=>Math.abs(v-b[i])));scan.maxChannelDifference=Math.max(scan.maxChannelDifference,delta);roi.maxChannelDifference=Math.max(roi.maxChannelDifference,delta);
    assert(delta<=1,`native line primitive ROI ${line.id} at ${at}: ${a} vs ${b}`);
    scan.actualMass+=(255-a[2])/235;scan.referenceMass+=(255-b[2])/235;
   }
   // Each Blue byte differs by at most one; this is the implied integral bound.
   scan.massDifferenceBound=(last-first+1)/235;assert(Math.abs(scan.actualMass-scan.referenceMass)<=scan.massDifferenceBound+1e-12,'native coverage mass follows the per-pixel byte bound');
   roi.actualMass+=scan.actualMass/9;roi.referenceMass+=scan.referenceMass/9;roi.scanlines.push(scan);
  }
  roi.massDifferenceBound=(2*radius+1)/235;assert(Math.abs(roi.actualMass-roi.referenceMass)<=roi.massDifferenceBound+1e-12);rois.push(roi);
 }
 return{actualBytes,referenceBytes,actualDataUrl:raster.actual,record:{size:raster.size,referenceAttached:raster.referenceAttached,canvasRect:raster.canvasRect,camera,rois}};
}

try{
 for(const[name,engine,opts]of engines){
  const browser=await engine.launch({headless:true,...opts});
  try{for(const dpr of dprs){
   const entry={name,dpr,passed:false,frames:[],rejections:[],inputs:[],viewportDiagnostics:[]};report.contexts.push(entry);
   const context=await browser.newContext({viewport:{width:1600,height:1200},deviceScaleFactor:dpr}),page=await context.newPage(),errors=[];
   page.on('pageerror',e=>errors.push(e.message));
   try{
    await page.goto(`http://127.0.0.1:${server.address().port}/examples/browser-host/index.html`);await page.waitForFunction(()=>window.hostReady===true);
    await page.evaluate(()=>{document.querySelector('.panels').style.display='none';window.treeFnInputs=[];window.addEventListener('click',e=>window.treeFnInputs.push({trusted:e.isTrusted,x:e.clientX,y:e.clientY}),true);});
    const api=(fn,r,...args)=>page.evaluate(([fn,...args])=>window.hostFixture['tree_functions_'+fn](...args),[fn,r,...args]);
    const state=async r=>JSON.parse(await api('snapshot',r)),rootNode=r=>page.locator('#tree-functions-'+r),surface=r=>rootNode(r).locator('.kartis-surface');
    const flush=async()=>{for(const r of ['svg','canvas'])await api('flush',r);await page.evaluate(()=>new Promise(done=>requestAnimationFrame(()=>requestAnimationFrame(done))));};
    let phase=0,width=720,zoom=false;
    const frame=async label=>{
     await page.mouse.move(1550,20);await flush();const e=expected(phase,width);
     for(const r of ['svg','canvas']){
      const s=await state(r),d=s.accepted;assert.equal(s.state.width,width);assert.equal(s.state.height,320);assert(vec(s.state.view,zoom?[1.1,-.1*(width-80)/2,-13]:[1,0,0]));
      const probes=validateAccepted(d,e,phase),domLabels=r==='svg'?await rootNode(r).locator('svg text').allTextContents():null;
      if(domLabels!==null)assert.deepEqual(domLabels,e.labels.map(l=>l.text),'actual SVG text uses function label');
      const controls=await rootNode(r).locator('.kartis-datum').allTextContents();
      assert.equal(controls.length,e.nodes.length,'canonical raw source controls remain distinct from drawn callback label');
      for(const raw of e.raw)assert(controls.some(t=>t.includes(raw.label)),'actual canonical source label');
      assert(controls.every(t=>!t.includes('function-')));
      await surface(r).scrollIntoViewIfNeeded();
      const qualification=await surface(r).evaluate(n=>({renderer:n.dataset.renderer,svg:n.querySelectorAll('svg').length,canvas:[...n.querySelectorAll('canvas')].map(c=>[c.width,c.height])}));
      assert.equal(qualification.renderer,r);assert(r==='svg'?qualification.svg>0:qualification.canvas.some(([w,h])=>w>0&&h>0));
      const native=r==='canvas'?await nativeLineEvidence(page,'tree-functions-'+r,e,phase,width,dpr,zoom):null;
      const shot=await captureViewport(page,surface(r)),file=`${name}-${dpr}-${r}-${entry.frames.length}.png`;await writeFile(join(out,file),shot.bytes);
      const record={label,renderer:r,phase,width,snapshot:s,domLabels,controls,qualification,geometry:shot.geometry,file,pngSha256:hash(shot.bytes),samples:[],viewportWidthDiagnostics:[]};entry.frames.push(record);
      if(native){
       const after=await rootNode(r).evaluate(n=>n.querySelector('canvas').toDataURL('image/png'));assert.equal(after,native.actualDataUrl,'actual native Canvas is stable across viewport screenshot');
       const actualFile=file.replace('.png','-native-actual.png'),referenceFile=file.replace('.png','-native-reference.png');
       await writeFile(join(out,actualFile),native.actualBytes);await writeFile(join(out,referenceFile),native.referenceBytes);
       record.nativePrimitive={...native.record,actual:{file:actualFile,sha256:hash(native.actualBytes)},reference:{file:referenceFile,sha256:hash(native.referenceBytes)},stableAcrossViewportCapture:true};
      }
      const[k,tx,ty]=[s.state.view[0],40+s.state.view[1]-40*s.state.view[0],30+s.state.view[2]-30*s.state.view[0]];
      for(const p of probes){
       const point=[p.point[0]*k+tx,p.point[1]*k+ty],sample=rgbaAtLogical(shot,{point,size:[width,320]}),delta=sample.rgba.map((v,i)=>Math.abs(v-p.expected[i]));
       const gated=!(r==='canvas'&&p.kind==='link-width-interior');record.samples.push({...p,point,...sample,delta,gated});
       if(gated)assert(delta.every(v=>v<=1),`${label}/${r}/${p.kind}/${p.id}: ${sample.rgba} vs ${p.expected}`);
       else record.viewportWidthDiagnostics.push({...p,point,...sample,delta,gated:false,solidSourceColorWithinOneByte:delta.every(v=>v<=1),reason:'CSS composition can resample native AA. Actual width is gated by full native cross-section against an independent standard line primitive; this solid-color viewport assumption remains unaccepted.'});
      }
     }
    };
    for(const r of ['svg','canvas'])await api('mount',r);await frame('initial');
    for(const r of ['svg','canvas']){
     await surface(r).scrollIntoViewIfNeeded();const e=expected(0,width),n=e.nodes.find(n=>e.raw[n.sourceIndex].id==='a'),b=await surface(r).boundingBox();
     const requested=[b.x+n.position[0]*b.width/width,b.y+n.position[1]*b.height/320].map(Math.round),inputBefore=await page.evaluate(()=>window.treeFnInputs.length);
     await page.mouse.click(...requested);await flush();const s=await state(r),event=await page.evaluate(()=>window.treeFnInputs.at(-1));
     assert(event.trusted);assert.equal(await page.evaluate(()=>window.treeFnInputs.length),inputBefore+1);assert(Math.abs(event.x-requested[0])<=1&&Math.abs(event.y-requested[1])<=1,'actual delivered native click coordinates');
     assert.equal(s.state.selected,n.markId);assert(s.state.interaction.some(row=>row.id===n.markId&&row.value===12));assert.deepEqual(s.accepted.log,e.log,'pointer does not retain or replay borrowed functions');
     assert.equal(s.accepted.binding.find(s=>s.id===n.markId).label,'source-a');entry.inputs.push({renderer:r,requested,event,id:n.markId,inputBefore,inputAfter:inputBefore+1});
    }
    await frame('selected');
    const captures=new WeakMap();
    const witness=async r=>{
     await surface(r).scrollIntoViewIfNeeded();assert(await page.evaluate(()=>!document.elementFromPoint(1550,20)?.closest('.kartis-surface')));
     const shot=await captureViewport(page,surface(r));
     const value={snapshot:await state(r),dom:await rootNode(r).evaluate(n=>n.outerHTML),paint:await rootNode(r).evaluate(n=>({canvas:[...n.querySelectorAll('canvas')].map(c=>c.toDataURL('image/png')),svg:[...n.querySelectorAll('svg,svg *')].map(n=>{const s=getComputedStyle(n);return[n.tagName,s.fill,s.stroke,s.strokeWidth,s.opacity,s.transform];})})),geometry:shot.geometry};
     captures.set(value,shot);return value;
    };
    for(const r of ['svg','canvas']){const first=await witness(r),second=await witness(r);entry.noActionWitness??=[];entry.noActionWitness.push({renderer:r,first,second});assert.deepEqual(first,second,'no-action witness is itself stable');}
    const reasons={1:'Tree size rejected',2:'Tree color rejected',3:'Tree thickness rejected',4:'Tree link color rejected',5:'Tree label rejected',6:'node size',7:'link thickness',8:'label',101:'scene',102:'interaction'};
    for(const failure of [1,2,3,4,5,6,7,8,101,102])for(const r of ['svg','canvas']){
     const before=await witness(r);
     const result=await page.evaluate(([r,f])=>{try{window.hostFixture.tree_functions_set(r,1,720,f);return{accepted:true};}catch(e){if(e instanceof TypeError||e instanceof WebAssembly.RuntimeError)throw e;return{accepted:false,error:String(e),errorClass:e?.constructor?.name??typeof e};}},[r,failure]);
     assert.equal(result.accepted,false);assert(result.error.toLowerCase().includes(reasons[failure].toLowerCase()),JSON.stringify(result));
     const after=await witness(r);entry.rollbackWitness??=[];entry.rollbackWitness.push({renderer:r,failure,before,after});assert.deepEqual(after,before,'complete accepted source/callback-log/DOM/native paint rollback');
     const previous=captures.get(before),next=captures.get(after),diag={renderer:r,failure,byteEqual:hash(previous.bytes)===hash(next.bytes),hashBefore:hash(previous.bytes),hashAfter:hash(next.bytes),gated:false};
     if(!diag.byteEqual){diag.beforeFile=`${name}-${dpr}-${r}-${failure}-before.png`;diag.afterFile=`${name}-${dpr}-${r}-${failure}-after.png`;await writeFile(join(out,diag.beforeFile),previous.bytes);await writeFile(join(out,diag.afterFile),next.bytes);}
     entry.viewportDiagnostics.push(diag);entry.rejections.push({renderer:r,failure,result});
    }
    for(const p of [1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,0]){phase=p;for(const r of ['svg','canvas'])await api('set',r,p,width,0);await frame('phase-'+p);}
    for(const r of ['svg','canvas'])await rootNode(r).evaluate(n=>n.style.transform='scale(1.25)');await frame('css-scale');
    width=760;for(const r of ['svg','canvas']){await api('width',r,width);await api('set',r,phase,width,0);}await frame('resize');
    zoom=true;for(const r of ['svg','canvas'])await api('zoom',r,true);await frame('camera');
    zoom=false;for(const r of ['svg','canvas'])await api('zoom',r,false);await frame('reset');
    for(const r of ['svg','canvas'])await api('stop',r);assert.equal(await page.locator('[id^="tree-functions-"]').count(),0);assert.deepEqual(errors,[]);entry.passed=true;
   }catch(e){entry.error=e.stack;await writeFile(join(out,`${name}-${dpr}-failure.html`),await page.content());throw e;}finally{await context.close();}
  }}finally{await browser.close();}
 }
 report.provenance.end=await hashes();assert.deepEqual(report.provenance.end,initial);report.passed=true;
}catch(e){report.error=e.stack;throw e;}finally{await writeFile(join(out,'report.json'),JSON.stringify(report,null,2));await new Promise(done=>server.close(done));}
console.log(JSON.stringify({passed:report.passed,contexts:report.contexts.length,frames:report.contexts.reduce((n,c)=>n+c.frames.length,0),report:join(out,'report.json')}));
