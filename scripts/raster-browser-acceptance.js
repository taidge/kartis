// The Rust raster backend supplies PNG bytes; the browser only decodes and
// samples the actual Glory <img>. It never constructs an SVG or repaints paths.
(async (page,url='http://127.0.0.1:8815')=>{
  const errors=[];page.on('pageerror',error=>errors.push(error.message));
  const assert=(value,message)=>{if(!value)throw Error(message);};
  const snapshot=()=>page.evaluate(()=>JSON.parse(window.rasterFixture.snapshot()));
  const image=()=>page.locator('#app img.kartis-raster-image');
  const ready=async()=>{await image().evaluate(async img=>{await img.decode();if(!img.complete||!img.naturalWidth)throw Error('PNG image did not decode');});};
  const pixels=async(vertical=false)=>image().evaluate((img,vertical)=>{
    const canvas=document.createElement('canvas');canvas.width=img.naturalWidth;canvas.height=img.naturalHeight;
    const ctx=canvas.getContext('2d');ctx.drawImage(img,0,0);const sx=img.naturalWidth/parseFloat(img.style.width),sy=img.naturalHeight/parseFloat(img.style.height);
    const pixel=(x,y)=>[...ctx.getImageData(Math.floor(x*sx),Math.floor(y*sy),1,1).data];
    const ink=(x,y,w,h)=>{const p=ctx.getImageData(Math.floor(x*sx),Math.floor(y*sy),Math.floor(w*sx),Math.floor(h*sy)).data;let n=0;for(let i=0;i<p.length;i+=4)if(Math.max(p[i],p[i+1],p[i+2])<130&&p[i+3])n++;return n;};
    if(vertical)return {first:ink(176,35,48,42),second:ink(176,77,48,42),punctuation:ink(202,119,22,18),left:ink(176,119,26,42),lower:ink(176,139,48,22)};
    return{width:img.naturalWidth,height:img.naturalHeight,logicalWidth:parseFloat(img.style.width),logicalHeight:parseFloat(img.style.height),alpha:pixel(50,50),beta:pixel(250,50),alphaFont:ink(20,130,150,50),betaFont:ink(210,130,160,50),sharedFont:ink(100,182,210,28),source:img.getAttribute('src')};
  },vertical);
  await page.goto(url);await page.waitForFunction(()=>window.rasterReady===true);await ready();
  let sample=await pixels();
  assert(sample.source.startsWith('data:image/png;base64,iVBORw0KGgo'), 'Glory image is not encoded PNG');
  assert(sample.width===400&&sample.height===220,'Initial PNG dimensions incorrect');
  assert(sample.alpha.join(',')==='220,30,40,255'&&sample.beta.join(',')==='20,160,70,255','Decoded PNG geometry pixels incorrect');
  assert(sample.alphaFont>70&&sample.betaFont>70&&sample.sharedFont>70,'Actual font outlines missing from PNG pixels');
  assert(await page.locator('#app svg,#app canvas').count()===0,'Raster widget allocated a vector/canvas wrapper');
  assert(await page.locator('#app .kartis-datum').count()===2,'Glory data controls missing');
  await image().evaluate(img=>window.originalRasterImage=img);

  const firstLegend=page.locator('#app .kartis-legend-item').first();await firstLegend.click();await page.waitForFunction(()=>document.querySelectorAll('#app .kartis-datum').length===1);await ready();
  const hidden=await pixels();assert(hidden.alpha.join(',')==='255,255,255,255'&&hidden.beta.join(',')==='20,160,70,255','Legend did not redraw raster pixels');
  assert(hidden.alphaFont===0&&hidden.betaFont>70&&hidden.sharedFont>70,'Legend did not remove only owned raster labels');
  await firstLegend.click();await page.waitForFunction(()=>document.querySelectorAll('#app .kartis-datum').length===2);await ready();
  assert((await pixels()).source===sample.source,'Restoring visibility was not deterministic');

  await page.evaluate(()=>window.rasterFixture.update(42,440,2));await ready();let updated=await pixels();
  assert(updated.width===880&&updated.height===440&&updated.logicalWidth===440&&updated.logicalHeight===220,'Cage dimensions/DPR did not redraw actual PNG and logical image sizing');
  assert(updated.alpha.join(',')==='30,60,210,255','Cage geometry color update absent from PNG');
  assert(await image().evaluate(img=>img===window.originalRasterImage),'Reactive raster update replaced Glory image identity');
  const datum=page.locator('#app .kartis-datum').first();await datum.hover();await page.waitForFunction(()=>document.querySelector('#app .kartis-tooltip').textContent==='alpha: 42.00');
  await datum.click();await page.waitForFunction(()=>JSON.parse(window.rasterFixture.snapshot()).events.includes('select:alpha:42'));
  await datum.focus();await page.keyboard.press('ArrowDown');await page.waitForFunction(()=>document.activeElement?.classList.contains('kartis-datum')&&document.activeElement.textContent.includes('beta'));

  // Reject a PNG source publisher after a valid candidate raster. Image and
  // accepted controls must both remain at 42 until a complete retry succeeds.
  await page.evaluate(()=>{window.rasterFixture.fail_publication(true);window.rasterFixture.update(99,460,2);});
  assert(await image().getAttribute('src')===updated.source,'Failed PNG publication changed old frame');
  assert((await datum.textContent()).includes('42'),'Failed PNG publication changed controls');
  assert((await snapshot()).events.some(e=>e.includes('fixture PNG publication rejected')),'PNG publisher failure not reported');
  await page.evaluate(()=>{window.rasterFixture.fail_publication(false);window.rasterFixture.update(99,460,2);});
  // Same rejected source values do not notify a Cage. An independently changed
  // accepted input triggers the explicit retry.
  await page.evaluate(()=>window.rasterFixture.update(100,460,2));await ready();updated=await pixels();
  assert(updated.width===920&&(await datum.textContent()).includes('100'),'Complete retry did not publish image and controls');

  await page.evaluate(()=>window.rasterFixture.update(777,480,NaN));
  assert(await image().getAttribute('src')===updated.source&&(await datum.textContent()).includes('100'),'Invalid DPR partially committed image or controls');
  assert((await snapshot()).events.some(e=>e.includes('raster DPR')),'Invalid DPR did not report rejection');
  await page.evaluate(()=>window.rasterFixture.restore());await ready();

  await page.evaluate(()=>{window.staleRasterDatum=document.querySelector('#app .kartis-datum');window.staleRasterLegend=document.querySelector('#app .kartis-legend-item');window.rasterFixture.clear_widget();});
  const retired=await snapshot();assert(retired.nodes===0&&retired.handlers===0&&retired.drops===1,'Raster source captures or controls survived owner cleanup');
  await page.evaluate(()=>{window.staleRasterDatum.click();window.staleRasterLegend.click();window.rasterFixture.update(88,420,1);});
  const after=await snapshot();assert(after.publications===retired.publications&&JSON.stringify(after.events)===JSON.stringify(retired.events),'Retired callbacks or raster effects ran again');
  await page.evaluate(()=>window.rasterFixture.mount_fixture());await ready();assert((await pixels()).alpha.join(',')==='220,30,40,255','Raster widget failed to remount');

  await page.evaluate(()=>window.rasterFixture.mount_vertical_fixture());await ready();
  const vertical=await pixels(true);
  assert(vertical.first>120&&vertical.second>200,'Decoded PNG is missing upright CJK glyphs in two vertical cells');
  assert(vertical.punctuation>30&&vertical.left===0&&vertical.lower===0,'OpenType vertical punctuation ink is missing from the upper-right of the last cell');
  await page.evaluate(()=>window.rasterFixture.mount_mixed_vertical_fixture());await ready();
  const mixedMetrics=await page.evaluate(()=>JSON.parse(window.rasterFixture.mixed_vertical_metrics()));
  const near=(a,b)=>Math.abs(a-b)<1e-8;
  assert(mixedMetrics.columns.length===2&&near(mixedMetrics.columns[0][1],130.6)&&
    near(mixedMetrics.columns[1][0],-60)&&near(mixedMetrics.columns[1][1],144.32)&&near(mixedMetrics.advance,144.32),
    'Mixed vertical measurement disagrees with independent HarfBuzz column advances');
  assert(mixedMetrics.ink.every((v,i)=>near(v,[120.96,34.32,97.92,124.48][i])),
    'Mixed vertical measurement disagrees with independent oriented glyph ink bounds');
  const mixedInk=await image().evaluate(img=>{
    const canvas=document.createElement('canvas');canvas.width=img.naturalWidth;canvas.height=img.naturalHeight;
    const context=canvas.getContext('2d');context.drawImage(img,0,0);
    const data=context.getImageData(0,0,canvas.width,canvas.height).data;
    let x0=canvas.width,y0=canvas.height,x1=-1,y1=-1,count=0;
    for(let y=0;y<canvas.height;y++)for(let x=0;x<canvas.width;x++){
      const i=(y*canvas.width+x)*4;
      if(data[i+3]&&Math.max(data[i],data[i+1],data[i+2])<130){
        x0=Math.min(x0,x);y0=Math.min(y0,y);x1=Math.max(x1,x);y1=Math.max(y1,y);count++;
      }
    }
    return{bounds:[x0,y0,x1+1-x0,y1+1-y0],count};
  });
  assert(mixedInk.count>800&&mixedInk.bounds.every((v,i)=>Math.abs(v-[121,34,98,125][i])<=1),
    `Decoded mixed multi-column PNG ink disagrees with glyph bounds ${JSON.stringify(mixedInk)}`);
  await page.evaluate(()=>window.rasterFixture.mount_typed_surface_fixture());
  const typedImage=page.locator('#app img.kartis-raster-surface-image');
  const typedReady=()=>typedImage.evaluate(img=>img.decode());
  const typedSnapshot=()=>page.evaluate(()=>JSON.parse(window.rasterFixture.typed_surface_snapshot()));
  const typedPixel=(x,y)=>typedImage.evaluate((img,[x,y])=>{
    const canvas=document.createElement('canvas');canvas.width=img.naturalWidth;canvas.height=img.naturalHeight;
    const context=canvas.getContext('2d');context.drawImage(img,0,0);
    return [...context.getImageData(x,y,1,1).data];
  },[x,y]);
  await typedReady();
  assert((await typedPixel(65,55)).join()==='210,40,50,255','Typed raster layout did not paint the decoded PNG');
  assert(await page.locator('#app .kartis-datum').count()===2,'Unbound decoration became a typed data control');
  const missing=page.getByRole('button',{name:'Alpha: missing',exact:true});
  assert(await missing.count()===1,'Typed raster control confused None with the scalar zero');
  await missing.focus();await page.keyboard.press('Enter');
  await page.waitForFunction(()=>JSON.parse(window.rasterFixture.typed_surface_snapshot()).selected==='a');
  let typed=await typedSnapshot();
  assert(typed.selected==='a'&&typed.interaction.rows[0].value===null&&typed.interaction.rows[0].label==='source Alpha',
    `Raster keyboard activation lost the source nullable row ${JSON.stringify(typed)}`);
  await typedImage.evaluate(img=>{
    window.typedRasterImage=img;
    window.typedRasterControl=document.querySelector('#app .kartis-datum');
    img.addEventListener('pointermove',event=>{
      const rect=img.getBoundingClientRect();
      window.rasterFixture.typed_surface_pointer(event.clientX,event.clientY,[rect.x,rect.y,rect.width,rect.height]);
    });
  });
  await typedImage.scrollIntoViewIfNeeded();
  const typedBox=await typedImage.boundingBox();
  await page.mouse.move(typedBox.x+65/300*typedBox.width,typedBox.y+55/150*typedBox.height);
  await page.waitForFunction(()=>document.querySelector('#app .kartis-tooltip')?.textContent==='source Alpha: missing');
  typed=await typedSnapshot();
  assert(typed.hovered.includes('a')&&typed.interaction.anchor.join()==='65,55','Explicit host pointer coordinates lost the typed anchor');
  const acceptedSource=await typedImage.getAttribute('src');
  const acceptedRevision=typed.revision;
  assert(await page.evaluate(()=>window.rasterFixture.typed_surface_reject_binding()),'Invalid typed raster binding was accepted');
  assert((await typedSnapshot()).revision===acceptedRevision,'Rejected binding changed the typed raster revision');
  assert(!await page.evaluate(()=>window.rasterFixture.typed_surface_update(1,true)),'Rejected image publication was accepted');
  assert(await typedImage.getAttribute('src')===acceptedSource&&await missing.count()===1&&
    (await typedSnapshot()).interaction.rows[0].value===null,'Failed publication partially committed typed controls, image or binding');
  assert(await page.evaluate(()=>window.rasterFixture.typed_surface_update(1,false)),'Typed raster retry failed');
  await typedReady();
  assert((await typedPixel(90,55)).join()==='30,90,190,255','Typed raster update did not paint the new PNG');
  assert(await typedImage.evaluate(img=>img===window.typedRasterImage),'Typed layout update replaced the image identity');
  const zero=page.getByRole('button',{name:'Alpha: 0',exact:true});
  assert(await zero.count()===1&&await zero.evaluate(button=>button===window.typedRasterControl),'Typed control identity or actual zero label was lost');
  await zero.focus();await page.keyboard.press('Enter');
  typed=await typedSnapshot();
  assert(typed.interaction.rows[0].value===0&&typed.interaction.anchor.join()==='90,55','Accepted update retained stale business metadata');
  await page.evaluate(()=>window.rasterFixture.stop_typed_surface_fixture());
  assert(await page.locator('#app *').count()===0,'Typed raster surface leaked owner nodes');
  assert(errors.length===0,errors.join('\n'));
  return{passed:true,actualGloryImage:true,pngDecode:true,geometryPixels:true,fontOutlinePixels:{alpha:sample.alphaFont,beta:sample.betaFont,shared:sample.sharedFont},verticalOutlinePixels:vertical,mixedVertical:{metrics:mixedMetrics,decodedInk:mixedInk,harfbuzzOracle:true},typedSurface:{actualPng:true,nullableSourceRows:true,explicitHostPointer:true,keyboard:true,retainedImageAndControls:true,publicationAtomic:true,bindingRejected:true,cleanup:true},legendPixelAndLabelRedraw:true,cageDimensionsAndDpr:true,retainedImageIdentity:true,hoverFocusKeyboardSelection:true,publicationFailureAtomic:true,invalidDprAtomic:true,ownerCleanup:true,staleCallbacks:true,remount:true,nativePointerCapability:'not claimed',pageErrors:errors};
})
