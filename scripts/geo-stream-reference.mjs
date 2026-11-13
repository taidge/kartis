// Primary d3-geo 3.1.1 oracle; optional verification dependency only.
import {readFile,writeFile} from 'node:fs/promises';
import {execFileSync} from 'node:child_process';
import {fileURLToPath,pathToFileURL} from 'node:url';
import {join} from 'node:path';
const cwd=fileURLToPath(new URL('../',import.meta.url));
const target=JSON.parse(execFileSync('cargo',['metadata','--no-deps','--format-version','1'],{cwd,encoding:'utf8',windowsHide:true})).target_directory;
const base=join(target,'d3-geo-reference/node_modules/d3-geo');
const d3=await import(pathToFileURL(join(base,'src/index.js')).href);
const pkg=JSON.parse(await readFile(join(base,'package.json'),'utf8'));if(pkg.version!=='3.1.1')throw Error('Expected d3-geo 3.1.1');
const points=['# Actual d3-geo 3.1.1 geoTransverseMercator().scale(1).translate([0,0]); raw y points north.','centerLon|centerLat|roll|lon|lat|x|y'];
for(const center of [[0,0],[35,20],[-120,-40]])for(const roll of [0,25,-70])for(const lon of [-175,-70,-20,0,25,85,160])for(const lat of [-65,-20,0,35,80]){const p=d3.geoTransverseMercator().scale(1).translate([0,0]).rotate([-center[0],-center[1],-roll])([lon,lat]);if(p.every(Number.isFinite)&&Math.abs(p[0])<=Math.PI+1e-10)points.push([...center,roll,lon,lat,p[0],-p[1]].join('|'));}
await writeFile(new URL('../tests/fixtures/geo-transverse.csv',import.meta.url),points.join('\n')+'\n');
const g=['# Actual d3-geo 3.1.1 geoGraticule() source coordinates.','line|point|lon|lat'];d3.geoGraticule().lines().forEach((l,i)=>l.coordinates.forEach((p,j)=>g.push([i,j,...p].join('|'))));await writeFile(new URL('../tests/fixtures/geo-graticule.csv',import.meta.url),g.join('\n')+'\n');
const cases=[['equirectangular',[[170,30],[-170,45]]],['equirectangular',[[-85,10],[85,10]]],['transverse',[[10,-60],[60,30]]],['orthographic',[[-75,25],[75,25]]],['orthographic',[[-80,0],[80,0]]],['gnomonic',[[-80,20],[80,20]]]];
const lines=['# Actual d3-geo 3.1.1 streaming, scale120 translate300,200 precision0.25; cap60 for azimuthal.','case|projection|component|point|x|y'];
cases.forEach(([kind,coordinates],c)=>{const p=({equirectangular:d3.geoEquirectangular,transverse:d3.geoTransverseMercator,orthographic:d3.geoOrthographic,gnomonic:d3.geoGnomonic}[kind])().scale(120).translate([300,200]).precision(.25);if(kind==='orthographic'||kind==='gnomonic')p.clipAngle(60);let component=-1,index=0;d3.geoStream({type:'LineString',coordinates},p.stream({lineStart(){component++;index=0},point(x,y){lines.push([c,kind,component,index++,x,y].join('|'))},lineEnd(){},polygonStart(){},polygonEnd(){},sphere(){}}));});
await writeFile(new URL('../tests/fixtures/geo-stream-paths.csv',import.meta.url),lines.join('\n')+'\n');console.log(JSON.stringify({points:points.length-2,graticulePoints:g.length-2,pathPoints:lines.length-2}));
