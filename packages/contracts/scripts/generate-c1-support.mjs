// Canonical fixed local catalogue module; no network paths become executable.
import fs from 'node:fs';import {createHash} from 'node:crypto';
const bytes=fs.readFileSync('packages/contracts/g3-c1/catalogue.json'),c=JSON.parse(bytes),sha=createHash('sha256').update(bytes).digest('hex');
const lua=value=>typeof value==='string'?JSON.stringify(value).replace(/\\u([0-9a-fA-F]{4})/g,(_,h)=>[...Buffer.from(String.fromCharCode(parseInt(h,16)))].map(b=>'\\'+String(b).padStart(3,'0')).join('')):typeof value==='number'||typeof value==='boolean'?String(value):Array.isArray(value)?'{'+value.map(lua).join(',')+'}':'{'+Object.entries(value).map(([k,v])=>'['+lua(k)+']='+lua(v)).join(',')+'}';
const keyed=rows=>Object.fromEntries(rows.map(v=>[v.id,v]));
const output='packages/contracts/g3-c1/catalogue.lua',generated='-- GENERATED fixed approved metadata. Base-game assets remain local references.\nreturn '+lua({revision:c.revision,sha256:sha,perks:keyed(c.perks),orbs:keyed(c.orbs),materials:Object.fromEntries(c.materials.map(id=>[id,true])),teamPerks:Object.fromEntries(c.teamPerks.map(id=>[id,true])),bannedPerks:Object.fromEntries(c.bannedPerks.map(id=>[id,true]))})+'\n';
if(process.argv.includes('--check')){if(fs.readFileSync(output,'utf8')!==generated)throw Error('Generated item support drift');}else fs.writeFileSync(output,generated);
console.log(JSON.stringify({revision:c.revision,sha256:sha,lua:output,perks:c.perks.length,orbs:c.orbs.length,materials:c.materials.length}));
