import {renderRaceStatistics} from './race-view.js';
import {ServiceError,validate} from './client.js';
export const STAT_COLUMNS=[['hearts','Hearts collected'],['orbs','Orbs collected'],['steveKills','Steve kills'],['deaths','Deaths'],['wins','Wins'],['goldDeposited','Gold deposited'],['goldWithdrawn','Gold withdrawn'],['itemsDeposited','Items deposited'],['itemsClaimed','Items claimed']];
export function createStatisticsTransport(client){let schema=null,service='';return async payload=>{
 const info=client.info;if(!info)throw new ServiceError('INITIALIZATION_REQUIRED');const key=info.serviceId+':'+info.serviceEpoch;
 if(service!==key){schema=null;service=key;}if(!schema){const next=await client.transport('/v3/stats/schema');if(next.revision!=='g3-stats-1'||client.info!==info)throw new ServiceError('STALE_EPOCH');schema=next;}
 const request={major:1,revision:'g3-stats-1',serviceId:info.serviceId,serviceEpoch:info.serviceEpoch,requestId:globalThis.crypto.randomUUID(),...(client.credentials?{sessionEpoch:client.credentials.sessionEpoch}:{}),...payload};validate(schema,'Request',request);
 const result=await client.transport('/v3/stats/rpc',request,client.credentials?.accessToken);if(client.info!==info)throw new ServiceError('STALE_EPOCH');validate(schema,'Response',result);if(result.requestId!==request.requestId||result.major!==1||result.revision!==request.revision||result.serviceId!==info.serviceId||result.serviceEpoch!==info.serviceEpoch)throw new ServiceError('INVALID_MESSAGE');if(result.error)throw new ServiceError(result.error.code);return result;
};}
export function elapsedText(seconds){const n=Number(seconds);if(!Number.isSafeInteger(n)||n<0)return 'Unavailable';const days=Math.floor(n/86400),hours=Math.floor(n/3600)%24,minutes=Math.floor(n/60)%60;return (days?days+'d ':'')+hours+'h '+minutes+'m '+n%60+'s';}
export function createStatisticsView({document,window,request,identity,onBack,schedule=setTimeout,cancel=clearTimeout,now=Date.now}){
 const $=id=>document.getElementById(id);const help=$('statistics-help'),coreHelp=help?.textContent;let selected=null,generation=0,inFlight=false,wanted=false,timer=null,last=-Infinity,backoff=5000,disposed=false;
 const clear=()=>{$('statistics-rows').replaceChildren();$('statistics-title').textContent='Lobby statistics';$('statistics-time').textContent='';$('statistics-status').textContent='';};
 function stop(){selected=null;generation++;wanted=false;if(timer!==null)cancel(timer);timer=null;clear();}
 function later(delay){if(timer!==null)cancel(timer);timer=schedule(()=>{timer=null;void refresh();},Math.max(0,delay));}
 function render(s){if(help)help.textContent=coreHelp;const table=$('statistics-rows').closest?.('table'),head=table?.querySelector?.('thead');if(head?.dataset?.race==='true'){const tr=document.createElement('tr');for(const label of ['Player',...STAT_COLUMNS.map(v=>v[1])]){const th=document.createElement('th');th.scope='col';th.textContent=label;tr.append(th);}head.replaceChildren(tr);head.dataset.race='false';}clear();$('statistics-title').textContent=s.lobbyName??'Lobby statistics';const state=s.collectionState;
  const messages={none:'No gameplay statistics yet.',unavailable:'Statistics unavailable for this run. It started before complete collection was available.',partial:'Some game-reported statistics are unavailable. The recorded totals below may be incomplete.',expired:'These gameplay statistics have expired.',available:s.runStatus==='live'?'Current run · Live':'Latest run · Finished'};
  $('statistics-status').textContent=messages[state]??'Statistics unavailable.';
  if(s.startedAt)$('statistics-time').textContent='Started '+new Date(s.startedAt).toLocaleString()+(s.endedAt?' · Ended '+new Date(s.endedAt).toLocaleString():'')+' · Run elapsed '+elapsedText(s.elapsedSeconds);
  if(!['available','partial'].includes(state))return;
  for(const row of s.rows??[]){const tr=document.createElement('tr'),name=document.createElement('th');name.scope='row';name.textContent=row.displayName;tr.append(name);for(const [key]of STAT_COLUMNS){const td=document.createElement('td');td.textContent=/^(0|[1-9][0-9]{0,18})$/.test(row[key]??'')?row[key]:'Unavailable';tr.append(td);}$('statistics-rows').append(tr);}
 }
 async function refresh(){
  if(disposed||!selected||document.hidden)return;if(inFlight){wanted=true;return;}const delay=last+5000-now();if(delay>0){wanted=true;later(delay);return;}
  const target=selected,turn=generation,scope=identity();inFlight=true;wanted=false;last=now();$('statistics-refresh').disabled=true;
  try{const value=await request({read:{locator:target,limit:32}});if(disposed||selected!==target||generation!==turn||identity()!==scope)return;if(value.race)renderRaceStatistics(document,value.race);else render(value.summary);backoff=5000;}
  catch(e){if(disposed||selected!==target||generation!==turn||identity()!==scope)return;clear();$('statistics-status').textContent=['NOT_FOUND','FORBIDDEN','UNAUTHENTICATED'].includes(e.code)?'This lobby’s statistics are private or no longer available.':e.code==='STALE_EPOCH'?'Service changed. Reload this page to refresh statistics.':'Statistics service unavailable. Retrying shortly.';backoff=Math.min(60000,Math.max(5000,backoff*2));}
  finally{inFlight=false;$('statistics-refresh').disabled=false;if(!disposed&&selected&&!document.hidden)later(wanted?Math.max(0,last+5000-now()):Math.max(backoff,last+5000-now()));}
 }
 const visible=()=>{if(document.hidden){if(timer!==null)cancel(timer);timer=null;}else if(selected)void refresh();};
 $('statistics-back').addEventListener('click',()=>{stop();onBack();});$('statistics-refresh').addEventListener('click',()=>void refresh());document.addEventListener?.('visibilitychange',visible);window.addEventListener('focus',visible);
 return {open(locator){stop();selected=locator;clear();$('statistics-status').textContent='Loading statistics…';void refresh();},stop,invalidate(){if(!selected)return;generation++;clear();wanted=true;void refresh();},dispose(){disposed=true;stop();document.removeEventListener?.('visibilitychange',visible);window.removeEventListener?.('focus',visible);}};
}
