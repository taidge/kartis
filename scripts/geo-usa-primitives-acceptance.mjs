// Projection/postclip primitive diagnostics, not integrated USA map acceptance.
import { readFile, writeFile } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import { createServer } from 'node:http';
import { resolve, relative, isAbsolute, join, extname } from 'node:path';
import { chromium, firefox, webkit } from 'playwright';

const metadata = JSON.parse(execFileSync('cargo', ['metadata', '--no-deps', '--format-version', '1'], {encoding:'utf8'}));
const root = resolve(process.argv[2] || join(metadata.target_directory, 'geo-usa-primitives-final'));
const output = process.argv[3] || join(root, 'browser-report.json');
const server = createServer(async (request, response) => {
  try {
    const name = decodeURIComponent(new URL(request.url, 'http://localhost').pathname);
    const file = resolve(root, '.' + (name === '/' ? '/index.html' : name));
    const child = relative(root, file);
    if (child.startsWith('..') || isAbsolute(child)) throw Error('Outside export directory');
    response.setHeader('Content-Type', {'.html':'text/html', '.svg':'image/svg+xml', '.png':'image/png', '.json':'application/json'}[extname(file)] || 'application/octet-stream');
    response.end(await readFile(file));
  } catch { response.writeHead(404); response.end(); }
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const url = `http://127.0.0.1:${server.address().port}/`;
const report = {passed:false, contexts:[]};
try {
  const browsers = process.argv.includes('--chromium-only') ? [['Chromium',chromium,{}]] : [['Chrome',chromium,{channel:'chrome'}],['Firefox',firefox,{}],['WebKit',webkit,{}],['Edge',chromium,{channel:'msedge'}]];
  for (const [name, engine, options] of browsers) {
    const browser = await engine.launch({headless:true, ...options});
    try {
      for (const dpr of [1,2]) {
        const context = await browser.newContext({deviceScaleFactor:dpr});
        try {
          const page = await context.newPage();
          const errors = []; page.on('pageerror', error => errors.push(String(error)));
          await page.goto(url);
          const result = await page.evaluate(async () => {
            const frames = JSON.parse(document.querySelector('#frames').textContent);
            let comparisons = 0;
            for (const frame of frames) {
              const native = document.getElementById(`canvas-${frame.name}`);
              const image = new Image(); image.src = `${frame.name}.svg`; await image.decode();
              const svg = document.createElement('canvas'); svg.width = 320 * devicePixelRatio; svg.height = 240 * devicePixelRatio;
              svg.getContext('2d').drawImage(image,0,0,svg.width,svg.height);
              for (const canvas of [native,svg]) {
                const scale = canvas.width / 320;
                for (const sample of frame.samples) {
                  const actual = [...canvas.getContext('2d').getImageData(Math.floor(sample.point[0]*scale),Math.floor(sample.point[1]*scale),1,1).data];
                  if (actual.some((v,i) => Math.abs(v-sample.rgba[i]) > 1)) throw Error(`${frame.name} ${canvas===native?'Canvas':'SVG'} at ${sample.point}: ${actual} != ${sample.rgba}`);
                  comparisons++;
                }
              }
            }
            return {frames:frames.length, comparisons};
          });
          if (errors.length || result.frames !== 10 || result.comparisons !== 74) throw Error(JSON.stringify({errors,result}));
          report.contexts.push({browser:name,dpr,...result,passed:true});
        } finally { await context.close(); }
      }
    } finally { await browser.close(); }
  }
  report.passed = report.contexts.length === browsers.length * 2;
} finally {
  server.close();
  await writeFile(output, JSON.stringify(report,null,2));
}
console.log(JSON.stringify(report,null,2));
