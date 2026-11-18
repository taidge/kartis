import assert from 'node:assert/strict';

// Observe the actual plotted layer separately from the tooltip and hover outline.
function watchBase(surface, renderer) {
  const findBase = () => surface.querySelector(':scope > ' + renderer);
  const base = findBase();
  const counts = {mutations: 0, clears: 0, draws: 0};
  const observer = new MutationObserver(records => { counts.mutations += records.length; });
  observer.observe(base, {attributes: true, subtree: true, childList: true, characterData: true});
  const restore = [];
  if (renderer === 'canvas') {
    const context = base.getContext('2d');
    for (const name of ['reset', 'clearRect', 'fillRect', 'strokeRect', 'fill', 'stroke',
      'fillText', 'strokeText', 'drawImage', 'putImageData']) {
      const original = context[name];
      if (typeof original !== 'function') continue;
      context[name] = function(...args) {
        counts[name === 'clearRect' || name === 'reset' ? 'clears' : 'draws']++;
        return original.apply(this, args);
      };
      restore.push(() => { delete context[name]; });
    }
  }
  const ink = () => renderer === 'svg' ? base.innerHTML : base.toDataURL();
  const initial = ink();
  return {
    read() {
      counts.mutations += observer.takeRecords().length;
      return {...counts, sameNode: base === findBase(), sameInk: initial === ink()};
    },
    stop() { observer.disconnect(); restore.forEach(fn => fn()); },
  };
}

async function referenceTargets(surface) {
  return surface.evaluate(node => {
    const bounds = node.getBoundingClientRect();
    const paths = [...node.querySelectorAll(':scope > svg path[data-id][data-series]')]
      .filter(path => path.dataset.series && path.getBoundingClientRect().width > 5
        && path.getBoundingClientRect().height > 5);
    const first = paths[0];
    const second = paths.find(path => path.dataset.series !== first.dataset.series) ?? paths[1];
    return [first, second].map(path => {
      const rect = path.getBoundingClientRect();
      return {label: path.querySelector('title').textContent,
        x: (rect.x + rect.width / 2 - bounds.x) / bounds.width,
        y: (rect.y + rect.height / 2 - bounds.y) / bounds.height};
    });
  });
}

async function checkHeatmapMotion(page, surface, renderer, select, settle, first, second, move) {
  const animation = page.locator('#animation-enabled');
  const preset = page.locator('#spring-preset');
  const wasEnabled = await animation.isChecked();
  const originalPreset = await preset.inputValue();
  const reduced = await page.evaluate(() => matchMedia('(prefers-reduced-motion: reduce)').matches);
  const paintHash = () => surface.evaluate(async (node, renderer) => {
    const base = node.querySelector(':scope > ' + renderer);
    const pixels = renderer === 'canvas' ? base.getContext('2d').getImageData(0, 0, base.width, base.height).data :
      new TextEncoder().encode(JSON.stringify([...base.querySelectorAll('path,rect,text')].map(mark =>
        ['d','x','y','width','height','fill','stroke','opacity'].map(key => mark.getAttribute(key)).concat(mark.textContent))));
    return [...new Uint8Array(await crypto.subtle.digest('SHA-256', pixels))].join(',');
  }, renderer);
  try {
    await page.emulateMedia({reducedMotion: 'no-preference'});
    await animation.check();
    await preset.selectOption('gentle');
    const start = await settle();
    await surface.scrollIntoViewIfNeeded();
    await move(first);
    const initial = await paintHash();
    await page.locator('#new-data').evaluate(button => button.click());
    await page.waitForFunction(() => window.kartisExplorer.snapshot().animating);
    const liveSurface = await surface.elementHandle();
    try {
      await preset.evaluate(node => {
        node.value = 'wobbly';
        node.dispatchEvent(new Event('change', {bubbles:true}));
      });
      assert(await liveSurface.evaluate(node => node === document.querySelector('#chart-stage .kartis-surface')),
        renderer + ' changing the running spring preserves its chart');
    } finally { await liveSurface.dispose(); }
    await move(second);
    assert(await page.evaluate(() => window.kartisExplorer.snapshot().animating),
      renderer + ' hover while shuffling leaves the running heat map spring active');
    const middle = await paintHash();
    const end = await settle();
    assert.equal(end.seed, start.seed + 1);
    const final = await paintHash();
    assert.equal(new Set([initial, middle, final]).size, 3, renderer + ' spring has distinct painted frames');
    const idle = await surface.evaluateHandle(watchBase, renderer);
    try {
      await move(second, 1);
      assert.deepEqual(await idle.evaluate(probe => probe.read()),
        {mutations: 0, clears: 0, draws: 0, sameNode: true, sameInk: true});
    } finally { await idle.evaluate(probe => probe.stop()); await idle.dispose(); }
    await animation.uncheck();
    await select('heatmap');
    const direct = await settle();
    assert.equal(direct.seed, end.seed);
    assert.equal(await paintHash(), final, renderer + ' settled heat map matches direct rendering');
    return {duringSpring: true, distinctFrames: 3, settledMatchesDirect: true, idleRetained: true};
  } finally {
    await preset.selectOption(originalPreset);
    await animation.setChecked(wasEnabled);
    await page.emulateMedia({reducedMotion: reduced ? 'reduce' : 'no-preference'});
    await settle();
  }
}

export async function checkHoverPainting(page, select, settle) {
  const report = [];
  const surface = page.locator('#chart-stage .kartis-surface');
  const tip = page.locator('#chart-stage .kartis-surface-tooltip');
  const outline = page.locator('#chart-stage .kartis-datum-outline path[data-id]');
  const content = page.locator('#chart-option-tooltip-content');
  const frames = () => page.evaluate(() => new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r))));
  for (const slug of ['heatmap', 'bar', 'calendar']) {
    await select(slug);
    for (const renderer of ['svg', 'canvas']) {
      const context = slug + '/' + renderer;
      await page.locator('#renderer-svg').click();
      await content.selectOption('detail');
      await settle();
      await surface.scrollIntoViewIfNeeded();
      const [first, second] = await referenceTargets(surface);
      await page.locator('#renderer-' + renderer).click();
      await settle();
      await surface.scrollIntoViewIfNeeded();
      const move = async (target, offset = 0) => {
        const bounds = await surface.boundingBox();
        await page.mouse.move(bounds.x + bounds.width * target.x + offset, bounds.y + bounds.height * target.y);
        await frames();
      };
      const leave = async () => {
        const bounds = await surface.boundingBox();
        await page.mouse.move(bounds.x - 8, bounds.y + 5);
        await frames();
      };
      await leave();
      const idleBase = await surface.evaluateHandle(watchBase, renderer);
      let idle;
      try {
        await move(first);
        await tip.waitFor({state: 'visible'});
        assert((await tip.textContent()).includes(first.label), context + ' hit the first datum');
        const text = await tip.evaluateHandle(node => node.firstChild);
        const firstPath = await outline.first().getAttribute('d', {timeout: 3000});
        assert(firstPath, context + ' painted the hovered datum outline');
        const retainedPath = await outline.first().elementHandle();
        try {
          await move(first, 2);
          assert(await tip.evaluate((node, previous) => node.firstChild === previous, text),
            context + ' retains tooltip text when only its position changes');
          assert(await outline.first().evaluate((node, previous) => node === previous, retainedPath),
            context + ' retains the outline node within a datum');
          assert.equal(await outline.first().getAttribute('d'), firstPath);
        } finally { await text.dispose(); await retainedPath.dispose(); }
        await move(second);
        assert((await tip.textContent()).includes(second.label), context + ' retargeted the tooltip');
        assert.notEqual(await outline.first().getAttribute('d'), firstPath, context + ' moved the outline');
        await leave();
        await tip.waitFor({state: 'hidden'});
        await outline.first().waitFor({state: 'hidden'});
        idle = await idleBase.evaluate(probe => probe.read());
        assert.deepEqual(idle, {mutations: 0, clears: 0, draws: 0, sameNode: true, sameInk: true},
          context + ' pointer entry, same-datum movement, crossing and exit retain all base ink');
      } finally {
        await idleBase.evaluate(probe => probe.stop());
        await idleBase.dispose();
      }
      await content.selectOption('hidden');
      await settle();
      await surface.scrollIntoViewIfNeeded();
      await move(first);
      await tip.waitFor({state: 'hidden'});
      assert(await outline.first().isVisible(), context + ' still indicates the hit with tooltips hidden');
      await leave();
      await content.selectOption('detail');
      const before = await settle();
      const updateBase = await surface.evaluateHandle(watchBase, renderer);
      try {
        await page.locator('#new-data').click();
        const after = await settle();
        const changed = await updateBase.evaluate(probe => probe.read());
        assert.equal(after.seed, before.seed + 1);
        assert(changed.sameNode && !changed.sameInk, context + ' data change updates the same live plot');
        assert(renderer === 'svg' ? changed.mutations > 0 : changed.draws > 0,
          context + ' instrumentation detects actual plot writes');
      } finally {
        await updateBase.evaluate(probe => probe.stop());
        await updateBase.dispose();
      }
      const motion = slug === 'heatmap'
        ? await checkHeatmapMotion(page, surface, renderer, select, settle, first, second, move) : undefined;
      report.push({slug, renderer, idle, dataUpdated: true, motion});
    }
  }
  return report;
}
