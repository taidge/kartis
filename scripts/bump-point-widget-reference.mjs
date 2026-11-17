// Verify original Bump computation/component recipes without a React runtime.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {createRequire,stripTypeScriptTypes} from 'node:module';
import {resolve,dirname} from 'node:path';
import {fileURLToPath,pathToFileURL} from 'node:url';
import {execFileSync} from 'node:child_process';
const root=resolve(fileURLToPath(new URL('..',import.meta.url)));
if(!process.argv[2])throw Error('usage: bump-point-widget-reference.mjs <nivo-checkout> [output] | --frozen [fixture-directory]');
const frozen=process.argv[2]==='--frozen';const nivo=frozen?undefined:resolve(process.argv[2]);
const output=resolve(process.argv[3]??resolve(root,'tests/fixtures/bump-point-widgets'));
const expected=frozen?JSON.parse(await readFile(resolve(output,'reference.json'),'utf8')):undefined;
const {babelParse,traverse}=createRequire(import.meta.url)(resolve(root,'node_modules/playwright/lib/transform/babelBundle.js'));
const sha=b=>createHash('sha256').update(b).digest('hex');
const paths=Object.fromEntries(['Bump.tsx','Point.tsx','hooks.ts','compute.ts','types.ts','defaults.ts','Mesh.tsx'].map(n=>['bump/'+n,'packages/bump/src/bump/'+n]));
paths['scales/pointScale.ts']='packages/scales/src/pointScale.ts';
const original=Object.fromEntries(await Promise.all(Object.entries(paths).map(async([name,path])=>[name,await readFile(frozen?resolve(output,'original',name):resolve(nivo,path))])));
const license=await readFile(frozen?resolve(output,'original/LICENSE.md'):resolve(nivo,'packages/bump/LICENSE.md'));
const scaleLicense=await readFile(frozen?resolve(output,'original/scales/LICENSE.md'):resolve(nivo,'packages/scales/LICENSE.md'));
const sources=Object.fromEntries(Object.entries(original).map(([name,b])=>[name,{sha256:sha(b),bytes:b.length}]));
const licenses=[{file:'original/LICENSE.md',sha256:sha(license),bytes:license.length},{file:'original/scales/LICENSE.md',sha256:sha(scaleLicense),bytes:scaleLicense.length}];
const producer={file:'scripts/bump-point-widget-reference.mjs',sha256:sha(await readFile(fileURLToPath(import.meta.url)))};
const dependencyPaths=['d3-scale/package.json','d3-scale/LICENSE','d3-scale/src/band.js','d3-scale/src/ordinal.js','d3-scale/src/init.js','d3-array/package.json','d3-array/LICENSE','d3-array/dist/d3-array.mjs'];
const dependencies=Object.fromEntries(await Promise.all(dependencyPaths.map(async name=>{const b=await readFile(resolve(root,'node_modules',name));return[name,{sha256:sha(b),bytes:b.length}]})));
assert.equal(JSON.parse(await readFile(resolve(root,'node_modules/d3-scale/package.json'),'utf8')).version,'4.0.2');
assert.equal(JSON.parse(await readFile(resolve(root,'node_modules/d3-array/package.json'),'utf8')).version,'3.2.5');
if(frozen){assert.deepEqual(sources,expected.originalSources,'original source hashes');assert.deepEqual(licenses,expected.licenses,'complete MIT licenses');assert.deepEqual(producer,expected.producer,'producer SHA');assert.deepEqual(dependencies,expected.dependencies,'actual point-scale dependency source hashes');assert.equal(expected.package,'0.99.0');assert.match(expected.commit,/^[0-9a-f]{40}$/);}
const {scalePoint}=await import(pathToFileURL(resolve(root,'node_modules/d3-scale/src/index.js')));
const text=name=>original[name].toString('utf8');const extracts={};
const expressionBody=(s,invoke=false)=>stripTypeScriptTypes(`const __expression=(${s});`)+`\nreturn __expression${invoke?'()':''};`;
function variable(file,name){const source=text(file);let found;traverse(babelParse(source,file,true),{VariableDeclarator(path){if(path.node.id.name===name){assert.equal(found,undefined,file+':'+name);found=path.node;}}});assert.ok(found,file+':'+name);return{node:found,source};}
function declaration(file,name){const v=variable(file,name);return'const '+v.source.slice(v.node.start,v.node.end)+';';}
extracts.castPointScale=declaration('scales/pointScale.ts','castPointScale');extracts.computeSeries=declaration('bump/compute.ts','computeSeries');
const compute=new Function('scalePoint',stripTypeScriptTypes(extracts.castPointScale+'\n'+extracts.computeSeries)+'\nreturn computeSeries;')(scalePoint);
const input={width:240,height:160,xPadding:.6,xOuterPadding:.5,yOuterPadding:.5,data:[
 {id:'alpha',extra:'original alpha',data:[{x:'Q3',y:null},{x:'Q1',y:0},{x:1,y:3},{x:'1',y:-1},{x:'Q4',y:null}]},
 {id:'beta',extra:'original beta',data:[{x:'Q4',y:2},{x:'Q3',y:null},{x:'Q1',y:3}]},
 {id:'gap-only',extra:'original gaps',data:[{x:'Q3',y:null},{x:'Q4',y:null}]},
]};
const raw=compute(input);assert.deepEqual(raw.series.map(s=>s.points.map(p=>p.id)),[['alpha.1','alpha.2','alpha.3'],['beta.0','beta.2'],[]]);
assert.equal(raw.series.reduce((n,s)=>n+s.points.length,0),5);assert.equal(input.data.reduce((n,s)=>n+s.data.length,0),10);
for(const s of raw.series)for(const p of s.points){assert.notEqual(p.data.y,null);assert.equal(p.data,s.data.data[Number(p.id.slice(p.id.lastIndexOf('.')+1))]);assert.equal(p.serie,s.data);}
for(const name of ['useSerieDerivedProp','useSerieStyle','usePointStyle'])extracts[name]=declaration('bump/hooks.ts',name);
// These adapters evaluate the original synchronous recipe bodies. They do not
// implement React hook identity, memo invalidation, scheduling or lifecycle.
const hooks=new Function('useMemo','useCallback',stripTypeScriptTypes(['useSerieDerivedProp','useSerieStyle','usePointStyle'].map(n=>extracts[n]).join('\n'))+'\nreturn {usePointStyle,useSerieStyle};')(f=>f(),f=>f);
for(const name of ['series','points']){const v=variable('bump/hooks.ts',name);extracts[name]=v.source.slice(v.node.init.arguments[0].start,v.node.init.arguments[0].end);}
const makeSeries=new Function('rawSeries','getColor','getSerieStyle',expressionBody(extracts.series,true));
const makePoints=new Function('series','activeSerieIds','getPointColor','getPointBorderColor','getPointStyle',expressionBody(extracts.points,true));
const defaults=variable('bump/defaults.ts','commonDefaultProps');extracts.defaults=defaults.source.slice(defaults.node.init.start,defaults.node.init.end);
const common=new Function('LineTooltip','PointTooltip',expressionBody(extracts.defaults))(()=>{},()=>{});
const selectedDefaults=Object.fromEntries(['pointSize','activePointSize','inactivePointSize','pointBorderWidth','activePointBorderWidth','inactivePointBorderWidth','lineWidth','activeLineWidth','inactiveLineWidth','opacity','activeOpacity','inactiveOpacity','isInteractive','useMesh'].map(k=>[k,common[k]]));
const bump=text('bump/Bump.tsx');let creation,pointGate,meshGate;
traverse(babelParse(bump,'Bump.tsx',true),{CallExpression(path){const n=path.node;if(n.callee.type==='MemberExpression'&&n.callee.object.name==='points'&&n.callee.property.name==='map'){assert.equal(creation,undefined);creation=bump.slice(n.start,n.end);}},IfStatement(path){const s=bump.slice(path.node.test.start,path.node.test.end);if(s==="layers.includes('points')")pointGate=s;if(s==="isInteractive && useMesh && layers.includes('mesh')")meshGate=s;}});
assert.ok(creation);assert.ok(pointGate);assert.ok(meshGate);Object.assign(extracts,{creation,pointGate,meshGate});
const create=new Function('points','pointComponent','createElement',expressionBody(creation));
const gates=new Function('layers','isInteractive','useMesh',`return {points:${pointGate},mesh:${meshGate}};`);
const pointSource=text('bump/Point.tsx');let spring,radius;
traverse(babelParse(pointSource,'Point.tsx',true),{CallExpression(path){const n=path.node;if(n.callee.name==='useSpring'){assert.equal(spring,undefined);spring=pointSource.slice(n.arguments[0].start,n.arguments[0].end);}if(n.callee.name==='to'){assert.equal(radius,undefined);radius=pointSource.slice(n.arguments[1].start,n.arguments[1].end);}}});
assert.ok(spring);assert.ok(radius);extracts.spring=spring;extracts.radius=radius;extracts.pointStyle=declaration('bump/Point.tsx','pointStyle');
const defaultPointStyle=new Function(stripTypeScriptTypes(extracts.pointStyle)+'\nreturn pointStyle;')();assert.deepEqual(defaultPointStyle,{pointerEvents:'none'});
const runSpring=new Function('point','springConfig','animate',expressionBody(spring));const clampRadius=new Function(expressionBody(radius))();
const circleAttributes=[];traverse(babelParse(pointSource,'Point.tsx',true),{JSXAttribute(path){circleAttributes.push(path.node.name.name);}});
assert(!circleAttributes.includes('opacity'));assert(!circleAttributes.some(k=>k.startsWith('on')));
extracts.circleAttributeNames=circleAttributes;
let defined;const hookSource=text('bump/hooks.ts');traverse(babelParse(hookSource,'hooks.ts',true),{CallExpression(path){const n=path.node;if(n.callee.type==='MemberExpression'&&n.callee.property.name==='defined'){assert.equal(defined,undefined);defined=hookSource.slice(n.arguments[0].start,n.arguments[0].end);}}});assert.ok(defined);extracts.lineDefined=defined;
const lineDefined=new Function(expressionBody(defined))();
const profileInputs=[
 {name:'normal',isInteractive:true,activeSerieIds:[],activePointIds:[]},
 {name:'series-only',isInteractive:true,activeSerieIds:['alpha'],activePointIds:[]},
 {name:'point-only',isInteractive:true,activeSerieIds:[],activePointIds:['alpha.2']},
 {name:'different-active-axes',isInteractive:true,activeSerieIds:['beta'],activePointIds:['alpha.2']},
 {name:'interaction-disabled',isInteractive:false,activeSerieIds:['beta'],activePointIds:['alpha.2']},
 {name:'zero-sizes',isInteractive:true,activeSerieIds:[],activePointIds:[],pointSize:0,activePointSize:0,inactivePointSize:0},
];
const component={};const cases=[];
for(const profile of profileInputs){
 const config={...common,...profile};const getSerieStyle=hooks.useSerieStyle(config),actualStyle=hooks.usePointStyle(config);
 const series=makeSeries(raw.series,s=>s.id==='alpha'?'#123456':'#abcdef',getSerieStyle);const accessors=[];
 const points=makePoints(series,profile.activeSerieIds,p=>{accessors.push({stage:'color',id:p.id,hasColor:Object.hasOwn(p,'color'),hasBorderColor:Object.hasOwn(p,'borderColor')});assert.equal(p.serie,series.find(s=>s.id===p.serie.id));return p.serie.color;},p=>{accessors.push({stage:'borderColor',id:p.id,hasColor:Object.hasOwn(p,'color'),hasBorderColor:Object.hasOwn(p,'borderColor')});return'#112233';},p=>{accessors.push({stage:'style',id:p.id,hasColor:Object.hasOwn(p,'color'),hasBorderColor:Object.hasOwn(p,'borderColor')});return actualStyle(p);});
 assert.equal(points.length,5);assert.deepEqual(accessors.map(a=>a.stage),points.flatMap(()=>['color','borderColor','style']));
 const nodes=create(points,component,(type,props)=>{assert.equal(type,component);assert.deepEqual(Object.keys(props),['key','point']);assert.equal(props.key,props.point.id);assert.equal(props.point,points.find(p=>p.id===props.key));return{key:props.key,point:props.point};});
 const springs=points.map(point=>({id:point.id,input:[false,true].map(animate=>({animate,spring:runSpring(point,{mass:1},animate)}))}));
 if(profile.name==='zero-sizes')assert(nodes.every(n=>n.point.size===0));
 if(profile.name==='different-active-axes'){assert.equal(points.find(p=>p.id==='alpha.2').size,8);assert.equal(points.find(p=>p.id==='alpha.2').isInactive,true);assert.equal(points.find(p=>p.id==='beta.0').isActive,true);assert.equal(points.find(p=>p.id==='beta.0').size,4);}
 if(profile.name==='interaction-disabled')assert(points.every(p=>p.size===6));
 cases.push({profile,series,points,accessors,nodes,springs});
}
const gateCases=[];for(const layers of [[],['points'],['mesh'],['points','mesh']])for(const isInteractive of[false,true])for(const useMesh of[false,true]){const result=gates(layers,isInteractive,useMesh);assert.equal(result.points,layers.includes('points'));assert.equal(result.mesh,isInteractive&&useMesh&&layers.includes('mesh'));gateCases.push({layers,isInteractive,useMesh,result});}
const pointStyleProbe={style:defaultPointStyle,hasOpacityAttribute:circleAttributes.includes('opacity'),hasDirectEventAttributes:circleAttributes.some(k=>k.startsWith('on')),radiusCases:[-3,0,4].map(value=>({value,result:clampRadius(value)}))};assert.deepEqual(pointStyleProbe.radiusCases.map(p=>p.result),[0,0,4]);
// Isolate the downstream hook's behavior from the normal computed pipeline.
// This invented computed input is never emitted by computeSeries: its null X
// point was already filtered above. It is not an additional production role.
const manualRaw={...raw.series[0],points:[{id:'alpha.0',serie:input.data[0],data:input.data[0].data[0],x:null,y:null}]};
const manualSeries=makeSeries([manualRaw],()=> '#123456',hooks.useSerieStyle({...common,activeSerieIds:[]}));
const manualPoints=makePoints(manualSeries,[],p=>p.serie.color,()=> '#112233',hooks.usePointStyle({...common,activePointIds:[]}));
const manualNodes=create(manualPoints,component,(_type,props)=>({key:props.key,point:props.point}));
assert.equal(manualNodes.length,1);assert.equal(manualNodes[0].point.x,null);assert.equal(manualNodes[0].point.y,null);
const manualHookInputDiagnostic={scope:'Hand-authored computed-series input that computeSeries never emits. The downstream hooks/creation mapping add no second null filter. Not a normal upstream pipeline result or an extra Rust role obligation.',points:manualPoints,nodes:manualNodes};
const lineEligibility=raw.series.map(s=>({id:s.id,linePoints:s.linePoints,defined:s.linePoints.map(lineDefined)}));assert.equal(lineEligibility.reduce((n,s)=>n+s.defined.filter(v=>!v).length,0),5);
const tsv=['series\tsource_index\traw_x_kind\traw_x\trank\tx\ty',...raw.series.flatMap(s=>s.points.map(p=>[s.id,Number(p.id.slice(p.id.lastIndexOf('.')+1)),typeof p.data.x,p.data.x,p.data.y,p.x,p.y].join('\t')))].join('\n')+'\n';
const canonical=v=>JSON.parse(JSON.stringify(v,(_k,n)=>typeof n==='number'&&Object.is(n,-0)?{$number:'-0'}:n));
const result=canonical({scope:'Original computeSeries/castPointScale plus actual d3-scale scalePoint, point/series style recipes, points mapping/createElement arguments and Point spring/clamp expressions. Synchronous hook shims and accessor probes do not model React lifecycle, SpringValues, animation, inherited-color math or D3 line drawing.',package:frozen?expected.package:JSON.parse(await readFile(resolve(nivo,'packages/bump/package.json'),'utf8')).version,commit:frozen?expected.commit:execFileSync('git',['-C',nivo,'rev-parse','HEAD'],{encoding:'utf8'}).trim(),originalSources:sources,licenses,dependencies,producer,execution:extracts,input,raw:{series:raw.series,xDomain:raw.xScale.domain(),yDomain:raw.yScale.domain(),xRange:raw.xScale.range(),yRange:raw.yScale.range(),xStep:raw.xScale.step(),yStep:raw.yScale.step(),xType:raw.xScale.type,yType:raw.yScale.type},selectedDefaults,cases,gateCases,pointStyleProbe,manualHookInputDiagnostic,lineEligibility,fixtureArtifacts:{pointsTsv:{file:'points.tsv',sha256:sha(tsv),bytes:Buffer.byteLength(tsv)}}});
if(frozen){assert.deepEqual(result,expected,'exact frozen original Bump execution');assert.equal(await readFile(resolve(output,'points.tsv'),'utf8'),tsv,'exact native point sidecar');}
else{for(const[name,b]of Object.entries(original)){const p=resolve(output,'original',name);await mkdir(dirname(p),{recursive:true});await writeFile(p,b);}await writeFile(resolve(output,'original/LICENSE.md'),license);await writeFile(resolve(output,'original/scales/LICENSE.md'),scaleLicense);await writeFile(resolve(output,'points.tsv'),tsv);await writeFile(resolve(output,'reference.json'),JSON.stringify(result,null,2)+'\n');}
console.log(JSON.stringify({mode:frozen?'frozen-verify':'produce',sourceFiles:Object.keys(original).length,rawDatums:10,computedPoints:5,gaps:5,profiles:cases.length,createElementCalls:cases.reduce((n,c)=>n+c.nodes.length,0),gateCases:gateCases.length,springInputs:cases.reduce((n,c)=>n+c.springs.length*2,0),producerSha256:producer.sha256,output}));
