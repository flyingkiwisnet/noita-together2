import test from 'node:test';
import assert from 'node:assert/strict';
import {createG3CandidateApp} from '../src/g3/candidate-loot01.js';
test('normal metadata regression seam refuses fixed LAN, wrong database and non-package scope before calling verifier',async()=>{
 for(const changed of [
  {namespace:'g3_rejoin_lan_192_168_0_104_4358'},
  {databaseUrl:'postgres://nt2_g1:unused@127.0.0.1:5432/nt2_g1'},
  {databaseUrl:'postgres://nt2_g1:unused@127.0.0.1:54328/other'},
  {rejoinPackage:false},
  {rejoinInput:{path:'arbitrary',sha256:'0'.repeat(64)}},
 ]){
  let called=false;
  await assert.rejects(createG3CandidateApp({databaseUrl:'postgres://nt2_g1:unused@127.0.0.1:54328/nt2_g1',namespace:'g3_rejoin_0123456789abcdef0123456789abcdef',rejoin:true,rejoinPackage:true,rejoinPackageVerifier:async()=>{called=true;throw Error('Verifier should not run');},...changed} as any),/FORBIDDEN/);
  assert.equal(called,false);
 }
});
