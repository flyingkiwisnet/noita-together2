import type {Tx} from '../g1/database.js';
import {demand,fingerprint} from '../g1/security.js';
import {HourglassLedger,type HourglassActor} from '../../../../packages/domain/src/g3-hourglass.js';
import {storeHourglass,loadHourglass} from './hourglass-storage.js';
export const recipientKey=(principal:string,world:string)=>principal+':'+world;
const eventRow=(r:any)=>r&&({...r,event_id:r.activation_id,body:r.c2_body,digest:r.c2_digest,source_principal:r.c2_body.sourcePrincipalId,source_world:r.c2_body.sourceWorldToken,source_device:r.c2_device,source_state:r.c2_source_state,source_reason:r.c2_source_reason,sequence:r.c2_sequence,hourglass:true});
export async function c2Event(tx:Tx,run:string,id:string){return (await tx.query('SELECT * FROM c2_events WHERE run_id=$1 AND event_id=$2 FOR UPDATE',[run,id])).rows[0]??eventRow((await tx.query('SELECT * FROM g3_hourglass_activations WHERE run_id=$1 AND activation_id=$2 AND c2_body IS NOT NULL FOR UPDATE',[run,id])).rows[0]);}
export async function c2StoreEvent(tx:Tx,run:string,room:string,body:any,digest:string,device:string,worlds:any[],sender:any){
 if(!body.payload.hourglass){await tx.query('INSERT INTO c2_events(run_id,event_id,room_id,source_principal,source_world,source_device,sequence,rules_revision,digest,body,source_state) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)',[run,body.eventId,room,body.sourcePrincipalId,body.sourceWorldToken,device,body.sequence,body.rulesRevision,digest,body,body.senderDisposition]);for(const w of worlds)await tx.query('INSERT INTO c2_deliveries(run_id,event_id,recipient_principal,recipient_world,sequence,recipient_progression) VALUES($1,$2,$3,$4,$5,$6)',[run,body.eventId,w.scope.principalId,w.scope.worldToken,body.sequence,body.sourceProgression]);return;}
 await tx.query('SELECT id FROM g3_runs WHERE id=$1 FOR UPDATE',[run]);demand(!(await tx.query('SELECT 1 FROM g3_hourglass_activations WHERE run_id=$1 AND activation_id=$2',[run,body.eventId])).rowCount,'CONFLICT');
 const p=body.payload.hourglass,source=worlds.find(w=>w.scope.principalId===body.sourcePrincipalId);demand(!source,'INVALID_MESSAGE');
 const actor=(w:any):HourglassActor=>({...w.scope,principalId:w.scope.principalId,gameSessionId:w.scope.sessionId,name:w.displayName,live:true,mode:'core',capabilities:['core.hourglass'],catalogueRevision:p.catalogueRevision});
 // Reuse the existing authoritative plan/record and storage, one activation per row.
 const ledger=new HourglassLedger({records:[]}),record=ledger.activate(actor(sender),{activationId:body.eventId,effectId:p.effectId,materialId:p.materialId},[sender,...worlds].map(actor));
 const work=Object.fromEntries(worlds.map(w=>[recipientKey(w.scope.principalId,w.scope.worldToken),{state:'pending',reason:'',issued:false,recipient_progression:body.sourceProgression}]));
 const sequence=String(BigInt((await tx.query('SELECT coalesce(max(sequence),0) AS n FROM g3_hourglass_activations WHERE run_id=$1',[run])).rows[0].n)+1n);
 await tx.query('INSERT INTO g3_hourglass_activations(run_id,activation_id,sequence,record,c2_body,c2_digest,c2_device,c2_sequence,c2_source_state,c2_work) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)',[run,body.eventId,sequence,storeHourglass(record),body,digest,device,body.sequence,body.senderDisposition,work]);
}
export async function c2Pending(tx:Tx,run:string,principal:string,world:string,includeUncertain=true){
 const key=recipientKey(principal,world),states=includeUncertain?['pending','uncertain']:['pending'],ordinary=(await tx.query("SELECT e.*,d.state,d.reason,d.issued,d.recipient_progression FROM c2_deliveries d JOIN c2_events e USING(run_id,event_id) WHERE d.run_id=$1 AND d.recipient_principal=$2 AND d.recipient_world=$3 AND d.state=ANY($4::text[]) ORDER BY d.sequence LIMIT 8",[run,principal,world,states])).rows;
 const glass=(await tx.query("SELECT * FROM g3_hourglass_activations WHERE run_id=$1 AND c2_body IS NOT NULL AND (c2_work @> $2::jsonb OR c2_work @> $3::jsonb) ORDER BY c2_sequence LIMIT 8",[run,JSON.stringify({[key]:{state:'pending'}}),JSON.stringify({[key]:{state:includeUncertain?'uncertain':'pending'}})])).rows.map(r=>({...eventRow(r),...r.c2_work[key]}));
 return [...ordinary,...glass].sort((a,b)=>BigInt(a.sequence)<BigInt(b.sequence)?-1:1).slice(0,8);
}
export async function c2SetDelivery(tx:Tx,event:any,principal:string,world:string,state:string,reason:string,issued=true){
 if(!event.hourglass){await tx.query('UPDATE c2_deliveries SET state=$5,reason=$6,issued=$7 WHERE run_id=$1 AND event_id=$2 AND recipient_principal=$3 AND recipient_world=$4',[event.run_id,event.event_id,principal,world,state,reason,issued]);return;}
 const key=recipientKey(principal,world),row=(await tx.query('SELECT * FROM g3_hourglass_activations WHERE run_id=$1 AND activation_id=$2 FOR UPDATE',[event.run_id,event.event_id])).rows[0];demand(row.c2_work[key],'NOT_FOUND');const work={...row.c2_work,[key]:{...row.c2_work[key],state,reason,issued}},record=loadHourglass(row.record);
 if(state!=='pending'){const d=record.deliveries.find(d=>d.recipient.principalId===principal&&d.recipient.worldToken===world);demand(d,'NOT_FOUND');const outcome=state==='uncertain'?'unknown':state==='applied'?'applied_reported':'not_applied_reported';if(d.outcome!=='pending')demand(d.outcome===outcome,'CONFLICT');d.outcome=outcome;d.receiptId=event.event_id;}
 await tx.query('UPDATE g3_hourglass_activations SET c2_work=$3,record=$4 WHERE run_id=$1 AND activation_id=$2',[event.run_id,event.event_id,work,storeHourglass(record)]);
}
export async function c2SourceReceipt(tx:Tx,e:any,p:any){const reason=p.reason??'';if(e.source_state!=='pending'){demand(e.source_state===p.state&&(e.source_reason??'')===reason,'CONFLICT');return true;}if(e.hourglass)await tx.query('UPDATE g3_hourglass_activations SET c2_source_state=$3,c2_source_reason=$4 WHERE run_id=$1 AND activation_id=$2',[e.run_id,e.event_id,p.state,reason]);else await tx.query('UPDATE c2_events SET source_state=$3,source_reason=$4 WHERE run_id=$1 AND event_id=$2',[e.run_id,e.event_id,p.state,reason]);e.source_state=p.state;e.source_reason=reason;return false;}
export const c2Digest=(principal:string,original:any)=>fingerprint({principal,...original});
