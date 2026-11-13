(async function verifyCustomLineGallery(page) {
  return page.evaluate(() => {
    const article = [...document.querySelectorAll('article')].find(node =>
      node.querySelector('h2').textContent === 'Line · offset hollow custom points');
    if (!article) throw Error('Missing custom Line example');
    const svg = article.querySelector('svg'), canvas = article.querySelector('canvas');
    const nodes = [...svg.querySelectorAll('path[data-id^="line-point:"]')];
    const close = (a,b) => Math.abs(a-b) < .001;
    const geometry = node => {
      const b = node.getBBox();
      return {x:b.x+b.width/2,y:b.y+b.height/2,left:b.x,right:b.x+b.width};
    };
    const middle = geometry(nodes[1]), edge = geometry(nodes[3]);
    const point = new DOMPoint(middle.right, middle.y);
    const pixel = (x,y) => [...canvas.getContext('2d').getImageData(
      Math.floor(x * canvas.width / 720), Math.floor(y * canvas.height / 400), 1, 1,
    ).data];
    const color = [171,38,91,255].join();
    const anchorX = 64 + 632 / 3;
    const label = [...svg.querySelectorAll('text')].find(node =>
      node.textContent === '5' && close(+node.getAttribute('x'), anchorX));
    const stroke = pixel(point.x, point.y), hollow = pixel(middle.x, middle.y);
    const visibleEdge = pixel(edge.left, edge.y), clippedEdge = pixel(edge.x, edge.y);
    const expected = nodes.length === 4 && nodes.every(node =>
      node.getAttribute('fill') === 'none' && node.getAttribute('stroke-width') === '3') &&
      +nodes[1].dataset.value === 5 && close(middle.x, anchorX+16) && close(middle.y, 190) &&
      label && close(+label.getAttribute('y'), 173) &&
      nodes[1].isPointInStroke(point) && !nodes[1].isPointInStroke(new DOMPoint(middle.x,middle.y)) &&
      stroke.join() === color && hollow.join() === '255,255,255,255' &&
      close(edge.x, 702) && close(edge.y, 95.2) && edge.left < 696 && edge.right > 696 &&
      nodes[3].ownerSVGElement.getAttribute('overflow') === 'hidden' &&
      visibleEdge.join() === color && clippedEdge.join() === '255,255,255,255';
    if (!expected) throw Error(`Custom Line contour, label anchor or crop failed: ${JSON.stringify({
      middle,edge,stroke,hollow,visibleEdge,clippedEdge,
    })}`);
    return {anchorX, middle, edge, stroke, hollow, visibleEdge, clippedEdge};
  });
})
