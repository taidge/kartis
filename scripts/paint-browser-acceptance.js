(async page=>{
  const errors=[];const error=e=>errors.push(e.message);page.on('pageerror',error);
  const assert=(ok,message)=>{if(!ok)throw Error(message);};
  try{
    await page.evaluate(()=>window.hostFixture.paints_mount());await page.waitForFunction(()=>window.hostFixture.paints_ready());
    const compare=()=>page.evaluate(async()=>{
      const liveSvg=document.querySelector('#paint-svg svg'),source=liveSvg.cloneNode(true);
      // Standalone SVG images do not retain the live tree's inherited language.
      const languageOwner=liveSvg.closest('[lang]');
      let language;
      for(let node=liveSvg;node;node=node.parentElement){
        const xmlLanguage=node.getAttributeNS('http://www.w3.org/XML/1998/namespace','lang')??node.getAttribute('xml:lang');
        if(xmlLanguage!==null){language=xmlLanguage;break;}
        if(node===languageOwner){language=node.getAttribute('lang');break;}
      }
      source.setAttribute('lang',language??liveSvg.ownerDocument.documentElement.lang);
      source.setAttribute('width','600');source.setAttribute('height','260');
      const image=new Image();image.src='data:image/svg+xml;base64,'+btoa(new XMLSerializer().serializeToString(source));await image.decode();
      const live=document.querySelector('#paint-canvas canvas'),sample=document.createElement('canvas');sample.width=live.width;sample.height=live.height;const c=sample.getContext('2d');c.drawImage(image,0,0,sample.width,sample.height);
      const a=c.getImageData(0,0,sample.width,sample.height).data,b=live.getContext('2d').getImageData(0,0,live.width,live.height).data;let sum=0;for(let i=0;i<a.length;i++)sum+=Math.abs(a[i]-b[i]);
      const pixel=(x,y)=>[...live.getContext('2d').getImageData(Math.floor(x*live.width/600),Math.floor(y*live.height/260),1,1).data];
      return{mean:sum/a.length,center:pixel(420,120),left:pixel(45,100),right:pixel(245,100),clip:pixel(25,100),definitions:source.querySelectorAll('linearGradient,radialGradient').length};
    });
    const initial=await compare();assert(initial.mean<0.5,`Gradient SVG/Canvas differ: ${initial.mean}`);assert(initial.definitions===2&&initial.center[0]>245&&initial.center[1]>220&&initial.left[0]>200&&initial.right[2]>200&&initial.clip.every(v=>v===255),'Actual linear/radial/clip paint pixels failed');
    await page.evaluate(()=>{const root=document.querySelector('#paint-svg svg');window.paintProbe={path:root.querySelector('[data-id="linear"]'),radial:root.querySelector('radialGradient'),changes:0};const observer=new MutationObserver(records=>window.paintProbe.changes+=records.length);observer.observe(root,{subtree:true,attributes:true,childList:true,characterData:true});window.paintProbe.observer=observer;window.hostFixture.paints_case(0);window.hostFixture.paints_flush();});
    await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(resolve)));
    assert(await page.evaluate(()=>window.paintProbe.changes===0),'Unchanged gradient repaint mutated SVG');
    await page.evaluate(()=>{window.paintProbe.observer.disconnect();window.hostFixture.paints_case(1);window.hostFixture.paints_flush();});
    const changed=await compare();assert(changed.mean<0.5&&changed.left[1]>140,'Gradient style update did not reach real Canvas');
    assert(await page.evaluate(()=>{const r=document.querySelector('#paint-svg svg');return r.querySelector('[data-id="linear"]')===window.paintProbe.path&&r.querySelector('radialGradient')===window.paintProbe.radial&&r.querySelectorAll('linearGradient').length===1;}),'Style update lost keyed path/unchanged definition or leaked old definitions');
    await page.evaluate(()=>{window.hostFixture.paints_case(2);window.hostFixture.paints_flush();});const solid=await compare();assert(solid.mean<0.5&&solid.definitions===0,'Gradient removal retained definitions or wrong solid fill');
    await page.evaluate(()=>{window.hostFixture.paints_case(3);window.hostFixture.paints_flush();});const removed=await compare();assert(removed.mean<0.5&&removed.definitions===1,'Mark exit retained unused paint');
    assert(errors.length===0,errors.join('\n'));
    return{passed:true,initial,changed,solid,removed,keyedPath:true,unchangedMutationZero:true,definitionsRetired:true,pageErrors:errors};
  }finally{await page.evaluate(()=>{window.hostFixture.paints_stop();delete window.paintProbe;});page.off('pageerror',error);}
})
