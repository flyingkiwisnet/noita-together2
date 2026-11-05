import test from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import type {GameObservation} from '../../../packages/contracts/g2-next/g2-next.js';
import {canonicalContent,contentDigest,contentFailure,validateObservation} from '../src/g2-next/observation.js';
const base:GameObservation={attachmentId:'attachment',gameInstanceId:'game',worldToken:'world',observationRevision:1,actualSeed:'0',seedStatus:'known',activeMods:[],beta:'unknown',inventoryComplete:true,collectorFailure:'',protected:true,loaded:true,modVersion:'mod-v3',bridgeVersion:'bridge-v3',compatibilityProvenance:'game-thread-client-observation'};
const golden=createRequire(import.meta.url)('../../../packages/contracts/g2-next/content-digest-golden.json') as {fixtures:{tuple:[string|null,string[],boolean|null,string,string];canonical:string;utf8Hex:string;sha256:string}[]};
test('content digest goldens preserve decimal precision, Unicode and ordered duplicates independently of phase',()=>{
 for(const f of golden.fixtures){const [seed,mods,beta,modVersion,bridgeVersion]=f.tuple;const report:GameObservation={...base,actualSeed:seed??'0',seedStatus:seed===null?'unknown':'known',activeMods:mods,beta:beta===null?'unknown':String(beta),modVersion,bridgeVersion,inventoryComplete:seed!==null,collectorFailure:seed===null?'seed_unavailable':''};if(seed===null)delete report.actualSeed;validateObservation(report);assert.equal(canonicalContent(report),f.canonical);assert.equal(Buffer.from(canonicalContent(report)).toString('hex'),f.utf8Hex);assert.equal(contentDigest(report),f.sha256);assert.equal(contentDigest({...report,protected:false,loaded:false,observationRevision:2}),f.sha256);}
});
test('observations preserve unknown/failure facts but never make them eligible; canonical seeds and text bounds reject hostile facts',()=>{
 const unknown:GameObservation={...base,seedStatus:'unknown',inventoryComplete:false,collectorFailure:'seed_unavailable'};delete unknown.actualSeed;validateObservation(unknown);assert.equal(contentFailure(unknown,0,{modVersion:'mod-v3',bridgeVersion:'bridge-v3'}),'seed_unknown');
 for(const actualSeed of ['00','-1','1.0','18446744073709551616'])assert.throws(()=>validateObservation({...base,actualSeed}));
 assert.throws(()=>validateObservation({...unknown,actualSeed:'0'}));assert.throws(()=>validateObservation({...unknown,inventoryComplete:true}));
 assert.throws(()=>validateObservation({...base,activeMods:['bad\u0080']}));assert.throws(()=>validateObservation({...base,activeMods:['\ud800']}));
 const bounded={...base,activeMods:Array(240).fill('x'.repeat(128))};validateObservation(bounded);assert.ok(Buffer.byteLength(JSON.stringify(bounded))<=32768);
 assert.throws(()=>validateObservation({...bounded,activeMods:Array(256).fill('x'.repeat(128))}));
 assert.notEqual(contentDigest({...base,activeMods:['a','a']}),contentDigest({...base,activeMods:['a']}));
});
