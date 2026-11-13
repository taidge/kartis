// Capacity + OS process counters in a separate run, never a frame-rate test.
import {chromium} from 'playwright';
import {readFile, writeFile} from 'node:fs/promises';
import {execFile} from 'node:child_process';
import {promisify} from 'node:util';
import {isAbsolute} from 'node:path';
import {createHash} from 'node:crypto';
const run=promisify(execFile),args=process.argv.slice(2);
const option=(name,fallback)=>{const i=args.indexOf(name);return i<0?fallback:args[i+1];};
const output=option('--output',null),url=option('--url','http://127.0.0.1:8814');
const cycles=Number(option('--cycles','2'));
const collectGarbage=args.includes('--collect-garbage');
if(!output||!isAbsolute(output))throw Error('Pass --output with an absolute path under the actual workspace target');
if(!Number.isInteger(cycles)||cycles<1||cycles>3)throw Error('--cycles must be 1..3');
const channel=option('--channel',process.env.PLAYWRIGHT_CHANNEL);
const browser=await chromium.launch({headless:true,...(channel?{channel}:{})});
const report={passed:false,startedAt:new Date().toISOString(),browser:browser.version(),channel:channel||'bundled',platform:process.platform,cycles,url,collectGarbage,
  semantics:{wasmLinearMemoryBytes:'Actual exported WebAssembly.Memory.buffer.byteLength, 64KiB pages. Capacity includes live objects, free allocator space and runtime; it does not shrink when objects are dropped.',
    browserProcessWorkingSet:'Per-process OS resident working set; sum double-counts shared pages and is not unique physical RAM. Process IDs come from this owned browser CDP session.',
    privateCommit:'Windows private committed virtual bytes, not resident bytes. Linux reports RSS/HWM with private commit null.',
    peak:'OS lifetime peak for each individual process; not a simultaneous browser-tree peak or per-operation peak.',
    limitations:'Synchronous external memory sampling is separate from throughput. No live Rust-heap, allocator-count, GPU allocation, compositor FPS, leak-free or physical device claim.'},snapshots:[],pageErrors:[]};
async function processCounters(processes){
  if(process.platform==='win32'){
    const ids=processes.map(p=>p.id);
    if(!ids.length||ids.some(id=>!Number.isSafeInteger(id)||id<=0))throw Error('CDP returned invalid owned process IDs');
    // Only validated numeric IDs enter this fixed PowerShell source. No shell
    // quoting, path concatenation, filesystem mutation or global cache changes.
    const script=`$rows=@();foreach($probeId in @(${ids.join(',')})){try{$p=[System.Diagnostics.Process]::GetProcessById($probeId);$p.Refresh();$rows+=[pscustomobject]@{pid=$probeId;workingSetBytes=$p.WorkingSet64;peakWorkingSetBytes=$p.PeakWorkingSet64;privateCommitBytes=$p.PrivateMemorySize64};$p.Dispose()}catch{$rows+=[pscustomobject]@{pid=$probeId;unavailable=$_.Exception.Message}}};ConvertTo-Json -Compress -InputObject @($rows)`;
    const {stdout}=await run('pwsh',['-NoLogo','-NoProfile','-NonInteractive','-Command',script],{windowsHide:true,timeout:10000,maxBuffer:1048576});
    return JSON.parse(stdout.replace(/^\uFEFF/,'')).map(row=>({...row,type:processes.find(p=>p.id===row.pid).type}));
  }
  if(process.platform==='linux')return Promise.all(processes.map(async p=>{
    try{const text=await readFile(`/proc/${p.id}/status`,'utf8'),kb=key=>Number(text.match(new RegExp(`^${key}:\\s+(\\d+)`,'m'))?.[1])*1024;
      return{pid:p.id,type:p.type,workingSetBytes:kb('VmRSS'),peakWorkingSetBytes:kb('VmHWM'),privateCommitBytes:null};
    }catch(error){return{pid:p.id,type:p.type,unavailable:error.message};}
  }));
  return processes.map(p=>({pid:p.id,type:p.type,unavailable:'OS counter backend is implemented only for Windows and Linux'}));
}
try{
  const context=await browser.newContext({viewport:{width:1280,height:1000},deviceScaleFactor:2});
  const page=await context.newPage();page.on('pageerror',error=>report.pageErrors.push(error.message));
  let moduleCapture=null;
  page.on('response',response=>{
    if(new URL(response.url()).pathname.endsWith('/host_bg.wasm'))moduleCapture=response.body().then(bytes=>({url:response.url(),bytes:bytes.length,sha256:createHash('sha256').update(bytes).digest('hex')}));
  });
  const cdp=await browser.newBrowserCDPSession();
  const pageCdp=collectGarbage?await context.newCDPSession(page):null;
  const frames=()=>page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
  await page.goto(url);await page.waitForFunction(()=>window.hostReady===true);
  if(!moduleCapture)throw Error('The actual initialized fixture WASM response was not observed');
  report.wasmModule=await moduleCapture;
  await page.emulateMedia({reducedMotion:'reduce'});
  await page.locator('.panels').evaluate(el=>{el.style.display='none';});await frames();
  const baselineDom=await page.locator('*').count();
  const sample=async(stage,cycle=0,renderer=null,count=0)=>{
    const counters=await page.evaluate(()=>{
      const memory=window.hostWasmMemory;
      if(!(memory instanceof WebAssembly.Memory))throw Error('Rebuild/reload fixture with its explicit exported WASM memory observation');
      const canvas=document.querySelector('#density-root canvas');
      return{wasmLinearMemoryBytes:memory.buffer.byteLength,domNodes:document.querySelectorAll('*').length,
        densityPaths:document.querySelectorAll('#density-root svg path[data-id]').length,
        densityCanvases:document.querySelectorAll('#density-root canvas').length,
        canvasRgbaBackingBytes:canvas?canvas.width*canvas.height*4:0,
        jsHeap:performance.memory?{used:performance.memory.usedJSHeapSize,total:performance.memory.totalJSHeapSize,limit:performance.memory.jsHeapSizeLimit}:null,
        density:JSON.parse(window.hostFixture.density_snapshot())};
    });
    if(counters.wasmLinearMemoryBytes<=0||counters.wasmLinearMemoryBytes%65536!==0)throw Error('Invalid actual WASM capacity');
    if(stage==='mounted'||stage==='changed'){
      if(renderer==='svg'&&counters.densityPaths!==count)throw Error('Measured SVG shape count differs from source');
      if(renderer==='canvas'&&(counters.densityCanvases!==1||counters.canvasRgbaBackingBytes!==1280*720*4))throw Error('Measured Canvas backing is not the actual fixed DPR=2 surface');
    }
    const processes=(await cdp.send('SystemInfo.getProcessInfo')).processInfo;
    const os=await processCounters(processes);
    if(['win32','linux'].includes(process.platform)&&!os.some(p=>p.workingSetBytes>0))throw Error('OS process counter sampling returned no live process');
    report.snapshots.push({stage,cycle,renderer,count,sampledAt:new Date().toISOString(),...counters,processes:os,
      sumReportedWorkingSetBytes:os.reduce((s,p)=>s+(p.workingSetBytes||0),0),
      sumReportedPrivateCommitBytes:process.platform==='win32'?os.reduce((s,p)=>s+(p.privateCommitBytes||0),0):null});
  };
  await sample('baseline');
  for(let cycle=1;cycle<=cycles;cycle++)for(const renderer of ['svg','canvas'])for(const count of [1000,5000,10000,20000]){
    await page.evaluate(({renderer,count})=>window.hostFixture.density_mount(renderer,count),{renderer,count});await frames();
    const state=await page.evaluate(()=>JSON.parse(window.hostFixture.density_snapshot()));
    if(!state.active||state.count!==count||state.pending!==0)throw Error('Initial density frame was not accepted');
    await sample('mounted',cycle,renderer,count);
    await page.evaluate(async()=>{for(let phase=1;phase<=8;phase++){
      window.hostFixture.density_update(phase);window.hostFixture.density_flush();
      await new Promise(resolve=>requestAnimationFrame(resolve));
    }});
    const current=await page.evaluate(()=>JSON.parse(window.hostFixture.density_snapshot()));
    if(current.phase!==8||current.pending!==0)throw Error('Measured changes did not publish');
    await sample('changed',cycle,renderer,count);
    await page.evaluate(()=>window.hostFixture.density_stop());await frames();
    if(await page.locator('*').count()!==baselineDom)throw Error('Density stop did not return DOM to baseline');
    await sample('stopped',cycle,renderer,count);
    if(pageCdp){
      // Diagnostic only, after the ordinary stop sample. Never present this as
      // the application's natural GC policy or mix it into throughput samples.
      await pageCdp.send('HeapProfiler.collectGarbage');await frames();
      await sample('post-explicit-gc',cycle,renderer,count);
    }
  }
  if(report.pageErrors.length)throw Error(report.pageErrors.join('\n'));
  report.passed=true;
}catch(error){report.error=error.stack;process.exitCode=1;}
finally{await browser.close();report.finishedAt=new Date().toISOString();await writeFile(output,JSON.stringify(report,null,2));}
console.log(JSON.stringify({passed:report.passed,browser:report.browser,snapshots:report.snapshots.length,output,error:report.error},null,2));
