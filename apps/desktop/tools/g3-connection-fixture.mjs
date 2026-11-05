// Disposable one-issuer fixture only. Core still owns every lifecycle request.
import {createInterface} from 'node:readline';
import {createServer,connect} from 'node:net';
import {startG3Fixture as startLegacyG3Fixture} from '../../service/scripts/g3-fixture.ts';
const rejoin=process.argv.includes('--rejoin');
const loot01=rejoin||process.argv.includes('--loot01-runtime');
const startG3Fixture=loot01?(await import('../../service/scripts/g3-loot01-fixture.ts')).startG3Fixture:startLegacyG3Fixture;
let offset=0,optional='none',holdGame=false,dropChatReply=false;
let pollTrace={chatList:0,inspectionList:0,roomGet:0,chatSendIds:[]};
const c3=process.argv.includes('--c3');
const potionCorrection=process.argv.includes('--potion-correction');
const potionDiagnostic=process.argv.includes('--potion-diagnostic');
const fixture=await startG3Fixture({...(rejoin?{rejoin:true,rejoinInput:{path:process.env.NT2_REJOIN_INPUT,sha256:process.env.NT2_REJOIN_INPUT_SHA}}:{}),live:process.argv.includes('--live'),tls:true,...(loot01?{loot01Runtime:true}:process.argv.includes('--seed-loot-runtime')?{seedLootRuntime:true}:potionCorrection?{potionCorrection:true}:potionDiagnostic?{potionDiagnostic:true}:c3?{c2Runtime:true}:{social:true}),now:()=>Date.now()+offset});
const timers=new Set();
// Bounded encrypted TCP delay proxy: TLS and canonical messages remain real.
// Pipeline bytes by arrival time, preserving order without serial frame sleeps.
let rtt=0,packet=0,holdAuthNext=false;const sockets=new Set(),channels=new Map();let proxy;
if(process.argv.includes('--live')&&!rejoin){
 const origin=new URL(fixture.origin),server=fixture.service.app.server;
 // Keep the issuer/Host/TLS origin exact: move ONLY this disposable listener
 // behind the proxy, which occupies its original port. No trust checks change.
 await new Promise((resolve,reject)=>server.close(error=>error?reject(error):resolve()));
 await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));const upstreamPort=server.address().port;
 server.prependListener('upgrade',(req,socket)=>{if(req.url==='/v3/live'){const channel=channels.get(socket.remotePort);if(channel){channel.live=true;if(holdAuthNext){channel.hold=true;holdAuthNext=false;}}}});
 proxy=createServer(client=>{
  const remote=connect(upstreamPort,'127.0.0.1'),channel={hold:false,live:false,client,remote};sockets.add(client);sockets.add(remote);remote.on('connect',()=>channels.set(remote.localPort,channel));remote.on('close',()=>{for(const [port,current] of channels){if(current===channel)channels.delete(port);}});
  const forward=(source,target)=>{
   let queue=[],bytes=0,timer,lastDue=0,blocked=false;
   const clear=()=>{if(timer){clearTimeout(timer);timers.delete(timer);}queue=[];bytes=0;};
   const flush=()=>{
    if(timer){timers.delete(timer);timer=undefined;}if(blocked||target.destroyed)return;
    while(queue.length&&queue[0].due<=Date.now()){const item=queue.shift();bytes-=item.chunk.length;if(!target.write(item.chunk)){blocked=true;target.once('drain',()=>{blocked=false;flush();});break;}}
    if(queue.length&&!blocked){timer=setTimeout(flush,Math.max(1,queue[0].due-Date.now()));timers.add(timer);}
   };
   source.on('data',chunk=>{if(queue.length>=64||bytes+chunk.length>1048576){source.destroy();target.destroy();return;}const jitter=rtt?[-8,0,8][packet++%3]:0;let extra=0;if(source===client&&channel.hold){channel.hold=false;extra=1000;console.log(JSON.stringify({event:'held-auth'}));}lastDue=Math.max(lastDue,Date.now()+rtt/2+jitter+extra);queue.push({chunk,due:lastDue});bytes+=chunk.length;if(!timer&&!blocked)flush();});
   source.on('close',()=>{clear();target.destroy();sockets.delete(source);});source.on('error',()=>target.destroy());
  };forward(client,remote);forward(remote,client);
 });await new Promise(resolve=>proxy.listen(Number(origin.port),'127.0.0.1',resolve));
}
fixture.service.app.server.prependListener('request',(req,res)=>{
 if(!['/v1/rpc','/v3/social/rpc'].includes(req.url))return;
 // Fault selection only; production issuer still authenticates the token.
 let kind;try{kind=JSON.parse(Buffer.from(req.headers.authorization?.split(' ')[1]?.split('.')[1]??'','base64url')).kind;}catch{}
 let chunks=[],size=0,message;
 const emit=req.emit.bind(req);
 req.emit=function(event,...args){if(event==='data'){size+=args[0].length;if(size<=65536)chunks.push(args[0]);}return emit(event,...args);};
 req.on('end',()=>{if(size<=65536){try{message=JSON.parse(Buffer.concat(chunks).toString());}catch{}}chunks=[];if(message?.chatList)pollTrace.chatList++;if(message?.inspectionList)pollTrace.inspectionList++;if(message?.roomGet&&kind==='desktop')pollTrace.roomGet++;if(message?.chatSend&&pollTrace.chatSendIds.length<16)pollTrace.chatSendIds.push(message.requestId);});
 const end=res.end.bind(res);
 res.end=function(...args){
  if(dropChatReply&&message?.chatSend){dropChatReply=false;req.socket.destroy();return res;} 
  const isOptional=req.url==='/v3/social/rpc'||message?.chatList||message?.inspectionList||message?.roomList||message?.roomGet&&kind==='desktop';
  if(isOptional&&optional!=='none'){
   console.log(JSON.stringify({event:'optional',mode:optional}));
   if(optional==='500'){res.statusCode=500;return end('{}');}
   if(optional==='malformed')return end('{');
   if(optional==='capacity'){const body=JSON.parse(args[0].toString());delete body.chatMessages;delete body.inspections;body.error={code:'CAPACITY'};return end(JSON.stringify(body));}
   const timer=setTimeout(()=>{timers.delete(timer);end(...args);},optional==='timeout'?6000:1500);timers.add(timer);return res;
  }
  if(holdGame&&message?.roomGet){holdGame=false;console.log(JSON.stringify({event:'held-game'}));const timer=setTimeout(()=>{timers.delete(timer);end(...args);},800);timers.add(timer);return res;}
  return end(...args);
 };
});
console.log(JSON.stringify({event:'ready',origin:proxy?`https://127.0.0.1:${proxy.address().port}`:fixture.origin,caPath:fixture.caPath,namespace:fixture.namespace}));
const input=createInterface({input:process.stdin});
try{await new Promise(resolve=>{
 const lifetime=setTimeout(resolve,180000);lifetime.unref();
 input.on('line',async line=>{
  if(line==='quit'){resolve();return;}
  if(line.startsWith('advance:')){const step=Number(line.slice(8));if(!Number.isSafeInteger(step)||step<0||step>20000)throw Error('Bounded clock step required');offset+=step;}
  else if(line.startsWith('participant-loss:')){await fixture.loseParticipant(line.slice(17));}
  else if(line.startsWith('session-loss:')){await fixture.setSessionLoss(line.slice(13),true);}
  else if(line.startsWith('session-restore:')){await fixture.setSessionLoss(line.slice(16),false);}
  else if(line==='restart'){await fixture.restart();}
  else if(line==='drop-chat-reply'){dropChatReply=true;}
  else if(line==='poll-trace-reset'){pollTrace={chatList:0,inspectionList:0,roomGet:0,chatSendIds:[]};}
  else if(line==='poll-trace'){console.log(JSON.stringify({event:line,trace:pollTrace}));return;}
  else if(line.startsWith('optional:')){const mode=line.slice(9);if(!['none','500','capacity','malformed','slow','timeout'].includes(mode))throw Error('Unknown fault');optional=mode;}
  else if(line==='hold-game-next'){holdGame=true;}
  else if(line.startsWith('rtt:')){const value=Number(line.slice(4));if(![0,50,100,200].includes(value))throw Error('Bounded RTT required');rtt=value;}
  else if(line==='disconnect-live'){for(const channel of channels.values()){if(channel.live){channel.client.destroy();channel.remote.destroy();}}}
  else if(line==='hold-auth-next'){holdAuthNext=true;}
  else throw Error('Unknown fixture command');
  console.log(JSON.stringify({event:line,offset}));
 });input.on('close',resolve);
});}finally{input.close();for(const timer of timers)clearTimeout(timer);for(const socket of sockets)socket.destroy();if(proxy)await new Promise(resolve=>proxy.close(resolve));await fixture.close();}
