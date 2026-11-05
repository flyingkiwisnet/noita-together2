import test from 'node:test';
import assert from 'node:assert/strict';
import {createServer,connect,type Socket} from 'node:net';
import {Agent} from 'node:https';
import {connect as tlsConnect,checkServerIdentity} from 'node:tls';
import {startG3Fixture,provisionG3Fixture} from '../scripts/g3-fixture.js';
import {livePeer} from '../scripts/g3-live-fixture.js';
const sleep=(ms:number)=>new Promise<void>(r=>setTimeout(r,ms));
const percentile=(values:number[],p:number)=>[...values].sort((a,b)=>a-b)[Math.min(values.length-1,Math.floor(values.length*p))]??0;

// Real TLS/WebSocket bytes traverse this bounded loopback TCP relay in both
// directions. Delayed delivery is remote receipt, not a rendering-FPS estimate.
async function delayedRelay(f:Awaited<ReturnType<typeof startG3Fixture>>,rtt:number){
 const sockets=new Set<Socket>(),timers=new Set<NodeJS.Timeout>();let peakBytes=0;
 const server=createServer(client=>{const upstream=connect({host:'127.0.0.1',port:Number(new URL(f.origin).port)});sockets.add(client);sockets.add(upstream);
  const pipe=(source:Socket,target:Socket)=>{let bytes=0,due=0,n=0,active=false;const queue:{chunk:Buffer;due:number}[]=[];
   const drain=()=>{if(active||!queue.length)return;active=true;const item=queue[0]!;const timer=setTimeout(()=>{timers.delete(timer);queue.shift();bytes-=item.chunk.length;if(!target.destroyed)target.write(item.chunk);active=false;drain();},Math.max(0,item.due-Date.now()));timers.add(timer);};
   source.on('data',chunk=>{bytes+=chunk.length;peakBytes=Math.max(peakBytes,bytes);if(bytes+target.writableLength>1048576){source.destroy();target.destroy();return;}
    due=Math.max(due,Date.now()+rtt/2+([0,5,-5,10][n++%4]??0));queue.push({chunk,due});drain();
   });source.on('error',()=>target.destroy());source.on('close',()=>{sockets.delete(source);target.destroy();});};pipe(client,upstream);pipe(upstream,client);
 });await new Promise<void>(r=>server.listen(0,'127.0.0.1',r));const port=(server.address() as {port:number}).port;
 const agent=new Agent({ca:f.ca!,checkServerIdentity:(_host,cert)=>checkServerIdentity('127.0.0.1',cert)});
 agent.createConnection=(options:any)=>tlsConnect({...options,host:'127.0.0.1',port,servername:'',ca:f.ca!,checkServerIdentity:(_host,cert)=>checkServerIdentity('127.0.0.1',cert)});
 return {agent,get peakBytes(){return peakBytes;},async close(){agent.destroy();for(const timer of timers)clearTimeout(timer);for(const socket of sockets)socket.destroy();await new Promise<void>(r=>server.close(()=>r()));}};
}

test('8/16/32 real shared-NAT WSS actors across three rooms: 50/100/200ms RTT+jitter remote cadence and bounds',async()=>{
 const results:unknown[]=[];
 for(const [players,rtt]of [[8,50],[16,100],[32,200]].filter(([n])=>!process.env.NT2_LIVE_PRESSURE_PLAYERS||process.env.NT2_LIVE_PRESSURE_PLAYERS.split(',').includes(String(n)))){
  let offset=0;const f=await startG3Fixture({live:true,tls:true,a1Runtime:true,a1HeartbeatBusy:true,nativeSocial:true,now:()=>Date.now()+offset});
  const relay=await delayedRelay(f,rtt!),peers:Awaited<ReturnType<typeof livePeer>>[]=[],rooms:Awaited<ReturnType<typeof provisionG3Fixture>>[]=[];let healthError:unknown,healthBusy=false,health=Promise.resolve(),pressure=false;
  // Population/admission also obey original health deadlines; no lease widening.
  const keepalive=setInterval(()=>{if(healthBusy)return;healthBusy=true;health=(async()=>{for(const s of rooms)await s.pulse();})().catch(e=>{healthError=e;}).finally(()=>{healthBusy=false;});},3000);
  try{
   for(const [i,count]of [players!/4,players!/4,players!/2].entries()){
    const s=await provisionG3Fixture(f,{playerCount:count,devicePrefix:'pressure-'+players+'-'+i,beforeRpc:()=>{if(!pressure)offset+=30;}});await s.start();await s.playing();rooms.push(s);
   }
   const jobs=rooms.flatMap(s=>s.games.map((_,i)=>()=>livePeer(f,s,i,{agent:relay.agent})));
   for(let i=0;i<jobs.length;i+=4)peers.push(...await Promise.all(jobs.slice(i,i+4).map(job=>job())));
   clearInterval(keepalive);await health;if(healthError)throw healthError;pressure=true;for(const s of rooms)await s.pulse();
   const start=Date.now(),sequences=new Map<string,number>();
   for(let tick=1;tick<=30;tick++){
    for(const peer of peers){const sequence=(sequences.get(peer.scope.sessionId!)??0)+1;sequences.set(peer.scope.sessionId!,sequence);peer.send({pose:{sequence,xMilli:String(tick*1000),yMilli:'0',vxMilli:'0',vyMilli:'0'}});}
    await sleep(102);
   }
   await sleep(rtt!+200);const finish=Date.now();const rates:number[]=[],ages:number[]=[],gaps:number[]=[];
   for(const peer of peers){const seen=new Map<string,number>(),arrival:number[]=[];let updates=0;
    for(const r of peer.received){if(!r.frame.snapshot||r.at<start+500||r.at>finish-300)continue;arrival.push(r.at);
     for(const remote of r.frame.snapshot.peers??[]){assert.equal(rooms.some(s=>s.scopes.some(scope=>scope.roomId===peer.scope.roomId&&scope.attachmentId===remote.attachmentId)),true,'snapshot room isolation');const latest=remote.poses!.at(-1)!.sequence!;if(latest>(seen.get(remote.sessionId!)??0)){seen.set(remote.sessionId!,latest);updates++;ages.push(r.at-(Date.parse(remote.acceptedAt!)-offset));}}
    }
    for(let i=1;i<arrival.length;i++)gaps.push(arrival[i]!-arrival[i-1]!);
    const others=rooms.find(s=>s.room.roomId===peer.scope.roomId)!.games.length-1;rates.push(updates/others/((finish-start-800)/1000));
   }
   const stats=f.service.live!.stats;if(stats.receivedPoses!==players!*30)process.stdout.write('PRESSURE_DIAGNOSTIC '+JSON.stringify({players,stats,retirements:peers.map(p=>p.received.filter(r=>r.frame.retire).map(r=>r.frame.retire))})+'\n');assert.equal(stats.receivedPoses,players!*30);assert.equal(stats.connections,players);assert.ok(Math.min(...rates)>7,'unique remotely received peer samples sustain cadence');assert.ok(percentile(gaps,.99)<300);assert.ok(percentile(ages,.99)<500);assert.ok(stats.maxQueuedBytes<=65536);assert.ok(stats.maxControlFrames<=8);assert.ok(stats.maxSamples<=players!);assert.ok(relay.peakBytes<=1048576);
   results.push({players,rtt,jitterMs:[-5,0,5,10],remoteUpdatesHzMin:Math.min(...rates),sampleAgeMs:{p50:percentile(ages,.5),p95:percentile(ages,.95),p99:percentile(ages,.99)},snapshotGapMs:{p95:percentile(gaps,.95),p99:percentile(gaps,.99)},stats,relayPeakQueuedBytes:relay.peakBytes});
  }finally{clearInterval(keepalive);await health;for(const peer of peers)await peer.close();await relay.close();await f.close();}
 }
 process.stdout.write('LIVE_PRESSURE '+JSON.stringify(results)+'\n');
});
