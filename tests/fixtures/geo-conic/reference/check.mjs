import {readFile} from 'node:fs/promises';
import {resolve,dirname} from 'node:path';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
import {spawnSync} from 'node:child_process';
import vm from 'node:vm';
import assert from 'node:assert/strict';
import {originalUMD,originalESM} from './loader.mjs';
import {generate} from './generate.mjs';
const root=dirname(fileURLToPath(import.meta.url)),sha=b=>createHash('sha256').update(b).digest('hex');
if(process.argv.length!==3||process.argv[2]!=='--check')throw Error('Check-only producer: node check.mjs --check');
if(typeof vm.SourceTextModule!=='function'){const child=spawnSync(process.execPath,['--experimental-vm-modules',fileURLToPath(import.meta.url),'--check'],{stdio:'inherit',windowsHide:true});process.exit(child.status??1);}
const manifestBytes=await readFile(resolve(root,'fixtures/conic-d3-source.json')),manifest=JSON.parse(manifestBytes),fixtureBytes=await readFile(resolve(root,'fixtures/conic-fixtures.json')),fixtureManifest=JSON.parse(fixtureBytes);
// Frozen manifest digests are inserted by the one-time candidate freezer.
const sourceDigest='12cace196f9eb5b2d25a67c27e4bdc04843d03462727879db793eebeedcacb14',fixtureDigest='53abece7bd486be715ac7085a2d810243c594bcfa2762e6f522d064e69bab450';
assert.equal(sha(manifestBytes),sourceDigest,'Original source manifest changed');assert.equal(sha(fixtureBytes),fixtureDigest,'Frozen fixture manifest changed');
for(const file of manifest.files){const bytes=await readFile(resolve(root,file.path));assert.equal(bytes.length,file.bytes,file.path);assert.equal(sha(bytes),file.sha256,file.path);}
const bundle=await readFile(resolve(root,manifest.bundle.path)),licenses=await readFile(resolve(root,manifest.licenses.path));
assert.equal(sha(bundle),manifest.bundle.sha256);assert.equal(sha(licenses),manifest.licenses.sha256);
for(const p of manifest.packages){assert.equal(JSON.parse(await readFile(resolve(root,`vendor/${p.name}/package.json`))).version,p.version);if(p.start!==undefined){assert.equal(sha(bundle.subarray(p.start,p.end)),p.distSha256);assert.equal(sha(licenses.subarray(p.licenseStart,p.licenseEnd)),p.licenseSha256);}}
const prior=JSON.parse(await readFile(resolve(root,manifest.originalProbe.path)));
const umd=generate(await originalUMD(root),prior),esm=generate(await originalESM(root),prior);
let records=0;for(const [name,expected]of Object.entries(umd.files)){assert.equal(esm.files[name],expected,`Original geo ESM differs from original UMD: ${name}`);const actual=await readFile(resolve(root,'fixtures',name),'utf8'),entry=fixtureManifest.files.find(file=>file.path===`fixtures/${name}`);assert(entry,name);assert.equal(actual,expected,`Original UMD fixture changed: ${name}`);assert.equal(sha(actual),entry.sha256);assert.equal(Buffer.byteLength(actual),entry.bytes);records+=actual.trimEnd().split('\n').length;}
assert.equal(Object.keys(umd.files).length,fixtureManifest.files.length);
console.log(JSON.stringify({passed:true,scope:'Candidate original UMD / original geo ESM record-exact reference preparation only, not Rust implementation or feature acceptance',checkOnly:true,sourceFiles:manifest.files.length,fixtures:fixtureManifest.files.length,records,counts:umd.counts,bundleSha256:manifest.bundle.sha256,sourceManifestSha256:sourceDigest,fixtureManifestSha256:fixtureDigest,dependencyScope:'geo 3.1.1 original ESM; array 3.2.5 original UMD exports bridged with SyntheticModule because original local package has no ESM source; no dependency rewrite, download or package installation',node:process.version}));
