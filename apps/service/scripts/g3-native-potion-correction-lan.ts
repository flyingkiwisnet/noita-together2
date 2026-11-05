import {prepareNativePotionCorrectionLanLaunch} from '../src/g3/native-potion-correction-lan.js';
import {createG3CandidateApp} from '../src/g3/candidate.js';
const args=process.argv.slice(2),check=args.includes('--check-only'),configPath=args.find(p=>p!=='--check-only');
if(!configPath||args.length!==(check?2:1))throw Error('Usage: g3-native-potion-correction-lan <exact host configuration.json> [--check-only]');
const prepared=await prepareNativePotionCorrectionLanLaunch(configPath);
if(check)console.log(JSON.stringify({preflight:true,serviceStarted:false,databaseTouched:false,origin:prepared.config.origin,namespace:prepared.config.namespace,serviceId:prepared.config.serviceId,potionCorrection:true,live:true,runtimeApproval:prepared.metadata.runtimeApproval,sourceDependency:prepared.metadata.desktopApproval,developmentOnly:true,installApproved:false,actualNoitaEvidence:false}));
else{
 const service=await createG3CandidateApp(prepared.options);try{await service.app.listen({host:prepared.config.ip,port:prepared.config.port});}catch(e){await service.app.close();throw e;}
 for(const signal of ['SIGINT','SIGTERM'] as const)process.once(signal,()=>{void service.app.close();});
 console.log('Owner-started Potion correction TEST '+prepared.config.origin+'; namespace='+prepared.config.namespace);
}
