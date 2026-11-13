import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {readFile, writeFile} from 'node:fs/promises';
import {join, resolve} from 'node:path';
import {fileURLToPath} from 'node:url';
import {chromium, firefox, webkit} from 'playwright';

const root = fileURLToPath(new URL('../', import.meta.url));
const fixture = 'examples/browser-widget';
const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');
const sources = [
    'src/lib.rs', 'src/widget.rs', 'src/widget/plot_widgets.rs', 'tests/plot_widgets.rs',
    `${fixture}/src/lib.rs`, `${fixture}/src/plot_widgets.rs`, `${fixture}/plot-widgets.html`,
    'scripts/plot-widgets-browser-acceptance.mjs',
];

export async function verifyPlotWidgets(page, url, {stagedPackage} = {}) {
    const pageErrors = [];
    const recordError = error => pageErrors.push(error.message);
    page.on('pageerror', recordError);
    try {
        const result = await checkPlotWidgets(page, url, stagedPackage);
        assert.deepEqual(pageErrors, []);
        return result;
    } finally {
        page.off('pageerror', recordError);
    }
}

async function checkPlotWidgets(page, url, stagedPackage) {
    if (stagedPackage) {
        // Serve exact local fixture bytes while leaving canonical pkg intact.
        const html = await readFile(join(root, fixture, 'plot-widgets.html'));
        await page.route(url, route => route.fulfill({body: html, contentType: 'text/html'}));
        await page.route('**/pkg/widget.js', route => route.fulfill({
            path: join(stagedPackage, 'widget.js'), contentType: 'text/javascript',
        }));
        await page.route('**/pkg/widget_bg.wasm', route => route.fulfill({
            path: join(stagedPackage, 'widget_bg.wasm'), contentType: 'application/wasm',
        }));
    }
    const wasmResponse = page.waitForResponse(response => response.url().endsWith('/pkg/widget_bg.wasm'));
    await page.goto(url);
    await page.waitForFunction(() => Boolean(window.plotWidgets));
    const wasmSha256 = sha256(await (await wasmResponse).body());
    const alpha = page.locator('.plot-alpha');
    const beta = page.locator('.plot-beta');
    const app = page.locator('#plot-widgets-app');
    const legend = app.getByRole('button', {name: /^(Hide|Show) alpha$/});
    const box = page.locator('.kartis-plot-widgets');
    const phase = id => page.evaluate(id => window.plotWidgets.plot_widgets_phase(id), id);
    const report = () => page.evaluate(() => window.plotWidgets.plot_widgets_report());
    const label = locator => locator.textContent();
    const flushFrame = () => page.evaluate(() => new Promise(resolve => requestAnimationFrame(resolve)));
    const waitLabel = expected => page.waitForFunction(expected =>
        document.querySelector('.plot-alpha')?.textContent === expected, expected);
    const measurements = () => page.evaluate(() => {
        const root = document.querySelector('.kartis-plot-widgets').getBoundingClientRect();
        const a = document.querySelector('.plot-alpha');
        const clip = a.closest('.kartis-plot-widget-clip');
        const relative = element => {
            const rect = element.getBoundingClientRect();
            return [rect.x - root.x, rect.y - root.y, rect.width, rect.height];
        };
        return {root: [root.x, root.y, root.width, root.height], alpha: relative(a), clip: relative(clip), inert: clip.inert};
    });
    const sameWidgets = () => page.evaluate(() => window.plotAlpha === document.querySelector('.plot-alpha') &&
        window.plotBeta === document.querySelector('.plot-beta'));
    const assertReport = async (factories, cleanups, errors = []) => {
        assert.deepEqual((await report()).trim().split('\n').slice(0, 3), [
            `factory ${factories}`, `cleanup ${cleanups}`, 'selection 0',
        ]);
        const actual = (await report()).trim().split('\n').slice(3);
        assert.equal(actual.length, errors.length);
        for (let i = 0; i < errors.length; i++) assert.match(actual[i], errors[i]);
    };

    const initial = await measurements();
    assert.deepEqual(initial.root.slice(2), [260, 155]);
    assert.deepEqual(initial.alpha, [27, 51, 92, 28]);
    assert.deepEqual(initial.clip, initial.alpha);
    assert.equal(initial.inert, false);
    assert.equal(await label(alpha), 'alpha 10 r0 ×0');
    assert.equal(await label(beta), 'beta 20 r0 ×0');
    assert.equal(await page.evaluate(() => {
        window.plotAlpha = document.querySelector('.plot-alpha');
        window.plotBeta = document.querySelector('.plot-beta');
        const b = window.plotAlpha.getBoundingClientRect();
        return document.elementFromPoint(b.x + 7, b.y + 7) === window.plotAlpha;
    }), true, 'The actual child receives pointer hits inside its measured slot');
    await alpha.click();
    await waitLabel('alpha 10 r0 ×1');
    await alpha.focus();
    await page.keyboard.press('Space');
    await waitLabel('alpha 10 r0 ×2');
    await assertReport(2, 0);

    await phase(1);
    assert.equal(await sameWidgets(), true);
    assert.deepEqual(await app.locator('.kartis-plot-widget-slot button').evaluateAll(nodes =>
        nodes.map(node => node.classList.contains('plot-alpha') ? 'alpha' : 'beta')), ['beta', 'alpha']);
    assert.equal(await label(alpha), 'alpha 42 r0 ×2');
    const moved = await measurements();
    assert.deepEqual(moved.root.slice(2), [320, 155]);
    assert.deepEqual(moved.alpha, [51, 64, 92, 28]);
    await assertReport(2, 0);

    await legend.click();
    await alpha.waitFor({state: 'hidden'});
    assert.equal(await alpha.isVisible(), false);
    assert.equal(await alpha.evaluate(node => node.closest('.kartis-plot-widget-clip').inert), true);
    assert.equal(await beta.isVisible(), true);
    await legend.click();
    await alpha.waitFor({state: 'visible'});
    assert.equal(await alpha.isVisible(), true);
    assert.equal(await sameWidgets(), true);
    assert.equal(await label(alpha), 'alpha 42 r0 ×2');

    await alpha.focus();
    await phase(2);
    await flushFrame();
    assert.equal(await sameWidgets(), true);
    const partial = await measurements();
    assert.deepEqual(partial.alpha, moved.alpha);
    assert.deepEqual(partial.clip, [70, 64, 73, 28]);
    assert.equal(partial.inert, true);
    assert.equal(await alpha.isVisible(), true, 'Partial clipping leaves visible child ink');
    assert.equal(await alpha.evaluate(node => {
        node.focus();
        const rect = node.getBoundingClientRect();
        const target = document.elementFromPoint(rect.x + 25, rect.y + 6);
        return document.activeElement !== node && !node.contains(target);
    }), true, 'Native inert prevents focus and hit in the visible part of a clipped child');
    await page.mouse.click(partial.root[0] + 76, partial.root[1] + 70);
    assert.equal(await label(alpha), 'alpha 42 r0 ×2');
    const screenshot = await page.screenshot();
    const clipPixels = await page.evaluate(async ({bytes, origin}) => {
        const image = new Image();
        image.src = `data:image/png;base64,${bytes}`;
        await image.decode();
        const canvas = document.createElement('canvas');
        canvas.width = image.width;
        canvas.height = image.height;
        const ctx = canvas.getContext('2d');
        ctx.drawImage(image, 0, 0);
        const sample = x => [...ctx.getImageData(
            Math.floor((origin[0] + x) * image.width / innerWidth),
            Math.floor((origin[1] + 70) * image.height / innerHeight), 1, 1,
        ).data];
        return {clippedOut: sample(56), visible: sample(76)};
    }, {bytes: screenshot.toString('base64'), origin: partial.root});
    assert.deepEqual(clipPixels.clippedOut, [255, 255, 255, 255]);
    assert.deepEqual(clipPixels.visible, [19, 98, 81, 255]);

    const factoryError = /fixture slot factory refused revision 1/;
    const buildError = /fixture slot child build refused revision 2/;
    const duplicateError = /duplicate plot Widget datum/;
    await phase(3);
    assert.equal(await sameWidgets(), true);
    await assertReport(3, 0, [factoryError]);
    await phase(3);
    await assertReport(3, 0, [factoryError]);
    await phase(4);
    assert.equal(await sameWidgets(), true);
    assert.equal(await alpha.count(), 1);
    assert.equal(await label(alpha), 'alpha 42 r0 ×2');
    await assertReport(4, 1, [factoryError, buildError]);
    await phase(9);
    assert.equal(await sameWidgets(), true);
    await assertReport(4, 1, [factoryError, buildError]);
    await alpha.click();
    await waitLabel('alpha 42 r0 ×3');

    await phase(5);
    assert.equal(await sameWidgets(), false);
    assert.equal(await beta.evaluate(node => window.plotBeta === node), true);
    assert.equal(await label(alpha), 'alpha 42 r3 ×0');
    await alpha.focus();
    await phase(6);
    assert.deepEqual((await measurements()).root.slice(2), [320, 155]);
    assert.equal(await label(alpha), 'alpha 42 r3 ×0');
    await assertReport(5, 2, [factoryError, buildError, duplicateError]);
    await phase(7);
    assert.deepEqual((await measurements()).root.slice(2), [350, 155]);
    assert.equal(await alpha.evaluate(node => node === document.activeElement), true);
    await page.keyboard.press('Enter');
    await waitLabel('alpha 42 r3 ×1');
    await phase(8);
    assert.equal(await alpha.count(), 0);
    assert.equal(await beta.evaluate(node => window.plotBeta === node), true);
    await assertReport(5, 3, [factoryError, buildError, duplicateError]);

    await page.evaluate(() => window.plotWidgets.plot_widgets_stop());
    assert.equal(await box.count(), 0);
    assert.equal(await app.locator('*').count(), 0);
    assert.equal(await app.getAttribute('data-gly-app'), null);
    await page.evaluate(() => window.plotAlpha.click());
    await assertReport(5, 4, [factoryError, buildError, duplicateError]);
    await page.evaluate(() => window.plotWidgets.plot_widgets_mount());
    assert.equal(await label(alpha), 'alpha 10 r0 ×0');
    await page.evaluate(() => window.plotWidgets.plot_widgets_stop());
    await assertReport(2, 2);
    return {passed: true, wasmSha256, initial, moved, partial, clipPixels, lifecycle: true, pointer: true, keyboard: true};
}

async function main() {
    const args = process.argv.slice(2);
    const option = (name, fallback) => {
        const index = args.indexOf(name);
        return index === -1 ? fallback : args[index + 1];
    };
    const url = option('--url', 'http://127.0.0.1:8813/plot-widgets.html');
    const stagedPackage = option('--package', null);
    const output = option('--output', 'target/gallery/plot-widgets-browser-report.json');
    const requested = option('--browsers', 'chrome').split(',');
    const engines = {chrome: chromium, edge: chromium, firefox, webkit};
    const results = {startedAt: new Date().toISOString(), url, browsers: [], sourceStamp: {}};
    for (const source of sources) results.sourceStamp[source] = sha256(await readFile(join(root, source)));
    for (const name of requested) {
        if (!Object.hasOwn(engines, name)) throw Error(`Unknown browser ${name}`);
        const entry = {name};
        let browser;
        try {
            const channel = name === 'chrome' ? 'chrome' : name === 'edge' ? 'msedge' : undefined;
            const executablePath = option(`--${name}-path`, undefined);
            browser = await engines[name].launch({headless: true, channel, executablePath});
            entry.version = browser.version();
            const page = await browser.newPage({viewport: {width: 1120, height: 750}, deviceScaleFactor: 2});
            page.setDefaultTimeout(10_000);
            entry.result = await verifyPlotWidgets(page, url, {stagedPackage});
            entry.passed = true;
            console.log(`${name} ${entry.version}: passed`);
        } catch (error) {
            entry.passed = false;
            entry.error = error.stack || String(error);
            console.error(`${name} ${entry.version ?? ''}: ${entry.error}`);
        } finally {
            if (browser) await browser.close();
        }
        results.browsers.push(entry);
    }
    results.finishedAt = new Date().toISOString();
    await writeFile(output, JSON.stringify(results, null, 2) + '\n');
    console.log(`Report written to ${output}`);
    process.exitCode = results.browsers.every(entry => entry.passed) ? 0 : 1;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
    await main();
}
