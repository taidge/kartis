import fs from 'node:fs';import vm from 'node:vm';import assert from'node:assert/strict';import{createHash}from'node:crypto';
assert.deepEqual(process.argv.slice(2),['--check']);
const root=new URL('./',import.meta.url),bundle=fs.readFileSync(new URL('../geo-inverse-d3-oracle.js',root));
assert.equal(createHash('sha256').update(bundle).digest('hex'),'873638c3542defc67135946f1dae3a1d0426c147989b339a8033ce39e4e92550');
const ctx=vm.createContext({});new vm.Script(bundle.toString()).runInContext(ctx);const d=ctx.d3;
const fixture=JSON.parse(fs.readFileSync(new URL('witnesses.json',root)));
for(const c of fixture.cases){const actual={area:d.geoArea(c.geometry),length:d.geoLength(c.geometry),centroid:d.geoCentroid(c.geometry),contains:c.queries.map(q=>d.geoContains(c.geometry,q))};assert.deepEqual(JSON.parse(JSON.stringify(actual)),c.original,String(c.id));}
console.log(JSON.stringify({passed:true,checkOnly:true,cases:fixture.cases.length,originalBadValuesPreserved:true,originalVersion:'d3-geo3.1.1/d3-array3.2.5 UMD'}));
