// Execute actual Nivo computePoints/getNodeSizeGenerator and actual scale
// factories/pipeline; strip TypeScript only. Lodash adapters are restricted to
// the primitive/native-Date fixture inputs. D3 uses its real runtime source.
import {readFile,writeFile} from 'node:fs/promises';
import {execFileSync} from 'node:child_process';
import {stripTypeScriptTypes,createRequire} from 'node:module';
import {fileURLToPath,pathToFileURL} from 'node:url';
import {join} from 'node:path';
import {createHash} from 'node:crypto';
const workspace=fileURLToPath(new URL('../',import.meta.url));
const target=JSON.parse(execFileSync('cargo',['metadata','--no-deps','--format-version','1'],{cwd:workspace,encoding:'utf8',windowsHide:true})).target_directory;
const nivo=process.argv.slice(2).find(s=>!s.startsWith('--'))??'E:/Repos/nivo';
const scaleRoot=join(nivo,'packages/scales/src'),sources={};
const packageRoot=join(target,'d3-color-reference/node_modules');
const referenceRequire=createRequire(join(target,'d3-color-reference/package.json'));
for(const [name,expected] of [['d3-scale','4.0.2'],['d3-array','3.2.5'],['d3-time','3.1.1']]){
 const actual=JSON.parse(await readFile(join(packageRoot,name,'package.json'),'utf8')).version;
 if(actual!==expected)throw Error(`Expected ${name} ${expected}, got ${actual}`);
}
const adapters={
 get:'const get=(object,path)=>path.split(".").reduce((value,key)=>value?.[key],object);',
 isString:'const isString=value=>typeof value==="string";',
 isNumber:'const isNumber=value=>typeof value==="number";',
 isPlainObject:'const isPlainObject=value=>value!==null&&Object.getPrototypeOf(value)===Object.prototype;',
 uniq:'const uniq=values=>[...new Set(values)];',
 uniqBy:'const uniqBy=(values,key)=>{const seen=new Set();return values.filter(value=>{const k=key(value);if(seen.has(k))return false;seen.add(k);return true;});};',
 sortBy:'const sortBy=(values,key)=>values.slice().sort((a,b)=>{a=key(a);b=key(b);return a<b?-1:a>b?1:0;});',
 last:'const last=values=>values[values.length-1];',
 isDate:'const isDate=value=>value instanceof Date;',
};
const modules=new Map();
async function load(relative){
 if(modules.has(relative))return modules.get(relative);
 const full=join(nivo,'packages',relative),original=await readFile(full,'utf8');
 sources[relative]=createHash('sha256').update(original).digest('hex');
 let source=original.replace(/import\s+\{[^}]*\}\s+from\s+'(?:\.\/types|@nivo\/scales)'\s*\r?\n/g,'');
 // stripTypeScriptTypes cannot know which mixed D3 named imports are types.
 source=source.replace(/import\s*\{([^}]*)\}\s*from\s*'(d3-[^']+)'/g,(_,names,dependency)=>'import {'+names.split(',').map(s=>s.trim()).filter(s=>s&&!/^[A-Z]/.test(s)).join(',')+"} from '"+dependency+"'");
 source=source.replace(/import (\w+) from 'lodash\/([^']+)\.js'/g,(_,name,key)=>{
  if(name!==key||!adapters[key])throw Error(`Unsupported lodash import ${name}/${key}`);return adapters[key];
 });
 const imports=[...source.matchAll(/from '([^']+)'/g)];
 for(const [,specifier] of imports){
  let url;
  if(specifier.startsWith('d3-'))url=pathToFileURL(referenceRequire.resolve(specifier)).href;
  else if(specifier.startsWith('./'))url=await load('scales/src/'+specifier.slice(2)+'.ts');
  else throw Error('Unexpected actual-source dependency '+specifier);
  source=source.replaceAll("from '"+specifier+"'","from "+JSON.stringify(url));
 }
 const compiled=stripTypeScriptTypes(source,{mode:'transform'});
 const url='data:text/javascript;base64,'+Buffer.from(compiled).toString('base64');modules.set(relative,url);return url;
}
const scatter=await import(await load('scatterplot/src/compute.ts'));
const scales=await import(await load('scales/src/compute.ts'));
const props=await readFile(join(nivo,'packages/scatterplot/src/props.tsx'),'utf8');
sources['scatterplot/src/props.tsx']=createHash('sha256').update(props).digest('hex');
if(!/nodeSize:\s*9,/.test(props))throw Error('Actual Scatter default size changed');
const base=[{id:'Alpha',data:[{id:'a',x:2,y:4,metric:-10},{id:'b',x:5,y:8,metric:0},{id:'c',x:9,y:3,metric:50}]},{id:'Beta',data:[{id:'d',x:4,y:6,metric:100},{id:'e',x:7,y:1,metric:150}]}];
const profiles=[
 {name:'default',data:base,x:{type:'linear'},y:{type:'linear'},size:9},
 {name:'dynamic',data:base,x:{type:'linear',min:0,max:10,nice:false},y:{type:'linear',min:0,max:10,nice:false},size:{key:'data.metric',values:[0,100],sizes:[10,30]}},
 {name:'constant-size-domain',data:base,x:{type:'linear',min:0,max:10,nice:false},y:{type:'linear',min:0,max:10,nice:false},size:{key:'data.metric',values:[5,5],sizes:[2,18]}},
 {name:'descending-size-domain',data:base,x:{type:'linear',min:0,max:10,nice:false},y:{type:'linear',min:0,max:10,nice:false},size:{key:'data.metric',values:[100,0],sizes:[8,24]}},
 {name:'constant-xy',data:[{id:'Alpha',data:[{id:'a',x:5,y:2,metric:50},{id:'b',x:5,y:2,metric:150}]}],x:{type:'linear',min:'auto',max:'auto'},y:{type:'linear',min:'auto',max:'auto'},size:9},
 {name:'reverse-round',data:base,x:{type:'linear',min:0,max:10,nice:false,reverse:true,round:true},y:{type:'linear',min:0,max:10,nice:false,reverse:true,round:true},size:9},
 {name:'point-xy',data:[{id:'Alpha',data:[{id:'a',x:'1',y:'up',metric:10},{id:'b',x:1,y:'down',metric:40},{id:'c',x:'last',y:'up',metric:90}]}],x:{type:'point'},y:{type:'point'},size:9},
 {name:'utc-x',data:[{id:'Alpha',data:[{id:'a',x:new Date('2024-02-28T00:00:00Z'),y:2,metric:10},{id:'b',x:new Date('2024-02-29T00:00:00Z'),y:6,metric:50},{id:'c',x:new Date('2024-03-01T00:00:00Z'),y:4,metric:100}]}],x:{type:'time',format:'native',useUTC:true,nice:false},y:{type:'linear',min:0,max:10,nice:false},size:9},
 {name:'negative-size-extension',data:base,x:{type:'linear',min:0,max:10,nice:false},y:{type:'linear',min:0,max:10,nice:false},size:{key:'data.metric',values:[0,100],sizes:[0,10]}},
];
const records=[],domains=[];
const type=value=>value instanceof Date?'t':typeof value==='number'?'n':'s';
const value=x=>x instanceof Date?x.getTime():x;
for(const [profileIndex,p] of profiles.entries())for(const [width,height] of [[400,240],[517,293]]){
 // Native Nivo mutates its nested copies during normalization; preserve the
 // fixture source and actual Date instances across all independent profiles.
 const computed=scales.computeXYScalesForSeries(structuredClone(p.data),p.x,p.y,width,height);
 const points=scatter.computePoints({series:computed.series,formatX:x=>'x:'+value(x),formatY:y=>'y:'+value(y),getNodeId:point=>point.serieId+':'+point.data.id});
 const size=scatter.getNodeSizeGenerator(p.size);
 domains.push([profileIndex,p.name,width,height,p.x.type,p.y.type,computed.xScale.domain().map(v=>type(v)+':'+value(v)).join(','),computed.yScale.domain().map(v=>type(v)+':'+value(v)).join(',')].join('|'));
 for(const node of points)records.push([profileIndex,p.name,width,height,node.id,node.index,node.serieIndex,node.serieId,type(node.xValue),value(node.xValue),type(node.yValue),value(node.yValue),node.data.metric,node.x,node.y,size(node),node.formattedX,node.formattedY].join('|'));
}
const header='# Actual Nivo 0.99 source SHA256 '+JSON.stringify(sources)+'; d3-scale4.0.2 d3-array3.2.5 d3-time3.1.1; top-left local plot coordinates.';
const outputs=[['scatter-nivo.csv',[header,'profile|name|width|height|id|index|serieIndex|series|xType|rawX|yType|rawY|sizeValue|pixelX|pixelY|diameter|formattedX|formattedY',...records]],['scatter-domains.csv',[header,'profile|name|width|height|xKind|yKind|xDomain|yDomain',...domains]]];
for(const [name,rows]of outputs){const output=rows.join('\n')+'\n',path=new URL('../tests/fixtures/'+name,import.meta.url);if(process.argv.includes('--check')){if((await readFile(path,'utf8')).replaceAll('\r\n','\n')!==output)throw Error('Scatter frozen oracle differs: '+name);}else await writeFile(path,output);}
console.log(JSON.stringify({profiles:profiles.length,points:records.length,domains:domains.length,sources}));
