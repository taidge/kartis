import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';

// Compare the final painted target with a fresh, instant render of the same inputs.
export async function checkRetargeting(page, select, settle, paint, slugs, setting='layout') {
  const records = [];
  for (const slug of slugs) {
    await select(slug);
    for (const renderer of ['svg','canvas']) {
      await page.locator('#renderer-'+renderer).click();
      await page.locator('#animation-enabled').check();
      await page.locator('#spring-preset').selectOption('gentle');
      await settle();
      const ink = async () => {
        if (renderer==='canvas') return (await paint()).sha256;
        const attrs = await page.locator('#chart-stage .kartis-surface > svg').evaluate(svg =>
          [...svg.querySelectorAll('path,rect,circle,text')].map(node=>
            ['d','width','height','x','y','fill','stroke','opacity','stroke-width'].map(key=>node.getAttribute(key)).concat(node.textContent)));
        return createHash('sha256').update(JSON.stringify(attrs)).digest('hex');
      };
      const changeConfig = async () => {
        let control;
        if(setting!=='layout') control=page.locator('#chart-option-'+setting);
        else if(slug==='bump') control=page.locator('#variant-select');
        else control=page.locator('#chart-controls [data-group="Layout"] select').last();
        const index = await control.evaluate(node=>node.selectedIndex===0?1:0);
        await control.selectOption({index});
      };
      const shuffle = () => page.locator('#new-data').click();
      for (const configFirst of [true,false]) {
        const before = await settle();
        const surface = await page.locator('#chart-stage .kartis-surface').elementHandle();
        const frames = new Set([await ink()]);
        const first = configFirst ? changeConfig : shuffle;
        const second = configFirst ? shuffle : changeConfig;
        await first();
        await page.waitForFunction(()=>window.kartisExplorer.snapshot().animating);
        frames.add(await ink());
        const secondEvent = configFirst ? 'click' : 'change';
        const interrupted = page.evaluate(eventName => new Promise(resolve => {
          document.addEventListener(eventName,()=>resolve(window.kartisExplorer.snapshot().animating),
            {once:true,capture:true});
        }), secondEvent);
        await second();
        assert(await interrupted,slug+'/'+renderer+' receives the second change during the first spring');
        await page.waitForFunction(()=>window.kartisExplorer.snapshot().animating);
        for(let i=0;i<4;i++) {
          await page.evaluate(()=>new Promise(done=>requestAnimationFrame(()=>requestAnimationFrame(done))));
          frames.add(await ink());
        }
        const after=await settle();
        assert.equal(after.seed,before.seed+1);
        assert(await surface.evaluate(node=>node===document.querySelector('#chart-stage .kartis-surface')));
        await surface.dispose();
        assert(frames.size>=3,slug+'/'+renderer+' has intermediate frames');
        const finalInk=await ink();
        await page.locator('#animation-enabled').uncheck();
        // The animation switch now retains the chart. Navigate to the same
        // family again to build an independent, fully painted reference.
        await select(slug);
        const instant=await settle();
        assert.deepEqual(instant.settings,after.settings);
        assert.equal(instant.variant,after.variant);
        assert.equal(await ink(),finalInk,slug+'/'+renderer+' settles on the exact re-rendered target');
        await page.locator('#animation-enabled').check();
        records.push({slug,renderer,setting,configFirst,frames:frames.size});
      }
    }
    console.log(JSON.stringify({slug,interleavedTransitions:4,passed:true}));
  }
  return records;
}
