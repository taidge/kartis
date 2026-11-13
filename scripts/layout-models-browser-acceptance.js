(async function verifyLayoutModels(page) {
  const errors = [];
  const onError = error => errors.push(error.message);
  page.on('pageerror', onError);
  const assert = (condition, detail) => { if (!condition) throw Error(detail); };
  const fixture = (method, ...args) => page.evaluate(([method, args]) => window.hostFixture[method](...args), [method, args]);
  const snapshot = async renderer => JSON.parse(await fixture('layout_models_snapshot', renderer));
  const flush = async () => {
    await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
    await fixture('layout_models_flush');
  };
  const priorPanel = await page.evaluate(() => {
    const panels = document.querySelector('.panels');
    const display = panels.style.display;
    panels.style.display = 'none';
    return display;
  });
  const baseline = await page.locator('body *').count();
  const samples = async target => page.evaluate(async ({id, point, gap}) => {
    const svg = document.querySelector('#layout-model-svg .kartis-surface svg');
    const path = [...svg.querySelectorAll('path[data-id]')].find(path => path.dataset.id === id);
    const canvas = document.querySelector('#layout-model-canvas canvas');
    const copy = document.createElement('canvas');
    copy.width = canvas.width;
    copy.height = canvas.height;
    const standalone = svg.cloneNode(true);
    standalone.setAttribute('width', '640');
    standalone.setAttribute('height', '360');
    const svgImage = new Image();
    svgImage.src = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(new XMLSerializer().serializeToString(standalone))}`;
    await svgImage.decode();
    copy.getContext('2d').drawImage(svgImage, 0, 0, copy.width, copy.height);
    const pixel = (surface, [x, y]) => [...surface.getContext('2d').getImageData(
      Math.floor(x * surface.width / 640), Math.floor(y * surface.height / 360), 1, 1,
    ).data];
    return {svg: pixel(copy, point), canvas: pixel(canvas, point),
      gap: gap ? [pixel(copy, gap), pixel(canvas, gap)] : null,
      pathValue: Number(path?.dataset.value), retained: window.layoutModelPath ? window.layoutModelPath === path : null};
  }, target);
  const move = async (renderer, point, click = false) => {
    const surface = page.locator(`#layout-model-${renderer} .kartis-surface`);
    await surface.scrollIntoViewIfNeeded();
    await page.mouse.move(2, 2);
    const box = await surface.boundingBox();
    const [x, y] = [box.x + point[0] / 640 * box.width, box.y + point[1] / 360 * box.height];
    await page.mouse.move(x, y);
    if (click) await page.mouse.click(x, y);
    await flush();
  };
  const cases = [];
  try {
    for (const kind of ['funnel', 'waffle-cells', 'waffle-areas', 'sunburst']) {
      await page.mouse.move(2, 2);
      await fixture('layout_models_mount', kind);
      await flush();
      const initial = (await snapshot('svg')).target;
      assert(initial.id && initial.kind === kind, `${kind}: missing target`);
      const firstPixels = await samples(initial);
      const checkPixels = (pixels, target) => {
        assert(pixels.pathValue === target.value && [pixels.svg, pixels.canvas].every(pixel => pixel.join() === target.color.join()),
          `${kind}: SVG or Canvas fill differs from typed color ${JSON.stringify({pixels,target})}`);
        if (target.gap) {
          const expected = kind === 'waffle-cells' ? '255,255,255,255' : target.color.join();
          assert(pixels.gap.every(pixel => pixel.join() === expected), `${kind}: grid padding profile mismatch ${JSON.stringify(pixels.gap)}`);
        }
      };
      checkPixels(firstPixels, initial);
      await page.evaluate(id => {
        window.layoutModelPath = [...document.querySelectorAll('#layout-model-svg .kartis-surface svg path[data-id]')]
          .find(path => path.dataset.id === id);
      }, initial.id);
      await fixture('layout_models_phase', kind, 1);
      await flush();
      const target = (await snapshot('svg')).target;
      assert(target.id === initial.id && target.value === initial.value + 8, `${kind}: update lost ID or source value`);
      const secondPixels = await samples(target);
      checkPixels(secondPixels, target);
      assert(secondPixels.retained, `${kind}: update replaced the keyed SVG path`);
      if (kind === 'funnel') {
        assert(target.metadata.current && target.metadata.sourceIndex === 1 && target.metadata.separators === 8,
          `${kind}: stage profile or separators absent ${JSON.stringify(target.metadata)}`);
      } else if (kind === 'sunburst') {
        assert(target.metadata.own === 24 && target.metadata.parentTotal === 38 &&
          target.metadata.angles[1] > target.metadata.angles[0], `${kind}: D3 partition metadata incorrect`);
      } else {
        assert(target.metadata.cells === 32 && target.metadata.occupied === 20 && target.metadata.polygons === 1,
          `${kind}: slot allocation or area topology incorrect ${JSON.stringify(target.metadata)}`);
      }
      for (const renderer of ['svg', 'canvas']) {
        if (target.gap) {
          await move(renderer, target.gap);
          const hovered = (await snapshot(renderer)).hovered;
          assert(hovered.includes(target.id) === (kind === 'waffle-areas'),
            `${kind}/${renderer}: gap picking differs from painted profile ${JSON.stringify({hovered,gap:target.gap})}`);
        }
        await move(renderer, target.point, true);
        const picked = await snapshot(renderer);
        assert(picked.hovered.includes(target.id) && picked.selected === target.id &&
          picked.events.some(event => event.type === 'activated' && event.id === target.id),
          `${kind}/${renderer}: typed pointer activation mismatch ${JSON.stringify(picked)}`);
        const control = page.locator(`#layout-model-${renderer} .kartis-datum`).filter({hasText: target.label}).first();
        const activations = picked.events.filter(event => event.type === 'activated').length;
        await control.focus();
        await page.keyboard.press('Enter');
        await flush();
        assert((await snapshot(renderer)).events.filter(event => event.type === 'activated').length > activations,
          `${kind}/${renderer}: native keyboard did not activate data control`);
      }
      await fixture('layout_models_stop');
      assert(await page.locator('body *').count() === baseline, `${kind}: retained chart leaked DOM`);
      await page.evaluate(() => { delete window.layoutModelPath; });
      cases.push({kind, firstPixels, secondPixels, value: target.value, typed: target.metadata, pointer: true, keyboard: true, cleanup: true});
    }
    assert(errors.length === 0, errors.join('\n'));
    return {passed: true, cases, pageErrors: errors};
  } finally {
    await fixture('layout_models_stop');
    await page.evaluate(display => {
      document.querySelector('.panels').style.display = display;
      delete window.layoutModelPath;
    }, priorPanel);
    page.off('pageerror', onError);
  }
})
