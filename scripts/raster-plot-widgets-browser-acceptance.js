// Decode only the real Rust-produced PNG; native Glory buttons remain live DOM.
(async (page, url = 'http://127.0.0.1:8815') => {
  const errors = [];
  const onError = error => errors.push(error.message);
  page.on('pageerror', onError);
  const assert = (condition, message) => { if (!condition) throw Error(message); };
  const near = (actual, expected, tolerance = 1e-8) => actual.length === expected.length &&
    actual.every((value, index) => Math.abs(value - expected[index]) <= tolerance);
  const call = (name, ...args) => page.evaluate(([name, args]) => window.rasterFixture[name](...args), [name, args]);
  const snapshot = () => page.evaluate(() => JSON.parse(window.rasterFixture.raster_overlay_snapshot()));
  const settled = () => page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  const image = () => page.locator('#app img.kartis-raster-image');
  const button = () => page.locator('#app button.raster-overlay-alpha');
  const ready = async () => image().evaluate(async image => {
    await image.decode();
    if (!image.complete || !image.naturalWidth) throw Error('actual CPU PNG did not decode');
  });
  const value = async (value, count) => page.waitForFunction(([value, count]) => {
    const snapshot = JSON.parse(window.rasterFixture.raster_overlay_snapshot());
    return snapshot.frame?.value === value && snapshot.count === count &&
      document.querySelector('#app .raster-overlay-alpha')?.textContent === `${value} / ${count}`;
  }, [value, count]);
  const sample = () => image().evaluate(image => {
    const canvas = document.createElement('canvas');
    canvas.width = image.naturalWidth; canvas.height = image.naturalHeight;
    const context = canvas.getContext('2d'); context.drawImage(image, 0, 0);
    const sx = image.naturalWidth / parseFloat(image.style.width);
    const sy = image.naturalHeight / parseFloat(image.style.height);
    const pixel = (x, y) => [...context.getImageData(Math.floor(x * sx), Math.floor(y * sy), 1, 1).data];
    const origin = image.getBoundingClientRect();
    const slot = document.querySelector('#app .kartis-plot-widget-clip').getBoundingClientRect();
    return {
      source: image.getAttribute('src'), natural: [image.naturalWidth, image.naturalHeight],
      logical: [parseFloat(image.style.width), parseFloat(image.style.height)],
      rect: [slot.x - origin.x, slot.y - origin.y, slot.width, slot.height],
      initial: pixel(115, 70), center: pixel(180, 96), movedEdge: pixel(147, 96),
    };
  });
  const identity = () => page.evaluate(() =>
    document.querySelector('#app .raster-overlay-alpha') === window.rasterOverlayButton &&
    document.querySelector('#app .kartis-raster-image') === window.rasterOverlayImage &&
    document.querySelector('#app .kartis-plot-widget-clip') === window.rasterOverlayClip);
  try {
    await page.goto(url);
    await page.waitForFunction(() => window.rasterReady === true &&
      typeof window.rasterFixture.mount_raster_overlay_fixture === 'function');
    await call('mount_raster_overlay_fixture');
    await value(10, 0); await ready();
    const initial = await sample();
    assert(initial.source.startsWith('data:image/png;base64,iVBORw0KGgo'), 'Overlay plot is not actual Rust PNG');
    assert(near(initial.natural, [300, 160]) && near(initial.logical, [300, 160]), 'Initial image dimensions differ');
    assert(initial.initial.join() === '210,40,50,255', 'Initial PNG is missing the raw alpha circle');
    assert(near(initial.rect, [55, 54, 120, 32], 0.04), `Initial measured slot differs: ${initial.rect}`);
    assert(await page.locator('#app svg,#app canvas').count() === 0, 'Raster overlay allocated semantic graphics');
    await page.evaluate(() => {
      window.rasterOverlayButton = document.querySelector('#app .raster-overlay-alpha');
      window.rasterOverlayImage = document.querySelector('#app .kartis-raster-image');
      window.rasterOverlayClip = document.querySelector('#app .kartis-plot-widget-clip');
    });
    await button().click(); await value(10, 1);
    await call('raster_overlay_phase', 1);
    await value(20, 1); await ready();
    const updated = await sample();
    let accepted = await snapshot();
    assert(near(updated.natural, [600, 320]) && near(updated.logical, [300, 160]), 'DPR2 changed logical image dimensions');
    assert(near(accepted.frame.rect, [108, 76.8, 144, 38.4]), 'Accepted datum frame did not project through the raster camera');
    assert(near(updated.rect, [108, 76.8, 144, 38.4], 0.04), `Actual DOM slot differs from projected frame: ${updated.rect}`);
    assert(updated.center.join() === '30,90,190,255' && updated.movedEdge.join() === '30,90,190,255', 'PNG view/DPR lost real circle ink');
    assert(await identity(), 'View/geometry/value update rebuilt the original button, clip or image');
    assert(accepted.factories === 1 && accepted.count === 1, 'Local widget state was reconstructed');

    await call('raster_overlay_phase', 2);
    await page.waitForFunction(() => {
      const s = JSON.parse(window.rasterFixture.raster_overlay_snapshot());
      return s.frame?.interactive === false && s.frame.visible?.[2] === 12;
    });
    accepted = await snapshot();
    assert(near(accepted.frame.visible, [168, 84, 12, 24]), 'Partial clip failed to project the world mark.clip');
    await page.evaluate(() => window.rasterOverlayButton.click());
    await settled();
    assert((await snapshot()).count === 1, 'Clipped inert button received an activation');
    assert(await identity(), 'Partial clip rebuilt a retained child');
    await call('raster_overlay_phase', 0); await value(10, 1); await ready();
    assert((await sample()).initial.join() === '210,40,50,255', 'Phase0 did not restore original raster view');
    await call('raster_overlay_phase', 1); await value(20, 1); await ready();

    const beforeFailure = await sample();
    const beforeFrame = (await snapshot()).frame;
    await call('raster_overlay_update', 21, true);
    await page.waitForFunction(() => JSON.parse(window.rasterFixture.raster_overlay_snapshot()).errors
      .some(error => error.includes('raster overlay publication rejected')));
    accepted = await snapshot();
    assert(accepted.frame.value === 20 && JSON.stringify(accepted.frame) === JSON.stringify(beforeFrame), 'Rejected publisher exposed a candidate widget frame');
    assert(await image().getAttribute('src') === beforeFailure.source && await button().textContent() === '20 / 1', 'Rejected publication changed either PNG or local widget label');
    assert(await identity(), 'Rejected publication replaced a preceding native node');
    await call('raster_overlay_update', 22, false);
    await value(22, 1); await ready();
    const retried = await sample();
    accepted = await snapshot();
    assert(retried.source !== beforeFailure.source && retried.movedEdge.join() === '255,255,255,255', 'Retry did not publish the new shifted circle PNG');
    assert(near(accepted.frame.rect, [110.4, 76.8, 144, 38.4]) && near(retried.rect, [110.4, 76.8, 144, 38.4], 0.04), 'Retry did not commit PNG and widget geometry together');
    assert(await identity() && accepted.factories === 1, 'Retry rebuilt native widget state');

    await page.locator('#app .raster-overlay-toggle').click();
    await page.waitForFunction(() => JSON.parse(window.rasterFixture.raster_overlay_snapshot()).frame?.visible === null);
    await ready();
    assert((await sample()).center.join() === '255,255,255,255' && !await button().isVisible(), 'Hidden series retained raster or overlay ink');
    await page.evaluate(() => window.rasterOverlayButton.click());
    await settled();
    assert((await snapshot()).count === 1, 'Hidden child callback remained active');
    await page.locator('#app .raster-overlay-toggle').click();
    await page.waitForFunction(() => JSON.parse(window.rasterFixture.raster_overlay_snapshot()).frame?.interactive === true &&
      document.querySelector('#app .raster-overlay-alpha')?.textContent === '22 / 1');
    await ready(); assert(await identity(), 'Hide/restore reconstructed the native overlay');

    await call('clear_raster_overlay_fixture');
    await page.waitForFunction(() => document.querySelectorAll('#app *').length === 0);
    const retired = await snapshot();
    assert(!retired.active && retired.nodes === 0 && retired.handlers === 0 && retired.childrenCleaned === 1 &&
      retired.factoryDrops === 1 && retired.publisherDrops === 1, `Owner/capture cleanup failed: ${JSON.stringify(retired)}`);
    await page.evaluate(() => window.rasterOverlayButton.click());
    await settled();
    const stale = await snapshot();
    assert(stale.count === retired.count && stale.publications === retired.publications && stale.factories === retired.factories, 'Retired native callback or publisher remained live');
    await call('mount_raster_overlay_fixture'); await value(10, 0); await ready();
    assert(await page.locator('#app .raster-overlay-alpha').count() === 1 &&
      await page.locator('#app .kartis-raster-image').count() === 1 && (await snapshot()).factories === 1, 'Remount leaked duplicate DOM or stale local state');
    await call('stop_raster_overlay_fixture');
    await page.waitForFunction(() => document.querySelectorAll('#app *').length === 0);
    assert(errors.length === 0, errors.join('\n'));
    return {passed:true, actualCpuPng:true, nativeGloryWidget:true, localStateRetained:true,
      viewAndDpr:true, partialClipInert:true, publicationAtomic:true, hideRestore:true,
      captureCleanup:true, staleCallbacks:true, remount:true, pageErrors:errors};
  } finally {
    page.off('pageerror', onError);
  }
})
