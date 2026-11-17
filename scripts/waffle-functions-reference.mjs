// Execute original Nivo Waffle callback and grid functions, with cold-render hook adapters.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createRequire, stripTypeScriptTypes } from 'node:module';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const script = fileURLToPath(import.meta.url), workspace = resolve(dirname(script), '..');
const args = Object.fromEntries(process.argv.slice(2).map(argument => {
    const split = argument.indexOf('=');
    assert(split > 2, 'usage: node scripts/waffle-functions-reference.mjs --nivo=<checkout> --out=<new-directory>');
    return [argument.slice(2, split), argument.slice(split + 1)];
}));
assert.deepEqual(Object.keys(args).sort(), ['nivo', 'out']);
const checkout = resolve(args.nivo), out = resolve(args.out);
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const git = (...values) => execFileSync('git', ['-C', checkout, ...values], { encoding: 'utf8' }).trim();
const commit = git('rev-parse', 'HEAD');
assert.match(commit, /^[a-f0-9]{40}$/);
const require = createRequire(import.meta.url);
const { babelParse, traverse } = require(resolve(workspace, 'node_modules/playwright/lib/transform/babelBundle.js'));
const compiler = {
    parser: 'existing Playwright Babel TypeScript parser',
    playwright: JSON.parse(await readFile(resolve(workspace, 'node_modules/playwright/package.json'))).version,
    parserBundleSha256: sha(await readFile(resolve(workspace, 'node_modules/playwright/lib/transform/babelBundleImpl.js'))),
    eraser: 'node:module.stripTypeScriptTypes mode strip; no mathematical rewriting',
    node: process.version,
};
const names = [
    'waffle/src/hooks.ts', 'waffle/src/types.ts', 'waffle/src/defaults.ts',
    'waffle/package.json', 'waffle/LICENSE.md', 'grid/src/grid.ts',
    'grid/src/boundingBoxes.ts', 'grid/src/polygon.ts', 'grid/LICENSE.md',
];
const originals = {}, sources = {}, ast = {}, extracts = {};
for (const name of names) {
    const bytes = await readFile(resolve(checkout, 'packages', name));
    assert(bytes.equals(execFileSync('git', ['-C', checkout, 'show', `${commit}:packages/${name}`])), `uncommitted reference source: ${name}`);
    originals[name] = bytes;
    sources[name] = { sha256: sha(bytes), bytes: bytes.length };
    if (/\.ts$/.test(name)) ast[name] = babelParse(bytes.toString('utf8'), name, true);
}
assert.equal(JSON.parse(originals['waffle/package.json']).version, '0.99.0');
for (const name of ['waffle/LICENSE.md', 'grid/LICENSE.md']) assert.match(originals[name].toString(), /Permission is hereby granted/);
function extract(file, name, visitor) {
    let found;
    traverse(ast[file], visitor(node => {
        assert.equal(found, undefined, `nonunique extract ${file}:${name}`);
        found = node;
    }));
    assert(found, `missing source expression ${file}:${name}`);
    const typescript = originals[file].toString('utf8').slice(found.start, found.end);
    const javascript = stripTypeScriptTypes(`(${typescript})`);
    extracts[`${file}:${name}`] = {
        file, start: found.start, end: found.end, startLine: found.loc.start.line,
        endLine: found.loc.end.line, typescript, javascript,
        sha256: sha(typescript), javascriptSha256: sha(javascript),
    };
    return javascript;
}
const variable = (file, name) => extract(file, name, found => ({
    VariableDeclarator(path) { if (path.node.id.name === name) found(path.node.init); },
}));
const evaluate = (expression, dependencies = {}) => new Function(
    ...Object.keys(dependencies), `"use strict"; return (${expression});`,
)(...Object.values(dependencies));
const expression = {};
for (const [file, names] of [
    ['grid/src/grid.ts', ['computeCellDimensions', 'generateGrid']],
    ['grid/src/boundingBoxes.ts', ['areBoundingBoxTouching']],
    ['grid/src/polygon.ts', ['getCellsPolygons']],
    ['waffle/src/hooks.ts', ['computeGrid', 'mergeCellsData', 'findPolygons', 'useWaffle']],
    ['waffle/src/types.ts', ['isDataCell']],
]) for (const name of names) expression[name] = variable(file, name);
expression.perpendicularPolygon = extract('grid/src/polygon.ts', 'perpendicularPolygon', found => ({
    FunctionDeclaration(path) { if (path.node.id.name === 'perpendicularPolygon') found(path.node); },
}));
// Archive the specific original closure separately as well as executing its complete enclosing hook.
extract('waffle/src/hooks.ts', 'computedData-useMemo-closure', found => ({
    VariableDeclarator(path) {
        if (path.node.id.name === 'computedData') found(path.node.init.arguments[0]);
    },
}));
const computeCellDimensions = evaluate(expression.computeCellDimensions);
const generateGrid = evaluate(expression.generateGrid, { computeCellDimensions });
const computeGrid = evaluate(expression.computeGrid, { generateGrid });
const mergeCellsData = evaluate(expression.mergeCellsData);
const areBoundingBoxTouching = evaluate(expression.areBoundingBoxTouching);
const perpendicularPolygon = evaluate(expression.perpendicularPolygon);
const getCellsPolygons = evaluate(expression.getCellsPolygons, { areBoundingBoxTouching, perpendicularPolygon });
const findPolygons = evaluate(expression.findPolygons, { getCellsPolygons });
const isDataCell = evaluate(expression.isDataCell);
const bits = value => {
    const bytes = Buffer.alloc(8);
    bytes.writeDoubleBE(value);
    return bytes.toString('hex');
};
const canonical = value => JSON.parse(JSON.stringify(value, (_key, item) =>
    typeof item === 'number' && Object.is(item, -0) ? { $number: '-0' } : item));
const freeze = value => {
    if (value && typeof value === 'object' && !Object.isFrozen(value)) {
        Object.values(value).forEach(freeze);
        Object.freeze(value);
    }
    return value;
};
const rawData = () => [
    ['negative-zero', -0, ''], ['one', 1, 'One'], ['hidden', 2, 'Hidden'],
    ['positive-zero', 0, 'Zero'], ['two', 1, 'Two'], ['three', 1, 'Three'],
    ['four', 1, 'Four'], ['five', 1, 'Five'], ['tail', 3, 'Tail'],
].map(([id, value, label], sourceIndex) => freeze({
    id, value, label, extra: { sourceIndex, status: sourceIndex % 2 ? 'late' : 'ready', retained: ['raw', sourceIndex] },
}));
const inputs = [], outputs = [];
const profiles = ['top', 'right', 'bottom', 'left'].flatMap(fillDirection => [
    { name: `${fillDirection}-overflow-full`, fillDirection, total: 12 },
    { name: `${fillDirection}-sparse`, fillDirection, total: 24 },
]).concat([
    { name: 'all-hidden', fillDirection: 'bottom', total: 12, allHidden: true },
    { name: 'empty-data', fillDirection: 'top', total: 12, empty: true },
]);
for (const profile of profiles) {
    const data = profile.empty ? [] : rawData(), trace = [], effects = [];
    const hiddenIds = profile.allHidden ? data.map(row => row.id) : ['hidden'];
    const input = {
        ...profile, width: 253, height: 181, rows: 2, columns: 3,
        emptyColor: '#abcdef', emptyOpacity: 0.375, data, hiddenIds,
        rawValueBits: data.map(row => bits(row.value)),
    };
    const colorFor = row => row.extra.status === 'late' ? '#aa2244' : '#117799';
    const formatFor = value => `value:${Object.is(value, -0) ? '-0' : value}`;
    const borderFor = row => row.isHidden ? '#445566' : '#778899';
    let colorCount = 0, formatCount = 0, groupBorderCount = 0, emptyBorderCount = 0;
    const colors = row => {
        const rawIndex = data.indexOf(row);
        assert.equal(rawIndex, colorCount++);
        trace.push({ kind: 'color', rawIndex, argument: canonical(row), valueBits: bits(row.value), returned: colorFor(row) });
        return colorFor(row);
    };
    const valueFormat = value => {
        const rawIndex = formatCount++;
        assert(Object.is(value, data[rawIndex].value));
        trace.push({ kind: 'value-format', rawIndex, argument: canonical(value), valueBits: bits(value), returned: formatFor(value) });
        return formatFor(value);
    };
    const borderColor = row => {
        if (!Object.hasOwn(row, 'data')) {
            emptyBorderCount++;
            assert.deepEqual(Object.keys(row), ['color']);
            assert.equal(row.color, input.emptyColor);
            trace.push({ kind: 'empty-border', argument: canonical(row), returned: '#334455' });
            return '#334455';
        }
        const rawIndex = groupBorderCount++;
        assert.equal(row.data, data[rawIndex]);
        assert.equal(row.groupIndex, rawIndex);
        assert.equal(row.borderColor, row.color);
        assert.deepEqual(row.polygons, []);
        trace.push({ kind: 'computed-border', rawIndex, argument: canonical(row), valueBits: bits(row.value), returned: borderFor(row) });
        return borderFor(row);
    };
    const forwardLegendData = value => effects.push(value);
    // These adapters execute a single cold-render pass. They are not React,
    // React scheduling, scale implementations, defs binding, or runtime memoization.
    const useWaffle = evaluate(expression.useWaffle, {
        useMemo: fn => fn(), useRef: current => ({ current }), useEffect: fn => fn(),
        useValueFormatter: fn => fn, useOrdinalColorScale: fn => fn,
        useInheritedColor: fn => fn, useTheme: () => Object.freeze({}),
        commonDefaultProps: Object.freeze({}),
        bindDefs: (defs, computed, fill) => {
            assert.deepEqual(defs, []); assert.deepEqual(fill, []);
            assert.equal(computed.length, data.length); return [];
        },
        computeGrid, mergeCellsData, findPolygons, isDataCell,
    });
    const result = useWaffle({ ...input, colors, valueFormat, borderColor, forwardLegendData, defs: [], fill: [] });
    assert.deepEqual(trace.map(row => row.kind), data.flatMap(() => ['color', 'value-format', 'computed-border']).concat('empty-border'));
    assert.equal(colorCount, data.length);
    assert.equal(formatCount, data.length);
    assert.equal(groupBorderCount, data.length);
    assert.equal(emptyBorderCount, 1);
    assert.equal(result.cells.length, 6);
    assert.equal(effects.length, 1);
    assert.equal(effects[0], result.legendData);
    assert.deepEqual(result.legendData.map(row => row.id), ['top', 'left'].includes(profile.fillDirection)
        ? [...data].reverse().map(row => row.id) : data.map(row => row.id));
    const occupancy = data.map((row, index) => {
        const group = result.computedData[index];
        assert.equal(group.data, row);
        assert(Object.is(group.value, row.value));
        assert.equal(group.formattedValue, formatFor(row.value));
        assert.equal(group.color, colorFor(row));
        assert.equal(group.borderColor, borderFor(group));
        const indices = result.cells.filter(cell => cell.data === group).map(cell => cell.index);
        const legend = result.legendData.find(entry => entry.id === row.id);
        assert.equal(legend.data, group);
        assert.equal(legend.color, group.color);
        for (const cell of result.cells.filter(cell => cell.data === group)) {
            assert.equal(cell.color, group.color); assert.equal(cell.borderColor, group.borderColor);
        }
        return { sourceIndex: index, rawValueBits: bits(row.value), requested: [group.startAt, group.endAt], occupiedIndices: indices,
            occupied: [Math.min(6, group.startAt), Math.min(6, group.endAt)], rawReferenceEqualsInput: true,
            actualCellReferenceEqualsComputedGroup: true, legendReferenceEqualsComputedGroup: true };
    });
    const occupiedCount = result.cells.filter(isDataCell).length;
    if (profile.name.endsWith('overflow-full')) {
        assert.equal(occupiedCount, 6);
        assert.deepEqual(occupancy.at(-1).requested, [5, 7]);
        assert.deepEqual(occupancy.at(-1).occupied, [5, 6]);
    } else if (profile.name.endsWith('sparse')) assert.equal(occupiedCount, 1);
    else assert.equal(occupiedCount, 0);
    assert.deepEqual(result.cells.map(cell => cell.key), result.cells.map(cell => `${cell.row}.${cell.column}`));
    inputs.push(canonical(input));
    outputs.push(canonical({ name: profile.name, trace, computedData: result.computedData, cells: result.cells,
        legendData: result.legendData, occupancy, occupiedCount, emptyCount: 6 - occupiedCount,
        counts: { colorCount, formatCount, groupBorderCount, emptyBorderCount }, actualForwardLegendReference: true }));
}
assert.equal(inputs[0].rawValueBits[0], '8000000000000000');
assert.equal(inputs[0].rawValueBits[3], '0000000000000000');
assert.equal(git('rev-parse', 'HEAD'), commit);
for (const name of names) assert.equal(sha(await readFile(resolve(checkout, 'packages', name))), sources[name].sha256);
const scope = {
    executed: 'Complete original useWaffle function (including original computedData useMemo closure), computeGrid, grid generation, mergeCellsData, findPolygons, polygon helpers and isDataCell.',
    adapters: 'Immediate single cold-render useMemo/useEffect/useRef; authored formatter/color/border callbacks; scale hooks return those callbacks; empty defs/fill binding stub only.',
    excluded: 'No React runtime/memo invalidation, colors-scale implementation, defs binding, Spring, browser/native input, Canvas, Rust execution or feature acceptance.',
    rustProfile: 'RadialDatum/String IDs remain the base API; generic raw D is an additive WaffleDataFrame wrapper. IndependentRound requested ranges and actual capacity-clipped occupancy are distinct. No claim that the fixed base API preserves arbitrary JavaScript D fields.',
};
const documents = {
    'inputs.json': JSON.stringify(inputs, null, 2) + '\n',
    'outputs.json': JSON.stringify(outputs, null, 2) + '\n',
    'extracts.json': JSON.stringify(extracts, null, 2) + '\n',
};
const reference = {
    passed: true, featureAccepted: false, commit, package: 'waffle@0.99.0', scope, compiler, sources,
    producer: { file: 'scripts/waffle-functions-reference.mjs', sha256: sha(await readFile(script)) },
    artifacts: Object.fromEntries(Object.entries(documents).map(([file, bytes]) => [file, { sha256: sha(bytes), bytes: Buffer.byteLength(bytes) }])),
    checks: { profiles: outputs.length, sourceGroups: outputs.reduce((n, row) => n + row.computedData.length, 0),
        callbackCalls: outputs.reduce((n, row) => n + row.trace.length, 0),
        emptyTemplateCalls: outputs.reduce((n, row) => n + row.counts.emptyBorderCount, 0),
        fullOccupancyProfiles: 4, sparseProfiles: 4, allHiddenProfiles: 1, emptyDataProfiles: 1,
        allRawGroupsSourceOrderColorFormatBorder: true, emptyBorderOnceAfterAllGroupsEvenWhenGridFull: true,
        temporaryBorderEqualsColorAndPolygonsEmpty: true, rawAndComputedReferencesVerifiedBeforeSerialization: true,
        signedZerosDistinguished: true, negativeZeroJsonTagged: true, originalAlgorithmsNotReimplemented: true },
};
await mkdir(out, { recursive: false });
for (const [name, bytes] of Object.entries(originals)) {
    const path = resolve(out, 'original', name);
    await mkdir(dirname(path), { recursive: true });
    await writeFile(path, bytes);
}
for (const [name, bytes] of Object.entries(documents)) await writeFile(resolve(out, name), bytes);
await writeFile(resolve(out, 'reference.json'), JSON.stringify(reference, null, 2) + '\n');
await writeFile(resolve(out, 'executed-driver.mjs'), await readFile(script));
await writeFile(resolve(out, 'README.md'), `# Nivo Waffle function reference\n\n${scope.executed}\n\n${scope.adapters}\n\n${scope.excluded}\n\n${scope.rustProfile}\n\nOriginal sources and MIT notices are preserved under original/. Callback argument snapshots are captured before polygon mutation. Object identity assertions execute before JSON serialization; signed zero is also recorded as IEEE-754 bits and a tagged JSON number.\n`);
process.stdout.write(JSON.stringify({ passed: true, out, commit, ...reference.checks }) + '\n');
