import {readFile} from 'node:fs/promises';
import {resolve} from 'node:path';
import assert from 'node:assert/strict';
const decode=v=>Array.isArray(v)?v.map(decode):v==='n:-0'?-0:v==='n:NaN'?NaN:v==='n:+Infinity'?Infinity:v==='n:-Infinity'?-Infinity:v;
const scalar=v=>Number.isNaN(v)?'NaN':v===Infinity?'inf':v===-Infinity?'-inf':Object.is(v,-0)?'-0':String(v);
export async function derive(root){
 const read=async n=>(await readFile(resolve(root,'reference/fixtures',n),'utf8')).trimEnd().split('\n').map(l=>l.split('|').map(c=>decode(JSON.parse(c))));
 const rows=await read('conic-raw.csv'),critical=await read('conic-critical.csv'),defaults=await read('conic-defaults.csv');
 const profiles=new Map(),ordinary=[],allForward=[];
 const cone=(kind,[a,b])=>{[a,b]=[a,b].map(v=>v*(Math.PI/180));let n;
  if(kind==='equalArea')n=(Math.sin(a)+Math.sin(b))/2;
  else if(kind==='equidistant')n=a===b?Math.sin(a):(Math.cos(a)-Math.cos(b))/(b-a);
  else n=a===b?Math.sin(a):Math.log(Math.cos(a)/Math.cos(b))/Math.log(Math.tan((Math.PI/2+b)/2)/Math.tan((Math.PI/2+a)/2));
  return (kind!=='conformal'&&Math.abs(n)<1e-6)||n===0?null:n;
 };
 const line=r=>r.map(scalar).join(',');
 for(const r of rows){if(r[0]==='C'){profiles.set(r[1],{kind:r[2],p:r[3]});continue;}
  const p=profiles.get(r[1]),values=[p.kind,...p.p,...r[2],...r[3],...r[4]];
  allForward.push(line(values));
  if(Math.abs(r[3][0])<Math.PI&&Math.abs(r[3][1])<=85*Math.PI/180&&p.p.every(v=>Math.abs(v)<=75)&&(cone(p.kind,p.p)===null||Math.abs(cone(p.kind,p.p))>=1e-3))ordinary.push(line(values));
 }
 assert.equal(ordinary.length,8208);
 const criticalLines=critical.map(r=>line([r[1],...r[2],...r[3],...r[4],r[8]]));
 assert.equal(criticalLines.length,72);assert.equal(critical.filter(r=>r[8]==='original-sign-zero-inverse-error').length,22);
 const albers=defaults.filter(r=>r[0]==='A').map(r=>line([...r[1],(r[2][0]-480)/1070,(250-r[2][1])/1070]));
 const files={'ordinary.csv':ordinary.join('\n')+'\n','critical.csv':criticalLines.join('\n')+'\n','all-native-forward.csv':allForward.join('\n')+'\n','albers-unit.csv':albers.join('\n')+'\n'};
 return {files,counts:{ordinary:ordinary.length,critical:critical.length,nativeBadInverse:22,nativeForward:allForward.length,albers:albers.length}};
}


