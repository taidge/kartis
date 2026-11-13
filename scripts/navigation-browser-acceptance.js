(async page => {
  const assert=(ok,message)=>{if(!ok)throw Error(message);};
  const snapshot=id=>page.evaluate(id=>JSON.parse(window.hostFixture.snapshot(id)),id);
  const probe=id=>page.evaluate(id=>JSON.parse(window.hostFixture.navigation_probe(id)),id);
  const wait=id=>page.waitForFunction(id=>{const s=JSON.parse(window.hostFixture.snapshot(id));return s.domains&&s.pending===0;},id);
  const flush=id=>page.evaluate(id=>window.hostFixture.flush(id),id);
  const client=async(id,mark)=>{const surface=page.locator(`#${id}-root .kartis-surface`);await surface.scrollIntoViewIfNeeded();const box=await surface.boundingBox(),s=await snapshot(id);return{x:box.x+mark.x/s.width*box.width,y:box.y+mark.y/s.height*box.height};};
  const frames=()=>page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
  const reports=[];
  for(const id of ['svg','canvas'])for(const category of [false,true]){
    await page.evaluate(({id,category})=>window.hostFixture.remount_navigation(id,category),{id,category});await flush(id);await wait(id);
    await page.evaluate(id=>window.hostFixture.navigation_zoom(id,0,4),id);await flush(id);
    const xOnly=await snapshot(id);assert(xOnly.domains.y[0]===0,`${id}: X request changed Y`);
    await page.evaluate(id=>window.hostFixture.navigation_zoom(id,1,2),id);await flush(id);
    const independent=await snapshot(id), p0=await probe(id);
    assert(Math.abs(p0.zoom[0]-4)<1e-9&&Math.abs(p0.zoom[1]-2)<1e-9,`${id}: independent scales lost ${JSON.stringify(p0.zoom)}`);
    assert(JSON.stringify(independent.domains.x)===JSON.stringify(xOnly.domains.x),`${id}: Y request changed X`);
    if(category){const labels=p0.labels.filter(s=>s.startsWith('Month '));assert(labels.length>=2&&labels.length<=4,`${id}: category ticks did not follow visible domain`);assert(p0.marks.length<24&&p0.marks.length>=4,`${id}: category marks did not crop`);}
    const target=p0.marks.find(m=>m.x>=p0.plot.x+2&&m.x<=p0.plot.x+p0.plot.width-2&&m.y>=p0.plot.y+2&&m.y<=p0.plot.y+p0.plot.height-2);
    assert(target,`${id}: navigation has no hittable mark`);
    // Hover text may change page height at the scroll boundary. Re-measure the
    // same logical mark after that layout, before sending the actual click.
    await page.mouse.move(2,2);await frames();let point=await client(id,target);
    await page.mouse.move(point.x,point.y);await frames();point=await client(id,target);
    await page.mouse.move(point.x,point.y);
    await page.waitForFunction(({id,mark})=>JSON.parse(window.hostFixture.snapshot(id)).hovered.includes(mark),{id,mark:target.id});
    await frames();point=await client(id,target);
    await page.mouse.click(point.x,point.y);await page.waitForFunction(({id,mark})=>JSON.parse(window.hostFixture.snapshot(id)).selected===mark,{id,mark:target.id});
    // Relative wheel gestures preserve unequal domain zoom rather than forcing
    // both axes back to the scalar surface baseline.
    point=await client(id,{x:p0.plot.x+p0.plot.width/2,y:p0.plot.y+p0.plot.height/2});
    await page.mouse.move(point.x,point.y);await page.mouse.wheel(0,-100);
    await page.waitForFunction(id=>JSON.parse(window.hostFixture.navigation_probe(id)).zoom[0]>4.1,id);
    let wheel=await probe(id);assert(Math.abs(wheel.zoom[0]/wheel.zoom[1]-2)<1e-8,`${id}: wheel collapsed unequal X/Y zoom`);
    const beforePan=await snapshot(id);await page.mouse.down();await page.mouse.move(point.x+12,point.y+7,{steps:4});await page.mouse.up();await flush(id);
    const afterPan=await snapshot(id);assert(afterPan.domains.x[0]<beforePan.domains.x[0]&&afterPan.domains.y[0]>beforePan.domains.y[0],`${id}: independent pan has wrong data direction`);
    const domains=JSON.stringify(afterPan.domains);const preInset=await probe(id);
    await page.evaluate(id=>window.hostFixture.navigation_inset(id,84),id);await flush(id);const inset=await probe(id);
    assert(inset.plot.x===84&&inset.plot.width===preInset.plot.width-28,`${id}: dynamic insets not applied`);
    assert(JSON.stringify((await snapshot(id)).domains)===domains,`${id}: dynamic inset changed data domain`);
    // Invalid dynamic geometry and callback errors are atomic.
    await page.evaluate(id=>{window.hostFixture.clear_events(id);window.hostFixture.navigation_inset(id,100000);},id);await flush(id);
    assert((await probe(id)).plot.x===84&&JSON.stringify((await snapshot(id)).domains)===domains,`${id}: invalid plot committed`);
    await page.evaluate(id=>{window.hostFixture.navigation_inset(id,84);window.hostFixture.navigation_reject(id,true);window.hostFixture.navigation_zoom(id,0,1.1);},id);await flush(id);
    assert(JSON.stringify((await snapshot(id)).domains)===domains,`${id}: callback failure committed independent domain`);
    const errors=await page.evaluate(id=>JSON.parse(window.hostFixture.events(id)).filter(e=>e.type==='error'),id);
    assert(errors.some(e=>e.message.includes('viewport plot'))&&errors.some(e=>e.message.includes('navigation fixture rejection')),`${id}: expected refusal events missing ${JSON.stringify(errors)}`);
    await page.evaluate(id=>{window.hostFixture.navigation_reject(id,false);window.hostFixture.reset_view(id);},id);await flush(id);
    const reset=await snapshot(id);assert(reset.domains.x[0]===0&&reset.domains.x[1]===(category?12:100)&&reset.domains.y[0]===0,`${id}: reset did not restore both original axes`);
    reports.push({renderer:id,category,independentZoom:true,currentPixelHit:true,wheelPreservesRatio:true,pan:true,dynamicInsets:true,invalidPlotAtomic:true,layoutErrorAtomic:true,reset:true});
  }
  const edgeCases=[];
  for(const id of ['svg','canvas']){
    // A dynamic plot callback may replace its own binding and then fail. Its
    // obsolete error/result must not reach the replacement controller.
    await page.evaluate(id=>window.hostFixture.remount_navigation(id,false),id);await flush(id);await wait(id);
    const accepted=(await snapshot(id)).domains;
    await page.evaluate(id=>{window.hostFixture.clear_events(id);window.hostFixture.navigation_replace_plot_then_fail(id);},id);
    await flush(id);await wait(id);
    const replacementEvents=await page.evaluate(id=>JSON.parse(window.hostFixture.events(id)),id);
    assert(replacementEvents.some(e=>e.type==='navigation-replacement-layout'),`${id}: replacement plot layout never ran`);
    assert(!replacementEvents.some(e=>e.type==='navigation-stale-layout'||e.type==='error'),`${id}: stale plot callback leaked ${JSON.stringify(replacementEvents)}`);
    assert(JSON.stringify((await snapshot(id)).domains)===JSON.stringify(accepted),`${id}: plot replacement changed accepted domains`);
    // Change the margin and emit a real DOM wheel hook in one JS task, so RAF
    // sees both. The event was measured in old pixels; it must use the old plot
    // before the responsive geometry is installed.
    await page.evaluate(id=>window.hostFixture.remount_navigation(id,false),id);await flush(id);await wait(id);
    const old=await snapshot(id),oldProbe=await probe(id);
    await page.locator(`#${id}-root .kartis-surface`).scrollIntoViewIfNeeded();
    const anchor=await page.evaluate(({id,p,s})=>{
      const surface=document.querySelector(`#${id}-root .kartis-surface`),box=surface.getBoundingClientRect();
      const clientX=Math.round(box.x+(p.x+p.width*.25)/s.width*box.width),clientY=Math.round(box.y+(p.y+p.height*.25)/s.height*box.height);
      const x=((clientX-box.x)*s.width/box.width-p.x)/p.width,y=((clientY-box.y)*s.height/box.height-p.y)/p.height;
      window.hostFixture.navigation_stage_inset(id,84);
      surface.dispatchEvent(new WheelEvent('wheel',{clientX,clientY,deltaY:-Math.log(2)/.002,bubbles:true,cancelable:true}));
      return {x,y};
    },{id,p:oldProbe.plot,s:old});
    await flush(id);await wait(id);
    const resized=await snapshot(id),newProbe=await probe(id);
    const expectedX=old.domains.x[0]+(old.domains.x[1]-old.domains.x[0])*anchor.x/2;
    const expectedY=old.domains.y[0]+(old.domains.y[1]-old.domains.y[0])*(1-anchor.y)/2;
    assert(Math.abs(resized.domains.x[0]-expectedX)<1e-8&&Math.abs(resized.domains.y[0]-expectedY)<1e-8,`${id}: dynamic margin reinterpreted old wheel pixels ${JSON.stringify({actual:resized.domains,expectedX,expectedY,anchor})}`);
    assert(newProbe.plot.x===84&&newProbe.plot.width===oldProbe.plot.width-28,`${id}: same-frame responsive plot not committed`);
    // One category can cap only X. A value-axis request and a native wheel
    // must continue to navigate Y while preserving the complete category.
    await page.evaluate(id=>window.hostFixture.remount_one_category(id),id);await flush(id);await wait(id);
    await page.evaluate(id=>window.hostFixture.navigation_zoom(id,1,2),id);await flush(id);await wait(id);
    const yOnly=await snapshot(id),one=await probe(id);
    assert(JSON.stringify(yOnly.domains.x)==='[0,1]'&&JSON.stringify(yOnly.domains.y)==='[25,75]',`${id}: one category froze Y ${JSON.stringify(yOnly.domains)}`);
    const center=await client(id,{x:one.plot.x+one.plot.width/2,y:one.plot.y+one.plot.height/2});
    await page.mouse.move(center.x,center.y);await page.mouse.wheel(0,-100);
    await page.waitForFunction(id=>JSON.parse(window.hostFixture.navigation_probe(id)).zoom[1]>2.1,id);
    const wheel=await snapshot(id),oneWheel=await probe(id);
    assert(JSON.stringify(wheel.domains.x)==='[0,1]'&&oneWheel.zoom[0]===1&&oneWheel.zoom[1]>2.1,`${id}: wheel coupled one-category cap to Y`);
    assert(!((await page.evaluate(id=>JSON.parse(window.hostFixture.events(id)),id)).some(e=>e.type==='error')),`${id}: edge navigation emitted an error`);
    edgeCases.push({renderer:id,obsoletePlotErrorSuppressed:true,sameFramePlotWheelUsesAcceptedPixels:true,oneCategoryValueAxisRequest:true,oneCategoryNativeWheel:true});
  }
  for(const id of ['svg','canvas']){await page.evaluate(id=>{window.hostFixture.remount(id);window.hostFixture.set_phase(id,1,false);},id);await flush(id);}
  return{passed:true,profiles:reports,edgeCases};
})
