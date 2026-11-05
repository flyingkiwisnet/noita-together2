import test from 'node:test';import assert from 'node:assert/strict';import {randomUUID} from 'node:crypto';
import {startG3Fixture} from '../scripts/g3-fixture.js';
// @ts-expect-error Production browser controller has no Node-only type layer.
import {createHostController} from '../../web/candidate/app.js';
// @ts-expect-error Exercise the production browser client over the real service.
import {HostClient,ServiceError} from '../../web/candidate/client.js';
// @ts-expect-error Test DOM adapter; real service remains the sole authority.
import {controllerDom} from '../../web/test/dom-fixture.mjs';

test('actual website Create/Configure preserves all four semantic defaults for the new C2 artifact ID',async()=>{
 const f=await startG3Fixture({c2Runtime:true,live:true,tls:true});let controller:any;
 try{
  const calls:any[]=[],transport=async(path:string,body?:any,token?:string)=>{if(body?.roomCreate||body?.configure)calls.push(structuredClone(body));const r=await f.call(path,body,token);assert.equal(r.status,200);const value=JSON.parse(r.body);if(value.error)throw new ServiceError(value.error.code);return value;};
  const client=new HostClient(transport,randomUUID),dom=await controllerDom('../candidate/index.html');controller=createHostController({...dom,client,schedule:()=>{}});await controller.ready;assert.ok(client.schema,dom.get('notice').textContent);await dom.click('mock-website');assert.ok(client.credentials,dom.get('notice').textContent);
  await dom.submit('create',{name:'Website native social',seed:'424242',mode:'core',visibility:'public'});assert.equal(dom.get('notice').dataset.error,'false');const roomId=dom.get('invite').value;assert.ok(roomId);
  const created=(await client.rpc({roomId,roomGet:{}})).room;assert.deepEqual(created.config.requiredCapabilities,f.profile.compatibility.capabilities);assert.deepEqual(calls[0].roomCreate.config.requiredCapabilities,f.profile.compatibility.capabilities);
  await dom.submit('configure',{seed:'1234',mode:'core'});assert.equal(dom.get('notice').dataset.error,'false');assert.deepEqual(calls[1].configure.requiredCapabilities,f.profile.compatibility.capabilities);
  const configured=(await client.rpc({roomId,roomGet:{}})).room;assert.equal(configured.config.seed,1234);assert.deepEqual(configured.config.requiredCapabilities,f.profile.compatibility.capabilities);
  await dom.click('prepare');assert.equal(dom.get('notice').dataset.error,'false');const prepared=(await client.rpc({roomId,roomGet:{}})).room;assert.equal(prepared.state,'staging');assert.deepEqual(prepared.config.requiredCapabilities,f.profile.compatibility.capabilities);
 }finally{controller?.dispose();await f.close();}
});
