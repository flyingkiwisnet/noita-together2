import test from 'node:test';import assert from 'node:assert/strict';
import {matchesGoldControl} from '../scripts/lc1-gold-controls.js';
test('private lost-reply plan matches exact original kind, batch index, operation, owner and successful result',()=>{
 const plan={kind:'itemDeposit' as const,operationId:'original'},owner={roomId:'room',principalId:'owner'},operation={operationId:'original',principalId:'owner'};
 const single={scope:{roomId:'room'},itemDeposit:{operationId:'original'}};assert.equal(matchesGoldControl(plan,single,{itemOperation:operation},owner),true);
 const input={scope:{roomId:'room'},bankBatch:{entries:[{scope:{roomId:'room'},itemStatus:{operationId:'original'}},{scope:{roomId:'room'},itemDeposit:{operationId:'original'}}]}};
 const output={bankBatch:{entries:[{index:0,operationId:'original',itemOperation:operation},{index:1,operationId:'original',itemOperation:operation}]}};assert.equal(matchesGoldControl(plan,input,output,owner),true);
 const check=(modify:(v:any)=>void)=>{const value=structuredClone(output);modify(value);assert.equal(matchesGoldControl(plan,input,value,owner),false);};
 check(v=>v.bankBatch.entries[1].error={code:'CAPACITY'});check(v=>v.bankBatch.entries[1].index=0);check(v=>v.bankBatch.entries[1].operationId='other');check(v=>v.bankBatch.entries[1].itemOperation.operationId='other');check(v=>v.bankBatch.entries[1].itemOperation.principalId='other');check(v=>v.bankBatch.entries.pop());check(v=>v.error={code:'CAPACITY'});
 const wrongKind=structuredClone(input);wrongKind.bankBatch.entries.pop();assert.equal(matchesGoldControl(plan,wrongKind,output,owner),false);
 assert.equal(matchesGoldControl(plan,{...input,scope:{roomId:'other'}},output,owner),false);
 assert.equal(matchesGoldControl(plan,single,{error:{code:'CAPACITY'}},owner),false);
});
