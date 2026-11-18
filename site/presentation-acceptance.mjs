import assert from 'node:assert/strict';

// Exercise the same native mark in the SVG chart and the Canvas hit tester.
export async function checkBrowserPresentation(page, select, settle) {
  const report = [];
  const reduced = await page.evaluate(()=>matchMedia('(prefers-reduced-motion: reduce)').matches);
  const animationWasEnabled = await page.locator('#animation-enabled').isChecked();
  for(const slug of ['bar','heatmap']) {
    await select(slug);
    await page.locator('#renderer-svg').click();
    await settle();
    const target = page.locator('#chart-stage .kartis-surface > svg path[data-series]:not([data-series=""])').first();
    await target.scrollIntoViewIfNeeded();
    const mark = await target.boundingBox();
    const surface = page.locator('#chart-stage .kartis-surface');
    const box = await surface.boundingBox();
    assert(mark.width > 1 && mark.height > 1);
    const xRatio = (mark.x + mark.width / 2 - box.x) / box.width;
    const yRatio = (mark.y + mark.height / 2 - box.y) / box.height;
    const option = (key) => page.locator('#chart-option-' + key);
    const tip = page.locator('#chart-stage .kartis-surface-tooltip');
    const legend = page.locator('#chart-stage .kartis-legend');
    const hover = async () => {
      await surface.scrollIntoViewIfNeeded();
      const bounds = await surface.boundingBox();
      const x = bounds.x + bounds.width * xRatio;
      const y = bounds.y + bounds.height * yRatio;
      await page.mouse.move(x + 1, y);
      await page.mouse.move(x, y);
      await settle();
    };
    for (const renderer of ['svg', 'canvas']) {
      await page.locator('#renderer-' + renderer).click();
      await settle();
      const owned = await surface.elementHandle();
      const state = await page.evaluate(() => window.kartisExplorer.snapshot());
      await hover();
      await tip.waitFor({state:'visible'});
      const detail = await tip.textContent();
      assert(detail.includes(':'), renderer + ' native detail tooltip');
      await option('tooltip-content').selectOption('value');
      await option('tooltip-format').selectOption('fixed');
      await option('tooltip-style').selectOption('light');
      await hover();
      assert.match(await tip.textContent(), /^-?\d+\.\d{2}$/);
      assert.equal(await tip.evaluate(node => getComputedStyle(node).backgroundColor), 'rgb(251, 253, 248)');
      await option('tooltip-content').selectOption('hidden');
      await hover();
      await tip.waitFor({state:'hidden'});
      // Restore the same native pointer target, formatter and colours.
      for (const [key, value] of [['tooltip-content','detail'],['tooltip-format','auto'],['tooltip-style','default']]) {
        await option(key).selectOption(value);
      }
      await hover();
      await tip.waitFor({state:'visible'});
      assert.equal(await tip.textContent(), detail);
      if(slug==='bar') {
        const button = legend.getByRole('button', {name:/^Hide /}).first();
        const series = (await button.textContent()).replace(/^Hide /,'');
        await button.click();
        await settle();
        await option('legend-display').selectOption('hidden');
        await settle();
        assert(!await legend.isVisible());
        await option('legend-display').selectOption('stacked');
        await settle();
        const rows = await legend.locator('.kartis-legend-item').evaluateAll(nodes=>nodes.map(n=>n.getBoundingClientRect().y));
        assert(rows.length >= 2 && rows[1] > rows[0], renderer+' stacked legend');
        assert(await legend.evaluate(node => {
          const last = [...node.querySelectorAll('.kartis-legend-item')].at(-1);
          return last.getBoundingClientRect().bottom <= document.querySelector('.stage-status').getBoundingClientRect().top+1;
        }), renderer+' entire native legend fits above the status bar');
        await legend.getByRole('button', {name:'Show '+series, exact:true}).click();
        await settle();
        await option('legend-display').selectOption('inline');
      } else {
        assert.equal(await option('legend-display').count(),0);
      }
      const restored = await settle();
      assert.equal(restored.seed, state.seed);
      assert(await owned.evaluate(node=>node===document.querySelector('#chart-stage .kartis-surface')));
      await owned.dispose();
      await page.emulateMedia({reducedMotion:'no-preference'});
      await page.locator('#animation-enabled').check();
      await settle();
      const runningSurface=await surface.elementHandle();
      await page.locator('#new-data').click();
      await page.waitForFunction(()=>window.kartisExplorer.snapshot().animating);
      // Dispatch through the visible control while an actual spring is in flight.
      const transition=await option('tooltip-content').evaluate(node=>{
        const before=window.kartisExplorer.snapshot();
        node.value='value';node.dispatchEvent(new Event('change',{bubbles:true}));
        const after=window.kartisExplorer.snapshot();
        return {before,after};
      });
      assert(transition.before.animating && transition.after.animating);
      assert.equal(transition.before.seed,transition.after.seed);
      assert.equal(transition.after.settings['tooltip-content'],'value');
      assert.equal(transition.before.revision,transition.after.revision);
      const finished=await settle();
      assert.equal(finished.seed,transition.before.seed);
      assert(await runningSurface.evaluate(node=>node===document.querySelector('#chart-stage .kartis-surface')));
      await runningSurface.dispose();
      await option('tooltip-content').selectOption('detail');
      await page.emulateMedia({reducedMotion:reduced?'reduce':'no-preference'});
      if(!animationWasEnabled) await page.locator('#animation-enabled').uncheck();
      report.push({slug,renderer,hover:true,fixedDecimals:true,lightTooltip:true,hiddenTooltip:true,retainedMotion:true,legend:slug==='bar'?'hidden, stacked and toggled':'no legend control'});
    }
  }
  await select('waffle');
  for (const renderer of ['svg','canvas']) {
    await page.locator('#renderer-'+renderer).click();
    await page.locator('#chart-option-legend-display').selectOption('stacked');
    await settle();
    const bounds = await page.locator('#chart-stage .kartis-legend').evaluate(legend => {
      const items = [...legend.querySelectorAll('.kartis-legend-item')];
      return {items:items.length,last:items.at(-1).getBoundingClientRect().bottom,
        status:document.querySelector('.stage-status').getBoundingClientRect().top};
    });
    assert.equal(bounds.items,4);
    assert(bounds.last<=bounds.status+1, renderer+' complete Waffle legend is visible');
    await page.locator('#chart-option-legend-display').selectOption('inline');
    await settle();
    report.push({slug:'waffle',renderer,legendBounds:bounds});
  }
  return report;
}
