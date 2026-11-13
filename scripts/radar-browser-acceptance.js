(async page=>{
 const assert=(condition,message)=>{if(!condition)throw Error(message)},errors=[],onError=e=>errors.push(e.message);page.on('pageerror',onError);
 const flush=async()=>{await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));await page.evaluate(()=>window.hostFixture.radar_flush());};
 const snapshot=renderer=>page.evaluate(renderer=>JSON.parse(window.hostFixture.radar_snapshot(renderer)),renderer);
 const previous=await page.evaluate(()=>{const panel=document.querySelector('.panels'),old=panel.style.display;panel.style.display='none';return old});const baseline=await page.locator('body *').count(),phases=[];
 const pixels=()=>page.evaluate(async()=>{
  const live=document.querySelector('#radar-svg svg'),svg=live.cloneNode(true),language=live.closest('[lang]')?.getAttribute('lang')||live.closest('[xml\\:lang]')?.getAttribute('xml:lang')||document.documentElement.lang||navigator.language;
  svg.setAttribute('width','640');svg.setAttribute('height','400');svg.setAttribute('lang',language);svg.setAttributeNS('http://www.w3.org/XML/1998/namespace','xml:lang',language);
  const image=new Image();image.src=`data:image/svg+xml;charset=utf-8,${encodeURIComponent(new XMLSerializer().serializeToString(svg))}`;await image.decode();
  const actual=document.querySelector('#radar-canvas canvas'),copy=document.createElement('canvas');copy.width=actual.width;copy.height=actual.height;copy.getContext('2d').drawImage(image,0,0,copy.width,copy.height);
  const a=copy.getContext('2d').getImageData(0,0,copy.width,copy.height).data,b=actual.getContext('2d').getImageData(0,0,actual.width,actual.height).data;let difference=0,ink=0;for(let i=0;i<a.length;i++)difference+=Math.abs(a[i]-b[i]);for(let i=0;i<b.length;i+=4)ink+=Number(Math.min(b[i],b[i+1],b[i+2])<220);return{mean:difference/a.length,ink,backing:[actual.width,actual.height]};
 });
 try {
  await page.mouse.move(2,2);await page.evaluate(()=>window.hostFixture.radar_mount());await flush();
  for(const phase of [0,1]) {
   if(phase){await page.evaluate(phase=>window.hostFixture.radar_phase(phase),phase);await flush();}
   // Compare the same neutral presentation state after the previous phase's
   // pointer, legend and keyboard work, including scroll-driven pointer enters.
   for(const renderer of ['svg','canvas'])await page.locator(`#radar-${renderer} .kartis-surface`).press('Escape');
   await page.evaluate(()=>document.activeElement?.blur());await flush();
   const comparison=await pixels();assert(comparison.mean<.5&&comparison.ink>6000,`Radar SVG/Canvas pixels ${JSON.stringify(comparison)}`);
   const geometry=await page.evaluate(phase=>{
    const svg=document.querySelector('#radar-svg svg'),series=svg.querySelector('[data-id="radar-series:5:alpha"]'),grid=svg.querySelector('[data-id="radar-grid:0"]'),point=svg.querySelector('[data-id="radar-point:5:alpha:5:speed"]');
    return{closed:series.getAttribute('d').endsWith('Z'),cubics:(series.getAttribute('d').match(/C/g)||[]).length,gridCubics:(grid.getAttribute('d').match(/C/g)||[]).length,fill:series.getAttribute('fill'),stroke:series.getAttribute('stroke'),opacity:series.getAttribute('opacity'),width:series.getAttribute('stroke-width'),dash:series.getAttribute('stroke-dasharray'),pointLines:(point.getAttribute('d').match(/L/g)||[]).length,spokes:svg.querySelectorAll('[data-id^="radar-spoke:"]').length,axis:[...svg.querySelectorAll('text')].some(t=>t.textContent==='Speed <safe>'&&t.getAttribute('font-size')==='17'),safe:svg.querySelectorAll('script').length===0,phase};
   },phase);
   assert(geometry.closed&&geometry.cubics===5&&geometry.gridCubics===(phase?0:4)&&geometry.fill.endsWith(',0.1803921568627451)')&&geometry.stroke.endsWith(',1)')&&geometry.opacity==='1'&&geometry.width==='4'&&geometry.dash==='10 4'&&geometry.pointLines===3&&geometry.spokes===5&&geometry.axis&&geometry.safe,`Configured radar geometry/style ${JSON.stringify(geometry)}`);
   for(const renderer of ['svg','canvas']) {
    const state=await snapshot(renderer),surface=page.locator(`#radar-${renderer} .kartis-surface`);assert(state.target.value===(phase?55:80),'Radar source update retained stale value');await surface.scrollIntoViewIfNeeded();
    // At phase1 the two north diamonds overlap. Aim at Alpha's exposed top
    // half, rather than demanding a hit through the later painted Beta glyph.
    const box=await surface.boundingBox(),x=box.x+state.target.x/state.width*box.width,y=box.y+(state.target.y-(phase?8:0))/state.height*box.height;
    await page.mouse.move(x,y);await flush();assert((await snapshot(renderer)).hovered.includes(state.target.id),`${renderer} radar did not pick actual glyph`);assert((await page.locator(`#radar-${renderer} .kartis-surface-tooltip`).textContent()).includes(`${phase?55:80}%`),`${renderer} radar tooltip retained stale/formatted value`);
    await page.mouse.click(x,y);await flush();assert((await snapshot(renderer)).selected===state.target.id,`${renderer} radar click selected wrong point`);
    const control=page.locator(`#radar-${renderer} .kartis-datum`).filter({hasText:'Alpha <A> · Speed <safe>:'}).first();await control.focus();await page.keyboard.press('Enter');await flush();assert((await snapshot(renderer)).events.some(e=>e.event.includes('Activated')&&e.event.includes(state.target.id)),`${renderer} radar keyboard did not activate current datum`);
    const before=renderer==='svg'?await page.locator('#radar-svg svg text').count():0;const legend=page.locator(`#radar-${renderer} .kartis-legend-item`).filter({hasText:'Alpha <A>'}).first();await legend.click();await flush();assert((await snapshot(renderer)).hidden.includes('alpha'),`${renderer} radar legend did not hide alpha`);
    if(renderer==='svg'){assert(!await page.locator('#radar-svg [data-series="alpha"]').count(),'Hidden radar series geometry retained');assert(await page.locator('#radar-svg svg text').count()===before-5,'Hidden radar retained owned point labels');assert(await page.locator('#radar-svg svg text').filter({hasText:'Speed <safe>'}).count()===1,'Hidden radar removed shared axis label');}
    await legend.click();await flush();assert(!(await snapshot(renderer)).hidden.includes('alpha'),'Radar legend did not restore series');
   }
   await page.mouse.move(2,2);await page.evaluate(()=>document.activeElement?.blur());await flush();
   await page.evaluate(phase=>{window.radarMutations=0;window.radarObserver=new MutationObserver(records=>window.radarMutations+=records.length);window.radarObserver.observe(document.querySelector('#radar-svg svg'),{subtree:true,attributes:true,childList:true,characterData:true});window.hostFixture.radar_phase(phase);window.hostFixture.radar_flush();},phase);await page.evaluate(()=>new Promise(requestAnimationFrame));const mutations=await page.evaluate(()=>{window.radarObserver.disconnect();return window.radarMutations});assert(mutations===0,`Unchanged radar mutated SVG ${mutations} times`);
   const accepted=await snapshot('svg'),markup=await page.locator('#radar-svg svg').innerHTML();assert(await page.evaluate(()=>window.hostFixture.radar_rejected()),'Invalid radar source/scene was admitted');assert((await snapshot('svg')).revision===accepted.revision,'Rejected radar queued state');await flush();assert(await page.locator('#radar-svg svg').innerHTML()===markup,'Rejected radar altered actual retained SVG');
   phases.push({phase,pixels:comparison,geometry,currentPicking:true,currentTooltip:true,legendAndOwnedLabels:true,keyboard:true,unchangedMutationZero:true,rejectedAtomic:true});
  }
  await page.evaluate(()=>{window.oldRadar=document.querySelector('#radar-svg .kartis-datum');window.hostFixture.radar_stop()});assert(await page.locator('body *').count()===baseline,'Radar stop leaked owned DOM');await page.evaluate(()=>window.oldRadar?.click());await flush();assert(!(await snapshot('svg')).active,'Stale radar input revived chart');assert(!errors.length,errors.join('\n'));
  return{passed:true,phases,cleanup:true,staleListenerCleanup:true,pageErrors:errors};
 } finally {await page.evaluate(old=>{window.hostFixture.radar_stop();document.querySelector('.panels').style.display=old;window.radarObserver?.disconnect();delete window.oldRadar;delete window.radarObserver;delete window.radarMutations;},previous);page.off('pageerror',onError);}
})
