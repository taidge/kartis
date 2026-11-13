// Execute the original Nivo computeSeries function and D3 line generator.
import {execFileSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {readFile, writeFile} from 'node:fs/promises';
import {stripTypeScriptTypes} from 'node:module';
import {join} from 'node:path';
import {fileURLToPath, pathToFileURL} from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const {target_directory: target} = JSON.parse(execFileSync('cargo',
  ['metadata', '--no-deps', '--format-version', '1'], {cwd:root, encoding:'utf8'}));
const nivo = process.argv.slice(2).find(argument => !argument.startsWith('--'));
if (!nivo) throw Error('Provide the Nivo checkout as a positional argument.');
const scaleRoot = join(target, 'd3-color-reference', 'node_modules', 'd3-scale');
const shapeRoot = join(target, 'd3-reference', 'node_modules', 'd3-shape');
for (const [directory, version] of [[scaleRoot, '4.0.2'], [shapeRoot, '3.2.0']]) {
  if (JSON.parse(await readFile(join(directory, 'package.json'), 'utf8')).version !== version) {
    throw Error(`Unexpected D3 reference version in ${directory}`);
  }
}
const scaleEntry = pathToFileURL(join(scaleRoot, 'src/index.js')).href;
const shape = await import(pathToFileURL(join(shapeRoot, 'src/index.js')).href);
const source = await readFile(join(nivo, 'packages/bump/src/bump/compute.ts'), 'utf8');
const moduleText = stripTypeScriptTypes(source
  .replace("'d3-scale'", JSON.stringify(scaleEntry))
  .replace(/^import .* from '@nivo\/scales'\r?\n/m, 'const castPointScale = scale => scale;\n')
  .replace(/^import \{\r?\n[\s\S]*?\} from '\.\/types'\r?\n/m, ''));
const {computeSeries} = await import('data:text/javascript;base64,' + Buffer.from(moduleText).toString('base64'));
const profiles = [
  {xPadding:.6,xOuterPadding:.5,yOuterPadding:.5,smooth:true,data:[
    {id:'A',data:[{x:'Q1',y:1},{x:1,y:4},{x:'1',y:10},{x:'Q4',y:4}]},
    {id:'B',data:[{x:'Q1',y:4},{x:1,y:1},{x:'1',y:null},{x:'Q4',y:10}]},
  ]},
  {xPadding:3,xOuterPadding:0,yOuterPadding:.25,smooth:false,data:[
    {id:'A',data:[{x:'Q1',y:null},{x:'Q2',y:4},{x:'Q3',y:null},{x:'Q4',y:10},{x:'Q5',y:null}]},
    {id:'B',data:[{x:'Q1',y:1},{x:'Q2',y:10},{x:'Q3',y:4},{x:'Q4',y:1},{x:'Q5',y:4}]},
  ]},
  {xPadding:.6,xOuterPadding:0,yOuterPadding:0,smooth:true,data:[
    {id:'A',data:[{x:'only',y:4}]},{id:'B',data:[{x:'only',y:null}]},
  ]},
  {xPadding:0,xOuterPadding:1.2,yOuterPadding:.75,smooth:false,data:[
    {id:'A',data:[{x:'Q3',y:-1.5},{x:'Q1',y:0},{x:'Q2',y:2.25}]},
    {id:'B',data:[{x:'Q2',y:10},{x:'Q3',y:2.25}]},
  ]},
  {xPadding:.6,xOuterPadding:.5,yOuterPadding:.5,smooth:true,data:[
    {id:'A',data:[{x:'Q1',y:null},{x:'Q2',y:null}]},
  ]},
  {xPadding:.6,xOuterPadding:0,yOuterPadding:0,smooth:false,data:[
    {id:'A',data:[{x:'Q1',y:1},{x:'Q2',y:2}]},
    {id:'B',data:[{x:'Q2',y:2},{x:'Q1',y:1}]},
  ]},
];
const rows = [
  '# Nivo bump/src/bump/compute.ts SHA256 ' + createHash('sha256').update(source).digest('hex'),
  '# d3-scale 4.0.2 and d3-shape 3.2.0, width 400, height 240, plot origin 0,0.',
  '# H profile,padding,outerX,outerY,smooth; X typed X positions; Y rank positions; P source data points; L shoulder points; G raw D3 path commands.',
];
const encode = key => typeof key === 'number' ? `n:${key}` : `s:${key}`;
for (const [profileIndex, profile] of profiles.entries()) {
  const result = computeSeries({width:400,height:240,...profile});
  rows.push(['H',profileIndex,profile.xPadding,profile.xOuterPadding,profile.yOuterPadding,+profile.smooth].join('|'));
  for (const key of result.xScale.domain()) rows.push(['X',profileIndex,encode(key),result.xScale(key)].join('|'));
  for (const key of result.yScale.domain()) rows.push(['Y',profileIndex,key,result.yScale(key)].join('|'));
  for (const [seriesIndex, series] of result.series.entries()) {
    for (const point of series.points) {
      const sourceIndex = Number(point.id.slice(point.id.lastIndexOf('.') + 1));
      rows.push(['P',profileIndex,seriesIndex,sourceIndex,encode(point.data.x),point.data.y,point.x,point.y].join('|'));
    }
    for (const [i, point] of series.linePoints.entries()) rows.push(['L',profileIndex,seriesIndex,i,...point.map(v=>v===null?'null':v)].join('|'));
    const record = (name, values) => rows.push(['G',profileIndex,seriesIndex,name,...values].join('|'));
    const context = {moveTo:(...v)=>record('M',v),lineTo:(...v)=>record('L',v),
      bezierCurveTo:(...v)=>record('C',v),closePath:()=>record('Z',[])};
    shape.line().defined(point=>point[0]!==null&&point[1]!==null)
      .curve(profile.smooth?shape.curveBasis:shape.curveLinear).context(context)(series.linePoints);
  }
}
const output = rows.join('\n') + '\n';
const fixture = join(root, 'tests/fixtures/bump-nivo.txt');
if (process.argv.includes('--check')) {
  if (await readFile(fixture, 'utf8') !== output) throw Error('Bump reference fixture differs.');
} else {
  await writeFile(fixture, output);
}
console.log(`Verified ${profiles.length} Bump profiles from original Nivo and D3 sources.`);
