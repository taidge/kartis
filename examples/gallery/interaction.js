// Standalone gallery inspection uses the same native dash/cap and explicit
// clip definitions as its generated SVG. Production widgets use Rust picking.
document.querySelectorAll('article').forEach(card=>{
  const svg=card.querySelector('svg'),canvas=card.querySelector('canvas');
  const status=card.querySelector('.status'),ctx=canvas.getContext('2d');
  const definitions=new Map([...svg.querySelectorAll('clipPath[id]')].map(def=>{
    const path=def.querySelector('path');
    return[def.id,{path:new Path2D(path.getAttribute('d')),rule:path.getAttribute('clip-rule')||'nonzero'}];
  }));
  const items=[...svg.querySelectorAll('path[data-id]')].map(el=>{
    const clips=[];
    for(let parent=el.parentElement;parent&&parent!==svg;parent=parent.parentElement){
      if(parent.localName==='svg'){
        const box=parent.getAttribute('viewBox').split(/[ ,]+/).map(Number);clips.push({box});
      }
      const reference=parent.getAttribute('clip-path');
      if(reference){const definition=definitions.get(reference.slice(5,-1));if(!definition)throw Error('Missing generated clip definition');clips.push(definition);}
    }
    return{el,path:new Path2D(el.getAttribute('d')),clips};
  });
  card.querySelector('.toggle').onclick=event=>{
    const enabled=card.classList.toggle('canvas-mode');event.currentTarget.setAttribute('aria-pressed',String(enabled));
    event.currentTarget.textContent=enabled?'Canvas → SVG':'SVG → Canvas';
  };
  const move=event=>{
    const box=event.currentTarget.getBoundingClientRect(),view=svg.viewBox.baseVal;
    const x=view.x+(event.clientX-box.left)*view.width/box.width,y=view.y+(event.clientY-box.top)*view.height/box.height;
    let item;
    ctx.save();
    try{
      ctx.setTransform(1,0,0,1,0,0);
      for(let i=items.length-1;i>=0;i--){
        const it=items[i],el=it.el;
        if(!el.querySelector('title').textContent||Number(el.getAttribute('opacity')||1)===0)continue;
        if(it.clips.some(clip=>clip.box
          ?x<clip.box[0]||x>clip.box[0]+clip.box[2]||y<clip.box[1]||y>clip.box[1]+clip.box[3]
          :!ctx.isPointInPath(clip.path,x,y,clip.rule)))continue;
        ctx.lineWidth=Number(el.getAttribute('stroke-width')||0)+4;
        ctx.lineCap=el.getAttribute('stroke-linecap')||'butt';ctx.miterLimit=4;
        const dash=el.getAttribute('stroke-dasharray');
        ctx.setLineDash(dash&&dash!=='none'?dash.split(/[ ,]+/).map(Number):[]);
        ctx.lineDashOffset=Number(el.getAttribute('stroke-dashoffset')||0);
        if(el.getAttribute('fill')!=='none'&&ctx.isPointInPath(it.path,x,y)
          ||el.hasAttribute('stroke')&&ctx.isPointInStroke(it.path,x,y)){item=it;break;}
      }
    }finally{ctx.restore();}
    status.textContent=item?item.el.querySelector('title').textContent+' · '+item.el.dataset.value:items.length+' marks · shared Rust scene';
  };
  svg.addEventListener('pointermove',move);canvas.addEventListener('pointermove',move);
});
