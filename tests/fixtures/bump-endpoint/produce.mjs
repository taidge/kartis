import {readFileSync,writeFileSync} from 'node:fs';
import {stripTypeScriptTypes} from 'node:module';
import {createHash} from 'node:crypto';
const file=n=>new URL(n,import.meta.url);
const source=readFileSync(file('useBumpSeriesLabels.ts'),'utf8');
const pins=JSON.parse(readFileSync(file('source-pins.json'),'utf8'));
if(createHash('sha256').update(source).digest('hex')!==pins.excerptSha256) throw Error('Original hook bytes changed');
const code=stripTypeScriptTypes(source.replace('export const','const'));
const hook=new Function('useTheme','useInheritedColor','useMemo',code+';return useBumpSeriesLabels;')(
()=>({}), color=>{if(typeof color!== 'string')throw Error('fixed CSS only');return ()=>color;}, fn=>fn());
const rows=[];
for(const reverse of [false,true]) for(const opacity of [1,.5,0]) for(const gap of [false,true]) {
 const points=reverse?[[712,267],[0,89]]:[[0,89],[712,267]];
 if(gap)points[0]=[null,null];
 const series=[{id:'Alpha',data:{},linePoints:points,opacity}];
 for(const [position,padding,color] of [['start',8,'red'],['end',24,'blue']]) {
 const results=hook({series,position,padding,color,getLabel:true});
 rows.push(...results.map(({id,label,x,y,color,opacity,textAnchor})=>[+reverse,opacity,+gap,position,x+64,y+32,color,textAnchor,id,label].join('|')));
 }
}
const output='# Original Nivo hook, margin [64,32]; reverse/opacity/start-gap/side/x/y/css/anchor/id/text\n'+rows.join('\n')+'\n';
if(process.argv.includes('--check')){if(readFileSync(file('values.txt'),'utf8')!==output)throw Error('Oracle records changed');console.log(JSON.stringify({passed:true,records:rows.length}));}
else writeFileSync(file('values.txt'),output);
