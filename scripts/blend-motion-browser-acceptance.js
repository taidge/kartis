(async function verifyBlendMotion(page) {
    const assert = (ok, message) => { if (!ok) throw Error(message); };
    const call = (name, ...args) => page.evaluate(([name,args]) => window.hostFixture[`blend_motion_${name}`](...args), [name,args]);
    const snapshot = async name => JSON.parse(await call('snapshot', name));
    const renderers = ['svg', 'canvas'];
    const pageErrors = [];
    const listener = error => pageErrors.push(error.message);
    page.on('pageerror', listener);
    const display = await page.evaluate(() => {
        window.hostFixture.blend_motion_stop();
        const panels = document.querySelector('.panels');
        const previous = panels.style.display;
        panels.style.display = 'none';
        const originalRequest = window.requestAnimationFrame;
        const originalCancel = window.cancelAnimationFrame;
        let time = performance.now(), id = 0;
        const frames = new Map();
        window.requestAnimationFrame = callback => { frames.set(++id, callback); return id; };
        window.cancelAnimationFrame = token => { frames.delete(token); };
        window.blendMotionClock = {
            advance(delta) {
                time += delta;
                const pending = [...frames.values()];
                frames.clear();
                pending.forEach(callback => callback(time));
                return pending.length;
            },
            restore() {
                window.requestAnimationFrame = originalRequest;
                window.cancelAnimationFrame = originalCancel;
            },
        };
        return previous;
    });
    const baseline = await page.locator('body *').count();
    const advance = delta => page.evaluate(delta => window.blendMotionClock.advance(delta), delta);
    async function mount(kind, reduced) {
        await call('mount', kind, reduced);
        await page.evaluate(() => {
            // Keep the live SVG compositor and Canvas presentation aligned to
            // the viewport's device pixel origin, independent of host content.
            const row = document.querySelector('#blend-motion-row');
            Object.assign(row.style, {position:'fixed',left:'0px',top:'0px',zIndex:'1000'});
        });
    }
    async function settle() {
        for (let i=0; i<40; i++) {
            await advance(100);
            const states = await Promise.all(renderers.map(snapshot));
            if (states.every(state => !state.animating)) return;
        }
        throw Error('Artistic blend springs did not settle');
    }
    const drawn = () => page.evaluate(() => [...document.querySelectorAll('#blend-motion-svg svg path[data-id]')]
        .filter(path => path.getAttribute('data-id') !== 'budget')
        .map(path => ({
            id: path.getAttribute('data-id'),
            x: path.getBBox().x,
            mode: path.closest('g[style*="mix-blend-mode"]')?.style.mixBlendMode ?? 'normal',
            alpha: +path.getAttribute('opacity'),
        })));
    const base = [160,120,80,255];
    const source = [200,100,50,255];
    const near = (a, b, tolerance=2) => a.every((channel, i) => Math.abs(channel-b[i]) <= tolerance);
    function expected(layers) {
        const color = [...base];
        for (const {mode,alpha} of layers) {
            for (let i=0;i<3;i++) {
                const back = color[i], front = source[i];
                let blend;
                switch (mode) {
                    case 'normal': blend = front; break;
                    case 'multiply': blend = back * front / 255; break;
                    case 'screen': blend = 255 - (255-back)*(255-front)/255; break;
                    case 'difference': blend = Math.abs(back-front); break;
                    default: throw Error(`Unrecognized mode in rendered SVG ${mode}`);
                }
                color[i] = back + alpha*(blend-back);
            }
        }
        return color;
    }
    async function readPixels(live = false) {
        const shot = live ? (await page.screenshot({scale:'device'})).toString('base64') : null;
        return page.evaluate(async shot => {
            const actual = document.querySelector('#blend-motion-canvas canvas');
            const svg = document.querySelector('#blend-motion-svg svg').cloneNode(true);
            const logical = [svg.viewBox.baseVal.width, svg.viewBox.baseVal.height];
            const load = async url => {
                const image = new Image();
                image.src = url;
                await image.decode();
                return image;
            };
            const rgba = (image, regions) => {
                const canvas = Object.assign(document.createElement('canvas'), {width:image.width,height:image.height});
                const context = canvas.getContext('2d');
                context.drawImage(image,0,0);
                return regions.map(region => context.getImageData(...region).data);
            };
            const width = actual.width, height = actual.height;
            let a, b, capture;
            if (shot) {
                const image = await load('data:image/png;base64,'+shot);
                const surfaces = ['svg','canvas'].map(name => {
                    const bounds = document.querySelector(`#blend-motion-${name} .kartis-surface`).getBoundingClientRect();
                    const box = [bounds.left,bounds.top,bounds.width,bounds.height];
                    const crop = box.map(value => value*devicePixelRatio);
                    const [x,y,w,h] = crop;
                    if (!crop.every(Number.isInteger) || w!==width || h!==height ||
                        x<0 || y<0 || x+w>image.width || y+h>image.height) {
                        throw Error(`Live blend surface is outside its shared device pixel grid ${JSON.stringify({name,box,crop,width,height})}`);
                    }
                    return {name,box,crop};
                });
                if (surfaces[0].crop[0] !== 0 || surfaces[0].crop[1] !== 0 || surfaces[1].crop[1] !== 0) {
                    throw Error('Live blend fixture did not align to the viewport origin');
                }
                [a,b] = rgba(image,surfaces.map(surface => surface.crop));
                capture = {dpr:devicePixelRatio,screenshot:[image.width,image.height],surfaces};
            } else {
                svg.setAttribute('width', String(width));
                svg.setAttribute('height', String(height));
                [a] = rgba(await load('data:image/svg+xml;charset=utf-8,' + encodeURIComponent(new XMLSerializer().serializeToString(svg))),[[0,0,width,height]]);
                b = actual.getContext('2d').getImageData(0,0,width,height).data;
            }
            let total = 0;
            for (let i=0;i<a.length;i++) total += Math.abs(a[i]-b[i]);
            const samples = [[34,30],[50,40],[110,70]].map(([x,y]) => {
                const at = 4*(Math.floor(y*height/logical[1])*width + Math.floor(x*width/logical[0]));
                return {svg:[...a.slice(at,at+4)], canvas:[...b.slice(at,at+4)]};
            });
            return {mean:total/a.length, samples, width, height, capture};
        }, shot);
    }
    async function checkPaint(layers, label, live=false) {
        const pixels = await readPixels(live);
        for (const [i, color] of [expected(layers),base,base].entries()) {
            const actual = pixels.samples[i];
            assert(near(actual.svg,color) && near(actual.canvas,color), `${label} pixel ${i}: ${JSON.stringify(actual)} expected ${color}`);
        }
        assert(pixels.mean < 0.5, `${label} SVG/Canvas parity ${pixels.mean}`);
        return pixels;
    }
    async function trustedMove(renderer) {
        const box = await page.locator(`#blend-motion-${renderer} .kartis-surface`).boundingBox();
        await page.mouse.move(box.x+34, box.y+30);
        await advance(0);
    }
    async function waitResize() {
        for (let i=0;i<30;i++) {
            await page.evaluate(() => new Promise(resolve => setTimeout(resolve, 20)));
            await advance(0);
            if ((await snapshot('svg')).width === 150 && (await snapshot('canvas')).width === 150) return;
        }
        throw Error('ResizeObserver did not deliver the new blend dimensions');
    }
    async function canvasData() {
        return page.locator('#blend-motion-canvas canvas').evaluate(canvas => canvas.toDataURL());
    }
    const profiles = [];
    try {
        for (const kind of [0,1]) {
            await mount(kind, false);
            await advance(0);
            const initialLayers = await drawn();
            assert(initialLayers.length === 1 && initialLayers[0].mode === 'multiply', JSON.stringify(initialLayers));
            const initial = await checkPaint(initialLayers, `solver ${kind} initial`);
            await page.evaluate(() => { window.blendMotionPath = document.querySelector('#blend-motion-svg [data-id="paint"]'); });

            await call('phase', 1);
            await advance(0);
            await advance(65);
            const movedLayers = await drawn();
            assert(movedLayers.length === 1 && movedLayers[0].mode === 'multiply' && movedLayers[0].x > 16 && movedLayers[0].x < 24, `Compatible motion ${JSON.stringify(movedLayers)}`);
            assert(await page.evaluate(() => window.blendMotionPath === document.querySelector('#blend-motion-svg [data-id="paint"]')), 'Compatible retarget lost its retained mark');
            const moved = await checkPaint(movedLayers, `solver ${kind} geometry`);
            await settle();

            await call('phase', 2);
            await advance(0);
            const armed = await drawn();
            const outgoing = armed.find(layer => layer.id !== 'paint');
            const entering = armed.find(layer => layer.id === 'paint');
            assert(armed.length === 2 && outgoing?.mode === 'multiply' && outgoing.alpha === 1 &&
                entering?.mode === 'screen' && entering.alpha === 0,
                `Mode replacement did not preserve paint identities ${JSON.stringify(armed)}`);
            await advance(65);
            const mixedLayers = await drawn();
            assert(mixedLayers.every(layer => layer.alpha > 0 && layer.alpha < 1), JSON.stringify(mixedLayers));
            const mixed = await checkPaint(mixedLayers, `solver ${kind} live crossfade`, true);
            for (const renderer of renderers) {
                await trustedMove(renderer);
                const state = await snapshot(renderer);
                assert(state.hovered.join() === 'paint' && state.values?.[0] === 26, `Retired ink leaked into datum ownership ${JSON.stringify(state)}`);
                assert(await page.locator(`#blend-motion-${renderer} .kartis-datum`).count() === 1, 'A paint ghost became a datum control');
            }

            const retainedPixels = await canvasData();
            const rejected = JSON.parse(await call('rejected'));
            assert(rejected.atomic && rejected.error?.includes('blend scratch pixel budget'), `Armed union admission ${JSON.stringify(rejected)}`);
            assert(await canvasData() === retainedPixels, 'Rejected retarget touched the live backing store');

            await call('phase', 3);
            await advance(0);
            const three = await drawn();
            const newLayer = three.find(layer => layer.id === 'paint');
            const retiring = three.filter(layer => layer.id !== 'paint');
            assert(three.length === 3 && newLayer?.mode === 'difference' && newLayer.alpha === 0 &&
                retiring.every((layer, index) => layer.mode === mixedLayers[index].mode && layer.alpha === mixedLayers[index].alpha),
                `Interrupted modes lost accepted paint ${JSON.stringify({mixedLayers,three})}`);
            await advance(60);
            const interrupted = await checkPaint(await drawn(), `solver ${kind} interrupted`);

            // An actual resize replaces active physical ghosts with the
            // responsive Scene and its authored target profile.
            await page.evaluate(() => {
                for (const name of ['svg','canvas']) document.querySelector(`#blend-motion-${name}`).style.width = '150px';
            });
            await waitResize();
            const resizeLayers = await drawn();
            assert(resizeLayers.length === 1 && resizeLayers[0].mode === 'difference' && !(await snapshot('svg')).animating, JSON.stringify(resizeLayers));
            const resized = await checkPaint(resizeLayers, `solver ${kind} resized`);

            await call('phase', 2);
            await advance(0);
            await advance(40);
            await call('viewport');
            await advance(0);
            const viewportLayers = await drawn();
            assert(viewportLayers.length === 1 && viewportLayers[0].mode === 'screen' &&
                viewportLayers[0].x < 24 && (await snapshot('canvas')).relayout && !(await snapshot('canvas')).animating,
                `Viewport relayout lost authored blend or geometry ${JSON.stringify(viewportLayers)}`);
            const relayout = await checkPaint(viewportLayers, `solver ${kind} viewport`);

            await call('phase', 4);
            await advance(0);
            await advance(35);
            const beforeFailure = await canvasData();
            await page.evaluate(() => {
                window.blendMotionContext = HTMLCanvasElement.prototype.getContext;
                window.blendMotionScratchFailed = false;
                HTMLCanvasElement.prototype.getContext = function (...args) {
                    if (!this.isConnected && !window.blendMotionScratchFailed) {
                        window.blendMotionScratchFailed = true;
                        return null;
                    }
                    return window.blendMotionContext.apply(this, args);
                };
            });
            try { await advance(35); }
            finally {
                await page.evaluate(() => {
                    HTMLCanvasElement.prototype.getContext = window.blendMotionContext;
                    delete window.blendMotionContext;
                });
            }
            const frozen = await snapshot('canvas');
            assert(await page.evaluate(() => window.blendMotionScratchFailed) &&
                frozen.errors.length === 1 && frozen.errors[0].includes('blend scratch') &&
                !frozen.animating && await canvasData() === beforeFailure,
                `Scratch failure did not preserve the accepted blended frame ${JSON.stringify(frozen)}`);
            await call('phase', 4);
            await advance(0);
            await settle();
            const settledLayers = await drawn();
            assert(settledLayers.length === 1 && settledLayers[0].mode === 'normal' && settledLayers[0].alpha === 1, JSON.stringify(settledLayers));
            const normal = await checkPaint(settledLayers, `solver ${kind} normal settled`);
            await call('stop');
            assert(await page.locator('body *').count() === baseline, 'Animated chart leaked its DOM after stop');

            await mount(kind, true);
            await advance(0);
            await call('phase', 2);
            await advance(0);
            const reducedLayers = await drawn();
            assert(reducedLayers.length === 1 && reducedLayers[0].mode === 'screen' &&
                !(await snapshot('svg')).animating && !(await snapshot('canvas')).animating, 'Reduced preference left paint ghosts');
            const reduced = await checkPaint(reducedLayers, `solver ${kind} reduced`);
            await call('stop');
            assert(await page.locator('body *').count() === baseline, 'Reduced-motion chart leaked its DOM');
            profiles.push({kind,initial,moved,mixed,mixedLayers,interrupted,three,rejected,resized,relayout,normal,reduced,allocationAtomic:true});
        }
        assert(pageErrors.length === 0, `Unexpected page errors ${pageErrors.join(' | ')}`);
        return {passed:true,profiles,pageErrors};
    } finally {
        await page.evaluate(display => {
            window.hostFixture.blend_motion_stop();
            window.blendMotionClock.restore();
            delete window.blendMotionClock;
            delete window.blendMotionPath;
            delete window.blendMotionScratchFailed;
            document.querySelector('.panels').style.display = display;
        }, display);
        page.off('pageerror', listener);
    }
})
