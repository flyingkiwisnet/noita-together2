import {createRaceEditor} from './race-goal-editor.js';
import {RACE_DEFAULT,applyRaceDraft,raceGoalLabel} from './race-goal-ui.js';
import {catalogue} from './race-goal-data.js';
const {content}=catalogue;
window.NT2_RACE_UI={
 attach(form,config){form.nt2RaceEditor?.fieldset.remove();const editor=createRaceEditor(document,form,catalogue,RACE_DEFAULT);editor.fieldset.dataset.raceEditor=form.id;editor.load(config?.race);form.nt2RaceEditor=editor;
  const mode=form.elements.mode;if(mode&&!mode.dataset.raceModeBound){mode.dataset.raceModeBound='true';mode.addEventListener('change',()=>uxRaceMode(form,!mode.disabled));}
  const target=document.getElementById(form.id==='lobby-config'?'config-fields':'create-fields');target.append(editor.fieldset);for(const field of editor.fieldset.querySelectorAll('input,select,button'))field.setAttribute('form',form.id);
  for(const event of ['input','change'])editor.fieldset.addEventListener(event,()=>form.dataset.dirty='true');
 },
 configure(config,form,validate=true){if(config.mode==='race'&&validate)return applyRaceDraft(config,form.elements.raceDraft?.value,catalogue);if(config.mode==='race')return {...config,race:JSON.parse(form.elements.raceDraft.value)};return applyRaceDraft(config,undefined,catalogue);},
 render(view){
  let root=document.getElementById('race-standings');if(!root){root=document.createElement('section');root.id='race-standings';document.getElementById('tab-race').append(root);}
  const active=view.room?.config?.mode==='race';document.getElementById('race-tab').hidden=!active;if(!active&&ux.tab==='race')ux.tab='players';root.hidden=!active;if(root.hidden){root.replaceChildren();delete root.dataset.revision;return;}
  const snapshot=view.g3?.race?.snapshot;
  if(!snapshot?.config){root.replaceChildren();delete root.dataset.revision;const title=document.createElement('h3');title.textContent='Selected Race goals';const status=document.createElement('p');status.textContent=view.g3?.race?.detectorFailure??'Waiting for inspected host Start.';const goals=document.createElement('ul');for(const goal of view.room.config.race?.goals??[]){const item=document.createElement('li');item.textContent=raceGoalLabel(goal,catalogue);goals.append(item);}root.append(title,status,goals);return;}
  const status=(snapshot.status==='ended'?'Final results':'Race in progress')+' · Elapsed '+Math.floor(Number(snapshot.elapsedMs??0)/1000)+'s'+(snapshot.deadlineAt?' · Deadline '+new Date(snapshot.deadlineAt).toLocaleString():'')+(view.g3.race.error?' · Reconnecting results':'');
  const revision=JSON.stringify([snapshot.runKey,snapshot.revision,snapshot.collectionState]);if(root.dataset.revision===revision){root.querySelector('[data-race-time]').textContent=status;return;}
  const expanded=new Set([...root.querySelectorAll('details')].filter(n=>n.open).map(n=>n.dataset.row));root.replaceChildren();root.dataset.revision=revision;
  const node=(tag,text)=>{const n=document.createElement(tag);n.textContent=text;return n;};const timer=node('p',status);timer.dataset.raceTime='true';root.append(node('h3','Race goals and standings'),timer);
  const goals=node('ul','');for(const goal of snapshot.config.goals)goals.append(node('li',raceGoalLabel(goal,catalogue)));root.append(goals);
  const ranked=(snapshot.entrants??[]).map((row,index)=>({row,index})).sort((a,b)=>{const rank=r=>Number(r.rank)>0?Number(r.rank):Infinity;return rank(a.row)-rank(b.row)||a.index-b.index;});
  for(const {row} of ranked){const details=document.createElement('details');details.dataset.row=row.rowKey;details.open=expanded.has(row.rowKey);details.append(node('summary',(row.rank?'#'+row.rank+' ':'')+row.displayName+' · '+row.completedGoals+'/'+snapshot.config.goals.length+' · '+row.status+(row.finalCountAtMs?' · '+(Number(row.finalCountAtMs)/1000).toFixed(3)+'s':'')));
   const list=node('ul','');for(const goal of snapshot.config.goals){const p=row.progress?.find(p=>p.instanceId===goal.instanceId);if(!p)continue;const item=node('li',raceGoalLabel(goal,catalogue)+' · '+p.value+'/'+p.target+(p.complete?' · Complete at '+(Number(p.completedAtMs)/1000).toFixed(3)+'s':''));
    if(goal.bossIds){const bosses=node('ul','');for(const id of goal.bossIds){const credit=p.credits?.find(c=>c.id===id);bosses.append(node('li',(credit?'✓ ':'○ ')+(content.bosses.find(b=>b.id===id)?.name??id)+(credit?' · '+(Number(credit.firstAtMs)/1000).toFixed(3)+'s':'')));}item.append(bosses);}list.append(item);
   }details.append(list);root.append(details);
  }
 }
};

for(const id of ['lobby-config','create-form']){const form=document.getElementById(id);if(form?.elements.mode&&!form.nt2RaceEditor){window.NT2_RACE_UI.attach(form,id==='lobby-config'?current?.room?.config:undefined);uxRaceMode(form,!form.elements.mode.disabled);}}
if(typeof current!=='undefined'&&current)window.NT2_RACE_UI.render(current);
