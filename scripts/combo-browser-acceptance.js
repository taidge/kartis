(async page=>{
  const assert=(ok,message)=>{if(!ok)throw Error(message);};
  const errors=[],listener=e=>errors.push(e.message);page.on('pageerror',listener);
  const snapshot=id=>page.evaluate(id=>JSON.parse(window.hostFixture.combo_styles_snapshot(id)),id);
  const flush=async()=>{await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));await page.evaluate(()=>window.hostFixture.combo_styles_flush());};
  const reports=[];
  const panels=await page.evaluate(()=>{const e=document.querySelector('.panels');const previous=e.style.display;e.style.display='none';return previous;});
  const baseline=await page.locator('body *').count();
  const pixels=()=>page.evaluate(async()=>{
    const liveSvg=document.querySelector('#combo-svg svg'),svg=liveSvg.cloneNode(true);
    // Standalone SVG images do not retain the live tree's inherited language.
    const languageOwner=liveSvg.closest('[lang]');
    let language;
    for(let node=liveSvg;node;node=node.parentElement){
      const xmlLanguage=node.getAttributeNS('http://www.w3.org/XML/1998/namespace','lang')??node.getAttribute('xml:lang');
      if(xmlLanguage!==null){language=xmlLanguage;break;}
      if(node===languageOwner){language=node.getAttribute('lang');break;}
    }
    svg.setAttribute('lang',language??liveSvg.ownerDocument.documentElement.lang);
    svg.setAttribute('width','640');svg.setAttribute('height','360');
    const image=new Image();image.src=`data:image/svg+xml;charset=utf-8,${encodeURIComponent(new XMLSerializer().serializeToString(svg))}`;await image.decode();
    const actual=document.querySelector('#combo-canvas canvas'),copy=document.createElement('canvas');copy.width=actual.width;copy.height=actual.height;copy.getContext('2d').drawImage(image,0,0,copy.width,copy.height);
    const a=copy.getContext('2d').getImageData(0,0,copy.width,copy.height).data,b=actual.getContext('2d').getImageData(0,0,actual.width,actual.height).data;let difference=0,ink=0;for(let i=0;i<a.length;i++)difference+=Math.abs(a[i]-b[i]);for(let i=0;i<b.length;i+=4)ink+=Number(Math.min(b[i],b[i+1],b[i+2])<220&&b[i+3]>0);
    return{mean:difference/a.length,ink,backing:[actual.width,actual.height]};
  });
  try{
    await page.mouse.move(2,2);
    for(let kind=0;kind<3;kind++){
      await page.evaluate(kind=>window.hostFixture.combo_styles_mount(kind),kind);await flush();
      const initial=await pixels();assert(initial.mean<0.5&&initial.ink>2000,`combo${kind}: SVG/Canvas actual pixels diverged ${JSON.stringify(initial)}`);
      const geometry=await page.evaluate(kind=>{
        const root=document.querySelector('#combo-svg svg'),state=JSON.parse(window.hostFixture.combo_styles_snapshot('svg'));
        const datum=root.querySelector(`[data-id="${state.target.id}"]`),box=datum.getBBox();
        const label=[...root.querySelectorAll('text')].find(t=>t.textContent===state.target.value.toFixed(1));
        const shared=[...root.querySelectorAll('path[data-id]')].filter(p=>p.dataset.id==='combo-axis-left'||p.dataset.id==='combo-axis-right').length===(kind===2?1:2);
        const styled=label&&Number(label.getAttribute('font-size'))===17&&label.getAttribute('fill')==='rgba(40,80,150,1)';
        if(kind===0){const bar=root.querySelector('path[data-series="Bars"]').getBBox(),point=root.querySelector('path[data-series="Line"][data-id*="datum"]').getBBox();return{shared,styled,boxX:box.x+box.width/2,barX:bar.x+bar.width/2,lineX:point.x+point.width/2,boxWidth:box.width,boxHeight:box.height,medianWidth:Number(root.querySelector(`[data-id="${state.target.id}:median"]`).getAttribute('stroke-width')),whiskerWidth:Number(root.querySelector(`[data-id="${state.target.id}:whiskers"]`).getAttribute('stroke-width'))};}
        if(kind===1)return{shared,styled,caps:(datum.getAttribute('d').match(/M/g)||[]).length,width:Number(datum.getAttribute('stroke-width')),scatterRadius:root.querySelector('path[data-series="Scatter"]').getBBox().width/2};
        const bands=[...root.querySelectorAll('path[data-series="Range"][data-id*="band:"]')];return{shared,styled,bands:bands.length,closed:bands.every(p=>(p.getAttribute('d').match(/M/g)||[]).length===1&&p.getAttribute('d').endsWith('Z')),opacities:bands.map(p=>Number(p.getAttribute('opacity'))),radius:box.width/2};
      },kind);
      assert(geometry.shared&&geometry.styled,`combo${kind}: common axes or actual text style missing ${JSON.stringify(geometry)}`);
      if(kind===0)assert(Math.abs(geometry.boxX-179)<.001&&Math.abs(geometry.barX-111)<.001&&Math.abs(geometry.lineX-145)<.001&&Math.abs(geometry.boxWidth-68)<.001&&Math.abs(geometry.boxHeight-112)<.001&&geometry.medianWidth===5&&geometry.whiskerWidth===3,'Box/bar grouping or dual-axis summary projection failed');
      if(kind===1)assert(geometry.caps===3&&geometry.width===4&&geometry.scatterRadius===8,'Error whisker caps or per-series scatter size failed');
      if(kind===2)assert(geometry.bands===2&&geometry.closed&&geometry.opacities.every(o=>Math.abs(o-.126)<1e-8)&&geometry.radius===6,'Range gaps, bounded envelopes or series area/point style failed');
      const before=await snapshot('svg');await page.evaluate(()=>window.hostFixture.combo_styles_phase(1));await flush();const changed=await pixels();assert(changed.mean<.5,`combo${kind}: changed SVG/Canvas pixels diverged ${changed.mean}`);
      const current=await snapshot('svg');assert(current.revision>before.revision&&current.target.id===before.target.id&&current.target.value!==before.target.value,'Changed geometry did not preserve current stable datum identity');
      for(const renderer of ['svg','canvas']){
        const s=await snapshot(renderer),surface=page.locator(`#combo-${renderer} .kartis-surface`);await surface.scrollIntoViewIfNeeded();const bounds=await surface.boundingBox();
        const x=bounds.x+s.target.x/s.width*bounds.width,y=bounds.y+s.target.y/s.height*bounds.height;await page.mouse.move(x,y);await flush();
        const hovered=await snapshot(renderer);assert(hovered.hovered.includes(s.target.id),`${renderer}/combo${kind}: actual current geometry pointer hit missed`);
        const tooltip=await page.locator(`#combo-${renderer} .kartis-surface-tooltip`).textContent();assert(tooltip.includes(s.target.label),`${renderer}/combo${kind}: pointer tooltip used stale datum`);
        await page.mouse.click(x,y);await flush();assert((await snapshot(renderer)).selected===s.target.id,`${renderer}/combo${kind}: actual shape activation did not select datum`);
        const control=page.locator(`#combo-${renderer} .kartis-datum`).filter({hasText:s.target.label}).first();await control.focus();await page.keyboard.press('Enter');await flush();assert((await snapshot(renderer)).events.some(e=>e.type==='activated'&&e.id===s.target.id),`${renderer}/combo${kind}: keyboard activation missing`);
        const legend=page.locator(`#combo-${renderer} .kartis-legend-item`).filter({hasText:s.target.series}).first();await legend.click();await flush();assert((await snapshot(renderer)).hidden.includes(s.target.series),`${renderer}/combo${kind}: legend hide not accepted`);
        if(renderer==='svg'){const hidden=await page.evaluate(s=>{const root=document.querySelector('#combo-svg svg');return !root.querySelector(`[data-id="${s.target.id}"]`)&&s.ownedLabels.every(l=>![...root.querySelectorAll('text')].some(t=>t.textContent===l.text))&&[...root.querySelectorAll('text')].some(t=>t.textContent==='A');},s);assert(hidden,`combo${kind}: legend hid shared axes or kept owned data labels`);}
      }
      await page.mouse.move(2,2);await page.evaluate(()=>document.activeElement?.blur());await flush();
      const accepted=await snapshot('svg'),acceptedDOM=await page.locator('#combo-svg svg').first().innerHTML();assert(await page.evaluate(()=>window.hostFixture.combo_styles_rejected()),'Invalid combo scene accepted');const rejected=await snapshot('svg');assert(rejected.revision===accepted.revision&&rejected.pending===0,'Invalid combo update queued or changed accepted state');await flush();assert(await page.locator('#combo-svg svg').first().innerHTML()===acceptedDOM,'Invalid combo update changed retained geometry');
      await page.evaluate(()=>{window.staleComboSurface=document.querySelector('#combo-svg .kartis-surface');window.staleComboButton=document.querySelector('#combo-svg .kartis-datum');window.hostFixture.combo_styles_stop();});
      assert(await page.locator('body *').count()===baseline,'Combo stop retained DOM controls or surface');await page.evaluate(()=>{window.staleComboSurface.dispatchEvent(new PointerEvent('pointermove',{clientX:100,clientY:100}));window.staleComboButton?.click();});await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(resolve)));assert(!(await snapshot('svg')).active,'Stale detached combo listener revived owner');
      reports.push({case:kind,initial,changed,geometry,currentGeometryHit:true,currentTooltip:true,keyboard:true,ownedLegendLabels:true,rejectedAtomic:true,cleanup:true});
    }
    assert(errors.length===0,errors.join('\n'));return{passed:true,scenes:reports,pageErrors:errors};
  }finally{await page.evaluate(previous=>{window.hostFixture.combo_styles_stop();document.querySelector('.panels').style.display=previous;delete window.staleComboSurface;delete window.staleComboButton;},panels);page.off('pageerror',listener);}
})
