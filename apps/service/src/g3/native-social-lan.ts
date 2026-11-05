// Manual owner-requested LAN TEST launch only. Preparing it starts no app/DB work.
import {readFile} from 'node:fs/promises';import {dirname,resolve} from 'node:path';import {createHash,generateKeyPairSync,randomBytes} from 'node:crypto';import {createSecureContext} from 'node:tls';
import {lanConfig,validateLanConfig,localInterface,validateCertificate} from '../g2/lan.js';import {ensureUnusedPort} from '../g2-next/launch.js';
import {verifyNativeSocialDevelopmentArtifacts,nativeSocialRuntimeResource,nativeSocialDesktopResource} from './native-social-profile.js';
const root=new URL('../../../../',import.meta.url),sha=(b:Buffer)=>createHash('sha256').update(b).digest('hex');
export function nativeSocialLanConfig(ip:unknown,port:unknown){const base=lanConfig(ip,port);if([4351,4352,4355].includes(base.port))throw Error('G3 test port must be separate from existing installed service ports');return {...base,serviceId:'nt2-g3-native-social-development',namespace:'g3_social_lan_'+base.ip.replaceAll('.','_')+'_'+base.port+'_cp13'};}
export function validateNativeSocialLanConfig(value:unknown){
 if(!value||typeof value!=='object'||Array.isArray(value))throw Error('Invalid G3 native social LAN configuration');const v=value as Record<string,unknown>,base=nativeSocialLanConfig(v.ip,v.port),expected={...base,runtimeApprovalSha256:nativeSocialRuntimeResource.sha256,desktopReviewSha256:nativeSocialDesktopResource.sha256,tlsSourceConfig:v.tlsSourceConfig,databaseSecretConfig:v.databaseSecretConfig};
 if(Object.keys(v).sort().join()!==Object.keys(expected).sort().join()||Object.entries(expected).some(([k,x])=>v[k]!==x)||typeof v.tlsSourceConfig!=='string'||!v.tlsSourceConfig||typeof v.databaseSecretConfig!=='string'||!v.databaseSecretConfig)throw Error('G3 LAN origin/identity/namespace/exact source profile and local secret paths must match');return {...expected,tlsSourceConfig:v.tlsSourceConfig,databaseSecretConfig:v.databaseSecretConfig};
}
export async function prepareNativeSocialLanLaunch(configPath:string){
 if(process.env.NODE_ENV==='production')throw Error('G3 manual test launcher refuses production');const config=validateNativeSocialLanConfig(JSON.parse(await readFile(configPath,'utf8')));localInterface(config.ip);
 const metadata=await verifyNativeSocialDevelopmentArtifacts();
 // Refuse before DB work; do not kill/stop whichever process owns this address.
 try{await ensureUnusedPort(config.ip,config.port);}catch{throw Error('G3 test port is occupied or unavailable; existing listener unchanged; no application or database was started');}
 const sourcePath=resolve(dirname(resolve(configPath)),config.tlsSourceConfig),source=validateLanConfig(JSON.parse(await readFile(sourcePath,'utf8')));
 if(source.ip!==config.ip||source.namespace===config.namespace||source.serviceId===config.serviceId||source.port===config.port)throw Error('Reuse only the separate existing same-IP public certificate source');
 const publicFolder=new URL('deploy/g2-lan/public/',root),bundle=JSON.parse(await readFile(new URL('bundle.json',publicFolder),'utf8')),ca=await readFile(new URL('ca.cer',publicFolder));
 if(bundle.origin!==source.origin||bundle.serviceId!==source.serviceId||sha(ca)!==bundle.caSha256||bundle.caSha256!=='730917075cc240a6c9eecdc0d05b1b4d837b79fab325d796fda7f804c9dd6d22')throw Error('Existing public TLS bundle/source/CA differs');
 validateCertificate(await readFile(new URL('server.cer',publicFolder)),ca,config.ip);
 const directory=dirname(sourcePath),tlsSecret=JSON.parse(await readFile(resolve(directory,'tls.json'),'utf8'));if(typeof tlsSecret.passphrase!=='string'||tlsSecret.passphrase.length<32)throw Error('Invalid local TLS source');
 const tls={pfx:await readFile(resolve(directory,'server.pfx')),passphrase:tlsSecret.passphrase};createSecureContext(tls);
 const databasePath=resolve(dirname(resolve(configPath)),config.databaseSecretConfig),databaseSecret=JSON.parse(await readFile(databasePath,'utf8'));if(typeof databaseSecret.databaseUrl!=='string')throw Error('Missing LOCAL database connection configuration');
 let databaseUrl:URL;try{databaseUrl=new URL(databaseSecret.databaseUrl);}catch{throw Error('Invalid LOCAL database connection configuration');}if(!['postgres:','postgresql:'].includes(databaseUrl.protocol)||!['127.0.0.1','localhost','[::1]'].includes(databaseUrl.hostname)||databaseUrl.search||databaseUrl.hash||!databaseUrl.pathname||!databaseUrl.username)throw Error('Manual G3 test uses existing local PostgreSQL only');
 const pair=generateKeyPairSync('ed25519'),keys={privateKey:pair.privateKey.export({format:'pem',type:'pkcs8'}).toString(),publicKey:pair.publicKey.export({format:'pem',type:'spki'}).toString(),recoveryKey:randomBytes(32).toString('base64url'),issuer:config.issuer};
 return {config,metadata,options:{databaseUrl:databaseSecret.databaseUrl,namespace:config.namespace,serviceId:metadata.serviceId,keys,mockEnabled:true,website:true,nativeSocial:true,tls,canonicalOrigin:config.origin,compatibility:metadata.compatibility}};
}
