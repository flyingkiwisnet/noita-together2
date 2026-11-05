import test from 'node:test';import assert from 'node:assert/strict';
import {WebSocket} from 'ws';import {Agent} from 'node:https';import {checkServerIdentity} from 'node:tls';import {randomUUID} from 'node:crypto';
import {startG3Fixture,provisionG3Fixture} from '../scripts/g3-fixture.js';import {livePeer} from '../scripts/g3-live-fixture.js';
import {encode,schema} from '../../../packages/contracts/src/g3-live.js';
const sleep=(ms:number)=>new Promise<void>(r=>setTimeout(r,ms));

test('WSS admission/frame abuse, reconnect pose fence, slow drain/control priority and genuine expiry',async()=>{
 let now=Date.now();const f=await startG3Fixture({live:true,tls:true,a1Runtime:true,a1HeartbeatBusy:true,nativeSocial:true,now:()=>now});const peers:Awaited<ReturnType<typeof livePeer>>[]=[];
 try{
  const s=await provisionG3Fixture(f,{beforeRpc:()=>{now+=100;}});await s.start();await s.playing();await s.pulse();
  const a=await livePeer(f,s,0),b=await livePeer(f,s,1);peers.push(a,b);
  const raw=async(payload:(socket:WebSocket)=>void,url='/v3/live',headers:Record<string,string>={},protocol='nt2.g3.live.v1')=>{
   const agent=new Agent({ca:f.ca!,checkServerIdentity:(_h,cert)=>checkServerIdentity('127.0.0.1',cert)}),socket=new WebSocket(f.origin.replace('https:','wss:')+url,protocol,{agent,headers});
   return new Promise<{code:number;reason:string}>((resolve)=>{let done=false;const finish=(code:number,reason:string)=>{if(done)return;done=true;clearTimeout(timer);socket.terminate();agent.destroy();resolve({code,reason});};const timer=setTimeout(()=>finish(0,'timeout'),4500);socket.on('open',()=>payload(socket));socket.on('close',(code,reason)=>finish(code,reason.toString()));socket.on('unexpected-response',(_req,res)=>{res.resume();finish(res.statusCode!,'upgrade');});socket.on('error',()=>finish(-1,'error'));});
  };
  const auth={major:1,revision:schema.revision,serviceId:f.service.db.serviceId,serviceEpoch:f.service.db.epoch,connectionId:randomUUID(),clientGeneration:1,sequence:1,scope:a.scope,authenticate:{accessToken:s.games[0]!.accessToken!}};
  assert.equal((await raw(ws=>ws.send(Buffer.from(encode('ClientFrame',auth))))).reason,'CAPACITY','one active connection per session');
  assert.equal((await raw(ws=>ws.send(Buffer.from(encode('ClientFrame',{...auth,scope:{...a.scope,worldToken:'wrong-world'}}))))).reason,'FORBIDDEN');assert.equal(b.socket.readyState,1,'wrong-scope admission cannot invalidate committed room authority');
  assert.equal((await raw(ws=>ws.send('text'))).reason,'INVALID_MESSAGE');
  assert.equal((await raw(ws=>ws.send(Buffer.alloc(32769)))).code,1009);
  assert.equal((await raw(()=>{},'/v3/live?credential=forbidden')).code,403);
  assert.equal((await raw(()=>{},'/v3/live',{origin:'https://wrong.example'})).code,403);
  assert.equal((await raw(()=>{},'/v3/live',{},'wrong.protocol')).code,403);
  assert.equal((await raw(()=>{})).reason,'EXPIRED','initial auth has a real deadline');
  a.send({pose:{sequence:1,xMilli:'1',yMilli:'0',vxMilli:'0',vyMilli:'0'}});await b.wait(v=>v.snapshot?.peers?.some(p=>p.poses?.at(-1)?.sequence===1)??false);await a.close();
  const retry=await livePeer(f,s,0);peers.push(retry);retry.send({pose:{sequence:1,xMilli:'1',yMilli:'0',vxMilli:'0',vyMilli:'0'}});assert.equal((await retry.wait(v=>!!v.retire)).retire!.code,'STALE_EPOCH');await retry.close();
  const abusive=await livePeer(f,s,0);peers.push(abusive);for(let i=2;i<13;i++)abusive.send({pose:{sequence:i,xMilli:'2',yMilli:'0',vxMilli:'0',vyMilli:'0'}});assert.equal((await abusive.wait(v=>!!v.retire)).retire!.code,'CAPACITY');assert.equal(b.socket.readyState,1,'abuse does not retire another participant');
  // Controlled stalled server write callback with a real WSS peer. The one latest
  // snapshot slot must coalesce; retirement cannot wait behind that blocked write.
  const serverSocket=[...f.service.app.websocketServer.clients].find(ws=>(ws as any)._socket.remotePort===(b.socket as any)._socket.localPort)!;assert.ok(serverSocket);
  const send=serverSocket.send.bind(serverSocket);(serverSocket as any).send=(data:any,options:any,callback:any)=>{if(typeof options==='function')callback=options;send(data,typeof options==='object'?options:{},()=>{setTimeout(()=>callback?.(),2500).unref();});};
  let closeCode=0;b.socket.once('close',code=>{closeCode=code;});await sleep(2300);assert.equal(closeCode,1013);assert.ok(f.service.live!.stats.coalescedSnapshots>0);assert.ok(f.service.live!.stats.maxControlFrames<=8);assert.ok(f.service.live!.stats.maxQueuedBytes<=65536);
  now+=1000;const expired=await livePeer(f,s,1);peers.push(expired);now+=31000;await expired.wait(v=>!!v.retire);assert.equal(expired.socket.readyState===1,false,'real original health deadline holds despite live socket activity');
 }finally{for(const p of peers)await p.close();await f.close();}
});

test('verified same-session renewal crosses original access expiry; old credential and late revocation refuse',async()=>{
 let now=Date.now();const f=await startG3Fixture({live:true,tls:true,a1Runtime:true,a1HeartbeatBusy:true,nativeSocial:true,now:()=>now});const peers:Awaited<ReturnType<typeof livePeer>>[]=[];
 try{
  const s=await provisionG3Fixture(f,{beforeRpc:()=>{now+=100;}});await s.start();await s.playing();await s.pulse();const oldToken=s.games[0]!.accessToken!,originalExpiry=Date.parse(s.games[0]!.expiresAt!);
  const a=await livePeer(f,s,0),b=await livePeer(f,s,1);peers.push(a,b);a.send({pose:{sequence:1,xMilli:'1',yMilli:'0',vxMilli:'0',vyMilli:'0'}});await b.wait(v=>!!v.snapshot?.peers?.length);
  while(now<originalExpiry-50000){now=Math.min(now+20000,originalExpiry-50000);await s.pulse();}
  for(const d of s.desktops){const refreshed=(await s.rpc(d,{refresh:{secret:d.refreshSecret!,deviceId:d.deviceId!}})).credentials!;Object.assign(d,refreshed);}
  const renewed=(await s.rpc(s.desktops[0]!,{gameRenew:{attachmentId:s.games[0]!.attachmentId!,gameSessionId:s.games[0]!.sessionId!,gameSessionEpoch:s.games[0]!.sessionEpoch!,gameInstanceId:s.observations[0]!.gameInstanceId!,worldToken:s.scopes[0]!.worldToken!}})).credentials!;Object.assign(s.games[0]!,renewed);a.send({reauthenticate:{accessToken:renewed.accessToken!,authRevision:2}});await a.wait(v=>v.welcome?.authRevision===2);
  while(now<originalExpiry-5000){now=Math.min(now+20000,originalExpiry-5000);await s.pulse();}now=originalExpiry+1000;
  a.send({pose:{sequence:2,xMilli:'2',yMilli:'0',vxMilli:'0',vyMilli:'0'}});await b.wait(v=>!!v.retire);await a.wait(v=>v.snapshot?.acceptedAt===new Date(now).toISOString());assert.equal(a.socket.readyState,1);assert.equal(f.service.live!.stats.receivedPoses,2);
  await a.close();s.games[0]!.accessToken=oldToken;await assert.rejects(livePeer(f,s,0),/UNAUTHENTICATED/);s.games[0]!.accessToken=renewed.accessToken!;
  const current=await livePeer(f,s,0);peers.push(current);
  // Actual account revocation goes through the existing family authority.
  await f.service.db.transaction(async tx=>{const actor=await f.service.auth.authenticate(tx,s.desktops[0]!.accessToken!);await f.service.auth.revoke(tx,actor.familyId);});await current.wait(v=>!!v.retire);assert.equal(f.service.live!.stats.samples,0);
 }finally{for(const p of peers)await p.close();await f.close();}
});
