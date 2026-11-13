// Run the unmodified d3-brush package in a real browser. No Rust geometry is
// imported, and expected selections/handles are read directly from D3's DOM.
import assert from 'node:assert/strict';
import {execFileSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {readFile, writeFile} from 'node:fs/promises';
import {join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {chromium, firefox, webkit} from 'playwright';

const root = fileURLToPath(new URL('../', import.meta.url));
const target = JSON.parse(execFileSync('cargo', ['metadata', '--offline', '--no-deps', '--format-version', '1'],
  {cwd: root, encoding: 'utf8', windowsHide: true})).target_directory;
const packages = join(target, 'd3-brush-reference', 'node_modules');
const packageNames = ['d3-dispatch', 'd3-selection', 'd3-drag', 'd3-color', 'd3-interpolate',
  'd3-timer', 'd3-ease', 'd3-transition', 'd3-brush'];
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const vendorPaths = {
  bundle: 'tests/fixtures/editable-brush-d3-oracle.js',
  licenses: 'tests/fixtures/editable-brush-d3-LICENSE.txt',
  manifest: 'tests/fixtures/editable-brush-d3-source.json',
};
// Preparation is explicit. Ordinary --check reads the original vendored code,
// so browser acceptance never depends on a machine's npm installation cache.
if (process.argv.includes('--prepare')) {
  const sources = [], parts = [], bundle = [], licenses = [];
  let bundleBytes = 0, licenseBytes = 0;
  const appendBundle = bytes => { bytes = Buffer.from(bytes); bundle.push(bytes); bundleBytes += bytes.length; };
  const appendLicense = bytes => { bytes = Buffer.from(bytes); licenses.push(bytes); licenseBytes += bytes.length; };
  appendBundle('// Development-only independent oracle. Original UMD bytes and full licenses follow.\n');
  appendLicense('Development-only d3-brush oracle dependencies. All original license texts follow.\n');
  for (const name of packageNames) {
    const dir = join(packages, name), pkg = JSON.parse(await readFile(join(dir, 'package.json'), 'utf8'));
    if (name === 'd3-brush') assert.equal(pkg.version, '3.0.0');
    const dist = await readFile(join(dir, 'dist', `${name}.js`)), license = await readFile(join(dir, 'LICENSE'));
    const source = {name, version: pkg.version, license: pkg.license, distSha256: sha(dist), licenseSha256: sha(license)};
    sources.push(source);
    const provenance = `https://www.npmjs.com/package/${name}/v/${pkg.version}`;
    appendBundle(`\n/* ${name}@${pkg.version} | ${pkg.license}\n${provenance}\nOriginal dist SHA-256 ${source.distSha256}\nOriginal LICENSE SHA-256 ${source.licenseSha256}\n`);
    appendBundle(license); appendBundle(`*/\n// BEGIN ORIGINAL ${name}@${pkg.version}\n`);
    const distStart = bundleBytes; appendBundle(dist); const distEnd = bundleBytes;
    appendBundle(`\n// END ORIGINAL ${name}@${pkg.version}\n`);
    appendLicense(`\n----- ${name}@${pkg.version} | ${pkg.license} -----\n${provenance}\n`);
    const licenseStart = licenseBytes; appendLicense(license); const licenseEnd = licenseBytes;
    parts.push({name, version: pkg.version, provenance, distStart, distEnd, licenseStart, licenseEnd});
  }
  const bundleContent = Buffer.concat(bundle), licenseContent = Buffer.concat(licenses);
  const source = {name: 'd3-brush', version: '3.0.0', url: 'https://github.com/d3/d3-brush/blob/v3.0.0/src/brush.js',
    sourceSha256: sha(await readFile(join(packages, 'd3-brush/src/brush.js'))), packages: sources};
  const manifest = {scope: 'Development-only original UMD packages; not a Kartis runtime dependency.', source,
    bundle: {path: vendorPaths.bundle, bytes: bundleContent.length, sha256: sha(bundleContent)},
    licenses: {path: vendorPaths.licenses, bytes: licenseContent.length, sha256: sha(licenseContent)}, parts};
  for (const [path, content] of [[vendorPaths.bundle, bundleContent], [vendorPaths.licenses, licenseContent],
    [vendorPaths.manifest, Buffer.from(JSON.stringify(manifest, null, 2) + '\n')]]) {
    if (process.argv.includes('--check')) assert.deepEqual(await readFile(join(root, path)), content, `${path}: original package preparation differs`);
    else await writeFile(join(root, path), content);
  }
}
const sourceManifest = JSON.parse(await readFile(join(root, vendorPaths.manifest), 'utf8'));
const bundleContent = await readFile(join(root, vendorPaths.bundle));
const licenseContent = await readFile(join(root, vendorPaths.licenses));
assert.equal(sourceManifest.bundle.sha256, sha(bundleContent));
assert.equal(sourceManifest.bundle.bytes, bundleContent.length);
assert.equal(sourceManifest.licenses.sha256, sha(licenseContent));
assert.equal(sourceManifest.licenses.bytes, licenseContent.length);
const sources = sourceManifest.source.packages;
assert.deepEqual(sources.map(p => p.name), packageNames);
assert.equal(sourceManifest.source.version, '3.0.0');
for (const [index, source] of sources.entries()) {
  const part = sourceManifest.parts[index];
  assert.equal(part.name, source.name); assert.equal(part.version, source.version);
  assert.equal(sha(bundleContent.subarray(part.distStart, part.distEnd)), source.distSha256, `${source.name}: original UMD bytes changed`);
  assert.equal(sha(licenseContent.subarray(part.licenseStart, part.licenseEnd)), source.licenseSha256, `${source.name}: original LICENSE bytes changed`);
}
const sourceSha256 = sourceManifest.source.sourceSha256;
const extent = [10, 20, 310, 220], initial = [80, 70, 220, 170];
const down = (target, point, modifiers = {}) => ({type: 'down', target, point, modifiers});
const move = point => ({type: 'move', point});
const up = point => ({type: 'up', point});
const key = (key, down) => ({type: 'key', key, down});
const program = selection => ({type: 'program', selection});
const cases = [];
const add = (id, axis, seed, steps, extra = {}) => cases.push({id, axis, extent, initial: seed,
  handleSize: 10, profile: 'geometry', steps, ...extra});
const starts = {n: [150, 70], w: [80, 120], e: [220, 120], s: [150, 170],
  nw: [80, 70], ne: [220, 70], sw: [80, 170], se: [220, 170]};
const outward = {n: [150, 35], w: [30, 120], e: [290, 120], s: [150, 205],
  nw: [35, 40], ne: [285, 40], sw: [35, 205], se: [285, 205]};
const flipped = {n: [[150, 190], [150, 210]], w: [[250, 120], [280, 120]],
  e: [[50, 120], [25, 120]], s: [[150, 45], [150, 25]],
  nw: [[250, 190], [280, 210]], ne: [[50, 190], [25, 210]],
  sw: [[250, 45], [280, 25]], se: [[50, 45], [25, 25]]};
for (const handle of Object.keys(starts)) {
  add(`resize-${handle}`, 'xy', initial, [down(handle, starts[handle]), move(outward[handle]), up(outward[handle])]);
  add(`flip-${handle}`, 'xy', initial, [down(handle, starts[handle]), ...flipped[handle].map(move), up(flipped[handle].at(-1))]);
}
for (const [suffix, end] of [['nw', [-50, -50]], ['ne', [390, -50]], ['sw', [-50, 290]], ['se', [390, 290]]]) {
  add(`move-clamp-${suffix}`, 'xy', initial, [down('selection', [150, 120]), move(end), up(end)]);
  add(`create-clamp-${suffix}`, 'xy', null, [down('overlay', [150, 120]), move(end), up(end)]);
}
add('create-reverse', 'xy', null, [down('overlay', [260, 190]), move([40, 45]), up([40, 45])]);
add('create-click-clears', 'xy', null, [down('overlay', [120, 100]), up([120, 100])]);
add('resize-zero-clears', 'xy', initial, [down('e', starts.e), move([80, 120]), up([80, 120])]);
add('create-noop-move', 'xy', null, [down('overlay', [100, 90]), move([160, 130]), move([160, 130]), up([160, 130])]);
for (const handle of ['n', 'w', 'e', 's', 'nw', 'ne', 'sw', 'se'])
  add(`alt-center-${handle}`, 'xy', initial, [down(handle, starts[handle], {alt: true}), move(outward[handle]), up(outward[handle])]);
add('alt-toggle-resize', 'xy', initial, [down('se', starts.se), move([240, 185]), key('Alt', true),
  move([260, 195]), key('Alt', false), move([280, 205]), up([280, 205])]);
add('alt-center-create', 'xy', null, [down('overlay', [150, 120], {alt: true}), move([220, 160]),
  move([350, 260]), up([350, 260])]);
add('space-toggle-resize', 'xy', initial, [down('se', starts.se), move([240, 185]), key('Space', true),
  move([260, 195]), key('Space', false), move([280, 205]), up([280, 205])]);
add('space-alt-priority', 'xy', initial, [down('nw', starts.nw, {alt: true}), move([60, 50]), key('Space', true),
  move([90, 75]), key('Space', false), move([100, 90]), key('Alt', false), move([110, 95]), up([110, 95])]);
add('space-clamp-create', 'xy', null, [down('overlay', [60, 60]), move([160, 130]), key('Space', true),
  move([390, 290]), move([-20, -30]), key('Space', false), move([80, 100]), up([80, 100])]);
add('shift-horizontal-create', 'xy', null, [down('overlay', [90, 80], {shift: true}), move([170, 100]),
  move([180, 160]), key('Shift', false), move([190, 170]), up([190, 170])]);
add('shift-vertical-create', 'xy', null, [down('overlay', [90, 80], {shift: true}), move([110, 160]),
  move([180, 180]), key('Shift', false), move([190, 190]), up([190, 190])]);
add('shift-equal-delta-create', 'xy', null, [down('overlay', [90, 80], {shift: true}), move([130, 120]), up([130, 120])]);
add('shift-toggle-resize', 'xy', initial, [down('se', starts.se), move([230, 180]), key('Shift', true),
  move([270, 190]), move([280, 215]), key('Shift', false), move([290, 210]), up([290, 210])]);
add('shift-selection-lock', 'xy', initial, [down('selection', [150, 120], {shift: true}), move([180, 125]),
  move([200, 180]), key('Shift', false), move([205, 185]), up([205, 185])]);
add('meta-force-create-selection', 'xy', initial, [down('selection', [150, 120], {meta: true}), move([260, 190]), up([260, 190])]);
add('meta-force-create-handle', 'xy', initial, [down('nw', starts.nw, {meta: true}), move([40, 40]), up([40, 40])]);
for (const axis of ['x', 'y']) {
  const seed = axis === 'x' ? [80, 20, 220, 220] : [10, 70, 310, 170];
  const near = axis === 'x' ? 'w' : 'n', far = axis === 'x' ? 'e' : 's';
  const nearPoint = axis === 'x' ? [80, 120] : [160, 70];
  const farPoint = axis === 'x' ? [220, 120] : [160, 170];
  add(`${axis}-create-inactive-movement`, axis, null, [down('overlay', [100, 80]), move([200, 180]), up([200, 180])]);
  add(`${axis}-move-clamp`, axis, seed, [down('selection', [150, 120]), move([390, 290]), move([-20, -30]), up([-20, -30])]);
  add(`${axis}-resize-${near}`, axis, seed, [down(near, nearPoint), move([30, 35]), up([30, 35])]);
  add(`${axis}-resize-${far}`, axis, seed, [down(far, farPoint), move([290, 205]), up([290, 205])]);
  add(`${axis}-flip`, axis, seed, [down(far, farPoint), move([30, 35]), up([30, 35])]);
  add(`${axis}-alt-space`, axis, seed, [down(far, farPoint, {alt: true}), move([260, 195]), key('Space', true),
    move([280, 205]), key('Space', false), move([290, 215]), up([290, 215])]);
  add(`${axis}-shift-ignored`, axis, null, [down('overlay', [100, 80], {shift: true}), move([200, 180]), up([200, 180])]);
  add(`${axis}-zero-active-clears`, axis, null, [down('overlay', [100, 80]), move(axis === 'x' ? [100, 180] : [200, 80]),
    up(axis === 'x' ? [100, 180] : [200, 80])]);
}
add('ctrl-filter', 'xy', initial, [down('selection', [150, 120], {ctrl: true}), move([200, 160]), up([200, 160])], {profile: 'filter'});
add('meta-disabled', 'xy', initial, [down('selection', [150, 120], {meta: true}), move([200, 160]), up([200, 160])], {keys: false});
add('alt-disabled', 'xy', initial, [down('se', starts.se, {alt: true}), move([270, 200]), up([270, 200])], {keys: false});
add('program-change-repeat-clear', 'xy', initial, [program([40, 45, 180, 150]), program([40, 45, 180, 150]),
  program(null), program(null)], {profile: 'd3-program'});
add('program-outside-extent', 'xy', null, [program([-20, -30, 350, 260])], {profile: 'd3-program'});
add('program-zero-kept', 'xy', null, [program([100, 80, 100, 180])], {profile: 'd3-program'});
add('program-mid-gesture', 'xy', initial, [down('se', starts.se), move([240, 180]), program([40, 40, 180, 150]),
  move([260, 200]), up([260, 200])], {profile: 'd3-program-reentrant'});
add('callback-replace-on-start', 'xy', initial, [down('se', starts.se), move([260, 200]), up([260, 200])],
  {profile: 'd3-program-reentrant', callback: {phase: 'start', selection: [40, 40, 180, 150]}});
add('callback-replace-on-change', 'xy', initial, [down('se', starts.se), move([240, 180]), move([260, 200]), up([260, 200])],
  {profile: 'd3-program-reentrant', callback: {phase: 'brush', selection: [40, 40, 180, 150]}});
add('callback-clear-on-start', 'xy', initial, [down('se', starts.se), up(starts.se)],
  {profile: 'd3-program-reentrant', callback: {phase: 'start', selection: null}});
add('x-inactive-zero', 'x', null, [down('overlay', [100, 20]), move([200, 20]), up([200, 20])],
  {profile: 'd3-degenerate', extent: [10, 20, 310, 20], dispatch: 'target'});
add('y-inactive-zero', 'y', null, [down('overlay', [10, 80]), move([10, 180]), up([10, 180])],
  {profile: 'd3-degenerate', extent: [10, 20, 10, 220], dispatch: 'target'});

const edgeCases = process.argv.includes('--edge-cases');
if (edgeCases) {
  cases.length = 0;
  add('center-clamped-space-outside-extent', 'x', [0, -80, 100, 180],
    [down('w', [0, 50], {alt: true}), move([-500, 50]), key('Space', true), up([-500, 50])],
    {profile: 'd3-safety-difference', extent: [-100, -80, 200, 180]});
}

const browserNameIndex = process.argv.indexOf('--browser');
const browserName = browserNameIndex < 0 ? 'chrome' : process.argv[browserNameIndex + 1];
assert(['chrome', 'edge', 'firefox', 'webkit'].includes(browserName), 'Choose --browser chrome, edge, firefox or webkit');
const executableIndex = process.argv.indexOf('--executable');
const executable = executableIndex < 0 ? undefined : process.argv[executableIndex + 1];
assert(executableIndex < 0 || typeof executable === 'string' && executable.length > 0 && !executable.startsWith('--'), '--executable requires a browser executable path');
const eventExact = process.argv.includes('--event-exact');
const browserType = browserName === 'firefox' ? firefox : browserName === 'webkit' ? webkit : chromium;
let launch = {};
if (executable !== undefined) launch = {executablePath: executable};
else if (browserName === 'chrome') launch = {channel: 'chrome'};
else if (browserName === 'edge') launch = {channel: 'msedge'};
const browser = await browserType.launch({headless: true, ...launch});
const startedAt = new Date().toISOString(), results = [];
try {
  const page = await browser.newPage({viewport: {width: 500, height: 400}, deviceScaleFactor: 1});
  page.setDefaultTimeout(45000);
  await page.setContent('<!doctype html><html><head><style>html,body{margin:0}svg{display:block;position:absolute;left:0;top:0}</style></head><body><svg id="oracle" width="400" height="300" viewBox="0 0 400 300"></svg></body></html>', {waitUntil: 'domcontentloaded'});
  await page.addScriptTag({content: bundleContent.toString('utf8')});
  const keyName = {Alt: 'Alt', Shift: 'Shift', Space: 'Space', Meta: 'Meta', Control: 'Control'};
  for (const c of cases) {
    for (const value of Object.values(keyName)) await page.keyboard.up(value);
    await page.mouse.up();
    await page.evaluate(c => {
      const clone = x => x == null ? null : JSON.parse(JSON.stringify(x));
      const rect = value => value == null ? null : [value[0][0], value[0][1], value[1][0], value[1][1]];
      const input = value => value == null ? null : c.axis === 'x' ? [value[0], value[2]] : c.axis === 'y' ? [value[1], value[3]] : [[value[0], value[1]], [value[2], value[3]]];
      const svg = d3.select('#oracle'); svg.selectAll('*').remove();
      const g = svg.append('g');
      const b = (c.axis === 'x' ? d3.brushX() : c.axis === 'y' ? d3.brushY() : d3.brush())
        .extent([[c.extent[0], c.extent[1]], [c.extent[2], c.extent[3]]]).handleSize(c.handleSize).keyModifiers(c.keys ?? true);
      let fired = false, armed = false;
      window.oracleEvents = [];
      b.on('start.oracle brush.oracle end.oracle', event => {
        window.oracleEvents.push({type: event.type, mode: event.mode ?? null, selection: rect(g.node().__brush.selection), publicSelection: clone(event.selection), source: event.sourceEvent?.type ?? null,
          sourcePoint: Number.isFinite(event.sourceEvent?.clientX) && Number.isFinite(event.sourceEvent?.clientY) ? [event.sourceEvent.clientX, event.sourceEvent.clientY] : null});
        if (armed && !fired && c.callback?.phase === event.type) { fired = true; g.call(b.move, input(c.callback.selection)); }
      });
      g.call(b);
      if (c.initial !== null) g.call(b.move, input(c.initial));
      armed = true;
      window.oracleEvents = [];
      window.oracle = {g, b, input, rect};
      window.oracleSnapshot = () => ({selection: rect(g.node().__brush.selection), publicSelection: clone(d3.brushSelection(g.node())),
        events: window.oracleEvents.splice(0), cursor: g.select('.overlay').attr('cursor'),
        handles: g.selectAll('.handle').nodes().map(n => ({handle: n.__data__.type, visible: getComputedStyle(n).display !== 'none',
          bounds: n.hasAttribute('x') ? ['x', 'y', 'width', 'height'].map(k => Number(n.getAttribute(k))) : null}))});
    }, c);
    const trace = [], held = new Set(), initialSnapshot = await page.evaluate(() => window.oracleSnapshot());
    const dispatchMouse = async (type, point, target) => page.evaluate(({type, point, target, modifiers}) => {
      const el = type === 'mousedown' ? window.oracle.g.select(target === 'overlay' || target === 'selection' ? `.${target}` : `.handle--${target}`).node() : window;
      el.dispatchEvent(new MouseEvent(type, {bubbles: true, view: window, clientX: point[0], clientY: point[1],
        buttons: type === 'mouseup' ? 0 : 1, altKey: modifiers.includes('Alt'), shiftKey: modifiers.includes('Shift'),
        metaKey: modifiers.includes('Meta'), ctrlKey: modifiers.includes('Control')}));
    }, {type, point, target, modifiers: [...held]});
    for (const step of c.steps) {
      let actualTarget = null;
      if (step.type === 'down') {
        for (const [flag, name] of [['alt', 'Alt'], ['shift', 'Shift'], ['meta', 'Meta'], ['ctrl', 'Control']])
          if (step.modifiers[flag]) { await page.keyboard.down(name); held.add(name); }
        const [x, y] = step.point;
        actualTarget = await page.evaluate(({x, y}) => document.elementFromPoint(x, y)?.__data__?.type ?? null, {x, y});
        if (c.dispatch === 'target' || eventExact) {
          if (c.dispatch !== 'target') assert.equal(actualTarget, step.target, `${c.id}: actual hit target differs from requested ${step.target}`);
          await dispatchMouse('mousedown', step.point, step.target);
        }
        else {
          assert.equal(actualTarget, step.target, `${c.id}: actual hit target differs from requested ${step.target}`);
          await page.mouse.move(x, y); await page.mouse.down();
        }
      } else if (step.type === 'move') {
        if (eventExact) await dispatchMouse('mousemove', step.point); else await page.mouse.move(...step.point);
      }
      else if (step.type === 'up') {
        if (eventExact) { await dispatchMouse('mousemove', step.point); await dispatchMouse('mouseup', step.point); }
        else { await page.mouse.move(...step.point); await page.mouse.up(); }
      } else if (step.type === 'key') {
        await page.keyboard[step.down ? 'down' : 'up'](keyName[step.key]);
        if (step.down) held.add(keyName[step.key]); else held.delete(keyName[step.key]);
      }
      else if (step.type === 'program') await page.evaluate(selection => {
        const {g, b, input} = window.oracle; g.call(b.move, input(selection));
      }, step.selection);
      else throw new Error(`Unknown action ${step.type}`);
      trace.push({input: step, actualTarget, ...await page.evaluate(() => window.oracleSnapshot())});
    }
    results.push({...c, initialSnapshot, trace});
  }
} finally { await browser.close(); }

const fixture = {source: {name: 'd3-brush', version: '3.0.0', url: 'https://github.com/d3/d3-brush/blob/v3.0.0/src/brush.js', sourceSha256, packages: sources},
  method: 'Unmodified official UMD packages; frozen from actual Chrome mouse/keyboard events. Degenerate zero-size targets explicitly use DOM MouseEvent dispatch because they have no hit area. Optional --event-exact sends mouse DOM events at explicit logical coordinates to distinguish numeric D3 behavior from native pointer quantization; it is not device acceptance.',
  profileDifferences: ['D3 emits start/brush/end for repeated programmatic move; the checked Rust controller may suppress unchanged updates.',
    'D3 programmatic move permits coordinates outside the configured extent and keeps zero-span rectangles; the checked Rust API rejects or explicitly constrains them.',
    'D3 emptiness tests both dimensions even for brushX/brushY; a zero inactive extent clears on mouseup, unlike the requested Rust single-axis contract.',
    'D3 programmatic mutation during a gesture may retain its original baseline; the Rust session contract retires that baseline.',
    ...(edgeCases ? ['Original D3 can escape its extent when Space follows an Alt-centered drag already clamped at both edges; the checked Rust contract preserves the current selection size and remains bounded.'] : [])],
  cases: results};
const json = JSON.stringify(fixture, null, 2) + '\n';
const csv = ['# Actual d3-brush 3.0.0 browser oracle. C case, S step, H actual handle geometry; coordinates are logical SVG units.',
  '# C|id|axis|extent|initial|handleSize|profile|keys. S|id|index|action|target|point|key|modifiers|programRect|selection|events. H|id|index|handle|visible|xywh'];
const list = value => value === null || value === undefined ? 'null' : value.join(',');
for (const c of results) {
  csv.push(['C', c.id, c.axis, list(c.extent), list(c.initial), c.handleSize, c.profile, Number(c.keys ?? true)].join('|'));
  c.trace.forEach((step, index) => {
    const i = step.input, flags = Object.entries(i.modifiers ?? {}).filter(([, v]) => v).map(([k]) => k).sort().join(',');
    const events = step.events.map(e => [e.type, e.mode ?? 'none', list(e.selection)].join(':')).join(';');
    csv.push(['S', c.id, index, i.type, i.target ?? '', list(i.point), i.type === 'key' ? `${i.key},${Number(i.down)}` : '', flags,
      list(i.selection), list(step.selection), events].join('|'));
    for (const h of step.handles) csv.push(['H', c.id, index, h.handle, Number(h.visible), list(h.bounds)].join('|'));
  });
}
const fixtureName = edgeCases ? 'editable-brush-d3-edge-cases' : 'editable-brush-d3';
const outputs = [[`tests/fixtures/${fixtureName}.json`, json], [`tests/fixtures/${fixtureName}.csv`, csv.join('\n') + '\n']];
for (const [path, content] of outputs) {
  if (process.argv.includes('--check')) {
    const expected = await readFile(join(root, path), 'utf8');
    if (expected !== content) {
      await writeFile(join(target, `editable-brush-reference-${browserName}-observed.${path.endsWith('.json') ? 'json' : 'csv'}`), content);
      const a = expected.split('\n'), b = content.split('\n');
      const line = a.findIndex((value, index) => value !== b[index]);
      throw new Error(`${path}: original D3 oracle changed at line ${line + 1}; frozen=${a[line]}; observed=${b[line]}`);
    }
  }
  else await writeFile(join(root, path), content);
}
const receipt = {passed: true, startedAt, finishedAt: new Date().toISOString(), engine: browserName,
  version: browser.version(), mouseInput: eventExact ? 'DOM MouseEvent at exact logical coordinates' : 'native Playwright mouse', sourceSha256, cases: results.length, steps: results.reduce((n, c) => n + c.trace.length, 0),
  events: results.reduce((n, c) => n + c.trace.reduce((m, s) => m + s.events.length, 0), 0),
  handles: results.reduce((n, c) => n + c.trace.reduce((m, s) => m + s.handles.length, 0), 0),
  outputSha256: Object.fromEntries(outputs.map(([path, content]) => [path, sha(content)]))};
await writeFile(join(target, `editable-brush-reference-${browserName}${eventExact ? '-event-exact' : ''}${edgeCases ? '-edge-cases' : ''}-receipt.json`), JSON.stringify(receipt, null, 2) + '\n');
console.log(JSON.stringify(receipt));
