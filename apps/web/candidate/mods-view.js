// Adapted from legacy nt-app/src/views/Room.vue modList: count descending,
// case-insensitive name ascending. Map/Set preserve exact IDs safely and count
// current desktop players once, even when display names or mod entries repeat.
export function aggregateMods(members,reports){
 const players=new Map(members.filter(m=>m.desktopId).map(m=>[m.desktopId,m])),mods=new Map();let inspected=0;
 for(const [id,m]of players){const r=reports.get(id);if(!r||r.desktopId!==id||r.inventoryComplete!==true||r.collectorFailure||!Array.isArray(r.activeMods))continue;inspected++;
  for(const name of new Set(r.activeMods)){if(typeof name!=='string')continue;let users=mods.get(name);if(!users)mods.set(name,users=[]);users.push({desktopId:id,name:m.principal?.displayName??'Player'});}}
 const compare=(a,b)=>a<b?-1:a>b?1:0;
 return {total:players.size,inspected,unknown:players.size-inspected,rows:[...mods].map(([name,users])=>({name,users,count:users.length})).sort((a,b)=>b.count-a.count||compare(a.name.toLowerCase(),b.name.toLowerCase())||compare(a.name,b.name))};
}
export function createModsView(document){
 const list=document.getElementById('mods-list'),coverage=document.getElementById('mods-coverage'),empty=document.getElementById('mods-empty');let scope=null;const rows=new Map();
 function reset(){scope=null;rows.clear();list.replaceChildren();coverage.textContent='Awaiting inspection.';empty.hidden=false;empty.textContent='No inspected mods available.';}
 function render(nextScope,members,reports){if(nextScope!==scope){reset();scope=nextScope;}const value=aggregateMods(members,reports),present=new Set(value.rows.map(r=>r.name));for(const name of rows.keys())if(!present.has(name))rows.delete(name);
  coverage.textContent=value.inspected+' / '+value.total+' current players inspected'+(value.unknown?' · '+value.unknown+' awaiting complete inspection. Counts cover inspected players only.':'.');empty.hidden=value.rows.length>0;empty.textContent=value.unknown?'No inspected mods available yet.':'No active mods reported by inspected players.';
  const focused=document.activeElement,nodes=[];
  for(const mod of value.rows){let row=rows.get(mod.name);if(!row){const item=document.createElement('li'),button=document.createElement('button'),users=document.createElement('ul');button.type='button';button.className='mod-toggle';button.setAttribute('aria-expanded','false');users.hidden=true;button.addEventListener('click',()=>{users.hidden=!users.hidden;button.setAttribute('aria-expanded',String(!users.hidden));});item.append(button,users);row={item,button,users};rows.set(mod.name,row);}row.button.textContent=mod.name+' · '+mod.count+' player'+(mod.count===1?'':'s');row.users.replaceChildren(...mod.users.map(user=>{const li=document.createElement('li');li.textContent=user.name;return li;}));nodes.push(row.item);}
  list.replaceChildren(...nodes);if([...rows.values()].some(r=>r.button===focused))focused.focus();return value;
 }
 reset();return {render,reset};
}
