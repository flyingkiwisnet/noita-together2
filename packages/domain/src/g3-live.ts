import {validate,schema} from '../../contracts/src/g3-live.js';
import {createHash} from 'node:crypto';
export const LIVE_POLICY={endpoint:'/v3/live',subprotocol:'nt2.g3.live.v1',encoding:'protobuf-binary',implemented:true,
 poseHz:10,snapshotHz:10,selectedPlayers:32,rooms:128,connections:4096,connectionsPerSession:1,
 authDeadlineMs:3000,reauthDeadlineMs:3000,reauthLeadMs:60000,controlFramesPerSecond:4,
 poseTtlMs:2000,frameBytes:32768,controlQueueFrames:8,latestPoseSlots:1,latestSnapshotSlots:1,
 outboundBytes:65536,slowConsumerDeadlineMs:2000,closeDrainMs:1000,bankCoalesceMs:200,latestBankSlots:1,reconnectBackoffMs:[1000,2000,4000,8000,10000]} as const;
const uuid=(v:unknown)=>typeof v==='string'&&/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/.test(v);
export function validateLiveFrame(direction:'ClientFrame'|'ServerFrame',value:unknown){
 const f=validate(direction,value) as any;
 if(f.major!==1||f.revision!==schema.revision||!f.serviceId||!uuid(f.serviceEpoch)||!uuid(f.connectionId)||!f.clientGeneration||!f.sequence)throw Error('INVALID_MESSAGE');
 for(const key of ['principalId','sessionId','sessionEpoch','roomId','runId','runEpoch','attachmentId'])if(!uuid(f.scope[key]))throw Error('INVALID_MESSAGE');
 if(!f.scope.configRevision||!f.scope.worldToken)throw Error('INVALID_MESSAGE');
 if(direction==='ClientFrame'&&f.authenticate){if(f.sequence!==1||(f.authorityGeneration??0)!==0||!f.authenticate.accessToken)throw Error('INVALID_MESSAGE');}
 else if(!f.authorityGeneration)throw Error('STALE_EPOCH');
 if(f.reauthenticate&&(!f.reauthenticate.authRevision||!f.reauthenticate.accessToken))throw Error('INVALID_MESSAGE');
 if(f.c2Changed&&(!/^(0|[1-9][0-9]{0,18})$/.test(f.c2Changed.sequence)||BigInt(f.c2Changed.sequence)>9223372036854775807n))throw Error('INVALID_MESSAGE');
 if(f.c1Changed&&(!/^(0|[1-9][0-9]{0,18})$/.test(f.c1Changed.sequence)||BigInt(f.c1Changed.sequence)>9223372036854775807n))throw Error('INVALID_MESSAGE');
 if(f.bankChanged&&(!/^(0|[1-9][0-9]{0,18})$/.test(f.bankChanged.revision)||BigInt(f.bankChanged.revision)>9223372036854775807n))throw Error('INVALID_MESSAGE');
 const poses=f.pose?[f.pose]:(f.snapshot?.peers??[]).flatMap((p:any)=>p.poses??[]);
 for(const p of poses){if(!p.sequence)throw Error('INVALID_MESSAGE');for(const key of ['xMilli','yMilli','vxMilli','vyMilli'])if(!/^(0|-?[1-9][0-9]{0,9})$/.test(p[key])||Math.abs(Number(p[key]))>2147483647)throw Error('INVALID_MESSAGE');}
 return f;
}
const stable=(v:any):any=>Array.isArray(v)?v.map(stable):v&&typeof v==='object'?Object.fromEntries(Object.entries(v).sort(([a],[b])=>a.localeCompare(b)).map(([k,x])=>[k,stable(x)])):v;
export const liveScopeDigest=(scope:unknown)=>createHash('sha256').update(JSON.stringify(stable(scope))).digest('hex');
// C2 must supply only committed, currently authenticated authority to this fence.
export function acceptLiveFrame(value:unknown,binding:{serviceId:string;serviceEpoch:string;connectionId:string;clientGeneration:number;authorityGeneration:number;scope:unknown;lastSequence:number;lastPoseSequence:number;authRevision:number;accessUntil:number;authorityUntil:number},now:number){
 const f=validateLiveFrame('ClientFrame',value);
 if(f.authenticate||f.serviceId!==binding.serviceId||f.serviceEpoch!==binding.serviceEpoch||f.connectionId!==binding.connectionId||f.clientGeneration!==binding.clientGeneration||f.authorityGeneration!==binding.authorityGeneration||liveScopeDigest(f.scope)!==liveScopeDigest(binding.scope)||f.sequence<=binding.lastSequence)throw Error('STALE_EPOCH');
 if(now>=binding.accessUntil||now>=binding.authorityUntil)throw Error('EXPIRED');
 if(f.pose&&f.pose.sequence<=binding.lastPoseSequence)throw Error('STALE_EPOCH');
 if(f.reauthenticate&&f.reauthenticate.authRevision!==binding.authRevision+1)throw Error('STALE_EPOCH');
 return f;
}
