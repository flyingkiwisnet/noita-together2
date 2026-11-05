// RJ3 persistent DEV LAN preflight; no DB/app/key creation here.
// Reuses the proven LAN origin/certificate/port checks, and starts no DB/app.
import {readFile} from 'node:fs/promises';import {dirname,resolve} from 'node:path';import {createHash} from 'node:crypto';import {createSecureContext} from 'node:tls';
import {lanConfig,validateLanConfig,localInterface,validateCertificate} from '../g2/lan.js';import {ensureUnusedPort} from '../g2-next/launch.js';
import {verifyRejoinPackage,rejoinRuntimePath,rejoinSourcePath} from './package-profile.js';
const nativeRejoinRuntimeResource={path:rejoinRuntimePath,sha256:'ac6ed13ccaf57c604d25c58e75bed5dd2ac5fec2b071f4f24cf7ebdc73f7852f'},nativeRejoinSourceResource={path:rejoinSourcePath,sha256:'9cf69c87b141274ce64dfaa9ebcd93006278e8301822200a423afe5d6328526c'};
const root=new URL('../../../../',import.meta.url),sha=(b:Buffer)=>createHash('sha256').update(b).digest('hex');
export function nativeRejoinLanConfig(ip:unknown,port:unknown){
 const base=lanConfig(ip,port);if(base.ip!=='192.168.0.104'||base.port!==4358)throw Error('Rejoin DEV test launcher requires assigned host192.168.0.104 and4358');
 return {...base,serviceId:'nt2-g3-c2-potion-correction',namespace:'g3_rejoin_lan_'+base.ip.replaceAll('.','_')+'_'+base.port};
}
export function validateNativeRejoinLanConfig(value:unknown){
 if(!value||typeof value!=='object'||Array.isArray(value))throw Error('Invalid Rejoin DEV LAN configuration');const v=value as Record<string,unknown>,base=nativeRejoinLanConfig(v.ip,v.port);
 const expected={...base,loot01Runtime:true,rejoin:true,rejoinPackage:true,persistentHostIdentity:'database',live:true,runtimeApprovalSha256:nativeRejoinRuntimeResource.sha256,sourceReviewSha256:nativeRejoinSourceResource.sha256,tlsSourceConfig:'../../secrets/g2-lan-192-168-0-104-4352/config.json',databaseSecretConfig:'../../secrets/g3-native-social-lan/database.json'};
 if(v.testingStartingLoadout!==undefined&&typeof v.testingStartingLoadout!=='boolean')throw Error('testingStartingLoadout must be boolean');
 if(v.debugDiagnostics!==undefined&&typeof v.debugDiagnostics!=='boolean')throw Error('debugDiagnostics must be boolean');
 if(Object.keys(v).filter(k=>!['debugDiagnostics','testingStartingLoadout'].includes(k)).sort().join()!==Object.keys(expected).sort().join()||Object.entries(expected).some(([k,x])=>v[k]!==x))throw Error('Rejoin DEV LAN identity/namespace/TLS paths/exact source resource must match; no caller capability grant');
 return {...expected,...(v.testingStartingLoadout!==undefined?{testingStartingLoadout:v.testingStartingLoadout as boolean}:{}),...(v.debugDiagnostics!==undefined?{debugDiagnostics:v.debugDiagnostics as boolean}:{})};
}
export async function prepareNativeRejoinLanLaunch(configPath:string,seams:{read?:typeof readFile;interfaceCheck?:typeof localInterface;verify?:typeof verifyRejoinPackage;portCheck?:typeof ensureUnusedPort;certificateCheck?:typeof validateCertificate;secureContext?:typeof createSecureContext}={}){
 const read=seams.read??readFile;
 if(process.env.NODE_ENV==='production')throw Error('Rejoin DEV manual test launcher refuses production');const config=validateNativeRejoinLanConfig(JSON.parse(await read(configPath,'utf8')));(seams.interfaceCheck??localInterface)(config.ip);
 const metadata=await (seams.verify??verifyRejoinPackage)();
 // Existing listeners stay untouched. This check opens no service or DB connection.
 try{await (seams.portCheck??ensureUnusedPort)(config.ip,config.port);}catch{throw Error('Rejoin DEV test port occupied/unavailable; existing listener unchanged; no app/database started');}
 const sourcePath=resolve(dirname(resolve(configPath)),config.tlsSourceConfig),source=validateLanConfig(JSON.parse(await read(sourcePath,'utf8')));
 if(source.ip!==config.ip||source.namespace===config.namespace||source.serviceId===config.serviceId||source.port===config.port)throw Error('Reuse only the separate existing same-IP certificate source');
 const publicFolder=new URL('deploy/g2-lan/public/',root),bundle=JSON.parse(await read(new URL('bundle.json',publicFolder),'utf8')),ca=await read(new URL('ca.cer',publicFolder));
 if(bundle.origin!==source.origin||bundle.serviceId!==source.serviceId||sha(ca)!==bundle.caSha256||bundle.caSha256!=='730917075cc240a6c9eecdc0d05b1b4d837b79fab325d796fda7f804c9dd6d22')throw Error('Existing public TLS bundle/source/CA differs');
 (seams.certificateCheck??validateCertificate)(await read(new URL('server.cer',publicFolder)),ca,config.ip);
 const directory=dirname(sourcePath),tlsSecret=JSON.parse(await read(resolve(directory,'tls.json'),'utf8'));if(typeof tlsSecret.passphrase!=='string'||tlsSecret.passphrase.length<32)throw Error('Invalid local TLS source');
 const tls={pfx:await read(resolve(directory,'server.pfx')),passphrase:tlsSecret.passphrase};(seams.secureContext??createSecureContext)(tls);
 const databasePath=resolve(dirname(resolve(configPath)),config.databaseSecretConfig),databaseSecret=JSON.parse(await read(databasePath,'utf8'));if(typeof databaseSecret.databaseUrl!=='string')throw Error('Missing LOCAL database configuration');
 let databaseUrl:URL;try{databaseUrl=new URL(databaseSecret.databaseUrl);}catch{throw Error('Invalid LOCAL database configuration');}if(!['postgres:','postgresql:'].includes(databaseUrl.protocol)||!['127.0.0.1','localhost','[::1]'].includes(databaseUrl.hostname)||databaseUrl.search||databaseUrl.hash||!databaseUrl.pathname||!databaseUrl.username)throw Error('Rejoin DEV test uses existing local PostgreSQL only');
 // Keys are loaded/created only on an explicit start via the new namespace DB; check-only never connects.
 return {config,metadata,options:{...(config.testingStartingLoadout!==undefined?{testingStartingLoadout:config.testingStartingLoadout}:{}),...(config.debugDiagnostics!==undefined?{debugDiagnostics:config.debugDiagnostics}:{}),databaseUrl:databaseSecret.databaseUrl,namespace:config.namespace,serviceId:metadata.serviceId,mockEnabled:true,website:true,loot01Runtime:true,rejoin:true,rejoinPackage:true,live:true,tls,canonicalOrigin:config.origin,compatibility:metadata.compatibility}};
}

