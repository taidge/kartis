import assert from 'node:assert/strict';
import {writeFile} from 'node:fs/promises';
import {inflateSync} from 'node:zlib';
import {createHash} from 'node:crypto';
const hash=b=>createHash('sha256').update(b).digest('hex');
const canonicalState=s=>{const{revision,calls,events,...accepted}=s;return JSON.stringify(accepted);};
const canonicalDOM=s=>s.replace(/ data-revision="[^"]*"/g,'');
const crcTable=Uint32Array.from({length:256},(_,n)=>{for(let i=0;i<8;i++)n=n&1?0xedb88320^(n>>>1):n>>>1;return n>>>0;});
function crc32(bytes){let crc=0xffffffff;for(const b of bytes)crc=crcTable[(crc^b)&255]^(crc>>>8);return(crc^0xffffffff)>>>0;}

function png(bytes){
 assert.equal(bytes.subarray(0,8).toString('hex'),'89504e470d0a1a0a','Not PNG');
 let width,height,channels,ended=false;const idat=[];
 for(let p=8;p<bytes.length;){assert(p+12<=bytes.length,'Truncated PNG chunk');const n=bytes.readUInt32BE(p),type=bytes.toString('ascii',p+4,p+8),body=bytes.subarray(p+8,p+8+n);assert(p+12+n<=bytes.length,'Truncated PNG');assert.equal(crc32(bytes.subarray(p+4,p+8+n)),bytes.readUInt32BE(p+8+n),`PNG CRC ${type}`);
  if(type==='IHDR'){assert.equal(p,8);assert.equal(n,13);width=body.readUInt32BE(0);height=body.readUInt32BE(4);assert.equal(body[8],8);channels=({0:1,2:3,4:2,6:4})[body[9]];assert(channels&&body[10]===0&&body[11]===0&&body[12]===0,'Unsupported PNG profile');assert(width>0&&height>0&&width*height<=32_000_000,'PNG budget');}
  if(type==='IDAT')idat.push(body);if(type==='IEND')ended=true;p+=12+n;
 }
 assert(ended&&idat.length&&channels,'PNG missing data');const scan=inflateSync(Buffer.concat(idat)),stride=width*channels;
 assert.equal(scan.length,(stride+1)*height);const rows=Buffer.alloc(stride*height);
 const paeth=(a,b,c)=>{const p=a+b-c,pa=Math.abs(p-a),pb=Math.abs(p-b),pc=Math.abs(p-c);return pa<=pb&&pa<=pc?a:pb<=pc?b:c;};
 for(let y=0;y<height;y++){const f=scan[y*(stride+1)];assert(f<=4,'Bad PNG filter');for(let x=0;x<stride;x++){
  const a=x>=channels?rows[y*stride+x-channels]:0,b=y?rows[(y-1)*stride+x]:0,c=y&&x>=channels?rows[(y-1)*stride+x-channels]:0;
  rows[y*stride+x]=(scan[y*(stride+1)+1+x]+(f===0?0:f===1?a:f===2?b:f===3?Math.floor((a+b)/2):paeth(a,b,c)))&255;
 }}
 const data=new Uint8Array(width*height*4);for(let p=0;p<width*height;p++){const i=p*channels,j=p*4;
  if(channels===1||channels===2){data[j]=data[j+1]=data[j+2]=rows[i];data[j+3]=channels===2?rows[i+1]:255;}
  else {data[j]=rows[i];data[j+1]=rows[i+1];data[j+2]=rows[i+2];data[j+3]=channels===4?rows[i+3]:255;}
 }
 return{width,height,data};
}

function pixelDelta(a,b){
 assert.equal(a.length,3);assert.equal(b.length,3);const images=[...a,...b];
 if(images.some(i=>i.width!==images[0].width||i.height!==images[0].height))return{sameDimensions:false};
 let rawMaxChannelDelta=0,maxChannelDelta=0,changedPixels=0,total=0;
 for(let i=0;i<a[0].data.length;i+=4){let changed=false;for(let j=0;j<4;j++){
  const p0=a[0].data[i+j],p1=a[1].data[i+j],p2=a[2].data[i+j],q0=b[0].data[i+j],q1=b[1].data[i+j],q2=b[2].data[i+j];
  const loA=Math.min(p0,p1,p2),hiA=Math.max(p0,p1,p2),loB=Math.min(q0,q1,q2),hiB=Math.max(q0,q1,q2);
  rawMaxChannelDelta=Math.max(rawMaxChannelDelta,Math.abs(hiA-loB),Math.abs(hiB-loA));
  const d=Math.abs(p0+p1+p2-loA-hiA-(q0+q1+q2-loB-hiB));maxChannelDelta=Math.max(maxChannelDelta,d);total+=d;changed||=d!==0;
 }changedPixels+=Number(changed);}
 return{sameDimensions:true,rawMaxChannelDelta,maxChannelDelta,changedPixels,mean:total/a[0].data.length};
}

function checkpoint(renderer){
 const root=document.querySelector('#piecewise-scales-'+renderer+' .kartis-surface'),paint=root.querySelector(renderer==='svg'?'svg':'canvas'),r=root.getBoundingClientRect(),p=paint.getBoundingClientRect(),m=renderer==='svg'?paint.getScreenCTM():paint.getContext('2d').getTransform();
 return{state:JSON.parse(window.hostFixture.piecewise_scales_snapshot(renderer)),dom:root.outerHTML,rectangle:{rect:[r.x,r.y,r.width,r.height],paintRect:[p.x,p.y,p.width,p.height],scroll:[scrollX,scrollY],documentOrigin:[r.x+scrollX,r.y+scrollY],paintMatrix:[m.a,m.b,m.c,m.d,m.e,m.f],backing:renderer==='canvas'?[paint.width,paint.height]:null,viewBox:renderer==='svg'?[paint.viewBox.baseVal.x,paint.viewBox.baseVal.y,paint.viewBox.baseVal.width,paint.viewBox.baseVal.height]:null,dpr:devicePixelRatio},sourceData:renderer==='svg'?new XMLSerializer().serializeToString(paint):paint.toDataURL('image/png')};
}
async function exportPaint(renderer){
 const root=document.querySelector('#piecewise-scales-'+renderer+' .kartis-surface'),paint=root.querySelector(renderer==='svg'?'svg':'canvas');
 if(renderer==='canvas'){
  const uri=paint.toDataURL('image/png');return{kind:'canvas-backing',pngData:uri,sourceData:uri,metadata:{kind:'canvas-backing',dpr:devicePixelRatio,dimensions:[paint.width,paint.height],intrinsic:null,contextAttributes:paint.getContext('2d').getContextAttributes?.()??null,externalRefs:[]}};
 }
 const live=new XMLSerializer().serializeToString(paint),clone=paint.cloneNode(true),view=paint.viewBox.baseVal;
 let lang,langSource;for(let n=paint;n;n=n.parentElement){const xml=n.getAttributeNS('http://www.w3.org/XML/1998/namespace','lang')??n.getAttribute('xml:lang');if(xml!==null){lang=xml;langSource={value:xml,attribute:'xml:lang',tag:n.tagName,id:n.id??''};break;}if(n.hasAttribute('lang')){lang=n.getAttribute('lang');langSource={value:lang,attribute:'lang',tag:n.tagName,id:n.id??''};break;}}
 if(lang===undefined){lang=document.documentElement.lang;langSource={value:lang,attribute:'document.lang',tag:'HTML',id:document.documentElement.id??''};}
 const normalization={width:{before:paint.getAttribute('width'),after:String(view.width)},height:{before:paint.getAttribute('height'),after:String(view.height)},lang:{before:paint.getAttribute('lang'),after:lang},'xml:lang':{before:paint.getAttributeNS('http://www.w3.org/XML/1998/namespace','lang'),after:lang}};
 clone.setAttribute('width',String(view.width));clone.setAttribute('height',String(view.height));clone.setAttribute('lang',lang);clone.setAttributeNS('http://www.w3.org/XML/1998/namespace','xml:lang',lang);
 const resolved=new XMLSerializer().serializeToString(clone);
 const strip=s=>{const end=s.indexOf('>');return s.slice(0,end).replace(/ (?:width|height|lang|xml:lang)="[^"]*"/g,'')+s.slice(end);};
 if(strip(live)!==strip(resolved))throw Error('SVG paint capture changed more than root dimensions/language');
 const nodes=[paint,...paint.querySelectorAll('*')],ids=new Set(nodes.map(n=>n.getAttribute('id')).filter(Boolean)),internalRefs=[],externalRefs=[];
 for(const node of nodes)for(const attr of node.attributes){
  if(attr.localName==='href'){
   if(attr.value.startsWith('#'))internalRefs.push({fragment:attr.value.slice(1),resolves:ids.has(attr.value.slice(1)),sourceAttribute:attr.name});else externalRefs.push({attribute:attr.name,value:attr.value});
  }
  for(const match of attr.value.matchAll(/url\(([^)]+)\)/g)){const value=match[1].trim().replace(/^['"]|['"]$/g,'');if(value.startsWith('#'))internalRefs.push({fragment:value.slice(1),resolves:ids.has(value.slice(1)),sourceAttribute:attr.name});else externalRefs.push({attribute:attr.name,value});}
 }
 if(externalRefs.length||internalRefs.some(r=>!r.resolves))throw Error('Unresolved/external SVG paint resource in numeric fixture');
 const fontSamples=[...paint.querySelectorAll('text')].map(t=>{const css=getComputedStyle(t);return{text:t.textContent,fontFamily:css.fontFamily,fontSize:css.fontSize,fontWeight:css.fontWeight,fontStyle:css.fontStyle,explicitFamily:t.getAttribute('font-family'),explicitSize:t.getAttribute('font-size'),explicitWeight:t.getAttribute('font-weight')};});
 const image=new Image(),canvas=document.createElement('canvas'),url=URL.createObjectURL(new Blob([resolved],{type:'image/svg+xml;charset=utf-8'}));
 try{
  image.src=url;await image.decode();canvas.width=Math.round(view.width*devicePixelRatio);canvas.height=Math.round(view.height*devicePixelRatio);const context=canvas.getContext('2d',{willReadFrequently:true});context.drawImage(image,0,0,canvas.width,canvas.height);
  return{kind:'svg-live-dom',pngData:canvas.toDataURL('image/png'),sourceData:live,resolvedSVG:resolved,metadata:{kind:'svg-live-dom',dpr:devicePixelRatio,dimensions:[canvas.width,canvas.height],intrinsic:[image.naturalWidth,image.naturalHeight],lang,langSource,fontSamples,internalRefs,externalRefs,normalization,resolvedSvgOnlyRootDimensionsAndLanguage:true,contextAttributes:context.getContextAttributes?.()??null}};
 }finally{
  image.onload=image.onerror=null;image.removeAttribute('src');URL.revokeObjectURL(url);canvas.width=canvas.height=0;
 }
}
const uriBytes=data=>Buffer.from(data.slice(data.indexOf(',')+1),'base64');
export async function capturePaintSample(page,renderer,prefix){
 const before=await page.evaluate(checkpoint,renderer),pixelGrid=gridForRectangle(before.rectangle,[before.state.width,before.state.height]);
 const screenBytes=await page.locator('#piecewise-scales-'+renderer+' .kartis-surface').screenshot({path:prefix+'.png'});
 const exported=await page.evaluate(exportPaint,renderer),paintBytes=uriBytes(exported.pngData),after=await page.evaluate(checkpoint,renderer);
 const sourceBytes=s=>renderer==='canvas'?uriBytes(s):Buffer.from(s,'utf8');
 const beforeSHA=hash(sourceBytes(before.sourceData)),exportSHA=hash(sourceBytes(exported.sourceData)),afterSHA=hash(sourceBytes(after.sourceData));
 const metadata={...exported.metadata,sourceBeforeSHA:beforeSHA,sourceAtExportSHA:exportSHA,sourceAfterSHA:afterSHA,sourceStable:beforeSHA===exportSHA&&exportSHA===afterSHA};
 const write=async(suffix,data)=>{const path=prefix+suffix;await writeFile(path,data);return{path,sha256:hash(data)};};
 const paint=await write('.paint.png',paintBytes),beforeState=await write('.before-state.json',JSON.stringify(before.state,null,2)+'\n'),state=await write('.json',JSON.stringify(after.state,null,2)+'\n'),beforeDOM=await write('.before.html',before.dom),dom=await write('.html',after.dom),beforeRect=await write('.before.rect.json',JSON.stringify(before.rectangle,null,2)+'\n'),rect=await write('.rect.json',JSON.stringify(after.rectangle,null,2)+'\n');
 if(renderer==='svg'){
  const live=await write('.live.svg',exported.sourceData),resolved=await write('.resolved.svg',exported.resolvedSVG);Object.assign(metadata,{svgLiveFile:live.path,svgLiveSha256:live.sha256,svgResolvedFile:resolved.path,svgResolvedSha256:resolved.sha256});
 }
 const meta=await write('.paint-meta.json',JSON.stringify(metadata,null,2)+'\n');
 // Persist before asserting so failed samples retain complete checkpoints.
 assert.equal(canonicalState(before.state),canonicalState(after.state),'Paint sample changed accepted state');assert.equal(canonicalDOM(before.dom),canonicalDOM(after.dom),'Paint sample changed actual DOM');assert.deepEqual(before.rectangle,after.rectangle,'Paint sample changed actual rectangle/paint geometry');assert(metadata.sourceStable,'Live SVG/backing changed across screen/export checkpoints');
 const screenDecoded=png(screenBytes);assert.deepEqual([screenDecoded.width,screenDecoded.height],pixelGrid.expectedPNGDimensions,'Actual screen PNG must match accepted logical dimensions times DPR exactly');
 const decoded=png(paintBytes);assert.deepEqual([decoded.width,decoded.height],metadata.dimensions);if(renderer==='canvas')assert.equal(exportSHA,paint.sha256);else assert.equal(beforeSHA,metadata.svgLiveSha256);
 return{screenBytes,paintBytes,record:{pixelGrid,png:prefix+'.png',pngSha256:hash(screenBytes),paintPNG:paint.path,paintSha256:paint.sha256,state:state.path,stateSha256:state.sha256,dom:dom.path,domSha256:dom.sha256,beforeState:beforeState.path,beforeStateSha256:beforeState.sha256,beforeDOM:beforeDOM.path,beforeDOMSha256:beforeDOM.sha256,rectFile:rect.path,rectSha256:rect.sha256,beforeRectFile:beforeRect.path,beforeRectSha256:beforeRect.sha256,paintMetadataFile:meta.path,paintMetadataSha256:meta.sha256,rect:after.rectangle.rect,canonicalStateSha256:hash(canonicalState(after.state)),canonicalDOMSha256:hash(canonicalDOM(after.dom))}};
}
export function comparePNGTriples(a,b){return pixelDelta(a.map(png),b.map(png));}
export const paintGate=d=>d.sameDimensions&&d.rawMaxChannelDelta<=1&&d.maxChannelDelta<=1&&d.changedPixels<=4;
export const screenGate=d=>d.sameDimensions&&d.rawMaxChannelDelta<=1;

// Read actual screenshot bytes directly; never decode them into a browser canvas.
export function samplePNGWitness(bytes,geometry,expected){
 const image=png(bytes),x=Math.floor(geometry.fraction[0]*image.width),y=Math.floor(geometry.fraction[1]*image.height);let solid=0;
 for(let dy=-1;dy<=1;dy++)for(let dx=-1;dx<=1;dx++){
  const px=x+dx,py=y+dy;if(px<0||py<0||px>=image.width||py>=image.height)continue;
  const i=(py*image.width+px)*4;solid+=Number(expected.every((v,j)=>image.data[i+j]===v)&&image.data[i+3]===255);
 }
 return{solid,backing:[image.width,image.height],css:geometry.box.slice(2),sample:[x,y],expected};
}

function pixelGridDetail(input){
 const renderer=typeof input==='string'?input:input.renderer,host=document.getElementById('piecewise-scales-'+renderer),surface=host.querySelector('.kartis-surface'),paint=surface.querySelector(renderer==='svg'?'svg':'canvas');
 const read=()=>{
  const b=surface.getBoundingClientRect(),p=paint.getBoundingClientRect(),screen=renderer==='svg'?paint.getScreenCTM():paint.getContext('2d').getTransform(),local=renderer==='svg'?paint.getCTM():paint.getContext('2d').getTransform();
  return{renderer,state:window.hostFixture.piecewise_scales_snapshot(renderer),childDOM:surface.outerHTML,hostStyle:host.getAttribute('style'),rect:[b.x,b.y,b.width,b.height],documentOrigin:[b.x+scrollX,b.y+scrollY],scroll:[scrollX,scrollY],localPaintRect:[p.x-b.x,p.y-b.y,p.width,p.height],localMatrix:[local.a,local.b,local.c,local.d,local.e,local.f],screenMatrix:[screen.a,screen.b,screen.c,screen.d,screen.e,screen.f],dpr:devicePixelRatio};
 };
 const before=read();if(typeof input==='string'||!input.align)return before;
 host.style.position='relative';host.style.left=String((parseFloat(host.style.left)||0)+Math.round(before.documentOrigin[0])-before.documentOrigin[0])+'px';host.style.top=String((parseFloat(host.style.top)||0)+Math.round(before.documentOrigin[1])-before.documentOrigin[1])+'px';
 return{before,after:read()};
}
function gridForRectangle(rectangle,logicalSize){
 const origin=rectangle.documentOrigin,[width,height]=rectangle.rect.slice(2),x=Math.floor(origin[0]+1e-3),y=Math.floor(origin[1]+1e-3),right=Math.ceil(origin[0]+width-1e-3),bottom=Math.ceil(origin[1]+height-1e-3);
 assert(origin.every(v=>Number.isFinite(v)&&Math.abs(v-Math.round(v))<=1e-3),'Document origin is outside Playwright integer-grid precision');
 assert.deepEqual([right-x,bottom-y],logicalSize.map(Math.round),'Integer screenshot envelope differs from accepted logical dimensions');
 return{actualDocumentOrigin:origin,integerEnvelope:{x,y,width:right-x,height:bottom-y},logicalSize,precision:1e-3,expectedPNGDimensions:logicalSize.map(v=>Math.round(v*rectangle.dpr))};
}
export async function verifyHostPixelGrid(page,renderer){
 const detail=await page.evaluate(pixelGridDetail,renderer),state=JSON.parse(detail.state);return gridForRectangle(detail,[state.width,state.height]);
}
export async function alignHostPixelGrid(page,renderer,prefix,reason='mount'){
 // One browser task preserves the exact queue/model baseline across the style change.
 const{before,after}=await page.evaluate(pixelGridDetail,{renderer,align:true}),bytes=JSON.stringify({renderer,reason,before,after},null,2)+'\n',path=prefix+'.pixel-grid-alignment.json';await writeFile(path,bytes);
 assert.equal(before.state,after.state,'Pixel-grid setup changed model state');assert.equal(before.childDOM,after.childDOM,'Pixel-grid setup changed child paint DOM');assert.deepEqual(before.localPaintRect,after.localPaintRect,'Pixel-grid setup changed local paint rectangle');assert.deepEqual(before.localMatrix,after.localMatrix,'Pixel-grid setup changed local paint matrix');assert.deepEqual(before.rect.slice(2),after.rect.slice(2),'Pixel-grid setup changed accepted size');assert.deepEqual(before.scroll,after.scroll,'Pixel-grid setup scrolled the document');
 const state=JSON.parse(after.state),grid=gridForRectangle(after,[state.width,state.height]);await page.evaluate(()=>new Promise(requestAnimationFrame));
 return{renderer,reason,path,sha256:hash(bytes),grid,stateBeforeSHA:hash(before.state),stateAfterSHA:hash(after.state),childDOMBeforeSHA:hash(before.childDOM),childDOMAfterSHA:hash(after.childDOM),localGeometryUnchanged:true};
}
