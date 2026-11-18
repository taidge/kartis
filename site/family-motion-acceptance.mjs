// Compare individually triggered showcase springs against an independent instant mount.
import assert from 'node:assert/strict';
import {mkdir, writeFile} from 'node:fs/promises';
import {dirname, resolve} from 'node:path';
import {fileURLToPath} from 'node:url';
import {parseArgs} from 'node:util';
import {chromium, firefox} from 'playwright';
import {installFrameClock, serve} from './frame-test-helpers.mjs';

async function audit({slug, renderer, compareFrames = false}) {
  const api = window.kartisExplorer;
  const el = id => document.getElementById(id);
  const surface = () => document.querySelector('#chart-stage .kartis-surface > '+renderer);
  const motion = enabled => {
    if (el('animation-enabled').checked !== enabled) el('animation-enabled').click();
  };
  const navigate = name => api.select(name);
  let fullPaints = 0;
  const pump = count => {
    for (let i=0; i<count; i++) if (window.advanceChartFrame()) fullPaints++;
  };
  const settle = () => {
    pump(2);
    for (let i=0; i<240; i++) {
      const state = api.snapshot();
      if (state.errors.length) throw Error(state.errors.join('\n'));
      if (state.active && !state.pending && !state.animating) return state;
      pump(1);
    }
    throw Error('spring did not settle');
  };
  const pixels = async () => {
    let canvas = surface();
    if (renderer === 'svg') {
      const svg = canvas.cloneNode(true);
      const rect = canvas.getBoundingClientRect();
      canvas = document.createElement('canvas');
      canvas.width = Math.round(rect.width);
      canvas.height = Math.round(rect.height);
      svg.setAttribute('width',canvas.width);
      svg.setAttribute('height',canvas.height);
      const blob = new Blob([new XMLSerializer().serializeToString(svg)],{type:'image/svg+xml'});
      const url = URL.createObjectURL(blob);
      try {
        const image = new Image();
        image.src = url;
        await image.decode();
        canvas.getContext('2d').drawImage(image,0,0);
      } finally { URL.revokeObjectURL(url); }
    }
    return canvas.getContext('2d').getImageData(0,0,canvas.width,canvas.height).data;
  };
  const diff = (a,b) => {
    if (a.length !== b.length) throw Error('surface dimensions changed');
    const x = new Uint32Array(a.buffer,a.byteOffset,a.length/4);
    const y = new Uint32Array(b.buffer,b.byteOffset,b.length/4);
    let changed = 0;
    for (let i=0; i<x.length; i++) if (x[i] !== y[i]) changed++;
    return changed;
  };
  const read = async (action, trigger) => {
    let prior = action === 'entrance' ? null : await pixels();
    const oldSeed = api.snapshot().seed;
    const beforePaints = fullPaints;
    trigger();
    const mounted = surface().parentNode;
    prior ??= await pixels();
    const samples = [];
    let frame = 0;
    for (const checkpoint of [2,6,12,20]) {
      pump(checkpoint-frame);
      frame = checkpoint;
      if (surface().parentNode !== mounted) throw Error(action+' replaced the surface');
      samples.push({ink:await pixels(), active:api.snapshot().animating});
    }
    const end = settle();
    if (end.slug !== slug || end.renderer !== renderer || (action==='data' && end.seed !== oldSeed+1))
      throw Error(action+' failed to apply');
    const last = await pixels();
    const changes = samples.map(({ink},i) => diff(ink,i ? samples[i-1].ink : prior));
    const interior = samples.filter(({ink,active})=>active && diff(ink,prior)>0 && diff(ink,last)>0);
    if (changes.filter(n=>n>0).length < 3 || interior.length < 2)
      throw Error(action+' '+end.variant+' has insufficient distinct intermediate paintings '+JSON.stringify(changes));
    const target = api.targetSvg();
    motion(false);
    navigate(slug);
    const fresh = settle();
    if (surface().parentNode === mounted || fresh.seed !== end.seed || fresh.variant !== end.variant || target !== api.targetSvg())
      throw Error(action+' reference did not reproduce the selected inputs');
    const mismatch = diff(last,await pixels());
    if (mismatch) throw Error(action+' '+end.variant+' differs from direct render at '+mismatch+' pixels');
    motion(true);
    const result = {action, variant:end.variant, seed:end.seed, intermediateFrames:interior.length, changedPixels:changes, finalMismatchPixels:mismatch};
    if (compareFrames) {
      result.fullPaints = fullPaints - beforePaints;
      result.hashes = await Promise.all(samples.map(async ({ink}) => {
        const bytes = new Uint8Array(await crypto.subtle.digest('SHA-256',ink));
        return [...bytes].map(byte=>byte.toString(16).padStart(2,'0')).join('');
      }));
    }
    return result;
  };

  motion(false);
  navigate(slug === 'line' ? 'bar' : 'line');
  el('renderer-'+renderer).click();
  settle();
  motion(true);
  const results = [];
  results.push(await read('entrance',()=>navigate(slug)));
  const variants = [...el('variant-select').options].map(option=>option.value);
  for (const variant of [...variants.slice(1), variants[0]]) {
    results.push(await read('configuration',()=>{
      const select = el('variant-select');
      select.value = variant;
      select.dispatchEvent(new Event('change',{bubbles:true}));
    }));
  }
  results.push(await read('data',()=>el('new-data').click()));
  return results;
}

async function openAudit(browser,url,errors,deviceScaleFactor=1,full=false) {
  const page = await browser.newPage({viewport:{width:1280,height:900},deviceScaleFactor,reducedMotion:'no-preference'});
  page.on('pageerror', error=>errors.push(String(error)));
  await page.addInitScript(installFrameClock,full);
  await page.goto(new URL('#chart/bar',url).href);
  await page.waitForFunction(()=>window.kartisExplorer?.catalog?.length===35,null,{polling:40});
  return page;
}

async function checkFractionalEdge(browser,url,errors,dpr) {
  const [retained,reference] = await Promise.all([false,true].map(async full=>{
    const page = await openAudit(browser,url,errors,dpr,full);
    try {
      return await page.evaluate(audit,{slug:'treemap',renderer:'canvas',compareFrames:true});
    } finally { await page.close(); }
  }));
  assert(reference.every(row=>row.fullPaints>0),'reference must repaint the backing store for every spring');
  assert(retained.every(row=>row.fullPaints===0),'retained owner must exercise the normal paint path');
  const strip = rows=>rows.map(({fullPaints,...frame})=>frame);
  assert.deepEqual(strip(retained),strip(reference),'retained Canvas samples match full repaints at DPR '+dpr);
  return {browser:browser.browserType().name(),version:browser.version(),dpr,
    actions:reference.length,matchingFrames:reference.reduce((count,row)=>count+row.hashes.length,0)};
}

const {values} = parseArgs({options:{url:{type:'string'},out:{type:'string'}}});
const out = resolve(values.out ?? resolve(dirname(fileURLToPath(import.meta.url)),'verification/family-motion'));
const report = {passed:false, cases:[], errors:[]};
let expectedPresets = 0;
let browser, server;
try {
  if (!values.url) server = await serve();
  const url = values.url ?? 'http://127.0.0.1:'+server.address().port+'/';
  report.build = (await (await fetch(new URL('build.json',url))).json()).asset_revision;
  browser = await chromium.launch();
  report.browser = browser.version();
  // Independent storage and input seeds for the two renderers.
  await Promise.all(['svg','canvas'].map(async renderer=>{
    const page = await openAudit(browser,url,report.errors);
    const catalog = await page.evaluate(()=>window.kartisExplorer.catalog.map(({slug,variants})=>({slug,count:variants.length})));
    expectedPresets += catalog.reduce((sum,chart)=>sum+chart.count,0);
    for (const {slug} of catalog) {
      try {
        const rows = await page.evaluate(audit,{slug,renderer});
        report.cases.push(...rows.map(row=>({slug,renderer,...row})));
      } catch (e) {
        const message = slug+'/'+renderer+' '+e.message;
        report.errors.push(message);
        console.error(message.split('\n')[0]);
      }
    }
  }));
  assert.equal(report.cases.filter(row=>row.action==='entrance').length,70);
  assert.equal(report.cases.filter(row=>row.action==='data').length,70);
  assert.equal(report.cases.filter(row=>row.action==='configuration').length,expectedPresets);
  report.treemapEdges = [];
  const ff = await firefox.launch();
  try {
    for (const engine of [browser,ff]) {
      for (const dpr of [1,2]) report.treemapEdges.push(await checkFractionalEdge(engine,url,report.errors,dpr));
    }
  } finally { await ff.close(); }
  assert.deepEqual(report.errors,[]);
  report.passed = true;
} catch (e) {
  report.error = String(e.stack ?? e);
  process.exitCode = 1;
} finally {
  await browser?.close();
  if (server) await new Promise(done=>server.close(done));
  await mkdir(out,{recursive:true});
  await writeFile(resolve(out,'report.json'),JSON.stringify(report,null,2)+'\n');
  console.log(JSON.stringify({passed:report.passed,checked:report.cases.length,errors:report.errors.length,out}));
}
