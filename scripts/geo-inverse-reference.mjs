// Independent original d3-geo inverse/stream/forward oracle. Preparation reads
// original npm packages explicitly; ordinary checks use the vendored bytes.
import assert from 'node:assert/strict';
import {readFile, writeFile, mkdir} from 'node:fs/promises';
import {execFileSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {fileURLToPath} from 'node:url';
import {join,resolve} from 'node:path';
import vm from 'node:vm';
const root = fileURLToPath(new URL('../', import.meta.url));
// Explicit regeneration can use existing package sources without invoking Cargo.
// --output-directory affects oracle reports only, never a build/cache setting.
const args=process.argv.slice(2), option=name=>{const i=args.indexOf(name);if(i<0)return null;assert(args[i+1],`Missing ${name}`);return resolve(args[i+1]);};
const generate=args.includes('--generate');
const target = option('--output-directory') ?? JSON.parse(execFileSync('cargo',['metadata','--offline','--no-deps','--format-version','1'],
  {cwd:root,encoding:'utf8',windowsHide:true})).target_directory;
await mkdir(target,{recursive:true});
const sha = data => createHash('sha256').update(data).digest('hex');
const paths = {bundle:'tests/fixtures/geo-inverse-d3-oracle.js', licenses:'tests/fixtures/geo-inverse-d3-LICENSE.txt',
  manifest:'tests/fixtures/geo-inverse-d3-source.json', queries:'tests/fixtures/geo-inverse-d3.csv'};
const files = ['src/projection/index.js','src/rotation.js','src/math.js','src/clip/circle.js','src/projection/equirectangular.js',
  'src/projection/mercator.js','src/projection/transverseMercator.js','src/projection/equalEarth.js',
  'src/projection/naturalEarth1.js','src/projection/azimuthal.js','src/projection/orthographic.js',
  'src/projection/azimuthalEqualArea.js','src/projection/azimuthalEquidistant.js','src/projection/stereographic.js','src/projection/gnomonic.js'];
if (generate || args.includes('--prepare')) {
  const base = option('--modules') ?? join(target,'d3-geo-reference/node_modules'), bundle=[], licenses=[], packages=[];
  let count=0, licenseCount=0;
  const append=data=>{data=Buffer.from(data);bundle.push(data);count+=data.length;};
  const license=data=>{data=Buffer.from(data);licenses.push(data);licenseCount+=data.length;};
  append('// Development-only original d3-geo numeric oracle; not a Kartis runtime dependency.\n');
  license('Full original development-oracle licenses.\n');
  for (const name of ['d3-array','d3-geo']) {
    const dir=join(base,name), pkg=JSON.parse(await readFile(join(dir,'package.json'),'utf8'));
    assert.equal(pkg.version,name==='d3-geo'?'3.1.1':'3.2.5');
    const dist=await readFile(join(dir,'dist',`${name}.js`)), text=await readFile(join(dir,'LICENSE'));
    const provenance=`https://www.npmjs.com/package/${name}/v/${pkg.version}`;
    append(`\n/* ${name}@${pkg.version} | ${pkg.license}\n${provenance}\nOriginal dist SHA-256 ${sha(dist)}\n`);
    append(text);append(`*/\n// BEGIN ORIGINAL ${name}\n`);const start=count;append(dist);const end=count;
    append(`\n// END ORIGINAL ${name}\n`);
    license(`\n----- ${name}@${pkg.version} -----\n${provenance}\n`);const licenseStart=licenseCount;license(text);const licenseEnd=licenseCount;
    packages.push({name,version:pkg.version,license:pkg.license,provenance,start,end,distSha256:sha(dist),licenseStart,licenseEnd,licenseSha256:sha(text)});
  }
  const originalFiles={};for(const path of files)originalFiles[path]=sha(await readFile(join(base,'d3-geo',path)));
  const js=Buffer.concat(bundle), notices=Buffer.concat(licenses);
  const manifest={scope:'Original development-only d3-geo functions, never Rust-derived expected values.',
    source:'https://github.com/d3/d3-geo/tree/v3.1.1/src/projection',originalFiles,packages,
    bundle:{path:paths.bundle,sha256:sha(js),bytes:js.length},licenses:{path:paths.licenses,sha256:sha(notices),bytes:notices.length}};
  for(const[path,data]of[[paths.bundle,js],[paths.licenses,notices],[paths.manifest,Buffer.from(JSON.stringify(manifest,null,2)+'\n')]]) {
    if(process.argv.includes('--check'))assert.deepEqual(await readFile(join(root,path)),data,`Original package preparation changed: ${path}`);
    else await writeFile(join(root,path),data);
  }
}
const manifest=JSON.parse(await readFile(join(root,paths.manifest),'utf8'));
const bundle=await readFile(join(root,paths.bundle)), notices=await readFile(join(root,paths.licenses));
assert.equal(sha(bundle),manifest.bundle.sha256);assert.equal(bundle.length,manifest.bundle.bytes);
assert.equal(sha(notices),manifest.licenses.sha256);assert.equal(notices.length,manifest.licenses.bytes);
assert.deepEqual(manifest.packages.map(p=>p.name),['d3-array','d3-geo']);
for(const p of manifest.packages) {
  assert.equal(sha(bundle.subarray(p.start,p.end)),p.distSha256);
  assert.equal(sha(notices.subarray(p.licenseStart,p.licenseEnd)),p.licenseSha256);
}
assert.equal(manifest.packages[1].version,'3.1.1');
assert.equal(manifest.packages[0].version,'3.2.5');
const context=vm.createContext({});vm.runInContext(bundle.toString('utf8'),context,{timeout:10000});const d3=context.d3;
const constructors={equirectangular:'geoEquirectangular',mercator:'geoMercator',transverse:'geoTransverseMercator',
  equalEarth:'geoEqualEarth',naturalEarth:'geoNaturalEarth1',orthographic:'geoOrthographic',equalArea:'geoAzimuthalEqualArea',
  equidistant:'geoAzimuthalEquidistant',stereographic:'geoStereographic',gnomonic:'geoGnomonic'};
const azimuthal=new Set(['orthographic','equalArea','equidistant','stereographic','gnomonic']);
const frame=[700,500,35,25,25,35], views=[[0,0,0],[35,20,25],[-120,-40,-70]];
const rows=[`# Original d3-geo 3.1.1 inverse/forward/stream oracle; bundle SHA-256 ${sha(bundle)}.`,
  '# C|id|projection|centerLon|centerLat|roll|cap|fit|scale|translateX|translateY|expectedScale|expectedTranslateX|expectedTranslateY. G|id|lon|lat. Q|id|x|y|expectedLon|expectedLat. B: singular-boundary spherical comparison only. H: exact cap, actual D3 inverse and strict point visibility.'];
const initialRows=[];
let cases=0,queries=0,positive=0,negative=0,boundaries=0,capBoundaries=0;
const list=value=>value.map(v=>Object.is(v,-0)?0:v);
function visible(p,point){let seen=false;p.stream({point(){seen=true;},lineStart(){},lineEnd(){},polygonStart(){},polygonEnd(){},sphere(){}}).point(...point);return seen;}
function expected(p,pixel){
  const candidate=p.invert(pixel);
  if(!candidate||!candidate.every(Number.isFinite)||Math.abs(candidate[0])>180+1e-10||Math.abs(candidate[1])>90+1e-10)return null;
  const round=p(candidate);
  if(!round.every(Number.isFinite)||Math.hypot(round[0]-pixel[0],round[1]-pixel[1])>1e-7||!visible(p,candidate))return null;
  return list(candidate);
}
function add(kind,view,fit='world',style=0,capOverride=null){
  const [cx,cy,roll]=view, cap=azimuthal.has(kind)?capOverride??(kind==='gnomonic'?60:90):null;
  const scale=style?120:1, translate=style?[310,210]:[0,0];
  const p=d3[constructors[kind]]().scale(scale).translate(translate).rotate([-cx,-cy,-roll]).precision(.000001);
  if(cap!==null)p.clipAngle(cap);
  let ring=[];
  if(fit==='data'){
    const rotate=d3.geoRotation([-cx,-cy,-roll]);
    ring=[[-20,-10],[-20,0],[-20,10],[0,10],[20,10],[20,0],[20,-10],[0,-10],[-20,-10]].map(v=>list(rotate.invert(v)));
    p.fitExtent([[42,32],[668,458]],{type:'Polygon',coordinates:[ring]});
  }
  const id=cases++;
  rows.push(['C',id,kind,cx,cy,roll,cap??'none',fit,fit==='data'?'auto':scale,...(fit==='data'?['auto','auto']:translate),p.scale(),...p.translate()].join('|'));
  for(const point of ring)rows.push(['G',id,...point].join('|'));
  const pixels=[];
  for(const x of[-3.5,-1.7,-.73,0,.31,.93,1.07,2.3,3.7])for(const y of[-3.5,-1.7,-.73,0,.31,.93,1.07,2.3,3.7])
    pixels.push([p.translate()[0]+x*p.scale(),p.translate()[1]+y*p.scale()]);
  for(const lon of[-170,-75,-10,35,95,170])for(const lat of[-70,-20,10,55,80])pixels.push(p([lon,lat]));
  for(const pixel of pixels){
    if(!pixel?.every(v=>Number.isFinite(v)&&Math.abs(v)<=1e6))continue;
    const candidate=p.invert(pixel), local=candidate?.every(Number.isFinite)?d3.geoRotation([-cx,-cy,-roll])(candidate):null;
    const round=local?p(candidate):null;
    // Exact cap boundaries have a deliberately separate profile: original D3
    // point streams use a strict cap, while Kartis's accepted forward policy
    // includes a bounded floating-point tolerance. Keep D3's actual inverse.
    if(cap!==null&&local&&Math.hypot(round[0]-pixel[0],round[1]-pixel[1])<1e-7&&Math.abs(Math.cos(local[0]*Math.PI/180)*Math.cos(local[1]*Math.PI/180)-Math.cos(cap*Math.PI/180))<1e-12){
      const position=expected(p,pixel);
      const original=['Q',id,...list(pixel),...(position??['none','none'])].join('|');
      initialRows.push({index:rows.length,original});
      rows.push(['H',id,...list(pixel),...list(candidate),visible(p,candidate)].join('|'));capBoundaries++;continue;
    }
    const position=expected(p,pixel);
    rows.push(['Q',id,...list(pixel),...(position??['none','none'])].join('|'));queries++;if(position)positive++;else negative++;
  }
  for(const point of[[-180,0],[180,0],[0,-90],[0,90]]){
    const pixel=p(point), inverse=p.invert(pixel);
    if(pixel.every(v=>Number.isFinite(v)&&Math.abs(v)<=1e6)&&inverse?.every(v=>Number.isFinite(v))&&Math.abs(inverse[1])<=90+1e-10){
      rows.push(['B',id,...list(pixel),...list(inverse)].join('|'));boundaries++;
    }
  }
}
for(const kind of Object.keys(constructors)){
  for(const view of views)for(const style of[0,1])add(kind,view,'world',style);
  for(const view of views)add(kind,view,'data');
  if(azimuthal.has(kind))for(const cap of[30,75])add(kind,[35,20,25],'world',1,cap);
}
const contents=rows.join('\n')+'\n';
const originalRows=rows.slice();
originalRows[1]='# C|id|projection|centerLon|centerLat|roll|cap|fit|scale|translateX|translateY|expectedScale|expectedTranslateX|expectedTranslateY. G|id|lon|lat. Q|id|x|y|expectedLon|expectedLat. B: singular-boundary spherical comparison only.';
for(const {index,original} of initialRows)originalRows[index]=original;
// Preserve the first-run oracle evidence, including strict D3 horizon results.
await writeFile(join(target,'geo-inverse-first-oracle.csv'),originalRows.join('\n')+'\n');
if(process.argv.includes('--check'))assert.equal(await readFile(join(root,paths.queries),'utf8'),contents,'Original D3 geographic inverse oracle changed');
else await writeFile(join(root,paths.queries),contents);
const receipt={passed:true,cases,queries,positive,negative,boundaries,capBoundaries,frame,
  firstOracleSha256:sha(originalRows.join('\n')+'\n'),
  exactCapProfile:'H preserves D3 invert and strict point-stream visibility, separately asserting the existing inclusive Kartis forward cap tolerance; excluded from ordinary Q equality.',
  scope:'Numerical inverse and actual D3 point-stream visibility; Kartis additionally checks Scene budgets and exposes explicit plot-window clipping. Singular poles/seams use spherical-position equivalence.',
  sourceManifestSha256:sha(await readFile(join(root,paths.manifest))),bundleSha256:sha(bundle),queriesSha256:sha(contents),
  sourceFiles:manifest.originalFiles};
await writeFile(join(target,'geo-inverse-reference-receipt.json'),JSON.stringify(receipt,null,2)+'\n');
console.log(JSON.stringify(receipt));
