import type {Tx} from '../g1/database.js';
import {demand} from '../g1/security.js';
import type {HourglassActor} from '../../../../packages/domain/src/g3-hourglass.js';
// Read the existing selected inspection/playing authority, never synthetic caller flags.
export async function hourglassRoster(tx:Tx,runId:string,epoch:string,capabilities:readonly string[],catalogueRevision:string,now:number):Promise<(HourglassActor&{sessionEpoch:string;settings:{key:string;value:string}[]})[]>{
  const rows=(await tx.query(`SELECT a.id AS attachment_id,a.game_session_id,w.world_token,s.epoch AS session_epoch,f.account_id,p.display_name,n.id AS run_id,n.epoch AS run_epoch,n.config_revision,r.id AS room_id,r.config
   FROM runs n JOIN rooms r ON r.id=n.room_id JOIN inspected_releases x ON x.run_id=n.id
   JOIN attachments a ON a.run_id=n.id JOIN sessions s ON s.id=a.game_session_id JOIN sessions d ON d.id=a.desktop_id AND s.parent_id=d.id
   JOIN session_families f ON f.id=s.family_id AND d.family_id=f.id JOIN principal_profiles p ON p.principal_id=f.account_id
   JOIN memberships m ON m.room_id=r.id AND m.desktop_id=d.id AND m.account_id=f.account_id JOIN world_observations w ON w.attachment_id=a.id
   JOIN authorization_play_entries e ON e.authorization_uuid=x.authorization_uuid AND e.attachment_id=a.id
   WHERE n.id=$1 AND n.state='running' AND r.state='running' AND n.service_epoch=$2 AND a.service_epoch=$2
   AND a.run_epoch=n.epoch AND a.config_revision=n.config_revision AND r.config_revision=n.config_revision
   AND x.revoked_at IS NULL AND a.revoked_at IS NULL AND s.revoked_at IS NULL AND d.revoked_at IS NULL AND f.revoked_at IS NULL AND f.expires_at>$3
   AND m.selected AND m.presence_until>$3 AND a.presence_until>$3 AND w.body->>'protected'='false' AND w.body->>'loaded'='true'
   AND EXISTS(SELECT 1 FROM jsonb_array_elements(x.selected_reports->'participants') t WHERE t->>'desktopId'=d.id::text AND t->>'attachmentId'=a.id::text AND t->>'worldToken'=w.world_token AND t->>'contentDigest'=w.content_digest)
   ORDER BY a.game_session_id LIMIT 33`,[runId,epoch,new Date(now)])).rows;
  demand(rows.length<=32,'CAPACITY');
 return rows.map(v=>({principalId:v.account_id,gameSessionId:v.game_session_id,attachmentId:v.attachment_id,worldToken:v.world_token,name:v.display_name,roomId:v.room_id,runId:v.run_id,runEpoch:v.run_epoch,configRevision:v.config_revision,mode:v.config.mode,live:true,capabilities:v.config.requiredCapabilities.filter((cap:string)=>capabilities.includes(cap)),catalogueRevision,sessionEpoch:v.session_epoch,settings:v.config.settings??[]}));
}

// Same selected authority for other isolated gameplay adapters.
export const selectedGameplayRoster=hourglassRoster;
