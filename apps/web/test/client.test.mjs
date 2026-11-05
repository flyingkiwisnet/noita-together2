import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {HostClient,ServiceError,validate} from '../public/client.js';
const schema=JSON.parse(await readFile(new URL('../../../packages/contracts/g2/schema.json',import.meta.url),'utf8'));
const creds={accessToken:'old',refreshSecret:'refresh',accountId:'account',sessionId:'session',sessionEpoch:'epoch',kind:'browser',deviceId:'browser-id',expiresAt:'2030-01-01T00:00:00Z'};
function ready(transport,now=()=>0){const c=new HostClient(transport,()=> 'id',now);c.schema=schema;c.info={serviceId:'service',serviceEpoch:'service-epoch'};c.credentials={...creds};return c;}
const response=req=>({major:1,revision:schema.revision,serviceId:'service',serviceEpoch:'service-epoch',requestId:req.requestId});
test('lost committed host operation retries identical request rather than creating another operation',async()=>{const calls=[];const c=ready(async(_path,req)=>{calls.push(structuredClone(req));if(calls.length===1)throw Error('lost');return response(req);});await c.rpc({roomCreate:{name:'<img onerror=bad>',config:{seed:4,mode:'core'}}});assert.equal(calls.length,2);assert.deepEqual(calls[0],calls[1]);});
test('renew before host operation preserves session and never uses revoked access',async()=>{const tokens=[];const c=ready(async(_path,req,token)=>{tokens.push(token);return req.refresh?{...response(req),credentials:{...creds,accessToken:'new'}}:response(req);},()=>Date.parse('2029-12-31T23:59:59Z'));await c.rpc({roomList:{}});assert.deepEqual(tokens,[undefined,'new']);assert.equal(c.credentials.sessionId,'session');});
test('renewed credentials from another identity cannot be adopted',async()=>{const c=ready(async(_path,req)=>({...response(req),credentials:{...creds,accountId:'other'}}),()=>Date.parse('2029-12-31T23:59:59Z'));await assert.rejects(c.rpc({roomList:{}}),e=>e.code==='STALE_EPOCH');assert.equal(c.credentials,null);});
test('bounded single active operation and disposal fence late responses',async()=>{let resolve;const c=ready(()=>new Promise(r=>resolve=r));const pending=c.rpc({roomGet:{}});await assert.rejects(c.rpc({roomList:{}}),e=>e instanceof ServiceError&&e.code==='CAPACITY');c.generation++;resolve(response({requestId:'id'}));await assert.rejects(pending,e=>e.code==='CANCELLED');assert.equal(c.active,false);});
test('response identity and UTF8/message bounds reject hostile transport',async()=>{const c=ready(async(_path,req)=>({...response(req),serviceId:'other'}));await assert.rejects(c.rpc({roomList:{}}),e=>e.code==='INVALID_MESSAGE');assert.throws(()=>validate(schema,'ChatSend',{text:'🔥'.repeat(257)}));assert.throws(()=>validate(schema,'ChatSend',{text:'hello',accountId:'forged'}));assert.throws(()=>validate(schema,'Request',{chatSend:{text:'hello'},logout:{}}));});
test('logout clears credentials even when remote confirmation is lost',async()=>{const c=ready(async()=>{throw Error('offline');});await assert.rejects(c.logout());assert.equal(c.credentials,null);});

function recovery(transport){let time=Date.parse('2029-12-31T23:59:59Z'),nonce=0;const c=new HostClient(transport,()=>nonce++?'request-'+nonce:'id',()=>time);c.schema=schema;c.info={serviceId:'service',serviceEpoch:'service-epoch',profile:{id:'profile'}};c.credentials={...creds};return {c,advance:ms=>time+=ms};}
test('two committed refresh reply losses recover the original operation across later rpc and renew calls',async()=>{
 const requests=[];let drops=2;const {c,advance}=recovery(async(_path,req)=>{if(req.refresh){requests.push(structuredClone(req));if(drops-->0)throw Error('committed reply lost');return {...response(req),credentials:{...creds,accessToken:'new',refreshSecret:'new-refresh',expiresAt:'2030-02-01T00:00:00Z'}};}return response(req);});
 await assert.rejects(c.rpc({roomList:{}}));const pending=c.pendingRefresh;assert.ok(Object.isFrozen(pending.request));assert.ok(Object.isFrozen(pending.request.refresh));advance(5000);await c.rpc({roomList:{}});
 assert.equal(requests.length,3);assert.deepEqual(requests[0],requests[1]);assert.deepEqual(requests[1],requests[2]);assert.equal(c.credentials.accessToken,'new');assert.equal(c.pendingRefresh,null);assert.equal(pending.deadline,Date.parse('2029-12-31T23:59:59Z')+30000);
 // Direct renew uses the same serialized recovery path, even when access is not near expiry.
 drops=2;await assert.rejects(c.renew());const second=c.pendingRefresh;advance(1000);await c.renew();assert.equal(requests.at(-1).requestId,second.request.requestId);
});
test('pending refresh expiry never submits a fresh ID with the spent secret',async()=>{
 let count=0;const {c,advance}=recovery(async()=>{count++;throw Error('lost');});await assert.rejects(c.renew());advance(30000);await assert.rejects(c.rpc({roomList:{}}),e=>e.code==='EXPIRED');assert.equal(count,2);assert.equal(c.credentials,null);assert.equal(c.pendingRefresh,null);await assert.rejects(c.renew(),e=>e.code==='UNAUTHENTICATED');assert.equal(count,2);
});
test('logout discards a pending refresh and later calls cannot resurrect it',async()=>{
 const commands=[];const {c}=recovery(async(_path,req)=>{commands.push(req);if(req.refresh)throw Error('lost');return response(req);});await assert.rejects(c.renew());await c.logout();assert.equal(c.pendingRefresh,null);assert.equal(c.credentials,null);assert.equal(commands.filter(r=>r.refresh).length,2);await assert.rejects(c.rpc({roomList:{}}),e=>e.code==='UNAUTHENTICATED');
});
test('explicit sign-in replaces pending recovery without reusing the previous identity',async()=>{
 const commands=[];const {c}=recovery(async(path,req)=>{commands.push({path,req});if(path==='/v1/mock/browser-login')return {credentials:{...creds,accountId:'other',sessionId:'other-session',accessToken:'other-access',refreshSecret:'other-refresh',expiresAt:'2030-02-01T00:00:00Z'}};if(req.refresh)throw Error('lost');return response(req);});await assert.rejects(c.renew());const generation=c.generation;await c.login('other');await c.rpc({roomList:{}});assert.ok(c.generation>generation);assert.equal(c.pendingRefresh,null);assert.equal(c.credentials.accountId,'other');assert.equal(commands.filter(r=>r.req.refresh).length,2);
});
test('pending refresh is fenced to browser identity and service/profile/protocol/transport',async()=>{
 for(const change of [c=>c.credentials={...c.credentials,accountId:'other'},c=>c.info={...c.info,serviceId:'other'},c=>c.info={...c.info,serviceEpoch:'other'},c=>c.info={...c.info,profile:{id:'other'}},c=>c.schema={...c.schema,revision:'other'},c=>c.transport=async()=>{throw Error('wrong destination');}]){
  let requests=0;const {c}=recovery(async()=>{requests++;throw Error('lost');});await assert.rejects(c.renew());change(c);await assert.rejects(c.renew(),e=>e.code==='STALE_EPOCH');assert.equal(requests,2);assert.equal(c.pendingRefresh,null);
 }
});
test('late refresh success after disposal or deadline cannot adopt credentials',async()=>{
 for(const expired of [false,true]){let resolve;const {c,advance}=recovery((_path,req)=>new Promise(r=>resolve=()=>r({...response(req),credentials:{...creds,accessToken:'late'}})));const pending=c.renew();if(expired)advance(30000);else c.dispose();resolve();await assert.rejects(pending,e=>e.code===(expired?'EXPIRED':'CANCELLED'));assert.equal(c.credentials,null);assert.equal(c.pendingRefresh,null);}
});
