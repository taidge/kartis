// Check-only original D3 execution. No package installation, Rust, or network access.
import assert from 'node:assert/strict';
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import vm from 'node:vm';
import {dirname,resolve} from 'node:path';
import {fileURLToPath} from 'node:url';

const sha=bytes=>createHash('sha256').update(bytes).digest('hex');
const number=value=>Number.isNaN(value)?'NaN':value===Infinity?'+Infinity':value===-Infinity?'-Infinity':Object.is(value,-0)?'-0':String(value);
export const token=value=>value===undefined?'u':value===null?'z':typeof value==='number'?`n:${number(value)}`:typeof value==='boolean'?`b:${+value}`:typeof value==='string'?`s:${Buffer.from(value).toString('hex')}`:typeof value==='bigint'?`i:${value}`:typeof value==='symbol'?'y:symbol':`o:${number(value.valueOf())}`;
const pack=values=>Array.from(values,token);
const view=new DataView(new ArrayBuffer(8));
const nextUp=value=>{if(Number.isNaN(value)||value===Infinity)return value;if(value===0)return Number.MIN_VALUE;view.setFloat64(0,value);view.setBigUint64(0,view.getBigUint64(0)+(value>0?1n:-1n));return view.getFloat64(0);};
const nextDown=value=>-nextUp(-value);
const queryProfile=value=>typeof value==='number'&&Number.isFinite(value)?'finite-number':value==null||typeof value==='number'&&Number.isNaN(value)?'native-missing':'native-query-coercion-or-infinity';

function inputs(){
  const cases=[];
  const add=(id,domain,profile='finite-domain-n-ge-2')=>cases.push({id,domain,profile});
  add('empty',[],'empty-native-I-minus-zero');
  for(const [id,value]of [['zero',0],['minus-zero',-0],['positive',7],['negative',-7],['subnormal',Number.MIN_VALUE],['max',Number.MAX_VALUE],['min',-Number.MAX_VALUE]])add(`singleton-${id}`,[value],'singleton-native-I-NaN');
  for(const [id,value]of [['zero',0],['minus-zero',-0],['positive',7],['negative',-7],['subnormal',Number.MIN_VALUE],['max',Number.MAX_VALUE]])for(const count of [2,3,5,17])add(`constant-${id}-${count}`,Array(count).fill(value));
  const shapes=[
    ['two',[0,1]],['uneven',[0,1,10,100]],['offset',[10,11,12,1000]],['signed',[-10,-1,0,2]],
    ['extremes',[-Number.MAX_VALUE,0,Number.MAX_VALUE]],
    ['subnormal',[Number.MIN_VALUE,2*Number.MIN_VALUE,3*Number.MIN_VALUE,16*Number.MIN_VALUE]],
    ['adjacent-max',[nextDown(nextDown(Number.MAX_VALUE)),nextDown(Number.MAX_VALUE),Number.MAX_VALUE]],
    ['adjacent-unit',[1,nextUp(1),nextUp(nextUp(1)),2]],
    ['ties-first',[0,0,5,10]],['ties-middle',[-1,5,5,10]],['ties-last',[0,5,10,10]],
    ['ties-all',[0,0,5,10,10]],['zero-order',[0,-0,1,1]],['minus-zero-order',[-0,0,1,1]],
    ['mixed-zero',[-1,0,-0,0,-0,1]],['small-decimal',[-.7,-.1,.01,.3,.9]],
  ];
  for(const [id,domain]of shapes){add(`${id}-authored`,domain);add(`${id}-reverse`,[...domain].reverse());add(`${id}-rotated`,domain.slice(1).concat(domain[0]));}
  for(let n=2;n<=33;n++){
    const domain=Array.from({length:n},(_,i)=>((i*17+n*13)%23-11)**3/10);
    add(`deterministic-${n}`,domain);
  }
  for(const n of [255,256,257])add(`rank-boundary-${n}`,Array.from({length:n},(_,i)=>i*i-20));
  add('budget-20000-ties',Array.from({length:20000},(_,i)=>(i*37)%101-50));
  add('budget-20000-distinct',Array.from({length:20000},(_,i)=>(i*8191)%20000-10000));
  add('coerced-primitives',[null,undefined,NaN,'2','-0',true,false,'bad',Infinity,-Infinity],'native-domain-coercion-and-infinity');
  add('coerced-numeric-text',['',' ','0x10','1e2','-4','.5','Infinity','-Infinity','NaN'],'native-domain-coercion-and-infinity');
  add('infinite-pair',[-Infinity,Infinity],'native-domain-infinity');
  add('infinite-ties',[-Infinity,-Infinity,0,Infinity,Infinity],'native-domain-infinity');
  add('positive-infinity-only',[Infinity,Infinity],'native-domain-infinity');
  add('negative-infinity-only',[-Infinity,-Infinity],'native-domain-infinity');
  add('valueof-objects',[{valueOf:()=>3},{valueOf:()=>-0},{valueOf:()=>NaN},1],'native-domain-object-coercion');
  return cases;
}
function queriesFor(sorted){
  const all=new Map();
  const add=value=>all.set(token(value),value);
  [-100,-10,-1,-0,0,Number.MIN_VALUE,1,5,7,10,100,-Infinity,Infinity,NaN,null,undefined,'5','oops','',true,false].forEach(add);
  const unique=sorted.filter((value,i)=>!i||value!==sorted[i-1]);
  const selected=unique.length<=257?unique:[...new Set([0,1,2,255,256,Math.floor(unique.length/4),Math.floor(unique.length/2),Math.floor(unique.length*3/4),unique.length-3,unique.length-2,unique.length-1])].map(i=>unique[i]);
  selected.forEach(value=>{add(nextDown(value));add(value);add(nextUp(value));});
  for(let i=1;i<selected.length;i++){const a=selected[i-1],b=selected[i];if(Number.isFinite(a)&&Number.isFinite(b)){const midpoint=a/2+b/2;add(midpoint);}}
  return [...all.values()];
}
const attempt=fn=>{try{return{value:token(fn())};}catch(error){return{error:{name:error.name,message:error.message}};}};

export function generate(d3){
  assert.equal(typeof d3.scaleSequentialQuantile,'function');
  const cases=[];
  for(const spec of inputs()){
    const calls=[];
    const scale=d3.scaleSequentialQuantile().domain(spec.domain).interpolator(value=>{calls.push(value);return value;});
    const sorted=Array.from(scale.domain());
    const queries=queriesFor(sorted);
    const mapped=queries.map(value=>{calls.length=0;const output=attempt(()=>scale(value));return{input:token(value),profile:queryProfile(value),...output,interpolatorArguments:pack(calls)};});
    const range=pack(scale.range());
    const quantiles=[0,1,2,3,4,7,16,31,256].map(segments=>({segments,values:pack(scale.quantiles(segments)),profile:segments===0?'native-zero-count':'positive-integer-count'}));
    const getter=scale.domain();
    if(getter.length)getter[0]=-123456.25;
    getter.push(765432.5);
    const freshGetter=pack(scale.domain());
    assert.deepEqual(freshGetter,pack(sorted));
    const copied=scale.copy();
    const copyGetter=copied.domain();copyGetter.push(222222.5);
    scale.domain([37,41]).interpolator(value=>10+value);
    const copy={domain:pack(copied.domain()),range:pack(copied.range()),queries:queries.map(value=>({input:token(value),...attempt(()=>copied(value))})),originalAfterMutation:{domain:pack(scale.domain()),range:pack(scale.range()),queries:[36,37,39,41,42].map(value=>({input:token(value),...attempt(()=>scale(value))}))}};
    assert.deepEqual(copy.domain,pack(sorted));assert.deepEqual(copy.range,range);
    assert.deepEqual(copy.queries.map(row=>row.value),mapped.map(row=>row.value));
    cases.push({id:spec.id,profile:spec.profile,sourceDomain:pack(spec.domain),domain:pack(sorted),queries:mapped,range,quantiles,getter:{mutatedReturnedArray:pack(getter),freshDomain:freshGetter},copy,methods:Object.fromEntries(['clamp','unknown','ticks','invert','invertExtent','quantiles','copy'].map(name=>[name,typeof copied[name]]))});
  }
  const nativeExceptions=[];
  for(const [label,value]of [['symbol',Symbol('original-query')],['bigint',1n]]){
    nativeExceptions.push({operation:'domain',input:label,...attempt(()=>d3.scaleSequentialQuantile().domain([0,value,1]))});
    nativeExceptions.push({operation:'query',input:label,...attempt(()=>d3.scaleSequentialQuantile().domain([0,1])(value))});
  }
  const countScale=d3.scaleSequentialQuantile().domain([0,1,10,100]);
  const nativeCounts=[];
  for(const count of [-1,-.5,1.5,NaN,null,undefined,'2'])nativeCounts.push({input:token(count),values:pack(countScale.quantiles(count)),profile:'native-count-coercion-not-rust-contract'});
  const records={schema:'kartis-sequential-quantile-original-d3-v1',numberEncoding:'n:<Number.toString>, n:-0, n:NaN, n:+Infinity, n:-Infinity; u=undefined; z=null; b:0/1; s:<UTF8 hex>; i:<bigint>; y:symbol; o:<numeric valueOf>',scope:'Actual original functions only; finite-domain n>=2 comparisons separated from empty/singleton, JS coercion, infinities, exceptions and invalid count profiles',cases,nativeExceptions,nativeCounts};
  const csv=['# C|id|profile|source-domain|sorted-domain; Q|id|index|query-profile|input|output|interpolator-arguments; R|id|range; P|id|segments|quantile-values; G|id|mutated-getter|fresh-domain; Y|id|copied-domain|copied-range; M|id|copy-query-index|input|output. All scalar tokens preserve raw types/nonfinite/negative zero.'];
  for(const row of cases){csv.push(['C',row.id,row.profile,row.sourceDomain.join(','),row.domain.join(',')].join('|'));row.queries.forEach((q,i)=>csv.push(['Q',row.id,i,q.profile,q.input,q.value??`error:${q.error.name}`,q.interpolatorArguments.join(',')].join('|')));csv.push(['R',row.id,row.range.join(',')].join('|'));row.quantiles.forEach(q=>csv.push(['P',row.id,q.segments,q.values.join(',')].join('|')));csv.push(['G',row.id,row.getter.mutatedReturnedArray.join(','),row.getter.freshDomain.join(',')].join('|'));csv.push(['Y',row.id,row.copy.domain.join(','),row.copy.range.join(',')].join('|'));row.copy.queries.forEach((q,i)=>csv.push(['M',row.id,i,q.input,q.value??`error:${q.error.name}`].join('|')));}
  const counts={cases:cases.length,finiteDomainNAtLeastTwo:cases.filter(row=>row.profile==='finite-domain-n-ge-2').length,emptyDomain:cases.filter(row=>row.profile==='empty-native-I-minus-zero').length,singletonDomain:cases.filter(row=>row.profile==='singleton-native-I-NaN').length,nativeDomainProfiles:cases.filter(row=>row.profile.startsWith('native-')).length,queries:cases.reduce((sum,row)=>sum+row.queries.length,0),finiteNumberQueries:cases.reduce((sum,row)=>sum+row.queries.filter(q=>q.profile==='finite-number').length,0),finiteDomainNAtLeastTwoNumberQueries:cases.filter(row=>row.profile==='finite-domain-n-ge-2').reduce((sum,row)=>sum+row.queries.filter(q=>q.profile==='finite-number').length,0),quantileRequests:cases.reduce((sum,row)=>sum+row.quantiles.length,0),rangeValues:cases.reduce((sum,row)=>sum+row.range.length,0),copyQueries:cases.reduce((sum,row)=>sum+row.copy.queries.length,0),getterCases:cases.length,nativeExceptions:nativeExceptions.length,nativeCountProfiles:nativeCounts.length};
  return{json:JSON.stringify(records,null,2)+'\n',csv:csv.join('\n')+'\n',counts};
}

async function main(){
  const args=process.argv.slice(2),receiptIndex=args.indexOf('--receipt');
  assert(args.includes('--check'),'Use --check; original reference files are never rewritten');
  assert(args.every((arg,i)=>arg==='--check'||arg==='--receipt'||receiptIndex>=0&&i===receiptIndex+1),'Unknown argument');
  if(receiptIndex>=0)assert(args[receiptIndex+1],'Missing receipt path');
  const fixtures=new URL('../tests/fixtures/',import.meta.url);
  const manifestBytes=await readFile(new URL('sequential-quantile-d3-source.json',fixtures));
  const manifest=JSON.parse(manifestBytes);
  assert.equal(manifest.schema,'kartis-sequential-quantile-original-d3-source-v1');
  assert.equal(manifest.packages.find(row=>row.name==='d3-scale').version,'4.0.2');
  assert.equal(manifest.packages.find(row=>row.name==='d3-array').version,'3.2.5');
  const bundle=await readFile(new URL(manifest.bundle.file,fixtures));
  assert.equal(bundle.length,manifest.bundle.bytes);assert.equal(sha(bundle),manifest.bundle.sha256);
  for(const row of manifest.packages){const segment=bundle.subarray(row.start,row.start+row.bytes);assert.equal(segment.length,row.bytes);assert.equal(sha(segment),row.sha256);assert(segment.toString('utf8').slice(0,180).includes(`v${row.version}`));}
  const licenses=await readFile(new URL(manifest.licenses.file,fixtures));assert.equal(sha(licenses),manifest.licenses.sha256);
  for(const row of manifest.licenses.packages){assert.equal(Buffer.byteLength(row.text),row.bytes);assert.equal(sha(row.text),row.sha256);assert(licenses.toString('utf8').includes(`===== ${row.name} ${row.version} =====\n`+row.text));}
  const sourceBytes=await readFile(new URL(manifest.originalSources.file,fixtures));assert.equal(sourceBytes.length,manifest.originalSources.bytes);assert.equal(sha(sourceBytes),manifest.originalSources.sha256);
  for(const row of manifest.originalSources.segments)assert.equal(sha(sourceBytes.subarray(row.start,row.start+row.bytes)),row.sha256);
  assert.equal(sha(await readFile(new URL('./continuous-quantile-reference.mjs',import.meta.url))),manifest.producerSha256);
  const context=vm.createContext({});new vm.Script(bundle.toString('utf8'),{filename:'original-sequential-quantile-d3.js'}).runInContext(context,{timeout:5000});
  const generated=generate(context.d3);
  for(const [file,text]of Object.entries({'sequential-quantile-d3.json':generated.json,'sequential-quantile-d3.csv':generated.csv})){const expected=await readFile(new URL(file,fixtures));assert.equal(expected.length,manifest.fixtures[file].bytes);assert.equal(sha(expected),manifest.fixtures[file].sha256);assert(expected.equals(Buffer.from(text)),`Original D3 reference mismatch: ${file}`);}
  assert.deepEqual(generated.counts,manifest.counts);
  const output={passed:true,scope:'Portable original-D3 candidate reference check only; no implemented Rust color feature, consumer or browser acceptance claim',versions:manifest.packages.map(({name,version})=>({name,version})),bundleSha256:sha(bundle),manifestSha256:sha(manifestBytes),producerSha256:manifest.producerSha256,counts:generated.counts,profiles:manifest.profileDifferences};
  const path=receiptIndex>=0?resolve(args[receiptIndex+1]):fileURLToPath(new URL('../target/sequential-quantile-reference-receipt.json',import.meta.url));await mkdir(dirname(path),{recursive:true});await writeFile(path,JSON.stringify(output,null,2)+'\n');console.log(JSON.stringify(output));
}
if(process.argv[1]&&resolve(process.argv[1])===fileURLToPath(import.meta.url))await main();
