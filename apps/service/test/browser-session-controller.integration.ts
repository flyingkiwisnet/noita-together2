import test from 'node:test';import assert from 'node:assert/strict';import {randomUUID} from 'node:crypto';
import {startG3Fixture} from '../scripts/g3-loot01-fixture.js';
const {HostClient,browserSignInCapacityMessage}=await import(new URL('../../web/candidate/client.js',import.meta.url).href);
const {createHostController}=await import(new URL('../../web/candidate/app.js',import.meta.url).href);
const {controllerDom}=await import(new URL('../../web/test/dom-fixture.mjs',import.meta.url).href);
type Fixture=Awaited<ReturnType<typeof startG3Fixture>>;
async function run(label:string,fn:(f:Fixture)=>Promise<void>){const f=await startG3Fixture({databaseUrl:'postgres://nt2_g1:local-fixture-only@127.0.0.1:54328/nt2_g1',rejoin:true,loot01Runtime:true,live:true,tls:true});try{await fn(f);console.log(JSON.stringify({browserControllerProduction:label,schema:f.namespace,passed:true,real:'production controller/client/auth/TLS/SQL',synthetic:'DOM/pagehide/pageshow/timers'}));}finally{await f.close();}}

test('production controller pagehide logs out and bfcache pageshow restores sign-in with one current polling chain',()=>run('pagehide-pageshow-real-wiring',async f=>{
 const dom=await controllerDom('../candidate/index.html'),scheduled:Array<()=>Promise<unknown>>=[],calls:any[]=[];let roomId='',logoutDone!:()=>void;const logout=new Promise<void>(r=>logoutDone=r);
 const client=new HostClient(async(path:string,b:any,t:string,options:any)=>{calls.push({path,operation:b?Object.keys(b).find(k=>['roomGet','roomList','chatList','logout'].includes(k)):undefined,keepalive:options?.keepalive===true});const v=JSON.parse((await f.call(path,b,t)).body);if(b?.roomCreate)roomId=v.room.roomId;if(b?.logout)logoutDone();return v;});
 const controller=createHostController({...dom,client,schedule:(fn:()=>Promise<unknown>)=>scheduled.push(fn)});
 try{
  await controller.ready;assert.equal(scheduled.length,1);await dom.click('mock-twitch');assert.ok(client.credentials);await dom.submit('create',{name:'Browser restored room',seedPolicy:'shared-fixed',seed:'424242',mode:'core'});assert.ok(roomId);
  const oldPoll=scheduled.shift()!;await dom.window.fire('pagehide');await logout;assert.equal(client.credentials,null);assert.ok(calls.some(c=>c.operation==='logout'&&c.keepalive));
  for(const handler of dom.window.handlers.get('pageshow'))await handler({persisted:true});assert.equal(client.pageClosed,false);assert.equal(dom.get('login').hidden,false);assert.equal(scheduled.length,1);
  const before=calls.length;await oldPoll();assert.equal(calls.length,before);assert.equal(scheduled.length,1);
  await dom.click('mock-twitch');assert.ok(client.credentials);await dom.submit('preview',{roomId});const beforePoll=calls.filter(c=>c.operation==='roomGet').length;await scheduled.shift()!();assert.ok(calls.filter(c=>c.operation==='roomGet').length>beforePoll);assert.equal(scheduled.length,1);
  await scheduled.shift()!();assert.equal(scheduled.length,1);
 }finally{controller.dispose();await client.closePage();}
}));

test('production controller sign-in CAPACITY text distinguishes browser sessions from room/player capacity',()=>run('sign-in-specific-capacity-message',async f=>{
 for(let i=0;i<8;i++){const result=JSON.parse((await f.call('/v1/mock/browser-login',{subject:'twitch-local-alex',deviceId:'other-tab-'+randomUUID(),requestId:randomUUID()})).body);assert.equal(result.error,undefined);}
 const dom=await controllerDom('../candidate/index.html'),client=new HostClient(async(path:string,b:any,t:string)=>JSON.parse((await f.call(path,b,t)).body));const controller=createHostController({...dom,client,schedule:()=>{}});
 try{await controller.ready;await dom.click('mock-twitch');assert.equal(dom.get('notice').textContent,browserSignInCapacityMessage);assert.equal(dom.get('notice').dataset.error,'true');assert.equal(client.credentials,null);}finally{controller.dispose();}
}));

test('successful login followed by discovery CAPACITY retains ordinary discovery error and current browser session',()=>run('discovery-capacity-is-not-login-capacity',async f=>{
 const dom=await controllerDom('../candidate/index.html');let discovered=false;
 const client=new HostClient(async(path:string,b:any,t:string)=>{if(b?.roomList){discovered=true;return {major:1,revision:'g2-draft-5',serviceId:f.service.db.serviceId,serviceEpoch:f.service.db.epoch,requestId:b.requestId,error:{code:'CAPACITY'}};}return JSON.parse((await f.call(path,b,t)).body);});
 const controller=createHostController({...dom,client,schedule:()=>{}});
 try{await controller.ready;await dom.click('mock-twitch');assert.equal(discovered,true);assert.ok(client.credentials);assert.equal(dom.get('notice').textContent,'Room or service capacity reached. Wait before retrying.');assert.notEqual(dom.get('notice').textContent,browserSignInCapacityMessage);}finally{await client.closePage();controller.dispose();}
}));
