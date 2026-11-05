import test from 'node:test';import assert from 'node:assert/strict';import {fetchTransport} from '../candidate/client.js';
test('production fetchTransport forwards bounded logout keepalive with bearer header and no browser credential storage/cookies',async()=>{
 const original=globalThis.fetch;let sent;
 globalThis.fetch=async(path,options)=>{sent={path,options};return new Response('{}',{status:200,headers:{'content-type':'application/json'}});};
 try{await fetchTransport('/v1/rpc',{logout:{}},'test-ephemeral-token',{keepalive:true});assert.equal(sent.path,'/v1/rpc');assert.equal(sent.options.keepalive,true);assert.equal(sent.options.credentials,'omit');assert.equal(sent.options.headers.authorization,'Bearer test-ephemeral-token');assert.equal(sent.options.redirect,'error');assert.ok(sent.options.signal);assert.deepEqual(JSON.parse(sent.options.body),{logout:{}});}finally{globalThis.fetch=original;}
});
