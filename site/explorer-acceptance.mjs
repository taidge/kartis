// Independent public-explorer checks: actual controls, renderers and Rust springs.
import assert from 'node:assert/strict';
import {createServer} from 'node:http';
import {createHash} from 'node:crypto';
import {readFile, mkdir, writeFile} from 'node:fs/promises';
import {dirname, resolve, extname, sep} from 'node:path';
import {fileURLToPath} from 'node:url';
import {chromium} from 'playwright';

const here = dirname(fileURLToPath(import.meta.url));
const flags = Object.fromEntries(process.argv.slice(2).map(arg => {
  const split = arg.indexOf('=');
  assert(split > 2, 'Use --name=value arguments');
  return [arg.slice(2, split), arg.slice(split + 1)];
}));
const out = resolve(flags.out ?? resolve(here, 'verification/explorer'));
await mkdir(out, {recursive:true});
const report = {passed:false, checks:[], charts:[], motions:[], errors:[], networkErrors:[], analytics:[], runtimeAssets:[]};
const sha = text => createHash('sha256').update(text).digest('hex');
let server, browser, page;
const checked = name => report.checks.push(name);
const snapshot = () => page.evaluate(() => {
  const value = window.kartisExplorer.snapshot();
  return typeof value === 'string' ? JSON.parse(value) : value;
});
const settle = async () => {
  await page.waitForFunction(() => {
    const raw = window.kartisExplorer?.snapshot();
    const value = typeof raw === 'string' ? JSON.parse(raw) : raw;
    return value?.active && !value.animating && value.pending === 0;
  }, null, {timeout:25000});
  await page.evaluate(() => new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r))));
  const state = await snapshot();
  assert.deepEqual(state.errors, [], 'Rust browser owner errors');
  return state;
};
const geometry = () => page.locator('#chart-stage .kartis-surface > svg').evaluate(svg =>
  [...svg.querySelectorAll('path,rect,circle,ellipse,line,polyline,polygon')].map(node =>
    [node.tagName, node.getAttribute('data-id'), ...['d','x','y','cx','cy','r','rx','ry','width','height','points','x1','x2','y1','y2','transform'].map(key => node.getAttribute(key))]));
const paint = () => page.locator('#chart-stage .kartis-surface > canvas').evaluate(async canvas => {
  const pixels = canvas.getContext('2d').getImageData(0,0,canvas.width,canvas.height).data;
  const baseline = [...pixels.slice(0,4)];
  let changed = 0;
  for (let i=0; i<pixels.length; i+=4) if ([0,1,2,3].some(k => pixels[i+k] !== baseline[k])) changed++;
  const digest = await crypto.subtle.digest('SHA-256', pixels);
  return {width:canvas.width,height:canvas.height,changed,sha256:[...new Uint8Array(digest)].map(v=>v.toString(16).padStart(2,'0')).join('')};
});
const select = async slug => {
  // Exercise the visible navigation rather than a diagnostic mount function.
  await page.locator(`#chart-nav [data-slug="${slug}"]`).click();
  await page.waitForFunction(slug => {
    const raw = window.kartisExplorer.snapshot();
    return (typeof raw === 'string' ? JSON.parse(raw) : raw).slug === slug;
  }, slug);
  return settle();
};
try {
  let url = flags.url;
  if (!url) {
    const dist = resolve(here, 'dist');
    const types = {'.html':'text/html','.js':'text/javascript','.css':'text/css','.json':'application/json','.svg':'image/svg+xml','.wasm':'application/wasm','.txt':'text/plain'};
    server = createServer(async (request,response) => {
      try {
        const pathname = decodeURIComponent(new URL(request.url,'http://local').pathname);
        const file = resolve(dist,'.'+(pathname==='/'?'/index.html':pathname));
        if (!file.startsWith(dist+sep)) { response.writeHead(403).end(); return; }
        response.writeHead(200,{'Content-Type':types[extname(file)]??'application/octet-stream','Cache-Control':'public, max-age=0, must-revalidate, no-transform'}).end(await readFile(file));
      } catch { response.writeHead(404).end(); }
    });
    await new Promise(r=>server.listen(0,'127.0.0.1',r));
    url = `http://127.0.0.1:${server.address().port}`;
  }
  report.url = url;
  browser = await chromium.launch({headless:true});
  report.browser = browser.version();
  page = await browser.newPage({viewport:{width:1440,height:1050},acceptDownloads:true,reducedMotion:'reduce'});
  const headers = {};
  page.on('pageerror',error=>report.errors.push(String(error)));
  page.on('console',message=>{if(message.type()==='error') report.errors.push(message.text());});
  page.on('request',request=>{
    if(/cloudflareinsights|\/cdn-cgi\/(rum|beacon)/i.test(request.url())) report.analytics.push(request.url());
    if(/\.(js|css|wasm)$/.test(new URL(request.url()).pathname)) report.runtimeAssets.push(request.url());
  });
  page.on('requestfailed',request=>report.networkErrors.push({url:request.url(),error:request.failure()?.errorText}));
  page.on('response',response=>{
    if(response.status()>=400) report.networkErrors.push({url:response.url(),status:response.status()});
    if(['document','script','stylesheet','fetch'].includes(response.request().resourceType())) headers[response.url()]=response.headers()['cache-control']??'';
  });
  await page.goto(url);
  await page.waitForFunction(()=>window.kartisExplorer?.catalog?.length>=35);
  const catalog = await page.evaluate(()=>window.kartisExplorer.catalog);
  const buildResponse=await page.request.get(new URL('build.json',url).href);
  assert(buildResponse.ok());
  const buildManifest=await buildResponse.json();
  assert.match(buildManifest.asset_revision,/^[a-f0-9]{64}$/,'content-derived runtime asset revision');
  report.assetRevision=buildManifest.asset_revision;
  for(const asset of report.runtimeAssets) assert(new URL(asset).pathname.startsWith(`/assets/${report.assetRevision}/`),`versioned runtime asset: ${asset}`);
  checked('content-versioned HTML entry points and complete JS/CSS/WASM module graph');
  assert.equal(catalog.length,35);
  assert.equal(new Set(catalog.map(chart=>chart.slug)).size,35);
  const required = ['line','area','bar','scatter','radar','stream','bump','area-bump','marimekko','parallel-coordinates','pie','radial-bar','polar-bar','waffle','funnel','tree','treemap','sunburst','circle-packing','icicle','chord','sankey','network','choropleth','geo-map','heatmap','boxplot','bullet','calendar','time-range','swarm','voronoi','delaunay','contour','density'];
  assert.deepEqual(catalog.map(chart=>chart.slug).sort(),required.sort());
  assert.equal(await page.locator('#catalog-grid .catalog-card').count(),35);
  for(const slug of required) assert(await page.locator(`#catalog-grid .catalog-card[data-slug="${slug}"] svg`).count()>0,`${slug} real SVG thumbnail`);
  assert.equal(await page.locator('script[src*="cloudflareinsights"],script[data-cf-beacon]').count(),0);
  assert.equal(await page.locator('#auto-demo').getAttribute('aria-pressed'),'false','OS reduced motion disables automatic demo');
  checked('35 distinct chart families, Rust SVG thumbnails and reduced-motion initial state');
  await page.locator('#search').fill('sankey');
  assert.equal(await page.locator('#catalog-grid .catalog-card:visible').count(),1);
  await page.locator('#search').fill('');
  await page.locator('#catalog-grid [data-slug="line"]').click();
  await settle();
  checked('search and catalogue-card navigation');
  await select('bar');
  await page.goBack();
  await page.waitForFunction(()=>window.kartisExplorer.snapshot().slug==='line');
  await settle();
  checked('browser Back restores the previously selected chart');

  // Every named family must have real SVG geometry, authored data and painted Canvas.
  for(const chart of catalog) {
    await select(chart.slug);
    await page.locator('#renderer-svg').click();
    await settle();
    assert.equal(await page.locator('#chart-stage .kartis-surface > svg').count(),1);
    assert.equal(await page.locator('#chart-stage .kartis-surface').count(),1,'one live owner');
    const before = await geometry();
    const inkBefore = await page.locator('#chart-stage .kartis-surface > svg').evaluate(node=>node.outerHTML);
    assert(before.length>0,`${chart.slug} SVG geometry`);
    await page.locator('[data-tab="data"]').click();
    const dataBefore = await page.locator('#data-output').textContent();
    assert.doesNotThrow(()=>JSON.parse(dataBefore),`${chart.slug} input JSON`);
    await page.locator('[data-tab="code"]').click();
    assert((await page.locator('#code-output').textContent()).trim().length>10,`${chart.slug} core API call`);
    await page.locator('[data-tab="chart"]').click();
    await page.locator('#new-data').click();
    const state = await settle();
    const after = await geometry();
    const inkAfter = await page.locator('#chart-stage .kartis-surface > svg').evaluate(node=>node.outerHTML);
    assert.notEqual(inkBefore,inkAfter,`${chart.slug} actual SVG ink update`);
    const input = await page.evaluate(()=>window.kartisExplorer.input());
    const dataAfter = typeof input==='string'?JSON.parse(input):input;
    assert.notDeepEqual(JSON.parse(dataBefore),dataAfter,`${chart.slug} real input update`);
    assert.notEqual(await page.evaluate(()=>window.kartisExplorer.targetSvg()),'',`${chart.slug} exportable author scene`);
    await page.locator('#renderer-canvas').click();
    await settle();
    const canvas = await paint();
    assert(canvas.changed>20,`${chart.slug} actual Canvas paint`);
    assert.equal(await page.locator('#chart-stage .kartis-surface').count(),1);
    report.charts.push({slug:chart.slug,marks:state.marks,svgBefore:sha(JSON.stringify(before)),svgAfter:sha(JSON.stringify(after)),inputChanged:true,canvas});
    console.log(JSON.stringify({chart:chart.slug,renderers:['svg','canvas'],passed:true}));
  }
  checked('all 35 families: actual SVG, authored input updates, core API panel and painted Canvas');

  // Controls must change real layout geometry, rather than presentation text.
  for(const slug of ['line','bar','pie','tree']) {
    await select(slug); await page.locator('#renderer-svg').click(); await settle();
    const variants = await page.locator('#variant-select option').evaluateAll(nodes=>nodes.map(node=>node.value));
    assert(variants.length>=2,`${slug} configurable layout`);
    const outcomes=[];
    for(const variant of variants) {
      await page.locator('#variant-select').selectOption(variant); await settle();
      outcomes.push({variant,sha256:sha(JSON.stringify(await geometry()))});
    }
    assert.equal(new Set(outcomes.map(row=>row.sha256)).size,outcomes.length,`${slug} actual geometry for every variant`);
    (report.variants??=[]).push({slug,outcomes});
  }
  checked('Line, Bar, Pie and Tree options alter actual geometry');

  await select('line');
  await page.locator('#renderer-svg').click(); await settle();
  const download = page.waitForEvent('download'); await page.locator('#download-svg').click();
  const downloaded = await download;
  const downloadPath=resolve(out,'download.svg'); await downloaded.saveAs(downloadPath);
  assert((await readFile(downloadPath,'utf8')).includes('<svg'));
  checked('actual SVG download');

  // Restore normal OS motion, then observe actual Rust spring frames in both backends.
  await page.emulateMedia({reducedMotion:'no-preference'});
  await page.locator('#animation-enabled').check();
  for(const renderer of ['svg','canvas']) for(const slug of ['line','bar','pie','tree','funnel','circle-packing']) {
    await select(slug); await page.locator(`#renderer-${renderer}`).click(); await settle();
    await page.locator('#spring-preset').selectOption('gentle'); await settle();
    const start = renderer==='svg'?sha(JSON.stringify(await geometry())):(await paint()).sha256;
    await page.locator('#new-data').click();
    await page.waitForFunction(()=>window.kartisExplorer.snapshot().animating,{},{timeout:3000});
    const frames=[];
    for(let i=0;i<6;i++) {
      await page.evaluate(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))));
      frames.push(renderer==='svg'?sha(JSON.stringify(await geometry())):(await paint()).sha256);
    }
    const endState=await settle();
    const end=renderer==='svg'?sha(JSON.stringify(await geometry())):(await paint()).sha256;
    assert.notEqual(start,end,`${slug}/${renderer} changed final geometry or paint`);
    assert(new Set([start,...frames,end]).size>=3,`${slug}/${renderer} actual intermediate spring frames`);
    assert(!endState.animating);
    report.motions.push({slug,renderer,start,frames,end,settled:true});
  }
  checked('12 real Rust spring transitions: intermediate SVG geometry / Canvas pixels and final settlement');

  await select('bar'); await page.locator('#renderer-svg').click(); await settle();
  const retargetSeed=(await snapshot()).seed;
  await page.locator('#new-data').click();
  await page.waitForFunction(()=>window.kartisExplorer.snapshot().animating);
  await page.locator('#new-data').click();
  const retargeted=await settle();
  assert.equal(retargeted.seed,retargetSeed+2,'an active spring accepts a second real dataset');
  checked('retargeting a running spring preserves the newest authored dataset');

  await select('bar'); await page.locator('#renderer-svg').click(); await settle();
  await page.locator('#auto-demo').click();
  assert.equal(await page.locator('#auto-demo').getAttribute('aria-pressed'),'true');
  const seed=(await snapshot()).seed;
  await page.waitForFunction(seed=>window.kartisExplorer.snapshot().seed!==seed,seed,{timeout:15000});
  await page.locator('#auto-demo').click(); await settle();
  const paused=(await snapshot()).seed;
  await page.waitForTimeout(5000);
  assert.equal((await snapshot()).seed,paused,'paused demo leaves authored dataset unchanged');
  await page.locator('#animation-enabled').uncheck();
  await page.locator('#new-data').click(); await settle();
  assert(!(await snapshot()).animating);
  checked('automatic data demo, pause and animation-off controls');

  await page.locator('#theme-toggle').click(); await settle();
  assert.equal((await snapshot()).theme,'dark');
  await page.screenshot({path:resolve(out,'desktop-dark.png'),fullPage:true});
  await page.locator('#theme-toggle').click(); await settle();
  await page.setViewportSize({width:390,height:844});
  await page.waitForFunction(()=>document.documentElement.scrollWidth<=innerWidth);
  await page.waitForFunction(()=>window.kartisExplorer.snapshot().width<=390);
  await settle();
  const mobile=await snapshot(); assert(mobile.width<=390&&mobile.width>=220);
  await page.locator('#menu-toggle').click();
  await page.waitForFunction(()=>document.querySelector('#sidebar').classList.contains('open'));
  await select('pie');
  await page.waitForFunction(()=>document.querySelector('#sidebar').getBoundingClientRect().right<=0);
  assert.equal(await page.locator('#menu-toggle').getAttribute('aria-expanded'),'false');
  assert.equal(await page.locator('#chart-title').textContent(),'Pie');
  await page.screenshot({path:resolve(out,'mobile.png'),fullPage:true});
  await page.setViewportSize({width:1440,height:1050}); await settle();
  await page.locator('#back-to-gallery').click();
  await page.evaluate(()=>scrollTo(0,0));
  await page.screenshot({path:resolve(out,'home-viewport.png')});
  await page.screenshot({path:resolve(out,'catalogue.png'),fullPage:true});
  checked('dark theme, responsive mobile chart relayout and catalogue return');
  assert.deepEqual(report.errors,[]); assert.deepEqual(report.networkErrors,[]); assert.deepEqual(report.analytics,[]);
  if(flags.url) for(const [asset,value] of Object.entries(headers)) if(new URL(asset).origin===new URL(url).origin) assert(value.includes('no-transform'),`no-transform for ${asset}`);
  report.assetHeaders=headers;
  checked('no browser errors, failed requests or injected analytics; remote asset policy');
  report.passed=true;
} catch(error) {
  report.error=String(error.stack??error);
  if(page) {
    try{report.lastState=await snapshot();await page.screenshot({path:resolve(out,'failure.png'),fullPage:true});}catch{}
  }
} finally {
  await browser?.close(); if(server) await new Promise(r=>server.close(r));
  await writeFile(resolve(out,'report.json'),JSON.stringify(report,null,2)+'\n');
}
console.log(JSON.stringify({passed:report.passed,charts:report.charts.length,motions:report.motions.length,checks:report.checks,error:report.error,report:resolve(out,'report.json')},null,2));
if(!report.passed) process.exitCode=1;
