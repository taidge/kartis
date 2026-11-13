(async function verifyGeoInverse(page, options = {}) {
  const {readFile,writeFile,mkdir}=await import('node:fs/promises');
  const {createHash,randomUUID}=await import('node:crypto');
  const assert=(ok,message)=>{if(!ok)throw Error(message);};
  const close=(a,b,epsilon=1e-7)=>Number.isFinite(a)&&Number.isFinite(b)&&Math.abs(a-b)<=epsilon;
  const bundle=await readFile('tests/fixtures/geo-inverse-d3-oracle.js','utf8');
  const manifest=JSON.parse(await readFile('tests/fixtures/geo-inverse-d3-source.json','utf8'));
  const oracleSha256=createHash('sha256').update(bundle).digest('hex');
  assert(oracleSha256===manifest.bundle.sha256,'Geo original D3 oracle hash mismatch');
  const suffix=randomUUID(),errors=[],onError=e=>errors.push(e.message),cases=[],css=[],camera=[],ink=[],navigation=[];
  let current={section:'initialization'};
  await mkdir('target/gallery',{recursive:true});
  page.on('pageerror',onError);
  const previous=await page.evaluate(()=>{const panels=document.querySelector('.panels'),display=panels.style.display;panels.style.display='none';return display;});
  const baseline=await page.locator('body *').count();
  const flush=async()=>{await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));await page.evaluate(()=>window.hostFixture.geo_inverse_flush());};
  const state=r=>page.evaluate(r=>JSON.parse(window.hostFixture.geo_inverse_snapshot(r)),r);
  const surface=r=>page.locator(`#geo-inverse-${r} .kartis-surface`);
  await page.evaluate(bundle=>{
    const frame=document.createElement('iframe');frame.id='geo-inverse-original-oracle';frame.hidden=true;document.body.append(frame);
    const script=frame.contentDocument.createElement('script');script.textContent=bundle;frame.contentDocument.head.append(script);
    const d3=frame.contentWindow.d3;
    window.__geoInverseOracle=s=>{
      const names=['geoEquirectangular','geoMercator','geoTransverseMercator','geoEqualEarth','geoNaturalEarth1','geoOrthographic','geoAzimuthalEqualArea','geoAzimuthalEquidistant','geoStereographic','geoGnomonic'];
      const k=s.scale,t=s.translate,c=s.fittedRawCenter;
      return d3[names[s.case]]().rotate([-s.center[0],-s.center[1],-s.roll]).scale(k).translate([t[0]-c[0]*k,t[1]+c[1]*k]);
    };
    window.__geoInversePackets=[];
    window.__geoInversePacketListener=e=>window.__geoInversePackets.push({type:e.type,trusted:e.isTrusted,key:e.key,client:[e.clientX,e.clientY],renderer:e.target.closest?.('[id^="geo-inverse-"]')?.id});
    for(const type of['wheel','keydown','pointerdown','pointerup'])document.addEventListener(type,window.__geoInversePacketListener,true);
  },bundle);
  const point=async(r,world,expected='some')=>{
    current={...current,renderer:r,world,expected};
    await surface(r).scrollIntoViewIfNeeded();const before=await state(r),box=await surface(r).boundingBox();
    const viewport=[before.plot[0]+(world[0]-before.plot[0])*before.view.scale+before.view.x,before.plot[1]+(world[1]-before.plot[1])*before.view.scale+before.view.y];
    const client=[box.x+viewport[0]/before.width*box.width,box.y+viewport[1]/before.height*box.height];
    await page.mouse.move(client[0]+3,client[1]+2);await page.mouse.move(...client);await flush();
    const s=await state(r),receipt=s.receipt;
    assert(!receipt.error&&receipt.trusted&&receipt.pointerType==='mouse'&&receipt.count>(before.receipt.count??0),`${r} inverse did not consume a new trusted native pointer: ${JSON.stringify(s)}`);
    const independent=await page.evaluate(({r,s})=>{
      const root=document.querySelector(`#geo-inverse-${r}`),paint=root.querySelector(r==='svg'?'svg':'canvas'),b=paint.getBoundingClientRect();
      let matrix=null;
      if(r==='svg'){
        const c=paint.getScreenCTM(),p=new DOMPoint(...s.receipt.client).matrixTransform(c.inverse());
        const floatUlp=value=>{const f=new Float32Array([Math.abs(value)]),u=new Uint32Array(f.buffer);u[0]++;return f[0]-Math.fround(Math.abs(value));};
        matrix={a:c.a,b:c.b,c:c.c,d:c.d,e:c.e,f:c.f,viewport:[p.x,p.y],originUlpBudget:c.a===1&&c.d===1?1:2,originUlps:[Math.abs(c.e-b.x)/floatUlp(Math.max(Math.abs(c.e),Math.abs(b.x))),Math.abs(c.f-b.y)/floatUlp(Math.max(Math.abs(c.f),Math.abs(b.y)))]};
      }
      // The host contract is CSS bounds. Read the distinct actual paint root,
      // rather than its surrounding input surface. Firefox's SVG matrix
      // translation and CSSRect differ by up to two float32 ULPs in the covered
      // composed CSS transforms (one ULP without scaling); retain both
      // measurements and bound that discrepancy separately. This suite uses
      // positive axis scaling only, and verifies that matrix prerequisite.
      const viewport=[(s.receipt.client[0]-b.x)*s.width/b.width,(s.receipt.client[1]-b.y)*s.height/b.height];
      const world=[s.plot[0]+(viewport[0]-s.plot[0]-s.view.x)/s.view.scale,s.plot[1]+(viewport[1]-s.plot[1]-s.view.y)/s.view.scale];
      const p=window.__geoInverseOracle(s),position=p.invert(world),projected=position&&p(position);
      let painted=null;
      if(r==='canvas'&&s.receipt.marker){
        const [x,y]=s.receipt.marker,vx=s.plot[0]+(x-s.plot[0])*s.view.scale+s.view.x,vy=s.plot[1]+(y-s.plot[1])*s.view.scale+s.view.y;
        painted=[...paint.getContext('2d').getImageData(Math.floor(vx*paint.width/s.width),Math.floor(vy*paint.height/s.height),1,1).data];
      }
      const marker=root.querySelector('[data-id="geo-inverse-marker"]'),start=marker?.getAttribute('d').match(/^M([^ ]+) ([^C]+)C/);
      return{world,position,projected,painted,matrix,svgMarker:marker?{cx:Number(start?.[1])-5,cy:Number(start?.[2]),visible:marker.getBoundingClientRect().width>0&&getComputedStyle(marker).fill==='rgb(215, 35, 75)'}:null,paintBox:[b.x,b.y,b.width,b.height]};
    },{r,s});
    if(r==='svg')assert(independent.matrix.a>0&&independent.matrix.d>0&&independent.matrix.b===0&&independent.matrix.c===0&&close(independent.matrix.a,independent.paintBox[2]/s.width)&&close(independent.matrix.d,independent.paintBox[3]/s.height)&&independent.matrix.originUlps.every(v=>v<=independent.matrix.originUlpBudget),`SVG paint bounds/matrix differ beyond float32 representation or contain unsupported rotation/shear ${JSON.stringify({matrix:independent.matrix,paintBox:independent.paintBox})}`);
    assert(receipt.scene.every((v,i)=>close(v,independent.world[i])),`${r} public client-to-Scene chain drifted from actual paint ${JSON.stringify({receipt,independent})}`);
    const readout=page.locator(`#geo-inverse-${r} output.kartis-geo-inverse-status`),readoutText=await readout.textContent();
    assert(await readout.getAttribute('aria-live')==='polite'&&await readout.getAttribute('aria-label')==='Geographic coordinates','Geographic coordinate output is not accessible');
    if(expected==='some'){
      assert(receipt.position&&receipt.marker&&independent.position&&receipt.position.every((v,i)=>close(v,independent.position[i])),`${r} geographic inverse differs from original D3 ${JSON.stringify({receipt,independent})}`);
      assert(receipt.marker.every((v,i)=>close(v,independent.world[i]))&&independent.projected.every((v,i)=>close(v,independent.world[i])),`${r} geographic inverse marker failed round trip`);
      if(r==='canvas')assert(independent.painted?.[0]>150&&independent.painted[1]<100&&independent.painted[3]===255,`Canvas inverse marker was not actually painted ${JSON.stringify(independent.painted)}`);
      else assert(independent.svgMarker?.visible&&close(independent.svgMarker.cx,receipt.marker[0])&&close(independent.svgMarker.cy,receipt.marker[1]),'SVG inverse marker missing or stale');
      const values=readoutText.match(/^Longitude ([-\d.]+)°, latitude ([-\d.]+)°$/);
      assert(values&&receipt.position.every((v,i)=>close(Number(values[i+1]),v,.000005001)),`Visible geographic readout is stale: ${readoutText}`);
    }else{
      assert(receipt.position===null&&receipt.marker===null,'Outside geographic/plot footprint produced a visible marker');
      if(r==='svg')assert(independent.svgMarker===null,'Outside inverse retained an old SVG marker');
      if(expected==='intrinsicNone')assert(receipt.intrinsic===null,'Intrinsic geographic footprint resurrected an outside point');
      if(expected==='plotNone'&&s.case===0&&s.phase===1)assert(receipt.intrinsic!==null,'Explicit plot clipping was not distinguished from the projection intrinsic inverse');
      assert(readoutText==='Outside the visible map.','Outside geographic footprint retained stale longitude/latitude text');
    }
    return{trusted:true,sourceClient:receipt.client,scene:receipt.scene,paintWorld:independent.world,paintBounds:independent.paintBox,svgMatrix:independent.matrix,position:receipt.position,intrinsic:receipt.intrinsic,marker:receipt.marker,originalD3:independent.position,painted:r==='svg'?!!independent.svgMarker?.visible:!!independent.painted,readout:readoutText,expected};
  };
  try{
    for(let c=0;c<10;c++){
      await page.evaluate(c=>window.hostFixture.geo_inverse_mount(c,0),c);await flush();
      for(let phase=0;phase<3;phase++){
        if(phase){await page.evaluate(phase=>window.hostFixture.geo_inverse_phase(phase),phase);await flush();}
        for(const r of['svg','canvas']){
          current={case:c,phase,renderer:r,section:'geometry'};
          const s=await state(r),points=[];
          assert(s.fit===(phase===2?'Data':'World')&&s.probePoint&&s.anchorPoint,'Geo current projection fixture unavailable');
          if(phase===2)assert(s.fittedRawCenter.some(v=>Math.abs(v)>.01),'DataFit inverse did not exercise a nonzero fitted raw center');
          if(!s.receipt.count)assert(await page.locator(`#geo-inverse-${r} output`).textContent()==='Move over the map to read longitude and latitude.','Accepted new projection retained a stale coordinate readout');
          points.push(await point(r,s.probePoint));points.push(await point(r,s.anchorPoint));
          await page.mouse.move(2,2);await flush();
          const beforeKey=await state(r),control=page.locator(`#geo-inverse-${r} .kartis-datum`).filter({hasText:'Geographic anchor'}).first();
          const beforeKeyReadout=await page.locator(`#geo-inverse-${r} output`).textContent();
          const packetStart=await page.evaluate(()=>window.__geoInversePackets.length);
          await control.focus();await page.keyboard.press('Enter');await flush();const afterKey=await state(r);
          const trustedEnter=await page.evaluate(({start,r})=>window.__geoInversePackets.slice(start).some(p=>p.type==='keydown'&&p.key==='Enter'&&p.trusted&&p.renderer===`geo-inverse-${r}`),{start:packetStart,r});
          assert(trustedEnter&&afterKey.events.some(e=>e.type==='activated'&&e.id==='geo-symbol:6:anchor')&&JSON.stringify(afterKey.receipt)===JSON.stringify(beforeKey.receipt)&&await page.locator(`#geo-inverse-${r} output`).textContent()===beforeKeyReadout,'Geo keyboard activation fabricated/replaced a pointer inverse');
          const paint=page.locator(`#geo-inverse-${r} ${r==='svg'?'svg':'canvas'}`).first();
          const fingerprint=node=>({markup:node.outerHTML,pixels:node instanceof HTMLCanvasElement?node.toDataURL():null});
          const accepted=await state(r),frame=await paint.evaluate(fingerprint);
          assert(await page.evaluate(()=>window.hostFixture.geo_inverse_rejected()),'Geo inverse accepted nonfinite/over-budget proposal');
          const refused=await state(r);assert(refused.revision===accepted.revision&&refused.pending===0&&JSON.stringify(refused.receipt)===JSON.stringify(accepted.receipt),'Rejected inverse proposal changed accepted frame');
          assert(JSON.stringify(await paint.evaluate(fingerprint))===JSON.stringify(frame),'Rejected inverse changed the accepted SVG/Canvas frame');
          await flush();assert(JSON.stringify(await paint.evaluate(fingerprint))===JSON.stringify(frame),'Rejected inverse changed ink on the next explicit frame');
          let boundaries;
          if(phase===1){
            current={...current,section:'boundaries'};
            boundaries={outsidePlot:await point(r,[s.plot[0]-8,190],'plotNone'),outsideProjection:await point(r,[600,190],'intrinsicNone')};
            await page.locator(`#geo-inverse-${r}`).evaluate(root=>{root.style.transformOrigin='0 0';root.style.transform='translate(23px, 17px) scale(1.25, .75)';});
            current={...current,section:'css'};
            const actual=await point(r,s.probePoint);css.push({case:c,renderer:r,scale:[1.25,.75],...actual});
            await page.locator(`#geo-inverse-${r}`).evaluate(root=>root.style.transform='');await flush();
            await point(r,s.probePoint);
            current={...current,section:'sceneCamera'};
            const box=await surface(r).boundingBox();await page.mouse.move(box.x+box.width*.5,box.y+box.height*.5);const wheelStart=await page.evaluate(()=>window.__geoInversePackets.length);await page.mouse.wheel(0,-180);await flush();
            const trustedWheel=await page.evaluate(({start,r})=>window.__geoInversePackets.slice(start).some(p=>p.type==='wheel'&&p.trusted&&p.renderer===`geo-inverse-${r}`),{start:wheelStart,r});
            const zoom=await state(r);assert(trustedWheel&&zoom.view.scale>1,'Trusted geographic wheel did not change Scene camera');
            const actualCamera=await point(r,s.probePoint),panBefore=await state(r),panBox=await surface(r).boundingBox(),panStart=[panBox.x+panBox.width*.5,panBox.y+panBox.height*.5];
            const panPaintBounds=await page.locator(`#geo-inverse-${r} ${r==='svg'?'svg':'canvas'}`).first().evaluate(paint=>{const b=paint.getBoundingClientRect();return{x:b.x,y:b.y,width:b.width,height:b.height};});
            await page.mouse.move(...panStart);const panPacketStart=await page.evaluate(()=>window.__geoInversePackets.length);await page.mouse.down();await flush();
            await page.mouse.move(panStart[0]+18,panStart[1]+12,{steps:3});await flush();await page.mouse.up();await flush();
            const panAfter=await state(r),panPackets=await page.evaluate(({start,r})=>window.__geoInversePackets.slice(start).filter(p=>p.renderer===`geo-inverse-${r}`&&['pointerdown','pointerup'].includes(p.type)),{start:panPacketStart,r});
            const down=panPackets.find(p=>p.type==='pointerdown'),up=panPackets.find(p=>p.type==='pointerup');
            assert(down?.trusted&&up?.trusted&&panAfter.receipt.eventType==='pointerup'&&panAfter.receipt.trusted,'Geographic pan release did not refresh from a real completed gesture');
            const delta=[(up.client[0]-down.client[0])*s.width/panPaintBounds.width,(up.client[1]-down.client[1])*s.height/panPaintBounds.height];
            assert(close(panAfter.view.x,panBefore.view.x+delta[0])&&close(panAfter.view.y,panBefore.view.y+delta[1]),`Geographic hover marker interrupted native pan authority ${JSON.stringify({before:panBefore.view,after:panAfter.view,delta,panPackets,panBox,panPaintBounds})}`);
            const afterPan=await point(r,s.probePoint);camera.push({case:c,renderer:r,view:zoom.view,trustedWheel,...actualCamera,pan:{trustedDown:true,trustedUp:true,delta,pointerBounds:panBox,paintBounds:panPaintBounds,packets:panPackets,from:panBefore.view,to:panAfter.view,release:panAfter.receipt,afterPan}});
            await page.evaluate(r=>window.hostFixture.geo_inverse_reset_view(r),r);await flush();assert((await state(r)).view.scale===1,'Geo camera reset failed');
            if(c===0){await point(r,s.probePoint);const path=`target/gallery/geo-inverse-${r}-dpr${await page.evaluate(()=>devicePixelRatio)}-${suffix}.png`;await surface(r).screenshot({path});const bytes=await readFile(path);ink.push({renderer:r,path,sha256:createHash('sha256').update(bytes).digest('hex'),bytes:bytes.length});}
          }
          cases.push({case:c,phase,renderer:r,points,keyboard:true,invalidAtomic:true,boundaries});
        }
      }
      const old=await page.evaluate(()=>{const controls=['svg','canvas'].map(r=>document.querySelector(`#geo-inverse-${r} .kartis-datum`));window.__oldGeoInverse=controls;window.hostFixture.geo_inverse_stop();return controls.filter(Boolean).length;});
      assert(old===2,'Geo fixture surfaces missing before stop');await flush();
      for(const r of['svg','canvas']){const stopped=await state(r);assert(!stopped.active&&stopped.retired&&stopped.handleStopped&&stopped.callbackReleased,'Geo stop retained callback/handle resources');}
      assert(await page.locator('body *').count()===baseline+1,'Geo stop leaked owned DOM');
      await page.evaluate(()=>{for(const old of window.__oldGeoInverse)old.click();delete window.__oldGeoInverse;});await flush();
      assert(!(await state('svg')).active,'Detached old geographic control revived stopped chart');
    }
    current={section:'publicNavigation'};
    await page.locator('[data-mode="geo-inverse"]').click();await flush();
    assert(await page.locator('[data-mode="geo-inverse"]').getAttribute('aria-pressed')==='true','Geographic inverse navigation was not selected');
    assert((await page.locator('#mode-help').textContent()).includes('longitude'),'Public geographic inverse help retained another demo description');
    const phases=[];
    for(const phase of[0,1,2]){
      if(phase){await page.locator('#next-style').click();await flush();}
      const points=[];
      for(const r of['svg','canvas']){const s=await state(r);assert(s.active&&s.case===0&&s.phase===phase,'Public geographic inverse mode retained an old projection');points.push(await point(r,s.probePoint));}
      phases.push({phase,points});
    }
    await page.locator('[data-mode="transform"]').click();await flush();
    for(const r of['svg','canvas']){const s=await state(r);assert(!s.active&&s.callbackReleased&&await page.locator(`#geo-inverse-${r}`).count()===0,'Leaving public geographic inverse mode retained owned resources');}
    navigation.push({mode:'geo-inverse',phases,exitCleanup:true});
    assert(errors.length===0,errors.join('\n'));
    let dpr1=null;
    const dpr=await page.evaluate(()=>devicePixelRatio);
    if(!options.nested&&dpr!==1){
      const context=await page.context().browser().newContext({viewport:{width:1280,height:1100},deviceScaleFactor:1});
      try{const nested=await context.newPage();await nested.goto(page.url());await nested.waitForFunction(()=>!!window.hostFixture?.geo_inverse_mount);dpr1=await verifyGeoInverse(nested,{nested:true});}finally{await context.close();}
    }
    return{passed:true,devicePixelRatio:dpr,oracleSha256,oracleScope:'original vendored d3-geo inverse; Rust consumes trusted native mouse/wheel/keyboard; no injected pointer events',cases,css,camera,ink,navigation,dpr1,cleanup:true,pageErrors:errors};
  }catch(error){
    const path=`target/gallery/geo-inverse-failure-${suffix}.json`;
    const diagnostic=await page.evaluate(()=>({scrollY,activeElement:document.activeElement?.outerHTML,packets:window.__geoInversePackets?.slice(-32),snapshots:['svg','canvas'].map(r=>{try{return JSON.parse(window.hostFixture.geo_inverse_snapshot(r));}catch(error){return{renderer:r,error:String(error)};}})}));
    await writeFile(path,JSON.stringify({current,error:error.stack,diagnostic},null,2)+'\n');
    await page.screenshot({path:path.replace(/\.json$/,'.png'),fullPage:true});
    error.message+=` (diagnostic ${path})`;throw error;
  }finally{
    await page.evaluate(previous=>{window.hostFixture.geo_inverse_stop();for(const type of['wheel','keydown','pointerdown','pointerup'])document.removeEventListener(type,window.__geoInversePacketListener,true);document.querySelector('#geo-inverse-original-oracle')?.remove();document.querySelector('.panels').style.display=previous;delete window.__geoInverseOracle;delete window.__oldGeoInverse;delete window.__geoInversePackets;delete window.__geoInversePacketListener;},previous);
    page.off('pageerror',onError);
  }
})
