// Actual original Nivo BoxPlot auto/fixed endpoints, backed by d3-scale 4.0.2.
// node scripts/boxplot-domain-bounds-reference.mjs <node_modules> [--check]
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
const vendor=join(root,'tests/fixtures/boxplot-domain-bounds/vendor'),bar=join(referenceRoot,'tests/fixtures/bar-category-band/vendor/raw');
const hash=v=>createHash('sha256').update(v).digest('hex'),sourcePins={};
const pinned=JSON.parse(fs.readFileSync(join(vendor,'source-pins.json')));
for(const [name,entry] of Object.entries(pinned.files)){const bytes=fs.readFileSync(join(vendor,name));assert.equal(hash(bytes),entry.sha256);sourcePins['tests/fixtures/boxplot-domain-bounds/vendor/'+name]=entry.sha256;}
const barPins=JSON.parse(fs.readFileSync(join(referenceRoot,'tests/fixtures/bar-category-band/source-and-fixtures.json'))).files;
for(const name of ['nivo-bandScale.ts','nivo-LICENSE','d3-scale-LICENSE','d3-array-LICENSE']){const bytes=fs.readFileSync(join(bar,name));assert.equal(hash(bytes),barPins.find(e=>e.path==='vendor/raw/'+name).sha256);sourcePins['tests/fixtures/bar-category-band/vendor/raw/'+name]=hash(bytes);}
const versions={'d3-scale':'4.0.2','d3-array':'3.2.5','d3-interpolate':'3.0.1'};
for(const [name,version] of Object.entries(versions))assert.equal(JSON.parse(fs.readFileSync(join(modules,name,'package.json'))).version,version);
const entry=name=>JSON.stringify(pathToFileURL(join(modules,name,'src/index.js')).href),read=name=>fs.readFileSync(join(vendor,name),'utf8').replaceAll('\r\n','\n');
const noTypes=s=>s.replace(/^import \{[^}]*\} from '(?:\.\.\/|\.\/)types'\n/gm,'');
const url=s=>'data:text/javascript;base64,'+Buffer.from(stripTypeScriptTypes(noTypes(s),{mode:'transform'})).toString('base64');
const linear=url(read('linearScale.ts').replace(/^import \{[^}]*\} from 'd3-scale'/m,'import {scaleLinear} from '+entry('d3-scale')).replace(/^import \{[^}]*\} from 'd3-interpolate'/m,'import {interpolateRound,interpolateNumber} from '+entry('d3-interpolate')));
const band=url(fs.readFileSync(join(bar,'nivo-bandScale.ts'),'utf8').replaceAll('\r\n','\n').replace(/^import \{[^}]*\} from 'd3-scale'/m,'import {scaleBand} from '+entry('d3-scale')));
const dispatcherBody=read('compute.ts').match(/^export function computeScale[\s\S]*?^\}/m)?.[0];assert(dispatcherBody);
const dispatcher=url(`import {createLinearScale} from ${JSON.stringify(linear)};\nimport {createBandScale} from ${JSON.stringify(band)};\n${dispatcherBody}`);
const replaceScale=s=>s.replace(/^import \{[^}]*\} from '@nivo\/scales'/m,'import {computeScale} from '+JSON.stringify(dispatcher));
const common=url(replaceScale(read('common.ts')));
const {generateBoxPlots}=await import(url(replaceScale(read('generation.ts')).replace("'./common'",JSON.stringify(common))));
const {summarizeDistribution}=await import(url(read('stratification.ts').replace("import { defaultProps } from '../props'",'const defaultProps = {}')));
const rows=['samples|min|max|vertical|reverse|round|nice|source|display|quantiles|pixels'];
for(const samples of [[2,4,8],[4,6,9]])for(const bounds of [['auto','auto'],['auto',10],[0,'auto'],[0,10],['auto',4],[6,'auto']])for(const vertical of [false,true])for(const reverse of [false,true]){
 const [min,max]=bounds,round=reverse,nice=min===0?5:false;
 const summary=summarizeDistribution({data:samples.map(value=>({value})),getValue:d=>d.value,groups:['A'],subGroups:[''],groupIndex:0,subGroupIndex:0,quantiles:[0,.25,.5,.75,1]});
 const originalSource=[min==='auto'?samples[0]:min,max==='auto'?samples.at(-1):max];
 const result=generateBoxPlots({data:[summary],layout:vertical?'vertical':'horizontal',groups:['A'],subGroups:[''],formatValue:String,getTooltipLabel:d=>d.group,minValue:min,maxValue:max,width:320,height:320,padding:.1,innerPadding:0,valueScale:{type:'linear',nice,reverse,round},indexScale:{type:'band',round:false}});
 const scale=vertical?result.yScale:result.xScale,display=scale.domain();if(reverse)display.reverse();
 rows.push([samples,min,max,+vertical,+reverse,+round,nice===false?-1:nice,originalSource,display,summary.values,summary.values.map(v=>scale(v)+20)].join('|'));
}
const csv=rows.join('\n')+'\n';
const manifest=JSON.stringify({schema:1,records:rows.length-1,csvSha256:hash(csv),scope:'Actual original Nivo independent minValue/maxValue and original linear/band scale factories. Quantile endpoints 0/1 isolate endpoint selection from the existing Kartis extrema versus default-Nivo-summary policy. Nivo reverse-domain normalized to Kartis reverse-range. Empty nonzero fallback is a Rust extension and is tested separately, not claimed identical.',producer:'scripts/boxplot-domain-bounds-reference.mjs',command:'node scripts/boxplot-domain-bounds-reference.mjs <node_modules> [--check]',dependencies:versions,sourcePins},null,2)+'\n';
const output=join(root,'tests/fixtures/boxplot-domain-bounds.csv'),meta=join(root,'tests/fixtures/boxplot-domain-bounds-source.json');
if(check){assert.equal(fs.readFileSync(output,'utf8'),csv);assert.equal(fs.readFileSync(meta,'utf8'),manifest);}else{fs.writeFileSync(output,csv);fs.writeFileSync(meta,manifest);}
console.log(JSON.stringify({passed:true,checked:check,records:rows.length-1,csvSha256:hash(csv),sourceManifestSha256:hash(manifest)}));
