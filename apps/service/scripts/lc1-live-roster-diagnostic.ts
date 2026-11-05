// Private first-loss evidence at the actual wire-frame boundary. No payloads,
// scopes, tokens or SQL are retained; original methods/results are unchanged.
import {createHash} from 'node:crypto';
import {performance} from 'node:perf_hooks';
export function traceLiveRoster(live:any,now:()=>number){
 const began=performance.now(),frames=new WeakMap<object,{hadPeers:boolean;last:number}>(),sizes=new WeakMap<object,number>();
 let snapshots=0,maxFrameGapMs=0,firstEmptySnapshot:any,firstProjectionReadFailure:any,firstProjectionShrink:any;
 const frame=live.frame,read=live.projection.read,change=live.projection.onChange;
 const observedFrame=function(this:any,c:any,payload:any){const value=frame.call(this,c,payload);try{
  if(value.snapshot&&!c.retired){const at=performance.now(),prior=frames.get(c),wirePeers=value.snapshot.peers?.length??0,gap=prior?Math.round(at-prior.last):0;
   snapshots++;maxFrameGapMs=Math.max(maxFrameGapMs,gap);frames.set(c,{hadPeers:!!prior?.hadPeers||wirePeers>0,last:at});
   const room=this.projection.rooms.get(c.binding?.scope.roomId),peers=room?[...room.members.values()].filter((m:any)=>m.scope.sessionId!==c.binding?.scope.sessionId):[];
   if(!firstEmptySnapshot&&prior?.hadPeers&&peers.length>0&&wirePeers===0)firstEmptySnapshot={elapsedMs:Math.round(at-began),connectionTag:createHash('sha256').update(String(c.binding.connectionId)).digest('hex').slice(0,16),lastFrameGapMs:gap,sourcePeers:payload.snapshot.peers?.length??0,wirePeers,eligiblePeers:peers.length,roomTerminal:!!room.terminal,busy:!!c.busy,authorityRemainingMs:value.authorityRemainingMs,accessRemainingMs:value.accessRemainingMs,receivedPoses:this.metrics.receivedPoses,samples:peers.slice(0,32).map((m:any)=>{const s=this.samples.get(m.scope.sessionId);return {present:!!s,...(s?{ageMs:Math.max(0,Math.floor(now()-s.acceptedAt)),generationMatches:s.generation===m.generation,poses:s.poses.length}:{}),lastAcceptedPoseSequence:m.lastPose};})};
  }
 }catch{/* Observation must never change frame construction. */}return value;};
 const observedRead=async function(this:any,...args:any[]){try{return await read.apply(this,args);}catch(e){try{if(!firstProjectionReadFailure){const code=String((e as any)?.code??(e instanceof Error?e.message:''));firstProjectionReadFailure={elapsedMs:Math.round(performance.now()-began),code:/^[A-Z0-9_]{1,64}$/.test(code)?code:'UNCLASSIFIED_READ_FAILURE'};}}catch{}throw e;}};
 const observedChange=function(this:any,room:any){try{const previous=sizes.get(room);if(!firstProjectionShrink&&previous!==undefined&&room.members.size<previous)firstProjectionShrink={elapsedMs:Math.round(performance.now()-began),previousMembers:previous,members:room.members.size,terminal:!!room.terminal,normalEnd:!!room.normalEnd};sizes.set(room,room.members.size);}catch{}return change.call(this,room);};
 live.frame=observedFrame;live.projection.read=observedRead;live.projection.onChange=observedChange;
 return {status:()=>structuredClone({snapshots,maxFrameGapMs,firstEmptySnapshot:firstEmptySnapshot??null,firstProjectionReadFailure:firstProjectionReadFailure??null,firstProjectionShrink:firstProjectionShrink??null}),close:()=>{if(live.frame===observedFrame)live.frame=frame;if(live.projection.read===observedRead)live.projection.read=read;if(live.projection.onChange===observedChange)live.projection.onChange=change;}};
}
