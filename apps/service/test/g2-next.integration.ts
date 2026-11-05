import test from 'node:test';
import assert from 'node:assert/strict';
import {randomUUID,randomBytes} from 'node:crypto';
import {readFile} from 'node:fs/promises';
import pg from 'pg';
import {generateKeyPair,exportPKCS8,exportSPKI} from 'jose';
import {CandidateDatabase as Database} from '../src/g2-next/database.js';
import {CandidateRooms} from '../src/g2-next/rooms.js';
import {Security,hash} from '../src/g1/security.js';
import {MockSignIn} from '../src/g2-next/signin.js';
import {PrincipalAuth} from '../src/g2-next/auth.js';
import {InspectionAuthority} from '../src/g2-next/inspection.js';
import type {Actor} from '../src/g1/auth.js';
import type {Request,GameObservation} from '../../../packages/contracts/g2-next/g2-next.js';
const url=process.env.NT2_G2_DATABASE_URL??'postgres://nt2_g1:local-fixture-only@127.0.0.1:54328/nt2_g1';
async function fixture(){
 const namespace='g2_next_test_'+randomUUID().replaceAll('-',''),db=new Database(url,'nt2-g2-next-test',namespace,{},true),pair=await generateKeyPair('EdDSA',{extractable:true});let now=Date.now();
 const security=new Security({privateKey:await exportPKCS8(pair.privateKey),publicKey:await exportSPKI(pair.publicKey),recoveryKey:randomBytes(32).toString('base64url'),issuer:'https://candidate.invalid'},db.serviceId),auth=new PrincipalAuth(db,security,()=>now,true);
 await db.migrate();await db.start();
 const authority=new InspectionAuthority(db,auth,{gameVersion:'test',modVersion:'test',bridgeVersion:'test',capabilities:['core.seed']});
 const rooms=new CandidateRooms(db,auth,{gameVersion:'test',modVersion:'test',bridgeVersion:'test',capabilities:['core.seed'],modes:['core'],settingKeys:[]});
 return {db,auth,authority,rooms,advance(ms:number){now+=ms;},async actor(token:string){return db.transaction(tx=>auth.authenticate(tx,token));},async close(){await db.close();const pool=new pg.Pool({connectionString:url});try{await pool.query(`DROP SCHEMA "${namespace}" CASCADE`);}finally{await pool.end();}}};
}
test('candidate guest issue separates duplicate names, exact recovery, lifetime and host authority',async()=>{
 const f=await fixture();try{
  const input={deviceId:'desktop-a',displayName:'Alex',issuanceProof:randomBytes(32).toString('base64url')},request=randomUUID();
  const a=await f.db.transaction(tx=>f.auth.guest(tx,input,request));const retry=await f.db.transaction(tx=>f.auth.guest(tx,input,request));assert.deepEqual(retry,a);
  const b=await f.db.transaction(tx=>f.auth.guest(tx,{...input,deviceId:'desktop-b',issuanceProof:randomBytes(32).toString('base64url')},randomUUID()));assert.notEqual(a.accountId,b.accountId);assert.equal(a.principal?.kind,'guest');assert.equal(a.principal?.displayName,b.principal?.displayName);
  await assert.rejects(f.db.transaction(tx=>f.auth.family(tx,a.accountId!,input.deviceId,'browser')),e=>(e as {code:string}).code==='FORBIDDEN');
  await assert.rejects(f.db.transaction(tx=>tx.query("INSERT INTO rooms(id,name,host_account_id,state,config) VALUES($1,'guest host',$2,'waiting',$3)",[randomUUID(),a.accountId,{seed:1,mode:'core'}])));
  f.advance(30000);await assert.rejects(f.db.transaction(tx=>f.auth.guest(tx,input,request)),e=>(e as {code:string}).code==='EXPIRED');
  f.advance(86400000);await assert.rejects(f.actor(a.accessToken!));
 }finally{await f.close();}
});
test('candidate loaded observations remain protected staging; all members inspect; host Start binds report',async()=>{
 const f=await fixture();try{
  const id=await f.db.transaction(tx=>f.auth.mockRegistered(tx,'website','host','Host'));
  const hostCreds=await f.db.transaction(tx=>f.auth.family(tx,id,'browser','browser')),host=await f.actor(hostCreds.accessToken!);
  const guestCreds=await f.db.transaction(tx=>f.auth.guest(tx,{deviceId:'guest',displayName:'Guest',issuanceProof:randomBytes(32).toString('base64url')},randomUUID())),guest=await f.actor(guestCreds.accessToken!);
  const roomId=randomUUID();await f.db.transaction(async tx=>{await tx.query("INSERT INTO rooms(id,name,host_account_id,state,config,allow_guests) VALUES($1,'inspection',$2,'waiting',$3,true)",[roomId,id,{seed:424242,mode:'core',requiredCapabilities:['core.seed']}]);await tx.query('INSERT INTO memberships(desktop_id,room_id,account_id,presence_until,ready_config_revision) VALUES($1,$2,$3,$4,1)',[guest.sessionId,roomId,guest.accountId,new Date(f.auth.now()+30000)]);});
  let runId='',runEpoch='';
  const request=async(actor:Actor):Promise<Request>=>f.db.transaction(async tx=>{const r=(await tx.query('SELECT * FROM rooms WHERE id=$1',[roomId])).rows[0];return {major:1,revision:'g2-draft-5',serviceId:f.db.serviceId,serviceEpoch:f.db.epoch,sessionEpoch:actor.sessionEpoch,roomId,requestId:randomUUID(),configRevision:1,expectedRevision:r.revision,runId,runEpoch};});
  const prepared=await f.db.transaction(tx=>f.authority.prepare(tx,host,{serviceId:f.db.serviceId,serviceEpoch:f.db.epoch,sessionEpoch:host.sessionEpoch,roomId,configRevision:1,expectedRevision:1}));runId=prepared.runId;runEpoch=prepared.runEpoch;
  const ownReq=await request(guest);const own=await f.db.transaction(tx=>f.authority.prepare(tx,guest,{...ownReq,prepare:{}}));
  const attachReq=await request(guest);attachReq.gameAttach={preparationId:own.preparation!.preparationId!,gameInstanceId:'game',gameVersion:'test',modVersion:'test',bridgeVersion:'test',capabilities:['core.seed']};
  const gameCreds=await f.db.transaction(tx=>f.authority.attach(tx,guest,attachReq)),game=await f.actor(gameCreds.accessToken!);
  const observation:GameObservation={attachmentId:gameCreds.attachmentId!,gameInstanceId:'game',worldToken:'world-a',observationRevision:1,actualSeed:'424242',seedStatus:'known',collectorFailure:'',modVersion:'test',bridgeVersion:'test',compatibilityProvenance:'game-thread-client-observation',activeMods:['noita_together2','different-mod'],inventoryComplete:true,beta:'unknown',protected:true,loaded:true};
  const observeReq=await request(game);const captured=await f.db.transaction(tx=>f.authority.observe(tx,game,observeReq,observation));f.advance(1000);
  const duplicate=await f.db.transaction(tx=>f.authority.observe(tx,game,observeReq,observation));assert.equal(+duplicate.captured_at,+captured.captured_at);
  const pulse=await request(game);pulse.gamePulse={attachmentId:gameCreds.attachmentId!,gameInstanceId:'game',worldToken:'world-a'};await f.db.transaction(tx=>f.authority.pulse(tx,game,pulse));
  const listReq=await request(guest);listReq.inspectionList={limit:1};const reports=await f.db.transaction(tx=>f.authority.list(tx,guest,listReq));assert.equal(reports.inspections[0].capturedAt,captured.captured_at.toISOString());assert.deepEqual(reports.inspections[0].activeMods,observation.activeMods);
  const state=await f.db.transaction(tx=>tx.query('SELECT state FROM rooms WHERE id=$1',[roomId]));assert.equal(state.rows[0].state,'staging');
  const participant={desktopId:guest.sessionId,attachmentId:gameCreds.attachmentId!,worldToken:'world-a',observationRevision:1,contentDigest:captured.content_digest};
  await assert.rejects(f.db.transaction(tx=>f.authority.start(tx,guest,{...listReq,requestId:randomUUID()},{participants:[participant]})),e=>(e as {code:string}).code==='FORBIDDEN');
  const startReq=await request(host);await assert.rejects(f.db.transaction(tx=>f.authority.start(tx,host,startReq,{participants:[{...participant,contentDigest:'wrong'}]})),e=>(e as {code:string}).code==='REPORT_CHANGED');
  await f.db.transaction(tx=>f.authority.start(tx,host,startReq,{participants:[participant],excludedDesktopIds:[]}));await f.db.transaction(tx=>f.authority.start(tx,host,startReq,{participants:[participant],excludedDesktopIds:[]}));
  assert.equal((await f.db.transaction(tx=>tx.query('SELECT state FROM rooms WHERE id=$1',[roomId]))).rows[0].state,'running');
  const changed={...observation,observationRevision:2,activeMods:['changed'],protected:false};await f.db.transaction(tx=>f.authority.observe(tx,game,{...observeReq,requestId:randomUUID()},changed));
  assert.equal((await f.db.transaction(tx=>tx.query('SELECT state FROM rooms WHERE id=$1',[roomId]))).rows[0].state,'suspended');
 }finally{await f.close();}
});

test('candidate guest policy is default off; failed admission preserves membership; disabling does not evict',async()=>{
 const f=await fixture();try{
  const id=await f.db.transaction(tx=>f.auth.mockRegistered(tx,'website','policy-host','Host')),hostCreds=await f.db.transaction(tx=>f.auth.family(tx,id,'browser','browser')),host=await f.actor(hostCreds.accessToken!);
  const issued=await f.db.transaction(tx=>f.auth.guest(tx,{deviceId:'guest',displayName:'Alex',issuanceProof:randomBytes(32).toString('base64url')},randomUUID())),guest=await f.actor(issued.accessToken!);
  const req=(actor:Actor,extra:Partial<Request>):Request=>({major:1,revision:'g2-draft-5',serviceId:f.db.serviceId,serviceEpoch:f.db.epoch,sessionEpoch:actor.sessionEpoch,requestId:randomUUID(),...extra});
  const config={seed:1,mode:'core',requiredCapabilities:['core.seed']};
  const create=async(allowGuests?:boolean)=>f.db.transaction(tx=>f.rooms.execute(tx,req(host,{roomCreate:{name:'policy',config,...(allowGuests===undefined?{}:{allowGuests})}}),host));
  const off=(await create()).room!,on=(await create(true)).room!;assert.equal(off.allowGuests,false);assert.equal(on.allowGuests,true);
  const join=(room:typeof off)=>f.db.transaction(tx=>f.rooms.execute(tx,req(guest,{roomId:room.roomId!,expectedRevision:room.revision!,configRevision:1,roomJoin:{}}),guest));
  await assert.rejects(join(off),e=>(e as {code:string}).code==='ACCOUNT_REQUIRED');const admitted=(await join(on)).room!;
  await assert.rejects(join(off),e=>(e as {code:string}).code==='ACCOUNT_REQUIRED');
  assert.equal((await f.db.transaction(tx=>tx.query('SELECT room_id FROM memberships WHERE desktop_id=$1',[guest.sessionId]))).rows[0].room_id,on.roomId);
  const policy=req(host,{roomId:on.roomId!,expectedRevision:admitted.revision!,configRevision:1,roomPolicy:{capacity:32,allowGuests:false}});const disabled=(await f.db.transaction(tx=>f.rooms.execute(tx,policy,host))).room!;assert.equal(disabled.allowGuests,false);assert.equal(disabled.members?.length,1);
  assert.equal((await join(disabled)).room!.members?.[0]?.principal?.kind,'guest');
 }finally{await f.close();}
});
test('candidate mock sign-in status cannot redeem; exchange needs grant and initiator, retries once, cancellation and restart revoke',async()=>{
 const f=await fixture();try{
  const signin=new MockSignIn(f.auth),initiatorProof=randomBytes(32).toString('base64url'),callbackState=randomBytes(32).toString('base64url');
  const begin=()=>f.db.transaction(tx=>signin.begin(tx,{deviceId:'desktop',appInstanceId:'app',provider:'twitch',proofChallenge:hash(initiatorProof),callbackPort:49152,callbackState},randomUUID()));
  const a=await begin(),proof={attemptId:a.attemptId,deviceId:'desktop',appInstanceId:'app',initiatorProof};
  const completion=await f.db.transaction(tx=>signin.complete(tx,a.attemptId,'alex','Alex',randomUUID()));assert.equal(completion.url,'http://127.0.0.1:49152/nt2-return');assert.equal(completion.body.callbackState,callbackState);
  const status=await f.db.transaction(tx=>signin.status(tx,proof));assert.equal(status.state,'completed');assert.equal('completionGrant' in status,false);assert.equal('credentials' in status,false);
  await assert.rejects(f.db.transaction(tx=>signin.exchange(tx,{...proof,completionGrant:'wrong'},randomUUID())),e=>(e as {code:string}).code==='UNAUTHENTICATED');
  await assert.rejects(f.db.transaction(tx=>signin.exchange(tx,{...proof,initiatorProof:'wrong',completionGrant:completion.body.completionGrant},randomUUID())),e=>(e as {code:string}).code==='UNAUTHENTICATED');
  const input={...proof,completionGrant:completion.body.completionGrant},request=randomUUID(),issued=await f.db.transaction(tx=>signin.exchange(tx,input,request));assert.equal(issued.principal?.source,'mock:twitch');assert.deepEqual(await f.db.transaction(tx=>signin.exchange(tx,input,request)),issued);
  await assert.rejects(f.db.transaction(tx=>signin.exchange(tx,input,randomUUID())),e=>(e as {code:string}).code==='ALREADY_EXCHANGED');
  const b=await begin(),bProof={...proof,attemptId:b.attemptId};await f.db.transaction(tx=>signin.cancel(tx,bProof));await assert.rejects(f.db.transaction(tx=>signin.complete(tx,b.attemptId,'alex','Alex',randomUUID())));
  const c=await begin(),oldEpoch=f.db.epoch;await f.db.close();const restarted=new Database(url,f.db.serviceId,f.db.namespace,{},true);try{await restarted.migrate();await restarted.start();assert.notEqual(restarted.epoch,oldEpoch);const resumed=new MockSignIn(new PrincipalAuth(restarted,f.auth.security,f.auth.now,true));await assert.rejects(restarted.transaction(tx=>resumed.status(tx,{...proof,attemptId:c.attemptId})),e=>(e as {code:string}).code==='STALE_EPOCH');assert.equal((await restarted.transaction(tx=>tx.query('SELECT state FROM account_signin_attempts WHERE id=$1',[c.attemptId]))).rows[0].state,'cancelled');}finally{await restarted.close();}
 }finally{await f.close();}
});
