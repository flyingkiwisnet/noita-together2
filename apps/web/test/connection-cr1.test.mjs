import test from 'node:test';import assert from 'node:assert/strict';import {readFile} from 'node:fs/promises';
import {HostClient} from '../candidate/client.js';
const schema=JSON.parse(await readFile(new URL('../../../packages/contracts/g2-next/schema.json',import.meta.url),'utf8'));
const recovery=JSON.parse(await readFile(new URL('../../../packages/contracts/g3-rejoin/schema.json',import.meta.url),'utf8'));
const old={accessToken:'old',refreshSecret:'refresh',accountId:'account',sessionId:'session',sessionEpoch:'epoch',kind:'browser',deviceId:'browser-id',expiresAt:'2030-01-01T00:00:00Z'};
function ready(transport){let id=0;const c=new HostClient(transport,()=>++id===1?'id':'request-'+id,()=>Date.parse('2030-01-01T00:00:01Z'));c.schema=schema;c.info={serviceId:'service',serviceEpoch:'service-epoch'};c.credentials={...old};c.device=old.deviceId;c.rejoinSchema=recovery;return c;}
const response=(req,rev=schema.revision)=>({major:1,revision:rev,serviceId:'service',serviceEpoch:'service-epoch',requestId:req.requestId});
test('CR1 browser first recovery action after sleep refreshes before review and retries exact original mutation',async()=>{
 const calls=[];let lost=true;
 const c=ready(async(path,req,token)=>{calls.push({path,req:structuredClone(req),token});if(req.refresh)return {...response(req),credentials:{...old,accessToken:'new',refreshSecret:'new-refresh',expiresAt:'2030-02-01T00:00:00Z'}};if(lost){lost=false;throw Error('committed review reply lost');}return response(req,recovery.revision);});
 await c.rejoin({roomId:'11111111-1111-1111-1111-111111111111',runId:'22222222-2222-2222-2222-222222222222',participantId:'33333333-3333-3333-3333-333333333333',worldToken:'synthetic-world',configRevision:1,generation:1,review:{participantId:'33333333-3333-3333-3333-333333333333',observationRevision:1,contentDigest:'a'.repeat(64)}});
 assert.equal(calls[0].path,'/v1/rpc');assert.ok(calls[0].req.refresh);assert.equal(calls[1].token,'new');assert.deepEqual(calls[1],calls[2]);
});
test('CR1 browser late recovery reply cannot update disposed session or service epoch',async()=>{
 let resolve;const c=ready(async(_path,req)=>{if(req.refresh)return {...response(req),credentials:{...old,accessToken:'new',expiresAt:'2030-02-01T00:00:00Z'}};return new Promise(r=>{resolve=()=>r({...response(req,recovery.revision),serviceEpoch:'late'});});});
 const pending=c.rejoin({roomId:'11111111-1111-1111-1111-111111111111',list:{}});while(!resolve)await new Promise(r=>setTimeout(r,0));c.dispose();resolve();await assert.rejects(pending,e=>e.code==='CANCELLED');assert.equal(c.info.serviceEpoch,'service-epoch');assert.equal(c.credentials,null);
});
