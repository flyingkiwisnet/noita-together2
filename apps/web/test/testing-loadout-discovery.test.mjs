import test from 'node:test';import assert from 'node:assert/strict';import fs from 'node:fs';
import {HostClient,ServiceError} from '../candidate/client.js';import {createHostController} from '../candidate/app.js';import {controllerDom} from './dom-fixture.mjs';
const info=JSON.parse(fs.readFileSync(new URL('../../../packages/test-kit/fixtures/web-service-info.json',import.meta.url),'utf8'));
const schema=JSON.parse(fs.readFileSync(new URL('../../../packages/contracts/g2-next/schema.json',import.meta.url),'utf8'));
const clientFor=value=>new HostClient(async path=>path==='/schema.json'?schema:structuredClone(value),()=> 'fixture-request');
test('Browser production discovery accepts captured actual Testing host Info and legacy absence, retaining strict negative checks',async()=>{
 const client=clientFor(info);assert.deepEqual(await client.initialize(),info);assert.equal(client.schema.revision,schema.revision);
 const legacy=structuredClone(info);delete legacy.developmentFeatures;assert.ok(await clientFor(legacy).initialize());
 for(let fault=0;fault<12;fault++){const bad=structuredClone(info),f=bad.developmentFeatures.testingStartingLoadout;switch(fault){case 0:bad.developmentFeatures.unknownFeature={};break;case 1:f.extra=true;break;case 2:f.available=false;break;case 3:f.default=true;break;case 4:f.default='false';break;case 5:f.settingKey='other';break;case 6:bad.compatibility.settingKeys=bad.compatibility.settingKeys.filter(k=>k!=='NT_testing_mode');break;case 7:bad.compatibility.settingKeys.push('NT_testing_mode');break;case 8:bad.unexpectedInfoField=true;break;case 9:bad.roomSemantics.requiredCapabilities.pop();break;case 10:bad.major=2;break;default:bad.developmentFeatures=null;}
  await assert.rejects(clientFor(bad).initialize(),e=>e instanceof ServiceError);}
});
test('Failed browser initialization clears metadata and refuses login before sending an authentication request',async()=>{
 const calls=[],bad=structuredClone(info);bad.developmentFeatures.testingStartingLoadout.default=true;const c=new HostClient(async path=>{calls.push(path);return path==='/schema.json'?schema:bad;},()=> 'request');
 await assert.rejects(c.initialize(),e=>e.code==='INVALID_MESSAGE');assert.equal(c.schema,null);assert.equal(c.info,null);await assert.rejects(c.login('website-local-alex'),e=>e.code==='INITIALIZATION_REQUIRED');assert.deepEqual(calls.sort(),['/schema.json','/v1/info']);
});
test('Production controller disables sign-in after initialization failure and never reports a generic timeout',async()=>{
 const dom=await controllerDom('../candidate/index.html');let logins=0;const client={generation:1,credentials:null,info:null,schema:null,async initialize(){throw new ServiceError('INVALID_MESSAGE');},async login(){logins++;},dispose(){}};
 const controller=createHostController({...dom,client,schedule:()=>{}});try{await controller.ready;assert.match(dom.get('notice').textContent,/initialization failed/i);assert.equal(dom.get('login').elements.subject.disabled,true);await dom.submit('login',{subject:'website-local-alex'});assert.equal(logins,0);assert.match(dom.get('notice').textContent,/initialization/i);assert.doesNotMatch(dom.get('notice').textContent,/network timeout/i);}finally{controller.dispose();}
});
