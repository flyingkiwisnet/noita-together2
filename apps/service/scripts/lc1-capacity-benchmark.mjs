// Independent disposable Server benchmark. No installed host, native client or engine.
// node --import tsx apps/service/scripts/lc1-capacity-benchmark.mjs [rooms] [playersPerRoom] [seconds]
import {fork} from 'node:child_process';
import {randomUUID,randomBytes} from 'node:crypto';
import {readFile,writeFile} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import {Agent,request} from 'node:https';
import {checkServerIdentity} from 'node:tls';
import {performance,monitorEventLoopDelay} from 'node:perf_hooks';
import {WebSocket} from 'ws';
import protobuf from 'protobufjs';
import {startG3Fixture} from './g3-loot01-fixture.ts';
import {fingerprint} from '../src/g1/security.ts';
import {schema as lobby} from '../../../packages/contracts/src/g2-next.ts';
import {schema as lifecycle} from '../../../packages/contracts/src/g3-lifecycle.ts';
import {schema as live,encode,decode} from '../../../packages/contracts/src/g3-live.ts';

const sleep=ms=>new Promise(r=>setTimeout(r,ms));
const summarize=a=>{const s=[...a].sort((x,y)=>x-y);return {n:s.length,mean:s.reduce((a,b)=>a+b,0)/(s.length||1),p50:s[Math.floor((s.length-1)*.5)]??0,p95:s[Math.floor((s.length-1)*.95)]??0,p99:s[Math.floor((s.length-1)*.99)]??0,max:s.at(-1)??0};};
const bump=(map,key)=>map[key]=(map[key]??0)+1;

async function worker(){
 process.stdout.on('error',()=>{});process.stderr.on('error',()=>{});
 const emit=value=>{if(process.connected)process.send(value,()=>{});};
 let f,clock=Date.now(),wallStart,closing=false;
 const now=()=>Math.floor(clock+(wallStart===undefined?0:performance.now()-wallStart));
 let metric,baseCpu,baseElu,baseAt;
 const lag=monitorEventLoopDelay({resolution:10});lag.enable();
 const reset=()=>{metric={queries:0,queryMs:0,queryErrors:{},mutationLocks:[],tokenChecks:0,tokenIds:0,projectionPrepares:0,projectionReads:0,projectionMs:0,diagnostics:{},maxDbActive:0,maxPoolWaiting:0,maxRss:0,maxHeap:0};baseCpu=process.cpuUsage();baseElu=performance.eventLoopUtilization();baseAt=performance.now();lag.reset();};reset();
 const sampler=setInterval(()=>{if(!f)return;const d=f.service.db.workStats,m=process.memoryUsage();metric.maxDbActive=Math.max(metric.maxDbActive,d.active??0);metric.maxPoolWaiting=Math.max(metric.maxPoolWaiting,d.poolWaiting);metric.maxRss=Math.max(metric.maxRss,m.rss);metric.maxHeap=Math.max(metric.maxHeap,m.heapUsed);},20);sampler.unref();
 const close=async()=>{if(closing)return;closing=true;clearInterval(sampler);lag.disable();await f?.close();};
 process.on('disconnect',()=>{void close();});
 try{
  f=await startG3Fixture({rejoin:true,loot01Runtime:true,live:true,tls:true,now,databaseUrl:'postgres://nt2_g1:local-fixture-only@127.0.0.1:54328/nt2_g1'});
  const db=f.service.db,projection=f.service.live.projection;
  const instrument=client=>{
   if(client.capacityObserved)return;client.capacityObserved=true;const original=client.query;
   client.query=function(...args){
    const at=performance.now(),sql=typeof args[0]==='string'?args[0]:args[0]?.text??'',values=Array.isArray(args[1])?args[1]:args[0]?.values;
    const observed=metric;
    const done=error=>{const ms=performance.now()-at;observed.queries++;observed.queryMs+=ms;if(error)bump(observed.queryErrors,/^[0-9A-Z]{5}$/.test(error.code??'')?error.code:'OTHER');if(sql.includes('pg_advisory_xact_lock')&&values?.[0]==='nt2-g1-mutation:'+db.namespace)observed.mutationLocks.push(ms);if(sql.includes('FROM access_tokens t JOIN sessions')&&sql.includes('ANY($1::uuid[])')){observed.tokenChecks++;observed.tokenIds+=values?.[0]?.length??0;}};
    if(typeof args.at(-1)==='function'){const callback=args.at(-1);args[args.length-1]=function(error,...rest){done(error);return callback.call(this,error,...rest);};return original.apply(this,args);}
    try{const result=original.apply(this,args);return result.then(v=>{done();return v;},e=>{done(e);throw e;});}catch(e){done(e);throw e;}
   };
  };
  db.pool.on('connect',instrument);for(const client of db.pool._clients)instrument(client);
  db.transactionDiagnostic=d=>bump(metric.diagnostics,[d.category,d.phase,d.code,d.sqlState??'',d.commit].join(':'));
  const read=projection.read.bind(projection),prepare=projection.prepare.bind(projection);
  projection.read=async(...a)=>{metric.projectionReads++;return read(...a);};
  projection.prepare=async(...a)=>{const observed=metric,t=performance.now();observed.projectionPrepares++;try{return await prepare(...a);}finally{observed.projectionMs+=performance.now()-t;}};
  const base=()=>({major:1,revision:lobby.revision,serviceId:db.serviceId,serviceEpoch:db.epoch,requestId:randomUUID()});
  const call=async(path,q,c)=>{for(let retry=0;retry<4;retry++){clock++;const r=await f.call(path,q,c?.accessToken),v=JSON.parse(r.body);if(v.error?.code==='CAPACITY'){clock+=1000;continue;}if(v.error||r.status!==200)throw Error('SETUP:'+path+':'+Object.keys(q).at(-1)+':'+(v.error?.code??r.status));return v;}throw Error('SETUP:CAPACITY');};
  const host=(await call('/v1/mock/browser-login',{subject:'twitch-local-alex',deviceId:'capacity-host-'+randomUUID(),requestId:randomUUID()})).credentials;
  const users=[],rooms=[];
  const roomCount=+process.argv[3],size=+process.argv[4];
  for(let r=0;r<roomCount;r++){
   let room;
   const rpc=async(c,payload)=>{const q={...base(),sessionEpoch:c.sessionEpoch,...(room?{roomId:room.roomId,runId:room.runId,runEpoch:room.runEpoch,configRevision:room.configRevision,expectedRevision:room.revision}:{}),...payload};const v=await call('/v1/rpc',q,c);if(v.room)room=v.room;return v;};
   await rpc(host,{roomCreate:{name:'Capacity fixture '+r,allowGuests:true,config:{seed:424242,seedPolicy:'shared-fixed',randomizeLoot:true,mode:'core',requiredCapabilities:f.profile.compatibility.capabilities}}});
   await rpc(host,{roomPolicy:{capacity:32,allowGuests:true}});
   const group=[];
   for(let i=0;i<size;i++){
    // Setup only: direct canonical identity issuance avoids conflating one-IP
    // anonymous sign-up policy with authenticated steady-state capacity.
    const desktop=await db.transaction(tx=>f.service.auth.guest(tx,{deviceId:randomUUID(),issuanceProof:randomBytes(32).toString('base64url'),displayName:`Capacity ${r}-${i}`},randomUUID()));
    await rpc(desktop,{roomPreview:{}});await rpc(desktop,{roomJoin:{}});
    const proof=randomBytes(32).toString('base64url'),worldId=randomUUID();
    const prepared=await call('/v3/lifecycle/rpc',{...base(),revision:lifecycle.revision,sessionEpoch:desktop.sessionEpoch,prepare:{worldId,roomId:room.roomId,proof,configRevision:room.configRevision,configDigest:fingerprint(room.config)}},desktop);
    const observation={worldToken:worldId,gameInstanceId:'capacity-'+worldId,observationRevision:1,actualSeed:'424242',seedStatus:'known',activeMods:['noita_together2'],inventoryComplete:true,collectorFailure:'',beta:'unknown',protected:true,loaded:true,modVersion:f.profile.compatibility.modVersion,bridgeVersion:f.profile.compatibility.bridgeVersion,compatibilityProvenance:'game-thread-client-observation'};
    const reconcile={association:prepared.prepared.association,proof,attempt:1,connectionId:'capacity-'+worldId,action:'connect',gameVersion:f.profile.compatibility.gameVersion,modVersion:f.profile.compatibility.modVersion,bridgeVersion:f.profile.compatibility.bridgeVersion,capabilities:f.profile.compatibility.capabilities,observation};
    await call('/v3/lifecycle/rpc',{...base(),revision:lifecycle.revision,sessionEpoch:desktop.sessionEpoch,reconcile},desktop);
    group.push({desktop,reconcile});
   }
   await rpc(host,{roomGet:{}});const reports=[];let cursor='';
   do{const v=await rpc(host,{inspectionList:{limit:1,afterDesktopId:cursor}});reports.push(...v.inspections);cursor=v.inspectionNextDesktopId??'';}while(cursor);
   if(reports.length!==size)throw Error('SETUP:roster-count');
   await rpc(host,{finalStart:{participants:reports.map(p=>({desktopId:p.desktopId,attachmentId:p.attachmentId,worldToken:p.worldToken,observationRevision:p.observationRevision,contentDigest:p.contentDigest})),excludedDesktopIds:[]}});
   for(const user of group){user.reconcile.observation={...user.reconcile.observation,protected:false,observationRevision:2};const v=await call('/v3/lifecycle/rpc',{...base(),revision:lifecycle.revision,sessionEpoch:user.desktop.sessionEpoch,reconcile:user.reconcile},user.desktop);if(!v.canonicalAdmission?.decision?.playing)throw Error('SETUP:not-playing');users.push({...user,game:v.credentials,scope:v.canonicalAdmission.scope,room});}
   rooms.push(room.roomId);console.log(JSON.stringify({setupRoom:r+1,players:users.length}));
  }
  emit({type:'ready',origin:f.origin,ca:f.ca.toString('base64'),namespace:f.namespace,caPath:f.caPath,base:base(),users,rooms});
  process.on('message',async message=>{
   const {id,command}=message;
   try{
    let value;
    if(command==='begin'){if(wallStart===undefined)wallStart=performance.now();reset();value={live:f.service.live.stats};}
    else if(command==='metrics'){const cpu=process.cpuUsage(baseCpu),elapsedMs=performance.now()-baseAt;value={...metric,mutationLocks:summarize(metric.mutationLocks),elapsedMs,cpuMs:(cpu.user+cpu.system)/1000,cpuOneCorePercent:(cpu.user+cpu.system)/elapsedMs/10,eventLoopUtilization:performance.eventLoopUtilization(baseElu).utilization,eventLoopDelayMs:{p95:lag.percentile(95)/1e6,p99:lag.percentile(99)/1e6,max:lag.max/1e6},memory:process.memoryUsage(),db:db.workStats,live:f.service.live.stats};}
    else if(command==='hold'){await db.transaction(async tx=>{await tx.query('SELECT 1 FROM rooms WHERE id=$1 FOR UPDATE',[rooms[0]]);emit({type:'held'});await sleep(650);});value={heldMs:650};}
    else if(command==='close'){await close();emit({id,value:{cleaned:true}});process.disconnect();return;}
    else throw Error('COMMAND');
    emit({id,value});
   }catch(e){emit({id,error:/^[A-Z_:0-9/-]{1,120}$/.test(e.message)?e.message:'WORKER_ERROR'});}
  });
 }catch(e){console.log(JSON.stringify({workerFailure:String(e.message)}));await close();emit({type:'failed',error:String(e.message)});if(process.connected)process.disconnect();process.exitCode=1;}
}

// Read bounded protobuf envelope/peer counters without allocating and validating
// every pose object in the load generator. Full production decoding is sampled
// once per client per five seconds, plus every control frame.
function observeFrame(data,p){
 const r=protobuf.Reader.create(data),s=p.stats;let snapshot=false;
 while(r.pos<r.len){const tag=r.uint32(),field=tag>>>3;
  if(field!==live.messages.ServerFrame.fields.snapshot.id){r.skipType(tag&7);continue;}
  snapshot=true;s.snapshots++;const end=r.uint32()+r.pos;let peers=0;
  while(r.pos<end){const tag=r.uint32();if((tag>>>3)!==live.messages.Snapshot.fields.peers.id){r.skipType(tag&7);continue;}const peerEnd=r.uint32()+r.pos;peers++;
   while(r.pos<peerEnd){const t=r.uint32(),id=t>>>3;if(id===live.messages.Peer.fields.sessionId.id){if(!p.allowed.has(r.string()))s.foreignPeers++;}else if(id===live.messages.Peer.fields.poses.id){s.poseRecords++;r.skipType(t&7);}else r.skipType(t&7);}
  }s.peerRecords+=peers;if(peers)s.nonempty++;
 }
 return snapshot;
}

async function parent(){
 const roomCount=+(process.argv[2]??4),size=+(process.argv[3]??30),seconds=+(process.argv[4]??16);
 if(!Number.isInteger(roomCount)||roomCount<1||roomCount>32||!Number.isInteger(size)||size<2||size>32||roomCount*size>120||seconds<6||seconds>60)throw Error('Bounded fixture arguments');
 const child=fork(fileURLToPath(import.meta.url),['--worker',String(roomCount),String(size)],{execArgv:['--import','tsx'],windowsHide:true,stdio:['ignore','pipe','pipe','ipc']});
 let output='',verifiedAbsence=false;
 child.stdout.on('data',b=>{process.stdout.write(b);output+=String(b);if(output.includes('"verifiedAbsentSchema"'))verifiedAbsence=true;output=output.slice(-4096);});child.stderr.on('data',b=>process.stderr.write(b));
 let readyResolve,readyReject,heldResolve;const ready=new Promise((r,j)=>{readyResolve=r;readyReject=j;}),pending=new Map();let exited=false;
 child.on('exit',code=>{exited=true;readyReject(Error('WORKER_EXIT:'+code));for(const p of pending.values())p.reject(Error('WORKER_EXIT'));});
 child.on('message',m=>{if(m.type==='ready')readyResolve(m);else if(m.type==='failed')readyReject(Error(m.error));else if(m.type==='held')heldResolve?.();else if(pending.has(m.id)){const p=pending.get(m.id);pending.delete(m.id);m.error?p.reject(Error(m.error)):p.resolve(m.value);}});
 const command=(command)=>new Promise((resolve,reject)=>{const id=randomUUID();pending.set(id,{resolve,reject});child.send({id,command});});
 const peers=[];let agent,fixture,results;
 try{
  fixture=await ready;agent=new Agent({keepAlive:true,maxSockets:120,ca:Buffer.from(fixture.ca,'base64'),checkServerIdentity:(_host,cert)=>checkServerIdentity('127.0.0.1',cert)});
  const {users}=fixture;
  const envelope=(u,payload,rev=lobby.revision)=>({...fixture.base,requestId:randomUUID(),revision:rev,sessionEpoch:u.desktop.sessionEpoch,...payload});
  const scoped=u=>({roomId:u.room.roomId,runId:u.room.runId,runEpoch:u.room.runEpoch,configRevision:u.room.configRevision});
  const sendHttp=(u,path,q)=>new Promise(resolve=>{
   const at=performance.now(),body=JSON.stringify(q);let settled=false;
   const finish=v=>{if(!settled){settled=true;resolve({...v,ms:performance.now()-at,requestBytes:Buffer.byteLength(body)});}};
   const req=request(new URL(path,fixture.origin),{method:'POST',agent,headers:{authorization:'Bearer '+u.desktop.accessToken,'content-type':'application/json','content-length':Buffer.byteLength(body)}},res=>{let body='',bytes=0;res.on('data',b=>{bytes+=b.length;if(bytes<=1048576)body+=b;else req.destroy(Error('RESPONSE_BOUND'));});res.on('end',()=>{try{const v=JSON.parse(body);finish({code:v.error?.code??(res.statusCode===200?'OK':'HTTP_'+res.statusCode),bytes,playing:v.canonicalAdmission?.decision?.playing});}catch{finish({code:'INVALID_JSON',bytes});}});res.on('error',()=>finish({code:'RESPONSE_ERROR',bytes}));});
   req.setTimeout(3000,()=>req.destroy(Error('DEADLINE')));req.on('error',e=>finish({code:e.message==='DEADLINE'?'CLIENT_DEADLINE':'NETWORK_ERROR',bytes:0}));req.end(body);
  });
  for(const u of users){
   const socket=new WebSocket(fixture.origin.replace('https:','wss:')+'/v3/live','nt2.g3.live.v1',{agent,perMessageDeflate:false});
   const p={socket,sequence:0,pose:0,generation:0,stats:{frames:0,bytes:0,snapshots:0,peerRecords:0,poseRecords:0,nonempty:0,retire:{},decodeErrors:0,foreignPeers:0},base:{major:1,revision:live.revision,serviceId:fixture.base.serviceId,serviceEpoch:fixture.base.serviceEpoch,connectionId:randomUUID(),clientGeneration:1,scope:{...u.scope,principalId:u.game.principal.principalId,sessionId:u.game.sessionId,sessionEpoch:u.game.sessionEpoch}},allowed:new Set(users.filter(x=>x.room.roomId===u.room.roomId).map(x=>x.game.sessionId))};
   peers.push(p);let welcome;const authenticated=new Promise((resolve,reject)=>{welcome=resolve;socket.once('error',reject);setTimeout(()=>reject(Error('WSS_AUTH_TIMEOUT')),5000).unref();});
   p.send=payload=>{if(socket.readyState===1)socket.send(Buffer.from(encode('ClientFrame',{...p.base,sequence:++p.sequence,authorityGeneration:p.generation,...payload})),{binary:true});};
   socket.on('error',()=>{});socket.on('message',data=>{const s=p.stats;s.frames++;s.bytes+=data.length;try{const snapshot=observeFrame(data,p);if(!snapshot||performance.now()>=(p.decodeNext??0)){const v=decode('ServerFrame',data);if(snapshot)p.decodeNext=performance.now()+5000;if(v.scope?.roomId!==p.base.scope.roomId)s.foreignPeers++;if(v.welcome){p.generation=v.authorityGeneration;welcome();}if(v.retire)bump(s.retire,v.retire.code);}}catch{s.decodeErrors++;}});
   await new Promise((r,j)=>{socket.once('open',r);socket.once('error',j);});p.send({authenticate:{accessToken:u.game.accessToken}});await authenticated;
  }
  results={revision:2,at:new Date().toISOString(),namespace:fixture.namespace,privateFixtureCaPath:fixture.caPath,distribution:{rooms:roomCount,playersPerRoom:size,total:users.length},substitutions:['Synthetic complete engine observations; no native/game rendering','Setup clock advances 1ms per HTTP call and 1s after bounded CAPACITY refusals; measurement uses elapsed wall time with no such advances or retries','Guest issuance uses actual Auth.guest/SQL internally during setup only; measured HTTP traffic retains all production budgets','TLS verified loopback; load generator separate process from service; no WAN simulation','9Hz offered movement provides margin below production 10/sec traffic limit','All frame bytes/peer-session IDs/pose counts inspected with bounded wire reader; full production snapshot decoder sampled per-client every 5s and all control frames'],phases:[]};
  async function phase(name,full=false,inject=false){
   const before=await command('begin');for(const p of peers)p.stats={frames:0,bytes:0,snapshots:0,peerRecords:0,poseRecords:0,nonempty:0,retire:{},decodeErrors:0,foreignPeers:0};
   const at=performance.now(),cpuStart=process.cpuUsage(),eluStart=performance.eventLoopUtilization(),end=at+seconds*1000,requests=[],byKind={},perRoom={},jobs=new Set();let skipped=0,maxInflight=0,offered=0,poseOffered=0,lateMax=0;
   const startJob=(u,kind,fn)=>{offered++;const job=fn().then(v=>{requests.push(v.ms);const k=byKind[kind]??={latencies:[],codes:{},bytes:0,requestBytes:0,notPlaying:0};k.latencies.push(v.ms);bump(k.codes,v.code);k.bytes+=v.bytes;k.requestBytes+=v.requestBytes;if(kind==='reconcile'&&v.code==='OK'&&v.playing!==true)k.notPlaying++;const r=perRoom[u.room.roomId]??={latencies:[],codes:{}};r.latencies.push(v.ms);bump(r.codes,v.code);return v;}).finally(()=>jobs.delete(job));jobs.add(job);maxInflight=Math.max(maxInflight,jobs.size);return job;};
   const runners=users.map((u,i)=>({u,index:i,next:at+i*2000/users.length,busy:new Set(),poseNext:at+i*(1000/9)/users.length}));
   let holdPromise,holdStarted=false;
   while(performance.now()<end){const t=performance.now();
    if(inject&&!holdStarted&&t>=at+2000){holdStarted=true;holdPromise=command('hold').then(value=>value,error=>({refused:error.message}));}
    for(const r of runners){
     if(t>=r.poseNext){if(peers[r.index].socket.readyState===1){peers[r.index].send({pose:{sequence:++peers[r.index].pose,xMilli:String(peers[r.index].pose*10),yMilli:'2000',vxMilli:'0',vyMilli:'0'}});poseOffered++;}r.poseNext+=1000/9;if(r.poseNext<t)r.poseNext=t+1000/9;}
     if(t<r.next)continue;lateMax=Math.max(lateMax,t-r.next);r.next+=2000;
     const tasks=[['reconcile',()=>sendHttp(r.u,'/v3/lifecycle/rpc',envelope(r.u,{reconcile:r.u.reconcile},lifecycle.revision))],['presence',()=>sendHttp(r.u,'/v1/rpc',envelope(r.u,{...scoped(r.u),presence:{present:true}}))]];
     if(full)tasks.push(['optional',async()=>{let last;for(const payload of [{roomList:{limit:32}},{...scoped(r.u),roomGet:{}},{...scoped(r.u),chatList:{limit:32}},{...scoped(r.u),inspectionList:{limit:1}}]){last=await startJob(r.u,Object.keys(payload).at(-1),()=>sendHttp(r.u,'/v1/rpc',envelope(r.u,payload)));if(last.code!=='OK')break;}return last;}]);
     for(const [kind,fn] of tasks){if(r.busy.has(kind)){skipped++;continue;}r.busy.add(kind);const job=(kind==='optional'?fn():startJob(r.u,kind,fn)).finally(()=>r.busy.delete(kind));if(kind==='optional'){jobs.add(job);void job.finally(()=>jobs.delete(job));}}
    }
    await sleep(3);
   }
   await Promise.all([...jobs]);const injection=await holdPromise;const elapsedMs=performance.now()-at,service=await command('metrics'),generatorCpu=process.cpuUsage(cpuStart);
   for(const value of Object.values(byKind)){value.latencyMs=summarize(value.latencies);delete value.latencies;}
   const rooms=Object.values(perRoom).map(r=>({latencyMs:summarize(r.latencies),codes:r.codes}));
   const movement=peers.reduce((a,p)=>{for(const k of ['frames','bytes','snapshots','peerRecords','poseRecords','nonempty','decodeErrors','foreignPeers'])a[k]+=p.stats[k];for(const [k,v]of Object.entries(p.stats.retire))a.retire[k]=(a.retire[k]??0)+v;return a;},{frames:0,bytes:0,snapshots:0,peerRecords:0,poseRecords:0,nonempty:0,decodeErrors:0,foreignPeers:0,retire:{}});
   const measured={name,elapsedMs,offered,completed:requests.length,achievedRequestsPerSecond:requests.length/elapsedMs*1000,skippedPerClientWork:skipped,maxInflight,schedulerMaxLatenessMs:lateMax,generator:{cpuMs:(generatorCpu.user+generatorCpu.system)/1000,cpuOneCorePercent:(generatorCpu.user+generatorCpu.system)/elapsedMs/10,eventLoopUtilization:performance.eventLoopUtilization(eluStart).utilization},...(injection?{injection}:{}),latencyMs:summarize(requests),byKind,rooms,movement:{...movement,poseOffered,poseOfferedPerUserHz:poseOffered/users.length/(seconds),snapshotBytesPerSecond:movement.bytes/elapsedMs*1000,minimumSnapshotsPerClient:Math.min(...peers.map(p=>p.stats.snapshots)),openSockets:peers.filter(p=>p.socket.readyState===1).length},service,liveDelta:{receivedPoses:service.live.receivedPoses-before.live.receivedPoses,snapshotsSent:service.live.snapshotsSent-before.live.snapshotsSent,retired:service.live.retired-before.live.retired}};
   results.phases.push(measured);console.log(JSON.stringify({capacityPhase:name,distribution:results.distribution,throughput:measured.achievedRequestsPerSecond,p99:measured.latencyMs.p99,codes:Object.fromEntries(Object.entries(byKind).map(([k,v])=>[k,v.codes])),live:measured.liveDelta}));
  }
  await phase('reconcile-presence-movement');await phase('ordinary-with-optional',true);await phase('other-room-650ms-transaction',false,true);
 }finally{
  for(const p of peers){if(p.socket.readyState===1)p.socket.close();else p.socket.terminate();}await sleep(100);agent?.destroy();
  if(!exited){const exit=new Promise(r=>child.once('exit',r));await command('close');await exit;}
  if(results){results.cleanup={workerClosed:true,ownedSchemaAbsenceReported:verifiedAbsence};const path=new URL(`../../../docs/g3/server-lc1-capacity-${roomCount}x${size}.json`,import.meta.url);await writeFile(path,JSON.stringify(results,null,2)+'\n');console.log(JSON.stringify({capacityResult:fileURLToPath(path)}));}
 }
}
if(process.argv[2]==='--worker')await worker();else await parent();
