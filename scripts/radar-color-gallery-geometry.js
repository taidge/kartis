(async function verifyRadarColorGallery(page) {
  const result = await page.evaluate(async () => {
    const card = name => [...document.querySelectorAll('article')].find(node => node.querySelector('h2').textContent === name);
    const bboxCenter = node => {
      const box = node.getBBox();
      return [box.x + box.width / 2, box.y + box.height / 2];
    };
    const close = (a, b) => Math.abs(a - b) < .001;
    const labels = svg => [...svg.querySelectorAll('text')].map(node => node.textContent);
    const at = (node, x, y, value) => {
      const [actualX, actualY] = bboxCenter(node);
      return close(actualX, x) && close(actualY, y) && Number(node.dataset.value) === value;
    };
    const log = card('Radar · logarithmic decades').querySelector('svg');
    const signed = card('Radar · signed SymLog zero ring').querySelector('svg');
    const [x, y, radius] = [380, 190, 134.3];
    const grid = svg => [...svg.querySelectorAll('path[data-id^="radar-grid:"]')];
    const point = (svg, series, axis) => svg.querySelector(`path[data-id="radar-point:${series.length}:${series}:${axis.length}:${axis}"]`);
    const signedPositive = radius * (1 + Math.log1p(10 / 2) / Math.log1p(40 / 2)) / 2;
    const logarithmicDecades = grid(log).length === 3 && grid(log).every((node, i) => close(node.getBBox().width / 2, (i + 1) * radius / 3)) &&
      labels(log).slice(0, 3).join() === '10,100,1000' &&
      at(point(log, 'Measured', 'Response'), x, y - radius / 3, 10) &&
      at(point(log, 'Measured', 'Capacity'), x + 2 * radius / 3, y, 100) &&
      at(point(log, 'Measured', 'Noise'), x, y, 1);
    const signedZeroRing = grid(signed).length === 4 && close(grid(signed)[1].getBBox().width / 2, radius / 2) &&
      labels(signed).slice(0, 5).join() === '-40,-10,0,10,40' &&
      at(point(signed, 'Primary', 'Capacity'), x - radius / 2, y, 0) &&
      at(point(signed, 'Primary', 'Noise'), x, y + signedPositive, 10) &&
      at(point(signed, 'Primary', 'Throughput'), x + radius, y, 40);

    const colors = [];
    for (const [name, expected, stopCount] of [
      ['Heatmap · HCL shortest hue', [196, 1, 166, 255], 33],
      ['Heatmap · HCL direct hue', [0, 132, 42, 255], 33],
      ['Heatmap · Viridis', [33, 145, 140, 255], 512],
    ]) {
      const scene = card(name);
      const svg = scene.querySelector('svg');
      const canvas = scene.querySelector('canvas');
      const copy = svg.cloneNode(true);
      const out = document.createElement('canvas');
      out.width = canvas.width;
      out.height = canvas.height;
      const svgRaster = out.getContext('2d');
      const canvasRaster = canvas.getContext('2d');
      const url = URL.createObjectURL(new Blob([new XMLSerializer().serializeToString(copy)], {type: 'image/svg+xml'}));
      try {
        const image = new Image();
        await new Promise((resolve, reject) => {
          image.onload = resolve;
          image.onerror = reject;
          image.src = url;
        });
        svgRaster.drawImage(image, 0, 0, canvas.width, canvas.height);
      } finally {
        URL.revokeObjectURL(url);
      }
      const pixel = (raster, x, y) => [...raster.getImageData(Math.floor(x * canvas.width / 720), Math.floor(y * canvas.height / 400), 1, 1).data];
      const matches = (actual, color, tolerance = 0) => actual.every((channel, i) => Math.abs(channel - color[i]) <= tolerance);
      const actual = [svgRaster, canvasRaster].map(raster => ({
        cell: pixel(raster, 384, 81),
        ramp: pixel(raster, 360, 352),
        previousRamp: pixel(raster, 359, 352),
      }));
      const gradients = svg.querySelectorAll('linearGradient');
      const stops = [...gradients[0].querySelectorAll('stop')];
      const midpoint = stops.filter(stop => Number(stop.getAttribute('offset')) === .5);
      const previous = [33, 144, 141, 255];
      const lookupBoundary = stopCount !== 512 || (
        midpoint.length === 2 &&
        midpoint[0].getAttribute('stop-color') === 'rgba(33,144,141,1)' &&
        midpoint[1].getAttribute('stop-color') === 'rgba(33,145,140,1)' &&
        actual.every(sample => matches(sample.previousRamp, previous, 1) &&
          sample.previousRamp[1] <= sample.ramp[1] &&
          sample.previousRamp[2] >= sample.ramp[2] &&
          sample.previousRamp.join() !== sample.ramp.join())
      );
      // Browser gradients can quantize a painted channel by one unit even
      // inside a constant bin. The SVG boundary colors above must be exact.
      colors.push({name, actual, stops: stops.length, passed: gradients.length === 1 && stops.length === stopCount &&
        svg.querySelectorAll('[data-id*=":band:"]').length === 1 && lookupBoundary &&
        actual.every(sample => matches(sample.cell, expected) && matches(sample.ramp, expected, 1))});
    }
    return {logarithmicDecades, signedZeroRing, colors};
  });
  if (!result.logarithmicDecades || !result.signedZeroRing || result.colors.some(color => !color.passed)) {
    throw Error(`Radar scales or color profiles failed: ${JSON.stringify(result)}`);
  }
  return result;
})
