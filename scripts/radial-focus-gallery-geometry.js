(async function verifyRadialFocusGallery(page) {
  return page.evaluate(() => {
    const opacityOf=p=>{
      if(p.hasAttribute('opacity')&&p.hasAttribute('fill-opacity'))return NaN;
      if(p.hasAttribute('fill-opacity')&&(p.hasAttribute('stroke')||p.getAttribute('fill')==='none'||p.getAttribute('fill')?.startsWith('url(')))return NaN;
      return Number(p.getAttribute('opacity')??p.getAttribute('fill-opacity')??1);
    };

    const card = title => {
      const node = [...document.querySelectorAll('article')].find(article =>
        article.querySelector('h2').textContent === title);
      if (!node) throw Error(`Missing example ${title}`);
      const canvas = node.querySelector('canvas');
      const pixel = (x, y) => [...canvas.getContext('2d').getImageData(
        Math.floor(x * canvas.width / 720), Math.floor(y * canvas.height / 400), 1, 1,
      ).data];
      return {node, pixel};
    };

    const radial = card('Stacked RadialBar · rounded sectors');
    const sectors = [...radial.node.querySelectorAll('path[data-id^="radial-segment:"]')];
    const tracks = [...radial.node.querySelectorAll('path[data-id^="radial-track:"]')];
    const ink = new DOMPoint(430, 148);
    const gap = new DOMPoint(380, 126);
    const curved = paths => paths.every(path => (path.getAttribute('d').match(/C/g) ?? []).length >= 5);
    const sectorPixel = radial.pixel(ink.x, ink.y);
    const gapPixel = radial.pixel(gap.x, gap.y);
    if (sectors.length !== 6 || tracks.length !== 2 || !curved(sectors) || !curved(tracks) ||
        +sectors[0].dataset.value !== 30 || !sectors[0].isPointInFill(ink) ||
        sectors.some(path => path.isPointInFill(gap)) ||
        sectorPixel.join() !== '78,121,167,255' || gapPixel.join() === sectorPixel.join()) {
      throw Error(`Rounded radial segments, tracks or gap mismatch: ${JSON.stringify({sectorPixel,gapPixel})}`);
    }

    const focus = ['portfolio', 'sampled motion', 'cloud'].map(phase => {
      const {node, pixel} = card(`Treemap focus · ${phase}`);
      const cloud = node.querySelector('path[data-id="node:Cloud"]');
      const phone = node.querySelector('path[data-id="node:Phone"]');
      return {width: cloud.getBBox().width, phoneOpacity: phone ? opacityOf(phone) : null,
        sample: pixel(560, 190)};
    });
    const [initial, sampled, final] = focus;
    if (!(initial.width < sampled.width && sampled.width < final.width &&
        initial.phoneOpacity === 1 && sampled.phoneOpacity > 0 && sampled.phoneOpacity < 1 &&
        final.phoneOpacity === null && initial.sample.join() === '118,183,178,255' &&
        final.sample.join() === '237,201,72,255' && sampled.sample[3] === 255 &&
        sampled.sample.join() !== initial.sample.join() && sampled.sample.join() !== final.sample.join())) {
      throw Error(`Treemap motion width, opacity or Canvas ink mismatch: ${JSON.stringify(focus)}`);
    }
    return {radial: {sectors: sectors.length, tracks: tracks.length, sectorPixel, gapPixel}, focus};
  });
})
