import init, {catalog_json, thumbnail_svg, mount_chart, next_dataset, gallery_snapshot, input_json, source_code, export_svg, stop_gallery} from './pkg/kartis_site.js';

const $ = selector => document.querySelector(selector);
const media = matchMedia('(prefers-reduced-motion: reduce)');
const state = {slug:'line',detail:false,renderer:'svg',animate:true,preset:'default',theme:'light',variant:'',category:'All charts',query:'',auto:false,tab:'chart',suspended:false};
let catalog = [], autoTimer = null, statusTimer = null, routedUrl = null, stageHeight = 0;
const glyphs = {Cartesian:'╱','Part to whole':'◕',Hierarchy:'⋔',Network:'⋈',Geography:'◇',Specialized:'▦'};
function report(error) {
  setAuto(false);
  console.error(error);
  $('#error').hidden=false;
  $('#error').textContent='This chart could not complete the update. '+String(error)+' You can choose another chart or turn motion off.';
}
function safely(action) {try {return action();} catch(error) {report(error);}}
function snapshot() {return JSON.parse(gallery_snapshot());}
function closeMenu() {$('#sidebar').classList.remove('open');$('#nav-backdrop').hidden=true;$('#menu-toggle').setAttribute('aria-expanded','false');}
function entry(slug=state.slug) {return catalog.find(kind=>kind.slug===slug);}
function defaultVariant(slug) {return entry(slug)?.variants[0]?.value??'';}
function setTab(tab) {
  state.tab=tab;
  for(const button of document.querySelectorAll('[data-tab]')) button.setAttribute('aria-selected',String(button.dataset.tab===tab));
  for(const name of ['chart','code','data']) $('#panel-'+name).hidden=name!==tab;
  if(tab!=='chart') updatePanels();
}
function updatePanels() {
  $('#code-output').textContent=source_code();
  $('#data-output').textContent=input_json();
}
function sync() {
  const current=snapshot();
  document.documentElement.dataset.ready='true';
  document.documentElement.dataset.renderer=state.renderer;
  document.documentElement.dataset.dataset=String(current.dataset??1);
  document.documentElement.dataset.chart=state.slug;
  document.documentElement.dataset.animating=String(!!current.animating);
  $('#dataset-status').textContent='Dataset '+(current.dataset??1);
  $('#motion-status').classList.toggle('moving',!!current.animating);
  $('#motion-status').replaceChildren();
  $('#motion-status').append(document.createElement('i'),document.createTextNode(current.animating?'Spring in motion':(media.matches||!state.animate?'Instant updates':'Spring at rest')));
  $('#mark-count').textContent=(current.marks??0)+' rendered marks · '+Math.round(current.width??0)+' × '+Math.round(current.height??0);
  document.querySelectorAll('[data-renderer]').forEach(button=>button.setAttribute('aria-pressed',String(button.dataset.renderer===state.renderer)));
  $('.render-label').textContent=state.renderer.toUpperCase()+' · RUST / WASM';
  $('#motion-note').textContent=media.matches?'Reduced motion is enabled in your system. Charts update instantly and the automatic demo is disabled.':'Your system’s reduced-motion preference is respected. Some path topology changes crossfade instead of morphing.';
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
  mount_chart(state.slug,state.renderer,state.animate,state.preset,state.theme,state.variant);
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
  $('#variant-field').hidden=!variants.length;
  $('#variant-select').replaceChildren(...variants.map(v=>new Option(v.label,v.value)));
  $('#variant-select').value=state.variant;
}
function showGallery({history=true}={}) {
  state.detail=false;state.slug='line';state.variant=defaultVariant('line');
  $('#catalog-view').hidden=false;$('#detail-view').hidden=true;
  $('#hero-mount').append($('#stage-shell'));$('#stage-shell').classList.add('overview-stage');
  setTab('chart');mountCurrent();closeMenu();
  if(history){const target=location.pathname+location.search;if(location.hash)window.history.pushState(null,'',target);routedUrl=location.href;}
  $('#status').textContent='Explore '+catalog.length+' real chart types. Choose a thumbnail to open its playground.';
}
function selectChart(slug,{history=true}={}) {
  const kind=entry(slug);if(!kind)throw new Error('Unknown chart: '+slug);
  setAuto(false);state.detail=true;state.slug=slug;state.variant=defaultVariant(slug);
  $('#catalog-view').hidden=true;$('#detail-view').hidden=false;
  $('#detail-mount').append($('#stage-shell'));$('#stage-shell').classList.remove('overview-stage');
  $('#chart-title').textContent=kind.name;$('#chart-category').textContent=kind.category;$('#chart-description').textContent=kind.description;$('#map-note').hidden=!['choropleth','geo-map'].includes(slug);
  configureVariants();setTab('chart');mountCurrent();closeMenu();
  if(history){const target='#chart/'+encodeURIComponent(slug);if(location.hash!==target)window.history.pushState(null,'',target);routedUrl=location.href;}
  $('#status').textContent=kind.name+' · real Rust layout, native interaction and property springs';
  scrollTo({top:0,behavior:'instant'});
}
function matches(kind) {return(state.category==='All charts'||kind.category===state.category)&&[kind.name,kind.slug,kind.category,kind.description].join(' ').toLowerCase().includes(state.query);}
function filterCatalog() {
  let count=0;
  for(const card of document.querySelectorAll('.catalog-card')) {const visible=matches(entry(card.dataset.slug));card.hidden=!visible;if(visible)count++;}
  for(const button of document.querySelectorAll('#chart-nav [data-slug]')) button.hidden=!matches(entry(button.dataset.slug));
  $('#result-count').textContent=String(count);$('#category-title').firstChild.textContent=state.category+' ';
  $('#no-results').hidden=count!==0;
  document.querySelectorAll('[data-category]').forEach(button=>button.setAttribute('aria-pressed',String(button.dataset.category===state.category)));
}
function drawPreviews() {
  for(const card of document.querySelectorAll('.catalog-card')) {
    const svg=thumbnail_svg(card.dataset.slug,state.theme);
    card.querySelector('.thumbnail').innerHTML=svg;
    const element=card.querySelector('svg');element.setAttribute('aria-hidden','true');element.setAttribute('focusable','false');
  }
}
function buildCatalog() {
  const categories=['All charts',...new Set(catalog.map(kind=>kind.category))];
  $('#category-list').replaceChildren(...categories.map(category=>{
    const button=document.createElement('button');button.type='button';button.dataset.category=category;button.setAttribute('aria-pressed',String(category===state.category));
    const label=document.createElement('span');label.textContent=category;const count=document.createElement('small');count.textContent=String(category==='All charts'?catalog.length:catalog.filter(kind=>kind.category===category).length);button.append(label,count);
    button.addEventListener('click',()=>safely(()=>{state.category=category;if(state.detail)showGallery();filterCatalog();closeMenu();}));return button;
  }));
  $('#chart-nav').replaceChildren(...catalog.map(kind=>{
    const button=document.createElement('button');button.type='button';button.dataset.slug=kind.slug;button.setAttribute('aria-current','false');const icon=document.createElement('span');icon.className='nav-glyph';icon.setAttribute('aria-hidden','true');icon.textContent=glyphs[kind.category]??'◇';button.append(icon,document.createTextNode(kind.name));button.addEventListener('click',()=>safely(()=>selectChart(kind.slug)));return button;
  }));
  $('#catalog-grid').replaceChildren(...catalog.map(kind=>{
    const button=document.createElement('button');button.type='button';button.className='catalog-card';button.dataset.slug=kind.slug;button.setAttribute('aria-label','Explore '+kind.name+' chart');
    const preview=document.createElement('div');preview.className='thumbnail';
    const meta=document.createElement('div');meta.className='card-meta';const heading=document.createElement('div');const title=document.createElement('h3');title.textContent=kind.name;const arrow=document.createElement('span');arrow.textContent='↗';heading.append(title,arrow);const detail=document.createElement('p');detail.textContent=kind.description;const category=document.createElement('div');category.className='card-category';category.textContent=kind.category.toUpperCase();meta.append(heading,detail,category);button.append(preview,meta);button.addEventListener('click',()=>safely(()=>selectChart(kind.slug)));return button;
  }));
  $('#chart-count').textContent=String(catalog.length);$('#type-count').textContent=String(catalog.length);drawPreviews();filterCatalog();
}
function toggleTheme() {
  state.theme=state.theme==='light'?'dark':'light';
  document.documentElement.dataset.theme=state.theme;
  $('#theme-toggle').setAttribute('aria-pressed',String(state.theme==='dark'));$('#theme-toggle').setAttribute('aria-label',state.theme==='dark'?'Switch to light theme':'Switch to dark theme');
  drawPreviews();mountCurrent();
}
function downloadSvg() {
  const url=URL.createObjectURL(new Blob([export_svg()],{type:'image/svg+xml'}));const link=document.createElement('a');link.href=url;link.download='kartis-'+state.slug+'.svg';link.click();setTimeout(()=>URL.revokeObjectURL(url),1000);
}
try {
  await init();catalog=JSON.parse(catalog_json());
  state.variant=defaultVariant('line');buildCatalog();
  $('#search').addEventListener('input',event=>{state.query=event.target.value.trim().toLowerCase();if(state.detail)safely(()=>showGallery());filterCatalog();});
  $('#menu-toggle').addEventListener('click',()=>{const open=!$('#sidebar').classList.contains('open');$('#sidebar').classList.toggle('open',open);$('#nav-backdrop').hidden=!open;$('#menu-toggle').setAttribute('aria-expanded',String(open));});
  $('#nav-backdrop').addEventListener('click',closeMenu);
  addEventListener('keydown',event=>{if(event.key==='/'&&!['INPUT','TEXTAREA','SELECT'].includes(document.activeElement.tagName)){event.preventDefault();$('#sidebar').classList.add('open');if(innerWidth<=900)$('#nav-backdrop').hidden=false;$('#search').focus();}if(event.key==='Escape')closeMenu();});
  $('#hero-open').addEventListener('click',()=>safely(()=>selectChart('line')));
  $('#back-to-gallery').addEventListener('click',()=>safely(()=>showGallery()));
  document.querySelectorAll('[data-renderer]').forEach(button=>button.addEventListener('click',()=>safely(()=>{state.renderer=button.dataset.renderer;mountCurrent();button.focus({preventScroll:true});})));
  $('#new-data').addEventListener('click',()=>safely(randomize));
  $('#auto-demo').addEventListener('click',()=>setAuto(!state.auto));
  $('#animation-enabled').addEventListener('change',event=>safely(()=>{state.animate=event.target.checked;mountCurrent();}));
  $('#spring-preset').addEventListener('change',event=>safely(()=>{state.preset=event.target.value;mountCurrent();}));
  $('#variant-select').addEventListener('change',event=>safely(()=>{state.variant=event.target.value;mountCurrent();}));
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
  addEventListener('pagehide',()=>{state.suspended=true;clearInterval(autoTimer);clearInterval(statusTimer);stop_gallery();});
  addEventListener('pageshow',event=>{if(event.persisted){state.suspended=false;safely(mountCurrent);startStatus();setAuto(state.auto);}});
  document.addEventListener('visibilitychange',()=>{if(document.hidden){clearInterval(autoTimer);clearInterval(statusTimer);}else if(!state.suspended){startStatus();setAuto(state.auto);}});
} catch(error) {report(error);}
