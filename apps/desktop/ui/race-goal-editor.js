// Shared import-free DOM editor. Supply RACE_DEFAULT from race-goal-ui.js as fourth argument.
export function createRaceEditor(document,form,catalogue,defaultRace){
 const node=(tag,text)=>{const el=document.createElement(tag);if(text)el.textContent=text;return el;};
 const fieldset=node('fieldset'),legend=node('legend','Race goals'),hint=node('p','Select every required goal. One life, shared seed, sharing off. Classic Ending must follow the other goals. Overlapping objectives can advance on the same event.'),list=node('div'),add=node('button','Add goal'),duration=node('input'),hidden=node('input');
 const help=node('details'),summary=node('summary','Timing and supported content');help.append(summary,node('p','Progress is game-reported; times use server acceptance. New reports at or after the deadline do not count. Full finishers keep their order; unfinished entrants rank by goals completed, then time first reaching that count. Exact ties share a place. Classic +33 orbs is unavailable. NG+ and modified-map Parallel Worlds are unavailable. Physical availability of33 orbs and exact Parallel World edge timing still need game verification.'));
 hidden.type='hidden';hidden.name='raceDraft';duration.type='number';duration.min='0';duration.max='10080';duration.step='0.001';duration.value='0';add.type='button';add.className='race-goal-add';hint.className='race-goal-intro';list.className='race-goal-list';const durationLabel=node('label','Maximum duration in minutes (0 = off)');durationLabel.className='race-goal-duration';durationLabel.append(duration);fieldset.className='race-goal-editor';fieldset.append(legend,hint,list,add,durationLabel,help,hidden);const submit=Array.from(form.children).find(el=>el.tagName==='BUTTON'&&el.type==='submit');if(submit)form.insertBefore(fieldset,submit);else form.append(fieldset);
 let rows=[],serial=0;
 const update=()=>{hidden.value=JSON.stringify({registryRevision:catalogue.content.revision,durationMs:Math.round(Number(duration.value)*60000),goals:rows.map(r=>r.read())});for(const r of rows)r.describe();add.disabled=rows.length>=16;};
 const option=(select,value,text)=>{const el=node('option',text);el.value=value;select.append(el);};
 function row(initial){
  const root=node('div'),type=node('select'),parameters=node('span'),remove=node('button','×'),description=node('p');root.className='race-goal-row';root.setAttribute('role','group');type.className='race-goal-type';type.setAttribute('aria-label','Goal type');parameters.className='race-goal-parameters';description.className='race-goal-description';remove.type='button';remove.className='race-goal-remove';remove.title='Remove goal';
  for(const g of catalogue.schema.goals)if(catalogue.content.goalSupport[g.id]?.available)option(type,g.id,g.name??g.id.replaceAll('_',' '));
  type.value=initial.id;let instanceId=initial.instanceId;while(!instanceId){const next='goal_'+(++serial);if(!rows.some(r=>r.read().instanceId===next))instanceId=next;}let controls={};
  description.id='race-description-'+(form.id??'draft')+'-'+instanceId;type.setAttribute('aria-describedby',description.id);
  const result={root,type,read:()=>{const g={instanceId,id:type.value};for(const [k,el]of Object.entries(controls))g[k]=el.multiple?Array.from(el.selectedOptions).map(o=>o.value):['target','index'].includes(k)?Number(el.value):el.value;return g;},describe:()=>{
   const g=result.read(),definition=catalogue.schema.goals.find(e=>e.id===g.id),levels=['easy','medium','hard','extreme'];let difficulty=definition.difficulty;
   if(g.id==='visit_biome')difficulty=catalogue.content.biomes.find(e=>e.id===g.biomeId)?.difficulty;
   else if(g.id==='defeat_named_bosses'&&g.bossIds?.length){const hardest=Math.max(...g.bossIds.map(id=>levels.indexOf(catalogue.content.bosses.find(e=>e.id===id)?.difficulty)));difficulty=levels[Math.min(3,hardest+(g.bossIds.length>=3?1:0))];}
   else if(definition.difficultyBands)difficulty=definition.difficultyBands.find(([max])=>(g.target??g.index)<=max)?.[1];
   description.textContent=definition.description+(difficulty?' · Provisional difficulty: '+difficulty:'');root.setAttribute('aria-label',(definition.name??g.id.replaceAll('_',' '))+' goal');remove.setAttribute('aria-label','Remove '+(definition.name??g.id.replaceAll('_',' '))+' goal');
  }};
  const render=(draft={})=>{
   parameters.replaceChildren();controls={};const definition=catalogue.schema.goals.find(g=>g.id===type.value);description.textContent=definition.description;root.dataset.goal=type.value;parameters.dataset.kind=type.value;parameters.hidden=Object.keys(definition.parameters).length===0;
   for(const [key,rule]of Object.entries(definition.parameters)){
    const label=node('label',({bossIds:'Required bosses (all selected)',biomeId:'Biome',target:'Target',direction:'Direction',index:'World level'})[key]??key),el=node(rule.type==='integer'?'input':'select');label.className='race-goal-parameter';el.className='race-goal-field';el.setAttribute('aria-describedby',description.id);controls[key]=el;
    if(rule.type==='integer'){el.type='number';el.min=String(rule.min);el.max=String(rule.max);el.step='1';el.value=String(draft[key]??definition.presets?.[0]??rule.min);}
    else if(rule.type==='enum'){for(const v of rule.values)option(el,v,v);el.value=draft[key]??rule.values[0];}
    else {el.multiple=rule.type==='ids';if(el.multiple){el.size=6;label.className+=' is-multiple';}for(const entry of catalogue.content[rule.registry].filter(e=>e.available))option(el,entry.id,entry.name);if(el.multiple){for(const o of el.options)o.selected=(draft[key]??[]).includes(o.value);}else if(draft[key])el.value=draft[key];}
    el.required=true;label.append(el);parameters.append(label);
   }
   update();
  };
  type.addEventListener('change',()=>render());remove.addEventListener('click',()=>{const index=rows.indexOf(result),focused=document.activeElement===remove;rows=rows.filter(r=>r!==result);root.remove();update();if(focused)(rows[index]?.type??rows[index-1]?.type??add).focus();hidden.dispatchEvent(new Event('input',{bubbles:true}));});root.addEventListener('input',update);root.addEventListener('change',update);root.append(type,parameters,remove,description);list.append(root);rows.push(result);render(initial);return result;
 }
 const load=value=>{rows=[];serial=0;list.replaceChildren();const v=value??defaultRace;duration.value=String(v.durationMs/60000);for(const g of v.goals)row(g);update();};
 add.addEventListener('click',()=>{const focused=document.activeElement===add,added=row({id:'reach_max_hp'});update();if(focused)added.type.focus();hidden.dispatchEvent(new Event('input',{bubbles:true}));});duration.addEventListener('input',update);
 const mode=enabled=>{fieldset.hidden=form.elements.mode.value!=='race';fieldset.disabled=!enabled;};
 form.elements.mode.addEventListener('change',()=>mode(!form.elements.mode.disabled));load();mode(true);return {load,mode,fieldset};
}
