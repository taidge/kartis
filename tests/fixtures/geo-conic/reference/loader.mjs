import {readFile} from 'node:fs/promises';
import {resolve,dirname} from 'node:path';
import vm from 'node:vm';
import assert from 'node:assert/strict';
export async function originalUMD(root){const context=vm.createContext({});new vm.Script(await readFile(resolve(root,'../../geo-inverse-d3-oracle.js'),'utf8'),{filename:'unmodified-d3-geo-3.1.1-original-UMD.js'}).runInContext(context);return context.d3;}
export async function originalESM(root){
 assert.equal(typeof vm.SourceTextModule,'function','Use --experimental-vm-modules');
 const context=vm.createContext({}),arrayContext=vm.createContext({});
 new vm.Script(await readFile(resolve(root,'vendor/d3-array/dist/d3-array.js'),'utf8'),{filename:'unmodified-d3-array-3.2.5.js'}).runInContext(arrayContext);
 const names=Object.keys(arrayContext.d3),array=new vm.SyntheticModule(names,function(){for(const name of names)this.setExport(name,arrayContext.d3[name]);},{context,identifier:'original-d3-array-3.2.5-UMD-exports'}),cache=new Map();
 async function load(path){path=resolve(path);if(cache.has(path))return cache.get(path);const mod=new vm.SourceTextModule(await readFile(path,'utf8'),{context,identifier:path});cache.set(path,mod);return mod;}
 const main=await load(resolve(root,'vendor/d3-geo/src/index.js'));
 await main.link(async(specifier,referencing)=>{if(specifier==='d3-array')return array;assert(specifier.startsWith('.'),'Unexpected external ESM dependency');const path=resolve(dirname(referencing.identifier),specifier);assert(path.startsWith(resolve(root,'vendor/d3-geo/src')+ '/'.replace('/',process.platform==='win32'?'\\':'/')),'ESM import escaped source');return load(path);});
 await main.evaluate();return main.namespace;
}
