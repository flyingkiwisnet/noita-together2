import test from 'node:test';import assert from 'node:assert/strict';import {readFileSync} from 'node:fs';
import {AUTH1_ROLES,maySetRole,mayResetLocal,normalizeUsername,auth1Entitlements,auth1Limits} from '../../../packages/domain/src/auth1.js';
import {validate,parseJson,decode,encode} from '../../../packages/contracts/src/g3-auth1.js';import {Providers,authenticationFromEnvironment,normalizeTwitchTokenScope,reportAuthFailure} from '../src/auth1/providers.js';
test('AUTH1 role matrix and current entitlements are cumulative and bounded',()=>{for(const actor of AUTH1_ROLES)for(const target of AUTH1_ROLES)for(const next of AUTH1_ROLES){assert.equal(maySetRole(actor,target,next),actor==='dev'||actor==='admin'&&['user','power-user'].includes(target)&&['user','power-user'].includes(next));assert.equal(mayResetLocal(actor,target),actor==='dev'||actor==='admin'&&['user','power-user'].includes(target));}
 for(const [rank,role]of AUTH1_ROLES.entries()){const e=auth1Entitlements(role,'人'.repeat(60),true);assert.ok(Buffer.byteLength(e.generatedLobbyName)<=80);assert.equal(e.customLobbyName,rank>=1);assert.equal(e.capacityLimit,rank>=1?16:8);assert.equal(e.manageUsers,rank>=2);assert.equal(e.testingMode,rank===3);assert.equal(auth1Entitlements(role,'Alex',false).testingMode,false);}
 for(const pair of [[0,16],[16,8],[8,33],[1.5,16]])assert.throws(()=>auth1Limits(pair[0],pair[1]));assert.equal(maySetRole('admin','dev','user'),false);assert.equal(maySetRole('unknown','user','dev'),false);
});
test('AUTH1 username policy and generated canonical fixtures reject malformed roles, bounds, duplicates and missing authority metadata',()=>{
 assert.equal(normalizeUsername('  ＴＥＳＴ.User  '),'test.user');for(const name of ['ab','a@b','a b','a'.repeat(33),'../root','a\u0000b'])assert.throws(()=>normalizeUsername(name));
 const golden=JSON.parse(readFileSync(new URL('../../../packages/contracts/g3-auth1/golden.json',import.meta.url),'utf8'));for(const f of golden.fixtures){assert.deepEqual(decode(f.type,Buffer.from(f.hex,'hex')),f.json);assert.deepEqual(parseJson(f.type,JSON.stringify(f.json)),f.json);assert.equal(Buffer.from(encode(f.type,f.json)).toString('hex'),f.hex);}
 const account=structuredClone(golden.fixtures.find((f:any)=>f.type==='AccountResponse').json);for(const role of ['administrator','owner','DEV','']){const bad=structuredClone(account);bad.account.role=role;assert.throws(()=>validate('AccountResponse',bad));}
 for(const key of ['accountId','role','entitlements']){const bad=structuredClone(account);delete bad.account[key];assert.throws(()=>validate('AccountResponse',bad));}account.account.entitlements.capacityLimit=33;assert.throws(()=>validate('AccountResponse',account));
 assert.throws(()=>parseJson('LocalCreate','{"username":"first","username":"second","operationId":"x"}'));assert.throws(()=>validate('LocalCreate',{username:'local',role:'dev',operationId:'x'}));assert.throws(()=>validate('RoleChange',{role:'dev'}));
});
test('AUTH1 provider discovery remains explicit and never mocks an absent provider or accepts an unsafe public origin',()=>{
 assert.deepEqual(new Providers({origin:'https://nt2.flyingkiwis.net'}).discovery().map(v=>[v.id,v.available]),[['twitch',false],['google',false],['local',true]]);for(const origin of ['http://nt2.flyingkiwis.net','https://nt2.flyingkiwis.net/','https://nt2.flyingkiwis.net/path','https://user:secret@nt2.flyingkiwis.net'])assert.throws(()=>new Providers({origin}));
});
test('Twitch scope adaptation is bounded and diagnostics contain only fixed fields and allowlisted codes',()=>{
 const token={scope:['openid','channel:read:subscriptions'],access_token:'private-access',id_token:'private-id'};assert.deepEqual(normalizeTwitchTokenScope(token),{...token,scope:'openid channel:read:subscriptions'});assert.deepEqual(token.scope,['openid','channel:read:subscriptions']);assert.deepEqual(normalizeTwitchTokenScope({scope:'openid'}),{scope:'openid'});
 for(const scope of [[{}],[null],[42],[''],['openid profile'],['a\nsecret'],['a'.repeat(129)],Array(65).fill('openid')])assert.throws(()=>normalizeTwitchTokenScope({scope}));
 const events:unknown[]=[],options={origin:'https://nt2.flyingkiwis.net',diagnostic:(event:unknown)=>{events.push(event);}};
 reportAuthFailure(options,'twitch','token-grant',{code:'private-secret',message:'private-message',stack:'private-stack',body:token,cause:token});reportAuthFailure(options,'google','userinfo',{code:'OAUTH_INVALID_RESPONSE',body:token});
 assert.deepEqual(events,[{event:'auth1-signin-failure',provider:'twitch',stage:'token-grant',code:'UNCLASSIFIED'},{event:'auth1-signin-failure',provider:'google',stage:'userinfo',code:'OAUTH_INVALID_RESPONSE'}]);assert.doesNotMatch(JSON.stringify(events),/private|access_token|id_token|message|stack|body|cause/);
});
