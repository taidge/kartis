// Execute only the original source bodies: no rewritten eligibility/position formula.
import {readFile} from 'node:fs/promises';
import {stripTypeScriptTypes} from 'node:module';
import {createHash} from 'node:crypto';
const base=new URL('./',import.meta.url),read=p=>readFile(new URL(p,base),'utf8');
const pins=JSON.parse(await read('source-pins.json'));
for(const p of pins.records){const b=await readFile(new URL(p.vendoredPath,base));if(createHash('sha256').update(b).digest('hex')!==p.sha256)throw Error(`Original source hash mismatch ${p.sourcePath}`);}
const hooks=await read('packages/bar/src/hooks.ts');
const body=/const shouldRenderBarLabel = useCallback\(\s*\(\{ width, height \}: \{ height: number; width: number \}\) => \{([\s\S]*?)\n        \},/.exec(hooks)?.[1];
if(!body)throw Error('Original callback not found');
const shouldRender=new Function('enableLabel','labelSkipWidth','labelSkipHeight','width','height',body);
const common=await read('packages/bar/src/compute/common.ts');
const positionSource=common.slice(common.indexOf('export function useComputeLabelLayout'));
const positionJS=stripTypeScriptTypes(positionSource,{mode:'strip'}).replace('export function','function');
const getPosition=new Function('commonDefaultProps',`${positionJS}; return useComputeLabelLayout;`)({layout:'vertical',labelPosition:'middle',labelOffset:0});
const records=[];
for(const horizontal of [false,true])for(const enabled of [false,true])for(const [width,height] of [[0,20],[20,0],[0,0],[20,20]])for(const skip of [0,.001])for(const position of ['start','middle','end']){
 const labelSkipWidth=horizontal?skip:0,labelSkipHeight=horizontal?0:skip;
 records.push({horizontal,enabled,width,height,labelSkipWidth,labelSkipHeight,position,offset:3,nativeShouldRender:shouldRender(enabled,labelSkipWidth,labelSkipHeight,width,height),nativePosition:getPosition(horizontal?'horizontal':'vertical',false,position,3)(width,height)});
}
const output=JSON.stringify({scope:'Original shouldRenderBarLabel and useComputeLabelLayout source bodies; dimensions include zero-category/both-zero native acceptance deliberately outside Rust checked opt-in',sourceHashes:pins.records,records},null,2)+'\n';
const fixturePins=JSON.parse(await read('fixture-pins.json'));
for(const row of fixturePins.files){const data=await readFile(new URL(row.path,base));if(createHash('sha256').update(data).digest('hex')!==row.sha256)throw Error(`Fixture pin mismatch ${row.path}`);}
if(output!==await read('nivo-original.json'))throw Error('Original Nivo re-execution differs from preserved records');
console.log(JSON.stringify({status:'PASS',originalRecords:records.length,noRustOutputRead:true,originalSourceHashesChecked:true,fixtureBytesChecked:true}));
