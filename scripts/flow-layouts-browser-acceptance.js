(async function verifyFlowLayouts(page) {
  const errors = [];
  const onError = error => errors.push(error.message);
  page.on('pageerror', onError);
  const assert = (condition, detail) => { if (!condition) throw Error(detail); };
  const fixture = (method, ...args) => page.evaluate(([method, args]) => window.hostFixture[method](...args), [method, args]);
  const snapshot = async renderer => JSON.parse(await fixture('layout_models_snapshot', renderer));
  const flush = async () => {
    await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
    await fixture('layout_models_flush');
  };
  const panels = await page.evaluate(() => {
    const panels = document.querySelector('.panels');
    const display = panels.style.display;
    panels.style.display = 'none';
    return display;
  });
  const baseline = await page.locator('body *').count();
  const pixels = target => page.evaluate(async target => {
    const svg = document.querySelector('#layout-model-svg .kartis-surface svg');
    const canvas = document.querySelector('#layout-model-canvas canvas');
    const copy = document.createElement('canvas');
    copy.width = canvas.width;
    copy.height = canvas.height;
    const standalone = svg.cloneNode(true);
    standalone.setAttribute('width', '640');
    standalone.setAttribute('height', '360');
    const image = new Image();
    image.src = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(new XMLSerializer().serializeToString(standalone))}`;
    await image.decode();
    copy.getContext('2d').drawImage(image, 0, 0, copy.width, copy.height);
    const samplePoint = target.samplePoint ?? target.point;
    const sample = surface => [...surface.getContext('2d').getImageData(
      Math.floor(samplePoint[0] * surface.width / 640), Math.floor(samplePoint[1] * surface.height / 360), 1, 1,
    ).data];
    const mark = [...svg.querySelectorAll('path[data-id]')].find(path => path.dataset.id === target.id);
    return {svg: sample(copy), canvas: sample(canvas), value: Number(mark?.dataset.value),
      retained: window.flowLayoutPath ? mark === window.flowLayoutPath : null};
  }, target);
  const move = async (renderer, point, click = false) => {
    const surface = page.locator(`#layout-model-${renderer} .kartis-surface`);
    await surface.scrollIntoViewIfNeeded();
    await page.mouse.move(2, 2);
    const box = await surface.boundingBox();
    const x = box.x + point[0] / 640 * box.width;
    const y = box.y + point[1] / 360 * box.height;
    await page.mouse.move(x, y);
    if (click) await page.mouse.click(x, y);
    await flush();
  };
  const maskedLabelPixels=()=>page.evaluate(async()=>{
    const svg=document.querySelector('#layout-model-svg .kartis-surface svg');
    const surface=document.querySelector('#layout-model-canvas canvas');
    const copy=document.createElement('canvas');copy.width=surface.width;copy.height=surface.height;
    const ctx=copy.getContext('2d');
    const wrappers=[...svg.querySelectorAll('g[clip-path]')].filter(g=>g.querySelector('text'));
    const masks=wrappers.map(g=>{
      const id=g.getAttribute('clip-path').slice(5,-1);
      const clip=[...svg.querySelectorAll('clipPath')].find(p=>p.id===id);
      return [...clip.querySelectorAll('path')].map(path=>[new Path2D(path.getAttribute('d')),path.getAttribute('fill-rule')||'nonzero']);
    }).flat();
    const render=async unmasked=>{
      const clone=svg.cloneNode(true);clone.setAttribute('width','640');clone.setAttribute('height','360');
      if(unmasked)for(const g of clone.querySelectorAll('g[clip-path]'))if(g.querySelector('text'))g.removeAttribute('clip-path');
      const img=new Image();img.src='data:image/svg+xml;charset=utf-8,'+encodeURIComponent(new XMLSerializer().serializeToString(clone));await img.decode();
      ctx.clearRect(0,0,copy.width,copy.height);ctx.drawImage(img,0,0,copy.width,copy.height);return ctx.getImageData(0,0,copy.width,copy.height).data;
    };
    const stats=data=>{
      let ink=0,outside=0;
      for(let y=0;y<copy.height;y++)for(let x=0;x<copy.width;x++){
        const i=(y*copy.width+x)*4;
        if(data[i+3]&&Math.max(data[i],data[i+1],data[i+2])<90){
          ink++;const sx=(x+.5)*640/copy.width,sy=(y+.5)*360/copy.height;
          if(!masks.some(([path,rule])=>[[0,0],[1,0],[-1,0],[0,1],[0,-1]].some(([dx,dy])=>ctx.isPointInPath(path,sx+dx,sy+dy,rule))))outside++;
        }
      }
      return{ink,outside};
    };
    const masked=stats(await render(false));
    const canvas=stats(surface.getContext('2d').getImageData(0,0,surface.width,surface.height).data);
    const unmasked=stats(await render(true));
    return{masks:masks.length,svg:masked,canvas,unmasked};
  });
  const cases = [];
  try {
    for (const kind of ['chord-ribbon', 'chord-rim', 'bump', 'bump-nearest', 'stream-area', 'stream-slice', 'stream-diverging', 'stream-dot', 'stream-dot-nearest', 'hierarchy-labels',
      'pie-radius-labels','radial-radius-labels','sunburst-labels','sunburst-masked-labels']) {
      let maskPixels=null;
      await page.mouse.move(2, 2);
      await fixture('layout_models_mount', kind);
      await flush();
      const before = (await snapshot('svg')).target;
      const initial = await pixels(before);
      const checkPixels = (sample, target) => {
        assert(sample.svg.join() === target.color.join() && sample.canvas.join() === target.color.join(),
          `${kind}: actual SVG/Canvas pixels differ from typed paint ${JSON.stringify({sample,target})}`);
        if (kind !== 'stream-slice') assert(sample.value === target.value, `${kind}: path has stale business value`);
      };
      checkPixels(initial, before);
      await page.evaluate(id => {
        window.flowLayoutPath = [...document.querySelectorAll('#layout-model-svg .kartis-surface svg path[data-id]')]
          .find(path => path.dataset.id === id);
      }, before.id);
      await fixture('layout_models_phase', kind, 1);
      await flush();
      const after = (await snapshot('svg')).target;
      assert(after.id === before.id && after.value !== before.value, `${kind}: source update lost stable ID or current value`);
      const updated = await pixels(after);
      checkPixels(updated, after);
      if (kind !== 'stream-slice') assert(updated.retained, `${kind}: update replaced keyed SVG mark`);
      if (kind.startsWith('chord-')) {
        assert(before.metadata.source === 'B' && after.metadata.source === 'A' &&
          after.metadata.sourceValue === 12 && after.metadata.targetValue === 9 &&
          after.metadata.tooltip.includes('12') && after.metadata.tooltip.includes('9'),
          `${kind}: bidirectional values, dominance or tooltip rows mismatch`);
      } else if (kind.startsWith('bump')) {
        assert(after.metadata.typedX && after.metadata.ranks.join() === '1,4,10' && after.metadata.rank === 10 &&
          after.metadata.shoulders && after.metadata.missing > 0,
          `${kind}: typed category, distinct rank lattice, shoulders or missing slots mismatch`);
      } else if (kind.endsWith('radius-labels')) {
        assert(before.metadata.labels===0&&before.metadata.thickness<before.metadata.skip&&
          after.metadata.labels===after.metadata.afterLabels&&after.metadata.thickness>=after.metadata.skip&&
          before.metadata.legacyLabels===after.metadata.afterLabels&&after.metadata.legacyLabels===after.metadata.afterLabels,
          `${kind}: Nivo ring-thickness filtering or compatible outer-radius profile mismatch`);
      } else if (kind === 'hierarchy-labels'||kind.startsWith('sunburst-')) {
        assert(after.metadata.candidates === 4 && after.metadata.placed > 0 && after.metadata.placed <= 4,
          `${kind}: measured labels disappeared`);
        if(kind==='sunburst-masked-labels')assert(after.metadata.masked===after.metadata.placed&&after.metadata.maskQueries>0,
          `${kind}: actual arc mask was not applied to measured labels`);
        else assert(after.metadata.placed<4&&after.metadata.moved&&(kind!=='hierarchy-labels'||after.metadata.hiddenOmitted),
          `${kind}: measured label displacement or explicit hiding was not applied`);
        const boxes = after.metadata.boxes;
        for (let a = 0; a < boxes.length; a++) for (let b = a + 1; b < boxes.length; b++) {
          const [x,y,w,h] = boxes[a], [u,v,s,t] = boxes[b];
          assert(Math.max(0, Math.min(x+w,u+s)-Math.max(x,u)) * Math.max(0, Math.min(y+h,v+t)-Math.max(y,v)) === 0,
            `${kind}: accepted measured labels still overlap`);
        }
      } else {
        assert(after.metadata.rawMissing && after.metadata.samples === 8 && after.metadata.sourceValue !== before.metadata.sourceValue,
          `${kind}: nullable samples or original value mismatch`);
      }
      if(kind==='sunburst-masked-labels') {
        maskPixels=await maskedLabelPixels();
        assert(maskPixels.masks>0&&maskPixels.svg.ink>25&&maskPixels.canvas.ink>25&&
          maskPixels.svg.outside===0&&maskPixels.canvas.outside===0&&maskPixels.unmasked.ink>maskPixels.svg.ink+25,
          `${kind}: actual SVG/Canvas text was not clipped to arc masks ${JSON.stringify(maskPixels)}`);
      }
      for (const renderer of ['svg', 'canvas']) {
        if (kind === 'bump' || kind === 'stream-dot') {
          await move(renderer, after.metadata.logical);
          assert(!(await snapshot(renderer)).hovered.includes(after.id),
            `${kind}/${renderer}: Geometry picking used the logical origin as visible point ink`);
        }
        await move(renderer, after.point, true);
        const picked = await snapshot(renderer);
        assert(picked.hovered.includes(after.id) && picked.selected !== null, `${kind}/${renderer}: typed pointer pick mismatch`);
        if (kind === 'stream-slice') {
          assert(picked.interaction && picked.interaction.rows.length === 2 &&
            picked.interaction.rows.some(row => row.value === null) &&
            picked.interaction.rows.some(row => row.value === after.value) &&
            picked.interaction.rows.map(row => row.series).join() === after.metadata.sliceOrder.join(),
            `${kind}/${renderer}: slice collapsed missing into zero or lost stack rows ${JSON.stringify(picked.interaction)}`);
          assert(await page.locator(`#layout-model-${renderer}`).getByRole('button', {
            name: after.metadata.missingControl, exact: true,
          }).count() === 1, `${kind}/${renderer}: missing data control used the scalar zero label`);
        }
        const control = page.locator(`#layout-model-${renderer} .kartis-datum`).filter({hasText: after.control}).first();
        await control.focus();
        const activations = (await snapshot(renderer)).events.filter(event => event.type === 'activated').length;
        await page.keyboard.press('Enter');
        await flush();
        assert((await snapshot(renderer)).events.filter(event => event.type === 'activated').length > activations,
          `${kind}/${renderer}: data keyboard activation failed`);
      }
      await page.mouse.move(2, 2);
      await page.evaluate(() => document.activeElement?.blur());
      await flush();
      const accepted = await snapshot('svg');
      const markup = await page.locator('#layout-model-svg .kartis-surface svg').first().innerHTML();
      assert(await fixture('layout_models_rejected', kind, 1), `${kind}: invalid interaction accepted`);
      assert((await snapshot('svg')).revision === accepted.revision,
        `${kind}: rejected interaction queued a paint`);
      await flush();
      assert(await page.locator('#layout-model-svg .kartis-surface svg').first().innerHTML() === markup,
        `${kind}: rejected interaction changed accepted frame`);
      await fixture('layout_models_stop');
      assert(await page.locator('body *').count() === baseline, `${kind}: lifecycle leaked DOM`);
      await page.evaluate(() => { delete window.flowLayoutPath; });
      cases.push({kind, initial, updated, raw: after.metadata, maskPixels, pointer: true, keyboard: true, rejectedAtomic: true, cleanup: true});
    }
    assert(errors.length === 0, errors.join('\n'));
    return {passed: true, cases, pageErrors: errors};
  } finally {
    await fixture('layout_models_stop');
    await page.evaluate(display => {
      document.querySelector('.panels').style.display = display;
      delete window.flowLayoutPath;
    }, panels);
    page.off('pageerror', onError);
  }
})
