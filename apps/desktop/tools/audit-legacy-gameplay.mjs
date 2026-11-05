// Read-only source inventory; never executes legacy Lua or edits shared scope authority.
import fs from 'node:fs';import path from 'node:path';import {createHash} from 'node:crypto';import {fileURLToPath} from 'node:url';
const desktop=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const source=path.resolve(desktop,'../../../noita-together-main');
const entries=[],files=[];
const rules=[
    ['append',/ModLuaFileAppend\s*\(\s*["']([^"']+)["']\s*,\s*["']([^"']+)["']/g],
    ['integration',/ModIsEnabled\s*\(\s*["']([^"']+)["']/g],
    ['event_handler',/(customEvents|wsEvents)\s*\[\s*["']([^"']+)["']\s*\]\s*=\s*function/g],
    ['outgoing_event',/event\s*=\s*["']([^"']+)["']/g],
    ['custom_event',/name\s*=\s*["'](PlayerPOI|FungalShift|TeamPerk|Coop[^"']*|Emote|Skin|SecretHourglass|Nemesis[^"']*)["']/g],
    ['room_flag',/["'](NT_(?:GAMEMODE|sync|send|team|alt|ban|death|world|NEMESIS|is_host)[A-Za-z0-9_]*)["']/g],
    ['setting',/id\s*=\s*["'](NT_[A-Za-z0-9_]+)["']/g],
    ['nemesis_ability',/\b([A-Za-z0-9_]+)\s*=\s*\{\s*id\s*=\s*["']([A-Za-z0-9_]+)["']/g],
    ['hourglass',/RegisterHourglassEvent\s*\(\s*["']([^"']+)["']\s*,\s*["']([^"']+)["']/g],
    ['item_entity',/["'](data\/entities\/items\/[^"']+\.xml)["']/g],
    ['callback',/function\s+(On[A-Za-z0-9_]+)\s*\(/g],
];
function walk(dir){for(const f of fs.readdirSync(dir,{withFileTypes:true}).sort((a,b)=>a.name.localeCompare(b.name))){const full=path.join(dir,f.name);if(f.isDirectory())walk(full);else if(f.name.endsWith('.lua')){
    const raw=fs.readFileSync(full,'utf8'),relative=path.relative(source,full).replaceAll('\\','/');files.push({file:relative,sha256:createHash('sha256').update(raw).digest('hex')});
    // Preserve line positions while excluding Lua long comments and single-line comments.
    const clean=raw.replace(/--\[(=*)\[[\s\S]*?\]\1\]/g,m=>m.replace(/[^\n]/g,' ')).replace(/--[^\n]*/g,m=>' '.repeat(m.length));
    for(const [kind,pattern]of rules){for(const match of clean.matchAll(pattern)){if(kind==='nemesis_ability'&&!relative.endsWith('nemesis/files/events.lua'))continue;const line=clean.slice(0,match.index).split('\n').length;entries.push({kind,file:relative,line,values:match.slice(1)});}}
}}}
walk(path.join(source,'noita_mod/core'));walk(path.join(source,'noita_mod/nemesis'));
entries.sort((a,b)=>a.file.localeCompare(b.file)||a.line-b.line||a.kind.localeCompare(b.kind));
const output={revision:'legacy-core-nemesis-audit-1',scope:'Source index supporting G1 parity review; not shared inventory authority or gameplay acceptance',sourceRoot:source.replaceAll('\\','/'),files,entries};
const target=path.join(desktop,'gameplay-source-index.json');const data=JSON.stringify(output,null,2)+'\n';
if(process.argv.includes('--check')){if(fs.readFileSync(target,'utf8')!==data)throw Error('Gameplay source index drift');}else fs.writeFileSync(target,data);
console.log(`PASS gameplay source index: ${files.length} Lua files, ${entries.length} indexed references; ${new Set(entries.filter(e=>e.kind==='nemesis_ability').map(e=>e.values[1])).size} Nemesis abilities.`);
