import {randomUUID} from 'node:crypto';
import type {Request,Response,Room,Config,RoomSummary} from '../../../../packages/contracts/g2/g2.js';
import type {Tx,Database} from './database.js';
import {Auth,type Actor} from './auth.js';
import {demand,fingerprint,PolicyError} from './security.js';
import {compatibleConfiguration,rosterFailure,admissionFailure} from '../../../../packages/domain/src/g1.js';
import {passwordMatches,passwordVerifier} from '../g2/password.js';
const uuid=(value:unknown):value is string=>typeof value==='string'&&/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(value);
export interface Compatibility {gameVersion:string;modVersion:string;bridgeVersion:string;capabilities:string[];modes:string[];settingKeys:string[]}
export class Rooms {
 // Successor lobbies may withdraw visibility without removing historical rows.
 protected get activeRoomSql(){return '';}
 protected mayCreateRoom(actor:Actor){return actor.kind==='browser';}
 roomBudget?: (roomId:string,tx:Tx)=>void;
 private sweeping:Promise<void>|undefined;private sweepCount=0;
 constructor(readonly db:Database,readonly auth:Auth,readonly compatibility:Compatibility,readonly g2=false){}
 get now(){return this.auth.now();}
 async refreshRoster(_tx:Tx,_r:Record<string,any>):Promise<void>{}
 async snapshot(tx:Tx,r:Record<string,any>):Promise<Room>{
  await this.refreshRoster(tx,r);
  const run=r.current_run_id?(await tx.query('SELECT * FROM runs WHERE id=$1',[r.current_run_id])).rows[0]:undefined;
  const members=await tx.query(`SELECT m.*,s.revoked_at,a.id AS attachment_id,a.revoked_at AS attachment_revoked,a.presence_until AS game_presence,a.seed_applied,a.loaded FROM memberships m JOIN sessions s ON s.id=m.desktop_id LEFT JOIN attachments a ON a.desktop_id=m.desktop_id AND a.run_id=$2 AND a.revoked_at IS NULL WHERE m.room_id=$1 AND NOT m.roster_retired ORDER BY m.desktop_id`,[r.id,r.current_run_id]);
  return {...(this.g2?{capacity:r.capacity,passwordRequired:!!r.password_verifier}:{}),roomId:r.id,name:r.name,hostAccountId:r.host_account_id,state:r.state,revision:r.revision,configRevision:r.config_revision,config:r.config,runId:run?.id??'',runEpoch:run?.epoch??'',preparationExpiresAt:run?.preparation_until.toISOString()??'',members:members.rows.map(m=>({accountId:m.account_id,desktopId:m.desktop_id,present:!m.revoked_at&&m.presence_until&&+m.presence_until>this.now||false,ready:!m.revoked_at&&m.ready_config_revision===r.config_revision,selected:m.selected,attached:!!m.attachment_id&&!m.attachment_revoked&&+m.game_presence>this.now,seedApplied:!!m.seed_applied&&!m.attachment_revoked,loaded:!!m.loaded&&!m.attachment_revoked}))};
 }
 resolveConfig(config:Config):Config{return config;}
 validateConfig(config:Config){
  demand(compatibleConfiguration(config,this.compatibility),'INCOMPATIBLE_GAME');
 }
 validateConfigChange(current:Config,next:Config){
  demand(next.mode===current.mode,'INVALID_STATE');
  this.validateConfig(next);
 }
 async canAdmit(tx:Tx,r:Record<string,any>,accountId:string){if(this.g2&&r.host_account_id!==accountId&&(await tx.query('SELECT 1 FROM room_bans WHERE room_id=$1 AND account_id=$2',[r.id,accountId])).rowCount)return false;return r.visibility==='public'||r.host_account_id===accountId||!!(await tx.query('SELECT 1 FROM room_admissions WHERE room_id=$1 AND account_id=$2',[r.id,accountId])).rowCount;}
 async visible(tx:Tx,r:Record<string,any>,accountId:string){return await this.canAdmit(tx,r,accountId)||!!(await tx.query('SELECT 1 FROM memberships WHERE room_id=$1 AND account_id=$2',[r.id,accountId])).rowCount;}
 async summary(tx:Tx,r:Record<string,any>,actor:Actor):Promise<RoomSummary>{
  await this.refreshRoster(tx,r);
  const membership=(await tx.query('SELECT room_id FROM memberships WHERE desktop_id=$1',[actor.sessionId])).rows[0];
  const count=+(await tx.query('SELECT count(*) FROM memberships WHERE room_id=$1 AND NOT roster_retired',[r.id])).rows[0].count;
  const joinAllowed=!admissionFailure({kind:actor.kind,entitled:await this.canAdmit(tx,r,actor.accountId),existingMember:membership?.room_id===r.id,inOtherRoom:!!membership&&membership.room_id!==r.id,state:r.state,memberCount:count,capacity:r.capacity??32});
  return {...(this.g2?{capacity:r.capacity,passwordRequired:!!r.password_verifier}:{}),roomId:r.id,name:r.name,state:r.state,revision:r.revision,configRevision:r.config_revision,visibility:r.visibility,joinAllowed};
 }
 async cancel(tx:Tx,r:Record<string,any>,reason:string,state:'waiting'|'ended'='waiting'){
  if(r.current_run_id){await tx.query('UPDATE runs SET state=$2,reason=$3,ended_at=$4 WHERE id=$1',[r.current_run_id,state==='ended'?'ended':'cancelled',reason,new Date(this.now)]);await tx.query('UPDATE sessions SET revoked_at=$2 WHERE id IN (SELECT game_session_id FROM attachments WHERE run_id=$1)',[r.current_run_id,new Date(this.now)]);await tx.query('UPDATE attachments SET revoked_at=$2 WHERE run_id=$1',[r.current_run_id,new Date(this.now)]);}
  await tx.query('UPDATE memberships SET selected=false,ready_config_revision=NULL WHERE room_id=$1',[r.id]);
  await tx.query('DELETE FROM memberships WHERE room_id=$1 AND desktop_id IN (SELECT id FROM sessions WHERE revoked_at IS NOT NULL)',[r.id]);
  await tx.query('UPDATE rooms SET state=$2,revision=revision+1 WHERE id=$1',[r.id,state]);r.state=state;r.revision++;
 }
 async reconcile(tx:Tx,r:Record<string,any>){
  if(!r.current_run_id||!['preparing','loading','running','suspended'].includes(r.state))return;
  const run=(await tx.query('SELECT * FROM runs WHERE id=$1',[r.current_run_id])).rows[0];
  if(['preparing','loading'].includes(r.state)){
   const missing=await tx.query('SELECT 1 FROM memberships m JOIN sessions s ON s.id=m.desktop_id WHERE room_id=$1 AND selected AND (presence_until<=$2 OR presence_until IS NULL OR s.revoked_at IS NOT NULL) LIMIT 1',[r.id,new Date(this.now)]);
   if(+run.preparation_until<=this.now||missing.rowCount){await this.cancel(tx,r,+run.preparation_until<=this.now?'preparation_expired':'desktop_lost');return;}
  }
  if(r.state==='running'){
   const missing=await tx.query('SELECT 1 FROM attachments WHERE run_id=$1 AND (presence_until<=$2 OR revoked_at IS NOT NULL) LIMIT 1',[r.current_run_id,new Date(this.now)]);
   if(missing.rowCount){await tx.query("UPDATE runs SET state='suspended',reason='game_disconnected' WHERE id=$1",[r.current_run_id]);await tx.query("UPDATE rooms SET state='suspended',revision=revision+1 WHERE id=$1",[r.id]);r.state='suspended';r.revision++;}
  }
 }
 get sweepStats(){return {running:this.sweeping?1:0,started:this.sweepCount};}
 sweep():Promise<void>{
  if(this.sweeping)return this.sweeping;
  this.sweepCount++;
  this.sweeping=this.db.transaction(async tx=>{const q=await tx.query("SELECT * FROM rooms WHERE state IN ('preparing','loading','running') ORDER BY id FOR UPDATE");for(const r of q.rows)await this.reconcile(tx,r);if(this.g2){await tx.query('DELETE FROM room_chat WHERE (room_id,sequence) IN (SELECT room_id,sequence FROM room_chat WHERE sent_at<$1 ORDER BY sent_at LIMIT 512)',[new Date(this.now-15*60*1000)]);await tx.query("UPDATE command_results SET result='{\"expired\":true}'::jsonb WHERE (scope,request_id) IN (SELECT scope,request_id FROM command_results WHERE operation='chat' AND created_at<$1 AND NOT result ? 'expired' ORDER BY created_at LIMIT 512)",[new Date(this.now-15*60*1000)]);await tx.query('DELETE FROM room_chat_limits WHERE session_id IN (SELECT session_id FROM room_chat_limits WHERE started_at<$1 ORDER BY started_at LIMIT 512)',[new Date(this.now-10000)]);}}).finally(()=>{this.sweeping=undefined;});return this.sweeping;
 }

 async chatPage(tx:Tx,r:Record<string,any>,input:{afterSequence?:number;limit?:number}):Promise<Partial<Response>>{
  const limit=input.limit??32,after=input.afterSequence??0;demand(limit>=1&&limit<=32&&after<=+r.chat_sequence,'INVALID_MESSAGE');
  await tx.query('DELETE FROM room_chat WHERE room_id=$1 AND (sent_at<$2 OR sequence<=$3)',[r.id,new Date(this.now-15*60*1000),Math.max(0,+r.chat_sequence-256)]);
  const first=+(await tx.query('SELECT min(sequence) AS first FROM room_chat WHERE room_id=$1',[r.id])).rows[0].first;
  const reset=after>0&&((first>0&&after<first-1)||(first===0&&after<+r.chat_sequence));
  const rows=(await tx.query('SELECT * FROM room_chat WHERE room_id=$1 AND sequence>$2 ORDER BY sequence LIMIT $3',[r.id,reset?0:after,limit])).rows;
  return {chatMessages:rows.map(m=>({sequence:+m.sequence,accountId:m.account_id,sessionKind:m.session_kind,sentAt:m.sent_at.toISOString(),text:m.text})),chatNextSequence:rows.length?+rows.at(-1).sequence:+r.chat_sequence,chatReset:reset};
 }
 async lobbyCommand(tx:Tx,r:Record<string,any>,req:Request,actor:Actor):Promise<Partial<Response>>{
  demand(actor.kind!=='game','FORBIDDEN');
  if(req.chatSend){
   const text=req.chatSend.text??'';demand(text.trim().length>0&&!/[\u0000-\u0008\u000b\u000c\u000e-\u001f]/.test(text),'INVALID_MESSAGE');
   const rate=(await tx.query('SELECT * FROM room_chat_limits WHERE session_id=$1 FOR UPDATE',[actor.sessionId])).rows[0];const reset=!rate||+rate.started_at<=this.now-10000;if(!reset&&rate.count>=5)throw new PolicyError('CAPACITY','CAPACITY',Math.max(1,+rate.started_at+10000-this.now));await tx.query('INSERT INTO room_chat_limits(session_id,started_at,count) VALUES($1,$2,$3) ON CONFLICT(session_id) DO UPDATE SET started_at=EXCLUDED.started_at,count=EXCLUDED.count',[actor.sessionId,reset?new Date(this.now):rate.started_at,reset?1:rate.count+1]);
   demand(+r.chat_sequence<0xffffffff,'CAPACITY');r.chat_sequence=+r.chat_sequence+1;
   await tx.query('UPDATE rooms SET chat_sequence=$2 WHERE id=$1',[r.id,r.chat_sequence]);
   await tx.query('INSERT INTO room_chat(room_id,sequence,account_id,session_id,session_kind,sent_at,text) VALUES($1,$2,$3,$4,$5,$6,$7)',[r.id,r.chat_sequence,actor.accountId,actor.sessionId,actor.kind,new Date(this.now),text]);
   const page=await this.chatPage(tx,r,{afterSequence:r.chat_sequence-1,limit:1});return page;
  }
  demand(r.host_account_id===actor.accountId,'FORBIDDEN');
  if(req.roomPolicy){
   demand(['waiting','ended'].includes(r.state),'INVALID_STATE');const p=req.roomPolicy;demand(p.capacity!==undefined&&p.capacity>=1&&p.capacity<=32,'INVALID_MESSAGE');
   demand(+(await tx.query('SELECT count(*) FROM memberships WHERE room_id=$1 AND NOT roster_retired',[r.id])).rows[0].count<=p.capacity!,'CAPACITY');
   if(p.password!==undefined)r.password_verifier=p.password?await passwordVerifier(p.password):null;r.capacity=p.capacity;r.revision++;
   await tx.query('UPDATE rooms SET capacity=$2,password_verifier=$3,revision=$4 WHERE id=$1',[r.id,r.capacity,r.password_verifier,r.revision]);return {room:await this.snapshot(tx,r)};
  }
  const p=req.roomModerate!;demand(uuid(p.accountId)&&['kick','ban','unban'].includes(p.action??''),'INVALID_MESSAGE');demand(p.accountId!==actor.accountId,'FORBIDDEN');
  demand((await tx.query('SELECT 1 FROM accounts WHERE id=$1',[p.accountId])).rowCount,'NOT_FOUND');
  if(p.action==='unban')await tx.query('DELETE FROM room_bans WHERE room_id=$1 AND account_id=$2',[r.id,p.accountId]);
  else {
   const selected=(await tx.query('SELECT 1 FROM memberships WHERE room_id=$1 AND account_id=$2 AND selected',[r.id,p.accountId])).rowCount;
   if(selected&&['preparing','loading'].includes(r.state))await this.cancel(tx,r,'host_moderation');
   else if(selected&&['running','suspended'].includes(r.state)){
    r.state='suspended';await tx.query("UPDATE runs SET state='suspended',reason='host_moderation' WHERE id=$1",[r.current_run_id]);
   }
   await tx.query('UPDATE sessions SET revoked_at=$3 WHERE id IN (SELECT a.game_session_id FROM attachments a JOIN memberships m ON m.desktop_id=a.desktop_id WHERE m.room_id=$1 AND m.account_id=$2)',[r.id,p.accountId,new Date(this.now)]);
   await tx.query('UPDATE attachments SET revoked_at=$3 WHERE desktop_id IN (SELECT desktop_id FROM memberships WHERE room_id=$1 AND account_id=$2)',[r.id,p.accountId,new Date(this.now)]);
   await tx.query('DELETE FROM memberships WHERE room_id=$1 AND account_id=$2',[r.id,p.accountId]);
   if(p.action==='ban')await tx.query('INSERT INTO room_bans(room_id,account_id) VALUES($1,$2) ON CONFLICT DO NOTHING',[r.id,p.accountId]);
  }
  r.revision++;await this.persist(tx,r);return {room:await this.snapshot(tx,r)};
 }

 async execute(tx:Tx,req:Request,actor:Actor):Promise<Partial<Response>>{
  demand(req.serviceEpoch===this.db.epoch&&req.sessionEpoch===actor.sessionEpoch,'STALE_EPOCH');
  if(req.roomList){
   demand(actor.kind!=='game','FORBIDDEN');const limit=req.roomList.limit??32,cursor=req.roomList.afterRoomId??'';demand(limit>=1&&limit<=32&&(!cursor||uuid(cursor)),'INVALID_MESSAGE');
   const banFilter=this.g2?" AND (host_account_id=$1 OR NOT EXISTS (SELECT 1 FROM room_bans WHERE room_bans.room_id=rooms.id AND room_bans.account_id=$1))":"";
   const q=await tx.query("SELECT * FROM rooms WHERE (visibility='public' OR host_account_id=$1 OR id IN (SELECT room_id FROM memberships WHERE account_id=$1) OR id IN (SELECT room_id FROM room_admissions WHERE account_id=$1)) AND ($2::uuid IS NULL OR id>$2::uuid) "+this.activeRoomSql+banFilter+" ORDER BY id LIMIT $3",[actor.accountId,cursor||null,limit+1]);
   const rows=q.rows.slice(0,limit),summaries:RoomSummary[]=[];for(const r of rows)summaries.push(await this.summary(tx,r,actor));return {rooms:summaries,nextRoomId:q.rows.length>limit?rows.at(-1)!.id:''};
  }
  if(this.g2&&req.roomId){const room=(await tx.query('SELECT * FROM rooms WHERE id=$1',[req.roomId])).rows[0];if(room&&room.host_account_id!==actor.accountId){demand(!(await tx.query('SELECT 1 FROM room_bans WHERE room_id=$1 AND account_id=$2',[room.id,actor.accountId])).rowCount,'NOT_FOUND');if(!req.roomJoin&&!req.roomPreview)demand((await tx.query('SELECT 1 FROM memberships WHERE room_id=$1 AND account_id=$2',[room.id,actor.accountId])).rowCount,'FORBIDDEN');}}
  const fp=fingerprint(req);
  const previous=(await tx.query('SELECT * FROM command_results WHERE scope=$1 AND request_id=$2',[actor.sessionId,req.requestId])).rows[0];
  if(previous){
   demand(previous.fingerprint===fp,'CONFLICT');if(req.chatSend)demand(+previous.created_at>this.now-15*60*1000,'EXPIRED');const result=this.auth.security.open<Partial<Response>>(previous.result.sealed);
   if(req.start||req.gameAttach||req.gameResume||req.gameSeedApplied||req.gameLoaded){
    const current=(await tx.query('SELECT * FROM rooms WHERE id=$1 FOR UPDATE',[result.room?.roomId])).rows[0];
    demand(current&&current.current_run_id===result.room?.runId,'STALE_EPOCH');
    const run=(await tx.query('SELECT state FROM runs WHERE id=$1',[current.current_run_id])).rows[0];
    demand(run?.state!=='cancelled','CANCELLED');demand(run&&['preparing','loading','running','suspended'].includes(run.state),'INVALID_STATE');
    if(result.credentials?.kind==='game'){
     const session=(await tx.query('SELECT revoked_at,epoch FROM sessions WHERE id=$1',[result.credentials.sessionId])).rows[0];demand(session&&!session.revoked_at,'RESTART_REQUIRED');demand(session.epoch===result.credentials.sessionEpoch,'STALE_EPOCH');
    }
    result.room=await this.snapshot(tx,current);
   }
   return result;
  }
  let r:Record<string,any>;
  if(req.roomCreate){
   demand(this.mayCreateRoom(actor),'FORBIDDEN');demand(req.roomCreate.name?.trim()&&req.roomCreate.config,'INVALID_MESSAGE');this.validateConfig(req.roomCreate.config);
   await tx.query('SELECT id FROM accounts WHERE id=$1 FOR UPDATE',[actor.accountId]);
   demand(+(await tx.query('SELECT count(*) FROM rooms WHERE host_account_id=$1'+this.activeRoomSql,[actor.accountId])).rows[0].count<32,'CAPACITY');
   const visibility=req.roomCreate.visibility??'public';demand(visibility==='public'||visibility==='private','INVALID_MESSAGE');
   r=(await tx.query("INSERT INTO rooms(id,name,host_account_id,state,config,visibility) VALUES($1,$2,$3,'waiting',$4,$5) RETURNING *",[randomUUID(),req.roomCreate.name,actor.accountId,this.resolveConfig(req.roomCreate.config),visibility])).rows[0];
  }else{
   demand(uuid(req.roomId),'INVALID_MESSAGE');r=(await tx.query('SELECT * FROM rooms WHERE id=$1 FOR UPDATE',[req.roomId])).rows[0];demand(r,'NOT_FOUND');
   const member=(await tx.query('SELECT * FROM memberships WHERE room_id=$1 AND desktop_id=$2',[r.id,actor.kind==='game'?actor.parentId:actor.sessionId])).rows[0];
   const accountMember=(await tx.query('SELECT 1 FROM memberships WHERE room_id=$1 AND account_id=$2 LIMIT 1',[r.id,actor.accountId])).rowCount;
   if(req.roomPreview){demand(actor.kind!=='game','FORBIDDEN');demand(await this.visible(tx,r,actor.accountId),'NOT_FOUND');await this.reconcile(tx,r);return {admission:await this.summary(tx,r,actor)};}
   if(actor.kind==='game')demand(member?.selected&&(await tx.query('SELECT 1 FROM attachments WHERE game_session_id=$1 AND run_id=$2 AND revoked_at IS NULL',[actor.sessionId,r.current_run_id])).rowCount,'FORBIDDEN');
   demand(req.roomJoin?actor.kind==='desktop':r.host_account_id===actor.accountId||accountMember,'FORBIDDEN');
   if(req.roomJoin&&!member)demand(await this.canAdmit(tx,r,actor.accountId),'NOT_FOUND');
   if(!req.roomJoin||member||r.host_account_id===actor.accountId)this.roomBudget?.(r.id,tx);
   await this.reconcile(tx,r);
   if(req.cancelRun||req.endRun||req.gameAttach||req.gameSeedApplied||req.gameLoaded||req.gameResume){
    const scope=r.current_run_id?(await tx.query('SELECT epoch FROM runs WHERE id=$1',[r.current_run_id])).rows[0]:undefined;demand(scope&&scope.epoch===req.runEpoch,'STALE_EPOCH');
   }
   if(req.roomGet)return {room:await this.snapshot(tx,r)};
   if(req.chatList){demand(this.g2&&actor.kind!=='game','FORBIDDEN');return this.chatPage(tx,r,req.chatList);}
   // Chat appends to its own sequence; it does not consume a configuration or
   // roster revision. Keep current room authorization and the original receipt
   // identity, but allow ordinary sends from an older lobby snapshot.
   if(req.chatSend){demand(this.g2,'FORBIDDEN');const result=await this.lobbyCommand(tx,r,req,actor);return this.record(tx,req,actor,fp,result);}
   demand(req.configRevision===r.config_revision,'STALE_REVISION');
   // Heartbeats do not consume config/roster revisions; explicit changes do.
   if(!req.presence&&!req.gameResume)demand(req.expectedRevision===r.revision,'STALE_REVISION');
   if(req.roomPolicy||req.roomModerate){demand(this.g2,'FORBIDDEN');const result=await this.lobbyCommand(tx,r,req,actor);return this.record(tx,req,actor,fp,result);}
   if(req.roomJoin){
    if(this.g2&&!member&&r.password_verifier)demand(!!req.roomJoin.password&&await passwordMatches(req.roomJoin.password,r.password_verifier),'PASSWORD_REQUIRED');
    const old=(await tx.query('SELECT room_id FROM memberships WHERE desktop_id=$1',[actor.sessionId])).rows[0];const count=+(await tx.query('SELECT count(*) FROM memberships WHERE room_id=$1 AND NOT roster_retired',[r.id])).rows[0].count;
    const failure=admissionFailure({kind:actor.kind,entitled:await this.canAdmit(tx,r,actor.accountId),existingMember:!!member,inOtherRoom:!!old&&old.room_id!==r.id,state:r.state,memberCount:count,capacity:r.capacity??32});demand(!failure,failure??'FORBIDDEN');
    if(!member)this.roomBudget?.(r.id,tx);
    if(!old){await tx.query('INSERT INTO memberships(desktop_id,room_id,account_id,presence_until) VALUES($1,$2,$3,$4)',[actor.sessionId,r.id,actor.accountId,new Date(this.now+30000)]);r.revision++;}
   }else if(req.roomAccess){
    demand(actor.kind!=='game'&&r.host_account_id===actor.accountId,'FORBIDDEN');const input=req.roomAccess;
    demand(input.visibility==='public'||input.visibility==='private','INVALID_MESSAGE');const ids=input.admittedAccountIds??[];demand(ids.every(uuid)&&new Set(ids).size===ids.length,'INVALID_MESSAGE');
    const known=(await tx.query('SELECT count(*) FROM accounts WHERE id=ANY($1::uuid[]) AND NOT disabled',[ids])).rows[0].count;demand(+known===ids.length,'NOT_FOUND');
    await tx.query('UPDATE rooms SET visibility=$2 WHERE id=$1',[r.id,input.visibility]);await tx.query('DELETE FROM room_admissions WHERE room_id=$1',[r.id]);for(const id of ids)await tx.query('INSERT INTO room_admissions(room_id,account_id) VALUES($1,$2)',[r.id,id]);r.visibility=input.visibility;r.revision++;
   }else if(req.roomLeave){
    demand(actor.kind==='desktop'&&member,'FORBIDDEN');if(member.selected){if(['preparing','loading'].includes(r.state))await this.cancel(tx,r,'selected_desktop_left');else demand(['waiting','ended'].includes(r.state),'INVALID_STATE');}await tx.query('DELETE FROM memberships WHERE desktop_id=$1',[actor.sessionId]);r.revision++;
   }else if(req.presence){
    demand(actor.kind==='desktop'&&member,'FORBIDDEN');await tx.query('UPDATE memberships SET presence_until=$2 WHERE desktop_id=$1',[actor.sessionId,req.presence.present?new Date(this.now+30000):null]);
    if(!req.presence.present){await tx.query('UPDATE memberships SET ready_config_revision=NULL WHERE desktop_id=$1',[actor.sessionId]);await this.reconcile(tx,r);}
   }else if(req.ready){
    demand(actor.kind==='desktop'&&member,'FORBIDDEN');demand(['waiting','ended','suspended'].includes(r.state),'INVALID_STATE');demand(req.configRevision===r.config_revision,'STALE_REVISION');demand(member.presence_until&&+member.presence_until>this.now,'NOT_READY');await tx.query('UPDATE memberships SET ready_config_revision=$2 WHERE desktop_id=$1',[actor.sessionId,req.ready.ready?r.config_revision:null]);r.revision++;
   }else if(req.configure){
    demand(actor.kind!=='game'&&r.host_account_id===actor.accountId,'FORBIDDEN');demand(['waiting','ended'].includes(r.state),'INVALID_STATE');demand(req.configRevision===r.config_revision,'STALE_REVISION');this.validateConfigChange(r.config,req.configure);r.config=req.configure;r.config_revision++;r.revision++;await tx.query('UPDATE memberships SET ready_config_revision=NULL WHERE room_id=$1',[r.id]);
   }else if(req.start){
    this.validateConfig(r.config);
    demand(actor.kind!=='game'&&r.host_account_id===actor.accountId,'FORBIDDEN');demand(['waiting','ended','suspended'].includes(r.state),'INVALID_STATE');demand(req.configRevision===r.config_revision,'STALE_REVISION');
    const members=(await tx.query('SELECT * FROM memberships WHERE room_id=$1 AND NOT roster_retired ORDER BY desktop_id',[r.id])).rows;
    const selected=req.start.participantDesktopIds??[],excluded=req.start.excludedDesktopIds??[];
    const failure=rosterFailure(members.map(m=>({desktopId:m.desktop_id,present:!!m.presence_until&&+m.presence_until>this.now,readyConfigRevision:m.ready_config_revision})),selected,excluded,r.config_revision);demand(!failure,failure??'ROSTER_MISMATCH');
    if(r.state==='suspended')await this.cancel(tx,r,'explicit_new_run','ended');
    const runId=randomUUID(),runEpoch=randomUUID();await tx.query("INSERT INTO runs(id,room_id,epoch,service_epoch,config_revision,config,roster,excluded_roster,state,preparation_until,created_at) VALUES($1,$2,$3,$4,$5,$6,$7,$8,'preparing',$9,$10)",[runId,r.id,runEpoch,this.db.epoch,r.config_revision,r.config,selected,excluded,new Date(this.now+120000),new Date(this.now)]);
    await tx.query('UPDATE memberships SET selected=(desktop_id=ANY($2::uuid[])) WHERE room_id=$1',[r.id,selected]);r.current_run_id=runId;r.state='preparing';r.revision++;
   }else if(req.cancelRun||req.endRun){
    demand(actor.kind!=='game'&&r.host_account_id===actor.accountId,'FORBIDDEN');this.runScope(req,r);
    if(req.cancelRun)demand(['preparing','loading'].includes(r.state),'INVALID_STATE');else demand(['running','suspended','preparing','loading'].includes(r.state),'INVALID_STATE');
    await this.cancel(tx,r,req.cancelRun?'host_cancelled':'host_ended',req.endRun?'ended':'waiting');
   }else if(req.gameAttach){
    demand(actor.kind==='desktop'&&member?.selected,'FORBIDDEN');this.runScope(req,r);demand(['preparing','loading'].includes(r.state),'INVALID_STATE');
    const run=(await tx.query('SELECT * FROM runs WHERE id=$1',[r.current_run_id])).rows[0];demand(req.runEpoch===run.epoch,'STALE_EPOCH');
    const input=req.gameAttach;demand(input.gameInstanceId&&input.gameVersion===this.compatibility.gameVersion&&input.modVersion===this.compatibility.modVersion&&input.bridgeVersion===this.compatibility.bridgeVersion,'INCOMPATIBLE_GAME');demand((r.config.requiredCapabilities??[]).every((c:string)=>input.capabilities?.includes(c)),'INCOMPATIBLE_GAME');
    const old=(await tx.query('SELECT id FROM attachments WHERE run_id=$1 AND desktop_id=$2',[r.current_run_id,actor.sessionId])).rowCount;demand(!old,'RESTART_REQUIRED');
    const id=randomUUID(),sid=randomUUID(),epoch=randomUUID();await tx.query("INSERT INTO sessions(id,family_id,kind,epoch,parent_id) VALUES($1,$2,'game',$3,$4)",[sid,actor.familyId,epoch,actor.sessionId]);await tx.query('INSERT INTO attachments(id,run_id,desktop_id,game_session_id,game_instance_id,service_epoch,run_epoch,config_revision,presence_until,resume_until) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)',[id,r.current_run_id,actor.sessionId,sid,input.gameInstanceId,this.db.epoch,run.epoch,r.config_revision,new Date(this.now+30000),new Date(this.now+60000)]);
    r.revision++;await this.persist(tx,r);const result={credentials:await this.auth.mint(tx,actor.accountId,actor.deviceId,'game',actor.familyId,sid,epoch,id),room:await this.snapshot(tx,r)};return this.record(tx,req,actor,fp,result);
   }else if(req.gameSeedApplied||req.gameLoaded||req.gameResume){
    demand(actor.kind==='game'&&member?.selected,'FORBIDDEN');this.runScope(req,r);
    const input=req.gameSeedApplied??req.gameLoaded??req.gameResume;demand(uuid(input?.attachmentId),'INVALID_MESSAGE');
    const a=(await tx.query('SELECT * FROM attachments WHERE id=$1 AND game_session_id=$2 FOR UPDATE',[input.attachmentId,actor.sessionId])).rows[0];demand(a&&!a.revoked_at&&a.game_instance_id===input.gameInstanceId,'RESTART_REQUIRED');demand(a.service_epoch===this.db.epoch&&a.run_epoch===req.runEpoch&&a.config_revision===req.configRevision,'STALE_EPOCH');
    demand(['preparing','loading','running','suspended'].includes(r.state),'INVALID_STATE');demand(+a.resume_until>this.now,'RESTART_REQUIRED');
    if(req.gameResume){
     demand(r.state!=='suspended'||(await tx.query('SELECT service_epoch FROM runs WHERE id=$1',[r.current_run_id])).rows[0].service_epoch===this.db.epoch,'RESTART_REQUIRED');
     const epoch=randomUUID();await tx.query('UPDATE sessions SET epoch=$2 WHERE id=$1',[actor.sessionId,epoch]);await tx.query('UPDATE access_tokens SET revoked_at=$2 WHERE session_id=$1',[actor.sessionId,new Date(this.now)]);await tx.query('UPDATE attachments SET presence_until=$2,resume_until=$3 WHERE id=$1',[a.id,new Date(this.now+30000),new Date(this.now+60000)]);
     if(r.state==='suspended'){
      const missing=(await tx.query('SELECT 1 FROM attachments WHERE run_id=$1 AND (presence_until<=$2 OR revoked_at IS NOT NULL) LIMIT 1',[r.current_run_id,new Date(this.now)])).rowCount;
      if(!missing){r.state='running';r.revision++;await tx.query("UPDATE runs SET state='running',reason=NULL WHERE id=$1",[r.current_run_id]);}
     }
     await this.persist(tx,r);return this.record(tx,req,actor,fp,{credentials:await this.auth.mint(tx,actor.accountId,actor.deviceId,'game',actor.familyId,actor.sessionId,epoch,a.id),room:await this.snapshot(tx,r)});
    }
    demand(['preparing','loading'].includes(r.state),'INVALID_STATE');
    if(req.gameLoaded)demand(a.seed_applied&&r.state==='loading','INVALID_STATE');
    await tx.query('UPDATE attachments SET seed_applied=seed_applied OR $2,loaded=loaded OR $3,presence_until=$4,resume_until=$5 WHERE id=$1',[a.id,!!req.gameSeedApplied,!!req.gameLoaded,new Date(this.now+30000),new Date(this.now+60000)]);
    const facts=(await tx.query('SELECT count(*) AS n,bool_and(seed_applied) AS seed,bool_and(loaded) AS loaded FROM attachments WHERE run_id=$1 AND revoked_at IS NULL',[r.current_run_id])).rows[0];const count=(await tx.query('SELECT cardinality(roster) AS n FROM runs WHERE id=$1',[r.current_run_id])).rows[0].n;
    if(+facts.n===count&&facts.seed)r.state=facts.loaded?'running':'loading';r.revision++;await tx.query('UPDATE runs SET state=$2 WHERE id=$1',[r.current_run_id,r.state]);
   }else demand(false,'INVALID_MESSAGE');
  }
  await this.persist(tx,r);return this.record(tx,req,actor,fp,{room:await this.snapshot(tx,r)});
 }
 runScope(req:Request,r:Record<string,any>){demand(req.runId===r.current_run_id&&req.configRevision===r.config_revision,'STALE_REVISION');demand(uuid(req.runEpoch),'STALE_EPOCH');}
 async persist(tx:Tx,r:Record<string,any>){await tx.query('UPDATE rooms SET state=$2,revision=$3,config_revision=$4,config=$5,current_run_id=$6 WHERE id=$1',[r.id,r.state,r.revision,r.config_revision,r.config,r.current_run_id??null]);}
 async record(tx:Tx,req:Request,actor:Actor,fp:string,result:Partial<Response>){if(this.g2)await tx.query('INSERT INTO command_results(scope,request_id,fingerprint,result,created_at,operation) VALUES($1,$2,$3,$4,$5,$6)',[actor.sessionId,req.requestId,fp,{sealed:this.auth.security.seal(result)},new Date(this.now),req.chatSend?'chat':'foundation']);else await tx.query('INSERT INTO command_results(scope,request_id,fingerprint,result,created_at) VALUES($1,$2,$3,$4,$5)',[actor.sessionId,req.requestId,fp,{sealed:this.auth.security.seal(result)},new Date(this.now)]);return result;}
}
