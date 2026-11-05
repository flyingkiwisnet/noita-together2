import test from 'node:test';import assert from 'node:assert/strict';import {readFile} from 'node:fs/promises';
import {controllerDom} from './dom-fixture.mjs';import {mountStatisticsPage} from '../g3/page.js';import {StatisticsClient} from '../g3/client.js';
const malicious='<svg onload=alert(1)><script>F12()</script>';
const scope={roomId:'11111111-1111-4111-8111-111111111111',runId:'22222222-2222-4222-8222-222222222222',runEpoch:'33333333-3333-4333-8333-333333333333',configRevision:1};
const run=sequence=>({sequence:String(sequence),scope:{...scope,runId:sequence===1?scope.runId:'44444444-4444-4444-8444-444444444444'},roomName:malicious,createdAt:'2026-10-06',state:'ended',collectionState:'available',retentionDays:30,canDelete:sequence===1});
test('statistics page selects bounded permitted prior runs, sends frozen scope and renders hostile F12 names literally',async()=>{
 const dom=await controllerDom('../g3/index.html');for(const id of ['rooms','records'])dom.get(id).ownerDocument=dom.document;
 const browser={credentials:null,async initialize(){},async login(){this.credentials={accountId:'owner'};},async logout(){this.credentials=null;}};
 const calls=[],discovery=[];const statistics={async discover(...args){discovery.push(args);return args[0]==='0'?{runs:[run(1),run(2)],nextSequence:'2',highSequence:'3'}:{runs:[run(3)],nextSequence:'3',highSequence:'3'};},async request(scope,payload){calls.push({scope,payload});return payload.statisticsList?{statistics:[{displayName:malicious,kind:'player.death',count:1,provenance:'validated-client-report',capturedAt:'now'}],nextSequence:'1'}:payload.statisticsAggregate?{aggregates:[{kind:malicious,count:'1',provenance:'validated-client-report'}]}:{};}};
 mountStatisticsPage(browser,statistics,dom.document);assert.equal(dom.get('rooms').disabled,true);await dom.click('login');assert.deepEqual(discovery[0],['0',undefined,32]);
 assert.equal(dom.get('rooms').children.length,3);assert.ok(dom.get('rooms').textContent.includes(malicious));assert.equal(dom.get('rooms').children[1].children.length,0);
 dom.get('rooms').value='1';await dom.get('rooms').fire('change');assert.deepEqual(calls[0].scope,scope);assert.ok(dom.get('records').textContent.includes(malicious));assert.equal(dom.get('records').children[0].children.length,0);assert.ok(dom.get('totals').textContent.includes(malicious));assert.equal(dom.get('delete').disabled,false);
 await dom.click('delete');assert.ok(calls.some(c=>c.payload.statisticsDelete&&c.scope.runId===scope.runId));assert.ok(dom.get('run').textContent.includes('deleted'));
 const count=calls.length;dom.get('rooms').value='guessed-private-id';await dom.get('rooms').fire('change');assert.equal(calls.length,count);assert.equal(dom.get('records').children.length,0);
 await dom.click('history-next');assert.deepEqual(discovery[1],['2','3',32]);assert.equal(dom.get('rooms').children.length,2);assert.equal(dom.get('history-next').disabled,true);assert.equal(dom.get('records').children.length,0);
 dom.get('rooms').value='3';await dom.get('rooms').fire('change');assert.equal(dom.get('delete').disabled,true);
 await dom.click('logout');assert.equal(dom.get('rooms').disabled,true);assert.equal(dom.get('rooms').children.length,1);assert.equal(dom.get('records').children.length,0);assert.equal(dom.get('history-next').disabled,true);
});
test('historical discovery client rejects duplicate/out-of-order cursors and stale account response; page clears cancelled state',async()=>{
 const schema=JSON.parse(await readFile(new URL('../../../packages/contracts/g3-history/schema.json',import.meta.url))),browser={credentials:{sessionEpoch:scope.runEpoch,accessToken:'test',expiresAt:'2099-01-01'},info:{serviceId:'test'},generation:1,now:()=>0,uuid:()=> 'test-request',exclusive:fn=>fn()};
 let records=[run(1)],cancel=false;const transport=async(path,input)=>path.endsWith('/schema')?schema:path.endsWith('/info')?{revision:schema.revision,serviceId:'test',serviceEpoch:scope.runEpoch}:(cancel&&(browser.generation++),{major:1,revision:schema.revision,serviceId:'test',serviceEpoch:scope.runEpoch,requestId:input.requestId,runs:records,nextSequence:'3',highSequence:'3'});
 const client=new StatisticsClient(browser,transport);assert.equal((await client.discover()).runs.length,1);
 records=[run(1),run(1)];await assert.rejects(client.discover(),{code:'INVALID_MESSAGE'});records=[run(2),run(1)];await assert.rejects(client.discover(),{code:'INVALID_MESSAGE'});records=[run(1)];cancel=true;await assert.rejects(client.discover(),{code:'CANCELLED'});
 const dom=await controllerDom('../g3/index.html');dom.get('rooms').ownerDocument=dom.document;dom.get('records').ownerDocument=dom.document;
 const identity={credentials:{},async initialize(){},async login(){}};mountStatisticsPage(identity,{async discover(){return {runs:[run(1)],nextSequence:'1',highSequence:'1'};},async request(){throw {code:'CANCELLED'};}},dom.document);
 await dom.click('login');dom.get('rooms').value='1';await dom.get('rooms').fire('change');assert.equal(dom.get('rooms').children.length,1);assert.equal(dom.get('records').children.length,0);assert.equal(dom.get('run').textContent,'');
});
