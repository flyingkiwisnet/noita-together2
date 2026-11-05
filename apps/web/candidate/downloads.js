import {validateReleaseMetadata,releaseLinks} from '/packages/contracts/src/release-metadata.js';
export function renderRelease(document,metadata){
 const status=document.getElementById('release-status'),details=document.getElementById('release-details'),links=document.getElementById('release-links');
 details.replaceChildren();links.replaceChildren();
 const m=validateReleaseMetadata(metadata);
 status.textContent=m.status==='pending'?`Windows beta ${m.version} is being prepared. Downloads will appear here after release approval.`:`Windows beta ${m.version}`;
 if(m.status==='pending')return;
 const p=document.createElement('p');p.textContent=`Windows x64 · Built ${m.builtAt.slice(0,10)} · ${(m.artifact.bytes/1048576).toFixed(1)} MiB · ${m.signed?'Signed':'Unsigned Windows build'}`;details.append(p);
 const hash=document.createElement('p');hash.className='checksum';hash.textContent=`SHA-256: ${m.artifact.sha256}`;details.append(hash);
 for(const [key,label] of [['artifact','Download Windows Beta'],['manifest','Release manifest'],['checksums','SHA-256 checksums']]){const a=document.createElement('a');a.href=releaseLinks(m)[key];a.textContent=label;if(key==='artifact')a.className='download-button';links.append(a);}
}
export async function loadRelease(document,fetcher=fetch){
 try{const response=await fetcher('/releases/latest.json',{credentials:'omit',cache:'no-store',redirect:'error'});if(!response.ok)throw Error('Unavailable');renderRelease(document,await response.json());}
 catch{document.getElementById('release-details').replaceChildren();document.getElementById('release-links').replaceChildren();document.getElementById('release-status').textContent='Downloads are temporarily unavailable. Please try again later.';}
}
if(typeof document!=='undefined')void loadRelease(document);
