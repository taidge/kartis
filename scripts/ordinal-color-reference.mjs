// Regenerate or check ordinal fixtures against a Nivo checkout and released D3.
// Run node scripts/ordinal-color-reference.mjs <nivo-root> [--check].
// D3 packages use the same workspace target references as the other color oracles.
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { readFile, writeFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const [reference] = process.argv.slice(2).filter(argument => argument !== '--check');
if (!reference) throw Error('Supply the Nivo checkout root');
const root = fileURLToPath(new URL('../', import.meta.url));
const metadata = JSON.parse(execFileSync('cargo', ['metadata', '--no-deps', '--format-version', '1'], {
  cwd: root, encoding: 'utf8', windowsHide: true,
}));
const load = async (directory, packageName, version) => {
  const location = join(metadata.target_directory, directory, 'node_modules', packageName);
  const manifest = JSON.parse(await readFile(join(location, 'package.json'), 'utf8'));
  if (manifest.version !== version) throw Error(`Expected ${packageName}@${version}`);
  return import(pathToFileURL(join(location, 'src/index.js')).href);
};
const [chromatic, { scaleOrdinal }] = await Promise.all([
  load('d3-chromatic-reference', 'd3-scale-chromatic', '3.1.0'),
  load('d3-color-reference', 'd3-scale', '4.0.2'),
]);
const readNivo = async path => (await readFile(resolve(reference, 'packages/colors/src', path), 'utf8'))
  .replaceAll('\r\n', '\n');
const [categorical, sequential, diverging, ordinal] = await Promise.all([
  readNivo('schemes/categorical.ts'),
  readNivo('schemes/sequential.ts'),
  readNivo('schemes/diverging.ts'),
  readNivo('scales/ordinalColorScale.ts'),
]);
const hash = source => createHash('sha256').update(source).digest('hex');
const lines = [
  '# Nivo scheme catalogs with d3-scale-chromatic 3.1.0 and d3-scale 4.0.2',
  `# Nivo sources (LF SHA-256) ${[categorical, sequential, diverging, ordinal].map(hash).join(' ')}`,
];
const entries = (source, name) => {
  const object = source.match(new RegExp(`^export const ${name} = \\{([\\s\\S]*?)^\\}`, 'm'))?.[1];
  if (!object) throw Error(`Missing Nivo catalog ${name}`);
  return object.split('\n').map(line => line.trim()).filter(line => line && !line.startsWith('//'))
    .map(line => {
      const pair = /^([A-Za-z0-9_]+): (scheme[A-Za-z0-9]+),$/.exec(line);
      if (pair) return [pair[1], chromatic[pair[2]]];
      const inline = /^nivo: \[((?:'#[0-9a-f]{6}',?\s*)+)\],$/.exec(line);
      if (inline) return ['nivo', inline[1].match(/#[0-9a-f]{6}/g)];
      throw Error(`Unsupported Nivo catalog entry ${line}`);
    });
};
const encodeColors = colors => colors.map(color => color.slice(1)).join(' ');
const fixedPalettes = entries(categorical, 'categoricalColorSchemes');
if (fixedPalettes.length !== 11) throw Error('Unexpected categorical palette count');
for (const [name, colors] of fixedPalettes) lines.push(`categorical|${name}|${encodeColors(colors)}`);

const ordinalRanges = [
  ...entries(sequential, 'sequentialColorSchemes'),
  ...entries(diverging, 'divergingColorSchemes'),
];
if (ordinalRanges.length !== 27) throw Error('Unexpected ordinal ColorBrewer catalog size');
for (const [name, sizes] of ordinalRanges) {
  // The actual Nivo default uses the largest defined palette for that family.
  const colors = sizes.at(-1);
  lines.push(`brewer|${name}|${encodeColors(colors)}`);
}

const colors = new Map(fixedPalettes).get('nivo').slice(0, 3);
const domain = [2, '2', -0, 0, '0', 'hidden', '2', '東京'];
const queries = ['unseen', 2, '2', -0, 0, '0', 'hidden', '東京', 'unseen', 99, '99', '', 2, 'B', 'C'];
const encode = key => `${typeof key === 'number' ? 'number' : 'text'}:${String(key)}`;
lines.push(`range|${encodeColors(colors)}`);
for (const mode of ['fixed', 'extend']) {
  const scale = scaleOrdinal(domain, colors);
  if (mode === 'fixed') scale.unknown('#010203');
  lines.push(`domain|${mode}|${domain.map(encode).join('|')}`);
  for (const key of queries) lines.push(`query|${mode}|${encode(key)}|${scale(key).slice(1)}`);
  lines.push(`final|${mode}|${scale.domain().map(encode).join('|')}`);
}
const fixture = lines.join('\n') + '\n';
const file = join(root, 'tests/fixtures/ordinal-color-d3.csv');
if (process.argv.includes('--check')) {
  if ((await readFile(file, 'utf8')).replaceAll('\r\n', '\n') !== fixture) {
    throw Error('Frozen ordinal color fixture differs');
  }
} else {
  await writeFile(file, fixture);
}
console.log(`11 categorical palettes, 27 default ColorBrewer palettes, ${queries.length * 2} ordinal queries`);
