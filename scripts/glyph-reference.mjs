// Executes actual d3-shape 3.2.0 symbol draw methods into a recording context.
// Node/reference dependencies are development-only and resolved from the real
// Cargo workspace target; no target/cache redirection is performed.
import{readFile,writeFile}from'node:fs/promises';import{execFileSync}from'node:child_process';import{fileURLToPath,pathToFileURL}from'node:url';import{join}from'node:path';import{createHash}from'node:crypto';
const workspace=fileURLToPath(new URL('../',import.meta.url));
const target=JSON.parse(execFileSync('cargo',['metadata','--no-deps','--format-version','1'],{cwd:workspace,encoding:'utf8',windowsHide:true})).target_directory;
const directory=join(target,'d3-reference/node_modules/d3-shape');
const version=JSON.parse(await readFile(join(directory,'package.json'),'utf8')).version;if(version!=='3.2.0')throw Error('Expected d3-shape 3.2.0');
const d3=await import(pathToFileURL(join(directory,'src/index.js')).href);
const types=[['fill-circle','symbolCircle','circle'],['fill-cross','symbolCross','cross'],['fill-diamond','symbolDiamond','diamond'],['fill-square','symbolSquare','square'],['fill-star','symbolStar','star'],['fill-triangle','symbolTriangle','triangle'],['fill-wye','symbolWye','wye'],['stroke-circle','symbolCircle','circle'],['stroke-plus','symbolPlus','plus'],['stroke-times','symbolTimes','times'],['stroke-triangle','symbolTriangle2','triangle2'],['stroke-asterisk','symbolAsterisk','asterisk'],['stroke-square','symbolSquare2','square2'],['stroke-diamond','symbolDiamond2','diamond2']];
const sources={};for(const[, ,file]of types)sources[file]=createHash('sha256').update(await readFile(join(directory,'src/symbol/'+file+'.js'))).digest('hex');
const rows=['# Actual d3-shape '+version+' symbol sources SHA256 '+JSON.stringify(sources),'profile|shape|size|cx|cy|rotation|op|coordinates'];let profile=0;
for(const[name,key]of types)for(const size of[0,1,7,64,1000,1e6])for(const rotation of[0,.37,-1.2]){
 const cx=13.5,cy=-8.25,c=Math.cos(rotation),s=Math.sin(rotation),ops=[];const point=(x,y)=>[cx+x*c-y*s,cy+x*s+y*c];
 const context={moveTo(x,y){ops.push(['M',point(x,y)])},lineTo(x,y){ops.push(['L',point(x,y)])},closePath(){ops.push(['Z',[]])},rect(x,y,w,h){this.moveTo(x,y);this.lineTo(x+w,y);this.lineTo(x+w,y+h);this.lineTo(x,y+h);this.closePath()},arc(x,y,r,a,b,ccw){ops.push(['A',[...point(x,y),r,a+rotation,b+rotation,Number(!!ccw)]])}};
 d3[key].draw(context,size);for(const[op,coordinates]of ops)rows.push([profile,name,size,cx,cy,rotation,op,coordinates.join(',')].join('|'));profile++;
}
const output=rows.join('\n')+'\n',file=new URL('../tests/fixtures/glyph-d3.csv',import.meta.url);
if(process.argv.includes('--check')){if(output!==await readFile(file,'utf8'))throw Error('Glyph oracle differs');}else await writeFile(file,output);
console.log(JSON.stringify({profiles:profile,records:rows.length-2,version,sources}));
