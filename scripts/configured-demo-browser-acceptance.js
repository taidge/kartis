(async page=>{
  const assert=(ok,message)=>{if(!ok)throw Error(message);};
  const errors=[],listener=e=>errors.push(e.message);page.on('pageerror',listener);
  const choose=name=>page.getByRole('button',{name,exact:true}).click();
  try{
    await choose('Tree styles');
    await page.waitForFunction(()=>['svg','canvas'].every(id=>JSON.parse(window.hostFixture.tree_snapshot(id)).active));
    assert(await page.locator('#tree-svg .kartis-surface,#tree-canvas .kartis-surface').count()===2,'Tree navigation did not mount both actual controllers');
    assert(await page.locator('.panels').evaluate(el=>getComputedStyle(el).display==='none'),'Ordinary panels obscured configured examples');
    assert(await page.evaluate(()=>['svg','canvas'].every(id=>!JSON.parse(window.hostFixture.snapshot(id)).active)),'Configured navigation retained hidden ordinary controllers');
    await choose('Parallel axes');
    await page.waitForFunction(()=>{const s=JSON.parse(window.hostFixture.parallel_snapshot('svg'));return s.active&&!s.vertical&&!s.pending;});
    assert(await page.locator('#tree-svg,#tree-canvas').count()===0,'Switching examples retained Tree controllers');
    const horizontal=await page.locator('#parallel-svg svg [data-id]').first().getAttribute('d');
    await choose('Vertical parallel axes');
    await page.waitForFunction(()=>{const s=JSON.parse(window.hostFixture.parallel_snapshot('svg'));return s.active&&s.vertical&&!s.pending;});
    assert(await page.locator('#parallel-svg svg [data-id]').first().getAttribute('d')!==horizontal,'Layout navigation left the horizontal geometry in place');
    await choose('Drag network nodes');
    await page.waitForFunction(()=>['svg','canvas'].every(id=>JSON.parse(window.hostFixture.network_drag_snapshot(id)).active));
    assert(await page.locator('#parallel-svg,#parallel-canvas').count()===0,'Switching to Network retained parallel controllers');
    assert(await page.locator('#network-svg .kartis-surface,#network-canvas .kartis-surface').count()===2,'Network navigation did not mount both renderers');
    const families=[['Tree styles','tree','tree'],['Parallel axes','parallel','parallel'],['Drag network nodes','network_drag','network'],['Pie styles','pie','pie'],['Radar styles','radar','radar'],['Sankey styles','sankey_styles','sankey-style'],['Line styles','styled_line','styled-line'],['Color scales','color','colors'],['Bar styles','bar_style','bar-style'],['HeatMap cells','heatmap','heatmap'],['Stacked radial bars','stacked_radial','stacked-radial']];
    const styleCases=[['Pie styles','pie','pie',null,3],['Radar styles','radar','radar',null,2],['Sankey styles','sankey_styles','sankey-style',0,2],['Vertical Sankey','sankey_styles','sankey-style',1,2],['Circular Sankey','sankey_styles','sankey-style',2,2],['Vertical circular Sankey','sankey_styles','sankey-style',3,2],['Line styles','styled_line','styled-line',null,3],['Color scales','color','colors',null,3],['Bar styles','bar_style','bar-style',0,2],['Horizontal stacked bars','bar_style','bar-style',1,2],['Percent bar styles','bar_style','bar-style',2,2],['HeatMap cells','heatmap','heatmap',0,2],['HeatMap circles','heatmap','heatmap',1,2],['HeatMap size variation','heatmap','heatmap',2,2],['HeatMap circle sizes','heatmap','heatmap',3,2],['Stacked radial bars','stacked_radial','stacked-radial',null,3]];
    families.push(['Compact calendar','calendar_layout','calendar-layout'],['Vertical box plots','statistics','statistics'],['Focus treemap','hierarchy','hierarchy'],['Area bump ranks','area_bump','area-bump']);
    families.push(['Independent Marimekko','marimekko','marimekko'],['Geographic streams','geo_stream','geo-stream'],['Delaunay mesh','delaunay','delaunay']);
    families.push(['Nivo padded donut','pie_layout','pie-layout'],['Line point crosshair','line_crosshair','line-crosshair'],['Radar polar slices','radar_slices','radar-slices'],['Tree relationships','relations','relations']);
    families.push(['Configured linear scales','scale_profiles','scale-profiles'],['Star glyph points','glyphs','glyphs'],['Geographic glyphs','geo_glyphs','geo-glyphs']);
    families.push(['Typed scatter nodes','scatter','scatter']);
    styleCases.push(['Compact calendar','calendar_layout','calendar-layout',0,2],['Vertical time range','calendar_layout','calendar-layout',1,2],['Styled yearly calendar','calendar_layout','calendar-layout',2,2],['Vertical yearly calendar','calendar_layout','calendar-layout',3,2],['Vertical box plots','statistics','statistics',0,2],['Horizontal box plots','statistics','statistics',1,2],['Horizontal bullet rows','statistics','statistics',2,2],['Vertical bullet rows','statistics','statistics',3,2],['Focus treemap','hierarchy','hierarchy',null,3],['Focus circle packing','hierarchy','hierarchy',null,3],['Focus icicle right','hierarchy','hierarchy',null,3],['Focus icicle left','hierarchy','hierarchy',null,3],['Area bump ranks','area_bump','area-bump',0,2],['Smooth area bump','area_bump','area-bump',1,2],['Gapped area bump','area_bump','area-bump',2,2]);
    const styles=[];
    styleCases.push(['Independent Marimekko','marimekko','marimekko',0,2],['Expanded Marimekko','marimekko','marimekko',1,2],['Signed Marimekko','marimekko','marimekko',2,2],['Stream-offset Marimekko','marimekko','marimekko',3,2],['Geographic streams','geo_stream','geo-stream',0,2],['Transverse Mercator','geo_stream','geo-stream',1,2],['Clipped globe routes','geo_stream','geo-stream',2,2],['Delaunay mesh','delaunay','delaunay',0,2],['Delaunay duplicates','delaunay','delaunay',1,2],['Collinear Delaunay','delaunay','delaunay',2,2]);
    let previousHierarchyShape;
    styleCases.push(['Nivo padded donut','pie_layout','pie-layout',0,2],['Partial pie fit','pie_layout','pie-layout',1,2],['Pie pad-radius profiles','pie_layout','pie-layout',2,2],['Measured pie link labels','pie_layout','pie-layout',3,2],['Line point crosshair','line_crosshair','line-crosshair',0,2],['Line X slices','line_crosshair','line-crosshair',1,2],['Line Y slices','line_crosshair','line-crosshair',2,2],['Radar polar slices','radar_slices','radar-slices',0,2],['Radar point slices','radar_slices','radar-slices',1,2],['Counterclockwise radar slices','radar_slices','radar-slices',2,2],['Tree relationships','relations','relations',0,2],['Network neighbors','relations','relations',1,2],['Chord relationships','relations','relations',2,2],['Sankey relationships','relations','relations',3,2],['Circular Sankey relationships','relations','relations',4,2]);
    styleCases.push(['Configured linear scales','scale_profiles','scale-profiles',0,2],['Configured log and symlog scales','scale_profiles','scale-profiles',1,2],['Configured UTC scales','scale_profiles','scale-profiles',2,2],['Ordinal point line','scale_profiles','scale-profiles',3,2],['Star glyph points','glyphs','glyphs',0,2],['Wye glyph points','glyphs','glyphs',1,2],['Hollow glyph points','glyphs','glyphs',2,2],['Radar glyph points','glyphs','glyphs',3,2],['Geographic glyphs','geo_glyphs','geo-glyphs',0,2],['Clipped globe glyphs','geo_glyphs','geo-glyphs',1,2]);
    styleCases.push(['Typed scatter nodes','scatter','scatter',0,2],['Dynamic scatter sizes','scatter','scatter',1,2],['Scatter area glyphs','scatter','scatter',2,2],['Hollow scatter glyphs','scatter','scatter',3,2],['UTC scatter axes','scatter','scatter',4,2],['Ordinal scatter axes','scatter','scatter',5,2]);
    for(const [name,prefix,root,kind,phases] of styleCases){
      await page.evaluate(families=>{window.configuredStaleControl=document.querySelector(families.map(([,prefix,root])=>`#${root}-svg .kartis-datum`).join(','));},families);
      await choose(name);
      await page.waitForFunction(({prefix,kind})=>['svg','canvas'].every(id=>{const state=JSON.parse(window.hostFixture[`${prefix}_snapshot`](id));return state.active&&!state.pending&&state.phase===0&&(kind===null||state.case===kind);}),{prefix,kind});
      assert(await page.locator(`#${root}-svg .kartis-surface,#${root}-canvas .kartis-surface`).count()===2,`${name}: navigation did not mount both real renderers`);
      const retired=await page.evaluate(({families,prefix})=>families.filter(f=>f[1]!==prefix).every(([,other])=>['svg','canvas'].every(id=>!JSON.parse(window.hostFixture[`${other}_snapshot`](id)).active)),{families,prefix});
      assert(retired,`${name}: switching kept another configured family alive`);
      assert(await page.getByRole('button',{name:'Change styles',exact:true}).isEnabled(),`${name}: style phases are not available from the UI`);
      assert(await page.getByRole('button',{name:'Animate',exact:true}).isDisabled()&&await page.getByRole('button',{name:'Reset view',exact:true}).isDisabled(),`${name}: ordinary controls still target hidden charts`);
      assert(await page.locator('[data-mode][aria-pressed="true"]').count()===1&&await page.getByRole('button',{name,exact:true}).getAttribute('aria-pressed')==='true',`${name}: navigation state is inconsistent`);
      assert((await page.locator('#heatmap-target').isEnabled())===(prefix==='heatmap'),`${name}: HeatMap peer selector targets the wrong family`);
      assert((await page.locator('#hierarchy-focus').isEnabled())===(prefix==='hierarchy'),`${name}: hierarchy controls target the wrong family`);
      if(prefix==='heatmap'){
        for(const [value,target]of [['0','Cell'],['1','Row'],['2','Column'],['3','RowColumn'],['4','All']]){await page.locator('#heatmap-target').selectOption(value);await page.waitForFunction(target=>['svg','canvas'].every(id=>JSON.parse(window.hostFixture.heatmap_snapshot(id)).hoverTarget===target),target);}
      }
      if(prefix==='hierarchy'){
        const shape=await page.evaluate(()=>[...document.querySelectorAll('#hierarchy-svg .kartis-surface>svg [data-id]')].map(p=>p.dataset.id+' '+p.getAttribute('d')).join('|'));
        if(previousHierarchyShape!==undefined)assert(shape!==previousHierarchyShape,`${name}: hierarchy view menu retained the previous geometry`);previousHierarchyShape=shape;
        await choose('Focus branch');await page.waitForFunction(()=>['svg','canvas'].every(id=>JSON.parse(window.hostFixture.hierarchy_snapshot(id)).focus==='group-a'));
        assert((await page.locator('#hierarchy-path').textContent()).includes('group-a'),'Visible hierarchy path ignored data-space focus');
        await choose('Go back');await page.waitForFunction(()=>['svg','canvas'].every(id=>JSON.parse(window.hostFixture.hierarchy_snapshot(id)).focus==='root'));
        await choose('Focus branch');await choose('Hierarchy root');await page.waitForFunction(()=>['svg','canvas'].every(id=>JSON.parse(window.hostFixture.hierarchy_snapshot(id)).focus==='root'));
        // This uses only the page's real event-to-flush bridge, without calling
        // the fixture's hierarchy_flush helper from the acceptance script.
        await page.locator('#hierarchy-svg .kartis-datum').filter({hasText:'<Leaf A>'}).first().focus();await page.keyboard.press('Enter');
        await page.waitForFunction(()=>JSON.parse(window.hostFixture.hierarchy_snapshot('svg')).focus==='leaf-a');
        await choose('Hierarchy root');
        assert(await page.evaluate(()=>['svg','canvas'].every(id=>{const s=JSON.parse(window.hostFixture.hierarchy_snapshot(id));return s.focus==='root'&&s.view.scale===1&&s.view.x===0&&s.view.y===0;})),'Hierarchy menu used a camera transform or failed to restore root');
      }
      await page.evaluate(()=>window.configuredStaleControl?.click());
      const first=await page.locator(`#${root}-svg svg`).first().innerHTML();
      for(let phase=1;phase<=phases;phase++){
        await choose('Change styles');
        await page.waitForFunction(({prefix,phase,phases})=>['svg','canvas'].every(id=>{const state=JSON.parse(window.hostFixture[`${prefix}_snapshot`](id));return state.active&&!state.pending&&state.phase===phase%phases;}),{prefix,phase,phases});
        if(phase===1)assert(await page.locator(`#${root}-svg svg`).first().innerHTML()!==first,`${name}: Change styles changed metadata without rendering`);
      }
      // Selecting the same mode starts a fresh, phase-zero owner as well.
      await page.evaluate(root=>{window.configuredOldSurface=document.querySelector(`#${root}-svg .kartis-surface`);},root);
      await choose(name);await page.waitForFunction(({prefix})=>JSON.parse(window.hostFixture[`${prefix}_snapshot`]('svg')).phase===0,{prefix});
      assert(await page.evaluate(()=>!window.configuredOldSurface.isConnected),`${name}: same-mode remount retained its previous surface`);
      styles.push({name,case:kind,phases,actualRenderers:true,phaseControls:true,switchCleanup:true,sameModeRemount:true,...(prefix==='hierarchy'?{focusBackRoot:true,nativeKeyboardFocus:true,identityCamera:true}: {})});
    }
    await choose('Transform scene');
    await page.waitForFunction(()=>['svg','canvas'].every(id=>JSON.parse(window.hostFixture.snapshot(id)).active));
    assert(await page.locator(families.flatMap(([,prefix,root])=>[`#${root}-svg`,`#${root}-canvas`]).join(',')).count()===0,'Returning to ordinary charts retained configured owners');
    assert(await page.locator('.panels').evaluate(el=>getComputedStyle(el).display!=='none'),'Returning to ordinary charts left their panels hidden');
    assert(await page.getByRole('button',{name:'Change styles',exact:true}).isDisabled()&&await page.getByRole('button',{name:'Animate',exact:true}).isEnabled()&&await page.getByRole('button',{name:'Reset view',exact:true}).isEnabled(),'Ordinary controls were not restored');
    assert(await page.locator('#heatmap-target').isDisabled(),'Ordinary chart kept HeatMap-only peer selector enabled');
    assert(await page.locator('#hierarchy-focus').isDisabled()&&await page.locator('#hierarchy-back').isDisabled()&&await page.locator('#hierarchy-root').isDisabled(),'Ordinary chart retained hierarchy-only navigation');
    assert(!errors.length,errors.join('\n'));
    return{passed:true,tree:true,parallelHorizontal:true,parallelVertical:true,network:true,styles,switchCleanup:true,pageErrors:errors};
  }finally{await choose('Transform scene');await page.evaluate(()=>{delete window.configuredStaleControl;delete window.configuredOldSurface;});page.off('pageerror',listener);}
})
