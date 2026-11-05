import {fingerprint} from '../g1/security.js';
import type {Tx,Database} from '../g1/database.js';
import {contentFailure} from '../g2-next/observation.js';
import {decideWorldAdmission} from '../../../../packages/domain/src/world-admission.js';

export interface AdmissionScope {principalId:string;sessionId:string;sessionEpoch:string;roomId:string;runId:string;runEpoch:string;configRevision:number;attachmentId:string;worldToken:string}
export interface AdmissionOwner {familyId:string;desktopId:string;desktopSessionEpoch:string;gameSessionId:string;gameSessionEpoch:string;attachmentId:string;gameInstanceId:string}
export interface SelectedInspection {desktopId:string;attachmentId:string;worldToken:string;observationRevision:number;contentDigest:string;capturedAt:string;gameSessionId:string;gameSessionEpoch:string;desktopSessionEpoch:string;gameInstanceId:string;actualSeed?:string;seedPolicy?:string;randomizeLoot?:boolean}
export interface AdmissionCompatibility {modVersion:string;bridgeVersion:string;capabilities:string[]}
export interface AdmittedWorld {
 serviceId:string;serviceEpoch:string;decision:ReturnType<typeof decideWorldAdmission>;until:number;generation:number;
 // Roster identity is independent of transport/report freshness. This does not
 // grant playing authority, and may retain a fenced reconnecting participant.
 rosterEligible:boolean;
 participantId?:string;scope:AdmissionScope;owner:AdmissionOwner;
 originalStart:{requestId:string;authorizedAt:string;selectedInspection:SelectedInspection|undefined};
 immutable:{authorizationId:string;startRequestId:string;authorizedAt:string;selectedInspection:SelectedInspection|undefined};
 currentReport:{revision:number|undefined;contentDigest:string|undefined;capturedAt:string|undefined;complete:boolean;loaded:boolean;protected:boolean;contentValid:boolean};
 displayName:string;mode:string;capabilities:string[];
}
export interface RoomAdmissions {terminal:boolean;normalEnd:boolean;runId?:string;worlds:AdmittedWorld[];ownerSessionIds:string[]}
// Current-owner rows only, never historical sessions. One source of predicates for
// release, HTTP gameplay, WSS and social. No cache survives a transaction/mutation.
export async function readRoomAdmissions(tx:Tx,db:Database,compatibility:AdmissionCompatibility,now:number,roomId:string):Promise<RoomAdmissions>{
 const rejoin=!!(db as Database&{rejoinEnabled?:boolean}).rejoinEnabled;
 const currentOwnerClause=rejoin?" OR EXISTS(SELECT 1 FROM rejoin_participants current_owner WHERE current_owner.attachment_id=a.id)":'';
 // One statement/snapshot for the same canonical facts. This avoids repeated
 // database round trips during reply decoration and live projection without
 // caching authority across requests, mutations or absolute lease deadlines.
 const r=(await tx.query(`SELECT r.*,to_jsonb(n) AS admission_run,to_jsonb(x) AS admission_release,
 (SELECT COALESCE(jsonb_agg(to_jsonb(owner_rows)),'[]'::jsonb) FROM (SELECT a.*,s.epoch AS game_epoch,s.revoked_at AS game_revoked,s.parent_id,
 d.epoch AS desktop_epoch,d.revoked_at AS desktop_revoked,d.family_id AS desktop_family,
 f.id AS family_id,f.account_id,f.device_id AS owner_device,f.expires_at AS family_until,f.revoked_at AS family_revoked,
 (SELECT life.state FROM c1_life life WHERE life.run_id=n.id AND life.principal_id=f.account_id AND life.world_token=w.world_token) AS c1_life_state,u.disabled,EXISTS(SELECT 1 FROM room_bans b WHERE b.room_id=r.id AND b.account_id=f.account_id) AS banned,p.display_name,p.kind AS principal_kind,p.guest_expires_at,
 m.presence_until AS desktop_until,m.selected,m.account_id AS member_account,
 w.body,w.revision,w.world_token,w.content_digest,w.captured_at,e.entered_playing_at
 FROM attachments a JOIN sessions s ON s.id=a.game_session_id AND s.kind='game'
 JOIN sessions d ON d.id=a.desktop_id AND d.kind='desktop' JOIN session_families f ON f.id=s.family_id
 JOIN accounts u ON u.id=f.account_id JOIN principal_profiles p ON p.principal_id=u.id
 LEFT JOIN memberships m ON m.desktop_id=d.id AND m.room_id=r.id
 LEFT JOIN world_observations w ON w.attachment_id=a.id
 LEFT JOIN authorization_play_entries e ON e.attachment_id=a.id AND e.authorization_uuid=x.authorization_uuid
 WHERE a.run_id=n.id AND (a.revoked_at IS NULL${currentOwnerClause}) ORDER BY a.id LIMIT 33) owner_rows) AS admission_owners,
 ${rejoin?"(SELECT COALESCE(jsonb_agg(to_jsonb(participant_rows)),'[]'::jsonb) FROM rejoin_participants participant_rows WHERE participant_rows.run_id=n.id)":"NULL::jsonb"} AS admission_participants
 FROM rooms r LEFT JOIN runs n ON n.id=r.current_run_id LEFT JOIN inspected_releases x ON x.run_id=n.id WHERE r.id=$1`,[roomId])).rows[0];
 const n=r?.admission_run,x=r?.admission_release;
 if(!r||r.state!=='running'||!n||n.state!=='running'||n.service_epoch!==db.epoch||r.config_revision!==n.config_revision||!x||x.revoked_at)return {terminal:true,normalEnd:r?.state==='ended',worlds:[],ownerSessionIds:[]};
 const reports=x.selected_reports?.participants;
 if(!Array.isArray(reports)||reports.length>32)throw Error('CAPACITY');
 const rows=r.admission_owners as Record<string,any>[];
 if(rows.length>32)throw Error('CAPACITY');
 const participants=rejoin?r.admission_participants as Record<string,any>[]:undefined;
 const worlds:AdmittedWorld[]=[],ownerSessionIds:string[]=[];
 for(const a of rows){
  const selected=reports.find((v:any)=>v.attachmentId===a.id),p=participants?.find((v:any)=>v.attachment_id===a.id);
  const currentOwner=!a.revoked_at&&!a.game_revoked&&!a.desktop_revoked&&!a.family_revoked&&a.parent_id===a.desktop_id&&a.desktop_family===a.family_id&&(!participants||!!p&&!p.terminal);
  if(currentOwner)ownerSessionIds.push(a.game_session_id);
  const identity=fingerprint(n.config)===fingerprint(r.config)&&!!selected&&currentOwner&&!!a.selected&&a.member_account===a.account_id&&a.service_epoch===db.epoch&&a.run_epoch===n.epoch&&a.config_revision===r.config_revision
   &&a.desktop_id===selected.desktopId&&a.world_token===selected.worldToken&&a.content_digest===selected.contentDigest
   &&a.game_session_id===selected.gameSessionId&&a.game_epoch===selected.gameSessionEpoch&&a.desktop_epoch===selected.desktopSessionEpoch&&a.game_instance_id===selected.gameInstanceId
   &&(!p||p.account_id===a.account_id&&p.device_id===a.owner_device&&p.desktop_id===a.desktop_id&&p.world_token===a.world_token&&p.accepted_digest===a.content_digest);
  const rosterEligible=!!a.selected&&a.member_account===a.account_id&&a.parent_id===a.desktop_id&&a.desktop_family===a.family_id
   &&a.service_epoch===db.epoch&&a.run_epoch===n.epoch&&a.config_revision===r.config_revision&&fingerprint(n.config)===fingerprint(r.config)
   &&typeof a.world_token==='string'&&a.world_token.length>0
   &&(participants?!!p&&!p.terminal&&p.account_id===a.account_id&&p.device_id===a.owner_device&&p.desktop_id===a.desktop_id&&p.world_token===a.world_token
    :!!selected&&selected.desktopId===a.desktop_id&&selected.gameSessionId===a.game_session_id&&selected.gameSessionEpoch===a.game_epoch&&selected.desktopSessionEpoch===a.desktop_epoch&&selected.gameInstanceId===a.game_instance_id&&selected.worldToken===a.world_token);
  const terminalReason=['terminal','won'].includes(a.c1_life_state)?'player_'+a.c1_life_state:a.banned?'host_ban':a.disabled?'account_disabled':a.principal_kind==='guest'&&(!r.allow_guests||+new Date(a.guest_expires_at)<=now)?'access_revoked':p?.terminal?p.reason||'retired':undefined;
  const generation=p?.generation??1;
  const transportUntil=Math.min(+new Date(a.presence_until),a.desktop_until?+new Date(a.desktop_until):0,+new Date(a.family_until),a.guest_expires_at?+new Date(a.guest_expires_at):Infinity);
  const validReport=!!a.body&&!contentFailure(a.body,r.config,compatibility);
  const decision=decideWorldAdmission({...(terminalReason?{terminalReason}:{}),identityValid:identity,ownerActive:!participants||p?.state==='active',...(p?.reason?{ownerReason:p.reason}:{}),reportValid:validReport,protected:!!a.body?.protected,entered:!!a.entered_playing_at,generation,now,transportUntil,reviewUntil:+new Date(n.review_until)});
  const scope={principalId:a.account_id,sessionId:a.game_session_id,sessionEpoch:a.game_epoch,roomId:r.id,runId:n.id,runEpoch:n.epoch,configRevision:r.config_revision,attachmentId:a.id,worldToken:p?.world_token??a.world_token??selected?.worldToken};
  worlds.push({serviceId:db.serviceId,serviceEpoch:db.epoch,decision,until:now+decision.remainingMs,generation,rosterEligible,...(p?{participantId:p.id}:{}),scope,
   owner:{familyId:a.family_id,desktopId:a.desktop_id,desktopSessionEpoch:a.desktop_epoch,gameSessionId:a.game_session_id,gameSessionEpoch:a.game_epoch,attachmentId:a.id,gameInstanceId:a.game_instance_id},
   originalStart:{requestId:x.authorization_id,authorizedAt:x.authorized_at,selectedInspection:p?.original_report??selected},
   immutable:{authorizationId:p?.authorization_id??x.authorization_uuid,startRequestId:x.authorization_id,authorizedAt:p?.authorized_at?new Date(p.authorized_at).toISOString():x.authorized_at,selectedInspection:selected},
   currentReport:{revision:a.revision??undefined,contentDigest:a.content_digest??undefined,capturedAt:a.captured_at?new Date(a.captured_at).toISOString():undefined,complete:!!a.body?.inventoryComplete,loaded:!!a.body?.loaded,protected:!!a.body?.protected,contentValid:validReport},
   displayName:a.display_name,mode:r.config.mode,capabilities:(r.config.requiredCapabilities??[]).filter((v:string)=>compatibility.capabilities.includes(v))});
 }
 return {terminal:false,normalEnd:false,runId:n.id,worlds,ownerSessionIds};
}
