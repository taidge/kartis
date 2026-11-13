(async function verifyBlends(page) {
    const assert = (condition, message) => { if (!condition) throw Error(message); };
    const pageErrors = [];
    const onError = error => pageErrors.push(error.message);
    page.on('pageerror', onError);
    const call = (name, ...args) => page.evaluate(([name, args]) => window.hostFixture[name](...args), [name, args]);
    const snapshot = async name => JSON.parse(await call('blend_snapshot', name));
    const flush = async () => {
        await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
        await call('blend_flush');
    };
    const near = (actual, expected, tolerance = 2) => actual.every((v, i) => Math.abs(v - expected[i]) <= tolerance);
    const expected = [
        [122,104,71,255], [173,153,125,255], [150,134,91,255], [135,120,80,255],
        [160,137,116,255], [195,171,146,255], [99,93,50,255], [144,135,105,255],
        [154,126,89,255], [125,92,86,255], [151,124,105,255], [129,130,109,255],
        [160,120,79,255], [128,131,109,255], [167,127,87,255],
    ];
    const base = [160,120,80,255];
    const probes = [[35,32], [32,32], [52,40], [35,40], [28,32], [101,22]];
    async function pixels(points, view = [1,0,0], staticFixture = null) {
        return page.evaluate(async ({ points, view, staticFixture }) => {
            const image = async source => { const image = new Image(); image.src = source; await image.decode(); return image; };
            const newCanvas = (w, h) => Object.assign(document.createElement('canvas'), { width: w, height: h });
            const liveCanvas = staticFixture ? newCanvas(120,80) : document.querySelector('#blend-canvas canvas');
            const liveSvg = staticFixture ? null : document.querySelector('#blend-svg svg');
            if (staticFixture) {
                liveCanvas.id = 'blend-script';
                document.body.append(liveCanvas);
                try { Function(staticFixture.script)(); }
                finally { liveCanvas.remove(); }
            }
            const svg = staticFixture ? new DOMParser().parseFromString(staticFixture.svg, 'image/svg+xml').documentElement : liveSvg.cloneNode(true);
            const logical = [svg.viewBox.baseVal.width, svg.viewBox.baseVal.height];
            const w = liveCanvas.width, h = liveCanvas.height;
            svg.setAttribute('width', String(w));
            svg.setAttribute('height', String(h));
            const raster = newCanvas(w, h).getContext('2d');
            const url = 'data:image/svg+xml;charset=utf-8,' + encodeURIComponent(new XMLSerializer().serializeToString(svg));
            raster.drawImage(await image(url), 0, 0);
            const a = raster.getImageData(0,0,w,h).data;
            const b = liveCanvas.getContext('2d').getImageData(0,0,w,h).data;
            const samples = points.map(([x,y]) => {
                const px = Math.floor((x * view[0] + view[1]) * w / logical[0]);
                const py = Math.floor((y * view[0] + view[2]) * h / logical[1]);
                const i = (py * w + px) * 4;
                return { svg: [...a.slice(i,i+4)], canvas: [...b.slice(i,i+4)] };
            });
            let sum = 0;
            for (let i=0; i<a.length; i++) sum += Math.abs(a[i]-b[i]);
            return { samples, mean: sum / a.length, backing: [w,h] };
        }, {points, view, staticFixture});
    }
    const sample = (pair, expected, label, tolerance = 2) => {
        assert(near(pair.svg, expected, tolerance) && near(pair.canvas, expected, tolerance), `${label}: ${JSON.stringify(pair)} != ${expected}`);
    };
    const opaque = (result, label) => {
        for (const [i,color] of [expected[0], [126,113,79,255], [151,102,64,255], base, base, [22,41,80,255]].entries()) {
            sample(result.samples[i], color, `${label} probe ${i}`);
        }
        assert(result.mean < 1.5, `${label} SVG and Canvas diverged by ${result.mean}`);
    };
    const panels = await page.evaluate(() => {
        window.hostFixture.blend_stop();
        const p = document.querySelector('.panels');
        const display = p.style.display;
        p.style.display = 'none';
        return display;
    });
    const baseline = await page.locator('body *').count();
    try {
        assert(!(await snapshot('svg')).active, 'Unmounted blend fixture reported active');
        await call('blend_mount');
        await page.evaluate(() => {
            const row = document.createElement('div');
            row.id = 'blend-fixture-row';
            row.style.cssText = 'display:flex;align-items:flex-start;gap:48px';
            document.body.append(row);
            for (const name of ['svg','canvas']) row.append(document.querySelector(`#blend-${name}`));
        });
        await flush();
        const initial = await pixels(probes);
        opaque(initial, 'retained initial');
        const staticResult = await pixels(probes, [1,0,0], JSON.parse(await call('blend_static', 1, 0)));
        opaque(staticResult, 'standalone and script');

        await page.evaluate(() => { window.blendKept = document.querySelector('#blend-svg [data-id="mixed"]'); });
        const allModes = [];
        for (let mode=1; mode<=15; mode++) {
            await call('blend_mode', mode);
            await flush();
            const result = await pixels([[35,32]]);
            sample(result.samples[0], expected[mode-1], `artistic mode ${mode}`);
            assert(result.mean < 1.5, `Mode ${mode} draw parity ${result.mean}`);
            allModes.push(result.samples[0]);
        }
        assert(await page.evaluate(() => window.blendKept === document.querySelector('#blend-svg [data-id="mixed"]')), 'Mode updates replaced the retained business path');
        await page.evaluate(() => {
            window.blendMutationCount = 0;
            window.blendObserver = new MutationObserver(records => { window.blendMutationCount += records.length; });
            window.blendObserver.observe(document.querySelector('#blend-svg svg'), {attributes:true, childList:true, characterData:true, subtree:true});
        });
        await call('blend_mode', 15);
        await flush();
        assert(await page.evaluate(() => {
            const changed = window.blendMutationCount + window.blendObserver.takeRecords().length;
            window.blendObserver.disconnect();
            return changed === 0;
        }), 'Identical mode/layout caused retained SVG mutations');

        await call('blend_mode', 1);
        await call('blend_phase', 1);
        await flush();
        const transparent = await pixels([[25,40],[50,40],[75,40],[90,40]]);
        for (const [i,color] of [[160,120,80,128],[162,89,49,192],[200,100,50,128],[0,0,0,0]].entries()) sample(transparent.samples[i],color,`transparent probe ${i}`);
        const shot = (await page.locator('#blend-svg .kartis-surface').screenshot()).toString('base64');
        const external = await page.evaluate(async bytes => {
            const image = new Image(); image.src = 'data:image/png;base64,' + bytes; await image.decode();
            const canvas = document.createElement('canvas'); canvas.width = image.width; canvas.height = image.height;
            const ctx = canvas.getContext('2d'); ctx.drawImage(image,0,0);
            const p = x => [...ctx.getImageData(Math.floor(x*image.width/120),Math.floor(image.height/2),1,1).data];
            return [p(50), p(75), p(90)];
        }, shot);
        assert(near(external[0], [143,93,89,255], 3) && near(external[1], [143,102,129,255], 3) && near(external[2], [86,104,208,255]), `External CSS background leaked into blend ${JSON.stringify(external)}`);

        await call('blend_phase', 0);
        await call('blend_zoom');
        await flush();
        const zoomed = await snapshot('svg');
        assert(zoomed.view[0] > 1, 'Blend fixture did not enter transformed view');
        const zoom = await pixels(probes.slice(0,5), zoomed.view);
        sample(zoom.samples[0], expected[0], 'zoom keeps shared backdrop');
        sample(zoom.samples[3], base, 'zoom keeps even-odd hole');
        assert(zoom.mean < 1.5, `Zoom blend parity ${zoom.mean}`);
        assert(await page.evaluate(() => document.querySelector('#blend-svg svg g[transform]').firstElementChild.tagName.toLowerCase() === 'rect'), 'Retained plot did not share its transformed group with the backdrop');

        await page.evaluate(() => { for (const r of ['svg','canvas']) document.querySelector(`#blend-${r}`).style.width='150px'; });
        await flush();
        const resized = await snapshot('svg');
        assert(resized.width === 150 && (await snapshot('canvas')).width === 150, 'Responsive blend layout did not publish new dimensions');
        const resize = await pixels([[35,32]], resized.view);
        sample(resize.samples[0], expected[0], 'resize preserves ID mode');
        await call('blend_viewport');
        await flush();
        assert((await snapshot('svg')).relayout && (await snapshot('canvas')).relayout, 'Data viewport failed to publish its accepted layout');
        const relayout = await pixels(probes);
        opaque(relayout, 'data viewport');

        const legend = page.locator('#blend-svg .kartis-legend-item').first();
        await legend.click();
        await flush();
        assert((await snapshot('svg')).hidden.includes('mixed-series') && await page.locator('#blend-svg [data-id="mixed"]').count() === 0, 'Hidden blend mark kept painting');
        await legend.click();
        await flush();
        sample((await pixels([[35,32]])).samples[0], expected[0], 'restored hidden ID preserves mode');

        const before = await snapshot('canvas');
        const rejected = JSON.parse(await call('blend_reject'));
        assert(rejected.stale && rejected.animation && rejected.budget, `Blend admission failed ${JSON.stringify(rejected)}`);
        assert((await snapshot('canvas')).pending === before.pending, 'Rejected profile mutated queued state');
        const beforeFailedAllocation = await pixels([[35,32]]);
        await page.evaluate(() => {
            window.blendGetContext = HTMLCanvasElement.prototype.getContext;
            HTMLCanvasElement.prototype.getContext = function (...args) {
                return this.isConnected ? window.blendGetContext.apply(this, args) : null;
            };
        });
        try {
            await call('blend_mode', 2);
            try { await call('blend_flush'); }
            catch (error) { assert(String(error).includes('blend scratch'), String(error)); }
        } finally {
            await page.evaluate(() => { HTMLCanvasElement.prototype.getContext = window.blendGetContext; delete window.blendGetContext; });
        }
        const afterFailedAllocation = await pixels([[35,32]]);
        assert(near(afterFailedAllocation.samples[0].canvas, beforeFailedAllocation.samples[0].canvas, 0), 'Failed scratch allocation modified visible Canvas pixels');

        await call('blend_phase', 2);
        await flush();
        assert(await page.locator('#blend-svg g[style*="mix-blend-mode"]').count() === 0, 'Plain layout replacement did not clear modes');
        assert((await snapshot('canvas')).active && pageErrors.length === 0, `Unexpected blend fixture page errors ${pageErrors.join('; ')}`);
        return { passed: true, initial, static: staticResult, allModes, transparent, external, zoom, resize, relayout, rejected, retained: true, allocationAtomic: true, pageErrors };
    } finally {
        await page.evaluate(display => { window.hostFixture.blend_stop(); document.querySelector('#blend-fixture-row')?.remove(); document.querySelector('.panels').style.display = display; delete window.blendKept; }, panels);
        page.off('pageerror', onError);
        assert(await page.locator('body *').count() === baseline, 'Stopped blend fixture leaked owned DOM');
    }
})
