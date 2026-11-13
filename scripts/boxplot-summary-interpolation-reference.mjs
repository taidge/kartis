// Actual original Nivo BoxPlot summary-based automatic domains and Date TimeClip.
// node scripts/boxplot-summary-interpolation-reference.mjs <node_modules> [--check]
import fs from 'node:fs';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {stripTypeScriptTypes} from 'node:module';
import {dirname,join,resolve} from 'node:path';
import {fileURLToPath,pathToFileURL} from 'node:url';
const root=resolve(dirname(fileURLToPath(import.meta.url)),'..');
const args=process.argv.slice(2),check=args.includes('--check');
const ri=args.indexOf('--reference-root'),referenceRoot=ri<0?root:resolve(args[ri+1]);
const modules=resolve(args[0]??'');assert(args[0]&&!args[0].startsWith('--'),'Supply a node_modules directory containing d3-scale 4.0.2 and its dependencies');
const vendor=join(referenceRoot,'tests/fixtures/boxplot-domain-bounds/vendor'),bar=join(referenceRoot,'tests/fixtures/bar-category-band/vendor/raw');
const hash=v=>createHash('sha256').update(v).digest('hex'),sourcePins={};
const pinned=JSON.parse(fs.readFileSync(join(vendor,'source-pins.json')));
for(const [name,entry] of Object.entries(pinned.files)){const bytes=fs.readFileSync(join(vendor,name));assert.equal(hash(bytes),entry.sha256);sourcePins['tests/fixtures/boxplot-domain-bounds/vendor/'+name]=entry.sha256;}
const barPins=JSON.parse(fs.readFileSync(join(referenceRoot,'tests/fixtures/bar-category-band/source-and-fixtures.json'))).files;
for(const name of ['nivo-bandScale.ts','nivo-LICENSE','d3-scale-LICENSE','d3-array-LICENSE']){const bytes=fs.readFileSync(join(bar,name));assert.equal(hash(bytes),barPins.find(e=>e.path==='vendor/raw/'+name).sha256);sourcePins['tests/fixtures/bar-category-band/vendor/raw/'+name]=hash(bytes);}
const versions={'d3-scale':'4.0.2','d3-array':'3.2.5','d3-interpolate':'3.0.1','d3-time-format':'4.1.1'};
for(const [name,version] of Object.entries(versions))assert.equal(JSON.parse(fs.readFileSync(join(modules,name,'package.json'))).version,version);
const runtimeSourcePins={};
for(const path of ['d3-scale/src/time.js','d3-scale/src/utcTime.js','d3-scale/src/continuous.js','d3-scale/src/linear.js','d3-scale/src/log.js','d3-interpolate/src/number.js','d3-time-format/dist/d3-time-format.mjs'])runtimeSourcePins[path]=hash(fs.readFileSync(join(modules,path)));
const entry=name=>JSON.stringify(pathToFileURL(join(modules,name,JSON.parse(fs.readFileSync(join(modules,name,'package.json'))).module)).href),read=name=>fs.readFileSync(join(vendor,name),'utf8').replaceAll('\r\n','\n');
const noTypes=s=>s.replace(/^import \{[^}]*\} from '(?:\.\.\/|\.\/)types'\n/gm,'');
const url=s=>'data:text/javascript;base64,'+Buffer.from(stripTypeScriptTypes(noTypes(s),{mode:'transform'})).toString('base64');
const linear=url(read('linearScale.ts').replace(/^import \{[^}]*\} from 'd3-scale'/m,'import {scaleLinear} from '+entry('d3-scale')).replace(/^import \{[^}]*\} from 'd3-interpolate'/m,'import {interpolateRound,interpolateNumber} from '+entry('d3-interpolate')));
const band=url(fs.readFileSync(join(bar,'nivo-bandScale.ts'),'utf8').replaceAll('\r\n','\n').replace(/^import \{[^}]*\} from 'd3-scale'/m,'import {scaleBand} from '+entry('d3-scale')));
const dispatcherBody=read('compute.ts').match(/^export function computeScale[\s\S]*?^\}/m)?.[0];assert(dispatcherBody);
const extra=join(root,'tests/fixtures/boxplot-summary-domain/vendor'),extraPins=JSON.parse(fs.readFileSync(join(extra,'source-pins.json'))).files;
for(const [name,pin] of Object.entries(extraPins)){const bytes=fs.readFileSync(join(extra,name));assert.equal(hash(bytes),pin.sha256);sourcePins['tests/fixtures/boxplot-summary-domain/vendor/'+name]=pin.sha256;}
const extraRead=name=>fs.readFileSync(join(extra,name),'utf8').replaceAll('\r\n','\n');
const log=url(extraRead('logScale.ts').replace(/^import \{[^}]*\} from 'd3-scale'/m,'import {scaleLog} from '+entry('d3-scale')));
const timeHelpers=url(extraRead('timeHelpers.ts').replace("'d3-time-format'",entry('d3-time-format')));
const time=url(extraRead('timeScale.ts').replace(/^import \{[^}]*\} from 'd3-scale'/m,'import {scaleTime,scaleUtc} from '+entry('d3-scale')).replace("'./timeHelpers'",JSON.stringify(timeHelpers)));
const dispatcher=url(`import {createLogScale} from ${JSON.stringify(log)};\nimport {createTimeScale} from ${JSON.stringify(time)};\nimport {createLinearScale} from ${JSON.stringify(linear)};\nimport {createBandScale} from ${JSON.stringify(band)};\n${dispatcherBody}`);
const replaceScale=s=>s.replace(/^import \{[^}]*\} from '@nivo\/scales'/m,'import {computeScale} from '+JSON.stringify(dispatcher));
const common=url(replaceScale(read('common.ts')));
const {generateBoxPlots}=await import(url(replaceScale(read('generation.ts')).replace("'./common'",JSON.stringify(common))));
const {summarizeDistribution}=await import(url(read('stratification.ts').replace("import { defaultProps } from '../props'",'const defaultProps = {}')));
const q=[.1,.25,.5,.75,.9],cases=[];
for(const base of [8e15,-8e15,8640000000000000-64,-8640000000000000])for(let offset=0;offset<16;offset++)for(let width=1;width<=16;width++)cases.push(['time',base+offset,base+offset+width,q,'auto','auto']);
for(const [x,y] of [[8000000000000000,8000000000000005],[8000000000000001,8000000000000004]])for(const [min,max] of [[x,'auto'],['auto',y],[x,y]])cases.push(['time',x,y,q,min,max]);
for(const [x,y] of [[.1,.3],[7,7],[7,10],[1e15,1e15+3],[-1e15,-1e15+5],[-0,0],[0,-0],[Number.MAX_VALUE,Number.MAX_VALUE]])for(const quantiles of [q,[0,.1,.5,.9,1]])cases.push(['linear',x,y,quantiles,'auto','auto']);
const number=v=>Number.isNaN(v)?'nan':v===Infinity?'inf':v===-Infinity?'-inf':Object.is(v,-0)?'-0':String(v),list=a=>a.map(number).join(',');
const lines=['kind|x|y|quantiles|min|max|summary|domain|pixels'];let weightedDifferences=0;
for(const [kind,x,y,quantiles,min,max] of cases){
 const summary=summarizeDistribution({data:[{value:x},{value:y}],getValue:d=>d.value,groups:['A'],subGroups:[''],groupIndex:0,subGroupIndex:0,quantiles});
 const result=generateBoxPlots({data:[summary],layout:'horizontal',groups:['A'],subGroups:[''],formatValue:String,getTooltipLabel:d=>d.group,minValue:min,maxValue:max,width:320,height:320,padding:.1,innerPadding:0,valueScale:{type:kind,nice:false,format:'native',precision:'millisecond',useUTC:true},indexScale:{type:'band',round:false}}),scale=result.xScale;
 weightedDifferences+=summary.values.filter((v,i)=>v!==x*(1-quantiles[i])+y*quantiles[i]).length;
 lines.push([kind,number(x),number(y),list(quantiles),min,max,list(summary.values),list(scale.domain().map(Number)),list(summary.values.map(v=>scale(v)+20))].join('|'));
}
const csv=lines.join('\n')+'\n',manifest=JSON.stringify({schema:1,records:cases.length,utcGridRecords:1024,utcFixedBoundRecords:6,scalarRecords:16,weightedValueDifferences:weightedDifferences,csvSha256:hash(csv),producer:'scripts/boxplot-summary-interpolation-reference.mjs',command:'node scripts/boxplot-summary-interpolation-reference.mjs <node_modules> [--check]',scope:'Actual original summarizeDistribution and generateBoxPlots/createTimeScale or linear factory. Four legal large narrow UTC regions16offsets×16widths, both supplied root witnesses with independent/both fixed endpoints, finite scalar/local duplicates/signedzero/constantMAX. Original native arithmetic and Date domains/query pixels are frozen exactly; no handwritten quantile oracle.',dependencies:versions,sourcePins,runtimeSourcePins},null,2)+'\n';
const csvPath=join(root,'tests/fixtures/boxplot-summary-interpolation.csv'),metaPath=join(root,'tests/fixtures/boxplot-summary-interpolation-source.json');
if(check){assert.equal(fs.readFileSync(csvPath,'utf8'),csv);assert.equal(fs.readFileSync(metaPath,'utf8'),manifest);}else{fs.writeFileSync(csvPath,csv);fs.writeFileSync(metaPath,manifest);}
console.log(JSON.stringify({passed:true,checked:check,records:cases.length,weightedDifferences,csvSha256:hash(csv)}));
