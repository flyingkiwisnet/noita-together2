// Render only structured text. Never interpolate client names/provenance into HTML/attributes.
export function renderStatistics(container,response){
 const document=container.ownerDocument??globalThis.document;
 const nodes=(response.statistics??[]).map(record=>{const row=document.createElement('li');row.textContent=`${record.displayName} · ${record.kind} · ${record.count} · ${record.provenance} · ${record.capturedAt}`;return row;});container.replaceChildren(...nodes);
}
export function renderRunChoices(container,runs){
 const document=container.ownerDocument??globalThis.document,choose=document.createElement('option');choose.value='';choose.textContent='Choose a recorded run';
 const nodes=runs.map(run=>{const option=document.createElement('option');option.value=run.sequence;option.textContent=`${run.roomName||'Run'} · ${run.createdAt} · ${run.state} · ${run.collectionState}`;return option;});container.replaceChildren(choose,...nodes);
}
