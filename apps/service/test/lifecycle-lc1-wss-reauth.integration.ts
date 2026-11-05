import test from 'node:test';import assert from 'node:assert/strict';import {randomUUID,randomBytes} from 'node:crypto';
import {startLifecycleLc1Fixture} from '../scripts/lifecycle-lc1-fixture.js';import {livePeer} from '../scripts/g3-live-fixture.js';
type Live=Awaited<ReturnType<typeof livePeer>>;
const sleep=(ms:number)=>new Promise<void>(r=>setTimeout(r,ms));
async function setup(guest=false){const x=await startLifecycleLc1Fixture();try{const peer=await x.bootstrapPeer(),peerRequest=x.request(peer.desktop,1,'peer',peer.observation);peerRequest.reconcile!.association=peer.association;peerRequest.reconcile!.proof=peer.proof;
 let owner=x.desktop,association=x.association,proof=x.proof,initial=x.observation;
 if(guest){await x.rpc(x.host,{roomGet:{}});await x.rpc(x.host,{roomPolicy:{capacity:32,allowGuests:true}});owner=(await x.rpc(undefined,{guestIssue:{deviceId:randomUUID(),issuanceProof:randomBytes(32).toString('base64url'),displayName:'Scoped guest expiry check'}})).credentials;await x.rpc(owner,{roomPreview:{}});await x.rpc(owner,{roomJoin:{}});const p=await x.prepareWorld(owner);association=p.association;proof=p.proof;initial={...initial,worldToken:association.worldId!,gameInstanceId:'guest-reauth'};}
 const request=(observation=initial)=>{const q=x.request(owner,1,'lc1-connection-1',observation);q.reconcile!.association=association;q.reconcile!.proof=proof;return q;};
 await x.send(owner,request());await x.send(peer.desktop,peerRequest);await x.start(guest?{selectedWorldIds:[association.worldId!,peer.association.worldId!],excludedDesktopIds:[x.desktop.sessionId!]}:undefined);
 const observation={...initial,protected:false,observationRevision:2},a=await x.send(owner,request(observation));peerRequest.reconcile!.observation={...peer.observation,protected:false,observationRevision:2};const b=await x.send(peer.desktop,peerRequest);
 const sockets:Live[]=[],open=async(value:any)=>{const p=await livePeer(x.fixture as any,{games:[value.credentials],scopes:[value.canonicalAdmission.scope]} as any,0);sockets.push(p);return p;};
 return {x,a,b,owner,observation,open,mint:()=>x.send(owner,request(observation)),close:async()=>{for(const s of sockets)await s.close();await x.close();}};
 }catch(e){await x.close();throw e;}}
test('LC1 equal-expiry distinct credential and same-token retry acknowledge without extending time; later expiry succeeds, older expiry refuses',async()=>{
 const s=await setup();try{const a=await s.open(s.a),peer=await s.open(s.b),next=await s.mint();assert.notEqual(next.credentials.accessToken,s.a.credentials.accessToken);assert.equal(next.credentials.sessionId,s.a.credentials.sessionId);assert.equal(next.credentials.expiresAt,s.a.credentials.expiresAt);
 a.send({reauthenticate:{accessToken:next.credentials.accessToken,authRevision:2}});const result=await a.wait(f=>!!f.retire||f.welcome?.authRevision===2);assert.ok(result.welcome);assert.equal(result.welcome.accessExpiresAt,s.a.credentials.expiresAt);
 s.x.advance(300);a.send({reauthenticate:{accessToken:next.credentials.accessToken,authRevision:3}});const retry=await a.wait(f=>f.welcome?.authRevision===3);assert.equal(retry.welcome!.accessExpiresAt,result.welcome.accessExpiresAt);assert.ok(retry.accessRemainingMs!<=result.accessRemainingMs!);
 s.x.advance(2000);const later=await s.mint();assert.ok(+new Date(later.credentials.expiresAt)>+new Date(next.credentials.expiresAt));a.send({reauthenticate:{accessToken:later.credentials.accessToken,authRevision:4}});assert.equal((await a.wait(f=>f.welcome?.authRevision===4)).welcome!.accessExpiresAt,later.credentials.expiresAt);
 s.x.advance(1000);a.send({reauthenticate:{accessToken:next.credentials.accessToken,authRevision:5}});assert.equal((await a.wait(f=>!!f.retire)).retire!.code,'STALE_EPOCH');assert.equal(peer.socket.readyState,1);
 console.log(JSON.stringify({reauthAssertions:{equalExpiryAccepted:true,sameTokenNextRevisionAccepted:true,noExpiryExtension:true,laterExpiryAccepted:true,olderEffectiveExpiryRefused:true,peerOpen:true}}));
 }finally{await s.close();}
});
test('LC1 backpressured welcome payload revision/expiry/authority and credential header correlate at drain time',async()=>{
 const s=await setup();let release:(()=>void)|undefined,serverSocket:any,originalSend:any;
 try{const a=await s.open(s.a),peer=await s.open(s.b),live=s.x.fixture.service.live as any,c=[...live.connections].find((c:any)=>c.binding?.scope.sessionId===s.a.credentials.sessionId);
  serverSocket=c.socket;originalSend=serverSocket.send;let held=false;
  serverSocket.send=function(data:any,options:any,callback:any){return originalSend.call(this,data,options,(error:any)=>{if(!held){held=true;release=()=>callback(error);}else callback(error);});};
  a.send({ping:{}});for(let i=0;i<100&&!release;i++)await sleep(10);assert.ok(release);assert.equal(c.sending,true);
  s.x.advance(2000);const next=await s.mint();assert.notEqual(next.credentials.expiresAt,s.a.credentials.expiresAt);const claims=await s.x.fixture.service.auth.security.verify(next.credentials.accessToken,s.x.fixture.service.auth.now());
  a.send({reauthenticate:{accessToken:next.credentials.accessToken,authRevision:2}});for(let i=0;i<150&&c.binding.authRevision!==2;i++)await sleep(10);assert.equal(c.binding.authRevision,2);
  serverSocket.send=originalSend;release!();release=undefined;
  const current=await a.wait(f=>f.welcome?.authRevision===2&&f.credentialId===claims.jti);assert.equal(current.welcome!.accessExpiresAt,next.credentials.expiresAt);assert.equal(current.welcome!.authorityUntil,new Date(c.binding.authorityUntil).toISOString());
  for(const {frame}of a.received.filter(r=>r.frame.welcome&&r.frame.credentialId===claims.jti)){assert.equal(frame.welcome!.authRevision,2);assert.equal(frame.welcome!.accessExpiresAt,next.credentials.expiresAt);assert.ok(frame.accessRemainingMs!<=Math.max(0,c.binding.accessUntil-s.x.fixture.service.auth.now()));}
  assert.equal(peer.socket.readyState,1);
  console.log(JSON.stringify({queuedWelcomeAssertions:{currentCredentialAndPayloadCorrelate:true,currentAuthRevision:2,noOldExpiryPayload:true,authorityBoundPreserved:true,peerOpen:true}}));
 }finally{if(serverSocket&&originalSend)serverSocket.send=originalSend;release?.();await s.close();}
});
for(const capKind of ['family','guest'] as const)test('LC1 '+capKind+' effective cap permits repeated equal-expiry rotation without capacity churn',async()=>{
 const s=await setup(capKind==='guest');try{const a=await s.open(s.a),peer=await s.open(s.b),until=new Date(Math.floor(s.x.fixture.service.auth.now()/1000)*1000+300000);
  await s.x.fixture.service.db.transaction(async tx=>{if(capKind==='guest')await tx.query('UPDATE principal_profiles SET guest_expires_at=$2 WHERE principal_id=$1',[s.owner.accountId,until]);else await tx.query('UPDATE session_families SET expires_at=$2 WHERE id=(SELECT family_id FROM sessions WHERE id=$1)',[s.owner.sessionId,until]);});
  const count=capKind==='family'?70:3;
  for(let i=0;i<count;i++){s.x.advance(300);const next=await s.mint();assert.equal(next.credentials.expiresAt,until.toISOString());a.send({reauthenticate:{accessToken:next.credentials.accessToken,authRevision:i+2}});const welcome=await a.wait(f=>!!f.retire||f.welcome?.authRevision===i+2);assert.ok(welcome.welcome,capKind+' valid capped credential refused');assert.equal(welcome.welcome.accessExpiresAt,until.toISOString());assert.ok(welcome.accessRemainingMs!<=+until-s.x.fixture.service.auth.now());}
  assert.equal(a.socket.readyState,1);assert.equal(peer.socket.readyState,1);peer.send({ping:{}});await peer.wait(f=>!!f.pong);
  console.log(JSON.stringify({cappedReauthAssertions:{cap:capKind,acceptedRotations:count,deadlineUnchanged:true,noHistoryCapacityDisconnect:true,peerOpen:true}}));
 }finally{await s.close();}
});
test('LC1 reauth preserves stale sequence/revision/scope/generation, foreign/revoked/expired token and terminal fences',async()=>{
 const s=await setup();try{const peer=await s.open(s.b);
  for(const kind of ['sequence','revision','scope','generation','foreign','revoked','expired'] as const){s.x.advance(1100);const a=await s.open(s.a),next=await s.mint();
   if(kind==='revoked'||kind==='expired'){const claims=await s.x.fixture.service.auth.security.verify(next.credentials.accessToken,s.x.fixture.service.auth.now());await s.x.fixture.service.db.transaction(tx=>kind==='revoked'?tx.query('UPDATE access_tokens SET revoked_at=$2 WHERE id=$1',[claims.jti,new Date(s.x.fixture.service.auth.now())]):tx.query('UPDATE access_tokens SET expires_at=$2 WHERE id=$1',[claims.jti,new Date(s.x.fixture.service.auth.now()-1)]));}
   a.send({reauthenticate:{accessToken:kind==='foreign'?s.b.credentials.accessToken:next.credentials.accessToken,authRevision:kind==='revision'?3:2},...(kind==='sequence'?{sequence:1}:{}),...(kind==='scope'?{scope:{...a.scope,worldToken:randomUUID()}}:{}),...(kind==='generation'?{authorityGeneration:a.received[0]!.frame.authorityGeneration!+1}:{})});
   const retired=await a.wait(f=>!!f.retire);assert.equal(retired.retire!.code,kind==='foreign'?'FORBIDDEN':kind==='revoked'||kind==='expired'?'UNAUTHENTICATED':'STALE_EPOCH');assert.equal(peer.socket.readyState,1);await a.close();
  }
  const a=await s.open(s.a);await s.x.end();assert.ok((await a.wait(f=>!!f.retire)).retire);assert.ok((await peer.wait(f=>!!f.retire)).retire);await assert.rejects(s.open(s.a),/Live frame timeout/);
  console.log(JSON.stringify({reauthFenceAssertions:{staleSequence:true,staleRevision:true,staleScope:true,staleGeneration:true,foreign:true,revoked:true,expired:true,terminal:true}}));
 }finally{await s.close();}
});

