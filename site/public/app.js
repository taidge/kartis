import init, {mount_gallery, next_dataset, export_svg, stop_gallery} from './pkg/kartis_site.js';

const status = document.querySelector('#status');
const errorBox = document.querySelector('#error');
let renderer = 'svg';
let dataset = 1;
function report(error) {
  console.error(error);
  errorBox.hidden = false;
  errorBox.textContent = 'The gallery could not finish drawing. Please reload, or explore the documentation above. ' + String(error);
  status.textContent = 'Chart engine unavailable.';
}
function mounted() {
  status.classList.add('ready');
  status.textContent = `Running in your browser · ${renderer.toUpperCase()} · Dataset ${dataset} · Hover or tap a chart to explore`;
  document.querySelectorAll('[data-renderer]').forEach(button => button.setAttribute('aria-pressed', String(button.dataset.renderer === renderer)));
  document.querySelectorAll('.render-label').forEach(label => { label.textContent = renderer.toUpperCase() + ' RENDERER'; });
  document.documentElement.dataset.ready = 'true';
  document.documentElement.dataset.renderer = renderer;
  document.documentElement.dataset.dataset = String(dataset);
}
try {
  await init();
  mount_gallery(renderer);
  document.querySelectorAll('button').forEach(button => { button.disabled = false; });
  mounted();
  document.querySelectorAll('[data-renderer]').forEach(button => button.addEventListener('click', () => {
    if (renderer === button.dataset.renderer) return;
    try { mount_gallery(button.dataset.renderer); renderer = button.dataset.renderer; mounted(); button.focus({preventScroll:true}); } catch (error) { report(error); }
  }));
  document.querySelector('#new-data').addEventListener('click', () => {
    try { next_dataset(); dataset++; mounted(); document.querySelector('#new-data').focus({preventScroll:true}); } catch (error) { report(error); }
  });
  document.querySelectorAll('[data-download]').forEach(button => button.addEventListener('click', () => {
    try {
      const blob = new Blob([export_svg(button.dataset.download)], {type: 'image/svg+xml'});
      const url = URL.createObjectURL(blob);
      const link = document.createElement('a'); link.href = url; link.download = `kartis-${button.dataset.download}.svg`; link.click();
      setTimeout(() => URL.revokeObjectURL(url), 1000);
    } catch (error) { report(error); }
  }));
  window.addEventListener('pagehide', () => stop_gallery());
  window.addEventListener('pageshow', event => { if (event.persisted) { mount_gallery(renderer); mounted(); } });
} catch (error) { report(error); }
