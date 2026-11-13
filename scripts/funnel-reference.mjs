// Node 24+. Pass the Nivo checkout, d3-shape package and d3-scale package.
// The hook bodies are extracted so React and the rendering wrapper are unnecessary.
import {readFile} from 'node:fs/promises';
import {execFileSync} from 'node:child_process';
import {stripTypeScriptTypes} from 'node:module';
import {resolve} from 'node:path';
import {pathToFileURL} from 'node:url';

const [nivo, d3Shape, d3Scale] = process.argv.slice(2);
if (!nivo || !d3Shape || !d3Scale) throw Error('usage: funnel-reference.mjs <nivo> <d3-shape> <d3-scale>');
const hooks = (await readFile(resolve(nivo, 'packages/funnel/src/hooks.ts'), 'utf8')).replaceAll('\r\n', '\n');
const shape = await import(pathToFileURL(resolve(d3Shape, 'src/index.js')));
const scale = await import(pathToFileURL(resolve(d3Scale, 'src/index.js')));
const version = async path => JSON.parse(await readFile(resolve(path, 'package.json'), 'utf8')).version;
const [nivoVersion, shapeVersion, scaleVersion] = await Promise.all([
  version(resolve(nivo, 'packages/funnel')), version(d3Shape), version(d3Scale),
]);
const commit = execFileSync('git', ['-C', nivo, 'rev-parse', 'HEAD'], {encoding: 'utf8'}).trim();
function between(start, end) {
  const from = hooks.indexOf(start);
  const to = hooks.indexOf(end, from + 1);
  if (from < 0 || to < 0) throw Error(`Missing Nivo source markers ${start}, ${end}`);
  return hooks.slice(from, to);
}

const source = [
  between('export const computeShapeGenerators', 'interface CustomBandScale'),
  between('export const computeScales', 'export const computePartsHandlers'),
].join('\n').replaceAll('export const', 'const');
const factory = new Function('area', 'line', 'curveBasis', 'curveLinear', 'scaleLinear',
  `${stripTypeScriptTypes(source)}\nreturn {computeShapeGenerators, computeScales, computeSeparators};`);
const {computeShapeGenerators, computeScales, computeSeparators} = factory(
  shape.area, shape.line, shape.curveBasis, shape.curveLinear, scale.scaleLinear,
);
const parts = between('const enhancedParts =', '    }, [\n        data,');
const computePartsSource = stripTypeScriptTypes(`function compute() {\n${parts}\n}`);
const computeParts = new Function('data', 'direction', 'bandScale', 'innerWidth', 'innerHeight',
  'paddingBefore', 'rawShapeBlending', 'currentPartId', 'currentPartSizeExtension', 'getSize',
  'const getColor = () => "black", formatValue = String, getBorderColor = getColor, getLabelColor = getColor;\n' +
  'const fillOpacity = 1, borderOpacity = 0.66, borderWidth = 6, currentBorderWidth = 2;\n' +
  `${computePartsSource}\nreturn compute();`);

function commands(generator, points) {
  const out = [];
  generator.context({
    moveTo: (...p) => out.push(['M', ...p]),
    lineTo: (...p) => out.push(['L', ...p]),
    bezierCurveTo: (...p) => out.push(['C', ...p]),
    closePath: () => out.push(['Z']),
  })(points);
  return out.map(parts => parts.join(',')).join('|');
}

console.log(`# Nivo ${nivoVersion} ${commit}, D3 Shape ${shapeVersion} and Scale ${scaleVersion}`);
for (const [id, direction, interpolation, values, spacing, rawShapeBlending, lengths, offsets, sizes, currentPartId, currentPartSizeExtension] of [
  ['vertical-basis', 'vertical', 'smooth', [80, 40, 20], 12, 0.66, [18, 28], [4, 6], null, 's1', 7],
  ['horizontal-linear', 'horizontal', 'linear', [3, 9, 1], 10, 0.5, [10, 14], [3, 2], [28, 70, 16], null, 0],
  ['zero-domain', 'vertical', 'smooth', [0, 0], 4, 0.3, [0, 0], [0, 0], null, null, 0],
]) {
  const width = 200, height = 180;
  const data = values.map((value, index) => ({id: `s${index}`, value}));
  const paddingBefore = lengths[0] + offsets[0];
  const reserve = paddingBefore + lengths[1] + offsets[1];
  const innerWidth = width - (direction === 'vertical' ? reserve : 0);
  const innerHeight = height - (direction === 'horizontal' ? reserve : 0);
  const [band, value] = computeScales({data, direction, width: innerWidth, height: innerHeight, spacing});
  const parts = computeParts(data, direction, band, innerWidth, innerHeight, paddingBefore,
    rawShapeBlending, currentPartId, currentPartSizeExtension,
    datum => sizes ? sizes[data.indexOf(datum)] : value(datum.value));
  const [fill, border] = computeShapeGenerators(interpolation, direction);
  const separators = computeSeparators({parts, direction, width, height, spacing,
    enableBeforeSeparators: true, enableAfterSeparators: true,
    beforeSeparatorOffset: offsets[0], afterSeparatorOffset: offsets[1]});
  console.log([id, band.bandwidth, value.domain()[1], value.range()[1]].join('\t'));
  for (const part of parts) {
    console.log(['part', part.data.id, [part.x0,part.y0,part.width,part.height,part.x,part.y].join(','),
      commands(fill, part.areaPoints), commands(border, part.borderPoints)].join('\t'));
  }
  for (const [side, guides] of separators.entries()) {
    for (const s of guides) console.log(['separator', side, s.partId, [s.x0,s.y0,s.x1,s.y1].join(',')].join('\t'));
  }
}
