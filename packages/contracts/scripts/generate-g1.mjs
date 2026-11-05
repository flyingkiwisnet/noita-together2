import {readFile,writeFile} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import protobuf from 'protobufjs';
const root=new URL('../g1/',import.meta.url);
const source=JSON.parse(await readFile(new URL('schema.json',root),'utf8'));
const scalar={string:'string',bool:'boolean',uint32:'number'};
const rustScalar={string:'String',bool:'bool',uint32:'u32'};
const snake=s=>s.replace(/[A-Z]/g,m=>'_'+m.toLowerCase());
let proto='// GENERATED from schema.json. Do not edit.\nsyntax = "proto3";\npackage nt2.g1;\n';
let ts='// GENERATED from schema.json. Do not edit.\nexport const G1_LIMITS = '+JSON.stringify(source.limits)+' as const;\n';
let rust='// GENERATED from schema.json. Uses serde derive; no transport or secret storage.\n';
const descriptor={nested:{nt2:{nested:{g1:{nested:{}}}}}};
for(const [name,msg] of Object.entries(source.messages)){
  const fields=Object.entries(msg.fields);
  if(new Set(fields.map(([,f])=>f.id)).size!==fields.length) throw Error('Duplicate field ID');
  if(fields.some(([,f])=>!Number.isInteger(f.id)||f.id<1||f.id>536870911||(f.id>=19000&&f.id<=19999)||(!scalar[f.type]&&!source.messages[f.type])))throw Error('Invalid protobuf field definition');
  if(msg.oneof?.some(k=>!msg.fields[k]||msg.fields[k].repeated))throw Error('Invalid oneof definition');
  proto+=`\nmessage ${name} {\n`;
  for(const [key,f] of fields.filter(([k])=>!msg.oneof?.includes(k))) proto+=`  ${f.repeated?'repeated ':''}${f.type} ${key} = ${f.id};\n`;
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
  const d={fields:Object.fromEntries(fields.map(([k,f])=>[k,{type:f.type,id:f.id,...(f.repeated?{rule:'repeated'}:{})}]))};
  if(msg.oneof) d.oneofs={payload:{oneof:msg.oneof}};
  descriptor.nested.nt2.nested.g1.nested[name]=d;
}
const codec=protobuf.Root.fromJSON(descriptor);
const samples=[{type:'Request',json:{major:source.major,revision:source.revision,serviceId:'nt2-g1-local',requestId:'golden-request',roomGet:{}}},{type:'Request',json:{major:source.major,revision:source.revision,serviceId:'nt2-g1-local',requestId:'golden-start',serviceEpoch:'service-epoch',sessionEpoch:'desktop-epoch',roomId:'room',expectedRevision:7,configRevision:1,start:{participantDesktopIds:['desktop-a','desktop-b'],excludedDesktopIds:[]}}},{type:'Response',json:{major:source.major,revision:source.revision,serviceId:'nt2-g1-local',serviceEpoch:'new-epoch',requestId:'stale-request',error:{code:'STALE_EPOCH',message:'STALE_EPOCH',retryAfterMs:0}}}];
samples.push({type:'Request',json:{major:source.major,revision:source.revision,serviceId:'nt2-g1-local',requestId:'golden-preview',serviceEpoch:'service-epoch',sessionEpoch:'guest-epoch',roomId:'11111111-1111-4111-8111-111111111111',roomPreview:{}}});
samples.push({type:'Response',json:{major:source.major,revision:source.revision,serviceId:'nt2-g1-local',serviceEpoch:'service-epoch',requestId:'golden-preview',admission:{roomId:'11111111-1111-4111-8111-111111111111',name:'Public lobby',state:'waiting',revision:2,configRevision:1,visibility:'public',joinAllowed:true}}});
samples.push({type:'Request',json:{major:source.major,revision:source.revision,serviceId:'nt2-g1-local',requestId:'golden-join',serviceEpoch:'service-epoch',sessionEpoch:'guest-epoch',roomId:'11111111-1111-4111-8111-111111111111',expectedRevision:2,configRevision:1,roomJoin:{}}});
samples.push({type:'Response',json:{major:source.major,revision:source.revision,serviceId:'nt2-g1-local',serviceEpoch:'service-epoch',requestId:'golden-private-denial',error:{code:'NOT_FOUND',message:'NOT_FOUND',retryAfterMs:0}}});
const fixtures=samples.map(({type,json})=>{const t=codec.lookupType('nt2.g1.'+type);const bytes=t.encode(t.fromObject(json)).finish();const normalized=t.toObject(t.decode(bytes),{defaults:false,arrays:false,objects:false});return {type,json:normalized,hex:Buffer.from(bytes).toString('hex')};});
const rejected=[{name:'unknown-json-field',type:'Request',json:{...samples[0].json,sender:'host'},error:'INVALID_MESSAGE'},{name:'conflicting-command',type:'Request',json:{...samples[0].json,logout:{}},error:'INVALID_MESSAGE'},{name:'unknown-wire-field',type:'Request',hex:fixtures[0].hex+'7801',error:'INVALID_MESSAGE'},{name:'duplicate-wire-field',type:'Request',hex:fixtures[0].hex+'0801',error:'INVALID_MESSAGE'},{name:'invalid-utf8',type:'PairBegin',hex:'0a01ff',error:'INVALID_MESSAGE'}];
rejected.push({name:'private-data-in-summary',type:'RoomSummary',json:{roomId:'11111111-1111-4111-8111-111111111111',config:{seed:424242}},error:'INVALID_MESSAGE'});
rejected.push({name:'unknown-summary-wire-field',type:'RoomSummary',hex:'4200',error:'INVALID_MESSAGE'});
const outputs={'g1.proto':proto,'g1.ts':ts,'g1.rs':rust,'descriptor.json':JSON.stringify(descriptor,null,2)+'\n','golden.json':JSON.stringify({revision:source.revision,fixtures,rejected},null,2)+'\n'};
for(const [name,data] of Object.entries(outputs)){
 const path=new URL(name,root);
 if(process.argv.includes('--check')){if(await readFile(path,'utf8').catch(()=>null)!==data)throw Error('Generated contract drift: '+fileURLToPath(path));}
 else await writeFile(path,data);
}
console.log('G1 generated contract '+(process.argv.includes('--check')?'verified':'written'));
