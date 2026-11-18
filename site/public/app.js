import init, {catalog_json, thumbnail_svg, mount_chart, configure_chart, configure_motion, configure_theme, replay_entrance, next_dataset, gallery_snapshot, input_json, source_code, export_svg, stop_gallery} from './pkg/kartis_site.js';

const $ = selector => document.querySelector(selector);
const media = matchMedia('(prefers-reduced-motion: reduce)');
const state = {slug:'line',detail:false,renderer:'svg',animate:true,preset:'default',theme:'light',variant:'',category:'All charts',query:'',auto:false,tab:'chart',suspended:false};
let catalog = [], autoTimer = null, statusTimer = null, routedUrl = null, stageHeight = 0;
const layouts = new Map();
const springs = {default:[170,26], gentle:[120,14], wobbly:[180,12], stiff:[210,20], slow:[100,28], molasses:[280,120]};
let spacing = 100;
let chartSettings = {};
// The plot has a fixed responsive height. Give wrapped or stacked controls
// room below it without resizing the plot or restarting its animation.
function fitChartControls() {
  const stage=$('#chart-stage');
  const controls=stage.querySelector('.kartis-controls');
  const overflow=controls ? Math.ceil(controls.getBoundingClientRect().bottom-stage.getBoundingClientRect().bottom) : 0;
  $('#panel-chart').style.paddingBottom=Math.max(0,overflow)+'px';
}
const controlObserver=new ResizeObserver(fitChartControls);
function report(error) {
  setAuto(false);
  console.error(error);
  $('#error').hidden=false;
  $('#error').textContent='This chart could not complete the update. '+String(error)+' You can choose another chart or turn motion off.';
}
function safely(action) {try {return action();} catch(error) {report(error);}}
function snapshot() {return JSON.parse(gallery_snapshot());}
const drawer = matchMedia('(max-width: 900px)');
function setMenu(open) {
  const visible = open && drawer.matches;
  $('#sidebar').classList.toggle('open',visible);
  $('#nav-backdrop').hidden = !visible;
  $('#menu-toggle').setAttribute('aria-expanded',String(visible));
}
function closeMenu() { setMenu(false); }
function revealActiveChart() {
  const nav = $('#chart-nav');
  const active = nav.querySelector('[aria-current="true"]');
  if (!active) return;
  const viewport = nav.getBoundingClientRect();
  const item = active.getBoundingClientRect();
  if (item.top < viewport.top) nav.scrollTop += item.top - viewport.top - 8;
  else if (item.bottom > viewport.bottom) nav.scrollTop += item.bottom - viewport.bottom + 8;
}
function entry(slug=state.slug) {return catalog.find(kind=>kind.slug===slug);}
function defaultVariant(slug) {return entry(slug)?.variants[0]?.value??'';}
function setTab(tab) {
  state.tab=tab;
  for(const button of document.querySelectorAll('[data-tab]')) button.setAttribute('aria-selected',String(button.dataset.tab===tab));
  for(const name of ['chart','code','data']) $('#panel-'+name).hidden=name!==tab;
  if(tab!=='chart') updatePanels();
}
function updatePanels() {
  setText($('#code-output'),source_code());
  setText($('#data-output'),input_json());
}
function setText(element, value) {
  if(element.textContent!==value) element.textContent=value;
}
function setAttribute(element, name, value) {
  if(element.getAttribute(name)!==value) element.setAttribute(name,value);
}
function sync() {
  const current=snapshot();
  const root=document.documentElement;
  const dataset=String(current.dataset??1);
  const instant=media.matches||!state.animate;
  setAttribute(root,'data-ready','true');
  setAttribute(root,'data-renderer',state.renderer);
  setAttribute(root,'data-dataset',dataset);
  setAttribute(root,'data-chart',state.slug);
  setAttribute(root,'data-animating',String(!!current.animating));
  setText($('#dataset-status'),'Dataset '+dataset);
  const replay=$('#replay-entrance');
  if(replay.disabled!==instant) replay.disabled=instant;
  const motion=$('#motion-status');
  motion.classList.toggle('moving',!!current.animating);
  setText(motion.lastChild,current.animating?'Spring in motion':(instant?'Instant updates':'Spring at rest'));
  setText($('#mark-count'),(current.marks??0)+' rendered marks · '+Math.round(current.width??0)+' × '+Math.round(current.height??0));
  document.querySelectorAll('.renderer-switch button[data-renderer]').forEach(button=>setAttribute(button,'aria-pressed',String(button.dataset.renderer===state.renderer)));
  setText($('.render-label'),state.renderer.toUpperCase()+' · RUST / WASM');
  setText($('#motion-note'),media.matches?'Reduced motion is enabled in your system. Charts update instantly and the automatic demo is disabled.':'Your system’s reduced-motion preference is respected. Some path topology changes crossfade instead of morphing.');
  if(current.errors?.length) throw new Error(current.errors.at(-1));
  return current;
}
function startStatus() {
  clearInterval(statusTimer);
  statusTimer=setInterval(()=>{if(!state.suspended&&!document.hidden)safely(sync);},100);
}
function mountCurrent() {
  $('#error').hidden=true;
  stageHeight=$('#chart-stage').getBoundingClientRect().height;
  mount_chart(state.slug,state.renderer,state.animate,state.preset,state.theme,JSON.stringify([state.variant,chartSettings]),spacing);
  controlObserver.disconnect();
  const controls=$('#chart-stage .kartis-controls');
  if(controls) controlObserver.observe(controls);
  fitChartControls();
  updatePanels();sync();
  document.querySelectorAll('#chart-nav [data-slug]').forEach(button=>button.setAttribute('aria-current',String(state.detail&&button.dataset.slug===state.slug)));
}
function randomize() {
  next_dataset();updatePanels();sync();
}
function setAuto(enabled) {
  clearInterval(autoTimer);autoTimer=null;
  state.auto=!!enabled&&!media.matches;
  $('#auto-demo').setAttribute('aria-pressed',String(state.auto));
  $('#auto-demo').innerHTML=state.auto?'Ⅱ <span>Pause demo</span>':'▶ <span>Play demo</span>';
  $('#auto-demo').title='Automatically change the dataset every 2.8 seconds. Pausing lets the current spring finish.';
  if(state.auto&&!document.hidden&&!state.suspended) autoTimer=setInterval(()=>safely(randomize),2800);
}
function configureVariants() {
  const variants=entry()?.variants??[];
  $('#bump-controls').hidden=state.slug!=='bump';
  $('#variant-field').hidden=!variants.length;
  $('#variant-select').replaceChildren(...variants.map(v=>new Option(v.label,v.value)));
  $('#variant-select').value=state.variant;
  const groups=new Map();
  for(const {key,label,choices,group} of entry().controls){
    if(!groups.has(group)){
      const section=document.createElement('fieldset');
      section.className='chart-properties';
      section.dataset.group=group;
      const heading=document.createElement('legend');
      heading.textContent=group;
      section.append(heading);
      groups.set(group,section);
    }
    const field=document.createElement('div');
    const select=document.createElement('select');
    select.id='chart-option-'+key;
    select.dataset.setting=key;
    const caption=document.createElement('label');
    caption.htmlFor=select.id;
    caption.textContent=label;
    select.replaceChildren(...choices.map(([value,name])=>new Option(name,value)));
    select.value=chartSettings[key]??choices[0][0];
    field.append(caption,select);
    groups.get(group).append(field);
  }
  $('#chart-controls').replaceChildren(...groups.values());
  $('#plot-spacing').value=String(spacing);
  $('#spacing-value').value=spacing+'%';
}
function changeLayout() {
  const variant = $('#variant-select').value;
  const percent = Number($('#plot-spacing').value);
  const settings={};
  for(const control of entry().controls){
    const value=$('#chart-option-'+control.key).value;
    if(value!==control.choices[0][0]) settings[control.key]=value;
  }
  configure_chart(JSON.stringify([variant,settings]),percent,Number($('#bump-width').value),$('#bump-labels').checked);
  fitChartControls();
  chartSettings=settings;
  state.variant = variant;
  spacing = percent;
  layouts.set(state.slug,{variant,spacing,settings});
  $('#spacing-value').value=spacing+'%';
  updatePanels();
  sync();
}
function changeSpring(custom) {
  const selector = $('#spring-preset');
  if(custom) selector.value='custom';
  const preset = selector.value;
  if(preset!=='custom') {
    const [stiffness,damping]=springs[preset];
    $('#spring-stiffness').value=stiffness;
    $('#spring-damping').value=damping;
  }
  const stiffness=$('#spring-stiffness').value;
  const damping=$('#spring-damping').value;
  $('#stiffness-value').value=stiffness;
  $('#damping-value').value=damping;
  state.preset=preset==='custom'?`custom/${stiffness}/${damping}`:preset;
  configure_motion(state.animate,state.preset);
  sync();
}
function showGallery({history=true}={}) {
  state.detail=false;state.slug='line';state.variant=defaultVariant('line');spacing=100;chartSettings={};
  $('#catalog-view').hidden=false;$('#detail-view').hidden=true;
  drawPreviews();
  $('#hero-mount').append($('#stage-shell'));$('#stage-shell').classList.add('overview-stage');
  setTab('chart');mountCurrent();closeMenu();
  if(history){const target=location.pathname+location.search;if(location.hash)window.history.pushState(null,'',target);routedUrl=location.href;}
  $('#status').textContent='Explore '+catalog.length+' real chart types. Choose a thumbnail to open its playground.';
}
function selectChart(slug,{history=true}={}) {
  const kind=entry(slug);if(!kind)throw new Error('Unknown chart: '+slug);
  setAuto(false);state.detail=true;state.slug=slug;
  state.variant=layouts.get(slug)?.variant??defaultVariant(slug);
  spacing=layouts.get(slug)?.spacing??100;
  chartSettings=layouts.get(slug)?.settings??{};
  $('#catalog-view').hidden=true;$('#detail-view').hidden=false;
  $('#detail-mount').append($('#stage-shell'));$('#stage-shell').classList.remove('overview-stage');
  $('#chart-title').textContent=kind.name;$('#chart-category').textContent=kind.category;$('#chart-description').textContent=kind.description;$('#map-note').hidden=!['choropleth','geo-map'].includes(slug);
  configureVariants();setTab('chart');mountCurrent();closeMenu();
  revealActiveChart();
  if(history){const target='#chart/'+encodeURIComponent(slug);if(location.hash!==target)window.history.pushState(null,'',target);routedUrl=location.href;}
  $('#status').textContent=kind.name+' · real Rust layout, native interaction and property springs';
  scrollTo({top:0,behavior:'instant'});
}
function matches(kind) {return(state.category==='All charts'||kind.category===state.category)&&[kind.name,kind.slug,kind.category,kind.description].join(' ').toLowerCase().includes(state.query);}
function filterCatalog() {
  let count=0;
  for(const card of document.querySelectorAll('.catalog-card')) {const visible=matches(entry(card.dataset.slug));card.hidden=!visible;if(visible)count++;}
  for(const button of document.querySelectorAll('#chart-nav [data-slug]')) button.hidden=!matches(entry(button.dataset.slug));
  for (const group of document.querySelectorAll('.nav-group')) group.hidden = !group.querySelector('button:not([hidden])');
  $('#result-count').textContent=String(count);$('#category-title').firstChild.textContent=state.category+' ';
  $('#no-results').hidden=count!==0;
  document.querySelectorAll('[data-category]').forEach(button=>button.setAttribute('aria-pressed',String(button.dataset.category===state.category)));
}
let previewTheme;
function drawPreviews() {
  if(previewTheme===state.theme)return;
  for(const card of document.querySelectorAll('.catalog-card')) {
    const svg=thumbnail_svg(card.dataset.slug,state.theme);
    card.querySelector('.thumbnail').innerHTML=svg;
    const element=card.querySelector('svg');element.setAttribute('aria-hidden','true');element.setAttribute('focusable','false');
  }
  previewTheme=state.theme;
}
function buildCatalog() {
  const categories=['All charts',...new Set(catalog.map(kind=>kind.category))];
  $('#category-list').replaceChildren(...categories.map(category=>{
    const button=document.createElement('button');button.type='button';button.dataset.category=category;button.setAttribute('aria-pressed',String(category===state.category));
    const label=document.createElement('span');label.textContent=category;const count=document.createElement('small');count.textContent=String(category==='All charts'?catalog.length:catalog.filter(kind=>kind.category===category).length);button.append(label,count);
    button.addEventListener('click',()=>safely(()=>{state.category=category;if(state.detail)showGallery();filterCatalog();closeMenu();}));return button;
  }));
  const navGroups = categories.slice(1).map(category => {
    const group = document.createElement('div');
    group.className = 'nav-group';
    const title = document.createElement('h3');
    title.textContent = category;
    group.append(title);
    for (const kind of catalog.filter(kind => kind.category === category)) {
      const button = document.createElement('button');
      button.type = 'button';
      button.dataset.slug = kind.slug;
      button.setAttribute('aria-current','false');
      button.textContent = kind.name;
      button.addEventListener('click', () => safely(() => selectChart(kind.slug)));
      group.append(button);
    }
    return group;
  });
  $('#chart-nav').replaceChildren(...navGroups);
  $('#catalog-grid').replaceChildren(...catalog.map(kind=>{
    const button=document.createElement('button');button.type='button';button.className='catalog-card';button.dataset.slug=kind.slug;button.setAttribute('aria-label','Explore '+kind.name+' chart');
    const preview=document.createElement('div');preview.className='thumbnail';
    const meta=document.createElement('div');meta.className='card-meta';const heading=document.createElement('div');const title=document.createElement('h3');title.textContent=kind.name;const arrow=document.createElement('span');arrow.textContent='↗';heading.append(title,arrow);const detail=document.createElement('p');detail.textContent=kind.description;const category=document.createElement('div');category.className='card-category';category.textContent=kind.category;meta.append(category,heading,detail);button.append(preview,meta);button.addEventListener('click',()=>safely(()=>selectChart(kind.slug)));return button;
  }));
  $('#chart-count').textContent=String(catalog.length);$('#type-count').textContent=String(catalog.length);drawPreviews();filterCatalog();
}
function toggleTheme() {
  state.theme=state.theme==='light'?'dark':'light';
  document.documentElement.dataset.theme=state.theme;
  $('#theme-toggle').setAttribute('aria-pressed',String(state.theme==='dark'));$('#theme-toggle').setAttribute('aria-label',state.theme==='dark'?'Switch to light theme':'Switch to dark theme');
  if(!state.detail)drawPreviews();
  configure_theme(state.theme);
  sync();
}
function downloadSvg() {
  const url=URL.createObjectURL(new Blob([export_svg()],{type:'image/svg+xml'}));const link=document.createElement('a');link.href=url;link.download='kartis-'+state.slug+'.svg';link.click();setTimeout(()=>URL.revokeObjectURL(url),1000);
}
try {
  await init();catalog=JSON.parse(catalog_json());
  state.variant=defaultVariant('line');buildCatalog();
  $('#search').addEventListener('input',event=>{
    state.query=event.target.value.trim().toLowerCase();
    if(state.detail)safely(()=>showGallery());
    filterCatalog();
    if(drawer.matches){setMenu(true);$('#search').focus();}
  });
  $('#menu-toggle').addEventListener('click',()=>{
    const open=!$('#sidebar').classList.contains('open');
    setMenu(open);
    if(open)$('#search').focus();
  });
  $('#nav-backdrop').addEventListener('click',()=>{closeMenu();$('#menu-toggle').focus();});
  drawer.addEventListener('change',closeMenu);
  addEventListener('keydown',event=>{
    if(event.key==='/'&&!['INPUT','TEXTAREA','SELECT'].includes(document.activeElement.tagName)){
      event.preventDefault();setMenu(true);$('#search').focus();
    }
    if(event.key==='Escape' && $('#sidebar').classList.contains('open')){
      closeMenu();$('#menu-toggle').focus();
    }
  });
  for (const [button, target] of [['skip-to-main','main'],['browse-collection','collection'],['jump-settings','chart-settings'],['jump-preview','detail-mount']]) {
    $('#'+button).addEventListener('click', event => {
      event.preventDefault();
      const section = $('#'+target);
      section.tabIndex = -1;
      section.focus({preventScroll:true});
      section.scrollIntoView({block:'start'});
    });
  }
  $('#hero-open').addEventListener('click',()=>safely(()=>selectChart('line')));
  $('#back-to-gallery').addEventListener('click',()=>safely(()=>showGallery()));
  document.querySelectorAll('.renderer-switch button[data-renderer]').forEach(button=>button.addEventListener('click',()=>safely(()=>{if(state.renderer!==button.dataset.renderer){state.renderer=button.dataset.renderer;mountCurrent();}button.focus({preventScroll:true});})));
  $('#new-data').addEventListener('click',()=>safely(randomize));
  $('#auto-demo').addEventListener('click',()=>setAuto(!state.auto));
  $('#animation-enabled').addEventListener('change',event=>safely(()=>{state.animate=event.target.checked;configure_motion(state.animate,state.preset);sync();}));
  $('#replay-entrance').addEventListener('click',()=>safely(()=>{replay_entrance();sync();}));
  $('#spring-preset').addEventListener('change',()=>safely(()=>changeSpring(false)));
  for(const name of ['stiffness','damping']) {
    const slider=$('#spring-'+name);
    slider.addEventListener('input',()=>$('#'+name+'-value').value=slider.value);
    slider.addEventListener('change',()=>safely(()=>changeSpring(true)));
  }
  $('#variant-select').addEventListener('change',()=>safely(changeLayout));
  $('#chart-controls').addEventListener('change',()=>safely(changeLayout));
  $('#bump-width').addEventListener('input',event=>$('#width-value').value=event.target.value+' px');
  for(const id of ['bump-width','bump-labels']) $('#'+id).addEventListener('change',()=>safely(changeLayout));
  $('#plot-spacing').addEventListener('input',event=>$('#spacing-value').value=event.target.value+'%');
  $('#plot-spacing').addEventListener('change',()=>safely(changeLayout));
  $('#theme-toggle').addEventListener('click',()=>safely(toggleTheme));$('#theme-setting').addEventListener('click',()=>safely(toggleTheme));
  $('#download-svg').addEventListener('click',()=>safely(downloadSvg));
  document.querySelectorAll('[data-tab]').forEach(button=>button.addEventListener('click',()=>safely(()=>setTab(button.dataset.tab))));
  for(const panel of ['code','data']) $('#copy-'+panel).addEventListener('click',()=>navigator.clipboard.writeText($('#'+panel+'-output').textContent).then(()=>{$('#copy-'+panel).textContent='Copied';setTimeout(()=>{$('#copy-'+panel).textContent='Copy';},1200);}).catch(()=>{$('#copy-'+panel).textContent='Select text to copy';}));
  media.addEventListener('change',()=>{if(media.matches)setAuto(false);safely(sync);});
  const route=()=>{if(routedUrl===location.href)return;routedUrl=location.href;const match=location.hash.match(/^#chart\/(.+)$/);safely(()=>match?selectChart(decodeURIComponent(match[1]),{history:false}):showGallery({history:false}));};
  addEventListener('popstate',route);addEventListener('hashchange',route);route();
  if(!state.detail&&!media.matches){setAuto(true);setTimeout(()=>{if(state.auto&&!state.suspended&&!document.hidden)safely(randomize);},700);}
  startStatus();$('#status').classList.add('ready');
  const stageObserver=new ResizeObserver(()=>{const height=$('#chart-stage').getBoundingClientRect().height;if(!state.suspended&&height>0&&Math.abs(height-stageHeight)>.5)safely(mountCurrent);});stageObserver.observe($('#chart-stage'));
  window.kartisExplorer=Object.freeze({catalog,select:slug=>selectChart(slug),showGallery:()=>showGallery(),snapshot,targetSvg:()=>export_svg(),input:()=>JSON.parse(input_json()),pause:()=>setAuto(false),randomize});
  addEventListener('pagehide',()=>{state.suspended=true;controlObserver.disconnect();clearInterval(autoTimer);clearInterval(statusTimer);stop_gallery();});
  addEventListener('pageshow',event=>{if(event.persisted){state.suspended=false;safely(mountCurrent);startStatus();setAuto(state.auto);}});
  document.addEventListener('visibilitychange',()=>{if(document.hidden){clearInterval(autoTimer);clearInterval(statusTimer);}else if(!state.suspended){startStatus();setAuto(state.auto);}});
} catch(error) {report(error);}
