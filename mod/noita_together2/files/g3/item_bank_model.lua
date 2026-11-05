-- Adapted from Noita Together ui.lua filterItems/sortItems/tabToggles (MIT,
-- Soler91, 2022). Presentation state only; no ownership or transfer authority.
local text=dofile('mods/noita_together2/files/g3/a2_text.lua')
local M={PAGE_SIZE=40}
local criteria={
 {key='shuffleDeckWhenEmpty',label='Shuffle',ascending=true},
 {key='actionsPerRound',label='Casts',ascending=true},
 {key='fireRateWait',label='Cast Delay',ascending=true},
 {key='reloadTime',label='Recharge Time',ascending=true},
 {key='manaMax',label='Mana Max',ascending=false},
 {key='manaChargeSpeed',label='Mana Charge Speed',ascending=false},
 {key='deckCapacity',label='Capacity',ascending=false},
 {key='spreadDegrees',label='Spread',ascending=true},
}
M.criteria=criteria
M.categories={'Projectiles','Static Projectiles','Modifiers','Multicasts','Materials','Other','Utility','Passive'}
local rank={wand=1,spell=2,potion=3,powder=3,container=3,object=4}
local function finite(v)return type(v)=='number'and v==v and v>-math.huge and v<math.huge end
local function value(row,key)local v=row.stats and row.stats[key];if type(v)=='boolean'then return v and 1 or 0 end;return finite(v)and v or nil end
local function identity(row)return tostring(row.id or '')end
function M.new()
 local self={tab='all',search='',page=0,spell_filters={},wand_filters={},grouped=false,rows={},revision=0,scope=nil,tombstones={}}
 for i=1,8 do self.spell_filters[i]=true;self.wand_filters[i]=i<=4 end
 function self:bind(scope)
  if self.scope~=scope then self.scope=scope;self.rows={};self.revision=0;self.tombstones={};self.page=0 end
 end
 function self:set_tab(tab)if tab~='all'and tab~='wands'and tab~='spells'and tab~='items'then return false end;if self.tab~=tab then self.tab=tab;self.page=0 end;return true end
 function self:edit(search)
  if type(search)~='string'or not text.length(search)then return false end
  while (text.length(search)or 0)>32 do search=text.backspace(search)end
  if self.search~=search then self.search=search;self.page=0 end;return true
 end
 function self:toggle(which,index,solo)
  local toggles=which=='spells'and self.spell_filters or which=='wands'and self.wand_filters
  if not toggles or type(index)~='number'or index<1 or index>8 or index%1~=0 then return false end
  if solo then for i=1,8 do toggles[i]=i==index end else toggles[index]=not toggles[index]end
  self.page=0;return true
 end
 function self:sort()self.grouped=true;self.page=0 end
 function self:removed(id,revision)
  if type(id)~='string'or not finite(revision)or revision%1~=0 or revision<0 then return false end
  self.tombstones[id]=math.max(self.tombstones[id]or 0,revision)
  for _,row in ipairs(self.rows)do if row.id==id and (row.revision or 0)<=revision then row.removed=true end end
  return true
 end
 -- Called only for a complete, coherent service snapshot. A partial network
 -- page never replaces the displayed bank or claims absence of unloaded rows.
 function self:replace(scope,revision,rows)
  if scope~=self.scope or not finite(revision)or revision%1~=0 or revision<self.revision or type(rows)~='table'then return false end
  local seen,next_rows={},{}
  for _,row in ipairs(rows)do
   if type(row)~='table'or type(row.id)~='string'or seen[row.id]or not rank[row.kind]then return false end
   seen[row.id]=true;local next={};for key,v in pairs(row)do next[key]=v end
   if self.tombstones[row.id]and (row.revision or revision)<=self.tombstones[row.id]then next.removed=true end
   next_rows[#next_rows+1]=next
  end
  self.rows=next_rows;self.revision=revision;self:visible();return true
 end
 function self:matches(row)
  if self.tab=='wands'and row.kind~='wand'or self.tab=='spells'and row.kind~='spell'or self.tab=='items'and row.kind~='potion'and row.kind~='powder'and row.kind~='container'and row.kind~='object'then return false end
  if self.tab=='spells'and not self.spell_filters[(row.category or 5)+1]then return false end
  local needle=self.search:lower()
  return needle==''or tostring(row.search_text or row.name or ''):lower():find(needle,1,true)~=nil
 end
 function self:visible()
  local rows={};for _,row in ipairs(self.rows)do if self:matches(row)then rows[#rows+1]=row end end
  table.sort(rows,function(a,b)
   if self.grouped and rank[a.kind]~=rank[b.kind]then return rank[a.kind]<rank[b.kind]end
   if self.tab=='wands'and a.kind=='wand'and b.kind=='wand'then
    for i,criterion in ipairs(criteria)do if self.wand_filters[i]then local av,bv=value(a,criterion.key),value(b,criterion.key)
     if av~=bv then if av==nil then return false elseif bv==nil then return true end;return criterion.ascending and av<bv or not criterion.ascending and av>bv end
    end end
   end
   if (a.order or 0)~=(b.order or 0)then return (a.order or 0)<(b.order or 0)end
   return identity(a)<identity(b)
  end)
  local pages=math.max(1,math.ceil(#rows/M.PAGE_SIZE));self.page=math.max(0,math.min(self.page,pages-1))
  local page={};for i=self.page*M.PAGE_SIZE+1,math.min(#rows,(self.page+1)*M.PAGE_SIZE)do page[#page+1]=rows[i]end
  return page,pages,#rows
 end
 function self:move(delta)local _,pages=self:visible();self.page=math.max(0,math.min(pages-1,self.page+delta));return self.page end
 return self
end
return M
