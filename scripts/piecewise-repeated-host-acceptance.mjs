// Repeated-domain consumers on actual SVG and Canvas browser hosts.
import assert from 'node:assert/strict';
import {readFile, writeFile, mkdir} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {createServer} from 'node:http';
import {execFileSync} from 'node:child_process';
import {resolve, relative, isAbsolute, extname, join} from 'node:path';
import {pathToFileURL} from 'node:url';
import {chromium, firefox, webkit} from 'playwright';

const metadata=JSON.parse(execFileSync('cargo',['metadata','--no-deps','--format-version','1'],{encoding:'utf8',windowsHide:true}));
const root=resolve('.'),out=resolve(process.argv[2]||join(metadata.target_directory,'piecewise-repeated-host'));
await mkdir(out,{recursive:true});
const sha=b=>createHash('sha256').update(b).digest('hex');
const reference=JSON.parse(await readFile('tests/fixtures/piecewise-repeated/reference.json'));
const modules=join(root,'node_modules');
for(const [file,hash] of Object.entries(reference.sourceSHA256))assert.equal(sha(await readFile(join(modules,file))),hash,file);
const d3=await import(pathToFileURL(join(modules,'d3-scale/src/index.js')));
const {Transform}=await import(pathToFileURL(join(modules,'d3-zoom/src/transform.js')));
const make=s=>{
  const f=s.kind==='symlog'?d3.scaleSymlog().constant(s.parameter):s.kind==='pow'?d3.scalePow().exponent(s.parameter):s.kind==='utc'?d3.scaleUtc():d3.scaleLinear();
  f.domain(s.domain).clamp(s.clamp);return s.round?f.rangeRound(s.range):f.range(s.range);
};
const near=(a,b,label)=>assert(Math.abs(a-b)<=2e-10*Math.max(1,Math.abs(b)),`${label}: ${a} != ${b}`);
const array=(a,b,label)=>{assert.equal(a.length,b.length,label);a.forEach((v,i)=>near(v,Number(b[i]),`${label}/${i}`));};
const check=(s,label)=>{
  for(const [axis,key,base,origin] of [['x','xScale','baseX',s.plot[0]],['y','yScale','baseY',s.plot[1]]]) {
    if(s.transform) {
      const k=s.transform[axis==='x'?'kx':'ky'],offset=s.transform[axis==='x'?'tx':'ty']+origin*(1-k);
      const t=new Transform(k,axis==='x'?offset:0,axis==='y'?offset:0),f=make(s[base]);
      const expected=(axis==='x'?t.rescaleX(f):t.rescaleY(f)).domain().map(Number);
      // UTC inversion applies Date TimeClip; its milliseconds are exact.
      if(s[key].kind==='utc')assert.deepEqual(s[key].domain,expected,`${label}/${axis}/TimeClip`);
      else array(s[key].domain,expected,`${label}/${axis}/all knots`);
    }
  }
  for(const site of s.sites)array(site.position,[make(s.xScale)(site.raw[0]),make(s.yScale)(site.raw[1])],`${label}/${site.id}/native pixels`);
  assert(s.sites.length>0,`${label}/no source sites`);
};
const server=createServer(async(req,res)=>{
  try {
    const path=resolve(root,'.'+decodeURIComponent(new URL(req.url,'http://localhost').pathname)),child=relative(root,path);
    if(child.startsWith('..')||isAbsolute(child))throw Error('Outside root');
    res.setHeader('Content-Type',{'.html':'text/html','.js':'text/javascript','.wasm':'application/wasm'}[extname(path)]||'application/octet-stream');
    res.end(await readFile(path));
  }catch{res.writeHead(404);res.end();}
});
await new Promise(r=>server.listen(0,'127.0.0.1',r));
const engines=process.argv.includes('--chromium-only')?[['Chromium',chromium,{}]]:[['Chrome',chromium,{channel:'chrome'}],['Firefox',firefox,{}],['WebKit',webkit,{}],['Edge',chromium,{channel:'msedge'}]];
const report={passed:false,scope:'Repeated-domain native D3 mapping, full-stop relayout and reset, SVG/Canvas paint, trusted pointer and keyboard; no physical compositor byte-identity claim',contexts:[]};
try {
  for(const [name,engine,options] of engines) {
    const browser=await engine.launch({headless:true,...options});
    try {for(const dpr of [1,2]) {
      const context=await browser.newContext({deviceScaleFactor:dpr,viewport:{width:1400,height:1000}}),page=await context.newPage();
      const result={name,dpr,passed:false,frames:[],picks:[],collapsed:[],rejected:[]};report.contexts.push(result);
      try {
        const errors=[];page.on('pageerror',e=>errors.push(e.message));
        await page.goto(`http://127.0.0.1:${server.address().port}/examples/browser-host/index.html`);
        await page.waitForFunction(()=>window.hostReady===true);
        await page.evaluate(()=>{
          document.querySelector('.panels').style.display='none';window.__repeatPackets=[];
          for(const type of ['pointermove','keydown'])document.addEventListener(type,e=>{if(e.target.closest?.('[id^="piecewise-scales-"]'))window.__repeatPackets.push({type,trusted:e.isTrusted,key:e.key});},true);
        });
        const flush=async()=>{await page.evaluate(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))));await page.evaluate(()=>window.hostFixture.piecewise_scales_flush());};
        const state=r=>page.evaluate(r=>JSON.parse(window.hostFixture.piecewise_scales_snapshot(r)),r);
        const surface=r=>page.locator(`#piecewise-scales-${r} .kartis-surface`);
        const frame=async(label)=>{
          const svg=await state('svg'),canvas=await state('canvas');check(svg,`${label}/svg`);check(canvas,`${label}/canvas`);
          assert.deepEqual(svg.sites,canvas.sites,`${label}/renderer sites`);
          const pixels=await page.evaluate(async()=>{
            const live=document.querySelector('#piecewise-scales-svg svg'),canvas=document.querySelector('#piecewise-scales-canvas canvas'),copy=live.cloneNode(true);
            // An SVG image document does not inherit the live HTML language.
            // Preserve that actual font-selection input when sampling it.
            let lang=null;
            for(let node=live;node;node=node.parentElement) {
              const xml=node.getAttributeNS('http://www.w3.org/XML/1998/namespace','lang')??node.getAttribute('xml:lang');
              if(xml!==null){lang=xml;break;}
              if(node.hasAttribute('lang')){lang=node.getAttribute('lang');break;}
            }
            lang??=document.documentElement.lang;
            copy.setAttribute('lang',lang);copy.setAttributeNS('http://www.w3.org/XML/1998/namespace','xml:lang',lang);
            copy.setAttribute('width',canvas.width/devicePixelRatio);copy.setAttribute('height',canvas.height/devicePixelRatio);
            const image=new Image(),url=URL.createObjectURL(new Blob([new XMLSerializer().serializeToString(copy)],{type:'image/svg+xml'}));
            try {
              image.src=url;await image.decode();const c=document.createElement('canvas');c.width=canvas.width;c.height=canvas.height;c.getContext('2d').drawImage(image,0,0,c.width,c.height);
              const a=c.getContext('2d').getImageData(0,0,c.width,c.height).data,b=canvas.getContext('2d').getImageData(0,0,c.width,c.height).data;
              let delta=0,ink=0;for(let i=0;i<a.length;i++)delta+=Math.abs(a[i]-b[i]);for(let i=0;i<b.length;i+=4)ink+=Number(Math.min(b[i],b[i+1],b[i+2])<220);
              const mean=delta/a.length;
              return {mean,ink,backing:[canvas.width,canvas.height],assets:mean>=.5?{svg:new XMLSerializer().serializeToString(copy),svgPNG:c.toDataURL(),canvasPNG:canvas.toDataURL()}:null};
            }finally{URL.revokeObjectURL(url);}
          });
          if(pixels.assets) {
            const prefix=join(out,`${name}-${dpr}-${label.replaceAll('/','-')}`);
            await writeFile(prefix+'-states.json',JSON.stringify({svg,canvas},null,2));
            await writeFile(prefix+'.svg',pixels.assets.svg);
            for(const key of ['svgPNG','canvasPNG'])await writeFile(prefix+`-${key}.png`,Buffer.from(pixels.assets[key].split(',')[1],'base64'));
            delete pixels.assets;
          }
          assert(pixels.mean<.5&&pixels.ink>500,`${label}/actual paint ${JSON.stringify(pixels)}`);
          assert.deepEqual(pixels.backing,[Math.round(canvas.width*dpr),Math.round(canvas.height*dpr)]);
          result.frames.push({label,sites:svg.sites.length,domain:svg.xScale.domain,pixels});return svg;
        };
        for(const c of [7,8,9,10,11,12,13,14])for(const scatter of [false,true]) {
          await page.evaluate(({c,scatter})=>window.hostFixture.piecewise_scales_mount(c,scatter,0),{c,scatter});await flush();
          const initial=await frame(`${c}/${scatter}/initial`);
          if(c===7)assert(initial.sites.some(site=>site.raw[0]===-1),'Projected repeated-edge source was dropped');
          assert.equal(initial.viewportRejected,[10,11].includes(c));
          for(const r of ['svg','canvas']) {
            const s=await state(r);assert(s.target,`${c}/${scatter}/${r}/no actual ink target`);
            await surface(r).scrollIntoViewIfNeeded();await flush();const b=await surface(r).boundingBox(),p=s.target.point;
            await page.mouse.move(b.x+p[0]*b.width/s.width,b.y+p[1]*b.height/s.height);await flush();
            assert.deepEqual((await state(r)).hovered,[s.target.id],`${c}/${scatter}/${r}/pointer identity`);
            const control=page.locator(`#piecewise-scales-${r} .kartis-datum`).first();await control.focus();await control.press('Enter');await flush();
            assert((await state(r)).selected,`${c}/${scatter}/${r}/keyboard selection`);
            result.picks.push({c,scatter,r,pointer:true,keyboard:true});
          }
          if(c===7) {
            await page.evaluate(()=>window.hostFixture.piecewise_scales_window(.1,.3,false));await flush();
            const collapsed=await frame(`${c}/${scatter}/collapsed`);assert(collapsed.xScale.domain.every(v=>v===0));
            result.collapsed.push({scatter,domain:collapsed.xScale.domain});
            await page.evaluate(()=>window.hostFixture.piecewise_scales_reset());await flush();
            assert.deepEqual((await frame(`${c}/${scatter}/reset`)).xScale.domain,initial.xScale.domain);
          }
          if(![10,11].includes(c)) {
            await page.evaluate(()=>window.hostFixture.piecewise_scales_reject(true));
            const rollback=await page.evaluate(()=>{
              const read=r=>JSON.parse(window.hostFixture.piecewise_scales_snapshot(r));
              const snapshot=r=>{const s=read(r);return ['xScale','yScale','sites','view','transform','stops','hidden','selected','hovered'].map(k=>s[k]);},paint=r=>document.querySelector(`#piecewise-scales-${r} ${r==='svg'?'svg':'canvas'}`),bytes=r=>r==='svg'?paint(r).outerHTML:paint(r).toDataURL();
              const capture=()=>JSON.stringify(['svg','canvas'].map(r=>[snapshot(r),bytes(r)]));
              const beforeIdle=['svg','canvas'].map(read);
              window.hostFixture.piecewise_scales_flush();
              const before=['svg','canvas'].map(read),baseline=capture();
              window.hostFixture.piecewise_scales_window(.2,.5,false);
              window.hostFixture.piecewise_scales_flush();
              const after=['svg','canvas'].map(read);
              return {same:baseline===capture(),revisionDelta:after.map((s,i)=>s.revision-before[i].revision),idleRevisionDelta:before.map((s,i)=>s.revision-beforeIdle[i].revision),refused:after.every((s,i)=>s.calls>before[i].calls&&s.events.some(e=>e.event?.includes('piecewise fixture layout rejection')))};
            });
            assert(rollback.refused&&rollback.same,`${c}/${scatter}/rejection changed accepted state or paint ${JSON.stringify(rollback)}`);
            assert.deepEqual(rollback.revisionDelta,rollback.idleRevisionDelta,'Explicit flush paint telemetry differs from idle control');
            await page.evaluate(()=>window.hostFixture.piecewise_scales_reject(false));await flush();
            const accepted=await state('svg');assert.deepEqual(accepted.xScale.domain,initial.xScale.domain);
            result.rejected.push({c,scatter,refused:true,acceptedStateAndPaintUnchanged:rollback.same,revisionDelta:rollback.revisionDelta,idleRevisionDelta:rollback.idleRevisionDelta});
          }
          await page.evaluate(()=>window.hostFixture.piecewise_scales_phase(1));await flush();await frame(`${c}/${scatter}/reversed-clamped-rounded`);
          await page.evaluate(()=>window.hostFixture.piecewise_scales_stop());await flush();assert.equal(await page.locator('[id^="piecewise-scales-"]').count(),0);
        }
        const packets=await page.evaluate(()=>window.__repeatPackets);assert(packets.some(p=>p.type==='pointermove'&&p.trusted)&&packets.some(p=>p.type==='keydown'&&p.key==='Enter'&&p.trusted));
        assert.deepEqual(errors,[]);result.passed=true;console.log(`${name} DPR${dpr}: ${result.frames.length} paint frames, ${result.picks.length} picks`);
      }catch(error){result.error=error.stack;throw error;}finally{await context.close();}
    }}finally{await browser.close();}
  }
  report.passed=report.contexts.length===engines.length*2&&report.contexts.every(c=>c.passed);
}finally{
  server.close();report.scriptSha256=sha(await readFile(new URL(import.meta.url)));report.wasmSha256=sha(await readFile('examples/browser-host/pkg/host_bg.wasm'));
  await writeFile(join(out,'report.json'),JSON.stringify(report,null,2));
}
console.log(JSON.stringify({passed:report.passed,contexts:report.contexts.length,output:join(out,'report.json')}));
