// Execute pinned vendored Nivo Waffle functions with explicit cold-hook/JSX adapters.
// No React reconciler, browser, native DOM, Canvas or product code is executed.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {execFileSync} from 'node:child_process';
import {readFile,writeFile,mkdir,readdir} from 'node:fs/promises';
import {createRequire,stripTypeScriptTypes} from 'node:module';
import {resolve,dirname,relative} from 'node:path';
import {fileURLToPath} from 'node:url';
import {line as d3Line,curveLinearClosed} from 'd3-shape';

const self=fileURLToPath(import.meta.url),root=resolve(dirname(self),'..');
const flags=Object.fromEntries(process.argv.slice(2).map(s=>{const n=s.indexOf('=');assert(n>2);return[s.slice(2,n),s.slice(n+1)];}));
assert(flags.out && Object.keys(flags).every(k=>['nivo','out'].includes(k)),'--out=<new directory> [--nivo=<explicit source checkout>]');
const fixtures=resolve(root,'tests/fixtures/waffle-svg'),index=JSON.parse(await readFile(resolve(fixtures,'source-index.json'))),checkout=flags.nivo?resolve(flags.nivo):resolve(fixtures,'original'),out=resolve(flags.out),sha=b=>createHash('sha256').update(b).digest('hex');
await mkdir(out,{recursive:false});
const git=(...args)=>execFileSync('git',['-C',checkout,...args],{encoding:'utf8'}).trim();
const commit=flags.nivo?git('rev-parse','HEAD'):index.commit;assert.equal(commit,'0dae2c32052a573f9f1d66ec1b453b572119b265');
const require=createRequire(resolve(root,'package.json'));
const babelPath=resolve(root,'node_modules/playwright/lib/transform/babelBundle.js');
const {babelParse,babelTransform,traverse}=require(babelPath);
const names=[
 'packages/waffle/src/Waffle.tsx','packages/waffle/src/WaffleCells.tsx','packages/waffle/src/WaffleCell.tsx',
 'packages/waffle/src/WaffleAreas.tsx','packages/waffle/src/WaffleArea.tsx','packages/waffle/src/WaffleTooltip.tsx',
 'packages/waffle/src/hooks.ts','packages/waffle/src/types.ts','packages/waffle/src/defaults.ts',
 'packages/waffle/package.json','packages/waffle/LICENSE.md',
 'packages/grid/src/grid.ts','packages/grid/src/boundingBoxes.ts','packages/grid/src/polygon.ts','packages/grid/src/types.ts','packages/grid/LICENSE.md',
 'packages/core/src/hooks/useDimensions.js','packages/core/src/defaults/index.js','packages/core/src/components/SvgWrapper.js','packages/core/LICENSE.md',
 'packages/legends/src/svg/BoxLegendSvg.tsx','packages/legends/src/svg/LegendSvg.tsx','packages/legends/src/svg/LegendSvgItem.tsx',
 'packages/legends/src/svg/symbols/SymbolSquare.tsx','packages/legends/src/svg/symbols/SymbolCircle.tsx',
 'packages/legends/src/svg/symbols/SymbolDiamond.tsx','packages/legends/src/svg/symbols/SymbolTriangle.tsx',
 'packages/legends/src/compute.ts','packages/legends/src/defaults.ts','packages/legends/src/types.ts','packages/legends/LICENSE.md',
 'packages/colors/src/inheritedColor.ts','packages/colors/src/scales/ordinalColorScale.ts','packages/colors/LICENSE.md',
];
const originals={},asts={},sources={},extracts={};
for(const name of names){const bytes=await readFile(resolve(checkout,name));assert.equal(sha(bytes),index.sources[name].sha256,'pinned source '+name);if(flags.nivo)assert(bytes.equals(execFileSync('git',['-C',checkout,'show',`${commit}:${name}`])),'uncommitted source '+name);originals[name]=bytes;sources[name]={sha256:sha(bytes),bytes:bytes.length};if(/\.[jt]sx?$/.test(name))asts[name]=babelParse(bytes.toString(),'original-'+name,true);}
assert.equal(JSON.parse(originals['packages/waffle/package.json']).version,'0.99.0');
for(const name of names.filter(n=>n.endsWith('LICENSE.md')))assert.match(originals[name].toString(),/Permission is hereby granted/);
function expression(file,name,declaration=false){let found;traverse(asts[file],declaration?{FunctionDeclaration(p){if(p.node.id?.name===name){assert(!found);found=p.node;}}}:{VariableDeclarator(p){if(p.node.id.name===name){assert(!found);found=p.node.init;}}});assert(found,file+':'+name);const text=originals[file].toString().slice(found.start,found.end);extracts[file+':'+name]={file,start:found.start,end:found.end,startLine:found.loc.start.line,endLine:found.loc.end.line,sha256:sha(text),typescript:text};return text;}
function evaluate(file,name,deps={},declaration=false){const js=stripTypeScriptTypes('('+expression(file,name,declaration)+')');extracts[file+':'+name].javascript=js;return new Function(...Object.keys(deps),'"use strict";return '+js)(...Object.values(deps));}
const Fragment=Symbol('Fragment');
const jsx=(type,props,key)=>({type,props:props??{},key:key??null});
const runtime={jsx,jsxs:jsx,Fragment};
function tsx(file,name,deps={}){const js=babelTransform('const '+name+' = '+expression(file,name)+';','original-'+name+'.tsx',false,[],[]).code;extracts[file+':'+name].javascript=js;return new Function('require',...Object.keys(deps),js+'\nreturn '+name+';')(p=>{assert(p.replaceAll('\\','/').endsWith('/playwright/jsx-runtime'));return runtime;},...Object.values(deps));}
const hooks='packages/waffle/src/hooks.ts';
const useMemo=f=>f(),useCallback=f=>f,useRef=current=>({current}),useEffect=f=>f();
const theme={background:'#ffffff',legends:{text:{fill:'#333333',fontSize:12},hidden:{text:{opacity:.4},symbol:{opacity:.4}}}};
const motion={animate:false,config:{mass:1,tension:170,friction:26}};
let transitions=[],effects=[],events=[],lastSource,raw=[],customCalls=[],cellCalls=[],legendCalls=[];
const createElement=(type,props)=>jsx(type,props,props?.key);
const common={useMemo,useCallback,useRef,useEffect,createElement,useTheme:()=>theme,Fragment};
const computeCellDimensions=evaluate('packages/grid/src/grid.ts','computeCellDimensions');
const generateGrid=evaluate('packages/grid/src/grid.ts','generateGrid',{computeCellDimensions});
const computeGrid=evaluate(hooks,'computeGrid',{generateGrid});
const mergeCellsData=evaluate(hooks,'mergeCellsData');
const areBoundingBoxTouching=evaluate('packages/grid/src/boundingBoxes.ts','areBoundingBoxTouching');
const perpendicularPolygon=evaluate('packages/grid/src/polygon.ts','perpendicularPolygon',{},true);
const getCellsPolygons=evaluate('packages/grid/src/polygon.ts','getCellsPolygons',{areBoundingBoxTouching,perpendicularPolygon});
const findPolygons=evaluate(hooks,'findPolygons',{getCellsPolygons});
const isDataCell=evaluate('packages/waffle/src/types.ts','isDataCell');
const useAreaPathGenerator=evaluate(hooks,'useAreaPathGenerator',{useMemo,d3Line,curveLinearClosed});
const useTooltip=()=>({showTooltipFromEvent:(node,event)=>{assert.equal(node.props.data.data,raw[node.props.data.groupIndex]);events.push({kind:'tooltip-show',groupIndex:node.props.data.groupIndex,event,fullComputedReference:node.props.data===lastSource.computedData[node.props.data.groupIndex]});},hideTooltip:()=>events.push({kind:'tooltip-hide'})});
const useAreaMouseHandlers=evaluate(hooks,'useAreaMouseHandlers',{...common,useTooltip});
const useTransition=(cells,request)=>{transitions.push({keys:cells.map(request.keys),initial:cells.map(request.initial),enter:cells.map(request.enter),update:cells.map(request.update),trail:request.trail,immediate:request.immediate,config:request.config});return render=>cells.map(cell=>render(request.update(cell),cell));};
const useAnimatedCells=evaluate(hooks,'useAnimatedCells',{useMemo,useMotionConfig:()=>motion,useTransition});
const WaffleCell=tsx('packages/waffle/src/WaffleCell.tsx','WaffleCell',{animated:{rect:'rect'}});
const WaffleTooltip=tsx('packages/waffle/src/WaffleTooltip.tsx','WaffleTooltip',{BasicTooltip:'basic-tooltip'});
const commonDefaultProps=evaluate('packages/waffle/src/defaults.ts','commonDefaultProps',{WaffleTooltip});
const svgDefaultProps=evaluate('packages/waffle/src/defaults.ts','svgDefaultProps',{commonDefaultProps,WaffleCell,WaffleTooltip});
assert.deepEqual(svgDefaultProps.layers,['cells','areas','legends']);
const rawUseWaffle=evaluate(hooks,'useWaffle',{...common,useValueFormatter:f=>f,useOrdinalColorScale:f=>f,useInheritedColor:f=>f,commonDefaultProps,bindDefs:(defs,data,fill)=>{assert.deepEqual(defs,[]);assert.deepEqual(fill,[]);return[];},computeGrid,mergeCellsData,findPolygons,isDataCell});
const useWaffle=p=>{lastSource=rawUseWaffle(p);return lastSource;};
const useCustomLayerProps=evaluate(hooks,'useCustomLayerProps',{useMemo});
const WaffleCells=tsx('packages/waffle/src/WaffleCells.tsx','WaffleCells',{useAnimatedCells,createElement});
const WaffleArea=tsx('packages/waffle/src/WaffleArea.tsx','WaffleArea',{useAreaMouseHandlers});
const WaffleAreas=tsx('packages/waffle/src/WaffleAreas.tsx','WaffleAreas',{useAreaPathGenerator,WaffleArea});
const defaultMargin=evaluate('packages/core/src/defaults/index.js','defaultMargin');
const useDimensions=evaluate('packages/core/src/hooks/useDimensions.js','useDimensions',{useMemo,defaultMargin});
const SvgWrapper=tsx('packages/core/src/components/SvgWrapper.js','SvgWrapper',{forwardRef:f=>f,useTheme:()=>theme,Defs:'defs'});
const legendDefaults=evaluate('packages/legends/src/defaults.ts','legendDefaults');
const isObject=evaluate('packages/legends/src/compute.ts','isObject'),zeroPadding=evaluate('packages/legends/src/compute.ts','zeroPadding');
const computeDimensions=evaluate('packages/legends/src/compute.ts','computeDimensions',{isObject,zeroPadding});
const computePositionFromAnchor=evaluate('packages/legends/src/compute.ts','computePositionFromAnchor');
const computeItemLayout=evaluate('packages/legends/src/compute.ts','computeItemLayout');
const symbols={};for(const name of ['SymbolSquare','SymbolCircle','SymbolDiamond','SymbolTriangle'])symbols[name]=tsx('packages/legends/src/svg/symbols/'+name+'.tsx',name);
const LegendSvgItem=tsx('packages/legends/src/svg/LegendSvgItem.tsx','LegendSvgItem',{useState:v=>[v,next=>effects.push({kind:'legend-state-request',next})],useCallback,useTheme:()=>theme,Text:'text',legendDefaults,computeItemLayout,symbolByShape:{square:symbols.SymbolSquare,circle:symbols.SymbolCircle,diamond:symbols.SymbolDiamond,triangle:symbols.SymbolTriangle},React:{createElement}});
const LegendSvg=tsx('packages/legends/src/svg/LegendSvg.tsx','LegendSvg',{LegendSvgItem,legendDefaults,computeDimensions});
const BoxLegendSvg=tsx('packages/legends/src/svg/BoxLegendSvg.tsx','BoxLegendSvg',{LegendSvg,legendDefaults,computeDimensions,computePositionFromAnchor});
const InnerWaffle=tsx('packages/waffle/src/Waffle.tsx','InnerWaffle',{...common,useDimensions,useWaffle,useCustomLayerProps,svgDefaultProps,WaffleCells,WaffleAreas,BoxLegendSvg,SvgWrapper});
const nodeChildren=n=>n?.props?.children===undefined?[]:Array.isArray(n.props.children)?n.props.children:[n.props.children];
function expand(node,role=null){if(node===null||node===undefined||node===false)return[];if(Array.isArray(node))return node.flatMap(n=>expand(n,role));if(typeof node!=='object')return[{type:'#text',value:node,role}];if(node.type===Fragment)return expand(nodeChildren(node),role);if(typeof node.type==='function'){if(node.type===WaffleCells)role='cells';else if(node.type===WaffleAreas)role='areas';else if(node.type===BoxLegendSvg){role='legends';assert.equal(node.props.data,lastSource.legendData);legendCalls.push({containerWidth:node.props.containerWidth,containerHeight:node.props.containerHeight,sourceReference:true});}else if(node.type===CustomLayer||node.type===EmptyCustom)role='custom';return expand(node.type(node.props),role);}const props={...node.props};delete props.children;return[{type:node.type,props,key:node.key,role,children:expand(nodeChildren(node),role)}];}
function flatten(nodes){return nodes.flatMap(n=>[n,...flatten(n.children??[])]);}
function CustomLayer(props){assert.deepEqual(Object.keys(props).sort(),['cells','computedData']);assert.equal(props.cells,lastSource.cells);assert.equal(props.computedData,lastSource.computedData);props.computedData.forEach((g,i)=>assert.equal(g.data,raw[i]));customCalls.push({cells:props.cells.length,groups:props.computedData.length,rawObjectIdentity:true,wholePools:true,firstRawBits:raw[0]?bits(raw[0].value):null});return jsx('g',{'data-custom':'summary',children:[jsx('text',{children:props.computedData.map(g=>g.data.extra.status).join('|')}),jsx('foreignObject',{x:4,y:5,width:60,height:20,children:jsx('button',{children:'owned application content'})})]});}
function EmptyCustom(props){assert.equal(props.cells,lastSource.cells);assert.equal(props.computedData,lastSource.computedData);customCalls.push({kind:'null-result',wholePools:true});return null;}
function CellProbe(props){assert(lastSource.cells.includes(props.cell));if(isDataCell(props.cell)){assert.equal(props.cell.data,lastSource.computedData[props.cell.data.groupIndex]);assert.equal(props.cell.data.data,raw[props.cell.data.groupIndex]);}else assert(!Object.hasOwn(props.cell,'data'));cellCalls.push({key:props.cell.key,empty:!isDataCell(props.cell),rawObjectIdentity:!isDataCell(props.cell)||props.cell.data.data===raw[props.cell.data.groupIndex],propsKeys:Object.keys(props).sort()});return WaffleCell(props);}
function bits(v){const b=Buffer.alloc(8);b.writeDoubleBE(v);return b.toString('hex');}
const canonical=v=>JSON.parse(JSON.stringify(v,(_k,v)=>typeof v==='function'?{$function:v.name||'anonymous'}:typeof v==='symbol'?{$symbol:v.description}:typeof v==='number'&&Object.is(v,-0)?{$number:'-0'}:v));
const rawData=()=>[{id:'zero',label:'',value:-0,extra:{status:'zero',rawMetric:-0}},{id:'a',label:'<A>',value:4,extra:{status:'ready',rawMetric:-0}},{id:'hidden',label:'Hidden',value:3,extra:{status:'hidden',rawMetric:9}},{id:'b',label:'Bee',value:4,extra:{status:'late',rawMetric:7}},{id:'overflow',label:'Tail',value:8,extra:{status:'overflow',rawMetric:11}}];
const freeze=x=>{if(x&&typeof x==='object'){Object.values(x).forEach(freeze);Object.freeze(x);}return x;};
function contains(p,polygon){let yes=false;for(let i=0,j=polygon.length-1;i<polygon.length;j=i++){const a=polygon[i],b=polygon[j];if((a[1]>p[1])!==(b[1]>p[1])&&p[0]<(b[0]-a[0])*(p[1]-a[1])/(b[1]-a[1])+a[0])yes=!yes;}return yes;}
function gapWitness(cells,padding){for(const a of cells)for(const b of cells){if(a===b||!isDataCell(a)||a.data!==b.data)continue;let point;if(a.x+a.width===b.x&&a.y===b.y)point=[b.x,a.y+a.height/2];else if(a.y+a.height===b.y&&a.x===b.x)point=[a.x+a.width/2,b.y];if(!point)continue;const padded=cells.some(c=>point[0]>c.x+padding/2&&point[0]<c.x+c.width-padding/2&&point[1]>c.y+padding/2&&point[1]<c.y+c.height-padding/2);const region=a.data.polygons.some(p=>contains(point,p));assert(!padded&&region);return{point,groupIndex:a.data.groupIndex,rawID:a.data.id,paddedCellInterior:false,originalGroupPolygonInterior:true,scope:'Independent Euclidean containment of original grid/polygon output; not native DOM dispatch proof.'};}return null;}
const sequences=[undefined,[],['cells'],['areas'],['legends'],['legends','areas','cells'],[CustomLayer,'cells',CustomLayer],['cells','areas','areas','legends','cells'],['areas',CustomLayer,'cells'],['cells',CustomLayer,'areas'],['unknown-layer'],[EmptyCustom,'areas','legends']];
const outputs=[],sourceCases=[];let eventCalls=0,gapCases=0,sourceBuilds=0;
for(const direction of ['top','right','bottom','left'])for(const state of ['full','sparse','all-hidden','empty'])for(const interactive of [true,false])for(let layerCase=0;layerCase<sequences.length;layerCase++){
 raw=state==='empty'?[]:freeze(rawData());transitions=[];effects=[];events=[];customCalls=[];cellCalls=[];legendCalls=[];sourceBuilds++;
 const counts={color:0,format:0,border:0,forwardLegend:0};
 const callbacks={};for(const [prop,name]of [['onMouseEnter','enter'],['onMouseMove','move'],['onMouseLeave','leave'],['onClick','click']])callbacks[prop]=(data,event)=>{assert.equal(data,lastSource.computedData[data.groupIndex]);assert.equal(data.data,raw[data.groupIndex]);events.push({kind:'hook-'+name,groupIndex:data.groupIndex,event,rawReference:true});};
 const props={width:500,height:340,margin:{top:23,right:31,bottom:37,left:41},data:raw,total:state==='sparse'?32:16,rows:3,columns:4,fillDirection:direction,padding:8,hiddenIds:state==='all-hidden'?raw.map(d=>d.id):['hidden'],colors:d=>{assert.equal(d,raw[counts.color++]);return ['#112233','#ab2233','#445566','#2277aa','#66aa33'][raw.indexOf(d)];},valueFormat:v=>{assert(Object.is(v,raw[counts.format++].value));return Object.is(v,-0)?'raw:-0':'raw:'+v;},borderColor:d=>{counts.border++;return d.data?'#445566':'#888888';},forwardLegendData:data=>{counts.forwardLegend++;assert.equal(data.length,raw.length);},defs:[],fill:[],isInteractive:interactive,tooltip:WaffleTooltip,...callbacks,cellComponent:CellProbe,borderWidth:2,borderRadius:3,motionStagger:5,testIdPrefix:'reference',legends:[{anchor:'top-right',direction:'column',itemWidth:100,itemHeight:18,symbolShape:'square',symbolSize:8,symbolSpacing:4,onClick:(d,e)=>{assert(lastSource.legendData.includes(d));events.push({kind:'legend-click',id:d.id,event:e});},toggleSerie:id=>events.push({kind:'legend-toggle',id})}]};
 if(sequences[layerCase]!==undefined)props.layers=sequences[layerCase];
 motion.animate=interactive;
 const tree=InnerWaffle(props),literal=props.layers??svgDefaultProps.layers,occurrences=nodeChildren(tree),alias=[];
 assert.equal(occurrences.length,literal.length);
 for(let i=0;i<literal.length;i++){const l=literal[i];if(typeof l==='string'&&!['cells','areas','legends'].includes(l)){assert.equal(occurrences[i],null);continue;}if(typeof l==='function'){assert.equal(occurrences[i].type,Fragment);assert.equal(occurrences[i].key,i);}else{assert.equal(occurrences[i].key,l);for(let j=0;j<i;j++)if(l===literal[j]){assert.equal(occurrences[i],occurrences[j]);alias.push([j,i]);}}}
 const expanded=expand(tree),nodes=flatten(expanded),cellRects=nodes.filter(n=>n.role==='cells'&&n.type==='rect'),areaPaths=nodes.filter(n=>n.role==='areas'&&n.type==='path'),legendTexts=nodes.filter(n=>n.role==='legends'&&n.type==='text');
 const cellOccurrences=literal.filter(l=>l==='cells').length,areaOccurrences=literal.filter(l=>l==='areas').length,legendOccurrences=literal.filter(l=>l==='legends').length,customOccurrences=literal.filter(l=>typeof l==='function').length;
 assert.equal(cellRects.length,12*cellOccurrences);assert.equal(cellCalls.length,12*cellOccurrences);assert.equal(customCalls.length,customOccurrences);assert.equal(legendCalls.length,legendOccurrences);assert.equal(legendTexts.length,raw.length*legendOccurrences);
 assert.equal(counts.color,raw.length);assert.equal(counts.format,raw.length);assert.equal(counts.border,raw.length+1);assert.equal(counts.forwardLegend,1);
 const polygonCount=lastSource.computedData.filter(g=>!g.isHidden).reduce((n,g)=>n+g.polygons.length,0);assert.equal(areaPaths.length,polygonCount*areaOccurrences);
 for(const p of areaPaths){assert.equal(p.props.fill,'#000000');assert.equal(p.props.fillOpacity,0);for(const k of ['onMouseEnter','onMouseMove','onMouseLeave','onClick'])assert.equal(typeof p.props[k],interactive?'function':'undefined');}
 for(const c of cellRects){assert.equal(c.props.rx,3);assert.equal(c.props.ry,3);assert.equal(c.props.strokeWidth,2);for(const k of ['onMouseEnter','onMouseMove','onMouseLeave','onClick'])assert.equal(c.props[k],undefined);}
 for(const legend of legendCalls){assert.equal(legend.containerWidth,500);assert.equal(legend.containerHeight,340);}
 const firstArea=areaPaths[0];if(firstArea&&interactive){for(const [name,kind]of [['onMouseEnter','enter'],['onMouseMove','move'],['onMouseLeave','leave'],['onClick','click']]){const event={kind,authored:true,eventIdentity:events.length};firstArea.props[name](event);assert.equal(events.at(-1).event,event);eventCalls++;}assert.deepEqual(events.map(e=>e.kind),['tooltip-show','hook-enter','tooltip-show','hook-move','tooltip-hide','hook-leave','tooltip-hide','hook-click']);}
 const legendRect=nodes.find(n=>n.role==='legends'&&n.type==='rect'&&typeof n.props.onClick==='function');if(legendRect){const event={kind:'legend-click',authored:true};legendRect.props.onClick(event);assert.deepEqual(events.slice(-2).map(e=>e.kind),['legend-click','legend-toggle']);}
 const gap=gapWitness(lastSource.cells,8);if(gap)gapCases++;
 const identity=lastSource.computedData.map((g,i)=>({index:i,rawReference:g.data===raw[i],rawValueBits:bits(g.data.value),extraReference:g.data.extra===raw[i].extra}));assert(identity.every(r=>r.rawReference&&r.extraReference));
 if(interactive&&layerCase===0)sourceCases.push(canonical({direction,state,dimensions:useDimensions(500,340,props.margin),input:raw,cells:lastSource.cells,computedData:lastSource.computedData,legendData:lastSource.legendData,identity,gap}));
 outputs.push(canonical({direction,state,interactive,layerCase,layers:literal.map(l=>typeof l==='function'?'custom:'+l.name:l),literalOrder:occurrences.map(n=>n===null?null:n.key),builtinRepeatedSameDescriptor:alias,counts:{...counts,cells:cellRects.length,areaPaths:areaPaths.length,legendText:legendTexts.length,customCalls:customCalls.length},customCalls,cellCalls,transitionRequests:transitions,areas:areaPaths.map(p=>({key:p.key,d:p.props.d,fill:p.props.fill,fillOpacity:p.props.fillOpacity,interactive:typeof p.props.onClick==='function'})),legendCalls,legendTexts:legendTexts.map(n=>({x:n.props.x,y:n.props.y,text:n.children.map(c=>c.value).join(''),style:n.props.style})),events,gap,identity}));
}
const documents={'outputs.json':outputs,'source-cases.json':sourceCases,'extracts.json':extracts};
// A compact lossless numeric table for Rust tests, generated from the actual
// original function results above, rather than copied from Kartis output.
const table=[];
sourceCases.forEach((c,caseIndex)=>{
 table.push(['CASE',caseIndex,c.direction,c.state].join('\t'));
 c.cells.forEach(cell=>table.push(['CELL',caseIndex,cell.index,cell.row,cell.column,cell.data?.groupIndex??-1,cell.x,cell.y,cell.width,cell.height].join('\t')));
 c.computedData.forEach(g=>{
  table.push(['GROUP',caseIndex,g.groupIndex,g.isHidden?1:0,g.startAt,g.endAt,g.polygons.length].join('\t'));
  g.polygons.forEach((polygon,polygonIndex)=>table.push(['POLYGON',caseIndex,g.groupIndex,polygonIndex,...polygon.flat()].join('\t')));
 });
});
assert.equal(outputs.length,384);assert.equal(sourceCases.length,16);assert(gapCases>0);
for(const [name,b]of Object.entries(originals)){const path=resolve(out,'original',name);await mkdir(dirname(path),{recursive:true});await writeFile(path,b,{flag:'wx'});}
const dependencies={};for(const name of ['d3-shape','d3-path']){const entry=require.resolve(name),base=dirname(dirname(entry)),files=['package.json','LICENSE'];async function walk(dir){for(const e of await readdir(dir,{withFileTypes:true})){const p=resolve(dir,e.name);if(e.isDirectory())await walk(p);else if(/\.[cm]?js$/.test(e.name))files.push(relative(base,p).replaceAll('\\','/'));}}await walk(dirname(entry));dependencies[name]={version:JSON.parse(await readFile(resolve(base,'package.json'))).version,files:{}};for(const f of new Set(files)){const bytes=await readFile(resolve(base,f));dependencies[name].files[f]=sha(bytes);const path=resolve(out,'dependencies',name,f);await mkdir(dirname(path),{recursive:true});await writeFile(path,bytes,{flag:'wx'});}}
const hashes={};for(const [file,value]of Object.entries(documents)){const text=JSON.stringify(value,null,2)+'\n';await writeFile(resolve(out,file),text,{flag:'wx'});hashes[file]={sha256:sha(text),bytes:Buffer.byteLength(text)};}
const tsv=table.join('\n')+'\n';await writeFile(resolve(out,'cases.tsv'),tsv,{flag:'wx'});hashes['cases.tsv']={sha256:sha(tsv),bytes:Buffer.byteLength(tsv)};
if(flags.nivo)assert.equal(git('rev-parse','HEAD'),commit);for(const name of names)assert.equal(sha(await readFile(resolve(checkout,name))),sources[name].sha256);
const report={passed:true,featureAccepted:false,broadGoalComplete:false,commit,version:'0.99.0',scope:{executed:'Original InnerWaffle literal loop, WaffleCells/Cell/Areas/Area, full useWaffle, grid/polygon algorithms, area event callbacks, default SvgWrapper, BoxLegendSvg/LegendSvg/LegendSvgItem and symbol render functions. Real D3 line and original grid polygons.',adapters:'Cold useMemo/useRef/useEffect/useCallback, request-recording useTransition with current target values, JSX descriptor runtime without React key coercion; authored color/formatter callbacks and empty defs binding; static theme and Text token; no palette implementation or text layout.',excluded:'No React runtime/reconciliation/duplicate-key behavior, actual ReactSpring dynamics, browser/DOM/event dispatch/occlusion/CSS/native SVG hit testing, legend re-render after setState, Canvas, Rust or feature acceptance. Gap coverage is explicit pure geometry containment, not browser input.'},counts:{profiles:outputs.length,sourceCases:sourceCases.length,sourceBuilds,eventFunctionCalls:eventCalls,gapWitnessCases:gapCases,customContexts:outputs.reduce((n,o)=>n+o.counts.customCalls,0),cellComponentCalls:outputs.reduce((n,o)=>n+o.cellCalls.length,0)},sources,dependencies,artifacts:hashes,runnerSha256:sha(await readFile(self)),vendoredSourceIndexSha256:sha(await readFile(resolve(fixtures,'source-index.json'))),sourceInput:flags.nivo?'explicit-git-checkout':'vendored-original',compiler:{node:process.version,nodeExecutableSha256:sha(await readFile(process.execPath)),playwright:JSON.parse(await readFile(resolve(root,'node_modules/playwright/package.json'))).version,babelBundleSha256:sha(await readFile(babelPath)),babelImplementationSha256:sha(await readFile(resolve(dirname(babelPath),'babelBundleImpl.js')))}};
await writeFile(resolve(out,'reference.json'),JSON.stringify(report,null,2)+'\n',{flag:'wx'});await writeFile(resolve(out,'executed-reference.mjs'),await readFile(self),{flag:'wx'});
console.log(JSON.stringify({passed:true,out,...report.counts}));
