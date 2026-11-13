// Actual original Nivo BoxPlot summary-based automatic domains and Date TimeClip.
// node scripts/boxplot-summary-domain-reference.mjs <node_modules> [--check]
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
const defaults=[.1,.25,.5,.75,.9],cases=[
 ['default','linear',[1,2,3,4],defaults,'auto','auto','exact'],
 ['fixed-min','linear',[1,2,3,4],defaults,2,'auto','exact'],
 ['fixed-max','linear',[1,2,3,4],defaults,'auto',3,'exact'],
 ['both-fixed','linear',[1,2,3,4],defaults,1,4,'exact'],
 ['custom-repeated','linear',[-10,-2,0,6,20],[.2,.2,.5,.8,.8],'auto','auto','exact'],
 ['median-only','linear',[1,2,3,4],[.5,.5,.5,.5,.5],'auto','auto','exact'],
 ['constant','linear',[7,7,7],defaults,'auto','auto','exact'],
 ['hidden-other-series','linear',[1,2,3,4],defaults,'auto','auto','exact'],
 ['empty','linear',[],defaults,'auto','auto','rust-finite-fallback'],
 ['all-hidden','linear',[],defaults,'auto','auto','rust-finite-fallback'],
 ['finite-extreme','linear',[-1e308,0,1e308],defaults,'auto','auto','exact'],
 ['overflow-original-quantile','linear',[-1e308,1e308],defaults,'auto','auto','rust-stable-quantile'],
 ['positive-log','log',[1,2,4,8],defaults,'auto','auto','exact'],
 ['negative-log','log',[-8,-4,-2,-1],defaults,'auto','auto','exact'],
 ['raw-zero-log','log',[0,1,2,3],defaults,'auto','auto','rust-raw-source-reject'],
 ['utc-positive-sub-ms','time',[0,1],defaults,'auto','auto','exact'],
 ['utc-negative-sub-ms','time',[-1,0],defaults,'auto','auto','exact'],
 ['utc-cross-sub-ms','time',[-1,1],defaults,'auto','auto','exact'],
 ['utc-zero-falsy-original','time',[0,0],defaults,'auto','auto','rust-zero-domain'],
 ['utc-fractional-statistics','time',[0,10],defaults,'auto','auto','exact'],
 ['utc-fixed-min','time',[0,10],defaults,2,'auto','exact'],
 ['utc-fixed-max','time',[0,10],defaults,'auto',8,'exact'],
 ['utc-max-date','time',[8640000000000000-10,8640000000000000],defaults,'auto','auto','date-bound'],
 ['utc-min-date','time',[-8640000000000000,-8640000000000000+10],defaults,'auto','auto','date-bound'],
];
const number=n=>Number.isNaN(n)?'nan':n===Infinity?'inf':n===-Infinity?'-inf':String(Object.is(n,-0)?0:n),list=a=>a.map(number).join(',');
const rows=['name|kind|samples|quantiles|min|max|summary|domain|pixels|scope'],details=[];
for(const [name,kind,samples,quantiles,min,max,scope] of cases){
 const summary=samples.length?summarizeDistribution({data:samples.map(value=>({value})),getValue:d=>d.value,groups:['A'],subGroups:[''],groupIndex:0,subGroupIndex:0,quantiles}):null;
 const result=generateBoxPlots({data:summary?[summary]:[],layout:'horizontal',groups:['A'],subGroups:[''],formatValue:String,getTooltipLabel:d=>d.group,minValue:min,maxValue:max,width:320,height:320,padding:.1,innerPadding:0,valueScale:{type:kind,nice:false,format:'native',precision:'millisecond',useUTC:true},indexScale:{type:'band',round:false}});
 const scale=result.xScale,domain=scale.domain().map(Number),values=summary?.values??[],pixels=values.map(v=>scale(v)+20);
 rows.push([name,kind,list(samples),list(quantiles),min,max,list(values),list(domain),list(pixels),scope].join('|'));
 details.push({name,scope,summary:summary?{extrema:summary.extrema.map(number),values:values.map(number),mean:number(summary.mean)}:null,originalDomain:domain.map(number),originalQueryPixels:pixels.map(number)});
}
const csv=rows.join('\n')+'\n',manifest=JSON.stringify({schema:1,records:cases.length,csvSha256:hash(csv),scope:'Actual original default/custom summary quantiles, independent endpoints, original linear/log/native-millisecond time factories. Hidden cases supply the already-visible summaries to original generateBoxPlots. Original empty/nonfinite/falsy-zero and raw-log-source differences remain explicitly recorded; native Rust intentionally retains existing finite/stable/strict-source policies. UTC Date TimeClip affects automatic endpoints only; summary/query values stay fractional.',producer:'scripts/boxplot-summary-domain-reference.mjs',command:'node scripts/boxplot-summary-domain-reference.mjs <node_modules> [--check]',dependencies:versions,sourcePins,runtimeSourcePins,originalDetails:details},null,2)+'\n';
const output=join(root,'tests/fixtures/boxplot-summary-domain.csv'),meta=join(root,'tests/fixtures/boxplot-summary-domain-source.json');
if(check){assert.equal(fs.readFileSync(output,'utf8'),csv);assert.equal(fs.readFileSync(meta,'utf8'),manifest);}else{fs.writeFileSync(output,csv);fs.writeFileSync(meta,manifest);}
console.log(JSON.stringify({passed:true,checked:check,records:cases.length,csvSha256:hash(csv),sourceManifestSha256:hash(manifest)}));
