import test from 'node:test';
import assert from 'node:assert/strict';
import {randomUUID,randomBytes,generateKeyPairSync} from 'node:crypto';
import pg from 'pg';
import {createCandidateApp} from '../src/g2-next/app.js';
import {inspectionMetadata} from '../src/g2-next/profile.js';
import {schema,validate,encode,decode} from '../../../packages/contracts/src/g2-next.js';
import type {Credentials,Room,Request,Response} from '../../../packages/contracts/g2-next/g2-next.js';
test('source-built candidate Info/negotiation is isolated and rejects historical attachment without consuming the grant',async()=>{
 const databaseUrl=process.env.NT2_G2_DATABASE_URL??'postgres://nt2_g1:local-fixture-only@127.0.0.1:54328/nt2_g1',namespace='g2_next_profile_'+randomUUID().replaceAll('-',''),pair=generateKeyPairSync('ed25519'),metadata=inspectionMetadata(true);
 const options={databaseUrl,namespace,serviceId:'nt2-g2-next-profile-fixture',mockEnabled:true,website:true,keys:{privateKey:pair.privateKey.export({format:'pem',type:'pkcs8'}).toString(),publicKey:pair.publicKey.export({format:'pem',type:'spki'}).toString(),recoveryKey:randomBytes(32).toString('base64url'),issuer:'http://127.0.0.1:4356'},...metadata};
 await assert.rejects(createCandidateApp({...options,databaseUrl:'invalid',profile:{...metadata.profile,id:'nt2-g2-native-1'}}),/inspection candidate profile/);
 await assert.rejects(createCandidateApp({...options,databaseUrl:'invalid',profile:{id:'g2-next-simulated',evidenceStatus:'simulated-game-facts'}}),/inspection candidate profile/);
 const service=await createCandidateApp(options);try{
  const info=(await service.app.inject({method:'GET',url:'/v1/info'})).json();validate('Info',info);const {settingKeys,...wireCompatibility}=info.compatibility;assert.deepEqual(decode('Info',encode('Info',info)),{...info,compatibility:wireCompatibility});assert.equal(info.revision,'g2-draft-5');assert.equal(info.serviceId,options.serviceId);assert.equal(info.profile.id,metadata.profile.id);assert.equal(info.profile.bridgeSha256,metadata.profile.bridgeSha256);assert.equal(info.compatibility.modVersion,'nt2-g2-inspect-1');assert.deepEqual(info.capabilities,['core.seed']);assert.deepEqual(info.compatibility.modes,['core']);
  let room:Room|undefined;
  const rpc=async(c:Credentials|undefined,body:Partial<Request>)=>{const input:Request={major:1,revision:schema.revision,serviceId:service.db.serviceId,serviceEpoch:service.db.epoch,requestId:randomUUID(),...(c?{sessionEpoch:c.sessionEpoch!}:{}),...(room?{roomId:room.roomId!,expectedRevision:room.revision!,configRevision:room.configRevision!,runId:room.runId??'',runEpoch:room.runEpoch??''}:{}),...body};const response=(await service.app.inject({method:'POST',url:'/v1/rpc',headers:c?{authorization:'Bearer '+c.accessToken}:{},payload:input})).json() as Response;if(response.room)room=response.room;return response;};
  const ok=async(c:Credentials|undefined,body:Partial<Request>)=>{const r=await rpc(c,body);assert.equal(r.error,undefined,r.error?.code);return r;};
  const host=(await service.app.inject({method:'POST',url:'/v1/mock/browser-login',payload:{subject:'website-local-alex',deviceId:'host',requestId:randomUUID()}})).json().credentials as Credentials;
  await ok(host,{roomCreate:{name:'isolated native profile',allowGuests:true,config:{seed:424242,mode:'core',requiredCapabilities:['core.seed']}}});const desktop=(await ok(undefined,{guestIssue:{deviceId:'candidate',displayName:'Candidate',issuanceProof:randomBytes(32).toString('base64url')}})).credentials!;await ok(desktop,{roomJoin:{}});await ok(desktop,{ready:{ready:true}});await ok(host,{prepare:{}});const preparation=(await ok(desktop,{prepare:{}})).preparation!;
  const attach={preparationId:preparation.preparationId!,gameInstanceId:'candidate-game',gameVersion:metadata.compatibility.gameVersion,modVersion:metadata.compatibility.modVersion,bridgeVersion:metadata.compatibility.bridgeVersion,capabilities:['core.seed']};
  for(const change of [{modVersion:'nt2-g2-1'},{bridgeVersion:'g2-pipe32-1'},{gameVersion:'fixture'},{capabilities:[]}])assert.equal((await rpc(desktop,{gameAttach:{...attach,...change}})).error?.code,'INCOMPATIBLE_GAME');
  assert.equal((await rpc(desktop,{revision:'g2-draft-1',roomGet:{}})).error?.code,'VERSION_MISMATCH');assert.equal((await rpc(desktop,{serviceId:'nt2-g2-lan-192-168-0-104',roomGet:{}})).error?.code,'SERVICE_MISMATCH');
  assert.ok((await ok(desktop,{gameAttach:attach})).credentials?.attachmentId);const count=await service.db.transaction(tx=>tx.query('SELECT count(*)::integer AS n FROM attachments'));assert.equal(count.rows[0].n,1);
 }finally{await service.app.close();const pool=new pg.Pool({connectionString:databaseUrl});try{await pool.query(`DROP SCHEMA "${namespace}" CASCADE`);}finally{await pool.end();}}
});
