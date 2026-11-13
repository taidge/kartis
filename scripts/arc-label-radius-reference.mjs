// Execute the real Nivo ArcLabelsLayer filter independently of the Rust layouts.
import {readFileSync} from 'node:fs';
import {resolve} from 'node:path';
const [nivo] = process.argv.slice(2);
if (!nivo) throw Error('usage: arc-label-radius-reference.mjs <nivo>');
const source = readFileSync(resolve(nivo, 'packages/arcs/src/arc_labels/ArcLabelsLayer.tsx'), 'utf8');
const match = source.match(/data\.filter\(datum => \{([\s\S]*?)\n\s*\}\)/);
if (!match) throw Error('Nivo arc-label filter changed');
const filter = new Function('data', 'skipAngle', 'skipRadius', 'radiansToDegrees',
    `return data.filter(datum => {${match[1]}})`);
console.log('# Actual Nivo ArcLabelsLayer filter; angles in radians, radii in pixels.');
console.log('inner|outer|angle|skip_angle|skip_radius|admitted');
for (const [inner, outer, angle, skipAngle, skipRadius] of [
    [90,100,Math.PI/2,0,11], [90,100,Math.PI/2,0,10],
    [90,100,Math.PI/2,0,9], [0,100,Math.PI/2,0,100],
    [0,100,Math.PI/2,0,101], [100,100,Math.PI/2,0,0],
    [100,100,Math.PI/2,0,0.01], [100,90,Math.PI/2,0,10],
    [90,100,-Math.PI/2,Math.PI/2,10],
    [90,100,Math.PI/2,Math.PI/2+0.01,10],
    [90,100,0,0,10], [90,100,0,0.01,10],
]) {
    const datum = {arc:{startAngle:0,endAngle:angle,innerRadius:inner,outerRadius:outer}};
    const result = filter([datum],skipAngle*180/Math.PI,skipRadius,x=>x*180/Math.PI);
    console.log([inner,outer,angle,skipAngle,skipRadius,result.length].join('|'));
}
