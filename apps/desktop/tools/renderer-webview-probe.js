// Loaded only by the isolated Rust example, never shipped in the production UI.
window.addEventListener('DOMContentLoaded',()=>setTimeout(async()=>{
    const checks=[];
    const check=(name,condition)=>{if(!condition)throw Error(name);checks.push(name);};
    try{
        const view=await window.__TAURI__.core.invoke('g1_status');render(view);
        const row=el('players').firstElementChild,checkbox=row.firstElementChild;
        const message=el('chat-messages').firstElementChild,option=el('room').firstElementChild;
        checkbox.focus();checkbox.checked=false;checkbox.dispatchEvent(new Event('change'));
        el('room').value=option.value;
        render(view);
        check('focused roster node retained',document.activeElement===checkbox&&el('players').firstElementChild===row&&!checkbox.checked);
        check('room option retained',el('room').firstElementChild===option&&el('room').value===option.value);
        view.rooms=[{roomId:'discovered',name:'Discovered room',state:'waiting'}];render(view);el('room').value='discovered';const selectedOption=[...el('room').children].find(o=>o.value==='discovered');view.rooms=[];render(view);
        check('discovery-page change retains selected room',el('room').value==='discovered'&&[...el('room').children].includes(selectedOption));
        const selection=window.getSelection(),range=document.createRange();range.setStart(message.firstChild,2);range.setEnd(message.firstChild,8);selection.removeAllRanges();selection.addRange(range);
        const selectedText=message.textContent.slice(2,8);
        render(view);check('selected chat text retained',selection.toString()===selectedText&&el('chat-messages').firstElementChild===message);
        const draft=el('chat-text');draft.focus();draft.value='draft text';draft.dispatchEvent(new Event('input'));draft.setSelectionRange(2,7);
        draft.dispatchEvent(new CompositionEvent('compositionstart'));
        render(view);check('draft focus caret and composition retained',document.activeElement===draft&&draft.value==='draft text'&&draft.selectionStart===2&&draft.selectionEnd===7&&composing.has('chat-text'));
        draft.dispatchEvent(new CompositionEvent('compositionend'));
        const changed=structuredClone(view);changed.room.members[0].loaded=true;
        const details=row.children[2];details.open=true;changed.inspections=[{desktopId:'desktop-one',actualSeed:'424242',activeMods:['noita_together2','other'],inventoryComplete:true,loaded:true,protected:true}];render(changed);
        check('inspection expansion survives poll',row.children[2]===details&&details.open&&details.children[2].children.length===2);
        changed.chat_messages=Array.from({length:60},(_,i)=>({sequence:i+1,accountId:'host',text:'Message '+i}));render(changed);
        const chat=el('chat-messages');chat.scrollTop=40;render(changed);check('reader scroll retained',chat.scrollTop===40);
        check('player node survives changed status',el('players').firstElementChild===row&&!checkbox.checked);
        draft.value='older draft';draft.dispatchEvent(new Event('input'));
        const sending=el('chat-send').onclick();draft.value='newer draft';draft.dispatchEvent(new Event('input'));await sending;
        check('typing during IPC preserved',draft.value==='newer draft');
        render({...view,desktop_id:'replacement'});check('replacement scope clears draft',draft.value==='');
        await window.__TAURI__.core.invoke('g1_action',{input:{action:'probe_results',passed:true,checks}});
    }catch(error){await window.__TAURI__.core.invoke('g1_action',{input:{action:'probe_results',passed:false,checks,error:String(error)}});}
},100));
