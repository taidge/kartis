// Fixed original Nivo native-circle paint reference. No product pixels or paths enter this model.
import assert from 'node:assert/strict';
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {resolve,dirname,join,relative} from 'node:path';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {execFile} from 'node:child_process';
import {chromium,firefox,webkit} from 'playwright';
import {png} from './acceptance-png.mjs';

const self=fileURLToPath(import.meta.url),root=resolve(dirname(self),'..'),original=resolve(root,'tests/fixtures/funnel-annotations/original');
const sha=b=>createHash('sha256').update(b).digest('hex'),require=createRequire(import.meta.url);
const compilerFiles=['package.json','lib/transform/babelBundle.js','lib/transform/babelBundleImpl.js'].map(p=>resolve(root,'node_modules/playwright',p));
export const referenceFiles=[self,...['DotAnnotationOutline.tsx','props.ts','NIVO-LICENSE.md','source-index.json'].map(p=>join(original,p)),...compilerFiles];
const engines={Chrome:[chromium,{channel:'chrome',args:['--disable-gpu']}],Edge:[chromium,{channel:'msedge',args:['--disable-gpu']}],Chromium:[chromium,{args:['--disable-gpu']}],Firefox:[firefox,{}],WebKit:[webkit,{}]};
const cache=new Map(),workers=new Set();let initialProof;
async function provenance(){const source=JSON.parse(await readFile(join(original,'source-index.json'),'utf8'));for(const[name,f]of Object.entries(source.files)){const bytes=await readFile(join(original,name));assert.equal(sha(bytes),f.sha256);assert.equal(bytes.length,f.bytes);}return{source,files:Object.fromEntries(await Promise.all(referenceFiles.map(async p=>[relative(root,p).replaceAll('\\','/'),sha(await readFile(p))]))),node:{version:process.version,execPath:process.execPath,sha256:sha(await readFile(process.execPath))}};}
export async function originalDotStack(center){
 const {babelTransform}=require(compilerFiles[1]);
 const sources=await Promise.all(['props.ts','DotAnnotationOutline.tsx'].map(p=>readFile(join(original,p),'utf8'))),compiled=sources.map((s,i)=>babelTransform(s,['props.ts','DotAnnotationOutline.tsx'][i],false,[],[]).code);
 const props={};new Function('exports','require',compiled[0])(props,()=>{throw Error('unexpected props import');});
 const theme={annotations:{outline:{fill:'none',stroke:'rgb(20,60,180)',strokeWidth:2,outlineWidth:1,outlineColor:'rgb(255,255,255)',outlineOpacity:1,opacity:1},symbol:{fill:'rgb(240,30,80)',stroke:'none',strokeWidth:0,opacity:1}}};
 const jsx=(type,props)=>({type,props}),module={};
 const imports=name=>{if(name==='@react-spring/web')return{animated:{circle:'circle'},useSpring:p=>({x:p.x,y:p.y,radius:p.radius})};if(name==='@nivo/core')return{useMotionConfig:()=>({animate:false,config:{}})};if(name==='@nivo/theming')return{useTheme:()=>theme};if(name==='./props')return props;if(name.replaceAll('\\','/').endsWith('/playwright/jsx-runtime'))return{jsx,jsxs:jsx,Fragment:'fragment'};throw Error('unexpected original import '+name);};
 new Function('exports','require',compiled[1])(module,imports);
 const flat=n=>n===false||n==null?[]:n.type==='fragment'?n.props.children.flatMap(flat):[n];
 const circles=[12,6].flatMap(size=>flat(module.DotAnnotationOutline({x:center[0],y:center[1],size})));
 assert.deepEqual(circles.map(c=>[c.type,c.props.cx,c.props.cy,c.props.r]),[6,6,3,3].map(r=>['circle',...center,r]));
 return{circles,theme,compiled:compiled.map((code,i)=>({file:['props.ts','DotAnnotationOutline.tsx'][i],sha256:sha(code),code})),adapter:'cold numeric useSpring; real original TSX JSX descriptors; no time integration'};
}
function markup(inputs,stack){
 const {center,view:[k,tx,ty],cssScale:[sx,sy],copies,underBodyColor}=inputs;
 const style=s=>Object.entries(s).map(([key,value])=>key.replace(/[A-Z]/g,m=>'-'+m.toLowerCase())+':'+value).join(';');
 const circles=stack.circles.map((c,i)=>`<circle data-brush="${i}" cx="${c.props.cx}" cy="${c.props.cy}" r="${c.props.r}" style="${style(c.props.style)}"/>`).join('');
 const camera=`matrix(${k} 0 0 ${k} ${40+tx-40*k} ${28+ty-28*k})`;
 const copy=()=>`<foreignObject x="0" y="0" width="600" height="420" style="overflow:hidden;pointer-events:none"><div xmlns="http://www.w3.org/1999/xhtml" style="position:relative;width:100%;height:100%;overflow:clip;pointer-events:none;isolation:isolate"><svg xmlns="http://www.w3.org/2000/svg" width="100%" height="100%" preserveAspectRatio="none" overflow="hidden" viewBox="0 0 600 420"><g transform="${camera}">${circles}</g></svg></div></foreignObject>`;
 assert(center.every(Number.isFinite));
 return`<!doctype html><html lang="en"><meta charset="utf-8"><style>body{margin:0;background:white}</style><section id="surface" style="position:relative;width:600px;height:420px;margin:24px 0 0 80px;overflow:clip;isolation:isolate;transform:scale(${sx},${sy});transform-origin:0 0"><svg xmlns="http://www.w3.org/2000/svg" width="600" height="420" viewBox="0 0 600 420" style="position:absolute;left:0;top:0;width:100%;height:100%;overflow:hidden"><rect width="600" height="420" fill="rgb(${underBodyColor.slice(0,3).join(',')})"/>${Array.from({length:copies},copy).join('')}</svg></section></html>`;
}
async function worker(request,out){
 const {inputs,key,provenance:expected}=request,current=await provenance();assert.deepEqual(current,expected);assert(engines[inputs.browser]);assert([1,2].includes(inputs.dpr));assert([1,2].includes(inputs.copies));
 const stack=await originalDotStack(inputs.center),html=markup(inputs,stack),[engine,launch]=engines[inputs.browser],receipt={key,inputs,source:current.source,provenance:current,launchOptions:{headless:true,...launch},stack,html:'reference.html',png:'reference.png',passed:false};
 await writeFile(join(out,'reference.html'),html,{flag:'wx'});let browser,context;
 try{browser=await engine.launch(receipt.launchOptions);context=await browser.newContext({viewport:{width:1200,height:850},deviceScaleFactor:inputs.dpr});const page=await context.newPage(),errors=[];page.on('pageerror',e=>errors.push(String(e)));await page.setContent(html);await page.evaluate(()=>new Promise(done=>requestAnimationFrame(()=>requestAnimationFrame(done))));
  receipt.dom=await page.evaluate(()=>{const surface=document.querySelector('#surface'),rect=n=>{const b=n.getBoundingClientRect();return[b.x,b.y,b.width,b.height];};return{surface:rect(surface),html:surface.outerHTML,productOwner:!!window.host,dpr:devicePixelRatio,circles:[...surface.querySelectorAll('circle')].map(n=>{const b=n.getBBox(),m=n.getCTM(),s=getComputedStyle(n);return{tag:n.localName,connected:n.isConnected,brush:Number(n.dataset.brush),cx:Number(n.getAttribute('cx')),cy:Number(n.getAttribute('cy')),r:Number(n.getAttribute('r')),bbox:[b.x,b.y,b.width,b.height],rect:rect(n),matrix:[m.a,m.b,m.c,m.d,m.e,m.f],fill:s.fill,stroke:s.stroke,strokeWidth:s.strokeWidth,opacity:s.opacity};})};});
  assert.equal(receipt.dom.productOwner,false);assert.equal(receipt.dom.dpr,inputs.dpr);receipt.dom.surface.forEach((v,i)=>assert(Math.abs(v-[80,24,600*inputs.cssScale[0],420*inputs.cssScale[1]][i])<=1/30));assert.equal(receipt.dom.circles.length,4*inputs.copies);
  receipt.dom.circles.forEach((c,i)=>{const r=i%4<2?6:3;assert.equal(c.tag,'circle');assert(c.connected);assert.deepEqual([c.cx,c.cy,c.r],[...inputs.center,r]);assert.deepEqual(c.bbox,[c.cx-r,c.cy-r,r*2,r*2]);assert.equal(c.opacity,'1');assert.equal(c.fill,i%2===0?'none':'rgb(240, 30, 80)');assert.equal(c.stroke,i%2===0?'rgb(255, 255, 255)':'none');assert.equal(parseFloat(c.strokeWidth),i%2===0?2:0);const[a,b,cx,d,e,f]=c.matrix,[k,tx,ty]=inputs.view;for(const[x,y]of[[c.cx-r,c.cy-r],[c.cx+r,c.cy+r]])assert(Math.max(Math.abs(a*x+cx*y+e-(k*x+40+tx-40*k)),Math.abs(b*x+d*y+f-(k*y+28+ty-28*k)))<=1/30);});
  const bytes=await page.screenshot({caret:'initial',animations:'allow'});await writeFile(join(out,receipt.png),bytes,{flag:'wx'});receipt.pngSha256=sha(bytes);receipt.browserVersion=browser.version();assert.deepEqual(errors,[]);receipt.pageErrors=errors;receipt.passed=true;
 }catch(error){receipt.error=String(error?.stack??error);throw error;}finally{try{if(context)await context.close();}finally{if(browser)await browser.close();}receipt.sourceEnd=await provenance();assert.deepEqual(receipt.sourceEnd,current);await writeFile(join(out,'reference.json'),JSON.stringify(receipt,null,2)+'\n',{flag:'wx'});}
}
export async function getDotReference({out,browser,dpr,pose}){
 const inputs={browser,dpr,...structuredClone(pose)},proof=await(initialProof??=provenance()),key=sha(JSON.stringify({inputs,proof}));
 if(!cache.has(key))cache.set(key,(async()=>{const dir=join(out,'native-dot-reference',key);await mkdir(dir,{recursive:true});const request={inputs,key,provenance:proof};await writeFile(join(dir,'request.json'),JSON.stringify(request,null,2)+'\n',{flag:'wx'});
  // Separate Node and browser process: this never changes the product page's focus/current.
  const child=execFile(process.execPath,[self,'--worker-launch-marker',join(dir,'request.json'),dir],{cwd:root,windowsHide:true,maxBuffer:1024*1024});workers.add(child);try{const result=await new Promise((done,reject)=>{let stdout='',stderr='';child.stdout.on('data',b=>stdout+=b);child.stderr.on('data',b=>stderr+=b);child.on('error',reject);child.on('close',(code,signal)=>done({code,signal,stdout,stderr}));});await writeFile(join(dir,'terminal.json'),JSON.stringify(result,null,2)+'\n',{flag:'wx'});assert.equal(result.code,0,'isolated native reference worker: '+result.stderr);}finally{workers.delete(child);}
  const receipt=JSON.parse(await readFile(join(dir,'reference.json'),'utf8'));assert(receipt.passed);return{key,inputs,json:relative(out,join(dir,'reference.json')).replaceAll('\\','/'),png:relative(out,join(dir,'reference.png')).replaceAll('\\','/'),pngSha256:receipt.pngSha256,source:receipt.source,provenance:receipt.provenance,launchOptions:receipt.launchOptions};
 })());const certificate=await cache.get(key),bytes=await readFile(join(out,certificate.png));assert.equal(sha(bytes),certificate.pngSha256);return{certificate,image:png(bytes)};
}
export async function closeDotReferences(){await Promise.all([...workers].map(child=>new Promise(done=>{child.once('close',done);child.kill();})));workers.clear();}
if(process.argv[2]==='--worker-launch-marker'){try{await worker(JSON.parse(await readFile(process.argv[3],'utf8')),resolve(process.argv[4]));}catch(error){console.error(error);process.exitCode=1;}}
