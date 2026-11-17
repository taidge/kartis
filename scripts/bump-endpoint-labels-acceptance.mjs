// Editable endpoint text and separately outlined supplied-font ink.
import assert from 'node:assert/strict';
import { readFile, writeFile } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import { createServer } from 'node:http';
import { resolve, join, relative, isAbsolute, extname } from 'node:path';
import { chromium, firefox, webkit } from 'playwright';

const metadata=JSON.parse(execFileSync('cargo',['metadata','--no-deps','--format-version','1'],{encoding:'utf8'}));
const root=resolve(process.argv[2]||join(metadata.target_directory,'bump-endpoint-labels-v2'));
const output=process.argv[3]||join(root,'browser-report.json');
const server=createServer(async(req,res)=>{try{
 const path=new URL(req.url,'http://localhost').pathname;
 const file=resolve(root,'.'+(path==='/'?'/index.html':path)),child=relative(root,file);
 if(child.startsWith('..')||isAbsolute(child))throw Error('outside export');
 res.setHeader('Content-Type',{'.html':'text/html','.svg':'image/svg+xml','.png':'image/png','.json':'application/json'}[extname(file)]||'application/octet-stream');res.end(await readFile(file));
}catch{res.writeHead(404);res.end();}});
await new Promise(done=>server.listen(0,'127.0.0.1',done));
const report={passed:false,contexts:[]};
const browsers=process.argv.includes('--chromium-only')?[['Chromium',chromium,{}]]:[['Chrome',chromium,{channel:'chrome'}],['Firefox',firefox,{}],['WebKit',webkit,{}],['Edge',chromium,{channel:'msedge'}]];
try{
 for(const [name,engine,options]of browsers){const browser=await engine.launch({headless:true,...options});try{
  for(const dpr of [1,2]){const context=await browser.newContext({deviceScaleFactor:dpr});try{
   const page=await context.newPage(),errors=[];page.on('pageerror',e=>errors.push(String(e)));
   await page.goto(`http://127.0.0.1:${server.address().port}/`);
   await page.waitForFunction(()=>window.bumpReady||window.bumpError);
   const result=await page.evaluate(async()=>{
    if(window.bumpError)throw Error(window.bumpError);
    if(![...document.fonts].some(f=>f.family==='BumpFixture'&&f.status==='loaded'))throw Error('fixture face not loaded');
    const frames=JSON.parse(document.querySelector('#frames').textContent);let labels=0,emptyLabels=0,inkComparisons=0,emptyFrames=0,nativeCanvasLabels=0;
    function bounds(c){const a=c.getContext('2d').getImageData(0,0,c.width,c.height).data;const b=[c.width,c.height,0,0];for(let y=0;y<c.height;y++)for(let x=0;x<c.width;x++){const i=(y*c.width+x)*4;if(a[i]<180&&a[i+1]<180&&a[i+2]<180&&a[i+3]>128){b[0]=Math.min(b[0],x);b[1]=Math.min(b[1],y);b[2]=Math.max(b[2],x+1);b[3]=Math.max(b[3],y+1);}}return b[2]>b[0]?b.map(v=>v/(c.width/620)):null;}
    for(const f of frames){
     const texts=[...document.getElementById(f.name).querySelectorAll('svg text')];
     if(texts.length!==f.labels.length)throw Error(`${f.name}: lost owned text slots`);
     f.labels.forEach((l,i)=>{
      const t=texts[i];if(t.textContent!==l.text)throw Error(`${f.name}: text mismatch`);
      if(Math.abs(Number(t.getAttribute('x'))-l.x)>1e-9||Math.abs(Number(t.getAttribute('y'))-l.y)>1e-9||t.getAttribute('text-anchor')!==l.anchor)throw Error(`${f.name}: position/anchor changed`);
      if(!getComputedStyle(t).fontFamily.includes('BumpFixture'))throw Error('wrong editable face');
      labels++;if(!l.text)emptyLabels++;
     });
     const calls=window.bumpNativeCalls.filter(c=>c.id===`native-${f.name}`);
     if(calls.length!==f.labels.length)throw Error(`${f.name}: lost native Canvas text calls`);
     f.labels.forEach((l,i)=>{
      const c=calls[i];if(c.text!==l.text||Math.abs(c.x-l.x)>1e-9||Math.abs(c.y-l.y)>1e-9||c.anchor!==l.anchor||c.baseline!=='alphabetic'||!c.font.includes('BumpFixture'))throw Error(`${f.name}: native Canvas text/position/font differs`);
      nativeCanvasLabels++;
     });
     if((bounds(document.getElementById(`native-${f.name}`))===null)!==(f.ink===null))throw Error(`${f.name}: native Canvas empty/nonempty differs`);
     const image=new Image();image.src=`${f.name}.svg`;await image.decode();
     const svg=document.createElement('canvas');svg.width=620*devicePixelRatio;svg.height=320*devicePixelRatio;svg.getContext('2d').drawImage(image,0,0,svg.width,svg.height);
     if(f.ink===null)emptyFrames++;
     for(const c of [svg,document.getElementById(`canvas-${f.name}`)]){
      const b=bounds(c);if((b===null)!==(f.ink===null))throw Error(`${f.name}: unexpected empty/nonempty ink`);
      if(b&&b.some((v,i)=>Math.abs(v-f.ink[i])>1.5))throw Error(`${f.name}: ink ${b} vs CPU ${f.ink}`);
      inkComparisons++;
     }
    }
    return {frames:frames.length,labels,emptyLabels,emptyFrames,inkComparisons,nativeCanvasLabels};
   });
   assert.equal(errors.length,0,errors.join('\n'));assert.deepEqual(result,{frames:16,labels:24,emptyLabels:8,emptyFrames:6,inkComparisons:32,nativeCanvasLabels:24});
   report.contexts.push({browser:name,dpr,passed:true,...result});
  }finally{await context.close();}}
 }finally{await browser.close();}}
 report.passed=report.contexts.length===browsers.length*2;
}finally{server.close();await writeFile(output,JSON.stringify(report,null,2));}
console.log(JSON.stringify(report,null,2));
