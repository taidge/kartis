// Node 24+. Extract the actual Nivo Stream and scale hook bodies and run D3.
import {readFile} from 'node:fs/promises';
import {execFileSync} from 'node:child_process';
import {stripTypeScriptTypes} from 'node:module';
import {resolve} from 'node:path';
import {pathToFileURL} from 'node:url';

const [nivo, shapePath, scalePath] = process.argv.slice(2);
if (!nivo || !shapePath || !scalePath) throw Error('usage: stream-reference.mjs <nivo> <d3-shape> <d3-scale>');
const read = async path => (await readFile(path, 'utf8')).replaceAll('\r\n', '\n');
const shape = await import(pathToFileURL(resolve(shapePath, 'src/index.js')));
const scale = await import(pathToFileURL(resolve(scalePath, 'src/index.js')));
const interpolate = await import(pathToFileURL(resolve(scalePath, '../d3-interpolate/src/index.js')));
const hooks = await read(resolve(nivo, 'packages/stream/src/hooks.ts'));
const linear = await read(resolve(nivo, 'packages/scales/src/linearScale.ts'));
const point = await read(resolve(nivo, 'packages/scales/src/pointScale.ts'));
const scaleSource = (linear.slice(linear.indexOf('export const linearScaleDefaults')) + '\n' +
  point.slice(point.indexOf('export const createPointScale'))).replaceAll('export const', 'const');
const scalesFactory = new Function('scaleLinear', 'scalePoint', 'interpolateRound', 'interpolateNumber',
  stripTypeScriptTypes(scaleSource) + '\nreturn {createLinearScale,createPointScale};');
const {createLinearScale, createPointScale} = scalesFactory(scale.scaleLinear, scale.scalePoint,
  interpolate.interpolateRound, interpolate.interpolateNumber);
const from = hooks.indexOf('        const allMin: number[] = []');
const to = hooks.indexOf('    }, [stack, data, width, height])', from);
if (from < 0 || to < 0) throw Error('Nivo Stream hook markers changed');
const body = stripTypeScriptTypes(`function compute() {\n${hooks.slice(from,to)}\n}`);
const runHook = new Function('stack', 'data', 'width', 'height', 'createLinearScale', 'createPointScale',
  body + '\nreturn compute();');

function commands(generator, data) {
  const out = [];
  generator.context({
    moveTo: (...values) => out.push(['M', ...values]),
    lineTo: (...values) => out.push(['L', ...values]),
    bezierCurveTo: (...values) => out.push(['C', ...values]),
    closePath: () => out.push(['Z']),
  })(data);
  return out.map(command => command.join(',')).join('|');
}

const version = async root => JSON.parse(await read(resolve(root, 'package.json'))).version;
const [nivoVersion, shapeVersion, scaleVersion] = await Promise.all([
  version(resolve(nivo, 'packages/stream')), version(shapePath), version(scalePath),
]);
const commit = execFileSync('git', ['-C', nivo, 'rev-parse', 'HEAD'], {encoding:'utf8'}).trim();
console.log(`# Nivo ${nivoVersion} ${commit}, D3 Shape ${shapeVersion} and Scale ${scaleVersion}`);
const orders = {
  none:shape.stackOrderNone, reverse:shape.stackOrderReverse, ascending:shape.stackOrderAscending,
  descending:shape.stackOrderDescending, insideOut:shape.stackOrderInsideOut,
};
const offsets = {
  none:shape.stackOffsetNone, diverging:shape.stackOffsetDiverging, expand:shape.stackOffsetExpand,
  silhouette:shape.stackOffsetSilhouette, wiggle:shape.stackOffsetWiggle,
};
const curves = {
  linear:shape.curveLinear, basis:shape.curveBasis, cardinal:shape.curveCardinal,
  catmullRom:shape.curveCatmullRom, monotoneX:shape.curveMonotoneX, monotoneY:shape.curveMonotoneY,
  natural:shape.curveNatural, step:shape.curveStep, stepBefore:shape.curveStepBefore,
  stepAfter:shape.curveStepAfter,
};
const profiles = [
  ['none','wiggle','catmullRom'], ['reverse','none','linear'],
  ['ascending','diverging','linear'], ['descending','expand','basis'],
  ['insideOut','silhouette','natural'],
  ...Object.keys(curves).map(curve=>['none','wiggle',curve]),
];
for (const [order, offset, curve] of profiles) {
  const signed = offset === 'none' || offset === 'diverging';
  const data = [
    {A:10,B:16,C:3}, {A:20,B:null,C:5},
    {A:30,B:signed?-14:14,C:8}, {A:16,B:22,C:signed?-6:6},
  ];
  const raw = shape.stack().keys(['A','B','C']).order(orders[order]).offset(offsets[offset]);
  const [layers,x,y] = runHook(raw,data,200,160,createLinearScale,createPointScale);
  const original = raw(data);
  const area = shape.area().x(d=>d.x).y0(d=>d.y1).y1(d=>d.y2).curve(curves[curve]);
  console.log(['case',order,offset,curve,y.domain().join(',')].join('\t'));
  layers.forEach((layer, index)=>{
    const points = layer.map((p,i)=>({x:x(i),y1:y(p[0]),y2:y(p[1])}));
    console.log(['layer',original[index].key,original[index].index,commands(area,points)].join('\t'));
    layer.forEach((p,i)=>console.log(['sample',original[index].key,i,p.value===null?'null':p.value,
      [p[0],p[1],x(i),y(p[0]),y(p[1])].join(',')].join('\t')));
  });
}
