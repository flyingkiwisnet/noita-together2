import {persistentRejoinIdentity} from '../src/rejoin/host-identity.js';
import {prepareNativeRejoinLanLaunch} from '../src/rejoin/lan.js';
import {createG3CandidateApp} from '../src/g3/candidate-loot01.js';
const args=process.argv.slice(2),check=args.includes('--check-only'),configPath=args.find(p=>p!=='--check-only');
if(!configPath||args.length!==(check?2:1))throw Error('Usage: g3-rejoin-lan <exact host configuration.json> [--check-only]');
let stage=performance.now();const timed=(name:string)=>{console.log('Host '+name+': '+Math.round(performance.now()-stage)+'ms');stage=performance.now();};
const prepared=await prepareNativeRejoinLanLaunch(configPath);
timed('TLS/port preflight');
if(check)console.log(JSON.stringify({preflight:true,serviceStarted:false,databaseTouched:false,origin:prepared.config.origin,namespace:prepared.config.namespace,serviceId:prepared.config.serviceId,rejoin:true,persistentDevelopmentHost:true,live:true,runtimeApproval:prepared.metadata.runtimeApproval,sourceDependency:prepared.metadata.desktopApproval,developmentOnly:true,installApproved:false,actualNoitaEvidence:false}));
else{
 const keys=await persistentRejoinIdentity(prepared.options.databaseUrl,prepared.config.serviceId,prepared.config.namespace,prepared.config.issuer);timed('persistent identity');
 const service=await createG3CandidateApp({...prepared.options,keys});timed('migrations and recovery');try{await service.app.listen({host:prepared.config.ip,port:prepared.config.port});}catch(e){await service.app.close();throw e;}timed('listen');
 for(const signal of ['SIGINT','SIGTERM'] as const)process.once(signal,()=>{void service.app.close();});
 console.log('Owner-started Rejoin DEV host '+prepared.config.origin+'; namespace='+prepared.config.namespace);
}
