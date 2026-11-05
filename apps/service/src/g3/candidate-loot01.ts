import {raceCompatibility} from '../../../../packages/domain/src/g3-race-config.js';
import {RaceService} from './race.js';
import {registerRaceRoutes} from './race-routes.js';
import {parseJson as parseRace} from '../../../../packages/contracts/src/g3-race.js';
import {LobbyStatistics,registerStatisticsRoutes} from './stats.js';
import {parseJson as parseStatistics} from '../../../../packages/contracts/src/g3-stats.js';
import {testingLoadoutCompatibility} from '../../../../packages/domain/src/testing-starting-loadout.js';
import {C2Authority} from './c2.js';
import {registerC2Routes} from './c2-app.js';
import {parseJson as parseC2} from '../../../../packages/contracts/src/g3-c2.js';
import {C1Authority} from './c1.js';
import {registerC1Routes} from './c1-app.js';
import {parseJson as parseC1} from '../../../../packages/contracts/src/g3-c1.js';
import {ServerDebugDiagnostics,diagnosticEnabled,diagnosticServiceTag,type DebugOptions} from './debug-diagnostics.js';
import {verifyLifecycleFixtureRuntime} from '../rejoin/lifecycle-fixture-profile.js';
import {registerLifecycleRoutes} from '../rejoin/lifecycle-routes.js';
import {parseJson as parseLifecycle} from '../../../../packages/contracts/src/g3-lifecycle.js';
import {parseJson as parseWaiting} from '../../../../packages/contracts/src/g3-waiting.js';
import {connectionReply} from './connection.js';
import {registerConnectionRoutes} from './connection-routes.js';
import {parseJson as parseConnection} from '../../../../packages/contracts/src/g3-connection.js';
import {readRoomAdmissions} from './admission.js';
import type {Tx} from '../g1/database.js';
import {verifyRejoinPackage} from '../rejoin/package-profile.js';
import {verifyApprovedNormalPackage,type NormalPackageApproval} from '../rejoin/approved-package.js';
import {verifyRejoinCompositionInput,type ArtifactPin} from '../rejoin/composition-profile.js';
import {rejoinFixtureProfile} from '../rejoin/fixture-profile.js';
import {RejoinDatabase} from '../rejoin/database.js';
import {RejoinAuth} from '../rejoin/auth.js';
import {RejoinRooms} from '../rejoin/rooms.js';
import {registerRejoinRoutes} from '../rejoin/routes.js';
import {parseJson as parseRejoin} from '../../../../packages/contracts/src/g3-rejoin.js';
import {verifyNativeLoot01Artifacts} from './native-loot01-profile.js';
import {verifyNativePotionCorrectionArtifacts} from './native-potion-correction-profile.js';
import {verifyNativePotionDiagnosticArtifacts} from './native-potion-diagnostic-profile.js';
import {verifyNativeC2DevelopmentArtifacts} from './native-c2-profile.js';
import {LiveService} from './live-service.js';
import {HourglassCandidateDatabase} from './hourglass-database.js';
import {HourglassAuthority} from './hourglass.js';
import {registerHourglassRoutes} from './hourglass-app.js';
import {parseJson as parseHourglass} from '../../../../packages/contracts/src/g3-hourglass-v2.js';
import {createCandidateApp,type CandidateOptions} from '../g2-next/app.js';
import {parseJson as parseLobby} from '../../../../packages/contracts/src/g2-next.js';
import {parseJson as parseGameplay} from '../../../../packages/contracts/src/g3.js';
import {G3CandidateDatabase,G3CandidateRooms,synchronizeGameplay} from './lifecycle.js';
import {G3Authority} from './authority.js';
import {registerG3Routes} from './app.js';
import {demand,fingerprint} from '../g1/security.js';
import {readFile} from 'node:fs/promises';
import {parseJson as parseSocial} from '../../../../packages/contracts/src/g3-social.js';
import {SocialCandidateDatabase} from './social-database.js';
import {SocialAuthority} from './social.js';
import {registerSocialRoutes} from './social-app.js';
import {verifyNativeDevelopmentArtifacts} from './native-profile.js';
import {verifyNativeA1DevelopmentArtifacts} from './native-a1-profile.js';
import {verifyNativeA1RuntimeFixDevelopmentArtifacts} from './native-a1-runtime-fix-profile.js';
import {verifyNativeA1DiagnosticDevelopmentArtifacts} from './native-a1-diagnostic-profile.js';
import {verifyNativeA1HeartbeatBusyDevelopmentArtifacts} from './native-a1-heartbeat-busy-profile.js';
import {verifyNativeSocialDevelopmentArtifacts,nativeSocialRuntimeResource,nativeSocialDesktopResource} from './native-social-profile.js';
import {parseJson as parseHistory} from '../../../../packages/contracts/src/g3-history.js';
import {registerHistoryRoutes} from './history.js';
import {NemesisCandidateDatabase,NemesisCandidateRooms} from './nemesis-database.js';
import {NemesisAuthority} from './nemesis.js';
import {registerNemesisRoutes} from './nemesis-app.js';
import {parseJson as parseNemesis} from '../../../../packages/contracts/src/g3-nemesis.js';

export async function createG3CandidateApp(options:CandidateOptions&{rejoinPackageApproval?:NormalPackageApproval;worldRules?:boolean;itemTransfers?:boolean;b1Gold?:boolean;debugDiagnostics?:boolean;debugDiagnosticOptions?:DebugOptions;rejoinPackageVerifier?:typeof verifyRejoinPackage;rejoinFixtureRuntime?:ArtifactPin;rejoinPackage?:boolean;rejoinInput?:ArtifactPin;rejoin?:boolean;loot01Runtime?:boolean;potionCorrection?:boolean;potionDiagnostic?:boolean;c2Runtime?:boolean;live?:boolean;social?:boolean;hourglass?:boolean;nemesis?:boolean;native?:boolean;nativeSocial?:boolean;a1Runtime?:boolean;a1RuntimeFix?:boolean;a1RuntimeDiagnostic?:boolean;a1HeartbeatBusy?:boolean;nativeArtifactReader?:Parameters<typeof verifyNativeDevelopmentArtifacts>[0]}){
 // Internal normal-metadata regression seam: never selectable by a LAN config.
 if(options.itemTransfers)demand(options.b1Gold,'FORBIDDEN');
 if(options.b1Gold){const target=new URL(options.databaseUrl);demand(options.rejoin&&!options.rejoinPackage&&/^g3_rejoin_[a-f0-9]{32}$/.test(options.namespace??'')&&target.hostname==='127.0.0.1'&&target.port==='54328'&&target.pathname==='/nt2_g1','FORBIDDEN');}
 if(options.rejoinPackageVerifier){const target=new URL(options.databaseUrl);demand(options.rejoinPackage&&options.rejoin&&!options.rejoinInput&&!options.rejoinFixtureRuntime&&/^g3_rejoin_[a-f0-9]{32}$/.test(options.namespace??'')&&target.hostname==='127.0.0.1'&&target.port==='54328'&&target.pathname==='/nt2_g1','FORBIDDEN');}
 if(options.rejoinFixtureRuntime){const target=new URL(options.databaseUrl);demand(options.rejoin&&!options.rejoinPackage&&!options.rejoinInput&&/^g3_rejoin_[a-f0-9]{32}$/.test(options.namespace??'')&&target.hostname==='127.0.0.1'&&target.port==='54328'&&target.pathname==='/nt2_g1','FORBIDDEN');}
 demand(!options.rejoinPackageApproval||options.authentication&&!options.mockEnabled&&options.rejoinPackage&&!options.rejoinPackageVerifier,'FORBIDDEN');
 demand(!options.rejoinPackage||options.rejoin&&!options.rejoinInput,'FORBIDDEN');
 demand(!options.rejoin||options.loot01Runtime&&options.namespace?.startsWith('g3_rejoin_'),'FORBIDDEN');
 const c2Selected=Boolean(options.loot01Runtime||options.c2Runtime||options.potionDiagnostic||options.potionCorrection);demand([options.loot01Runtime,options.c2Runtime,options.potionDiagnostic,options.potionCorrection].filter(Boolean).length<=1,'FORBIDDEN');
 // Native development is a separate exact reviewed profile, verified before DB work.
 demand(!options.profile&&options.namespace?.startsWith('g3_'),'FORBIDDEN');
 demand(!options.native||!options.social&&!options.hourglass,'FORBIDDEN');
 demand(!options.hourglass||!options.social,'FORBIDDEN');
 demand(!options.nemesis||!options.native&&!options.social&&!options.hourglass,'FORBIDDEN');
 demand(!options.nativeSocial||!options.native&&!options.social&&!options.hourglass&&!options.nemesis&&options.tls&&options.namespace?.startsWith('g3_social_'),'FORBIDDEN');
 demand(!options.a1Runtime||Boolean(options.native||options.nativeSocial)&&Boolean(options.tls),'FORBIDDEN');
 demand(!options.a1RuntimeFix||options.a1Runtime,'FORBIDDEN');
 demand(!options.a1RuntimeDiagnostic||options.a1Runtime&&!options.a1RuntimeFix,'FORBIDDEN');
 demand(!options.a1HeartbeatBusy||options.a1Runtime&&!options.a1RuntimeFix&&!options.a1RuntimeDiagnostic,'FORBIDDEN');
 demand(!c2Selected||options.live===true&&Boolean(options.tls)&&!options.native&&!options.nativeSocial&&!options.social&&!options.hourglass&&!options.nemesis&&!options.a1Runtime&&!options.a1RuntimeFix&&!options.a1RuntimeDiagnostic&&!options.a1HeartbeatBusy,'FORBIDDEN');
 const socialEnabled=options.social||options.nativeSocial||c2Selected;
 const metadata=options.rejoinPackage?options.rejoinPackageApproval?await verifyApprovedNormalPackage(options.rejoinPackageApproval):await (options.rejoinPackageVerifier??verifyRejoinPackage)():options.rejoin?options.rejoinFixtureRuntime?await verifyLifecycleFixtureRuntime(options.rejoinFixtureRuntime):options.rejoinInput?await verifyRejoinCompositionInput(options.rejoinInput,options.nativeArtifactReader):await rejoinFixtureProfile(options.b1Gold===true,options.itemTransfers===true,options.worldRules===true):options.loot01Runtime?await verifyNativeLoot01Artifacts(options.nativeArtifactReader):options.potionCorrection?await verifyNativePotionCorrectionArtifacts(options.nativeArtifactReader):options.potionDiagnostic?await verifyNativePotionDiagnosticArtifacts(options.nativeArtifactReader):c2Selected?await verifyNativeC2DevelopmentArtifacts(options.nativeArtifactReader):options.a1HeartbeatBusy?await verifyNativeA1HeartbeatBusyDevelopmentArtifacts(Boolean(options.nativeSocial),options.nativeArtifactReader):options.a1RuntimeDiagnostic?await verifyNativeA1DiagnosticDevelopmentArtifacts(Boolean(options.nativeSocial),options.nativeArtifactReader):options.a1RuntimeFix?await verifyNativeA1RuntimeFixDevelopmentArtifacts(Boolean(options.nativeSocial),options.nativeArtifactReader):options.a1Runtime?await verifyNativeA1DevelopmentArtifacts(Boolean(options.nativeSocial),options.nativeArtifactReader):options.nativeSocial?await verifyNativeSocialDevelopmentArtifacts(options.nativeArtifactReader):options.native?await verifyNativeDevelopmentArtifacts(options.nativeArtifactReader):JSON.parse(await readFile(new URL('../../../../deploy/profiles/'+(options.nemesis?'g3-nemesis-simulated-candidate.json':options.hourglass?'g3-hourglass-simulated-candidate-2.json':options.social?'g3-social-simulated-candidate.json':'g3-simulated-candidate.json'),import.meta.url),'utf8'));
 demand(fingerprint(options.compatibility)===fingerprint(metadata.compatibility),'FORBIDDEN');
 if(options.rejoin)options={...options,compatibility:raceCompatibility(options.compatibility)};
 options={...options,compatibility:testingLoadoutCompatibility(options.compatibility,options.testingStartingLoadout===true)};
 demand(!(options.nativeSocial||options.a1Runtime)||options.serviceId===metadata.serviceId,'FORBIDDEN');
 demand(!c2Selected||options.serviceId===metadata.serviceId&&(options.rejoin||options.namespace?.startsWith(metadata.namespacePrefix)),'FORBIDDEN');
 demand(!options.a1Runtime||options.namespace?.startsWith(metadata.namespacePrefix),'FORBIDDEN');
 demand(!options.live||Boolean(options.tls)&&!options.nemesis&&!options.hourglass,'FORBIDDEN');
 let live:LiveService|undefined;let candidateDb:import('../g1/database.js').Database;let candidateAuth:import('../g2-next/auth.js').PrincipalAuth;
 const candidate=await createCandidateApp(options,{
  namespacePrefix:'g3_',
  infoProfile:options.nativeSocial||c2Selected?metadata.buildRegistry:options.native?metadata.profile:{id:metadata.id,evidenceStatus:'simulated-game-facts'},
  auth:options.rejoin?(db,security,o)=>new RejoinAuth(db,security,o.now,o.mockEnabled,o.providerAvailable):undefined,
  // Recovery commands/inspection check the owning scope inside the mutation.
  // Periodic bounded room housekeeping must not gate an unrelated RPC.
  requestHousekeeping:options.rejoin?async()=>{}:undefined,
  database:o=>new (options.rejoin?RejoinDatabase:options.nemesis?NemesisCandidateDatabase:options.hourglass?HourglassCandidateDatabase:socialEnabled?SocialCandidateDatabase:G3CandidateDatabase)(o.databaseUrl,o.serviceId,o.namespace,o.databaseBudget,true),
  rooms:(db,auth,o)=>new (options.rejoin?RejoinRooms:options.nemesis?NemesisCandidateRooms:G3CandidateRooms)(db,auth,o.compatibility),
  decorate:async(tx,input,value,token)=>connectionReply(tx,candidateDb!,candidateAuth!,options.compatibility,input,value,token),
  parseJson:(url,text)=>url==='/rpc/g3/race'?parseRace('Request',text):url==='/v3/stats/rpc'?parseStatistics('Request',text):options.rejoin&&url==='/v3/c2/rpc'?parseC2('Request',text):options.rejoin&&url==='/v3/c1/rpc'?parseC1('Request',text):options.rejoin&&url==='/v3/lifecycle/rpc'?parseLifecycle('Request',text):options.rejoin&&url==='/v3/waiting/rpc'?parseWaiting('Request',text):url==='/v3/connection/rpc'?parseConnection('Request',text):options.rejoin&&url==='/v3/rejoin/rpc'?parseRejoin('Request',text):options.nemesis&&url==='/v3/nemesis/rpc'?parseNemesis('Request',text):options.hourglass&&url==='/v3/hourglass/rpc'?parseHourglass('Request',text):url==='/v3/history/rpc'?parseHistory('Request',text):socialEnabled&&url==='/v3/social/rpc'?parseSocial('Request',text):url==='/v3/rpc'?parseGameplay('Request',text):parseLobby(url==='/v1/mock/browser-login'?'BrowserLogin':'Request',text),
  register:async(app,{db,auth,rooms,traffic,webToken})=>{
   candidateDb=db;candidateAuth=auth;await registerStatisticsRoutes(app,new LobbyStatistics(db,auth,options.compatibility),webToken);await registerConnectionRoutes(app,db,auth,options.compatibility);
   if(options.rejoin){const race=new RaceService(db,auth,options.compatibility,rooms as RejoinRooms);(rooms as RejoinRooms).race=race;await registerRaceRoutes(app,race,webToken);await registerRejoinRoutes(app,(rooms as RejoinRooms).recovery);await registerLifecycleRoutes(app,rooms as RejoinRooms);}
   if(options.live){const debugOptions={...options.debugDiagnosticOptions,serviceTag:diagnosticServiceTag(db.serviceId,db.epoch),...(options.debugDiagnostics!==undefined?{enabled:options.debugDiagnostics}:{})};const debug=new ServerDebugDiagnostics(metadata.id,await diagnosticEnabled(debugOptions),auth.now,debugOptions);live=new LiveService(db,auth,rooms,traffic,debug);await live.register(app);}
   const social=socialEnabled?new SocialAuthority(()=>db.epoch,options.compatibility.capabilities,options.now,options.rejoin===true):undefined;
   const authority=new G3Authority(()=>db.epoch,options.now,social?(tx,actor,scope)=>social.recordDeath(tx,actor,scope):undefined);
   const admission=(tx:Tx,id:string)=>readRoomAdmissions(tx,db,options.compatibility,auth.now(),id);authority.admission=admission;if(social)social.admission=admission;
   if(social&&options.rejoin&&['core.world-rules','core.hourglass'].every(c=>options.compatibility.capabilities.includes(c)))await registerC2Routes(app,db,auth,new C2Authority(db,admission,auth.now,(id,tx)=>traffic.authorizedRoom(id,tx)));
   if(social&&options.rejoin)await registerC1Routes(app,db,auth,new C1Authority(db,admission,auth.now,(id,tx)=>traffic.authorizedRoom(id,tx),(tx,room,sourceWorld)=>(rooms as RejoinRooms).endForPlayerDeath(tx,room,sourceWorld)));
   authority.roomBudget=(id,tx)=>traffic.authorizedRoom(id,tx);if(social)social.roomBudget=(id,tx)=>traffic.authorizedRoom(id,tx);
   // Rejoin already uses scoped canonical checks and timer-driven housekeeping
   // for its main RPC. Optional gameplay/social reads must not add a global
   // discovery + per-room sweep ahead of every authenticated transaction.
   const requestSweep=options.rejoin?async()=>{}:()=>rooms.sweep();
   await registerG3Routes(app,{db,auth,authority,sweep:requestSweep,beforeExecute:(tx,input)=>synchronizeGameplay(tx,db.epoch,options.compatibility.capabilities,auth.now(),{db,compatibility:options.compatibility,roomId:input.scope?.roomId})});
   await registerHistoryRoutes(app,{db,auth});
   if(social)await registerSocialRoutes(app,{db,auth,authority:social,sweep:requestSweep,...((options.nativeSocial||c2Selected)?{developmentRuntime:{runtimeProfile:metadata.profile,runtimeApproval:options.a1Runtime||c2Selected?metadata.runtimeApproval:nativeSocialRuntimeResource,desktopApproval:options.a1Runtime||c2Selected?metadata.desktopApproval:nativeSocialDesktopResource,sourceApproved:options.rejoin?!!options.rejoinInput||!!options.rejoinPackage||!!options.rejoinFixtureRuntime:true,developmentOnly:true,installApproved:false,packaged:false,actualNoitaEvidence:false}}:{})});
   if(options.nativeSocial||c2Selected){const body=Buffer.from(metadata.runtimeResourceJson);app.get('/v3/social/runtime',async(_req,reply)=>reply.type('application/json').header('cache-control','no-store').send(body));}
   if(options.hourglass)await registerHourglassRoutes(app,{db,auth,authority:new HourglassAuthority(()=>db.epoch,options.compatibility.capabilities,options.now),sweep:()=>rooms.sweep()});
   if(options.nemesis)await registerNemesisRoutes(app,{db,auth,authority:new NemesisAuthority(()=>db.epoch,options.compatibility.capabilities,options.now),sweep:()=>rooms.sweep()});
   if(options.website){
    for(const [path,file,type] of [['/statistics','index.html','text/html'],['/g3/page.js','page.js','text/javascript'],['/g3/client.js','client.js','text/javascript'],['/g3/statistics.js','statistics.js','text/javascript']] as const){
     const body=await readFile(new URL('../../../../apps/web/g3/'+file,import.meta.url));
     app.get(path,async(_req,reply)=>reply.type(type+'; charset=utf-8').headers({'content-security-policy':"default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'",'x-content-type-options':'nosniff','cache-control':'no-store','referrer-policy':'no-referrer'}).send(body));
    }
    // Relative ES module imports resolve through this alias to the existing browser client.
    const client=await readFile(new URL('../../../../apps/web/candidate/client.js',import.meta.url));
    app.get('/candidate/client.js',async(_req,reply)=>reply.type('text/javascript; charset=utf-8').header('cache-control','no-store').header('x-content-type-options','nosniff').send(client));
   }
  }
 });
 return {...candidate,live};
}



