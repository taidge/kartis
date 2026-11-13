// Actual DOM text measurements feed the shared Rust placement algorithm.
(async page=>{
  const errors=[];const receive=error=>errors.push(error.message);page.on('pageerror',receive);
  const assert=(condition,message)=>{if(!condition)throw Error(message);};
  const snapshot=id=>page.evaluate(id=>JSON.parse(window.hostFixture.snapshot(id)),id);
  const wait=(id,predicate)=>page.waitForFunction(({id,predicate})=>new Function('s',`return (${predicate})(s)`)(JSON.parse(window.hostFixture.snapshot(id))),{id,predicate:predicate.toString()});
  const hover=async(id,name,nx,ny)=>{
    const surface=page.locator(`#${id}-root .kartis-surface`);await surface.scrollIntoViewIfNeeded();const box=await surface.boundingBox();
    await page.mouse.move(2,2);await page.mouse.move(box.x+box.width*nx,box.y+box.height*ny);await wait(id,s=>s.pending===0&&s.hovered.length>0);
    await page.waitForFunction(({id,name})=>JSON.parse(window.hostFixture.snapshot(id)).hovered.includes(name),{id,name});
    return page.locator(`#${id}-root .kartis-surface-tooltip`).evaluate(el=>{
      const s=el.parentElement.querySelector('.kartis-surface').getBoundingClientRect(),r=el.getBoundingClientRect();
      return{tooltip:{x:r.x-s.x,y:r.y-s.y,width:r.width,height:r.height},surface:{width:s.width,height:s.height},text:el.textContent,htmlChildren:el.childElementCount,scrollHeight:el.scrollHeight,clientHeight:el.clientHeight,style:getComputedStyle(el).overflow};
    });
  };
  const bounded=(report,label)=>{const r=report.tooltip,v=report.surface;assert(r.width>0&&r.height>0&&r.x>=3.5&&r.y>=3.5&&r.x+r.width<=v.width-3.5&&r.y+r.height<=v.height-3.5,`${label}: tooltip overflowed measured viewport ${JSON.stringify(report)}`);};
  const results=[];
  try{
    for(const id of['svg','canvas']){
      await page.evaluate(id=>window.hostFixture.remount_tooltip_layout(id,0),id);await wait(id,s=>s.active&&s.pending===0);
      const bottom=await hover(id,'corner-br',.95,.95);bounded(bottom,`${id} bottom-right`);assert(bottom.text.includes('<b>Automatic safe tooltip</b>')&&bottom.text.includes('12.50')&&bottom.htmlChildren===0,`${id}: safe multi-line text was altered or parsed`);
      assert(bottom.tooltip.y+bottom.tooltip.height<bottom.surface.height*.95,`${id}: automatic top anchor did not avoid bottom pointer`);
      const top=await hover(id,'corner-tl',.05,.05);bounded(top,`${id} top-left`);
      await page.locator(`#${id}-root`).evaluate(el=>el.style.width='75%');await wait(id,s=>s.pending===0);await page.waitForFunction(id=>{const s=JSON.parse(window.hostFixture.snapshot(id)),r=document.querySelector(`#${id}-root .kartis-surface`).getBoundingClientRect();return Math.abs(s.width-r.width)<1;},id);
      bounded(await hover(id,'corner-br',.95,.95),`${id} resized`);
      await page.locator(`#${id}-root`).evaluate(el=>el.style.width='');

      await page.evaluate(id=>window.hostFixture.remount_tooltip_layout(id,1),id);await wait(id,s=>s.active&&s.pending===0);
      const avoided=await hover(id,'center',.45,.40);bounded(avoided,`${id} forbidden zone`);
      const overlap=Math.max(0,Math.min(avoided.tooltip.y+avoided.tooltip.height,260)-Math.max(avoided.tooltip.y,130))*avoided.tooltip.width;
      assert(overlap<.1,`${id}: actual tooltip overlaps forbidden lower half ${JSON.stringify(avoided)}`);

      await page.evaluate(id=>window.hostFixture.remount_tooltip_layout(id,2),id);await wait(id,s=>s.active&&s.pending===0);
      const oversize=await hover(id,'corner-br',.95,.95);bounded(oversize,`${id} oversized`);
      assert(oversize.scrollHeight>oversize.clientHeight&&oversize.style==='auto',`${id}: oversized tooltip was neither constrained nor scrollable`);
      assert(oversize.text.includes('Line39 <em>safe</em>')&&oversize.htmlChildren===0,`${id}: oversized safe text lost content or became HTML`);
      await page.evaluate(id=>{window.hostFixture.remount(id);window.hostFixture.set_phase(id,1,false);},id);await wait(id,s=>s.active&&s.phase===1&&s.pending===0&&!s.animating);
      results.push({renderer:id,actualDomMeasurement:true,eightWayDefault:true,cornerClamp:true,resize:true,forbiddenAvoidance:true,oversizeScroll:true,safeMultilineText:true});
    }
    assert(errors.length===0,errors.join('\n'));return{passed:true,renderers:results,pageErrors:errors};
  }finally{page.off('pageerror',receive);}
})
