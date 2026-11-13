(async function verifyTypography(page, url = 'http://127.0.0.1:8814') {
  const errors = [];
  const onError = error => errors.push(error.message);
  page.on('pageerror', onError);
  const assert = (ok, message) => { if (!ok) throw Error(message); };
  const call = (name, ...args) => page.evaluate(([name, args]) => window.hostFixture[name](...args), [name, args]);
  const snapshot = async () => JSON.parse(await call('typography_snapshot'));
  const settle = () => page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  await page.goto(url);
  await page.waitForFunction(() => window.hostReady === true);
  const panels = await page.evaluate(() => {
    const panels = document.querySelector('.panels');
    const display = panels.style.display;
    panels.style.display = 'none';
    return display;
  });
  await page.evaluate(async () => {
    const regular = window.hostFixture.typography_font_bytes(false);
    const bold = window.hostFixture.typography_font_bytes(true);
    for (const family of ['Kartis Fixture Typography', 'Chart "Quoted"']) {
      for (const [weight, bytes] of [[400, regular], [700, bold]]) {
        const face = new FontFace(family, bytes, {weight: String(weight)});
        document.fonts.add(await face.load());
      }
    }
    // These families expose one known face each, independently of CSS's
    // weight matching within the two-face family used by the chart.
    document.fonts.add(await new FontFace('Kartis Oracle Regular', regular, {weight: '400'}).load());
    document.fonts.add(await new FontFace('Kartis Oracle Bold', bold, {weight: '700'}).load());
    await document.fonts.ready;
  });
  const styles = () => page.evaluate(() => {
    const details = node => {
      const style = getComputedStyle(node);
      return {weight: Number(style.fontWeight), family: style.fontFamily};
    };
    const svgTexts = [...document.querySelectorAll('#typography-svg .kartis-surface svg text')];
    const semanticTexts = [...document.querySelectorAll('#typography-semantic .kartis-label')].map(node => node.firstElementChild);
    return {
      svg: svgTexts.map(node => ({...details(node), weightAttribute: node.getAttribute('font-weight'), familyAttribute: node.getAttribute('font-family')})),
      semantic: semanticTexts.map(details),
      legend: details(document.querySelector('#typography-semantic .kartis-legend-item')),
      tooltip: details(document.querySelector('#typography-semantic .kartis-tooltip')),
      parent: details(document.querySelector('#typography-svg')),
      retained: window.typographyNodes ? svgTexts.every((node, i) => node === window.typographyNodes[i]) : true,
      semanticRetained: window.typographySemanticNodes ? semanticTexts.every((node, i) => node === window.typographySemanticNodes[i]) : true,
      pathRetained: window.typographyPath ? document.querySelector('#typography-svg path[data-id="datum"]') === window.typographyPath : true,
      clip: Boolean(document.querySelector('#typography-svg svg svg[overflow="hidden"] path[data-id="datum"]')),
    };
  });
  const pixels = async () => {
    const svgScreenshot = (await page.locator('#typography-svg .kartis-surface').screenshot({scale: 'css'})).toString('base64');
    return page.evaluate(async svgScreenshot => {
    const surface = document.querySelector('#typography-canvas canvas');
    const image = new Image();
    image.src = 'data:image/png;base64,' + svgScreenshot;
    await image.decode();
    const make = () => { const c = document.createElement('canvas'); c.width = 360; c.height = 220; return c; };
    const svgCanvas = make(); svgCanvas.getContext('2d').drawImage(image, 0, 0);
    const canvas = make(); canvas.getContext('2d').drawImage(surface, 0, 0, 360, 220);
    const svgBytes = svgCanvas.getContext('2d').getImageData(0, 0, 360, 220).data;
    const canvasBytes = canvas.getContext('2d').getImageData(0, 0, 360, 220).data;
    const ink = (bytes, x, y) => {
      const offset = (y * 360 + x) * 4;
      return bytes[offset] < 80 && bytes[offset + 1] < 80 && bytes[offset + 2] < 80 && bytes[offset + 3] > 200;
    };
    const roi = bytes => {
      let count = 0, left = 360, right = -1, top = 220, bottom = -1;
      for (let y = 25; y < 95; y++) for (let x = 25; x < 180; x++) if (ink(bytes, x, y)) {
        count++; left = Math.min(left, x); right = Math.max(right, x); top = Math.min(top, y); bottom = Math.max(bottom, y);
      }
      return {count, bounds: [left, top, right + 1, bottom + 1]};
    };
    const outside = (a, b) => {
      let bad = 0;
      for (let y = 25; y < 95; y++) for (let x = 25; x < 180; x++) if (ink(a, x, y)) {
        let neighbor = false;
        for (let dy = -1; dy <= 1; dy++) for (let dx = -1; dx <= 1; dx++) neighbor ||= ink(b, x + dx, y + dy);
        if (!neighbor) bad++;
      }
      return bad;
    };
    const first = document.querySelector('#typography-svg .kartis-surface svg text');
    const font = `${first.getAttribute('font-weight')} ${first.getAttribute('font-size')}px ${first.getAttribute('font-family')}`;
    const native = make().getContext('2d'); native.font = font;
    const metrics = native.measureText('AB');
    const actualWeight = Number(first.getAttribute('font-weight'));
    const oracleWeight = actualWeight < 600 ? 400 : 700;
    const oracle = make().getContext('2d');
    oracle.font = `${oracleWeight} 50px "Kartis Oracle ${oracleWeight === 400 ? 'Regular' : 'Bold'}"`;
    return {svg: roi(svgBytes), canvas: roi(canvasBytes), outside: [outside(svgBytes, canvasBytes), outside(canvasBytes, svgBytes)],
      nativeAdvance: metrics.width, oracleAdvance: oracle.measureText('AB').width, oracleWeight, oracleFont: oracle.font,
      svgAdvance: first.getComputedTextLength(), nativeFont: native.font};
    }, svgScreenshot);
  };
  const assertPhase = async (phase, weight, expectedAdvance) => {
    const state = await snapshot();
    const style = await styles();
    assert(style.parent.weight === 900, 'Host font pollution setup missing');
    assert(style.svg[0].weight === weight && style.semantic[0].weight === weight, `Phase ${phase} lost shared weight: ${JSON.stringify(style)}`);
    assert(style.svg[1].weight === 400 && style.semantic[1].weight === 400, `Phase ${phase} per-label regular override lost`);
    assert(style.legend.weight === weight && style.tooltip.weight === 700, `Phase ${phase} legend/tooltip inheritance mismatch: ${JSON.stringify(style)}`);
    assert(style.retained && style.semanticRetained && style.pathRetained && style.clip, `Phase ${phase} font update replaced keyed nodes or dropped clip`);
    const pixel = await pixels();
    const measured = state.measurements.filter(row => row.text === 'AB' && row.size === 50).at(-1);
    assert(pixel.svg.count > 100 && pixel.canvas.count > 100, `Phase ${phase} did not paint real text`);
    assert(pixel.outside.every(count => count <= 4), `Phase ${phase} actual SVG/Canvas ink mismatch: ${JSON.stringify(pixel)}`);
    if (expectedAdvance !== null) {
      assert(Math.abs(pixel.nativeAdvance - pixel.oracleAdvance) < 0.0001 && Math.abs(pixel.svgAdvance - expectedAdvance) < 0.05,
        `Phase ${phase} selected the wrong actual font face: ${JSON.stringify(pixel)}`);
      assert(measured && measured.weight === weight && Math.abs(measured.width - pixel.oracleAdvance) < 0.0001,
        `Phase ${phase} label layout did not measure the painted font: ${JSON.stringify(measured)}`);
    }
    return {phase, weight, style, pixel, measurement: measured ?? null, measuredWidth: measured?.width ?? null,
      revision: state.charts.map(chart => chart.revision)};
  };
  const cases = [];
  try {
    await call('typography_mount'); await settle();
    cases.push(await assertPhase(0, 400, 60));
    await page.evaluate(() => {
      window.typographyNodes = [...document.querySelectorAll('#typography-svg .kartis-surface svg text')];
      window.typographySemanticNodes = [...document.querySelectorAll('#typography-semantic .kartis-label')].map(node => node.firstElementChild);
      window.typographyPath = document.querySelector('#typography-svg path[data-id="datum"]');
    });
    await call('typography_phase', 1); await settle();
    cases.push(await assertPhase(1, 700, 100));
    assert(cases[1].pixel.canvas.count > cases[0].pixel.canvas.count * 1.4, 'Bold face has no real geometry/ink change');
    const surface = page.locator('#typography-svg .kartis-surface');
    await surface.scrollIntoViewIfNeeded();
    const box = await surface.boundingBox();
    await page.mouse.move(box.x + 285 / 360 * box.width, box.y + 70 / 220 * box.height);
    await settle();
    await page.waitForFunction(() => document.querySelector('#typography-svg .kartis-surface-tooltip')?.textContent?.includes('AB'));
    const before = await snapshot();
    const accepted = await page.evaluate(() => ({
      svg: document.querySelector('#typography-svg .kartis-surface').innerHTML,
      semantic: document.querySelector('#typography-semantic').innerHTML,
      png: document.querySelector('#typography-canvas canvas').toDataURL(),
      tooltip: document.querySelector('#typography-svg .kartis-surface-tooltip').textContent,
    }));
    const rejected = JSON.parse(await call('typography_phase', 3)); await settle();
    const after = await snapshot();
    assert(rejected.browserRejected.length === 2 && rejected.browserRejected.every(Boolean) && rejected.errors.length === 1,
      `Invalid font was not rejected by all consumers: ${JSON.stringify(rejected)}`);
    assert(after.charts.every((chart, i) => chart.revision === before.charts[i].revision), 'Invalid font changed the retained revision');
    assert(await page.evaluate(accepted =>
      document.querySelector('#typography-svg .kartis-surface').innerHTML === accepted.svg &&
      document.querySelector('#typography-semantic').innerHTML === accepted.semantic &&
      document.querySelector('#typography-canvas canvas').toDataURL() === accepted.png &&
      document.querySelector('#typography-svg .kartis-surface-tooltip').textContent === accepted.tooltip, accepted),
    'Rejected font partially published markup, image or tooltip');
    await page.mouse.move(1, 1);
    for (const [phase, weight, advance] of [[2, 400, 60], [5, 500, 60], [6, 600, 100]]) {
      await call('typography_phase', phase); await settle();
      cases.push(await assertPhase(phase, weight, advance));
    }
    assert(cases[2].style.svg[0].familyAttribute === JSON.stringify('Chart "Quoted"'), 'Quoted face name was not CSS escaped');
    await call('typography_hidden', true); await settle();
    assert(await page.locator('#typography-svg .kartis-surface svg text').count() === 0, 'Owned SVG text survived hiding');
    assert(await page.locator('#typography-semantic .kartis-label').evaluateAll(nodes => nodes.every(node => getComputedStyle(node).display === 'none')), 'Owned semantic text survived hiding');
    await call('typography_hidden', false); await settle();
    // Hiding filters the public scene's label slots; restore may create nodes.
    // Font-only changes were already checked against retained node identities.
    await page.evaluate(() => { window.typographyNodes = null; window.typographyPath = null; });
    await call('typography_phase', 4); await settle();
    cases.push(await assertPhase(4, 400, null));
    assert(cases.at(-1).style.svg[0].familyAttribute === 'sans-serif', 'Reset retained a prior explicit family');
    const cleanup = JSON.parse(await call('typography_stop'));
    assert(cleanup.nodes === 0 && cleanup.handlers === 0, `Semantic owner cleanup leaked: ${JSON.stringify(cleanup)}`);
    assert(await page.locator('[id^="typography-"]').count() === 0, 'Typography roots leaked');
    await call('typography_mount'); await settle(); await call('typography_stop');
    assert(await page.locator('[id^="typography-"]').count() === 0, 'Typography remount leaked');
    assert(errors.length === 0, errors.join('\n'));
    return {passed: true, realFontFaces: true, actualSvgCanvasInk: true, fontAwareMeasurements: true, hostWeightReset: true,
      fontOnlyRetained: true, quotedFamily: true, cssWeightMatching: true, invalidAtomic: true, hideRestore: true, cleanup: true, remount: true, cases, pageErrors: errors};
  } finally {
    await call('typography_stop');
    await page.evaluate(display => { document.querySelector('.panels').style.display = display; }, panels);
    page.off('pageerror', onError);
  }
})
