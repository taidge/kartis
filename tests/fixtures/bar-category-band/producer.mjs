import fs from 'node:fs';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
import vm from 'node:vm';
import {spawnSync} from 'node:child_process';
import {stripTypeScriptTypes} from 'node:module';
import * as array from './vendor/raw/d3-array.mjs';
if(!vm.SourceTextModule){const r=spawnSync(process.execPath,['--experimental-vm-modules',...process.argv.slice(1)],{stdio:'inherit',windowsHide:true});process.exit(r.status??1);}
const ctx=vm.createContext({}),modules=new Map(),base=new URL('./vendor/raw/',import.meta.url);
const arrayModule=new vm.SyntheticModule(['range','InternMap'],function(){this.setExport('range',array.range);this.setExport('InternMap',array.InternMap);},{context:ctx});
for(const file of ['band.js','init.js','ordinal.js'])modules.set(file,new vm.SourceTextModule(fs.readFileSync(new URL(file,base),'utf8'),{context:ctx,identifier:file}));
await modules.get('band.js').link(s=>s==='d3-array'?arrayModule:modules.get(s.slice(2)));await modules.get('band.js').evaluate();const band=modules.get('band.js').namespace.default;
const bandModule=new vm.SyntheticModule(['scaleBand'],function(){this.setExport('scaleBand',band);},{context:ctx});
const original=fs.readFileSync(new URL('nivo-bandScale.ts',base),'utf8');const source=stripTypeScriptTypes(original.replace(/import \{ scaleBand, ScaleBand as D3ScaleBand \} from 'd3-scale'/,"import {scaleBand} from 'd3-scale'").replace(/import \{ ComputedSerieAxis, ScaleBand, ScaleBandSpec, StringValue, ScaleAxis \} from '\.\/types'/,''),{mode:'strip'});
const nivo=new vm.SourceTextModule(source,{context:ctx});await nivo.link(()=>bandModule);await nivo.evaluate();
const originalRows=[];for(const size of [101,249.75])for(const axis of ['x','y'])for(const round of [false,true]){const keys=['A','B','C'],s=nivo.namespace.createBandScale({type:'band',round},{all:keys,min:0,max:0},size,axis).padding(.2);originalRows.push({size,axis,round,positions:keys.map(k=>s(k)),step:s.step(),bandwidth:s.bandwidth()});}
if(JSON.stringify(originalRows)!==JSON.stringify(JSON.parse(fs.readFileSync(new URL('nivo-original.json',import.meta.url))).rows))throw Error('original Nivo records differ');

const pins=JSON.parse(fs.readFileSync(new URL('source-and-fixtures.json',import.meta.url),'utf8'));for(const pin of pins.files){const bytes=fs.readFileSync(new URL(pin.path,import.meta.url));if(createHash('sha256').update(bytes).digest('hex')!==pin.sha256)throw Error('pinned original/fixture differs: '+pin.path);}
const rows=[];
for(const n of [0,1,3,17])for(const range of [[0,101],[101,0],[-20,81],[7.25,257]])for(const round of [false,true])for(const align of [0,.5,1])for(const [inner,outer]of [[0,0],[.2,.2],[.3,.7],[1,4]]){
 const domain=Array.from({length:n},(_,i)=>'C'+i),s=band().domain(domain).range(range).paddingInner(inner).paddingOuter(outer).align(align).round(round);
 rows.push({n,range,round,align,inner,outer,positions:domain.map(k=>s(k)),step:s.step(),bandwidth:s.bandwidth()});
}
const here=new URL('.',import.meta.url),file=new URL('d3-values.json',here),text=JSON.stringify(rows)+'\n';
if(process.argv.includes('--check')){if(fs.readFileSync(file,'utf8')!==text)throw Error('native D3 fixture differs');console.log('exact original D3 records '+rows.length);}else fs.writeFileSync(file,text);
