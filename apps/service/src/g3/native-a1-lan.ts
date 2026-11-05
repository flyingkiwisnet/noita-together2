// Separate A1 manual TEST path; previous launcher/configs remain immutable.
// Reuses the proven LAN origin/certificate/port checks, and starts no DB/app.
import {readFile} from 'node:fs/promises';import {dirname,resolve} from 'node:path';import {createHash,generateKeyPairSync,randomBytes} from 'node:crypto';import {createSecureContext} from 'node:tls';
import {lanConfig,validateLanConfig,localInterface,validateCertificate} from '../g2/lan.js';import {ensureUnusedPort} from '../g2-next/launch.js';
import {verifyNativeA1DevelopmentArtifacts,nativeA1RuntimeResources,nativeA1SourceReviewResource} from './native-a1-profile.js';
const root=new URL('../../../../',import.meta.url),sha=(b:Buffer)=>createHash('sha256').update(b).digest('hex');
export function nativeA1LanConfig(ip:unknown,port:unknown){
 const base=lanConfig(ip,port);if(base.ip!=='192.168.0.104'||![4356,4357].includes(base.port))throw Error('A1 test launcher requires assigned host192.168.0.104 and4356/4357');
 return {...base,serviceId:'nt2-g3-native-social-development-a1',namespace:'g3_social_a1_lan_'+base.ip.replaceAll('.','_')+'_'+base.port};
}
export function validateNativeA1LanConfig(value:unknown){
 if(!value||typeof value!=='object'||Array.isArray(value))throw Error('Invalid A1 LAN configuration');const v=value as Record<string,unknown>,base=nativeA1LanConfig(v.ip,v.port);
 const expected={...base,a1Runtime:true,runtimeApprovalSha256:nativeA1RuntimeResources.social.sha256,sourceReviewSha256:nativeA1SourceReviewResource.sha256,tlsSourceConfig:'../../secrets/g2-lan-192-168-0-104-4352/config.json',databaseSecretConfig:'../../secrets/g3-native-social-lan/database.json'};
 if(Object.keys(v).sort().join()!==Object.keys(expected).sort().join()||Object.entries(expected).some(([k,x])=>v[k]!==x))throw Error('A1 LAN identity/namespace/TLS paths/exact source resource must match; no caller capability grant');
 return expected;
}
type A1LanSelectionConfig=Omit<ReturnType<typeof validateNativeA1LanConfig>,'runtimeApprovalSha256'|'sourceReviewSha256'>&{runtimeApprovalSha256:string;sourceReviewSha256:string};
export async function prepareNativeA1LanLaunch(configPath:string,selection?:{validateConfig:(value:unknown)=>A1LanSelectionConfig;verifyMetadata:()=>ReturnType<typeof verifyNativeA1DevelopmentArtifacts>;a1RuntimeFix?:true;a1RuntimeDiagnostic?:true;a1HeartbeatBusy?:true}){
 if(process.env.NODE_ENV==='production')throw Error('A1 manual test launcher refuses production');const config=(selection?.validateConfig??validateNativeA1LanConfig)(JSON.parse(await readFile(configPath,'utf8')));localInterface(config.ip);
 const metadata=await (selection?.verifyMetadata??(()=>verifyNativeA1DevelopmentArtifacts(true)))();
 // Existing listeners stay untouched. This check opens no service or DB connection.
 try{await ensureUnusedPort(config.ip,config.port);}catch{throw Error('A1 test port occupied/unavailable; existing listener unchanged; no app/database started');}
 const sourcePath=resolve(dirname(resolve(configPath)),config.tlsSourceConfig),source=validateLanConfig(JSON.parse(await readFile(sourcePath,'utf8')));
 if(source.ip!==config.ip||source.namespace===config.namespace||source.serviceId===config.serviceId||source.port===config.port)throw Error('Reuse only the separate existing same-IP certificate source');
 const publicFolder=new URL('deploy/g2-lan/public/',root),bundle=JSON.parse(await readFile(new URL('bundle.json',publicFolder),'utf8')),ca=await readFile(new URL('ca.cer',publicFolder));
 if(bundle.origin!==source.origin||bundle.serviceId!==source.serviceId||sha(ca)!==bundle.caSha256||bundle.caSha256!=='730917075cc240a6c9eecdc0d05b1b4d837b79fab325d796fda7f804c9dd6d22')throw Error('Existing public TLS bundle/source/CA differs');
 validateCertificate(await readFile(new URL('server.cer',publicFolder)),ca,config.ip);
 const directory=dirname(sourcePath),tlsSecret=JSON.parse(await readFile(resolve(directory,'tls.json'),'utf8'));if(typeof tlsSecret.passphrase!=='string'||tlsSecret.passphrase.length<32)throw Error('Invalid local TLS source');
 const tls={pfx:await readFile(resolve(directory,'server.pfx')),passphrase:tlsSecret.passphrase};createSecureContext(tls);
 const databasePath=resolve(dirname(resolve(configPath)),config.databaseSecretConfig),databaseSecret=JSON.parse(await readFile(databasePath,'utf8'));if(typeof databaseSecret.databaseUrl!=='string')throw Error('Missing LOCAL database configuration');
 let databaseUrl:URL;try{databaseUrl=new URL(databaseSecret.databaseUrl);}catch{throw Error('Invalid LOCAL database configuration');}if(!['postgres:','postgresql:'].includes(databaseUrl.protocol)||!['127.0.0.1','localhost','[::1]'].includes(databaseUrl.hostname)||databaseUrl.search||databaseUrl.hash||!databaseUrl.pathname||!databaseUrl.username)throw Error('A1 test uses existing local PostgreSQL only');
 const pair=generateKeyPairSync('ed25519'),keys={privateKey:pair.privateKey.export({format:'pem',type:'pkcs8'}).toString(),publicKey:pair.publicKey.export({format:'pem',type:'spki'}).toString(),recoveryKey:randomBytes(32).toString('base64url'),issuer:config.issuer};
 return {config,metadata,options:{databaseUrl:databaseSecret.databaseUrl,namespace:config.namespace,serviceId:metadata.serviceId,keys,mockEnabled:true,website:true,a1Runtime:true,...(selection?.a1RuntimeFix?{a1RuntimeFix:true}:{}),...(selection?.a1RuntimeDiagnostic?{a1RuntimeDiagnostic:true}:{}),...(selection?.a1HeartbeatBusy?{a1HeartbeatBusy:true}:{}),nativeSocial:true,tls,canonicalOrigin:config.origin,compatibility:metadata.compatibility}};
}
