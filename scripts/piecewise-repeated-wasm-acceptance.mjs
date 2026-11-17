// Full frozen original d3-scale matrix through the compiled WASM core.
// Numeric verification only: this does not claim chart paint or compositor QA.
import assert from 'node:assert/strict';
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {createServer} from 'node:http';
import {execFileSync} from 'node:child_process';
import {resolve,join,relative,isAbsolute,extname} from 'node:path';
import {chromium} from 'playwright';

const root=resolve('.'),metadata=JSON.parse(execFileSync('cargo',['metadata','--offline','--no-deps','--format-version','1'],{encoding:'utf8',windowsHide:true}));
const stamp=new Date().toISOString().replaceAll(':','-').replaceAll('.','-');
const out=resolve(process.argv[2]||join(metadata.target_directory,`piecewise-repeated-wasm-${stamp}`));
await mkdir(out,{recursive:true});await writeFile(join(out,'run.json'),JSON.stringify({startedAt:new Date().toISOString(),script:'piecewise-repeated-wasm-acceptance.mjs'})+'\n',{flag:'wx'});
const sha=b=>createHash('sha256').update(b).digest('hex');
const bytes=await readFile('tests/fixtures/piecewise-repeated/values.json'),reference=JSON.parse(await readFile('tests/fixtures/piecewise-repeated/reference.json','utf8')),fixture=JSON.parse(bytes);
assert.equal(sha(bytes),reference.valuesSHA256,'Frozen original native values hash');assert.equal(fixture.cases.length,9496);
assert.equal(sha(await readFile('scripts/piecewise-repeated-reference.mjs')),reference.producerSHA256,'Original producer hash');
for(const [path,hash]of Object.entries(reference.sourceSHA256))assert.equal(sha(await readFile(join(root,'node_modules',path))),hash,path);
const encode=v=>typeof v==='number'?(Object.is(v,-0)?'-0':Number.isNaN(v)?'NaN':v===Infinity?'Infinity':v===-Infinity?'-Infinity':v):v;
const stringify=v=>JSON.stringify(v,(_,v)=>encode(v),2)+'\n';
const number=v=>typeof v==='number'?v:Number(v);
const same=(a,b)=>a===b||Number.isNaN(a)&&Number.isNaN(b);
const close=(a,b)=>same(a,b)||Number.isFinite(a)&&Number.isFinite(b)&&Math.abs(a-b)<=Math.max(1,Math.abs(b))*2e-12;
const pkg=resolve('examples/browser-host/pkg');
const html='<!doctype html><meta charset="utf-8"><title>Frozen numeric WASM probe</title><div id="svg-root" style="width:640px;height:400px"></div><pre id="svg-log"></pre><div id="canvas-root" style="width:640px;height:400px"></div><pre id="canvas-log"></pre><script type="module">try{const host=await import("/pkg/host.js");await host.default();host.stop("svg");host.stop("canvas");if(typeof host.piecewise_repeated_probe!=="function")throw Error("missing rebuilt bulk WASM probe export");window.numericProbe=host.piecewise_repeated_probe;window.numericReady=true;}catch(e){window.numericBootError=String(e);throw e;}</script>';
const server=createServer(async(req,res)=>{try{
    if(!['GET','HEAD'].includes(req.method)){res.writeHead(405);res.end();return;}
    const path=decodeURIComponent(new URL(req.url,'http://localhost').pathname);
    if(path==='/'){res.setHeader('Content-Type','text/html;charset=utf-8');res.end(req.method==='HEAD'?undefined:html);return;}
    if(!path.startsWith('/pkg/'))throw Error('Unknown path');const file=resolve(pkg,path.slice(5)),child=relative(pkg,file);
    if(child.startsWith('..')||isAbsolute(child))throw Error('Outside package');const body=await readFile(file);res.setHeader('Content-Type',{'.js':'text/javascript','.wasm':'application/wasm'}[extname(file)]||'application/octet-stream');res.end(req.method==='HEAD'?undefined:body);
}catch{res.writeHead(404);res.end();}});
await new Promise(done=>server.listen(0,'127.0.0.1',done));
const report={passed:false,scope:'All 9496 frozen original d3-scale main profiles on actual compiled WASM, in one Chromium context. The harness does not mock or replace math. Production wasm32+web general signed power uses its real host Math.pow; native/standalone core uses platform powf. Every fixture profile is checked, including expected construction rejections. Does not execute viewport, paint, Firefox, WebKit, OS Safari or physical input.',platform:process.platform,node:process.version,valuesSHA256:sha(bytes),producerSHA256:reference.producerSHA256,probeSourceSHA256:sha(await readFile('examples/browser-host/src/browser/piecewise_repeated_probe.rs')),driverSHA256:sha(await readFile(new URL(import.meta.url))),hostJavaScriptSHA256:sha(await readFile(join(pkg,'host.js'))),hostWasmSHA256:sha(await readFile(join(pkg,'host_bg.wasm'))),playwrightVersion:JSON.parse(await readFile('node_modules/playwright/package.json','utf8')).version,policy:fixture.policy,counts:{cases:0,expectedAccepted:0,expectedRejected:0,actualAccepted:0,actualRejected:0,checkedMaps:0,checkedInputBranchRejects:0,globalInverses:0,branchInverses:0,ticks:0,roundedHalfUlpDifferences:0,blockedOperations:0},packets:[],failures:[],pageErrors:[]};
const diff=(index,c,operation,details)=>report.failures.push({index,name:c.name,kind:c.kind,parameter:c.parameter,domain:c.domain,range:c.range,options:c.options,operation,...details});
function sequence(index,c,label,actual,expected,exact=false){if(actual.length!==expected.length){diff(index,c,label,{reason:'length',actual:actual.length,expected:expected.length});return;}for(let i=0;i<expected.length;i++){const a=number(actual[i]),b=number(expected[i]);if(!(exact?Object.is(a,b):close(a,b)))diff(index,c,`${label}/${i}`,{actual:a,expected:b});}}
function inverted(index,c,label,actual,expected,permitted){if(!permitted||!Number.isFinite(expected)){if(actual.ok)diff(index,c,label,{reason:'expected checked inverse rejection',actual:actual.value,expected});return;}if(!actual.ok){diff(index,c,label,{reason:'unexpected inverse rejection',error:actual.error,expected});return;}const a=number(actual.value);if(!(c.kind==='utc'?a===expected:close(a,expected)))diff(index,c,label,{actual:a,expected,reason:c.kind==='utc'?'exact UTC TimeClip':'numeric inverse'});}
function compare(index,c,actual){
    const count=report.counts;count.cases++;const accepted=c.checkedConstruction==='accept';count[accepted?'expectedAccepted':'expectedRejected']++;count[actual.accepted?'actualAccepted':'actualRejected']++;
    if(actual.accepted!==accepted)diff(index,c,'construction',{actual:actual.accepted,expected:accepted,error:actual.error});
    if(!actual.accepted){if(accepted)count.blockedOperations+=c.native.queries.length+c.native.inverse.length+c.native.branches.reduce((n,b)=>n+b.pixels.length,0);return;}
    if(!accepted)return; // Explicitly rejected profile; its unexpected construction is a recorded failure, and all actual outputs remain in the packet.
    sequence(index,c,'source',actual.source,c.domain,true);sequence(index,c,'display',actual.display,c.native.display);sequence(index,c,'range',actual.range,c.native.range);
    if(actual.invertible!==c.native.strictRangeInverse)diff(index,c,'inverse-policy',{actual:actual.invertible,expected:c.native.strictRangeInverse});
    assert.equal(actual.mapped.length,c.native.queries.length,'Probe map output inventory');
    for(let j=0;j<c.native.queries.length;j++){
        const query=c.native.queries[j],a=number(actual.mapped[j].value),raw=number(actual.mapped[j].unrounded),b=number(query.mapped),nativeRaw=number(query.unrounded);
        if(!query.checkedInput){count.checkedInputBranchRejects++;if(!Number.isNaN(a)||!Number.isNaN(raw))diff(index,c,`input-branch/${j}`,{input:query.input,actual:a,raw,expected:'NaN under checked input branch',nativeValue:b});continue;}
        count.checkedMaps++;if(!close(raw,nativeRaw))diff(index,c,`unrounded/${j}`,{input:query.input,actual:raw,expected:nativeRaw});
        if(!c.options.round){if(!close(a,b))diff(index,c,`map/${j}`,{input:query.input,actual:a,expected:b,raw,nativeRaw});continue;}
        if(Object.is(a,b)||Number.isNaN(a)&&Number.isNaN(b))continue;
        const half=Math.floor(nativeRaw)+.5,buffer=new ArrayBuffer(8),f=new Float64Array(buffer),u=new BigUint64Array(buffer);f[0]=Math.abs(half);u[0]++;const ulp=f[0]-Math.abs(half);
        const permitted=Number.isFinite(a)&&Number.isFinite(b)&&Math.abs(a-b)===1&&Math.abs(raw-half)<=ulp&&Math.abs(nativeRaw-half)<=ulp&&Object.is(a,Math.round(raw))&&Object.is(b,Math.round(nativeRaw));
        if(permitted)count.roundedHalfUlpDifferences++;else diff(index,c,`round/${j}`,{input:query.input,actual:a,expected:b,raw,nativeRaw,half,ulp,reason:'frozen exact/ONE-ULP same-half policy'});
    }
    assert.equal(actual.inverse.length,c.native.inverse.length,'Probe inverse output inventory');
    c.native.inverse.forEach((q,j)=>{count.globalInverses++;inverted(index,c,`inverse/${j}`,actual.inverse[j],number(q.value),c.native.strictRangeInverse);});
    assert.equal(actual.branches.length,c.native.branches.length,'Probe branch inventory');
    c.native.branches.forEach((branch,j)=>{assert.equal(actual.branches[j].index,branch.index);assert.equal(actual.branches[j].values.length,branch.pixels.length);branch.pixels.forEach((q,k)=>{count.branchInverses++;inverted(index,c,`branch-${branch.index}/${k}`,actual.branches[j].values[k],number(q.inverse),!branch.flat);});});
    count.ticks+=c.native.ticks.length;sequence(index,c,'ticks',actual.ticks,c.native.ticks,c.kind==='utc');
}
let browser;
try{
    browser=await chromium.launch({headless:true});report.browserVersion=browser.version();const context=await browser.newContext({viewport:{width:900,height:700},deviceScaleFactor:1});
    try{
        const page=await context.newPage();page.on('pageerror',e=>report.pageErrors.push(e.message));await page.goto(`http://127.0.0.1:${server.address().port}/`);
        await page.waitForFunction(()=>window.numericReady||window.numericBootError);const boot=await page.evaluate(()=>window.numericBootError);if(boot)throw Error(boot);
        report.userAgent=await page.evaluate(()=>navigator.userAgent);
        for(let start=0;start<fixture.cases.length;start+=128){
            const cases=fixture.cases.slice(start,start+128),payload=cases.map((c,i)=>({index:start+i,kind:c.kind,parameter:c.parameter,domain:c.domain,range:c.range,options:c.options,queries:c.native.queries.map(q=>q.input),inversePixels:c.native.inverse.map(q=>q.pixel),branches:c.native.branches.map(b=>({index:b.index,pixels:b.pixels.map(q=>q.pixel)}))}));
            const actualText=await page.evaluate(payload=>window.numericProbe(payload),JSON.stringify(payload));const actual=JSON.parse(actualText);assert.equal(actual.length,cases.length,'WASM case output inventory');
            const name=`packet-${String(start).padStart(5,'0')}.json`;await writeFile(join(out,name),actualText+'\n',{flag:'wx'});report.packets.push({start,count:cases.length,file:name,sha256:sha(actualText+'\n')});
            actual.forEach((a,i)=>{assert.equal(a.index,start+i,'WASM source case identity');compare(start+i,cases[i],a);});
        }
        assert.equal(report.counts.cases,9496);assert.equal(report.counts.expectedAccepted,8960);assert.equal(report.counts.expectedRejected,536);assert.equal(report.pageErrors.length,0,'Actual browser errors');
        report.passed=report.failures.length===0;
    }finally{await context.close();}
}catch(e){report.fatal=String(e.stack||e);}
finally{
    if(browser)await browser.close();await new Promise(done=>server.close(done));
    report.finishedAt=new Date().toISOString();await writeFile(join(out,'report.json'),stringify(report),{flag:'wx'});
    console.log(JSON.stringify({passed:report.passed,out,counts:report.counts,failures:report.failures.length,first:report.failures.slice(0,3),fatal:report.fatal}));
    if(!report.passed)process.exitCode=1;
}
