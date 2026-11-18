// Run after building the site and installing Playwright's Chromium and Firefox.
import assert from 'node:assert/strict';
import {readFile, mkdir, writeFile} from 'node:fs/promises';
import {createServer} from 'node:http';
import {dirname, extname, resolve, sep} from 'node:path';
import {fileURLToPath} from 'node:url';
import {parseArgs} from 'node:util';
import {chromium, firefox} from 'playwright';

const site = dirname(fileURLToPath(import.meta.url));
const {values} = parseArgs({options:{url:{type:'string'}, out:{type:'string'}}});
const out = resolve(values.out ?? resolve(site, 'verification/partial-frames'));
const viewport = {width:1263, height:1000};

async function serve() {
  const dist = resolve(site, 'dist');
  const types = {'.html':'text/html', '.js':'text/javascript', '.css':'text/css', '.wasm':'application/wasm', '.json':'application/json', '.svg':'image/svg+xml'};
  const server = createServer(async (request, response) => {
    try {
      const path = decodeURIComponent(new URL(request.url, 'http://localhost').pathname);
      const file = resolve(dist, '.' + (path === '/' ? '/index.html' : path));
      if (!file.startsWith(dist + sep)) { response.writeHead(403).end(); return; }
      const content = await readFile(file);
      response.writeHead(200, {'Content-Type':types[extname(file)] ?? 'application/octet-stream'}).end(content);
    } catch { response.writeHead(404).end(); }
  });
  await new Promise((done, reject) => server.once('error',reject).listen(0,'127.0.0.1',done));
  return server;
}

// Both owners receive exactly the same RAF times. Invalidating the backing size
// before the reference paint forces the renderer to recreate the entire bitmap.
function installFrameClock(full) {
  const queue = new Map();
  let token = 0, time = 1000;
  window.requestAnimationFrame = fn => { queue.set(++token,fn); return token; };
  window.cancelAnimationFrame = id => queue.delete(id);
  window.advanceChartFrame = () => {
    const callbacks = [...queue.values()];
    queue.clear();
    time += 16;
    const state = window.kartisExplorer?.snapshot();
    const canvas = document.querySelector('#chart-stage .kartis-surface > canvas');
    const invalidate = full && canvas && callbacks.length && (state?.animating || state?.pending);
    if (invalidate) canvas.width++;
    for (const callback of callbacks) callback(time);
    return Boolean(invalidate);
  };
}

async function runPieFrames() {
  const api = window.kartisExplorer;
  const $ = selector => document.querySelector(selector);
  const plot = () => $('#chart-stage .kartis-surface > canvas');
  const pump = async count => {
    for (let i=0; i<count; i++) {
      window.advanceChartFrame();
      if (i%3 === 0) await new Promise(resolve => setTimeout(resolve,2));
    }
  };
  const readPixels = async () => {
    const c = plot(), bytes = c.getContext('2d').getImageData(0,0,c.width,c.height).data;
    const hash = await crypto.subtle.digest('SHA-256',bytes);
    return [...new Uint8Array(hash)].map(byte => byte.toString(16).padStart(2,'0')).join('');
  };
  const change = key => {
    const option = $('#chart-option-'+key);
    option.selectedIndex = 1;
    option.dispatchEvent(new Event('change',{bubbles:true}));
  };
  api.select('pie');
  $('#renderer-canvas').click();
  if (!$('#animation-enabled').checked) $('#animation-enabled').click();
  await pump(70);
  const canvas = plot(), samples = [await readPixels()], states = [];
  let invalidations = 0;
  // Leave a continuous tail without readbacks. Reading at every RAF forces a
  // raster flush in Chromium and can conceal stale pixels in the settled frame.
  const checkpoints = new Set([0,1,2,4,5,6,9,10,11,14,15,16,19,25,35,50,80,119]);
  for (let i=0; i<120; i++) {
    if (i===0 || i===15) $('#new-data').click();
    if (i===5) change('label-mode');
    if (i===10) change('palette');
    if (window.advanceChartFrame()) invalidations++;
    if (checkpoints.has(i)) {
      samples.push(await readPixels());
      const {animating,seed,pending,errors} = api.snapshot();
      states.push({frame:i, animating, seed, pending, errors, width:canvas.width, height:canvas.height});
    }
  }
  const target = api.targetSvg();
  $('#renderer-svg').click();
  await pump(5);
  $('#renderer-canvas').click();
  await pump(10);
  if (plot() === canvas || canvas.isConnected || api.targetSvg() !== target) throw Error('Pie reference mount did not reproduce the target');
  return {samples,states,invalidations,fresh:await readPixels()};
}

async function pie(url, errors) {
  const browser = await chromium.launch();
  try {
    const [retained, reference] = await Promise.all([false,true].map(async full => {
      const page = await browser.newPage({viewport,deviceScaleFactor:1.25,reducedMotion:'no-preference'});
      page.on('pageerror', e => errors.push(String(e)));
      await page.addInitScript(installFrameClock,full);
      await page.goto(new URL('#chart/heatmap',url).href);
      await page.waitForFunction(() => window.kartisExplorer?.catalog?.length===35, null,{polling:40});
      return page.evaluate(runPieFrames);
    }));
    assert(reference.invalidations > 15, 'reference reconstructed the backing store during the spring');
    assert.equal(retained.invalidations, 0);
    assert.deepEqual(retained.states, reference.states, 'frame times, dimensions and accepted inputs agree');
    assert(retained.states.every(s => !s.errors.length));
    assert(retained.states.some(s => s.animating) && !retained.states.at(-1).animating);
    assert.equal(retained.states.at(-1).seed,2);
    assert(new Set(retained.samples).size > 5, 'multiple distinct intermediate paintings');
    for (let i=0; i<reference.samples.length; i++) {
      const frame = i ? retained.states[i-1].frame : 'initial';
      assert.equal(retained.samples[i],reference.samples[i], 'Pie/DPR 1.25 exact RGBA at frame '+frame);
    }
    assert.equal(retained.samples.at(-1),retained.fresh,'settled Pie pixels match a new canvas');
    return {browser:browser.version(), dpr:1.25, frames:120, fullRebuilds:reference.invalidations,
      snapshots:retained.samples.length, distinct:new Set(retained.samples).size, settledMatchesFresh:true};
  } finally { await browser.close(); }
}

async function labels(url, errors) {
  const browser = await firefox.launch();
  try {
    const page = await browser.newPage({viewport,deviceScaleFactor:1,reducedMotion:'reduce'});
    page.on('pageerror',e => errors.push(String(e)));
    await page.goto(new URL('#chart/line',url).href);
    await page.waitForFunction(() => window.kartisExplorer?.catalog?.length===35);
    const results = [];
    for (const slug of ['line','heatmap']) {
      results.push(await page.evaluate(async slug => {
        const api = window.kartisExplorer, $ = selector => document.querySelector(selector);
        const surface = () => $('#chart-stage .kartis-surface > canvas');
        const settle = async () => {
          for (let i=0; i<100; i++) {
            await new Promise(requestAnimationFrame);
            const state = api.snapshot();
            if (state.errors.length) throw Error(state.errors.join('\n'));
            if (state.slug===slug && state.active && !state.pending && !state.animating) {
              await new Promise(requestAnimationFrame);
              return;
            }
          }
          throw Error(slug+' did not settle');
        };
        const pixels = c => c.getContext('2d').getImageData(0,0,c.width,c.height);
        api.select(slug);
        $('#renderer-canvas').click();
        await settle();
        const canvas = surface(), before = pixels(canvas);
        const option = $('#chart-option-label-mode');
        option.value='featured';
        option.dispatchEvent(new Event('change',{bubbles:true}));
        await settle();
        const after = pixels(canvas), target = api.targetSvg();
        if (surface()!==canvas || !after.data.some((v,i)=>v!==before.data[i])) throw Error(slug+' label change did not paint');
        $('#renderer-svg').click();
        await settle();
        $('#renderer-canvas').click();
        await settle();
        const reference = pixels(surface());
        if (canvas.isConnected || target!==api.targetSvg() || reference.width!==after.width || reference.height!==after.height) {
          throw Error(slug+' invalid fresh label reference');
        }
        let differentPixels = 0;
        for (let i=0; i<after.data.length; i+=4) {
          for (let k=0; k<4; k++) if (after.data[i+k]!==reference.data[i+k]) { differentPixels++; break; }
        }
        return {slug,differentPixels};
      },slug));
    }
    assert(results.every(row => row.differentPixels===0), 'Firefox featured text must match complete fresh RGBA: '+JSON.stringify(results));
    return {browser:browser.version(), dpr:1, results};
  } finally { await browser.close(); }
}

const report = {passed:false, errors:[]};
let server;
try {
  if (!values.url) server = await serve();
  const url = values.url ?? 'http://127.0.0.1:'+server.address().port+'/';
  report.build = (await (await fetch(new URL('build.json',url))).json()).asset_revision;
  report.pie = await pie(url, report.errors);
  report.labels = await labels(url, report.errors);
  assert.deepEqual(report.errors,[]);
  report.passed = true;
} catch (error) {
  report.error = String(error.stack ?? error);
  process.exitCode = 1;
} finally {
  if (server) await new Promise(done => server.close(done));
  await mkdir(out,{recursive:true});
  await writeFile(resolve(out,'report.json'),JSON.stringify(report,null,2)+'\n');
  console.log(JSON.stringify(report,null,2));
}
