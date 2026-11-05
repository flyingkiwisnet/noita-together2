// Private isolated service support for the Client-owned normal-scheduler driver.
// stdout is sanitized control metadata; test credentials live only in its
// inherited private TLS fixture directory and are deleted on cooperative close.
import {startLifecycleLc1Fixture} from './lifecycle-lc1-fixture.js';
import {readFileSync} from 'node:fs';import {createInterface} from 'node:readline';import {writeFile} from 'node:fs/promises';import {dirname,join} from 'node:path';
const args=process.argv.slice(2),race=args[0]==='--race';if(race)args.shift();const raceConfiguration=args[0]==='--race-config'?JSON.parse(readFileSync(args[1]!,'utf8')):undefined;if(raceConfiguration){if(!race)throw Error('RACE_FLAG_REQUIRED');args.splice(0,2);}const itemTransfers=args[0]==='--items',b1Gold=itemTransfers||args[0]==='--b1-gold';if(b1Gold)args.shift();const sendGold=args[0]!=='--send-gold-off';if(!sendGold){if(!b1Gold)throw Error('Send Gold fixture flag requires B1');args.shift();}
if(args.length&&!(args.length===4&&args[0]==='--runtime'&&args[2]==='--runtime-sha256'&&/^[a-f0-9]{64}$/.test(args[3]!)))throw Error('Usage: node --import tsx lifecycle-lc1-fixture-host.ts [--items|--b1-gold [--send-gold-off]] [--runtime path --runtime-sha256 sha256]');
const runtime=args.length?{path:args[1]!,sha256:args[3]!}:undefined;
const x=await startLifecycleLc1Fixture({race,...(raceConfiguration?{raceConfiguration}:{}),...runtime?{runtime}:{},itemTransfers,b1Gold,sendGold}),ready=join(dirname(x.fixture.caPath!),'lc1-private-ready.json');
async function descriptor(){await writeFile(ready,JSON.stringify({revision:'g3-lifecycle-lc1-2',fixtureControlRevision:2,origin:x.fixture.origin,caPath:x.fixture.caPath,namespace:x.fixture.namespace,serviceId:x.fixture.service.db.serviceId,serviceEpoch:x.fixture.service.db.epoch,profile:x.fixture.profile,...(runtime?{fixtureRuntime:runtime}:{}),desktop:x.desktop,hostApp:x.hostApp??null,deviceId:x.devices[0],room:x.room,association:x.association,proof:x.proof,observation:x.observation,peer:x.peer??null})+'\n',{mode:0o600});}
const emit=(v:unknown)=>process.stdout.write(JSON.stringify(v)+'\n');await descriptor();emit({event:'ready',privateDescriptor:ready});
const lines=createInterface({input:process.stdin,crlfDelay:Infinity});
let closed=false;const close=async()=>{if(!closed){closed=true;lines.close();await x.close();}};
for(const signal of ['SIGINT','SIGTERM'] as const)process.once(signal,()=>{void close();});
try{for await(const line of lines){if(line.length>4096){emit({event:'error',code:'CONTROL_BOUNDS'});continue;}let command:any;try{command=JSON.parse(line);let result:any={};switch(command.op){
 case 'raceDropReply':if(Object.keys(command).some(k=>!['id','op','target'].includes(k)))throw Error('CONTROL_OPERATION');result=x.faults.raceControl.dropReply(command.target);break;
 case 'raceStatus':result=x.faults.raceControl.status();break;
 case 'd1DropReply':if(Object.keys(command).some(k=>!['id','op','target'].includes(k)))throw Error('CONTROL_OPERATION');result=x.faults.d1Control.dropReply(command.target);break;
 case 'd1Delay':if(Object.keys(command).some(k=>!['id','op','target','durationMs'].includes(k)))throw Error('CONTROL_OPERATION');result=x.faults.d1Control.delay(command.target,command.durationMs);break;
 case 'd1Status':if(Object.keys(command).some(k=>!['id','op'].includes(k)))throw Error('CONTROL_OPERATION');result=x.faults.d1Control.status();break;
 case 'start':result={...await x.start(command.selectedWorldIds===undefined&&command.excludedDesktopIds===undefined?undefined:{selectedWorldIds:command.selectedWorldIds,excludedDesktopIds:command.excludedDesktopIds},descriptor),startDiagnostic:x.startDiagnostics().at(-1)};break;
 case 'startReviewed':if(Object.keys(command).some(k=>!['id','op','selectedWorldIds','excludedDesktopIds'].includes(k)))throw Error('CONTROL_OPERATION');result={...await x.startReviewed(command.selectedWorldIds===undefined&&command.excludedDesktopIds===undefined?undefined:{selectedWorldIds:command.selectedWorldIds,excludedDesktopIds:command.excludedDesktopIds},descriptor),startDiagnostic:x.startDiagnostics().at(-1)};break;
 case 'startDiagnostics':if(Object.keys(command).some(k=>!['id','op'].includes(k)))throw Error('CONTROL_OPERATION');result={diagnostics:x.startDiagnostics()};break;
 case 'end':{const value=await x.end(),r=value.room;if(!r||r.state!=='ended')throw Error('FIXTURE_END_RESPONSE');await descriptor();result={ended:true,room:{roomId:r.roomId,runId:r.runId,runEpoch:r.runEpoch,state:r.state,configRevision:r.configRevision},runId:r.runId,hostActionDiagnostic:x.hostActionDiagnostics()};break;}
 case 'hostApp':if(Object.keys(command).some(k=>!['id','op'].includes(k)))throw Error('CONTROL_OPERATION');await x.prepareHostApp();await descriptor();result={prepared:true};break;
 case 'peer':await x.bootstrapPeer();await descriptor();result={privateDescriptor:ready};break;
 case 'moderate':await x.moderate(command.target,command.action);await descriptor();result={moderated:true,...await x.state()};break;
 case 'lobbyCreate':if(!b1Gold||Object.keys(command).some(k=>!['id','op'].includes(k)))throw Error('FIXTURE_ROOM');await x.createLobby();await descriptor();result={created:true,privateDescriptor:ready,...await x.state()};break;
 case 'coreLobbyCreate':if(!race||Object.keys(command).some(k=>!['id','op','config'].includes(k)))throw Error('CONTROL_OPERATION');result={...await x.createCoreLobby(command.config),created:true};await descriptor();result={...result,privateDescriptor:ready,...await x.state()};break;
 case 'lobbyDeleteForbidden':if(!b1Gold||Object.keys(command).some(k=>!['id','op','roomId'].includes(k)))throw Error('FIXTURE_ROOM');result=await x.forbiddenLobbyDelete(command.roomId);break;
 case 'lobbyTransferSummary':if(!b1Gold||Object.keys(command).some(k=>!['id','op','roomId'].includes(k)))throw Error('FIXTURE_ROOM');result={summary:await x.lobbyTransferSummary(command.roomId)};break;
 case 'lobbyDelete':if(!b1Gold||Object.keys(command).some(k=>!['id','op','roomId'].includes(k)))throw Error('FIXTURE_ROOM');result=await x.deleteLobby(command.roomId);break;
 case 'nextRun':await x.nextRun(command.config);await descriptor();result={opened:true,privateDescriptor:ready,...await x.state()};break;
 case 'advance':if(Object.keys(command).some(k=>!['id','op','ms','kind'].includes(k)))throw Error('FIXTURE_CLOCK_ARGUMENT');result={clock:x.advance(command.ms,command.kind),advanced:!(command.kind==='wall'&&x.clockStatus().mode==='elapsed')};break;
 case 'clockMode':if(Object.keys(command).some(k=>!['id','op','mode'].includes(k)))throw Error('FIXTURE_CLOCK_ARGUMENT');result={clock:x.clockMode(command.mode)};break;
 case 'clockStatus':if(Object.keys(command).some(k=>!['id','op'].includes(k)))throw Error('FIXTURE_CLOCK_ARGUMENT');result={clock:x.clockStatus()};break;
 case 'signIn':await x.signIn();await descriptor();result={privateDescriptor:ready};break;
 case 'arm':result={armId:x.faults.arm(command.plan)};break;
 case 'release':x.faults.release();result={released:true};break;
 case 'state':result=await x.state(command.runId);break;
 case 'events':result={events:x.faults.snapshot()};break;
 case 'transport':await x.transport(command.kind,command.enabled,command.target);result={configured:true};break;
 case 'networkProfile':if(!['rtt80','rtt200','rtt400'].includes(command.profile)||Object.keys(command).some(k=>!['id','op','target','profile','seed','durationMs'].includes(k)))throw Error('FIXTURE_NETWORK_ARGUMENT');result={network:x.faults.transportControl.networkProfile(command.target,command.profile,command.seed,command.durationMs)};break;
 case 'networkStatus':if(Object.keys(command).some(k=>!['id','op'].includes(k)))throw Error('FIXTURE_NETWORK_ARGUMENT');result={network:x.faults.transportControl.networkStatus()};break;
 case 'liveCapacityStatus':if(Object.keys(command).some(k=>!['id','op'].includes(k)))throw Error('FIXTURE_CAPACITY_ARGUMENT');result={liveCapacity:x.faults.capacityDiagnostics!.status()};break;
 case 'liveAuthStatus':if(Object.keys(command).some(k=>!['id','op'].includes(k)))throw Error('FIXTURE_AUTH_ARGUMENT');result={liveAuth:x.faults.authDiagnostics!.status()};break;
 case 'networkReset':if(Object.keys(command).some(k=>!['id','op','target'].includes(k)))throw Error('FIXTURE_NETWORK_ARGUMENT');result={network:x.faults.transportControl.networkReset(command.target)};break;
 case 'denyWssOnce':if(Object.keys(command).some(k=>!['id','op','target'].includes(k)))throw Error('FIXTURE_WSS_REFUSAL_ARGUMENT');result=x.denyWssOnce(command.target);break;
 case 'wssRefusalStatus':if(Object.keys(command).some(k=>!['id','op'].includes(k)))throw Error('FIXTURE_WSS_REFUSAL_ARGUMENT');result=x.wssRefusalStatus();break;
 case 'clearWssRefusal':if(Object.keys(command).some(k=>!['id','op'].includes(k)))throw Error('FIXTURE_WSS_REFUSAL_ARGUMENT');result=x.clearWssRefusal();break;
 case 'optionalStall':x.faults.optionalStall(command.ms);result={configured:true};break;
 case 'chat':if(Object.keys(command).some(k=>!['id','op','target','text'].includes(k)))throw Error('FIXTURE_CHAT_ARGUMENT');result=await x.chat(command.target,command.text);break;
 case 'c2Rules':{const {id,op,...input}=command;result=await x.c2Rules(input);break;}
 case 'c2Status':if(Object.keys(command).some(k=>!['id','op','eventId'].includes(k)))throw Error('FIXTURE_C2_AUDIT');result={c2:await x.c2Status(command.eventId)};break;
 case 'c2DropPublishReply':if(Object.keys(command).some(k=>!['id','op','target','eventId'].includes(k)))throw Error('FIXTURE_C2_DROP');result=x.c2DropPublishReply(command.target,command.eventId);break;
 case 'c2Network':if(Object.keys(command).some(k=>!['id','op','target','durationMs'].includes(k)))throw Error('FIXTURE_C2_NETWORK');result={network:x.c2Network(command.target,command.durationMs)};break;
 case 'c1Rules':{const {id,op,...input}=command;result=await x.c1Rules(input);break;}
 case 'c1Status':if(Object.keys(command).some(k=>!['id','op','eventId'].includes(k)))throw Error('FIXTURE_C1_AUDIT');result={c1:await x.c1Status(command.eventId)};break;
 case 'c1DropPublishReply':if(Object.keys(command).some(k=>!['id','op','target','eventId'].includes(k)))throw Error('FIXTURE_C1_DROP');result=x.c1DropPublishReply(command.target,command.eventId);break;
 case 'c1Network':if(Object.keys(command).some(k=>!['id','op','target','durationMs'].includes(k)))throw Error('FIXTURE_C1_NETWORK');result={network:x.c1Network(command.target,command.durationMs)};break;
 case 'goldArm':if(!b1Gold||Object.keys(command).some(k=>!['id','op','plan'].includes(k)))throw Error('FIXTURE_GOLD_ARGUMENT');result=x.faults.goldControl.arm(command.plan);break;
 case 'goldStatus':if(!b1Gold||Object.keys(command).some(k=>!['id','op'].includes(k)))throw Error('FIXTURE_GOLD_ARGUMENT');result=x.faults.goldControl.status();break;
 case 'goldRelease':if(!b1Gold||Object.keys(command).some(k=>!['id','op'].includes(k)))throw Error('FIXTURE_GOLD_ARGUMENT');x.faults.goldControl.release();result={released:true};break;
 case 'socialArm':if(Object.keys(command).some(k=>!['id','op','plan'].includes(k)))throw Error('FIXTURE_SOCIAL_ARGUMENT');result=x.faults.socialControl.arm(command.plan);break;
 case 'socialRateArm':if(Object.keys(command).some(k=>!['id','op','plan'].includes(k)))throw Error('FIXTURE_SOCIAL_RATE_ARGUMENT');result=x.faults.socialControl.armRate(command.plan);break;
 case 'socialStatus':if(Object.keys(command).some(k=>!['id','op'].includes(k)))throw Error('FIXTURE_SOCIAL_ARGUMENT');result={social:x.faults.socialControl.status()};break;
 case 'socialRelease':if(Object.keys(command).some(k=>!['id','op'].includes(k)))throw Error('FIXTURE_SOCIAL_ARGUMENT');result=x.faults.socialControl.release();break;
 case 'seedSocialPois':if(Object.keys(command).some(k=>!['id','op','target','eventId','total'].includes(k)))throw Error('FIXTURE_SOCIAL_ARGUMENT');result={seeded:await x.seedSocialPois(command.target,command.eventId,command.total)};break;
 case 'close':await close();emit({id:command.id,event:'closed',schema:x.fixture.namespace});break;
 default:throw Error('CONTROL_OPERATION');
 }if(closed)break;emit({id:command.id,event:'result',...result});}catch(e){const message=e instanceof Error?e.message:'';emit({id:command?.id,event:'error',code:['c2Rules','c2Status','c2DropPublishReply','c2Network'].includes(command?.op)?'FIXTURE_C2_CONTROL_FAILED':command?.op==='c1Rules'?'FIXTURE_C1_RULE_CONTROL_FAILED':/^[A-Z_]{1,64}$/.test(message)?message:'FIXTURE_CONTROL_FAILED',...(['c2Rules','c2Status','c2DropPublishReply','c2Network'].includes(command?.op)?{operation:command.op,c2Diagnostic:x.c2RulesDiagnostics()}:{}),...(command?.op==='c1Rules'?{operation:'c1Rules',c1Diagnostic:x.c1RulesDiagnostics()}:{}),...(command?.op==='chat'?{chatDiagnostic:x.chatDiagnostics()}:{}),...(command?.op==='end'?{hostActionDiagnostic:x.hostActionDiagnostics()}:{}),...(['start','startReviewed'].includes(command?.op)?{operation:command.op,startDiagnostic:x.startDiagnostics().at(-1)}:{})});}}
}finally{await close();}



