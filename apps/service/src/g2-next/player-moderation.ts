import type {CandidateRooms} from './rooms.js';import type {Tx} from '../g1/database.js';import type {Actor} from '../g1/auth.js';import type {Request,Response,PlayerDetails} from '../../../../packages/contracts/g2-next/g2-next.js';import {demand,fingerprint} from '../g1/security.js';
const uuid=(v:unknown):v is string=>typeof v==='string'&&/^[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}$/i.test(v);
export class PlayerModeration {
 constructor(readonly rooms:CandidateRooms){}
 private async role(tx:Tx,id:string,requireEnabled=true){const row=(await tx.query('SELECT id,role,disabled FROM accounts WHERE id=$1 FOR UPDATE',[id])).rows[0];demand(row&&(!requireEnabled||!row.disabled),'NOT_FOUND');return row.role as string;}
 private authority(actor:Actor,r:any,actorRole:string,target:string,targetRole:string){return actor.kind!=='game'&&actor.accountId!==target&&target!==r.host_account_id&&(actor.accountId===r.host_account_id||actorRole==='dev'||actorRole==='admin'&&['user','power-user'].includes(targetRole));}
 private async details(tx:Tx,r:any,actor:Actor,actorRole:string,id:string,desktopId?:string):Promise<PlayerDetails>{
  demand(uuid(id)&&(!desktopId||uuid(desktopId)),'INVALID_MESSAGE');const role=await this.role(tx,id,false),members=(await tx.query('SELECT desktop_id FROM memberships WHERE room_id=$1 AND NOT roster_retired AND account_id=$2 ORDER BY desktop_id',[r.id,id])).rows;
  const banned=!!(await tx.query('SELECT 1 FROM room_bans WHERE room_id=$1 AND account_id=$2',[r.id,id])).rowCount,muted=!!(await tx.query('SELECT 1 FROM room_chat_mutes WHERE room_id=$1 AND account_id=$2',[r.id,id])).rowCount;
  demand(desktopId?members.some(m=>m.desktop_id===desktopId):members.length||banned,'NOT_FOUND');
  const enabled=!!(await tx.query('SELECT 1 FROM accounts WHERE id=$1 AND NOT disabled',[id])).rowCount;
  const actions:string[]=[];if(this.authority(actor,r,actorRole,id,role)){if(members.length&&enabled)actions.push('kick','ban',muted?'unmute':'mute');if(banned)actions.push('unban');}
  return {accountId:id,...(members.length?{desktopId:desktopId??members[0].desktop_id}:{}),principal:await this.rooms.principalAuth.principal(tx,id),role,chatMuted:muted,banned,roomRevision:r.revision,actions};
 }
 async execute(tx:Tx,req:Request,actor:Actor):Promise<Partial<Response>>{
  const rooms=this.rooms;demand(actor.kind!=='game','FORBIDDEN');demand(req.serviceId===rooms.db.serviceId&&req.serviceEpoch===rooms.db.epoch&&req.sessionEpoch===actor.sessionEpoch,'STALE_EPOCH');demand(uuid(req.roomId),'INVALID_MESSAGE');
  const r=(await tx.query('SELECT * FROM rooms WHERE id=$1 FOR UPDATE',[req.roomId])).rows[0];demand(r&&!r.deleted_at&&await rooms.visible(tx,r,actor.accountId),'NOT_FOUND');demand(!(await tx.query('SELECT 1 FROM room_bans WHERE room_id=$1 AND account_id=$2',[r.id,actor.accountId])).rowCount,'NOT_FOUND');rooms.roomBudget?.(r.id,tx);
  await rooms.refreshRoster(tx,r);
  const actorRole=await this.role(tx,actor.accountId);
  if(req.roomPlayerDetails)return {playerDetails:await this.details(tx,r,actor,actorRole,req.roomPlayerDetails.accountId!,req.roomPlayerDetails.desktopId)};
  if(req.roomBanList){
   demand(actor.accountId===r.host_account_id||['admin','dev'].includes(actorRole),'FORBIDDEN');const {afterAccountId:after='',limit=32}=req.roomBanList;demand((!after||uuid(after))&&limit>=1&&limit<=32,'INVALID_MESSAGE');
   const rows=(await tx.query('SELECT b.account_id FROM room_bans b WHERE b.room_id=$1 AND ($2::uuid IS NULL OR b.account_id>$2) ORDER BY b.account_id LIMIT $3',[r.id,after||null,limit+1])).rows,bans:PlayerDetails[]=[];
   for(const b of rows.slice(0,limit))bans.push(await this.details(tx,r,actor,actorRole,b.account_id));return {roomBans:bans,nextRoomBanAccountId:rows.length>limit?bans.at(-1)!.accountId!:''};
  }
  const input=req.roomModerate!;demand(uuid(input.accountId)&&['kick','ban','unban','mute','unmute'].includes(input.action??''),'INVALID_MESSAGE');const targetRole=await this.role(tx,input.accountId,input.action!=='unban');demand(this.authority(actor,r,actorRole,input.accountId,targetRole),'FORBIDDEN');
  const fp=fingerprint(req),old=(await tx.query('SELECT * FROM command_results WHERE scope=$1 AND request_id=$2',[actor.sessionId,req.requestId])).rows[0];if(old){demand(old.fingerprint===fp,'CONFLICT');demand(+old.created_at+30000>rooms.now,'EXPIRED');return {room:await rooms.snapshot(tx,r)};}
  demand(req.expectedRevision===r.revision&&req.configRevision===r.config_revision,'STALE_REVISION');const details=await this.details(tx,r,actor,actorRole,input.accountId,input.desktopId);demand(details.actions?.includes(input.action!),'FORBIDDEN');
  if(input.action==='mute')await tx.query('INSERT INTO room_chat_mutes(room_id,account_id,actor_id,created_at) VALUES($1,$2,$3,$4) ON CONFLICT DO NOTHING',[r.id,input.accountId,actor.accountId,new Date(rooms.now)]);
  else if(input.action==='unmute')await tx.query('DELETE FROM room_chat_mutes WHERE room_id=$1 AND account_id=$2',[r.id,input.accountId]);
  else if(input.action==='unban')await tx.query('DELETE FROM room_bans WHERE room_id=$1 AND account_id=$2',[r.id,input.accountId]);
  else{
   // Effective rejoin authority retires only this account's participants first.
   await rooms.onPlayerRemoved(tx,r,input.accountId,input.action!);
   const selected=!!(await tx.query('SELECT 1 FROM memberships WHERE room_id=$1 AND account_id=$2 AND selected',[r.id,input.accountId])).rowCount;
   if(selected&&['preparing','loading'].includes(r.state))await rooms.cancel(tx,r,'host_moderation');
   else if(selected&&['running','suspended'].includes(r.state)){r.state='suspended';await tx.query("UPDATE runs SET state='suspended',reason='host_moderation' WHERE id=$1",[r.current_run_id]);}
   await tx.query('UPDATE sessions SET revoked_at=$3 WHERE id IN (SELECT a.game_session_id FROM attachments a JOIN memberships m ON m.desktop_id=a.desktop_id WHERE m.room_id=$1 AND m.account_id=$2)',[r.id,input.accountId,new Date(rooms.now)]);
   await tx.query('UPDATE attachments SET revoked_at=$3 WHERE desktop_id IN (SELECT desktop_id FROM memberships WHERE room_id=$1 AND account_id=$2)',[r.id,input.accountId,new Date(rooms.now)]);
   await tx.query("UPDATE preparations SET state='cancelled' WHERE desktop_id IN (SELECT desktop_id FROM memberships WHERE room_id=$1 AND account_id=$2) AND state IN ('issued','consumed','loaded')",[r.id,input.accountId]);
   await tx.query('DELETE FROM memberships WHERE room_id=$1 AND account_id=$2',[r.id,input.accountId]);if(input.action==='ban')await tx.query('INSERT INTO room_bans(room_id,account_id) VALUES($1,$2) ON CONFLICT DO NOTHING',[r.id,input.accountId]);
  }
  await tx.query('UPDATE rooms SET revision=revision+1,state=$2 WHERE id=$1',[r.id,r.state]);r.revision++;
  await tx.query('INSERT INTO room_moderation_audit(room_id,actor_id,target_id,session_id,request_id,action,created_at) VALUES($1,$2,$3,$4,$5,$6,$7)',[r.id,actor.accountId,input.accountId,actor.sessionId,req.requestId,input.action,new Date(rooms.now)]);
  return rooms.record(tx,req,actor,fp,{room:await rooms.snapshot(tx,r)});
 }
}
