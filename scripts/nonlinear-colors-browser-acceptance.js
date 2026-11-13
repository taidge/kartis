(async function verifyNonlinearColors(page, options = {}) {
  const assert = (ok, message) => { if (!ok) throw Error(message); };
  const {readFile, writeFile} = await import('node:fs/promises');
  const {execFileSync} = await import('node:child_process');
  const {join} = await import('node:path');
  const {pathToFileURL} = await import('node:url');
  const {createHash, randomUUID} = await import('node:crypto');
  const hash = bytes => createHash('sha256').update(bytes).digest('hex');
  const metadata = JSON.parse(execFileSync('cargo', ['metadata','--no-deps','--format-version','1'], {encoding:'utf8',windowsHide:true}));
  const modules = join(metadata.target_directory,'d3-color-reference','node_modules');
  const manifest = JSON.parse(await readFile('tests/fixtures/nonlinear-color-d3-source.json','utf8'));
  for (const [path, sha] of Object.entries(manifest.sources)) assert(hash(await readFile(join(modules,path)))===sha,`Original D3 source changed: ${path}`);
  const [d3,{piecewise,interpolateRgb}] = await Promise.all(['d3-scale','d3-interpolate'].map(name=>import(pathToFileURL(join(modules,name,'src/index.js')).href)));
  const descriptors = [
    ['Linear',1,[-11,17],3,false], ['Log',1,[1,1000],10,false], ['Log',1,[-1000,-1],-10,false],
    ['Pow',2,[-4,9],0,false], ['Sqrt',.5,[-4,9],0,false], ['Symlog',2,[-100,200],0,false],
    ['Log',1,[1,1000],10,true], ['Symlog',2,[-100,200],0,true],
    ['Pow',-1,[1,100],10,false], ['Pow',-1,[1,100],10,true],
  ];
  const palette = diverging => diverging ? [[10,30,80,255],[30,220,130,255],[220,30,40,255]] : [[5,30,90,255],[245,220,20,255]];
  const css = color=>`rgba(${color[0]},${color[1]},${color[2]},${color[3]/255})`;
  const rgba = text=>{const out=text.match(/[+-]?(?:\d*\.?\d+)(?:e[+-]?\d+)?/gi).map(Number);if(out.length===3)out.push(1);out[3]=Math.round(out[3]*255);return out;};
  const oracle = phase => {
    const [name,parameter,domain,midpoint,diverging]=descriptors[phase];
    const configure = scale=>{if(name==='Pow')scale.exponent(parameter);if(name==='Symlog')scale.constant(parameter);return scale;};
    const suffix=name==='Linear'?'':name;
    const color=configure(d3[`scale${diverging?'Diverging':'Sequential'}${suffix}`]()).domain(diverging?[domain[0],midpoint,domain[1]]:domain).clamp(true).interpolator(piecewise(interpolateRgb,palette(diverging).map(css))).unknown(css([123,45,67,255]));
    const axis=configure(d3[`scale${name}`]()).domain(domain).range([0,1]);
    return {name,parameter,domain,midpoint,diverging,color,axis,values:[domain[0],midpoint,domain[1],null]};
  };
  const errors=[],onError=error=>errors.push(error.message);page.on('pageerror',onError);
  const previous=await page.evaluate(()=>{const panels=document.querySelector('.panels'),old=panels.style.display;panels.style.display='none';return old;});
  let publicationCounterMutations=0;
  const baseline=await page.locator('body *').count(), reports=[],ink=[],negativeControls=[],nativeEvents=[],resetFallback=[],gradientFaults=[],evidenceId=randomUUID();
  const state=renderer=>page.evaluate(renderer=>JSON.parse(window.hostFixture.nonlinear_colors_snapshot(renderer)),renderer);
  const surface=renderer=>page.locator(`#nonlinear-colors-${renderer} .kartis-surface`);
  const flush=async()=>{await page.evaluate(()=>new Promise(requestAnimationFrame));await page.evaluate(()=>window.hostFixture.nonlinear_colors_flush());await page.evaluate(()=>new Promise(requestAnimationFrame));await page.evaluate(()=>window.hostFixture.nonlinear_colors_flush());};
  const move=async(renderer,point)=>{await surface(renderer).scrollIntoViewIfNeeded();await flush();const s=await state(renderer),box=await surface(renderer).boundingBox();await page.mouse.move(box.x+point[0]/s.width*box.width,box.y+point[1]/s.height*box.height);await flush();};
  const close=(a,b)=>Math.abs(a-b)<=1e-7*Math.max(1,Math.abs(b));
  const delta=(a,b)=>Math.max(...a.map((value,i)=>Math.abs(value-b[i])));
  const canvasResetSupported=await page.evaluate(()=>typeof CanvasRenderingContext2D.prototype.reset==='function');
  const capture=async(renderer,s,consumer,phase,scope='modern')=>{
    await surface(renderer).scrollIntoViewIfNeeded();await flush();
    const bytes=await surface(renderer).screenshot();
    const points=[...s.sites.map(site=>site.position),...[.2,.5,.8].map(t=>[80+450*t,320])];
    const pixels=await page.evaluate(async({png,points,width,height})=>{
      const image=await createImageBitmap(new Blob([new Uint8Array(png)],{type:'image/png'}));
      const canvas=document.createElement('canvas');canvas.width=image.width;canvas.height=image.height;const ctx=canvas.getContext('2d');ctx.drawImage(image,0,0);image.close();
      const samples=points.map(point=>{const x=Math.min(canvas.width-1,Math.max(0,Math.round(point[0]/width*canvas.width-.5))),y=Math.min(canvas.height-1,Math.max(0,Math.round(point[1]/height*canvas.height-.5)));return{point:[(x+.5)/canvas.width*width,(y+.5)/canvas.height*height],pixel:[...ctx.getImageData(x,y,1,1).data]};});
      return{backing:[canvas.width,canvas.height],samples};
    },{png:[...bytes],points,width:s.width,height:s.height});
    const expected=oracle(phase);
    for(let i=0;i<4;i++)assert(delta(pixels.samples[i].pixel,rgba(expected.color(expected.values[i]??undefined)))<=1,`${renderer} consumer${consumer} phase${phase} actual datum PNG ${i}: ${JSON.stringify(pixels.samples[i])}`);
    for(const sample of pixels.samples.slice(4)){
      const value=expected.axis.invert((sample.point[0]-80)/450),color=rgba(expected.color(value));
      // A locator screenshot rounds a fractional CSS clip to device pixels.
      // This permits at most two RGBA8 levels at an interior gradient sample.
      assert(delta(sample.pixel,color)<=2,`${renderer} consumer${consumer} phase${phase} actual legend PNG: ${JSON.stringify({sample,value,color})}`);
    }
    const path=`target/gallery/nonlinear-colors-${evidenceId}-${renderer}-c${consumer}-p${phase}-${scope}-dpr${await page.evaluate(()=>devicePixelRatio)}.png`;
    await writeFile(path,bytes);
    const receipt={renderer,consumer,phase,path,sha256:hash(bytes),backing:pixels.backing,dataSamples:4,legendSamples:3,originalD3Pixels:true};
    if(scope==='modern')ink.push(receipt);
    return {bytes,receipt};
  };
  const samePaint=async phase=>{
    await page.evaluate(phase=>{window.__nonlinearMutations=0;window.__nonlinearCounterMutations=0;window.__nonlinearObserver=new MutationObserver(rows=>{for(const row of rows){if(row.type==='attributes'&&row.attributeName==='data-revision'&&row.target.matches('.kartis-surface'))window.__nonlinearCounterMutations++;else window.__nonlinearMutations++;}});for(const renderer of ['svg','canvas'])window.__nonlinearObserver.observe(document.querySelector(`#nonlinear-colors-${renderer} .kartis-surface`),{subtree:true,attributes:true,childList:true,characterData:true});window.hostFixture.nonlinear_colors_phase(phase);window.hostFixture.nonlinear_colors_flush();},phase);
    await flush();assert(await page.evaluate(()=>{window.__nonlinearObserver.disconnect();return window.__nonlinearMutations===0;}),'Identical nonlinear color frame mutated paint DOM');publicationCounterMutations+=await page.evaluate(()=>window.__nonlinearCounterMutations);
  };
  try {
    for(const consumer of [0,1,2]) {
      await page.mouse.move(2,2);await page.evaluate(consumer=>window.hostFixture.nonlinear_colors_mount(consumer),consumer).catch(error=>{throw Error(`Consumer${consumer} mount: ${error.message}`);});await flush();
      await page.evaluate(()=>{
        window.__nonlinearPackets=[];window.__nonlinearKeys=[];
        window.__nonlinearPointer=e=>{if(e.target.closest?.('[id^="nonlinear-colors-"] .kartis-surface'))window.__nonlinearPackets.push({type:e.type,client:[e.clientX,e.clientY],trusted:e.isTrusted});};
        window.__nonlinearKey=e=>{if(e.target.closest?.('[id^="nonlinear-colors-"]'))window.__nonlinearKeys.push({key:e.key,trusted:e.isTrusted});};
        document.addEventListener('pointermove',window.__nonlinearPointer,true);document.addEventListener('keydown',window.__nonlinearKey,true);
        window.__nonlinearRetained={};
      });
      for(const phase of [0,1,2,3,4,5,6,7,8,9]) {
        await page.mouse.move(2,2);await page.evaluate(()=>document.activeElement?.blur());await flush();
        if(phase){await page.evaluate(phase=>window.hostFixture.nonlinear_colors_phase(phase),phase).catch(error=>{throw Error(`Consumer${consumer} phase${phase}: ${error.message}`);});await flush();}
        const expected=oracle(phase);
        for(const renderer of ['svg','canvas']) {
          const s=await state(renderer);assert(s.active&&s.phase===phase&&s.consumer===consumer&&s.sites.length===4,'Nonlinear color source/typed data missing');
          assert(JSON.stringify(s.domain)===JSON.stringify(expected.domain),'Nonlinear input domain changed');
          for(let i=0;i<4;i++)assert(s.sites[i].value===expected.values[i]&&delta(s.sites[i].color,rgba(expected.color(expected.values[i]??undefined)))<=1,`${renderer} c${consumer} p${phase} current raw color data ${i}`);
          for(const tick of s.ticks)assert(close(tick.position[0],80+450*expected.axis(tick.value)),`${renderer} c${consumer} p${phase} legend tick uses a linear axis`);
          const retained=await page.evaluate(({renderer,target,phase})=>{
            const root=document.getElementById(`nonlinear-colors-${renderer}`),buttons=[...root.querySelectorAll('.kartis-datum')],button=buttons.length===4?buttons[1]:null;
            const paint=renderer==='svg'?[...root.querySelectorAll('svg [data-id]')].find(n=>n.dataset.id===target.id):root.querySelector('canvas');
            if(!phase){window.__nonlinearRetained[renderer]={button,paint};return !!button&&!!paint;}
            const old=window.__nonlinearRetained[renderer];return old.button===button&&old.paint===paint&&!!button&&!!paint;
          },{renderer,target:s.sites[1],phase});assert(retained,`${renderer} consumer${consumer} phase${phase} replaced retained data/control node`);
          const {bytes:before}=await capture(renderer,s,consumer,phase);
          if(phase===0){
            await surface(renderer).evaluate(node=>node.style.filter='grayscale(1)');let after;
            try{after=await surface(renderer).screenshot();}finally{await surface(renderer).evaluate(node=>node.style.filter='');}
            const changed=await page.evaluate(async({a,b})=>{const load=async bytes=>{const image=await createImageBitmap(new Blob([new Uint8Array(bytes)],{type:'image/png'}));const canvas=document.createElement('canvas');canvas.width=image.width;canvas.height=image.height;const ctx=canvas.getContext('2d');ctx.drawImage(image,0,0);image.close();return ctx.getImageData(0,0,canvas.width,canvas.height).data;};const left=await load(a),right=await load(b);let count=0;for(let i=0;i<left.length;i+=4)count+=Number(Math.abs(left[i]-right[i])+Math.abs(left[i+1]-right[i+1])+Math.abs(left[i+2]-right[i+2])>20);return count;},{a:[...before],b:[...after]});
            assert(changed>=8,`${renderer} actual color PNG detector did not observe negative control`);negativeControls.push({renderer,consumer,changedPixels:changed,actualCssFilter:true});
          }
          for(const index of [1,3]) {
            const target=s.sites[index];await move(renderer,target.position);const hovered=await state(renderer);
            assert(hovered.hovered.includes(target.id),`${renderer} nonlinear current datum hover failed`);
            assert(hovered.interaction?.rows.some(row=>row.id===target.id&&row.value===target.value),`${renderer} nonlinear typed tooltip lost original/null value`);
            const tooltip=page.locator(`#nonlinear-colors-${renderer} .kartis-surface-tooltip`);assert(await tooltip.isVisible(),`${renderer} c${consumer} p${phase} row${index} nonlinear tooltip is hidden`);
            assert((await tooltip.textContent()).includes(`${target.label}: ${target.value??'missing'}`),`${renderer} nonlinear actual tooltip lost raw value: ${await tooltip.textContent()}`);
          }
          const target=s.sites[1],button=page.locator(`#nonlinear-colors-${renderer} .kartis-datum`).nth(1);
          await button.focus();await flush();const eventCount=(await state(renderer)).events.length;await page.keyboard.press('Enter');await flush();
          assert((await state(renderer)).events.slice(eventCount).some(event=>event.type==='activated'&&event.id===target.id),`${renderer} nonlinear keyboard activated stale source`);
          reports.push({renderer,consumer,phase,currentData:true,unknown:true,tooltip:true,keyboard:true,retainedNode:true,nonuniformLegend:true,actualPng:true});
        }
        await page.mouse.move(2,2);await page.evaluate(()=>document.activeElement?.blur());await flush();
        await samePaint(phase);
        const old=await state('svg'),markup=await page.locator('#nonlinear-colors-svg svg').innerHTML();assert(await page.evaluate(()=>window.hostFixture.nonlinear_colors_rejected()),'Invalid nonlinear transform or Scene was admitted');
        let phaseRejected=false;try{await page.evaluate(()=>window.hostFixture.nonlinear_colors_phase(255));}catch{phaseRejected=true;}assert(phaseRejected,'Invalid nonlinear phase admitted');
        const next=await state('svg');assert(old.revision===next.revision&&old.pending===next.pending&&JSON.stringify(old.sites)===JSON.stringify(next.sites),'Rejected nonlinear update changed accepted frame or queue');await flush();assert(await page.locator('#nonlinear-colors-svg svg').innerHTML()===markup,'Rejected nonlinear update changed actual SVG');
      }
      await page.mouse.move(2,2);await page.evaluate(()=>document.activeElement?.blur());await flush();
      // Explicit compatibility profile, separate from the modern zero-mutation gate.
      await page.evaluate(()=>{
        const prototype=CanvasRenderingContext2D.prototype,descriptor=Object.getOwnPropertyDescriptor(prototype,'reset');
        window.__nonlinearResetRestore=()=>{if(descriptor)Object.defineProperty(prototype,'reset',descriptor);else delete prototype.reset;delete window.__nonlinearResetRestore;};
        Object.defineProperty(prototype,'reset',{configurable:true,writable:true,value:undefined});
        window.__nonlinearFallbackWrites=[];window.__nonlinearFallbackObserver=new MutationObserver(rows=>window.__nonlinearFallbackWrites.push(...rows.filter(row=>row.type==='attributes').map(row=>row.attributeName)));
        window.__nonlinearFallbackObserver.observe(document.querySelector('#nonlinear-colors-canvas canvas'),{attributes:true});
      });
      let fallbackImage,fallbackWrites;
      try {
        assert(await page.evaluate(()=>typeof CanvasRenderingContext2D.prototype.reset==='undefined'),'Canvas reset was not actually masked');
        await page.evaluate(()=>window.hostFixture.nonlinear_colors_phase(1));await flush();
        fallbackImage=(await capture('canvas',await state('canvas'),consumer,1,'reset-fallback')).receipt;
        fallbackWrites=await page.evaluate(()=>{window.__nonlinearFallbackObserver.disconnect();return window.__nonlinearFallbackWrites;});
        assert(fallbackWrites.includes('width')&&fallbackWrites.includes('height'),'Legacy Canvas fallback did not reset the actual backing');
      } finally {await page.evaluate(()=>{window.__nonlinearFallbackObserver?.disconnect();window.__nonlinearResetRestore?.();});}
      await page.evaluate(()=>window.hostFixture.nonlinear_colors_phase(9));await flush();await samePaint(9);
      resetFallback.push({renderer:'canvas',consumer,actualResetMasked:true,dimensionWrites:fallbackWrites,originalD3Pixels:true,restoredModernGate:true,png:fallbackImage,scope:'Compatibility redraw, excluded from modern zero-DOM evidence'});
      const oldImage=await capture('canvas',await state('canvas'),consumer,9,'gradient-before');
      await page.evaluate(()=>{
        const context=document.querySelector('#nonlinear-colors-canvas canvas').getContext('2d'),contexts=CanvasRenderingContext2D.prototype,gradients=CanvasGradient.prototype;
        const createDescriptor=Object.getOwnPropertyDescriptor(contexts,'createLinearGradient'),stopDescriptor=Object.getOwnPropertyDescriptor(gradients,'addColorStop'),resetDescriptor=Object.getOwnPropertyDescriptor(contexts,'reset');
        const watched=new WeakSet(),stats={faults:0,successfulStops:0,resetCalls:0};window.__nonlinearGradientStats=stats;
        Object.defineProperty(contexts,'createLinearGradient',{...createDescriptor,value:function(...args){const gradient=createDescriptor.value.apply(this,args);if(this===context)watched.add(gradient);return gradient;}});
        Object.defineProperty(gradients,'addColorStop',{...stopDescriptor,value:function(...args){if(watched.has(this)){if(!stats.faults&&stats.successfulStops>=1){stats.faults++;throw new DOMException('nonlinear controlled gradient fault','InvalidStateError');}stats.successfulStops++;}return stopDescriptor.value.apply(this,args);}});
        if(resetDescriptor&&typeof resetDescriptor.value==='function')Object.defineProperty(contexts,'reset',{...resetDescriptor,value:function(...args){if(this===context)stats.resetCalls++;return resetDescriptor.value.apply(this,args);}});
        window.__nonlinearGradientRestore=()=>{Object.defineProperty(contexts,'createLinearGradient',createDescriptor);Object.defineProperty(gradients,'addColorStop',stopDescriptor);if(resetDescriptor)Object.defineProperty(contexts,'reset',resetDescriptor);delete window.__nonlinearGradientRestore;};
        // An owned frame must clear foreign clip/save state before it paints.
        context.save();context.beginPath();context.rect(0,0,1,1);context.clip();context.setLineDash([17,23]);
      });
      let transaction;
      try {transaction=await page.evaluate(()=>{const f=window.hostFixture,before=JSON.parse(f.nonlinear_colors_snapshot('canvas'));f.nonlinear_colors_phase(9);f.nonlinear_colors_flush();return{before,after:JSON.parse(f.nonlinear_colors_snapshot('canvas')),stats:{...window.__nonlinearGradientStats}};});}
      finally {await page.evaluate(()=>window.__nonlinearGradientRestore?.());}
      const {before:beforeFault,after:afterFault,stats:faultStats}=transaction;
      assert(faultStats.faults===1&&faultStats.successfulStops>=1,'Actual caught addColorStop exception did not occur after a partial gradient write');
      assert(afterFault.revision===beforeFault.revision&&JSON.stringify(afterFault.sites)===JSON.stringify(beforeFault.sites),`Gradient publication failure changed the accepted model/revision: ${JSON.stringify({beforeRevision:beforeFault.revision,afterRevision:afterFault.revision,beforeSites:beforeFault.sites,afterSites:afterFault.sites,events:afterFault.events.slice(beforeFault.events.length),faultStats})}`);
      assert(afterFault.events.slice(beforeFault.events.length).some(event=>event.type==='error'&&event.message.includes('nonlinear controlled gradient fault')),'Actual gradient API exception was swallowed');
      const recoveredImage=await capture('canvas',afterFault,consumer,9,'gradient-recovery');
      const difference=await page.evaluate(async({a,b})=>{const load=async bytes=>{const image=await createImageBitmap(new Blob([new Uint8Array(bytes)],{type:'image/png'}));const canvas=document.createElement('canvas');canvas.width=image.width;canvas.height=image.height;const ctx=canvas.getContext('2d');ctx.drawImage(image,0,0);image.close();return ctx.getImageData(0,0,canvas.width,canvas.height).data;};const left=await load(a),right=await load(b);if(left.length!==right.length)return -1;let difference=0;for(let i=0;i<left.length;i++)difference+=Number(left[i]!==right[i]);return difference;},{a:[...oldImage.bytes],b:[...recoveredImage.bytes]});
      assert(difference===0,'Recovered Canvas PNG did not exactly restore the accepted full frame/clip');
      assert(!canvasResetSupported||faultStats.resetCalls>=2,'Modern attempted and recovery frames did not both reset Canvas state');
      await samePaint(9);
      gradientFaults.push({renderer:'canvas',consumer,actualCaughtApi:'CanvasGradient.addColorStop',faults:faultStats.faults,successfulStops:faultStats.successfulStops,resetCalls:faultStats.resetCalls,foreignClipInjected:true,errorObserved:true,revisionRetained:true,modelRetained:true,fullPngRestored:true,differingChannels:difference,beforePng:oldImage.receipt,recoveredPng:recoveredImage.receipt,scope:'One recoverable caught gradient exception; no guarantee for a persistently failing host API'});
      const native=await page.evaluate(()=>({pointer:window.__nonlinearPackets,keys:window.__nonlinearKeys}));assert(native.pointer.length>=40&&native.pointer.every(packet=>packet.trusted),'Nonlinear hover lacked actual trusted input');assert(native.keys.length>=20&&native.keys.every(event=>event.trusted),'Nonlinear keyboard lacked actual trusted key input');nativeEvents.push({consumer,pointerPackets:native.pointer.length,keyPackets:native.keys.length,pointersAllTrusted:native.pointer.every(packet=>packet.trusted),keysAllTrusted:native.keys.every(event=>event.trusted)});
      await page.evaluate(()=>{document.removeEventListener('pointermove',window.__nonlinearPointer,true);document.removeEventListener('keydown',window.__nonlinearKey,true);window.__nonlinearDetached=document.querySelector('#nonlinear-colors-svg .kartis-datum');window.hostFixture.nonlinear_colors_stop();});await flush();assert(await page.locator('body *').count()===baseline,'Nonlinear color Stop retained DOM');await page.evaluate(()=>window.__nonlinearDetached?.click());await flush();assert(!(await state('svg')).active&&!(await state('canvas')).active,'Detached nonlinear control revived stopped owner');
    }
    assert(errors.length===0,errors.join('\n'));
    let dpr1;
    if(!options.nested){const context=await page.context().browser().newContext({viewport:{width:1440,height:1000},deviceScaleFactor:1});try{const nested=await context.newPage();nested.setDefaultTimeout(45000);await nested.goto(page.url());await nested.waitForFunction(()=>window.hostReady===true);await nested.bringToFront();dpr1=await verifyNonlinearColors(nested,{nested:true});}finally{await context.close();await page.bringToFront();}}
    return {passed:true,contextDpr:await page.evaluate(()=>devicePixelRatio),reference:{package:'d3-scale',version:'4.0.2',sourceManifest:'tests/fixtures/nonlinear-color-d3-source.json',oracle:'Original D3 Node functions; PNG samples use its actual axis inverse and color interpolator'},profiles:reports,ink,negativeControls,nativeEvents,canvasResetSupported,resetFallback,gradientFaults,unchangedMutationZero:true,publicationCounterMutations,sameFrameTelemetry:'Only surface data-revision publication counter is excluded from paint mutations',rejectedAtomic:true,cleanup:true,staleListenerCleanup:true,pageErrors:errors,dpr1};
  } finally {
    await page.evaluate(old=>{window.__nonlinearResetRestore?.();window.__nonlinearGradientRestore?.();window.__nonlinearFallbackObserver?.disconnect();window.hostFixture?.nonlinear_colors_stop();window.__nonlinearObserver?.disconnect();document.removeEventListener('pointermove',window.__nonlinearPointer,true);document.removeEventListener('keydown',window.__nonlinearKey,true);document.querySelector('.panels').style.display=old;for(const key of ['__nonlinearPackets','__nonlinearKeys','__nonlinearPointer','__nonlinearKey','__nonlinearRetained','__nonlinearMutations','__nonlinearCounterMutations','__nonlinearObserver','__nonlinearDetached','__nonlinearFallbackWrites','__nonlinearFallbackObserver','__nonlinearGradientStats'])delete window[key];},previous).catch(()=>{});page.off('pageerror',onError);
  }
})
