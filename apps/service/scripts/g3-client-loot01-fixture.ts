import {writeFile,rm} from 'node:fs/promises';
import {join} from 'node:path';
import {createInterface} from 'node:readline';
import {startG3Fixture,provisionG3Fixture,privateG3Directory} from './g3-loot01-fixture.js';

const raw=process.argv.slice(2),policyIndex=raw.indexOf('--seed-policy'),seedPolicy=policyIndex>=0?raw[policyIndex+1]:undefined;if(policyIndex>=0&&!['shared-fixed','shared-random','individual-random'].includes(seedPolicy??''))throw Error('Invalid fixture seed policy');const args=raw.filter((_v,i)=>i!==policyIndex&&i!==policyIndex+1||policyIndex<0);if((seedPolicy||args.includes('--loot-off'))&&!args.includes('--loot01-runtime'))throw Error('Policy selector requires exact seed/loot runtime');
if(args.some(x=>!['--loot01-runtime','--loot-off','--potion-correction','--potion-diagnostic','--c2-runtime','--live','--tls','--playing','--keepalive','--social','--hourglass','--nemesis','--native','--native-social','--a1-runtime','--a1-runtime-fix','--a1-runtime-diagnostic','--a1-heartbeat-busy'].includes(x)))throw Error('Usage: g3:fixture [--loot01-runtime requires --live --tls; --seed-policy shared-fixed|shared-random|individual-random; --loot-off] [--potion-correction requires --live --tls, excludes other profiles] [--potion-diagnostic requires --live --tls, excludes regular C2 and other profiles] [--c2-runtime requires --live --tls and excludes other profiles] [--live requires --tls] [--tls] [--playing] [--keepalive] [--social | --hourglass | --nemesis | --native | --native-social] [--a1-runtime requires --tls and --native or --native-social] [--a1-runtime-fix | --a1-runtime-diagnostic | --a1-heartbeat-busy requires --a1-runtime]');
const fixture=await startG3Fixture({loot01Runtime:args.includes('--loot01-runtime'),potionCorrection:args.includes('--potion-correction'),potionDiagnostic:args.includes('--potion-diagnostic'),c2Runtime:args.includes('--c2-runtime'),live:args.includes('--live'),tls:args.includes('--tls'),social:args.includes('--social'),hourglass:args.includes('--hourglass'),nemesis:args.includes('--nemesis'),native:args.includes('--native'),nativeSocial:args.includes('--native-social'),a1Runtime:args.includes('--a1-runtime'),a1RuntimeFix:args.includes('--a1-runtime-fix'),a1RuntimeDiagnostic:args.includes('--a1-runtime-diagnostic'),a1HeartbeatBusy:args.includes('--a1-heartbeat-busy')});
let directory:string|undefined;
try{
 const state=await provisionG3Fixture(fixture,{...(seedPolicy?{seedPolicy}:{}),...(args.includes('--loot-off')?{randomizeLoot:false}:{})});
 if(args.includes('--playing')){await state.start();await state.playing();}
 directory=await privateG3Directory('nt2-g3-client-');
 const bundlePath=join(directory,'credentials.json');
 const hourglass=args.includes('--hourglass')?JSON.parse((await fixture.call('/v3/hourglass/info')).body):undefined;
 const nemesis=args.includes('--nemesis')?JSON.parse((await fixture.call('/v3/nemesis/info')).body):undefined;
 const social=args.includes('--loot01-runtime')||args.includes('--social')||args.includes('--native-social')||args.includes('--c2-runtime')||args.includes('--potion-diagnostic')||args.includes('--potion-correction')?JSON.parse((await fixture.call('/v3/social/info')).body):undefined;
 const nativeSocial=args.includes('--loot01-runtime')||args.includes('--native-social')||args.includes('--c2-runtime')||args.includes('--potion-diagnostic')||args.includes('--potion-correction')?{nativeSocialDevelopmentProfile:{id:fixture.profile.id,profile:fixture.profile.profile,compatibility:fixture.profile.compatibility,runtimeApproval:fixture.profile.runtimeApproval,desktopApproval:fixture.profile.desktopApproval,requiredLocalFiles:fixture.profile.requiredLocalFiles,sourceApproved:true,developmentOnly:true,installApproved:false,packaged:false,actualNoitaEvidence:false}}:{};
 await writeFile(bundlePath,JSON.stringify({evidenceStatus:'simulated-game-facts',...(args.includes('--loot01-runtime')?{loot01Runtime:true,canonicalPolicyRevision:'g3-seed-loot-1'}:{}),...(args.includes('--potion-correction')?{potionCorrection:true,correctionCandidate:true,diagnosticOnly:false,gameplaySemanticsChanged:true}:{}),...(args.includes('--potion-diagnostic')?{potionDiagnostic:true,diagnosticOnly:true}:{}),...(args.includes('--c2-runtime')?{c2Runtime:true}:{}),...(args.includes('--live')?{live:{endpoint:'/v3/live',subprotocol:'nt2.g3.live.v1',revision:'g3-live-draft-1',encoding:'protobuf-binary'}}:{}),...(args.includes('--a1-runtime')?{a1Runtime:true}:{}),...(args.includes('--a1-runtime-fix')?{a1RuntimeFix:true}:{}),...(args.includes('--a1-runtime-diagnostic')?{a1RuntimeDiagnostic:true}:{}),...(args.includes('--a1-heartbeat-busy')?{a1HeartbeatBusy:true}:{}),...(args.includes('--native')?{nativeDevelopmentProfile:fixture.profile}:{}),...nativeSocial,origin:fixture.origin,namespace:fixture.namespace,caPath:fixture.caPath,serviceId:fixture.service.db.serviceId,serviceEpoch:fixture.service.db.epoch,lobbyRevision:'g2-draft-5',gameplayRevision:'g3-draft-1',schemaSha256:'325f01bafaa493a7ff6614d3675c9908ed3f2d21c757e092e283838973fad1ff',...(social?{social}:{}),...(hourglass?{hourglass}:{}),...(nemesis?{nemesis}:{}),host:state.host,desktops:state.desktops,games:state.games,scopes:state.scopes,room:state.room},null,2),{mode:0o600,flag:'wx'});
 console.log(JSON.stringify({origin:fixture.origin,namespace:fixture.namespace,caPath:fixture.caPath,bundlePath,state:args.includes('--playing')?'simulated-playing':'protected-inspected',simulatedKeepalive:args.includes('--keepalive'),expiresInSeconds:600}));
 console.log('Disposable simulated fixture only. Commands: start, playing, end, quit. Credentials are in the temporary bundle; do not commit or log them.');
 const input=createInterface({input:process.stdin,output:process.stdout});
 let stop!:()=>void;const done=new Promise<void>(resolve=>{stop=resolve;});
 const deadline=setTimeout(stop,600000);deadline.unref();
 for(const signal of ['SIGINT','SIGTERM'] as const)process.once(signal,stop);
 input.on('close',stop);
 // Serialize lifecycle commands; failure does not silently advance the fixture.
 let commands=Promise.resolve(),pulsing=false;
 const keepalive=args.includes('--keepalive')?setInterval(()=>{if(pulsing)return;pulsing=true;commands=commands.then(()=>state.pulse()).catch(()=>{console.log('Simulated keepalive rejected; fixture stopping.');stop();}).finally(()=>{pulsing=false;});},10000):undefined;
 input.on('line',line=>{commands=commands.then(async()=>{
  switch(line.trim()){
   case 'start':await state.start();console.log('Selected inspected Start accepted; games still protected.');break;
   case 'playing':await state.playing();console.log('Simulated playing observations accepted.');break;
   case 'end':await state.rpc(state.host,{endRun:{}});if(keepalive)clearInterval(keepalive);console.log('Run ended; original status recovery only.');break;
   case 'quit':stop();break;
   default:console.log('Commands: start, playing, end, quit.');
  }
 }).catch(()=>console.log('Lifecycle command rejected; inspect client/service responses.'));});
 await done;input.close();clearTimeout(deadline);if(keepalive)clearInterval(keepalive);await commands;
}finally{try{await fixture.close();}finally{if(directory)await rm(directory,{recursive:true,force:true});}}
