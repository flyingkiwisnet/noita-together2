// Bounded attribution probe: one actual room-row transaction stalls an unrelated
// room's ordinary presence even though the work gate has spare permits.
import assert from 'node:assert/strict';
import {fork} from 'node:child_process';
import {Agent,request} from 'node:https';
import {randomUUID} from 'node:crypto';
import {fileURLToPath} from 'node:url';
import {writeFile} from 'node:fs/promises';
import {performance} from 'node:perf_hooks';
import {livePeer} from './g3-live-fixture.ts';
import {schema} from '../../../packages/contracts/src/g2-next.ts';
const sleep=ms=>new Promise(r=>setTimeout(r,ms));
const child=fork(fileURLToPath(new URL('./lc1-capacity-benchmark.mjs',import.meta.url)),['--worker','2','2'],{execArgv:['--import','tsx'],windowsHide:true,stdio:['ignore','pipe','pipe','ipc']});
let absent=false,heldResolve,readyResolve,readyReject;const pending=new Map(),ready=new Promise((r,j)=>{readyResolve=r;readyReject=j;});
child.stdout.on('data',b=>{process.stdout.write(b);if(String(b).includes('verifiedAbsentSchema'))absent=true;});child.stderr.on('data',b=>process.stderr.write(b));
child.on('message',m=>{if(m.type==='ready')readyResolve(m);else if(m.type==='failed')readyReject(Error(m.error));else if(m.type==='held')heldResolve?.();else if(pending.has(m.id)){const p=pending.get(m.id);pending.delete(m.id);m.error?p.reject(Error(m.error)):p.resolve(m.value);}});
const command=command=>new Promise((resolve,reject)=>{const id=randomUUID();pending.set(id,{resolve,reject});child.send({id,command});});
let agent,timer,result;const sockets=[];
try{
 const f=await ready;agent=new Agent({ca:Buffer.from(f.ca,'base64'),keepAlive:true});
 const peerFixture={origin:f.origin,ca:Buffer.from(f.ca,'base64'),service:{db:{serviceId:f.base.serviceId,epoch:f.base.serviceEpoch}}};
 for(let i=0;i<f.users.length;i++)sockets.push(await livePeer(peerFixture,{games:f.users.map(u=>u.game),scopes:f.users.map(u=>u.scope)},i,{agent}));
 const sequence=f.users.map(()=>0);timer=setInterval(()=>sockets.forEach((s,i)=>s.send({pose:{sequence:++sequence[i],xMilli:String(sequence[i]*10),yMilli:'1000',vxMilli:'0',vyMilli:'0'}})),125);
 await command('begin');
 const presence=u=>new Promise((resolve,reject)=>{
  const at=performance.now(),q={...f.base,revision:schema.revision,requestId:randomUUID(),sessionEpoch:u.desktop.sessionEpoch,roomId:u.room.roomId,runId:u.room.runId,runEpoch:u.room.runEpoch,configRevision:u.room.configRevision,presence:{present:true}},body=JSON.stringify(q);
  const req=request(f.origin+'/v1/rpc',{method:'POST',agent,headers:{authorization:'Bearer '+u.desktop.accessToken,'content-type':'application/json','content-length':Buffer.byteLength(body)}},res=>{let body='';res.on('data',b=>{body+=b;});res.on('end',()=>{try{const v=JSON.parse(body);resolve({roomId:u.room.roomId,code:v.error?.code??'OK',latencyMs:performance.now()-at});}catch(e){reject(e);}});});req.setTimeout(3000,()=>req.destroy(Error('DEADLINE')));req.on('error',reject);req.end(body);
 });
 const representatives=[f.users[0],f.users[2]],before=[];for(const u of representatives)before.push(await presence(u));assert.ok(before.every(v=>v.code==='OK'));
 const held=new Promise(r=>{heldResolve=r;}),hold=command('hold').then(value=>({value}),error=>({error:error.message}));
 await Promise.race([held,hold.then(v=>{if(v.error)throw Error(v.error);})]);
 const during=await Promise.all(representatives.map(presence)),holdOutcome=await hold;
 assert.equal(holdOutcome.error,undefined);assert.ok(during.every(v=>v.code==='SERVICE_UNAVAILABLE'));assert.ok(during.every(v=>v.latencyMs>=450));
 const after=[];for(const u of representatives)after.push(await presence(u));assert.ok(after.every(v=>v.code==='OK'));
 await sleep(250);for(const p of sockets){assert.equal(p.socket.readyState,1);assert.ok(p.received.some(v=>v.frame.snapshot?.peers?.some(peer=>peer.poses?.some(p=>p.sequence>=2))));assert.equal(p.received.some(v=>v.frame.retire),false);}
 result={gate:'G3A reconnect/rejoin before A2',owner:'Server',namespace:f.namespace,privateFixtureCaPath:f.caPath,substitutions:['Actual TLS/SQL and WSS; synthetic engine reports and movement; setup uses bounded fixture clock, measurement elapsed wall time','650ms fixture transaction locks only room0 row; unchanged namespace mutation advisory lock acquired by Database.transaction'],targetRoom:f.rooms[0],unrelatedRoom:f.rooms[1],before,during,after,holdOutcome,allFourSocketsOpenAndFreshPeerPoses:true,service:await command('metrics')};
}finally{
 clearInterval(timer);for(const p of sockets)await p.close();agent?.destroy();const exited=new Promise(r=>child.once('exit',r));await command('close');await exited;
 if(result){result.cleanup={workerClosed:true,ownedSchemaAbsenceReported:absent};assert.equal(absent,true);await writeFile(new URL('../../../docs/g3/server-lc1-capacity-isolation.json',import.meta.url),JSON.stringify(result,null,2)+'\n');console.log(JSON.stringify({isolationPassed:true,ownedSchemaCleaned:true}));}
}
