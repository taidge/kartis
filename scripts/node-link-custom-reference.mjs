// Verify-only original Nivo 0.99.0 function/body execution, not a reimplementation.
// A source import is optional: --source <Nivo checkout>. Normal --check uses the
// frozen complete upstream files and MIT notice, with no local checkout needed.
import assert from 'node:assert/strict';
import {readFile,writeFile,mkdir,copyFile} from 'node:fs/promises';
import {join,dirname,resolve} from 'node:path';
import {stripTypeScriptTypes} from 'node:module';
import {execFileSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {scaleOrdinal} from 'd3-scale';
import {chromium} from 'playwright';

const root=resolve('.'),directory=join(root,'tests/fixtures/node-link-custom'),vendor=join(directory,'vendor/nivo'),args=process.argv.slice(2),check=args.includes('--check');
const sources=['network/package.json','network/src/types.ts','network/src/defaults.ts','network/src/hooks.ts','network/src/renderCanvasLink.ts','network/src/NetworkLink.tsx','network/src/NetworkLinks.tsx','network/src/NetworkCanvas.tsx','swarmplot/package.json','swarmplot/src/types.ts','swarmplot/src/props.ts','swarmplot/src/compute.ts','swarmplot/src/hooks.ts','swarmplot/src/Circles.tsx','swarmplot/src/CircleSvg.tsx','swarmplot/src/SwarmPlotCanvas.tsx'];
const sha=b=>createHash('sha256').update(b).digest('hex');
const sourceArg=args.indexOf('--source');
if(sourceArg>=0){assert(!check,'--check cannot refresh upstream sources');const sourceRoot=resolve(args[sourceArg+1]);for(const name of sources){const dst=join(vendor,'packages',name);await mkdir(dirname(dst),{recursive:true});await copyFile(join(sourceRoot,'packages',name),dst);}await copyFile(join(sourceRoot,'LICENSE.md'),join(vendor,'LICENSE.md'));const commit=execFileSync('git',['-C',sourceRoot,'rev-parse','HEAD'],{encoding:'utf8',windowsHide:true}).trim();await writeFile(join(vendor,'source.json'),JSON.stringify({repository:'https://github.com/plouc/nivo',commit,files:sources},null,2)+'\n');}
const sourceInfo=JSON.parse(await readFile(join(vendor,'source.json'),'utf8')),full={},sourceSHA256={};
for(const name of sources){const bytes=await readFile(join(vendor,'packages',name));full[name]=bytes.toString('utf8').replaceAll('\r\n','\n');sourceSHA256[`packages/${name}`]=sha(bytes);}
for(const packageName of ['network','swarmplot'])assert.equal(JSON.parse(full[`${packageName}/package.json`]).version,'0.99.0');
const license=await readFile(join(vendor,'LICENSE.md'),'utf8');
function between(source,start,end){const a=source.indexOf(start),b=source.indexOf(end,a+start.length);assert(a>=0&&b>a,`Original source markers: ${start} / ${end}`);return source.slice(a,b);}
function balanced(source,start,open,close){const a=source.indexOf(start);assert(a>=0,start);const begin=source.indexOf(open,a);let level=0,quote=null,escaped=false;for(let i=begin;i<source.length;i++){const c=source[i];if(quote){if(escaped)escaped=false;else if(c==='\\')escaped=true;else if(c===quote)quote=null;continue;}if(c==='"'||c==="'"||c==='`'){quote=c;continue;}if(c===open)level++;else if(c===close&&!--level)return source.slice(a,i+1);}throw Error('Unclosed original source '+start);}
const compile=source=>stripTypeScriptTypes(source.replace(/\bexport (?=const|function)/g,''),{mode:'strip'});
const linkRenderer=compile(full['network/src/renderCanvasLink.ts'].replace(/^import .*\r?\n/gm,''));
const swarmRenderer=compile(between(full['swarmplot/src/SwarmPlotCanvas.tsx'],'export const renderCircleDefault','type InnerSwarmCanvasPlotProps'));
const networkLoop=compile(balanced(full['network/src/NetworkCanvas.tsx'],'layers.forEach(', '(',')'));
const swarmLoop=compile(balanced(full['swarmplot/src/SwarmPlotCanvas.tsx'],'layers.forEach(', '(',')'));
const networkTransitions=compile(between(full['network/src/NetworkLinks.tsx'],'const getEnterTransition','export const NetworkLinks'));
const swarmTransitions=compile(between(full['swarmplot/src/Circles.tsx'],'const getTransitionPhases','export const Circles'));
const computedLinks=compile(`function originalComputedLinks(){${balanced(full['network/src/hooks.ts'],'return transientLinks.map(', '(',')')};}`);
const nativeComputeLinks=new Function('transientLinks','computedNodes','getLinkThickness','getLinkColor',`${computedLinks};return originalComputedLinks();`);
const originalOrdinal=compile(between(full['swarmplot/src/compute.ts'],'export const computeOrdinalScale','export const computeValueScale'));
const nativeOrdinal=new Function('scaleOrdinal',`${originalOrdinal};return computeOrdinalScale;`)(scaleOrdinal);
const nodePreparation=compile(balanced(full['swarmplot/src/compute.ts'],'const simulatedNodes:', '(',')'));
const nativePrepareNodes=new Function('data','getId','getGroup','parseValue','getValue','getSize',`${nodePreparation};return simulatedNodes;`);
const nodes=[{id:'alpha',index:0,x:30,y:30,vx:1,vy:-2,size:12,color:'#e11450',borderWidth:2,borderColor:'#123456',data:{id:'alpha',label:'A',radius:6}},{id:'beta',index:1,x:100,y:60,vx:-3,vy:4,size:0,color:'#10bcee00',borderWidth:0,borderColor:'#abcdef',data:{id:'beta',label:'B',radius:0}},{id:'gamma',index:2,x:160,y:90,vx:0,vy:0,size:18,color:'#4e79a7',borderWidth:1.5,borderColor:'#010203',data:{id:'gamma',label:'C',radius:9}}];
const rawLinks=[{source:'alpha',target:'beta',value:4,thickness:2,color:'#e1e5eb'},{source:'alpha',target:'beta',value:9,thickness:0,color:'#e1145000'},{source:'gamma',target:'gamma',value:1,thickness:1,color:'#4e79a7'},{source:'beta',target:'alpha',value:144,thickness:12,color:'#010203'}];
const transientLinks=rawLinks.map((data,index)=>({index,data,source:{id:data.source},target:{id:data.target}}));
const links=nativeComputeLinks(transientLinks,nodes,l=>l.data.thickness,l=>l.data.color);assert.equal(links.length,4);assert.equal(links[0].id,links[1].id,'Original Nivo concat IDs collide for parallel links; retain the indices as evidence');
const rawSwarm=[{id:'first',group:'left',value:0,size:0,color:'#e1145000',borderWidth:0,borderColor:'#123456'},{id:'second',group:'right',value:7,size:12,color:'#4e79a7',borderWidth:2.5,borderColor:'#010203'},{id:'third',group:'left',value:-3,size:18,color:'#f28e2b',borderWidth:0,borderColor:'#abcdef'}];
const prepared=nativePrepareNodes(rawSwarm,d=>d.id,d=>d.group,x=>x,d=>d.value,d=>d.size);
assert(prepared.every((d,i)=>d.data!==rawSwarm[i]&&JSON.stringify(d.data)===JSON.stringify(rawSwarm[i])),'Original source performs a shallow clone');
const swarmNodes=prepared.map((n,index)=>({...n,index,x:[30,100,160][index],y:[30,60,90][index],formattedValue:String(n.value),color:n.data.color}));
const networkCases=[];for(const lineCap of ['butt','round','square'])for(const globalAlpha of [0,1])networkCases.push({name:`links-${lineCap}-alpha${globalAlpha}`,nodes,links,layers:['links','nodes'],seed:{lineCap,globalAlpha,lineWidth:3},throwAt:null});
networkCases.push({name:'links-repeat-layer-and-order',nodes,links:[links[3],links[0],links[2],links[1]],layers:['nodes','links','links'],seed:{lineCap:'square',globalAlpha:.35,lineWidth:3},throwAt:null});
networkCases.push({name:'links-no-layer',nodes,links,layers:['nodes'],seed:{lineCap:'butt',globalAlpha:1,lineWidth:1},throwAt:null});
networkCases.push({name:'links-first-failure-stops',nodes,links,layers:['links'],seed:{lineCap:'round',globalAlpha:1,lineWidth:3},throwAt:1});
networkCases.push({name:'links-empty',nodes:[],links:[],layers:['links','nodes'],seed:{lineCap:'butt',globalAlpha:1,lineWidth:1},throwAt:null});
const swarmCases=[];for(const globalAlpha of [0,1])swarmCases.push({name:`nodes-alpha${globalAlpha}`,nodes:swarmNodes,layers:['circles'],seed:{lineCap:'butt',globalAlpha,lineWidth:3},throwAt:null});
swarmCases.push({name:'nodes-reordered-repeat-layer',nodes:[swarmNodes[2],swarmNodes[0],swarmNodes[1]],layers:['circles','circles'],seed:{lineCap:'square',globalAlpha:.35,lineWidth:3},throwAt:null});
swarmCases.push({name:'nodes-no-layer',nodes:swarmNodes,layers:[],seed:{lineCap:'butt',globalAlpha:1,lineWidth:3},throwAt:null});
swarmCases.push({name:'nodes-first-failure-stops',nodes:swarmNodes,layers:['circles'],seed:{lineCap:'round',globalAlpha:1,lineWidth:3},throwAt:1});
swarmCases.push({name:'nodes-empty',nodes:[],layers:['circles'],seed:{lineCap:'butt',globalAlpha:1,lineWidth:3},throwAt:null});
const browser=await chromium.launch({headless:true});let actual;
try{const page=await browser.newPage();actual=await page.evaluate(({sources,networkCases,swarmCases})=>{
    const renderLink=new Function(`${sources.linkRenderer};return renderCanvasLink;`)(),renderCircle=new Function(`${sources.swarmRenderer};return renderCircleDefault;`)();
    const dispatchLinks=new Function('ctx','layers','links','nodes','renderLink','renderNode',sources.networkLoop),dispatchNodes=new Function('ctx','layers','nodes','renderCircle','getBorderWidth','getBorderColor',sources.swarmLoop);
    const transitions=new Function(`${sources.networkTransitions};return {enter:getEnterTransition(),update:getRegularTransition()};`)(),swarmTransitions=new Function(`${sources.swarmTransitions};return getTransitionPhases;`)();
    function run(kind,c){const canvas=document.createElement('canvas');canvas.width=220;canvas.height=160;const ctx=canvas.getContext('2d');Object.assign(ctx,c.seed);const events=[],calls=[],otherCalls=[];const before=JSON.stringify(c);let thrown=null;
        const paint=()=>({lineWidth:ctx.lineWidth,strokeStyle:ctx.strokeStyle,fillStyle:ctx.fillStyle,lineCap:ctx.lineCap,globalAlpha:ctx.globalAlpha});
        const forwarded=new Proxy(ctx,{set(target,key,value){target[key]=value;events.push(['set',key,value,target[key]]);return true;},get(target,key){const value=Reflect.get(target,key,target);if(typeof value!=='function')return value;return(...args)=>{const result=value.apply(target,args);events.push([key,...args,...(['stroke','fill'].includes(key)?[paint()]:[])]);return result;};}});
        const customLink=(context,link)=>{const call={index:link.index,id:link.id,source:link.source.id,target:link.target.id,sourcePosition:[link.source.x,link.source.y],targetPosition:[link.target.x,link.target.y],data:link.data,thickness:link.thickness,color:link.color};calls.push(call);if(c.throwAt===calls.length-1)throw Error('explicit custom callback failure');renderLink(context,link);};
        const customNode=(context,props)=>{const {node,getBorderWidth,getBorderColor}=props;calls.push({index:node.index,id:node.id,group:node.group,value:node.value,formattedValue:node.formattedValue,position:[node.x,node.y],size:node.size,color:node.color,data:node.data,resolversAreFunctions:typeof getBorderWidth==='function'&&typeof getBorderColor==='function'});if(c.throwAt===calls.length-1)throw Error('explicit custom callback failure');renderCircle(context,props);};
        try{if(kind==='network')dispatchLinks(forwarded,c.layers,c.links,c.nodes,customLink,(_,node)=>otherCalls.push(node.id));else dispatchNodes(forwarded,c.layers,c.nodes,customNode,node=>node.data.borderWidth,node=>node.data.borderColor);}catch(e){thrown=String(e.message);}
        if(before!==JSON.stringify(c))throw Error('Original drawing mutated its source');
        const witnesses=(kind==='network'?c.nodes:c.nodes).map(n=>({id:n.id,position:[n.x,n.y],rgba:[...ctx.getImageData(n.x,n.y,1,1).data]}));
        const animation=kind==='network'?c.links.map(link=>({index:link.index,enter:transitions.enter(link),update:transitions.update(link)})):c.nodes.map(node=>{const phases=swarmTransitions(n=>n.data.borderColor);return{index:node.index,enter:phases.enter(node),update:phases.update(node),leave:phases.leave(node)};});
        return{name:c.name,calls,otherCalls,events,witnesses,animation,thrown,sourceUnchanged:true};
    }
    return{network:networkCases.map(c=>run('network',c)),swarm:swarmCases.map(c=>run('swarm',c))};
},{sources:{linkRenderer,swarmRenderer,networkLoop,swarmLoop,networkTransitions,swarmTransitions},networkCases,swarmCases});}finally{await browser.close();}
assert.equal(actual.network.find(c=>c.name==='links-no-layer').calls.length,0);assert.equal(actual.swarm.find(c=>c.name==='nodes-no-layer').calls.length,0);
for(const kind of ['network','swarm']){const fail=actual[kind].find(c=>c.name.endsWith('first-failure-stops'));assert.equal(fail.calls.length,2);assert.equal(fail.thrown,'explicit custom callback failure');}
const ordinalCases=[];for(const axis of ['x','y'])for(const groups of [['left'],['left','right'],['z','x','y']]){const input={width:220,height:160,axis,groups,gap:8},scale=nativeOrdinal(input);ordinalCases.push({input,domain:scale.domain(),range:scale.range(),mapped:groups.map(g=>scale(g))});}
const values=JSON.stringify({schema:1,nivoVersion:'0.99.0',scope:'Original pure default renderer bodies, original Canvas layer loops, original transition functions and original source-preparation/computed-link fragments execute without rewriting their algorithms. Canvas operations forward to actual Playwright Chromium Canvas2D, including requested/applied paint. This is not React hook/lifecycle equivalence or final force-layout position equivalence.',policy:{rustCompatibility:'Factory inherit preserves the existing Rust Scene, not Nivo defaults. Native linkThickness 0 is assigned and ignored by real Canvas2D, retaining its previous positive width; Rust need not reproduce that unchecked paint quirk. SVG NetworkLink declares round caps while original Canvas link leaves the inherited cap unchanged. Canvas default renderers do not assign globalAlpha; zero alpha still calls them.',callbackOrder:'Native Canvas walks provided computed arrays/layers without sorting or ink-based culling. Rust Swarm factory preserves its existing packed-site order; it does not claim original force simulation source order.',source:'Nivo Swarm source preparation shallow-copies raw data; computed-link fields use original raw index and computed source/target objects. Native concatenated parallel-link ids collide; Rust retains explicit relation ids or source indices.',failure:'Explicit callback throw stops remaining native iteration; its earlier Canvas operations remain. Rust checked layout/publication rejection must instead preserve the accepted whole frame.'},networkCases,swarmCases,native:actual,ordinalCases,preparedSwarm:prepared,computedLinks:links},null,2)+'\n';
const counts={networkCases:networkCases.length,swarmCases:swarmCases.length,networkCalls:actual.network.reduce((n,c)=>n+c.calls.length,0),swarmCalls:actual.swarm.reduce((n,c)=>n+c.calls.length,0),ordinalCases:ordinalCases.length,realCanvasWitnesses:[...actual.network,...actual.swarm].reduce((n,c)=>n+c.witnesses.length,0)};
const reference=JSON.stringify({schema:1,nivoVersion:'0.99.0',upstream:sourceInfo,license:{name:'MIT',file:'vendor/nivo/LICENSE.md',sha256:sha(license),text:license},sourceSHA256,executedBodies:{linkRendererSHA256:sha(linkRenderer),swarmRendererSHA256:sha(swarmRenderer),networkLoopSHA256:sha(networkLoop),swarmLoopSHA256:sha(swarmLoop),networkTransitionsSHA256:sha(networkTransitions),swarmTransitionsSHA256:sha(swarmTransitions),computedLinksSHA256:sha(computedLinks),nodePreparationSHA256:sha(nodePreparation),ordinalSHA256:sha(originalOrdinal)},producerSHA256:sha(await readFile(new URL(import.meta.url))),valuesSHA256:sha(values),runtime:{nodeMajor:24,playwrightVersion:JSON.parse(await readFile('node_modules/playwright/package.json','utf8')).version,canvas:'Actual Chromium CanvasRenderingContext2D; Proxy forwards all operations without replacing math or Canvas behavior'},counts},null,2)+'\n';
await mkdir(directory,{recursive:true});for(const[name,text]of [['values.json',values],['reference.json',reference]]){const file=join(directory,name);if(check)assert.equal(await readFile(file,'utf8'),text,name);else await writeFile(file,text);}
console.log(JSON.stringify({passed:true,checked:check,...counts,valuesSHA256:sha(values)}));
