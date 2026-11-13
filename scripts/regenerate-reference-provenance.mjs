// Explicit source-reference regeneration after a project wrapper/schema rename.
// No Cargo, browser, download, dependency installation, or historical target edits.
// node --experimental-vm-modules scripts/regenerate-reference-provenance.mjs --generate
//   --geo-modules <existing d3-geo-reference/node_modules>
// node --experimental-vm-modules scripts/regenerate-reference-provenance.mjs --check
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import {existsSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {spawnSync} from 'node:child_process';
import {dirname,resolve,relative,join} from 'node:path';
import {fileURLToPath,pathToFileURL} from 'node:url';
import vm from 'node:vm';
import {generate as quantile} from './continuous-quantile-reference.mjs';
import {generateOverflow} from './continuous-quantile-overflow-reference.mjs';
import {originalUMD,originalESM} from '../tests/fixtures/geo-conic/reference/loader.mjs';
import {generate as conic} from '../tests/fixtures/geo-conic/reference/generate.mjs';
import {derive} from '../tests/fixtures/geo-conic/derive.mjs';

const root=resolve(dirname(fileURLToPath(import.meta.url)),'..');
const args=process.argv.slice(2),generate=args.includes('--generate'),check=args.includes('--check');
assert.notEqual(generate,check,'Choose exactly one of --generate or --check');
for(let i=0;i<args.length;i++) {
  if(['--generate','--check'].includes(args[i]))continue;
  assert.equal(args[i],'--geo-modules','Unknown option');assert(args[++i],'Missing --geo-modules');
}
assert.equal(typeof vm.SourceTextModule,'function','Use node --experimental-vm-modules');
const out=join(root,'target/kartis-rename-reference-regeneration');
await fs.mkdir(out,{recursive:true});
const sha=b=>createHash('sha256').update(b).digest('hex');
const json=v=>Buffer.from(JSON.stringify(v,null,2)+'\n');
const read=p=>fs.readFile(resolve(root,p));
const object=async p=>JSON.parse(await read(p));
const before=new Map(),changed=[],replays=[],commands=[];
async function remember(p){if(before.has(p))return;const b=await read(p);before.set(p,b);const dest=join(out,'before',p);await fs.mkdir(dirname(dest),{recursive:true});if(!existsSync(dest))await fs.writeFile(dest,b);}
async function write(p,bytes){
  const dest=resolve(root,p);assert(dest.startsWith(root+'/'.replace('/',process.platform==='win32'?'\\':'/')));assert(!relative(root,dest).split(/[\\/]/).includes('target'));
  bytes=Buffer.from(bytes);const old=await read(p);
  if(check){assert(old.equals(bytes),`Regenerated provenance differs: ${p}`);return;}
  await remember(p);if(!old.equals(bytes)){await fs.writeFile(dest,bytes);changed.push(p);}
}
async function rewrite(p,transform){await write(p,transform((await read(p)).toString('utf8')));}
async function literal(p,old,next){
  const text=(await read(p)).toString('utf8');
  if(old===next||!text.includes(old)){assert(text.includes(next),`Missing pin in ${p}`);return;}
  await write(p,text.replaceAll(old,next));
}
async function entries(m,base){
  for(const f of m.files){const b=await read(join(base,f.path));
    // Official source bytes are never re-pinned to make this migration pass.
    if(f.path.startsWith('vendor/')){assert.equal(sha(b),f.sha256,f.path);assert.equal(b.length,f.bytes,f.path);}
    f.bytes=b.length;f.sha256=sha(b);
  }
  return m;
}
function run(script,extra){
  const command=[process.execPath,'--experimental-vm-modules',script,...extra];
  const result=spawnSync(command[0],command.slice(1),{cwd:root,encoding:'utf8',windowsHide:true,maxBuffer:8e6});
  const index=commands.length;commands.push({command,exitCode:result.status,error:result.error?.message??null});
  return fs.writeFile(join(out,`${check?'check':'generate'}-command-${index}.log`),(result.stdout??'')+(result.stderr??'')).then(()=>{assert.equal(result.status,0,`${script}: ${result.stdout}\n${result.stderr}`);});
}
async function exactFiles(result,base,name){
  for(const [file,expected]of Object.entries(result.files)){
    const actual=await read(join(base,file));assert(actual.equals(Buffer.from(expected)),`Original numeric output changed: ${base}/${file}`);
  }
  replays.push({name,files:Object.keys(result.files).length,counts:result.counts,numericFilesByteExact:true});
}

// The only package-source input is an explicit existing normal-project source
// directory. Generated reports go in this new prefix, not old oracle receipts.
const modulesIndex=args.indexOf('--geo-modules');
const modules=modulesIndex<0?join(root,'target/d3-geo-reference/node_modules'):resolve(args[modulesIndex+1]);
const geoBefore=await object('tests/fixtures/geo-inverse-d3-source.json');
const inverseBefore=await read('tests/fixtures/geo-inverse-d3.csv');
const originalPackagePins=geoBefore.packages.map(p=>({name:p.name,distSha256:p.distSha256,licenseSha256:p.licenseSha256}));
if(generate){
  for(const p of ['tests/fixtures/geo-inverse-d3-oracle.js','tests/fixtures/geo-inverse-d3-LICENSE.txt','tests/fixtures/geo-inverse-d3-source.json','tests/fixtures/geo-inverse-d3.csv'])await remember(p);
  await run('scripts/geo-inverse-reference.mjs',['--generate','--modules',modules,'--output-directory',join(out,'geo-inverse')]);
}else await run('scripts/geo-inverse-reference.mjs',['--check','--output-directory',join(out,'geo-inverse-check')]);
const geo=await object('tests/fixtures/geo-inverse-d3-source.json');
assert.deepEqual(geo.packages.map(p=>({name:p.name,distSha256:p.distSha256,licenseSha256:p.licenseSha256})),originalPackagePins,'Original package distributions/licenses must stay exact');
const inverseAfter=await read('tests/fixtures/geo-inverse-d3.csv');
assert.equal(inverseAfter.toString().split('\n').slice(1).join('\n'),inverseBefore.toString().split('\n').slice(1).join('\n'),'Inverse numeric rows must stay exact; only bundle header changes');
replays.push({name:'inverse',numericRowsByteExact:true,packageDistributionsByteExact:true});

// Execute every original UMD and original ESM fixture before changing hashes.
const ref='tests/fixtures/geo-conic/reference';
const umd=await originalUMD(resolve(root,ref)),esm=await originalESM(resolve(root,ref));
const prior=await object(ref+'/provenance/original-next-probe.json');
const c=conic(umd,prior),ce=conic(esm,prior);assert.deepEqual(c,ce,'Original conic UMD/ESM record equality');
await exactFiles(c,ref+'/fixtures','conic UMD + ESM');
const derived=await derive(resolve(root,'tests/fixtures/geo-conic'));await exactFiles(derived,'tests/fixtures/geo-conic/fixtures','derived conic');
for(const name of ['geo-conic-unsafe','geo-spherical','geo-spherical-relations']){
  const mod=await import(pathToFileURL(resolve(root,'tests/fixtures',name,'generate.mjs')));
  const a=mod.generate(umd),b=mod.generate(esm);assert.deepEqual(a,b,`${name}: original UMD/ESM`);
  await exactFiles(a,'tests/fixtures/'+name,name);
}

// Rebuild current package provenance, retaining the immutable original probe
// and original source files as separate historical inputs. Offsets come from
// the actual newly assembled wrapper, not an assumed one-byte adjustment.
const sourcePath=ref+'/fixtures/conic-d3-source.json',source=await object(sourcePath);
source.bundle={...source.bundle,bytes:geo.bundle.bytes,sha256:geo.bundle.sha256};
for(const p of source.packages){const next=geo.packages.find(n=>n.name===p.name);if(next)for(const k of ['start','end','distSha256','licenseStart','licenseEnd','licenseSha256'])p[k]=next[k];}
await entries(source,ref);await write(sourcePath,json(source));
const rawManifestPath=ref+'/fixtures/conic-fixtures.json',rawManifest=await object(rawManifestPath);
await entries(rawManifest,ref);await write(rawManifestPath,json(rawManifest));
const sourceDigest=sha(await read(sourcePath)),fixtureDigest=sha(await read(rawManifestPath));
await rewrite(ref+'/check.mjs',s=>s.replace(/sourceDigest='[a-f0-9]{64}',fixtureDigest='[a-f0-9]{64}'/,`sourceDigest='${sourceDigest}',fixtureDigest='${fixtureDigest}'`));
const inventoryPath='tests/fixtures/geo-conic/migration-inventory.json',inventory=await object(inventoryPath);
await entries(inventory,ref);await write(inventoryPath,json(inventory));
const inventoryDigest=sha(await read(inventoryPath));
await rewrite('tests/fixtures/geo-conic/producer.mjs',s=>s.replace(/assert.equal\(sha\(inventoryBytes\),'[a-f0-9]{64}'\)/,`assert.equal(sha(inventoryBytes),'${inventoryDigest}')`));
for(const name of ['geo-conic-unsafe','geo-spherical','geo-spherical-relations']){
  const base='tests/fixtures/'+name,path=base+'/source-and-fixtures.json',m=await object(path);
  await entries(m,base);await write(path,json(m));const digest=sha(await read(path));
  await rewrite(base+'/producer.mjs',s=>s.replace(/assert.equal\(sha\((manifest|bytes)\),'[a-f0-9]{64}'\)/,(_,v)=>`assert.equal(sha(${v}),'${digest}')`));
}
const oldBundle=(await object('tests/fixtures/geo-spherical-stability/source-and-fixtures.json')).originalBundleSha256;
for(const p of ['tests/fixtures/geo-spherical-stability/original-check.mjs','tests/fixtures/conic-midpoint/generate.mjs','scripts/geo-conic-gallery-geometry.js','scripts/geo-spherical-gallery-geometry.js'])await literal(p,oldBundle,geo.bundle.sha256);
const stabilityPath='tests/fixtures/geo-spherical-stability/source-and-fixtures.json',stability=await object(stabilityPath);
stability.originalBundleSha256=geo.bundle.sha256;
// This manifest uses fixtures rather than files, all relative to its directory.
for(const f of stability.fixtures){const b=await read('tests/fixtures/geo-spherical-stability/'+f.path);f.sha256=sha(b);}
await write(stabilityPath,json(stability));
const midpointPath='tests/fixtures/conic-midpoint/source-and-fixture.json',midpoint=await object(midpointPath);
midpoint.producerSha256=sha(await read('tests/fixtures/conic-midpoint/generate.mjs'));
await write(midpointPath,json(midpoint));

// Execute the pinned original sequential-quantile bundle, including original
// nonfinite/coercion profiles. Only the project schema differs after renaming.
const sq='tests/fixtures/',quantilePath=sq+'sequential-quantile-d3-source.json',qm=await object(quantilePath);
const qb=await read(sq+qm.bundle.file);assert.equal(qb.length,qm.bundle.bytes);assert.equal(sha(qb),qm.bundle.sha256);
for(const p of qm.packages)assert.equal(sha(qb.subarray(p.start,p.start+p.bytes)),p.sha256,p.name);
const qlicense=await read(sq+qm.licenses.file);assert.equal(sha(qlicense),qm.licenses.sha256);
const qsource=await read(sq+qm.originalSources.file);assert.equal(sha(qsource),qm.originalSources.sha256);
for(const p of qm.originalSources.segments)assert.equal(sha(qsource.subarray(p.start,p.start+p.bytes)),p.sha256);
const qcontext=vm.createContext({});new vm.Script(qb.toString('utf8')).runInContext(qcontext,{timeout:5000});
const q=quantile(qcontext.d3),overflow=generateOverflow(qcontext.d3);
assert.deepEqual(q.counts,qm.counts);
assert.equal(q.csv,(await read(sq+'sequential-quantile-d3.csv')).toString(),'Original sequential-quantile numeric CSV unchanged');
assert.deepEqual(JSON.parse(q.json),await object(sq+'sequential-quantile-d3.json'),'Original sequential-quantile JSON data/schema matches actual generator');
assert.equal(overflow.csv,(await read(sq+'sequential-quantile-overflow-d3.csv')).toString(),'Original overflow outputs unchanged');
for(const [file,value]of Object.entries({'sequential-quantile-d3.json':q.json,'sequential-quantile-d3.csv':q.csv})){
  await write(sq+file,value);qm.fixtures[file]={...qm.fixtures[file],bytes:Buffer.byteLength(value),sha256:sha(value)};
}
qm.producerSha256=sha(await read('scripts/continuous-quantile-reference.mjs'));
await write(quantilePath,json(qm));
const quantileDigest=sha(await read(quantilePath));
await rewrite('scripts/continuous-quantile-overflow-reference.mjs',s=>s.replace(/const MANIFEST_SHA='[a-f0-9]{64}'/,`const MANIFEST_SHA='${quantileDigest}'`));
replays.push({name:'sequential quantile',counts:q.counts,numericCSVByteExact:true,jsonActualOriginalExecutionExact:true,overflowNumericCSVByteExact:true,overflowCounts:overflow.counts});

// Existing checked producers run only after actual outputs and full current
// provenance have been regenerated. Their tolerances/algorithms are unchanged.
for(const p of ['tests/fixtures/geo-conic/producer.mjs','tests/fixtures/geo-conic-unsafe/producer.mjs','tests/fixtures/geo-spherical/producer.mjs','tests/fixtures/geo-spherical-relations/producer.mjs','tests/fixtures/geo-spherical-stability/original-check.mjs','tests/fixtures/conic-midpoint/generate.mjs','scripts/continuous-quantile-reference.mjs','scripts/continuous-quantile-overflow-reference.mjs']){
  const extra=['--check'];
  if(p.startsWith('scripts/continuous-quantile'))extra.push('--receipt',join(out,(check?'check-':'generate-')+p.split('/').at(-1).replace('.mjs','.json')));
  await run(p,extra);
}
const diffs=[];
for(const [path,b]of before){const after=await read(path);if(!b.equals(after))diffs.push({path,beforeSha256:sha(b),sha256:sha(after),beforeBytes:b.length,bytes:after.length});}
const receipt={schema:'kartis-reference-provenance-regeneration-v1',passed:true,mode:generate?'generate':'check',at:new Date().toISOString(),commands,replays,changedFiles:diffs,
  originalGeoPackagePins:originalPackagePins,currentGeoBundle:geo.bundle,currentConicSourceSHA256:sourceDigest,currentConicFixtureSHA256:fixtureDigest,currentSequentialManifestSHA256:quantileDigest,
  scope:{cargoExecuted:false,browserExecuted:false,downloadOrInstall:false,historicalTargetEvidenceModified:false,fontsModified:false,originalUpstreamSourceBytesUnchanged:true,numericTolerancesChanged:false},
  historicalQualification:'Original probe/plan source and prior target evidence remain historical inputs. Newly generated current wrapper offsets/provenance describe Kartis; old original-probe bundle references are not retrospectively rewritten.',
  limits:['Gallery helper source pins are migrated after original numerical execution, but browser gallery acceptance is not run here.','No Rust/Cargo, browser, CPU rendering, arbitrary-device or feature-acceptance claim.']};
const dest=join(out,generate?'generation-receipt.json':'check-receipt.json');await fs.writeFile(dest,json(receipt));
console.log(JSON.stringify({passed:true,mode:receipt.mode,replays:replays.map(p=>({name:p.name,counts:p.counts})),changedFiles:diffs.length,receipt:dest,sha256:sha(await fs.readFile(dest))}));
