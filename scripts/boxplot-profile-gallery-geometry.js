(async function verifyBoxPlotProfileGallery(page) {
  return page.evaluate(() => {
    const card = name => {
      const article = [...document.querySelectorAll('article')].find(node => node.querySelector('h2').textContent === name);
      if (!article) throw Error(`Missing BoxPlot profile ${name}`);
      return {svg:article.querySelector('svg'), canvas:article.querySelector('canvas')};
    };
    const assert = (condition, message) => {
      if (!condition) throw Error(`BoxPlot profile: ${message}`);
    };
    const close = (a,b) => Math.abs(a-b) < .001;
    const mark = (chart,id) => chart.svg.querySelector(`[data-id="${id}"]`);
    const pixel = ({canvas}, x,y) => [...canvas.getContext('2d').getImageData(
      Math.floor(x * canvas.width / 720), Math.floor(y * canvas.height / 400), 1, 1,
    ).data].join();
    const axis = chart => [...chart.svg.querySelectorAll('[data-id^="axis-bar-value-tick-"]')];

    const log = card('BoxPlot · reversed logarithmic samples');
    const logX = value => 64 + 632 * (7 - Math.log2(value)) / 6;
    const logBox = mark(log, 'box:4:Read:0:');
    const logMedian = mark(log, 'box:4:Read:0::median');
    const lowOutlier = mark(log, 'box:4:Read:0::outlier:6:Read-0');
    const highOutlier = mark(log, 'box:4:Read:0::outlier:6:Read-6');
    const logBounds = logBox.getBBox();
    const logY = logBounds.y + logBounds.height / 2;
    assert(+logBox.dataset.value === 16 && +logMedian.dataset.value === 16 &&
      close(logBounds.x, logX(48)) && close(logBounds.x + logBounds.width, logX(6)) &&
      close(logMedian.getBBox().x, logX(16)), 'reversed Log Q1, median and Q3');
    assert(axis(log).length === 7 && axis(log).every((node,i) => close(node.getBBox().x, logX(2 ** (i+1)))),
      'reversed base-2 ticks');
    assert(+lowOutlier.dataset.value === 2 && +highOutlier.dataset.value === 128 &&
      lowOutlier.ownerSVGElement.getAttribute('overflow') === 'hidden' &&
      close(lowOutlier.getBBox().x + lowOutlier.getBBox().width / 2, 696) &&
      close(highOutlier.getBBox().x + highOutlier.getBBox().width / 2, 64) &&
      pixel(log,69,logY) === '174,42,83,255' && pixel(log,63,logY) === '255,255,255,255' &&
      pixel(log,300,logY) === '37,94,153,255', 'Log outlier geometry and actual plot crop');

    const utc = card('BoxPlot · fractional UTC median');
    const midnight = Date.parse('2024-02-29T00:00:00Z');
    const timeX = value => 64 + 632 * (value - (midnight - 2)) / 10;
    const utcBox = mark(utc, 'box:6:Edge A:0:');
    const utcMedian = mark(utc, 'box:6:Edge A:0::median');
    const utcBounds = utcBox.getBBox();
    const utcY = utcBounds.y + utcBounds.height / 2;
    const utcMedianX = utcMedian.getBBox().x;
    const labels = [...utc.svg.querySelectorAll('text')].map(node => node.textContent);
    assert(+utcMedian.dataset.value === midnight + .5 && +utcBox.dataset.value === midnight + .5 &&
      close(utcMedianX, timeX(midnight + .5)) && close(utcBounds.x, timeX(midnight - .75)) &&
      close(utcBounds.x + utcBounds.width, timeX(midnight + 3.25)) &&
      pixel(utc,utcMedianX,utcY) === '245,188,46,255', 'fractional UTC median and quartile ink');
    assert(axis(utc).length === 6 && axis(utc).every((node,i) =>
      close(node.getBBox().x, timeX(midnight - 2 + i * 2))) &&
      ['23:59:59.998','00:00:00.000','00:00:00.008'].every(value => labels.includes(value)),
      'integral UTC tick positions and labels');
    return {
      log:{median:16, medianX:logMedian.getBBox().x, croppedOutlier:true},
      utc:{median:+utcMedian.dataset.value, medianX:utcMedianX, ticks:axis(utc).length},
    };
  });
})
