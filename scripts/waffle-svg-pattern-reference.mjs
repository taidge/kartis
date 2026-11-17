// Independent native-SVG pattern raster reference. Never accepts Chart DOM/data/PNG.
import assert from 'node:assert/strict';
import {writeFile,readFile} from 'node:fs/promises';
import {join} from 'node:path';
import {createHash} from 'node:crypto';
import {png} from './acceptance-png.mjs';

export const wafflePatternReferenceFiles=['scripts/waffle-svg-pattern-reference.mjs','scripts/acceptance-png.mjs'];
const hash=b=>createHash('sha256').update(b).digest('hex');
const inside=(p,[x,y])=>{let n=false;for(let i=0,j=p.length-1;i<p.length;j=i++)if((p[i][1]>y)!==(p[j][1]>y)&&x<(p[j][0]-p[i][0])*(y-p[i][1])/(p[j][1]-p[i][1])+p[i][0])n=!n;return n;};
const rgba=(image,x,y)=>[...image.data.subarray(4*(y*image.width+x),4*(y*image.width+x)+4)];
const css=c=>`rgba(${c[0]},${c[1]},${c[2]},${c[3]/255})`;

function authorModel(profile,state){
 assert(Number.isInteger(profile)&&profile>=0&&profile<4);
 assert.deepEqual(state.size,[600,420]);
 const ids=['alpha','beta','hidden','zero','overflow'];
 let rows=ids.map((id,i)=>({id,i,value:(state.phase===3?[12,10,2,-0,4]:[5,4,2,-0,1])[i]}));
 if(state.phase===4)rows=[];
 if(state.phase===5)[rows[0],rows[1]]=[rows[1],rows[0]];
 if(state.phase===6)rows.shift();
 const colors=state.phase===1?[[220,120,20,255],[20,170,80,255],[100,110,120,255],[160,80,190,255],[30,180,180,255]]:[[200,40,60,255],[30,100,210,255],[100,110,120,255],[160,80,190,255],[30,180,180,255]];
 let cursor=0;const groups=rows.map(r=>{const start=cursor;if(!state.hidden.includes(r.id))cursor+=Math.round(r.value/(20/15));return{...r,start:Math.min(start,15),end:Math.min(cursor,15),color:colors[r.i]};});
 const pitch=106,grid=[40,49];
 const cells=Array.from({length:15},(_,index)=>{
  const [row,column]=profile===0?[2-Math.floor(index/5),4-index%5]:profile===1?[index%3,Math.floor(index/3)]:profile===2?[Math.floor(index/5),index%5]:[2-index%3,4-Math.floor(index/3)],center=[grid[0]+(column+.5)*pitch,grid[1]+(row+.5)*pitch],radius=pitch/3,group=groups.find(g=>g.start<=index&&index<g.end);
  return{index,row,column,center,triangle:[[center[0]-radius,center[1]+radius],[center[0],center[1]-radius],[center[0]+radius,center[1]+radius]],color:group?.color??[210,215,220,255],occupied:!!group};
 });
 const cell=cells.find(c=>c.index!==0&&c.occupied);assert(cell,'author state must have an occupied non-button cell');
 const distance=(p,a,b)=>{const v=[b[0]-a[0],b[1]-a[1]],q=Math.max(0,Math.min(1,((p[0]-a[0])*v[0]+(p[1]-a[1])*v[1])/(v[0]**2+v[1]**2)));return Math.hypot(p[0]-a[0]-q*v[0],p[1]-a[1]-q*v[1]);};
 const select=offset=>{const candidates=[];for(let y=Math.ceil((cell.center[1]-pitch/3)/8)*8+offset;y<cell.center[1]+pitch/3;y+=8)for(let x=Math.ceil((cell.center[0]-pitch/3)/8)*8+offset;x<cell.center[0]+pitch/3;x+=8){const point=[x,y],clearance=Math.min(...cell.triangle.map((p,i)=>distance(point,p,cell.triangle[(i+1)%3])));if(inside(cell.triangle,point)&&clearance>5)candidates.push({point,clearance,distanceToCenter:(x-cell.center[0])**2+(y-cell.center[1])**2});}candidates.sort((a,b)=>a.distanceToCenter-b.distanceToCenter||a.point[1]-b.point[1]||a.point[0]-b.point[0]);assert(candidates.length);return candidates[0];};
 const foreground=select(4),background=select(0),tileOrigin=foreground.point.map(v=>Math.floor(v/8)*8),corners=[[0,0],[8,0],[0,8],[8,8]].map(p=>p.map((v,i)=>v+tileOrigin[i]));
 assert(corners.every(p=>inside(cell.triangle,p)),'whole authored pattern tile lies strictly within one triangle');
 return{pitch,groups,cells,cell,foreground,background,tileOrigin,tileCorners:corners,border:state.phase===1?[0,0,0,255]:[100,30,180,255]};
}

export async function renderWafflePatternReference({browser,engine,dpr,profile,state,outDir,key}){
 const authorState={phase:state.phase,hidden:[...state.hidden],size:[...state.size],view:[...state.view],css:[...state.css]},model=authorModel(profile,authorState);
 // This fixed full-tile protocol is deliberately limited to the tested native
 // pattern pose, rather than inventing resampling rules for arbitrary cameras.
 assert.equal(authorState.view[0],1);assert.equal(authorState.view[1],0);assert(authorState.view[2]===0);assert.deepEqual(authorState.css,[1,0]);assert([1,2].includes(dpr));
 const inputs={protocol:'independent-waffle-native-pattern-v1',engine,dpr,profile,state:authorState,scene:[600,420],root:[80,24,600,420],foreground:[20,20,20,255],spacing:8,radius:2,model};
 const summary={inputs,cases:[],completed:false,featureAccepted:false,broadGoalComplete:false};
 const context=await browser.newContext({viewport:{width:1400,height:900},deviceScaleFactor:dpr});
 const filePrefix=key+'-native-pattern-reference';
 let failure;
 try{
  for(const variant of ['base','direct-circle','wrong-foreground','wrong-spacing','missing-circle']){
   const page=await context.newPage(),record={variant,completed:false};summary.cases.push(record);
   try{
    const fg=variant==='wrong-foreground'?[80,160,40,255]:inputs.foreground,spacing=variant==='wrong-spacing'?10:8;
    const defs=model.cells.map(c=>`<pattern id="pattern-${c.index}" patternUnits="userSpaceOnUse" patternContentUnits="userSpaceOnUse" overflow="hidden" width="${spacing}" height="${spacing}" patternTransform="matrix(1 0 -0 1 0 0)"><rect width="${spacing}" height="${spacing}" fill="${css(c.color)}"/>${variant==='missing-circle'?'':`<circle cx="${spacing/2}" cy="${spacing/2}" r="2" fill="${css(fg)}"/>`}</pattern>`).join('');
    const paths=model.cells.map(c=>`<path data-reference-cell="${c.index}" d="M${c.triangle[0].join(' ')}L${c.triangle[1].join(' ')}L${c.triangle[2].join(' ')}Z" fill="${variant==='direct-circle'&&c.index===model.cell.index?css(c.color):`url(#pattern-${c.index})`}" stroke="${css(model.border)}" stroke-width="2" opacity="1"/>`).join('');
    const direct=variant==='direct-circle'?`<circle data-reference-direct="true" cx="${model.foreground.point[0]}" cy="${model.foreground.point[1]}" r="2" fill="${css(inputs.foreground)}"/>`:'';
    const native=`<foreignObject x="0" y="0" width="600" height="420" pointer-events="none"><div xmlns="http://www.w3.org/1999/xhtml" data-reference-native="true" style="position:relative;width:100%;height:100%;overflow:clip;isolation:isolate;pointer-events:none;will-change:transform"><svg xmlns="http://www.w3.org/2000/svg" width="100%" height="100%" viewBox="0 0 600 420" preserveAspectRatio="none" overflow="hidden" pointer-events="none"><defs>${defs}</defs><rect width="100%" height="100%" fill="rgba(255,255,255,0)"/><g transform="translate(0 0) scale(1)">${paths}${direct}</g></svg></div></foreignObject>`;
    const html=`<!doctype html><meta charset="utf-8"><style>body{margin:0;background:white}</style><section data-reference-root="true" style="position:relative;width:600px;height:420px;margin:24px 0 0 80px;transform-origin:0 0"><div style="position:relative;width:100%;height:100%;overflow:visible"><div style="position:relative;width:100%;height:100%;overflow:clip;isolation:isolate"><svg xmlns="http://www.w3.org/2000/svg" style="position:absolute;left:0;top:0;visibility:visible;overflow:hidden;opacity:1" width="600" height="420" viewBox="0 0 600 420"><rect width="100%" height="100%" fill="white"/>${native}</svg></div></div></section>`;
    record.html=filePrefix+'-'+variant+'.html';await writeFile(join(outDir,record.html),html,{flag:'wx'});record.htmlSHA256=hash(Buffer.from(html));
    await page.setContent(html);await page.evaluate(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))));
    record.dom=await page.evaluate(index=>{const rect=n=>{const b=n.getBoundingClientRect();return[b.x,b.y,b.width,b.height];},target=document.querySelector(`[data-reference-cell="${index}"]`),box=target.getBBox(),m=target.getScreenCTM(),container=document.querySelector('[data-reference-native]');return{dpr:devicePixelRatio,root:rect(document.querySelector('[data-reference-root]')),native:rect(container),willChange:getComputedStyle(container).willChange,target:{html:target.outerHTML,rect:rect(target),bbox:[box.x,box.y,box.width,box.height],ctm:[m.a,m.b,m.c,m.d,m.e,m.f]},patterns:[...document.querySelectorAll('pattern')].map(p=>p.outerHTML),html:document.body.innerHTML};},model.cell.index);
    assert.deepEqual(record.dom.root,inputs.root);assert.deepEqual(record.dom.native,inputs.root);assert.equal(record.dom.dpr,dpr);assert.equal(record.dom.willChange,'transform');assert.deepEqual(record.dom.target.ctm,[1,0,0,1,80,24]);
    const bytes=await page.screenshot({type:'png',scale:'device',caret:'initial',animations:'allow'});record.png=filePrefix+'-'+variant+'.png';record.pngSHA256=hash(bytes);await writeFile(join(outDir,record.png),bytes,{flag:'wx'});const image=png(bytes);assert.equal(image.width,1400*dpr);assert.equal(image.height,900*dpr);
    record.selected=Object.fromEntries(['foreground','background'].map(k=>{const point=model[k].point,index=point.map((v,i)=>Math.floor((v+[80,24][i])*dpr));return[k,{point,index,color:rgba(image,...index)}];}));
    const origin=model.tileOrigin.map((v,i)=>(v+[80,24][i])*dpr),size=8*dpr;record.tile={origin,size:[size,size],rgba:Array.from({length:size},(_,y)=>Array.from({length:size},(_,x)=>rgba(image,origin[0]+x,origin[1]+y)))};
    if(variant==='direct-circle')assert.deepEqual(record.selected.foreground.color,inputs.foreground,'opaque independent direct circle center positive');
    if(variant==='missing-circle')for(const line of record.tile.rgba)for(const color of line)assert.deepEqual(color,model.cell.color,'circle-free reference tile is exactly background');
    record.completed=true;
   }catch(error){record.error=error.stack??String(error);throw error;}finally{record.json=filePrefix+'-'+variant+'.json';await writeFile(join(outDir,record.json),JSON.stringify(record,null,2)+'\n',{flag:'wx'});await page.close();}
  }
  const base=summary.cases[0];summary.controls=[];
  for(const variant of ['wrong-foreground','wrong-spacing','missing-circle']){const other=summary.cases.find(c=>c.variant===variant);let error;try{assert.deepEqual(other.tile.rgba,base.tile.rgba,'strict independent native pattern tile');}catch(e){error=e;}assert(error instanceof assert.AssertionError,'actual reference mutation must fail the original exact tile comparison');summary.controls.push({variant,rejected:true,errorName:error.name,basePNG:base.png,mutatedPNG:other.png});}
  summary.completed=true;
 }catch(error){failure=error;summary.error=error.stack??String(error);}finally{await context.close();summary.json=filePrefix+'.json';await writeFile(join(outDir,summary.json),JSON.stringify(summary,null,2)+'\n',{flag:'wx'});}
 if(failure)throw failure;
 return{json:summary.json,sha256:hash(await readFile(join(outDir,summary.json))),inputs,cases:summary.cases,controls:summary.controls,completed:summary.completed};
}
