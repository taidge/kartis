(async page => {
  const assert = (ok, message) => { if (!ok) throw Error(message); };
  const errors = [], listener = error => errors.push(error.message);
  page.on('pageerror', listener);
  const snapshot = renderer => page.evaluate(renderer => JSON.parse(window.hostFixture.sankey_styles_snapshot(renderer)), renderer);
  const flush = async () => {
    await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
    await page.evaluate(() => window.hostFixture.sankey_styles_flush());
  };
  const previous = await page.evaluate(() => { const panels = document.querySelector('.panels'); const old = panels.style.display; panels.style.display = 'none'; return old; });
  const baseline = await page.locator('body *').count(), reports = [];
  const pixels = () => page.evaluate(async () => {
    const live = document.querySelector('#sankey-style-svg svg'), svg = live.cloneNode(true);
    let language;
    for (let e = live; e; e = e.parentElement) {
      language = e.getAttributeNS('http://www.w3.org/XML/1998/namespace', 'lang') ?? e.getAttribute('xml:lang') ?? e.getAttribute('lang');
      if (language !== null) break;
    }
    svg.setAttribute('lang', language ?? document.documentElement.lang);
    svg.setAttribute('width', '640'); svg.setAttribute('height', '420');
    const image = new Image();
    image.src = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(new XMLSerializer().serializeToString(svg))}`;
    await image.decode();
    const canvas = document.querySelector('#sankey-style-canvas canvas'), reference = document.createElement('canvas');
    reference.width = canvas.width; reference.height = canvas.height;
    const ctx = reference.getContext('2d'); ctx.drawImage(image, 0, 0, reference.width, reference.height);
    const a = ctx.getImageData(0, 0, reference.width, reference.height).data;
    const b = canvas.getContext('2d').getImageData(0, 0, canvas.width, canvas.height).data;
    let sum = 0, ink = 0;
    for (let i = 0; i < a.length; i++) sum += Math.abs(a[i] - b[i]);
    for (let i = 0; i < b.length; i += 4) if (b[i] < 245 || b[i + 1] < 245 || b[i + 2] < 245) ink++;
    return { mean: sum / a.length, ink, backing: [canvas.width, canvas.height] };
  });
  try {
    for (let kind = 0; kind < 4; kind++) {
      await page.evaluate(kind => window.hostFixture.sankey_styles_mount(kind), kind); await flush();
      const initial = await pixels();
      assert(initial.mean < .5 && initial.ink > 3000, `Sankey${kind}: real SVG/Canvas pixels differ or no ink ${JSON.stringify(initial)}`);
      const before = await snapshot('svg');
      const geometry = await page.evaluate(state => {
        const svg = document.querySelector('#sankey-style-svg svg');
        const node = state.nodes.find(n => n.id === 'A'), link = state.links.find(l => l.id === 'ab');
        const nodeElement = [...svg.querySelectorAll('[data-id]')].find(e => e.dataset.id === node.markId);
        const linkElement = [...svg.querySelectorAll('[data-id]')].find(e => e.dataset.id === link.markId);
        const box = nodeElement.getBBox(), fill = linkElement.getAttribute('fill'), id = fill?.match(/^url\(#(.+)\)$/)?.[1];
        const gradient = id ? [...svg.querySelectorAll('linearGradient')].find(e => e.id === id) : null;
        const source = gradient ? [Number(gradient.getAttribute('x1')), Number(gradient.getAttribute('y1'))] : null;
        const target = gradient ? [Number(gradient.getAttribute('x2')), Number(gradient.getAttribute('y2'))] : null;
        const stops = gradient ? [...gradient.querySelectorAll('stop')].map(e => e.getAttribute('stop-color')) : [];
        const text = [...svg.querySelectorAll('text')].find(e => e.textContent === 'Source <safe>');
        return { box: [box.x, box.y, box.width, box.height], rounded: (nodeElement.getAttribute('d').match(/C/g) || []).length === 4,
          border: nodeElement.getAttribute('stroke-width'), opacity: linkElement.getAttribute('opacity'), source, target, stops,
          textSize: text.getAttribute('font-size'), rotation: text.getAttribute('transform'), safe: !svg.querySelector('safe') };
      }, before);
      const node = before.nodes.find(n => n.id === 'A'), link = before.links.find(l => l.id === 'ab');
      // SVG getBBox() exposes float32 geometry; at this viewport one ULP can
      // exceed 1e-5 logical pixels. The path's serialized coordinates remain f64.
      assert(geometry.box.every((v, i) => Math.abs(v - node.rect[i]) < .0001), `Sankey${kind}: node geometry lost solved bounds ${JSON.stringify({actual:geometry.box,expected:node.rect})}`);
      assert(geometry.rounded && Number(geometry.border) === 2 && Math.abs(Number(geometry.opacity) - .7) < 1e-8 && geometry.safe, `Sankey${kind}: actual rounded/border/opacity/safe text properties missing`);
      assert(geometry.source?.every((v, i) => Math.abs(v - link.sourcePort[i]) < .000001) && geometry.target?.every((v, i) => Math.abs(v - link.targetPort[i]) < .000001) && geometry.stops.length === 2 && geometry.stops[0] !== geometry.stops[1], `Sankey${kind}: source→target gradient is not backed by actual ports/stops`);
      assert(Number(geometry.textSize) === 15 && (kind % 2 === 0 ? !geometry.rotation : geometry.rotation?.includes('rotate(-90')), `Sankey${kind}: label orientation/style absent`);
      for (const n of before.nodes) {
        assert(Math.abs(n.rect[kind % 2 === 0 ? 2 : 3] - 16) < 1e-8, `Sankey${kind}: node thickness transposed incorrectly`);
        for (const incoming of [false, true]) {
          const flows = before.links.filter(l => incoming ? l.target === n.id : l.source === n.id), breadth = kind % 2 === 0 ? n.rect[3] : n.rect[2];
          if ((incoming ? n.inflow : n.outflow) === n.value) assert(Math.abs(flows.reduce((sum, l) => sum + l.width, 0) - breadth) < 1e-7, `Sankey${kind}: broken port flow conservation for ${n.id}`);
        }
      }
      const returns = before.links.filter(l => l.route.type === 'return');
      assert(kind < 2 ? returns.length === 0 : returns.length >= 2, `Sankey${kind}: wrong cyclic topology`);
      assert(returns.every(l => (kind % 2 === 0 ? ['Top', 'Bottom'] : ['Left', 'Right']).includes(l.route.side)), `Sankey${kind}: return side metadata ignores real orientation`);
      await page.evaluate(() => window.hostFixture.sankey_styles_phase(1)); await flush();
      const changed = await pixels(), current = await snapshot('svg');
      assert(changed.mean < .5, `Sankey${kind}: updated gradient/rounded pixels diverged ${changed.mean}`);
      assert(current.revision > before.revision && current.target.id === before.target.id && current.target.value !== before.target.value, `Sankey${kind}: update did not preserve stable current ID`);
      await page.evaluate(() => window.hostFixture.sankey_styles_hide_target(true)); await flush();
      const hidden = await snapshot('svg');
      assert(hidden.nodes.find(n => n.id === 'B').visible === false && hidden.links.every(l => l.visible === (l.source !== 'B' && l.target !== 'B')), `Sankey${kind}: explicit hidden target kept incident ink`);
      assert(hidden.links.every((l, i) => l.width === current.links[i].width && JSON.stringify(l.sourcePort) === JSON.stringify(current.links[i].sourcePort) && JSON.stringify(l.targetPort) === JSON.stringify(current.links[i].targetPort)), `Sankey${kind}: hidden target changed ports or flow widths`);
      assert(hidden.labels.every(l => l.series !== 'B') && !hidden.markIds.includes(current.nodes.find(n => n.id === 'B').markId), `Sankey${kind}: target-owned node/label remained visible`);
      const hiddenPixels = await pixels(); assert(hiddenPixels.mean < .5, `Sankey${kind}: hidden SVG/Canvas pixels differ`);
      await page.evaluate(() => window.hostFixture.sankey_styles_hide_target(false)); await flush();
      for (const renderer of ['svg', 'canvas']) {
        const state = await snapshot(renderer), surface = page.locator(`#sankey-style-${renderer} .kartis-surface`);
        await surface.scrollIntoViewIfNeeded(); const bounds = await surface.boundingBox();
        const x = bounds.x + state.target.x / state.width * bounds.width, y = bounds.y + state.target.y / state.height * bounds.height;
        await page.mouse.move(x, y); await flush();
        assert((await snapshot(renderer)).hovered.includes(state.target.id), `${renderer}/Sankey${kind}: actual current ribbon hit missed`);
        const tooltip = await page.locator(`#sankey-style-${renderer} .kartis-surface-tooltip`).textContent();
        assert(tooltip.includes(state.target.label) && !(await page.locator(`#sankey-style-${renderer} safe`).count()), `${renderer}/Sankey${kind}: current safe tooltip absent`);
        await page.mouse.click(x, y); await flush(); assert((await snapshot(renderer)).selected === state.target.id, `${renderer}/Sankey${kind}: pointer did not select current link`);
        const control = page.locator(`#sankey-style-${renderer} .kartis-datum`).filter({ hasText: state.target.label }).first(); await control.focus(); await page.keyboard.press('Enter'); await flush();
        assert((await snapshot(renderer)).events.some(e => e.type === 'activated' && e.id === state.target.id), `${renderer}/Sankey${kind}: keyboard activation missing`);
        await page.locator(`#sankey-style-${renderer} .kartis-legend-item`).filter({ hasText: 'Source <safe>' }).click(); await flush();
        assert((await snapshot(renderer)).hidden.includes('A'), `${renderer}/Sankey${kind}: legend did not hide exact source owner`);
        if (renderer === 'svg') {
          const visible = await page.evaluate(id => [...document.querySelectorAll('#sankey-style-svg svg [data-id]')].some(e => e.dataset.id === id), state.target.id);
          const texts = await page.locator('#sankey-style-svg svg text').allTextContents();
          assert(!visible && !texts.includes('Source <safe>') && texts.includes('B'), `Sankey${kind}: source-owned link/label filter hid other labels or retained ink`);
        }
      }
      await page.mouse.move(2, 2); await page.evaluate(() => document.activeElement?.blur()); await flush();
      const accepted = await snapshot('svg'), dom = await page.locator('#sankey-style-svg svg').innerHTML(), image = await page.evaluate(() => document.querySelector('#sankey-style-canvas canvas').toDataURL());
      assert(await page.evaluate(() => window.hostFixture.sankey_styles_rejected()), `Sankey${kind}: invalid candidate accepted`);
      assert((await snapshot('svg')).revision === accepted.revision, `Sankey${kind}: rejected candidate changed accepted revision`); await flush();
      assert(await page.locator('#sankey-style-svg svg').innerHTML() === dom && await page.evaluate(() => document.querySelector('#sankey-style-canvas canvas').toDataURL()) === image, `Sankey${kind}: rejected candidate changed SVG or Canvas frame`);
      await page.evaluate(() => { window.staleSankey = document.querySelector('#sankey-style-svg .kartis-surface'); window.hostFixture.sankey_styles_stop(); });
      assert(await page.locator('body *').count() === baseline, `Sankey${kind}: cleanup retained surface/controls`);
      await page.evaluate(() => window.staleSankey.dispatchEvent(new PointerEvent('pointermove', { clientX: 100, clientY: 100 })));
      assert(!(await snapshot('svg')).active, `Sankey${kind}: stale listener revived owner`);
      reports.push({ case: kind, initial, changed, hiddenPixels, geometry, flowConservation: true, metadata: true, currentHit: true, safeTooltip: true, stableUpdate: true, sourceLegend: true, hiddenTargetNoRelayout: true, keyboard: true, rejectedAtomic: true, cleanup: true });
    }
    assert(errors.length === 0, errors.join('\n'));
    return { passed: true, scenes: reports, pageErrors: errors };
  } finally {
    await page.evaluate(previous => { window.hostFixture.sankey_styles_stop(); document.querySelector('.panels').style.display = previous; delete window.staleSankey; }, previous);
    page.off('pageerror', listener);
  }
})
