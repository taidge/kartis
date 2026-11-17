// Product-site smoke test. Uses real owners/renderers, never acceptance fixtures.
import assert from 'node:assert/strict';
import {createServer} from 'node:http';
import {readFile, mkdir, writeFile} from 'node:fs/promises';
import {resolve, dirname, extname, sep} from 'node:path';
import {fileURLToPath} from 'node:url';
import {chromium} from 'playwright';

const here = dirname(fileURLToPath(import.meta.url));
const flags = Object.fromEntries(process.argv.slice(2).map(v => { const i = v.indexOf('='); assert(i > 2); return [v.slice(2, i), v.slice(i + 1)]; }));
const out = resolve(flags.out ?? resolve(here, 'verification'));
await mkdir(out, {recursive: true});
const report = {passed: false, scope: 'Static gallery live SVG/Canvas, dataset, resize, tooltip, download and error smoke', errors: [], networkErrors: [], checks: []};
let server, browser, page;
const check = name => report.checks.push(name);
try {
  let url = flags.url;
  if (!url) {
    const dist = resolve(here, 'dist');
    const mime = {'.html':'text/html', '.js':'text/javascript', '.css':'text/css', '.svg':'image/svg+xml', '.wasm':'application/wasm', '.json':'application/json', '.txt':'text/plain'};
    server = createServer(async (request, response) => {
      try {
        const path = decodeURIComponent(new URL(request.url, 'http://local').pathname);
        const file = resolve(dist, '.' + (path === '/' ? '/index.html' : path));
        if (!file.startsWith(dist + sep)) { response.writeHead(403).end(); return; }
        const data = await readFile(file);
        response.writeHead(200, {'Content-Type': mime[extname(file)] ?? 'application/octet-stream'}).end(data);
      } catch { response.writeHead(404).end('Not found'); }
    });
    await new Promise(r => server.listen(0, '127.0.0.1', r));
    url = `http://127.0.0.1:${server.address().port}`;
  }
  report.url = url;
  // Exercise normal browser caching; Playwright disables BFCache by default.
  report.launchOptions = {headless:true, ignoreDefaultArgs:['--disable-back-forward-cache']};
  browser = await chromium.launch(report.launchOptions);
  report.browser = browser.version();
  page = await browser.newPage({viewport: {width:1440, height:1000}, deviceScaleFactor:1, acceptDownloads:true});
  page.on('pageerror', e => report.errors.push(String(e)));
  page.on('console', m => { if (m.type() === 'error') report.errors.push(m.text()); });
  page.on('response', r => { if (r.status() >= 400) report.networkErrors.push({url:r.url(), status:r.status()}); });
  page.on('requestfailed', r => report.networkErrors.push({url:r.url(), error:r.failure()?.errorText}));
  await page.goto(url);
  await page.waitForFunction(() => document.documentElement.dataset.ready === 'true');
  await page.waitForFunction(() => document.querySelectorAll('.chart .kartis-surface > svg').length === 6);
  assert.equal(await page.locator('.chart-card').count(), 6);
  assert.equal(await page.locator('.chart .kartis-surface > svg').count(), 6);
  check('six actual SVG chart surfaces');
  await page.screenshot({path:resolve(out, 'desktop-svg.png'), fullPage:true});
  const geometry = () => page.locator('#chart-line .kartis-surface').evaluate(n => [...n.querySelectorAll('path[data-id]')].map(n => n.getAttribute('d')).join('|'));
  const before = await geometry(); assert(before.length > 50);
  await page.locator('#new-data').click();
  await page.waitForFunction(previous => [...document.querySelectorAll('#chart-line .kartis-surface path[data-id]')].map(n => n.getAttribute('d')).join('|') !== previous, before);
  assert.equal(await page.locator('html').getAttribute('data-dataset'), '2');
  check('new Rust dataset changes actual rendered geometry');
  const target = page.locator('#chart-bar .kartis-surface > svg path[data-series]:not([data-series=""])').first();
  await target.scrollIntoViewIfNeeded();
  const bounds = await target.boundingBox(); assert(bounds?.width > 1 && bounds.height > 1);
  await page.mouse.move(bounds.x + bounds.width / 2, bounds.y + bounds.height / 2);
  await page.waitForFunction(() => { const n = document.querySelector('#chart-bar [role=tooltip]'); return n && getComputedStyle(n).display !== 'none' && n.textContent.trim().length > 0; });
  report.tooltip = await page.locator('#chart-bar [role=tooltip]').textContent();
  check('native hover produces real Kartis tooltip');
  const legend = page.locator('#chart-bar button').filter({hasText:'Hide Studio'});
  assert.equal(await legend.count(),1);
  await legend.click();
  await page.locator('#chart-bar button').filter({hasText:'Show Studio'}).waitFor();
  await page.locator('#chart-bar button').filter({hasText:'Show Studio'}).click();
  await page.locator('#chart-bar button').filter({hasText:'Hide Studio'}).waitFor();
  check('real Glory legend hides and restores its series');
  const downloadPromise = page.waitForEvent('download');
  await page.locator('[data-download=line]').click();
  const download = await downloadPromise;
  await download.saveAs(resolve(out, download.suggestedFilename()));
  const exported = await readFile(resolve(out, download.suggestedFilename()), 'utf8');
  assert(exported.includes('<svg') && exported.includes('<path') && exported.includes('Kartis line example'));
  check('actual SVG file download');
  await page.locator('[data-renderer=canvas]').click();
  await page.waitForFunction(() => document.querySelectorAll('.chart .kartis-surface canvas').length === 6);
  report.canvas = await page.locator('.chart .kartis-surface canvas').evaluateAll(nodes => nodes.map(n => {
    const data = n.getContext('2d').getImageData(0, 0, n.width, n.height).data;
    let colored = 0;
    for (let i=0;i<data.length;i+=4) if (data[i+3] > 0 && Math.min(data[i],data[i+1],data[i+2]) < 220) colored++;
    return {width:n.width,height:n.height,colored};
  }));
  assert(report.canvas.every(n => n.width > 200 && n.height > 200 && n.colored > 500));
  check('six actual painted Canvas surfaces');
  await page.locator('#new-data').click();
  assert.equal(await page.locator('html').getAttribute('data-dataset'), '3');
  await page.screenshot({path:resolve(out, 'desktop-canvas.png'), fullPage:true});
  await page.locator('[data-renderer=svg]').click();
  await page.waitForFunction(() => document.querySelectorAll('.chart .kartis-surface > svg').length === 6);
  await page.setViewportSize({width:390,height:844});
  await page.waitForFunction(() => { const n=document.querySelector('#chart-line .kartis-surface > svg'); return n && n.viewBox.baseVal.width < 380; });
  report.mobile = await page.evaluate(() => ({viewport:innerWidth, document:document.documentElement.scrollWidth, cards:[...document.querySelectorAll('.chart-card')].map(n=>n.getBoundingClientRect().width)}));
  assert(report.mobile.document <= report.mobile.viewport);
  assert(report.mobile.cards.every(w => w > 250 && w < 390));
  check('real responsive mobile layout and chart relayout, no horizontal overflow');
  await page.screenshot({path:resolve(out, 'mobile-svg.png'), fullPage:true});
  const settled = () => page.evaluate(() => new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r))));
  await settled();
  const retainedGeometry = await geometry();
  const retainedDataset = await page.locator('html').getAttribute('data-dataset');
  report.lifecycle = {handlerCycles:[], nativeRestores:[]};
  for (let cycle=0;cycle<2;cycle++) {
    // Deterministic handler coverage, explicitly separate from UA BFCache proof.
    const stopped = await page.evaluate(() => { dispatchEvent(new PageTransitionEvent('pagehide',{persisted:true})); return document.querySelectorAll('.chart .kartis-surface').length; });
    assert.equal(stopped,0,'every pagehide disposes all chart owners');
    await page.evaluate(() => dispatchEvent(new PageTransitionEvent('pageshow',{persisted:true})));
    await settled();
    assert.equal(await geometry(),retainedGeometry,'restoring owners retains the author dataset');
    assert.equal(await page.locator('html').getAttribute('data-dataset'),retainedDataset);
    report.lifecycle.handlerCycles.push({cycle,stoppedSurfaces:stopped,restored:true,dataset:retainedDataset});
  }
  check('two pagehide/pageshow handler cycles dispose owners and retain the Rust dataset');
  await page.evaluate(() => { window.__kartisSmokeRestores=[]; addEventListener('pageshow',e => window.__kartisSmokeRestores.push({trusted:e.isTrusted,persisted:e.persisted})); });
  for (let cycle=0;cycle<2;cycle++) {
    await page.goto(new URL('/robots.txt',url).href);
    await page.goBack();
    await page.waitForFunction(() => document.documentElement.dataset.ready === 'true' && document.querySelectorAll('.chart .kartis-surface > svg').length === 6);
    await settled();
    const actual = await page.evaluate(() => window.__kartisSmokeRestores?.at(-1) ?? null);
    report.lifecycle.nativeRestores.push({cycle,event:actual});
    if (!actual?.persisted) { report.lifecycle.nativeOutcome='not-retained-by-browser'; break; }
    assert(actual.trusted,'native BFCache restore must be a trusted UA event');
    assert.equal(await geometry(),retainedGeometry);
    assert.equal(await page.locator('html').getAttribute('data-dataset'),retainedDataset);
  }
  if (report.lifecycle.nativeRestores.length===2 && report.lifecycle.nativeRestores.every(r=>r.event?.trusted&&r.event.persisted)) check('two actual trusted BFCache back navigations preserve the dataset and redraw owners');
  assert.deepEqual(report.errors, []); assert.deepEqual(report.networkErrors, []);
  assert.equal(await page.locator('#error').isVisible(), false);
  check('no JavaScript, console or failed network errors');
  report.passed = true;
} catch (error) { report.error = error.stack ?? String(error); process.exitCode = 1; }
finally {
  if (page && !report.passed) {
    await page.screenshot({path:resolve(out,'failure.png'),fullPage:true}).catch(() => {});
    await writeFile(resolve(out,'failure.html'),await page.content()).catch(() => {});
    report.domCounts = await page.locator('.chart').evaluateAll(nodes => nodes.map(n => ({id:n.id,svgs:n.querySelectorAll('svg').length,surfaceSvgs:n.querySelectorAll('.kartis-surface svg').length,canvases:n.querySelectorAll('canvas').length}))).catch(() => []);
  }
  if (browser) await browser.close();
  if (server) await new Promise(r => server.close(r));
  await writeFile(resolve(out,'report.json'), JSON.stringify(report,null,2)+'\n');
  console.log(JSON.stringify(report,null,2));
}
