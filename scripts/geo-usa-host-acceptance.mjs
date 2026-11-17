// Actual native browser input, USA provenance, and editable SVG/Canvas text.
import assert from 'node:assert/strict';
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {createServer} from 'node:http';
import {resolve,relative,isAbsolute,extname,join} from 'node:path';
import {execFileSync} from 'node:child_process';
import {chromium,firefox,webkit} from 'playwright';

const metadata=JSON.parse(execFileSync('cargo',['metadata','--no-deps','--format-version','1'],{encoding:'utf8'}));
const root=resolve('.'),out=resolve(process.argv[2]||join(metadata.target_directory,'geo-usa-host-final'));
await mkdir(out,{recursive:true});
const sha=b=>createHash('sha256').update(b).digest('hex');
const bundle=await readFile('tests/fixtures/geo-inverse-d3-oracle.js','utf8');
const original=JSON.parse(await readFile('tests/fixtures/geo-inverse-d3-source.json','utf8'));
assert.equal(sha(bundle),original.bundle.sha256);
const server=createServer(async(req,res)=>{
  try {
    const path=resolve(root,'.'+decodeURIComponent(new URL(req.url,'http://localhost').pathname));
    const child=relative(root,path);if(child.startsWith('..')||isAbsolute(child))throw Error('Outside root');
    res.setHeader('Content-Type',{'.html':'text/html','.js':'text/javascript','.wasm':'application/wasm','.json':'application/json'}[extname(path)]||'application/octet-stream');
    res.end(await readFile(path));
  }catch{res.writeHead(404);res.end();}
});
await new Promise(r=>server.listen(0,'127.0.0.1',r));
const url=`http://127.0.0.1:${server.address().port}/examples/browser-host/index.html`;
const report={passed:false,originalD3Sha256:sha(bundle),contexts:[],scope:'Trusted pointer/wheel/pan/keyboard, actual browser host USA coordinates and editable native text clipping; no physical-device claim'};
report.firefoxInputDriverSha256=sha(await readFile('node_modules/playwright-core/lib/server/firefox/ffInput.js'));
const engines=process.argv.includes('--chromium-only')?[['Chromium',chromium,{}]]:[['Chrome',chromium,{channel:'chrome'}],['Firefox',firefox,{}],['WebKit',webkit,{}],['Edge',chromium,{channel:'msedge'}]];
const near=(a,b,t=1e-7)=>a?.length===b?.length&&a.every((v,i)=>Number.isFinite(v)&&Math.abs(v-b[i])<=t);
const floatUlp=value=>{const f=new Float32Array([Math.abs(value)]),u=new Uint32Array(f.buffer);u[0]++;return f[0]-Math.fround(Math.abs(value));};
try {
  for(const [name,engine,opts] of engines) {
    const browser=await engine.launch({headless:true,...opts});
    try {
      for(const dpr of [1,2]) {
        const ctx=await browser.newContext({deviceScaleFactor:dpr,viewport:{width:1400,height:1000}}),page=await ctx.newPage();
        page.setDefaultTimeout(30000);
        const result={browser:name,dpr,passed:false,queries:[],noneQueries:[],gestures:[],keyboard:[],labels:[],lines:[],retirements:[]};
        report.contexts.push(result);
        const errors=[];page.on('pageerror',e=>errors.push(e.message));
        const flush=async()=>{await page.evaluate(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))));await page.evaluate(()=>window.hostFixture.usa_flush());};
        const state=r=>page.evaluate(r=>JSON.parse(window.hostFixture.usa_snapshot(r)),r);
        const surface=r=>page.locator(`#usa-${r} .kartis-surface`);
        const stop=async()=>{
          await page.evaluate(()=>window.hostFixture.usa_stop());await flush();
          for(const r of ['svg','canvas']) {const s=await state(r);assert(!s.active&&s.stopped&&s.released,JSON.stringify(s));}
          assert.equal(await page.locator('#usa-svg,#usa-canvas').count(),0);
          result.retirements.push({stopped:true,released:true});
        };
        try {
          await page.goto(url);await page.waitForFunction(()=>window.hostReady===true);
          await page.evaluate(bundle=>{
            document.querySelector('.panels').style.display='none';
            const frame=document.createElement('iframe');frame.hidden=true;frame.id='usa-native-oracle';document.body.append(frame);
            const script=frame.contentDocument.createElement('script');script.textContent=bundle;frame.contentDocument.head.append(script);
            window.__usaOracle=({s,p,region})=>{
              const d3=frame.contentWindow.d3,k=s.scale,[x,y]=s.origin;
              const composite=d3.geoAlbersUsa().scale(k).translate([x,y]);
              const child=region==='Alaska'?d3.geoConicEqualArea().rotate([154,0]).center([-2,58.5]).parallels([55,65]).scale(.35*k).translate([x-.307*k,y+.201*k]):region==='Hawaii'?d3.geoConicEqualArea().rotate([157,0]).center([-3,19.9]).parallels([8,18]).scale(k).translate([x-.205*k,y+.212*k]):d3.geoAlbers().scale(k).translate([x,y]);
              const nx=(p[0]-x)/k,ny=(p[1]-y)/k;
              const selected=ny>=.120&&ny<.234&&nx>=-.425&&nx<-.214?'Alaska':ny>=.166&&ny<.234&&nx>=-.214&&nx<-.115?'Hawaii':'Lower48';
              return {known:child.invert(p),default:composite.invert(p),region:selected};
            };
            window.__usaPackets=[];
            for(const type of ['wheel','pointermove','pointerdown','pointerup','keydown'])document.addEventListener(type,e=>{
              const renderer=e.target.closest?.('#usa-svg,#usa-canvas,#usa-lines-svg,#usa-lines-canvas')?.id;
              if(renderer)window.__usaPackets.push({type,key:e.key,trusted:e.isTrusted,renderer,client:[e.clientX,e.clientY]});
            },true);
          },bundle);
          const query=async(r,probe,index,tag)=>{
            await surface(r).scrollIntoViewIfNeeded();await page.evaluate(i=>window.hostFixture.usa_region(i),index);
            const before=await state(r),b=await surface(r).boundingBox(),p=probe.point;
            const viewport=[before.plot[0]+(p[0]-before.plot[0])*before.view.scale+before.view.x,before.plot[1]+(p[1]-before.plot[1])*before.view.scale+before.view.y];
            const intendedClient=[b.x+viewport[0]/640*b.width,b.y+viewport[1]/400*b.height];
            // Request integer CSS pixels so Firefox's driver does not add a
            // separate flooring displacement to an already fractional request.
            const client=intendedClient.map(Math.round);
            await page.mouse.move(client[0]+1,client[1]+1);await page.mouse.move(...client);await flush();
            const s=await state(r),receipt=s.receipt;
            assert(!receipt.error&&receipt.trusted&&receipt.pointerType==='mouse'&&receipt.count>before.receipt.count,JSON.stringify({tag,s}));
            const paint=await page.locator(`#usa-${r} ${r==='svg'?'svg':'canvas'}`).first().evaluate((paint,{s,r})=>{
              const b=paint.getBoundingClientRect();
              const viewport=[(s.receipt.client[0]-b.x)*s.width/b.width,(s.receipt.client[1]-b.y)*s.height/b.height];
              const scene=[s.plot[0]+(viewport[0]-s.plot[0]-s.view.x)/s.view.scale,s.plot[1]+(viewport[1]-s.plot[1]-s.view.y)/s.view.scale];
              const rgba=r==='canvas'?[...paint.getContext('2d').getImageData(Math.floor(viewport[0]*paint.width/s.width),Math.floor(viewport[1]*paint.height/s.height),1,1).data]:null;
              return {scene,rgba,bounds:[b.x,b.y,b.width,b.height]};
            },{s,r});
            // The installed Playwright Firefox driver floors dispatch x/y.
            // This bounds only requested-vs-received input. The actual event's
            // CSS/Scene chain and both native D3 inverses remain at 1e-7.
            const clientTolerance=client.map((v,i)=>Math.abs(v-intendedClient[i])+(name==='Firefox'?1:0)+2*floatUlp(v));
            const inputTolerance=clientTolerance.map((v,i)=>v*[640/b.width,400/b.height][i]/before.view.scale+1e-7);
            assert(near(receipt.scene,paint.scene)&&receipt.scene.every((v,i)=>Math.abs(v-p[i])<=inputTolerance[i]),JSON.stringify({tag,receipt,paint,p,inputTolerance}));
            const native=await page.evaluate(({s,p,region})=>window.__usaOracle({s,p,region}),{s,p:receipt.scene,region:['Lower48','Alaska','Hawaii'][index]});
            const intended=await page.evaluate(({s,p,region})=>window.__usaOracle({s,p,region}),{s,p,region:['Lower48','Alaska','Hawaii'][index]});
            assert(near(receipt.known,native.known)&&near(intended.known,probe.source),JSON.stringify({tag,receipt,native,intended}));
            assert(near(receipt.default?.position,native.default)&&receipt.default.region===native.region&&near(receipt.general,native.default),JSON.stringify({tag,receipt,native}));
            if(tag!=='ambiguous') {
              assert(near(receipt.inPlot,native.default));
              if(r==='canvas')assert(paint.rgba[0]<50&&paint.rgba[1]>=75&&paint.rgba[1]<=105&&paint.rgba[2]>=140&&paint.rgba[2]<=180,JSON.stringify(paint));
              else {
                const id=`geo-symbol:7:place-${index}`;
                const ink=await page.locator(`#usa-${r} [data-id="${id}"]`).boundingBox();
                assert(ink&&Math.abs(ink.x+ink.width/2-receipt.client[0])<clientTolerance[0]+.1&&Math.abs(ink.y+ink.height/2-receipt.client[1])<clientTolerance[1]+.1,'SVG glyph does not coincide with native input');
              }
            }else assert(!near(receipt.known,receipt.default.position,.1),'Ambiguous composite inverse must retain distinct known-region provenance');
            assert.equal(await page.locator(`#usa-${r} output`).getAttribute('aria-live'),'polite');
            result.queries.push({tag,phase:s.phase,renderer:r,source:probe.source,requestedClient:client,intendedClient,receipt,native,intended,inputTolerance,clientTolerance,paint});
          };
          const noneQuery=async(r,p,index,defaultNone)=>{
            await surface(r).scrollIntoViewIfNeeded();await page.evaluate(i=>window.hostFixture.usa_region(i),index);
            const before=await state(r),b=await surface(r).boundingBox();
            const viewport=[before.plot[0]+(p[0]-before.plot[0])*before.view.scale+before.view.x,before.plot[1]+(p[1]-before.plot[1])*before.view.scale+before.view.y];
            const client=[Math.round(b.x+viewport[0]/640*b.width),Math.round(b.y+viewport[1]/400*b.height)];
            await page.mouse.move(...client);await flush();const s=await state(r),receipt=s.receipt;
            assert(!receipt.error&&receipt.trusted&&receipt.count>before.receipt.count);assert.equal(receipt.known,null);
            assert.equal(receipt.default===null,defaultNone);assert.equal(receipt.general===null,defaultNone);
            const actual=await page.locator(`#usa-${r} ${r==='svg'?'svg':'canvas'}`).first().evaluate((paint,s)=>{
              const b=paint.getBoundingClientRect(),v=[(s.receipt.client[0]-b.x)*640/b.width,(s.receipt.client[1]-b.y)*400/b.height];
              return [s.plot[0]+(v[0]-s.plot[0]-s.view.x)/s.view.scale,s.plot[1]+(v[1]-s.plot[1]-s.view.y)/s.view.scale];
            },s);
            assert(near(receipt.scene,actual));
            const rect=s.clips[index].rect;assert(!(actual[0]>=rect[0]&&actual[0]<=rect[0]+rect[2]&&actual[1]>=rect[1]&&actual[1]<=rect[1]+rect[3]));
            assert.equal(await page.locator(`#usa-${r} output`).textContent(),'Outside the selected region.');
            result.noneQueries.push({phase:s.phase,renderer:r,defaultNone,receipt,actual});
          };
          for(const phase of [0,1,2]) {
            await page.evaluate(p=>window.hostFixture.usa_mount(p),phase);await flush();
            for(const r of ['svg','canvas']) {
              let s=await state(r);
              if(phase===2)assert(s.fitCenter.some(v=>Math.abs(v)>.001));
              for(let i=0;i<3;i++)await query(r,s.probes[i],i,'base');
              if(phase<2)await query(r,s.probes[3],0,'ambiguous');
              const control=page.locator(`#usa-${r} .kartis-datum`).filter({hasText:'place-0'}).first();
              const before=await state(r),packetStart=await page.evaluate(()=>window.__usaPackets.length);
              await control.focus();await page.keyboard.press('Enter');await flush();
              const after=await state(r),packets=await page.evaluate(n=>window.__usaPackets.slice(n),packetStart);
              assert(packets.some(p=>p.type==='keydown'&&p.key==='Enter'&&p.trusted&&p.renderer===`usa-${r}`));
              assert(after.events.some(e=>e.type==='activated'&&e.id==='geo-symbol:7:place-0'));
              assert.deepEqual(after.receipt,before.receipt);
              result.keyboard.push({phase,renderer:r,trusted:true,activated:true,pointerReceiptUnchanged:true});
              const rejected=await page.evaluate(r=>{
                const frame=()=>{const p=document.querySelector(`#usa-${r} ${r==='svg'?'svg':'canvas'}`);return r==='svg'?p.outerHTML:p.toDataURL();};
                const before=JSON.parse(window.hostFixture.usa_snapshot(r)),paint=frame();
                const refused=window.hostFixture.usa_rejected();
                return {before,after:JSON.parse(window.hostFixture.usa_snapshot(r)),refused,paintUnchanged:paint===frame()};
              },r);
              assert(rejected.refused&&rejected.paintUnchanged);assert.deepEqual(rejected.after,rejected.before);
              if(phase===1) {
                await surface(r).scrollIntoViewIfNeeded();const box=await surface(r).boundingBox();
                const start=await page.evaluate(()=>window.__usaPackets.length);
                await page.mouse.move(box.x+box.width/2,box.y+box.height/2);await page.mouse.wheel(0,-100);await flush();
                const zoom=await state(r);assert(zoom.view.scale>1);
                await query(r,s.probes[1],1,'zoom');
                const panBefore=await state(r),panBox=await surface(r).boundingBox();
                const inputBounds=await surface(r).evaluate(e=>{const b=e.getBoundingClientRect();return {x:b.x,y:b.y,width:b.width,height:b.height};});
                await page.mouse.move(panBox.x+panBox.width/2,panBox.y+panBox.height/2);await page.mouse.down();await flush();
                await page.mouse.move(panBox.x+panBox.width/2+14,panBox.y+panBox.height/2+9,{steps:3});await flush();await page.mouse.up();await flush();
                const pan=await state(r);
                const packets=await page.evaluate(n=>window.__usaPackets.slice(n),start);
                for(const type of ['wheel','pointerdown','pointerup'])assert(packets.some(p=>p.type===type&&p.trusted&&p.renderer===`usa-${r}`));
                const down=packets.find(p=>p.type==='pointerdown'&&p.renderer===`usa-${r}`),up=packets.find(p=>p.type==='pointerup'&&p.renderer===`usa-${r}`);
                const delta=[(up.client[0]-down.client[0])*640/inputBounds.width,(up.client[1]-down.client[1])*400/inputBounds.height];
                assert(Math.abs(pan.view.x-panBefore.view.x-delta[0])<1e-7&&Math.abs(pan.view.y-panBefore.view.y-delta[1])<1e-7,JSON.stringify({panBefore,pan,delta}));
                await query(r,s.probes[2],2,'pan');
                result.gestures.push({renderer:r,zoom:zoom.view,pan:pan.view,delta,inputBounds,protocolBounds:panBox,packets});
                await page.evaluate(r=>window.hostFixture.usa_reset(r),r);await flush();
                await page.locator(`#usa-${r}`).evaluate(e=>{e.style.transformOrigin='0 0';e.style.transform='translate(13px,11px) scale(1.2,.8)';});
                await query(r,s.probes[0],0,'css');
                await page.locator(`#usa-${r}`).evaluate(e=>e.style.transform='');
              }
              await noneQuery(r,s.probes[0].point,2,false);
              if(phase===1)await noneQuery(r,[8,8],0,true);
            }
            await stop();
          }
          await page.evaluate(()=>{
            window.__usaTextCalls=[];window.__usaFillText=CanvasRenderingContext2D.prototype.fillText;
            CanvasRenderingContext2D.prototype.fillText=function(...args){window.__usaTextCalls.push({text:args[0],x:args[1],y:args[2],font:this.font});return Reflect.apply(window.__usaFillText,this,args);};
          });
          for(const masked of [false,true]) {
            await page.evaluate(m=>window.hostFixture.usa_labels_mount(m),masked);await flush();
            for(const r of ['svg','canvas']) {
              const pixels=await page.locator(`#usa-${r} ${r==='svg'?'svg':'canvas'}`).first().evaluate(async(paint,r)=>{
                let canvas=paint,nativeText=null;
                if(r==='svg') {
                  nativeText=[...paint.querySelectorAll('text')].map(t=>t.textContent);
                  const url=URL.createObjectURL(new Blob([new XMLSerializer().serializeToString(paint)],{type:'image/svg+xml'}));
                  try {const im=new Image();im.src=url;await im.decode();canvas=document.createElement('canvas');canvas.width=640*devicePixelRatio;canvas.height=400*devicePixelRatio;canvas.getContext('2d').drawImage(im,0,0,canvas.width,canvas.height);}finally{URL.revokeObjectURL(url);}
                }
                const rgba=canvas.getContext('2d').getImageData(0,0,canvas.width,canvas.height).data;
                let count=0;for(let i=0;i<rgba.length;i+=4)if(rgba[i]>190&&rgba[i+1]<65&&rgba[i+2]>110&&rgba[i+2]<200&&rgba[i+3]>100)count++;
                return {count,nativeText};
              },r);
              if(r==='svg')assert.deepEqual(pixels.nativeText,['HI <native & editable>']);
              if(masked)assert.equal(pixels.count,0);else assert(pixels.count>30,JSON.stringify(pixels));
              result.labels.push({renderer:r,masked,...pixels});
            }
            await stop();
          }
          const calls=await page.evaluate(()=>{CanvasRenderingContext2D.prototype.fillText=window.__usaFillText;return window.__usaTextCalls;});
          assert(calls.some(c=>c.text==='HI <native & editable>'));
          result.nativeCanvasTextCalls=calls;
          await page.evaluate(()=>window.hostFixture.usa_lines_mount());
          const lineFlush=async()=>{await page.evaluate(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))));await page.evaluate(()=>window.hostFixture.usa_lines_flush());};
          const lineState=r=>page.evaluate(r=>JSON.parse(window.hostFixture.usa_lines_snapshot(r)),r);
          await lineFlush();
          for(const phase of [0,1]) {
            if(phase){await page.evaluate(p=>window.hostFixture.usa_lines_phase(p),phase);await lineFlush();}
            for(const r of ['svg','canvas']) {
              const s=await lineState(r),paint=page.locator(`#usa-lines-${r} ${r==='svg'?'svg':'canvas'}`).first(),input=page.locator(`#usa-lines-${r} .kartis-surface`);
              assert.equal(s.controls,1);assert.equal(s.aliases,2);assert.equal(s.sourceIndex,1);assert.equal(s.sourceSeries,'');assert.equal(s.series,'route');
              assert.equal(await page.locator(`#usa-lines-${r} .kartis-datum`).count(),1);
              assert.equal(s.parts.length,3);
              const parts=[];
              for(const part of s.parts) {
                await input.scrollIntoViewIfNeeded();const b=await input.boundingBox();
                const packetStart=await page.evaluate(()=>window.__usaPackets.length);
                await page.mouse.move(Math.round(b.x+part.probe[0]/640*b.width),Math.round(b.y+part.probe[1]/400*b.height));await lineFlush();
                const active=await lineState(r),packets=await page.evaluate(n=>window.__usaPackets.slice(n),packetStart);
                assert(packets.some(p=>p.type==='pointermove'&&p.trusted&&p.renderer===`usa-lines-${r}`));
                assert.deepEqual(active.hovered,[s.id]);assert.deepEqual(active.interaction.ids,[s.id]);assert(near(active.interaction.anchor,s.anchor));
                assert.equal(active.interaction.rows.length,1);assert.equal(active.interaction.rows[0].value,phase===0?null:42);
                const tooltip=await page.locator(`#usa-lines-${r} .kartis-surface-tooltip`).textContent();
                assert(tooltip.includes(phase===0?'not supplied':'42'),tooltip);
                const ink=await paint.evaluate((p,{part,r})=>{
                  if(r==='canvas')return {rgba:[...p.getContext('2d').getImageData(Math.floor(part.probe[0]*p.width/640),Math.floor(part.probe[1]*p.height/400),1,1).data]};
                  const mark=p.querySelector(`[data-id="${part.id}"]`),clip=mark?.closest('svg[overflow="hidden"]');return {present:!!mark,path:mark?.getAttribute('d'),clipped:!!clip,clip:clip?['x','y','width','height'].map(a=>Number(clip.getAttribute(a))):null};
                },{part,r});
                if(r==='canvas')assert(ink.rgba[0]>180&&ink.rgba[1]<70&&ink.rgba[2]<110,JSON.stringify(ink));else assert(ink.present&&ink.path&&ink.clipped,JSON.stringify(ink));
                parts.push({region:part.region,physicalId:part.id,logicalId:s.id,anchor:active.interaction.anchor,value:active.interaction.rows[0].value,ink,trusted:true});
              }
              const control=page.locator(`#usa-lines-${r} .kartis-datum`);
              const beforeKey=await page.evaluate(()=>window.__usaPackets.length);
              await control.focus();await page.keyboard.press('Enter');await lineFlush();
              const keyed=await lineState(r),keys=await page.evaluate(n=>window.__usaPackets.slice(n),beforeKey);
              assert(keys.some(p=>p.type==='keydown'&&p.trusted&&p.key==='Enter'&&p.renderer===`usa-lines-${r}`));
              assert(keyed.events.some(e=>e.type==='activated'&&e.id===s.id));
              const rejection=await page.evaluate(r=>{
                const before=JSON.parse(window.hostFixture.usa_lines_snapshot(r));
                const mark=document.querySelector(`#usa-lines-${r} .kartis-datum`),paint=document.querySelector(`#usa-lines-${r} ${r==='svg'?'svg':'canvas'}`);
                const image=r==='svg'?paint.outerHTML:paint.toDataURL();
                const refused=window.hostFixture.usa_lines_rejected();
                return {refused,before,after:JSON.parse(window.hostFixture.usa_lines_snapshot(r)),sameControl:mark===document.querySelector(`#usa-lines-${r} .kartis-datum`),samePaint:image===(r==='svg'?paint.outerHTML:paint.toDataURL())};
              },r);
              assert(rejection.refused&&rejection.sameControl&&rejection.samePaint);assert.deepEqual(rejection.after,rejection.before);
              result.lines.push({phase,renderer:r,controls:s.controls,aliases:s.aliases,sourceIndex:s.sourceIndex,sourceSeries:s.sourceSeries,series:s.series,parts,trustedKeyboard:true,invalidAtomic:true});
            }
          }
          await page.evaluate(()=>window.hostFixture.usa_lines_stop());await lineFlush();assert.equal(await page.locator('#usa-lines-svg,#usa-lines-canvas').count(),0);
          assert.equal(result.lines.length,4);
          assert.equal(result.noneQueries.length,8);
          assert.equal(result.queries.length,28);assert.equal(result.keyboard.length,6);assert.equal(result.gestures.length,2);assert.equal(result.labels.length,4);
          assert.deepEqual(errors,[]);result.passed=true;
          console.log(`${name} DPR${dpr}: ${result.queries.length} native queries, text clips passed`);
        }catch(error){result.error=error.stack;throw error;}
        finally{await ctx.close();}
      }
    }finally{await browser.close();}
  }
  report.passed=report.contexts.length===engines.length*2&&report.contexts.every(c=>c.passed);
}finally{
  server.close();
  report.wasmSha256=sha(await readFile('examples/browser-host/pkg/host_bg.wasm'));
  report.scriptSha256=sha(await readFile(new URL(import.meta.url)));
  await writeFile(join(out,'report.json'),JSON.stringify(report,null,2));
}
console.log(JSON.stringify({passed:report.passed,contexts:report.contexts.length,output:join(out,'report.json')}));
