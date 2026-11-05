import test from 'node:test';import assert from 'node:assert/strict';
import {randomUUID,randomBytes} from 'node:crypto';
import {startAuth1Fixture} from '../scripts/auth1-fixture.js';
import {hash,PolicyError} from '../src/g1/security.js';
import {signInSource,SIGNIN_ADMISSION} from '../src/auth1/admission.js';
import {PROVIDER_SCHEDULE} from '../src/auth1/provider-schedule.js';
import {b1Fixture} from './b1-fixture-support.js';
type Fixture=Awaited<ReturnType<typeof startAuth1Fixture>>;
const request=(f:Fixture,payload:any,ip='198.51.100.2',requestId=randomUUID())=>f.service.app.inject({method:'POST',url:'/v1/rpc',remoteAddress:ip,headers:{host:new URL(f.origin).host},payload:{major:1,revision:'g2-draft-5',serviceId:f.service.db.serviceId,serviceEpoch:f.service.db.epoch,requestId,...payload}});
const beginInput=()=>{const proof=randomBytes(32).toString('base64url');return {proof,input:{provider:'local',deviceId:randomUUID(),appInstanceId:randomUUID(),proofChallenge:hash(proof),callbackState:randomBytes(32).toString('base64url'),callbackPort:33333}};};
test('AUTH1 source-bound outstanding/sustained admission isolates web/app abuse while shared-NAT retries and cancellation work',async()=>{
 const f=await startAuth1Fixture();try{
  const web=(ip:string,headers:any={})=>f.service.app.inject({method:'GET',url:'/auth/login/local',remoteAddress:ip,headers:{host:new URL(f.origin).host,...headers}});
  let accepted=0;for(let i=0;i<128;i++){f.advance(100);const r=await web('203.0.113.77',{'x-forwarded-for':'198.51.100.'+i});if(r.statusCode===200)accepted++;else assert.equal(r.json().error.code,'CAPACITY');}
  assert.equal(accepted,SIGNIN_ADMISSION.outstandingPerSource);assert.equal((await web('198.51.100.4')).statusCode,200);
  accepted=0;for(let i=0;i<128;i++){f.advance(100);const r=await request(f,{authBegin:beginInput().input},'203.0.113.77');if(r.json().authAttempt)accepted++;else assert.equal(r.json().error.code,'CAPACITY');}
  assert.equal(accepted,SIGNIN_ADMISSION.outstandingPerSource);assert.ok((await request(f,{authBegin:beginInput().input},'198.51.100.4')).json().authAttempt);
  const nat='192.0.2.44',users=[];for(let i=0;i<32;i++){f.advance(100);const user=beginInput(),id=randomUUID(),r=await request(f,{authBegin:user.input},nat,id);assert.ok(r.json().authAttempt,r.body);users.push({...user,id,attempt:r.json().authAttempt});}
  const own=users[0]!;f.advance(1000);assert.equal((await request(f,{authBegin:own.input},nat,own.id)).json().authAttempt.attemptId,own.attempt.attemptId,'exact original retry consumes no extra slot');
  const proof={attemptId:own.attempt.attemptId,deviceId:own.input.deviceId,appInstanceId:own.input.appInstanceId,initiatorProof:own.proof};
  const flow=await f.service.app.inject({method:'GET',url:'/signin/'+own.attempt.attemptId,remoteAddress:'192.0.2.45',headers:{host:new URL(f.origin).host}});assert.equal(flow.statusCode,200);
  assert.equal((await request(f,{authCancel:proof},nat)).json().authAttempt.state,'cancelled');
  const flowCount=await f.service.db.transaction(async tx=>(await tx.query('SELECT count(*)::int AS n FROM auth1_flows WHERE attempt_id=$1 AND consumed_at IS NULL',[own.attempt.attemptId])).rows[0].n);assert.equal(flowCount,0);
  assert.ok((await request(f,{authBegin:beginInput().input},nat)).json().authAttempt,'cancel frees reservation');
  // Cancellation cannot turn admission into unbounded durable-row churn.
  for(let i=0;i<SIGNIN_ADMISSION.startsPerWindow;i++){f.advance(150);const u=beginInput(),r=await request(f,{authBegin:u.input},'192.0.2.111');assert.ok(r.json().authAttempt,r.body);await request(f,{authCancel:{attemptId:r.json().authAttempt.attemptId,deviceId:u.input.deviceId,appInstanceId:u.input.appInstanceId,initiatorProof:u.proof}},'192.0.2.111');}
  f.advance(1000);assert.equal((await request(f,{authBegin:beginInput().input},'192.0.2.111')).json().error.code,'CAPACITY');
  // A browser-owned local flow is replaced without extending its expiry or
  // accumulating stale flow cookies. A caller-chosen flow ID is insufficient.
  const jar=new Map<string,string>();let firstExpiry=0;
  for(let i=0;i<70;i++){f.advance(100);const r=await web('192.0.2.99',{cookie:[...jar].map(([k,v])=>k+'='+v).join('; ')});assert.equal(r.statusCode,200,r.body);for(const item of ([] as string[]).concat(r.headers['set-cookie'] as any??[])){const [key,value]=item.split(';')[0]!.split('=');if(value)jar.set(key!,value);else jar.delete(key!);}if(i===0)firstExpiry=await f.service.db.transaction(async tx=>+(await tx.query('SELECT expires_at FROM auth1_flows WHERE source_hash=$1',[signInSource('192.0.2.99')])).rows[0].expires_at);}
  const reused=await f.service.db.transaction(async tx=>(await tx.query('SELECT expires_at FROM auth1_flows WHERE source_hash=$1',[signInSource('192.0.2.99')])).rows);assert.equal(reused.length,1);assert.equal(+reused[0]!.expires_at,firstExpiry);assert.equal(jar.size,2);
  const protectedFlow=[...jar].find(([name])=>name.endsWith('nt2-login-flow'))![1];
  const passwordAccount=await f.service.db.transaction(async tx=>{const id=await f.service.authentication!.accounts.provision(tx,'google','protected-password-flow','Password flow');await tx.query('UPDATE auth1_flows SET password_account_id=$2,password_revision=1 WHERE id=$1',[protectedFlow,id]);return id;});
  const reopened=await web('192.0.2.99',{cookie:[...jar].map(([k,v])=>k+'='+v).join('; ')});assert.equal(reopened.statusCode,200);const preserved=await f.service.db.transaction(async tx=>(await tx.query('SELECT password_account_id FROM auth1_flows WHERE id=$1',[protectedFlow])).rows[0]);assert.equal(preserved.password_account_id,passwordAccount,'ordinary login reuse cannot discard a forced password-change flow');
  assert.equal(signInSource('::ffff:192.0.2.1'),signInSource('192.0.2.1'));assert.equal(signInSource('2001:db8::1'),signInSource('2001:0db8:0:0:0:0:0:1'));
  console.log(JSON.stringify({webAbuseAccepted:32,appAbuseAccepted:32,otherSourceCanStart:true,sharedNat:32,ownedRetryAndCancellation:true,syntheticClock:true}));
 }finally{await f.close();}
});

test('AUTH1 authenticated password flows share source admission and retain CSRF/current-revision checks',async()=>{
 const f=await startAuth1Fixture();try{
  const a=f.service.authentication!,source='192.0.2.77',password='Original fixture password 123!',next='Updated fixture password 456!';
  const issued=await a.accounts.issueLocal(undefined,{username:'password-admission',operationId:randomUUID()},'isolated-regression');
  // Synthetic established local credential and issued browser; all requests use
  // the actual service routes and disposable SQL, with a synthetic clock.
  const encoded=await a.accounts.kdf(password);
  await f.service.db.transaction(tx=>tx.query('UPDATE auth1_local_credentials SET password_hash=$2,temporary_until=NULL,temporary_consumed=false WHERE account_id=$1',[issued.account.accountId,encoded]));
  const browser=await f.service.db.transaction(tx=>a.issueBrowser(tx,issued.account.accountId));
  const jar=new Map([[a.sessionCookie,browser.handle]]);
  const call=async(method:'GET'|'POST',url:string,payload?:Record<string,string>,ip=source)=>{
   const r=await f.service.app.inject({method,url,remoteAddress:ip,headers:{host:new URL(f.origin).host,cookie:[...jar].map(([k,v])=>k+'='+v).join('; '),...(payload?{origin:f.origin,'content-type':'application/x-www-form-urlencoded'}:{})},...(payload?{payload:new URLSearchParams(payload).toString()}:{})});
   for(const c of ([] as string[]).concat(r.headers['set-cookie']??[])){const pair=c.split(';')[0]!,at=pair.indexOf('=');if(pair.slice(at+1))jar.set(pair.slice(0,at),pair.slice(at+1));else jar.delete(pair.slice(0,at));}return r;
  };
  let accepted=0,first:any;
  for(let i=0;i<130;i++){f.advance(100);const r=await call('GET','/auth/password');if(r.statusCode===200){accepted++;first??=r;}else assert.equal(r.json().error.code,'CAPACITY',r.body);}
  assert.equal(accepted,SIGNIN_ADMISSION.outstandingPerSource);
  const rows=await f.service.db.transaction(async tx=>(await tx.query('SELECT count(*)::int AS total,count(*) FILTER(WHERE source_hash IS NULL)::int AS unbound FROM auth1_flows WHERE consumed_at IS NULL')).rows[0]);
  assert.deepEqual(rows,{total:32,unbound:0});
  const victim=await f.service.app.inject({method:'GET',url:'/auth/login/google',remoteAddress:'198.51.100.2',headers:{host:new URL(f.origin).host}});assert.equal(victim.statusCode,302,victim.body);
  const form={flowId:first.body.match(/name="flowId" value="([^"]+)"/)![1]!,csrf:first.body.match(/name="csrf" value="([^"]+)"/)![1]!,currentPassword:password,password:next};
  assert.equal((await call('POST','/auth/password',{...form,csrf:'invalid'})).json().error.code,'FORBIDDEN');
  assert.equal((await call('POST','/auth/password',{...form,currentPassword:'wrong password'})).json().error.code,'UNAUTHENTICATED');
  // A revision change between password verification and the committing
  // transaction cannot authorize a stale password update.
  const verify=a.accounts.localPassword.bind(a.accounts);a.accounts.localPassword=async(...args:Parameters<typeof verify>)=>{const verified=await verify(...args);await f.service.db.transaction(tx=>tx.query('UPDATE auth1_local_credentials SET revision=revision+1 WHERE account_id=$1',[issued.account.accountId]));return verified;};
  try{assert.equal((await call('POST','/auth/password',form)).json().error.code,'EXPIRED');}finally{a.accounts.localPassword=verify;}
  const originalBinding=jar.get(a.flowCookie(form.flowId))!;const changed=await call('POST','/auth/password',form);assert.equal(changed.statusCode,302,changed.body);assert.equal(changed.headers.location,'/#signed-in');
  assert.equal((await a.accounts.localPassword('password-admission',next)).account_id,issued.account.accountId);
  await assert.rejects(a.accounts.localPassword('password-admission',password),/UNAUTHENTICATED/);
  jar.set(a.flowCookie(form.flowId),originalBinding);assert.equal((await call('POST','/auth/password',form)).json().error.code,'EXPIRED','consumed flow cannot replay even with its original binding');
  console.log(JSON.stringify({passwordFlowsAccepted:accepted,allFlowsSourceBound:true,unrelatedGoogleLogin:true,csrfAndRevisionBinding:true,legitimatePasswordChange:true,syntheticBrowserAndClock:true}));
 }finally{await f.close();}
});

async function providerFamilies(f:Fixture,count:number){const a=f.service.authentication!;const rows=[];for(let i=0;i<count;i++)rows.push(await f.service.db.transaction(async tx=>{
 const account=await a.accounts.provision(tx,'twitch','cohort-'+i,'Cohort '+i),issued=await a.issueBrowser(tx,account,{subject:String(i),tokens:{access_token:'synthetic-token',expiresAt:f.service.auth.now()+7200000}});
 const row=(await tx.query('SELECT w.family_id,w.credentials_ciphertext,p.next_check_at,p.valid_until,p.checked_at FROM auth1_web_sessions w JOIN auth1_provider_sessions p ON p.family_id=w.family_id WHERE w.handle_hash=$1',[hash(issued.handle)])).rows[0];
 assert.ok(+row.next_check_at<=+row.checked_at+PROVIDER_SCHEDULE.checkMs+1000);assert.ok(+row.next_check_at<+row.valid_until-PROVIDER_SCHEDULE.marginMs,'actual credential issuance schedules validation early');
 return {family:row.family_id,credentials:f.service.auth.security.open<any>(row.credentials_ciphertext)};
}));return rows;}

test('AUTH1 provider worker drains a clustered 120-family cohort with bounded concurrency; failure backoff is fair and revoked families stay revoked',async()=>{
 const f=await startAuth1Fixture();try{
  const families=await providerFamilies(f,140),a=f.service.authentication!,seen=new Set<number>();let concurrent=0,maxConcurrent=0,calls=0,failures=true;
  a.providers.checkTwitch=async(subject,tokens)=>{const i=Number(subject);calls++;concurrent++;maxConcurrent=Math.max(maxConcurrent,concurrent);try{await new Promise(ok=>setTimeout(ok,25+i%3));if(i===139)throw new PolicyError('UNAUTHENTICATED');if(i>=120&&failures)throw new PolicyError('PROVIDER_UNAVAILABLE');seen.add(i);return {...tokens,expiresAt:f.service.auth.now()+7200000};}finally{concurrent--;}};
  f.advance(1800001);await a.providerTick();assert.equal(calls,128);assert.ok(maxConcurrent<=PROVIDER_SCHEDULE.concurrency);
  await a.providerTick();assert.equal(calls,140,'remaining due rows drain before backed-off failures');assert.equal(seen.size,120);assert.ok(Array.from({length:120},(_,i)=>i).every(i=>seen.has(i)));
  const state=await f.service.db.transaction(async tx=>(await tx.query('SELECT p.*,f.revoked_at AS family_revoked FROM auth1_provider_sessions p JOIN session_families f ON f.id=p.family_id')).rows);
  assert.equal(state.filter(r=>+r.valid_until<=f.service.auth.now()&&!r.revoked_at).length,0);assert.ok(state.find(r=>r.subject==='139')!.family_revoked);assert.ok(state.filter(r=>Number(r.subject)>=120&&Number(r.subject)<139).every(r=>r.failures===1&&+r.next_check_at>f.service.auth.now()));
  const before=calls;await a.providerTick();assert.equal(calls,before,'backoff prevents immediate retry loops');
  failures=false;f.advance(30001);await a.providerTick();assert.equal(seen.size,139);assert.ok(!seen.has(139));
  // A network success cannot race administrator revocation to restore a family.
  const victim=families[0]!;await f.service.db.transaction(tx=>tx.query('UPDATE auth1_provider_sessions SET next_check_at=$2 WHERE family_id=$1',[victim.family,new Date(f.service.auth.now())]));
  a.providers.checkTwitch=async(_subject,tokens)=>{await f.service.db.transaction(tx=>f.service.auth.revoke(tx,victim.family));return {...tokens,expiresAt:f.service.auth.now()+7200000};};await a.providerTick();
  await assert.rejects(f.service.db.transaction(tx=>f.service.auth.authenticate(tx,victim.credentials.accessToken!)),/UNAUTHENTICATED/);
  const expired=families[1]!,until=f.service.auth.now()+1000;await f.service.db.transaction(tx=>tx.query('UPDATE auth1_provider_sessions SET next_check_at=$2,valid_until=$3 WHERE family_id=$1',[expired.family,new Date(f.service.auth.now()),new Date(until)]));a.providers.checkTwitch=async()=>{throw new PolicyError('PROVIDER_UNAVAILABLE');};await a.providerTick();f.advance(1001);
  await assert.rejects(f.service.db.transaction(tx=>f.service.auth.refresh(tx,{requestId:randomUUID(),refresh:{secret:expired.credentials.refreshSecret!,deviceId:expired.credentials.deviceId!}})),/UNAUTHENTICATED/,'provider outage never extends validation expiry');
  await f.service.db.transaction(tx=>tx.query("UPDATE auth1_provider_sessions SET next_check_at=$1 WHERE subject::int BETWEEN 2 AND 6",[new Date(f.service.auth.now())]));
  let entered=0,release!:()=>void,started!:()=>void;const gate=new Promise<void>(r=>release=r),ready=new Promise<void>(r=>started=r);
  a.providers.checkTwitch=async(_subject,tokens)=>{entered++;if(entered===2)started();await gate;return {...tokens,expiresAt:f.service.auth.now()+7200000};};const work=a.providerTick();let timer:ReturnType<typeof setTimeout>|undefined;
  try{await Promise.race([ready,new Promise((_,reject)=>{timer=setTimeout(()=>reject(Error('Provider concurrency did not start')),5000);})]);const info=await f.service.app.inject({method:'GET',url:'/v1/auth/info',headers:{host:new URL(f.origin).host}});assert.equal(info.statusCode,200,'provider backlog does not hold HTTP readiness');const stopping=a.providerIdle();release();await stopping;await work;assert.equal(entered,2,'shutdown drains in-flight requests without starting the remaining backlog');}
  finally{if(timer)clearTimeout(timer);release();await work;}
  console.log(JSON.stringify({syntheticProviderAndClock:true,healthyCohort:120,totalWithFailures:140,maximumConcurrency:maxConcurrent,noHealthyStarvation:true,capacityCertification:false}));
 }finally{await f.close();}
});

test('AUTH1 production rejoin sweep bounds expired retention, preserves live credentials/refresh replay fences and compacts routine snapshots only',async()=>{
 const f=await startAuth1Fixture();try{
  const a=f.service.authentication!,auth=f.service.auth,now=auth.now();
  const c=await f.service.db.transaction(async tx=>{const id=await a.accounts.provision(tx,'google','retention','Retention');return auth.family(tx,id,'retention-device','desktop');});
  const actor=await f.service.db.transaction(tx=>auth.authenticate(tx,c.accessToken!));const fp='a'.repeat(64);
  const refreshed=await f.service.db.transaction(tx=>auth.refresh(tx,{requestId:'rotate-once',refresh:{secret:c.refreshSecret!,deviceId:c.deviceId!}}));assert.ok(refreshed.credentials);
  const exactRetry=await f.service.db.transaction(tx=>auth.refresh(tx,{requestId:'rotate-once',refresh:{secret:c.refreshSecret!,deviceId:c.deviceId!}}));assert.equal(exactRetry.credentials?.accessToken,refreshed.credentials!.accessToken);
  const abandoned=await f.service.db.transaction(async tx=>{const credential=await auth.family(tx,c.accountId!,'abandoned-browser','browser');const family=(await tx.query('SELECT family_id FROM sessions WHERE id=$1',[credential.sessionId])).rows[0].family_id;await tx.query('UPDATE access_tokens SET expires_at=$2 WHERE session_id=$1',[credential.sessionId,new Date(now-31*60000)]);return {session:credential.sessionId,family};});
  await f.service.db.transaction(async tx=>{
   await tx.query("INSERT INTO auth_requests(scope,request_id,fingerprint,recovery_ciphertext,recovery_until) SELECT 'old-source',i::text,$1,'expired-private-recovery',$2 FROM generate_series(1,300) i",[fp,new Date(now-7200000)]);
   await tx.query("INSERT INTO auth_requests VALUES('current-source','live',$1,'current-private-recovery',$2)",[fp,new Date(now+60000)]);
   await tx.query("INSERT INTO account_signin_attempts(id,device_id,app_instance_id,provider,proof_hash,callback_port,callback_state_hash,return_context_ciphertext,state,expires_at,service_epoch,created_at) SELECT gen_random_uuid(),'old-device',i::text,'local',$1,33333,$1,'old-context','expired',$2,$3,$2 FROM generate_series(1,128) i",[fp,new Date(now-7200000),f.service.db.epoch]);
   await tx.query("INSERT INTO command_results(scope,request_id,fingerprint,result,created_at,operation) SELECT $1,'old-pulse-'||i,$2,'{}',$3,'routine' FROM generate_series(1,300) i",[actor.sessionId,fp,new Date(now-2*86400000)]);
   await tx.query("INSERT INTO command_results(scope,request_id,fingerprint,result,created_at,operation) VALUES($1,'durable-world-transfer-fence',$2,'{}',$3,'foundation')",[actor.sessionId,fp,new Date(now-2*86400000)]);
   await tx.query('INSERT INTO access_tokens(id,session_id,expires_at) SELECT gen_random_uuid(),$1,$2 FROM generate_series(1,12)',[actor.sessionId,new Date(now-100000)]);
   const result:any={room:{roomId:randomUUID(),runId:randomUUID(),members:Array.from({length:32},()=>({accountId:randomUUID()}))}};
   const returned=await f.service.rooms.record(tx,{presence:{present:true},requestId:'small-pulse'},actor,fp,result);assert.equal(returned,result);
   const stored=(await tx.query("SELECT operation,result FROM command_results WHERE request_id='small-pulse'")).rows[0];assert.equal(stored.operation,'routine');const opened=auth.security.open<any>(stored.result.sealed);assert.deepEqual(Object.keys(opened.room).sort(),['roomId','runId']);
  });
  f.advance(31000);await f.service.rooms.sweep();
  const counts=()=>f.service.db.transaction(async tx=>(await tx.query("SELECT (SELECT count(*)::int FROM auth_requests WHERE scope='old-source') AS old_auth,(SELECT count(*)::int FROM command_results WHERE operation='routine' AND request_id LIKE 'old-pulse-%') AS old_pulse,(SELECT count(*)::int FROM account_signin_attempts WHERE state='expired') AS old_attempts")).rows[0]);assert.deepEqual(await counts(),{old_auth:44,old_pulse:44,old_attempts:0});
  await f.service.rooms.sweep();assert.deepEqual(await counts(),{old_auth:0,old_pulse:0,old_attempts:0});
  await f.service.db.transaction(async tx=>{assert.equal((await tx.query('SELECT 1 FROM access_tokens WHERE session_id=$1',[abandoned.session])).rowCount,1,'expired browser watermark survives compaction');assert.equal(await auth.reclaimAbandonedBrowser(tx,c.accountId!),abandoned.family);});
  await f.service.db.transaction(async tx=>{assert.equal((await tx.query("SELECT 1 FROM command_results WHERE request_id='durable-world-transfer-fence'")).rowCount,1);assert.equal((await tx.query("SELECT 1 FROM auth_requests WHERE scope='current-source'")).rowCount,1);const old=(await tx.query('SELECT * FROM refresh_tokens WHERE hash=$1',[hash(c.refreshSecret!)])).rows[0];assert.ok(old.spent_at);assert.equal(old.recovery_ciphertext,null);assert.ok((await auth.authenticate(tx,refreshed.credentials!.accessToken!)).accountId);});
  const replay=await f.service.db.transaction(tx=>auth.refresh(tx,{requestId:'different-original',refresh:{secret:c.refreshSecret!,deviceId:c.deviceId!}}));assert.equal(replay.error?.code,'REFRESH_REUSED','spent hash still revokes replay');
  console.log(JSON.stringify({actualProductionSweep:true,boundedPerTable:256,expiredAnonymousRowsRemoved:true,activeCredentialsPreserved:true,refreshReplayFencePreserved:true,durableCommandFencesUntouched:true,syntheticClock:true}));
 }finally{await f.close();}
});

test('AUTH1 retention leaves an unresolved real bank original and proof-bound world associations recoverable',async()=>{
 const f=await b1Fixture();try{
  const original={goldIntent:{operationId:randomUUID(),direction:'deposit',amount:7}},pending=(await f.ok(0,original)).operation;assert.equal(pending.state,'awaiting_local');
  const db=f.x.fixture.service.db,auth=f.x.fixture.service.auth;
  const read=()=>db.transaction(async tx=>({operation:(await tx.query('SELECT * FROM g3_operations WHERE operation_id=$1',[original.goldIntent.operationId])).rows,associations:(await tx.query('SELECT world_id,high_water,generation,owner_attempt,terminal_reason FROM world_associations WHERE run_id=$1 ORDER BY world_id',[f.x.room.runId])).rows}));
  const before=await read();assert.equal(before.operation.length,1);assert.equal(before.associations.length,2);
  await db.transaction(tx=>tx.query("INSERT INTO command_results(scope,request_id,fingerprint,result,created_at,operation) VALUES($1,'expired-routine',$2,'{}',$3,'routine')",[f.x.desktop.sessionId,'a'.repeat(64),new Date(auth.now()-86400000)]));
  await f.x.fixture.service.rooms.sweep();assert.deepEqual(await read(),before);assert.equal((await f.ok(0,original)).operation.deliveryId,pending.deliveryId);
  const receipt=f.receipt(pending,'escrowed'),done=(await f.ok(0,receipt)).operation;assert.equal(done.state,'completed');assert.equal(done.balance,7);assert.equal((await f.ok(0,receipt)).operation.balance,7);
  console.log(JSON.stringify({actualTlsSqlLifecycle:true,unresolvedOriginalPreserved:true,worldAssociationsPreserved:true,receiptAppliesOnceAfterCleanup:true,engineWalletFacts:'synthetic'}));
 }finally{await f.close();}
});
