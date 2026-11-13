// Install exact reference packages into the workspace target_directory:
// npm install --prefix <target_directory>/d3-chromatic-reference --no-save --ignore-scripts d3-scale-chromatic@3.1.0 d3-color@3.1.0 d3-interpolate@3.0.1
// Run node scripts/color-schemes-reference.mjs [--check].
import {execFileSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {readFile, readdir, writeFile} from 'node:fs/promises';
import {join} from 'node:path';
import {fileURLToPath, pathToFileURL} from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const metadata = execFileSync('cargo', ['metadata', '--no-deps', '--format-version', '1'],
  {cwd: root, encoding: 'utf8', windowsHide: true});
const modules = join(JSON.parse(metadata).target_directory, 'd3-chromatic-reference', 'node_modules');
const pinned = {
  'd3-scale-chromatic': '3.1.0',
  'd3-color': '3.1.0',
  'd3-interpolate': '3.0.1',
};
for (const [name, version] of Object.entries(pinned)) {
  const found = JSON.parse(await readFile(join(modules, name, 'package.json'), 'utf8')).version;
  if (found !== version) throw Error(`Expected ${name}@${version}, got ${found}`);
}
const load = name => import(pathToFileURL(join(modules, name, 'src/index.js')).href);
const chromatic = await load('d3-scale-chromatic');
const {rgb} = await load('d3-color');
const chromaticSource = join(modules, 'd3-scale-chromatic', 'src');

// Nivo's exact IDs, with the D3 export suffix and the Rust data identifier.
const brewer = [
  ['blues', 'Blues', 'BLUES'],
  ['greens', 'Greens', 'GREENS'],
  ['greys', 'Greys', 'GREYS'],
  ['oranges', 'Oranges', 'ORANGES'],
  ['purples', 'Purples', 'PURPLES'],
  ['reds', 'Reds', 'REDS'],
  ['blue_green', 'BuGn', 'BLUE_GREEN'],
  ['blue_purple', 'BuPu', 'BLUE_PURPLE'],
  ['green_blue', 'GnBu', 'GREEN_BLUE'],
  ['orange_red', 'OrRd', 'ORANGE_RED'],
  ['purple_blue_green', 'PuBuGn', 'PURPLE_BLUE_GREEN'],
  ['purple_blue', 'PuBu', 'PURPLE_BLUE'],
  ['purple_red', 'PuRd', 'PURPLE_RED'],
  ['red_purple', 'RdPu', 'RED_PURPLE'],
  ['yellow_green_blue', 'YlGnBu', 'YELLOW_GREEN_BLUE'],
  ['yellow_green', 'YlGn', 'YELLOW_GREEN'],
  ['yellow_orange_brown', 'YlOrBr', 'YELLOW_ORANGE_BROWN'],
  ['yellow_orange_red', 'YlOrRd', 'YELLOW_ORANGE_RED'],
  ['brown_blueGreen', 'BrBG', 'BROWN_BLUE_GREEN'],
  ['purpleRed_green', 'PRGn', 'PURPLE_RED_GREEN'],
  ['pink_yellowGreen', 'PiYG', 'PINK_YELLOW_GREEN'],
  ['purple_orange', 'PuOr', 'PURPLE_ORANGE'],
  ['red_blue', 'RdBu', 'RED_BLUE'],
  ['red_grey', 'RdGy', 'RED_GREY'],
  ['red_yellow_blue', 'RdYlBu', 'RED_YELLOW_BLUE'],
  ['red_yellow_green', 'RdYlGn', 'RED_YELLOW_GREEN'],
  ['spectral', 'Spectral', 'SPECTRAL'],
];
const lookup = [
  ['viridis', 'Viridis', 'VIRIDIS'],
  ['inferno', 'Inferno', 'INFERNO'],
  ['magma', 'Magma', 'MAGMA'],
  ['plasma', 'Plasma', 'PLASMA'],
];
const analytic = [
  ['turbo', 'Turbo'],
  ['cividis', 'Cividis'],
  ['warm', 'Warm'],
  ['cool', 'Cool'],
  ['cubehelixDefault', 'CubehelixDefault'],
  ['rainbow', 'Rainbow'],
  ['sinebow', 'Sinebow'],
];

const hash = s => createHash('sha256').update(s).digest('hex');
const manifest = [];
for (const sub of ['.', 'sequential-single', 'sequential-multi', 'diverging']) {
  for (const file of (await readdir(join(chromaticSource, sub))).sort()) {
    if (!file.endsWith('.js')) continue;
    const path = [sub === '.' ? 'src' : `src/${sub}`, file].join('/');
    manifest.push(`${path}:${hash(await readFile(join(modules, 'd3-scale-chromatic', path)))}`);
  }
}
for (const path of ['d3-color/src/cubehelix.js', 'd3-color/src/color.js',
  'd3-interpolate/src/basis.js', 'd3-interpolate/src/rgb.js',
  'd3-interpolate/src/cubehelix.js', 'd3-interpolate/src/color.js']) {
  manifest.push(`${path}:${hash(await readFile(join(modules, path)))}`);
}
const hex = color => rgb(color).formatHex().slice(1);
const literal = color => '0x' + hex(color);
const data = [
  '//! Generated from d3-scale-chromatic 3.1.0 by scripts/color-schemes-reference.mjs.',
  '//! See LICENSE-d3-scale-chromatic.txt for the ISC and ColorBrewer notices.',
];
const palettes = [];
for (const [id, d3, name] of brewer) {
  const scheme = chromatic[`scheme${d3}`];
  const sizes = scheme.slice(3);
  if (sizes.length !== 7 && sizes.length !== 9) throw Error(`Unexpected palette sizes for ${id}`);
  data.push('', '#[rustfmt::skip]', `pub(super) const ${name}: &[&[u32]] = &[`);
  for (const colors of sizes) {
    data.push(`    &[${colors.map(literal).join(', ')}],`);
    palettes.push(`${id}|${colors.length}|${colors.map(hex).join(' ')}`);
  }
  data.push('];');
}
for (const [, d3, name] of lookup) {
  const interpolate = chromatic[`interpolate${d3}`];
  data.push('', '#[rustfmt::skip]', `pub(super) const ${name}: &[u32; 256] = &[`);
  // Sample one interior point of every true D3 lookup bucket.
  const values = Array.from({length: 256}, (_, i) => literal(interpolate((i + .5) / 256)));
  for (let i = 0; i < 256; i += 8) data.push(`    ${values.slice(i, i + 8).join(', ')},`);
  data.push('];');
}

const u64 = new BigUint64Array(1);
const f64 = new Float64Array(u64.buffer);
function next(x, direction) {
  f64[0] = x;
  u64[0] += BigInt(direction);
  return f64[0];
}
function around(x) { return [next(x, -1), x, next(x, 1)]; }
let seed = 0xc4f712e5;
const random = Array.from({length: 32}, () => {
  seed ^= seed << 13;
  seed ^= seed >>> 17;
  seed ^= seed << 5;
  return (seed >>> 0) / 4294967296;
});
const probes = [-1e300, -.1, 0, Number.MIN_VALUE, .01, .0625, .1, .125,
  .2, .25, .3, 1 / 3, .4, .5, .6, 2 / 3, .7, .75, .8, .875, .9, .99,
  next(1, -1), 1, next(1, 1), 1e300, ...random];
const samples = [];
for (const [id, d3] of [...brewer, ...lookup, ...analytic]) {
  const interpolate = chromatic[`interpolate${d3}`];
  const scheme = chromatic[`scheme${d3}`];
  const buckets = scheme ? scheme.length - 2 : lookup.some(([x]) => x === id) ? 256 : 0;
  const parameters = new Set(probes);
  for (let i = 1; i < buckets; ++i) {
    for (const t of around(i / buckets)) parameters.add(t);
  }
  for (const t of [...parameters].sort((a, b) => a - b)) {
    // Nivo's sequential/diverging domain scales pass clamped unit inputs.
    samples.push(`${id}|${t}|${hex(interpolate(Math.max(0, Math.min(1, t))))}`);
  }
}

const check = process.argv.includes('--check');
async function emit(path, lines) {
  const contents = lines.join('\n') + '\n';
  const url = new URL(`../${path}`, import.meta.url);
  if (check) {
    if ((await readFile(url, 'utf8')).replaceAll('\r\n', '\n') !== contents) {
      throw Error(`Color scheme fixture drift: ${path}`);
    }
  } else {
    await writeFile(url, contents);
  }
}
const sourceStamp = `# ${Object.entries(pinned).map(([name, version]) => `${name}@${version}`).join(' ')} SHA256 ${hash(manifest.join('\n'))}`;
await emit('src/color_scale/schemes/data.rs', data);
await emit('tests/fixtures/color-schemes-d3.csv', [sourceStamp, '# Nivo ID|input|RGB', ...samples]);
await emit('tests/fixtures/color-scheme-palettes-d3.csv', [sourceStamp, '# Nivo ID|size|RGBs', ...palettes]);
console.log(`${samples.length} D3 samples, ${palettes.length} official palettes`);
