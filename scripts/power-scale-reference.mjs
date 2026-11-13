// Independent d3-scale oracle; execute original functions, not copied math.
import { execFileSync } from 'node:child_process';
import { readFile, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { createHash } from 'node:crypto';
const root=fileURLToPath(new URL('../',import.meta.url));
const target=JSON.parse(execFileSync('cargo',['metadata','--offline','--no-deps','--format-version','1'],{cwd:root,encoding:'utf8',windowsHide:true})).target_directory;
const module=join(target,'d3-color-reference/node_modules/d3-scale');
const version=JSON.parse(await readFile(join(module,'package.json'),'utf8')).version;
if(version!=='4.0.2')throw Error('Expected d3-scale 4.0.2');
const {scalePow,scaleSqrt}=await import(pathToFileURL(join(module,'src/index.js')).href);
const hash=bytes=>createHash('sha256').update(bytes).digest('hex');
const stamps=await Promise.all(['pow.js','continuous.js','linear.js'].map(async file=>file+':'+hash(await readFile(join(module,'src',file)))));
const profiles=[
 ['sqrt',.5,[-9,25]],['sqrt',.5,[64,0]],['sqrt',.5,[-64,-1]],['sqrt',.5,[7,7]],
 ['pow',.5,[-9,25]],['pow',1,[-.71,.01]],['pow',1,[9,-3]],['pow',2,[-3,8]],
 ['pow',2,[81,-4]],['pow',3,[-7,5]],['pow',.25,[-100,400]],['pow',1.7,[-5,20]],
 ['pow',2.5,[.01,.9]],['pow',3,[2,2]],['pow',.1,[-2,9]],
 ['pow',-1,[1,16]],['pow',-1,[-16,-1]],['pow',-1,[16,1]],['pow',-1,[1,2]],
 ['pow',-1,[-2,-1]],['pow',-.5,[4,25]],['pow',-2,[-10,-2]],['pow',-2,[3,3]],
];
const rows=[`# Actual d3-scale ${version}; ${stamps.join('; ')}. C: kind,exponent,source,range,nice,reverse,round,clamp,display,queries,values,pixels,inverses,ticks. R: finite-profile nice rejection. T: raw-value ticks.`];
let configs=0;
for(const[kind,exponent,domain]of profiles)for(const nice of[-1,0,3,17])for(const reverse of[0,1])for(const round of[0,1])for(const clamp of[0,1]){
 const range=reverse?[207.5,-10.5]:[-10.5,207.5];
 const s=(kind==='sqrt'?scaleSqrt():scalePow().exponent(exponent)).domain(domain).clamp(!!clamp);
 round?s.rangeRound(range):s.range(range);if(nice===0)s.nice();else if(nice>0)s.nice(nice);
 const display=s.domain();const prefix=[kind,exponent,...domain,-10.5,207.5,nice,reverse,round,clamp,...display];
 configs++;
 if(exponent<0&&(display.includes(0)||Math.sign(display[0])!==Math.sign(display[1]))){rows.push(['R',...prefix].join('|'));continue;}
 const queries=[...domain,(domain[0]+domain[1])/2,0,-1,1,-256,-16,-4,-.25,.25,4,16,256]
  .filter(v=>exponent>0||clamp||v!==0&&Math.sign(v)===Math.sign(domain[0]));
 const pixels=[-600,-10.5,0,40,98.5,207.5,400];
 rows.push(['C',...prefix,queries.join(','),queries.map(v=>s(v)).join(','),pixels.join(','),pixels.map(v=>s.invert(v)).join(','),s.ticks(7).join(',')].join('|'));
}
for(const[kind,exponent,domain]of profiles)for(const count of[0,1,3,7,1000]){
 const s=(kind==='sqrt'?scaleSqrt():scalePow().exponent(exponent)).domain(domain);
 rows.push(['T',kind,exponent,...domain,count,s.ticks(count).join(',')].join('|'));
}
// Exact inverse singularities expose +/-0 arithmetic before D3's raw clamp.
for(const domain of [[1,2],[-2,-1]])for(const clamp of[0,1]){
 const s=scalePow().exponent(-1).domain(domain).range([0,200]).clamp(!!clamp);
 const pixels=[-400,-200,0,100,200,400,600];
 rows.push(['I',...domain,clamp,pixels.join(','),pixels.map(v=>s.invert(v)).join(',')].join('|'));
}
for(const[kind,exponent,domain]of [profiles[0],profiles[4],profiles[7],profiles[15],profiles[16]])for(const xr of[0,1])for(const yr of[0,1])for(const pan of[0,1]){
 const make=range=>(kind==='sqrt'?scaleSqrt():scalePow().exponent(exponent)).domain(domain).range(range);
 const xRange=xr?[200,0]:[0,200],yRange=yr?[0,200]:[200,0];
 const sx=make(xRange),sy=make(yRange);
 const dx=-70+(pan?20:0),dy=-90+(pan?-10:0);
 const x=xRange.map(p=>sx.invert((p-dx)/2)).sort((a,b)=>a-b),y=yRange.map(p=>sy.invert((p-dy)/2)).sort((a,b)=>a-b);
 rows.push(['Z',kind,exponent,...domain,xr,yr,pan,...x,...y].join('|'));
}
const path=new URL('../tests/fixtures/power-scale-d3.csv',import.meta.url),contents=rows.join('\n')+'\n';
if(process.argv.includes('--check')){if(await readFile(path,'utf8')!==contents)throw Error('Power scale oracle changed');}else await writeFile(path,contents);
console.log(JSON.stringify({version,configurations:configs,records:rows.length-1}));
