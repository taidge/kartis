(async page=>{
  const assert=(ok,msg)=>{if(!ok)throw Error(msg);};
  const reports=[];
  await page.emulateMedia({reducedMotion:'no-preference'});
  const mediaQueue=[];
  for(const id of ['svg','canvas']){
    await page.emulateMedia({reducedMotion:'reduce'});
    await page.evaluate(id=>window.hostFixture.remount(id),id);
    await page.waitForFunction(id=>{const s=JSON.parse(window.hostFixture.snapshot(id));return s.pending===0&&!s.animating;},id);
    // Establish the initial reduced policy before freezing controller RAFs;
    // the actual three changes under saturation below remain deliberately held.
    await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
    await page.waitForFunction(id=>JSON.parse(window.hostFixture.snapshot(id)).pending===0,id);
    const saturated=await page.evaluate(id=>{
      window.fixtureOriginalRaf=window.requestAnimationFrame;
      // Hold controller frames while allowing the browser's actual media
      // change event to run. flush() will consume the held ticket afterward.
      window.requestAnimationFrame=()=>window.fixtureOriginalRaf(()=>{});
      window.fixtureMedia=matchMedia('(prefers-reduced-motion: reduce)');
      window.fixtureMediaChanges=[];
      window.fixtureMediaListener=e=>window.fixtureMediaChanges.push(e.matches);
      window.fixtureMedia.addEventListener('change',window.fixtureMediaListener);
      return JSON.parse(window.hostFixture.overflow_actions(id));
    },id);
    assert(saturated.accepted===256&&saturated.rejected===256&&saturated.pending===256,`${id}: media regression did not saturate action budget: ${JSON.stringify(saturated)}`);
    let preferencePending;
    try{
      for(const [count,preference] of [[1,'no-preference'],[2,'reduce'],[3,'no-preference']]){
        await page.emulateMedia({reducedMotion:preference});
        await page.waitForFunction(({count,reduced})=>window.fixtureMediaChanges.length>=count&&window.fixtureMediaChanges.at(-1)===reduced,{count,reduced:preference==='reduce'});
      }
      preferencePending=await page.evaluate(id=>JSON.parse(window.hostFixture.snapshot(id)),id);
      assert(preferencePending.pending===257,`${id}: latest system preference was lost or consumed an action slot`);
      assert(preferencePending.dropped===saturated.rejected,`${id}: authoritative media changes were dropped at a full action queue`);
    }finally{
      await page.evaluate(()=>{
        window.requestAnimationFrame=window.fixtureOriginalRaf;
        window.fixtureMedia.removeEventListener('change',window.fixtureMediaListener);
        delete window.fixtureMedia;delete window.fixtureMediaListener;delete window.fixtureOriginalRaf;
      });
    }
    await page.evaluate(id=>{window.hostFixture.flush(id);window.hostFixture.set_phase(id,1,true);},id);
    const animated=await page.evaluate(async id=>{
      const revisions=new Set();let properties=false;
      for(let n=0;n<400;n++){
        await new Promise(requestAnimationFrame);
        const s=JSON.parse(window.hostFixture.snapshot(id));revisions.add(s.revision);properties||=s.animationKind==='Properties';
        if(n>8&&!s.animating&&s.pending===0)return {frames:n,revisions:revisions.size,properties};
      }
      throw Error('Media configuration queue recovery never settled');
    },id);
    assert(animated.properties&&animated.revisions>5,`${id}: false media preference did not resume actual multi-frame property motion`);
    assert((await page.locator(`#${id}-root .kartis-datum`).first().textContent()).includes('alpha: 40'),`${id}: resumed motion did not reach exact target`);
    mediaQueue.push({renderer:id,coalescedChanges:3,actionSlots:256,pendingWithPreference:preferencePending.pending,animated});
  }
  for(const id of ['svg','canvas']){
    await page.evaluate(id=>window.hostFixture.remount_morph(id),id);
    await page.waitForFunction(id=>{const s=JSON.parse(window.hostFixture.snapshot(id));return s.active&&s.pending===0;},id);
    await page.evaluate(id=>window.hostFixture.morph_phase(id,1),id);
    const forward=await page.evaluate(async id=>{
      const seen=new Set();let sampled=false,oneMark=true;
      for(let n=0;n<400;n++){
        await new Promise(requestAnimationFrame);const s=JSON.parse(window.hostFixture.snapshot(id));seen.add(s.revision);
        if(id==='svg'){
          const paths=document.querySelectorAll('#svg-root svg path[data-id="datum-alpha"]');
          oneMark&&=paths.length===1&&!document.querySelector('#svg-root svg path[data-id*="ghost"]');
          sampled||=(paths[0]?.getAttribute('d').match(/L/g)||[]).length===47;
        }
        if(n>8&&!s.animating&&s.pending===0)return{frames:n,revisions:seen.size,sampled,oneMark};
      }
      throw Error('Morph did not settle');
    },id);
    assert(forward.revisions>8&&forward.oneMark,`${id}: morph failed to produce independent RAF frames`);
    if(id==='svg')assert(forward.sampled,'SVG morph did not render actual 48-point contour');
    assert((await page.locator(`#${id}-root .kartis-datum`).first().textContent()).includes('alpha: 40'),`${id}: morph target datum is stale`);
    await page.evaluate(id=>window.hostFixture.morph_phase(id,0),id);
    await page.waitForFunction(id=>JSON.parse(window.hostFixture.snapshot(id)).animating,id);
    // Retarget during a sampled trajectory, then reduce motion. The exact
    // circle target must replace the approximation and release the RAF.
    await page.evaluate(id=>window.hostFixture.morph_phase(id,1),id);
    await page.emulateMedia({reducedMotion:'reduce'});
    await page.waitForFunction(id=>{const s=JSON.parse(window.hostFixture.snapshot(id));return !s.animating&&s.pending===0;},id);
    if(id==='svg'){
      const d=await page.locator('#svg-root svg path[data-id="datum-alpha"]').getAttribute('d');
      assert(d.includes('C')&&!d.includes('L'),'Settled SVG morph did not restore exact circle geometry');
    }else{
      const ink=await page.locator('#canvas-root canvas').evaluate(canvas=>{const p=canvas.getContext('2d').getImageData(Math.floor(canvas.width*.44),Math.floor(canvas.height*.425),1,1).data;return p[2]>100&&p[0]<100;});
      assert(ink,'Canvas morph target did not paint the actual circle center');
    }
    reports.push({renderer:id,forward,retarget:true,reducedMotionExactTarget:true});
    await page.evaluate(id=>{window.hostFixture.remount(id);window.hostFixture.set_phase(id,1,false);},id);
    await page.waitForFunction(id=>JSON.parse(window.hostFixture.snapshot(id)).pending===0,id);
    await page.emulateMedia({reducedMotion:'no-preference'});
  }
  return{passed:true,mediaPreferenceQueue:mediaQueue,renderers:reports};
})
