(async ({page,state,flush,set,mouse,surface,rectEqual,assert,nested}) => {
  const {writeFile}=await import('node:fs/promises'),{createHash,randomUUID}=await import('node:crypto');
  const evidenceSuffix=randomUUID();
  const positiveContexts=[],ink=[],safety=[],viewport=[],navigation=[],publication=[],terminalPublication=[],touch={scope:nested?'trustedTapAndInjectedDrag':'coveredByDpr1',trustedTap:[],injectedDrag:[]};
  const configure=async(r,mode=0)=>{await page.evaluate(({r,mode})=>window.hostFixture.editable_brush_configure(r,mode,1,true),{r,mode});await flush();};
  const init=async(r,c)=>page.evaluate(({r,c})=>{window.__editableBrushOracle.init(r,c);window.__editableBrushPackets=[];},{r,c});
  const packets=async r=>page.evaluate(r=>window.__editableBrushPackets.splice(0).filter(p=>p.renderer===r),r);
  const replay=async(r,p)=>page.evaluate(({r,p})=>window.__editableBrushOracle.replay(r,p),{r,p});
  const drag=async(r,a,b)=>{await mouse(r,a);await page.mouse.down();await flush();await mouse(r,b);await page.mouse.up();await flush();};
  const options=async r=>{const s=await state(r);return{axis:'xy',extent:s.editor.extent,initial:[80,80,300,220],handleSize:10,keys:true};};

  for(const r of ['svg','canvas']){
    for(const operation of ['idleSet','activeSet','activeConfigure']){
      await configure(r);await set(r,[80,80,300,220]);
      const active=operation!=='idleSet';
      if(active){await init(r,await options(r));await mouse(r,[300,150]);await page.mouse.down();await flush();await mouse(r,[320,150]);await flush();const native=await packets(r),expected=await replay(r,native);assert(native.every(p=>p.trusted)&&rectEqual((await state(r)).editor.selection,expected.selection),`${r}/${operation} initial native gesture did not match D3`);}
      const fault=await page.evaluate(({r,operation,active})=>{
        const fixture=window.hostFixture,surface=document.querySelector(`#editable-brush-${r} .kartis-surface`),editor=surface.querySelector('.kartis-editable-brush'),selection=editor.querySelector('.kartis-editable-brush-selection'),before=JSON.parse(fixture.editable_brush_snapshot(r)),dom=editor.outerHTML,focus=document.activeElement;
        const captureBefore=active&&surface.hasPointerCapture(before.editor.captured);
        const style=CSSStyleDeclaration.prototype.setProperty,attribute=Element.prototype.setAttribute;let faults=0,partialWriteObserved=false;
        CSSStyleDeclaration.prototype.setProperty=function(name,value,priority){if(operation!=='activeConfigure'&&this===selection.style&&name==='top'&&faults===0){faults++;partialWriteObserved=editor.outerHTML!==dom;throw new DOMException('intentional brush publication failure','InvalidStateError');}return style.call(this,name,value,priority);};
        Element.prototype.setAttribute=function(name,value){if(operation==='activeConfigure'&&this===surface&&name==='role'&&value==='img'&&faults===0){faults++;partialWriteObserved=!editor.isConnected;throw new DOMException('intentional brush publication failure','InvalidStateError');}return attribute.call(this,name,value);};
        try{if(operation==='activeConfigure')fixture.editable_brush_configure(r,0,1,false);else fixture.editable_brush_set(r,[100,90,320,230]);fixture.editable_brush_flush();}finally{CSSStyleDeclaration.prototype.setProperty=style;Element.prototype.setAttribute=attribute;}
        const after=JSON.parse(fixture.editable_brush_snapshot(r));
        return{faults,partialWriteObserved,before,after,domRetained:surface.querySelector('.kartis-editable-brush')===editor&&editor.outerHTML===dom,focusRetained:!active||document.activeElement===focus,captureBefore,captureAfter:active&&surface.hasPointerCapture(after.editor.captured)};
      },{r,operation,active});
      assert(fault.faults===1&&fault.partialWriteObserved&&fault.domRetained&&fault.focusRetained&&fault.after.revision===fault.before.revision&&JSON.stringify(fault.after.editor)===JSON.stringify(fault.before.editor),`${r}/${operation} actual partial DOM failure did not retain accepted state ${JSON.stringify(fault)}`);
      assert(fault.after.events.slice(fault.before.events.length).some(e=>e.error?.includes('intentional brush publication failure'))&&!fault.after.events.slice(fault.before.events.length).some(e=>e.phase),`${r}/${operation} failure emitted a rejected brush lifecycle`);
      if(active){assert(fault.captureBefore&&fault.captureAfter&&fault.after.editor.active&&fault.after.editor.session===fault.before.editor.session,`${r}/${operation} rollback lost the live session/capture`);await mouse(r,[340,150]);await flush();const native=await packets(r),expected=await replay(r,native),continued=await state(r);assert(native.every(p=>p.trusted)&&rectEqual(continued.editor.selection,expected.selection),`${r}/${operation} restored gesture diverged from original D3 on actual native continuation ${JSON.stringify({actual:continued.editor,expected:expected.selection,native})}`);await page.mouse.up();await flush();await replay(r,await packets(r));}
      await set(r,[100,90,320,230]);assert(rectEqual((await state(r)).editor.selection,[100,90,320,230]),`${r}/${operation} recovery could not retry`);
      publication.push({renderer:r,operation,scope:active?'active':'idle',actualDomFault:true,partialWriteObserved:true,modelDomAtomic:true,revisionRetained:true,sessionRetained:active?true:undefined,continuedGesture:active?true:undefined,nativeInputD3Continuation:active?true:undefined,actualCaptureRetained:active?true:undefined,focusRetained:true,retry:true});
    }
    await configure(r);await set(r,[80,80,300,220]);await mouse(r,[300,150]);await page.mouse.down();await flush();await mouse(r,[320,150]);await flush();
    const terminalState=await state(r),terminalBox=await surface(r).boundingBox(),terminalPoint={x:terminalBox.x+340/terminalState.width*terminalBox.width,y:terminalBox.y+150/terminalState.height*terminalBox.height};
    await page.evaluate(r=>{
      const surface=document.querySelector(`#editable-brush-${r} .kartis-surface`),selection=surface.querySelector('.kartis-editable-brush-selection');
      const held=new Map(),raf=window.requestAnimationFrame,cancel=window.cancelAnimationFrame,setter=CSSStyleDeclaration.prototype.setProperty;
      const receipt={r,surface,selection,before:JSON.parse(window.hostFixture.editable_brush_snapshot(r)),styles:[...surface.querySelectorAll('.kartis-editable-brush-control')].map(n=>n.style.cssText),raf,cancel,setter,held,next:1_000_000_000,faults:0,partial:false};
      window.__editableBrushPackets=[];window.__editableTerminal=receipt;
      window.requestAnimationFrame=callback=>{const token=receipt.next++;held.set(token,callback);return token;};
      window.cancelAnimationFrame=token=>{if(held.has(token))held.delete(token);else cancel.call(window,token);};
      CSSStyleDeclaration.prototype.setProperty=function(name,value,priority){if(this===selection.style&&name==='height'&&receipt.faults===0){receipt.faults++;receipt.partial=selection.style.cssText!==receipt.styles[0];throw new DOMException('intentional terminal brush publication failure','InvalidStateError');}return setter.call(this,name,value,priority);};
    },r);
    let terminal;
    try{
      // Only the scheduler is held: both pointer events remain native trusted
      // Playwright input, and pointerup physically releases capture before RAF.
      await page.mouse.move(terminalPoint.x,terminalPoint.y);await page.mouse.up();
      terminal=await page.evaluate(()=>{
        const c=window.__editableTerminal;
        const beforeFlush=JSON.parse(window.hostFixture.editable_brush_snapshot(c.r));
        window.hostFixture.editable_brush_flush();
        const after=JSON.parse(window.hostFixture.editable_brush_snapshot(c.r));
        const packets=window.__editableBrushPackets.splice(0).filter(p=>p.renderer===c.r);
        return{before:c.before,beforeFlush,after,packets,faults:c.faults,partial:c.partial,domGeometryRetained:[...c.surface.querySelectorAll('.kartis-editable-brush-control')].every((n,i)=>n.style.cssText===c.styles[i]),actualCapture:c.surface.hasPointerCapture(c.before.editor.captured)};
      });
    }finally{
      await page.evaluate(()=>{const c=window.__editableTerminal;if(c){window.requestAnimationFrame=c.raf;window.cancelAnimationFrame=c.cancel;CSSStyleDeclaration.prototype.setProperty=c.setter;for(const callback of c.held.values())c.raf.call(window,callback);delete window.__editableTerminal;}});
    }
    assert(terminal.faults===1&&terminal.partial&&terminal.beforeFlush.pending>=2&&terminal.packets.some(p=>p.type==='move')&&terminal.packets.some(p=>p.type==='up')&&terminal.packets.every(p=>p.trusted),`${r} terminal publication test did not contain a real queued trusted move/up ${JSON.stringify(terminal)}`);
    assert(rectEqual(terminal.after.editor.selection,terminal.before.editor.selection)&&terminal.after.revision===terminal.before.revision&&terminal.domGeometryRetained&&!terminal.after.editor.active&&terminal.after.editor.session===null&&terminal.after.editor.captured===null&&!terminal.actualCapture,`${r} terminal DOM failure did not retain accepted ink while retiring the ended contact ${JSON.stringify(terminal)}`);
    assert(terminal.after.events.slice(terminal.before.events.length).some(e=>e.error?.includes('intentional terminal brush publication failure'))&&!terminal.after.events.slice(terminal.before.events.length).some(e=>e.phase),`${r} terminal rejection emitted a failed lifecycle instead of its error`);
    await mouse(r,[360,150]);await flush();assert(rectEqual((await state(r)).editor.selection,terminal.before.editor.selection)&&!(await state(r)).editor.active,`${r} ended pointer revived after a terminal publication fault`);
    terminalPublication.push({renderer:r,operation:'pointerUp',scope:'terminal',actualDomFault:true,partialWriteObserved:true,physicalEnded:true,sessionRetired:true,acceptedSelectionRetained:true,domGeometryRetained:true,revisionRetained:true,staleInputRetired:true,trustedMoveUp:true,errorOnly:true,queuedInputs:terminal.beforeFlush.pending});
    for(const kind of ['scale','nonuniform','zoom']){
      await configure(r);await set(r,[80,80,300,220]);
      await page.evaluate(({r,kind})=>{const root=document.getElementById(`editable-brush-${r}`);root.style.transformOrigin='0 0';root.style.transform=kind==='scale'?'scale(1.25)':kind==='nonuniform'?'translate(11px,7px) scale(1.25,.8)':'';root.style.zoom=kind==='zoom'?'1.25':'';},{r,kind});await flush();await flush();
      await init(r,await options(r));await drag(r,[300,150],[340,180]);
      const observed=await packets(r),expected=await replay(r,observed),current=await state(r);
      assert(rectEqual(current.view,[1,0,0]),`${r}/${kind} editable gesture also moved the ordinary camera`);
      assert(observed.length>=2&&observed.every(p=>p.trusted)&&rectEqual(current.editor.selection,expected.selection),`${r}/${kind} positive context lost actual-input D3 alignment`);
      const handles=await page.evaluate(r=>[...document.querySelectorAll(`#editable-brush-${r} .kartis-editable-brush-handle`)].map(n=>({name:n.dataset.brushTarget,visible:getComputedStyle(n).display!=='none',css:['left','top','width','height'].map(k=>n.style[k]),box:[n.getBoundingClientRect().width,n.getBoundingClientRect().height]})),r);
      expected.handles.filter(h=>h.visible).forEach(h=>{const actual=handles.find(n=>n.name===h.handle);assert(actual?.visible&&actual.box.every(v=>v>0)&&actual.css.every((v,i)=>v===h.cssBounds[i]),`${r}/${kind}/${h.handle} visible HTML handle lost scaled geometry`);});
      positiveContexts.push({renderer:r,kind,nativeInputD3Replay:true,selection:true,htmlHandles:true,packets:observed.length});
      await page.evaluate(r=>{const root=document.getElementById(`editable-brush-${r}`);root.style.transform='';root.style.transformOrigin='';root.style.zoom='';},r);await flush();await flush();
    }
    // Start the image evidence from the same authored 640x400 scene for each
    // renderer, rather than preserving dimensions from preceding CSS probes.
    await page.evaluate(()=>window.hostFixture.editable_brush_mount(0));await flush();await flush();
    await set(r,[80,80,300,220]);await surface(r).scrollIntoViewIfNeeded();
    const handle=page.locator(`#editable-brush-${r} .kartis-editable-brush-handle--nw`),sb=await surface(r).boundingBox(),hb=await handle.boundingBox();
    const before=await surface(r).screenshot();
    await handle.evaluate(n=>n.style.transform='translateX(4px)');
    let after;
    try{after=await surface(r).screenshot();}finally{await handle.evaluate(n=>n.style.transform='');}
    const pixels=await page.evaluate(async({a,b,rect,box})=>{
      const load=async data=>{const image=await createImageBitmap(new Blob([new Uint8Array(data)],{type:'image/png'}));const canvas=document.createElement('canvas');canvas.width=image.width;canvas.height=image.height;const ctx=canvas.getContext('2d');ctx.drawImage(image,0,0);image.close();return{ctx,w:canvas.width,h:canvas.height};};
      const first=await load(a),second=await load(b);const sx=first.w/box.width,sy=first.h/box.height;
      const x=Math.max(0,Math.floor((rect.x-box.x)*sx)),y=Math.max(0,Math.floor((rect.y-box.y)*sy)),w=Math.max(1,Math.ceil(rect.width*sx)),h=Math.max(1,Math.ceil(rect.height*sy));
      const ink=first.ctx.getImageData(x,y,w,h).data,all=first.ctx.getImageData(0,0,first.w,first.h).data,changed=second.ctx.getImageData(0,0,second.w,second.h).data;let blue=0,diff=0;
      for(let i=0;i<ink.length;i+=4)blue+=Number(ink[i]<120&&ink[i+2]-ink[i]>40&&ink[i+3]>0);
      for(let i=0;i<all.length;i+=4)diff+=Number(Math.abs(all[i]-changed[i])+Math.abs(all[i+1]-changed[i+1])+Math.abs(all[i+2]-changed[i+2])>40);
      return{paintedPixels:blue,negativePixels:diff,backing:[first.w,first.h],handleCrop:[x,y,w,h]};
    },{a:[...before],b:[...after],rect:hb,box:sb});
    assert(pixels.paintedPixels>0&&pixels.negativePixels>=8,`${r} real HTML handle ink/negative-control detector failed ${JSON.stringify(pixels)}`);
    const dpr=await page.evaluate(()=>devicePixelRatio),path=`target/gallery/editable-brush-${evidenceSuffix}-${r}-dpr${dpr}-handles.png`;await writeFile(path,before);
    ink.push({renderer:r,...pixels,negativeControlDetected:true,path,sha256:createHash('sha256').update(before).digest('hex')});

    // Reproduce the official center->Space overflow with positive coordinates
    // so all native pointers stay inside the actual browser viewport.
    const c={axis:'x',extent:[300,100,600,360],initial:[400,100,500,360],handleSize:10,keys:true};
    await page.evaluate(({r,c})=>window.hostFixture.editable_brush_profile(r,1,c.extent,true),{r,c});await flush();await set(r,c.initial);await init(r,c);
    await page.keyboard.down('Alt');await mouse(r,[400,230]);await page.mouse.down();await flush();await mouse(r,[50,230]);await flush();await page.keyboard.down('Space');await flush();
    const observed=await packets(r),original=await replay(r,observed),safe=(await state(r)).editor.selection;
    assert(observed.every(p=>p.trusted)&&original.selection.some((v,i)=>i<2?v<c.extent[i]:v>c.extent[i]),`${r} original D3 overflow was not actually reproduced: ${JSON.stringify(original.selection)}`);
    assert(safe&&safe[0]>=300&&safe[2]<=600&&safe[1]===100&&safe[3]===360,`${r} checked brush escaped its extent: ${safe}`);
    await page.mouse.up();await page.keyboard.up('Space');await page.keyboard.up('Alt');await flush();
    safety.push({renderer:r,originalD3EscapesExtent:true,rustBounded:true,actualNativeTrace:true,original:original.selection,checked:safe,extent:c.extent});

    await configure(r);await set(r,null);
    await set(r,null);await page.evaluate(()=>window.__editableBrushPackets=[]);
    await page.evaluate(r=>{
      const surface=document.querySelector(`#editable-brush-${r} .kartis-surface`),overlay=surface.querySelector('.kartis-editable-brush-overlay'),s=JSON.parse(window.hostFixture.editable_brush_snapshot(r)),b=surface.getBoundingClientRect();
      const send=(target,type,id,x,y)=>target.dispatchEvent(new PointerEvent(type,{bubbles:true,pointerId:id,pointerType:'touch',isPrimary:id===91,button:0,buttons:type==='pointerup'?0:1,clientX:b.x+x/s.width*b.width,clientY:b.y+y/s.height*b.height}));
      send(overlay,'pointerdown',91,80,80);send(overlay,'pointerdown',92,180,180);send(window,'pointermove',91,260,200);send(window,'pointerup',92,180,180);send(window,'pointerup',91,260,200);
    },r);await flush();
    const synthetic=(await state(r)).editor;
    assert(rectEqual(synthetic.selection,[80,80,260,200])&&!synthetic.active&&synthetic.captured===null,`${r} injected touch drag/second pointer arbitration failed ${JSON.stringify(synthetic)}`);
    touch.injectedDrag.push({renderer:r,domInjected:true,independentDevice:false,selection:true,secondTouchIgnored:true});
  }

  if(nested){
    const context=await page.context().browser().newContext({viewport:{width:1440,height:1000},deviceScaleFactor:1,hasTouch:true});
    try{
      const tapPage=await context.newPage();tapPage.setDefaultTimeout(45000);await tapPage.goto(page.url());await tapPage.waitForFunction(()=>window.hostReady===true);await tapPage.bringToFront();
      await tapPage.evaluate(()=>{document.querySelector('.panels').style.display='none';window.hostFixture.editable_brush_mount(0);window.__trustedBrushTouch=[];window.__touchDiagnostics=[];for(const type of ['pointerdown','pointerup','touchstart','touchend'])window.addEventListener(type,event=>{const root=event.target.closest?.('[id^="editable-brush-"]');if(!root)return;const r=root.id.slice('editable-brush-'.length);if(type.startsWith('touch')){window.__touchDiagnostics.push({type,trusted:event.isTrusted});return;}const surface=root.querySelector('.kartis-surface'),s=JSON.parse(window.hostFixture.editable_brush_snapshot(r));window.__trustedBrushTouch.push({r,type,pointerType:event.pointerType,trusted:event.isTrusted,capturedByHost:s.editor?.captured===event.pointerId,actualCapture:surface.hasPointerCapture(event.pointerId)});});});
      const tapFlush=async()=>{await tapPage.evaluate(()=>new Promise(resolve=>requestAnimationFrame(resolve)));await tapPage.evaluate(()=>window.hostFixture.editable_brush_flush());await tapPage.evaluate(()=>new Promise(resolve=>requestAnimationFrame(resolve)));await tapPage.evaluate(()=>window.hostFixture.editable_brush_flush());};await tapFlush();
      for(const r of ['svg','canvas']){
        const target=tapPage.locator(`#editable-brush-${r} .kartis-surface`);await target.scrollIntoViewIfNeeded();const b=await target.boundingBox(),s=await tapPage.evaluate(r=>JSON.parse(window.hostFixture.editable_brush_snapshot(r)),r);
        await tapPage.touchscreen.tap(b.x+100/s.width*b.width,b.y+90/s.height*b.height);await tapFlush();
        const result=await tapPage.evaluate(r=>({native:window.__trustedBrushTouch.filter(p=>p.r===r),diagnostics:window.__touchDiagnostics,snapshot:JSON.parse(window.hostFixture.editable_brush_snapshot(r)),pointerEvents:!!window.PointerEvent,maxTouchPoints:navigator.maxTouchPoints}),r);
        assert(result.native.length>=2&&result.native.every(p=>p.trusted&&p.pointerType==='touch')&&result.native.some(p=>p.capturedByHost&&p.actualCapture)&&result.snapshot.editor.selection===null&&!result.snapshot.editor.active&&result.snapshot.editor.captured===null,`${r} dedicated trusted touch tap/capture/zero span failed ${JSON.stringify(result)}`);
        touch.trustedTap.push({renderer:r,pointerPackets:result.native.length,allTrusted:true,captureObserved:true,zeroSpanCleared:true,dedicatedTouchContext:true,contextDpr:1});
      }
      await tapPage.evaluate(()=>window.hostFixture.editable_brush_stop());
    }finally{await context.close();await page.bringToFront();}
  }

  for(const mode of [0,1,2]){
    await page.evaluate(mode=>window.hostFixture.editable_brush_viewport_mount(mode),mode);await flush();await flush();
    for(const r of ['svg','canvas']){
      const first=await state(r),p=first.plot,selection=[p[0]+p[2]*.2,p[1]+p[3]*.25,p[0]+p[2]*.8,p[1]+p[3]*.75];
      await page.evaluate(r=>{const node=[...document.querySelectorAll(`#editable-brush-${r} [data-id]`)].find(n=>n.dataset.id==='line-point:5:alpha:1');window.__brushRetained??={};window.__brushRetained[r]=node;},r);
      await drag(r,selection.slice(0,2),selection.slice(2));await flush();
      const current=await state(r),ended=current.events.findLast(e=>e.phase==='End'&&e.source==='User'),actual=ended.selection;
      const inverseX=x=>10-(x-p[0])/p[2]*10,inverseY=y=>-10+(y-p[1])/p[3]*30;
      const expectedX=mode===2?[0,10]:[inverseX(actual[0]),inverseX(actual[2])].sort((a,b)=>a-b),expectedY=mode===1?[-10,20]:[inverseY(actual[1]),inverseY(actual[3])].sort((a,b)=>a-b);
      assert(rectEqual(current.domains.x,expectedX)&&rectEqual(current.domains.y,expectedY),`${r}/${mode} editable brush did not invert actual reverse scales exactly once ${JSON.stringify({actual,current:current.domains,expectedX,expectedY})}`);
      await page.evaluate(r=>window.hostFixture.editable_brush_viewport_phase(r,1,false),r);await flush();
      let updated=await state(r),fixed=updated.sites.find(s=>s.series==='alpha'&&s.source===1);
      assert(fixed?.raw[1]===12&&await page.evaluate(r=>[...document.querySelectorAll(`#editable-brush-${r} [data-id]`)].find(n=>n.dataset.id==='line-point:5:alpha:1')===window.__brushRetained[r],r),`${r}/${mode} current business values or retained phase node failed`);
      const checkIds=s=>{
        const [x,y,w,h]=s.plot,[x0,x1]=s.domains.x,[y0,y1]=s.domains.y,box=s.editor.selection;
        const data=[['alpha',[[1,-5],[5,12],[9,15]]],['beta',[[2,10],[6,2],[8,18]]]];
        const ids=data.flatMap(([name,points])=>points.flatMap(([rx,ry],index)=>{const px=x+w-(rx-x0)/(x1-x0)*w,py=y+(ry-y0)/(y1-y0)*h;return box&&rx>=x0&&rx<=x1&&ry>=y0&&ry<=y1&&px>=box[0]-1e-7&&px<=box[2]+1e-7&&py>=box[1]-1e-7&&py<=box[3]+1e-7?[`line-point:${name.length}:${name}:${index}`]:[];})).sort();
        assert(JSON.stringify([...s.editor.ids].sort())===JSON.stringify(ids),`${r}/${mode} selection IDs use stale geometry/data ${JSON.stringify({actual:s.editor.ids,ids})}`);
      };
      checkIds(updated);await page.evaluate(r=>window.hostFixture.editable_brush_viewport_phase(r,1,true),r);await flush();updated=await state(r);checkIds(updated);
      assert(updated.changedPlot&&updated.plot[0]===112&&updated.plot[1]===55&&rectEqual(updated.domains.x,current.domains.x)&&rectEqual(updated.domains.y,current.domains.y),`${r}/${mode} changed asymmetric plot altered accepted domains`);
      const markup=r==='svg'?await page.locator(`#editable-brush-${r} svg`).first().innerHTML():null,accepted=updated.domains;
      await page.evaluate(r=>window.hostFixture.editable_brush_viewport_reject(r,true),r);
      const q=updated.plot;await drag(r,[q[0]+q[2]*.3,q[1]+q[3]*.3],[q[0]+q[2]*.7,q[1]+q[3]*.7]);await flush();
      const rejected=await state(r);assert(rectEqual(rejected.domains.x,accepted.x)&&rectEqual(rejected.domains.y,accepted.y)&&rejected.events.some(e=>e.error==='intentional brush viewport layout rejection'),`${r}/${mode} rejected viewport callback committed a domain`);
      if(markup!==null)assert(await page.locator(`#editable-brush-${r} svg`).first().innerHTML()===markup,`${r}/${mode} rejected viewport modified accepted SVG`);
      await page.evaluate(r=>window.hostFixture.editable_brush_viewport_reject(r,false),r);
      viewport.push({renderer:r,axis:['XY','X','Y'][mode],reverseDomainInverse:true,currentData:true,retainedPhaseNode:true,changedPlot:true,rejectedDomainAtomic:true});
    }
    await page.evaluate(()=>window.hostFixture.editable_brush_stop());await flush();assert(await page.evaluate(()=>window.hostFixture.editable_brush_viewport_retired()),'Viewport brush callbacks survived stop');
  }

  for(const [mode,axis]of [['brush-xy','XY'],['brush-x','X'],['brush-y','Y']]){
    await page.locator(`[data-mode="${mode}"]`).click();await flush();
    for(const r of ['svg','canvas']){const current=await state(r);assert(current.active&&current.editor.axis===axis&&await page.locator(`#editable-brush-${r} .kartis-editable-brush`).count()===1,`${mode} navigation did not mount both real editors`);}
    await page.locator('[data-mode="transform"]').click();await flush();
    assert(await page.locator('.kartis-editable-brush').count()===0,'Switching to ordinary demo retained brush controls');
    for(const r of ['svg','canvas']){const stopped=await state(r);assert(!stopped.active&&stopped.callbackCleanup&&stopped.staleHandleRejected,`${mode} navigation retained a callback/handle`);}
    navigation.push({mode,axis,bothRenderers:true,ordinarySwitchCleanup:true});
  }
  await page.evaluate(()=>{delete window.__brushRetained;});
  return {positiveContexts,ink,safety,viewport,touch,navigation,publication,terminalPublication};
})
