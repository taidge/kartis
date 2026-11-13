// Optional frozen oracle, not a chart runtime dependency. Uses the owning
// workspace's normal Cargo target directory and the existing color references.
import {execFileSync} from 'node:child_process';
import {readFile,writeFile} from 'node:fs/promises';
import {fileURLToPath,pathToFileURL} from 'node:url';
import {join} from 'node:path';
const root=fileURLToPath(new URL('../',import.meta.url));
const metadata=JSON.parse(execFileSync('cargo',['metadata','--no-deps','--format-version','1'],{cwd:root,encoding:'utf8',windowsHide:true}));
const modules=join(metadata.target_directory,'d3-color-reference','node_modules');
const scaleVersion=JSON.parse(await readFile(join(modules,'d3-scale/package.json'),'utf8')).version;
const arrayVersion=JSON.parse(await readFile(join(modules,'d3-array/package.json'),'utf8')).version;
if(scaleVersion!=='4.0.2'||arrayVersion!=='3.2.5')throw Error(`Unexpected oracle versions ${scaleVersion}/${arrayVersion}`);
const {scaleLinear}=await import(pathToFileURL(join(modules,'d3-scale/src/index.js')).href);
const domains=[[.01,.71],[0,14.1],[.99,1.1],[.013,.083],[2.55,2.66],[-.71,-.01],[-.01,.71],[-1.1,-.99],[1e-250,7.31e-250],[1e100,1.47e100],[1.001,1.0010001],[0,1],[7,7],[0,0],[-0,-0],[5e-200,8e-200],[-101,-4.7],[234.1,947.9],[-.125,.987],[.00007,.00019]];
// D3's decimal ceil/floor can expand these by one more tick on a subsequent
// call. Preserve actual behavior instead of assuming exact idempotence.
domains.push([.99,1.11],[.99,1.12]);
// A deterministic set of additional non-round endpoints catches changes in
// decimal powers and step factors, rather than only reproducing one bug.
for(let i=0;i<24;i++){const p=10**(i%13-6),lo=((i*17)%31-15+.013)*p;domains.push([lo,lo+(1+(i*11)%17+.071)*p]);}
const rows=[`# Actual d3-scale ${scaleVersion} / d3-array ${arrayVersion} scaleLinear().nice(count).domain().`,'lo|hi|count|nice0|nice1'];
for(const domain of domains)for(const reverse of [false,true])for(const count of [0,1,2,5,10,17,1000]){
 const d=reverse?[...domain].reverse():domain,r=scaleLinear().domain(d).nice(count).domain();
 if(!r.every(Number.isFinite))throw Error('Non-finite oracle profile');
 rows.push([...d,count,...r].join('|'));
}
await writeFile(new URL('../tests/fixtures/linear-nice-d3.csv',import.meta.url),rows.join('\n')+'\n');
console.log(`${rows.length-2} independent linear nice queries written`);
