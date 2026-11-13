(async function verifyConfiguredPaintGallery(page) {
  return page.evaluate(() => {
    const cards = [...document.querySelectorAll('article')];
    const card = name => {
      const result = cards.find(node => node.querySelector('h2').textContent === name);
      if (!result) throw Error(`Missing configured chart: ${name}`);
      return {
        svg: result.querySelector('svg'),
        canvas: result.querySelector('canvas'),
      };
    };
    const assert = (condition, message) => {
      if (!condition) throw Error(`Configured paint geometry failed: ${message}`);
    };
    const close = (a, b) => Math.abs(a - b) < .001;
    const color = (node, attr) => {
      const value = node.getAttribute(attr);
      return value.startsWith('rgba(') ? value.slice(5, -1).split(',').map(Number) : [];
    };
    const sameRgb = (actual, rgb) => rgb.every((value, i) => actual[i] === value);
    const pixel = ({canvas}, x, y) => [...canvas.getContext('2d').getImageData(
      Math.floor(x * canvas.width / 720), Math.floor(y * canvas.height / 400), 1, 1,
    ).data];
    const center = node => {
      const box = node.getBBox();
      return [box.x + box.width / 2, box.y + box.height / 2];
    };
    // Independent d3-scale-chromatic Category10 and d3-color darker(.8).
    const cases = [
      {id:'South', value:28, rgb:[255,127,14], border:[192,95,11], label:[0,0,0], points:[15,25,28]},
      {id:'Central', value:32, rgb:[44,160,44], border:[33,120,33], label:[0,0,0], points:[14,22,32]},
      {id:'North', value:40, rgb:[31,119,180], border:[23,89,135], label:[255,255,255], points:[25,30,40]},
    ];
    const pie = card('Pie · ordinal colors and contrast');
    const pieColors = [];
    const pad = .035, available = 2 * Math.PI - cases.length * pad;
    let angle = 0;
    for (const {id, value, rgb, border, label} of cases) {
      const mark = pie.svg.querySelector(`path[data-id="${id}"]`);
      const leader = pie.svg.querySelector(`path[data-id^="pie-layout-link:"][data-series="${id}"]`);
      const text = [...pie.svg.querySelectorAll('text[text-anchor="middle"]')].find(node => node.textContent === id);
      const extent = available * value / 100 + pad;
      const middle = angle + extent / 2;
      const ink = pixel(pie, 380 + 100 * Math.sin(middle), 190 - 100 * Math.cos(middle));
      assert(+mark.dataset.value === value && sameRgb(color(mark, 'fill'), rgb) &&
        sameRgb(color(mark, 'stroke'), border) && mark.getAttribute('stroke-width') === '2' &&
        sameRgb(color(leader, 'stroke'), rgb) && close(color(leader, 'stroke')[3], 179 / 255) &&
        sameRgb(color(text, 'fill'), label) && sameRgb(ink, rgb) && ink[3] === 255, `Pie paint for ${id}`);
      pieColors.push({id, ink, contrast:color(text, 'fill')});
      angle += extent;
    }

    const bar = card('Bar · fixed ordinal color domain');
    const bars = [...bar.svg.querySelectorAll('path[data-id^="styled-bar:"]')];
    const barColors = [];
    assert(bars.length === 3, 'bar count');
    let previousX = 0;
    for (const [index, id] of ['Central','North','South'].entries()) {
      const mark = bars[index];
      const [x, y] = center(mark);
      const {rgb, value} = cases.find(entry => entry.id === id);
      const ink = pixel(bar, x, y);
      assert(x > previousX && mark.dataset.id.endsWith(`:${id}`) && +mark.dataset.value === value &&
        sameRgb(color(mark, 'fill'), rgb) && sameRgb(ink, rgb) && ink[3] === 255, `bar color identity for ${id}`);
      barColors.push({id, x, ink});
      previousX = x;
    }

    const line = card('Line · ordinal series colors');
    const lineColors = cases.map(({id, rgb, points}) => {
      const stroke = line.svg.querySelector(`path[data-id^="line-stroke:"][data-series="${id}"]`);
      const marks = [...line.svg.querySelectorAll(`path[data-id^="line-point:"][data-series="${id}"]`)];
      const ink = pixel(line, ...center(marks[1]));
      assert(sameRgb(color(stroke, 'stroke'), rgb) && marks.length === 3 &&
        marks.every((mark, i) => +mark.dataset.value === points[i] && sameRgb(color(mark, 'fill'), rgb)) &&
        sameRgb(ink, rgb) && ink[3] === 255, `line and points for ${id}`);
      return {id, ink, points};
    });

    const radar = card('Radar · offset custom point outlines');
    const dots = [...radar.svg.querySelectorAll('path[data-id^="radar-point:"]')];
    const focus = dots.find(mark => mark.dataset.id.endsWith(':Focus'));
    const anchorX = 380 + 134.3 * .96;
    const [dotX, dotY] = center(focus);
    const label = [...radar.svg.querySelectorAll('text')].find(node => node.textContent === '96');
    const edgeInk = pixel(radar, dotX + 12, dotY);
    const centerInk = pixel(radar, dotX, dotY);
    assert(dots.length === 4 && +focus.dataset.value === 96 && close(dotX, anchorX + 24) &&
      close(dotY, 190) && close(+label.getAttribute('x'), anchorX) &&
      focus.getAttribute('fill') === 'none' && focus.getAttribute('stroke-width') === '2.5' &&
      focus.ownerSVGElement.getAttribute('overflow') === 'hidden' &&
      focus.isPointInStroke(new DOMPoint(dotX + 12, dotY)) &&
      !focus.isPointInStroke(new DOMPoint(dotX, dotY)) &&
      sameRgb(edgeInk, [171,38,91]) && sameRgb(centerInk, [255,255,255]), 'Radar offset hollow outline and crop');

    const bullet = card('Bullet · logarithmic ranges and markers');
    const x = value => 64 + 632 * Math.log10(value) / 3;
    const request = part => bullet.svg.querySelector(`path[data-id="bullet:8:Requests:${part}"]`);
    const axis = [...bullet.svg.querySelectorAll('path[data-id^="bullet-axis:8:Requests-tick-"]')];
    const ranges = ['range:0', 'range:1', 'range:domain'].map((part, i) => {
      const mark = request(part), box = mark.getBBox();
      const [from,to] = [[1,10],[10,100],[100,1000]][i];
      assert(close(box.x, x(from)) && close(box.x + box.width, x(to)) &&
        +mark.dataset.value === to, `Bullet range ${from} to ${to}`);
      return [box.x, box.x + box.width];
    });
    const marker = request('marker:0');
    const [markerX, markerY] = center(marker);
    const markerInk = pixel(bullet, markerX, markerY);
    assert(axis.length === 4 && axis.every((tick, i) => close(center(tick)[0], x(10 ** i))) &&
      close(markerX, x(100)) && +marker.dataset.value === 100 && sameRgb(markerInk, [225,87,89]) &&
      close(request('measure:0').getBBox().x + request('measure:0').getBBox().width, x(28)),
      'Bullet tick, measure and marker projections');
    return {
      pie:pieColors, bar:barColors, line:lineColors,
      radar:{anchorX, dotX, dotY, edgeInk, centerInk},
      bullet:{ranges, markerX, markerInk},
    };
  });
})
