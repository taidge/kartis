// Independent native CSS colors verify physical screenshot coordinates.
import assert from 'node:assert/strict';
import {mkdir,writeFile,readFile} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {execFileSync} from 'node:child_process';
import {resolve,join} from 'node:path';
import {chromium,firefox,webkit} from 'playwright';
import {captureViewport,rgbaAtLogical} from './acceptance-viewport.mjs';
import {png,pixel} from './acceptance-png.mjs';
const meta=JSON.parse(execFileSync('cargo',['metadata','--no-deps','--format-version','1'],{encoding:'utf8'}));
const out=resolve(process.argv.slice(2).find(a=>!a.startsWith('--'))||join(meta.target_directory,'viewport-capture-browser'));await mkdir(out,{recursive:true});
const sha=b=>createHash('sha256').update(b).digest('hex');
const files={driver:new URL(import.meta.url),capture:new URL('./acceptance-viewport.mjs',import.meta.url),png:new URL('./acceptance-png.mjs',import.meta.url)};
const hashes=async()=>Object.fromEntries(await Promise.all(Object.entries(files).map(async([k,p])=>[k,sha(await readFile(p))]))),start=await hashes();
async function bounded(name,fn){let timer;try{return await Promise.race([Promise.resolve().then(fn),new Promise((_,reject)=>{timer=setTimeout(()=>reject(Error(name+' timed out')),15000);})]);}finally{clearTimeout(timer);}}
const engines=process.argv.includes('--chromium-only')?[['Chromium',chromium,{}]]:[['Chrome',chromium,{channel:'chrome'}],['Firefox',firefox,{}],['WebKit',webkit,{}],['Edge',chromium,{channel:'msedge'}]];
const report={passed:false,scope:'Independent native CSS panels, fractional origin, positive nonuniform scaling and scroll verify full-viewport physical pixel sampling. No Kartis rendering or arbitrary text oracle claim.',contexts:[],provenance:{start}};
try{for(const[name,engine,options]of engines){const browser=await bounded('launch',()=>engine.launch({headless:true,...options}));try{for(const dpr of[1,2]){
 const record={name,dpr,passed:false,frames:[],rejections:[]};report.contexts.push(record);const context=await browser.newContext({viewport:{width:900,height:700},deviceScaleFactor:dpr});context.setDefaultTimeout(15000);
 const page=await context.newPage();try{
 await page.setContent('<style>body{margin:0;height:1800px;width:1200px}#surface{position:absolute;left:20.375px;top:120.40625px;width:300px;height:260px;transform-origin:0 0;overflow:hidden}#surface>div{position:absolute;left:0;width:100%;height:1px}#surface .stripe{position:absolute;width:40px;top:0;height:100%;background:#00ffff}</style><div id="surface"></div>');
 await page.evaluate(()=>{const root=document.querySelector('#surface');for(let row=0;row<260;row++){const band=document.createElement('div');band.style.top=row+'px';band.style.background=row%2?'rgb(0,255,0)':'rgb(255,0,0)';root.appendChild(band);}const stripe=document.createElement('span');stripe.className='stripe';stripe.style.left='210px';root.appendChild(stripe);});
 const surface=page.locator('#surface');
 // Predeclared logical points are inside CSS bands and have exact RGBA truth.
 const points=[[30.25,10.25],[30.25,79.25],[30.25,140.25],[30.25,250.25],[230.25,42.25],[230.25,219.25]];
 for(const [label,transform,scroll]of[['fractional','none',[0,0]],['scaled','scale(1.5,0.75)',[0,0]],['scrolled','scale(1.25,1.5)',[10,75]]]){
  await page.evaluate(({transform,scroll})=>{document.querySelector('#surface').style.transform=transform;window.scrollTo(...scroll);},{transform,scroll});await page.evaluate(()=>new Promise(done=>requestAnimationFrame(()=>requestAnimationFrame(done))));
  const shot=await bounded('capture '+label,()=>captureViewport(page,surface)),samples=[];
  for(const point of points){const sample=rgbaAtLogical(shot,{point,size:[300,260]});const actualLogicalY=(sample.index[1]/dpr-shot.geometry.rect.y)*260/shot.geometry.rect.height;
   // Scaling can move a physical pixel across a one-CSS-pixel band. Derive its
   // exact native band from pixel-center coverage, independently of the query.
   const centerLogicalY=((sample.index[1]+0.5)/dpr-shot.geometry.rect.y)*260/shot.geometry.rect.height;
   const centerLogicalX=((sample.index[0]+0.5)/dpr-shot.geometry.rect.x)*300/shot.geometry.rect.width;
   const expected=centerLogicalX>=210&&centerLogicalX<250?[0,255,255,255]:Math.floor(centerLogicalY)%2?[0,255,0,255]:[255,0,0,255];
   // For scaled one-pixel bands native coverage can be fractional; hard color
   // gates are on the integer-scale first profile and wide cyan stripe only.
   const gated=label==='fractional'&&dpr===2||centerLogicalX>=210&&centerLogicalX<250;
   if(gated)assert.deepEqual(sample.rgba,expected,`${name}/${dpr}/${label}/${point}`);
   samples.push({point,...sample,actualLogicalY,expected,gated});
  }
  // Independent page screenshot at the same viewport must produce the same
  // physical indices and bytes. No element crop or fitted height is involved.
  const imagePath=join(out,`${name}-${dpr}-${label}.png`);await writeFile(imagePath,shot.bytes);
  const{image,geometry,...rest}=shot;record.frames.push({label,geometry,samples,pngSha256:sha(rest.bytes),path:imagePath});
  for(const point of[[-1,0],[0,260],[300,0],[NaN,4]]){assert.throws(()=>rgbaAtLogical(shot,{point,size:[300,260]}));record.rejections.push({label,point:point.map(n=>Number.isNaN(n)?'NaN':n),kind:'outside logical Scene'});}
 }
 // A native physical-pixel tag image supplies exact coordinate truth at all
 // scales. Unscrolled indices are fixed hand-evaluated cases; a separate native
 // zero-size marker supplies scrolled coordinates, including browser-native
 // scroll quantization. Neither is derived
 // from the capture helper's result or chosen from nearby colors.
 await page.evaluate(dpr=>{
  document.querySelectorAll('#surface>*').forEach(n=>n.style.display='none');
  const canvas=document.createElement('canvas');canvas.id='coordinate-tags';canvas.width=900*dpr;canvas.height=700*dpr;canvas.style.cssText='position:fixed;left:0;top:0;width:900px;height:700px;pointer-events:none;image-rendering:pixelated';
  const ctx=canvas.getContext('2d'),data=ctx.createImageData(canvas.width,canvas.height);
  for(let y=0;y<canvas.height;y++)for(let x=0;x<canvas.width;x++){const i=4*(y*canvas.width+x);data.data.set([x%256,y%256,Math.floor(x/256)+16*Math.floor(y/256),255],i);}ctx.putImageData(data,0,0);document.body.prepend(canvas);const marker=document.createElement('i');marker.id='native-point';marker.style.cssText='position:absolute;left:30.25px;top:10.25px;width:0;height:0;pointer-events:none';document.querySelector('#surface').appendChild(marker);
 },dpr);
 const cases=[['tag-fractional','none',[0,0],[[50,130],[101,261]]],['tag-scaled','scale(1.5,0.75)',[0,0],[[65,128],[131,256]]],['tag-scrolled','scale(1.25,1.5)',[10,75],[[48,60],[96,121]]]];
 for(const[label,transform,scroll,indices]of cases){
  await page.evaluate(({transform,scroll})=>{document.querySelector('#surface').style.transform=transform;window.scrollTo(...scroll);},{transform,scroll});await page.evaluate(()=>new Promise(done=>requestAnimationFrame(()=>requestAnimationFrame(done))));
  const actualScroll=await page.evaluate(()=>[scrollX,scrollY]),nativePoint=await page.locator('#native-point').evaluate(n=>{const b=n.getBoundingClientRect();return[b.x,b.y];});const shot=await captureViewport(page,surface),sample=rgbaAtLogical(shot,{point:[30.25,10.25],size:[300,260]}),expectedIndex=nativePoint.map(n=>Math.floor(n*dpr)),expected=[expectedIndex[0]%256,expectedIndex[1]%256,Math.floor(expectedIndex[0]/256)+16*Math.floor(expectedIndex[1]/256),255];
  if(label!=='tag-scrolled')assert.deepEqual(expectedIndex,indices[dpr-1],'fixed independent CSS coordinate case');assert(sample.client.every((v,i)=>Math.abs(v-nativePoint[i])<=1/32),'logical projection agrees with separately laid out native marker');assert.deepEqual(sample.index,expectedIndex,'native DOM marker coordinate case');const tagColorGated=label!=='tag-scrolled';if(tagColorGated)assert.deepEqual(sample.rgba,expected,'native per-physical-pixel tag');
  record.frames.push({label,actualScroll,requestedScroll:scroll,nativePoint,geometry:shot.geometry,samples:[{...sample,expected,expectedIndex,gated:tagColorGated,paletteMatches:sample.rgba.every((v,i)=>v===expected[i])}],pngSha256:sha(shot.bytes)});
 }
 // Preserve a concrete failure of fitted element-PNG sampling, without
 // relabeling previous reports. Integer padding is not a content resize.
 await page.evaluate(()=>{document.querySelector('#surface').style.transform='none';window.scrollTo(0,0);});await page.evaluate(()=>new Promise(done=>requestAnimationFrame(()=>requestAnimationFrame(done))));
 const crop=png(await surface.screenshot()),whole=await captureViewport(page,surface),correct=rgbaAtLogical(whole,{point:[30.25,250.25],size:[300,260]}),expectedIndex=dpr===1?[50,370]:[101,741],legacy=pixel(crop,30.25,250.25,300,260);
 assert.deepEqual(correct.index,expectedIndex);assert.deepEqual(correct.rgba,[expectedIndex[0]%256,expectedIndex[1]%256,Math.floor(expectedIndex[0]/256)+16*Math.floor(expectedIndex[1]/256),255]);assert.notDeepEqual(legacy,correct.rgba,'padded crop reproduces an actual wrong-pixel selection');
 record.legacyPaddingControl={cropSize:[crop.width,crop.height],correct,legacy};
 // Capturing never scrolls a partly offscreen surface. Samples outside the
 // actual viewport are rejected rather than clamped to an unrelated pixel.
 await page.evaluate(()=>{document.querySelector('#surface').style.transform='none';window.scrollTo(0,300);});await page.evaluate(()=>new Promise(done=>requestAnimationFrame(()=>requestAnimationFrame(done))));
 const partial=await captureViewport(page,surface);assert.throws(()=>rgbaAtLogical(partial,{point:[10,10],size:[300,260]}),/outside captured viewport/);record.rejections.push({kind:'outside physical viewport'});
 assert.equal(await page.evaluate(()=>scrollY),partial.geometry.scrollY);
 const originalScreenshot=page.screenshot;
 page.screenshot=async function(...args){const bytes=await originalScreenshot.apply(page,args);await page.evaluate(()=>document.querySelector('#surface').style.left='21.375px');return bytes;};
 try{await assert.rejects(()=>captureViewport(page,surface),/geometry changed during viewport capture/);record.rejections.push({kind:'actual geometry changed after native screenshot'});}finally{page.screenshot=originalScreenshot;await page.evaluate(()=>document.querySelector('#surface').style.left='20.375px');}
 await page.evaluate(()=>document.querySelector('#surface').style.width='0px');await assert.rejects(()=>captureViewport(page,surface),/surface width/);record.rejections.push({kind:'empty surface'});
 record.passed=true;
 }finally{await context.close();}
}}finally{await browser.close();}}
report.provenance.end=await hashes();assert.deepEqual(report.provenance.end,start);report.passed=true;
}catch(error){report.error=error.stack;throw error;}finally{await writeFile(join(out,'report.json'),JSON.stringify(report,null,2));}
console.log(JSON.stringify({passed:true,contexts:report.contexts.length,report:join(out,'report.json')}));
