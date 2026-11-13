(async function verifyAreaBumpLabels(page) {
  const assert = (condition, message) => { if (!condition) throw Error(message); };
  const errors = [], onError = error => errors.push(error.message);
  page.on('pageerror', onError);
  const display = await page.evaluate(() => {
    const panels = document.querySelector('.panels'), previous = panels.style.display;
    panels.style.display = 'none';
    return previous;
  });
  const baseline = await page.locator('body *').count();
  try {
    await page.evaluate(() => {
      window.hostFixture.area_label_mount();
      window.hostFixture.area_label_flush();
      window.areaLabelNodes = new Map([...document.querySelectorAll('#area-label-svg [data-id$=":leader"]')]
        .map(node => [node.dataset.id, node]));
    });
    const phases = [];
    for (const phase of [0, 1]) {
      if (phase) {
        await page.evaluate(phase => {
          window.hostFixture.area_label_phase(phase);
          window.hostFixture.area_label_flush();
        }, phase);
      }
      const proof = await page.evaluate(() => {
        const state = JSON.parse(window.hostFixture.area_label_snapshot('svg'));
        const canvasState = JSON.parse(window.hostFixture.area_label_snapshot('canvas'));
        const canvas = document.querySelector('#area-label-canvas canvas');
        const ctx = canvas.getContext('2d');
        const measure = document.createElement('canvas').getContext('2d');
        const svg = document.querySelector('#area-label-svg svg');
        const texts = [...svg.querySelectorAll('text')];
        const labels = state.labels.filter(label => label.bounds !== null);
        const dx = canvas.width / state.width, dy = canvas.height / state.height;
        const image = ctx.getImageData(0, 0, canvas.width, canvas.height);
        const same = (a,b) => a.length === b.length && a.every((v,i) => Math.abs(v-b[i]) < 0.001);
        const overlap = ([ax,ay,aw,ah], [bx,by,bw,bh]) => Math.min(ax+aw,bx+bw)>Math.max(ax,bx) &&
          Math.min(ay+ah,by+bh)>Math.max(ay,by);
        const retained = [];
        const metrics = labels.map((label,index) => {
          const [x,y,w,h] = label.bounds, [lx,ly] = label.position, [ex] = label.endpoint;
          const side = label.side;
          measure.font = '19px sans-serif';
          measure.textBaseline = 'alphabetic';
          measure.textAlign = side === 'start' ? 'end' : 'start';
          const m = measure.measureText(label.text);
          const expected = [lx-m.actualBoundingBoxLeft,ly-m.actualBoundingBoxAscent,
            m.actualBoundingBoxLeft+m.actualBoundingBoxRight,m.actualBoundingBoxAscent+m.actualBoundingBoxDescent];
          const node = texts.find(text => text.textContent === label.text &&
            text.getAttribute('text-anchor') === measure.textAlign);
          const leader = svg.querySelector(`[data-id="${label.leader}"]`);
          retained.push(leader === window.areaLabelNodes.get(label.leader));
          let ink = 0;
          for (let py = Math.floor(y*dy); py < Math.ceil((y+h)*dy); py++) {
            for (let px = Math.floor(x*dx); px < Math.ceil((x+w)*dx); px++) {
              const offset = 4*(py*canvas.width+px), rgba = image.data;
              if (rgba[offset] >= 215 && rgba[offset+1] <= 80 && rgba[offset+2] >= 135) ink++;
            }
          }
          return {
            id:label.id, side, bounds:label.bounds, ink,
            moved:Math.abs(ly-label.original[1])>0.001,
            measured:same(expected,label.bounds),
            svg:!!node && !!leader && same([+node.getAttribute('x'),+node.getAttribute('y')],label.position),
            fits:x>=0 && y>=0 && x+w<=state.width && y+h<=state.height && lx===label.original[0] &&
              (side==='start' ? x+w<=ex : x>=ex),
            clear:!overlap(label.bounds,state.obstacle) && labels.slice(0,index).every(other => !overlap(label.bounds,other.bounds)),
          };
        });
        return {
          phase:state.phase, metrics, labelsMatch:JSON.stringify(state.labels)===JSON.stringify(canvasState.labels),
          retained:retained.every(Boolean), obstacle:state.obstacle,
          physical:[canvas.width,canvas.height], firstViewport:[state.width,state.height],
        };
      });
      assert(proof.phase===phase && proof.labelsMatch && proof.retained, `AreaBump scene or retained endpoints differ: ${JSON.stringify(proof)}`);
      assert(proof.metrics.length===6 && proof.metrics.filter(m => m.side==='start').length===3 &&
        proof.metrics.some(m => m.moved), `AreaBump did not reflow both endpoint lanes: ${JSON.stringify(proof)}`);
      assert(proof.metrics.every(m => m.measured && m.svg && m.fits && m.clear && m.ink>50),
        `AreaBump measured bounds, SVG, or Canvas ink mismatch: ${JSON.stringify(proof)}`);
      phases.push(proof);
    }
    const active = await page.evaluate(() => {
      const host = window.hostFixture, read = () => JSON.parse(host.area_label_snapshot('svg'));
      const before = read();
      host.area_label_focus(true);
      host.area_label_flush();
      const focused = read();
      const unchangedGeometry = focused.labels.every((label,index) => {
        const node = document.querySelector(`#area-label-svg [data-id="${label.leader}"]`);
        return label.opacity === (label.series==='Waiting' ? 1 : 0.15) &&
          JSON.stringify(label.bounds) === JSON.stringify(before.labels[index].bounds) &&
          node === window.areaLabelNodes.get(label.leader) && +node.getAttribute('opacity')===label.opacity;
      });
      const both = () => ['svg','canvas'].map(renderer => JSON.parse(host.area_label_snapshot(renderer)));
      const beforeRejected = both();
      const rejected = host.area_label_rejected();
      const afterRejected = both();
      return {focus:focused.focus, unchangedGeometry, rejected, atomic:JSON.stringify(beforeRejected)===JSON.stringify(afterRejected)};
    });
    assert(active.focus.join()==='Waiting' && active.unchangedGeometry && active.rejected && active.atomic,
      `AreaBump focus or rejected measure changed layout: ${JSON.stringify(active)}`);
    await page.evaluate(() => window.hostFixture.area_label_stop());
    assert(await page.locator('body *').count()===baseline, 'AreaBump label fixture leaked its roots');
    assert(!errors.length, errors.join('\n'));
    return {passed:true, phases, ...active, pageErrors:errors};
  } finally {
    await page.evaluate(previous => {
      window.hostFixture.area_label_stop();
      document.querySelector('.panels').style.display = previous;
      delete window.areaLabelNodes;
    }, display);
    page.off('pageerror',onError);
  }
})
