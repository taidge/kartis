(async page => {
  const assert = (condition, message) => { if (!condition) throw Error(message); };
  const errors = [], onError = error => errors.push(error.message);
  page.on('pageerror', onError);
  const snapshot = renderer => page.evaluate(renderer => JSON.parse(hostFixture.swarm_snapshot(renderer)), renderer);
  const flush = async () => {
    // Glory controls publish through their own animation-frame queue.
    await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
    await page.evaluate(() => hostFixture.swarm_flush());
  };
  const panels = await page.evaluate(() => {
    const panel = document.querySelector('.panels'), previous = panel.style.display;
    panel.style.display = 'none';
    return previous;
  });
  const baseline = await page.locator('body *').count();
  const comparisons = [];
  const close = (a, b) => Math.abs(a - b) < 1e-6;
  const move = async (renderer, point, width, height, click = false) => {
    const surface = page.locator(`#swarm-${renderer} .kartis-surface`);
    await surface.scrollIntoViewIfNeeded();
    const box = await surface.boundingBox();
    const x = box.x + point[0] * box.width / width, y = box.y + point[1] * box.height / height;
    if (click) await page.mouse.click(x, y);
    else await page.mouse.move(x, y);
    await flush();
  };
  const pixels = () => page.evaluate(async () => {
    const source = document.querySelector('#swarm-svg svg').cloneNode(true);
    const actual = document.querySelector('#swarm-canvas canvas');
    const image = new Image();
    source.setAttribute('width', '640');
    source.setAttribute('height', '360');
    source.setAttributeNS('http://www.w3.org/XML/1998/namespace', 'xml:lang', document.documentElement.lang || 'en');
    image.src = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(new XMLSerializer().serializeToString(source))}`;
    await image.decode();
    const raster = document.createElement('canvas');
    raster.width = actual.width;
    raster.height = actual.height;
    raster.getContext('2d').drawImage(image, 0, 0, raster.width, raster.height);
    const a = raster.getContext('2d').getImageData(0, 0, raster.width, raster.height).data;
    const b = actual.getContext('2d').getImageData(0, 0, actual.width, actual.height).data;
    let difference = 0, ink = 0;
    for (let i = 0; i < a.length; i++) difference += Math.abs(a[i] - b[i]);
    for (let i = 0; i < b.length; i += 4) if (b[i + 3] && Math.min(b[i], b[i + 1], b[i + 2]) < 200) ink++;
    return {mean: difference / a.length, ink};
  });
  try {
    await page.mouse.move(2, 2);
    await page.evaluate(() => hostFixture.swarm_mount());
    await flush();
    await page.evaluate(() => { window.swarmPath = document.querySelector('#swarm-svg [data-id="swarm-node:5:Trial:6:target"]'); });
    const initial = await snapshot('svg');
    for (const [phase, value] of [[0, 8], [1, 12]]) {
      if (phase) {
        await page.mouse.move(2, 2);
        await page.evaluate(() => hostFixture.swarm_phase());
      }
      const comparison = await pixels();
      assert(comparison.mean < .5 && comparison.ink > 1000, `Swarm ${phase}: SVG/Canvas mismatch ${JSON.stringify(comparison)}`);
      comparisons.push(comparison);
      for (const renderer of ['svg', 'canvas']) {
        const current = await snapshot(renderer);
        const [start, end] = current.range;
        const expected = start + (end - start) * Math.log2(value) / 6;
        assert(current.domain.join(',') === '1,64' && (phase ? end < start : end > start) &&
          close(current.center[0], expected) && close(current.sibling[0], expected) &&
          Math.abs(current.center[1] - current.sibling[1]) >= current.inkRadius * 2 + 5 - 1e-6,
        `Swarm ${renderer} ${phase}: scale or collision failed ${JSON.stringify(current)}`);
        if (phase) assert(current.id === initial.id && current.revision > initial.revision && !close(initial.center[0], expected), `${renderer}: stale Swarm update`);
        await move(renderer, current.center, current.width, current.height);
        assert(!(await snapshot(renderer)).hovered.length, `${renderer}: hollow Swarm center was hit`);
        await move(renderer, current.edge, current.width, current.height);
        const hit = await snapshot(renderer);
        assert(hit.hovered[0] === current.id && hit.active?.value === value && hit.active?.formatted === `${value}.0` && hit.active.position.every((v, i) => close(v, current.center[i])),
          `${renderer}: painted Swarm stroke missed or metadata stale ${JSON.stringify(hit)}`);
        await move(renderer, current.edge, current.width, current.height, true);
        assert((await snapshot(renderer)).selected === current.id, `${renderer}: stroke click did not select`);
      }
    }
    assert(await page.evaluate(() => {
      const current = document.querySelector('#swarm-svg [data-id="swarm-node:5:Trial:6:target"]');
      return current === window.swarmPath && current?.getAttribute('fill') === 'none' && current.getAttribute('stroke-width') === '3';
    }), 'Swarm update lost retained mark identity or authored paint');
    const rejection = await page.evaluate(() => {
      const root = document.querySelector('#swarm-svg');
      const path = root.querySelector('[data-id="swarm-node:5:Trial:6:target"]');
      const revision = root.querySelector('.kartis-surface').getAttribute('data-revision');
      const rejected = hostFixture.swarm_bad_layout();
      return {rejected, unchanged: root.querySelector('.kartis-surface').getAttribute('data-revision') === revision, paint: path.outerHTML};
    });
    assert(rejection.rejected && rejection.unchanged, 'Swarm accepted a stale typed anchor or published a frame during rejection');
    await flush();
    assert(await page.locator('#swarm-svg [data-id="swarm-node:5:Trial:6:target"]').evaluate(path => path.outerHTML) === rejection.paint,
      'Rejected Swarm frame changed retained paint');
    for (const renderer of ['svg', 'canvas']) {
      await page.locator(`#swarm-${renderer} .kartis-legend-item`).filter({hasText:'Trial'}).click();
      await flush();
      assert((await snapshot(renderer)).hidden.includes('Trial'), `${renderer}: Swarm legend missed group ownership`);
    }
    assert(await page.locator('#swarm-svg [data-id="swarm-node:5:Trial:6:target"]').count() === 0, 'Hidden Swarm group retained SVG ink');
    await page.evaluate(() => hostFixture.swarm_stop());
    assert(await page.locator('body *').count() === baseline, 'Swarm stop retained fixture nodes');
    assert(errors.length === 0, errors.join('\n'));
    return {passed: true, comparisons, rendererModes: 4, pageErrors: errors};
  } finally {
    await page.evaluate(previous => {
      hostFixture.swarm_stop();
      document.querySelector('.panels').style.display = previous;
      delete window.swarmPath;
    }, panels);
    page.off('pageerror', onError);
  }
})
