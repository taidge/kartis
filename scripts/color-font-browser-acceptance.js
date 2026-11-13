(async function verifyColorFonts(page,url='http://127.0.0.1:8815') {
  const errors=[];const onError=e=>errors.push(e.message);page.on('pageerror',onError);
  const assert=(ok,message)=>{if(!ok)throw Error(message);};
  const call=(name,...args)=>page.evaluate(([name,args])=>window.rasterFixture[name](...args),[name,args]);
  const ready=()=>page.locator('.color-raster img').evaluate(img=>img.decode());
  const samples=(palette,alpha)=>page.evaluate(async({palette,alpha})=>{
    const points=[[50,50],[70,80],[100,80],[125,80],[50,125]];
    const sample=ctx=>points.map(([x,y])=>[...ctx.getImageData(x,y,1,1).data]);
    const canvas=()=>{const c=document.createElement('canvas');c.width=180;c.height=180;const ctx=c.getContext('2d');ctx.fillStyle='white';ctx.fillRect(0,0,180,180);return ctx;};
    const raster=canvas();const img=document.querySelector('.color-raster img');await img.decode();raster.drawImage(img,0,0);
    const svg=document.createElementNS('http://www.w3.org/2000/svg','svg');svg.setAttribute('width','180');svg.setAttribute('height','180');
    for(const child of document.querySelectorAll('.color-semantic .kartis-painted-label svg'))svg.append(child.cloneNode(true));
    const graphic=new Image();graphic.src='data:image/svg+xml;charset=utf-8,'+encodeURIComponent(new XMLSerializer().serializeToString(svg));await graphic.decode();
    const semantic=canvas();semantic.drawImage(graphic,0,0);
    const bakedImage=new Image();bakedImage.src='data:image/svg+xml;charset=utf-8,'+encodeURIComponent(window.rasterFixture.color_fixture_svg(palette,alpha));await bakedImage.decode();
    const standaloneSvg=canvas();standaloneSvg.drawImage(bakedImage,0,0);
    const standaloneCanvas=document.createElement('canvas');window.rasterFixture.color_fixture_canvas(palette,alpha,standaloneCanvas);
    const native=canvas();native.font='100px KartisColorFixture';native.textBaseline='alphabetic';native.fillStyle='#00ff00';native.fillText('B',40,140);
    return{raster:sample(raster),semantic:sample(semantic),standaloneSvg:sample(standaloneSvg),canvas:sample(standaloneCanvas.getContext('2d')),native:sample(native)};
  },{palette,alpha});
  const close=(actual,expected,context)=>assert(actual.every((p,i)=>p.every((v,j)=>Math.abs(v-expected[i][j])<=1)),`${context}: ${JSON.stringify({actual,expected})}`);
  await page.goto(url);await page.waitForFunction(()=>window.rasterReady===true);
  await page.evaluate(async()=>{const bytes=window.rasterFixture.color_fixture_font();const font=new FontFace('KartisColorFixture',bytes);document.fonts.add(await font.load());});
  const cases=[];
  try {
    for(const palette of [0,1]) {
      await call('mount_color_fixture',palette);await ready();
      assert(await page.locator('.color-semantic .kartis-painted-glyph').count()===4,'COLR layers or transparent layer lost');
      assert(await page.locator('.color-semantic .kartis-outline-text').textContent()==='B','Original safe label text lost');
      await page.evaluate(()=>{window.colorLabel=document.querySelector('.kartis-painted-label');window.colorGlyphs=[...document.querySelectorAll('.kartis-painted-glyph')];window.colorImage=document.querySelector('.color-raster img');});
      const expected=palette===0?[[255,0,0,255],[255,255,255,255],[127,127,255,255],[127,0,128,255],[0,255,0,255]]:
        [[0,180,0,255],[255,255,255,255],[255,255,127,255],[128,218,0,255],[0,255,0,255]];
      const initial=await samples(palette,255);for(const backend of ['raster','semantic','standaloneSvg','canvas'])close(initial[backend],expected,`palette${palette}/${backend}`);
      if(palette===0)close(initial.native,expected,'native FontFace/Canvas independent COLR reference');
      await call('color_fixture_update','B',0);await ready();
      const transparent=await samples(palette,0);const expectedTransparent=expected.map(p=>p.slice());expectedTransparent[4]=expected[0].slice();
      for(const backend of ['raster','semantic','standaloneSvg','canvas'])close(transparent[backend],expectedTransparent,`transparent foreground/${backend}`);
      assert(await page.evaluate(()=>document.querySelector('.kartis-painted-label')===window.colorLabel&&
        [...document.querySelectorAll('.kartis-painted-glyph')].every((node,i)=>node===window.colorGlyphs[i])&&document.querySelector('.color-raster img')===window.colorImage),'Color update replaced glyph layers or PNG widget');
      const acceptedSource=await page.locator('.color-raster img').getAttribute('src');
      const acceptedMarkup=await page.locator('.color-semantic').innerHTML();
      await call('color_fixture_update','Missing 🐱',255);
      assert(await page.locator('.color-raster img').getAttribute('src')===acceptedSource&&await page.locator('.color-semantic').innerHTML()===acceptedMarkup,'Rejected color shaping partially published a frame');
      assert(JSON.parse(await call('color_fixture_snapshot')).errors.length===2,'Both consumers must report rejected missing glyph');
      await call('color_fixture_update','B',128);await ready();
      const retry=await samples(palette,128);const expectedRetry=expected.map(p=>p.slice());expectedRetry[4]=palette===0?[127,128,0,255]:[0,218,0,255];
      for(const backend of ['raster','semantic','standaloneSvg','canvas'])close(retry[backend],expectedRetry,`retry/${backend}`);
      const source=await page.locator('.color-raster img').getAttribute('src');
      await page.getByRole('button',{name:'Toggle color glyph',exact:true}).click();
      await page.waitForFunction(()=>getComputedStyle(document.querySelector('.kartis-painted-label')).display==='none');
      await page.waitForFunction(source=>document.querySelector('.color-raster img').getAttribute('src')!==source,source);await ready();
      const blank=await page.locator('.color-raster img').evaluate(img=>{const c=document.createElement('canvas');c.width=180;c.height=180;const x=c.getContext('2d');x.drawImage(img,0,0);return [...x.getImageData(50,50,1,1).data];});
      assert(blank.join()==='255,255,255,255','Owned palette glyph survived hiding');
      await page.getByRole('button',{name:'Toggle color glyph',exact:true}).focus();await page.keyboard.press('Enter');
      await page.waitForFunction(()=>getComputedStyle(document.querySelector('.kartis-painted-label')).display!=='none');await ready();
      close((await samples(palette,128)).semantic,expectedRetry,'keyboard restore');
      await call('stop_color_fixture');assert(await page.locator('#app *').count()===0,'Color font scopes leaked');
      cases.push({palette,initial,transparent,retry,holes:true,retained:true,rejectedAtomic:true,hideAndKeyboardRestore:true,cleanup:true});
    }
    assert(errors.length===0,errors.join('\n'));return{passed:true,nativeFontFacePaletteZero:true,cases,pageErrors:errors};
  } finally {await call('stop_color_fixture');page.off('pageerror',onError);}
})
