import test from 'node:test';import assert from 'node:assert/strict';import {EventEmitter} from 'node:events';import {randomUUID} from 'node:crypto';
import {LifecycleTransport,LC1_FIXTURE_MESSAGE_LIMITS as limits} from '../scripts/lifecycle-lc1-transport.js';import {schema,encode} from '../../../packages/contracts/src/g3-live.js';
const sleep=()=>new Promise(r=>setTimeout(r,0));
function setup(){
 let release!:()=>void;const gate=new Promise<void>(r=>{release=r;}),received:{bytes:Buffer;binary:boolean}[]=[];
 class Socket extends EventEmitter{readyState=1;closed:{code:number;reason:string}|undefined;send(_data:any,_options?:any,callback?:()=>void){callback?.();}close(code:number,reason:string){this.closed={code,reason};this.readyState=3;this.emit('close');}}
 const transport=new LifecycleTransport(),service:any={app:{addHook(){}},db:{transaction:(fn:any)=>fn({})},auth:{authenticate:async()=>{await gate;return {accountId:randomUUID(),deviceId:randomUUID(),sessionId:randomUUID(),sessionEpoch:randomUUID()};}},live:{projection:{authorize:async()=>{}},connect:(socket:Socket)=>socket.on('message',(bytes:Buffer,binary:boolean)=>received.push({bytes,binary}))}};
 transport.install(service);const socket=new Socket();service.live.connect(socket);
 const base={major:1,revision:schema.revision,serviceId:'fixture-service',serviceEpoch:randomUUID(),connectionId:randomUUID(),clientGeneration:1,authorityGeneration:1,scope:{principalId:randomUUID(),sessionId:randomUUID(),sessionEpoch:randomUUID(),roomId:randomUUID(),runId:randomUUID(),runEpoch:randomUUID(),attachmentId:randomUUID(),configRevision:1,worldToken:randomUUID()}};
 const frame=(sequence:number,payload:any)=>Buffer.from(encode('ClientFrame',{...base,sequence,...payload}));
 return {socket,received,release,frame};
}
test('pending verified reauth preserves raw FIFO for poses, control and malformed/binary-flag frames',async()=>{
 const s=setup(),frames=[s.frame(2,{reauthenticate:{accessToken:'fixture-token',authRevision:2}}),s.frame(3,{pose:{sequence:1,xMilli:'0',yMilli:'0',vxMilli:'0',vyMilli:'0'}}),s.frame(4,{ping:{}}),Buffer.from([255]),Buffer.alloc(32769)];
 frames.forEach((bytes,i)=>s.socket.emit('message',bytes,i!==3));assert.equal(s.received.length,0);s.release();await sleep();
 assert.equal(s.received.length,frames.length);s.received.forEach((v,i)=>{assert.deepEqual(v.bytes,frames[i]);assert.equal(v.binary,i!==3);});assert.equal(s.socket.closed,undefined);
});
for(const kind of ['close','error'] as const)test('pending lookup cannot dispatch after socket '+kind,async()=>{
 const s=setup();s.socket.emit('message',s.frame(2,{reauthenticate:{accessToken:'fixture-token',authRevision:2}}),true);s.socket.emit('message',s.frame(3,{ping:{}}),true);
 if(kind==='close')s.socket.close(1000,'test close');else s.socket.emit('error',Error('synthetic socket error'));
 s.release();await sleep();assert.equal(s.received.length,0);s.socket.emit('message',s.frame(4,{ping:{}}),true);assert.equal(s.received.length,0);
});
for(const kind of ['frames','bytes'] as const)test('pending lookup has a finite '+kind+' bound including its auth frame',async()=>{
 const s=setup();s.socket.emit('message',s.frame(2,{reauthenticate:{accessToken:'fixture-token',authRevision:2}}),true);
 if(kind==='frames')for(let i=0;i<limits.frames;i++)s.socket.emit('message',Buffer.from([255]),true);
 else{ s.socket.emit('message',Buffer.alloc(32768),true);s.socket.emit('message',Buffer.alloc(32768),true); }
 assert.equal(s.socket.closed?.code,1013);s.release();await sleep();assert.equal(s.received.length,0);
});
