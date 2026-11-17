// Original d3-geo 3.1.1 Albers USA multiplexed polygon stream and fit oracle.
// ISC, Copyright 2010-2024 Mike Bostock; preserved notice:
// tests/fixtures/geo-inverse-d3-LICENSE.txt. This is verification-only code.
import assert from 'node:assert/strict';
import { readFile, writeFile, mkdir, readdir } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { join, resolve, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const check=process.argv.includes('--check'),root=resolve('.');
const metadata=JSON.parse(execFileSync('cargo',['metadata','--no-deps','--format-version','1'],{encoding:'utf8'}));
const modules=join(metadata.target_directory,'d3-geo-reference','node_modules');
const base=join(modules,'d3-geo');
assert.equal(JSON.parse(await readFile(join(base,'package.json'),'utf8')).version,'3.1.1');
const d3=await import(pathToFileURL(join(base,'src/index.js')));
const sha=b=>createHash('sha256').update(b).digest('hex');
const precision=.02,boundaryBand=.05;
const area=ring=>ring.reduce((v,p,i)=>{const q=ring[(i+1)%ring.length];return v+p[0]*q[1]-q[0]*p[1];},0)/2;
const perimeter=rings=>rings.reduce((n,ring)=>n+ring.reduce((s,p,i)=>{const q=ring[(i+1)%ring.length];return s+Math.hypot(p[0]-q[0],p[1]-q[1]);},0),0);
function bounds(rings){let b=[Infinity,Infinity,-Infinity,-Infinity];for(const r of rings)for(const [x,y]of r){b=[Math.min(b[0],x),Math.min(b[1],y),Math.max(b[2],x),Math.max(b[3],y)];}return b[0]===Infinity?null:b;}
function close(r){return r.length&&r[0][0]===r.at(-1)[0]&&r[0][1]===r.at(-1)[1]?r:[...r,r[0]];}
function lonRect(x0,y0,x1,y1){return close([[x0,y0],[x0,y1],[x1,y1],[x1,y0]]);}
function feature(id,value,polygons){return {id,label:`Source ${id}`,value,polygons};}
function object(f){return {type:'Feature',id:f.id,properties:{value:f.value},geometry:{type:'MultiPolygon',coordinates:f.polygons}};}
function collection(features){return {type:'FeatureCollection',features:features.map(object)};}
function capture(projection,object){
 let current=null,rings=[];
 const sink={point(x,y){assert(current,'unexpected point outside polygon line');current.push([x,y]);},lineStart(){assert.equal(current,null);current=[];},lineEnd(){rings.push(current);current=null;},polygonStart(){},polygonEnd(){}};
 d3.geoStream(object,projection.stream(sink));
 assert.equal(current,null);return rings;
}
function winding(rings,[x,y]){let n=0;for(const r of rings)for(let i=0;i<r.length;i++){const a=r[i],b=r[(i+1)%r.length],cross=(b[0]-a[0])*(y-a[1])-(b[1]-a[1])*(x-a[0]);if(a[1]<=y){if(b[1]>y&&cross>0)n++;}else if(b[1]<=y&&cross<0)n--;}return n!==0;}
function distance(rings,p){let d=Infinity;for(const r of rings)for(let i=0;i<r.length;i++){const a=r[i],b=r[(i+1)%r.length],dx=b[0]-a[0],dy=b[1]-a[1],length=dx*dx+dy*dy,t=length===0?0:Math.max(0,Math.min(1,((p[0]-a[0])*dx+(p[1]-a[1])*dy)/length));d=Math.min(d,Math.hypot(p[0]-a[0]-t*dx,p[1]-a[1]-t*dy));}return d;}
function regions(k,[x,y]){const e=1e-6;return [
 {region:'Lower48',rect:[x-.455*k,y-.238*k,x+.455*k,y+.238*k]},
 {region:'Alaska',rect:[x-.425*k+e,y+.120*k+e,x-.214*k-e,y+.234*k-e]},
 {region:'Hawaii',rect:[x-.214*k+e,y+.166*k+e,x-.115*k-e,y+.234*k-e]},
];}
const nativeAk=d3.geoConicEqualArea().rotate([154,0]).center([-2,58.5]).parallels([55,65]).scale(150*.35).translate([-.307*150,.201*150]);
const [ak]=regions(150,[0,0]).filter(r=>r.region==='Alaska');
function akPlaneRing(uv){return close(uv.map(([u,v])=>{const p=nativeAk.invert([ak.rect[0]+u/10*(ak.rect[2]-ak.rect[0]),ak.rect[1]+v/10*(ak.rect[3]-ak.rect[1])]);assert(p.every(Number.isFinite)&&Math.abs(p[0])<=180&&Math.abs(p[1])<=90);return p;}));}
const lower=lonRect(-120,30,-90,45),alaska=lonRect(-160,55,-140,65),hawaii=lonRect(-160.5,18.5,-154.5,22.5);
const outer=akPlaneRing([[-2,-2],[12,-2],[12,12],[-2,12]]);
const holes=[akPlaneRing([[-.5,2],[-.5,4],[3,4],[3,2]]),akPlaneRing([[7,6],[7,8],[10.5,8],[10.5,6]])];
for(const hole of holes)for(const point of hole)assert(d3.geoContains({type:'Polygon',coordinates:[outer]},point),'source hole must be contained before projection clipping');
const groups=[
 {name:'three-regions',features:[feature('all-regions',37,[[lower],[alaska],[hawaii]])]},
 {name:'antimeridian',features:[feature('aleutians',11,[[lonRect(170,50,-170,63)]]),feature('mainland',5,[[lonRect(-105,35,-95,45)]])]},
 {name:'concave-boundary',features:[feature('split-shape',23,[[akPlaneRing([[-1,1],[8,1],[8,3],[-.5,3],[-.5,7],[8,7],[8,9],[-1,9]])]])]},
 {name:'holes-boundary',features:[feature('notched-shape',41,[[outer,...holes]])]},
 {name:'color-domain',features:[feature('zero-lower',0,[[lonRect(-108,32,-95,42)]]),feature('ten-alaska',10,[[alaska]]),feature('missing-hawaii',null,[[hawaii]]),feature('invisible-extreme',1000,[[lonRect(1,48,3,49)]])]},
];
for(const g of groups)for(const f of g.features){const a=d3.geoArea(object(f));assert(a>0&&a<2*Math.PI,`${g.name}/${f.id}: expected small correctly wound source, got ${a}`);}
const configs=[
 {width:960,height:600,margin:{left:40,right:40,top:30,bottom:30},padding:12},
 {width:640,height:420,margin:{left:30,right:30,top:20,bottom:20},padding:7},
];
const cases=[];
function add(group,config,fit,explicit=null){
 const {width,height,margin,padding}=config,inner=[width-margin.left-margin.right,height-margin.top-margin.bottom];
 const extent=[[margin.left+padding,margin.top+padding],[width-margin.right-padding,height-margin.bottom-padding]];
 const data=collection(group.features);
 const temp=d3.geoAlbersUsa().scale(150).translate([0,0]).precision(precision),tempBounds=bounds(capture(temp,data));
 assert(tempBounds,'authored fixture needs nonempty native geometry');
 const p=d3.geoAlbersUsa().precision(precision);
 if(fit==='Data')p.fitExtent(extent,data);
 else p.scale(explicit?.scale??Math.min((inner[0]-2*padding)/.91,(inner[1]-2*padding)/.476)).translate(explicit?.translate??[margin.left+inner[0]/2,margin.top+inner[1]/2]);
 const k=p.scale(),translate=p.translate();
 const fitCenter=[margin.left+inner[0]/2,margin.top+inner[1]/2];
 const rawCenter=fit==='Data'?[(tempBounds[0]+tempBounds[2])/300,-(tempBounds[1]+tempBounds[3])/300]:[0,0];
 const delta=2*precision;
 const span=Math.min(tempBounds[2]-tempBounds[0],tempBounds[3]-tempBounds[1]);
 assert(span>2*delta,'fixture fit envelope must be well conditioned');
 const fitRelativeLimit=fit==='Data'?2*delta/(span-2*delta):0;
 assert(fitRelativeLimit<.03,'fixture must not hide poorly conditioned fitting');
 const rs=regions(k,translate);
 const outputs=[];
 for(const f of group.features){
  const rings=capture(p,object(f)),b=bounds(rings),areas=rings.map(area),queries=[];
  const grids=rs.map(r=>r.rect);if(b)grids.push([b[0]-1,b[1]-1,b[2]+1,b[3]+1]);
  for(const g of grids){const r=[Math.max(0,g[0]),Math.max(0,g[1]),Math.min(width,g[2]),Math.min(height,g[3])];if(r[2]<=r[0]||r[3]<=r[1])continue;
   for(let j=0;j<11;j++)for(let i=0;i<17;i++){const q=[r[0]+(i+.37)/17*(r[2]-r[0]),r[1]+(j+.61)/11*(r[3]-r[1])],d=distance(rings,q);queries.push({position:q,inside:winding(rings,q),distance:Number.isFinite(d)?d:null,excluded:d<=boundaryBand});}
  }
  outputs.push({id:f.id,rings,bounds:b,signedArea:areas.reduce((a,b)=>a+b,0),perimeter:perimeter(rings),windingCounts:{positive:areas.filter(a=>a>1e-8).length,negative:areas.filter(a=>a< -1e-8).length,degenerate:areas.filter(a=>Math.abs(a)<=1e-8).length},coverage:queries,coverageCounts:{total:queries.length,excluded:queries.filter(q=>q.excluded).length,included:queries.filter(q=>!q.excluded).length,inside:queries.filter(q=>!q.excluded&&q.inside).length}});
 }
 const pointInputs=[[-74.006,40.7128],[-122.4194,37.7749],[-149.9003,61.2181],[-134.4197,58.3019],[-157.8583,21.3069],[-155.09,19.7297],[178,55],[2.35,48.86]];
 cases.push({name:`${group.name}-${width}-${fit}${explicit?'-explicit':''}`,group:group.name,config:{width,height,margin},options:{fit,padding,scale:explicit?.scale??null,translate:explicit?.translate??null,precision},features:group.features,native:{scale:k,translate,rawCenter,fitCenter:explicit?.translate??fitCenter,tempBounds,regions:rs,features:outputs,points:pointInputs.map(input=>({input,output:p(input)}))},allowances:{fitRelativeLimit,rawCenter:fit==='Data'?delta/150:1e-12,geometryPrecision:precision,boundaryBand,coordinateRoundoff:1e-7,materialContourArea:1e-8}});
}
for(const group of groups)for(const config of configs)for(const fit of ['World','Data'])add(group,config,fit);
for(const group of groups.filter(g=>['three-regions','color-domain'].includes(g.name))){
 add(group,configs[0],'World',{scale:1070,translate:[480,250]});
 add(group,configs[1],'World',{scale:750,translate:[320.25,210.5]});
}
assert.equal(cases.length,24);
for(const c of cases.filter(c=>c.group==='color-domain'))assert.equal(c.native.features.find(f=>f.id==='invisible-extreme').rings.length,0);
const fixture=join(root,'tests/fixtures/geo-usa-map');await mkdir(fixture,{recursive:true});
const value=JSON.stringify({schema:1,d3GeoVersion:'3.1.1',scope:'Original composite multiplex polygon streams and native fitExtent; geometry uses independently sampled paths, not point-count equality. World fit is an explicitly authored normalized-frame affine. Coverage exclusions are retained, counted and fixed at .05 final native pixels before any Rust execution.',precision,boundaryBand,cases},null,2)+'\n';
const sourceSHA256={};
async function inventory(dir,prefix){for(const entry of await readdir(dir,{withFileTypes:true})){const p=join(dir,entry.name);if(entry.isDirectory())await inventory(p,prefix);else if(entry.name.endsWith('.js'))sourceSHA256[relative(prefix,p).replaceAll('\\','/')]=sha(await readFile(p));}}
await inventory(join(base,'src'),base);
const counts={cases:cases.length,featureStates:cases.reduce((n,c)=>n+c.features.length,0),contours:cases.reduce((n,c)=>n+c.native.features.reduce((m,f)=>m+f.rings.length,0),0),coverageQueries:cases.reduce((n,c)=>n+c.native.features.reduce((m,f)=>m+f.coverageCounts.total,0),0),coverageExcluded:cases.reduce((n,c)=>n+c.native.features.reduce((m,f)=>m+f.coverageCounts.excluded,0),0),coverageIncluded:cases.reduce((n,c)=>n+c.native.features.reduce((m,f)=>m+f.coverageCounts.included,0),0),projectionQueries:cases.reduce((n,c)=>n+c.native.points.length,0)};
const reference=JSON.stringify({schema:1,d3GeoVersion:'3.1.1',license:{name:'ISC',notice:'../geo-inverse-d3-LICENSE.txt'},sourceSHA256,producerSHA256:sha(await readFile(new URL(import.meta.url))),valuesSHA256:sha(value),counts,allowancePolicy:'World affine 1e-8 absolute; Data temp150 center +/-2*precision/150 and relative scale bound 4*precision/(minTempSpan-4*precision). Geometry is affine-normalized to the native frame before bounds/winding/area/coverage comparison. Native boundary band .05 pixels is fixed and every excluded query remains in the fixture.'},null,2)+'\n';
for(const[name,text]of [['values.json',value],['reference.json',reference]]){const p=join(fixture,name);if(check)assert.equal(await readFile(p,'utf8'),text,name);else await writeFile(p,text);}
console.log(JSON.stringify({passed:true,checked:check,...counts,valuesSHA256:sha(value)}));
