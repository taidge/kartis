import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
export const encode=(_,v)=>typeof v==='number'?(Number.isNaN(v)?'n:NaN':v===Infinity?'n:+Infinity':v===-Infinity?'n:-Infinity':Object.is(v,-0)?'n:-0':v):v;
export const stringify=v=>JSON.stringify(v,encode);
const rad=Math.PI/180,finite=p=>Array.isArray(p)&&p.every(Number.isFinite),error=(a,b)=>finite(a)&&finite(b)?Math.max(...a.map((x,i)=>Math.abs(x-b[i]))):null;
const degree=[-180,-180+1e-9,-150,-90,-45,-1e-9,-0,0,1e-9,45,90,150,180-1e-9,180];
const latitude=[-90,-90+1e-9,-89.999999,-85,-45,-1e-9,-0,0,1e-9,30,60,85,89.999999,90-1e-9,90];
export const pairs=[[0,60],[30,30],[29.5,45.5],[30,60],[60,30],[-45,-30],[-30,-30],[0,0],[-30,30],[30,-30],[-30,30+1e-8],[-30,30+1e-5],[-30,30+1e-4],[-30,30+.001],[-89,89],[89,-89],[90,90],[-90,-90],[-90,90],[0,90],[89.999999,89.999999],[-89.999999,-89.999999],[NaN,30],[Infinity,30],[-100,30],[30,100],[100,100],[-0,0],[0,-0],[-30,30-1e-8],[-30,30-1e-5],[-30,30-.0001],[-30,30-.001],[-0.0000572957795,0.0000572957795],[0.0000572957795,0.0000572957795],[0.0000572957796,0.0000572957796],[-0.0000572957795,-0.0000572957795],[15,15],[75,75],[-75,-75],[15,75],[-75,-15],[0,1e-8],[0,-1e-8],[NaN,NaN],[-Infinity,30]];
const getters=p=>Object.fromEntries(['parallels','scale','translate','center','rotate','angle','reflectX','reflectY','precision','clipAngle','clipExtent'].map(k=>[k,typeof p[k]==='function'?p[k]():null]));
const profiles=(kind,pair)=>!pair.every(Number.isFinite)?'native-nonfinite-parallels':pair.some(v=>Math.abs(v)>90)?'native-out-of-range-parallels':pair.some(v=>Math.abs(v)===90)?'native-polar-parallels':pair[0]===pair[1]?'same-parallel':pair[0]===-pair[1]?'symmetric-degenerate':Math.abs(pair[0]+pair[1])<.002?'near-symmetric-epsilon':'ordinary';
const region={type:'Polygon',coordinates:[[[-125,25],[-125,50],[-65,50],[-65,25],[-125,25]],[[-105,32],[-95,32],[-95,40],[-105,40],[-105,32]]]};
function capture(d3,p,feature,budget=20000){
 const events=[];let attempted=0,failure=null;
 try{d3.geoStream(feature,p.stream({point(x,y,z){if(++attempted>budget)throw Error('oracle sink point budget exceeded');events.push(['p',x,y,z===undefined?null:z]);},lineStart(){events.push(['ls']);},lineEnd(){events.push(['le']);},polygonStart(){events.push(['ps']);},polygonEnd(){events.push(['pe']);},sphere(){events.push(['s']);}}));}catch(e){failure=String(e);}
 const pts=events.filter(e=>e[0]==='p'),valid=pts.filter(e=>Number.isFinite(e[1])&&Number.isFinite(e[2]));
 const bounds=valid.length?[Math.min(...valid.map(p=>p[1])),Math.min(...valid.map(p=>p[2])),Math.max(...valid.map(p=>p[1])),Math.max(...valid.map(p=>p[2]))]:null;
 return {events,summary:{attempted,points:pts.length,nonfinite:pts.length-valid.length,bounds,complete:failure===null,error:failure,eventSha256:createHash('sha256').update(stringify(events)).digest('hex')}};
}
function line(row){return row.map(x=>stringify(x)).join('|');}
export function generate(d3,prior){
 const family=[['equalArea',d3.geoConicEqualArea,d3.geoConicEqualAreaRaw],['conformal',d3.geoConicConformal,d3.geoConicConformalRaw],['equidistant',d3.geoConicEquidistant,d3.geoConicEquidistantRaw]],files={},tables={raw:[],inverse:[],critical:[],parallels:[],defaults:[],fit:[],world:[],stream:[]};
 const counts={profiles:0,raw:0,rawNonfinite:0,inverse:0,inverseNonfinite:0,inverseOutsideOrNotRoundtripping:0,critical:0,criticalBadOriginalInverse:0,setters:0,nativeNaNConformalFinite:0,defaults:0,albersExactPreset:0,albersUSAIsolated:0,fit:0,fitEvents:0,world:0,worldIncomplete:0,worldNonfinite:0,streamEvents:0};
 for(const [kind,factory,rawFactory]of family)for(const [index,pair]of pairs.entries()){
  const id=`${kind}-${index}`,profile=profiles(kind,pair),raw=rawFactory(...pair.map(x=>x*rad));counts.profiles++;
  tables.raw.push(['C',id,kind,pair,profile]);
  for(const l of degree)for(const phi of latitude){const source=[l*rad,phi*rad],point=raw(...source),inverse=raw.invert(...point),again=raw(...inverse);counts.raw++;if(!finite(point))counts.rawNonfinite++;tables.raw.push(['F',id,[l,phi],source,point,inverse,again,error(point,again)]);}
  for(const x of [-10,-3,-1,-0,0,1,3,10])for(const y of [-10,-3,-1,-0,0,1,3,10]){
   const point=[x,y],inverse=raw.invert(...point),again=raw(...inverse),branch=finite(inverse)&&Math.abs(inverse[0])<=Math.PI&&Math.abs(inverse[1])<=Math.PI/2,err=error(point,again);counts.inverse++;if(!finite(inverse))counts.inverseNonfinite++;else if(!branch||err===null||err>1e-7)counts.inverseOutsideOrNotRoundtripping++;tables.inverse.push(['I',id,point,inverse,again,branch,err]);
  }
  const p=factory().scale(75).translate([311,207]).center([0,0]).rotate([-35,-20,-25]),before=getters(p),same=p.parallels(pair)===p,point=p([10,30]),inverse=p.invert(point);
  tables.parallels.push(['P',id,profile,pair,before,getters(p),same,[10,30],point,inverse]);counts.setters++;
  if(kind==='conformal'&&pair.some(Number.isNaN)&&finite(point))counts.nativeNaNConformalFinite++;
 }
 for(const old of prior.critical){const [, ,factory]=family.find(f=>f[0]===old.kind),raw=factory(...old.parallels.map(x=>x*rad)),point=raw(...old.sourceRadians),inverse=raw.invert(...point),again=raw(...inverse),err=error(point,again);tables.critical.push(['K',old.kind,old.parallels,old.sourceRadians,point,inverse,again,err,err>1e-7?'original-sign-zero-inverse-error':'original-native-critical']);counts.critical++;if(err>1e-7)counts.criticalBadOriginalInverse++;}
 const sources=[[-96.6,38.7],[-98,38],[-122.42,37.77],[-74,40.7],[-149.9,61.2],[-157.8,21.3],[0,0],[180,90],[0,-90]];
 for(const [kind,factory]of [...family,['albers',d3.geoAlbers]]){
  const p=factory();tables.defaults.push(['D',kind,getters(p)]);counts.defaults++;
  for(const source of sources){const point=p(source);tables.defaults.push(['Q',kind,source,point,point?p.invert(point):null]);}
  for(const point of [[480,250],[0,0],[100,400],[200,480],[1000,1000]]){const source=p.invert(point);tables.defaults.push(['V',kind,point,source,source?p(source):null]);}
 }
 const albers=d3.geoAlbers(),equivalent=d3.geoConicEqualArea().parallels([29.5,45.5]).scale(1070).translate([480,250]).rotate([96,0]).center([-.6,38.7]),wrongCamera=d3.geoConicEqualArea().parallels([29.5,45.5]).scale(1070).translate([480,250]).rotate([96.6,-38.7]).center([0,0]);
 for(const source of sources){const a=albers(source),b=equivalent(source);assert.equal(stringify(a),stringify(b));tables.defaults.push(['A',source,a,b,wrongCamera(source)]);counts.albersExactPreset++;}
 const usa=d3.geoAlbersUsa();files['conic-albers-usa-isolated.csv']=[['SCOPE','Separate compound projection. Never counted as ordinary Albers or any Conic fit profile.'],['D','albersUSA',getters(usa)],...sources.map(source=>{const p=usa(source);return ['Q','albersUSA',source,p,p?usa.invert(p):null];}),...[[480,250],[0,0],[100,400],[200,480],[1000,1000]].map(point=>{const p=usa.invert(point);return ['V','albersUSA',point,p,p?usa(p):null];})].map(line).join('\n')+'\n';counts.albersUSAIsolated=sources.length+5;
 for(const [kind,factory]of [...family,['albers',d3.geoAlbers]])for(const camera of [[0,0,0],[35,20,25],[-80,30,-30]]){
  const p=factory().rotate(camera).precision(1e-6).fitExtent([[70,30],[690,290]],region),captured=capture(d3,p,region);tables.fit.push(['T',kind,camera,getters(p),captured.summary,sources.slice(0,4).map(source=>{const point=p(source);return [source,point,p.invert(point)];})]);
  for(const [index,event]of captured.events.entries())tables.stream.push(['T',kind,camera,index,event]);counts.fit++;counts.fitEvents+=captured.events.length;
 }
 for(const [kind,factory,rawFactory]of family)for(const pair of [[0,60],[30,30],[29.5,45.5],[0,0],[-30,30],[-45,-30]]){
  const p=factory().parallels(pair).scale(1).translate([0,0]).rotate([0,0]).center([0,0]).precision(1e-5),captured=capture(d3,p,{type:'Sphere'}),raw=rawFactory(...pair.map(x=>x*rad)),x=raw(Math.PI,0)[0],y=raw(0,Math.PI/2)[1];
  tables.world.push(['W',kind,pair,getters(p),captured.summary,[-x,-y,x,y],captured.summary.bounds?[captured.summary.bounds[0],-captured.summary.bounds[3],captured.summary.bounds[2],-captured.summary.bounds[1]]:null]);
  for(const [index,event]of captured.events.entries())tables.stream.push(['W',kind,pair,index,event]);counts.world++;counts.worldIncomplete+=Number(!captured.summary.complete);counts.worldNonfinite+=captured.summary.nonfinite;
 }
 counts.streamEvents=tables.stream.length;
 for(const [key,rows]of Object.entries(tables))files[`conic-${key}.csv`]=rows.map(line).join('\n')+'\n';
 files['conic-profiles.json']=JSON.stringify({schema:1,scope:'Native original outputs only; no checked inverse or Rust implementation decision. No tolerance is applied to NodeVM versus original geo ESM comparisons.',encoding:'JSON numbers preserve finite values; numeric -0/NaN/±Infinity use n:-0/n:NaN/n:+Infinity/n:-Infinity. CSV is JSON-cell pipe-delimited, one record per line.',counts,profiles:pairs.map((parallels,index)=>({index,parallels})),criticalDifference:'All 72 old authored critical sources are retained, including 22 original inverse sign-zero misprojections. Do not count them as mathematically correct inverse equivalence.',nativeParallelDifference:'Nonfinite and out-of-range standards are recorded, never accepted by a future checked API merely because output is finite; NaN Conformal can fall back to Mercator.',inverseDifference:'Arbitrary raw inverse results can be finite while outside canonical lambda/phi or unable to forward-roundtrip. No safety profile has been implemented.',fit:'D3 fitExtent independent of Rust; camera values are original D3.rotate, and D3.center is projected-plane recentering, not Kartis spherical camera center.',world:'Incomplete streams explicitly retain attempted budget/error/partial events. Partial bounds are never a complete World frame. Generic Conformal Mercator fallback may contain nonfinite Sphere points.',albers:'Ordinary Albers is the exact default ConicEqualArea preset; AlbersUSA is isolated and excluded from ordinary counts.'},encode,2)+'\n';
 assert.equal(counts.critical,72);assert.equal(counts.criticalBadOriginalInverse,22);assert(counts.nativeNaNConformalFinite>0);return {files,counts};
}
