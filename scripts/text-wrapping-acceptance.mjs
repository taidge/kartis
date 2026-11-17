// Real browser text metrics plus SVG-path/Canvas/CPU ink placement.
import { readFile, writeFile } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import { createServer } from 'node:http';
import { resolve, join, relative, isAbsolute, extname } from 'node:path';
import { chromium, firefox, webkit } from 'playwright';
const metadata=JSON.parse(execFileSync('cargo',['metadata','--no-deps','--format-version','1'],{encoding:'utf8'}));
const root=resolve(process.argv[2]||join(metadata.target_directory,'text-wrapping-export'));
const server=createServer(async(req,res)=>{try{
  const name=decodeURIComponent(new URL(req.url,'http://localhost').pathname);
  const file=resolve(root,'.'+(name==='/'?'/index.html':name));
  const child=relative(root,file); if(child.startsWith('..')||isAbsolute(child))throw Error('outside export');
  res.setHeader('Content-Type',{'.html':'text/html','.svg':'image/svg+xml','.png':'image/png'}[extname(file)]||'application/octet-stream');res.end(await readFile(file));
}catch{res.writeHead(404);res.end();}});
await new Promise(done=>server.listen(0,'127.0.0.1',done));
const report={passed:false,contexts:[]};
try{
 for(const [name,engine,opts]of[['Chrome',chromium,{channel:'chrome'}],['Firefox',firefox,{}],['WebKit',webkit,{}],['Edge',chromium,{channel:'msedge'}]]){
  const browser=await engine.launch({headless:true,...opts});
  try{for(const dpr of [1,2]){
   const context=await browser.newContext({deviceScaleFactor:dpr});
   try{
    const page=await context.newPage();const errors=[];page.on('pageerror',e=>errors.push(String(e)));
    await page.goto(`http://127.0.0.1:${server.address().port}/`);
    await page.waitForFunction(()=>window.wrapReady||window.wrapError);
    const result=await page.evaluate(async()=>{
     if(window.wrapError)throw Error(window.wrapError);
     const blocks=JSON.parse(document.querySelector('#blocks').textContent);
     const labels=blocks.flatMap(b=>b.lines.map(line=>({...line,maxWidth:b.width}))); const texts=[...document.querySelectorAll('svg text')];
     if(texts.length!==labels.length)throw Error('lost editable line');
     const measure=document.createElement('canvas').getContext('2d');measure.font='18px WrapFixture';
     let maxAdvanceError=0;
     labels.forEach((line,i)=>{
      if(texts[i].textContent!==line.text)throw Error('line content differs');
      const advance=measure.measureText(line.text).width;
      const delta=Math.abs(advance-line.advance);
      maxAdvanceError=Math.max(maxAdvanceError,delta);
      if(advance>line.maxWidth)throw Error(`native line exceeds its column ${line.text}: ${advance} > ${line.maxWidth}`);
     });
     if(blocks.length!==3||!blocks[2].truncated)throw Error('ellipsis case not exercised');
     async function imageCanvas(url){const i=new Image();i.src=url;await i.decode();const c=document.createElement('canvas');c.width=500*devicePixelRatio;c.height=360*devicePixelRatio;c.getContext('2d').drawImage(i,0,0,c.width,c.height);return c;}
     function bounds(c){const d=c.getContext('2d').getImageData(0,0,c.width,c.height).data;let x0=c.width,y0=c.height,x1=-1,y1=-1;for(let y=0;y<c.height;y++)for(let x=0;x<c.width;x++){let p=(y*c.width+x)*4;if(d[p]<180&&d[p+1]<180&&d[p+2]<180&&d[p+3]>128){x0=Math.min(x0,x);x1=Math.max(x1,x);y0=Math.min(y0,y);y1=Math.max(y1,y);}}if(x1<0)throw Error('no text ink');const s=c.width/500;return [x0/s,y0/s,(x1+1)/s,(y1+1)/s];}
     const cpu=bounds(await imageCanvas('wrapped.png'));
     const candidates=[document.querySelector('#outlined'),await imageCanvas('outlined.svg')];
     for(const c of candidates){const b=bounds(c);if(b.some((v,i)=>Math.abs(v-cpu[i])>1.5))throw Error(`ink placement ${b} != ${cpu}`);}
     return {lines:labels.length,nativeColumnFit:true,nativeAdvanceExactWithinPointOne:maxAdvanceError<=0.1,maxAdvanceError,inkBounds:cpu,outlineBackends:2};
    });
    if(errors.length)throw Error(errors.join('\n'));
    report.contexts.push({browser:name,dpr,passed:true,...result});
   }finally{await context.close();}
  }}finally{await browser.close();}
 }
 report.passed=report.contexts.length===8;
}finally{server.close();await writeFile(process.argv[3]||join(root,'browser-report.json'),JSON.stringify(report,null,2));}
console.log(JSON.stringify(report,null,2));
