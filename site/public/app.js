/* The browser shows the loading screen before it downloads the Rust application. */
const loading = document.getElementById('boot');
try {
  const wasm = await import('./pkg/kartis_site.js');
  await wasm.default();
  wasm.start_showcase();
  // The same observable API is used by the browser acceptance suite and by embeds.
  window.kartisExplorer = Object.freeze({
    catalog: JSON.parse(wasm.catalog_json()),
    select: wasm.open_chart,
    showGallery: wasm.open_gallery,
    pause: wasm.pause_updates,
    randomize: wasm.shuffle_data,
    snapshot: () => JSON.parse(wasm.gallery_snapshot()),
    targetSvg: wasm.export_svg,
    input: () => JSON.parse(wasm.input_json()),
  });
  loading.remove();
} catch (error) {
  console.error(error);
  loading.setAttribute('aria-busy', 'false');
  document.getElementById('boot-title').textContent = 'Kartis could not start';
  document.getElementById('boot-message').textContent = 'Check your connection and try loading the chart explorer again.';
  loading.querySelector('progress').hidden = true;
  const retry = document.getElementById('boot-retry');
  retry.hidden = false;
  retry.addEventListener('click', () => location.reload());
}
