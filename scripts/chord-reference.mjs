// Node 24+. Evaluate Nivo's original compute functions with its pinned
// d3-chord 1.0.6 and compatible original d3-array/d3-path distributions.
import {readFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {stripTypeScriptTypes} from 'node:module';
import {resolve, join} from 'node:path';
import {pathToFileURL} from 'node:url';
import {runInNewContext} from 'node:vm';

const [nivoRoot, chordRoot, arrayRoot, pathRoot, shapeRoot] = process.argv.slice(2);
if (!shapeRoot) throw Error('usage: chord-reference.mjs <nivo> <d3-chord> <d3-array> <d3-path> <d3-shape>');
const version = path => JSON.parse(readFileSync(join(path, 'package.json'), 'utf8')).version;
const load = (root, name, dependencies = {}) => {
  const source = readFileSync(join(root, 'dist', `${name}.js`), 'utf8');
  const exports = {};
  runInNewContext(source, {exports, module:{exports}, require:name => {
    if (!(name in dependencies)) throw Error(`Unexpected oracle dependency ${name}`);
    return dependencies[name];
  }});
  return exports;
};
const array = load(arrayRoot, 'd3-array');
const path = load(pathRoot, 'd3-path');
const chord = load(chordRoot, 'd3-chord', {'d3-array':array, 'd3-path':path});
const shape = await import(pathToFileURL(resolve(shapeRoot, 'src/index.js')));
const source = readFileSync(join(nivoRoot, 'packages/chord/src/compute.ts'), 'utf8');
const first = source.indexOf('export const computeChordLayout');
const last = source.indexOf('export const computeArcPath');
if (first < 0 || last <= first) throw Error('Nivo Chord oracle source markers missing');
const factory = new Function('d3Chord', 'd3Arc', 'd3Ribbon',
  stripTypeScriptTypes(source.slice(first, last).replaceAll('export const', 'const')) +
  '\nreturn {computeChordLayout, computeChordGenerators, computeChordArcsAndRibbons};');
const compute = factory(chord.chord, shape.arc, chord.ribbon);
const sourceHash = createHash('sha256').update(source).digest('hex');
console.log(`# Nivo ${version(join(nivoRoot, 'packages/chord'))}; compute.ts SHA256 ${sourceHash}`);
console.log(`# d3-chord ${version(chordRoot)}; d3-array ${version(arrayRoot)}; d3-path ${version(pathRoot)}; d3-shape ${version(shapeRoot)}`);
const profiles = [
  ['asymmetric', [[2,4,0],[9,0,1],[0,0,0]], .08, .9, .12],
  ['dominance-reversed', [[2,12,0],[9,0,1],[0,0,0]], .08, .9, .12],
  ['symmetric', [[0,1],[1,0]], .04, .9, 0],
  ['self', [[7]], 0, .9, 0],
  ['oneway', [[0,5],[0,0]], .03, .75, .2],
];
function commands(generator, datum) {
  const records = [];
  const save = name => (...args) => records.push([name, ...args].join(','));
  generator.context({moveTo:save('M'), lineTo:save('L'), arc:save('A'),
    quadraticCurveTo:save('Q'), bezierCurveTo:save('C'), closePath:save('Z')})(datum);
  generator.context(null);
  return records.join('~');
}
for (const [name, data, padAngle, innerRadiusRatio, innerRadiusOffset] of profiles) {
  const keys = data.map((_, i) => `g${i}`);
  const colors = ['#e8c1a0', '#f47560', '#f1e15b'];
  const {arcGenerator, ribbonGenerator, center, radius, innerRadius} = compute.computeChordGenerators({
    width:200, height:180, innerRadiusRatio, innerRadiusOffset,
  });
  const result = compute.computeChordArcsAndRibbons({
    chord:compute.computeChordLayout({padAngle}), data, keys,
    getLabel:datum => `label-${datum.id}`, formatValue:value => value.toFixed(2),
    getColor:datum => colors[datum.index],
  });
  console.log([name,'case',padAngle,innerRadiusRatio,innerRadiusOffset,center.join(','),radius,innerRadius].join('|'));
  for (const arc of result.arcs) {
    console.log([name,'arc',arc.index,arc.id,arc.value,arc.startAngle,arc.endAngle,arc.formattedValue,arc.color].join('|'));
    console.log([name,'arc-path',arc.index,commands(arcGenerator,arc)].join('|'));
  }
  for (const ribbon of result.ribbons) {
    const {source, target} = ribbon;
    console.log([name,'ribbon',source.index,target.index,source.value,target.value,
      source.startAngle,source.endAngle,target.startAngle,target.endAngle,ribbon.id].join('|'));
    console.log([name,'ribbon-path',source.index,target.index,commands(ribbonGenerator,ribbon)].join('|'));
  }
}
