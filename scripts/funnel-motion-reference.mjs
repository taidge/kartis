// Original Nivo request/keys/path execution and an independent analytic oracle.
// Importing the helpers starts no process, browser, timer or source checkout.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {execFileSync} from 'node:child_process';
import {readFile,readdir,mkdir,writeFile} from 'node:fs/promises';
import {createRequire,stripTypeScriptTypes} from 'node:module';
import {dirname,relative,resolve} from 'node:path';
import {fileURLToPath} from 'node:url';
import {interpolateString} from 'd3-interpolate';
import {area,line,curveBasis,curveLinear} from 'd3-shape';
import omit from 'lodash/omit.js';
import {computeShapeGenerators} from '../tests/fixtures/funnel-motion/shape-generators.mjs';

export function spring(state,target,dt,config={mass:1,stiffness:170,damping:26}) {
    const {mass:m,stiffness:k,damping:c}=config,{value:x,velocity:v}=state;
    assert([m,k,c,x,v,target,dt].every(Number.isFinite)&&m>0&&k>0&&c>=0&&dt>=0);
    if(dt===0)return {value:x,target,velocity:v};
    const a=c/(2*m),w2=k/m,y=x-target,disc=a*a-w2;
    let next,velocity;
    if(Math.abs(disc)<=1e-12*Math.max(1,a*a,w2)) {
        const b=v+a*y,e=Math.exp(-a*dt);next=e*(y+b*dt);velocity=e*(b-a*(y+b*dt));
    } else if(disc<0) {
        const w=Math.sqrt(-disc),co=Math.cos(w*dt),si=Math.sin(w*dt),b=(v+a*y)/w,e=Math.exp(-a*dt);
        next=e*(y*co+b*si);velocity=e*(-a*(y*co+b*si)-y*w*si+b*w*co);
    } else {
        const d=Math.sqrt(disc),r1=-a+d,r2=-a-d,b1=(v-r2*y)/(r1-r2),b2=y-b1,e1=Math.exp(r1*dt),e2=Math.exp(r2*dt);
        next=b1*e1+b2*e2;velocity=r1*b1*e1+r2*b2*e2;
    }
    return {value:target+next,target,velocity};
}
export const springAtRest=(state,target,config={position_epsilon:.0001,velocity_epsilon:.0001})=>Math.abs(state.value-target)<=config.position_epsilon&&Math.abs(state.velocity)<=config.velocity_epsilon;
export const pathInterpolate=(previousTarget,nextTarget,progress)=>interpolateString(previousTarget,nextTarget)(progress);
export function targetRecipe({part,direction='vertical',interpolation='smooth'}) {
    const [body,border]=computeShapeGenerators(interpolation,direction);
    return {areaPath:body(part.areaPoints),borderPath:border(part.borderPoints),style:{areaColor:part.color,borderWidth:part.borderWidth,borderColor:part.borderColor},label:{transform:`translate(${part.x}, ${part.y})`,color:part.labelColor},targetOnly:{fillOpacity:part.fillOpacity,borderOpacity:part.borderOpacity,borderMounted:part.borderWidth>0,formattedValue:part.formattedValue}};
}

const script=fileURLToPath(import.meta.url),root=resolve(dirname(script),'..'),sha=b=>createHash('sha256').update(b).digest('hex');
const canonical=value=>JSON.parse(JSON.stringify(value,(_k,v)=>typeof v==='function'?{$function:v.name||'anonymous'}:typeof v==='symbol'?{$symbol:v.description}:typeof v==='number'&&Object.is(v,-0)?{$number:'-0'}:v));
async function main() {
    const args=Object.fromEntries(process.argv.slice(2).map(s=>{const i=s.indexOf('=');assert(i>2);return[s.slice(2,i),s.slice(i+1)];}));assert.deepEqual(Object.keys(args).sort(),['nivo','out']);
    const checkout=resolve(args.nivo),out=resolve(args.out),fixture=resolve(root,'tests/fixtures/funnel-motion'),index=JSON.parse(await readFile(resolve(fixture,'source-index.json')));
    const git=(...a)=>execFileSync('git',['-C',checkout,...a],{encoding:'utf8'}).trim();assert.equal(git('rev-parse','HEAD'),index.commit);
    const original={},asts={},extracts={};
    const require=createRequire(import.meta.url),{babelParse,babelTransform,traverse}=require(resolve(root,'node_modules/playwright/lib/transform/babelBundle.js'));
    for(const [name,info]of Object.entries(index.files)) {
        const b=await readFile(resolve(checkout,name));assert.equal(sha(b),info.sha256);assert(b.equals(execFileSync('git',['-C',checkout,'show',`${index.commit}:${name}`])));assert(b.equals(await readFile(resolve(fixture,'original',name))));original[name]=b;
        if(/\.[jt]sx?$/.test(name))asts[name]=babelParse(b.toString(),'original-'+name,true);
    }
    function expression(file,name) {
        let found;traverse(asts[file],{VariableDeclarator(p){if(p.node.id.name===name){assert(!found);found=p.node.init;}}});assert(found,file+':'+name);
        const text=original[file].toString().slice(found.start,found.end);extracts[file+':'+name]={file,start:found.start,end:found.end,startLine:found.loc.start.line,endLine:found.loc.end.line,sha256:sha(text),typescript:text};return text;
    }
    function evaluate(file,name,deps={}) {const js=stripTypeScriptTypes('('+expression(file,name)+')');extracts[file+':'+name].javascript=js;return new Function(...Object.keys(deps),`"use strict";return ${js}`)(...Object.values(deps));}
    const Fragment=Symbol('Fragment'),jsx=(type,props,key)=>({type,props,key:key??null}),runtime={jsx,jsxs:jsx,Fragment};
    function tsx(file,name,deps={}) {const js=babelTransform('const '+name+' = '+expression(file,name)+';','original-'+name+'.tsx',false,[],[]).code;extracts[file+':'+name].javascript=js;return new Function('require',...Object.keys(deps),js+'\nreturn '+name+';')(name=>{assert(name.replaceAll('\\','/').endsWith('/playwright/jsx-runtime'));return runtime;},...Object.values(deps));}
    const shapeExpression=expression('packages/funnel/src/hooks.ts','computeShapeGenerators'),generated='// Generated from the Git-exact original Nivo function; source range is recorded.\nimport {area,line,curveBasis,curveLinear} from \'d3-shape\';\n'+stripTypeScriptTypes('export const computeShapeGenerators = '+shapeExpression+';')+'\n';
    assert.equal(await readFile(resolve(fixture,'shape-generators.mjs'),'utf8'),generated);
    const motion={animate:true,config:{mass:1,tension:170,friction:26,authoredProbe:'request only, not React Spring integration'}};
    function pathHook() {
        const ref={current:undefined},requests=[];let pending=[],progress=0,interpolator;
        const usePrevious=evaluate('packages/core/src/hooks/useAnimatedPath.js','usePrevious',{useRef:()=>ref,useEffect:f=>pending.push(f)});
        const hook=evaluate('packages/core/src/hooks/useAnimatedPath.js','useAnimatedPath',{useMotionConfig:()=>motion,usePrevious,useMemo:f=>f(),interpolateString,useSpring:r=>{requests.push(canonical(r));return{value:progress};},to:(v,i)=>{interpolator=i;return i(v);}});
        return {render(path,p){progress=p;pending=[];const previous=ref.current,value=hook(path);return{previous,value,request:requests.at(-1),samples:[-.25,0,.375,1,1.25].map(t=>({progress:t,path:interpolator(t)}))};},commit(){pending.forEach(f=>f());}};
    }
    function authoredPart(direction,phase) {
        const x0=30+phase*11,x1=170+phase*23,y0=20+phase*7,y1=120+phase*13;
        const before=[{x:x0,y:y0},{x:x0+5,y:y0+25},{x:x0+15,y:y1-20},{x:x0+20,y:y1}],after=before.map((p,i)=>({x:x1-i*7,y:p.y}));
        const points=before.map((p,i)=>direction==='vertical'?{x0:p.x,x1:after[i].x,y:p.y}:{x:p.y,y0:p.x,y1:after[i].x});
        const orient=p=>direction==='vertical'?p:{x:p.y,y:p.x};
        return {data:{id:'stage',value:12,extra:{phase}},x:100+phase*20,y:70+phase*15,color:phase?'#23a967':'#d43245',borderColor:phase?'#8643aa':'#223344',borderWidth:phase===2?0:6+phase*2,fillOpacity:.5,borderOpacity:.66,labelColor:phase?'#112233':'#ffffff',formattedValue:'phase-'+phase,areaPoints:points,borderPoints:[...before.map(orient),null,...after.map(orient)]};
    }
    const pathCases=[];
    for(const direction of ['vertical','horizontal'])for(const interpolation of ['linear','smooth']) {
        const targets=[0,1,2].map(p=>targetRecipe({part:authoredPart(direction,p),direction,interpolation})),paths={};
        for(const channel of ['areaPath','borderPath']) {
            const h=pathHook(),cold=h.render(targets[0][channel],0);assert(cold.samples.every(s=>s.path===targets[0][channel]));h.commit();
            const update=h.render(targets[1][channel],.375);h.commit();const retarget=h.render(targets[2][channel],0);
            assert.equal(retarget.previous,targets[1][channel]);assert.equal(retarget.value,pathInterpolate(targets[1][channel],targets[2][channel],0));assert.notEqual(update.value,targets[1][channel]);
            for(const r of [cold,update,retarget])assert(r.request.reset===true&&r.request.from.value===0&&r.request.to.value===1&&r.request.immediate===false);
            paths[channel]=canonical({cold,update,retarget,displayedSampleIsNotRetargetSource:update.value!==retarget.previous});
        }
        pathCases.push({direction,interpolation,targets,paths});
    }
    const springs=[],theme={grid:{line:{stroke:'#555555',strokeWidth:2}},labels:{text:{fontSize:13}},annotations:{link:{stroke:'#555555',strokeWidth:1,outlineWidth:2,outlineColor:'#ffffff',outlineOpacity:1},outline:{stroke:'#222222',strokeWidth:2,outlineWidth:2,outlineColor:'#ffffff',outlineOpacity:1},symbol:{fill:'#f01e50'},text:{fill:'#111111',outlineWidth:2,outlineColor:'#ffffff'}}};
    const useSpring=request=>{springs.push(canonical(request));return Object.fromEntries(Object.keys(request).filter(k=>!['config','immediate'].includes(k)).map(k=>[k,'SPRING('+k+')']));},common={useMotionConfig:()=>motion,useSpring,useTheme:()=>theme,animated:{path:'path',g:'g',line:'line',circle:'circle',rect:'rect',text:'text'},Text:'text'};
    const componentCases=[];
    for(const animate of [true,false]) {
        motion.animate=animate;
        for(const [file,name,input,extra]of [
            ['funnel','Part',{part:authoredPart('vertical',1),areaGenerator:computeShapeGenerators('smooth','vertical')[0],borderGenerator:computeShapeGenerators('smooth','vertical')[1]},{useAnimatedPath:p=>'PATH('+p+')'}],
            ['funnel','Part',{part:authoredPart('vertical',2),areaGenerator:computeShapeGenerators('linear','vertical')[0],borderGenerator:computeShapeGenerators('linear','vertical')[1]},{useAnimatedPath:p=>'PATH('+p+')'}],
            ['funnel','Separator',{separator:{partId:'stage',x0:2,x1:31,y0:5,y1:17}},{}],
            ['funnel','PartLabel',{part:authoredPart('vertical',1)},{}],
            ['annotations','CircleAnnotationOutline',{x:101,y:93,size:28},{}],
            ['annotations','DotAnnotationOutline',{x:101,y:93,size:8},{defaultProps:{dotSize:4}}],
            ['annotations','RectAnnotationOutline',{x:101,y:93,width:36,height:22,borderRadius:9},{}],
            ['annotations','AnnotationNote',{datum:{id:'raw-stage'},x:119,y:107,note:'target text'},{omit,createElement:jsx}],
            ['annotations','AnnotationNote',{datum:{id:'raw-stage'},x:119,y:107,note:function CustomNote(){}},{omit,createElement:jsx}],
            ['annotations','AnnotationLink',{points:[[4,8],[17,23],[42,23]],isOutline:false},{useMemo:f=>f(),useAnimatedPath:p=>'PATH('+p+')'}],
        ]) {
            const inputDatum=input.datum,start=springs.length;
            // The custom createElement assertion above closes over this case's
            // exact original datum, rather than comparing serialized copies.
            if(name==='AnnotationNote'&&typeof input.note==='function')extra.createElement=(type,props)=>{assert.equal(props.x,input.x);assert.equal(props.y,input.y);assert.equal(props.datum,inputDatum);return jsx(type,props);};
            const fn=tsx(`packages/${file}/src/${name}.tsx`,name,{...common,...extra}),tree=fn(input),requests=springs.slice(start);
            for(const r of requests)assert.equal(r.immediate,!animate);
            componentCases.push(canonical({name,animate,input,requests,tree}));
        }
    }
    const keyCases=[];
    for(const ids of [['a','b',1,'1'],['b','a'],['b','c']]) {
        const parts=ids.map(id=>({data:{id}})),separators=ids.map(partId=>({partId})).concat([{partId:'none'}]);
        for(const [name,input,deps]of [['Parts',{parts,areaGenerator:'A',borderGenerator:'B'},{Part:'part-probe'}],['PartLabels',{parts},{PartLabel:'label-probe'}],['Separators',{beforeSeparators:separators,afterSeparators:separators},{Separator:'separator-probe'}],['FunnelAnnotations',{parts,annotations:ids.map(id=>({id}))},{useFunnelAnnotations:(_parts,a)=>a,Annotation:'annotation-probe'}]]) {
            keyCases.push(canonical({name,ids,tree:tsx('packages/funnel/src/'+name+'.tsx',name,deps)(input)}));
        }
    }
    const mathCases=[];
    for(const config of [{mass:1,stiffness:170,damping:26},{mass:1,stiffness:100,damping:20},{mass:1,stiffness:100,damping:35},{mass:2,stiffness:70,damping:0}])for(const dt of [0,.016,.08,.25,.75]) {
        const state={value:-3.5,velocity:7.25},target=11,next=spring(state,target,dt,config),split=spring(spring(state,target,dt/2,config),target,dt/2,config);
        assert(Math.abs(next.value-split.value)<1e-10&&Math.abs(next.velocity-split.velocity)<1e-10);mathCases.push({config,state,target,dt,next,split});
    }
    const topologyCases=[['M0,0L10,20Z','M30,40C50,60,70,80,90,100Z'],['M0,0L10,20','M1e2,-3.5L4,5'],[undefined,'M3,4L8,9']].map(([previousTarget,nextTarget])=>({previousTarget:previousTarget??null,nextTarget,samples:[-.25,0,.5,1,1.25].map(progress=>({progress,path:pathInterpolate(previousTarget,nextTarget,progress)}))}));
    await mkdir(out,{recursive:false});const dependencies={};
    for(const name of ['d3-shape','d3-path','d3-interpolate','d3-color','lodash']) {
        const entry=require.resolve(name),base=name==='lodash'?dirname(entry):dirname(dirname(entry)),files=['package.json','LICENSE'];
        async function walk(dir){for(const e of await readdir(dir,{withFileTypes:true})){const p=resolve(dir,e.name);if(e.isDirectory())await walk(p);else if(/\.[cm]?js$/.test(e.name))files.push(relative(base,p).replaceAll('\\','/'));}}await walk(name==='lodash'?base:dirname(entry));
        dependencies[name]={version:JSON.parse(await readFile(resolve(base,'package.json'))).version,files:{}};
        for(const f of new Set(files)){const b=await readFile(resolve(base,f));original['dependencies/'+name+'/'+f]=b;dependencies[name].files[f]=sha(b);}
    }
    for(const [name,b]of Object.entries(original)){const p=resolve(out,'original',name);await mkdir(dirname(p),{recursive:true});await writeFile(p,b,{flag:'wx'});}
    const outputs={pathCases,componentCases,keyCases,mathCases,topologyCases},payload=JSON.stringify(outputs,null,2)+'\n';await writeFile(resolve(out,'outputs.json'),payload,{flag:'wx'});await writeFile(resolve(out,'extracts.json'),JSON.stringify(extracts,null,2)+'\n',{flag:'wx'});
    const committed=resolve(fixture,'reference.json');try{assert.equal(await readFile(committed,'utf8'),payload);}catch(e){if(e.code!=='ENOENT')throw e;await writeFile(committed,payload,{flag:'wx'});}
    for(const [name,info]of Object.entries(index.files))assert.equal(sha(await readFile(resolve(checkout,name))),info.sha256);assert.equal(git('rev-parse','HEAD'),index.commit);
    const report={passed:true,featureAccepted:false,broadGoalComplete:false,commit:index.commit,sources:index.files,dependencies,counts:{pathProfiles:pathCases.length,pathHookInstances:pathCases.length*2,componentCases:componentCases.length,keyTrees:keyCases.length,mathCases:mathCases.length,topologyCases:topologyCases.length},scope:{executed:'Git-exact original usePrevious/useAnimatedPath, Part/Separator/PartLabel, Parts/Separators/PartLabels/FunnelAnnotations and annotation render functions; real D3 generators and interpolateString.',adapters:'Explicit single-ref effect-commit adapter; authored scalar progress; request-recording useSpring returns sentinel properties; JSX descriptors preserve authored keys without React key coercion; authored bound annotation input; independent analytic oscillator without rest snapping.',excluded:'No React/ReactSpring scheduler or reconciliation, actual preset resolver, Spring trajectory equivalence, browser/native input/Glory/RAF/pixels or product acceptance.'},outputsSha256:sha(payload),extractsSha256:sha(await readFile(resolve(out,'extracts.json'))),runnerSha256:sha(await readFile(script)),compiler:{node:process.version,nodeExecutableSha256:sha(await readFile(process.execPath)),playwright:JSON.parse(await readFile(resolve(root,'node_modules/playwright/package.json'))).version,babelBundleSha256:sha(await readFile(resolve(root,'node_modules/playwright/lib/transform/babelBundle.js'))),babelImplSha256:sha(await readFile(resolve(root,'node_modules/playwright/lib/transform/babelBundleImpl.js')))}};
    await writeFile(resolve(out,'executed-runner.mjs'),await readFile(script),{flag:'wx'});await writeFile(resolve(out,'report.json'),JSON.stringify(report,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify({passed:true,out,...report.counts}));
}
if(process.argv[1]&&resolve(process.argv[1])===script)await main();
