// Independent explicit-clock algebra and Scene projection; never imports Rust.
import assert from 'node:assert/strict';
import {spring,pathInterpolate} from './funnel-motion-reference.mjs';

const DEFAULT={mass:1,stiffness:170,damping:26,position_epsilon:.0001,velocity_epsilon:.0001};
const configOf=c=>({...DEFAULT,...(c?.spring??c??{})});
const copy=v=>structuredClone(v),same=(a,b)=>assert.deepEqual(a,b),clamp=(v,a,b)=>Math.min(b,Math.max(a,v));
const close=(a,b,label)=>assert(Number.isFinite(a)&&Number.isFinite(b)&&Math.abs(a-b)<=1e-8,`${label}: ${a} != ${b}`);
const byte=v=>clamp(Math.round(v),0,255);
const fillKeys=['FillRed','FillGreen','FillBlue','FillAlpha'],strokeKeys=['StrokeRed','StrokeGreen','StrokeBlue','StrokeAlpha'];
const labelKeys=['Red','Green','Blue','Alpha'];
const flat=commands=>commands.flatMap(c=>c.slice(1));
const pathString=commands=>commands.map(c=>c[0]+c.slice(1).join(',')).join('');
function parsePath(s) {
    const tokens=s.match(/[MLCZ]|[-+]?(?:\d*\.\d+|\d+\.?\d*)(?:[eE][-+]?\d+)?/g)??[],out=[];
    for(let i=0;i<tokens.length;) {const letter=tokens[i++],count={M:2,L:2,C:6,Z:0}[letter];assert(count!==undefined,'unexpected target command');out.push([letter,...tokens.slice(i,i+count).map(Number)]);i+=count;}
    return out;
}
function commandsEqual(actual,expected,label) {
    assert.equal(actual.length,expected.length,label+' command count');
    for(let i=0;i<expected.length;i++){assert.equal(actual[i][0],expected[i][0],label+' target command template');assert.equal(actual[i].length,expected[i].length);for(let j=1;j<expected[i].length;j++)close(actual[i][j],expected[i][j],label);}
}
function rect(x,y,w,h) {return {kind:'Rect',values:[x,y,Math.max(0,w),Math.max(0,h)],commands:[['M',x,y],['L',x+Math.max(0,w),y],['L',x+Math.max(0,w),y+Math.max(0,h)],['L',x,y+Math.max(0,h)],['Z']]};}
function circle(x,y,r) {
    r=Math.max(0,r);const k=r*4*(Math.SQRT2-1)/3;
    return {kind:'Circle',values:[x,y,r],commands:[['M',x+r,y],['C',x+r,y+k,x+k,y+r,x,y+r],['C',x-k,y+r,x-r,y+k,x-r,y],['C',x-r,y-k,x-k,y-r,x,y-r],['C',x+k,y-r,x+r,y-k,x+r,y],['Z']]};
}
function roundedRect(x,y,w,h,radius) {
    if(w<=0||h<=0)return {kind:'Path',values:null,commands:[]};
    const r=Math.min(radius,w/2,h/2);if(r<=0)return rect(x,y,w,h);
    // Four independent quarter-circle Beziers, with target-authored radius.
    const k=r*4*(Math.SQRT2-1)/3,commands=[['M',x+r,y],['L',x+w-r,y],['C',x+w-r+k,y,x+w,y+r-k,x+w,y+r],['L',x+w,y+h-r],['C',x+w,y+h-r+k,x+w-r+k,y+h,x+w-r,y+h],['L',x+r,y+h],['C',x+r-k,y+h,x,y+h-r+k,x,y+h-r],['L',x,y+r],['C',x,y+r-k,x+r-k,y,x+r,y],['Z']];
    return {kind:'Path',values:null,commands};
}
function replaceNumbers(template,values) {let cursor=0;const commands=template.commands.map(c=>[c[0],...c.slice(1).map(()=>values[cursor++])]);assert.equal(cursor,values.length);return {kind:'Path',values:null,commands};}
function geometryEqual(actual,expected,label) {assert.equal(actual.kind,expected.kind,label+' actual geometry kind');if(expected.values===null)assert.equal(actual.values,null);else{assert.equal(actual.values.length,expected.values.length);actual.values.forEach((v,i)=>close(v,expected.values[i],label+' primitive value'));}commandsEqual(actual.commands,expected.commands,label);}
function state(s,target,label) {assert(Array.isArray(s)&&s.length===3&&s.every(Number.isFinite),label+' finite public state');close(s[2],target,label+' independently derived target');}
function stateEqual(actual,expected,label) {assert(actual?.length===3&&expected?.length===3);actual.forEach((v,i)=>close(v,expected[i],label+' ['+i+']'));}
function integrate(s,dt,config) {
    if(dt===0||s[0]===s[2]&&s[1]===0)return copy(s);
    const n=spring({value:s[0],velocity:s[1]},s[2],dt,config);
    return config.damping>0&&Math.abs(n.value-s[2])<=config.position_epsilon&&Math.abs(n.velocity)<=config.velocity_epsilon?[s[2],0,s[2]]:[n.value,n.velocity,s[2]];
}
function motionOf(owner) {return owner?.motion??null;}
function envelope(owner) {
    const m=motionOf(owner);if(!m)return null;
    if(owner.data){same(m.target,owner.data.computed);same(m.targetScene,owner.data.scene);assert.equal(m.generation,owner.generation);assert.equal(m.publication,owner.publication);}
    if(owner.motionState)assert.equal(m.configuration,owner.motionState.configuration);
    const s=m.scene,t=m.targetScene;same(s.order,t.order);assert.equal(s.marks.length,t.marks.length,'no ghost marks');assert.equal(s.labels.length,t.labels.length,'no ghost labels');same(s.background,t.background);assert.equal(s.width,t.width);assert.equal(s.height,t.height);
    assert.equal(m.roles.length,t.marks.length);assert.equal(m.labelParts.length,t.labels.length);assert.equal(m.marks.length,t.marks.length);assert.equal(m.labels.length,t.labels.length);
    same(m.marks.map(x=>x.id),t.marks.map(x=>x.id));assert.equal(new Set(t.marks.map(x=>x.id)).size,t.marks.length);
    return m;
}
function roles(m) {
    const marks=new Map(),labels=new Map(),t=m.targetScene;
    m.roles.forEach((role,i)=>{
        if(!role)return;const mark=t.marks[i];
        if(role.kind==='body'||role.kind==='border') {
            const part=m.target.parts[role.partIndex];assert(part);assert.equal(mark.id,part[role.kind]);
            marks.set(i,{key:role.kind+':'+part.datum.id.identity,geometry:mark.geometry.kind==='Path'?'path':'numeric',colors:true});
        } else {
            assert.equal(role.kind,'separator');const sep=m.target.separators[role.separatorIndex];assert(sep);
            commandsEqual(mark.geometry.commands,[['M',...sep.from],['L',...sep.to]],'authored separator target');
            marks.set(i,{key:'separator:'+sep.side+':'+(sep.part?.identity??'terminal'),geometry:'numeric',colors:false});
        }
    });
    m.labelParts.forEach((p,i)=>{if(p!==null){const part=m.target.parts[p];assert(part);assert.equal(t.labels[i].text,part.label);labels.set(i,{key:'label:'+part.datum.id.identity,colors:true});}});
    const a=m.annotations;
    if(a) {
        let cursor=0;
        a.bindings.forEach((binding,b)=>{
            const markSlots=[],labelSlots=[];
            while(a.order[cursor]?.[0]==='M')markSlots.push(a.order[cursor++][1]+a.baseMarks);
            while(a.order[cursor]?.[0]==='L')labelSlots.push(a.order[cursor++][1]+a.baseLabels);
            assert(markSlots.length>=2&&markSlots.length<=4,'bounded built-in annotation roles, no custom-note role guessing');assert(labelSlots.length>=1&&labelSlots.length<=2,'bounded native text note slots');
            const isLink=i=>{const c=t.marks[i].geometry.commands;return c.length===3&&c[0][0]==='M'&&c.slice(1).every(c=>c[0]==='L');};
            const last=markSlots.pop();assert(isLink(last));marks.set(last,{key:`annotation:${b}:link`,geometry:'path',colors:false});
            if(isLink(markSlots[0])){const i=markSlots.shift();marks.set(i,{key:`annotation:${b}:link-halo`,geometry:'path',colors:false});}
            assert(markSlots.length>=1&&markSlots.length<=2);
            markSlots.forEach((i,n)=>marks.set(i,{key:`annotation:${b}:`+(n===markSlots.length-1?'shape':'shape-halo'),geometry:binding.kind==='Rect'?'annotation-rect':'annotation-circle',binding,colors:false}));
            labelSlots.forEach((i,n)=>labels.set(i,{key:`annotation:${b}:note:${n}`,colors:false}));
        });
        assert.equal(cursor,a.order.length,'all annotation slots accounted');
    }
    assert.equal(new Set([...marks.values(),...labels.values()].map(v=>v.key)).size,marks.size+labels.size,'independent structural role keys');
    return {marks,labels};
}
function targets(m,descriptors) {
    const out=new Map(),t=m.targetScene;
    descriptors.marks.forEach((d,i)=>{
        const mark=t.marks[i],channels={};
        if(d.colors){if(mark.fill)fillKeys.forEach((k,j)=>channels[k]=mark.fill[j]);if(mark.stroke){strokeKeys.forEach((k,j)=>channels[k]=mark.stroke.color[j]);channels.StrokeWidth=mark.stroke.width;}}
        let numbers;
        if(d.geometry==='path')numbers=[1];
        else if(d.geometry==='annotation-circle')numbers=[...d.binding.center,d.binding.size/2];
        else if(d.geometry==='annotation-rect'){const b=d.binding;numbers=[b.center[0]-b.width/2,b.center[1]-b.height/2,b.width,b.height];}
        else numbers=mark.geometry.values??flat(mark.geometry.commands);
        out.set(d.key,{...d,index:i,kind:'mark',target:mark,channels,numbers});
    });
    descriptors.labels.forEach((d,i)=>{const label=t.labels[i],channels={X:label.position[0],Y:label.position[1]};if(d.colors)labelKeys.forEach((k,j)=>channels[k]=label.color[j]);out.set(d.key,{...d,index:i,kind:'label',target:label,channels});});
    return out;
}
function stateMap(m,descriptors=roles(m)) {
    const out=new Map(),target=targets(m,descriptors);
    for(const [key,d]of target) {
        const holder=d.kind==='mark'?m.marks[d.index]:m.labels[d.index],channels=d.kind==='mark'?holder.channels:holder;
        same(Object.keys(channels).sort(),Object.keys(d.channels).sort());Object.entries(d.channels).forEach(([k,t])=>state(channels[k],t,key+':'+k));
        if(d.kind==='mark') {
            assert.equal(holder.progress!==null,d.geometry==='path');assert.equal(holder.geometry?.length,d.numbers.length);
            holder.geometry.forEach((s,i)=>state(s,d.numbers[i],key+':geometry'+i));
            if(d.geometry==='path')stateEqual(holder.progress,holder.geometry[0],key+':duplicate public progress');
        }
        out.set(key,{...d,targetChannels:d.channels,channels,geometryStates:d.kind==='mark'?holder.geometry:null});
    }
    return out;
}
function rawStates(m) {return [...stateMap(m).values()].flatMap(d=>[...Object.values(d.channels),...(d.geometryStates??[])]);}
function paint(owner,anchors) {
    const m=envelope(owner);if(!m)return {marks:0,labels:0,scalarStates:0,pathNumbers:0};
    const desc=roles(m),states=stateMap(m,desc),expected=copy(m.targetScene);let pathNumbers=0;
    m.targetScene.marks.forEach((mark,i)=>{
        const d=desc.marks.get(i),holder=m.marks[i];if(!d){assert.equal(holder.progress,null);assert.equal(holder.geometry,null);same(holder.channels,{});return;}
        const entry=states.get(d.key),numbers=entry.geometryStates.map(s=>s[0]);let geometry;
        if(d.geometry==='path') {
            assert(anchors.has(d.key),'missing prior public TARGET template '+d.key);
            const str=pathInterpolate(pathString(anchors.get(d.key).commands),pathString(mark.geometry.commands),numbers[0]);geometry={kind:'Path',values:null,commands:parsePath(str)};pathNumbers+=flat(geometry.commands).length;
        } else if(d.geometry==='annotation-circle')geometry=numbers[2]<=0?{kind:'Path',values:null,commands:[]}:circle(...numbers);
        else if(d.geometry==='annotation-rect')geometry=roundedRect(...numbers,d.binding.radius);
        else if(mark.geometry.kind==='Circle')geometry=circle(...numbers);
        else if(mark.geometry.kind==='Rect')geometry=rect(...numbers);
        else geometry=replaceNumbers(mark.geometry,numbers);
        expected.marks[i].geometry=geometry;
        const ch=entry.channels;if(mark.fill&&fillKeys.every(k=>ch[k]))expected.marks[i].fill=fillKeys.map(k=>byte(ch[k][0]));
        if(ch.StrokeWidth?.[0]<=0)expected.marks[i].stroke=null;
        else if(mark.stroke&&strokeKeys.every(k=>ch[k])){expected.marks[i].stroke.color=strokeKeys.map(k=>byte(ch[k][0]));expected.marks[i].stroke.width=clamp(ch.StrokeWidth[0],.000001,10000);}
    });
    m.targetScene.labels.forEach((label,i)=>{const d=desc.labels.get(i);if(!d){same(m.labels[i],{});return;}const ch=states.get(d.key).channels;expected.labels[i].position=[ch.X[0],ch.Y[0]];if(labelKeys.every(k=>ch[k]))expected.labels[i].color=labelKeys.map(k=>byte(ch[k][0]));});
    expected.marks.forEach((mark,i)=>{geometryEqual(m.scene.marks[i].geometry,mark.geometry,'actual paint '+mark.id);const a=copy(m.scene.marks[i]),b=copy(mark);delete a.geometry;delete b.geometry;same(a,b);});
    expected.labels.forEach((label,i)=>{const a=copy(m.scene.labels[i]);a.position.forEach((v,j)=>close(v,label.position[j],'paint label position'));delete a.position;const b=copy(label);delete b.position;same(a,b);});
    const ss=rawStates(m);assert.equal(m.settled,ss.every(s=>s[0]===s[2]&&s[1]===0),'settled reflects all physical channels');
    return {marks:expected.marks.length,labels:expected.labels.length,scalarStates:ss.length,pathNumbers};
}
function verifyStepStates(before,after,dt,config) {
    const a=envelope(before),b=envelope(after);assert(a&&b);assert(Number.isFinite(dt)&&dt>=0&&dt<=60);same(a.target,b.target);same(a.targetScene,b.targetScene);same(before.data,after.data);assert.equal(before.generation,after.generation);assert.equal(b.targetSerial,a.targetSerial);
    assert.equal(b.sampleSerial,a.sampleSerial+(dt>0&&!a.settled?1:0),'explicit sample serial');
    const left=stateMap(a),right=stateMap(b);same([...left.keys()].sort(),[...right.keys()].sort());let checked=0;
    for(const [key,d]of left){const n=right.get(key);for(const k of Object.keys(d.channels)){stateEqual(n.channels[k],integrate(d.channels[k],dt,config),key+':'+k);checked++;}if(d.geometryStates)d.geometryStates.forEach((s,i)=>{stateEqual(n.geometryStates[i],integrate(s,dt,config),key+':geometry'+i);checked++;});}
    return {physicalStates:checked};
}
function updateAnchors(before,after,anchors) {
    const a=envelope(before),b=envelope(after),left=a?stateMap(a):new Map(),right=stateMap(b),next=new Map();
    for(const [key,d]of right)if(d.geometry==='path') {const old=left.get(key);next.set(key,old?.geometry==='path'&&old.target.geometry.kind===d.target.geometry.kind?(JSON.stringify(old.target.geometry)===JSON.stringify(d.target.geometry)?copy(anchors.get(key)):copy(old.target.geometry)):copy(d.target.geometry));}
    return {left,right,anchors:next};
}
function verifyRetargetStates(before,after,left,right) {
    const a=motionOf(before),b=envelope(after),reduced=after.motionState?.reduced===true;assert(b);
    if(a){assert.equal(b.targetSerial,a.targetSerial+1);assert.equal(b.sampleSerial,a.sampleSerial);}else{assert.equal(b.targetSerial,0);assert.equal(b.sampleSerial,0);}
    let checked=0;
    for(const [key,d]of right){const old=left.get(key);for(const [k,t]of Object.entries(d.targetChannels)){const previous=old?.channels[k];stateEqual(d.channels[k],previous&&!reduced?[previous[0],previous[1],t]:[t,0,t],key+':retarget '+k);checked++;}
        if(!d.geometryStates)continue;
        const oldDesc=old&&left.get(key),isPath=d.geometry==='path',oldPath=oldDesc?.geometry==='path';
        if(isPath){const changed=oldPath&&JSON.stringify(oldDesc.target.geometry)!==JSON.stringify(d.target.geometry);const expected=!oldPath||reduced?[1,0,1]:changed?[0,0,1]:oldDesc.geometryStates[0];stateEqual(d.geometryStates[0],expected,key+':retarget progress');checked++;}
        else {
            const shapeCompatible=oldDesc&&oldDesc.kind==='mark'&&oldDesc.geometry===d.geometry&&oldDesc.geometryStates.length===d.geometryStates.length&&(d.geometry.startsWith('annotation-')||oldDesc.target.geometry.kind===d.target.geometry.kind);
            d.geometryStates.forEach((s,i)=>{const p=shapeCompatible?oldDesc.geometryStates[i]:null;stateEqual(s,p&&!reduced?[p[0],p[1],d.numbers[i]]:[d.numbers[i],0,d.numbers[i]],key+':retarget numeric '+i);checked++;});
        }
    }
    return {retargetStates:checked};
}

export function createOracle(initialOwner,initialConfig=DEFAULT) {
    let config=configOf(initialConfig),last=copy(initialOwner),anchors=new Map(),serial;
    function initialize(owner){const m=envelope(owner);anchors=new Map();serial=m?.targetSerial;if(m){assert(m.settled,'initial attachment needs a settled public target; retain the oracle across subsequent samples');const entries=stateMap(m);for(const [key,d]of entries)if(d.kind==='mark'&&d.geometry==='path')anchors.set(key,copy(d.target.geometry));}}
    initialize(initialOwner);
    const api={
        verifyPaint(owner){const m=motionOf(owner);if(!m){anchors=new Map();serial=undefined;last=copy(owner);return{marks:0,labels:0,scalarStates:0,pathNumbers:0};}if(serial===undefined)initialize(owner);assert.equal(m.targetSerial,serial,'observe each target admission before sampling');const counts=paint(owner,anchors);last=copy(owner);return counts;},
        verifyStep(before,after,dt,nextConfig=config){config=configOf(nextConfig);assert.equal(motionOf(before)?.targetSerial,serial);const counts=verifyStepStates(before,after,dt,config);return {...counts,...api.verifyPaint(after)};},
        verifyRetarget(before,after){assert.equal(motionOf(before)?.targetSerial,serial);const updated=updateAnchors(before,after,anchors),counts=verifyRetargetStates(before,after,updated.left,updated.right);anchors=updated.anchors;serial=after.motion.targetSerial;return {...counts,...api.verifyPaint(after)};},
        verifyFinish(before,after){const a=envelope(before),b=envelope(after);assert(a&&b);same(a.target,b.target);same(a.targetScene,b.targetScene);assert.equal(b.targetSerial,a.targetSerial);assert.equal(b.sampleSerial,a.sampleSerial+(a.settled?0:1));for(const s of rawStates(b))stateEqual(s,[s[2],0,s[2]],'finish physical target');return api.verifyPaint(after);},
        observe(owner,op='paint',args={}){if(op==='step')return api.verifyStep(last,owner,args.dt, args.config??config);if(op==='retarget')return api.verifyRetarget(last,owner);if(op==='finish')return api.verifyFinish(last,owner);if(op==='auto'&&owner.motion&&owner.motion.targetSerial!==serial){assert.equal(owner.motion.targetSerial,serial+1,'unobserved intermediate target cannot be recovered from sampled paint');anchors=updateAnchors(last,owner,anchors).anchors;serial=owner.motion.targetSerial;}if(op==='config')config=configOf(args.config);return api.verifyPaint(owner);},
    };
    api.verifyPaint(initialOwner);return api;
}
// Stateless entry points work for a settled initial frame. Active paths need
// their retained instance so previous TARGET provenance is never guessed.
export const verifyStep=(before,after,dt,config,oracle)=> (oracle??createOracle(before,config)).verifyStep(before,after,dt,config);
export const verifyRetarget=(before,after,oracle)=> (oracle??createOracle(before)).verifyRetarget(before,after);
