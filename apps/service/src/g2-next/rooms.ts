import {validateRaceConfig} from '../../../../packages/domain/src/g3-race-config.js';
import {randomInt} from 'node:crypto';
import {policyCapabilities} from '../../../../packages/domain/src/seed-policy.js';
import type {Request,Response,Room,RoomSummary,Config} from '../../../../packages/contracts/g2-next/g2-next.js';
import {schema,validate} from '../../../../packages/contracts/src/g2-next.js';
import {admissionFailure,roomDeletionFailure} from '../../../../packages/domain/src/g1.js';
import {Rooms,type Compatibility} from '../g1/rooms.js';
import type {Actor} from '../g1/auth.js';
import type {Database,Tx} from '../g1/database.js';
import {demand,fingerprint} from '../g1/security.js';
import {passwordMatches,passwordVerifier} from '../g2/password.js';
import {PlayerModeration} from './player-moderation.js';
import {PrincipalAuth} from './auth.js';
import {InspectionAuthority} from './inspection.js';
import {TESTING_STARTING_LOADOUT,testingStartingLoadout} from '../../../../packages/domain/src/testing-starting-loadout.js';
import type {Accounts} from '../auth1/accounts.js';
export class CandidateRooms extends Rooms {
 protected override mayCreateRoom(actor:Actor){return actor.kind==='browser'||actor.kind==='desktop';}
 readonly moderation=new PlayerModeration(this);
 async onPlayerRemoved(_tx:Tx,_room:Record<string,any>,_accountId:string,_action:string):Promise<void>{}
 accountEntitlements:Accounts|undefined;
 testingStartingLoadoutEnabled=false;
 // Future Dev-role enforcement belongs here; ordinary participants can join a
 // host's testing room without acquiring permission to enable the option.
 protected async testingStartingLoadoutHostAllowed(tx:Tx,actor:Actor){return this.testingStartingLoadoutEnabled&&actor.kind!=='game'&&(!this.accountEntitlements||(await this.accountEntitlements.role(tx,actor.accountId,true)).role==='dev');}
 protected async testingStartingLoadoutConfigMutable(tx:Tx,room:Record<string,any>){return !(await tx.query('SELECT 1 FROM preparations WHERE run_id=$1 LIMIT 1',[room.current_run_id])).rowCount;}
 protected async onRoomDeleted(_tx:Tx,_room:Record<string,any>):Promise<void>{}
 protected override get activeRoomSql(){return ' AND deleted_at IS NULL';}
 override async canAdmit(tx:Tx,r:Record<string,any>,accountId:string){return !r.deleted_at&&super.canAdmit(tx,r,accountId);}
 override async visible(tx:Tx,r:Record<string,any>,accountId:string){return !r.deleted_at&&super.visible(tx,r,accountId);}
 readonly inspection:InspectionAuthority;private candidateSweep:Promise<void>|undefined;
 constructor(db:Database,readonly principalAuth:PrincipalAuth,compatibility:Compatibility){super(db,principalAuth,compatibility,true);this.inspection=new InspectionAuthority(db,principalAuth,compatibility);this.inspection.roomBudget=(id,tx)=>this.roomBudget?.(id,tx);}
 override validateConfig(config:Config){
  validate('Config',config);validateRaceConfig(config);
  const testing=(config.settings??[]).filter(s=>s.key===TESTING_STARTING_LOADOUT.settingKey);
  demand(!testing.length||this.testingStartingLoadoutEnabled,'FORBIDDEN');
  demand(testing.every(s=>TESTING_STARTING_LOADOUT.values.includes(s.value??'')),'INVALID_MESSAGE');
  demand((config.settings??[]).every(s=>!['NT_send_gold','NT_send_wands','NT_send_flasks','NT_send_items'].includes(s.key??'')||['true','false'].includes(s.value??'')),'INVALID_MESSAGE');
  const normalized=config.seedPolicy===undefined?config:{...config,seed:config.seed??0};super.validateConfig(normalized);
  demand(policyCapabilities(config).every(cap=>this.compatibility.capabilities.includes(cap)&&config.requiredCapabilities?.includes(cap)),'INCOMPATIBLE_GAME');
 }
 override resolveConfig(config:Config):Config{
  if(config.seedPolicy===undefined)return structuredClone(config);
  return {...structuredClone(config),seed:config.seedPolicy==='shared-random'&&!(config.seed??0)?randomInt(1,0x100000000):config.seed??0,randomizeLoot:config.randomizeLoot??false};
 }
 override async snapshot(tx:Tx,r:Record<string,any>):Promise<Room>{
  demand(r&&!r.deleted_at,'NOT_FOUND');
  const room=await super.snapshot(tx,r),run=r.current_run_id?(await tx.query('SELECT * FROM runs WHERE id=$1',[r.current_run_id])).rows[0]:undefined;
  const members=(await tx.query('SELECT * FROM memberships WHERE room_id=$1 AND NOT roster_retired ORDER BY desktop_id',[r.id])).rows,projected=[];
  // A transaction has one PostgreSQL client: avoid parallel queries on it.
  for(const m of room.members??[])projected.push({...m,...await this.inspection.readiness(tx,r,run,members.find(row=>row.desktop_id===m.desktopId)),principal:await this.principalAuth.principal(tx,m.accountId!)});
  const muted=(await tx.query('SELECT account_id FROM room_chat_mutes WHERE room_id=$1',[r.id])).rows;
  return {...room,publicStats:!!r.public_stats,statsLocator:r.stats_locator,visibility:r.visibility,joinLocked:r.join_locked,host:await this.principalAuth.principal(tx,r.host_account_id),preparationExpiresAt:'',allowGuests:r.allow_guests,reviewExpiresAt:run?.review_until?.toISOString()??'',members:projected.map(m=>({...m,chatMuted:muted.some(p=>p.account_id===m.accountId)}))};
 }
 override async record(tx:Tx,req:Request,actor:Actor,fp:string,result:Partial<Response>){
  if(req.roomCreate&&result.room){const r=(await tx.query('SELECT * FROM rooms WHERE id=$1 FOR UPDATE',[result.room.roomId])).rows[0];r.capacity=req.roomCreate.capacity??this.accountEntitlements?.limits.standard??32;r.password_verifier=req.roomCreate.password?await passwordVerifier(req.roomCreate.password):null;await tx.query('UPDATE rooms SET capacity=$2,password_verifier=$3 WHERE id=$1',[r.id,r.capacity,r.password_verifier]);await this.inspection.openWaiting(tx,r);result.room=await this.snapshot(tx,r);}
  if(req.presence||req.gamePulse){const stored={...(result.room?{room:{roomId:result.room.roomId,runId:result.room.runId}}:{}),...(result.preparation?{preparation:result.preparation}:{})};await tx.query('INSERT INTO command_results(scope,request_id,fingerprint,result,created_at,operation) VALUES($1,$2,$3,$4,$5,$6)',[actor.sessionId,req.requestId,fp,{sealed:this.auth.security.seal(stored)},new Date(this.now),'routine']);return result;}
  return super.record(tx,req,actor,fp,result);
 }
 override async chatPage(tx:Tx,r:Record<string,any>,input:{afterSequence?:number;limit?:number}):Promise<Partial<Response>>{
  const result=await super.chatPage(tx,r,input),messages=result.chatMessages??[];
  const profiles=(await tx.query('SELECT principal_id,kind,display_name,source FROM principal_profiles WHERE principal_id=ANY($1::uuid[])',[[...new Set(messages.map(m=>m.accountId!))]])).rows;
  return {...result,chatMessages:messages.map(m=>{const p=profiles.find(p=>p.principal_id===m.accountId);return {...m,...(p?{principal:{principalId:p.principal_id,kind:p.kind,displayName:p.display_name,source:p.source}}:{})};})};
 }
 override async lobbyCommand(tx:Tx,r:Record<string,any>,req:Request,actor:Actor):Promise<Partial<Response>>{
  if(req.roomPolicy?.publicStats!==undefined){demand(actor.kind!=='game'&&r.host_account_id===actor.accountId,'FORBIDDEN');demand(Object.keys(req.roomPolicy).length===1,'INVALID_MESSAGE');if(r.public_stats!==req.roomPolicy.publicStats){await tx.query('UPDATE rooms SET public_stats=$2,revision=revision+1 WHERE id=$1',[r.id,req.roomPolicy.publicStats]);r.public_stats=req.roomPolicy.publicStats;r.revision++;}return {room:await this.snapshot(tx,r)};}
  if(req.roomPolicy?.joinLocked!==undefined){demand(actor.kind!=='game'&&r.host_account_id===actor.accountId,'FORBIDDEN');demand(['waiting','ended','staging'].includes(r.state),'INVALID_STATE');await tx.query('UPDATE rooms SET join_locked=$2 WHERE id=$1',[r.id,req.roomPolicy.joinLocked]);r={...r,join_locked:req.roomPolicy.joinLocked};}
  if(req.chatSend)demand(!(await tx.query('SELECT 1 FROM room_chat_mutes WHERE room_id=$1 AND account_id=$2',[r.id,actor.accountId])).rowCount,'CHAT_MUTED');
  if(req.roomPolicy?.name!==undefined){const name=req.roomPolicy.name;demand(!!this.accountEntitlements&&name.trim()&&Buffer.byteLength(name)<=80&&!/[\x00-\x1f\x7f]/.test(name),'INVALID_MESSAGE');demand(actor.kind!=='game'&&r.host_account_id===actor.accountId,'FORBIDDEN');const p=this.accountEntitlements.project(await this.accountEntitlements.role(tx,actor.accountId,true));demand(name===r.name||p.entitlements.customLobbyName,'FORBIDDEN');demand(['waiting','ended','staging'].includes(r.state),'INVALID_STATE');await tx.query('UPDATE rooms SET name=$2 WHERE id=$1',[r.id,name]);r={...r,name};}
  // Admission policy does not mutate the frozen seed/configuration.
  if(req.roomPolicy&&r.state==='staging'){const result=await super.lobbyCommand(tx,{...r,state:'waiting'},req,actor);result.room=await this.snapshot(tx,(await tx.query('SELECT * FROM rooms WHERE id=$1',[r.id])).rows[0]);return result;}
  return super.lobbyCommand(tx,r,req,actor);
 }
 override async summary(tx:Tx,r:Record<string,any>,actor:Actor):Promise<RoomSummary>{await this.refreshRoster(tx,r);const summary=await super.summary(tx,{...r,state:r.state==='staging'?'waiting':r.state},actor),principal=await this.principalAuth.principal(tx,actor.accountId);const member=(await tx.query('SELECT 1 FROM memberships WHERE desktop_id=$1 AND room_id=$2',[actor.sessionId,r.id])).rowCount;const statsAllowed=r.visibility==='public'&&r.public_stats||r.host_account_id===actor.accountId||!!(await tx.query('SELECT 1 FROM memberships WHERE room_id=$1 AND account_id=$2',[r.id,actor.accountId])).rowCount;return {...summary,publicStats:!!r.public_stats,statsLocator:r.stats_locator,statsAllowed,joinLocked:r.join_locked,host:await this.principalAuth.principal(tx,r.host_account_id),mode:r.config.mode,playerCount:Number((await tx.query('SELECT count(*) FROM memberships WHERE room_id=$1 AND NOT roster_retired',[r.id])).rows[0].count),state:r.state,allowGuests:r.allow_guests,joinAllowed:(!r.join_locked||!!member||r.host_account_id===actor.accountId)&&!!summary.joinAllowed&&(!!member||principal.kind!=='guest'||r.allow_guests)};}
 override async execute(tx:Tx,req:Request,actor:Actor):Promise<Partial<Response>>{
  validate('Request',req);demand(req.major===schema.major&&req.revision===schema.revision,'VERSION_MISMATCH');
  const principal=await this.principalAuth.principal(tx,actor.accountId,true);
  if(req.roomPlayerDetails||req.roomBanList||req.roomModerate){demand(!req.roomModerate||principal.kind==='registered','FORBIDDEN');return this.moderation.execute(tx,req,actor);}
  if(req.roomCreate?.capacity!==undefined)demand(req.roomCreate.capacity>=1&&req.roomCreate.capacity<=32,'INVALID_MESSAGE');
  if(this.accountEntitlements&&(req.roomCreate||req.roomPolicy)){
   demand(actor.kind!=='game','FORBIDDEN');const account=this.accountEntitlements.project(await this.accountEntitlements.role(tx,actor.accountId,true));
   if(req.roomCreate)demand(account.entitlements.customLobbyName||req.roomCreate.name===account.entitlements.generatedLobbyName,'FORBIDDEN');
   if(req.roomCreate?.capacity!==undefined)demand(req.roomCreate.capacity<=account.entitlements.capacityLimit,'FORBIDDEN');
   if(req.roomPolicy?.capacity!==undefined){const old=(await tx.query('SELECT capacity FROM rooms WHERE id=$1 FOR UPDATE',[req.roomId])).rows[0];demand(old,'NOT_FOUND');demand(req.roomPolicy.capacity<=old.capacity||req.roomPolicy.capacity<=account.entitlements.capacityLimit,'FORBIDDEN');}
  }
  if(req.roomCreate&&testingStartingLoadout(req.roomCreate.config??{}))demand(await this.testingStartingLoadoutHostAllowed(tx,actor),'FORBIDDEN');
  if(req.roomDelete){
   demand(req.serviceId===this.db.serviceId&&req.serviceEpoch===this.db.epoch&&req.sessionEpoch===actor.sessionEpoch,'STALE_EPOCH');
   demand(typeof req.roomId==='string'&&/^[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}$/i.test(req.roomId),'INVALID_MESSAGE');
   const r=(await tx.query('SELECT * FROM rooms WHERE id=$1 FOR UPDATE',[req.roomId])).rows[0];demand(r,'NOT_FOUND');
   demand(actor.kind!=='game'&&principal.kind==='registered'&&r.host_account_id===actor.accountId,'FORBIDDEN');this.roomBudget?.(r.id,tx);
   const fp=fingerprint(req),previous=(await tx.query('SELECT * FROM command_results WHERE scope=$1 AND request_id=$2',[actor.sessionId,req.requestId])).rows[0];if(previous)demand(previous.fingerprint===fp,'CONFLICT');
   if(r.deleted_at){await this.onRoomDeleted(tx,r);return {deletedRoomId:r.id};}
   demand(req.expectedRevision===r.revision&&req.configRevision===r.config_revision,'STALE_REVISION');
   const prepared=!!(await tx.query("SELECT 1 FROM preparations WHERE run_id=$1 AND state IN ('issued','consumed','loaded') AND expires_at>$2 LIMIT 1",[r.current_run_id,new Date(this.now)])).rowCount;
   const attached=!!(await tx.query('SELECT 1 FROM attachments WHERE run_id=$1 AND revoked_at IS NULL LIMIT 1',[r.current_run_id])).rowCount;
   const failure=roomDeletionFailure({kind:actor.kind==='desktop'?'browser':actor.kind,registered:principal.kind==='registered',host:r.host_account_id===actor.accountId,state:r.state,prepared,attached});demand(!failure,failure??'INVALID_STATE');
   await tx.query('UPDATE rooms SET deleted_at=$2,revision=revision+1 WHERE id=$1',[r.id,new Date(this.now)]);
   await tx.query('DELETE FROM memberships WHERE room_id=$1',[r.id]);await tx.query('DELETE FROM room_admissions WHERE room_id=$1',[r.id]);
   await tx.query('DELETE FROM room_chat_mutes WHERE room_id=$1',[r.id]);
   await tx.query("UPDATE preparations SET state='cancelled' WHERE run_id IN (SELECT id FROM runs WHERE room_id=$1) AND state IN ('issued','consumed','loaded')",[r.id]);
   await tx.query('UPDATE sessions SET revoked_at=$2 WHERE id IN (SELECT a.game_session_id FROM attachments a JOIN runs n ON n.id=a.run_id WHERE n.room_id=$1) AND revoked_at IS NULL',[r.id,new Date(this.now)]);
   await tx.query('UPDATE attachments SET revoked_at=$2 WHERE run_id IN (SELECT id FROM runs WHERE room_id=$1) AND revoked_at IS NULL',[r.id,new Date(this.now)]);
   await this.onRoomDeleted(tx,r);
   return this.record(tx,req,actor,fp,{deletedRoomId:r.id});
  }
  if(req.roomId){demand(/^[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}$/i.test(req.roomId),'INVALID_MESSAGE');const r=(await tx.query('SELECT deleted_at FROM rooms WHERE id=$1',[req.roomId])).rows[0];demand(!r?.deleted_at,'NOT_FOUND');}
  if(req.roomPreview){
   demand(actor.kind!=='game','FORBIDDEN');demand(req.serviceId===this.db.serviceId,'SERVICE_MISMATCH');demand(req.serviceEpoch===this.db.epoch&&req.sessionEpoch===actor.sessionEpoch,'STALE_EPOCH');
   demand(typeof req.roomId==='string'&&/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(req.roomId),'INVALID_MESSAGE');
   // One read lock makes admission and displayed config/revisions coherent.
   // Never snapshot members/inspections or reconcile/prepare from a preview.
   const r=(await tx.query('SELECT * FROM rooms WHERE id=$1 FOR SHARE',[req.roomId])).rows[0];demand(r,'NOT_FOUND');
   if(r.host_account_id!==actor.accountId)demand(!(await tx.query('SELECT 1 FROM room_bans WHERE room_id=$1 AND account_id=$2',[r.id,actor.accountId])).rowCount,'NOT_FOUND');
   demand(await this.visible(tx,r,actor.accountId),'NOT_FOUND');this.roomBudget?.(r.id,tx);
   const run=r.current_run_id?(await tx.query('SELECT id,epoch FROM runs WHERE id=$1',[r.current_run_id])).rows[0]:undefined;
   return {admission:await this.summary(tx,r,actor),room:{roomId:r.id,name:r.name,state:r.state,revision:r.revision,configRevision:r.config_revision,config:r.config,...(run?{runId:run.id,runEpoch:run.epoch}:{})}};
  }
  if(req.roomGet&&actor.kind==='game'){const result=await this.inspection.status(tx,actor,req);const r=(await tx.query('SELECT * FROM rooms WHERE id=$1',[req.roomId])).rows[0];return {...result,room:await this.snapshot(tx,r)};}
  if((req.roomGet||req.chatList)&&actor.kind==='browser'){
   demand(req.serviceId===this.db.serviceId&&req.serviceEpoch===this.db.epoch&&req.sessionEpoch===actor.sessionEpoch,'STALE_EPOCH');
   if(req.chatList)demand((await tx.query('SELECT 1 FROM rooms WHERE id=$1 AND host_account_id=$2 UNION ALL SELECT 1 FROM memberships WHERE room_id=$1 AND account_id=$2 LIMIT 1',[req.roomId,actor.accountId])).rowCount,'FORBIDDEN');
   const r=(await tx.query('SELECT * FROM rooms WHERE id=$1 FOR UPDATE',[req.roomId])).rows[0];demand(r&&!r.deleted_at&&await this.visible(tx,r,actor.accountId),'NOT_FOUND');demand(!(await tx.query('SELECT 1 FROM room_bans WHERE room_id=$1 AND account_id=$2',[r.id,actor.accountId])).rowCount,'NOT_FOUND');this.roomBudget?.(r.id,tx);await this.reconcile(tx,r);return req.chatList?this.chatPage(tx,r,req.chatList):{room:await this.snapshot(tx,r)};
  }

  if(principal.kind==='guest')demand(!req.roomCreate&&!req.roomPolicy&&!req.roomAccess&&!req.roomModerate&&!req.configure&&!req.finalStart&&!req.cancelRun&&!req.endRun,'FORBIDDEN');
  const custom=!!(req.presence||req.configure||req.prepare||req.preparationRenew||req.gameAttach||req.gameObservation||req.gamePulse||req.inspectionList||req.finalStart||req.roomJoin||req.ready||req.cancelRun||req.endRun||req.gameRenew||req.preparationCancel);
  if(!custom){
   const result:Partial<Response>=await super.execute(tx,req,actor);
   if(result.room){const current=(await tx.query('SELECT deleted_at FROM rooms WHERE id=$1',[result.room.roomId])).rows[0];demand(current&&!current.deleted_at,'NOT_FOUND');}
   if(req.roomCreate||req.roomPolicy){const policy=req.roomCreate??req.roomPolicy!;if(policy.allowGuests!==undefined){await tx.query('UPDATE rooms SET allow_guests=$2 WHERE id=$1',[result.room!.roomId,policy.allowGuests]);result.room=await this.snapshot(tx,(await tx.query('SELECT * FROM rooms WHERE id=$1',[result.room!.roomId])).rows[0]);}}
   if(req.roomLeave){await tx.query("UPDATE preparations SET state='cancelled' WHERE desktop_id=$1 AND state IN ('issued','consumed','loaded')",[actor.sessionId]);await tx.query('UPDATE attachments SET revoked_at=$2 WHERE desktop_id=$1 AND revoked_at IS NULL',[actor.sessionId,new Date(this.now)]);await tx.query("UPDATE sessions SET revoked_at=$2 WHERE kind='game' AND parent_id=$1 AND revoked_at IS NULL",[actor.sessionId,new Date(this.now)]);}
   if(req.roomGet&&actor.kind==='desktop'&&result.room?.runId){const preparation=await this.inspection.ownPreparation(tx,actor,result.room.runId);if(preparation)result.preparation=preparation;}
   return result;
  }
  demand(req.serviceId===this.db.serviceId&&req.serviceEpoch===this.db.epoch&&req.sessionEpoch===actor.sessionEpoch,'STALE_EPOCH');
  const r=(await tx.query('SELECT * FROM rooms WHERE id=$1 FOR UPDATE',[req.roomId])).rows[0];demand(r,'NOT_FOUND');
  if(actor.kind!=='game'&&(r.host_account_id===actor.accountId||await this.visible(tx,r,actor.accountId)))await this.refreshRoster(tx,r);
  const fp=fingerprint(req),old=(await tx.query('SELECT * FROM command_results WHERE scope=$1 AND request_id=$2',[actor.sessionId,req.requestId])).rows[0];
  if(old){demand(old.fingerprint===fp,'CONFLICT');demand(+old.created_at+30000>this.now,'EXPIRED');
   if(req.finalStart)await this.inspection.start(tx,actor,req,req.finalStart);
   const recovered=this.auth.security.open<Partial<Response>>(old.result.sealed);
   if(recovered.credentials){const session=(await tx.query('SELECT revoked_at,epoch FROM sessions WHERE id=$1',[recovered.credentials.sessionId])).rows[0];demand(session&&!session.revoked_at&&session.epoch===recovered.credentials.sessionEpoch&&recovered.room?.runId===r.current_run_id,'RESTART_REQUIRED');}
   if(req.prepare||req.gameAttach||req.gameRenew)demand(recovered.room?.runId===r.current_run_id,'STALE_EPOCH');if(recovered.preparation){const preparation=await this.inspection.ownPreparation(tx,actor,r.current_run_id,recovered.preparation.preparationId);demand(preparation,'RESTART_REQUIRED');recovered.preparation=preparation;}
   if(recovered.room)recovered.room=await this.snapshot(tx,r);return recovered;
  }
  let result:Partial<Response>={};
  if(req.presence){
   demand(actor.kind==='desktop'&&req.configRevision===r.config_revision,'FORBIDDEN');const member=(await tx.query('SELECT 1 FROM memberships WHERE desktop_id=$1 AND room_id=$2',[actor.sessionId,r.id])).rowCount;demand(member,'FORBIDDEN');
   // Presence loss blocks readiness, but is not withdrawal of join consent.
   // Configuration changes, explicit leave/cancel and revocation still retire it.
   await tx.query('UPDATE memberships SET presence_until=$2 WHERE desktop_id=$1',[actor.sessionId,req.presence.present?new Date(this.now+30000):null]);await this.reconcile(tx,r);
  }else if(req.roomJoin){
   demand(actor.kind==='desktop','FORBIDDEN');
   demand(!(await tx.query('SELECT 1 FROM memberships WHERE room_id=$1 AND account_id=$2 AND desktop_id<>$3 AND NOT roster_retired',[r.id,actor.accountId,actor.sessionId])).rowCount,'ACCOUNT_ALREADY_JOINED');const member=(await tx.query('SELECT * FROM memberships WHERE desktop_id=$1',[actor.sessionId])).rows[0],existing=member?.room_id===r.id;
   const entitled=await this.canAdmit(tx,r,actor.accountId);demand(entitled||existing,'NOT_FOUND');
   demand(existing||principal.kind!=='guest'||r.allow_guests,'ACCOUNT_REQUIRED');
   demand(existing||!r.join_locked||r.host_account_id===actor.accountId,'LOBBY_LOCKED');
   const count=+(await tx.query('SELECT count(*) FROM memberships WHERE room_id=$1 AND NOT roster_retired',[r.id])).rows[0].count;
   const failure=admissionFailure({kind:actor.kind,entitled,existingMember:existing,inOtherRoom:!!member&&!existing,state:r.state==='staging'?'waiting':r.state,memberCount:count,capacity:r.capacity});demand(!failure,failure??'FORBIDDEN');
   demand(req.expectedRevision===r.revision&&req.configRevision===r.config_revision,'STALE_REVISION');
   if(!existing&&r.password_verifier)demand(await passwordMatches(req.roomJoin.password??'',r.password_verifier),'PASSWORD_REQUIRED');
   this.roomBudget?.(r.id,tx);
   if(!existing){await tx.query('INSERT INTO memberships(desktop_id,room_id,account_id,presence_until,ready_config_revision) VALUES($1,$2,$3,$4,$5)',[actor.sessionId,r.id,actor.accountId,new Date(this.now+30000),r.config_revision]);await tx.query('UPDATE rooms SET revision=revision+1 WHERE id=$1',[r.id]);}
   else{demand(['waiting','ended','staging'].includes(r.state),'INVALID_STATE');await tx.query('UPDATE memberships SET ready_config_revision=$2,presence_until=$3 WHERE desktop_id=$1',[actor.sessionId,r.config_revision,new Date(this.now+30000)]);}
   const current=(await tx.query('SELECT * FROM rooms WHERE id=$1',[r.id])).rows[0];
   const prepared=await this.inspection.prepare(tx,actor,{...req,expectedRevision:current.revision});if(prepared.preparation)result.preparation=prepared.preparation;
  }else if(req.configure){
   demand(actor.kind!=='game'&&r.host_account_id===actor.accountId,'FORBIDDEN');demand(['waiting','ended','staging'].includes(r.state),'INVALID_STATE');demand(req.expectedRevision===r.revision&&req.configRevision===r.config_revision,'STALE_REVISION');this.validateConfigChange(r.config,req.configure);
   if(testingStartingLoadout(req.configure)!==testingStartingLoadout(r.config)&&r.state==='staging')demand(await this.testingStartingLoadoutConfigMutable(tx,r),'INVALID_STATE');
   if(testingStartingLoadout(req.configure)&&!testingStartingLoadout(r.config))demand(await this.testingStartingLoadoutHostAllowed(tx,actor),'FORBIDDEN');
   // A consumed world may already have generated this seed even if its report
   // was lost. Require explicit cancel/New Game instead of reconfiguring it.
   if(r.state==='staging')demand(!(await tx.query("SELECT 1 FROM preparations WHERE run_id=$1 AND state IN ('consumed','loaded') LIMIT 1",[r.current_run_id])).rowCount&&!(await tx.query('SELECT 1 FROM attachments WHERE run_id=$1 LIMIT 1',[r.current_run_id])).rowCount,'INVALID_STATE');
   if(r.current_run_id)await this.cancel(tx,r,'configuration_changed');
   r.config=this.resolveConfig(req.configure);r.config_revision++;r.revision++;await tx.query('UPDATE memberships SET ready_config_revision=NULL,selected=false WHERE room_id=$1',[r.id]);await this.persist(tx,r);await this.inspection.openWaiting(tx,r);
  }else if(req.ready){
   demand(actor.kind==='desktop'&&['waiting','ended','staging'].includes(r.state),'INVALID_STATE');
   demand(req.expectedRevision===r.revision&&req.configRevision===r.config_revision,'STALE_REVISION');
   const member=(await tx.query('SELECT * FROM memberships WHERE desktop_id=$1 AND room_id=$2',[actor.sessionId,r.id])).rows[0];demand(member&&+member.presence_until>this.now,'NOT_READY');this.roomBudget?.(r.id,tx);
   await tx.query('UPDATE memberships SET ready_config_revision=$2 WHERE desktop_id=$1',[actor.sessionId,req.ready.ready?r.config_revision:null]);if(!req.ready.ready){const p=await this.inspection.ownPreparation(tx,actor,r.current_run_id);if(p)await this.inspection.cancelPreparation(tx,actor,{...req,preparationCancel:{preparationId:p.preparationId!}});}await tx.query('UPDATE rooms SET revision=revision+1 WHERE id=$1',[r.id]);
  }else if(req.cancelRun||req.endRun){
   demand(principal.kind==='registered'&&r.host_account_id===actor.accountId&&actor.kind!=='game','FORBIDDEN');demand((req.endRun?['staging','running','suspended'].includes(r.state):r.state==='staging')&&req.runId===r.current_run_id&&req.expectedRevision===r.revision,'INVALID_STATE');const run=(await tx.query('SELECT epoch FROM runs WHERE id=$1',[r.current_run_id])).rows[0];demand(req.runEpoch===run.epoch&&req.configRevision===r.config_revision,'STALE_EPOCH');this.roomBudget?.(r.id,tx);await this.cancel(tx,r,req.endRun?'host_ended':'host_cancelled',req.endRun?'ended':'waiting');await tx.query('UPDATE inspected_releases SET revoked_at=$2,reason=$3 WHERE run_id=$1 AND revoked_at IS NULL',[r.current_run_id,new Date(this.now),req.endRun?'host_ended':'host_cancelled']);r.revision++;await this.persist(tx,r);
  }else if(req.prepare){const prepared=await this.inspection.prepare(tx,actor,req);if(prepared.preparation)result.preparation=prepared.preparation;}
  else if(req.preparationRenew)result.preparation=await this.inspection.renewPreparation(tx,actor,req);
  else if(req.gameRenew)result.credentials=await this.inspection.renew(tx,actor,req);
  else if(req.preparationCancel)await this.inspection.cancelPreparation(tx,actor,req);
  else if(req.gameAttach)result.credentials=await this.inspection.attach(tx,actor,req);
  else if(req.gameObservation)await this.inspection.observe(tx,actor,req,req.gameObservation);
  else if(req.gamePulse)await this.inspection.pulse(tx,actor,req);
  else if(req.inspectionList)result=await this.inspection.list(tx,actor,req);
  else if(req.finalStart)await this.inspection.start(tx,actor,req,req.finalStart);
  if(actor.kind==='desktop'&&!result.preparation){const preparation=await this.inspection.ownPreparation(tx,actor,(await tx.query('SELECT current_run_id FROM rooms WHERE id=$1',[r.id])).rows[0].current_run_id);if(preparation)result.preparation=preparation;}
  result.room=await this.snapshot(tx,(await tx.query('SELECT * FROM rooms WHERE id=$1',[r.id])).rows[0]);return this.record(tx,req,actor,fp,result);
 }
 override async cancel(tx:Tx,r:Record<string,any>,reason:string,state:'waiting'|'ended'='waiting'){await super.cancel(tx,r,reason,state);if(r.current_run_id){await tx.query("UPDATE preparations SET state='cancelled' WHERE run_id=$1 AND state IN ('issued','consumed','loaded')",[r.current_run_id]);await tx.query('UPDATE inspected_releases SET revoked_at=$2,reason=$3 WHERE run_id=$1 AND revoked_at IS NULL',[r.current_run_id,new Date(this.now),reason]);}}
 override async reconcile(tx:Tx,r:Record<string,any>){
  if(r.state!=='running')return;
  const run=(await tx.query('SELECT * FROM runs WHERE id=$1',[r.current_run_id])).rows[0],release=(await tx.query('SELECT * FROM inspected_releases WHERE run_id=$1',[r.current_run_id])).rows[0];
  if(!run||!release||release.revoked_at){await this.inspection.suspend(tx,r,'authorization_missing');return;}
  for(const p of release.selected_reports.participants){if(!await this.inspection.binding(tx,p,r,run)){await this.inspection.suspend(tx,r,'selected_context_lost');return;}}
 }
 override sweep(){if(!this.candidateSweep)this.candidateSweep=this.performSweep().finally(()=>{this.candidateSweep=undefined;});return this.candidateSweep;}
 private async performSweep(){await super.sweep();await this.db.transaction(async tx=>{
  const expired=(await tx.query("SELECT * FROM preparations WHERE state IN ('issued','consumed') AND expires_at<=$1 ORDER BY expires_at,id LIMIT 512 FOR UPDATE",[new Date(this.now)])).rows;for(const p of expired)await this.inspection.invalidatePreparation(tx,p,'expired');
  const rooms=(await tx.query("SELECT r.* FROM rooms r JOIN runs n ON n.id=r.current_run_id WHERE r.state='staging' AND COALESCE(n.review_until,n.preparation_until)<=$1 ORDER BY r.id LIMIT 32 FOR UPDATE OF r",[new Date(this.now)])).rows;
  for(const r of rooms){await this.cancel(tx,r,'inspection_expired');r.revision++;await this.persist(tx,r);}
  await tx.query("UPDATE inspected_releases x SET revoked_at=$1,reason='run_suspended' FROM runs n WHERE n.id=x.run_id AND n.state<>'running' AND x.revoked_at IS NULL",[new Date(this.now)]);
 });}
}
