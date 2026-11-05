import {touchC1Room} from './c1.js';
import {readRoomAdmissions} from './admission.js';
import {markBankDirty} from './bank-events.js';
import {CandidateDatabase} from '../g2-next/database.js';
import {CandidateRooms} from '../g2-next/rooms.js';
import {migrateGameplay} from './database.js';
import type {Tx,Database} from '../g1/database.js';
import type {Actor} from '../g1/auth.js';
import type {Request,Response} from '../../../../packages/contracts/g2-next/g2-next.js';

export class G3CandidateDatabase extends CandidateDatabase {
 protected override readonly namespacePrefix='g3_';
 override async migrate(){await super.migrate();await migrateGameplay(this);}
 override async start(){await super.start();await this.transaction(async tx=>{
  await tx.query('UPDATE g3_contexts SET revoked_at=$1 WHERE revoked_at IS NULL',[new Date()]);
  await tx.query("UPDATE g3_runs SET state='ended' WHERE state='running'");
 });}
}

// Projection only. CandidateRooms/InspectionAuthority remain the room/run authority.
// Called inside the same transaction as Start and before any G3 command.
export async function synchronizeGameplay(tx:Tx,serviceEpoch:string,capabilities:string[],now:number,context:{db:Database;compatibility:any;roomId?:string|undefined}){
 await tx.query(`INSERT INTO g3_runs(id,room_id,epoch,config_revision,owner_id,state,mode,room_name,created_at)
  SELECT n.id,n.room_id,n.epoch,n.config_revision,r.host_account_id,'running',r.config->>'mode',r.name,n.created_at
  FROM runs n JOIN rooms r ON r.id=n.room_id JOIN inspected_releases x ON x.run_id=n.id
  WHERE n.state='running' AND r.state='running' AND n.service_epoch=$1 AND x.revoked_at IS NULL AND r.config->>'mode' IN ('core','race')
  ON CONFLICT(id) DO NOTHING`,[serviceEpoch]);
 await tx.query(`UPDATE g3_runs g SET state='ended' WHERE g.state='running' AND NOT EXISTS(
  SELECT 1 FROM runs n JOIN rooms r ON r.id=n.room_id JOIN inspected_releases x ON x.run_id=n.id
  WHERE n.id=g.id AND n.epoch=g.epoch AND n.service_epoch=$1 AND n.state='running' AND r.state='running' AND x.revoked_at IS NULL)`,[serviceEpoch]);
 await tx.query(`INSERT INTO g3_access(run_id,principal_id,display_name)
  SELECT DISTINCT g.id,p.principal_id,p.display_name FROM g3_runs g JOIN rooms r ON r.id=g.room_id
  JOIN principal_profiles p ON p.principal_id=r.host_account_id OR p.principal_id IN(
   SELECT m.account_id FROM memberships m WHERE m.room_id=r.id)
  WHERE g.state='running' ON CONFLICT DO NOTHING`);
 await tx.query(`UPDATE g3_contexts c SET revoked_at=now() WHERE c.revoked_at IS NULL AND (
  NOT EXISTS(SELECT 1 FROM g3_runs g WHERE g.id=c.run_id AND g.state='running') OR
  NOT EXISTS(SELECT 1 FROM attachments a JOIN sessions s ON s.id=a.game_session_id JOIN sessions d ON d.id=s.parent_id JOIN session_families f ON f.id=s.family_id
   WHERE a.id=c.attachment_id AND a.revoked_at IS NULL AND s.revoked_at IS NULL AND d.revoked_at IS NULL AND f.revoked_at IS NULL AND a.service_epoch=$1))`,[serviceEpoch]);
 // A protected restoration is a temporary hold, not a new world or permission to
 // continue gameplay. Its fence survives; a valid playing report can renew it.
 await tx.query(`UPDATE g3_contexts c SET presence_until=LEAST(c.presence_until,$1)
  WHERE c.revoked_at IS NULL AND NOT EXISTS(SELECT 1 FROM world_observations w
   JOIN attachments a ON a.id=w.attachment_id JOIN memberships m ON m.desktop_id=a.desktop_id
   WHERE w.attachment_id=c.attachment_id AND w.world_token=c.world_token AND w.body->>'protected'='false' AND m.selected)`,[new Date(now)]);
 const rooms=(await tx.query("SELECT r.id FROM rooms r JOIN g3_runs g ON g.id=r.current_run_id AND g.state='running' WHERE r.state='running' AND ($1::uuid IS NULL OR r.id=$1)",[context.roomId??null])).rows;
 for(const room of rooms){
  const admission=await readRoomAdmissions(tx,context.db,context.compatibility,now,room.id);
  await tx.query('UPDATE g3_contexts SET presence_until=LEAST(presence_until,$2) WHERE run_id=$1 AND revoked_at IS NULL',[admission.runId,new Date(now)]);
  for(const w of admission.worlds.filter(w=>w.decision.playing))await tx.query(`INSERT INTO g3_contexts(attachment_id,run_id,principal_id,game_session_id,session_epoch,service_epoch,world_token,capabilities,presence_until) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9) ON CONFLICT(attachment_id) DO UPDATE SET presence_until=EXCLUDED.presence_until`,[w.owner.attachmentId,w.scope.runId,w.scope.principalId,w.owner.gameSessionId,w.owner.gameSessionEpoch,serviceEpoch,w.scope.worldToken,w.capabilities,new Date(w.until)]);
 }

}

export class G3CandidateRooms extends CandidateRooms {
 protected override async onRoomDeleted(tx:Tx,room:Record<string,any>){await tx.query('SELECT g3_delete_room_transfers($1)',[room.id]);await tx.query('UPDATE d1_runs SET ended_at=COALESCE(ended_at,$2),expires_at=$2 WHERE room_id=$1',[room.id,new Date(this.now)]);await tx.query('UPDATE race_runs SET ended_at=COALESCE(ended_at,$2),expires_at=$2 WHERE room_id=$1',[room.id,new Date(this.now)]);}
 override async execute(tx:Tx,req:Request,actor:Actor):Promise<Partial<Response>>{
  const result=await super.execute(tx,req,actor);
  const roomId=req.roomId??result.room?.roomId;
  if(roomId&&(req.roomJoin||req.roomLeave||req.roomModerate))await touchC1Room(tx,roomId);
  // These reads publish canonical room/admission facts, not gameplay storage.
  // Keep their authentication/reconciliation and committed live projection, but
  // do not rebuild g3_runs/access/contexts for each UI/chat poll. Every actual
  // gameplay command still synchronizes before checking its current authority;
  // mutations (including presence, moderation, Start and End) keep this sync.
  const gameplayRead=!!(req.roomGet||req.chatList||req.inspectionList||req.roomPlayerDetails||req.roomBanList);
  if(roomId&&!gameplayRead)await synchronizeGameplay(tx,this.db.epoch,this.compatibility.capabilities,this.now,{db:this.db,compatibility:this.compatibility,roomId});
  if(roomId&&(req.configure||req.endRun||req.finalStart||req.roomDelete||req.roomPolicy||req.roomModerate||req.roomAccess)){const r=(await tx.query('SELECT current_run_id FROM rooms WHERE id=$1',[roomId])).rows[0];if(r?.current_run_id)markBankDirty(tx,r.current_run_id);}
  return result;
 }
}
