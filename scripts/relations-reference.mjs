// Executes actual Nivo selection functions. React state/memo adapters and
// computed-node fixtures are supplied; selection algorithms are not rewritten.
import {readFile,writeFile} from 'node:fs/promises';
import {stripTypeScriptTypes} from 'node:module';
import {createHash} from 'node:crypto';
import {join} from 'node:path';
const nivo=process.argv.slice(2).find(v=>!v.startsWith('--'))??'E:/Repos/nivo';
const tree=await readFile(join(nivo,'packages/tree/src/hooks.ts'),'utf8');
const chord=await readFile(join(nivo,'packages/chord/src/hooks.ts'),'utf8');
const sankey=await readFile(join(nivo,'packages/sankey/src/Sankey.tsx'),'utf8');
const sha=s=>createHash('sha256').update(s).digest('hex');
const module=code=>'data:text/javascript;base64,'+Buffer.from(stripTypeScriptTypes(code,{mode:'transform'})).toString('base64');
const slice=(source,start,end)=>{const a=source.indexOf(start),b=source.indexOf(end,a+start.length);if(a<0||b<0)throw Error('Nivo selection source markers changed');return source.slice(a,b);};
const treeModule=await import(module('const useCallback=fn=>fn;'+slice(tree,'const useSetCurrentNode','export const useTree =')+'export {useSetCurrentNode};'));
const chordModule=await import(module(`let states=[],cursor=0,selection;const useState=()=>[states[cursor++],()=>{}];const useMemo=fn=>{const r=fn();if(r&&Array.isArray(r.selectedArcIds))selection=r;return r;};
${slice(chord,'export const useChordSelection','export const useCustomLayerProps')}
export function select(kind,id,arcs,ribbons){states=[kind==='node'?arcs.find(a=>a.id===id):null,kind==='edge'?ribbons.find(r=>r.id===id):null];cursor=0;selection=null;const api=useChordSelection({ribbons,arcOpacity:.5,activeArcOpacity:.9,inactiveArcOpacity:.1,ribbonOpacity:.5,activeRibbonOpacity:.9,inactiveRibbonOpacity:.1});return{...selection,highlight:arcs.some(a=>api.getArcOpacity(a)!==.5)||ribbons.some(r=>api.getRibbonOpacity(r)!==.5)};}`));
const sankeyBody=slice(sankey,'        let isCurrentNode:', '    }, [currentLink, currentNode, links])');
const sankeyModule=await import(module(`const uniq=a=>[...new Set(a)];export function select(currentNode,currentLink,links){${sankeyBody}}`));
const treeNodes=[['r',null],['a','r'],['b','r'],['a1','a'],['a2','a'],['b1','b']].map(([uid,parent])=>({uid,parent}));
const byTree=new Map(treeNodes.map(n=>[n.uid,n]));
for(const n of treeNodes){n.ancestorUids=[];for(let p=n.parent;p!==null;p=byTree.get(p).parent)n.ancestorUids.unshift(p);n.descendantUids=treeNodes.filter(m=>{for(let p=m.parent;p!==null;p=byTree.get(p).parent)if(p===n.uid)return true;return false;}).map(m=>m.uid);}
const treeLinks=treeNodes.filter(n=>n.parent!==null).map(n=>({id:n.uid,source:byTree.get(n.parent),target:n}));
const ids=['a','b','c','isolated'],arcs=ids.map(id=>({id})),byId=new Map(arcs.map(a=>[a.id,a]));
const edges=[['ab','a','b'],['ab2','a','b'],['ba','b','a'],['bc','b','c'],['aa','a','a'],['ca','c','a']].map(([id,a,b])=>({id,source:byId.get(a),target:byId.get(b)}));
const set=a=>[...new Set(a)].sort().join(',');
const rows=[`# Actual Nivo 0.99 selectors; tree SHA256 ${sha(tree)}; chord ${sha(chord)}; sankey ${sha(sankey)}.`,'mode|kind|id|flags|nodes|edges|highlight'];
for(let flags=0;flags<16;flags++)for(const node of [...treeNodes,null]){
 let nodes=[],links=[];const select=treeModule.useSetCurrentNode({setActiveNodeUids:x=>nodes=x,setActiveLinkIds:x=>links=x,links:treeLinks,highlightAncestorNodes:!!(flags&1),highlightDescendantNodes:!!(flags&2),highlightAncestorLinks:!!(flags&4),highlightDescendantLinks:!!(flags&8)});select(node);
 rows.push(['tree',node?'node':'none',node?.uid??'',flags,set(nodes),set(links),Number(node!==null)].join('|'));
}
for(const [kind,items]of [['node',arcs],['edge',edges],['none',[{id:''}]]])for(const item of items){
 const c=chordModule.select(kind,item.id,arcs,edges);rows.push(['chord',kind,item.id,0,set(c.selectedArcIds),set(c.selectedRibbonIds),Number(c.highlight)].join('|'));
 const s=sankeyModule.select(kind==='node'?item:null,kind==='edge'?item:null,edges);rows.push(['sankey',kind,item.id,0,set(arcs.filter(s.isCurrentNode).map(a=>a.id)),set(edges.filter(s.isCurrentLink).map(e=>e.id)),Number(kind!=='none')].join('|'));
}
const fixture=new URL('../tests/fixtures/relations-nivo.csv',import.meta.url),output=rows.join('\n')+'\n';
if(process.argv.includes('--check')){if((await readFile(fixture,'utf8')).replaceAll('\r\n','\n')!==output)throw Error('Frozen Nivo relation fixture differs');console.log(`${rows.length-2} actual Nivo selections verified`);}else{await writeFile(fixture,output);console.log(`${rows.length-2} actual Nivo selections written`);}
