// Browser-native geometric reference. Reads authored/verified Scene values only;
// it never reads product DOM, screenshots, pixels or product hit-test results.
export function selectFunnelPoint({scene,targetScene,id,bodyIDs=[],mode='paint'}){
 const canvas=document.createElement('canvas'),ctx=canvas.getContext('2d');
 const path=m=>new Path2D(m.geometry.commands.map(c=>c.join(' ')).join(' '));
 const maps=new Map(scene.marks.map(m=>[m.id,{mark:m,path:path(m)}]));
 const wanted=mode==='target-only'?targetScene:scene,body=wanted.marks.find(m=>m.id===id);
 if(!body||!body.fill||!body.fill[3]||!body.opacity)return null;
 const bodyPath=path(body),target=targetScene?.marks.find(m=>m.id===id),targetPath=target&&path(target);
 const points=body.geometry.commands.flatMap(c=>c.slice(1)).filter(Number.isFinite),xs=points.filter((_,i)=>i%2===0),ys=points.filter((_,i)=>i%2===1);
 if(!xs.length)return null;
 const clip=body.clip??[0,0,scene.width,scene.height],bounds=[Math.max(clip[0],Math.min(...xs)),Math.max(clip[1],Math.min(...ys)),Math.min(clip[0]+clip[2],Math.max(...xs)),Math.min(clip[1]+clip[3],Math.max(...ys))];
 const stroke=(p,x,y,width)=>{ctx.lineWidth=width;ctx.lineCap='round';ctx.lineJoin='round';return ctx.isPointInStroke(p,x,y);};
 const contains=(m,p,x,y)=>{const c=m.clip;return(!c||(x>=c[0]&&y>=c[1]&&x<=c[0]+c[2]&&y<=c[1]+c[3]))&&ctx.isPointInPath(p,x,y);};
 const bodyIndex=wanted.marks.indexOf(body),after=new Set();let found=false;
 for(const [kind,i] of wanted.order){if(kind==='M'&&i===bodyIndex)found=true;else if(found)after.add(kind+':'+i);}
 let tested=0;
 function eligible(x,y){
  tested++;if(x<=bounds[0]+4||x>=bounds[2]-4||y<=bounds[1]+4||y>=bounds[3]-4||!ctx.isPointInPath(bodyPath,x,y)||stroke(bodyPath,x,y,8))return false;
  if(mode==='sample-only'&&targetPath&&(ctx.isPointInPath(targetPath,x,y)||stroke(targetPath,x,y,8)))return false;
  if(mode==='target-only')return !bodyIDs.some(k=>{const v=maps.get(k);return v&&(contains(v.mark,v.path,x,y)||stroke(v.path,x,y,8));});
  if(mode==='sample-only')return true;
  for(const key of after){const [kind,index]=key.split(':'),i=Number(index);
   if(kind==='L'){const l=wanted.labels[i],w=l.size*(l.text.length*.75+1)+6;if(Math.abs(x-l.position[0])<=w&&Math.abs(y-l.position[1])<=l.size+6)return false;}
   else{const m=wanted.marks[i];if(!m.opacity)continue;const p=path(m);if((m.fill?.[3]&&contains(m,p,x,y))||(m.stroke?.color[3]&&stroke(p,x,y,m.stroke.width+8)))return false;}
  }return true;
 }
 const cx=(bounds[0]+bounds[2])/2,cy=(bounds[1]+bounds[3])/2;
 for(const[dx,dy]of[[12,28],[28,12],[-12,28],[-28,12],[12,-28],[28,-12],[-12,-28],[-28,-12]]){const x=cx+dx,y=cy+dy;if(eligible(x,y))return{id,mode,point:[x,y],bounds,tested};}
 for(let y=Math.ceil(bounds[1]/8)*8+.5;y<bounds[3];y+=8)for(let x=Math.ceil(bounds[0]/8)*8+.5;x<bounds[2];x+=8)if(eligible(x,y))return{id,mode,point:[x,y],bounds,tested};
 return null;
}
