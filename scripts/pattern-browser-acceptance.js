(async page=>{
  const assert=(ok,message)=>{if(!ok)throw Error(message);};
  const errors=[];const error=e=>errors.push(e.message);page.on('pageerror',error);
  try {
    await page.evaluate(()=>window.hostFixture.paints_mount());
    await page.waitForFunction(()=>window.hostFixture.paints_ready());
    const cases=[];
    for(const phase of [4,5,6]) {
      await page.evaluate(phase=>{window.hostFixture.paints_case(phase);window.hostFixture.paints_flush();},phase);
      const result=await page.evaluate(async()=>{
        const root=document.querySelector('#paint-svg svg'),canvas=document.querySelector('#paint-canvas canvas');
        const source=root.cloneNode(true);
        // Standalone SVG images do not retain the live tree's inherited language.
        const languageOwner=root.closest('[lang]');
        let language;
        for(let node=root;node;node=node.parentElement){
          const xmlLanguage=node.getAttributeNS('http://www.w3.org/XML/1998/namespace','lang')??node.getAttribute('xml:lang');
          if(xmlLanguage!==null){language=xmlLanguage;break;}
          if(node===languageOwner){language=node.getAttribute('lang');break;}
        }
        source.setAttribute('lang',language??root.ownerDocument.documentElement.lang);
        source.setAttribute('width','600');source.setAttribute('height','260');
        const image=new Image();image.src='data:image/svg+xml;base64,'+btoa(new XMLSerializer().serializeToString(source));await image.decode();
        const target=document.createElement('canvas');target.width=canvas.width;target.height=canvas.height;
        const c=target.getContext('2d');c.drawImage(image,0,0,target.width,target.height);
        const a=c.getImageData(0,0,target.width,target.height).data,b=canvas.getContext('2d').getImageData(0,0,canvas.width,canvas.height).data;
        let sum=0;for(let i=0;i<a.length;i++)sum+=Math.abs(a[i]-b[i]);
        const motifs=new Set();const ratio=canvas.width/600;
        for(let y=50;y<190;y+=2)for(let x=50;x<240;x+=2){const i=(Math.floor(y*ratio)*canvas.width+Math.floor(x*ratio))*4;motifs.add(Math.floor(b[i]/20));}
        return {mean:sum/a.length,motifColors:motifs.size,definitions:root.querySelectorAll('pattern').length,sharedFill:new Set([...root.querySelectorAll('[data-id]')].map(p=>p.getAttribute('fill'))).size};
      });
      assert(result.mean<2,`Pattern ${phase} SVG/Canvas error ${result.mean}`);
      assert(result.definitions===1&&result.sharedFill===1&&result.motifColors>=5,`Pattern ${phase} lacks shared definition or actual motifs`);
      cases.push({phase,...result});
    }
    const unchanged=await page.evaluate(async()=>{
      const root=document.querySelector('#paint-svg svg'),path=root.querySelector('[data-id="linear"]'),pattern=root.querySelector('pattern');let mutations=0;
      const observer=new MutationObserver(records=>mutations+=records.length);observer.observe(root,{subtree:true,attributes:true,childList:true,characterData:true});
      window.hostFixture.paints_case(6);window.hostFixture.paints_flush();await new Promise(r=>requestAnimationFrame(r));observer.disconnect();
      return{mutations,pathKept:root.querySelector('[data-id="linear"]')===path,patternKept:root.querySelector('pattern')===pattern};
    });
    assert(unchanged.mutations===0&&unchanged.pathKept&&unchanged.patternKept,'Unchanged pattern frame mutated retained DOM');
    await page.evaluate(()=>{window.hostFixture.paints_case(2);window.hostFixture.paints_flush();});
    assert(await page.locator('#paint-svg pattern').count()===0,'Pattern definition leaked after solid replacement');
    assert(!errors.length,errors.join('\n'));
    return{passed:true,cases,unchanged,definitionsRetired:true,pageErrors:errors};
  } finally {await page.evaluate(()=>window.hostFixture.paints_stop());page.off('pageerror',error);}
})
