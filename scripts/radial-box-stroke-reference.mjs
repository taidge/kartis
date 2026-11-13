// Execute complete original Nivo hooks, transitions, JSX components and resolver.
// node scripts/radial-box-stroke-reference.mjs <scale-node_modules> <shape-node_modules> [--reference-root <project>] [--check]
import fs from 'node:fs';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {stripTypeScriptTypes,createRequire} from 'node:module';
import {dirname,join,resolve} from 'node:path';
import {fileURLToPath,pathToFileURL} from 'node:url';
const root=resolve(dirname(fileURLToPath(import.meta.url)),'..'),args=process.argv.slice(2),check=args.includes('--check');
const ri=args.indexOf('--reference-root'),reference=ri<0?root:resolve(args[ri+1]);
assert(args[0]&&args[1],'Supply existing released scale/color and shape node_modules roots');
const locations={scale:resolve(args[0]),shape:resolve(args[1])},fixture=join(root,'tests/fixtures/radial-box-stroke'),vendor=join(fixture,'vendor');
const hash=v=>createHash('sha256').update(v).digest('hex'),sourcePins={},runtimePins={};
const read=(p)=>{const b=fs.readFileSync(p);sourcePins[p.startsWith(root)?p.slice(root.length+1).replaceAll('\\','/'):p.slice(reference.length+1).replaceAll('\\','/')]=hash(b);return b.toString('utf8').replaceAll('\r\n','\n');};
const vendorPins=JSON.parse(read(join(vendor,'source-pins.json')));
for(const [name,r]of Object.entries(vendorPins.files))assert.equal(hash(fs.readFileSync(join(vendor,name))),r.sha256);
const shared=join(reference,'tests/fixtures/bar-label-color'),boxVendor=join(reference,'tests/fixtures/boxplot-domain-bounds/vendor'),bandVendor=join(reference,'tests/fixtures/bar-category-band/vendor/raw');
for(const [manifest,folder]of [[join(boxVendor,'source-pins.json'),boxVendor]])for(const [name,r]of Object.entries(JSON.parse(read(manifest)).files))assert.equal(hash(fs.readFileSync(join(folder,name))),r.sha256);
const versions={'d3-scale':['scale','4.0.2'],'d3-interpolate':['scale','3.0.1'],'d3-array':['scale','3.2.5'],'d3-color':['scale','3.1.0'],'d3-shape':['shape','3.2.0']};
function entry(name){const [where,version]=versions[name],pkg=JSON.parse(fs.readFileSync(join(locations[where],name,'package.json')));assert.equal(pkg.version,version);for(const f of ['package.json',pkg.module])runtimePins[name+'/'+f]=hash(fs.readFileSync(join(locations[where],name,f)));return JSON.stringify(pathToFileURL(join(locations[where],name,pkg.module)).href);}
const require=createRequire(import.meta.url),babelPath=join(reference,'node_modules/playwright/lib/transform/babelBundle.js');
const {babelTransform}=require(babelPath),playwrightPackage=JSON.parse(fs.readFileSync(join(reference,'node_modules/playwright/package.json')));
assert.equal(playwrightPackage.version,'1.56.1');for(const f of ['package.json','lib/transform/babelBundle.js','lib/transform/babelBundleImpl.js','LICENSE'])runtimePins['playwright/'+f]=hash(fs.readFileSync(join(reference,'node_modules/playwright',f)));
const stripImports=s=>s.replace(/^import[\s\S]*?from\s*['"][^'"]+['"]\n/gm,'');
const dataURL=s=>'data:text/javascript;base64,'+Buffer.from(s).toString('base64');
const types=s=>dataURL(stripTypeScriptTypes(s,{mode:'transform'}));
const adapter=types(`
export const useMemo=f=>f(),useCallback=f=>f;
let theme,stateOverride,transitions=[];
export const setTheme=t=>{theme=t},useTheme=()=>theme;
export const setState=v=>{stateOverride=v},useState=v=>[stateOverride===undefined?v:stateOverride,()=>{throw Error('No React state commits in cold snapshot');}];
export const get=(value,path)=>{if(typeof path!=='string'||!/^\\w+(?:\\.\\w+)*$/.test(path))throw Error('Property path outside adapter scope');return path.split('.').reduce((v,k)=>v==null?undefined:v[k],value);};
export const set=(value,path,x)=>{if(!/^\\w+(?:\\.\\w+)*$/.test(path))throw Error('Property write outside scope');const keys=path.split('.');let o=value;for(const k of keys.slice(0,-1))o=o[k]??=( {} );o[keys.at(-1)]=x;};
export const isPlainObject=v=>v!==null&&typeof v==='object'&&Object.getPrototypeOf(v)===Object.prototype;
export const isFunction=v=>typeof v==='function';
export const pick=(v,keys)=>Object.fromEntries(keys.filter(k=>k in v).map(k=>[k,v[k]]));
export const isEqual=(a,b)=>JSON.stringify(a)===JSON.stringify(b);
export const uniqBy=(values,key)=>{const seen=new Set();return values.filter(v=>{const k=key(v);if(seen.has(k))return false;seen.add(k);return true;});};
export const sortBy=(values,key)=>[...values].sort((a,b)=>key(a)-key(b));
export const usePropertyAccessor=p=>typeof p==='function'?p:d=>get(d,p);
export const getPropertyAccessor=usePropertyAccessor;
export const useValueFormatter=f=>typeof f==='function'?f:String;
export const useMotionConfig=()=>({animate:false,config:{}});
export const useTooltip=()=>({showTooltipFromEvent:()=>{throw Error('Tooltip interaction not executed')},hideTooltip:()=>{throw Error('Tooltip interaction not executed')}});
export const degreesToRadians=x=>x*Math.PI/180;
export const getScaleTicks=scale=>{if(scale.type!=='band')throw Error('Only native band-radius ticks used');return scale.domain();};
export const unsupported=()=>{throw Error('Adapter branch outside declared source profile');};
export const Fragment='Fragment',jsx=(type,props,key)=>({type,props:props??{},key}),jsxs=jsx;
export const createElement=(type,props,...children)=>({type,props:{...props,...(children.length?{children}: {})}});
export const animated=new Proxy({}, {get:(_,key)=>key});
export const to=(values,fn)=>fn(...values);
export const resetTransitions=()=>{transitions=[]},getTransitions=()=>transitions;
export const useTransition=(items,options)=>{
const rows=items.map(item=>({key:options.keys(item),initial:typeof options.initial==='function'?options.initial(item):options.initial,
from:options.from(item),enter:options.enter(item),update:options.update(item),leave:options.leave(item)}));
transitions.push({initialKind:typeof options.initial,immediate:options.immediate,rows});
return render=>items.map((item,i)=>render(rows[i].update,item));};
`);
const adapters=await import(adapter);
function ts(name,prefix=''){return types(prefix+'\n'+stripImports(read(join(vendor,name))));}
function tsx(name,prefix=''){
 const original=prefix+'\n'+stripImports(read(join(vendor,name)));
 let code=babelTransform(original,'radial-box-stroke/'+name,true,[],[]).code;
 code=code.replace(/^import \{([^}]+)\} from ['"][^'"]*jsx-runtime['"];?/gm,(_,bindings)=>'import {'+bindings+'} from '+JSON.stringify(adapter)+';').replace(/^\/\/# sourceMappingURL=.*$/gm,'');
 assert(!code.includes('playwright/jsx-runtime'));return dataURL(code);
}
const inherited=types(`import {useMemo,get,isPlainObject}from ${JSON.stringify(adapter)};import{rgb}from ${entry('d3-color')};\n`+stripImports(read(join(shared,'nivo-inheritedColor.ts'))));
const ordinal=ts('ordinal-color.ts',`import{useMemo,get,isPlainObject,unsupported}from ${JSON.stringify(adapter)};import{scaleOrdinal}from ${entry('d3-scale')};const colorSchemes={};const isCategoricalColorScheme=unsupported,isSequentialColorScheme=unsupported,isDivergingColorScheme=unsupported;`);
const radialProps=ts('radial-props.ts','const RadialBarTooltip=()=>{throw Error("Tooltip component not executed")};');
const boxProps=ts('box-props.ts','const BoxPlotItem=()=>{throw Error("Props component placeholder not executed")},BoxPlotTooltip=BoxPlotItem,BoxPlotTooltipLabel=BoxPlotItem;');
const boxDefaults=(await import(boxProps)).defaultProps,radialDefaults=(await import(radialProps)).svgDefaultProps;
const linear=types(`import{scaleLinear}from ${entry('d3-scale')};import{interpolateNumber,interpolateRound}from ${entry('d3-interpolate')};\n`+stripImports(read(join(boxVendor,'linearScale.ts'))));
const band=types(`import{scaleBand}from ${entry('d3-scale')};\n`+stripImports(read(join(bandVendor,'nivo-bandScale.ts'))));
const dispatch=read(join(boxVendor,'compute.ts')).match(/^export function computeScale[\s\S]*?^\}/m)?.[0];assert(dispatch);
const scales=types(`import{createLinearScale}from ${JSON.stringify(linear)};import{createBandScale}from ${JSON.stringify(band)};import{unsupported}from ${JSON.stringify(adapter)};const createLogScale=unsupported,createSymlogScale=unsupported,createTimeScale=unsupported;\n`+dispatch);
const common=types(`import{computeScale}from ${JSON.stringify(scales)};\n`+stripImports(read(join(boxVendor,'common.ts'))));
const strata=types(`import{defaultProps}from ${JSON.stringify(boxProps)};\n`+stripImports(read(join(boxVendor,'stratification.ts'))));
const generation=types(`import{computeScale}from ${JSON.stringify(scales)};import{getIndexScale}from ${JSON.stringify(common)};\n`+stripImports(read(join(boxVendor,'generation.ts'))));
const legends=ts('box-legends.ts',`import{getPropertyAccessor,uniqBy,sortBy}from ${JSON.stringify(adapter)};`);
const boxHook=ts('box-hooks.ts',`import{useMemo,useState,usePropertyAccessor,useValueFormatter,useTheme,useTransition}from ${JSON.stringify(adapter)};import{useInheritedColor}from ${JSON.stringify(inherited)};import{useOrdinalColorScale}from ${JSON.stringify(ordinal)};import{defaultProps}from ${JSON.stringify(boxProps)};import{stratifyData,summarizeDistribution}from ${JSON.stringify(strata)};import{generateBoxPlots}from ${JSON.stringify(generation)};import{getLegendData}from ${JSON.stringify(legends)};`);
const boxItem=tsx('box-item.tsx',`import{createElement,useCallback,useMemo,animated,useTooltip}from ${JSON.stringify(adapter)};`);
const radialHook=ts('radial-hooks.ts',`import{useMemo,degreesToRadians,useValueFormatter,getScaleTicks}from ${JSON.stringify(adapter)};import{scaleBand,scaleLinear}from ${entry('d3-scale')};import{arc as d3Arc}from ${entry('d3-shape')};import{useOrdinalColorScale}from ${JSON.stringify(ordinal)};import{castLinearScale}from ${JSON.stringify(linear)};import{castBandScale}from ${JSON.stringify(band)};import{commonDefaultProps,svgDefaultProps}from ${JSON.stringify(radialProps)};`);
const arcMode=ts('arc-transition-mode.ts',`import{useMemo}from ${JSON.stringify(adapter)};`);
const interpolate=ts('interpolate-arc.ts',`import{to}from ${JSON.stringify(adapter)};`);
const arcTransition=ts('arcs-transition.ts',`import{useTransition,useMotionConfig}from ${JSON.stringify(adapter)};import{useArcTransitionMode}from ${JSON.stringify(arcMode)};import{interpolateArc}from ${JSON.stringify(interpolate)};`);
const arcShape=tsx('arc-shape.tsx',`import{useCallback,animated}from ${JSON.stringify(adapter)};`);
const arcLayer=tsx('arcs-layer.tsx',`import{createElement,useTheme}from ${JSON.stringify(adapter)};import{useInheritedColor}from ${JSON.stringify(inherited)};import{useArcsTransition}from ${JSON.stringify(arcTransition)};import{ArcShape}from ${JSON.stringify(arcShape)};`);
const radialArcs=tsx('radial-arcs.tsx',`import{createElement,useCallback,useTooltip}from ${JSON.stringify(adapter)};import{ArcsLayer}from ${JSON.stringify(arcLayer)};`);
const defs=types(`import{isFunction,isPlainObject,pick,isEqual,get,set}from ${JSON.stringify(adapter)};const gradientTypes={linearGradient:1},patternTypes={patternDots:1};\n`+stripImports(read(join(vendor,'bind-defs.js'))));
const {useBoxPlot,useBoxPlotTransition}=await import(boxHook),{BoxPlotItem}=await import(boxItem),{useRadialBar}=await import(radialHook),{RadialBarArcs}=await import(radialArcs),{bindDefs}=await import(defs),{getInheritedColorGenerator}=await import(inherited);
function render(node){if(node==null||node===false)return null;if(Array.isArray(node))return node.flatMap(x=>{const y=render(x);return Array.isArray(y)?y:[y];}).filter(x=>x!=null);if(typeof node!=='object')return node;if(typeof node.type==='function')return render(node.type(node.props));const {children,...props}=node.props;return {tag:node.type,attrs:Object.fromEntries(Object.entries(props).filter(([k,v])=>typeof v!=='function'&&v!==undefined)),children:render(children??[])};}
function flatten(node){if(node==null)return [];if(Array.isArray(node))return node.flatMap(flatten);return typeof node==='object'?[node,...flatten(node.children)]:[];}
const rgba=c=>`rgba(${c[0]},${c[1]},${c[2]},${c[3]/255})`;
function bytes(css){if(typeof css!=='string')return {nativeType:typeof css,value:css??null};const m=css.match(/^rgba?\(\s*([\d.]+),\s*([\d.]+),\s*([\d.]+)(?:,\s*([\d.]+))?\)$/);return m?[+m[1],+m[2],+m[3],Math.round((m[4]===undefined?1:+m[4])*255)]:{nativeCSS:css};}
const theme={labels:{text:{fill:rgba([70,80,90,123])}},grid:{line:{stroke:rgba([120,20,80,90])}},axis:{domain:{line:{stroke:rgba([20,90,120,200])}}}};
adapters.setTheme(theme);
const profiles=[['native-default',null],['inherit',{from:'color'}],['darker',{from:'color',modifiers:[['darker',1]]}],['ordered',{from:'color',modifiers:[['brighter',1],['opacity',.5],['darker',1]]}],['fixed',rgba([12,34,56,78])],['theme','THEME'],['nested',{from:'data.tint'}],['alpha0-visible',{from:'color',modifiers:[['darker',1],['opacity',1]]}]];
const result={radial:[],box:[],qualificationCases:[],nativeErrors:[],propertyBoundaries:[]};
const palette=[rgba([20,40,60,160]),rgba([200,180,160,220]),rgba([9,8,7,0])];
const inputRadial=[{id:'later',data:[{x:'B',y:2,tint:rgba([80,10,20,110]),color:123},{x:'A',y:0,tint:rgba([40,90,10,120]),color:456}]},{id:'first',data:[{x:'C',y:3,tint:rgba([10,20,90,130]),color:789},{x:'B',y:null,tint:rgba([70,30,20,140]),color:0}]}];
for(const reverse of [false,true])for(const width of [0,2])for(const [profile,config]of profiles){
 const data=structuredClone(reverse?[...inputRadial].reverse():inputRadial);
 const layout=useRadialBar({data,width:320,height:280,maxValue:'auto',valueFormat:String,colors:palette});
 assert.equal(layout.bars.length,4);const categories=[...new Set(data.flatMap(s=>s.data.map(d=>d.x)))];assert.deepEqual(layout.legendData.map(d=>d.id),categories);
 const paintBefore=layout.bars.map(d=>d.color);const bound=bindDefs([{id:'pattern',type:'patternDots',color:'inherit',background:'white'}],layout.bars,[{id:'pattern',match:d=>d.value===2}],{dataKey:'data',targetKey:'fill'});
 assert.deepEqual(layout.bars.map(d=>d.color),paintBefore);
 adapters.resetTransitions();const cfg=profile==='native-default'?radialDefaults.borderColor:config==='THEME'?{theme:'labels.text.fill'}:config;
 const rendered=render(adapters.createElement(RadialBarArcs,{center:layout.center,bars:layout.bars,borderWidth:width,borderColor:cfg,arcGenerator:layout.arcGenerator,isInteractive:false,transitionMode:'centerRadius'}));
 const paths=flatten(rendered).filter(x=>x.tag==='path');assert.equal(paths.length,4);const phases=adapters.getTransitions();assert.equal(phases.length,1);
 const records=layout.bars.map((bar,i)=>{const transition=phases[0].rows[i],path=paths[i].attrs;assert.equal(path.fill,bar.fill||bar.color);assert.equal(path.stroke,transition.update.borderColor);assert.equal(path.strokeWidth,width);assert.equal(path.opacity,1);return {id:bar.id,groupId:bar.groupId,category:bar.category,rawValue:bar.value,formattedValue:bar.formattedValue,stackedValue:bar.stackedValue,arc:bar.arc,rawDatum:bar.data,numericColor:bar.color,numericRGBA:bytes(bar.color),boundFill:bar.fill??null,roleBorderCSS:path.stroke,roleBorderRGBA:bytes(path.stroke),transition,renderAttrs:path};});
 result.radial.push({id:`radial-${reverse?'reversed':'authored'}-w${width}-${profile}`,profile,borderColor:cfg,borderWidth:width,groups:data.map(d=>d.id),categories,palette,legend:layout.legendData,boundDefs:bound,bars:records,scope:'OriginalRadialwholehook+ArcsLayer/mode/transition/ArcShape; nativeNULLbar kept, not claimedRustnullableeligible'});
}
// Explicit main-case outline width; native-default/zero-width qualification remains unchanged.
const mainBoxBorderWidth=2;
const rawBox=[{group:'A',subGroup:'x',v:1},{group:'A',subGroup:'x',v:3},{group:'A',subGroup:'y',v:0},{group:'A',subGroup:'y',v:4},{group:'B',subGroup:'x',v:2},{group:'B',subGroup:'x',v:6},{group:'B',subGroup:'z',v:2},{group:'B',subGroup:'z',v:4},{group:'C',subGroup:'x',v:5},{group:'C',subGroup:'x',v:5}];
for(const colorBy of ['group','subGroup'])for(const layout of ['vertical','horizontal'])for(const [profile,config]of profiles.filter(p=>p[0]!=='nested')){
 adapters.setState(undefined);
 const cfg=role=>profile==='native-default'?boxDefaults[role+'Color']:config==='THEME'?{theme:{border:'labels.text.fill',median:'grid.line.stroke',whisker:'axis.domain.line.stroke'}[role]}:config;
 const options={data:rawBox,value:'v',groupBy:'group',subGroupBy:'subGroup',groups:['A','B','C'],subGroups:['x','y','z'],width:360,height:240,layout,minValue:'auto',maxValue:'auto',valueScale:{type:'linear',nice:false},indexScale:{type:'band',round:false},padding:.1,innerPadding:4,colorBy,colors:palette,opacity:.8,activeOpacity:.9,inactiveOpacity:.2,borderColor:cfg('border'),medianColor:cfg('median'),whiskerColor:cfg('whisker'),tooltipLabel:d=>d.group+'/'+d.subGroup,valueFormat:String,legends:[]};
 const computed=useBoxPlot(options);assert.equal(computed.boxPlots.length,5);
 const before=computed.boxPlots.map(d=>d.color),bound=bindDefs([{id:'gradient',type:'linearGradient',colors:[{offset:0,color:'inherit'},{offset:100,color:'white'}]}],computed.boxPlots,[{id:'gradient',match:d=>d.group==='A'&&d.subGroup==='x'}],{dataKey:'data',targetKey:'fill'});assert.deepEqual(computed.boxPlots.map(d=>d.color),before);
 adapters.resetTransitions();const transition=useBoxPlotTransition({...computed,animate:false,springConfig:{}});
 const rendered=render(transition((animatedProps,boxPlot)=>adapters.createElement(BoxPlotItem,{boxPlot,layout,animatedProps,borderRadius:0,borderWidth:mainBoxBorderWidth,medianWidth:boxDefaults.medianWidth,whiskerWidth:boxDefaults.whiskerWidth,whiskerEndSize:boxDefaults.whiskerEndSize,isInteractive:false,isFocusable:false,setActiveItem:computed.setActiveItem})));
 const phases=adapters.getTransitions();assert.equal(phases.length,1);const groups=rendered.filter(x=>x.tag==='g');assert.equal(groups.length,5);
 const boxes=computed.boxPlots.map((box,i)=>{const items=flatten(groups[i]),rect=items.find(x=>x.tag==='rect'),lines=items.filter(x=>x.tag==='line'),props=phases[0].rows[i];assert.equal(rect.attrs.fill,box.fill??box.color);assert.equal(rect.attrs.stroke,props.update.borderColor);assert.equal(lines[0].attrs.stroke,props.update.medianColor);assert(lines.slice(1).every(x=>x.attrs.stroke===props.update.whiskerColor));return {key:box.key,group:box.group,subGroup:box.subGroup,sourceSummary:box.data,geometry:{x:box.x,y:box.y,width:box.width,height:box.height,coordinates:box.coordinates},numericColor:box.color,numericRGBA:bytes(box.color),boundFill:box.fill??null,roles:Object.fromEntries(['border','median','whisker'].map(role=>[role,{css:props.update[role+'Color'],rgba:bytes(props.update[role+'Color'])}])),transition:props,render:{groupAttrs:groups[i].attrs,rectAttrs:rect.attrs,lineAttrs:lines.map(x=>x.attrs)}};});
 result.box.push({id:`box-${colorBy}-${layout}-${profile}`,profile,colorBy,layout,palette,configs:{border:cfg('border'),median:cfg('median'),whisker:cfg('whisker')},widths:{border:mainBoxBorderWidth,median:boxDefaults.medianWidth,whisker:boxDefaults.whiskerWidth},boundDefs:bound,boxes,rustComparableMode:colorBy==='subGroup',scope:'OriginalwholeuseBoxPlot+stratification/generation+transition+BoxPlotItem; groupmode independentlyrecordedRustAPIgap, not forcedparity'});
}
// Explicit upstream-input and state adapters distinguish native eligibility from
// typed Rust semantics; no native hidden prop or React lifecycle is fabricated.
const boxOptions={data:rawBox,value:'v',groupBy:'group',subGroupBy:'subGroup',groups:['A','B','C'],subGroups:['x','y','z'],width:360,height:240,layout:'vertical',minValue:'auto',maxValue:'auto',valueScale:{type:'linear',nice:false},indexScale:{type:'band',round:false},padding:.1,innerPadding:4,colorBy:'subGroup',colors:palette,opacity:.8,activeOpacity:.9,inactiveOpacity:.2,borderColor:boxDefaults.borderColor,medianColor:boxDefaults.medianColor,whiskerColor:boxDefaults.whiskerColor,tooltipLabel:d=>d.group+'/'+d.subGroup,valueFormat:String,legends:[]};
function qualifyBox(id,data,extra={},active){
 adapters.setState(active);const computed=useBoxPlot({...boxOptions,data,...extra});adapters.resetTransitions();
 const transition=useBoxPlotTransition({...computed,animate:false,springConfig:{}});
 const attrs=render(transition((animatedProps,boxPlot)=>adapters.createElement(BoxPlotItem,{boxPlot,layout:'vertical',animatedProps,borderWidth:extra.strokeWidthsZero?0:0,medianWidth:extra.strokeWidthsZero?0:2,whiskerWidth:extra.strokeWidthsZero?0:2,whiskerEndSize:.6,isInteractive:false,setActiveItem:computed.setActiveItem})));
 const boxes=computed.boxPlots.map((b,i)=>({key:b.key,group:b.group,subGroup:b.subGroup,sourceSummary:b.data,color:b.color,opacity:b.opacity,transition:adapters.getTransitions()[0].rows[i],render:attrs[i]}));
 result.qualificationCases.push({id,family:'box',input:data,extra:Object.fromEntries(Object.entries(extra).filter(([,v])=>typeof v!=='function')),activeInput:active??null,boxes,scope:'Actualoriginalwholefunctions; hidden cases are explicit upstream datafilter, null raw maps through nativeNumber(null), no Rusteligibility assertion'});adapters.setState(undefined);return computed;
}
qualifyBox('box-empty',[]);
qualifyBox('box-upstream-filter-hidden-B',rawBox.filter(d=>d.group!=='B'));
qualifyBox('box-native-null-number-zero',[{group:'A',subGroup:'x',v:null},{group:'A',subGroup:'x',v:2},{group:'A',subGroup:'x',v:4}]);
qualifyBox('box-constant-widths-zero',[{group:'A',subGroup:'x',v:5}],{strokeWidthsZero:true});
const baseline=qualifyBox('box-active-source-baseline',rawBox);
qualifyBox('box-active-original-computed-item',rawBox,{},baseline.boxPlots[0]);
const precomputed={...baseline.boxPlots[0].data,tint:rgba([80,10,20,110])};
qualifyBox('box-precomputed-nested-property',[precomputed],{borderColor:{from:'data.tint'},medianColor:{from:'data.tint'},whiskerColor:{from:'data.tint'}});
for(const [id,data]of [['radial-empty',[]],['radial-upstream-filter-hidden-later',inputRadial.filter(s=>s.id!=='later')],['radial-zero-only',[{id:'zero',data:[{x:'A',y:0}]}]],['radial-null-only',[{id:'null',data:[{x:'A',y:null}]}]]]){
 const layout=useRadialBar({data,width:320,height:280,maxValue:'auto',colors:palette,valueFormat:String});adapters.resetTransitions();
 const rendered=render(adapters.createElement(RadialBarArcs,{center:layout.center,bars:layout.bars,borderWidth:0,borderColor:radialDefaults.borderColor,arcGenerator:layout.arcGenerator,isInteractive:false,transitionMode:'centerRadius'}));
 result.qualificationCases.push({id,family:'radial',input:data,categories:layout.legendData.map(x=>x.id),bars:layout.bars,transition:adapters.getTransitions(),render:rendered,scope:'Nativecomputed0/nullsegments retained; upstreamfilter changes firstoccurrencepalette domain, not a native hidden API'});
}
for(const [name,invoke]of [['empty-radial-whole-layer-invalid',()=>render(adapters.createElement(RadialBarArcs,{center:[0,0],bars:[],borderWidth:0,borderColor:{from:'color',modifiers:[['bad',1]]},isInteractive:false,transitionMode:'centerRadius'}))],['empty-box-whole-hook-invalid',()=>useBoxPlot({...boxOptions,data:[],borderColor:{from:'color',modifiers:[['bad',1]]}})]]){
 try{invoke();assert.fail('Wholehookmustrejectintrinsicconfig');}catch(e){assert(!e.message.includes('Wholehookmustreject'));result.nativeErrors.push({name,message:e.message,sourceEntry:'actualwholehook/componentwithnoeligibleitems'});}
}
for(const [name,config,themeValue]of [['missing-theme',{theme:'labels.text.fill'},undefined],['missing-path',{theme:'missing.path'},theme],['unknown-modifier',{from:'color',modifiers:[['bogus',1]]},theme],['invalid-object',{unexpected:true},theme]]){
 try{getInheritedColorGenerator(config,themeValue);assert.fail('Originalresolvermustthrow');}catch(e){assert(!e.message.includes('Originalresolvermustthrow'));result.nativeErrors.push({name,message:e.message});}
}
for(const config of [{from:'data.color'},{from:'data.tint'},{from:'missing.path'}]){const datum={color:rgba([20,40,60,160]),data:{color:123,tint:rgba([9,8,7,0])}},value=getInheritedColorGenerator(config,theme)(datum);result.propertyBoundaries.push({config,datum,returned:bytes(value),scope:'Actualarbitrarypropertyrole not supportedbyRusttypedFromContext; do not fabricateCSSorforceparity'});}
for(const c of result.radial)for(const b of c.bars){
 b.rustNullableSourceEligible=typeof b.rawValue==='number'&&Number.isFinite(b.rawValue);
 b.cssAlpha0ParserBoundary=Array.isArray(b.numericRGBA)&&b.numericRGBA[3]===0&&Array.isArray(c.borderColor?.modifiers)&&c.borderColor.modifiers.length>0;
 b.rustComparableRole=b.rustNullableSourceEligible&&c.profile!=='nested'&&!b.cssAlpha0ParserBoundary;
 b.nativeZeroExtentComputedDatum=b.rawValue===null||b.rawValue===0;
 if(c.profile==='alpha0-visible'&&b.cssAlpha0ParserBoundary)assert.deepEqual(b.roleBorderRGBA,[0,0,0,255]);
}
for(const c of result.box)for(const b of c.boxes)for(const role of ['border','median','whisker']){
 const config=c.configs[role],boundary=b.numericRGBA[3]===0&&config?.from==='color'&&Array.isArray(config.modifiers)&&config.modifiers.length>0;
 b.roles[role].cssAlpha0ParserBoundary=boundary;b.roles[role].rustComparable=c.colorBy==='subGroup'&&!boundary;
 if(c.profile==='alpha0-visible'&&boundary)assert.deepEqual(b.roles[role].rgba,[0,0,0,255]);
 if(c.profile==='inherit'&&b.numericRGBA[3]===0)assert.deepEqual(b.roles[role].rgba,[9,8,7,0]);
}
assert.equal(result.qualificationCases.find(c=>c.id==='box-native-null-number-zero').boxes[0].sourceSummary.n,3);
assert(result.qualificationCases.find(c=>c.id==='box-native-null-number-zero').boxes[0].sourceSummary.extrema[0]===0);
assert.equal(result.qualificationCases.find(c=>c.id==='box-empty').boxes.length,0);
assert.equal(result.qualificationCases.find(c=>c.id==='radial-empty').bars.length,0);
assert.equal(result.qualificationCases.find(c=>c.id==='radial-null-only').bars[0].value,null);
const data=JSON.stringify(result,null,2)+'\n';
const metadata={schema:'radial-box-stroke-original-v2',nivoVersion:vendorPins.nivoVersion,nivoCommit:vendorPins.nivoCommit,counts:{radialCases:result.radial.length,radialBars:result.radial.reduce((n,c)=>n+c.bars.length,0),boxCases:result.box.length,boxItems:result.box.reduce((n,c)=>n+c.boxes.length,0),boxRoleOutputs:result.box.reduce((n,c)=>n+3*c.boxes.length,0),qualificationCases:result.qualificationCases.length,nativeErrors:result.nativeErrors.length,propertyBoundaries:result.propertyBoundaries.length},valuesSHA256:hash(data),sourcePins,runtimePins,scope:'Actual completeoriginalfunctions andTSXcomponentbodies executedunderexplicitcoldReact/Spring/JSXrecordingadapters. Main Box outlines explicitly width2; qualification retains native default/zero width0. Original renderattributes recorded, notpixelsorReactlifecycle/animation. Explicitgroup/subgrouppalettes with actualthird alpha0slot; schemebranches retainedbutnotexecuted. CSSalpha0loss withmodifiers preserved; typedRustContrastisextension. NativeJSnull andarbitrarypropertymode not assertedRustparity.'};
const manifest=JSON.stringify(metadata,null,2)+'\n';
for(const [file,contents]of [['values.json',data],['reference.json',manifest]]){const p=join(fixture,file);if(check)assert.equal(fs.readFileSync(p,'utf8'),contents);else fs.writeFileSync(p,contents);}
console.log(JSON.stringify({passed:true,checked:check,...metadata.counts,valuesSHA256:metadata.valuesSHA256}));
