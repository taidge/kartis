// Regression: an existing client may still cache the old unversioned runtime.
// Keep normal browser caching enabled; interception would invalidate this test.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {mkdir, readFile, writeFile} from 'node:fs/promises';
import {createServer} from 'node:http';
import {dirname, extname, resolve, sep} from 'node:path';
import {fileURLToPath} from 'node:url';
import {parseArgs} from 'node:util';
import {chromium} from 'playwright';

const here = dirname(fileURLToPath(import.meta.url));
const {values: flags} = parseArgs({options: {
  out: {type: 'string'}, dist: {type: 'string'}, help: {type: 'boolean'},
}});
if (flags.help) {
  console.log('Usage: node site/cache-acceptance.mjs [--dist site/dist] [--out site/verification/cache]');
  process.exit(0);
}
const dist = resolve(flags.dist ?? resolve(here, 'dist'));
const out = resolve(flags.out ?? resolve(here, 'verification/cache'));
await mkdir(out, {recursive: true});
const hash = data => createHash('sha256').update(data).digest('hex');
const oldAssets = {
  '/app.js': {type: 'text/javascript', body: '/* KARTIS_OLD_UNVERSIONED_APP */'},
  '/style.css': {type: 'text/css', body: '/* KARTIS_OLD_UNVERSIONED_STYLE */'},
  '/pkg/kartis_site.js': {type: 'text/javascript', body: 'export const marker = "KARTIS_OLD_UNVERSIONED_GLUE";'},
  '/pkg/kartis_site_bg.wasm': {type: 'application/wasm', body: 'KARTIS_OLD_UNVERSIONED_WASM_NOT_INSTANTIATED'},
};
const mime = {'.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css',
  '.wasm': 'application/wasm', '.json': 'application/json', '.svg': 'image/svg+xml',
  '.woff2': 'font/woff2', '.txt': 'text/plain'};
const report = {
  passed: false,
  scope: 'Same-context HTTP cache: old unversioned assets remain cached while a new 35-chart owner loads one content-addressed runtime',
  cacheInterceptionUsed: false,
  checks: [], errors: [], requests: [], responses: [], serverRequests: [],
};
let server, browser, context, page;
let phase = 'setup';
const serverCounts = new Map();
const oldCounts = () => Object.fromEntries(Object.keys(oldAssets).map(path => [path, serverCounts.get(path) ?? 0]));
const check = name => report.checks.push(name);

try {
  const manifestBytes = await readFile(resolve(dist, 'build.json'));
  const manifest = JSON.parse(manifestBytes);
  assert.match(manifest.asset_revision, /^[0-9a-f]{64}$/);
  const prefix = '/assets/' + manifest.asset_revision + '/';
  report.assetRevision = manifest.asset_revision;
  report.prefix = prefix;
  report.inputs = {
    scriptSHA256: hash(await readFile(fileURLToPath(import.meta.url))),
    buildManifestSHA256: hash(manifestBytes),
    indexSHA256: hash(await readFile(resolve(dist, 'index.html'))),
    sourceCommit: manifest.source_commit,
  };
  for (const path of Object.keys(oldAssets)) {
    const entry = prefix.slice(1) + path.slice(1);
    assert(manifest.assets[entry], 'versioned manifest must include ' + path);
    const bytes = await readFile(resolve(dist, entry));
    assert.equal(hash(bytes), manifest.assets[entry].sha256, 'actual asset hash: ' + entry);
    assert.equal(bytes.length, manifest.assets[entry].bytes, 'actual asset length: ' + entry);
  }
  check('new runtime files match the content-addressed build manifest');
  server = createServer(async (request, response) => {
    const path = new URL(request.url, 'http://local').pathname;
    serverCounts.set(path, (serverCounts.get(path) ?? 0) + 1);
    report.serverRequests.push({phase, path, method: request.method});
    if (oldAssets[path]) {
      const asset = oldAssets[path];
      response.writeHead(200, {'Content-Type': asset.type,
        'Cache-Control': 'public, max-age=14400, no-transform'}).end(asset.body);
      return;
    }
    if (path === '/warm') {
      response.writeHead(200, {'Content-Type': 'text/html', 'Cache-Control': 'no-store'})
        .end('<!doctype html><meta charset="utf-8"><link rel="icon" href="data:,"><title>Old runtime cache warmup</title>Cache warmup');
      return;
    }
    try {
      const relative = decodeURIComponent(path === '/' ? '/index.html' : path);
      const file = resolve(dist, '.' + relative);
      assert(file.startsWith(dist + sep), 'path must remain inside built site');
      const bytes = await readFile(file);
      response.writeHead(200, {'Content-Type': mime[extname(file)] ?? 'application/octet-stream',
        'Cache-Control': path.startsWith(prefix)
          ? 'public, max-age=31536000, immutable, no-transform' : 'no-store, no-transform'}).end(bytes);
    } catch {
      response.writeHead(404, {'Cache-Control': 'no-store'}).end('Not found');
    }
  });
  await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', resolve);
  });
  const origin = `http://127.0.0.1:${server.address().port}`;
  report.origin = origin;
  browser = await chromium.launch({headless: true});
  report.browser = browser.version();
  context = await browser.newContext({viewport: {width: 1280, height: 900}});
  page = await context.newPage();
  page.on('pageerror', error => report.errors.push(String(error)));
  page.on('console', message => {
    if (message.type() === 'error') report.errors.push(message.text());
  });
  page.on('request', request => report.requests.push({phase, url: request.url(), type: request.resourceType()}));
  page.on('response', response => report.responses.push({phase, url: response.url(),
    status: response.status(), cacheControl: response.headers()['cache-control'] ?? ''}));
  page.on('requestfailed', request => report.errors.push(`${request.url()}: ${request.failure()?.errorText}`));

  phase = 'warm-old-assets';
  await page.goto(origin + '/warm');
  // The exact same fetch operation is repeated before navigation. A missing
  // wire request is the negative control proving that the cache is effective.
  const fetchOld = () => page.evaluate(async paths => {
    const rows = [];
    for (const path of paths) {
      const response = await fetch(path);
      rows.push({path, status: response.status, body: await response.text(),
        cacheControl: response.headers.get('cache-control')});
    }
    return rows;
  }, Object.keys(oldAssets));
  report.warmFirst = await fetchOld();
  report.warmSecond = await fetchOld();
  for (const rows of [report.warmFirst, report.warmSecond]) {
    for (const row of rows) {
      assert.equal(row.status, 200);
      assert.equal(row.body, oldAssets[row.path].body);
      assert.match(row.cacheControl, /max-age=14400/);
    }
  }
  report.oldWireCountsBeforeNavigation = oldCounts();
  for (const count of Object.values(report.oldWireCountsBeforeNavigation)) assert.equal(count, 1,
    'repeated old-resource fetch must be a real HTTP-cache hit before navigation');
  check('all four OLD resources remain cached after two normal fetches, one wire request each');

  phase = 'new-site-navigation';
  await page.goto(origin + '/');
  await page.waitForFunction(() => document.documentElement.dataset.ready === 'true'
    && window.kartisExplorer?.catalog?.length === 35
    && window.kartisExplorer.snapshot().active, null, {timeout: 45000});
  report.ready = await page.evaluate(() => ({
    catalogCount: window.kartisExplorer.catalog.length,
    catalog: window.kartisExplorer.catalog.map(row => row.slug),
    snapshot: window.kartisExplorer.snapshot(),
    ownerCount: document.querySelectorAll('#chart-stage .kartis-surface').length,
    actualRendererCount: document.querySelectorAll('#chart-stage .kartis-surface > svg, #chart-stage .kartis-surface > canvas').length,
    entries: [...document.querySelectorAll('script[src],link[rel=stylesheet][href]')].map(node => node.src || node.href),
  }));
  assert.equal(report.ready.catalogCount, 35);
  assert.equal(new Set(report.ready.catalog).size, 35);
  assert.equal(report.ready.ownerCount, 1);
  assert.equal(report.ready.actualRendererCount, 1);
  assert.equal(report.ready.snapshot.active, true);
  assert.deepEqual(report.ready.snapshot.errors, []);
  assert(report.ready.entries.length >= 2);
  assert(report.ready.entries.every(value => new URL(value).origin === origin
    && new URL(value).pathname.startsWith(prefix)), 'HTML entry points use the new runtime revision');
  report.runtimeRequests = report.requests.filter(row => row.phase === 'new-site-navigation'
    && (/\.(?:js|css|wasm)$/.test(new URL(row.url).pathname) || ['script', 'stylesheet'].includes(row.type)));
  assert(report.runtimeRequests.length >= 4, 'observe actual JavaScript, stylesheet and WASM requests');
  for (const row of report.runtimeRequests) assert(new URL(row.url).origin === origin
    && new URL(row.url).pathname.startsWith(prefix), 'new runtime may not request an old or mixed-revision asset: ' + row.url);
  for (const path of Object.keys(oldAssets)) assert(report.runtimeRequests.some(row =>
    new URL(row.url).pathname === prefix + path.slice(1)), 'actual versioned runtime request: ' + path);
  report.oldWireCountsAfterNavigation = oldCounts();
  assert.deepEqual(report.oldWireCountsAfterNavigation, report.oldWireCountsBeforeNavigation,
    'new page must never fall back to an old unversioned asset');
  check('35-chart catalog and exactly one real owner load exclusively from the new JS/CSS/WASM revision');

  phase = 'old-cache-negative-control';
  report.oldAfterNavigation = await fetchOld();
  for (const row of report.oldAfterNavigation) {
    assert.equal(row.status, 200);
    assert.equal(row.body, oldAssets[row.path].body, 'the old entry remains cached after the new application loads');
  }
  report.oldWireCountsAfterNegativeControl = oldCounts();
  assert.deepEqual(report.oldWireCountsAfterNegativeControl, report.oldWireCountsBeforeNavigation,
    'OLD responses after navigation must be served by the still-effective browser cache');
  check('OLD entries are still HTTP-cache hits after navigation, with no additional server requests');
  assert.deepEqual(report.errors, []);
  report.passed = true;
} catch (error) {
  report.failure = String(error?.stack ?? error);
  if (page && !page.isClosed()) {
    try { await page.screenshot({path: resolve(out, 'failure.png')}); } catch { /* Report the original error. */ }
  }
  process.exitCode = 1;
} finally {
  await context?.close();
  await browser?.close();
  if (server) await new Promise(resolve => server.close(resolve));
  report.serverCounts = Object.fromEntries(serverCounts);
  await writeFile(resolve(out, 'report.json'), JSON.stringify(report, null, 2) + '\n');
  console.log(JSON.stringify({passed: report.passed, checks: report.checks,
    oldWireCounts: report.oldWireCountsAfterNegativeControl ?? oldCounts(),
    out, ...(report.failure ? {failure: report.failure} : {})}, null, 2));
}
