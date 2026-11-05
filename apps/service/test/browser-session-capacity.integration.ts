// Isolated policy candidate. Existing production/fixture/pinned sources unchanged.
import test from 'node:test';import assert from 'node:assert/strict';import {randomUUID} from 'node:crypto';
import {startG3Fixture} from '../scripts/g3-loot01-fixture.js';import {installBrowserSessionCandidate,reclaimAbandonedBrowserCandidate,BROWSER_ABANDONED_GRACE_MS} from './candidates/browser-session-policy.js';
type Fixture=Awaited<ReturnType<typeof startG3Fixture>>;
async function run(label:string,candidate:boolean,fn:(f:Fixture,advance:(ms:number)=>void)=>Promise<void>){let now=Date.now();const f=await startG3Fixture({databaseUrl:'postgres://nt2_g1:local-fixture-only@127.0.0.1:54328/nt2_g1',rejoin:true,loot01Runtime:true,live:true,tls:true,now:()=>now,...(candidate?{configure:installBrowserSessionCandidate}:{})});try{await fn(f,ms=>now+=ms);console.log(JSON.stringify({browserCapacity:label,candidate,schema:f.namespace,passed:true}));}finally{await f.close();}}
const body=(subject='twitch-local-alex')=>({subject,deviceId:'browser-'+randomUUID(),requestId:randomUUID()});
const login=async(f:Fixture,b=body())=>JSON.parse((await f.call('/v1/mock/browser-login',b)).body);
const families=(f:Fixture,accountId:string)=>f.service.db.transaction(async tx=>(await tx.query('SELECT id,kind,revoked_at FROM session_families WHERE account_id=$1 ORDER BY id',[accountId])).rows);

test('baseline current browser sessions accumulate8 across page instances and repeat logins; short retry cannot free them',()=>run('baseline-eight-families',false,async(f,advance)=>{
 let account='';for(let i=0;i<8;i++){const result=await login(f);assert.equal(result.error,undefined);account=result.credentials.accountId;}
 assert.equal((await login(f)).error.code,'CAPACITY');advance(60000);assert.equal((await login(f)).error.code,'CAPACITY');assert.equal((await families(f,account)).filter(r=>!r.revoked_at).length,8);
 const website=await login(f,body('website-local-alex'));assert.equal(website.error,undefined);assert.notEqual(website.credentials.accountId,account);
}));

test('candidate reclaims only one abandoned same-account browser and preserves active browser/desktop/game plus another account',()=>run('abandoned-versus-active-boundaries',true,async(f,advance)=>{
 const browsers=[];for(let i=0;i<8;i++)browsers.push((await login(f)).credentials);const account=browsers[0].accountId;
 const other=(await login(f,body('website-local-alex'))).credentials;
 const dependent=await f.service.db.transaction(async tx=>{const desktop=await f.service.auth.family(tx,account,'desktop-independent','desktop'),family=(await tx.query('SELECT family_id FROM sessions WHERE id=$1',[desktop.sessionId])).rows[0].family_id,sid=randomUUID();await tx.query("INSERT INTO sessions(id,family_id,kind,epoch,parent_id) VALUES($1,$2,'game',$3,$4)",[sid,family,randomUUID(),desktop.sessionId]);return {desktop,family,sid};});
 // All initial access credentials are now expired beyond the candidate grace.
 advance(900000+BROWSER_ABANDONED_GRACE_MS+1000);
 // A genuinely active browser renews before new allocation; its slot must survive.
 const request={major:1,revision:'g2-draft-5',serviceId:f.service.db.serviceId,serviceEpoch:f.service.db.epoch,requestId:randomUUID(),refresh:{secret:browsers[0].refreshSecret,deviceId:browsers[0].deviceId}};
 const active=JSON.parse((await f.call('/v1/rpc',request)).body).credentials;assert.ok(active.accessToken);
 const created=await login(f);assert.equal(created.error,undefined);const rows=await families(f,account);assert.equal(rows.filter(r=>r.kind==='browser'&&!r.revoked_at).length,8);assert.equal(rows.filter(r=>r.kind==='browser'&&r.revoked_at).length,1);
 const kept=await f.service.db.transaction(async tx=>(await tx.query('SELECT s.id,s.revoked_at,f.revoked_at AS family_revoked FROM sessions s JOIN session_families f ON f.id=s.family_id WHERE s.id=ANY($1::uuid[])',[[active.sessionId,dependent.desktop.sessionId,dependent.sid,other.sessionId]])).rows);assert.equal(kept.length,4);assert.ok(kept.every(r=>!r.revoked_at&&!r.family_revoked));
}));

test('candidate preserves8 recently usable browsers and only becomes eligible after the exact expiry grace',()=>run('active-cap-and-grace',true,async(f,advance)=>{
 const logins=[];for(let i=0;i<8;i++)logins.push((await login(f)).credentials);const account=logins[0].accountId;
 assert.equal((await login(f)).error.code,'CAPACITY');advance(900000+BROWSER_ABANDONED_GRACE_MS-2000);assert.equal((await login(f)).error.code,'CAPACITY');assert.ok((await families(f,account)).every(r=>!r.revoked_at));advance(4000);assert.equal((await login(f)).error,undefined);assert.equal((await families(f,account)).filter(r=>r.revoked_at).length,1);
}));

test('same original browser login after a lost reply returns the exact issued credentials without consuming another slot',()=>run('lost-reply-original',true,async f=>{
 const original=body();const committed=await login(f,original);const recovered=await login(f,original);assert.deepEqual(recovered.credentials,committed.credentials);assert.deepEqual(recovered.credentialValidity,committed.credentialValidity);assert.equal((await families(f,committed.credentials.accountId)).length,1);
 const conflicting=await login(f,{...original,subject:'website-local-alex'});assert.equal(conflicting.error.code,'CONFLICT');assert.equal((await families(f,committed.credentials.accountId)).length,1);
}));

test('concurrent tabs serialize at cap and concurrent duplicate originals do not create duplicate families',()=>run('concurrent-tabs-and-originals',true,async f=>{
 const original=body(),duplicates=await Promise.all([login(f,original),login(f,original)]);assert.deepEqual(duplicates[0].credentials,duplicates[1].credentials);const account=duplicates[0].credentials.accountId;
 for(let i=0;i<6;i++)assert.equal((await login(f)).error,undefined);
 const contenders=await Promise.all([login(f),login(f)]);assert.equal(contenders.filter(v=>!v.error).length,1);assert.equal(contenders.filter(v=>v.error?.code==='CAPACITY').length,1);assert.equal((await families(f,account)).filter(r=>!r.revoked_at).length,8);
}));

test('refresh winning family lock survives reclamation started with an older token snapshot',()=>run('refresh-lock-snapshot-recheck',true,async(f,advance)=>{
 const issued=[];for(let i=0;i<8;i++)issued.push((await login(f)).credentials);const account=issued[0].accountId;advance(900000+BROWSER_ABANDONED_GRACE_MS+1000);
 const refresh=await f.service.db.pool.connect(),reclaim=await f.service.db.pool.connect();let work:Promise<string|null>|undefined;
 try{
  await refresh.query('BEGIN');await reclaim.query('BEGIN');
  const oldest=(await refresh.query("SELECT f.id,s.id AS session_id FROM session_families f JOIN sessions s ON s.family_id=f.id WHERE f.account_id=$1 AND f.kind='browser' ORDER BY f.id LIMIT 1",[account])).rows[0];const credential=issued.find(c=>c.sessionId===oldest.session_id)!;
  await refresh.query('SELECT id FROM session_families WHERE id=$1 FOR UPDATE',[oldest.id]);await reclaim.query('SELECT id FROM accounts WHERE id=$1 FOR UPDATE',[account]);const pid=(await reclaim.query('SELECT pg_backend_pid() AS pid')).rows[0].pid;
  work=reclaimAbandonedBrowserCandidate(f.service,reclaim,account,f.service.auth.now());void work.catch(()=>{});
  let blocked=false;for(let i=0;i<25;i++){const row=(await f.service.db.pool.query("SELECT wait_event_type FROM pg_stat_activity WHERE pid=$1 AND datname=current_database()",[pid])).rows[0];if(row?.wait_event_type==='Lock'){blocked=true;break;}await new Promise(r=>setTimeout(r,5));}assert.equal(blocked,true,'Reclaim must actually wait behind the owned refresh family lock');
  const renewed=await f.service.auth.refresh(refresh,{requestId:randomUUID(),refresh:{secret:credential.refreshSecret,deviceId:credential.deviceId}});assert.ok(renewed.credentials);await refresh.query('COMMIT');
  const reclaimed=await work;await reclaim.query('COMMIT');assert.ok(reclaimed);assert.notEqual(reclaimed,oldest.id);
  await f.service.db.transaction(async tx=>{await f.service.auth.authenticate(tx,renewed.credentials!.accessToken!);assert.equal((await tx.query('SELECT revoked_at FROM session_families WHERE id=$1',[oldest.id])).rows[0].revoked_at,null);});
 }finally{await Promise.allSettled([refresh.query('ROLLBACK'),reclaim.query('ROLLBACK')]);await work?.catch(()=>{});refresh.release();reclaim.release();}
}));
