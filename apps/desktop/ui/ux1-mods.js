// Adapted from legacy nt-app/src/views/Room.vue modList and expanding Mods rows.
// The legacy vModUserTooltip name lookup is keyed by identity, never by name.
// Current roster desktopId = one player on both app and web. Counts descend;
// names ascend case-insensitively, then by exact identity for deterministic ties.
const uxMods={scope:null,expanded:new Set()};
function uxModList(v){
 const players=new Map((v.room?.members??[]).filter(m=>typeof m.desktopId==='string').map(m=>[m.desktopId,m]));
 const mods=new Map();let known=0;
 for(const[id,member]of players){
  const reports=(v.inspections??[]).filter(r=>r.desktopId===id);
  const r=reports.length===1?reports[0]:null;
  if(!r||r.inventoryComplete!==true||r.collectorFailure||!Array.isArray(r.activeMods))continue;
  known++;
  for(const name of new Set(r.activeMods.filter(name=>typeof name==='string'))){
   if(!mods.has(name))mods.set(name,{name,users:[]});
   mods.get(name).users.push({id,name:uxName(member)});
  }
 }
 const cmp=(a,b)=>a>b?1:a<b?-1:0;
 return{known,total:players.size,rows:[...mods.values()].sort((a,b)=>b.users.length-a.users.length||cmp(a.name.toLowerCase(),b.name.toLowerCase())||cmp(a.name,b.name))};
}
function uxRenderMods(v){
 const scope=v.signed_in&&v.room?JSON.stringify([scopeOf(v),v.room.roomId,v.room.runId,v.room.runEpoch,v.room.configRevision]):null;
 if(scope!==uxMods.scope){uxMods.scope=scope;uxMods.expanded.clear();}
 const list=scope?uxModList(v):{known:0,total:0,rows:[]};
 const names=new Set(list.rows.map(r=>r.name));for(const name of uxMods.expanded)if(!names.has(name))uxMods.expanded.delete(name);
 text(el('mods-coverage'),!scope?'':list.known<list.total?`${list.known} of ${list.total} players inspected · Incomplete coverage: awaiting complete game inspections.`:`${list.known} of ${list.total} players inspected · Complete coverage.`);
 text(el('mods-empty'),!scope?'':list.rows.length?'':list.known<list.total?'Awaiting complete mod inspections.':'No active mods reported.');
 reconcile(el('mods-rows'),list.rows,r=>r.name,()=>{
  const row=uxNode('tr'),cell=uxNode('td'),button=uxNode('button'),name=uxNode('span'),count=uxNode('span'),users=uxNode('ul');
  button.type='button';button.className='mod-expand';button.append(name,count);cell.append(button,users);row.append(cell);return row;
 },(row,r)=>{
  const cell=row.firstChild,button=cell.firstChild,users=cell.lastChild,open=uxMods.expanded.has(r.name);
  text(button.firstChild,r.name);text(button.lastChild,`${r.users.length} ${r.users.length===1?'player':'players'}`);
  button.setAttribute('aria-label',`${r.name} · ${r.users.length} ${r.users.length===1?'player':'players'}`);
  button.setAttribute('aria-expanded',String(open));button.onclick=()=>{if(uxMods.expanded.has(r.name))uxMods.expanded.delete(r.name);else uxMods.expanded.add(r.name);uxRenderMods(current);};
  users.hidden=!open;
  reconcile(users,r.users,u=>u.id,()=>uxNode('li'),(node,u)=>{text(node,u.name);node.dataset.playerId=u.id;});
 });
}
