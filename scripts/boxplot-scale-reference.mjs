// Executes local Nivo BoxPlot code and released D3 scale modules.
// Run node scripts/boxplot-scale-reference.mjs <nivo-root> [--check].
import {execFileSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {readFile, writeFile} from 'node:fs/promises';
import {stripTypeScriptTypes} from 'node:module';
import {join} from 'node:path';
import {fileURLToPath, pathToFileURL} from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const [nivo] = process.argv.slice(2).filter(value => value !== '--check');
if (!nivo) throw Error('Supply the Nivo checkout root');
const {target_directory: target} = JSON.parse(execFileSync('cargo',
  ['metadata', '--no-deps', '--format-version', '1'], {cwd:root, encoding:'utf8', windowsHide:true}));
const modules = join(target, 'd3-color-reference', 'node_modules');
for (const [name, expected] of Object.entries({'d3-scale':'4.0.2', 'd3-array':'3.2.5', 'd3-interpolate':'3.0.1', 'd3-time':'3.1.1'})) {
  const {version} = JSON.parse(await readFile(join(modules, name, 'package.json'), 'utf8'));
  if (version !== expected) throw Error(`Expected ${name}@${expected}`);
}
const entry = name => JSON.stringify(pathToFileURL(join(modules, name, 'src/index.js')).href);
const paths = [
  'boxplot/src/compute/stratification.ts', 'boxplot/src/compute/generation.ts',
  'boxplot/src/compute/common.ts', 'scales/src/compute.ts', 'scales/src/linearScale.ts',
  'scales/src/logScale.ts', 'scales/src/symlogScale.ts', 'scales/src/bandScale.ts',
];
const sources = await Promise.all(paths.map(async path =>
  (await readFile(join(nivo, 'packages', path), 'utf8')).replaceAll('\r\n', '\n')));
const [stratification, generation, common, scaleCompute, ...factories] = sources;
const withoutTypes = source => source.replace(/^import \{[^}]*\} from '(?:\.\.\/|\.\/)types'\n/gm, '');
const moduleUrl = source => 'data:text/javascript;base64,' +
  Buffer.from(stripTypeScriptTypes(withoutTypes(source), {mode:'transform'})).toString('base64');
const factoryUrls = factories.map(source => moduleUrl(source
  .replace(/^import \{[^}]*\} from 'd3-scale'/m, "import {scaleLinear,scaleLog,scaleSymlog,scaleBand} from " + entry('d3-scale'))
  .replace(/^import \{[^}]*\} from 'd3-interpolate'/m, "import {interpolateRound,interpolateNumber} from " + entry('d3-interpolate'))));
const names = ['createLinearScale', 'createLogScale', 'createSymlogScale', 'createBandScale'];
const compute = scaleCompute.match(/^export function computeScale[\s\S]*?^\}/m)?.[0];
if (!compute) throw Error('Missing Nivo computeScale dispatcher');
// The time adapter uses D3 numeric UTC queries, with the same value-axis
// orientation used by the BoxPlot profile for the other scale kinds.
const dispatcher = moduleUrl(names.map((name, i) => `import {${name}} from ${JSON.stringify(factoryUrls[i])}`).join('\n') +
  `\nimport {scaleUtc} from ${entry('d3-scale')}
const createTimeScale = (spec, _data, size) => {
  const scale = scaleUtc().domain(spec.reverse ? [spec.max, spec.min] : [spec.min, spec.max]);
  scale[spec.round ? 'rangeRound' : 'range'](spec.axis === 'x' ? [0,size] : [size,0]);
  return spec.nice ? scale.nice(spec.nice === true ? undefined : spec.nice) : scale;
};
${compute}`);
const {summarizeDistribution} = await import(moduleUrl(stratification
  .replace("import { defaultProps } from '../props'", 'const defaultProps = {}')));
const importScale = source => source.replace(
  /^import \{[^}]*\} from '@nivo\/scales'/m,
  `import {computeScale} from ${JSON.stringify(dispatcher)}`,
);
const index = moduleUrl(importScale(common));
const {generateBoxPlots} = await import(moduleUrl(importScale(generation)
  .replace("'./common'", JSON.stringify(index))));

const day = 86400000, leapDay = Date.parse('2024-02-29T00:00:00Z');
// Kind, base or constant, domain, samples, orientation, reverse, round, nice.
// Nice 0 selects the D3 default; -1 disables it.
const profiles = [
  ['linear', 0, [-12,56], [-12,-4,0,2,7,19,38,56], false, false, false, -1],
  ['linear', 0, [4,4], [4,4,4], true, false, true, -1],
  ['log', 10, [3,770], [2,5,11,19,42,151,700], true, false, false, 0],
  ['log', 2, [-3,-90], [-80,-32,-16,-8,-6,-4], false, true, true, 0],
  ['symlog', 3, [51,-41], [-41,-20,-3,0,2,19,33,51], true, false, true, 7],
  ['utc', 0, [leapDay,leapDay+day+1], [leapDay,leapDay+1,leapDay+day,leapDay+day+1], true, true, false, -1],
];
const lines = [
  '# Nivo (LF SHA-256) ' + paths.map((path, i) =>
    `${path} ${createHash('sha256').update(sources[i]).digest('hex')}`).join('; '),
  '# D3 scale 4.0.2, array 3.2.5, interpolate 3.0.1, time 3.1.1; plot size 320, origin 20.',
  '# kind|parameter|domain|samples|vertical|reverse|round|nice|display|values|mean|box(x,y,w,h)|value coordinates|ticks(value,coordinate)',
];
for (const [kind, parameter, domain, samples, vertical, reverse, round, nice] of profiles) {
  const layout = vertical ? 'vertical' : 'horizontal';
  const groups = ['Sample'], subGroups = ['sensor'];
  const summary = summarizeDistribution({
    data:samples.map(value => ({value})), getValue:row => row.value,
    groups, subGroups, groupIndex:0, subGroupIndex:0, quantiles:[.1,.25,.5,.75,.9],
  });
  const result = generateBoxPlots({
    data:[summary], layout, groups, subGroups, formatValue:String, getTooltipLabel:datum => datum.group,
    minValue:domain[0], maxValue:domain[1], width:320, height:320, padding:.1, innerPadding:6,
    valueScale:{type:kind === 'utc' ? 'time' : kind, base:parameter, constant:parameter, axis:vertical ? 'y' : 'x',
      reverse, round, nice:nice < 0 ? false : nice === 0 ? true : nice},
    indexScale:{type:'band', round:false},
  });
  const scale = vertical ? result.yScale : result.xScale;
  const [box] = result.boxPlots;
  const coords = box.coordinates.values;
  const rect = vertical
    ? [box.x+20, Math.min(coords[1],coords[3])+20, box.width, box.height]
    : [Math.min(coords[1],coords[3])+20, box.y+20, box.width, box.height];
  const display = scale.domain().map(Number), ticks = scale.ticks(5).map(Number);
  // Nivo reverses its domain; Kartis reverses the output range.
  if (reverse) {
    display.reverse();
    ticks.reverse();
  }
  lines.push([kind, parameter, domain, samples, +vertical, +reverse, +round, nice, display,
    summary.values, summary.mean, rect, coords.map(value => value + 20),
    ticks.map(value => [value,scale(value)+20]).join(';')].join('|'));
}
const data = lines.join('\n') + '\n';
const output = new URL('../tests/fixtures/boxplot-scale-nivo-d3.csv', import.meta.url);
if (process.argv.includes('--check')) {
  if ((await readFile(output, 'utf8')).replaceAll('\r\n', '\n') !== data) {
    throw Error('BoxPlot scale oracle changed');
  }
} else {
  await writeFile(output, data);
}
console.log(`${profiles.length} independent Nivo/D3 BoxPlot profiles`);
