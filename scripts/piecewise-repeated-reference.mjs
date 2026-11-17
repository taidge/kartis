// Verification only: original d3-scale 4.0.2 and d3-zoom 3.0.0 execution.
// Full original ISC notices and source hashes are preserved in reference.json.
import assert from 'node:assert/strict';
import {execFileSync} from 'node:child_process';
import {readFile,writeFile,mkdir,readdir} from 'node:fs/promises';
import {join,relative,resolve} from 'node:path';
import {pathToFileURL} from 'node:url';
import {createHash} from 'node:crypto';
const root=resolve('.'),check=process.argv.includes('--check');
const target=JSON.parse(execFileSync('cargo',['metadata','--offline','--no-deps','--format-version','1'],{encoding:'utf8',windowsHide:true})).target_directory;
const modules=join(root,'node_modules'),scaleRoot=join(modules,'d3-scale');
assert.equal(JSON.parse(await readFile(join(scaleRoot,'package.json'),'utf8')).version,'4.0.2');
const zoomRoot=join(modules,'d3-zoom');
assert.equal(JSON.parse(await readFile(join(zoomRoot,'package.json'),'utf8')).version,'3.0.0');
const d3=await import(pathToFileURL(join(scaleRoot,'src/index.js')));
const {symlogish}=await import(pathToFileURL(join(scaleRoot,'src/symlog.js')));
const {powish}=await import(pathToFileURL(join(scaleRoot,'src/pow.js')));
const {loggish}=await import(pathToFileURL(join(scaleRoot,'src/log.js')));
const {Transform}=await import(pathToFileURL(join(zoomRoot,'src/transform.js')));
const sha=b=>createHash('sha256').update(b).digest('hex');
const encoded=v=>typeof v==='number'?(Object.is(v,-0)?'-0':Number.isNaN(v)?'NaN':v===Infinity?'Infinity':v===-Infinity?'-Infinity':v):v;
const json=v=>JSON.stringify(v,(_,v)=>encoded(v));
function nextUp(x){if(!Number.isFinite(x))return x;if(x===0)return Number.MIN_VALUE;const a=new Float64Array([x]),u=new BigUint64Array(a.buffer);u[0]+=x>0?1n:-1n;return a[0];}
const nextDown=x=>-nextUp(-x);
const make=(kind,p)=>kind==='linear'?d3.scaleLinear():kind==='log'?d3.scaleLog().base(p):kind==='symlog'?d3.scaleSymlog().constant(p):kind==='pow'?d3.scalePow().exponent(p):kind==='sqrt'?d3.scaleSqrt():d3.scaleUtc();
const profiles=[
 {name:'linear',kind:'linear',parameter:1,base:[-9.3,0,17.6]},
 {name:'positive-log',kind:'log',parameter:10,base:[1.3,3.7,17.2]},
 {name:'negative-log',kind:'log',parameter:2,base:[-17.2,-3.7,-1.3]},
 {name:'symlog',kind:'symlog',parameter:2,base:[-9.3,0,17.6]},
 {name:'square',kind:'pow',parameter:2,base:[-9.3,0,17.6]},
 {name:'quarter-power',kind:'pow',parameter:.25,base:[-9.3,0,17.6]},
 {name:'positive-reciprocal',kind:'pow',parameter:-1,base:[1.3,3.7,17.2]},
 {name:'negative-reciprocal-square',kind:'pow',parameter:-2,base:[-17.2,-3.7,-1.3]},
 {name:'sqrt',kind:'sqrt',parameter:.5,base:[-9.3,0,17.6]},
 {name:'utc',kind:'utc',parameter:1,base:[1704067200000,1704072600000,1704159000000]},
];
function patterns([a,b,c]){return [
 ['two-constant',[b,b]],['first',[a,a,c]],['interior',[a,b,b,c]],['last',[a,c,c]],
 ['interior-run',[a,b,b,b,c]],['all-three',[b,b,b]],['all-four',[b,b,b,b]],
];}
const settings=[];
for(const reverse of [false,true])for(const round of [false,true])for(const clamp of [false,true])settings.push({nice:-1,reverse,round,clamp});
for(const nice of [0,5])for(const [reverse,round,clamp]of [[false,false,false],[true,false,true],[false,true,true],[true,true,false]])settings.push({nice,reverse,round,clamp});
const shapes=['up','down','turn','plateau'];
function range(shape,n){return Array.from({length:n},(_,i)=>shape==='up'?-10.5+53*i:shape==='down'?207.5-37*i:shape==='turn'?(i%2?207.5:-10.5):i<2?-10.5:53*i-10.5);}
const strict=xs=>xs.slice(1).every((v,i)=>xs.at(-1)>xs[0]?v>xs[i]:v<xs[i]);
const monotonic=xs=>xs.every(Number.isFinite)&&(xs.slice(1).every((v,i)=>v>=xs[i])||xs.slice(1).every((v,i)=>v<=xs[i]));
const unique=xs=>[...new Map(xs.map(v=>[String(encoded(v)),v])).values()];
function originalTransform(profile,domain){
 if(profile.kind==='linear'||profile.kind==='utc')return x=>x;
 let transform,storedDomain=domain;const fake=function(){};fake.domain=function(v){if(arguments.length){storedDomain=v;return fake;}return storedDomain;};
 const capture=t=>(transform=t,fake);
 if(profile.kind==='log')loggish(capture).base(profile.parameter).domain(domain);
 else if(profile.kind==='symlog')symlogish(capture).constant(profile.parameter);
 else powish(capture).exponent(profile.kind==='sqrt'?.5:profile.parameter);
 return transform;
}
function checkedProfile(profile,domain,transformed){
 if(!monotonic(domain))return 'reject-true-turn';
 if(profile.kind==='pow'&&profile.parameter<0&&(domain.includes(0)||domain.some(v=>Math.sign(v)!==Math.sign(domain[0]))))return 'reject-reciprocal-branch';
 if(!transformed.every(Number.isFinite)||!Number.isFinite(transformed.at(-1)-transformed[0]))return 'reject-transform-overflow';
 const direction=profile.kind==='pow'&&profile.parameter<0?-1:1;
 for(let i=1;i<domain.length;i++)if(domain[i]!==domain[i-1]&&(transformed[i]===transformed[i-1]||Math.sign((transformed[i]-transformed[i-1])*direction)!==Math.sign(domain[i]-domain[i-1])))return 'reject-distinct-transform-collapse';
 return 'accept';
}
function configured(p,domain,r,o){const s=make(p.kind,p.parameter).domain(domain).clamp(o.clamp);o.round?s.rangeRound(r):s.range(r);if(o.nice===0)s.nice();else if(o.nice>0)s.nice(o.nice);return s;}
const cases=[];
function add(profile,pattern,domain,shape,o,reject=null){
 const rawRange=range(shape,domain.length),r=o.reverse?rawRange.toReversed():rawRange;
 const s=configured(profile,domain,r,o),display=s.domain().map(Number),raw=make(profile.kind,profile.parameter).domain(display).range(r).clamp(o.clamp);
 const lo=Math.min(...display),hi=Math.max(...display),span=hi-lo||Math.max(1,Math.abs(lo)*.5);
 const neighbors=unique([...domain,...display]).flatMap(v=>profile.kind==='utc'?[v-1,v,v+1]:[nextDown(v),v,nextUp(v)]);
 let queries=unique([...domain,...display,...neighbors,lo-span/4,hi+span/4,(lo+hi)/2,0,-1,1]);
 if(profile.kind==='utc')queries=unique(queries.map(Math.trunc));
 const checkedQuery=x=>o.clamp||!(profile.kind==='log'||profile.kind==='pow'&&profile.parameter<0)||x!==0&&Math.sign(x)===Math.sign(domain[0]);
 const pixels=unique([-600,-10.5,0,17.25,73,207.5,400,600,...r,...r.slice(1).map((v,i)=>(v+r[i])/2)]);
 const branches=r.slice(1).map((v,i)=>{const range=[r[i],v],b=make(profile.kind,profile.parameter).domain(display.slice(i,i+2)).range(range).clamp(o.clamp);return {index:i,flat:range[0]===range[1],pixels:unique([range[0]-1,range[0],(range[0]+range[1])/2,range[1],range[1]+1]).map(pixel=>({pixel,inverse:+b.invert(pixel)}))};});
 const reciprocalNiceInvalid=profile.kind==='pow'&&profile.parameter<0&&(display.includes(0)||Math.sign(display[0])!==Math.sign(display.at(-1)));
 cases.push({name:`${profile.name}/${pattern}/${shape}/${o.nice}/${+o.reverse}${+o.round}${+o.clamp}/${cases.length}`,kind:profile.kind,parameter:profile.parameter,pattern,domain,range:rawRange,options:o,checkedConstruction:reject??(reciprocalNiceInvalid?'reject-reciprocal-nice-zero':'accept'),native:{display,range:r,queries:queries.map(input=>({input,checkedInput:checkedQuery(input),mapped:s(input),unrounded:raw(input)})),inverse:pixels.map(pixel=>({pixel,value:+s.invert(pixel)})),branches,ticks:s.ticks(7).map(Number),strictRangeInverse:strict(r),rawDuplicate:domain.some((v,i)=>i&&v===domain[i-1])}});
}
for(const profile of profiles)for(const [pattern,source]of patterns(profile.base))for(const descending of [false,true])for(const shape of shapes)for(const o of settings)add(profile,`${pattern}/${descending?'descending':'ascending'}`,descending?source.toReversed():source,shape,o);
for(const [pattern,domain]of [['signed-zero',[-1,-0,0,0,1]],['all-signed-zero',[-0,0,-0]],['first-run',[-9.3,-9.3,-9.3,0,17.6]],['last-run',[-9.3,0,17.6,17.6,17.6]]])for(const descending of [false,true])for(const shape of shapes)for(const o of settings)add(profiles[0],`${pattern}/${descending?'descending':'ascending'}`,descending?domain.toReversed():domain,shape,o);
const collapseProfiles=[
 {name:'distinct-log-transform-collapse',kind:'log',parameter:10,domain:[1e100,nextUp(1e100),1e101]},
 {name:'distinct-symlog-transform-collapse',kind:'symlog',parameter:1,domain:[1e100,nextUp(1e100),1e101]},
 {name:'distinct-positive-power-transform-collapse',kind:'pow',parameter:1e-300,domain:[1,2,3]},
 {name:'distinct-negative-power-transform-collapse',kind:'pow',parameter:-1e-300,domain:[1,2,3]},
 {name:'distinct-power-overflow',kind:'pow',parameter:1e300,domain:[1,2,3]},
 {name:'distinct-linear-span-overflow',kind:'linear',parameter:1,domain:[-Number.MAX_VALUE,0,Number.MAX_VALUE]},
];
for(const p of collapseProfiles)for(const descending of [false,true])for(const clamp of [false,true])add(p,p.name,descending?p.domain.toReversed():p.domain,'up',{nice:-1,reverse:false,round:false,clamp},`reject-${p.name}`);

// Execute the original Transform methods; rescale domain never comes from a
// handwritten inverse or a two-endpoint approximation of the piecewise scale.
const viewportCases=[];
const viewportProfiles=profiles.map(p=>({...p,base:p.kind==='utc'?[0,4194304,8388608]:p.kind==='log'?(p.base[0]<0?[-8,-2,-1]:[1,2,8]):p.kind==='pow'&&p.parameter<0?(p.base[0]<0?[-8,-2,-1]:[1,2,8]):[-4,0,16]}));
for(const profile of viewportProfiles)for(const [pattern,base]of patterns(profile.base).filter(([p])=>['first','interior','last','all-three','two-constant'].includes(p)))for(const descending of [false,true])for(const nice of [-1,0])for(const reverse of [false,true])for(const round of [false,true])for(const axis of ['x','y']){
 const source=descending?base.toReversed():base,extent=axis==='x'?340.5:240.5;
 const rangeFractions=source.length===4?[0,.25,.75,1]:source.map((_,i)=>i/(source.length-1));
 let r=rangeFractions.map(f=>extent*(axis==='x'?f:1-f));if(reverse)r.reverse();
 const s=configured(profile,source,r,{nice,round,clamp:false}),display=s.domain().map(Number),windows=[{start:0,span:1,duplicateGap:false},{start:.2,span:.4,duplicateGap:false}];
 const gap=display.slice(1).findIndex((v,i)=>v===display[i]);if(gap>=0&&source[0]!==source.at(-1)){const lo=Math.min(r[gap],r[gap+1])/extent,width=Math.abs(r[gap+1]-r[gap])/extent;windows.push({start:lo+.2*width,span:.6*width,duplicateGap:true});}
 for(const window of windows){const k=1/window.span,offset=-window.start*extent*k,transform=new Transform(k,axis==='x'?offset:0,axis==='y'?offset:0),next=axis==='x'?transform.rescaleX(s):transform.rescaleY(s),rescaled=next.domain().map(Number),rescaledConstant=rescaled.every(v=>v===rescaled[0]);
  const lo=Math.min(...source),hi=Math.max(...source),span=hi-lo||Math.max(1,Math.abs(lo)*.5);
  let inputs=unique([...source,...display,...source.slice(1).map((v,i)=>(v+source[i])/2),lo-span/4,hi+span/4]);if(profile.kind==='utc')inputs=unique(inputs.map(Math.trunc));
  const unrounded=make(profile.kind,profile.parameter).domain(display).range(r),nextUnrounded=make(profile.kind,profile.parameter).domain(rescaled).range(r);
  const probes=inputs.map(input=>({input,checkedInput:!(profile.kind==='log'||profile.kind==='pow'&&profile.parameter<0)||input!==0&&Math.sign(input)===Math.sign(source[0]),baselinePixel:s(input),viewportPixel:next(input),baselineUnrounded:unrounded(input),viewportUnrounded:nextUnrounded(input)}));
  const rescaledTransformed=rescaled.map(originalTransform(profile,rescaled)),displayTransformed=display.map(originalTransform(profile,display));
  viewportCases.push({name:`${profile.name}/${pattern}/${descending?'descending':'ascending'}/${nice}/${+reverse}${+round}/${axis}/${viewportCases.length}`,kind:profile.kind,parameter:profile.parameter,source,rangeFractions,nice,reverse,round,axis,extent,window,native:{display,range:r,rescaled,initialSourceConstant:source[0]===source.at(-1),rescaledConstant,rescaledMonotonic:monotonic(rescaled),rescaledTransformed,rescaledCheckedProfile:checkedProfile(profile,rescaled,rescaledTransformed),displayCheckedProfile:checkedProfile(profile,display,displayTransformed),probes}});
 }
}
for(const kind of ['linear','pow','utc'])assert(viewportCases.some(c=>c.kind===kind&&c.window.duplicateGap&&c.native.rescaledConstant&&!c.native.initialSourceConstant),`${kind}: missing a genuine original native constant-window profile`);
const counts={cases:cases.length,accepted:cases.filter(c=>c.checkedConstruction==='accept').length,rejected:cases.filter(c=>c.checkedConstruction!=='accept').length,mapQueries:cases.reduce((n,c)=>n+c.native.queries.length,0),inverseQueries:cases.reduce((n,c)=>n+c.native.inverse.length,0),branchInverseQueries:cases.reduce((n,c)=>n+c.native.branches.reduce((m,b)=>m+b.pixels.length,0),0),viewportCases:viewportCases.length,viewportDuplicateGap:viewportCases.filter(c=>c.window.duplicateGap).length,viewportGapStrictConstant:viewportCases.filter(c=>c.window.duplicateGap&&c.native.rescaledConstant).length,viewportGapNativeUlpVariation:viewportCases.filter(c=>c.window.duplicateGap&&!c.native.rescaledConstant).length,viewportTrueTurns:viewportCases.filter(c=>!c.native.rescaledMonotonic).length,viewportInitialConstant:viewportCases.filter(c=>c.native.initialSourceConstant).length,viewportRescaledConstant:viewportCases.filter(c=>c.native.rescaledConstant).length};
const policy={unroundedRelativeTolerance:2e-12,rounding:'Rounded outputs must agree exactly, including zero sign. A one-pixel cross-runtime difference is permitted only when BOTH independently checked unrounded values are within ONE ULP of the SAME half integer; both rounded values must equal ECMAScript Math.round of their respective raw values. No generic pixel allowance.',utcInverse:'Integral Date TimeClip outputs compare exactly; nonfinite or out-of-profile inverses must reject.',globalInverse:'Turning and plateau ranges retain original native inverse values here, while the checked Rust profile rejects global inverse. Explicit flat range branches reject; nonflat branch inverses compare to original two-stop D3.',rawDuplicate:'Raw equal adjacent knots, including all-constant domains, are accepted. Distinct raw knots whose transform collapses or overflows still reject even if native D3 computes a value. Reciprocal nice crossing zero rejects. Original native outputs are retained for every rejected profile and unchecked reciprocal input.',signedZero:'JSON strings -0, NaN, Infinity and -Infinity preserve IEEE special values; raw source stops retain signed zero. Numerical display-stop comparison treats signed zero as equal; rounded outputs preserve native zero sign.'};
const fixture=join(root,'tests/fixtures/piecewise-repeated');await mkdir(fixture,{recursive:true});
const values='{\n"schema":1,"d3ScaleVersion":"4.0.2","d3ZoomVersion":"3.0.0","policy":'+json(policy)+',\n"cases":[\n'+cases.map(json).join(',\n')+'\n],\n"viewportCases":[\n'+viewportCases.map(json).join(',\n')+'\n]}\n';
const sourceSHA256={},licenses=[];
async function inventory(dir,prefix,label){for(const e of await readdir(dir,{withFileTypes:true})){const p=join(dir,e.name);if(e.isDirectory()&&e.name!=='node_modules')await inventory(p,prefix,label);else if(e.isFile()&&(e.name.endsWith('.js')||e.name.endsWith('.mjs')))sourceSHA256[`${label}/${relative(prefix,p).replaceAll('\\','/')}`]=sha(await readFile(p));}}
for(const name of ['d3-scale','d3-array','d3-interpolate','d3-format','d3-time','d3-time-format','d3-color','internmap']){const p=join(modules,name),pkg=JSON.parse(await readFile(join(p,'package.json'),'utf8')),text=await readFile(join(p,'LICENSE'),'utf8');licenses.push({package:name,version:pkg.version,text,sha256:sha(text)});await inventory(p,p,name);}
licenses.push({package:'d3-zoom',version:'3.0.0',text:await readFile(join(zoomRoot,'LICENSE'),'utf8')});sourceSHA256['d3-zoom/src/transform.js']=sha(await readFile(join(zoomRoot,'src/transform.js')));
const reference=JSON.stringify({schema:1,d3ScaleVersion:'4.0.2',d3ZoomVersion:'3.0.0',licenses,sourceSHA256,producerSHA256:sha(await readFile(new URL(import.meta.url))),valuesSHA256:sha(values),counts,policy,nativeFindings:{twoStopConstant:'Always the sole range pair midpoint.',threePlusConstant:'Unclamped below transformed knot uses the first pair midpoint; at/above uses the last pair midpoint. Raw clamp happens before transformed bisect.',tie:'Polymap reverses descending TRANSFORMED knots and range before bisectRight. Negative power reverses raw tie direction; descending raw tie cannot reuse ascending raw selection.',duplicateEdgeClamp:'Whole raw domain clamp changes the selected duplicate edge branch before normalization. Segment clamp alone is not equivalent.',inverse:'Strict range remains invertible despite repeated domain; source-value plateaus are valid outputs.',collapseProfiles:collapseProfiles.map(p=>({name:p.name,domain:p.domain,firstPairNativeMapped:p.domain.slice(0,2).map(x=>make(p.kind,p.parameter).domain(p.domain.slice(0,2)).range([0,1])(x))}))}},(_,v)=>encoded(v),2)+'\n';
for(const [name,text]of [['values.json',values],['reference.json',reference]]){const p=join(fixture,name);if(check)assert.equal(await readFile(p,'utf8'),text,name);else await writeFile(p,text);}
console.log(JSON.stringify({passed:true,checked:check,...counts,valuesSHA256:sha(values)}));
