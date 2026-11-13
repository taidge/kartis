// Original Nivo 0.99 bars and index legend. No package installation or cache copying.
// node scripts/bar-index-legend-reference.mjs <scales-node_modules> <shape-node_modules> [--check]
import fs from 'node:fs';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {stripTypeScriptTypes} from 'node:module';
import {dirname, resolve, join} from 'node:path';
import {fileURLToPath, pathToFileURL} from 'node:url';
const root=resolve(dirname(fileURLToPath(import.meta.url)),'..');
const fixture=join(root,'tests/fixtures/bar-index-legend'), vendor=join(fixture,'vendor');
const args=process.argv.slice(2), check=args.includes('--check');
assert(args[0] && args[1], 'Supply released scales/shape node_modules directories');
const modules={scale:resolve(args[0]),shape:resolve(args[1])};
const hash=s=>createHash('sha256').update(s).digest('hex');
const manifest=JSON.parse(fs.readFileSync(join(fixture,'source-pins.json'),'utf8'));
for(const [name,pin] of Object.entries(manifest.files)) assert.equal(hash(fs.readFileSync(join(vendor,name))),pin.sha256);
const versions={'d3-scale':['scale','4.0.2'],'d3-interpolate':['scale','3.0.1'],'d3-array':['scale','3.2.5'],'d3-shape':['shape','3.2.0']};
for(const [name,[where,version]] of Object.entries(versions)) assert.equal(JSON.parse(fs.readFileSync(join(modules[where],name,'package.json'))).version,version);
const external=(name,where='scale')=>JSON.stringify(pathToFileURL(join(modules[where],name,'src/index.js')).href);
const read=name=>fs.readFileSync(join(vendor,name),'utf8').replaceAll('\r\n','\n');
const noImports=s=>s.replace(/^import[\s\S]*?from ['"][^'"]+['"]\n/gm,'');
const module=s=>'data:text/javascript;base64,'+Buffer.from(stripTypeScriptTypes(s,{mode:'transform'})).toString('base64');
const linear=module(`import {scaleLinear} from ${external('d3-scale')};\nimport {interpolateRound,interpolateNumber} from ${external('d3-interpolate')};\n`+noImports(read('linearScale.ts')));
const band=module(`import {scaleBand} from ${external('d3-scale')};\n`+noImports(read('bandScale.ts')));
const dispatch=read('compute.ts').match(/^export function computeScale[\s\S]*?^\}/m)?.[0]; assert(dispatch);
const scales=module(`import {createLinearScale} from ${JSON.stringify(linear)};\nimport {createBandScale} from ${JSON.stringify(band)};\n`+dispatch);
const common=module(`import {computeScale} from ${JSON.stringify(scales)};\nconst commonDefaultProps={};\n`+noImports(read('common.ts')));
const prefix=`import {computeScale} from ${JSON.stringify(scales)};\nimport {coerceValue,filterNullValues,getIndexScale,normalizeData} from ${JSON.stringify(common)};\n`;
const {generateGroupedBars}=await import(module(prefix+noImports(read('grouped.ts'))));
const {generateStackedBars}=await import(module(prefix+`import {stack,stackOffsetDiverging} from ${external('d3-shape','shape')};\n`+noImports(read('stacked.ts'))));
const {getLegendDataForIndexes}=await import(module(`const uniqBy=(a,key)=>{const seen=new Set();return a.filter(x=>{const k=key(x);if(seen.has(k))return false;seen.add(k);return true;});};\nconst getPropertyAccessor=k=>d=>d[k];\n`+noImports(read('legends.ts'))));
const source=[{index:'A',z:null,a:2},{index:'B',z:0,a:null},{index:'C',z:5,a:-1},{index:'D',z:null,a:null}];
const colors={z:'#ff0000',a:'#0000ff'}, rows=['# Original Nivo actual bars include nullable source bars; all numbers retain original doubles.'];
let cases=0, bars=0, labels=0, nullable=0;
for(const layout of ['grouped','stacked']) for(const horizontal of [false,true]) for(const reverse of [false,true]) for(const round of [false,true]) for(const authored of [true,false]) {
 const keys=authored?['z','a']:['a','z'];
 const hidden=cases%3===0?['z']:[];
 const id=cases++;
 const result=(layout==='grouped'?generateGroupedBars:generateStackedBars)({
  data:source,keys,hiddenIds:hidden,layout:horizontal?'horizontal':'vertical',width:320,height:220,
  padding:.2,innerPadding:0,indexScale:{type:'band',round},valueScale:{type:'linear',min:-2,max:8,nice:false,reverse,round},
  getIndex:d=>d.index,getColor:d=>colors[d.id],getTooltipLabel:d=>String(d.id),formatValue:String,margin:{left:40,top:40,right:40,bottom:40},
 });
 rows.push(['H',id,layout,+horizontal,+reverse,+round,keys.join(','),hidden.join(',')].join('|'));
 const legend=getLegendDataForIndexes(result.bars,horizontal?'horizontal':'vertical',d=>d.indexValue);
 for(const item of legend) {rows.push(['L',id,item.id,item.label,item.color].join('|'));labels++;}
 for(const b of result.bars) {rows.push(['B',id,b.data.id,b.data.indexValue,b.data.value===null?'null':b.data.value,b.absX,b.absY,b.width,b.height,b.color].join('|'));bars++;if(b.data.value===null)nullable++;}
}
// Keep actual missing-color/empty-index fallback independent from the Rust checked profile.
const unusual=getLegendDataForIndexes([{data:{indexValue:''},color:null},{data:{indexValue:'A'},color:null},{data:{indexValue:'A'},color:'red'}],'vertical',d=>d.indexValue);
const outputs={'values.txt':rows.join('\n')+'\n','native-unusual.json':JSON.stringify({scope:'Original native fallback only: Rust Bar rejects empty category and always resolves a Color. These records are preserved, not used as checked Rust expectations.',records:unusual},null,2)+'\n'};
const receipt={schema:1,cases,bars,legendItems:labels,nativeNullBars:nullable,source:manifest.files,dependencies:versions,outputs:Object.fromEntries(Object.entries(outputs).map(([n,s])=>[n,hash(s)])),scope:'Execute original generateGroupedBars/generateStackedBars with actual released D3; original index legend on original bars before null filter. Keys retain unsorted authored and lexical profiles. Only linear/band dispatcher branches. Flat-property and string first-uniq adapters; no React, complete lodash or CSS color parsing claim.'};
outputs['reference.json']=JSON.stringify(receipt,null,2)+'\n';
for(const [name,text] of Object.entries(outputs)) {const dest=join(fixture,name);if(check)assert.equal(fs.readFileSync(dest,'utf8'),text);else fs.writeFileSync(dest,text);}
console.log(JSON.stringify({passed:true,checked:check,cases,bars,legendItems:labels,nativeNullBars:nullable}));
