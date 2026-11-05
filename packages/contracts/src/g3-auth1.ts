import { createRequire } from 'node:module';
import protobuf from 'protobufjs';
export type * from '../g3-auth1/g3-auth1.js';
const require=createRequire(import.meta.url);
export const schema=require('../g3-auth1/schema.json') as Schema;
const root=protobuf.Root.fromJSON(require('../g3-auth1/descriptor.json'));
interface Field {id:number; type:string; repeated?:boolean; max?:number; itemMax?:number;required?:boolean;maxValue?:number;values?:string[]}
interface Message {fields:Record<string,Field>; oneof?:string[]}
interface Schema {revision:string;major:number;limits:{bytes:number;fields:number;depth:number};errors:string[];messages:Record<string,Message>}
export class InvalidMessage extends Error {}
function fail():never{throw new InvalidMessage('Message violates canonical schema or bounds');}
export function validate(name:string,value:unknown,depth=0):Record<string,unknown>{
 const definition=schema.messages[name];if(!definition||!value||typeof value!=='object'||Array.isArray(value)||depth>schema.limits.depth)fail();
 const obj=value as Record<string,unknown>;
 for(const [key,f]of Object.entries(definition.fields))if(f.required&&!Object.hasOwn(obj,key))fail();
 for(const [key,v] of Object.entries(obj)){
   const f=definition.fields[key];if(!f)fail();
   const check=(item:unknown)=>{if(f.type==='string'){if(typeof item!=='string'||Buffer.byteLength(item)>(f.repeated?f.itemMax??128:f.max??128)||f.values&&!f.values.includes(item))fail();}
     else if(f.type==='bool'){if(typeof item!=='boolean')fail();}
     else if(f.type==='uint32'){if(typeof item!=='number'||!Number.isSafeInteger(item)||item<0||item>(f.maxValue??0xffffffff))fail();}
     else validate(f.type,item,depth+1);};
   if(f.repeated){if(!Array.isArray(v)||v.length>(f.max??32))fail();for(const item of v)check(item);}else check(v);
 }
 if(definition.oneof&&definition.oneof.filter(k=>Object.hasOwn(obj,k)).length!==1)fail();
 return obj;
}
// Scan before protobufjs decoding: reject unknown tags, duplicates, overlong varints,
// repeated singular fields and conflicting oneofs rather than silently dropping them.
export function decode(name:string,bytes:Uint8Array):Record<string,unknown>{
 if(bytes.length>schema.limits.bytes)fail();let budget=0;
 function scan(type:string,start:number,end:number,depth:number){
  if(depth>schema.limits.depth)fail();const def=schema.messages[type];if(!def)fail();let p=start;const seen=new Set<number>();let oneof=0;
  const variable=()=>{let n=0,m=1;for(let i=0;i<5;i++){if(p>=end)fail();const b=bytes[p++]!;n+=(b&127)*m;if(!(b&128)){if(i>0&&b===0)fail();return n;}m*=128;}return fail();};
  while(p<end){if(++budget>schema.limits.fields)fail();const tag=variable(),id=tag>>>3,wire=tag&7;const match=Object.entries(def.fields).find(([,f])=>f.id===id);if(!match)fail();const [key,f]=match;
   if(!f.repeated&&seen.has(id))fail();seen.add(id);if(def.oneof?.includes(key)&&++oneof>1)fail();
   if(f.type==='bool'||f.type==='uint32'){if(wire!==0)fail();const n=variable();if(n>0xffffffff||(f.type==='bool'&&n>1))fail();}
   else {if(wire!==2)fail();const len=variable();if(p+len>end)fail();if(f.type!=='string')scan(f.type,p,p+len,depth+1);else {try{new TextDecoder('utf-8',{fatal:true}).decode(bytes.subarray(p,p+len));}catch{fail();}}p+=len;}
  }
  if(def.oneof&&oneof!==1)fail();
 }
 scan(name,0,bytes.length,0);
 const type=root.lookupType('nt2.auth1.'+name);
 const obj=type.toObject(type.decode(bytes),{defaults:false,arrays:false,objects:false}) as Record<string,unknown>;
 return validate(name,obj);
}
export function encode(name:string,value:unknown):Uint8Array{
 const obj=validate(name,value);const type=root.lookupType('nt2.auth1.'+name);const bytes=type.encode(type.fromObject(obj)).finish();if(bytes.length>schema.limits.bytes)fail();return bytes;
}
export function parseJson(name:string,text:string):Record<string,unknown>{
 if(Buffer.byteLength(text)>schema.limits.bytes)fail();let p=0,count=0;
 const space=()=>{while(/\s/.test(text[p]??'')&&p<text.length)p++;};
 const string=()=>{const start=p++;while(p<text.length){const c=text[p++];if(c==='\\'){p++;continue;}if(c==='"'){try{return JSON.parse(text.slice(start,p)) as string;}catch{fail();}}}return fail();};
 const scan=(depth:number)=>{if(depth>schema.limits.depth)fail();space();const c=text[p];
  if(c==='{'||c==='['){p++;space();const close=c==='{'?'}':']';const keys=new Set<string>();if(text[p]===close){p++;return;}
   while(p<text.length){if(++count>schema.limits.fields)fail();space();if(c==='{'){if(text[p]!=='"')fail();const key=string();if(keys.has(key))fail();keys.add(key);space();if(text[p++]!==':')fail();}scan(depth+1);space();if(text[p]===close){p++;return;}if(text[p++]!==',')fail();}fail();
  }else if(c==='"')string();else{const start=p;while(p<text.length&&!/[\s,}\]]/.test(text[p]!))p++;if(p===start)fail();}
 };
 scan(0);space();if(p!==text.length)fail();let value:unknown;try{value=JSON.parse(text);}catch{fail();}return validate(name,value);
}
