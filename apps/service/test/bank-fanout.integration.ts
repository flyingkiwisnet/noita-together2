import pg from 'pg';
// Service-only load check: actual issuer/TLS/WSS/SQL, simulated playing reports.
// Four rooms x30 authenticated sessions. No native/game/save or WAN capacity claim.
import test from 'node:test';import assert from 'node:assert/strict';import {randomUUID} from 'node:crypto';
import {startG3Fixture,provisionG3Fixture} from '../scripts/g3-fixture.js';import {livePeer} from '../scripts/g3-live-fixture.js';
import {markBankDirty,bankRevision} from '../src/g3/bank-events.js';import {schema} from '../../../packages/contracts/src/g3.js';
const pause=(ms:number)=>new Promise<void>(r=>setTimeout(r,ms));
test('120 actual authenticated subscribers: idle bank SQL zero, four shared balances, burst latest coalescing and healthy control',async()=>{
 const databaseUrl='postgres://nt2_g1:local-fixture-only@127.0.0.1:54328/nt2_g1';let now=Date.now();const f=await startG3Fixture({live:true,tls:true,now:()=>now,databaseUrl}),rooms:Awaited<ReturnType<typeof provisionG3Fixture>>[]=[],peers:Awaited<ReturnType<typeof livePeer>>[]=[];
 let restore=()=>{};const call=f.call;
 console.log(JSON.stringify({allocatedFixtureSchema:f.namespace,databaseTarget:'127.0.0.1:54328/nt2_g1'}));
 // Monotonic fixture clock advances at each RPC, respecting anonymous20/s
 // without expiring early actors during long simulated roster provisioning.
 f.call=async(...args:Parameters<typeof f.call>)=>{now+=(args[1] as any)?.guestIssue?120:50;return call(...args);};
 try{
  for(let room=0;room<4;room++){
   const s=await provisionG3Fixture(f,{playerCount:30,devicePrefix:'bank-fanout-'+room+'-'+randomUUID().slice(0,8),roomName:'Bank fanout '+room});rooms.push(s);await s.start();await s.playing();
   for(const ready of rooms)await ready.pulse();
  }
  for(const s of rooms)for(let i=0;i<30;i++)peers.push(await livePeer(f,s,i));
  for(const p of peers)await p.wait(v=>!!v.bankChanged);assert.equal(peers.length,120);
  const db=f.service.db;let bankSelects=0,balanceSelects=0,totalBankSelects=0;
  await db.transaction(async tx=>{const prototype=Object.getPrototypeOf(tx),query=prototype.query;prototype.query=function(...args:any[]){const sql=typeof args[0]==='string'?args[0]:args[0]?.text??'';if(/^SELECT.*g3_bank_versions/.test(sql)){bankSelects++;totalBankSelects++;}if(sql.startsWith('SELECT amount,version FROM g3_balances'))balanceSelects++;return query.apply(this,args);};restore=()=>{prototype.query=query;};});
  const bank=async(room:number,player:number)=>{const s=rooms[room]!,c=s.games[player]!;const response=await f.call('/v3/rpc',{major:1,revision:schema.revision,serviceId:db.serviceId,serviceEpoch:db.epoch,sessionEpoch:c.sessionEpoch,requestId:randomUUID(),scope:s.scopes[player],goldBank:{}},c.accessToken);const value=JSON.parse(response.body);assert.equal(response.status,200);assert.equal(value.error,undefined);return value;};
  // Warm each public revision, including initial balance-row creation. Private
  // originals/notices stay independently authorized and queried per caller.
  for(let r=0;r<4;r++){await bank(r,0);await bank(r,1);}balanceSelects=0;
  for(let r=0;r<4;r++)for(let p=0;p<30;p++)assert.equal((await bank(r,p)).bank.availableBalance,0);
  assert.equal(balanceSelects,0,'120 caller-authorized reads reuse four committed balance values while retaining row locks');
  bankSelects=0;await pause(700);assert.equal(bankSelects,0,'idle sockets/heartbeat snapshots perform no bank SQL');
  peers.forEach(p=>p.received.splice(0));const started=Date.now();
  const versions=await db.transaction(async tx=>{
   for(const s of rooms){const runId=s.scopes[0]!.runId!;for(let i=0;i<20;i++)await tx.query('UPDATE g3_balances SET amount=amount+1,version=version+1 WHERE run_id=$1',[runId]);markBankDirty(tx,runId);}
   const values:string[]=[];for(const s of rooms)values.push(await bankRevision(tx,s.scopes[0]!.runId!));return values;
  });
  for(let i=0;i<120;i++)await peers[i]!.wait(v=>v.bankChanged?.revision===versions[Math.floor(i/30)],2000);
  const visibility=Math.max(...peers.map((p,i)=>p.received.find(r=>r.frame.bankChanged?.revision===versions[Math.floor(i/30)])!.at))-started;
  assert.ok(visibility<2000);assert.ok(peers.every(p=>p.received.filter(r=>r.frame.bankChanged).length===1),'20 mutations per room coalesce to one latest revision per socket');
  peers[1]!.send({pose:{sequence:1,xMilli:'1',yMilli:'2',vxMilli:'0',vyMilli:'0'}});await peers[2]!.wait(v=>v.snapshot?.peers?.some(p=>p.sessionId===peers[1]!.scope.sessionId)??false,1000);
  peers[2]!.send({ping:{}});await peers[2]!.wait(v=>!!v.pong,1000);assert.ok(peers.every(p=>p.socket.readyState===1));
  console.log(JSON.stringify({subscribed:120,rooms:4,publicBalanceReadsFor120AfterWarm:balanceSelects,idleBankSelects:0,burstMutations:80,bankNotifications:120,commitToVisibleUpperMs:visibility,bankRevisionSelects:totalBankSelects,limitsUnchanged:true,actualTlsWssSql:true,engineFactsInjected:true}));
 }finally{restore();for(const p of peers)await p.close();await f.close();const pool=new pg.Pool({connectionString:databaseUrl});try{assert.equal((await pool.query('SELECT to_regnamespace($1) AS schema',[f.namespace])).rows[0].schema,null);console.log(JSON.stringify({verifiedAbsentSchema:f.namespace}));}finally{await pool.end();}}
});
