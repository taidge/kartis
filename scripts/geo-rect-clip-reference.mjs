// Original d3-geo 3.1.1 rectangle clip stream: contours and open lines.
import assert from 'node:assert/strict';
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
const check=process.argv.includes('--check');
const metadata=JSON.parse(execFileSync('cargo',['metadata','--no-deps','--format-version','1'],{encoding:'utf8'}));
const base=join(metadata.target_directory,'d3-geo-reference/node_modules/d3-geo');
assert.equal(JSON.parse(await readFile(join(base,'package.json'),'utf8')).version,'3.1.1');
const {geoClipRectangle}=await import(pathToFileURL(join(base,'src/index.js')));
const sha=b=>createHash('sha256').update(b).digest('hex'),sources={};
for(const name of ['clip/rectangle.js','clip/buffer.js','clip/line.js','clip/rejoin.js','pointEqual.js'])sources[name]=sha(await readFile(join(base,'src',name)));
const rect=[0,0,10,10],cases=[];
function execute(kind,input,bounds=rect){const output=[];let line;
 const sink={point(x,y){line.push([x,y]);},lineStart(){line=[];},lineEnd(){output.push(line);},polygonStart(){},polygonEnd(){}};
 const stream=geoClipRectangle(...bounds)(sink);if(kind==='polygon')stream.polygonStart();
 for(const r of input){stream.lineStart();const points=kind==='polygon'&&r.length>1&&r[0][0]===r.at(-1)[0]&&r[0][1]===r.at(-1)[1]?r.slice(0,-1):r;for(const p of points)stream.point(...p);stream.lineEnd();}
 if(kind==='polygon')stream.polygonEnd();return output;
}
function add(name,kind,input,bounds=rect){cases.push({name,kind,rect:bounds,input,output:execute(kind,input,bounds)});}
add('inside','polygon',[[[2,2],[8,2],[8,8],[2,8]]]);
add('outside','polygon',[[[-5,-5],[-2,-5],[-2,-2],[-5,-2]]]);
add('cover','polygon',[[[-5,-5],[15,-5],[15,15],[-5,15]]]);
add('hole','polygon',[[[-5,-5],[15,-5],[15,15],[-5,15]],[[3,3],[3,7],[7,7],[7,3]]]);
add('hole-through-edge','polygon',[[[-5,-5],[15,-5],[15,15],[-5,15]],[[-2,3],[-2,7],[7,7],[7,3]]]);
add('hole-covers-window','polygon',[[[-10,-10],[20,-10],[20,20],[-10,20]],[[-2,-2],[-2,12],[12,12],[12,-2]]]);
add('concave-split','polygon',[[[-5,1],[8,1],[8,3],[-2,3],[-2,7],[8,7],[8,9],[-5,9]]]);
add('boundary','polygon',[[[0,0],[10,0],[10,10],[0,10],[0,0]]]);
add('touch-corner','polygon',[[[-2,-2],[0,0],[-2,2]]]);
add('two-exteriors','polygon',[[[-2,1],[4,1],[4,4],[-2,4]],[[6,6],[12,6],[12,9],[6,9]]]);
add('open-cross','line',[[[-5,5],[15,5]]]);
add('open-reenter','line',[[[2,2],[15,2],[15,8],[2,8]]]);
add('open-multi','line',[[[-5,0],[15,0]],[[-5,10],[15,10]],[[5,-5],[5,15]]]);
add('open-corner','line',[[[-5,-5],[0,0],[-5,5]]]);
add('inside-point','line',[[[5,5]]]);
for(let i=0;i<24;i++){
 const x=(i%6)-3,y=Math.floor(i/6)-2;
 add(`translated-concave-${i}`,'polygon',[[[-5+x,1+y],[8+x,1+y],[8+x,3+y],[-2+x,3+y],[-2+x,7+y],[8+x,7+y],[8+x,9+y],[-5+x,9+y]]]);
 add(`translated-line-${i}`,'line',[[[-5+x,2+y],[5+x,12+y],[15+x,2+y]]]);
}
// Deterministic simple star-shaped rings, disjoint source holes and open lines.
// Input coordinates come from this producer; Rust never reproduces the shapes.
let seed=0x6d2b79f5;
function random(){seed^=seed<<13;seed^=seed>>>17;seed^=seed<<5;return(seed>>>0)/4294967296;}
for(let i=0;i<128;i++){
 const count=5+i%8,cx=-3+16*random(),cy=-3+16*random(),ring=[];
 for(let j=0;j<count;j++){const a=j*2*Math.PI/count,r=2+12*random();ring.push([cx+r*Math.cos(a),cy+r*Math.sin(a)]);}
 if(i%2)ring.reverse();add(`generated-ring-${i}`,'polygon',[ring]);
 const line=[];for(let j=0;j<3+i%9;j++)line.push([Math.round((-8+26*random())*8)/8,Math.round((-8+26*random())*8)/8]);
 add(`generated-line-${i}`,'line',[line]);
}
for(let i=0;i<64;i++){
 const outer=[[-50,-50],[50,-50],[50,50],[-50,50]],holes=[];
 for(const cx of [-2,12]){const r=3+2*random(),cy=-2+14*random(),ring=[];for(let j=0;j<8;j++){const a=-j*Math.PI/4;ring.push([cx+r*Math.cos(a),cy+r*Math.sin(a)]);}holes.push(ring);}
 add(`generated-holes-${i}`,'polygon',[outer,...holes]);
}
const fixture=resolve('tests/fixtures/geo-rect-clip');await mkdir(fixture,{recursive:true});
const values=JSON.stringify({schema:1,d3GeoVersion:'3.1.1',scope:'Actual original planar rectangle polygon/line stream outputs; contours preserve native winding and implicit closure.',cases},null,2)+'\n';
const reference=JSON.stringify({schema:1,d3GeoVersion:'3.1.1',sourceSHA256:sources,producerSHA256:sha(await readFile(new URL(import.meta.url))),valuesSHA256:sha(values),cases:cases.length},null,2)+'\n';
for(const [name,text]of [['values.json',values],['reference.json',reference]]){const file=join(fixture,name);if(check)assert.equal(await readFile(file,'utf8'),text,name);else await writeFile(file,text);}
console.log(JSON.stringify({passed:true,checked:check,cases:cases.length}));
