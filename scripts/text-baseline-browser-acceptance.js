(async function verifyTextBaseline(page, url = 'http://127.0.0.1:8814') {
  async function innerCore(page, url) {
  const errors = [], assert = (ok, detail) => { if (!ok) throw Error(detail); };
  const onError = error => errors.push(error.message); page.on('pageerror', onError);
  const call = (name, ...args) => page.evaluate(([name, args]) => window.hostFixture[name](...args), [name, args]);
  const state = async () => JSON.parse(await call('text_baseline_snapshot'));
  const settle = () => page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  await page.goto(url); await page.waitForFunction(() => window.hostReady === true);
  const panelDisplay = await page.evaluate(() => { const p = document.querySelector('.panels'); const old = p.style.display; p.style.display = 'none'; return old; });
  await page.evaluate(async () => {
    for (const [family, weight, kind] of [['Kartis Fixture Typography', 400, 0], ['Kartis Fixture Typography', 700, 1], ['Kartis Baseline ABeeZee', 400, 2]]) {
      document.fonts.add(await new FontFace(family, window.hostFixture.text_baseline_font(kind), {weight: String(weight)}).load());
    }
    await document.fonts.ready;
  });
  const geometry = () => page.evaluate(() => {
    const label = document.querySelector('#text-baseline-semantic .kartis-label');
    const text = label.firstElementChild; const css = getComputedStyle(text); const rect = text.getBoundingClientRect();
    return {tag: text.tagName, text: text.textContent, left: parseFloat(label.style.left), top: parseFloat(label.style.top),
      width: rect.width, height: rect.height, weight: Number(css.fontWeight), family: css.fontFamily, lineHeight: parseFloat(css.lineHeight),
      retained: !window.baselineText || text === window.baselineText,
      graphicCount: document.querySelectorAll('#text-baseline-semantic .kartis-label svg, #text-baseline-semantic .kartis-outline-glyph, #text-baseline-semantic img').length,
      probeCount: document.querySelectorAll('.kartis-semantic-text-probe').length};
  });
  const pixels = async () => {
    // Compare the renderers on the same pixel grid. A fractional document
    // origin changes screenshot downsampling relative to a raw Canvas buffer.
    // This moves only the whole fixture frame, never the label's coordinates.
    await page.evaluate(() => {
      for (const [rootId, selector] of [['text-baseline-semantic', '.kartis-plot'], ['text-baseline-svg', '.kartis-surface']]) {
        const root = document.getElementById(rootId), r = root.querySelector(selector).getBoundingClientRect();
        root.style.top = `${(parseFloat(root.style.top) || 0) + Math.round(r.top + scrollY) - r.top - scrollY}px`;
        root.style.left = `${(parseFloat(root.style.left) || 0) + Math.round(r.left + scrollX) - r.left - scrollX}px`;
      }
    });
    const capture = async selector => {
      const target = page.locator(selector); await target.scrollIntoViewIfNeeded();
      await page.evaluate(() => window.scrollTo(Math.round(scrollX), Math.round(scrollY)));
      return (await target.screenshot({scale: 'css'})).toString('base64');
    };
    const semantic = await capture('#text-baseline-semantic .kartis-plot');
    const svg = await capture('#text-baseline-svg .kartis-surface');
    return page.evaluate(async ({semantic, svg}) => {
      const size = 360;
      const canvas = () => { const c = document.createElement('canvas'); c.width = c.height = size; const ctx = c.getContext('2d'); ctx.fillStyle = 'white'; ctx.fillRect(0, 0, size, size); return c; };
      const decode = async src => { const c = canvas(); const img = new Image(); img.src = 'data:image/png;base64,' + src; await img.decode(); c.getContext('2d').drawImage(img, 0, 0); return c; };
      const semanticCanvas = await decode(semantic), svgCanvas = await decode(svg);
      const browserCanvas = canvas(); browserCanvas.getContext('2d').drawImage(document.querySelector('#text-baseline-canvas canvas'), 0, 0, size, size);
      const ratio1 = document.createElement('canvas'), ratio2 = document.createElement('canvas');
      window.hostFixture.text_baseline_draw(ratio1, 1); window.hostFixture.text_baseline_draw(ratio2, 2);
      const downsample = source => { const c = canvas(); c.getContext('2d').drawImage(source, 0, 0, size, size); return c; };
      const arrays = Object.fromEntries(Object.entries({semantic: semanticCanvas, svg: svgCanvas, canvas: browserCanvas, ratio1: downsample(ratio1), ratio2: downsample(ratio2)})
        .map(([name, c]) => [name, c.getContext('2d').getImageData(0, 0, size, size).data]));
      const ink = (a, x, y) => { const i = (y * size + x) * 4; return a[i] < 90 && a[i + 1] < 90 && a[i + 2] < 90 && a[i + 3] > 200; };
      const stats = a => { let count = 0, left = size, top = size, right = -1, bottom = -1;
        for (let y = 0; y < size; y++) for (let x = 0; x < size; x++) if (ink(a, x, y)) { count++; left = Math.min(left, x); top = Math.min(top, y); right = Math.max(right, x); bottom = Math.max(bottom, y); }
        return {count, bounds: [left, top, right + 1, bottom + 1]}; };
      const outside = (a, b) => { let bad = 0;
        for (let y = 1; y < size - 1; y++) for (let x = 1; x < size - 1; x++) if (ink(a, x, y)) {
          let near = false; for (let dy = -1; dy <= 1; dy++) for (let dx = -1; dx <= 1; dx++) near ||= ink(b, x + dx, y + dy); if (!near) bad++;
        } return bad; };
      return {stats: Object.fromEntries(Object.entries(arrays).map(([name, a]) => [name, stats(a)])),
        outside: Object.fromEntries(['svg', 'canvas', 'ratio1', 'ratio2'].map(name => [name, [outside(arrays.semantic, arrays[name]), outside(arrays[name], arrays.semantic)]])),
        backingStores: [[ratio1.width, ratio1.height], [ratio2.width, ratio2.height]], dpr: devicePixelRatio,
        origins: Object.fromEntries(['#text-baseline-semantic .kartis-plot', '#text-baseline-svg .kartis-surface'].map(selector => { const r = document.querySelector(selector).getBoundingClientRect(); return [selector, [r.x + scrollX, r.y + scrollY, r.width, r.height]]; }))};
    }, {semantic, svg});
  };
  const verify = async (kind, anchor, shifted = false) => {
    const snapshot = await state(), measured = snapshot.measurement, ordinary = await geometry();
    assert(measured.ink !== null, `Nonempty text lost its actual ink bounds: ${JSON.stringify(measured)}`);
    assert(ordinary.text === (kind >= 3 ? (kind === 3 ? 'ég' : 'Aég') : 'B') && ordinary.graphicCount === 0,
      `Ordinary Text was replaced by glyph/image content: ${JSON.stringify(ordinary)}`);
    assert(ordinary.probeCount === 1, `Measurement resource is not owned: ${JSON.stringify(ordinary)}`);
    const expectedLeft = 180 - measured.advance * [0, .5, 1][anchor];
    assert(Math.abs(ordinary.left - expectedLeft) < .05, `Anchor did not use measured advance: ${JSON.stringify({ordinary, measured, anchor})}`);
    if (!shifted) assert(Math.abs(ordinary.top + measured.baseline - 140) < .05, `Alphabetic baseline misplaced: ${JSON.stringify({ordinary, measured})}`);
    assert(Math.abs(ordinary.lineHeight - measured.requestedLineHeight) < .05, 'DOM-quantized line height was fed back into CSS');
    const pixel = await pixels();
    assert(Object.values(pixel.stats).every(row => row.count > 20), `Missing actual text ink: ${JSON.stringify(pixel)}`);
    assert(Object.values(pixel.outside).every(pair => pair.every(count => count <= 6)), `Actual ordinary Text/SVG/Canvas ink diverges: ${JSON.stringify({kind, anchor, ordinary, measured, pixel})}`);
    assert(pixel.backingStores[0].join() === '360,360' && pixel.backingStores[1].join() === '720,720', 'DPR drawing did not change backing pixels');
    const baseline = ordinary.top + measured.baseline;
    if (measured.ink) {
      const [x, y, w, h] = measured.ink, b = pixel.stats.semantic.bounds;
      assert(b[0] >= ordinary.left + x - 1 && b[1] >= baseline + y - 1 && b[2] <= ordinary.left + x + w + 1 && b[3] <= baseline + y + h + 1,
        `Public ink bounds exclude real HTML text: ${JSON.stringify({ordinary, measured, pixel})}`);
    }
    return {kind, anchor, ordinary, measurement: measured, pixel};
  };
  const verifyScaledContext = async () => {
    await call('text_baseline_mount', 0, 0, false); await settle();
    const baseline = JSON.parse(await call('text_baseline_probe', 'B', 1)).measurement;
    await page.evaluate(() => {
      for (const id of ['text-baseline-semantic', 'text-baseline-svg']) {
        const root = document.getElementById(id), wrapper = document.createElement('div');
        wrapper.className = 'text-baseline-scale-wrapper'; wrapper.style.cssText = 'position:relative;width:max-content';
        root.before(wrapper); wrapper.append(root); root.style.transformOrigin = '0 0';
      }
    });
    const scaledPixels = async (sx, sy) => {
      await page.evaluate(() => {
        for (const [rootId, selector] of [['text-baseline-semantic', '.kartis-plot'], ['text-baseline-svg', '.kartis-surface']]) {
          const root = document.getElementById(rootId), wrapper = root.parentElement; wrapper.style.transform = 'none';
          const r = root.querySelector(selector).getBoundingClientRect();
          wrapper.style.transform = `translate(${Math.round(r.x + scrollX) - r.x - scrollX}px,${Math.round(r.y + scrollY) - r.y - scrollY}px)`;
        }
      });
      const capture = async selector => {
        const target = page.locator(selector); await target.scrollIntoViewIfNeeded();
        await page.evaluate(() => window.scrollTo(Math.round(scrollX), Math.round(scrollY)));
        return (await target.screenshot({scale: 'css'})).toString('base64');
      };
      const semantic = await capture('#text-baseline-semantic .kartis-plot'), svg = await capture('#text-baseline-svg .kartis-surface');
      return page.evaluate(async ({semantic, svg, sx, sy}) => {
        const width = Math.round(360 * sx), height = Math.round(360 * sy);
        const decode = async data => { const c = document.createElement('canvas'); c.width = width; c.height = height;
          const x = c.getContext('2d'); x.fillStyle = 'white'; x.fillRect(0,0,width,height);
          const img = new Image(); img.src = 'data:image/png;base64,'+data; await img.decode();
          if (img.width !== width || img.height !== height) throw Error(`Scaled screenshot frame mismatch ${img.width}x${img.height} / ${width}x${height}`);
          x.drawImage(img,0,0); return x.getImageData(0,0,width,height).data; };
        const ordinary = await decode(semantic), reference = await decode(svg);
        const ink = (a,x,y) => { const i=(y*width+x)*4; return a[i]<90&&a[i+1]<90&&a[i+2]<90&&a[i+3]>200; };
        const stats = a => { let count=0,left=width,top=height,right=-1,bottom=-1;
          for(let y=0;y<height;y++)for(let x=0;x<width;x++)if(ink(a,x,y)){count++;left=Math.min(left,x);top=Math.min(top,y);right=Math.max(right,x);bottom=Math.max(bottom,y);}
          return{count,bounds:[left,top,right+1,bottom+1]}; };
        const outside = (a,b) => {let bad=0;for(let y=1;y<height-1;y++)for(let x=1;x<width-1;x++)if(ink(a,x,y)){
          let near=false;for(let dy=-1;dy<=1;dy++)for(let dx=-1;dx<=1;dx++)near ||= ink(b,x+dx,y+dy);if(!near)bad++;}return bad;};
        const rect=document.querySelector('#text-baseline-semantic .kartis-plot').getBoundingClientRect();
        const text=document.querySelector('#text-baseline-semantic .kartis-label').firstElementChild;
        const line=text.getBoundingClientRect();const node=text.parentElement;
        return {semantic:stats(ordinary),svg:stats(reference),outside:[outside(ordinary,reference),outside(reference,ordinary)],frame:[width,height],
          origin:[rect.x+scrollX,rect.y+scrollY],lineTop:line.top-rect.top,logicalTop:parseFloat(node.style.top),logicalWidth:parseFloat(node.style.width),
          screenLineHeight:line.height,plainTag:text.tagName,graphics:node.querySelectorAll('svg,img').length};
      },{semantic,svg,sx,sy});
    };
    const cases=[]; const rejected=[];
    try {
      for (const [index, spec] of [
        {name:'scale2',transform:'scale(2)',sx:2,sy:2},
        {name:'nonuniform',transform:'scale(1.25,1.5)',sx:1.25,sy:1.5},
        {name:'translatedScale',transform:'translate(17px,23px) scale(2)',sx:2,sy:2},
        {name:'zoom',transform:'none',zoom:1.25,sx:1.25,sy:1.25},
      ].entries()) {
        await page.evaluate(spec => {
          for(const id of ['text-baseline-semantic','text-baseline-svg']){
            const root=document.getElementById(id);root.style.transform=spec.transform;root.style.zoom=String(spec.zoom??1);
            // CSS transforms keep their unscaled flow height. Reserve the full
            // visual frame so later fixtures cannot paint over the screenshot.
            root.parentElement.style.height=`${384*spec.sy}px`;
          }
        },spec);
        await call('text_baseline_revision',BigInt(20+index),false);await settle();
        const measurement=JSON.parse(await call('text_baseline_probe','B',1)).measurement;
        // Transform is applied after layout; CSS zoom may change the host's
        // font rounding. Its actual baseline is checked against the independent
        // SVG alphabetic anchor below, rather than replaced by an unzoomed guess.
        assert(Math.abs(measurement.advance-baseline.advance)<.05&&Math.abs(measurement.lineHeight-baseline.lineHeight)<.05
          &&Number.isFinite(measurement.baseline)&&Math.abs(measurement.baseline)<=1_000_000
          &&(spec.name==='zoom'||Math.abs(measurement.baseline-baseline.baseline)<.05),
          `Scaled probe returned screen-space metrics: ${JSON.stringify({spec,baseline,measurement})}`);
        const pixel=await scaledPixels(spec.sx,spec.sy);
        assert(pixel.plainTag==='SPAN'&&pixel.graphics===0,'Scaled ordinary Text was replaced by a graphic');
        assert(Math.abs(pixel.logicalTop+measurement.baseline-140)<.05&&Math.abs(pixel.lineTop+measurement.baseline*spec.sy-140*spec.sy)<.1,
          `Scaled ordinary Text baseline is not y*scale: ${JSON.stringify({spec,measurement,pixel})}`);
        assert(Math.abs(pixel.screenLineHeight-measurement.lineHeight*spec.sy)<.05&&Math.abs(pixel.logicalWidth-measurement.advance)<.05,
          `Scaled ordinary Text has doubled local dimensions: ${JSON.stringify({spec,measurement,pixel})}`);
        assert(pixel.outside.every(n=>n<=6)&&pixel.semantic.count>20,`Scaled ordinary Text/SVG ink mismatch: ${JSON.stringify({spec,measurement,pixel})}`);
        cases.push({spec,measurement,pixel});
      }
      for(const spec of [{name:'rotate',transform:'rotate(12deg)'},{name:'skew',transform:'skewX(10deg)'},{name:'reflect',transform:'scale(-1,1)'},
        {name:'collapsed',transform:'scale(0)'},{name:'3d',transform:'rotateY(15deg)'},{name:'perspective',perspective:'500px'},
        {name:'rotateLonghand',rotate:'12deg'},{name:'scaleLonghandReflect',scale:'-1 1'}]) {
        const result=await page.evaluate(spec=>{
          const root=document.getElementById('text-baseline-semantic');root.style.transform=spec.transform??'none';root.style.zoom='1';root.style.rotate=spec.rotate??'none';root.style.scale=spec.scale??'none';root.style.perspective=spec.perspective??'none';
          try{window.hostFixture.text_baseline_probe('B',1);return {name:spec.name,rejected:false};}catch(error){return{name:spec.name,rejected:true,error:String(error)};}
        },spec);assert(result.rejected,`Unsupported transformed ancestry silently accepted: ${JSON.stringify(spec)}`);rejected.push(result);
      }
      const shadow=await page.evaluate(()=>{
        const rows=[];
        for(const slotted of [false,true]){
          const host=document.createElement('div');host.style.cssText='position:relative;transform:scale(1.25,1.5);transform-origin:0 0';document.body.append(host);
          const root=host.attachShadow({mode:'open'}),wrapper=document.createElement('div');wrapper.style.transform='translate(17px,23px)';root.append(wrapper);
          const parent=document.createElement('div');parent.style.position='relative';
          if(slotted){const slot=document.createElement('slot');slot.name='metric';wrapper.append(slot);parent.slot='metric';host.append(parent);}else wrapper.append(parent);
          const measurement=JSON.parse(window.hostFixture.text_baseline_measure_in(parent,'B'));
          wrapper.style.transform='rotate(12deg)';let bad=false;try{window.hostFixture.text_baseline_measure_in(parent,'B');}catch{bad=true;}
          rows.push({slotted,measurement,rotatedAncestorRejected:bad,remainingProbes:root.querySelectorAll('.kartis-semantic-text-probe').length+host.querySelectorAll('.kartis-semantic-text-probe').length});host.remove();
        }return rows;
      });
      for(const row of shadow)assert(Math.abs(row.measurement.advance-baseline.advance)<.05&&Math.abs(row.measurement.lineHeight-baseline.lineHeight)<.05&&Math.abs(row.measurement.baseline-baseline.baseline)<.05&&row.rotatedAncestorRejected&&row.remainingProbes===0,
        `Composed shadow ancestry measurement failed: ${JSON.stringify(row)}`);
      return {passed:true,baseline,cases,rejected,shadow};
    } finally {
      await call('text_baseline_stop');await page.evaluate(()=>{for(const node of document.querySelectorAll('.text-baseline-scale-wrapper'))node.remove();});
    }
  };
  const cases = []; let pendingRoute = null;
  try {
    await call('text_baseline_mount', 0, 0, false); await settle();
    for (const kind of [0, 1, 2, 3, 4]) for (const anchor of [0, 1, 2]) {
      await call('text_baseline_phase', kind, anchor); await settle(); cases.push(await verify(kind, anchor));
    }
    assert(Math.abs(cases[0].measurement.baseline - cases[6].measurement.baseline) > 2,
      'Different ascender/descender fonts did not yield distinct HTML baselines');
    await call('text_baseline_phase', 0, 0); await settle();
    await page.evaluate(() => { window.baselineText = document.querySelector('#text-baseline-semantic .kartis-label').firstElementChild; });
    await call('text_baseline_phase', 1, 0); await settle();
    const fontOnly = await verify(1, 0); assert(fontOnly.ordinary.retained, 'Font-only update replaced ordinary Text');
    const correctTop = await page.locator('#text-baseline-semantic .kartis-label').evaluate(node => node.style.top);
    await page.locator('#text-baseline-semantic .kartis-label').evaluate(node => { node.style.top = `${parseFloat(node.style.top) + 8}px`; });
    const baselineNegativeControl = await pixels();
    await page.locator('#text-baseline-semantic .kartis-label').evaluate((node, top) => { node.style.top = top; }, correctTop);
    assert(baselineNegativeControl.outside.svg.some(count => count > 6), 'Pixel test did not detect an intentionally displaced ordinary Text baseline');
    const acceptedHtml = await page.locator('#text-baseline-semantic .kartis-chart').innerHTML();
    const accepted = await state();
    await call('text_baseline_revision', 1n, true); await settle();
    const rejected = await state();
    assert(rejected.errors.length === accepted.errors.length + 1 && rejected.measurement.revision === accepted.measurement.revision,
      'Stale resource revision was accepted');
    assert(await page.locator('#text-baseline-semantic .kartis-chart').innerHTML() === acceptedHtml, 'Rejected baseline measurement partially changed ordinary Text');
    await call('text_baseline_revision', 1n, false); await settle();
    assert((await state()).measurement.revision === 1, 'Same resource revision retry did not publish');
    const pendingFont = await page.evaluate(() => Array.from(window.hostFixture.text_baseline_font(0)));
    let routeReady; const routeObserved = new Promise(resolve => { routeReady = resolve; });
    await page.route('**/text-baseline-pending.ttf', route => { pendingRoute = route; routeReady(); });
    await page.evaluate(() => {
      const face = new FontFace('Kartis Pending Baseline', 'url(/text-baseline-pending.ttf)'); document.fonts.add(face);
      window.baselinePendingFont = face.load(); window.baselinePendingFont.catch(() => {});
    });
    await routeObserved;
    await page.waitForFunction(() => document.fonts.status === 'loading');
    const beforeLoading = await page.locator('#text-baseline-semantic .kartis-chart').innerHTML();
    await call('text_baseline_revision', 2n, false); await settle();
    assert((await state()).measurement.revision === 1 && await page.locator('#text-baseline-semantic .kartis-chart').innerHTML() === beforeLoading,
      'Unready font resources published fallback baseline metrics');
    await pendingRoute.fulfill({contentType: 'font/ttf', body: Buffer.from(pendingFont)}); pendingRoute = null;
    await page.evaluate(async () => { await window.baselinePendingFont; await document.fonts.ready; });
    await call('text_baseline_revision', 2n, false); await settle();
    assert((await state()).measurement.revision === 2, 'Ready resource retry did not recover');
    await page.unroute('**/text-baseline-pending.ttf');
    const probe = JSON.parse(await call('text_baseline_probe', '  B   ', 1.4));
    assert(probe.overBudgetRejected && Math.abs(probe.measurement.advance - fontOnly.measurement.advance) < .05, 'Probe did not apply actual HTML whitespace or reject batch budget');
    const emptyCases = [];
    for (const raw of ['', '   ', '\t\r\n']) {
      const empty = JSON.parse(await call('text_baseline_probe', raw, 1));
      assert(empty.measurement.advance === 0 && empty.measurement.lineHeight === 0 && empty.measurement.baseline === 0 && empty.measurement.ink === null,
        `Baseline marker fabricated an empty Text line: ${JSON.stringify(empty)}`);
      emptyCases.push(empty);
    }
    await page.locator('#text-baseline-semantic').evaluate(node => { node.style.display = 'none'; });
    let detachedLayoutRejected = false;
    try { await call('text_baseline_probe', 'B', 1); } catch { detachedLayoutRejected = true; }
    await page.locator('#text-baseline-semantic').evaluate(node => { node.style.display = ''; });
    assert(detachedLayoutRejected, 'Nonempty text in a non-layout parent was accepted as zero metrics');
    const unsafe = JSON.parse(await call('text_baseline_probe', '<img src=x onerror=alert(1)>', 1.4));
    assert(unsafe.overBudgetRejected && await page.locator('.kartis-semantic-text-probe img').count() === 0, 'Text measurement interpreted markup');
    await call('text_baseline_hidden', true); await settle();
    assert(await page.locator('#text-baseline-semantic .kartis-label').evaluate(node => getComputedStyle(node).display === 'none'), 'Hidden ordinary text still paints');
    await call('text_baseline_hidden', false); await settle();
    await verify(1, 0);
    await call('text_baseline_stop'); await page.evaluate(() => { window.baselineText = null; });
    await call('text_baseline_mount', 0, 0, true); await settle();
    const moved = await geometry(), current = await state(); const dy = moved.top + current.measurement.baseline - 140;
    assert(Math.abs(dy) > 20, `Measured collision did not move the blocked label: ${JSON.stringify({moved, current, dy})}`);
    await call('text_baseline_reference_shift', dy); await settle();
    const collision = await verify(0, 0, true);
    const bounds = collision.pixel.stats.semantic.bounds;
    assert(bounds[3] <= 40 || bounds[1] >= 150, `Collision label still overlaps the obstacle: ${JSON.stringify(collision)}`);
    const cleanup = JSON.parse(await call('text_baseline_stop'));
    assert(cleanup.nodes === 0 && cleanup.handlers === 0 && cleanup.measurerReleased, `Ordinary Text owner cleanup leaked: ${JSON.stringify(cleanup)}`);
    assert(await page.locator('.kartis-semantic-text-probe, [id^="text-baseline-"]').count() === 0, 'Baseline DOM probes or roots leaked');
    const scaledContext=await verifyScaledContext();
    await call('text_baseline_mount', 2, 1, false); await settle(); await verify(2, 1); await call('text_baseline_stop');
    assert(errors.length === 0, errors.join('\n'));
    return {passed: true, contextDpr: await page.evaluate(() => devicePixelRatio), actualOrdinaryText: true, noGlyphSubstitution: true, htmlBaselineMarker: true, realSvgCanvasInk: true,
      actualFontsAndFallback: true, anchors: true, dpr: true, fontOnlyRetained: true, staleAtomic: true, fontReadiness: true,
      whitespaceAndSafeText: true, emptyLineBoxes: true, missingLayoutRejected: true, budgets: true, hiddenRestore: true, movedBaseline: true, cleanup: true, remount: true,
      cases, fontOnly, collision, baselineNegativeControl, scaledContext, probes: {whitespace: probe, empty: emptyCases, safeText: unsafe, missingLayoutRejected: detachedLayoutRejected}, pageErrors: errors};
  } finally {
    if (pendingRoute) await pendingRoute.abort().catch(() => {});
    await page.unroute('**/text-baseline-pending.ttf').catch(() => {});
    await call('text_baseline_stop');
    await page.evaluate(display => { document.querySelector('.panels').style.display = display; window.baselineText = null; }, panelDisplay);
    page.off('pageerror', onError);
  }
  }
  const primary = await innerCore(page, url);
  if (primary.contextDpr === 1) return {...primary, ordinaryDpr1: primary};
  const browser = page.context().browser();
  if (!browser) throw Error('Ordinary Text DPR acceptance requires an independent browser context');
  const context = await browser.newContext({viewport: {width: 1440, height: 1000}, deviceScaleFactor: 1});
  try {
    const secondaryPage = await context.newPage(); secondaryPage.setDefaultTimeout(45000);
    const ordinaryDpr1 = await innerCore(secondaryPage, url);
    if (ordinaryDpr1.contextDpr !== 1) throw Error('Ordinary Text secondary context did not use actual DPR 1');
    return {...primary, ordinaryDpr1};
  } finally { await context.close(); }
})
