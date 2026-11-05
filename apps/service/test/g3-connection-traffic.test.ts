import test from 'node:test';import assert from 'node:assert/strict';import Fastify from 'fastify';import {generateKeyPairSync} from 'node:crypto';
import {TrafficBudgets,installTrafficBudgets,explicitTrustedProxies,TRAFFIC_LIMITS} from '../src/g2-next/traffic.js';
function charge(b:TrafficBudgets,a:{sessionId:string;principalId:string},room:string,kind:Parameters<TrafficBudgets['authenticated']>[1]){b.authenticated(a,kind);b.authorizedRoom(room);}
import {Security} from '../src/g1/security.js';
test('8/16/32 shared-NAT actors across rooms sustain cadence; abusive actor and finite maps refuse',()=>{
 let now=0;
 for(const players of [8,16,32]){
  const b=new TrafficBudgets(()=>now);
  for(let second=0;second<10;second++){
   now=second*1000;
   for(let tick=0;tick<10;tick++)for(let i=0;i<players;i++){now=second*1000+tick*100;
    const a={sessionId:'session'+i,principalId:'principal'+i},room='room'+i%3;
    charge(b,a,room,'pose');charge(b,a,room,'read');if(tick<4)charge(b,a,room,'social');if(tick<2)charge(b,a,room,'health');
   }
  }
  assert.ok(b.size<TRAFFIC_LIMITS.keys);assert.throws(()=>{for(let i=0;i<3;i++)charge(b,{sessionId:'session0',principalId:'principal0'},'room0','pose');},/CAPACITY/);
  charge(b,{sessionId:'new',principalId:'new'},'room0','pose');
 }
 const bounded=new TrafficBudgets(()=>now,8);for(let i=0;i<8;i++)bounded.take('k'+i,1);assert.throws(()=>bounded.take('overflow',1),/CAPACITY/);now+=1000;bounded.take('fresh',1);assert.equal(bounded.size,1);
});
test('actual signed traffic hook exceeds old shared IP60 limit; spoofed forwarded IP cannot bypass login limit',async()=>{
 let now=Date.now();const pair=generateKeyPairSync('ed25519');
 const security=new Security({privateKey:pair.privateKey.export({type:'pkcs8',format:'pem'}).toString(),publicKey:pair.publicKey.export({type:'spki',format:'pem'}).toString(),recoveryKey:Buffer.alloc(32,1).toString('base64url'),issuer:'https://fixture.example'},'fixture');
 const app=Fastify({trustProxy:explicitTrustedProxies(undefined)});const traffic=installTrafficBudgets(app,security,()=>now);
 app.post('/rpc',async()=>({ok:true}));
 try{
  const tokens=await Promise.all(Array.from({length:32},(_,i)=>security.sign('p'+i,'s'+i,'e'+i,'game','j'+i,now)));
  for(let tick=0;tick<10;tick++)for(const token of tokens){now+=100/tokens.length;const r=await app.inject({method:'POST',url:'/rpc',headers:{authorization:'Bearer '+token},payload:{roomId:'room',movement:{}}});assert.equal(r.statusCode,200);}
  const burst=[];for(let i=0;i<3;i++)burst.push((await app.inject({method:'POST',url:'/rpc',headers:{authorization:'Bearer '+tokens[0]},payload:{movement:{}}})).statusCode);assert.equal(burst.at(-1),429);
  // Arbitrary body room IDs above leave the protected room's allowance untouched.
  for(let i=0;i<TRAFFIC_LIMITS.room;i++)traffic.authorizedRoom('room');
  assert.throws(()=>traffic.authorizedRoom('room'),/CAPACITY/);
  now+=1000;
  const tx={};traffic.authorizedRoom('room',tx);for(let i=0;i<TRAFFIC_LIMITS.room;i++)traffic.authorizedRoom('room',tx);
  for(let i=1;i<TRAFFIC_LIMITS.room;i++)traffic.authorizedRoom('room');assert.throws(()=>traffic.authorizedRoom('room'),/CAPACITY/);
  now+=1000;
  for(let i=0;i<21;i++){const r=await app.inject({method:'POST',url:'/rpc',headers:{'x-forwarded-for':'10.0.0.'+i},payload:{authBegin:{}}});assert.equal(r.statusCode,i<20?200:429);}
  assert.throws(()=>explicitTrustedProxies(['0.0.0.0/0']));assert.throws(()=>explicitTrustedProxies(['any']));assert.deepEqual(explicitTrustedProxies(['127.0.0.1']),['127.0.0.1']);
 }finally{await app.close();}
});
