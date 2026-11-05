import test from 'node:test';import assert from 'node:assert/strict';import fs from 'node:fs';
import {CookieHostClient,ServiceError} from '../candidate/client.js';import {createHostController} from '../candidate/app.js';import {controllerDom} from './dom-fixture.mjs';
const schema=JSON.parse(fs.readFileSync(new URL('../../../packages/contracts/g2-next/schema.json',import.meta.url))),info=JSON.parse(fs.readFileSync(new URL('../../../packages/test-kit/fixtures/web-service-info.json',import.meta.url))),fixtures=JSON.parse(fs.readFileSync(new URL('../../../packages/contracts/g3-auth1/fixture-inputs.json',import.meta.url)));
function fixture({discoveryMissing=false,mockProvider=info.mockProvider}={}){let account=structuredClone(fixtures[1].json.account),held,heldRooms;const calls=[],discovery={...fixtures[0].json,serviceId:info.serviceId,serviceEpoch:info.serviceEpoch},session=()=>({authenticated:true,csrf:'fixture-csrf-not-a-credential',account,session:{accountId:account.accountId,sessionId:'browser-public-session',sessionEpoch:'browser-public-epoch',deviceId:'web-fixture',kind:'browser',expiresAt:'2099-01-01T00:00:00.000Z'}});
 const client=new CookieHostClient(async(p,b,t,o)=>{calls.push({p,b,t,o});if(p==='/schema.json')return schema;if(p==='/v1/info')return {...structuredClone(info),mockProvider};if(p==='/v1/auth/info'){if(discoveryMissing)throw new ServiceError('NOT_FOUND');return discovery;}if(p==='/v1/web/session'){if(held instanceof Error)throw held;return held??session();}if(p==='/v1/web/logout')return {signedOut:true};if(p==='/v1/rpc'&&b.roomList)return {major:1,revision:schema.revision,serviceId:info.serviceId,serviceEpoch:info.serviceEpoch,requestId:b.requestId,...(await (heldRooms??{rooms:[]}))};throw Error('Unexpected fixture route '+p);});return {client,calls,get account(){return account;},set account(v){account=v;},hold(p){held=p;},holdRooms(p){heldRooms=p;},session};}
test('Production cookie website client exposes public identity only, sends CSRF and preserves cookie session on page close',async()=>{
 const f=fixture();await f.client.initialize();assert.equal(f.client.credentials.accountId,f.account.accountId);assert.equal(f.client.credentials.accessToken,undefined);assert.equal(f.client.credentials.refreshSecret,undefined);await assert.rejects(f.client.login('website-local-alex'),e=>e.code==='FORBIDDEN');await f.client.closePage();assert.equal(f.client.account,null);assert.equal(f.client.credentials,null);assert.equal(f.calls.some(c=>c.p==='/v1/web/logout'),false);f.client.restorePage();await f.client.initialize();await f.client.logout();const logout=f.calls.find(c=>c.p==='/v1/web/logout');assert.equal(logout.o.credentials,'same-origin');assert.equal(logout.o.csrf,'fixture-csrf-not-a-credential');assert.equal(logout.t,undefined);
});
test('Late production cookie account read cannot restore credentials or privileges after page close',async()=>{
 const f=fixture();await f.client.initialize();let done;f.hold(new Promise(r=>done=r));const pending=f.client.refreshAccount();await f.client.closePage();done(f.session());await assert.rejects(pending,e=>e.code==='CANCELLED');assert.equal(f.client.credentials,null);assert.equal(f.client.account,null);
});
test('Current host UI consumes role promotions/demotions without password/token exposure or a new connection controller',async()=>{
 const f=fixture(),dom=await controllerDom('../candidate/index.html'),scheduled=[],controller=createHostController({...dom,client:f.client,schedule:fn=>scheduled.push(fn)});try{await controller.ready;assert.equal(dom.get('mock-choices').hidden,true);assert.equal(dom.get('login').hidden,true);assert.equal(dom.get('users-link').hidden,true);assert.equal(dom.get('create').elements.name.readOnly,true);assert.equal(dom.get('create').elements.name.value,f.account.entitlements.generatedLobbyName);assert.equal(dom.get('create-testing-mode').hidden,true);
 f.account={...f.account,role:'dev',entitlements:{...f.account.entitlements,customLobbyName:true,capacityLimit:16,manageUsers:true,manageAllRoles:true,testingMode:true}};await scheduled.shift()();assert.equal(dom.get('users-link').hidden,false);assert.equal(dom.get('create').elements.name.readOnly,false);assert.equal(dom.get('create-testing-mode').hidden,false);
 f.account={...f.account,role:'user',entitlements:{...f.account.entitlements,customLobbyName:false,capacityLimit:8,manageUsers:false,manageAllRoles:false,testingMode:false}};await scheduled.shift()();assert.equal(dom.get('users-link').hidden,true);assert.equal(dom.get('create').elements.name.readOnly,true);assert.equal(dom.get('create-testing-mode').hidden,true);
 }finally{controller.dispose();}
});

const assertNoCreate=dom=>{assert.equal(dom.get('create-panel').hidden,true);assert.ok(dom.get('create').elements.every(el=>el.disabled));};
const assertNoMocks=dom=>{assert.equal(dom.get('mock-choices').hidden,true);assert.equal(dom.get('login').hidden,true);assert.equal(dom.get('mock-twitch').disabled,true);assert.equal(dom.get('mock-website').disabled,true);};
test('Initial HTML and pending authentication show neutral text with hidden disabled creation and mock controls',async()=>{
 const dom=await controllerDom('../candidate/index.html');assertNoCreate(dom);assertNoMocks(dom);
 const html=fs.readFileSync(new URL('../candidate/index.html',import.meta.url),'utf8');assert.match(html,/<p id="identity">Checking sign-in availability…<\/p>/);assert.doesNotMatch(html,/Desktop mock sign-in returns|Local development service/);
 const f=fixture();let done;f.hold(new Promise(r=>done=r));const controller=createHostController({...dom,client:f.client});try{assertNoCreate(dom);assertNoMocks(dom);assert.doesNotMatch(dom.get('auth-badge').textContent,/mock/i);done({authenticated:false});await controller.ready;assertNoCreate(dom);assertNoMocks(dom);}finally{controller.dispose();}
});
test('Real anonymous browser cannot submit room creation or invoke hidden mock login',async()=>{
 const f=fixture();f.hold({authenticated:false});const dom=await controllerDom('../candidate/index.html'),controller=createHostController({...dom,client:f.client});try{await controller.ready;assertNoCreate(dom);assertNoMocks(dom);assert.equal(dom.get('provider-choices').hidden,false);await dom.submit('create');await dom.click('mock-twitch');assert.equal(f.calls.some(c=>c.p==='/v1/rpc'||c.p==='/v1/mock/browser-login'),false);assertNoCreate(dom);}finally{controller.dispose();}
});
test('Signed-in creation is hidden and disabled immediately on logout and stays unavailable afterwards',async()=>{
 const f=fixture(),dom=await controllerDom('../candidate/index.html'),controller=createHostController({...dom,client:f.client});try{await controller.ready;assert.equal(dom.get('create-panel').hidden,false);assert.equal(dom.get('create').elements.name.disabled,false);const pending=dom.click('logout');assertNoCreate(dom);await pending;assertNoCreate(dom);assertNoMocks(dom);await dom.submit('create');assert.equal(f.calls.some(c=>c.p==='/v1/rpc'&&c.b.roomCreate),false);}finally{controller.dispose();}
});
test('Expired or failed session refresh hides creation; a successful later refresh restores it',async()=>{
 const f=fixture(),dom=await controllerDom('../candidate/index.html'),scheduled=[],controller=createHostController({...dom,client:f.client,schedule:fn=>scheduled.push(fn)});try{await controller.ready;f.hold(new ServiceError('SERVICE_UNAVAILABLE'));await scheduled.shift()();assertNoCreate(dom);assertNoMocks(dom);f.hold(null);await scheduled.shift()();assert.equal(dom.get('create-panel').hidden,false);f.hold({authenticated:false});await scheduled.shift()();assertNoCreate(dom);await dom.submit('create');assert.equal(f.calls.some(c=>c.p==='/v1/rpc'&&c.b.roomCreate),false);}finally{controller.dispose();}
});
test('Failed authentication discovery never enables creation or falls back to mocks without explicit metadata',async()=>{
 const f=fixture({discoveryMissing:true,mockProvider:false}),dom=await controllerDom('../candidate/index.html'),controller=createHostController({...dom,client:f.client});try{await controller.ready;assertNoCreate(dom);assertNoMocks(dom);assert.equal(dom.get('provider-choices').hidden,true);assert.match(dom.get('notice').textContent,/initialization failed/i);await dom.submit('create');assert.equal(f.calls.some(c=>c.p==='/v1/rpc'&&c.b.roomCreate),false);}finally{controller.dispose();}
});
test('Mock choices appear only after successful initialization with explicit mockProvider true',async()=>{
 const f=fixture({discoveryMissing:true,mockProvider:true}),dom=await controllerDom('../candidate/index.html'),controller=createHostController({...dom,client:f.client});try{assertNoMocks(dom);await controller.ready;assert.equal(dom.get('mock-choices').hidden,false);assert.equal(dom.get('login').hidden,false);assert.equal(dom.get('mock-twitch').disabled,false);assertNoCreate(dom);assert.match(dom.get('auth-badge').textContent,/mock/);}finally{controller.dispose();}
});
test('Restored signed-in cookie session discovers existing lobbies on initial load and browser page restoration',async()=>{
 const f=fixture();f.holdRooms({rooms:[{roomId:'existing-room',name:'Existing expedition',state:'waiting'}]});const dom=await controllerDom('../candidate/index.html'),controller=createHostController({...dom,client:f.client});
 try{await controller.ready;assert.match(dom.get('rooms').textContent,/Existing expedition/);assert.equal(f.calls.filter(c=>c.b?.roomList).length,1);
  await dom.window.fire('pagehide');for(const handler of dom.window.handlers.get('pageshow'))await handler({persisted:true});assert.match(dom.get('rooms').textContent,/Existing expedition/);assert.equal(f.calls.filter(c=>c.b?.roomList).length,2);
 }finally{controller.dispose();}
});
test('Successful sign-in return shows one temporary notice; restored sessions do not repeat it',async()=>{
 const f=fixture(),dom=await controllerDom('../candidate/index.html'),scheduled=[],replaced=[];
 dom.window.location={pathname:'/',search:'',hash:'#signed-in'};dom.window.history={replaceState(_state,_title,path){replaced.push(path);dom.window.location.hash='';}};
 const controller=createHostController({...dom,client:f.client,schedule:(fn,ms)=>scheduled.push({fn,ms})});try{
  await controller.ready;assert.equal(dom.get('notice').textContent,'Ready. Your account is signed in.');assert.deepEqual(replaced,['/']);
  const expiry=scheduled.find(t=>t.ms===5000);assert.ok(expiry);expiry.fn();assert.equal(dom.get('notice').textContent,'');
  await dom.window.fire('pagehide');for(const handler of dom.window.handlers.get('pageshow'))await handler({persisted:true});assert.equal(dom.get('notice').textContent,'');
 }finally{controller.dispose();}
});

test('Sign-in notice clears on navigation without clearing a newer error or notice',async()=>{
 for(const next of ['navigation','error','notice']){
  const f=fixture(),dom=await controllerDom('../candidate/index.html'),scheduled=[];dom.window.location={pathname:'/',search:'',hash:'#signed-in'};dom.window.history={replaceState(){dom.window.location.hash='';}};
  const controller=createHostController({...dom,client:f.client,schedule:(fn,ms)=>scheduled.push({fn,ms})});try{
   await controller.ready;const expiry=scheduled.find(t=>t.ms===5000);assert.ok(expiry);
   if(next==='error'){f.holdRooms(new ServiceError('SERVICE_UNAVAILABLE'));await dom.click('discover');assert.equal(dom.get('notice').dataset.error,'true');}
   if(next==='notice')await dom.click('logout');
   const before=dom.get('notice').textContent;await dom.click(next==='notice'?'nav-signin':'nav-account');expiry.fn();assert.equal(dom.get('notice').textContent,next==='navigation'?'':before);
  }finally{controller.dispose();}
 }
});

test('Presentation return marker never authenticates an unsigned session',async()=>{
 const f=fixture(),dom=await controllerDom('../candidate/index.html');f.hold({authenticated:false});dom.window.location={pathname:'/',search:'',hash:'#signed-in'};dom.window.history={replaceState(){dom.window.location.hash='';}};
 const controller=createHostController({...dom,client:f.client,schedule(){}});try{await controller.ready;assertNoCreate(dom);assert.notEqual(dom.get('notice').textContent,'Ready. Your account is signed in.');assert.equal(dom.window.location.hash,'');}finally{controller.dispose();}
});

test('Late initial lobby discovery cannot repopulate list after logout',async()=>{
 const f=fixture();let finish,started;const requested=new Promise(r=>started=r);f.holdRooms(new Promise(r=>finish=r));const rpc=f.client.rpc.bind(f.client);f.client.rpc=payload=>{if(payload.roomList)started();return rpc(payload);};
 const dom=await controllerDom('../candidate/index.html'),controller=createHostController({...dom,client:f.client});try{await requested;await f.client.logout();finish({rooms:[{roomId:'old-room',name:'Old private expedition',state:'waiting'}]});await controller.ready;assert.equal(dom.get('rooms').children.length,0);assertNoCreate(dom);assert.equal(f.client.credentials,null);}finally{controller.dispose();}
});
