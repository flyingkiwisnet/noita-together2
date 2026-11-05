import {readFile} from 'node:fs/promises';
// Small DOM/form adapter for executing the real controller and its registered
// submit handlers. No room, identity or admission decisions live in this fixture.
export async function controllerDom(asset='../public/index.html'){
 const html=await readFile(new URL(asset,import.meta.url),'utf8');
 const ids=new Map(),all=[];
 class Element {
  constructor(tag){this.tagName=tag;this.children=[];this.handlers=new Map();this.dataset={};this.value='';this.defaultValue='';this.checked=false;this.defaultChecked=false;this.disabled=false;this.hidden=false;this._text='';this.scrollTop=0;this.scrollHeight=100;this.clientHeight=100;all.push(this);}
  set textContent(value){this._text=String(value);this.children=[];}
  get textContent(){return this._text+this.children.map(c=>typeof c==='string'?c:c.textContent).join('');}
  addEventListener(event,fn){const list=this.handlers.get(event)??[];list.push(fn);this.handlers.set(event,list);}
  async fire(event,details={}){for(const fn of this.handlers.get(event)??[])await fn({preventDefault(){},target:this,currentTarget:this,...details});}
  setAttribute(name,value){this.attributes??={};this.attributes[name]=String(value);}
  removeAttribute(name){if(this.attributes)delete this.attributes[name];}
  getAttribute(name){return this.attributes?.[name]??null;}
  focus(){document.activeElement=this;}
  append(...children){for(const child of children){this.children.push(child);if(typeof child==='object'){child.parent=this;child.connect(this.form);}}}
  insertBefore(child,before){if(child.parent)child.remove();const index=before?this.children.indexOf(before):this.children.length;this.children.splice(index,0,child);child.parent=this;child.connect(this.form);}
  connect(form){this.form=form;if(form&&this.name&&!form.elements.includes(this)){form.elements.push(this);form.elements[this.name]=this;}for(const child of this.children)if(typeof child==='object')child.connect(form);}
  replaceChildren(...children){this.children=[];this._text='';this.append(...children);}
  get firstChild(){return this.children[0];}
  get options(){return this.children.filter(e=>e.tagName==='option');}
  get selectedOptions(){return this.options.filter(e=>e.selected);}
  querySelector(tag){for(const child of this.children){if(child.tagName===tag)return child;const nested=child.querySelector?.(tag);if(nested)return nested;}return null;}
  closest(tag){return this.tagName===tag?this:this.parent?.closest?.(tag)??null;}
  dispatchEvent(event){void this.fire(event.type);if(event.bubbles)this.parent?.dispatchEvent?.(event);return true;}
  remove(){if(this.parent)this.parent.children=this.parent.children.filter(c=>c!==this);}
  reset(){for(const field of this.elements){field.value=field.defaultValue;field.checked=field.defaultChecked;}}
 }
 const attributes=text=>Object.fromEntries([...text.matchAll(/([\w-]+)(?:="([^"]*)")?/g)].map(m=>[m[1],m[2]??'']));
 for(const m of html.matchAll(/<(\w+)\b([^>]*\bid="[^"]+"[^>]*)>/g)){const attrs=attributes(m[2]);const el=new Element(m[1]);el.id=attrs.id;el.hidden=Object.hasOwn(attrs,'hidden');el.disabled=Object.hasOwn(attrs,'disabled');ids.set(el.id,el);}
 for(const m of html.matchAll(/<form\b([^>]*)>([\s\S]*?)<\/form>/g)){
  const form=ids.get(attributes(m[1]).id);form.elements=[];form.form=form;
  for(const nested of m[2].matchAll(/\bid="([^"]+)"/g))if(ids.has(nested[1]))ids.get(nested[1]).form=form;
  for(const c of m[2].matchAll(/<(input|select|textarea|button)\b([^>]*)>/g)){
   const attrs=attributes(c[2]),el=attrs.id&&ids.has(attrs.id)?ids.get(attrs.id):new Element(c[1]);el.name=attrs.name;el.type=attrs.type;el.className=attrs.class??'';el.defaultValue=attrs.value??'';el.defaultChecked=Object.hasOwn(attrs,'checked');
   if(c[1]==='select'){const tail=m[2].slice(c.index+c[0].length).split('</select>')[0];const options=[...tail.matchAll(/<option\b([^>]*)>([^<]*)/g)];const choice=options.find(o=>Object.hasOwn(attributes(o[1]),'selected'))??options[0];if(choice)el.defaultValue=attributes(choice[1]).value??choice[2];}
   el.value=el.defaultValue;el.checked=el.defaultChecked;el.disabled=Object.hasOwn(attrs,'disabled');form.elements.push(el);if(el.name)form.elements[el.name]=el;el.form=form;
  }
 }
 const document={getElementById:id=>{if(!ids.has(id))throw Error('Missing fixture element '+id);return ids.get(id);},createElement:tag=>new Element(tag),createTextNode:text=>{const e=new Element('#text');e.textContent=text;return e;},querySelectorAll:selector=>all.filter(e=>selector==='button'?e.tagName==='button':selector==='.mode'?e.className?.split(' ').includes('mode'):false)};
 const window=new Element('window');
 class FormDataCtor {constructor(form){this.entries=form.elements.filter(e=>e.name&&!e.disabled&&(e.type!=='checkbox'||e.checked)).map(e=>[e.name,e.type==='checkbox'?'on':e.value]);}*[Symbol.iterator](){yield* this.entries;}}
 return {document,window,FormDataCtor,schedule(){},get:id=>document.getElementById(id),async submit(id,values={}){const form=document.getElementById(id);for(const [name,value] of Object.entries(values)){const field=form.elements[name];if(!field)throw Error('Missing fixture control '+name);field.value=String(value);}await form.fire('submit');},async click(id){await document.getElementById(id).fire('click');}};
}
