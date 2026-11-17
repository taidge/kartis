// Original Nivo anchor formula plus separate supplied-font outline rendering.
import assert from 'node:assert/strict';
import { readFile, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { stripTypeScriptTypes } from 'node:module';
import { execFileSync } from 'node:child_process';
import { createServer } from 'node:http';
import { resolve, relative, isAbsolute, join, extname } from 'node:path';
import { chromium, firefox, webkit } from 'playwright';

const metadata = JSON.parse(execFileSync('cargo', ['metadata','--no-deps','--format-version','1'], {encoding:'utf8'}));
const root = resolve(process.argv[2] || join(metadata.target_directory,'calendar-year-legend-v2'));
const output = process.argv[3] || join(root,'browser-report.json');
const vendor = resolve('tests/fixtures/calendar-month-spacing/vendor');
const pins = JSON.parse(await readFile(join(vendor,'source-pins.json'),'utf8'));
const bytes = await readFile(join(vendor,'calendar.ts'));
assert.equal(createHash('sha256').update(bytes).digest('hex'),pins.files['calendar.ts'].sha256);
const source = bytes.toString('utf8').replaceAll('\r\n','\n');
const start = source.indexOf('export const computeYearLegendPositions =');
const end = source.indexOf('export const computeMonthLegendPositions =',start);
assert(start>=0 && end>start);
const code = stripTypeScriptTypes(source.slice(start,end),{mode:'transform'}).replace('export const','const');
const original = new Function(code+'\nreturn computeYearLegendPositions;')();
const frames = JSON.parse(await readFile(join(root,'frames.json'),'utf8'));
assert.equal(frames.length,24);
let originalComparisons=0;
for(const frame of frames){
  const expected = original({years:frame.years,direction:frame.direction,position:frame.position,offset:frame.offset});
  assert.equal(expected.length,2);
  for(let i=0;i<expected.length;i++){
    const l=frame.labels[i],e=expected[i],r=l.rotation*Math.PI/180;
    // Existing Calendar applies a size/3 rotated alphabetic-baseline shift.
    // Compare the unshifted placement anchor; this is not Nivo middle baseline.
    assert(Math.abs(l.x+Math.sin(r)*l.size/3-e.x)<1e-9,`${frame.name} x`);
    assert(Math.abs(l.y-Math.cos(r)*l.size/3-e.y)<1e-9,`${frame.name} y`);
    assert.equal(l.rotation,e.rotation);
    assert.equal(l.text,`FY ${e.year}`);
    originalComparisons++;
  }
}
const server=createServer(async(req,res)=>{try{
 const file=resolve(root,'.'+new URL(req.url,'http://localhost').pathname.replace(/^\/$/,'/index.html'));
 const child=relative(root,file);if(child.startsWith('..')||isAbsolute(child))throw Error('outside export');
 res.setHeader('Content-Type',{'.html':'text/html','.svg':'image/svg+xml','.png':'image/png','.json':'application/json'}[extname(file)]||'application/octet-stream');res.end(await readFile(file));
}catch{res.writeHead(404);res.end();}});
await new Promise(done=>server.listen(0,'127.0.0.1',done));
const report={passed:false,nivoCommit:pins.nivoCommit,nivoSourceSHA256:pins.files['calendar.ts'].sha256,originalComparisons,contexts:[]};
try{
 const browsers=process.argv.includes('--chromium-only') ? [['Chromium',chromium,{}]] : [['Chrome',chromium,{channel:'chrome'}],['Firefox',firefox,{}],['WebKit',webkit,{}],['Edge',chromium,{channel:'msedge'}]];
 for(const [name,engine,options] of browsers){
  const browser=await engine.launch({headless:true,...options});
  try{for(const dpr of [1,2]){const context=await browser.newContext({deviceScaleFactor:dpr});try{
   const page=await context.newPage(),errors=[];page.on('pageerror',e=>errors.push(String(e)));
   await page.goto(`http://127.0.0.1:${server.address().port}/`);
   const result=await page.evaluate(async()=>{
    const frames=JSON.parse(document.querySelector('#frames').textContent);let comparisons=0;
    function ink(c){const a=c.getContext('2d').getImageData(0,0,c.width,c.height).data;let b=[c.width,c.height,0,0];for(let y=0;y<c.height;y++)for(let x=0;x<c.width;x++){let i=(y*c.width+x)*4;if(a[i]<180&&a[i+1]<180&&a[i+2]<180&&a[i+3]>128){b[0]=Math.min(b[0],x);b[1]=Math.min(b[1],y);b[2]=Math.max(b[2],x+1);b[3]=Math.max(b[3],y+1);}}if(b[2]<=b[0])throw Error('missing outline ink');return b.map(v=>v/(c.width/500));}
    for(const f of frames){
     const image=new Image();image.src=`${f.name}.svg`;await image.decode();
     const svg=document.createElement('canvas');svg.width=500*devicePixelRatio;svg.height=400*devicePixelRatio;svg.getContext('2d').drawImage(image,0,0,svg.width,svg.height);
     for(const c of [svg,document.getElementById(`canvas-${f.name}`)]){const b=ink(c);if(b.some((v,i)=>Math.abs(v-f.ink[i])>1.5))throw Error(`${f.name}: ${b} vs CPU ${f.ink}`);comparisons++;}
    }
    return {frames:frames.length,comparisons};
   });
   assert.equal(errors.length,0,errors.join('\n'));assert.equal(result.frames,24);assert.equal(result.comparisons,48);
   report.contexts.push({browser:name,dpr,...result,passed:true});
  }finally{await context.close();}}}finally{await browser.close();}
 }
 report.passed=report.contexts.length===browsers.length*2;
}finally{server.close();await writeFile(output,JSON.stringify(report,null,2));}
console.log(JSON.stringify(report,null,2));
