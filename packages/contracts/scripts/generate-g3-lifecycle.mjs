import fs from 'node:fs';
const folder=new URL('../g3-lifecycle/',import.meta.url);fs.mkdirSync(folder,{recursive:true});
const base=JSON.parse(fs.readFileSync(new URL('../g3-waiting/schema.json',import.meta.url)));
const f=(id,type,extra={})=>({id,type,required:true,...extra}),str=(id,max=128)=>f(id,'string',{max});
const messages={
 PrepareWorld:{fields:{worldId:str(1),roomId:str(2),proof:str(3,43),configRevision:f(4,'uint32'),configDigest:str(5,64)}},
 PreparedWorld:{fields:{association:f(1,'WorldAssociation'),config:f(2,'Config'),runEpoch:str(3),reviewRemainingMs:f(4,'uint32',{maxValue:3600000})}},
 WorldAssociation:{fields:{worldId:str(1),roomId:str(2),runId:str(3),preparationId:str(4),configRevision:f(5,'uint32'),configDigest:str(6,64)}},
 Reconcile:{fields:{association:f(1,'WorldAssociation'),proof:str(2,43),attempt:f(3,'uint32',{maxValue:2147483647}),connectionId:str(4),action:f(5,'string',{values:['connect','offline'],max:16}),gameVersion:str(6),modVersion:str(7),bridgeVersion:str(8),capabilities:f(9,'string',{repeated:true,max:32,itemMax:128}),observation:f(10,'GameObservation',{required:false})}},
 WorldState:{fields:{association:f(1,'WorldAssociation'),state:f(2,'string',{values:['waiting','playing','retry','review','terminal'],max:16}),reason:str(3),attempt:f(4,'uint32'),generation:f(5,'uint32'),mayPlay:f(6,'bool'),retryAfterMs:f(7,'uint32',{maxValue:30000}),ownerRemainingMs:f(8,'uint32',{maxValue:30000}),reviewRemainingMs:f(9,'uint32',{maxValue:3600000}),runEpoch:str(10),participantId:f(11,'string',{required:false,max:128}),currentReport:f(12,'ConnectionReport',{required:false}),connectionId:f(13,'string',{required:false,max:128})}},
 Request:{fields:{major:f(1,'uint32'),revision:str(2),serviceId:str(3),serviceEpoch:str(4),sessionEpoch:str(5),requestId:str(6),reconcile:f(10,'Reconcile',{required:false}),prepare:f(11,'PrepareWorld',{required:false})},oneof:['reconcile','prepare']},
 Response:{fields:{major:f(1,'uint32'),revision:str(2),serviceId:str(3),serviceEpoch:str(4),requestId:str(5),world:f(10,'WorldState',{required:false}),credentials:f(11,'Credentials',{required:false}),credentialValidity:f(12,'ConnectionCredential',{required:false}),canonicalAdmission:f(13,'ConnectionAdmission',{required:false}),prepared:f(14,'PreparedWorld',{required:false}),error:f(20,'Error',{required:false})}}
};
const lobby=JSON.parse(fs.readFileSync(new URL('../g2-next/schema.json',import.meta.url)));
function pull(type){if(messages[type]||['string','bool','uint32'].includes(type))return;messages[type]=base.messages[type]??lobby.messages[type];if(!messages[type])throw Error(type);for(const field of Object.values(messages[type].fields))pull(field.type);}
for(const m of Object.values({...messages}))for(const field of Object.values(m.fields))pull(field.type);
const schema={major:1,revision:'g3-lifecycle-lc1-2',limits:base.limits,messages};
const write=(name,data)=>{const u=new URL(name,folder);if(process.argv.includes('--check')){if(fs.readFileSync(u,'utf8')!==data)throw Error('Lifecycle generated drift '+name);}else fs.writeFileSync(u,data);};
write('schema.json',JSON.stringify(schema,null,2)+'\n');
// Reuse the existing deterministic schema generator, changing only namespace/output names.
let generator=fs.readFileSync(new URL('./generate-g3-waiting.mjs',import.meta.url),'utf8').replaceAll('g3-waiting','g3-lifecycle').replaceAll('nt2.waiting','nt2.lifecycle').replaceAll('nested:{waiting:','nested:{lifecycle:').replaceAll('.nested.waiting','.nested.lifecycle').replaceAll('WAITING_LIMITS','LIFECYCLE_LIMITS');
// The shared generator resolves its schema relative to its own scripts directory.
const generated=new URL('./.generate-g3-lifecycle-current.mjs',import.meta.url);fs.writeFileSync(generated,generator);
try{await import(generated.href+'?run='+Date.now());}finally{fs.unlinkSync(generated);}
