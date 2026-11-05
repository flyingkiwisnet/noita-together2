import {readFile,writeFile} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import protobuf from 'protobufjs';
const root=new URL('../g3-c1/',import.meta.url);
const source=JSON.parse(await readFile(new URL('schema.json',root),'utf8'));
const scalar={string:'string',bool:'boolean',uint32:'number'};
const rustScalar={string:'String',bool:'bool',uint32:'u32'};
const snake=s=>s.startsWith('NT_')?s.toLowerCase():s.replace(/[A-Z]/g,m=>'_'+m.toLowerCase());
let proto='// GENERATED from schema.json. Do not edit.\nsyntax = "proto3";\npackage nt2.c1;\n';
let ts='// GENERATED from schema.json. Do not edit.\nexport const C1_LIMITS = '+JSON.stringify(source.limits)+' as const;\n';
let rust='// GENERATED from schema.json. Uses serde derive; no transport or secret storage.\n';
const descriptor={nested:{nt2:{nested:{c1:{nested:{}}}}}};
for(const [name,msg] of Object.entries(source.messages)){
  const fields=Object.entries(msg.fields);
  if(new Set(fields.map(([,f])=>f.id)).size!==fields.length) throw Error('Duplicate field ID');
  if(fields.some(([,f])=>!Number.isInteger(f.id)||f.id<1||f.id>536870911||(f.id>=19000&&f.id<=19999)||(!scalar[f.type]&&!source.messages[f.type])))throw Error('Invalid protobuf field definition');
  if(msg.oneof?.some(k=>!msg.fields[k]||msg.fields[k].repeated))throw Error('Invalid oneof definition');
  proto+=`\nmessage ${name} {\n`;
  for(const [key,f] of fields.filter(([k])=>!msg.oneof?.includes(k))) proto+=`  ${f.repeated?'repeated ':scalar[f.type]?'optional ':''}${f.type} ${key} = ${f.id};\n`;
  if(msg.oneof){proto+='  oneof payload {\n';for(const key of msg.oneof){const f=msg.fields[key];proto+=`    ${f.type} ${key} = ${f.id};\n`;}proto+='  }\n';}
  proto+='}\n';
  ts+=`export interface ${name} {\n`;
  rust+=`\n#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]\n#[serde(rename_all = "camelCase", deny_unknown_fields)]\npub struct ${name} {\n`;
  for(const [key,f] of fields){
    const type=scalar[f.type]??f.type;
    ts+=`  ${key}?: ${type}${f.repeated?'[]':''};\n`;
    const rt=rustScalar[f.type]??f.type;
    rust+=`    #[serde(rename = "${key}", default, skip_serializing_if = "Option::is_none")]\n    pub ${snake(key)}: Option<${f.repeated?'Vec<'+rt+'>':rt}>,\n`;
  }
  ts+='}\n';rust+='}\n';
  const d={fields:Object.fromEntries(fields.map(([k,f])=>[k,{type:f.type,id:f.id,...(f.repeated?{rule:'repeated'}:scalar[f.type]?{options:{features:{field_presence:'EXPLICIT'}}}:{})}]))};
  if(msg.oneof) d.oneofs={payload:{oneof:msg.oneof}};
  descriptor.nested.nt2.nested.c1.nested[name]=d;
}
const codec=protobuf.Root.fromJSON(descriptor);
const fixturesInput=JSON.parse(await readFile(new URL('fixture-inputs.json',root),'utf8'));
const fixtures=fixturesInput.map(({type,json})=>{const t=codec.lookupType('nt2.c1.'+type),bytes=t.encode(t.fromObject(json)).finish();return {type,json:t.toObject(t.decode(bytes),{defaults:false,arrays:false,objects:false}),hex:Buffer.from(bytes).toString('hex')};});
const rejected=[{type:'Request',json:{rulesGet:{},publish:{}}},{type:'Request',hex:fixtures[1].hex+'f80701'}];
const outputs={'g3-c1.proto':proto,'g3-c1.ts':ts,'g3-c1.rs':rust,'descriptor.json':JSON.stringify(descriptor,null,2)+'\n','golden.json':JSON.stringify({revision:source.revision,fixtures,rejected},null,2)+'\n'};
for(const [name,data]of Object.entries(outputs)){const path=new URL(name,root);if(process.argv.includes('--check')){if(await readFile(path,'utf8').catch(()=>null)!==data)throw Error('Generated contract drift: '+fileURLToPath(path));}else await writeFile(path,data);}console.log('Social contract '+(process.argv.includes('--check')?'verified':'generated'));
