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
const report = {passed: false, scope: '35-type Rust explorer, live springs, original six-family SVG/Canvas, controls and lifecycle smoke', errors: [], networkErrors: [], analyticsRequests: [], checks: []};
const analyticsURL = value => /(?:^|\.)cloudflareinsights\.com$/i.test(new URL(value).hostname) || /^\/cdn-cgi\/(?:rum|beacon)(?:\/|$)/i.test(new URL(value).pathname);
const cacheResponses = [];
let server, browser, page;
const check = name => report.checks.push(name);
try {
  let url = flags.url;
  if (!url) {
    const dist = resolve(here, 'dist');
    const headers = await readFile(resolve(dist,'_headers'),'utf8');
    const declarations = [...headers.matchAll(/^\s*Cache-Control:\s*(.+)$/gmi)].map(m=>m[1]);
    assert(declarations.length > 0 && declarations.every(v=>/(?:^|,)\s*no-transform\s*(?:,|$)/i.test(v)),'all built Cache-Control rules must prohibit edge transformations');
    report.cachePolicy = {mode:'built-source',source:'site/dist/_headers',declarations};
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
  page.on('response', r => {
    if (r.status() >= 400) report.networkErrors.push({url:r.url(), status:r.status()});
    const resource = new URL(r.url());
    if (resource.origin === new URL(url).origin && (r.request().isNavigationRequest() || /\.(?:js|css|wasm)$/.test(resource.pathname))) cacheResponses.push({url:r.url(),status:r.status(),cacheControl:r.headers()['cache-control']??''});
  });
  page.on('request', r => { if (analyticsURL(r.url())) report.analyticsRequests.push({url:r.url(),type:r.resourceType()}); });
  page.on('requestfailed', r => report.networkErrors.push({url:r.url(), error:r.failure()?.errorText}));
  await page.goto(url);
  await page.waitForFunction(() => document.documentElement.dataset.ready === 'true');
  report.analyticsScripts = await page.locator('script').evaluateAll(nodes => nodes.map(n=>({src:n.src,beacon:n.hasAttribute('data-cf-beacon'),body:n.textContent})).filter(n=>n.beacon || /cloudflareinsights\.com|\/cdn-cgi\/(?:rum|beacon)/i.test(n.src+' '+n.body)));
  assert.deepEqual(report.analyticsScripts,[],'no Cloudflare analytics script may be injected into the built page');
  check('no Cloudflare analytics script injection');
  if (flags.url) {
    report.cachePolicy = {mode:'remote-http',responses:cacheResponses};
    for (const path of ['/app.js','/style.css','/pkg/kartis_site.js','/pkg/kartis_site_bg.wasm']) assert(cacheResponses.some(r=>new URL(r.url).pathname===path),'HTTP response observed for '+path);
    assert(cacheResponses.length>=5 && cacheResponses.every(r=>/(?:^|,)\s*no-transform\s*(?:,|$)/i.test(r.cacheControl)),'remote HTML, JS, CSS and WASM responses must all declare no-transform');
  }
  check(flags.url?'remote HTTP no-transform headers':'built no-transform header rules');

  const snapshot=()=>page.evaluate(()=>window.kartisExplorer.snapshot());
  const settled=async()=>{await page.waitForFunction(()=>{const s=window.kartisExplorer.snapshot();return s.active&&!s.animating&&s.pending===0;});await page.evaluate(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))));assert.deepEqual((await snapshot()).errors,[]);};
  const geometry=()=>page.locator('#chart-stage .kartis-surface > svg').evaluate(svg=>[...svg.querySelectorAll('path[data-id]')].map(p=>['d','fill','stroke','opacity','data-series'].map(k=>p.getAttribute(k)).join('~')).join('|'));
  const entries=await page.evaluate(()=>window.kartisExplorer.catalog);
  assert(entries.length>=35);assert.equal(await page.locator('.catalog-card').count(),entries.length);
  assert.equal(await page.locator('.catalog-card .thumbnail > svg').count(),entries.length);
  check('35 real Rust-rendered catalog thumbnails');
  assert.equal(await page.locator('#chart-stage .kartis-surface').count(),1);
  await page.waitForFunction(()=>window.kartisExplorer.snapshot().animating,null,{timeout:10000});
  const mid=await geometry();
  await page.evaluate(()=>window.kartisExplorer.pause());
  assert.equal((await snapshot()).animationKind,'Properties');
  await settled();assert.notEqual(await geometry(),mid,'visible live hero spring reaches a different settled geometry');
  check('first-screen automatic Rust property spring shows a real intermediate and settles');
  await page.screenshot({path:resolve(out,'desktop-catalog.png'),fullPage:true});
  report.charts=[];
  for(const slug of ['line','bar','pie','scatter','waffle','heatmap']) {
    await page.locator(`#chart-nav [data-slug="${slug}"]`).click();
    if(await page.locator('#animation-enabled').isChecked())await page.locator('#animation-enabled').uncheck();
    await settled();
    assert.equal((await snapshot()).slug,slug);assert.equal(await page.locator('#chart-stage .kartis-surface > svg').count(),1);
    const before=await geometry();await page.locator('#new-data').click();await settled();assert.notEqual(await geometry(),before,slug+' actual SVG geometry or paint updates');
    await page.locator('#renderer-canvas').click();await settled();
    const canvas=await page.locator('#chart-stage .kartis-surface > canvas').evaluate(n=>{const data=n.getContext('2d').getImageData(0,0,n.width,n.height).data;let colored=0;for(let i=0;i<data.length;i+=4)if(data[i+3]&&Math.min(data[i],data[i+1],data[i+2])<220)colored++;return{width:n.width,height:n.height,colored};});
    assert(canvas.width>200&&canvas.height>200&&canvas.colored>200);report.charts.push({slug,canvas});
    await page.locator('#renderer-svg').click();await settled();
  }
  check('original six chart families: actual SVG dataset updates and painted Canvas');
  await page.locator('#chart-nav [data-slug="bar"]').click();await settled();
  const target=page.locator('#chart-stage .kartis-surface > svg path[data-series]:not([data-series=""])').first();
  await target.scrollIntoViewIfNeeded();const bounds=await target.boundingBox();assert(bounds?.width>1&&bounds.height>1);
  await page.mouse.move(bounds.x+bounds.width/2,bounds.y+bounds.height/2);
  await page.waitForFunction(()=>{const tip=document.querySelector('#chart-stage [role=tooltip]');return tip&&getComputedStyle(tip).display!=='none'&&tip.textContent.trim();});
  report.tooltip=await page.locator('#chart-stage [role=tooltip]').textContent();check('native tooltip');
  const legend=page.locator('#chart-stage button').filter({hasText:/^Hide /}).first();const label=await legend.textContent();
  await legend.click();await page.locator('#chart-stage button').filter({hasText:label.replace('Hide ','Show ')}).click();await settled();check('real Glory legend hide and restore');
  await page.locator('[data-tab=data]').click();assert(JSON.parse(await page.locator('#data-output').textContent()));
  await page.locator('[data-tab=code]').click();assert((await page.locator('#code-output').textContent()).includes('bar'));await page.locator('[data-tab=chart]').click();check('actual author data and core Rust call panels');
  const downloadPromise=page.waitForEvent('download');await page.locator('#download-svg').click();const download=await downloadPromise;await download.saveAs(resolve(out,download.suggestedFilename()));
  const exported=await readFile(resolve(out,download.suggestedFilename()),'utf8');assert(exported.includes('<svg')&&exported.includes('<path'));check('actual SVG download');
  await page.screenshot({path:resolve(out,'desktop-detail.png'),fullPage:true});
  await page.setViewportSize({width:390,height:844});await settled();
  report.mobile=await page.evaluate(()=>({viewport:innerWidth,document:document.documentElement.scrollWidth,stage:document.querySelector('#chart-stage').getBoundingClientRect().width}));
  assert(report.mobile.document<=report.mobile.viewport);assert(report.mobile.stage<390&&report.mobile.stage>250);check('mobile responsive chart relayout without overflow');
  await page.screenshot({path:resolve(out,'mobile-detail.png'),fullPage:true});
  const retainedGeometry=await geometry(),retainedInput=await page.evaluate(()=>window.kartisExplorer.input());report.lifecycle=[];
  for(let cycle=0;cycle<2;cycle++){
    const stopped=await page.evaluate(()=>{dispatchEvent(new PageTransitionEvent('pagehide',{persisted:true}));return{surfaces:document.querySelectorAll('#chart-stage .kartis-surface').length,active:window.kartisExplorer.snapshot().active};});
    assert.equal(stopped.surfaces,0);assert.equal(stopped.active,false);
    await page.evaluate(()=>dispatchEvent(new PageTransitionEvent('pageshow',{persisted:true})));await settled();assert.equal(await geometry(),retainedGeometry);assert.deepEqual(await page.evaluate(()=>window.kartisExplorer.input()),retainedInput);report.lifecycle.push({cycle,...stopped,restored:true});
  }
  check('repeated lifecycle cleanup retains the real author dataset');
  assert.deepEqual(report.errors, []); assert.deepEqual(report.networkErrors, []);
  assert.deepEqual(report.analyticsRequests,[],'no analytics beacon requests, including aborted requests');
  assert.equal(await page.locator('#error').isVisible(), false);
  check('no JavaScript, console or failed network errors');
  report.passed = true;
} catch (error) { report.error = error.stack ?? String(error); process.exitCode = 1; }
finally {
  if (page && !report.passed) {
    await page.screenshot({path:resolve(out,'failure.png'),fullPage:true}).catch(() => {});
    await writeFile(resolve(out,'failure.html'),await page.content()).catch(() => {});
    report.domCounts = await page.locator('#chart-stage').evaluateAll(nodes => nodes.map(n => ({id:n.id,svgs:n.querySelectorAll('svg').length,surfaceSvgs:n.querySelectorAll('.kartis-surface svg').length,canvases:n.querySelectorAll('canvas').length}))).catch(() => []);
  }
  if (browser) await browser.close();
  if (server) await new Promise(r => server.close(r));
  await writeFile(resolve(out,'report.json'), JSON.stringify(report,null,2)+'\n');
  console.log(JSON.stringify(report,null,2));
}
