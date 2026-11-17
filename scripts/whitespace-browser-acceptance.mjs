import { readFile, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { execFileSync } from 'node:child_process';
import { resolve,join,relative,isAbsolute,extname } from 'node:path';
import { chromium,firefox,webkit } from 'playwright';
const metadata=JSON.parse(execFileSync('cargo',['metadata','--no-deps','--format-version','1'],{encoding:'utf8'}));
const root=resolve('examples/browser-widget');
const server=createServer(async(req,res)=>{try{const file=resolve(root,'.'+new URL(req.url,'http://localhost').pathname);const child=relative(root,file);if(child.startsWith('..')||isAbsolute(child))throw Error('outside fixture');res.setHeader('Content-Type',{'.html':'text/html','.js':'application/javascript','.wasm':'application/wasm'}[extname(file)]||'application/octet-stream');res.end(await readFile(file));}catch{res.writeHead(404);res.end();}});
await new Promise(done=>server.listen(0,'127.0.0.1',done));
const report={passed:false,contexts:[]};
try{for(const[name,engine,opts]of[['Chrome',chromium,{channel:'chrome'}],['Firefox',firefox,{}],['WebKit',webkit,{}],['Edge',chromium,{channel:'msedge'}]]){
 const browser=await engine.launch({headless:true,...opts});
 try{for(const dpr of[1,2]){const context=await browser.newContext({deviceScaleFactor:dpr});try{
 const page=await context.newPage();const errors=[];page.on('pageerror',e=>errors.push(String(e)));await page.goto(`http://127.0.0.1:${server.address().port}/whitespace.html`);try{await page.waitForFunction(()=>window.whitespaceReady||window.whitespaceBootError);}catch(e){throw Error(`Fixture did not initialize: ${errors.join('; ')}; ${e}`);}const bootError=await page.evaluate(()=>window.whitespaceBootError);if(bootError)throw Error(bootError);
 const result=await page.evaluate(async()=>{
  const svg=document.querySelector('#retained svg text');if(!svg)throw Error('missing retained text');
  let previous=0;const phases=[];
  for(const preserve of[false,true,false,true]){
   window.whitespace.whitespace_mode(preserve);
   await new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));
   const current=document.querySelector('#retained svg text');if(current!==svg)throw Error('lost retained node identity');
   const candidates=[...document.querySelectorAll('#semantic *')].filter(e=>e.childElementCount===0&&e.textContent.includes('A')&&e.textContent.includes('B'));
   if(candidates.length!==1)throw Error(`ordinary text candidates ${candidates.length}`);
   const text=candidates[0], style=getComputedStyle(text), expected=preserve?'  A   B  C  ':'  A \t B  C  ';
   if(svg.textContent!==expected||text.textContent!==expected)throw Error(`content mismatch ${JSON.stringify({svg:svg.textContent,text:text.textContent,expected})}`);
   if(preserve &&(getComputedStyle(svg).whiteSpace!=='pre'||style.whiteSpace!=='pre'))throw Error('Preserve CSS missing');
   if(!preserve&&(svg.hasAttribute('style')||style.whiteSpace!=='nowrap'))throw Error('Legacy style not restored');
   const calls=window.whitespace.whitespace_measurements();if(phases.length&&calls<=previous)throw Error('mode change reused old measurement');previous=calls;
   const width=svg.getComputedTextLength(),htmlWidth=text.getBoundingClientRect().width;
   const range=document.createRange();range.selectNodeContents(text);const glyphWidth=range.getBoundingClientRect().width;
   if(preserve &&(Math.abs(width-htmlWidth)>1||Math.abs(width-glyphWidth)>1))throw Error(`Preserve SVG/ordinary advance mismatch ${width} vs ${htmlWidth}/${glyphWidth}`);
   let canvasInk=null;
   if(preserve){
    const canvas=document.querySelector('#canvas canvas');if(!canvas)throw Error('missing real Canvas host');
    const blob=new Blob([new XMLSerializer().serializeToString(svg.ownerSVGElement)],{type:'image/svg+xml'});const url=URL.createObjectURL(blob);const image=new Image();image.src=url;await image.decode();
    const copy=document.createElement('canvas');copy.width=canvas.width;copy.height=canvas.height;copy.getContext('2d').drawImage(image,0,0,copy.width,copy.height);URL.revokeObjectURL(url);
    function ink(c){const a=c.getContext('2d').getImageData(0,0,c.width,c.height).data;let x0=c.width,y0=c.height,x1=-1,y1=-1;for(let y=0;y<c.height;y++)for(let x=0;x<c.width;x++){const i=(y*c.width+x)*4;if(a[i]<180&&a[i+1]<180&&a[i+2]<180&&a[i+3]>128){x0=Math.min(x0,x);y0=Math.min(y0,y);x1=Math.max(x1,x);y1=Math.max(y1,y);}}if(x1<0)throw Error('no native text ink');const scale=c.width/320;return[x0/scale,y0/scale,(x1+1)/scale,(y1+1)/scale];}
    canvasInk=ink(canvas);const svgInk=ink(copy);if(canvasInk.some((v,i)=>Math.abs(v-svgInk[i])>1.5))throw Error(`SVG/Canvas ink placement mismatch ${canvasInk} vs ${svgInk}`);
   }
   phases.push({preserve,calls,svgWidth:width,htmlWidth,glyphWidth,canvasInk,ordinaryWhiteSpace:style.whiteSpace});
  }
  if(phases[1].svgWidth<=phases[0].svgWidth)throw Error('space preservation not visible');
  return {phases,retainedIdentity:true,ordinaryText:true};
 });if(errors.length)throw Error(errors.join('\n'));report.contexts.push({browser:name,dpr,passed:true,...result});
 }finally{await context.close();}}}finally{await browser.close();}
}report.passed=report.contexts.length===8;
}finally{server.close();await writeFile(process.argv[2]||join(metadata.target_directory,'whitespace-browser-report.json'),JSON.stringify(report,null,2));}
console.log(JSON.stringify(report,null,2));
