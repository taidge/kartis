// Independent portable engine acceptance. No CDP, browser installation, cache
// environment rewrites, or reduced pixel thresholds. Uses existing verifiers.
import {chromium,firefox,webkit} from 'playwright';
import {readFile,writeFile,readdir} from 'node:fs/promises';
import {existsSync} from 'node:fs';
import {homedir,platform} from 'node:os';
import {join,dirname} from 'node:path';
import {verifyPlotWidgets} from './plot-widgets-browser-acceptance.mjs';

const args=process.argv.slice(2), option=(name,fallback)=>{const index=args.indexOf(name);return index<0?fallback:args[index+1];};
const selected=option('--browsers','firefox,webkit,edge').split(',');
if(selected.some(name=>!['chromium','firefox','webkit','edge'].includes(name)))throw Error('Supported --browsers: chromium,firefox,webkit,edge');
const output=option('--output','target/gallery/cross-browser-report.json');
const urls={gallery:option('--gallery','http://127.0.0.1:8811'),widget:option('--widget','http://127.0.0.1:8813'),host:option('--host','http://127.0.0.1:8814'),raster:option('--raster','http://127.0.0.1:8815')};
const cache=platform()==='win32'?join(process.env.LOCALAPPDATA||join(homedir(),'AppData','Local'),'ms-playwright'):platform()==='darwin'?join(homedir(),'Library','Caches','ms-playwright'):join(homedir(),'.cache','ms-playwright');
async function candidates(name,type){
  const explicit=option(`--${name}-path`,null);if(explicit)return[explicit];
  const preferred=type.executablePath(), paths=[preferred];
  // The current driver's expected revision may be absent while a newer
  // installed Playwright bundle exists. Record its exact path/version and
  // validate the protocol in a real launch; do not claim pinned-driver parity.
  const dirs=await readdir(cache,{withFileTypes:true}).catch(()=>[]);
  const revisions=dirs.filter(d=>d.isDirectory()&&new RegExp(`^${name}-\\d+$`).test(d.name)).sort((a,b)=>Number(b.name.split('-')[1])-Number(a.name.split('-')[1]));
  for(const d of revisions){const root=join(cache,d.name);paths.push(name==='firefox'?join(root,'firefox',platform()==='win32'?'firefox.exe':'firefox'):join(root,platform()==='win32'?'Playwright.exe':'pw_run.sh'));}
  return[...new Set(paths)].filter(existsSync);
}
async function launch(name){
  if(name==='chromium')return{browser:await chromium.launch({headless:true}),launch:{engine:'chromium',path:chromium.executablePath()},attempts:[]};
  if(name==='edge')return{browser:await chromium.launch({headless:true,channel:'msedge'}),launch:{channel:'msedge',engine:'chromium',path:'system Microsoft Edge channel'},attempts:[]};
  const type=name==='firefox'?firefox:webkit,paths=await candidates(name,type),attempts=[];
  for(const path of paths){try{return{browser:await type.launch({headless:true,executablePath:path,timeout:20000}),launch:{engine:name,path,driverExpectedPath:type.executablePath(),usesDriverExpectedRevision:path===type.executablePath()},attempts};}catch(error){attempts.push({path,error:error.message});}}
  throw Error(`No usable installed ${name}: ${JSON.stringify(attempts)}`);
}
const source=name=>readFile(new URL(name,import.meta.url),'utf8').then(text=>(0,eval)(text));
const verifiers={gallery:await source('./browser-acceptance.js'),widget:await source('./widget-browser-acceptance.js'),outline:await source('./outline-browser-acceptance.js'),paints:await source('./paint-browser-acceptance.js'),contours:await source('./contours-browser-acceptance.js'),raster:await source('./raster-browser-acceptance.js')};
const verifyGallery = verifiers.gallery;
verifiers.colorFonts=await source('./color-font-browser-acceptance.js');
verifiers.rasterPlotWidgets=await source('./raster-plot-widgets-browser-acceptance.js');
const verifyRadarColorGallery = await source('./radar-color-gallery-geometry.js');
const verifyConfiguredPaintGallery = await source('./configured-paint-gallery-geometry.js');
const verifyBoxplotProfileGallery = await source('./boxplot-profile-gallery-geometry.js');
const verifyLineCustomGallery = await source('./line-custom-gallery-geometry.js');
const verifyRadialFocusGallery = await source('./radial-focus-gallery-geometry.js');
const verifyContourDensityGallery = await source('./contour-density-gallery-geometry.js');
const verifySequentialQuantileGallery = await source('./sequential-quantile-gallery-geometry.js');
const verifyGeoConicGallery = await source('./geo-conic-gallery-geometry.js');
const verifyGeoSphericalGallery = await source('./geo-spherical-gallery-geometry.js');
verifiers.gallery = async (page, url) => ({
  ...await verifyGallery(page, url),
  radarColorGeometry: await verifyRadarColorGallery(page),
  configuredPaintGeometry: await verifyConfiguredPaintGallery(page),
  boxplotProfileGeometry: await verifyBoxplotProfileGallery(page),
  lineCustomGeometry: await verifyLineCustomGallery(page),
  radialFocusGeometry: await verifyRadialFocusGallery(page),
  contourDensityGeometry: await verifyContourDensityGallery(page),
  sequentialQuantileGeometry: await verifySequentialQuantileGallery(page),
  geoConicGeometry: await verifyGeoConicGallery(page),
  geoSphericalGeometry: await verifyGeoSphericalGallery(page),
});
Object.assign(verifiers,{patterns:await source('./pattern-browser-acceptance.js'),clips:await source('./clip-browser-acceptance.js'),combo:await source('./combo-browser-acceptance.js')});
Object.assign(verifiers,{dashes:await source('./dash-widget-browser-acceptance.js')});
Object.assign(verifiers,{parallel:await source('./parallel-browser-acceptance.js'),tree:await source('./tree-browser-acceptance.js'),networkDrag:await source('./network-drag-browser-acceptance.js')});
Object.assign(verifiers,{configuredExamples:await source('./configured-demo-browser-acceptance.js')});
Object.assign(verifiers,{pie:await source('./pie-browser-acceptance.js'),radar:await source('./radar-browser-acceptance.js'),sankeyStyles:await source('./sankey-style-browser-acceptance.js')});
Object.assign(verifiers,{styledLine:await source('./styled-line-browser-acceptance.js')});
Object.assign(verifiers,{colors:await source('./color-browser-acceptance.js'),barStyles:await source('./bar-style-browser-acceptance.js'),heatmap:await source('./heatmap-browser-acceptance.js'),stackedRadial:await source('./stacked-radial-browser-acceptance.js')});
Object.assign(verifiers,{svgIsolation:await source('./svg-isolation-browser-acceptance.js')});
Object.assign(verifiers,{calendarLayout:await source('./calendar-layout-browser-acceptance.js'),statistics:await source('./statistics-browser-acceptance.js'),hierarchyFocus:await source('./hierarchy-focus-browser-acceptance.js')});
Object.assign(verifiers,{areaBump:await source('./area-bump-browser-acceptance.js')});
Object.assign(verifiers,{marimekko:await source('./marimekko-browser-acceptance.js'),geoStreams:await source('./geo-stream-browser-acceptance.js'),delaunay:await source('./delaunay-browser-acceptance.js')});
Object.assign(verifiers,{pieLayout:await source('./pie-layout-browser-acceptance.js'),lineCrosshair:await source('./line-crosshair-browser-acceptance.js'),radarSlices:await source('./radar-slices-browser-acceptance.js'),relations:await source('./relations-browser-acceptance.js')});
const hostSuites=new Set(['paints','contours','patterns','clips','combo','parallel','tree','networkDrag','configuredExamples','pie','radar','sankeyStyles','styledLine','colors','barStyles','heatmap','stackedRadial','svgIsolation','calendarLayout','statistics','hierarchyFocus','areaBump','marimekko','geoStreams','delaunay','pieLayout','lineCrosshair','radarSlices','relations']);
Object.assign(verifiers,{scaleProfiles:await source('./scale-profiles-browser-acceptance.js'),glyphs:await source('./glyph-browser-acceptance.js'),geoGlyphs:await source('./geo-glyph-browser-acceptance.js')});
for(const suite of ['scaleProfiles','glyphs','geoGlyphs'])hostSuites.add(suite);
Object.assign(verifiers,{scatter:await source('./scatter-browser-acceptance.js')});
hostSuites.add('scatter');
verifiers.layoutModels = await source('./layout-models-browser-acceptance.js');
hostSuites.add('layoutModels');
verifiers.flowLayouts = await source('./flow-layouts-browser-acceptance.js');
hostSuites.add('flowLayouts');
verifiers.typography = await source('./typography-browser-acceptance.js');
hostSuites.add('typography');
verifiers.textBaseline = await source('./text-baseline-browser-acceptance.js');
hostSuites.add('textBaseline');
verifiers.editableBrush = await source('./editable-brush-browser-acceptance.js');
hostSuites.add('editableBrush');
for(const [suite,file] of [['nonlinearColors','nonlinear-colors-browser-acceptance.js'],['geoInverse','geo-inverse-browser-acceptance.js'],['piecewiseScales','piecewise-scales-browser-acceptance.js'],['contourDensity','contour-density-browser-acceptance.js'],['sequentialQuantile','sequential-quantile-browser-acceptance.js'],['svgTransactions','svg-transactions-browser-acceptance.js'],['geoConic','geo-conic-browser-acceptance.js']]) {
  verifiers[suite] = await source('./'+file);
  hostSuites.add(suite);
}
Object.assign(verifiers, {swarm: await source('./swarm-browser-acceptance.js'), networkGlyphs: await source('./network-glyph-browser-acceptance.js'), scatterCustom: await source('./scatter-custom-browser-acceptance.js'), blends: await source('./blend-browser-acceptance.js'), browserPlotWidgets: await source('./browser-plot-widgets-acceptance.js'), blendMotion: await source('./blend-motion-browser-acceptance.js'), radarCustomPoints: await source('./radar-custom-points-browser-acceptance.js'), lineCustomPoints: await source('./line-custom-points-browser-acceptance.js'), areaBumpLabels: await source('./area-bump-labels-browser-acceptance.js')});
for (const suite of ['swarm', 'networkGlyphs', 'scatterCustom', 'blends', 'browserPlotWidgets', 'blendMotion', 'radarCustomPoints', 'lineCustomPoints', 'areaBumpLabels']) hostSuites.add(suite);
verifiers.plotWidgets = (page, url) => verifyPlotWidgets(page, new URL('plot-widgets.html', url.endsWith('/') ? url : `${url}/`).href);
// Additive suites retain their own assertions and pixel thresholds.
const report={startedAt:new Date().toISOString(),playwrightVersion:JSON.parse(await readFile(new URL('../node_modules/playwright/package.json',import.meta.url),'utf8')).version,platform:platform(),urls,scope:'Unmodified portable gallery/widget/outline/paint/contour/raster assertions; no CDP-only cases',excluded:[{capability:'Chrome CDP Input.dispatchTouchEvent',reason:'Chromium-specific protocol; no cross-engine touch/device claim'},{capability:'Chrome CDP screenshot/layer or process metrics',reason:'Not used by portable suite; no cross-engine compositing/performance/RSS claim'},{capability:'OS Safari, mobile devices and physical input',reason:'Installed Playwright WebKit is a desktop test bundle, not those platforms'}],browsers:[]};
const requestedSuites=option('--suites',null);
const suiteNames=requestedSuites===null?Object.keys(verifiers):requestedSuites.split(',');
if(suiteNames.some(name=>!Object.hasOwn(verifiers,name))||new Set(suiteNames).size!==suiteNames.length)throw Error(`Unknown or duplicate --suites: ${requestedSuites}`);
report.scope=`${suiteNames.length} shared portable suites; ${requestedSuites===null?'complete matrix':'explicit subset: '+suiteNames.join(',')}; no CDP-only cases`;
report.suites=suiteNames;
for(const name of selected){
  let instance;const entry={name,available:false,passed:false,tests:{}};
  try{
    instance=await launch(name);entry.available=true;entry.version=instance.browser.version();entry.launch=instance.launch;entry.launchAttempts=instance.attempts;
    for(const suite of suiteNames){
      const context=await instance.browser.newContext({viewport:{width:1440,height:1000},deviceScaleFactor:2});const page=await context.newPage();page.setDefaultTimeout(45000);const pageErrors=[];page.on('pageerror',error=>pageErrors.push(error.message));
      const start=Date.now();
      try{
        let result;
        if(hostSuites.has(suite)){await page.goto(urls.host);await page.waitForFunction(()=>window.hostReady===true);result=await verifiers[suite](page);}
        else result=await verifiers[suite](page,suite==='gallery'?urls.gallery:['raster','colorFonts','rasterPlotWidgets'].includes(suite)?urls.raster:urls.widget);
        if(pageErrors.length)throw Error(pageErrors.join('\n'));
        entry.tests[suite]={passed:true,elapsedMs:Date.now()-start,result};console.log(`${name} ${entry.version} ${suite}: passed`);
      }catch(error){entry.tests[suite]={passed:false,elapsedMs:Date.now()-start,error:error.message,pageErrors};console.error(`${name} ${entry.version} ${suite}: FAILED ${error.message}`);}
      finally{await context.close();}
    }
    entry.passed=Object.values(entry.tests).every(test=>test.passed);
  }catch(error){entry.error=error.message;console.error(`${name}: unavailable or rejected: ${error.message}`);}
  finally{if(instance)await instance.browser.close();}
  report.browsers.push(entry);
}
report.passed=report.browsers.every(entry=>entry.passed);report.finishedAt=new Date().toISOString();
// The caller supplies the report location. Do not introduce any global cache,
// output directory redirects or installation during acceptance.
if(!existsSync(dirname(output)))throw Error(`Report directory must already exist: ${dirname(output)}`);
await writeFile(output,JSON.stringify(report,null,2));
console.log(JSON.stringify({passed:report.passed,output,browsers:report.browsers.map(b=>({name:b.name,version:b.version,passed:b.passed,tests:Object.fromEntries(Object.entries(b.tests).map(([name,test])=>[name,test.passed]))}))},null,2));
if(!report.passed)process.exitCode=1;
