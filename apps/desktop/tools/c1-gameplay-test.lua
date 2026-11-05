-- Production legacy arithmetic, injected engine values; no actual Noita evidence.
local repo=assert(os.getenv('NT2_ITEM_REPO'))
local m=dofile(repo..'/mod/noita_together2/files/g3/c1_gameplay.lua')
local checks=0
local function eq(a,b)assert(type(a)=='number'and math.abs(a-b)<1e-10 or a==b,tostring(a)..' ~= '..tostring(b));checks=checks+1 end
for _,n in ipairs({0,1,2,6,7,100,10000})do
 for _,mult in ipairs({0.5,1,2,10})do
  for _,large in ipairs({false,true})do
   for _,remote in ipairs({false,true})do
    for _,cap in ipairs({0,4,4.1})do
     local hp,max,zero=m.heart(2,4,cap,n,mult,large,remote)
     local minimum=large and not remote and 0.32 or 0.16
     local amount=large and not remote and 2 or 1
     local add=math.max(minimum,n>0 and amount/n or minimum)*mult
     eq(hp,remote and 2+add or 2);eq(max,cap>0 and math.min(4+add,cap)or 4+add);eq(zero,n==0)
    end
   end
  end
 end
end
local hp,max=m.heart(2,4,0,2,1,false,false);eq(hp*25,50);eq(max*25,112.5)
hp,max=m.heart(2,4,0,2,1,true,false);eq(hp*25,50);eq(max*25,125)
hp,max=m.heart(2,4,0,2,1,true,true);eq(hp*25,62.5);eq(max*25,112.5)
hp,max=m.heart(4,4,4,1,1,false,true);eq(hp,5);eq(max,4)
for _,maximum in ipairs({1.249,1.25,1.251,4})do
 for _,dying in ipairs({false,true})do
  local h,x,terminal=m.penalty(2,maximum,dying);eq(terminal,maximum*0.8<1)
  if not terminal then eq(x,maximum*0.8);eq(h,dying and x or math.min(2,x))end
 end
end
for _,alt in ipairs({false,true})do
 local h,x,stored=m.heartache_start(2,4,alt,false,0)
 eq(h,1);eq(x,alt and 4 or 2)
 -- Expiry reads captured alt, even when the live setting changed.
 h,x=m.heartache_end(h,x,stored,alt);eq(h,2);eq(x,4)
 h,x,stored=m.heartache_start(0.01,0.5,alt,false,0);eq(h,0.04)
 h,x,stored=m.heartache_start(2,4,alt,true,33);eq(h,2);eq(x,4);eq(stored,0)
end
local count=0;for id in pairs(m.team_perks)do count=count+1;eq(m.share_perk({NT_team_perks=true},id,false),true);eq(m.share_perk({NT_sync_perks=true},id,true),false)end;eq(count,9)
eq(m.share_perk({NT_sync_perks=true,NT_ban_perks=true},'GLASS_CANNON',false),false)
eq(m.share_perk({NT_sync_perks=true},'GLASS_CANNON',false),true)
eq(m.share_perk({},'EXTRA_MONEY',false),false)
for _,iteration in ipairs({19,20,21})do local ok,reason,next=m.fungal_gate(18000,0,false,iteration);eq(ok,iteration<=20);eq(next,iteration+1)end
local ok,reason,next=m.fungal_gate(17999,0,false,20);eq(ok,false);eq(reason,'fungal_cooldown');eq(next,20)
ok,reason,next=m.fungal_gate(18000,0,true,20);eq(ok,false);eq(reason,'gold_world');eq(next,20)
eq(m.orb_allowed({NT_sync_orbs=true,NT_sync_orb_count=0},100,false),true)
eq(m.orb_allowed({NT_sync_orbs=true,NT_sync_orb_count=2},2,false),false)
eq(m.orb_allowed({NT_sync_orbs=true,NT_sync_orb_count=2},1,true),false)
print('PASS '..checks..' C1 legacy arithmetic checks (injected values)')
