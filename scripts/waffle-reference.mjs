// Node 24+. Execute the original Nivo grid and allocation routines.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {readFileSync, writeFileSync} from 'node:fs';
import {stripTypeScriptTypes} from 'node:module';
import {join} from 'node:path';

const [root, report] = process.argv.slice(2);
if (!root || !report) throw Error('usage: waffle-reference.mjs <nivo> <report.json>');
const read = name => readFileSync(join(root, 'packages', name), 'utf8');
const sourceNames = ['waffle/src/hooks.ts', 'grid/src/grid.ts', 'grid/src/boundingBoxes.ts', 'grid/src/polygon.ts'];
const sources = Object.fromEntries(sourceNames.map(name => [name, read(name)]));
const compile = source => stripTypeScriptTypes(source
  .replace(/^import .* from '[^']+'\r?$/gm, '')
  .replace(/\bexport (?=const|function)/g, ''));
const hooks = sources['waffle/src/hooks.ts'];
const body = hooks.slice(hooks.indexOf('let currentPosition = 0'), hooks.indexOf('return enhancedData') + 'return enhancedData'.length);
const gridHook = hooks.slice(hooks.indexOf('export const computeGrid'), hooks.indexOf('export const mergeCellsData'));
const compute = new Function(compile([
  ...sourceNames.slice(1).map(name => sources[name]), gridHook,
  `function allocate(data, hiddenIds, unit) {
    const getColor = () => '#aaa';
    const getBorderColor = () => '#999';
    const formatValue = String;
    ${body}
  }`,
].join('\n')) + '\nreturn {computeGrid, getCellsPolygons, allocate};')();

const starts = {
  top: [[2,3],[2,2],[2,1],[2,0],[1,3]],
  right: [[0,0],[1,0],[2,0],[0,1],[1,1]],
  bottom: [[0,0],[0,1],[0,2],[0,3],[1,0]],
  left: [[2,3],[1,3],[0,3],[2,2],[1,2]],
};
const grid = fillDirection => compute.computeGrid({width:248, height:174, rows:3, columns:4,
  fillDirection, emptyColor:'#ccc', emptyOpacity:1, getBorderColor:({color})=>color});
const cellBounds = {}, areaProfiles = [];
for (const [direction, expected] of Object.entries(starts)) {
  const cells = grid(direction);
  assert.deepEqual(cells.slice(0, 5).map(({row,column}) => [row,column]), expected);
  assert(cells.every(cell=>cell.width===58 && cell.height===58 && cell.x===8+cell.column*58 && cell.y===cell.row*58));
  cellBounds[direction] = cells.map(({row,column,x,y,width,height})=>({row,column,x,y,width,height}));
  for (let start=0; start<12; start++) for (const count of [1, Math.min(12-start, 5)]) {
    const polygons = compute.getCellsPolygons(cells.slice(start, start+count));
    const area = polygons.reduce((sum, polygon) => sum + polygon.reduce((a, [x,y], i) => {
      const [nx,ny] = polygon[(i+1)%polygon.length];
      return a + x*ny-y*nx;
    },0)/2,0);
    assert.equal(area, count*58**2);
    areaProfiles.push({direction,start,count,area});
  }
}
const raw = [1,1,1,1,1,3].map((value,i) => ({value, id:String(i), label:String(i)}));
const ranges = (data, hiddenIds, unit) => compute.allocate(data,hiddenIds,unit).map(({startAt,endAt})=>[startAt,endAt]);
const allocation = {
  overflow: ranges(raw,[],2),
  hidden: ranges(raw,['0','1','2'],2),
  normal: ranges([{id:'one',label:'one',value:27},{id:'two',label:'two',value:34}],[],100/12),
};
assert.deepEqual(allocation.overflow, [[0,1],[1,2],[2,3],[3,4],[4,5],[5,7]]);
assert.deepEqual(allocation.hidden, [[0,0],[0,0],[0,0],[0,1],[1,2],[2,4]]);
assert.deepEqual(allocation.normal, [[0,3],[3,7]]);
const sourceHashes = Object.fromEntries(sourceNames.map(name => [name, createHash('sha256').update(sources[name]).digest('hex')]));
const version = JSON.parse(read('waffle/package.json')).version;
writeFileSync(report, JSON.stringify({passed:true, version, sourceHashes, cellBounds, areaProfiles, allocation}, null, 2));
console.log(JSON.stringify({passed:true, cells:48, areaProfiles:areaProfiles.length, allocation}));
