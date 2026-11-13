// Original Nivo caption callbacks through the real property accessor and Lodash.
// node scripts/box-legend-labels-reference.mjs <released-D3-node_modules> [--shared-vendor <path>] [--check]
import fs from 'node:fs';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {stripTypeScriptTypes} from 'node:module';
import {dirname,join,resolve,relative} from 'node:path';
import {fileURLToPath,pathToFileURL} from 'node:url';
const script=fileURLToPath(import.meta.url),root=resolve(dirname(script),'..'),args=process.argv.slice(2),check=args.includes('--check');
assert(args[0],'Supply released D3 node_modules root');
const modules=resolve(args[0]),fixture=join(root,'tests/fixtures/box-legend-labels'),vendor=join(fixture,'vendor');
const sharedIndex=args.indexOf('--shared-vendor');
const sharedVendor=sharedIndex>=0?resolve(args[sharedIndex+1]):join(root,'tests/fixtures/box-colorby-hover/vendor');
const hash=b=>createHash('sha256').update(b).digest('hex'),sourcePins={},runtimePins={};
function readAt(base,name,key){const b=fs.readFileSync(join(base,name));sourcePins[key+'/'+name]=hash(b);return b.toString('utf8').replaceAll('\r\n','\n');}
const read=name=>readAt(sharedVendor,name,'tests/fixtures/box-colorby-hover/vendor');
const readOwn=name=>readAt(vendor,name,'tests/fixtures/box-legend-labels/vendor');
const pins=JSON.parse(read('source-pins.json')),ownPins=JSON.parse(readOwn('source-pins.json'));
for(const [name,pin]of Object.entries(pins.files)){assert.equal(hash(fs.readFileSync(join(sharedVendor,name))),pin.sha256);read(name);}
for(const [name,pin]of Object.entries(ownPins.files)){assert.equal(hash(fs.readFileSync(join(vendor,name))),pin.sha256);readOwn(name);}
for(const name of ['nivo-LICENSE','d3-scale-LICENSE','d3-array-LICENSE','d3-interpolate-LICENSE','d3-color-LICENSE','internmap-LICENSE'])read(name);
const versions={'d3-scale':'4.0.2','d3-interpolate':'3.0.1','d3-array':'3.2.5','d3-color':'3.1.0','d3-format':'3.1.3'};
function pinPackage(name){const base=join(modules,name);function walk(dir){for(const e of fs.readdirSync(dir,{withFileTypes:true})){const p=join(dir,e.name);if(e.isDirectory())walk(p);else if(e.isFile()&&(/\.(js|mjs|json)$/.test(e.name)||e.name==='LICENSE'))runtimePins[name+'/'+relative(base,p).replaceAll('\\','/')]=hash(fs.readFileSync(p));}}walk(base);}
for(const name of [...Object.keys(versions),'d3-time','d3-time-format','internmap'])pinPackage(name);
function entry(name){const pkg=JSON.parse(fs.readFileSync(join(modules,name,'package.json')));assert.equal(pkg.version,versions[name]);return JSON.stringify(pathToFileURL(join(modules,name,pkg.module)).href);}
for(const name of Object.keys(versions))entry(name);
const lodashURL=pathToFileURL(join(vendor,'lodash.cjs')).href,_=(await import(lodashURL)).default;
assert.equal(_.VERSION,'4.17.21');
const stripImports=s=>s.replace(/^import[\s\S]*?from\s*['"][^'"]+['"]\n/gm,'');
const types=s=>'data:text/javascript;base64,'+Buffer.from(stripTypeScriptTypes(s,{mode:'transform'})).toString('base64');
const adapter=types(`
import _ from ${JSON.stringify(lodashURL)};
let state,memos=[],transitions=[];
export const useMemo=(fn,deps)=>{const value=fn();memos.push({value,deps});return value;};
export const useState=value=>[state===undefined?value:state,()=>{throw Error('Cold adapter does not commit React state');}];
export const prepare=value=>{state=value;memos=[];transitions=[];};
export const records=()=>({memos,transitions});
export const useTheme=()=>({labels:{text:{fill:'rgb(10,20,30)'}}});
export const get=_.get,isPlainObject=_.isPlainObject;
export const useValueFormatter=f=>typeof f==='function'?f:String;
export const unsupported=()=>{throw Error('Outside declared linear/band/array-palette reference');};
export const useTransition=(items,opts)=>{const rows=items.map(item=>({key:opts.keys(item),initial:typeof opts.initial==='function'?opts.initial(item):opts.initial,from:opts.from(item),enter:opts.enter(item),update:opts.update(item),leave:opts.leave(item)}));transitions.push(rows);return render=>items.map((item,i)=>render(rows[i].update,item));};
`);
const adapters=await import(adapter),ts=(name,prefix='')=>types(prefix+'\n'+stripImports(read(name)));
const props=ts('box-props.ts','const BoxPlotItem=()=>{throw Error("UI placeholder not executed")},BoxPlotTooltip=BoxPlotItem,BoxPlotTooltipLabel=BoxPlotItem;');
const defaults=(await import(props)).defaultProps;
const inherited=ts('nivo-inheritedColor.ts',`import{useMemo,get,isPlainObject}from ${JSON.stringify(adapter)};import{rgb}from ${entry('d3-color')};`);
const ordinal=ts('ordinal-color.ts',`import{useMemo,get,isPlainObject,unsupported}from ${JSON.stringify(adapter)};import{scaleOrdinal}from ${entry('d3-scale')};const colorSchemes={};const isCategoricalColorScheme=unsupported,isSequentialColorScheme=unsupported,isDivergingColorScheme=unsupported;`);
const linear=ts('linearScale.ts',`import{scaleLinear}from ${entry('d3-scale')};import{interpolateNumber,interpolateRound}from ${entry('d3-interpolate')};`);
const band=ts('nivo-bandScale.ts',`import{scaleBand}from ${entry('d3-scale')};`);
const dispatch=read('compute.ts').match(/^export function computeScale[\s\S]*?^\}/m)?.[0];assert(dispatch,'complete original dispatch function');
const scales=types(`import{createLinearScale}from ${JSON.stringify(linear)};import{createBandScale}from ${JSON.stringify(band)};import{unsupported}from ${JSON.stringify(adapter)};const createLogScale=unsupported,createSymlogScale=unsupported,createTimeScale=unsupported;\n`+dispatch);
const common=ts('common.ts',`import{computeScale}from ${JSON.stringify(scales)};`);
const stratification=ts('stratification.ts',`import{defaultProps}from ${JSON.stringify(props)};`);
const generation=ts('generation.ts',`import{computeScale}from ${JSON.stringify(scales)};import{getIndexScale}from ${JSON.stringify(common)};`);
const properties=types(`import _ from ${JSON.stringify(lodashURL)};const {isFunction,get}= _;import {format} from ${entry('d3-format')};import {useMemo} from ${JSON.stringify(adapter)};\n`+stripImports(readOwn('nivo-propertiesConverters.js')));
const accessor=await import(properties);
const legends=ts('box-legends.ts',`import{getPropertyAccessor}from ${JSON.stringify(properties)};import _ from ${JSON.stringify(lodashURL)};const{uniqBy,sortBy}= _;`);
const hooks=ts('box-hooks.ts',`import{useMemo,useState,useValueFormatter,useTheme,useTransition}from ${JSON.stringify(adapter)};import{usePropertyAccessor}from ${JSON.stringify(properties)};import{useInheritedColor}from ${JSON.stringify(inherited)};import{useOrdinalColorScale}from ${JSON.stringify(ordinal)};import{defaultProps}from ${JSON.stringify(props)};import{stratifyData,summarizeDistribution}from ${JSON.stringify(stratification)};import{generateBoxPlots}from ${JSON.stringify(generation)};import{getLegendData}from ${JSON.stringify(legends)};`);
const {useBoxPlot,useBoxPlotTransition}=await import(hooks),{getLegendData}=await import(legends);

const rgba=c=>`rgba(${c[0]},${c[1]},${c[2]},${c[3]/255})`,paletteRGBA=[[20,40,60,160],[200,180,160,220],[9,8,7,0],[40,120,180,255]],palette=paletteRGBA.map(rgba);
function bytes(css){const m=css.match(/^rgba?\(\s*([\d.]+),\s*([\d.]+),\s*([\d.]+)(?:,\s*([\d.]+))?\)$/);assert(m,css);return [+m[1],+m[2],+m[3],Math.round((m[4]===undefined?1:+m[4])*255)];}
const clone=x=>JSON.parse(JSON.stringify(x));
const fullData=[['B','y',[1,3]],['B','x',[2,4]],['A','y',[3,5]],['A','x',[4,6]]].flatMap(([group,subGroup,vs])=>vs.map((v,i)=>({id:`${group}/${subGroup}/${i}`,group,subGroup,v})));
const groups=['reserved','B','A'],subGroups=['missing','y','x'];
const shapes=[
 {id:'complete',data:fullData},
 {id:'sparse',data:fullData.filter(d=>!(d.group==='B'&&d.subGroup==='x'))},
 {id:'deleted-group-B',data:fullData.filter(d=>d.group!=='B')},
 {id:'deleted-subgroup-y',data:fullData.filter(d=>d.subGroup!=='y')},
 {id:'empty-reserved-domain',data:[]},
];
const legendProps=[{anchor:'bottom',direction:'row',itemWidth:90,itemHeight:16},{anchor:'right',direction:'column',itemWidth:100,itemHeight:18}];
function descriptor(colorBy,variant){
 const keys=colorBy==='group'?groups:subGroups;
 const unique=colorBy==='group'?{reserved:'Unused group',B:'South',A:'North'}:{missing:'Unused subgroup',y:'Plan',x:'Observed'};
 return {kind:'map',variant,map:Object.fromEntries(keys.map(key=>[key,variant==='unique'?unique[key]:variant==='duplicate'?'All observations':'']))};
}
function options(input,callback){return {data:input.data,value:'v',groupBy:'group',subGroupBy:'subGroup',groups:input.groups,subGroups:input.subGroups,width:480,height:300,layout:'vertical',minValue:'auto',maxValue:'auto',valueScale:{type:'linear',nice:false},indexScale:{type:'band',round:false},padding:.1,innerPadding:4,colorBy:input.colorBy,colors:palette,opacity:.8,activeOpacity:.9,inactiveOpacity:.2,borderColor:defaults.borderColor,medianColor:defaults.medianColor,whiskerColor:defaults.whiskerColor,tooltipLabel:d=>d.group+'/'+d.subGroup,valueFormat:String,legendLabel:callback,legends:legendProps};}
function run(input,activeItem=null){
 const calls=[];
 const callback=datum=>{
  const selectedKey=datum[input.colorBy];
  const label=input.caption.kind==='pair-outside-map'?`${datum.group}/${datum.subGroup}`:Object.hasOwn(input.caption.map,selectedKey)?input.caption.map[selectedKey]:selectedKey;
  calls.push({group:datum.group,subGroup:datum.subGroup,groupIndex:datum.groupIndex,subGroupIndex:datum.subGroupIndex,n:datum.n,selectedKey,returnedCaption:label});
  return label;
 };
 assert.equal(accessor.getPropertyAccessor(callback),callback,'Actual original core accessor must preserve the configured function identity');
 adapters.prepare(activeItem);const opts=options(input,callback);assert.equal(typeof opts.legendLabel,'function');
 const computed=useBoxPlot(opts);
 const activeMemos=adapters.records().memos.filter(r=>Array.isArray(r.deps)&&r.deps.length===2&&r.deps[1]===computed.activeItem&&Array.isArray(r.deps[0])&&r.deps[0].every(x=>x&&typeof x==='object'&&'key'in x)&&Array.isArray(r.value)&&r.value.every(x=>typeof x==='string'));
 assert.equal(activeMemos.length,1,'Capture the original activeKeys return, never reconstruct its predicate');
 const direct=getLegendData({boxPlots:computed.boxPlots,dataFrom:input.colorBy,legendLabel:opts.legendLabel});
 for(const [props,data]of computed.legendsData){assert(legendProps.some(p=>p===props));assert.deepEqual(data,direct);}
 useBoxPlotTransition({...computed,animate:false,springConfig:{}});assert.equal(adapters.records().transitions.length,1);
 const snapshot={sourceActiveItem:clone(computed.activeItem),activeKeys:clone(activeMemos[0].value),ordinalDomain:computed.getColor.scale.domain(),originalAccessorKeepsCallback:accessor.getPropertyAccessor(callback)===callback,legendLabelRuntimeType:typeof opts.legendLabel,callbackCalls:calls,
  boxes:computed.boxPlots.map((b,i)=>({key:b.key,group:b.group,subGroup:b.subGroup,sourceSummary:clone(b.data),numericColor:b.color,numericRGBA:bytes(b.color),opacity:b.opacity,position:{x:b.x,y:b.y,width:b.width,height:b.height},transition:clone(adapters.records().transitions[0][i])})),
  legendsData:computed.legendsData.map(([props,data])=>({legend:clone(props),data:data.map(d=>({...d,rgba:bytes(d.color),sourceKey:(input.colorBy==='group'?input.groups:input.subGroups)[d.id]}))}))};
 return {computed,snapshot};
}
const result={schema:'box-legend-labels-original-v1',profiles:[],qualifications:[]};
for(const colorBy of ['group','subGroup'])for(const variant of ['unique','duplicate','empty'])for(const shape of shapes){
 const input={shape:shape.id,colorBy,groups,subGroups,data:clone(shape.data),paletteRGBA,palette,caption:descriptor(colorBy,variant)};
 const baseline=run(input),activeItem=baseline.computed.boxPlots[0]??null,active=run(input,activeItem),restored=run(input);
 assert.deepEqual(restored.snapshot,baseline.snapshot);
 result.profiles.push({id:`${colorBy}-${variant}-${shape.id}`,input,baseline:baseline.snapshot,active:active.snapshot,restored:restored.snapshot,restorationExact:true,rustScope:{keyStableCaptionMap:true,sourceKeyAnnotationNotNativeField:true,numericLegendIDsNeedTranslation:true,nativeDeduplicatesByCaption:true,rustRetainsIdentityDespiteDuplicateOrEmptyCaption:true,legacyRustSubgroupLegendRetainsFullDomain:colorBy==='subGroup',nativeSourceDeletionNotRustHiddenAPI:true}});
}
function qualify(id,input,scope){const out=run(input);result.qualifications.push({id,input:clone(input),actual:out.snapshot,scope});}
for(const colorBy of ['group','subGroup']){
 const map=colorBy==='group'?{A:'North'}:{x:'Observed'};
 qualify(`${colorBy}-partial-map`,{shape:'complete',colorBy,groups,subGroups,data:fullData,paletteRGBA,palette,caption:{kind:'map',variant:'partial',map}},'Missing entries take the native raw selected identity. Rust empty-subgroup fallback Samples is separately recorded.');
}
qualify('group-one-key-multiple-labels',{shape:'complete',colorBy:'group',groups,subGroups,data:fullData,paletteRGBA,palette,caption:{kind:'pair-outside-map',variant:'outside-key-stable-map'}},'Actual original callback can yield distinct labels for the same selected group, preserving repeated native numeric IDs. This is not representable by the bounded key-stable caption map and is not claimed as supported.');
const single={shape:'empty-raw-subgroup',colorBy:'subGroup',groups:['reserved','A'],subGroups:[''],data:[{id:'zero',group:'A',subGroup:'',v:0}],paletteRGBA,palette};
qualify('empty-raw-subgroup-explicit-empty',{...single,caption:{kind:'map',variant:'explicit-empty',map:{'':''}}},'Explicit empty caption is accepted natively and by the Rust key-stable map.');
qualify('empty-raw-subgroup-unmapped-fallback',{...single,caption:{kind:'map',variant:'empty-map',map:{}}},'Native fallback raw identity is empty. Rust None/empty-map preserves its legacy Samples caption; this difference is not silently removed.');
qualify('native-unused-extra-map-key',{shape:'complete',colorBy:'group',groups,subGroups,data:fullData,paletteRGBA,palette,caption:{kind:'map',variant:'unused-extra',map:{unknown:'Unused'}}},'Map is a callback-input descriptor, not a native prop. Native callback ignores this unused key; the Rust checked map rejects keys outside the complete selected domain.');
qualify('native-caption-over-rust-byte-budget',{shape:'complete',colorBy:'group',groups,subGroups,data:fullData,paletteRGBA,palette,caption:{kind:'map',variant:'oversize',map:{A:'a'.repeat(1025)}}},'Actual native helper returns this string. Rust caps every configured caption at 1024 UTF-8 bytes before emission, including unused entries. No React/pixel layout for oversized native text is claimed.');
assert.equal(result.profiles.length,30);assert.equal(result.qualifications.length,7);
for(const p of result.profiles){const domain=p.input.colorBy==='group'?p.input.groups:p.input.subGroups;for(const snapshot of [p.baseline,p.active,p.restored]){assert.deepEqual(snapshot.ordinalDomain,domain);assert.equal(snapshot.legendLabelRuntimeType,'function');assert(snapshot.originalAccessorKeepsCallback);assert.deepEqual(snapshot.legendsData[0].data,snapshot.legendsData[1].data);}}
const snapshots=result.profiles.flatMap(p=>[p.baseline,p.active,p.restored]).concat(result.qualifications.map(q=>q.actual));
const values=JSON.stringify(result,null,2)+'\n';
const metadata={schema:result.schema,nivoVersion:pins.nivoVersion,nivoCommit:pins.nivoCommit,lodashVersion:_.VERSION,producerSHA256:hash(fs.readFileSync(script)),counts:{profiles:result.profiles.length,qualificationSnapshots:result.qualifications.length,actualSnapshots:snapshots.length,actualComputedBoxRecords:snapshots.reduce((n,s)=>n+s.boxes.length,0),actualLegendRowsAcrossTwoConfigurations:snapshots.reduce((n,s)=>n+s.legendsData.reduce((v,l)=>v+l.data.length,0),0),actualCallbackCalls:snapshots.reduce((n,s)=>n+s.callbackCalls.length,0)},valuesSHA256:hash(values),sourcePins,runtimePins,
 adapters:{react:'Cold original memo evaluation and supplied original computed active item; no mounted lifecycle, state commits or event/UI rendering.',property:'Complete original Nivo propertiesConverters.js; actual getPropertyAccessor and usePropertyAccessor receive the function and preserve its reference. Actual released Lodash get/isFunction/isPlainObject/uniqBy/sortBy, not replacement algorithms.',spring:'Complete original useBoxPlotTransition records from/enter/update/leave with a synchronous recording adapter; no Spring integration/animation.',scales:'Actual released D3 linear/band/array ordinal, original full dispatcher retained; other scale branches reject execution.'},
 scope:'Full original hook, palette prewarming, property accessor, legend helper and transition functions execute. A JSON map descriptor becomes an actual callback input, not an expected legend-row generator. Native label deduplication and OR association outputs are never replaced or independently reconstructed. Rust stable IDs/full SubGroup domain/empty fallback/checked keys and byte bounds remain explicit profile differences.'};
for(const [name,content]of [['values.json',values],['reference.json',JSON.stringify(metadata,null,2)+'\n']]){const p=join(fixture,name);if(check)assert.equal(fs.readFileSync(p,'utf8'),content);else fs.writeFileSync(p,content);}
console.log(JSON.stringify({passed:true,checked:check,...metadata.counts,valuesSHA256:metadata.valuesSHA256}));
