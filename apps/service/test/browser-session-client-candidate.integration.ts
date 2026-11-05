import test from 'node:test';import assert from 'node:assert/strict';import {randomUUID} from 'node:crypto';
import {startG3Fixture} from '../scripts/g3-loot01-fixture.js';import {installBrowserSessionCandidate} from './candidates/browser-session-policy.js';
const {BrowserSessionCandidate,browserSignInCapacityMessage}=await import(new URL('../../web/test/candidates/browser-session-client.js',import.meta.url).href);
const {HostClient}=await import(new URL('../../web/test/candidates/browser-session-baseline-client.js',import.meta.url).href);
type Fixture=Awaited<ReturnType<typeof startG3Fixture>>;
async function run(label:string,fn:(f:Fixture)=>Promise<void>){const f=await startG3Fixture({databaseUrl:'postgres://nt2_g1:local-fixture-only@127.0.0.1:54328/nt2_g1',rejoin:true,loot01Runtime:true,live:true,tls:true,configure:installBrowserSessionCandidate});try{await fn(f);console.log(JSON.stringify({browserClientCandidate:label,schema:f.namespace,passed:true,real:'TLS/auth/logout/SQL',synthetic:'pagehide/reload/transport delivery'}));}finally{await f.close();}}
const transport=(f:Fixture)=>async(path:string,body?:unknown,token?:string)=>JSON.parse((await f.call(path,body,token)).body);
const count=(f:Fixture,account:string)=>f.service.db.transaction(async tx=>(await tx.query("SELECT count(*)::int AS count FROM session_families WHERE account_id=$1 AND kind='browser' AND revoked_at IS NULL",[account])).rows[0].count);

test('actual shared HostClient repeated same-page login and page disposal reproduce eight-slot accumulation',()=>run('baseline-shared-web-client',async f=>{
 const send=transport(f),same=new HostClient(send);await same.initialize();let account='';for(let i=0;i<4;i++){const result=await same.login('twitch-local-alex');account=result.accountId;}assert.equal(await count(f,account),4);same.dispose();assert.equal(await count(f,account),4);
 for(let i=0;i<4;i++){const page=new HostClient(send);await page.initialize();await page.login('twitch-local-alex');page.dispose();}assert.equal(await count(f,account),8);const blocked=new HostClient(send);await blocked.initialize();await assert.rejects(blocked.login('twitch-local-alex'),/CAPACITY/);
}));

test('candidate orderly pagehide/reload and repeated same-page login do not consume browser slots',()=>run('reload-and-repeat',async f=>{
 let account='';for(let i=0;i<12;i++){let keepalive=false;const send=transport(f),c=new BrowserSessionCandidate(async(path:string,b:unknown,t:string,options:any)=>{if(options?.keepalive)keepalive=true;return send(path,b,t);});await c.initialize();const a=await c.login('twitch-local-alex');account=a.accountId;for(let n=0;n<3;n++)assert.equal((await c.login('twitch-local-alex')).sessionId,a.sessionId);assert.equal(await count(f,account),1);assert.equal((await c.closePage()).confirmed,true);assert.equal(keepalive,true);assert.equal(c.credentials,null);assert.equal(await count(f,account),0);}
 assert.match(browserSignInCapacityMessage,/Browser sign-in/);assert.match(browserSignInCapacityMessage,/Sign out/);assert.ok(!browserSignInCapacityMessage.includes('Room or service'));
}));

test('candidate uncertain login retains one original request and recovers actual committed lost reply',()=>run('pending-login-original',async f=>{
 const send=transport(f),ids:string[]=[];let drop=true;const c=new BrowserSessionCandidate(async(path:string,b:any,t:string)=>{const v=await send(path,b,t);if(path==='/v1/mock/browser-login'){ids.push(b.requestId);if(drop){drop=false;throw Error('Injected reply loss after actual commit');}}return v;});await c.initialize();await assert.rejects(c.login('twitch-local-alex'),/Injected reply loss/);assert.ok(c.pendingLogin);assert.equal(c.credentials,null);const recovered=await c.login('twitch-local-alex');assert.equal(ids.length,2);assert.equal(ids[0],ids[1]);assert.equal(await count(f,recovered.accountId),1);await c.closePage();
}));

test('late successful login after pagehide is revoked rather than adopted; explicit account switch preserves independent tab',()=>run('late-response-and-account-boundary',async f=>{
 const send=transport(f);let release!:()=>void,committed!:()=>void;const ready=new Promise<void>(r=>committed=r),held=new Promise<void>(r=>release=r);
 const c=new BrowserSessionCandidate(async(path:string,b:any,t:string)=>{const v=await send(path,b,t);if(path==='/v1/mock/browser-login'){committed();await held;}return v;});await c.initialize();const pending=c.login('twitch-local-alex');await ready;await c.closePage();release();await assert.rejects(pending,/CANCELLED/);assert.equal(c.credentials,null);
 const a=new BrowserSessionCandidate(send),b=new BrowserSessionCandidate(send);await a.initialize();await b.initialize();const first=await a.login('twitch-local-alex'),other=await b.login('website-local-alex');assert.equal(await count(f,first.accountId),1);assert.equal(await count(f,other.accountId),1);await assert.rejects(a.login('website-local-alex'),/INVALID_STATE/);assert.equal(await a.revokeBrowser(other),false);assert.equal((await b.rpc({roomList:{limit:1}})).error,undefined);await a.closePage();assert.equal(await count(f,other.accountId),1);await b.closePage();
}));

test('bfcache restore reopens sign-in and polling; late old logout cannot revoke restored session',()=>run('bfcache-signin-and-poll',async f=>{
 const send=transport(f);let release!:()=>void,hidden!:()=>void;const blocked=new Promise<void>(r=>release=r),started=new Promise<void>(r=>hidden=r);let hold=false;
 const c=new BrowserSessionCandidate(async(path:string,b:any,t:string)=>{if(hold&&b?.logout){hidden();await blocked;}return send(path,b,t);});await c.initialize();const old=await c.login('twitch-local-alex');hold=true;const exit=c.closePage();await started;
 c.restorePage();await c.initialize();const restored=await c.login('twitch-local-alex');assert.notEqual(restored.sessionId,old.sessionId);release();await exit;hold=false;
 assert.equal((await c.rpc({roomList:{limit:1}})).error,undefined);assert.equal(c.credentials.sessionId,restored.sessionId);assert.equal(c.pageClosed,false);assert.equal(await count(f,restored.accountId),1);await c.closePage();
}));
