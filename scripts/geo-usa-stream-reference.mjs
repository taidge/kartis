// Original d3-geo 3.1.1 multiplex + child open-line streams, verification only.
// ISC, Copyright 2010-2024 Mike Bostock. Preserved notice:
// tests/fixtures/geo-inverse-d3-LICENSE.txt.
import assert from 'node:assert/strict';
import {readFile,writeFile,mkdir,readdir} from 'node:fs/promises';
import {execFileSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {join,resolve,relative} from 'node:path';
import {pathToFileURL} from 'node:url';

const check=process.argv.includes('--check'),root=resolve('.');
const metadata=JSON.parse(execFileSync('cargo',['metadata','--no-deps','--format-version','1'],{encoding:'utf8'}));
const base=join(metadata.target_directory,'d3-geo-reference/node_modules/d3-geo');
assert.equal(JSON.parse(await readFile(join(base,'package.json'),'utf8')).version,'3.1.1');
const d3=await import(pathToFileURL(join(base,'src/index.js')));
const sha=b=>createHash('sha256').update(b).digest('hex');
const mapBytes=await readFile(join(root,'tests/fixtures/geo-usa-map/values.json'));
const maps=JSON.parse(mapBytes).cases.filter(c=>c.group==='three-regions');
assert.equal(maps.length,6);
const precision=.02,clipEpsilon=1e-6,materialLength=.05,tubeRadius=2;
const graticule={extentMajor:[[-180,-75],[180,75]],extentMinor:[[-170,-70],[-60,70]],stepMajor:[90,45],stepMinor:[30,20],precision:5};
const nativeGrid=d3.geoGraticule().extentMajor(graticule.extentMajor).extentMinor(graticule.extentMinor).stepMajor(graticule.stepMajor).stepMinor(graticule.stepMinor).precision(graticule.precision).lines();
function feature(id,lines){return {id,label:`Source ${id}`,series:`source:${id}`,value:id.length,lines};}
function geometry(f){return {type:'MultiLineString',coordinates:f.lines};}
function capture(p,object,multiplex=false){let current=null,lines=[],active=0,maxActive=0,nestedLineStarts=0;const events=[];
 const sink={point(x,y){assert(active>0,'native point outside open line');if(!multiplex)current.push([x,y]);events.push(['point',x,y]);},lineStart(){if(active)nestedLineStarts++;active++;maxActive=Math.max(maxActive,active);if(!multiplex){assert.equal(current,null);current=[];}events.push(['lineStart']);},lineEnd(){assert(active>0);active--;if(!multiplex){lines.push(current);current=null;}events.push(['lineEnd']);},polygonStart(){assert.fail('line became polygon');},polygonEnd(){assert.fail('line became polygon');}};
 d3.geoStream(object,p.stream(sink));assert.equal(active,0);assert.equal(current,null);return multiplex?{events,maxActive,nestedLineStarts}:{lines,events};
}
function regions(k,[x,y]){return [
 {region:'Lower48',rect:[x-.455*k,y-.238*k,x+.455*k,y+.238*k]},
 {region:'Alaska',rect:[x-.425*k+clipEpsilon,y+.120*k+clipEpsilon,x-.214*k-clipEpsilon,y+.234*k-clipEpsilon]},
 {region:'Hawaii',rect:[x-.214*k+clipEpsilon,y+.166*k+clipEpsilon,x-.115*k-clipEpsilon,y+.234*k-clipEpsilon]},
];}
function children(k,[x,y]){return regions(k,[x,y]).map((r,i)=>({...r,projection:(i===0?d3.geoAlbers():d3.geoConicEqualArea().rotate([i===1?154:157,0]).center(i===1?[-2,58.5]:[-3,19.9]).parallels(i===1?[55,65]:[8,18])).precision(precision).scale(k*(i===1?.35:1)).translate(i===0?[x,y]:i===1?[x-.307*k,y+.201*k]:[x-.205*k,y+.212*k]).clipExtent([[r.rect[0],r.rect[1]],[r.rect[2],r.rect[3]]])}));}
function length(line){let n=0;for(let i=1;i<line.length;i++)n+=Math.hypot(line[i][0]-line[i-1][0],line[i][1]-line[i-1][1]);return n;}
function bounds(lines){let b=[Infinity,Infinity,-Infinity,-Infinity];for(const r of lines)for(const [x,y]of r)b=[Math.min(b[0],x),Math.min(b[1],y),Math.max(b[2],x),Math.max(b[3],y)];return b[0]===Infinity?null:b;}
function distance(lines,p){let d=Infinity;for(const r of lines)for(let i=1;i<r.length;i++){const a=r[i-1],b=r[i],dx=b[0]-a[0],dy=b[1]-a[1],l=dx*dx+dy*dy,t=l===0?0:Math.max(0,Math.min(1,((p[0]-a[0])*dx+(p[1]-a[1])*dy)/l));d=Math.min(d,Math.hypot(p[0]-a[0]-t*dx,p[1]-a[1]-t*dy));}return d;}
function clipAmplification(lines,rect){let factor=1;for(const l of lines)for(const reverse of [false,true]){const q=reverse?[...l].reverse():l;if(q.length<2)continue;const a=q[0],b=q.find(p=>Math.hypot(p[0]-a[0],p[1]-a[1])>1e-10);if(!b)continue;const d=[b[0]-a[0],b[1]-a[1]],len=Math.hypot(...d);for(let axis=0;axis<2;axis++)if([rect[axis],rect[axis+2]].some(edge=>Math.abs(a[axis]-edge)<=1e-7))factor=Math.max(factor,len/Math.max(Math.abs(d[axis]),1e-12));}assert(factor<1000,'ill-conditioned fixture clipping angle');return factor;}
const cases=[];
for(const map of maps){
 const k=map.native.scale,t=map.native.translate,cs=children(k,t),ak=cs[1],hi=cs[2];
 const plane=(child,uv)=>uv.map(([u,v])=>child.projection.invert([child.rect[0]+u/10*(child.rect[2]-child.rect[0]),child.rect[1]+v/10*(child.rect[3]-child.rect[1])]));
 const source=[
  feature('all-regions',[[[-122,35],[-110,40],[-95,38]],[[-105,32],[-90,36]],[[-165,58],[-150,64],[-135,58]],[[-160,56],[-145,60]],[[-160,20],[-157,21.5],[-154,19.5]],[[-159,19.5],[-155,20]]]),
  feature('cross-insets',[[[-120,35],[-150,61],[-157,21],[-100,40]]]),
  feature('alaska-antimeridian',[[[170,55],[-175,58],[-160,55],[175,53]]]),
  feature('rectangle-reentry',[plane(ak,[[-2,2],[3,2],[12,2],[12,5],[4,5],[-2,5],[-2,8],[5,8],[12,8]])]),
  feature('source-gaps',[[[-118,34],[-112,36]],[[-99,40],[-90,42]],[[-159,20],[-155,20]]]),
  feature('invisible',[[[1,48],[3,49]]]),
  feature('corner-touch',[plane(ak,[[-1,-1],[0,0]]),plane(hi,[[11,11],[10,10]])]),
  ...nativeGrid.map((g,i)=>({...feature(`graticule-${i}`,[g.coordinates]),graticuleIndex:i})),
 ];
 const output=[];
 for(const f of source){
  const composite=capture(d3.geoAlbersUsa().scale(k).translate(t).precision(precision),geometry(f),true);
  const parts=[];
  for(const child of cs){
   const raw=capture(child.projection,geometry(f)).lines,material=raw.filter(l=>length(l)>materialLength);
   const amplification=clipAmplification(material,child.rect);
   // Frozen before Rust: independent .02px chord errors, plus near-boundary
   // intersection amplification measured solely from the original native line.
   const ratioLimit=1/(1-map.allowances.fitRelativeLimit),errorLimit=(precision*(1+ratioLimit)+1e-7)*amplification;
   const band=Math.max(.05,errorLimit),queries=[];
   const r=child.rect;
   for(let j=0;j<5;j++)for(let i=0;i<7;i++)queries.push([r[0]+(i+.37)/7*(r[2]-r[0]),r[1]+(j+.61)/5*(r[3]-r[1])]);
   for(const line of material){const stride=Math.max(1,Math.floor((line.length-1)/6));for(let i=1;i<line.length;i+=stride){const a=line[i-1],b=line[i],dx=b[0]-a[0],dy=b[1]-a[1],len=Math.hypot(dx,dy);if(len===0)continue;for(const offset of [-2.3,-1.7,0,1.7,2.3])queries.push([(a[0]+b[0])/2-dy/len*offset,(a[1]+b[1])/2+dx/len*offset]);}}
   const coverage=queries.map(position=>{const d=distance(material,position);return {position,distance:Number.isFinite(d)?d:null,inside:d<tubeRadius,excluded:Math.abs(d-tubeRadius)<=band};});
   parts.push({region:child.region,rect:child.rect,lines:raw,materialComponents:material.length,shortComponents:raw.length-material.length,bounds:bounds(material),amplification,errorLimit,coverageBand:band,coverage,coverageCounts:{total:coverage.length,included:coverage.filter(q=>!q.excluded).length,excluded:coverage.filter(q=>q.excluded).length}});
  }
  // Composite is genuinely executed above; child identity is recorded separately.
  // Native multiplex may interleave child lineStart/lineEnd for long source arcs.
  // Preserve those real events; do not pretend they form unambiguous components.
  // Independent original child streams supply identity and nonbridging geometry.
  const normalized=points=>points.map(p=>JSON.stringify(p)).sort();
  assert.deepEqual(normalized(composite.events.filter(e=>e[0]==='point').map(e=>e.slice(1))),normalized(parts.flatMap(p=>p.lines.flat())),`${map.name}/${f.id}: original composite/child point multiset mismatch`);
  output.push({id:f.id,multiplex:composite,parts});
 }
 const multi=output.find(f=>f.id==='all-regions');assert(multi.parts.every(p=>p.materialComponents>=2));
 assert(output.find(f=>f.id==='invisible').parts.every(p=>p.materialComponents===0));
 cases.push({name:map.name,map:{config:map.config,options:map.options,features:map.features,native:{scale:k,translate:t,rawCenter:map.native.rawCenter},allowances:map.allowances},features:source,native:output});
}
const counts={cases:cases.length,featureStates:0,regionStates:0,materialComponents:0,shortComponents:0,nativeVertices:0,multiplexNestedStarts:0,multiplexInterleavedFeatures:0,coverageQueries:0,coverageIncluded:0,coverageExcluded:0};
let maxClipAmplification=1,maxErrorLimit=0;
for(const c of cases)for(const f of c.native){counts.featureStates++;counts.multiplexNestedStarts+=f.multiplex.nestedLineStarts;if(f.multiplex.nestedLineStarts)counts.multiplexInterleavedFeatures++;for(const p of f.parts){counts.regionStates++;counts.materialComponents+=p.materialComponents;counts.shortComponents+=p.shortComponents;counts.nativeVertices+=p.lines.reduce((n,l)=>n+l.length,0);counts.coverageQueries+=p.coverageCounts.total;counts.coverageIncluded+=p.coverageCounts.included;counts.coverageExcluded+=p.coverageCounts.excluded;maxClipAmplification=Math.max(maxClipAmplification,p.amplification);maxErrorLimit=Math.max(maxErrorLimit,p.errorLimit);}}
const fixture=join(root,'tests/fixtures/geo-usa-stream');await mkdir(fixture,{recursive:true});
const values=JSON.stringify({schema:1,d3GeoVersion:'3.1.1',precision,clipEpsilon,materialLength,tubeRadius,graticule,nativeGraticule:nativeGrid.map(g=>g.coordinates),policy:'Original multiplex events and original child streams; fitted frame comes from polygon map source, never lines. Native multiplex can interleave open child components; its real events and nesting counts are preserved and its point multiset checked against children. Regional geometry comparison uses original child streams to avoid interpreting interleaved events as safe bridges. Compare material components by region and open path bounds plus bidirectional vertex/midpoint-to-segment distance after affine normalization. Chord error is .02*(1+1/(1-mapFitRelativeLimit)); endpoint clip amplification is frozen from original native boundary crossing angles. Geometric 2px tube coverage excludes only the recorded native threshold band, at least .05px, expanded by the same native error bound. All raw short/touch components remain recorded; length<=.05px is diagnostic, not material topology equality. This does not verify painted stroke masks.',cases},null,2)+'\n';
const sourceSHA256={};async function inventory(dir){for(const e of await readdir(dir,{withFileTypes:true})){const p=join(dir,e.name);if(e.isDirectory())await inventory(p);else if(e.name.endsWith('.js'))sourceSHA256[relative(base,p).replaceAll('\\','/')]=sha(await readFile(p));}}await inventory(join(base,'src'));
const interleaving=cases.flatMap(c=>c.native.filter(f=>f.multiplex.nestedLineStarts).map(f=>({case:c.name,id:f.id,source:c.features.find(s=>s.id===f.id).lines,maxActive:f.multiplex.maxActive,nestedLineStarts:f.multiplex.nestedLineStarts,compositePoints:f.multiplex.events.filter(e=>e[0]==='point').length,childPoints:f.parts.reduce((n,p)=>n+p.lines.reduce((m,l)=>m+l.length,0),0),pointMultisetEqual:true})));
const reference=JSON.stringify({schema:1,d3GeoVersion:'3.1.1',license:{name:'ISC',notice:'../geo-inverse-d3-LICENSE.txt'},sourceSHA256,mapValuesSHA256:sha(mapBytes),producerSHA256:sha(await readFile(new URL(import.meta.url))),valuesSHA256:sha(values),counts,maxClipAmplification,maxErrorLimit,nativeMultiplexCaveat:{source:'src/projection/albersUsa.js:6-7',comment:'The projections must have mutually exclusive clip regions on the sphere, as this will avoid emitting interleaving lines and polygons.',interpretation:'Original composite events are preserved without repair. Child streams have identical point multisets but their noninterleaving regional geometry is not equivalent to interpreting the composite as a flat path.',interleaving}},null,2)+'\n';
for(const[name,value]of [['values.json',values],['reference.json',reference]]){const p=join(fixture,name);if(check)assert.equal(await readFile(p,'utf8'),value,name);else await writeFile(p,value);}
console.log(JSON.stringify({passed:true,checked:check,...counts,maxClipAmplification,maxErrorLimit,valuesSHA256:sha(values)}));
