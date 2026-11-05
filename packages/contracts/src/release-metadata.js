// Public static distribution metadata. Informational only: never gameplay authority.
const version=/^(0|[1-9][0-9]{0,3})\.(0|[1-9][0-9]{0,3})\.(0|[1-9][0-9]{0,3})(?:-(?:alpha|beta|rc)\.[1-9][0-9]{0,3})?$/;
const exact=(v,keys)=>v&&typeof v==='object'&&!Array.isArray(v)&&Object.keys(v).sort().join(',')===[...keys].sort().join(',');
const need=v=>{if(!v)throw Error('Invalid release metadata');};
export function validateReleaseMetadata(m){
 need(m&&m.schemaVersion===1&&['pending','available'].includes(m.status)&&typeof m.version==='string'&&version.test(m.version)&&m.platform==='windows-x64');
 if(m.status==='pending'){need(exact(m,['schemaVersion','status','version','platform']));return m;}
 need(exact(m,['schemaVersion','status','version','platform','builtAt','signed','artifact','compatibility']));
 need(typeof m.builtAt==='string'&&/^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d(?:\.\d{3})?Z$/.test(m.builtAt)&&Number.isFinite(Date.parse(m.builtAt))&&new Date(m.builtAt).toISOString()===m.builtAt.replace(/(?<!\.\d{3})Z$/,'.000Z')&&typeof m.signed==='boolean');
 need(exact(m.artifact,['file','bytes','sha256'])&&m.artifact.file===`NoitaTogether2-${m.version}-windows-x64.zip`&&Number.isSafeInteger(m.artifact.bytes)&&m.artifact.bytes>0&&m.artifact.bytes<=2147483648&&typeof m.artifact.sha256==='string'&&/^[a-f0-9]{64}$/.test(m.artifact.sha256));
 const keys=['profileId','gameVersion','modVersion','bridgeVersion','lobbyRevision','gameplayRevision'];need(exact(m.compatibility,keys)&&keys.every(k=>typeof m.compatibility[k]==='string'&&/^[A-Za-z0-9][A-Za-z0-9._ -]{0,127}$/.test(m.compatibility[k])));
 return m;
}
export function releaseLinks(metadata){const m=validateReleaseMetadata(metadata);return m.status==='available'?{artifact:`/downloads/${m.version}/${m.artifact.file}`,manifest:`/releases/${m.version}/manifest.json`,checksums:`/releases/${m.version}/SHA256SUMS.txt`}:null;}
