import assert from 'node:assert/strict';

async function settledInk(surface) {
  if (surface instanceof SVGElement) {
    return [...surface.querySelectorAll('path[data-id]')].map(path=>path.getAttribute('d'));
  }
  const pixels = surface.getContext('2d').getImageData(0,0,surface.width,surface.height).data;
  const digest = await crypto.subtle.digest('SHA-256', pixels);
  return [...new Uint8Array(digest)];
}

// The old gallery resampled even compatible D3 basis ribbons into 64 straight
// segments. A pixel hash changing during the animation cannot catch that.
function watchCanvas(canvas) {
  const ctx = canvas.getContext('2d');
  const commands = {};
  const originals = new Map();
  const painted = [];
  for (const name of ['beginPath','lineTo','bezierCurveTo','stroke','fill']) {
    originals.set(name, Object.getOwnPropertyDescriptor(ctx, name));
    const original = ctx[name];
    ctx[name] = function(...args) {
      if (name === 'beginPath') {
        commands.lines = commands.cubics = 0;
      } else if (name === 'lineTo') {
        commands.lines++;
      } else if (name === 'bezierCurveTo') {
        commands.cubics++;
      } else if (commands.lines + commands.cubics >= 10) {
        // Excludes the axis ticks, circles and rectangles in these two charts.
        painted.push(commands.cubics);
      }
      return original.apply(this,args);
    };
  }
  return {
    read: () => painted.splice(0),
    stop() {
      for (const [name, descriptor] of originals) {
        if (descriptor) Object.defineProperty(ctx, name, descriptor);
        else delete ctx[name];
      }
    },
  };
}

async function checkLabelBounds(page, settle, slug, width) {
  await page.setViewportSize({width,height:900});
  await page.goto(new URL('#chart/'+slug, page.url()).href);
  await settle();
  await page.locator('#animation-enabled').uncheck();
  await page.locator('#renderer-svg').click();
  await page.locator('#chart-option-label-size').selectOption('default');
  await page.locator('#plot-spacing').evaluate(slider => {
    slider.value='100';
    slider.dispatchEvent(new Event('change',{bubbles:true}));
  });
  await settle();
  for (const large of [false,true]) {
    if (large) {
      await page.locator('#chart-option-label-size').selectOption('large');
      await page.locator('#plot-spacing').evaluate(slider => {
        slider.value=slider.min;
        slider.dispatchEvent(new Event('change',{bubbles:true}));
      });
      await settle();
    }
    const {labels,clipped,span}=await page.locator('#chart-stage .kartis-surface > svg').evaluate(svg=>{
      const {width,height}=svg.viewBox.baseVal;
      const labels=[...svg.querySelectorAll('text')];
      const ticks=labels.filter(node=>/^Q\d+$/.test(node.textContent)).map(node=>Number(node.getAttribute('x')));
      const clipped=labels.filter(node=>{
        const r=node.getBBox();
        return r.x<1 || r.y<1 || r.x+r.width>width-1 || r.y+r.height>height-1;
      }).map(node=>node.textContent);
      return {labels:labels.map(node=>node.textContent),clipped,span:Math.max(...ticks)-Math.min(...ticks)};
    });
    assert(labels.includes(slug==='bump'?'South':'Partners'));
    assert.deepEqual(clipped,[],slug+' at '+width+' / '+(large?'large labels with minimum spacing':'default spacing'));
    assert(span>110,slug+' retained space for all six columns');
  }
}

export async function checkRankMotion(explorerPage) {
  const page = await explorerPage.context().browser().newPage({
    viewport:{width:1440,height:1000}, reducedMotion:'no-preference',
  });
  const errors = [], results = [];
  page.on('pageerror', error => errors.push(error.message));
  const settle = async () => {
    await page.waitForFunction(() => {
      const state = window.kartisExplorer?.snapshot();
      return state?.active && !state.animating && !state.pending;
    });
    await page.evaluate(() => new Promise(done => requestAnimationFrame(() => requestAnimationFrame(done))));
  };
  try {
    await page.goto(new URL('#chart/line', explorerPage.url()).href);
    await settle();
    for (const [slug, smooth] of [['bump','smooth'],['area-bump','middle']]) {
      for (const renderer of ['svg','canvas']) {
        const name = slug+'/'+renderer;
        await page.locator('#chart-nav [data-slug="line"]').click();
        await page.locator('#renderer-'+renderer).click();
        await page.locator('#animation-enabled').check();
        await page.locator('#spring-preset').selectOption('gentle');
        await settle();
        await page.locator('#chart-nav [data-slug="'+slug+'"]').click();
        await page.waitForFunction(() => window.kartisExplorer.snapshot().animating);
        await settle();
        const surface = page.locator('#chart-stage .kartis-surface > '+renderer);
        const trace = renderer==='canvas' ? await surface.evaluateHandle(watchCanvas) : null;
        try {
          const frame = async () => {
            // Read while the spring is moving, before the authored endpoint
            // has been restored. This also checks interrupted curve changes.
            await page.evaluate(() => new Promise(done => requestAnimationFrame(() => requestAnimationFrame(done))));
            assert(await page.evaluate(() => window.kartisExplorer.snapshot().animating), name+' moving');
            const cubics = trace
              ? await trace.evaluate(t => t.read())
              : await surface.locator('path[data-id^="bump-line:"],path[data-id^="area-bump:"]').evaluateAll(
                paths => paths.map(n => (n.getAttribute('d').match(/C/g) ?? []).length));
            assert(cubics.length>=3 && cubics.every(n=>n>=10), name+' retained native cubics in all moving series: '+cubics);
            return Math.min(...cubics);
          };
          const setCurve = value => page.locator('#variant-select').evaluate((select,value) => {
            select.value=value; select.dispatchEvent(new Event('change',{bubbles:true}));
          },value);
          await page.locator('#new-data').click();
          const data = await frame();
          await setCurve('linear');
          const toLinear = await frame();
          await setCurve(smooth);
          const interrupted = await frame();
          await settle();
          const final = await surface.evaluate(settledInk);
          await page.locator('#animation-enabled').uncheck();
          await page.locator('#chart-nav [data-slug="'+slug+'"]').click();
          await settle();
          assert.deepEqual(await surface.evaluate(settledInk),final,name+' settles on the independent complete render');
          results.push({slug,renderer,data,toLinear,interrupted});
        } finally {
          if(trace){await trace.evaluate(t=>t.stop());await trace.dispose();}
        }
      }
    }
    for(const width of [1440,320]) for(const slug of ['bump','area-bump']) {
      await checkLabelBounds(page,settle,slug,width);
    }
  } finally { await page.close(); }
  assert.deepEqual(errors,[]);
  return results;
}
