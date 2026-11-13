// Oracle for numeric RGBA inputs using released D3 modules.
import { execFileSync } from 'node:child_process';
import { readFile, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const workspace = fileURLToPath(new URL('../', import.meta.url));
const metadata = JSON.parse(execFileSync('cargo', ['metadata', '--no-deps', '--format-version', '1'], {cwd: workspace, encoding: 'utf8', windowsHide: true}));
const reference = join(metadata.target_directory, 'd3-color-reference', 'node_modules');
const load = name => import(pathToFileURL(join(reference, name, 'src/index.js')).href);
const [color, interpolate, scale] = await Promise.all(['d3-color', 'd3-interpolate', 'd3-scale'].map(load));
const factories = {
  'rgb-2.2': interpolate.interpolateRgb.gamma(2.2),
  lab: interpolate.interpolateLab,
  'hcl-short': interpolate.interpolateHcl,
  'hcl-direct': interpolate.interpolateHclLong,
  'hsl-short': interpolate.interpolateHsl,
  'hsl-direct': interpolate.interpolateHslLong,
  'helix-short': interpolate.interpolateCubehelix,
  'helix-direct-2.2': interpolate.interpolateCubehelixLong.gamma(2.2),
};
const configs = [
  {kind: 'sequential', domain: [1, 9], mid: 5, clamp: true, colors: [[235, 30, 113, 0], [255, 140, 10, 201], [25, 92, 225, 255]], values: [1.5, 2, 3.25, 4.99, 5.01, 6.1, 7, 8.5]},
  {kind: 'sequential', domain: [10, -6], mid: 2, clamp: false, colors: [[0, 0, 0, 255], [65, 199, 40, 40], [255, 255, 255, 180]], values: [12, 9.5, 8, 7, 3.33, 1, -3, -8]},
  {kind: 'diverging', domain: [-9, 22], mid: 1, clamp: true, colors: [[37, 106, 210, 240], [179, 179, 179, 152], [210, 51, 46, 20]], values: [-8.5, -7, -5, -2, 0.99, 1.01, 3, 8, 13, 21.5]},
  {kind: 'diverging', domain: [18, -9], mid: 4, clamp: false, colors: [[230, 36, 80, 120], [55, 219, 170, 255], [105, 30, 225, 40]], values: [21, 16.5, 12, 8, 4.01, 3.99, 0, -3.3, -8.5, -11]},
];
const rows = [
  '# Independent d3-interpolate 3.0.1, d3-color 3.1.0 and d3-scale 4.0.2. Authored RGBA anchors are tested separately.',
  'profile|kind|domain0|domain1|midpoint|clamp|palette|value|r|g|b|a',
];
for (const [profile, factory] of Object.entries(factories)) {
  for (const {kind, domain, mid, clamp, colors, values} of configs) {
    const ramp = interpolate.piecewise(factory, colors.map(([r, g, b, a]) => color.rgb(r, g, b, a / 255)));
    const mapping = kind === 'sequential' ? scale.scaleSequential().domain(domain) : scale.scaleDiverging().domain([domain[0], mid, domain[1]]);
    mapping.clamp(clamp).interpolator(ramp);
    for (const value of values) {
      const channels = mapping(value).match(/[+-]?(?:\d*\.?\d+)(?:e[+-]?\d+)?/gi).map(Number);
      rows.push([profile, kind, ...domain, mid, Number(clamp), colors.map(c => c.join(':')).join(';'), value, ...channels.slice(0, 3), Math.round((channels[3] ?? 1) * 255)].join('|'));
    }
  }
}
const output = new URL('../tests/fixtures/color-interpolation-d3.csv', import.meta.url);
const expected = rows.join('\n') + '\n';
if (process.argv.includes('--check')) {
  if (await readFile(output, 'utf8') !== expected) throw Error('Color interpolation oracle differs');
} else {
  await writeFile(output, expected);
}
console.log(`${rows.length - 2} independent interpolation queries`);
