// Retained SVG identity and order acceptance. Leaves the ordinary fixture intact.
(async page => {
  const assert=(ok,message)=>{if(!ok)throw Error(message);};
  const wait=()=>page.waitForFunction(()=>{const s=JSON.parse(window.hostFixture.snapshot('svg'));return s.active&&s.pending===0&&!s.animating;});
  await page.evaluate(()=>window.hostFixture.remount('svg'));await wait();
  await page.evaluate(()=>{
    const root=document.querySelector('#svg-root svg');
    window.retainedSvg={root,alpha:root.querySelector('path[data-id="datum-alpha"]'),beta:root.querySelector('path[data-id="datum-beta"]'),label:root.querySelector('text'),group:root.querySelector('g'),title:root.querySelector('title'),background:root.querySelector('rect')};
  });
  await page.evaluate(()=>window.hostFixture.set_phase('svg',1,true));
  await page.waitForFunction(()=>JSON.parse(window.hostFixture.snapshot('svg')).animating);
  const animation=await page.evaluate(async()=>{
    let frames=0,paths=new Set();while(frames<300){await new Promise(requestAnimationFrame);const saved=window.retainedSvg,root=document.querySelector('#svg-root svg');
      if(root!==saved.root||root.querySelector('path[data-id="datum-alpha"]')!==saved.alpha||root.querySelector('text')!==saved.label)throw Error('Animation recreated keyed SVG nodes');
      paths.add(saved.alpha.getAttribute('d'));frames++;if(!JSON.parse(window.hostFixture.snapshot('svg')).animating)break;
    }return{frames,distinct:paths.size};
  });assert(animation.distinct>5,'Retained path never received animation patches');await wait();
  await page.evaluate(()=>{const root=window.retainedSvg.root;window.svgMutations=[];window.svgObserver=new MutationObserver(records=>window.svgMutations.push(...records));window.svgObserver.observe(root,{attributes:true,childList:true,subtree:true,characterData:true});window.hostFixture.set_phase('svg',1,false);window.hostFixture.flush('svg');});
  await page.evaluate(()=>new Promise(resolve=>queueMicrotask(resolve)));
  assert(await page.evaluate(()=>window.svgMutations.length)===0,'An unchanged frame mutated the retained SVG');
  await page.evaluate(()=>window.svgObserver.disconnect());
  await page.mouse.move(2,2);
  await page.evaluate(()=>{window.hostFixture.set_phase('svg',0,false);window.hostFixture.set_phase('canvas',0,false);});await wait();
  await page.waitForFunction(()=>JSON.parse(window.hostFixture.snapshot('canvas')).pending===0);
  const meanChannelError=await page.evaluate(async()=>{
    const liveSvg=document.querySelector('#svg-root svg'),source=liveSvg.cloneNode(true),canvas=document.querySelector('#canvas-root canvas');
    // Standalone SVG images do not retain the live tree's inherited language.
    const languageOwner=liveSvg.closest('[lang]');
    let language;
    for(let node=liveSvg;node;node=node.parentElement){
      const xmlLanguage=node.getAttributeNS('http://www.w3.org/XML/1998/namespace','lang')??node.getAttribute('xml:lang');
      if(xmlLanguage!==null){language=xmlLanguage;break;}
      if(node===languageOwner){language=node.getAttribute('lang');break;}
    }
    source.setAttribute('lang',language??liveSvg.ownerDocument.documentElement.lang);
    const raster=document.createElement('canvas');raster.width=canvas.width;raster.height=canvas.height;
    // The two panels have equal measured logical dimensions in this fixture.
    source.setAttribute('width',String(canvas.width));source.setAttribute('height',String(canvas.height));
    const url=URL.createObjectURL(new Blob([new XMLSerializer().serializeToString(source)],{type:'image/svg+xml'}));
    try{const image=new Image();await new Promise((resolve,reject)=>{image.onload=resolve;image.onerror=reject;image.src=url;});raster.getContext('2d').drawImage(image,0,0);const a=raster.getContext('2d').getImageData(0,0,raster.width,raster.height).data,b=canvas.getContext('2d').getImageData(0,0,canvas.width,canvas.height).data;let diff=0;for(let i=0;i<a.length;i++)diff+=Math.abs(a[i]-b[i]);return diff/a.length;}finally{URL.revokeObjectURL(url);}
  });assert(meanChannelError<1,`Retained SVG differs from live Canvas: ${meanChannelError}`);
  await page.evaluate(()=>window.hostFixture.svg_patch_case(1));await wait();
  const clipped=await page.evaluate(()=>{const s=window.retainedSvg,root=document.querySelector('#svg-root svg');return{same:root===s.root&&root.querySelector('[data-id="datum-alpha"]')===s.alpha&&root.querySelector('text')===s.label,clip:s.alpha.parentElement.localName,stroke:s.alpha.getAttribute('stroke-width'),order:[...s.group.children].map(el=>el.localName==='svg'?el.querySelector('path').dataset.id:el.dataset.id||el.textContent),text:s.label.textContent,unescaped:root.querySelectorAll('safe').length};});
  assert(clipped.same&&clipped.clip==='svg'&&clipped.stroke==='3','Clip/style update replaced the stable path');
  assert(clipped.order[0]==='<safe & retained>'&&clipped.text==='<safe & retained>'&&clipped.unescaped===0,'Mixed order or safe text patch failed');
  await page.evaluate(()=>window.hostFixture.svg_patch_case(2));await wait();
  assert(await page.evaluate(()=>!window.retainedSvg.beta.isConnected&&document.querySelector('#svg-root [data-id="datum-beta"]')===null),'Removed datum remained in the DOM');
  await page.evaluate(()=>window.hostFixture.svg_patch_case(3));await wait();
  const restored=await page.evaluate(()=>{const s=window.retainedSvg;return {root:document.querySelector('#svg-root svg')===s.root,alpha:document.querySelector('#svg-root [data-id="datum-alpha"]')===s.alpha,parent:s.alpha.parentElement===s.group,stroke:s.alpha.getAttribute('stroke'),width:s.alpha.getAttribute('stroke-width'),title:s.alpha.querySelector('title')!==null,beta:document.querySelector('#svg-root [data-id="datum-beta"]')!==s.beta};});assert(restored.root&&restored.alpha&&restored.parent&&!restored.stroke&&!restored.width&&restored.title&&restored.beta,`Unclipping or removal/reentry retirement failed ${JSON.stringify(restored)}`);
  await page.evaluate(()=>window.hostFixture.svg_patch_case(4));await wait();
  assert(await page.evaluate(()=>{const s=window.retainedSvg;return document.querySelector('#svg-root [data-id="datum-alpha"]')===s.alpha&&s.alpha.parentElement.localName==='svg'&&s.alpha.getAttribute('d').includes('C')&&s.alpha.getAttribute('fill')==='none'&&s.alpha.getAttribute('stroke-linecap')==='round';}),'Geometry topology or clip re-entry replaced the mark path');
  await page.evaluate(()=>{window.hostFixture.stop('svg');});
  assert(await page.evaluate(()=>!window.retainedSvg.root.isConnected&&!window.retainedSvg.alpha.isConnected),'Stopped chart retained connected SVG');
  await page.evaluate(()=>{window.hostFixture.remount('svg');window.hostFixture.set_phase('svg',1,false);});await wait();
  await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));await wait();
  assert(await page.evaluate(()=>document.querySelector('#svg-root svg')!==window.retainedSvg.root),'Remount reused retired SVG root');
  await page.evaluate(()=>{delete window.retainedSvg;delete window.svgObserver;delete window.svgMutations;});
  return{animation,meanChannelError,rootIdentity:true,markIdentity:true,labelSlotIdentity:true,unchangedFrameNoMutations:true,clipTransition:true,geometryTopology:true,mixedOrder:true,safeText:true,retirement:true,remount:true};
})
