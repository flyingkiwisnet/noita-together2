import test from 'node:test';
import assert from 'node:assert/strict';
import {databasePrerequisite,candidateDatabaseUrl} from '../src/g2-next/db-prerequisite.js';
test('host database prerequisite reuses healthy DB; only absent default permits explicit Compose startup',async()=>{
 assert.deepEqual(await databasePrerequisite(candidateDatabaseUrl,async()=>{}),{status:'healthy',canStartExistingCompose:false});
 const failure=(code:string)=>async()=>{throw Object.assign(new Error('Do not expose connection details'),{code});};
 assert.deepEqual(await databasePrerequisite(candidateDatabaseUrl,failure('ECONNREFUSED')),{status:'absent',canStartExistingCompose:true});
 assert.deepEqual(await databasePrerequisite('postgres://custom/db',failure('ECONNREFUSED')),{status:'absent',canStartExistingCompose:false});
 for(const code of ['28P01','3D000','ECONNRESET','ETIMEDOUT'])assert.deepEqual(await databasePrerequisite(candidateDatabaseUrl,failure(code)),{status:'unhealthy',canStartExistingCompose:false});
});
