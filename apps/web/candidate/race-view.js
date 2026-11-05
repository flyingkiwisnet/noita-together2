import {ServiceError,validate} from './client.js';
import {applyRaceDraft,RACE_DEFAULT,raceGoalLabel} from '../../../packages/contracts/src/race-goal-ui.js';
export {applyRaceDraft,RACE_DEFAULT};
import {createRaceEditor as canonicalRaceEditor} from '../../../packages/contracts/src/race-goal-editor.js';
export function createRaceEditor(document,form,catalogue){return canonicalRaceEditor(document,form,catalogue,RACE_DEFAULT);}
export function createRaceTransport(client){let schema,service='';return async payload=>{
 const info=client.info;if(!info)throw new ServiceError('INITIALIZATION_REQUIRED');const key=info.serviceId+':'+info.serviceEpoch;
 if(service!==key){schema=null;service=key;}if(!schema){schema=await client.transport('/v3/race/schema');if(schema.revision!=='g3-race-1'||client.info!==info)throw new ServiceError('STALE_EPOCH');}
 const request={major:1,revision:'g3-race-1',serviceId:info.serviceId,serviceEpoch:info.serviceEpoch,requestId:globalThis.crypto.randomUUID(),...(client.credentials?{sessionEpoch:client.credentials.sessionEpoch}:{}),...payload};validate(schema,'Request',request);
 const response=await client.transport('/rpc/g3/race',request,client.credentials?.accessToken);if(client.info!==info)throw new ServiceError('STALE_EPOCH');validate(schema,'Response',response);
 if(response.major!==1||response.requestId!==request.requestId||response.serviceId!==info.serviceId||response.serviceEpoch!==info.serviceEpoch||response.revision!==request.revision)throw new ServiceError('INVALID_MESSAGE');if(response.error)throw new ServiceError(response.error.code);return response;
};}
export function createRaceStatisticsTransport(client,statistics){const request=createRaceTransport(client);let catalogue,service;return async payload=>{
 const info=client.info;
 if(!payload.read||(client.info?.compatibility?.capabilities??[]).includes('race.goals.v1')===false)return statistics(payload);
 let result=await request({read:{locator:payload.read.locator,limit:8}}),snapshot=result.snapshot;
 if(snapshot.collectionState==='none'||snapshot.lobbyMode!=='race')return statistics(payload);
 const entrants=[...(snapshot.entrants??[])];let pages=0;
 while(snapshot.nextRow){if(++pages>3)throw new ServiceError('CAPACITY');const next=await request({read:{locator:snapshot.locator,afterRow:snapshot.nextRow,limit:8,runKey:snapshot.runKey,snapshotRevision:snapshot.revision}});if(next.snapshot.runKey!==snapshot.runKey||next.snapshot.revision!==snapshot.revision)throw new ServiceError('STALE_REVISION');entrants.push(...next.snapshot.entrants??[]);snapshot=next.snapshot;}
 if(service!==client.info){catalogue=await client.transport('/v3/race/catalogue');service=client.info;}if(client.info!==info)throw new ServiceError('STALE_EPOCH');return {race:{...snapshot,entrants,catalogue}};
};}
export function renderRaceStatistics(document,snapshot){
 const help=document.getElementById('statistics-help');if(help)help.textContent='Race progress is game-reported. Times use server acceptance; new reports at or after the deadline do not count. Full finishers retain their order; unfinished entrants use completed goals then the first time reaching that count. Exact ties share a place. Results expire30 days after End, or on lobby deletion.';
 const rows=document.getElementById('statistics-rows'),table=rows.closest('table'),head=table.querySelector('thead');rows.replaceChildren();head.replaceChildren();head.dataset.race='true';
 const header=document.createElement('tr');for(const label of ['Place','Player','Goals','Time to count / finish','Outcome']){const th=document.createElement('th');th.scope='col';th.textContent=label;header.append(th);}head.append(header);
 document.getElementById('statistics-title').textContent=snapshot.lobbyName??'Race';document.getElementById('statistics-status').textContent=snapshot.collectionState==='expired'?'These Race results have expired.':snapshot.status==='ended'?'Final Race standings'+(snapshot.endReason==='race_deadline'?' · Time limit reached':snapshot.endReason==='host_ended'?' · Host ended the race':snapshot.endReason==='race_all_terminal'?' · All entrants terminal':''):'Race in progress · Provisional count/time standings';
 document.getElementById('statistics-time').textContent=snapshot.startedAt?'Started '+new Date(snapshot.startedAt).toLocaleString()+' · Elapsed '+Math.floor(Number(snapshot.elapsedMs)/1000)+'s'+(snapshot.deadlineAt?' · Deadline '+new Date(snapshot.deadlineAt).toLocaleString():''):'';
 if(snapshot.collectionState!=='available')return;
 for(const entrant of [...snapshot.entrants].sort((a,b)=>a.rank-b.rank)){
  const tr=document.createElement('tr');for(const value of [entrant.rank,entrant.displayName,entrant.completedGoals+' / '+snapshot.config.goals.length,entrant.finalCountAtMs===''?'—':(Number(entrant.finalCountAtMs)/1000).toFixed(3)+'s',entrant.status]){const td=document.createElement('td');td.textContent=String(value);tr.append(td);}rows.append(tr);
  const detail=document.createElement('tr'),cell=document.createElement('td');cell.colSpan=5;const list=document.createElement('ul');
  for(const goal of snapshot.config.goals){const p=entrant.progress.find(v=>v.instanceId===goal.instanceId),item=document.createElement('li');item.textContent=raceGoalLabel(goal,snapshot.catalogue)+': '+p.value+' / '+p.target+(p.complete?' · Complete at '+(Number(p.completedAtMs)/1000).toFixed(3)+'s':'');if(goal.bossIds){const checklist=document.createElement('ul');for(const id of goal.bossIds){const boss=document.createElement('li'),credit=p.credits?.find(c=>c.id===id);boss.textContent=(credit?'✓ ':'○ ')+(snapshot.catalogue?.content.bosses.find(b=>b.id===id)?.name??id)+(credit?' · '+(Number(credit.firstAtMs)/1000).toFixed(3)+'s':'');checklist.append(boss);}item.append(checklist);}list.append(item);}cell.append(list);detail.append(cell);rows.append(detail);
 }
}
