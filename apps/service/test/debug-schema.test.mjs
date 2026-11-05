import test from 'node:test';import assert from 'node:assert/strict';
import {mkdtemp,readFile,rm,mkdir,writeFile} from 'node:fs/promises';import {join} from 'node:path';
import Ajv2020 from 'ajv/dist/2020.js';
import {ServerDebugDiagnostics,serverDebugSchema,diagnosticServiceTag,diagnosticTag} from '../src/g3/debug-diagnostics.ts';
test('actual persisted files match strict generated export schema; arbitrary fields and pose objects are refused',async()=>{
 const root=await mkdtemp(join(process.cwd(),'spikes/noita-entry/.build/nt2-debug-schema-'));
 const sink=new ServerDebugDiagnostics('nt2-debug-safe-fixture',true,()=>1000,{root,serviceTag:diagnosticServiceTag('fixture-service','fixture-epoch')});
 try{
  const c={};sink.record('pose_accepted',{poseSequence:21,sessionTag:diagnosticTag('fixture-game')},false);sink.snapshot(c,['fixture-game'],['fixture-game'],{});sink.record('sample_removed',{reason:'pose_ttl',sampleAgeMs:2001,sampleGeneration:1,memberGeneration:1,lastAcceptedPoseSequence:21,eligible:true});sink.snapshot(c,[],['fixture-game'],{});sink.snapshot(c,['fixture-game'],['fixture-game'],{});await sink.close();
  const schema=serverDebugSchema(),validate=new Ajv2020({strict:false,allErrors:true}).compile(schema);
  assert.deepEqual(JSON.parse(await readFile('docs/g3/server-debug.schema.json','utf8')),schema);
  const manifest=JSON.parse(await readFile(join(sink.folder,'manifest.json'),'utf8')),firstLoss=JSON.parse(await readFile(join(sink.folder,'first-loss.json'),'utf8')),events=(await readFile(join(sink.folder,'events.jsonl'),'utf8')).trim().split('\n').map(s=>JSON.parse(s));
  for(const value of [manifest,firstLoss,...events])assert.equal(validate(value),true,JSON.stringify(validate.errors));
  for(const extra of [{token:'private'},{poses:[{xMilli:'7'}]},{code:'ARBITRARY_RAW_MESSAGE'}])assert.equal(validate({...events[0],...extra}),false);
  assert.equal(validate({...firstLoss,before:[{raw:'no'}]}),false);
  await mkdir('apps/service/test/fixtures',{recursive:true});await writeFile('apps/service/test/fixtures/server-debug-safe-v1.json',JSON.stringify({manifest,events,firstLoss},null,2)+'\n');
 }finally{await sink.close();await rm(root,{recursive:true,force:true});}
});
