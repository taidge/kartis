// Independent fixed Tree-label application oracle. Never reads host metadata.
export const ids=['a','c','r','b','d'];
const parents={a:'r',c:'a',r:null,b:'r',d:'a'},depths={a:1,c:2,r:0,b:1,d:2},heights={a:1,c:0,r:2,b:0,d:0};
const colors={a:[0,100,255,255],c:[230,110,20,255],r:[160,80,30,255],b:[0,200,0,255],d:[100,20,180,255]};
export const bits=v=>{const a=new DataView(new ArrayBuffer(8));a.setFloat64(0,v);return a.getBigUint64(0).toString();};
export const project=(p,view)=>p.map((v,i)=>v*view[0]+40+view[i+1]-40*view[0]);
const option=v=>v===null?'None':`Some(${v})`;
export function expected(phase=0,width=720,focus=null,view=[1,0,0],height=340){
 const values={a:0,c:phase===1?35:30,r:0,b:phase===1?5:-0,d:10};
 const sourceIds=phase===12?[]:phase===2?[...ids].reverse():ids;
 const raw=sourceIds.map(id=>({id,parent:parents[id],label:phase===9&&id==='b'?'':'source-'+id,value:values[id],bits:bits(values[id])}));
 const index=new Map(raw.map((d,i)=>[d.id,i])),children=raw.map(d=>raw.flatMap((v,i)=>v.parent===d.id?[i]:[])),root=raw.findIndex(d=>d.parent===null),bfs=root<0?[]:[root];
 for(let i=0;i<bfs.length;i++)bfs.push(...children[bfs[i]]);
 const ancestors=new Set();for(let p=focus===null?null:parents[focus];p!==null;p=parents[p])ancestors.add(p);
 const active=raw.map(d=>focus===null?null:d.id===focus||ancestors.has(d.id));
 const norms=phase===2?{r:.4,a:.6,b:.2,c:.8,d:.4}:{r:.6,a:.4,b:.8,c:.2,d:.6};
 const nodes=raw.map((d,i)=>({index:i,parent:d.parent===null?null:index.get(d.parent),children:children[i],depth:depths[d.id],height:heights[d.id],position:[64+depths[d.id]/2*(width-128),64+norms[d.id]*(height-128)],radius:focus===null?12+2*depths[d.id]:active[i]?18+2*depths[d.id]:4,rawBits:d.bits,value:0,markId:'node:'+d.id,color:colors[d.id]}));
 for(const i of [...bfs].reverse())nodes[i].value=children[i].length?children[i].reduce((s,j)=>s+nodes[j].value,0):raw[i].value;
 const links=bfs.filter(i=>raw[i].parent!==null).map((i,n)=>{const p=index.get(raw[i].parent);return{index:n,source:p,target:i,position:nodes[p].position.map((v,j)=>(v+nodes[i].position[j])/2),sourcePosition:nodes[p].position,targetPosition:nodes[i].position,markId:'link:'+raw[i].id};});
 const linkActive=links.map(l=>focus===null?null:raw[l.target].id===focus||ancestors.has(raw[l.target].id));
 const activity={focus,index:focus===null?null:index.get(focus),nodes:active,links:linkActive,flags:[true,false,true,false],depth:null};
 const rotation=phase===3?30:phase===4?-90:0,rad=rotation*Math.PI/180,sin=Math.sin(rad),cos=Math.cos(rad);
 const labels=phase===6?[]:bfs.filter(i=>phase!==5||children[i].length===0).map((i,n)=>{const d=raw[i],p=nodes[i].position;return{index:n,owner:d.id,label:{position:rotation===0?[p[0],p[1]+nodes[i].radius+5+12.8]:[p[0]-sin*16/3,p[1]+nodes[i].radius+5+cos*16/3],text:phase===9&&d.id==='b'?'':d.id+':'+option(active[i]),size:16,color:[100,20,180,phase===7&&d.id==='c'?0:255],rotation,anchor:d.id==='r'||d.id==='c'?'Start':d.id==='b'?'End':'Middle'}};});
 const plot=[40,40,width-80,height-80],rows=[];
 const clip=(rect,extra)=>{let x=Math.max(rect[0],plot[0]),y=Math.max(rect[1],plot[1]),r=Math.min(rect[0]+rect[2],width-40),b=Math.min(rect[1]+rect[3],height-40);if(extra){x=Math.max(x,extra[0]);y=Math.max(y,extra[1]);r=Math.min(r,extra[0]+extra[2]);b=Math.min(b,extra[1]+extra[3]);}return r>x&&b>y?[x,y,r-x,b-y]:null;};
 const projectedRect=rect=>[...project(rect.slice(0,2),view),rect[2]*view[0],rect[3]*view[0]];
 if(index.has('b'))rows.push({key:'node:b',role:'node',index:index.get('b'),active:active[index.get('b')],rect:projectedRect([nodes[index.get('b')].position[0]-10,nodes[index.get('b')].position[1]+30,90,22]),transform:null});
 const lc=links.find(l=>raw[l.target].id==='c');if(lc)rows.push({key:'link:c',role:'link',index:lc.index,active:linkActive[lc.index],rect:projectedRect([lc.position[0]-50,lc.position[1]-26,100,22]),transform:null});
 for(const l of labels){if(l.owner==='d')continue;const offset=[l.label.anchor==='Start'?0:l.label.anchor==='End'?-48:-24,-14],anchor=project(l.label.position,view),m=[cos*view[0],sin*view[0],-sin*view[0],cos*view[0],anchor[0]+(cos*offset[0]-sin*offset[1])*view[0],anchor[1]+(sin*offset[0]+cos*offset[1])*view[0]],corners=[[0,0],[48,0],[48,18],[0,18]].map(([x,y])=>[m[0]*x+m[2]*y+m[4],m[1]*x+m[3]*y+m[5]]),xs=corners.map(p=>p[0]),ys=corners.map(p=>p[1]),rect=[Math.min(...xs),Math.min(...ys),Math.max(...xs)-Math.min(...xs),Math.max(...ys)-Math.min(...ys)];
  rows.push({key:'tree-label:'+l.owner,role:'label',index:index.get(l.owner),labelIndex:l.index,label:l.label,active:active[index.get(l.owner)],rect,pose:{matrix:m,measured:[48,18]},transform:rotation!==0||view[0]!==1?{matrix:m,measured:[48,18]}:null,clip:phase===10&&l.owner==='c'?projectedRect([l.label.position[0]+8,l.label.position[1]-12,20,12]):null,sourceClip:phase===10&&l.owner==='c'?[l.label.position[0]+8,l.label.position[1]-12,20,12]:null,transparent:l.label.color[3]===0});
 }
 // Plot apertures are viewport-fixed while source geometry follows the camera.
 for(const row of rows){row.visible=clip(row.rect,row.clip);row.interactive=!!row.visible&&row.visible.every((v,i)=>Math.abs(v-row.rect[i])<=1e-9);}
 return{raw,nodes,links,labels,activity,plot,rows,bfs};
}
export function localPoint(row,p){const m=row.pose?.matrix??row.transform?.matrix;if(m)return[m[0]*p[0]+m[2]*p[1]+m[4],m[1]*p[0]+m[3]*p[1]+m[5]];return[row.rect[0]+p[0],row.rect[1]+p[1]];}
