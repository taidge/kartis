(async page=>{
  const assert=(ok,message)=>{if(!ok)throw Error(message);},errors=[],error=e=>errors.push(e.message);page.on('pageerror',error);
  const flush=async()=>{await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));await page.evaluate(()=>window.hostFixture.parallel_flush());};
  const snapshot=renderer=>page.evaluate(renderer=>JSON.parse(window.hostFixture.parallel_snapshot(renderer)),renderer);
  const panels=await page.evaluate(()=>{const panel=document.querySelector('.panels'),style=panel.style.display;panel.style.display='none';return style;});const baseline=await page.locator('body *').count();const reports=[];
  // Keep inherited language when an inline SVG becomes a standalone image;
  // generic font selection otherwise differs from the live Canvas in Firefox.
  const pixels=()=>page.evaluate(async()=>{const source=document.querySelector('#parallel-svg svg').cloneNode(true);source.setAttribute('width','640');source.setAttribute('height','360');source.setAttributeNS('http://www.w3.org/XML/1998/namespace','xml:lang',document.documentElement.lang||navigator.language);const image=new Image();image.src=`data:image/svg+xml;charset=utf-8,${encodeURIComponent(new XMLSerializer().serializeToString(source))}`;await image.decode();const actual=document.querySelector('#parallel-canvas canvas'),copy=document.createElement('canvas');copy.width=actual.width;copy.height=actual.height;copy.getContext('2d').drawImage(image,0,0,copy.width,copy.height);const a=copy.getContext('2d').getImageData(0,0,copy.width,copy.height).data,b=actual.getContext('2d').getImageData(0,0,actual.width,actual.height).data;let diff=0,ink=0;for(let i=0;i<a.length;i++)diff+=Math.abs(a[i]-b[i]);for(let i=0;i<b.length;i+=4)ink+=Number(Math.min(b[i],b[i+1],b[i+2])<220);return{mean:diff/a.length,ink,backing:[actual.width,actual.height]};});
  // Reproduce a same-frame handoff across the native plot and Glory's
  // separately scheduled control event queue. The initial button hover is real.
  async function hoverOwnership(renderer) {
    const root = page.locator(`#parallel-${renderer}`);
    await root.locator('.kartis-datum').filter({hasText:'Sample B'}).hover();
    await flush();
    const before = await snapshot(renderer);
    assert(before.hovered.includes('parallel:5:row-b'), `${renderer}: control hover did not acquire ownership`);
    const transition = await page.evaluate(renderer => {
      const root = document.querySelector(`#parallel-${renderer}`);
      const surface = root.querySelector('.kartis-surface');
      const plot = surface.querySelector('svg,canvas');
      const buttons = [...root.querySelectorAll('.kartis-datum')];
      const old = buttons.find(button => button.textContent.includes('Sample B'));
      const same = buttons.find(button => button.textContent.includes('Sample <A>'));
      const snapshot = () => JSON.parse(hostFixture.parallel_snapshot(renderer));
      const state = snapshot();
      const rect = surface.getBoundingClientRect();
      const x = rect.left + state.target.x / state.width * rect.width;
      const y = rect.top + state.target.y / state.height * rect.height;
      const send = (element, type, relatedTarget, bubbles = true) => element.dispatchEvent(new PointerEvent(type, {
        pointerId:1, pointerType:'mouse', isPrimary:true, bubbles, clientX:x, clientY:y, relatedTarget,
      }));
      const settle = () => Array.from({length:3}, () => {
        hostFixture.parallel_flush();
        return snapshot().hovered;
      });
      send(old, 'pointerout', plot);
      send(plot, 'pointerover', old);
      send(plot, 'pointermove', null);
      const plotFrames = settle();
      // The corresponding datum button can claim the same business ID after
      // a real plot leave, even though the plot just published that ID.
      send(surface, 'pointerleave', same, false);
      send(same, 'pointerover', plot);
      const controlFrames = settle();
      send(same, 'pointerout', document.body);
      const exitFrames = settle();
      const priorEvents = snapshot().events.length;
      send(old, 'pointerover', document.body);
      send(old, 'pointerout', document.body);
      const staleEnterFrames = settle();
      const staleEnterEvents = snapshot().events.slice(priorEvents);
      return {plotFrames, controlFrames, exitFrames, staleEnterFrames, staleEnterEvents, id:state.target.id};
    }, renderer);
    const {plotFrames, controlFrames, exitFrames, staleEnterFrames, staleEnterEvents, id} = transition;
    assert(plotFrames.every(ids => ids.length === 1 && ids[0] === id),
      `${renderer}: delayed control leave overwrote plot hover ${JSON.stringify(transition)}`);
    assert(controlFrames.at(-1).length === 1 && controlFrames.at(-1)[0] === id && exitFrames.at(-1).length === 0,
      `${renderer}: plot-to-control handoff or control leave lost ownership ${JSON.stringify(transition)}`);
    assert(staleEnterFrames.every(ids => ids.length === 0) && !staleEnterEvents.some(event => event.type === 'hover' && event.ids.length),
      `${renderer}: delayed control enter revived hover after leaving both regions ${JSON.stringify(transition)}`);
    await page.mouse.move(2,2);
    await flush();
    return transition;
  }
  try{
    await page.mouse.move(2,2);
    for(const vertical of [false,true]){
      await page.evaluate(vertical=>window.hostFixture.parallel_mount(vertical),vertical);await flush();const initial=await pixels();assert(initial.mean<.5&&initial.ink>2000,`Parallel ${vertical}: actual SVG/Canvas pixels differ ${JSON.stringify(initial)}`);
      const geometry=await page.evaluate(vertical=>{const root=document.querySelector('#parallel-svg svg'),axis=id=>root.querySelector(`[data-id="parallel-axis:${id.length}:${id}"]`),record=root.querySelector('[data-id="parallel:5:row-a"]'),gap=root.querySelector('[data-id="parallel:7:row-gap"]');const text=[...root.querySelectorAll('text')];const captions=['Cost','Speed','Balance','Quality'];return{axes:captions.every(name=>!!axis(name.toLowerCase())),curve:(record.getAttribute('d').match(/C/g)||[]).length,strokeWidth:Number(record.getAttribute('stroke-width')),dash:record.getAttribute('stroke-dasharray'),opacity:Number(record.getAttribute('opacity')),gapMoves:(gap.getAttribute('d').match(/M/g)||[]).length,gapLines:(gap.getAttribute('d').match(/[LC]/g)||[]).length,captions:captions.every(name=>text.some(t=>t.textContent===name&&Number(t.getAttribute('font-size'))===17)),mask:text.find(t=>t.textContent==='Sample <A>')?.closest('[clip-path]')?.getAttribute('clip-path'),rowTextSafe:root.querySelectorAll('script').length===0&&text.some(t=>t.textContent==='Sample <A>'),quality:axis('quality').getAttribute('d'),vertical};},vertical);
      assert(geometry.axes&&geometry.curve===3&&geometry.strokeWidth===4&&geometry.dash==='12 7'&&geometry.opacity===.8&&geometry.gapMoves===2&&geometry.gapLines===0&&geometry.captions&&geometry.mask&&geometry.rowTextSafe,`Parallel independent axes/curve/gap/style/label-mask geometry failed ${JSON.stringify(geometry)}`);
      const before=await snapshot('svg');await page.evaluate(()=>window.hostFixture.parallel_phase(1));await flush();const changed=await pixels();assert(changed.mean<.5,'Changed parallel geometry SVG/Canvas pixels differ');const after=await snapshot('svg');assert(after.target.id===before.target.id&&after.target.value===30&&after.target.value!==before.target.value&&after.revision>before.revision,'Parallel update changed row identity or retained stale values');
      assert(after.typed.values.join() === [30, 20, 10, Date.UTC(2024, 4, 8)].join() &&
        after.typed.axisDomain.join() === '0,75' && after.typed.dateDomain.join() === [Date.UTC(2024, 1, 28), Date.UTC(2024, 5, 7)].join() &&
        after.typed.positions[0].join() === (vertical ? '264,310' : '60,198') &&
        after.target.label.includes('Quality: 2024-05-08'), 'Parallel typed source, scale, rounding or UTC label changed');
      const hoverHandoff = vertical ? null : [await hoverOwnership('svg'), await hoverOwnership('canvas')];
      for(const renderer of ['svg','canvas']){
        const state=await snapshot(renderer),surface=page.locator(`#parallel-${renderer} .kartis-surface`);await surface.scrollIntoViewIfNeeded();const box=await surface.boundingBox(),x=box.x+state.target.x/state.width*box.width,y=box.y+state.target.y/state.height*box.height;await page.mouse.move(x,y);await flush();assert((await snapshot(renderer)).hovered.includes(state.target.id),`${renderer} parallel ${vertical}: actual painted dash hit missed`);assert((await page.locator(`#parallel-${renderer} .kartis-surface-tooltip`).textContent()).includes('$30.0'),`${renderer}: parallel tooltip did not read current formatted values`);await page.mouse.click(x,y);await flush();assert((await snapshot(renderer)).selected===state.target.id,`${renderer}: parallel shape click missed stable row`);
        const control=page.locator(`#parallel-${renderer} .kartis-datum`).filter({hasText:'Sample <A>'}).first();await control.focus();await page.keyboard.press('Enter');await flush();assert((await snapshot(renderer)).events.some(e=>e.type==='activated'&&e.id===state.target.id),`${renderer}: current parallel row keyboard activation missing`);const legend=page.locator(`#parallel-${renderer} .kartis-legend-item`).filter({hasText:'Alpha group'}).first();await legend.click();await flush();assert((await snapshot(renderer)).hidden.includes('alpha'),`${renderer}: parallel group legend did not hide series`);if(renderer==='svg')assert(await page.evaluate(()=>{const root=document.querySelector('#parallel-svg svg');return!root.querySelector('[data-id="parallel:5:row-a"]')&&![...root.querySelectorAll('text')].some(t=>t.textContent==='Sample <A>')&&[...root.querySelectorAll('text')].some(t=>t.textContent==='Cost');}),'Parallel legend removed shared captions or retained owned endpoint label');
      }
      await page.mouse.move(2,2);await page.evaluate(()=>document.activeElement?.blur());await flush();const accepted=await snapshot('svg'),markup=await page.locator('#parallel-svg svg').first().innerHTML();assert(await page.evaluate(()=>window.hostFixture.parallel_rejected()),'Parallel fixture accepted invalid scene');assert((await snapshot('svg')).revision===accepted.revision,'Rejected parallel scene queued mutation');await flush();assert(await page.locator('#parallel-svg svg').first().innerHTML()===markup,'Rejected parallel scene changed retained geometry');
      await page.evaluate(()=>{window.oldParallel=document.querySelector('#parallel-svg .kartis-datum');window.hostFixture.parallel_stop();});assert(await page.locator('body *').count()===baseline,'Parallel cleanup leaked owned nodes');await page.evaluate(()=>window.oldParallel?.click());await flush();assert(!(await snapshot('svg')).active,'Detached parallel controls revived stopped chart');reports.push({vertical,initial,changed,geometry,typed:after.typed,currentHit:true,currentTooltip:true,keyboard:true,groupLegend:true,hoverHandoff,rejectedAtomic:true,cleanup:true});
    }
    assert(errors.length===0,errors.join('\n'));return{passed:true,orientations:reports,pageErrors:errors};
  }finally{await page.evaluate(previous=>{window.hostFixture.parallel_stop();document.querySelector('.panels').style.display=previous;delete window.oldParallel;},panels);page.off('pageerror',error);}
})
