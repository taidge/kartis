(async function verifyRadarCustomPoints(page) {
  const assert = (condition, message) => { if (!condition) throw Error(message); };
  const errors = [], onError = error => errors.push(error.message);
  page.on('pageerror', onError);
  const previous = await page.evaluate(() => {
    const panel = document.querySelector('.panels');
    const display = panel.style.display;
    panel.style.display = 'none';
    return display;
  });
  const baseline = await page.locator('body *').count();
  const renderers = ['svg', 'canvas'];
  const call = (method, ...args) => page.evaluate(([method, args]) =>
    window.hostFixture[`radar_custom_${method}`](...args), [method, args]);
  const snapshot = async renderer => JSON.parse(await call('snapshot', renderer));
  const flush = async () => {
    await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
    await call('flush');
  };
  const samePoint = (a, b) => a?.length === 2 && b.every((v, i) => Math.abs(a[i] - v) < 1e-8);
  const pointer = async (renderer, [x, y], click = false) => {
    const surface = page.locator(`#radar-custom-${renderer} .kartis-surface`);
    await surface.scrollIntoViewIfNeeded();
    const box = await surface.boundingBox(), state = await snapshot(renderer);
    const position = [box.x + x * box.width / state.width, box.y + y * box.height / state.height];
    if (click) await page.mouse.click(...position);
    else await page.mouse.move(...position);
    await flush();
    return snapshot(renderer);
  };
  const probes = state => {
    const [x, y] = state.target.position;
    const [eastX, eastY] = state.east.position;
    return {
      origin: [x, y], stroke: [x + 22, y], hollow: [x + 34, y],
      inside: [eastX - 3, eastY], clipped: [eastX + 5, eastY],
    };
  };
  const pixels = ({width, height}, points) => page.evaluate(async ({width, height, points}) => {
    const svg = document.querySelector('#radar-custom-svg svg').cloneNode(true);
    const canvas = document.querySelector('#radar-custom-canvas canvas');
    svg.setAttribute('width', String(canvas.width));
    svg.setAttribute('height', String(canvas.height));
    const image = new Image();
    image.src = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(new XMLSerializer().serializeToString(svg))}`;
    await image.decode();
    const copy = document.createElement('canvas');
    copy.width = canvas.width;
    copy.height = canvas.height;
    copy.getContext('2d').drawImage(image, 0, 0);
    const a = copy.getContext('2d').getImageData(0, 0, copy.width, copy.height).data;
    const b = canvas.getContext('2d').getImageData(0, 0, canvas.width, canvas.height).data;
    let sum = 0;
    for (let i = 0; i < a.length; i++) sum += Math.abs(a[i] - b[i]);
    const rgba = (data, [x, y]) => {
      const offset = 4 * (Math.floor(y * canvas.height / height) * canvas.width + Math.floor(x * canvas.width / width));
      return [...data.slice(offset, offset + 4)];
    };
    return {
      mean: sum / a.length,
      samples: Object.fromEntries(Object.entries(points).map(([name, point]) =>
        [name, {svg: rgba(a, point), canvas: rgba(b, point)}])),
    };
  }, {width, height, points});
  try {
    await page.mouse.move(2, 2);
    await call('mount', 0);
    await flush();
    const first = await snapshot('svg');
    const retained = page.locator(`#radar-custom-svg [data-id="${first.target.id}"]`);
    await retained.evaluate(node => { window.radarCustomRetained = node; });
    const phases = [];
    for (const phase of [0, 1]) {
      if (phase) {
        await page.mouse.move(2, 2);
        await call('phase', phase);
        await flush();
      }
      const raw = 5 + 2 * phase;
      const state = await snapshot('svg');
      const points = probes(state);
      const proof = await pixels(state, points);
      assert(proof.mean < .5, `Custom Radar renderer parity ${JSON.stringify(proof)}`);
      for (const [name, pair] of Object.entries(proof.samples)) {
        const expected = name === 'stroke' || name === 'inside' ? '190,24,76,255' : '255,255,255,255';
        assert(pair.svg.join() === expected && pair.canvas.join() === expected,
          `Custom Radar sample ${name}: ${JSON.stringify(pair)}`);
      }
      assert(samePoint(state.target.position, [160, 120 - 10 * raw]) &&
        samePoint(state.east.position, [260, 120]) && state.target.id === first.target.id,
        'Radar phase changed datum identity or mapped position');
      for (const renderer of renderers) {
        assert(!(await pointer(renderer, points.origin)).current && !(await pointer(renderer, points.hollow)).current,
          `${renderer}: hollow custom Radar dot has false exact ink`);
        const hit = await pointer(renderer, points.stroke, true);
        assert(hit.selected === state.target.id && hit.current?.rows[0].raw === raw &&
          samePoint(hit.current.anchor, state.target.position), `${renderer}: wrong custom Radar exact value or anchor`);
        assert((await pointer(renderer, points.inside)).hovered.includes(state.east.id) &&
          !(await pointer(renderer, points.clipped)).current, `${renderer}: custom Radar plot clipping failed`);
      }
      assert(await retained.evaluate((node, raw) => node === window.radarCustomRetained &&
        node.getAttribute('data-value') === String(raw) && node.getAttribute('fill') === 'none' &&
        node.getAttribute('stroke-width') === '4' && node.parentElement.getAttribute('viewBox') === '60 20 200 200', raw),
      'Custom Radar path identity, raw value, stroke, or clip changed');
      phases.push({phase, ...proof});
    }
    const markup = await retained.evaluate(node => node.outerHTML);
    const rejection = await page.evaluate(() => {
      const status = () => ['svg', 'canvas'].map(renderer => {
        const {revision, pending} = JSON.parse(window.hostFixture.radar_custom_snapshot(renderer));
        return [revision, pending];
      });
      const before = status();
      const rejected = window.hostFixture.radar_custom_rejected();
      return {before, rejected, after: status()};
    });
    assert(rejection.rejected && rejection.before.every(([revision, pending], i) =>
      pending === 0 && rejection.after[i][0] === revision && rejection.after[i][1] === 0),
    `Rejected Radar frame queued work ${JSON.stringify(rejection)}`);
    // An explicit flush paints even with an empty queue. Let native RAF run
    // before checking that the rejected frame never changed the retained path.
    await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
    assert(await retained.evaluate((node, markup) => node === window.radarCustomRetained && node.outerHTML === markup, markup),
      'Rejected Radar changed the retained path');

    await page.mouse.move(2, 2);
    await call('mount', 1);
    await flush();
    for (const renderer of renderers) {
      const state = await snapshot(renderer), points = probes(state);
      assert((await pointer(renderer, points.origin)).hovered.includes(state.target.id) &&
        !(await pointer(renderer, points.stroke)).current, `${renderer}: Nearest lost the logical Radar anchor`);
    }

    await page.mouse.move(2, 2);
    await call('mount', 2);
    await call('phase', 1);
    await flush();
    for (const renderer of renderers) {
      const state = await snapshot(renderer);
      const probe = [state.center[0], state.center[1] - 30];
      const current = (await pointer(renderer, probe)).current;
      assert(current?.slice === state.northSlice && current.rows.map(row => row.raw).join() === '7,2',
        `${renderer}: wrong Radar angular slice or current member order`);
      await page.mouse.wheel(0, -110);
      await flush();
      const [scale, x, y] = (await snapshot(renderer)).view;
      const projected = [60 + x + scale * (probe[0] - 60), 20 + y + scale * (probe[1] - 20)];
      assert(scale > 1 && (await pointer(renderer, projected)).current?.slice === state.northSlice,
        `${renderer}: Radar camera did not preserve the angular slice`);
    }
    await call('stop');
    assert(await page.locator('body *').count() === baseline && !(await snapshot('svg')).active,
      'Custom Radar leaked a stopped owner');
    assert(!errors.length, errors.join('\n'));
    return {passed:true, phases, nearestOrigin:true, exactInkAndClip:true, polarCamera:true, rejectedAtomic:true, rejection, pageErrors:errors};
  } finally {
    await page.evaluate(display => {
      window.hostFixture.radar_custom_stop();
      document.querySelector('.panels').style.display = display;
      delete window.radarCustomRetained;
    }, previous);
    page.off('pageerror', onError);
  }
})
