import {verifyNativeA1DevelopmentArtifacts} from './native-a1-profile.js';
export const nativeA1DiagnosticResources={
 primary:{path:'deploy/profiles/g3-native-runtime-approval.record5-a1-diagnostic.json',sha256:'54ffd19ba95f4a278a59bdf306725415cbd806072361c7c3918991dbcfa6c042'},
 social:{path:'deploy/profiles/g3-native-social-runtime-approval.record4-a1-diagnostic.json',sha256:'be6fec66dae95a141b09bceff5f3b22565ca7647958495507bcd441f014dde0b'}
} as const;
export const nativeA1DiagnosticSourceResource={path:'deploy/profiles/g3-native-a1-source-review.record22.json',sha256:'15bbb5fa1b343cd13af52c374cde9670c443a33819d455a61a05c3ce4bb16feb'} as const;
export function verifyNativeA1DiagnosticDevelopmentArtifacts(social=false,reader?:Parameters<typeof verifyNativeA1DevelopmentArtifacts>[1]){
 return verifyNativeA1DevelopmentArtifacts(social,reader,{resources:nativeA1DiagnosticResources,sourceReview:nativeA1DiagnosticSourceResource,localTest:true});
}
