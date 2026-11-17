// Actual retained force-driver candidates, custom link paint and native input.
import assert from 'node:assert/strict';
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {createServer} from 'node:http';
import {execFileSync} from 'node:child_process';
import {resolve,relative,isAbsolute,extname,join} from 'node:path';
import {chromium,firefox,webkit} from 'playwright';

const root=resolve('.'),metadata=JSON.parse(execFileSync('cargo',['metadata','--offline','--no-deps','--format-version','1'],{encoding:'utf8',windowsHide:true}));
const out=resolve(process.argv[2]||join(metadata.target_directory,'bullet-widgets-host'));await mkdir(out,{recursive:true});
const sha=x=>createHash('sha256').update(x).digest('hex');
const report={passed:false,scope:'Actual Glory typed Bullet overlays above SVG/Canvas: accepted frame+metadata, stable child state/focus, atomic candidate rejection and explicit child revision retention. Original ink remains; Scene exports omit Widgets.',contexts:[]};
const server=createServer(async(req,res)=>{try{
  if(!['GET','HEAD'].includes(req.method))throw Error('Read only');
  const file=resolve(root,'.'+decodeURIComponent(new URL(req.url,'http://localhost').pathname)),child=relative(root,file);
  if(child.startsWith('..')||isAbsolute(child))throw Error('Outside root');
  res.setHeader('Content-Type',{'.html':'text/html','.js':'text/javascript','.wasm':'application/wasm','.json':'application/json'}[extname(file)]||'application/octet-stream');res.end(await readFile(file));
}catch{res.writeHead(404);res.end();}});
await new Promise(r=>server.listen(0,'127.0.0.1',r));
const engines=process.argv.includes('--chromium-only')?[['Chromium',chromium,{}]]:[['Chrome',chromium,{channel:'chrome'}],['Firefox',firefox,{}],['WebKit',webkit,{}],['Edge',chromium,{channel:'msedge'}]];
try{for(const [name,engine,opts]of engines){const browser=await engine.launch({headless:true,...opts});try{for(const dpr of [1,2]){
  const ctx=await browser.newContext({viewport:{width:1400,height:1100},deviceScaleFactor:dpr}),page=await ctx.newPage();
  const record={name,dpr,passed:false,frames:[],rejections:[],lifecycle:[]};report.contexts.push(record);
  try{
    const errors=[];page.on('pageerror',e=>errors.push(e.message));
    await page.goto(`http://127.0.0.1:${server.address().port}/examples/browser-host/index.html`);await page.waitForFunction(()=>window.hostReady===true);
    await page.evaluate(()=>{document.querySelector('.panels').style.display='none';window.__bulletPackets=[];for(const type of ['click','keydown'])window.addEventListener(type,e=>{if(e.target.closest?.('[id^="bullet-widgets-"]'))window.__bulletPackets.push({type,trusted:e.isTrusted,key:e.key});},true);if(typeof window.hostFixture.bullet_widgets_update_for!=='function')throw Error('Missing renderer-specific typed update export');window.hostFixture.bullet_widgets_mount();});
    const snapshot=r=>page.evaluate(r=>JSON.parse(window.hostFixture.bullet_widgets_snapshot(r)),r);
    const flush=async()=>{await page.evaluate(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))));await page.evaluate(()=>window.hostFixture.bullet_widgets_flush());};
    const buttons=r=>page.locator(`#bullet-widgets-${r} [class^="bullet-widget-"]`);
    const update=async(phase,width=600,revision=0,note=phase,invalid=0)=>{await page.evaluate(a=>window.hostFixture.bullet_widgets_update(...a),[phase,width,revision,note,invalid]);await flush();};
    const paint=()=>page.evaluate(async()=>{
      const svg=document.querySelector('#bullet-widgets-svg svg'),canvas=document.querySelector('#bullet-widgets-canvas canvas'),copy=svg.cloneNode(true);
      let lang=null;for(let n=svg;n;n=n.parentElement){const x=n.getAttributeNS('http://www.w3.org/XML/1998/namespace','lang')??n.getAttribute('xml:lang');if(x!==null){lang=x;break;}if(n.hasAttribute('lang')){lang=n.getAttribute('lang');break;}}
      lang??=document.documentElement.lang;copy.setAttribute('lang',lang);copy.setAttributeNS('http://www.w3.org/XML/1998/namespace','xml:lang',lang);copy.setAttribute('width',canvas.width/devicePixelRatio);copy.setAttribute('height',canvas.height/devicePixelRatio);
      const image=new Image(),url=URL.createObjectURL(new Blob([new XMLSerializer().serializeToString(copy)],{type:'image/svg+xml'}));
      try{image.src=url;await image.decode();const c=document.createElement('canvas');c.width=canvas.width;c.height=canvas.height;c.getContext('2d').drawImage(image,0,0,c.width,c.height);const a=c.getContext('2d').getImageData(0,0,c.width,c.height).data,b=canvas.getContext('2d').getImageData(0,0,c.width,c.height).data;let total=0;for(let i=0;i<a.length;i++)total+=Math.abs(a[i]-b[i]);const pixel=(x,y)=>Array.from(canvas.getContext('2d').getImageData(Math.floor(x*devicePixelRatio),Math.floor(y*devicePixelRatio),1,1).data);return{mean:total/a.length,width:canvas.width,height:canvas.height,knownInk:{range:pixel(40,30),measure:pixel(40,80)}};}finally{URL.revokeObjectURL(url);}
    });
    const frame=async label=>{const p=await paint();assert(p.mean<0.5,`${label}: ${JSON.stringify(p)}`);assert.equal(p.height,360*dpr);assert.deepEqual(p.knownInk.range,[195,203,214,255]);assert.deepEqual(p.knownInk.measure,[55,85,130,255]);record.frames.push({label,...p});};
    await flush();for(const r of ['svg','canvas']){assert.equal(await buttons(r).count(),6);assert.equal((await snapshot(r)).calls,6);}
    await frame('initial');
    // Keep references to real DOM nodes: text equality alone cannot prove retention.
    await page.evaluate(()=>{window.__bulletElements=new Map([...document.querySelectorAll('[class^="bullet-widget-"]')].map(b=>[b.closest('[id^="bullet-widgets-"]').id+'/'+b.textContent.split('|').slice(0,2).join('|'),b]));});
    const retained=()=>page.evaluate(()=>[...window.__bulletElements].every(([key,node])=>node.isConnected&&node.closest('[id^="bullet-widgets-"]').id+'/'+node.textContent.split('|').slice(0,2).join('|')===key));
    for(const r of ['svg','canvas']){
      const button=page.locator(`#bullet-widgets-${r} .bullet-widget-range`).first();await button.click();await button.press('Enter');await flush();assert((await button.textContent()).endsWith('count=2'),await button.textContent());
    }
    const focused=await page.locator('#bullet-widgets-canvas .bullet-widget-range').first().elementHandle();
    await update(1);assert(await retained());assert(await focused.evaluate(b=>document.activeElement===b));
    for(const r of ['svg','canvas']){assert.equal((await snapshot(r)).calls,6);const texts=await buttons(r).allTextContents();assert(texts.every(t=>t.includes('note=1')));assert(texts.some(t=>t.includes('|range|v=28|rank=1|')));assert(texts.some(t=>t.includes('|measure|v=46|rank=1|')));assert(texts.some(t=>t.includes('|marker|v=76|rank=0|')));}
    await frame('source-and-row-reorder');
    await update(1,600,0,42,6);assert(await retained());for(const r of ['svg','canvas'])assert((await buttons(r).allTextContents()).every(t=>t.includes('note=42')));
    await frame('metadata-only');
    for(const invalid of [1,2,3,4,5,7]){for(const renderer of ['svg','canvas']){
      const result=await page.evaluate(([invalid,renderer])=>{
        const witness=()=>['svg','canvas'].map(r=>{const root=document.getElementById(`bullet-widgets-${r}`);return{snapshot:JSON.parse(window.hostFixture.bullet_widgets_snapshot(r)),widgets:root.querySelector('.kartis-plot-widgets').outerHTML,paint:r==='svg'?root.querySelector('svg').outerHTML:root.querySelector('canvas').toDataURL()};});
        const before=witness();let rejected=false,reason=null;try{window.hostFixture.bullet_widgets_update_for(renderer,2,520,0,999,invalid);}catch(error){if(error instanceof TypeError||error instanceof WebAssembly.RuntimeError)throw error;rejected=true;reason=String(error);}return{rejected,reason,same:JSON.stringify(before)===JSON.stringify(witness())};
      },[invalid,renderer]);assert(result.rejected&&result.same,JSON.stringify({invalid,renderer,result}));assert(await retained());record.rejections.push({invalid,renderer,...result});
    }}
    await page.evaluate(()=>window.hostFixture.bullet_widgets_hidden(true));await flush();assert(await retained());
    for(const r of ['svg','canvas']){const visibility=await page.locator(`#bullet-widgets-${r} .kartis-plot-widget-clip`).evaluateAll(a=>a.map(n=>({row:n.textContent.split('|')[0],display:getComputedStyle(n).display,inert:n.inert||n.hasAttribute('inert')})));assert.equal(visibility.filter(n=>n.row==='north:温度'&&n.display==='none').length,3);assert.equal(visibility.filter(n=>n.row==='south'&&n.display!=='none').length,3);record.lifecycle.push({renderer:r,visibility});assert.equal((await snapshot(r)).calls,6);}
    await page.evaluate(()=>window.hostFixture.bullet_widgets_hidden(false));await flush();assert(await retained());
    await update(2,520);assert(await retained());await frame('resize-source');
    await page.evaluate(()=>{for(const r of ['svg','canvas'])document.getElementById(`bullet-widgets-${r}`).style.transform='scale(.8,1.1)';});await flush();assert(await retained());
    // Every child rectangle still follows the source's measured slot after CSS scaling.
    for(const r of ['svg','canvas']){const measured=await page.locator(`#bullet-widgets-${r} .kartis-plot-widget-slot`).evaluateAll(a=>a.map(n=>{const b=n.getBoundingClientRect(),base=n.closest('.kartis-plot-widgets').getBoundingClientRect(),[row,role]=n.textContent.split('|'),band=['range','measure','marker'].indexOf(role),rowIndex=row==='north:温度'?0:1;return{actual:[b.x,b.y,b.width,b.height],expected:[base.x+(20+380*(band+1)/4)*base.width/520,base.y+(20+172*rowIndex+6+34*band)*base.height/360,100*base.width/520,26*base.height/360]}}));assert(measured.every(({actual,expected})=>actual.every((v,i)=>Math.abs(v-expected[i])<0.1)),JSON.stringify(measured));record.lifecycle.push({renderer:r,measured});}
    await frame('nonuniform-css-scale');
    // Widget construction failure is deliberately local: old range children read
    // the accepted new typed frame, while successful other roles replace children.
    await update(3,520,2,77);for(const r of ['svg','canvas']){const s=await snapshot(r);assert(s.errors.some(e=>e.includes('range Widget revision rejected')));assert.equal(s.calls,12);const ranges=await page.locator(`#bullet-widgets-${r} .bullet-widget-range`).allTextContents();assert(ranges.every(t=>t.includes('v=34')&&t.includes('note=77')));assert(ranges.some(t=>t.endsWith('count=2')));assert.equal(s.cleanups,4);}
    await frame('rejected-child-revision-new-accepted-data');
    await update(3,520,3,88);for(const r of ['svg','canvas']){const s=await snapshot(r);assert.equal(s.calls,18);assert.equal(s.cleanups,10);assert((await buttons(r).allTextContents()).every(t=>t.endsWith('count=0')));}
    await update(3,520,3,88,8);for(const r of ['svg','canvas']){assert.equal(await buttons(r).count(),0);assert.equal((await snapshot(r)).cleanups,16);}
    await update(0,600);for(const r of ['svg','canvas']){assert.equal(await buttons(r).count(),6);assert.equal((await snapshot(r)).calls,24);}
    const packets=await page.evaluate(()=>window.__bulletPackets);assert(packets.some(e=>e.type==='click'&&e.trusted));assert(packets.some(e=>e.type==='keydown'&&e.trusted&&e.key==='Enter'));
    await page.evaluate(()=>window.hostFixture.bullet_widgets_stop());assert.equal(await page.locator('[id^="bullet-widgets-"]').count(),0);await page.evaluate(()=>window.hostFixture.bullet_widgets_mount());await flush();await page.evaluate(()=>window.hostFixture.bullet_widgets_stop());
    assert.deepEqual(errors,[]);record.passed=true;console.log(`${name} DPR${dpr}: ${record.frames.length} paired paint frames, typed state/focus/rejections/lifecycle passed`);
  }catch(error){record.error=error.stack;throw error;}finally{await ctx.close();}
}}finally{await browser.close();}}
report.passed=report.contexts.length===engines.length*2&&report.contexts.every(c=>c.passed);
}finally{server.close();report.wasmSha256=sha(await readFile('examples/browser-host/pkg/host_bg.wasm'));report.scriptSha256=sha(await readFile(new URL(import.meta.url)));await writeFile(join(out,'report.json'),JSON.stringify(report,null,2));}
console.log(JSON.stringify({passed:report.passed,contexts:report.contexts.length,out}));
