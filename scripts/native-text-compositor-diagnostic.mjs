// Native browser diagnosis, not a universal glyph/color acceptance oracle.
import assert from 'node:assert/strict';
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {execFileSync} from 'node:child_process';
import {resolve,join} from 'node:path';
import {chromium} from 'playwright';
import {captureViewport,rgbaAtLogical} from './acceptance-viewport.mjs';
const meta=JSON.parse(execFileSync('cargo',['metadata','--no-deps','--format-version','1'],{encoding:'utf8'}));
const out=resolve(process.argv[2]||join(meta.target_directory,'native-text-compositor'));await mkdir(out,{recursive:true});
const inputPath=new URL('../tests/fixtures/native-widget-text/input.json',import.meta.url),bytes=await readFile(inputPath),input=JSON.parse(bytes),sha=b=>createHash('sha256').update(b).digest('hex');
const files={driver:new URL(import.meta.url),capture:new URL('./acceptance-viewport.mjs',import.meta.url),png:new URL('./acceptance-png.mjs',import.meta.url),input:inputPath};
const hashes=async()=>Object.fromEntries(await Promise.all(Object.entries(files).map(async([k,p])=>[k,sha(await readFile(p))]))),start=await hashes();
const report={completed:false,featureAccepted:false,productionFixImplemented:false,scope:'Independent native HTML only; default versus diagnostic --disable-lcd-text profiles. Every variant is reported; no closest-color selection, default production flag change or universal text parity claim.',contexts:[],provenance:{start}};
async function bounded(label,fn){let timer;try{return await Promise.race([Promise.resolve().then(fn),new Promise((_,reject)=>{timer=setTimeout(()=>reject(Error(label+' timed out')),15000);})]);}finally{clearTimeout(timer);}}
try{for(const flags of[[],['--disable-lcd-text']]){
 const browser=await bounded('launch',()=>chromium.launch({headless:true,args:flags}));
 try{const context=await browser.newContext({viewport:{width:1400,height:1100},deviceScaleFactor:1}),page=await context.newPage();context.setDefaultTimeout(15000);
 const row={flags,browserVersion:browser.version(),variants:[]};report.contexts.push(row);
 for(const variant of['same-color-font8','same-color-font0','black-font8']){
  await page.setContent('<html lang="en" dir="ltr"><style>html,body{margin:0;background:white}.plot{position:absolute;background:white;overflow:hidden}.probe{position:absolute;display:block;min-width:0;border:0;border-radius:0;padding:0;margin:0;box-shadow:none;outline:none;appearance:none;overflow:hidden}</style><div class="plot"><button class="probe"></button></div>');
  await page.evaluate(({input,variant})=>{const root=document.querySelector('.plot'),button=document.querySelector('.probe');const set=(node,r)=>{[node.style.left,node.style.top,node.style.width,node.style.height]=r.map(v=>v+'px');};set(root,input.plot);set(button,input.button);button.style.background='rgb(0,100,255)';button.style.color=variant==='black-font8'?'black':'rgb(0,100,255)';button.style.font=variant==='same-color-font0'?'0px monospace':'8px monospace';button.textContent=input.text;},{input,variant});
  const capture=await bounded('native viewport',()=>captureViewport(page,page.locator('.plot')));
  const sample=rgbaAtLogical(capture,{point:input.point,size:[600,260]}),region=[];
  for(let y=141;y<152;y++)for(let x=249;x<274;x++)region.push(rgbaAtLogical(capture,{point:[x,y],size:[600,260]}).rgba);
  const filename=`${flags.length?'without-lcd':'default'}-${variant}.png`;await writeFile(join(out,filename),capture.bytes);
  const facts=await page.locator('.probe').evaluate(n=>{const b=n.getBoundingClientRect(),s=getComputedStyle(n);return{rect:[b.x,b.y,b.width,b.height],font:s.font,color:s.color,background:s.backgroundColor,language:document.documentElement.lang,direction:document.documentElement.dir,textRange:(()=>{const r=document.createRange();r.selectNodeContents(n);const b=r.getBoundingClientRect();return[b.x,b.y,b.width,b.height];})(),userAgent:navigator.userAgent};});
  assert.equal(await page.evaluate(()=>typeof window.hostFixture),'undefined','native diagnostic must not load the chart host');
  row.variants.push({variant,facts,geometry:capture.geometry,sample,region,pngSha256:sha(capture.bytes),file:filename});
 }
 await context.close();}finally{await browser.close();}
}
 const a=report.contexts[0].variants,b=report.contexts[1].variants;
 report.comparisons=a.map((v,i)=>({variant:v.variant,changedPixels:v.region.filter((p,j)=>p.some((c,k)=>c!==b[i].region[j][k])).length,maxChannelDifference:Math.max(...v.region.flatMap((p,j)=>p.map((c,k)=>Math.abs(c-b[i].region[j][k]))))}));
 report.provenance.end=await hashes();assert.deepEqual(report.provenance.end,start);report.completed=true;
}catch(error){report.error=error.stack;throw error;}finally{await writeFile(join(out,'report.json'),JSON.stringify(report,null,2));}
console.log(JSON.stringify({completed:report.completed,featureAccepted:false,comparisons:report.comparisons,report:join(out,'report.json')}));
