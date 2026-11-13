// Reuse the original licensed D3 sources already pinned for Bar category bands.
// Run with node --experimental-vm-modules; no npm install or build is needed.
import fs from 'node:fs';
import vm from 'node:vm';
import assert from 'node:assert/strict';
import {resolve,dirname,join} from 'node:path';
import {fileURLToPath,pathToFileURL} from 'node:url';
import {createHash} from 'node:crypto';
const options=new Map();let check=false;
for(let i=2;i<process.argv.length;i++){const flag=process.argv[i];if(flag==='--check')check=true;else{assert(['--reference-root','--output'].includes(flag),'unknown option '+flag);assert(process.argv[i+1],'missing option value');options.set(flag,process.argv[++i]);}}
const root=resolve(options.get('--reference-root')??join(dirname(fileURLToPath(import.meta.url)),'..')),output=resolve(options.get('--output')??join(root,'tests/fixtures/boxplot-category-band.csv')),sourceOutput=output.replace(/\.csv$/, '-source.json');assert.notEqual(output,sourceOutput);
const base=join(root,'tests/fixtures/bar-category-band'),vendor=join(base,'vendor/raw'),sha=bytes=>createHash('sha256').update(bytes).digest('hex'),pinned=JSON.parse(fs.readFileSync(join(base,'source-and-fixtures.json'))),needed=['band.js','init.js','ordinal.js','d3-array.mjs','d3-scale-LICENSE','d3-array-LICENSE','internmap-LICENSE'];
const sourcePins={};for(const name of needed){const key='vendor/raw/'+name,entry=pinned.files.find(f=>f.path===key);assert(entry,'missing pinned original '+name);const bytes=fs.readFileSync(join(vendor,name));assert.equal(sha(bytes),entry.sha256);sourcePins['tests/fixtures/bar-category-band/'+key]=entry.sha256;}
const array=await import(pathToFileURL(join(vendor,'d3-array.mjs')).href),context=vm.createContext({}),modules=new Map(),arrayModule=new vm.SyntheticModule(['range','InternMap'],function(){this.setExport('range',array.range);this.setExport('InternMap',array.InternMap);},{context});
for(const name of ['band.js','init.js','ordinal.js'])modules.set(name,new vm.SourceTextModule(fs.readFileSync(join(vendor,name),'utf8'),{context,identifier:name}));await modules.get('band.js').link(spec=>spec==='d3-array'?arrayModule:modules.get(spec.slice(2)));await modules.get('band.js').evaluate();const band=modules.get('band.js').namespace.default;
const lines=['horizontal,reverse,round,inner,outer,align,step,bandwidth,A,B,C'];
for(const horizontal of[0,1])for(const reverse of[0,1])for(const round of[0,1])for(const inner of[.1,.3])for(const outer of[.1,.25])for(const align of[0,.5,1]){let range=horizontal?[24,216]:[60,161];if(reverse)range.reverse();const s=band().domain(['A','B','C']).range(range).paddingInner(inner).paddingOuter(outer).align(align).round(!!round);lines.push([horizontal,reverse,round,inner,outer,align,s.step(),s.bandwidth(),...['A','B','C'].map(k=>s(k))].join(','));}
const csv=lines.join('\n')+'\n',manifest=JSON.stringify({schema:1,scope:'96 actual original d3-scale4.0.2 band profiles. Nivo indexScale round plus equal inner/outer padding is the compatible subset; align,reverse and separate padding are explicit D3/Rust extensions.',records:96,csvSha256:sha(csv),producer:'scripts/boxplot-category-band-reference.mjs',commands:{regenerate:'node --experimental-vm-modules scripts/boxplot-category-band-reference.mjs',check:'node --experimental-vm-modules scripts/boxplot-category-band-reference.mjs --check'},sourcePins},null,2)+'\n';
if(check){assert.equal(fs.readFileSync(output,'utf8'),csv,'frozen CSV must match original D3');assert.equal(fs.readFileSync(sourceOutput,'utf8'),manifest,'frozen source manifest must match original sources');}else{fs.writeFileSync(output,csv);fs.writeFileSync(sourceOutput,manifest);}
console.log(JSON.stringify({passed:true,checked:check,records:96,csvSha256:sha(csv),sourceManifestSha256:sha(manifest)}));
