(async function verifyEditableBrush(page, options = {}) {
  const assert = (ok, message) => { if (!ok) throw Error(message); };
  const {readFile} = await import('node:fs/promises');
  const {createHash} = await import('node:crypto');
  const bundle = await readFile('tests/fixtures/editable-brush-d3-oracle.js', 'utf8');
  const extraSource = await readFile('scripts/editable-brush-browser-extra.js', 'utf8');
  const reference = JSON.parse(await readFile('tests/fixtures/editable-brush-d3.json', 'utf8'));
  const cases = reference.cases.filter(c => c.profile === 'geometry' || c.profile === 'filter');
  assert(cases.length >= 60, 'Independent D3 brush geometry cases missing');
  const errors = [], onError = error => errors.push(error.message);
  page.on('pageerror', onError);
  const state = r => page.evaluate(r => JSON.parse(window.hostFixture.editable_brush_snapshot(r)), r);
  const flush = async () => {
    await page.evaluate(() => new Promise(resolve => requestAnimationFrame(resolve)));
    await page.evaluate(() => window.hostFixture.editable_brush_flush());
    await page.evaluate(() => new Promise(resolve => requestAnimationFrame(resolve)));
    await page.evaluate(() => window.hostFixture.editable_brush_flush());
  };
  const close = (a,b) => Math.abs(a-b) <= 1e-7 * Math.max(1,Math.abs(a),Math.abs(b));
  const rectEqual = (a,b) => a === null || b === null ? a === b : a.length === b.length && a.every((v,i) => close(v,b[i]));
  const surface = r => page.locator(`#editable-brush-${r} .kartis-surface`);
  const location = async (r, point) => {
    await surface(r).scrollIntoViewIfNeeded();
    const s = await state(r), b = await surface(r).boundingBox();
    return {x:b.x+point[0]/s.width*b.width,y:b.y+point[1]/s.height*b.height};
  };
  const mouse = async (r, point) => { const p = await location(r,point); await page.mouse.move(p.x,p.y); };
  const set = async (r, selection) => { await page.evaluate(({r,selection}) => window.hostFixture.editable_brush_set(r,selection ?? undefined),{r,selection}); await flush(); };
  const previous = await page.evaluate(() => { const panels=document.querySelector('.panels'), old=panels.style.display; panels.style.display='none'; return old; });
  const baseline = await page.locator('body *').count();
  const keyName = {Alt:'Alt',Shift:'Shift',Space:'Space',Meta:'Meta',Control:'Control'};
  const held = new Set(), reports = [];
  try {
    await page.evaluate(bundle => {
      const iframe = document.createElement('iframe');
      iframe.className='kartis-editable-brush-oracle';
      iframe.setAttribute('aria-hidden','true'); iframe.setAttribute('inert','');
      iframe.style.cssText='position:fixed;left:-10000px;top:0;width:640px;height:400px;opacity:0;pointer-events:none';
      document.body.appendChild(iframe);
      const doc=iframe.contentDocument;
      doc.open(); doc.write('<!doctype html><html><head><style>html,body{margin:0}svg{display:block;position:absolute;left:0;top:0}</style></head><body><svg id="oracle" width="640" height="400" viewBox="0 0 640 400"></svg></body></html>'); doc.close();
      const script=doc.createElement('script'); script.textContent=bundle; doc.body.appendChild(script);
      const win=iframe.contentWindow, d3=win.d3;
      const models=new Map();
      const rect=value=>value==null?null:[value[0][0],value[0][1],value[1][0],value[1][1]];
      window.__editableBrushOracle={
        init(r,c){
          const svg=d3.select(doc.getElementById('oracle'));
          svg.selectAll('*').remove(); models.clear();
          const g=svg.append('g'), input=value=>value==null?null:c.axis==='x'?[value[0],value[2]]:c.axis==='y'?[value[1],value[3]]:[[value[0],value[1]],[value[2],value[3]]];
          const b=(c.axis==='x'?d3.brushX():c.axis==='y'?d3.brushY():d3.brush()).extent([[c.extent[0],c.extent[1]],[c.extent[2],c.extent[3]]]).handleSize(c.handleSize).keyModifiers(c.keys??true);
          const model={g,b,events:[]};
          b.on('start.acceptance brush.acceptance end.acceptance',e=>model.events.push({phase:e.type,mode:e.mode??null,selection:rect(g.node().__brush.selection)}));
          g.call(b); if(c.initial!==null)g.call(b.move,input(c.initial)); model.events=[];models.set(r,model);
        },
        replay(r,packets){
          const model=models.get(r);
          for(const p of packets){
            if(p.type==='key'){
              const codes={Alt:18,Shift:16,Space:32,Meta:91,Control:17};
              win.dispatchEvent(new win.KeyboardEvent(p.down?'keydown':'keyup',{bubbles:true,key:p.key==='Space'?' ':p.key,keyCode:codes[p.key],code:p.key,altKey:p.mods.alt,shiftKey:p.mods.shift,metaKey:p.mods.meta,ctrlKey:p.mods.ctrl}));
            }else{
              const type=p.type==='down'?'mousedown':p.type==='up'||p.type==='cancel'?'mouseup':'mousemove';
              const target=type==='mousedown'?model.g.select(p.target==='overlay'||p.target==='selection'?`.${p.target}`:`.handle--${p.target}`).node():win;
              const event=new win.MouseEvent(type,{bubbles:true,view:win,clientX:p.point[0],clientY:p.point[1],buttons:type==='mouseup'?0:1,altKey:p.mods.alt,shiftKey:p.mods.shift,metaKey:p.mods.meta,ctrlKey:p.mods.ctrl});
              // MouseEventInit converts client coordinates to integer longs;
              // native PointerEvents can carry fractional CSS coordinates.
              // D3 reads these getters, so replay the captured actual doubles.
              Object.defineProperty(event,'clientX',{value:p.point[0]});
              Object.defineProperty(event,'clientY',{value:p.point[1]});
              target.dispatchEvent(event);
            }
          }
          const state=JSON.parse(window.hostFixture.editable_brush_snapshot(r));
          return {selection:rect(model.g.node().__brush.selection),events:model.events.splice(0),handles:model.g.selectAll('.handle').nodes().map(node=>{
            const bounds=node.hasAttribute('x')?['x','y','width','height'].map(k=>Number(node.getAttribute(k))):null;
            const css=doc.createElement('div');
            if(bounds)bounds.forEach((value,i)=>css.style[['left','top','width','height'][i]]=`${value/(i%2?state.height:state.width)*100}%`);
            return{handle:node.__data__.type,visible:win.getComputedStyle(node).display!=='none',bounds,cssBounds:['left','top','width','height'].map(k=>css.style[k])};
          })};
        }
      };
      window.__editableBrushPackets=[];
      const ownership={active:null,space:false};
      const record=event=>{
        if(event.type==='keydown'||event.type==='keyup'){
          const name=event.key===' '?'Space':event.key;
          if(name==='Space')ownership.space=event.type==='keydown';
          if(!ownership.active||!['Alt','Shift','Space','Meta','Control'].includes(name))return;
          window.__editableBrushPackets.push({renderer:ownership.active.r,type:'key',key:name,down:event.type==='keydown',trusted:event.isTrusted,mods:{alt:event.altKey,shift:event.shiftKey,meta:event.metaKey,ctrl:event.ctrlKey,space:ownership.space}});return;
        }
        if(event.type==='pointerdown'){
          const root=event.target.closest?.('[id^="editable-brush-"]');if(!root||event.button!==0)return;
          ownership.active={r:root.id.slice('editable-brush-'.length),id:event.pointerId};
        }
        const active=ownership.active;if(!active||event.pointerId!==active.id)return;
        const s=JSON.parse(window.hostFixture.editable_brush_snapshot(active.r)),surface=document.querySelector(`#editable-brush-${active.r} .kartis-surface`),box=surface.getBoundingClientRect();
        const target=event.target.closest?.('[data-brush-target]')?.getAttribute('data-brush-target')??'overlay';
        window.__editableBrushPackets.push({renderer:active.r,type:event.type.slice(7),target,point:[(event.clientX-box.x)*s.width/box.width,(event.clientY-box.y)*s.height/box.height],client:[event.clientX,event.clientY],pointerId:event.pointerId,pointerType:event.pointerType,trusted:event.isTrusted,mods:{alt:event.altKey,shift:event.shiftKey,meta:event.metaKey,ctrl:event.ctrlKey,space:ownership.space}});
        if(event.type==='pointerup'||event.type==='pointercancel')ownership.active=null;
      };
      const names=['pointerdown','pointermove','pointerup','pointercancel','keydown','keyup'];
      names.forEach(name=>window.addEventListener(name,record,true));
      const captured=event=>{const active=ownership.active;if(!active||active.id!==event.pointerId)return;const packet=window.__editableBrushPackets.findLast(p=>p.type==='down'&&p.pointerId===event.pointerId&&p.renderer===active.r);if(packet){const s=JSON.parse(window.hostFixture.editable_brush_snapshot(packet.renderer)),surface=document.querySelector(`#editable-brush-${packet.renderer} .kartis-surface`);if(surface){packet.capturedByHost=s.editor?.captured===event.pointerId;packet.actualCapture=surface.hasPointerCapture(event.pointerId);}}};
      window.addEventListener('pointerdown',captured);
      window.__editableBrushRecorderStop=()=>{names.forEach(name=>window.removeEventListener(name,record,true));window.removeEventListener('pointerdown',captured);iframe.remove();delete window.__editableBrushOracle;delete window.__editableBrushPackets;delete window.__editableBrushRecorderStop;};
    },bundle);

    await page.evaluate(() => window.hostFixture.editable_brush_mount(0)); await flush();
    for(const r of ['svg','canvas']) {
      for(const c of cases) {
        await page.mouse.up();for(const key of [...held]){await page.keyboard.up(keyName[key]);held.delete(key);}
        const mode=c.axis==='x'?1:c.axis==='y'?2:0;
        await page.evaluate(({r,c,mode})=>window.hostFixture.editable_brush_profile(r,mode,c.extent,c.keys??true),{r,c,mode});await flush();
        await set(r,c.initial);
        await page.evaluate(({r,c})=>{window.__editableBrushOracle.init(r,c);window.__editableBrushPackets=[];},{r,c});
        let previousCount=(await state(r)).events.length;
        let trustedPointers=0, steps=0;
        for(const step of c.steps) {
          if(step.type==='down'){
            for(const [name,active]of Object.entries(step.modifiers??{})){if(!active)continue;const key={alt:'Alt',shift:'Shift',meta:'Meta',ctrl:'Control'}[name];await page.keyboard.down(keyName[key]);held.add(key);}
            await mouse(r,step.point);await page.mouse.down();
          }else if(step.type==='move')await mouse(r,step.point);
          else if(step.type==='up'){await mouse(r,step.point);await page.mouse.up();}
          else if(step.type==='key'){if(step.down){await page.keyboard.down(keyName[step.key]);held.add(step.key);}else{await page.keyboard.up(keyName[step.key]);held.delete(step.key);}}
          else throw Error(`Unexpected D3 host trace step ${step.type}`);
          await flush();
          const packets=await page.evaluate(r=>{const all=window.__editableBrushPackets.splice(0);return all.filter(p=>p.renderer===r);},r);
          assert(packets.every(p=>p.trusted),`${r}/${c.id}: Rust host received synthetic native input`);
          trustedPointers+=packets.filter(p=>p.type!=='key').length;
          const expected=await page.evaluate(({r,packets})=>window.__editableBrushOracle.replay(r,packets),{r,packets});
          const current=await state(r), actual=current.editor?.selection??null;
          assert(rectEqual(current.view,[1,0,0]),`${r}/${c.id}/${steps} brush input also moved the ordinary pan/zoom camera: ${current.view}`);
          assert(rectEqual(actual,expected.selection),`${r}/${c.id}/${steps} selection ${JSON.stringify(actual)} != original D3 on actual native inputs ${JSON.stringify(expected.selection)}; packets=${JSON.stringify(packets)}`);
          const emitted=current.events.slice(previousCount).filter(e=>e.phase&&e.source==='User');previousCount=current.events.length;
          const phase={Start:'start',Change:'brush',End:'end'},mode={Create:'handle',Resize:'handle',Move:'drag',Center:'center',Space:'space'};
          assert(emitted.length===expected.events.length,`${r}/${c.id}/${steps} lifecycle count ${JSON.stringify(emitted)} != ${JSON.stringify(expected.events)}`);
          emitted.forEach((e,i)=>assert(phase[e.phase]===expected.events[i].phase&&mode[e.mode]===expected.events[i].mode&&rectEqual(e.selection,expected.events[i].selection),`${r}/${c.id}/${steps} lifecycle differs from original D3: ${JSON.stringify(e)} != ${JSON.stringify(expected.events[i])}`));
          const geometry=await page.evaluate(r=>{const root=document.getElementById(`editable-brush-${r}`);return [...root.querySelectorAll('.kartis-editable-brush-handle')].map(node=>({handle:node.dataset.brushTarget,visible:getComputedStyle(node).display!=='none',cssBounds:['left','top','width','height'].map(key=>node.style[key]),role:node.getAttribute('role'),label:node.getAttribute('aria-label')}));},r);
          assert(geometry.filter(h=>h.visible).length===expected.handles.filter(h=>h.visible).length,`${r}/${c.id}/${steps} inactive-axis handles remained visible`);
          const names={n:'top edge',w:'left edge',e:'right edge',s:'bottom edge',nw:'top-left corner',ne:'top-right corner',sw:'bottom-left corner',se:'bottom-right corner'};
          for(const h of expected.handles){const actual=geometry.find(a=>a.handle===h.handle);assert(actual&&actual.visible===h.visible,`${r}/${c.id}/${steps} handle ${h.handle} visibility`);if(h.visible)assert(actual.cssBounds.every((v,i)=>v===h.cssBounds[i])&&actual.role==='slider'&&actual.label?.includes(`Resize selection ${names[h.handle]}`),`${r}/${c.id}/${steps} actual HTML handle ${h.handle} serialized CSS geometry/accessibility ${JSON.stringify(actual)} != independent D3 geometry serialized by the same browser ${JSON.stringify(h)}`);}
          assert(current.dropped===0,`${r}/${c.id} dropped input`);steps++;
        }
        assert(c.profile==='filter'||trustedPointers>=2,`${r}/${c.id} has no native pointer trace`);
        reports.push({renderer:r,case:c.id,axis:c.axis,steps,trustedPointers,actualInputD3Replay:true,selection:true,lifecycle:true,htmlHandles:true});
      }
    }

    for(const key of [...held]){await page.keyboard.up(keyName[key]);held.delete(key);}
    const lifecycle=[];
    for(const r of ['svg','canvas']){
      await page.evaluate(r=>window.hostFixture.editable_brush_configure(r,0,1,true),r);await flush();
      await set(r,[80,80,300,220]);
      assert(JSON.stringify([...(await state(r)).editor.ids].sort())===JSON.stringify(['point-alpha-0','point-alpha-1','point-beta-0']),`${r} programmatic brush IDs did not use actual current datum centers`);
      assert(await surface(r).getByRole('slider').count()===8&&await surface(r).getByRole('slider',{name:/top-left corner/}).count()===1,`${r} handles were not exposed as named accessible controls`);
      const selected=page.locator(`#editable-brush-${r} .kartis-editable-brush-selection`);
      await selected.focus();await page.keyboard.press('ArrowRight');await flush();
      assert(rectEqual((await state(r)).editor.selection,[81,80,301,220]),`${r} focused selection keyboard move failed`);
      const east=page.locator(`#editable-brush-${r} .kartis-editable-brush-handle--e`);
      await east.focus();await page.keyboard.press('Shift+ArrowRight');await flush();
      assert(rectEqual((await state(r)).editor.selection,[81,80,311,220]),`${r} focused handle keyboard resize failed`);
      const before=await state(r);assert(await page.evaluate(r=>window.hostFixture.editable_brush_rejected(r),r),`${r} rejected command changed queue/frame`);await flush();assert(rectEqual((await state(r)).editor.selection,before.editor.selection),`${r} rejected command mutated selection`);
      await set(r,[80,80,300,220]);await page.evaluate(r=>window.hostFixture.editable_brush_reenter(r),r);
      await mouse(r,[300,150]);await page.mouse.down();await flush();await mouse(r,[320,150]);await flush();await flush();
      assert(rectEqual((await state(r)).editor.selection,[350,60,500,160]),`${r} callback replacement was not authoritative`);
      await mouse(r,[370,190]);await page.mouse.up();await flush();
      assert(rectEqual((await state(r)).editor.selection,[350,60,500,160])&&!(await state(r)).editor.active,`${r} stale move/up revived callback-retired session`);
      await set(r,[80,80,300,220]);await mouse(r,[300,150]);await page.mouse.down();await flush();await mouse(r,[340,150]);await flush();await page.keyboard.press('Escape');await flush();await page.mouse.up();await flush();
      assert(rectEqual((await state(r)).editor.selection,[80,80,300,220]),`${r} Escape did not restore the pre-gesture selection`);
      await selected.focus();await page.keyboard.press('Delete');await flush();assert((await state(r)).editor.selection===null&&await surface(r).getByRole('slider').count()===0,`${r} keyboard clear left active accessible handles`);
      assert(await page.evaluate(r=>{const handle=document.querySelector(`#editable-brush-${r} .kartis-editable-brush-handle--n`);handle.focus();return document.activeElement!==handle;},r),`${r} an unavailable handle accepted keyboard focus`);
      await set(r,[80,80,300,220]);await mouse(r,[300,150]);await page.mouse.down();await flush();await mouse(r,[340,150]);await flush();await page.evaluate(r=>window.hostFixture.editable_brush_hidden(r,true),r);await flush();
      const hiddenBeforeUp=await state(r);assert(!hiddenBeforeUp.editor.active&&hiddenBeforeUp.editor.captured===null&&hiddenBeforeUp.editor.session===null&&rectEqual(hiddenBeforeUp.editor.selection,[80,80,300,220])&&JSON.stringify(hiddenBeforeUp.editor.ids)===JSON.stringify(['point-beta-0']),`${r} hidden state kept a live session or stale IDs before pointerup`);
      await mouse(r,[360,150]);await page.mouse.up();await flush();
      assert(!(await state(r)).editor.active&&(await state(r)).editor.captured===null,`${r} hidden series retained an active pointer`);
      await page.evaluate(r=>window.hostFixture.editable_brush_hidden(r,false),r);await flush();
      await set(r,null);await mouse(r,[80,80]);await page.mouse.down();await flush();await mouse(r,[200,150]);await flush();await page.keyboard.down('Space');await mouse(r,[220,170]);await flush();await page.mouse.up();await flush();await page.keyboard.up('Space');await flush();
      await set(r,[80,80,300,220]);
      await page.evaluate(r=>{const s=JSON.parse(window.hostFixture.editable_brush_snapshot(r));window.__editableBrushOracle.init(r,{axis:'xy',extent:s.editor.extent,initial:[80,80,300,220],handleSize:10,keys:true});window.__editableBrushPackets=[];},r);
      await mouse(r,[300,150]);await page.mouse.down();await flush();await page.keyboard.down('Space');await flush();
      assert((await state(r)).editor.mode==='Space',`${r} Space released after pointerup leaked into the next gesture`);
      await mouse(r,[320,180]);await flush();const resumed=await page.evaluate(r=>{const packets=window.__editableBrushPackets.splice(0).filter(p=>p.renderer===r);return{packets,expected:window.__editableBrushOracle.replay(r,packets)};},r);
      assert(resumed.packets.every(p=>p.trusted)&&rectEqual((await state(r)).editor.selection,resumed.expected.selection),`${r} a second Space gesture did not match actual-input D3`);
      await page.mouse.up();await page.keyboard.up('Space');await flush();await page.evaluate(r=>window.__editableBrushOracle.replay(r,window.__editableBrushPackets.splice(0).filter(p=>p.renderer===r)),r);
      await set(r,[80,80,300,220]);const beforeOutside=(await state(r)).events.length;
      await page.keyboard.down('Shift');await mouse(r,[20,20]);await page.mouse.down();await mouse(r,[30,25]);await page.mouse.up();await page.keyboard.up('Shift');await flush();
      const outside=await state(r);assert(!outside.events.slice(beforeOutside).some(e=>e.other?.startsWith('Interaction(Brush {'))&&rectEqual(outside.editor.selection,[80,80,300,220])&&!outside.editor.active,`${r} outside-extent Shift input created an invisible legacy brush`);
      await page.evaluate(r=>window.hostFixture.editable_brush_configure(r,0,1,false),r);await flush();assert(await page.locator(`#editable-brush-${r} .kartis-editable-brush`).count()===0&&!(await state(r)).editor,`${r} disabling brush left controls`);
      lifecycle.push({renderer:r,keyboardSelection:true,keyboardHandle:true,accessibleHandles:true,inactiveHandlesUnfocusable:true,currentIds:true,rejectedAtomic:true,callbackReplacement:true,staleQueueRetirement:true,escapeRollback:true,hiddenCancellation:true,hiddenBeforeUp:true,spaceReleaseAcrossGestures:true,outsideExtentArbitration:true,disableCleanup:true});
    }
    const enhanced=await (0,eval)(extraSource)({page,state,flush,set,mouse,surface,rectEqual,assert,nested:!!options.nested});
    await page.evaluate(()=>window.hostFixture.editable_brush_stop());await flush();
    for(const r of ['svg','canvas']){const stopped=await state(r);assert(!stopped.active&&stopped.callbackCleanup&&stopped.staleHandleRejected,`${r} stop retained callback or active handle`);}
    await page.evaluate(()=>window.__editableBrushRecorderStop());
    assert(await page.locator('body *').count()===baseline,'Editable brush stop retained DOM');
    assert(errors.length===0,errors.join('\n'));
    let dpr1;
    if(!options.nested){const context=await page.context().browser().newContext({viewport:{width:1440,height:1000},deviceScaleFactor:1});try{const next=await context.newPage();next.setDefaultTimeout(45000);await next.goto(page.url());await next.waitForFunction(()=>window.hostReady===true);await next.bringToFront();dpr1=await verifyEditableBrush(next,{nested:true});}finally{await context.close();await page.bringToFront();}}
    return {passed:true,contextDpr:await page.evaluate(()=>devicePixelRatio),reference:{package:'d3-brush',version:'3.0.0',bundleSha256:createHash('sha256').update(bundle).digest('hex'),caseSource:'tests/fixtures/editable-brush-d3.json',actualRustInputs:'trusted pointer/key events',oracle:'original D3 in isolated iframe; event-exact replay of captured actual logical coordinates; no independent-device claim'},geometry:reports,lifecycle,...enhanced,dpr1,cleanup:true,pageErrors:errors};
  } finally {
    await page.mouse.up().catch(()=>{});for(const key of held)await page.keyboard.up(keyName[key]).catch(()=>{});
    await page.evaluate(old=>{window.hostFixture?.editable_brush_stop();window.__editableBrushRecorderStop?.();document.querySelector('.panels').style.display=old;},previous).catch(()=>{});
    page.off('pageerror',onError);
  }
})
