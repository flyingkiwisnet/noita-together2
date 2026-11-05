import test from 'node:test';import assert from 'node:assert/strict';import {randomUUID} from 'node:crypto';
import {startG3Fixture,provisionG3Fixture} from '../scripts/g3-fixture.js';import {NemesisAuthority} from '../src/g3/nemesis.js';import {nemesisF64Bits,type NemesisKillEvidence} from '../../../packages/domain/src/g3-nemesis-provenance.js';
import type {Tx} from '../src/g1/database.js';import type {Actor} from '../src/g1/auth.js';import type {Scope} from '../../../packages/contracts/g3-nemesis/g3-nemesis.js';
const kill=(hp=3.5):NemesisKillEvidence=>{const playerEntityId=randomUUID();return {killId:randomUUID(),victimId:randomUUID(),playerEntityId,responsibleEntityId:playerEntityId,responsibleParentId:null,maxHpBits:nemesisF64Bits(hp),nemesisEnemy:true};};
const group=()=>({groupId:randomUUID(),level:2,offers:[{offerId:randomUUID(),abilityId:'grounded'}]});
async function fixture(){let now=Date.now();const f=await startG3Fixture({nemesis:true,tls:true,now:()=>now});try{
 const state=await provisionG3Fixture(f,{beforeRpc:()=>{now+=25;}}),authority=new NemesisAuthority(()=>f.service.db.epoch,f.profile.compatibility.capabilities,()=>now);
 const as=async<T>(i:number,fn:(tx:Tx,actor:Actor,scope:Scope)=>Promise<T>,desktop=false)=>f.service.db.transaction(async tx=>{const actor=await f.service.auth.authenticate(tx,(desktop?state.desktops[i]:state.games[i])!.accessToken!);await authority.maintain(tx);return fn(tx,actor,state.scopes[i]!);});
 const earn=(i:number,evidence:NemesisKillEvidence)=>as(i,(tx,a,s)=>authority.recordKillFixture(tx,a,s,evidence));
 return {f,state,authority,as,earn,advance:()=>{now+=1100;},sql:(q:string,args:unknown[]=[])=>f.service.db.transaction(tx=>tx.query(q,args))};
 }catch(e){await f.close();throw e;}}
test('isolated zero-initial NP credits and existing canonical debits commit once under concurrent/lost-response originals and retain private status after End/restart',async()=>{
 const t=await fixture();try{
 const e=kill();await assert.rejects(t.earn(0,e));await t.state.start();await assert.rejects(t.earn(0,e));await t.state.playing();
 await t.earn(0,e); // Discard committed answer to model a lost internal adapter reply.
 const [a,b]=await Promise.all([t.earn(0,e),t.earn(0,e)]);assert.deepEqual(a,b);assert.equal(a.points,35);assert.equal(a.enemyBroadcastEligible,false);assert.equal((await t.sql('SELECT count(*)::int n FROM g3_nemesis_kills')).rows[0].n,1);
 const g=group();await t.as(0,(tx,a,s)=>t.authority.provisionOffersFixture(tx,a,s,g));const intent={operationId:randomUUID(),offerId:g.offers[0]!.offerId};
 const purchases=await Promise.all([0,1].map(()=>t.as(0,(tx,a,s)=>t.authority.purchaseFromEarningsFixture(tx,a,s,intent))));assert.deepEqual(purchases[0],purchases[1]);assert.equal(purchases[0]!.pointsAfter,0);assert.equal(purchases[0]!.recipients.length,1);
 assert.equal((await t.sql('SELECT count(*)::int n FROM g3_nemesis_debits')).rows[0].n,1);assert.equal((await t.sql('SELECT count(*)::int n FROM g3_nemesis_balances')).rows[0].n,0);assert.equal((await t.sql('SELECT count(*)::int n FROM g3_nemesis_earning_accounts')).rows[0].n,1);
 await t.earn(0,kill());assert.deepEqual(await t.as(0,(tx,a,s)=>t.authority.purchaseFromEarningsFixture(tx,a,s,intent)),purchases[0]);
 const g2=group();await t.as(0,(tx,a,s)=>t.authority.provisionOffersFixture(tx,a,s,g2));const other={operationId:randomUUID(),offerId:g2.offers[0]!.offerId};await t.as(0,(tx,a,s)=>t.authority.purchaseFromEarningsFixture(tx,a,s,other));
 assert.equal((await t.sql('SELECT sum(points)::int n FROM g3_nemesis_kills')).rows[0].n,70);assert.equal((await t.sql('SELECT sum(cost)::int n FROM g3_nemesis_debits')).rows[0].n,70);
 await assert.rejects(t.earn(0,{...e,killId:randomUUID()}));await assert.rejects(t.earn(1,e));await assert.rejects(t.earn(0,{...e,maxHpBits:nemesisF64Bits(4)}));await assert.rejects(t.as(0,(tx,a,s)=>t.authority.recordKillFixture(tx,a,{...s,worldToken:'changed'},kill())));
 await assert.rejects(t.as(1,(tx,a,s)=>t.authority.killStatusFixture(tx,a,s,e.killId),true));await assert.rejects(t.as(0,(tx,a,s)=>t.authority.killStatusFixture(tx,a,{...s,worldToken:'changed'},e.killId),true));
 const old=(await t.sql('SELECT record FROM g3_nemesis_kills ORDER BY sequence')).rows;await t.state.rpc(t.state.host,{endRun:{}});assert.deepEqual(await t.as(0,(tx,a,s)=>t.authority.killStatusFixture(tx,a,s,e.killId),true),a);await assert.rejects(t.earn(0,kill()));await t.f.restart();assert.deepEqual((await t.sql('SELECT record FROM g3_nemesis_kills ORDER BY sequence')).rows,old);assert.deepEqual(await t.as(0,(tx,a,s)=>t.authority.killStatusFixture(tx,a,s,e.killId),true),a);
 assert.equal((await t.sql('SELECT count(*)::int n FROM g3_nemesis_debits')).rows[0].n,2);
 }finally{await t.f.close();}
});
test('earning fixtures cannot mix arbitrary108 funding, weaken immutable guards, spend insufficient NP or enter a nonfixture schema',async()=>{
 const t=await fixture();try{await t.state.start();await t.state.playing();await t.earn(0,kill(3.4));const g=group();await t.as(0,(tx,a,s)=>t.authority.provisionOffersFixture(tx,a,s,g));await assert.rejects(t.as(0,(tx,a,s)=>t.authority.purchaseFromEarningsFixture(tx,a,s,{operationId:randomUUID(),offerId:g.offers[0]!.offerId})));assert.equal((await t.sql('SELECT count(*)::int n FROM g3_nemesis_debits')).rows[0].n,0);assert.equal((await t.sql('SELECT record FROM g3_nemesis_groups')).rows[0].record.consumedBy,undefined);
 await assert.rejects(t.as(0,(tx,a,s)=>t.authority.provisionFixture(tx,a,s,999,group())));await t.as(1,(tx,a,s)=>t.authority.provisionFixture(tx,a,s,999,group()));await assert.rejects(t.earn(1,kill()));
 for(const q of ['UPDATE g3_nemesis_balances SET points=points+1','DELETE FROM g3_nemesis_kills','UPDATE g3_nemesis_kills SET points=points+1','DELETE FROM g3_nemesis_earning_accounts',"UPDATE g3_nemesis_earning_accounts SET provenance='anything'"])await assert.rejects(t.sql(q));
 await assert.rejects(t.as(0,async(tx,a,s)=>{await tx.query('SET LOCAL search_path=pg_catalog');return t.authority.recordKillFixture(tx,a,s,kill());}));
 // The unchanged public contract has no earning/shop/funding fields.
 const body={major:1,revision:'g3-nemesis-draft-1',serviceId:t.f.service.db.serviceId,serviceEpoch:t.f.service.db.epoch,sessionEpoch:t.state.games[0]!.sessionEpoch,requestId:randomUUID(),scope:t.state.scopes[0],catalogueRevision:t.f.profile.catalogueRevision,earning:kill()};assert.ok((await t.f.call('/v3/nemesis/rpc',body,t.state.games[0]!.accessToken)).status>=400);
 const info=JSON.parse((await t.f.call('/v3/nemesis/info')).body);assert.equal(info.earningEnabled,false);assert.equal(info.offeringEnabled,false);assert.equal(info.fixtureFundingOnly,true);
 }finally{await t.f.close();}
});
test('durable shop audit preserves shifted source tiers, immutable pool/group/offer IDs and does not mint purchase offers',async()=>{
 const t=await fixture();try{await t.state.start();await t.state.playing();const evidence={groupId:randomUUID(),spawnKind:'spitter' as const,xBits:nemesisF64Bits(3.5),yBits:nemesisF64Bits(1100),choices:['grounded','shrooms','confusion'].map(abilityId=>({offerId:randomUUID(),abilityId}))};
 const call=()=>t.as(0,(tx,a,s)=>t.authority.recordShopFixture(tx,a,s,evidence)),[a,b]=await Promise.all([call(),call()]);assert.deepEqual(a,b);assert.equal(a.rngVerified,false);assert.equal(a.offers[0]!.x,1.5);assert.equal(a.offers[0]!.cost,35);
 assert.equal((await t.sql('SELECT count(*)::int n FROM g3_nemesis_groups')).rows[0].n,0);assert.equal((await t.sql('SELECT count(*)::int n FROM g3_nemesis_offers')).rows[0].n,0);await assert.rejects(t.as(0,(tx,a,s)=>t.authority.recordShopFixture(tx,a,s,{...evidence,groupId:randomUUID()})));await assert.rejects(t.as(1,(tx,a,s)=>t.authority.recordShopFixture(tx,a,s,evidence)));
 for(const q of ['DELETE FROM g3_nemesis_shop_provenance',"UPDATE g3_nemesis_shop_provenance SET record=jsonb_set(record,'{rngVerified}','true')"])await assert.rejects(t.sql(q));
 const saved=(await t.sql('SELECT record FROM g3_nemesis_shop_provenance')).rows;await t.state.rpc(t.state.host,{endRun:{}});await assert.rejects(call());await t.f.restart();assert.deepEqual((await t.sql('SELECT record FROM g3_nemesis_shop_provenance')).rows,saved);
 }finally{await t.f.close();}
});
