// Execute original d3-zoom rescaleX/rescaleY and d3-scale. No Rust input/output.
import {execFileSync} from 'node:child_process';
import {readFile, writeFile} from 'node:fs/promises';
import {join} from 'node:path';
import {fileURLToPath, pathToFileURL} from 'node:url';
import {createHash} from 'node:crypto';
const root = fileURLToPath(new URL('../', import.meta.url));
const target = JSON.parse(execFileSync('cargo', ['metadata', '--offline', '--no-deps', '--format-version', '1'],
    {cwd:root, encoding:'utf8', windowsHide:true})).target_directory;
const modules = join(target, 'piecewise-viewport-reference/node_modules');
const scaleRoot = join(modules, 'd3-scale'), zoomRoot = join(modules, 'd3-zoom');
const versions = await Promise.all([scaleRoot,zoomRoot].map(async p=>JSON.parse(await readFile(join(p,'package.json'),'utf8')).version));
if (versions[0] !== '4.0.2' || versions[1] !== '3.0.0') throw Error('Expected d3-scale 4.0.2 and d3-zoom 3.0.0');
const d3 = await import(pathToFileURL(join(scaleRoot,'src/index.js')));
const {Transform} = await import(pathToFileURL(join(zoomRoot,'src/transform.js')));
const sha = b=>createHash('sha256').update(b).digest('hex');
const hashes = await Promise.all([
    [scaleRoot,'src/continuous.js'], [scaleRoot,'src/time.js'], [zoomRoot,'src/transform.js'],
].map(async([p,f])=>f+':'+sha(await readFile(join(p,f)))));
const make = (kind,p)=>kind==='linear'?d3.scaleLinear():kind==='log'?d3.scaleLog().base(p):
    kind==='symlog'?d3.scaleSymlog().constant(p):kind==='sqrt'?d3.scaleSqrt():
    kind==='utc'?d3.scaleUtc():d3.scalePow().exponent(p);
const profiles = [
    ['linear',1,[-.7,2.1,9.3]], ['linear',1,[23,8,1,-5]],
    ['log',10,[.3,2,80]], ['log',2,[-180,-9,-2,-.3]],
    ['symlog',.5,[-23,-1,0,3,71]], ['pow',2,[-3,-1,0,2,8]],
    ['pow',-1,[1,2,8,32]], ['pow',-1,[-32,-8,-2,-1]],
    ['sqrt',.5,[-9,0,4,25]], ['utc',1,[0,1800000,90000000]],
];
const rows = ['# Actual d3-scale 4.0.2 + d3-zoom 3.0.0; '+hashes.join('; ')+
    '. kind|parameter|sourceStops|fractions|nice(-1=off)|reverse|round|axis|start|span|displayStops|rescaledStops'];
for (const [kind,p,domain] of profiles) for (const nice of [-1,3]) {
    if (kind==='pow' && p<0 && nice!==-1) continue;
    const fractions = domain.map((_,i)=>Math.pow(i/(domain.length-1),1.7));
    for (const reverse of [0,1]) for (const round of [0,1]) for (const axis of ['x','y']) {
        const extent = axis==='x'?340.5:240.5;
        const rawRange = fractions.map(f=>axis==='x'?extent*f:extent*(1-f));
        const range = reverse?rawRange.toReversed():rawRange;
        const s = make(kind,p).domain(domain);
        round?s.rangeRound(range):s.range(range);
        if (nice!==-1) s.nice(nice);
        for (const [start,span] of [[0,1],[.2,.4],[.17,.25]]) {
            const k=1/span, offset=-start*extent*k;
            const transform = new Transform(k,axis==='x'?offset:0,axis==='y'?offset:0);
            const next = axis==='x'?transform.rescaleX(s):transform.rescaleY(s);
            rows.push([kind,p,domain.join(','),fractions.join(','),nice,reverse,round,axis,start,span,
                s.domain().map(Number).join(','),next.domain().map(Number).join(',')].join('|'));
        }
    }
}
const text = rows.join('\n')+'\n';
const output = new URL('../tests/fixtures/piecewise-viewport-d3.csv',import.meta.url);
if (process.argv.includes('--check')) {
    if (await readFile(output,'utf8')!==text) throw Error('Original full-breakpoint viewport oracle changed');
} else await writeFile(output,text);
console.log(JSON.stringify({scaleVersion:versions[0],zoomVersion:versions[1],records:rows.length-1,sha256:sha(text)}));
