(async function verifyContourDensityGallery(page) {
  const [{readFile},{resolve},{createHash},vm] = await Promise.all([
    import('node:fs/promises'), import('node:path'), import('node:crypto'), import('node:vm'),
  ]);
  const assert = (condition,message) => { if (!condition) throw Error(`Contour/Density gallery: ${message}`); };
  const sha = bytes => createHash('sha256').update(bytes).digest('hex');
  const fixtures = resolve('tests/fixtures');
  const manifestBytes = await readFile(resolve(fixtures,'contour-density-d3-source.json'));
  const manifest = JSON.parse(manifestBytes);
  const bundle = await readFile(resolve(fixtures,manifest.bundle.file));
  assert(bundle.length===manifest.bundle.bytes && sha(bundle)===manifest.bundle.sha256,'original D3 bundle integrity');
  assert(JSON.stringify(manifest.packages.map(p=>[p.name,p.version]))===JSON.stringify([['d3-array','3.2.4'],['d3-contour','4.0.2']]),'original package versions');
  for (const p of manifest.packages) assert(sha(bundle.subarray(p.start,p.start+p.bytes))===p.sha256,`original ${p.name} bytes`);
  const licenses = await readFile(resolve(fixtures,manifest.licenses.file));
  assert(sha(licenses)===manifest.licenses.sha256,'complete original licenses');
  for (const p of manifest.licenses.packages) assert(sha(p.text)===p.originalSha256 && licenses.toString().includes(p.text),`complete ${p.name} license`);
  const context = vm.createContext({});
  new vm.Script(bundle.toString(),{filename:'unmodified-d3-contour-gallery-oracle.js'}).runInContext(context,{timeout:5000});
  const d3=context.d3, plot=[70,30,620,260], white=[255,255,255,255];
  const close=(a,b)=>Number.isFinite(a)&&Number.isFinite(b)&&Math.abs(a-b)<=2e-12*Math.max(1,Math.abs(b));
  const equalPoint=(a,b)=>close(a[0],b[0])&&close(a[1],b[1]);
  const rgba = color => {const n=color.match(/[+-]?(?:\d*\.?\d+)(?:e[+-]?\d+)?/gi)?.map(Number);assert(n?.length===4,'RGBA source paint');return [...n.slice(0,3),Math.round(n[3]*255)];};
  const colorAt=(palette,t)=>palette[0].map((v,i)=>Math.round(v*(1-t)+palette[1][i]*t)).concat(255);
  const bits = value => {const b=new DataView(new ArrayBuffer(8));b.setFloat64(0,value===0?0:value,false);return b.getBigUint64(0,false).toString(16).padStart(16,'0');};
  const id = value => `contour:7:contour:${bits(value)}:0`;
  const gridValues=Array.from({length:225},(_,i)=>Math.floor(Math.min(i%15,Math.floor(i/15),14-i%15,14-Math.floor(i/15))/2)%2===0?2:0);
  const grid=d3.contours().size([15,15]).thresholds([.5,1,1.5])(gridValues);
  const raw=[];
  for (let group=0;group<2;group++) for(let index=0;index<100;index++) {
    const angle=index*2.399963229728653,radius=Math.sqrt(index/100);
    const x=(group===0?30:73)+radius*12*Math.cos(angle),y=(group===0?65:31)+radius*18*Math.sin(angle);
    const series=group===0?'Coastal':'Inland',node=`sample-${index}`;
    raw.push({id:`scatter-node:${series.length}:${series}:${node.length}:${node}`,series,node,x,y,
      position:[plot[0]+x/100*plot[2],plot[1]+(100-y)/100*plot[3]],weight:group===0?1:.65,
      color:group===0?[32,91,156,255]:[219,111,35,255]});
  }
  const density=d3.contourDensity().size(plot.slice(2)).bandwidth(18).cellSize(4).weight(p=>p[2]).thresholds(6)(raw.map(p=>[p.position[0]-plot[0],p.position[1]-plot[1],p.weight]));
  const project=(geometry,grid)=>geometry.coordinates.flatMap(polygon=>polygon.map(ring=>ring.map(p=>grid?[plot[0]+p[0]/15*plot[2],plot[1]+p[1]/15*plot[3]]:[plot[0]+p[0],plot[1]+p[1]])));
  const parsePath = text => {
    const token=/[MLZ]|[-+]?(?:\d+\.?\d*|\.\d+)(?:[eE][-+]?\d+)?/g;
    assert(text.replace(token,'').replace(/[\s,]/g,'')==='','linear contour commands only');
    const t=text.match(token)??[],rings=[];let current=null;
    for(let i=0;i<t.length;){const command=t[i++];if(command==='Z'){assert(current?.length>=3,'closed contour ring');current.push(current[0]);rings.push(current);current=null;}else{assert(command==='M'||command==='L','contour command sequence');const point=[Number(t[i++]),Number(t[i++])];assert(point.every(Number.isFinite),'finite contour endpoint');if(command==='M'){assert(current===null,'ring starts after close');current=[point];}else{assert(current!==null,'line follows move');current.push(point);}}}
    assert(current===null,'no unfinished ring');return rings;
  };
  const signedArea=ring=>ring.slice(1).reduce((sum,p,i)=>sum+ring[i][0]*p[1]-p[0]*ring[i][1],0)/2;
  const sameRing=(actual,expected)=>{
    if(actual.length!==expected.length||!equalPoint(actual[0],actual.at(-1)))return false;
    const n=expected.length-1;
    for(let offset=0;offset<n;offset++)if(equalPoint(actual[0],expected[offset])&&actual.slice(0,-1).every((p,i)=>equalPoint(p,expected[(i+offset)%n])))return true;
    return false;
  };
  const compareRings=(actual,expected,label)=>{
    assert(actual.length===expected.length,`${label} ring count including holes`);
    const left=expected.slice();let vertices=0;
    for(const ring of actual){const index=left.findIndex(other=>sameRing(ring,other));assert(index>=0,`${label} every ordered vertex and winding within original 2e-12 tolerance`);vertices+=ring.length;left.splice(index,1);}
    assert(left.length===0,`${label} complete geometry`);return vertices;
  };
  const contains=(ring,p)=>{let inside=false;for(let i=0,j=ring.length-1;i<ring.length;j=i++){const a=ring[i],b=ring[j];if((a[1]>p[1])!==(b[1]>p[1])&&p[0]<(b[0]-a[0])*(p[1]-a[1])/(b[1]-a[1])+a[0])inside=!inside;}return inside;};
  const filled=(geometry,p)=>geometry.coordinates.some(poly=>contains(poly[0],p)&&!poly.slice(1).some(r=>contains(r,p)));
  const segmentDistance=(p,a,b)=>{const dx=b[0]-a[0],dy=b[1]-a[1],length=dx*dx+dy*dy,t=length?Math.max(0,Math.min(1,((p[0]-a[0])*dx+(p[1]-a[1])*dy)/length)):0;return Math.hypot(p[0]-a[0]-t*dx,p[1]-a[1]-t*dy);};
  const densityPalette=[[243,237,226],[125,98,161]],gridPalette=[[205,236,234],[12,104,127]];
  const domain=[density[0].value,density.at(-1).value];
  const densityWitness=[];
  for(let level=0;level<density.length;level++) {
    let witness;
    search:for(let y=6;y<plot[3]-6;y+=3)for(let x=6;x<plot[2]-6;x+=3){const p=[x,y];
      let top=-1;for(let k=0;k<density.length;k++)if(filled(density[k],p))top=k;
      if(top!==level||raw.some(r=>Math.hypot(r.position[0]-plot[0]-x,r.position[1]-plot[1]-y)<7))continue;
      if(density.some(g=>g.coordinates.some(poly=>poly.some(r=>r.slice(1).some((b,i)=>segmentDistance(p,r[i],b)<2)))))continue;
      witness=[x+plot[0],y+plot[1]];break search;
    }
    assert(witness,`original D3 level ${level} interior paint witness`);
    densityWitness.push({point:witness,color:colorAt(densityPalette,(density[level].value-domain[0])/(domain[1]-domain[0])),level});
  }
  const gridWitness=[[.5,7.5,true],[2.5,7.5,false],[4.5,7.5,true],[7.5,7.5,false]].map(([x,y,solid])=>({point:[plot[0]+x/15*plot[2],plot[1]+y/15*plot[3]],color:solid?colorAt(gridPalette,.75):white,solid}));
  const samples={grid:gridWitness.map(w=>w.point),density:[...densityWitness.map(w=>w.point),raw[0].position,raw[100].position]};
  const actual=await page.evaluate(samples=>{
    const card=(name,index,points)=>{
      const article=[...document.querySelectorAll('article')].find(n=>n.querySelector('h2')?.textContent===name);if(!article)throw Error(`Missing ${name}`);
      const svg=article.querySelector('.svg-view>svg'),canvas=article.querySelector('canvas'),ctx=canvas.getContext('2d');
      const paths=[...svg.querySelectorAll('[data-id]')].map((p,order)=>{const clip=p.closest('svg');return{id:p.dataset.id,series:p.dataset.series,value:Number(p.dataset.value),d:p.getAttribute('d'),fill:p.getAttribute('fill'),opacity:p.getAttribute('opacity'),fillRule:getComputedStyle(p).fillRule,order,clip:{overflow:clip.getAttribute('overflow'),x:clip.getAttribute('x'),y:clip.getAttribute('y'),width:clip.getAttribute('width'),height:clip.getAttribute('height'),viewBox:clip.getAttribute('viewBox')}};});
      const pixel=(x,y)=>[...ctx.getImageData(Math.floor(x/720*canvas.width),Math.floor(y/400*canvas.height),1,1).data];
      const hit=points.map(point=>[...svg.querySelectorAll('[data-id^="contour:"]')].map(p=>p.isPointInFill(new DOMPoint(...point))));
      return{index:Number(article.dataset.index),expectedIndex:index,viewBox:svg.getAttribute('viewBox'),paths,text:[...svg.querySelectorAll('text')].map(t=>({text:t.textContent,x:+t.getAttribute('x'),y:+t.getAttribute('y')})),gradients:[...svg.querySelectorAll('linearGradient')].map(g=>({id:g.id,units:g.getAttribute('gradientUnits'),x1:+g.getAttribute('x1'),y1:+g.getAttribute('y1'),x2:+g.getAttribute('x2'),y2:+g.getAttribute('y2'),stops:[...g.querySelectorAll('stop')].map(s=>({offset:+s.getAttribute('offset'),color:s.getAttribute('stop-color')}))})),pixels:points.map(p=>pixel(...p)),hit,backing:[canvas.width,canvas.height]};
    };return{grid:card('Contour · nested grid holes',239,samples.grid),density:card('Density · weighted scatter observations',240,samples.density)};
  },samples);
  const checkClip=path=>assert(path.clip.overflow==='hidden'&&+path.clip.x===70&&+path.clip.y===30&&+path.clip.width===620&&+path.clip.height===260&&path.clip.viewBox==='70 30 620 260','actual plot clipping frame');
  const verifyContours=(chart,geometry,isGrid,palette,colorDomain)=>{
    const paths=chart.paths.filter(p=>p.id.startsWith('contour:'));assert(paths.length===geometry.length,'complete original D3 levels');let vertices=0,holes=0,polygons=0;
    for(let i=0;i<paths.length;i++){const p=paths[i],expected=geometry[i];assert(p.id===id(expected.value)&&p.value===expected.value,'original threshold metadata and stable ID');checkClip(p);assert(p.opacity==='1'&&p.fillRule==='nonzero','opaque nonzero hole paint');const rings=parsePath(p.d),reference=project(expected,isGrid);vertices+=compareRings(rings,reference,`${isGrid?'grid':'density'} threshold ${expected.value}`);const outerSign=Math.sign(signedArea(reference[0]));const expectedHoles=expected.coordinates.reduce((sum,p)=>sum+p.length-1,0);assert(rings.filter(r=>Math.sign(signedArea(r))!==outerSign).length===expectedHoles,'hole winding and ownership');holes+=expectedHoles;polygons+=expected.coordinates.length;assert(JSON.stringify(rgba(p.fill))===JSON.stringify(colorAt(palette,(expected.value-colorDomain[0])/(colorDomain[1]-colorDomain[0]))),'threshold colors from declared numeric domain');}
    return{levels:paths.length,vertices,polygons,holes,paths};
  };
  const gridResult=verifyContours(actual.grid,grid,true,gridPalette,[0,2]);
  const densityResult=verifyContours(actual.density,density,false,densityPalette,domain);
  assert(gridResult.levels===3&&gridResult.polygons===6&&gridResult.holes===6,'three independent nested-hole layers');
  const verifyAxes=(chart,prefix,values,horizontal,domain)=>{
    const paths=chart.paths.filter(p=>p.id.startsWith(`${prefix}-tick-`));assert(paths.length===values.length,'axis tick count');
    paths.forEach((p,i)=>{const match=/^M([^ ]+) ([^L ]+)/.exec(p.d);assert(match,'axis move endpoint');const coordinate=Number(match[horizontal?1:2]),expected=horizontal?plot[0]+values[i]/domain*plot[2]:plot[1]+(domain===100?1-values[i]/domain:values[i]/domain)*plot[3];assert(close(coordinate,expected),'independent raw-domain axis position');});
  };
  verifyAxes(actual.grid,'grid-column',[0,5,10,15],true,15);verifyAxes(actual.grid,'grid-row',[0,5,10,15],false,15);
  verifyAxes(actual.density,'scatter-axis-bottom',[0,20,40,60,80,100],true,100);verifyAxes(actual.density,'scatter-axis-left',[0,20,40,60,80,100],false,100);
  const verifyLegend=(chart,palette,title)=>{
    assert(chart.gradients.length===1,'real continuous legend gradient');const g=chart.gradients[0],band=chart.paths.find(p=>p.id==='color-legend:12:color-legend:band:0');assert(band?.fill===`url(#${g.id})`,'legend uses its local gradient');assert(g.units==='userSpaceOnUse'&&g.x1===110&&g.x2===610&&g.y1===364&&g.y2===364,'legend paint coordinates');assert(g.stops.length===16,'full shared ramp samples');g.stops.forEach((s,i)=>assert(close(s.offset,i/15)&&JSON.stringify(rgba(s.color))===JSON.stringify(colorAt(palette,i/15)),'independent legend ramp colors'));assert(chart.text.some(t=>t.text===title),'visible legend title');
  };
  verifyLegend(actual.grid,gridPalette,'Contour threshold');verifyLegend(actual.density,densityPalette,'Weight per square pixel');
  assert(['Grid column','Grid row'].every(t=>actual.grid.text.some(v=>v.text===t))&&['Observed x','Observed y'].every(t=>actual.density.text.some(v=>v.text===t)),'visible raw-axis labels');
  const verifySources=(paths)=>{
    const sources=paths.filter(p=>p.id.startsWith('scatter-node:'));assert(sources.length===200,'all 200 source glyphs');
    sources.forEach((p,i)=>{const expected=raw[i],match=/^M([^ ]+) ([^C ]+)/.exec(p.d);assert(p.id===expected.id&&p.series===expected.series&&close(p.value,expected.y),'original raw source identity and value');const [x,y]=expected.position,r=2.5,k=r*4*(Math.sqrt(2)-1)/3,circle=[x+r,y,x+r,y+k,x+k,y+r,x,y+r,x-k,y+r,x-r,y+k,x-r,y,x-r,y-k,x-k,y-r,x,y-r,x+k,y-r,x+r,y-k,x+r,y],coordinates=(p.d.match(/[-+]?(?:\d+\.?\d*|\.\d+)(?:[eE][-+]?\d+)?/g)??[]).map(Number);assert(match&&close(+match[1]-2.5,x)&&close(+match[2],y)&&(p.d.match(/[MCZ]/g)??[]).join('')==='MCCCCZ'&&coordinates.length===circle.length&&coordinates.every((v,j)=>close(v,circle[j])),'independent raw XY and every five-pixel cubic-circle endpoint/control point');assert(JSON.stringify(rgba(p.fill))===JSON.stringify(expected.color)&&p.opacity==='1','source color');checkClip(p);});
    assert(Math.min(...sources.map(p=>p.order))>Math.max(...paths.filter(p=>p.id.startsWith('contour:')).map(p=>p.order)),'raw glyph foreground order');return sources.length;
  };
  const sourceCount=verifySources(actual.density.paths);
  for(const name of ['grid','density'])assert(actual[name].index===actual[name].expectedIndex&&actual[name].viewBox==='0 0 720 400'&&actual[name].backing[0]>0&&actual[name].backing[0]/720===actual[name].backing[1]/400,'appended gallery identity and canvas backing');
  gridWitness.forEach((w,i)=>{assert(JSON.stringify(actual.grid.pixels[i])===JSON.stringify(w.color),'actual standalone Canvas grid ink/hole pixel');assert(actual.grid.hit[i].every(v=>v===w.solid),'actual SVG filled islands and holes');});
  densityWitness.forEach((w,i)=>{assert(JSON.stringify(actual.density.pixels[i])===JSON.stringify(w.color),'original D3 tier standalone Canvas ink');assert(actual.density.hit[i][w.level]&&!actual.density.hit[i].slice(w.level+1).some(Boolean),'actual SVG tier fill');});
  [raw[0],raw[100]].forEach((p,i)=>assert(JSON.stringify(actual.density.pixels[densityWitness.length+i])===JSON.stringify(p.color),'actual source glyph foreground Canvas pixel'));
  const reject=run=>{let rejected=false;try{run();}catch{rejected=true;}assert(rejected,'negative control must reject');return true;};
  const rings=parsePath(gridResult.paths[0].d),expected=project(grid[0],true);
  const negativeControls={missingHole:reject(()=>compareRings(rings.slice(1),expected,'removed hole')),shiftedVertex:reject(()=>{const r=structuredClone(rings);r[0][1][0]+=.01;compareRings(r,expected,'shifted vertex');}),missingSource:reject(()=>verifySources(actual.density.paths.filter(p=>p.id!==raw[0].id))),foregroundOrder:reject(()=>verifySources(actual.density.paths.map(p=>p.id===raw[0].id?{...p,order:-1}:p)))};
  return{passed:true,scope:'Standalone gallery SVG geometry and Canvas export pixels; BrowserChart input/backend acceptance is separate',oracle:{versions:manifest.packages.map(p=>({name:p.name,version:p.version})),manifestSha256:sha(manifestBytes),bundleSha256:sha(bundle)},indices:[239,240],coordinateTolerance:'2e-12*max(1,abs(expected))',grid:{levels:gridResult.levels,polygons:gridResult.polygons,holes:gridResult.holes,vertices:gridResult.vertices,axesTicks:8,legendStops:16,pixels:actual.grid.pixels},density:{levels:densityResult.levels,polygons:densityResult.polygons,holes:densityResult.holes,vertices:densityResult.vertices,sourceGlyphs:sourceCount,axesTicks:12,legendStops:16,originalD3PaintWitness:densityWitness,pixels:actual.density.pixels},negativeControls};
})
