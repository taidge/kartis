// Execute pinned upstream binding/geometry with real Lodash and D3 dependencies.
// useMemo is a cold adapter; this is not a React/Spring runtime.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {execFileSync} from 'node:child_process';
import {readFile,readdir,mkdir,writeFile} from 'node:fs/promises';
import {createRequire,stripTypeScriptTypes} from 'node:module';
import {dirname,resolve,relative} from 'node:path';
import {fileURLToPath} from 'node:url';
import {scaleLinear} from 'd3-scale';

const root=resolve(dirname(fileURLToPath(import.meta.url)),'..');
const flags=Object.fromEntries(process.argv.slice(2).map(v=>{const i=v.indexOf('=');assert(i>2);return[v.slice(2,i),v.slice(i+1)];}));
assert.deepEqual(Object.keys(flags).sort(),['nivo','out']);
const checkout=resolve(flags.nivo),out=resolve(flags.out),sha=b=>createHash('sha256').update(b).digest('hex');
await mkdir(out,{recursive:false});
const git=(...a)=>execFileSync('git',['-C',checkout,...a]),commit=git('rev-parse','HEAD').toString().trim();
const require=createRequire(import.meta.url),{babelParse,traverse}=require(resolve(root,'node_modules/playwright/lib/transform/babelBundle.js'));
const names=['funnel/src/hooks.ts','funnel/src/types.ts','funnel/src/Funnel.tsx','funnel/src/FunnelAnnotations.tsx','funnel/LICENSE.md','funnel/package.json','annotations/src/compute.ts','annotations/src/hooks.ts','annotations/src/types.ts','annotations/src/utils.ts','annotations/src/props.ts','annotations/src/Annotation.tsx','annotations/src/AnnotationNote.tsx','annotations/src/AnnotationLink.tsx','annotations/src/CircleAnnotationOutline.tsx','annotations/src/DotAnnotationOutline.tsx','annotations/src/RectAnnotationOutline.tsx','annotations/src/canvas.ts','annotations/LICENSE.md','annotations/package.json','core/src/lib/polar/utils.js','theming/src/defaults.ts'];
const original={},sources={},asts={},extracts={};
for(const n of names){const b=await readFile(resolve(checkout,'packages',n));assert(b.equals(git('show',`${commit}:packages/${n}`)));original[n]=b;sources[n]=sha(b);if(/\.[jt]sx?$/.test(n))asts[n]=babelParse(b.toString(),n,true);}
assert.equal(JSON.parse(original['annotations/package.json']).version,'0.99.0');
function evaluate(file,name,deps={}){let found;traverse(asts[file],{VariableDeclarator(p){if(p.node.id.name===name){assert(!found);found=p.node.init;}}});assert(found,file+':'+name);const text=original[file].toString().slice(found.start,found.end),js=stripTypeScriptTypes('('+text+')');extracts[file+':'+name]={file,start:found.start,end:found.end,sha256:sha(text),typescript:text,javascript:js};return new Function(...Object.keys(deps),'"use strict";return '+js)(...Object.values(deps));}
const filter=require('lodash/filter.js'),isNumber=require('lodash/isNumber.js'),omit=require('lodash/omit.js');
const lodashRoot=dirname(require.resolve('lodash/package.json'));
assert.equal(JSON.parse(await readFile(resolve(lodashRoot,'package.json'))).version,'4.17.21');
const polar={};for(const n of ['radiansToDegrees','normalizeAngleDegrees','degreesToRadians','positionFromAngle'])polar[n]=evaluate('core/src/lib/polar/utils.js',n);
const guards={};for(const n of ['isCircleAnnotation','isRectAnnotation'])guards[n]=evaluate('annotations/src/utils.ts',n);
const defaults=evaluate('annotations/src/props.ts','defaultProps');
const bindAnnotations=evaluate('annotations/src/compute.ts','bindAnnotations',{filter,omit,...guards});
const getLinkAngle=evaluate('annotations/src/compute.ts','getLinkAngle',polar);
const computeAnnotation=evaluate('annotations/src/compute.ts','computeAnnotation',{isNumber,getLinkAngle,defaultProps:defaults,...guards,...polar});
const useAnnotations=evaluate('annotations/src/hooks.ts','useAnnotations',{useMemo:f=>f(),bindAnnotations});
const useFunnelAnnotations=evaluate('funnel/src/hooks.ts','useFunnelAnnotations',{useAnnotations});
const computeScales=evaluate('funnel/src/hooks.ts','computeScales',{scaleLinear});
function source(phase){let data=['1',1,'zero','transparent','tail'].map((id,i)=>({id,value:[phase===1?10:8,4,phase===1?0:-0,2,0][i],label:`bound-${i}${phase===1?'-new':''}`,size:[180,120,0,90,60][i],extra:{index:i}}));if(phase===2)[data[0],data[1]]=[data[1],data[0]];if(phase===3)data.splice(1,1);return data;}
function parts(profile,phase,current){const data=source(phase),direction=profile%2===0?'vertical':'horizontal',innerWidth=530,innerHeight=360,[bandScale]=computeScales({data,direction,width:innerWidth,height:innerHeight,spacing:0});return evaluate('funnel/src/hooks.ts','parts',{useMemo:f=>f(),data,direction,bandScale,innerWidth,innerHeight,paddingBefore:0,rawShapeBlending:.5,currentPartId:current,getSize:d=>d.size,getColor:()=> '#bf2832',formatValue:String,getBorderColor:()=> '#000',getLabelColor:()=> '#333',borderOpacity:.66,borderWidth:0,currentBorderWidth:undefined,currentPartSizeExtension:12,fillOpacity:1});}
function recipe(r){if(r===0)return[];const make=(match,type,note,noteX,noteY)=>({match,type,note,noteX,noteY});if(r===2)return[[0,-100],[100,-100],[100,0],[100,100],[0,100],[-100,100],[-100,0],[-100,-100]].map(([x,y])=>({...make(['data.id','1'],'rect','eight',x,y),offset:4,noteWidth:24}));return[
 {...make(['data.id','1'],'dot','dot <&>',60,-10),size:12,offset:91,noteWidth:48},
 {...make(['data.id',1],'circle','circle',-70,-12),size:46,offset:3,noteWidth:42},
 {...make(['data.id','zero'],'rect','rect',60,14),offset:6,width:777,height:555,x:999,y:888,borderRadius:3,noteWidth:36},
 {...make(p=>p.data.id==='transparent','dot',function customNote(){},-80,-16),size:0,noteWidth:32},
 {...make(['data.id','tail'],'circle','',{abs:500},{abs:370}),size:0,offset:2,noteWidth:22},
 make(()=>false,'dot','absent',30,30),
 {...make((_p,i)=>i<2,'dot','multi',30,20),size:4,noteWidth:24},
 ];}
const encode=v=>JSON.parse(JSON.stringify(v,(_k,n)=>typeof n==='function'?{$function:n.name||'anonymous'}:typeof n==='number'&&Object.is(n,-0)?{$number:'-0'}:n));
const cases=[],tsv=['# profile phase current(-1/0/1) recipe count; rows spec part kind centerx centery size width height points[6] text[2] angle'];
for(let profile=0;profile<4;profile++)for(let phase=0;phase<4;phase++)for(const[currentIndex,current]of [[-1,null],[0,'1'],[1,1]])for(let r=0;r<3;r++){
 const pool=parts(profile,phase,current),specs=recipe(r),bound=useFunnelAnnotations(pool,specs);let lastSpec=-1,lastPart=-1;
 const rows=bound.map(b=>{const part=pool.indexOf(b.datum);assert(part>=0&&b.datum.data===pool[part].data);const spec=specs.findIndex(s=>s.note===b.note&&s.noteX===b.noteX&&s.noteY===b.noteY);assert(spec>=0);assert(spec>lastSpec||spec===lastSpec&&part>lastPart);lastSpec=spec;lastPart=part;const computed=computeAnnotation(b),offset=([x,y])=>[x+40,y+28];const row={spec,part,kind:b.type,center:offset([b.x,b.y]),size:b.size,width:b.width,height:b.height,points:computed.points.map(offset),text:offset(computed.text),angle:computed.angle};tsv.push([spec,part,b.type,...row.center,b.size,b.width,b.height,...row.points.flat(),...row.text,row.angle].join('\t'));return row;});
 const header=['case',profile,phase,currentIndex,r,bound.length].join('\t');tsv.splice(tsv.length-bound.length,0,header);cases.push({profile,phase,currentIndex,recipe:r,parts:encode(pool),specs:encode(specs),bindings:rows});
}
const matchPool=parts(0,0,null),matchForms=[null,'data.value',['data.id',1],['data.id','1'],{data:{id:1}},{data:{id:'1'}},['data.value',0],{data:{value:-0}}];
const shorthand=matchForms.map(match=>({match:encode(match),indices:filter(matchPool,match).map(p=>matchPool.indexOf(p))}));
assert.deepEqual(shorthand.map(v=>v.indices),[[0,1,2,3,4],[0,1,3],[1],[0],[1],[0],[2,4],[2,4]]);
const degrees=[];for(const kind of ['circle','rect','dot'])for(const d of [0,22.5,45,67.5,90,112.5,135,157.5,180,202.5,225,247.5,270,292.5,315,337.5]){
 const a={type:kind,x:130,y:80,width:70,height:30,size:50,noteX:Math.cos(d*Math.PI/180)*90,noteY:Math.sin(d*Math.PI/180)*90,noteWidth:29,noteTextOffset:7};degrees.push({input:a,computed:computeAnnotation(a)});
}
const dependencySources={};
async function archiveDependency(name){const entry=require.resolve(name),base=name==='lodash'?dirname(entry):dirname(dirname(entry));const files=['package.json','LICENSE'];async function walk(d){for(const e of await readdir(d,{withFileTypes:true})){const p=resolve(d,e.name);if(e.isDirectory())await walk(p);else if(/\.[cm]?js$/.test(e.name))files.push(relative(base,p).replaceAll('\\','/'));}}await walk(name==='lodash'?base:dirname(entry));for(const f of new Set(files)){const b=await readFile(resolve(base,f)),n='dependencies/'+name+'/'+f;original[n]=b;dependencySources[n]=sha(b);}}
for(const n of ['lodash','d3-scale','d3-array','d3-interpolate','d3-color','d3-format','d3-time','d3-time-format','internmap'])await archiveDependency(n);
for(const[n,b]of Object.entries(original)){const p=resolve(out,'original',n);await mkdir(dirname(p),{recursive:true});await writeFile(p,b,{flag:'wx'});}
await writeFile(resolve(out,'executed-runner.mjs'),await readFile(fileURLToPath(import.meta.url)),{flag:'wx'});
const payloads={'outputs.json':JSON.stringify({cases,shorthand,degrees},null,2)+'\n','reference.tsv':tsv.join('\n')+'\n','extracts.json':JSON.stringify(extracts,null,2)+'\n'};
for(const[n,b]of Object.entries(payloads))await writeFile(resolve(out,n),b,{flag:'wx'});
const result={passed:true,featureAccepted:false,broadGoalComplete:false,commit,source:sources,dependencySources,counts:{cases:cases.length,bindings:cases.reduce((s,c)=>s+c.bindings.length,0),shorthand:shorthand.length,angleCases:degrees.length},artifacts:Object.fromEntries(Object.entries(payloads).map(([n,b])=>[n,sha(b)])),adapters:['useMemo evaluates once without React','Functions extracted with exact AST source ranges and Node TypeScript stripping','No React/Spring lifecycle or JSX/Canvas render equivalence'],rootPackageSha256:sha(await readFile(resolve(root,'package.json'))),rootLockSha256:sha(await readFile(resolve(root,'package-lock.json')))};
await writeFile(resolve(out,'reference.json'),JSON.stringify(result,null,2)+'\n',{flag:'wx'});
console.log(JSON.stringify({passed:true,commit,counts:result.counts}));
