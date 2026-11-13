(async page=>{
  const errors=[];const onError=error=>errors.push(error.message);page.on('pageerror',onError);
  const assert=(ok,message)=>{if(!ok)throw Error(message);};
  const ready=()=>page.waitForFunction(()=>{const states=JSON.parse(window.hostFixture.clips_snapshot());return states.length===2&&states.every(s=>s.active&&s.pending===0);});
  const phase=async value=>{await page.evaluate(value=>{window.hostFixture.clips_phase(value);window.hostFixture.clips_flush();},value);await ready();};
  const compare=()=>page.evaluate(async()=>{
    const live=document.querySelector('#clip-canvas canvas');
    const liveSvg=document.querySelector('#clip-svg svg'),source=liveSvg.cloneNode(true);
    // Standalone SVG images do not retain the live tree's inherited language.
    const languageOwner=liveSvg.closest('[lang]');
    let language;
    for(let node=liveSvg;node;node=node.parentElement){
      const xmlLanguage=node.getAttributeNS('http://www.w3.org/XML/1998/namespace','lang')??node.getAttribute('xml:lang');
      if(xmlLanguage!==null){language=xmlLanguage;break;}
      if(node===languageOwner){language=node.getAttribute('lang');break;}
    }
    source.setAttribute('lang',language??liveSvg.ownerDocument.documentElement.lang);
    source.setAttribute('width',String(live.width));source.setAttribute('height',String(live.height));
    const image=new Image();image.src='data:image/svg+xml;charset=utf-8,'+encodeURIComponent(new XMLSerializer().serializeToString(source));await image.decode();
    const sample=document.createElement('canvas');sample.width=live.width;sample.height=live.height;
    const context=sample.getContext('2d');context.drawImage(image,0,0);
    const svg=context.getImageData(0,0,sample.width,sample.height).data;
    const cpu=live.getContext('2d').getImageData(0,0,live.width,live.height).data;
    const pixel=(data,x,y)=>{const px=Math.floor(x*live.width/600),py=Math.floor(y*live.height/280),offset=(py*live.width+px)*4;return [...data.slice(offset,offset+4)];};
    const ink=(data,x0,y0,x1,y1)=>{
      let total=0;for(let y=Math.ceil(y0*live.height/280);y<Math.floor(y1*live.height/280);y++)for(let x=Math.ceil(x0*live.width/600);x<Math.floor(x1*live.width/600);x++){
        const i=(y*live.width+x)*4;if(Math.max(data[i],data[i+1],data[i+2])<100&&data[i+3]>0)total++;
      }return total;
    };
    let sum=0;for(let i=0;i<svg.length;i++)sum+=Math.abs(svg[i]-cpu[i]);
    const probes=data=>({fill:pixel(data,120,100),stroke:pixel(data,120,50),cutStroke:pixel(data,120,42),hole:pixel(data,400,190),ring:pixel(data,320,140),dash:pixel(data,476,70),dashCut:pixel(data,483,70),textInside:ink(data,45,190,175,260),textOutside:ink(data,185,160,280,260)});
    return {meanChannelError:sum/svg.length,svg:probes(svg),canvas:probes(cpu),definitions:source.querySelectorAll('clipPath').length};
  });
  try{
    await page.evaluate(()=>window.hostFixture.clips_mount());await ready();
    const initial=await compare();
    assert(initial.meanChannelError<0.75,`Clipped SVG/Canvas parity ${initial.meanChannelError}`);
    assert(initial.definitions===4,'Missing mark/text clip definitions');
    for(const p of [initial.svg,initial.canvas]){
      assert(p.fill.join(',')==='45,110,185,255','Circle clip lost fill');
      assert(p.stroke.join(',')==='225,115,30,255'&&p.cutStroke.join(',')==='255,255,255,255','Clip did not crop the actual stroke');
      assert(p.hole.join(',')==='255,255,255,255'&&p.ring.join(',')==='55,165,85,255','Even-odd clip hole was not empty');
      assert(p.dash.join(',')==='125,45,160,255'&&p.dashCut.join(',')==='255,255,255,255','Dashed stroke did not intersect both clip layers');
      assert(p.textInside>0&&p.textOutside===0,'Rotated text was not clipped in logical coordinates');
    }
    await page.evaluate(()=>{
      const root=document.querySelector('#clip-svg svg');
      window.clipProbe={root,paths:[...root.querySelectorAll('path[data-id]')],text:root.querySelector('text'),mutations:0,definitions:[...root.querySelectorAll('clipPath')]};
      const observer=new MutationObserver(records=>window.clipProbe.mutations+=records.length);observer.observe(root,{subtree:true,attributes:true,childList:true,characterData:true});window.clipProbe.observer=observer;
      window.hostFixture.clips_phase(0);window.hostFixture.clips_flush();
    });
    await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(resolve)));
    assert(await page.evaluate(()=>window.clipProbe.mutations===0),'Unchanged clipped frame mutated retained SVG');
    await page.evaluate(()=>window.clipProbe.observer.disconnect());
    await phase(1);const changed=await compare();
    assert(changed.meanChannelError<0.75,`Updated clipping parity ${changed.meanChannelError}`);
    assert(changed.definitions===4&&changed.svg.hole.join(',')==='55,165,85,255'&&changed.canvas.hole.join(',')==='55,165,85,255','Nonzero clip rule update failed');
    assert(await page.evaluate(()=>{
      const r=document.querySelector('#clip-svg svg'),probe=window.clipProbe;
      return r===probe.root&&probe.paths.every(path=>r.querySelector(`[data-id="${path.dataset.id}"]`)===path)&&r.querySelector('text')===probe.text
        &&!r.querySelector('[data-id="clip-dash"]').hasAttribute('stroke-dasharray')&&!r.querySelector('[data-id="clip-dash"]').hasAttribute('stroke-dashoffset')
        &&probe.definitions.filter(definition=>definition.isConnected).length===1;
    }),'Clip/rule/dash update lost path/text identity, retained old defs or stale dash attributes');
    const transitions=[];
    for(const value of [2,3,4,0]){
      await phase(value);const result=await compare();
      assert(result.meanChannelError<0.75,`Clip wrapper transition ${value} parity ${result.meanChannelError}`);
      assert(result.definitions===([2,4].includes(value)?0:4),`Unused definitions retained at case ${value}`);
      assert(await page.evaluate(()=>{
        const root=document.querySelector('#clip-svg svg');return window.clipProbe.paths.every(path=>root.querySelector(`[data-id="${path.dataset.id}"]`)===path)&&root.querySelector('text')===window.clipProbe.text;
      }),`Rect/path wrapper transition ${value} replaced keyed nodes`);
      if(value===2)assert(result.svg.textOutside>0&&result.canvas.textOutside>0,'Removing text clip did not expose the original glyphs');
      transitions.push({case:value,meanChannelError:result.meanChannelError,definitions:result.definitions});
    }
    assert((await page.evaluate(()=>JSON.parse(window.hostFixture.clips_errors()))).length===0,'Controller reported a clip render error');
    await page.evaluate(()=>window.hostFixture.clips_stop());
    assert(await page.locator('#clip-svg,#clip-canvas').count()===0,'Stop retained clipped controller roots');
    assert(errors.length===0,errors.join('\n'));
    return {passed:true,initial,changed,transitions,strokeMask:true,rotatedText:true,evenoddHole:true,pathIdentity:true,textIdentity:true,unchangedMutationZero:true,definitionsRetired:true,stop:true,pageErrors:errors};
  }finally{
    await page.evaluate(()=>{window.clipProbe?.observer.disconnect();window.hostFixture.clips_stop();delete window.clipProbe;});page.off('pageerror',onError);
  }
})
