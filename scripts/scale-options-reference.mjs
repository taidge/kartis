// Frozen independent oracle: executes actual d3-scale, never a reimplementation.
import {execFileSync} from 'node:child_process';
import {readFile,writeFile} from 'node:fs/promises';
import {fileURLToPath,pathToFileURL} from 'node:url';
import {join} from 'node:path';
import {createHash} from 'node:crypto';
const root=fileURLToPath(new URL('../',import.meta.url));
const target=JSON.parse(execFileSync('cargo',['metadata','--no-deps','--format-version','1'],{cwd:root,encoding:'utf8',windowsHide:true})).target_directory;
const modules=join(target,'d3-color-reference/node_modules');
const version=JSON.parse(await readFile(join(modules,'d3-scale/package.json'),'utf8')).version;
if(version!=='4.0.2')throw Error(`Expected actual d3-scale 4.0.2, got ${version}`);
const timeVersion=JSON.parse(await readFile(join(modules,'d3-time/package.json'),'utf8')).version,arrayVersion=JSON.parse(await readFile(join(modules,'d3-array/package.json'),'utf8')).version;
if(timeVersion!=='3.1.1'||arrayVersion!=='3.2.5')throw Error(`Unexpected time/array oracle versions ${timeVersion}/${arrayVersion}`);
const d3=await import(pathToFileURL(join(modules,'d3-scale/src/index.js')).href);
const sha=s=>createHash('sha256').update(s).digest('hex');
const stamps=await Promise.all(['log.js','linear.js','band.js','symlog.js','utcTime.js'].map(async p=>`${p}:${sha(await readFile(join(modules,'d3-scale/src',p)))}`));
stamps.push(`d3-time ${timeVersion}:${sha(await readFile(join(modules,'d3-time/dist/d3-time.js')))}`,`d3-array ${arrayVersion}:${sha(await readFile(join(modules,'d3-array/dist/d3-array.js')))}`);
const rows=[`# actual d3-scale ${version}; ${stamps.join('; ')}. C: kind,param,source,range,nice(-1 off/0 default),reverse,round,clamp,display,mapped,inverted,ticks; P: padding,align,round,reverse,range,domain positions.`];
const profiles=[['linear',1,[-.71,.01]],['linear',1,[.99,1.1]],['linear',1,[9,-3]],['linear',1,[7,7]],['log',10,[.13,870]],['log',10,[-870,-.13]],['log',2,[.7,83]],['log',2,[-.7,-83]],['log',2.5,[.3,70]],['log',1.5,[-70,-.3]],['log',Math.E,[.3,70]],['log',10,[.99,1.01]],['symlog',1,[-81,27]],['symlog',.03,[-.71,.01]],['symlog',7,[4,-13]],['utc',1,[-1234,8912]],['utc',1,[Date.UTC(2024,1,27,13),Date.UTC(2024,2,3,1)]],['utc',1,[Date.UTC(2023,10,9),Date.UTC(2025,4,9)]],['utc',1,[Date.UTC(1999,11,31),Date.UTC(2024,2,19)]],['utc',1,[28*86400000,-86400000]],['utc',1,[123,123]]];
profiles.push(['log',10,[1e23,2e23]],['log',10,[1e-23,9e-23]],['log',Math.E,[-1e20,-1e-10]],['log',1.000001,[.99,1.01]]);
for(const[kind,param,domain]of profiles)for(const nice of[-1,0,3,17])for(const reverse of[0,1])for(const round of[0,1]){
 const range=reverse?[207.5,-10.5]:[-10.5,207.5],s=kind==='linear'?d3.scaleLinear():kind==='log'?d3.scaleLog().base(param):kind==='symlog'?d3.scaleSymlog().constant(param):d3.scaleUtc();
 s.domain(domain);round?s.rangeRound(range):s.range(range);if(nice===0)s.nice();else if(nice>0)s.nice(nice);
 const output=s.domain().map(Number);if(output.some(v=>!Number.isFinite(v)))throw Error('Nonfinite display domain');
 for(const clamp of[0,1]){s.clamp(!!clamp);const extent=Math.max(...domain.map(Math.abs)),queries=[domain[0],Math.trunc((domain[0]+domain[1])/2),domain[1],Math.trunc(Math.min(...domain)-extent),Math.trunc(Math.max(...domain)+extent),0,-1,1],pixels=[-50,-10.5,10,98.5,207.5,250];const tickCount=kind==='log'?11:7;
 rows.push(['C',kind,param,...domain,-10.5,207.5,nice,reverse,round,clamp,...output,queries.join(','),queries.map(x=>s(x)).join(','),pixels.join(','),pixels.map(x=>+s.invert(x)).join(','),tickCount,s.ticks(tickCount).map(Number).join(',')].join('|'));}
}
for(const n of[0,1,2,5])for(const padding of[0,.2,1,3])for(const align of[0,.5,1])for(const round of[0,1])for(const reverse of[0,1]){
 const keys=Array.from({length:n},(_,i)=>`k${i}`),range=reverse?[97.5,-10.5]:[-10.5,97.5],s=d3.scalePoint().domain(keys).range(range).padding(padding).align(align).round(!!round);
 rows.push(['P',n,padding,align,round,reverse,-10.5,97.5,s.step(),keys.map(k=>s(k)).join(',')].join('|'));
}
// Independent sparse/dense tick requests, including the complete bounded UTC
// span and the full same-sign IEEE log span; no rendering or nice involved.
for(const[kind,param,domain]of [...profiles,['utc',1,[-8640000000000000,8640000000000000]],['log',10,[Number.MIN_VALUE,Number.MAX_VALUE]],['log',10,[-Number.MAX_VALUE,-Number.MIN_VALUE]]])for(const count of[1,2,5,10,1000]){
 const s=kind==='linear'?d3.scaleLinear():kind==='log'?d3.scaleLog().base(param):kind==='symlog'?d3.scaleSymlog().constant(param):d3.scaleUtc();s.domain(domain);
 rows.push(['T',kind,param,...domain,count,s.ticks(count).map(Number).join(',')].join('|'));
}
const contents=rows.join('\n')+'\n',dest=new URL('../tests/fixtures/scale-options-d3.csv',import.meta.url);
if(process.argv.includes('--check')){if(contents!==await readFile(dest,'utf8'))throw Error('Scale options oracle differs');}else await writeFile(dest,contents);
console.log(JSON.stringify({version,queries:rows.length-1}));
