(async function verifyDelaunay(page) {
  const assert=(ok,message)=>{if(!ok)throw Error(message);};
  const errors=[],onError=error=>errors.push(error.message);page.on('pageerror',onError);
  const flush=async()=>{await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));await page.evaluate(()=>window.hostFixture.delaunay_flush());};
  const snapshot=renderer=>page.evaluate(renderer=>JSON.parse(window.hostFixture.delaunay_snapshot(renderer)),renderer);
  const triangleId=triangle=>'delaunay:triangle:'+triangle.map(id=>`${new TextEncoder().encode(id).length}:${id}`).join(':');
  const previous=await page.evaluate(()=>{const panels=document.querySelector('.panels'),previous=panels.style.display;panels.style.display='none';return previous;});
  const baseline=await page.locator('body *').count(),reports=[];
  const pixels=()=>page.evaluate(async()=>{
    const source=document.querySelector('#delaunay-svg svg').cloneNode(true);source.setAttribute('width','640');source.setAttribute('height','360');source.setAttributeNS('http://www.w3.org/XML/1998/namespace','xml:lang',document.documentElement.lang||navigator.language);
    const image=new Image();image.src=`data:image/svg+xml;charset=utf-8,${encodeURIComponent(new XMLSerializer().serializeToString(source))}`;await image.decode();
    const live=document.querySelector('#delaunay-canvas canvas'),copy=document.createElement('canvas');copy.width=live.width;copy.height=live.height;copy.getContext('2d').drawImage(image,0,0,copy.width,copy.height);
    const a=copy.getContext('2d').getImageData(0,0,copy.width,copy.height).data,b=live.getContext('2d').getImageData(0,0,live.width,live.height).data;let diff=0,ink=0;for(let i=0;i<a.length;i++)diff+=Math.abs(a[i]-b[i]);for(let i=0;i<b.length;i+=4)ink+=Number(Math.min(b[i],b[i+1],b[i+2])<220);
    return{mean:diff/a.length,ink,backing:[live.width,live.height]};
  });
  function certify(state) {
    const sites=new Map(state.sites.map(s=>[s.id,s])),links=new Set(state.links.map(e=>JSON.stringify(e))),orientation=(a,b,c)=>(b.x-a.x)*(c.y-a.y)-(b.y-a.y)*(c.x-a.x);
    const circle=(a,b,c,d)=>{const ax=a.x-d.x,ay=a.y-d.y,bx=b.x-d.x,by=b.y-d.y,cx=c.x-d.x,cy=c.y-d.y;return(ax*ax+ay*ay)*(bx*cy-by*cx)-(bx*bx+by*by)*(ax*cy-ay*cx)+(cx*cx+cy*cy)*(ax*by-ay*bx);};
    assert(links.size===state.links.length,'Delaunay repeated undirected edge');
    for(const t of state.triangles){const[a,b,c]=t.map(id=>sites.get(id));assert(orientation(a,b,c)>0,'Delaunay triangle winding/area invalid');for(const p of state.sites)if(!t.includes(p.id))assert(circle(a,b,c,p)<=0,'Delaunay empty circumcircle violated');for(let i=0;i<3;i++){const edge=[t[i],t[(i+1)%3]].sort();assert(links.has(JSON.stringify(edge)),'Triangle side missing in adjacency');}}
    if(state.triangles.length)assert(state.sites.length+state.triangles.length===state.links.length+1,'Delaunay Euler identity failed');
    const expected=state.links.filter(edge=>edge.includes(state.target.raw_id)).map(edge=>edge.find(id=>id!==state.target.raw_id)).sort();assert(JSON.stringify(expected)===JSON.stringify(state.target.neighbors),'Current neighbor metadata disagrees with links');assert(state.target.value===expected.length,'Current site value must be its degree');
  }
  try {
    for(let test=0;test<3;test++) {
      await page.mouse.move(2,2);await page.evaluate(test=>window.hostFixture.delaunay_mount(test),test);await flush();
      const initial=await snapshot('svg');certify(initial);
      if(test===1)assert(initial.raw.length===initial.sites.length+1&&initial.raw.find(p=>p.id==='zz-duplicate').owner==='p0','Exact duplicate raw ID did not retain canonical ownership');
      if(test===2)assert(initial.triangles.length===0&&initial.links.length===4&&initial.hull.length===5,'Collinear sites were jittered into triangles or lost chain adjacency');
      const phases=[];
      for(let phase=0;phase<2;phase++) {
        let shared=[];
        if(phase===1) {
          await page.evaluate(id=>{window.oldDelaunaySite=document.querySelector(`#delaunay-svg [data-id="${CSS.escape(id)}"]`);window.oldDelaunayTriangles=new Map([...document.querySelectorAll('#delaunay-svg [data-id^="delaunay:triangle:"]')].map(node=>[node.getAttribute('data-id'),node]));},initial.target.id);
          await page.evaluate(()=>window.hostFixture.delaunay_phase(1));await flush();
          const changed=await snapshot('svg');shared=changed.triangles.map(triangleId).filter(id=>initial.triangles.map(triangleId).includes(id));
          assert(await page.evaluate(({id,shared})=>window.oldDelaunaySite===document.querySelector(`#delaunay-svg [data-id="${CSS.escape(id)}"]`)&&shared.every(id=>window.oldDelaunayTriangles.get(id)===document.querySelector(`#delaunay-svg [data-id="${CSS.escape(id)}"]`)),{id:initial.target.id,shared}),'Stable site/triangle identities were needlessly recreated');
          assert(changed.target.label!==initial.target.label,'Updated tooltip retained old source coordinates');
          if(test===2)assert(changed.triangles.length>0,'Collinear → noncollinear update retained stale topology');
        }
        const state=await snapshot('svg');certify(state);const image=await pixels();assert(image.mean<.5&&image.ink>700,`Delaunay ${test}/${phase} actual SVG/Canvas ink differs ${JSON.stringify(image)}`);
        const geometry=await page.evaluate(({state,triangleIds})=>{const svg=document.querySelector('#delaunay-svg svg'),circle=svg.querySelector(`[data-id="${CSS.escape(state.target.id)}"]`),ids=[...svg.querySelectorAll('[data-id]')].map(n=>n.getAttribute('data-id'));return{circles:svg.querySelectorAll('path[data-id^="delaunay:site:"]').length,triangles:svg.querySelectorAll('[data-id^="delaunay:triangle:"]').length,links:svg.querySelectorAll('[data-id^="delaunay:link:"]').length,unique:new Set(ids).size===ids.length,targetPath:circle.getAttribute('d'),targetStart:circle.getAttribute('d').match(/^M([^ ]+) ([^C]+)/).slice(1).map(Number),targetCubics:(circle.getAttribute('d').match(/C/g)||[]).length,trianglesReal:triangleIds.every(id=>{const path=svg.querySelector(`[data-id="${CSS.escape(id)}"]`);return path&&(path.getAttribute('d').match(/L/g)||[]).length===2&&path.getAttribute('d').includes('Z');}),safe:svg.querySelectorAll('script').length===0&&[...svg.querySelectorAll('text')].some(n=>n.textContent===state.target.raw_id)};},{state,triangleIds:state.triangles.map(triangleId)});
        assert(geometry.unique&&geometry.safe&&geometry.trianglesReal&&geometry.circles===state.sites.length&&geometry.links===state.links.length&&geometry.triangles===state.triangles.length&&geometry.targetStart[0]===state.target.x+7&&geometry.targetStart[1]===state.target.y&&geometry.targetCubics===4,`Delaunay actual topology/labels geometry failed ${JSON.stringify(geometry)}`);
        for(const renderer of ['svg','canvas']) {
          const current=await snapshot(renderer),surface=page.locator(`#delaunay-${renderer} .kartis-surface`);await surface.scrollIntoViewIfNeeded();const box=await surface.boundingBox(),x=box.x+current.target.x/current.width*box.width,y=box.y+current.target.y/current.height*box.height;
          await page.mouse.move(x,y);await flush();assert((await snapshot(renderer)).hovered.includes(current.target.id),`${renderer} Delaunay ${test}/${phase}: current site missed`);assert((await page.locator(`#delaunay-${renderer} .kartis-surface-tooltip`).textContent()).includes(current.target.label),`${renderer} current source coordinates/degree tooltip missing`);
          await page.mouse.click(x,y);await flush();assert((await snapshot(renderer)).selected===current.target.id,`${renderer} Delaunay current site selection failed`);
          await page.mouse.move(box.x+4,box.y+4);await flush();assert((await snapshot(renderer)).hovered.length===0,`${renderer} Delaunay empty plot margin should miss`);
          const activationSequence=(await snapshot(renderer)).events.at(-1)?.sequence||0;const control=page.locator(`#delaunay-${renderer} .kartis-datum`).filter({hasText:'target <E>'}).first();await control.focus();await page.keyboard.press('Enter');await flush();assert((await snapshot(renderer)).events.some(e=>e.type==='activated'&&e.id===current.target.id&&e.sequence>activationSequence),`${renderer} Delaunay keyboard activation missing`);
          const datumCount=await page.locator(`#delaunay-${renderer} .kartis-datum`).count();assert(datumCount===current.sites.length,`${renderer} Delaunay ${test}/${phase}: expected ${current.sites.length} canonical site controls, got ${datumCount}; decorations/duplicates must not fabricate data`);
          const legend=page.locator(`#delaunay-${renderer} .kartis-legend-item`).filter({hasText:'Sites'}).first();await legend.click();await flush();assert((await snapshot(renderer)).hidden.includes('sites'),`${renderer} Delaunay Sites legend did not hide`);
          if(renderer==='svg')assert(await page.evaluate(()=>{const svg=document.querySelector('#delaunay-svg svg');return !svg.querySelector('[data-id^="delaunay:site:"]')&&!svg.querySelector('text')&&svg.querySelector('[data-id^="delaunay:link:"]');}),'Delaunay hidden sites retained owned labels or removed independent links');
          await legend.click();await flush();assert(!(await snapshot(renderer)).hidden.includes('sites'),'Delaunay legend failed to restore sites');
          await surface.focus();await page.keyboard.press('Escape');await page.evaluate(()=>document.activeElement?.blur());await page.mouse.move(2,2);await flush();
        }
        await page.evaluate(()=>{window.delaunayMutations=[];window.delaunayObserver=new MutationObserver(records=>window.delaunayMutations.push(...records));window.delaunayObserver.observe(document.querySelector('#delaunay-svg svg'),{subtree:true,attributes:true,childList:true,characterData:true});});
        await page.evaluate(phase=>window.hostFixture.delaunay_phase(phase),phase);await flush();const mutations=await page.evaluate(()=>{const count=window.delaunayMutations.length+window.delaunayObserver.takeRecords().length;window.delaunayObserver.disconnect();return count;});assert(mutations===0,`Identical Delaunay candidate mutated retained SVG ${mutations} times`);
        const before=await snapshot('svg'),markup=await page.locator('#delaunay-svg svg').innerHTML();assert(await page.evaluate(()=>window.hostFixture.delaunay_rejected()),'Delaunay accepted invalid graph/scene');const rejectedState=await snapshot('svg');assert(rejectedState.revision===before.revision&&rejectedState.pending===before.pending,`Delaunay ${test}/${phase}: rejected input mutated revision or queue before explicit flush`);await flush();assert(await page.locator('#delaunay-svg svg').innerHTML()===markup,`Delaunay ${test}/${phase}: rejected candidate changed accepted SVG`);
        phases.push({phase,pixels:image,geometry,rawSites:state.raw.length,canonicalSites:state.sites.length,triangles:state.triangles.length,links:state.links.length,sharedRetainedTriangles:shared.length,currentHit:true,currentTooltip:true,keyboard:true,legend:true,zeroMutation:true,rejectedAtomic:true});
      }
      await page.evaluate(()=>{window.oldDelaunayControl=document.querySelector('#delaunay-svg .kartis-datum');window.hostFixture.delaunay_stop();});assert(await page.locator('body *').count()===baseline,'Delaunay cleanup leaked owned nodes');await page.evaluate(()=>window.oldDelaunayControl?.click());await flush();assert(!(await snapshot('svg')).active,'Detached Delaunay control revived stopped chart');reports.push({case:test,phases,cleanup:true});
    }
    assert(errors.length===0,errors.join('\n'));return{passed:true,cases:reports,pageErrors:errors};
  } finally {
    await page.evaluate(previous=>{window.hostFixture.delaunay_stop();document.querySelector('.panels').style.display=previous;window.delaunayObserver?.disconnect();delete window.delaunayObserver;delete window.delaunayMutations;delete window.oldDelaunayControl;delete window.oldDelaunaySite;delete window.oldDelaunayTriangles;},previous);page.off('pageerror',onError);
  }
})
