// Minimal DOM adapter for exercising the shipped scripts without a GUI or network.
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import {webcrypto} from 'node:crypto';
import {LOBBY_SETTING_HINTS} from '../ui/lobby-setting-hints.js';
class Node{
 constructor(tag,doc){this.tagName=tag.toLowerCase();this.doc=doc;this.children=[];this.dataset={};this.attributes={};this.style={};this.events={};this._text='';this._value='';this.hidden=false;this.disabled=false;this.checked=false;this.scrollTop=0;this.scrollHeight=0;this.clientHeight=0;this.selectionStart=0;this.selectionEnd=0;const classes=new Set();this.classList={toggle:(key,on)=>on?classes.add(key):classes.delete(key),contains:key=>classes.has(key)};}
 append(...nodes){for(const n of nodes)this.insertBefore(n,null);}appendChild(n){this.append(n);return n;}
 insertBefore(n,before){if(typeof n==='string'){this._text+=n;return;}n.remove();const i=before?this.children.indexOf(before):this.children.length;this.children.splice(i<0?this.children.length:i,0,n);n.parentNode=this;if(this.tagName==='select'&&this.children.length===1)this._value=n.value;}
 after(n){const p=this.parentNode;if(p)p.insertBefore(n,this.nextElementSibling);}
 remove(){if(this.parentNode){const p=this.parentNode;p.children.splice(p.children.indexOf(this),1);this.parentNode=null;}}
 replaceChildren(...nodes){for(const n of this.children)n.parentNode=null;this.children=[];this._text='';this.append(...nodes);}
 get firstChild(){return this.children[0];}get lastChild(){return this.children.at(-1);}get firstElementChild(){return this.firstChild;}get parentElement(){return this.parentNode;}get nextElementSibling(){const p=this.parentNode;return p?.children[p.children.indexOf(this)+1];}
 get textContent(){return this._text+this.children.map(n=>n.textContent).join('');}set textContent(v){this.replaceChildren();this._text=String(v??'');}
 get value(){return this._value;}set value(v){this._value=String(v??'');}
 get elements(){const controls=n=>['input','select','button','textarea'].includes(n.tagName);const list=this.all().filter(controls);if(this.tagName==='form'&&this.id)for(const n of this.doc.body.all())if(controls(n)&&n.getAttribute('form')===this.id&&!list.includes(n))list.push(n);for(const n of list)if(n.name)list[n.name]=n;return list;}
 all(){return this.children.flatMap(n=>[n,...n.all()]);}
 setAttribute(k,v){this.attributes[k]=String(v);if(k.startsWith('data-'))this.dataset[k.slice(5).replace(/-([a-z])/g,(_,x)=>x.toUpperCase())]=String(v);else if(k==='class')this.className=v;else this[k]=['hidden','disabled','checked','open'].includes(k)?true:String(v);}
 getAttribute(k){return this.attributes[k]??null;}
 matches(s){if(s===':focus')return this.doc.activeElement===this;if(s==='input:checked')return this.tagName==='input'&&this.checked;if(s.startsWith('.'))return(this.className??'').split(' ').includes(s.slice(1));const match=/^(\w+)?\[([\w-]+)(\^?=)?["']?([^\]"']*)["']?\]$/.exec(s);if(match){const actual=match[2].startsWith('data-')?this.dataset[match[2].slice(5).replace(/-([a-z])/g,(_,x)=>x.toUpperCase())]:this[match[2]];return(!match[1]||match[1]===this.tagName)&&actual!==undefined&&(!match[3]||(match[3]==='^='?String(actual).startsWith(match[4]):String(actual)===match[4]));}return this.tagName===s.toLowerCase();}
 querySelectorAll(s){return this.all().filter(n=>s.split(',').some(q=>n.matches(q.trim())));}querySelector(s){return this.querySelectorAll(s)[0]??null;}
 addEventListener(k,fn){(this.events[k]??=[]).push(fn);}removeEventListener(k,fn){this.events[k]=(this.events[k]??[]).filter(f=>f!==fn);}
 dispatchEvent(event){event.target??=this;event.currentTarget=this;for(const fn of this.events[event.type]??[])fn(event);}
 focus(){this.doc.activeElement=this;}select(){this.focus();}setSelectionRange(a,b){this.selectionStart=a;this.selectionEnd=b;}
 showModal(){this.open=true;this.focus();}close(){this.open=false;this.dispatchEvent({type:'close'});}reset(){for(const n of this.elements)n.value='';}
}
function parse(html,doc){const root=new Node('root',doc),stack=[root],voids=new Set(['meta','link','input','br','hr','img']);for(const token of html.match(/<[^>]+>|[^<]+/g)??[]){if(token.startsWith('<!'))continue;if(token.startsWith('</')){const tag=token.slice(2,-1).trim();while(stack.length>1){if(stack.pop().tagName===tag)break;}continue;}if(token.startsWith('<')){const tag=/^<([\w-]+)/.exec(token)?.[1];if(!tag)continue;const n=new Node(tag,doc);for(const a of token.matchAll(/([\w-]+)(?:="([^"]*)")?/g)){if(a.index===1)continue;n.setAttribute(a[1],a[2]??'');}stack.at(-1).append(n);if(!voids.has(tag)&&!token.endsWith('/>'))stack.push(n);}else if(token.trim())stack.at(-1)._text+=token.replaceAll('&amp;','&').trim();}return root;}
export async function fixture(initial,handler){const calls=[],intervals=[];const document={activeElement:null,createElement:tag=>new Node(tag,document),execCommand:()=>true};const root=parse(readFileSync(new URL('../ui/index.html',import.meta.url),'utf8'),document);document.body=root.all().find(n=>n.tagName==='body');document.getElementById=id=>root.all().find(n=>n.id===id)??null;document.querySelectorAll=s=>root.querySelectorAll(s);let current=initial;
 const invoke=async(command,args)=>{calls.push({command,input:args?.input});if(handler){const value=await handler(command,args?.input,current);if(value!==undefined){current=value;return value;}}return current;};
 const context=vm.createContext({document,window:{crypto:webcrypto,NT2_LOBBY_SETTING_HINTS:LOBBY_SETTING_HINTS,__NT2_CANDIDATE__:true,__TAURI__:{core:{invoke}},getSelection:()=>null},TextEncoder,Map,Set,Date,JSON,Promise,queueMicrotask,setTimeout,clearTimeout,setInterval:fn=>intervals.push(fn),Option:function(label,value){const n=new Node('option',document);n.textContent=label;n.value=value;return n;}});
 for(const name of['ux1.js','ux1-mods.js','ux1-forms.js','ux1-chat.js','ux1-players.js','ux1-outcome.js','app.js'])vm.runInContext(readFileSync(new URL('../ui/'+name,import.meta.url),'utf8'),context,{filename:name});
 const settle=async()=>{for(let i=0;i<12;i++)await new Promise(resolve=>setImmediate(resolve));};await settle();
 return{context,calls,el:id=>document.getElementById(id),settle,read:code=>vm.runInContext(code,context),render:async value=>{current=value;context.testView=value;vm.runInContext('render(testView)',context);await settle();}};
}
