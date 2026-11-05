import {prepareNativeA1RuntimeFixLanLaunch} from '../src/g3/native-a1-runtime-fix-lan.js';
import {createG3CandidateApp} from '../src/g3/candidate.js';
const args=process.argv.slice(2),check=args.includes('--check-only'),configPath=args.find(p=>p!=='--check-only');
if(!configPath||args.length!==(check?2:1))throw Error('Usage: g3-native-a1-runtime-fix-lan <exact host configuration.json> [--check-only]');
const prepared=await prepareNativeA1RuntimeFixLanLaunch(configPath);
if(check)console.log(JSON.stringify({preflight:true,serviceStarted:false,databaseTouched:false,origin:prepared.config.origin,namespace:prepared.config.namespace,serviceId:prepared.config.serviceId,a1Runtime:true,a1RuntimeFix:true,runtimeApproval:prepared.metadata.runtimeApproval,sourceReview:prepared.metadata.desktopApproval,capabilities:prepared.metadata.compatibility.capabilities,sourceApproved:true,developmentOnly:true,installApproved:false,productionApproved:false,packaged:false,actualNoitaEvidence:false,finalRebuiltDesktopReviewRequired:true,tls:'Existing certificate/CA unchanged; normal hostname/root verification required'}));
else{
 const service=await createG3CandidateApp(prepared.options);try{await service.app.listen({host:prepared.config.ip,port:prepared.config.port});}catch(e){await service.app.close();throw e;}
 for(const signal of ['SIGINT','SIGTERM'] as const)process.once(signal,()=>{void service.app.close();});
 console.log('Owner-started A1 runtime-fix TEST '+prepared.config.origin+'; namespace='+prepared.config.namespace+'; matching app review/package/owner game acceptance remain separate.');
}
