import {readFile} from 'node:fs/promises';
import {resolve,dirname} from 'node:path';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
import {spawnSync} from 'node:child_process';
import vm from 'node:vm';
import assert from 'node:assert/strict';
import {originalESM} from '../geo-conic/reference/loader.mjs';
import {generate} from './generate.mjs';
const root=dirname(fileURLToPath(import.meta.url)),sha=b=>createHash('sha256').update(b).digest('hex');assert.deepEqual(process.argv.slice(2),['--check']);
if(typeof vm.SourceTextModule!=='function'){const p=spawnSync(process.execPath,['--experimental-vm-modules',fileURLToPath(import.meta.url),'--check'],{stdio:'inherit',windowsHide:true});process.exit(p.status??1);}
const base=spawnSync(process.execPath,[resolve(root,'../geo-conic/producer.mjs'),'--check'],{encoding:'utf8',windowsHide:true,maxBuffer:2e6});assert.equal(base.status,0,base.stdout+base.stderr);
const manifest=await readFile(resolve(root,'source-and-fixtures.json'));assert.equal(sha(manifest),'b9dc8353906d7fbc75ec6df119e692295555e2e40b5e0c36b1ca1ae12cc2676f');const m=JSON.parse(manifest);
for(const f of m.files){const b=await readFile(resolve(root,f.path));assert.equal(b.length,f.bytes,f.path);assert.equal(sha(b),f.sha256,f.path);}
const ctx=vm.createContext({});new vm.Script(await readFile(resolve(root,'../geo-inverse-d3-oracle.js'),'utf8')).runInContext(ctx);
const umd=generate(ctx.d3),esm=generate(await originalESM(resolve(root,'../geo-conic/reference')));assert.deepEqual(esm,umd,'Original geo ESM / canonical UMD supplemental outputs differ');
for(const[name,bytes]of Object.entries(umd.files))assert.equal(await readFile(resolve(root,name),'utf8'),bytes,name);
console.log(JSON.stringify({passed:true,supplement:true,checkOnly:true,counts:umd.counts,baseRecords:266732,source:'Original geo3.1.1 ESM with original array3.2.5 UMD bridge and canonical99a bundle; no new npm/dependency or source rewrite'}));
