export const chatBytes=text=>new TextEncoder().encode(text).length;
export function senderColour(name){let n=2166136261;for(const c of name)n=Math.imul(n^c.codePointAt(0),16777619);return String((n>>>0)%6);}
export function mentions(text,name){if(!name)return false;const hay=text.toLocaleLowerCase(),needle='@'+name.toLocaleLowerCase();let at=hay.indexOf(needle);while(at>=0){if((!at||/\s/.test(hay[at-1]))&&(at+needle.length===hay.length||/[\s.,!?;:]/.test(hay[at+needle.length])))return true;at=hay.indexOf(needle,at+1);}return false;}
export function completionCandidates(value,caret,names,mods){const prefix=value.slice(0,caret),match=prefix.match(/(?:^|\s)([@:])([^\s]*)$/u);if(!match)return [];const typed=match[2].toLocaleLowerCase(),start=caret-match[2].length-1;return [...new Set(match[1]==='@'?names:mods)].filter(s=>s&&s.toLocaleLowerCase().startsWith(typed)).slice(0,8).map(s=>({label:match[1]+s,start,end:caret}));}
// Presentation only. The existing controller/client retain transport, session
// fencing, request identities and receipts.
export function createChatView(document,current){
 const $=id=>document.getElementById(id),list=$('messages'),field=$('chat').elements.text,rows=new Map();let unread=0,suggestions=[],choice=0,closed=false;
 const near=()=>list.scrollHeight-list.clientHeight-list.scrollTop<=24;
 function unreadUI(){ $('chat-unread').hidden=!unread;$('chat-unread').textContent=unread+' unread';$('chat-latest').hidden=!unread;$('chat-nav-unread').hidden=!unread;$('chat-nav-unread').textContent='('+unread+')';}
 function latest(){list.scrollTop=list.scrollHeight;unread=0;unreadUI();}
 function hideSuggestions(){suggestions=[];closed=true;$('chat-completions').hidden=true;$('chat-completions').replaceChildren();}
 function accept(index){const suggestion=suggestions[index];if(!suggestion)return;field.value=field.value.slice(0,suggestion.start)+suggestion.label+' '+field.value.slice(suggestion.end);const caret=suggestion.start+suggestion.label.length+1;field.selectionStart=field.selectionEnd=caret;hideSuggestions();field.focus?.();}
 function suggest(){const {room,reports}=current();if(!room||field.composing)return hideSuggestions();closed=false;suggestions=completionCandidates(field.value,field.selectionStart??field.value.length,(room.members??[]).map(m=>m.principal?.displayName??''),[...(reports?.values()??[])].flatMap(r=>r.activeMods??[]));choice=0;renderSuggestions();}
 function renderSuggestions(){const root=$('chat-completions');root.replaceChildren();root.hidden=!suggestions.length;for(const [i,s]of suggestions.entries()){const button=document.createElement('button');button.type='button';button.setAttribute?.('role','option');button.setAttribute?.('aria-selected',String(choice===i));button.textContent=s.label;button.addEventListener('mousedown',e=>e.preventDefault());button.addEventListener('click',()=>accept(i));root.append(button);}}
 field.addEventListener('input',()=>{suggest();$('chat-limits').textContent=chatBytes(field.value)+' / 1,024 UTF-8 bytes · 5 sends per 10 seconds. Recent history only.';});
 field.addEventListener('keydown',e=>{if(field.composing)return;if(e.key==='Escape'){hideSuggestions();return;}if(!suggestions.length||closed)return;if(['ArrowDown','ArrowUp'].includes(e.key)){e.preventDefault();choice=(choice+(e.key==='ArrowDown'?1:-1)+suggestions.length)%suggestions.length;renderSuggestions();}else if(e.key==='Enter'||e.key==='Tab'){e.preventDefault();accept(choice);}});
 list.addEventListener('scroll',()=>{if(near()){unread=0;unreadUI();}});$('chat-latest').addEventListener('click',latest);$('jump-chat').addEventListener('click',()=>{$('chat-panel').scrollIntoView?.({behavior:'smooth',block:'nearest'});});
 function feedback(value){$('chat-feedback').textContent=value;}
 function reset(){rows.clear();list.replaceChildren();field.value='';unread=0;unreadUI();hideSuggestions();feedback('');}
 function append(result){const state=current();if(!state.room)return;const following=near(),top=list.scrollTop;let added=0;if(result.chatReset){rows.clear();list.replaceChildren();}
  for(const message of result.chatMessages??[]){if(rows.has(message.sequence))continue;const row=document.createElement('li'),meta=document.createElement('small'),sender=document.createElement('span'),body=document.createElement('span');const name=message.principal?.displayName??state.room.members?.find(m=>m.accountId===message.accountId)?.principal?.displayName??'Player';const date=new Date(message.sentAt);meta.textContent=Number.isFinite(+date)?date.toLocaleTimeString([],{hour:'2-digit',minute:'2-digit',hour12:false}):'';sender.className='sender';sender.dataset.colour=senderColour(name);sender.textContent=name+': ';body.textContent=message.text;row.className=mentions(message.text,state.account?.displayName??state.room.members?.find(m=>m.accountId===state.credentials?.accountId)?.principal?.displayName)?'mention':'';row.append(meta,sender,body);rows.set(message.sequence,row);added++;}
  // A send receipt may arrive before older messages from the next list page.
  const ordered=[...rows.keys()].sort((a,b)=>a-b);while(ordered.length>250){const first=ordered.shift();rows.get(first).remove();rows.delete(first);}for(const [index,key]of ordered.entries())if(list.children[index]!==rows.get(key))list.insertBefore(rows.get(key),list.children[index]??null);
  if(following)latest();else{list.scrollTop=top;unread+=added;unreadUI();}
 }
 return {append,reset,feedback,hideSuggestions};
}
