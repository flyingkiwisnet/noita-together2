import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
class Node {
 constructor(tag){this.tag=tag;this.children=[];this.dataset={};this.textContent='';}
 append(...nodes){for(const node of nodes)this.insertBefore(node,null);}
 insertBefore(node,before){node.remove();this.children.splice(before?this.children.indexOf(before):this.children.length,0,node);node.parentNode=this;}
 remove(){if(this.parentNode){const list=this.parentNode.children;list.splice(list.indexOf(this),1);this.parentNode=null;}}
 setAttribute(){}matches(){return false;}
 get firstChild(){return this.children[0];}get lastChild(){return this.children.at(-1);}get firstElementChild(){return this.firstChild;}
 get nextElementSibling(){return this.parentNode?.children[this.parentNode.children.indexOf(this)+1];}
 querySelector(selector){const role=selector.match(/data-role="([^"]+)"/)?.[1];return this.children.flatMap(n=>[n,...n.all()]).find(n=>n.dataset.role===role);}
 all(){return this.children.flatMap(n=>[n,...n.all()]);}
}
const body=new Node('body'),calls=[];
const document={body,createElement:tag=>new Node(tag),getElementById:id=>body.all().find(n=>n.id===id)};
const context=vm.createContext({document,window:{},call:(...args)=>calls.push(args)});
const source=readFileSync(new URL('../ui/app.js',import.meta.url),'utf8');
vm.runInContext(source.slice(source.indexOf('function text('),source.indexOf('function scopeOf('))+`const el=id=>document.getElementById(id);`+source.slice(source.indexOf('function render(v){'),source.indexOf('    const nextSession='))+'}',context);
const value={signed_in:true,g3:{enabled:true,bank:{status:'Gold transfer completed in this session.',items:{status:''}},operations:[{operation_id:'retained-id',room_id:'old-lobby',direction:'deposit',asset:'item',state:'retained_unverified',restored:true,current_scope:false}]}};
context.value=value;vm.runInContext('render(value)',context);
const panel=document.getElementById('g3-originals'),history=panel.children.find(n=>n.tag==='details');
assert(history&&!history.open);assert.match(history.firstChild.textContent,/retained history/);
assert.equal(panel.querySelector('[data-role="bank-status"]').textContent,value.g3.bank.status);
const row=panel.querySelector('[data-role="records"]').firstChild;
assert.match(row.firstChild.textContent,/old-lobby.*loaded-world outcome unverified/);
row.lastChild.onclick();assert.equal(calls[0][0],'g3_original_status');assert.equal(calls[0][1].operationId,'retained-id');
history.open=true;vm.runInContext('render(value)',context);assert.equal(panel.children.find(n=>n.tag==='details'),history);assert(history.open);
value.g3.operations[0]={...value.g3.operations[0],state:'applied_session',restored:false,current_scope:true};vm.runInContext('render(value)',context);assert.match(row.firstChild.textContent,/saved-world persistence unproven/);
console.log('Bank history UI: 9 assertions passed');
