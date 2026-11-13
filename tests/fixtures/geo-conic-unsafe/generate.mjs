const rad=Math.PI/180;
export const encode=(_,v)=>typeof v==='number'?(Number.isNaN(v)?'n:NaN':v===Infinity?'n:+Infinity':v===-Infinity?'n:-Infinity':Object.is(v,-0)?'n:-0':v):v;
export const stringify=v=>JSON.stringify(v,encode);
const pointDistance=(a,b)=>a&&b&&[...a,...b].every(Number.isFinite)?Math.hypot(...a.map((v,i)=>v-b[i])):null;
const vector=([l,p])=>[Math.cos(p)*Math.cos(l),Math.cos(p)*Math.sin(l),Math.sin(p)];
function coefficients(kind,parallels){const[a,b]=parallels.map(v=>v*rad),tan=v=>Math.tan((Math.PI/2+v)/2);let n=kind==='equalArea'?(Math.sin(a)+Math.sin(b))/2:kind==='equidistant'?(a===b?Math.sin(a):(Math.cos(a)-Math.cos(b))/(b-a)):(a===b?Math.sin(a):Math.log(Math.cos(a)/Math.cos(b))/Math.log(tan(b)/tan(a)));
 const fallback=kind==='conformal'?!n:Math.abs(n)<1e-6,g=kind==='equidistant'&&!fallback?Math.cos(a)/n+a:null;
 return {radians:[a,b],n,g,fallback,nativeClassification:!Number.isFinite(n)?'native-nonfinite-coefficient':Math.abs(n)>1?'native-multiple-turn-coefficient':g!==null&&Math.sign(n)*g<Math.PI/2?'native-full-latitude-radial-fold':'native-no-selected-guard-violation'};}
export function generate(d3){
 const a=[-.9585319170638371,-.9585319170638373],b=[-53.13008026016052,-53.130080260160526],pairs=[];
 for(const[id,p]of [['adjacent-a',a],['adjacent-b',b]]){pairs.push([id,p], [id+'-reverse',[p[1],p[0]]],[id+'-same-a',[p[0],p[0]]],[id+'-same-b',[p[1],p[1]]],[id+'-nearby',[p[0],p[0]+1e-4]]);}
 pairs.push(['standard',[0,60]],['standard-same',[30,30]],['subnormal-smallest',[-Number.MIN_VALUE,Number.MIN_VALUE]],['signed-zero',[-0,0]],['native-invalid-nan',[NaN,30]],['native-invalid-infinity',[Infinity,30]],['native-pole-parallels',[90,90]]);
 const queries=[[.14,0],[.16,0],[1.5*Math.PI/32,0],[.1,-90*rad],[.1,-89*rad],[.1,-88*rad],[.1,-87*rad],[.1,-80*rad],[0,0],[-0,0],[Math.PI,-Math.PI/2],[-Math.PI,Math.PI/2]];
 const factories={equalArea:d3.geoConicEqualAreaRaw,equidistant:d3.geoConicEquidistantRaw,conformal:d3.geoConicConformalRaw},records=[],profiles=[];
 for(const[id,p]of pairs)for(const[kind,factory]of Object.entries(factories)){
  const raw=factory(...p.map(v=>v*rad)),coefficient=coefficients(kind,p),profile={id,kind,parallels:p,...coefficient};
  // This expectation is the explicit selected-family contract, separately
  // checked against actual Rust. It never filters or repairs native records.
  profile.rustPolicy=!p.every(v=>Number.isFinite(v)&&Math.abs(v)<90)?'invalid-parallels':!Number.isFinite(coefficient.n)||Math.abs(coefficient.n)>1||coefficient.g!==null&&Math.sign(coefficient.n)*coefficient.g<Math.PI/2?'selected-family-error':'selected-family-accept';
  profiles.push(profile);
  for(const source of queries){const forward=raw(...source),inverse=raw.invert(...forward),again=raw(...inverse),sourceVectorError=pointDistance(vector(source),vector(inverse)),rawResidual=pointDistance(forward,again);
   records.push({id,kind,source,forward,inverse,again,rawResidual,sourceVectorError,nativeCap:kind==='conformal'&&source[1]<-Math.PI/2+1e-6&&coefficient.n>0||kind==='conformal'&&source[1]>Math.PI/2-1e-6&&coefficient.n<0?'native-many-to-one-clamp':'ordinary-native-branch',rustVisibility:kind==='conformal'&&Math.abs(source[1])>1.4844222297453324?'outside-explicit-conformal-cap':'inside-explicit-visibility'});
  }
 }
 const metadata={scope:'Supplement original unsafe adjacent-parallel evidence. Native outputs are not checked Rust success expectations. No original ordinary/critical tolerance changes.',profiles,counts:{profiles:profiles.length,records:records.length,multipleTurnProfiles:profiles.filter(p=>p.nativeClassification==='native-multiple-turn-coefficient').length,radialFoldProfiles:profiles.filter(p=>p.nativeClassification==='native-full-latitude-radial-fold').length,rawResidualPassButSourceWrong:records.filter(r=>r.rawResidual!==null&&r.rawResidual<=1e-12&&r.sourceVectorError>1e-6).length}};
 const csv=records.map(r=>[r.id,r.kind,r.source,r.forward,r.inverse,r.again,r.rawResidual,r.sourceVectorError,r.nativeCap,r.rustVisibility].map(stringify).join('|')).join('\n')+'\n';
 const policies=profiles.map(p=>[p.id,p.kind,...p.parallels.map(v=>Object.is(v,-0)?'-0':String(v)),p.rustPolicy].join(',')).join('\n')+'\n';
 return {files:{'native.csv':csv,'metadata.json':JSON.stringify(metadata,encode,2)+'\n','policies.csv':policies},counts:metadata.counts};
}
