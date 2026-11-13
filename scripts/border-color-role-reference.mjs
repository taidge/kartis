// Execute original Nivo inherited-color, Bar and HeatMap bodies with released D3.
// node scripts/border-color-role-reference.mjs <scale-node_modules> <shape-node_modules> [--reference-root <project>] [--check]
import fs from 'node:fs';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {stripTypeScriptTypes} from 'node:module';
import {dirname,resolve,join} from 'node:path';
import {fileURLToPath,pathToFileURL} from 'node:url';
const root=resolve(dirname(fileURLToPath(import.meta.url)),'..'),args=process.argv.slice(2),check=args.includes('--check'),ri=args.indexOf('--reference-root'),reference=ri<0?root:resolve(args[ri+1]);
assert(args[0]&&args[1],'Supply existing released scale/color and shape module directories');
const modules={scale:resolve(args[0]),shape:resolve(args[1])},fixture=join(root,'tests/fixtures/border-color-role'),vendor=join(fixture,'vendor'),hash=s=>createHash('sha256').update(s).digest('hex');
const pins={};
const readFile=(path)=>{const bytes=fs.readFileSync(path);pins[path.startsWith(root)?path.slice(root.length+1).replaceAll('\\','/'):path.slice(reference.length+1).replaceAll('\\','/')]=hash(bytes);return bytes.toString('utf8').replaceAll('\r\n','\n');};
const vendorPins=JSON.parse(readFile(join(vendor,'source-pins.json')));for(const [name,p]of Object.entries(vendorPins.files))assert.equal(hash(fs.readFileSync(join(vendor,name))),p.sha256);
const shared=join(reference,'tests/fixtures/bar-label-color'),barVendor=join(reference,'tests/fixtures/bar-index-legend/vendor');
const sharedManifest=JSON.parse(readFile(join(shared,'source-and-fixtures.json')));for(const f of sharedManifest.files)assert.equal(hash(fs.readFileSync(join(shared,f.path))),f.sha256);
const barManifest=JSON.parse(readFile(join(reference,'tests/fixtures/bar-index-legend/source-pins.json')));for(const [name,p]of Object.entries(barManifest.files))assert.equal(hash(fs.readFileSync(join(barVendor,name))),p.sha256);
const versions={'d3-scale':['scale','4.0.2'],'d3-interpolate':['scale','3.0.1'],'d3-array':['scale','3.2.5'],'d3-color':['scale','3.1.0'],'d3-shape':['shape','3.2.0']};
for(const [name,[location,version]]of Object.entries(versions))assert.equal(JSON.parse(fs.readFileSync(join(modules[location],name,'package.json'))).version,version);
const moduleEntry=name=>JSON.parse(fs.readFileSync(join(modules[versions[name][0]],name,'package.json'))).module;
const runtimePins={};for(const [name,[location]]of Object.entries(versions))for(const file of ['package.json',moduleEntry(name)])runtimePins[name+'/'+file]=hash(fs.readFileSync(join(modules[location],name,file)));
for(const [name,file]of [['d3-color','src/color.js'],['d3-scale','src/band.js'],['d3-scale','src/linear.js'],['d3-interpolate','src/number.js'],['d3-shape','src/stack.js']])runtimePins[name+'/'+file]=hash(fs.readFileSync(join(modules[versions[name][0]],name,file)));
const external=(name)=>JSON.stringify(pathToFileURL(join(modules[versions[name][0]],name,moduleEntry(name))).href);
const noImports=s=>s.replace(/^import[\s\S]*?from ['"][^'"]+['"]\n/gm,'');
const module=s=>'data:text/javascript;base64,'+Buffer.from(stripTypeScriptTypes(s,{mode:'transform'})).toString('base64');
const adapters=module(`export const useMemo=f=>f();export const useCallback=f=>f;export const useState=v=>[v,()=>{throw Error('React state mutation outside snapshot scope');}];
export const get=(value,path)=>{if(typeof path!=='string'||!/^\\w+(?:\\.\\w+)*$/.test(path))throw Error('Unimplemented property adapter path');return path.split('.').reduce((v,k)=>v==null?undefined:v[k],value);};
export const isPlainObject=v=>v!==null&&typeof v==='object'&&Object.getPrototypeOf(v)===Object.prototype;
export const usePropertyAccessor=key=>typeof key==='function'?key:d=>get(d,key);
export const useValueFormatter=()=>String;let currentTheme;export const setTheme=t=>{currentTheme=t;};export const useTheme=()=>currentTheme;
export const useOrdinalColorScale=colors=>{if(typeof colors!=='function')throw Error('Only explicit function palette adapter');return colors;};`);
const {setTheme}=await import(adapters);
const inherited=module(`import {useMemo,get,isPlainObject} from ${JSON.stringify(adapters)};import {rgb} from ${external('d3-color')};\n`+noImports(readFile(join(shared,'nivo-inheritedColor.ts'))));
const {getInheritedColorGenerator,useInheritedColor}=await import(inherited);
const linear=module(`import {scaleLinear} from ${external('d3-scale')};import {interpolateNumber,interpolateRound} from ${external('d3-interpolate')};\n`+noImports(readFile(join(barVendor,'linearScale.ts'))));
const band=module(`import {scaleBand} from ${external('d3-scale')};\n`+noImports(readFile(join(barVendor,'bandScale.ts'))));
const bandExports=await import(band);
const dispatch=readFile(join(barVendor,'compute.ts')).match(/^export function computeScale[\s\S]*?^\}/m)?.[0];assert(dispatch);
const scales=module(`import {createLinearScale} from ${JSON.stringify(linear)};import {createBandScale} from ${JSON.stringify(band)};\n${dispatch}`);
const common=module(`import {computeScale} from ${JSON.stringify(scales)};const commonDefaultProps={};\n`+noImports(readFile(join(barVendor,'common.ts'))));
const prefix=`import {computeScale} from ${JSON.stringify(scales)};import {coerceValue,filterNullValues,getIndexScale,normalizeData} from ${JSON.stringify(common)};\n`;
const grouped=module(prefix+noImports(readFile(join(barVendor,'grouped.ts'))));
const stacked=module(prefix+`import {stack,stackOffsetDiverging} from ${external('d3-shape')};\n`+noImports(readFile(join(barVendor,'stacked.ts'))));
const totals=module('const commonDefaultProps={};\n'+noImports(readFile(join(vendor,'nivo-bar-totals.ts'))));
const legends=module(`import {usePropertyAccessor as getPropertyAccessor} from ${JSON.stringify(adapters)};const uniqBy=(values,key)=>{const seen=new Set();return values.filter(v=>{const k=key(v);if(seen.has(k))return false;seen.add(k);return true;});};\n`+noImports(readFile(join(barVendor,'legends.ts'))));
const extractDefault=(text)=>{const line=text.match(/^\s*borderColor:\s*(.+),$/m);assert(line);return stripTypeScriptTypes('export const value='+line[1]+';',{mode:'transform'});};
const barDefault=(await import(module(extractDefault(readFile(join(vendor,'nivo-bar-defaults.ts')))))).value;
const heatDefault=(await import(module(extractDefault(readFile(join(vendor,'nivo-heatmap-defaults.ts')))))).value;
const barHook=module(`import {useMemo,useCallback,useState,useTheme,usePropertyAccessor,useValueFormatter,useOrdinalColorScale}from ${JSON.stringify(adapters)};import {useInheritedColor}from ${JSON.stringify(inherited)};import {generateGroupedBars}from ${JSON.stringify(grouped)};import {generateStackedBars}from ${JSON.stringify(stacked)};import {getLegendData}from ${JSON.stringify(legends)};import {computeBarTotals}from ${JSON.stringify(totals)};const commonDefaultProps={borderColor:${JSON.stringify(barDefault)}};\n`+noImports(readFile(join(shared,'nivo-bar-hooks.ts'))));
const {useBar}=await import(barHook);
const heatCompute=module(`import {scaleBand,scaleLinear}from ${external('d3-scale')};import {castBandScale}from ${JSON.stringify(band)};\n`+noImports(readFile(join(vendor,'nivo-heatmap-compute.ts'))));
const heatHook=module(`import {useMemo,useCallback,useState,useTheme,usePropertyAccessor,useValueFormatter}from ${JSON.stringify(adapters)};import {useInheritedColor}from ${JSON.stringify(inherited)};import {computeCells,computeSizeScale,getCellAnnotationPosition,getCellAnnotationDimensions}from ${JSON.stringify(heatCompute)};const commonDefaultProps={};const getContinuousColorScale=()=>{throw Error('Only explicit function palette scope');};const useAnnotations=()=>{throw Error('Annotations outside scope');};\n`+noImports(readFile(join(vendor,'nivo-heatmap-hooks.ts')))+'\nexport {useCellsStyle};');
const {computeCells}=await import(heatCompute),{useCellsStyle}=await import(heatHook);
const rounded=module(noImports(readFile(join(vendor,'nivo-roundedRect.ts'))));
const noLabels=`const setCanvasFont=()=>{throw Error('labels disabled in recording scope');};const drawCanvasText=setCanvasFont;\n`;
const {renderBar}=await import(module(`import {roundedRect}from ${JSON.stringify(rounded)};\n`+noLabels+noImports(readFile(join(vendor,'nivo-renderBar.ts')))));
const {renderRect,renderCircle}=await import(module(noLabels+noImports(readFile(join(vendor,'nivo-heatmap-canvas.tsx')))));
const rgba=c=>`rgba(${c[0]},${c[1]},${c[2]},${c[3]/255})`;
const bytesFromReturnedCSS=s=>{const m=String(s).match(/^rgba?\(\s*([\d.]+),\s*([\d.]+),\s*([\d.]+)(?:,\s*([\d.]+))?\)$/);assert(m,'Expected actual numeric rgb/rgba output, not a second CSS parse');return [+m[1],+m[2],+m[3],Math.round((m[4]===undefined?1:+m[4])*255)];};
const themeRGBA=[70,80,90,123],fixedRGBA=[12,34,56,78];setTheme({border:{stroke:rgba(themeRGBA)},labels:{text:{fill:rgba(themeRGBA)}}});
const profiles=[['native-default',undefined],['inherit',{from:'color'}],['darker',{from:'color',modifiers:[['darker',1]]}],['ordered',{from:'color',modifiers:[['brighter',1],['opacity',.5],['darker',1]]}],['fixed',rgba(fixedRGBA)],['theme',{theme:'border.stroke'}]];
const values={sharedResolver:[],bar:[],heatmap:[],nativeErrors:[],propertyAndFunction:[]};
const old=JSON.parse(readFile(join(shared,'values.json')));
for(const [i,v]of old.entries()){
 const config={from:'color',modifiers:v.modifiers},generated=getInheritedColorGenerator(config)({color:v.cssSource}),bytes=bytesFromReturnedCSS(generated);assert.deepEqual(bytes,v.nativeNivoCSS);
 values.sharedResolver.push({case:i,input:v.input,modifiers:v.modifiers,actualNativeReturned:generated,actualNativeRGBA:bytes,typedD3ConstructorRGBA:v.nativeTypedRGBA,scope:v.profile,nativeResolverParsesSourceCSS:v.modifiers.length>0});
}
const palette={color:[20,40,60,255],b:[100,120,140,255]},source=[{index:'one',color:2,b:3,roleCss:rgba([31,41,51,61])},{index:'zero',color:0,b:null,roleCss:rgba([32,42,52,62])},{index:'null',color:null,b:null,roleCss:rgba([33,43,53,63])}];
function recorder(){const calls=[],state={globalAlpha:1},stack=[];return{calls,ctx:new Proxy(state,{set(o,k,v){o[k]=v;calls.push({op:'set',name:k,value:v});return true;},get(o,k){if(k in o)return o[k];if(k==='save')return()=>{stack.push({...o});calls.push({op:'save'});};if(k==='restore')return()=>{const p=stack.pop();for(const key of Object.keys(o))delete o[key];Object.assign(o,p);calls.push({op:'restore'});};return(...args)=>calls.push({op:String(k),args,alpha:o.globalAlpha,stroke:o.strokeStyle});}})};}
for(const groupMode of ['grouped','stacked'])for(const layout of ['vertical','horizontal'])for(const [profile,borderColor]of profiles){
 const props={data:source,indexBy:'index',keys:['color','b'],label:'formattedValue',tooltipLabel:d=>String(d.id),colors:d=>rgba(palette[d.id]),colorBy:'id',labelTextColor:{theme:'labels.text.fill'},groupMode,layout,margin:{left:20,top:20,right:20,bottom:20},width:240,height:160,padding:.2,innerPadding:0,valueScale:{type:'linear',min:0,max:6,nice:false},indexScale:{type:'band',round:false},initialHiddenIds:[],enableLabel:false,labelSkipWidth:0,labelSkipHeight:0,legends:[],totalsOffset:0};if(borderColor!==undefined)props.borderColor=borderColor;
 const result=useBar(props);assert.equal(result.bars.length,6);assert.equal(result.barsWithValue.length,3);assert.equal(result.barsWithValue.filter(b=>b.data.value===0).length,1);
 for(const b of result.barsWithValue){assert.equal(typeof b.color,'string');assert.equal(b.data.data.color,b.data.indexValue==='zero'?undefined:2);const border=result.getBorderColor(b),row={groupMode,layout,profile,series:b.data.id,index:b.data.indexValue,raw:b.data.value,originalSourceNumericColor:b.data.data.color??null,nativeFilteredRawColorPropertyAbsent:!Object.hasOwn(b.data.data,'color'),rawInputNumericColor:source.find(d=>d.index===b.data.indexValue).color,originalComputedColor:b.color,sourceRGBA:palette[b.data.id],actualBorderCSS:border,actualBorderRGBA:bytesFromReturnedCSS(border),traces:[]};
  for(const width of [0,2]){const rec=recorder();renderBar(rec.ctx,{bar:b,borderColor:border,borderRadius:0,borderWidth:width,shouldRenderLabel:false,label:'',labelStyle:{},labelX:0,labelY:0,textAnchor:'middle'});assert.equal(rec.calls.filter(c=>c.op==='stroke').length,width===0?0:1);row.traces.push({borderWidth:width,calls:rec.calls});}
  values.bar.push(row);
 }
 if(groupMode==='grouped'&&layout==='vertical'&&profile==='inherit'){
  const b=result.barsWithValue[0];const fromRaw=getInheritedColorGenerator({from:'data.data.roleCss'})(b);values.propertyAndFunction.push({case:'realBarNestedRawProperty',actual:fromRaw,rgba:bytesFromReturnedCSS(fromRaw),originalComputedBarColor:b.color,sourceRawNumericColor:b.data.data.color});
  const fn=d=>d.color;assert.equal(getInheritedColorGenerator(fn),fn);values.propertyAndFunction.push({case:'customFunctionIdentity',actual:getInheritedColorGenerator(fn)(b),sameFunctionObject:true});
 }
}
const heatSource=[{id:'r',data:[{x:'low',y:0},{x:'high',y:10},{x:'null',y:null}]}],heatPalette=[[20,40,60,160],[200,180,160,220]],unknown=[90,100,110,64];
const computed=computeCells({data:heatSource,width:240,height:120,xInnerPadding:0,xOuterPadding:0,yInnerPadding:0,yOuterPadding:0,forceSquare:false});assert.equal(computed.cells.length,3);
for(const [profile,configured]of profiles)for(const activeIds of [[],['r.null']]){
 const styled=useCellsStyle({cells:computed.cells,minValue:0,maxValue:10,sizeVariation:false,colors:c=>rgba(heatPalette[c.value===0?0:1]),emptyColor:rgba(unknown),opacity:.4,activeOpacity:1,inactiveOpacity:.15,borderColor:configured??heatDefault,label:'formattedValue',labelTextColor:{theme:'labels.text.fill'},activeIds});
 for(const cell of styled.cells){const sourceRGBA=cell.value===null?unknown:heatPalette[cell.value===0?0:1],row={profile,activeIds,id:cell.id,row:cell.serieId,column:cell.data.x,raw:cell.value,formattedValue:cell.formattedValue,sourceRGBA,color:cell.color,opacity:cell.opacity,actualBorderCSS:cell.borderColor,actualBorderRGBA:bytesFromReturnedCSS(cell.borderColor),traces:[]};assert.equal(cell.color,rgba(sourceRGBA));
  for(const width of [0,2])for(const [shape,renderer]of [['rect',renderRect],['circle',renderCircle]]){const rec=recorder();renderer(rec.ctx,{cell,borderWidth:width,enableLabels:false,theme:{}});assert.equal(rec.calls.filter(c=>c.op===(shape==='rect'?'strokeRect':'stroke')).length,width===0?0:1);row.traces.push({shape,borderWidth:width,calls:rec.calls});}
  values.heatmap.push(row);
 }
}
for(const [name,config,theme]of [['themeMissingObject',{theme:'border.stroke'},undefined],['themeMissingPath',{theme:'border.missing'},{}],['unknownModifier',{from:'color',modifiers:[['invalid',1]]},{}],['plainObjectWithoutRole',{},{}]]){let error;try{getInheritedColorGenerator(config,theme);}catch(e){error=e.message;}assert(error);values.nativeErrors.push({name,error,generatorCreationBeforeAnyDatum:true});}
const valueText=JSON.stringify(values,null,2)+'\n';
const referenceText=JSON.stringify({schema:1,scope:'Execute original inherited-color generator/hook, original useBar plus grouped/stacked generators, original computeCells/useCellsStyle and original Canvas renderer function bodies. Immediate React hook adapters are isolated snapshots, not React lifecycle. Labels disabled and Canvas records commands only, not pixels. Numeric RGB serialization adapter decodes actual returned CSS without reparsing and losing alpha-zero hidden RGB.',upstream:{version:vendorPins.version,commit:vendorPins.commit},counts:{sharedResolver:values.sharedResolver.length,bar:values.bar.length,heatmap:values.heatmap.length,nativeErrors:values.nativeErrors.length},defaults:{bar:barDefault,heatmap:heatDefault},dependencies:versions,sourcePins:pins,runtimePins,adapters:['useMemo=f=>f/useCallback identity/useState initial snapshot (setter deliberately throws)','useTheme explicit fixture theme object','flat/dotted own-data property lookup only; no bracket/lodash-wide equivalence','explicit function palette only; no built-in scheme inference recomputation','property/function accessor and String formatter for supplied inputs','bar legends empty; string-key firstuniq adapter not exercised','Canvas recording Proxy is not native canvas; font/text functions throw if called'],frozenShared42CasesReplayedWithCompleteOriginalResolver:true,valuesSHA256:hash(valueText),limitations:['Contrast is Rust-only extension and is not claimed as an original Nivo resolver branch.','Hidden/cropped/disabled-border preflight timing in checked Rust is a declared extension; original React lifecycle not simulated.','CSS parser alpha-zero RGB loss is retained in six actual native resolver cases; no shared Rust resolver change to force parity.','Original Nivo borderWidth0 avoids drawing but still computes role before renderer; Rust None border stays disabled.','Whole native Canvas function body ran with recording context, no browser/pixel/Glory rendering acceptance.']},null,2)+'\n';
for(const [name,text]of [['values.json',valueText],['reference.json',referenceText]]){const path=join(fixture,name);if(check)assert.equal(fs.readFileSync(path,'utf8'),text);else fs.writeFileSync(path,text);}
console.log(JSON.stringify({passed:true,checked:check,shared42:values.sharedResolver.length,bar:values.bar.length,heatmap:values.heatmap.length,errors:values.nativeErrors.length,valuesSHA256:hash(valueText)}));
