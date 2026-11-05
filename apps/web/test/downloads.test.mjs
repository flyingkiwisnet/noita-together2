import {test} from 'node:test';
import assert from 'node:assert/strict';
import {readFile,mkdtemp,mkdir,writeFile,rm} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {resolve,join,posix,win32} from 'node:path';
import {fileURLToPath} from 'node:url';
import {validateReleaseMetadata,releaseLinks} from '../../../packages/contracts/src/release-metadata.js';
import {prepareBetaHosting,inside} from '../../service/scripts/prepare-beta-hosting.mjs';
import {controllerDom} from './dom-fixture.mjs';
const source=await readFile(new URL('../candidate/downloads.js',import.meta.url),'utf8');
const module=await import('data:text/javascript;base64,'+Buffer.from(source.replace("'/packages/contracts/src/release-metadata.js'",JSON.stringify(new URL('../../../packages/contracts/src/release-metadata.js',import.meta.url).href))).toString('base64'));
const pending={schemaVersion:1,status:'pending',version:'0.1.0-beta.1',platform:'windows-x64'};
const available=()=>({...pending,status:'available',builtAt:'2026-11-05T00:00:00.000Z',signed:false,artifact:{file:'NoitaTogether2-0.1.0-beta.1-windows-x64.zip',bytes:3,sha256:createHash('sha256').update('zip').digest('hex')},compatibility:{profileId:'beta',gameVersion:'1',modVersion:'1',bridgeVersion:'1',lobbyRevision:'1',gameplayRevision:'1'}});
test('staging containment supports POSIX and Windows children and rejects parent, sibling and drive escapes',()=>{
 for(const [paths,parent] of [[posix,'/scratch/build'],[win32,'C:\\scratch\\build']]){
  assert.equal(inside(parent,paths.join(parent,'release'),paths),true);
  assert.equal(inside(parent,paths.join(parent,'nested','release'),paths),true);
  assert.equal(inside(parent,parent,paths),false);
  assert.equal(inside(parent,paths.dirname(parent),paths),false);
  assert.equal(inside(parent,parent+'-sibling',paths),false);
 }
 assert.equal(inside('C:\\scratch\\build','D:\\scratch\\build\\release',win32),false);
 assert.equal(inside('C:\\scratch\\build','\\\\server\\share\\release',win32),false);
});
test('metadata rejects traversal, extra URLs, malformed timestamps and wrong artifact identities',()=>{
 assert.equal(releaseLinks(pending),null);assert.equal(validateReleaseMetadata(available()).signed,false);
 for(const change of [m=>m.version='../beta',m=>m.url='https://elsewhere.invalid',m=>m.artifact.file='../payload.zip',m=>m.artifact.bytes=0,m=>m.artifact.sha256='A'.repeat(64),m=>m.builtAt='2026-02-30T00:00:00.000Z',m=>m.compatibility.profileId='<script>',m=>m.builtAt='yesterday']){const m=available();change(m);assert.throws(()=>validateReleaseMetadata(m));}
 const m=available();m.builtAt='2026-11-05T00:00:00Z';assert.doesNotThrow(()=>validateReleaseMetadata(m));
});
test('anonymous renderer only creates matching links for available metadata and clears stale links on failures',async()=>{
 const dom=await controllerDom('../candidate/download.html');module.renderRelease(dom.document,pending);assert.equal(dom.get('release-links').children.length,0);
 module.renderRelease(dom.document,available());assert.deepEqual(dom.get('release-links').children.map(x=>x.href),Object.values(releaseLinks(available())));assert.match(dom.get('release-details').textContent,/Unsigned/);
 let options;await module.loadRelease(dom.document,async(path,opts)=>{assert.equal(path,'/releases/latest.json');options=opts;return {ok:true,json:async()=>pending};});assert.equal(options.credentials,'omit');assert.equal(dom.get('release-links').children.length,0);
 for(const fetcher of [async()=>({ok:false}),async()=>({ok:true,json:async()=>({...available(),url:'https://elsewhere.invalid'})}),async()=>{throw Error('offline');}]){module.renderRelease(dom.document,available());await module.loadRelease(dom.document,fetcher);assert.equal(dom.get('release-links').children.length,0);assert.match(dom.get('release-status').textContent,/temporarily unavailable/);}
});
test('hosting stages matching bytes and refuses tampering, overwrite and escaped destinations',async()=>{
 const base=fileURLToPath(new URL('../../../spikes/noita-entry/.build',import.meta.url));await mkdir(base,{recursive:true});const scratch=await mkdtemp(join(base,'beta-host-test-'));
 try{const m=available(),manifest=join(scratch,'manifest.json'),artifact=join(scratch,m.artifact.file);await writeFile(manifest,JSON.stringify(m));await writeFile(artifact,'zip');const out=join(scratch,'host');await prepareBetaHosting({manifest,artifact,out});assert.equal(await readFile(join(out,'downloads',m.version,m.artifact.file),'utf8'),'zip');assert.deepEqual(JSON.parse(await readFile(join(out,'releases/latest.json'),'utf8')),m);assert.match(await readFile(join(out,'releases',m.version,'SHA256SUMS.txt'),'utf8'),new RegExp(m.artifact.sha256));await assert.rejects(prepareBetaHosting({manifest,artifact,out}));await assert.rejects(prepareBetaHosting({manifest,artifact,out:resolve('dist')}));await writeFile(artifact,'bad');await assert.rejects(prepareBetaHosting({manifest,artifact,out:join(scratch,'tampered')}));}
 finally{await rm(scratch,{recursive:true,force:true});}
});
test('public pages and shipped route/build inputs retain anonymous download/help and omit removed login intro',async()=>{
 const html=await readFile(new URL('../candidate/index.html',import.meta.url),'utf8');for(const text of ['WELCOME TO THE EXPEDITION','Your world. Your party.','Share the adventure, keep your own world.'])assert.equal(html.includes(text),false);assert.match(html,/id="provider-choices"/);assert.match(html,/href="\/download"/);
 for(const page of ['download','help','credits']){const s=await readFile(new URL(`../candidate/${page}.html`,import.meta.url),'utf8');assert.match(s,/name="viewport"/);assert.doesNotMatch(s,/\son\w+=|<script(?![^>]*src=)/i);}
 const service=await readFile(new URL('../../service/src/g2-next/app.ts',import.meta.url),'utf8');for(const route of ['/download','/help','/credits','/releases/latest.json'])assert.ok(service.includes("'"+route+"'"));
});
