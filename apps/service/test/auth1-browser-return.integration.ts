// Real headless browser navigation/form headers; synthetic providers and an
// owned loopback receiver. Never installs trust or opens the owner's browser.
import test from 'node:test';import assert from 'node:assert/strict';import {createServer} from 'node:http';import {once} from 'node:events';import {execFile} from 'node:child_process';import {promisify} from 'node:util';import {randomBytes,randomUUID} from 'node:crypto';import {join} from 'node:path';import {startAuth1Fixture} from '../scripts/auth1-fixture.js';import {hash} from '../src/g1/security.js';

test('Real Chrome native form return retains exact initiating Origin without leaking callback URL',async()=>{
 const browser=process.env.NT2_AUTH1_TEST_BROWSER??'C:/Program Files/Google/Chrome/Application/chrome.exe',f=await startAuth1Fixture();
 let observed:{originMatches:boolean;originNull:boolean;referrerOriginOnly:boolean;bindingMatches:boolean}|undefined,grant:string|undefined,attemptId='';
 const state=randomBytes(32).toString('base64url'),proof=randomBytes(32).toString('base64url'),deviceId=randomUUID(),appInstanceId=randomUUID();
 const receiver=createServer(async(req,res)=>{if(req.method!=='POST'||req.url!=='/nt2-return'){res.writeHead(404);res.end();return;}let body='';for await(const chunk of req){body+=chunk;if(body.length>8192){res.writeHead(413);res.end();return;}}const form=new URLSearchParams(body);observed={originMatches:req.headers.origin===f.origin,originNull:req.headers.origin==='null',referrerOriginOnly:req.headers.referer===f.origin+'/',bindingMatches:req.method==='POST'&&req.url==='/nt2-return'&&form.get('callbackState')===state&&form.get('attemptId')===attemptId&&!!form.get('completionGrant')};if(observed.originMatches&&observed.bindingMatches)grant=form.get('completionGrant')!;res.writeHead(grant?200:400,{'content-type':'text/plain','cache-control':'no-store'});res.end(grant?'Return accepted':'Return refused');});
 const rpc=async(payload:unknown)=>JSON.parse((await f.service.app.inject({method:'POST',url:'/v1/rpc',headers:{host:new URL(f.origin).host},payload:{major:1,revision:'g2-draft-5',serviceId:f.service.db.serviceId,serviceEpoch:f.service.db.epoch,requestId:randomUUID(),...payload as object}})).body);
 try{
  receiver.listen(0,'127.0.0.1');await once(receiver,'listening');const callbackPort=(receiver.address() as {port:number}).port;
  const begin=await rpc({authBegin:{provider:'twitch',deviceId,appInstanceId,proofChallenge:hash(proof),callbackPort,callbackState:state}});assert.ok(begin.authAttempt);attemptId=begin.authAttempt.attemptId;
  // Test-only certificate bypass is confined to this isolated process/profile;
  // browser generates Origin and Referer itself. No headers are injected.
  try{await promisify(execFile)(browser,['--headless=new','--no-first-run','--no-default-browser-check','--disable-background-networking','--ignore-certificate-errors','--user-data-dir='+join(f.directory,'browser-profile'),'--dump-dom','--timeout=15000','--virtual-time-budget=5000',begin.authAttempt.launchUrl],{windowsHide:true,timeout:25000,maxBuffer:1024*1024});}catch{throw Error('Disposable headless browser did not complete; browser output suppressed');}
  assert.ok(observed,'Native form did not reach loopback receiver');console.log(JSON.stringify({nativeBrowserForm:observed,headersManuallyInjected:false,providersSynthetic:true,actualDesktop:false}));
  assert.equal(observed.originMatches,true,'Browser must send exact initiating service Origin');assert.equal(observed.referrerOriginOnly,true,'Callback URL path/query must not leak');assert.equal(observed.bindingMatches,true);assert.ok(grant);
  const exchange={attemptId,deviceId,appInstanceId,initiatorProof:proof,completionGrant:grant};assert.equal((await rpc({authExchange:{...exchange,initiatorProof:randomBytes(32).toString('base64url')}})).error.code,'UNAUTHENTICATED');assert.ok((await rpc({authExchange:exchange})).credentials);assert.equal((await rpc({authExchange:exchange})).error.code,'ALREADY_EXCHANGED');
 }finally{receiver.closeAllConnections();await new Promise<void>(resolve=>receiver.close(()=>resolve()));await f.close();}
});
