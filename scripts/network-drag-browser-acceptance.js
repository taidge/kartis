(async page=>{
 const assert=(ok,message)=>{if(!ok)throw Error(message)};
 const errors=[];const onError=e=>errors.push(e.message);page.on('pageerror',onError);
 const snapshot=id=>page.evaluate(id=>JSON.parse(window.hostFixture.network_drag_snapshot(id)),id);
 const wait=async(id,predicate)=>{const deadline=Date.now()+30000;let current;while(Date.now()<deadline){current=await snapshot(id);if(predicate(current))return current;await page.evaluate(()=>new Promise(requestAnimationFrame));}throw Error(`Network ${id} did not reach state: ${JSON.stringify(current)}`)};
 const ready=id=>wait(id,s=>s.active&&!s.pending&&s.chart?.pending===0);
 const flush=()=>page.evaluate(()=>window.hostFixture.network_drag_flush());
 const mousePoint=()=>page.evaluate(()=>window.networkTrustedPointer);
 const point=async(id,node='a',offset=[0,0])=>{
  const surface=page.locator(`#network-${id} .kartis-surface`);await surface.scrollIntoViewIfNeeded();await ready(id);
  const s=await snapshot(id),n=s.nodes.find(n=>n.id===node),box=await surface.boundingBox();
  const x=(n.x+s.chart.width/2+10)*s.chart.view.scale+s.chart.view.x;
  const y=(n.y+s.chart.height/2-10)*s.chart.view.scale+s.chart.view.y;
  return{x:box.x+x/s.chart.width*box.width+offset[0],y:box.y+y/s.chart.height*box.height+offset[1],state:s,node:n,box};
 };
 const cooled=id=>wait(id,s=>s.active&&!s.dragging&&!s.captured&&!s.raf&&s.alpha<=0.001&&s.target===0&&!s.pending&&s.chart?.pending===0);
 const baseline=await page.locator('body *').count();const reports=[];
 try{
  // Playwright engines quantize physical mouse coordinates differently. Read
  // the actual trusted native input independently, before the drag listener,
  // then test its CSS-to-world mapping without loosening numeric tolerances.
  await page.evaluate(()=>{window.networkRecordPointer=e=>{if(e.isTrusted&&e.pointerType==='mouse')window.networkTrustedPointer=[e.clientX,e.clientY];};for(const type of ['pointerdown','pointermove','pointerup'])window.addEventListener(type,window.networkRecordPointer,true);});
  await page.evaluate(()=>window.hostFixture.network_drag_mount());
  for(const id of ['svg','canvas']){
   await ready(id);await wait(id,s=>!s.raf);const initial=await snapshot(id);assert(initial.ticks===0,'Idle initial simulation unexpectedly ran');
   await page.evaluate(id=>document.querySelector(`#network-${id}`).style.display='none',id);await flush();const suspended=await snapshot(id);assert(suspended.active&&suspended.suspended&&!suspended.captured,'Hidden host was destroyed rather than suspended');await page.evaluate(id=>document.querySelector(`#network-${id}`).style.display='',id);await wait(id,s=>s.active&&!s.suspended&&!s.raf);
   let p=await point(id,'a',[4,3]);await page.mouse.move(p.x,p.y);await page.mouse.down();
   const started=await wait(id,s=>s.dragging?.node==='a'&&s.ticks>0&&s.captured!==null);
   const initialClient=await mousePoint();
   assert(Math.abs(started.nodes.find(n=>n.id==='a').x-p.node.x)<1e-8,'Pointer down jumped node to pointer center');
   assert(started.target>=0.3,'Drag did not heat force simulation');
   await page.evaluate(({id,x,y})=>{const surface=document.querySelector(`#network-${id} .kartis-surface`);for(const type of ['pointerdown','pointermove','pointerup'])surface.dispatchEvent(new PointerEvent(type,{bubbles:true,pointerId:999,isPrimary:true,pointerType:'touch',button:0,buttons:type==='pointerup'?0:1,clientX:x+20,clientY:y+10}));},{id,x:p.x,y:p.y});
   await flush();const foreign=await snapshot(id);assert(foreign.captured===started.captured&&foreign.dragging?.node==='a'&&JSON.stringify(foreign.chart.view)===JSON.stringify(started.chart.view),'A foreign pointer stole node ownership or started camera pan/pinch');
   await page.mouse.move(p.x+60,p.y+30,{steps:5});
   const movedClient=await mousePoint(),expectedMove=[p.node.x+(movedClient[0]-initialClient[0])*p.state.chart.width/p.box.width,p.node.y+(movedClient[1]-initialClient[1])*p.state.chart.height/p.box.height];
   const moved=await wait(id,s=>s.dragging?.node==='a'&&Math.abs(s.nodes.find(n=>n.id==='a').x-expectedMove[0])<0.01);
   const a=moved.nodes.find(n=>n.id==='a');assert(Math.abs(a.y-expectedMove[1])<0.01,'Drag CSS to world Y delta wrong');
   assert(a.fixed&&Math.abs(a.fixed[0]-a.x)<1e-8,'Dragged node was not held through actual force ticks');
   const paint=await page.evaluate(({id,x,y})=>{const root=document.querySelector(`#network-${id}`),canvas=root.querySelector('canvas');if(canvas){const s=JSON.parse(window.hostFixture.network_drag_snapshot(id)),px=(x+s.chart.width/2+10)*s.chart.view.scale+s.chart.view.x,py=(y+s.chart.height/2-10)*s.chart.view.scale+s.chart.view.y;return{pixel:[...canvas.getContext('2d').getImageData(Math.floor(px*canvas.width/s.chart.width),Math.floor(py*canvas.height/s.chart.height),1,1).data]};}const path=root.querySelector('[data-id="force-node:a"]'),b=path.getBBox();return{cx:b.x+b.width/2,cy:b.y+b.height/2};},{id,x:a.x,y:a.y});
   if(id==='svg')assert(Math.abs(paint.cx-(a.x+moved.chart.width/2+10))<.001,'Live SVG did not repaint actual node physics');
   else assert(paint.pixel[3]===255&&Math.min(...paint.pixel.slice(0,3))<220,'Actual Canvas pixels did not move with node');
   await page.mouse.up();const released=await ready(id);assert(!released.dragging&&!released.captured&&released.target===0&&released.nodes.find(n=>n.id==='a').fixed===null,'Up did not release free node or restore heat target');await cooled(id);
   const stable=await snapshot(id);await page.evaluate(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))));assert((await snapshot(id)).ticks===stable.ticks,'Cooled force RAF did not become idle');
   await page.evaluate(id=>window.hostFixture.network_drag_view(id,2,-120,-80),id);await flush();await wait(id,s=>Math.abs(s.chart.view.scale-2)<1e-8);
   p=await point(id,'a',[3,2]);const before=p.node;await page.mouse.move(p.x,p.y);await page.mouse.down();await wait(id,s=>s.dragging?.node==='a');const zoomStart=await mousePoint();await page.mouse.move(p.x+48,p.y+24,{steps:4});
   const zoomEnd=await mousePoint(),expectedZoom=[before.x+(zoomEnd[0]-zoomStart[0])*p.state.chart.width/p.box.width/2,before.y+(zoomEnd[1]-zoomStart[1])*p.state.chart.height/p.box.height/2];
   const zoomed=await wait(id,s=>s.dragging?.node==='a'&&Math.abs(s.nodes.find(n=>n.id==='a').x-expectedZoom[0])<.02);assert(Math.abs(zoomed.nodes.find(n=>n.id==='a').y-expectedZoom[1])<.02,'Zoomed drag applied camera inverse twice');
   const resizeWorld=zoomed.nodes.find(n=>n.id==='a');await page.evaluate(id=>window.hostFixture.network_drag_resize(id,500),id);
   await wait(id,s=>s.chart.width===500&&s.dragging?.node==='a');await page.mouse.move(p.x+48,p.y+24);await flush();
   const rebased=await snapshot(id);assert(Math.abs(rebased.nodes.find(n=>n.id==='a').x-resizeWorld.x)<.02&&Math.abs(rebased.nodes.find(n=>n.id==='a').y-resizeWorld.y)<.02,'Resize/camera rebase jumped the held world node');
   const resizeClient=await mousePoint(),resizeBox=await page.locator(`#network-${id} .kartis-surface`).boundingBox();await page.mouse.move(p.x+72,p.y+24,{steps:3});const resizeEnd=await mousePoint();await wait(id,s=>Math.abs(s.nodes.find(n=>n.id==='a').x-(resizeWorld.x+(resizeEnd[0]-resizeClient[0])*500/resizeBox.width/2))<.02);await page.mouse.up();await cooled(id);
   await page.evaluate(id=>window.hostFixture.network_drag_view(id,1,0,0),id);await flush();await wait(id,s=>s.chart.view.scale===1);
   p=await point(id,'b');
   await page.mouse.move(p.x,p.y);await page.evaluate(()=>{window.networkOldRAF=window.requestAnimationFrame;let ticket=10000000;window.requestAnimationFrame=()=>++ticket;});
   try {await page.mouse.down();await flush();assert((await snapshot(id)).dragging?.node==='b','First trusted gesture did not start');await page.mouse.up();await page.mouse.down();await flush();const ordered=await snapshot(id);assert(ordered.captured!==null&&ordered.dragging?.node==='b'&&ordered.error===null,'Queued Up then Down lost the new pointer gesture');}
   finally {await page.evaluate(()=>{window.requestAnimationFrame=window.networkOldRAF;delete window.networkOldRAF});await flush();await page.mouse.up();}await cooled(id);
   p=await point(id,'b');const prior=p.node.fixed;assert(prior,'Pinned fixture missing');await page.mouse.move(p.x,p.y);await page.mouse.down();await wait(id,s=>s.dragging?.node==='b');await page.mouse.move(p.x-30,p.y+18,{steps:3});
   await page.evaluate(id=>{const s=JSON.parse(window.hostFixture.network_drag_snapshot(id)),surface=document.querySelector(`#network-${id} .kartis-surface`);if(surface.hasPointerCapture(s.captured))surface.releasePointerCapture(s.captured);surface.dispatchEvent(new PointerEvent('lostpointercapture',{bubbles:true,pointerId:s.captured,isPrimary:true}));},id);
   const cancelled=await wait(id,s=>!s.dragging&&!s.captured&&s.target===0);assert(cancelled.nodes.find(n=>n.id==='b').fixed.every((v,i)=>Math.abs(v-prior[i])<1e-8),'Lost capture did not restore original pinned position');await page.mouse.up();await cooled(id);
   p=await point(id,'a');await page.mouse.move(p.x,p.y);await page.mouse.down();await wait(id,s=>s.dragging?.node==='a');
   await page.evaluate(id=>{const s=JSON.parse(window.hostFixture.network_drag_snapshot(id)),surface=document.querySelector(`#network-${id} .kartis-surface`),e=new PointerEvent('pointerup',{bubbles:true,pointerId:s.captured,isPrimary:true});Object.defineProperty(e,'clientX',{value:NaN});surface.dispatchEvent(e);},id);
   await wait(id,s=>!s.dragging&&!s.captured&&s.target===0);await page.mouse.up();await cooled(id);
   for(const policy of ['blur','hidden']) {
    p=await point(id,'b');const fixed=p.node.fixed;await page.mouse.move(p.x,p.y);await page.mouse.down();await wait(id,s=>s.dragging?.node==='b');
    await page.evaluate(policy=>{if(policy==='blur')window.dispatchEvent(new Event('blur'));else{const old=Object.getOwnPropertyDescriptor(document,'hidden');Object.defineProperty(document,'hidden',{configurable:true,value:true});document.dispatchEvent(new Event('visibilitychange'));if(old)Object.defineProperty(document,'hidden',old);else delete document.hidden;}},policy);
    await flush();const ended=await wait(id,s=>!s.dragging&&!s.captured&&s.target===0);assert(ended.nodes.find(n=>n.id==='b').fixed.every((v,i)=>Math.abs(v-fixed[i])<1e-8),`${policy} failed to restore prior pin`);await page.mouse.up();await cooled(id);
   }
   p=await point(id,'a');await page.mouse.move(p.x,p.y);await page.mouse.down();await wait(id,s=>s.dragging?.node==='a');await page.evaluate(id=>window.hostFixture.network_drag_hidden(id,true),id);await flush();
   const hidden=await wait(id,s=>s.chart.hidden.includes('a')&&!s.dragging&&!s.captured);assert(hidden.target===0,'Hidden held node kept simulation permanently hot');await page.mouse.up();
   if(id==='svg')assert(await page.locator('#network-svg text').filter({hasText:'Drag free A'}).count()===0,'Hidden network retained node-owned label');
   await page.evaluate(id=>window.hostFixture.network_drag_hidden(id,false),id);await flush();await cooled(id);
   assert((await snapshot(id)).error===null,'Network driver reported an unexpected error');reports.push({renderer:id,pointerCapture:true,grabOffset:true,actualPaint:true,heatAndIdle:true,independentCamera:true,resizeRebase:true,lostCapture:true,invalidUpTerminates:true,foreignPointerIgnored:true,orderedRapidGestures:true,surfaceSuspension:true,blurAndVisibilityCancel:true,hiddenCancelsDrag:true});
  }
  await page.evaluate(()=>window.hostFixture.network_drag_arm_stop('svg'));let p=await point('svg','a');const stale=await page.locator('#network-svg .kartis-surface').elementHandle();await page.mouse.move(p.x,p.y);await page.mouse.down();
  const stopped=await wait('svg',s=>!s.active);assert(!stopped.captured&&!stopped.raf&&stopped.target===0,'Callback stop retained capture/heat/RAF');assert(await page.locator('#network-svg').evaluate(e=>e.childElementCount)===0,'Callback stop retained actual chart DOM');await page.mouse.up();
  await stale.evaluate(e=>e.dispatchEvent(new PointerEvent('pointermove',{bubbles:true,pointerId:1,isPrimary:true,clientX:20,clientY:20})));await page.evaluate(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))));assert((await snapshot('svg')).ticks===stopped.ticks,'Stale listener advanced stopped physics');
  await page.evaluate(()=>window.hostFixture.network_drag_drop('canvas'));assert(!(await snapshot('canvas')).active,'Drop retained active network owner');assert(await page.locator('#network-canvas').evaluate(e=>e.childElementCount)===0,'Drop retained Canvas chart DOM');
  await page.evaluate(()=>window.hostFixture.network_drag_stop());
  await page.evaluate(()=>window.hostFixture.network_drag_mount());
  const replacements=[];
  for(const id of ['svg','canvas']) {
   await ready(id);await wait(id,s=>!s.raf);
   await page.evaluate(id=>{window.hostFixture.network_drag_select(id,'b');window.hostFixture.network_drag_hidden(id,true);},id);await flush();
   const before=await snapshot(id);assert(before.chart.selected==='force-node:b'&&before.chart.hidden.includes('a'),'Replacement selection/hidden fixture missing');
   if(id==='svg')await page.evaluate(()=>window.networkOldB=document.querySelector('#network-svg [data-id="force-node:b"]'));
   p=await point(id,'b');await page.mouse.move(p.x,p.y);await page.mouse.down();await wait(id,s=>s.dragging?.node==='b');
   await page.evaluate(id=>window.hostFixture.network_drag_replace(id,0),id);await flush();
   const changed=await wait(id,s=>s.nodes.map(n=>n.id).join(',')==='b,d'&&!s.pending&&!s.raf);
   assert(!changed.dragging&&!changed.captured&&changed.target===0&&changed.nodes[0].fixed===null,'Graph update retained old capture/pin/drag heat');
   assert(changed.chart.selected==='force-node:b'&&changed.chart.hidden.length===0,'Graph update did not retain common selection/prune removed hidden IDs');await page.mouse.up();
   if(id==='svg')assert(await page.evaluate(()=>document.querySelector('#network-svg [data-id="force-node:b"]')===window.networkOldB),'Graph update remounted a retained node ID');
   assert((await page.locator(`#network-${id} .kartis-data`).textContent()).includes('Updated B')&&!(await page.locator(`#network-${id} .kartis-data`).textContent()).includes('Moving C'),'Graph data controls used old topology');
   const current=await point(id,'d');await page.mouse.move(current.x,current.y);await wait(id,s=>s.chart.hovered.includes('force-node:d'));
   assert((await page.locator(`#network-${id} .kartis-surface-tooltip`).textContent()).includes('New D'),'Updated graph tooltip used old datum');
   await page.mouse.move(2,2);await wait(id,s=>!s.raf&&!s.pending&&s.chart.pending===0);
   const refused=await page.evaluate(id=>{const before=JSON.parse(window.hostFixture.network_drag_snapshot(id));const old=window.requestAnimationFrame;window.requestAnimationFrame=()=>{throw Error('network replacement RAF refusal')};let rejected=false;try{window.hostFixture.network_drag_replace(id,1)}catch(_){rejected=true}finally{window.requestAnimationFrame=old}const after=JSON.parse(window.hostFixture.network_drag_snapshot(id));return{rejected,same:JSON.stringify(before.nodes)===JSON.stringify(after.nodes)&&before.chart.revision===after.chart.revision&&!after.pending&&after.active};},id);
   assert(refused.rejected&&refused.same,'Failed graph RAF admission changed accepted physics/DOM/pending slot');
   const invalid=await page.evaluate(id=>{const before=JSON.parse(window.hostFixture.network_drag_snapshot(id));const rejected=window.hostFixture.network_drag_reject();const after=JSON.parse(window.hostFixture.network_drag_snapshot(id));return rejected&&JSON.stringify(before.nodes)===JSON.stringify(after.nodes)&&before.chart.revision===after.chart.revision&&!after.pending},id);assert(invalid,'Malformed graph changed accepted network');
   await page.evaluate(id=>{window.hostFixture.network_drag_hide_node(id,'d',true);window.hostFixture.network_drag_select(id,'b')},id);await flush();
   const pending=await page.evaluate(id=>{window.hostFixture.network_drag_replace(id,1);window.hostFixture.network_drag_replace(id,2);return JSON.parse(window.hostFixture.network_drag_snapshot(id))},id);assert(pending.pending&&pending.nodes.map(n=>n.id).join(',')==='b,d','Replacement was not a deferred latest candidate');
   await flush();const final=await wait(id,s=>s.nodes.map(n=>n.id).join(',')==='d,e'&&!s.pending&&!s.raf);
   assert(final.chart.selected===null&&final.chart.hidden.join(',')==='d','Latest graph did not reconcile removed selection/common hidden node');
   const controls=await page.locator(`#network-${id} .kartis-data`).textContent();assert(controls.includes('Final E')&&!controls.includes('Updated B')&&!controls.includes('Final D'),'Current graph owned visibility/control metadata wrong');
   const failure=await page.evaluate(id=>{
    const before=JSON.parse(window.hostFixture.network_drag_snapshot(id)),root=document.querySelector(`#network-${id}`),html=root.innerHTML;
    window.hostFixture.network_drag_replace(id,3);let rejected=false;try{window.hostFixture.network_drag_flush()}catch(_){rejected=true;}
    const after=JSON.parse(window.hostFixture.network_drag_snapshot(id));const failures=JSON.parse(window.hostFixture.network_drag_events(id)).filter(e=>e.event.startsWith('Error('));
    return{rejected,active:after.active,oldPhysics:JSON.stringify(before.nodes)===JSON.stringify(after.nodes),oldDOM:html===root.innerHTML,errors:failures.length,pending:after.pending};
   },id);assert(failure.rejected&&failure.active&&failure.oldPhysics&&failure.oldDOM&&failure.errors===1&&!failure.pending,'Late force candidate failure: '+JSON.stringify(failure));
   await page.evaluate(id=>window.hostFixture.network_drag_replace(id,2),id);await flush();await wait(id,s=>s.active&&!s.pending&&!s.raf);
   await page.evaluate(id=>{window.hostFixture.network_drag_arm_recover(id);window.hostFixture.network_drag_replace(id,3);try{window.hostFixture.network_drag_flush()}catch(_){}},id);
   await wait(id,s=>s.active&&!s.pending&&!s.raf&&s.chart.selected==='force-node:e');
   const reentrantEvents=await page.evaluate(id=>JSON.parse(window.hostFixture.network_drag_events(id)),id);assert(!reentrantEvents.some(e=>e.recoveryError)&&reentrantEvents.some(e=>e.event?.includes('Selected')&&e.event.includes('force-node:e')),'Error callback nested flush lost a deferred selection event');
   replacements.push({renderer:id,currentTopology:true,retainedNodeIdentity:true,currentTooltip:true,selectionAndHidden:true,latestCandidate:true,rafRejectedAtomic:true,sourceRejectedAtomic:true,captureRetired:true,lateFailureAtomic:true,recoverable:true,errorCallbackReentrant:true});
  }
  await page.evaluate(()=>window.hostFixture.network_drag_stop());
  await page.evaluate(()=>window.hostFixture.network_drag_mount());await ready('svg');p=await point('svg','a');await page.mouse.move(p.x,p.y);await page.mouse.down();await wait('svg',s=>s.dragging?.node==='a');await page.evaluate(()=>window.dispatchEvent(new Event('pagehide')));await page.mouse.up();
  const retired=await snapshot('svg');assert(!retired.active&&!retired.captured&&!retired.raf&&retired.target===0,'pagehide did not stop network/capture/heat');
  await page.evaluate(()=>window.hostFixture.network_drag_stop());assert(await page.locator('body *').count()===baseline,'Network stop did not return DOM to baseline');assert(!errors.length,errors.join('\n'));
  return{passed:true,renderers:reports,replacements,callbackStop:true,pagehideCleanup:true,staleListenerCleanup:true,drop:true,domBaseline:true,pageErrors:errors};
 }finally{await page.mouse.up();await page.evaluate(()=>window.hostFixture.network_drag_stop());await page.evaluate(()=>{delete window.networkOldB;for(const type of ['pointerdown','pointermove','pointerup'])window.removeEventListener(type,window.networkRecordPointer,true);delete window.networkRecordPointer;delete window.networkTrustedPointer;});page.off('pageerror',onError);}
})




