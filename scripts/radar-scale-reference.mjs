// Run from the Kartis workspace. Uses the same pinned D3 reference installation
// as scale-options-reference.mjs. No JavaScript is used by the Rust layouts.
import {execFileSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {readFile, writeFile} from 'node:fs/promises';
import {join} from 'node:path';
import {fileURLToPath, pathToFileURL} from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const {target_directory: target} = JSON.parse(execFileSync('cargo',
  ['metadata', '--no-deps', '--format-version', '1'], {cwd: root, encoding: 'utf8'}));
const modules = join(target, 'd3-color-reference', 'node_modules');
const version = async name => JSON.parse(await readFile(join(modules, name, 'package.json'))).version;
if (await version('d3-scale') !== '4.0.2' || await version('d3-array') !== '3.2.5') {
  throw Error('Expected d3-scale 4.0.2 and d3-array 3.2.5');
}
const files = ['linear.js', 'log.js', 'symlog.js', 'continuous.js'];
const stamps = await Promise.all(files.map(async name => {
  const src = await readFile(join(modules, 'd3-scale', 'src', name));
  return `${name}:${createHash('sha256').update(src).digest('hex')}`;
}));
const d3 = await import(pathToFileURL(join(modules, 'd3-scale', 'src', 'index.js')).href);
const profiles = [
  ['linear', 1, [-7, 31], -1, 'even', 5, [-7, 0, 15, 31]],
  ['linear', 1, [-7, 31], 4, 'ticks', 4, [-7, 0, 15, 31]],
  ['log', 10, [.1, 1000], -1, 'even', 4, [.1, 1, 10, 100, 1000]],
  ['log', 2, [-53, -.75], 0, 'ticks', 5, [-53, -8, -2, -.75]],
  ['log', 2.5, [.3, 70], 0, 'ticks', 8, [.3, .7, 9, 70]],
  ['symlog', .5, [-80, 20], -1, 'even', 6, [-80, -20, -1, 0, 1, 20]],
  ['symlog', 4, [-33, 27], 5, 'ticks', 5, [-33, -4, 0, 4, 27]],
  ['symlog', 7, [-42, -2], -1, 'even', 7, [-42, -11, -2]],
];
const rows = [
  `# d3-scale 4.0.2 with d3-array 3.2.5; ${stamps.join('; ')}`,
  '# kind|parameter|source|nice(-1 off,0 default)|tick mode|count|values|display|radii|tick values|tick radii',
];
for (const [kind, parameter, domain, nice, mode, count, values] of profiles) {
  const scale = (kind === 'linear' ? d3.scaleLinear()
    : kind === 'log' ? d3.scaleLog().base(parameter) : d3.scaleSymlog().constant(parameter))
    .domain(domain).range([0, 128]);
  if (nice === 0) scale.nice();
  if (nice > 0) scale.nice(nice);
  const display = scale.domain();
  const ticks = mode === 'even' ? Array.from({length: count}, (_, i) => {
    // The final ring is the exact display endpoint, even if a floating
    // log/exp round-trip moves that value by one ULP.
    return i + 1 === count ? display[1] : scale.invert(128 * ((i + 1) / count));
  }) : scale.ticks(count);
  rows.push([kind, parameter, domain, nice, mode, count, values,
    display, values.map(scale), ticks, ticks.map(scale)].join('|'));
}
const data = rows.join('\n') + '\n';
const output = new URL('../tests/fixtures/radar-scale-d3.csv', import.meta.url);
if (process.argv.includes('--check')) {
  if ((await readFile(output, 'utf8')).replaceAll('\r\n', '\n') !== data) {
    throw Error('Radar scale oracle fixture drift');
  }
} else {
  await writeFile(output, data);
}
console.log(`${profiles.length} independent D3 Radar scale profiles`);
