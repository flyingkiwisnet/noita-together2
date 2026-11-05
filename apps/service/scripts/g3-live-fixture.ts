import {Agent} from 'node:https';
import {randomUUID} from 'node:crypto';
import {WebSocket} from 'ws';
import {checkServerIdentity} from 'node:tls';
import {startG3Fixture,provisionG3Fixture} from './g3-fixture.js';
import {encode,decode,schema} from '../../../packages/contracts/src/g3-live.js';
import type {ClientFrame,ServerFrame,Scope} from '../../../packages/contracts/g3-live/g3-live.js';
const sleep=(ms:number)=>new Promise<void>(r=>setTimeout(r,ms));

export async function livePeer(f:Awaited<ReturnType<typeof startG3Fixture>>,s:Awaited<ReturnType<typeof provisionG3Fixture>>,i:number,options:{agent?:Agent}={}){
 const game=s.games[i]!,scope:Scope={...s.scopes[i]!,principalId:game.principal!.principalId!,sessionId:game.sessionId!,sessionEpoch:game.sessionEpoch!};
 const agent=options.agent??new Agent({ca:f.ca!,checkServerIdentity:(_host,cert)=>checkServerIdentity('127.0.0.1',cert)});
 const socket=new WebSocket(f.origin.replace('https:','wss:')+'/v3/live','nt2.g3.live.v1',{agent,perMessageDeflate:false});
 const received:{frame:ServerFrame;at:number}[]=[],base={major:1,revision:schema.revision,serviceId:f.service.db.serviceId,serviceEpoch:f.service.db.epoch,connectionId:randomUUID(),clientGeneration:1,scope};let sequence=0,generation=0;
 socket.on('error',()=>{});socket.on('message',data=>{const frame=decode('ServerFrame',data as Buffer) as ServerFrame;received.push({frame,at:Date.now()});if(frame.welcome)generation=frame.authorityGeneration!;});
 const send=(payload:Partial<ClientFrame>)=>socket.send(Buffer.from(encode('ClientFrame',{...base,sequence:++sequence,authorityGeneration:generation,...payload})),{binary:true});
 const wait=async(predicate:(f:ServerFrame)=>boolean,ms=3500)=>{const until=Date.now()+ms;while(Date.now()<until){const found=received.find(r=>predicate(r.frame));if(found)return found.frame;if(socket.readyState===3)break;await sleep(10);}throw Error('Live frame timeout; '+JSON.stringify(received.map(r=>r.frame.retire??r.frame.welcome??'snapshot')));};
 try{await new Promise<void>((resolve,reject)=>{socket.once('open',resolve);socket.once('error',reject);});send({authenticate:{accessToken:game.accessToken!}});await wait(v=>!!v.welcome);}catch(e){socket.terminate();if(!options.agent)agent.destroy();throw e;}
 return {socket,scope,received,send,wait,async close(){socket.terminate();if(!options.agent)agent.destroy();await sleep(10);}};
}

