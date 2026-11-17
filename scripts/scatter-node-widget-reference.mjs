// Execute original Nivo functions/element arguments, not React/spring lifecycle.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {createRequire,stripTypeScriptTypes} from 'node:module';
import {resolve} from 'node:path';
import {fileURLToPath} from 'node:url';
import {execFileSync} from 'node:child_process';

const root=resolve(fileURLToPath(new URL('..',import.meta.url)));
if(!process.argv[2])throw Error('usage: scatter-node-widget-reference.mjs <nivo-checkout> [output] | --frozen [fixture-directory]');
const frozen=process.argv[2]==='--frozen';
const nivo=frozen?undefined:resolve(process.argv[2]);
const output=resolve(process.argv[3]??resolve(root,'tests/fixtures/scatter-node-widgets'));
const expected=frozen?JSON.parse(await readFile(resolve(output,'reference.json'),'utf8')):undefined;
const require=createRequire(import.meta.url);
const {babelParse,traverse}=require(resolve(root,'node_modules/playwright/lib/transform/babelBundle.js'));
const sha=bytes=>createHash('sha256').update(bytes).digest('hex');
const paths=['Nodes.tsx','Node.tsx','compute.ts','hooks.ts','types.ts','props.tsx'];
const original=Object.fromEntries(await Promise.all(paths.map(async name=>[name,await readFile(frozen?resolve(output,'original',name):resolve(nivo,'packages/scatterplot/src',name))])));
const license=await readFile(frozen?resolve(output,'original/LICENSE.md'):resolve(nivo,'packages/scatterplot/LICENSE.md'));
const originalSources=Object.fromEntries(Object.entries(original).map(([n,b])=>[n,{sha256:sha(b),bytes:b.length}]));
const licenseReference={file:'original/LICENSE.md',sha256:sha(license),bytes:license.length};
const producer={file:'scripts/scatter-node-widget-reference.mjs',sha256:sha(await readFile(fileURLToPath(import.meta.url)))};
if(frozen){assert.deepEqual(originalSources,expected.originalSources,'original source hashes');assert.deepEqual(licenseReference,expected.license,'complete MIT license');assert.deepEqual(producer,expected.producer,'producer SHA');assert.equal(expected.package,'0.99.0');assert.match(expected.commit,/^[0-9a-f]{40}$/);}
const source=original['Nodes.tsx'].toString('utf8');
const declarations={};let transitionCallback,transitionOptions;
traverse(babelParse(source,'Nodes.tsx',true),{
 VariableDeclarator(path){const name=path.node.id.name;if(['getNodeKey','regularTransition','leaveTransition'].includes(name)){assert.equal(declarations[name],undefined);declarations[name]='const '+source.slice(path.node.start,path.node.end)+';';}},
 CallExpression(path){const node=path.node;
  if(node.callee.name==='transition'){assert.equal(transitionCallback,undefined);transitionCallback=source.slice(node.arguments[0].start,node.arguments[0].end);}
  if(node.callee.name==='useTransition'){assert.equal(transitionOptions,undefined);transitionOptions=source.slice(node.arguments[1].start,node.arguments[1].end);}
 }
});
assert.equal(Object.keys(declarations).length,3);assert.ok(transitionCallback);assert.ok(transitionOptions);
const funcs=new Function(stripTypeScriptTypes(Object.values(declarations).join('\n'))+'\nreturn {getNodeKey,regularTransition,leaveTransition};')();
const options=new Function('getNodeKey','regularTransition','leaveTransition','springConfig','animate',`return (${transitionOptions});`)(funcs.getNodeKey,funcs.regularTransition,funcs.leaveTransition,{mass:1},false);
for(const role of ['from','enter','update'])assert.equal(options[role],funcs.regularTransition);
assert.equal(options.leave,funcs.leaveTransition);assert.equal(options.keys,funcs.getNodeKey);assert.equal(options.immediate,true);
const computeSource=original['compute.ts'].toString('utf8');let computeDeclaration;
traverse(babelParse(computeSource,'compute.ts',true),{VariableDeclarator(path){if(path.node.id.name==='computePoints'){assert.equal(computeDeclaration,undefined);computeDeclaration='const '+computeSource.slice(path.node.start,path.node.end)+';';}}});
assert.ok(computeDeclaration);
const compute=new Function('series','formatX','formatY','getNodeId',stripTypeScriptTypes(computeDeclaration)+'\nreturn computePoints({series,formatX,formatY,getNodeId});');
const series=[{id:'first',data:[{position:{x:-20,y:30},data:{x:1,y:2,note:'raw outside'}},{position:{x:70,y:80},data:{x:3,y:4,note:'raw zero size'}}]},
 {id:'second',data:[{position:{x:100,y:40},data:{x:'category',y:5,note:'raw category'}}]}];
const nodes=compute(series,x=>`x:${x}`,y=>`y:${y}`,p=>`${p.serieId}:${p.serieIndex}`);
assert.deepEqual(nodes.map(n=>[n.index,n.serieIndex,n.serieId]),[[0,0,'first'],[1,1,'first'],[2,0,'second']]);
for(const n of nodes){const raw=series.find(s=>s.id===n.serieId).data[n.serieIndex].data;assert.equal(n.data,raw);}
const colored=nodes.map((node,i)=>({...node,size:[9,0,18][i],color:['#123456','#abcdef','#654321'][i]}));
const handlerNames=['onMouseEnter','onMouseMove','onMouseLeave','onMouseDown','onMouseUp','onClick','onDoubleClick'];
const bindings=['handleMouseEnter','handleMouseMove','handleMouseLeave','handleMouseDown','handleMouseUp','handleClick','handleDoubleClick'];
const component={};const handlers=bindings.map(name=>Object.defineProperty(()=>{},'name',{value:name}));
const cases=[];
for(const interactive of [false,true])for(const phase of ['regular','leave']){
 const actual=[];
 const callback=new Function('createElement','nodeComponent','blendMode','isInteractive',...bindings,`return (${transitionCallback});`)((type,props)=>{assert.equal(type,component);actual.push(props);return props;},component,'multiply',interactive,...handlers);
 for(const node of colored){const style=phase==='regular'?funcs.regularTransition(node):funcs.leaveTransition(node);const props=callback(style,node);assert.equal(props.node,node);assert.equal(props.style,style);assert.equal(funcs.getNodeKey(node),node.id);for(let i=0;i<handlerNames.length;i++)assert.equal(props[handlerNames[i]],interactive?handlers[i]:undefined);}
 cases.push({interactive,phase,nodes:actual.map(props=>({...props,...Object.fromEntries(handlerNames.map(name=>[name,props[name]?.name??null]))}))});
}
const nodeSource=original['Node.tsx'].toString('utf8');let radius;
traverse(babelParse(nodeSource,'Node.tsx',true),{VariableDeclarator(path){if(path.node.id.name==='interpolateRadius'){assert.equal(radius,undefined);radius='const '+nodeSource.slice(path.node.start,path.node.end)+';';}}});
assert.ok(radius);const circle=new Function(stripTypeScriptTypes(radius)+'\nreturn interpolateRadius;')();
const defaultRadius=[0,9,18].map(size=>({size,radius:circle(size)}));assert.deepEqual(defaultRadius,[{size:0,radius:0},{size:9,radius:4.5},{size:18,radius:9}]);
const result={scope:'Original computePoints, transition functions/options, transition createElement callback and radius helper executed unchanged. Callback styles are plain probe values, not real SpringValues. No React lifecycle, animation scheduling, rendering or physical-event claim.',
 package:frozen?expected.package:JSON.parse(await readFile(resolve(nivo,'packages/scatterplot/package.json'),'utf8')).version,
 commit:frozen?expected.commit:execFileSync('git',['-C',nivo,'rev-parse','HEAD'],{encoding:'utf8'}).trim(),originalSources,license:licenseReference,producer,
 execution:{getNodeKey:declarations.getNodeKey,regularTransition:declarations.regularTransition,leaveTransition:declarations.leaveTransition,transitionCallback,transitionOptions,computePoints:computeDeclaration,radius},
 input:series,computedNodes:colored,transitionRecipe:{keys:'getNodeKey',from:'regularTransition',enter:'regularTransition',update:'regularTransition',leave:'leaveTransition',immediate:options.immediate,config:options.config},cases,defaultRadius};
if(frozen){assert.deepEqual(result,expected,'frozen original execution results');}
else{await mkdir(resolve(output,'original'),{recursive:true});for(const [name,bytes]of Object.entries(original))await writeFile(resolve(output,'original',name),bytes);await writeFile(resolve(output,'original/LICENSE.md'),license);await writeFile(resolve(output,'reference.json'),JSON.stringify(result,null,2)+'\n');}
console.log(JSON.stringify({mode:frozen?'frozen-verify':'produce',output,sourceFiles:paths.length,computedNodes:colored.length,callbackProfiles:cases.length,createElementCalls:cases.reduce((n,c)=>n+c.nodes.length,0),defaultRadiusCases:defaultRadius.length,producerSha256:producer.sha256}));
