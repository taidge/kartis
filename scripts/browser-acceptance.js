// A reusable async Playwright page function; also accepts an optional gallery URL.
(async (page, url = 'http://127.0.0.1:8811') => {
  const failures = [];
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.goto(url);
  await page.waitForSelector('article canvas', {state:'attached'});
  const charts = await page.evaluate(async () => {
    const output = [];
    for (const card of document.querySelectorAll('article')) {
      const liveSvg = card.querySelector('svg'), svg = liveSvg.cloneNode(true), canvas = card.querySelector('canvas');
      // Standalone SVG images do not retain the live tree's inherited language.
      const languageOwner=liveSvg.closest('[lang]');
      let language;
      for(let node=liveSvg;node;node=node.parentElement){
        const xmlLanguage=node.getAttributeNS('http://www.w3.org/XML/1998/namespace','lang')??node.getAttribute('xml:lang');
        if(xmlLanguage!==null){language=xmlLanguage;break;}
        if(node===languageOwner){language=node.getAttribute('lang');break;}
      }
      svg.setAttribute('lang',language??liveSvg.ownerDocument.documentElement.lang);
      const source = canvas.getContext('2d', { willReadFrequently: true });
      const pixels = source.getImageData(0, 0, canvas.width, canvas.height).data;
      let colored = 0;
      for (let i = 0; i < pixels.length; i += 4) if (pixels[i+3] && Math.min(pixels[i], pixels[i+1], pixels[i+2]) < 235) colored++;
      const raster = document.createElement('canvas');
      raster.width = canvas.width; raster.height = canvas.height;
      const context = raster.getContext('2d', { willReadFrequently: true });
      const image = new Image();
      const objectURL = URL.createObjectURL(new Blob([new XMLSerializer().serializeToString(svg)], {type:'image/svg+xml'}));
      try {
        await new Promise((resolve, reject) => { image.onload = resolve; image.onerror = reject; image.src = objectURL; });
        context.drawImage(image, 0, 0, canvas.width, canvas.height);
      } finally { URL.revokeObjectURL(objectURL); }
      const a = context.getImageData(0, 0, canvas.width, canvas.height).data;
      let difference = 0;
      for (let i = 0; i < a.length; i++) difference += Math.abs(a[i] - pixels[i]);
      const button = card.querySelector('.toggle');
      button.click(); const canvasVisible = getComputedStyle(card.querySelector('.canvas-view')).display !== 'none';
      button.click(); const svgVisible = getComputedStyle(card.querySelector('.svg-view')).display !== 'none';
      const name = card.querySelector('h2').textContent;
      const record = {name, width:canvas.width, height:canvas.height, coloredPixels:colored, meanChannelError:difference/a.length, toggle:canvasVisible && svgVisible};
      if (name === 'Area · isolated multiply blending') {
        const areas = [...liveSvg.querySelectorAll('path[data-id^="area-"]')];
        const sample = (data, x, y) => {
          const offset = 4 * (Math.floor(y * canvas.height / 400) * canvas.width + Math.floor(x * canvas.width / 720));
          return [...data.slice(offset, offset + 4)];
        };
        const background = [250, 247, 237, 255], first = [39, 113, 181], second = [220, 103, 73];
        const alpha = .82 * 155 / 255;
        const expected = background.slice(0, 3).map((value, i) => Math.round(value * (1 - alpha + alpha * first[i] / 255) * (1 - alpha + alpha * second[i] / 255)));
        expected.push(255);
        const actualSvg = sample(a, 350, 276), actualCanvas = sample(pixels, 350, 276);
        const equal = (left, right, tolerance) => left.every((value, i) => Math.abs(value - right[i]) <= tolerance);
        record.blend = {
          groups: liveSvg.style.isolation === 'isolate' && areas.length === 2 && areas.every(path => path.closest('g').style.mixBlendMode === 'multiply' && path.getAttribute('opacity') === '0.82' && path.getAttribute('stroke-width') === '2'),
          overlap: equal(actualSvg, expected, 2) && equal(actualCanvas, expected, 2),
          backdrop: equal(sample(a, 20, 20), background, 0) && equal(sample(pixels, 20, 20), background, 0),
          stateRestored: source.globalCompositeOperation === 'source-over' && source.globalAlpha === 1,
          expected, actualSvg, actualCanvas,
        };
      }
      output.push(record);
    }
    return output;
  });
  if (charts.length !== 248) failures.push(`Expected 248 examples, got ${charts.length}`);
  for (const name of ['Chord · directional ribbon values', 'Chord · changed dominant source', 'Bump · distinct rank steps and gaps',
    'Bump · offset datum diamonds', 'Stream · InsideOut and Wiggle', 'Stream · end-boundary dots', 'Stream · signed diverging layers',
    'Stream · shared star dots', 'Stream · offset custom dots', 'Pie · ring-thickness label gate',
    'RadialBar · ring-thickness labels', 'Sunburst · measured label placement', 'Sunburst · measured arc masks',
    'Color fonts · COLR palette layers', 'Color fonts · transparent foreground',
    'Typography · authored static 400/700 faces', 'Typography · explicit family fallback',
    'Heatmap · sample quantiles', 'Heatmap · explicit thresholds',
    'Scales · signed square and square root', 'Scales · reciprocal branch and square root',
    'Heatmap · logarithmic colors and legend', 'Heatmap · symmetric-log diverging center',
    'Line · full-breakpoint value axes', 'Scatter · piecewise log and symlog axes',
    'Geo · fitted inverse readback', 'Geo · globe inverse readback']) {
    if (!charts.some(chart => chart.name === name)) failures.push(`Missing flow layout example: ${name}`);
  }
  for (const name of ['Funnel · spaced and highlighted stages', 'Waffle · rounded cells', 'Waffle · contiguous areas',
    'Sunburst · equal-area radial partition', 'Parallel · rounded, logarithmic and UTC axes']) {
    if (!charts.some(chart => chart.name === name)) failures.push(`Missing typed layout example: ${name}`);
  }
  const discreteColorGeometry = await page.evaluate(() => {
    const values=[0,0,1,1,2,3,8,15,40,100,null];
    const colors=['rgb(42,91,151)','rgb(53,153,150)','rgb(153,199,122)','rgb(239,183,71)'];
    return [
      ['Heatmap · sample quantiles',[1,2.5,13.25]],
      ['Heatmap · explicit thresholds',[1,5,20]],
    ].map(([name,cuts])=>{
      const card=[...document.querySelectorAll('article')].find(card=>card.querySelector('h2').textContent===name);
      const svg=card?.querySelector('svg');
      if(!svg)return{name,valid:false};
      const cells=[...svg.querySelectorAll('[data-id^="heatmap-cell:"]')].sort((a,b)=>a.getBBox().x-b.getBBox().x);
      const paint=cells.map(cell=>getComputedStyle(cell).fill.replaceAll(' ',''));
      const expected=values.map(value=>value===null?'rgb(238,238,238)':colors[cuts.filter(cut=>cut<=value).length]);
      const bands=[...svg.querySelectorAll('[data-id]')].filter(mark=>mark.dataset.id.includes(':band:'));
      const widths=bands.map(mark=>mark.getBBox().width);
      const boundaries=[0,...cuts,100];
      const expectedWidths=cuts.concat(100).map((end,i)=>(end-boundaries[i])*4.7);
      return{name,paint,widths,valid:paint.length===11&&paint.every((color,i)=>color===expected[i])&&widths.length===4&&widths.every((width,i)=>Math.abs(width-expectedWidths[i])<.001)&&svg.querySelectorAll('linearGradient').length===0};
    });
  });
  for(const result of discreteColorGeometry)if(!result.valid)failures.push(`Discrete color geometry failed: ${JSON.stringify(result)}`);
  const blend = charts.find(chart => chart.name === 'Area · isolated multiply blending')?.blend;
  if (!blend?.groups || !blend.overlap || !blend.backdrop || !blend.stateRestored) failures.push(`Isolated Multiply paint failed: ${JSON.stringify(blend)}`);
  for(const name of ['Scatter: default nine-pixel circles','Scatter: dynamic diameter extrapolation','Scatter: filled Star area and origins','Scatter: hollow Triangle centers','Scatter: UTC and reversed value scale','Scatter: typed point X and Y'])if(!charts.some(c=>c.name===name))failures.push(`Missing typed Scatter example: ${name}`);
  for(const name of ['Scales · reversed rounded clamped line','Scales · base-2 log and value-domain SymLog','Scales · UTC calendar nice and reverse','Point · typed categories and gaps','Point · ordinal affine window','Glyphs · seven equal-area filled symbols','Glyphs · seven stroke-only centerlines','Glyphs · rotated true anchors','Radar · filled Wye glyphs','Radar · hollow triangle glyphs','Geographic Star glyphs with shared legends','Clipped globe: stroked glyphs'])if(!charts.some(c=>c.name===name))failures.push(`Missing scale/glyph profile: ${name}`);
  for(const name of ["Pie · D3 padded donut snapshot", "Pie · partial sector fit snapshot", "Pie · zero-value border snapshot", "Line · point crosshair snapshot", "Line · SliceX crosshair snapshot", "Line · SliceY crosshair snapshot", "Radar · clockwise slice snapshot", "Radar · counterclockwise slice snapshot", "Radar · zero-value hidden slice snapshot", "Tree · ancestor relation snapshot", "Network · neighbor relation snapshot", "Chord · incident relation snapshot", "Sankey · parallel relation snapshot"])if(!charts.some(c=>c.name===name))failures.push(`Missing interaction snapshot: ${name}`);
  for(const name of ['Marimekko independent thickness','Marimekko horizontal expanded dimensions','Marimekko signed diverging dimensions','Marimekko niced thickness scale','Delaunay triangles and neighbors','Delaunay duplicates and cocircular sites','Delaunay exact collinear chain','Adaptive geographic routes and graticule','Transverse Mercator shared overlays','Orthographic clipped route streams','Adaptive great-circle data fit'])if(!charts.some(c=>c.name===name))failures.push(`Missing independent-layout example: ${name}`);
  for(const name of ['AreaBump variable-width smooth bands','AreaBump sparse linear gaps','AreaBump controlled series focus','Compact calendar across New Year','Vertical leap-day calendar','Styled full-year calendar panels','Grouped sample quantile box plots','Horizontal Tukey boxes and outliers','Bullet rows with a shared signed domain','Vertical reversed bullet rows','Treemap with reserved parent headers','Circle packing focused subtree','Icicle rightward hierarchy','Icicle leftward focused subtree'])if(!charts.some(c=>c.name===name))failures.push(`Missing P1 layout example: ${name}`);
  for(const name of ['Bar independent padding and totals','Signed bar totals and category colors','Percent bar raw totals and window','Heatmap diverging colors and labels','Heatmap circles and size variation','Heatmap row-column focus','Stacked radial bars with axes','Polar bars with calibrated axes','Choropleth shared diverging scale','Calendar shared quantized scale'])if(!charts.some(c=>c.name===name))failures.push(`Missing data-style example: ${name}`);
  for(const name of ['Rounded pie with link labels','Active donut radius offsets','Counterclockwise rounded pie','Smooth radar with point labels','Radar circular grid and symbols','Styled Sankey horizontal','Styled Sankey vertical','Styled circular Sankey horizontal','Styled circular Sankey vertical','Styled line and area','Line invisible interaction sites'])if(!charts.some(c=>c.name===name))failures.push(`Missing presentation example: ${name}`);
  for(const name of ['Tree step links and node styles','Radial tree link styles','Tree straight links','Parallel coordinates independent scales','Parallel coordinates gaps and horizontal axes','Network before drag','Network after drag'])if(!charts.some(c=>c.name===name))failures.push(`Missing configurable example: ${name}`);
  for(const name of ['Repeating pattern fills','Dashed curves and cap shapes','Even-odd path masks','Box plot and line combination','Range band and error bars','Projection mask with symbol stroke'])if(!charts.some(c=>c.name===name))failures.push(`Missing styled example: ${name}`);
  for(const name of ['Symbols clipped to projection','Contour morph initial','Contour morph keyed holes','Contour morph exact target','Gradient rounded bars','Radial gradient fills','Nonconvex horizon clipping','Gnomonic small-circle clip','Circular Sankey','Category window bars','Category window combo','Azimuthal equal-area symbols','Azimuthal equidistant symbols','Stereographic cross symbols','Morph initial','Morph sampled transition','Morph exact target','Measured label collision placement','Local calendar day ticks','Local calendar month ticks','Chinese date labels','Fixed-offset clock','Viewport original domains','Viewport relayout domains','Open spline comparison','Cardinal tension comparison','Periodic closed splines','Long closed area','Bump X and Y','Monotone Y curve','Geographic symbol legends','Globe symbol legends','Mixed Cartesian chart','Resquarify initial','Resquarify updated','Rotated category labels','Basis curve','Catmull-Rom curve','Bundle curve','Top and right axes','Locale number format','Dual value axes','Sankey left alignment','Sankey right alignment','Sankey center alignment','Sankey justify alignment','Logarithmic axes','UTC time axis','Clipped domains','Wiggle stream','Squarify','Tidy radial tree','Voronoi mesh','Custom layers','Equal Earth map','Natural Earth map','Orthographic map','Antimeridian map','Multi-year calendar','Barnes-Hut network'])if(!charts.some(c=>c.name===name))failures.push(`Missing upgraded example: ${name}`);
  const upgraded = await page.evaluate(() => {
    const svg = name => [...document.querySelectorAll('article')].find(card => card.querySelector('h2').textContent === name).querySelector('svg');
    const lines = name => [...svg(name).querySelectorAll('path[data-id]')].filter(p => p.dataset.id.includes('line-'));
    const long = svg('Long closed area');
    const fills = [...long.querySelectorAll('path[data-id^="area-"]')];
    const open = lines('Open spline comparison');
    const tensions = lines('Cardinal tension comparison').map(p => p.getAttribute('d'));
    const bumps = lines('Bump X and Y').map(p => p.getAttribute('d'));
    const monotone = lines('Monotone Y curve')[0].getAttribute('d').match(/[MLC][^MLCZ]+/g).map(command => command.slice(1).trim().split(/[ ,]+/).map(Number).slice(-2));
    const geo = svg('Geographic symbol legends'), globe = svg('Globe symbol legends');
    const symbols = root => [...root.querySelectorAll('path[data-id^="geo-symbol:"]')];
    const sizeSamples = root => [...root.querySelectorAll('path[data-id^="geo-size-legend:"]')];
    const swatches = root => [...root.querySelectorAll('path[data-id^="geo-color-legend:"]')];
    const sizes = sizeSamples(geo).map(p => p.getBBox().width / 2);
    const localized = svg('Chinese date labels'), clock = svg('Fixed-offset clock');
    const labels = root => [...root.querySelectorAll('text')].map(el => el.textContent);
    const before = svg('Viewport original domains'), after = svg('Viewport relayout domains');
    const datum = root => root.querySelector('path[data-id="point-0-3"]');
    const oldPoint = datum(before).getBBox(), newPoint = datum(after).getBBox();
    const beforeClip = datum(before).parentElement.getAttribute('viewBox'), afterClip = datum(after).parentElement.getAttribute('viewBox');
    const [plotX,,plotWidth] = beforeClip.split(' ').map(Number);
    const oldX=oldPoint.x+oldPoint.width/2, newX=newPoint.x+newPoint.width/2;
    const rounded=svg('Rounded pie with link labels'),active=svg('Active donut radius offsets'),ccw=svg('Counterclockwise rounded pie');
    const slice=(root,id)=>root.querySelector(`path[data-id="${id}"]`);
    const winding=path=>{const length=path.getTotalLength(),points=Array.from({length:256},(_,i)=>path.getPointAtLength(length*i/256));return points.reduce((sum,p,i)=>{const q=points[(i+1)%points.length];return sum+p.x*q.y-q.x*p.y},0)};
    const outerRadius=path=>{const length=path.getTotalLength();return Math.max(...Array.from({length:256},(_,i)=>{const p=path.getPointAtLength(length*i/256);return Math.hypot(p.x-360,p.y-200)}))};
    const radarSmooth=svg('Smooth radar with point labels'),radarCircle=svg('Radar circular grid and symbols');
    const flowNodes=root=>[...root.querySelectorAll('path[data-id^="sankey-node:"]')];
    const flowProfiles=[['Styled Sankey horizontal',false,false],['Styled Sankey vertical',true,false],['Styled circular Sankey horizontal',false,true],['Styled circular Sankey vertical',true,true]];
    const styledArea=svg('Styled line and area'),hiddenSites=svg('Line invisible interaction sites');
    const invisiblePoints=[...hiddenSites.querySelectorAll('path[data-id^="line-point:"]')];
    // WebKit's f32 cubic bounding-box calculation can report a one-ulp height
    // for an exactly degenerate path. Check the actual exported coordinates
    // and zero arclength instead of requiring a browser's box to be exact.
    const zeroSite=path=>{const d=path.getAttribute('d'),commands=(d.match(/[MLCZ]/g)||[]).join(''),coordinates=(d.match(/[-+]?(?:\d*\.?\d+)(?:e[-+]?\d+)?/gi)||[]).map(Number);return commands==='MCCCCZ'&&coordinates.length===26&&coordinates.every((v,i)=>Number.isFinite(v)&&v===coordinates[i%2])&&path.getTotalLength()===0&&path.getAttribute('fill')==='rgba(0,0,0,0)'&&!path.hasAttribute('stroke')};
    const grouped=svg('Bar independent padding and totals'),signedBars=svg('Signed bar totals and category colors'),percentBars=svg('Percent bar raw totals and window');
    const heat=svg('Heatmap diverging colors and labels'),circles=svg('Heatmap circles and size variation'),focus=svg('Heatmap row-column focus'),mapScale=svg('Choropleth shared diverging scale'),calendarScale=svg('Calendar shared quantized scale');
    const heatCells=root=>[...root.querySelectorAll('path[data-id^="heatmap-cell:"]')];
    const firstPercent=percentBars.querySelector('[data-id="styled-bar:5:North:3:Jan"]');
    const polarTicks=svg('Polar bars with calibrated axes'),radius20=polarTicks.querySelector('[data-id="gallery-polar:grid-radius:16:4034000000000000"]').getBBox().width/2;
    const bumpSmooth=svg('AreaBump variable-width smooth bands'),bumpGaps=svg('AreaBump sparse linear gaps'),bumpFocus=svg('AreaBump controlled series focus');
    const compactCalendar=svg('Compact calendar across New Year'),verticalCalendar=svg('Vertical leap-day calendar'),yearCalendar=svg('Styled full-year calendar panels');
    const sampleBoxes=svg('Grouped sample quantile box plots'),tukeyBoxes=svg('Horizontal Tukey boxes and outliers'),sharedBullets=svg('Bullet rows with a shared signed domain'),reverseBullets=svg('Vertical reversed bullet rows');
    const headedTreemap=svg('Treemap with reserved parent headers'),focusedPacking=svg('Circle packing focused subtree'),rightIcicle=svg('Icicle rightward hierarchy'),leftIcicle=svg('Icicle leftward focused subtree');
    const paths=(root,prefix)=>[...root.querySelectorAll('path[data-id]')].filter(p=>p.dataset.id.startsWith(prefix));
    const datePaths=root=>paths(root,'').filter(p=>/^\d{4}-\d\d-\d\d$/.test(p.dataset.id));
    const datumBox=(root,id)=>[...root.querySelectorAll('path[data-id]')].find(p=>p.dataset.id===id).getBBox();
    const gapSearch=paths(bumpGaps,'area-bump:6:Search:gap:').map(p=>p.getBBox());
    const opacityOf=p=>{
      if(p.hasAttribute('opacity')&&p.hasAttribute('fill-opacity'))return NaN;
      if(p.hasAttribute('fill-opacity')&&(p.hasAttribute('stroke')||p.getAttribute('fill')==='none'||p.getAttribute('fill')?.startsWith('url(')))return NaN;
      return Number(p.getAttribute('opacity')??p.getAttribute('fill-opacity')??1);
    };
    const boxParts=root=>paths(root,'box:').filter(p=>p.dataset.id.split(':').length===5);
    return {
      isolatedInlinePaintDefinitions:(()=>{const ids=[...document.querySelectorAll('article svg defs [id]')].map(el=>el.id);return new Set(ids).size===ids.length})(),
      rankBandSharedScale:paths(bumpSmooth,'area-bump:').length===3&&paths(bumpSmooth,'area-bump:').every(p=>(p.getAttribute('d').match(/C/g)||[]).length>8)&&paths(bumpSmooth,'area-bump-grid:').length===5&&labels(bumpSmooth).includes('Q5'),
      rankBandMissingTopology:gapSearch.length===2&&gapSearch[0].x+gapSearch[0].width<gapSearch[1].x&&paths(bumpGaps,'area-bump:').every(p=>!p.getAttribute('d').includes('C')),
      rankBandControlledPaint:paths(bumpFocus,'area-bump:').length===3&&bumpFocus.querySelector('[data-id="area-bump:6:Direct"]').getAttribute('stroke-width')==='2'&&paths(bumpFocus,'area-bump:').filter(p=>p.dataset.id!=='area-bump:6:Direct').every(p=>!p.hasAttribute('stroke')),
      compactCivilDateRange:datePaths(compactCalendar).length===18&&paths(compactCalendar,'calendar-month:').length===2&&!!compactCalendar.querySelector('[data-id="2023-12-24"]')&&!!compactCalendar.querySelector('[data-id="2024-01-10"]')&&compactCalendar.querySelector('[data-id="2024-01-04"]').getAttribute('fill')!==compactCalendar.querySelector('[data-id="2023-12-24"]').getAttribute('fill'),
      verticalLeapDayCells:datePaths(verticalCalendar).length===14&&!!verticalCalendar.querySelector('[data-id="2024-02-29"]')&&datePaths(verticalCalendar).every(p=>(p.getAttribute('d').match(/C/g)||[]).length===4)&&paths(verticalCalendar,'calendar-month:').length===2,
      styledFullYearOutlines:datePaths(yearCalendar).length===731&&paths(yearCalendar,'calendar-month:').length===24&&labels(yearCalendar).includes('2023')&&labels(yearCalendar).includes('2024'),
      sampleStatisticsRealParts:boxParts(sampleBoxes).length===6&&paths(sampleBoxes,'box:').filter(p=>p.dataset.id.endsWith(':median')).length===6&&paths(sampleBoxes,'box:').filter(p=>p.dataset.id.includes(':outlier:')).length===12&&boxParts(sampleBoxes).every(p=>opacityOf(p)===0.4),
      tukeyStylesAndSamples:boxParts(tukeyBoxes).length===6&&boxParts(tukeyBoxes).every(p=>(p.getAttribute('d').match(/C/g)||[]).length===4)&&paths(tukeyBoxes,'box:').filter(p=>p.dataset.id.includes(':outlier:')).length===6&&paths(tukeyBoxes,'box:').filter(p=>p.dataset.id.endsWith(':border')).every(p=>p.getAttribute('opacity')==='1'&&p.getAttribute('fill')==='none'),
      bulletSharedSignedIntervals:paths(sharedBullets,'bullet:').filter(p=>p.dataset.id.includes(':measure:')).length===6&&paths(sharedBullets,'bullet:').filter(p=>p.dataset.id.includes(':marker:')).length===3&&labels(sharedBullets).includes('-10')&&labels(sharedBullets).includes('Online'),
      bulletReverseVerticalProjection:paths(reverseBullets,'bullet:').filter(p=>p.dataset.id.includes(':marker:')).length===3&&datumBox(reverseBullets,'bullet:5:North:marker:0').y<datumBox(reverseBullets,'bullet:5:South:marker:0').y&&Math.abs(datumBox(reverseBullets,'bullet:5:North:measure:0').y-75)<1e-8,
      treemapReservedHeaderGeometry:paths(headedTreemap,'node:').length===11&&datumBox(headedTreemap,'node:Cloud').y>=datumBox(headedTreemap,'node:Portfolio').y+25&&headedTreemap.querySelectorAll('clipPath').length>=11,
      packingFocusActualSubtree:paths(focusedPacking,'node:').length===4&&!focusedPacking.querySelector('[data-id="node:Devices"]')&&paths(focusedPacking,'node:').every(p=>(p.getAttribute('d').match(/C/g)||[]).length===4)&&labels(focusedPacking).includes('Compute'),
      icicleDataFocusAndDirections:paths(rightIcicle,'node:').length===11&&paths(leftIcicle,'node:').length===4&&datumBox(rightIcicle,'node:Compute').x>datumBox(rightIcicle,'node:Cloud').x&&datumBox(leftIcicle,'node:Compute').x<datumBox(leftIcicle,'node:Cloud').x&&!leftIcicle.querySelector('[data-id="node:Devices"]'),
      roundedPie:rounded.querySelectorAll('path[data-id^="pie-leader:"]').length===4&&(slice(rounded,'Search').getAttribute('d').match(/C/g)||[]).length>=6&&[...rounded.querySelectorAll('text')].some(t=>t.textContent==='42%'),
      activePieRadius:Math.abs(outerRadius(slice(active,'Partners'))-outerRadius(slice(rounded,'Partners'))-16)<.1,
      counterclockwiseWinding:winding(slice(rounded,'Search'))>0&&winding(slice(ccw,'Search'))<0,
      configuredRadar:radarSmooth.querySelectorAll('path[data-id^="radar-point:"]').length===10&&radarSmooth.querySelectorAll('path[data-id^="radar-spoke:"]').length===5&&[...radarSmooth.querySelectorAll('path[data-id^="radar-series:"]')].every(p=>(p.getAttribute('d').match(/C/g)||[]).length===5&&p.getAttribute('opacity')==='1')&&[...radarCircle.querySelectorAll('path[data-id^="radar-grid:"]')].every(p=>(p.getAttribute('d').match(/C/g)||[]).length===4),
      styledSankeyOrientations:flowProfiles.every(([name,vertical,cyclic])=>{const root=svg(name),nodes=flowNodes(root);return nodes.length===5&&nodes.every(p=>{const b=p.getBBox();return Math.abs((vertical?b.height:b.width)-19)<.001&&(p.getAttribute('d').match(/C/g)||[]).length===4})&&root.querySelectorAll('linearGradient').length>=5&&root.querySelectorAll('path[data-id^="sankey-link:"]').length===(cyclic?8:5)&&(!cyclic||(root.querySelector('path[data-id="sankey-link:8:feedback"]').getAttribute('d').match(/L/g)||[]).length>=5)}),
      styledAreaAndGap:styledArea.querySelectorAll('linearGradient').length===1&&styledArea.querySelectorAll('path[data-id^="line-area:8:Observed:"]').length===2&&[...styledArea.querySelectorAll('path[data-id^="line-stroke:9:Reference:"]')].every(p=>p.getAttribute('stroke-dasharray')==='10 5'),
      invisibleMeshSites:invisiblePoints.length===17&&invisiblePoints.every(zeroSite)&&hiddenSites.querySelectorAll('path[data-id^="line-stroke:8:Observed:"]').length===1&&!!hiddenSites.querySelector('path[data-id="gallery-nearest-found"]'),
      groupedBarLabelsAndTotals:grouped.querySelectorAll('path[data-id^="styled-bar:"]').length===12&&labels(grouped).includes('Σ125')&&labels(grouped).includes('Σ58'),
      signedBarCategoryColors:[...signedBars.querySelectorAll('path[data-id^="styled-bar:"]')].filter(p=>p.dataset.id.endsWith(':6:Retail')).every((p,_,items)=>p.getAttribute('fill')===items[0].getAttribute('fill'))&&labels(signedBars).includes('Σ-15')&&labels(signedBars).includes('Σ73'),
      percentRawTotalsAndTrueCrop:firstPercent.getBBox().x<82&&firstPercent.parentElement.getAttribute('viewBox')==='82 64 583 284'&&labels(percentBars).includes('Σ57')&&labels(percentBars).includes('100%'),
      nullableDivergingHeatmap:heatCells(heat).length===24&&heat.querySelector('[data-id="heatmap-cell:5:South:3:Feb"]').getAttribute('fill')!==heat.querySelector('[data-id="heatmap-cell:5:North:3:Feb"]').getAttribute('fill')&&labels(heat).includes('—')&&heat.querySelectorAll('linearGradient').length>0,
      sizeVariationHeatmap:heatCells(circles).length===35&&heatCells(circles).every(p=>(p.getAttribute('d').match(/C/g)||[]).length===4)&&new Set(heatCells(circles).map(p=>p.getBBox().width.toFixed(3))).size>5,
      exactRowColumnFocus:heatCells(focus).filter(p=>Number(p.getAttribute('opacity'))===1).length===9&&heatCells(focus).filter(p=>Number(p.getAttribute('opacity'))===.15).length===15,
      stackedRingsAndCalibratedPolar:svg('Stacked radial bars with axes').querySelectorAll('path[data-id^="radial-segment:"]').length===9&&svg('Stacked radial bars with axes').querySelectorAll('path[data-id^="radial-track:"]').length===3&&Math.abs(radius20-Math.sqrt((143*.18)**2+(143**2-(143*.18)**2)*20/65))<.001,
      sharedMapMidpoint:mapScale.querySelectorAll('path[data-id^="map:"]').length===6&&mapScale.querySelector('[data-id="map:Southwest"]').getAttribute('fill')===heat.querySelector('[data-id="heatmap-cell:5:South:3:Feb"]').getAttribute('fill')&&mapScale.querySelector('[data-id="map:Unknown"]').getAttribute('fill')!==mapScale.querySelector('[data-id="map:Southwest"]').getAttribute('fill'),
      calendarQuantizeAndMissing:[...calendarScale.querySelectorAll('path[data-id]')].filter(p=>/^\d{4}-\d\d-\d\d$/.test(p.dataset.id)).length===731&&!!calendarScale.querySelector('[data-id="2024-02-29"]')&&calendarScale.querySelectorAll('path[data-id^="color-legend:13:calendar-bins:band:"]').length===5&&calendarScale.querySelectorAll('linearGradient').length===0&&calendarScale.querySelector('[data-id="2023-01-01"]').getAttribute('fill')!==calendarScale.querySelector('[data-id="2023-04-12"]').getAttribute('fill'),
      gradientBars:svg('Gradient rounded bars').querySelectorAll('linearGradient').length===1&&[...svg('Gradient rounded bars').querySelectorAll('path[data-series="Volume"]')].every(p=>(p.getAttribute('d').match(/C/g)||[]).length===4),
      radialPaint:svg('Radial gradient fills').querySelectorAll('radialGradient').length===3,
      contourHoles:[...svg('Contour morph keyed holes').querySelectorAll('path[data-series="donut"]')].some(p=>(p.getAttribute('d').match(/M/g)||[]).length===2&&(p.getAttribute('d').match(/L/g)||[]).length===190),
      nonconvexRejoin:[...svg('Nonconvex horizon clipping').querySelectorAll('path[data-id]')].some(p=>(p.getAttribute('d').match(/M/g)||[]).length>=2),
      longContour: fills.length === 1 && (fills[0].getAttribute('d').match(/C/g) || []).length === 700 && fills[0].getAttribute('d').endsWith('Z'),
      openInteriorEndpoints: open.length === 3 && open.every(p => (p.getAttribute('d').match(/C/g) || []).length === 5),
      distinctTensions: tensions.length === 3 && new Set(tensions).size === 3,
      distinctBumps: bumps.length === 2 && new Set(bumps).size === 2,
      monotoneY: monotone.length === 6 && monotone.slice(1).every((p, i) => p[1] < monotone[i][1]) && monotone.some((p, i) => i > 0 && p[0] < monotone[i-1][0]) && monotone.some((p, i) => i > 0 && p[0] > monotone[i-1][0]),
      geographicLegends: symbols(geo).length === 7 && sizes.length === 3 && sizes.every((r, i) => Math.abs(r - [9,14.4,18][i]) < 0.001) && swatches(geo).length === 4,
      globeLegendsAndBackface: symbols(globe).length === 4 && sizeSamples(globe).length === 3 && swatches(globe).length === 2 && !symbols(globe).some(p => p.dataset.id.endsWith(':Back face')),
      chineseLeapDay: ['2月28日 周三','2月29日 周四','3月1日 周五','3月2日 周六'].every(text => labels(localized).includes(text)),
      fixedOffsetMidnight: ['02-29 20:00','02-29 23:00','03-01 02:00','03-01 05:00','03-01 08:00'].every(text => labels(clock).includes(text)),
      viewportFixedAxes: ['axis-x','axis-y'].every(id => before.querySelector(`path[data-id="${id}"]`).getAttribute('d') === after.querySelector(`path[data-id="${id}"]`).getAttribute('d')) && beforeClip === afterClip,
      viewportRelayoutGeometry: before.querySelectorAll('path[data-id^="point-"]').length === 11 && after.querySelectorAll('path[data-id^="point-"]').length === 5 && Math.abs(newX-(plotX+2*(oldX-plotX-plotWidth/4))) < 0.001 && Math.abs(oldPoint.y-newPoint.y) < 0.001 && Math.abs(oldPoint.width-newPoint.width) < 0.001,
    };
  });
  for(const [name, passed] of Object.entries(upgraded)) if(!passed) failures.push(`Upgraded geometry failed: ${name}`);
  const p2Geometry=await page.evaluate(()=>{
    const svg=name=>[...document.querySelectorAll('article')].find(card=>card.querySelector('h2').textContent===name).querySelector('svg');
    const paths=(root,prefix)=>[...root.querySelectorAll('path[data-id]')].filter(p=>p.dataset.id.startsWith(prefix));
    const cell=(root,category,dimension='North')=>[...root.querySelectorAll('path[data-id]')].find(p=>p.dataset.id===`mekko:${dimension.length}:${dimension}:${category.length}:${category}`);
    const mekko=svg('Marimekko independent thickness'),expanded=svg('Marimekko horizontal expanded dimensions'),signed=svg('Marimekko signed diverging dimensions'),niced=svg('Marimekko niced thickness scale');
    const boxes=['Services','Products','Support'].map(c=>cell(mekko,c).getBBox());
    const triangles=svg('Delaunay triangles and neighbors'),duplicates=svg('Delaunay duplicates and cocircular sites'),chain=svg('Delaunay exact collinear chain');
    const map=svg('Adaptive geographic routes and graticule'),transverse=svg('Transverse Mercator shared overlays'),globe=svg('Orthographic clipped route streams'),fitted=svg('Adaptive great-circle data fit');
    const startMatchesSymbol=root=>{const path=paths(root,'geo-line:Atlantic')[0],symbol=paths(root,'geo-symbol:').find(p=>p.dataset.id.endsWith(':New York'));if(!path||!symbol)return false;const p=path.getPointAtLength(0),b=symbol.getBBox();return Math.hypot(p.x-(b.x+b.width/2),p.y-(b.y+b.height/2))<.01;};
    const signedCells=paths(signed,'mekko:').filter(p=>!p.dataset.id.endsWith(':border'));
    const fittedBox=paths(fitted,'geo-line:High latitude route')[0].getBBox();
    return {
      independentThicknessRatios:Math.abs(boxes[1].width/boxes[0].width-3)<.001&&Math.abs(boxes[2].width/boxes[0].width-2)<.001&&Math.abs(boxes[0].height-206.25)<.01,
      expandedHorizontalEndpoints:['Services','Products','Support'].every(category=>{const p=paths(expanded,'mekko:').filter(p=>!p.dataset.id.endsWith(':border')&&p.dataset.id.endsWith(`:${category}`)).map(p=>p.getBBox());return Math.abs(Math.max(...p.map(b=>b.x+b.width))-665)<.01;}),
      signedRawAndGeometry:signedCells.length===9&&signedCells.filter(p=>Number(p.dataset.value)<0).length===5&&signedCells.every(p=>{const b=p.getBBox();return Number(p.dataset.value)<0?b.y>=195-.01:b.y+b.height<=195+.01;}),
      nicedThicknessSpace:Math.abs((cell(niced,'Support').getBBox().x+cell(niced,'Support').getBBox().width)-(65+8+(600-16-20)*14.1/16+20))<.01,
      robustTriangulation:paths(triangles,'delaunay:site:').length===7&&paths(triangles,'delaunay:triangle:').length===6&&paths(triangles,'delaunay:link:').length===12,
      duplicateOwnership:paths(duplicates,'delaunay:site:').length===4&&paths(duplicates,'delaunay:triangle:').length===2&&paths(duplicates,'delaunay:link:').length===5,
      exactCollinearChain:paths(chain,'delaunay:site:').length===6&&paths(chain,'delaunay:triangle:').length===0&&paths(chain,'delaunay:link:').length===5,
      graticuleAndSeam:paths(map,'geo-line:graticule:').length===17&&(paths(map,'geo-line:Pacific seam')[0].getAttribute('d').match(/M/g)||[]).length===2,
      sharedProjectionEndpoints:[map,transverse,globe].every(startMatchesSymbol)&&paths(map,'geo-line:Atlantic')[0].getAttribute('d')!==paths(transverse,'geo-line:Atlantic')[0].getAttribute('d'),
      globeRouteMask:globe.querySelectorAll('clipPath').length>=1&&!!paths(globe,'geo-line:Atlantic')[0].closest('[clip-path]'),
      adaptiveDataFit:fittedBox.height>100&&fittedBox.x>=79.9&&fittedBox.y>=84.9&&fittedBox.x+fittedBox.width<=650.1&&fittedBox.y+fittedBox.height<=330.1,
    };
  });
  for(const [name,passed]of Object.entries(p2Geometry))if(!passed)failures.push(`Independent layout geometry failed: ${name}`);
  const interactionGeometry=await page.evaluate(()=>{
    const svg=name=>[...document.querySelectorAll('article')].find(c=>c.querySelector('h2').textContent===name).querySelector('svg');
    const paths=(root,prefix='')=>[...root.querySelectorAll('path[data-id]')].filter(p=>p.dataset.id.startsWith(prefix));
    const mark=(root,id)=>paths(root).find(p=>p.dataset.id===id);
    const close=(a,b,t=.01)=>Math.abs(a-b)<=t;
    const commands=path=>(path.getAttribute('d').match(/[MLC][^MLCZ]+/g)||[]).map(c=>(c.slice(1).match(/[-+]?(?:\d*\.?\d+)(?:e[-+]?\d+)?/gi)||[]).map(Number));
    const point=(x,y)=>new DOMPoint(x,y),center=[380,190],radius=134.3;
    const zeroSite=path=>{const d=path.getAttribute('d'),coords=(d.match(/[-+]?(?:\d*\.?\d+)(?:e[-+]?\d+)?/gi)||[]).map(Number);return(d.match(/[MLCZ]/g)||[]).join('')==='MCCCCZ'&&coords.length===26&&coords.every((v,i)=>Number.isFinite(v)&&v===coords[i%2])&&path.getTotalLength()===0&&path.getAttribute('fill')==='rgba(0,0,0,0)'&&!path.hasAttribute('stroke');};
    const circleCenter=path=>{const b=path.getBBox();return[b.x+b.width/2,b.y+b.height/2];};
    const maximumRadius=path=>{const length=path.getTotalLength();return Math.max(...Array.from({length:513},(_,i)=>{const p=path.getPointAtLength(length*i/512);return Math.hypot(p.x-center[0],p.y-center[1]);}));};
    const donut=svg('Pie · D3 padded donut snapshot'),partial=svg('Pie · partial sector fit snapshot'),zero=svg('Pie · zero-value border snapshot');
    const donutPaths=paths(donut),firstEnd=.34*(Math.PI*2-4*.075)+.075;
    const ringPoint=angle=>point(center[0]+123.24*Math.sin(angle),center[1]-123.24*Math.cos(angle));
    const partialBoxes=paths(partial).map(p=>p.getBBox()),partialExtent={x:Math.min(...partialBoxes.map(b=>b.x)),right:Math.max(...partialBoxes.map(b=>b.x+b.width)),y:Math.min(...partialBoxes.map(b=>b.y)),bottom:Math.max(...partialBoxes.map(b=>b.y+b.height))};
    const linePoint=svg('Line · point crosshair snapshot'),lineX=svg('Line · SliceX crosshair snapshot'),lineY=svg('Line · SliceY crosshair snapshot');
    const lineEnd=(root,id,a,b)=>{const p=mark(root,id),v=commands(p);return v.length===2&&v[0].length===2&&v[1].length===2&&v[0].every((n,i)=>close(n,a[i]))&&v[1].every((n,i)=>close(n,b[i]))&&p.getAttribute('stroke-dasharray')==='6 6'&&p.getAttribute('opacity')==='0.75'&&p.getAttribute('fill')==='none'&&p.getTotalLength()>300;};
    const clockwise=svg('Radar · clockwise slice snapshot'),counter=svg('Radar · counterclockwise slice snapshot'),hidden=svg('Radar · zero-value hidden slice snapshot');
    const spoke=(root,angle)=>{const p=mark(root,'kartis-crosshair:polar'),v=commands(p);return v.length===2&&v[0].every((n,i)=>close(n,center[i]))&&close(v[1][0],center[0]+radius*Math.sin(angle))&&close(v[1][1],center[1]-radius*Math.cos(angle))&&close(p.getTotalLength(),radius)&&p.getAttribute('fill')==='none';};
    const tree=svg('Tree · ancestor relation snapshot'),network=svg('Network · neighbor relation snapshot'),chord=svg('Chord · incident relation snapshot'),sankey=svg('Sankey · parallel relation snapshot');
    const opacityOf=p=>{
      if(p.hasAttribute('opacity')&&p.hasAttribute('fill-opacity'))return NaN;
      if(p.hasAttribute('fill-opacity')&&(p.hasAttribute('stroke')||p.getAttribute('fill')==='none'||p.getAttribute('fill')?.startsWith('url(')))return NaN;
      return Number(p.getAttribute('opacity')??p.getAttribute('fill-opacity')??1);
    };
    const opacity=(root,id,value)=>{const p=mark(root,id);return !!p&&close(opacityOf(p),value,1e-12);};
    const connected=(root,link,source,target)=>{const path=mark(root,link),a=circleCenter(mark(root,source)),b=circleCenter(mark(root,target)),start=path.getPointAtLength(0),end=path.getPointAtLength(path.getTotalLength());return path.getTotalLength()>10&&close(start.x,a[0])&&close(start.y,a[1])&&close(end.x,b[0])&&close(end.y,b[1]);};
    const main=commands(mark(sankey,'sankey-link:4:main')),parallel=commands(mark(sankey,'sankey-link:8:parallel'));
    const caption=root=>[...root.querySelectorAll('text')].some(t=>t.textContent.startsWith('State snapshot:'));
    return {
      d3PaddedDonutHoleAndActiveRadius:donutPaths.length===4&&donutPaths.every(p=>!p.isPointInFill(point(...center)))&&mark(donut,'North').isPointInFill(ringPoint(firstEnd/2))&&donutPaths.every(p=>!p.isPointInFill(ringPoint(firstEnd)))&&close(maximumRadius(mark(donut,'West')),168,.08)&&close(maximumRadius(mark(donut,'North')),158,.08),
      partialSectorActualFit:paths(partial).length===4&&partialExtent.x>=63.9&&partialExtent.right<=696.1&&partialExtent.y>=31.9&&partialExtent.bottom<=348.1&&partialExtent.right-partialExtent.x>560&&partialExtent.bottom-partialExtent.y>290,
      zeroValueRealRadialBorders:['Zero A','Zero B'].every(id=>{const p=mark(zero,id),v=commands(p),a=p.getPointAtLength(p.getTotalLength()/4),dx=v[1][0]-v[0][0],dy=v[1][1]-v[0][1],length=Math.hypot(dx,dy),off=point(a.x-2*dy/length,a.y+2*dx/length);return Number(p.dataset.value)===0&&(p.getAttribute('d').match(/[MLCZ]/g)||[]).join('')==='MLZ'&&v.length===2&&close(p.getTotalLength(),139.04,.02)&&p.getAttribute('stroke-width')==='3'&&p.isPointInStroke(a)&&!p.isPointInFill(off)&&!p.isPointInStroke(off);}),
      pointCrosshairActualTwoAxes:paths(linePoint,'kartis-crosshair:').length===2&&paths(linePoint,'line-point:').length===6&&paths(linePoint,'line-point:').every(zeroSite)&&lineEnd(linePoint,'kartis-crosshair:x',[380,32],[380,348])&&lineEnd(linePoint,'kartis-crosshair:y',[64,126.8],[696,126.8]),
      sliceXFullHeightOverride:paths(lineX,'kartis-crosshair:').length===1&&lineEnd(lineX,'kartis-crosshair:x',[380,32],[380,348])&&close(circleCenter(mark(lineX,'line-point:5:North:1'))[0],380)&&close(circleCenter(mark(lineX,'line-point:5:South:1'))[0],380),
      sliceYFullWidthOverride:paths(lineY,'kartis-crosshair:').length===1&&lineEnd(lineY,'kartis-crosshair:y',[64,126.8],[696,126.8])&&Number(mark(lineY,'line-point:5:North:1').dataset.value)===7&&Number(mark(lineY,'line-point:5:South:2').dataset.value)===7,
      clockwiseRealPolarSpoke:paths(clockwise,'radar-point:').length===15&&paths(clockwise,'radar-series:').length===3&&spoke(clockwise,Math.PI*2/5),
      counterclockwiseRotatedSpoke:paths(counter,'radar-point:').length===15&&paths(counter,'radar-series:').length===3&&spoke(counter,.55-Math.PI*2/5),
      zeroRadiusHiddenPolarSites:paths(hidden,'radar-point:').length===10&&paths(hidden,'radar-point:').every(zeroSite)&&paths(hidden,'radar-series:').length===2&&!hidden.querySelector('[data-series="South"]')&&Number(mark(hidden,'radar-point:5:North:5:Speed').dataset.value)===0&&Number(mark(hidden,'radar-point:6:Online:5:Speed').dataset.value)===0&&spoke(hidden,0),
      treeExplicitAncestorTopology:paths(tree,'node:').length===6&&paths(tree,'link:').length===5&&['Product','Cloud','API'].every(id=>opacity(tree,'node:'+id,1))&&['Devices','Storage','Phone'].every(id=>opacity(tree,'node:'+id,.2))&&['Cloud','API'].every(id=>opacity(tree,'link:'+id,1))&&['Devices','Storage','Phone'].every(id=>opacity(tree,'link:'+id,.1))&&connected(tree,'link:Cloud','node:Product','node:Cloud')&&connected(tree,'link:API','node:Cloud','node:API'),
      networkExplicitNeighborTopology:paths(network,'force-node:').length===6&&paths(network,'relation-network-link:').length===5&&['Core','Search','Storage','Web'].every(id=>opacity(network,'force-node:'+id,1))&&['Batch','Isolated'].every(id=>opacity(network,'force-node:'+id,.2))&&[['core-search','Core','Search'],['core-storage','Core','Storage'],['core-web','Core','Web'],['search-batch','Search','Batch'],['storage-web','Storage','Web']].every(([id,a,b],i)=>opacity(network,`relation-network-link:${id.length}:${id}`,i<3?1:.1)&&connected(network,`relation-network-link:${id.length}:${id}`,'force-node:'+a,'force-node:'+b)),
      chordIncidentRolesAndPaint:paths(chord,'relation-chord-node:').length===4&&paths(chord,'relation-chord-link:').length===6&&opacity(chord,'relation-chord-node:5:South',1)&&['5:North','6:Online','6:Retail'].every(id=>opacity(chord,'relation-chord-node:'+id,.2))&&['5:North:5:South','6:Online:5:South','6:Retail:5:South'].every(id=>opacity(chord,'relation-chord-link:'+id,.65))&&['5:North:6:Online','5:North:6:Retail','6:Online:6:Retail'].every(id=>opacity(chord,'relation-chord-link:'+id,.065))&&paths(chord,'relation-chord-link:').every(p=>p.getAttribute('d').endsWith('Z')&&p.getTotalLength()>200),
      sankeyParallelRolesPortsAndWidths:paths(sankey,'sankey-node:').length===4&&paths(sankey,'sankey-link:').length===5&&['6:Source','6:Branch'].every(id=>opacity(sankey,'sankey-node:'+id,1))&&['5:Other','4:Sink'].every(id=>opacity(sankey,'sankey-node:'+id,.2))&&['4:main','8:parallel'].every(id=>opacity(sankey,'sankey-link:'+id,.45))&&['10:branch-out','5:other','9:other-out'].every(id=>opacity(sankey,'sankey-link:'+id,.045))&&main.length===4&&parallel.length===4&&close(main[3][5],parallel[0][1])&&close(main[2][1],parallel[1][5])&&close((main[2][1]-main[1][5])/(parallel[2][1]-parallel[1][5]),2,1e-8)&&[donut,partial,zero,linePoint,lineX,lineY,clockwise,counter,hidden,tree,network,chord,sankey].every(caption),
    };
  });
  for(const[name,passed]of Object.entries(interactionGeometry))if(!passed)failures.push(`Interaction snapshot geometry failed: ${name}`);
  const scaleGlyphGeometry=await page.evaluate(()=>{
    const svg=name=>[...document.querySelectorAll('article')].find(c=>c.querySelector('h2').textContent===name).querySelector('svg');
    const paths=(root,prefix='')=>[...root.querySelectorAll('path[data-id]')].filter(p=>p.dataset.id.startsWith(prefix));
    const mark=(root,id)=>paths(root).find(p=>p.dataset.id===id);
    const close=(a,b,t=.01)=>Number.isFinite(a)&&Math.abs(a-b)<=t;
    const center=p=>{const b=p.getBBox();return[b.x+b.width/2,b.y+b.height/2];};
    const vertices=p=>(p.getAttribute('d').match(/[ML][^MLCZ]+/g)||[]).map(c=>(c.slice(1).match(/[-+]?(?:\d*\.?\d+)(?:e[-+]?\d+)?/gi)||[]).map(Number));
    const area=p=>{const v=vertices(p);return Math.abs(v.reduce((a,p,i)=>{const q=v[(i+1)%v.length];return a+p[0]*q[1]-p[1]*q[0];},0)/2);};
    const centroid=p=>{const v=vertices(p);return[0,1].map(k=>v.reduce((a,p)=>a+p[k],0)/v.length);};
    const texts=root=>[...root.querySelectorAll('text')].map(t=>t.textContent);
    const linear=svg('Scales · reversed rounded clamped line'),log=svg('Scales · base-2 log and value-domain SymLog'),utc=svg('Scales · UTC calendar nice and reverse');
    const full=svg('Point · typed categories and gaps'),windowed=svg('Point · ordinal affine window');
    const filled=svg('Glyphs · seven equal-area filled symbols'),stroke=svg('Glyphs · seven stroke-only centerlines'),rotated=svg('Glyphs · rotated true anchors');
    const wye=svg('Radar · filled Wye glyphs'),hollow=svg('Radar · hollow triangle glyphs');
    const stars=svg('Geographic Star glyphs with shared legends'),globe=svg('Clipped globe: stroked glyphs');
    const filledPaths=paths(filled,'gallery-glyph-fill:'),strokePaths=paths(stroke,'gallery-glyph-stroke:');
    const coord=(root,id,x,y)=>{const p=center(mark(root,id));return close(p[0],x)&&close(p[1],y);};
    const expectedLogX=70+620*(Math.log2(9)+1)/8,expectedSymY=335-300*(Math.log(4)+Math.log(11))/(Math.log(26)+Math.log(11));
    const plus=mark(stroke,'gallery-glyph-stroke:Plus'),plusRadius=Math.sqrt(398)*.87559;
    const radarHollow=paths(hollow,'radar-point:'),radarWye=paths(wye,'radar-point:');
    const mainStar=mark(stars,'geo-symbol:4:main'),smallStar=mark(stars,'geo-symbol:5:small'),missingStar=mark(stars,'geo-symbol:7:missing');
    const mainTriangle=mark(globe,'geo-symbol:4:main'),partialTriangle=mark(globe,'geo-symbol:7:partial'),partialBox=partialTriangle.getBBox();
    const clip=rootPath=>{const owner=rootPath.closest('[clip-path]');if(!owner)return null;const id=/url\(#([^)]*)\)/.exec(owner.getAttribute('clip-path'))?.[1];let svg=rootPath.ownerSVGElement;while(svg?.ownerSVGElement)svg=svg.ownerSVGElement;return id?svg.querySelector(`[id="${id}"]`):null;};
    // Firefox can return an empty box for non-rendered geometry in <defs>.
    // Measure the exact same path in its real outer SVG for every engine;
    // visibility:hidden keeps it unpainted while retaining geometry layout.
    const renderedBounds=path=>{if(!path)return null;let outer=path.ownerSVGElement;while(outer?.ownerSVGElement)outer=outer.ownerSVGElement;const clone=path.cloneNode(true);for(const name of['id','data-id','clip-path'])clone.removeAttribute(name);clone.setAttribute('visibility','hidden');clone.setAttribute('pointer-events','none');outer.appendChild(clone);try{const b=clone.getBBox();return{x:b.x,y:b.y,width:b.width,height:b.height};}finally{clone.remove();}};
    const cap=clip(mainTriangle),capPath=cap?.querySelector('path'),capBox=renderedBounds(capPath);
    return{
      configuredRoundReverseClamp:paths(linear,'line-point:').length===6&&paths(linear,'line-point:').every(p=>center(p).every(n=>close(n,Math.round(n),.001)))&&coord(linear,'line-point:5:North:1',485,163)&&coord(linear,'line-point:5:North:0',690,335)&&Number(mark(linear,'line-point:5:North:0').dataset.value)===12.5&&texts(linear).includes('0')&&texts(linear).includes('10'),
      actualBase2AndValueDomainSymLog:coord(log,'line-point:5:North:1',expectedLogX,expectedSymY)&&['-20','0','50'].every(t=>texts(log).includes(t))&&texts(log).includes('128'),
      utcCalendarNiceAndReverse:coord(utc,'line-point:5:North:1',336,132.5)&&texts(utc).includes('2024-02-29')&&center(mark(utc,'line-point:5:North:0'))[0]>center(mark(utc,'line-point:5:North:2'))[0],
      typedPointEndpointsAndGap:paths(full,'line-point:').length===9&&coord(full,'line-point:5:North:0',70,245)&&coord(full,'line-point:5:North:1',225,125)&&!mark(full,'line-point:5:North:2')&&paths(full,'line-stroke:5:North:').length===2&&texts(full).includes('1')&&texts(full).includes('#1'),
      ordinalAffineWindowAndRawLabels:coord(windowed,'line-point:5:North:1',173,125)&&coord(windowed,'line-point:5:North:3',587,215)&&!mark(windowed,'line-point:5:North:0')&&!mark(windowed,'line-point:5:North:4')&&texts(windowed).some(t=>t==='#1, 7')&&texts(windowed).some(t=>t==='April, 4')&&!texts(windowed).some(t=>/^173(?:\.0+)?,/.test(t)),
      sevenMathematicalAreas:filledPaths.length===7&&filledPaths.every(p=>p.getAttribute('fill')!=='none'&&!p.hasAttribute('stroke')&&Number(p.dataset.value)===900&&(p.dataset.id.endsWith(':Circle')?close(Math.PI*(p.getBBox().width/2)**2,900,.01):close(area(p),900,.01))),
      sevenTrueStrokeCenterlines:strokePaths.length===7&&strokePaths.every(p=>p.getAttribute('fill')==='none'&&p.getAttribute('stroke-width')==='1.5'&&p.getAttribute('stroke-linecap')==='round')&&(plus.getAttribute('d').match(/[MLCZ]/g)||[]).join('')==='MLML'&&close(plus.getTotalLength(),4*plusRadius,.01)&&plus.isPointInStroke(new DOMPoint(170,190))&&!plus.isPointInFill(new DOMPoint(170,190)),
      rotatedGlyphFactoryOrigins:['Triangle','Star','Wye'].every((id,i)=>{const p=mark(rotated,'gallery-glyph-rotated:'+id),a=center(mark(rotated,'gallery-glyph-anchor:'+id)),v=centroid(p);return close(area(p),1600,.01)&&close(a[0],180+i*180)&&close(a[1],180)&&close(v[0],a[0])&&close(v[1],a[1]);})&&Math.hypot(...center(mark(rotated,'gallery-glyph-rotated:Triangle')).map((v,i)=>v-[180,180][i]))>1,
      radarWyeAndHollowTrueOrigins:radarWye.length===10&&radarWye.every(p=>vertices(p).length===9&&p.getAttribute('fill')!=='none'&&close(area(p),110,.01))&&radarHollow.length===10&&radarHollow.every(p=>vertices(p).length===3&&p.getAttribute('fill')==='none'&&p.getAttribute('stroke-width')==='2'&&!p.isPointInStroke(new DOMPoint(...centroid(p))))&&centroid(mark(hollow,'radar-point:5:North:5:Speed')).every((v,i)=>close(v,[360,101.4][i])),
      geographicGlyphAreasAndLegendGeometry:paths(stars,'geo-symbol:').length===3&&!mark(stars,'geo-symbol:4:zero')&&!mark(stars,'geo-symbol:6:hidden')&&[mainStar,smallStar,missingStar].every(p=>vertices(p).length===10)&&close(area(mainStar),Math.PI*324,.01)&&close(area(smallStar),Math.PI*36,.01)&&close(area(missingStar),Math.PI*64,.01)&&close(vertices(mainStar)[0][0],360)&&close(vertices(mainStar)[0][1],205-Math.sqrt(Math.PI*324*.8908130915292852))&&paths(stars,'geo-size-legend:').length===3&&paths(stars,'geo-color-legend:').length===2&&[...paths(stars,'geo-size-legend:'),...paths(stars,'geo-color-legend:')].every(p=>vertices(p).length===10),
      globeHollowGlyphMaskAndBackface:paths(globe,'geo-symbol:').length===3&&!mark(globe,'geo-symbol:4:back')&&paths(globe,'geo-symbol:').every(p=>vertices(p).length===3&&p.getAttribute('fill')==='none'&&p.getAttribute('stroke-width')==='4'&&clip(p))&&centroid(mainTriangle).every((v,i)=>close(v,[360,205][i]))&&!mainTriangle.isPointInStroke(new DOMPoint(360,205))&&close(centroid(partialTriangle)[0],360+135*Math.sin(85*Math.PI/180))&&partialBox.x+partialBox.width>495&&capBox&&close(capBox.x,225)&&close(capBox.y,70)&&close(capBox.width,270)&&close(capBox.height,270)&&paths(globe,'geo-size-legend:').length===2&&paths(globe,'geo-color-legend:').length===3&&[...paths(globe,'geo-size-legend:'),...paths(globe,'geo-color-legend:')].every(p=>vertices(p).length===3&&p.getAttribute('fill')==='none'&&!clip(p)),
    };
  });
  for(const[name,passed]of Object.entries(scaleGlyphGeometry))if(!passed)failures.push(`Scale/glyph geometry failed: ${name}`);
  const extendedLayoutGeometry = await page.evaluate(() => {
    const svg = name => [...document.querySelectorAll('article')].find(card => card.querySelector('h2').textContent === name).querySelector('svg');
    const paths = (root, prefix) => [...root.querySelectorAll('path[data-id]')].filter(path => path.dataset.id.startsWith(prefix));
    const mark = (root, id) => root.querySelector(`path[data-id="${id}"]`);
    const labels = root => [...root.querySelectorAll('text')].map(label => label.textContent);
    const close = (a, b, tolerance = .001) => Number.isFinite(a) && Math.abs(a - b) <= tolerance;
    const samePoint = (a, b) => a.every((coordinate, index) => close(coordinate, b[index]));
    const vertices = path => [...path.getAttribute('d').matchAll(/[ML]([^MLCZ]+)/g)].map(([, coordinates]) => coordinates.trim().split(/[ ,]+/).map(Number));
    const origin = points => [0, 1].map(axis => points.reduce((sum, point) => sum + point[axis], 0) / points.length);
    const distance = (a, b) => Math.hypot(a[0] - b[0], a[1] - b[1]);
    const radius = (points, center) => Math.max(...points.map(point => distance(point, center)));
    const area = points => Math.abs(points.reduce((sum, [x, y], i) => {
      const [u, v] = points[(i + 1) % points.length];
      return sum + x * v - y * u;
    }, 0) / 2);
    const separation = (sites, spacing) => sites.every((a, i) => sites.slice(i + 1).every(b => distance(a.center, b.center) >= a.radius + b.radius + spacing - .001));
  
    const reversed = svg('Bars · reversed rounded value scale');
    const mapBar = v => Math.round(696 - 632 * v / 10);
    const roundedBars = paths(reversed, 'styled-bar:');
    const reversedTicks = paths(reversed, 'axis-bar-value-tick-');
    const signed = svg('Bars · signed SymLog stacks');
    const mapSigned = v => 190 - 158 * Math.sign(v) * Math.log1p(Math.abs(v) / 12) / Math.log(16);
    const southReturns = mark(signed, 'styled-bar:7:Returns:5:South').getBBox();
    const northSales = mark(signed, 'styled-bar:5:Sales:5:North').getBBox();
    const southTotal = [...signed.querySelectorAll('text')].find(text => text.textContent === '-44');
    const northTotal = [...signed.querySelectorAll('text')].find(text => text.textContent === '120');
    const signedTicks = paths(signed, 'axis-bar-value-tick-');
  
    const logSwarm = svg('Swarm · logarithmic Wye packing');
    const wyes = paths(logSwarm, 'swarm-node:').map(path => {
      const points = vertices(path), center = origin(points);
      return {path, points, center, radius: radius(points, center)};
    });
    const maxWye = Math.max(...wyes.map(site => site.radius));
    const mapLog = v => 64 + maxWye + (632 - 2 * maxWye) * (Math.log2(v) - 1) / 5;
    const logTicks = paths(logSwarm, 'swarm-value-axis-tick-');
    const utcSwarm = svg('Swarm · UTC hollow symbols');
    const diamonds = paths(utcSwarm, 'swarm-node:').map(path => {
      const points = vertices(path), center = origin(points);
      return {path, points, center, radius: radius(points, center) + 4};
    });
    const maxDiamond = Math.max(...diamonds.map(site => site.radius));
    const feb28 = Date.UTC(2024, 1, 28), day = 86_400_000;
    const mapUtc = v => 32 + maxDiamond + (316 - 2 * maxDiamond) * (v - feb28) / (4 * day);
    const dateLabels = Array.from({length: 5}, (_, i) => new Intl.DateTimeFormat('en-US', {
      month: 'short', day: 'numeric', timeZone: 'UTC',
    }).format(new Date(feb28 + i * day)));
    const utcTicks = paths(utcSwarm, 'swarm-value-axis-tick-');
  
    const network = svg('Network · independent force and glyph sizes');
    const hub = vertices(mark(network, 'force-node:Hub'));
    const west = vertices(mark(network, 'force-node:West'));
    const eastPath = mark(network, 'force-node:East'), east = vertices(eastPath);
    const north = mark(network, 'force-node:North').getBBox();
    const linkEndpoints = [[376, 98], [245, 253], [528, 254]];
    const combo = svg('Combo · independent SymLog and Log axes');
    const comboX = index => 64 + 580 * ((index + .58) / 3.16 - .12) / .82;
    const leftY = value => Math.round(32 + 316 * (Math.sign(value) * Math.log1p(Math.abs(value) / 2) + Math.log(3)) / Math.log(12));
    const rightY = value => 348 - 316 * Math.log10(value / 100) / 2;
    const leftTicks = paths(combo, 'combo-axis-left-tick-');
    const rightTicks = paths(combo, 'combo-axis-right-tick-');
    const orders = vertices(paths(combo, 'combo:6:Orders:path:')[0]);
    const retailPath = mark(combo, 'combo:6:Retail:datum:c3:Jan');
    const retailJan = retailPath.getBBox();
    const wholesaleFeb = mark(combo, 'combo:9:Wholesale:datum:c3:Feb');
    const wholesaleBounds = wholesaleFeb.getBBox();
    const custom = svg('Scatter · custom datum badges');
    const measured = mark(custom, 'scatter-node:8:Measured:1:a');
    const cropped = mark(custom, 'scatter-node:9:Estimated:7:cropped');
    const [mx, my] = [64 + 632 * .18, 348 - 316 * .28];
    const measuredVertices = vertices(measured), cropVertices = vertices(cropped);
    const cropBounds = cropped.getBBox(), cropClip = cropped.parentElement;
    const [cx, cy] = [64 - 632 * .035, 348 - 316 * .78];
    return {
      roundedReverseBarAxisAndGeometry: roundedBars.length === 8 && roundedBars.every(path => {
        const bounds = path.getBBox();
        return close(bounds.x, mapBar(Number(path.dataset.value))) && close(bounds.x + bounds.width, 696);
      }) && reversedTicks.length === 6 && reversedTicks.every((path, i) => close(vertices(path)[0][0], mapBar(2 * i))) && labels(reversed).slice(0, 6).join(',') === '0,2,4,6,8,10',
      signedSymLogStacksAndTotals: paths(signed, 'styled-bar:').length === 12 &&
        signedTicks.length === 3 && signedTicks.every((path, i) => close(vertices(path)[0][1], mapSigned((i - 1) * 100))) &&
        close(southReturns.y, mapSigned(-15)) && close(southReturns.y + southReturns.height, mapSigned(-44)) &&
        close(northSales.y, mapSigned(120)) && close(northSales.y + northSales.height, mapSigned(25)) &&
        close(Number(southTotal.getAttribute('y')), mapSigned(-44) + 10) && close(Number(northTotal.getAttribute('y')), mapSigned(120) - 10),
      logarithmicWyeAreaAndPacking: wyes.length === 18 && logTicks.length === 6 &&
        logTicks.every((path, i) => close(vertices(path)[0][0], mapLog(2 ** (i + 1)))) &&
        wyes.every(({path, points, center}, i) => points.length === 9 && close(area(points), 155 + 34 * (i % 6)) && close(center[0], mapLog(Number(path.dataset.value)))) &&
        separation(wyes, 5) && labels(logSwarm).slice(-3).join(',') === 'Trial,Standard,Plus',
      reversedUtcHollowPacking: diamonds.length === 12 && utcTicks.length === 5 &&
        utcTicks.every((path, i) => close(vertices(path)[0][1], mapUtc(feb28 + i * day))) &&
        diamonds.every(({path, points, center}) => points.length === 4 && path.getAttribute('fill') === 'none' && path.getAttribute('stroke-width') === '2' &&
          close(center[1], mapUtc(Number(path.dataset.value))) && !path.isPointInStroke(new DOMPoint(...center)) && path.isPointInStroke(new DOMPoint(...points[0]))) &&
        separation(diamonds, 2) && dateLabels.every((label, i) => labels(utcSwarm)[i] === label),
      networkGlyphOriginsAndLinkEndpoints: paths(network, 'force-node:').length === 4 && paths(network, 'force-link:').length === 3 &&
        hub.length === 10 && close(area(hub), 2200) && samePoint(origin(hub), [380, 202]) &&
        west.length === 9 && close(area(west), 1100) && samePoint(origin(west), linkEndpoints[1]) &&
        east.length === 3 && samePoint(origin(east), linkEndpoints[2]) && eastPath.getAttribute('fill') === 'none' && !eastPath.isPointInStroke(new DOMPoint(...linkEndpoints[2])) &&
        close(north.width, 16) && close(north.height, 16) && samePoint([north.x + north.width / 2, north.y + north.height / 2], linkEndpoints[0]) &&
        linkEndpoints.every((point, i) => {const [from, to] = vertices(mark(network, `force-link:${i}`)); return samePoint(from, [380, 202]) && samePoint(to, point);}),
      comboWindowIndependentAxesAndStack: labels(combo).slice(0, 3).join(',') === 'Jan,Feb,Mar' &&
        leftTicks.length === 6 && [-4,-2,0,2,4,6].every((value, i) => close(vertices(leftTicks[i])[0][1], leftY(value))) &&
        rightTicks.length === 5 && [100,300,1000,3000,10000].every((value, i) => close(vertices(rightTicks[i])[0][1], rightY(value))) &&
        orders.length === 3 && orders.every((point, i) => samePoint(point, [comboX(i), rightY(100 * 10 ** i)])) &&
        retailJan.x < 64 && retailJan.x + retailJan.width > 64 && retailPath.parentElement.getAttribute('viewBox') === '64 32 580 316' &&
        retailPath.parentElement.getAttribute('overflow') === 'hidden' && close(retailJan.y, leftY(0)) && close(retailJan.y + retailJan.height, leftY(2)) &&
        Number(wholesaleFeb.dataset.value) === -1 && close(wholesaleBounds.y, leftY(-4)) && close(wholesaleBounds.y + wholesaleBounds.height, leftY(-3)),
      customGradientAndAsymmetricShape: measuredVertices.length === 6 && samePoint(measuredVertices[0], [mx - 1.45 * 12.5, my - .9 * 12.5]) &&
        close(measuredVertices[2][0], mx + 1.55 * 12.5) && close(measuredVertices[5][0], mx - .78 * 12.5) &&
        measured.getAttribute('fill').startsWith('url(#') && measured.isPointInFill(new DOMPoint(mx, my)) && !measured.isPointInFill(new DOMPoint(mx - 12.5, my)),
      customOffsetDashAndRealPlotClip: cropVertices.length === 4 && samePoint(cropVertices[0], [cx + .95 * 12.5, cy - 1.05 * 12.5]) &&
        cropped.getAttribute('fill') === 'none' && cropped.getAttribute('stroke-dasharray') === '8 4' && cropped.getAttribute('stroke-width') === '3' &&
        cropBounds.x < 64 && cropBounds.x + cropBounds.width > 64 && cropClip.getAttribute('viewBox') === '64 32 632 316' &&
        cropClip.getAttribute('overflow') === 'hidden' && !cropped.isPointInStroke(new DOMPoint(cx, cy)),
    };
  });
  for (const [name, passed] of Object.entries(extendedLayoutGeometry)) if (!passed) failures.push(`Configured layout geometry failed: ${name}`);
  const scatterGeometry=await page.evaluate(()=>{
    const svg=name=>[...document.querySelectorAll('article')].find(c=>c.querySelector('h2').textContent===name).querySelector('svg');
    const nodes=root=>[...root.querySelectorAll('path[data-id]')].filter(p=>p.dataset.id.startsWith('scatter-node:'));
    const mark=(root,series,id)=>nodes(root).find(p=>p.dataset.id===`scatter-node:${new TextEncoder().encode(series).length}:${series}:${new TextEncoder().encode(id).length}:${id}`);
    const close=(a,b,t=.01)=>Number.isFinite(a)&&Math.abs(a-b)<=t;
    const center=p=>{const b=p.getBBox();return[b.x+b.width/2,b.y+b.height/2];};
    const vertices=p=>(p.getAttribute('d').match(/[ML][^MLCZ]+/g)||[]).map(c=>(c.slice(1).match(/[-+]?(?:\d*\.?\d+)(?:e[-+]?\d+)?/gi)||[]).map(Number));
    const centroid=p=>{const v=vertices(p);return v.reduce((a,p)=>[a[0]+p[0]/v.length,a[1]+p[1]/v.length],[0,0]);};
    const area=p=>{const v=vertices(p);return Math.abs(v.reduce((s,p,i)=>{const q=v[(i+1)%v.length];return s+p[0]*q[1]-q[0]*p[1];},0)/2);};
    const at=(p,x,y)=>{const c=center(p);return close(c[0],x)&&close(c[1],y);};
    const plotClip=p=>{const clip=p.ownerSVGElement;return clip?.ownerSVGElement&&close(Number(clip.getAttribute('x')),64)&&close(Number(clip.getAttribute('y')),32)&&close(Number(clip.getAttribute('width')),632)&&close(Number(clip.getAttribute('height')),316)&&clip.getAttribute('overflow')==='hidden';};
    const defaults=svg('Scatter: default nine-pixel circles'),dynamic=svg('Scatter: dynamic diameter extrapolation'),stars=svg('Scatter: filled Star area and origins'),hollow=svg('Scatter: hollow Triangle centers'),utc=svg('Scatter: UTC and reversed value scale'),point=svg('Scatter: typed point X and Y');
    const sizes=[['North','a',8],['North','b',10],['North','c',20],['South','d',30],['South','e',40]];
    const positions=[['left',190.4],['center',380],['right',569.6]];
    const text=[...utc.querySelectorAll('text')].map(t=>t.textContent);
    return{
      defaultDiameterAndNivoAutoDomains:nodes(defaults).length===5&&nodes(defaults).every(p=>close(p.getBBox().width,9)&&close(p.getBBox().height,9)&&plotClip(p))&&at(mark(defaults,'North','a'),64+632*2/9,190)&&at(mark(defaults,'North','b'),64+632*5/9,32)&&at(mark(defaults,'North','c'),696,229.5)&&Number(mark(defaults,'North','a').dataset.value)===4,
      dynamicDiameterExtrapolation:nodes(dynamic).length===5&&sizes.every(([series,id,size])=>{const p=mark(dynamic,series,id);return close(p.getBBox().width,size)&&close(p.getBBox().height,size);})&&at(mark(dynamic,'North','a'),190.4,221.6)&&at(mark(dynamic,'South','e'),506.4,316.4)&&Number(mark(dynamic,'South','e').dataset.value)===1,
      filledAreaAndPhysicalOrigins:nodes(stars).length===3&&positions.every(([id,x])=>{const p=mark(stars,'Glyphs',id),c=centroid(p);return vertices(p).length===10&&close(area(p),900)&&close(c[0],x)&&close(c[1],190)&&p.getAttribute('fill')!=='none';}),
      hollowOriginsAndActualPlotClips:nodes(hollow).length===3&&positions.every(([id,x])=>{const p=mark(hollow,'Glyphs',id),c=centroid(p),b=center(p);return vertices(p).length===3&&p.getAttribute('fill')==='none'&&p.getAttribute('stroke-width')==='2'&&close(c[0],x)&&close(c[1],190)&&Math.hypot(b[0]-c[0],b[1]-c[1])>1&&!p.isPointInStroke(new DOMPoint(...c))&&plotClip(p);}),
      utcLeapDayAndReversedValueAxis:nodes(utc).length===3&&at(mark(utc,'Dates','Feb 28'),64,95.2)&&at(mark(utc,'Dates','Leap day'),357,221.6)&&at(mark(utc,'Dates','Mar 1'),650,158.4)&&Number(mark(utc,'Dates','Leap day').dataset.value)===6&&['2024-02-28','2024-02-29','2024-03-01'].every(date=>text.filter(t=>t===date).length===1),
      typedPointIdentityAndDownwardYAxis:nodes(point).length===3&&at(mark(point,'Typed','text one'),64,32)&&at(mark(point,'Typed','number one'),380,348)&&at(mark(point,'Typed','last'),696,32)&&nodes(point).every(p=>Number(p.dataset.value)===0)&&new Set(nodes(point).map(p=>p.dataset.id)).size===3,
    };
  });
  for(const[name,passed]of Object.entries(scatterGeometry))if(!passed)failures.push(`Typed Scatter geometry failed: ${name}`);
  const clipLayout=await page.evaluate(()=>{
    const card=[...document.querySelectorAll('article')].find(c=>c.querySelector('h2').textContent==='Clipped domains'),root=card.querySelector('svg');
    const outer=root.getBoundingClientRect();return [...root.querySelectorAll('svg')].every(el=>Math.abs(el.getBoundingClientRect().width/outer.width-Number(el.getAttribute('width'))/720)<0.001);
  });
  if(!clipLayout)failures.push('Inline SVG plot clipping viewport was overridden by page CSS');
  for (const chart of charts) {
    if (chart.coloredPixels < 100 || !chart.toggle || chart.meanChannelError > 3.5) failures.push(`Invalid render: ${JSON.stringify(chart)}`);
  }
  const galleryPicking={dashGap:true,pathMaskHole:true,maskExterior:true};
  for(const [name,kind] of [['Dashed curves and cap shapes','dash'],['Even-odd path masks','mask']]){
    const card=page.locator('article').filter({has:page.getByRole('heading',{name,exact:true})});
    const points=kind==='dash'?await card.locator('path[data-id="dash-0"]').evaluate(path=>[15,5.5].map(length=>{const p=path.getPointAtLength(length);return{x:p.x,y:p.y};})):[{x:360,y:80},{x:360,y:200},{x:120,y:150}];
    for(const renderer of ['svg','canvas']){
      const surface=card.locator(renderer==='svg'?'.svg-view>svg':'.canvas-view>canvas');
      if(renderer==='canvas')await card.locator('.toggle').click();
      await surface.scrollIntoViewIfNeeded();const bounds=await surface.boundingBox();
      for(let i=0;i<points.length;i++){
        await page.mouse.move(bounds.x+points[i].x/720*bounds.width,bounds.y+points[i].y/400*bounds.height);
        const text=await card.locator('.status').textContent(),expected=i===0;
        if(text.includes(kind==='dash'?'dash-0':'masked-stroke')!==expected)failures.push(`${renderer} gallery ${kind} picking failed at sample ${i}: ${text}`);
      }
      if(renderer==='canvas')await card.locator('.toggle').click();
    }
  }
  await page.setViewportSize({width: 390, height: 844});
  const mobile = await page.evaluate(() => ({overflow:document.documentElement.scrollWidth > innerWidth, columns:getComputedStyle(document.querySelector('main')).gridTemplateColumns.split(' ').length}));
  if (mobile.overflow || mobile.columns !== 1) failures.push('Gallery mobile layout failed');
  const button = page.locator('article .toggle').first();
  await button.focus(); await page.keyboard.press('Enter');
  if (await button.getAttribute('aria-pressed') !== 'true') failures.push('Keyboard toggle failed');
  // Use a real interior point. The x=0 point is clipped at the plot boundary;
  // native mouse coordinate quantization can legitimately put it outside.
  const canvas = page.locator('article canvas').first();
  const box = await canvas.boundingBox();
  const hit = await page.evaluate(() => {
    const card=document.querySelector('article'), path=card.querySelector('path[data-id="point-0-1"]');
    if(!path)return null;const d=path.getAttribute('d');const match=/M([^ ]+) ([^CMLZ ]+)/.exec(d);return match?{x:Number(match[1])-3,y:Number(match[2])}:null;
  });
  if (box && hit) {
    await page.mouse.move(box.x+hit.x/720*box.width, box.y+hit.y/400*box.height);
    const status = await page.locator('article .status').first().textContent();
    if (!status.includes('North')) failures.push('Canvas pointer tooltip failed');
  } else failures.push('Line pointer fixture missing');
  if (errors.length) failures.push(...errors);
  if (failures.length) throw new Error(failures.join('\n'));
  await page.setViewportSize({width:1440,height:1000});
  await page.evaluate(() => {window.scrollTo(0,0);});
  return {passed:true, charts, discreteColorGeometry, upgraded, p2Geometry, interactionGeometry, scaleGlyphGeometry, scatterGeometry, extendedLayoutGeometry, galleryPicking, mobile, pageErrors:errors};
})
