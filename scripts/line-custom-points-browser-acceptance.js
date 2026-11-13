(async function verifyLineCustomPoints(page) {
  const assert = (ok, message) => { if (!ok) throw Error(message); };
  const renderers = ['svg', 'canvas'];
  const errors = [], onError = error => errors.push(error.message);
  page.on('pageerror', onError);
  const previous = await page.evaluate(() => {
    const panel = document.querySelector('.panels');
    const display = panel.style.display;
    panel.style.display = 'none';
    return display;
  });
  const baseline = await page.locator('body *').count();
  const call = (method, ...args) => page.evaluate(([method, args]) =>
    window.hostFixture['line_custom_' + method](...args), [method, args]);
  const snapshot = async renderer => JSON.parse(await call('snapshot', renderer));
  const raf = () => page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  const flush = async () => { await raf(); await call('flush'); };
  const samePoint = (a, b) => a?.length === 2 && a.every((value, i) => Math.abs(value - b[i]) < 1e-8);
  const pointer = async (renderer, [x, y], click = false) => {
    const surface = page.locator('#line-custom-' + renderer + ' .kartis-surface');
    await surface.scrollIntoViewIfNeeded();
    const bounds = await surface.boundingBox(), state = await snapshot(renderer);
    const point = [bounds.x + x * bounds.width / state.width, bounds.y + y * bounds.height / state.height];
    if (click) await page.mouse.click(...point);
    else await page.mouse.move(...point);
    await flush();
    return snapshot(renderer);
  };
  const probes = ({sites: [text, number, edge]}) => {
    const [x, y] = number.position;
    return {
      text: [text.position[0] + 2, text.position[1]],
      origin: [x, y], stroke: [x + 22, y], hollow: [x + 34, y],
      inside: [edge.position[0] - 3, edge.position[1]],
      clipped: [edge.position[0] + 5, edge.position[1]],
    };
  };
  const pixels = ({width, height}, points) => page.evaluate(async ({width, height, points}) => {
    const svg = document.querySelector('#line-custom-svg svg').cloneNode(true);
    const canvas = document.querySelector('#line-custom-canvas canvas');
    const w = canvas.width, h = canvas.height;
    svg.setAttribute('width', String(w));
    svg.setAttribute('height', String(h));
    const image = new Image();
    image.src = 'data:image/svg+xml;charset=utf-8,' + encodeURIComponent(new XMLSerializer().serializeToString(svg));
    await image.decode();
    const copy = Object.assign(document.createElement('canvas'), {width:w, height:h});
    const ctx = copy.getContext('2d');
    ctx.drawImage(image, 0, 0);
    const a = ctx.getImageData(0, 0, w, h).data;
    const b = canvas.getContext('2d').getImageData(0, 0, w, h).data;
    let difference = 0;
    for (let i = 0; i < a.length; i++) difference += Math.abs(a[i] - b[i]);
    const rgba = (data, [x, y]) => {
      const offset = 4 * (Math.floor(y * h / height) * w + Math.floor(x * w / width));
      return [...data.slice(offset, offset + 4)];
    };
    return {mean:difference / a.length, samples:Object.fromEntries(Object.entries(points).map(([key, point]) =>
      [key, {svg:rgba(a, point), canvas:rgba(b, point)}]))};
  }, {width, height, points});

  try {
    await page.mouse.move(2, 2);
    await call('mount', false);
    await flush();
    const initial = await snapshot('svg');
    const path = page.locator('#line-custom-svg [data-id="' + initial.sites[1].id + '"]');
    await path.evaluate(node => { window.lineCustomRetained = node; });
    const phases = [];
    for (const phase of [0, 1]) {
      if (phase) {
        await page.mouse.move(2, 2);
        await call('phase', phase);
        await flush();
      }
      const state = await snapshot('svg');
      const [text, number, edge] = state.sites;
      assert(text.x.text === '1' && number.x.number === 1 && text.formattedX === number.formattedX &&
        text.id !== number.id && number.id === initial.sites[1].id &&
        samePoint(text.position, [60, 180]) && samePoint(number.position, [160, 120 - 40 * phase]),
        'PointLine lost typed keys, stable IDs, or original positions');
      const points = probes(state);
      const proof = await pixels(state, points);
      assert(proof.mean < .5, 'Custom PointLine SVG/Canvas parity ' + proof.mean);
      for (const [key, pair] of Object.entries(proof.samples)) {
        const expected = ['text', 'stroke', 'inside'].includes(key) ? '174,34,97,255' : '255,255,255,255';
        assert(pair.svg.join() === expected && pair.canvas.join() === expected,
          'Custom PointLine pixel ' + key + ': ' + JSON.stringify(pair));
      }
      for (const renderer of renderers) {
        const textHit = await pointer(renderer, points.text);
        assert(textHit.current?.id === text.id && textHit.current.raw === 2, renderer + ': wrong text key');
        assert(!(await pointer(renderer, points.origin)).current && !(await pointer(renderer, points.hollow)).current,
          renderer + ': logical anchor or hollow center became exact ink');
        const hit = await pointer(renderer, points.stroke, true);
        assert(hit.selected === number.id && hit.current?.id === number.id && hit.current.raw === 5 + 2 * phase &&
          samePoint(hit.current.anchor, number.position), renderer + ': wrong current numeric key or anchor');
        assert((await pointer(renderer, points.inside)).current?.id === edge.id &&
          !(await pointer(renderer, points.clipped)).current, renderer + ': wrong plot clip');
      }
      assert(await path.evaluate((node, raw) => node === window.lineCustomRetained &&
        node.getAttribute('data-value') === String(raw) && node.getAttribute('fill') === 'none' &&
        node.getAttribute('stroke-width') === '4' && node.parentElement.getAttribute('viewBox') === '60 20 200 200', number.y),
      'PointLine failed to retain the custom SVG node or update its raw value');
      phases.push({phase, ...proof});
    }
    const markup = await path.evaluate(node => node.outerHTML);
    const rejection = await page.evaluate(() => {
      const status = () => ['svg', 'canvas'].map(renderer => {
        const {revision, pending} = JSON.parse(window.hostFixture.line_custom_snapshot(renderer));
        return [revision, pending];
      });
      const before = status(), rejected = window.hostFixture.line_custom_rejected();
      return {before, rejected, after:status()};
    });
    assert(rejection.rejected && rejection.before.every(([revision, pending], i) =>
      pending === 0 && rejection.after[i][0] === revision && rejection.after[i][1] === 0),
    'Stale PointLine path queued work: ' + JSON.stringify(rejection));
    await raf();
    assert(await path.evaluate((node, markup) => node === window.lineCustomRetained && node.outerHTML === markup, markup),
      'Rejected PointLine changed the retained node');
    await page.mouse.move(2, 2);
    await call('mount', true);
    await flush();
    for (const renderer of renderers) {
      const state = await snapshot(renderer), points = probes(state);
      assert((await pointer(renderer, points.origin)).current?.id === state.sites[1].id &&
        !(await pointer(renderer, points.stroke)).current, renderer + ': nearest missed the raw anchor');
    }
    await call('stop');
    assert(await page.locator('body *').count() === baseline && !(await snapshot('svg')).active,
      'Custom PointLine leaked its stopped owner');
    assert(!errors.length, errors.join('\n'));
    return {passed:true, phases, typedKeys:true, exactInkAndClip:true, nearestOrigin:true, rejectedAtomic:true, pageErrors:errors};
  } finally {
    await page.evaluate(display => {
      window.hostFixture.line_custom_stop();
      document.querySelector('.panels').style.display = display;
      delete window.lineCustomRetained;
    }, previous);
    page.off('pageerror', onError);
  }
})
