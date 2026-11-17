// Original SVG and Canvas compared to CPU flat ink, across region/cap/dash masks.
import { readFile, writeFile } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import { createServer } from 'node:http';
import { resolve, relative, isAbsolute, join, extname } from 'node:path';
import { chromium, firefox, webkit } from 'playwright';

const metadata = JSON.parse(execFileSync('cargo', ['metadata', '--no-deps', '--format-version', '1'], {encoding:'utf8'}));
const args = process.argv.slice(2).filter(arg => !arg.startsWith('--'));
const root = resolve(args[0] || join(metadata.target_directory, 'geo-usa-streams-final'));
const output = args[1] || join(root, 'browser-report.json');
const frames = JSON.parse(await readFile(join(root, 'frames.json'), 'utf8'));
const samples = frames.reduce((count, frame) => count + frame.samples.length, 0);
if (frames.length !== 37 || samples !== 240) throw Error(`Unexpected fixture inventory: ${frames.length} frames, ${samples} samples`);
for (const region of ['Lower48', 'Alaska', 'Hawaii']) {
  for (const cap of ['Butt', 'Round', 'Square']) {
    for (const dash of ['solid', 'dash']) {
      const original = frames.find(f => f.name === `${region}-${cap}-${dash}-original`);
      const masked = frames.find(f => f.name === `${region}-${cap}-${dash}-mask`);
      if (!original || !masked || original.samples.length !== 6 || masked.samples.length !== 6 ||
          original.samples.some(s => !s.hit) || masked.samples.filter(s => !s.hit).length !== 3) {
        throw Error(`Missing real stroke-spill witness for ${region}/${cap}/${dash}`);
      }
    }
  }
}
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
const report = {passed:false, fixtureFrames:frames.length, fixtureSamples:samples, contexts:[]};
try {
  const browsers = process.argv.includes('--chromium-only') ? [['Chromium',chromium,{}]] :
    [['Chrome',chromium,{channel:'chrome'}],['Firefox',firefox,{}],['WebKit',webkit,{}],['Edge',chromium,{channel:'msedge'}]];
  for (const [name, engine, options] of browsers) {
    const browser = await engine.launch({headless:true, ...options});
    try {
      for (const dpr of [1,2]) {
        const context = await browser.newContext({deviceScaleFactor:dpr});
        try {
          const page = await context.newPage();
          const errors = [];
          page.on('pageerror', error => errors.push(String(error)));
          await page.goto(url);
          const result = await page.evaluate(async () => {
            const frames = JSON.parse(document.querySelector('#frames').textContent);
            let comparisons = 0;
            for (const frame of frames) {
              const native = document.getElementById(`canvas-${frame.name}`);
              const image = new Image(); image.src = `${frame.name}.svg`; await image.decode();
              const svg = document.createElement('canvas'); svg.width = 800 * devicePixelRatio; svg.height = 500 * devicePixelRatio;
              svg.getContext('2d').drawImage(image,0,0,svg.width,svg.height);
              for (const canvas of [native,svg]) {
                const scale = canvas.width / 800;
                for (const sample of frame.samples) {
                  const actual = [...canvas.getContext('2d').getImageData(Math.floor(sample.point[0]*scale),Math.floor(sample.point[1]*scale),1,1).data];
                  if (actual.some((v,i) => Math.abs(v-sample.rgba[i]) > 1)) throw Error(`${frame.name} ${canvas===native?'Canvas':'SVG'} at ${sample.point}: ${actual} != ${sample.rgba}`);
                  comparisons++;
                }
              }
            }
            return {frames:frames.length, comparisons};
          });
          if (errors.length || result.frames !== frames.length || result.comparisons !== samples * 2) throw Error(JSON.stringify({errors,result}));
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
