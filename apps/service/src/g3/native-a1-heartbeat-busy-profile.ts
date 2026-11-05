import {verifyNativeA1DevelopmentArtifacts} from './native-a1-profile.js';
export const nativeA1HeartbeatBusyResources={
 primary:{path:'deploy/profiles/g3-native-runtime-approval.record6-a1-heartbeat-busy.json',sha256:'f43a9d00836d92f4e0b37c06066d7f85e95e24e792a099fa6c545d95788142c0'},
 social:{path:'deploy/profiles/g3-native-social-runtime-approval.record5-a1-heartbeat-busy.json',sha256:'758b597a0deace517b4891fe1c6460ed55696e736aa0c03123bc5f1139595217'}
} as const;
export const nativeA1HeartbeatBusySourceResource={path:'deploy/profiles/g3-native-a1-heartbeat-busy-dependencies.json',sha256:'316815ebb3d2fa9c86fd83234b5978d888f9b8dab1517b79e0f9c5bd89ccef05'} as const;
export function verifyNativeA1HeartbeatBusyDevelopmentArtifacts(social=false,reader?:Parameters<typeof verifyNativeA1DevelopmentArtifacts>[1]){
 return verifyNativeA1DevelopmentArtifacts(social,reader,{resources:nativeA1HeartbeatBusyResources,sourceReview:nativeA1HeartbeatBusySourceResource,localTest:true});
}
