// Execute original Nivo 0.99 BoxPlot hooks + getLegendData with explicit cold adapters.
// node scripts/box-colorby-hover-reference.mjs <released-scale-node_modules> [--check]
import fs from 'node:fs';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {stripTypeScriptTypes} from 'node:module';
import {dirname,join,resolve} from 'node:path';
import {fileURLToPath,pathToFileURL} from 'node:url';
const root=resolve(dirname(fileURLToPath(import.meta.url)),'..'),args=process.argv.slice(2),check=args.includes('--check');
assert(args[0],'Supply existing released D3 node_modules root');
const modules=resolve(args[0]),fixture=join(root,'tests/fixtures/box-colorby-hover'),vendor=join(fixture,'vendor');
const hash=x=>createHash('sha256').update(x).digest('hex'),sourcePins={},runtimePins={};
const read=name=>{const b=fs.readFileSync(join(vendor,name));sourcePins['tests/fixtures/box-colorby-hover/vendor/'+name]=hash(b);return b.toString('utf8').replaceAll('\r\n','\n');};
const pins=JSON.parse(read('source-pins.json'));
for(const [name,pin]of Object.entries(pins.files))assert.equal(hash(fs.readFileSync(join(vendor,name))),pin.sha256);
for(const name of ['nivo-LICENSE','d3-scale-LICENSE','d3-array-LICENSE','d3-interpolate-LICENSE','d3-color-LICENSE','internmap-LICENSE'])read(name);
const versions={'d3-scale':'4.0.2','d3-interpolate':'3.0.1','d3-array':'3.2.5','d3-color':'3.1.0'};
function entry(name){const pkg=JSON.parse(fs.readFileSync(join(modules,name,'package.json')));assert.equal(pkg.version,versions[name]);for(const f of ['package.json',pkg.module])runtimePins[name+'/'+f]=hash(fs.readFileSync(join(modules,name,f)));return JSON.stringify(pathToFileURL(join(modules,name,pkg.module)).href);}
// Pin each declared released dependency, including transitive d3-array.
for(const name of Object.keys(versions))entry(name);
const stripImports=s=>s.replace(/^import[\s\S]*?from\s*['"][^'"]+['"]\n/gm,'');
const types=s=>'data:text/javascript;base64,'+Buffer.from(stripTypeScriptTypes(s,{mode:'transform'})).toString('base64');
const adapter=types(`
let state,memos=[],transitions=[];
export const useMemo=(fn,deps)=>{const value=fn();memos.push({value,deps});return value;};
export const useState=value=>[state===undefined?value:state,()=>{throw Error('Cold adapter does not perform React state commits');}];
export const prepare=value=>{state=value;memos=[];transitions=[];};
export const records=()=>({memos,transitions});
export const useTheme=()=>({labels:{text:{fill:'rgb(10,20,30)'}}});
export const get=(object,path)=>{if(typeof path!=='string'||!/^\\w+(?:\\.\\w+)*$/.test(path))throw Error('Only ordinary dotted property paths in adapter');return path.split('.').reduce((v,k)=>v==null?undefined:v[k],object);};
export const usePropertyAccessor=p=>typeof p==='function'?p:d=>get(d,p),getPropertyAccessor=usePropertyAccessor;
export const useValueFormatter=f=>typeof f==='function'?f:String;
export const isPlainObject=v=>v!==null&&typeof v==='object'&&Object.getPrototypeOf(v)===Object.prototype;
export const unsupported=()=>{throw Error('Adapter branch outside explicit linear/band/array-palette scope');};
export const uniqBy=(array,key)=>{const seen=new Set();return array.filter(item=>{const k=key(item);if(seen.has(k))return false;seen.add(k);return true;});};
export const sortBy=(array,key)=>[...array].sort((a,b)=>key(a)-key(b));
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
const legends=ts('box-legends.ts',`import{getPropertyAccessor,uniqBy,sortBy}from ${JSON.stringify(adapter)};`);
const hooks=ts('box-hooks.ts',`import{useMemo,useState,usePropertyAccessor,useValueFormatter,useTheme,useTransition}from ${JSON.stringify(adapter)};import{useInheritedColor}from ${JSON.stringify(inherited)};import{useOrdinalColorScale}from ${JSON.stringify(ordinal)};import{defaultProps}from ${JSON.stringify(props)};import{stratifyData,summarizeDistribution}from ${JSON.stringify(stratification)};import{generateBoxPlots}from ${JSON.stringify(generation)};import{getLegendData}from ${JSON.stringify(legends)};`);
const {useBoxPlot,useBoxPlotTransition}=await import(hooks),{getLegendData}=await import(legends);
const rgba=c=>`rgba(${c[0]},${c[1]},${c[2]},${c[3]/255})`,paletteRGBA=[[20,40,60,160],[200,180,160,220],[9,8,7,0],[40,120,180,255]],palette=paletteRGBA.map(rgba);
function bytes(css){assert.equal(typeof css,'string');const m=css.match(/^rgba?\(\s*([\d.]+),\s*([\d.]+),\s*([\d.]+)(?:,\s*([\d.]+))?\)$/);assert(m,css);return [+m[1],+m[2],+m[3],Math.round((m[4]===undefined?1:+m[4])*255)];}
const clone=x=>JSON.parse(JSON.stringify(x));
const shape=(id,groups,subGroups,cells)=>({id,groups,subGroups,data:cells.flatMap(([group,subGroup,values])=>values.map((v,i)=>({id:`${group.length}:${group}/${subGroup.length}:${subGroup}/${i}`,group,subGroup,v})))});
const shapes=[
shape('complete',['reserved','B','A'],['missing','y','x'],[['B','y',[1,3]],['B','x',[2,4]],['A','y',[3,5]],['A','x',[4,6]]]),
shape('partial',['reserved','B','A'],['missing','y','x'],[['B','y',[1,3]],['A','y',[5,5]],['A','x',[2,4]]]),
shape('unicode-collision',['reserved','a|b','a','北/南😀'],['missing','c','b|c','同名'],[['a|b','c',[1,3]],['a','b|c',[2,4]],['北/南😀','同名',[5,5]]]),
shape('nullable-zero-constant',['reserved','B','A'],['missing','y','x'],[['B','y',[null,2,4]],['B','x',[0,0]],['A','missing',[null]],['A','y',[1,3]],['A','x',[5,5]]]),
];
const legendProps=[{anchor:'bottom',direction:'row',itemWidth:90,itemHeight:16},{anchor:'right',direction:'column',itemWidth:100,itemHeight:18}];
function options(input){return {data:input.data,value:'v',groupBy:'group',subGroupBy:'subGroup',groups:input.groups,subGroups:input.subGroups,width:480,height:300,layout:'vertical',minValue:'auto',maxValue:'auto',valueScale:{type:'linear',nice:false},indexScale:{type:'band',round:false},padding:.1,innerPadding:4,colorBy:input.colorBy,colors:palette,opacity:.8,activeOpacity:.9,inactiveOpacity:.2,borderColor:defaults.borderColor,medianColor:defaults.medianColor,whiskerColor:defaults.whiskerColor,tooltipLabel:d=>d.group+'/'+d.subGroup,valueFormat:String,legends:legendProps,...(input.legendLabel==='group-property'?{legendLabel:'group'}:input.legendLabel==='callback-pair'?{legendLabel:d=>d.group+' / '+d.subGroup}:{})};}
function run(input,activeItem=null){
 adapters.prepare(activeItem);const opts=options(input),computed=useBoxPlot(opts);
 const activeMemos=adapters.records().memos.filter(r=>Array.isArray(r.deps)&&r.deps.length===2&&r.deps[1]===computed.activeItem&&Array.isArray(r.deps[0])&&r.deps[0].every(x=>x&&typeof x==='object'&&'key'in x)&&Array.isArray(r.value)&&r.value.every(x=>typeof x==='string'));
 assert.equal(activeMemos.length,1,'Unique actual original activeKeys useMemo result, no algorithm reconstruction');
 const activeKeys=clone(activeMemos[0].value);
 const direct=getLegendData({boxPlots:computed.boxPlots,dataFrom:input.colorBy,legendLabel:opts.legendLabel});
 for(const [actualProps,actualData]of computed.legendsData){assert.deepEqual(actualData,direct);assert(legendProps.some(p=>p===actualProps));}
 useBoxPlotTransition({...computed,animate:false,springConfig:{}});assert.equal(adapters.records().transitions.length,1);
 const snapshots={sourceActiveItem:clone(computed.activeItem),activeKeys,activeMemoEvidence:{uniqueDependencyMatch:true,unStyledKeys:activeMemos[0].deps[0].map(x=>x.key)},ordinalDomain:computed.getColor.scale.domain(),boxes:computed.boxPlots.map((b,i)=>({key:b.key,group:b.group,subGroup:b.subGroup,sourceSummary:clone(b.data),numericColor:b.color,numericRGBA:bytes(b.color),opacity:b.opacity,position:{x:b.x,y:b.y,width:b.width,height:b.height},transition:clone(adapters.records().transitions[0][i]),rustNullableSourceComparable:input.data.filter(d=>d.group===b.group&&d.subGroup===b.subGroup).every(d=>d.v!==null)})),legendsData:computed.legendsData.map(([props,data])=>({legend:clone(props),data:data.map(x=>({...x,rgba:bytes(x.color),sourceKey:(input.colorBy==='group'?input.groups:input.subGroups)[x.id]}))}))};
 return {computed,snapshot:snapshots};
}
const result={schema:'box-colorby-hover-original-v1',profiles:[],qualifications:[]};
for(const colorBy of ['group','subGroup'])for(const reverse of [false,true])for(const dataShape of shapes)for(const legendLabel of ['default','group-property']){
 const input={shape:dataShape.id,colorBy,groups:reverse?[...dataShape.groups].reverse():dataShape.groups,subGroups:reverse?[...dataShape.subGroups].reverse():dataShape.subGroups,data:clone(dataShape.data),paletteRGBA,palette,legendLabel,opacity:.8,activeOpacity:.9,inactiveOpacity:.2};
 const baseline=run(input),sourceActiveItem=baseline.computed.boxPlots[0];assert(sourceActiveItem);
 const active=run(input,sourceActiveItem),restored=run(input);assert.deepEqual(restored.snapshot,baseline.snapshot);
 result.profiles.push({id:`${colorBy}-${reverse?'reversed':'authored'}-${dataShape.id}-${legendLabel}`,input,baseline:baseline.snapshot,active:active.snapshot,restorationExact:true,rustScope:{numericPaletteAndGroupMode:true,legendLabelSupported:legendLabel==='default',nativeSubgroupLegendOnlyActualVersusLegacyRustFullDomain:colorBy==='subGroup',nullableStatisticsComparable:!dataShape.data.some(d=>d.v===null),activeItemRequiresRustExistingTypedKey:true}});
}
const baseInput=clone(result.profiles.find(c=>c.id==='group-authored-complete-default').input),base=run(baseInput),activeSource=base.computed.boxPlots[0];
function qualify(id,input,activeItem=null,scope=''){const out=run(input,activeItem);result.qualifications.push({id,input:clone(input),sourceActiveItem:clone(activeItem),actual:out.snapshot,scope});return out;}
qualify('empty-complete-domain',{...baseInput,data:[]});
qualify('upstream-filter-group-B',{...baseInput,data:baseInput.data.filter(d=>d.group!=='B')},activeSource,'Actual original full groups remain prewarmed; no Nivo hidden prop is fabricated. Stale source active may still match another subgroup.');
qualify('upstream-filter-subgroup-y',{...baseInput,data:baseInput.data.filter(d=>d.subGroup!=='y')},activeSource,'Actual original subgroup filter retains prewarmed domains; active group still links existing boxes.');
qualify('stale-key-same-identities',baseInput,{...clone(activeSource),key:'not-an-original-key'},'Native ignores the numeric key field when group/subGroup stay unchanged. Rust BoxPlotKey exposes those identities only; no overrideable native numeric key field is claimed.');
qualify('stale-group-existing-subgroup',baseInput,{...clone(activeSource),group:'absent-group'},'Native partially matches subgroup with foreign group. Rust typed existing BoxPlotKey rejects.');
qualify('stale-both-identities',baseInput,{...clone(activeSource),group:'absent-group',subGroup:'absent-subgroup'},'Native accepts active object but no activeKeys; every existing box gets inactiveOpacity.');
qualify('malformed-empty-active',baseInput,{},'Native JavaScript accepts empty active object and dims every box; not a valid Rust typed source target.');
qualify('explicit-callback-legend-pair',{...baseInput,legendLabel:'callback-pair'},null,'Only this explicit callback is exercised; no arbitrary lodash path/React UI accessor coverage.');
const n=shapes.find(x=>x.id==='nullable-zero-constant');qualify('native-null-number-zero',{...baseInput,...clone(n),shape:n.id},null,'Original Number(null)=0, including null-only box; Rust None omits observations/empty cells.');
const sameName=shape('same-name',['reserved','same','other'],['reserved','same','other'],[['same','other',[1,3]],['other','same',[2,4]],['other','other',[3,5]]]);
qualify('same-name-group-and-subgroup',{...baseInput,...sameName,shape:sameName.id},null,'Group/subgroup namespaces stay independent even with identical strings.');
assert.equal(result.profiles.length,32);assert.equal(result.qualifications.length,10);
// Independent consistency checks only; all expected keys/opacity/legend values above
// are actual original hook/memo results, never produced by these assertions.
for(const p of result.profiles){
 const domain=p.input.colorBy==='group'?p.input.groups:p.input.subGroups;
 assert.deepEqual(p.baseline.ordinalDomain,domain);assert.deepEqual(p.active.ordinalDomain,domain);
 assert.equal(p.baseline.legendsData.length,2);assert.deepEqual(p.baseline.legendsData[0].data,p.baseline.legendsData[1].data);
 assert(p.baseline.boxes.every(b=>b.opacity===.8));assert(p.active.boxes.some(b=>b.opacity===.9));
 const counterpart=result.profiles.find(x=>x.input.colorBy!==p.input.colorBy&&x.input.shape===p.input.shape&&x.input.legendLabel===p.input.legendLabel&&JSON.stringify(x.input.groups)===JSON.stringify(p.input.groups));
 assert.deepEqual(p.active.activeKeys,counterpart.active.activeKeys,'Original OR hover is colorBy-independent');
}
const values=JSON.stringify(result,null,2)+'\n';
const metadata={schema:result.schema,nivoVersion:pins.nivoVersion,nivoCommit:pins.nivoCommit,counts:{profiles:result.profiles.length,baselineAndActiveSnapshots:result.profiles.length*2,qualificationSnapshots:result.qualifications.length,actualComputedBoxRecords:result.profiles.reduce((n,c)=>n+c.baseline.boxes.length+c.active.boxes.length,0)+result.qualifications.reduce((n,c)=>n+c.actual.boxes.length,0)},valuesSHA256:hash(values),sourcePins,runtimePins,adapters:{react:'Cold useMemo executes original complete bodies and records retval/deps; useState supplies previous actual computed item. No state commits, mounted React lifecycle or event dispatch.',lodash:'Ordinary dot-word accessor and function, stable primitive-key uniqBy + numeric sortBy only; no general lodash deep path API.',scales:'Actual released D3 linear/band and array ordinal; original whole computeScale dispatch retained, other scale branches explicit unsupported.',babel:'Not used: complete original TypeScript hooks/helpers/transitions only, no TSX or original legend component/pixels claimed.'},scope:'Actual whole original useLevels/useBoxPlot/getLegendData/useBoxPlotTransition and original color resolver/generation/stratification; native full-domain prewarming is executed by original hooks103-109. All source/computed color, legend numeric IDs/labels and activeKeys/opacity from original functions. Rust None/legend domain/accessor/typed stale-key boundaries explicit.'};
for(const [name,content]of [['values.json',values],['reference.json',JSON.stringify(metadata,null,2)+'\n']]){const p=join(fixture,name);if(check)assert.equal(fs.readFileSync(p,'utf8'),content);else fs.writeFileSync(p,content);}
console.log(JSON.stringify({passed:true,checked:check,...metadata.counts,valuesSHA256:metadata.valuesSHA256}));
