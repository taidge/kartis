(async function verifyHeatmapCustom(page) {
  const errors = [], onError = error => errors.push(error.message);
  page.on('pageerror', onError);
  try {
    const result = await page.evaluate(async () => {
      const assert = (value, message) => { if (!value) throw Error(`Custom HeatMap: ${message}`); };
      const close = (a, b, tolerance = 1e-9) => Math.abs(a - b) <= tolerance;
      const metadata = document.querySelector('#heatmap-custom-data');
      assert(metadata, 'missing generated typed source');
      // Independent authored sample: plot [20,20,280,140], three lexical
      // columns [null,value,zero], no padding. No position is fitted to pixels.
      const columns = ['null', 'value', 'zero'], raw = [null, 7, 0];
      const position = index => [20 + (index + .5) * 280 / 3, 90];
      const id = column => `heatmap-cell:1:r:${column.length}:${column}`;
      const readSource = () => {
        const frames = JSON.parse(metadata.textContent);
        assert(frames.length === 2, 'two original source frames');
        for (const [index, frame] of frames.entries()) {
          assert(frame.frame === ['base', 'active'][index] && frame.width === 320 && frame.height === 180, 'frame identity');
          assert(JSON.stringify(frame.plot) === '[20,20,280,140]' && frame.cells.length === 3, 'plot/source count');
          frame.cells.forEach((cell, i) => {
            const expected = position(i), opacity = index === 1 && i !== 0 ? .15 : 1;
            assert(cell.row === 'r' && cell.column === columns[i] && cell.raw === raw[i] && cell.tooltipValue === raw[i], 'original nullable raw/tooltip values');
            assert(cell.position.length === 2 && cell.position.every((v, j) => close(v, expected[j])) && close(cell.opacity, opacity), 'logical origin/controlled opacity');
          });
        }
        return frames;
      };
      const checkSVG = () => {
        const circles = [];
        for (const name of ['base', 'active']) {
          const svg = document.querySelector(`#frame-${name} svg`);
          assert(svg && svg.getAttribute('viewBox') === '0 0 320 180', 'actual SVG dimensions');
          assert(svg.querySelectorAll('[data-id]').length === 3, 'three actual data marks');
          columns.forEach((column, i) => {
            const mark = svg.querySelector(`[data-id="${id(column)}"]`), [x, y] = position(i);
            assert(mark && +mark.dataset.value === (raw[i] ?? 0), 'finite Scene scalar compatibility');
            const opacity = name === 'active' && i !== 0 ? .15 : 1;
            assert(close(+(mark.getAttribute('opacity') ?? 1), opacity), 'actual SVG opacity');
            if (column === 'zero') {
              // Geometry::Circle is normalized by Geometry::commands into the
              // same four cubic segments for SVG and Canvas. Verify that exact
              // source-authored support, rather than assuming an SVG circle tag.
              assert(mark.localName === 'path', 'actual normalized zero circle Path');
              const d = mark.getAttribute('d'), token = /[MCZ]|[-+]?(?:\d*\.\d+|\d+\.?\d*)(?:[eE][-+]?\d+)?/g;
              assert(typeof d === 'string' && d.replace(token, '').replace(/[\s,]/g, '') === '', 'canonical zero circle path grammar');
              const actual = (d.match(token) || []).map(value => /^[MCZ]$/.test(value) ? value : Number(value));
              const r = 20, k = r * .5522847498307936;
              const expected = [
                'M', x+r, y,
                'C', x+r, y+k, x+k, y+r, x, y+r,
                'C', x-k, y+r, x-r, y+k, x-r, y,
                'C', x-r, y-k, x-k, y-r, x, y-r,
                'C', x+k, y-r, x+r, y-k, x+r, y,
                'Z',
              ];
              assert(actual.length === expected.length && expected.every((value, j) => typeof value === 'string' ? actual[j] === value : Number.isFinite(actual[j]) && close(actual[j], value)), 'actual zero circle origin/radius/cubic support');
              const box = mark.getBBox();
              assert(close(box.x, x-r, .001) && close(box.y, y-r, .001) && close(box.width, 2*r, .001) && close(box.height, 2*r, .001), 'actual zero circle bounds');
              assert(mark.getAttribute('fill') === 'none' && +mark.getAttribute('stroke-width') === 2, 'hollow circle ink');
              circles.push({frame:name, column, logicalOrigin:[x,y], radius:r, actual, expected});
            } else {
              const box = mark.getBBox();
              assert(mark.localName === 'path' && close(box.x, x - 20, .001) && close(box.y, y - 20, .001) && close(box.width, 40, .001) && close(box.height, 40, .001), 'actual diamond bounds');
              assert(mark.getAttribute('fill').startsWith('url('), 'actual pattern/gradient paint');
            }
          });
        }
        return circles;
      };
      readSource(); const circleSupport = checkSVG();
      const negatives = [], rejected = action => {
        try { action(); return false; } catch { return true; }
      };
      const originalMetadata = metadata.textContent;
      try {
        const changed = JSON.parse(originalMetadata); changed[0].cells[0].raw = 0;
        metadata.textContent = JSON.stringify(changed);
        assert(rejected(readSource), 'actual nullable-source mutation must reject');
      } finally { metadata.textContent = originalMetadata; }
      readSource(); negatives.push({kind:'wrongNullableSource', rejected:true, restored:true, actualDOM:true});
      const shape = document.querySelector(`#frame-base [data-id="${id('null')}"]`), originalPath = shape.getAttribute('d');
      try {
        let n = 0;
        shape.setAttribute('d', originalPath.replace(/[-+]?(?:\d*\.\d+|\d+\.?\d*)(?:[eE][-+]?\d+)?/g, value => String(+value + (n++ % 2 === 0 ? 12 : 0))));
        assert(rejected(checkSVG), 'actual geometry mutation must reject');
      } finally { shape.setAttribute('d', originalPath); }
      checkSVG(); negatives.push({kind:'wrongDiamondGeometry', rejected:true, restored:true, actualDOM:true});
      const zero = document.querySelector(`#frame-base [data-id="${id('zero')}"]`), zeroPath = zero.getAttribute('d');
      const zeroBounds = () => { const r = zero.getBBox(); return [r.x,r.y,r.width,r.height]; };
      const zeroBefore = zeroBounds(); let zeroDuring;
      try {
        const parts = zeroPath.match(/[MCZ]|[-+]?(?:\d*\.\d+|\d+\.?\d*)(?:[eE][-+]?\d+)?/g);
        parts[5] = String(+parts[5] + 1); // first cubic handle Y; cardinal endpoints stay fixed
        zero.setAttribute('d', parts.join(' ')); zeroDuring = zeroBounds();
        assert(JSON.stringify(zeroDuring) === JSON.stringify(zeroBefore), 'zero cubic mutation keeps original bounds');
        assert(rejected(checkSVG), 'actual same-bounds zero cubic mutation must reject');
      } finally { zero.setAttribute('d', zeroPath); }
      checkSVG(); negatives.push({kind:'wrongZeroCubicSupport', rejected:true, restored:true, actualDOM:true, beforeBounds:zeroBefore, mutatedBounds:zeroDuring, afterBounds:zeroBounds()});
      const rows = [], mean = (a, b) => {
        let sum = 0;
        for (let i = 0; i < a.length; i++) sum += Math.abs(a[i] - b[i]);
        return sum / a.length;
      };
      for (const name of ['base', 'active']) {
        const section = document.querySelector(`#frame-${name}`), svg = section.querySelector('svg'), canvas = section.querySelector('canvas');
        assert(canvas.width === 320 * devicePixelRatio && canvas.height === 180 * devicePixelRatio, 'actual DPR backing dimensions');
        const box = element => {
          const r = element.getBoundingClientRect();
          return {x:r.x, y:r.y, width:r.width, height:r.height};
        };
        const geometry = () => ({section:box(section), svg:box(svg), canvas:box(canvas), logical:[320,180], backing:[canvas.width,canvas.height], dpr:devicePixelRatio, scroll:[scrollX,scrollY]});
        const geometryBefore = geometry(), ctx = canvas.getContext('2d'), original = ctx.getImageData(0, 0, canvas.width, canvas.height);
        const clone = svg.cloneNode(true); clone.setAttribute('lang', document.documentElement.lang || 'en');
        const url = URL.createObjectURL(new Blob([new XMLSerializer().serializeToString(clone)], {type:'image/svg+xml'}));
        const image = new Image(); image.src = url;
        try { await image.decode(); } finally { URL.revokeObjectURL(url); }
        const reference = document.createElement('canvas'); reference.width = canvas.width; reference.height = canvas.height;
        reference.getContext('2d').drawImage(image, 0, 0, reference.width, reference.height);
        const pixels = reference.getContext('2d').getImageData(0, 0, reference.width, reference.height), error = mean(original.data, pixels.data);
        assert(error < .5, 'existing whole-Scene mean-channel gate<0.5');
        const points = [
          {kind:'pattern-stripe', position:[20+280/6,88], color:[200,30,40,255]},
          {kind:'pattern-background', position:[20+280/6,92], color:[20,80,150,255]},
          {kind:'hollow-center', position:[20+280*5/6,90], color:[255,255,255,255]},
          {kind:'hollow-stroke', position:[20+280*5/6+20,90], color:[10,20,30,255]},
          {kind:'gradient-middle', position:[160,90], gradient:true},
        ], witnesses = [];
        for (const point of points) {
          const x = Math.floor(point.position[0] * devicePixelRatio), y = Math.floor(point.position[1] * devicePixelRatio), offset = (y * canvas.width + x) * 4;
          let expected = point.color;
          if (point.gradient) {
            const t = ((x + .5) / devicePixelRatio - 140) / 40;
            expected = [0, 200 * (1 - t), 200 * t, 255];
          }
          if (name === 'active' && (point.gradient || point.kind === 'hollow-stroke')) expected = expected.map((v, i) => i === 3 ? 255 : v * .15 + 255 * .85);
          for (const [backend, data] of [['canvas',original.data],['decodedSVG',pixels.data]]) {
            const actual = [...data.slice(offset, offset + 4)];
            assert(actual.every((v, i) => Math.abs(v - expected[i]) <= 2), `actual ${backend} source-position ${point.kind}`);
            witnesses.push({backend, kind:point.kind, logical:point.position, backing:[x,y], actual, expected});
          }
        }
        ctx.save();
        try { ctx.setTransform(1,0,0,1,0,0); ctx.fillStyle = '#fff'; ctx.fillRect(0,0,canvas.width,canvas.height); }
        finally { ctx.restore(); }
        let badMean;
        try {
          badMean = mean(ctx.getImageData(0,0,canvas.width,canvas.height).data, pixels.data);
          assert(badMean >= .5, 'actual blank-paint negative must reject');
        } finally { ctx.putImageData(original,0,0); }
        const restored = ctx.getImageData(0,0,canvas.width,canvas.height).data;
        assert(restored.every((v, i) => v === original.data[i]), 'full original bitmap restored');
        const geometryAfter = geometry();
        assert(JSON.stringify(geometryAfter) === JSON.stringify(geometryBefore), 'stable actual geometry');
        negatives.push({kind:'wrongCanvasPaint', frame:name, rejected:true, restored:true, actualCanvas:true, badMean});
        rows.push({name, mean:error, logical:[320,180], backing:[canvas.width,canvas.height], geometryBefore, geometryAfter, witnesses, canvasPNG:canvas.toDataURL(), svgPNG:reference.toDataURL()});
      }
      return {passed:true, scope:'Fixed custom HeatMap geometry/paint and controlled-opacity exports, with original nullable metadata and actual source-position pixels. Labels=false; no generic-font, arbitrary Widget or BrowserChart event acceptance.', rows, circleSupport, negativeControls:negatives};
    });
    if (errors.length) throw Error(errors.join('; '));
    return result;
  } finally { page.off('pageerror', onError); }
})
