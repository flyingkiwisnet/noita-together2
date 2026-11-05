// Server-owned bounded source-profile review; never installs or starts a service.
import {readFile,writeFile} from 'node:fs/promises';
import {createHash} from 'node:crypto';
const root=new URL('../../../',import.meta.url),sha=b=>createHash('sha256').update(b).digest('hex');
const path='apps/desktop/g3-native-review-request.record2.json',bytes=await readFile(new URL(path,root)),m=JSON.parse(bytes);
if(sha(bytes)!=='ccfc578524292620af02b5377751304bbd95873edbbc6fa66dbdfbef8894b1b9')throw Error('Review request changed');
const frozen='spikes/noita-entry/.build/g3-candidates/nt2-g3-cp6-20261006-121758-b39929bb';
if(!m.frozen||m.frozenRoot!==frozen||m.pins.length!==61||m.pins.some(p=>!p.path.startsWith(frozen+'/')||p.path.includes('/../')))throw Error('Unreviewed frozen snapshot');
const snapshotRequest=frozen+'/review-request.json';
if(sha(await readFile(new URL(snapshotRequest,root)))!==sha(bytes))throw Error('Frozen request differs');
for(const entry of m.sourceProvenance)if(!m.pins.some(p=>p.path===entry.frozenPath&&p.sha256===entry.sha256))throw Error('Provenance differs');
const old=JSON.parse(await readFile(new URL('deploy/profiles/g2-inspect-multibuild-candidate.json',root)));
if(JSON.stringify(m.requestedCapabilities)!==JSON.stringify(['core.seed','core.ghost'])||m.actualNoitaEvidence||m.installApproved||m.packaged)throw Error('Review scope differs');
for(const p of m.pins)if(sha(await readFile(new URL(p.path,root)))!==p.sha256)throw Error('Pin changed: '+p.path);
for(const pair of m.requestedBuildPairs)if(!old.profile.supportedBuilds.some(v=>v.id===pair.id&&v.store===pair.store&&v.gameExeSha256===pair.gameExeSha256&&v.originalLuaSha256===pair.originalLuaSha256))throw Error('Unreviewed exact game/Lua pair');
const pin=path=>m.pins.find(p=>p.path===path).sha256;
const profile={id:'nt2-g3-native-development-2',evidenceStatus:'candidate-engine-evidence-pending',gameExeSha256:m.requestedBuildPairs[0].gameExeSha256,originalLuaSha256:m.requestedBuildPairs[0].originalLuaSha256,bridgeSha256:pin(m.nativeArtifact),modSha256:pin(m.entrySource),gameFamilyId:'noita-jan2025-x86-luajit-core',supportedBuilds:m.requestedBuildPairs.map(v=>({id:v.id,store:v.store,branch:'unclassified',buildIdentity:'PE32 x86; Jan25-2025 reference pair',gameExeSha256:v.gameExeSha256,originalLuaSha256:v.originalLuaSha256,evidenceStatus:'candidate-engine-evidence-pending'}))};
const result={id:profile.id,status:'source-reviewed-development-only',serviceId:'nt2-g3-native-development',namespacePrefix:'g3_',lobbyRevision:'g2-draft-5',gameplayRevision:m.schemaRevision,schemaSha256:m.schemaSha256,compatibility:{gameVersion:m.gameVersion,modVersion:m.modVersion,bridgeVersion:m.bridgeVersion,capabilities:m.requestedCapabilities,modes:['core'],settingKeys:[]},profile,provenance:{manifest:{path,revision:m.revision,sha256:sha(bytes)},snapshotRequest:{path:snapshotRequest,sha256:sha(bytes)},frozenRoot:frozen,pins:m.pins,sourceProvenance:m.sourceProvenance,review:'docs/g3/coordinator-review-20261006-overnight5.md',framing:m.framing,limits:'Source-profile authorization only. No actual Noita compatibility, installation, packaging, production release, gold or social capability approval.',joinedEvidence:'Joined Core/proof-bound sign-in/selected Start/HTTPS/two OS pipes/nonempty ghosts source fixture reviewed; engine facts and second pipe namespace simulated. Actual Noita/two-PC/save proof pending.'},installApproved:false,actualNoitaEvidence:false};
await writeFile(new URL('deploy/profiles/g3-native-development-candidate.json',root),JSON.stringify(result,null,2)+'\n');
