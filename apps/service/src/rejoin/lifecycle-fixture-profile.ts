// Explicit uninstalled metadata input for disposable LC1 composition only.
// Never selected by the installed LAN config/profile or public RPC input.
import {readFile,realpath} from 'node:fs/promises';import {createHash} from 'node:crypto';import {fileURLToPath} from 'node:url';import {resolve,dirname,sep} from 'node:path';
import {safeArtifactPath,inspectNativeExports,type ArtifactPin} from './composition-profile.js';
import {validateBuildRegistry} from '../g2-next/build-registry.js';import {fingerprint} from '../g1/security.js';
import {SOCIAL_CONTROL_CAPABILITIES} from '../../../../packages/domain/src/g3-social.js';
const root=new URL('../../../../',import.meta.url),sha=(b:Buffer)=>createHash('sha256').update(b).digest('hex');
const a2Files=['a2_text.lua','a2_capture.lua','a2_ui_model.lua','a2_biomes.lua','a2_input_lock.lua','a2_renderer.lua','a2_native_input.lua','poi_proximity.lua','ui/keyboard.png','ui/player_ghosts.png','ui/hide_player_ghosts.png','ui/chat.png','ui/hide_chat.png','ui/player_list.png','ui/radar.png','ui/poi_emitter.png'] as const;
const demand=(v:unknown,m:string)=>{if(!v)throw Error('LC1 fixture runtime: '+m);};
export async function verifyLifecycleFixtureRuntime(pin:ArtifactPin){
 demand(process.env.NODE_ENV!=='production'&&pin.path.startsWith('spikes/noita-entry/.build/')&&pin.path.endsWith('.json'),'private uninstalled build input required');
 const checked=async(p:ArtifactPin)=>{safeArtifactPath(p.path);demand(/^[a-f0-9]{64}$/.test(p.sha256),'hash');const url=new URL(p.path,root);demand(resolve(await realpath(url))===resolve(fileURLToPath(url)),'physical current input required');const b=await readFile(url);demand(sha(b)===p.sha256,'changed '+p.path);return b;};
 const bytes=await checked(pin);demand(bytes.length<=1024*1024,'metadata bound');const r=JSON.parse(bytes.toString());
 for(const key of ['fixtureOnly','sourceApproved','developmentOnly','testBuild'])demand(r[key]===true,'fixture flags');
 for(const key of ['installApproved','productionApproved','packaged','actualNoitaEvidence'])demand(r[key]===false,'uninstalled candidate only');
 const baseline=JSON.parse((await checked({path:'deploy/profiles/g3-se1-runtime.20261018.json',sha256:'28e855ead443bbfcf98b474f0db8ffe61bb11268c92f17307ece3bcf22899409'})).toString());
 const b1=r.b1Fixture;if(b1!==undefined)demand(b1?.revision==='b1-gold-private-1'&&b1.installed===false&&r.a3Fixture!==undefined&&r.fixtureNative?.private===true,'explicit private B1 gold selector');
 const a3=r.a3Fixture,c1=r.c1Fixture,c2=r.c2Fixture,race=r.raceFixture;
 const raceFiles=['catalogue.lua','hook_api.lua','local.lua','panel.lua','player_death.lua','register.lua','sample.lua','sha256.lua','storage.lua','append/boss.lua','append/orb.lua'];
 if(race!==undefined)demand(race&&Object.keys(race).sort().join(',')==='injectedEngine,installed,revision'&&race.revision==='race-first-release-private-1'&&race.installed===false&&race.injectedEngine===true&&c2!==undefined&&r.fixtureNative?.private===true&&a3?.runtimeFiles===324,'explicit private Race selector/324-file closure');
 if(c1!==undefined){demand(c1&&c1.revision==='c1-player-private-1'&&c1.installed===false&&c1.injectedEngine===true&&c1.serviceMocked===false&&c1.normalProfilesChanged===false&&a3!==undefined&&r.fixtureNative?.private===true,'explicit private C1 selector');const path='packages/contracts/g3-c1/catalogue.json',current=await readFile(new URL(path,root));demand(r.c1Catalogue?.revision===JSON.parse(current.toString()).revision&&r.c1Catalogue?.sha256===sha(current),'canonical C1 catalogue');}
 else demand(r.c1Catalogue===undefined,'C1 catalogue requires explicit selector');
 if(a3!==undefined)demand(a3&&a3.revision==='a3-private-1'&&a3.baseRuntimeFiles===66&&Number.isInteger(a3.runtimeFiles)&&a3.runtimeFiles>66&&a3.runtimeFiles<=512&&a3.injectedEngine===true&&a3.serviceMocked===false&&a3.normalProfilesChanged===false&&a3.installed===false&&r.fixtureNative?.private===true&&r.a2Fixture!==undefined,'private A3 fixture selector');
 if(c2!==undefined){demand(c1!==undefined&&c2.revision==='c2-world-private-1'&&c2.installed===false&&c2.injectedEngine===true&&c2.serviceMocked===false&&c2.normalProfilesChanged===false&&r.fixtureNative?.private===true,'explicit private C2 selector');const b=await readFile(new URL('packages/contracts/g3-hourglass/catalogue.json',root));demand(r.hourglassCatalogue?.revision===JSON.parse(b.toString()).revision&&r.hourglassCatalogue?.sha256===sha(b),'canonical hourglass catalogue');}
 const compatibility=a3===undefined?baseline.compatibility:{...baseline.compatibility,capabilities:[...baseline.compatibility.capabilities,...SOCIAL_CONTROL_CAPABILITIES]};
 if(b1!==undefined){compatibility.capabilities=[...compatibility.capabilities,'core.gold'];compatibility.settingKeys=[...new Set([...compatibility.settingKeys,'NT_send_gold'])];const gold=await readFile(new URL('packages/contracts/g3/schema.json',root));demand(r.gameplayRevision===JSON.parse(gold.toString()).revision&&r.gameplayRevision==='g3-draft-4'&&r.schemaSha256===sha(gold),'current B1 canonical contract');}
 if(r.itemFixture!==undefined){demand(b1!==undefined&&r.itemFixture.revision==='items-private-1'&&r.itemFixture.installed===false,'private items selector');compatibility.capabilities=[...compatibility.capabilities,'core.items'];compatibility.settingKeys=[...compatibility.settingKeys,'NT_send_wands','NT_send_flasks','NT_send_items'];const c=await readFile(new URL('packages/contracts/g3-items/catalogue.json',root));demand(r.itemCatalogue?.revision===JSON.parse(c.toString()).revision&&r.itemCatalogue?.sha256===sha(c),'canonical item catalogue');}
 if(c2!==undefined)compatibility.capabilities=[...compatibility.capabilities,'core.world-rules','core.hourglass'];
 if(race!==undefined)compatibility.capabilities=[...compatibility.capabilities,'race.goals.v1'];
 demand(typeof r.id==='string'&&/^[a-z0-9][a-z0-9._-]{0,127}$/.test(r.id)&&r.serviceId===baseline.serviceId&&fingerprint(r.compatibility)===fingerprint(compatibility),'service/compatibility');
 if(a3!==undefined){const social=await readFile(new URL('packages/contracts/g3-social/schema.json',root));demand(r.socialRevision===JSON.parse(social.toString()).revision&&r.socialSchemaSha256===sha(social),'current A3 canonical contract');}
 validateBuildRegistry(r.buildRegistry);
 demand(fingerprint(Object.fromEntries(Object.keys(r.buildRegistry).map(k=>[k,r.profile?.[k]])))===fingerprint(r.buildRegistry),'profile registry');
 for(const k of ['gameVersion','modVersion','bridgeVersion','capabilities'])demand(fingerprint(r.profile[k])===fingerprint(r.compatibility[k]),'profile compatibility');
 for(const k of ['supportedBuilds','gameExeSha256','originalLuaSha256','gameFamilyId'])for(const registry of [r.buildRegistry,r.profile])demand(fingerprint(registry[k])===fingerprint(baseline.buildRegistry[k]),'original game pairs');
 demand(r.buildRegistry.id===r.id&&r.profile.id===r.id,'profile identity');
 const a2=r.a2Fixture;
 if(a2!==undefined){
  demand(a2&&typeof a2==='object'&&!Array.isArray(a2)&&a2.revision==='a2-private-1'&&a2.baseRuntimeFiles===50&&a2.runtimeFiles===66&&a2.injectedEngine===true&&a2.serviceMocked===false&&a2.normalProfilesChanged===false&&a2.installed===false&&r.fixtureNative?.private===true,'private A2 fixture selector');
  demand(Array.isArray(a2.addedFiles)&&a2.addedFiles.length===16&&fingerprint([...a2.addedFiles].sort())===fingerprint([...a2Files].sort()),'exact A2 additions');
 }
 const count=a3?.runtimeFiles??(a2===undefined?50:66);
 demand(Array.isArray(r.runtimeMappings)&&r.runtimeMappings.length===count&&Array.isArray(r.requiredLocalFiles)&&r.requiredLocalFiles.length===count,a2===undefined?'current50-file closure':'current66-file A2 closure');
 const itemCatalogueSource='packages/contracts/g3-items/catalogue.lua',itemCatalogueDestination='mods/noita_together2/files/g3/item_catalogue_data.lua';
 const canonicalItemMapping=(m:any)=>r.itemFixture?.revision==='items-private-1'&&r.itemFixture.installed===false&&m.sourcePath===itemCatalogueSource&&m.path===itemCatalogueDestination;
 const c1CatalogueSource='packages/contracts/g3-c1/catalogue.lua',c1CatalogueDestination='mods/noita_together2/files/g3/c1_catalogue_data.lua';
 const canonicalC1Mapping=(m:any)=>c1!==undefined&&m.sourcePath===c1CatalogueSource&&m.path===c1CatalogueDestination;
 const ownedC2Mapping=(m:any)=>c2!==undefined&&m.sourcePath===m.path.replace(/^mods\//,'mod/')&&/^mods\/noita_together2\/files\/c2\/[A-Za-z0-9_./-]+$/.test(m.path);
 const ownedC1Mapping=(m:any)=>c1!==undefined&&m.sourcePath===m.path.replace(/^mods\//,'mod/')&&/^mods\/noita_together2\/files\/c1\/[A-Za-z0-9_./-]+$/.test(m.path);
 // D1 reuses only C2's saved progression helper in a private C1 composition;
 // this does not select C2 gameplay or admit its remaining subtree.
 const ownedD1Support=(m:any)=>c1!==undefined&&m.sourcePath===m.path.replace(/^mods\//,'mod/')&&['mods/noita_together2/files/g3/d1_hook_api.lua','mods/noita_together2/files/g3/d1_local.lua','mods/noita_together2/files/c1/append/steve_death.lua','mods/noita_together2/files/c2/storage.lua'].includes(m.path);
 const testingCatalogueSource='packages/contracts/g3/testing-starting-loadout.json',testingCatalogueDestination='mods/noita_together2/files/startup/testing-starting-loadout.json';
 const canonicalTestingMapping=(m:any)=>c1!==undefined&&m.sourcePath===testingCatalogueSource&&m.path===testingCatalogueDestination;
 const ownedLifecycleAddition=(m:any)=>c1!==undefined&&m.sourcePath===m.path.replace(/^mods\//,'mod/')&&['mods/noita_together2/files/rejoin/lobby_status.lua','mods/noita_together2/files/startup/testing_loadout.lua'].includes(m.path);
 const ownedRaceMapping=(m:any)=>race!==undefined&&m.sourcePath===m.path.replace(/^mods\//,'mod/')&&raceFiles.some(p=>m.path==='mods/noita_together2/files/race/'+p);
 const mapped=new Map<string,any>();for(const m of r.runtimeMappings){safeArtifactPath(m.path);safeArtifactPath(m.sourcePath);demand(m.path.startsWith('mods/noita_together2/')&&!mapped.has(m.path)&&(m.sourcePath.startsWith('mod/noita_together2/')||m.sourcePath.startsWith('spikes/noita-entry/.build/')||canonicalItemMapping(m)||canonicalC1Mapping(m)||canonicalTestingMapping(m)),'owned unique mapping');mapped.set(m.path,m);await checked({path:m.sourcePath,sha256:m.sha256});}
 if(r.itemFixture!==undefined){const m=mapped.get(itemCatalogueDestination);demand(m&&canonicalItemMapping(m),'fixed canonical item catalogue mapping');const current=await checked({path:itemCatalogueSource,sha256:m.sha256});demand(current.toString().includes('["sha256"]="'+r.itemCatalogue.sha256+'"'),'item Lua/JSON catalogue agreement');}
 if(c1!==undefined){const m=mapped.get(c1CatalogueDestination);demand(m&&canonicalC1Mapping(m),'fixed canonical C1 catalogue mapping');const current=await checked({path:c1CatalogueSource,sha256:m.sha256});demand(current.toString().includes('["sha256"]="'+r.c1Catalogue.sha256+'"')&&current.toString().includes('["revision"]="'+r.c1Catalogue.revision+'"'),'C1 Lua/JSON catalogue agreement');}
 const required=new Map<string,string>();for(const p of r.requiredLocalFiles){demand(!required.has(p.path)&&mapped.get(p.path)?.sha256===p.sha256,'required file mapping');required.set(p.path,p.sha256);}
 if(race!==undefined)for(const file of raceFiles){const m=mapped.get('mods/noita_together2/files/race/'+file);demand(m&&ownedRaceMapping(m),'fixed Race mapping '+file);}
 if(a2!==undefined){
  demand(baseline.runtimeMappings.length===50,'A2 base closure');
  const allowed=new Set<string>(baseline.runtimeMappings.map((m:any)=>m.path));
  for(const p of allowed)demand(mapped.has(p),'retained A2 base file');
  for(const file of a2Files){const destination='mods/noita_together2/files/g3/'+file,source='mod/noita_together2/files/g3/'+file;demand(mapped.get(destination)?.sourcePath===source,'fixed A2 mapping '+file);allowed.add(destination);}
  for(const [destination,m]of mapped)if(!allowed.has(destination))demand(a3!==undefined&&(canonicalItemMapping(m)||canonicalC1Mapping(m)||canonicalTestingMapping(m)||ownedLifecycleAddition(m)||ownedD1Support(m)||ownedC1Mapping(m)||ownedC2Mapping(m)||ownedRaceMapping(m)||m.sourcePath===destination.replace(/^mods\//,'mod/')&&/^mods\/noita_together2\/files\/(emotes|g3)\/[A-Za-z0-9_./-]+$/.test(destination)),'unknown A2/A3 file');
 }
 for(const p of ['settings.lua','mod.xml','files/startup/bundle.lua','files/startup/settings_scope.lua','files/startup/scope_registry.lua'])demand(mapped.has('mods/noita_together2/'+p),'startup closure');
 const entry=mapped.get('mods/noita_together2/init.lua');demand(entry?.sourcePath==='mod/noita_together2/init-g3-seed-loot.lua'&&r.buildRegistry.modSha256===entry.sha256&&r.profile.modSha256===entry.sha256,'current entry registry');
 demand(Array.isArray(r.nativeArtifacts)&&r.nativeArtifacts.length===3&&new Set(r.nativeArtifacts.map((a:any)=>a.mode)).size===3,'three native images');
 const privateNative=r.fixtureNative?.private===true;
 if(privateNative){
  const prefix='\\\\.\\pipe\\nt2-lc1-',selector=r.fixtureNative;
  demand(typeof selector.pipePrefix==='string'&&(a2===undefined?selector.pipePrefix.startsWith(prefix)&&/^[a-f0-9]{32}-$/.test(selector.pipePrefix.slice(prefix.length)):['\\\\.\\pipe\\nt2-a2-client-20261020-01-','\\\\.\\pipe\\nt2-a2-retained-20261022-01-'].includes(selector.pipePrefix)),'private LC1/A2 pipe selector');
  const buildRoot=resolve(fileURLToPath(new URL('spikes/noita-entry/.build/',root)))+sep;
  demand(typeof selector.testAppPath==='string'&&resolve(selector.testAppPath).toLowerCase().startsWith(buildRoot.toLowerCase())&&selector.testAppPath.endsWith('.exe'),'private test app selector');
  if(race!==undefined)demand(resolve(await realpath(selector.testAppPath))===resolve(selector.testAppPath),'physical private Race app');
  const pins=selector.sourcePins;demand(Array.isArray(pins)&&pins.length===4,'private native source pins');
  for(const path of ['native/g3-ipc/broker.cpp','native/g3-ipc/wrapper.cpp','native/g3-ipc/abi.h','native/g3-ipc/build.ps1']){const p=pins.find((p:any)=>p.path===path);demand(p,'private native source');await checked(p);}
 }
 for(const a of r.nativeArtifacts){
  demand(['broker','primary','social'].includes(a.mode)&&a.path.startsWith('spikes/noita-entry/.build/')&&a.path.endsWith('.dll')&&a.testFlags===privateNative&&a.private===privateNative,'explicit native flags');
  const exports=inspectNativeExports(await checked(a));demand(fingerprint(exports)===fingerprint([...a.exports].sort())&&!exports.some(e=>/test/i.test(e)),'native exports');
  demand(exports.includes('nt2_g3_private_api')===(privateNative&&a.mode!=='broker')&&!exports.some(e=>e.includes('private')&&e!=='nt2_g3_private_api'),'private wrapper export');
  demand(a.buildCommand?.path?.startsWith('spikes/noita-entry/.build/')&&a.buildCommand.path.endsWith('/compile.cmd')&&dirname(a.buildCommand.path)===dirname(a.path),'current compile receipt');
  const command=(await checked(a.buildCommand)).toString();demand(command.includes('/Brepro')&&command.includes('-arch=x86'),'reproducible x86 receipt');
  if(privateNative){const s=r.fixtureNative;demand(command.includes('/DNT2_IPC_PRIVATE')&&command.includes('/DNT2_TEST_APP_PATH=L"""'+s.testAppPath.replaceAll('\\','\\\\')+'"""')&&command.includes('/DNT2_PIPE_PREFIX=L"""'+s.pipePrefix.replaceAll('\\','\\\\')+'"""'),'private compiled selectors');}
  else demand(!/\/DNT2_(IPC_PRIVATE|TEST_|PIPE_PREFIX)/.test(command),'normal compile flags');
  demand(r.runtimeMappings.some((m:any)=>m.sourcePath===a.path&&m.sha256===a.sha256),'image mapping');if(a.mode==='primary')demand(r.buildRegistry.bridgeSha256===a.sha256&&r.profile.bridgeSha256===a.sha256,'current primary registry');
 }
 for(const m of r.runtimeMappings)if(m.path.endsWith('.dll'))demand(r.nativeArtifacts.some((a:any)=>a.path===m.sourcePath&&a.sha256===m.sha256),'unverified DLL');
 return {...r,runtimeResourceJson:bytes.toString(),runtimeApproval:pin,desktopApproval:pin};
}
