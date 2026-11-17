// Independent plain-HTML percent-translation reference. No chart inputs allowed.
import assert from 'node:assert/strict';

export const TOOLTIP_GEOMETRY_TOLERANCE=1/30+.0001;
const fractions={Top:[-.5,-1],TopRight:[0,-1],Right:[0,-.5],BottomRight:[0,0],Bottom:[-.5,0],BottomLeft:[-1,0],Left:[-1,-.5],TopLeft:[-1,-1],Center:[-.5,-.5]};
const finite=(values,name)=>assert(values.every(Number.isFinite),name+' must be finite');
const near=(actual,expected)=>Math.abs(actual-expected)<=TOOLTIP_GEOMETRY_TOLERANCE;

// Preserve the documented public arithmetic order, including its rounding edge.
export function tooltipPlacementFromClient({pointerClient,parentRect,parentCssSize,requestedAnchor,gap=14}){
    finite([...pointerClient,...parentRect,...parentCssSize,gap],'tooltip placement');
    assert(parentRect[2]>0&&parentRect[3]>0&&parentCssSize.every(v=>v>0));
    assert(fractions[requestedAnchor]);
    const local=[(pointerClient[0]-parentRect[0])/parentRect[2]*parentCssSize[0],(pointerClient[1]-parentRect[1])/parentRect[3]*parentCssSize[1]];
    const anchor=requestedAnchor==='Left'||requestedAnchor==='Right'?(local[0]<parentRect[2]/2?'Right':'Left'):requestedAnchor;
    const shifts={Top:[0,-gap],TopRight:[gap,-gap],Right:[gap,0],BottomRight:[gap,gap],Bottom:[0,gap],BottomLeft:[-gap,gap],Left:[-gap,0],TopLeft:[-gap,-gap],Center:[0,0]};
    return{local,anchor,declaredLeftTop:local.map((value,i)=>Math.floor(value+shifts[anchor][i]+.5))};
}

// cssSize is the computed BORDER BOX in local CSS coordinates, never BCR size.
// Device rounding affects the percent reference only; physical box size stays raw.
export function expectedTooltipBox({model,dpr,parentRect,parentCssSize,cssSize,declaredLeftTop,anchor}){
    assert(model==='css'||model==='device-border','unknown tooltip reference model');
    assert(Number.isFinite(dpr)&&dpr>0);
    finite([...parentRect,...parentCssSize,...cssSize,...declaredLeftTop],'tooltip box');
    assert(parentRect[2]>0&&parentRect[3]>0&&parentCssSize.every(v=>v>0)&&cssSize.every(v=>v>=0));
    assert(fractions[anchor]);
    const scale=[parentRect[2]/parentCssSize[0],parentRect[3]/parentCssSize[1]];
    const referenceCssSize=model==='css'?[...cssSize]:cssSize.map(value=>Math.round(value*dpr)/dpr);
    const rect=[...declaredLeftTop.map((value,i)=>parentRect[i]+(value+fractions[anchor][i]*referenceCssSize[i])*scale[i]),cssSize[0]*scale[0],cssSize[1]*scale[1]];
    return{rect,referenceCssSize,scale};
}

export async function calibrateTooltipReference(page,{dpr}){
    const parentInputs=[{key:'plain',left:40,top:32,size:[420,300],translate:[0,0],scale:[1,1]},{key:'scaled',left:40,top:32,size:[420,300],translate:[130,90],scale:[1.2,.8]}];
    const cases=[
        ...[[84.140625,18.1875],[136.25,18.1875]].map((size,i)=>({key:'calibration-'+i,calibration:true,parent:'plain',size,declaredLeftTop:[210,84],anchor:'Top'})),
        ...parentInputs.flatMap(parent=>Object.keys(fractions).flatMap(anchor=>[[137.375,31.375],[41.5,23.625]].map((size,i)=>({key:`holdout-${parent.key}-${anchor}-${i}`,calibration:false,parent:parent.key,size,declaredLeftTop:[210,84],anchor})))),
    ];
    const observed=await page.evaluate(({parentInputs,cases,fractions})=>{
        const host=document.createElement('div');host.dataset.tooltipReferenceControl='v1';document.body.append(host);
        const rect=node=>{const r=node.getBoundingClientRect();return[r.x,r.y,r.width,r.height];};
        try{
            const parents=new Map(parentInputs.map(input=>{
                const node=document.createElement('div');node.dataset.referenceParent=input.key;
                node.style.cssText=`position:fixed;left:${input.left}px;top:${input.top}px;width:${input.size[0]}px;height:${input.size[1]}px;transform-origin:0 0;transform:translate(${input.translate[0]}px,${input.translate[1]}px) scale(${input.scale[0]},${input.scale[1]});padding:0;border:0;margin:0;box-sizing:border-box;pointer-events:none;`;
                host.append(node);return[input.key,node];
            }));
            const nodes=cases.map(input=>{
                const node=document.createElement('div');const f=fractions[input.anchor];node.dataset.referenceCase=input.key;
                node.style.cssText=`position:absolute;left:${input.declaredLeftTop[0]}px;top:${input.declaredLeftTop[1]}px;width:${input.size[0]}px;height:${input.size[1]}px;transform:translate(${f[0]*100}%,${f[1]*100}%);padding:0;border:0;margin:0;box-sizing:border-box;overflow:auto;background:rgb(200,30,40);pointer-events:none;`;
                parents.get(input.parent).append(node);return node;
            });
            return{dpr:devicePixelRatio,html:host.outerHTML,rows:nodes.map((node,i)=>{
                const style=getComputedStyle(node),parent=parents.get(cases[i].parent),parentStyle=getComputedStyle(parent);
                return{key:cases[i].key,rect:rect(node),cssSize:[parseFloat(style.width),parseFloat(style.height)],declaredLeftTop:[parseFloat(style.left),parseFloat(style.top)],computedTransform:style.transform,boxSizing:style.boxSizing,offsetSize:[node.offsetWidth,node.offsetHeight],parent:{rect:rect(parent),cssSize:[parseFloat(parentStyle.width),parseFloat(parentStyle.height)]},style:node.getAttribute('style')};
            })};
        }finally{host.remove();}
    },{parentInputs,cases,fractions});
    assert.equal(observed.dpr,dpr,'actual browser DPR equals context input');
    const certificate={protocol:'plain-tooltip-reference-v1',dpr,tolerance:TOOLTIP_GEOMETRY_TOLERANCE,parentInputs,cases,observed,model:null,candidates:[],holdouts:[],passed:false};
    try{
        for(const [i,row]of observed.rows.entries()){
            const input=cases[i],parent=parentInputs.find(p=>p.key===input.parent);
            assert.equal(row.key,input.key);assert.equal(row.boxSizing,'border-box');assert.deepEqual(row.declaredLeftTop,input.declaredLeftTop);
            row.cssSize.forEach((value,j)=>assert(near(value,input.size[j]),'constant authored CSS size'));
            row.rect.slice(2).forEach((value,j)=>assert(near(value,input.size[j]*parent.scale[j]),'unrounded BCR box size'));
            row.parent.rect.forEach((value,j)=>assert(near(value,j<2?[parent.left,parent.top][j]+parent.translate[j]:parent.size[j-2]*parent.scale[j-2]),'constant authored parent'));
            assert.deepEqual(row.parent.cssSize,parent.size);
        }
        for(const model of ['css','device-border']){
            const rows=observed.rows.filter((_,i)=>cases[i].calibration);
            const matches=rows.every(row=>expectedTooltipBox({model,dpr,parentRect:row.parent.rect,parentCssSize:row.parent.cssSize,cssSize:row.cssSize,declaredLeftTop:row.declaredLeftTop,anchor:'Top'}).rect.every((value,j)=>near(value,row.rect[j])));
            certificate.candidates.push({model,matches});
        }
        const matches=certificate.candidates.filter(candidate=>candidate.matches);
        assert.equal(matches.length,1,'two fixed independent constants must uniquely select one model');
        certificate.model=matches[0].model;
        // The model is now fixed. A failing holdout is a hard error, never a retry.
        for(const [i,row]of observed.rows.entries())if(!cases[i].calibration){
            const expected=expectedTooltipBox({model:certificate.model,dpr,parentRect:row.parent.rect,parentCssSize:row.parent.cssSize,cssSize:row.cssSize,declaredLeftTop:row.declaredLeftTop,anchor:cases[i].anchor});
            const passed=expected.rect.every((value,j)=>near(value,row.rect[j]));certificate.holdouts.push({key:row.key,expected,actual:row.rect,passed});
            assert(passed,'fixed tooltip reference model failed holdout '+row.key);
        }
        certificate.passed=true;return certificate;
    }catch(error){error.referenceCalibration=certificate;throw error;}
}
