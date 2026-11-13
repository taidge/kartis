// Data-space focus: native host state remains at the identity camera transform.
(async page=>{
  const assert=(ok,message)=>{if(!ok)throw Error(message);},errors=[],onError=e=>errors.push(e.message);page.on('pageerror',onError);
  const flush=async()=>{await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));await page.evaluate(()=>window.hostFixture.hierarchy_flush());};
  const snapshot=renderer=>page.evaluate(renderer=>JSON.parse(window.hostFixture.hierarchy_snapshot(renderer)),renderer);
  const action=async command=>{for(const renderer of ['svg','canvas'])await page.evaluate(({renderer,command})=>window.hostFixture.hierarchy_action(renderer,command),{renderer,command});await flush();};
  const previous=await page.evaluate(()=>{const p=document.querySelector('.panels'),old=p.style.display;p.style.display='none';return old;});
  const baseline=await page.locator('body *').count(),reports=[];
  const compare=()=>page.evaluate(async()=>{
    const liveSvg=document.querySelector('#hierarchy-svg svg'),source=liveSvg.cloneNode(true),live=document.querySelector('#hierarchy-canvas canvas');
    const languageOwner=liveSvg.closest('[lang]');let language;for(let node=liveSvg;node;node=node.parentElement){const xml=node.getAttributeNS('http://www.w3.org/XML/1998/namespace','lang')??node.getAttribute('xml:lang');if(xml!==null){language=xml;break;}if(node===languageOwner){language=node.getAttribute('lang');break;}}
    source.setAttribute('lang',language??liveSvg.ownerDocument.documentElement.lang);source.setAttribute('width',String(live.width));source.setAttribute('height',String(live.height));
    const image=new Image();image.src='data:image/svg+xml;charset=utf-8,'+encodeURIComponent(new XMLSerializer().serializeToString(source));await image.decode();const copy=document.createElement('canvas');copy.width=live.width;copy.height=live.height;const ctx=copy.getContext('2d');ctx.drawImage(image,0,0);
    const a=ctx.getImageData(0,0,copy.width,copy.height).data,b=live.getContext('2d').getImageData(0,0,live.width,live.height).data;let difference=0,ink=0;for(let i=0;i<a.length;i++)difference+=Math.abs(a[i]-b[i]);for(let i=0;i<b.length;i+=4)ink+=Number(Math.min(b[i],b[i+1],b[i+2])<220);
    return{meanChannelError:difference/a.length,ink,backing:[live.width,live.height]};
  });
  const pixels=async label=>{const p=await compare();assert(p.meanChannelError<.5&&p.ink>1500,`${label} SVG/Canvas parity ${JSON.stringify(p)}`);return p;};
  const neutral=async()=>{await page.mouse.move(2,2);for(const renderer of ['svg','canvas']){await page.locator(`#hierarchy-${renderer} .kartis-surface`).focus();await page.keyboard.press('Escape');}await page.evaluate(()=>document.activeElement?.blur());await flush();};
  const sameAndRejected=async()=>{
    await neutral();await page.evaluate(()=>{const root=document.querySelector('#hierarchy-svg svg');window.hierarchyProbe.observer=new MutationObserver(records=>window.hierarchyProbe.mutations+=records.length);window.hierarchyProbe.mutations=0;window.hierarchyProbe.observer.observe(root,{subtree:true,attributes:true,childList:true,characterData:true});});
    await action('same');assert(await page.evaluate(()=>window.hierarchyProbe.mutations===0),'Hierarchy identical accepted layout mutated DOM');await page.evaluate(()=>window.hierarchyProbe.observer.disconnect());
    const accepted=await snapshot('svg'),markup=await page.locator('#hierarchy-svg svg').first().innerHTML();assert(await page.evaluate(()=>window.hostFixture.hierarchy_rejected()),'Bad hierarchy layout/Scene accepted');const after=await snapshot('svg');assert(after.revision===accepted.revision&&after.controllerRevision===accepted.controllerRevision&&after.focus===accepted.focus&&after.pending===0,'Rejected hierarchy changed accepted controller/queue '+JSON.stringify({accepted,after}));assert(await page.locator('#hierarchy-svg svg').first().innerHTML()===markup,'Rejected hierarchy changed accepted DOM');await flush();assert(await page.locator('#hierarchy-svg svg').first().innerHTML()===markup,'Rejected hierarchy changed DOM on the next frame');
  };
  const animatedFocus=async()=>{
    await page.emulateMedia({reducedMotion:'no-preference'});
    await page.evaluate(()=>window.hostFixture.hierarchy_mount(1));await flush();
    const initial=await page.evaluate(()=>{
      const svg=document.querySelector('#hierarchy-svg svg');
      const path=svg.querySelector('path[data-id="node:leaf-a"]');
      const canvas=document.querySelector('#hierarchy-canvas canvas');
      const canvasSignature=()=>{
        const data=canvas.getContext('2d').getImageData(0,0,canvas.width,canvas.height).data;
        let signature=0;
        for(let i=0;i<data.length;i+=28)signature=Math.imul(signature,33)^(data[i]<<16|data[i+1]<<8|data[i+2]);
        return signature;
      };
      window.hierarchyMotionProbe={svg,path,canvasSignature};
      return {width:path.getBBox().width,d:path.getAttribute('d'),canvas:canvasSignature()};
    });
    const started=await page.evaluate(()=>{
      for(const renderer of ['svg','canvas'])window.hostFixture.hierarchy_action(renderer,'group-a');
      return ['svg','canvas'].map(renderer=>JSON.parse(window.hostFixture.hierarchy_snapshot(renderer)));
    });
    assert(started.every(s=>s.animating&&s.propertiesMotion&&s.focus==='group-a'&&s.view.scale===1),'Hierarchy focus did not start property motion on both renderers');
    await page.waitForFunction(({width,targetWidth})=>{
      const path=document.querySelector('#hierarchy-svg svg path[data-id="node:leaf-a"]');
      const current=path.getBBox().width;
      return current>width+1&&current<targetWidth-1;
    },{width:initial.width,targetWidth:started[0].target.bounds[2]});
    const sampled=await page.evaluate(()=>{
      const {svg,path,canvasSignature}=window.hierarchyMotionProbe;
      const live=document.querySelector('#hierarchy-svg svg path[data-id="node:leaf-a"]');
      const box=live.getBBox(),center=[box.x+box.width/2,box.y+box.height/2];
      for(const renderer of ['svg','canvas']){
        const surface=document.querySelector(`#hierarchy-${renderer} .kartis-surface`);
        const {left,top,width,height}=surface.getBoundingClientRect();
        surface.dispatchEvent(new PointerEvent('pointermove',{
          pointerId:1,pointerType:'mouse',isPrimary:true,bubbles:true,
          clientX:left+center[0]/640*width,clientY:top+center[1]/360*height,
        }));
      }
      window.hostFixture.hierarchy_flush();
      const active=['svg','canvas'].map(renderer=>JSON.parse(window.hostFixture.hierarchy_snapshot(renderer)));
      const controls=['svg','canvas'].map(renderer=>[...document.querySelectorAll(`#hierarchy-${renderer} .kartis-datum`)].map(button=>button.textContent));
      const current=live.getBBox();
      return {retained:svg===document.querySelector('#hierarchy-svg svg')&&path===live,width:current.width,center:[current.x+current.width/2,current.y+current.height/2],canvas:canvasSignature(),active,controls};
    });
    assert(sampled.retained&&sampled.width>initial.width+1&&sampled.canvas!==initial.canvas,'Animated focus lost its keyed path or sampled SVG/Canvas geometry');
    for(const [i,s] of sampled.active.entries()){
      const row=s.interaction?.rows.find(row=>row.id==='node:leaf-a');
      assert(s.animating&&s.hovered.includes('node:leaf-a')&&row?.label==='<Leaf A>'&&row.value===2&&row.formattedValue==='2.0'&&s.interaction.anchor.every((value,i)=>value===row.position[i]),`Live hierarchy picking lost typed data ${JSON.stringify(s)}`);
      assert(sampled.controls[i].every(label=>!label.includes('group-b')),'Retiring hierarchy marks remained available to keyboard controls');
    }
    assert(sampled.active[0].interaction.anchor.every((value,i)=>Math.abs(value-sampled.center[i])<.01),'Typed interaction did not follow the live SVG circle center');
    const retargeted=await page.evaluate(()=>{
      for(const renderer of ['svg','canvas'])window.hostFixture.hierarchy_action(renderer,'back');
      const live=document.querySelector('#hierarchy-svg svg path[data-id="node:leaf-a"]');
      return {width:live.getBBox().width,retained:live===window.hierarchyMotionProbe.path,states:['svg','canvas'].map(renderer=>JSON.parse(window.hostFixture.hierarchy_snapshot(renderer)))};
    });
    assert(retargeted.retained&&retargeted.width>initial.width+1&&retargeted.states.every(s=>s.animating&&s.focus==='root'),'Back discarded sampled hierarchy geometry');
    await page.waitForFunction(()=>['svg','canvas'].every(renderer=>{const s=JSON.parse(window.hostFixture.hierarchy_snapshot(renderer));return !s.animating&&!s.pending;}));
    assert(await page.evaluate(d=>{const path=document.querySelector('#hierarchy-svg svg path[data-id="node:leaf-a"]');return path===window.hierarchyMotionProbe.path&&path.getAttribute('d')===d;},initial.d),'Hierarchy motion did not restore exact root geometry');
    await neutral();
    const settled=await pixels('animated hierarchy root');
    assert((await Promise.all(['svg','canvas'].map(snapshot))).every(s=>!s.events.some(e=>e.type==='error')),'Hierarchy motion reported a browser error');
    await page.evaluate(()=>window.hostFixture.hierarchy_stop());
    assert(await page.locator('body *').count()===baseline,'Animated hierarchy Stop leaked DOM');
    return {sampledWidth:sampled.width,initialWidth:initial.width,retargetedWidth:retargeted.width,liveTypedPicking:true,settled};
  };
  try{
    await page.emulateMedia({reducedMotion:'reduce'});
    for(const caseIndex of [0,1,2,3]){
      await page.mouse.move(2,2);await page.evaluate(i=>window.hostFixture.hierarchy_mount(i),caseIndex);await flush();
      await page.evaluate(()=>{const root=document.querySelector('#hierarchy-svg svg');window.hierarchyProbe={root,paths:new Map([...root.querySelectorAll('path[data-id]')].map(p=>[p.dataset.id,p])),observer:null,mutations:0};});
      const stages=[{name:'root',pixels:await pixels(`case${caseIndex} root`)}];
      for(const renderer of ['svg','canvas']){
        const state=await snapshot(renderer),surface=page.locator(`#hierarchy-${renderer} .kartis-surface`);assert(state.focus==='root'&&state.target.raw==='group-a','Hierarchy group target is not exposed');await surface.scrollIntoViewIfNeeded();const box=await surface.boundingBox(),x=box.x+state.target.x/state.width*box.width,y=box.y+state.target.y/state.height*box.height;
        await page.mouse.move(x,y);await flush();assert((await snapshot(renderer)).hovered.includes(state.target.id),`${renderer} current parent hit failed`);const tooltip=await page.locator(`#hierarchy-${renderer} .kartis-surface-tooltip`).textContent();assert(tooltip.includes('group-a')&&tooltip.includes('5'),`${renderer} parent current total missing`);
        await page.mouse.click(x,y);await flush();const focused=await snapshot(renderer);assert(focused.focus==='group-a'&&focused.ancestors.join(',')==='root'&&focused.view.scale===1&&focused.view.x===0&&focused.view.y===0,'Activation used camera zoom or lost ancestor metadata');assert(!focused.animating,'Hierarchy focus animated under reduced motion');assert(focused.nodes.find(n=>n.id==='group-b').visible===false&&focused.nodes.find(n=>n.id==='group-b').depth===null,'Focus retained unrelated projected metadata');
      }
      await neutral();stages.push({name:'group',pixels:await pixels(`case${caseIndex} group`)});
      assert(await page.evaluate(()=>{const root=document.querySelector('#hierarchy-svg svg');return root===window.hierarchyProbe.root&&[...root.querySelectorAll('path[data-id]')].every(p=>window.hierarchyProbe.paths.get(p.dataset.id)===p);}),`case${caseIndex} focus recreated common keyed paths`);
      for(const renderer of ['svg','canvas']){
        const control=page.locator(`#hierarchy-${renderer} .kartis-datum`).filter({hasText:'<Leaf A>'}).first();await control.focus();await page.keyboard.press('Enter');await flush();const s=await snapshot(renderer);assert(s.focus==='leaf-a'&&s.ancestors.join(',')==='root,group-a'&&s.nodes.filter(n=>n.visible).length===1&&s.events.some(e=>e.type==='activated'&&e.id==='node:leaf-a'),`${renderer} keyboard did not navigate to current leaf`);
      }
      await neutral();stages.push({name:'leaf',pixels:await pixels(`case${caseIndex} leaf`)});await sameAndRejected();
      await page.evaluate(()=>window.hostFixture.hierarchy_update(1));await flush();for(const renderer of ['svg','canvas']){const s=await snapshot(renderer);assert(s.focus==='leaf-a'&&s.target.value===9&&s.target.tooltip.includes('9'),`${renderer} current-data focus not preserved`);const surface=page.locator(`#hierarchy-${renderer} .kartis-surface`);await surface.scrollIntoViewIfNeeded();const box=await surface.boundingBox();await page.mouse.move(box.x+s.target.x/s.width*box.width,box.y+s.target.y/s.height*box.height);await flush();assert((await page.locator(`#hierarchy-${renderer} .kartis-surface-tooltip`).textContent()).includes('9'),`${renderer} updated current tooltip stale`);}
      await neutral();stages.push({name:'updated',pixels:await pixels(`case${caseIndex} updated`)});
      await page.evaluate(()=>window.hostFixture.hierarchy_update(2));await flush();for(const renderer of ['svg','canvas']){const s=await snapshot(renderer);assert(s.focus==='group-a'&&s.nodes.every(n=>n.id!=='leaf-a')&&s.nodes.find(n=>n.id==='group-a').value===3,'Deleted focus did not recover nearest surviving ancestor');}
      stages.push({name:'deleted-recovery',pixels:await pixels(`case${caseIndex} deletion`)});await action('back');assert((await snapshot('svg')).focus==='root','Back history did not recover root');
      await page.evaluate(()=>window.hostFixture.hierarchy_update(0));await flush();for(const renderer of ['svg','canvas'])await page.evaluate(renderer=>window.hostFixture.hierarchy_hidden(renderer,true),renderer);await flush();
      assert(await page.evaluate(()=>{const root=document.querySelector('#hierarchy-svg svg');return !root.querySelector('[data-id="node:leaf-a"]')&&!Array.from(root.querySelectorAll('text')).some(t=>t.textContent==='<Leaf A>')&&!!root.querySelector('[data-id="node:leaf-b"]');}),'Hidden leaf retained its ink/owned label or removed sibling');for(const renderer of ['svg','canvas'])assert(await page.locator(`#hierarchy-${renderer} .kartis-datum`).filter({hasText:'<Leaf A>'}).count()===0,'Hidden owned control retained');
      stages.push({name:'hidden',pixels:await pixels(`case${caseIndex} hidden`)});await action('group-a');await action('parent');assert((await snapshot('svg')).focus==='root','Parent navigation failed');await action('root');
      await page.evaluate(()=>{window.oldHierarchy=document.querySelector('#hierarchy-svg .kartis-datum');window.hostFixture.hierarchy_stop();});assert(await page.locator('body *').count()===baseline,'Hierarchy Stop leaked DOM');await page.evaluate(()=>window.oldHierarchy?.click());await flush();assert(!(await snapshot('svg')).active,'Stopped hierarchy revived from detached input');reports.push({case:caseIndex,stages,pointer:true,keyboardFocus:true,backParentRoot:true,deletedFocusRecovery:true,currentData:true,retainedPaths:true,unchangedMutationZero:true,rejectedAtomic:true,ownedLabels:true,cleanup:true});
    }
    const motion=await animatedFocus();
    assert(errors.length===0,errors.join('\n'));return{passed:true,cases:reports,motion,pageErrors:errors};
  }finally{await page.evaluate(previous=>{window.hierarchyProbe?.observer?.disconnect();window.hostFixture.hierarchy_stop();document.querySelector('.panels').style.display=previous;delete window.hierarchyProbe;delete window.hierarchyMotionProbe;delete window.oldHierarchy;},previous);await page.emulateMedia({reducedMotion:'no-preference'});page.off('pageerror',onError);}
})

