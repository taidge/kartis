// Uses Chrome's real touch dispatch, producing primary and nonprimary PointerEvents.
(async page => {
  const assert = (ok, message) => { if (!ok) throw Error(message); };
  const snapshot = id => page.evaluate(id => JSON.parse(window.hostFixture.snapshot(id)), id);
  const wait = (id, condition) => page.waitForFunction(({id, condition}) =>
    new Function('s', `return (${condition})(s)`)(JSON.parse(window.hostFixture.snapshot(id))),
    {id, condition: condition.toString()});
  const reset = async id => {
    await page.evaluate(id => { window.hostFixture.reset_view(id); window.hostFixture.clear_events(id); }, id);
    await wait(id, s => s.pending === 0 && s.view.scale === 1 && s.touches === 0);
  };
  const probe = async id => {
    const el = page.locator(`#${id}-root .kartis-surface`);
    await el.scrollIntoViewIfNeeded();
    await el.evaluate(el => {
      el.__touches = [];
      el.__touchProbe = event => { if(event.pointerType === 'touch') el.__touches.push({id:event.pointerId, primary:event.isPrimary}); };
      el.addEventListener('pointerdown', el.__touchProbe);
    });
    return el;
  };
  const client = (box, x, y, id) => ({x:box.x+box.width*x, y:box.y+box.height*y, id, radiusX:2, radiusY:2, force:1});
  const cdp = await page.context().newCDPSession(page);
  await cdp.send('Emulation.setTouchEmulationEnabled', {enabled:true,maxTouchPoints:5});
  const send = (type, points=[]) => cdp.send('Input.dispatchTouchEvent', {type,touchPoints:points});
  const reports = [];
  try {
    for(const id of ['svg','canvas']) {
      await reset(id);
      const el = await probe(id), box = await el.boundingBox(), before = await snapshot(id);
      let a = client(box,.35,.5,11), b = client(box,.65,.5,22);
      await send('touchStart',[a,b]);
      await wait(id,s=>s.pinching&&s.touches===2&&s.capturedTouches===2);
      const contacts = await el.evaluate(el=>({events:el.__touches, captured:el.__touches.every(p=>el.hasPointerCapture(p.id))}));
      assert(contacts.captured&&contacts.events.length===2&&contacts.events[0].primary&&!contacts.events[1].primary,`${id}: second real nonprimary touch was not captured`);
      a=client(box,.2,.5,11);b=client(box,.8,.5,22);
      await send('touchMove',[a,b]);await wait(id,s=>Math.abs(s.view.scale-2)<.01);
      let zoomed=await snapshot(id);
      assert(Math.abs((before.width/2-zoomed.view.x)/zoomed.view.scale-before.width/2)<1,`${id}: pinch x anchor moved`);
      assert(Math.abs((before.height/2-zoomed.view.y)/zoomed.view.scale-before.height/2)<1,`${id}: pinch y anchor moved`);
      await send('touchEnd',[a]);await wait(id,s=>!s.pinching&&s.touches===1&&s.capturedTouches===1);
      // Chrome releases the supplied contact on partial touchEnd; b remains.
      const prior=await snapshot(id);b={...b,x:b.x+15};await send('touchMove',[b]);
      await page.waitForFunction(({id,x})=>Math.abs(JSON.parse(window.hostFixture.snapshot(id)).view.x-x)>5,{id,x:prior.view.x});
      zoomed=await snapshot(id);
      assert(Math.abs(zoomed.view.x-prior.view.x-15*prior.width/box.width)<1,`${id}: pinch-to-pan jumped ${JSON.stringify({before:prior.view,after:zoomed.view,width:prior.width,box,expected:15*prior.width/box.width})}`);
      await send('touchEnd');await wait(id,s=>s.touches===0&&s.capturedTouches===0);
      const log=await page.evaluate(id=>JSON.parse(window.hostFixture.events(id)),id);
      assert(!log.some(e=>e.type==='activated'),`${id}: pinch produced a synthetic tap`);
      await reset(id);
      a=client(box,.35,.5,11);b=client(box,.65,.5,22);
      await send('touchStart',[a,b]);await wait(id,s=>s.touches===2);
      await send('touchStart',[a,b,client(box,.5,.3,33)]);await wait(id,s=>s.pending===0);
      assert((await snapshot(id)).touches===2,`${id}: third touch replaced a contact`);
      await send('touchCancel');await wait(id,s=>s.touches===0&&s.capturedTouches===0&&!s.pinching);
      await reset(id);
      a=client(box,.44,.425,31);await send('touchStart',[a]);await wait(id,s=>s.touches===1);
      await send('touchEnd');await wait(id,s=>s.selected==='datum-alpha'&&s.touches===0);
      await el.evaluate(el=>el.removeEventListener('pointerdown',el.__touchProbe));
      reports.push({renderer:id,nonprimaryCapture:true,anchor:true,pinchToPan:true,noSyntheticTap:true,thirdTouchIgnored:true,cancel:true,singleTap:true});
    }
    // Resize must retire contacts even with no layout callback or a failed layout.
    for(const mode of [0,2]) {
      await page.evaluate(mode=>window.hostFixture.remount_layout_profile('svg',mode),mode);
      await wait('svg',s=>s.active&&s.revision>0&&s.pending===0);
      const el=await probe('svg'),box=await el.boundingBox(),old=await snapshot('svg');
      await send('touchStart',[client(box,.35,.5,11),client(box,.65,.5,22)]);await wait('svg',s=>s.pinching&&s.touches===2);
      await page.locator('#svg-root').evaluate(el=>el.style.width='75%');
      await wait('svg',s=>s.touches===0&&s.capturedTouches===0&&!s.pinching);
      const state=await snapshot('svg');assert(state.width===old.width,`Resize profile ${mode}: rejected/static layout changed scene`);
      assert(await el.evaluate(el=>el.__touches.every(p=>!el.hasPointerCapture(p.id))),`Resize profile ${mode}: capture retained`);
      await send('touchCancel');
      await page.locator('#svg-root').evaluate(el=>el.style.width='');
    }
    // Hiding the host also retires baselines without needing pointer geometry.
    await page.evaluate(()=>window.hostFixture.remount('svg'));await wait('svg',s=>s.active&&s.pending===0&&s.revision>0);
    const el=await probe('svg'),box=await el.boundingBox();
    await send('touchStart',[client(box,.35,.5,11),client(box,.65,.5,22)]);await wait('svg',s=>s.pinching);
    await page.locator('#svg-root').evaluate(el=>el.style.display='none');await wait('svg',s=>s.touches===0&&s.capturedTouches===0);
    await send('touchCancel');await page.locator('#svg-root').evaluate(el=>el.style.display='');
    await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
    await wait('svg',s=>s.pending===0);
    for(const [id,action] of [['svg','stop'],['canvas','drop_chart']]) {
      const surface=await probe(id),element=await surface.elementHandle(),rect=await surface.boundingBox();
      await send('touchStart',[client(rect,.35,.5,11),client(rect,.65,.5,22)]);await wait(id,s=>s.pinching&&s.capturedTouches===2);
      await page.evaluate(({id,action})=>window.hostFixture[action](id),{id,action});await wait(id,s=>!s.active);
      assert(await element.evaluate(el=>el.__touches.every(p=>!el.hasPointerCapture(p.id))),`${id}: active gesture capture survived ${action}`);
      await send('touchCancel');await page.evaluate(id=>window.hostFixture.remount(id),id);await wait(id,s=>s.active&&s.pending===0&&s.revision>0);
      await page.evaluate(id=>{window.hostFixture.set_phase(id,1,false);window.hostFixture.flush(id);},id);
    }
    return {renderers:reports,resizeWithoutLayout:true,resizeRejectedLayout:true,hiddenHostCleanup:true,activeStopCleanup:true,activeDropCleanup:true};
  } finally {
    await send('touchCancel').catch(()=>{});
    await cdp.send('Emulation.setTouchEmulationEnabled',{enabled:false});
    await cdp.detach();
  }
})
