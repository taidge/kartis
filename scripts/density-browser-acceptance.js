// Scene-generation + accepted BrowserChart changed-frame timing. No no-op or
// frame-rate/RSS inference. Each measured update moves every mark's rectangle.
(async page=>{
  const errors=[];const receive=e=>errors.push(e.message);page.on('pageerror',receive);
  const assert=(condition,message)=>{if(!condition)throw Error(message);};
  const frames=()=>page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
  const snapshot=()=>page.evaluate(()=>JSON.parse(window.hostFixture.density_snapshot()));
  const stats=samples=>{const sorted=[...samples].sort((a,b)=>a-b);return{samples,median:sorted.length%2?sorted[(sorted.length-1)/2]:(sorted[sorted.length/2-1]+sorted[sorted.length/2])/2,p95:sorted[Math.ceil(sorted.length*.95)-1],minimum:sorted[0],maximum:sorted.at(-1)};};
  const heap=()=>page.evaluate(()=>performance.memory?{usedJSHeapSize:performance.memory.usedJSHeapSize,totalJSHeapSize:performance.memory.totalJSHeapSize,jsHeapSizeLimit:performance.memory.jsHeapSizeLimit}:null);
  await page.waitForFunction(()=>window.hostReady===true&&typeof window.hostFixture.density_mount==='function');
  await page.setViewportSize({width:1280,height:1000});await page.emulateMedia({reducedMotion:'reduce'});
  await page.locator('.panels').evaluate(el=>{el.dataset.densityDisplay=el.style.display;el.style.display='none';});await frames();
  const baseline=await page.evaluate(()=>document.querySelectorAll('*').length), results=[],heapBefore=await heap();
  try{
    for(const renderer of['svg','canvas'])for(const count of[1000,5000,10000,20000]){
      await page.mouse.move(1,1);
      const cold=await page.evaluate(({renderer,count})=>{const start=performance.now();window.hostFixture.density_mount(renderer,count);const milliseconds=performance.now()-start;return{milliseconds,snapshot:JSON.parse(window.hostFixture.density_snapshot())};},{renderer,count});
      assert(cold.snapshot.active&&cold.snapshot.count===count&&cold.snapshot.pending===0,`${renderer}/${count}: initial accepted frame missing`);
      const surface=page.locator('#density-root .kartis-surface');await surface.scrollIntoViewIfNeeded();await frames();
      assert(await page.locator('#density-root .kartis-datum,#density-root .kartis-legend-item').count()===0,`${renderer}/${count}: per-datum controls contaminated density fixture`);
      const firstBefore=await page.locator('#density-root svg path[data-id="density-0"]').count()?await page.locator('#density-root svg path[data-id="density-0"]').getAttribute('d'):null;
      const canvasDigest=()=>page.locator('#density-root canvas').evaluate(canvas=>{const ctx=canvas.getContext('2d'),data=ctx.getImageData(0,0,canvas.width,canvas.height).data;let hash=2166136261;for(let i=0;i<data.length;i++)hash=Math.imul(hash^data[i],16777619);return hash>>>0;});
      const digestBefore=renderer==='canvas'?await canvasDigest():null;
      // One excluded warm-up. It is a genuine changed scene as well.
      await page.evaluate(()=>{window.hostFixture.density_update(1);window.hostFixture.density_flush();});await frames();
      const measurements=await page.evaluate(async()=>{
        const sync=[],frameIntervals=[],revisions=[];
        for(let i=0;i<8;i++){
          const start=performance.now();window.hostFixture.density_update(i+2);window.hostFixture.density_flush();sync.push(performance.now()-start);
          revisions.push(JSON.parse(window.hostFixture.density_snapshot()).revision);
          await new Promise(resolve=>requestAnimationFrame(()=>{frameIntervals.push(performance.now()-start);resolve();}));
        }return{sync,frameIntervals,revisions};
      });
      assert(new Set(measurements.revisions).size===8,`${renderer}/${count}: updates were not distinct accepted frames`);
      const current=await snapshot();assert(current.phase===9&&current.pending===0,`${renderer}/${count}: final frame did not settle`);
      if(renderer==='svg')assert(await page.locator('#density-root svg path[data-id="density-0"]').getAttribute('d')!==firstBefore,`${renderer}/${count}: timed SVG geometry did not change`);
      else assert(await canvasDigest()!==digestBefore,`${renderer}/${count}: timed Canvas pixels did not change`);

      const dom=await page.locator('#density-root').evaluate(el=>({descendants:el.querySelectorAll('*').length,paths:el.querySelectorAll('svg path[data-id]').length,svgs:el.querySelectorAll('svg').length,canvases:el.querySelectorAll('canvas').length}));
      if(renderer==='svg')assert(dom.paths===count,`${renderer}/${count}: actual shape count differs`);else assert(dom.canvases===1&&dom.paths===0,`${renderer}/${count}: Canvas allocated per-mark DOM`);
      let backing=null,pixelCheck;
      if(renderer==='canvas'){
        pixelCheck=await page.locator('#density-root canvas').evaluate((canvas,state)=>{
          const ctx=canvas.getContext('2d'),pixel=p=>[...ctx.getImageData(Math.floor(p.x/state.width*canvas.width),Math.floor(p.y/state.height*canvas.height),1,1).data];
          return{first:pixel(state.first),last:pixel(state.last),backing:{width:canvas.width,height:canvas.height,rgbaBytes:canvas.width*canvas.height*4,cssWidth:canvas.getBoundingClientRect().width,cssHeight:canvas.getBoundingClientRect().height}};
        },current);backing=pixelCheck.backing;
        assert(pixelCheck.first[2]>130&&pixelCheck.first[0]<100&&pixelCheck.first[3]===255,`${renderer}/${count}: first actual Canvas shape pixel missing`);
        const expectedBlue=(count-1)%2===0;assert(pixelCheck.last[expectedBlue?2:1]>120&&pixelCheck.last[0]<100,`${renderer}/${count}: last actual Canvas shape pixel missing`);
      }else{
        const points=await page.locator('#density-root svg path[data-id="density-0"],#density-root svg path[data-id="density-'+(count-1)+'"]').evaluateAll(paths=>paths.map(p=>{const r=p.getBoundingClientRect();return{id:p.dataset.id,width:r.width,height:r.height,d:p.getAttribute('d')};}));
        assert(points.length===2&&points.every(p=>p.width>0&&p.height>0),`${renderer}/${count}: actual SVG shape boxes missing`);pixelCheck={shapeBoxes:points};
        // Draw only the actual first semantic path into a tiny verification
        // image, outside all timed sections. Avoid serializing 20k SVG nodes.
        const ink=await page.locator('#density-root svg path[data-id="density-0"]').evaluate(async path=>{
          const svg=document.createElementNS('http://www.w3.org/2000/svg','svg');svg.setAttribute('width','640');svg.setAttribute('height','360');svg.setAttribute('viewBox','0 0 640 360');svg.appendChild(path.cloneNode(true));const image=new Image();image.src='data:image/svg+xml;charset=utf-8,'+encodeURIComponent(new XMLSerializer().serializeToString(svg));await image.decode();const canvas=document.createElement('canvas');canvas.width=1280;canvas.height=720;const ctx=canvas.getContext('2d');ctx.drawImage(image,0,0,1280,720);const data=ctx.getImageData(0,0,1280,720).data;let ink=0;for(let i=3;i<data.length;i+=4)if(data[i]>100)ink++;return ink;
        });assert(ink>0,`${renderer}/${count}: actual SVG path produced no pixels`);pixelCheck.verifiedFirstPathInk=ink;
      }
      // Native pointer listeners must still pick the current geometry even
      // though no per-datum controls or tooltips were allocated.
      const box=await surface.boundingBox(),point={x:box.x+current.first.x/current.width*box.width,y:box.y+current.first.y/current.height*box.height};
      await surface.evaluate(el=>el.addEventListener('pointermove',event=>{el.__densityPointer={x:event.clientX,y:event.clientY};},{once:true}));
      await page.mouse.move(point.x,point.y);await page.waitForFunction(()=>JSON.parse(window.hostFixture.density_snapshot()).hovered.includes('density-0'));await page.mouse.click(point.x,point.y);await page.waitForFunction(()=>JSON.parse(window.hostFixture.density_snapshot()).selected==='density-0');
      const deliveredPointer=await surface.evaluate(el=>el.__densityPointer);assert(deliveredPointer&&Math.abs(deliveredPointer.x-point.x)<.01&&Math.abs(deliveredPointer.y-point.y)<.01,`${renderer}/${count}: native pointer lost fractional coordinates`);
      const beforeCleanupHeap=await heap();await page.evaluate(()=>window.hostFixture.density_stop());await frames();assert(await page.locator('#density-root').count()===0,`${renderer}/${count}: root retained after stop`);assert(await page.evaluate(()=>document.querySelectorAll('*').length)===baseline,`${renderer}/${count}: DOM did not return to baseline`);
      results.push({renderer,marks:count,coldMountAndFlushMs:cold.milliseconds,warmChangedSetSceneAndFlushMs:stats(measurements.sync),changedUpdateToNextRafMs:stats(measurements.frameIntervals),revisions:measurements.revisions,dom,backing,pixelCheck,currentGeometryHit:true,fractionalPointer:{requested:point,delivered:deliveredPointer,logicalMark:current.first,surfaceOrigin:{x:box.x,y:box.y}},cleanupDomBaseline:baseline,jsHeapBeforeCleanup:beforeCleanupHeap,jsHeapAfterCleanup:await heap()});
    }
    assert(errors.length===0,errors.join('\n'));return{passed:true,fixture:'BrowserChart 640x360, retained SVG / Canvas, opaque rectangle marks',timingScope:'performance.now around Rust scene generation + set_scene + explicit flush; next RAF separately; excludes screenshot/pixel sampling/hit/cleanup',samplesPerCase:8,warmupChangedFrames:1,heapScope:'Optional Chromium performance.memory JS heap only; not process RSS, not full WASM/native/DOM allocation accounting; no forced GC',browser:await page.evaluate(()=>({userAgent:navigator.userAgent,devicePixelRatio,hardwareConcurrency:navigator.hardwareConcurrency})),baselineDomNodes:baseline,jsHeapBefore:heapBefore,jsHeapAfter:await heap(),cases:results,pageErrors:errors};
  }finally{
    await page.evaluate(()=>{window.hostFixture.density_stop();const panels=document.querySelector('.panels');if(panels){panels.style.display=panels.dataset.densityDisplay||'';delete panels.dataset.densityDisplay;}});await page.emulateMedia({reducedMotion:'no-preference'});page.off('pageerror',receive);
  }
})
