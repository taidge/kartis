// Optional verification tooling. Chart runtime and layouts remain pure Rust.
import { readFile, writeFile } from 'node:fs/promises';
import { chromium } from 'playwright';

const wasm = process.argv.includes('--wasm');
const widget = process.argv.includes('--widget');
const host = process.argv.includes('--host');
const raster = process.argv.includes('--raster');
const density = process.argv.includes('--density');
const outline = process.argv.includes('--outline');
if([wasm,widget,host,raster,density,outline].filter(Boolean).length>1)throw Error('Choose one browser fixture');
const args = process.argv.slice(2).filter(arg=>!['--wasm','--widget','--host','--raster','--density','--outline'].includes(arg));
async function runSuite(name, run) {
  try {
    const result = await run();
    if (result?.passed !== true) throw Error('Verifier did not report a passing result');
    console.error(`${name}: passed`);
    return result;
  } catch (error) {
    throw new Error(`Browser suite ${name} failed: ${error.message}`, {cause:error});
  }
}
const verify = (0, eval)(await readFile(new URL(density?'./density-browser-acceptance.js':outline?'./outline-browser-acceptance.js':raster?'./raster-browser-acceptance.js':host?'./host-browser-acceptance.js':widget?'./widget-browser-acceptance.js':wasm ? './wasm-browser-acceptance.js' : './browser-acceptance.js', import.meta.url), 'utf8'));
const touchVerifier = host ? (0, eval)(await readFile(new URL('./touch-browser-acceptance.js', import.meta.url), 'utf8')) : undefined;
const viewportVerifier = host ? (0, eval)(await readFile(new URL('./viewport-browser-acceptance.js', import.meta.url), 'utf8')) : undefined;
const svgVerifier = host ? (0, eval)(await readFile(new URL('./svg-browser-acceptance.js', import.meta.url), 'utf8')) : undefined;
const navigationVerifier = host ? (0, eval)(await readFile(new URL('./navigation-browser-acceptance.js', import.meta.url), 'utf8')) : undefined;
const presentationVerifier = host ? (0, eval)(await readFile(new URL('./presentation-browser-acceptance.js', import.meta.url), 'utf8')) : undefined;
const motionVerifier = host ? (0, eval)(await readFile(new URL('./motion-browser-acceptance.js', import.meta.url), 'utf8')) : undefined;
const radarColorGeometry = (0, eval)(await readFile(new URL('./radar-color-gallery-geometry.js', import.meta.url), 'utf8'));
const configuredPaintGeometry = (0, eval)(await readFile(new URL('./configured-paint-gallery-geometry.js', import.meta.url), 'utf8'));
const boxplotProfileGeometry = (0, eval)(await readFile(new URL('./boxplot-profile-gallery-geometry.js', import.meta.url), 'utf8'));
const lineCustomGeometry = (0, eval)(await readFile(new URL('./line-custom-gallery-geometry.js', import.meta.url), 'utf8'));
const radialFocusGeometry = (0, eval)(await readFile(new URL('./radial-focus-gallery-geometry.js', import.meta.url), 'utf8'));
const contourDensityGeometry = (0, eval)(await readFile(new URL('./contour-density-gallery-geometry.js', import.meta.url), 'utf8'));
const sequentialQuantileGeometry = (0, eval)(await readFile(new URL('./sequential-quantile-gallery-geometry.js', import.meta.url), 'utf8'));
const geoConicGeometry = (0, eval)(await readFile(new URL('./geo-conic-gallery-geometry.js', import.meta.url), 'utf8'));
const geoSphericalGeometry = (0, eval)(await readFile(new URL('./geo-spherical-gallery-geometry.js', import.meta.url), 'utf8'));
const browser = await chromium.launch({ headless: true, ...(process.env.PLAYWRIGHT_CHANNEL ? {channel:process.env.PLAYWRIGHT_CHANNEL} : {}) });
try {
  const context = await browser.newContext({ viewport: { width: 1440, height: 1000 }, deviceScaleFactor: 2 });
  const page = await context.newPage();
  const url=args[0] || (raster?'http://127.0.0.1:8815':host||density?'http://127.0.0.1:8814':widget||outline?'http://127.0.0.1:8813':wasm ? 'http://127.0.0.1:8812' : 'http://127.0.0.1:8811');
  if(density){await page.goto(url);await page.waitForFunction(()=>window.hostReady===true);}
  const fixture = density?'density':outline?'outline':raster?'raster':host?'host':widget?'widget':wasm?'wasm':'gallery';
  const report = await runSuite(fixture,()=>verify(page, url, touchVerifier, viewportVerifier, svgVerifier, navigationVerifier, presentationVerifier, motionVerifier));
  if(raster){const extra=(0,eval)(await readFile(new URL('./color-font-browser-acceptance.js',import.meta.url),'utf8'));report.colorFonts=await runSuite('colorFonts',()=>extra(page,url));}
  if(raster){const extra=(0,eval)(await readFile(new URL('./raster-plot-widgets-browser-acceptance.js',import.meta.url),'utf8'));report.rasterPlotWidgets=await runSuite('rasterPlotWidgets',()=>extra(page,url));}
  if (fixture === 'gallery') {
    report.radarColorGeometry = await radarColorGeometry(page);
    report.configuredPaintGeometry = await configuredPaintGeometry(page);
    report.boxplotProfileGeometry = await boxplotProfileGeometry(page);
    report.lineCustomGeometry = await lineCustomGeometry(page);
    report.radialFocusGeometry = await radialFocusGeometry(page);
    report.contourDensityGeometry = await contourDensityGeometry(page);
    report.sequentialQuantileGeometry = await sequentialQuantileGeometry(page);
    report.geoConicGeometry = await geoConicGeometry(page);
    report.geoSphericalGeometry = await geoSphericalGeometry(page);
  }
  if(host){
    for(const [name,file] of [['paints','paint-browser-acceptance.js'],['contours','contours-browser-acceptance.js'],['clips','clip-browser-acceptance.js'],['combo','combo-browser-acceptance.js'],['patterns','pattern-browser-acceptance.js'],['parallel','parallel-browser-acceptance.js'],['tree','tree-browser-acceptance.js'],['networkDrag','network-drag-browser-acceptance.js'],['pie','pie-browser-acceptance.js'],['radar','radar-browser-acceptance.js'],['sankeyStyles','sankey-style-browser-acceptance.js'],['styledLine','styled-line-browser-acceptance.js'],['colors','color-browser-acceptance.js'],['barStyles','bar-style-browser-acceptance.js'],['heatmap','heatmap-browser-acceptance.js'],['stackedRadial','stacked-radial-browser-acceptance.js'],['svgIsolation','svg-isolation-browser-acceptance.js'],['calendarLayout','calendar-layout-browser-acceptance.js'],['statistics','statistics-browser-acceptance.js'],['hierarchyFocus','hierarchy-focus-browser-acceptance.js'],['areaBump','area-bump-browser-acceptance.js']]){
      const extra=(0,eval)(await readFile(new URL(`./${file}`,import.meta.url),'utf8'));report[name]=await runSuite(name,()=>extra(page));
    }
  }
  if(widget){const extra=(0,eval)(await readFile(new URL('./outline-browser-acceptance.js',import.meta.url),'utf8'));report.outline=await runSuite('outline',()=>extra(page,url));}
  if(host){
    for(const [name,file] of [['marimekko','marimekko-browser-acceptance.js'],['geoStreams','geo-stream-browser-acceptance.js'],['delaunay','delaunay-browser-acceptance.js'],['pieLayout','pie-layout-browser-acceptance.js'],['lineCrosshair','line-crosshair-browser-acceptance.js'],['radarSlices','radar-slices-browser-acceptance.js'],['relations','relations-browser-acceptance.js']]){
      const extra=(0,eval)(await readFile(new URL(`./${file}`,import.meta.url),'utf8'));report[name]=await runSuite(name,()=>extra(page));
    }
  }
  if(host){const extra=(0,eval)(await readFile(new URL('./configured-demo-browser-acceptance.js',import.meta.url),'utf8'));report.configuredExamples=await runSuite('configuredExamples',()=>extra(page));}
  if(host){for(const [name,file] of [['scaleProfiles','scale-profiles-browser-acceptance.js'],['glyphs','glyph-browser-acceptance.js'],['geoGlyphs','geo-glyph-browser-acceptance.js']]){const extra=(0,eval)(await readFile(new URL(`./${file}`,import.meta.url),'utf8'));report[name]=await runSuite(name,()=>extra(page));}}
  if(widget){const extra=(0,eval)(await readFile(new URL('./dash-widget-browser-acceptance.js',import.meta.url),'utf8'));report.dashes=await runSuite('dashes',()=>extra(page,url));}
  if (widget) {
    const {verifyPlotWidgets} = await import('./plot-widgets-browser-acceptance.mjs');
    const fixtureUrl = new URL('plot-widgets.html', url.endsWith('/') ? url : `${url}/`).href;
    report.plotWidgets = await runSuite('plotWidgets', () => verifyPlotWidgets(page, fixtureUrl));
  }
  if(host){const extra=(0,eval)(await readFile(new URL('./scatter-browser-acceptance.js',import.meta.url),'utf8'));report.scatter=await runSuite('scatter',()=>extra(page));}
  if (host) {
    const verify = (0, eval)(await readFile(new URL('./layout-models-browser-acceptance.js', import.meta.url), 'utf8'));
    report.layoutModels = await runSuite('layoutModels', () => verify(page));
  }
  if (host) {
    const verify = (0, eval)(await readFile(new URL('./flow-layouts-browser-acceptance.js', import.meta.url), 'utf8'));
    report.flowLayouts = await runSuite('flowLayouts', () => verify(page));
    const typography = (0, eval)(await readFile(new URL('./typography-browser-acceptance.js', import.meta.url), 'utf8'));
    report.typography = await runSuite('typography', () => typography(page));
    const textBaseline = (0, eval)(await readFile(new URL('./text-baseline-browser-acceptance.js', import.meta.url), 'utf8'));
    report.textBaseline = await runSuite('textBaseline', () => textBaseline(page));
    const editableBrush = (0, eval)(await readFile(new URL('./editable-brush-browser-acceptance.js', import.meta.url), 'utf8'));
    report.editableBrush = await runSuite('editableBrush', () => editableBrush(page));
    for (const [suite,file] of [['nonlinearColors','nonlinear-colors-browser-acceptance.js'],['geoInverse','geo-inverse-browser-acceptance.js'],['piecewiseScales','piecewise-scales-browser-acceptance.js'],['contourDensity','contour-density-browser-acceptance.js'],['sequentialQuantile','sequential-quantile-browser-acceptance.js'],['svgTransactions','svg-transactions-browser-acceptance.js'],['geoConic','geo-conic-browser-acceptance.js']]) {
      const verifyAdditional = (0, eval)(await readFile(new URL('./'+file, import.meta.url), 'utf8'));
      report[suite] = await runSuite(suite, () => verifyAdditional(page));
    }
  }
  if (host) {
    for (const [name, file] of [['swarm', 'swarm-browser-acceptance.js'], ['networkGlyphs', 'network-glyph-browser-acceptance.js'], ['scatterCustom', 'scatter-custom-browser-acceptance.js'], ['blends', 'blend-browser-acceptance.js'], ['browserPlotWidgets', 'browser-plot-widgets-acceptance.js'], ['blendMotion', 'blend-motion-browser-acceptance.js'], ['radarCustomPoints', 'radar-custom-points-browser-acceptance.js'], ['lineCustomPoints', 'line-custom-points-browser-acceptance.js'], ['areaBumpLabels', 'area-bump-labels-browser-acceptance.js']]) {
      const extra = (0, eval)(await readFile(new URL(`./${file}`, import.meta.url), 'utf8'));
      report[name] = await runSuite(name, () => extra(page));
    }
  }
  console.log(JSON.stringify(report, null, 2));
  if (args[1]) await writeFile(args[1], JSON.stringify(report, null, 2));
} finally { await browser.close(); }
