(async (page, url = 'http://127.0.0.1:8812') => {
  const errors=[];
  page.on('pageerror', e=>errors.push(e.message));
  await page.setViewportSize({width:1280,height:900});
  await page.goto(url);
  await page.waitForFunction(()=>document.documentElement.dataset.canvasReady==='true');
  const canvas=page.locator('#chart');
  const pixels=await canvas.evaluate(canvas=>{
    const p=canvas.getContext('2d').getImageData(0,0,canvas.width,canvas.height).data;
    for(let y=40;y<canvas.height;y++)for(let x=10;x<canvas.width;x++){
      const i=(y*canvas.width+x)*4;
      if(p[i]===78&&p[i+1]===121&&p[i+2]===167){return {x:(x+2)/canvas.width,y:(y+2)/canvas.height,width:canvas.width,height:canvas.height};}
    }
    return null;
  });
  if(!pixels)throw Error('Rust/WASM Canvas has no painted Desktop bar');
  const box=await canvas.boundingBox();
  await page.mouse.move(box.x+pixels.x*box.width,box.y+pixels.y*box.height);
  if(!(await canvas.getAttribute('data-hovered-mark')).startsWith('bar-'))throw Error('Rust pointer hit testing failed');
  await page.mouse.click(box.x+pixels.x*box.width,box.y+pixels.y*box.height);
  if(!(await canvas.getAttribute('data-selected-mark')).startsWith('bar-'))throw Error('Rust click selection failed');
  await canvas.focus();await page.keyboard.press('Escape');await page.keyboard.press('ArrowRight');
  const selected=await canvas.getAttribute('data-selected-mark');
  if(!selected.startsWith('bar-'))throw Error('Rust keyboard traversal failed');
  await page.locator('#legend-0').click();
  if(await canvas.getAttribute('data-hidden-series')!=='Desktop')throw Error('Rust legend filtering failed');
  const before=Number(await canvas.getAttribute('data-render-revision'));
  await page.setViewportSize({width:390,height:844});
  await page.waitForFunction(revision=>Number(document.querySelector('#chart').dataset.renderRevision)>revision,before);
  const mobile=await canvas.evaluate(el=>({width:Number(el.dataset.sceneWidth),height:Number(el.dataset.sceneHeight),overflow:document.documentElement.scrollWidth>innerWidth}));
  if(mobile.width>390||mobile.overflow)throw Error('Rust responsive relayout failed');
  await page.evaluate(async ()=>{const app=await import('/pkg/kartis_canvas_demo.js');app.stop();});
  if(await page.locator('html').getAttribute('data-canvas-ready')!=='false')throw Error('Rust listener cleanup hook failed');
  const stopped=await canvas.getAttribute('data-render-revision');
  await page.locator('#legend-1').click();
  if(await canvas.getAttribute('data-render-revision')!==stopped)throw Error('Rust listeners survived cleanup');
  if(errors.length)throw Error(errors.join('\n'));
  return {passed:true,backingStore:{width:pixels.width,height:pixels.height},selected,mobile,cleanup:true,pageErrors:errors};
})
