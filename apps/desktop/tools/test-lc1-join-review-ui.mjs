import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
const source=readFileSync(new URL('../ui/app.js',import.meta.url),'utf8');
function fixture(invoke){
 const elements=new Map();const el=id=>{if(!elements.has(id))elements.set(id,{value:'',textContent:''});return elements.get(id);};
 const scope=vm.createContext({invoke,el,render:value=>{scope.current=value;},queued:0,uiIssued:0,uiRendered:0,queue:Promise.resolve(),current:{setup:{enabled:true}}});
 vm.runInContext(source.slice(source.indexOf('function call(command,input)'),source.indexOf('function meCandidate')),scope);
 vm.runInContext(source.slice(source.indexOf("el('join').onclick="),source.indexOf("el('room').onchange=")),scope);
 el('room').value='room';el('password').value='retained-password';return {scope,el};
}
{
 const calls=[];const fresh={setup:{enabled:true,preview:{room_id:'room',revision:4,config_revision:2}}};
 const f=fixture(async(command,{input}={})=>{calls.push({command,input});if(command==='g1_action')throw 'Review the refreshed lobby';return fresh;});
 await f.el('join').onclick();assert.equal(calls.length,2);assert.equal(calls[0].input.action,'join');assert.equal(calls[0].input.room,'room');assert.equal(calls[0].input.review_revision,undefined);assert.equal(calls[0].input.config_revision,undefined);assert.equal(calls[1].command,'g1_status');assert.equal(f.scope.current.setup.preview.revision,4);assert.equal(f.el('password').value,'retained-password');assert.equal(f.el('error').textContent,'Review the refreshed lobby');
}
{
 const calls=[];let release;const pending=new Promise(resolve=>release=resolve);
 const f=fixture(async(command,{input}={})=>{calls.push(input.action);if(input.action==='join'&&input.room==='room')await pending;return f.scope.current;});
 const join=f.scope.action('join',{room:'room'});await Promise.resolve();assert.deepEqual(calls,['join']);
 const preview=f.scope.action('preview_lobby',{room:'other'});await Promise.resolve();assert.deepEqual(calls,['join','preview_lobby'],'A newer selection must reach the backend fence while Join is in flight');release();await Promise.all([join,preview]);assert.equal(f.scope.queued,0);
}
console.log('PASS direct Join UI: one native action without user review revisions, preserved error/password and in-flight selection fencing');

{
 const scope=vm.createContext({});vm.runInContext(source.split('const el=')[0],scope);
 const shared_rules={bootstrap:true,metadata_ready:true,native_epoch_matches:true};
 assert.match(scope.recoveryDescription({phase:'Ready',host_started:true,shared_rules}),/initializing shared player rules/);
 assert.match(scope.recoveryDescription({phase:'Checking',host_started:true,waiting_for_game:true,shared_rules}),/initializing shared player rules/);
 assert.match(scope.recoveryDescription({phase:'Playing',shared_rules}),/^Playing/);
 assert.doesNotMatch(scope.recoveryDescription({phase:'Checking',waiting_for_game:true,app_connected:true}),/shared player rules/);
 console.log('PASS scoped C1 bootstrap UI: reports the current metadata phase without inventing game acknowledgement');
}
