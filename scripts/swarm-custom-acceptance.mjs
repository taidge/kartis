// Shared custom Swarm ink, native CPU colors and actual BrowserChart input.
import assert from 'node:assert/strict';
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {createServer} from 'node:http';
import {execFileSync} from 'node:child_process';
import {resolve,relative,isAbsolute,extname,join} from 'node:path';
import {chromium,firefox,webkit} from 'playwright';

const metadata=JSON.parse(execFileSync('cargo',['metadata','--no-deps','--format-version','1'],{encoding:'utf8',windowsHide:true}));
const root=resolve('.'),exports=join(metadata.target_directory,'swarm-custom-export');
const out=resolve(process.argv.slice(2).find(x=>!x.startsWith('--'))||join(metadata.target_directory,'swarm-custom-acceptance'));
await mkdir(out,{recursive:true});
const frames=JSON.parse(await readFile(join(exports,'frames.json'),'utf8'));
assert.equal(frames.length,8);
const sha=b=>createHash('sha256').update(b).digest('hex');
const report={passed:false,scope:'Shared checked geometry after deterministic packing; actual SVG/Canvas vs CPU interior samples at <=1 RGBA channel, trusted mouse/keyboard, atomic factory rejection, source update, resize and stop. No force-layout positional or arbitrary component parity claim.',sourceSHA256:{},contexts:[]};
for(const file of ['src/swarm_layout/custom.rs','examples/shared/swarm_custom.rs','examples/browser-host/src/browser/swarm_custom.rs','scripts/swarm-custom-acceptance.mjs','examples/browser-host/pkg/host_bg.wasm'])report.sourceSHA256[file]=sha(await readFile(file));
const server=createServer(async(req,res)=>{
  try {
    const path=decodeURIComponent(new URL(req.url,'http://localhost').pathname),base=path.startsWith('/export/')?exports:root;
    const file=resolve(base,'.'+(path.startsWith('/export/')?path.slice('/export'.length):path));
    const child=relative(base,file);if(child.startsWith('..')||isAbsolute(child))throw Error('Outside root');
    res.setHeader('Content-Type',{'.html':'text/html','.svg':'image/svg+xml','.js':'text/javascript','.wasm':'application/wasm','.png':'image/png','.json':'application/json'}[extname(file)]||'application/octet-stream');
    res.end(await readFile(file));
  }catch{res.writeHead(404);res.end();}
});
await new Promise(r=>server.listen(0,'127.0.0.1',r));
const base=`http://127.0.0.1:${server.address().port}`;
const engines=process.argv.includes('--chromium-only')?[['Chromium',chromium,{}]]:[['Chrome',chromium,{channel:'chrome'}],['Firefox',firefox,{}],['WebKit',webkit,{}],['Edge',chromium,{channel:'msedge'}]];
try {
 for(const [name,engine,options] of engines) {
  const browser=await engine.launch({headless:true,...options});
  try { for(const dpr of [1,2]) {
   const context=await browser.newContext({deviceScaleFactor:dpr,viewport:{width:1400,height:1000}}),page=await context.newPage();
   const result={name,dpr,passed:false,exports:[],hostFrames:[],picks:[],keyboard:[],rejects:[],retirements:[]};report.contexts.push(result);
   try {
    const errors=[];page.on('pageerror',e=>errors.push(e.message));
    await page.goto(base+'/export/index.html');
    result.exports=await page.evaluate(async({frames,dpr})=>{
      const receipts=[];
      for(const frame of frames) {
        const native=document.getElementById(`canvas-${frame.name}`),image=new Image();image.src=`${frame.name}.svg`;await image.decode();
        const svg=document.createElement('canvas');svg.width=frame.width*dpr;svg.height=frame.height*dpr;svg.getContext('2d').drawImage(image,0,0,svg.width,svg.height);
        let comparisons=0;
        for(const [renderer,canvas] of [['svg',svg],['canvas',native]]) {
          if(canvas.width!==frame.width*dpr||canvas.height!==frame.height*dpr)throw Error('Backing scale mismatch');
          for(const sample of frame.samples.filter(s=>s.dpr===dpr)) {
            const actual=[...canvas.getContext('2d').getImageData(Math.floor(sample.point[0]*dpr),Math.floor(sample.point[1]*dpr),1,1).data];
            if(actual.some((v,i)=>Math.abs(v-sample.rgba[i])>1))throw Error(JSON.stringify({frame:frame.name,renderer,sample,actual}));
            comparisons++;
          }
        }
        receipts.push({name:frame.name,comparisons});
      }
      return receipts;
    },{frames,dpr});
    await page.goto(base+'/examples/browser-host/index.html');await page.waitForFunction(()=>window.hostReady===true);
    await page.evaluate(()=>{
      document.querySelector('.panels').style.display='none';window.__swarmPackets=[];
      for(const type of ['pointermove','keydown'])document.addEventListener(type,e=>{if(e.target.closest?.('[id^="swarm-custom-"]'))window.__swarmPackets.push({type,key:e.key,trusted:e.isTrusted});},true);
    });
    const flush=async()=>{await page.evaluate(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))));await page.evaluate(()=>window.hostFixture.swarm_custom_flush());};
    const state=r=>page.evaluate(r=>JSON.parse(window.hostFixture.swarm_custom_snapshot(r)),r);
    const surface=r=>page.locator(`#swarm-custom-${r} .kartis-surface`);
    const move=async(r,p)=>{
      await surface(r).scrollIntoViewIfNeeded();const s=await state(r),b=await surface(r).boundingBox();
      const client=[b.x+p[0]*b.width/s.width,b.y+p[1]*b.height/s.height].map(Math.round);
      const before=await page.evaluate(()=>window.__swarmPackets.length);
      await page.mouse.move(client[0]+1,client[1]+1);await page.mouse.move(...client);await flush();
      const packets=await page.evaluate(n=>window.__swarmPackets.slice(n),before);
      assert(packets.some(p=>p.type==='pointermove'&&p.trusted),'Expected trusted pointer packet');
      return state(r);
    };
    const paint=async(label)=>{
      const a=await state('svg'),b=await state('canvas');assert.equal(a.pending,0,'SVG input queue drained');assert.equal(b.pending,0,'Canvas input queue drained');assert.deepEqual(a.sites,b.sites);assert.deepEqual(a.metadata,b.metadata);
      const frame=frames.find(f=>f.width===a.width&&f.phase===a.phase&&f.vertical===a.vertical);assert(frame,`Missing CPU frame ${JSON.stringify(a)}`);
      assert.deepEqual(a.sites.map(s=>({id:s.id,node:s.node,position:s.position,inkRadius:s.inkRadius})),frame.sites);
      const samples=frame.samples.filter(s=>s.dpr===dpr);
      const painted=await page.evaluate(async({samples,dpr,width,height})=>{
        const live=document.querySelector('#swarm-custom-svg svg'),canvas=document.querySelector('#swarm-custom-canvas canvas'),copy=live.cloneNode(true);
        let lang=null;for(let n=live;n;n=n.parentElement){const x=n.getAttributeNS('http://www.w3.org/XML/1998/namespace','lang')??n.getAttribute('xml:lang');if(x!==null){lang=x;break;}if(n.hasAttribute('lang')){lang=n.getAttribute('lang');break;}}
        lang??=document.documentElement.lang;copy.setAttribute('lang',lang);copy.setAttributeNS('http://www.w3.org/XML/1998/namespace','xml:lang',lang);copy.setAttribute('width',width);copy.setAttribute('height',height);
        const image=new Image(),url=URL.createObjectURL(new Blob([new XMLSerializer().serializeToString(copy)],{type:'image/svg+xml'}));
        try{
          image.src=url;await image.decode();const svg=document.createElement('canvas');svg.width=width*dpr;svg.height=height*dpr;svg.getContext('2d').drawImage(image,0,0,svg.width,svg.height);
          const receipts=[];
          for(const [renderer,c] of [['svg',svg],['canvas',canvas]]) {
            if(c.width!==width*dpr||c.height!==height*dpr)throw Error(`Backing mismatch ${renderer}: ${c.width},${c.height}`);
            for(const sample of samples){const actual=[...c.getContext('2d').getImageData(Math.floor(sample.point[0]*dpr),Math.floor(sample.point[1]*dpr),1,1).data];if(actual.some((v,i)=>Math.abs(v-sample.rgba[i])>1))throw Error(JSON.stringify({renderer,sample,actual}));receipts.push({renderer,label:sample.label,actual});}
          }
          return receipts;
        }finally{URL.revokeObjectURL(url);}
      },{samples,dpr,width:a.width,height:a.height});
      result.hostFrames.push({label,phase:a.phase,width:a.width,vertical:a.vertical,comparisons:painted.length,painted});
      assert.equal(a.metadata.filter(m=>m.hidden).length,1);assert.equal(a.metadata.filter(m=>m.node==='culled'&&m.site===null).length,1);
      assert(!a.sites.some(s=>s.node==='hidden'||s.node==='culled'));
    };
    for(const vertical of [false,true]) {for(const nearest of [false,true]) {
      await page.evaluate(({vertical,nearest})=>window.hostFixture.swarm_custom_mount(vertical,nearest),{vertical,nearest});await flush();await paint(`initial-${vertical}-${nearest}`);
      for(const renderer of ['svg','canvas']) {
        const s=await state(renderer),target=s.sites.find(s=>s.node==='offset'),zero=s.sites.find(s=>s.node==='zero');
        const origin=await move(renderer,target.position);assert.deepEqual(origin.hovered,nearest?[target.id]:[],'Original anchor exact/nearest contract');
        const ink=await move(renderer,target.probe);assert.deepEqual(ink.hovered,nearest?[]:[target.id],'Offset ink exact/nearest contract');
        const noInk=await move(renderer,zero.position);assert.deepEqual(noInk.hovered,nearest?[zero.id]:[],'Zero node exact/nearest contract');
        result.picks.push({renderer,vertical,nearest,origin:origin.hovered,ink:ink.hovered,zero:noInk.hovered});
        const control=page.locator(`#swarm-custom-${renderer} .kartis-datum`).filter({hasText:'offset'}).first();
        await control.focus();const before=await page.evaluate(()=>window.__swarmPackets.length);await page.keyboard.press('Enter');await flush();
        const activated=await state(renderer),packets=await page.evaluate(n=>window.__swarmPackets.slice(n),before);
        assert(packets.some(p=>p.type==='keydown'&&p.key==='Enter'&&p.trusted));assert(activated.events.some(e=>e.type==='activated'&&e.id===target.id));
        result.keyboard.push({renderer,vertical,nearest,activated:target.id});
      }
      // BrowserChart.flush paints even an idle frame and increments revision.
      // Measure that idle control separately; the atomic rejection itself is
      // checked synchronously, before any additional flush or native event.
      await flush();
      const idleBefore=Object.fromEntries(await Promise.all(['svg','canvas'].map(async r=>[r,await state(r)])));
      await flush();
      const original=Object.fromEntries(await Promise.all(['svg','canvas'].map(async r=>[r,await state(r)])));
      const idleDelta=Object.fromEntries(['svg','canvas'].map(r=>[r,original[r].revision-idleBefore[r].revision]));
      assert(Object.values(idleDelta).every(v=>Number.isInteger(v)&&v>=1),'Explicit idle flush must paint');
      for(const invalid of [1,2]) {
        const rejected=await page.evaluate(({invalid})=>{
          const capture=()=>Object.fromEntries(['svg','canvas'].map(r=>{
            const root=document.getElementById(`swarm-custom-${r}`),paint=root.querySelector(r==='svg'?'svg':'canvas');
            return [r,{state:JSON.parse(window.hostFixture.swarm_custom_snapshot(r)),paint:r==='svg'?paint.outerHTML:paint.toDataURL(),controls:[...root.querySelectorAll('.kartis-datum')].map(n=>n.outerHTML),rootStyle:root.getAttribute('style')}];
          }));
          const before=capture();let error=null;try{window.hostFixture.swarm_custom_update(1,480,invalid);}catch(e){error=String(e);}
          return {error,before,after:capture()};
        },{invalid});assert(rejected.error,'Invalid factory must fail');
        assert.deepEqual(rejected.after,rejected.before,'Rejected factory must preserve synchronous revision, source, paint, controls and root style');
        await flush();
        const afterControls=await page.evaluate(()=>Object.fromEntries(['svg','canvas'].map(r=>{const root=document.getElementById(`swarm-custom-${r}`);return[r,{controls:[...root.querySelectorAll('.kartis-datum')].map(n=>n.outerHTML),rootStyle:root.getAttribute('style')}];})));
        for(const r of ['svg','canvas'])assert.deepEqual(afterControls[r],{controls:rejected.before[r].controls,rootStyle:rejected.before[r].rootStyle},'Post-flush accepted controls and root style remain unchanged');
        for(const r of ['svg','canvas']){const s=await state(r),delta=s.revision-rejected.after[r].state.revision;result.rejects.push({vertical,nearest,invalid,renderer:r,before:rejected.before[r].state,after:s,error:rejected.error,idleDelta:idleDelta[r],postFlushRevisionDelta:delta,synchronousSourcePaintControlsEqual:true});assert.deepEqual({...s,revision:rejected.after[r].state.revision},rejected.after[r].state,'Post-flush source and interaction must remain identical');assert(Number.isInteger(delta)&&delta>=1,'Explicit post-rejection flush paints independently of admission');assert.deepEqual(s.sites,original[r].sites);assert.deepEqual(s.metadata,original[r].metadata);assert.equal(s.phase,0);assert.equal(s.width,640);}
        await paint(`rejected-${vertical}-${nearest}-${invalid}`);
      }
      await page.evaluate(()=>window.hostFixture.swarm_custom_update(1,640,0));await flush();await paint(`updated-${vertical}-${nearest}`);
      for(const r of ['svg','canvas']){const s=await state(r);assert(s.sites.some(s=>s.node==='added'));assert.notDeepEqual(s.sites,original[r].sites);assert.equal(s.phase,1);}
      await page.evaluate(()=>window.hostFixture.swarm_custom_update(1,480,0));await flush();await paint(`resized-${vertical}-${nearest}`);
      for(const r of ['svg','canvas']){const s=await state(r);assert.equal(s.width,480);assert.equal(await page.locator(`#swarm-custom-${r} .kartis-datum`).count(),s.sites.length);}
      await page.evaluate(()=>window.hostFixture.swarm_custom_stop());await flush();assert.equal(await page.locator('#swarm-custom-svg,#swarm-custom-canvas').count(),0);result.retirements.push({vertical,nearest,stopped:true});
    }}
    assert.deepEqual(errors,[]);result.passed=true;
   }catch(error){result.error=String(error.stack||error);await page.screenshot({path:join(out,`${name}-${dpr}-failed.png`),fullPage:true});throw error;}
   finally{await context.close();await writeFile(join(out,'report.json'),JSON.stringify(report,null,2));}
  }}finally{await browser.close();}
 }
 report.passed=report.contexts.every(c=>c.passed)&&report.contexts.length===engines.length*2;
}finally{server.close();await writeFile(join(out,'report.json'),JSON.stringify(report,null,2));}
console.log(JSON.stringify({passed:report.passed,contexts:report.contexts.map(c=>({name:c.name,dpr:c.dpr,exports:c.exports.length,hostFrames:c.hostFrames.length,picks:c.picks.length,keyboard:c.keyboard.length,rejects:c.rejects.length}))},null,2));
