// Execute actual local Nivo hooks, replacing only React useMemo and framework
// color/format/accessor adapters. Geometric calculations are never rewritten.
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {execFileSync} from 'node:child_process';
import {fileURLToPath,pathToFileURL} from 'node:url';
import {join} from 'node:path';
import {stripTypeScriptTypes} from 'node:module';
import {createHash} from 'node:crypto';
const workspace=fileURLToPath(new URL('../',import.meta.url));
const target=JSON.parse(execFileSync('cargo',['metadata','--no-deps','--format-version','1'],{cwd:workspace,encoding:'utf8',windowsHide:true})).target_directory;
const nivo=process.argv[2]??'E:/Repos/nivo';
const scalePath=join(nivo,'packages/scales/src/linearScale.ts'),hooksPath=join(nivo,'packages/marimekko/src/hooks.ts');
const shapes=pathToFileURL(join(target,'d3-reference/node_modules/d3-shape/src/index.js')).href;
const scales=pathToFileURL(join(target,'d3-color-reference/node_modules/d3-scale/src/index.js')).href;
const versions={};for(const [name,dir]of [['scale','d3-color-reference/node_modules/d3-scale'],['array','d3-color-reference/node_modules/d3-array'],['shape','d3-reference/node_modules/d3-shape']])versions[name]=JSON.parse(await readFile(join(target,dir,'package.json'),'utf8')).version;
const originalScale=await readFile(scalePath,'utf8'),originalHooks=await readFile(hooksPath,'utf8');
const sha=s=>createHash('sha256').update(s).digest('hex');
let scale=originalScale.replace(/import[^\n]+from 'd3-scale'/,`import {scaleLinear} from ${JSON.stringify(scales)}`).replace(/import[^\n]+from 'd3-interpolate'/,`import {interpolateRound,interpolateNumber} from ${JSON.stringify(pathToFileURL(join(target,'d3-color-reference/node_modules/d3-interpolate/src/index.js')).href)}`).replace(/import[^\n]+from '\.\/types'/,'');
const scaleCode=stripTypeScriptTypes(scale,{mode:'transform'});const scaleModule='data:text/javascript;base64,'+Buffer.from(scaleCode).toString('base64');
let hooks=originalHooks.replace(/import[\s\S]*?from ['"][^'"]+['"]/g,'');
hooks=`import {stack as d3Stack,stackOffsetNone,stackOffsetExpand,stackOffsetDiverging,stackOffsetSilhouette,stackOffsetWiggle} from ${JSON.stringify(shapes)};
import {createLinearScale} from ${JSON.stringify(scaleModule)};
const useMemo=fn=>fn();const get=(datum,key,fallback)=>datum[key]??fallback;
const useValueFormatter=formatter=>formatter??(value=>String(value));
const useTheme=()=>({});const useInheritedColor=()=>()=> '#000';const useOrdinalColorScale=()=>()=> '#000';
const offsetById={none:stackOffsetNone,expand:stackOffsetExpand,diverging:stackOffsetDiverging,silouhette:stackOffsetSilhouette,wiggle:stackOffsetWiggle};
`+hooks;
const code=stripTypeScriptTypes(hooks,{mode:'transform'});const api=await import('data:text/javascript;base64,'+Buffer.from(code).toString('base64'));
const cases=[
 [{id:'A',weight:1,p:2,q:6},{id:'B',weight:3,p:1,q:3}],
 [{id:'A',weight:2,p:1,q:3},{id:'zero',weight:0,p:20,q:10},{id:'C',weight:5,p:4,q:2}],
 [{id:'A',weight:3,p:-2,q:3,r:-4},{id:'B',weight:4,p:4,q:-1,r:2}],
 [{id:'A',weight:0.01,p:0.01,q:0.11},{id:'B',weight:0.71,p:0.71,q:0.31}],
];
const rows=[`# Actual Nivo hooks SHA256 ${sha(originalHooks)}; linearScale SHA256 ${sha(originalScale)}; versions ${JSON.stringify(versions)}; innerWidth400 innerHeight240.`,`case|offset|horizontal|outer|inner|category|dimension|x|y|width|height|start|end|displayed|thicknessMax|dimensionsMin|dimensionsMax`];
for(const [caseId,data]of cases.entries())for(const offset of(caseId===2?['none','diverging']:['none','expand','diverging','silouhette','wiggle']))for(const horizontal of[false,true])for(const[outer,inner]of[[0,0],[5,3]]){
 const dimensions=Object.keys(data[0]).filter(k=>!['id','weight'].includes(k)).map(id=>({id,value:id}));
 const result=api.useMarimekko({data,id:'id',value:'weight',dimensions,layout:horizontal?'horizontal':'vertical',offset,outerPadding:outer,innerPadding:inner,width:400,height:240,borderWidth:0});
 const {dimensionIds,dimensions:accessors}=api.useDataDimensions(dimensions);const stacked=api.useStackedData(api.useStack(dimensionIds,accessors,offset),data).stacked;
 for(const[ci,category]of result.computedData.entries())for(const d of category.dimensions){const s=stacked.find(layer=>layer.key===d.id)[ci];rows.push([caseId,offset,Number(horizontal),outer,inner,ci,d.id,d.x,d.y,d.width,d.height,s[0],s[1],d.value,result.thicknessScale.domain()[1],...result.dimensionsScale.domain()].join('|'));}
}
await mkdir(new URL('../tests/fixtures/',import.meta.url),{recursive:true});await writeFile(new URL('../tests/fixtures/marimekko-nivo.csv',import.meta.url),rows.join('\n')+'\n');console.log(JSON.stringify({rows:rows.length-2,versions,sha256:sha(originalHooks)}));
