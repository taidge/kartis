// Whole original Nivo ByLabel pipeline, real property accessor and genuine Lodash.
// node scripts/box-legend-factory-reference.mjs <released-D3-node_modules> [--check]
import fs from 'node:fs';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {stripTypeScriptTypes} from 'node:module';
import {dirname,join,resolve,relative} from 'node:path';
import {fileURLToPath,pathToFileURL} from 'node:url';
const script=fileURLToPath(import.meta.url),root=resolve(dirname(script),'..'),args=process.argv.slice(2),check=args.includes('--check');
assert(args[0],'Supply released D3 node_modules root');
const modules=resolve(args[0]),fixture=join(root,'tests/fixtures/box-legend-factory'),vendor=join(fixture,'vendor');
const sharedVendor=vendor;
const hash=b=>createHash('sha256').update(b).digest('hex'),sourcePins={},runtimePins={};
function readAt(base,name,key){const b=fs.readFileSync(join(base,name));sourcePins[key+'/'+name]=hash(b);return b.toString('utf8').replaceAll('\r\n','\n');}
const read=name=>readAt(sharedVendor,name,'tests/fixtures/box-legend-factory/vendor');
const readOwn=name=>readAt(vendor,name,'tests/fixtures/box-legend-factory/vendor');
const pins=JSON.parse(read('source-pins.json'));
for(const [name,pin]of Object.entries(pins.files)){assert.equal(hash(fs.readFileSync(join(sharedVendor,name))),pin.sha256);read(name);}
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
let state,memos=[],transitions=[],legendOps=[];
export const recordLegendOp=(kind,input,output)=>legendOps.push({kind,input,output});
export const useMemo=(fn,deps)=>{const value=fn();memos.push({value,deps});return value;};
export const useState=value=>[state===undefined?value:state,()=>{throw Error('Cold adapter does not commit React state');}];
export const prepare=value=>{state=value;memos=[];transitions=[];legendOps=[];};
export const records=()=>({memos,transitions,legendOps});
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
const legends=ts('box-legends.ts',`import{getPropertyAccessor}from ${JSON.stringify(properties)};import _ from ${JSON.stringify(lodashURL)};import{recordLegendOp}from ${JSON.stringify(adapter)};const uniqBy=(input,...args)=>{const output=_.uniqBy(input,...args);recordLegendOp('uniqBy',input,output);return output;};const sortBy=(input,...args)=>{const output=_.sortBy(input,...args);recordLegendOp('sortBy',input,output);return output;};`);
const hooks=ts('box-hooks.ts',`import{useMemo,useState,useValueFormatter,useTheme,useTransition}from ${JSON.stringify(adapter)};import{usePropertyAccessor}from ${JSON.stringify(properties)};import{useInheritedColor}from ${JSON.stringify(inherited)};import{useOrdinalColorScale}from ${JSON.stringify(ordinal)};import{defaultProps}from ${JSON.stringify(props)};import{stratifyData,summarizeDistribution}from ${JSON.stringify(stratification)};import{generateBoxPlots}from ${JSON.stringify(generation)};import{getLegendData}from ${JSON.stringify(legends)};`);
const {useBoxPlot,useBoxPlotTransition}=await import(hooks),{getLegendData}=await import(legends);

const clone=x=>JSON.parse(JSON.stringify(x));
const paletteRGBA=[[20,40,60,160],[200,180,160,220],[9,8,7,0],[40,120,180,255]];
const palette=paletteRGBA.map(c=>`rgba(${c[0]},${c[1]},${c[2]},${c[3]/255})`);
const quantileProbabilities=[.1,.25,.5,.75,.9];
function rgba(css){const m=css.match(/^rgba?\(\s*([\d.]+),\s*([\d.]+),\s*([\d.]+)(?:,\s*([\d.]+))?\)$/);assert(m,css);return [+m[1],+m[2],+m[3],Math.round((m[4]===undefined?1:+m[4])*255)];}
const complete=[['B','y',[1,3]],['B','x',[2,4]],['A','y',[3,5]],['A','x',[4,6]]].flatMap(([group,subGroup,values])=>values.map((v,i)=>({id:`${group}/${subGroup}/${i}`,group,subGroup,v})));
const groups=['reserved','B','A'],subGroups=['missing','y','x'];
const legend={anchor:'bottom',direction:'row',itemWidth:90,itemHeight:16};
const shapes=[
 {id:'complete',groups,subGroups,data:complete},
 {id:'shuffled-full-domain',groups:['A','reserved','B'],subGroups:['x','missing','y'],data:[...complete].reverse()},
 {id:'partial-real-zero',groups,subGroups,data:complete.filter(d=>!(d.group==='B'&&d.subGroup==='x')).map(d=>({...d,v:d.group==='A'&&d.subGroup==='x'?0:d.v}))},
 {id:'empty-reserved-domain',groups,subGroups,data:[]},
];
function options(input,legendLabel){return {data:input.effectiveData,value:'v',groupBy:'group',subGroupBy:'subGroup',groups:input.groups,subGroups:input.subGroups,quantiles:quantileProbabilities,width:480,height:300,layout:'vertical',minValue:'auto',maxValue:'auto',valueScale:{type:'linear',nice:false},indexScale:{type:'band',round:false},padding:.1,innerPadding:4,colorBy:input.colorBy,colors:palette,opacity:.8,activeOpacity:.9,inactiveOpacity:.2,borderColor:defaults.borderColor,medianColor:defaults.medianColor,whiskerColor:defaults.whiskerColor,tooltipLabel:d=>d.group+'/'+d.subGroup,valueFormat:String,legendLabel,legends:input.legendEnabled?[legend]:[]};}
function descriptorLabel(descriptor,d){
 switch(descriptor.kind){
  case 'pair':return `${d.group} / ${d.subGroup} Å <>&`;
  case 'constant':return 'same caption';
  case 'raw-key':return d[descriptor.colorBy];
  case 'empty':return '';
  default:throw Error('Unknown declared callback input '+descriptor.kind);
 }
}
function run(input){
 const calls=[];
 const callback=d=>{const label=descriptorLabel(input.label,d);calls.push({datum:d,label});return label;};
 const legendLabel=input.label.kind==='property'?input.label.path:callback;
 if(typeof legendLabel==='function')assert.equal(accessor.getPropertyAccessor(legendLabel),callback);
 adapters.prepare(null);const computed=useBoxPlot(options(input,legendLabel));
 const records=adapters.records(),ops=records.legendOps;
 assert.equal(ops.length,input.legendEnabled?2:0,'Only the original hook calls the observed legend helper');
 if(input.legendEnabled){assert.equal(ops[0].kind,'uniqBy');assert.equal(ops[1].kind,'sortBy');assert.equal(ops[0].input.length,computed.boxPlots.length);assert.deepEqual(ops[1].input,ops[0].output);assert.equal(computed.legendsData.length,1);assert.equal(computed.legendsData[0][1],ops[1].output,'Actual hook output object is the original sortBy result');}
 const identities=computed.boxPlots.map((b,summaryIndex)=>({summaryIndex,key:b.key,group:b.group,subGroup:b.subGroup,groupIndex:b.data.groupIndex,subGroupIndex:b.data.subGroupIndex,cellIndex:b.data.groupIndex*input.subGroups.length+b.data.subGroupIndex,colorIndex:input.colorBy==='group'?b.data.groupIndex:b.data.subGroupIndex,rawColorKey:b.data[input.colorBy]}));
 const observations=d=>input.rawData.flatMap((row,i)=>row.group===d.group&&row.subGroup===d.subGroup?[i]:[]);
 const effectiveObservations=d=>input.effectiveData.flatMap((row,i)=>row.group===d.group&&row.subGroup===d.subGroup?[i]:[]);
 const boxes=computed.boxPlots.map((b,i)=>({...identities[i],sourceSummary:clone(b.data),quantileProbabilities:clone(b.data.quantiles),computedStatistics:clone(b.data.values),observationIndices:observations(b.data),effectiveObservationIndices:effectiveObservations(b.data),numericColor:b.color,numericRGBA:rgba(b.color),opacity:b.opacity,position:{x:b.x,y:b.y,width:b.width,height:b.height},coordinates:clone(b.coordinates),formatted:clone(b.formatted)}));
 const callbackContexts=calls.map(({datum,label})=>{const i=computed.boxPlots.findIndex(b=>b.data===datum);assert(i>=0,'Original property callback receives the same actual summary object used by the computed box');assert.deepEqual(datum.quantiles,quantileProbabilities);return {...identities[i],data:clone(datum),observationIndices:observations(datum),effectiveObservationIndices:effectiveObservations(datum),quantileProbabilities:clone(datum.quantiles),computedStatistics:clone(datum.values),returnedLabel:label};});
 const expectedLegend=input.legendEnabled?computed.legendsData[0][1].map((row,rowIndex)=>{const index=ops[0].input.indexOf(row);assert(index>=0,'Both real Lodash functions retain the selected original row reference');const representative=identities[index];assert.equal(row.id,representative.colorIndex);assert.equal(row.color,computed.boxPlots[index].color);return {rowIndex,nativeId:row.id,fullColorIndex:row.id,label:row.label,color:row.color,rgba:rgba(row.color),representative:clone(representative),target:{kind:input.colorBy==='group'?'Category':'Series',rawKey:representative.rawColorKey},representativeInputIndex:index};}):[];
 const originalLegendOperations=ops.map(op=>({kind:op.kind,input:clone(op.input),output:clone(op.output),selectedInputIndices:op.output.map(row=>op.input.indexOf(row))}));
 useBoxPlotTransition({...computed,animate:false,springConfig:{}});
 assert.equal(adapters.records().transitions.length,1);
 const transitions=clone(adapters.records().transitions[0]);
 assert.equal(callbackContexts.length,input.legendEnabled&&typeof legendLabel==='function'?boxes.length:0,'This cold invocation has one legend spec, not a React lifecycle callback promise');
 return {ordinalDomain:clone(computed.getColor.scale.domain()),boxes,callbackContexts,expectedLegend,originalLegendOperations,transitions,configuredQuantileProbabilities:quantileProbabilities,nativeLegendConfigurations:input.legendEnabled?1:0,originalAccessorKind:typeof legendLabel,actualAccessorPreservesFunction:typeof legendLabel==='function'?accessor.getPropertyAccessor(legendLabel)===callback:null};
}
function inputFor(shape,colorBy,label,overrides={}){return {shape:shape.id,colorBy,groups:clone(shape.groups),subGroups:clone(shape.subGroups),rawData:clone(shape.data),effectiveData:clone(shape.data),hiddenGroups:[],hiddenSubGroups:[],legendEnabled:true,label:{kind:label,colorBy},paletteRGBA,quantileProbabilities,...overrides};}
const result={schema:'box-legend-factory-original-v1',profiles:[],qualifications:[]};
for(const colorBy of ['group','subGroup'])for(const shape of shapes)for(const label of ['pair','constant','raw-key','empty']){
 const input=inputFor(shape,colorBy,label),actual=run(input),repeated=run(input);assert.deepEqual(actual,repeated,'Cold deterministic complete original pipeline');
 result.profiles.push({id:`${colorBy}-${shape.id}-${label}`,input,actual,rustScope:{roleWiring:true,byLabelSummaryFactory:label!=='raw-key',nativeFirstRepresentativeNotUnion:true,fullIndexStableSort:true,numericNativeRowIdNotUniqueTransportId:true,nullableDifference:false,legacyKeysDomainDifference:colorBy==='subGroup',callbackCountIsOnlyThisColdInvocation:true}});
}
function qualify(id,input,scope){result.qualifications.push({id,input,actual:run(input),scope});}
for(const colorBy of ['group','subGroup']){
 const nullableShape={id:'nullable-actual-number-null',groups,subGroups,data:[{id:'null-only',group:'B',subGroup:'y',v:null},{id:'mixed-null',group:'A',subGroup:'x',v:null},{id:'mixed-positive',group:'A',subGroup:'x',v:4},{id:'real-zero',group:'B',subGroup:'x',v:0}]};
 qualify(`${colorBy}-nullable-null-only-and-mixed`,inputFor(nullableShape,colorBy,'pair'),{nativeNumberNullIsZero:true,rustOmitsNoneFromStatistics:true,rustNoneOnlyHasNoComputedSummary:true,rawObservationIndicesIncludeNone:true,doNotClaimNullableParity:true});
 const hidden=inputFor(shapes[0],colorBy,'pair',{hiddenGroups:colorBy==='group'?['B']:[],hiddenSubGroups:colorBy==='subGroup'?['y']:[]});
 hidden.effectiveData=hidden.rawData.filter(d=>!hidden.hiddenGroups.includes(d.group)&&!hidden.hiddenSubGroups.includes(d.subGroup));
 qualify(`${colorBy}-hidden-upstream-filter-full-domain`,hidden,{nativeHasNoHiddenPropHere:true,explicitUpstreamFilter:true,fullConfiguredDomainStillPrewarmsPalette:true,rustHiddenRetainsSourceRatherThanDeletingIt:true});
 const emptyShape={id:'raw-empty-subgroup',groups:['reserved','Å'],subGroups:['missing',''],data:[{id:'empty-raw-real-zero',group:'Å',subGroup:'',v:0}]};
 qualify(`${colorBy}-raw-empty-subgroup`,inputFor(emptyShape,colorBy,'raw-key'),{nativeRawEmptyLabelAllowed:colorBy==='subGroup',byLabelRawKeyIsNotLegacySamplesCaption:true,realZeroSummaryEligible:true});
}
qualify('legend-false-callback-not-invoked',inputFor(shapes[0],'subGroup','pair',{legendEnabled:false}),{nativeLegendsEmptyArray:true,noNativeLegendCallbackCalls:true,rustLegendFalseMaySkipFactory:true});
qualify('original-real-lodash-nested-property-accessor',inputFor(shapes[0],'subGroup','pair',{label:{kind:'property',path:'values[2]'}}),{originalAccessorPathAndActualLodashGet:true,nativeNumericLabelAllowed:true,rustStringFactoryRequiresExplicitConversion:true,notClaimingArbitraryPropertyAccessorRustAPI:true});
assert.equal(result.profiles.length,32);assert.equal(result.qualifications.length,8);
const all=[...result.profiles,...result.qualifications];
for(const p of all){assert.deepEqual(p.actual.ordinalDomain,p.input.colorBy==='group'?p.input.groups:p.input.subGroups);for(const b of p.actual.boxes)assert.deepEqual(b.quantileProbabilities,quantileProbabilities);}
// These assertions assess actual original results; they never generate expected rows.
for(const colorBy of ['group','subGroup']){
 const pair=result.profiles.find(p=>p.id===`${colorBy}-complete-pair`).actual;
 assert.equal(pair.expectedLegend.length,4);assert.equal(new Set(pair.expectedLegend.map(r=>r.nativeId)).size,2);
 const constant=result.profiles.find(p=>p.id===`${colorBy}-complete-constant`).actual;
 assert.equal(constant.expectedLegend.length,1);assert.equal(constant.expectedLegend[0].representativeInputIndex,0);
 const empty=result.profiles.find(p=>p.id===`${colorBy}-complete-empty`).actual;
 assert.equal(empty.expectedLegend.length,1);assert.equal(empty.expectedLegend[0].label,'');
 const tiedIds=pair.expectedLegend.map(r=>r.nativeId);assert.deepEqual(tiedIds,[1,1,2,2]);
}
const values=JSON.stringify(result,null,2)+'\n';
const counts={primarySnapshotColdInvocations:40,deterministicRecheckInvocations:32,totalOriginalColdInvocations:72,profiles:result.profiles.length,qualifications:result.qualifications.length,actualComputedBoxes:all.reduce((n,p)=>n+p.actual.boxes.length,0),actualCallbackContexts:all.reduce((n,p)=>n+p.actual.callbackContexts.length,0),actualLegendRows:all.reduce((n,p)=>n+p.actual.expectedLegend.length,0),actualObservedLodashCalls:all.reduce((n,p)=>n+p.actual.originalLegendOperations.length,0)};
const metadata={schema:result.schema,nivoVersion:pins.nivoVersion,nivoCommit:pins.nivoCommit,lodashVersion:_.VERSION,producerSHA256:hash(fs.readFileSync(script)),valuesSHA256:hash(values),counts,countScope:'Computed boxes, callback contexts, legend rows and Lodash observations count the 40 archived primary snapshots; 32 full repeated hook evaluations additionally assert deterministic equality. This is not a React lifecycle callback budget.',sourcePins,runtimePins,
 executedOriginalFunctions:['useLevels','useBoxPlot','useBoxPlotTransition','stratifyData','summarizeDistribution','generateBoxPlots','getLegendData','getPropertyAccessor','usePropertyAccessor','useInheritedColor','useOrdinalColorScale','createLinearScale','createBandScale','computeScale (complete original dispatch function)'],
 adapters:{react:'Cold useMemo immediately evaluates original callbacks; useState is null and its setter throws. No React lifecycle, events, commits or memo reuse claim.',spring:'Whole original transition hook executes through a synchronous recording adapter, no physical animation.',property:'Whole original propertiesConverters body plus genuine Lodash4.17.21. Callback policies are declared input functions; numeric nested property qualification uses original accessor and actual Lodash.get.',legendObservation:'Original complete getLegendData body unchanged. Import wrappers delegate directly to actual Lodash.uniqBy/sortBy and record actual input/output object references. Representative identity comes from selected original mapped row index, never from a handwritten dedup/sort algorithm.',palette:'Released D3 ordinal array palette, whole original hook prewarms full configured domain. Alpha-zero original CSS raw RGB remains explicit; no shared resolver forced to erase Rust raw RGB.',scales:'Complete original linear/band functions and full original dispatcher with unsupported scale variants throwing; no time/log/symlog claim.',hidden:'Native hook has no hidden setting. Explicit input filter supplies effectiveData, with rawData and complete configured domain retained as distinct recorded inputs.'},
 limits:['No mounted React, widget, renderer, pixel, compositor, animation or callback lifecycle equivalence claim.','Native Number(null)=0 is recorded; Rust omits None. None-only native summaries must not be silently treated as Rust eligible summaries.','Source annotations observationIndices/representative/target are reference provenance; original native rows only contain id,label,color.','Native numeric row ID can repeat. Canonical Rust series encoding and transport row IDs are separately owned APIs, not invented by this oracle.','Original probabilities data.quantiles differ from computed statistics data.values.','Only String callbacks map to the Rust text-factory API; original nested property numeric labels are an explicit boundary.']};
for(const [name,content]of [['values.json',values],['reference.json',JSON.stringify(metadata,null,2)+'\n']]){const p=join(fixture,name);if(check)assert.equal(fs.readFileSync(p,'utf8'),content);else fs.writeFileSync(p,content);}
console.log(JSON.stringify({passed:true,checked:check,...counts,valuesSHA256:metadata.valuesSHA256}));

