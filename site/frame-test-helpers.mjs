import {readFile} from 'node:fs/promises';
import {createServer} from 'node:http';
import {dirname, extname, resolve, sep} from 'node:path';
import {fileURLToPath} from 'node:url';

const site = dirname(fileURLToPath(import.meta.url));

export async function serve() {
  const dist = resolve(site, 'dist');
  const types = {'.html':'text/html', '.js':'text/javascript', '.css':'text/css', '.wasm':'application/wasm', '.json':'application/json', '.svg':'image/svg+xml'};
  const server = createServer(async (request, response) => {
    try {
      const path = decodeURIComponent(new URL(request.url, 'http://localhost').pathname);
      const file = resolve(dist, '.' + (path === '/' ? '/index.html' : path));
      if (!file.startsWith(dist + sep)) { response.writeHead(403).end(); return; }
      const content = await readFile(file);
      response.writeHead(200, {'Content-Type':types[extname(file)] ?? 'application/octet-stream'}).end(content);
    } catch { response.writeHead(404).end(); }
  });
  await new Promise((done, reject) => server.once('error',reject).listen(0,'127.0.0.1',done));
  return server;
}

// Both owners receive exactly the same RAF times. Invalidating the backing size
// before the reference paint forces the renderer to recreate the entire bitmap.
export function installFrameClock(full) {
  const queue = new Map();
  let token = 0, time = 1000;
  window.requestAnimationFrame = fn => { queue.set(++token,fn); return token; };
  window.cancelAnimationFrame = id => queue.delete(id);
  window.advanceChartFrame = () => {
    const callbacks = [...queue.keys()];
    time += 16;
    const state = window.kartisExplorer?.snapshot();
    const canvas = document.querySelector('#chart-stage .kartis-surface > canvas');
    const invalidate = full && canvas && callbacks.length && (state?.animating || state?.pending);
    if (invalidate) canvas.width++;
    // A callback can cancel a later callback from the same frame (e.g. an unmount).
    for (const id of callbacks) {
      const callback = queue.get(id);
      queue.delete(id);
      if (callback) callback(time);
    }
    return Boolean(invalidate);
  };
}
