// Independent pinned d3-scale oracle; reference packages are build artifacts,
// never Rust runtime dependencies. Run from the ordinary Kartis workspace.
import { execFileSync } from 'node:child_process';
import { readFile, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const workspace = fileURLToPath(new URL('../', import.meta.url));
const metadata = JSON.parse(execFileSync('cargo', ['metadata', '--no-deps', '--format-version', '1'], {
    cwd: workspace, encoding: 'utf8', windowsHide: true,
}));
const reference = join(metadata.target_directory, 'd3-color-reference', 'node_modules', 'd3-scale');
const version = JSON.parse(await readFile(join(reference, 'package.json'), 'utf8')).version;
if (version !== '4.0.2') throw new Error(`Expected d3-scale 4.0.2, got ${version}`);
const { scaleQuantile, scaleThreshold } = await import(pathToFileURL(join(reference, 'src/index.js')));
const adjacent = (value, direction) => {
    if (value === 0) return direction * Number.MIN_VALUE;
    const view = new DataView(new ArrayBuffer(8));
    view.setFloat64(0, value);
    view.setBigUint64(0, view.getBigUint64(0) + (value > 0 ? BigInt(direction) : -BigInt(direction)));
    return view.getFloat64(0);
};
const cases = [];
for (const samples of [
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9], [100, null, -3, 0, 0, 0, 2, 7],
    [5, 5, 5, 5], [4], [-1e300, -1e200, 0, 1e200, 1e300],
    [1, adjacent(1, 1), adjacent(adjacent(1, 1), 1)],
    [Number.MIN_VALUE, 2 * Number.MIN_VALUE, 4 * Number.MIN_VALUE],
]) for (const count of [1, 2, 3, 5, 16]) cases.push({ kind: 'quantile', samples, breaks: [], count });
for (const breaks of [[], [0], [-5, 0, 7], [0, 0], [-1e300, 0, 1e300],
    [Number.MIN_VALUE, 2 * Number.MIN_VALUE], [1, adjacent(1, 1)],
]) cases.push({ kind: 'threshold', samples: [], breaks, count: breaks.length + 1 });

const rows = ['# Independent d3-scale 4.0.2. Missing is undefined; finite Rust inputs only.',
    'kind|samples|breaks|colors|operation|argument|expected0|expected1'];
for (const item of cases) {
    const { kind, samples, breaks, count } = item;
    const scale = (kind === 'quantile' ? scaleQuantile().domain(samples) : scaleThreshold().domain(breaks))
        .range(Array.from({ length: count }, (_, i) => i)).unknown(-1);
    const cuts = kind === 'quantile' ? scale.quantiles() : scale.domain();
    const prefix = [kind, samples.map(v => v === null ? 'missing' : v).join(','), breaks.join(','), count];
    const push = (...values) => rows.push([...prefix, ...values].join('|'));
    const probes = [undefined, NaN, -1e301, -100, -5, -1, 0, 0.5, 1, 4, 5, 7, 9, 100, 1e301,
        ...samples.filter(v => v !== null), ...cuts.flatMap(v => [adjacent(v, -1), v, adjacent(v, 1)])];
    for (const value of probes) push('map', value === undefined ? 'missing' : String(value), scale(value), '');
    cuts.forEach((value, i) => push('cut', i, value, ''));
    for (let i = 0; i < count; i++) {
        const [start, end] = scale.invertExtent(i);
        push('extent', i, start === undefined ? 'unbounded' : start, end === undefined ? 'unbounded' : end);
    }
}
const output = new URL('../tests/fixtures/discrete-color-d3.csv', import.meta.url);
const contents = rows.join('\n') + '\n';
if (process.argv.includes('--check')) {
    if (await readFile(output, 'utf8') !== contents) throw new Error('Discrete color oracle changed');
} else await writeFile(output, contents);
console.log(`${cases.length} configurations, ${rows.length - 2} independent discrete color records verified`);
