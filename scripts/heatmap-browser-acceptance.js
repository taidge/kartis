(async function verifyHeatmap(page){
 const assert=(ok,message)=>{if(!ok)throw Error(message);},errors=[],onError=e=>errors.push(e.message);page.on('pageerror',onError);
 const snapshot=renderer=>page.evaluate(renderer=>JSON.parse(window.hostFixture.heatmap_snapshot(renderer)),renderer);
 const flush=async()=>{await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(()=>requestAnimationFrame(resolve)))));await page.evaluate(()=>window.hostFixture.heatmap_flush());};
 const previous=await page.evaluate(()=>{const p=document.querySelector('.panels'),old=p.style.display;p.style.display='none';return old;}),baseline=await page.locator('body *').count(),reports=[];
 const same=(a,b)=>JSON.stringify([...a].sort())===JSON.stringify([...b].sort());
 const related=(c,t,mode)=>[c.id===t.id,c.row===t.row,c.column===t.column,c.row===t.row||c.column===t.column,true][mode];
 const move=async(renderer,point)=>{const surface=page.locator(`#heatmap-${renderer} .kartis-surface`);await surface.scrollIntoViewIfNeeded();const b=await surface.boundingBox();await page.mouse.move(b.x+point[0]/640*b.width,b.y+point[1]/420*b.height);await flush();};
 const clear=async()=>{await page.mouse.move(2,2);await page.evaluate(()=>document.activeElement?.blur());await flush();};
 const pixels=()=>page.evaluate(async()=>{
  const live=document.querySelector('#heatmap-svg svg'),svg=live.cloneNode(true),canvas=document.querySelector('#heatmap-canvas canvas');let language;
  for(let e=live;e;e=e.parentElement){language=e.getAttributeNS('http://www.w3.org/XML/1998/namespace','lang')??e.getAttribute('lang');if(language!==null)break;}
  svg.setAttribute('lang',language??document.documentElement.lang);svg.setAttribute('width',String(canvas.width));svg.setAttribute('height',String(canvas.height));const image=new Image();image.src='data:image/svg+xml;charset=utf-8,'+encodeURIComponent(new XMLSerializer().serializeToString(svg));await image.decode();
  const ref=document.createElement('canvas');ref.width=canvas.width;ref.height=canvas.height;const ctx=ref.getContext('2d');ctx.drawImage(image,0,0);const a=ctx.getImageData(0,0,ref.width,ref.height).data,b=canvas.getContext('2d').getImageData(0,0,canvas.width,canvas.height).data;let difference=0,ink=0;for(let i=0;i<a.length;i++)difference+=Math.abs(a[i]-b[i]);for(let i=0;i<b.length;i+=4)ink+=Number(Math.min(b[i],b[i+1],b[i+2])<220);return{mean:difference/a.length,ink,backing:[canvas.width,canvas.height]};
 });
 try{
  for(let kind=0;kind<4;kind++){
   await clear();await page.evaluate(kind=>window.hostFixture.heatmap_mount(kind),kind);await flush();const phases=[];
   for(let phase=0;phase<2;phase++){
    if(phase){await clear();await page.evaluate(()=>window.hostFixture.heatmap_phase(1));await flush();}
    await clear();const initial=await snapshot('svg'),parity=await pixels();assert(parity.mean<.5&&parity.ink>3000,`HeatMap${kind}/${phase} actual pixels differ ${JSON.stringify(parity)}`);
    assert(initial.cells.length===11&&initial.null.value===null&&initial.zero.value===0&&initial.null.formatted===null&&initial.zero.formatted==='0.0','NULL, zero or absent sparse pair was conflated');
    assert(initial.target.value===6+phase&&initial.target.row==='South'&&initial.target.column==='gamma','HeatMap update changed stable key or retained old value');
    assert(initial.rows.join(',')===(phase?'West,South,North':'North,South,West')&&initial.columns.join(',')===(phase?'delta,gamma,beta,alpha':'alpha,beta,gamma,delta'),'Explicit row/column order did not affect actual layout');
    const text=await page.evaluate(()=>{const svg=document.querySelector('#heatmap-svg svg'),x=[...svg.querySelectorAll('text')].find(t=>t.textContent==='Alpha');return{size:x.getAttribute('font-size'),rotation:x.getAttribute('transform'),safe:!svg.querySelector('safe')};});assert(Number(text.size)===17&&text.rotation.includes('rotate(-25')&&text.safe,'HeatMap typed axis font/rotation/safe labels missing');
    const modes=[];
    for(let mode=0;mode<5;mode++){
     await page.evaluate(mode=>window.hostFixture.heatmap_target(mode),mode);await flush();
     for(const renderer of ['svg','canvas']){
      await clear();let state=await snapshot(renderer);await move(renderer,state.target.center);state=await snapshot(renderer);
      const expected=state.cells.filter(c=>c.visible&&related(c,state.target,mode)).map(c=>c.id);assert(same(state.activeIds,expected),`${renderer}/HeatMap${kind}/${phase}/mode${mode} exact peers mismatch ${JSON.stringify(state.activeIds)}`);
      assert(state.hovered.includes(state.target.id),'Controlled peer styling replaced the actual current hover datum');
      assert(state.cells.every(c=>Math.abs(c.opacity-(expected.includes(c.id)?.9:.17))<1e-12),'Controlled layout did not apply active/inactive opacities');
      if(renderer==='svg'){
       const painted=await page.evaluate(()=>[...document.querySelectorAll('#heatmap-svg svg [data-id]')].filter(p=>p.dataset.id.startsWith('heatmap-cell:')).map(p=>({id:p.dataset.id,opacity:Number(p.getAttribute('opacity')),border:p.getAttribute('stroke'),width:Number(p.getAttribute('stroke-width'))})));
       assert(painted.every(p=>Math.abs(p.opacity-(expected.includes(p.id)?.9:.17))<1e-12&&p.border==='rgba(15,30,50,1)'&&p.width===2),'Browser automatic series dim/stroke overrode controlled cell peers');
      }else{
       // A cross-row peer proves Controlled avoids the generic series dim.
       const cell=state.cells.find(c=>c.row==='North'&&c.column==='gamma'),g=cell.geometry,b=cell.band;
       const point=g.type==='circle'?[g.x+g.radius*.45,g.y-g.radius*.5]:[cell.center[0]+b[2]*.2,cell.center[1]-b[3]*.28];
       const actual=await page.evaluate(point=>{const canvas=document.querySelector('#heatmap-canvas canvas'),x=Math.floor(point[0]*canvas.width/640),y=Math.floor(point[1]*canvas.height/420);return Array.from(canvas.getContext('2d').getImageData(x,y,1,1).data);},point);
       const opacity=expected.includes(cell.id)?.9:.17,wanted=cell.color.slice(0,3).map(c=>Math.round(c*opacity+255*(1-opacity)));
       assert(wanted.every((c,i)=>Math.abs(c-actual[i])<=1)&&actual[3]===255,`Canvas controlled cross-row peer opacity differs ${JSON.stringify({actual,wanted,mode,kind,phase})}`);
      }
      const tooltip=await page.locator(`#heatmap-${renderer} .kartis-surface-tooltip`).textContent();assert(tooltip===state.target.label,'Current HeatMap tooltip is stale or double-formatted');
     }
     modes.push({mode,exactPeers:true,controlledSvgOpacityAndBorder:true,controlledCanvasCrossRowPixel:true});
    }
    for(const renderer of ['svg','canvas']){
     await clear();let state=await snapshot(renderer);await move(renderer,state.null.center);let current=await snapshot(renderer);assert(current.hovered.includes(state.null.id),'Explicit NULL cell is not independently hittable');const nullText=await page.locator(`#heatmap-${renderer} .kartis-surface-tooltip`).textContent();assert(nullText===state.null.label&&nullText.endsWith('NULL')&&!nullText.endsWith(': 0'),'NULL tooltip was converted to numeric zero');
     await move(renderer,state.zero.center);current=await snapshot(renderer);assert(current.hovered.includes(state.zero.id),'Zero was treated as missing');assert((await page.locator(`#heatmap-${renderer} .kartis-surface-tooltip`).textContent()).endsWith('0.0'),'Zero formatted value lost');
     const b=state.target.band;await move(renderer,[b[0]+.1,b[1]+.1]);assert((await snapshot(renderer)).hovered.length===0,`${renderer}: outside circle/rounded cell corner hit its bounding rectangle`);
     const row=state.cells.filter(c=>c.row==='South').sort((a,b)=>a.band[0]-b.band[0]),a=row[0].band,d=row[1].band;
     await move(renderer,[(a[0]+a[2]+d[0])/2,a[1]+a[3]/2]);assert((await snapshot(renderer)).hovered.length===0,`${renderer}: independent padding gap hit a cell`);
     await move(renderer,state.target.center);const surface=page.locator(`#heatmap-${renderer} .kartis-surface`),bounds=await surface.boundingBox();await page.mouse.click(bounds.x+state.target.center[0]/640*bounds.width,bounds.y+state.target.center[1]/420*bounds.height);await flush();assert((await snapshot(renderer)).selected===state.target.id,'HeatMap pointer selection missed');
     const control=page.locator(`#heatmap-${renderer} .kartis-datum`).filter({hasText:state.target.label}).first();await control.focus();await page.keyboard.press('Enter');await flush();current=await snapshot(renderer);assert(current.events.some(e=>e.type==='activated'&&e.id===state.target.id),'Current HeatMap keyboard callback missing');assert(current.events.some(e=>e.type==='heatmap-current'&&e.id===state.target.id&&e.value===6+phase),'HeatMap callback used stale pre-update value');
     await surface.press('Escape');await clear();
    }
    phases.push({phase,parity,modes,nullDistinctFromZero:true,exactGeometryAndGaps:true,currentPointerKeyboardCallbacks:true});
   }
   await clear();const beforeHidden=await snapshot('svg');await page.evaluate(()=>window.hostFixture.heatmap_hide_row(true));await flush();const hidden=await snapshot('svg');assert(hidden.rows.join(',')==='West,South'&&hidden.cells.filter(c=>c.row==='North').every(c=>!c.visible&&c.band===null),'Explicit hidden rows were left in the band domain');assert(hidden.target.id===beforeHidden.target.id&&hidden.target.center[1]!==beforeHidden.target.center[1],'Hiding a row did not genuinely reflow bands');const hiddenPixels=await pixels();assert(hiddenPixels.mean<.5,'Hidden HeatMap SVG/Canvas pixels differ');
   await page.evaluate(()=>window.hostFixture.heatmap_hide_row(false));await flush();
   for(const renderer of ['svg','canvas']){await page.locator(`#heatmap-${renderer} .kartis-legend-item`).filter({hasText:'North <safe>'}).click();await flush();assert((await snapshot(renderer)).hidden.includes('North'),'Row legend did not hide exact row owner');assert(await page.locator(`#heatmap-${renderer} .kartis-datum`).count()===7,'Hidden row left its cell controls');}
   assert(await page.evaluate(()=>{const svg=document.querySelector('#heatmap-svg svg');return ![...svg.querySelectorAll('[data-id]')].some(p=>p.dataset.series==='North')&&[...svg.querySelectorAll('text')].some(t=>t.textContent==='North <safe>');}),'Legend removed shared row-axis label or retained owned cells');
   await clear();const accepted=await snapshot('svg'),markup=await page.locator('#heatmap-svg svg').first().innerHTML(),canvas=await page.evaluate(()=>document.querySelector('#heatmap-canvas canvas').toDataURL());assert(await page.evaluate(()=>window.hostFixture.heatmap_rejected()),'Invalid HeatMap Scene or controlled key was accepted');assert((await snapshot('svg')).revision===accepted.revision,'Rejected HeatMap update changed revision');await flush();assert(await page.locator('#heatmap-svg svg').first().innerHTML()===markup&&await page.evaluate(()=>document.querySelector('#heatmap-canvas canvas').toDataURL())===canvas,'Rejected HeatMap update changed accepted SVG/Canvas frame');
   await page.evaluate(()=>{window.staleHeatmap=document.querySelector('#heatmap-svg .kartis-datum');window.hostFixture.heatmap_stop();});assert(await page.locator('body *').count()===baseline,'HeatMap stop retained DOM');await page.evaluate(()=>window.staleHeatmap?.click());await flush();assert(!(await snapshot('svg')).active,'Detached HeatMap callback revived stopped owner');reports.push({case:kind,phases,hiddenPixels,hiddenReflow:true,rowLegendOwnership:true,rejectedAtomic:true,cleanup:true});
  }
  assert(errors.length===0,errors.join('\n'));return{passed:true,cases:reports,pageErrors:errors};
 }finally{await page.evaluate(previous=>{window.hostFixture.heatmap_stop();document.querySelector('.panels').style.display=previous;delete window.staleHeatmap;},previous);page.off('pageerror',onError);}
})
