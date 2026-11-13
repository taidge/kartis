// Execute the actual local Nivo TypeScript compute function, stripping only
// types and replacing @nivo/scales' identity cast helpers. No algorithm rewrite.
// Reference packages live under the workspace's normal metadata target_directory.
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {execFileSync} from 'node:child_process';
import {fileURLToPath,pathToFileURL} from 'node:url';
import {join} from 'node:path';
import {stripTypeScriptTypes} from 'node:module';
import {createHash} from 'node:crypto';
const workspace=fileURLToPath(new URL('../',import.meta.url));
const metadata=JSON.parse(execFileSync('cargo',['metadata','--no-deps','--format-version','1'],{cwd:workspace,encoding:'utf8',windowsHide:true}));
const target=metadata.target_directory,sourcePath=join(process.argv[2]??'E:/Repos/nivo','packages/bump/src/area-bump/compute.ts');
for(const [directory,expected]of [['d3-color-reference/node_modules/d3-scale','4.0.2'],['d3-reference/node_modules/d3-shape','3.2.0']]){
 const actual=JSON.parse(await readFile(join(target,directory,'package.json'),'utf8')).version;
 if(actual!==expected)throw Error(`Reference version mismatch: ${directory}: ${actual}, expected ${expected}`);
}
const original=await readFile(sourcePath,'utf8'),sha=createHash('sha256').update(original).digest('hex');
let source=original.replace(/import\s*\{[^}]*\}\s*from\s*'d3-scale'/,"import {scalePoint,scaleLinear} from "+JSON.stringify(pathToFileURL(join(target,'d3-color-reference/node_modules/d3-scale/src/index.js')).href));
source=source.replace(/import\s*\{[^}]*\}\s*from\s*'@nivo\/scales'/,'const castPointScale = x => x; const castLinearScale = x => x;');
source=source.replace(/import\s*\{[^}]*\}\s*from\s*'\.\/types'/,'');
const code=stripTypeScriptTypes(source,{mode:'transform'});
const {computeSeries}=await import('data:text/javascript;base64,'+Buffer.from(code).toString('base64'));
const {area,curveLinear,curveBasis}=await import(pathToFileURL(join(target,'d3-reference/node_modules/d3-shape/src/index.js')).href);
const cases=[
 [{id:'A',data:[{x:0,y:6},{x:1,y:2},{x:2,y:9}]},{id:'B',data:[{x:0,y:2},{x:1,y:8},{x:2,y:3}]},{id:'C',data:[{x:0,y:0},{x:1,y:2},{x:2,y:0}]}],
 [{id:'A',data:[{x:0,y:4},{x:'0',y:2},{x:2,y:5}]},{id:'B',data:[{x:0,y:3},{x:'0',y:8},{x:2,y:2}]}],
 [{id:'A',data:[{x:'left',y:6},{x:'right',y:2}]},{id:'B',data:[{x:'left',y:2},{x:'middle',y:8},{x:'right',y:4}]},{id:'C',data:[{x:'middle',y:4},{x:'right',y:0}]}],
];
const rows=[`# Actual Nivo compute.ts SHA256 ${sha}; d3-scale 4.0.2; width400 height240.`,`case|align|spacing|xPadding|series|source|xType|x|pixelX|y0|y1`];
const paths=[`# Actual Nivo areaPoints + d3-shape 3.2.0 linear/basis, 15 digits.`,`case|align|spacing|xPadding|series|interpolation|path`];
for(const [caseIndex,data]of cases.entries())for(const align of ['start','middle','end'])for(const spacing of [0,4])for(const xPadding of [0,.6,1]){
 const result=computeSeries({data,width:400,height:240,align,spacing,xPadding});
 for(const s of result.series){
  for(const [index,p]of s.points.entries())rows.push([caseIndex,align,spacing,xPadding,s.id,index,typeof p.data.x==='number'?'n':'s',p.data.x,p.x,p.y-p.height/2,p.y+p.height/2].join('|'));
  // Sparse out-of-order input is aligned by the Rust layout before drawing;
  // compare its exact slice math above, not Nivo's nonmonotone raw-input path.
  if(caseIndex<2)for(const interpolation of ['linear','smooth']){
   const generator=area().x(p=>p.x).y0(p=>p.y0).y1(p=>p.y1).curve(interpolation==='linear'?curveLinear:curveBasis).digits(15);
   paths.push([caseIndex,align,spacing,xPadding,s.id,interpolation,generator(s.areaPoints)].join('|'));
  }
 }
}
await mkdir(new URL('../tests/fixtures/',import.meta.url),{recursive:true});
await writeFile(new URL('../tests/fixtures/area-bump-nivo.csv',import.meta.url),rows.join('\n')+'\n');
await writeFile(new URL('../tests/fixtures/area-bump-paths.csv',import.meta.url),paths.join('\n')+'\n');
console.log(JSON.stringify({points:rows.length-2,paths:paths.length-2,sourceSha256:sha}));
