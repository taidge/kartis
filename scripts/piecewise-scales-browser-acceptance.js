(async function verifyPiecewiseScales(page, options = {}) {
  const {readFile, writeFile, mkdir} = await import('node:fs/promises');
  const {execFileSync} = await import('node:child_process');
  const {join} = await import('node:path');
  const {pathToFileURL} = await import('node:url');
  const {createHash, randomUUID} = await import('node:crypto');
  const assert = (ok, message) => { if (!ok) throw Error(message); };
  const hash = bytes => createHash('sha256').update(bytes).digest('hex');
  const metadata = JSON.parse(execFileSync('cargo', ['metadata','--offline','--no-deps','--format-version','1'], {encoding:'utf8',windowsHide:true}));
  const {capturePaintSample,comparePNGTriples,screenGate,samplePNGWitness,alignHostPixelGrid,verifyHostPixelGrid}=await import(pathToFileURL(join(metadata.workspace_root,'scripts','piecewise-paint-capture.mjs')).href);
  const acceptanceScope={profile:'actualPaintOutputWithBoundedCompositor',paintGate:{rawMaxChannelDelta:1,medianMaxChannelDelta:1,medianChangedPixels:4},screenGate:{rawMaxChannelDelta:1,changedPixels:'diagnostic; not a gate'},screenByteIdentity:false,pixelGrid:{profile:'integer-document-origin',precision:1e-3,pngDimensions:'accepted logical dimensions times DPR exactly',alignment:'outer host only at completed layout epochs; active gestures, rollback pairs and negative capture only verify; each epoch preserves exact child DOM/model/local geometry',scope:'Strict rollback/control physical PNG samples on the configured integer document grid; natural fractional-origin chart input, actual pixels and SVG/Canvas parity are separately required without raw byte-identity claims'},settling:{requestedDelayMs:1000,rafCount:2,per:'fixed three-sample group',noRepaintOrRetry:true,scope:'Steady output after the fixed delay; does not promise atomic display of every physical frame during the first 1000ms after input'},gestureScreenWitnessesUnchanged:true,svgCanvasMeanThreshold:0.5,paintSource:'live Canvas backing PNG; exact live SVG DOM with inherited language and explicit intrinsic dimensions rasterized by same-browser Image to DPR readback canvas',limits:'Numeric fixture defaults, internal SVG resources, explicit label font attributes; no arbitrary external webfont/resource guarantee'};
  const modules = join(metadata.target_directory,'piecewise-viewport-reference','node_modules');
  const header = (await readFile(join(metadata.workspace_root,'tests/fixtures/piecewise-viewport-d3.csv'),'utf8')).split('\n')[0];
  const sourceHashes = {};
  for (const [pkg, file] of [['d3-scale','src/continuous.js'],['d3-scale','src/time.js'],['d3-zoom','src/transform.js']]) {
    const actual = hash(await readFile(join(modules,pkg,file)));
    assert(header.includes(`${file}:${actual}`),`Original ${pkg}/${file} differs from frozen oracle`);
    sourceHashes[`${pkg}/${file}`] = actual;
  }
  for (const [pkg,version] of [['d3-scale','4.0.2'],['d3-zoom','3.0.0']]) assert(JSON.parse(await readFile(join(modules,pkg,'package.json'),'utf8')).version===version,`Wrong original ${pkg} version`);
  const d3 = await import(pathToFileURL(join(modules,'d3-scale','src/index.js')).href);
  const {Transform} = await import(pathToFileURL(join(modules,'d3-zoom','src/transform.js')).href);
  const make = s => {
    const scale = s.kind==='linear'?d3.scaleLinear():s.kind==='log'?d3.scaleLog().base(s.parameter):s.kind==='symlog'?d3.scaleSymlog().constant(s.parameter):s.kind==='pow'?d3.scalePow().exponent(s.parameter):s.kind==='sqrt'?d3.scaleSqrt():d3.scaleUtc();
    scale.domain(s.domain).clamp(s.clamp);return s.round?scale.rangeRound(s.range):scale.range(s.range);
  };
  const close = (a,b,kind='linear') => Number.isFinite(a)&&Number.isFinite(b)&&Math.abs(a-b)<=(kind==='utc'?1:2e-8*Math.max(1,Math.abs(b)));
  const array = (a,b,kind,context) => assert(a.length===b.length&&a.every((v,i)=>close(v,Number(b[i]),kind)),`${context}: ${JSON.stringify(a)} vs D3 ${JSON.stringify(b)}`);
  const authored = (c,phase) => {
    const domains=[[-10,-1,4,20],[1,2,15,128],[-30,-2,3,80],phase?[-32,-8,-2,-1]:[-4,-1,1,9],[-9,-1,4,25],[1708992000000,1709078400000,1709251200000,1709510400000],[-10,-1,4,20]];
    const x=domains[c].slice(),y=[0,2,7,12];if(phase&&c!==2)x.reverse();if(phase)y.reverse();
    return{x,y,reverseX:!!phase&&[1,2,3,5].includes(c),reverseY:!!phase,xFractions:c===6?[0,1,.2,1]:[0,.18,.73,1],yFractions:[0,.53,.81,1]};
  };
  const dpr = await page.evaluate(()=>devicePixelRatio), suffix=randomUUID(), reports=[], ink=[], frames=[], rollbacks=[], screenshotControls=[], navigation=[], errors=[];
  const onError=e=>errors.push(e.message);page.on('pageerror',onError);
  const output=join(metadata.target_directory,'gallery');await mkdir(output,{recursive:true});
  let current={section:'setup'};
  const previous=await page.evaluate(()=>{const p=document.querySelector('.panels'),old=p.style.display;p.style.display='none';return old;});
  const baseline=await page.locator('body *').count(),hostOriginAlignments=[],fractionalProfiles=[];let pixelGridEpoch=0;
  const setupPixelGrid=async label=>{const epoch=++pixelGridEpoch;for(const r of['svg','canvas'])hostOriginAlignments.push({...await alignHostPixelGrid(page,r,join(output,`piecewise-scales-${label}-epoch${epoch}-${r}-dpr${dpr}-${suffix}`),label),epoch});};
  const checkPixelGrid=async()=>{for(const r of['svg','canvas'])await verifyHostPixelGrid(page,r);};
  const state=r=>page.evaluate(r=>JSON.parse(window.hostFixture.piecewise_scales_snapshot(r)),r);
  const surface=r=>page.locator(`#piecewise-scales-${r} .kartis-surface`);
  const flush=async()=>{await page.evaluate(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))));await page.evaluate(()=>window.hostFixture.piecewise_scales_flush());await page.evaluate(()=>new Promise(requestAnimationFrame));await page.evaluate(()=>window.hostFixture.piecewise_scales_flush());};
  const arm=async(r,s)=>page.evaluate(({r,s})=>{
    window.__piecewisePackets??=[];window.__piecewiseListeners??=[];
    for(const old of window.__piecewiseListeners.filter(x=>x.r===r))for(const name of ['wheel','pointerdown','pointerup','keydown'])old.el.removeEventListener(name,old.listener,true);
    window.__piecewiseListeners=window.__piecewiseListeners.filter(x=>x.r!==r);
    const el=document.querySelector(`#piecewise-scales-${r}`),paint=el.querySelector('.kartis-surface');
    const listener=e=>{const b=paint.getBoundingClientRect();window.__piecewisePackets.push({r,type:e.type,trusted:e.isTrusted,key:e.key,delta:e.deltaY,mode:e.deltaMode,point:[(e.clientX-b.x)/b.width*s.width,(e.clientY-b.y)/b.height*s.height]});};
    for(const name of ['wheel','pointerdown','pointerup','keydown'])el.addEventListener(name,listener,true);
    window.__piecewiseListeners.push({r,el,listener});
    return window.__piecewisePackets.length;
  },{r,s});
  const packets=async(start,r)=>page.evaluate(({start,r})=>(window.__piecewisePackets??[]).slice(start).filter(p=>p.r===r),{start,r});
  const location=async(r,s,point)=>{await surface(r).scrollIntoViewIfNeeded();await flush();const b=await surface(r).boundingBox();return[b.x+point[0]/s.width*b.width,b.y+point[1]/s.height*b.height];};
  const mapped=(s,label)=>{
    const profile=authored(s.case,s.phase);
    assert(s.baseX.kind===['linear','log','symlog','pow','sqrt','utc','linear'][s.case]&&s.baseX.parameter===(s.case===1?10:s.case===2?2:s.case===3?(s.phase?-1:2):s.case===4?.5:1)&&s.baseY.kind==='linear',`${label}: wrong authored numeric family`);
    array(s.baseX.domain,profile.x,'linear',`${label}/authored X knots`);array(s.baseY.domain,profile.y,'linear',`${label}/authored Y knots`);
    for(const [key,fractions,reverse,limits]of[['baseX',profile.xFractions,profile.reverseX,[s.plot[0],s.plot[0]+s.plot[2]]],['baseY',profile.yFractions,profile.reverseY,[s.plot[1]+s.plot[3],s.plot[1]]]]){
      const range=fractions.map(t=>limits[0]*(1-t)+limits[1]*t);if(reverse)range.reverse();array(s[key].range,range,'linear',`${label}/${key} authored range`);
      assert(s[key].round===!!s.phase&&s[key].clamp===!!s.phase,`${label}: round/clamp options disappeared`);
    }
    const sourceCount=21*(2-s.hidden.length);
    assert(s.sites.length>0&&s.sites.length<=sourceCount,`${label}: invalid visible source subset`);
    if(!s.transform||(s.transform.kx===1&&s.transform.ky===1))assert(s.sites.length===sourceCount,`${label}: baseline source sites missing`);
    const rawDomain=profile.x.slice().sort((a,b)=>a-b);
    for(const site of s.sites){const segment=Math.floor(site.source/7),fraction=(site.source%7+.37)/7;let x=rawDomain[segment]*(1-fraction)+rawDomain[segment+1]*fraction;if(s.case===5)x=Math.round(x);array(site.raw,[x,1+(site.source*3+(site.series==='alpha'?0:5))%11],'linear',`${label}/immutable source ${site.id}`);}
    for(const [key,baseKey,axis,origin] of [['xScale','baseX','x',s.plot[0]],['yScale','baseY','y',s.plot[1]]]) {
      const scale=make(s[key]);assert(s[key].domain.length===4&&s[key].range.length===4,`${label}: discarded interior knots`);
      if(s.transform){
        const k=s.transform[axis==='x'?'kx':'ky'],t=s.transform[axis==='x'?'tx':'ty']+origin*(1-k);
        const transform=new Transform(k,axis==='x'?t:0,axis==='y'?t:0);
        const expected=(axis==='x'?transform.rescaleX(make(s[baseKey])):transform.rescaleY(make(s[baseKey]))).domain().map(Number);
        array(s[key].domain,expected,s[key].kind,`${label}/${axis} full original D3 zoom domain`);
        array(s.stops[axis],s[key].domain,s[key].kind,`${label}/${axis} controller/layout knots`);
      }
      for(const site of s.sites){const actual=site.position[axis==='x'?0:1],expected=scale(site.raw[axis==='x'?0:1]);assert(close(actual,expected),`${label}/${axis} typed site ${site.id}: ${actual} vs original D3 ${expected}`);}
      array(s[key].ticks,scale.ticks(5).map(Number),s[key].kind,`${label}/${axis} value ticks`);
    }
    assert(s.labels.length>=5,`${label}: missing actual axis labels`);
    const xs=make(s.xScale),bottom=s.plot[1]+s.plot[3];
    const tickPixels=xs.ticks(6).map(t=>xs(t));
    assert(tickPixels.some(x=>s.labels.some(l=>l.y>bottom&&close(l.x,x))),`${label}: axes did not use current piecewise pixel mapping`);
  };
  const pick=async r=>{
    const s=await state(r);assert(s.target,`${r}: no current visible typed point`);
    const point=await location(r,s,s.target.point);await page.mouse.move(point[0]+2,point[1]+2);await page.mouse.move(...point);await flush();
    const hovered=await state(r);assert(hovered.hovered.includes(s.target.id),`${r}: hover picked stale/two-stop geometry`);
    assert((await page.locator(`#piecewise-scales-${r} .kartis-surface-tooltip`).textContent()).includes(s.target.formattedY),`${r}: tooltip lost current raw value`);
    await page.mouse.click(...point);await flush();assert((await state(r)).selected===s.target.id,`${r}: click lost typed identity`);
    const datum=page.locator(`#piecewise-scales-${r} .kartis-datum`).filter({hasText:s.target.label}).first();
    await datum.focus();const start=await arm(r,s);await page.keyboard.press('Enter');await flush();
    // Data controls may live outside the paint surface; their actual Activate
    // event is also observed by the application-owned typed event receipt.
    const keyboard=await packets(start,r);
    assert(keyboard.some(e=>e.type==='keydown'&&e.key==='Enter'&&e.trusted)&&(await state(r)).events.some(e=>e.event?.includes('Activated')&&e.event.includes(s.target.id)),`${r}: trusted Enter did not activate current source`);
    return{source:s.target.source,raw:s.target.raw,id:s.target.id,packets:keyboard};
  };
  const clean=async()=>{await page.mouse.move(2,2);for(const r of['svg','canvas'])await surface(r).press('Escape');await page.evaluate(()=>document.activeElement?.blur());await flush();};
  const capture=async(label,{pixelGrid=true,receipts=ink}={})=>{
    await clean();if(pixelGrid)await checkPixelGrid();
    const pixels=await page.evaluate(async()=>{
      const live=document.querySelector('#piecewise-scales-svg svg'),svg=live.cloneNode(true),canvas=document.querySelector('#piecewise-scales-canvas canvas');
      svg.setAttribute('width',String(live.viewBox.baseVal.width));svg.setAttribute('height',String(live.viewBox.baseVal.height));
      let lang;for(let n=live;n;n=n.parentElement){const xml=n.getAttributeNS('http://www.w3.org/XML/1998/namespace','lang')??n.getAttribute('xml:lang');if(xml!==null){lang=xml;break;}if(n.hasAttribute('lang')){lang=n.getAttribute('lang');break;}}
      lang??=document.documentElement.lang;svg.setAttribute('lang',lang);svg.setAttributeNS('http://www.w3.org/XML/1998/namespace','xml:lang',lang);
      const image=new Image(),copy=document.createElement('canvas'),url=URL.createObjectURL(new Blob([new XMLSerializer().serializeToString(svg)],{type:'image/svg+xml;charset=utf-8'}));
      try{
        image.src=url;await image.decode();copy.width=canvas.width;copy.height=canvas.height;copy.getContext('2d').drawImage(image,0,0,copy.width,copy.height);
        const a=copy.getContext('2d').getImageData(0,0,copy.width,copy.height).data,b=canvas.getContext('2d').getImageData(0,0,canvas.width,canvas.height).data;
        let diff=0,ink=0;for(let i=0;i<a.length;i++)diff+=Math.abs(a[i]-b[i]);for(let i=0;i<b.length;i+=4)ink+=Number(Math.min(b[i],b[i+1],b[i+2])<220);
        return{mean:diff/a.length,ink,backing:[canvas.width,canvas.height],referenceIntrinsic:[image.naturalWidth,image.naturalHeight]};
      }finally{
        image.onload=image.onerror=null;image.removeAttribute('src');URL.revokeObjectURL(url);copy.width=copy.height=0;
      }
    });
    assert(pixels.mean<0.5&&pixels.ink>2000,`${label}: SVG/Canvas parity ${JSON.stringify(pixels)}`);
    const s=await state('canvas');assert(pixels.backing[0]===Math.round(s.width*dpr)&&pixels.backing[1]===Math.round(s.height*dpr),`${label}: actual Canvas backing DPR mismatch`);
    for(const r of['svg','canvas']){
      const path=join(output,`piecewise-scales-${label}-${r}-dpr${dpr}-${suffix}.png`);
      const grid=pixelGrid?await verifyHostPixelGrid(page,r):null;await surface(r).screenshot({path});const bytes=await readFile(path);assert(bytes.length>1000,`${label}: empty actual PNG`);const sampled=samplePNGWitness(bytes,{fraction:[0,0],box:[0,0,...(grid?.logicalSize??[s.width,s.height])]},[0,0,0]);if(grid)assert(JSON.stringify(sampled.backing)===JSON.stringify(grid.expectedPNGDimensions),`${label}: screen PNG differs from exact pixel-grid dimensions`);
      receipts.push({label,renderer:r,path,sha256:hash(bytes),bytes:bytes.length,pixels});
    }
    return pixels;
  };
  const frameAtTarget=async(r,label,{pixelGrid=true,receipts=frames}={})=>{
    await page.mouse.move(2,2);await page.evaluate(()=>document.activeElement?.blur());await surface(r).scrollIntoViewIfNeeded();await flush();
    const s=await state(r);assert(s.target,`${label}: no visible paint witness`);
    const path=join(output,`piecewise-scales-frame-${label}-${r}-dpr${dpr}-${suffix}.png`);
    const measure=()=>page.evaluate(({point,width,height,r})=>{
      const surface=document.querySelector(`#piecewise-scales-${r} .kartis-surface`),box=surface.getBoundingClientRect(),paint=surface.querySelector(r==='svg'?'svg':'canvas'),b=paint.getBoundingClientRect(),m=r==='svg'?paint.getScreenCTM():paint.getContext('2d').getTransform();
      const client=r==='svg'?new DOMPoint(...point).matrixTransform(m):{x:b.x+point[0]/width*b.width,y:b.y+point[1]/height*b.height};
      return{renderer:r,logicalSize:[width,height],box:[box.x,box.y,box.width,box.height],paintRect:[b.x,b.y,b.width,b.height],paintMatrix:[m.a,m.b,m.c,m.d,m.e,m.f],fraction:[(client.x-box.x)/box.width,(client.y-box.y)/box.height],scroll:[scrollX,scrollY],dpr:devicePixelRatio};
    },{point:s.target.position,width:s.width,height:s.height,r});
    const grid=pixelGrid?await verifyHostPixelGrid(page,r):null,before=await measure(),geometryPath=path+'.geometry.json';
    await writeFile(geometryPath,JSON.stringify({before},null,2)+'\n');
    const bytes=await surface(r).screenshot({path}),after=await measure(),geometry={before,after};
    const geometryBytes=JSON.stringify(geometry,null,2)+'\n';await writeFile(geometryPath,geometryBytes);
    assert(JSON.stringify(before)===JSON.stringify(after),`${label}: actual frame geometry changed across single screenshot`);
    assert(bytes.length>0,`${label}: actual screenshot returned zero bytes; no retry`);
    const expected=s.target.series==='alpha'?[78,121,167]:[242,142,43],actual=samplePNGWitness(bytes,before,expected);
    // Mount-time host alignment establishes exact physical screenshot dimensions.
    assert(actual.solid>0&&(grid?actual.backing.every((v,i)=>v===grid.expectedPNGDimensions[i]):actual.backing.every((v,i)=>Math.abs(v-Math.round(actual.css[i]*dpr))<=2)),`${label}: actual current PNG retained stale point ink ${JSON.stringify(actual)}`);
    receipts.push({label,renderer:r,path,sha256:hash(bytes),bytes:bytes.length,geometryPath,geometrySha256:hash(geometryBytes),witness:{id:s.target.id,raw:s.target.raw,position:s.target.position},actual});
  };
  const canonicalState=s=>{const{revision,calls,events,...accepted}=s;return JSON.stringify(accepted);};
  const sampleBundle=async(r,label)=>{
    await verifyHostPixelGrid(page,r);
    // A predeclared steady-output observation, never a retry or a repaint.
    const cadence=await page.evaluate(async({renderer,label})=>{
      const startedAt=performance.now();await new Promise(resolve=>setTimeout(resolve,1000));const timerResolvedAt=performance.now(),rafTimestamps=[];
      await new Promise(resolve=>requestAnimationFrame(t=>{rafTimestamps.push(t);requestAnimationFrame(t=>{rafTimestamps.push(t);resolve();});}));
      const finishedAt=performance.now();return{schema:1,renderer,label,requestedDelayMs:1000,rafCount:2,startedAt,timerResolvedAt,rafTimestamps,finishedAt,actualDelayMs:finishedAt-startedAt};
    },{renderer:r,label});
    const cadenceFile=join(output,`piecewise-scales-${label}-${r}-dpr${dpr}-${suffix}-cadence.json`),cadenceBytes=JSON.stringify(cadence,null,2)+'\n';await writeFile(cadenceFile,cadenceBytes);
    assert(cadence.actualDelayMs>=1000&&cadence.rafTimestamps.length===2,`${label}: fixed steady-output cadence was not observed`);
    const bytes=[],screenBytes=[],samples=[];
    for(let i=0;i<3;i++){
      if(i)await page.evaluate(()=>new Promise(requestAnimationFrame));
      const captureRequestedAt=await page.evaluate(()=>performance.now()),prefix=join(output,`piecewise-scales-${label}-${r}-dpr${dpr}-${suffix}-sample${i}`),actual=await capturePaintSample(page,r,prefix);
      Object.assign(actual.record,{cadenceFile,cadenceSha256:hash(cadenceBytes),captureRequestedAt});
      bytes.push(actual.paintBytes);screenBytes.push(actual.screenBytes);samples.push(actual.record);
    }
    assert(samples.every(s=>s.canonicalStateSha256===samples[0].canonicalStateSha256&&s.canonicalDOMSha256===samples[0].canonicalDOMSha256&&JSON.stringify(s.rect)===JSON.stringify(samples[0].rect)),`${label}: state/DOM/rect changed during fixed three-frame capture`);
    return{bytes,screenBytes,samples};
  };
  const pngDifference=async(a,b)=>comparePNGTriples(a,b);
  const stablePNG=d=>d.sameDimensions&&d.rawMaxChannelDelta<=1&&d.maxChannelDelta<=1&&d.changedPixels<=4;
  try{
    current={section:'settledScreenshotQuantization'};
    await page.evaluate(()=>window.hostFixture.piecewise_scales_mount(4,false,0));await flush();await setupPixelGrid('control-mount');
    for(const r of['svg','canvas']){
      await page.mouse.move(2,2);await surface(r).scrollIntoViewIfNeeded();await flush();
      const reference=await sampleBundle(r,'no-input-reference'),comparisons=[];
      for(let group=0;group<3;group++){
        // Empty flush is a paint request, not a business input or a rejection.
        await flush();const control=await sampleBundle(r,`no-input-control${group}`),delta=await pngDifference(reference.bytes,control.bytes);
        assert(control.samples[0].canonicalStateSha256===reference.samples[0].canonicalStateSha256&&control.samples[0].canonicalDOMSha256===reference.samples[0].canonicalDOMSha256&&JSON.stringify(control.samples[0].rect)===JSON.stringify(reference.samples[0].rect)&&stablePNG(delta),`No-input screenshot control exceeded fixed profile ${JSON.stringify(delta)}`);
        const screenDelta=await pngDifference(reference.screenBytes,control.screenBytes);assert(screenGate(screenDelta),'No-input compositor screen exceeded raw one-LSB bound');
        comparisons.push({samples:control.samples,delta,screenDelta});
      }
      const witness=await state(r);
      const change=await page.evaluate(({r,id})=>{
        if(r==='svg'){
          const node=[...document.querySelectorAll('#piecewise-scales-svg [data-id]')].find(n=>n.dataset.id===id);if(!node)throw Error('Missing datum for geometry negative control');
          const old=node.getAttribute('transform');node.setAttribute('transform',`translate(1 0) ${old??''}`);window.__piecewiseNegative={node,old};return{kind:'SVG datum geometry translated +1 Scene pixel',id};
        }
        const canvas=document.querySelector('#piecewise-scales-canvas canvas'),context=canvas.getContext('2d'),data=context.getImageData(0,0,canvas.width,canvas.height),copy=document.createElement('canvas');copy.width=canvas.width;copy.height=canvas.height;copy.getContext('2d').putImageData(data,0,0);
        context.save();context.setTransform(1,0,0,1,0,0);context.clearRect(0,0,canvas.width,canvas.height);context.drawImage(copy,devicePixelRatio,0);context.restore();window.__piecewiseNegative={context,data};return{kind:'Canvas raster translated +1 CSS pixel',pixels:devicePixelRatio};
      },{r,id:witness.target.id});
      const negative=await sampleBundle(r,'one-pixel-negative'),negativeDelta=await pngDifference(reference.bytes,negative.bytes);
      assert(negativeDelta.rawMaxChannelDelta>1&&!stablePNG(negativeDelta),'Fixed median/raw pixel gates failed to reject real one-pixel geometry change');
      const negativeScreenDelta=await pngDifference(reference.screenBytes,negative.screenBytes);assert(negativeScreenDelta.rawMaxChannelDelta>1&&!screenGate(negativeScreenDelta),'Compositor screen raw gate failed to reject actual one-pixel geometry change');
      await page.evaluate(()=>{const n=window.__piecewiseNegative;if(n.node){if(n.old===null)n.node.removeAttribute('transform');else n.node.setAttribute('transform',n.old);}else n.context.putImageData(n.data,0,0);delete window.__piecewiseNegative;});
      await flush();
      const restored=await sampleBundle(r,'one-pixel-restored'),restoredDelta=await pngDifference(reference.bytes,restored.bytes),restoredScreenDelta=await pngDifference(reference.screenBytes,restored.screenBytes);
      assert(stablePNG(restoredDelta)&&screenGate(restoredScreenDelta),'Restored actual paint/screen did not return to bounded original output');
      screenshotControls.push({renderer:r,profile:'actualPaintOutputWithBoundedCompositor',sampling:'fixed 1000ms+2RAF per group, then 3 actual samples per side; no retry; paint raw1/median1/4; screen raw1 only',reference:reference.samples,comparisons,negative:{change,samples:negative.samples,delta:negativeDelta,screenDelta:negativeScreenDelta,rejected:true,screenRejected:true},restored:{samples:restored.samples,delta:restoredDelta,screenDelta:restoredScreenDelta,passed:true}});
    }
    await page.evaluate(()=>window.hostFixture.piecewise_scales_stop());await flush();assert(await page.locator('body *').count()===baseline,'Screenshot control retained DOM');
    for(const scatter of[false,true])for(const c of[0,1,2,3,4,5]){
      current={section:'family',case:c,scatter};await page.evaluate(({c,scatter})=>window.hostFixture.piecewise_scales_mount(c,scatter,0),{c,scatter});await flush();
      const fractional={case:c,scatter,phase:0,ink:[],frames:[],typed:[],scope:'Natural document origin before grid setup; actual input and pixels, no raw rollback identity claim'};
      for(const r of['svg','canvas'])mapped(await state(r),`natural/${c}/${scatter}/${r}`);
      fractional.paintParity=await capture(`natural-case${c}-${scatter?'scatter':'line'}`,{pixelGrid:false,receipts:fractional.ink});
      for(const r of['svg','canvas']){fractional.typed.push({renderer:r,...await pick(r)});await frameAtTarget(r,`natural-case${c}-${scatter?'scatter':'line'}`,{pixelGrid:false,receipts:fractional.frames});}
      fractionalProfiles.push(fractional);await clean();await setupPixelGrid(`family-case${c}-${scatter?'scatter':'line'}-mount`);
      let retained;
      for(const phase of[0,1]){
        current={...current,phase};
        if(phase){
          // Viewport clipping legitimately retires out-of-view point nodes.
          // Measure retention at this actual style transition, after reset.
          await page.evaluate(id=>{window.__piecewiseRetained=[...document.querySelectorAll('#piecewise-scales-svg [data-id]')].find(n=>n.dataset.id===id);window.hostFixture.piecewise_scales_phase(1);},retained);await flush();await setupPixelGrid(`family-case${c}-${scatter?'scatter':'line'}-phase1`);
        }
        for(const r of['svg','canvas'])mapped(await state(r),`${c}/${scatter}/${phase}/${r}/baseline`);
        const stable=(await state('svg')).sites[5];
        if(!phase){retained=stable.id;await page.evaluate(id=>{window.__piecewiseRetained=[...document.querySelectorAll('#piecewise-scales-svg [data-id]')].find(n=>n.dataset.id===id);window.__piecewiseSurface=document.querySelector('#piecewise-scales-svg .kartis-surface');},retained);}
        else assert(stable.id===retained&&await page.evaluate(id=>window.__piecewiseRetained===[...document.querySelectorAll('#piecewise-scales-svg [data-id]')].find(n=>n.dataset.id===id)&&window.__piecewiseSurface===document.querySelector('#piecewise-scales-svg .kartis-surface'),retained),'Next profile replaced retained source/node/chart');
        const pixels=await capture(`case${c}-${scatter?'scatter':'line'}-phase${phase}`);
        for(const r of['svg','canvas']){
          current={...current,renderer:r};const typed=await pick(r);await surface(r).press('Escape');await flush();
          const legend=page.locator(`#piecewise-scales-${r} .kartis-legend-item`).filter({hasText:'beta'}).first();
          const beforeHidden=await state(r);await legend.click();await flush();const hidden=await state(r);
          assert(hidden.hidden.includes('beta')&&hidden.sites.every(site=>site.series==='alpha'),'Legend retained hidden typed source sites');
          array(hidden.xScale.domain,beforeHidden.xScale.domain,hidden.xScale.kind,'Hidden relayout X knots');mapped(hidden,'hidden relayout');
          await legend.click();await flush();await setupPixelGrid(`family-case${c}-${scatter?'scatter':'line'}-phase${phase}-${r}-legend-restore`);
          const before=await state(r),[left,top,width,height]=before.plot,anchor=[left+width*0.37,top+height*0.43],client=await location(r,before,anchor),start=await arm(r,before);
          await page.mouse.move(...client);await page.mouse.wheel(0,-120);await flush();
          const zoomed=await state(r),wheel=(await packets(start,r)).find(p=>p.type==='wheel');
          assert(wheel?.trusted&&wheel.mode===0&&zoomed.transform.kx>1,`${r}: no trusted wheel/domain relayout`);
          const factor=Math.exp(-wheel.delta*0.002);
          for(const [k,t,coordinate,origin]of[['kx','tx',0,left],['ky','ty',1,top]]){
            const expectedK=before.transform[k]*factor,expectedT=(wheel.point[coordinate]-origin)*(1-factor)+before.transform[t]*factor;
            assert(close(zoomed.transform[k],expectedK)&&close(zoomed.transform[t],expectedT),`${r}: actual eccentric wheel transform drift`);
          }
          mapped(zoomed,`${c}/${scatter}/${phase}/${r}/wheel`);await setupPixelGrid(`family-case${c}-${scatter?'scatter':'line'}-phase${phase}-${r}-wheel-complete`);await pick(r);await frameAtTarget(r,`case${c}-${scatter?'scatter':'line'}-phase${phase}-wheel`);
          if(r==='svg')assert(await page.evaluate(()=>{const n=document.querySelector('#piecewise-scales-svg [data-id]'),m=n?.getCTM();return m&&Math.abs(m.a-1)<1e-9&&Math.abs(m.d-1)<1e-9&&Math.abs(m.b)+Math.abs(m.c)+Math.abs(m.e)+Math.abs(m.f)<1e-8;}),'Relayout painted the Scene camera twice');
          await surface(r).press('Escape');await flush();
          const panBefore=await state(r),pan=await location(r,panBefore,[left+width*.55,top+height*.55]),panStart=await arm(r,panBefore);
          await page.mouse.move(...pan);await page.mouse.down();await flush();await page.mouse.move(pan[0]+15,pan[1]+10,{steps:3});await flush();await page.mouse.up();await flush();
          const moved=await state(r),p=(await packets(panStart,r)).filter(p=>['pointerdown','pointerup'].includes(p.type));
          assert(p.length===2&&p.every(p=>p.trusted),`${r}: pan lacked trusted terminal events`);
          const expectedX=Math.max((1-panBefore.transform.kx)*width,Math.min(0,panBefore.transform.tx+p[1].point[0]-p[0].point[0]));
          const expectedY=Math.max((1-panBefore.transform.ky)*height,Math.min(0,panBefore.transform.ty+p[1].point[1]-p[0].point[1]));
          assert(close(moved.transform.tx,expectedX)&&close(moved.transform.ty,expectedY),`${r}: bounded pan lost actual logical pointer delta ${JSON.stringify({before:panBefore.transform,after:moved.transform,packets:p,expected:[expectedX,expectedY]})}`);
          mapped(moved,`${c}/${scatter}/${phase}/${r}/pan`);await setupPixelGrid(`family-case${c}-${scatter?'scatter':'line'}-phase${phase}-${r}-pan-complete`);await frameAtTarget(r,`case${c}-${scatter?'scatter':'line'}-phase${phase}-pan`);
          await page.evaluate(()=>window.hostFixture.piecewise_scales_reset());await flush();await setupPixelGrid(`family-case${c}-${scatter?'scatter':'line'}-phase${phase}-${r}-reset-complete`);
          const brushBefore=await state(r),a=await location(r,brushBefore,[left+width*.2,top+height*.2]),b=await location(r,brushBefore,[left+width*.8,top+height*.8]),brushStart=await arm(r,brushBefore);
          await page.keyboard.down('Shift');try{await page.mouse.move(...a);await page.mouse.down();await page.mouse.move(...b,{steps:4});await page.mouse.up();}finally{await page.keyboard.up('Shift');}await flush();
          const brushed=await state(r),bounds=brushed.events.findLast(e=>e.brush)?.brush,terminal=(await packets(brushStart,r)).filter(p=>['pointerdown','pointerup'].includes(p.type));
          assert(bounds&&terminal.length===2&&terminal.every(p=>p.trusted)&&!brushed.events.some(e=>e.brushError),`${r}: completed native Shift brush lost domain request`);
          const bx=(bounds[0]-left)/width,by=(bounds[1]-top)/height,sx=(bounds[2]-bounds[0])/width,sy=(bounds[3]-bounds[1])/height;
          assert(close(brushed.transform.kx,1/sx)&&close(brushed.transform.ky,1/sy)&&close(brushed.transform.tx,-bx*width/sx)&&close(brushed.transform.ty,-by*height/sy),`${r}: brush was rebased/inverted twice`);
          mapped(brushed,`${c}/${scatter}/${phase}/${r}/brush`);await setupPixelGrid(`family-case${c}-${scatter?'scatter':'line'}-phase${phase}-${r}-brush-complete`);await pick(r);await frameAtTarget(r,`case${c}-${scatter?'scatter':'line'}-phase${phase}-brush`);
          await page.evaluate(()=>window.hostFixture.piecewise_scales_reset());await flush();await setupPixelGrid(`family-case${c}-${scatter?'scatter':'line'}-phase${phase}-${r}-reset-complete`);
          reports.push({case:c,scatter,phase,renderer:r,typed,pixels,allKnotsOriginalD3:true,trustedWheel:true,trustedPan:true,trustedBrush:true,keyboard:true,identityPaint:true});
        }
        await clean();
        const before=await state('svg'),beforeCanvas=await state('canvas'),markup=await page.locator('#piecewise-scales-svg svg').first().innerHTML();
        assert(await page.evaluate(()=>window.hostFixture.piecewise_scales_invalid()),'Invalid breakpoints/Scene/typed binding were admitted');
        const invalidAfter=await state('svg'),invalidCanvasAfter=await state('canvas');
        assert(invalidAfter.revision===before.revision&&invalidCanvasAfter.revision===beforeCanvas.revision&&invalidAfter.pending===before.pending&&invalidCanvasAfter.pending===beforeCanvas.pending,'Rejected direct update changed accepted revision/queue');
        // Explicit flush paints even an empty queue; admission must be checked
        // before asking for another paint, and geometry remains equal afterward.
        await flush();assert(await page.locator('#piecewise-scales-svg svg').first().innerHTML()===markup,'Rejected direct update changed accepted geometry');
        await page.evaluate(()=>window.hostFixture.piecewise_scales_reject(true));
        for(const r of['svg','canvas']){
          const rejectBefore=await state(r),rawAt=await location(r,rejectBefore,rejectBefore.target.point),at=rawAt.map(Math.round);
          await page.mouse.move(...at);await flush();
          const rejectedMarkup=r==='svg'?await page.locator('#piecewise-scales-svg svg').first().innerHTML():null;
          const label=`rollback-case${c}-${scatter?'scatter':'line'}-phase${phase}`;
          const beforePNG=await sampleBundle(r,label+'-before'),rejectedBase=await state(r);await page.mouse.wheel(0,-80);await flush();const rejected=await state(r);
          assert(JSON.stringify(rejected.xScale)===JSON.stringify(rejectedBase.xScale)&&JSON.stringify(rejected.yScale)===JSON.stringify(rejectedBase.yScale)&&JSON.stringify(rejected.sites)===JSON.stringify(rejectedBase.sites)&&JSON.stringify(rejected.labels)===JSON.stringify(rejectedBase.labels)&&JSON.stringify(rejected.transform)===JSON.stringify(rejectedBase.transform)&&rejected.calls>rejectedBase.calls&&rejected.events.some(e=>e.event?.includes('piecewise fixture layout rejection')),'Failing viewport callback committed knots/controller/typed data or did not execute');
          if(r==='svg')assert(await page.locator('#piecewise-scales-svg svg').first().innerHTML()===rejectedMarkup,'Failing viewport callback repainted SVG');
          const afterPNG=await sampleBundle(r,label+'-after'),delta=await pngDifference(beforePNG.bytes,afterPNG.bytes);
          assert(canonicalState(rejected)===canonicalState(rejectedBase)&&beforePNG.samples[0].canonicalStateSha256===afterPNG.samples[0].canonicalStateSha256&&beforePNG.samples[0].canonicalDOMSha256===afterPNG.samples[0].canonicalDOMSha256&&JSON.stringify(beforePNG.samples[0].rect)===JSON.stringify(afterPNG.samples[0].rect),'Rejected viewport changed exact accepted state/DOM/rectangle');
          assert(stablePNG(delta),`${r}: rejected viewport exceeded settledScreenshotQuantization ${JSON.stringify(delta)}`);
          const screenDelta=await pngDifference(beforePNG.screenBytes,afterPNG.screenBytes);assert(screenGate(screenDelta),'Rejected viewport changed compositor screen beyond raw one-LSB bound');
          rollbacks.push({case:c,scatter,phase,renderer:r,profile:'actualPaintOutputWithBoundedCompositor',sampling:'fixed 1000ms+2RAF per group, then 3 actual samples per side; no retry; paint raw1/median1/4; screen raw1 only',before:beforePNG.samples,after:afterPNG.samples,delta,screenDelta,paintPNGStable:true,screenRawBounded:true,typedKnotsUnchanged:true});
        }
        await page.evaluate(()=>window.hostFixture.piecewise_scales_reject(false));
        const domains={x:(await state('svg')).xScale.domain,y:(await state('svg')).yScale.domain};
        await page.evaluate(()=>{for(const r of['svg','canvas'])document.getElementById(`piecewise-scales-${r}`).style.width='600px';});await flush();await setupPixelGrid(`family-case${c}-${scatter?'scatter':'line'}-phase${phase}-resize600`);
        for(const r of['svg','canvas']){const resized=await state(r);assert(resized.width===600,'Resize did not publish accepted dimensions');array(resized.xScale.domain,domains.x,resized.xScale.kind,'Resize X knots');array(resized.yScale.domain,domains.y,resized.yScale.kind,'Resize Y knots');mapped(resized,'resize');await pick(r);await frameAtTarget(r,`case${c}-${scatter?'scatter':'line'}-phase${phase}-resize`);}
        await page.evaluate(()=>{for(const r of['svg','canvas'])document.getElementById(`piecewise-scales-${r}`).style.width='640px';});await flush();await setupPixelGrid(`family-case${c}-${scatter?'scatter':'line'}-phase${phase}-resize640`);
      }
      await page.evaluate(()=>{window.__piecewiseOld=document.querySelector('#piecewise-scales-svg .kartis-datum');window.hostFixture.piecewise_scales_stop();});await flush();
      assert(await page.locator('body *').count()===baseline,'Stopped piecewise fixture retained DOM');await page.evaluate(()=>window.__piecewiseOld?.click());await flush();assert(!(await state('svg')).active,'Detached typed control revived retired owner');
    }
    current={section:'nonmonotonic'};const turning=[];
    for(const scatter of[false,true]){
      await page.evaluate(scatter=>window.hostFixture.piecewise_scales_mount(6,scatter,0),scatter);await flush();await setupPixelGrid(`turning-${scatter?'scatter':'line'}-mount`);
      for(const r of['svg','canvas']){const s=await state(r);assert(s.viewportRejected&&!s.transform&&s.sites.length===42,'Turning-range demo claimed a global inverse viewport');for(const site of s.sites)array(site.position,[make(s.xScale)(site.raw[0]),make(s.yScale)(site.raw[1])],'linear','Turning forward mapping');await pick(r);turning.push({renderer:r,scatter,forwardOnly:true,viewportRejected:true});}
      await capture(`turning-${scatter?'scatter':'line'}`);await page.evaluate(()=>window.hostFixture.piecewise_scales_stop());await flush();
    }
    current={section:'publicNavigation'};
    for(const [mode,c,scatter]of[['piecewise-line',0,false],['piecewise-scatter',1,true]]){
      await page.locator(`[data-mode="${mode}"]`).click();await flush();assert(await page.locator(`[data-mode="${mode}"]`).getAttribute('aria-pressed')==='true','Public piecewise mode not selected');await setupPixelGrid(`public-${mode}-mount`);
      const phases=[];let retained;
      for(const phase of[0,1]){
        if(phase){await page.locator('#next-style').click();await flush();await setupPixelGrid(`public-${mode}-phase1`);}
        const s=await state('svg');assert(s.case===c&&s.scatter===scatter&&s.phase===phase,'Public NextStyle retained another piecewise profile');mapped(s,mode);
        if(!phase){retained=s.sites[5].id;await page.evaluate(id=>{window.__piecewiseNavRetained=[...document.querySelectorAll('#piecewise-scales-svg [data-id]')].find(n=>n.dataset.id===id);},retained);}
        else assert(await page.evaluate(id=>window.__piecewiseNavRetained===[...document.querySelectorAll('#piecewise-scales-svg [data-id]')].find(n=>n.dataset.id===id),retained),'Public NextStyle replaced stable data node');
        phases.push({phase,typed:[await pick('svg'),await pick('canvas')]});
      }
      await page.locator('[data-mode="transform"]').click();await flush();assert(!(await state('svg')).active&&await page.locator('#piecewise-scales-svg').count()===0,'Leaving piecewise public mode retained owner');navigation.push({mode,phases,exitCleanup:true});
    }
    assert(errors.length===0,errors.join('\n'));
    let alternateDpr=null;
    if(!options.nested){const context=await page.context().browser().newContext({viewport:{width:1440,height:1000},deviceScaleFactor:dpr===1?2:1});try{const nested=await context.newPage();await nested.goto(page.url());await nested.waitForFunction(()=>!!window.hostFixture?.piecewise_scales_mount);alternateDpr=await verifyPiecewiseScales(nested,{nested:true});}finally{await context.close();}}
    return{passed:true,devicePixelRatio:dpr,sourceHashes,oracle:'Original d3-scale 4.0.2 + d3-zoom 3.0.0; actual trusted mouse/key input; nonmonotonic forward-only profile is separate',profiles:reports,turning,ink,frames,rollbacks,screenshotControls,acceptanceScope,hostOriginAlignments,fractionalProfiles,navigation,alternateDpr,cleanup:true,pageErrors:errors};
  }catch(error){
    const path=join(output,`piecewise-scales-failure-${suffix}.json`),diagnostic=await page.evaluate(()=>({packets:window.__piecewisePackets?.slice(-32),snapshots:['svg','canvas'].map(r=>{try{return JSON.parse(window.hostFixture.piecewise_scales_snapshot(r));}catch(e){return{r,error:String(e)};}})}));
    await writeFile(path,JSON.stringify({current,error:error.stack,diagnostic,acceptanceScope},null,2)+'\n');await page.screenshot({path:path.replace(/\.json$/,'.png'),fullPage:true});error.message+=` (diagnostic ${path})`;throw error;
  }finally{
    await page.evaluate(previous=>{const n=window.__piecewiseNegative;if(n){if(n.node){if(n.old===null)n.node.removeAttribute('transform');else n.node.setAttribute('transform',n.old);}else n.context.putImageData(n.data,0,0);delete window.__piecewiseNegative;}window.hostFixture.piecewise_scales_stop();for(const entry of window.__piecewiseListeners??[])for(const type of['wheel','pointerdown','pointerup','keydown'])entry.el.removeEventListener(type,entry.listener,true);for(const name of['__piecewiseListeners','__piecewisePackets','__piecewiseRetained','__piecewiseSurface','__piecewiseOld','__piecewiseNavRetained'])delete window[name];document.querySelector('.panels').style.display=previous;},previous);page.off('pageerror',onError);
  }
})
