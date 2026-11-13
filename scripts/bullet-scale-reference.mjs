// Executes local Nivo Bullet and D3 code. The frozen fixture is used by Rust tests.
import {execFileSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {readFile, writeFile} from 'node:fs/promises';
import {stripTypeScriptTypes} from 'node:module';
import {join} from 'node:path';
import {fileURLToPath, pathToFileURL} from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const {target_directory: target} = JSON.parse(execFileSync('cargo',
  ['metadata', '--no-deps', '--format-version', '1'], {cwd: root, encoding: 'utf8'}));
const modules = join(target, 'd3-color-reference', 'node_modules');
const nivo = process.argv.slice(2).find(argument => !argument.startsWith('--'));
if (!nivo) throw Error('Provide the Nivo checkout as a positional argument.');
const version = async name => JSON.parse(await readFile(join(modules, name, 'package.json'))).version;
if (await version('d3-scale') !== '4.0.2' || await version('d3-array') !== '3.2.5' ||
    await version('d3-interpolate') !== '3.0.1' || await version('d3-time') !== '3.1.1') {
  throw Error('Unexpected D3 reference versions.');
}
const entry = name => pathToFileURL(join(modules, name, 'src', 'index.js')).href;
const names = ['bullet/src/hooks.ts', 'bullet/src/compute.ts', 'scales/src/linearScale.ts'];
const [hooks, compute, linear] = await Promise.all(names.map(name => readFile(join(nivo, 'packages', name), 'utf8')));
const hash = source => createHash('sha256').update(source).digest('hex');
const withoutTypes = source => source.replace(/^import[^\n]+from '\.\/types'\r?\n/gm, '');
const moduleUrl = source => 'data:text/javascript;base64,' +
  Buffer.from(stripTypeScriptTypes(withoutTypes(source), {mode: 'transform'})).toString('base64');
const linearUrl = moduleUrl(linear
  .replace(/^import[^\n]+from 'd3-scale'/m, 'import {scaleLinear} from ' + JSON.stringify(entry('d3-scale')))
  .replace(/^import[^\n]+from 'd3-interpolate'/m,
    'import {interpolateRound, interpolateNumber} from ' + JSON.stringify(entry('d3-interpolate'))));
const {useEnhancedData} = await import(moduleUrl(hooks
  .replace("'@nivo/scales'", JSON.stringify(linearUrl))
  .replace("import { useMemo } from 'react'", 'const useMemo = callback => callback()')));
const {stackValues, computeRects} = await import(moduleUrl(compute
  .replace(/^import[^\n]+from '@nivo\/core'\r?\n/m, '')));
const d3 = await import(entry('d3-scale'));

const feb28 = Date.parse('2024-02-28T00:00:00Z'), day = 86400000;
// kind, parameter, source, ranges, measures, markers, vertical, reverse, scale reverse, round, nice
const profiles = [
  ['nivo', 0, [3, 37], [11, 28], [6, 21], [16], false, false, false, true, 0],
  ['nivo', 0, [-33, -7], [-9, -21], [-27, -15], [-11], true, true, false, true, 0],
  ['linear', 0, [-7, 31], [8, 22], [-3, 19], [12], true, false, false, false, 4],
  ['log', 10, [3, 760], [100, 10], [4, 40], [100, 500], false, false, true, true, 0],
  ['log', 2, [-60, -3], [-12, -24], [-40, -9], [-8], true, false, false, false, 0],
  ['symlog', 2, [-37, 41], [-12, 26], [-8, 18], [-2, 4], false, true, true, false, 5],
  ['utc', 0, [feb28, feb28 + 2 * day], [feb28 + day], [feb28 + day / 2], [feb28 + 1.5 * day], true, false, true, false, -1],
];
const rows = [
  '# Nivo ' + names.map((name, i) => name + ' SHA256 ' + hash([hooks, compute, linear][i])).join('; '),
  '# d3-scale 4.0.2, d3-array 3.2.5, d3-interpolate 3.0.1, d3-time 3.1.1; value and cross lengths 320, origin 20.',
  '# kind|parameter|source|ranges|measures|markers|vertical|reverse|scale reverse|round|nice|display|ranges(v0,v1,x,y,w,h)|measures(same)|marker coordinates|tick(value,coordinate)',
];
for (const [kind, parameter, source, ranges, measures, markers, vertical, reverse, scaleReverse, round, nice] of profiles) {
  const layout = vertical ? 'vertical' : 'horizontal';
  const effectiveReverse = reverse !== scaleReverse;
  const datum = {id: 'oracle', ranges, measures, markers};
  let raw;
  if (kind === 'nivo') {
    [{scale: raw}] = useEnhancedData([datum], {
      layout, minValue: source[0], maxValue: source[1], reverse: effectiveReverse, width: 320, height: 320,
    });
  } else {
    raw = (kind === 'log' ? d3.scaleLog().base(parameter)
      : kind === 'symlog' ? d3.scaleSymlog().constant(parameter)
      : kind === 'utc' ? d3.scaleUtc() : d3.scaleLinear()).domain(source).clamp(true);
    raw[round ? 'rangeRound' : 'range'](vertical !== effectiveReverse ? [320, 0] : [0, 320]);
    if (nice === 0) raw.nice();
    if (nice > 0) raw.nice(nice);
  }
  const scale = value => raw(value);
  // The Nivo rectangle implementation expects numeric endpoints; UTC Dates
  // are adapted to the integer epoch milliseconds consumed by the Rust layout.
  scale.domain = () => raw.domain().map(Number);
  const rects = (values, kind, thickness) => computeRects({
    data: stackValues(values, scale, index => index, kind),
    layout, reverse: effectiveReverse, scale, height: thickness,
  }).map(({data: {v0, v1}, x, y, width, height}) => {
    const cross = (320 - thickness) / 2;
    return [v0, v1, x + 20 + (vertical ? cross : 0), y + 20 + (vertical ? 0 : cross), width, height];
  }).join(';');
  rows.push([kind, parameter, source, ranges, measures, markers, +vertical, +reverse, +scaleReverse, +round,
    nice, scale.domain(), rects(ranges, 'range', 320), rects(measures, 'measures', 128),
    markers.map(value => scale(value) + 20),
    raw.ticks(4).map(Number).map(value => [value, scale(value) + 20]).join(';')].join('|'));
}
const data = rows.join('\n') + '\n';
const output = new URL('../tests/fixtures/bullet-scale-nivo-d3.csv', import.meta.url);
if (process.argv.includes('--check')) {
  if ((await readFile(output, 'utf8')).replaceAll('\r\n', '\n') !== data) {
    throw Error('Bullet scale oracle fixture drift.');
  }
} else {
  await writeFile(output, data);
}
console.log(profiles.length + ' independent Nivo/D3 Bullet profiles');
