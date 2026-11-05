import test from'node:test';import assert from'node:assert/strict';import{DatabaseWorkScheduler}from'../src/g1/work.js';import{Database}from'../src/g1/database.js';import{EventEmitter}from'node:events';
const signal=()=>new AbortController();const turn=()=>new Promise<void>(r=>setImmediate(r));
test('optional saturation leaves two control places, total stays7 and only one dispatches',async()=>{
 const s=new DatabaseWorkScheduler(7),c=signal(),first=await s.enter('optional',c.signal),optional=Array.from({length:4},()=>s.enter('optional',c.signal));await assert.rejects(s.enter('optional',c.signal),/CAPACITY/);
 const controls=[s.enter('control',c.signal),s.enter('control',c.signal)];assert.equal(s.stats.active,7);assert.equal(s.stats.queued,6);assert.equal(s.stats.executing,'optional');await assert.rejects(s.enter('control',c.signal),/CAPACITY/);
 first();const a=await controls[0]!;assert.equal(s.stats.executing,'control');a();const b=await controls[1]!;b();for(const pending of optional)(await pending)();assert.equal(s.stats.active,0);assert.equal(s.stats.peak,7);
});
test('nonpreemptive control preference dispatches optional after at most three controls',async()=>{
 const s=new DatabaseWorkScheduler(7),c=signal(),first=await s.enter('optional',c.signal),optional=s.enter('optional',c.signal),controls=Array.from({length:4},()=>s.enter('control',c.signal));first();
 for(let i=0;i<3;i++){const release=await controls[i]!;assert.equal(s.stats.executing,'control');release();}const release=await optional;assert.equal(s.stats.executing,'optional');release();(await controls[3]!)();assert.equal(s.stats.active,0);
});
test('smaller budgets retain an optional place and reserve only available control places',async()=>{
 for(const limit of [1,2,3]){const s=new DatabaseWorkScheduler(limit),c=signal(),first=await s.enter('optional',c.signal);assert.equal(s.stats.reservedControl,limit-1);await assert.rejects(s.enter('optional',c.signal),/CAPACITY/);const pending=Array.from({length:limit-1},()=>s.enter('control',c.signal));first();for(const p of pending)(await p)();assert.equal(s.stats.active,0);}
});
test('queued cancellation, close and dispatch/abort races release once without dispatching cancelled work',async()=>{
 const s=new DatabaseWorkScheduler(7),c=signal(),first=await s.enter('optional',c.signal),cancel=signal(),p=s.enter('control',cancel.signal),rejected=assert.rejects(p,/SERVICE_UNAVAILABLE/);cancel.abort();await rejected;assert.equal(s.stats.queued,0);assert.equal(s.stats.active,1);
 const next=s.enter('control',c.signal);first();const release=await next;release();release();assert.equal(s.stats.active,0);
 const hold=await s.enter('optional',c.signal),queued=s.enter('optional',c.signal),closed=assert.rejects(queued,/SERVICE_UNAVAILABLE/);s.close();await closed;assert.equal(s.stats.active,1);hold();assert.equal(s.stats.active,0);await assert.rejects(s.enter('control',c.signal),/SERVICE_UNAVAILABLE/);
});
function memoryDatabase(transactionMs=2500,maxWork=7){
 const db=new Database('postgres://unused/unused','memory','g3_memory',{transactionMs,maxWork}),clients:any[]=[];let connects=0,ended=false;const pool:any=db.pool;
 pool.connect=async()=>{connects++;const tx:any=new EventEmitter();tx.releases=0;tx.query=async(sql:string)=>({command:sql==='COMMIT'?'COMMIT':sql==='ROLLBACK'?'ROLLBACK':'SELECT',rows:sql==='SELECT epoch FROM service_identity'?[{epoch:'epoch'}]:[]});tx.release=()=>{tx.releases++;};clients.push(tx);return tx;};pool.end=async()=>{ended=true;};(db as any).healthy=true;db.epoch='epoch';return{db,clients,connects:()=>connects,ended:()=>ended};
}
test('Database queues before pool acquisition and cancels queued callbacks without SQL or permit leaks',async()=>{
 const f=memoryDatabase();let finish!:()=>void,called=false;const held=f.db.transaction(async()=>new Promise<void>(r=>{finish=r;}));await turn();const c=signal(),queued=f.db.transaction(async()=>{called=true;},true,c.signal,'control'),rejected=assert.rejects(queued,/SERVICE_UNAVAILABLE/);assert.equal(f.connects(),1);c.abort();await rejected;assert.equal(called,false);assert.equal(f.connects(),1);finish();await held;assert.equal(f.db.workStats.active,0);assert.ok(f.clients.every(c=>c.releases===1));await f.db.close();
});
test('transaction deadline includes queue wait and expiry-before-dispatch never invokes callback',async()=>{
 const f=memoryDatabase(30);let called=false;const pool:any=f.db.pool,connect=pool.connect;pool.connect=()=>new Promise(resolve=>setTimeout(()=>{void connect().then(resolve);},65));
 const held=f.db.transaction(async()=>{called=true;}),heldRejection=assert.rejects(held,/SERVICE_UNAVAILABLE/);await turn();const queued=f.db.transaction(async()=>{called=true;}),queuedRejection=assert.rejects(queued,/SERVICE_UNAVAILABLE/);await queuedRejection;assert.equal(called,false);await heldRejection;assert.equal(called,false);assert.equal(f.db.workStats.active,0);assert.equal(f.connects(),1);assert.ok(f.clients.every(c=>c.releases===1));await f.db.close();
});
test('close removes queued work and aborts executing work before pool end, including dispatch race',async()=>{
 const f=memoryDatabase();let finish!:()=>void,called=false;const held=f.db.transaction(async()=>new Promise<void>(r=>{finish=r;})),a=assert.rejects(held,/SERVICE_UNAVAILABLE/);await turn();const queued=f.db.transaction(async()=>{called=true;},true,undefined,'control'),b=assert.rejects(queued,/SERVICE_UNAVAILABLE/);await f.db.close();await Promise.all([a,b]);finish();await turn();assert.equal(called,false);assert.equal(f.ended(),true);assert.equal(f.db.workStats.active,0);assert.ok(f.clients.every(c=>c.releases===1));
 const race=memoryDatabase(),abort=signal(),pending=race.db.transaction(async()=>{called=true;},true,abort.signal,'control'),rejected=assert.rejects(pending,/SERVICE_UNAVAILABLE/);abort.abort();await rejected;assert.equal(called,false);assert.equal(race.connects(),0);assert.equal(race.db.workStats.active,0);await race.db.close();
});
