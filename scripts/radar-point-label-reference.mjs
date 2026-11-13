// Execute the complete original accessor and complete original pure useMemo
// stage. No component mount, hook lifecycle, or dot geometry claim.
import fs from 'node:fs';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {stripTypeScriptTypes} from 'node:module';
import {fileURLToPath,pathToFileURL} from 'node:url';
import {dirname,resolve,join} from 'node:path';
const root=resolve(dirname(fileURLToPath(import.meta.url)),'..'),fixture=join(root,'tests/fixtures/radar-point-label'),vendor=join(fixture,'vendor'),sha=b=>createHash('sha256').update(b).digest('hex');
const pins=JSON.parse(fs.readFileSync(join(vendor,'source-pins.json')));
for(const[name,p]of Object.entries(pins.files)){const b=fs.readFileSync(join(vendor,name));assert.equal(sha(b),p.sha256);assert.equal(b.length,p.bytes);}
const lodash=(await import(pathToFileURL(join(vendor,'lodash.cjs')))).default;assert.equal(lodash.VERSION,'4.17.21');
const accessorSource=fs.readFileSync(join(vendor,'propertiesConverters.js'),'utf8').replace(/^import[^\n]*\n/gm,'').replaceAll('export const','const');
const getPropertyAccessor=new Function('isFunction','get','format','useMemo',accessorSource+'\nreturn getPropertyAccessor;')(lodash.isFunction,lodash.get,()=>{throw Error('unrequested format generator');},f=>f());
const dots=fs.readFileSync(join(vendor,'RadarDots.tsx'),'utf8'),start=dots.indexOf('    const points: PointProps[] = useMemo('),end=dots.indexOf('\n    return (',start);assert(start>0&&end>start);
const originalMemo=stripTypeScriptTypes(dots.slice(start,end),{mode:'transform'});
const run=new Function('data','keys','getIndex','colorByKey','formatValue','getLabel','enableLabel','fillColor','strokeColor','positionFromAngle','rotation','angleStep','radiusScale','useMemo',originalMemo+'\nreturn points;');
const defaultsSource=fs.readFileSync(join(vendor,'radar-defaults.ts'),'utf8').replace(/^import[^\n]*\n/gm,'');
const defaultsJS=stripTypeScriptTypes(defaultsSource,{mode:'transform'}).replaceAll('export const','const');
const publicDefaults=new Function('RadarGridLabel','RadarSliceTooltip',defaultsJS+'\nreturn svgDefaultProps;')(()=>{},()=>{});assert.equal(publicDefaults.dotLabel,'formattedValue');
assert.match(dots,/label = 'value'/,'separate internal destructuring fallback');
const data=[{axis:'speed',Revenue:7,Costs:7},{axis:'cost <&> Å café',Revenue:7,Costs:2},{axis:'risk',Revenue:3,Costs:0}],keys=['Revenue','Costs'],colorByKey={Revenue:'rgb(220,20,30)',Costs:'rgb(20,40,220)'};
const specs=[['public-default',publicDefaults.dotLabel],['index','index'],['key','key'],['value','value'],['formattedValue','formattedValue'],['color','color'],['missing','rawDatum.nonexistent'],['typed-function',d=>`${d.key}/${d.index}: ${d.formattedValue}`],['empty',()=>'' ],['unicode-escaping',d=>`<&> Å café ${d.index}`]];
const cases=[];
for(const[name,label]of specs){const calls=[],accessor=getPropertyAccessor(label),points=run(data,keys,d=>d.axis,colorByKey,(value,key)=>value.toFixed(1),d=>{calls.push({...d});return accessor(d);},true,d=>d.color,d=>d.color,()=>({}),0,Math.PI*2/data.length,x=>x,f=>f());assert.equal(calls.length,6);cases.push({name,calls,labels:points.map(p=>({key:p.key,label:p.label===undefined?null:p.label,input:p.data}))});}
let disabledCalls=0;const disabled=run(data,keys,d=>d.axis,colorByKey,v=>v.toFixed(1),()=>{disabledCalls++;return'forbidden';},false,d=>d.color,d=>d.color,()=>({}),0,1,x=>x,f=>f());assert.equal(disabledCalls,0);assert(disabled.every(p=>p.label===undefined));
const values={schema:'original-radar-point-label-accessor-v1',data,keys,cases,disabledCalls,publicDefaultAccessor:publicDefaults.dotLabel,internalFallbackAccessor:'value',scope:'The complete original getPropertyAccessor executes real Lodash. The complete original RadarDots useMemo stage builds pointData and calls getLabel. The complete original public defaults object also executes with component references stubbed and unasserted. useMemo is invoked synchronously; position/color plumbing is injected and unasserted. JSX is only source-pinned, never mounted. Original point iteration axis-major differs from native series-major paint order; comparisons use typed index/key identities.'};
const encoded=JSON.stringify(values,null,2)+'\n',reference={passed:true,producerSHA256:sha(fs.readFileSync(fileURLToPath(import.meta.url))),sourcePinsSHA256:sha(fs.readFileSync(join(vendor,'source-pins.json'))),memoSHA256:sha(Buffer.from(originalMemo)),valuesSHA256:sha(encoded),accessorProfiles:cases.length,actualPointDataRecords:cases.length*6,disabledAccessorCalls:disabledCalls};
for(const[name,b]of [['values.json',encoded],['reference.json',JSON.stringify(reference,null,2)+'\n']]){const p=join(fixture,name);if(process.argv.includes('--check'))assert.equal(fs.readFileSync(p,'utf8'),b);else fs.writeFileSync(p,b);}
console.log(JSON.stringify(reference));
