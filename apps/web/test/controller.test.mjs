import test from 'node:test';
import assert from 'node:assert/strict';
import {createHostController} from '../public/app.js';
import {controllerDom} from './dom-fixture.mjs';
async function setup(){
 const dom=await controllerDom(),scheduled=[];
 const room={roomId:'room-a',name:'Room A',hostAccountId:'host',state:'waiting',revision:1,configRevision:1,config:{seed:424242,mode:'core',settings:[]},members:[{accountId:'player',desktopId:'desktop-a',present:true,ready:true}]};
 const client={generation:1,credentials:null,schema:{revision:'g2-draft-1'},info:{serviceId:'service',serviceEpoch:'epoch',profile:{id:'profile'},compatibility:{modes:['core'],settingKeys:[]}},async initialize(){return this.info;},async login(){this.credentials={accountId:'host',sessionId:'browser',sessionEpoch:'browser-epoch',deviceId:'tab'};},dispose(){this.credentials=null;this.generation++;},async rpc(payload){if(payload.roomCreate||payload.roomGet)return {room:structuredClone(room)};if(payload.roomList)return {rooms:[]};if(payload.chatList)return {chatMessages:[],chatNextSequence:0};if(payload.chatSend)return {};throw Error('Unexpected command');}};
 const controller=createHostController({...dom,client,schedule:fn=>scheduled.push(fn)});await controller.ready;await dom.submit('login',{subject:'host'});await dom.submit('create',{name:'Room A',seed:424242,mode:'core',visibility:'public'});
 return {dom,client,room,controller,async poll(){await scheduled.shift()();}};
}
test('background polling preserves player nodes, selection and admission drafts without disabling inputs',async()=>{
 const f=await setup();const row=f.dom.get('players').children[0],check=row.children[0].children[0],name=row.children[0].children[1];check.checked=true;await check.fire('change');const draft=f.dom.get('policy').elements.password;draft.value='unsubmitted password';
 const original=f.client.rpc;let release;f.client.rpc=async payload=>{if(payload.roomGet)await new Promise(r=>release=r);return original.call(f.client,payload);};
 const polling=f.poll();await Promise.resolve();assert.equal(draft.disabled,false);release();await polling;
 assert.equal(f.dom.get('players').children[0],row);assert.equal(row.children[0].children[1],name);assert.equal(check.checked,true);assert.equal(draft.value,'unsubmitted password');
 f.client.rpc=original;f.room.members[0].present=false;await f.poll();assert.equal(f.dom.get('players').children[0],row);assert.match(row.children[1].textContent,/offline/);f.controller.dispose();assert.equal(f.dom.get('players').children.length,0);
});
test('pending chat retains newer edits and composition; failure retains submitted draft',async()=>{
 const f=await setup(),field=f.dom.get('chat').elements.text,original=f.client.rpc;let release;
 f.client.rpc=async payload=>{if(payload.chatSend)return new Promise(r=>release=r);return original.call(f.client,payload);};
 const send=f.dom.submit('chat',{text:'first'});await Promise.resolve();field.value='new draft';release({});await send;assert.equal(field.value,'new draft');
 const composing=f.dom.submit('chat',{text:'same draft'});await Promise.resolve();await field.fire('compositionstart');release({});await composing;assert.equal(field.value,'same draft');await field.fire('compositionend');
 f.client.rpc=async payload=>{if(payload.chatSend)throw Error('offline');return original.call(f.client,payload);};await f.dom.submit('chat',{text:'retry me'});assert.equal(field.value,'retry me');assert.equal(f.dom.get('notice').dataset.error,'true');
 f.client.rpc=original;await f.dom.submit('chat',{text:'finished'});assert.equal(field.value,'');f.controller.dispose();
});
