// Original Funnel handler/tooltip functions. Explicit cold adapters, no React/DOM.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {execFileSync} from 'node:child_process';
import {mkdir,readFile,writeFile} from 'node:fs/promises';
import {createRequire,stripTypeScriptTypes} from 'node:module';
import {dirname,resolve} from 'node:path';
import {fileURLToPath} from 'node:url';
import {area,line,curveBasis,curveLinear} from 'd3-shape';

const script=fileURLToPath(import.meta.url),root=resolve(dirname(script),'..');
const args=Object.fromEntries(process.argv.slice(2).map(value=>{const i=value.indexOf('=');assert(i>2);return[value.slice(2,i),value.slice(i+1)];}));
assert.deepEqual(Object.keys(args).sort(),['nivo','out']);
const checkout=resolve(args.nivo),out=resolve(args.out),sha=b=>createHash('sha256').update(b).digest('hex');
const commit=execFileSync('git',['-C',checkout,'rev-parse','HEAD'],{encoding:'utf8'}).trim();
const require=createRequire(import.meta.url),{babelParse,babelTransform,traverse}=require(resolve(root,'node_modules/playwright/lib/transform/babelBundle.js'));
const names=['funnel/src/hooks.ts','funnel/src/Part.tsx','funnel/src/PartTooltip.tsx','funnel/src/Parts.tsx','funnel/src/Funnel.tsx','funnel/src/types.ts','funnel/LICENSE.md','funnel/package.json','tooltip/src/hooks.ts','tooltip/src/context.ts','tooltip/src/TooltipWrapper.tsx','tooltip/src/BasicTooltip.tsx','tooltip/LICENSE.md','tooltip/package.json'];
const originals={},sources={},asts={},extracts={};
for(const name of names){const b=await readFile(resolve(checkout,'packages',name));assert(b.equals(execFileSync('git',['-C',checkout,'show',`${commit}:packages/${name}`])));originals[name]=b;sources[name]=sha(b);if(/\.tsx?$/.test(name))asts[name]=babelParse(b.toString(),'upstream-'+name,true);}
assert.equal(JSON.parse(originals['funnel/package.json']).version,'0.99.0');
for(const name of ['funnel/LICENSE.md','tooltip/LICENSE.md'])assert.match(originals[name].toString(),/Permission is hereby granted/);
function variable(file,name){let found;traverse(asts[file],{VariableDeclarator(path){if(path.node.id.name===name){assert(!found);found=path.node.init;}}});assert(found,file+':'+name);const text=originals[file].toString().slice(found.start,found.end);extracts[file+':'+name]={file,start:found.start,end:found.end,startLine:found.loc.start.line,endLine:found.loc.end.line,typescript:text,sha256:sha(text)};return text;}
function evaluate(file,name,deps={}){const js=stripTypeScriptTypes('('+variable(file,name)+')');extracts[file+':'+name].javascript=js;return new Function(...Object.keys(deps),'"use strict";return '+js)(...Object.values(deps));}
const Fragment=Symbol('Fragment'),jsx=(type,props,key)=>({type,props,key:key??null}),runtime={jsx,jsxs:jsx,Fragment};
function tsx(file,name,deps){const js=babelTransform('const '+name+' = '+variable(file,name)+';','original-'+name+'.tsx',false,[],[]).code;extracts[file+':'+name].javascript=js;return new Function('require',...Object.keys(deps),js+'\nreturn '+name)(value=>{assert(value.replaceAll('\\','/').endsWith('/playwright/jsx-runtime'));return runtime;},...Object.values(deps));}
const encode=value=>JSON.parse(JSON.stringify(value,(_k,v)=>typeof v==='function'?{$function:v.name||'anonymous'}:typeof v==='number'&&Object.is(v,-0)?{$number:'-0'}:typeof v==='symbol'?{$symbol:v.description}:v));
const inherited=resolve(root,'artifacts/verification/funnel-svg-layer-reference-third'),priorBytes=await readFile(resolve(inherited,'reference.json')),prior=JSON.parse(priorBytes);
assert(prior.passed&&prior.commit===commit);
const geometryBytes=await readFile(resolve(inherited,'outputs.json'));assert.equal(sha(geometryBytes),prior.artifacts['outputs.json'].sha256);
const geometry=JSON.parse(geometryBytes,(_k,v)=>v&&v.$number==='-0'?-0:v).shapeOutputs.find(p=>p.direction==='vertical'&&p.interpolation==='smooth'&&p.current===null);
assert(geometry&&geometry.parts.length===4);const parts=geometry.parts;
const generators=evaluate('funnel/src/hooks.ts','computeShapeGenerators',{area,line,curveBasis,curveLinear})('smooth','vertical');
for(const[i,p]of parts.entries()){assert.equal(generators[0](p.areaPoints),geometry.paths[i].body);assert.equal(generators[1](p.borderPoints),geometry.paths[i].border);}
const BasicTooltip='BasicTooltip',PartTooltip=tsx('funnel/src/PartTooltip.tsx','PartTooltip',{BasicTooltip});
const hidden=evaluate('tooltip/src/context.ts','hiddenTooltipState');
const useMemo=f=>f(),useCallback=f=>f;
const profiles=[],coordinates=[],exceptions=[];let seq=0;
function tooltipMachine(bounds={left:30,top:40,width:400,height:200},offsetWidth=400,trace=[]){let state=hidden;const useTooltipHandlers=evaluate('tooltip/src/hooks.ts','useTooltipHandlers',{useState:initial=>[initial,value=>{state=value;trace.push({op:'tooltip-state',visible:value.isVisible,anchor:value.anchor,position:value.position});}],useCallback,useMemo,hiddenTooltipState:hidden});const result=useTooltipHandlers({current:{getBoundingClientRect:()=>bounds,offsetWidth}});assert.equal(result.state,hidden);return{actions:result.actions,get:()=>state};}
for(const interactive of [false,true])for(const click of [false,true])for(const custom of [false,true]){
 const trace=[];let current=null;const machine=tooltipMachine(undefined,undefined,trace),customTooltip=({part})=>jsx('custom-tooltip',{part}),tooltip=custom?customTooltip:PartTooltip;
 const createElement=(type,props)=>{assert.equal(type,tooltip);assert(parts.includes(props.part));trace.push({op:'create-tooltip',part:parts.indexOf(props.part)});return jsx(type,props);};
 const handlers=evaluate('funnel/src/hooks.ts','computePartsHandlers',{createElement,PartTooltip});
 let activeEvent,activePart;const user=kind=>(part,event)=>{assert.equal(part,activePart);assert.equal(part.data,activePart.data);assert.equal(event,activeEvent);trace.push({op:kind,part:parts.indexOf(part),sequence:event.sequence,current,tipVisible:machine.get().isVisible});};
 const bound=handlers({parts,isInteractive:interactive,setCurrentPartId:id=>{current=id;trace.push({op:'current',id});},...machine.actions,tooltip:custom?customTooltip:undefined,onMouseEnter:user('enter'),onMouseMove:user('move'),onMouseLeave:user('leave'),onClick:click?user('click'):undefined});
 assert.equal(bound===parts,!interactive);
 for(const[i,part]of bound.entries()){
  if(!interactive){assert.equal(part,parts[i]);for(const name of ['onMouseEnter','onMouseMove','onMouseLeave','onClick'])assert.equal(part[name],undefined);continue;}
  assert.notEqual(part,parts[i]);assert.equal(part.data,parts[i].data);assert.equal(Boolean(part.onClick),click);
  const ActualPart=tsx('funnel/src/Part.tsx','Part',{useAnimatedPath:p=>p,useMotionConfig:()=>({animate:false,config:{}}),useSpring:p=>p,animated:{path:'animated.path'}});
  const nodes=ActualPart({part,areaGenerator:generators[0],borderGenerator:generators[1]}).props.children;
  assert.equal(nodes[0].props.onMouseEnter,undefined);const body=nodes[1];assert.equal(body.props.onMouseEnter,part.onMouseEnter);assert.equal(body.props.onMouseMove,part.onMouseMove);assert.equal(body.props.onMouseLeave,part.onMouseLeave);assert.equal(body.props.onClick,part.onClick);
  activePart=parts[i];activeEvent=Object.freeze({type:'authored-mouse',sequence:++seq,clientX:123,clientY:156});
  let start=trace.length;body.props.onMouseEnter(activeEvent);assert.deepEqual(trace.slice(start).map(v=>v.op),['current','create-tooltip','tooltip-state','enter']);assert.equal(current,parts[i].data.id);assert.equal(machine.get().content.props.part,parts[i]);
  const rendered=machine.get().content.type(machine.get().content.props);if(!custom){assert.equal(rendered.type,BasicTooltip);assert.deepEqual(rendered.props,{id:parts[i].data.label,value:parts[i].formattedValue,color:parts[i].color,enableChip:true});}
  current='manual-current';start=trace.length;body.props.onMouseMove(activeEvent);assert.deepEqual(trace.slice(start).map(v=>v.op),['create-tooltip','tooltip-state','move']);assert.equal(current,'manual-current');
  if(click){start=trace.length;body.props.onClick(activeEvent);assert.deepEqual(trace.slice(start).map(v=>v.op),['click']);assert.equal(current,'manual-current');}
  start=trace.length;body.props.onMouseLeave(activeEvent);assert.deepEqual(trace.slice(start).map(v=>v.op),['current','tooltip-state','leave']);assert.equal(current,null);assert.equal(machine.get(),hidden);
 }
 profiles.push(encode({interactive,click,custom,trace,originalPartIdentity:true,bodyOnly:true,parts:parts.map(p=>({id:p.data.id,rawValue:p.data.value,formatted:p.formattedValue}))}));
}
// No application handler is needed for automatic current and tooltip actions.
{
 const trace=[],machine=tooltipMachine(undefined,undefined,trace);let current=null;
 const handlers=evaluate('funnel/src/hooks.ts','computePartsHandlers',{createElement:(type,props)=>jsx(type,props),PartTooltip});
 const bound=handlers({parts,isInteractive:true,setCurrentPartId:id=>{current=id;},...machine.actions});
 const event={clientX:50,clientY:80};bound[0].onMouseEnter(event);assert.equal(current,parts[0].data.id);assert(machine.get().isVisible);assert.equal(bound[0].onClick,undefined);bound[0].onMouseLeave(event);assert.equal(current,null);assert.equal(machine.get(),hidden);
 profiles.push({name:'automatic-without-user-handlers',trace});
}
// The old closure's Leave never changes its payload to a freshly bound object.
{
 const trace=[],machine=tooltipMachine(undefined,undefined,trace);let delivered;
 const handlers=evaluate('funnel/src/hooks.ts','computePartsHandlers',{createElement:(type,props)=>jsx(type,props),PartTooltip});
 const config={isInteractive:true,setCurrentPartId:()=>{},...machine.actions,onMouseLeave:part=>{delivered=part;}};
 const old=handlers({parts,...config}),replacement=parts.map(p=>({...p,x:p.x+100,data:{...p.data,label:'new raw-only label'}}));
 handlers({parts:replacement,...config});old[0].onMouseLeave({clientX:1,clientY:2});assert.equal(delivered,parts[0]);assert.notEqual(delivered,replacement[0]);assert.equal(delivered.data,parts[0].data);
 // Repeating a builtin uses the same original bound function and captured part.
 const repeated=[old,old];assert.equal(repeated[0][0].onMouseLeave,repeated[1][0].onMouseLeave);
 profiles.push({name:'historical-leave-and-repeated-handler-identity',oldPart:encode(delivered),replacement:encode(replacement[0]),trace});
}
for(const kind of ['Enter','Move','Leave','Click']){
 const trace=[],machine=tooltipMachine(undefined,undefined,trace);
 const handlers=evaluate('funnel/src/hooks.ts','computePartsHandlers',{createElement:(type,props)=>{trace.push({op:'create-tooltip'});return jsx(type,props);},PartTooltip});
 const bound=handlers({parts,isInteractive:true,setCurrentPartId:id=>trace.push({op:'current',id}),...machine.actions,['onMouse'+kind]:()=>{trace.push({op:'throw'});throw Error('authored callback rejection');},...(kind==='Click'?{onClick:()=>{trace.push({op:'throw'});throw Error('authored callback rejection');}}:{})});
 const name=kind==='Click'?'onClick':'onMouse'+kind;assert.throws(()=>bound[0][name]({clientX:80,clientY:90}),/authored callback rejection/);
 assert.deepEqual(trace.map(v=>v.op),kind==='Enter'?['current','create-tooltip','tooltip-state','throw']:kind==='Move'?['create-tooltip','tooltip-state','throw']:kind==='Leave'?['current','tooltip-state','throw']:['throw']);exceptions.push({kind,trace});
}
// Actual tooltip action source: default, flips, CSS width scaling and touches helper.
for(const offsetWidth of [400,200])for(const anchor of [undefined,'top','left','right','bottom','center'])for(const x of [60,330])for(const touch of [false,true]){
 const bounds={left:30,top:40,width:400,height:200},machine=tooltipMachine(bounds,offsetWidth),input=touch?{touches:[{clientX:x,clientY:110}]}:{clientX:x,clientY:110},content=jsx('content',{});machine.actions.showTooltipFromEvent(content,input,anchor);
 const local=[(x-30)*offsetWidth/400,70*offsetWidth/400],resolved=anchor==='left'||anchor==='right'?(local[0]<200?'right':'left'):anchor??'top';assert.deepEqual(machine.get().position,local);assert.equal(machine.get().anchor,resolved);assert.equal(machine.get().content,content);
 coordinates.push({offsetWidth,anchor:anchor??'default',touch,client:[x,110],state:encode(machine.get())});machine.actions.hideTooltip();assert.equal(machine.get(),hidden);
}
const tooltipStyle=evaluate('tooltip/src/TooltipWrapper.tsx','tooltipStyle'),translate=evaluate('tooltip/src/TooltipWrapper.tsx','translate');
const wrapper=tsx('tooltip/src/TooltipWrapper.tsx','TooltipWrapper',{memo:f=>f,useTheme:()=>({tooltip:{}}),useMotionConfig:()=>({animate:false,config:{}}),useMeasure:()=>[()=>{},{width:80,height:30}],useRef:value=>({current:value}),useSpring:value=>value.to??{},animated:{div:'animated.div'},tooltipStyle,translate,TOOLTIP_OFFSET:14});
const natural=[];for(const anchor of ['top','right','bottom','left','center']){const node=wrapper({position:[100.4,90.6],anchor,children:'actual-content'});natural.push({anchor,style:encode(node.props.style)});assert.equal(node.props.style.pointerEvents,'none');assert.equal(node.props.style.opacity,1);}
assert.deepEqual(natural.map(v=>v.style.transform),['translate(60px, 47px)','translate(114px, 76px)','translate(60px, 105px)','translate(6px, 76px)','translate(60px, 76px)']);
const defaultNoLabel=PartTooltip({part:{...parts[0],data:{...parts[0].data,label:undefined}}});assert.equal(defaultNoLabel.props.id,undefined,'upstream does not invent ID fallback');
for(const name of names)assert.equal(sha(await readFile(resolve(checkout,'packages',name))),sources[name]);
await mkdir(out,{recursive:false});for(const[name,b]of Object.entries(originals)){const path=resolve(out,'original',name);await mkdir(dirname(path),{recursive:true});await writeFile(path,b);}
const artifacts={};for(const[name,value]of Object.entries({'inputs.json':{parts:encode(parts),geometryInheritedFrom:'funnel-svg-layer-reference-third/outputs.json'},'outputs.json':{profiles,exceptions,coordinates,natural},'extracts.json':extracts})){const b=JSON.stringify(encode(value),null,2)+'\n';await writeFile(resolve(out,name),b);artifacts[name]=sha(b);}
const result={passed:true,featureAccepted:false,broadGoalComplete:false,commit,package:'funnel@0.99.0',sources,artifacts,producerSha256:sha(await readFile(script)),compiler:{node:process.version,playwright:JSON.parse(await readFile(resolve(root,'node_modules/playwright/package.json'))).version,bundleSha256:sha(await readFile(resolve(root,'node_modules/playwright/lib/transform/babelBundleImpl.js')))},inherited:{referenceSha256:sha(priorBytes),outputsSha256:sha(geometryBytes),scope:'Full original computed parts from the accepted original-function reference; actual d3-shape generators independently rerun against all four inherited paths.'},checks:{handlerProfiles:profiles.length,exceptionProfiles:exceptions.length,tooltipCoordinateProfiles:coordinates.length,naturalAnchorProfiles:natural.length,completePartAndEventIdentity:true,enterMoveLeaveClickOrdering:true,bodyOnlyBindings:true,noMoveCurrentSetter:true,noClickAutomaticSetters:true,historicalLeaveIdentity:true,defaultNoLabelNoIdFallback:true},scope:{executed:'Original computePartsHandlers and useTooltipHandlers functions, Part/PartTooltip/TooltipWrapper cold render functions, actual D3 generators.',adapters:'useState records setters without React scheduling; useCallback/useMemo cold identity; JSX descriptors; immediate useSpring; authored measured tooltip size80x30; inherited original computed parts.',excluded:'No React reconciliation/commit/unmount, native browser delivery/trust/touch compatibility, Glory DOM/pixels/Scope lifecycle, real useMeasure or Spring animation. Repeated handler identity does not model React key coercion or native occurrence metadata.'}};
await writeFile(resolve(out,'reference.json'),JSON.stringify(result,null,2)+'\n');await writeFile(resolve(out,'executed-driver.mjs'),await readFile(script));console.log(JSON.stringify({passed:true,out,...result.checks}));
