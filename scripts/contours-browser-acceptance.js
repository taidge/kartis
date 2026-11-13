(async page => {
  const assert = (ok, message) => { if (!ok) throw Error(message); };
  const reports = [];
  await page.emulateMedia({ reducedMotion: 'no-preference' });
  const settled = id => page.waitForFunction(id => {
    const s = JSON.parse(window.hostFixture.contours_snapshot(id));
    return s.active && !s.animating && s.pending === 0;
  }, id);
  const pixels = id => page.evaluate(async id => {
    const state = JSON.parse(window.hostFixture.contours_snapshot(id));
    const root = document.getElementById(`${id}-root`);
    let canvas = root.querySelector('canvas');
    if (!canvas) {
      const liveSvg = root.querySelector('svg'), svg = liveSvg.cloneNode(true);
      // Standalone SVG images do not retain the live tree's inherited language.
      const languageOwner=liveSvg.closest('[lang]');
      let language;
      for(let node=liveSvg;node;node=node.parentElement){
        const xmlLanguage=node.getAttributeNS('http://www.w3.org/XML/1998/namespace','lang')??node.getAttribute('xml:lang');
        if(xmlLanguage!==null){language=xmlLanguage;break;}
        if(node===languageOwner){language=node.getAttribute('lang');break;}
      }
      svg.setAttribute('lang',language??liveSvg.ownerDocument.documentElement.lang);
      // Percentage dimensions have engine-specific standalone image intrinsic
      // sizes. Rasterize the accepted logical viewport explicitly.
      svg.setAttribute('width',String(state.width));
      svg.setAttribute('height',String(state.height));
      const image = new Image();
      await new Promise((resolve,reject) => { image.onload=resolve; image.onerror=reject; image.src=`data:image/svg+xml;charset=utf-8,${encodeURIComponent(new XMLSerializer().serializeToString(svg))}`; });
      canvas = document.createElement('canvas');
      canvas.width = Math.ceil(state.width); canvas.height = Math.ceil(state.height);
      canvas.getContext('2d').drawImage(image,0,0,state.width,state.height);
    }
    const context=canvas.getContext('2d');
    const pixel=(x,y)=>[...context.getImageData(Math.floor(x*canvas.width/state.width),Math.floor(y*canvas.height/state.height),1,1).data];
    return { hole:pixel(state.width*.5,state.height*.5),ring:pixel(state.width*.5+50,state.height*.5) };
  }, id);
  for (const id of ['svg','canvas']) {
    await page.evaluate(id => window.hostFixture.contours_mount(id,false),id);
    await settled(id);
    assert((await pixels(id)).hole[0]===30,`${id}: initial single contour was not solid`);
    await page.evaluate(id => window.hostFixture.contours_phase(id,1),id);
    const forward=await page.evaluate(async id => {
      let sampled=false,properties=false,oneMark=true;const revisions=new Set();
      for(let n=0;n<400;n++) {
        await new Promise(requestAnimationFrame);
        const s=JSON.parse(window.hostFixture.contours_snapshot(id));revisions.add(s.revision);
        properties ||= s.kind.includes('Properties');
        if(id==='svg') {
          const paths=document.querySelectorAll('#svg-root svg path[data-id="donut"]');
          oneMark &&= paths.length===1&&!document.querySelector('#svg-root svg path[data-id*="ghost"]');
          const d=paths[0]?.getAttribute('d')||'';
          sampled ||= (d.match(/M/g)||[]).length===2&&(d.match(/L/g)||[]).length===190;
        }
        if(n>8&&!s.animating&&s.pending===0) return {frames:n,revisions:revisions.size,sampled,properties,oneMark};
      }
      throw Error('Multi-contour RAF failed to settle');
    },id);
    assert(forward.revisions>8&&forward.properties&&forward.oneMark,`${id}: no actual PropertyMotion RAF frames`);
    if(id==='svg') assert(forward.sampled,'SVG did not render two real 96-point contours');
    const ring=await pixels(id);
    assert(ring.hole.join(',')==='255,255,255,255'&&ring.ring.join(',')==='30,90,160,255',`${id}: winding hole pixels failed`);
    // A completed reordered target must retain its accepted bindings after the
    // running engine is retired. Ordinary animate_to now uses full slot order.
    await page.evaluate(id => window.hostFixture.contours_phase(id,2),id);
    await settled(id);
    await page.evaluate(id => window.hostFixture.contours_phase(id,3),id);
    await page.waitForFunction(id=>JSON.parse(window.hostFixture.contours_snapshot(id)).animating,id);
    await page.evaluate(id => window.hostFixture.contours_phase(id,1),id);
    await page.emulateMedia({reducedMotion:'reduce'});
    await settled(id);
    if(id==='svg') {
      const d=await page.locator('#svg-root svg path[data-id="donut"]').getAttribute('d');
      assert((d.match(/M/g)||[]).length===2&&(d.match(/C/g)||[]).length===8&&!d.includes('L'),'Reduced motion did not restore original cubic contours');
      const numbers=(d.match(/^M([^ ]+) ([^C]+)/)||[]);
      const s=await page.evaluate(id=>JSON.parse(window.hostFixture.contours_snapshot(id)),id);
      assert(Math.abs(Number(numbers[1])-(s.width*.5+30))<.001,'Exact target did not retain hole-first contour order');
    }
    assert((await pixels(id)).hole[0]===255,`${id}: reduced-motion target lost its hole`);
    const before=await page.evaluate(id=>JSON.parse(window.hostFixture.contours_snapshot(id)),id);
    assert(await page.evaluate(id=>window.hostFixture.contours_bad_binding_rejected(id),id),`${id}: invalid mapping was accepted`);
    const after=await page.evaluate(id=>JSON.parse(window.hostFixture.contours_snapshot(id)),id);
    assert(before.revision===after.revision&&after.pending===0,`${id}: bad mapping changed accepted state`);
    await page.emulateMedia({reducedMotion:'no-preference'});
    // MediaQueryList change is a queued native input. Let both the media event
    // and its RAF commit run before requesting another animated target.
    await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
    await page.evaluate(id=>window.hostFixture.contours_phase(id,3),id);
    await settled(id);
    assert((await pixels(id)).hole[0]===30,`${id}: disappearing hole did not restore solid fill`);
    await page.evaluate(id=>window.hostFixture.contours_phase(id,1),id);
    await page.waitForFunction(id=>JSON.parse(window.hostFixture.contours_snapshot(id)).animating,id);
    await page.evaluate(id=>window.hostFixture.contours_drop(id),id);
    await page.waitForTimeout(80);
    const dropped=await page.evaluate(id=>({snapshot:JSON.parse(window.hostFixture.contours_snapshot(id)),children:document.getElementById(`${id}-root`).children.length}),id);
    assert(!dropped.snapshot.active&&dropped.children===0,`${id}: dropping active morph retained its host`);
    await page.evaluate(id=>window.hostFixture.contours_mount(id,true),id);
    const sceneRejected=await page.evaluate(id=>{try { window.hostFixture.contours_phase(id,1);return false; } catch {return true;}},id);
    assert(sceneRejected,`${id}: Scene animation silently accepted explicit contour bindings`);
    const errors=await page.evaluate(id=>JSON.parse(window.hostFixture.contours_snapshot(id)).errors,id);
    assert(errors===0,`${id}: unexpected host errors`);
    await page.evaluate(id=>{window.hostFixture.contours_drop(id);window.hostFixture.remount(id);},id);
    reports.push({renderer:id,forward,pixels:ring,retarget:true,exactReducedTarget:true,badBindingAtomic:true,sceneRejectsBindings:true,activeDrop:true});
  }
  return {passed:true,renderers:reports};
})
