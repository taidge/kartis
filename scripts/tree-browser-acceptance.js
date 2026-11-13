(async page=>{
  const assert=(ok,message)=>{if(!ok)throw Error(message);},errors=[],onError=e=>errors.push(e.message);page.on('pageerror',onError);
  const flush=async()=>{await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));await page.evaluate(()=>window.hostFixture.tree_flush());};
  const snapshot=renderer=>page.evaluate(renderer=>JSON.parse(window.hostFixture.tree_snapshot(renderer)),renderer);
  const previous=await page.evaluate(()=>{const p=document.querySelector('.panels'),old=p.style.display;p.style.display='none';return old;});
  const baseline=await page.locator('body *').count(),reports=[];
  const compare=()=>page.evaluate(async()=>{
    const liveSvg=document.querySelector('#tree-svg svg'),source=liveSvg.cloneNode(true),live=document.querySelector('#tree-canvas canvas');
    // Image SVGs lose the live document's language inheritance (notably Firefox fonts).
    const languageOwner=liveSvg.closest('[lang]');let language;
    for(let node=liveSvg;node;node=node.parentElement){const xml=node.getAttributeNS('http://www.w3.org/XML/1998/namespace','lang')??node.getAttribute('xml:lang');if(xml!==null){language=xml;break;}if(node===languageOwner){language=node.getAttribute('lang');break;}}
    source.setAttribute('lang',language??liveSvg.ownerDocument.documentElement.lang);source.setAttribute('width',String(live.width));source.setAttribute('height',String(live.height));
    const image=new Image();image.src='data:image/svg+xml;charset=utf-8,'+encodeURIComponent(new XMLSerializer().serializeToString(source));await image.decode();
    const copy=document.createElement('canvas');copy.width=live.width;copy.height=live.height;const ctx=copy.getContext('2d');ctx.drawImage(image,0,0);
    const a=ctx.getImageData(0,0,copy.width,copy.height).data,b=live.getContext('2d').getImageData(0,0,live.width,live.height).data;
    let difference=0,ink=0;for(let i=0;i<a.length;i++)difference+=Math.abs(a[i]-b[i]);for(let i=0;i<b.length;i+=4)ink+=Number(Math.min(b[i],b[i+1],b[i+2])<220);
    const target=JSON.parse(window.hostFixture.tree_snapshot('canvas')).target,pixel=(data,x,y)=>{const i=(Math.floor(y*live.height/360)*live.width+Math.floor(x*live.width/640))*4;return [...data.slice(i,i+4)];};
    return{meanChannelError:difference/a.length,ink,backing:[live.width,live.height],svgCenter:pixel(a,target.x,target.y),canvasCenter:pixel(b,target.x,target.y),svgCorner:pixel(a,target.x+8,target.y+8),canvasCorner:pixel(b,target.x+8,target.y+8)};
  });
  try{
    await page.mouse.move(2,2);await page.evaluate(()=>window.hostFixture.tree_mount());await flush();
    assert(await page.locator('#tree-svg .kartis-legend-item').count()===0,'Tree unexpectedly generated a legend');
    await page.evaluate(()=>{const root=document.querySelector('#tree-svg svg');window.treeProbe={root,paths:[...root.querySelectorAll('path[data-id]')]};});
    for(const phase of [0,1,2]){
      if(phase){await page.evaluate(phase=>window.hostFixture.tree_phase(phase),phase);await flush();}
      const pixels=await compare();assert(pixels.meanChannelError<.75&&pixels.ink>1500,`Tree phase ${phase}: SVG/Canvas parity ${JSON.stringify(pixels)}`);
      const expected=phase===1?'40,115,190,255':'40,155,100,255';assert(pixels.svgCenter.join(',')===expected&&pixels.canvasCenter.join(',')===expected,`Tree phase ${phase}: node fill absent`);
      if(phase===0)assert(pixels.svgCorner.join(',')===expected&&pixels.canvasCorner.join(',')===expected,'Square leaf corners were not painted');
      if(phase===1)assert(pixels.svgCorner.join(',')==='255,255,255,255'&&pixels.canvasCorner.join(',')==='255,255,255,255','Circle update retained square corners');
      const geometry=await page.evaluate(()=>{
        const r=document.querySelector('#tree-svg svg'),node=r.querySelector('[data-id="node:leaf-a"]'),internal=r.querySelector('[data-id="node:group-a"]'),links=[...r.querySelectorAll('path[data-id^="link:"]')],labels=[...r.querySelectorAll('text')];
        return{nodes:r.querySelectorAll('path[data-id^="node:"]').length,links:links.length,labelCount:labels.length,leafPath:node.getAttribute('d'),internalPath:internal.getAttribute('d'),linkPath:links.find(p=>p.dataset.id==='link:leaf-a').getAttribute('d'),dash:links[0].getAttribute('stroke-dasharray'),width:Number(links[0].getAttribute('stroke-width')),rotated:labels.filter(t=>(t.getAttribute('transform')??'').includes('rotate(')).length,mask:node.parentElement.tagName.toLowerCase()==='svg'&&node.parentElement.getAttribute('overflow')==='hidden',safeText:labels.some(t=>t.textContent==='<Leaf A>')&&r.querySelectorAll('script').length===0,identity:r===window.treeProbe.root&&window.treeProbe.paths.every(p=>r.querySelector(`[data-id="${p.dataset.id}"]`)===p)};
      });
      assert(geometry.nodes===7&&geometry.links===6&&geometry.labelCount===(phase===0?4:7)&&geometry.safeText&&geometry.mask&&geometry.identity,`Tree phase ${phase}: topology, clipping, labels, stable nodes ${JSON.stringify(geometry)}`);
      assert(geometry.width===(phase===1?4:3)&&geometry.dash===(phase===1?null:'10 7'),'Tree link style update retained stale attributes');
      if(phase===0)assert(!geometry.linkPath.includes('C')&&(geometry.internalPath.match(/L/g)||[]).length===3,'Cartesian StepBefore/internal Diamond geometry missing');
      if(phase===1)assert(geometry.linkPath.includes('C')&&geometry.leafPath.includes('C')&&geometry.rotated===7,'Bump/circle/rotated label update missing');
      if(phase===2)assert(geometry.linkPath.includes('C')&&geometry.rotated>=4,'Radial step arcs/AlongLayout labels missing');
      for(const renderer of ['svg','canvas']){
        const state=await snapshot(renderer),surface=page.locator(`#tree-${renderer} .kartis-surface`);assert(state.sceneHit===state.target.id,'Pure tree scene hit missed leaf');
        await surface.scrollIntoViewIfNeeded();const box=await surface.boundingBox(),x=box.x+state.target.x/state.width*box.width,y=box.y+state.target.y/state.height*box.height;
        await page.mouse.move(x,y);await flush();assert((await snapshot(renderer)).hovered.includes(state.target.id),`${renderer} phase${phase}: pointer hit missed leaf`);
        const tooltip=await page.locator(`#tree-${renderer} .kartis-surface-tooltip`).textContent();assert(tooltip.includes('<Leaf A>')&&tooltip.includes('10'),`${renderer}: current tree tooltip missing`);
        await page.mouse.click(x,y);await flush();assert((await snapshot(renderer)).selected===state.target.id,`${renderer}: tree click missed leaf`);
        const control=page.locator(`#tree-${renderer} .kartis-datum`).filter({hasText:'<Leaf A>'}).first();await control.focus();await page.keyboard.press('Enter');await flush();assert((await snapshot(renderer)).events.some(e=>e.type==='activated'&&e.id===state.target.id),`${renderer}: tree keyboard activation missing`);
      }
      // An identical accepted frame must preserve keyed paths without a DOM write.
      await page.mouse.move(2,2);await page.evaluate(()=>document.activeElement?.blur());await flush();
      await page.evaluate(phase=>{const root=document.querySelector('#tree-svg svg');window.treeProbe.mutations=0;window.treeProbe.observer=new MutationObserver(records=>window.treeProbe.mutations+=records.length);window.treeProbe.observer.observe(root,{subtree:true,attributes:true,childList:true,characterData:true});window.hostFixture.tree_phase(phase);window.hostFixture.tree_flush();},phase);
      await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(resolve)));assert(await page.evaluate(()=>window.treeProbe.mutations===0),`Tree phase${phase}: identical frame mutated DOM`);await page.evaluate(()=>window.treeProbe.observer.disconnect());
      const accepted=await snapshot('svg'),markup=await page.locator('#tree-svg svg').first().innerHTML();assert(await page.evaluate(()=>window.hostFixture.tree_rejected()),'Bad Tree configuration/scene accepted');assert((await snapshot('svg')).revision===accepted.revision&&(await snapshot('svg')).pending===0,'Bad Tree source queued mutation');await flush();assert(await page.locator('#tree-svg svg').first().innerHTML()===markup,'Rejected Tree changed accepted DOM');
      reports.push({phase,pixels,geometry,pointer:true,keyboard:true,rejectedAtomic:true,unchangedMutationZero:true});
    }
    for(const renderer of ['svg','canvas']){await page.evaluate(renderer=>window.hostFixture.tree_hidden(renderer,true),renderer);await flush();assert((await snapshot(renderer)).hidden.includes('leaf-a'),`${renderer}: explicit hidden owner missing`);assert(await page.locator(`#tree-${renderer} .kartis-datum`).filter({hasText:'<Leaf A>'}).count()===0,`${renderer}: hidden node control retained`);}
    assert(await page.evaluate(()=>{const root=document.querySelector('#tree-svg svg');return !root.querySelector('[data-id="node:leaf-a"]')&&![...root.querySelectorAll('text')].some(t=>t.textContent==='<Leaf A>')&&!!root.querySelector('[data-id="node:leaf-b"]')&&!!root.querySelector('[data-id="link:leaf-a"]');}),'Hiding tree owner retained its node/label or removed shared links/siblings');
    const hidden=await compare();assert(hidden.meanChannelError<.75,'Hidden tree SVG/Canvas parity failed');
    await page.evaluate(()=>{window.oldTree=document.querySelector('#tree-svg .kartis-datum');window.hostFixture.tree_stop();});assert(await page.locator('body *').count()===baseline,'Tree Stop leaked owned DOM');await page.evaluate(()=>window.oldTree?.click());await flush();assert(!(await snapshot('svg')).active,'Detached tree control revived stopped chart');assert(errors.length===0,errors.join('\n'));
    return{passed:true,phases:reports,hiddenOwnership:true,hiddenPixels:hidden,cleanup:true,pageErrors:errors};
  }finally{await page.evaluate(previous=>{window.treeProbe?.observer?.disconnect();window.hostFixture.tree_stop();document.querySelector('.panels').style.display=previous;delete window.treeProbe;delete window.oldTree;},previous);page.off('pageerror',onError);}
})
