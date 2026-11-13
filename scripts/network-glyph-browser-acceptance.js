(async function verifyNetworkGlyphs(page) {
  const assert = (condition, message) => { if (!condition) throw Error(message); };
  const ids = { star: 'force-node:a:wing', hollow: 'force-node:b:hollow' };
  const renderers = ['svg', 'canvas'];
  const errors = [];
  const onError = error => errors.push(error.message);
  page.on('pageerror', onError);
  const snap = name => page.evaluate(name => JSON.parse(window.hostFixture.network_glyphs_snapshot(name)), name);
  const node = (state, id) => state.nodes.find(item => `force-node:${item.id}` === id);
  const root = name => page.locator(`#network-glyphs-${name}`);
  const surface = name => root(name).locator('.kartis-surface');
  const near = (a, b, error = 1e-7) => Math.hypot(a[0] - b[0], a[1] - b[1]) < error;
  const flush = async () => {
    await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
    await page.evaluate(() => window.hostFixture.network_glyphs_flush());
  };
  const pointer = async (name, point) => {
    await surface(name).scrollIntoViewIfNeeded();
    const box = await surface(name).boundingBox();
    const state = await snap(name);
    const [scale, tx, ty] = state.view;
    await page.mouse.move(box.x + (point[0] * scale + tx) * box.width / state.width,
      box.y + (point[1] * scale + ty) * box.height / state.height);
    await flush();
    return snap(name);
  };
  const path = id => root('svg').locator(`[data-id="${id}"]`);
  const parity = () => page.evaluate(async () => {
    const live = document.querySelector('#network-glyphs-canvas canvas');
    const svg = document.querySelector('#network-glyphs-svg svg').cloneNode(true);
    svg.setAttribute('width', String(live.width));
    svg.setAttribute('height', String(live.height));
    svg.setAttributeNS('http://www.w3.org/XML/1998/namespace', 'xml:lang', document.documentElement.lang || navigator.language);
    const image = new Image();
    image.src = 'data:image/svg+xml;charset=utf-8,' + encodeURIComponent(new XMLSerializer().serializeToString(svg));
    await image.decode();
    const copy = document.createElement('canvas');
    copy.width = live.width;
    copy.height = live.height;
    copy.getContext('2d').drawImage(image, 0, 0);
    const a = copy.getContext('2d').getImageData(0, 0, copy.width, copy.height).data;
    const b = live.getContext('2d').getImageData(0, 0, live.width, live.height).data;
    let error = 0, ink = 0;
    for (let i = 0; i < a.length; i++) error += Math.abs(a[i] - b[i]);
    for (let i = 0; i < b.length; i += 4) ink += Number(Math.min(b[i], b[i + 1], b[i + 2]) < 180);
    return { mean: error / a.length, ink };
  });
  const oldPanels = await page.evaluate(() => {
    const panels = document.querySelector('.panels');
    const value = panels.style.display;
    panels.style.display = 'none';
    return value;
  });
  const baseline = await page.locator('body *').count();
  try {
    assert(!(await snap('svg')).active, 'Unmounted Network glyph snapshot is active');
    await page.evaluate(() => window.hostFixture.network_glyphs_mount());
    await flush();
    const initialPixels = await parity();
    assert(initialPixels.mean < 0.5 && initialPixels.ink > 1500, `Network glyph initial SVG/Canvas pixels ${JSON.stringify(initialPixels)}`);
    const initialPath = await path(ids.star).getAttribute('d');
    await path(ids.star).evaluate(el => { window.networkGlyphStar = el; });
    assert((initialPath.match(/[ML]/g) || []).length === 10, 'Star did not use its ten actual support points');
    const border = await path(ids.hollow).evaluate(el => {
      const [x, y] = el.getAttribute('d').match(/-?(?:\d*\.)?\d+(?:e[-+]?\d+)?/gi).map(Number);
      return [x, y];
    });
    const cases = [];
    for (const name of renderers) {
      let state = await snap(name);
      const hollow = node(state, ids.hollow);
      state = await pointer(name, hollow.anchor);
      assert(!state.hovered.includes(ids.hollow), `${name} hollow center acted as painted ink`);
      await page.mouse.down();
      await flush();
      assert(!(await snap(name)).dragging, `${name} hollow center started a force drag`);
      await page.mouse.up();
      await flush();
      state = await pointer(name, border);
      assert(state.hovered.includes(ids.hollow) && near(state.interaction.anchor, hollow.anchor), `${name} border did not retain the actual typed origin`);
      assert(state.interaction.rows[0].value === null, `${name} force radius appeared as a raw datum value`);
      assert(await path(ids.hollow).getAttribute('stroke-width') === '5', 'Highlight changed the authored hollow stroke');

      const star = node(state, ids.star);
      const wing = [star.anchor[0], star.anchor[1] - 26];
      await pointer(name, wing);
      await page.mouse.wheel(0, -120);
      await flush();
      state = await pointer(name, wing);
      assert(state.view[0] > 1 && state.hovered.includes(ids.star), `${name} zoomed Star wing outside its force radius lost the hit`);
      await page.mouse.down();
      await flush();
      const grabbed = await snap(name);
      assert(grabbed.dragging?.node === 'a:wing' && grabbed.capture !== null, `${name} actual glyph wing did not acquire capture`);
      assert(near(node(grabbed, ids.star).world, star.world) && Math.abs(grabbed.dragging.grab[1]) > star.radius + 15, `${name} drag snapped to the pointer or physics disk`);

      const rejection = await page.evaluate(name => {
        const path = document.querySelector('#network-glyphs-svg [data-id="force-node:a:wing"]');
        const data = path.getAttribute('d');
        const before = JSON.parse(window.hostFixture.network_glyphs_snapshot(name));
        const rejected = window.hostFixture.network_glyphs_rejected(name);
        const after = JSON.parse(window.hostFixture.network_glyphs_snapshot(name));
        return { rejected, before, after, samePath: path.getAttribute('d') === data };
      }, name);
      assert(rejection.rejected && rejection.samePath, `${name} invalid style replacement modified retained glyph`);
      const { before, after } = rejection;
      assert(after.revision === before.revision && after.pending === before.pending && after.capture === before.capture && near(node(after, ids.star).world, node(before, ids.star).world), `${name} invalid replacement changed capture or physics`);

      await root(name).evaluate(el => { el.style.width = '600px'; });
      await flush();
      const resized = await snap(name);
      assert(resized.width === 600 && resized.dragging?.node === 'a:wing' && near(node(resized, ids.star).world, star.world), `${name} resize failed to rebase the same drag`);
      // The force driver maps native client coordinates against the DOM rect.
      // Firefox's Playwright quad-based boundingBox can round its height differently.
      const box = await surface(name).evaluate(element => {
        const { width, height } = element.getBoundingClientRect();
        return { width, height };
      });
      await page.mouse.move(resized.dragging.last[0] + 18, resized.dragging.last[1] + 11);
      await flush();
      const moved = await snap(name);
      const expected = node(resized, ids.star).world.map((value, index) => value +
        (moved.dragging.last[index] - resized.dragging.last[index]) * (index ? resized.height / box.height : resized.width / box.width) / resized.view[0]);
      assert(near(node(moved, ids.star).world, expected), `${name} move ignored the rebased anchor under CSS scaling and camera zoom`);
      await page.mouse.up();
      await flush();
      state = await snap(name);
      assert(!state.dragging && state.capture === null && near(node(state, ids.star).fixed, expected), `${name} release lost pinned position`);
      if (name === 'svg') {
        assert(await path(ids.star).evaluate(el => el === window.networkGlyphStar), 'Force frames replaced the SVG node');
      }

      await page.evaluate(name => window.hostFixture.network_glyphs_hidden(name, true), name);
      await flush();
      state = await snap(name);
      await pointer(name, node(state, ids.star).anchor);
      await page.mouse.down();
      await flush();
      const hidden = await snap(name);
      assert(hidden.hidden.includes('a:wing') && !hidden.hovered.includes(ids.star) && !hidden.dragging, `${name} hidden glyph remained pickable`);
      await page.mouse.up();
      await flush();
      if (name === 'svg') {
        assert(await path(ids.star).count() === 0 && await root(name).locator('svg text').filter({ hasText: 'Star <safe>' }).count() === 0, 'Hidden glyph or its label remained painted');
      }
      await page.evaluate(name => window.hostFixture.network_glyphs_hidden(name, false), name);
      await flush();
      cases.push({ renderer: name, actualHollowAndWing: true, typedOrigin: true, resizeRebase: true, rejectedAtomic: true, releasePinned: true, hidden: true });
    }
    // Restore shared frame dimensions and verify that each camera survives
    // replacement before resetting both cameras for pixel parity.
    for (const name of renderers) await root(name).evaluate(el => { el.style.width = '640px'; });
    await flush();
    const beforeViews = await Promise.all(renderers.map(snap));
    await path(ids.star).evaluate(el => { window.networkGlyphBeforeReplacement = el; });
    await page.evaluate(() => window.hostFixture.network_glyphs_phase(1));
    await flush();
    const afterViews = await Promise.all(renderers.map(snap));
    assert(afterViews.every((state, index) => state.view.every((value, coordinate) => value === beforeViews[index].view[coordinate])), 'Replacement changed a live camera');
    await page.evaluate(() => window.hostFixture.network_glyphs_reset_view());
    await flush();
    const replacementPixels = await parity();
    assert(replacementPixels.mean < 0.5 && replacementPixels.ink > 1500, `Network glyph replacement pixels ${JSON.stringify(replacementPixels)}`);
    assert(await path(ids.star).evaluate(el => el === window.networkGlyphBeforeReplacement), 'Replacement retired the stable SVG glyph');
    assert((await path(ids.star).getAttribute('d')).match(/[ML]/g).length === 9, 'Wye replacement did not update actual path topology');
    for (const name of renderers) {
      const state = await snap(name);
      const current = await pointer(name, node(state, ids.star).anchor);
      assert(current.interaction?.rows[0].label === 'Wye updated <safe>' && near(current.interaction.anchor, node(current, ids.star).anchor), `${name} old metadata leaked through replacement`);
      assert(current.error === null && current.events.every(event => event.kind !== 'error'), `${name} force frame reported an error`);
    }
    await page.evaluate(() => {
      window.networkGlyphRecords = [];
      window.networkGlyphObserver = new MutationObserver(records => window.networkGlyphRecords.push(...records));
      window.networkGlyphObserver.observe(document.querySelector('#network-glyphs-svg svg'), { subtree: true, attributes: true, childList: true, characterData: true });
    });
    await flush();
    assert(await page.evaluate(() => {
      const count = window.networkGlyphRecords.length + window.networkGlyphObserver.takeRecords().length;
      window.networkGlyphObserver.disconnect();
      return count === 0;
    }), 'An unchanged force frame mutated the retained SVG');
    await page.evaluate(() => window.hostFixture.network_glyphs_stop());
    assert(await page.locator('body *').count() === baseline && !(await snap('canvas')).active, 'Network glyph owner leaked its mount');
    assert(errors.length === 0, `Network glyph page errors ${JSON.stringify(errors)}`);
    return { passed: true, initialPixels, replacementPixels, cases, retainedReplacement: true, zeroMutation: true, cleanup: true, pageErrors: errors };
  } finally {
    await page.evaluate(oldPanels => {
      window.hostFixture.network_glyphs_stop();
      document.querySelector('.panels').style.display = oldPanels;
    }, oldPanels);
    page.off('pageerror', onError);
  }
})
