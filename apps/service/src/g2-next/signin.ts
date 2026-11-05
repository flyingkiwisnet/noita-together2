import {providerCheckAt} from '../auth1/provider-schedule.js';
import {admitSignIn,signInSource} from '../auth1/admission.js';
import {randomUUID} from 'node:crypto';
import type {Tx} from '../g1/database.js';
import {demand,fingerprint,hash,matches,secret} from '../g1/security.js';
import type {AuthBegin,AuthProof,AuthExchange,Credentials} from '../../../../packages/contracts/g2-next/g2-next.js';
import {validate} from '../../../../packages/contracts/src/g2-next.js';
import {PrincipalAuth} from './auth.js';
export class MockSignIn {
 constructor(readonly auth:PrincipalAuth,readonly providerEnabled?:((provider:string)=>boolean)){}
 private get now(){return this.auth.now();}
 private public(row:Record<string,any>){return {attemptId:row.id,state:row.state,provider:row.provider,expiresAt:row.expires_at.toISOString(),grantExpiresAt:row.grant_expires_at?.toISOString()??''};}
 async begin(tx:Tx,input:AuthBegin,requestId:string,ip='127.0.0.1'){
  validate('AuthBegin',input);demand(this.providerEnabled?this.providerEnabled(input.provider??''):this.auth.mockEnabled,'PROVIDER_UNAVAILABLE');
  demand(input.deviceId&&input.appInstanceId&&(this.providerEnabled?['twitch','google','local']:['twitch','website']).includes(input.provider??'')&&/^[a-f0-9]{64}$/.test(input.proofChallenge??'')&&/^[A-Za-z0-9_-]{43}$/.test(input.callbackState??'')&&input.callbackPort&&input.callbackPort>=1024&&input.callbackPort<=65535,'INVALID_MESSAGE');
  demand(/^[A-Za-z0-9_-]{1,128}$/.test(requestId),'INVALID_MESSAGE');const scope='g2-next-begin:'+hash(input.deviceId+':'+input.appInstanceId+':'+input.proofChallenge),fp=fingerprint(input);const cached=(await tx.query('SELECT * FROM auth_requests WHERE scope=$1 AND request_id=$2',[scope,requestId])).rows[0];if(cached){demand(cached.fingerprint===fp,'CONFLICT');demand(+cached.recovery_until>this.now,'EXPIRED');const value=this.auth.security.open<{attemptId:string;launchUrl:string}>(cached.recovery_ciphertext);const current=(await tx.query('SELECT * FROM account_signin_attempts WHERE id=$1',[value.attemptId])).rows[0];demand(current&&current.service_epoch===this.auth.db.epoch&&current.state==='pending','STALE_EPOCH');return {...this.public(current),launchUrl:value.launchUrl};}
  const source=signInSource(ip);await admitSignIn(tx,source,this.now,'app');
  const id=randomUUID();const row=(await tx.query("INSERT INTO account_signin_attempts(id,device_id,app_instance_id,provider,proof_hash,callback_port,callback_state_hash,state,expires_at,service_epoch,created_at,return_context_ciphertext,source_hash) VALUES($1,$2,$3,$4,$5,$6,$7,'pending',$8,$9,$10,$11,$12) RETURNING *",[id,input.deviceId,input.appInstanceId,input.provider,input.proofChallenge,input.callbackPort,hash(input.callbackState!),new Date(this.now+300000),this.auth.db.epoch,new Date(this.now),this.auth.security.seal({callbackState:input.callbackState}),source])).rows[0];
  const value={...this.public(row),launchUrl:this.auth.security.keys.issuer+'/signin/'+id};await tx.query('INSERT INTO auth_requests(scope,request_id,fingerprint,recovery_ciphertext,recovery_until) VALUES($1,$2,$3,$4,$5)',[scope,requestId,fp,this.auth.security.seal(value),new Date(this.now+30000)]);return value;
 }
 private async proof(tx:Tx,input:AuthProof){
  validate('AuthProof',{attemptId:input.attemptId,deviceId:input.deviceId,appInstanceId:input.appInstanceId,initiatorProof:input.initiatorProof});
  const row=(await tx.query('SELECT * FROM account_signin_attempts WHERE id=$1 FOR UPDATE',[input.attemptId])).rows[0];
  demand(row&&row.device_id===input.deviceId&&row.app_instance_id===input.appInstanceId&&matches(row.proof_hash,hash(input.initiatorProof??'')),'UNAUTHENTICATED');demand(row.service_epoch===this.auth.db.epoch,'STALE_EPOCH');return row;
 }
 async status(tx:Tx,input:AuthProof){const row=await this.proof(tx,input);if(+row.expires_at<=this.now&&['pending','completed'].includes(row.state)){await tx.query("UPDATE account_signin_attempts SET state='expired',grant_hash=NULL WHERE id=$1",[row.id]);row.state='expired';}return this.public(row);}
 async cancel(tx:Tx,input:AuthProof){const row=await this.proof(tx,input);demand(row.state!=='exchanged','ALREADY_EXCHANGED');await tx.query("UPDATE account_signin_attempts SET state='cancelled',grant_hash=NULL WHERE id=$1",[row.id]);await tx.query('UPDATE auth1_flows SET consumed_at=$2,context_ciphertext=NULL WHERE attempt_id=$1 AND consumed_at IS NULL',[row.id,new Date(this.now)]);row.state='cancelled';return this.public(row);}
 // Called only after a bounded same-origin mock account form with anti-CSRF validation.
 // The return grant belongs in the local callback POST body, never polling or a URL.
 async complete(tx:Tx,id:string,subject:string,displayName:string,requestId:string){
  const row=(await tx.query('SELECT * FROM account_signin_attempts WHERE id=$1 FOR UPDATE',[id])).rows[0];demand(row&&row.service_epoch===this.auth.db.epoch,'NOT_FOUND');const scope='g2-next-complete:'+id,fp=fingerprint({subject,displayName});const cached=(await tx.query('SELECT * FROM auth_requests WHERE scope=$1 AND request_id=$2',[scope,requestId])).rows[0];if(cached){demand(cached.fingerprint===fp&&row.state==='completed'&&+cached.recovery_until>this.now&&+row.grant_expires_at>this.now,'CONFLICT');return this.auth.security.open<{url:string;body:{attemptId:string;completionGrant:string;callbackState:string}}>(cached.recovery_ciphertext);}demand(row.state==='pending'&&+row.expires_at>this.now,'EXPIRED');const {callbackState}=this.auth.security.open<{callbackState:string}>(row.return_context_ciphertext);demand(matches(row.callback_state_hash,hash(callbackState)),'FORBIDDEN');
  const principalId=await this.auth.mockRegistered(tx,row.provider,subject,displayName),grant=secret();
  await tx.query("UPDATE account_signin_attempts SET state='completed',principal_id=$2,grant_hash=$3,grant_expires_at=$4 WHERE id=$1",[id,principalId,hash(grant),new Date(this.now+60000)]);
  const value={url:`http://127.0.0.1:${row.callback_port}/nt2-return`,body:{attemptId:id,completionGrant:grant,callbackState}};await tx.query('INSERT INTO auth_requests(scope,request_id,fingerprint,recovery_ciphertext,recovery_until) VALUES($1,$2,$3,$4,$5)',[scope,requestId,fp,this.auth.security.seal(value),new Date(this.now+30000)]);return value;
 }
 async exchange(tx:Tx,input:AuthExchange,requestId:string){
  validate('AuthExchange',input);demand(/^[A-Za-z0-9_-]{1,128}$/.test(requestId),'INVALID_MESSAGE');const row=await this.proof(tx,input),scope='g2-next-signin:'+row.id,fp=fingerprint(input);
  if(row.state==='exchanged'){
   const old=(await tx.query('SELECT * FROM auth_requests WHERE scope=$1 AND request_id=$2',[scope,requestId])).rows[0];demand(old&&old.fingerprint===fp&&row.consumed_request_id===requestId,'ALREADY_EXCHANGED');demand(+old.recovery_until>this.now,'EXPIRED');const credentials=this.auth.security.open<Credentials>(old.recovery_ciphertext);const session=(await tx.query('SELECT s.revoked_at,f.revoked_at AS family_revoked FROM sessions s JOIN session_families f ON f.id=s.family_id WHERE s.id=$1',[credentials.sessionId])).rows[0];demand(session&&!session.revoked_at&&!session.family_revoked,'UNAUTHENTICATED');await this.auth.principal(tx,credentials.accountId!,true);return credentials;
  }
  demand(row.state==='completed','INVALID_STATE');demand(+row.expires_at>this.now&&+row.grant_expires_at>this.now,'EXPIRED');demand(matches(row.grant_hash,hash(input.completionGrant??'')),'UNAUTHENTICATED');
  const credentials=await this.auth.family(tx,row.principal_id,row.device_id,'desktop') as Credentials;
  if(this.providerEnabled){await tx.query('UPDATE session_families SET authenticated_at=$2 WHERE id=(SELECT family_id FROM sessions WHERE id=$1)',[credentials.sessionId,new Date(this.now)]);if(row.provider_session_ciphertext){const context=this.auth.security.open<any>(row.provider_session_ciphertext);await tx.query("INSERT INTO auth1_provider_sessions(family_id,provider,subject,token_ciphertext,checked_at,valid_until,next_check_at) SELECT family_id,'twitch',$2,$3,$4,$5,$6 FROM sessions WHERE id=$1",[credentials.sessionId,context.subject,this.auth.security.seal(context.tokens),new Date(this.now),new Date(Math.min(this.now+3600000,context.tokens.expiresAt)),new Date(providerCheckAt(this.now,Math.min(this.now+3600000,context.tokens.expiresAt)))]);}}
  await tx.query("UPDATE account_signin_attempts SET state='exchanged',grant_hash=NULL,consumed_request_id=$2 WHERE id=$1",[row.id,requestId]);
  await tx.query('INSERT INTO auth_requests(scope,request_id,fingerprint,recovery_ciphertext,recovery_until) VALUES($1,$2,$3,$4,$5)',[scope,requestId,fp,this.auth.security.seal(credentials),new Date(this.now+30000)]);return credentials;
 }
 async completePrincipal(tx:Tx,id:string,principalId:string,provider:string,providerContext?:unknown){
  const row=(await tx.query('SELECT * FROM account_signin_attempts WHERE id=$1 FOR UPDATE',[id])).rows[0];demand(row&&row.provider===provider&&row.service_epoch===this.auth.db.epoch&&row.state==='pending'&&+row.expires_at>this.now,'EXPIRED');
  const context=this.auth.security.open<{callbackState:string}>(row.return_context_ciphertext);demand(matches(row.callback_state_hash,hash(context.callbackState)),'FORBIDDEN');const grant=secret();
  await tx.query("UPDATE account_signin_attempts SET state='completed',principal_id=$2,grant_hash=$3,grant_expires_at=$4,provider_session_ciphertext=$5 WHERE id=$1",[id,principalId,hash(grant),new Date(Math.min(+row.expires_at,this.now+60000)),providerContext?this.auth.security.seal(providerContext):null]);
  return {url:`http://127.0.0.1:${row.callback_port}/nt2-return`,body:{attemptId:id,completionGrant:grant,callbackState:context.callbackState}};
 }
}
