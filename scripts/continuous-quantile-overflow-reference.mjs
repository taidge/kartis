// Independent small edge supplement. The seven original migrated files stay exact.
import assert from 'node:assert/strict';
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import vm from 'node:vm';
import {fileURLToPath} from 'node:url';
import {resolve,dirname} from 'node:path';
import {token} from './continuous-quantile-reference.mjs';
const sha=bytes=>createHash('sha256').update(bytes).digest('hex');
const FIXTURE_SHA='f004fc400e5c8e2e6f162b7573c3dc3db38da16d317221388c49e5e2ab3c549e';
const MANIFEST_SHA='3c655a4b1f06daa2272d0e2b802c096281518a1a767f0b7cb6050207390cd8bc';

export function generateOverflow(d3){
  const max=Number.MAX_VALUE,min=Number.MIN_VALUE;
  const cases=[['two-extremes',[-max,max]],['asymmetric-extremes',[-max,max/2]],['three-overflow-gap',[-max,-max/2,max]],['reversed-extremes',[max,-max]],['subnormal',[-min,0,min]],['mixed-zero',[0,-0]]];
  const lines=['# Original d3-scale 4.0.2 / d3-array 3.2.5. Separate finite-source R7 overflow profile; original 124-case fixtures are unchanged. C|id|source|sorted-domain; Q|id|profile|input|raw-rank; P|id|segments|profile|raw-quantiles. n:NaN/n:+Infinity/n:-Infinity/n:-0 and u preserve raw values.'];
  let queries=0,requests=0,finiteStatistics=0,nonfiniteStatistics=0;
  for(const[id,domain]of cases){
    const scale=d3.scaleSequentialQuantile().domain(domain);
    lines.push(['C',id,domain.map(token).join(','),scale.domain().map(token).join(',')].join('|'));
    for(const query of [...domain,0,-min,min,-Infinity,Infinity,NaN,null,undefined]){lines.push(['Q',id,typeof query==='number'&&Number.isFinite(query)?'finite-number':'native-query',token(query),token(scale(query))].join('|'));queries++;}
    for(const segments of [1,2,3,4,7,16,256]){const values=scale.quantiles(segments),finite=values.every(Number.isFinite);lines.push(['P',id,segments,finite?'finite-r7':'native-nonfinite-r7',values.map(token).join(',')].join('|'));requests++;if(finite)finiteStatistics++;else nonfiniteStatistics++;}
  }
  return{csv:lines.join('\n')+'\n',counts:{cases:cases.length,queries,quantileRequests:requests,finiteStatistics,nonfiniteStatistics}};
}

async function main(){
  const args=process.argv.slice(2),ri=args.indexOf('--receipt');assert(args.includes('--check'),'Use --check; original references are never rewritten');assert(args.every((arg,i)=>arg==='--check'||arg==='--receipt'||ri>=0&&i===ri+1),'Unknown argument');if(ri>=0)assert(args[ri+1]);
  const fixtures=new URL('../tests/fixtures/',import.meta.url),manifestBytes=await readFile(new URL('sequential-quantile-d3-source.json',fixtures));assert.equal(sha(manifestBytes),MANIFEST_SHA);const manifest=JSON.parse(manifestBytes);assert.equal(manifest.packages.find(p=>p.name==='d3-scale').version,'4.0.2');assert.equal(manifest.packages.find(p=>p.name==='d3-array').version,'3.2.5');
  const bundle=await readFile(new URL(manifest.bundle.file,fixtures));assert.equal(bundle.length,manifest.bundle.bytes);assert.equal(sha(bundle),manifest.bundle.sha256);for(const part of manifest.packages)assert.equal(sha(bundle.subarray(part.start,part.start+part.bytes)),part.sha256);
  const licenses=await readFile(new URL(manifest.licenses.file,fixtures));assert.equal(sha(licenses),manifest.licenses.sha256);for(const part of manifest.licenses.packages){assert.equal(sha(part.text),part.sha256);assert.equal(Buffer.byteLength(part.text),part.bytes);assert(licenses.toString('utf8').includes(`===== ${part.name} ${part.version} =====\n`+part.text));}
  const sources=await readFile(new URL(manifest.originalSources.file,fixtures));assert.equal(sha(sources),manifest.originalSources.sha256);for(const part of manifest.originalSources.segments)assert.equal(sha(sources.subarray(part.start,part.start+part.bytes)),part.sha256);assert.equal(sha(await readFile(new URL('./continuous-quantile-reference.mjs',import.meta.url))),manifest.producerSha256);
  const context=vm.createContext({});new vm.Script(bundle.toString('utf8')).runInContext(context,{timeout:5000});const actual=generateOverflow(context.d3),bytes=await readFile(new URL('sequential-quantile-overflow-d3.csv',fixtures));assert.equal(sha(bytes),FIXTURE_SHA);assert(bytes.equals(Buffer.from(actual.csv)),'Original D3 overflow supplement mismatch');assert(actual.counts.nonfiniteStatistics>0);
  const receipt={passed:true,scope:'Original finite-domain R7 overflow/subnormal supplement only; no Rust acceptance claim',manifestSha256:MANIFEST_SHA,bundleSha256:sha(bundle),fixtureSha256:sha(bytes),counts:actual.counts,originalMigratedFixturesUnchanged:true};const path=ri>=0?resolve(args[ri+1]):fileURLToPath(new URL('../target/sequential-quantile-overflow-reference-receipt.json',import.meta.url));await mkdir(dirname(path),{recursive:true});await writeFile(path,JSON.stringify(receipt,null,2)+'\n');console.log(JSON.stringify(receipt));
}
if(process.argv[1]&&resolve(process.argv[1])===fileURLToPath(import.meta.url))await main();
