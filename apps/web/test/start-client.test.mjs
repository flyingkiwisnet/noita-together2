import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {HostClient,ServiceError} from '../candidate/client.js';
const schema=JSON.parse(await readFile(new URL('../../../packages/contracts/g2-next/schema.json',import.meta.url)));
function client(transport){const c=new HostClient(transport,()=> 'original-id',()=>0);c.schema=schema;c.info={serviceId:'service',serviceEpoch:'epoch'};c.credentials={accountId:'host',deviceId:'browser',sessionId:'browser',sessionEpoch:'epoch',accessToken:'token',expiresAt:'2030-01-01T00:00:00Z'};return c;}
const response=req=>({major:1,revision:schema.revision,serviceId:'service',serviceEpoch:'epoch',requestId:req.requestId});
test('only a correlated finalStart rejection is marked as a verified Start refusal',async()=>{
 for(const mode of ['verified','wrong-request','wrong-service','transport-code']){
  const c=client(async(_path,req)=>{if(mode==='transport-code')throw new ServiceError('STALE_REVISION');return {...response(req),...(mode==='wrong-request'?{requestId:'other'}:mode==='wrong-service'?{serviceId:'other'}:{}),error:{code:'STALE_REVISION'}};});
  await assert.rejects(c.rpc({finalStart:{participants:[],excludedDesktopIds:[]}}),e=>{assert.equal(e.verifiedResponse===true,mode==='verified');if(mode==='verified')assert.equal(e.operation,'finalStart');return true;});
 }
 const c=client(async(_path,req)=>({...response(req),error:{code:'STALE_REVISION'}}));await assert.rejects(c.rpc({roomGet:{}}),e=>e.verifiedResponse===true&&e.operation===null);
});
test('lost Start reply transport retry uses the exact original request and request ID',async()=>{
 const originals=[];const c=client(async(_path,req)=>{originals.push(structuredClone(req));if(originals.length===1)throw Error('reply lost after commit');return response(req);});
 await c.rpc({finalStart:{participants:[],excludedDesktopIds:['explicitly-excluded']}},{roomId:'room',runId:'run',runEpoch:'run-epoch',configRevision:1,revision:5});
 assert.equal(originals.length,2);assert.deepEqual(originals[0],originals[1]);assert.equal(originals[0].requestId,'original-id');
});
