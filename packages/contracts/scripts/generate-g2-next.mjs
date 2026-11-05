import {readFile,writeFile} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import protobuf from 'protobufjs';
const root=new URL('../g2-next/',import.meta.url);
const source=JSON.parse(await readFile(new URL('schema.json',root),'utf8'));
const scalar={string:'string',bool:'boolean',uint32:'number'};
const rustScalar={string:'String',bool:'bool',uint32:'u32'};
const snake=s=>s.replace(/[A-Z]/g,m=>'_'+m.toLowerCase());
let proto='// GENERATED from schema.json. Do not edit.\nsyntax = "proto3";\npackage nt2.g2next;\n';
let ts='// GENERATED from schema.json. Do not edit.\nexport const G2_NEXT_LIMITS = '+JSON.stringify(source.limits)+' as const;\n';
let rust='// GENERATED from schema.json. Uses serde derive; no transport or secret storage.\n';
const descriptor={nested:{nt2:{nested:{g2next:{nested:{}}}}}};
for(const [name,msg] of Object.entries(source.messages)){
  const fields=Object.entries(msg.fields);const explicit=name.startsWith('Connection');
  if(new Set(fields.map(([,f])=>f.id)).size!==fields.length) throw Error('Duplicate field ID');
  if(fields.some(([,f])=>!Number.isInteger(f.id)||f.id<1||f.id>536870911||(f.id>=19000&&f.id<=19999)||(!scalar[f.type]&&!source.messages[f.type])))throw Error('Invalid protobuf field definition');
  if(msg.oneof?.some(k=>!msg.fields[k]||msg.fields[k].repeated))throw Error('Invalid oneof definition');
  proto+=`\nmessage ${name} {\n`;
  for(const [key,f] of fields.filter(([k])=>!msg.oneof?.includes(k))) proto+=`  ${f.repeated?'repeated ':(explicit||key==='publicStats')&&scalar[f.type]?'optional ':''}${f.type} ${key} = ${f.id}${f.repeated&&f.type==='uint32'?' [packed=false]':''};\n`;
  if(msg.oneof){proto+='  oneof payload {\n';for(const key of msg.oneof){const f=msg.fields[key];proto+=`    ${f.type} ${key} = ${f.id};\n`;}proto+='  }\n';}
  proto+='}\n';
  ts+=`export interface ${name} {\n`;
  rust+=`\n#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]\n#[serde(rename_all = "camelCase", deny_unknown_fields)]\npub struct ${name} {\n`;
  for(const [key,f] of fields){
    const type=scalar[f.type]??f.type;
    ts+=`  ${key}?: ${type}${f.repeated?'[]':''};\n`;
    const rt=rustScalar[f.type]??f.type;
    rust+=`    #[serde(default, skip_serializing_if = "Option::is_none")]\n    pub ${snake(key)}: Option<${f.repeated?'Vec<'+rt+'>':rt}>,\n`;
  }
  ts+='}\n';rust+='}\n';
  const d={fields:Object.fromEntries(fields.map(([k,f])=>[k,{type:f.type,id:f.id,...(f.repeated?{rule:'repeated',...(f.type==='uint32'?{options:{packed:false}}:{})}:explicit&&scalar[f.type]?{options:{proto3_optional:true}}:k==='publicStats'?{options:{features:{field_presence:'EXPLICIT'}}}:{})}]))};
  if(msg.oneof) d.oneofs={payload:{oneof:msg.oneof}};
  if(explicit)for(const [k,f]of fields)if(!f.repeated&&scalar[f.type]&&!msg.oneof?.includes(k)){d.oneofs??={};d.oneofs['_'+k]={oneof:[k]};}
  descriptor.nested.nt2.nested.g2next.nested[name]=d;
}
const codec=protobuf.Root.fromJSON(descriptor);
const envelope={major:source.major,revision:source.revision,serviceId:'nt2-g2-next-local',requestId:'golden-request'};
const samples=[
 {type:'Request',json:{...envelope,roomGet:{}}},
 {type:'Request',json:{...envelope,guestIssue:{deviceId:'desktop-a',displayName:'Alex',issuanceProof:'rust-held-random-proof'}}},
 {type:'Request',json:{...envelope,authBegin:{deviceId:'desktop-a',appInstanceId:'app-a',provider:'twitch',proofChallenge:'a'.repeat(64),callbackPort:49152,callbackState:'rust-held-local-state'}}},
 {type:'Request',json:{...envelope,prepare:{},roomId:'room',expectedRevision:2,configRevision:1}},
 {type:'Request',json:{...envelope,gameObservation:{attachmentId:'attachment-a',gameInstanceId:'game-a',worldToken:'world-a',observationRevision:1,actualSeed:'424242',seedStatus:'known',collectorFailure:'',modVersion:'test',bridgeVersion:'test',compatibilityProvenance:'game-thread-client-observation',activeMods:['noita_together2','example'],inventoryComplete:true,beta:'unknown',protected:true,loaded:true}}},
 {type:'Request',json:{...envelope,finalStart:{participants:[{desktopId:'desktop-a',attachmentId:'attachment-a',worldToken:'world-a',observationRevision:1,contentDigest:'b'.repeat(64)}],excludedDesktopIds:[]}}},
 {type:'Response',json:{...envelope,serviceEpoch:'service-epoch',credentials:{accountId:'principal-a',kind:'desktop',principal:{principalId:'principal-a',kind:'guest',displayName:'Alex',source:'guest'}}}}
];
samples.push({type:'Response',json:{...envelope,serviceEpoch:'service-epoch',gameStatus:{phase:'restoring',reason:''},release:{authorizationId:'authorization-a',startRequestId:'start-request-a',contractRevision:source.revision,serviceId:envelope.serviceId,serviceEpoch:'service-epoch',roomId:'room-a',runId:'run-a',runEpoch:'run-epoch',configRevision:1,desktopId:'desktop-a',desktopSessionEpoch:'desktop-epoch',gameSessionId:'game-session-a',gameSessionEpoch:'game-epoch',attachmentId:'attachment-a',gameInstanceId:'game-a',worldToken:'world-a',observationRevision:1,contentDigest:'b'.repeat(64),capturedAt:'2026-10-06T00:00:00.000Z',authorizedAt:'2026-10-06T00:00:01.000Z',releaseExpiresAt:'2026-10-06T00:30:00.000Z',phase:'restoring',enteredPlayingAt:'2026-10-06T00:00:02.000Z'}}});
samples.push({type:'CompatibilityProfile',json:JSON.parse(await readFile(new URL('../g2-next/profile-fixture.json',import.meta.url),'utf8'))});
samples.push({type:'Request',json:{...envelope,preparationRenew:{preparationId:'preparation-a'},roomId:'room-a',runId:'run-a',runEpoch:'run-epoch',configRevision:1}});
samples.push({type:'Member',json:{desktopId:'desktop-a',present:true,ready:false,included:true,consentedConfigRevision:1,readiness:'waiting_noita',readinessReason:'actual_game_required'}});
samples.push({type:'Member',json:{desktopId:'desktop-b',present:true,ready:true,included:true,consentedConfigRevision:1,attached:true,loaded:true,seedApplied:true,readiness:'ready',readinessReason:''}});
samples.push({type:'Response',json:{...envelope,serviceEpoch:'service-epoch',admission:{roomId:'room-a',name:'Visible lobby',state:'staging',revision:2,configRevision:1,visibility:'public',joinAllowed:true},room:{roomId:'room-a',name:'Visible lobby',state:'staging',revision:2,configRevision:1,config:{seed:424242,mode:'core',settings:[{key:'exampleRule',value:'displayed'}],requiredCapabilities:['core.seed']}}}});
for(const [seedPolicy,seed,randomizeLoot] of [['shared-fixed',424242,false],['shared-fixed',424242,true],['shared-random',0,true],['shared-random',123456,true],['individual-random',0,false]])samples.push({type:'Config',json:{seedPolicy,seed,randomizeLoot,mode:'core',requiredCapabilities:['core.seed-policy.v1',...(randomizeLoot?['core.randomize-loot.v1']:[])]}});
for(const seedPolicy of ['shared-random','individual-random'])samples.push({type:'GameRelease',json:{...samples.find(s=>s.json.release).json.release,actualSeed:seedPolicy==='shared-random'?'123456':'987',seedPolicy,randomizeLoot:seedPolicy==='shared-random'}});
samples.push({type:'Request',json:{...envelope,roomId:'room-a',expectedRevision:3,configRevision:1,roomDelete:{}}},{type:'Response',json:{...envelope,deletedRoomId:'room-a'}});
// R2: strict browser-login consumers must recognize exact-issued-token samples.
// Cached original replies retain identity and shorten the remaining duration.
for(const accessRemainingMs of [800000,799000])samples.push({type:'BrowserLoginResponse',json:{credentials:{accessToken:'synthetic-browser-access-not-a-token',refreshSecret:'synthetic-browser-refresh-not-a-secret',accountId:'principal-a',sessionId:'browser-session-a',sessionEpoch:'browser-epoch-a',kind:'browser',deviceId:'fixture-browser',expiresAt:'2026-10-15T01:15:00.000Z'},credentialValidity:{principalId:'principal-a',sessionId:'browser-session-a',sessionEpoch:'browser-epoch-a',credentialId:'fixture-credential-id',accessRemainingMs,originRequestId:'fixture-browser-login-original',sampleKey:'1'.repeat(64)}}});
// UX1 consumers share these concrete DTO fixtures with the service and website.
const ux1Account='11111111-1111-4111-8111-111111111111',ux1Desktop='22222222-2222-4222-8222-222222222222';
samples.push({type:'Request',json:{...envelope,roomPlayerDetails:{accountId:ux1Account,desktopId:ux1Desktop}}});
samples.push({type:'Request',json:{...envelope,roomBanList:{afterAccountId:ux1Account,limit:32}}});
samples.push({type:'Request',json:{...envelope,roomModerate:{accountId:ux1Account,desktopId:ux1Desktop,action:'mute'}}});
samples.push({type:'PlayerDetails',json:{accountId:ux1Account,desktopId:ux1Desktop,principal:{principalId:ux1Account,kind:'registered',displayName:'Fixture player',source:'twitch'},role:'user',chatMuted:false,banned:false,roomRevision:4,actions:['kick','ban','mute']}});
samples.push({type:'RoomCreate',json:{name:'Fixture lobby',capacity:8,password:'synthetic-lobby-password',visibility:'public',config:{seed:0,seedPolicy:'shared-random',randomizeLoot:true,mode:'core',settings:[],requiredCapabilities:['core.seed','core.seed-policy.v1','core.randomize-loot.v1']}}});
samples.push({type:'RoomSummary',json:{roomId:ux1Desktop,name:'Fixture lobby',state:'staging',host:{principalId:ux1Account,kind:'registered',displayName:'Fixture host',source:'google'},mode:'core',playerCount:2,capacity:8}});
const fixtures=samples.map(({type,json})=>{const t=codec.lookupType('nt2.g2next.'+type);const bytes=t.encode(t.fromObject(json)).finish();const normalized=t.toObject(t.decode(bytes),{defaults:false,arrays:false,objects:false});return {type,json:normalized,hex:Buffer.from(bytes).toString('hex')};});
const rejected=[{name:'unknown-json-field',type:'Request',json:{...samples[0].json,sender:'host'},error:'INVALID_MESSAGE'},{name:'conflicting-command',type:'Request',json:{...samples[0].json,logout:{}},error:'INVALID_MESSAGE'},{name:'unknown-wire-field',type:'Request',hex:fixtures[0].hex+'7801',error:'INVALID_MESSAGE'},{name:'duplicate-wire-field',type:'Request',hex:fixtures[0].hex+'0801',error:'INVALID_MESSAGE'},{name:'invalid-utf8',type:'AuthBegin',hex:'0a01ff',error:'INVALID_MESSAGE'}];
rejected.push({name:'private-data-in-summary',type:'RoomSummary',json:{roomId:'11111111-1111-4111-8111-111111111111',config:{seed:424242}},error:'INVALID_MESSAGE'});

for(const [name,json] of [['unknown-seed-policy',{seed:1,seedPolicy:'arbitrary',randomizeLoot:false}],['individual-forced-seed',{seed:1,seedPolicy:'individual-random',randomizeLoot:false}],['individual-extra-loot',{seed:0,seedPolicy:'individual-random',randomizeLoot:true}],['untyped-loot',{seed:1,seedPolicy:'shared-fixed',randomizeLoot:'true'}],['omitted-policy-loot',{seed:1,randomizeLoot:true}]])rejected.push({name,type:'Config',json,error:'INVALID_MESSAGE'});
const outputs={'g2-next.proto':proto,'g2-next.ts':ts,'g2-next.rs':rust,'descriptor.json':JSON.stringify(descriptor,null,2)+'\n','golden.json':JSON.stringify({revision:source.revision,fixtures,rejected},null,2)+'\n'};
for(const [name,data] of Object.entries(outputs)){
 const path=new URL(name,root);
 if(process.argv.includes('--check')){if(await readFile(path,'utf8').catch(()=>null)!==data)throw Error('Generated contract drift: '+fileURLToPath(path));}
 else await writeFile(path,data);
}
console.log('G2 generated contract '+(process.argv.includes('--check')?'verified':'written'));
