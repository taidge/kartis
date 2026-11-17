// Actual Nivo component recipe execution; no React/Spring rendering claim.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {createRequire,stripTypeScriptTypes} from 'node:module';
import {resolve,dirname} from 'node:path';
import {fileURLToPath} from 'node:url';
import {execFileSync} from 'node:child_process';
const root=resolve(fileURLToPath(new URL('..',import.meta.url)));
if(!process.argv[2])throw Error('usage: bar-widget-reference.mjs <nivo-checkout> [output] | --frozen [fixture-directory]');
const frozen=process.argv[2]==='--frozen';const nivo=frozen?undefined:resolve(process.argv[2]);
const output=resolve(process.argv[3]??resolve(root,'tests/fixtures/bar-widgets'));
const expected=frozen?JSON.parse(await readFile(resolve(output,'reference.json'),'utf8')):undefined;
const {babelParse,traverse}=createRequire(import.meta.url)(resolve(root,'node_modules/playwright/lib/transform/babelBundle.js'));
const sha=b=>createHash('sha256').update(b).digest('hex');
const names=['Bar.tsx','BarItem.tsx','hooks.ts','types.ts','defaults.ts','compute/common.ts','compute/grouped.ts','compute/stacked.ts'];
const original=Object.fromEntries(await Promise.all(names.map(async n=>[n,await readFile(frozen?resolve(output,'original',n):resolve(nivo,'packages/bar/src',n))])));
const license=await readFile(frozen?resolve(output,'original/LICENSE.md'):resolve(nivo,'packages/bar/LICENSE.md'));
const sources=Object.fromEntries(Object.entries(original).map(([n,b])=>[n,{sha256:sha(b),bytes:b.length}]));
const licenseReference={file:'original/LICENSE.md',sha256:sha(license),bytes:license.length};
const producer={file:'scripts/bar-widget-reference.mjs',sha256:sha(await readFile(fileURLToPath(import.meta.url)))};
if(frozen){assert.deepEqual(sources,expected.originalSources,'original source hashes');assert.deepEqual(licenseReference,expected.license,'complete MIT license');assert.deepEqual(producer,expected.producer,'producer SHA');assert.equal(expected.package,'0.99.0');assert.match(expected.commit,/^[0-9a-f]{40}$/);}
const text=n=>original[n].toString('utf8');const extracts={};
const expressionBody=(source,invoke=false)=>stripTypeScriptTypes(`const __expression = (${source});`)+`\nreturn __expression${invoke?'()':''};`;
function variable(file,name){let found;const s=text(file);traverse(babelParse(s,file,true),{VariableDeclarator(path){if(path.node.id.name===name){assert.equal(found,undefined,name);found=path.node;}}});assert.ok(found,name);return{node:found,source:s};}
const hook=variable('hooks.ts','barsWithValue');extracts.barsWithValue=hook.source.slice(hook.node.init.arguments[0].start,hook.node.init.arguments[0].end);
const selectBars=new Function('bars',expressionBody(extracts.barsWithValue,true));
const label=variable('hooks.ts','shouldRenderBarLabel');extracts.shouldRenderBarLabel=label.source.slice(label.node.init.arguments[0].start,label.node.init.arguments[0].end);
const makeShouldLabel=new Function('enableLabel','labelSkipWidth','labelSkipHeight',expressionBody(extracts.shouldRenderBarLabel));
const bar=text('Bar.tsx');let options,callback,gate;
traverse(babelParse(bar,'Bar.tsx',true),{CallExpression(path){const n=path.node;
 if(n.callee.name==='useTransition'){assert.equal(options,undefined);options=bar.slice(n.arguments[1].start,n.arguments[1].end);}
 if(n.callee.name==='transition'){assert.equal(callback,undefined);callback=bar.slice(n.arguments[0].start,n.arguments[0].end);}},
 IfStatement(path){const s=bar.slice(path.node.test.start,path.node.test.end);if(s==="layers.includes('bars')"){assert.equal(gate,undefined);gate=s;}}
});assert.ok(options);assert.ok(callback);assert.ok(gate);Object.assign(extracts,{transitionOptions:options,transitionCallback:callback,barsLayerGate:gate});
const common=variable('Bar.tsx','commonProps');extracts.commonProps=common.source.slice(common.node.init.arguments[0].start,common.node.init.arguments[0].end);
const sourceCommon=text('compute/common.ts');let labelLayout;
traverse(babelParse(sourceCommon,'common.ts',true),{FunctionDeclaration(path){if(path.node.id.name==='useComputeLabelLayout'){assert.equal(labelLayout,undefined);labelLayout=sourceCommon.slice(path.node.start,path.node.end);}}});assert.ok(labelLayout);
extracts.labelLayout=labelLayout;
const computeLabel=new Function(stripTypeScriptTypes(labelLayout)+'\nreturn useComputeLabelLayout;')();
for(const name of ['normalizeData','filterNullValues','coerceValue']){const v=variable('compute/common.ts',name);extracts[name]='const '+v.source.slice(v.node.start,v.node.end)+';';}
const commonFunctions=new Function(stripTypeScriptTypes(['normalizeData','filterNullValues','coerceValue'].map(n=>extracts[n]).join('\n'))+'\nreturn {normalizeData,filterNullValues,coerceValue};')();
const rawRows=[{id:'one',value:null,zero:0,negativeZero:-0,negative:-3,empty:'',flag:false},{id:'two',value:7}];
const normalized=commonFunctions.normalizeData(rawRows,['value','zero','negativeZero']);
const cleaned=normalized.map(commonFunctions.filterNullValues);
const coerced=[null,0,-0,7,-3].map(value=>({input:value,result:commonFunctions.coerceValue(value)}));
assert.equal(normalized[1].zero,null);assert.ok(!('zero' in cleaned[0]));assert.ok(!('negativeZero' in cleaned[0]));assert.equal(cleaned[0].negative,-3);
const inputBars=[null,0,-0,7,-3].map((value,index)=>({key:`value.${index}`,index,data:{id:'value',value,formattedValue:String(value),hidden:false,index,indexValue:`category${index}`,data:rawRows[0]},x:20+index*25,y:30,absX:40+index*25,absY:50,width:18,height:value===null||value===0?0:Math.abs(value)*5,color:'#123456',label:`source ${index}`}));
const bars=selectBars(inputBars);assert.equal(bars.length,4);assert.deepEqual(bars.map(b=>b.index),[0,1,2,3]);assert.ok(Object.is(bars[1].data.value,-0));
const fn=name=>Object.defineProperty(()=>name,'name',{value:name});
const component={};const cases=[];
for(const layout of ['vertical','horizontal'])for(const isInteractive of [false,true])for(const enableLabel of [false,true]){
 const computeLabelLayout=computeLabel(layout,false,'middle',0);
 const transition=new Function('getBorderColor','getLabelColor','computeLabelLayout','layout','springConfig','animate',expressionBody(options))(()=> '#abcdef',()=> '#654321',computeLabelLayout,layout,{mass:1},false);
 assert.equal(transition.immediate,true);assert.equal(transition.initial,null);
 const input={borderRadius:3,borderWidth:2,enableLabel,isInteractive,labelSkipWidth:0,labelSkipHeight:0,onClick:fn('probeClick'),onMouseEnter:fn('probeEnter'),onMouseLeave:fn('probeLeave'),getTooltipLabel:fn('probeTooltipLabel'),tooltip:fn('ProbeTooltip'),isFocusable:true,barAriaLabel:fn('ariaLabel'),barAriaLabelledBy:undefined,barAriaDescribedBy:undefined,barAriaHidden:undefined,barAriaDisabled:undefined};
 const props=new Function(...Object.keys(input),expressionBody(extracts.commonProps,true))(...Object.values(input));
 assert.equal(props.isInteractive,isInteractive);assert.equal(props.enableLabel,enableLabel);
 const should=makeShouldLabel(enableLabel,0,0);const rendered=[];
 const create=new Function('createElement','barComponent','commonProps','shouldRenderBarLabel','getLabel',expressionBody(callback))((type,p)=>{assert.equal(type,component);rendered.push(p);return p;},component,props,should,d=>d.formattedValue);
 const phases={};for(const phase of ['from','enter','update','leave']){phases[phase]=bars.map(b=>{assert.equal(transition.keys(b),b.key);const style=transition[phase](b);const p=create(style,b);assert.equal(p.bar,b);assert.equal(p.style,style);return p;});}
 assert.equal(rendered.length,16);assert.equal(phases.enter[0].bar.data.value,0);assert.equal(phases.enter[0].style.height,0);
 cases.push({layout,isInteractive,enableLabel,phases});
}
const gateRun=new Function('layers',`return ${gate};`);const gateCases=[[],['bars'],['grid','bars']].map(layers=>({layers,result:gateRun(layers)}));assert.deepEqual(gateCases.map(c=>c.result),[false,true,true]);
const item=text('BarItem.tsx');const scalarExpressions={};
traverse(babelParse(item,'BarItem.tsx',true),{JSXAttribute(path){const n=path.node;const name=n.name.name;const value=n.value?.expression;if(!value)return;
 if(['width','height'].includes(name)&&value.type==='CallExpression'&&value.callee.name==='to'){scalarExpressions[name]=item.slice(value.arguments[1].start,value.arguments[1].end);}
 if(['fill','onMouseEnter','onMouseMove','onMouseLeave','onClick','onFocus','onBlur','tabIndex'].includes(name))scalarExpressions[name]=item.slice(value.start,value.end);
}});assert.equal(Object.keys(scalarExpressions).length,10);extracts.defaultRect=scalarExpressions;
const clamps=Object.fromEntries(['width','height'].map(name=>[name,[-4,0,12].map(value=>({value,result:new Function(`return (${scalarExpressions[name]});`)()(value)}))]));assert.deepEqual(clamps.width.map(v=>v.result),[0,0,12]);
const itemHandlers=[];
for(const isInteractive of [false,true])for(const isFocusable of [false,true]){
 const args={isInteractive,isFocusable,handleMouseEnter:fn('enter'),handleTooltip:fn('move'),handleMouseLeave:fn('leave'),handleClick:fn('click'),handleFocus:fn('focus'),handleBlur:fn('blur')};const result={};
 for(const name of ['onMouseEnter','onMouseMove','onMouseLeave','onClick','onFocus','onBlur','tabIndex'])result[name]=new Function(...Object.keys(args),`return (${scalarExpressions[name]});`)(...Object.values(args));
 assert.equal(Boolean(result.onClick),isInteractive);assert.equal(Boolean(result.onFocus),isInteractive&&isFocusable);assert.equal(result.tabIndex,isFocusable?0:undefined);itemHandlers.push({isInteractive,isFocusable,result});
}
const fills=[undefined,'url(#pattern)',''].map(fill=>({fill,result:new Function('data','color',`return (${scalarExpressions.fill});`)({fill},'#123456')}));assert.deepEqual(fills.map(v=>v.result),['#123456','url(#pattern)','']);
const labelThresholds=[[true,0,0],[false,0,0],[true,20,0],[true,0,1]].map(([enabled,w,h])=>({enabled,w,h,zeroBar:makeShouldLabel(enabled,w,h)(bars[0]),positiveBar:makeShouldLabel(enabled,w,h)(bars[2])}));assert.equal(labelThresholds[0].zeroBar,true);
const canonical=value=>JSON.parse(JSON.stringify(value,(_key,v)=>typeof v==='number'&&Object.is(v,-0)?{$number:'-0'}:typeof v==='function'?{$function:v.name}:v));
const result=canonical({scope:'Original Nivo component selection, props, transition/style recipes and scalar projections executed unchanged. Plain style values and function probes are not React Spring values or live React rendering. No pixel or lifecycle equivalence claim.',package:frozen?expected.package:JSON.parse(await readFile(resolve(nivo,'packages/bar/package.json'),'utf8')).version,commit:frozen?expected.commit:execFileSync('git',['-C',nivo,'rev-parse','HEAD'],{encoding:'utf8'}).trim(),originalSources:sources,license:licenseReference,producer,execution:extracts,rawRows,normalized,cleaned,coerced,inputBars,barsWithValue:bars,cases,gateCases,clamps,itemHandlers,fills,labelThresholds});
if(frozen)assert.deepEqual(result,expected,'exact frozen original execution results');
else{await mkdir(resolve(output,'original'),{recursive:true});for(const [n,b]of Object.entries(original)){await mkdir(dirname(resolve(output,'original',n)),{recursive:true});await writeFile(resolve(output,'original',n),b);}await writeFile(resolve(output,'original/LICENSE.md'),license);await writeFile(resolve(output,'reference.json'),JSON.stringify(result,null,2)+'\n');}
console.log(JSON.stringify({mode:frozen?'frozen-verify':'produce',output,sourceFiles:names.length,barsWithValue:bars.length,componentProfiles:cases.length,createElementCalls:cases.length*4*bars.length,gateCases:gateCases.length,defaultRectHandlerProfiles:itemHandlers.length,producerSha256:producer.sha256}));
