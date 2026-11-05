import {randomUUID} from 'node:crypto';
import {Auth,type Actor} from '../g1/auth.js';
import type {Tx} from '../g1/database.js';
import {demand,hash,secret,fingerprint} from '../g1/security.js';
import type {Credentials,GuestIssue,Principal} from '../../../../packages/contracts/g2-next/g2-next.js';
import {validate} from '../../../../packages/contracts/src/g2-next.js';
export class PrincipalAuth extends Auth {
 auth1Enabled=false;
 override async authenticate(tx:Tx,token:string,req?:import('../../../../packages/contracts/g1/g1.js').Request){const actor=await super.authenticate(tx,token,req);if(this.auth1Enabled){const lease=(await tx.query('SELECT valid_until,revoked_at FROM auth1_provider_sessions WHERE family_id=$1',[actor.familyId])).rows[0];demand(!lease||!lease.revoked_at&&+lease.valid_until>this.now(),'UNAUTHENTICATED');}return actor;}
 override async refresh(tx:Tx,req:import('../../../../packages/contracts/g1/g1.js').Request){if(this.auth1Enabled&&req.refresh?.secret){const lease=(await tx.query('SELECT p.valid_until,p.revoked_at FROM auth1_provider_sessions p JOIN refresh_tokens r ON r.family_id=p.family_id WHERE r.hash=$1',[hash(req.refresh.secret)])).rows[0];demand(!lease||!lease.revoked_at&&+lease.valid_until>this.now(),'UNAUTHENTICATED');}return super.refresh(tx,req);}
 async principal(tx:Tx,id:string,requireLive=false):Promise<Principal>{
  const p=(await tx.query('SELECT * FROM principal_profiles WHERE principal_id=$1',[id])).rows[0];demand(p,'UNAUTHENTICATED');
  demand(!requireLive||p.kind!=='guest'||+p.guest_expires_at>this.now(),'EXPIRED');
  return {principalId:p.principal_id,kind:p.kind,displayName:p.display_name,source:p.source};
 }
 override async mint(tx:Tx,accountId:string,deviceId:string,kind:Actor['kind'],familyId:string,sessionId:string,epoch:string,attachmentId=''):Promise<Credentials>{
  const principal=await this.principal(tx,accountId,true);demand(principal.kind!=='guest'||kind!=='browser','FORBIDDEN');
  const row=(await tx.query('SELECT f.expires_at,p.guest_expires_at FROM session_families f JOIN principal_profiles p ON p.principal_id=f.account_id WHERE f.id=$1',[familyId])).rows[0];demand(row,'UNAUTHENTICATED');
  const familyEnd=Math.min(+row.expires_at,row.guest_expires_at?+row.guest_expires_at:Infinity);demand(familyEnd>this.now(),'EXPIRED');
  if(row.guest_expires_at)await tx.query('UPDATE session_families SET expires_at=$2 WHERE id=$1',[familyId,new Date(familyEnd)]);
  const expiresSeconds=Math.floor(Math.min(this.now()+900000,familyEnd)/1000),expiresAt=new Date(expiresSeconds*1000),jti=randomUUID();
  await tx.query('INSERT INTO access_tokens(id,session_id,expires_at) VALUES($1,$2,$3)',[jti,sessionId,expiresAt]);
  const accessToken=await this.security.sign(accountId,sessionId,epoch,kind,jti,this.now(),expiresSeconds);
  let refreshSecret='';if(kind!=='game'){refreshSecret=secret();await tx.query('INSERT INTO refresh_tokens(hash,family_id,session_id,expires_at) VALUES($1,$2,$3,$4)',[hash(refreshSecret),familyId,sessionId,new Date(familyEnd)]);}
  return {accessToken,refreshSecret,accountId,sessionId,sessionEpoch:epoch,kind,deviceId,expiresAt:expiresAt.toISOString(),attachmentId,principal};
 }
 async guest(tx:Tx,input:GuestIssue,requestId:string){
  validate('GuestIssue',input);demand(this.mockEnabled,'FORBIDDEN');
  demand(input.deviceId&&/^[A-Za-z0-9_-]{1,128}$/.test(input.deviceId)&&/^[A-Za-z0-9_-]{43}$/.test(input.issuanceProof??'')&&/^[A-Za-z0-9_-]{1,128}$/.test(requestId),'INVALID_MESSAGE');
  const displayName=input.displayName?.trim();demand(displayName&&Buffer.byteLength(displayName)<=80&&!/[\u0000-\u001f\u007f]/.test(displayName),'INVALID_MESSAGE');
  const scope='g2-next-guest:'+hash(input.deviceId+':'+input.issuanceProof),fp=fingerprint(input);
  const old=(await tx.query('SELECT * FROM auth_requests WHERE scope=$1 AND request_id=$2',[scope,requestId])).rows[0];
  if(old){demand(old.fingerprint===fp,'CONFLICT');demand(+old.recovery_until>this.now(),'EXPIRED');const recovered=this.security.open<Credentials>(old.recovery_ciphertext);await this.principal(tx,recovered.accountId!,true);const session=(await tx.query('SELECT s.revoked_at,f.revoked_at AS family_revoked FROM sessions s JOIN session_families f ON f.id=s.family_id WHERE s.id=$1',[recovered.sessionId])).rows[0];demand(session&&!session.revoked_at&&!session.family_revoked,'UNAUTHENTICATED');return recovered;}
  // The candidate request gate adds per-peer issuance limits before entering this transaction.
  const count=(await tx.query("SELECT count(*) FROM principal_profiles WHERE kind='guest' AND guest_expires_at>$1",[new Date(this.now())])).rows[0];demand(+count.count<128,'CAPACITY');
  const id=await this.account(tx,'nt2-g2-next-internal-guest',randomUUID());
  await tx.query("INSERT INTO principal_profiles(principal_id,kind,display_name,source,created_at,guest_expires_at) VALUES($1,'guest',$2,'guest',$3,$4)",[id,displayName,new Date(this.now()),new Date(this.now()+86400000)]);
  const credentials=await this.family(tx,id,input.deviceId,'desktop');
  await tx.query('INSERT INTO auth_requests(scope,request_id,fingerprint,recovery_ciphertext,recovery_until) VALUES($1,$2,$3,$4,$5)',[scope,requestId,fp,this.security.seal(credentials),new Date(this.now()+30000)]);
  return credentials as Credentials;
 }
 async mockRegistered(tx:Tx,provider:'twitch'|'website',subject:string,displayName:string){
  demand(this.mockEnabled&&this.providerAvailable(),'PROVIDER_UNAVAILABLE');demand(['twitch','website'].includes(provider)&&/^[A-Za-z0-9_-]{1,80}$/.test(subject),'INVALID_MESSAGE');
  demand(displayName.trim()&&Buffer.byteLength(displayName)<=80&&!/[\u0000-\u001f\u007f]/.test(displayName),'INVALID_MESSAGE');
  const id=await this.account(tx,'nt2-g2-next-mock:'+provider,subject);
  await tx.query("INSERT INTO principal_profiles(principal_id,kind,display_name,source) VALUES($1,'registered',$2,$3) ON CONFLICT(principal_id) DO NOTHING",[id,displayName.trim(),'mock:'+provider]);return id;
 }
}
