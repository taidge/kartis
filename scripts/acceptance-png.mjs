// Bounded PNG readback for browser acceptance; no image transformations.
import assert from 'node:assert/strict';
import {inflateSync} from 'node:zlib';
const table=Uint32Array.from({length:256},(_,n)=>{for(let i=0;i<8;i++)n=n&1?0xedb88320^(n>>>1):n>>>1;return n>>>0;});
const crc=b=>{let n=0xffffffff;for(const x of b)n=table[(n^x)&255]^(n>>>8);return(n^0xffffffff)>>>0;};
export function png(bytes){
 assert.equal(bytes.subarray(0,8).toString('hex'),'89504e470d0a1a0a');
 let width,height,channels,ended=false;const parts=[];
 for(let p=8;p<bytes.length;){
  const n=bytes.readUInt32BE(p),kind=bytes.subarray(p+4,p+8).toString(),body=bytes.subarray(p+8,p+8+n);
  assert(p+12+n<=bytes.length);assert.equal(crc(bytes.subarray(p+4,p+8+n)),bytes.readUInt32BE(p+8+n));
  if(kind==='IHDR'){width=body.readUInt32BE(0);height=body.readUInt32BE(4);channels={0:1,2:3,4:2,6:4}[body[9]];assert(width>0&&height>0&&width*height<=32_000_000);assert.equal(body[8],8);assert.equal(body[10],0);assert.equal(body[11],0);assert.equal(body[12],0);}
  if(kind==='IDAT')parts.push(body);if(kind==='IEND')ended=true;p+=n+12;
 }
 assert(ended&&channels);const stride=width*channels,raw=inflateSync(Buffer.concat(parts),{maxOutputLength:(stride+1)*height}),rows=Buffer.alloc(stride*height);
 assert.equal(raw.length,(stride+1)*height);
 const paeth=(a,b,c)=>{const v=a+b-c,aa=Math.abs(v-a),bb=Math.abs(v-b),cc=Math.abs(v-c);return aa<=bb&&aa<=cc?a:bb<=cc?b:c;};
 for(let y=0;y<height;y++){const filter=raw[y*(stride+1)];assert(filter<=4);for(let x=0;x<stride;x++){
  const a=x>=channels?rows[y*stride+x-channels]:0,b=y?rows[(y-1)*stride+x]:0,c=y&&x>=channels?rows[(y-1)*stride+x-channels]:0;
  rows[y*stride+x]=(raw[y*(stride+1)+x+1]+(filter===0?0:filter===1?a:filter===2?b:filter===3?Math.floor((a+b)/2):paeth(a,b,c)))&255;
 }}
 const data=Buffer.alloc(width*height*4);
 for(let p=0;p<width*height;p++){const i=p*channels,j=p*4;if(channels<3){data[j]=data[j+1]=data[j+2]=rows[i];data[j+3]=channels===2?rows[i+1]:255;}else{data[j]=rows[i];data[j+1]=rows[i+1];data[j+2]=rows[i+2];data[j+3]=channels===4?rows[i+3]:255;}}
 return{width,height,data};
}
export function pixel(image,x,y,width,height){
 assert(x>=0&&x<width&&y>=0&&y<height);const i=4*(Math.floor(y*image.height/height)*image.width+Math.floor(x*image.width/width));return[...image.data.subarray(i,i+4)];
}
