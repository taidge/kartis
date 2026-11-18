import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';

// Use the emitted SVG's rendered contents: the outer owner's generated ID changes on remount.
const svgInk = page => page.locator('#chart-stage .kartis-surface > svg').evaluate(node=>node.innerHTML)
  .then(html=>createHash('sha256').update(html).digest('hex'));

export async function checkVisualControls(page,catalog,select,settle,paint) {
  const results=[];
  await page.emulateMedia({reducedMotion:'reduce'});
  for(const {slug,variants,controls} of catalog) {
    const visual=controls.filter(c=>['Axes and grid','Colors and labels'].includes(c.group));
    assert(visual.some(c=>c.key==='palette'),slug+' has a color scheme');
    assert(visual.some(c=>c.key==='label-mode'),slug+' has data labels');
    assert.equal(new Set(controls.map(c=>c.key)).size,controls.length,slug+' has unique settings');
    await select(slug);
    await page.locator('#variant-select').selectOption(variants[0].value);
    for(const renderer of ['svg','canvas']) {
      await page.locator('#renderer-'+renderer).click();
      const initial=await settle();
      const ink=renderer==='svg'?()=>svgInk(page):async()=>(await paint()).sha256;
      const baseline=await ink();
      for(const control of visual) {
        const picker=page.locator('#chart-option-'+control.key);
        const observed=new Set([baseline]);
        for(const [value] of control.choices.slice(1)) {
          await picker.selectOption(value);
          const updated=await settle();
          assert.equal(updated.settings[control.key],value);
          assert.equal(updated.seed,initial.seed);
          observed.add(await ink());
        }
        assert.equal(observed.size,control.choices.length,`${slug}/${renderer}/${control.key} choices change paint`);
        await picker.selectOption(control.choices[0][0]);
        await settle();
        assert.equal(await ink(),baseline,`${slug}/${renderer} restores ${control.key}`);
      }
      for(const control of visual) {
        await page.locator('#chart-option-'+control.key).selectOption(control.choices[1][0]);
      }
      const combined=await settle();
      assert.equal(combined.seed,initial.seed);
      assert.notEqual(await ink(),baseline);
      for(const control of visual) {
        assert.equal(combined.settings[control.key],control.choices[1][0]);
        await page.locator('#chart-option-'+control.key).selectOption(control.choices[0][0]);
      }
      await settle();
      assert.equal(await ink(),baseline);
      results.push({slug,renderer,controls:visual.map(c=>c.key)});
    }
    console.log(JSON.stringify({slug,appearanceAndAxes:true}));
  }
  return results;
}
