// Verify-only original Nivo 0.99 StreamDots createElement-argument oracle.
// React rendering/lifecycle and Rust nullable extensions are outside its scope.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {readFile, mkdir, writeFile} from 'node:fs/promises';
import {createRequire, stripTypeScriptTypes} from 'node:module';
import {resolve} from 'node:path';
import {execFileSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';

const root = resolve(fileURLToPath(new URL('..', import.meta.url)));
if (!process.argv[2]) throw Error('usage: stream-dot-widget-reference.mjs <nivo-checkout> [output-directory] | --frozen [fixture-directory]');
const frozen = process.argv[2] === '--frozen';
const nivo = frozen ? undefined : resolve(process.argv[2]);
const output = resolve(process.argv[3] ?? resolve(root, 'tests/fixtures/stream-dot-widgets'));
const expected = frozen ? JSON.parse(await readFile(resolve(output, 'reference.json'), 'utf8')) : undefined;
const require = createRequire(import.meta.url);
const {babelParse, traverse} = require(resolve(root, 'node_modules/playwright/lib/transform/babelBundle.js'));
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const paths = ['StreamDots.tsx', 'StreamDotsItem.tsx', 'Stream.tsx', 'hooks.ts', 'types.ts', 'props.ts'];
const original = Object.fromEntries(await Promise.all(paths.map(async name => [name,
  await readFile(frozen ? resolve(output, 'original', name) : resolve(nivo, 'packages/stream/src', name)),
])));
const license = await readFile(frozen ? resolve(output, 'original/LICENSE.md') : resolve(nivo, 'packages/stream/LICENSE.md'));
const originalSources = Object.fromEntries(Object.entries(original).map(([name,bytes])=>[name,{sha256:sha(bytes),bytes:bytes.length}]));
const licenseReference = {file:'original/LICENSE.md',sha256:sha(license),bytes:license.length};
const producer = {file:'scripts/stream-dot-widget-reference.mjs',sha256:sha(await readFile(fileURLToPath(import.meta.url)))};
if (frozen) {
  assert.deepEqual(originalSources, expected.originalSources, 'frozen original source SHA/length');
  assert.deepEqual(licenseReference, expected.license, 'frozen complete MIT license SHA/length');
  assert.deepEqual(producer, expected.producer, 'frozen producer SHA');
  assert.equal(expected.package, '0.99.0', 'frozen Nivo package version');
  assert.match(expected.commit, /^[0-9a-f]{40}$/, 'recorded original checkout commit');
}
const dots = original['StreamDots.tsx'].toString('utf8');
const stream = original['Stream.tsx'].toString('utf8');
let mapNode;
traverse(babelParse(dots, 'StreamDots.tsx', true), {CallExpression(path) {
  const node = path.node;
  if (node.callee.type === 'MemberExpression' && node.callee.object.name === 'data' && node.callee.property.name === 'map') {
    assert.equal(mapNode, undefined, 'unambiguous original data.map'); mapNode = node;
  }
}});
assert.ok(mapNode);
const getY = dots.slice(dots.indexOf('const getDotY ='), dots.indexOf('export const StreamDots'));
assert.ok(getY.startsWith('const getDotY ='));
const expression = dots.slice(mapNode.start, mapNode.end);
const execute = new Function('data', 'position', 'dotComponent', 'createElement', 'getSize', 'getColor', 'getBorderWidth', 'getBorderColor',
  stripTypeScriptTypes(`${getY}\nfunction run() { return ${expression}; }\n`) + '\nreturn run();');
let gate;
traverse(babelParse(stream, 'Stream.tsx', true), {IfStatement(path) {
  const source = stream.slice(path.node.test.start, path.node.test.end);
  if (source === "chartLayers.includes('dots') && enableDots") { assert.equal(gate, undefined); gate = source; }
}});
assert.ok(gate);
const runGate = new Function('chartLayers', 'enableDots', 'isInteractive', `return ${gate};`);
const item = original['StreamDotsItem.tsx'].toString('utf8');
let spring;
traverse(babelParse(item, 'StreamDotsItem.tsx', true), {CallExpression(path) {
  if (path.node.callee.name === 'useSpring') { assert.equal(spring, undefined); spring = item.slice(path.node.arguments[0].start,path.node.arguments[0].end); }
}});
assert.ok(spring);
const runSpring = new Function('x','y','size','color','springConfig','animate',`return (${spring});`);
const defaultCircleInputs = [0,6,12].map(size => {
  const result = runSpring(31,93,size,'#123456',{mass:1},false);
  assert.equal(result.radius,size*0.5); assert.equal(result.immediate,true);
  return {size,result};
});
const data = [
  {layerId:'A',layerLabel:'alpha',index:0,value:4,formattedValue:'four',color:'#123456',x:31,y1:153,y2:73},
  {layerId:'A',layerLabel:'alpha',index:1,value:0,formattedValue:'zero',color:'#123456',x:136,y1:93,y2:93},
  {layerId:'A',layerLabel:'alpha',index:2,value:-3,formattedValue:'negative',color:'#123456',x:241,y1:53,y2:113},
];
const component = {};
const cases = [];
for (const position of ['start','center','end']) for (const size of ['zero','six','accessor']) {
  const calls = [];
  const get = (name, fn) => datum => { calls.push([name, datum.index]); return fn(datum); };
  const nodes = execute(data, position, component, (type, props) => {
    assert.equal(type, component); assert.equal(props.datum, data[props.key]);
    calls.push(['createElement', props.key]); return props;
  }, get('size', d => size === 'zero' ? 0 : size === 'six' ? 6 : d.index * 2),
  get('color', d => d.color), get('borderWidth', d => d.index), get('borderColor', () => '#abcdef'));
  assert.equal(nodes.length, 3);
  assert.deepEqual(nodes.map(n => n.y), {start:[153,93,53],center:[113,93,83],end:[73,93,113]}[position]);
  assert.deepEqual(nodes.map(n => n.size), size === 'zero' ? [0,0,0] : size === 'six' ? [6,6,6] : [0,2,4]);
  assert.deepEqual(calls, data.flatMap(d => ['size','color','borderWidth','borderColor','createElement'].map(name => [name,d.index])));
  cases.push({position, size, nodes, calls});
}
const gateCases = [];
for (const chartLayers of [[], ['dots'], ['layers','dots','slices']]) for (const enableDots of [false,true]) for (const isInteractive of [false,true]) {
  const result = runGate(chartLayers,enableDots,isInteractive);
  assert.equal(result, chartLayers.includes('dots') && enableDots);
  gateCases.push({chartLayers,enableDots,isInteractive,result});
}
// Preserve an actual runtime-null invocation separately. The public TS datum
// is number-only; this is not an official nullable Nivo component contract.
const runtimeNull = execute([{...data[1],value:null}], 'center', component, (_type, props) => props,
  () => 0, d => d.color, () => 0, () => '#abcdef');
const reference = {
  scope:'Original getDotY and data.map executed unchanged; createElement arguments before React reserved-key handling. No React lifecycle, pixel oracle, or official nullable-prop claim.',
  package:frozen ? expected.package : JSON.parse(await readFile(resolve(nivo,'packages/stream/package.json'),'utf8')).version,
  commit:frozen ? expected.commit : execFileSync('git',['-C',nivo,'rev-parse','HEAD'],{encoding:'utf8'}).trim(),
  originalSources,
  license:licenseReference,
  producer,
  execution:{node:process.version,getDotY:getY,map:expression,gate,defaultCircleSpringInput:spring},
  input:data,cases,gateCases,defaultCircleInputs,runtimeNullOutsidePublicType:runtimeNull,
};
if (frozen) {
  for (const key of ['scope','input','cases','gateCases','defaultCircleInputs','runtimeNullOutsidePublicType']) {
    assert.deepEqual(reference[key], expected[key], `frozen ${key} execution result`);
  }
  // Node patch versions are diagnostic. Source extracts and all computed
  // values are checked exactly; no numeric tolerances or fixture writes.
  const {node: _currentNode, ...actualExecution} = reference.execution;
  const {node: _recordedNode, ...expectedExecution} = expected.execution;
  assert.deepEqual(actualExecution, expectedExecution, 'frozen exact executed source expressions');
} else {
  await mkdir(output, {recursive:true}); await mkdir(resolve(output, 'original'), {recursive:true});
  for (const [name, bytes] of Object.entries(original)) await writeFile(resolve(output, 'original', name), bytes);
  await writeFile(resolve(output, 'original/LICENSE.md'), license);
  await writeFile(resolve(output,'reference.json'), JSON.stringify(reference,null,2)+'\n');
}
console.log(JSON.stringify({mode:frozen?'frozen-verify':'produce',output,producerSha256:producer.sha256,
  sourceFiles:paths.length,cases:cases.length,createElementCalls:cases.reduce((n,c)=>n+c.nodes.length,0),
  gateCases:gateCases.length,defaultCircleInputs:defaultCircleInputs.length,runtimeNullOutsidePublicType:runtimeNull.length}));
