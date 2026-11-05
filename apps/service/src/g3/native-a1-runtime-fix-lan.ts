import {nativeA1LanConfig,prepareNativeA1LanLaunch} from './native-a1-lan.js';
import {verifyNativeA1RuntimeFixDevelopmentArtifacts,nativeA1RuntimeFixResources,nativeA1RuntimeFixSourceReviewResource} from './native-a1-runtime-fix-profile.js';
export function validateNativeA1RuntimeFixLanConfig(value:unknown){
 if(!value||typeof value!=='object'||Array.isArray(value))throw Error('Invalid A1 runtime-fix LAN configuration');
 const v=value as Record<string,unknown>,base=nativeA1LanConfig(v.ip,v.port);
 const expected={...base,a1Runtime:true,a1RuntimeFix:true,runtimeApprovalSha256:nativeA1RuntimeFixResources.social.sha256,sourceReviewSha256:nativeA1RuntimeFixSourceReviewResource.sha256,tlsSourceConfig:'../../secrets/g2-lan-192-168-0-104-4352/config.json',databaseSecretConfig:'../../secrets/g3-native-social-lan/database.json'};
 if(Object.keys(v).sort().join()!==Object.keys(expected).sort().join()||Object.entries(expected).some(([k,x])=>v[k]!==x))throw Error('A1 runtime-fix exact identity/namespace/resource/TLS references required');
 return expected;
}
export function prepareNativeA1RuntimeFixLanLaunch(configPath:string){
 return prepareNativeA1LanLaunch(configPath,{validateConfig:validateNativeA1RuntimeFixLanConfig,verifyMetadata:()=>verifyNativeA1RuntimeFixDevelopmentArtifacts(true),a1RuntimeFix:true});
}
