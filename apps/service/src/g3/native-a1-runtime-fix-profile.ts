import {verifyNativeA1DevelopmentArtifacts} from './native-a1-profile.js';
export const nativeA1RuntimeFixResources={
 primary:{path:'deploy/profiles/g3-native-runtime-approval.record4-a1-runtime-fix.json',sha256:'301b1a8bd0d88804303ea7d9a0d199c71a014ae38fe6c8528704b862a76f1eb8'},
 social:{path:'deploy/profiles/g3-native-social-runtime-approval.record3-a1-runtime-fix.json',sha256:'fe726f360170d2d738b91a5c95d0f787a5df87f0759b368d4c8d93dd193f0077'}
} as const;
export const nativeA1RuntimeFixSourceReviewResource={path:'deploy/profiles/g3-native-a1-source-review.record20.json',sha256:'4c84e37e56c7bd5b87df07749996083539fa04277e3dd5684782a0453b2114f7'} as const;
// Same proven closure/policy verifier; a separate explicit resource selection.
export function verifyNativeA1RuntimeFixDevelopmentArtifacts(social=false,reader?:Parameters<typeof verifyNativeA1DevelopmentArtifacts>[1]){
 return verifyNativeA1DevelopmentArtifacts(social,reader,{resources:nativeA1RuntimeFixResources,sourceReview:nativeA1RuntimeFixSourceReviewResource});
}
