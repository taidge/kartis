(async function verifyBrowserPlotWidgets(page) {
    const assert = (condition, message) => { if (!condition) throw Error(message); };
    const same = (actual, expected, message) => assert(JSON.stringify(actual) === JSON.stringify(expected),
        `${message}: got ${JSON.stringify(actual)}, expected ${JSON.stringify(expected)}`);
    const near = (actual, expected, message) => assert(Math.abs(actual - expected) < 0.1,
        `${message}: got ${actual}, expected ${expected}`);
    const reports = [];
    const pageErrors = [];
    const recordError = error => pageErrors.push(error.message);
    page.on('pageerror', recordError);
    const id = 'browser-measured-slot-fixture';
    const otherId = `${id}-second`;
    const host = page.locator(`#${id}`);
    const surface = host.locator('.kartis-surface');
    const alpha = host.locator('.browser-slot-alpha');
    const beta = host.locator('.browser-slot-beta');
    const legend = host.getByRole('button', {name: /^(Show|Hide) alpha$/});
    const flush = () => page.evaluate(id => hostFixture.browser_plot_widgets_flush(id), id);
    const phase = value => page.evaluate(({id, value}) => hostFixture.browser_plot_widgets_phase(id, value), {id, value});
    const status = () => page.evaluate(id => JSON.parse(hostFixture.browser_plot_widgets_snapshot(id)), id);
    const frame = () => page.evaluate(() => new Promise(resolve => requestAnimationFrame(resolve)));
    const box = () => surface.boundingBox();
    const clickWorld = async (x, y, mouse = 'click') => {
        const b = await box();
        const {state} = await status();
        await page.mouse[mouse](b.x + x * b.width / state.width, b.y + y * b.height / state.height);
    };
    const currentLayout = () => host.evaluate(root => {
        const surface = root.querySelector('.kartis-surface').getBoundingClientRect();
        const button = root.querySelector('.browser-slot-alpha');
        const clip = button.closest('.kartis-plot-widget-clip');
        const relative = element => {
            const b = element.getBoundingClientRect();
            return [b.x - surface.x, b.y - surface.y, b.width, b.height];
        };
        return {
            plot: [surface.x, surface.y, surface.width, surface.height],
            rect: relative(button), clip: relative(clip), inert: clip.inert,
            mask: getComputedStyle(clip).clipPath,
        };
    });
    const layoutMatches = async (anchor, state, camera = true) => {
        const geometry = await currentLayout();
        const [scale, tx, ty] = camera ? state.view : [1, 0, 0];
        const x = 20 + (anchor[0] - 24 - 20) * scale + tx;
        const y = 20 + (anchor[1] - 10 - 20) * scale + ty;
        const sx = geometry.plot[2] / state.width;
        const sy = geometry.plot[3] / state.height;
        const expected = [x * sx, y * sy, 48 * scale * sx, 20 * scale * sy];
        for (let i = 0; i < 4; i++) near(geometry.rect[i], expected[i], `World/CSS slot mapping axis ${i}`);
        return geometry;
    };
    const remember = () => host.evaluate(root => {
        window.slotWitness = [root.querySelector('.browser-slot-alpha'), root.querySelector('.browser-slot-beta')];
    });
    const identities = () => host.evaluate(root => [
        root.querySelector('.browser-slot-alpha') === window.slotWitness[0],
        root.querySelector('.browser-slot-beta') === window.slotWitness[1],
    ]);
    const paint = () => surface.evaluate(element =>
        element.querySelector('canvas')?.toDataURL() ?? element.querySelector('svg').outerHTML);
    const samplePixels = async (surface, scene, points) => {
        const bytes = (await page.screenshot()).toString('base64');
        return page.evaluate(async ({bytes, surface, scene, points}) => {
            const image = new Image();
            image.src = `data:image/png;base64,${bytes}`;
            await image.decode();
            const scratch = Object.assign(document.createElement('canvas'), {width:image.width,height:image.height});
            const ctx = scratch.getContext('2d');
            ctx.drawImage(image, 0, 0);
            return Object.fromEntries(Object.entries(points).map(([name,[x,y]]) => [name, [...ctx.getImageData(
                Math.floor((surface[0] + x * surface[2] / scene[0]) * image.width / innerWidth),
                Math.floor((surface[1] + y * surface[3] / scene[1]) * image.height / innerHeight), 1, 1,
            ).data]]));
        }, {bytes, surface, scene, points});
    };
    const mount = async (canvas, relayout) => {
        await host.evaluate(element => { element.style.width = '560px'; });
        await page.evaluate(({id, canvas, relayout}) => hostFixture.browser_plot_widgets_mount(id, canvas, relayout),
            {id, canvas, relayout});
        await flush();
        await alpha.waitFor({state: 'visible'});
        await frame();
    };
    const stop = async () => {
        await page.evaluate(id => hostFixture.browser_plot_widgets_stop(id), id);
        assert(await host.locator('*').count() === 0, 'Browser chart Stop retained overlay/paint DOM');
        assert(await host.getAttribute('data-kartis-browser') === null, 'Browser chart Stop retained the root claim');
    };

    try {
        await page.evaluate(id => {
            const root = document.createElement('section');
            root.id = id;
            root.style.cssText = 'position:fixed;left:18px;top:18px;z-index:9999;width:560px;background:#fff';
            const styles = document.createElement('style');
            styles.id = `${id}-styles`;
            styles.textContent = `#${id} .kartis-plot-widget-slot button{box-sizing:border-box;padding:0;border:0}` +
                `#${id} .browser-slot-alpha{background:#136251;color:#fff}` +
                `#${id} .browser-slot-beta{background:#96451a;color:#fff}`;
            document.head.append(styles);
            document.body.prepend(root);
        }, id);
        for (const canvas of [false, true]) {
            const renderer = canvas ? 'canvas' : 'svg';
            await mount(canvas, false);
            const initial = await layoutMatches([100, 75], (await status()).state);
            same((await status()).state.view, [1, 0, 0], `${renderer} initial camera`);
            assert(await alpha.textContent() === 'alpha 10 r0 ×0', 'Widget read interpolatable paint value instead of typed business value');
            assert(await alpha.evaluate(button => !button.closest('[role=img]')), 'Widget was flattened into the surface image role');
            await remember();

            // Plot → Widget → plot uses actual browser pointer ownership.
            await clickWorld(284, 130, 'move');
            await flush();
            same((await status()).state.hovered, ['beta'], `${renderer} uncovered plot hover`);
            await alpha.hover();
            await flush();
            same((await status()).state.hovered, [], `${renderer} overlay pointer retires plot hover`);
            await alpha.click();
            await alpha.focus();
            await page.keyboard.press('Space');
            await page.waitForFunction(() => document.querySelector('.browser-slot-alpha')?.textContent === 'alpha 10 r0 ×2');
            await flush();
            assert((await status()).selections === 0, 'Widget native click/keyboard bubbled into plot selection');
            await page.mouse.wheel(0, -140);
            await flush();
            same((await status()).state.view, [1, 0, 0], `${renderer} overlay wheel was consumed by plot`);
            await clickWorld(284, 130);
            await flush();
            assert((await status()).state.selected === 'beta', 'Uncovered plot could not select real datum');

            await clickWorld(106, 107, 'move');
            await page.mouse.wheel(0, -200);
            await flush();
            const zoomed = (await status()).state;
            assert(zoomed.view[0] > 1.1, 'Actual plot wheel did not reach camera');
            const zoomGeometry = await layoutMatches([100, 75], zoomed);
            await host.evaluate(element => { element.style.width = '660px'; });
            await frame();
            await flush();
            const responsive = await layoutMatches([100, 75], (await status()).state);
            near(responsive.plot[2], 660, 'Responsive surface actual width');
            near(responsive.rect[3], zoomGeometry.rect[3], 'Nonuniform resize changed CSS Y scale');
            same(await identities(), [true, true], 'Camera zoom or resize replaced keyed Widget');
            await surface.focus();
            await page.keyboard.press('0');
            await flush();
            same((await status()).state.view, [1, 0, 0], 'Surface keyboard reset no longer works');

            // First animation frame uses the old physical anchor and the new
            // accepted business value. Subsequent frames move that same node.
            const firstMotion = await host.evaluate((root, id) => {
                hostFixture.browser_plot_widgets_phase(id, 1);
                hostFixture.browser_plot_widgets_flush(id);
                return {
                    state: JSON.parse(hostFixture.browser_plot_widgets_snapshot(id)).state,
                    label: root.querySelector('.browser-slot-alpha').textContent,
                };
            }, id);
            assert(firstMotion.state.animating && firstMotion.label === 'alpha 42 r0 ×2', 'Typed animation did not publish the current value in-place');
            await page.waitForFunction(id => !JSON.parse(hostFixture.browser_plot_widgets_snapshot(id)).state.animating, id);
            const animated = await layoutMatches([125, 75], (await status()).state);
            same(await identities(), [true, true], 'Animation replaced a keyed child');
            await legend.click();
            await frame();
            await flush();
            await alpha.waitFor({state: 'hidden'});
            assert(await beta.isVisible(), 'Legend filtered an unrelated Widget');
            await legend.click();
            await frame();
            await flush();
            await alpha.waitFor({state: 'visible'});
            same(await identities(), [true, true], 'Legend hide/show remounted Widget');

            await alpha.focus();
            await phase(2);
            await flush();
            await frame();
            const partial = await currentLayout();
            near(partial.rect[0], 12 * 660 / 400, 'Original full child offset after clipping');
            near(partial.clip[0], 20 * 660 / 400, 'Fixed plot clip position');
            near(partial.clip[2], 40 * 660 / 400, 'Actual visible clipped width');
            assert(partial.inert && await alpha.isVisible(), 'Partial slot is not visibly inert');
            assert(await alpha.evaluate(button => {
                button.focus();
                const bounds = button.getBoundingClientRect();
                const target = document.elementFromPoint(bounds.x + bounds.width / 2, bounds.y + 5);
                return document.activeElement !== button && target !== button;
            }), 'Clipped visible Widget could still receive mouse/focus');
            const clipPixels = await samplePixels(partial.plot, [400,220], {outside:[16,67],visible:[58,67]});
            same(clipPixels, {outside: [255, 255, 255, 255], visible: [19, 98, 81, 255]}, `${renderer} partial clip pixels`);

            for (const rejected of [3, 4, 5]) { await phase(rejected); await flush(); }
            same(await identities(), [true, true], 'Factory/build rejection or rollback removed accepted sibling');
            assert(await alpha.textContent() === 'alpha 42 r0 ×2', 'Failed factory reset local state');
            let accepted = await status();
            assert(accepted.factories === 4 && accepted.cleanups === 1 && accepted.reentrantReads === 2 &&
                accepted.errors.length === 2 && accepted.errors[0].includes('factory refused') && accepted.errors[1].includes('child refused'),
            'Factory or child error was not locally isolated and delivered');
            await phase(6);
            await flush();
            same(await identities(), [false, true], 'Accepted revision did not replace only the target Widget');
            assert(await alpha.textContent() === 'alpha 42 r3 ×0', 'Replacement local state was stale');
            const acceptedPaint = await paint();
            const acceptedBounds = await currentLayout();
            const bad = await page.evaluate(id => {
                try { hostFixture.browser_plot_widgets_phase(id, 7); }
                catch (error) { return String(error); }
                return null;
            }, id);
            assert(bad?.includes('duplicate plot Widget datum'), 'Duplicate combined candidate was queued');
            assert((await status()).state.width === 400, 'Bad combined layout replaced accepted Scene');
            assert(await paint() === acceptedPaint && await alpha.textContent() === 'alpha 42 r3 ×0', 'Rejected combined layout changed paint or data');
            same(await currentLayout(), acceptedBounds, 'Rejected combined layout changed Widget geometry');
            await remember();

            await phase(8);
            await flush();
            accepted = await status();
            const masked = await layoutMatches([160,75], accepted.state);
            assert(masked.mask !== 'none' && masked.inert && accepted.errors.length === 2, 'Partial mask not installed as inert');
            assert(await alpha.textContent() === 'alpha 99 r3 ×0', 'Masked slot did not publish current value');
            same(await identities(), [true,true], 'Mask replaced a keyed Widget');
            const maskPoints = {ink:[144,69],hole:[160,75],outside:[137,75]};
            const expectedMaskPixels = {ink:[19,98,81,255],hole:[255,255,255,255],outside:[255,255,255,255]};
            const maskPixels = await samplePixels(masked.plot, [accepted.state.width, accepted.state.height], maskPoints);
            same(maskPixels, expectedMaskPixels, `${renderer} actual compound mask pixels`);
            assert(await alpha.evaluate(button => {
                button.focus();
                const rect = button.getBoundingClientRect();
                const target = document.elementFromPoint(rect.x + rect.width / 5, rect.y + 5);
                return document.activeElement !== button && target !== button;
            }), 'Partial path mask still accepted Widget focus or input');

            const isolation = await page.evaluate(({id,otherId,canvas}) => {
                const second = document.createElement('section');
                second.id = otherId;
                second.style.cssText = 'position:fixed;left:900px;top:18px;width:400px';
                document.body.append(second);
                const clip = root => root.querySelector('.browser-slot-alpha').closest('.kartis-plot-widget-clip');
                const first = clip(document.getElementById(id));
                const before = getComputedStyle(first).clipPath;
                try {
                    hostFixture.browser_plot_widgets_mount(otherId, canvas, false);
                    hostFixture.browser_plot_widgets_phase(otherId, 8);
                    hostFixture.browser_plot_widgets_flush(otherId);
                    const distinct = getComputedStyle(clip(second)).clipPath !== before &&
                        getComputedStyle(clip(second)).clipPath !== 'none';
                    hostFixture.browser_plot_widgets_stop(otherId);
                    return distinct && getComputedStyle(first).clipPath === before && !second.hasChildNodes();
                } finally {
                    hostFixture.browser_plot_widgets_stop(otherId);
                    second.remove();
                }
            }, {id,otherId,canvas});
            assert(isolation, 'Stopping a second masked instance changed the first mask');

            await clickWorld(190, 98, 'move');
            await page.mouse.wheel(0, -140);
            await flush();
            const maskZoom = (await status()).state;
            const maskZoomBox = await layoutMatches([160,75], maskZoom);
            const [maskScale, maskX, maskY] = maskZoom.view;
            assert(maskScale > 1 && maskZoomBox.inert, 'Camera did not advance an inert masked slot');
            const zoomedPoints = Object.fromEntries(Object.entries(maskPoints).map(([name,[x,y]]) =>
                [name,[20+(x-20)*maskScale+maskX,20+(y-20)*maskScale+maskY]]));
            same(await samplePixels(maskZoomBox.plot, [maskZoom.width,maskZoom.height], zoomedPoints),
                expectedMaskPixels, `${renderer} camera path mask pixels`);
            await surface.focus();
            await page.keyboard.press('0');
            await flush();
            await phase(12);
            await flush();
            const enclosed = await currentLayout();
            assert(!enclosed.inert && enclosed.mask === 'none', 'Enclosing mask prevented input');
            const selections = (await status()).selections;
            await alpha.click();
            await page.waitForFunction(id => document.getElementById(id)
                ?.querySelector('.browser-slot-alpha')?.textContent === 'alpha 99 r3 ×1', id);
            await flush();
            const clicked = await alpha.textContent();
            const afterClick = (await status()).selections;
            assert(clicked === 'alpha 99 r3 ×1' && afterClick === selections,
                `Enclosed mask lost native Widget click ownership: ${clicked}, selections ${selections} → ${afterClick}`);
            await phase(13);
            await flush();
            assert(await alpha.count() === 1 && !await alpha.isVisible() && (await currentLayout()).inert,
                'Slot entirely inside an even-odd hole was not retired from visibility');
            same(await identities(), [true,true], 'Mask removal or hole changed Widget identity');
            await phase(9);
            await flush();
            assert(await alpha.count() === 0 && (await identities())[1], 'Combined Scene/slots removal lost surviving row');
            await phase(10);
            await page.waitForFunction(id => !JSON.parse(hostFixture.browser_plot_widgets_snapshot(id)).active, id);
            const retired = await status();
            assert(!retired.active && retired.factories === 6 && retired.cleanups === 4 && retired.reentrantReads === 4,
                `Stop during factory left resources or held Engine borrow: ${JSON.stringify(retired)}`);
            await stop();

            await mount(canvas, true);
            const original = await status();
            assert(original.state.domains, 'Domain relayout fixture did not install its viewport');
            const domainAnchor = state => 20 + (20 / 9 - state.domains[0]) / (state.domains[1] - state.domains[0]) * (state.width - 40);
            await layoutMatches([domainAnchor(original.state), 75], original.state, false);
            await clickWorld(215, 110, 'move');
            await page.mouse.wheel(0, -160);
            await flush();
            const relaid = await status();
            assert(relaid.state.view[0] > 1.1 && relaid.state.domains[1] < original.state.domains[1], 'Actual wheel did not commit narrower data domain');
            const domainBox = await layoutMatches([domainAnchor(relaid.state), 75], relaid.state, false);
            await phase(11);
            const domainPaint = await paint();
            await clickWorld(215, 110, 'move');
            await page.mouse.wheel(0, -80);
            await flush();
            const failed = await status();
            same(failed.state.domains, relaid.state.domains, 'Rejected relayout advanced accepted domains');
            same(failed.state.view, relaid.state.view, 'Rejected relayout advanced virtual input camera');
            same(await currentLayout(), domainBox, 'Rejected relayout advanced Widget geometry');
            assert(await paint() === domainPaint && failed.errors.at(-1).includes('clip contours must be explicitly closed'), 'Rejected relayout advanced plot or omitted error');
            await phase(11);
            await host.evaluate(element => { element.style.width = '620px'; });
            await page.waitForFunction(id => JSON.parse(hostFixture.browser_plot_widgets_snapshot(id)).state.width === 620, id);
            const resized = await status();
            assert(resized.state.width === 620, 'Actual ResizeObserver did not rebuild the domain layout');
            same(resized.state.domains, relaid.state.domains, 'Responsive domain relayout reset accepted domain');
            const domainResize = await layoutMatches([domainAnchor(resized.state), 75], resized.state, false);
            await stop();
            reports.push({renderer, initial, zoomed: zoomGeometry, responsive, animated, partial, clipPixels, maskPixels,
                maskIsolation:isolation, fullMaskInput:true, holeRetained:true,
                domainBox, domainResize, atomicRejection: true, reentrantStop: true});
        }
        same(pageErrors, [], 'Unhandled browser errors');
        return {passed: true, renderers: reports};
    } finally {
        await page.evaluate(id => {
            hostFixture.browser_plot_widgets_stop(id);
            document.getElementById(id)?.remove();
            document.getElementById(`${id}-styles`)?.remove();
            delete window.slotWitness;
        }, id);
        page.off('pageerror', recordError);
    }
})
