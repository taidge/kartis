// Execute extracted Nivo recipes. This is not React/useTransition time integration.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createRequire, stripTypeScriptTypes } from 'node:module';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const script = fileURLToPath(import.meta.url);
const workspace = resolve(dirname(script), '..');
const args = Object.fromEntries(process.argv.slice(2).map(argument => {
    const split = argument.indexOf('=');
    assert(split > 2, 'usage: node scripts/tree-motion-reference.mjs --nivo=<checkout> --out=<new-directory>');
    return [argument.slice(2, split), argument.slice(split + 1)];
}));
assert.deepEqual(Object.keys(args).sort(), ['nivo', 'out'], 'both explicit arguments required');
const checkout = resolve(args.nivo), out = resolve(args.out);
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const git = (...values) => execFileSync('git', ['-C', checkout, ...values], { encoding: 'utf8' }).trim();
const commit = git('rev-parse', 'HEAD');
assert.match(commit, /^[a-f0-9]{40}$/);
const require = createRequire(import.meta.url);
const bundle = resolve(workspace, 'node_modules/playwright/lib/transform/babelBundle.js');
const { babelParse, traverse } = require(bundle);
const compiler = {
    parser: 'existing Playwright Babel TSX parser',
    playwright: JSON.parse(await readFile(resolve(workspace, 'node_modules/playwright/package.json'))).version,
    parserBundleSha256: sha(await readFile(resolve(workspace, 'node_modules/playwright/lib/transform/babelBundleImpl.js'))),
    eraser: 'node:module.stripTypeScriptTypes (mode: strip); no formula rewriting',
    node: process.version,
};
const names = [
    'tree/src/Nodes.tsx', 'tree/src/Links.tsx', 'tree/src/Labels.tsx',
    'tree/src/Node.tsx', 'tree/src/Link.tsx', 'tree/src/Label.tsx',
    'tree/src/hooks.ts', 'tree/src/labelsHooks.ts', 'tree/src/types.ts',
    'tree/src/defaults.ts', 'tree/src/TreeCanvas.tsx', 'tree/package.json',
    'tree/LICENSE.md', 'core/src/hooks/useAnimatedPath.js', 'core/LICENSE.md',
];
const originals = {}, sources = {}, ast = {};
for (const name of names) {
    const bytes = await readFile(resolve(checkout, 'packages', name));
    const committed = execFileSync('git', ['-C', checkout, 'show', `${commit}:packages/${name}`]);
    assert(bytes.equals(committed), `reference source must equal the recorded commit: ${name}`);
    originals[name] = bytes;
    sources[name] = { sha256: sha(bytes), bytes: bytes.length };
    if (/\.[jt]sx?$/.test(name)) ast[name] = babelParse(bytes.toString('utf8'), name, true);
}
assert.equal(JSON.parse(originals['tree/package.json']).version, '0.99.0');
assert.match(originals['tree/LICENSE.md'].toString(), /Permission is hereby granted/);
assert.match(originals['core/LICENSE.md'].toString(), /Permission is hereby granted/);

const extracts = {};
function extract(file, name, visitor) {
    let found;
    traverse(ast[file], visitor(node => {
        assert.equal(found, undefined, `unique extract: ${file}:${name}`);
        found = node;
    }));
    assert(found, `missing upstream expression: ${file}:${name}`);
    const text = originals[file].toString('utf8').slice(found.start, found.end);
    const javascript = stripTypeScriptTypes(`(${text})`);
    extracts[`${file}:${name}`] = {
        file, start: found.start, end: found.end,
        startLine: found.loc.start.line, endLine: found.loc.end.line,
        typescript: text, javascript, sha256: sha(text), javascriptSha256: sha(javascript),
    };
    return javascript;
}
function variable(file, name) {
    return extract(file, name, found => ({
        VariableDeclarator(path) { if (path.node.id.name === name) found(path.node.init); },
    }));
}
const evaluate = (expression, dependencies = {}) => new Function(
    ...Object.keys(dependencies), `"use strict"; return (${expression});`,
)(...Object.values(dependencies));
const recipes = {};
for (const [role, file] of [['nodes', 'Nodes.tsx'], ['links', 'Links.tsx'], ['labels', 'Labels.tsx']]) {
    const path = `tree/src/${file}`;
    const regularTransition = evaluate(variable(path, 'regularTransition'));
    const leaveTransition = evaluate(variable(path, 'leaveTransition'));
    const options = extract(path, 'useTransitionOptions', found => ({
        CallExpression(path) {
            if (path.node.callee.name === 'useTransition') found(path.node.arguments[1]);
        },
    }));
    const springConfig = Object.freeze({ authoredProbe: 'identity only; no spring evaluator' });
    const enabled = evaluate(options, { regularTransition, leaveTransition, springConfig, animate: true });
    const disabled = evaluate(options, { regularTransition, leaveTransition, springConfig, animate: false });
    for (const configuration of [enabled, disabled]) {
        assert.equal(configuration.from, regularTransition);
        assert.equal(configuration.enter, regularTransition);
        assert.equal(configuration.update, regularTransition);
        assert.equal(configuration.leave, leaveTransition);
        assert.equal(configuration.config, springConfig);
    }
    assert.equal(enabled.immediate, false);
    assert.equal(disabled.immediate, true);
    recipes[role] = enabled;
}

// Execute the original ID assignment and computed endpoint replacement as well.
const uidAssignment = extract('tree/src/hooks.ts', 'uidAssignment', found => ({
    AssignmentExpression(path) {
        const left = path.node.left;
        if (left.type === 'MemberExpression' && left.object.name === 'node' && left.property.name === 'uid') found(path.node);
    },
}));
const intermediateLinks = variable('tree/src/hooks.ts', 'intermediateLinks');
const labelMap = extract('tree/src/labelsHooks.ts', 'labelsMap', found => ({
    CallExpression(path) {
        const callee = path.node.callee;
        if (callee.type === 'MemberExpression' && callee.object.name === 'nodes' && callee.property.name === 'map') found(path.node);
    },
}));

const numberBits = value => {
    const buffer = Buffer.alloc(8);
    buffer.writeDoubleBE(value);
    return buffer.toString('hex');
};
const canonical = value => JSON.parse(JSON.stringify(value, (_key, item) =>
    typeof item === 'number' && Object.is(item, -0) ? { $number: '-0' } : item));
function freeze(value) {
    if (value && typeof value === 'object' && !Object.isFrozen(value)) {
        Object.values(value).forEach(freeze);
        Object.freeze(value);
    }
    return value;
}

function authoredInput(name, { reverse = false, reparent = false, update = false } = {}) {
    // Deliberately not hierarchy order. Two distinct raw objects share business ID leaf.
    const descriptors = [
        { slot: 'leafB', id: 'leaf', parent: 'b', value: -0, label: '' },
        { slot: 'b', id: 'b', parent: 'r', value: 10, label: 'right' },
        { slot: 'r', id: 'r', parent: null, value: +0, label: 'root' },
        { slot: 'leafA', id: 'leaf', parent: 'a', value: update ? 35 : 30, label: 'left leaf' },
        { slot: 'a', id: 'a', parent: reparent ? 'b' : 'r', value: +0, label: 'left' },
    ];
    const raw = descriptors.map((row, sourceIndex) => ({
        id: row.id, value: row.value, label: row.label, extra: { slot: row.slot, sourceIndex },
    }));
    const bySlot = Object.fromEntries(descriptors.map((row, index) => [row.slot, index]));
    const nodeBySlot = {}, positions = {};
    const geometry = [[412.25, 88.5], [236, 241], [43, 169.75], [412.25, 124.5], [236, 124.5]];
    function node(slot) {
        if (nodeBySlot[slot]) return nodeBySlot[slot];
        const index = bySlot[slot], row = descriptors[index];
        const parent = row.parent === null ? null : node(row.parent);
        const ancestorIds = parent ? [...parent.ancestorIds, parent.id] : [];
        const sourceNode = { data: raw[index] };
        evaluate(uidAssignment, { node: sourceNode, ancestorIds, getIdentity: datum => datum.id });
        const point = geometry[index];
        const value = {
            path: [...ancestorIds, row.id], uid: sourceNode.uid,
            isRoot: parent === null, isLeaf: !descriptors.some(other => other.parent === slot),
            ancestorIds, ancestorUids: parent ? [...parent.ancestorUids, parent.uid] : [],
            descendantUids: [], id: row.id, data: raw[index], depth: ancestorIds.length,
            height: 0, x: point[0] + (update ? 17.5 : 0), y: point[1] + (update ? -9.25 : 0),
            size: index === 0 ? 0 : (update ? 28 : 20) + index,
            color: index === 4 ? '#12345600' : update ? '#ff8800' : '#123456',
            isActive: update ? slot === 'leafA' : null,
        };
        nodeBySlot[slot] = value;
        positions[value.uid] = { x: value.x + 12.5, y: value.y - 3, rotation: update ? -90 : 0, textAnchor: 'start', baseline: 'middle' };
        return value;
    }
    descriptors.forEach(row => node(row.slot));
    function descendants(slot) {
        return descriptors.filter(row => row.parent === slot).flatMap(row => [row.slot, ...descendants(row.slot)]);
    }
    for (const row of descriptors) {
        const value = node(row.slot);
        value.descendantUids = descendants(row.slot).map(child => node(child).uid);
        value.height = Math.max(0, ...descendants(row.slot).map(child => node(child).depth - value.depth));
    }
    const traversal = reparent ? ['r', 'b', 'leafB', 'a', 'leafA'] : ['r', 'a', 'b', 'leafA', 'leafB'];
    const order = reverse ? [...traversal].reverse() : traversal;
    const nodes = order.map(node);
    const nodeByUid = Object.fromEntries(nodes.map(value => [value.uid, value]));
    const hierarchyLinks = order.filter(slot => descriptors[bySlot[slot]].parent !== null).map(slot => ({
        source: node(descriptors[bySlot[slot]].parent), target: node(slot),
    }));
    const mappedLinks = evaluate(intermediateLinks, {
        useMemo: callback => callback(), root: { links: () => hierarchyLinks }, nodeByUid,
    });
    const links = mappedLinks.map((link, index) => ({
        ...link, thickness: index === 0 ? 0 : update ? 7 : 3,
        color: index === 1 ? '#ff000000' : '#aa4400', isActive: update ? index === 2 : null,
    }));
    const labels = evaluate(labelMap, {
        nodes, getLabel: value => value.data.label, getPosition: value => positions[value.uid],
    });
    for (const [index, value] of nodes.entries()) assert.equal(value.data, raw[bySlot[order[index]]]);
    for (const [index, link] of links.entries()) {
        assert.equal(link.source, nodeByUid[hierarchyLinks[index].source.uid]);
        assert.equal(link.target, nodeByUid[hierarchyLinks[index].target.uid]);
    }
    labels.forEach((label, index) => assert.equal(label.node, nodes[index]));
    return freeze({
        name, descriptors, raw, nodes, links, labels,
        identities: {
            rawSourceOrder: descriptors.map(row => row.slot), computedNodeOrder: order,
            rawValueBits: raw.map(row => numberBits(row.value)),
            nodeDataSourceIndices: nodes.map(value => raw.indexOf(value.data)),
            linkEndpointNodeIndices: links.map(link => [nodes.indexOf(link.source), nodes.indexOf(link.target)]),
            labelNodeIndices: labels.map(label => nodes.indexOf(label.node)),
            nodeKeys: nodes.map(recipes.nodes.keys), linkKeys: links.map(recipes.links.keys), labelKeys: labels.map(recipes.labels.keys),
            parentChildReferencesExact: true, labelNodeReferencesExact: true, rawDataReferencesExact: true,
        },
    });
}

const inputs = [
    authoredInput('baseline'), authoredInput('same-identities-update', { update: true }),
    authoredInput('source-computed-order-reversed', { reverse: true }),
    authoredInput('parent-change', { reparent: true }),
];
const outputs = inputs.map(input => {
    const result = { name: input.name };
    for (const role of ['nodes', 'links', 'labels']) {
        const recipe = recipes[role];
        result[role] = input[role].map(item => {
            const from = recipe.from(item), enter = recipe.enter(item), update = recipe.update(item), leave = recipe.leave(item);
            assert.deepEqual(from, enter, `${role}: from is final entry target`);
            assert.deepEqual(update, enter, `${role}: update uses regular target`);
            if (role === 'nodes') {
                assert.equal(enter.size, item.size);
                assert.equal(leave.size, 0);
                assert.deepEqual({ ...leave, size: item.size }, enter);
            } else assert.deepEqual(leave, enter, `${role}: no explicit changed leave target`);
            return { key: recipe.keys(item), from, enter, update, leave };
        });
        assert.equal(new Set(result[role].map(row => row.key)).size, input[role].length);
        assert.deepEqual(result[role].map(row => row.key), input.identities[`${role === 'nodes' ? 'node' : role === 'links' ? 'link' : 'label'}Keys`]);
    }
    return result;
});
const baseline = inputs[0], reversed = inputs[2], reparented = inputs[3];
assert.equal(baseline.nodes.filter(node => node.id === 'leaf').length, 2);
assert.notEqual(...baseline.nodes.filter(node => node.id === 'leaf').map(recipes.nodes.keys));
assert.deepEqual(reversed.identities.nodeKeys, [...baseline.identities.nodeKeys].reverse());
assert.deepEqual(inputs[1].identities.nodeKeys, baseline.identities.nodeKeys);
assert.notEqual(reparented.nodes.find(node => node.id === 'a').uid, baseline.nodes.find(node => node.id === 'a').uid);
assert.equal(baseline.identities.rawValueBits[0], '8000000000000000');
assert.equal(baseline.identities.rawValueBits[2], '0000000000000000');
assert(inputs[0].labels.some(label => label.label === ''));
assert(inputs[0].nodes.some(node => node.size === 0));
assert(inputs[0].nodes.some(node => node.color.endsWith('00')));
for (const input of inputs) assert.deepEqual(input.identities.nodeKeys, input.identities.labelKeys);
const scope = {
    executed: 'Six unmodified transition expressions, actual useTransition option/key expressions, original uid assignment, original computed-link useMemo and label-map expressions.',
    authored: 'Finite raw rows, hierarchy protocol/order, positions, styles, activity and label-position dependency. This is not D3 layout execution.',
    excluded: 'No React rendering, useTransition lifecycle, React Spring integration/timing/velocity/exit duration, DOM, pixels, Canvas animation, or runtime parity claim.',
};
const inputBytes = JSON.stringify(canonical(inputs), null, 2) + '\n';
const outputBytes = JSON.stringify(canonical(outputs), null, 2) + '\n';
const extractBytes = JSON.stringify(extracts, null, 2) + '\n';
const reference = {
    passed: true, commit, package: 'tree@0.99.0', scope, compiler,
    producer: { file: 'scripts/tree-motion-reference.mjs', sha256: sha(await readFile(script)) },
    sources,
    artifacts: {
        inputs: { file: 'inputs.json', sha256: sha(inputBytes) },
        outputs: { file: 'outputs.json', sha256: sha(outputBytes) },
        extracts: { file: 'extracts.json', sha256: sha(extractBytes) },
    },
    checks: {
        cases: inputs.length, transitionRows: outputs.reduce((total, value) => total + value.nodes.length + value.links.length + value.labels.length, 0),
        originalFunctions: 6, originalOptions: 3, actualSharedEndpointReferences: true,
        actualSharedLabelNodeReferences: true, actualRawDataReferences: true,
        distinctUidDespiteDuplicateBusinessId: true, reversedOrderPreserved: true,
        sameIdentityOnUpdate: true, changedAncestorChangesUid: true,
        enterUsesRegularFinalTarget: true, onlyNodeLeaveShrinksSize: true,
        linkLabelLeaveEqualsRegular: true, animateFalseIsImmediate: true,
        signedZerosDistinguished: true,
    },
};
assert.equal(git('rev-parse', 'HEAD'), commit, 'upstream commit unchanged during execution');
for (const name of names) assert.equal(sha(await readFile(resolve(checkout, 'packages', name))), sources[name].sha256);
await mkdir(out, { recursive: false });
for (const [name, bytes] of Object.entries(originals)) {
    const path = resolve(out, 'original', name);
    await mkdir(dirname(path), { recursive: true });
    await writeFile(path, bytes);
}
await writeFile(resolve(out, 'inputs.json'), inputBytes);
await writeFile(resolve(out, 'outputs.json'), outputBytes);
await writeFile(resolve(out, 'extracts.json'), extractBytes);
await writeFile(resolve(out, 'reference.json'), JSON.stringify(reference, null, 2) + '\n');
await writeFile(resolve(out, 'executed-driver.mjs'), await readFile(script));
await writeFile(resolve(out, 'README.md'), `# Nivo Tree transition recipes\n\n${scope.executed}\n\n${scope.authored}\n\n${scope.excluded}\n\nUpstream commit: ${commit}. Complete original files and MIT notices are archived under original/. inputs.json preserves negative zero with a tagged number plus explicit IEEE-754 bits. Endpoint/node reference assertions ran before JSON serialization; index certificates record their exact source-pool identity.\n`);
process.stdout.write(JSON.stringify({ passed: true, out, commit, ...reference.checks }) + '\n');
