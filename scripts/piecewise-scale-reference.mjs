// Independent oracle: executes the original d3-scale functions, never Rust or
// a transcription of their interpolation. Requires the prepared dev packages.
import {execFileSync} from 'node:child_process';
import {readFile, writeFile} from 'node:fs/promises';
import {join} from 'node:path';
import {fileURLToPath, pathToFileURL} from 'node:url';
import {createHash} from 'node:crypto';
const root=fileURLToPath(new URL('../',import.meta.url));
const target=JSON.parse(execFileSync('cargo',['metadata','--offline','--no-deps','--format-version','1'],{cwd:root,encoding:'utf8',windowsHide:true})).target_directory;
const module=join(target,'d3-color-reference/node_modules/d3-scale');
const version=JSON.parse(await readFile(join(module,'package.json'),'utf8')).version;
if(version!=='4.0.2')throw Error('Expected d3-scale 4.0.2');
const d3=await import(pathToFileURL(join(module,'src/index.js')).href);
const hash=bytes=>createHash('sha256').update(bytes).digest('hex');
const stamps=await Promise.all(['continuous.js','linear.js','log.js','symlog.js','pow.js'].map(async file=>file+':'+hash(await readFile(join(module,'src',file)))));
const make=(kind,param)=>kind==='linear'?d3.scaleLinear():kind==='utc'?d3.scaleUtc():kind==='log'?d3.scaleLog().base(param):kind==='symlog'?d3.scaleSymlog().constant(param):kind==='sqrt'?d3.scaleSqrt():d3.scalePow().exponent(param);
const profiles=[
 ['linear',1,[-3,.7,9]],['linear',1,[23,8,1,-5]],['linear',1,[-.71,-.1,.01]],
 ['log',10,[.3,2,9,180]],['log',2,[64,8,1]],['log',10,[-180,-9,-2,-.3]],['log',Math.E,[-.3,-2,-180]],
 ['symlog',1,[-23,-1,0,3,71]],['symlog',2,[31,3,-1,-17]],['symlog',.1,[-.7,0,.03,.9]],
 ['pow',2,[-3,-1,0,2,8]],['pow',3,[17,2,-1,-5]],['pow',.25,[-100,-1,1,400]],
 ['pow',-1,[1,2,8,16]],['pow',-1,[-16,-8,-2,-1]],['pow',-2,[16,8,3]],
 ['sqrt',.5,[-9,0,4,25]],['sqrt',.5,[64,9,1,-4]],
 ['utc',1,[0,1800000,90000000]],['utc',1,[1704240000000,1704153600000,1704100000000,1704067200000]],
];
const fmt=xs=>xs.join(',');
const monotonic=xs=>xs.slice(1).every((v,i)=>xs[1]>xs[0]?v>xs[i]:v<xs[i]);
const rows=[`# Original d3-scale ${version}; ${stamps.join('; ')}. C|kind|parameter|sourceStops|rangeStops|nice(-1=off,0=default)|reverse|round|clamp|displayStops|displayRange|queries|mapped|pixels|D3inverse|ticks|branchPixels|branchInverse|uniqueInverse|unroundedMapped. R marks checked-profile rejection of reciprocal nice crossing zero. Raw oracle values also expose cross-runtime ULP sensitivity at exact half-pixel rounding boundaries.`];
let configurations=0,rejections=0;
for(const[kind,param,domain]of profiles)for(const shape of['up','down','turn','plateau'])for(const nice of[-1,0,3,17])for(const reverse of[0,1])for(const round of[0,1])for(const clamp of[0,1]){
 const n=domain.length;
 const range=domain.map((_,i)=>shape==='up'?-10.5+i*73:shape==='down'?207.5-i*51:shape==='turn'?(i%2?207.5:-10.5):(i<2?-10.5:207.5));
 const actualRange=reverse?range.toReversed():range;
 const s=make(kind,param).domain(domain).clamp(!!clamp);
 round?s.rangeRound(actualRange):s.range(actualRange);
 if(nice===0)s.nice();else if(nice>0)s.nice(nice);
 const display=s.domain().map(Number);
 const prefix=[kind,param,fmt(domain),fmt(range),nice,reverse,round,clamp,fmt(display),fmt(actualRange)];
 configurations++;
 if(kind==='pow'&&param<0&&(display.includes(0)||Math.sign(display[0])!==Math.sign(display.at(-1)))){
  rows.push(['R',...prefix].join('|'));rejections++;continue;
 }
 const queries=[...domain,...display,domain[0]*2,domain.at(-1)*2,(domain[0]+domain.at(-1))/2,0,-1,1]
  .filter(v=>clamp||kind!=='log'&&(kind!=='pow'||param>0)||v!==0&&Math.sign(v)===Math.sign(domain[0]));
 const pixels=[-600,-10.5,0,73,207.5,400,600];
 const branchPixels=actualRange.slice(1).map((v,i)=>(v+actualRange[i])/2);
 const branchInverse=branchPixels.map((p,i)=>+make(kind,param).domain(display.slice(i,i+2)).range(actualRange.slice(i,i+2)).clamp(!!clamp).invert(p));
 const raw=make(kind,param).domain(display).range(actualRange).clamp(!!clamp);
 rows.push(['C',...prefix,fmt(queries),fmt(queries.map(v=>s(v))),fmt(pixels),fmt(pixels.map(p=>+s.invert(p))),fmt(s.ticks(7).map(Number)),fmt(branchPixels),fmt(branchInverse),+monotonic(actualRange),fmt(queries.map(v=>raw(v)))].join('|'));
}
const contents=rows.join('\n')+'\n',path=new URL('../tests/fixtures/piecewise-scale-d3.csv',import.meta.url);
if(process.argv.includes('--check')){if(await readFile(path,'utf8')!==contents)throw Error('Piecewise D3 oracle changed');}
else await writeFile(path,contents);
console.log(JSON.stringify({version,configurations,rejections,records:rows.length-1,sha256:hash(contents)}));
