// Execute the actual pure partition/sort statements from local Nivo 0.99.0
// components; do not restate them as a Rust implementation or JS oracle copy.
import {readFile,writeFile} from 'node:fs/promises';import{createHash}from'node:crypto';
const root=process.argv[2]??'E:/Repos/nivo/packages/radar/src/';
const slices=await readFile(root+'RadarSlices.tsx','utf8'),slice=await readFile(root+'RadarSlice.tsx','utf8');
const pre=slices.match(/const halfAngleStep = angleStep \* 0\.5\s+let rootStartAngle = rotation - halfAngleStep/)[0];
const step=slices.match(/const startAngle = rootStartAngle\s+const endAngle = startAngle \+ angleStep\s+rootStartAngle \+= angleStep/)[0];
const sort=slice.match(/data\.sort\(\(a, b\) => a\.value - b\.value\)\s+data\.reverse\(\)/)[0];
const partition=Function('rotation','angleStep','count',`${pre};return Array.from({length:count},()=>{${step};return[startAngle,endAngle]})`);
const ordered=Function('data',`${sort};return data`);
const sha=s=>createHash('sha256').update(s).digest('hex');
const angles=[`# Actual RadarSlices.tsx SHA256 ${sha(slices)}`,'count|rotation|axis|start|end'];for(const count of[3,4,5,7,12,128])for(const rotation of[0,-.7,Math.PI*2+.2,12345.67])partition(rotation,Math.PI*2/count,count).forEach(([start,end],i)=>angles.push([count,rotation,i,start,end].join('|')));
const sorted=[`# Actual RadarSlice.tsx SHA256 ${sha(slice)}`,'case|position|id|value'];const cases=[[1,2,3],[1,1,0],[0,-0,0],[-4,2,-4],[Number.MAX_VALUE,-Number.MAX_VALUE,1]];cases.forEach((values,c)=>ordered(values.map((value,i)=>({id:'series'+i,value}))).forEach((d,i)=>sorted.push([c,i,d.id,Object.is(d.value,-0)?'-0':d.value].join('|'))));
await writeFile(new URL('../tests/fixtures/radar-slice-angles.csv',import.meta.url),angles.join('\n')+'\n');await writeFile(new URL('../tests/fixtures/radar-slice-sort.csv',import.meta.url),sorted.join('\n')+'\n');console.log(JSON.stringify({angles:angles.length-2,rows:sorted.length-2,slicesSha256:sha(slices),sliceSha256:sha(slice)}));
