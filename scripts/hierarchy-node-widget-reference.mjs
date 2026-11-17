// Actual upstream component recipes, with explicit non-React dependency probes.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {createRequire,stripTypeScriptTypes} from 'node:module';
import {resolve,dirname} from 'node:path';
import {fileURLToPath} from 'node:url';
import {execFileSync} from 'node:child_process';
const root=resolve(fileURLToPath(new URL('..',import.meta.url)));
if(!process.argv[2])throw Error('usage: hierarchy-node-widget-reference.mjs <nivo-checkout> [output] | --frozen [fixture-directory]');
const frozen=process.argv[2]==='--frozen',nivo=frozen?undefined:resolve(process.argv[2]);
const output=resolve(process.argv[3]??resolve(root,'tests/fixtures/hierarchy-node-widgets'));
const expected=frozen?JSON.parse(await readFile(resolve(output,'reference.json'),'utf8')):undefined;
const require=createRequire(import.meta.url),{babelParse,babelTransform,traverse}=require(resolve(root,'node_modules/playwright/lib/transform/babelBundle.js'));
const sha=b=>createHash('sha256').update(b).digest('hex');
const specs={treemap:['TreeMap.tsx','TreeMapHtml.tsx','TreeMapNodes.tsx','TreeMapNode.tsx','TreeMapHtmlNode.tsx','hooks.ts','types.ts','defaults.ts','transitions.ts'],
 'circle-packing':['CirclePacking.tsx','CirclePackingHtml.tsx','Circles.tsx','CircleSvg.tsx','CircleHtml.tsx','Labels.tsx','hooks.ts','types.ts','defaults.ts'],
 icicle:['Icicle.tsx','IcicleNodes.tsx','hooks.ts','types.ts','defaults.ts'],
 rects:['RectNodes.tsx','RectNodeWrapper.tsx','useRectsTransition.ts','RectNodeSvg.tsx','types.ts'],theming:['borderRadius.ts']};
const original={};for(const[family,names]of Object.entries(specs))for(const name of names){const key=family+'/'+name;original[key]=await readFile(frozen?resolve(output,'original',key):resolve(nivo,'packages',family,'src',name));}
const licenses=[];for(const family of Object.keys(specs)){const key='original/'+family+'/LICENSE.md',bytes=await readFile(frozen?resolve(output,key):resolve(nivo,'packages',family,'LICENSE.md'));licenses.push({family,file:key,sha256:sha(bytes),bytes:bytes.length,data:bytes});}
const sourceHashes=Object.fromEntries(Object.entries(original).map(([k,b])=>[k,{sha256:sha(b),bytes:b.length}]));
const licenseHashes=licenses.map(({data,...r})=>r),producer={file:'scripts/hierarchy-node-widget-reference.mjs',sha256:sha(await readFile(fileURLToPath(import.meta.url)))};
const compiler={package:'playwright',version:JSON.parse(await readFile(resolve(root,'node_modules/playwright/package.json'),'utf8')).version,bundleSha256:sha(await readFile(resolve(root,'node_modules/playwright/lib/transform/babelBundleImpl.js')))};
if(frozen){assert.deepEqual(sourceHashes,expected.originalSources,'original complete source hashes');assert.deepEqual(licenseHashes,expected.licenses,'complete MIT licenses');assert.deepEqual(producer,expected.producer,'producer SHA');assert.deepEqual(compiler,expected.compiler,'pinned existing TSX compiler');assert.deepEqual(Object.keys(expected.versions),Object.keys(specs));assert(Object.values(expected.versions).every(v=>v==='0.99.0'),'original package versions');assert.match(expected.commit,/^[0-9a-f]{40}$/);}
const extracts={};const source=k=>original[k].toString('utf8');
function variable(file,name){let found;const s=source(file);traverse(babelParse(s,file,true),{VariableDeclarator(path){if(path.node.id.name===name){assert.equal(found,undefined,file+':'+name);found=path.node;}}});assert.ok(found,file+':'+name);return{node:found,source:s};}
function decl(file,name){const v=variable(file,name),s='const '+v.source.slice(v.node.start,v.node.end)+';';extracts[file+':'+name]=s;return s;}
const exprBody=(s,invoke=false)=>stripTypeScriptTypes(`const __expression=(${s});`)+`\nreturn __expression${invoke?'()':''};`;
function evaluateDeclaration(file,name,bindings={}){return new Function(...Object.keys(bindings),stripTypeScriptTypes(decl(file,name))+'\nreturn '+name+';')(...Object.values(bindings));}
function hookBody(file,name){const v=variable(file,name),s=v.source.slice(v.node.init.arguments[0].start,v.node.init.arguments[0].end);extracts[file+':'+name+':memoBody']=s;return s;}
const canonical=v=>JSON.parse(JSON.stringify(v,(_k,n)=>typeof n==='function'?{$function:true}:n&&typeof n==='object'&&Object.hasOwn(n,'__scalar')?{$probeScalar:n.__scalar}:typeof n==='number'&&Object.is(n,-0)?{$number:'-0'}:n));
const plainTo=(values,fn)=>Array.isArray(values)?fn(...values.map(v=>v?.__scalar??v)):fn(values?.__scalar??values);
const scalar=value=>({__scalar:value,to:fn=>fn(value)});
const jsxCalls=[];const dispatch={};
function jsx(type,props,key){if(dispatch[type])return dispatch[type](props,props.ref);const out={type,key:key??null,props};jsxCalls.push(out);return out;}
const runtime={jsx,jsxs:jsx,Fragment:'Fragment'};
function tsx(file,name,bindings={}){
 const originalDecl=decl(file,name),code=babelTransform(originalDecl,'hierarchy-reference-'+name+'.tsx',false,[],[]).code;
 return new Function('require',...Object.keys(bindings),code+'\nreturn '+name+';')(name=>{assert(name.replaceAll('\\','/').endsWith('/playwright/jsx-runtime'),'only compiler JSX instrumentation is imported');return runtime;},...Object.values(bindings));
}
function callExpression(file,callee){const s=source(file);let found;traverse(babelParse(s,file,true),{CallExpression(path){if(path.node.callee.name===callee){assert.equal(found,undefined,file+':'+callee);found=path.node;}}});assert.ok(found);return{node:found,source:s};}
function transitionRecipe(file){const c=callExpression(file,'useTransition'),s=c.source.slice(c.node.arguments[1].start,c.node.arguments[1].end);extracts[file+':transitionOptions']=s;return s;}
function transitionCallback(file){const c=callExpression(file,'transition'),s=c.source.slice(c.node.arguments[0].start,c.node.arguments[0].end);extracts[file+':transitionCallback']=s;return s;}
const events=[];const tooltip={showTooltipFromEvent:()=>events.push('tooltip-event'),showTooltipAt:()=>events.push('tooltip-at'),hideTooltip:()=>events.push('tooltip-hide')};
const useMemo=f=>f(),useCallback=f=>f,useTooltip=()=>tooltip;
const theme={labels:{text:{fontSize:12}}};
const commonNode={formattedValue:'probe aggregate',color:'#123456',borderColor:'#112233',labelTextColor:'#223344',parentLabelTextColor:'#334455',labelRotation:0,parentLabelX:6,parentLabelY:10,parentLabelRotation:0,label:'leaf label',parentLabel:'parent label'};
const rawTree={id:'root',value:2,children:[{id:'a',value:0,children:[{id:'same',value:0}]},{id:'b',value:0,children:[{id:'same',value:5}]}]};
const nodes=[
 {...commonNode,id:'root',path:'root',pathComponents:['root'],data:{id:'root',value:2},value:7,treeDepth:0,treeHeight:2,isParent:true,isLeaf:false,x:0,y:0,width:120,height:80,opacity:.33},
 {...commonNode,id:'a',path:'root.a',pathComponents:['root','a'],data:{id:'a',value:0},value:0,treeDepth:1,treeHeight:1,isParent:true,isLeaf:false,x:0,y:0,width:0,height:40,opacity:.33},
 {...commonNode,id:'same',path:'root.a.same',pathComponents:['root','a','same'],data:{id:'same',value:0},value:0,treeDepth:2,treeHeight:0,isParent:false,isLeaf:true,x:0,y:0,width:0,height:0,opacity:0},
 {...commonNode,id:'b',path:'root.b',pathComponents:['root','b'],data:{id:'b',value:0},value:5,treeDepth:1,treeHeight:1,isParent:true,isLeaf:false,x:40,y:0,width:80,height:40,opacity:.33},
 {...commonNode,id:'same',path:'root.b.same',pathComponents:['root','b','same'],data:{id:'same',value:5},value:5,treeDepth:2,treeHeight:0,isParent:false,isLeaf:true,x:60,y:20,width:40,height:20,opacity:1,fill:'url(#probe-pattern)'},
].sort((a,b)=>a.treeDepth-b.treeDepth);
// Treemap: actual eligibility selector, handler hook, creation and default JSX.
let selector;const th=source('treemap/hooks.ts');traverse(babelParse(th,'hooks.ts',true),{ConditionalExpression(path){const s=th.slice(path.node.start,path.node.end);if(s==='leavesOnly ? root.leaves() : root.descendants()')selector=s;}});assert.ok(selector);extracts.treemapSelector=selector;
const select=new Function('root','leavesOnly',exprBody(selector));
const interactive=evaluateDeclaration('treemap/hooks.ts','useInteractiveTreeMapNodes',{useMemo,useCallback,useTooltip,createElement:(type,props)=>({type:'Tooltip',props})});
const regular=evaluateDeclaration('treemap/TreeMapNodes.tsx','getAnimatedNodeProps'),ending=evaluateDeclaration('treemap/TreeMapNodes.tsx','getEndingAnimatedNodeProps');
const tmOptions=new Function('getAnimatedNodeProps','getEndingAnimatedNodeProps','springConfig','animate',exprBody(transitionRecipe('treemap/TreeMapNodes.tsx')))(regular,ending,{mass:1},false);
assert.equal(tmOptions.initial,regular);assert.equal(tmOptions.from,ending);assert.equal(tmOptions.enter,regular);assert.equal(tmOptions.update,regular);assert.equal(tmOptions.leave,ending);
const svgNode=evaluateDeclaration('treemap/transitions.ts','svgNodeTransform',{to:plainTo}),svgLabel=evaluateDeclaration('treemap/transitions.ts','svgLabelTransform',{to:plainTo});
const treeMapDefault=tsx('treemap/TreeMapNode.tsx','NonMemoizedTreeMapNode',{useTheme:()=>theme,animated:{g:'g',rect:'rect'},to:plainTo,Text:'Text',svgNodeTransform:svgNode,svgLabelTransform:svgLabel});
const treemapCases=[];
for(const leavesOnly of[false,true])for(const isInteractive of[false,true])for(const labels of [{enableLabel:true,enableParentLabel:true,labelSkipSize:0},{enableLabel:true,enableParentLabel:true,labelSkipSize:20},{enableLabel:false,enableParentLabel:true,labelSkipSize:0},{enableLabel:true,enableParentLabel:false,labelSkipSize:0}]){
 const selected=select({leaves:()=>nodes.filter(n=>n.isLeaf),descendants:()=>nodes},leavesOnly);
 const bound=interactive(selected,{isInteractive,onClick:(node,event)=>{assert(selected.includes(node));events.push('treemap-click:'+node.path);},tooltip:'Tooltip'});
 for(let i=0;i<bound.length;i++){assert.equal(Boolean(bound[i].onClick),isInteractive);if(!isInteractive)assert.equal(bound[i],selected[i]);}
 const calls=[];const create=new Function('createElement','nodeComponent','borderWidth','enableLabel','labelSkipSize','enableParentLabel',exprBody(transitionCallback('treemap/TreeMapNodes.tsx')))((type,props)=>{assert.equal(type,'CustomTreeMap');calls.push(props);return props;},'CustomTreeMap',2,labels.enableLabel,labels.labelSkipSize,labels.enableParentLabel);
 const phases={};for(const phase of['initial','from','enter','update','leave'])phases[phase]=bound.map(node=>{assert.equal(tmOptions.keys(node),node.path);const animatedProps=tmOptions[phase](node),props=create(animatedProps,node);assert.equal(props.node,node);assert.equal(props.animatedProps,animatedProps);return props;});
 const defaults=phases.update.map(props=>treeMapDefault(props));
 for(let i=0;i<defaults.length;i++){
  const texts=defaults[i].props.children.filter(Boolean).filter(n=>n.type==='Text');
  const leaf=texts.find(n=>n.props['data-testid']==='label.'+bound[i].id),parent=texts.find(n=>n.props['data-testid']==='parentLabel.'+bound[i].id);
  assert.equal(Boolean(leaf),labels.enableLabel&&bound[i].isLeaf&&(labels.labelSkipSize===0||Math.min(bound[i].width,bound[i].height)>labels.labelSkipSize));
  assert.equal(Boolean(parent),labels.enableParentLabel&&bound[i].isParent);
  if(leaf)assert.equal(leaf.props.fillOpacity,1,'node opacity is not automatically leaf label opacity');
 }
 if(isInteractive)bound[0].onClick({type:'click-probe'});
 treemapCases.push({leavesOnly,isInteractive,labels,selectedPaths:selected.map(n=>n.path),phases,defaults});
}
// CirclePacking: actual complete Circles function, scalar projection probes.
const circleRadius=evaluateDeclaration('circle-packing/Circles.tsx','interpolateRadius',{to:plainTo});
const circlePhases=evaluateDeclaration('circle-packing/Circles.tsx','getTransitionPhases');
const circleRawTree={id:'root',value:2,children:[{id:'a',value:0,children:[{id:'zero',value:0}]},{id:'b',value:0,children:[{id:'value',value:5}]}]};
const circleRawByPath={root:circleRawTree,'root.a':circleRawTree.children[0],'root.a.same':circleRawTree.children[0].children[0],'root.b':circleRawTree.children[1],'root.b.same':circleRawTree.children[1].children[0]};
const circleRadii={root:60,'root.a':0,'root.a.same':0,'root.b':40,'root.b.same':20};
const circleNodes=nodes.map(n=>({id:circleRawByPath[n.path].id,path:[circleRawByPath[n.path].id,...n.pathComponents.slice(0,-1).reverse()],data:circleRawByPath[n.path],value:n.value,percentage:n.value/7*100,formattedValue:String(n.value),x:n.x+20,y:n.y+20,radius:circleRadii[n.path],depth:n.treeDepth,height:n.treeHeight,color:n.color}));
const cs=variable('circle-packing/hooks.ts','nodes');const circleSelector=cs.source.slice(cs.node.init.start,cs.node.init.end);extracts.circlePackingSelector=circleSelector;
const selectCircles=new Function('packedData','leavesOnly',exprBody(circleSelector));
const circles=tsx('circle-packing/Circles.tsx','Circles',{useMemo,useTooltip,useMotionConfig:()=>({animate:false,config:{mass:1}}),useTheme:()=>theme,useInheritedColor:()=>()=> '#112233',getTransitionPhases:circlePhases,interpolateRadius:circleRadius,React:{createElement:(type,props)=>({type,props})},useTransition:(selected,options)=>callback=>selected.map(node=>{assert.equal(options.keys(node),node.id);return callback(options.update(node),node);})});
const circleCases=[];for(const isInteractive of[false,true])for(const leavesOnly of[false,true]){const selected=selectCircles({leaves:()=>circleNodes.filter(n=>n.height===0),descendants:()=>circleNodes},leavesOnly);const result=circles({nodes:selected,borderWidth:2,borderColor:{from:'color'},component:'CustomCircle',isInteractive,onClick:(node,event)=>{assert(selected.includes(node));events.push('circle-click:'+node.id);},tooltip:'Tooltip'}).props.children;assert.equal(result.length,selected.length);for(const r of result)assert.equal(Boolean(r.props.onClick),isInteractive);if(isInteractive)result[0].props.onClick(result[0].props.node,{type:'click-probe'});circleCases.push({isInteractive,leavesOnly,nodes:result,phases:selected.map(node=>({id:node.id,enter:circlePhases(()=> '#112233').enter(node),update:circlePhases(()=> '#112233').update(node),leave:circlePhases(()=> '#112233').leave(node)}))});}
// Icicle uses actual shared RectNodes JSX + wrapper event gate and transition
// functions. Scalar .to adapters are probes, never real SpringValues.
const rectDefault=decl('rects/useRectsTransition.ts','transitionDefault'),rectModes=decl('rects/useRectsTransition.ts','rectTransitionModeById');
const modeTable=new Function(stripTypeScriptTypes(rectDefault+'\n'+rectModes)+'\nreturn rectTransitionModeById;')();
const useRectMode=evaluateDeclaration('rects/useRectsTransition.ts','useRectTransitionMode',{useMemo,rectTransitionModeById:modeTable});
const rectWrapper=tsx('rects/RectNodeWrapper.tsx','InnerRectNodeWrapper',{useMemo});dispatch.RectNodeWrapperProbe=rectWrapper;
const radiusCorners=evaluateDeclaration('theming/borderRadius.ts','normalizeBorderRadius');
let activePhase='update';const transitionCalls=[];
const useRects=evaluateDeclaration('rects/useRectsTransition.ts','useRectsTransition',{useMotionConfig:()=>({animate:false,config:{mass:1}}),useRectTransitionMode:useRectMode,useTransition:(selected,options)=>callback=>{transitionCalls.push({keys:selected.map(options.keys),initial:options.initial,immediate:options.immediate});return selected.map(node=>{const raw=options[activePhase](node);const props=Object.fromEntries(Object.entries(raw).map(([k,v])=>[k,typeof v==='number'?scalar(v):v]));return callback(props,node);});}});
const rectNodes=tsx('rects/RectNodes.tsx','RectNodes',{useCallback,usePropertyAccessor:uid=>{assert.equal(uid,'hierarchy.path');return n=>n.hierarchy.path;},useTheme:()=>theme,useInheritedColor:()=>()=> '#112233',useRectsTransition:useRects,RectNodeWrapper:'RectNodeWrapperProbe',normalizeBorderRadius:radiusCorners,to:plainTo});dispatch.RectNodesProbe=rectNodes;
const icicle=tsx('icicle/IcicleNodes.tsx','IcicleNodes',{useTooltip,useCallback,RectNodes:'RectNodesProbe'});
const icicleNodes=nodes.map(n=>({id:n.id,data:n.data,value:n.value,percentage:n.value/7*100,formattedValue:String(n.value),isVisible:n.path!=='root.b.same',color:n.color,rect:{x:n.x,y:n.y,width:n.width,height:n.height},hierarchy:{path:n.path,pathComponents:n.pathComponents,depth:n.treeDepth,height:n.treeHeight,parent:n.pathComponents.length>1?n.pathComponents.slice(0,-1).join('.'):null},a11y:{isFocusable:true,role:'treeitem'}}));
const icicleCases=[];
for(const isInteractive of[false,true])for(const mode of['flow-down','center','reveal-left'])for(const phase of['from','update','leave']){
 activePhase=phase;jsxCalls.length=0;const callbacks=[];
 const result=icicle({nodeRefs:{current:{}},nodes:icicleNodes,component:'CustomIcicle',borderRadius:2,borderWidth:1,borderColor:{from:'color'},isInteractive,enableZooming:true,zoom:path=>callbacks.push(['zoom',path]),onClick:(node,event)=>callbacks.push(['click',node.hierarchy.path]),tooltip:'Tooltip',isFocusable:true,nav:{moveUp:path=>callbacks.push(['up',path]),movePrev:path=>callbacks.push(['prev',path]),moveNext:path=>callbacks.push(['next',path]),moveDown:path=>callbacks.push(['down',path])},animateOnMount:false,transitionMode:mode});
 const calls=jsxCalls.filter(c=>c.type==='CustomIcicle');assert.equal(calls.length,5);assert(calls.some(c=>c.props.node.isVisible===false));for(const c of calls){assert.equal(Boolean(c.props.onClick),isInteractive);assert.equal(c.props.testId,'icicle.rect.'+c.props.node.hierarchy.path);}
 if(isInteractive){const hidden=calls.find(c=>!c.props.node.isVisible);hidden.props.onFocus({type:'focus'});assert.deepEqual(callbacks,[['zoom','root.b.same']]);hidden.props.onKeyDown({key:'ArrowDown',preventDefault:()=>callbacks.push(['prevent'])});}
 icicleCases.push({isInteractive,mode,phase,calls,result,callbacks});
}
const gateCases=[];for(const[file,needle]of[['treemap/TreeMap.tsx',"layers.includes('nodes')"],['circle-packing/CirclePacking.tsx',"layers.includes('circles')"],['icicle/Icicle.tsx',"layers.includes('rects')"]]){let expr;const s=source(file);traverse(babelParse(s,file,true),{IfStatement(path){const t=s.slice(path.node.test.start,path.node.test.end);if(t===needle)expr=t;}});assert.ok(expr);extracts[file+':layerGate']=expr;const run=new Function('layers',exprBody(expr));gateCases.push({family:file.split('/')[0],expression:expr,absent:run([]),present:run(['nodes','circles','rects'])});}
const result=canonical({scope:'Original Treemap/CirclePacking component creation and transitions plus Icicle/shared-RectNodes/Wrapper JSX and transition/event recipes. Input computed geometry is authored; selector methods, color/property accessors, synchronous hooks and scalar .to are explicit dependency probes. No D3 hierarchy/layout, React lifecycle, Spring runtime, browser focus transport or pixel equivalence claim.',versions:frozen?expected.versions:Object.fromEntries(await Promise.all(Object.keys(specs).map(async f=>[f,JSON.parse(await readFile(resolve(nivo,'packages',f,'package.json'),'utf8')).version]))),commit:frozen?expected.commit:execFileSync('git',['-C',nivo,'rev-parse','HEAD'],{encoding:'utf8'}).trim(),originalSources:sourceHashes,licenses:licenseHashes,producer,compiler,execution:extracts,input:{rawTree,circleRawTree,treemapNodes:nodes,circleNodes,icicleNodes},treemap:{cases:treemapCases},circlePacking:{cases:circleCases},icicle:{cases:icicleCases,transitionCalls},gateCases,events,
 semanticBoundaries:{aggregation:'sum(getValue), raw/aggregate normalization and path conventions are source-inspected only; no D3 hierarchy dependency is executed.',identities:'Treemap root-to-node dotpath keys permit repeated own IDs; Circle creation keys use own node.id and its path array is own-to-root; Icicle uid is hierarchy.path, own IDs may repeat across branches.',labels:'Treemap default node owns leaf/parent labels with independent flags and strict skip threshold; CirclePacking and Icicle labels are separate layers.',eligibility:'Computed zero-area/radius inputs are not filtered by these creation components. Icicle isVisible=false inputs still reach the wrapper and original focus handler requests zoom. Rust actual-Mark/focus/zero drawing eligibility must be stated separately.'}});
if(frozen)assert.deepEqual(result,expected,'exact original hierarchy component recipe execution');
else{for(const[name,bytes]of Object.entries(original)){const p=resolve(output,'original',name);await mkdir(dirname(p),{recursive:true});await writeFile(p,bytes);}for(const l of licenses)await writeFile(resolve(output,l.file),l.data);await writeFile(resolve(output,'reference.json'),JSON.stringify(result,null,2)+'\n');}
console.log(JSON.stringify({mode:frozen?'frozen-verify':'produce',sourceFiles:Object.keys(original).length,licenses:licenses.length,treemapProfiles:treemapCases.length,treemapCreationCalls:treemapCases.reduce((n,c)=>n+Object.values(c.phases).reduce((m,rows)=>m+rows.length,0),0),circleProfiles:circleCases.length,circleCreationCalls:circleCases.reduce((n,c)=>n+c.nodes.length,0),icicleProfiles:icicleCases.length,icicleCreationCalls:icicleCases.reduce((n,c)=>n+c.calls.length,0),producerSha256:producer.sha256,output}));
