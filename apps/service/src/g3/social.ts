import type {RoomAdmissions} from './admission.js';
import type {Actor} from '../g1/auth.js';
import type {Tx} from '../g1/database.js';
import {demand,fingerprint,PolicyError} from '../g1/security.js';
import {authorizeSocial,checkSocialBudget,mergeRetainedHud,SocialPolicyError,SOCIAL_CATALOG,emotePresentation,type SocialIntent,type SocialSender,type RetainedHud} from '../../../../packages/domain/src/g3-social.js';
import type {Request,Response,Scope,Projection,Marker,Player} from '../../../../packages/contracts/g3-social/g3-social.js';
import {transientEventId} from '../../../../packages/contracts/src/g3-social-replay.js';
const uuid=(v:unknown)=>typeof v==='string'&&/^[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}$/.test(v);
const coordinate=(v:unknown)=>{demand(typeof v==='string'&&/^(0|-?[1-9][0-9]{0,9})$/.test(v)&&Math.abs(Number(v))<=2147483647,'INVALID_MESSAGE');return Number(v);};
const kinds=['hud','emote','cosmetic','poi'] as const;
const marker=(row:any):Marker=>{const {operationSequence:_,...body}=row.body;return {...body,poiSequence:String(row.sequence)};};
type Kind=typeof kinds[number];
interface Live extends SocialSender {attachmentId:string;runEpoch:string;configRevision:number;displayName:string;sessionEpoch:string}
const scopeOf=(c:Live):Scope=>({roomId:c.roomId,runId:c.runId,runEpoch:c.runEpoch,configRevision:c.configRevision,attachmentId:c.attachmentId,worldToken:c.worldToken});

export class SocialAuthority {
 admission?: (tx:Tx,roomId:string)=>Promise<RoomAdmissions>;
 roomBudget?: (roomId:string,tx:Tx)=>void;
 constructor(readonly epoch:()=>string,readonly capabilities:readonly string[],readonly now:()=>number=Date.now,readonly recoverableWorlds=false){}
 // Reuse the selected inspection/playing authority itself, including both leases.
 // No caller-supplied boolean, principal, capabilities or context can grant access.
 private async roster(tx:Tx,runId:string,includeWaiting=false,canonical?:RoomAdmissions):Promise<Live[]>{
  if(this.admission){let a=canonical;if(!a){const room=(await tx.query('SELECT room_id FROM runs WHERE id=$1',[runId])).rows[0];if(!room)return [];a=await this.admission(tx,room.room_id);}return a.worlds.filter(w=>w.decision.playing||includeWaiting&&w.rosterEligible&&!w.decision.terminal&&w.decision.authorized).map(w=>({principalId:w.scope.principalId,gameSessionId:w.owner.gameSessionId,worldToken:w.scope.worldToken,roomId:w.scope.roomId,runId:w.scope.runId,mode:w.mode,live:true,attachmentId:w.owner.attachmentId,runEpoch:w.scope.runEpoch,configRevision:w.scope.configRevision,displayName:w.displayName,sessionEpoch:w.owner.gameSessionEpoch,capabilities:w.capabilities}));}
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
   ORDER BY a.game_session_id LIMIT 33`,[runId,this.epoch(),new Date(this.now())])).rows;
  demand(rows.length<=32,'CAPACITY');
  return rows.map(v=>({principalId:v.account_id,gameSessionId:v.game_session_id,worldToken:v.world_token,roomId:v.room_id,runId:v.run_id,mode:v.config.mode,live:true,attachmentId:v.attachment_id,runEpoch:v.run_epoch,configRevision:v.config_revision,displayName:v.display_name,sessionEpoch:v.session_epoch,capabilities:(v.config.requiredCapabilities as string[]).filter(cap=>this.capabilities.includes(cap))}));
 }
 private async context(tx:Tx,actor:Actor,input:Request){
  const scope=input.scope;demand(scope&&uuid(scope.runId)&&uuid(scope.roomId)&&uuid(scope.runEpoch)&&uuid(scope.attachmentId)&&typeof scope.worldToken==='string'&&Buffer.byteLength(scope.worldToken)>=1,'INVALID_MESSAGE');
  demand(actor.kind==='game','FORBIDDEN');
  // One canonical view for this locked transaction only; sender/recipients and
  // projection eligibility use the same facts. Never cached across requests.
  let canonical:RoomAdmissions|undefined;if(this.admission){const room=(await tx.query('SELECT room_id FROM runs WHERE id=$1',[scope.runId])).rows[0];if(room)canonical=await this.admission(tx,room.room_id);}
  const live=await this.roster(tx,scope.runId!,false,canonical),sender=live.find(c=>c.gameSessionId===actor.sessionId&&c.principalId===actor.accountId&&c.sessionEpoch===actor.sessionEpoch);
  demand(sender&&fingerprint(scopeOf(sender))===fingerprint(scope),'STALE_EPOCH');
  demand(['core','nemesis','race'].includes(sender.mode)&&sender.capabilities.some(cap=>SOCIAL_CATALOG.some(e=>e.capability===cap)),'FORBIDDEN');
  this.roomBudget?.(sender.roomId,tx);return {sender,live,canonical};
 }
 private limit(value:number|undefined){demand(value===undefined||Number.isInteger(value)&&value>=1&&value<=32,'INVALID_MESSAGE');return value??32;}
 private cursor(value:string|undefined){const v=value??'0';demand(/^(0|[1-9][0-9]{0,18})$/.test(v)&&BigInt(v)<=9223372036854775807n,'INVALID_MESSAGE');return v;}
 private async readBudget(tx:Tx,sender:Live){
  const old=(await tx.query("SELECT * FROM g3_social_fences WHERE run_id=$1 AND game_session_id=$2 AND kind='read'",[sender.runId,sender.gameSessionId])).rows[0];
  const now=this.now();demand(!old||+old.window_at<=now,'CAPACITY');const window=old&&+old.window_at+1000>now?+old.window_at:now,count=old&&window===+old.window_at?old.window_count+1:1;demand(count<=10,'CAPACITY');
  await tx.query("INSERT INTO g3_social_fences VALUES($1,$2,'read',0,$3,$4,NULL) ON CONFLICT(run_id,game_session_id,kind) DO UPDATE SET window_at=EXCLUDED.window_at,window_count=EXCLUDED.window_count",[sender.runId,sender.gameSessionId,new Date(window),count]);
 }
 async execute(tx:Tx,actor:Actor,input:Request):Promise<Partial<Response>>{
  try{return await this.apply(tx,actor,input);}catch(e){if(e instanceof SocialPolicyError)throw new PolicyError(e.code);throw e;}
 }
 private async apply(tx:Tx,actor:Actor,input:Request):Promise<Partial<Response>>{
  const {sender,live,canonical}=await this.context(tx,actor,input);
  if(input.publish){
   const p=input.publish;demand(uuid(p.eventId)&&Number.isInteger(p.sequence)&&p.sequence!>=1&&p.sequence!<=0xffffffff,'INVALID_MESSAGE');
   const kind=kinds.find(k=>p[k]!==undefined)!;demand(kind,'INVALID_MESSAGE');
   // Bounded metadata cannot remember arbitrary UUIDs forever. Bind transient
   // identity to its immutable authority/kind/sequence before looking up retries.
   demand(kind==='poi'?p.eventId![14]!=='8':p.eventId===transientEventId(scopeOf(sender),sender.gameSessionId,kind,p.sequence!),'CONFLICT');
   const payload={...p[kind]} as Record<string,unknown>;
   if(kind==='hud'){payload.hpKnown??=false;payload.sampoKnown??=false;payload.hpMilli??=0;payload.maxHpMilli??=0;payload.sampo??=false;}
   if(kind==='emote')payload.active??=false;
   if(kind==='cosmetic'){payload.hat??=false;payload.amulet??=false;payload.gem??=false;}
   const intent=(kind==='poi'?{catalogId:'world.poi',...payload,xMilli:coordinate(p.poi!.xMilli),yMilli:coordinate(p.poi!.yMilli)}:{catalogId:'player.'+kind,...payload}) as SocialIntent;
   // Protobuf omits false scalars; normalize only documented optional false defaults.
   if(intent.catalogId==='player.hud')intent.sampo??=false;
   if(intent.catalogId==='player.emote')intent.active??=false;
   if(intent.catalogId==='player.cosmetic'){intent.hat??=false;intent.amulet??=false;intent.gem??=false;}
   const event=authorizeSocial(sender,intent,live),digest=fingerprint({scope:scopeOf(sender),publish:{eventId:p.eventId,sequence:p.sequence,intent}});
   if(kind==='poi'){
    const original=(await tx.query('SELECT * FROM g3_social_pois WHERE run_id=$1 AND event_id=$2 ORDER BY sequence LIMIT 1',[sender.runId,p.eventId])).rows[0];
    if(original){demand(original.body.principalId===sender.principalId&&original.body.scope.worldToken===sender.worldToken&&original.body.operationSequence===p.sequence&&fingerprint(original.body.poi)===fingerprint(payload),'CONFLICT');return {acceptedEventId:p.eventId!,duplicate:true};}
   }
   const old=(await tx.query(`SELECT digest FROM g3_social_ids WHERE run_id=$1 AND game_session_id=$2 AND event_id=$3
    UNION ALL SELECT digest FROM g3_social_replay WHERE run_id=$1 AND game_session_id=$2 AND event_id=$3`,[sender.runId,sender.gameSessionId,p.eventId])).rows[0];
   if(old){demand(old.digest===digest,'CONFLICT');return {acceptedEventId:p.eventId!,duplicate:true};}
   const fence=(await tx.query('SELECT * FROM g3_social_fences WHERE run_id=$1 AND game_session_id=$2 AND kind=$3',[sender.runId,sender.gameSessionId,kind])).rows[0];
   demand(p.sequence!>Number(fence?.last_sequence??0),'CONFLICT');
   const dead=(await tx.query('SELECT dead FROM g3_social_life WHERE run_id=$1 AND game_session_id=$2',[sender.runId,sender.gameSessionId])).rows[0]?.dead;
   demand(!dead||kind==='hud','FORBIDDEN');
   const pois=kind==='poi'?(await tx.query(`SELECT p.body->'poi' AS poi FROM g3_social_pois p WHERE p.run_id=$1
    AND NOT EXISTS(SELECT 1 FROM g3_social_dismissals d WHERE d.run_id=p.run_id AND d.game_session_id=$2 AND d.poi_sequence=p.sequence)`,[sender.runId,sender.gameSessionId])).rows:[];
   if(kind==='poi')demand(+(await tx.query('SELECT count(*) FROM g3_social_pois WHERE run_id=$1',[sender.runId])).rows[0].count<256,'CAPACITY');
   const previousPoi=kind==='poi'?(await tx.query("SELECT created_at FROM g3_social_pois WHERE run_id=$1 AND body->>'principalId'=$2 AND body->'scope'->>'worldToken'=$3 ORDER BY sequence DESC LIMIT 1",[sender.runId,sender.principalId,sender.worldToken])).rows[0]:undefined;
   const presentation=kind==='emote'||kind==='cosmetic'?(await tx.query('SELECT * FROM g3_social_presentation WHERE run_id=$1 AND world_token=$2 AND kind=$3 FOR UPDATE',[sender.runId,sender.worldToken,kind])).rows[0]:undefined;
   demand(!presentation||presentation.principal_id===sender.principalId,'CONFLICT');
   const cleanup=kind==='emote'&&payload.active===false&&presentation?.body.gameSessionId===sender.gameSessionId&&presentation.body.emote.active===true;
   const now=this.now(),budget=cleanup?{windowAt:fence?+fence.window_at:this.now(),windowCount:fence?.window_count??0}:checkSocialBudget(intent,now,{windowAt:fence?+fence.window_at:now,windowCount:fence?.window_count??0,...(previousPoi?{lastPoiAt:+previousPoi.created_at}:fence?.last_poi_at?{lastPoiAt:+fence.last_poi_at}:{}),pois:pois.map(v=>({xMilli:Number(v.poi.xMilli),yMilli:Number(v.poi.yMilli)}))});
   if(kind==='poi')await tx.query('INSERT INTO g3_social_ids VALUES($1,$2,$3,$4,$5)',[sender.runId,sender.gameSessionId,p.eventId,digest,new Date(now)]);
   else{
    await tx.query('INSERT INTO g3_social_replay VALUES($1,$2,$3,$4,$5,$6)',[sender.runId,sender.gameSessionId,kind,p.sequence,p.eventId,digest]);
    // Retire only this sender/kind's oldest answers. Never reset its high-water
    // fence, budgets or latest state, and never evict another sender/kind.
    await tx.query(`DELETE FROM g3_social_replay WHERE run_id=$1 AND game_session_id=$2 AND kind=$3 AND sequence IN (
     SELECT sequence FROM g3_social_replay WHERE run_id=$1 AND game_session_id=$2 AND kind=$3 ORDER BY sequence DESC OFFSET 64)`,[sender.runId,sender.gameSessionId,kind]);
   }
   await tx.query('INSERT INTO g3_social_fences VALUES($1,$2,$3,$4,$5,$6,$7) ON CONFLICT(run_id,game_session_id,kind) DO UPDATE SET last_sequence=EXCLUDED.last_sequence,window_at=EXCLUDED.window_at,window_count=EXCLUDED.window_count,last_poi_at=EXCLUDED.last_poi_at',[sender.runId,sender.gameSessionId,kind,p.sequence,new Date(budget.windowAt),budget.windowCount,kind==='poi'?new Date(now):null]);
   const body={principalId:sender.principalId,gameSessionId:sender.gameSessionId,displayName:sender.displayName,scope:scopeOf(sender),eventId:p.eventId!,capturedAt:new Date(now).toISOString(),provenance:'validated-client-report',[kind]:payload};
   if(kind==='poi')await tx.query('INSERT INTO g3_social_pois(run_id,game_session_id,event_id,body,recipient_sessions,created_at) VALUES($1,$2,$3,$4,$5,$6)',[sender.runId,sender.gameSessionId,p.eventId,{...body,operationSequence:p.sequence!},event.recipientSessionIds,new Date(now)]);
   else if(kind==='hud'){
    const old=(await tx.query('SELECT * FROM g3_social_hud_state WHERE run_id=$1 AND world_token=$2 FOR UPDATE',[sender.runId,sender.worldToken])).rows[0];
    demand(!old||old.principal_id===sender.principalId,'CONFLICT');const revision=Number(old?.revision??0)+1;demand(revision<=0xffffffff,'CAPACITY');
    const hud=mergeRetainedHud(old?.body.hud,payload as unknown as RetainedHud);
    await tx.query('INSERT INTO g3_social_hud_state VALUES($1,$2,$3,$4,$5) ON CONFLICT(run_id,world_token) DO UPDATE SET revision=EXCLUDED.revision,body=EXCLUDED.body',[sender.runId,sender.worldToken,sender.principalId,revision,{...body,sequence:p.sequence!,hud}]);
   }else{
    const revision=Number(presentation?.revision??0)+1;
    // Reserve the final revision for stopping the final accepted loop.
    demand(revision<=0xffffffff&&(kind!=='emote'||payload.active!==true||revision<0xffffffff),'CAPACITY');
    const latest={...body,sequence:p.sequence!};
    await tx.query('INSERT INTO g3_social_presentation VALUES($1,$2,$3,$4,$5,$6,$7) ON CONFLICT(run_id,world_token,kind) DO UPDATE SET revision=EXCLUDED.revision,started_at=EXCLUDED.started_at,body=EXCLUDED.body',[sender.runId,sender.worldToken,sender.principalId,kind,revision,new Date(now),latest]);
   }
    if(kind==='hud'&&intent.catalogId==='player.hud'&&intent.hpKnown===true){
     const c1=(await tx.query('SELECT state FROM c1_life WHERE run_id=$1 AND principal_id=$2 AND world_token=$3',[sender.runId,sender.principalId,sender.worldToken])).rows[0];const isDead=c1?['terminal','won'].includes(c1.state):intent.hpMilli===0;await tx.query('INSERT INTO g3_social_life VALUES($1,$2,$3) ON CONFLICT(run_id,game_session_id) DO UPDATE SET dead=EXCLUDED.dead',[sender.runId,sender.gameSessionId,isDead]);
     if(isDead)await this.stopPresentation(tx,sender.runId,sender.worldToken,sender.principalId);
    }
   return {acceptedEventId:p.eventId!,duplicate:false};
  }
  if(input.dismissPoi){
   demand(sender.capabilities.includes('core.poi'),'FORBIDDEN');const id=this.cursor(input.dismissPoi.poiSequence);demand(id!=='0','INVALID_MESSAGE');
   const poi=(await tx.query('SELECT sequence FROM g3_social_pois WHERE sequence=$1 AND run_id=$2',[id,sender.runId])).rows[0];demand(poi,'NOT_FOUND');
   const old=(await tx.query('SELECT 1 FROM g3_social_dismissals WHERE run_id=$1 AND game_session_id=$2 AND poi_sequence=$3',[sender.runId,sender.gameSessionId,id])).rowCount;
   if(!old){await this.readBudget(tx,sender);await tx.query('INSERT INTO g3_social_dismissals VALUES($1,$2,$3)',[sender.runId,sender.gameSessionId,id]);}return {duplicate:!!old};
  }
  await this.readBudget(tx,sender);
  if(input.poiOutcome){demand(sender.capabilities.includes('core.poi')&&uuid(input.poiOutcome.eventId),'INVALID_MESSAGE');const original=(await tx.query('SELECT * FROM g3_social_pois WHERE run_id=$1 AND event_id=$2 ORDER BY sequence LIMIT 1',[sender.runId,input.poiOutcome.eventId])).rows[0];
   if(original)demand(original.body.principalId===sender.principalId&&original.body.scope.worldToken===sender.worldToken,'FORBIDDEN');
   return {poiOutcome:{eventId:input.poiOutcome.eventId!,status:original?'accepted':'not-found',...(original?{marker:marker(original)}:{})}};
  }
  if(input.roster){
   demand((input.roster.afterKey??'')===''&&(input.roster.limit===undefined||input.roster.limit===32),'INVALID_MESSAGE');
   let players:Player[];
   if(this.admission){const admission=canonical??await this.admission(tx,sender.roomId);
    players=admission.worlds.filter(w=>w.rosterEligible&&!w.decision.terminal).map(w=>({playerId:w.scope.worldToken,principalId:w.scope.principalId,displayName:w.displayName,gameSessionId:w.scope.sessionId,worldToken:w.scope.worldToken,state:w.decision.playing?'playing':w.decision.authorized?'waiting':'disconnected'}));
   }else players=live.map(w=>({playerId:w.worldToken,principalId:w.principalId,displayName:w.displayName,gameSessionId:w.gameSessionId,worldToken:w.worldToken,state:'playing'}));
   const lives=(await tx.query('SELECT principal_id,world_token,state FROM c1_life WHERE run_id=$1',[sender.runId])).rows;players=players.map(p=>lives.some(l=>l.principal_id===p.principalId&&l.world_token===p.worldToken&&l.state==='respawning')?{...p,state:'respawning'}:p);
   if(this.capabilities.includes('core.items')){const inventories=(await tx.query('SELECT i.game_session_id,i.world_token,i.revision,i.held_sprite_id FROM g3_item_inventory i JOIN g3_contexts c ON c.attachment_id=i.attachment_id AND c.world_token=i.world_token WHERE i.run_id=$1 AND c.revoked_at IS NULL AND c.presence_until>$2',[sender.runId,new Date(this.now())])).rows;players=players.map(p=>{const i=p.state==='playing'?inventories.find(i=>i.game_session_id===p.gameSessionId&&i.world_token===p.worldToken):undefined;return {...p,inventoryRevision:i?Number(i.revision):0,heldWandSpriteId:i?.held_sprite_id??''};});}
   if(sender.mode==='race'){
    const entrants=(await tx.query(`SELECT e.*,a.game_session_id FROM race_entrants e
     LEFT JOIN rejoin_participants p ON p.run_id=e.run_id AND p.account_id=e.account_id AND p.world_token=e.world_token
     LEFT JOIN attachments a ON a.id=p.attachment_id WHERE e.run_id=$1 ORDER BY e.row_key`,[sender.runId])).rows;
    for(const e of entrants){const old=players.find(p=>p.worldToken===e.world_token&&p.principalId===e.account_id);
     const race={raceRowKey:e.row_key,raceStatus:e.status,completedGoals:e.evaluation.progress.filter((g:any)=>g.complete).length};
     if(old)Object.assign(old,race);else players.push({playerId:e.world_token,principalId:e.account_id,displayName:e.display_name,gameSessionId:e.game_session_id??'',worldToken:e.world_token,state:'disconnected',...race});
    }
   }
   demand(players.length<=32&&new Set(players.map(p=>p.playerId)).size===players.length,'CAPACITY');return {players,rosterComplete:true};
  }
  if(input.snapshot){
   const after=input.snapshot.afterKey??'';demand(after===''||/^[a-f0-9-]{36}:(hud|emote|cosmetic)$/.test(after),'INVALID_MESSAGE');
   const connected=await this.roster(tx,sender.runId,true,canonical);
   const states=sender.capabilities.includes('core.hud')?(await tx.query('SELECT * FROM g3_social_hud_state WHERE run_id=$1 AND world_token=ANY($2::text[]) LIMIT 33',[sender.runId,connected.map(p=>p.worldToken)])).rows:[];demand(states.length<=32,'CAPACITY');
   const retained=states.flatMap(v=>{const owner=connected.find(p=>p.worldToken===v.world_token&&p.principalId===v.principal_id&&p.capabilities.includes('core.hud'));if(!owner)return [];
    return [{key:owner.gameSessionId+':hud',body:{principalId:owner.principalId,gameSessionId:owner.gameSessionId,displayName:owner.displayName,scope:scopeOf(owner),hudStateRevision:Number(v.revision),capturedAt:v.body.capturedAt,provenance:'validated-client-report',hud:v.body.hud}}];});
   const current=(await tx.query('SELECT * FROM g3_social_presentation WHERE run_id=$1 AND world_token=ANY($2::text[]) LIMIT 65',[sender.runId,connected.map(p=>p.worldToken)])).rows;demand(current.length<=64,'CAPACITY');
   const dead=(await tx.query('SELECT game_session_id FROM g3_social_life WHERE run_id=$1 AND dead AND game_session_id=ANY($2::uuid[])',[sender.runId,connected.map(p=>p.gameSessionId)])).rows.map(v=>v.game_session_id);
   const rows=current.flatMap(v=>{const eligible=v.kind==='emote'?live:connected,cap=v.kind==='emote'?'core.emotes':'core.cosmetics';
    const owner=eligible.find(p=>p.worldToken===v.world_token&&p.principalId===v.principal_id&&p.capabilities.includes(cap)&&!dead.includes(p.gameSessionId));
    if(!owner||!sender.capabilities.includes(cap)||v.kind==='emote'&&v.body.gameSessionId!==owner.gameSessionId)return [];
    const body:Projection={...v.body,gameSessionId:owner.gameSessionId,displayName:owner.displayName,scope:scopeOf(owner),presentationRevision:Number(v.revision),...(v.kind==='emote'?{emote:emotePresentation(v.body.emote,+v.started_at,this.now())}:{})};
    return [{key:owner.gameSessionId+':'+v.kind,body}];});
   const page=[...rows,...retained].filter(v=>v.key>after).sort((a,b)=>a.key.localeCompare(b.key)).slice(0,Math.min(16,this.limit(input.snapshot.limit)));
   return {projections:page.map(v=>v.body),nextKey:page.at(-1)?.key??after};
  }
  const page=input.poiSnapshot??input.poiOutbox;demand(page,'INVALID_MESSAGE');demand(sender.capabilities.includes('core.poi'),'FORBIDDEN');
  const after=this.cursor(page.afterSequence),high=(await tx.query('SELECT coalesce(max(sequence),0)::text AS value FROM g3_social_pois WHERE run_id=$1',[sender.runId])).rows[0].value;
  const limit=Math.min(16,this.limit(page.limit));
  const rows=(await tx.query(`SELECT p.* FROM g3_social_pois p WHERE run_id=$1 AND sequence>$2
   AND NOT EXISTS(SELECT 1 FROM g3_social_dismissals d WHERE d.run_id=p.run_id AND d.game_session_id=$3 AND d.poi_sequence=p.sequence)
   ORDER BY sequence LIMIT $4`,[sender.runId,after,sender.gameSessionId,limit+1])).rows;
  const more=rows.length>limit,visible=rows.slice(0,limit);
  // A dismissed tail still advances to the frozen high-water cursor, so a
  // recipient can finish without repeatedly requesting an empty page.
  return {markers:visible.map(marker),nextSequence:more?visible.at(-1)!.sequence:(BigInt(after)>BigInt(high)?after:high),highSequence:high};
 }
 private async stopPresentation(tx:Tx,runId:string,worldToken:string,principalId:string){
  await tx.query("UPDATE g3_social_presentation SET body=jsonb_set(body,'{emote,active}','false'::jsonb),revision=revision+1 WHERE run_id=$1 AND world_token=$2 AND principal_id=$3 AND kind='emote' AND body->'emote'->>'active'='true' AND revision<4294967295",[runId,worldToken,principalId]);
 }
 async recordDeath(tx:Tx,actor:Actor,scope:Scope){
  await this.stopPresentation(tx,scope.runId!,scope.worldToken!,actor.accountId);
  await tx.query('INSERT INTO g3_social_life VALUES($1,$2,true) ON CONFLICT(run_id,game_session_id) DO UPDATE SET dead=true',[scope.runId,actor.sessionId]);
  await tx.query('DELETE FROM g3_social_latest WHERE run_id=$1 AND game_session_id=$2',[scope.runId,actor.sessionId]);
  await tx.query("UPDATE g3_social_hud_state SET revision=revision+1,body=jsonb_set(jsonb_set(body,'{hud,hpMilli}','0'::jsonb),'{capturedAt}',to_jsonb($4::text)) WHERE run_id=$1 AND world_token=$2 AND principal_id=$3 AND body->'hud'->>'hpKnown'='true' AND revision<4294967295",[scope.runId,scope.worldToken,actor.accountId,new Date(this.now()).toISOString()]);
 }
 async maintain(tx:Tx,historical=true){
  // No sample-age/liveness timer. Retain across lease gaps and same-world owner
  // replacement; remove only when its selected actor/run lifecycle is gone.
  const actorExists=this.recoverableWorlds?`EXISTS(SELECT 1 FROM rejoin_participants p WHERE p.run_id=n.id AND p.world_token=h.world_token AND p.account_id=h.principal_id AND NOT p.terminal)`:
   `EXISTS(SELECT 1 FROM attachments a JOIN sessions s ON s.id=a.game_session_id JOIN session_families f ON f.id=s.family_id JOIN memberships m ON m.desktop_id=a.desktop_id AND m.room_id=r.id JOIN world_observations w ON w.attachment_id=a.id
    WHERE a.run_id=n.id AND f.account_id=h.principal_id AND m.account_id=h.principal_id AND m.selected AND w.world_token=h.world_token)`;
  await tx.query(`DELETE FROM g3_social_hud_state WHERE ctid IN (SELECT h.ctid FROM g3_social_hud_state h WHERE NOT EXISTS(
   SELECT 1 FROM runs n JOIN rooms r ON r.id=n.room_id WHERE n.id=h.run_id AND n.state IN ('running','staging') AND r.current_run_id=n.id AND r.deleted_at IS NULL
   AND ${actorExists}) LIMIT 1000)`);
  await tx.query(`DELETE FROM g3_social_presentation WHERE ctid IN (SELECT h.ctid FROM g3_social_presentation h WHERE NOT EXISTS(SELECT 1 FROM runs n JOIN rooms r ON r.id=n.room_id WHERE n.id=h.run_id AND n.state IN ('running','staging') AND r.current_run_id=n.id AND r.deleted_at IS NULL AND ${actorExists}) LIMIT 1000)`);
  // Ineligibility stops session-bound animation once; retained cosmetic state
  // is private until the durable actor is eligible again. No second liveness timer.
  await tx.query(`UPDATE g3_social_presentation SET body=jsonb_set(body,'{emote,active}','false'::jsonb),revision=revision+1
   WHERE ctid IN (SELECT l.ctid FROM g3_social_presentation l WHERE kind='emote' AND body->'emote'->>'active'='true' AND revision<4294967295 AND NOT EXISTS(
    SELECT 1 FROM runs n JOIN attachments a ON a.run_id=n.id JOIN sessions s ON s.id=a.game_session_id JOIN sessions d ON d.id=s.parent_id JOIN session_families f ON f.id=s.family_id JOIN memberships m ON m.desktop_id=d.id AND m.room_id=n.room_id JOIN world_observations w ON w.attachment_id=a.id
    WHERE n.id=l.run_id AND n.state='running' AND n.service_epoch=$2 AND a.game_session_id::text=l.body->>'gameSessionId' AND a.revoked_at IS NULL AND a.presence_until>$1 AND m.selected AND m.presence_until>$1 AND s.revoked_at IS NULL AND d.revoked_at IS NULL AND f.revoked_at IS NULL AND f.expires_at>$1 AND w.world_token=l.world_token AND w.body->>'protected'='false'
    AND NOT EXISTS(SELECT 1 FROM g3_social_life dead WHERE dead.run_id=l.run_id AND dead.game_session_id=a.game_session_id AND dead.dead)) LIMIT 1000)`,[new Date(this.now()),this.epoch()]);
  await tx.query(`DELETE FROM g3_social_latest WHERE ctid IN (SELECT l.ctid FROM g3_social_latest l WHERE expires_at<=$1 OR NOT EXISTS(
   SELECT 1 FROM runs n JOIN attachments a ON a.run_id=n.id JOIN sessions s ON s.id=a.game_session_id JOIN sessions d ON d.id=s.parent_id JOIN session_families f ON f.id=s.family_id JOIN memberships m ON m.desktop_id=d.id AND m.room_id=n.room_id JOIN world_observations w ON w.attachment_id=a.id
   WHERE n.id=l.run_id AND n.state='running' AND n.service_epoch=$2 AND a.game_session_id=l.game_session_id AND a.revoked_at IS NULL AND a.presence_until>$1 AND m.selected AND m.presence_until>$1 AND s.revoked_at IS NULL AND d.revoked_at IS NULL AND f.revoked_at IS NULL AND w.body->>'protected'='false') LIMIT 1000)`,[new Date(this.now()),this.epoch()]);
  // Terminal authority rejects all social calls. Retain original IDs/POIs privately
  // for 30 days, then bounded per-table cleanup (dismissals cascade with markers).
  // Periodic/default maintenance owns historical retention. Current eligibility
  // cleanup above still runs before every social operation.
  if(historical)for(const table of ['g3_social_pois','g3_social_ids','g3_social_replay','g3_social_fences','g3_social_life'])await tx.query(`DELETE FROM ${table} WHERE ctid IN (SELECT t.ctid FROM ${table} t JOIN runs n ON n.id=t.run_id WHERE n.state NOT IN ('running','staging') AND coalesce(n.ended_at,n.created_at)<$1 ${table==='g3_social_ids'?'AND NOT EXISTS(SELECT 1 FROM g3_social_pois p WHERE p.run_id=t.run_id AND p.game_session_id=t.game_session_id AND p.event_id=t.event_id)':''} LIMIT 1000)`,[new Date(this.now()-30*86400000)]);
 }
}
