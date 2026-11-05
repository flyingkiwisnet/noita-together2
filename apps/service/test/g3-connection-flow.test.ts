import test from 'node:test';import assert from 'node:assert/strict';
import {waitingReadiness,type WaitingFacts} from '../../../packages/domain/src/connection-flow.js';
import {decode,encode,validate} from '../../../packages/contracts/src/g2-next.js';
const valid:WaitingFacts={consented:true,desktopLive:true,reviewLive:true,attachment:true,gameLive:true,scopeMatches:true,reportPresent:true,loaded:true,protected:true};
test('U1 readiness projection requires every current game fact, not consent/presence alone',()=>{
 assert.equal(waitingReadiness(valid).ready,true);
 for(const key of ['consented','desktopLive','reviewLive','attachment','gameLive','scopeMatches','reportPresent','loaded','protected'] as const)assert.equal(waitingReadiness({...valid,[key]:false}).ready,false,key);
 for(const terminal of ['expired','cancelled'])assert.equal(waitingReadiness({...valid,terminal}).ready,false);
 for(const reportFailure of ['seed_unknown','seed_mismatch','inventory_incomplete','collector_failed','observed_version_mismatch'])assert.equal(waitingReadiness({...valid,reportFailure}).ready,false);
 assert.equal(waitingReadiness({...valid,excluded:true}).readiness,'excluded');assert.equal(waitingReadiness({...valid,running:true,protected:false}).readiness,'playing');
});
test('U1 generated canonical renewal and readiness codecs retain strict unknown/conflicting field fences',()=>{
 const req={major:1,revision:'g2-draft-5',serviceId:'service',requestId:'original',roomId:'room',runId:'run',runEpoch:'epoch',configRevision:1,preparationRenew:{preparationId:'same-id'}};
 assert.deepEqual(decode('Request',encode('Request',req)),req);
 assert.throws(()=>validate('Request',{...req,ready:{ready:true}}));assert.throws(()=>validate('PreparationRenew',{preparationId:'same-id',replace:true}));
 const member={desktopId:'desktop',ready:false,included:true,consentedConfigRevision:1,readiness:'waiting_noita',readinessReason:'actual_game_required'},decoded=decode('Member',encode('Member',member));assert.equal(decoded.ready??false,false);const {ready,...presentFields}=member;assert.deepEqual(decoded,presentFields);
});
