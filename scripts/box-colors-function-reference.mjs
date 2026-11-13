// Whole original Nivo custom/datum color pipeline, inherited roles and ByLabel with real Lodash.
// node scripts/box-colors-function-reference.mjs <released-D3-node_modules> [--check]
import fs from 'node:fs';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {stripTypeScriptTypes} from 'node:module';
import {dirname,join,resolve,relative} from 'node:path';
import {fileURLToPath,pathToFileURL} from 'node:url';
const script=fileURLToPath(import.meta.url),root=resolve(dirname(script),'..'),args=process.argv.slice(2),check=args.includes('--check');
assert(args[0],'Supply released D3 node_modules root');
const modules=resolve(args[0]),fixture=join(root,'tests/fixtures/box-colors-function'),vendor=join(fixture,'vendor');
const sharedVendor=vendor;
const hash=b=>createHash('sha256').update(b).digest('hex'),sourcePins={},runtimePins={};
function readAt(base,name,key){const b=fs.readFileSync(join(base,name));sourcePins[key+'/'+name]=hash(b);return b.toString('utf8').replaceAll('\r\n','\n');}
const read=name=>readAt(sharedVendor,name,'tests/fixtures/box-colors-function/vendor');
const readOwn=name=>readAt(vendor,name,'tests/fixtures/box-colors-function/vendor');
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

const {getOrdinalColorScale}=await import(ordinal),{rgb}=await import(JSON.parse(entry('d3-color')));
const clone=x=>JSON.parse(JSON.stringify(x));
const q=[.1,.25,.5,.75,.9],groups=['reserved','B','A'],subGroups=['missing','y','x'];
const complete=[['B','y',0],['B','x',1],['A','y',2],['A','x',3]].flatMap(([group,subGroup,offset])=>[0,2,4,6,8,10].map((v,i)=>({id:`${group}/${subGroup}/${i}`,group,subGroup,v:v+offset,meta:{ink:'rgba(180,20,70,0.5)'}})));
const legend={anchor:'bottom',direction:'row',itemWidth:90,itemHeight:16};
const css=c=>`rgba(${c[0]},${c[1]},${c[2]},${c[3]/255})`;
function inspectColor(value){if(value===undefined)return{kind:'undefined'};if(typeof value!=='string')return{kind:typeof value,value};const parsed=rgb(value),finite=Number.isFinite(parsed.r)&&Number.isFinite(parsed.g)&&Number.isFinite(parsed.b),m=value.match(/^rgba?\(\s*([\d.]+),\s*([\d.]+),\s*([\d.]+)(?:,\s*([\d.]+))?\)$/);return{kind:'css',css:value,rawRGBA:m?[+m[1],+m[2],+m[3],Math.round((m[4]===undefined?1:+m[4])*255)]:null,d3:{r:Number.isFinite(parsed.r)?parsed.r:null,g:Number.isFinite(parsed.g)?parsed.g:null,b:Number.isFinite(parsed.b)?parsed.b:null,opacity:parsed.opacity},d3FiniteChannels:finite};}
function colorValue(descriptor,d){
 const fallback='rgb(201,202,203)';
 switch(descriptor.kind){
  case'constant':return descriptor.css;
  case'group':return css(d.group==='A'?[120,40,90,200]:d.group==='B'?[22,80,120,255]:[12,13,14,255]);
  case'subgroup':return css(d.subGroup==='x'?[50,140,80,210]:d.subGroup==='y'?[100,60,190,255]:[12,13,14,255]);
  case'mean':return d.mean===undefined?fallback:css([Math.round(d.mean*12),40,90,200]);
  case'values':return d.values===undefined?fallback:css([Math.round(d.values[2]*10),Math.round(d.values[4]*8),30,220]);
  case'cell':return d.groupIndex===undefined||d.subGroupIndex===undefined?fallback:css([20+d.groupIndex*40,30+d.subGroupIndex*50,90,200]);
  case'unguarded-values':return css([d.values[2],20,30,255]);
  case'undefined':return undefined;
  default:throw Error('Unknown declared color descriptor '+descriptor.kind);
 }
}
function inputFor(colorBy,layout,color,overrides={}){return{colorBy,layout,groups:clone(groups),subGroups:clone(subGroups),rawData:clone(complete),effectiveData:clone(complete),quantileProbabilities:q,color,label:{kind:['mean','cell'].includes(color.kind)?'constant':'pair'},legendEnabled:true,hiddenGroups:[],hiddenSubGroups:[],stroke:{border:defaults.borderColor,median:defaults.medianColor,whisker:defaults.whiskerColor},...overrides};}
function run(input,active=null){
 const colorCalls=[],labelCalls=[];
 const callback=d=>{const rec={datum:clone(d),stage:Object.hasOwn(d,'n')?'summary':'partial-domain',returned:null};colorCalls.push(rec);try{const value=colorValue(input.color,d);rec.returned=inspectColor(value);return value;}catch(e){rec.error={name:e.name,message:e.message};throw e;}};
 const colors=input.color.kind==='datum'?{datum:input.color.path}:callback;
 if(typeof colors==='function')assert.equal(getOrdinalColorScale(colors,input.colorBy),callback,'Original ordinal branch returns actual custom function unchanged');
 const labels=d=>{const value=input.label.kind==='constant'?'same caption':input.label.kind==='empty'?'':`${d.group} / ${d.subGroup}`;labelCalls.push({datum:clone(d),value});return value;};
 const options={data:input.effectiveData,value:'v',groupBy:'group',subGroupBy:'subGroup',groups:input.groups,subGroups:input.subGroups,quantiles:q,width:480,height:300,layout:input.layout,minValue:'auto',maxValue:'auto',valueScale:{type:'linear',nice:false},indexScale:{type:'band',round:false},padding:.1,innerPadding:4,colorBy:input.colorBy,colors,opacity:.8,activeOpacity:.9,inactiveOpacity:.2,borderColor:input.stroke.border,medianColor:input.stroke.median,whiskerColor:input.stroke.whisker,tooltipLabel:d=>d.group+'/'+d.subGroup,valueFormat:String,legendLabel:labels,legends:input.legendEnabled?[legend]:[]};
 adapters.prepare(active);let computed;
 try{computed=useBoxPlot(options);}catch(e){return{error:{name:e.name,message:e.message},colorCalls,primingCalls:colorCalls.filter(c=>c.stage==='partial-domain'),summaryColorCalls:colorCalls.filter(c=>c.stage==='summary'),legendCallbackContexts:labelCalls,completedBoxHook:false};}
 const ops=adapters.records().legendOps;
 if(input.legendEnabled){assert.equal(ops.length,2);assert.equal(ops[0].kind,'uniqBy');assert.equal(ops[1].kind,'sortBy');assert.equal(ops[0].input.length,computed.boxPlots.length);assert.equal(computed.legendsData[0][1],ops[1].output);}
 const identities=computed.boxPlots.map((b,summaryIndex)=>({summaryIndex,key:b.key,group:b.group,subGroup:b.subGroup,groupIndex:b.data.groupIndex,subGroupIndex:b.data.subGroupIndex,cellIndex:b.data.groupIndex*input.subGroups.length+b.data.subGroupIndex,colorIndex:input.colorBy==='group'?b.data.groupIndex:b.data.subGroupIndex,rawColorKey:b.data[input.colorBy]}));
 const observations=(d,data)=>data.flatMap((row,i)=>row.group===d.group&&row.subGroup===d.subGroup?[i]:[]);
 const boxes=computed.boxPlots.map((b,i)=>({...identities[i],sourceSummary:clone(b.data),observationIndices:observations(b.data,input.rawData),effectiveObservationIndices:observations(b.data,input.effectiveData),numericColor:inspectColor(b.color),opacity:b.opacity,position:{x:b.x,y:b.y,width:b.width,height:b.height},coordinates:clone(b.coordinates),borderColor:inspectColor(computed.getBorderColor(b)),medianColor:inspectColor(computed.getMedianColor(b)),whiskerColor:inspectColor(computed.getWhiskerColor(b))}));
 const priming=colorCalls.filter(c=>c.stage==='partial-domain'),actual=colorCalls.filter(c=>c.stage==='summary');
 if(typeof colors==='function'){const domain=input.colorBy==='group'?input.groups:input.subGroups;assert.deepEqual(priming.map(c=>c.datum),domain.map(v=>({[input.colorBy]:v})));assert.equal(actual.length,boxes.length);for(let i=0;i<boxes.length;i++)assert.deepEqual(actual[i].datum,boxes[i].sourceSummary);assert(colorCalls.slice(0,domain.length).every(c=>c.stage==='partial-domain'));}
 const expectedLegend=input.legendEnabled?computed.legendsData[0][1].map((row,rowIndex)=>{const index=ops[0].input.indexOf(row);assert(index>=0);const representative=identities[index];assert.equal(row.id,representative.colorIndex);assert.equal(row.color,computed.boxPlots[index].color);return{rowIndex,nativeId:row.id,fullColorIndex:row.id,label:row.label,color:inspectColor(row.color),representative:clone(representative),representativeInputIndex:index,target:{kind:input.colorBy==='group'?'Category':'Series',rawKey:representative.rawColorKey}};}):[];
 useBoxPlotTransition({...computed,animate:false,springConfig:{}});const transitions=adapters.records().transitions;assert.equal(transitions.length,1);
 return{completedBoxHook:true,error:null,colorCalls,primingCalls:priming,summaryColorCalls:actual,hasD3OrdinalScale:!!computed.getColor.scale,ordinalDomain:computed.getColor.scale?clone(computed.getColor.scale.domain()):null,boxes,legendCallbackContexts:labelCalls,expectedLegend,originalLegendOperations:ops.map(op=>({kind:op.kind,input:clone(op.input),output:clone(op.output),selectedInputIndices:op.output.map(row=>op.input.indexOf(row))})),transitions:clone(transitions[0]),activeInput:active,configuredQuantileProbabilities:q};
}
const functions=[{id:'constant',kind:'constant',css:'rgba(70,80,90,1)'},{id:'group',kind:'group'},{id:'subgroup',kind:'subgroup'},{id:'mean',kind:'mean'},{id:'values',kind:'values'},{id:'cell',kind:'cell'},{id:'alpha0',kind:'constant',css:'rgba(61,42,23,0)'},{id:'half',kind:'constant',css:'rgba(61,42,23,0.5)'}];
const result={schema:'box-colors-function-original-v1',profiles:[],qualifications:[]};
for(const colorBy of['group','subGroup'])for(const layout of['vertical','horizontal'])for(const color of functions){const input=inputFor(colorBy,layout,color),actual=run(input);assert(actual.completedBoxHook);assert.deepEqual(actual,run(input),'Complete cold original function deterministic');result.profiles.push({id:`${colorBy}-${layout}-${color.id}`,input,actual,rustScope:{typedSummaryColor:true,noNativePartialPrimingPromise:true,noReactLifecyclePromise:true,firstRepresentativeColorAndTarget:true,probabilitiesNotStatistics:true,alphaZeroTypedChannelsRetainedInRust:true,nativeHasNoOutlierMarks:true}});}
function qualify(id,input,scope){result.qualifications.push({id,input,actual:run(input),scope});}
for(const colorBy of['group','subGroup']){
 qualify(`${colorBy}-real-partial-summary-values-throws`,inputFor(colorBy,'vertical',{kind:'unguarded-values'}),{nativePartialInputThrowsBeforeAnySummary:true,rustTypedSummaryFactoryDoesNotInventPartialCallbacks:true});
 const hidden=inputFor(colorBy,'vertical',{kind:'cell'},{hiddenGroups:colorBy==='group'?['B']:[],hiddenSubGroups:colorBy==='subGroup'?['y']:[]});hidden.effectiveData=hidden.rawData.filter(d=>!hidden.hiddenGroups.includes(d.group)&&!hidden.hiddenSubGroups.includes(d.subGroup));qualify(`${colorBy}-upstream-hidden`,hidden,{explicitUpstreamFilterNotNativeHiddenProp:true,fullConfiguredPrimingStillIncludesHiddenIdentity:true,rustRetainsRawSourceAndHasHiddenOptions:true});
 qualify(`${colorBy}-nullable`,inputFor(colorBy,'vertical',{kind:'mean'},{rawData:[{id:'null',group:'B',subGroup:'y',v:null},{id:'positive',group:'B',subGroup:'y',v:4},{id:'null-only',group:'A',subGroup:'x',v:null}],effectiveData:[{id:'null',group:'B',subGroup:'y',v:null},{id:'positive',group:'B',subGroup:'y',v:4},{id:'null-only',group:'A',subGroup:'x',v:null}]}),{nativeNumberNullIsZero:true,rustOmitsNone:true,rawObservationIndicesRetainNone:true});
 qualify(`${colorBy}-empty-reserved`,inputFor(colorBy,'vertical',{kind:'mean'},{rawData:[],effectiveData:[]}),{nativePartialPrimingOnly:true,noActualSummaryOrLegend:true,rustNoSyntheticEmptyContext:true});
}
qualify('fixed-three-strokes',inputFor('subGroup','vertical',{kind:'cell'},{stroke:{border:'#112233',median:'rgba(44,55,66,0.5)',whisker:'#778899'}}),{fixedStrokeColorsIndependentOfCustomNumericColor:true,nativeOutlierAbsentRustFixedOutlierRequiresOwnRustRegression:true});
qualify('alpha-zero-modifier-restores-black',inputFor('subGroup','vertical',{kind:'constant',css:'rgba(61,42,23,0)'},{stroke:{border:{from:'color',modifiers:[['opacity',1]]},median:{from:'color'},whisker:{from:'color'}}}),{actualD3CSSAlphaZeroDiscardsChannels:true,rustTypedRGBAKeepsChannelsAndRestoresOriginalRGB:true,doNotModifySharedResolverToMatchLossyCSS:true});
qualify('datum-raw-extra-property-lost',inputFor('group','vertical',{kind:'datum',path:'meta.ink'}),{realLodashPath:true,rawExtraFieldsDiscardedByNativeOrdinarySummarization:true,undefinedIsNotTypedRustColor:true});
const prec=[{id:'precomputed',group:'B',subGroup:'y',n:2,extrema:[1,9],mean:5,quantiles:q,values:[1,3,5,7,9],meta:{ink:'rgba(99,77,55,0.5)'}}];qualify('datum-precomputed-spread-keeps-extra-property',inputFor('group','vertical',{kind:'datum',path:'meta.ink'},{rawData:prec,effectiveData:prec}),{nativePrecomputedRuntimeSpreadKeepsExtraFields:true,partialPrimingReturnsUndefinedWithoutThrow:true,rustBoxObservationNoArbitraryExtraPropertyModel:true});
qualify('datum-summary-mean-is-number-not-css',inputFor('group','vertical',{kind:'datum',path:'mean'}),{nativeAccessorReturnsNumber:true,notValidTypedRustRGBA:true});
qualify('invalid-css-original-not-prevalidated',inputFor('subGroup','vertical',{kind:'constant',css:'not-a-css-color'}),{nativeReturnsInvalidCSSUnchanged:true,notValidTypedRustRGBA:true});
qualify('legend-disabled-still-computes-color',inputFor('group','vertical',{kind:'cell'},{legendEnabled:false}),{nativeColorIndependentOfLegendEnable:true,rustColorFactoryMustStillComputeActualSource:true});
const coldInput=inputFor('subGroup','vertical',{kind:'cell'}),base=run(coldInput),active={group:base.boxes[0].group,subGroup:base.boxes[0].subGroup};result.qualifications.push({id:'explicit-cold-active-reexecution',input:coldInput,actual:{before:base,after:run(coldInput,active)},scope:{twoSeparateColdOriginalInvocations:true,notMountedReactOrActualHoverLifecycle:true,nativeActualMapMayReevaluateOnActiveDependencies:true,rustHoverUsesAcceptedCachedColors:true}});
assert.equal(result.profiles.length,32);assert.equal(result.qualifications.length,16);
const snapshots=[...result.profiles.map(p=>p.actual),...result.qualifications.flatMap(p=>p.actual.before?[p.actual.before,p.actual.after]:[p.actual])];
for(const p of result.qualifications.filter(p=>p.id.includes('real-partial'))){assert(!p.actual.completedBoxHook);assert.equal(p.actual.summaryColorCalls.length,0);assert.equal(p.actual.primingCalls.length,1);assert.equal(p.actual.error.name,'TypeError');}
const lost=result.qualifications.find(p=>p.id==='datum-raw-extra-property-lost');assert(lost.actual.boxes.every(b=>b.numericColor.kind==='undefined'));const retained=result.qualifications.find(p=>p.id==='datum-precomputed-spread-keeps-extra-property');assert.equal(retained.actual.boxes[0].numericColor.css,'rgba(99,77,55,0.5)');
const values=JSON.stringify(result,null,2)+'\n',counts={profiles:32,qualifications:16,primarySnapshots:snapshots.length,deterministicRepeatedMainInvocations:32,totalColdHookInvocations:snapshots.length+32,completedSnapshots:snapshots.filter(s=>s.completedBoxHook).length,partialThrows:snapshots.filter(s=>!s.completedBoxHook).length,computedBoxes:snapshots.reduce((n,s)=>n+(s.boxes?.length||0),0),primingCalls:snapshots.reduce((n,s)=>n+s.primingCalls.length,0),actualSummaryColorCalls:snapshots.reduce((n,s)=>n+s.summaryColorCalls.length,0),actualLegendRows:snapshots.reduce((n,s)=>n+(s.expectedLegend?.length||0),0)};
const metadata={schema:result.schema,nivoVersion:pins.nivoVersion,nivoCommit:pins.nivoCommit,lodashVersion:_.VERSION,producerSHA256:hash(fs.readFileSync(script)),valuesSHA256:hash(values),counts,sourcePins,runtimePins,executedOriginalFunctions:['getOrdinalColorScale','useOrdinalColorScale','useBoxPlot','useBoxPlotTransition','stratifyData','summarizeDistribution','generateBoxPlots','getInheritedColorGenerator','getLegendData','getPropertyAccessor','usePropertyAccessor','actual Lodash.get/uniqBy/sortBy','complete linear/band original scale and dispatch functions'],adapters:{react:'Cold useMemo immediately evaluates callbacks; state is explicit null or supplied cold active object, not a mounted React lifecycle.',color:'Declared JSON descriptor creates a real JS colors function; complete original ordinal resolver returns it unchanged. Original hook supplies its own partial priming and summary objects; no adapter invents callback inputs.',datum:'Whole original ordinal {datum:path} branch with actual Lodash.get; ordinary summary discards arbitrary raw fields, original precomputed spread can retain them.',legend:'Actual original complete helper delegates to real Lodash. Row provenance comes from retained original object identity, not handwritten dedup/sort.',transition:'Whole original useBoxPlotTransition executes through recording adapter; component JSX renderer is source-pinned only, no pixel/ReactSpring claim.'},limits:['Typed Rust RGBA rejects undefined, numbers and invalid CSS rather than copying native permissive outputs.','Alpha0 CSS D3 modifier parsing loses hidden RGB; Rust typed color preserves it. Half CSS opacity is recorded exactly, with optional 8-bitRGBA rounding explicit.','Rust per-summary factory has no native partial-domain priming callbacks, and hover uses cached colors. No callback lifecycle/count parity claimed.','Native BoxPlot component renders no outlier marks. Rust default/fixed outlier color behavior needs a Rust regression, not a nonexistent native outlier oracle.','No arbitrary datum-property accessor API, raw-field preservation, renderer, widget, browser or GPU acceptance inferred.']};
for(const[name,content]of[['values.json',values],['reference.json',JSON.stringify(metadata,null,2)+'\n']]){const p=join(fixture,name);if(check)assert.equal(fs.readFileSync(p,'utf8'),content);else fs.writeFileSync(p,content);}console.log(JSON.stringify({passed:true,checked:check,...counts,valuesSHA256:metadata.valuesSHA256}));

