// Closed-form source-key state machine, independently authored targets and D3 paths.
import assert from 'node:assert/strict';
import {spring,pathInterpolate} from './funnel-motion-reference.mjs';
import {computeShapeGenerators} from '../tests/fixtures/funnel-motion/shape-generators.mjs';

const close=(a,b,label)=>assert(Number.isFinite(a)&&Number.isFinite(b)&&Math.abs(a-b)<1e-7,`${label}: ${a} != ${b}`);
const sameState=(a,b,label)=>{assert(a?.length===3);a.forEach((v,i)=>close(v,b[i],label));};
const commandTokens=s=>s.match(/[MLCZ]|[-+]?(?:\d*\.\d+|\d+\.?\d*)(?:[eE][-+]?\d+)?/g)??[];
function samePath(actual,expected){const a=actual.flat(),e=commandTokens(expected);assert.equal(a.length,e.length);a.forEach((v,i)=>typeof v==='string'?assert.equal(v,e[i]):close(v,Number(e[i]),'D3 sampled path'));}
function paths(part,profile){const vertical=profile%2===0,[body,border]=computeShapeGenerators(profile<2?'linear':'smooth',vertical?'vertical':'horizontal');body.digits(15);border.digits(15);
 const points=part.profile.map(p=>vertical?{x0:p.before[0],x1:p.after[0],y:p.before[1]}:{y0:p.before[1],y1:p.after[1],x:p.before[0]});
 return {Body:body(points),Border:border([...part.profile.map(p=>({x:p.before[0],y:p.before[1]})),null,...part.profile.toReversed().map(p=>({x:p.after[0],y:p.after[1]}))])};}
function advance(s,dt){const a=spring({value:s[0],velocity:s[1]},s[2],dt);return Math.abs(a.value-s[2])<=.0001&&Math.abs(a.velocity)<=.0001?[s[2],0,s[2]]:[a.value,a.velocity,s[2]];}
const ids=['s1','n3ff0000000000000','stail'];
export function createDormantOracle(profile){
 let epoch=0,phase=0,width=3,opacity=1,reduced=false,tracks=new Map();
 return {
  verify(owner,operation=null){
   const m=owner.motion;assert(m);const previous=tracks;
   if(operation){const {name,a,b}=operation;if(name==='source'){epoch=a;phase=b;}if(name==='dormant-border'){width=a;opacity=b;}if(name==='preference')reduced=a===1;
    if(name==='step')for(const row of previous.values()){row.progress=advance(row.progress,a);for(const[k,s]of Object.entries(row.channels))row.channels[k]=advance(s,a);}}
   const expectedIDs=phase===7||phase===6?ids.slice(1):ids;
   assert.deepEqual(m.target.parts.map(p=>p.datum.id.identity),expectedIDs,'authored source keys');
   if(epoch===0&&phase===0)assert.equal(m.target.parts[0].datum.id.value,'1','known typed string key');
   const fresh=new Map();
   m.target.parts.forEach((part,i)=>{
    const id=part.datum.id.identity,index=ids.indexOf(id);assert(index>=0);const target=paths(part,profile),fill=(phase===0?[[190,40,50,255],[30,90,210,255],[20,160,70,255]]:[[40,130,200,255],[210,110,20,255],[160,40,170,255]])[index];
    const border=epoch===0?[20,30,40,255]:[100,70,20,255];
    assert.equal(part.body===null,phase===4||phase===5,'authored zero-size body membership');assert.equal(part.border===null,width===0||opacity===0,'authored border ink policy');
    for(const role of ['Body','Border']){
     const key=id+'/'+role,old=previous.get(key),targets=role==='Body'?Object.fromEntries(['FillRed','FillGreen','FillBlue','FillAlpha'].map((k,i)=>[k,fill[i]])):Object.fromEntries(['StrokeRed','StrokeGreen','StrokeBlue','StrokeAlpha','StrokeWidth'].map((k,i)=>[k,[...border,width][i]]));
     const changed=old&&old.target!==target[role];let progress=old?changed?[0,0,1]:old.progress:[1,0,1];if(reduced)progress=[1,0,1];
     const channels=Object.fromEntries(Object.entries(targets).map(([k,v])=>[k,reduced||!old?[v,0,v]:[old.channels[k][0],old.channels[k][1],v]]));
     const from=changed?old.target:old?.from??target[role],row={from,target:target[role],progress,channels};fresh.set(key,row);
     const actual=m.dormant[i][role];assert.deepEqual(Object.keys(actual.channels).sort(),Object.keys(channels).sort());sameState(actual.progress,progress,'persistent path progress');for(const[k,s]of Object.entries(channels))sameState(actual.channels[k],s,'persistent '+key+' '+k);
     const markID=part[role==='Body'?'body':'border'];if(markID){const mark=m.scene.marks.find(x=>x.id===markID),targetMark=m.targetScene.marks.find(x=>x.id===markID);assert(mark&&targetMark);samePath(targetMark.geometry.commands,target[role]);samePath(mark.geometry.commands,pathInterpolate(from,target[role],progress[0]));
      if(role==='Body')assert.deepEqual(mark.fill,Object.values(channels).map(s=>Math.round(Math.max(0,Math.min(255,s[0])))),'independent sampled fill');
      else if(channels.StrokeWidth[0]<=0)assert.equal(mark.stroke,null,'nonpositive physical stroke has no ink');else{assert(mark.stroke);close(mark.stroke.width,channels.StrokeWidth[0],'independent sampled stroke');assert.deepEqual(mark.stroke.color,['StrokeRed','StrokeGreen','StrokeBlue','StrokeAlpha'].map(k=>Math.round(Math.max(0,Math.min(255,channels[k][0])))));}
     }
    }
   });
   tracks=fresh;
   return {epoch,phase,width,opacity,reduced,tracks:[...tracks].map(([key,row])=>({key,...structuredClone(row)}))};
  }
 };
}
