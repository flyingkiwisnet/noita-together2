// Transport/one-issuer factory only. Rust Core drives sign-in, reports and Start.
import {createInterface} from 'node:readline';
import {createHash} from 'node:crypto';
import {startG3Fixture} from '../../service/scripts/g3-fixture.ts';
const fixture=await startG3Fixture({tls:true,...(process.argv.includes('--a1-runtime-diagnostic')?{a1RuntimeDiagnostic:true}:process.argv.includes('--a1-runtime-fix')?{a1RuntimeFix:true}:{}),...(process.argv.includes('--native-social')?{nativeSocial:true,a1Runtime:true}:process.argv.includes('--a1-primary')?{native:true,a1Runtime:true}:{social:process.argv.includes('--social')})});
// One bounded transport fault, around the real factory rather than a second service.
let delayNext=false,delaySocial=false,delayRoom=false,dropAttach=false,droppedAttachHash,badSocialInfo=false,held;
const traceEnabled=process.argv.includes('--movement-trace');
const movementTrace={movementAttempts:0,ghostAttempts:0,movementReplies:0,ghostReplies:0,failures:[]};
fixture.service.app.server.prependListener('request',(request,response)=>{
 if(request.url!=='/v3/rpc'&&request.url!=='/v3/social/rpc'&&request.url!=='/v1/rpc'&&request.url!=='/v3/social/info')return;
 const original=response.end.bind(response);
 const captured=[];let capturedBytes=0;
 let movementKind;
 if(dropAttach||droppedAttachHash||traceEnabled&&request.url==='/v3/rpc'){const emit=request.emit.bind(request);request.emit=function(event,...values){if(event==='data'){const chunk=values[0];capturedBytes+=chunk.length;if(capturedBytes<=65536)captured.push(chunk);}return emit(event,...values);};}
 request.on('end',()=>{
  if(traceEnabled&&request.url==='/v3/rpc'&&capturedBytes<=65536){
   let message;try{message=JSON.parse(Buffer.concat(captured).toString());}catch{}
   movementKind=message?.movement?'movement':message?.ghostList?'ghost':undefined;
   if(movementKind)movementTrace[movementKind+'Attempts']++;
  }
  if(!droppedAttachHash||capturedBytes>65536)return;
  const input=Buffer.concat(captured);let message;try{message=JSON.parse(input.toString());}catch{}
  if(message?.gameAttach){console.log(JSON.stringify({event:'replayed-attach',exactRequest:createHash('sha256').update(input).digest('hex')===droppedAttachHash}));droppedAttachHash=undefined;}
 });
 response.end=function(...args){
  const body=args[0];
  if(traceEnabled&&movementKind&&body&&Buffer.byteLength(body)<=65536){
   let result;try{result=JSON.parse(body.toString());}catch{}
   movementTrace[movementKind+'Replies']++;
   if(result?.error&&movementTrace.failures.length<16){
    const code=result.error.code;
    movementTrace.failures.push({stage:movementKind,code:['CAPACITY','STALE_EPOCH','STALE_REVISION','INVALID_STATE','FORBIDDEN','INVALID_MESSAGE','INCOMPATIBLE_GAME','CONFLICT'].includes(code)?code:'other',status:response.statusCode});
   }
  }
  if(badSocialInfo&&request.url==='/v3/social/info'&&body){
   const changed=JSON.parse(body.toString());changed.runtimeProfile.bridgeSha256='0'.repeat(64);badSocialInfo=false;
   return original(JSON.stringify(changed));
  }
  if((delayNext||delaySocial||delayRoom||dropAttach||droppedAttachHash)&&body&&(Buffer.isBuffer(body)||typeof body==='string')&&Buffer.byteLength(body)<=32768){
   let payload;try{payload=JSON.parse(body.toString());}catch{}
   if(payload?.credentials?.kind==='game'&&(dropAttach||droppedAttachHash)){
    const input=Buffer.concat(captured);let requestBody;try{requestBody=JSON.parse(input.toString());}catch{}
    if(capturedBytes<=65536&&requestBody?.gameAttach){
     const fingerprint=createHash('sha256').update(input).digest('hex');
     if(dropAttach){dropAttach=false;droppedAttachHash=fingerprint;response.destroy();console.log(JSON.stringify({event:'dropped-attach'}));return response;}
    }
   }
   if((delayNext&&payload?.ghosts)||(delaySocial&&payload?.projections)||(delayRoom&&payload?.room&&!payload?.credentials)){
    const kind=payload.ghosts?'ghosts':payload.projections?'social':'room';delayNext=false;delaySocial=false;delayRoom=false;const release=()=>{if(!held)return;clearTimeout(held.timer);held=undefined;original(...args);};
    held={release,timer:setTimeout(release,kind==='social'?2000:450)};console.log(JSON.stringify({event:'held',kind}));return response;
   }
  }
  return original(...args);
 };
});
try{
 console.log(JSON.stringify({event:'ready',origin:fixture.origin,caPath:fixture.caPath,namespace:fixture.namespace,evidenceStatus:'simulated-engine-profile-only'}));
 const input=createInterface({input:process.stdin});
 await new Promise(resolve=>{
  const timer=setTimeout(resolve,180000);timer.unref();
  input.on('line',line=>{
   const command=line.trim();if(command==='quit'){resolve();return;}
   if(command==='delay-next'){delayNext=true;console.log(JSON.stringify({event:command}));}
   else if(command==='movement-trace'){console.log(JSON.stringify({event:command,...movementTrace}));}
   else if(command==='delay-social-next'){delaySocial=true;console.log(JSON.stringify({event:command}));}
   else if(command==='delay-room-next'){delayRoom=true;console.log(JSON.stringify({event:command}));}
   else if(command==='drop-attach-next'){dropAttach=true;console.log(JSON.stringify({event:command}));}
   else if(command==='bad-social-info-next'){badSocialInfo=true;console.log(JSON.stringify({event:command}));}
   else if(command==='release'){held?.release();console.log(JSON.stringify({event:command}));}
  });input.on('close',resolve);
 });input.close();
}finally{held?.release();await fixture.close();}
