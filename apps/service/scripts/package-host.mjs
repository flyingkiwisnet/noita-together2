// Reusable build-time packager. Produces fresh candidates only; never publishes.
import fs from 'node:fs';import path from 'node:path';import {createHash} from 'node:crypto';import {createRequire} from 'node:module';import ts from 'typescript';
const args=process.argv.slice(2),options={};for(let i=0;i<args.length;i+=2){if(!['--out','--launcher','--config','--runtime','--source'].includes(args[i])||!args[i+1]||options[args[i]])throw Error('Usage: package-host.mjs --out fresh/host --launcher fresh/support/Start-Host.ps1 --config config.json --runtime runtime.json --source source.json');options[args[i]]=args[i+1];}
if(Object.keys(options).length!==5)throw Error('Five explicit package paths required');
const out=path.resolve(options['--out']),launcher=path.resolve(options['--launcher']),sha=b=>createHash('sha256').update(b).digest('hex'),read=p=>fs.readFileSync(p),json=p=>JSON.parse(read(p)),write=(p,b)=>{fs.mkdirSync(path.dirname(p),{recursive:true});fs.writeFileSync(p,b,{flag:'wx'});};
if(fs.existsSync(out)||fs.existsSync(launcher))throw Error('Candidate exists; choose a fresh output directory');
const config=json(options['--config']),runtime=json(options['--runtime']),source=json(options['--source']);
const runtimePin={path:options['--runtime'].replaceAll('\\','/'),sha256:sha(read(options['--runtime']))},sourcePin={path:options['--source'].replaceAll('\\','/'),sha256:sha(read(options['--source']))};
for(const p of [runtimePin,sourcePin])if(path.isAbsolute(p.path)||/[\\:]/.test(p.path)||p.path.split('/').some(v=>!v||v==='.'||v==='..'))throw Error('Profiles must be repository relative paths');
if(config.runtimeApprovalSha256!==runtimePin.sha256||config.sourceReviewSha256!==sourcePin.sha256||runtime.serviceId!==config.serviceId||!runtime.sourceApproved||runtime.installApproved||runtime.packaged||runtime.actualNoitaEvidence||source.runtimeApproval?.sha256!==runtimePin.sha256)throw Error('Mismatched development approval/config inputs');
const pins=new Map(),external=new Set(),seen=new Set(),root=process.cwd(),require=createRequire(import.meta.url);
function input(p){p=p.replaceAll('\\','/');if(!pins.has(p))pins.set(p,{path:p,sha256:sha(read(p))});return read(p);}
function copy(p){if(!fs.existsSync(path.join(out,p)))write(path.join(out,p),input(p));}
function resolveModule(from,spec){if(spec.startsWith('node:'))return null;if(spec.startsWith('@nt2/')){const [scope,name,...sub]=spec.split('/'),pkg=json('packages/'+name+'/package.json'),key=sub.length?'./'+sub.join('/') : '.',exp=typeof pkg.exports==='string'?pkg.exports:pkg.exports[key];if(!exp)throw Error('Unknown workspace export '+spec);return path.posix.normalize('packages/'+name+'/'+exp);}if(!spec.startsWith('.')){external.add(spec.startsWith('@')?spec.split('/').slice(0,2).join('/'):spec.split('/')[0]);return null;}let p=path.posix.normalize(path.posix.join(path.posix.dirname(from),spec));if(!fs.existsSync(p)&&p.endsWith('.js')&&fs.existsSync(p.slice(0,-3)+'.ts'))p=p.slice(0,-3)+'.ts';if(!fs.existsSync(p))throw Error('Missing local module '+p);return p;}
const adapter=`import {readFile} from 'node:fs/promises';import {createHash} from 'node:crypto';
export const rejoinRuntimePath=${JSON.stringify(runtimePin.path)},rejoinSourcePath=${JSON.stringify(sourcePin.path)};
const root=new URL('../../../../',import.meta.url),sha=b=>createHash('sha256').update(b).digest('hex');
export async function verifyRejoinPackage(reader=u=>readFile(u)){const b=await reader(new URL(rejoinRuntimePath,root)),s=await reader(new URL(rejoinSourcePath,root));if(sha(b)!==${JSON.stringify(runtimePin.sha256)}||sha(s)!==${JSON.stringify(sourcePin.sha256)})throw Error('Frozen approval mismatch');const r=JSON.parse(b);if(r.id!==${JSON.stringify(runtime.id)}||r.serviceId!==${JSON.stringify(runtime.serviceId)}||!r.sourceApproved||r.installApproved||r.packaged||r.actualNoitaEvidence)throw Error('Frozen development scope mismatch');return {...r,runtimeResourceJson:b.toString(),runtimeApproval:${JSON.stringify(runtimePin)},desktopApproval:${JSON.stringify(sourcePin)}};}
`;
function module(p){
 if(seen.has(p))return;seen.add(p);
 if(p==='apps/service/src/rejoin/package-profile.ts'){input(p);write(out+'/apps/service/src/rejoin/package-profile.js',adapter);return;}
 // This package has one fixed normal selector. Legacy inspection/native
 // profiles are imported eagerly by shared source but never selected here.
 // Fail closed on those alternate launch modes instead of shipping their
 // historical profiles and their build/source reconstruction inputs.
 if(p==='apps/service/src/g2-next/profile.ts'){input(p);write(out+'/apps/service/src/g2-next/profile.js',`export function validateCandidateMetadata(o){if(o.profile)throw Error('Host package supports only its pinned normal profile');}export function inspectionMetadata(){throw Error('Inspection launcher is not packaged');}export function verifyInspectionArtifacts(){throw Error('Inspection launcher is not packaged');}`);return;}
 let code=input(p).toString();if(p.endsWith('.ts'))code=ts.transpileModule(code,{fileName:p,compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText;
 if(p==='apps/service/src/g3/native-profile.ts')code=code.replace(/export const nativeDevelopmentProfile = JSON\.parse\(await readFile\([^\n]+\);/,"export const nativeDevelopmentProfile = null;");
 if(p==='apps/service/src/rejoin/lan.ts')for(const [before,after]of [['ac6ed13ccaf57c604d25c58e75bed5dd2ac5fec2b071f4f24cf7ebdc73f7852f',runtimePin.sha256],['9cf69c87b141274ce64dfaa9ebcd93006278e8301822200a423afe5d6328526c',sourcePin.sha256]]){if(!code.includes(before))throw Error('LAN adapter anchor missing');code=code.replaceAll(before,after);}
 const ast=ts.createSourceFile(p,code,ts.ScriptTarget.Latest,true,ts.ScriptKind.JS),edits=[];
 function visit(n){if((ts.isImportDeclaration(n)||ts.isExportDeclaration(n))&&n.moduleSpecifier&&ts.isStringLiteral(n.moduleSpecifier)){const spec=n.moduleSpecifier.text,dep=resolveModule(p,spec);if(dep){module(dep);if(spec.startsWith('@nt2/')){let rel=path.posix.relative(path.posix.dirname(p),dep).replace(/\.ts$/,'.js');if(!rel.startsWith('.'))rel='./'+rel;edits.push([n.moduleSpecifier.getStart(ast),n.moduleSpecifier.end,JSON.stringify(rel)]);}}}if(ts.isCallExpression(n)&&n.expression.getText(ast)==='require'&&n.arguments.length===1&&ts.isStringLiteral(n.arguments[0])&&n.arguments[0].text.startsWith('.'))copy(path.posix.normalize(path.posix.join(path.posix.dirname(p),n.arguments[0].text)));ts.forEachChild(n,visit);}visit(ast);
 for(const [start,end,text]of edits.sort((a,b)=>b[0]-a[0]))code=code.slice(0,start)+text+code.slice(end);
 write(path.join(out,p.replace(/\.ts$/,'.js')),code);
}
module('apps/service/scripts/g3-rejoin-lan.ts');
function walk(dir){return fs.readdirSync(dir,{withFileTypes:true}).flatMap(e=>{const p=dir+'/'+e.name;if(e.isSymbolicLink())throw Error('Build input link '+p);return e.isDirectory()?walk(p):[p];});}
// Data selected by the normal production branch, rather than historical profiles
// or generated language bindings and fixtures. Keep all migration SQL.
for(const p of walk('packages/contracts').filter(p=>p.endsWith('.json')&&!/\/(test|scripts)\//.test(p)&&!/(golden|fixture|vectors)/i.test(p)))copy(p);
for(const folder of ['apps/web/candidate','apps/web/g3','db/migrations','deploy/g2-lan/public'])for(const p of walk(folder).filter(p=>!p.endsWith('.map')))copy(p);
copy('packages/contracts/src/room-semantics.js');copy(runtimePin.path);copy(sourcePin.path);
write(out+'/config.json',input(options['--config']));write(out+'/package.json',JSON.stringify({private:true,type:'module'}));
const packages=new Map(),queue=[...external];
const excluded=new Set(['test','tests','__tests__','test-fixtures','fixtures','example','examples','benchmark','benchmarks','bench','docs','doc','coverage','.github','.git','scripts']);
function dependencyFiles(folder,prefix=''){return fs.readdirSync(folder,{withFileTypes:true}).flatMap(e=>{if(e.isSymbolicLink())throw Error('Dependency link '+folder+'/'+e.name);const rel=prefix+e.name;if(e.isDirectory())return excluded.has(e.name)||e.name.startsWith('.')?[]:dependencyFiles(folder+'/'+e.name,rel+'/');return /(^|\/)(LICENSE|LICENCE|COPYING|NOTICE|AUTHORS)(\.|$)/i.test(rel)||/\.(js|mjs|cjs|json|node|proto|pem|wasm)$/.test(rel)&&!e.name.endsWith('.map')&&!['tsconfig.json','eslint.config.js'].includes(e.name)?[rel]:[];});}
while(queue.length){const name=queue.shift();if(packages.has(name)||name.startsWith('@nt2/'))continue;const folder='node_modules/'+name,pkg=json(folder+'/package.json');packages.set(name,pkg.version);const included=dependencyFiles(folder);for(const p of included)copy(folder+'/'+p);
 // Some npm distributions put their only copyright/license text in README.
 if(!included.some(p=>/(^|\/)(LICENSE|LICENCE|COPYING|NOTICE)(\.|$)/i.test(p)))for(const p of fs.readdirSync(folder).filter(p=>/^readme(?:\.|$)/i.test(p)))if(/license|licence|copyright/i.test(read(folder+'/'+p).toString()))copy(folder+'/'+p);
 queue.push(...Object.keys(pkg.dependencies??{}));for(const dep of Object.keys(pkg.optionalDependencies??{}))try{require.resolve(dep+'/package.json');queue.push(dep);}catch{}}
for(const p of ['LICENSE','LICENSE.md','COPYING','NOTICE'])if(fs.existsSync(p))copy(p);
write(out+'/boot.mjs',input('apps/service/scripts/host-bootstrap.mjs'));
input('apps/service/scripts/package-host.mjs');const launcherTemplate=input('apps/service/scripts/host-launcher.ps1').toString();
const manifest={format:'nt2-host-1',id:'nt2-development-host',serviceId:config.serviceId,origin:config.origin,namespace:config.namespace,nodeVersion:process.version,config:{path:'config.json',sha256:sha(read(options['--config']))},runtimeApproval:runtimePin,sourceApproval:sourcePin,sourcePins:[...pins.values()].filter(p=>!p.path.startsWith('node_modules/')),thirdPartyDependencies:[...packages].map(([name,version])=>({name,version})),files:walk(out).map(p=>({path:p.slice(out.length+1),sha256:sha(read(p))})).sort((a,b)=>a.path.localeCompare(b.path))};
write(out+'/manifest.json',JSON.stringify(manifest,null,2)+'\n');const manifestSha256=sha(read(out+'/manifest.json')),bootstrapSha256=sha(read(out+'/boot.mjs'));
write(launcher,launcherTemplate.replaceAll('@MANIFEST_SHA256@',manifestSha256).replaceAll('@BOOTSTRAP_SHA256@',bootstrapSha256));
console.log(JSON.stringify({format:'nt2-host-package-1',artifact:out,launcher,manifestSha256,bootstrapSha256,launcherSha256:sha(read(launcher)),files:manifest.files.length,bytes:manifest.files.reduce((n,p)=>n+fs.statSync(out+'/'+p.path).size,0),runtimeApproval:runtimePin,sourceApproval:sourcePin,config:manifest.config,serviceStarted:false,databaseTouched:false}));
