import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
const source=readFileSync(new URL('../ui/app.js',import.meta.url),'utf8');
const nodes=new Map(),el=id=>{if(!nodes.has(id))nodes.set(id,{value:'',textContent:'',hidden:false,disabled:false});return nodes.get(id);};
const context=vm.createContext({el,loginBusy:false,loginMessage:'',document:{createElement:()=>({})},text:(node,value)=>node.textContent=String(value??''),reconcile:(node,values,key,create,update)=>{node.options=values.map(value=>{const option=create();update(option,value);return option;});}});
vm.runInContext(source.split('const el=')[0],context);
vm.runInContext(source.slice(source.indexOf('function renderLogin(v)'),source.indexOf('async function doLogin')),context);
const view={protocol_revision:'g2-draft-5',server_origin:'https://service.test',authentication:{discovered:true,providers:[{id:'twitch',available:false,reason:'not-configured'},{id:'google',available:false,reason:'not-configured'},{id:'local',available:true}]}};
assert.equal(context.auth1Providers({}).some(p=>p.available),false);assert.equal(context.auth1Providers({mock:false}).some(p=>p.id==='website'),false);assert.match(context.auth1Providers({mock:true})[0].label,/development mock/);
for(const [role,title,privileged] of [['user','User',false],['power-user','Power User',true],['admin','Admin',true],['dev','Dev',true]]){
 const account={displayName:'<img onerror=bad>',provider:'local',role,entitlements:{capacityLimit:privileged?16:8,customLobbyName:privileged,testingMode:role==='dev',manageUsers:['admin','dev'].includes(role)}};
 context.renderLogin({...view,signed_in:true,authentication:{...view.authentication,account,password_change_available:true}});
 assert.match(el('account-summary').textContent,new RegExp(title));assert.ok(el('account-summary').textContent.startsWith('<img onerror=bad>'));assert.equal(el('change-password').hidden,false);
}
context.renderLogin({...view,signed_in:true,authentication:{...view.authentication,account_error:'UNAUTHENTICATED'}});assert.match(el('account-summary').textContent,/permissions unavailable/);assert.equal(el('change-password').hidden,true);
assert.equal(context.auth1AccountDescription({...view,signed_in:true,principal:{kind:'guest'}}),'Guest session');
assert.match(source,/cancel-signin'\)\.onclick=\(\)=>action\('cancel_sign_in'\)/);
console.log('PASS AUTH1 account UI: closed providers, role changes, stale permissions, literal text and browser-only password action; direct entry covered by UX1 shipped UI');
