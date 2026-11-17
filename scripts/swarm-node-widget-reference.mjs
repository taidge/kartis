// Original Nivo component recipes with explicit non-React dependency probes.
import assert from 'node:assert/strict';
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {createRequire,stripTypeScriptTypes} from 'node:module';
import {resolve,dirname} from 'node:path';
import {fileURLToPath} from 'node:url';
import {execFileSync} from 'node:child_process';
import {createHash} from 'node:crypto';
const root=resolve(fileURLToPath(new URL('..',import.meta.url))),frozen=process.argv[2]==='--frozen';
assert(process.argv[2],'usage: swarm-node-widget-reference.mjs <nivo-checkout> [output] | --frozen [fixtures]');
const checkout=frozen?undefined:resolve(process.argv[2]),out=resolve(process.argv[3]??resolve(root,'tests/fixtures/swarm-node-widgets'));
const sha=b=>createHash('sha256').update(b).digest('hex'),expected=frozen?JSON.parse(await readFile(resolve(out,'reference.json'),'utf8')):undefined;
const names=['Circles.tsx','CircleSvg.tsx','types.ts','hooks.ts','compute.ts','SwarmPlot.tsx','SwarmPlotCanvas.tsx','props.ts'];
const original={};for(const name of names)original['src/'+name]=await readFile(frozen?resolve(out,'original/src',name):resolve(checkout,'packages/swarmplot/src',name));
original['package.json']=await readFile(frozen?resolve(out,'original/package.json'):resolve(checkout,'packages/swarmplot/package.json'));
const license=await readFile(frozen?resolve(out,'original/LICENSE.md'):resolve(checkout,'packages/swarmplot/LICENSE.md'));
assert.equal(JSON.parse(original['package.json']).version,'0.99.0');
const sources=Object.fromEntries(Object.entries(original).map(([name,b])=>[name,{sha256:sha(b),bytes:b.length}])),licenseInfo={file:'original/LICENSE.md',sha256:sha(license),bytes:license.length};
const require=createRequire(import.meta.url),{babelParse,babelTransform,traverse}=require(resolve(root,'node_modules/playwright/lib/transform/babelBundle.js'));
const producer={file:'scripts/swarm-node-widget-reference.mjs',sha256:sha(await readFile(fileURLToPath(import.meta.url)))},compiler={package:'playwright',version:JSON.parse(await readFile(resolve(root,'node_modules/playwright/package.json'),'utf8')).version,bundleSha256:sha(await readFile(resolve(root,'node_modules/playwright/lib/transform/babelBundleImpl.js')))};
if(frozen){assert.deepEqual(sources,expected.sources,'complete original source hashes');assert.deepEqual(licenseInfo,expected.license,'complete MIT notice');assert.deepEqual(producer,expected.producer,'producer bytes');assert.deepEqual(compiler,expected.compiler,'pinned TSX compiler');assert.match(expected.commit,/^[a-f0-9]{40}$/);}
const execution={},source=name=>original['src/'+name].toString('utf8');
function variable(file,name){let found;const s=source(file);traverse(babelParse(s,file,true),{VariableDeclarator(p){if(p.node.id.name===name){assert.equal(found,undefined);found=p.node;}}});assert(found,file+':'+name);const text='const '+s.slice(found.start,found.end)+';';execution[file+':'+name]=text;return text;}
function evaluate(file,name,deps={}){return new Function(...Object.keys(deps),stripTypeScriptTypes(variable(file,name))+'\nreturn '+name+';')(...Object.values(deps));}
const jsx=(type,props,key)=>({type,props,key:key??null}),runtime={jsx,jsxs:jsx,Fragment:'Fragment'};
function tsx(file,name,deps){const code=babelTransform(variable(file,name),'swarm-widget-'+name+'.tsx',false,[],[]).code;return new Function('require',...Object.keys(deps),code+'\nreturn '+name+';')(name=>{assert(name.replaceAll('\\','/').endsWith('/playwright/jsx-runtime'));return runtime;},...Object.values(deps));}
const scalar=n=>({__scalar:n,to:fn=>fn(n)}),to=(values,fn)=>fn(...values.map(v=>v?.__scalar??v)),useMemo=fn=>fn();
const canonical=value=>JSON.parse(JSON.stringify(value,function(k,v){if(this[k] instanceof Date)return{$date:this[k].toISOString()};if(typeof v==='function')return{$function:true};if(v&&typeof v==='object'&&Object.hasOwn(v,'__scalar'))return{$probeScalar:v.__scalar};if(typeof v==='number'&&Object.is(v,-0))return{$number:'-0'};return v;}));
const input=[{id:'raw-zero',group:'left',value:-0,size:0,color:'#00ff0000',border:0,nested:{tag:'shared'}},{id:'positive',group:'right',value:3,size:12,color:'#e8c1a0',border:2,nested:{tag:'second'}},{id:'outside',group:'left',value:-2,size:8,color:'#123456',border:1,nested:{tag:'third'}},{id:'date',group:'right',value:new Date('2024-01-01T00:00:00Z'),size:6,color:'#abcdef',border:0,nested:{tag:'fourth'}}];
const prepared=new Function('data','getId','getGroup','parseValue','getValue','getSize',stripTypeScriptTypes(variable('compute.ts','simulatedNodes'))+'\nreturn simulatedNodes;')(input,d=>d.id,d=>d.group,v=>v,d=>d.value,d=>d.size);
for(let i=0;i<input.length;i++){assert.notEqual(prepared[i].data,input[i]);assert.equal(prepared[i].data.nested,input[i].nested);assert.equal(prepared[i].value,input[i].value);}
// The simulation stage is deliberately not run; positions/index are authored.
const positions=[[20,30],[100,60],[700,-25],[160,90]],nodes=prepared.map((n,index)=>({...n,index,x:positions[index][0],y:positions[index][1]}));
const augmented=evaluate('hooks.ts','augmentedNodes',{useMemo,nodes,formatValue:v=>'format-probe:'+(v instanceof Date?v.toISOString():String(v)),getColor:n=>n.data.color});
const getTransitionPhases=evaluate('Circles.tsx','getTransitionPhases'),interpolate=evaluate('Circles.tsx','interpolateRadius',{to});
assert.equal(interpolate(scalar(-2)),0);assert.equal(interpolate(scalar(0)),0);assert.equal(interpolate(scalar(3)),3);
const useBorderWidth=evaluate('hooks.ts','useBorderWidth',{useMemo}),events=[],creation=[];
const createElement=(type,configuration)=>{const call={type,configuration};creation.push(call);return call;};
const useTooltip=()=>({showTooltipFromEvent:(content,event)=>events.push(['show',content,event.type]),hideTooltip:()=>events.push(['hide'])});
let pose='update',transitionOptions=[];
const useTransition=(selected,options)=>{transitionOptions.push({keys:selected.map(options.keys),immediate:options.immediate,config:options.config});return callback=>selected.map(node=>callback(Object.fromEntries(Object.entries(options[pose](node)).map(([k,v])=>[k,typeof v==='number'?scalar(v):v])),node));};
let animate=false;
const circles=tsx('Circles.tsx','Circles',{React:{createElement},createElement,useMemo,useTooltip,useMotionConfig:()=>({animate,config:{mass:1}}),useTheme:()=>({}),useBorderWidth,useInheritedColor:()=>()=> '#112233',useTransition,getTransitionPhases,interpolateRadius:interpolate});
const circleSvg=tsx('CircleSvg.tsx','CircleSvg',{animated:{circle:'NativeCircle'}});
const handlers=['onMouseEnter','onMouseMove','onMouseLeave','onMouseDown','onMouseUp','onClick','onDoubleClick'],cases=[];
for(const animation of[false,true])for(const interactive of[false,true])for(const phase of['initial','from','enter','update','leave']){
 animate=animation;pose=phase;creation.length=0;events.length=0;transitionOptions=[];
 const consumers=Object.fromEntries(handlers.map(name=>[name,(node,event)=>{assert(augmented.includes(node));events.push([name,node.id,event.type]);}]));
 const result=circles({nodes:augmented,borderWidth:n=>n.data.border,borderColor:{from:'color'},component:'CustomCircle',isInteractive:interactive,tooltip:'Tooltip',...consumers});
 const calls=creation.filter(c=>c.type==='CustomCircle');assert.equal(calls.length,input.length);assert(calls.some(c=>c.configuration.node.size===0));assert(calls.some(c=>c.configuration.node.x>600));
 const defaults=[];for(const call of calls){const c=call.configuration;assert.equal(c.key,c.node.id);for(const name of handlers)assert.equal(typeof c[name],interactive?'function':'undefined');const element=circleSvg(c);assert.equal(element.type,'NativeCircle');assert.equal(element.props.r,phase==='from'||phase==='leave'?0:c.node.size/2);assert.equal(element.props.opacity.__scalar,phase==='from'||phase==='leave'?0:1);for(const name of handlers)element.props[name]({type:name});defaults.push(element);}
 cases.push(canonical({animate:animation,interactive,phase,calls,defaultCircleElements:defaults,transitionOptions,result,events}));
}
let gate;const parent=source('SwarmPlot.tsx');traverse(babelParse(parent,'SwarmPlot.tsx',true),{IfStatement(p){const text=parent.slice(p.node.test.start,p.node.test.end);if(text==="layers.includes('circles')")gate=text;}});assert(gate);execution['SwarmPlot.tsx:circlesGate']=gate;
const gateFn=new Function('layers','return '+gate),gateCases=[{layers:[],calls:gateFn([])},{layers:['axes'],calls:gateFn(['axes'])},{layers:['circles'],calls:gateFn(['circles'])}];
const canvasRenderer=evaluate('SwarmPlotCanvas.tsx','renderCircleDefault'),canvasCases=[];
for(const alpha of[0,1])for(const node of augmented){const operations=[],paint={globalAlpha:alpha,lineCap:'square',lineWidth:4,fillStyle:'#000',strokeStyle:'#000'};const ctx=new Proxy(paint,{get(t,k){if(['beginPath','arc','fill','stroke'].includes(k))return(...args)=>operations.push([k,...args,...(['fill','stroke'].includes(k)?[{...paint}]:[])]);return t[k];},set(t,k,v){operations.push(['set',k,v]);t[k]=v;return true;}});canvasRenderer(ctx,{node,getBorderWidth:n=>n.data.border,getBorderColor:()=> '#112233'});assert.equal(paint.globalAlpha,alpha);assert.equal(paint.lineCap,'square');canvasCases.push(canonical({node,alpha,operations,finalPaint:paint}));}
const reference=canonical({scope:'Original Swarm computed-node preparation/augmentation, full Circles creation/transition/handler and default CircleSvg JSX recipes, plus record-only default Canvas drawing commands. Positions/index authored; hook/accessor/color/scalar probes explicit. No React lifecycle, actual spring, force simulation, browser event transport or framebuffer/pixel equivalence claim.',packageVersion:'0.99.0',commit:frozen?expected.commit:execFileSync('git',['-C',checkout,'rev-parse','HEAD'],{encoding:'utf8'}).trim(),sources,license:licenseInfo,producer,compiler,execution,input,prepared,augmented,cases,gateCases,canvasCases,negativeRadiusInterpolation:interpolate(scalar(-2)),boundaries:{raw:'Original computeNodes shallow-clones raw datum; original hooks augment formattedValue/color without filtering. Numeric or Date value, not nullable prop.',eligibility:'No size0/alpha0/off-viewport/isInteractive filter in component creation; layer gate is separate. Rust actual-site hidden/cull/force-coordinate contracts are distinct.',identity:'Original transition key=node.id and simulation index; Rust globally unique source/metadata indices remain explicit.',motion:'initial/enter/update regular; from/leave radius0 opacity0. style.radius clamps animation undershoot to0.',canvas:'Recorded method calls execute original function on a state-recording proxy, not a browser Canvas. Real Canvas acceptance remains separate.'}});
if(frozen)assert.deepEqual(reference,expected,'exact original Swarm component recipe execution');else{for(const[name,b]of Object.entries(original)){const p=resolve(out,'original',name);await mkdir(dirname(p),{recursive:true});await writeFile(p,b);}await writeFile(resolve(out,'original/LICENSE.md'),license);await writeFile(resolve(out,'reference.json'),JSON.stringify(reference,null,2)+'\n');}
console.log(JSON.stringify({mode:frozen?'frozen-verify':'produce',sources:Object.keys(original).length,licenses:1,profiles:cases.length,creationCalls:cases.reduce((s,c)=>s+c.calls.length,0),canvasCases:canvasCases.length,producerSha256:producer.sha256}));
