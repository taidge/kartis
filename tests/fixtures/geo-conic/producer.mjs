import {readFile} from 'node:fs/promises';
import {dirname,resolve} from 'node:path';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
import {spawnSync} from 'node:child_process';
import assert from 'node:assert/strict';
import {derive} from './derive.mjs';
const root=dirname(fileURLToPath(import.meta.url)),sha=b=>createHash('sha256').update(b).digest('hex');
assert.deepEqual(process.argv.slice(2),['--check'],'check-only: node producer.mjs --check');
const inventoryBytes=await readFile(resolve(root,'migration-inventory.json'));
assert.equal(sha(inventoryBytes),'9c72438dbb299e187e127295d226923508e7371402e41ab1ba05ae4606ad8744');
for(const f of JSON.parse(inventoryBytes).files){const b=await readFile(resolve(root,'reference',f.path));assert.equal(b.length,f.bytes,f.path);assert.equal(sha(b),f.sha256,f.path);}
const result=spawnSync(process.execPath,[resolve(root,'reference/check.mjs'),'--check'],{encoding:'utf8',windowsHide:true,maxBuffer:2e6});
assert.equal(result.status,0,result.stdout+result.stderr);
const manifestBytes=await readFile(resolve(root,'fixtures/manifest.json'));
assert.equal(sha(manifestBytes),'686f10edaf62ff92f14c45c83fab959ed535008066da22ab4819c4cc12087ff4');
for(const f of JSON.parse(manifestBytes).files){const b=await readFile(resolve(root,'fixtures',f.path));assert.equal(b.length,f.bytes,f.path);assert.equal(sha(b),f.sha256,f.path);}
const output=await derive(root);
for(const [name,expected] of Object.entries(output.files))assert.equal(await readFile(resolve(root,'fixtures',name),'utf8'),expected,name);
console.log(JSON.stringify({passed:true,candidateOnly:true,original:JSON.parse(result.stdout.trim().split('\n').at(-1)),derived:output.counts}));
