// Actual composite screenshots and native pointer targets, including live UI.
import assert from 'node:assert/strict';
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {createServer} from 'node:http';
import {execFileSync} from 'node:child_process';
import {inflateSync} from 'node:zlib';
import {resolve,relative,isAbsolute,extname,join} from 'node:path';
import {chromium,firefox,webkit} from 'playwright';

const root=resolve('.'),meta=JSON.parse(execFileSync('cargo',['metadata','--offline','--no-deps','--format-version','1'],{encoding:'utf8',windowsHide:true}));
const out=resolve(process.argv.slice(2).find(arg=>!arg.startsWith('--'))||join(meta.target_directory,'ordered-widgets-browser'));await mkdir(out,{recursive:true});
const hash=bytes=>createHash('sha256').update(bytes).digest('hex');
const scriptAtStart=hash(await readFile(new URL(import.meta.url))),wasmAtStart=hash(await readFile('examples/browser-host/pkg/host_bg.wasm')),glueAtStart=hash(await readFile('examples/browser-host/pkg/host.js'));
const report={passed:false,scope:'Browser Scene-order live Widgets: actual composited screen pixels, once-only group alpha, native painted-pixel pointer occlusion, retained state and transactional failures. Native semantic/CPU same-layer input is not claimed.',pixelTolerance:1,hitProfile:'Canvas actual alpha > 0, quantized at the accepted renderer DPR, including for SVG native paint.',contexts:[]};

// Decode Playwright screenshots with Node built-ins; no image dependency.
const crcTable=Uint32Array.from({length:256},(_,n)=>{for(let i=0;i<8;i++)n=n&1?0xedb88320^(n>>>1):n>>>1;return n>>>0;});
const crc32=bytes=>{let crc=0xffffffff;for(const b of bytes)crc=crcTable[(crc^b)&255]^(crc>>>8);return(crc^0xffffffff)>>>0;};
function png(bytes){
 assert.equal(bytes.subarray(0,8).toString('hex'),'89504e470d0a1a0a');let width,height,channels,ended=false;const idat=[];
 for(let p=8;p<bytes.length;){assert(p+12<=bytes.length);const n=bytes.readUInt32BE(p),kind=bytes.toString('ascii',p+4,p+8),body=bytes.subarray(p+8,p+8+n);assert(p+12+n<=bytes.length);assert.equal(crc32(bytes.subarray(p+4,p+8+n)),bytes.readUInt32BE(p+8+n));
  if(kind==='IHDR'){assert.equal(body[8],8);width=body.readUInt32BE(0);height=body.readUInt32BE(4);channels=({0:1,2:3,4:2,6:4})[body[9]];assert(channels&&body[10]===0&&body[11]===0&&body[12]===0);assert(width*height<=32_000_000);}
  if(kind==='IDAT')idat.push(body);if(kind==='IEND')ended=true;p+=n+12;
 }
 assert(ended&&channels&&idat.length);const packed=inflateSync(Buffer.concat(idat)),stride=width*channels,rows=Buffer.alloc(stride*height);assert.equal(packed.length,(stride+1)*height);
 const paeth=(a,b,c)=>{const p=a+b-c,aa=Math.abs(p-a),bb=Math.abs(p-b),cc=Math.abs(p-c);return aa<=bb&&aa<=cc?a:bb<=cc?b:c;};
 for(let y=0;y<height;y++){const f=packed[y*(stride+1)];assert(f<=4);for(let x=0;x<stride;x++){const a=x>=channels?rows[y*stride+x-channels]:0,b=y?rows[(y-1)*stride+x]:0,c=y&&x>=channels?rows[(y-1)*stride+x-channels]:0;rows[y*stride+x]=(packed[y*(stride+1)+1+x]+(f===0?0:f===1?a:f===2?b:f===3?Math.floor((a+b)/2):paeth(a,b,c)))&255;}}
 const data=Buffer.alloc(width*height*4);for(let p=0;p<width*height;p++){const i=p*channels,j=p*4;if(channels<=2){data[j]=data[j+1]=data[j+2]=rows[i];data[j+3]=channels===2?rows[i+1]:255;}else{data[j]=rows[i];data[j+1]=rows[i+1];data[j+2]=rows[i+2];data[j+3]=channels===4?rows[i+3]:255;}}
 return{width,height,data};
}
const sample=(image,x,y,width=360,height=260)=>{const i=4*(Math.min(image.height-1,Math.floor(y*image.height/height))*image.width+Math.min(image.width-1,Math.floor(x*image.width/width)));return [...image.data.subarray(i,i+4)];};
const near=(actual,expected,context)=>assert(actual.every((v,i)=>Math.abs(v-expected[i])<=1),JSON.stringify({context,actual,expected}));
const server=createServer(async(req,res)=>{try{assert(['GET','HEAD'].includes(req.method));const file=resolve(root,'.'+decodeURIComponent(new URL(req.url,'http://localhost').pathname)),child=relative(root,file);assert(!child.startsWith('..')&&!isAbsolute(child));res.setHeader('Content-Type',{'.html':'text/html','.js':'text/javascript','.wasm':'application/wasm','.json':'application/json'}[extname(file)]||'application/octet-stream');res.end(await readFile(file));}catch{res.writeHead(404);res.end();}});
await new Promise(r=>server.listen(0,'127.0.0.1',r));
const engines=process.argv.includes('--chromium-only')?[['Chromium',chromium,{}]]:process.argv.includes('--firefox-only')?[['Firefox',firefox,{}]]:[['Chrome',chromium,{channel:'chrome'}],['Firefox',firefox,{}],['WebKit',webkit,{}],['Edge',chromium,{channel:'msedge'}]];
try{for(const [name,engine,opts] of engines){const browser=await engine.launch({headless:true,...opts});try{for(const dpr of [1,2]){
 const context=await browser.newContext({viewport:{width:1400,height:1000},deviceScaleFactor:dpr}),page=await context.newPage();
 const record={name,dpr,passed:false,frames:[],hits:[],rejections:[],lifecycle:[]};report.contexts.push(record);
 const prefix=`${name}-${dpr}`;
 try{
  const errors=[];page.on('pageerror',error=>errors.push(error.message));
  // Pure browser primitive oracle, independent of Kartis/Glory/source data.
  // Chromium's compositor can quantize CSS opacity .5 to 126 on white, so
  // keep the original mathematical RED and compare actual once-group paint
  // against this exact same runtime primitive at the unchanged 1-unit gate.
  const nativeGroup={},nativeHalfPaint={};record.nativeGroupControls={};record.nativeHalfPaintControls={};
  for(const renderer of ['svg','canvas'])for(const profile of ['group','mark-alpha']){
   const controlPage=await context.newPage();
   const source=renderer==='svg'?`<svg viewBox="0 0 360 260" width="360" height="260" style="position:absolute;inset:0"><rect id="blue" x="120" y="70" width="80" height="60" fill="blue" opacity="${profile==='group'?1:0.5}"/></svg>`:'<canvas id="blue" style="position:absolute;inset:0;width:360px;height:260px"></canvas>';
   const controlHTML=`<html><body style="margin:0;background:white"><div id="control" style="position:relative;width:360px;height:260px;isolation:isolate"><div id="group" style="position:absolute;inset:0;opacity:${profile==='group'?0.5:1}"><button style="position:absolute;left:60px;top:60px;width:160px;height:80px;background:rgb(255,0,0);border:0;padding:0;appearance:none"></button>${source}</div></div></body></html>`;
   await controlPage.setContent(controlHTML);if(renderer==='canvas')await controlPage.evaluate(profile=>{const c=document.getElementById('blue');c.width=360*devicePixelRatio;c.height=260*devicePixelRatio;const cx=c.getContext('2d');cx.scale(devicePixelRatio,devicePixelRatio);cx.fillStyle='blue';cx.globalAlpha=profile==='group'?1:0.5;cx.fillRect(120,70,80,60);},profile);
   const opacity=await controlPage.locator('#group').evaluate(n=>getComputedStyle(n).opacity);assert.equal(opacity,profile==='group'?'0.5':'1');
   const markAlpha=await controlPage.locator('#blue').evaluate(n=>n instanceof HTMLCanvasElement?n.getContext('2d').globalAlpha:Number(n.getAttribute('opacity')));assert.equal(markAlpha,profile==='group'?1:0.5);
   const controlFile=join(out,`${prefix}-independent-native-${profile}-${renderer}.png`),controlBytes=await controlPage.locator('#control').screenshot({path:controlFile}),controlImage=png(controlBytes),colors={red:sample(controlImage,80,80),blue:sample(controlImage,150,90)};
   assert.equal(colors.red[0],255);assert(colors.blue[2]>0);assert.deepEqual(colors.red.slice(1,3),[colors.red[1],colors.red[1]]);
   if(profile==='group'){assert.equal(colors.blue[2],255);assert(colors.red[1]>0&&colors.red[1]<255);nativeGroup[renderer]=colors;}else{near(colors.red,[255,0,0,255],'independent half-alpha keeps uncovered red');assert(colors.blue[0]>0&&colors.blue[0]<255&&colors.blue[2]<255);nativeHalfPaint[renderer]=colors;}
   const detail={...colors,cssOpacity:opacity,markAlpha,mathematicalRed:profile==='group'?[255,128,128,255]:[255,0,0,255],mathematicalBlue:profile==='group'?[128,128,255,255]:[128,0,128,255],file:controlFile,sha256:hash(controlBytes)};
   (profile==='group'?record.nativeGroupControls:record.nativeHalfPaintControls)[renderer]=detail;
   await writeFile(controlFile+'.html',controlHTML);await controlPage.close();
  }
  await page.goto(`http://127.0.0.1:${server.address().port}/examples/browser-host/index.html`);await page.waitForFunction(()=>window.hostReady===true);
  await page.evaluate(()=>{
   document.querySelector('.panels').style.display='none';if(typeof window.hostFixture.ordered_widgets_mount!=='function')throw Error('Missing ordered Widget fixture');window.__orderedPointerTrace=[];
   for(const type of ['pointermove','pointerover','pointerout'])document.addEventListener(type,event=>{
    const row={type,x:event.clientX,y:event.clientY,trusted:event.isTrusted,target:event.target.outerHTML?.slice(0,160)};window.__orderedPointerTrace.push(row);if(window.__orderedPointerTrace.length>20)window.__orderedPointerTrace.shift();
    if(type==='pointermove'){
     const surface=event.target.closest?.('.kartis-surface'),mask=event.target.closest?.('.kartis-ordered-hit-mask'),box=surface?.getBoundingClientRect();
     window.__orderedLastMove={...row,surface:box?[box.x,box.y,box.width,box.height]:null,mask:mask?.querySelector('clipPath path')?.getAttribute('d')??null,widget:event.target.closest?.('.ordered-widget-main')!==null};
    }
   },true);
  });
  const flush=async()=>{await page.evaluate(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))));await page.evaluate(()=>window.hostFixture.ordered_widgets_flush());};
  const mount=async()=>{for(const r of ['svg','canvas'])await page.evaluate(r=>window.hostFixture.ordered_widgets_mount(r,0),r);await flush();};
  const state=r=>page.evaluate(r=>JSON.parse(window.hostFixture.ordered_widgets_snapshot(r)),r);
  const button=r=>page.locator(`#ordered-widgets-${r} .ordered-widget-main`);
  const surface=r=>page.locator(`#ordered-widgets-${r} .kartis-surface`);
  const text=r=>button(r).textContent();
  const count=async r=>Number((await text(r)).match(/count=(\d+)/)[1]);
  const update=async({order=0,width=360,revision=0,note=0,variant=0,mode=0,invalid=0}={})=>{for(const r of ['svg','canvas'])await page.evaluate(args=>window.hostFixture.ordered_widgets_update(...args),[r,order,width,revision,note,variant,mode|(invalid<<8)]);await flush();};
  const point=async(r,x,y)=>{const s=await state(r),b=await surface(r).boundingBox(),[k,tx,ty]=s.view;return{x:b.x+(x*k+tx)*b.width/s.width,y:b.y+(y*k+ty)*b.height/s.height};};
  const hit=async(r,x,y,expected,label)=>{
   await surface(r).scrollIntoViewIfNeeded();const p=await point(r,x,y);await page.evaluate(()=>{window.__orderedLastMove=null;});await page.mouse.move(p.x,p.y);await flush();
   const transport=await page.evaluate(r=>{
    const actual=window.__orderedLastMove,s=JSON.parse(window.hostFixture.ordered_widgets_snapshot(r));if(!actual?.surface)return{actual};
    const b=actual.surface,rendered=[(actual.x-b[0])*s.width/b[2],(actual.y-b[1])*s.height/b[3]],scene=[(rendered[0]-s.view[1])/s.view[0],(rendered[1]-s.view[2])/s.view[0]],cx=document.createElement('canvas').getContext('2d');
    const glyph=window.__orderedGlyphAlpha;return{actual:{...actual,mask:actual.mask?'captured actual native Canvas-alpha path':null},rendered,scene,maskContainsActual:actual.mask?cx.isPointInPath(new Path2D(actual.mask),...rendered):null,independentGlyphAlpha:glyph?glyph.data[4*(Math.floor(scene[1]*glyph.ratio)*glyph.width+Math.floor(scene[0]*glyph.ratio))+3]:null};
   },r);
   assert(transport.actual?.trusted,'use a real trusted native pointer sample');
   const hover=await button(r).evaluate(n=>n.matches(':hover')),before=await count(r);
   if(hover!==expected){
    const diagnostic=await page.evaluate(({r,p,x,y})=>{
     const root=document.getElementById(`ordered-widgets-${r}`),rect=n=>{const b=n.getBoundingClientRect();return[b.x,b.y,b.width,b.height];},matrix=m=>m?[m.a,m.b,m.c,m.d,m.e,m.f]:null;
     return{point:p,logical:[x,y],pointerTrace:window.__orderedPointerTrace,scroll:[scrollX,scrollY],surface:rect(root.querySelector('.kartis-surface')),elements:document.elementsFromPoint(p.x,p.y).map(n=>({tag:n.tagName,class:n.getAttribute('class'),snippet:n.outerHTML.slice(0,220)})),masks:[...root.querySelectorAll('.kartis-ordered-hit-mask')].map(svg=>{const path=svg.querySelector('clipPath path'),target=svg.querySelector(':scope > rect'),cx=document.createElement('canvas').getContext('2d');return{svg:rect(svg),target:rect(target),screenMatrix:matrix(svg.getScreenCTM()),targetMatrix:matrix(target.getScreenCTM()),pointerEvents:getComputedStyle(target).pointerEvents,containsLogical:cx.isPointInPath(new Path2D(path.getAttribute('d')),x,y),clip:target.getAttribute('clip-path')};}),text:[...root.querySelectorAll('svg text')].filter(n=>n.textContent==='MMMM').map(n=>({source:n.outerHTML,font:getComputedStyle(n).font,rect:rect(n),screenMatrix:matrix(n.getScreenCTM())}))};
    },{r,p,x,y});
    await writeFile(join(out,`${prefix}-${r}-${label.replaceAll(/[^a-z0-9]/gi,'_')}-hit-diagnostic.json`),JSON.stringify(diagnostic,null,2));
   }
   assert.equal(hover,expected,`${r}/${label}: CSS hover must follow actual painted occlusion`);
   assert.equal(transport.actual.widget,expected,'actual native event target follows expected paint order');
   if(!expected)assert.equal(transport.maskContainsActual,true,'actual trusted pointer coordinate belongs to the emitted Canvas-alpha footprint');
   if(label==='label painted pixels')assert(transport.independentGlyphAlpha>0,'actual trusted label probe stays in the independent native Canvas glyph footprint');
   if(label==='label transparent pixels')assert.equal(transport.independentGlyphAlpha,0,'actual trusted gap probe stays outside independent native Canvas glyph footprint');
   await page.mouse.click(p.x,p.y);await flush();const after=await count(r);assert.equal(after-before,Number(expected),`${r}/${label}: real native mouse activation`);
   const accepted=await state(r);assert(!accepted.hovered.includes('stage'),'replaced source ink never revives as a physical hover target');
   if(expected){assert.deepEqual(accepted.hovered,[],'entering live Widget clears stale upper source hover');assert.equal(await page.locator(`#ordered-widgets-${r} .kartis-surface-tooltip`).evaluate(n=>getComputedStyle(n).display),'none','live Widget clears source tooltip');}
   const evidence={renderer:r,label,point:[x,y],requested:p,transport,hover,before,after};record.hits.push(evidence);return evidence;
  };
  const frame=async(label,expected)=>{
   for(const r of ['svg','canvas']){
    await page.mouse.move(1000,900);await flush();
    const file=join(out,`${prefix}-${label}-${r}.png`),bytes=await surface(r).screenshot({path:file}),image=png(bytes),s=await state(r);
    const samples=expected.map(([x,y,colors])=>{const color=Array.isArray(colors)?colors:colors[r],[k,tx,ty]=s.view,screen=[x*k+tx,y*k+ty],actual=sample(image,...screen,s.width,s.height);near(actual,color,`${r}/${label}/${x},${y}`);return{point:[x,y],screen,actual,expected:color};});
    const dom=await surface(r).evaluate(n=>n.outerHTML);await writeFile(file+'.html',dom);await writeFile(file+'.json',JSON.stringify(s,null,2));
    assert.equal(await surface(r).getAttribute('role'),'group');assert.equal(await surface(r).getAttribute('data-renderer'),r);
    assert.deepEqual(await surface(r).locator('.kartis-ordered-renderer-plane').evaluateAll(nodes=>nodes.map(n=>n.getAttribute('aria-hidden'))),Array(await surface(r).locator('.kartis-ordered-renderer-plane').count()).fill('true'));
    assert.equal(await surface(r).getByRole('button').count(),label==='zero-group'?1:label==='no-widget-restores-source'?0:2,'actual Widgets stay accessible independently of hidden native renderer planes');
    const groupOpacity=await surface(r).locator('.kartis-ordered-opacity-group').evaluateAll(nodes=>nodes.map(n=>getComputedStyle(n).opacity));
    if(label==='once-group-alpha'||label==='group-widget-after-blue')assert.deepEqual(groupOpacity,['0.5'],'source opacity stays exactly .5; runtime oracle does not retune production alpha');
    if(label==='zero-group')assert.deepEqual(groupOpacity,['0']);
    if(label==='semi-transparent-occluder'&&r==='svg')assert.equal(Number(await surface(r).locator('path[data-id="decorative-blue"]').getAttribute('opacity')),0.5,'authored mark opacity remains exactly .5');
    record.frames.push({renderer:r,label,file,sha256:hash(bytes),dimensions:[image.width,image.height],groupOpacity,samples});
   }
  };
  const retained=()=>page.evaluate(()=>[...window.__orderedElements].every(([r,n])=>n.isConnected&&document.querySelector(`#ordered-widgets-${r} .ordered-widget-main`)===n));
  const witness=()=>page.evaluate(()=>['svg','canvas'].map(r=>{
   const root=document.getElementById(`ordered-widgets-${r}`),s=JSON.parse(window.hostFixture.ordered_widgets_snapshot(r));
   return{renderer:r,accepted:{active:s.active,width:s.width,height:s.height,revision:s.revision,selected:s.selected,hovered:s.hovered,view:s.view},surfaceAttributes:{role:root.querySelector('.kartis-surface').getAttribute('role'),renderer:root.querySelector('.kartis-surface').getAttribute('data-renderer')},paint:[...root.querySelectorAll('.kartis-ordered-paint')].map(n=>n.outerHTML),canvases:[...root.querySelectorAll('canvas')].map(n=>n.toDataURL()),widgets:[...root.querySelectorAll('.ordered-widget')].map(n=>n.outerHTML),controls:[...root.querySelectorAll('.kartis-datum')].map(n=>n.outerHTML),focused:document.activeElement?.className};
  }));
  await mount();
  for(const renderer of ['svg','canvas']){
   const alignment=await page.evaluate(renderer=>{
    const host=document.getElementById(`ordered-widgets-${renderer}`),surface=host.querySelector('.kartis-surface'),paint=surface.querySelector('.kartis-ordered-renderer-plane svg, .kartis-ordered-renderer-plane canvas');
    const read=()=>{const b=surface.getBoundingClientRect(),p=paint.getBoundingClientRect(),m=paint instanceof SVGSVGElement?paint.getCTM():paint.getContext('2d').getTransform();return{state:window.hostFixture.ordered_widgets_snapshot(renderer),dom:surface.outerHTML,origin:[b.x+scrollX,b.y+scrollY],size:[b.width,b.height],localPaint:[p.x-b.x,p.y-b.y,p.width,p.height],matrix:m?[m.a,m.b,m.c,m.d,m.e,m.f]:null,scroll:[scrollX,scrollY]};};
    const before=read();host.style.position='relative';host.style.left=`${Math.round(before.origin[0])-before.origin[0]}px`;host.style.top=`${Math.round(before.origin[1])-before.origin[1]}px`;return{before,after:read()};
   },renderer);
   await writeFile(join(out,`${prefix}-${renderer}-pixel-grid-alignment.json`),JSON.stringify(alignment,null,2));
   assert.equal(alignment.before.state,alignment.after.state);assert.equal(alignment.before.dom,alignment.after.dom);assert.deepEqual(alignment.before.size,alignment.after.size);assert.deepEqual(alignment.before.matrix,alignment.after.matrix);assert.deepEqual(alignment.before.scroll,alignment.after.scroll);assert(alignment.before.localPaint.every((v,i)=>Math.abs(v-alignment.after.localPaint[i])<1e-9));assert(alignment.after.origin.every(v=>Math.abs(v-Math.round(v))<1e-3));
  }
  for(const r of ['svg','canvas']){assert.equal((await state(r)).calls,2);assert.equal(await page.locator(`#ordered-widgets-${r} .ordered-widget`).count(),2);}
  await page.evaluate(()=>{window.__orderedElements=new Map(['svg','canvas'].map(r=>[r,document.querySelector(`#ordered-widgets-${r} .ordered-widget-main`)]));});
  await frame('upper-blue',[[80,80,[255,0,0,255]],[150,90,[0,0,255,255]],[270,80,[255,255,255,255]],[290,80,[255,0,255,255]],[80,190,[255,255,0,255]]]);
  for(const r of ['svg','canvas']){await hit(r,150,90,false,'covered');await hit(r,80,80,true,'uncovered');const before=await count(r);await button(r).press('Enter');await flush();assert.equal(await count(r),before+1);record.lifecycle.push({renderer:r,nativeKeyboardActivation:true});}
  await update({order:1,note:11});assert(await retained());
  await frame('widget-after-blue',[[80,80,[255,0,0,255]],[150,90,[255,0,0,255]]]);
  for(const r of ['svg','canvas'])await hit(r,150,90,true,'source order reversed');
  // Same revision and Node retains actual focus across paint/group parent moves.
  await button('canvas').focus();const focusedBefore=await button('canvas').evaluate(n=>document.activeElement===n);assert(focusedBefore);
  await update({order:2,note:22});assert(await retained());assert(await button('canvas').evaluate(n=>document.activeElement===n),'cross-parent group insertion retains focus');
  for(const r of ['svg','canvas']){assert.equal((await state(r)).calls,2);assert((await text(r)).includes('note=22'));}
  await frame('once-group-alpha',[[80,80,{svg:nativeGroup.svg.red,canvas:nativeGroup.canvas.red}],[150,90,{svg:nativeGroup.svg.blue,canvas:nativeGroup.canvas.blue}]]);
  for(const r of ['svg','canvas']){await hit(r,150,90,false,'group covered');await hit(r,80,80,true,'group uncovered');}
  await update({order:3,note:23});assert(await retained());await frame('group-widget-after-blue',[[150,90,{svg:nativeGroup.svg.red,canvas:nativeGroup.canvas.red}]]);
  for(const r of ['svg','canvas'])await hit(r,150,90,true,'group reverse order');
  await update({order:0,note:31,variant:1});await frame('semi-transparent-occluder',[[150,90,{svg:nativeHalfPaint.svg.blue,canvas:nativeHalfPaint.canvas.blue}]]);
  for(const r of ['svg','canvas'])await hit(r,150,90,false,'nonzero transparent alpha occludes');
  await update({order:0,note:31,variant:2});await frame('zero-alpha',[[150,90,[255,0,0,255]]]);
  for(const r of ['svg','canvas'])await hit(r,150,90,true,'zero alpha is transparent to pointer');
  await update({order:0,note:31,variant:3});await frame('actual-hole',[[130,90,[0,0,255,255]],[150,90,[255,0,0,255]]]);
  for(const r of ['svg','canvas']){await hit(r,130,90,false,'ring ink');await hit(r,150,90,true,'EvenOdd hole');}
  await update({order:0,note:31,variant:4});await frame('actual-dashes',[[128,100,[0,0,255,255]],[144,100,[255,0,0,255]]]);
  for(const r of ['svg','canvas']){await hit(r,128,100,false,'dash ink');await hit(r,144,100,true,'dash gap');}
  await update({order:0,note:31,variant:5});await frame('original-clip',[[130,90,[0,0,255,255]],[150,90,[255,0,0,255]]]);
  for(const r of ['svg','canvas']){await hit(r,130,90,false,'clipped paint');await hit(r,150,90,true,'outside clip');}
  await update({order:0,note:31,variant:6});
  const nativeGlyph=await page.evaluate(()=>{
   const canvas=document.createElement('canvas'),ratio=devicePixelRatio;canvas.width=360*ratio;canvas.height=260*ratio;const cx=canvas.getContext('2d');cx.scale(ratio,ratio);cx.direction='ltr';cx.font='400 32px sans-serif';cx.fontKerning='normal';cx.textBaseline='alphabetic';cx.fillStyle='blue';cx.fillText('MMMM',124,110);
   const data=cx.getImageData(0,0,canvas.width,canvas.height).data;window.__orderedGlyphAlpha={data,ratio,width:canvas.width};
   const alpha=(x,y)=>data[4*(Math.floor(y*ratio)*canvas.width+Math.floor(x*ratio))+3],solid=[],empty=[];
   for(let y=80;y<112;y++)for(let x=124;x<215;x++){const values=[-1,0,1].flatMap(dy=>[-1,0,1].map(dx=>alpha(x+dx,y+dy)));if(values.every(v=>v>0))solid.push([x,y]);if(values.every(v=>v===0))empty.push([x,y]);}
   return{font:cx.font,kerning:cx.fontKerning,direction:cx.direction,baseline:cx.textBaseline,ratio,solidPolicy:'all nine native Canvas alpha values > 0; exact declared hit profile',emptyPolicy:'all nine native Canvas alpha values == 0',solid,empty};
  });
  // Find an interior painted label cell from the actual composite screenshot.
  // Its location is font-dependent; opaque blue and red samples are mandatory.
  for(const r of ['svg','canvas']){
   const file=join(out,`${prefix}-label-hit-${r}.png`),bytes=await surface(r).screenshot({path:file}),image=png(bytes);let blue=null,red=null,originalBlue=null;
   const interior=(x,y,color)=>[-1,0,1].every(dy=>[-1,0,1].every(dx=>sample(image,x+dx,y+dy).every((value,i)=>value===color[i])));
   for(let y=80;y<112;y++)for(let x=124;x<215;x++)if(!originalBlue&&interior(x,y,[0,0,255,255]))originalBlue=[x+.25,y+.25];
   const solidIntersection=nativeGlyph.solid.filter(([x,y])=>interior(x,y,[0,0,255,255])),emptyIntersection=nativeGlyph.empty.filter(([x,y])=>interior(x,y,[255,0,0,255]));
   if(solidIntersection.length)blue=solidIntersection[0].map(v=>v+.25);if(emptyIntersection.length)red=emptyIntersection[0].map(v=>v+.25);
   const probe={renderer:r,font:nativeGlyph,originalVisualBlue:originalBlue,solidIntersectionCount:solidIntersection.length,emptyIntersectionCount:emptyIntersection.length,blue,red};await writeFile(file+'.probe.json',JSON.stringify(probe,null,2));
   assert(blue&&red,'actual label and independent native Canvas must have stable ink and transparent interior intersections');await hit(r,...blue,false,'label painted pixels');await hit(r,...red,true,'label transparent pixels');record.frames.push({renderer:r,label:'native label hit',file,sha256:hash(bytes),...probe});
  }
  await update({order:0,note:31});for(const r of ['svg','canvas'])await page.evaluate(r=>window.hostFixture.ordered_widgets_blend(r,true),r);await flush();
  await frame('multiply-over-widget',[[80,80,[255,0,0,255]],[150,90,[0,0,0,255]]]);
  for(const r of ['svg','canvas'])await page.evaluate(r=>window.hostFixture.ordered_widgets_blend(r,false),r);await flush();
  await update({order:0,note:40});assert(await retained());
  // Strict admissions reject synchronously without changing accepted DOM/data.
  for(const r of ['svg','canvas'])for(const [mode,invalid] of [[1,0],[2,0],[0,1],[0,2],[0,3]]){
   await page.mouse.move(1000,900);await flush();const before=await witness();
   const rejected=await page.evaluate(args=>{try{window.hostFixture.ordered_widgets_update(...args);return null;}catch(error){if(error instanceof TypeError||error instanceof WebAssembly.RuntimeError)throw error;return String(error);}},[r,3,340,1,999,0,mode|(invalid<<8)]);
   const after=await witness();await writeFile(join(out,`${prefix}-reject-${r}-${mode}-${invalid}.json`),JSON.stringify({rejected,before,after},null,2));assert(rejected);assert.deepEqual(after,before,'strict candidate rejection preserves exact accepted frame');assert(await retained());record.rejections.push({renderer:r,mode,invalid,rejected});
  }
  // Inject a real third Canvas context access during multi-plane preparation,
  // for BOTH SVG and Canvas. All old composited paint and focused UI survives.
  for(const r of ['svg','canvas']){
   await button(r).focus();await flush();
   const idleBefore=await witness();await page.evaluate(()=>window.hostFixture.ordered_widgets_flush());const idleAfter=await witness();
   const idleDeltas=Object.fromEntries(idleAfter.map((entry,i)=>[entry.renderer,entry.accepted.revision-idleBefore[i].accepted.revision]));
   const idleStable=structuredClone(idleAfter);for(let i=0;i<idleStable.length;i++)idleStable[i].accepted.revision=idleBefore[i].accepted.revision;
   assert.deepEqual(idleStable,idleBefore,'an independent idle flush changes only paint revision telemetry');
   const before=await witness(),beforePNG=await surface(r).screenshot();
   const injected=await page.evaluate(r=>{
    const native=HTMLCanvasElement.prototype.getContext;let calls=0,thrown=0,reason=null;
    HTMLCanvasElement.prototype.getContext=function(...args){if(args[0]==='2d'&&++calls===3){thrown++;throw Error('ordered third-plane context refusal');}return native.apply(this,args);};
    try{try{window.hostFixture.ordered_widgets_update(r,3,340,1,909,0,0);window.hostFixture.ordered_widgets_flush();}catch(error){if(error instanceof TypeError||error instanceof WebAssembly.RuntimeError)throw error;reason=String(error);}}finally{HTMLCanvasElement.prototype.getContext=native;}
    return{calls,thrown,reason};
   },r);
   const after=await witness(),afterPNG=await surface(r).screenshot();await writeFile(join(out,`${prefix}-plane-refusal-${r}.json`),JSON.stringify({injected,idleControl:{before:idleBefore,after:idleAfter,deltas:idleDeltas},before,after,beforePNG:hash(beforePNG),afterPNG:hash(afterPNG)},null,2));await writeFile(join(out,`${prefix}-plane-refusal-${r}-before.png`),beforePNG);await writeFile(join(out,`${prefix}-plane-refusal-${r}-after.png`),afterPNG);
   const normalized=structuredClone(after);for(let i=0;i<normalized.length;i++)if(normalized[i].renderer!==r){assert.equal(normalized[i].accepted.revision-before[i].accepted.revision,idleDeltas[normalized[i].renderer],'non-target renderer follows measured idle-flush telemetry');normalized[i].accepted.revision=before[i].accepted.revision;}
   assert.equal(injected.thrown,1);assert.deepEqual(normalized,before,'plane preparation failure keeps exact accepted target revision and complete frame');assert.equal(hash(afterPNG),hash(beforePNG),'plane preparation failure keeps actual composite pixels');assert(await button(r).evaluate(n=>document.activeElement===n),'failed preparation keeps real DOM focus');assert(await retained());record.rejections.push({renderer:r,publication:injected,idleDeltas});
  }
  await update({order:0,note:50});
  await page.evaluate(()=>{for(const r of ['svg','canvas'])document.getElementById(`ordered-widgets-${r}`).style.transform='scale(.8,1.1)';});await flush();
  await frame('nonuniform-css',[[80,80,[255,0,0,255]],[150,90,[0,0,255,255]]]);
  for(const r of ['svg','canvas']){await hit(r,150,90,false,'nonuniform CSS covered');await hit(r,80,80,true,'nonuniform CSS uncovered');}
  await page.evaluate(()=>window.hostFixture.ordered_widgets_camera(false));await flush();
  await frame('camera-and-css',[[80,80,[255,0,0,255]],[150,90,[0,0,255,255]]]);
  for(const r of ['svg','canvas']){assert((await state(r)).view[0]>1);await hit(r,150,90,false,'camera covered');await hit(r,80,80,true,'camera uncovered');}
  assert(await retained());await page.evaluate(()=>window.hostFixture.ordered_widgets_camera(true));await flush();
  await page.evaluate(()=>{for(const r of ['svg','canvas'])document.getElementById(`ordered-widgets-${r}`).style.transform='none';});await update({order:4,note:51});assert(await retained());
  for(const r of ['svg','canvas'])assert.equal(await button(r).isVisible(),false);
  await frame('zero-group',[[80,80,[255,255,255,255]],[150,90,[255,255,255,255]]]);
  await update({order:2,note:52});for(const r of ['svg','canvas'])assert(await button(r).isVisible());assert(await retained());
  await page.evaluate(()=>window.hostFixture.ordered_widgets_hidden(true));await flush();for(const r of ['svg','canvas'])assert.equal(await button(r).isVisible(),false);
  await page.evaluate(()=>window.hostFixture.ordered_widgets_hidden(false));await flush();assert(await retained());
  await update({order:0,note:60,revision:3,mode:3});for(const r of ['svg','canvas'])assert((await state(r)).reads>=2);
  await update({order:0,note:61,revision:4,mode:4});for(const r of ['svg','canvas'])assert((await state(r)).reads>=4);
  await update({order:0,note:62,revision:4,invalid:4});
  for(const r of ['svg','canvas'])assert.equal(await page.locator(`#ordered-widgets-${r} .ordered-widget`).count(),0);
  await frame('no-widget-restores-source',[[80,80,[0,220,0,255]],[150,90,[0,0,255,255]]]);
  for(const r of ['svg','canvas'])for(const mode of [1,2]){
   const before=await witness(),rejected=await page.evaluate(args=>{try{window.hostFixture.ordered_widgets_update(...args);return null;}catch(error){if(error instanceof TypeError||error instanceof WebAssembly.RuntimeError)throw error;return String(error);}},[r,0,360,6,999,0,mode]);
   assert(rejected);assert.deepEqual(await witness(),before,'failed first keys retain original paint without an empty replacement hole');assert.equal(await page.locator(`#ordered-widgets-${r} .ordered-widget`).count(),0);record.rejections.push({renderer:r,newKeys:true,mode,rejected});
  }
  await update({order:0,note:63,revision:7});
  for(const r of ['svg','canvas']){
   const burst=await page.evaluate(r=>{let accepted=0,reason=null;for(let revision=100;revision<300;revision++){try{window.hostFixture.ordered_widgets_update(r,0,360,revision,70,0,0);accepted++;}catch(error){if(error instanceof TypeError||error instanceof WebAssembly.RuntimeError)throw error;reason=String(error);break;}}return{accepted,reason};},r);
   assert(burst.accepted>0&&burst.accepted<200&&/budget/.test(burst.reason),JSON.stringify(burst));await flush();record.lifecycle.push({renderer:r,burst});
  }
  await update({order:0,note:71,revision:301});await frame('budget-recovered',[[80,80,[255,0,0,255]],[150,90,[0,0,255,255]]]);
  for(const r of ['svg','canvas']){
   const stopped=await page.evaluate(r=>{try{window.hostFixture.ordered_widgets_update(r,0,360,302,99,0,5);return null;}catch(error){if(error instanceof TypeError||error instanceof WebAssembly.RuntimeError)throw error;return String(error);}},r);
   assert(stopped);assert.equal((await state(r)).active,false);assert((await state(r)).cleanups>0);record.lifecycle.push({renderer:r,stopped,cleanups:(await state(r)).cleanups});
  }
  await page.evaluate(()=>window.hostFixture.ordered_widgets_stop());assert.equal(await page.locator('[id^="ordered-widgets-"]').count(),0);
  for(const r of ['svg','canvas'])for(const mode of [1,2]){
   const rejected=await page.evaluate(([r,mode])=>{try{window.hostFixture.ordered_widgets_mount(r,mode);return null;}catch(error){if(error instanceof TypeError||error instanceof WebAssembly.RuntimeError)throw error;return String(error);}},[r,mode]);assert(rejected);assert.equal(await page.locator(`#ordered-widgets-${r}`).count(),0);record.lifecycle.push({renderer:r,initialRejected:mode,rejected});
  }
  await mount();await page.evaluate(()=>window.hostFixture.ordered_widgets_stop());assert.deepEqual(errors,[]);record.passed=true;console.log(`${name} DPR${dpr}: ordered pixels, native occlusion, identity and rollback passed`);
 }catch(error){record.error=error.stack;await page.screenshot({path:join(out,`${prefix}-failure.png`)}).catch(()=>{});await writeFile(join(out,`${prefix}-failure.html`),await page.content()).catch(()=>{});throw error;}finally{await context.close();}
}}finally{await browser.close();}}
report.passed=report.contexts.length===engines.length*2&&report.contexts.every(c=>c.passed);
}finally{
 server.close();const scriptAtEnd=hash(await readFile(new URL(import.meta.url))),wasmAtEnd=hash(await readFile('examples/browser-host/pkg/host_bg.wasm')),glueAtEnd=hash(await readFile('examples/browser-host/pkg/host.js'));
 report.scriptSha256=scriptAtStart;report.wasmSha256=wasmAtStart;report.glueSha256=glueAtStart;report.provenance={script:{start:scriptAtStart,end:scriptAtEnd},wasm:{start:wasmAtStart,end:wasmAtEnd},glue:{start:glueAtStart,end:glueAtEnd}};
 const unchanged=scriptAtStart===scriptAtEnd&&wasmAtStart===wasmAtEnd&&glueAtStart===glueAtEnd;report.provenance.unchanged=unchanged;if(!unchanged)report.passed=false;
 await writeFile(join(out,'report.json'),JSON.stringify(report,null,2));assert(unchanged,'acceptance script and loaded WASM/glue artifacts must not change during the run');
}
console.log(JSON.stringify({passed:report.passed,contexts:report.contexts.length,out}));
