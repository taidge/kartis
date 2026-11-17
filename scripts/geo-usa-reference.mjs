// Execute original d3-geo 3.1.1 composite and child projection streams.
import assert from 'node:assert/strict';
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const root=resolve('.'),check=process.argv.includes('--check');
const metadata=JSON.parse(execFileSync('cargo',['metadata','--no-deps','--format-version','1'],{encoding:'utf8'}));
const base=join(metadata.target_directory,'d3-geo-reference','node_modules','d3-geo');
assert.equal(JSON.parse(await readFile(join(base,'package.json'),'utf8')).version,'3.1.1');
const d3=await import(pathToFileURL(join(base,'src/index.js')));
const sha=b=>createHash('sha256').update(b).digest('hex');
const sources={};for(const name of ['projection/albersUsa.js','projection/albers.js','projection/conicEqualArea.js','projection/index.js','projection/fit.js','rotation.js','math.js','clip/rectangle.js','clip/buffer.js','clip/rejoin.js','clip/line.js'])sources[name]=sha(await readFile(join(base,'src',name)));
function pointStream(projection){let point=null;const stream=projection.stream({point(x,y){point=[x,y];}});return p=>{point=null;stream.point(...p);return point;};}
const input=[[-74.006,40.7128],[-122.4194,37.7749],[-149.9003,61.2181],[-147.7164,64.8378],[-134.4197,58.3019],[-157.8583,21.3069],[-155.09,19.7297],[178,52],[-122,22],[0,0],[2.35,48.86],[180,0],[-180,90]];
for(let lon=-180;lon<=180;lon+=10)for(let lat=-80;lat<=80;lat+=10)input.push([lon,lat]);
const cases=[];
for(const scale of [150,1070,3500])for(const translate of [[0,0],[480,250],[-123.25,876.5]]){
 const [x,y]=translate,k=scale,e=1e-6;
 const composite=d3.geoAlbersUsa().scale(k).translate(translate);
 const children=[
  ['Lower48',d3.geoAlbers().scale(k).translate(translate).clipExtent([[x-.455*k,y-.238*k],[x+.455*k,y+.238*k]])],
  ['Alaska',d3.geoConicEqualArea().rotate([154,0]).center([-2,58.5]).parallels([55,65]).scale(k*.35).translate([x-.307*k,y+.201*k]).clipExtent([[x-.425*k+e,y+.120*k+e],[x-.214*k-e,y+.234*k-e]])],
  ['Hawaii',d3.geoConicEqualArea().rotate([157,0]).center([-3,19.9]).parallels([8,18]).scale(k).translate([x-.205*k,y+.212*k]).clipExtent([[x-.214*k+e,y+.166*k+e],[x-.115*k-e,y+.234*k-e]])],
 ].map(([region,projection])=>({region,projection,point:pointStream(projection),clip:projection.clipExtent()}));
 const points=input.map(p=>{
  const output=composite(p),regions=children.map(c=>({region:c.region,output:c.point(p)}));
  const selected=regions.find(c=>c.output!==null);
  assert.deepEqual(output,selected?.output??null);
  return {input:p,output,region:selected?.region??null,regions};
 });
 function inverse(p){
  const nx=(p[0]-x)/k,ny=(p[1]-y)/k;
  const region=ny>=.120&&ny<.234&&nx>=-.425&&nx<-.214?'Alaska':ny>=.166&&ny<.234&&nx>=-.214&&nx<-.115?'Hawaii':'Lower48';
  const c=children.find(c=>c.region===region),[[x0,y0],[x1,y1]]=c.clip;
  const native=composite.invert(p),candidate=c.projection.invert(p),forward=candidate?.every(Number.isFinite)?c.projection(candidate):null;
  const forwardStreamVisible=candidate?.every(Number.isFinite)?c.point(candidate)!==null:false;
  const inside=x0<=p[0]&&p[0]<=x1&&y0<=p[1]&&p[1]<=y1;
  const strict=inside&&forward&&Math.hypot(forward[0]-p[0],forward[1]-p[1])<1e-6?candidate:null;
  return {input:p,region,native,strict,forwardStreamVisible};
 }
 const inverseInputs=points.filter(p=>p.output).map(p=>p.output);
 for(const nx of [-.455,-.425,-.214,-.115,0,.455,2])for(const ny of [-.238,.120,.166,.234,.238,2])inverseInputs.push([x+nx*k,y+ny*k]);
 const inverses=inverseInputs.map(inverse);
 const regionInverses=[];
 for(const p of points)for(const r of p.regions)if(r.output){const c=children.find(c=>c.region===r.region);regionInverses.push({input:r.output,region:r.region,output:c.projection.invert(r.output)});}
 cases.push({scale,translate,clips:children.map(c=>({region:c.region,extent:c.clip})),points,inverses,regionInverses});
}
const fixture=join(root,'tests/fixtures/geo-usa-kernel');await mkdir(fixture,{recursive:true});
const values=JSON.stringify({schema:2,d3GeoVersion:'3.1.1',inversePolicy:'Strict input pixel membership in selected region plus original unclipped child forward residual. Candidate forward-stream clipping is recorded separately and does not define inverse membership.',scope:'Original composite and three original child streams, pixel epsilon, default selection, and explicitly derived input-domain inverse expectations; not polygon/line/map acceptance.',cases},null,2)+'\n';
const reference=JSON.stringify({schema:2,d3GeoVersion:'3.1.1',sourceSHA256:sources,producerSHA256:sha(await readFile(new URL(import.meta.url))),valuesSHA256:sha(values),counts:{states:cases.length,points:cases.reduce((n,c)=>n+c.points.length,0),inverses:cases.reduce((n,c)=>n+c.inverses.length,0),regionInverses:cases.reduce((n,c)=>n+c.regionInverses.length,0)}},null,2)+'\n';
for(const [name,text]of [['values.json',values],['reference.json',reference]]){const path=join(fixture,name);if(check)assert.equal(await readFile(path,'utf8'),text,name);else await writeFile(path,text);}
console.log(JSON.stringify({passed:true,checked:check,...JSON.parse(reference).counts}));
