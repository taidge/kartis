(async function verifySvgIsolation(page){
 const assert=(ok,message)=>{if(!ok)throw Error(message);},errors=[],onError=e=>errors.push(e.message);page.on('pageerror',onError);
 const snapshot=name=>page.evaluate(name=>JSON.parse(window.hostFixture.svg_isolation_snapshot(name)),name);
 const flush=async()=>{await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));await page.evaluate(()=>window.hostFixture.svg_isolation_flush());};
 const clear=async()=>{await page.mouse.move(2,2);await page.evaluate(()=>document.activeElement?.blur());await flush();};
 const panels=await page.evaluate(()=>{const p=document.querySelector('.panels'),old=p.style.display;p.style.display='none';return old;}),baseline=await page.locator('body *').count();
 const definitions=()=>page.evaluate(()=>['first','second'].map(name=>{const root=document.querySelector(`#isolation-${name} .kartis-surface>svg`);if(!root)return{name,active:false};const defs=[...root.querySelectorAll('defs [id]')].map(e=>({id:e.id,tag:e.tagName})),references=[...root.querySelectorAll('[fill],[clip-path]')].flatMap(e=>['fill','clip-path'].flatMap(a=>{const match=e.getAttribute(a)?.match(/^url\(#([^)]*)\)$/);return match?[match[1]]:[]}));return{name,active:true,root:root.id,defs,references,dataIds:[...root.querySelectorAll('[data-id]')].map(e=>e.dataset.id)};}));
 const checkDefinitions=info=>{
  const ids=info.filter(x=>x.active).flatMap(x=>x.defs.map(d=>d.id));assert(new Set(ids).size===ids.length,'Independent SVG owners share definition IDs');
  for(const x of info.filter(x=>x.active)){assert(x.root&&x.defs.every(d=>d.id.startsWith(x.root+'-')),'Definition ID lacks a stable owner namespace');assert(x.references.every(id=>x.defs.some(d=>d.id===id)),'Paint/label/mark mask URL escapes its local owner');assert(x.dataIds.join(',')==='linear,radial,pattern,masked','Owner namespace changed user datum identities');}
 };
 // This measures the live composited SVG, including cross-owner URL lookup.
 // SVG-as-image serialization would isolate IDs and conceal the original bug.
 const measure=async(phase)=>{
  await clear();const surface=page.locator('#isolation-second .kartis-surface');await surface.scrollIntoViewIfNeeded();const png=await surface.screenshot();
  return page.evaluate(async({uri,phase})=>{
   const surface=document.querySelector('#isolation-second .kartis-surface'),svg=surface.querySelector(':scope>svg'),sr=surface.getBoundingClientRect(),vr=svg.getBoundingClientRect();const image=new Image();image.src=uri;await image.decode();
   const copy=document.createElement('canvas');copy.width=image.width;copy.height=image.height;const ctx=copy.getContext('2d');ctx.drawImage(image,0,0);const a=ctx.getImageData(0,0,copy.width,copy.height).data;
   const canvas=document.querySelector('#isolation-canvas canvas'),b=canvas.getContext('2d').getImageData(0,0,canvas.width,canvas.height).data;
   const actual=(x,y)=>{const px=Math.floor((vr.left-sr.left+x/600*vr.width)*copy.width/sr.width),py=Math.floor((vr.top-sr.top+y/260*vr.height)*copy.height/sr.height),i=(py*copy.width+px)*4;return Array.from(a.slice(i,i+4));};
   const expected=(x,y)=>{const i=(Math.floor(y/260*canvas.height)*canvas.width+Math.floor(x/600*canvas.width))*4;return Array.from(b.slice(i,i+4));};
   // A radial gradient's center is a distance-function cusp: a subpixel
   // screenshot offset changes its first pixel much more than a smooth area.
   // Sample 20px from the center and 30px from the boundary without changing
   // the fixed channel tolerance. Keep the cusp measurement as diagnostics.
   const samples=[[80,70],[290,75],[100,188],[100,151],[30,180],[402,66],[396,60]].map(([x,y])=>({point:[x,y],actual:actual(x,y),expected:expected(x,y)}));
   const radialCenterDiagnostic={actual:actual(270,75),expected:expected(270,75),
    surfaceCss:[sr.x,sr.y,sr.width,sr.height],svgCss:[vr.x,vr.y,vr.width,vr.height],
    screenshotDevicePixel:[Math.floor((vr.left-sr.left+270/600*vr.width)*copy.width/sr.width),Math.floor((vr.top-sr.top+75/260*vr.height)*copy.height/sr.height)],
    canvasDevicePixel:[Math.floor(270/600*canvas.width),Math.floor(75/260*canvas.height)]};
   const dark=([r,g,b])=>Math.min(r,g,b)<200;let inside=0,outside=0;
   for(let y=168;y<208;y++)for(let x=350;x<580;x++){if(x>392&&x<478){inside+=Number(dark(actual(x+.37,y+.29)));}else if(x<388||x>482){outside+=Number(dark(actual(x+.37,y+.29)));}}
   return{samples,radialCenterDiagnostic,labelInsideInk:inside,labelOutsideInk:outside,phase,screenshot:[copy.width,copy.height],backing:[canvas.width,canvas.height]};
  },{uri:'data:image/png;base64,'+png.toString('base64'),phase});
 };
 const checkPixels=(pixels,stage)=>{
  assert(pixels.samples.every(s=>s.actual.length===4&&s.expected.every((v,i)=>Math.abs(v-s.actual[i])<=2)),`${stage}: live SVG no longer matches Canvas interior paints ${JSON.stringify(pixels)}`);
  assert(pixels.labelInsideInk>50,`${stage}: clipped label lost live glyph ink`);
  if(pixels.phase!==2)assert(pixels.labelOutsideInk===0,`${stage}: label mask leaked outside its actual clip`);
 };
 try{
  await clear();await page.evaluate(()=>window.hostFixture.svg_isolation_mount());await flush();
  const initialDefinitions=await definitions();checkDefinitions(initialDefinitions);assert(initialDefinitions.every(d=>d.defs.length===5),'Fixture must cover linear/radial/pattern and mark/label masks');
  const initial=await measure(0);checkPixels(initial,'both visible');
  await page.evaluate(()=>window.hostFixture.svg_isolation_first(0));await flush();const hidden=await measure(0);checkPixels(hidden,'first owner hidden');assert((await snapshot('first')).active,'Hiding the first owner disposed it');
  await page.evaluate(()=>window.hostFixture.svg_isolation_first(1));await flush();assert(JSON.stringify(await definitions())===JSON.stringify(initialDefinitions),'Show/hide replaced stable owner or definitions');
  await page.evaluate(()=>window.hostFixture.svg_isolation_first(3));await flush();const dropped=await measure(0);checkPixels(dropped,'first owner dropped');assert(!(await snapshot('first')).active,'Dropped first guard stayed active');
  assert(await page.evaluate(ids=>ids.every(id=>!document.getElementById(id)),initialDefinitions[0].defs.map(d=>d.id)),'Dropped owner retained document definitions');
  await page.evaluate(()=>window.hostFixture.svg_isolation_first(4));await flush();const remounted=await definitions();checkDefinitions(remounted);assert(remounted[0].root!==initialDefinitions[0].root&&remounted[1].root===initialDefinitions[1].root,'Remount reused a retired namespace or replaced the surviving owner');
  await page.evaluate(()=>window.hostFixture.svg_isolation_first(2));await flush();const stopped=await measure(0);checkPixels(stopped,'first owner stopped');assert(!(await snapshot('first')).active,'Explicit stop left first owner active');
  assert(await page.evaluate(ids=>ids.every(id=>!document.getElementById(id)),remounted[0].defs.map(d=>d.id)),'Stopped owner retained shared URL definitions');
  await page.evaluate(()=>{const root=document.querySelector('#isolation-second .kartis-surface>svg');window.svgIsolationProbe={root,marks:[...root.querySelectorAll('[data-id]')],label:root.querySelector('text'),radial:root.querySelector('radialGradient')};});
  await page.evaluate(()=>window.hostFixture.svg_isolation_phase(1));await flush();const changed=await measure(1);checkPixels(changed,'changed paints');const changedDefinitions=await definitions();checkDefinitions(changedDefinitions);
  const retired=initialDefinitions[1].defs.map(d=>d.id).filter(id=>!changedDefinitions[1].defs.some(d=>d.id===id));assert(retired.length===2&&await page.evaluate(ids=>ids.every(id=>!document.getElementById(id)),retired),'Changed gradient/pattern definitions were not retired');
  assert(await page.evaluate(()=>{const p=window.svgIsolationProbe,r=document.querySelector('#isolation-second .kartis-surface>svg');return r===p.root&&[...r.querySelectorAll('[data-id]')].every((m,i)=>m===p.marks[i])&&r.querySelector('text')===p.label&&r.querySelector('radialGradient')===p.radial;}),'Namespace update recreated retained root/data/label/unchanged definitions');
  await page.evaluate(()=>window.hostFixture.svg_isolation_phase(2));await flush();const solid=await measure(2);checkPixels(solid,'solid/mask removal');const emptyDefs=await definitions();assert(emptyDefs.filter(d=>d.active).every(d=>d.defs.length===0&&d.references.length===0),'Solid update retained old paint or clip URL definitions');
  await page.evaluate(()=>window.hostFixture.svg_isolation_phase(0));await flush();checkDefinitions(await definitions());
  await clear();await page.evaluate(()=>{const p=window.svgIsolationProbe;p.count=0;p.observer=new MutationObserver(records=>p.count+=records.length);p.observer.observe(p.root,{subtree:true,attributes:true,childList:true,characterData:true});window.hostFixture.svg_isolation_phase(0);window.hostFixture.svg_isolation_flush();});await flush();assert(await page.evaluate(()=>window.svgIsolationProbe.count===0),'Identical Scene changed retained namespace/definitions/marks');await page.evaluate(()=>window.svgIsolationProbe.observer.disconnect());
  for(const name of ['second','canvas']){
   const surface=page.locator(`#isolation-${name} .kartis-surface`);await surface.scrollIntoViewIfNeeded();const box=await surface.boundingBox();await page.mouse.move(box.x+80/600*box.width,box.y+70/260*box.height);await flush();assert((await snapshot(name)).hovered.includes('linear'),'Namespaced owner lost actual pointer hit');assert(await page.locator(`#isolation-${name} .kartis-surface-tooltip`).textContent()==='linear <safe>','Namespacing changed safe user tooltip text');
   await page.mouse.click(box.x+80/600*box.width,box.y+70/260*box.height);await flush();assert((await snapshot(name)).selected==='linear','Namespacing changed pointer datum identity');const control=page.locator(`#isolation-${name} .kartis-datum`).filter({hasText:'linear <safe>'}).first();await control.focus();await page.keyboard.press('Enter');await flush();assert((await snapshot(name)).events.some(e=>e.type==='activated'&&e.id==='linear'),'Namespacing broke actual keyboard activation');await surface.press('Escape');
  }
  await clear();const accepted=await snapshot('second'),markup=await page.locator('#isolation-second .kartis-surface>svg').innerHTML(),canvas=await page.evaluate(()=>document.querySelector('#isolation-canvas canvas').toDataURL());assert(await page.evaluate(()=>window.hostFixture.svg_isolation_rejected()),'Invalid namespaced Scene accepted');assert((await snapshot('second')).revision===accepted.revision,'Invalid namespaced Scene changed accepted state');await flush();assert(await page.locator('#isolation-second .kartis-surface>svg').innerHTML()===markup&&await page.evaluate(()=>document.querySelector('#isolation-canvas canvas').toDataURL())===canvas,'Rejected update changed retained SVG/Canvas frames');
  const surviving=(await definitions()).filter(d=>d.active).flatMap(d=>d.defs.map(d=>d.id));await page.evaluate(()=>{window.oldSvgIsolation=document.querySelector('#isolation-second .kartis-datum');window.hostFixture.svg_isolation_stop();});assert(await page.locator('body *').count()===baseline,'Isolation owner cleanup retained DOM');assert(await page.evaluate(ids=>ids.every(id=>!document.getElementById(id)),surviving),'Cleanup leaked namespaced definitions');await page.evaluate(()=>window.oldSvgIsolation?.click());assert(!(await snapshot('second')).active,'Detached control revived disposed owner');assert(!errors.length,errors.join('\n'));
  return{passed:true,initial,hidden,dropped,stopped,changed,solid,uniqueOwnerIds:true,allPaintAndMaskReferencesLocal:true,retainedIdentity:true,unchangedMutationZero:true,definitionsRetired:true,pointerKeyboard:true,rejectedAtomic:true,cleanup:true,pageErrors:errors};
 }finally{await page.evaluate(panels=>{window.svgIsolationProbe?.observer?.disconnect();window.hostFixture.svg_isolation_stop();document.querySelector('.panels').style.display=panels;delete window.svgIsolationProbe;delete window.oldSvgIsolation;},panels);page.off('pageerror',onError);}
})
