// Real BrowserChart domain-relayout and shared-presentation acceptance.
// Called after the host fixture is loaded. Leaves both hosts at original phase 1.
(async page => {
  const errors = [];
  const onError = error => errors.push(error.message);
  page.on('pageerror', onError);
  const assert = (condition, message) => { if (!condition) throw Error(message); };
  const snapshot = id => page.evaluate(id => JSON.parse(window.hostFixture.snapshot(id)), id);
  const probe = id => page.evaluate(id => JSON.parse(window.hostFixture.viewport_probe(id)), id);
  const wait = (id, condition) => page.waitForFunction(({id, condition}) =>
    new Function('s', `return (${condition})(s)`)(JSON.parse(window.hostFixture.snapshot(id))),
    {id, condition: condition.toString()});
  const frames = () => page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  const client = async (id, x, y) => {
    const surface = page.locator(`#${id}-root .kartis-surface`);
    await surface.scrollIntoViewIfNeeded();
    const box = await surface.boundingBox(), state = await snapshot(id);
    return {x: box.x + x / state.width * box.width, y: box.y + y / state.height * box.height};
  };
  const axes = async id => {
    const state = await snapshot(id), data = await probe(id);
    if (id === 'svg') {
      const root = page.locator('#svg-root');
      const paths = await root.locator('svg path[data-id="axis-x"],svg path[data-id="axis-y"]').evaluateAll(paths => paths.map(path => ({id:path.dataset.id,d:path.getAttribute('d')})));
      assert(paths.length === 2, 'SVG: real Cartesian axis paths missing');
      return paths;
    }
    const ink = await page.locator('#canvas-root canvas').evaluate((canvas, {state, plot}) => {
      const ctx = canvas.getContext('2d'), sx = canvas.width/state.width, sy = canvas.height/state.height;
      const sample = (x,y) => {
        const pixels = ctx.getImageData(Math.max(0,Math.floor(x*sx)-1),Math.max(0,Math.floor(y*sy)-1),3,3).data;
        // The y=0 grid is intentionally painted over the horizontal baseline.
        for(let i=0;i<pixels.length;i+=4) if(Math.min(pixels[i],pixels[i+1],pixels[i+2])<240&&pixels[i+3])return true;
        return false;
      };
      let horizontal=0,vertical=0;
      for(let i=2;i<98;i++) {
        if(sample(plot.x+plot.width*i/100,plot.y+plot.height))horizontal++;
        if(sample(plot.x,plot.y+plot.height*i/100))vertical++;
      }
      return {horizontal,vertical};
    }, {state,plot:data.plot});
    assert(ink.horizontal > 85 && ink.vertical > 85, `Canvas: axes left their fixed pixel baselines ${JSON.stringify(ink)}`);
    return ink;
  };
  const hitCurrent = async id => {
    const data = await probe(id), point = data.points.find(point => point.id === 'point-0-0');
    assert(point, `${id}: center data point disappeared`);
    // Hover/control text may change document height while the browser is at
    // the scroll boundary. Measure after leaving the previous hover, then use
    // the current world-to-client mapping for each subsequent real input.
    await page.mouse.move(2,2); await frames();
    let p = await client(id,point.x,point.y);
    await page.mouse.move(p.x,p.y); await frames();
    p = await client(id,point.x,point.y);
    await page.mouse.move(p.x,p.y);
    await wait(id,s=>s.hovered.includes('point-0-0'));
    await frames();
    p = await client(id,point.x,point.y);
    await page.mouse.click(p.x,p.y);
    await wait(id,s=>s.selected==='point-0-0');
    if(id==='svg') {
      p = await client(id,point.x,point.y);
      await page.waitForFunction(p=>{
        const path=document.querySelector('#svg-root svg path[data-id="point-0-0"]');
        if(!path)return false;const r=path.getBoundingClientRect();
        return r.width>0&&Math.abs(r.x+r.width/2-p.x)<2&&Math.abs(r.y+r.height/2-p.y)<2;
      },p);
    } else {
      const blue = await page.locator('#canvas-root canvas').evaluate((canvas,{point,width,height})=>{
        const pixel=canvas.getContext('2d').getImageData(Math.floor(point.x/width*canvas.width),Math.floor(point.y/height*canvas.height),1,1).data;
        return pixel[0]<150&&pixel[2]>100&&pixel[3]>0;
      },{point,width:(await snapshot(id)).width,height:(await snapshot(id)).height});
      assert(blue,'Canvas: projected center pixel does not contain the current data mark');
    }
    return point;
  };
  await page.waitForFunction(()=>window.hostReady===true);
  const dates=await page.evaluate(()=>JSON.parse(window.hostFixture.date_time_samples()));
  const calendarTime=await page.evaluate(()=>JSON.parse(window.hostFixture.zoned_time_samples()));
  assert(JSON.stringify(calendarTime.days.map(t=>t.instant))===JSON.stringify([1709960400000,1710046800000,1710129600000,1710216000000]),'WASM local day ticks missed a 23-hour DST day');
  assert(JSON.stringify(calendarTime.hours.map(t=>t.instant))===JSON.stringify([1730606400000,1730610000000,1730613600000,1730617200000,1730620800000]),'WASM folded hours were omitted or duplicated as instants');
  assert(calendarTime.hours[1].label.includes('01:00:00.000 -0400')&&calendarTime.hours[2].label.includes('01:00:00.000 -0500'),'WASM repeated local hours lost their UTC offsets');
  assert(dates.beforeDst==='2024-03-10 01:59 -0500'&&dates.afterDst==='2024-03-10 03:00 -0400','Bundled WASM IANA zone missed the 2024 spring DST jump');
  assert(dates.futureSummer==='2101-07-01 08:00 -0400','WASM IANA future POSIX rules did not preserve 2101 summer DST');
  assert(dates.zh==='2024年3月10日 星期日 03:00:00 上午 3月 -0400','WASM Chinese locale/date-time formatting differs');
  assert(typeof dates.database==='string'&&dates.database.length>0,'IANA database version absent');
  await page.emulateMedia({reducedMotion:'reduce'});
  const cdp=await page.context().newCDPSession(page), reports=[];
  await cdp.send('Emulation.setTouchEmulationEnabled',{enabled:true,maxTouchPoints:5});
  try {
    for(const id of ['svg','canvas']) {
      await page.evaluate(id=>window.hostFixture.remount_viewport(id),id);
      await wait(id,s=>s.active&&s.domains!==null&&s.viewportCalls>0&&s.pending===0);
      const before=await snapshot(id), baseAxes=await axes(id);
      assert(before.domains.x[0]===0&&before.domains.x[1]===100&&before.domains.y[1]===200,`${id}: initial domains incorrect`);
      await hitCurrent(id);
      const initial=await probe(id), center=await client(id,initial.plot.x+initial.plot.width/2,initial.plot.y+initial.plot.height/2);
      await page.mouse.move(center.x,center.y);await page.mouse.wheel(0,-150);
      await wait(id,s=>s.pending===0&&s.domains.x[1]-s.domains.x[0]<90);
      const zoomed=await snapshot(id);
      assert(zoomed.domains.y[1]-zoomed.domains.y[0]<180,`${id}: wheel failed to resolve Y domain`);
      if(id==='svg')assert(JSON.stringify(await axes(id))===JSON.stringify(baseAxes),'SVG: wheel moved fixed axes');else await axes(id);
      await hitCurrent(id);
      await page.mouse.move(center.x,center.y);await page.mouse.down();await page.mouse.move(center.x+24,center.y+12,{steps:4});await page.mouse.up();
      await page.waitForFunction(({id,x})=>JSON.parse(window.hostFixture.snapshot(id)).domains.x[0]!==x,{id,x:zoomed.domains.x[0]});
      await wait(id,s=>s.pending===0);await hitCurrent(id);
      if(id==='svg')assert(JSON.stringify(await axes(id))===JSON.stringify(baseAxes),'SVG: pan moved fixed axes');else await axes(id);

      // Chrome dispatch produces real primary/nonprimary captured PointerEvents.
      const current=await hitCurrent(id), anchor=await client(id,current.x,current.y), box=await page.locator(`#${id}-root .kartis-surface`).boundingBox();
      const touch=(x,id)=>({x,y:anchor.y,id,radiusX:2,radiusY:2,force:1});
      const start=[touch(anchor.x-box.width*.12,101),touch(anchor.x+box.width*.12,202)];
      await cdp.send('Input.dispatchTouchEvent',{type:'touchStart',touchPoints:start});
      await wait(id,s=>s.pinching&&s.touches===2);
      const prePinch=await snapshot(id);
      await cdp.send('Input.dispatchTouchEvent',{type:'touchMove',touchPoints:[touch(anchor.x-box.width*.20,101),touch(anchor.x+box.width*.20,202)]});
      await page.waitForFunction(({id,span})=>{const d=JSON.parse(window.hostFixture.snapshot(id)).domains.x;return d[1]-d[0]<span*.8;},{id,span:prePinch.domains.x[1]-prePinch.domains.x[0]});
      await cdp.send('Input.dispatchTouchEvent',{type:'touchEnd',touchPoints:[]});await wait(id,s=>s.touches===0&&s.pending===0);
      await hitCurrent(id);if(id==='svg')assert(JSON.stringify(await axes(id))===JSON.stringify(baseAxes),'SVG: pinch moved fixed axes');else await axes(id);

      const preResize=await snapshot(id);
      await page.locator(`#${id}-root`).evaluate(el=>el.style.width='86%');
      await page.waitForFunction(({id,width})=>JSON.parse(window.hostFixture.snapshot(id)).width<width-15,{id,width:preResize.width});
      await wait(id,s=>s.pending===0);const resized=await snapshot(id), resizedProbe=await probe(id);
      assert(JSON.stringify(resized.domains)===JSON.stringify(preResize.domains),`${id}: resize reset accepted domains`);
      assert(resized.layouts===before.layouts,`${id}: on_resize ran while on_viewport took precedence`);
      assert(resizedProbe.plot.x===56&&resizedProbe.plot.y===24&&Math.abs(resized.width-resizedProbe.plot.width-80)<.001,`${id}: resize lost fixed plot insets`);
      await hitCurrent(id);await axes(id);

      // Failed callback leaves the entire accepted view, domains and geometry.
      await page.evaluate(id=>{window.hostFixture.clear_events(id);window.hostFixture.viewport_mode(id,1);},id);
      const accepted=await snapshot(id), acceptedGeometry=await probe(id), acceptedPath=id==='svg'?await axes(id):null;
      const rejectionPoint=await client(id,acceptedGeometry.plot.x+acceptedGeometry.plot.width/2,acceptedGeometry.plot.y+acceptedGeometry.plot.height/2);
      await page.mouse.move(rejectionPoint.x,rejectionPoint.y);await page.mouse.wheel(0,-120);
      await page.waitForFunction(id=>JSON.parse(window.hostFixture.events(id)).some(e=>e.type==='error'&&e.message.includes('viewport layout rejection')),id);
      await wait(id,s=>s.pending===0);const rejected=await snapshot(id);
      assert(JSON.stringify(rejected.domains)===JSON.stringify(accepted.domains)&&JSON.stringify(rejected.view)===JSON.stringify(accepted.view),`${id}: failed layout committed domains or transform`);
      assert(JSON.stringify((await probe(id)).points)===JSON.stringify(acceptedGeometry.points),`${id}: failed layout replaced current geometry`);
      if(id==='svg')assert(JSON.stringify(await axes(id))===JSON.stringify(acceptedPath),'SVG: failed layout mutated real axes');
      await hitCurrent(id);
      await page.evaluate(id=>{window.hostFixture.viewport_mode(id,0);window.hostFixture.refresh_viewport(id);},id);await wait(id,s=>s.pending===0);

      // A final Up coordinate can propose a view even with no intermediate
      // Move. Rejection must retire that ended mouse rather than resume a pan.
      await page.mouse.move(rejectionPoint.x,rejectionPoint.y);await page.mouse.down();await wait(id,s=>s.pending===0);
      const beforeUp=await snapshot(id);
      await page.evaluate(id=>{window.hostFixture.clear_events(id);window.hostFixture.viewport_mode(id,1);},id);
      await cdp.send('Input.dispatchMouseEvent',{type:'mouseReleased',x:rejectionPoint.x+14,y:rejectionPoint.y+9,button:'left',buttons:0,clickCount:1});
      await page.waitForFunction(id=>JSON.parse(window.hostFixture.events(id)).some(e=>e.type==='error'&&e.message.includes('viewport layout rejection')),id);
      await wait(id,s=>s.pending===0);
      const afterUp=await snapshot(id);
      assert(JSON.stringify(afterUp.view)===JSON.stringify(beforeUp.view)&&JSON.stringify(afterUp.domains)===JSON.stringify(beforeUp.domains),`${id}: rejected final mouse Up committed a view`);
      await page.evaluate(id=>window.hostFixture.viewport_mode(id,0),id);
      await page.mouse.move(rejectionPoint.x+32,rejectionPoint.y+19);await frames();
      assert(JSON.stringify((await snapshot(id)).view)===JSON.stringify(beforeUp.view),`${id}: rejected Up left a ghost mouse drag`);

      const contact=(x,id)=>({x,y:rejectionPoint.y,id,radiusX:2,radiusY:2,force:1});
      const a=contact(rejectionPoint.x-45,301),b=contact(rejectionPoint.x+45,302);
      await cdp.send('Input.dispatchTouchEvent',{type:'touchStart',touchPoints:[a,b]});await wait(id,s=>s.pinching&&s.touches===2&&s.capturedTouches===2);
      const beforeEnd=await snapshot(id);
      await page.evaluate(id=>{window.hostFixture.clear_events(id);window.hostFixture.viewport_mode(id,1);},id);
      await cdp.send('Input.dispatchTouchEvent',{type:'touchEnd',touchPoints:[{...a,x:a.x-20}]});
      await page.waitForFunction(id=>JSON.parse(window.hostFixture.events(id)).some(e=>e.type==='error'&&e.message.includes('viewport layout rejection')),id);
      await wait(id,s=>s.pending===0&&s.touches===1&&s.capturedTouches===1&&!s.pinching);
      const afterEnd=await snapshot(id);
      assert(JSON.stringify(afterEnd.view)===JSON.stringify(beforeEnd.view)&&JSON.stringify(afterEnd.domains)===JSON.stringify(beforeEnd.domains),`${id}: rejected final TouchEnd committed a view`);
      await cdp.send('Input.dispatchTouchEvent',{type:'touchEnd',touchPoints:[]});await wait(id,s=>s.touches===0&&s.capturedTouches===0&&s.pending===0);
      await page.evaluate(id=>window.hostFixture.viewport_mode(id,0),id);
      await page.mouse.move(rejectionPoint.x+40,rejectionPoint.y+20);await frames();
      assert(JSON.stringify((await snapshot(id)).view)===JSON.stringify(beforeEnd.view),`${id}: rejected TouchEnd left a ghost gesture`);
      await hitCurrent(id);

      // Old callback's deliberately wrong result must not overwrite its replacement.
      for(const mode of [4,3]) {
        const preReplace=await snapshot(id);
        await page.evaluate(({id,mode})=>{window.hostFixture.clear_events(id);window.hostFixture.viewport_mode(id,mode);window.hostFixture.refresh_viewport(id);},{id,mode});
        await page.waitForFunction(({id,count})=>{const s=JSON.parse(window.hostFixture.snapshot(id));return s.viewportReplacements>count&&s.pending===0;},{id,count:preReplace.viewportReplacements});
        assert(JSON.stringify((await snapshot(id)).domains)===JSON.stringify(preReplace.domains),`${id}: callback replacement reset accepted domains ${JSON.stringify({before:preReplace.domains,after:(await snapshot(id)).domains})}`);
        assert(!(await page.locator(`#${id}-root .kartis-data`).textContent()).includes('999'),`${id}: stale callback result committed`);
        assert(!await page.evaluate(id=>JSON.parse(window.hostFixture.events(id)).some(e=>e.type==='error'&&e.message.includes('stale viewport callback failure')),id),`${id}: replaced callback's obsolete error was published`);
        await hitCurrent(id);
      }
      await page.evaluate(id=>{window.hostFixture.viewport_mode(id,2);window.hostFixture.refresh_viewport(id);window.hostFixture.flush(id);},id);await wait(id,s=>!s.active);
      assert(await page.locator(`#${id}-root`).evaluate(el=>el.childElementCount)===0,`${id}: viewport callback stop retained DOM`);
      await page.locator(`#${id}-root`).evaluate(el=>el.style.width='');
      reports.push({renderer:id,wheel:true,pan:true,pinch:true,fixedAxes:true,currentPixelHit:true,resizePreservesDomains:true,layoutErrorAtomic:true,rejectedMouseUpRetired:true,rejectedTouchEndRetired:true,replacement:true,obsoleteCallbackErrorSuppressed:true,callbackStopFlushOk:true});

      await page.evaluate(id=>window.hostFixture.remount_presentation(id),id);await wait(id,s=>s.active&&s.pending===0);
      const root=page.locator(`#${id}-root`), legend=root.locator('.kartis-legend');
      const styles=await legend.evaluate(el=>({direction:getComputedStyle(el).flexDirection,wrap:getComputedStyle(el).flexWrap,gap:getComputedStyle(el).gap,align:getComputedStyle(el).alignItems}));
      assert(styles.direction==='column'&&styles.wrap==='nowrap'&&styles.gap==='11px'&&styles.align==='flex-end',`${id}: configured real legend layout absent ${JSON.stringify(styles)}`);
      const first=root.locator('.kartis-legend-item').first(), symbol=root.locator('.kartis-legend-symbol').first();
      assert((await first.textContent()).includes('[Alpha <safe>]'),`${id}: explicit legend display label missing`);
      assert(await first.locator('safe').count()===0,`${id}: legend label interpreted as HTML`);
      const symbolStyle=await symbol.evaluate(el=>({radius:getComputedStyle(el).borderRadius,width:getComputedStyle(el).width,color:getComputedStyle(el).backgroundColor}));
      assert(symbolStyle.radius==='8px'&&symbolStyle.width==='16px'&&symbolStyle.color==='rgb(180, 30, 90)',`${id}: legend symbol config absent`);
      const itemStyle=await first.evaluate(el=>({size:getComputedStyle(el).fontSize,weight:getComputedStyle(el).fontWeight,gap:getComputedStyle(el).gap,line:getComputedStyle(el).lineHeight}));
      assert(itemStyle.size==='17px'&&itemStyle.weight==='650'&&itemStyle.gap==='7px'&&Math.abs(parseFloat(itemStyle.line)-28.9)<.01,`${id}: configured legend text styles absent ${JSON.stringify(itemStyle)}`);
      const presentation=await snapshot(id), point=await client(id,presentation.width*.24,presentation.height*.425);
      await page.mouse.move(2,2);await page.mouse.move(point.x,point.y);await wait(id,s=>s.hovered.includes('datum-alpha'));
      const tooltip=root.locator('.kartis-surface-tooltip');
      assert(await tooltip.textContent()==='<b>Safe tooltip</b>\nAlpha <safe> = 10.00\nid: datum-alpha',`${id}: typed tooltip template/value format not applied`);
      assert(await tooltip.locator('b,safe').count()===0,`${id}: tooltip content interpreted as HTML`);
      const tooltipStyle=await tooltip.evaluate(el=>({size:getComputedStyle(el).fontSize,padding:getComputedStyle(el).padding,color:getComputedStyle(el).backgroundColor,line:getComputedStyle(el).lineHeight}));
      assert(tooltipStyle.size==='16px'&&tooltipStyle.padding==='13px'&&tooltipStyle.color==='rgb(12, 34, 56)'&&Math.abs(parseFloat(tooltipStyle.line)-28.8)<.01,`${id}: configured browser tooltip style absent ${JSON.stringify(tooltipStyle)}`);
      await first.click();await wait(id,s=>s.hidden.includes('alpha'));
      assert(await root.locator('.kartis-datum').count()===1,`${id}: presentation options broke actual legend filtering`);
      reports.at(-1).presentation={safeTemplates:true,formattedValue:true,realStyles:true,legendFilter:true};
      await page.evaluate(id=>{window.hostFixture.remount(id);window.hostFixture.set_phase(id,1,false);},id);
      await wait(id,s=>s.active&&s.phase===1&&s.pending===0&&!s.animating);
    }
    assert(errors.length===0,errors.join('\n'));
    return {passed:true,renderers:reports,dateTime:dates,calendarTime,pageErrors:errors};
  } finally {
    await cdp.send('Input.dispatchTouchEvent',{type:'touchCancel',touchPoints:[]}).catch(()=>{});
    await cdp.send('Emulation.setTouchEmulationEnabled',{enabled:false});
    await cdp.detach();
    await page.emulateMedia({reducedMotion:'no-preference'});
    page.off('pageerror',onError);
  }
})
