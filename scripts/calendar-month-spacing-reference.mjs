// Execute complete original Nivo Calendar functions, not a translated layout.
// node scripts/calendar-month-spacing-reference.mjs <released-node_modules> [--check]
import fs from 'node:fs';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {stripTypeScriptTypes} from 'node:module';
import {spawnSync} from 'node:child_process';
import vm from 'node:vm';
import {dirname,resolve,join,relative} from 'node:path';
import {fileURLToPath,pathToFileURL} from 'node:url';
const script=fileURLToPath(import.meta.url),root=resolve(dirname(script),'..');
const args=process.argv.slice(2),check=args.includes('--check');assert(args[0],'Supply existing released node_modules');
if(!args.includes('--utc-child')){
  const p=spawnSync(process.execPath,[script,...args,'--utc-child'],{env:{...process.env,TZ:'UTC'},stdio:'inherit',windowsHide:true});process.exit(p.status??1);
}
assert.equal(new Date(2024,0,31).getTimezoneOffset(),0,'Explicit child UTC Date profile');
const modules=resolve(args[0]),fixture=join(root,'tests/fixtures/calendar-month-spacing'),vendor=join(fixture,'vendor');
const sha=b=>createHash('sha256').update(b).digest('hex'),pins=JSON.parse(fs.readFileSync(join(vendor,'source-pins.json')));
for(const [name,p]of Object.entries(pins.files)){const b=fs.readFileSync(join(vendor,name));assert.equal(sha(b),p.sha256,name);assert.equal(b.length,p.bytes,name);}
const runtimePins={},versions={'d3-time':'3.1.1','d3-time-format':'4.1.1','d3-array':'3.2.5'};
for(const [name,version]of Object.entries(versions)){
  const base=join(modules,name);assert.equal(JSON.parse(fs.readFileSync(join(base,'package.json'))).version,version);
  function walk(dir){for(const e of fs.readdirSync(dir,{withFileTypes:true})){const p=join(dir,e.name);if(e.isDirectory())walk(p);else if(/\.(js|mjs|json)$/.test(e.name)||e.name==='LICENSE')runtimePins[name+'/'+relative(base,p).replaceAll('\\','/')]=sha(fs.readFileSync(p));}}
  walk(base);
}
// The existing official packages are dist-only. Execute their complete UMD
// distributions in order; do not synthesize or translate the time algorithms.
const context=vm.createContext({});
for(const name of ['d3-array','d3-time','d3-time-format'])new vm.Script(fs.readFileSync(join(modules,name,'dist',name+'.js'),'utf8'),{filename:name}).runInContext(context);
const time=context.d3,format=context.d3;
const _=(await import(pathToFileURL(join(vendor,'lodash.cjs')))).default;assert.equal(_.VERSION,'4.17.21');
const core=fs.readFileSync(join(vendor,'core-cartesian-utils.js'),'utf8').replaceAll('export const','const');
const alignBox=new Function(core+'\nreturn alignBox;')();
const source=fs.readFileSync(join(vendor,'calendar.ts'),'utf8').replaceAll('\r\n','\n').replace(/^import[\s\S]*?from\s*['"][^'"]+['"]\n/gm,'');
const js=stripTypeScriptTypes(source,{mode:'transform'}).replaceAll('export const','const');
const original=new Function('isDate','memoize','range','alignBox','timeFormat','timeDays','timeWeek','timeWeeks','timeMonths','timeYear',js+'\nreturn {computeLayout,bindDaysData,computeYearLegendPositions,computeMonthLegendPositions};')(_.isDate,_.memoize,_.range,alignBox,format.timeFormat,time.timeDays,time.timeWeek,time.timeWeeks,time.timeMonths,time.timeYear);
function vertices(path){
  const t=path.match(/[MHVZ]|-?\d+(?:\.\d*)?(?:e[+-]?\d+)?/gi);let x=0,y=0,i=0,points=[];
  while(i<t.length){const c=t[i++];if(c==='M'){x=Number(t[i++]);y=Number(t[i++]);}else if(c==='H')x=Number(t[i++]);else if(c==='V')y=Number(t[i++]);else {assert.equal(c,'Z');continue;}points.push([x,y]);}
  return points;
}
const specs=[];
for(const direction of ['horizontal','vertical'])for(const [first,last,spacing]of [[2024,2024,0],[2024,2024,8],[2024,2024,22],[2023,2024,6],[2028,2029,12]]){
  const n=last-first+1,yearSpacing=14,cross=70*n+yearSpacing*(n-1);
  specs.push({id:`${direction}-${first}-${last}-${spacing}`,direction,first,last,monthSpacing:spacing,yearSpacing,width:direction==='horizontal'?1000:cross,height:direction==='horizontal'?cross:1000});
}
const cases=specs.map(s=>{
  const p=original.computeLayout({...s,from:new Date(s.first,0,1),to:new Date(s.last,11,31),daySpacing:0,align:'center'});
  const dates=[{day:`${s.first}-01-31`,value:0},{day:`${s.first}-02-01`,value:7}];
  const bound=original.bindDaysData({days:p.days,data:dates,colorScale:v=>`value:${v}`,emptyColor:'empty'});
  const years=_.range(s.first,s.last+1);
  const nativeReservedWeekColumns=Math.max(...years.map(y=>time.timeWeeks(new Date(y,0,1),new Date(y+1,0,1)).length))+1;
  const occupiedWeekColumns=Math.max(...years.map(y=>Math.ceil((time.timeDay.count(new Date(y,0,1),new Date(y+1,0,1))+new Date(y,0,1).getDay())/7)));
  const weekAxis=s.direction==='horizontal'?s.width:s.height;
  const crossAxis=s.direction==='horizontal'?s.height:s.width;
  const crossCell=(crossAxis-(years.length-1)*s.yearSpacing)/(7*years.length);
  const nativeCell=Math.min((weekAxis-12*s.monthSpacing)/nativeReservedWeekColumns,crossCell);
  const occupiedCell=Math.min((weekAxis-12*s.monthSpacing)/occupiedWeekColumns,crossCell);
  assert.equal(nativeCell,10);assert.equal(occupiedCell,10);assert.equal(p.cellSize,nativeCell);
  const nativeWeekOrigin=(weekAxis-nativeCell*nativeReservedWeekColumns-12*s.monthSpacing)/2;
  const occupiedWeekOrigin=(weekAxis-occupiedCell*occupiedWeekColumns-12*s.monthSpacing)/2;
  assert.equal(s.direction==='horizontal'?p.originX:p.originY,nativeWeekOrigin);
  const occupiedWeekOriginShift=occupiedWeekOrigin-nativeWeekOrigin;
  return{...s,cellSize:p.cellSize,calendarWidth:p.calendarWidth,calendarHeight:p.calendarHeight,origin:[p.originX,p.originY],years:p.years,
    occupiedWeekColumns,nativeReservedWeekColumns,nativeWeekOrigin,occupiedWeekOrigin,occupiedWeekOriginShift,
    days:bound.map(d=>({date:[d.date.getFullYear(),d.date.getMonth()+1,d.date.getDate()],day:d.day,x:d.x,y:d.y,size:d.size,provided:d.data!==undefined,value:d.value??null,color:d.color})),
    months:p.months.map(m=>({year:m.year,month:m.month+1,bbox:m.bbox,path:m.path,vertices:vertices(m.path)})),
    monthLabels:original.computeMonthLegendPositions({months:p.months,direction:s.direction,position:'before',offset:10}).map(m=>({year:m.year,month:m.month+1,x:m.x,y:m.y,rotation:m.rotation})),
    yearLabels:original.computeYearLegendPositions({years:p.years,direction:s.direction,position:'before',offset:10})};
});
const witness=cases.find(c=>c.id==='horizontal-2024-2024-8');const jan=witness.days.find(d=>d.day==='2024-01-31'),feb=witness.days.find(d=>d.day==='2024-02-01');assert.equal(feb.x-jan.x,8);
const qualifications=[-1,NaN,Infinity].map(monthSpacing=>{const p=original.computeLayout({width:1000,height:70,direction:'horizontal',from:new Date(2024,0,1),to:new Date(2024,11,31),yearSpacing:0,monthSpacing,daySpacing:0,align:'center'});return{monthSpacing,cellSize:p.cellSize,firstPosition:[p.days[0].x,p.days[0].y],checkedRust:'reject invalid/nonfinite spacing, not a permissive-native parity profile'};});
const encode=(_,v)=>typeof v==='number'&&!Number.isFinite(v)?`n:${v}`:v;
const value=JSON.stringify({schema:1,profile:'Original Calendar FullYears, native Date in explicit UTC child, Sunday weeks, square cells, daySpacing0, center alignment. Native Sunday-Jan1 extra reserved week is preserved; Kartis legacy uses occupied week columns. Height-limited10px fixtures expose the fixed half-unused-week origin difference separately, not an inferred fit. Headers/weekday and arbitrary weekday/rectangular/union-padding extensions qualified separately.',cases,qualifications},encode,2)+'\n';
const reference=JSON.stringify({schema:1,nivoVersion:pins.nivoVersion,nivoCommit:pins.nivoCommit,versions,runtimePins,producerSHA256:sha(fs.readFileSync(script)),sourcePinsSHA256:sha(fs.readFileSync(join(vendor,'source-pins.json'))),valuesSHA256:sha(value),counts:{cases:cases.length,days:cases.reduce((n,c)=>n+c.days.length,0),months:cases.reduce((n,c)=>n+c.months.length,0),nativeInvalidProfiles:qualifications.length},witness:{id:witness.id,january31X:jan.x,february1X:feb.x,delta:8},scope:'Whole original computeLayout/bindDaysData/month/year legend-position functions executed with real Lodash and original alignment function. Source-only numeric oracle; not React/render/pixel acceptance.'},null,2)+'\n';
for(const[name,text]of [['values.json',value],['reference.json',reference]]){const p=join(fixture,name);if(check)assert.equal(fs.readFileSync(p,'utf8'),text,name);else fs.writeFileSync(p,text);}
console.log(JSON.stringify({passed:true,checked:check,cases:cases.length,days:cases.reduce((n,c)=>n+c.days.length,0),months:cases.reduce((n,c)=>n+c.months.length,0),valuesSHA256:sha(value),nativeInvalidProfiles:qualifications.length}));
