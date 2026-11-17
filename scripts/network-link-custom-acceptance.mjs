// Actual retained force-driver candidates, custom link paint and native input.
import assert from 'node:assert/strict';
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {createServer} from 'node:http';
import {execFileSync} from 'node:child_process';
import {resolve,relative,isAbsolute,extname,join} from 'node:path';
import {chromium,firefox,webkit} from 'playwright';

const root=resolve('.'),metadata=JSON.parse(execFileSync('cargo',['metadata','--offline','--no-deps','--format-version','1'],{encoding:'utf8',windowsHide:true}));
const out=resolve(process.argv[2]||join(metadata.target_directory,'network-link-custom-host'));await mkdir(out,{recursive:true});
const sha=x=>createHash('sha256').update(x).digest('hex');
const report={passed:false,scope:'Actual BrowserNetwork retained factory, current force endpoints, native SVG/Canvas paint, trusted node input, tick/resize/replacement, rejected candidates and callback lifetime. No arbitrary Widget or different-force positional equivalence claim.',contexts:[]};
const server=createServer(async(req,res)=>{try{
  if(!['GET','HEAD'].includes(req.method))throw Error('Read only');
  const file=resolve(root,'.'+decodeURIComponent(new URL(req.url,'http://localhost').pathname)),child=relative(root,file);
  if(child.startsWith('..')||isAbsolute(child))throw Error('Outside root');
  res.setHeader('Content-Type',{'.html':'text/html','.js':'text/javascript','.wasm':'application/wasm','.json':'application/json'}[extname(file)]||'application/octet-stream');res.end(await readFile(file));
}catch{res.writeHead(404);res.end();}});
await new Promise(r=>server.listen(0,'127.0.0.1',r));
const engines=process.argv.includes('--chromium-only')?[['Chromium',chromium,{}]]:[['Chrome',chromium,{channel:'chrome'}],['Firefox',firefox,{}],['WebKit',webkit,{}],['Edge',chromium,{channel:'msedge'}]];
const close=(a,b,t=1e-7)=>Math.abs(a-b)<=t;
try{for(const [name,engine,opts]of engines){const browser=await engine.launch({headless:true,...opts});try{for(const dpr of [1,2]){
  const ctx=await browser.newContext({viewport:{width:1400,height:1000},deviceScaleFactor:dpr}),page=await ctx.newPage();
  const record={name,dpr,passed:false,frames:[],gestures:[],rejections:[],lifecycle:[]};report.contexts.push(record);
  try{
    const errors=[];page.on('pageerror',e=>errors.push(e.message));
    await page.goto(`http://127.0.0.1:${server.address().port}/examples/browser-host/index.html`);await page.waitForFunction(()=>window.hostReady===true);
    await page.evaluate(()=>{document.querySelector('.panels').style.display='none';window.__networkPackets=[];for(const type of ['pointerdown','pointermove','pointerup','keydown'])window.addEventListener(type,e=>{const renderer=e.target.closest?.('[id^="network-link-custom-"]')?.id;if(renderer)window.__networkPackets.push({type,trusted:e.isTrusted,renderer,client:[e.clientX,e.clientY],key:e.key});},true);window.hostFixture.network_link_custom_mount();});
    const state=r=>page.evaluate(r=>JSON.parse(window.hostFixture.network_link_custom_snapshot(r)),r);
    const surface=r=>page.locator(`#network-link-custom-${r} .kartis-surface`);
    const flush=async()=>{await page.evaluate(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))));await page.evaluate(()=>window.hostFixture.network_link_custom_flush());};
    const client=async(r,s,p)=>{await surface(r).scrollIntoViewIfNeeded();const b=await surface(r).boundingBox();return[b.x+p[0]*b.width/s.width,b.y+p[1]*b.height/s.height];};
    const geometry=async(r)=>{
      const s=await state(r);assert(s.active&&s.nodes.length===3);const nodes=Object.fromEntries(s.nodes.map(n=>[n.id,n]));
      if(r==='svg'){
        const paths=await page.locator('#network-link-custom-svg [data-id^="force-link:"]').evaluateAll(paths=>paths.map(p=>({id:p.dataset.id,d:p.getAttribute('d')})));
        assert.equal(paths.length,4);
        const ends=[['a:wing','b'],['a:wing','b'],['b','c'],['c','c']];
        for(const path of paths){const i=Number(path.id.split(':').at(-1)),a=nodes[ends[i][0]].anchor,b=nodes[ends[i][1]].anchor,bend=i%2===0?-64:56;
          const expected=i===3?[...a,a[0]-48,a[1]-72,a[0]+48,a[1]-72,...a]:[...a,a[0],a[1]+bend,b[0],b[1]+bend,...b];
          const actual=path.d.match(/[-+]?(?:\d*\.)?\d+(?:e[-+]?\d+)?/gi).map(Number);
          assert.equal(actual.length,8);assert(actual.every((v,j)=>close(v,expected[j],1e-3)),JSON.stringify({path,actual,expected}));
        }
      }
      return s;
    };
    const frame=async(label)=>{
      const a=await geometry('svg'),b=await geometry('canvas');assert.deepEqual(a.nodes,b.nodes,`${label}/paired source`);
      const paint=await page.evaluate(async()=>{
        const live=document.querySelector('#network-link-custom-svg svg'),canvas=document.querySelector('#network-link-custom-canvas canvas'),copy=live.cloneNode(true);
        let lang=null;for(let n=live;n;n=n.parentElement){const x=n.getAttributeNS('http://www.w3.org/XML/1998/namespace','lang')??n.getAttribute('xml:lang');if(x!==null){lang=x;break;}if(n.hasAttribute('lang')){lang=n.getAttribute('lang');break;}}
        lang??=document.documentElement.lang;copy.setAttribute('lang',lang);copy.setAttributeNS('http://www.w3.org/XML/1998/namespace','xml:lang',lang);copy.setAttribute('width',canvas.width/devicePixelRatio);copy.setAttribute('height',canvas.height/devicePixelRatio);
        const image=new Image(),url=URL.createObjectURL(new Blob([new XMLSerializer().serializeToString(copy)],{type:'image/svg+xml'}));
        try{image.src=url;await image.decode();const c=document.createElement('canvas');c.width=canvas.width;c.height=canvas.height;c.getContext('2d').drawImage(image,0,0,c.width,c.height);const a=c.getContext('2d').getImageData(0,0,c.width,c.height).data,b=canvas.getContext('2d').getImageData(0,0,c.width,c.height).data;let delta=0,ink=0;for(let i=0;i<a.length;i++)delta+=Math.abs(a[i]-b[i]);for(let i=0;i<b.length;i+=4)ink+=Number(Math.min(b[i],b[i+1],b[i+2])<220);return{mean:delta/a.length,ink,backing:[canvas.width,canvas.height]};}finally{URL.revokeObjectURL(url);}
      });
      assert(paint.mean<.5&&paint.ink>500,`${label}/${JSON.stringify(paint)}`);assert.deepEqual(paint.backing,[Math.round(b.width*dpr),Math.round(b.height*dpr)]);
      record.frames.push({label,paint,nodes:a.nodes,calls:[a.calls,b.calls]});
      for(const r of ['svg','canvas'])await surface(r).screenshot({path:join(out,`${name}-${dpr}-${label}-${r}.png`)});
    };
    await flush();await frame('initial');
    const beforeTick=await state('svg');await page.evaluate(()=>window.hostFixture.network_link_custom_tick());await flush();
    const ticked=await state('svg');assert(ticked.ticks>beforeTick.ticks&&ticked.nodes.find(n=>n.id==='b').world.some((v,i)=>!close(v,beforeTick.nodes.find(n=>n.id==='b').world[i])));await frame('tick');
    for(const r of ['svg','canvas']){
      const s=await state(r),node=s.nodes.find(n=>n.id==='a:wing');const p=await client(r,s,node.anchor);await page.mouse.move(...p);await flush();
      assert((await state(r)).hovered.includes('force-node:a:wing'));
      const button=page.locator(`#network-link-custom-${r} .kartis-datum`).filter({hasText:node.label}).first();await button.focus();await button.press('Enter');await flush();assert.equal((await state(r)).selected,'force-node:a:wing');
      await page.evaluate(r=>{const root=document.getElementById(`network-link-custom-${r}`);root.style.transform='scale(.8,1.1)';root.style.transformOrigin='left top';},r);
      const start=await state(r),at=await client(r,start,start.nodes.find(n=>n.id==='a:wing').anchor),box=await surface(r).boundingBox();
      const packetStart=await page.evaluate(()=>window.__networkPackets.length);
      await page.mouse.move(...at);await page.mouse.down();await flush();assert.equal((await state(r)).dragging,'a:wing');
      await page.mouse.move(at[0]+31,at[1]+17,{steps:3});await flush();await page.mouse.up();await flush();
      const moved=await geometry(r);assert.equal(moved.capture,null);assert.equal(moved.dragging,null);assert(moved.nodes.find(n=>n.id==='a:wing').world.some((v,i)=>Math.abs(v-start.nodes.find(n=>n.id==='a:wing').world[i])>5));
      const packets=await page.evaluate(({r,packetStart})=>window.__networkPackets.slice(packetStart).filter(p=>p.renderer===`network-link-custom-${r}`&&['pointerdown','pointerup'].includes(p.type)),{r,packetStart});
      assert.equal(packets.length,2,JSON.stringify({r,packets,all:await page.evaluate(()=>window.__networkPackets.slice(-18))}));assert(packets.every(p=>p.trusted));
      const before=start.nodes.find(n=>n.id==='a:wing').world,after=moved.nodes.find(n=>n.id==='a:wing').world;
      for(let i=0;i<2;i++)assert(close(after[i]-before[i],(packets[1].client[i]-packets[0].client[i])*[start.width/box.width,start.height/box.height][i]/start.view[0],1e-5),JSON.stringify({r,before,after,packets,box}));
      record.gestures.push({r,node:node.id,trustedPointer:true,keyboard:true,currentLinksRebuilt:true,nonuniformCssScale:true,nativeClientPackets:packets});
      await page.evaluate(r=>{document.getElementById(`network-link-custom-${r}`).style.transform='';},r);
      await page.evaluate(()=>window.hostFixture.network_link_custom_phase(0));await flush();
    }
    await frame('after-drag-reset');
    for(const mode of [1,2]){
      const result=await page.evaluate(mode=>{
        const source=r=>{const s=JSON.parse(window.hostFixture.network_link_custom_snapshot(r));return{nodes:s.nodes,ticks:s.ticks,view:s.view,selected:s.selected,hovered:s.hovered};};
        const paint=r=>{const p=document.querySelector(`#network-link-custom-${r} ${r==='svg'?'svg':'canvas'}`);return r==='svg'?p.outerHTML:p.toDataURL();};
        const before=['svg','canvas'].map(r=>[source(r),paint(r)]);window.hostFixture.network_link_custom_mode(mode);window.hostFixture.network_link_custom_tick();let rejected=false;try{window.hostFixture.network_link_custom_flush();}catch{rejected=true;}
        return{rejected,same:JSON.stringify(before)===JSON.stringify(['svg','canvas'].map(r=>[source(r),paint(r)])),errors:['svg','canvas'].map(r=>JSON.parse(window.hostFixture.network_link_custom_snapshot(r)).error)};
      },mode);
      assert(result.rejected&&result.same&&result.errors.every(Boolean),JSON.stringify({mode,result}));record.rejections.push({mode,...result});await page.evaluate(()=>window.hostFixture.network_link_custom_mode(0));
    }
    // Reject a replacement after preflight, not merely before it is enqueued.
    const late=await page.evaluate(()=>{
      const source=r=>JSON.parse(window.hostFixture.network_link_custom_snapshot(r)).nodes;
      const old=['svg','canvas'].map(source);window.hostFixture.network_link_custom_phase(1);window.hostFixture.network_link_custom_mode(1);let failed=false;try{window.hostFixture.network_link_custom_flush();}catch{failed=true;}
      return{failed,same:JSON.stringify(old)===JSON.stringify(['svg','canvas'].map(source))};
    });assert(late.failed&&late.same);record.rejections.push({late:true,...late});
    await page.evaluate(()=>{window.hostFixture.network_link_custom_mode(0);window.hostFixture.network_link_custom_phase(1);});await flush();await frame('replacement');
    await page.evaluate(()=>{for(const r of ['svg','canvas'])document.getElementById(`network-link-custom-${r}`).style.width='520px';});await flush();
    await page.waitForFunction(()=>JSON.parse(window.hostFixture.network_link_custom_snapshot('svg')).width===520);await flush();await frame('resize');
    for(const mode of [4,5]){await page.evaluate(mode=>{window.hostFixture.network_link_custom_mode(mode);window.hostFixture.network_link_custom_tick();},mode);await flush();for(const r of ['svg','canvas'])assert((await state(r)).reentrant>0);record.lifecycle.push({mode,reentrantReadOrReject:true});}
    await page.evaluate(()=>{window.hostFixture.network_link_custom_mode(3);window.hostFixture.network_link_custom_tick();try{window.hostFixture.network_link_custom_flush();}catch{}});
    for(const r of ['svg','canvas']){const s=await state(r);assert.equal(s.active,false);assert.equal(s.drops,1);}
    await page.evaluate(()=>window.hostFixture.network_link_custom_stop());assert.equal(await page.locator('[id^="network-link-custom-"]').count(),0);
    await page.evaluate(()=>window.hostFixture.network_link_custom_mount());await flush();
    for(const r of ['svg','canvas']){
      assert(await page.evaluate(r=>window.hostFixture.network_link_custom_stop_in_preflight(r),r),'stop in replacement preflight must reject the candidate');
      const stopped=await state(r);assert.equal(stopped.active,false);assert.equal(stopped.drops,1);
      record.lifecycle.push({renderer:r,stopDuringReplacementPreflight:true,callbackDropped:stopped.drops});
    }
    await page.evaluate(()=>window.hostFixture.network_link_custom_stop());assert.equal(await page.locator('[id^="network-link-custom-"]').count(),0);
    const packets=await page.evaluate(()=>window.__networkPackets);assert(packets.some(e=>e.type==='pointerdown'&&e.trusted)&&packets.some(e=>e.type==='keydown'&&e.trusted&&e.key==='Enter'));
    assert.deepEqual(errors,[]);record.passed=true;console.log(`${name} DPR${dpr}: ${record.frames.length} frames, drag/tick/rejection/lifetime passed`);
  }catch(error){record.error=error.stack;throw error;}finally{await ctx.close();}
}}finally{await browser.close();}}
report.passed=report.contexts.length===engines.length*2&&report.contexts.every(c=>c.passed);
}finally{server.close();report.wasmSha256=sha(await readFile('examples/browser-host/pkg/host_bg.wasm'));report.scriptSha256=sha(await readFile(new URL(import.meta.url)));await writeFile(join(out,'report.json'),JSON.stringify(report,null,2));}
console.log(JSON.stringify({passed:report.passed,contexts:report.contexts.length,out}));
