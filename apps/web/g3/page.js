import {HostClient,fetchTransport} from '../candidate/client.js';
import {StatisticsClient} from './client.js';
import {renderStatistics,renderRunChoices} from './statistics.js';
export function mountStatisticsPage(browser,statistics,document){
 const $=id=>document.getElementById(id);let scope=null,cursor='0',busy=false,selection=0,runs=[],historyCursor='0',historyHigh;
 function clear(){scope=null;cursor='0';$('records').replaceChildren();$('totals').replaceChildren();$('run').textContent='';$('next').disabled=true;$('reload').disabled=true;$('delete').disabled=true;}
 function status(text){$('status').textContent=text;}
 function buttons(){const signed=!!browser.credentials;$('rooms').disabled=busy||!signed;$('history-first').disabled=busy||!signed;$('history-next').disabled=busy||!signed||historyHigh===undefined||BigInt(historyCursor)>=BigInt(historyHigh);}
 async function action(fn){if(busy)return;busy=true;buttons();try{await fn();}catch(e){if(['UNAUTHENTICATED','CANCELLED'].includes(e.code)){++selection;clear();runs=[];renderRunChoices($('rooms'),[]);historyHigh=undefined;}status(e.code??'SERVICE_UNAVAILABLE');}finally{busy=false;buttons();}}
 async function page(after='0'){
  const own=scope,version=selection;if(!own)return;
  const response=await statistics.request(own,{statisticsList:{afterSequence:after,limit:32}});
  if(version!==selection||own!==scope)return;
  renderStatistics($('records'),response);cursor=response.nextSequence??after;$('next').disabled=(response.statistics??[]).length<32;
  const total=await statistics.request(own,{statisticsAggregate:{}});if(version!==selection||own!==scope)return;
  $('totals').replaceChildren(...(total.aggregates??[]).map(v=>{const li=document.createElement('li');li.textContent=`${v.kind} · ${v.count} · ${v.provenance}`;return li;}));status(response.statistics?.length?'Statistics loaded.':'No retained events. Collection expires after 30 days; deletion and expiry do not erase run history.');
 }
 async function history(after='0',through){
  const version=++selection;clear();const r=await statistics.discover(after,through,32);if(version!==selection)return;
  runs=r.runs??[];historyCursor=r.nextSequence;historyHigh=r.highSequence;renderRunChoices($('rooms'),runs);$('rooms').value='';status(runs.length?'Choose a recorded run.':'No permitted runs on this page.');
 }
 $('login').addEventListener('click',()=>action(async()=>{++selection;clear();await browser.initialize();await browser.login('website-local-alex');await history();$('logout').disabled=false;$('login').disabled=true;}));
 $('history-first').addEventListener('click',()=>action(()=>history()));$('history-next').addEventListener('click',()=>action(()=>history(historyCursor,historyHigh)));
 $('rooms').addEventListener('change',()=>action(async()=>{
  ++selection;clear();const run=runs.find(r=>r.sequence===$('rooms').value);if(!run){status('Choose a recorded run.');return;}
  scope=run.scope;$('run').textContent=`Run ${run.scope.runId} · ${run.collectionState} · retention ${run.retentionDays} days`;$('reload').disabled=false;$('delete').disabled=!run.canDelete;await page();
 }));
 $('reload').addEventListener('click',()=>action(()=>page()));$('next').addEventListener('click',()=>action(()=>page(cursor)));
 $('delete').addEventListener('click',()=>action(async()=>{if(!scope)return;const own=scope;await statistics.request(own,{statisticsDelete:{}});if(own!==scope)return;const run=runs.find(r=>r.scope===own);if(run){run.collectionState='deleted';const selected=$('rooms').value;renderRunChoices($('rooms'),runs);$('rooms').value=selected;}$('run').textContent=`Run ${own.runId} · deleted · retention 30 days`;await page();}));
 $('logout').addEventListener('click',()=>action(async()=>{++selection;clear();runs=[];historyHigh=undefined;renderRunChoices($('rooms'),[]);try{await browser.logout();}finally{$('login').disabled=false;$('logout').disabled=true;status('Signed out.');}}));
 buttons();return {history,page};
}
if(typeof document!=='undefined'){const browser=new HostClient(fetchTransport);mountStatisticsPage(browser,new StatisticsClient(browser,fetchTransport),document);}
