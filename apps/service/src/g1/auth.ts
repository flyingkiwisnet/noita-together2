import {randomUUID,randomInt} from 'node:crypto';
import type {Credentials,Pairing,Request} from '../../../../packages/contracts/g1/g1.js';
import type {Database,Tx} from './database.js';
import {demand,hash,secret,fingerprint,matches,Security,PolicyError} from './security.js';
export interface Actor {accountId:string;sessionId:string;sessionEpoch:string;familyId:string;deviceId:string;kind:'browser'|'desktop'|'game';parentId:string|null;accessUntil?:number;credentialId?:string}
export class Auth {
 constructor(readonly db:Database,readonly security:Security,readonly now:()=>number=Date.now,readonly mockEnabled=false,readonly providerAvailable:()=>boolean=()=>true){}
 async authenticate(tx:Tx,token:string,req?:Request):Promise<Actor>{
  const p=await this.security.verify(token,this.now());demand(p.purpose==='access'&&typeof p.sid==='string'&&typeof p.jti==='string','UNAUTHENTICATED');
  const q=await tx.query(`SELECT s.*,f.account_id,f.device_id,f.expires_at,f.revoked_at AS family_revoked,a.disabled,t.expires_at AS access_expires,t.revoked_at AS access_revoked FROM sessions s JOIN session_families f ON f.id=s.family_id JOIN accounts a ON a.id=f.account_id JOIN access_tokens t ON t.session_id=s.id AND t.id=$2 WHERE s.id=$1 FOR UPDATE OF f`,[p.sid,p.jti]);
  const r=q.rows[0];demand(r&&+r.access_expires>this.now()&&r.account_id===p.sub&&r.kind===p.kind,'UNAUTHENTICATED');
  if(req?.logout&&r.family_revoked){
   const recovery=(await tx.query('SELECT * FROM command_results WHERE scope=$1 AND request_id=$2',[r.id,req.requestId])).rows[0];
   demand(recovery&&recovery.fingerprint===fingerprint(req)&&+recovery.created_at+30000>this.now(),'UNAUTHENTICATED');
   return {accountId:r.account_id,sessionId:r.id,sessionEpoch:p.sessionEpoch as string,familyId:r.family_id,deviceId:r.device_id,kind:r.kind,parentId:r.parent_id};
  }
  demand(!r.disabled&&!r.revoked_at&&!r.family_revoked&&+r.expires_at>this.now(),'UNAUTHENTICATED');
  if(r.access_revoked||r.epoch!==p.sessionEpoch){
   demand(req?.gameResume&&r.kind==='game'&&req.serviceEpoch===this.db.epoch&&req.sessionEpoch===p.sessionEpoch,'UNAUTHENTICATED');
   const recovery=(await tx.query('SELECT * FROM command_results WHERE scope=$1 AND request_id=$2',[r.id,req.requestId])).rows[0];
   demand(recovery&&recovery.fingerprint===fingerprint(req)&&+recovery.created_at+30000>this.now(),'UNAUTHENTICATED');
  }
  return {accountId:r.account_id,sessionId:r.id,sessionEpoch:p.sessionEpoch as string,familyId:r.family_id,deviceId:r.device_id,kind:r.kind,parentId:r.parent_id,accessUntil:Math.min(Number(p.exp)*1000,+r.access_expires,+r.expires_at),credentialId:p.jti};
 }
 async account(tx:Tx,namespace:string,subject:string){
  const q=await tx.query('INSERT INTO accounts(id,provider_namespace,provider_subject) VALUES($1,$2,$3) ON CONFLICT(provider_namespace,provider_subject) DO UPDATE SET provider_subject=EXCLUDED.provider_subject RETURNING *',[randomUUID(),namespace,subject]);demand(!q.rows[0].disabled,'FORBIDDEN');return q.rows[0].id as string;
 }
 async mint(tx:Tx,accountId:string,deviceId:string,kind:Actor['kind'],familyId:string,sessionId:string,epoch:string,attachmentId=''):Promise<Credentials>{
  const jti=randomUUID(),expiresAt=new Date((Math.floor(this.now()/1000)+900)*1000);await tx.query('INSERT INTO access_tokens(id,session_id,expires_at) VALUES($1,$2,$3)',[jti,sessionId,expiresAt]);
  const accessToken=await this.security.sign(accountId,sessionId,epoch,kind,jti,this.now());
  let refreshSecret='';if(kind!=='game'){refreshSecret=secret();await tx.query('INSERT INTO refresh_tokens(hash,family_id,session_id,expires_at) VALUES($1,$2,$3,$4)',[hash(refreshSecret),familyId,sessionId,new Date(this.now()+30*86400000)]);}
  return {accessToken,refreshSecret,accountId,sessionId,sessionEpoch:epoch,kind,deviceId,expiresAt:expiresAt.toISOString(),attachmentId};
 }
 async revoke(tx:Tx,familyId:string){
  await tx.query('UPDATE session_families SET revoked_at=$2 WHERE id=$1',[familyId,new Date(this.now())]);
  await tx.query('UPDATE sessions SET revoked_at=$2 WHERE family_id=$1 AND revoked_at IS NULL',[familyId,new Date(this.now())]);
  await tx.query('UPDATE attachments SET revoked_at=$2 WHERE game_session_id IN (SELECT id FROM sessions WHERE family_id=$1)',[familyId,new Date(this.now())]);
  const affected=await tx.query('SELECT DISTINCT r.id,r.state,r.current_run_id FROM rooms r JOIN memberships m ON m.room_id=r.id JOIN sessions s ON s.id=m.desktop_id WHERE s.family_id=$1 AND m.selected ORDER BY r.id',[familyId]);
  for(const r of affected.rows){await tx.query('SELECT id FROM rooms WHERE id=$1 FOR UPDATE',[r.id]);
   if(['preparing','loading'].includes(r.state)){
    await tx.query("UPDATE runs SET state='cancelled',reason='session_revoked',ended_at=$2 WHERE id=$1",[r.current_run_id,new Date(this.now())]);
    await tx.query("UPDATE rooms SET state='waiting',revision=revision+1 WHERE id=$1",[r.id]);
    await tx.query('UPDATE sessions SET revoked_at=$2 WHERE id IN (SELECT game_session_id FROM attachments WHERE run_id=$1)',[r.current_run_id,new Date(this.now())]);
    await tx.query('UPDATE attachments SET revoked_at=$2 WHERE run_id=$1',[r.current_run_id,new Date(this.now())]);
    await tx.query('UPDATE memberships SET selected=false,ready_config_revision=NULL WHERE room_id=$1',[r.id]);
   }else if(r.state==='running'){
    await tx.query("UPDATE rooms SET state='suspended',revision=revision+1 WHERE id=$1",[r.id]);await tx.query("UPDATE runs SET state='suspended',reason='session_revoked' WHERE id=$1",[r.current_run_id]);
   }
  }
  await tx.query('UPDATE memberships SET presence_until=NULL,ready_config_revision=NULL WHERE desktop_id IN (SELECT id FROM sessions WHERE family_id=$1)',[familyId]);
 }
 async reclaimAbandonedBrowser(tx:Tx,accountId:string){
  const now=this.now(),cutoff=new Date(now-30*60000);
  // A waiting lock SELECT keeps an older snapshot of child token rows. Recheck
  // in a fresh statement UNDER each acquired family lock before any revocation.
  const rows=(await tx.query(`SELECT f.id FROM session_families f WHERE f.account_id=$1 AND f.kind='browser' AND f.revoked_at IS NULL AND f.expires_at>$2
   AND (SELECT max(t.expires_at) FROM sessions s JOIN access_tokens t ON t.session_id=s.id WHERE s.family_id=f.id)<=$3
   ORDER BY (SELECT max(t.expires_at) FROM sessions s JOIN access_tokens t ON t.session_id=s.id WHERE s.family_id=f.id),f.id LIMIT 8 FOR UPDATE OF f`,[accountId,new Date(now),cutoff])).rows;
  for(const row of rows){const eligible=(await tx.query(`SELECT f.id FROM session_families f WHERE f.id=$1 AND f.account_id=$2 AND f.kind='browser' AND f.revoked_at IS NULL AND f.expires_at>$3
    AND (SELECT max(t.expires_at) FROM sessions s JOIN access_tokens t ON t.session_id=s.id WHERE s.family_id=f.id)<=$4`,[row.id,accountId,new Date(now),cutoff])).rowCount;
   if(eligible){await this.revoke(tx,row.id);return row.id as string;}}
  return null;
 }
 async family(tx:Tx,accountId:string,deviceId:string,kind:'browser'|'desktop'){
  // Serialize family limits and same-device supersession on the account row.
  await tx.query('SELECT id FROM accounts WHERE id=$1 FOR UPDATE',[accountId]);
  await tx.query('INSERT INTO devices(account_id,device_id) VALUES($1,$2) ON CONFLICT DO NOTHING',[accountId,deviceId]);
  if(kind==='desktop'){
   const old=await tx.query("SELECT id FROM session_families WHERE account_id=$1 AND device_id=$2 AND kind='desktop' AND revoked_at IS NULL FOR UPDATE",[accountId,deviceId]);for(const row of old.rows)await this.revoke(tx,row.id);
  }
  const liveCount=async()=>+(await tx.query('SELECT count(*) FROM session_families WHERE account_id=$1 AND kind=$2 AND revoked_at IS NULL AND expires_at>$3',[accountId,kind,new Date(this.now())])).rows[0].count;
  let count=await liveCount();if(kind==='browser'&&count>=8){await this.reclaimAbandonedBrowser(tx,accountId);count=await liveCount();}demand(count<8,'CAPACITY');
  const id=randomUUID(),sid=randomUUID(),epoch=randomUUID();await tx.query('INSERT INTO session_families(id,account_id,device_id,kind,expires_at) VALUES($1,$2,$3,$4,$5)',[id,accountId,deviceId,kind,new Date(this.now()+30*86400000)]);
  await tx.query('INSERT INTO sessions(id,family_id,kind,epoch) VALUES($1,$2,$3,$4)',[sid,id,kind,epoch]);return this.mint(tx,accountId,deviceId,kind,id,sid,epoch);
 }
 async mockBrowserLogin(subject:string,deviceId:string,requestId:string,signal?:AbortSignal){
  demand(this.mockEnabled,'FORBIDDEN');demand(/^[a-zA-Z0-9_-]{1,80}$/.test(subject)&&/^[a-zA-Z0-9_-]{1,128}$/.test(deviceId)&&/^[a-zA-Z0-9_-]{1,128}$/.test(requestId),'INVALID_MESSAGE');
  if(!this.providerAvailable())throw new PolicyError('PROVIDER_UNAVAILABLE','Mock identity provider unavailable',1000);
  return this.db.transaction(async tx=>{
   const accountId=await this.account(tx,'nt2-g1-mock',subject);await tx.query('SELECT id FROM accounts WHERE id=$1 FOR UPDATE',[accountId]);
   const scope='browser:'+accountId+':'+deviceId,fp=fingerprint({subject,deviceId});const prev=(await tx.query('SELECT * FROM auth_requests WHERE scope=$1 AND request_id=$2',[scope,requestId])).rows[0];
   if(prev){demand(prev.fingerprint===fp&&+prev.recovery_until>this.now(),'CONFLICT');return this.security.open<Credentials>(prev.recovery_ciphertext);}
   const result=await this.family(tx,accountId,deviceId,'browser');await tx.query('INSERT INTO auth_requests VALUES($1,$2,$3,$4,$5)',[scope,requestId,fp,this.security.seal(result),new Date(this.now()+30000)]);return result;
  },true,signal);
 }
 async begin(tx:Tx,req:Request):Promise<Pairing>{
  const deviceId=req.pairBegin?.deviceId;demand(deviceId&&/^[a-zA-Z0-9_-]{1,128}$/.test(deviceId),'INVALID_MESSAGE');
  await tx.query("UPDATE pairing_attempts SET state='expired' WHERE state IN ('pending','approved') AND expires_at<=$1",[new Date(this.now())]);
  demand(+(await tx.query("SELECT count(*) FROM pairing_attempts WHERE state IN ('pending','approved')")).rows[0].count<128,'CAPACITY');
  const id=randomUUID(),proof=secret(),code=randomInt(10000000,100000000).toString(),until=new Date(this.now()+60000);
  // Initiation is deliberately fresh: proof is returned only to the initiating Rust process.
  await tx.query('INSERT INTO pairing_attempts(id,device_id,proof_hash,code_hash,state,expires_at) VALUES($1,$2,$3,$4,$5,$6)',[id,deviceId,hash(proof),hash(code),'pending',until]);return {attemptId:id,proof,code,state:'pending',expiresAt:until.toISOString()};
 }
 async pairing(tx:Tx,req:Request,actor:Actor|undefined):Promise<{pairing?:Pairing;credentials?:Credentials;error?:{code:string}}>{
  const input=req.pairApprove??req.pairPoll??req.pairExchange??req.pairCancel;demand(input?.attemptId&&/^[0-9a-f-]{36}$/.test(input.attemptId),'INVALID_MESSAGE');
  const r=(await tx.query('SELECT * FROM pairing_attempts WHERE id=$1 FOR UPDATE',[input.attemptId])).rows[0];demand(r,'NOT_FOUND');
  if(req.pairApprove){demand(actor?.kind==='browser','FORBIDDEN');demand(matches(hash(req.pairApprove.code??''),r.code_hash),'FORBIDDEN');}
  else {const proof=req.pairPoll??req.pairExchange??req.pairCancel;demand(proof&&proof.deviceId===r.device_id&&matches(hash(proof.proof??''),r.proof_hash),'FORBIDDEN');}
  if(+r.expires_at<=this.now()&&['pending','approved'].includes(r.state)){await tx.query("UPDATE pairing_attempts SET state='expired' WHERE id=$1",[r.id]);r.state='expired';}
  const fp=fingerprint(input);
  if(req.pairExchange&&r.state==='exchanged'){
   if(r.request_id===req.requestId&&r.fingerprint===fp&&+r.recovery_until>this.now())return {credentials:this.security.open<Credentials>(r.recovery_ciphertext)};
   return {error:{code:'ALREADY_EXCHANGED'}};
  }
  if(req.pairPoll)return {pairing:{attemptId:r.id,state:r.state,expiresAt:r.expires_at.toISOString()}};
  demand(r.state!=='expired','EXPIRED');demand(r.state!=='cancelled','CANCELLED');
  if(req.pairApprove){demand(r.state==='pending'||(r.state==='approved'&&r.account_id===actor!.accountId),'CONFLICT');await tx.query("UPDATE pairing_attempts SET state='approved',account_id=$2 WHERE id=$1",[r.id,actor!.accountId]);r.state='approved';}
  if(req.pairCancel){demand(r.state!=='exchanged','ALREADY_EXCHANGED');await tx.query("UPDATE pairing_attempts SET state='cancelled' WHERE id=$1",[r.id]);r.state='cancelled';}
  if(req.pairExchange){demand(r.state==='approved','INVALID_STATE');const credentials=await this.family(tx,r.account_id,r.device_id,'desktop');await tx.query("UPDATE pairing_attempts SET state='exchanged',request_id=$2,fingerprint=$3,recovery_ciphertext=$4,recovery_until=$5 WHERE id=$1",[r.id,req.requestId,fp,this.security.seal(credentials),new Date(this.now()+30000)]);return {credentials};}
  return {pairing:{attemptId:r.id,state:r.state,expiresAt:r.expires_at.toISOString()}};
 }
 async refresh(tx:Tx,req:Request):Promise<{credentials?:Credentials;error?:{code:string}}>{
  const input=req.refresh;demand(input?.secret&&input.deviceId,'INVALID_MESSAGE');
  // Family first, then refresh row: the same lock order is used by authenticated commands.
  const found=(await tx.query('SELECT family_id FROM refresh_tokens WHERE hash=$1',[hash(input.secret)])).rows[0];demand(found,'UNAUTHENTICATED');
  const family=(await tx.query('SELECT f.*,a.disabled FROM session_families f JOIN accounts a ON a.id=f.account_id WHERE f.id=$1 FOR UPDATE OF f',[found.family_id])).rows[0];
  demand(family&&!family.revoked_at&&!family.disabled&&+family.expires_at>this.now(),'UNAUTHENTICATED');
  const r=(await tx.query('SELECT * FROM refresh_tokens WHERE hash=$1 FOR UPDATE',[hash(input.secret)])).rows[0];demand(+r.expires_at>this.now(),'EXPIRED');
  const fp=fingerprint(input);
  if(r.spent_at){
   if(r.request_id===req.requestId&&r.fingerprint===fp&&family.device_id===input.deviceId&&+r.recovery_until>this.now())return {credentials:this.security.open<Credentials>(r.recovery_ciphertext)};
   await this.revoke(tx,family.id);return {error:{code:'REFRESH_REUSED'}};
  }
  demand(family.device_id===input.deviceId,'FORBIDDEN');
  const session=(await tx.query('SELECT * FROM sessions WHERE id=$1',[r.session_id])).rows[0];demand(session&&!session.revoked_at&&session.kind!=='game','UNAUTHENTICATED');
  const credentials=await this.mint(tx,family.account_id,family.device_id,session.kind,family.id,session.id,session.epoch);
  await tx.query('UPDATE access_tokens SET revoked_at=$2 WHERE session_id=$1 AND id<>$3',[session.id,new Date(this.now()),(await this.security.verify(credentials.accessToken!,this.now())).jti]);
  await tx.query('UPDATE refresh_tokens SET spent_at=$2,request_id=$3,fingerprint=$4,recovery_ciphertext=$5,recovery_until=$6 WHERE hash=$1',[r.hash,new Date(this.now()),req.requestId,fp,this.security.seal(credentials),new Date(this.now()+30000)]);return {credentials};
 }
}
