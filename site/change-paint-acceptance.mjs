import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';

// The test observes the live renderer, independently of its bookkeeping.
function observeSvg(svg) {
  const selector = 'path[data-id],text';
  const key = node => node.localName === 'text'
    ? ['label',node.textContent,node.getAttribute('x'),node.getAttribute('y')].join(' ')
    : 'mark ' + node.getAttribute('data-id');
  const before = new Map([...svg.querySelectorAll(selector)].map(node =>
    [key(node), {node, html: node.outerHTML}]));
  const touched = new Set(), geometryIds = new Set();
  let mutations = 0, geometryWrites = 0;
  function collect(records) {
    for (const record of records) {
      mutations++;
      if (record.attributeName === 'd') {
        geometryWrites++;
        geometryIds.add(record.target.getAttribute('data-id'));
      }
      const target = record.target.nodeType === 1 ? record.target : record.target.parentElement;
      const owner = target?.closest(selector);
      if (owner) touched.add(owner);
      for (const child of [...record.addedNodes, ...record.removedNodes]) {
        if (child.nodeType !== 1) continue;
        if (child.matches(selector)) touched.add(child);
        for (const nested of child.querySelectorAll(selector)) touched.add(nested);
      }
    }
  }
  const observer = new MutationObserver(collect);
  observer.observe(svg, {subtree: true, childList: true, attributes: true, characterData: true});
  return {
    read() {
      collect(observer.takeRecords());
      const stableTouched = [];
      let unchanged = 0;
      for (const node of svg.querySelectorAll(selector)) {
        const original = before.get(key(node));
        if (!original || original.html !== node.outerHTML) continue;
        unchanged++;
        if (node !== original.node || touched.has(node)) stableTouched.push(key(node));
      }
      return {retained:svg.isConnected, mutations, geometryWrites, geometryIds:[...geometryIds], unchanged, stableTouched};
    },
    stop() { observer.disconnect(); },
  };
}

function observeCanvas(canvas) {
  const context = canvas.getContext('2d'), prototype = Object.getPrototypeOf(context);
  const counts = {}, scratch = {}, textDrawn = new Set(), clearRegions = new Set(), pathsDrawn = new Set();
  let fullClears = 0, backingChanges = 0;
  const resize = new MutationObserver(records => { backingChanges += records.length; });
  resize.observe(canvas, {attributes:true, attributeFilter:['width','height']});
  const restorations = [];
  function hook(name, log) {
    const descriptor = Object.getOwnPropertyDescriptor(prototype, name);
    if (typeof descriptor?.value !== 'function') return;
    const original = descriptor.value;
    Object.defineProperty(prototype, name, {...descriptor, value:function(...args) {
      log(this, args);
      return original.apply(this, args);
    }});
    restorations.push(() => Object.defineProperty(prototype, name, descriptor));
  }
  // Heatmap cells and Bullet ranges, measures and markers are straight-sided.
  // Their emitted paths identify individual painted data marks on both the
  // destination and any scratch surface used for overlap compositing.
  const paths = new WeakMap();
  hook('beginPath', ctx => paths.set(ctx, []));
  for (const name of ['moveTo','lineTo','rect']) {
    hook(name, (ctx, args) => paths.get(ctx)?.push(args.map(n => n.toFixed(2)).join(',')));
  }
  for (const name of ['reset','clearRect','fillRect','strokeRect','fill','stroke','fillText',
    'strokeText','drawImage','putImageData']) {
    hook(name, (ctx, args) => {
      const dest = ctx === context;
      const log = dest ? counts : scratch;
      log[name] = (log[name] ?? 0) + 1;
      if (dest && name === 'clearRect') {
        const [x,y,w,h] = args;
        const transform = ctx.getTransform();
        const a = transform.transformPoint({x,y});
        const b = transform.transformPoint({x:x+w,y:y+h});
        clearRegions.add([a.x,a.y,b.x-a.x,b.y-a.y].map(Math.round).join(','));
        if (a.x<=0 && a.y<=0 && b.x>=canvas.width && b.y>=canvas.height) fullClears++;
      } else if (name === 'fillText' || name === 'strokeText') {
        textDrawn.add(args[0]);
      } else if (name === 'fill' || name === 'stroke') {
        const path = paths.get(ctx);
        if (path?.length) pathsDrawn.add(name+' '+path.join(' '));
      }
    });
  }
  return {
    read() {
      backingChanges += resize.takeRecords().length;
      return {retained:canvas.isConnected, counts, scratch, backingChanges, fullClears,
        clearRegions:[...clearRegions], textDrawn:[...textDrawn], pathsDrawn:[...pathsDrawn]};
    },
    stop() { resize.disconnect(); restorations.forEach(restore => restore()); },
  };
}

const hash = bytes => createHash('sha256').update(bytes).digest('hex');
async function ink(base, renderer) {
  if (renderer === 'svg') {
    // Capture the same plot pixels after visiting settings below the chart.
    // A just-visible SVG can otherwise sit underneath the fixed site header.
    await base.evaluate(svg => svg.scrollIntoView({block:'center', behavior:'instant'}));
    return hash(await base.screenshot({animations:'allow'}));
  }
  return base.evaluate(async canvas => {
    const pixels = canvas.getContext('2d').getImageData(0, 0, canvas.width, canvas.height).data;
    const digest = await crypto.subtle.digest('SHA-256', pixels);
    return [...new Uint8Array(digest)].map(b => b.toString(16).padStart(2,'0')).join('');
  });
}

export async function checkChangePainting(explorerPage) {
  const page = await explorerPage.context().browser().newPage({
    viewport:explorerPage.viewportSize(),
    deviceScaleFactor:await explorerPage.evaluate(() => devicePixelRatio),
  });
  const url = new URL('#chart/heatmap', explorerPage.url()).href;
  const rows = [];
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  page.on('console', message => { if (message.type()==='error') errors.push(message.text()); });
  page.on('requestfailed', request => errors.push(request.url()+' '+request.failure()?.errorText));
  page.on('response', response => { if (response.status()>=400) errors.push(response.status()+' '+response.url()); });
  const settle = async () => {
    await page.waitForFunction(() => {
      const state = window.kartisExplorer?.snapshot();
      return state?.active && !state.animating && state.pending===0;
    },null,{timeout:25000});
    await page.evaluate(() => new Promise(done => requestAnimationFrame(() => requestAnimationFrame(done))));
    const state = await page.evaluate(() => window.kartisExplorer.snapshot());
    assert.deepEqual(state.errors,[]);
    return state;
  };
  const select = async slug => {
    await page.locator('#chart-nav [data-slug="'+slug+'"]').click();
    return settle();
  };
  const animation = page.locator('#animation-enabled');
  const option = (key, value) => page.locator('#chart-option-' + key).selectOption(value);
  const defaultLayout = async () => {
    for (const picker of await page.locator('#chart-controls select, #variant-select').all()) {
      await picker.selectOption({index:0});
    }
    await settle();
  };
  try {
    await page.emulateMedia({reducedMotion:'no-preference'});
    for (const renderer of ['svg','canvas']) {
      // Seeds belong to the chart's WASM owner. Start both renderers with
      // the same real datasets, independently of the surrounding explorer run.
      if (page.url() !== 'about:blank') await page.goto('about:blank');
      await page.goto(url);
      await settle();
      const rendererButton = page.locator('#renderer-' + renderer);
      await rendererButton.click();
      await animation.uncheck();
      await settle();
      const base = page.locator('#chart-stage .kartis-surface > ' + renderer);
      const run = async (operation, change, kind) => {
        const animated = kind === 'motion' || kind === 'axis-motion';
        const initial = await ink(base, renderer);
        const probe = await base.evaluateHandle(renderer === 'svg' ? observeSvg : observeCanvas);
        const start = await page.evaluate(() => window.kartisExplorer.snapshot());
        let intermediate, changes, final, end;
        try {
          await change();
          if (animated) {
            await page.waitForFunction(() => window.kartisExplorer.snapshot().animating);
            await page.evaluate(() => new Promise(done => requestAnimationFrame(() => requestAnimationFrame(done))));
            intermediate = await ink(base, renderer);
          }
          end = await settle();
          final = await ink(base, renderer);
          changes = await probe.evaluate(p => p.read());
        } finally { await probe.evaluate(p => p.stop()); await probe.dispose(); }
        const caseName = renderer + '/' + operation;
        const row = {slug:start.slug, renderer, operation, fromSeed:start.seed, toSeed:end.seed, ...changes};
        assert(changes.retained, caseName + ' retains its base layer');
        if (animated) {
          assert.equal(end.seed, start.seed + (kind === 'motion' ? 1 : 0), caseName);
          assert.equal(new Set([initial,intermediate,final]).size, 3, caseName + ' intermediate frame');
        }
        if (kind === 'no-ink') assert.equal(final, initial, caseName);
        else assert.notEqual(final, initial, caseName + ' must visibly change');
        // Switching away and back gives the accepted target an empty backing store.
        if (animated) await animation.uncheck();
        await page.locator('#renderer-' + (renderer === 'svg' ? 'canvas' : 'svg')).click();
        await settle();
        await rendererButton.click();
        await settle();
        assert.equal(await ink(base,renderer), final, caseName + ' matches a full render pixel-for-pixel');
        if (renderer === 'svg') {
          assert(changes.unchanged > 0, caseName + ' has an unchanged control');
          assert.deepEqual(changes.stableTouched, [], caseName + ' untouched nodes retain identity and attributes');
          if (start.slug === 'heatmap') assert.equal(changes.geometryWrites, 0, caseName + ' retained cell paths');
          if (kind === 'sparse-data') {
            assert.equal(changes.geometryWrites, 3, caseName + ' only the three measures change geometry');
            assert.equal(changes.geometryIds.filter(id => id?.endsWith(':measure:0')).length,3,caseName);
          }
          if (kind === 'rescaled-data') {
            assert(changes.geometryIds.some(id => id?.startsWith('bullet-axis:')), caseName + ' rescaled ticks');
            assert(changes.geometryIds.some(id => id?.includes(':range:')), caseName + ' rescaled ranges');
          }
          if (kind === 'no-ink') assert.equal(changes.mutations, 0, caseName);
        } else if (kind === 'no-ink') {
          assert.deepEqual(changes.counts, {}, caseName + ' retained base pixels');
          assert.deepEqual(changes.scratch, {}, caseName + ' skipped offscreen painting');
          assert.equal(changes.backingChanges, 0, caseName);
        } else if (kind === 'local' || kind === 'sparse-data') {
          assert.equal(changes.counts.reset ?? 0, 0, caseName + ' retained live context');
          assert.equal(changes.backingChanges, 0, caseName + ' retained backing store');
          assert.equal(changes.fullClears, 0, caseName + ' confined clears');
          if (start.slug === 'heatmap') {
            const fills = (changes.counts.fill ?? 0) + (changes.scratch.fill ?? 0);
            assert(fills < 56, caseName + ' confines foreground and offscreen cell paints');
            const ticks = changes.textDrawn.filter(text => /^(\d\d:\d\d|Mon|Tue|Wed|Thu|Fri|Sat|Sun)$/.test(text));
            assert(ticks.length < 15, caseName + ' retained distant tick text');
          } else if (kind === 'sparse-data') {
            assert(changes.pathsDrawn.length > 0 && changes.pathsDrawn.length < 15,
              caseName + ' repaints only measures and overlapping marks on both surfaces');
          }
        } else if (kind === 'axis-motion') {
          assert.equal(changes.counts.reset ?? 0, 0, caseName);
          assert.equal(changes.fullClears, 0, caseName);
          assert.deepEqual(changes.textDrawn.filter(text => /^(Mon|Tue|Wed|Thu|Fri|Sat|Sun)$/.test(text)), [],
            caseName + ' retained weekday labels while the time ticks faded');
        } else if (kind === 'palette' || kind === 'motion') {
          assert.deepEqual(changes.textDrawn, [], caseName + ' preserves static weekday and hour labels');
        }
        rows.push({...row, matchesDirect:true});
      };
      await run('tooltip content', () => option('tooltip-content','value'), 'no-ink');
      await option('tooltip-content','detail');
      await settle();
      await run('add three labels', () => option('label-mode','featured'), 'local');
      await run('remove three labels', () => option('label-mode','default'), 'local');
      await run('remove X tick labels', () => option('axis-x','hide'), 'local');
      await run('restore X tick labels', () => option('axis-x','show'), 'local');
      await run('remove Y tick labels', () => option('axis-y','hide'), 'local');
      await option('axis-y','show');
      await settle();
      await run('change palette', () => option('palette','viridis'), 'palette');
      await option('palette','default');
      await settle();
      const theme = page.locator('#theme-toggle');
      await run('switch plot theme', () => theme.click(), 'global');
      await theme.click();
      await settle();
      await run('enable animation', () => animation.check(), 'no-ink');
      const spring = page.locator('#spring-preset');
      const nextSpring = await spring.inputValue() === 'gentle' ? 'default' : 'gentle';
      await run('change spring', () => spring.selectOption(nextSpring), 'no-ink');
      await run('animate X tick removal', () => option('axis-x','hide'), 'axis-motion');
      await option('axis-x','show');
      await settle();
      await animation.check();
      await settle();
      await run('animate new data', () => page.locator('#new-data').click(), 'motion');
      await select('bullet');
      await defaultLayout();
      const domain = () => page.evaluate(() => {
        const rows = window.kartisExplorer.input();
        const values = rows.flatMap(row => [...row.ranges, ...row.measures, ...row.markers]);
        return [Math.min(...values), Math.max(...values)];
      });
      assert.equal((await settle()).seed,0);
      assert.deepEqual(await domain(),[50,100]);
      await run('update measures', () => page.locator('#new-data').click(), 'sparse-data');
      assert.deepEqual(await domain(),[50,100]);
      for (let seed=2; seed<=9; seed++) {
        await page.locator('#new-data').click();
        assert.equal((await settle()).seed,seed);
      }
      assert.deepEqual(await domain(),[50,100]);
      await run('update measures across domain minimum', () => page.locator('#new-data').click(), 'rescaled-data');
      const [min,max] = await domain();
      assert(min<50 && max===100, 'the new measure expands the shared scale');
    }
  } finally {
    await page.close();
  }
  assert.deepEqual(errors, [], 'no browser or network errors during isolated chart updates');
  return rows;
}
