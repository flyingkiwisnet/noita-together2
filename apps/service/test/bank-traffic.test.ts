import test from 'node:test';import assert from 'node:assert/strict';import {generateKeyPairSync,randomUUID} from 'node:crypto';import Fastify from 'fastify';
import {installTrafficBudgets,rememberTrafficBody} from '../src/g2-next/traffic.js';import {Security} from '../src/g1/security.js';import {encode,decode,schema} from '../../../packages/contracts/src/g3.js';
test('JSON and decoded protobuf batches charge the existing reliable budget per original',async()=>{
 let now=Date.now();const pair=generateKeyPairSync('ed25519'),security=new Security({privateKey:pair.privateKey.export({type:'pkcs8',format:'pem'}).toString(),publicKey:pair.publicKey.export({type:'spki',format:'pem'}).toString(),recoveryKey:Buffer.alloc(32,1).toString('base64url'),issuer:'https://bank.fixture'},'bank-fixture');
 const token=await security.sign(randomUUID(),randomUUID(),randomUUID(),'game',randomUUID(),now),app=Fastify();app.addContentTypeParser('application/x-protobuf',{parseAs:'buffer'},(_req,body,done)=>done(null,body));installTrafficBudgets(app,security,()=>now);
 app.post('/v3/rpc',{preValidation:async req=>{if(Buffer.isBuffer(req.body))rememberTrafficBody(req,decode('Request',req.body));}},async()=>({ok:true}));
 const request={major:1,revision:schema.revision,bankBatch:{entries:Array.from({length:4},()=>({itemStatus:{operationId:randomUUID()}}))}};
 try{for(const binary of [false,true]){now+=1000;for(let i=0;i<4;i++){const response=await app.inject({method:'POST',url:'/v3/rpc',headers:{authorization:'Bearer '+token,'content-type':binary?'application/x-protobuf':'application/json'},payload:binary?Buffer.from(encode('Request',request)):request});assert.equal(response.statusCode,i<3?200:429);}}}finally{await app.close();}
});
