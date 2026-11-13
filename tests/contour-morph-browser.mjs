// Run after the Rust raster test exports KARTIS_MORPH_ARTIFACT_DIR fixtures:
// node tests/contour-morph-browser.mjs <absolute-artifact-directory>
// Uses the existing optional browser acceptance devDependency, no chart JS.
import { readFile, writeFile } from 'node:fs/promises';
import { join, isAbsolute } from 'node:path';
import { chromium } from 'playwright';

const directory = process.argv[2];
if (!directory || !isAbsolute(directory)) throw Error('Provide absolute morph artifact directory');
const fixtures = await Promise.all(['initial', 'middle', 'target'].map(async name => ({
  name,
  svg: `data:image/svg+xml;base64,${(await readFile(join(directory, `${name}.svg`))).toString('base64')}`,
  png: `data:image/png;base64,${(await readFile(join(directory, `${name}.png`))).toString('base64')}`,
})));
const browser = await chromium.launch({ headless: true, ...(process.env.PLAYWRIGHT_CHANNEL ? { channel: process.env.PLAYWRIGHT_CHANNEL } : {}) });
try {
  const page = await browser.newPage();
  const report = await page.evaluate(async fixtures => {
    async function pixels(source) {
      const image = new Image();
      await new Promise((resolve, reject) => { image.onload = resolve; image.onerror = reject; image.src = source; });
      if (image.width !== 240 || image.height !== 200) throw Error('Unexpected fixture extent');
      const canvas = document.createElement('canvas');
      canvas.width = 240; canvas.height = 200;
      const ctx = canvas.getContext('2d');
      ctx.drawImage(image, 0, 0);
      return ctx.getImageData(0, 0, 240, 200).data;
    }
    const results = [];
    for (const fixture of fixtures) {
      const svg = await pixels(fixture.svg);
      const cpu = await pixels(fixture.png);
      const pixel = (data, x, y) => [...data.slice((y * 240 + x) * 4, (y * 240 + x) * 4 + 4)];
      for (const [x, y, expected] of [[120, 100, [255,255,255,255]], [170, 100, [30,90,160,255]], [5,5,[255,255,255,255]]]) {
        for (const data of [svg, cpu]) {
          if (JSON.stringify(pixel(data, x, y)) !== JSON.stringify(expected)) throw Error(`${fixture.name} incorrect hole/ring pixel ${x},${y}`);
        }
      }
      const meanChannelError = svg.reduce((sum, value, i) => sum + Math.abs(value - cpu[i]), 0) / svg.length;
      if (meanChannelError > 0.5) throw Error(`${fixture.name} unexpected SVG/CPU error ${meanChannelError}`);
      results.push({ name: fixture.name, hole: pixel(svg,120,100), ring: pixel(svg,170,100), meanChannelError });
    }
    return { cases: results, errors: [] };
  }, fixtures);
  await writeFile(join(directory, 'browser-report.json'), JSON.stringify(report, null, 2));
  console.log(JSON.stringify(report, null, 2));
} finally { await browser.close(); }
