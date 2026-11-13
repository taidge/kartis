(async page => {
  const assert = (condition, message) => { if (!condition) throw Error(message); };
  const errors = [], onError = error => errors.push(error.message);
  page.on('pageerror', onError);
  const previous = await page.evaluate(() => {
    const panel = document.querySelector('.panels'), old = panel.style.display;
    panel.style.display = 'none';
    return old;
  });
  const baseline = await page.locator('body *').count();
  const snapshot = renderer => page.evaluate(renderer => JSON.parse(hostFixture.scatter_snapshot(renderer)), renderer);
  const flush = async () => {
    await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
    await page.evaluate(() => hostFixture.scatter_flush());
  };
  const close = (a, b) => Math.abs(a - b) < 1e-6;
  const same = (a, b) => a?.length === b.length && a.every((v, i) => close(v, b[i]));
  const endpoints = site => site.geometry.commands.filter(command => command[0] <= 1).map(command => command.slice(1));
  const middle = (a, b) => a.map((v, i) => (v + b[i]) / 2);
  const move = async (renderer, point, click = false) => {
    const state = await snapshot(renderer), surface = page.locator(`#scatter-${renderer} .kartis-surface`);
    await surface.scrollIntoViewIfNeeded();
    const rect = await surface.evaluate(element => {
      const {x, y, width, height} = element.getBoundingClientRect();
      return {x, y, width, height};
    });
    const x = rect.x + point[0] * rect.width / state.width, y = rect.y + point[1] * rect.height / state.height;
    if (click) await page.mouse.click(x, y);
    else await page.mouse.move(x, y);
    await flush();
    return snapshot(renderer);
  };
  const parity = () => page.evaluate(async () => {
    const svg = document.querySelector('#scatter-svg svg').cloneNode(true);
    const canvas = document.querySelector('#scatter-canvas canvas');
    svg.setAttribute('width', String(canvas.width));
    svg.setAttribute('height', String(canvas.height));
    svg.setAttributeNS('http://www.w3.org/XML/1998/namespace', 'xml:lang', document.documentElement.lang || 'en');
    const image = new Image();
    image.src = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(new XMLSerializer().serializeToString(svg))}`;
    await image.decode();
    const copy = document.createElement('canvas');
    copy.width = canvas.width;
    copy.height = canvas.height;
    copy.getContext('2d').drawImage(image, 0, 0);
    const actual = canvas.getContext('2d').getImageData(0, 0, canvas.width, canvas.height).data;
    const expected = copy.getContext('2d').getImageData(0, 0, copy.width, copy.height).data;
    let difference = 0, ink = 0;
    for (let i = 0; i < actual.length; i++) difference += Math.abs(actual[i] - expected[i]);
    for (let i = 0; i < actual.length; i += 4) if (actual[i + 3] && Math.min(actual[i], actual[i + 1], actual[i + 2]) < 200) ink++;
    return {mean: difference / actual.length, ink};
  });
  try {
    await page.mouse.move(2, 2);
    await page.evaluate(() => hostFixture.scatter_mount(6));
    await flush();
    const initial = await snapshot('svg');
    const retained = page.locator(`#scatter-svg [data-id="${initial.target.id}"]`);
    await retained.evaluate(path => { window.scatterCustomPath = path; });
    const phases = [];
    for (const phase of [0, 1]) {
      if (phase) {
        await page.mouse.move(2, 2);
        await page.evaluate(() => hostFixture.scatter_phase(1));
        await flush();
      }
      const pixel = await parity();
      assert(pixel.mean < .5 && pixel.ink > 1000, `Custom Scatter SVG/Canvas mismatch ${JSON.stringify(pixel)}`);
      for (const renderer of ['svg', 'canvas']) {
        const state = await snapshot(renderer), {target} = state;
        const map = (range, value) => range[0] + (range[1] - range[0]) * value / 10;
        const [a, b, c, d] = endpoints(target);
        const inside = middle(middle(a, b), middle(c, d));
        const edge = middle(a, b);
        const offset = target.size / 2;
        assert(target.id === initial.target.id && target.geometry.commands.length === 5 &&
          target.size === 24 + phase * 6 && same(target.position, [map(state.xScale.range, 3 + phase), map(state.yScale.range, 5 + phase * 2)]) &&
          same(a, [target.position[0] + .95 * offset, target.position[1] - 1.05 * offset]), `${renderer}: custom shape or mapped origin drifted`);
        assert(!(await move(renderer, target.position)).current && !(await move(renderer, inside)).current,
          `${renderer}: custom anchor or hollow center became filled geometry`);
        const painted = await move(renderer, edge);
        assert(painted.hovered.includes(target.id) && same(painted.current.anchor, target.position) &&
          painted.current.rows[0].rawY.value === 5 + 2 * phase, `${renderer}: custom stroke lost actual pick, current value, or origin`);
        assert((await move(renderer, edge, true)).selected === target.id, `${renderer}: custom stroke click missed`);
        const clipped = state.sites.find(site => site.series === 'alpha' && site.node === 'edge');
        const [topStart, topEnd, bottomEnd, bottomStart] = endpoints(clipped);
        const top = middle(topStart, topEnd), bottom = middle(bottomEnd, bottomStart), plotBottom = state.plot[1] + state.plot[3];
        assert(top[1] < plotBottom && bottom[1] > plotBottom && (await move(renderer, top)).hovered.includes(clipped.id) && !(await move(renderer, bottom)).current,
          `${renderer}: custom silhouette did not obey the actual plot clip`);
      }
      assert(await retained.evaluate(path => path === window.scatterCustomPath && path.getAttribute('fill') === 'none' && path.getAttribute('stroke-width') === '3'),
        'Custom Scatter lost retained geometry or stroke-only paint');
      phases.push({phase, pixel});
    }
    await page.mouse.move(2, 2);
    await flush();
    assert(await page.evaluate(() => hostFixture.scatter_rejected()), 'Custom Scatter accepted an inconsistent typed frame');
    await page.evaluate(() => hostFixture.scatter_nearest(6));
    await flush();
    for (const renderer of ['svg', 'canvas']) {
      const {target} = await snapshot(renderer);
      const [a, b] = endpoints(target);
      assert((await move(renderer, target.position)).hovered.includes(target.id) && !(await move(renderer, middle(a, b))).current,
        `${renderer}: Nearest used the offset path rather than the data origin`);
    }
    await page.evaluate(() => hostFixture.scatter_stop());
    assert(await page.locator('body *').count() === baseline && !(await snapshot('svg')).active, 'Custom Scatter leaked its owner');
    assert(!errors.length, errors.join('\n'));
    return {passed: true, phases, nearestOrigin: true, exactInkAndClip: true, stableIds: true, pageErrors: errors};
  } finally {
    await page.evaluate(previous => {
      hostFixture.scatter_stop();
      document.querySelector('.panels').style.display = previous;
      delete window.scatterCustomPath;
    }, previous);
    page.off('pageerror', onError);
  }
})
